---
title: "Keybinding settings"
description: "Configurable keybindings for each action and the scope of each configuration section."
sidebar:
  order: 1
  label: "Keybindings"
---

Use `keybinds.*` to assign keys for navigation, text editing, command execution, returning from views, and other actions. The tables below list the available settings and embedded default bindings.

## Configuration syntax

Each heading is a configuration section name. The Setting column lists keys within that section. Values can be a key string or an array of key strings.

```toml
[keybinds.view.browse.input]
# Change the keys for exiting from the input view
exit = ["q", "Esc"]

[keybinds.view.browse]
# Change the search history keys (defaults: Up / Down)
older_search_history = ["Ctrl+P"]
newer_search_history = ["Ctrl+N"]
```

An empty array `[]` disables the binding for that action. Omitted settings use their defaults. See the [config command](../../explore/commands/config.md) to save and apply changes. Updated bindings appear in vy's `:help` and on-screen hints.

Join modifiers and key names with `+`, as in `Ctrl+P`. `Shift+G` represents `G`, `ScrollUp` / `ScrollDown` represent the mouse wheel, and `LeftDown` represents the left mouse button.

`keybinds.component.*` configures shared document and editor actions, while `keybinds.view.*` configures actions for individual views. For example, configure editor cursor movement in `keybinds.component.text_editor` and search history recall in `keybinds.view.browse`.

## keybinds.component.document

Configure keys for document and list navigation, scrolling, and display toggles. `toggle`, `expand_all`, and `collapse_all` apply to views that support tree expansion and folding.

| Setting | Default binding | Action |
| --- | --- | --- |
| `up` | `["k", "Up", "ScrollUp"]` | Move to the previous item / display line |
| `down` | `["j", "Down", "ScrollDown"]` | Move to the next item / display line |
| `page_up` | `["PageUp", "Ctrl+B"]` | Move up one page |
| `page_down` | `["PageDown", "Ctrl+F"]` | Move down one page |
| `half_page_up` | `["Ctrl+U"]` | Move up half a page |
| `half_page_down` | `["Ctrl+D"]` | Move down half a page |
| `move_to_head` | `["g", "Home"]` | Move to the beginning |
| `move_to_tail` | `["Shift+G", "End"]` | Move to the end |
| `toggle` | `["Space", "Enter", "LeftDown"]` | Expand or collapse a node; mouse clicks act on the clicked item |
| `expand_all` | `["e"]` | Expand everything |
| `collapse_all` | `["Shift+E"]` | Collapse everything |
| `toggle_overflow_mode` | `["z"]` | Toggle wrapping / truncation |
| `toggle_line_numbers` | `["Alt+N"]` | Toggle line numbers |

## keybinds.component.text_editor

Configure keys for cursor movement, line breaks, and deletion in editors.

Word boundaries use *jaq* delimiters (`.`, `|`, `(`, `)`, `[`, `]`). Search history up/down bindings take priority over editor up/down movement.

| Setting | Default binding | Action |
| --- | --- | --- |
| `insert_newline` | `["Shift+Enter"]` | Insert a newline |
| `backward` | `["Left", "Ctrl+B"]` | Move the cursor left |
| `forward` | `["Right", "Ctrl+F"]` | Move the cursor right |
| `move_up` | `["Up"]` | Move the cursor up a line |
| `move_down` | `["Down"]` | Move the cursor down a line |
| `move_to_line_head` | `["Home", "Ctrl+A"]` | Move to the beginning of the line |
| `move_to_line_tail` | `["End", "Ctrl+E"]` | Move to the end of the line |
| `move_to_head` | `["Ctrl+Home"]` | Move to the beginning |
| `move_to_tail` | `["Ctrl+End"]` | Move to the end |
| `move_to_previous_nearest` | `["Alt+B", "Ctrl+Left"]` | Move to the previous word boundary |
| `move_to_next_nearest` | `["Alt+F", "Ctrl+Right"]` | Move to the next word boundary |
| `erase` | `["Backspace", "Ctrl+H"]` | Delete the previous character |
| `erase_forward` | `["Delete", "Ctrl+D"]` | Delete the character at the cursor |
| `erase_all` | `["Ctrl+U"]` | Delete all input |
| `erase_to_previous_nearest` | `["Ctrl+W", "Alt+Backspace"]` | Delete to the previous word boundary |
| `erase_to_next_nearest` | `["Alt+D", "Ctrl+Delete"]` | Delete to the next word boundary |

