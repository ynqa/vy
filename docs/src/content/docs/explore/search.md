---
title: "Search"
description: "Search keys and values with regular expressions, move between matches, and recall search history."
sidebar:
  order: 2
---

Search keys and values using regular expressions. Recall and edit previous expressions from search history.

![Search with a regular expression and move to the next or previous match](/demos/explore/search/matches.gif)

## Search keys and values

Press `/`, enter a regular expression, and press `Enter` to search. You can also enter a plain word.

Search includes descendants of collapsed nodes. It moves to a matching node; press `n` for the next match or `N` for the previous one. After the last match, it wraps to the first.

| Example | Matches |
| --- | --- |
| `failed` | Keys or values containing `failed` |
| `failed\|scheduled` | Keys or values containing either word |
| `(?i)failed` | Keys or values containing `failed`, ignoring case with the `(?i)` regular expression flag |

Search does not run while you type. Press `Enter` to run it or `Esc` / `Ctrl+C` to cancel. Invalid regular expressions and empty expressions produce an error.

Regular search matches each node's own key or value. To filter by the full path, including ancestors, and the value together, use [*flatten*](commands/flatten.md).

## Narrow the search scope

To narrow the search, select the node you want to inspect and run `:focus`. You can also specify a path.

```text
:focus @0 .items[0]
```

Within *focus*, `/` searches only the opened subtree. The same search controls work on *jaq* results. In side-by-side mode, search applies to the active pane.

## Search history

![Browse search expressions, return to a draft, edit an expression, and recall it after restarting](/demos/explore/search/history.gif)

While entering a `/` search, press `↑` to recall an older expression or `↓` for a newer one. Edit the selected expression and press `Enter` to search.

Pressing `↓` past the newest history entry restores the input you had before browsing history. Moving beyond the oldest entry does not wrap around.

Search history is shared across the input, *focus*, and *jaq* views, with up to 1,000 entries saved across sessions. Valid expressions are saved even if there are no matches. Empty expressions, invalid regular expressions, and canceled input are not saved.

History is stored in `vy/history.search.jsonl` under the operating system's local data directory, separately from [command history](commands/hstr.md).

If you want to use the arrow keys to edit multiline expressions, you can change the history bindings.

```toml
[keybinds.view.browse]
older_search_history = ["Ctrl+P"] # Default: ["Up"]
newer_search_history = ["Ctrl+N"] # Default: ["Down"]
```
