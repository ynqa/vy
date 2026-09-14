mod flatten;
mod indexer;
mod loader;

use std::{borrow::Borrow, collections::HashMap, fmt, sync::Arc};

pub(crate) use flatten::{
    FlattenData, FlattenLoadState, FlattenLoader, FlattenSubscription, RelativeTarget,
};
pub(crate) use indexer::Indexer;
pub(crate) use loader::{CatalogLoadState, CatalogLoader, CatalogSubscription};

/// Zero-based index of a document in a JSON Lines or YAML document stream.
///
/// The index is stored once on [`CatalogDocument`] rather than repeated on every
/// [`CatalogEntry`]. It corresponds to the `@N` part of commands and displayed
/// catalog keys; for example, `DocumentIndex(1)` is displayed as `@1`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct DocumentIndex(pub(crate) usize);

impl DocumentIndex {
    #[allow(dead_code, reason = "part of the catalog's read-only consumer API")]
    pub(crate) fn get(self) -> usize {
        self.0
    }
}

impl fmt::Display for DocumentIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "@{}", self.0)
    }
}

/// Stable index of an entry within [`CatalogDocument::entries`].
///
/// Child relationships use this compact identifier instead of copying paths:
///
/// ```text
/// entries[EntryIndex(3)] = ".users[0].name"
/// children_by_parent[".users[0]"] = [EntryIndex(3)]
/// ```
///
/// Entry indices remain valid because entries are never reordered after the
/// catalog becomes ready.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct EntryIndex(pub(crate) usize);

/// A complete JSONPath used as the key within one catalog document.
///
/// The root is represented by `.`. Object properties and array elements are
/// appended using the same notation accepted by `goto`:
///
/// ```text
/// .
/// .users
/// .users[0]
/// .users[0].name
/// .users[0]["first.name"]
/// ```
///
/// The underlying string is reference-counted so a parent path can be shared
/// between an entry and [`CatalogDocument::children_by_parent`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct CatalogPath(Arc<str>);

impl Borrow<str> for CatalogPath {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for CatalogPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Completed, immutable catalog for every document in the input.
///
/// Documents and their entries retain input order. For a two-line JSON Lines
/// input, `documents[0]` represents `@0` and `documents[1]` represents `@1`.
/// The catalog is intended to be constructed completely in the background and
/// then shared as `Arc<Catalog>` by `goto`, `flatten`, and catalog search.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Catalog {
    /// Documents in input order.
    documents: Box<[CatalogDocument]>,
}

impl Catalog {
    /// Returns completed documents in input order.
    pub(crate) fn documents(&self) -> &[CatalogDocument] {
        &self.documents
    }

    /// Returns a document by its input index.
    pub(crate) fn document(&self, index: DocumentIndex) -> Option<&CatalogDocument> {
        self.documents.get(index.0)
    }

    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self {
            documents: Box::new([]),
        }
    }
}

/// Catalog data and direct-child index for one input document.
///
/// Given this input:
///
/// ```json
/// {"users":[{"name":"Alice","email":"alice@example.com"}]}
/// ```
///
/// the document is represented conceptually as:
///
/// ```text
/// entries:
///   0: .                     Object
///   1: .users                Array
///   2: .users[0]             Object
///   3: .users[0].name        "Alice"
///   4: .users[0].email       "alice@example.com"
///
/// children_by_parent:
///   "."             -> [1]
///   ".users"        -> [2]
///   ".users[0]"     -> [3, 4]
/// ```
///
/// `flatten` iterates `entries`, while `goto` completion looks up only the
/// relevant entry indices in `children_by_parent`.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct CatalogDocument {
    /// Index displayed as the `@N` portion of a catalog key.
    document_index: DocumentIndex,

    /// Catalog entries in preorder, preserving input object and array order.
    entries: Box<[CatalogEntry]>,

    /// Direct children of each parent path, stored as indices into `entries`.
    ///
    /// Only paths with children need an item in this map. The child slices
    /// retain input order and can be passed directly to completion search.
    children_by_parent: HashMap<CatalogPath, Box<[EntryIndex]>>,
}

impl CatalogDocument {
    pub(crate) fn document_index(&self) -> DocumentIndex {
        self.document_index
    }

    /// Returns entries in preorder and therefore in input order.
    #[allow(dead_code, reason = "reserved for the flatten view consumer")]
    pub(crate) fn entries(&self) -> &[CatalogEntry] {
        &self.entries
    }

    pub(crate) fn entry(&self, index: EntryIndex) -> Option<&CatalogEntry> {
        self.entries.get(index.0)
    }

    /// Returns only the direct children of `parent_path`, in input order.
    pub(crate) fn children(&self, parent_path: &str) -> &[EntryIndex] {
        self.children_by_parent
            .get(parent_path)
            .map_or(&[], AsRef::as_ref)
    }

    /// Builds one gron-style `K = V` line on demand.
    #[allow(dead_code, reason = "reserved for the flatten view consumer")]
    pub(crate) fn flatten_entry(&self, entry: &CatalogEntry) -> String {
        let path = entry.path();
        let key = if path == "." {
            self.document_index.to_string()
        } else {
            format!("{}{path}", self.document_index)
        };
        format!("{key} = {}", entry.value().representation())
    }

    /// Generates flatten output lazily without storing completed keys or lines.
    #[allow(dead_code, reason = "reserved for the flatten view consumer")]
    pub(crate) fn flatten_entries(&self) -> impl Iterator<Item = String> + '_ {
        self.entries().iter().map(|entry| self.flatten_entry(entry))
    }
}

