//! Two-document comparison UI with left and right panes.

use promkit::core::{
    ContentPosition, CreatedGraphemes, WidgetLayout, WidthMode,
    crossterm::style::{Attribute, ContentStyle},
    grapheme::{StyledGrapheme, StyledGraphemes},
};

use crate::viewer::DocumentAction;

use super::DocumentComponent;

const PANE_SEPARATOR: &str = " │ ";
const MIN_PANE_WIDTH: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum DocumentFocus {
    Left,
    Right,
}

/// Owns left and right documents together with their comparison layout and focus.
pub(in crate::viewer) struct DocumentComparisonComponent<T = DocumentComponent> {
    left: Option<T>,
    right: Option<T>,
    left_title: String,
    right_title: String,
    focus: DocumentFocus,
    side_by_side: bool,
    focus_pinned: bool,
    layout: Option<ComparisonLayout>,
}

/// A pane owns its data and exposes the document used by the comparison layout.
pub(in crate::viewer) trait ComparisonPane {
    fn document(&self) -> &DocumentComponent;
    fn document_mut(&mut self) -> &mut DocumentComponent;
}

impl ComparisonPane for DocumentComponent {
    fn document(&self) -> &DocumentComponent {
        self
    }
    fn document_mut(&mut self) -> &mut DocumentComponent {
        self
    }
}

#[cfg(test)]
impl DocumentComparisonComponent {
    pub(in crate::viewer) fn new(
        left: DocumentComponent,
        left_title: impl Into<String>,
        right_title: impl Into<String>,
    ) -> Self {
        Self::with_panes(Some(left), None, left_title, right_title)
    }
    pub(in crate::viewer) fn set_right(&mut self, right: DocumentComponent) {
        self.right = Some(right);
    }
}

impl<T: ComparisonPane> DocumentComparisonComponent<T> {
    pub(in crate::viewer) fn with_panes(
        left: Option<T>,
        right: Option<T>,
        left_title: impl Into<String>,
        right_title: impl Into<String>,
    ) -> Self {
        Self {
            left,
            right,
            left_title: left_title.into(),
            right_title: right_title.into(),
            focus: DocumentFocus::Right,
            side_by_side: false,
            focus_pinned: false,
            layout: None,
        }
    }

