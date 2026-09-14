use std::{collections::HashSet, ops::Range};

#[cfg(test)]
use std::sync::Arc;

use nucleo_matcher::{
    Config as MatcherConfig, Matcher,
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
};

use crate::catalog::{
    Catalog, CatalogDocument, CatalogEntry, CatalogLoadState, CatalogSubscription, DocumentIndex,
};

const MAX_COMPLETION_CANDIDATES: usize = 100;

use crate::command::{CompletionCandidate, CompletionRequest, CompletionState};

/// Completes document indices and paths from the receiving side of a catalog load.
pub(crate) struct PathCompleter {
    catalog: CatalogSubscription,
    matcher: Matcher,
}

impl PathCompleter {
    pub(crate) fn new(catalog: CatalogSubscription) -> Self {
        let mut config = MatcherConfig::DEFAULT;
        config.prefer_prefix = true;
        Self {
            catalog,
            matcher: Matcher::new(config),
        }
    }

    /// Cursor and replacement ranges are character offsets within the arguments.
    pub(crate) fn complete(&mut self, arguments: &str, cursor: usize) -> CompletionState {
        let Some(trigger) = parse_completion_trigger(arguments, cursor) else {
            return CompletionState::None;
        };

        match self.catalog.state_and_mark_seen() {
            CatalogLoadState::Loading => CompletionState::Loading,
            CatalogLoadState::Failed(error) => CompletionState::Failed(error),
            CatalogLoadState::Ready(catalog) => CompletionState::Ready(complete_resolved_trigger(
                &mut self.matcher,
                resolve_completion_trigger(trigger, &catalog),
            )),
        }
    }

    pub(crate) fn catalog_changed(&self) -> bool {
        self.catalog.has_changed()
    }

    #[cfg(test)]
    pub(crate) fn from_catalog(catalog: Catalog) -> Self {
        Self::from_state(CatalogLoadState::Ready(Arc::new(catalog)))
    }

    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self::from_catalog(Catalog::empty())
    }

    #[cfg(test)]
    pub(crate) fn from_state(state: CatalogLoadState) -> Self {
        Self::new(CatalogSubscription::from_state(state))
    }
}

/// The delimiter that starts one path segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PathDelimiter {
    /// Dot notation, as in `.users` from `@0 .us`.
    Dot,

    /// Bracket notation, as in `[0]` from `@0 .users[0`.
    Bracket,
}

impl PathDelimiter {
    fn from_character(character: char) -> Option<Self> {
        match character {
            '.' => Some(Self::Dot),
            '[' => Some(Self::Bracket),
            _ => None,
        }
    }

    fn character(self) -> char {
        match self {
            Self::Dot => '.',
            Self::Bracket => '[',
        }
    }

    fn strip_from_segment(self, segment: &str) -> Option<&str> {
        segment.strip_prefix(self.character())
    }
}

/// The catalog position whose direct children should be completed.
///
/// For example, the cursor in `@0 .users[0].na` has document `@0` and
/// parent path `.users[0]`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PathLocation {
    document_index: DocumentIndex,
    parent_path: String,
}

/// The cursor position within a non-empty segment query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SegmentQueryPosition {
    /// The cursor is inside an existing segment; for example,
    /// `@0 .na|me`, where `|` marks the cursor.
    WithinSegment,

    /// The cursor is at the segment end; for example, `@0 .name|`. An
    /// exact container may therefore proceed to its next delimiter.
    AtSegmentEnd,
}

/// A syntax-level reason that completion was requested.
///
/// Parsing this type does not inspect the catalog. That second step produces a
/// [`ResolvedCompletionTrigger`]. Keeping the two steps separate makes inputs
/// such as `@0 `, `@0 .`, and `@0 .us` distinct without relying
/// on empty-string or delimiter-string checks in the completion logic.
#[derive(Debug, PartialEq, Eq)]
enum ParsedCompletionTrigger {
    /// Completes a document index.
    ///
    /// For example, `@1` produces query `@1` and replaces the `@1` range.
    DocumentIndex {
        query: String,
        replacement: Range<usize>,
    },

    /// Completes the delimiter of the next path segment.
    ///
    /// For example, `@0 ` points at the root of `@0` before either `.` or
    /// `[` has been entered.
    PathStart {
        location: PathLocation,
        replacement: Range<usize>,
    },

