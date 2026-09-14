use std::{borrow::Cow, ops::Range, sync::Arc};

use promkit::core::{
    CreatedGraphemes, Widget, WidgetLayout, WidthMode,
    crossterm::{
        event::Event as TerminalEvent,
        style::{Attribute, ContentStyle},
    },
    grapheme::StyledGraphemes,
};

use crate::{
    catalog::{FlattenData, FlattenLoadState, FlattenSubscription},
    command::grep::{Completion, Event, GrepExecutor, LineMatch},
    config::{FeedbackThemeConfig, ThemeConfig},
    utils::{normalize_lines, one_line},
    viewer::{
        Action, ComponentAction, DEBUG_LOADING_TICKS, DocumentAction, FlattenViewAction,
        QueryFocus, View, ViewAction, ViewContext, ViewEffect, ViewEvent, ViewFrame,
        ViewNotification, ViewUpdate,
        feature::{Feedback, FeedbackKind, HintFocus, QueryHintState, QueryHints},
        ui::MultiLineEditorComponent,
    },
};

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Debug, PartialEq, Eq)]
enum GrepStatus {
    Idle,
    Running,
    Cancelling,
    Completed,
    Cancelled,
    Failed(String),
}

pub(in crate::viewer) struct FlattenView {
    subscription: FlattenSubscription,
    state: FlattenLoadState,
    grep_query: String,
    grep_editor: MultiLineEditorComponent,
    grep_executor: GrepExecutor,
    grep_status: GrepStatus,
    matches: Vec<LineMatch>,
    selected: usize,
    offset: usize,
    viewport_height: usize,
    spinner_frame: usize,
    preparation_loading_ticks_remaining: usize,
    grep_loading_ticks_remaining: usize,
    focus: QueryFocus,
    hints: QueryHints,
    feedback_theme: FeedbackThemeConfig,
    match_style: ContentStyle,
}

impl FlattenView {
    pub(in crate::viewer) fn new(
        subscription: FlattenSubscription,
        hints: QueryHints,
        theme: &ThemeConfig,
    ) -> Self {
        let state = subscription.state();
        let matches = all_lines(&state);
        Self {
            subscription,
            state,
            grep_query: String::new(),
            grep_editor: MultiLineEditorComponent::new(
                ":grep ",
                "",
                theme.editor.active_char_style,
            ),
            grep_executor: GrepExecutor::default(),
            grep_status: GrepStatus::Idle,
            matches,
            selected: 0,
            offset: 0,
            viewport_height: 1,
            spinner_frame: 0,
            preparation_loading_ticks_remaining: DEBUG_LOADING_TICKS,
            grep_loading_ticks_remaining: 0,
            focus: QueryFocus::Result,
            hints,
            feedback_theme: theme.feedback.clone(),
            match_style: theme.flatten.match_style,
        }
    }

    fn is_grep_running(&self) -> bool {
        matches!(
            self.grep_status,
            GrepStatus::Running | GrepStatus::Cancelling
        )
    }

    fn is_grep_available(&self) -> bool {
        self.preparation_loading_ticks_remaining == 0
            && matches!(self.state, FlattenLoadState::Ready(_))
    }

