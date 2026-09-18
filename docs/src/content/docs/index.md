---
title: Overview
description: "Explore and query JSON and YAML, right in your terminal."
---

vy is a terminal viewer for exploring JSON and YAML. Browse trees, search, extract and transform data with [*jaq*](https://github.com/01mf02/jaq), and use *focus* to explore subtrees.

![Navigate with goto, open a subtree with focus, and compare jaq input and results](/demos/index.gif)

Start with [Getting started](getting-started.md) for installation and basic usage.

See the [Changelog](changelog.mdx) for release notes and upcoming changes.

## Guides

| Guide | Topics |
| --- | --- |
| [Navigation](explore/navigation.md) | Move through, expand, and collapse trees with the keyboard |
| [Search](explore/search.md) | Regular expression search and search history |
| [Combining focus and jaq](explore/focus-and-jaq.md) | Transform subtrees and explore query results |
| [*goto*](explore/commands/goto.md) | Paths and completion |
| [*focus*](explore/commands/focus.md) | Explore subtrees |
| [*flatten*](explore/commands/flatten.md) | List and filter paths and values |
| [*jaq*](explore/commands/jaq.md) | Run and edit queries |
| [*preview*](explore/commands/preview.md) | Read strings |
| [*copy*](explore/commands/copy.md) | Copy to the clipboard |
| [*print*](explore/commands/print.md) | Write to standard output and exit |
| [*write*](explore/commands/write.md) | Save to a file and continue browsing |
| [*hstr*](explore/commands/hstr.md) | Recall command history |
| [*config*](explore/commands/config.md) | Configure display, colors, and keybindings |
| [*help*](explore/commands/help.md) | Look up controls |
| [*quit*](explore/commands/quit.md) | Exit vy |

## Reference

[Commands](reference/commands.md) · [Keybindings](reference/settings/keybindings.md) · [Display](reference/settings/display.md) · [Theme](reference/settings/theme.md) · [Feature comparison](reference/comparison.md)

See [Architecture](development/architecture.md) for the internal structure.
