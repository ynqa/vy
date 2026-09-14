---
title: "Command reference"
description: "Startup options, TUI commands, and target syntax for vy."
sidebar:
  order: 1
  label: "Commands"
---

A reference for startup options and commands available while browsing. Open `:help` to look them up during use, and press `Tab` to complete command names.

## Startup options

```text
vy [OPTIONS] [INPUT]
```

| Argument or option | Description |
| --- | --- |
| `INPUT` | Input file. Omit it or use `-` for standard input |
| `-f`, `--format <FORMAT>` | `auto` (default), `json`, or `yaml` |
| `-c`, `--config <CONFIG>` | Configuration file path |
| `-h`, `--help` | Help |
| `-V`, `--version` | Version |

Use `json` for JSON Lines / NDJSON.

## TUI commands

Press `:` in the input, *focus*, or *jaq* view to enter a command. Press `Enter` to run it, `Tab` to complete it, or `Esc` / `Ctrl+C` to close the editor. In the table below, `[...]` denotes optional arguments and `<...>` denotes a value to supply.

`:grep` is available only in the path and value list opened by `:flatten`. Press `:` there to open the grep expression editor.

| Syntax | Alias | Behavior |
| --- | --- | --- |
| `:help` | `:h` | Show help reflecting the current keybindings |
| `:quit` | — | Exit vy entirely |
| `:goto @<document-index> <path>` | — | Move to the target node |
| `:focus [@<document-index> <path>]` | — | Open the target subtree |
| `:flatten` | — | Open a path and value list for the target view |
| `:grep <expr>` | — | Available only in the view opened by `:flatten`. Filter paths and values with regular expressions, using `&` for AND and `\|` for OR |
| `:jaq <expr>` | `:j` | Open results in a new view from the input or *focus* view. Within *jaq*, update the current results |
| `:preview [@<document-index> <path>]` | — | Open a string in a dedicated view |
| `:copy path` | `:cp path` | Copy the selected node's path |
| `:copy value` | `:cp value` | Copy the selected scalar value |
| `:copy subtree` | `:cp subtree` | Copy the selected subtree |
| `:print [@<document-index> <path>]` | — | Write the target to standard output and exit |
| `:write <file-path> [@<document-index> <path>]` | — | Save to a new file and continue browsing |
| `:hstr` | — | Open command and query history |
| `:config view` | — | Display the configuration file |
| `:config get <key>` | — | Display a setting's value from the file |
| `:config set <key> <value>` | — | Validate, save, and apply a setting |
| `:config edit` | — | Edit the configuration in an external editor |

*help*, *quit*, *flatten*, and *hstr* take no arguments. Command completion shows canonical names.

## Specify a target

Document indices start at 0, and `.` denotes the root. A document index is required when specifying a path.

```text
:goto @0 .items[0]
:focus @0 .items[0]
:preview @0 .items[0].message
:write ./api.json @0 .items[0]
```

For *focus*, *preview*, *print*, and *write*, you can omit both the document index and path to target the selected node. *copy* always targets the selected node.

In side-by-side mode, operations target the active pane.
