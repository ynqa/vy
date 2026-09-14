---
title: "Combining focus and jaq"
description: "Transform a subtree with jaq, or explore part of a jaq result with focus."
---

Use *focus* → *jaq* to transform a subtree, or *jaq* → *focus* to explore part of a query result.

## *focus* → *jaq*: Transform a subtree

![Run jaq on a record opened with focus to extract its name and duration in seconds](/demos/explore/focus-and-jaq/focus-jaq.gif)

The subtree opened with *focus* becomes the input for the next *jaq* query. In this demo, we open the first record in the `items` array and extract its name and duration converted to seconds.

```text
:focus @0 .items[0]
:jaq {name, seconds: (.duration_ms / 1000)}
```

The opened record becomes the root, so the query can reference `.name` and `.duration_ms` directly. Press `Esc` while browsing to return to the *focus* view showing the original subtree.

## *jaq* → *focus*: Explore part of a result

![Extract failed records with jaq and open a result with focus to explore it](/demos/explore/focus-and-jaq/jaq-focus.gif)

Select a node in the *jaq* results and run `:focus` to explore its subtree in a separate view. In this demo, we extract records whose `status` is `failed`, then open one of those records.

```text
:jaq .items[] | select(.status == "failed")
:focus
```

In side-by-side mode, press `Tab` to switch to the result pane first. Within *focus*, the opened subtree becomes `@0 .`, and navigation and search are scoped to it. Press `Esc` while browsing to return to the original *jaq* results.

See [*focus*](commands/focus.md) and [*jaq*](commands/jaq.md) for the basics of each command.
