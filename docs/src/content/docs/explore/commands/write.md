---
title: "write"
description: "Save a value or subtree to a new file and continue browsing."
sidebar:
  order: 8
---

Save a value or subtree to a new file and continue browsing.

![Export selected data with write](/demos/explore/commands/write.gif)

## Save to a file

```text
:write ./api.json @0 .items[0]
:write ./selected.json
```

A destination is required. Omitting the document index and path saves the selected value or subtree. Press `Tab` to complete the destination; subsequent arguments support document index and data path completion.

Relative paths are resolved from the directory where vy was started. Quote destinations containing spaces.

```text
:write "./export files/api.json" @0 .items[0]
```

Existing files are not overwritten. Create parent directories in advance. Shell variables and `~` are not expanded. If saving fails, an error appears and you can correct the command.

Available in the input, *focus*, and *jaq* views. In side-by-side mode, it targets the active pane.

## Output format

*print* and *write* use the target view's JSON or YAML format and append a trailing newline. The destination file extension does not convert the format.

JSON output preserves the target's key order, number notation, and formatting. Results created by *jaq*, however, are separate data from the original source before the query. YAML is serialized again, so original comments and formatting are not preserved. Child count annotations shown on screen are excluded from output.

Use `:print` to [send data to standard output and exit](print.md).
