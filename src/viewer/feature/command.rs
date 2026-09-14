use std::{ops::Range, sync::Arc};

use promkit::{
    core::{
        CreatedGraphemes, Widget, WidgetLayout, WidthMode,
        crossterm::{
            event::{Event, KeyEvent, KeyEventKind},
            style::ContentStyle,
        },
        grapheme::StyledGraphemes,
    },
    widgets::prefix_search,
};

use crate::{
    command::{
        COMMAND_SUGGESTIONS, CommandName, CompletionCandidate, CompletionState,
        argument::document_path::PathCompleter, config, copy, write,
    },
    history::History,
    viewer::{
        CommandAction, TextEditorAction,
        ui::{CompletionCardinality, CompletionComponent, MultiLineEditorComponent},
    },
};

mod session;

pub(in crate::viewer) use session::{CommandSession, CommandTarget};

#[derive(Debug, PartialEq, Eq)]
pub(in crate::viewer) enum CommandOutcome {
    Editing,
    Cancelled,
    Submitted(String),
}

#[derive(Default)]
enum CommandCompletion {
    #[default]
    Inactive,
    Loading,
    Failed(Arc<str>),
    Ready {
        candidates: CompletionComponent,
        replacement: Range<usize>,
    },
}

impl CommandCompletion {
    fn is_active(&self) -> bool {
        self.candidates()
            .is_some_and(CompletionComponent::is_active)
    }

    fn cardinality(&self) -> CompletionCardinality {
        self.candidates().map_or(
            CompletionCardinality::Empty,
            CompletionComponent::cardinality,
        )
    }

    fn forward(&mut self) -> bool {
        self.candidates_mut()
            .is_some_and(CompletionComponent::forward)
    }

    fn backward(&mut self) -> bool {
        self.candidates_mut()
            .is_some_and(CompletionComponent::backward)
    }

    fn create_graphemes(&self) -> Option<CreatedGraphemes> {
        self.candidates()
            .filter(|candidates| !candidates.is_empty())
            .map(Widget::create_graphemes)
    }

    fn candidates(&self) -> Option<&CompletionComponent> {
        match self {
            Self::Ready { candidates, .. } => Some(candidates),
            Self::Inactive | Self::Loading | Self::Failed(_) => None,
        }
    }

    fn candidates_mut(&mut self) -> Option<&mut CompletionComponent> {
        match self {
            Self::Ready { candidates, .. } => Some(candidates),
            Self::Inactive | Self::Loading | Self::Failed(_) => None,
        }
    }

    fn accept(&mut self) -> Option<(CompletionCandidate, Range<usize>)> {
        let Self::Ready {
            candidates,
            replacement,
        } = self
        else {
            return None;
        };
        candidates
            .accept()
            .map(|candidate| (candidate, replacement.clone()))
    }

    fn accept_single(&mut self) -> Option<(CompletionCandidate, Range<usize>)> {
        let Self::Ready {
            candidates,
            replacement,
        } = self
        else {
            return None;
        };
        candidates
            .accept_single()
            .map(|candidate| (candidate, replacement.clone()))
    }
}

pub(in crate::viewer) struct CommandComponent {
    editor: Option<MultiLineEditorComponent>,
    error: Option<String>,
    prefix_search: prefix_search::PrefixSearch,
    suggestions: prefix_search::PrefixSearchResult,
    cycling_suggestions: bool,
    path_completer: PathCompleter,
    history: History,
    completion: CommandCompletion,
    active_char_style: ContentStyle,
}

#[cfg(test)]
impl Default for CommandComponent {
    fn default() -> Self {
        Self::new(
            PathCompleter::empty(),
            History::memory(),
            crate::config::EditorThemeConfig::default().active_char_style,
        )
    }
}

impl CommandComponent {
    pub(in crate::viewer) fn new(
        path_completer: PathCompleter,
        history: History,
        active_char_style: ContentStyle,
    ) -> Self {
        Self {
            editor: None,
            error: None,
            prefix_search: COMMAND_SUGGESTIONS.iter().copied().collect(),
            suggestions: prefix_search::PrefixSearchResult::default(),
            cycling_suggestions: false,
            path_completer,
            history,
            completion: CommandCompletion::default(),
            active_char_style,
        }
    }
    pub(in crate::viewer) fn replace_completer(&mut self, completer: PathCompleter) {
        self.path_completer = completer;
        self.dismiss_completion();
    }

