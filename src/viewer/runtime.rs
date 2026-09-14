use std::{borrow::Cow, time::Duration};

use crate::{
    catalog::FlattenSubscription,
    command::{argument::document_path::PathCompleter, config::Config as ConfigCommand, write},
    config::{Config, ConfigFile, ConfigLoadError, Keybinds, ThemeConfig},
    document_source::DocumentSource,
    history::History,
    viewer::{Action, ComponentAction, DocumentAction},
};
use futures::StreamExt;
use promkit::core::{
    ContentPosition, CreatedGraphemes, ScreenPosition,
    crossterm::{
        event::{Event, EventStream, MouseEvent, MouseEventKind},
        terminal,
    },
    render::{Renderer, SharedRenderer},
};

use super::feature::{Feedback, QueryHints};
use super::stack::ViewStack;
use super::view::{BrowseView, ConfigView, FlattenView, HelpView, HstrView, PreviewView};
use super::{
    ActionContext, ConfigUpdate, DocumentComponent, View, ViewEffect, ViewEvent, ViewNotification,
    ViewRequest, ViewUpdate, ViewerOutcome, ViewerSources, key_resolution, rendered_height,
};

const TICK_INTERVAL: Duration = Duration::from_millis(80);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Index {
    Feedback,
    Content,
    Suggestions,
    Editor,
}

enum StartupWarning {
    ConfigLoad(ConfigLoadError),
}

impl StartupWarning {
    fn create_graphemes(&self, theme: &ThemeConfig) -> CreatedGraphemes {
        match self {
            Self::ConfigLoad(error) => Feedback::Error {
                name: "config",
                message: Cow::Owned(error.to_string()),
            }
            .create_graphemes(&theme.feedback),
        }
    }
}

fn movement_lines(
    action: Action,
    event: &Event,
    scroll_lines: usize,
    content_height: usize,
) -> usize {
    match action {
        Action::Component(ComponentAction::Document(
            DocumentAction::PageUp | DocumentAction::PageDown,
        )) => content_height.max(1),
        Action::Component(ComponentAction::Document(
            DocumentAction::HalfPageUp | DocumentAction::HalfPageDown,
        )) => (content_height / 2).max(1),
        _ if matches!(
            event,
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp | MouseEventKind::ScrollDown,
                ..
            })
        ) =>
        {
            scroll_lines
        }
        _ => 1,
    }
}

pub(crate) struct Viewer {
    renderer: SharedRenderer<Index>,
    views: ViewStack,
    startup_warning: Option<StartupWarning>,
    keybinds: Keybinds,
    theme: ThemeConfig,
    scroll_lines: usize,
    content_height: usize,
    config_file: ConfigFile,
    clipboard: Option<arboard::Clipboard>,
    history: History,
}

enum ConfigCommandOutcome {
    Applied,
    Open(String),
    Edit,
}

impl ViewerSources {
    pub(crate) fn new(
        document: DocumentSource,
        flatten: FlattenSubscription,
        config_file: ConfigFile,
    ) -> Self {
        Self {
            document,
            flatten,
            config_file,
        }
    }
}

impl Viewer {
    pub(crate) async fn new(
        document: DocumentComponent,
        path_completer: PathCompleter,
        sources: ViewerSources,
        config_load_error: Option<ConfigLoadError>,
        keybinds: Keybinds,
        theme: ThemeConfig,
        scroll_lines: usize,
    ) -> anyhow::Result<Self> {
        let history = History::load_default();
        let input = BrowseView::new(
            document,
            sources.document.clone(),
            path_completer,
            sources.flatten.clone(),
            history.clone(),
            &theme,
        );
        let mut viewer = Self {
            renderer: SharedRenderer::new(Renderer::<Index>::try_new()?),
            views: ViewStack::new(Box::new(input)),
            startup_warning: config_load_error.map(StartupWarning::ConfigLoad),
            keybinds,
            theme,
            scroll_lines,
            content_height: 1,
            config_file: sources.config_file,
            clipboard: None,
            history,
        };
        viewer.render().await?;
        Ok(viewer)
    }