    /// Completes a segment immediately after its delimiter.
    ///
    /// For example, `@0 .` uses [`PathDelimiter::Dot`] with no query, and
    /// `@0 .users[` uses [`PathDelimiter::Bracket`] with no query.
    SegmentStart {
        location: PathLocation,
        delimiter: PathDelimiter,
        replacement: Range<usize>,
    },

    /// Completes the typed portion of a path segment.
    ///
    /// For example, `@0 .users[0].na` has parent `.users[0]`, delimiter
    /// [`PathDelimiter::Dot`], and query `na`. The delimiter is deliberately
    /// excluded from both `query` and `replacement`.
    SegmentQuery {
        location: PathLocation,
        delimiter: PathDelimiter,
        query: String,
        replacement: Range<usize>,
        position: SegmentQueryPosition,
    },
}

fn parse_completion_trigger(input: &str, cursor: usize) -> Option<ParsedCompletionTrigger> {
    let input = input.chars().collect::<Vec<_>>();
    let cursor = cursor.min(input.len());
    let arguments_start = skip_whitespace(&input, 0);
    if cursor < arguments_start {
        return None;
    }
    let document_end = input[arguments_start..]
        .iter()
        .position(|character| character.is_whitespace())
        .map_or(input.len(), |offset| arguments_start + offset);

    if cursor <= document_end {
        return Some(ParsedCompletionTrigger::DocumentIndex {
            query: input[arguments_start..cursor].iter().collect(),
            replacement: arguments_start..document_end,
        });
    }

    let document = input[arguments_start..document_end]
        .iter()
        .collect::<String>();
    let document_index = DocumentIndex(document.strip_prefix('@')?.parse().ok()?);
    let path_start = skip_whitespace(&input, document_end);
    if cursor < path_start {
        return None;
    }

    let path = &input[path_start..];
    let path_cursor = cursor - path_start;
    let segment = current_segment(path, path_cursor);
    let parent_path = if segment.start == 0 {
        ".".to_owned()
    } else {
        path[..segment.start].iter().collect()
    };

    let location = PathLocation {
        document_index,
        parent_path,
    };
    let segment_replacement = path_start + segment.start..path_start + segment.end;
    if path_cursor == segment.start {
        return Some(ParsedCompletionTrigger::PathStart {
            location,
            replacement: segment_replacement,
        });
    }
    let delimiter = path
        .get(segment.start)
        .copied()
        .and_then(PathDelimiter::from_character)?;

    let query_start = segment.start + 1;
    let replacement = path_start + query_start..path_start + segment.end;
    if path_cursor == query_start {
        return Some(ParsedCompletionTrigger::SegmentStart {
            location,
            delimiter,
            replacement,
        });
    }

    Some(ParsedCompletionTrigger::SegmentQuery {
        location,
        delimiter,
        query: path[query_start..path_cursor].iter().collect(),
        replacement,
        position: if path_cursor == segment.end {
            SegmentQueryPosition::AtSegmentEnd
        } else {
            SegmentQueryPosition::WithinSegment
        },
    })
}

fn skip_whitespace(input: &[char], mut position: usize) -> usize {
    while input
        .get(position)
        .is_some_and(|character| character.is_whitespace())
    {
        position += 1;
    }
    position
}

fn current_segment(path: &[char], cursor: usize) -> Range<usize> {
    let ranges = segment_ranges(path);
    ranges
        .iter()
        .find(|range| range.start == cursor)
        .or_else(|| {
            ranges
                .iter()
                .find(|range| range.start < cursor && cursor <= range.end)
        })
        .cloned()
        .unwrap_or(cursor..cursor)
}

fn segment_ranges(path: &[char]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;

    while start < path.len() {
        let mut end = start + 1;
        match path[start] {
            '.' => {
                while end < path.len() && !matches!(path[end], '.' | '[') {
                    end += 1;
                }
            }
            '[' => {
                let mut quoted = false;
                let mut escaped = false;
                while end < path.len() {
                    let character = path[end];
                    if quoted {
                        if escaped {
                            escaped = false;
                        } else if character == '\\' {
                            escaped = true;
                        } else if character == '"' {
                            quoted = false;
                        }
                    } else if character == '"' {
                        quoted = true;
                    } else if character == ']' {
                        end += 1;
                        break;
                    }
                    end += 1;
                }
            }
            _ => {
                while end < path.len() && !matches!(path[end], '.' | '[') {
                    end += 1;
                }
            }
        }
        ranges.push(start..end);
        start = end;
    }

    ranges
}