    pub(in crate::viewer) fn is_active(&self) -> bool {
        self.editor.is_some()
    }

    pub(in crate::viewer) fn enter(&mut self) {
        self.error = None;
        self.editor = Some(MultiLineEditorComponent::new(
            ":",
            "",
            self.active_char_style,
        ));
        self.refresh_suggestions();
    }

    pub(in crate::viewer) fn error(&self) -> Option<&str> {
        self.error.as_deref().or_else(|| match &self.completion {
            CommandCompletion::Failed(error) => Some(error.as_ref()),
            CommandCompletion::Inactive
            | CommandCompletion::Loading
            | CommandCompletion::Ready { .. } => None,
        })
    }

    pub(in crate::viewer) fn status(&self) -> Option<&str> {
        matches!(self.completion, CommandCompletion::Loading).then_some("catalog is loading")
    }

    pub(in crate::viewer) fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    pub(in crate::viewer) fn clear_error(&mut self) {
        self.error = None;
    }

    pub(in crate::viewer) fn set_active_char_style(&mut self, style: ContentStyle) {
        self.active_char_style = style;
        if let Some(editor) = &mut self.editor {
            editor.set_active_char_style(style);
        }
    }

    pub(in crate::viewer) fn config_set_target(&self) -> Option<String> {
        let input = self.editor.as_ref()?.value();
        config::set_target(&input).map(str::to_owned)
    }

    pub(in crate::viewer) fn recall(&mut self, command: &str) {
        self.enter();
        let editor = self.editor.as_mut().expect("command editor was opened");
        editor.replace(command);
        editor.move_to_tail();
        self.refresh_suggestions();
    }

    pub(in crate::viewer) fn apply_action(&mut self, action: CommandAction) -> CommandOutcome {
        if self.editor.is_none() && action != CommandAction::Open {
            return CommandOutcome::Editing;
        }

        match action {
            CommandAction::Open => {
                self.enter();
                CommandOutcome::Editing
            }
            CommandAction::Cancel if self.completion.is_active() => {
                self.dismiss_completion();
                CommandOutcome::Editing
            }
            CommandAction::Cancel => {
                self.close_editor();
                CommandOutcome::Cancelled
            }
            CommandAction::Submit if self.completion.is_active() => {
                self.accept_completion();
                CommandOutcome::Editing
            }
            CommandAction::Submit if self.cycling_suggestions => {
                self.accept_suggestion();
                CommandOutcome::Editing
            }
            CommandAction::Submit => {
                let editor = self.editor.as_ref().expect("command editor is active");
                let command = editor.value();
                self.history.record(&command);
                CommandOutcome::Submitted(command)
            }
            CommandAction::Complete
                if self.completion.cardinality() != CompletionCardinality::Empty =>
            {
                self.complete_current_candidates();
                CommandOutcome::Editing
            }
            CommandAction::Complete if !self.suggestions.is_empty() => {
                let candidate_count = self.suggestions.candidates().count();
                let next = if self.cycling_suggestions {
                    self.suggestions
                        .selected()
                        .map_or(0, |selected| (selected + 1) % candidate_count)
                } else {
                    0
                };
                self.suggestions.move_to(next);
                if let Some(candidate) = self.suggestions.get().map(str::to_owned) {
                    let editor = self.editor.as_mut().expect("command editor is active");
                    editor.replace(&candidate);
                    self.cycling_suggestions = true;
                    if candidate_count == 1 {
                        self.accept_suggestion();
                    }
                }
                CommandOutcome::Editing
            }
            CommandAction::Complete => {
                self.refresh_suggestions();
                self.complete_current_candidates();
                CommandOutcome::Editing
            }
        }
    }

