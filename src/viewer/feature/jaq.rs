use std::{borrow::Cow, sync::Arc};

use jaq_all::fmts::{
    Format,
    write::{Writer, json::Pp, write},
};

use crate::{
    command::jaq::{Completion, Event, JaqExecutor, parse_inputs},
    document_source::DocumentSource,
    input_format::InputFormat,
    utils::{normalize_lines, one_line},
    viewer::{
        DEBUG_LOADING_TICKS,
        feature::{Feedback, FeedbackKind, HintFocus, QueryHintState, QueryHints},
        ui::DocumentDisplayConfig,
    },
};

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const MAX_EVENTS_PER_TICK: usize = 64;

pub(in crate::viewer) enum JaqStatus {
    Running,
    Cancelling,
    Completed,
    Cancelled,
    Failed(String),
}

pub(in crate::viewer) struct JaqSession {
    query: String,
    inputs: Arc<[jaq_all::json::Val]>,
    executor: JaqExecutor,
    pub(in crate::viewer) output: Vec<u8>,
    display_config: DocumentDisplayConfig,
    pub(in crate::viewer) status: JaqStatus,
    spinner_frame: usize,
    loading_ticks_remaining: usize,
    hints: QueryHints,
}

impl JaqSession {
    pub(in crate::viewer) fn start(
        query: String,
        source: &DocumentSource,
        display_config: DocumentDisplayConfig,
        hints: QueryHints,
    ) -> anyhow::Result<Self> {
        let inputs: Arc<[jaq_all::json::Val]> =
            parse_inputs(source.format(), source.content())?.into();
        let mut executor = JaqExecutor::default();
        executor.execute(query.clone(), Arc::clone(&inputs));

        Ok(Self {
            query,
            inputs,
            executor,
            output: Vec::new(),
            display_config,
            status: JaqStatus::Running,
            spinner_frame: 0,
            loading_ticks_remaining: DEBUG_LOADING_TICKS,
            hints,
        })
    }

    pub(in crate::viewer) fn command_line(&self) -> String {
        format!("jaq {}", self.query)
    }

    pub(in crate::viewer) fn is_running(&self) -> bool {
        matches!(self.status, JaqStatus::Running | JaqStatus::Cancelling)
    }

    pub(in crate::viewer) fn tick(&mut self) -> bool {
        if !self.is_running() {
            return false;
        }

        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        if self.loading_ticks_remaining > 0 {
            self.loading_ticks_remaining -= 1;
            return false;
        }
        let mut output_changed = false;

        for _ in 0..MAX_EVENTS_PER_TICK {
            let Some(event) = self.executor.try_recv() else {
                break;
            };

            match event {
                Event::Output { value, .. } => {
                    let format = match self.display_config.format() {
                        InputFormat::Json => Format::Json,
                        InputFormat::Yaml => Format::Yaml,
                        InputFormat::Auto => {
                            unreachable!("input format must be resolved before executing jaq")
                        }
                    };
                    let pp = match format {
                        Format::Yaml => Pp {
                            indent: Some("  ".to_owned()),
                            sep_space: true,
                            ..Default::default()
                        },
                        _ => Pp::default(),
                    };
                    if let Err(error) = write(
                        &mut self.output,
                        &Writer {
                            format,
                            pp,
                            ..Default::default()
                        },
                        &value,
                    ) {
                        self.executor.cancel();
                        self.status = JaqStatus::Failed(error.to_string());
                        break;
                    }
                    output_changed = true;
                }
                Event::Finished { completion, .. } => {
                    match completion {
                        Completion::Completed => self.status = JaqStatus::Completed,
                        Completion::Cancelled => self.status = JaqStatus::Cancelled,
                        Completion::Failed(message) => {
                            self.status = JaqStatus::Failed(message);
                        }
                    }
                    break;
                }
            }
        }

        output_changed
    }

    pub(in crate::viewer) fn update_config(&mut self, config: &crate::config::Config) {
        self.display_config =
            DocumentDisplayConfig::from_config(self.display_config.format(), config);
        self.hints = QueryHints::jaq(&config.keybinds);
    }

    pub(in crate::viewer) fn cancel(&self) {
        self.executor.cancel();
    }

    pub(in crate::viewer) fn feedback(&self, focus: HintFocus) -> Feedback<'_> {
        let (message, hint_state, kind) = match &self.status {
            JaqStatus::Running => (
                format!("{} running", SPINNER_FRAMES[self.spinner_frame]),
                QueryHintState::Running,
                FeedbackKind::Loading,
            ),
            JaqStatus::Cancelling => (
                format!("{} cancelling", SPINNER_FRAMES[self.spinner_frame]),
                QueryHintState::Running,
                FeedbackKind::Warning,
            ),
            JaqStatus::Completed => (
                "completed".to_owned(),
                QueryHintState::Finished,
                FeedbackKind::Success,
            ),
            JaqStatus::Cancelled => (
                "cancelled".to_owned(),
                QueryHintState::Finished,
                FeedbackKind::Warning,
            ),
            JaqStatus::Failed(message) => (
                format!("failed: {}", normalize_lines(message)),
                QueryHintState::Finished,
                FeedbackKind::Error,
            ),
        };
        let message = if !matches!(focus, HintFocus::Editor) {
            format!("{message} — query: {}", one_line(&self.query))
        } else {
            message
        };

        Feedback::View {
            name: "jaq",
            message: Cow::Owned(message),
            hint: Cow::Owned(self.hints.create(focus, hint_state)),
            kind,
        }
    }

    pub(in crate::viewer) fn execute(&mut self, query: String) {
        self.query = query;
        self.output.clear();
        self.status = JaqStatus::Running;
        self.spinner_frame = 0;
        self.loading_ticks_remaining = DEBUG_LOADING_TICKS;
        self.executor
            .execute(self.query.clone(), Arc::clone(&self.inputs));
    }
}

impl Drop for JaqSession {
    fn drop(&mut self) {
        self.cancel();
    }
}