/// Why a parsed trigger cannot produce catalog-backed candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NoCandidateReason {
    /// The requested document does not exist; for example, `@9 .` when
    /// the catalog contains only `@0`.
    DocumentNotFound,

    /// The requested parent has no direct children; for example, completing
    /// after `@0 .name.` when `.name` is a string.
    ParentHasNoChildren,

    /// The typed segment is an exact leaf; for example, `@0 .name` when
    /// `.name` is a string and therefore has no next delimiter.
    ExactEntryHasNoChildren,
}

/// How direct path segments should be searched.
#[derive(Debug, PartialEq, Eq)]
enum SegmentSearch {
    /// Returns every segment of one delimiter type; for example, `@0 .`
    /// returns all dot-prefixed direct children under the root.
    All,

    /// Fuzzy-matches the typed text; for example, `@0 .us` searches dot
    /// segments using `us` and records whether the cursor is at the end.
    Query {
        text: String,
        position: SegmentQueryPosition,
    },
}

/// A completion trigger after resolving its catalog location.
///
/// Unlike [`ParsedCompletionTrigger`], each variant states which collection is
/// to be searched. Candidate generation therefore does not branch on special
/// query strings such as `""`, `"."`, or `"["`.
enum ResolvedCompletionTrigger<'catalog> {
    /// Searches document indices; for example, `@1` searches `@0`, `@1`,
    /// ... by the `@1` prefix.
    DocumentIndices {
        catalog: &'catalog Catalog,
        query: String,
        replacement: Range<usize>,
    },

    /// Offers the available next delimiters; for example, `@0 ` may offer
    /// both `.` and `[` depending on the root's direct children.
    Delimiters {
        document: &'catalog CatalogDocument,
        location: PathLocation,
        replacement: Range<usize>,
    },

    /// Searches direct child segments of one notation; for example,
    /// `@0 .us` searches dot segments under the root using query `us`.
    Segments {
        document: &'catalog CatalogDocument,
        location: PathLocation,
        delimiter: PathDelimiter,
        search: SegmentSearch,
        replacement: Range<usize>,
    },

    /// Produces an empty candidate list; for example, `@0 .name` when the
    /// exact `.name` entry is a leaf.
    NoCandidates {
        reason: NoCandidateReason,
        replacement: Range<usize>,
    },
}

fn resolve_completion_trigger(
    trigger: ParsedCompletionTrigger,
    catalog: &Catalog,
) -> ResolvedCompletionTrigger<'_> {
    match trigger {
        ParsedCompletionTrigger::DocumentIndex { query, replacement } => {
            ResolvedCompletionTrigger::DocumentIndices {
                catalog,
                query,
                replacement,
            }
        }
        ParsedCompletionTrigger::PathStart {
            location,
            replacement,
        } => {
            let Some(document) = catalog.document(location.document_index) else {
                return ResolvedCompletionTrigger::NoCandidates {
                    reason: NoCandidateReason::DocumentNotFound,
                    replacement,
                };
            };
            if document.children(&location.parent_path).is_empty() {
                ResolvedCompletionTrigger::NoCandidates {
                    reason: NoCandidateReason::ParentHasNoChildren,
                    replacement,
                }
            } else {
                ResolvedCompletionTrigger::Delimiters {
                    document,
                    location,
                    replacement,
                }
            }
        }
        ParsedCompletionTrigger::SegmentStart {
            location,
            delimiter,
            replacement,
        } => resolve_segment_trigger(
            catalog,
            location,
            delimiter,
            SegmentSearch::All,
            replacement,
        ),
        ParsedCompletionTrigger::SegmentQuery {
            location,
            delimiter,
            query,
            replacement,
            position,
        } => resolve_segment_trigger(
            catalog,
            location,
            delimiter,
            SegmentSearch::Query {
                text: query,
                position,
            },
            replacement,
        ),
    }
}

