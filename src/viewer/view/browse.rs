use std::borrow::Cow;

use crate::{
    catalog::{
        CatalogLoader, FlattenLoadState, FlattenLoader, FlattenSubscription, RelativeTarget,
    },
    command::argument::document_path::PathCompleter,
    config::{FeedbackThemeConfig, Keybinds, ThemeConfig},
    document_source::DocumentSource,
    history::History,
    viewer::{
        Action, BrowseFocus, BrowseViewAction, CommandAction, ComponentAction, JaqViewAction, View,
        ViewAction, ViewContext, ViewEffect, ViewEvent, ViewFrame, ViewNotification, ViewRequest,
        ViewUpdate,
        feature::{
            CommandSession, CommandTarget, Feedback, FeedbackKind, HintFocus, QueryHints,
            SearchComponent, SearchOutcome, format_bindings,
            jaq::{JaqSession, JaqStatus},
        },
        ui::{
            ComparisonPane, DocumentComparisonComponent, DocumentComponent, DocumentDisplayConfig,
            DocumentFocus,
        },
    },
};

/// Data and operations belonging to one immutable document snapshot.
pub(in crate::viewer) struct BrowsePane {
    origin: Option<(usize, String)>,
    document: DocumentComponent,
    document_source: DocumentSource,
    result_display_config: DocumentDisplayConfig,
    command: CommandSession,
    search: SearchComponent,
    navigation: FlattenSubscription,
    focus: BrowseFocus,
}

impl ComparisonPane for BrowsePane {
    fn document(&self) -> &DocumentComponent {
        &self.document
    }
    fn document_mut(&mut self) -> &mut DocumentComponent {
        &mut self.document
    }
}

impl BrowsePane {
    fn new(
        document: DocumentComponent,
        source: DocumentSource,
        completer: PathCompleter,
        navigation: FlattenSubscription,
        history: History,
        theme: &ThemeConfig,
    ) -> Self {
        Self {
            origin: None,
            result_display_config: document.display_config(),
            document,
            document_source: source,
            command: CommandSession::new(completer, history.clone(), theme),
            search: SearchComponent::new(
                navigation.clone(),
                theme.editor.active_char_style,
                history,
            ),
            navigation,
            focus: BrowseFocus::Document,
        }
    }

    fn from_source(
        source: DocumentSource,
        display: DocumentDisplayConfig,
        history: History,
        theme: &ThemeConfig,
    ) -> anyhow::Result<Self> {
        let document = DocumentComponent::parse(source.content(), display)?;
        let catalog = CatalogLoader::spawn(source.clone());
        let flatten = FlattenLoader::spawn(catalog.clone());
        Ok(Self::new(
            document,
            source,
            PathCompleter::new(catalog),
            flatten,
            history,
            theme,
        ))
    }

    fn replace_source(&mut self, source: DocumentSource, reset: bool) -> anyhow::Result<()> {
        let selected = (!reset).then(|| self.document.selected_path()).flatten();
        let mut document =
            DocumentComponent::parse(source.content(), self.document.display_config())?;
        if let Some((index, path)) = selected {
            document.move_to_path(index, &path);
        }
        let catalog = CatalogLoader::spawn(source.clone());
        let navigation = FlattenLoader::spawn(catalog.clone());
        self.command.replace_completer(PathCompleter::new(catalog));
        self.search.replace_source(navigation.clone());
        self.navigation = navigation;
        self.document = document;
        self.document_source = source;
        Ok(())
    }

    fn update_config(&mut self, update: &std::sync::Arc<crate::viewer::ConfigUpdate>) {
        self.result_display_config =
            DocumentDisplayConfig::from_config(self.document_source.format(), &update.config);
        self.document
            .apply_display_config(self.result_display_config.clone());
        self.search
            .set_active_char_style(update.config.theme.editor.active_char_style);
        self.notify_command(&ViewNotification::ConfigUpdated(update.clone()));
    }

    fn update_command(&mut self, input: &ViewEvent<'_>) -> ViewUpdate {
        self.command.update(
            input,
            CommandTarget {
                document: &mut self.document,
                source: &self.document_source,
                flatten: &self.navigation,
                display_config: &self.result_display_config,
            },
        )
    }
    fn notify_command(&mut self, notification: &ViewNotification) {
        self.command.notify(
            notification,
            CommandTarget {
                document: &mut self.document,
                source: &self.document_source,
                flatten: &self.navigation,
                display_config: &self.result_display_config,
            },
        );
    }
    fn apply_search_outcome(&mut self, outcome: SearchOutcome) -> ViewUpdate {
        match outcome {
            SearchOutcome::Editing | SearchOutcome::Failed => ViewUpdate::Render,
            SearchOutcome::Cancelled | SearchOutcome::NotFound => {
                self.focus = BrowseFocus::Document;
                ViewUpdate::Render
            }
            SearchOutcome::Moved {
                document_index,
                path,
            } => {
                self.focus = BrowseFocus::Document;
                self.document.move_to_path(document_index, &path);
                ViewUpdate::Render
            }
        }
    }

