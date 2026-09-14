use std::{collections::HashSet, ops::Range};

use promkit::{
    core::{
        CreatedGraphemes, Widget,
        crossterm::{
            event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
            style::ContentStyle,
        },
        grapheme::StyledGraphemes,
    },
    widgets::text_editor,
};

use crate::viewer::TextEditorAction;

const MAX_VISIBLE_LINES: usize = 5;

pub(in crate::viewer) struct MultiLineEditorComponent {
    state: text_editor::State,
}

impl MultiLineEditorComponent {
    pub(in crate::viewer) fn new(
        prefix: impl Into<String>,
        value: &str,
        active_char_style: ContentStyle,
    ) -> Self {
        let prefix = prefix.into();
        let continuation_prefix = " ".repeat(StyledGraphemes::from(prefix.as_str()).widths());
        Self {
            state: text_editor::State {
                texteditor: text_editor::TextEditor::new(value),
                config: text_editor::Config {
                    prefix,
                    continuation_prefix,
                    active_char_style,
                    word_break_chars: word_break_chars(),
                    lines: Some(MAX_VISIBLE_LINES),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    pub(in crate::viewer) fn value(&self) -> String {
        self.state.texteditor.text_without_cursor().to_string()
    }

    pub(in crate::viewer) fn position(&self) -> usize {
        self.state.texteditor.position()
    }

    pub(in crate::viewer) fn replace(&mut self, value: &str) {
        self.state.texteditor.replace(value);
    }

    pub(in crate::viewer) fn replace_range(&mut self, replacement: Range<usize>, value: &str) {
        let mut text = self.state.texteditor.text_without_cursor().chars();
        let position = replacement.start + value.chars().count();
        text.splice(replacement, value.chars());
        self.state
            .texteditor
            .replace(&text.into_iter().collect::<String>());
        self.state.texteditor.move_to(position);
    }

    pub(in crate::viewer) fn move_to_tail(&mut self) {
        self.state.texteditor.move_to_tail();
    }

    pub(in crate::viewer) fn insert(&mut self, character: char) {
        self.state.texteditor.insert(character);
    }

    pub(in crate::viewer) fn set_active_char_style(&mut self, style: ContentStyle) {
        self.state.config.active_char_style = style;
    }

    #[cfg(test)]
    pub(in crate::viewer) fn active_char_style(&self) -> ContentStyle {
        self.state.config.active_char_style
    }

    pub(in crate::viewer) fn handle_input(&mut self, event: &Event) -> bool {
        if let Event::Paste(text) = event {
            for character in text.chars() {
                self.insert(character);
            }
            return true;
        }

        let Event::Key(KeyEvent {
            code: KeyCode::Char(character),
            modifiers,
            kind: KeyEventKind::Press | KeyEventKind::Repeat,
            ..
        }) = event
        else {
            return false;
        };
        if modifiers.intersects(
            KeyModifiers::CONTROL
                | KeyModifiers::ALT
                | KeyModifiers::SUPER
                | KeyModifiers::HYPER
                | KeyModifiers::META,
        ) {
            return false;
        }
        self.insert(*character);
        true
    }

    /// Applies an editor action and reports whether the text may have changed.
    pub(in crate::viewer) fn apply_action(&mut self, action: TextEditorAction) -> bool {
        match action {
            TextEditorAction::Backward => {
                self.state.texteditor.backward();
                false
            }
            TextEditorAction::Forward => {
                self.state.texteditor.forward();
                false
            }
            TextEditorAction::MoveUp => {
                self.state.texteditor.move_up();
                false
            }
            TextEditorAction::MoveDown => {
                self.state.texteditor.move_down();
                false
            }
            TextEditorAction::MoveToLineHead => {
                self.state.texteditor.move_to_line_head();
                false
            }
            TextEditorAction::MoveToLineTail => {
                self.state.texteditor.move_to_line_tail();
                false
            }
            TextEditorAction::MoveToHead => {
                self.state.texteditor.move_to_head();
                false
            }
            TextEditorAction::MoveToTail => {
                self.state.texteditor.move_to_tail();
                false
            }
            TextEditorAction::MoveToPreviousNearest => {
                self.state
                    .texteditor
                    .move_to_previous_nearest(&self.state.config.word_break_chars);
                false
            }
            TextEditorAction::MoveToNextNearest => {
                self.state
                    .texteditor
                    .move_to_next_nearest(&self.state.config.word_break_chars);
                false
            }
            TextEditorAction::InsertNewline => {
                self.state.texteditor.insert_newline();
                true
            }
            TextEditorAction::Erase => {
                self.state.texteditor.erase();
                true
            }
            TextEditorAction::EraseForward => {
                self.state.texteditor.erase_forward();
                true
            }
            TextEditorAction::EraseAll => {
                self.state.texteditor.erase_all();
                true
            }
            TextEditorAction::EraseToPreviousNearest => {
                self.state
                    .texteditor
                    .erase_to_previous_nearest(&self.state.config.word_break_chars);
                true
            }
            TextEditorAction::EraseToNextNearest => {
                self.state
                    .texteditor
                    .erase_to_next_nearest(&self.state.config.word_break_chars);
                true
            }
        }
    }
}

impl Widget for MultiLineEditorComponent {
    fn create_graphemes(&self) -> CreatedGraphemes {
        self.state.create_graphemes()
    }
}

fn word_break_chars() -> HashSet<char> {
    [' ', '.', '|', '&', '(', ')', '[', ']']
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use promkit::core::crossterm::event::{KeyEvent, KeyModifiers};

    use super::*;

    #[test]
    fn accepts_plain_characters_and_paste() {
        let mut editor = MultiLineEditorComponent::new(":", "", ContentStyle::default());

        assert!(editor.handle_input(&Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
        ))));
        assert!(editor.handle_input(&Event::Paste("bc".to_owned())));

        assert_eq!(editor.value(), "abc");
    }

    #[test]
    fn ignores_modified_characters() {
        let mut editor = MultiLineEditorComponent::new(":", "", ContentStyle::default());

        assert!(!editor.handle_input(&Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::CONTROL,
        ))));

        assert_eq!(editor.value(), "");
    }

    #[test]
    fn replaces_a_range_and_places_the_cursor_after_it() {
        let mut editor = MultiLineEditorComponent::new(":", "goto foo", ContentStyle::default());

        editor.replace_range(5..8, "bar");

        assert_eq!(editor.value(), "goto bar");
        assert_eq!(editor.position(), 8);
    }

    #[test]
    fn inserts_and_renders_multiple_lines() {
        let mut editor = MultiLineEditorComponent::new(":", "a", ContentStyle::default());

        assert!(editor.apply_action(TextEditorAction::InsertNewline));
        editor.insert('b');

        let created = editor.create_graphemes();
        assert_eq!(editor.value(), "a\nb");
        assert_eq!(created.graphemes.to_string(), ":a\n b ");
        assert_eq!(created.layout.max_height, Some(MAX_VISIBLE_LINES));
        assert_eq!(created.cursor.map(|cursor| cursor.row), Some(1));
    }
}