/// One gron-style assignment before its document index and separators are
/// formatted for display.
///
/// A non-empty container also has an entry. For example, `.users` has
/// [`CatalogValue::Array`] and its elements appear as subsequent entries. This
/// allows flatten output to emit the container marker before its descendants.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct CatalogEntry {
    /// Complete path used as the K portion of a `K = V` assignment.
    path: CatalogPath,

    /// Byte offset of the final path segment, or `None` for the root entry.
    ///
    /// For example, the final segment of `.users[0].name` starts at byte
    /// position `9`:
    ///
    /// ```text
    /// .users[0].name
    ///          ^
    ///          byte position 9
    /// ```
    ///
    /// Slicing the path as `path[9..]` therefore returns `.name`. Storing the
    /// offset avoids allocating a second string solely for completion display.
    last_segment_start: Option<usize>,

    /// Preclassified value used as the V portion of a `K = V` assignment.
    value: CatalogValue,
}

impl CatalogEntry {
    /// Returns the complete path, such as `.users[0].name`.
    pub(crate) fn path(&self) -> &str {
        self.path.as_ref()
    }

    /// Returns the final path segment used for `goto` completion.
    ///
    /// ```text
    /// path:    ".users[0].name"
    /// segment: Some(".name")
    ///
    /// path:    "."
    /// segment: None
    /// ```
    pub(crate) fn segment(&self) -> Option<&str> {
        self.last_segment_start.map(|start| &self.path()[start..])
    }

    #[allow(dead_code, reason = "part of the catalog's read-only consumer API")]
    pub(crate) fn value(&self) -> &CatalogValue {
        &self.value
    }
}

/// Value assigned to a catalog path.
///
/// Containers are always represented by empty gron-style markers, even when
/// they have descendants:
///
/// ```text
/// Object  -> {}
/// Array   -> []
/// Literal -> null, true, 42, or "Alice"
/// ```
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum CatalogValue {
    /// Object marker rendered as `{}`.
    Object,

    /// Array marker rendered as `[]`.
    Array,

    /// Scalar value serialized by the indexer.
    Literal(CatalogLiteral),
}

impl CatalogValue {
    #[allow(dead_code, reason = "reserved for the flatten view consumer")]
    pub(crate) fn representation(&self) -> &str {
        match self {
            Self::Object => "{}",
            Self::Array => "[]",
            Self::Literal(literal) => literal.representation(),
        }
    }
}

/// Scalar value with its semantic kind and display-ready representation.
///
/// The indexer performs quoting and escaping once. For example, a string value
/// `Alice` is stored with the representation `"Alice"`, so flatten only needs
/// to concatenate the document index, path, separator, and representation.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct CatalogLiteral {
    /// Scalar kind retained for styling and future format-specific behavior.
    kind: LiteralKind,

    /// Serialized scalar fragment, such as `null`, `12.5`, or `"Alice"`.
    representation: Box<str>,
}

impl CatalogLiteral {
    #[allow(dead_code, reason = "reserved for catalog value styling")]
    pub(crate) fn kind(&self) -> LiteralKind {
        self.kind
    }

    #[allow(dead_code, reason = "reserved for the flatten view consumer")]
    pub(crate) fn representation(&self) -> &str {
        &self.representation
    }
}

/// Semantic kind of a serialized [`CatalogLiteral`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LiteralKind {
    /// Null value represented as `null`.
    Null,

    /// Boolean represented as `true` or `false`.
    Bool,

    /// Number retaining the parser's serialized representation.
    Number,

    /// Quoted and escaped string representation.
    String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_indices_reference_children_without_copying_paths() {
        let root_path = CatalogPath(Arc::from("."));
        let name_path = CatalogPath(Arc::from(".name"));
        let entries = vec![
            CatalogEntry {
                path: root_path.clone(),
                last_segment_start: None,
                value: CatalogValue::Object,
            },
            CatalogEntry {
                path: name_path,
                last_segment_start: Some(0),
                value: CatalogValue::Literal(CatalogLiteral {
                    kind: LiteralKind::String,
                    representation: r#""Alice""#.into(),
                }),
            },
        ]
        .into_boxed_slice();
        let children_by_parent =
            HashMap::from([(root_path, vec![EntryIndex(1)].into_boxed_slice())]);
        let document = CatalogDocument {
            document_index: DocumentIndex(0),
            entries,
            children_by_parent,
        };

        let children = document.children_by_parent.get(".").unwrap();
        let child = &document.entries[children[0].0];

        assert_eq!(child.path(), ".name");
        assert_eq!(child.segment(), Some(".name"));
    }

    #[test]
    fn last_segment_start_points_to_the_final_path_segment() {
        let entry = CatalogEntry {
            path: CatalogPath(Arc::from(".users[0].name")),
            last_segment_start: Some(9),
            value: CatalogValue::Literal(CatalogLiteral {
                kind: LiteralKind::String,
                representation: r#""Alice""#.into(),
            }),
        };

        assert_eq!(entry.segment(), Some(".name"));
    }

    #[test]
    fn formats_flatten_entries_without_storing_completed_lines() {
        let root_path = CatalogPath(Arc::from("."));
        let users_path = CatalogPath(Arc::from(".users"));
        let document = CatalogDocument {
            document_index: DocumentIndex(0),
            entries: vec![
                CatalogEntry {
                    path: root_path,
                    last_segment_start: None,
                    value: CatalogValue::Object,
                },
                CatalogEntry {
                    path: users_path,
                    last_segment_start: Some(0),
                    value: CatalogValue::Array,
                },
            ]
            .into_boxed_slice(),
            children_by_parent: HashMap::new(),
        };

        assert_eq!(
            document.flatten_entries().collect::<Vec<_>>(),
            ["@0 = {}", "@0.users = []"]
        );
    }
}