    fn move_to_relative(&mut self, direction: RelativeTarget) -> ViewUpdate {
        let Some((document_index, path)) = self.document.selected_path() else {
            return ViewUpdate::Handled;
        };
        let FlattenLoadState::Ready(data) = self.navigation.state_and_mark_seen() else {
            return ViewUpdate::Handled;
        };
        if let Some(target) = data.relative_target(document_index, &path, direction) {
            self.document
                .move_to_path(target.document_index(), target.path());
            ViewUpdate::Render
        } else {
            ViewUpdate::Handled
        }
    }

    fn collapse_or_move_to_parent(&mut self) -> ViewUpdate {
        if self.document.collapse_selected() {
            ViewUpdate::Render
        } else {
            self.move_to_relative(RelativeTarget::Parent)
        }
    }

    fn expand_or_move_to_first_child(&mut self) -> ViewUpdate {
        if self.document.expand_selected() {
            ViewUpdate::Render
        } else {
            self.move_to_relative(RelativeTarget::FirstChild)
        }
    }
}

enum BrowseMode {
    Input,
    Focused,
    Jaq(Box<JaqBrowseState>),
}

/// State that only exists while browsing jaq results.
struct JaqBrowseState {
    session: JaqSession,
    pending_output: bool,
    reset_result: bool,
}

/// Shared browser for original input, focused subtrees, and streaming jaq results.
pub(in crate::viewer) struct BrowseView {
    documents: DocumentComparisonComponent<BrowsePane>,
    mode: BrowseMode,
    back_hint: String,
    feedback_theme: FeedbackThemeConfig,
    history: History,
    theme: ThemeConfig,
}

impl BrowseView {
    pub(in crate::viewer) fn new(
        document: DocumentComponent,
        source: DocumentSource,
        completer: PathCompleter,
        flatten: FlattenSubscription,
        history: History,
        theme: &ThemeConfig,
    ) -> Self {
        let pane = BrowsePane::new(document, source, completer, flatten, history.clone(), theme);
        Self {
            documents: DocumentComparisonComponent::with_panes(None, Some(pane), "input", "result"),
            mode: BrowseMode::Input,
            back_hint: String::new(),
            feedback_theme: theme.feedback.clone(),
            history,
            theme: theme.clone(),
        }
    }

    pub(in crate::viewer) fn focused(
        source: DocumentSource,
        origin: (usize, String),
        display_config: DocumentDisplayConfig,
        history: History,
        keybinds: &Keybinds,
        theme: &ThemeConfig,
    ) -> anyhow::Result<Self> {
        let mut pane = BrowsePane::from_source(source, display_config, history.clone(), theme)?;
        pane.origin = Some(origin);
        Ok(Self {
            documents: DocumentComparisonComponent::with_panes(None, Some(pane), "input", "focus"),
            mode: BrowseMode::Focused,
            back_hint: Self::back_hint(keybinds),
            feedback_theme: theme.feedback.clone(),
            history,
            theme: theme.clone(),
        })
    }

    pub(in crate::viewer) fn with_comparison(
        mut self,
        input: DocumentSource,
        origin: Option<(usize, String)>,
    ) -> anyhow::Result<Self> {
        let display = self.pane().result_display_config.clone();
        let mut left = BrowsePane::from_source(input, display, self.history.clone(), &self.theme)?;
        left.origin = origin;
        self.documents.set_left(left);
        Ok(self)
    }

    pub(in crate::viewer) fn jaq(
        query: String,
        source: &DocumentSource,
        display_config: DocumentDisplayConfig,
        hints: QueryHints,
        history: History,
        theme: &ThemeConfig,
    ) -> anyhow::Result<Self> {
        let session = JaqSession::start(query, source, display_config.clone(), hints)?;
        let left = BrowsePane::from_source(
            source.clone(),
            display_config.clone(),
            history.clone(),
            theme,
        )?;
        let empty = DocumentSource::from_reader(
            source.format(),
            Box::new(std::io::Cursor::new(Vec::<u8>::new())),
        )?;
        let right = BrowsePane::from_source(empty, display_config, history.clone(), theme)?;
        Ok(Self {
            documents: DocumentComparisonComponent::with_panes(
                Some(left),
                Some(right),
                "input",
                "jaq",
            ),
            mode: BrowseMode::Jaq(Box::new(JaqBrowseState {
                session,
                pending_output: false,
                reset_result: false,
            })),
            back_hint: String::new(),
            feedback_theme: theme.feedback.clone(),
            history,
            theme: theme.clone(),
        })
    }

    fn pane(&self) -> &BrowsePane {
        self.documents.active().expect("browser has a result pane")
    }
    fn pane_mut(&mut self) -> &mut BrowsePane {
        self.documents
            .active_mut()
            .expect("browser has a result pane")
    }
    fn back_hint(keybinds: &Keybinds) -> String {
        format!(
            "{}: return — {}: toggle input — {}: switch focus",
            format_bindings(&keybinds.view.browse.focus.back),
            format_bindings(&keybinds.view.browse.toggle_side_by_side),
            format_bindings(&keybinds.view.browse.toggle_focus)
        )
    }