    pub(in crate::viewer) fn handle_input(&mut self, event: &Event) -> CommandOutcome {
        if self.editor.is_none() {
            return CommandOutcome::Editing;
        }

        if let Event::Paste(_) = event {
            self.finish_suggestion();
            self.dismiss_completion();
            let editor = self.editor.as_mut().expect("command editor is active");
            editor.handle_input(event);
            self.refresh_suggestions();
            return CommandOutcome::Editing;
        }

        let Event::Key(KeyEvent {
            kind: KeyEventKind::Press | KeyEventKind::Repeat,
            ..
        }) = event
        else {
            return CommandOutcome::Editing;
        };

        self.finish_suggestion();
        self.dismiss_completion();

        let editor = self.editor.as_mut().expect("command editor is active");
        if editor.handle_input(event) {
            self.refresh_suggestions();
        }
        CommandOutcome::Editing
    }

    pub(in crate::viewer) fn apply_text_editor_action(
        &mut self,
        action: TextEditorAction,
    ) -> CommandOutcome {
        if self.completion.is_active() {
            match action {
                TextEditorAction::MoveUp => {
                    self.completion.backward();
                    return CommandOutcome::Editing;
                }
                TextEditorAction::MoveDown => {
                    self.completion.forward();
                    return CommandOutcome::Editing;
                }
                _ => {}
            }
        }

        self.finish_suggestion();
        self.dismiss_completion();
        let Some(editor) = &mut self.editor else {
            return CommandOutcome::Editing;
        };

        editor.apply_action(action);
        self.refresh_suggestions();
        CommandOutcome::Editing
    }

    pub(in crate::viewer) fn create_suggestion_graphemes(&self, width: u16) -> CreatedGraphemes {
        if let Some(graphemes) = self.completion.create_graphemes() {
            return graphemes;
        }
        if self.editor.is_none() || self.suggestions.is_empty() {
            return CreatedGraphemes::default();
        }

        let selected = self.suggestions.selected();
        let suggestions = self
            .suggestions
            .candidates()
            .enumerate()
            .map(|(index, candidate)| {
                if Some(index) == selected {
                    format!("❯ {candidate}")
                } else {
                    format!("  {candidate}")
                }
            })
            .collect::<Vec<_>>();
        let widths = suggestions
            .iter()
            .map(|suggestion| StyledGraphemes::from(suggestion.as_str()).widths())
            .collect::<Vec<_>>();
        let width = usize::from(width);
        let selected = selected.unwrap_or_default();
        let mut start = 0;
        let mut visible_width = widths[..=selected]
            .iter()
            .sum::<usize>()
            .saturating_add(selected * 2);

        while start < selected && visible_width > width {
            visible_width = visible_width.saturating_sub(widths[start] + 2);
            start += 1;
        }

        let mut end = selected + 1;
        while let Some(candidate_width) = widths.get(end) {
            let next_width = visible_width.saturating_add(candidate_width.saturating_add(2));
            if next_width > width {
                break;
            }
            visible_width = next_width;
            end += 1;
        }
        let suggestions = suggestions[start..end].join("  ");

        CreatedGraphemes {
            graphemes: StyledGraphemes::from(suggestions),
            layout: WidgetLayout {
                max_height: Some(1),
                width_mode: WidthMode::Truncate,
                ..Default::default()
            },
            cursor: None,
        }
    }

    fn refresh_suggestions(&mut self) {
        self.cycling_suggestions = false;
        let Some(editor) = &self.editor else {
            self.suggestions.clear();
            self.dismiss_completion();
            return;
        };
        let input = editor.value();
        let cursor = editor.position();
        if let Some((name, arguments)) = input.split_once(char::is_whitespace) {
            self.suggestions.clear();
            let arguments_start = name.chars().count() + 1;
            let Ok(name) = CommandName::parse(name) else {
                self.dismiss_completion();
                return;
            };
            let Some(cursor) = cursor.checked_sub(arguments_start) else {
                self.dismiss_completion();
                return;
            };
            let state = match name {
                CommandName::Copy => copy::complete(arguments, cursor)
                    .map_or(CompletionState::None, CompletionState::Ready),
                CommandName::Config => config::complete(arguments, cursor)
                    .map_or(CompletionState::None, CompletionState::Ready),
                CommandName::Goto
                | CommandName::Focus
                | CommandName::Preview
                | CommandName::Print => self.path_completer.complete(arguments, cursor),
                CommandName::Write => write::complete(arguments, cursor, &mut self.path_completer),
                CommandName::Flatten
                | CommandName::Help
                | CommandName::Hstr
                | CommandName::Jaq
                | CommandName::Quit => CompletionState::None,
            };
            match state {
                CompletionState::None => self.dismiss_completion(),
                CompletionState::Loading => self.completion = CommandCompletion::Loading,
                CompletionState::Failed(error) => {
                    self.completion = CommandCompletion::Failed(error);
                }
                CompletionState::Ready(request) => {
                    let mut candidates = CompletionComponent::default();
                    candidates.set_candidates(request.candidates);
                    self.completion = CommandCompletion::Ready {
                        candidates,
                        replacement: arguments_start + request.replacement.start
                            ..arguments_start + request.replacement.end,
                    };
                }
            }
            return;
        }

        self.dismiss_completion();

        let mut result = self.prefix_search.query(&input);
        let is_exact_match = result.candidates().eq([input.as_str()]);
        if is_exact_match {
            result.clear();
        }
        self.suggestions = result;
    }

