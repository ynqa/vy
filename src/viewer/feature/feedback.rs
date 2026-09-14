//! Feedback presentation specific to vy's views and commands.

use std::borrow::Cow;

use promkit::core::{CreatedGraphemes, WidgetLayout, WidthMode, grapheme::StyledGraphemes};

use crate::{config::FeedbackThemeConfig, utils::normalize_lines};

#[derive(Clone, Copy)]
pub(in crate::viewer) enum FeedbackKind {
    Normal,
    Loading,
    Success,
    Warning,
    Error,
}

pub(in crate::viewer) enum Feedback<'a> {
    Error {
        name: &'static str,
        message: Cow<'a, str>,
    },
    CommandError(&'a str),
    CommandStatus(&'a str),
    View {
        name: &'static str,
        message: Cow<'a, str>,
        hint: Cow<'a, str>,
        kind: FeedbackKind,
    },
}

impl Feedback<'_> {
    pub(in crate::viewer) fn create_graphemes(
        &self,
        theme: &FeedbackThemeConfig,
    ) -> CreatedGraphemes {
        match self {
            Self::Error { name, message } => create(
                name,
                normalize_lines(message.as_ref()),
                "",
                FeedbackKind::Error,
                theme,
            ),
            Self::CommandError(error) => create(
                "command",
                format!("failed: {}", normalize_lines(error)),
                "",
                FeedbackKind::Error,
                theme,
            ),
            Self::CommandStatus(status) => create(
                "command",
                normalize_lines(status),
                "",
                FeedbackKind::Loading,
                theme,
            ),
            Self::View {
                name,
                message,
                hint,
                kind,
            } => create(name, message, hint, *kind, theme),
        }
    }
}

fn create(
    feedback_name: &str,
    message: impl AsRef<str>,
    hint: impl AsRef<str>,
    kind: FeedbackKind,
    theme: &FeedbackThemeConfig,
) -> CreatedGraphemes {
    let message_is_empty = message.as_ref().is_empty();
    let label =
        StyledGraphemes::from(format!("[vy:{feedback_name}]")).apply_style(theme.label_style);
    let separator = StyledGraphemes::from(" ");
    let message = StyledGraphemes::from(message.as_ref()).apply_style(match kind {
        FeedbackKind::Normal => theme.normal_style,
        FeedbackKind::Loading => theme.loading_style,
        FeedbackKind::Success => theme.success_style,
        FeedbackKind::Warning => theme.warning_style,
        FeedbackKind::Error => theme.error_style,
    });
    let hint = match (message_is_empty, hint.as_ref().is_empty()) {
        (_, true) => StyledGraphemes::default(),
        (true, false) => StyledGraphemes::from(hint.as_ref()).apply_style(theme.hint_style),
        (false, false) => {
            StyledGraphemes::from(format!(" — {}", hint.as_ref())).apply_style(theme.hint_style)
        }
    };

    CreatedGraphemes {
        graphemes: [label, separator, message, hint].into_iter().collect(),
        layout: WidgetLayout {
            max_height: None,
            width_mode: WidthMode::Wrap,
            ..Default::default()
        },
        cursor: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(feedback: Feedback<'_>) -> CreatedGraphemes {
        feedback.create_graphemes(&FeedbackThemeConfig::default())
    }

    #[test]
    fn labels_each_feedback_by_its_name() {
        assert_eq!(
            render(Feedback::Error {
                name: "config",
                message: Cow::Borrowed("invalid config"),
            })
            .graphemes
            .to_string(),
            "[vy:config] invalid config"
        );
        assert_eq!(
            render(Feedback::CommandError("unknown command"))
                .graphemes
                .to_string(),
            "[vy:command] failed: unknown command"
        );
        assert_eq!(
            render(Feedback::CommandStatus("catalog is loading"))
                .graphemes
                .to_string(),
            "[vy:command] catalog is loading"
        );
        assert_eq!(
            render(Feedback::View {
                name: "help",
                message: Cow::Borrowed(""),
                hint: Cow::Borrowed("focus: content — Esc: return"),
                kind: FeedbackKind::Normal,
            })
            .graphemes
            .to_string(),
            "[vy:help] focus: content — Esc: return"
        );
        assert_eq!(
            render(Feedback::View {
                name: "jaq",
                message: Cow::Borrowed("completed"),
                hint: Cow::Borrowed("focus: editor"),
                kind: FeedbackKind::Success,
            })
            .graphemes
            .to_string(),
            "[vy:jaq] completed — focus: editor"
        );
    }

    #[test]
    fn wraps_command_errors() {
        let feedback = Feedback::CommandError("compile error\r\ntry again");

        let created = feedback.create_graphemes(&FeedbackThemeConfig::default());
        assert_eq!(
            created.graphemes.to_string(),
            "[vy:command] failed: compile error\ntry again"
        );
        assert_eq!(created.layout.max_height, None);
        assert_eq!(created.layout.width_mode, WidthMode::Wrap);
    }

    #[test]
    fn wraps_every_feedback() {
        let feedbacks = [
            Feedback::Error {
                name: "config",
                message: Cow::Borrowed("invalid config"),
            },
            Feedback::CommandError("unknown command"),
            Feedback::CommandStatus("catalog is loading"),
            Feedback::View {
                name: "help",
                message: Cow::Borrowed("focus: content"),
                hint: Cow::Borrowed(""),
                kind: FeedbackKind::Normal,
            },
            Feedback::View {
                name: "jaq",
                message: Cow::Borrowed("⠋ running"),
                hint: Cow::Borrowed("focus: editor"),
                kind: FeedbackKind::Loading,
            },
        ];
        for feedback in &feedbacks {
            assert_eq!(
                feedback
                    .create_graphemes(&FeedbackThemeConfig::default())
                    .layout
                    .width_mode,
                WidthMode::Wrap
            );
        }
    }

    #[test]
    fn wraps_error_feedback() {
        let created = Feedback::Error {
            name: "config",
            message: Cow::Borrowed("invalid config"),
        }
        .create_graphemes(&FeedbackThemeConfig::default());
        assert_eq!(created.graphemes.to_string(), "[vy:config] invalid config");
        assert_eq!(created.layout.width_mode, WidthMode::Wrap);
    }
}