    fn feedback(&self) -> Option<Feedback<'_>> {
        let pane = self.pane();
        pane.command
            .feedback()
            .or_else(|| {
                pane.search
                    .feedback()
                    .map(|(message, is_error)| Feedback::View {
                        name: "search",
                        message: Cow::Owned(message),
                        hint: Cow::Borrowed("n: next — N: previous"),
                        kind: if is_error {
                            FeedbackKind::Error
                        } else {
                            FeedbackKind::Success
                        },
                    })
            })
            .or_else(|| {
                let BrowseMode::Jaq(state) = &self.mode else {
                    return None;
                };
                Some(state.session.feedback(if pane.command.is_active() {
                    HintFocus::Editor
                } else if self.documents.is_side_by_side()
                    && self.documents.focus() == DocumentFocus::Left
                {
                    HintFocus::Input
                } else {
                    HintFocus::Result
                }))
            })
            .or_else(|| {
                self.documents
                    .right()
                    .and_then(|pane| pane.origin.as_ref())
                    .map(|(index, path)| Feedback::View {
                        name: "focus",
                        message: Cow::Owned(format!("@{index} {path}")),
                        hint: Cow::Borrowed(&self.back_hint),
                        kind: FeedbackKind::Normal,
                    })
            })
    }

    fn is_running(&self) -> bool {
        matches!(&self.mode, BrowseMode::Jaq(state) if state.session.is_running())
    }

    fn needs_tick(&self) -> bool {
        self.is_running() || matches!(&self.mode, BrowseMode::Jaq(state) if state.pending_output)
    }

    fn tick(&mut self) {
        let BrowseMode::Jaq(state) = &mut self.mode else {
            return;
        };
        state.pending_output |= state.session.tick();
        // Keep the displayed source, cursor and completion indexes stable while editing.
        let pane = self.documents.active().expect("browser has a result pane");
        if state.pending_output && !pane.command.is_active() && pane.focus == BrowseFocus::Document
        {
            let pane = self.documents.right_mut().expect("jaq has a result pane");
            let source = DocumentSource::from_reader(
                pane.result_display_config.format(),
                Box::new(std::io::Cursor::new(state.session.output.clone())),
            );
            let result = source.and_then(|source| pane.replace_source(source, state.reset_result));
            match result {
                Ok(()) => {
                    state.reset_result = false;
                }
                Err(error) => {
                    state.session.cancel();
                    state.session.status = JaqStatus::Failed(error.to_string());
                }
            }
            state.pending_output = false;
        }
    }

    fn execute_query(&mut self, query: String) {
        if let BrowseMode::Jaq(state) = &mut self.mode {
            state.session.execute(query);
            state.pending_output = true;
            state.reset_result = true;
            self.pane_mut().command.finish_command();
            self.pane_mut().focus = BrowseFocus::Document;
            self.documents.focus_right();
            self.pane_mut().command.clear_feedback();
            self.tick();
        }
    }
}

impl View for BrowseView {
    fn context(&self) -> ViewContext {
        if self.pane().command.is_active() {
            ViewContext::CommandEditor
        } else {
            match &self.mode {
                BrowseMode::Input => ViewContext::Input(self.pane().focus),
                BrowseMode::Focused => ViewContext::Focus(self.pane().focus),
                BrowseMode::Jaq(_) if self.pane().focus == BrowseFocus::SearchEditor => {
                    ViewContext::Focus(BrowseFocus::SearchEditor)
                }
                BrowseMode::Jaq(_) => ViewContext::Jaq,
            }
        }
    }

    fn render(&mut self, width: u16, height: u16) -> ViewFrame {
        self.documents.pin_focus(
            self.pane().command.is_active() || self.pane().focus == BrowseFocus::SearchEditor,
        );
        self.documents.prepare_render(width);
        let feedback = self
            .feedback()
            .map(|f| f.create_graphemes(&self.feedback_theme))
            .unwrap_or_default();
        let pane = self.pane();
        let (suggestions, editor) = if pane.command.is_active() {
            pane.command.render(width)
        } else if pane.focus == BrowseFocus::SearchEditor {
            (
                Default::default(),
                promkit::core::Widget::create_graphemes(&pane.search),
            )
        } else {
            Default::default()
        };
        ViewFrame::create(width, height, feedback, suggestions, editor, |height| {
            self.documents.render(width, height)
        })
    }

    fn notify(&mut self, notification: ViewNotification) {
        if let ViewNotification::ConfigUpdated(update) = &notification {
            self.back_hint = Self::back_hint(&update.config.keybinds);
            self.theme = update.config.theme.clone();
            self.feedback_theme = self.theme.feedback.clone();
            for pane in self.documents.panes_mut() {
                pane.update_config(update);
            }
            if let BrowseMode::Jaq(state) = &mut self.mode {
                state.session.update_config(&update.config);
            }
            return;
        }
        self.pane_mut().notify_command(&notification);
        self.pane_mut().focus = BrowseFocus::Document;
    }

