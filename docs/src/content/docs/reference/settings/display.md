---
title: "Display settings"
description: "Settings for scrolling and displaying JSON and YAML."
sidebar:
  order: 2
  label: "Display"
---

Configure mouse wheel scrolling, JSON and YAML indentation, colors, line wrapping, line numbers, and more. See the [config command](../../explore/commands/config.md) to change settings, and [Style notation](theme.md#style-notation) to specify colors and attributes.

The tables show values from the embedded configuration used to create a new file. These may differ from the values used when an existing file omits a setting. In particular, omitting `show_child_count` defaults to `false`.

## Scrolling

| Key | Embedded default | Description |
| --- | --- | --- |
| `scroll_lines` | `3` | Lines moved per mouse wheel event. Must be at least 1 |

## json

| Key | Embedded default | Description |
| --- | --- | --- |
| `json.indent` | `2` | Number of spaces per indentation level |
| `json.curly_brackets_style` | `"attr=bold"` | Curly bracket style |
| `json.square_brackets_style` | `"attr=bold"` | Square bracket style |
| `json.key_style` | `"fg=darkblue"` | Key style |
| `json.string_value_style` | `"fg=darkgreen"` | String style |
| `json.number_value_style` | `""` | Number style |
| `json.boolean_value_style` | `""` | Boolean style |
| `json.null_value_style` | `"fg=darkgrey"` | Null style |
| `json.active_item_attribute` | `"bold"` | Attribute for the active item |
| `json.inactive_item_attribute` | `"dim"` | Attribute for inactive items |
| `json.overflow_mode` | `"Wrap"` | How content wider than the screen is displayed: Wrap / Truncate |
| `json.show_line_numbers` | `true` | Show line numbers |
| `json.show_child_count` | `true` | Show immediate child counts on collapsed and empty containers |
| `json.child_count_style` | `"fg=darkgrey"` | Child count annotation style |
| `json.lines` | Unset | Maximum lines to render. When unset, no configured line limit applies |

## yaml

| Key | Embedded default | Description |
| --- | --- | --- |
| `yaml.indent` | `2` | Number of spaces per indentation level |
| `yaml.map_style` | `"attr=bold"` | Mapping style |
| `yaml.sequence_style` | `"attr=bold"` | Sequence style |
| `yaml.key_style` | `"fg=darkblue"` | Key style |
| `yaml.tag_style` | `"fg=darkyellow"` | Tag style |
| `yaml.string_style` | `"fg=darkgreen"` | String style |
| `yaml.number_style` | `""` | Number style |
| `yaml.boolean_style` | `""` | Boolean style |
| `yaml.null_style` | `"fg=darkgrey"` | Null style |
| `yaml.active_item_attribute` | `"bold"` | Attribute for the active item |
| `yaml.inactive_item_attribute` | `"dim"` | Attribute for inactive items |
| `yaml.overflow_mode` | `"Wrap"` | How content wider than the screen is displayed: Wrap / Truncate |
| `yaml.show_line_numbers` | `true` | Show line numbers |
| `yaml.show_child_count` | `true` | Show immediate child counts on collapsed and empty containers |
| `yaml.child_count_style` | `"fg=darkgrey"` | Child count annotation style |
| `yaml.lines` | Unset | Maximum lines to render. When unset, no configured line limit applies |

## Example configuration

```toml
# Set at the top level, outside any section
scroll_lines = 5

[json]
indent = 4
show_line_numbers = false

[yaml]
overflow_mode = "Truncate"
```

You can also change settings while vy is running with `:config set`.

![Complete setting keys and values, change indentation and line numbers, and display the configuration file](/demos/reference/settings.gif)

```text
:config set json.indent 4
:config set json.show_line_numbers false
```
