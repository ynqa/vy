---
title: "Theme settings"
description: "Configure styles for messages, the editor cursor, and grep matches."
sidebar:
  order: 3
  label: "Theme"
---

Use `theme.*` to style messages, the editor cursor, and grep matches in *flatten*. See [Display settings](display.md) for JSON and YAML colors and display options, and the [config command](../../explore/commands/config.md) to change settings.

The tables show values from the embedded configuration used to create a new file.

## theme.feedback

| Key | Embedded default | Description |
| --- | --- | --- |
| `theme.feedback.label_style` | `"attr=bold"` | Feedback labels |
| `theme.feedback.normal_style` | `""` | Normal messages |
| `theme.feedback.loading_style` | `"fg=yellow"` | Loading messages |
| `theme.feedback.success_style` | `"fg=green"` | Success messages |
| `theme.feedback.warning_style` | `"fg=yellow"` | Warning messages |
| `theme.feedback.error_style` | `"fg=red"` | Error messages |
| `theme.feedback.hint_style` | `"fg=darkgrey"` | Action hints |

## theme.editor

| Key | Embedded default | Description |
| --- | --- | --- |
| `theme.editor.active_char_style` | `"attr=reverse"` | Character under the editor cursor |

## theme.flatten

| Key | Embedded default | Description |
| --- | --- | --- |
| `theme.flatten.match_style` | `"fg=yellow,attr=bold"` | Search matches in *flatten* |

## Style notation

Use `fg=<color>,bg=<color>,ul=<color>,attr=<attribute>` for `*_style` values. Include only the properties you need; an empty string specifies no style.

```toml
[theme.flatten]
match_style = "fg=yellow,attr=bold"

[json]
key_style = "fg=red,bg=#112233,attr=bold|italic"
```

`fg` sets the foreground color, `bg` the background color, `ul` the underline color, and `attr` the attributes. Named ANSI colors follow the terminal palette. `active_item_attribute` / `inactive_item_attribute` take attribute names rather than full style strings.
