use std::ops::Range;

use crate::{
    command::{CompletionCandidate, CompletionRequest},
    config,
};

const SUBCOMMANDS: &[&str] = &["edit", "get", "set", "view"];

/// Completes arguments using character offsets relative to the argument string.
pub(crate) fn complete(arguments: &str, cursor: usize) -> Option<CompletionRequest> {
    let characters = arguments.chars().collect::<Vec<_>>();
    let prefix = characters.get(..cursor)?.iter().collect::<String>();
    let token_end = cursor
        + characters[cursor..]
            .iter()
            .take_while(|character| !character.is_whitespace())
            .count();
    let raw_arguments = prefix.as_str();
    let arguments = raw_arguments.trim_start();
    let subcommand_start = raw_arguments.chars().count() - arguments.chars().count();
    let Some((subcommand, remainder)) = arguments.split_once(char::is_whitespace) else {
        return candidates(
            subcommand_start..token_end,
            arguments,
            SUBCOMMANDS.iter().copied(),
            true,
        );
    };

    if !matches!(subcommand, "get" | "set") {
        return None;
    }
    let remainder = remainder.trim_start();
    let key_start = cursor.saturating_sub(remainder.chars().count());
    if remainder.chars().any(char::is_whitespace) {
        return complete_value(subcommand, remainder, cursor, token_end);
    }

    candidates(
        key_start..token_end,
        remainder,
        config::keys().iter().map(String::as_str),
        subcommand == "set",
    )
}

pub(crate) fn set_target(input: &str) -> Option<&str> {
    let mut words = input.split_whitespace();
    if words.next()? != "config" || words.next()? != "set" {
        return None;
    }
    let key = words.next()?;
    config::keys()
        .iter()
        .any(|known| known == key)
        .then_some(key)
}

fn complete_value(
    subcommand: &str,
    remainder: &str,
    cursor: usize,
    token_end: usize,
) -> Option<CompletionRequest> {
    if subcommand != "set" {
        return None;
    }
    let (key, value) = remainder.split_once(char::is_whitespace)?;
    let value = value.trim_start();
    let value_start = cursor.saturating_sub(value.chars().count());
    let values: &[&str] = match key {
        "json.overflow_mode" | "yaml.overflow_mode" => &["Truncate", "Wrap"],
        key if key.ends_with("show_line_numbers") || key.ends_with("show_child_count") => {
            &["false", "true"]
        }
        _ => return None,
    };
    candidates(value_start..token_end, value, values.iter().copied(), false)
}

fn candidates<'a>(
    replacement: Range<usize>,
    query: &str,
    values: impl Iterator<Item = &'a str>,
    trailing_space: bool,
) -> Option<CompletionRequest> {
    let candidates = values
        .filter(|value| value.starts_with(query) && *value != query)
        .map(|value| CompletionCandidate {
            display: value.to_owned(),
            replacement: format!("{value}{}", if trailing_space { " " } else { "" }),
        })
        .collect::<Vec<_>>();
    (!candidates.is_empty()).then_some(CompletionRequest {
        replacement,
        candidates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_at_end(input: &str) -> CompletionRequest {
        complete(input, input.chars().count()).expect("completion request")
    }

    #[test]
    fn completes_subcommands() {
        let request = complete_at_end("g");
        assert_eq!(request.replacement, 0..1);
        assert_eq!(request.candidates[0].display, "get");
        assert_eq!(request.candidates[0].replacement, "get ");
    }

    #[test]
    fn completes_get_and_set_keys() {
        for input in ["get json.ind", "set json.ind"] {
            let request = complete_at_end(input);
            assert_eq!(request.candidates[0].display, "json.indent");
        }
        assert_eq!(
            complete_at_end("set json.ind").candidates[0].replacement,
            "json.indent "
        );
    }

    #[test]
    fn completes_known_values() {
        let request = complete_at_end("set json.overflow_mode T");
        assert_eq!(request.candidates[0].display, "Truncate");
        let request = complete_at_end("set json.show_line_numbers ");
        assert_eq!(request.candidates.len(), 2);
    }

    #[test]
    fn completes_child_count_settings_and_boolean_values() {
        for format in ["json", "yaml"] {
            let request = complete_at_end(&format!("set {format}.show_child_"));
            assert_eq!(
                request.candidates[0].replacement,
                format!("{format}.show_child_count ")
            );
            let request = complete_at_end(&format!("set {format}.child_count_"));
            assert_eq!(
                request.candidates[0].replacement,
                format!("{format}.child_count_style ")
            );
            let request = complete_at_end(&format!("set {format}.show_child_count "));
            assert_eq!(
                request
                    .candidates
                    .iter()
                    .map(|candidate| candidate.replacement.as_str())
                    .collect::<Vec<_>>(),
                ["false", "true"]
            );
            let request = complete_at_end(&format!("set {format}.show_child_count t"));
            assert_eq!(request.candidates[0].replacement, "true");
        }
    }

    #[test]
    fn replaces_the_complete_token_when_the_cursor_is_in_the_middle() {
        let request = complete("get json.indXXX", 12).unwrap();
        assert_eq!(request.replacement, 4..15);
        assert_eq!(request.candidates[0].replacement, "json.indent");
    }

    #[test]
    fn identifies_a_complete_set_target() {
        assert_eq!(set_target("config set json.indent"), Some("json.indent"));
        assert_eq!(set_target("config set json.indent 4"), Some("json.indent"));
        assert_eq!(set_target("config set json.ind"), None);
        assert_eq!(set_target("config get json.indent"), None);
    }
}
