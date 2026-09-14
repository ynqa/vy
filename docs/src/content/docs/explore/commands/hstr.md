---
title: "hstr"
description: "Filter command and jaq query history, then recall and edit an entry."
sidebar:
  order: 9
---

Find previous commands and queries, edit the parts you need, and run them again.

![Filter hstr history, recall a command, and edit it](/demos/explore/commands/hstr.gif)

## Recall a command or query

```text
:hstr
```

Lists previous commands and *jaq* queries, newest first.

1. Press `Tab` to move to the filter editor.
2. Enter a string to find, such as `jaq` or `items`.
3. Press `Tab` to return to the list, then select an entry with `j` / `k` or the arrow keys.
4. Press `Enter` or click to restore it to the original command editor.
5. Edit it if needed, then press `Enter` to run it.

Filtering updates as you type and uses case-insensitive substring matching, not regular expressions. Selecting a history entry does not execute the command.

`:hstr` takes no arguments. Press `Esc` to return to browsing.

## Storage and sharing

History is shared across the input, *focus*, and *jaq* views, with up to 1,000 entries kept across sessions. It is stored in `vy/history.jsonl` under the operating system's local data directory. Consecutive identical commands are stored as a single entry. `:hstr` itself is not added to history.

Expressions entered with `/` are saved in a separate [search history](../search.md#search-history). Grep expressions in *flatten* are not included in this persistent history.
