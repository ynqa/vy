---
title: "quit"
description: "Exit vy entirely, regardless of the current view."
sidebar:
  order: 12
---

![Run quit to return to the shell](/demos/explore/commands/quit.gif)

## Exit vy

`:quit` exits vy entirely and returns to the shell. It takes no arguments and is available in the command editor of the input, *focus*, and *jaq* views.

Pressing `Esc` while browsing *focus* or *jaq* returns to the previous view. `:quit` exits all views, including the calling view.

Use [*print*](print.md) to send selected data to standard output when exiting.