    pub(crate) async fn run(&mut self) -> anyhow::Result<ViewerOutcome> {
        self.render().await?;
        let mut events = EventStream::new();
        let mut ticker = tokio::time::interval(TICK_INTERVAL);

        loop {
            let update = tokio::select! {
                event = events.next() => {
                    let Some(event) = event else {
                        return Ok(ViewerOutcome::Exit);
                    };
                    self.dispatch_event(event?)
                }
                _ = ticker.tick() => self.active_view_mut().update(ViewEvent::Tick),
            };

            match update {
                ViewUpdate::Ignored | ViewUpdate::Handled => continue,
                ViewUpdate::Render => {}
                ViewUpdate::Effect(effect) => {
                    if let Some(outcome) = self.execute_effect(effect) {
                        return Ok(outcome);
                    }
                }
            }
            // Effects apply their notifications and navigation before the active view is rendered.
            self.render().await?;
        }
    }

    /// Executes one request, including result notifications. Only `run` schedules rendering.
    fn execute_effect(&mut self, effect: ViewEffect) -> Option<ViewerOutcome> {
        match effect {
            ViewEffect::Open(request) => self.open_view(request),
            ViewEffect::Config(command) => match self.execute_config_command(command) {
                ConfigCommandOutcome::Applied => {}
                ConfigCommandOutcome::Open(content) => {
                    self.open_view(ViewRequest::Config { content })
                }
                ConfigCommandOutcome::Edit => return Some(ViewerOutcome::EditConfig),
            },
            ViewEffect::InspectConfig(key) => {
                let target = self.config_file.get(&key).ok().map(|value| (key, value));
                self.active_view_mut()
                    .notify(ViewNotification::ConfigTarget(target));
            }
            ViewEffect::Copy { target, content } => {
                let notification = match self.copy_to_clipboard(content) {
                    Ok(()) => ViewNotification::CopySucceeded(target.name().to_owned()),
                    Err(error) => ViewNotification::CopyFailed(error.to_string()),
                };
                self.active_view_mut().notify(notification);
            }
            ViewEffect::Navigate {
                document_index,
                path,
            } => {
                self.views.return_to_caller(ViewNotification::Navigate {
                    document_index,
                    path,
                });
            }
            ViewEffect::RecallCommand(command) => {
                self.views
                    .return_to_caller(ViewNotification::RecallCommand(command));
            }
            ViewEffect::Back => self.views.back(),
            ViewEffect::Exit => return Some(ViewerOutcome::Exit),
            ViewEffect::Print(source) => return Some(ViewerOutcome::Print(source)),
            ViewEffect::Write {
                source,
                destination,
            } => {
                let notification = match write::save(&destination, &source) {
                    Ok(()) => ViewNotification::WriteSucceeded(destination.display().to_string()),
                    Err(error) => ViewNotification::WriteFailed(format!("{error:#}")),
                };
                self.active_view_mut().notify(notification);
            }
        }
        None
    }

    fn open_view(&mut self, request: ViewRequest) {
        match self.create_view(request) {
            Ok(view) => {
                self.active_view_mut()
                    .notify(ViewNotification::OpenSucceeded);
                self.views.push(view);
            }
            Err(error) => self
                .active_view_mut()
                .notify(ViewNotification::OpenFailed(error.to_string())),
        }
    }

    pub(crate) fn edit_config(&self) -> anyhow::Result<()> {
        self.config_file.edit()
    }

    pub(crate) fn finish_config_edit(&mut self, edit_result: anyhow::Result<()>) {
        let result = edit_result.and_then(|()| self.config_file.load());
        match result {
            Ok(config) => self.apply_config(config, "configuration reloaded".to_owned()),
            Err(error) => self.notify_config_failed(error.to_string()),
        }
    }

