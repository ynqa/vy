use std::borrow::Cow;

use promkit::core::{CreatedGraphemes, WidgetLayout, WidthMode, grapheme::StyledGraphemes};

use crate::{
    config::{FeedbackThemeConfig, Keybinds, ThemeConfig},
    viewer::{
        Action, ComponentAction, DocumentAction, HelpViewAction, View, ViewAction, ViewContext,
        ViewEffect, ViewEvent, ViewFrame, ViewNotification, ViewUpdate,
        feature::{Feedback, FeedbackKind, HelpHints},
    },
};

pub(in crate::viewer) struct ConfigView {
    lines: Vec<String>,
    hints: HelpHints,
    feedback_theme: FeedbackThemeConfig,
    offset: usize,
    viewport_height: usize,
}

impl ConfigView {
    pub(in crate::viewer) fn new(
        content: String,
        keybinds: &Keybinds,
        theme: &ThemeConfig,
    ) -> Self {
        Self {
            lines: content.lines().map(str::to_owned).collect(),
            hints: HelpHints::new(keybinds),
            feedback_theme: theme.feedback.clone(),
            offset: 0,
            viewport_height: 1,
        }
    }

    fn create_content_graphemes(&mut self, height: u16) -> CreatedGraphemes {
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

    fn apply_action(&mut self, action: DocumentAction, movement_lines: usize) {
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
            DocumentAction::MoveToHead => self.offset = 0,
            DocumentAction::MoveToTail => self.offset = self.max_offset(),
            _ => {}
        }
    }

    fn max_offset(&self) -> usize {
        self.lines.len().saturating_sub(self.viewport_height)
    }
}

impl View for ConfigView {
    fn context(&self) -> ViewContext {
        // Config and help are both read-only text views and share navigation bindings.
        ViewContext::Help
    }

    fn render(&mut self, width: u16, height: u16) -> ViewFrame {
        let feedback = Feedback::View {
            name: "config",
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
            |content_height| self.create_content_graphemes(content_height),
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
    use super::*;

    #[test]
    fn displays_and_scrolls_configuration_text() {
        let mut view = ConfigView::new(
            "first = 1\nsecond = 2\nthird = 3".into(),
            &Keybinds::default(),
            &ThemeConfig::default(),
        );

        assert_eq!(
            view.create_content_graphemes(2).graphemes.to_string(),
            "first = 1\nsecond = 2"
        );
        view.apply_action(DocumentAction::Down, 1);
        assert_eq!(
            view.create_content_graphemes(2).graphemes.to_string(),
            "second = 2\nthird = 3"
        );
    }
}
