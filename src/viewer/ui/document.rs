//! Document display state, input handling, and rendering.

use std::io::BufRead;

use promkit::widgets::json::jsonz::ContainerNode;
use promkit::{
    core::{ContentPosition, CreatedGraphemes, Widget},
    widgets::{
        json::{self, JsonHit, config::OverflowMode as JsonOverflowMode, jsonz::JsonNode},
        yaml::{self, YamlHit, config::OverflowMode as YamlOverflowMode, yamlz::YamlNode},
    },
};

use crate::{
    config::Config, document_source::DocumentSource, input_format::InputFormat,
    viewer::DocumentAction,
};

mod comparison;
pub(in crate::viewer) use comparison::{
    ComparisonPane, DocumentComparisonComponent, DocumentFocus,
};

#[derive(Clone)]
pub(crate) enum DocumentDisplayConfig {
    Json(json::Config),
    Yaml(yaml::Config),
}

impl DocumentDisplayConfig {
    pub(crate) fn from_config(format: InputFormat, config: &Config) -> Self {
        match format {
            InputFormat::Json => Self::Json(config.json.clone()),
            InputFormat::Yaml => Self::Yaml(config.yaml.clone()),
            InputFormat::Auto => unreachable!("input format must be resolved before parsing"),
        }
    }

    pub(crate) fn format(&self) -> InputFormat {
        match self {
            Self::Json(_) => InputFormat::Json,
            Self::Yaml(_) => InputFormat::Yaml,
        }
    }
}

enum DocumentState {
    Json(json::State),
    Yaml(yaml::State),
}

/// A JSON or YAML document with its display state, input handling, and rendering.
pub(crate) struct DocumentComponent {
    state: DocumentState,
}

impl DocumentComponent {
    pub(in crate::viewer) fn apply_action(
        &mut self,
        action: DocumentAction,
        click_position: Option<ContentPosition>,
        movement_lines: usize,
    ) {
        match action {
            DocumentAction::Up => self.up(movement_lines),
            DocumentAction::Down => self.down(movement_lines),
            DocumentAction::PageUp | DocumentAction::HalfPageUp => self.up(movement_lines),
            DocumentAction::PageDown | DocumentAction::HalfPageDown => self.down(movement_lines),
            DocumentAction::MoveToHead => {
                self.head();
            }
            DocumentAction::MoveToTail => {
                self.tail();
            }
            DocumentAction::Toggle => {
                match click_position.and_then(|position| self.hit_row(position)) {
                    Some(row_index) => self.toggle_at(row_index),
                    None => self.toggle(),
                }
            }
            DocumentAction::ExpandAll => self.expand_all(),
            DocumentAction::CollapseAll => self.collapse_all(),
            DocumentAction::ToggleOverflowMode => self.toggle_overflow_mode(),
            DocumentAction::ToggleLineNumbers => self.toggle_line_numbers(),
        }
    }

    pub(crate) fn display_config(&self) -> DocumentDisplayConfig {
        match &self.state {
            DocumentState::Json(state) => DocumentDisplayConfig::Json(state.config.clone()),
            DocumentState::Yaml(state) => DocumentDisplayConfig::Yaml(state.config.clone()),
        }
    }

    pub(crate) fn parse(
        reader: impl BufRead,
        display_config: DocumentDisplayConfig,
    ) -> anyhow::Result<Self> {
        match display_config {
            DocumentDisplayConfig::Json(config) => {
                let document = json::Document::from_reader(reader).map_err(anyhow::Error::from)?;
                Ok(Self {
                    state: DocumentState::Json(json::State { document, config }),
                })
            }
            DocumentDisplayConfig::Yaml(config) => {
                let document = yaml::Document::from_reader(reader).map_err(anyhow::Error::from)?;
                Ok(Self {
                    state: DocumentState::Yaml(yaml::State { document, config }),
                })
            }
        }
    }

    pub(crate) fn up(&mut self, lines: usize) {
        match &mut self.state {
            DocumentState::Json(state) => {
                for _ in 0..lines {
                    if !state.document.up() {
                        break;
                    }
                }
            }
            DocumentState::Yaml(state) => {
                for _ in 0..lines {
                    if !state.document.up() {
                        break;
                    }
                }
            }
        }
    }

    pub(crate) fn down(&mut self, lines: usize) {
        match &mut self.state {
            DocumentState::Json(state) => {
                for _ in 0..lines {
                    if !state.document.down() {
                        break;
                    }
                }
            }
            DocumentState::Yaml(state) => {
                for _ in 0..lines {
                    if !state.document.down() {
                        break;
                    }
                }
            }
        }
    }

