use std::{collections::HashSet, fmt, sync::Arc, sync::atomic::AtomicBool};

use anyhow::{Context, ensure};
use serde::{
    Deserialize,
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
};

use crate::{document_source::DocumentSource, input_format::InputFormat};

use super::{
    Catalog, CatalogDocument, CatalogEntry, CatalogLiteral, CatalogPath, CatalogValue,
    DocumentIndex, EntryIndex, LiteralKind,
};

/// Builds a complete immutable [`Catalog`] from one input source.
pub(crate) struct Indexer {
    source: DocumentSource,
    cancelled: Arc<AtomicBool>,
}

impl Indexer {
    pub(crate) fn new(source: DocumentSource) -> Self {
        Self {
            source,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(super) fn with_cancellation(mut self, cancelled: Arc<AtomicBool>) -> Self {
        self.cancelled = cancelled;
        self
    }

    pub(crate) fn index(self) -> anyhow::Result<Catalog> {
        ensure!(!self.cancelled(), "catalog indexing was cancelled");
        match self.source.format() {
            InputFormat::Json => self.index_json(),
            InputFormat::Yaml => self.index_yaml(),
            InputFormat::Auto => unreachable!("input format must be resolved before cataloging"),
        }
    }

    fn index_json(&self) -> anyhow::Result<Catalog> {
        let mut documents = Vec::new();

        for (document_index, document) in
            serde_json::Deserializer::from_slice(self.source.content())
                .into_iter::<JsonCatalogDocument>()
                .enumerate()
        {
            ensure!(!self.cancelled(), "catalog indexing was cancelled");
            let document = document.context("failed to parse JSON for catalog")?;
            documents.push(document.0.finish(DocumentIndex(document_index)));
        }

        Ok(Catalog {
            documents: documents.into_boxed_slice(),
        })
    }

    fn index_yaml(&self) -> anyhow::Result<Catalog> {
        let mut documents = Vec::new();

        for (document_index, deserializer) in
            serde_yaml::Deserializer::from_slice(self.source.content()).enumerate()
        {
            ensure!(!self.cancelled(), "catalog indexing was cancelled");
            let mut indexer = DocumentIndexer::new(CatalogFormat::Yaml);
            CatalogSeed::root(&mut indexer)
                .deserialize(deserializer)
                .context("failed to parse YAML for catalog")?;
            documents.push(indexer.finish(DocumentIndex(document_index)));
        }

        Ok(Catalog {
            documents: documents.into_boxed_slice(),
        })
    }

    fn cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Acquire)
    }
}

struct JsonCatalogDocument(DocumentIndexer);

impl<'de> Deserialize<'de> for JsonCatalogDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut indexer = DocumentIndexer::new(CatalogFormat::Json);
        CatalogSeed::root(&mut indexer).deserialize(deserializer)?;
        Ok(Self(indexer))
    }
}

#[derive(Clone, Copy)]
enum CatalogFormat {
    Json,
    Yaml,
}

struct DocumentIndexer {
    format: CatalogFormat,
    entries: Vec<CatalogEntry>,
    children_by_parent: std::collections::HashMap<CatalogPath, Vec<EntryIndex>>,
}

impl DocumentIndexer {
    fn new(format: CatalogFormat) -> Self {
        Self {
            format,
            entries: Vec::new(),
            children_by_parent: std::collections::HashMap::new(),
        }
    }

    fn push(&mut self, location: &Option<CatalogLocation>, value: CatalogValue) {
        let Some(location) = location else {
            return;
        };

        let entry_index = EntryIndex(self.entries.len());
        if let Some(parent_path) = &location.parent_path {
            self.children_by_parent
                .entry(parent_path.clone())
                .or_default()
                .push(entry_index);
        }
        self.entries.push(CatalogEntry {
            path: location.path.clone(),
            last_segment_start: location.last_segment_start,
            value,
        });
    }

    fn finish(self, document_index: DocumentIndex) -> CatalogDocument {
        CatalogDocument {
            document_index,
            entries: self.entries.into_boxed_slice(),
            children_by_parent: self
                .children_by_parent
                .into_iter()
                .map(|(parent, children)| (parent, children.into_boxed_slice()))
                .collect(),
        }
    }
}

