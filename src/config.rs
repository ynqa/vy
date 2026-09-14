//! Application settings. Loading, file editing, themes, and key resolution live in submodules.

use promkit::widgets::{json, yaml};
use serde::Deserialize;
use std::num::NonZeroUsize;

mod file;
mod keybinds;
#[cfg(test)]
mod schema;
mod theme;

pub(crate) use file::{ConfigFile, ConfigLoadError, keys};
pub(crate) use keybinds::*;
pub(crate) use theme::*;

pub(crate) const DEFAULT_CONFIG: &str = include_str!("../default.toml");
const CONFIG_VERSION: u32 = 1;

#[derive(Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[cfg_attr(test, schemars(deny_unknown_fields))]
pub(crate) struct Config {
    /// Configuration format version, independent of the vy release version.
    /// Omitted versions are treated as version 1 for existing configurations.
    #[serde(default = "legacy_config_version")]
    #[cfg_attr(test, schemars(extend("const" = CONFIG_VERSION)))]
    version: u32,
    #[serde(default = "default_scroll_lines")]
    pub(crate) scroll_lines: NonZeroUsize,
    #[serde(default)]
    pub(crate) keybinds: Keybinds,
    #[serde(default)]
    pub(crate) theme: ThemeConfig,
    #[cfg_attr(test, schemars(with = "schema::JsonConfig"))]
    pub(crate) json: json::Config,
    #[cfg_attr(test, schemars(with = "schema::YamlConfig"))]
    pub(crate) yaml: yaml::Config,
}

fn legacy_config_version() -> u32 {
    1
}

fn default_scroll_lines() -> NonZeroUsize {
    NonZeroUsize::new(3).expect("default scroll lines must be non-zero")
}

impl Config {
    pub(crate) fn load_from(content: &str) -> Result<Self, toml::de::Error> {
        let config: Self = toml::from_str(content)?;
        if config.version != CONFIG_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported configuration version {}; this vy supports version {CONFIG_VERSION}",
                config.version
            )));
        }
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::load_from(DEFAULT_CONFIG).expect("default configuration must be valid")
    }
}

#[cfg(test)]
mod tests {
    use promkit::core::crossterm::style::{Attribute, Color};
    use promkit::widgets::{
        json::config::OverflowMode as JsonOverflowMode,
        yaml::config::OverflowMode as YamlOverflowMode,
    };

    use super::*;

    #[test]
    fn loads_default_config() {
        let config = Config::load_from(DEFAULT_CONFIG).unwrap();

        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.scroll_lines.get(), 3);
        assert_eq!(config.keybinds, Keybinds::default());
        assert_eq!(config.theme, ThemeConfig::default());

        assert_eq!(config.json.indent, 2);
        assert_eq!(
            config.json.key_style.foreground_color,
            Some(Color::DarkBlue)
        );
        assert_eq!(config.json.active_item_attribute, Attribute::Bold);
        assert_eq!(config.json.overflow_mode, JsonOverflowMode::Wrap);
        assert!(config.json.show_line_numbers);
        assert!(config.json.show_child_count);
        assert_eq!(
            config.json.child_count_style.foreground_color,
            Some(Color::DarkGrey)
        );

        assert_eq!(config.yaml.indent, 2);
        assert_eq!(
            config.yaml.tag_style.foreground_color,
            Some(Color::DarkYellow)
        );
        assert_eq!(config.yaml.active_item_attribute, Attribute::Bold);
        assert_eq!(config.yaml.overflow_mode, YamlOverflowMode::Wrap);
        assert!(config.yaml.show_line_numbers);
        assert!(config.yaml.show_child_count);
        assert_eq!(
            config.yaml.child_count_style.foreground_color,
            Some(Color::DarkGrey)
        );
    }

    #[test]
    fn loads_versioned_and_unversioned_configuration() {
        for prefix in ["", "version = 1\n"] {
            let config =
                Config::load_from(&format!("{prefix}[json]\nindent = 4\n[yaml]\n")).unwrap();
            assert_eq!(config.version, 1);
            assert_eq!(config.json.indent, 4);
        }
    }

    #[test]
    fn rejects_unsupported_and_invalid_versions() {
        for version in ["0", "2", "4294967295"] {
            let error = Config::load_from(&format!("version = {version}\n[json]\n[yaml]\n"))
                .err()
                .expect("unsupported version must fail");
            assert!(error.to_string().contains(&format!(
                "unsupported configuration version {version}; this vy supports version 1"
            )));
        }
        for version in ["-1", "1.0", "\"1\"", "true", "[]", "4294967296"] {
            assert!(
                Config::load_from(&format!("version = {version}\n[json]\n[yaml]\n")).is_err(),
                "invalid version {version} must fail"
            );
        }
    }

    #[test]
    fn loads_child_count_overrides_and_older_configuration() {
        let old_config = Config::load_from("[json]\n[yaml]\n").unwrap();
        assert!(!old_config.json.show_child_count);
        assert!(!old_config.yaml.show_child_count);
        let config = Config::load_from(
            r#"
            [json]
            show_child_count = false
            child_count_style = "fg=cyan"
            [yaml]
            show_child_count = true
            child_count_style = "fg=yellow"
        "#,
        )
        .unwrap();
        assert!(!config.json.show_child_count);
        assert!(config.yaml.show_child_count);
        assert_eq!(
            config.json.child_count_style.foreground_color,
            Some(Color::Cyan)
        );
        assert_eq!(
            config.yaml.child_count_style.foreground_color,
            Some(Color::Yellow)
        );
    }

    #[test]
    fn loads_custom_document_config() {
        let config = Config::load_from(
            r#"
                [json]
                indent = 4
                key_style = "fg=red"
                overflow_mode = "Truncate"
                show_line_numbers = false

                [yaml]
                indent = 8
                tag_style = "fg=cyan"
                overflow_mode = "Truncate"
                show_line_numbers = false
            "#,
        )
        .unwrap();

        assert_eq!(config.json.indent, 4);
        assert_eq!(config.json.key_style.foreground_color, Some(Color::Red));
        assert_eq!(config.json.overflow_mode, JsonOverflowMode::Truncate);
        assert!(!config.json.show_line_numbers);

        assert_eq!(config.yaml.indent, 8);
        assert_eq!(config.yaml.tag_style.foreground_color, Some(Color::Cyan));
        assert_eq!(config.yaml.overflow_mode, YamlOverflowMode::Truncate);
        assert!(!config.yaml.show_line_numbers);
    }

    #[test]
    fn loads_custom_scroll_lines_and_rejects_zero() {
        let config = Config::load_from(
            r#"
                scroll_lines = 4

                [json]
                [yaml]
            "#,
        )
        .unwrap();

        assert_eq!(config.scroll_lines.get(), 4);
        assert!(
            Config::load_from(
                r#"
                    scroll_lines = 0

                    [json]
                    [yaml]
                "#,
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_invalid_config() {
        assert!(Config::load_from("[json]\nindent = \"wide\"").is_err());
    }
}
