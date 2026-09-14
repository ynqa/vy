---
title: "print"
description: "Write a value or subtree to standard output and exit vy."
sidebar:
  order: 7
---

Write a value or subtree to standard output and exit vy.

![Export selected data with print](/demos/explore/commands/print.gif)

## Send data to standard output

```sh
vy data.json > selected.json
```

Run `:print` while browsing to output the selected value or subtree and exit. You can also specify the target by path:

```text
:print @0 .items[0]
```

The TUI renders to standard error. vy restores the terminal before writing data to standard output. Exiting without `:print` produces no data output.

Available in the input, *focus*, and *jaq* views. In side-by-side mode, it targets the active pane.

## Output format

*print* and *write* use the target view's JSON or YAML format and append a trailing newline. The destination file extension does not convert the format.

JSON output preserves the target's key order, number notation, and formatting. Results created by *jaq*, however, are separate data from the original source before the query. YAML is serialized again, so original comments and formatting are not preserved. Child count annotations shown on screen are excluded from output.

Use `:write` to [save to a file and continue browsing](write.md).