    pub(crate) fn head(&mut self) {
        match &mut self.state {
            DocumentState::Json(state) => state.document.head(),
            DocumentState::Yaml(state) => state.document.head(),
        };
    }

    pub(crate) fn tail(&mut self) {
        match &mut self.state {
            DocumentState::Json(state) => state.document.tail(),
            DocumentState::Yaml(state) => state.document.tail(),
        };
    }

    pub(crate) fn move_to_path(&mut self, document_index: usize, path: &str) -> bool {
        match &mut self.state {
            DocumentState::Json(state) => state.document.move_to_path(document_index, path),
            DocumentState::Yaml(state) => state.document.move_to_path(document_index, path),
        }
    }

    pub(crate) fn selected_path(&self) -> Option<(usize, String)> {
        match &self.state {
            DocumentState::Json(state) => state.document.selected_path(),
            DocumentState::Yaml(state) => state.document.selected_path(),
        }
    }

    pub(crate) fn selected_value(&self, source: &DocumentSource) -> anyhow::Result<String> {
        let (document_index, path) = self
            .selected_path()
            .ok_or_else(|| anyhow::anyhow!("no node is selected"))?;
        source.value_at(document_index, &path)
    }

    pub(crate) fn selected_subtree(&self, source: &DocumentSource) -> anyhow::Result<String> {
        let (document_index, path) = self
            .selected_path()
            .ok_or_else(|| anyhow::anyhow!("no node is selected"))?;
        source.formatted_subtree_at(document_index, &path)
    }

    pub(crate) fn apply_display_config(&mut self, display_config: DocumentDisplayConfig) {
        match (&mut self.state, display_config) {
            (DocumentState::Json(state), DocumentDisplayConfig::Json(config)) => {
                state.config = config
            }
            (DocumentState::Yaml(state), DocumentDisplayConfig::Yaml(config)) => {
                state.config = config
            }
            _ => unreachable!("document format must not change while reloading configuration"),
        }
    }

    pub(crate) fn toggle(&mut self) {
        match &mut self.state {
            DocumentState::Json(state) => state.document.toggle(),
            DocumentState::Yaml(state) => state.document.toggle(),
        }
    }

    pub(crate) fn expand_selected(&mut self) -> bool {
        if self.selected_container_collapsed() == Some(true) {
            self.toggle();
            true
        } else {
            false
        }
    }

    pub(crate) fn collapse_selected(&mut self) -> bool {
        if self.selected_container_collapsed() == Some(false) {
            self.toggle();
            true
        } else {
            false
        }
    }

    fn selected_container_collapsed(&self) -> Option<bool> {
        let (document_index, path) = self.selected_path()?;
        match &self.state {
            DocumentState::Json(state) => {
                let row = state
                    .document
                    .row_index_for_path(document_index, &path)
                    .and_then(|index| state.document.rows().get(index))?;
                match &row.node {
                    JsonNode::Container(ContainerNode::Open { collapsed, .. }) => Some(*collapsed),
                    _ => None,
                }
            }
            DocumentState::Yaml(state) => {
                let row = state
                    .document
                    .row_index_for_path(document_index, &path)
                    .and_then(|index| state.document.rows().get(index))?;
                yaml_container_collapsed(&row.node)
            }
        }
    }

    pub(crate) fn expand_all(&mut self) {
        match &mut self.state {
            DocumentState::Json(state) => state.document.set_nodes_visibility(false),
            DocumentState::Yaml(state) => state.document.set_nodes_visibility(false),
        }
    }

    pub(crate) fn collapse_all(&mut self) {
        match &mut self.state {
            DocumentState::Json(state) => state.document.set_nodes_visibility(true),
            DocumentState::Yaml(state) => state.document.set_nodes_visibility(true),
        }
    }

    pub(crate) fn toggle_overflow_mode(&mut self) {
        match &mut self.state {
            DocumentState::Json(state) => {
                state.config.overflow_mode = match state.config.overflow_mode {
                    JsonOverflowMode::Truncate => JsonOverflowMode::Wrap,
                    JsonOverflowMode::Wrap => JsonOverflowMode::Truncate,
                };
            }
            DocumentState::Yaml(state) => {
                state.config.overflow_mode = match state.config.overflow_mode {
                    YamlOverflowMode::Truncate => YamlOverflowMode::Wrap,
                    YamlOverflowMode::Wrap => YamlOverflowMode::Truncate,
                };
            }
        }
    }