    pub(in crate::viewer) fn tick(&mut self) -> bool {
        if !matches!(self.completion, CommandCompletion::Loading)
            || !self.path_completer.catalog_changed()
        {
            return false;
        }
        self.refresh_suggestions();
        true
    }

    fn accept_completion(&mut self) {
        let Some((candidate, replacement)) = self.completion.accept() else {
            return;
        };
        self.apply_completion(candidate, replacement);
    }

    fn accept_single_completion(&mut self) -> bool {
        let Some((candidate, replacement)) = self.completion.accept_single() else {
            return false;
        };
        self.apply_completion(candidate, replacement);
        true
    }

    fn complete_current_candidates(&mut self) {
        match self.completion.cardinality() {
            CompletionCardinality::Empty => {}
            CompletionCardinality::Unique => {
                self.accept_single_completion();
            }
            CompletionCardinality::Multiple => {
                self.completion.forward();
            }
        }
    }

    fn apply_completion(&mut self, candidate: CompletionCandidate, replacement: Range<usize>) {
        let editor = self.editor.as_mut().expect("command editor is active");
        editor.replace_range(replacement, &candidate.replacement);
        self.refresh_suggestions();
    }

    fn dismiss_completion(&mut self) {
        self.completion = CommandCompletion::Inactive;
    }

    fn finish_suggestion(&mut self) {
        if self.cycling_suggestions {
            self.cycling_suggestions = false;
            self.suggestions.clear();
        }
    }

    /// Confirms a command or subcommand suggestion and starts its arguments.
    fn accept_suggestion(&mut self) {
        if !self.cycling_suggestions {
            return;
        }

        self.cycling_suggestions = false;
        self.suggestions.clear();
        let editor = self.editor.as_mut().expect("command editor is active");
        if !editor
            .value()
            .chars()
            .last()
            .is_some_and(|character| character.is_whitespace())
        {
            editor.insert(' ');
        }
        self.refresh_suggestions();
    }

    fn close_editor(&mut self) {
        self.editor = None;
        self.suggestions.clear();
        self.cycling_suggestions = false;
        self.dismiss_completion();
    }

    pub(in crate::viewer) fn finish_submission(&mut self) {
        self.close_editor();
    }
}

impl Widget for CommandComponent {
    fn create_graphemes(&self) -> CreatedGraphemes {
        self.editor
            .as_ref()
            .map(Widget::create_graphemes)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use promkit::core::crossterm::event::{KeyCode, KeyEventState, KeyModifiers};

    use crate::{
        catalog::{CatalogLoadState, Indexer},
        config::EditorThemeConfig,
        document_source::DocumentSource,
        input_format::InputFormat,
    };

    use super::*;

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn active_char_style() -> ContentStyle {
        EditorThemeConfig::default().active_char_style
    }

    fn state_with_json(input: &str) -> CommandComponent {
        let source = DocumentSource::from_reader(
            InputFormat::Json,
            Box::new(Cursor::new(input.as_bytes().to_vec())),
        )
        .unwrap();
        CommandComponent::new(
            PathCompleter::from_catalog(Indexer::new(source).index().unwrap()),
            History::memory(),
            active_char_style(),
        )
    }

    #[test]
    fn enters_command_mode_with_an_empty_prompt() {
        let mut state = CommandComponent::default();

        state.enter();

        assert!(state.is_active());
        assert_eq!(state.create_graphemes().graphemes.to_string(), ": ");
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "❯ config    copy    flatten    focus    goto    help    hstr    jaq    preview"
        );
    }