    pub(in crate::viewer) fn active(&self) -> Option<&T> {
        match self.focus {
            DocumentFocus::Left => self.left.as_ref(),
            DocumentFocus::Right => self.right.as_ref(),
        }
    }
    pub(in crate::viewer) fn active_mut(&mut self) -> Option<&mut T> {
        match self.focus {
            DocumentFocus::Left => self.left.as_mut(),
            DocumentFocus::Right => self.right.as_mut(),
        }
    }
    pub(in crate::viewer) fn set_left(&mut self, left: T) {
        self.left = Some(left);
    }
    pub(in crate::viewer) fn right(&self) -> Option<&T> {
        self.right.as_ref()
    }
    pub(in crate::viewer) fn right_mut(&mut self) -> Option<&mut T> {
        self.right.as_mut()
    }
    pub(in crate::viewer) fn panes_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.left.iter_mut().chain(self.right.iter_mut())
    }

    pub(in crate::viewer) fn pin_focus(&mut self, pinned: bool) {
        self.focus_pinned = pinned;
    }

    pub(in crate::viewer) fn focus(&self) -> DocumentFocus {
        self.focus
    }

    pub(in crate::viewer) fn focus_left(&mut self) -> bool {
        if self.side_by_side && self.layout.is_some() {
            self.focus = DocumentFocus::Left;
            true
        } else {
            false
        }
    }

    pub(in crate::viewer) fn focus_right(&mut self) {
        self.focus = DocumentFocus::Right;
    }

    pub(in crate::viewer) fn is_side_by_side(&self) -> bool {
        self.side_by_side
    }

    pub(in crate::viewer) fn toggle_side_by_side(&mut self) {
        self.side_by_side = self.left.is_some() && !self.side_by_side;
        if !self.side_by_side {
            self.focus_right();
            self.layout = None;
        }
    }

    #[cfg(test)]
    pub(in crate::viewer) fn create_right_graphemes(
        &self,
        width: u16,
        height: u16,
    ) -> CreatedGraphemes {
        self.right
            .as_ref()
            .map(|right| right.document().create_content_graphemes(width, height))
            .unwrap_or_default()
    }

    pub(in crate::viewer) fn prepare_render(&mut self, width: u16) {
        if !self.side_by_side || split_widths(width).is_none() {
            if !self.focus_pinned {
                self.focus_right();
            }
            self.layout = None;
        }
    }

    pub(in crate::viewer) fn render(&mut self, width: u16, height: u16) -> CreatedGraphemes {
        self.prepare_render(width);
        if !self.side_by_side {
            return self
                .active()
                .map(|pane| pane.document().create_content_graphemes(width, height))
                .unwrap_or_default();
        }

        let Some((content, layout)) = self.create_comparison_graphemes(width, height) else {
            return self
                .active()
                .map(|pane| pane.document().create_content_graphemes(width, height))
                .unwrap_or_default();
        };
        self.layout = Some(layout);
        content
    }

    pub(in crate::viewer) fn apply_action(
        &mut self,
        action: DocumentAction,
        click_position: Option<ContentPosition>,
        movement_lines: usize,
    ) -> bool {
        if let (Some(layout), Some(position)) = (&self.layout, click_position) {
            return match layout.document_position(position) {
                Some((DocumentFocus::Left, position)) => {
                    self.focus = DocumentFocus::Left;
                    self.left
                        .as_mut()
                        .expect("comparison has a left pane")
                        .document_mut()
                        .apply_action(action, Some(position), movement_lines);
                    true
                }
                Some((DocumentFocus::Right, position)) => {
                    self.focus = DocumentFocus::Right;
                    self.right.as_mut().is_some_and(|right| {
                        right
                            .document_mut()
                            .apply_action(action, Some(position), movement_lines);
                        true
                    })
                }
                None => false,
            };
        }

        match self.focus {
            DocumentFocus::Left if self.side_by_side => {
                self.left
                    .as_mut()
                    .expect("comparison has a left pane")
                    .document_mut()
                    .apply_action(action, click_position, movement_lines);
                true
            }
            DocumentFocus::Left | DocumentFocus::Right => {
                self.right.as_mut().is_some_and(|right| {
                    right
                        .document_mut()
                        .apply_action(action, click_position, movement_lines);
                    true
                })
            }
        }
    }

    fn create_comparison_graphemes(
        &self,
        width: u16,
        height: u16,
    ) -> Option<(CreatedGraphemes, ComparisonLayout)> {
        let (left_width, right_width) = split_widths(width)?;
        let content_height = height.saturating_sub(1);
        let title_rows = usize::from(height > 0);
        let left = PaneRows::create(
            self.left
                .as_ref()?
                .document()
                .create_content_graphemes(left_width as u16, content_height),
            left_width,
        )
        .into_viewport(usize::from(content_height));
        let right = PaneRows::create(
            self.right
                .as_ref()
                .map(|right| {
                    right
                        .document()
                        .create_content_graphemes(right_width as u16, content_height)
                })
                .unwrap_or_default(),
            right_width,
        )
        .into_viewport(usize::from(content_height));
        let row_count = usize::from(content_height);
        let separator = StyledGraphemes::from(PANE_SEPARATOR);
        let mut rows = Vec::with_capacity(usize::from(height));
        let mut right_rows = Vec::with_capacity(usize::from(height));
        if title_rows > 0 {
            rows.push(StyledGraphemes::from_iter([
                pad_to_width(
                    pane_title(
                        &self.left_title,
                        self.focus == DocumentFocus::Left,
                        left_width,
                    ),
                    left_width,
                ),
                separator.clone(),
                pane_title(
                    &self.right_title,
                    self.focus == DocumentFocus::Right,
                    right_width,
                ),
            ]));
            right_rows.push(None);
        }

        for row_index in 0..row_count {
            let left_row = left.rows.get(row_index).cloned().unwrap_or_default();
            let right_row = right.rows.get(row_index).cloned().unwrap_or_default();
            rows.push(StyledGraphemes::from_iter([
                pad_to_width(left_row, left_width),
                separator.clone(),
                right_row,
            ]));
            right_rows.push(right.logical_rows.get(row_index).copied());
        }

        let right_column = left_width + separator.widths();
        let cursor = match self.focus {
            DocumentFocus::Left => left.cursor_row.map(|row| ContentPosition {
                row: row + title_rows,
                column: 0,
            }),
            DocumentFocus::Right => right.cursor_row.map(|row| ContentPosition {
                row: row + title_rows,
                column: right_column,
            }),
        };
        let left_rows = std::iter::repeat_n(None, title_rows)
            .chain(left.logical_rows.into_iter().map(Some))
            .chain(std::iter::repeat(None))
            .take(usize::from(height))
            .collect();
        Some((
            CreatedGraphemes {
                graphemes: StyledGraphemes::from_lines(rows),
                layout: WidgetLayout {
                    width_mode: WidthMode::Truncate,
                    ..Default::default()
                },
                cursor,
            },
            ComparisonLayout {
                left_width,
                left_rows,
                right_column,
                right_rows,
            },
        ))
    }
}

