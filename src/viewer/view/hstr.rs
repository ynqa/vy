use std::borrow::Cow;

use promkit::core::{
    CreatedGraphemes, Widget, WidgetLayout, WidthMode,
    crossterm::{
        event::Event as TerminalEvent,
        style::{Attribute, ContentStyle},
    },
    grapheme::StyledGraphemes,
};

use crate::{
    config::{FeedbackThemeConfig, ThemeConfig},
    history::History,
    utils::one_line,
    viewer::{
        Action, ComponentAction, DocumentAction, HstrViewAction, QueryFocus, View, ViewAction,
        ViewContext, ViewEffect, ViewEvent, ViewFrame, ViewNotification, ViewUpdate,
        feature::{Feedback, FeedbackKind, HintFocus, QueryHintState, QueryHints},
        ui::MultiLineEditorComponent,
    },
};

pub(in crate::viewer) struct HstrView {
    entries: Vec<String>,
    matches: Vec<usize>,
    editor: MultiLineEditorComponent,
    selected: usize,
    offset: usize,
    viewport_height: usize,
    focus: QueryFocus,
    hints: QueryHints,
    feedback_theme: FeedbackThemeConfig,
}

impl HstrView {
    pub(in crate::viewer) fn new(
        history: &History,
        hints: QueryHints,
        theme: &ThemeConfig,
    ) -> Self {
        let entries = history.newest();
        let matches = (0..entries.len()).collect();
        Self {
            entries,
            matches,
            editor: MultiLineEditorComponent::new(":hstr ", "", theme.editor.active_char_style),
            selected: 0,
            offset: 0,
            viewport_height: 1,
            focus: QueryFocus::Result,
            hints,
            feedback_theme: theme.feedback.clone(),
        }
    }