    fn feedback(&self) -> Feedback<'_> {
        if self.preparation_loading_ticks_remaining > 0 {
            return self.preparation_loading_feedback();
        }
        let (message, hint_state, kind) = match &self.state {
            FlattenLoadState::Loading => return self.preparation_loading_feedback(),
            FlattenLoadState::Failed(error) => (
                format!("preparation failed: {}", normalize_lines(error)),
                None,
                FeedbackKind::Error,
            ),
            FlattenLoadState::Ready(data) => {
                let count = self.matches.len();
                let total = data.lines().len();
                match &self.grep_status {
                    GrepStatus::Idle => (
                        format!("{total} lines"),
                        Some(QueryHintState::Ready),
                        FeedbackKind::Normal,
                    ),
                    GrepStatus::Running => (
                        format!(
                            "{} grep running — {count}/{total} lines",
                            SPINNER_FRAMES[self.spinner_frame]
                        ),
                        Some(QueryHintState::Running),
                        FeedbackKind::Loading,
                    ),
                    GrepStatus::Cancelling => (
                        format!(
                            "{} cancelling grep — {count}/{total} lines",
                            SPINNER_FRAMES[self.spinner_frame]
                        ),
                        Some(QueryHintState::Running),
                        FeedbackKind::Warning,
                    ),
                    GrepStatus::Completed => (
                        format!("grep completed: {count}/{total} lines"),
                        Some(QueryHintState::Finished),
                        FeedbackKind::Success,
                    ),
                    GrepStatus::Cancelled => (
                        format!("grep cancelled: {count}/{total} lines"),
                        Some(QueryHintState::Finished),
                        FeedbackKind::Warning,
                    ),
                    GrepStatus::Failed(error) => (
                        format!("grep failed: {}", normalize_lines(error)),
                        Some(QueryHintState::Finished),
                        FeedbackKind::Error,
                    ),
                }
            }
        };
        let message = if self.focus == QueryFocus::Result && !self.grep_query.is_empty() {
            format!("{message} — query: {}", one_line(&self.grep_query))
        } else {
            message
        };
        Feedback::View {
            name: "flatten",
            message: Cow::Owned(message),
            hint: hint_state.map_or_else(
                || Cow::Owned(self.hints.loading()),
                |state| Cow::Owned(self.hints.create(self.hint_focus(), state)),
            ),
            kind,
        }
    }

    fn preparation_loading_feedback(&self) -> Feedback<'_> {
        Feedback::View {
            name: "flatten",
            message: Cow::Owned(format!(
                "{} preparing flatten view",
                SPINNER_FRAMES[self.spinner_frame]
            )),
            hint: Cow::Owned(self.hints.preparing()),
            kind: FeedbackKind::Loading,
        }
    }

    fn hint_focus(&self) -> HintFocus {
        match self.focus {
            QueryFocus::Editor => HintFocus::Editor,
            QueryFocus::Result => HintFocus::Result,
        }
    }

    fn create_content_graphemes(&mut self, height: u16) -> CreatedGraphemes {
        self.viewport_height = usize::from(height).max(1);
        self.reveal_selection();
        if self.preparation_loading_ticks_remaining > 0 {
            return CreatedGraphemes::default();
        }
        let FlattenLoadState::Ready(data) = &self.state else {
            return CreatedGraphemes::default();
        };
        let selected = (self.focus == QueryFocus::Result).then_some(self.selected);
        let lines = self
            .matches
            .iter()
            .skip(self.offset)
            .take(usize::from(height))
            .enumerate()
            .filter_map(|(visible_index, line_match)| {
                data.lines()
                    .get(line_match.line_index)
                    .map(|line| (visible_index, line, line_match.ranges.as_slice()))
            })
            .map(|(visible_index, line, ranges)| {
                styled_result_line(
                    line,
                    ranges,
                    self.match_style,
                    selected == Some(self.offset + visible_index),
                )
            });

        CreatedGraphemes {
            graphemes: StyledGraphemes::from_lines(lines),
            layout: WidgetLayout {
                max_height: Some(usize::from(height)),
                width_mode: WidthMode::Truncate,
                ..Default::default()
            },
            cursor: None,
        }
    }

    fn apply_movement(&mut self, action: DocumentAction, movement_lines: usize) {
        let last = self.matches.len().saturating_sub(1);
        match action {
            DocumentAction::Up | DocumentAction::PageUp | DocumentAction::HalfPageUp => {
                self.selected = self.selected.saturating_sub(movement_lines);
            }
            DocumentAction::Down | DocumentAction::PageDown | DocumentAction::HalfPageDown => {
                self.selected = self.selected.saturating_add(movement_lines).min(last);
            }
            DocumentAction::MoveToHead => self.selected = 0,
            DocumentAction::MoveToTail => self.selected = last,
            DocumentAction::Toggle
            | DocumentAction::ExpandAll
            | DocumentAction::CollapseAll
            | DocumentAction::ToggleOverflowMode
            | DocumentAction::ToggleLineNumbers => {}
        }
        self.reveal_selection();
    }

    fn reveal_selection(&mut self) {
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset.saturating_add(self.viewport_height) {
            self.offset = self
                .selected
                .saturating_add(1)
                .saturating_sub(self.viewport_height);
        }
        self.offset = self.offset.min(self.max_offset());
    }

    fn select_visible_row(&mut self, row: usize) -> bool {
        let selected = self.offset.saturating_add(row);
        if selected >= self.matches.len() {
            return false;
        }
        self.selected = selected;
        true
    }

    fn goto_selected(&self) -> Option<ViewUpdate> {
        let FlattenLoadState::Ready(data) = &self.state else {
            return None;
        };
        let line_match = self.matches.get(self.selected)?;
        let target = data.target(line_match.line_index)?;
        Some(ViewUpdate::Effect(ViewEffect::Navigate {
            document_index: target.document_index(),
            path: target.path().to_owned(),
        }))
    }

    fn max_offset(&self) -> usize {
        if self.preparation_loading_ticks_remaining > 0 {
            return 0;
        }
        self.matches.len().saturating_sub(self.viewport_height)
    }

    fn handle_input(&mut self, event: &TerminalEvent) {
        self.grep_editor.handle_input(event);
    }

    fn execute_edited_query(&mut self) {
        if !self.is_grep_available() {
            return;
        }
        let Some(data) = self.ready_data() else {
            unreachable!("grep availability requires prepared flatten data");
        };
        let query = self.grep_editor.value().trim().to_owned();
        if query.is_empty() {
            self.grep_status = GrepStatus::Failed("grep requires a query".to_owned());
            return;
        }

        self.grep_query = query;
        self.matches.clear();
        self.selected = 0;
        self.offset = 0;
        self.grep_status = GrepStatus::Running;
        self.spinner_frame = 0;
        self.grep_loading_ticks_remaining = DEBUG_LOADING_TICKS;
        self.grep_executor.execute(self.grep_query.clone(), data);
    }

    fn ready_data(&self) -> Option<Arc<FlattenData>> {
        match &self.state {
            FlattenLoadState::Ready(data) => Some(Arc::clone(data)),
            FlattenLoadState::Loading | FlattenLoadState::Failed(_) => None,
        }
    }

    fn update_preparation(&mut self) -> bool {
        if self.preparation_loading_ticks_remaining > 0 {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
            self.preparation_loading_ticks_remaining -= 1;
            if self.preparation_loading_ticks_remaining == 0 {
                let state = self.subscription.state_and_mark_seen();
                self.replace_state(state);
            }
            return true;
        }

        let mut changed = false;
        if matches!(self.state, FlattenLoadState::Loading) {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
            changed = true;
        }
        if self.subscription.has_changed() {
            let state = self.subscription.state_and_mark_seen();
            self.replace_state(state);
            changed = true;
        }
        changed
    }

    fn replace_state(&mut self, state: FlattenLoadState) {
        self.grep_executor.cancel();
        self.state = state;
        self.matches = all_lines(&self.state);
        self.grep_status = GrepStatus::Idle;
        self.selected = 0;
        self.offset = 0;
    }

    fn tick(&mut self) -> bool {
        let mut changed = self.update_preparation();
        if !self.is_grep_running() {
            return changed;
        }

        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        changed = true;
        if self.grep_loading_ticks_remaining > 0 {
            self.grep_loading_ticks_remaining -= 1;
            return changed;
        }

        let Some(Event::Finished {
            completion,
            matches,
            ..
        }) = self.grep_executor.try_recv()
        else {
            return changed;
        };
        if matches!(completion, Completion::Completed) {
            self.matches = matches;
            self.selected = 0;
            self.offset = 0;
        }
        self.grep_status = match completion {
            Completion::Completed => GrepStatus::Completed,
            Completion::Cancelled => GrepStatus::Cancelled,
            Completion::Failed(error) => GrepStatus::Failed(error),
        };
        changed
    }
}