struct ComparisonLayout {
    left_width: usize,
    left_rows: Vec<Option<usize>>,
    right_column: usize,
    right_rows: Vec<Option<usize>>,
}

impl ComparisonLayout {
    fn document_position(
        &self,
        position: ContentPosition,
    ) -> Option<(DocumentFocus, ContentPosition)> {
        let (focus, rows, column) = if position.column < self.left_width {
            (DocumentFocus::Left, &self.left_rows, position.column)
        } else if position.column >= self.right_column {
            (
                DocumentFocus::Right,
                &self.right_rows,
                position.column - self.right_column,
            )
        } else {
            return None;
        };
        rows.get(position.row)
            .copied()
            .flatten()
            .map(|row| (focus, ContentPosition { row, column }))
    }
}

struct PaneRows {
    rows: Vec<StyledGraphemes>,
    logical_rows: Vec<usize>,
    cursor_row: Option<usize>,
}

impl PaneRows {
    fn create(created: CreatedGraphemes, width: usize) -> Self {
        let mut rows = Vec::new();
        let mut logical_rows = Vec::new();
        let mut cursor_row = None;
        let cursor = created.cursor;

        for (logical_row, line) in created.graphemes.logical_lines().into_iter().enumerate() {
            if cursor.is_some_and(|cursor| cursor.row == logical_row) {
                cursor_row = Some(rows.len());
            }
            let visual_rows = match created.layout.width_mode {
                WidthMode::Wrap => {
                    let rows = line.wrapped_lines(width);
                    if rows.is_empty() {
                        vec![StyledGraphemes::default()]
                    } else {
                        rows
                    }
                }
                WidthMode::Truncate => {
                    vec![line.truncated_line_with_ellipsis(width, &StyledGraphemes::from("…"))]
                }
            };
            logical_rows.extend(std::iter::repeat_n(logical_row, visual_rows.len()));
            rows.extend(visual_rows);
        }

        Self {
            rows,
            logical_rows,
            cursor_row,
        }
    }

    fn into_viewport(self, height: usize) -> Self {
        let offset = self
            .cursor_row
            .map_or(0, |row| row.saturating_add(1).saturating_sub(height));
        let rows = self.rows.into_iter().skip(offset).take(height).collect();
        let logical_rows = self
            .logical_rows
            .into_iter()
            .skip(offset)
            .take(height)
            .collect();
        let cursor_row = self
            .cursor_row
            .and_then(|row| row.checked_sub(offset))
            .filter(|row| *row < height);
        Self {
            rows,
            logical_rows,
            cursor_row,
        }
    }
}

fn split_widths(width: u16) -> Option<(usize, usize)> {
    let available = usize::from(width).checked_sub(PANE_SEPARATOR.chars().count())?;
    let left = available / 2;
    let right = available - left;
    (left >= MIN_PANE_WIDTH && right >= MIN_PANE_WIDTH).then_some((left, right))
}

fn pad_to_width(mut row: StyledGraphemes, width: usize) -> StyledGraphemes {
    for _ in row.widths()..width {
        row.push_back(StyledGrapheme::from(' '));
    }
    row
}

