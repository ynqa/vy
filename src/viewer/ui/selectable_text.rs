use std::ops::Range;

use promkit::core::{
    ContentPosition, CreatedGraphemes, WidgetLayout, WidthMode,
    crossterm::{
        event::{MouseButton, MouseEventKind},
        style::Attribute,
    },
    grapheme::StyledGraphemes,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

/// Selectable, read-only text with cached visual rows and viewport-only rendering.
///
/// Owns its text so byte selections cannot outlive or become detached from the
/// content. Create a new component when replacing the text. Call `create_graphemes`
/// with the content pane size before forwarding content-relative mouse positions.
/// Views decide keybindings, feedback, and how to copy `selected_text()`.
pub(in crate::viewer) struct SelectableTextComponent {
    content: String,
    rows: Vec<Range<usize>>,
    width: usize,
    offset: usize,
    viewport_height: usize,
    selection: Option<Range<usize>>,
    drag: Option<DragSelection>,
}

struct DragSelection {
    anchor: Range<usize>,
    position: ContentPosition,
}

impl SelectableTextComponent {
    pub(in crate::viewer) fn new(content: &str) -> Self {
        Self {
            content: display_text(content),
            rows: Vec::new(),
            width: 0,
            offset: 0,
            viewport_height: 1,
            selection: None,
            drag: None,
        }
    }

    pub(in crate::viewer) fn offset(&self) -> usize {
        self.offset
    }

    /// Scroll to a visual row, clamping to the viewport bounds. While dragging,
    /// extend the selection under the last pointer position; otherwise retain it.
    pub(in crate::viewer) fn set_offset(&mut self, offset: usize) {
        let previous_offset = self.offset;
        self.offset = offset.min(self.max_offset());
        if previous_offset != self.offset
            && let Some(drag) = &self.drag
        {
            self.extend_selection(drag.position);
        }
    }

    fn layout_rows(&mut self, width: usize) {
        if width == 0 || width == self.width {
            return;
        }
        let anchor = self.rows.get(self.offset).map_or(0, |row| row.start);
        self.rows.clear();
        let mut start = 0;
        let mut columns = 0;
        for (index, grapheme) in self.content.grapheme_indices(true) {
            if grapheme == "\n" {
                self.rows.push(start..index);
                start = index + 1;
                columns = 0;
                continue;
            }
            let char_width: usize = grapheme.chars().map(|ch| ch.width().unwrap_or(0)).sum();
            if columns > 0 && columns + char_width > width {
                self.rows.push(start..index);
                start = index;
                columns = 0;
            }
            columns += char_width;
        }
        // Preserve empty strings and a final newline as an empty visual row.
        self.rows.push(start..self.content.len());
        self.width = width;
        self.offset = self
            .rows
            .partition_point(|row| row.start <= anchor)
            .saturating_sub(1);
    }

    pub(in crate::viewer) fn create_graphemes(
        &mut self,
        width: u16,
        height: u16,
    ) -> CreatedGraphemes {
        if width == 0 {
            return CreatedGraphemes::default();
        }
        self.layout_rows(usize::from(width));
        self.viewport_height = usize::from(height).max(1);
        self.offset = self.offset.min(self.max_offset());
        let lines = self
            .rows
            .iter()
            .skip(self.offset)
            .take(usize::from(height))
            .map(|range| self.render_row(range));
        CreatedGraphemes {
            graphemes: StyledGraphemes::from_lines(lines),
            layout: WidgetLayout {
                max_height: Some(usize::from(height)),
                // Rows are already wrapped so scrolling addresses visual rows.
                width_mode: WidthMode::Truncate,
                ..Default::default()
            },
            cursor: None,
        }
    }

    fn render_row(&self, row: &Range<usize>) -> StyledGraphemes {
        let Some(selection) = &self.selection else {
            return StyledGraphemes::from(&self.content[row.clone()]);
        };
        let start = selection.start.max(row.start);
        let end = selection.end.min(row.end);
        let mut rendered = if start < end {
            [
                StyledGraphemes::from(&self.content[row.start..start]),
                StyledGraphemes::from(&self.content[start..end])
                    .apply_attribute(Attribute::Reverse),
                StyledGraphemes::from(&self.content[end..row.end]),
            ]
            .into_iter()
            .collect::<StyledGraphemes>()
        } else {
            StyledGraphemes::from(&self.content[row.clone()])
        };
        // Make selected hard newlines visible, including otherwise empty lines.
        // This marker belongs only to the display; copying uses the content range.
        if selection.contains(&row.end)
            && self.content.as_bytes().get(row.end) == Some(&b'\n')
            && rendered.widths() < self.width
        {
            rendered = [
                rendered,
                StyledGraphemes::from(" ").apply_attribute(Attribute::Reverse),
            ]
            .into_iter()
            .collect();
        }
        rendered
    }

    /// Renderer columns are terminal cells, while selections are UTF-8 byte ranges.
    /// Select entire grapheme clusters, including wide characters and combining marks.
    fn text_span_at(&self, position: ContentPosition) -> Option<Range<usize>> {
        if position.row >= self.viewport_height || position.column >= self.width {
            return None;
        }
        let row = self.rows.get(self.offset + position.row)?;
        let mut column = 0;
        for (index, grapheme) in self.content[row.clone()].grapheme_indices(true) {
            column += grapheme
                .chars()
                .map(|ch| ch.width().unwrap_or(0))
                .sum::<usize>();
            if position.column < column {
                let start = row.start + index;
                return Some(start..start + grapheme.len());
            }
        }
        // Blank cells to the right of a line represent its end, not the next line.
        Some(row.end..row.end)
    }

    fn extend_selection(&mut self, position: ContentPosition) {
        let Some(head) = self.text_span_at(position) else {
            return;
        };
        if let Some(drag) = &mut self.drag {
            self.selection = Some(drag.anchor.start.min(head.start)..drag.anchor.end.max(head.end));
            drag.position = position;
        }
    }

    /// Handle a content-relative pointer event from the last rendered viewport.
    /// `None` means outside the content pane. Returns whether the event was consumed
    /// and the caller should render again; it never writes to the clipboard.
    pub(in crate::viewer) fn handle_pointer(
        &mut self,
        kind: MouseEventKind,
        position: Option<ContentPosition>,
    ) -> bool {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.drag = None;
                if let Some(position) = position
                    && let Some(anchor) = self.text_span_at(position)
                {
                    self.selection = None;
                    self.drag = Some(DragSelection { anchor, position });
                    return true;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) if self.drag.is_some() => {
                if let Some(position) = position {
                    self.extend_selection(position);
                }
                return true;
            }
            MouseEventKind::Up(MouseButton::Left) if self.drag.is_some() => {
                if let Some(position) = position
                    && (self.selection.is_some()
                        || self
                            .drag
                            .as_ref()
                            .is_some_and(|drag| drag.position != position))
                {
                    self.extend_selection(position);
                }
                self.drag = None;
                return true;
            }
            _ => {}
        }
        false
    }

    /// Text ready for copying, without soft-wrap newlines or selection markers.
    /// Tabs and control characters use the same normalization as the displayed text.
    pub(in crate::viewer) fn selected_text(&self) -> Option<&str> {
        self.selection
            .as_ref()
            .filter(|range| !range.is_empty())
            .map(|range| &self.content[range.clone()])
    }

    pub(in crate::viewer) fn max_offset(&self) -> usize {
        self.rows.len().saturating_sub(self.viewport_height)
    }
}