#[derive(Clone)]
struct CatalogLocation {
    path: CatalogPath,
    parent_path: Option<CatalogPath>,
    last_segment_start: Option<usize>,
}

struct CatalogSeed<'a> {
    indexer: &'a mut DocumentIndexer,
    location: Option<CatalogLocation>,
}

impl<'a> CatalogSeed<'a> {
    fn root(indexer: &'a mut DocumentIndexer) -> Self {
        Self {
            indexer,
            location: Some(CatalogLocation {
                path: CatalogPath(Arc::from(".")),
                parent_path: None,
                last_segment_start: None,
            }),
        }
    }
}

impl<'de> DeserializeSeed<'de> for CatalogSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(CatalogVisitor {
            indexer: self.indexer,
            location: self.location,
        })
    }
}

struct CatalogVisitor<'a> {
    indexer: &'a mut DocumentIndexer,
    location: Option<CatalogLocation>,
}

impl CatalogVisitor<'_> {
    fn push(self, value: CatalogValue) {
        self.indexer.push(&self.location, value);
    }

    fn push_literal(self, kind: LiteralKind, representation: impl Into<Box<str>>) {
        self.push(CatalogValue::Literal(CatalogLiteral {
            kind,
            representation: representation.into(),
        }));
    }
}

impl<'de> Visitor<'de> for CatalogVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON or YAML value")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        self.push_literal(LiteralKind::Null, "null");
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        self.push_literal(LiteralKind::Null, "null");
        Ok(())
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        self.push_literal(LiteralKind::Bool, value.to_string());
        Ok(())
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        self.push_literal(LiteralKind::Number, value.to_string());
        Ok(())
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        self.push_literal(LiteralKind::Number, value.to_string());
        Ok(())
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E> {
        self.push_literal(LiteralKind::Number, value.to_string());
        Ok(())
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E> {
        self.push_literal(LiteralKind::Number, value.to_string());
        Ok(())
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let representation = match self.indexer.format {
            CatalogFormat::Json => serde_json::Number::from_f64(value)
                .ok_or_else(|| E::custom("non-finite float is not valid JSON"))?
                .to_string(),
            CatalogFormat::Yaml => serde_yaml::Number::from(value).to_string(),
        };
        self.push_literal(LiteralKind::Number, representation);
        Ok(())
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_owned())
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_owned())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let representation = serde_json::to_string(&value)
            .map_err(|error| E::custom(format!("failed to serialize catalog string: {error}")))?;
        self.push_literal(LiteralKind::String, representation);
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        CatalogSeed {
            indexer: self.indexer,
            location: self.location,
        }
        .deserialize(deserializer)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self.visit_some(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let Self { indexer, location } = self;
        indexer.push(&location, CatalogValue::Array);
        let parent_path = location.map(|location| location.path);

        let mut index = 0;
        while sequence
            .next_element_seed(CatalogSeed {
                indexer: &mut *indexer,
                location: parent_path
                    .as_ref()
                    .map(|parent| append_index(parent, index)),
            })?
            .is_some()
        {
            index += 1;
        }
        Ok(())
    }

    fn visit_map<A>(self, mut mapping: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let Self { indexer, location } = self;
        indexer.push(&location, CatalogValue::Object);
        let parent_path = location.map(|location| location.path);

        match indexer.format {
            CatalogFormat::Json => {
                while let Some(key) = mapping.next_key::<String>()? {
                    mapping.next_value_seed(CatalogSeed {
                        indexer: &mut *indexer,
                        location: parent_path
                            .as_ref()
                            .map(|parent| append_string_key(parent, &key)),
                    })?;
                }
            }
            CatalogFormat::Yaml => {
                let mut keys = HashSet::with_capacity(mapping.size_hint().unwrap_or(0));
                while let Some(key) = mapping.next_key::<serde_yaml::Value>()? {
                    if !keys.insert(key.clone()) {
                        return Err(de::Error::custom("duplicate entry in YAML map"));
                    }
                    let location = parent_path
                        .as_ref()
                        .and_then(|parent| append_yaml_key(parent, &key));
                    mapping.next_value_seed(CatalogSeed {
                        indexer: &mut *indexer,
                        location,
                    })?;
                }
            }
        }
        Ok(())
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let (_tag, contents) = data.variant::<String>()?;
        contents.newtype_variant_seed(CatalogSeed {
            indexer: self.indexer,
            location: self.location,
        })
    }
}

