---
title: "copy"
description: "Copy the selected node's path, value, or subtree to the clipboard."
sidebar:
  order: 6
---

Copy the selected node to the operating system's clipboard and continue browsing. `:cp` is an alias for `:copy`. To change the target, select a different node in the tree first.

## Copy a path

`:copy path` copies the selected node's path. Within *focus*, paths are relative to the opened subtree.

![Copy the selected node's path with copy path](/demos/explore/commands/copy-path.gif)

## Copy a scalar value

`:copy value` copies the selected scalar value. Use `subtree` for arrays and objects.

![Select a string node and copy its value with copy value](/demos/explore/commands/copy-value.gif)

## Copy a subtree

`:copy subtree` copies the selected node and its descendants.

![Select an object and copy its subtree with copy subtree](/demos/explore/commands/copy-subtree.gif)

Available in the input, *focus*, and *jaq* views. In side-by-side mode, it targets the active pane. OSC52 and external copy commands are not supported.

Use [*preview*](preview.md) to select and copy part of a string.
