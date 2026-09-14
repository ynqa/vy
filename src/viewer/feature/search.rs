use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use promkit::core::{
    CreatedGraphemes, Widget,
    crossterm::{event::Event, style::ContentStyle},
};

use crate::{
    catalog::{FlattenData, FlattenLoadState, FlattenSubscription},
    history::History,
    viewer::{TextEditorAction, ui::MultiLineEditorComponent},
};

#[derive(Debug, PartialEq, Eq)]
pub(in crate::viewer) enum SearchOutcome {
    Editing,
    Cancelled,
    Moved { document_index: usize, path: String },
    NotFound,
    Failed,
}

pub(in crate::viewer) struct SearchComponent {
    source: FlattenSubscription,
    editor: Option<MultiLineEditorComponent>,
    query: String,
    history: History,
    history_entries: Vec<String>,
    history_position: Option<usize>,
    history_draft: String,
    matches: Vec<usize>,
    selected: Option<usize>,
    error: Option<String>,
    active_char_style: ContentStyle,
}

impl SearchComponent {
    pub(in crate::viewer) fn new(
        source: FlattenSubscription,
        active_char_style: ContentStyle,
        history: History,
    ) -> Self {
        Self {
            source,
            editor: None,
            query: String::new(),
            history,
            history_entries: Vec::new(),
            history_position: None,
            history_draft: String::new(),
            matches: Vec::new(),
            selected: None,
            error: None,
            active_char_style,
        }
    }

    pub(in crate::viewer) fn enter(&mut self) {
        self.error = None;
        self.history_entries = self.history.newest_searches();
        self.history_position = None;
        self.history_draft.clear();
        self.editor = Some(MultiLineEditorComponent::new(
            "/",
            "",
            self.active_char_style,
        ));
    }

    pub(in crate::viewer) fn cancel(&mut self) -> SearchOutcome {
        self.editor = None;
        self.error = None;
        SearchOutcome::Cancelled
    }

    pub(in crate::viewer) fn submit(&mut self, current: Option<(usize, &str)>) -> SearchOutcome {
        let Some(editor) = &self.editor else {
            return SearchOutcome::Editing;
        };
        let query = editor.value().trim().to_owned();
        if query.is_empty() {
            self.error = Some("search requires a pattern".to_owned());
            return SearchOutcome::Failed;
        }
        let matcher = match RegexMatcher::new_line_matcher(&query) {
            Ok(matcher) => matcher,
            Err(error) => {
                self.error = Some(error.to_string());
                return SearchOutcome::Failed;
            }
        };
        let data = match self.source.state_and_mark_seen() {
            FlattenLoadState::Loading => {
                self.error = Some("search index is loading".to_owned());
                return SearchOutcome::Failed;
            }
            FlattenLoadState::Failed(error) => {
                self.error = Some(error.to_string());
                return SearchOutcome::Failed;
            }
            FlattenLoadState::Ready(data) => data,
        };

        self.history.record_search(&query);
        self.query = query;
        self.matches = matching_lines(&matcher, &data);
        self.error = None;
        self.editor = None;
        if self.matches.is_empty() {
            self.selected = None;
            self.error = Some(format!("pattern not found: {}", self.query));
            return SearchOutcome::NotFound;
        }

        let current_line = current.and_then(|(document_index, path)| {
            data.lines().iter().enumerate().find_map(|(line_index, _)| {
                let target = data.target(line_index)?;
                (target.document_index() == document_index && target.path() == path)
                    .then_some(line_index)
            })
        });
        let selected = current_line
            .and_then(|line| self.matches.iter().position(|matched| *matched > line))
            .unwrap_or(0);
        self.selected = Some(selected);
        outcome_for(&data, &self.matches, selected)
    }

    pub(in crate::viewer) fn replace_source(&mut self, source: FlattenSubscription) {
        *self = Self::new(source, self.active_char_style, self.history.clone());
    }

    pub(in crate::viewer) fn recall_history(&mut self, older: bool) -> SearchOutcome {
        let Some(editor) = &mut self.editor else {
            return SearchOutcome::Editing;
        };
        let position = match (self.history_position, older) {
            (None, true) if !self.history_entries.is_empty() => {
                self.history_draft = editor.value();
                Some(0)
            }
            (Some(position), true) if position + 1 < self.history_entries.len() => {
                Some(position + 1)
            }
            (Some(0), false) => None,
            (Some(position), false) => Some(position - 1),
            _ => return SearchOutcome::Editing,
        };
        self.history_position = position;
        let value = position.map_or(self.history_draft.as_str(), |index| {
            self.history_entries[index].as_str()
        });
        editor.replace(value);
        editor.move_to_tail();
        self.error = None;
        SearchOutcome::Editing
    }

