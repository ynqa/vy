---
title: "Editor validation"
description: "Use the published schema to enable configuration completion and error reporting in editors such as VS Code."
sidebar:
  order: 4
---

Specify a JSON Schema URL at the top of your configuration file to enable completion and validation in supported editors. The distributed `default.toml` and configuration files created by vy include this directive.

## Enable in VS Code

Install [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml) and open the configuration file. For existing configurations, add a comment at the top matching your vy version.

```toml
#:schema https://github.com/ynqa/vy/releases/download/v0.1.0/config.schema.json

version = 1
scroll_lines = 3

[json]
overflow_mode = "Wrap"

[yaml]
```

The schema is distributed as `config.schema.json` with [each vy release](https://github.com/ynqa/vy/releases). Match the URL's tag to your vy version. Your editor fetches it from the URL, so you do not need to keep the schema next to the configuration file.

`#:schema` is a comment directive supported by [Taplo](https://taplo.tamasfe.dev/configuration/directives.html) and [Tombi](https://tombi-toml.github.io/tombi/docs/json-schema/).

## Detected configuration errors

| Example | Problem |
| --- | --- |
| `version = 2` | The currently supported configuration format is `1` |
| `scroll_lines = 0` | Requires an integer of at least 1 |
| `json.indnet = 2` | Unknown setting |
| `json.indent = "two"` | Requires a non-negative integer |
| `json.show_line_numbers = "true"` | Requires a boolean, not a string |
| `json.overflow_mode = "wrap"` | Requires `"Wrap"` or `"Truncate"` |
| `keybinds.view.browse.submit_command = [1]` | Requires a key string or an array of strings |

Errors appear at the relevant location in the editor and in VS Code's Problems panel. The schema does not validate detailed key or style string syntax or detect keybinding conflicts.
