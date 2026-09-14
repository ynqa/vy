use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tokio::{sync::watch, task};

use super::{Catalog, CatalogLoadState, CatalogSubscription, CatalogValue};

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct FlattenData {
    lines: Box<[String]>,
    targets: Box<[FlattenTarget]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FlattenTarget {
    document_index: usize,
    path: Box<str>,
    search_start: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RelativeTarget {
    Parent,
    FirstChild,
    NextSibling,
    PreviousSibling,
}

impl FlattenTarget {
    pub(crate) fn document_index(&self) -> usize {
        self.document_index
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }
}

impl FlattenData {
    fn from_catalog(catalog: &Catalog, cancelled: &AtomicBool) -> Option<Self> {
        let mut lines = Vec::new();
        let mut targets = Vec::new();
        for document in catalog.documents() {
            let mut array_paths = HashMap::new();
            for entry in document.entries() {
                if cancelled.load(Ordering::Acquire) {
                    return None;
                }
                let line = document.flatten_entry(entry);
                let parent_is_array = parent_path(entry.path())
                    .and_then(|parent| array_paths.get(parent))
                    .copied()
                    .unwrap_or(false);
                let search_start = search_start(
                    document.document_index().get(),
                    entry.path(),
                    parent_is_array,
                )?;
                array_paths.insert(entry.path(), matches!(entry.value(), CatalogValue::Array));
                lines.push(line);
                targets.push(FlattenTarget {
                    document_index: document.document_index().get(),
                    path: entry.path().into(),
                    search_start,
                });
            }
        }
        Some(Self {
            lines: lines.into_boxed_slice(),
            targets: targets.into_boxed_slice(),
        })
    }

    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn target(&self, line_index: usize) -> Option<&FlattenTarget> {
        self.targets.get(line_index)
    }

    /// Returns the searchable representation of one node without ancestor path segments.
    ///
    /// Flatten lines contain complete paths, so searching them directly would make every
    /// descendant match a pattern found only in an ancestor key. Object keys and node values are
    /// retained, while array positions are excluded to match fx/jless search behavior.
    pub(crate) fn search_text(&self, line_index: usize) -> Option<&str> {
        let line = self.lines.get(line_index)?;
        let target = self.targets.get(line_index)?;
        line.get(target.search_start..)
    }

    pub(crate) fn relative_target(
        &self,
        document_index: usize,
        path: &str,
        direction: RelativeTarget,
    ) -> Option<&FlattenTarget> {
        match direction {
            RelativeTarget::Parent => {
                let parent = parent_path(path)?;
                self.find_target(document_index, parent)
            }
            RelativeTarget::FirstChild => self.targets.iter().find(|target| {
                target.document_index == document_index && parent_path(&target.path) == Some(path)
            }),
            RelativeTarget::NextSibling | RelativeTarget::PreviousSibling => {
                let parent = parent_path(path)?;
                let mut previous = None;
                for target in &self.targets {
                    if target.document_index != document_index
                        || parent_path(&target.path) != Some(parent)
                    {
                        continue;
                    }
                    if target.path() == path {
                        if direction == RelativeTarget::PreviousSibling {
                            return previous;
                        }
                        previous = Some(target);
                        continue;
                    }
                    if direction == RelativeTarget::NextSibling
                        && previous.is_some_and(|target| target.path() == path)
                    {
                        return Some(target);
                    }
                    previous = Some(target);
                }
                None
            }
        }
    }

    fn find_target(&self, document_index: usize, path: &str) -> Option<&FlattenTarget> {
        self.targets
            .iter()
            .find(|target| target.document_index == document_index && target.path.as_ref() == path)
    }

    #[cfg(test)]
    pub(crate) fn from_lines_for_test(lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let lines = lines.into_iter().map(Into::into).collect::<Vec<String>>();
        let mut array_paths = HashMap::new();
        let targets = lines
            .iter()
            .map(|line| {
                let mut target = target_from_flatten_line(line).unwrap_or_else(|| FlattenTarget {
                    document_index: 0,
                    path: ".".into(),
                    search_start: 0,
                });
                let parent_is_array = parent_path(target.path())
                    .and_then(|parent| array_paths.get(parent))
                    .copied()
                    .unwrap_or(false);
                target.search_start =
                    search_start(target.document_index, target.path(), parent_is_array)
                        .unwrap_or(0);
                let value = line.get(value_start(target.document_index, target.path())..);
                array_paths.insert(target.path.clone(), value == Some("[]"));
                target
            })
            .collect::<Vec<_>>();
        Self {
            lines: lines.into_boxed_slice(),
            targets: targets.into_boxed_slice(),
        }
    }
}

fn parent_path(path: &str) -> Option<&str> {
    if path == "." {
        return None;
    }
    let start = last_segment_start(path)?;
    Some(if start == 0 { "." } else { &path[..start] })
}

fn last_segment_start(path: &str) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    let mut last_segment = None;
    for (index, character) in path.char_indices() {
        if escaped {
            escaped = false;
        } else if quoted && character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if !quoted && matches!(character, '.' | '[') {
            last_segment = Some(index);
        }
    }
    last_segment
}

fn decimal_digits(value: usize) -> usize {
    value.checked_ilog10().unwrap_or(0) as usize + 1
}

fn search_start(document_index: usize, path: &str, parent_is_array: bool) -> Option<usize> {
    if path == "." || parent_is_array {
        Some(value_start(document_index, path))
    } else {
        Some(1 + decimal_digits(document_index) + last_segment_start(path)?)
    }
}

fn value_start(document_index: usize, path: &str) -> usize {
    let path_len = if path == "." { 0 } else { path.len() };
    1 + decimal_digits(document_index) + path_len + " = ".len()
}

#[cfg(test)]
fn target_from_flatten_line(line: &str) -> Option<FlattenTarget> {
    let key = line.split_once(" = ").map(|(key, _)| key)?;
    let key = key.strip_prefix('@')?;
    let index_end = key
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(key.len());
    let document_index = key[..index_end].parse().ok()?;
    let path = match &key[index_end..] {
        "" => ".",
        path => path,
    };
    Some(FlattenTarget {
        document_index,
        path: path.into(),
        search_start: 0,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FlattenLoadState {
    Loading,
    Ready(Arc<FlattenData>),
    Failed(Arc<str>),
}

#[derive(Clone)]
pub(crate) struct FlattenSubscription {
    state: watch::Receiver<FlattenLoadState>,
}

impl FlattenSubscription {
    pub(crate) fn state(&self) -> FlattenLoadState {
        self.state.borrow().clone()
    }

    pub(crate) fn state_and_mark_seen(&mut self) -> FlattenLoadState {
        self.state.borrow_and_update().clone()
    }

    pub(crate) fn has_changed(&self) -> bool {
        self.state.has_changed().unwrap_or(false)
    }

    #[cfg(test)]
    pub(crate) async fn changed(&mut self) -> Option<FlattenLoadState> {
        self.state.changed().await.ok()?;
        Some(self.state_and_mark_seen())
    }

    #[cfg(test)]
    pub(crate) fn from_state(state: FlattenLoadState) -> Self {
        let (_sender, state) = watch::channel(state);
        Self { state }
    }

    #[cfg(test)]
    pub(crate) fn ready_for_test(lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::from_state(FlattenLoadState::Ready(Arc::new(
            FlattenData::from_lines_for_test(lines),
        )))
    }

    #[cfg(test)]
    pub(crate) fn channel_for_test(
        state: FlattenLoadState,
    ) -> (watch::Sender<FlattenLoadState>, Self) {
        let (sender, state) = watch::channel(state);
        (sender, Self { state })
    }
}

pub(crate) struct FlattenLoader;

impl FlattenLoader {
    pub(crate) fn spawn(mut catalog: CatalogSubscription) -> FlattenSubscription {
        let (state_tx, state) = watch::channel(FlattenLoadState::Loading);

        tokio::spawn(async move {
            let catalog = loop {
                match catalog.state_and_mark_seen() {
                    CatalogLoadState::Loading => {
                        tokio::select! {
                            changed = catalog.changed() => {
                                if changed.is_none() {
                                    state_tx.send_replace(FlattenLoadState::Failed(
                                        "catalog preparation stopped unexpectedly".into(),
                                    ));
                                    return;
                                }
                            }
                            _ = state_tx.closed() => return,
                        }
                    }
                    CatalogLoadState::Ready(catalog) => break catalog,
                    CatalogLoadState::Failed(error) => {
                        state_tx.send_replace(FlattenLoadState::Failed(error));
                        return;
                    }
                }
            };

            let cancelled = Arc::new(AtomicBool::new(false));
            let task_cancelled = Arc::clone(&cancelled);
            let build =
                task::spawn_blocking(move || FlattenData::from_catalog(&catalog, &task_cancelled));
            tokio::pin!(build);

            tokio::select! {
                result = &mut build => {
                    let next_state = match result {
                        Ok(Some(data)) => FlattenLoadState::Ready(Arc::new(data)),
                        Ok(None) => return,
                        Err(error) => FlattenLoadState::Failed(error.to_string().into()),
                    };
                    state_tx.send_replace(next_state);
                }
                _ = state_tx.closed() => {
                    cancelled.store(true, Ordering::Release);
                    let _ = build.await;
                }
            }
        });

        FlattenSubscription { state }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{
        catalog::CatalogLoader, document_source::DocumentSource, input_format::InputFormat,
    };

    use super::*;

    fn source(input: &str) -> DocumentSource {
        DocumentSource::from_reader(
            InputFormat::Json,
            Box::new(Cursor::new(input.as_bytes().to_vec())),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn prepares_flatten_data_after_the_shared_catalog() {
        let mut subscription = FlattenLoader::spawn(CatalogLoader::spawn(source(
            r#"{"users":[{"name":"Alice"}]}"#,
        )));

        assert_eq!(subscription.state(), FlattenLoadState::Loading);
        let state = loop {
            let state = subscription.changed().await.unwrap();
            if !matches!(state, FlattenLoadState::Loading) {
                break state;
            }
        };
        let FlattenLoadState::Ready(data) = state else {
            panic!("flatten data must become ready");
        };
        assert_eq!(
            data.lines(),
            [
                "@0 = {}",
                "@0.users = []",
                "@0.users[0] = {}",
                "@0.users[0].name = \"Alice\"",
            ]
        );
        assert_eq!(data.target(0).unwrap().document_index(), 0);
        assert_eq!(data.target(0).unwrap().path(), ".");
        assert_eq!(data.target(3).unwrap().path(), ".users[0].name");
    }

    #[tokio::test]
    async fn forwards_catalog_failures() {
        let mut subscription = FlattenLoader::spawn(CatalogLoader::spawn(source("{")));

        let state = loop {
            let state = subscription.changed().await.unwrap();
            if !matches!(state, FlattenLoadState::Loading) {
                break state;
            }
        };
        assert!(matches!(state, FlattenLoadState::Failed(_)));
    }

    #[test]
    fn resolves_parent_child_and_sibling_targets() {
        let data = FlattenData::from_lines_for_test([
            "@0 = {}",
            "@0.users = []",
            "@0.users[0] = {}",
            "@0.users[0].name = \"Alice\"",
            "@0.users[1] = {}",
            "@0.users[1].name = \"Bob\"",
        ]);

        assert_eq!(
            data.relative_target(0, ".users", RelativeTarget::Parent)
                .map(FlattenTarget::path),
            Some(".")
        );
        assert_eq!(
            data.relative_target(0, ".users", RelativeTarget::FirstChild)
                .map(FlattenTarget::path),
            Some(".users[0]")
        );
        assert_eq!(
            data.relative_target(0, ".users[0]", RelativeTarget::NextSibling)
                .map(FlattenTarget::path),
            Some(".users[1]")
        );
        assert_eq!(
            data.relative_target(0, ".users[1]", RelativeTarget::PreviousSibling)
                .map(FlattenTarget::path),
            Some(".users[0]")
        );
        assert!(
            data.relative_target(0, ".users[1]", RelativeTarget::NextSibling)
                .is_none()
        );
    }

    #[test]
    fn finds_parents_of_quoted_bracket_paths() {
        assert_eq!(
            parent_path(r#"["first.name"][0]"#),
            Some(r#"["first.name"]"#)
        );
        assert_eq!(parent_path(r#"["first.name"]"#), Some("."));
        assert_eq!(parent_path("."), None);
    }

    #[test]
    fn search_text_omits_ancestor_paths() {
        let data = FlattenData::from_lines_for_test([
            "@12 = {}",
            "@12[\"404\"] = []",
            "@12[\"404\"][0] = {}",
            "@12[\"404\"][0].status = 404",
        ]);

        assert_eq!(data.search_text(0), Some("{}"));
        assert_eq!(data.search_text(1), Some("[\"404\"] = []"));
        assert_eq!(data.search_text(2), Some("{}"));
        assert_eq!(data.search_text(3), Some(".status = 404"));
    }
}