    fn feedback(&self) -> Feedback<'_> {
        let message = if self.entries.is_empty() {
            "no history".to_owned()
        } else {
            format!("{}/{} entries", self.matches.len(), self.entries.len())
        };
        Feedback::View {
            name: "hstr",
            message: Cow::Owned(message),
            hint: Cow::Owned(self.hints.create(
                match self.focus {
                    QueryFocus::Editor => HintFocus::Editor,
                    QueryFocus::Result => HintFocus::Result,
                },
                QueryHintState::Ready,
            )),
            kind: FeedbackKind::Normal,
        }
    }

    fn create_content_graphemes(&mut self, height: u16) -> CreatedGraphemes {
        self.viewport_height = usize::from(height).max(1);
        self.reveal_selection();
        let selected = (self.focus == QueryFocus::Result).then_some(self.selected);
        let lines = self
            .matches
            .iter()
            .skip(self.offset)
            .take(usize::from(height))
            .enumerate()
            .filter_map(|(visible_index, entry_index)| {
                self.entries
                    .get(*entry_index)
                    .map(|entry| (visible_index, entry))
            })
            .map(|(visible_index, entry)| {
                let mut style = ContentStyle::default();
                if selected == Some(self.offset + visible_index) {
                    style.attributes.set(Attribute::Reverse);
                }
                StyledGraphemes::from_str(one_line(entry), style)
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

    fn max_offset(&self) -> usize {
        self.matches.len().saturating_sub(self.viewport_height)
    }

    fn select_visible_row(&mut self, row: usize) -> bool {
        let selected = self.offset.saturating_add(row);
        if selected >= self.matches.len() {
            return false;
        }
        self.selected = selected;
        true
    }

    fn selected_command(&self) -> Option<String> {
        self.matches
            .get(self.selected)
            .and_then(|index| self.entries.get(*index))
            .cloned()
    }

    fn refresh_matches(&mut self) {
        let query = self.editor.value().to_lowercase();
        self.matches = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| entry.to_lowercase().contains(&query).then_some(index))
            .collect();
        self.selected = 0;
        self.offset = 0;
    }

    fn handle_input(&mut self, event: &TerminalEvent) -> bool {
        self.editor.handle_input(event)
    }
}

impl View for HstrView {
    fn context(&self) -> ViewContext {
        ViewContext::Hstr(self.focus)
    }

    fn render(&mut self, width: u16, height: u16) -> ViewFrame {
        let editor = match self.focus {
            QueryFocus::Editor => self.editor.create_graphemes(),
            QueryFocus::Result => CreatedGraphemes::default(),
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
                action: Action::View(ViewAction::Hstr(action)),
                context,
            } => match action {
                HstrViewAction::Back => ViewUpdate::Effect(ViewEffect::Back),
                HstrViewAction::ToggleFocus => {
                    self.focus = match self.focus {
                        QueryFocus::Editor => QueryFocus::Result,
                        QueryFocus::Result => QueryFocus::Editor,
                    };
                    ViewUpdate::Render
                }
                HstrViewAction::RecallSelection => {
                    if context.is_mouse {
                        let Some(position) = context.click_position else {
                            return ViewUpdate::Handled;
                        };
                        if !self.select_visible_row(position.row) {
                            return ViewUpdate::Handled;
                        }
                    }
                    self.selected_command()
                        .map(|command| ViewUpdate::Effect(ViewEffect::RecallCommand(command)))
                        .unwrap_or(ViewUpdate::Handled)
                }
            },
            ViewEvent::Action {
                action: Action::Component(ComponentAction::TextEditor(action)),
                ..
            } => {
                if self.editor.apply_action(action) {
                    self.refresh_matches();
                }
                ViewUpdate::Render
            }
            ViewEvent::Action {
                action: Action::Component(ComponentAction::Document(action)),
                context,
            } => {
                self.apply_movement(action, context.movement_lines);
                ViewUpdate::Render
            }
            ViewEvent::Raw(event) if self.focus == QueryFocus::Editor => {
                if self.handle_input(event) {
                    self.refresh_matches();
                }
                ViewUpdate::Render
            }
            ViewEvent::Action { .. }
            | ViewEvent::Pointer { .. }
            | ViewEvent::Raw(_)
            | ViewEvent::Tick => ViewUpdate::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use promkit::core::crossterm::event::Event;

    use crate::config::{Config, Keybinds};

    use super::*;
    use crate::viewer::ActionContext;

    fn view() -> HstrView {
        let history = History::memory();
        history.record("config get json.indent");
        history.record("jaq .items[]");
        HstrView::new(
            &history,
            QueryHints::hstr(&Keybinds::default()),
            &Config::default().theme,
        )
    }

    fn action(action: Action) -> ViewEvent<'static> {
        ViewEvent::Action {
            action,
            context: ActionContext {
                click_position: None,
                is_mouse: false,
                movement_lines: 1,
            },
        }
    }

    #[test]
    fn renders_newest_history_as_a_list_and_moves_selection() {
        let mut view = view();
        assert_eq!(
            view.create_content_graphemes(10).graphemes.to_string(),
            "jaq .items[]\nconfig get json.indent"
        );

        view.update(action(Action::Component(ComponentAction::Document(
            DocumentAction::Down,
        ))));
        assert!(matches!(
            view.update(action(Action::View(ViewAction::Hstr(
                HstrViewAction::RecallSelection
            )))),
            ViewUpdate::Effect(ViewEffect::RecallCommand(command)) if command == "config get json.indent"
        ));
    }

    #[test]
    fn filters_history_from_the_editor() {
        let mut view = view();
        view.update(action(Action::View(ViewAction::Hstr(
            HstrViewAction::ToggleFocus,
        ))));
        view.update(ViewEvent::Raw(&Event::Paste("config".into())));

        assert_eq!(
            view.create_content_graphemes(10).graphemes.to_string(),
            "config get json.indent"
        );
        assert_eq!(view.matches.len(), 1);
    }
}