## keybinds.view.browse

Configure keys for command entry, search, and tree navigation in the input, *focus*, and *jaq* views. `toggle_focus` and `toggle_side_by_side` apply to *focus*. Configure *jaq* pane controls in `keybinds.view.jaq`.

| Setting | Default binding | Action |
| --- | --- | --- |
| `toggle_focus` | `["Tab"]` | Switch the active pane |
| `toggle_side_by_side` | `["s"]` | Toggle side-by-side input and results |
| `open_command_editor` | `[":", "Shift+:"]` | Open the regular command editor |
| `close_command_editor` | `["Esc", "Ctrl+C"]` | Close the regular command editor |
| `submit_command` | `["Enter"]` | Execute the command |
| `complete_command` | `["Tab"]` | Complete commands and arguments |
| `open_search_editor` | `["/"]` | Open the regular expression search editor |
| `close_search_editor` | `["Esc", "Ctrl+C"]` | Close the search editor |
| `submit_search` | `["Enter"]` | Run the search |
| `older_search_history` | `["Up"]` | Recall an older search expression |
| `newer_search_history` | `["Down"]` | Return to a newer search expression / draft |
| `next_search` | `["n"]` | Move to the next match |
| `previous_search` | `["Shift+N"]` | Move to the previous match |
| `collapse_or_move_to_parent` | `["h", "Left"]` | Collapse or move to the parent |
| `move_to_parent` | `["Shift+H"]` | Move to the parent |
| `expand_or_move_to_first_child` | `["l", "Right"]` | Expand or move to the first child |
| `move_to_next_sibling` | `["Shift+J"]` | Move to the next sibling |
| `move_to_previous_sibling` | `["Shift+K"]` | Move to the previous sibling |

## keybinds.view.browse.input

Configure keys for exiting vy from the input view.

| Setting | Default binding | Action |
| --- | --- | --- |
| `exit` | `["Esc", "Ctrl+C"]` | Exit vy |

## keybinds.view.browse.focus

Configure keys for returning from *focus* to the calling view.

| Setting | Default binding | Action |
| --- | --- | --- |
| `back` | `["Esc"]` | Return to the calling view |

## keybinds.view.preview

Configure keys for returning from the preview and copying selected text.

| Setting | Default binding | Action |
| --- | --- | --- |
| `back` | `["Esc"]` | Return to the calling view |
| `copy_selection` | `["y"]` | Copy the selected preview text |

## keybinds.view.help

Configure keys for returning from help to the calling view.

| Setting | Default binding | Action |
| --- | --- | --- |
| `back` | `["Esc"]` | Return to the calling view |

## keybinds.view.hstr

Configure keys for returning from history, switching between the editor and list, and selecting history entries.

| Setting | Default binding | Action |
| --- | --- | --- |
| `back` | `["Esc"]` | Return to the calling view |
| `toggle_focus` | `["Tab"]` | Switch between the history filter editor and results |
| `recall_selection` | `["Enter", "LeftDown"]` | Recall the selected history entry into the command editor |

## keybinds.view.flatten

Configure keys for editing and running grep expressions in *flatten*, canceling execution, and selecting results.

| Setting | Default binding | Action |
| --- | --- | --- |
| `back` | `["Esc"]` | Return to the calling view |
| `cancel_execution` | `["Ctrl+C"]` | Request cancellation of the running operation |
| `execute_query` | `["Enter"]` | Run the entered expression |
| `open_query_editor` | `[":", "Shift+:"]` | Open the editor for the current expression |
| `close_query_editor` | `["Esc"]` | Close the expression editor |
| `goto_selection` | `["Enter", "LeftDown"]` | Move to the selected result node |

## keybinds.view.jaq

Configure keys for returning from *jaq*, canceling a running query, and controlling the side-by-side panes.

| Setting | Default binding | Action |
| --- | --- | --- |
| `back` | `["Esc"]` | Return to the calling view |
| `cancel_execution` | `["Ctrl+C"]` | Request cancellation of the running operation |
| `toggle_focus` | `["Tab"]` | Switch the active pane |
| `toggle_side_by_side` | `["s"]` | Toggle side-by-side input and results |

Opening, submitting, and closing the expression editor use `open_command_editor`, `submit_command`, and `close_command_editor` from `keybinds.view.browse`.
