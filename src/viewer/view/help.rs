use std::borrow::Cow;

use promkit::core::{CreatedGraphemes, WidgetLayout, WidthMode, grapheme::StyledGraphemes};

use crate::{
    config::{FeedbackThemeConfig, Keybinds, ThemeConfig},
    viewer::{
        Action, ComponentAction, DocumentAction, HelpViewAction, View, ViewAction, ViewContext,
        ViewEffect, ViewEvent, ViewFrame, ViewNotification, ViewUpdate,
        feature::{Feedback, FeedbackKind, HelpHints, format_bindings},
    },
};

pub(in crate::viewer) struct HelpView {
    lines: Vec<String>,
    hints: HelpHints,
    feedback_theme: FeedbackThemeConfig,
    offset: usize,
    viewport_height: usize,
}

impl HelpView {
    pub(in crate::viewer) fn new(
        keybinds: &Keybinds,
        scroll_lines: usize,
        theme: &ThemeConfig,
    ) -> Self {
        let mut lines = vec![
            "Commands".to_owned(),
            "  :config view".to_owned(),
            "    View the active configuration file".to_owned(),
            "  :config get <key>".to_owned(),
            "    Get a configuration value by dotted key".to_owned(),
            "  :config set <key> <value>".to_owned(),
            "    Set and immediately apply a configuration value".to_owned(),
            "  :config edit".to_owned(),
            "    Edit the configuration file and immediately reload it".to_owned(),
            "  :copy, :cp path|value|subtree".to_owned(),
            "    Copy the selected path, scalar value, or serialized subtree".to_owned(),
            "  :focus".to_owned(),
            "    Open the selected subtree; commands and search use it as @0 .".to_owned(),
            "  :flatten".to_owned(),
            "    Open the prepared flattened K = V view with grep filtering".to_owned(),
            "  :goto @<document-index> <path>".to_owned(),
            "    Move to a path in a zero-based document".to_owned(),
            "  :help, :h".to_owned(),
            "    Show this help".to_owned(),
            "  :hstr".to_owned(),
            "    Search persistent command and jaq query history".to_owned(),
            "  :jaq, :j <query>".to_owned(),
            "    Execute a jaq query".to_owned(),
            "  :preview [@<document-index> <path>]".to_owned(),
            "    Read a string as text with line breaks, wrapping, and scrolling".to_owned(),
            "  :print [@<document-index> <path>]".to_owned(),
            "    Print the selected or specified subtree to stdout and exit".to_owned(),
            "  :quit".to_owned(),
            "    Exit vy".to_owned(),
            "  :write <file-path> [@<document-index> <path>]".to_owned(),
            "    Save the selected or specified subtree to a new file and continue browsing (quote file paths with spaces)"
                .to_owned(),
            String::new(),
            "Configuration".to_owned(),
            format!("  scroll_lines: {scroll_lines}"),
            "    Number of lines moved by each mouse scroll event".to_owned(),
            String::new(),
            "Keybindings".to_owned(),
        ];

        for entry in keybinds.entries() {
            lines.push(format!(
                "  {}: {}",
                entry.name,
                format_bindings(entry.events)
            ));
            lines.push(format!("    {}", entry.description));
        }

        Self {
            lines,
            hints: HelpHints::new(keybinds),
            feedback_theme: theme.feedback.clone(),
            offset: 0,
            viewport_height: 1,
        }
    }

    pub(super) fn create_content_graphemes(&mut self, height: u16) -> CreatedGraphemes {
        self.viewport_height = usize::from(height).max(1);
        self.offset = self.offset.min(self.max_offset());
        let lines = self
            .lines
            .iter()
            .skip(self.offset)
            .take(usize::from(height))
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n");

        CreatedGraphemes {
            graphemes: StyledGraphemes::from(lines),
            layout: WidgetLayout {
                max_height: Some(usize::from(height)),
                width_mode: WidthMode::Truncate,
                ..Default::default()
            },
            cursor: None,
        }
    }