    fn update(&mut self, input: ViewEvent<'_>) -> ViewUpdate {
        if matches!(input, ViewEvent::Tick) && self.needs_tick() {
            self.tick();
            return match self.pane_mut().update_command(&input) {
                ViewUpdate::Ignored | ViewUpdate::Handled => ViewUpdate::Render,
                update => update,
            };
        }
        if let BrowseMode::Jaq(state) = &self.mode
            && matches!(
                input,
                ViewEvent::Action {
                    action: Action::Component(ComponentAction::Command(CommandAction::Open)),
                    ..
                }
            )
        {
            let command = state.session.command_line();
            self.notify(ViewNotification::RecallCommand(command));
            return ViewUpdate::Render;
        }
        let mut update = self.pane_mut().update_command(&input);
        if !matches!(update, ViewUpdate::Ignored) {
            if let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Focus { comparison, .. })) =
                &mut update
            {
                comparison.origin = self.pane().origin.clone();
            }
            if let ViewUpdate::Effect(ViewEffect::Open(
                ViewRequest::Focus {
                    document_index,
                    path,
                    ..
                }
                | ViewRequest::Preview {
                    document_index,
                    path,
                    ..
                },
            )) = &mut update
                && let Some((index, parent)) = &self.pane().origin
            {
                *document_index = *index;
                *path = if path == "." {
                    parent.clone()
                } else if parent == "." {
                    path.clone()
                } else {
                    format!("{parent}{path}")
                };
            }
            if let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq { query, .. })) =
                &mut update
                && matches!(self.mode, BrowseMode::Jaq(_))
            {
                self.execute_query(std::mem::take(query));
                return ViewUpdate::Render;
            }
            self.pane_mut().focus = BrowseFocus::Document;
            return update;
        }
        match input {
            ViewEvent::Action {
                action: Action::View(ViewAction::Jaq(action)),
                ..
            } => {
                match action {
                    JaqViewAction::Back => return ViewUpdate::Effect(ViewEffect::Back),
                    JaqViewAction::CancelExecution => {
                        if let BrowseMode::Jaq(state) = &mut self.mode
                            && state.session.is_running()
                        {
                            state.session.cancel();
                            state.session.status = JaqStatus::Cancelling;
                        }
                    }
                    JaqViewAction::ToggleSideBySide => self.documents.toggle_side_by_side(),
                    JaqViewAction::ToggleFocus => match self.documents.focus() {
                        DocumentFocus::Left => self.documents.focus_right(),
                        DocumentFocus::Right => {
                            self.documents.focus_left();
                        }
                    },
                }
                ViewUpdate::Render
            }
            ViewEvent::Action {
                action: Action::View(ViewAction::Browse(action)),
                ..
            } => match action {
                BrowseViewAction::Leave => match self.mode {
                    BrowseMode::Input => ViewUpdate::Effect(ViewEffect::Exit),
                    BrowseMode::Focused | BrowseMode::Jaq(_) => {
                        ViewUpdate::Effect(ViewEffect::Back)
                    }
                },
                BrowseViewAction::ToggleSideBySide => {
                    self.documents.toggle_side_by_side();
                    ViewUpdate::Render
                }
                BrowseViewAction::ToggleFocus => {
                    match self.documents.focus() {
                        DocumentFocus::Left => self.documents.focus_right(),
                        DocumentFocus::Right => {
                            self.documents.focus_left();
                        }
                    };
                    ViewUpdate::Render
                }
                BrowseViewAction::OpenSearchEditor => {
                    let pane = self.pane_mut();
                    pane.command.clear_feedback();
                    pane.search.enter();
                    pane.focus = BrowseFocus::SearchEditor;
                    ViewUpdate::Render
                }
                BrowseViewAction::CloseSearchEditor => {
                    let pane = self.pane_mut();
                    let outcome = pane.search.cancel();
                    pane.apply_search_outcome(outcome)
                }
                BrowseViewAction::SubmitSearch => {
                    let pane = self.pane_mut();
                    let current = pane.document.selected_path();
                    let outcome = pane
                        .search
                        .submit(current.as_ref().map(|(i, p)| (*i, p.as_str())));
                    pane.apply_search_outcome(outcome)
                }
                BrowseViewAction::OlderSearchHistory | BrowseViewAction::NewerSearchHistory => {
                    let older = matches!(action, BrowseViewAction::OlderSearchHistory);
                    let pane = self.pane_mut();
                    let outcome = pane.search.recall_history(older);
                    pane.apply_search_outcome(outcome)
                }
                BrowseViewAction::NextSearch => {
                    let pane = self.pane_mut();
                    let outcome = pane.search.next();
                    pane.apply_search_outcome(outcome)
                }
                BrowseViewAction::PreviousSearch => {
                    let pane = self.pane_mut();
                    let outcome = pane.search.previous();
                    pane.apply_search_outcome(outcome)
                }
                BrowseViewAction::CollapseOrMoveToParent => {
                    self.pane_mut().collapse_or_move_to_parent()
                }
                BrowseViewAction::MoveToParent => {
                    self.pane_mut().move_to_relative(RelativeTarget::Parent)
                }
                BrowseViewAction::ExpandOrMoveToFirstChild => {
                    self.pane_mut().expand_or_move_to_first_child()
                }
                BrowseViewAction::MoveToNextSibling => self
                    .pane_mut()
                    .move_to_relative(RelativeTarget::NextSibling),
                BrowseViewAction::MoveToPreviousSibling => self
                    .pane_mut()
                    .move_to_relative(RelativeTarget::PreviousSibling),
            },
            ViewEvent::Action {
                action: Action::Component(ComponentAction::Document(action)),
                context,
            } => {
                self.documents
                    .apply_action(action, context.click_position, context.movement_lines);
                ViewUpdate::Render
            }
            ViewEvent::Action {
                action: Action::Component(ComponentAction::TextEditor(action)),
                ..
            } if self.pane().focus == BrowseFocus::SearchEditor => {
                let pane = self.pane_mut();
                let outcome = pane.search.apply_text_editor_action(action);
                pane.apply_search_outcome(outcome)
            }
            ViewEvent::Raw(event) if self.pane().focus == BrowseFocus::SearchEditor => {
                let pane = self.pane_mut();
                let outcome = pane.search.handle_input(event);
                pane.apply_search_outcome(outcome)
            }
            _ => ViewUpdate::Ignored,
        }
    }
}

