use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const MAX_ENTRIES: usize = 1_000;

#[derive(Clone)]
pub(crate) struct History {
    inner: Arc<Mutex<HistoryState>>,
    searches: Arc<Mutex<HistoryState>>,
}

struct HistoryState {
    path: Option<PathBuf>,
    entries: VecDeque<String>,
    needs_compaction: bool,
}

impl History {
    pub(crate) fn load_default() -> Self {
        let path = dirs::data_local_dir().map(|path| path.join("vy").join("history.jsonl"));
        Self::load(path)
    }

    fn load(path: Option<PathBuf>) -> Self {
        let search_path = path
            .as_ref()
            .map(|path| path.with_extension("search.jsonl"));
        Self {
            inner: Arc::new(Mutex::new(Self::load_state(path))),
            searches: Arc::new(Mutex::new(Self::load_state(search_path))),
        }
    }

    fn load_state(path: Option<PathBuf>) -> HistoryState {
        let mut needs_compaction = false;
        let mut entries = path
            .as_deref()
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|content| {
                content
                    .lines()
                    .filter_map(|line| serde_json::from_str::<String>(line).ok())
                    .filter(|entry| !entry.trim().is_empty())
                    .collect::<VecDeque<_>>()
            })
            .unwrap_or_default();
        if entries.len() > MAX_ENTRIES {
            needs_compaction = true;
            entries.drain(..entries.len() - MAX_ENTRIES);
        }
        HistoryState {
            path,
            entries,
            needs_compaction,
        }
    }

    #[cfg(test)]
    pub(crate) fn memory() -> Self {
        Self::load(None)
    }

    pub(crate) fn record(&self, command: &str) {
        let command = command.trim();
        if command.is_empty() || command.split_whitespace().next() == Some("hstr") {
            return;
        }

        Self::record_into(&self.inner, command);
    }

    pub(crate) fn record_search(&self, query: &str) {
        let query = query.trim();
        if !query.is_empty() {
            Self::record_into(&self.searches, query);
        }
    }

    fn record_into(store: &Mutex<HistoryState>, command: &str) {
        let mut state = store.lock().expect("history lock poisoned");
        if state.entries.back().is_some_and(|entry| entry == command) {
            return;
        }
        state.entries.push_back(command.to_owned());
        if state.entries.len() > MAX_ENTRIES {
            state.entries.pop_front();
            state.needs_compaction = true;
        }
        let _ = persist(&mut state, command);
    }

    pub(crate) fn newest(&self) -> Vec<String> {
        Self::newest_in(&self.inner)
    }

    pub(crate) fn newest_searches(&self) -> Vec<String> {
        Self::newest_in(&self.searches)
    }

    fn newest_in(store: &Mutex<HistoryState>) -> Vec<String> {
        store
            .lock()
            .expect("history lock poisoned")
            .entries
            .iter()
            .rev()
            .cloned()
            .collect()
    }
}

fn persist(state: &mut HistoryState, latest: &str) -> anyhow::Result<()> {
    let Some(path) = state.path.as_deref() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if state.needs_compaction {
        rewrite(path, &state.entries)?;
        state.needs_compaction = false;
        return Ok(());
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(latest)?)?;
    Ok(())
}

fn rewrite(path: &Path, entries: &VecDeque<String>) -> anyhow::Result<()> {
    let content = entries
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    fs::write(path, format!("{content}\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn stores_newest_commands_and_skips_consecutive_duplicates() {
        let history = History::memory();
        history.record("goto @0 .name");
        history.record("goto @0 .name");
        history.record("jaq .items[]");

        assert_eq!(history.newest(), vec!["jaq .items[]", "goto @0 .name"]);
    }

    #[test]
    fn does_not_store_history_searches() {
        let history = History::memory();
        history.record("hstr");
        history.record("hstr jaq");
        assert!(history.newest().is_empty());
    }

    #[test]
    fn persists_history_as_json_lines() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("vy-history-{}-{nonce}.jsonl", std::process::id()));
        let history = History::load(Some(path.clone()));
        history.record("jaq .items[] | .name");
        drop(history);

        assert_eq!(
            History::load(Some(path.clone())).newest(),
            vec!["jaq .items[] | .name"]
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn persists_searches_separately_and_shares_them_between_views() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "vy-search-history-{}-{nonce}.jsonl",
            std::process::id()
        ));
        let history = History::load(Some(path.clone()));
        history.record("jaq .items[]");
        let other_view = history.clone();
        other_view.record_search("name");
        other_view.record_search("name");
        other_view.record_search("hstr");
        assert_eq!(history.newest_searches(), vec!["hstr", "name"]);
        let reloaded = History::load(Some(path.clone()));
        assert_eq!(reloaded.newest(), vec!["jaq .items[]"]);
        assert_eq!(reloaded.newest_searches(), vec!["hstr", "name"]);
        fs::remove_file(path.with_extension("search.jsonl")).unwrap();
        fs::remove_file(path).unwrap();
    }
}
