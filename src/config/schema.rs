//! JSON Schema generation for development and release packaging.

use promkit::{
    core::crossterm::style::{Attribute, ContentStyle},
    widgets::{json, yaml},
};
use schemars::{JsonSchema, generate::SchemaSettings};

#[derive(JsonSchema)]
#[schemars(untagged)]
#[allow(dead_code)]
pub(super) enum KeyBinding {
    Key(String),
    Keys(Vec<String>),
}

#[derive(JsonSchema)]
#[schemars(
    remote = "json::Config",
    default = "json::Config::default",
    deny_unknown_fields
)]
#[allow(dead_code)]
#[schemars(rename = "JsonDisplayConfig")]
pub(super) struct JsonConfig {
    /// Style for {}.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub curly_brackets_style: ContentStyle,
    /// Style for [].
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub square_brackets_style: ContentStyle,
    /// Style for "key".
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub key_style: ContentStyle,
    /// Style for string values.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub string_value_style: ContentStyle,
    /// Style for number values.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub number_value_style: ContentStyle,
    /// Style for boolean values.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub boolean_value_style: ContentStyle,
    /// Style for null values.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub null_value_style: ContentStyle,

    /// Attribute for the selected line.
    #[serde(with = "termcfg::crossterm_config::attribute_serde")]
    #[schemars(with = "String")]
    pub active_item_attribute: Attribute,
    /// Attribute for unselected lines.
    #[serde(with = "termcfg::crossterm_config::attribute_serde")]
    #[schemars(with = "String")]
    pub inactive_item_attribute: Attribute,

    /// The number of spaces used for indentation in the rendered JSON structure.
    /// This value multiplies with the indentation level of a JSON element to determine
    /// the total indentation space. For example, an `indent` value of 4 means each
    /// indentation level will be 4 spaces wide.
    pub indent: usize,

    /// Rendering behavior when a line exceeds the terminal width.
    #[schemars(with = "JsonOverflowMode")]
    pub overflow_mode: json::config::OverflowMode,
    /// Number of lines available for rendering.
    pub lines: Option<usize>,
    /// Whether to display stable one-based line numbers to the left of the content.
    pub show_line_numbers: bool,
    /// Display immediate child counts next to collapsed and empty containers.
    /// Disabled by default; changing this does not change folding state.
    pub show_child_count: bool,
    /// Style for child count annotations, such as ` (3 items)` or ` (2 keys)`.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub child_count_style: ContentStyle,
}

#[derive(JsonSchema)]
#[schemars(remote = "json::config::OverflowMode")]
#[allow(dead_code)]
#[schemars(rename = "JsonOverflowMode")]
enum JsonOverflowMode {
    Truncate,
    Wrap,
}

#[derive(JsonSchema)]
#[schemars(
    remote = "yaml::Config",
    default = "yaml::Config::default",
    deny_unknown_fields
)]
#[allow(dead_code)]
#[schemars(rename = "YamlDisplayConfig")]
pub(super) struct YamlConfig {
    /// Style for `{}` and `{...}`.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub map_style: ContentStyle,

    /// Style for `[]` and `[...]`.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub sequence_style: ContentStyle,

    /// Style for YAML keys.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub key_style: ContentStyle,

    /// Style for YAML tags, such as `!MyTag`.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub tag_style: ContentStyle,

    /// Style for string values.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub string_style: ContentStyle,

    /// Style for number values.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub number_style: ContentStyle,

    /// Style for boolean values.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub boolean_style: ContentStyle,

    /// Style for null values.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub null_style: ContentStyle,

    /// Attribute for the selected line.
    #[serde(with = "termcfg::crossterm_config::attribute_serde")]
    #[schemars(with = "String")]
    pub active_item_attribute: Attribute,

    /// Attribute for unselected lines.
    #[serde(with = "termcfg::crossterm_config::attribute_serde")]
    #[schemars(with = "String")]
    pub inactive_item_attribute: Attribute,

    /// The number of spaces used for indentation in the rendered YAML structure.
    pub indent: usize,

    /// Rendering behavior when a line exceeds the terminal width.
    #[schemars(with = "YamlOverflowMode")]
    pub overflow_mode: yaml::config::OverflowMode,

    /// Number of lines available for rendering.
    pub lines: Option<usize>,
    /// Whether to display stable one-based line numbers to the left of the content.
    pub show_line_numbers: bool,
    /// Display immediate child counts next to collapsed and empty containers.
    /// Disabled by default; changing this does not change folding state.
    pub show_child_count: bool,
    /// Style for child count annotations, such as ` (3 items)` or ` (2 keys)`.
    #[serde(with = "termcfg::crossterm_config::content_style_serde")]
    #[schemars(with = "String")]
    pub child_count_style: ContentStyle,
}

#[derive(JsonSchema)]
#[schemars(remote = "yaml::config::OverflowMode")]
#[allow(dead_code)]
#[schemars(rename = "YamlOverflowMode")]
enum YamlOverflowMode {
    Truncate,
    Wrap,
}

fn schema_url() -> String {
    format!(
        "https://github.com/ynqa/vy/releases/download/v{}/config.schema.json",
        env!("CARGO_PKG_VERSION")
    )
}

// Serde accepts this legacy alias, but Schemars does not derive aliases.
pub(super) fn legacy_jaq_alias(schema: &mut schemars::Schema) {
    let properties = schema
        .get_mut("properties")
        .unwrap()
        .as_object_mut()
        .unwrap();
    let jaq = properties["jaq"].clone();
    properties.insert("jq".into(), jaq);
    schema.insert("not".into(), serde_json::json!({"required": ["jaq", "jq"]}));
}

fn generate() -> schemars::Schema {
    let mut schema = SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<super::Config>();
    schema.insert("$id".into(), schema_url().into());
    schema.insert("title".into(), "vy configuration".into());
    schema
}

#[test]
fn published_schema_matches_config_types() {
    let saved: serde_json::Value =
        serde_json::from_str(include_str!("../../config.schema.json")).unwrap();
    assert_eq!(
        saved,
        serde_json::to_value(generate()).unwrap(),
        "regenerate with cargo test config::schema::export -- --ignored --exact"
    );
    assert_eq!(
        super::DEFAULT_CONFIG.lines().next().unwrap(),
        format!("#:schema {}", schema_url())
    );
}

#[test]
fn remote_display_definitions_cover_promkit_fields() {
    let schema = serde_json::to_value(generate()).unwrap();
    for (name, value) in [
        (
            "JsonDisplayConfig",
            serde_json::to_value(json::Config::default()).unwrap(),
        ),
        (
            "YamlDisplayConfig",
            serde_json::to_value(yaml::Config::default()).unwrap(),
        ),
    ] {
        let actual = value
            .as_object()
            .unwrap()
            .keys()
            .collect::<std::collections::BTreeSet<_>>();
        let documented = schema["definitions"][name]["properties"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            actual, documented,
            "update the Schemars remote definition for {name}"
        );
    }
}

#[test]
#[ignore = "writes config.schema.json for release packaging"]
fn export() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config.schema.json");
    let schema = serde_json::to_string_pretty(&generate()).unwrap() + "\n";
    std::fs::write(path, schema).unwrap();
}