fn pane_title(title: &str, focused: bool, width: usize) -> StyledGraphemes {
    let mut style = ContentStyle::default();
    style.attributes.set(Attribute::Bold);
    if focused {
        style.attributes.set(Attribute::Reverse);
    }
    StyledGraphemes::from_str(title, style)
        .truncated_line_with_ellipsis(width, &StyledGraphemes::from_str("…", style))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use promkit::core::render::RendererLayout;

    use crate::{config::Config, viewer::DocumentAction, viewer::ui::DocumentDisplayConfig};

    use super::*;

    fn json_document(content: &[u8]) -> DocumentComponent {
        let mut config = Config::default().json;
        config.show_line_numbers = false;
        DocumentComponent::parse(Cursor::new(content), DocumentDisplayConfig::Json(config)).unwrap()
    }

    #[test]
    fn renders_and_maps_a_side_by_side_comparison() {
        let mut comparison = DocumentComparisonComponent::new(
            json_document(br#"{"items":[1,2]}"#),
            "source",
            "result",
        );
        comparison.set_right(json_document(br#"[1,2]"#));
        comparison.toggle_side_by_side();

        let content = comparison.render(41, 20);
        let rows = content.graphemes.logical_lines();
        assert_eq!(rows[0].to_string(), "source              │ result");
        assert!(rows.iter().all(|row| row.widths() <= 41));
        assert!(rows.iter().all(|row| row.chars().get(20) == Some(&'│')));
        assert!(content.graphemes.to_string().contains("\"items\""));
        assert!(content.graphemes.to_string().contains(" │ ["));

        let layout = comparison.layout.as_ref().expect("comparison layout");
        for column in [0, 22] {
            assert_eq!(
                layout.document_position(ContentPosition { row: 0, column }),
                None,
                "titles do not belong to either document"
            );
        }
        assert!(matches!(
            layout.document_position(ContentPosition { row: 1, column: 0 }),
            Some((DocumentFocus::Left, ContentPosition { row: 0, column: 0 }))
        ));
        assert!(matches!(
            layout.document_position(ContentPosition { row: 1, column: 22 }),
            Some((DocumentFocus::Right, ContentPosition { row: 0, column: 0 }))
        ));
        assert_eq!(
            layout.document_position(ContentPosition { row: 1, column: 19 }),
            None,
            "the separator does not belong to either document"
        );
        assert_eq!(content.cursor, Some(ContentPosition { row: 1, column: 22 }));
    }

    #[test]
    fn routes_actions_to_the_focused_document() {
        let mut comparison = DocumentComparisonComponent::new(
            json_document(br#"{"items":[1,2]}"#),
            "source",
            "result",
        );
        comparison.set_right(json_document(br#"[1,2]"#));
        comparison.toggle_side_by_side();
        comparison.render(41, 20);

        let right_path = comparison
            .right
            .as_ref()
            .and_then(|right| right.selected_path());
        assert!(comparison.focus_left());
        assert!(comparison.apply_action(DocumentAction::Down, None, 1));
        assert_eq!(
            comparison.left.as_ref().unwrap().selected_path(),
            Some((0, ".items".into()))
        );
        assert_eq!(
            comparison
                .right
                .as_ref()
                .and_then(|right| right.selected_path()),
            right_path,
            "left navigation must not move the right"
        );
    }

    #[test]
    fn falls_back_to_the_right_when_the_terminal_is_too_narrow() {
        let mut comparison =
            DocumentComparisonComponent::new(json_document(b"null"), "source", "result");
        comparison.set_right(json_document(b"42"));
        comparison.toggle_side_by_side();

        assert_eq!(comparison.render(12, 20).graphemes.to_string(), "42");
        assert_eq!(comparison.focus(), DocumentFocus::Right);
        assert!(comparison.layout.is_none());
    }

    #[test]
    fn fills_the_available_height_in_side_by_side_mode() {
        let mut comparison =
            DocumentComparisonComponent::new(json_document(b"null"), "source", "result");
        comparison.set_right(json_document(b"42"));
        comparison.toggle_side_by_side();

        let content = comparison.render(41, 5);
        assert_eq!(content.graphemes.logical_lines().len(), 5);

        let title_only = comparison.render(41, 1);
        assert_eq!(title_only.graphemes.logical_lines().len(), 1);
        assert!(title_only.cursor.is_none());
        assert!(comparison.render(41, 0).graphemes.is_empty());
    }

    #[test]
    fn keeps_left_rows_stable_while_scrolling_the_right() {
        let mut comparison = DocumentComparisonComponent::new(
            json_document(br#"{"input":"the original document remains fixed"}"#),
            "source",
            "result",
        );
        comparison.set_right(json_document(
            br#"{
                "first":"a long value that wraps in the result pane",
                "second":"another long value that wraps in the result pane",
                "third":"a third long value that wraps in the result pane",
                "fourth":"a fourth long value that wraps in the result pane"
            }"#,
        ));
        comparison.toggle_side_by_side();
        let mut renderer = RendererLayout::<u8>::default();

        let initial = visible_left_rows(&mut renderer, comparison.render(41, 5));
        for _ in 0..4 {
            assert!(comparison.apply_action(DocumentAction::Down, None, 1));
            let moved = visible_left_rows(&mut renderer, comparison.render(41, 5));
            assert_eq!(moved, initial);
        }
    }

    fn visible_left_rows(
        renderer: &mut RendererLayout<u8>,
        content: CreatedGraphemes,
    ) -> Vec<String> {
        renderer.layout([(0, content)], 41, 5).unwrap().panes()[0]
            .iter()
            .map(|row| row.to_string().split(" │ ").next().unwrap().to_owned())
            .collect()
    }
}
