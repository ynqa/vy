---
title: "Getting started"
description: "Install with Homebrew, the shell installer, or Cargo, open JSON and YAML, and start exploring."
---

<!-- Draft assumes distribution is available. Confirm the Homebrew formula, installer artifact, and crates.io package before publishing. -->

## Installation

### Homebrew

```sh
brew install ynqa/tap/vy
```

### Shell installer

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ynqa/vy/releases/latest/download/vy-cli-installer.sh | sh
```

### Cargo

If you have Rust and Cargo installed, run:

```sh
cargo install vy-cli --locked
```

The Cargo package is named `vy-cli`; the installed command is `vy`.

Add Cargo's installation `bin` directory to your `PATH`. The default location is `~/.cargo/bin`.

### Verify the installation

```sh
vy --version
vy --help
```

## Open a file or pipe

```sh
vy data.json
vy config.yaml
cat data.json | vy
vy --format yaml input.txt
```

`--format` accepts `auto` (the default), `json`, and `yaml`. Automatic detection checks known file extensions first, then infers the format from the beginning of the content. Specify the format explicitly if detection does not give the intended result.

| Input | Behavior |
| --- | --- |
| JSON | Displayed as a JSON tree |
| JSON Lines / NDJSON | Each JSON value is displayed as a separate document. Supports `--format json` |
| YAML | Displayed as a YAML tree |
| YAML separated by `---` | Each document is displayed separately |

vy reads the entire input before opening the interface. It does not support continuously browsing input that never ends, such as `tail -f`.

## First steps

After opening a file, navigate the tree to select the node you want to inspect. Enter commands beginning with a colon inside vy.

![Open JSON, move up and down, expand and collapse nodes, use goto and focus, search, return to the original view, and quit](/demos/getting-started.gif)

1. Move between lines with `j` / `k`, and expand or collapse a node with `Space`.
2. If you know the path, jump directly to it with `:goto`.
3. Run `:focus` to open only the selected subtree.
4. Press `/`, enter a search term or regular expression, and press `Enter` to search.
5. While browsing in *focus*, press `Esc` to return to the original view.
6. Run `:quit` to exit vy entirely.

Press `Tab` to complete command names and paths. Open `:help` whenever you need to look up the controls.

Next, try [Navigation](explore/navigation.md) or [*flatten*](explore/commands/flatten.md).
