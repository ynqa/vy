# Changelog

Notable changes to vy, newest first. Downloads are available on [GitHub Releases](https://github.com/ynqa/vy/releases).

## Unreleased

### Documentation

- Add a changelog to the documentation site.
- Add a demo GIF to the README.

## [0.1.0](https://github.com/ynqa/vy/releases/tag/v0.1.0) - 2026-09-14

Initial release.

### Added

- Explore JSON, JSON Lines / NDJSON, and YAML from files or standard input, including multi-document YAML.
- Navigate, expand, and collapse trees with the keyboard, and jump to paths with `:goto` and path completion.
- Search with regular expressions and recall search history.
- Open subtrees with `:focus`, and list and filter paths and values with `:flatten`.
- Run jq-compatible queries with `:jaq`, edit expressions, and compare input with results.
- Preview strings, copy paths and values to the clipboard, print to standard output, and save data to a file.
- Recall command history with `:hstr` and look up controls with `:help`.
- Configure display settings, colors, and keybindings in TOML, with a JSON Schema for editor support.
- Provide prebuilt binaries for macOS, Linux, and Windows, plus shell and Homebrew installers. The Cargo package is named `vy-cli`; the command is `vy`.
