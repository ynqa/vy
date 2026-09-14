use std::{fs, path::Path};

use crate::command::{CompletionCandidate, CompletionRequest, CompletionState};

use super::{decode, quote};

/// Cursor and replacement ranges use character offsets within the file argument.
pub(crate) fn complete(input: &str, cursor: usize) -> CompletionState {
    let input = if decode(input.trim_end(), false).is_some() {
        input.trim_end()
    } else {
        // Whitespace inside an unfinished quote is part of the filename query.
        input
    };
    if cursor > input.chars().count() {
        return CompletionState::None;
    }
    let prefix = input.chars().take(cursor).collect::<String>();
    let Some(prefix) = decode(&prefix, true) else {
        return CompletionState::None;
    };
    let (parent, query) = prefix
        .char_indices()
        .rfind(|(_, c)| std::path::is_separator(*c))
        .map_or(("", prefix.as_str()), |(offset, c)| {
            prefix.split_at(offset + c.len_utf8())
        });
    let directory = if parent.is_empty() {
        Path::new(".")
    } else {
        Path::new(parent)
    };
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return CompletionState::None,
        Err(error) => {
            return CompletionState::Failed(
                format!("cannot list {}: {error}", directory.display()).into(),
            );
        }
    };
    let mut candidates = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => return CompletionState::Failed(error.to_string().into()),
        };
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !name.starts_with(query) || (name.starts_with('.') && !query.starts_with('.')) {
            continue;
        }
        let suffix = if entry.path().is_dir() { "/" } else { "" };
        let mut replacement = quote(&format!("{parent}{name}{suffix}"));
        // Keep the cursor inside a quoted directory so the next filename can be typed.
        if !suffix.is_empty() && replacement.starts_with('"') {
            replacement.pop();
        }
        // Do not keep offering a file that has already been accepted.
        if replacement != input {
            candidates.push(CompletionCandidate {
                display: format!("{name}{suffix}"),
                replacement,
            });
        }
    }
    candidates.sort_by(|a, b| a.display.cmp(&b.display));
    candidates.truncate(100);
    CompletionState::Ready(CompletionRequest {
        replacement: 0..input.chars().count(),
        candidates,
    })
}
