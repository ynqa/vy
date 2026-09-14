mod complete;

use std::path::PathBuf;

pub(crate) use complete::complete;

/// Splits the first file argument, retaining whitespace in the remainder.
/// Unfinished quotes belong to the file argument during completion.
pub(crate) fn split_prefix(input: &str) -> (&str, &str) {
    let input = input.trim_start();
    let mut quote = input.chars().next().filter(|c| matches!(c, '\'' | '"'));
    let mut escaped = false;
    for (offset, character) in input.char_indices() {
        if offset == 0 && quote.is_some() {
            continue;
        }
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if delimiter == '"' && character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if character.is_whitespace() {
            return input.split_at(offset);
        }
    }
    (input, "")
}

/// File names are literal, or enclosed in single quotes / JSON-style double quotes.
/// Shell expansion is deliberately not performed.
pub(crate) fn parse(input: &str) -> anyhow::Result<PathBuf> {
    let input = input.trim();
    let decoded = decode(input, false)
        .ok_or_else(|| anyhow::anyhow!("requires one file path; quote names containing spaces"))?;
    anyhow::ensure!(!decoded.is_empty(), "requires a file path");
    Ok(PathBuf::from(decoded))
}

fn decode(input: &str, incomplete: bool) -> Option<String> {
    if input.starts_with('"') {
        serde_json::from_str::<String>(input).ok().or_else(|| {
            incomplete
                .then(|| serde_json::from_str::<String>(&format!("{input}\"")).ok())
                .flatten()
        })
    } else if let Some(rest) = input.strip_prefix('\'') {
        let rest = match rest.strip_suffix('\'') {
            Some(rest) => rest,
            None if incomplete => rest,
            None => return None,
        };
        (!rest.contains('\'')).then(|| rest.to_owned())
    } else {
        (!input
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '\'' | '"')))
        .then(|| input.to_owned())
    }
}

fn quote(input: &str) -> String {
    if input
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '\'' | '"' | '\\'))
    {
        serde_json::to_string(input).expect("strings can be serialized")
    } else {
        input.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_only_after_the_complete_file_argument() {
        for file in [
            r#""a\" b.json""#,
            r#""a\\ b.json""#,
            "'a b.json'",
            "日本語.json",
        ] {
            let input = format!("  {file}\u{3000}@0 .items");
            assert_eq!(split_prefix(&input), (file, "\u{3000}@0 .items"));
            assert!(parse(file).is_ok());
        }
        for unfinished in [r#""a b.json"#, "'a b.json"] {
            assert_eq!(split_prefix(unfinished), (unfinished, ""));
        }
    }

    #[test]
    fn parses_literal_and_quoted_names_without_shell_expansion() {
        for name in [
            "./new.json",
            "./日本語 file.json",
            "a\\b\"c.json",
            "$(command).json",
        ] {
            assert_eq!(parse(&quote(name)).unwrap(), PathBuf::from(name));
        }
        assert_eq!(
            parse("'./my file.json'").unwrap(),
            PathBuf::from("./my file.json")
        );
        for input in ["", "\"\"", "'unclosed", "\"unclosed", "a.json b.json"] {
            assert!(parse(input).is_err(), "{input}");
        }
    }
}
