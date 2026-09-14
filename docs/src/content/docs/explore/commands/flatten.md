---
title: "flatten"
description: "List paths and values, then filter them with regular expressions and AND / OR conditions."
sidebar:
  order: 3
---

Find data without navigating the hierarchy by listing and filtering paths and values. Jump from a matching row to its node in the original tree.

![Switch between AND and OR conditions in flatten and jump from a result to the original node](/demos/explore/commands/flatten.gif)

## Read paths and values together

```text
:flatten
```

Lists each node as a row containing its document index, path, and value. Search conditions can include its location in the hierarchy as well as its key or value.

Search using the parent path instead of opening each parent in turn. When opened from *focus*, the list covers only that subtree.

## Enter conditions

1. Press `:` while browsing *flatten*.
2. Enter regular expressions in the grep editor. Combine conditions with `&` for AND or `|` for OR.
3. Press `Enter` to apply the filter.
4. Press `Esc` to close the editor and return to the results.

For example, `status&failed` requires both `status` and `failed` on the same row. Use it to find a value under a specific key.

| Expression | Meaning |
| --- | --- |
| `failed` | Matches the regular expression `failed` |
| `status&failed` | Matches both regular expressions on the same row |
| `failed\|duration_ms` | Matches either regular expression |
| `status&(failed\|ok)` | Matches `status` and either alternative in the regular expression group |

Top-level `|` means OR and `&` means AND. Delimiters inside regular expression parentheses or character classes, and escaped delimiters, are not split. AND applies per row. Use [*jaq*](jaq.md) to combine conditions on fields such as `name` and `status` that appear on separate rows within a record.

Matches are highlighted, and the matching row count / total row count is shown. Change the highlight style with `theme.flatten.match_style`.

## Return to a matching node

Select a result with `j` / `k` or the arrow keys, then press `Enter` or click to jump to its node in the calling tree. From there, you can move to its parent, open *focus*, or copy its value.

Press `:` again to edit the expression. Each new search replaces the previous one. Press `Ctrl+C` to cancel a running search.

Press `Esc` while browsing to return to the calling view. In *flatten*, `:` opens the dedicated grep editor. Return to the original view to use regular commands.
