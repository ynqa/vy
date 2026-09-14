use std::ops::Range;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use tokio::sync::mpsc;

use crate::catalog::FlattenData;

const OUTPUT_BUFFER_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QueryId(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Completion {
    Completed,
    Cancelled,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Finished {
        query_id: QueryId,
        completion: Completion,
        matches: Vec<LineMatch>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LineMatch {
    pub(crate) line_index: usize,
    pub(crate) ranges: Vec<Range<usize>>,
}

impl Event {
    fn query_id(&self) -> QueryId {
        match self {
            Self::Finished { query_id, .. } => *query_id,
        }
    }
}

struct ActiveQuery {
    id: QueryId,
    cancelled: Arc<AtomicBool>,
}

/// Runs ripgrep matchers on Tokio's blocking thread pool and returns matches in one event.
pub(crate) struct GrepExecutor {
    next_query_id: u64,
    active: Option<ActiveQuery>,
    event_tx: mpsc::Sender<Event>,
    event_rx: mpsc::Receiver<Event>,
}

impl Default for GrepExecutor {
    fn default() -> Self {
        let (event_tx, event_rx) = mpsc::channel(OUTPUT_BUFFER_CAPACITY);
        Self {
            next_query_id: 0,
            active: None,
            event_tx,
            event_rx,
        }
    }
}

impl GrepExecutor {
    /// Starts a query and cooperatively cancels the previous invocation.
    pub(crate) fn execute(&mut self, query: impl Into<String>, data: Arc<FlattenData>) -> QueryId {
        self.cancel();

        self.next_query_id = self.next_query_id.wrapping_add(1);
        let id = QueryId(self.next_query_id);
        let cancelled = Arc::new(AtomicBool::new(false));
        self.active = Some(ActiveQuery {
            id,
            cancelled: Arc::clone(&cancelled),
        });

        let event_tx = self.event_tx.clone();
        let query = query.into();
        tokio::task::spawn_blocking(move || {
            let (completion, matches) = run_query(&query, &data, &cancelled);
            let _ = event_tx.blocking_send(Event::Finished {
                query_id: id,
                completion,
                matches,
            });
        });

        id
    }

    pub(crate) fn cancel(&self) -> bool {
        let Some(active) = &self.active else {
            return false;
        };
        active.cancelled.store(true, Ordering::Release);
        true
    }

    #[allow(
        dead_code,
        reason = "the UI batches buffered events with try_recv, while tests await recv"
    )]
    pub(crate) async fn recv(&mut self) -> Option<Event> {
        while let Some(event) = self.event_rx.recv().await {
            if let Some(event) = self.accept_active_event(event) {
                return Some(event);
            }
        }
        None
    }

    pub(crate) fn try_recv(&mut self) -> Option<Event> {
        while let Ok(event) = self.event_rx.try_recv() {
            if let Some(event) = self.accept_active_event(event) {
                return Some(event);
            }
        }
        None
    }

    fn accept_active_event(&mut self, mut event: Event) -> Option<Event> {
        let active = self.active.as_ref()?;
        if event.query_id() != active.id {
            return None;
        }
        if active.cancelled.load(Ordering::Acquire) {
            let Event::Finished {
                completion,
                matches,
                ..
            } = &mut event;
            *completion = Completion::Cancelled;
            matches.clear();
        }
        self.active = None;
        Some(event)
    }
}

impl Drop for GrepExecutor {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn run_query(
    query: &str,
    data: &FlattenData,
    cancelled: &AtomicBool,
) -> (Completion, Vec<LineMatch>) {
    if cancelled.load(Ordering::Acquire) {
        return (Completion::Cancelled, Vec::new());
    }
    let matchers = match compile_query(query) {
        Ok(matchers) => matchers,
        Err(error) => return (Completion::Failed(error), Vec::new()),
    };
    let mut matches = Vec::new();

    for (line_index, line) in data.lines().iter().enumerate() {
        if cancelled.load(Ordering::Acquire) {
            return (Completion::Cancelled, Vec::new());
        }
        if let Some(ranges) = match_line(&matchers, line) {
            matches.push(LineMatch { line_index, ranges });
        }
    }

    if cancelled.load(Ordering::Acquire) {
        (Completion::Cancelled, Vec::new())
    } else {
        (Completion::Completed, matches)
    }
}

fn compile_query(query: &str) -> Result<Vec<Vec<RegexMatcher>>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("grep requires a query".to_owned());
    }

    split_top_level(query, '|')
        .into_iter()
        .map(|clause| {
            let clause = clause.trim();
            if clause.is_empty() {
                return Err("grep contains an empty OR clause".to_owned());
            }
            split_top_level(clause, '&')
                .into_iter()
                .map(|pattern| {
                    let pattern = pattern.trim();
                    if pattern.is_empty() {
                        return Err("grep contains an empty AND term".to_owned());
                    }
                    RegexMatcher::new_line_matcher(pattern).map_err(|error| error.to_string())
                })
                .collect()
        })
        .collect()
}

