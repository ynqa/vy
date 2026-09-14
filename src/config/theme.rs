//! Theme settings and default terminal styles.

use promkit::core::crossterm::style::{Attribute, Attributes, Color, ContentStyle};
use serde::Deserialize;

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ThemeConfig {
    pub(crate) feedback: FeedbackThemeConfig,
    pub(crate) editor: EditorThemeConfig,
    pub(crate) flatten: FlattenThemeConfig,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct FeedbackThemeConfig {
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[cfg_attr(test, schemars(with = "String"))]
    pub(crate) label_style: ContentStyle,
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[cfg_attr(test, schemars(with = "String"))]
    pub(crate) normal_style: ContentStyle,
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[cfg_attr(test, schemars(with = "String"))]
    pub(crate) loading_style: ContentStyle,
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[cfg_attr(test, schemars(with = "String"))]
    pub(crate) success_style: ContentStyle,
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[cfg_attr(test, schemars(with = "String"))]
    pub(crate) warning_style: ContentStyle,
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[cfg_attr(test, schemars(with = "String"))]
    pub(crate) error_style: ContentStyle,
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[cfg_attr(test, schemars(with = "String"))]
    pub(crate) hint_style: ContentStyle,
}

impl Default for FeedbackThemeConfig {
    fn default() -> Self {
        Self {
            label_style: ContentStyle {
                attributes: Attributes::from(Attribute::Bold),
                ..Default::default()
            },
            normal_style: ContentStyle::default(),
            loading_style: foreground_style(Color::Yellow),
            success_style: foreground_style(Color::Green),
            warning_style: foreground_style(Color::Yellow),
            error_style: foreground_style(Color::Red),
            hint_style: foreground_style(Color::DarkGrey),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct EditorThemeConfig {
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[cfg_attr(test, schemars(with = "String"))]
    pub(crate) active_char_style: ContentStyle,
}

impl Default for EditorThemeConfig {
    fn default() -> Self {
        Self {
            active_char_style: ContentStyle {
                attributes: Attributes::from(Attribute::Reverse),
                ..Default::default()
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct FlattenThemeConfig {
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[cfg_attr(test, schemars(with = "String"))]
    pub(crate) match_style: ContentStyle,
}

impl Default for FlattenThemeConfig {
    fn default() -> Self {
        Self {
            match_style: ContentStyle {
                foreground_color: Some(Color::Yellow),
                attributes: Attributes::from(Attribute::Bold),
                ..Default::default()
            },
        }
    }
}

fn foreground_style(color: Color) -> ContentStyle {
    ContentStyle {
        foreground_color: Some(color),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    use super::*;

    #[test]
    fn loads_custom_theme() {
        let config = Config::load_from(
            r#"
                [theme.feedback]
                label_style = "fg=blue,attr=bold"
                normal_style = "fg=white"
                loading_style = "fg=cyan"
                success_style = "fg=green"
                warning_style = "fg=yellow"
                error_style = "fg=red"
                hint_style = "fg=darkgrey"

                [theme.editor]
                active_char_style = "fg=black,bg=white,attr=reverse"

                [theme.flatten]
                match_style = "fg=magenta,attr=bold"

                [json]
                [yaml]
            "#,
        )
        .unwrap();

        assert_eq!(
            config.theme.feedback.label_style,
            ContentStyle {
                foreground_color: Some(Color::Blue),
                attributes: Attributes::from(Attribute::Bold),
                ..Default::default()
            }
        );
        assert_eq!(
            config.theme.feedback.normal_style,
            foreground_style(Color::White)
        );
        assert_eq!(
            config.theme.feedback.loading_style,
            foreground_style(Color::Cyan)
        );
        assert_eq!(
            config.theme.feedback.success_style,
            foreground_style(Color::Green)
        );
        assert_eq!(
            config.theme.feedback.warning_style,
            foreground_style(Color::Yellow)
        );
        assert_eq!(
            config.theme.feedback.error_style,
            foreground_style(Color::Red)
        );
        assert_eq!(
            config.theme.feedback.hint_style,
            foreground_style(Color::DarkGrey)
        );
        assert_eq!(
            config.theme.editor.active_char_style,
            ContentStyle {
                foreground_color: Some(Color::Black),
                background_color: Some(Color::White),
                attributes: Attributes::from(Attribute::Reverse),
                ..Default::default()
            }
        );
        assert_eq!(
            config.theme.flatten.match_style,
            ContentStyle {
                foreground_color: Some(Color::Magenta),
                attributes: Attributes::from(Attribute::Bold),
                ..Default::default()
            }
        );
    }
}