impl Drop for FlattenView {
    fn drop(&mut self) {
        self.grep_executor.cancel();
    }
}

impl View for FlattenView {
    fn context(&self) -> ViewContext {
        ViewContext::Flatten(self.focus)
    }

    fn render(&mut self, width: u16, height: u16) -> ViewFrame {
        let editor = if self.focus == QueryFocus::Editor && self.is_grep_available() {
            self.grep_editor.create_graphemes()
        } else {
            CreatedGraphemes::default()
        };
        ViewFrame::create(
            width,
            height,
            self.feedback().create_graphemes(&self.feedback_theme),
            CreatedGraphemes::default(),
            editor,
            |content_height| self.create_content_graphemes(content_height),
        )
    }

    fn notify(&mut self, _notification: ViewNotification) {}

    fn update(&mut self, input: ViewEvent<'_>) -> ViewUpdate {
        match input {
            ViewEvent::Action {
                action: Action::View(ViewAction::Flatten(action)),
                context,
            } => match action {
                FlattenViewAction::Back => ViewUpdate::Effect(ViewEffect::Back),
                FlattenViewAction::CancelExecution if self.is_grep_running() => {
                    self.grep_executor.cancel();
                    self.grep_status = GrepStatus::Cancelling;
                    ViewUpdate::Render
                }
                FlattenViewAction::CancelExecution => ViewUpdate::Handled,
                FlattenViewAction::ExecuteQuery => {
                    self.execute_edited_query();
                    ViewUpdate::Render
                }
                FlattenViewAction::OpenQueryEditor => {
                    self.focus = QueryFocus::Editor;
                    ViewUpdate::Render
                }
                FlattenViewAction::CloseQueryEditor => {
                    self.focus = QueryFocus::Result;
                    ViewUpdate::Render
                }
                FlattenViewAction::GotoSelection => {
                    if context.is_mouse {
                        let Some(position) = context.click_position else {
                            return ViewUpdate::Handled;
                        };
                        if !self.select_visible_row(position.row) {
                            return ViewUpdate::Handled;
                        }
                    }
                    self.goto_selected().unwrap_or(ViewUpdate::Handled)
                }
            },
            ViewEvent::Action {
                action: Action::Component(ComponentAction::TextEditor(action)),
                ..
            } => {
                self.grep_editor.apply_action(action);
                ViewUpdate::Render
            }
            ViewEvent::Action {
                action: Action::Component(ComponentAction::Document(action)),
                context,
            } => {
                self.apply_movement(action, context.movement_lines);
                ViewUpdate::Render
            }
            ViewEvent::Raw(event)
                if self.focus == QueryFocus::Editor && self.is_grep_available() =>
            {
                self.handle_input(event);
                ViewUpdate::Render
            }
            ViewEvent::Tick => {
                if self.tick() {
                    ViewUpdate::Render
                } else {
                    ViewUpdate::Ignored
                }
            }
            ViewEvent::Action { .. } | ViewEvent::Pointer { .. } | ViewEvent::Raw(_) => {
                ViewUpdate::Ignored
            }
        }
    }
}

