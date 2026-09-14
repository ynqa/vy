---
title: "Navigation"
description: "Move through trees, expand and collapse nodes, navigate by page, and adjust the display."
sidebar:
  order: 1
---

Use the keyboard to navigate the tree and adjust the display. The keys below are the defaults.

## Move up and down

Use `j` / `k` or `↓` / `↑` to move to the next or previous visible item.

![Move up and down](/demos/explore/navigation/move.gif)

## Move to a parent or child

- `l` / `→` expands a collapsed node or moves to its first child if it is already expanded.
- `h` / `←` collapses an expanded node or moves to its parent if it is collapsed or has no children.
- `H` moves to the parent without changing the expansion state.

![Move to a parent or child](/demos/explore/navigation/parent-child.gif)

## Move between siblings

Use `J` / `K` to move to the next or previous sibling without stepping through each descendant.

![Move between siblings](/demos/explore/navigation/siblings.gif)

## Expand or collapse the selected node

Use `Space` / `Enter` to expand or collapse the selected array or object.

![Expand or collapse the selected node](/demos/explore/navigation/toggle.gif)

## Expand or collapse everything

Press `e` to expand everything or `E` to collapse everything. Depending on your settings, collapsed containers display the number of immediate children.

![Expand or collapse everything](/demos/explore/navigation/expand-collapse.gif)

## Move to the beginning or end

Use `g` / `Home` to move to the beginning or `G` / `End` to move to the end.

![Move to the beginning or end](/demos/explore/navigation/head-tail.gif)

## Move by page

- `PageDown` / `Ctrl+F` moves down one page.
- `PageUp` / `Ctrl+B` moves up one page.
- `Ctrl+D` moves down half a page.
- `Ctrl+U` moves up half a page.

![Move by page](/demos/explore/navigation/paging.gif)

## Toggle line wrapping

Press `z` to switch between wrapping and truncating long lines. Changing the display does not change the original values.

![Toggle line wrapping](/demos/explore/navigation/overflow.gif)

## Toggle line numbers

Press `Alt+N` to show or hide line numbers.

![Toggle line numbers](/demos/explore/navigation/line-numbers.gif)

See [*goto*](commands/goto.md) for path navigation and completion, and [*config*](commands/config.md) for saving display settings.