    #[test]
    fn applies_the_configured_editor_style() {
        let active_char_style = ContentStyle {
            foreground_color: Some(promkit::core::crossterm::style::Color::Blue),
            ..Default::default()
        };
        let mut state =
            CommandComponent::new(PathCompleter::empty(), History::memory(), active_char_style);

        state.enter();

        assert_eq!(
            state
                .editor
                .as_ref()
                .expect("command editor must be active")
                .active_char_style(),
            active_char_style
        );
    }

    #[test]
    fn suggests_canonical_names_for_alias_prefixes() {
        for (alias, suggestion) in [('h', "❯ help    hstr"), ('j', "❯ jaq")] {
            let mut state = CommandComponent::default();
            state.enter();
            state.handle_input(&key(KeyCode::Char(alias)));

            assert_eq!(
                state.create_suggestion_graphemes(80).graphemes.to_string(),
                suggestion
            );
        }
    }

    #[test]
    fn cycles_suggestions_with_tab_and_finishes_with_another_key() {
        let mut state = CommandComponent::default();
        state.enter();

        state.handle_input(&key(KeyCode::Char('h')));
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "❯ help    hstr"
        );

        state.apply_action(CommandAction::Complete);
        assert_eq!(state.create_graphemes().graphemes.to_string(), ":help ");
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "❯ help    hstr"
        );

        state.handle_input(&key(KeyCode::Char(' ')));

        assert_eq!(state.create_graphemes().graphemes.to_string(), ":help  ");
        assert!(state.create_suggestion_graphemes(80).graphemes.is_empty());
    }

    #[test]
    fn scrolls_suggestions_horizontally_to_keep_the_selection_visible() {
        let mut state = CommandComponent::default();
        state.enter();

        assert_eq!(
            state.create_suggestion_graphemes(12).graphemes.to_string(),
            "❯ config"
        );

        state.apply_action(CommandAction::Complete);
        assert_eq!(state.create_graphemes().graphemes.to_string(), ":config ");
        assert_eq!(
            state.create_suggestion_graphemes(12).graphemes.to_string(),
            "❯ config"
        );

        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_suggestion_graphemes(12).graphemes.to_string(),
            "❯ copy"
        );

        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_suggestion_graphemes(12).graphemes.to_string(),
            "❯ flatten"
        );

        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_suggestion_graphemes(12).graphemes.to_string(),
            "❯ focus"
        );
        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_suggestion_graphemes(12).graphemes.to_string(),
            "❯ goto"
        );
    }

    #[test]
    fn enter_accepts_a_selected_suggestion_before_submitting() {
        let mut state = CommandComponent::default();
        state.enter();
        state.handle_input(&key(KeyCode::Char('h')));
        state.apply_action(CommandAction::Complete);

        assert_eq!(
            state.apply_action(CommandAction::Submit),
            CommandOutcome::Editing
        );
        assert_eq!(state.create_graphemes().graphemes.to_string(), ":help  ");
        assert!(state.create_suggestion_graphemes(80).graphemes.is_empty());

        assert_eq!(
            state.apply_action(CommandAction::Submit),
            CommandOutcome::Submitted("help ".into())
        );
        assert!(state.is_active());
        state.finish_submission();
        assert!(!state.is_active());
    }

    #[test]
    fn records_submitted_commands() {
        let history = History::memory();
        let mut state =
            CommandComponent::new(PathCompleter::empty(), history.clone(), active_char_style());
        state.enter();
        state.handle_input(&Event::Paste("jaq .items[]".into()));

        assert_eq!(
            state.apply_action(CommandAction::Submit),
            CommandOutcome::Submitted("jaq .items[]".into())
        );
        assert_eq!(history.newest(), vec!["jaq .items[]"]);
    }

    #[test]
    fn tab_accepts_a_unique_command_and_stops_at_its_arguments() {
        for (prefix, command) in [
            ("go", "goto"),
            ("foc", "focus"),
            ("pre", "preview"),
            ("pri", "print"),
        ] {
            let mut state = state_with_json(r#"{"name":"Alice"}"#);
            state.enter();
            state.handle_input(&Event::Paste(prefix.into()));
            assert_eq!(
                state.apply_action(CommandAction::Complete),
                CommandOutcome::Editing
            );
            assert_eq!(
                state.create_graphemes().graphemes.to_string(),
                format!(":{command}  ")
            );
            assert_eq!(
                state.create_suggestion_graphemes(80).graphemes.to_string(),
                "  @0"
            );

            // The next Tab accepts the document index, rather than cycling the command.
            state.apply_action(CommandAction::Complete);
            assert_eq!(
                state.create_graphemes().graphemes.to_string(),
                format!(":{command} @0  ")
            );
        }
    }

    #[test]
    fn a_unique_argumentless_command_executes_on_enter_after_tab() {
        for (prefix, command) in [("hel", "help"), ("qu", "quit")] {
            let mut state = CommandComponent::default();
            state.enter();
            state.handle_input(&Event::Paste(prefix.into()));
            assert_eq!(
                state.apply_action(CommandAction::Complete),
                CommandOutcome::Editing
            );
            assert_eq!(
                state.create_graphemes().graphemes.to_string(),
                format!(":{command}  ")
            );
            assert!(state.create_suggestion_graphemes(80).graphemes.is_empty());
            assert_eq!(
                state.apply_action(CommandAction::Submit),
                CommandOutcome::Submitted(format!("{command} "))
            );
        }
    }

    #[test]
    fn completes_goto_arguments_in_a_vertical_list() {
        let mut state = state_with_json(concat!(
            r#"{"account":{},"users":[]}"#,
            "\n",
            r#"{"second":true}"#,
        ));
        state.enter();
        state.handle_input(&Event::Paste("goto ".into()));

        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  @0\n  @1"
        );

        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "❯ @0\n  @1"
        );
        assert_eq!(
            state.apply_action(CommandAction::Submit),
            CommandOutcome::Editing
        );
        assert_eq!(state.create_graphemes().graphemes.to_string(), ":goto @0  ");
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  ."
        );

        // A single delimiter is accepted by one Tab, then its direct children
        // are displayed without recursively accepting another candidate.
        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_graphemes().graphemes.to_string(),
            ":goto @0 . "
        );
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  .account\n  .users"
        );
    }

    #[test]
    fn shares_path_completion_without_replacing_the_command_prefix() {
        for name in ["goto", "focus", "preview", "print", "write ./out.json"] {
            for (arguments, completed) in [
                ("", "@0 "),
                ("@0 .na", "@0 .name"),
                (r#"@0 ["名"#, r#"@0 ["名前"]"#),
            ] {
                let mut state = state_with_json(r#"{"name":1,"名前":2}"#);
                state.enter();
                state.handle_input(&Event::Paste(format!("{name}  {arguments}")));
                state.apply_action(CommandAction::Complete);
                assert_eq!(
                    state.create_graphemes().graphemes.to_string(),
                    format!(":{name}  {completed} ")
                );
            }
        }
        let mut state = state_with_json(r#"{"name":1}"#);
        state.enter();
        state.handle_input(&Event::Paste("jaq @0 .na".into()));
        assert!(state.create_suggestion_graphemes(80).graphemes.is_empty());
    }

    #[test]
    fn completes_config_subcommands_and_keys() {
        let mut state = CommandComponent::default();
        state.enter();
        state.handle_input(&Event::Paste("config g".into()));

        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  get"
        );
        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_graphemes().graphemes.to_string(),
            ":config get  "
        );

        state.handle_input(&Event::Paste("json.ind".into()));
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  json.indent"
        );
        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.apply_action(CommandAction::Submit),
            CommandOutcome::Submitted("config get json.indent".into())
        );
    }

    #[test]
    fn completes_copy_targets() {
        let mut state = CommandComponent::default();
        state.enter();
        state.handle_input(&Event::Paste("copy sub".into()));

        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  subtree"
        );
        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.apply_action(CommandAction::Submit),
            CommandOutcome::Submitted("copy subtree".into())
        );
    }

    #[test]
    fn completes_arguments_in_the_middle_with_command_relative_offsets() {
        for (input, prefix, expected) in [
            ("copy  subXXX", "copy  sub", "copy  subtree"),
            ("cp\u{3000} subXXX", "cp\u{3000} sub", "cp\u{3000} subtree"),
            (
                "config  get json.indXXX",
                "config  get json.ind",
                "config  get json.indent",
            ),
        ] {
            let mut state = CommandComponent::default();
            state.enter();
            state.handle_input(&Event::Paste(input.into()));
            for _ in prefix.chars().count()..input.chars().count() {
                state.apply_text_editor_action(TextEditorAction::Backward);
            }
            state.apply_action(CommandAction::Complete);
            assert_eq!(state.editor.as_ref().unwrap().value(), expected);
        }
    }

    #[test]
    fn hides_argument_completion_when_editing_the_command_name() {
        for input in [
            "copy sub",
            "cp sub",
            "config g",
            "goto @",
            "focus @",
            "print @",
            "write ./out.json @",
        ] {
            let mut state = state_with_json(r#"{"name":1}"#);
            state.enter();
            state.handle_input(&Event::Paste(input.into()));
            state.apply_text_editor_action(TextEditorAction::MoveToHead);
            assert!(state.create_suggestion_graphemes(80).graphemes.is_empty());
        }
    }

    #[test]
    fn completes_config_set_values_with_known_domains() {
        let mut state = CommandComponent::default();
        state.enter();
        state.handle_input(&Event::Paste("config set json.overflow_mode T".into()));

        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  Truncate"
        );
        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.apply_action(CommandAction::Submit),
            CommandOutcome::Submitted("config set json.overflow_mode Truncate".into())
        );
    }

    #[test]
    fn cycles_goto_path_candidates_with_tab() {
        let mut state = state_with_json(r#"{"account":{},"users":[]}"#);
        state.enter();
        state.handle_input(&Event::Paste("goto @0 .".into()));

        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "❯ .account\n  .users"
        );

        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  .account\n❯ .users"
        );

        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "❯ .account\n  .users"
        );
    }

    #[test]
    fn accepts_a_fuzzy_path_segment_before_submitting_goto() {
        let mut state = state_with_json(r#"{"account":{},"users":[]}"#);
        state.enter();
        state.handle_input(&Event::Paste("goto @0 .usr".into()));

        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  .users"
        );
        // A single path candidate is accepted immediately by Tab.
        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_graphemes().graphemes.to_string(),
            ":goto @0 .users "
        );
        assert_eq!(
            state.apply_action(CommandAction::Submit),
            CommandOutcome::Submitted("goto @0 .users".into())
        );
    }

    #[test]
    fn accepts_one_candidate_per_tab_and_displays_the_next_level() {
        let mut state = state_with_json(r#"{"users":[{"name":"Alice"}]}"#);
        state.enter();
        state.handle_input(&Event::Paste("goto @0 .users".into()));

        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  ["
        );
        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_graphemes().graphemes.to_string(),
            ":goto @0 .users[ "
        );
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  [0]"
        );

        state.apply_action(CommandAction::Complete);
        assert_eq!(
            state.create_graphemes().graphemes.to_string(),
            ":goto @0 .users[0] "
        );
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  ."
        );
    }

    #[test]
    fn completes_after_manually_typed_delimiters_without_duplication() {
        let mut object = state_with_json(r#"{"only":true}"#);
        object.enter();
        object.handle_input(&Event::Paste("goto @0 .".into()));
        object.apply_action(CommandAction::Complete);
        assert_eq!(
            object.create_graphemes().graphemes.to_string(),
            ":goto @0 .only "
        );

        let mut array = state_with_json("[true]");
        array.enter();
        array.handle_input(&Event::Paste("goto @0 [".into()));
        array.apply_action(CommandAction::Complete);
        assert_eq!(
            array.create_graphemes().graphemes.to_string(),
            ":goto @0 [0] "
        );
    }

    #[test]
    fn accepts_a_single_document_candidate_and_stops_at_the_next_level() {
        let mut state = state_with_json(r#"{"name":"Alice"}"#);
        state.enter();
        state.handle_input(&Event::Paste("goto ".into()));

        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  @0"
        );
        state.apply_action(CommandAction::Complete);
        assert_eq!(state.create_graphemes().graphemes.to_string(), ":goto @0  ");
        assert_eq!(
            state.create_suggestion_graphemes(80).graphemes.to_string(),
            "  ."
        );
    }

    #[test]
    fn reports_catalog_loading_while_completing_goto() {
        let mut state = CommandComponent::new(
            PathCompleter::from_state(CatalogLoadState::Loading),
            History::memory(),
            active_char_style(),
        );
        state.enter();
        state.handle_input(&Event::Paste("goto @".into()));

        assert_eq!(state.status(), Some("catalog is loading"));
        assert!(state.create_suggestion_graphemes(80).graphemes.is_empty());
    }

    #[test]
    fn reports_catalog_failure_while_completing_goto() {
        let error: Arc<str> = "failed to parse input".into();
        let mut state = CommandComponent::new(
            PathCompleter::from_state(CatalogLoadState::Failed(Arc::clone(&error))),
            History::memory(),
            active_char_style(),
        );
        state.enter();
        state.handle_input(&Event::Paste("goto @".into()));

        assert_eq!(state.error(), Some(error.as_ref()));
        assert_eq!(state.status(), None);
        assert!(state.create_suggestion_graphemes(80).graphemes.is_empty());
    }

    #[test]
    fn hides_suggestions_for_arguments_and_unknown_prefixes() {
        let mut state = CommandComponent::default();
        state.enter();

        state.handle_input(&Event::Paste("jaq ".into()));
        assert!(state.create_suggestion_graphemes(80).graphemes.is_empty());

        state.enter();
        state.handle_input(&key(KeyCode::Char('x')));
        assert!(state.create_suggestion_graphemes(80).graphemes.is_empty());
    }

    #[test]
    fn edits_and_submits_command_text() {
        let mut state = CommandComponent::default();
        state.enter();

        state.handle_input(&key(KeyCode::Char('j')));
        state.handle_input(&key(KeyCode::Char('q')));
        state.handle_input(&key(KeyCode::Char('x')));
        state.apply_text_editor_action(TextEditorAction::Backward);
        state.apply_text_editor_action(TextEditorAction::Erase);

        assert_eq!(
            state.apply_action(CommandAction::Submit),
            CommandOutcome::Submitted("jx".into())
        );
        assert!(state.is_active());
        state.finish_submission();
        assert!(!state.is_active());
    }

    #[test]
    fn supports_word_and_whole_input_editing() {
        let mut state = CommandComponent::default();
        state.enter();
        state.handle_input(&Event::Paste("jaq .foo".into()));

        state.apply_text_editor_action(TextEditorAction::EraseToPreviousNearest);
        assert_eq!(state.create_graphemes().graphemes.to_string(), ":jaq . ");

        state.apply_text_editor_action(TextEditorAction::EraseAll);
        assert_eq!(state.create_graphemes().graphemes.to_string(), ": ");
    }

    #[test]
    fn supports_multiline_cursor_movement() {
        let mut editor = MultiLineEditorComponent::new("", "a\nbc", ContentStyle::default());

        editor.apply_action(TextEditorAction::MoveToLineHead);
        assert_eq!(editor.position(), 2);

        editor.apply_action(TextEditorAction::MoveUp);
        assert_eq!(editor.position(), 0);

        editor.apply_action(TextEditorAction::MoveToTail);
        assert_eq!(editor.position(), 4);
    }

    #[test]
    fn cancels_and_discards_command_text() {
        let mut state = CommandComponent::default();
        state.enter();
        state.handle_input(&Event::Paste("jaq .".into()));

        assert_eq!(
            state.apply_action(CommandAction::Cancel),
            CommandOutcome::Cancelled
        );

        state.enter();
        assert_eq!(state.create_graphemes().graphemes.to_string(), ": ");
    }

    #[test]
    fn retains_an_error_until_command_mode_is_reentered() {
        let mut state = CommandComponent::default();
        state.set_error("unknown command: invalid");

        assert_eq!(state.error(), Some("unknown command: invalid"));

        state.enter();
        assert_eq!(state.error(), None);
    }

    #[test]
    fn ignores_key_release_events() {
        let mut state = CommandComponent::default();
        state.enter();

        state.handle_input(&Event::Key(KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        }));

        assert_eq!(state.create_graphemes().graphemes.to_string(), ": ");
    }
}