fn match_line(clauses: &[Vec<RegexMatcher>], line: &str) -> Option<Vec<Range<usize>>> {
    let mut ranges = Vec::new();
    let mut matched = false;

    for clause in clauses {
        let mut clause_ranges = Vec::new();
        let clause_matches = clause.iter().all(|matcher| {
            let mut term_matched = false;
            matcher
                .find_iter(line.as_bytes(), |found| {
                    term_matched = true;
                    if !found.is_empty() {
                        clause_ranges.push(found.start()..found.end());
                    }
                    true
                })
                .expect("ripgrep RegexMatcher is infallible after compilation");
            term_matched
        });
        if clause_matches {
            matched = true;
            ranges.append(&mut clause_ranges);
        }
    }

    matched.then(|| merge_ranges(ranges))
}

fn merge_ranges(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.sort_unstable_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges {
        if let Some(previous) = merged.last_mut()
            && range.start <= previous.end
        {
            previous.end = previous.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}

/// Splits boolean operators only outside escaped text, character classes, and groups.
fn split_top_level(input: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut escaped = false;
    let mut in_class = false;
    let mut group_depth = 0usize;

    for (index, character) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '(' if !in_class => group_depth = group_depth.saturating_add(1),
            ')' if !in_class => group_depth = group_depth.saturating_sub(1),
            character if character == delimiter && !in_class && group_depth == 0 => {
                parts.push(&input[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;

    fn data(lines: &[&str]) -> Arc<FlattenData> {
        Arc::new(FlattenData::from_lines_for_test(lines.iter().copied()))
    }

    async fn collect(executor: &mut GrepExecutor) -> (Vec<LineMatch>, Completion) {
        timeout(Duration::from_secs(2), async {
            match executor.recv().await.expect("grep event channel closed") {
                Event::Finished {
                    completion,
                    matches,
                    ..
                } => (matches, completion),
            }
        })
        .await
        .expect("grep event timeout")
    }

    #[tokio::test]
    async fn searches_with_ripgrep_regexes_and_boolean_operators() {
        let input = data(&[
            "@0.name = \"Alice\"",
            "@0.role = \"admin\"",
            "@1.name = \"Bob\"",
        ]);
        let mut executor = GrepExecutor::default();

        executor.execute(r"^@0&Al.ce|^@1&B.b", input);
        let (matches, completion) = collect(&mut executor).await;

        assert_eq!(
            matches,
            [
                LineMatch {
                    line_index: 0,
                    ranges: vec![0..2, 11..16],
                },
                LineMatch {
                    line_index: 2,
                    ranges: vec![0..2, 11..14],
                },
            ]
        );
        assert_eq!(completion, Completion::Completed);
    }

    #[test]
    fn preserves_regex_operators_inside_groups_and_classes() {
        assert_eq!(
            split_top_level("foo(bar|baz)&[a&b]", '|'),
            ["foo(bar|baz)&[a&b]"]
        );
        assert_eq!(
            split_top_level("foo(bar|baz)&[a&b]", '&'),
            ["foo(bar|baz)", "[a&b]"]
        );
    }

    #[tokio::test]
    async fn rejects_invalid_regexes() {
        let mut executor = GrepExecutor::default();
        executor.execute("[", data(&["anything"]));

        let (_, completion) = collect(&mut executor).await;

        assert!(matches!(completion, Completion::Failed(_)));
    }

    #[tokio::test]
    async fn a_new_query_supersedes_the_previous_query() {
        let mut executor = GrepExecutor::default();
        let input = data(&["old", "new"]);
        executor.execute("old", Arc::clone(&input));
        executor.execute("new", input);

        let (matches, completion) = collect(&mut executor).await;

        assert_eq!(
            matches,
            [LineMatch {
                line_index: 1,
                ranges: std::iter::once(0..3).collect(),
            }]
        );
        assert_eq!(completion, Completion::Completed);
    }

    #[test]
    fn highlights_all_matches_from_satisfied_boolean_clauses() {
        let clauses = compile_query(r"foo&o+|bar").unwrap();

        assert_eq!(
            match_line(&clauses, "foo food bar"),
            Some(vec![0..3, 4..7, 9..12])
        );
        assert_eq!(
            match_line(&clauses, "foo"),
            Some(std::iter::once(0..3).collect())
        );
        assert_eq!(match_line(&clauses, "neither"), None);

        let clauses = compile_query("名前|太.").unwrap();
        assert_eq!(match_line(&clauses, "名前 = 太郎"), Some(vec![0..6, 9..15]));
    }
}
