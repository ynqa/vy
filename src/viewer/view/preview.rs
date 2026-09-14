use std::borrow::Cow;

use promkit::core::{
    CreatedGraphemes,
    crossterm::event::{MouseButton, MouseEventKind},
};

use crate::{
    command::copy::Target as CopyTarget,
    config::{FeedbackThemeConfig, Keybinds, ThemeConfig},
    viewer::{
        Action, ComponentAction, DocumentAction, PreviewViewAction, View, ViewAction, ViewContext,
        ViewEffect, ViewEvent, ViewFrame, ViewNotification, ViewUpdate,
        feature::{Feedback, FeedbackKind, format_bindings},
        ui::SelectableTextComponent,
    },
};

pub(in crate::viewer) struct PreviewView {
    text: SelectableTextComponent,
    origin: String,
    back_hint: String,
    copy_hint: String,
    feedback: Option<(String, FeedbackKind)>,
    feedback_theme: FeedbackThemeConfig,
}

impl PreviewView {
    pub(in crate::viewer) fn new(
        content: String,
        document_index: usize,
        path: String,
        keybinds: &Keybinds,
        theme: &ThemeConfig,
    ) -> Self {
        Self {
            text: SelectableTextComponent::new(&content),
            origin: format!("@{document_index} {path}"),
            back_hint: format_bindings(&keybinds.view.preview.back),
            copy_hint: format_bindings(&keybinds.view.preview.copy_selection),
            feedback: None,
            feedback_theme: theme.feedback.clone(),
        }
    }

    fn apply_action(&mut self, action: DocumentAction, movement_lines: usize) {
        let offset = match action {
            DocumentAction::Up | DocumentAction::PageUp | DocumentAction::HalfPageUp => {
                self.text.offset().saturating_sub(movement_lines)
            }
            DocumentAction::Down | DocumentAction::PageDown | DocumentAction::HalfPageDown => {
                self.text.offset().saturating_add(movement_lines)
            }
            DocumentAction::MoveToHead => 0,
            DocumentAction::MoveToTail => self.text.max_offset(),
            _ => return,
        };
        self.text.set_offset(offset);
    }
}

impl View for PreviewView {
    fn context(&self) -> ViewContext {
        ViewContext::Preview
    }

    fn render(&mut self, width: u16, height: u16) -> ViewFrame {
        let feedback = Feedback::View {
            name: "preview",
            message: self
                .feedback
                .as_ref()
                .map_or(Cow::Borrowed(self.origin.as_str()), |(message, _)| {
                    Cow::Owned(format!("{} — {}", self.origin, message))
                }),
            hint: Cow::Owned(format!(
                "left drag: select — {}: copy selection — {}: return",
                self.copy_hint, self.back_hint
            )),
            kind: self
                .feedback
                .as_ref()
                .map_or(FeedbackKind::Normal, |(_, kind)| *kind),
        }
        .create_graphemes(&self.feedback_theme);
        ViewFrame::create(
            width,
            height,
            feedback,
            CreatedGraphemes::default(),
            CreatedGraphemes::default(),
            |content_height| self.text.create_graphemes(width, content_height),
        )
    }

    fn notify(&mut self, notification: ViewNotification) {
        match notification {
            ViewNotification::CopySucceeded(_) => {
                self.feedback = Some(("selection copied".into(), FeedbackKind::Success));
            }
            ViewNotification::CopyFailed(error) => {
                self.feedback = Some((crate::utils::normalize_lines(&error), FeedbackKind::Error));
            }
            ViewNotification::ConfigUpdated(update) => {
                self.back_hint = format_bindings(&update.config.keybinds.view.preview.back);
                self.copy_hint =
                    format_bindings(&update.config.keybinds.view.preview.copy_selection);
                self.feedback_theme = update.config.theme.feedback.clone();
            }
            _ => {}
        }
    }

