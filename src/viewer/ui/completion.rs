use promkit::core::{
    ContentPosition, CreatedGraphemes, Widget, WidgetLayout, WidthMode, grapheme::StyledGraphemes,
};

use crate::command::CompletionCandidate;

const DEFAULT_VISIBLE_LINES: usize = 8;

/// The number of candidates relevant to one Tab action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum CompletionCardinality {
    /// No candidate can be completed; for example, `goto @0 .name` when
    /// `.name` is an exact leaf.
    Empty,

    /// Exactly one candidate can be inserted immediately; for example, only
    /// `[` is available after `goto @0 .users`.
    Unique,

    /// Multiple candidates require selection; for example, both `.account`
    /// and `.users` are available after `goto @0 .`.
    Multiple,
}

/// Owns selection and vertical rendering independently of a completer's data
/// source and replacement rules.
pub(in crate::viewer) struct CompletionComponent {
    candidates: Vec<CompletionCandidate>,
    selected: Option<usize>,
    active: bool,
    visible_lines: usize,
}

impl Default for CompletionComponent {
    fn default() -> Self {
        Self {
            candidates: Vec::new(),
            selected: None,
            active: false,
            visible_lines: DEFAULT_VISIBLE_LINES,
        }
    }
}

impl CompletionComponent {
    pub(in crate::viewer) fn set_candidates(
        &mut self,
        candidates: impl IntoIterator<Item = CompletionCandidate>,
    ) {
        self.candidates = candidates.into_iter().collect();
        self.selected = None;
        self.active = false;
    }

    fn activate(&mut self) -> bool {
        if self.candidates.is_empty() {
            return false;
        }
        self.active = true;
        self.selected = Some(0);
        true
    }

    pub(in crate::viewer) fn forward(&mut self) -> bool {
        if !self.active {
            return self.activate();
        }
        if self.candidates.is_empty() {
            return false;
        }
        let selected = self
            .selected
            .expect("active completion must have a selected candidate");
        self.selected = Some((selected + 1) % self.candidates.len());
        true
    }

    pub(in crate::viewer) fn backward(&mut self) -> bool {
        if !self.active {
            return self.activate();
        }
        if self.candidates.is_empty() {
            return false;
        }
        let selected = self
            .selected
            .expect("active completion must have a selected candidate");
        self.selected = Some(if selected == 0 {
            self.candidates.len() - 1
        } else {
            selected - 1
        });
        true
    }

    pub(in crate::viewer) fn accept(&mut self) -> Option<CompletionCandidate> {
        if !self.active {
            return None;
        }
        let candidate = self
            .selected
            .and_then(|selected| self.candidates.get(selected))
            .cloned();
        self.clear();
        candidate
    }

    pub(in crate::viewer) fn accept_single(&mut self) -> Option<CompletionCandidate> {
        if self.candidates.len() != 1 {
            return None;
        }
        let candidate = self.candidates.first().cloned();
        self.clear();
        candidate
    }

    fn clear(&mut self) {
        self.candidates.clear();
        self.selected = None;
        self.active = false;
    }

    pub(in crate::viewer) fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    pub(in crate::viewer) fn cardinality(&self) -> CompletionCardinality {
        match self.candidates.len() {
            0 => CompletionCardinality::Empty,
            1 => CompletionCardinality::Unique,
            _ => CompletionCardinality::Multiple,
        }
    }

    pub(in crate::viewer) fn is_active(&self) -> bool {
        self.active
    }
}

impl Widget for CompletionComponent {
    fn create_graphemes(&self) -> CreatedGraphemes {
        let graphemes = self
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                if self.active && Some(index) == self.selected {
                    StyledGraphemes::from(format!("❯ {}", candidate.display))
                } else {
                    StyledGraphemes::from(format!("  {}", candidate.display))
                }
            });

        CreatedGraphemes {
            graphemes: StyledGraphemes::from_lines(graphemes),
            layout: WidgetLayout {
                max_height: Some(self.visible_lines),
                width_mode: WidthMode::Truncate,
                ..Default::default()
            },
            cursor: (!self.candidates.is_empty()).then_some(ContentPosition {
                row: self.selected.unwrap_or_default(),
                column: 0,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates(values: &[&str]) -> Vec<CompletionCandidate> {
        values
            .iter()
            .map(|value| CompletionCandidate {
                display: (*value).into(),
                replacement: (*value).into(),
            })
            .collect()
    }

    #[test]
    fn activates_and_cycles_in_both_directions() {
        let mut component = CompletionComponent::default();
        component.set_candidates(candidates(&["@0", "@1"]));

        assert!(!component.is_active());
        assert!(component.forward());
        assert!(component.is_active());
        assert_eq!(component.selected, Some(0));

        assert!(component.forward());
        assert_eq!(component.selected, Some(1));
        assert!(component.forward());
        assert_eq!(component.selected, Some(0));

        assert!(component.backward());
        assert_eq!(component.selected, Some(1));
        assert!(component.backward());
        assert_eq!(component.selected, Some(0));
    }

    #[test]
    fn accepts_only_an_active_candidate_and_clears_the_list() {
        let mut component = CompletionComponent::default();
        component.set_candidates([CompletionCandidate {
            display: "@1".into(),
            replacement: "@1 ".into(),
        }]);

        assert_eq!(component.accept(), None);
        component.activate();
        assert_eq!(
            component.accept().map(|candidate| candidate.replacement),
            Some("@1 ".to_owned())
        );
        assert!(component.is_empty());
        assert!(!component.is_active());
    }

    #[test]
    fn accepts_a_single_inactive_candidate() {
        let mut component = CompletionComponent::default();
        component.set_candidates([CompletionCandidate {
            display: "[".into(),
            replacement: "[".into(),
        }]);

        assert_eq!(
            component
                .accept_single()
                .map(|candidate| candidate.replacement),
            Some("[".to_owned())
        );
        assert!(component.is_empty());
    }

    #[test]
    fn classifies_candidate_cardinality() {
        let mut component = CompletionComponent::default();
        assert_eq!(component.cardinality(), CompletionCardinality::Empty);

        component.set_candidates(candidates(&["."]));
        assert_eq!(component.cardinality(), CompletionCardinality::Unique);

        component.set_candidates(candidates(&[".", "["]));
        assert_eq!(component.cardinality(), CompletionCardinality::Multiple);
    }

    #[test]
    fn renders_a_vertically_scrollable_completion_list() {
        let mut component = CompletionComponent::default();
        component.set_candidates(candidates(&["@0", "@1", "@2"]));

        assert_eq!(
            component.create_graphemes().graphemes.to_string(),
            "  @0\n  @1\n  @2"
        );
        assert_eq!(
            component.create_graphemes().cursor,
            Some(ContentPosition { row: 0, column: 0 })
        );

        component.activate();
        component.forward();
        let created = component.create_graphemes();

        assert_eq!(created.graphemes.to_string(), "  @0\n❯ @1\n  @2");
        assert_eq!(created.layout.max_height, Some(DEFAULT_VISIBLE_LINES));
        assert_eq!(created.cursor, Some(ContentPosition { row: 1, column: 0 }));
    }
}
