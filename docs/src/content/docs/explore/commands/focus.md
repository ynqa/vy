---
title: "focus"
description: "Open a subtree as a temporary root to narrow the scope of searches and queries."
sidebar:
  order: 2
---

Open a subtree in a separate view to narrow the scope of navigation, search, and queries.

## Open a subtree

![Open the first element of the items array with focus and return to the original view with Esc](/demos/explore/commands/focus-open.gif)

Run `:focus` on the selected node to open a view rooted at that subtree. You can also specify the target by path.

```text
:focus @0 .items[0]
```

This makes the first element of the `items` array the root. The original document index and path appear in the status area.

## Open nested views

![Open the first element from the items array in a nested focus view and return one level at a time with Esc](/demos/explore/commands/focus-nested.gif)

You can run `:focus` again within *focus*. Press `Esc` while browsing to return one level, preserving the calling view's selection, scroll position, and folding state.

Pressing `Esc` while editing a command or search closes the editor and returns to browsing. Use `:quit` to exit vy entirely.

## Display the input and subtree side by side

![Display the input and subtree side by side, switch panes with Tab, and explore each pane](/demos/explore/commands/focus-compare.gif)

Press `s` to display the input and subtree side by side. Switch panes with `Tab` or the mouse. Commands, search, and completion target the active pane. Press `s` again to show only the subtree.

## Work relative to the subtree

Within *focus*, the entire subtree is treated as a single document, `@0 .`. You do not need to repeat the original document's path. These examples assume you opened an object with `name` and `message` fields.

| Example | Target |
| --- | --- |
| `:goto @0 .name` | Move to `name` directly under the root |
| `/` → `failed` → `Enter` | Search keys and values within the subtree |
| `:flatten` | List paths and values within the subtree |
| `:jaq .name` | Get `name` using the subtree as input |
| `:preview @0 .message` | Open `message` directly under the root |
| `:write ./selected.json @0 .` | Save the entire opened subtree to a new file |

Path completion is limited to this scope. The path returned by *copy path* is also relative to the *focus* root.

See [Combining focus and jaq](../focus-and-jaq.md) for examples of using the two commands together.
