//! Source data and path-based extraction, independent of the viewer.

use serde::Deserialize;
use std::{
    io::{BufRead, Read},
    sync::Arc,
};

use crate::input_format::InputFormat;

#[derive(Clone)]
pub(crate) struct DocumentSource {
    format: InputFormat,
    content: Arc<[u8]>,
}

impl DocumentSource {
    pub(crate) fn from_reader(
        format: InputFormat,
        mut reader: Box<dyn BufRead>,
    ) -> anyhow::Result<Self> {
        let mut content = Vec::new();
        reader.read_to_end(&mut content)?;
        Ok(Self {
            format,
            content: content.into(),
        })
    }

    pub(crate) fn format(&self) -> InputFormat {
        self.format
    }

    pub(crate) fn content(&self) -> &[u8] {
        &self.content
    }
    /// Decode a string node once, preserving literal backslashes and line breaks.
    pub(crate) fn text_at(&self, document_index: usize, path: &str) -> anyhow::Result<String> {
        let source = self.subtree(document_index, path)?;
        match self.format {
            InputFormat::Json => serde_json::from_slice::<String>(source.content())
                .map_err(|_| anyhow::anyhow!("preview requires a string node")),
            InputFormat::Yaml => {
                let mut value: serde_yaml::Value = serde_yaml::from_slice(source.content())?;
                while let serde_yaml::Value::Tagged(tagged) = value {
                    value = tagged.value;
                }
                match value {
                    serde_yaml::Value::String(text) => Ok(text),
                    _ => anyhow::bail!("preview requires a string node"),
                }
            }
            InputFormat::Auto => {
                unreachable!("input format must be resolved before extracting text")
            }
        }
    }

    /// Extract one node as an independent document in the same format. JSON is kept raw to
    /// preserve key order and numeric spelling; YAML remains YAML, including scalar types/tags.
    pub(crate) fn subtree(&self, document_index: usize, path: &str) -> anyhow::Result<Self> {
        let segments = parse_path(path)?;
        let content = match self.format {
            InputFormat::Json => {
                use serde_json::value::RawValue;
                let mut value = serde_json::Deserializer::from_slice(self.content())
                    .into_iter::<&RawValue>()
                    .nth(document_index)
                    .transpose()?
                    .ok_or_else(|| anyhow::anyhow!("document not found: @{document_index}"))?;
                for segment in segments {
                    value = match segment {
                        PathSegment::Key(key) => {
                            let mapping: std::collections::HashMap<String, &RawValue> =
                                serde_json::from_str(value.get())?;
                            mapping.get(&key).copied()
                        }
                        PathSegment::Bracket(key) if value.get().starts_with('{') => {
                            let mapping: std::collections::HashMap<String, &RawValue> =
                                serde_json::from_str(value.get())?;
                            let key: String = serde_json::from_str(&key)?;
                            mapping.get(&key).copied()
                        }
                        PathSegment::Bracket(index) => {
                            let sequence: Vec<&RawValue> = serde_json::from_str(value.get())?;
                            sequence.get(index.parse::<usize>()?).copied()
                        }
                    }
                    .ok_or_else(|| anyhow::anyhow!("selected path no longer exists"))?;
                }
                value.get().to_owned()
            }
            InputFormat::Yaml => {
                let document = serde_yaml::Deserializer::from_slice(self.content())
                    .nth(document_index)
                    .ok_or_else(|| anyhow::anyhow!("document not found: @{document_index}"))?;
                let document = serde_yaml::Value::deserialize(document)?;
                serde_yaml::to_string(select_yaml(&document, &segments)?)?
            }
            InputFormat::Auto => {
                unreachable!("input format must be resolved before extracting a subtree")
            }
        };
        Ok(Self {
            format: self.format,
            content: content.into_bytes().into(),
        })
    }