/// Expand tabs to four-column stops and render terminal controls as visible escapes.
/// Newlines are already decoded by the format parser; literal `\n` stays literal.
fn display_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut columns = 0;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' => {
                output.push('\n');
                columns = 0;
            }
            '\t' => {
                let spaces = 4 - columns % 4;
                output.extend(std::iter::repeat_n(' ', spaces));
                columns += spaces;
            }
            ch if ch.is_control() => {
                for escaped in ch.escape_default() {
                    output.push(escaped);
                    columns += 1;
                }
            }
            ch => {
                output.push(ch);
                columns += ch.width().unwrap_or(0);
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(content: &str) -> SelectableTextComponent {
        SelectableTextComponent::new(content)
    }

    fn pointer(
        view: &mut SelectableTextComponent,
        kind: MouseEventKind,
        row: usize,
        column: usize,
    ) {
        assert!(view.handle_pointer(kind, Some(ContentPosition { row, column })));
    }

    fn select(view: &mut SelectableTextComponent, from: (usize, usize), to: (usize, usize)) {
        pointer(
            view,
            MouseEventKind::Down(MouseButton::Left),
            from.0,
            from.1,
        );
        pointer(view, MouseEventKind::Drag(MouseButton::Left), to.0, to.1);
        pointer(view, MouseEventKind::Up(MouseButton::Left), to.0, to.1);
    }

    #[test]
    fn highlights_only_the_selected_characters() {
        let mut view = view("abcdef");
        view.create_graphemes(4, 2);
        select(&mut view, (0, 1), (1, 0));
        let expected = StyledGraphemes::from_lines([
            [
                StyledGraphemes::from("a"),
                StyledGraphemes::from("bcd").apply_attribute(Attribute::Reverse),
            ]
            .into_iter()
            .collect::<StyledGraphemes>(),
            [
                StyledGraphemes::from("e").apply_attribute(Attribute::Reverse),
                StyledGraphemes::from("f"),
            ]
            .into_iter()
            .collect::<StyledGraphemes>(),
        ]);
        assert_eq!(view.create_graphemes(4, 2).graphemes, expected);
    }

    #[test]
    fn maps_terminal_cells_to_complete_unicode_clusters() {
        let mut view = view("日e\u{301}本👩\u{200d}💻x");
        view.create_graphemes(20, 2);
        select(&mut view, (0, 1), (0, 2));
        assert_eq!(view.selected_text(), Some("日e\u{301}"));
        select(&mut view, (0, 5), (0, 8));
        assert_eq!(view.selected_text(), Some("👩\u{200d}💻"));
        view.create_graphemes(4, 5);
        assert_eq!(view.selected_text(), Some("👩\u{200d}💻"));
        assert!(
            view.rows
                .iter()
                .any(|row| &view.content[row.clone()] == "👩\u{200d}💻")
        );
    }

    #[test]
    fn scrolling_during_drag_extends_selection_from_its_original_anchor() {
        let mut view = view("abc\ndef\nghi\njkl");
        view.create_graphemes(10, 2);
        pointer(&mut view, MouseEventKind::Down(MouseButton::Left), 0, 1);
        pointer(&mut view, MouseEventKind::Drag(MouseButton::Left), 1, 1);
        view.set_offset(view.offset().saturating_add(1));
        assert_eq!(view.selected_text(), Some("bc\ndef\ngh"));
        pointer(&mut view, MouseEventKind::Up(MouseButton::Left), 1, 1);
        view.set_offset(view.max_offset());
        assert_eq!(view.selected_text(), Some("bc\ndef\ngh"));
    }

    #[test]
    fn release_outside_content_ends_drag_and_other_buttons_are_ignored() {
        let mut view = view("abcdef");
        view.create_graphemes(10, 2);
        pointer(&mut view, MouseEventKind::Down(MouseButton::Left), 0, 1);
        pointer(&mut view, MouseEventKind::Drag(MouseButton::Left), 0, 3);
        assert!(view.handle_pointer(MouseEventKind::Up(MouseButton::Left), None));
        assert!(view.drag.is_none());
        for kind in [
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Down(MouseButton::Right),
        ] {
            assert!(!view.handle_pointer(kind, Some(ContentPosition { row: 0, column: 5 })));
        }
        assert_eq!(view.selected_text(), Some("bcd"));
        assert!(!view.handle_pointer(MouseEventKind::Down(MouseButton::Left), None));
        assert_eq!(view.selected_text(), Some("bcd"));
    }

    #[test]
    fn blank_cells_select_line_end_and_empty_lines_preserve_newlines() {
        let mut view = view("abc\n\ndef");
        view.create_graphemes(10, 3);
        select(&mut view, (0, 8), (2, 0));
        assert_eq!(view.selected_text(), Some("\n\nd"));
        assert_eq!(
            view.render_row(&view.rows[1]),
            StyledGraphemes::from(" ").apply_attribute(Attribute::Reverse)
        );
        select(&mut view, (1, 2), (1, 5));
        assert_eq!(view.selected_text(), None);
        assert!(
            view.text_span_at(ContentPosition { row: 3, column: 0 })
                .is_none()
        );
        assert!(
            view.text_span_at(ContentPosition { row: 0, column: 10 })
                .is_none()
        );
    }

    #[test]
    fn scrolls_wrapped_unicode_text_and_preserves_blank_lines() {
        let mut view = view("abcdef\n日本語\n\nend\n");
        assert_eq!(
            view.create_graphemes(4, 2).graphemes.to_string(),
            "abcd\nef"
        );
        view.set_offset(view.offset().saturating_add(1));
        assert_eq!(
            view.create_graphemes(4, 2).graphemes.to_string(),
            "ef\n日本"
        );
        view.set_offset(view.offset().saturating_add(2));
        assert_eq!(view.create_graphemes(4, 2).graphemes.to_string(), "語\n");
        view.set_offset(view.max_offset());
        assert_eq!(view.create_graphemes(4, 2).graphemes.to_string(), "end\n");
        view.set_offset(view.offset().saturating_sub(2));
        assert_eq!(view.create_graphemes(4, 2).graphemes.to_string(), "語\n");
        view.set_offset(0);
        assert_eq!(view.create_graphemes(4, 1).graphemes.to_string(), "abcd");
    }

    #[test]
    fn reflows_on_resize_around_the_top_character() {
        let mut view = view("abcdefghij");
        view.create_graphemes(4, 1);
        view.set_offset(view.offset().saturating_add(1));
        assert_eq!(view.create_graphemes(4, 1).graphemes.to_string(), "efgh");
        assert_eq!(view.create_graphemes(3, 1).graphemes.to_string(), "def");
        assert_eq!(view.create_graphemes(6, 1).graphemes.to_string(), "abcdef");
        assert_eq!(
            view.create_graphemes(20, 2).graphemes.to_string(),
            "abcdefghij"
        );
    }

    #[test]
    fn handles_empty_text_tiny_viewports_and_combining_characters() {
        for text in ["", "\n", "日本", "e\u{301}x"] {
            let mut view = view(text);
            assert!(view.create_graphemes(0, 0).graphemes.is_empty());
            assert!(view.create_graphemes(1, 0).graphemes.is_empty());
            view.set_offset(view.max_offset());
            view.create_graphemes(1, 1);
            view.set_offset(0);
            assert_eq!(view.create_graphemes(80, 24).graphemes.to_string(), text);
        }
        assert_eq!(
            view("e\u{301}x")
                .create_graphemes(1, 2)
                .graphemes
                .to_string(),
            "e\u{301}\nx"
        );
    }

    #[test]
    fn leaves_literal_escapes_intact_and_displays_controls_safely() {
        assert_eq!(
            display_text("a\tb\r\n日本\tend\n\\n"),
            "a   b\n日本    end\n\\n"
        );
        let text = display_text("\x1b[31mred\x07\r");
        assert!(!text.chars().any(char::is_control));
        assert!(text.contains("[31mred"));
    }

    #[test]
    fn projects_only_the_viewport_of_a_large_single_line() {
        let mut view = view(&"0123456789".repeat(100_000));
        let frame = view.create_graphemes(80, 20);
        assert_eq!(frame.graphemes.len(), 80 * 20 + 19);
        view.set_offset(view.max_offset());
        let frame = view.create_graphemes(80, 20);
        assert_eq!(frame.graphemes.len(), 80 * 20 + 19);
        assert_eq!(frame.layout.max_height, Some(20));
    }
}