// Test helpers keep the existing browser behavior assertions focused on the active pane.
#[cfg(test)]
impl std::ops::Deref for BrowseView {
    type Target = BrowsePane;
    fn deref(&self) -> &BrowsePane {
        self.pane()
    }
}
#[cfg(test)]
impl std::ops::DerefMut for BrowseView {
    fn deref_mut(&mut self) -> &mut BrowsePane {
        self.pane_mut()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use promkit::core::crossterm::event::Event;

    use crate::{config::Config, document_source::DocumentSource, input_format::InputFormat};

    use super::*;
    use crate::viewer::{ActionContext, ViewRequest};
    use crate::{command::copy::Target as CopyTarget, viewer::CommandAction};

    fn input_view_from(format: InputFormat, input: &[u8]) -> BrowseView {
        input_view_with_search(format, input, std::iter::empty::<String>())
    }

    fn input_view_with_search(
        format: InputFormat,
        input: &[u8],
        search_lines: impl IntoIterator<Item = impl Into<String>>,
    ) -> BrowseView {
        let source =
            DocumentSource::from_reader(format, Box::new(Cursor::new(input.to_vec()))).unwrap();
        let display_config = DocumentDisplayConfig::from_config(format, &Config::default());
        let document = DocumentComponent::parse(source.content(), display_config).unwrap();
        BrowseView::new(
            document,
            source,
            PathCompleter::empty(),
            FlattenSubscription::ready_for_test(search_lines),
            History::memory(),
            &Config::default().theme,
        )
    }

    fn input_view() -> BrowseView {
        input_view_from(InputFormat::Json, br#"{"value": 1}"#)
    }

    fn view_action(action: BrowseViewAction) -> ViewEvent<'static> {
        ViewEvent::Action {
            action: Action::View(ViewAction::Browse(action)),
            context: ActionContext {
                click_position: None,
                is_mouse: false,
                movement_lines: 1,
            },
        }
    }

    fn command_action(action: CommandAction) -> ViewEvent<'static> {
        ViewEvent::Action {
            action: Action::Component(ComponentAction::Command(action)),
            context: ActionContext {
                click_position: None,
                is_mouse: false,
                movement_lines: 1,
            },
        }
    }