    pub(super) fn apply_action(&mut self, action: DocumentAction, movement_lines: usize) {
        match action {
            DocumentAction::Up | DocumentAction::PageUp | DocumentAction::HalfPageUp => {
                self.offset = self.offset.saturating_sub(movement_lines);
            }
            DocumentAction::Down | DocumentAction::PageDown | DocumentAction::HalfPageDown => {
                self.offset = self
                    .offset
                    .saturating_add(movement_lines)
                    .min(self.max_offset());
            }
            DocumentAction::MoveToHead => {
                self.offset = 0;
            }
            DocumentAction::MoveToTail => {
                self.offset = self.max_offset();
            }
            _ => {}
        }
    }

    fn max_offset(&self) -> usize {
        self.lines.len().saturating_sub(self.viewport_height)
    }
}

impl View for HelpView {
    fn context(&self) -> ViewContext {
        ViewContext::Help
    }

    fn render(&mut self, width: u16, height: u16) -> ViewFrame {
        let feedback = Feedback::View {
            name: "help",
            message: Cow::Borrowed(""),
            hint: Cow::Owned(self.hints.create()),
            kind: FeedbackKind::Normal,
        }
        .create_graphemes(&self.feedback_theme);
        ViewFrame::create(
            width,
            height,
            feedback,
            CreatedGraphemes::default(),
            CreatedGraphemes::default(),
            |content_height| HelpView::create_content_graphemes(self, content_height),
        )
    }

    fn notify(&mut self, _notification: ViewNotification) {}

    fn update(&mut self, input: ViewEvent<'_>) -> ViewUpdate {
        match input {
            ViewEvent::Action {
                action: Action::View(ViewAction::Help(HelpViewAction::Back)),
                ..
            } => ViewUpdate::Effect(ViewEffect::Back),
            ViewEvent::Action {
                action: Action::Component(ComponentAction::Document(action)),
                context,
            } => {
                self.apply_action(action, context.movement_lines);
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
    use promkit::core::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    use crate::config::Config;

    use super::*;

    #[test]
    fn displays_current_keybindings_and_disabled_operations() {
        let mut keybinds = Keybinds::default();
        keybinds.view.browse.input.exit.clear();
        keybinds
            .view
            .browse
            .input
            .exit
            .insert(Event::Key(KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL,
            )));
        keybinds.component.document.toggle.clear();

        let mut view = HelpView::new(&keybinds, 4, &Config::default().theme);
        let help = view
            .create_content_graphemes(view.lines.len().try_into().unwrap())
            .graphemes
            .to_string();

        assert!(help.contains("view.browse.input.exit: Ctrl+X"));
        assert!(help.contains("view.browse.open_command_editor"));
        assert!(!help.contains("view.browse.input.open_command_editor"));
        assert!(help.contains("component.document.down: Down, ScrollDown, j"));
        assert!(help.contains("component.document.expand_all: e"));
        assert!(help.contains("component.document.toggle: (disabled)"));
        assert!(help.contains("component.text_editor.erase_all: Ctrl+U"));
        assert!(help.contains(":goto @<document-index> <path>"));
        assert!(help.contains(":config get <key>"));
        assert!(help.contains(":config set <key> <value>"));
        assert!(help.contains(":copy, :cp path|value|subtree"));
        assert!(help.contains(":flatten"));
        assert!(help.contains(":focus"));
        assert!(help.contains("view.browse.focus.back"));
        assert!(help.contains(":help, :h"));
        assert!(help.contains(":hstr"));
        assert!(help.contains(":jaq, :j <query>"));
        assert!(help.contains("scroll_lines: 4"));
    }

    #[test]
    fn applies_document_movement_actions() {
        let keybinds = Keybinds::default();
        let mut help = HelpView::new(&keybinds, 3, &Config::default().theme);

        help.apply_action(DocumentAction::Down, 1);
        assert_eq!(help.offset, 1);
        help.apply_action(DocumentAction::Down, 3);
        assert_eq!(help.offset, 4);
        assert!(matches!(
            help.update(ViewEvent::Action {
                action: Action::View(ViewAction::Help(HelpViewAction::Back)),
                context: crate::viewer::ActionContext {
                    click_position: None,
                    is_mouse: false,
                    movement_lines: 1,
                },
            }),
            ViewUpdate::Effect(ViewEffect::Back)
        ));
    }
}