    pub(crate) fn value_at(&self, document_index: usize, path: &str) -> anyhow::Result<String> {
        let selected = self.node_at(document_index, path)?;
        match selected {
            SelectedNode::Json(value) => match value {
                serde_json::Value::Null => Ok("null".to_owned()),
                serde_json::Value::Bool(value) => Ok(value.to_string()),
                serde_json::Value::Number(value) => Ok(value.to_string()),
                serde_json::Value::String(value) => Ok(value),
                serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                    Err(anyhow::anyhow!("selected node is not a scalar"))
                }
            },
            SelectedNode::Yaml(value) => yaml_scalar(value),
        }
    }

    pub(crate) fn formatted_subtree_at(
        &self,
        document_index: usize,
        path: &str,
    ) -> anyhow::Result<String> {
        match self.node_at(document_index, path)? {
            SelectedNode::Json(value) => serde_json::to_string_pretty(&value).map_err(Into::into),
            SelectedNode::Yaml(value) => serde_yaml::to_string(&value)
                .map(|value| value.trim_end().to_owned())
                .map_err(Into::into),
        }
    }

    fn node_at(&self, document_index: usize, path: &str) -> anyhow::Result<SelectedNode> {
        let segments = parse_path(path)?;
        match self.format() {
            InputFormat::Json => {
                let document = serde_json::Deserializer::from_slice(self.content())
                    .into_iter::<serde_json::Value>()
                    .nth(document_index)
                    .transpose()?
                    .ok_or_else(|| anyhow::anyhow!("document not found: @{document_index}"))?;
                Ok(SelectedNode::Json(
                    select_json(&document, &segments)?.clone(),
                ))
            }
            InputFormat::Yaml => {
                let document = serde_yaml::Deserializer::from_slice(self.content())
                    .nth(document_index)
                    .ok_or_else(|| anyhow::anyhow!("document not found: @{document_index}"))?;
                let document = serde_yaml::Value::deserialize(document)?;
                Ok(SelectedNode::Yaml(
                    select_yaml(&document, &segments)?.clone(),
                ))
            }
            InputFormat::Auto => unreachable!("input format must be resolved before parsing"),
        }
    }
}

enum SelectedNode {
    Json(serde_json::Value),
    Yaml(serde_yaml::Value),
}

#[derive(Debug, PartialEq, Eq)]
enum PathSegment {
    Key(String),
    Bracket(String),
}

fn parse_path(path: &str) -> anyhow::Result<Vec<PathSegment>> {
    if path == "." {
        return Ok(Vec::new());
    }

    let characters = path.chars().collect::<Vec<_>>();
    let mut segments = Vec::new();
    let mut position = 0;
    while position < characters.len() {
        match characters[position] {
            '.' => {
                position += 1;
                let start = position;
                while position < characters.len() && !matches!(characters[position], '.' | '[') {
                    position += 1;
                }
                if start == position {
                    return Err(anyhow::anyhow!("invalid selected path: {path}"));
                }
                segments.push(PathSegment::Key(
                    characters[start..position].iter().collect(),
                ));
            }
            '[' => {
                position += 1;
                let start = position;
                let mut quoted = false;
                let mut escaped = false;
                while position < characters.len() {
                    let character = characters[position];
                    if escaped {
                        escaped = false;
                    } else if quoted && character == '\\' {
                        escaped = true;
                    } else if character == '"' {
                        quoted = !quoted;
                    } else if character == ']' && !quoted {
                        break;
                    }
                    position += 1;
                }
                if position == characters.len() || start == position {
                    return Err(anyhow::anyhow!("invalid selected path: {path}"));
                }
                segments.push(PathSegment::Bracket(
                    characters[start..position].iter().collect(),
                ));
                position += 1;
            }
            _ => return Err(anyhow::anyhow!("invalid selected path: {path}")),
        }
    }
    Ok(segments)
}

fn select_json<'a>(
    mut value: &'a serde_json::Value,
    segments: &[PathSegment],
) -> anyhow::Result<&'a serde_json::Value> {
    for segment in segments {
        value = match (value, segment) {
            (serde_json::Value::Object(mapping), PathSegment::Key(key)) => mapping.get(key),
            (serde_json::Value::Object(mapping), PathSegment::Bracket(key)) => {
                serde_json::from_str::<String>(key)
                    .ok()
                    .and_then(|key| mapping.get(&key))
            }
            (serde_json::Value::Array(sequence), PathSegment::Bracket(index)) => index
                .parse::<usize>()
                .ok()
                .and_then(|index| sequence.get(index)),
            _ => None,
        }
        .ok_or_else(|| anyhow::anyhow!("selected path no longer exists"))?;
    }
    Ok(value)
}