    fn execute_config_command(&mut self, command: ConfigCommand) -> ConfigCommandOutcome {
        match command {
            ConfigCommand::View(_) => match self.config_file.read() {
                Ok(content) => ConfigCommandOutcome::Open(content),
                Err(error) => {
                    self.notify_config_failed(error.to_string());
                    ConfigCommandOutcome::Applied
                }
            },
            ConfigCommand::Get(get) => {
                match self.config_file.get(&get.key) {
                    Ok(value) => {
                        self.active_view_mut()
                            .notify(ViewNotification::ConfigValue(format!(
                                "{} = {value}",
                                get.key
                            )));
                    }
                    Err(error) => self.notify_config_failed(error.to_string()),
                }
                ConfigCommandOutcome::Applied
            }
            ConfigCommand::Set(set) => {
                match self.config_file.set(&set.key, &set.value) {
                    Ok(config) => {
                        let value = self
                            .config_file
                            .get(&set.key)
                            .unwrap_or_else(|_| set.value.clone());
                        self.apply_config(config, format!("{} = {value}", set.key));
                    }
                    Err(error) => self.notify_config_failed(error.to_string()),
                }
                ConfigCommandOutcome::Applied
            }
            ConfigCommand::Edit(_) => ConfigCommandOutcome::Edit,
        }
    }

    fn apply_config(&mut self, config: Config, message: String) {
        self.keybinds = config.keybinds.clone();
        self.scroll_lines = config.scroll_lines.get();
        self.theme = config.theme.clone();
        self.startup_warning = None;
        self.views.update_config(ConfigUpdate { message, config });
    }

    fn notify_config_failed(&mut self, error: String) {
        self.active_view_mut()
            .notify(ViewNotification::ConfigFailed(error));
    }

    fn copy_to_clipboard(&mut self, content: String) -> anyhow::Result<()> {
        if self.clipboard.is_none() {
            self.clipboard = Some(arboard::Clipboard::new()?);
        }
        self.clipboard
            .as_mut()
            .expect("clipboard was initialized")
            .set_text(content)?;
        Ok(())
    }

    async fn render(&mut self) -> anyhow::Result<()> {
        let (width, height) = terminal::size()?;
        let mut frame = self.active_view_mut().render(width, height);
        if frame.feedback.graphemes.is_empty()
            && let Some(warning) = &self.startup_warning
        {
            let feedback = warning.create_graphemes(&self.theme);
            let content_height = height.saturating_sub(rendered_height(&feedback, width));
            // The view laid out its content without feedback, so render it again with the
            // startup warning's height reserved before composing the final frame.
            frame = self.active_view_mut().render(width, content_height);
            frame.feedback = feedback;
        }
        self.content_height = frame.content_height;
        let graphemes = [
            (Index::Feedback, frame.feedback),
            (Index::Content, frame.content),
            (Index::Suggestions, frame.suggestions),
            (Index::Editor, frame.editor),
        ];

        self.renderer.update(graphemes).render().await
    }

    /// Resolves every terminal event once, then dispatches the resulting typed action.
    fn dispatch_event(&mut self, event: Event) -> ViewUpdate {
        if let Event::Mouse(mouse) = &event
            && matches!(
                mouse.kind,
                MouseEventKind::Down(_) | MouseEventKind::Drag(_) | MouseEventKind::Up(_)
            )
        {
            let position = self.content_position_for_mouse(&event);
            let update = self.active_view_mut().update(ViewEvent::Pointer {
                kind: mouse.kind,
                position,
            });
            if !matches!(update, ViewUpdate::Ignored) {
                return update;
            }
        }
        let context = self.active_view().context();
        match key_resolution::resolve(&self.keybinds, context, &event) {
            Some(action) => {
                let action_context = ActionContext {
                    click_position: self.content_position_for_click(&event),
                    is_mouse: matches!(event, Event::Mouse(_)),
                    movement_lines: movement_lines(
                        action,
                        &event,
                        self.scroll_lines,
                        self.content_height,
                    ),
                };
                self.active_view_mut().update(ViewEvent::Action {
                    action,
                    context: action_context,
                })
            }
            None => self.active_view_mut().update(ViewEvent::Raw(&event)),
        }
    }