fn append_yaml_key(parent: &CatalogPath, key: &serde_yaml::Value) -> Option<CatalogLocation> {
    match key {
        serde_yaml::Value::String(key) => Some(append_string_key(parent, key)),
        serde_yaml::Value::Number(key) => Some(append_bracket(parent, &key.to_string())),
        serde_yaml::Value::Bool(key) => Some(append_bracket(parent, &key.to_string())),
        serde_yaml::Value::Null => Some(append_bracket(parent, "null")),
        serde_yaml::Value::Tagged(_)
        | serde_yaml::Value::Sequence(_)
        | serde_yaml::Value::Mapping(_) => None,
    }
}

fn append_string_key(parent: &CatalogPath, key: &str) -> CatalogLocation {
    if is_identifier(key) {
        let path = if parent.as_ref() == "." {
            format!(".{key}")
        } else {
            format!("{}.{key}", parent.as_ref())
        };
        child_location(parent, path)
    } else {
        let key = serde_json::to_string(key).expect("serializing a path key cannot fail");
        append_bracket(parent, &key)
    }
}

fn append_index(parent: &CatalogPath, index: usize) -> CatalogLocation {
    append_bracket(parent, &index.to_string())
}

fn append_bracket(parent: &CatalogPath, value: &str) -> CatalogLocation {
    let prefix = if parent.as_ref() == "." {
        ""
    } else {
        parent.as_ref()
    };
    child_location(parent, format!("{prefix}[{value}]"))
}

fn child_location(parent: &CatalogPath, path: String) -> CatalogLocation {
    let last_segment_start = if parent.as_ref() == "." {
        0
    } else {
        parent.as_ref().len()
    };
    CatalogLocation {
        path: CatalogPath(Arc::from(path)),
        parent_path: Some(parent.clone()),
        last_segment_start: Some(last_segment_start),
    }
}