fn resolve_segment_trigger(
    catalog: &Catalog,
    location: PathLocation,
    delimiter: PathDelimiter,
    search: SegmentSearch,
    replacement: Range<usize>,
) -> ResolvedCompletionTrigger<'_> {
    let Some(document) = catalog.document(location.document_index) else {
        return ResolvedCompletionTrigger::NoCandidates {
            reason: NoCandidateReason::DocumentNotFound,
            replacement,
        };
    };
    let entries = direct_entries(document, &location.parent_path);
    if entries.is_empty() {
        return ResolvedCompletionTrigger::NoCandidates {
            reason: NoCandidateReason::ParentHasNoChildren,
            replacement,
        };
    }

    if let SegmentSearch::Query {
        text: query,
        position: SegmentQueryPosition::AtSegmentEnd,
    } = &search
        && let Some(entry) = entries.iter().find(|entry| {
            entry
                .segment()
                .and_then(|segment| delimiter.strip_from_segment(segment))
                == Some(query.as_str())
        })
    {
        let exact_location = PathLocation {
            document_index: location.document_index,
            parent_path: entry.path().to_owned(),
        };
        let insertion = replacement.end..replacement.end;
        return if document.children(&exact_location.parent_path).is_empty() {
            ResolvedCompletionTrigger::NoCandidates {
                reason: NoCandidateReason::ExactEntryHasNoChildren,
                replacement: insertion,
            }
        } else {
            ResolvedCompletionTrigger::Delimiters {
                document,
                location: exact_location,
                replacement: insertion,
            }
        };
    }

    ResolvedCompletionTrigger::Segments {
        document,
        location,
        delimiter,
        search,
        replacement,
    }
}

fn direct_entries<'document>(
    document: &'document CatalogDocument,
    parent_path: &str,
) -> Vec<&'document CatalogEntry> {
    document
        .children(parent_path)
        .iter()
        .filter_map(|index| document.entry(*index))
        .collect()
}

fn complete_document_index(
    catalog: &Catalog,
    query: &str,
    replacement: Range<usize>,
) -> CompletionRequest {
    let candidates = catalog
        .documents()
        .iter()
        .map(|document| document.document_index().to_string())
        .filter(|candidate| candidate.starts_with(query))
        .take(MAX_COMPLETION_CANDIDATES)
        .map(|display| CompletionCandidate {
            replacement: format!("{display} "),
            display,
        })
        .collect();

    CompletionRequest {
        replacement,
        candidates,
    }
}

fn complete_resolved_trigger(
    matcher: &mut Matcher,
    trigger: ResolvedCompletionTrigger<'_>,
) -> CompletionRequest {
    match trigger {
        ResolvedCompletionTrigger::DocumentIndices {
            catalog,
            query,
            replacement,
        } => complete_document_index(catalog, &query, replacement),
        ResolvedCompletionTrigger::Delimiters {
            document,
            location,
            replacement,
        } => complete_delimiters(
            direct_entries(document, &location.parent_path)
                .into_iter()
                .filter_map(|entry| entry.segment()),
            replacement,
        ),
        ResolvedCompletionTrigger::Segments {
            document,
            location,
            delimiter,
            search,
            replacement,
        } => complete_segments(
            matcher,
            document,
            &location.parent_path,
            delimiter,
            search,
            replacement,
        ),
        ResolvedCompletionTrigger::NoCandidates {
            reason,
            replacement,
        } => {
            let _reason = reason;
            CompletionRequest {
                replacement,
                candidates: Vec::new(),
            }
        }
    }
}

/// One matching candidate split into what the user sees and what is replaced.
///
/// For example, after the user has already typed `.` in `@0 .`, the
/// `.users` candidate is displayed in full while only `users` is inserted.
#[derive(Clone, Copy)]
struct SegmentCandidate<'segment> {
    display: &'segment str,
    replacement: &'segment str,
}

impl AsRef<str> for SegmentCandidate<'_> {
    fn as_ref(&self) -> &str {
        self.replacement
    }
}

fn complete_segments(
    matcher: &mut Matcher,
    document: &CatalogDocument,
    parent_path: &str,
    delimiter: PathDelimiter,
    search: SegmentSearch,
    replacement: Range<usize>,
) -> CompletionRequest {
    let entries = direct_entries(document, parent_path);

    let mut seen = HashSet::new();
    let candidates = entries
        .into_iter()
        .filter_map(|entry| entry.segment())
        .filter_map(|segment| {
            delimiter
                .strip_from_segment(segment)
                .map(|replacement| SegmentCandidate {
                    display: segment,
                    replacement,
                })
        })
        .filter(|candidate| seen.insert(candidate.replacement))
        .collect::<Vec<_>>();
    let candidates: Vec<_> = match search {
        SegmentSearch::All => candidates
            .into_iter()
            .take(MAX_COMPLETION_CANDIDATES)
            .collect(),
        SegmentSearch::Query { text, .. } => Atom::new(
            &text,
            CaseMatching::Smart,
            Normalization::Smart,
            AtomKind::Fuzzy,
            false,
        )
        .match_list(candidates, matcher)
        .into_iter()
        .map(|(candidate, _score)| candidate)
        .take(MAX_COMPLETION_CANDIDATES)
        .collect(),
    };
    let candidates = candidates
        .into_iter()
        .map(|candidate| CompletionCandidate {
            display: candidate.display.to_owned(),
            replacement: candidate.replacement.to_owned(),
        })
        .collect();

    CompletionRequest {
        replacement,
        candidates,
    }
}