    pub(crate) fn toggle_line_numbers(&mut self) {
        match &mut self.state {
            DocumentState::Json(state) => {
                state.config.show_line_numbers = !state.config.show_line_numbers;
            }
            DocumentState::Yaml(state) => {
                state.config.show_line_numbers = !state.config.show_line_numbers;
            }
        }
    }

    pub(crate) fn toggle_at(&mut self, row_index: usize) {
        match &mut self.state {
            DocumentState::Json(state) => state.document.toggle_at(row_index),
            DocumentState::Yaml(state) => state.document.toggle_at(row_index),
        }
    }

    pub(crate) fn create_content_graphemes(&self, width: u16, height: u16) -> CreatedGraphemes {
        match &self.state {
            DocumentState::Json(state) => state.create_graphemes_in_viewport(width, height),
            DocumentState::Yaml(state) => state.create_graphemes_in_viewport(width, height),
        }
    }

    pub(crate) fn hit_row(&self, position: ContentPosition) -> Option<usize> {
        match &self.state {
            DocumentState::Json(state) => match state.hit_at_viewport(position) {
                Some(JsonHit::Toggle { row_index }) => Some(row_index),
                _ => None,
            },
            DocumentState::Yaml(state) => match state.hit_at_viewport(position) {
                Some(YamlHit::Toggle { row_index }) => Some(row_index),
                _ => None,
            },
        }
    }
}