fn styled_result_line(
    line: &str,
    ranges: &[Range<usize>],
    match_style: ContentStyle,
    selected: bool,
) -> StyledGraphemes {
    let mut normal_style = ContentStyle::default();
    let mut match_style = match_style;
    if selected {
        normal_style.attributes.set(Attribute::Reverse);
        match_style.attributes.set(Attribute::Reverse);
    }

    let mut cursor = 0;
    let mut segments = Vec::with_capacity(ranges.len().saturating_mul(2).saturating_add(1));
    for range in ranges {
        if cursor < range.start {
            segments.push(StyledGraphemes::from_str(
                &line[cursor..range.start],
                normal_style,
            ));
        }
        segments.push(StyledGraphemes::from_str(&line[range.clone()], match_style));
        cursor = range.end;
    }
    if cursor < line.len() {
        segments.push(StyledGraphemes::from_str(&line[cursor..], normal_style));
    }
    StyledGraphemes::from_iter(segments)
}

fn all_lines(state: &FlattenLoadState) -> Vec<LineMatch> {
    match state {
        FlattenLoadState::Ready(data) => (0..data.lines().len())
            .map(|line_index| LineMatch {
                line_index,
                ranges: Vec::new(),
            })
            .collect(),
        FlattenLoadState::Loading | FlattenLoadState::Failed(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, time::Duration};

    use promkit::core::{
        ContentPosition,
        crossterm::style::{Attribute, Attributes},
    };
    use tokio::time::{sleep, timeout};

    use crate::{
        catalog::{CatalogLoader, FlattenLoader},
        config::{Config, Keybinds},
        document_source::DocumentSource,
        input_format::InputFormat,
    };

    use super::*;

    fn action(action: Action) -> ViewEvent<'static> {
        ViewEvent::Action {
            action,
            context: crate::viewer::ActionContext {
                click_position: None,
                is_mouse: false,
                movement_lines: 1,
            },
        }
    }

    fn click_action(action: Action, row: usize) -> ViewEvent<'static> {
        ViewEvent::Action {
            action,
            context: crate::viewer::ActionContext {
                click_position: Some(ContentPosition { row, column: 0 }),
                is_mouse: true,
                movement_lines: 1,
            },
        }
    }

    fn outside_click_action(action: Action) -> ViewEvent<'static> {
        ViewEvent::Action {
            action,
            context: crate::viewer::ActionContext {
                click_position: None,
                is_mouse: true,
                movement_lines: 1,
            },
        }
    }

    fn ready_view() -> FlattenView {
        view(FlattenSubscription::ready_for_test([
            "@0.name = \"Alice\"",
            "@0.role = \"admin\"",
            "@1.name = \"Bob\"",
        ]))
    }

    fn view(subscription: FlattenSubscription) -> FlattenView {
        FlattenView::new(
            subscription,
            QueryHints::flatten(&Keybinds::default()),
            &Config::default().theme,
        )
    }

    async fn wait_for_grep(view: &mut FlattenView) {
        timeout(Duration::from_secs(2), async {
            while view.is_grep_running() {
                view.tick();
                sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("grep view timeout");
    }

    fn matching_line_indices(view: &FlattenView) -> Vec<usize> {
        view.matches.iter().map(|line| line.line_index).collect()
    }

    #[test]
    fn displays_prepared_flatten_lines() {
        let mut view = ready_view();

        assert_eq!(
            view.create_content_graphemes(20).graphemes.to_string(),
            "@0.name = \"Alice\"\n@0.role = \"admin\"\n@1.name = \"Bob\""
        );
        assert_eq!(
            view.grep_editor.create_graphemes().graphemes.to_string(),
            ":grep  "
        );
        assert_eq!(
            view.grep_editor.active_char_style(),
            Config::default().theme.editor.active_char_style
        );
    }

    #[tokio::test]
    async fn grep_runs_only_after_enter_and_supports_and_or() {
        let mut view = ready_view();
        view.handle_input(&TerminalEvent::Paste("@0&Alice|Bob".into()));

        assert_eq!(matching_line_indices(&view), [0, 1, 2]);
        assert_eq!(view.grep_status, GrepStatus::Idle);

        view.update(action(Action::View(ViewAction::Flatten(
            FlattenViewAction::ExecuteQuery,
        ))));
        assert_eq!(view.grep_status, GrepStatus::Running);
        assert!(
            view.feedback()
                .create_graphemes(&view.feedback_theme)
                .graphemes
                .to_string()
                .contains("grep running")
        );

        wait_for_grep(&mut view).await;

        assert_eq!(view.grep_status, GrepStatus::Completed);
        assert_eq!(matching_line_indices(&view), [0, 2]);
        assert!(
            view.feedback()
                .create_graphemes(&view.feedback_theme)
                .graphemes
                .to_string()
                .contains("grep completed: 2/3 lines — query: @0&Alice|Bob")
        );
        assert_eq!(
            view.create_content_graphemes(20).graphemes.to_string(),
            "@0.name = \"Alice\"\n@1.name = \"Bob\""
        );
        view.update(action(Action::Component(ComponentAction::Document(
            DocumentAction::Down,
        ))));
        assert!(matches!(
            view.update(action(Action::View(ViewAction::Flatten(
                FlattenViewAction::GotoSelection
            )))),
            ViewUpdate::Effect(ViewEffect::Navigate {
                document_index: 1,
                ref path,
            }) if path == ".name"
        ));
    }

    #[tokio::test]
    async fn accepts_edits_and_a_new_query_while_grep_is_running() {
        let mut view = ready_view();
        view.update(action(Action::View(ViewAction::Flatten(
            FlattenViewAction::OpenQueryEditor,
        ))));
        view.handle_input(&TerminalEvent::Paste("Alice".into()));
        view.execute_edited_query();

        view.update(action(Action::Component(ComponentAction::TextEditor(
            crate::viewer::TextEditorAction::EraseAll,
        ))));
        view.update(ViewEvent::Raw(&TerminalEvent::Paste("Bob".into())));
        assert_eq!(view.grep_status, GrepStatus::Running);
        view.update(action(Action::View(ViewAction::Flatten(
            FlattenViewAction::ExecuteQuery,
        ))));

        wait_for_grep(&mut view).await;

        assert_eq!(matching_line_indices(&view), [2]);
        assert_eq!(
            view.grep_editor.create_graphemes().graphemes.to_string(),
            ":grep Bob "
        );
    }

    #[tokio::test]
    async fn cancels_a_running_grep() {
        let mut view = ready_view();
        view.handle_input(&TerminalEvent::Paste("Alice".into()));
        view.execute_edited_query();

        view.update(action(Action::View(ViewAction::Flatten(
            FlattenViewAction::CancelExecution,
        ))));

        assert_eq!(view.grep_status, GrepStatus::Cancelling);
        assert!(
            view.feedback()
                .create_graphemes(&view.feedback_theme)
                .graphemes
                .to_string()
                .contains("cancelling grep")
        );
    }

    #[test]
    fn highlights_matches_while_preserving_the_selected_line_style() {
        let match_style = Config::default().theme.flatten.match_style;
        let mut selected_match_style = match_style;
        selected_match_style.attributes.set(Attribute::Reverse);
        let selected_style = ContentStyle {
            attributes: Attributes::from(Attribute::Reverse),
            ..Default::default()
        };

        assert_eq!(
            styled_result_line("name = Alice", &[0..4, 7..12], match_style, true),
            StyledGraphemes::from_iter([
                StyledGraphemes::from_str("name", selected_match_style),
                StyledGraphemes::from_str(" = ", selected_style),
                StyledGraphemes::from_str("Alice", selected_match_style),
            ])
        );
    }

    #[test]
    fn opens_and_closes_the_query_editor_before_returning() {
        use promkit::core::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut view = ready_view();
        let press = |view: &mut FlattenView, code| {
            let event = TerminalEvent::Key(KeyEvent::new(code, KeyModifiers::NONE));
            match crate::viewer::key_resolution::resolve(
                &Keybinds::default(),
                view.context(),
                &event,
            ) {
                Some(resolved) => view.update(action(resolved)),
                None => view.update(ViewEvent::Raw(&event)),
            }
        };
        assert_eq!(view.context(), ViewContext::Flatten(QueryFocus::Result));
        assert!(view.render(80, 20).editor.graphemes.is_empty());

        press(&mut view, KeyCode::Char(':'));
        assert_eq!(view.context(), ViewContext::Flatten(QueryFocus::Editor));
        press(&mut view, KeyCode::Char(':'));
        assert_eq!(view.grep_editor.value(), ":");
        press(&mut view, KeyCode::Tab);
        assert_eq!(view.context(), ViewContext::Flatten(QueryFocus::Editor));
        assert!(matches!(press(&mut view, KeyCode::Esc), ViewUpdate::Render));
        assert_eq!(view.context(), ViewContext::Flatten(QueryFocus::Result));
        assert!(view.render(80, 20).editor.graphemes.is_empty());
        press(&mut view, KeyCode::Char(':'));
        assert_eq!(view.grep_editor.value(), ":");
        press(&mut view, KeyCode::Esc);
        assert!(matches!(
            press(&mut view, KeyCode::Esc),
            ViewUpdate::Effect(ViewEffect::Back)
        ));
    }

    #[test]
    fn navigates_from_the_selected_result_with_enter_or_click() {
        let mut view = ready_view();
        view.create_content_graphemes(20);

        view.update(action(Action::Component(ComponentAction::Document(
            DocumentAction::Down,
        ))));
        assert_eq!(view.selected, 1);
        assert!(matches!(
            view.update(action(Action::View(ViewAction::Flatten(
                FlattenViewAction::GotoSelection
            )))),
            ViewUpdate::Effect(ViewEffect::Navigate {
                document_index: 0,
                ref path,
            }) if path == ".role"
        ));

        assert!(matches!(
            view.update(click_action(
                Action::View(ViewAction::Flatten(FlattenViewAction::GotoSelection)),
                2,
            )),
            ViewUpdate::Effect(ViewEffect::Navigate {
                document_index: 1,
                ref path,
            }) if path == ".name"
        ));
        assert_eq!(view.selected, 2);

        assert!(matches!(
            view.update(outside_click_action(Action::View(ViewAction::Flatten(
                FlattenViewAction::GotoSelection,
            )))),
            ViewUpdate::Handled
        ));
    }

    #[test]
    fn displays_a_spinner_while_flatten_data_is_loading() {
        let mut view = view(FlattenSubscription::from_state(FlattenLoadState::Loading));
        let before = view
            .feedback()
            .create_graphemes(&view.feedback_theme)
            .graphemes
            .to_string();

        assert!(matches!(view.update(ViewEvent::Tick), ViewUpdate::Render));
        let after = view
            .feedback()
            .create_graphemes(&view.feedback_theme)
            .graphemes
            .to_string();

        assert!(before.contains("preparing flatten view"));
        assert_ne!(before, after);
    }

    #[test]
    fn switches_from_loading_to_prepared_data_on_tick() {
        let (sender, subscription) =
            FlattenSubscription::channel_for_test(FlattenLoadState::Loading);
        let mut view = view(subscription);
        let ready = FlattenSubscription::ready_for_test(["@0 = true"]).state();
        sender.send_replace(ready);

        assert!(matches!(view.update(ViewEvent::Tick), ViewUpdate::Render));
        assert_eq!(
            view.create_content_graphemes(20).graphemes.to_string(),
            "@0 = true"
        );
    }

    #[tokio::test]
    async fn opens_data_prepared_by_the_background_loader() {
        let source = DocumentSource::from_reader(
            InputFormat::Json,
            Box::new(Cursor::new(br#"{"ready":true}"#.to_vec())),
        )
        .unwrap();
        let mut subscription = FlattenLoader::spawn(CatalogLoader::spawn(source));
        while matches!(subscription.state(), FlattenLoadState::Loading) {
            subscription.changed().await.unwrap();
        }
        let mut view = view(subscription);

        assert_eq!(
            view.create_content_graphemes(20).graphemes.to_string(),
            "@0 = {}\n@0.ready = true"
        );
    }
}