fn complete_delimiters<'a>(
    segments: impl IntoIterator<Item = &'a str>,
    replacement: Range<usize>,
) -> CompletionRequest {
    let mut has_dot = false;
    let mut has_bracket = false;
    for segment in segments {
        has_dot |= segment.starts_with('.');
        has_bracket |= segment.starts_with('[');
        if has_dot && has_bracket {
            break;
        }
    }

    let candidates = [('.', has_dot), ('[', has_bracket)]
        .into_iter()
        .filter(|(_, present)| *present)
        .map(|(delimiter, _)| {
            let delimiter = delimiter.to_string();
            CompletionCandidate {
                display: delimiter.clone(),
                replacement: delimiter,
            }
        })
        .collect();

    CompletionRequest {
        replacement,
        candidates,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{catalog::Indexer, document_source::DocumentSource, input_format::InputFormat};

    use super::*;

    fn completer_from(input: impl Into<Vec<u8>>) -> PathCompleter {
        let source =
            DocumentSource::from_reader(InputFormat::Json, Box::new(Cursor::new(input.into())))
                .unwrap();
        PathCompleter::from_catalog(Indexer::new(source).index().unwrap())
    }

    fn completer() -> PathCompleter {
        completer_from(
            concat!(
                r#"{"account":{},"users":[{"first_name":"Alice","family_name":"A","first.name":true}]}"#,
                "\n",
                r#"{"second":true}"#,
                "\n",
                r#"{}"#,
                r#"{}"#,
                r#"{}"#,
                r#"{}"#,
                r#"{}"#,
                r#"{}"#,
                r#"{}"#,
                r#"{}"#,
                r#"{}"#,
            )
            .as_bytes()
            .to_vec(),
        )
    }

    fn ready(state: CompletionState) -> CompletionRequest {
        let CompletionState::Ready(request) = state else {
            panic!("completion must be ready");
        };
        request
    }

    #[test]
    fn parses_document_index_trigger() {
        assert_eq!(
            parse_completion_trigger("@1", 2),
            Some(ParsedCompletionTrigger::DocumentIndex {
                query: "@1".to_owned(),
                replacement: 0..2,
            })
        );
    }

    #[test]
    fn parses_path_start_trigger() {
        assert_eq!(
            parse_completion_trigger("@0 ", 3),
            Some(ParsedCompletionTrigger::PathStart {
                location: PathLocation {
                    document_index: DocumentIndex(0),
                    parent_path: ".".to_owned(),
                },
                replacement: 3..3,
            })
        );
    }

    #[test]
    fn parses_segment_start_trigger() {
        assert_eq!(
            parse_completion_trigger("@0 .", 4),
            Some(ParsedCompletionTrigger::SegmentStart {
                location: PathLocation {
                    document_index: DocumentIndex(0),
                    parent_path: ".".to_owned(),
                },
                delimiter: PathDelimiter::Dot,
                replacement: 4..4,
            })
        );
    }

    #[test]
    fn parses_segment_query_trigger_without_its_delimiter() {
        let input = "@0 .users[0].na";

        assert_eq!(
            parse_completion_trigger(input, input.chars().count()),
            Some(ParsedCompletionTrigger::SegmentQuery {
                location: PathLocation {
                    document_index: DocumentIndex(0),
                    parent_path: ".users[0]".to_owned(),
                },
                delimiter: PathDelimiter::Dot,
                query: "na".to_owned(),
                replacement: 13..15,
                position: SegmentQueryPosition::AtSegmentEnd,
            })
        );
    }

    #[test]
    fn completes_document_indices_by_prefix_in_input_order() {
        let request = ready(completer().complete("@1", 2));

        assert_eq!(request.replacement, 0..2);
        assert_eq!(
            request
                .candidates
                .iter()
                .map(|candidate| candidate.display.as_str())
                .collect::<Vec<_>>(),
            ["@1", "@10"]
        );
        assert_eq!(request.candidates[0].replacement, "@1 ");
    }

    #[test]
    fn completes_only_the_current_path_segment_fuzzily() {
        let input = "@0 .users[0].fn";
        let request = ready(completer().complete(input, input.chars().count()));

        assert_eq!(request.replacement, 13..15);
        assert_eq!(
            request
                .candidates
                .iter()
                .map(|candidate| candidate.display.as_str())
                .collect::<Vec<_>>(),
            [".first_name", ".family_name"]
        );
        assert_eq!(request.candidates[0].replacement, "first_name");
    }

    #[test]
    fn offers_delimiters_before_direct_children() {
        let delimiter = ready(completer().complete("@0 ", 3));
        assert_eq!(
            delimiter
                .candidates
                .iter()
                .map(|candidate| candidate.display.as_str())
                .collect::<Vec<_>>(),
            ["."]
        );

        let request = ready(completer().complete("@0 .", 4));

        assert_eq!(
            request
                .candidates
                .iter()
                .map(|candidate| candidate.display.as_str())
                .collect::<Vec<_>>(),
            [".account", ".users"]
        );
    }

    #[test]
    fn offers_dot_and_bracket_delimiters_in_stable_order() {
        let mut completer = completer_from(br#"{"plain":1,"first name":2}"#.to_vec());

        let request = ready(completer.complete("@0 ", 3));
        assert_eq!(
            request
                .candidates
                .iter()
                .map(|candidate| candidate.display.as_str())
                .collect::<Vec<_>>(),
            [".", "["]
        );

        let request = ready(completer.complete("@0 [", 4));
        assert_eq!(
            request
                .candidates
                .iter()
                .map(|candidate| candidate.display.as_str())
                .collect::<Vec<_>>(),
            [r#"["first name"]"#]
        );
        assert_eq!(request.candidates[0].replacement, r#""first name"]"#);
    }

    #[test]
    fn offers_the_next_delimiter_after_an_exact_container() {
        let input = "@0 .users";
        let request = ready(completer().complete(input, input.chars().count()));
        let position = input.chars().count();

        assert_eq!(request.replacement, position..position);
        assert_eq!(request.candidates[0].replacement, "[");

        let input = "@0 .users[";
        let request = ready(completer().complete(input, input.chars().count()));
        assert_eq!(
            request
                .candidates
                .iter()
                .map(|candidate| candidate.display.as_str())
                .collect::<Vec<_>>(),
            ["[0]"]
        );
    }

    #[test]
    fn returns_no_candidates_after_an_exact_leaf() {
        let input = "@0 .users[0].first_name";
        let request = ready(completer().complete(input, input.chars().count()));

        assert!(request.candidates.is_empty());
    }

    #[test]
    fn deduplicates_candidates_while_preserving_input_order() {
        let mut completer = completer_from(br#"{"name":1,"name":2,"next":3}"#.to_vec());
        let request = ready(completer.complete("@0 .", 4));

        assert_eq!(
            request
                .candidates
                .iter()
                .map(|candidate| candidate.display.as_str())
                .collect::<Vec<_>>(),
            [".name", ".next"]
        );
    }

    #[test]
    fn limits_large_completion_lists() {
        let input = format!(
            "[{}]",
            (0..150)
                .map(|index| index.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut completer = completer_from(input.into_bytes());

        let request = ready(completer.complete("@0 [", 4));
        assert_eq!(request.candidates.len(), MAX_COMPLETION_CANDIDATES);
        assert_eq!(request.candidates[0].display, "[0]");
        assert_eq!(request.candidates[99].display, "[99]");
    }

    #[test]
    fn recognizes_dots_inside_quoted_bracket_segments() {
        let input = r#"@0 .users[0]["first.n"#;
        let request = ready(completer().complete(input, input.chars().count()));

        assert_eq!(
            request
                .candidates
                .iter()
                .map(|candidate| candidate.display.as_str())
                .collect::<Vec<_>>(),
            [r#"["first.name"]"#]
        );
        assert_eq!(request.candidates[0].replacement, r#""first.name"]"#);
    }

    #[test]
    fn ignores_invalid_document_inputs() {
        let mut completer = completer();

        assert_eq!(
            completer.complete("invalid .users", 14),
            CompletionState::None
        );
    }

    #[test]
    fn reports_catalog_loading_and_failure() {
        assert_eq!(
            PathCompleter::from_state(CatalogLoadState::Loading).complete("@", 1),
            CompletionState::Loading
        );

        let error: Arc<str> = "failed to index".into();
        assert_eq!(
            PathCompleter::from_state(CatalogLoadState::Failed(Arc::clone(&error)))
                .complete("@", 1),
            CompletionState::Failed(error)
        );
    }
}