fn yaml_container_collapsed(node: &YamlNode) -> Option<bool> {
    match node {
        YamlNode::Container(ContainerNode::Open { collapsed, .. }) => Some(*collapsed),
        YamlNode::Tagged { node, .. } => yaml_container_collapsed(node),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn display_config(format: InputFormat) -> DocumentDisplayConfig {
        DocumentDisplayConfig::from_config(format, &Config::default())
    }

    #[test]
    fn parses_json_document() {
        let reader = Box::new(Cursor::new(br#"{"name":"vy","items":[1,2]}"#));

        assert!(matches!(
            DocumentComponent::parse(reader, display_config(InputFormat::Json))
                .unwrap()
                .state,
            DocumentState::Json(_)
        ));
    }

    #[test]
    fn parses_yaml_document() {
        let reader = Box::new(Cursor::new(b"name: vy\nitems:\n  - 1\n  - 2\n"));

        assert!(matches!(
            DocumentComponent::parse(reader, display_config(InputFormat::Yaml))
                .unwrap()
                .state,
            DocumentState::Yaml(_)
        ));
    }

    #[test]
    fn moves_to_a_path_in_a_json_lines_document() {
        let reader = Box::new(Cursor::new(
            b"{\"first_only\":true}\n{\"second_only\":true}\n",
        ));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Json)).unwrap();

        assert!(document.move_to_path(1, ".second_only"));
        assert!(!document.move_to_path(0, ".second_only"));
    }

    #[test]
    fn moves_to_a_path_in_a_yaml_document_stream() {
        let reader = Box::new(Cursor::new(b"first_only: true\n---\nsecond_only: true\n"));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Yaml)).unwrap();

        assert!(document.move_to_path(1, ".second_only"));
        assert!(!document.move_to_path(0, ".second_only"));
    }

    #[test]
    fn toggles_json_overflow_mode() {
        let reader = Box::new(Cursor::new(br#"{"name":"vy"}"#));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Json)).unwrap();

        document.toggle_overflow_mode();
        let DocumentState::Json(state) = &document.state else {
            unreachable!();
        };
        assert_eq!(state.config.overflow_mode, JsonOverflowMode::Truncate);

        document.toggle_overflow_mode();
        let DocumentState::Json(state) = &document.state else {
            unreachable!();
        };
        assert_eq!(state.config.overflow_mode, JsonOverflowMode::Wrap);
    }

    #[test]
    fn toggles_yaml_overflow_mode() {
        let reader = Box::new(Cursor::new(b"name: vy\n"));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Yaml)).unwrap();

        document.toggle_overflow_mode();
        let DocumentState::Yaml(state) = &document.state else {
            unreachable!();
        };
        assert_eq!(state.config.overflow_mode, YamlOverflowMode::Truncate);

        document.toggle_overflow_mode();
        let DocumentState::Yaml(state) = &document.state else {
            unreachable!();
        };
        assert_eq!(state.config.overflow_mode, YamlOverflowMode::Wrap);
    }

    #[test]
    fn toggles_json_line_numbers() {
        let reader = Box::new(Cursor::new(br#"{"name":"vy"}"#));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Json)).unwrap();

        document.toggle_line_numbers();
        let DocumentState::Json(state) = &document.state else {
            unreachable!();
        };
        assert!(!state.config.show_line_numbers);

        document.toggle_line_numbers();
        let DocumentState::Json(state) = &document.state else {
            unreachable!();
        };
        assert!(state.config.show_line_numbers);
    }

    #[test]
    fn toggles_yaml_line_numbers() {
        let reader = Box::new(Cursor::new(b"name: vy\n"));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Yaml)).unwrap();

        document.toggle_line_numbers();
        let DocumentState::Yaml(state) = &document.state else {
            unreachable!();
        };
        assert!(!state.config.show_line_numbers);

        document.toggle_line_numbers();
        let DocumentState::Yaml(state) = &document.state else {
            unreachable!();
        };
        assert!(state.config.show_line_numbers);
    }

    #[test]
    fn expands_and_collapses_all_json_items() {
        let reader = Box::new(Cursor::new(br#"{"nested":{"value":1},"tail":2}"#));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Json)).unwrap();

        document.collapse_all();
        let DocumentState::Json(state) = &document.state else {
            unreachable!();
        };
        let collapsed_len = state.document.visible_rows().len();

        document.expand_all();
        let DocumentState::Json(state) = &document.state else {
            unreachable!();
        };
        assert!(state.document.visible_rows().len() > collapsed_len);
    }

    #[test]
    fn expands_and_collapses_all_yaml_items() {
        let reader = Box::new(Cursor::new(b"nested:\n  value: 1\ntail: 2\n"));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Yaml)).unwrap();

        document.collapse_all();
        let DocumentState::Yaml(state) = &document.state else {
            unreachable!();
        };
        let collapsed_len = state.document.visible_rows().len();

        document.expand_all();
        let DocumentState::Yaml(state) = &document.state else {
            unreachable!();
        };
        assert!(state.document.visible_rows().len() > collapsed_len);
    }

    #[test]
    fn expands_and_collapses_the_selected_json_item() {
        let reader = Box::new(Cursor::new(br#"{"nested":{"value":1}}"#));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Json)).unwrap();
        assert!(document.move_to_path(0, ".nested"));

        assert!(document.collapse_selected());
        assert!(!document.collapse_selected());
        assert!(document.expand_selected());
        assert!(!document.expand_selected());
    }

    #[test]
    fn expands_and_collapses_the_selected_tagged_yaml_item() {
        let reader = Box::new(Cursor::new(b"nested: !tag\n  value: 1\n"));
        let mut document =
            DocumentComponent::parse(reader, display_config(InputFormat::Yaml)).unwrap();
        assert!(document.move_to_path(0, ".nested"));

        assert!(document.collapse_selected());
        assert!(!document.collapse_selected());
        assert!(document.expand_selected());
        assert!(!document.expand_selected());
    }

    #[test]
    fn extracts_selected_json_value_and_subtree() {
        let input = br#"{"name":"vy","nested":{"enabled":true}}"#;
        let source =
            DocumentSource::from_reader(InputFormat::Json, Box::new(Cursor::new(input.to_vec())))
                .unwrap();
        let mut document =
            DocumentComponent::parse(source.content(), display_config(InputFormat::Json)).unwrap();

        assert!(document.move_to_path(0, ".name"));
        assert_eq!(document.selected_value(&source).unwrap(), "vy");
        assert_eq!(document.selected_subtree(&source).unwrap(), r#""vy""#);

        assert!(document.move_to_path(0, ".nested"));
        assert!(document.selected_value(&source).is_err());
        assert_eq!(
            document.selected_subtree(&source).unwrap(),
            "{\n  \"enabled\": true\n}"
        );
    }

    #[test]
    fn extracts_selected_yaml_value_and_subtree() {
        let input = b"name: vy\nnested:\n  enabled: true\n";
        let source =
            DocumentSource::from_reader(InputFormat::Yaml, Box::new(Cursor::new(input.to_vec())))
                .unwrap();
        let mut document =
            DocumentComponent::parse(source.content(), display_config(InputFormat::Yaml)).unwrap();

        assert!(document.move_to_path(0, ".name"));
        assert_eq!(document.selected_value(&source).unwrap(), "vy");
        assert_eq!(document.selected_subtree(&source).unwrap(), "vy");

        assert!(document.move_to_path(0, ".nested"));
        assert!(document.selected_value(&source).is_err());
        assert_eq!(document.selected_subtree(&source).unwrap(), "enabled: true");
    }
}
