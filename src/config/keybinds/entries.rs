//! Keybinding descriptions used by the help view.

use super::{KeybindEntry, Keybinds};

impl Keybinds {
    pub(crate) fn entries(&self) -> Vec<KeybindEntry<'_>> {
        vec![
            KeybindEntry {
                name: "view.browse.toggle_focus",
                description: "Switch comparison panes in focus",
                events: &self.view.browse.toggle_focus,
            },
            KeybindEntry {
                name: "view.browse.toggle_side_by_side",
                description: "Compare a focused subtree with its input",
                events: &self.view.browse.toggle_side_by_side,
            },
            KeybindEntry {
                name: "view.browse.input.exit",
                description: "Exit the application",
                events: &self.view.browse.input.exit,
            },
            KeybindEntry {
                name: "view.browse.open_command_editor",
                description: "Open the command editor",
                events: &self.view.browse.open_command_editor,
            },
            KeybindEntry {
                name: "view.browse.close_command_editor",
                description: "Close the command editor",
                events: &self.view.browse.close_command_editor,
            },
            KeybindEntry {
                name: "view.browse.submit_command",
                description: "Submit the command",
                events: &self.view.browse.submit_command,
            },
            KeybindEntry {
                name: "view.browse.complete_command",
                description: "Complete the command",
                events: &self.view.browse.complete_command,
            },
            KeybindEntry {
                name: "view.browse.open_search_editor",
                description: "Open the tree search editor",
                events: &self.view.browse.open_search_editor,
            },
            KeybindEntry {
                name: "view.browse.close_search_editor",
                description: "Close the tree search editor",
                events: &self.view.browse.close_search_editor,
            },
            KeybindEntry {
                name: "view.browse.submit_search",
                description: "Submit the tree search",
                events: &self.view.browse.submit_search,
            },
            KeybindEntry {
                name: "view.browse.older_search_history",
                description: "Recall an older search in the search editor",
                events: &self.view.browse.older_search_history,
            },
            KeybindEntry {
                name: "view.browse.newer_search_history",
                description: "Recall a newer search or restore the draft",
                events: &self.view.browse.newer_search_history,
            },
            KeybindEntry {
                name: "view.browse.next_search",
                description: "Move to the next tree search match",
                events: &self.view.browse.next_search,
            },
            KeybindEntry {
                name: "view.browse.previous_search",
                description: "Move to the previous tree search match",
                events: &self.view.browse.previous_search,
            },
            KeybindEntry {
                name: "view.browse.collapse_or_move_to_parent",
                description: "Collapse the node or move to its parent",
                events: &self.view.browse.collapse_or_move_to_parent,
            },
            KeybindEntry {
                name: "view.browse.move_to_parent",
                description: "Move directly to the parent node",
                events: &self.view.browse.move_to_parent,
            },
            KeybindEntry {
                name: "view.browse.expand_or_move_to_first_child",
                description: "Expand the node or move to its first child",
                events: &self.view.browse.expand_or_move_to_first_child,
            },
            KeybindEntry {
                name: "view.browse.move_to_next_sibling",
                description: "Move to the next sibling node",
                events: &self.view.browse.move_to_next_sibling,
            },
            KeybindEntry {
                name: "view.browse.move_to_previous_sibling",
                description: "Move to the previous sibling node",
                events: &self.view.browse.move_to_previous_sibling,
            },
            KeybindEntry {
                name: "view.browse.focus.back",
                description: "Return from the focus view",
                events: &self.view.browse.focus.back,
            },
            KeybindEntry {
                name: "view.preview.back",
                description: "Return from the text preview",
                events: &self.view.preview.back,
            },
            KeybindEntry {
                name: "view.preview.copy_selection",
                description: "Copy the selected preview text",
                events: &self.view.preview.copy_selection,
            },
            KeybindEntry {
                name: "view.help.back",
                description: "Return from the help view",
                events: &self.view.help.back,
            },
            KeybindEntry {
                name: "view.hstr.back",
                description: "Return from the history view",
                events: &self.view.hstr.back,
            },
            KeybindEntry {
                name: "view.hstr.toggle_focus",
                description: "Switch history focus between editor and result",
                events: &self.view.hstr.toggle_focus,
            },
            KeybindEntry {
                name: "view.hstr.recall_selection",
                description: "Recall the selected history entry",
                events: &self.view.hstr.recall_selection,
            },
            KeybindEntry {
                name: "view.flatten.back",
                description: "Return from the flatten view",
                events: &self.view.flatten.back,
            },
            KeybindEntry {
                name: "view.flatten.cancel_execution",
                description: "Cancel grep execution",
                events: &self.view.flatten.cancel_execution,
            },
            KeybindEntry {
                name: "view.flatten.execute_query",
                description: "Execute the grep query",
                events: &self.view.flatten.execute_query,
            },
            KeybindEntry {
                name: "view.flatten.open_query_editor",
                description: "Open the flatten query editor",
                events: &self.view.flatten.open_query_editor,
            },
            KeybindEntry {
                name: "view.flatten.close_query_editor",
                description: "Close the flatten query editor",
                events: &self.view.flatten.close_query_editor,
            },
            KeybindEntry {
                name: "view.flatten.goto_selection",
                description: "Go to the selected flatten result",
                events: &self.view.flatten.goto_selection,
            },
            KeybindEntry {
                name: "view.jaq.back",
                description: "Return from the jaq view",
                events: &self.view.jaq.back,
            },
            KeybindEntry {
                name: "view.jaq.cancel_execution",
                description: "Cancel jaq execution",
                events: &self.view.jaq.cancel_execution,
            },
            KeybindEntry {
                name: "view.jaq.toggle_focus",
                description: "Switch focus between the jaq input and result",
                events: &self.view.jaq.toggle_focus,
            },
            KeybindEntry {
                name: "view.jaq.toggle_side_by_side",
                description: "Compare the jaq result with its input",
                events: &self.view.jaq.toggle_side_by_side,
            },
            KeybindEntry {
                name: "component.document.up",
                description: "Move the active item up",
                events: &self.component.document.up,
            },
            KeybindEntry {
                name: "component.document.down",
                description: "Move the active item down",
                events: &self.component.document.down,
            },
            KeybindEntry {
                name: "component.document.page_up",
                description: "Move up one page",
                events: &self.component.document.page_up,
            },
            KeybindEntry {
                name: "component.document.page_down",
                description: "Move down one page",
                events: &self.component.document.page_down,
            },
            KeybindEntry {
                name: "component.document.half_page_up",
                description: "Move up half a page",
                events: &self.component.document.half_page_up,
            },
            KeybindEntry {
                name: "component.document.half_page_down",
                description: "Move down half a page",
                events: &self.component.document.half_page_down,
            },
            KeybindEntry {
                name: "component.document.move_to_head",
                description: "Move to the first item",
                events: &self.component.document.move_to_head,
            },
            KeybindEntry {
                name: "component.document.move_to_tail",
                description: "Move to the last item",
                events: &self.component.document.move_to_tail,
            },
            KeybindEntry {
                name: "component.document.toggle",
                description: "Expand or collapse the active item",
                events: &self.component.document.toggle,
            },
            KeybindEntry {
                name: "component.document.expand_all",
                description: "Expand all document items",
                events: &self.component.document.expand_all,
            },
            KeybindEntry {
                name: "component.document.collapse_all",
                description: "Collapse all document items",
                events: &self.component.document.collapse_all,
            },
            KeybindEntry {
                name: "component.document.toggle_overflow_mode",
                description: "Toggle document wrapping and truncation",
                events: &self.component.document.toggle_overflow_mode,
            },
            KeybindEntry {
                name: "component.document.toggle_line_numbers",
                description: "Toggle document line numbers",
                events: &self.component.document.toggle_line_numbers,
            },
            KeybindEntry {
                name: "component.text_editor.insert_newline",
                description: "Insert a newline in the editor",
                events: &self.component.text_editor.insert_newline,
            },
            KeybindEntry {
                name: "component.text_editor.backward",
                description: "Move the editor cursor backward",
                events: &self.component.text_editor.backward,
            },
            KeybindEntry {
                name: "component.text_editor.forward",
                description: "Move the editor cursor forward",
                events: &self.component.text_editor.forward,
            },
            KeybindEntry {
                name: "component.text_editor.move_up",
                description: "Move the editor cursor up",
                events: &self.component.text_editor.move_up,
            },
            KeybindEntry {
                name: "component.text_editor.move_down",
                description: "Move the editor cursor down",
                events: &self.component.text_editor.move_down,
            },
            KeybindEntry {
                name: "component.text_editor.move_to_line_head",
                description: "Move to the beginning of the editor line",
                events: &self.component.text_editor.move_to_line_head,
            },
            KeybindEntry {
                name: "component.text_editor.move_to_line_tail",
                description: "Move to the end of the editor line",
                events: &self.component.text_editor.move_to_line_tail,
            },
            KeybindEntry {
                name: "component.text_editor.move_to_head",
                description: "Move to the beginning of the editor",
                events: &self.component.text_editor.move_to_head,
            },
            KeybindEntry {
                name: "component.text_editor.move_to_tail",
                description: "Move to the end of the editor",
                events: &self.component.text_editor.move_to_tail,
            },
            KeybindEntry {
                name: "component.text_editor.move_to_previous_nearest",
                description: "Move to the previous editor word boundary",
                events: &self.component.text_editor.move_to_previous_nearest,
            },
            KeybindEntry {
                name: "component.text_editor.move_to_next_nearest",
                description: "Move to the next editor word boundary",
                events: &self.component.text_editor.move_to_next_nearest,
            },
            KeybindEntry {
                name: "component.text_editor.erase",
                description: "Erase the character before the editor cursor",
                events: &self.component.text_editor.erase,
            },
            KeybindEntry {
                name: "component.text_editor.erase_forward",
                description: "Erase the character after the editor cursor",
                events: &self.component.text_editor.erase_forward,
            },
            KeybindEntry {
                name: "component.text_editor.erase_all",
                description: "Erase all editor input",
                events: &self.component.text_editor.erase_all,
            },
            KeybindEntry {
                name: "component.text_editor.erase_to_previous_nearest",
                description: "Erase to the previous editor word boundary",
                events: &self.component.text_editor.erase_to_previous_nearest,
            },
            KeybindEntry {
                name: "component.text_editor.erase_to_next_nearest",
                description: "Erase to the next editor word boundary",
                events: &self.component.text_editor.erase_to_next_nearest,
            },
        ]
    }
}