fn select_yaml<'a>(
    mut value: &'a serde_yaml::Value,
    segments: &[PathSegment],
) -> anyhow::Result<&'a serde_yaml::Value> {
    for segment in segments {
        while let serde_yaml::Value::Tagged(tagged) = value {
            value = &tagged.value;
        }
        value = match (value, segment) {
            (serde_yaml::Value::Mapping(mapping), PathSegment::Key(key)) => {
                mapping.get(serde_yaml::Value::String(key.clone()))
            }
            (serde_yaml::Value::Mapping(mapping), PathSegment::Bracket(key)) => {
                serde_yaml::from_str::<serde_yaml::Value>(key)
                    .ok()
                    .and_then(|key| mapping.get(&key))
            }
            (serde_yaml::Value::Sequence(sequence), PathSegment::Bracket(index)) => index
                .parse::<usize>()
                .ok()
                .and_then(|index| sequence.get(index)),
            _ => None,
        }
        .ok_or_else(|| anyhow::anyhow!("selected path no longer exists"))?;
    }
    Ok(value)
}

fn yaml_scalar(value: serde_yaml::Value) -> anyhow::Result<String> {
    match value {
        serde_yaml::Value::Null => Ok("null".to_owned()),
        serde_yaml::Value::Bool(value) => Ok(value.to_string()),
        serde_yaml::Value::Number(value) => Ok(value.to_string()),
        serde_yaml::Value::String(value) => Ok(value),
        serde_yaml::Value::Tagged(tagged) => yaml_scalar(tagged.value),
        serde_yaml::Value::Sequence(_) | serde_yaml::Value::Mapping(_) => {
            Err(anyhow::anyhow!("selected node is not a scalar"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn extracts_raw_json_subtrees_without_reordering_or_rounding() {
        let input = br#"{} {"items":[{"z":1e+02,"a":9007199254740993,"first]name":"ok"}]}"#;
        let source =
            DocumentSource::from_reader(InputFormat::Json, Box::new(Cursor::new(input.to_vec())))
                .unwrap();
        let selected = source.subtree(1, ".items[0]").unwrap();
        assert_eq!(
            selected.content(),
            br#"{"z":1e+02,"a":9007199254740993,"first]name":"ok"}"#
        );
        assert_eq!(
            source
                .subtree(1, r#".items[0]["first]name"]"#)
                .unwrap()
                .content(),
            br#""ok""#
        );
        assert!(source.subtree(2, ".").is_err());
        assert!(source.subtree(1, ".missing").is_err());
        assert!(source.subtree(1, ".items[2]").is_err());
    }

    #[test]
    fn extracts_yaml_subtrees_preserving_tags_and_scalar_types() {
        let input = b"first: doc\n---\nitems:\n  - !tag {z: 'true', a: 2}\n";
        let source =
            DocumentSource::from_reader(InputFormat::Yaml, Box::new(Cursor::new(input.to_vec())))
                .unwrap();
        let selected = source.subtree(1, ".items[0]").unwrap();
        assert_eq!(selected.format(), InputFormat::Yaml);
        let value: serde_yaml::Value = serde_yaml::from_slice(selected.content()).unwrap();
        let serde_yaml::Value::Tagged(tagged) = value else {
            panic!("tag must be preserved");
        };
        assert_eq!(tagged.tag.to_string(), "!tag");
        let scalar = source.subtree(1, ".items[0].z").unwrap();
        assert_eq!(
            serde_yaml::from_slice::<serde_yaml::Value>(scalar.content()).unwrap(),
            serde_yaml::Value::String("true".into())
        );
        assert!(source.subtree(2, ".").is_err());
        assert!(source.subtree(1, ".missing").is_err());
    }

    #[test]
    fn parses_jq_style_path_segments() {
        assert_eq!(
            parse_path(r#".items[0]["first]name"]"#).unwrap(),
            vec![
                PathSegment::Key("items".into()),
                PathSegment::Bracket("0".into()),
                PathSegment::Bracket(r#""first]name""#.into()),
            ]
        );
    }
}