    fn document_action(action: crate::viewer::DocumentAction) -> ViewEvent<'static> {
        ViewEvent::Action {
            action: Action::Component(ComponentAction::Document(action)),
            context: ActionContext {
                click_position: None,
                is_mouse: false,
                movement_lines: 1,
            },
        }
    }

    #[test]
    fn does_not_show_the_selected_path_while_rendering_the_document() {
        let mut input = input_view_from(
            InputFormat::Json,
            br#"{"first": 1, "middle": 2, "last": 3}"#,
        );
        input.document.tail();

        assert!(input.render(80, 24).feedback.graphemes.is_empty());
    }

    #[test]
    fn searches_the_complete_tree_and_repeats_in_both_directions() {
        let mut input = input_view_with_search(
            InputFormat::Json,
            br#"{"users":[{"name":"Alice"},{"name":"Bob"}]}"#,
            [
                "@0 = {}",
                "@0.users = []",
                "@0.users[0] = {}",
                "@0.users[0].name = \"Alice\"",
                "@0.users[1] = {}",
                "@0.users[1].name = \"Bob\"",
            ],
        );
        input.update(document_action(crate::viewer::DocumentAction::CollapseAll));

        input.update(view_action(BrowseViewAction::OpenSearchEditor));
        assert_eq!(
            input.context(),
            ViewContext::Input(BrowseFocus::SearchEditor)
        );
        input.update(ViewEvent::Raw(&Event::Paste("name".into())));
        input.update(view_action(BrowseViewAction::SubmitSearch));

        assert_eq!(input.context(), ViewContext::Input(BrowseFocus::Document));
        assert_eq!(
            input.document.selected_path(),
            Some((0, ".users[0].name".into()))
        );
        assert_eq!(
            input.render(80, 24).feedback.graphemes.to_string(),
            "[vy:search] 1/2: name — n: next — N: previous"
        );

        input.update(view_action(BrowseViewAction::NextSearch));
        assert_eq!(
            input.document.selected_path(),
            Some((0, ".users[1].name".into()))
        );
        input.update(view_action(BrowseViewAction::PreviousSearch));
        assert_eq!(
            input.document.selected_path(),
            Some((0, ".users[0].name".into()))
        );
    }

    #[test]
    fn moves_between_parents_children_and_siblings() {
        let mut input = input_view_with_search(
            InputFormat::Json,
            br#"{"users":[{"name":"Alice"},{"name":"Bob"}]}"#,
            [
                "@0 = {}",
                "@0.users = []",
                "@0.users[0] = {}",
                "@0.users[0].name = \"Alice\"",
                "@0.users[1] = {}",
                "@0.users[1].name = \"Bob\"",
            ],
        );
        input.update(document_action(crate::viewer::DocumentAction::CollapseAll));

        input.update(view_action(BrowseViewAction::ExpandOrMoveToFirstChild));
        assert_eq!(input.document.selected_path(), Some((0, ".".into())));
        input.update(view_action(BrowseViewAction::ExpandOrMoveToFirstChild));
        assert_eq!(input.document.selected_path(), Some((0, ".users".into())));
        input.update(view_action(BrowseViewAction::ExpandOrMoveToFirstChild));
        assert_eq!(input.document.selected_path(), Some((0, ".users".into())));
        input.update(view_action(BrowseViewAction::ExpandOrMoveToFirstChild));
        assert_eq!(
            input.document.selected_path(),
            Some((0, ".users[0]".into()))
        );
        input.update(view_action(BrowseViewAction::MoveToNextSibling));
        assert_eq!(
            input.document.selected_path(),
            Some((0, ".users[1]".into()))
        );
        input.update(view_action(BrowseViewAction::MoveToPreviousSibling));
        assert_eq!(
            input.document.selected_path(),
            Some((0, ".users[0]".into()))
        );
        input.update(view_action(BrowseViewAction::ExpandOrMoveToFirstChild));
        input.update(view_action(BrowseViewAction::CollapseOrMoveToParent));
        assert_eq!(
            input.document.selected_path(),
            Some((0, ".users[0]".into()))
        );
        input.update(view_action(BrowseViewAction::CollapseOrMoveToParent));
        assert_eq!(input.document.selected_path(), Some((0, ".users".into())));
    }

    #[test]
    fn owns_command_editor_focus_and_requests_the_help_view() {
        let mut input = input_view();
        assert_eq!(input.context(), ViewContext::Input(BrowseFocus::Document));

        input.update(command_action(CommandAction::Open));
        assert_eq!(input.context(), ViewContext::CommandEditor);
        input.update(ViewEvent::Raw(&Event::Paste("help".into())));

        let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Help)) =
            input.update(command_action(CommandAction::Submit))
        else {
            panic!("submitting help must open the help view");
        };
        assert_eq!(input.context(), ViewContext::CommandEditor);
        input.notify(ViewNotification::OpenSucceeded);
        assert_eq!(input.context(), ViewContext::Input(BrowseFocus::Document));
    }

    #[test]
    fn requests_a_jaq_view_without_constructing_it() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("jaq .items[]".into())));

        let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq { query, .. })) =
            input.update(command_action(CommandAction::Submit))
        else {
            panic!("submitting jaq must request a jaq view");
        };
        assert_eq!(query, ".items[]");
        assert_eq!(input.context(), ViewContext::CommandEditor);
        input.notify(ViewNotification::OpenSucceeded);
        assert_eq!(input.context(), ViewContext::Input(BrowseFocus::Document));
    }

    #[test]
    fn opens_history_and_recalls_a_command_into_the_editor() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("hstr".into())));

        assert!(matches!(
            input.update(command_action(CommandAction::Submit)),
            ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Hstr))
        ));
        input.notify(ViewNotification::OpenSucceeded);
        input.notify(ViewNotification::RecallCommand("jaq .items[]".into()));

        assert_eq!(input.context(), ViewContext::CommandEditor);
        assert_eq!(
            input.command.render(80).1.graphemes.to_string(),
            ":jaq .items[] "
        );
    }

    #[test]
    fn requests_the_prepared_flatten_view() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("flatten".into())));

        assert!(matches!(
            input.update(command_action(CommandAction::Submit)),
            ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Flatten { .. }))
        ));
        assert_eq!(input.context(), ViewContext::CommandEditor);
        input.notify(ViewNotification::OpenSucceeded);
        assert_eq!(input.context(), ViewContext::Input(BrowseFocus::Document));
    }

    #[test]
    fn requests_copying_the_selected_path() {
        let mut input = input_view();
        input.document.move_to_path(0, ".value");
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("copy path".into())));

        assert!(matches!(
            input.update(command_action(CommandAction::Submit)),
            ViewUpdate::Effect(ViewEffect::Copy {
                target: CopyTarget::Path,
                content,
            }) if content == ".value"
        ));
        input.notify(ViewNotification::CopySucceeded("path".into()));
        assert_eq!(input.context(), ViewContext::Input(BrowseFocus::Document));
        assert_eq!(
            input.render(80, 24).feedback.graphemes.to_string(),
            "[vy:copy] copied path"
        );
    }

    #[test]
    fn rejects_copy_value_for_a_container() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("copy value".into())));

        assert!(matches!(
            input.update(command_action(CommandAction::Submit)),
            ViewUpdate::Render
        ));
        assert_eq!(
            input.render(80, 24).feedback.graphemes.to_string(),
            "[vy:command] failed: selected node is not a scalar"
        );
    }

    #[test]
    fn requests_config_commands_without_executing_file_operations() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste(
            "config set json.indent 4".into(),
        )));

        let ViewUpdate::Effect(ViewEffect::Config(crate::command::config::Config::Set(set))) =
            input.update(command_action(CommandAction::Submit))
        else {
            panic!("submitting config set must request a configuration operation");
        };
        assert_eq!(set.key, "json.indent");
        assert_eq!(set.value, "4");
    }

    #[test]
    fn displays_the_current_value_for_a_complete_config_set_target() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));

        assert!(matches!(
            input.update(ViewEvent::Raw(&Event::Paste(
                "config set json.indent".into()
            ))),
            ViewUpdate::Effect(ViewEffect::InspectConfig(key)) if key == "json.indent"
        ));
        input.notify(ViewNotification::ConfigTarget(Some((
            "json.indent".into(),
            "2".into(),
        ))));

        assert_eq!(
            input.render(80, 24).feedback.graphemes.to_string(),
            "[vy:config] editing: json.indent = 2"
        );
        assert!(matches!(
            input.update(ViewEvent::Raw(&Event::Paste(" 4".into()))),
            ViewUpdate::Render
        ));
        assert_eq!(
            input.render(80, 24).feedback.graphemes.to_string(),
            "[vy:config] editing: json.indent = 2"
        );
    }

    #[test]
    fn applies_reloaded_display_configuration_and_reports_success() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));
        let mut config = Config::default();
        config.json.indent = 4;

        input.notify(ViewNotification::ConfigUpdated(std::sync::Arc::new(
            crate::viewer::ConfigUpdate {
                message: "json.indent = 4".into(),
                config,
            },
        )));

        let DocumentDisplayConfig::Json(state) = input.document.display_config() else {
            panic!("input must remain JSON");
        };
        assert_eq!(state.indent, 4);
        assert_eq!(input.context(), ViewContext::Input(BrowseFocus::Document));
        assert_eq!(
            input.render(80, 24).feedback.graphemes.to_string(),
            "[vy:config] json.indent = 4"
        );
    }

    #[test]
    fn displays_child_counts_and_reloads_the_setting_without_unfolding() {
        for (format, source) in [
            (InputFormat::Json, r#"{"items": [1, [2, 3]], "empty": {}}"#),
            (InputFormat::Yaml, "items: [1, [2, 3]]\nempty: {}\n"),
        ] {
            let mut input = input_view_from(format, source.as_bytes());
            assert!(input.document.move_to_path(0, ".items"));
            input.update(document_action(crate::viewer::DocumentAction::Toggle));
            let rendered = input.render(80, 24).content.graphemes.to_string();
            assert!(rendered.contains("[…] (2 items)"), "{rendered}");
            assert!(rendered.contains("{} (0 keys)"), "{rendered}");
            let selected = input.document.selected_path();
            let subtree = input
                .document
                .selected_subtree(&input.document_source)
                .unwrap();

            for show in [false, true] {
                let mut config = Config::default();
                config.json.show_child_count = show;
                config.yaml.show_child_count = show;
                input.notify(ViewNotification::ConfigUpdated(std::sync::Arc::new(
                    crate::viewer::ConfigUpdate {
                        message: "child count display updated".into(),
                        config,
                    },
                )));
                let rendered = input.render(80, 24).content.graphemes.to_string();
                assert!(
                    rendered.contains("[…]"),
                    "container must stay collapsed: {rendered}"
                );
                assert_eq!(rendered.contains("(2 items)"), show);
                assert_eq!(rendered.contains("(0 keys)"), show);
                assert_eq!(input.document.selected_path(), selected);
                assert_eq!(
                    input
                        .document
                        .selected_subtree(&input.document_source)
                        .unwrap(),
                    subtree
                );
            }
            assert!(!subtree.contains("items)"));
            match format {
                InputFormat::Json => assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&subtree).unwrap(),
                    serde_json::json!([1, [2, 3]])
                ),
                InputFormat::Yaml => assert_eq!(
                    serde_yaml::from_str::<serde_yaml::Value>(&subtree).unwrap(),
                    serde_yaml::from_str::<serde_yaml::Value>("[1, [2, 3]]").unwrap()
                ),
                InputFormat::Auto => unreachable!(),
            }
            input.update(document_action(crate::viewer::DocumentAction::Toggle));
            assert!(
                !input
                    .render(80, 24)
                    .content
                    .graphemes
                    .to_string()
                    .contains("(2 items)")
            );
        }
    }

    #[test]
    fn applies_config_for_the_hosts_format_and_passes_it_to_jaq() {
        let mut input = input_view_from(InputFormat::Yaml, b"name: example\n");
        let mut config = Config::default();
        config.yaml.indent = 4;
        input.notify(ViewNotification::ConfigUpdated(std::sync::Arc::new(
            crate::viewer::ConfigUpdate {
                message: "yaml.indent = 4".into(),
                config,
            },
        )));
        let DocumentDisplayConfig::Yaml(state) = input.document.display_config() else {
            panic!("configuration must preserve the host's format");
        };
        assert_eq!(state.indent, 4);
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("jaq .".into())));
        let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq { display_config, .. })) =
            input.update(command_action(CommandAction::Submit))
        else {
            panic!("jaq must receive YAML configuration");
        };
        let DocumentDisplayConfig::Yaml(config) = *display_config else {
            panic!("Yaml display config expected");
        };
        assert_eq!(config.indent, 4);
    }

    #[test]
    fn keeps_result_display_settings_independent_of_temporary_document_toggles() {
        let mut input = input_view();
        let DocumentDisplayConfig::Json(expected) = input.result_display_config.clone() else {
            unreachable!();
        };
        input.update(document_action(
            crate::viewer::DocumentAction::ToggleLineNumbers,
        ));
        input.update(document_action(
            crate::viewer::DocumentAction::ToggleOverflowMode,
        ));
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("jaq .".into())));
        let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq { display_config, .. })) =
            input.update(command_action(CommandAction::Submit))
        else {
            panic!("jaq must receive the result display settings");
        };
        let DocumentDisplayConfig::Json(actual) = *display_config else {
            panic!("Json display config expected");
        };
        assert_eq!(actual.show_line_numbers, expected.show_line_numbers);
        assert_eq!(actual.overflow_mode, expected.overflow_mode);
        let DocumentDisplayConfig::Json(document) = input.document.display_config() else {
            unreachable!();
        };
        assert_ne!(actual.show_line_numbers, document.show_line_numbers);
        assert_ne!(actual.overflow_mode, document.overflow_mode);
    }

    #[test]
    fn moves_to_a_path_in_the_selected_document() {
        let mut input = input_view_from(
            InputFormat::Json,
            b"{\"first_only\":true}\n{\"second_only\":true}\n",
        );
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("goto @1 .second_only".into())));

        assert!(matches!(
            input.update(command_action(CommandAction::Submit)),
            ViewUpdate::Render
        ));
        assert_eq!(input.context(), ViewContext::Input(BrowseFocus::Document));
        assert_eq!(
            input.command.feedback().map(|f| f
                .create_graphemes(&Config::default().theme.feedback)
                .graphemes
                .to_string()),
            None
        );
    }

    #[test]
    fn reports_a_path_missing_from_the_selected_document() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("goto @1 .value".into())));

        assert!(matches!(
            input.update(command_action(CommandAction::Submit)),
            ViewUpdate::Render
        ));
        assert_eq!(
            input.command.feedback().map(|f| f
                .create_graphemes(&Config::default().theme.feedback)
                .graphemes
                .to_string()),
            Some("[vy:command] failed: path not found: @1 .value".into())
        );
        assert_eq!(input.context(), ViewContext::CommandEditor);
        assert_eq!(
            input.render(80, 24).editor.graphemes.to_string(),
            ":goto @1 .value "
        );
    }

    #[test]
    fn keeps_command_parse_errors_in_the_input_view() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("unknown".into())));

        assert!(matches!(
            input.update(command_action(CommandAction::Submit)),
            ViewUpdate::Render
        ));
        assert_eq!(input.context(), ViewContext::CommandEditor);
        assert_eq!(
            input.render(80, 24).editor.graphemes.to_string(),
            ":unknown "
        );
        assert!(
            input
                .render(80, 24)
                .feedback
                .graphemes
                .to_string()
                .contains("unknown command")
        );
    }

    #[test]
    fn displays_view_creation_errors_reported_by_the_viewer() {
        let mut input = input_view();
        input.update(command_action(CommandAction::Open));
        input.update(ViewEvent::Raw(&Event::Paste("jaq .items[]".into())));
        assert!(matches!(
            input.update(command_action(CommandAction::Submit)),
            ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq { .. }))
        ));

        input.notify(ViewNotification::OpenFailed(
            "failed to create jaq view".into(),
        ));
        assert!(
            input
                .render(80, 24)
                .feedback
                .graphemes
                .to_string()
                .contains("failed to create jaq view")
        );
        assert_eq!(input.context(), ViewContext::CommandEditor);
        assert_eq!(
            input.render(80, 24).editor.graphemes.to_string(),
            ":jaq .items[] "
        );
    }

    #[test]
    fn navigates_to_a_flatten_result() {
        let mut input = input_view();

        input.notify(ViewNotification::Navigate {
            document_index: 0,
            path: ".value".into(),
        });
        assert_eq!(input.context(), ViewContext::Input(BrowseFocus::Document));
        assert_eq!(
            input.command.feedback().map(|f| f
                .create_graphemes(&Config::default().theme.feedback)
                .graphemes
                .to_string()),
            None
        );
    }
}

#[cfg(test)]
mod focus_tests;

#[cfg(test)]
mod jaq_tests;

#[cfg(test)]
mod preview_tests;