    fn create_view(&self, request: ViewRequest) -> anyhow::Result<Box<dyn View>> {
        match request {
            ViewRequest::Focus {
                comparison,
                source,
                document_index,
                path,
                display_config,
            } => BrowseView::focused(
                source,
                (document_index, path),
                *display_config,
                self.history.clone(),
                &self.keybinds,
                &self.theme,
            )
            .and_then(|view| view.with_comparison(comparison.source, comparison.origin))
            .map(|view| Box::new(view) as Box<dyn View>),
            ViewRequest::Preview {
                content,
                document_index,
                path,
            } => Ok(Box::new(PreviewView::new(
                content,
                document_index,
                path,
                &self.keybinds,
                &self.theme,
            ))),
            ViewRequest::Config { content } => Ok(Box::new(ConfigView::new(
                content,
                &self.keybinds,
                &self.theme,
            ))),
            ViewRequest::Flatten { data } => Ok(Box::new(FlattenView::new(
                data,
                QueryHints::flatten(&self.keybinds),
                &self.theme,
            ))),
            ViewRequest::Help => Ok(Box::new(HelpView::new(
                &self.keybinds,
                self.scroll_lines,
                &self.theme,
            ))),
            ViewRequest::Hstr => Ok(Box::new(HstrView::new(
                &self.history,
                QueryHints::hstr(&self.keybinds),
                &self.theme,
            ))),
            ViewRequest::Jaq {
                query,
                source,
                display_config,
            } => BrowseView::jaq(
                query,
                &source,
                *display_config,
                QueryHints::jaq(&self.keybinds),
                self.history.clone(),
                &self.theme,
            )
            .map(|view| Box::new(view) as Box<dyn View>),
        }
    }

    fn active_view(&self) -> &dyn View {
        self.views.active()
    }

    fn active_view_mut(&mut self) -> &mut dyn View {
        self.views.active_mut()
    }

    fn content_position_for_click(&self, event: &Event) -> Option<ContentPosition> {
        let Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(_),
            ..
        }) = event
        else {
            return None;
        };

        self.content_position_for_mouse(event)
    }

    fn content_position_for_mouse(&self, event: &Event) -> Option<ContentPosition> {
        let Event::Mouse(MouseEvent { column, row, .. }) = event else {
            return None;
        };

        self.renderer
            .hit_test(ScreenPosition {
                row: *row,
                column: *column,
            })
            .filter(|position| position.index == Index::Content)
            .map(|position| position.content_position())
    }
}

#[cfg(test)]
mod tests {
    use std::{io, path::PathBuf};

    use promkit::core::{
        WidgetLayout, WidthMode,
        crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
        grapheme::StyledGraphemes,
    };

    use super::*;

    #[test]
    fn renders_config_load_errors_as_startup_warnings() {
        let warning = StartupWarning::ConfigLoad(ConfigLoadError::ReadFile {
            path: PathBuf::from("config.toml"),
            source: io::Error::other("invalid config"),
        });

        assert_eq!(
            warning
                .create_graphemes(&ThemeConfig::default())
                .graphemes
                .to_string(),
            "[vy:config] failed to read configuration file `config.toml`: invalid config"
        );
    }

    #[test]
    fn measures_wrapped_feedback_height() {
        let created = CreatedGraphemes {
            graphemes: StyledGraphemes::from("123456789"),
            layout: WidgetLayout {
                width_mode: WidthMode::Wrap,
                ..Default::default()
            },
            cursor: None,
        };

        assert_eq!(rendered_height(&created, 4), 3);
    }

    #[test]
    fn applies_scroll_lines_only_to_mouse_scroll_events() {
        let key = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        let scroll = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(
            movement_lines(
                Action::Component(ComponentAction::Document(DocumentAction::Down)),
                &key,
                3,
                24,
            ),
            1
        );
        assert_eq!(
            movement_lines(
                Action::Component(ComponentAction::Document(DocumentAction::Down)),
                &scroll,
                3,
                24,
            ),
            3
        );
    }

    #[test]
    fn applies_full_and_half_page_heights_to_page_actions() {
        let key = Event::Key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));

        assert_eq!(
            movement_lines(
                Action::Component(ComponentAction::Document(DocumentAction::PageDown)),
                &key,
                3,
                25,
            ),
            25
        );
        assert_eq!(
            movement_lines(
                Action::Component(ComponentAction::Document(DocumentAction::HalfPageDown)),
                &key,
                3,
                25,
            ),
            12
        );
    }
}
