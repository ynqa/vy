---
title: "vs fx, jless"
description: "A feature comparison based on official sources, related issues, and the vy implementation."
sidebar:
  order: 4
  label: "vs fx, jless"
---

[fx](https://fx.wtf/) and [jless](https://jless.io/) are popular terminal JSON viewers. This page compares their features with vy.

Comparison as of 2026-09-14. It covers the current vy implementation, fx [39.2.0](https://github.com/antonmedv/fx/releases/tag/39.2.0), and jless [0.9.0](https://github.com/PaulJuliusMartinez/jless/releases/tag/v0.9.0), based on official documentation, source code, and related issues.
Features available only by piping to external commands are not counted as built-in support.

- ⚪︎: Supported
- △: Partial support or limitations (see Notes)
- ×: Not supported in the release being compared

YAML and TOML browsing is also assessed on whether the original data format is preserved in the display.
Symbols for fx and jless link to supporting official sources or related issues where available. An issue's open or closed status alone does not determine feature support.

## Input

| Feature | vy | fx | jless | Notes |
| --- | :---: | :---: | :---: | --- |
| JSON browsing | ⚪︎ | [⚪︎](https://fx.wtf/getting-started#interactive-mode) | [⚪︎](https://jless.io/user-guide#usage) | vy detects the input format from the extension and content. Use `--format` to specify it explicitly. |
| YAML browsing | ⚪︎ | △ | △ | vy preserves YAML display. fx and jless can read YAML but display it as JSON. |
| JSON Lines / NDJSON browsing | ⚪︎ | [⚪︎](https://fx.wtf/getting-started#streaming-mode) | [⚪︎](https://jless.io/user-guide#usage) | Applies to finite input that reaches its end. |
| TOML browsing | × | △ | [×](https://github.com/PaulJuliusMartinez/jless/issues/129) | fx can read TOML but displays it as JSON. vy accepts JSON and YAML input. |
| Comments in JSON | × | [⚪︎](https://github.com/antonmedv/fx/issues/210#issuecomment-1725074054) | [×](https://github.com/PaulJuliusMartinez/jless/issues/154) | Separate from full JSON5 support. |
| YAML streams with multiple documents | ⚪︎ | △ | △ | vy displays each document as YAML and uses indices such as `@0` / `@1` to target *goto*, *focus*, and output. fx and jless can read these streams but display them as JSON. |

## Display

| Feature | vy | fx | jless | Notes |
| --- | :---: | :---: | :---: | --- |
| Wrap long strings in the tree | ⚪︎ | [⚪︎](https://fx.wtf/key-bindings) | [×](https://github.com/PaulJuliusMartinez/jless/issues/34) | vy toggles Wrap / Truncate with `z`. |
| Dedicated preview for long strings | ⚪︎ | [⚪︎](https://fx.wtf/getting-started#printing) | [△](https://github.com/PaulJuliusMartinez/jless/issues/168) | vy uses `:preview` to wrap and scroll decoded strings. jless has limitations with multipage output from `ps` / `pp`. |
| Child counts on collapsed arrays and objects | ⚪︎ | [⚪︎](https://fx.wtf/configuration#fx-show-size) | ⚪︎ | vy uses JSON / YAML `show_child_count` to display immediate child counts, including empty containers. |
| Read strings with escapes decoded | ⚪︎ | [⚪︎](https://github.com/antonmedv/fx/issues/287#issuecomment-1987372621) | [⚪︎](https://jless.io/user-guide#copying-and-printing) | vy's `:preview` decodes JSON escapes and YAML block strings, preserving line breaks and blank lines. |
| Select and copy ranges in a dedicated preview | ⚪︎ | [△](https://fx.wtf/getting-started#text-selection) | [△](https://jless.io/user-guide#copying-and-printing) | vy supports drag selection and copying with `y`. Selection persists after scrolling, and copied text excludes line breaks introduced by wrapping. fx and jless rely on terminal selection. |
| Display and toggle line numbers at runtime | ⚪︎ | [⚪︎](https://fx.wtf/configuration#fx-line-numbers) | [⚪︎](https://jless.io/user-guide#line-numbers) | vy displays line numbers for JSON / YAML and toggles them with `Alt+N`. Display settings can be saved in TOML. |

## Navigation

| Feature | vy | fx | jless | Notes |
| --- | :---: | :---: | :---: | --- |
| Vim-like navigation, including parent, child, and sibling nodes | ⚪︎ | [⚪︎](https://fx.wtf/key-bindings) | [⚪︎](https://jless.io/user-guide#moving) | vy uses `j` / `k`, `h` / `l`, `H`, `J` / `K`, `Ctrl-d` / `Ctrl-u`, and more. |
| Expand and collapse nodes individually or in bulk | ⚪︎ | [⚪︎](https://fx.wtf/key-bindings) | [⚪︎](https://jless.io/user-guide#moving) | vy toggles nodes with `Space` / `Enter` and expands / collapses everything with `e` / `E`. |
| Navigate to a path with path completion | ⚪︎ | [⚪︎](https://fx.wtf/getting-started#navigating) | [×](https://github.com/PaulJuliusMartinez/jless/issues/173) | vy uses `:goto @<document-index> <path>`. fx uses `.` / `@`. |
| Explore a selected subtree as a temporary root and return | ⚪︎ | [×](https://github.com/antonmedv/fx/issues/401) | [×](https://github.com/PaulJuliusMartinez/jless/issues/80) | vy opens the selected node in a separate view with `:focus` and returns with `Esc`. |

## Search

| Feature | vy | fx | jless | Notes |
| --- | :---: | :---: | :---: | --- |
| Regular expression search and next / previous match navigation | ⚪︎ | [⚪︎](https://fx.wtf/getting-started#searching) | [⚪︎](https://jless.io/user-guide#search) | vy uses `/` and `n` / `N`, including collapsed nodes. jless treats `[]` and `{}` as literals; character classes and repetition require escaping. |
| Search scoped to a selected subtree | ⚪︎ | [×](https://github.com/antonmedv/fx/issues/401) | [×](https://github.com/PaulJuliusMartinez/jless/issues/80) | In vy, use `/` after `:focus` to search within the subtree. |
| *flatten* view of paths and values with regular expression filtering | ⚪︎ | [×](https://fx.wtf/key-bindings) | [×](https://jless.io/user-guide#commands) | vy uses `:flatten`. The official fx and jless command lists do not document a dedicated view. |

## Queries

| Feature | vy | fx | jless | Notes |
| --- | :---: | :---: | :---: | --- |
| Run expressions in the TUI and explore filtered results | ⚪︎ | × | [×](https://github.com/PaulJuliusMartinez/jless/issues/70) | vy uses `:jaq <expr>` / `:j <expr>`. In fx 39.2.0, `.` navigates to an existing node by path; JavaScript transformations are specified as CLI arguments. |
| Display input and query results side by side | ⚪︎ | [×](https://fx.wtf/key-bindings) | [×](https://jless.io/user-guide#commands) | While browsing *jaq*, vy toggles side-by-side mode with `s` and switches panes with `Tab` or the mouse. This is separate from structural diffing. |

## Output

| Feature | vy | fx | jless | Notes |
| --- | :---: | :---: | :---: | --- |
| Copy a selected path, value, or subtree | ⚪︎ | ⚪︎ | [⚪︎](https://jless.io/user-guide#copying-and-printing) | vy supports `:copy path` / `value` / `subtree` in the active pane of the input, *focus*, and *jaq* views. |
| Send selected results to stdout for further processing | ⚪︎ | [⚪︎](https://fx.wtf/getting-started#printing) | × | vy uses `:print` and fx uses `P` to output the selected value and exit. When stdout is redirected, jless does not start the TUI: it formats the entire JSON input or outputs YAML unchanged. |
| Specify an output file in the TUI and continue browsing | ⚪︎ | [×](https://github.com/antonmedv/fx/issues/405) | × | vy uses `:write <file-path> [@<document-index> <path>]`. This differs from fx's CLI `save` or redirection at startup. |
| Save values or subtrees from query results or *focus* | ⚪︎ | [×](https://github.com/antonmedv/fx/issues/401) | [×](https://github.com/PaulJuliusMartinez/jless/issues/70) | vy supports `:write` / `:print` / `:copy` in *jaq* input and results and in *focus*. The compared fx and jless releases have no TUI query result view or equivalent of *focus*. |
| Separate the browsing interface from stdout data | ⚪︎ | [⚪︎](https://fx.wtf/getting-started#printing) | × | vy and fx render the TUI to stderr and output selected values to stdout. jless uses stdout for both the TUI and on-screen output from its `p` commands. |

## History

| Feature | vy | fx | jless | Notes |
| --- | :---: | :---: | :---: | --- |
| Recall and edit search history | ⚪︎ | △ | [×](https://github.com/PaulJuliusMartinez/jless/issues/132) | vy uses `↑` / `↓` while entering a `/` search to select history and saves it across sessions. fx can edit the most recent search with `/` in the same session, but cannot browse multiple entries or persist history. |
| List and search persistent command and query history | ⚪︎ | [×](https://github.com/antonmedv/fx/issues/332) | × | `:hstr` lists previous commands and *jaq* queries, newest first. Press `Tab` to enter the filter editor and search by substring as you type. |
| Recall a command from history for editing | ⚪︎ | × | × | Select a `:hstr` result with `Enter` or a click to restore it to the calling command editor. You can change paths or conditions before running it. |

## Completion

| Feature | vy | fx | jless | Notes |
| --- | :---: | :---: | :---: | --- |
| Path navigation and completion with a document index | ⚪︎ | × | [×](https://github.com/PaulJuliusMartinez/jless/issues/173) | vy specifies documents as in `:goto @1 .metadata.name` and completes document indices with `Tab`. fx path navigation targets the current document and has no document index selection or completion. |
| Fuzzy completion of individual path segments, including earlier segments | ⚪︎ | △ | [×](https://github.com/PaulJuliusMartinez/jless/issues/173) | vy completes the path segment at the cursor using fuzzy matching and can edit earlier segments. fx's `.` fuzzy-matches the final segment of the entered path, while `@` fuzzy-searches the entire path. |
| Shared path completion for navigation, *focus*, preview, and output | ⚪︎ | [×](https://fx.wtf/key-bindings) | [×](https://jless.io/user-guide#commands) | *goto*, *focus*, *preview*, *print*, and *write* share document index / path completion with `Tab`. Keys containing spaces or other special characters use quoted bracket notation. |
| Output file and directory completion | ⚪︎ | [×](https://github.com/antonmedv/fx/issues/405) | × | The first argument of `:write` completes the destination. Subsequent arguments switch to document index and data path completion. |

## Configuration

| Feature | vy | fx | jless | Notes |
| --- | :---: | :---: | :---: | --- |
| Persist display settings | ⚪︎ | [⚪︎](https://fx.wtf/configuration) | [×](https://github.com/PaulJuliusMartinez/jless/issues/174) | vy saves settings in TOML. fx environment variables can be persisted in shell configuration. jless cannot save or load a configuration file. |
| Customize keybindings | ⚪︎ | × | [×](https://github.com/PaulJuliusMartinez/jless/issues/109) | vy supports customization in TOML. The compared fx and jless releases do not support user-defined keybindings. |
| Inspect all settings or individual values after startup | ⚪︎ | × | × | vy provides `:config view` / `:config get <key>`. fx and jless have no command to query current setting values. |
| Complete setting keys and values and display current values | ⚪︎ | × | × | `:config get/set` completes keys. Values such as booleans and display modes are also completed, and the target setting's current value is shown while entering set commands. |
| Save and immediately apply display and interaction settings | ⚪︎ | [△](https://fx.wtf/configuration#fx-line-numbers) | [△](https://jless.io/user-guide#line-numbers) | vy validates, saves, and applies changes with `:config set <key> <value>`. fx can change some display options with `z` or `s`, and jless with commands such as `:set number`, but those changes are not saved. |
| Edit settings externally and return to browsing | ⚪︎ | × | [×](https://github.com/PaulJuliusMartinez/jless/issues/174) | vy opens settings with `:config edit` and reloads them on return. fx's `v` opens the input file being browsed; it does not edit or reload configuration. |
| Separate colors, indentation, and display settings for JSON / YAML | ⚪︎ | [△](https://fx.wtf/configuration) | [△](https://jless.io/user-guide#commands) | vy configures indentation, colors, wrapping, line numbers, and child counts per format. fx sets themes and indentation through environment variables. jless supports some display adjustments. |
| Assign multiple keys or disable bindings per view and action | ⚪︎ | × | [×](https://github.com/PaulJuliusMartinez/jless/issues/109) | Shared settings cover document navigation and text editing, while view-specific settings cover actions such as returning from *focus* or copying in *preview*. Use arrays of keys or empty arrays to customize them. |
| Help and action hints reflect customized keybindings | ⚪︎ | × | [×](https://github.com/PaulJuliusMartinez/jless/issues/109) | vy's `:help` and action hints reflect customized bindings. fx and jless provide help but do not support user-defined keybindings. |
