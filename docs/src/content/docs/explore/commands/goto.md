---
title: "goto"
description: "Navigate by document index and path, and complete paths with Tab."
sidebar:
  order: 1
---

Jump directly to a node without opening each level of the hierarchy.

![Complete a goto path and navigate by document index in a YAML stream with multiple documents](/demos/explore/commands/goto.gif)

## Specify a document index and path

```text
:goto @0 .items[0].name
:goto @0 .
```

`@0` is the first document. Document indices and array indices both start at 0. `.` represents the document root.

Use `@1` for the second document in JSON Lines or a YAML stream with multiple documents.

```text
:goto @1 .metadata.name
```

*goto* requires both a document index and a path. You cannot omit the document index as in `:goto .items`.

## Path notation and completion

| Target | Example path |
| --- | --- |
| Object key | `.items` |
| Array element | `.items[0]` |
| Nested key | `.items[0].status` |
| Key containing spaces or dots | `.items[0]["first name"]`, `["a.b"]` |

Press `Tab` to complete document indices or child elements at the current path level. Each path segment supports fuzzy matching, so you can type part of a name to find candidates. You can also move the cursor back to an earlier segment and complete it again.

The same document index and path syntax is used by [*focus*](focus.md), [*preview*](preview.md), [*print*](print.md), and [*write*](write.md).

## Scope in each view

The input view targets all input documents. Within *focus*, the opened subtree becomes a single document, `@0 .`. In *jaq* / *focus* side-by-side mode, document indices and paths refer to the active pane.

Use `:copy path` to copy the selected path.
