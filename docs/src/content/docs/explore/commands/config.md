---
title: "config"
description: "Change colors, display settings, and keybindings with TOML and the config command."
sidebar:
  order: 10
---

Colors, display settings, and keybindings are managed in TOML. Use `:config` while browsing to change them and apply the changes immediately.

## Configuration file

vy uses the file specified by `--config` / `-c` if provided. Otherwise, it uses `vy/config.toml` under the operating system's configuration directory.

| OS | Typical location |
| --- | --- |
| Linux | `~/.config/vy/config.toml` (under `XDG_CONFIG_HOME` if set) |
| macOS | `~/Library/Application Support/vy/config.toml` |
| Windows | `%APPDATA%\vy\config.toml` |

If the default file does not exist, vy creates it from the embedded configuration, including comments. Create files specified with `--config` in advance.

The configuration file includes a comment referencing the published JSON Schema for the corresponding release. See [Editor validation](../../reference/settings/schema.md) to enable completion and validation in editors such as VS Code.

```sh
vy --config ./config.toml data.json
```

If reading, parsing, or initial creation fails, vy starts with the embedded defaults and displays an error in the feedback area.

## Configuration format version

Set `version = 1` at the top level of the configuration file. This number identifies configuration format compatibility and is independent of the vy release version.

```toml
version = 1

[json]
[yaml]
```

The currently supported format is `1`. Existing files without `version` are also read as format `1` and are not rewritten automatically. Unsupported versions and non-integer values produce an error. At startup, vy falls back to the default configuration. `:config set` does not save invalid changes, and reloading after `:config edit` does not apply invalid settings.

## Display the configuration file

`:config view` displays the entire configuration file in a dedicated view. Scroll with `j` / `k` or page navigation, and press `Esc` to return.

![Display and scroll through the full configuration file with config view](/demos/explore/commands/config-view.gif)

## Inspect a setting

`:config get <key>` displays the value from the file. Press `Tab` to complete keys.

![Complete a key and display its current value with config get](/demos/explore/commands/config-get.gif)

## Change a setting

`:config set <key> <value>` validates the change, saves it while preserving TOML comments, and applies it to the display. Folding state is also preserved. `Tab` completes keys as well as values such as booleans and display modes. The target setting's current value is shown while you type.

![Change indentation and key colors with config set](/demos/explore/commands/config-set.gif)

## Edit in an external editor

`:config edit` opens the configuration file using `$VISUAL`, falling back to `$EDITOR`. After the editor exits, vy reloads the settings and returns to browsing.

![Open an editor with config edit, change indentation, and return to browsing](/demos/explore/commands/config-edit.gif)

## Change the display

JSON and YAML have independent display settings.

```text
:config set json.overflow_mode Truncate
:config set yaml.indent 4
:config set json.key_style fg=cyan,attr=bold
:config set theme.flatten.match_style fg=yellow,attr=bold
```

Unlike temporary display changes made with `z` or `Alt+N`, *config set* saves changes for future sessions.

## Show child counts

```text
:config set json.show_child_count true
:config set yaml.show_child_count true
```

Shows annotations such as `[…] (3 items)` on collapsed arrays and `{…} (2 keys)` on collapsed objects. Empty containers also display counts.

This is enabled in the embedded configuration. If an existing file omits this key, counts are hidden; use the commands above to enable them. Change the annotation colors with `json.child_count_style` / `yaml.child_count_style`.

## Change keybindings

Assign a key or an array of keys to an action in the configuration file. An empty array disables the binding; omitted actions use their defaults.

```toml
[keybinds.view.browse.input]
exit = ["q", "Esc"]

[keybinds.view.browse]
older_search_history = ["Ctrl+P"]
newer_search_history = ["Ctrl+N"]
```

You can also save a binding with a command.

```text
:config set keybinds.view.browse.input.exit ["q", "Esc"]
```

Use `keybinds.component.*` for shared navigation and text editing, and `keybinds.view.*` for view-specific actions. Updated bindings also appear in `:help` and on-screen hints.

See [Display settings](../../reference/settings/display.md) and [Theme settings](../../reference/settings/theme.md) for settings and defaults, and the [keybinding reference](../../reference/settings/keybindings.md) for action names.