    pub(in crate::viewer) fn next(&mut self) -> SearchOutcome {
        self.move_match(true)
    }

    pub(in crate::viewer) fn previous(&mut self) -> SearchOutcome {
        self.move_match(false)
    }

    fn move_match(&mut self, forward: bool) -> SearchOutcome {
        if self.matches.is_empty() {
            self.error = Some("no active search".to_owned());
            return SearchOutcome::Failed;
        }
        let selected = match (self.selected, forward) {
            (Some(selected), true) => (selected + 1) % self.matches.len(),
            (Some(0), false) | (None, false) => self.matches.len() - 1,
            (Some(selected), false) => selected - 1,
            (None, true) => 0,
        };
        self.selected = Some(selected);
        self.error = None;
        match self.source.state_and_mark_seen() {
            FlattenLoadState::Ready(data) => outcome_for(&data, &self.matches, selected),
            FlattenLoadState::Loading => {
                self.error = Some("search index is loading".to_owned());
                SearchOutcome::Failed
            }
            FlattenLoadState::Failed(error) => {
                self.error = Some(error.to_string());
                SearchOutcome::Failed
            }
        }
    }

    pub(in crate::viewer) fn handle_input(&mut self, event: &Event) -> SearchOutcome {
        let Some(editor) = &mut self.editor else {
            return SearchOutcome::Editing;
        };
        if editor.handle_input(event) {
            self.error = None;
        }
        SearchOutcome::Editing
    }

    pub(in crate::viewer) fn apply_text_editor_action(
        &mut self,
        action: TextEditorAction,
    ) -> SearchOutcome {
        if let Some(editor) = &mut self.editor {
            editor.apply_action(action);
            self.error = None;
        }
        SearchOutcome::Editing
    }

    pub(in crate::viewer) fn feedback(&self) -> Option<(String, bool)> {
        if let Some(error) = &self.error {
            return Some((error.clone(), true));
        }
        if self.editor.is_some() {
            return None;
        }
        let selected = self.selected?;
        Some((
            format!("{}/{}: {}", selected + 1, self.matches.len(), self.query),
            false,
        ))
    }

    pub(in crate::viewer) fn set_active_char_style(&mut self, style: ContentStyle) {
        self.active_char_style = style;
        if let Some(editor) = &mut self.editor {
            editor.set_active_char_style(style);
        }
    }
}

impl Widget for SearchComponent {
    fn create_graphemes(&self) -> CreatedGraphemes {
        self.editor
            .as_ref()
            .map(Widget::create_graphemes)
            .unwrap_or_default()
    }
}

fn matching_lines(matcher: &RegexMatcher, data: &FlattenData) -> Vec<usize> {
    data.lines()
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            let text = data.search_text(index)?;
            matcher
                .is_match(text.as_bytes())
                .expect("RegexMatcher is infallible after compilation")
                .then_some(index)
        })
        .collect()
}