fn is_identifier(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || matches!(character, '_' | '$'))
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '$'))
        && !matches!(
            key,
            "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "debugger"
                | "default"
                | "delete"
                | "do"
                | "else"
                | "export"
                | "extends"
                | "false"
                | "finally"
                | "for"
                | "function"
                | "if"
                | "import"
                | "in"
                | "instanceof"
                | "new"
                | "null"
                | "return"
                | "super"
                | "switch"
                | "this"
                | "throw"
                | "true"
                | "try"
                | "typeof"
                | "var"
                | "void"
                | "while"
                | "with"
                | "yield"
        )
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use promkit::widgets::{json, yaml};

    use super::*;

    fn source(format: InputFormat, input: &str) -> DocumentSource {
        DocumentSource::from_reader(format, Box::new(Cursor::new(input.as_bytes().to_vec())))
            .unwrap()
    }

    fn catalog(format: InputFormat, input: &str) -> Catalog {
        Indexer::new(source(format, input)).index().unwrap()
    }

    fn flattened(catalog: &Catalog) -> Vec<String> {
        catalog
            .documents()
            .iter()
            .flat_map(CatalogDocument::flatten_entries)
            .collect()
    }

    #[test]
    fn indexes_json_in_preorder_including_empty_containers() {
        let catalog = catalog(InputFormat::Json, r#"[{"name":"Alice","0":true},[]]"#);

        assert_eq!(
            flattened(&catalog),
            [
                "@0 = []",
                "@0[0] = {}",
                r#"@0[0].name = "Alice""#,
                r#"@0[0]["0"] = true"#,
                "@0[1] = []",
            ]
        );
    }

    #[test]
    fn indexes_each_json_lines_value_as_a_document() {
        let catalog = catalog(InputFormat::Json, "{\"id\":1}\n{\"id\":2}\n");

        assert_eq!(
            flattened(&catalog),
            ["@0 = {}", "@0.id = 1", "@1 = {}", "@1.id = 2"]
        );
        assert_eq!(
            catalog
                .documents()
                .iter()
                .map(|document| document.document_index().get())
                .collect::<Vec<_>>(),
            [0, 1]
        );
    }

    #[test]
    fn indexes_multiple_yaml_documents_scalar_keys_and_tags() {
        let catalog = catalog(
            InputFormat::Yaml,
            concat!(
                "name: Alice\n",
                "\"true\": null\n",
                "true: false\n",
                "tagged: !catalog { nested: value }\n",
                "---\n",
                "items: [one, two]\n",
            ),
        );

        assert_eq!(
            flattened(&catalog),
            [
                "@0 = {}",
                r#"@0.name = "Alice""#,
                r#"@0["true"] = null"#,
                "@0[true] = false",
                "@0.tagged = {}",
                r#"@0.tagged.nested = "value""#,
                "@1 = {}",
                "@1.items = []",
                r#"@1.items[0] = "one""#,
                r#"@1.items[1] = "two""#,
            ]
        );
    }

    #[test]
    fn excludes_values_beneath_complex_yaml_mapping_keys() {
        let catalog = catalog(
            InputFormat::Yaml,
            concat!(
                "? [complex, key]\n",
                ": { nested: value }\n",
                "? { complex: key }\n",
                ": { hidden: value }\n",
                "visible: true\n",
            ),
        );

        assert_eq!(flattened(&catalog), ["@0 = {}", "@0.visible = true"]);
    }

    #[test]
    fn builds_direct_children_and_last_segments_once() {
        let catalog = catalog(
            InputFormat::Json,
            r#"{"users":[{"name":"Alice","email":"a@example.com"}]}"#,
        );
        let document = &catalog.documents()[0];
        let children = document.children(".users[0]");

        assert_eq!(children.len(), 2);
        assert_eq!(
            document.entry(children[0]).unwrap().segment(),
            Some(".name")
        );
        assert_eq!(
            document.entry(children[1]).unwrap().segment(),
            Some(".email")
        );
        assert!(document.children(".users[0].name").is_empty());
    }

    #[test]
    fn classifies_literal_kinds_and_preserves_display_representations() {
        let catalog = catalog(InputFormat::Json, r#"[null,true,12.5,"text"]"#);
        let entries = catalog.documents()[0].entries();

        let actual = entries[1..]
            .iter()
            .map(|entry| match entry.value() {
                CatalogValue::Literal(literal) => (literal.kind(), literal.representation()),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            [
                (LiteralKind::Null, "null"),
                (LiteralKind::Bool, "true"),
                (LiteralKind::Number, "12.5"),
                (LiteralKind::String, r#""text""#),
            ]
        );
    }

    #[test]
    fn rejects_invalid_input_and_duplicate_yaml_keys() {
        assert!(
            Indexer::new(source(InputFormat::Json, "{"))
                .index()
                .is_err()
        );
        let error = Indexer::new(source(InputFormat::Yaml, "name: first\nname: second\n"))
            .index()
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("duplicate entry in YAML map"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn generates_paths_resolvable_by_promkit_documents() {
        let json_input = concat!(
            "{\"first name\":{\"true\":null},\"items\":[1,2]}\n",
            "{\"second\":true}\n",
        );
        let json_catalog = catalog(InputFormat::Json, json_input);
        let document = json::Document::from_reader(json_input.as_bytes()).unwrap();
        for catalog_document in json_catalog.documents() {
            for entry in catalog_document.entries() {
                assert!(
                    document
                        .row_index_for_path(catalog_document.document_index().get(), entry.path())
                        .is_some(),
                    "unresolvable JSON path: {} {}",
                    catalog_document.document_index(),
                    entry.path()
                );
            }
        }

        let yaml_input = concat!(
            "\"first name\": { \"true\": null }\n",
            "true: false\n",
            "items: [one, two]\n",
            "? !key tagged\n",
            ": hidden\n",
            "---\n",
            "second: true\n",
        );
        let yaml_catalog = catalog(InputFormat::Yaml, yaml_input);
        let document = yaml::Document::from_reader(yaml_input.as_bytes()).unwrap();
        for catalog_document in yaml_catalog.documents() {
            for entry in catalog_document.entries() {
                assert!(
                    document
                        .row_index_for_path(catalog_document.document_index().get(), entry.path())
                        .is_some(),
                    "unresolvable YAML path: {} {}",
                    catalog_document.document_index(),
                    entry.path()
                );
            }
        }
    }
}