    fn update(&mut self, input: ViewEvent<'_>) -> ViewUpdate {
        match input {
            ViewEvent::Pointer { kind, position } => {
                if self.text.handle_pointer(kind, position) {
                    if kind == MouseEventKind::Down(MouseButton::Left) {
                        self.feedback = None;
                    }
                    ViewUpdate::Render
                } else {
                    ViewUpdate::Ignored
                }
            }
            ViewEvent::Action {
                action: Action::View(ViewAction::Preview(PreviewViewAction::CopySelection)),
                ..
            } => {
                if let Some(content) = self.text.selected_text() {
                    ViewUpdate::Effect(ViewEffect::Copy {
                        target: CopyTarget::Value,
                        content: content.to_owned(),
                    })
                } else {
                    self.feedback =
                        Some(("drag to select text first".into(), FeedbackKind::Warning));
                    ViewUpdate::Render
                }
            }

            ViewEvent::Action {
                action: Action::View(ViewAction::Preview(PreviewViewAction::Back)),
                ..
            } => ViewUpdate::Effect(ViewEffect::Back),
            ViewEvent::Action {
                action: Action::Component(ComponentAction::Document(action)),
                context,
            } => {
                self.apply_action(action, context.movement_lines);
                ViewUpdate::Render
            }

            _ => ViewUpdate::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use promkit::core::{
        ContentPosition,
        crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers},
    };

    fn view(content: &str) -> PreviewView {
        PreviewView::new(
            content.into(),
            1,
            ".message".into(),
            &Keybinds::default(),
            &ThemeConfig::default(),
        )
    }

    fn pointer(view: &mut PreviewView, kind: MouseEventKind, row: usize, column: usize) {
        assert!(matches!(
            view.update(ViewEvent::Pointer {
                kind,
                position: Some(ContentPosition { row, column }),
            }),
            ViewUpdate::Render
        ));
    }

    fn select(view: &mut PreviewView, from: (usize, usize), to: (usize, usize)) {
        pointer(
            view,
            MouseEventKind::Down(MouseButton::Left),
            from.0,
            from.1,
        );
        pointer(view, MouseEventKind::Drag(MouseButton::Left), to.0, to.1);
        pointer(view, MouseEventKind::Up(MouseButton::Left), to.0, to.1);
    }

    fn copy_selection(view: &mut PreviewView) -> ViewUpdate {
        view.update(ViewEvent::Action {
            action: Action::View(ViewAction::Preview(PreviewViewAction::CopySelection)),
            context: crate::viewer::ActionContext {
                movement_lines: 1,
                click_position: None,
                is_mouse: false,
            },
        })
    }

    #[test]
    fn selects_in_both_directions_and_copies_only_logical_newlines() {
        for (from, to) in [((0, 1), (2, 0)), ((2, 0), (0, 1))] {
            let mut view = view("abcdef\nghi");
            view.text.create_graphemes(4, 4);
            select(&mut view, from, to);
            assert_eq!(view.text.selected_text(), Some("bcdef\ng"));
            assert!(matches!(
                copy_selection(&mut view),
                ViewUpdate::Effect(ViewEffect::Copy { target: CopyTarget::Value, content }) if content == "bcdef\ng"
            ));
        }
    }

    #[test]
    fn selection_survives_release_and_scroll_but_a_click_clears_it() {
        let mut view = view("abc\ndef\nghi\njkl");
        view.text.create_graphemes(10, 2);
        select(&mut view, (0, 1), (1, 1));
        view.apply_action(DocumentAction::Down, 1);
        view.text.create_graphemes(10, 2);
        assert_eq!(view.text.selected_text(), Some("bc\nde"));
        pointer(&mut view, MouseEventKind::Down(MouseButton::Left), 0, 0);
        pointer(&mut view, MouseEventKind::Up(MouseButton::Left), 0, 0);
        assert_eq!(view.text.selected_text(), None);
        assert!(matches!(copy_selection(&mut view), ViewUpdate::Render));
    }

    #[test]
    fn copy_feedback_preserves_selection() {
        let mut view = view("abc");
        view.text.create_graphemes(10, 2);
        select(&mut view, (0, 0), (0, 1));
        view.notify(ViewNotification::CopySucceeded("value".into()));
        assert!(
            view.render(120, 24)
                .feedback
                .graphemes
                .to_string()
                .contains("selection copied")
        );
        view.notify(ViewNotification::CopyFailed("clipboard unavailable".into()));
        assert!(
            view.render(120, 24)
                .feedback
                .graphemes
                .to_string()
                .contains("clipboard unavailable")
        );
        assert_eq!(view.text.selected_text(), Some("ab"));
    }

    #[test]
    fn copy_binding_is_configurable_and_scoped_to_preview() {
        let mut keybinds = Keybinds::default();
        let original = Event::Key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        let expected = Some(Action::View(ViewAction::Preview(
            PreviewViewAction::CopySelection,
        )));
        assert_eq!(
            crate::viewer::key_resolution::resolve(&keybinds, ViewContext::Preview, &original),
            expected
        );
        let custom = Event::Key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
        keybinds.view.preview.copy_selection = [custom.clone()].into();
        assert_eq!(
            crate::viewer::key_resolution::resolve(&keybinds, ViewContext::Preview, &custom),
            expected
        );
        assert_eq!(
            crate::viewer::key_resolution::resolve(&keybinds, ViewContext::Preview, &original),
            None
        );
        assert_eq!(
            crate::viewer::key_resolution::resolve(
                &keybinds,
                ViewContext::Focus(crate::viewer::BrowseFocus::Document),
                &custom
            ),
            None
        );
    }

    #[test]
    fn uses_independent_return_bindings_and_ignores_editing_keys() {
        let mut view = view("hello");
        let mut config = Config::default();
        let event = Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        config.keybinds.view.preview.back = [event.clone()].into();
        let action =
            crate::viewer::key_resolution::resolve(&config.keybinds, view.context(), &event)
                .unwrap();
        assert_eq!(
            action,
            Action::View(ViewAction::Preview(PreviewViewAction::Back))
        );
        assert_eq!(
            crate::viewer::key_resolution::resolve(
                &config.keybinds,
                ViewContext::Focus(crate::viewer::BrowseFocus::Document),
                &event
            ),
            None
        );
        view.notify(ViewNotification::ConfigUpdated(std::sync::Arc::new(
            crate::viewer::ConfigUpdate {
                config,
                message: String::new(),
            },
        )));
        assert!(
            view.render(120, 24)
                .feedback
                .graphemes
                .to_string()
                .contains("F2: return")
        );
        assert!(matches!(
            view.update(ViewEvent::Raw(&Event::Paste("changed".into()))),
            ViewUpdate::Ignored
        ));
        assert!(matches!(
            view.update(ViewEvent::Action {
                action,
                context: crate::viewer::ActionContext {
                    movement_lines: 1,
                    click_position: None,
                    is_mouse: false,
                }
            }),
            ViewUpdate::Effect(ViewEffect::Back)
        ));
        assert_eq!(
            view.text.create_graphemes(80, 24).graphemes.to_string(),
            "hello"
        );
    }
}