fn outcome_for(data: &FlattenData, matches: &[usize], selected: usize) -> SearchOutcome {
    let Some(target) = matches
        .get(selected)
        .and_then(|line_index| data.target(*line_index))
    else {
        return SearchOutcome::Failed;
    };
    SearchOutcome::Moved {
        document_index: target.document_index(),
        path: target.path().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(lines: &[&str]) -> SearchComponent {
        SearchComponent::new(
            FlattenSubscription::ready_for_test(lines.iter().copied()),
            ContentStyle::default(),
            History::memory(),
        )
    }

    #[test]
    fn searches_paths_and_values_and_wraps_in_both_directions() {
        let mut search = component(&[
            "@0 = {}",
            "@0.users = []",
            "@0.users[0] = {}",
            "@0.users[0].name = \"Alice\"",
            "@0.users[1] = {}",
            "@0.users[1].name = \"Bob\"",
        ]);
        search.enter();
        search.handle_input(&Event::Paste("name".into()));

        assert_eq!(
            search.submit(Some((0, ".users[0]"))),
            SearchOutcome::Moved {
                document_index: 0,
                path: ".users[0].name".into(),
            }
        );
        assert!(
            matches!(search.next(), SearchOutcome::Moved { path, .. } if path == ".users[1].name")
        );
        assert!(
            matches!(search.next(), SearchOutcome::Moved { path, .. } if path == ".users[0].name")
        );
        assert!(
            matches!(search.previous(), SearchOutcome::Moved { path, .. } if path == ".users[1].name")
        );
    }

    #[test]
    fn keeps_invalid_regex_in_the_editor() {
        let mut search = component(&["@0.name = \"Alice\""]);
        search.enter();
        search.handle_input(&Event::Paste("[".into()));

        assert_eq!(search.submit(None), SearchOutcome::Failed);
        assert!(search.feedback().is_some_and(|(_, error)| error));
        assert_eq!(search.create_graphemes().graphemes.to_string(), "/[ ");
    }

    #[test]
    fn does_not_match_descendants_only_because_an_ancestor_key_matches() {
        let mut search = component(&[
            "@0 = {}",
            "@0[\"404\"] = []",
            "@0[\"404\"][0] = {}",
            "@0[\"404\"][0].status = 200",
            "@0.items = []",
            "@0.items[404] = \"ordinary array item\"",
            "@0.code = 404",
        ]);
        search.enter();
        search.handle_input(&Event::Paste("404".into()));

        assert_eq!(
            search.submit(Some((0, "."))),
            SearchOutcome::Moved {
                document_index: 0,
                path: "[\"404\"]".into(),
            }
        );
        assert_eq!(search.feedback(), Some(("1/2: 404".into(), false)));
        assert!(matches!(search.next(), SearchOutcome::Moved { path, .. } if path == ".code"));
    }

    #[test]
    fn recalls_both_directions_without_wrapping_and_restores_the_draft() {
        let mut search = component(&["@0.name = \"Alice\""]);
        search.history.record_search("Alice");
        search.history.record_search("name");
        search.enter();
        search.handle_input(&Event::Paste("draft".into()));
        search.recall_history(false);
        assert_eq!(search.editor.as_ref().unwrap().value(), "draft");
        search.recall_history(true);
        assert_eq!(search.editor.as_ref().unwrap().value(), "name");
        search.recall_history(true);
        search.recall_history(true);
        assert_eq!(search.editor.as_ref().unwrap().value(), "Alice");
        search.recall_history(false);
        assert_eq!(search.editor.as_ref().unwrap().value(), "name");
        search.recall_history(false);
        search.recall_history(false);
        assert_eq!(search.editor.as_ref().unwrap().value(), "draft");
        search.recall_history(true);
        search.handle_input(&Event::Paste(".*".into()));
        assert_eq!(search.editor.as_ref().unwrap().value(), "name.*");
        assert!(matches!(search.submit(None), SearchOutcome::Moved { .. }));
        assert_eq!(
            search.history.newest_searches(),
            vec!["name.*", "name", "Alice"]
        );
    }

    #[test]
    fn empty_history_and_cancel_leave_the_draft_unrecorded() {
        let mut search = component(&["@0.name = \"Alice\""]);
        search.enter();
        search.handle_input(&Event::Paste("draft".into()));
        search.recall_history(true);
        search.recall_history(false);
        assert_eq!(search.editor.as_ref().unwrap().value(), "draft");
        search.cancel();
        search.enter();
        assert_eq!(search.editor.as_ref().unwrap().value(), "");
        assert!(search.history.newest_searches().is_empty());
    }

    #[test]
    fn records_valid_searches_even_without_matches_and_keeps_history_on_source_change() {
        let mut search = component(&["@0.name = \"Alice\""]);
        search.enter();
        search.handle_input(&Event::Paste("[".into()));
        assert_eq!(search.submit(None), SearchOutcome::Failed);
        assert!(search.history.newest_searches().is_empty());
        search.editor.as_mut().unwrap().replace("missing");
        assert_eq!(search.submit(None), SearchOutcome::NotFound);
        search.replace_source(FlattenSubscription::ready_for_test(["@0.other = 1"]));
        search.enter();
        search.recall_history(true);
        assert_eq!(search.editor.as_ref().unwrap().value(), "missing");
    }
}
