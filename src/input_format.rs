use std::path::Path;

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub(crate) enum InputFormat {
    #[default]
    Auto,
    Json,
    Yaml,
}

impl InputFormat {
    /// Resolves `Auto` without parsing the entire input.
    ///
    /// A known file extension is treated as authoritative. Otherwise, only a
    /// bounded prefix of the input is inspected. Since JSON is a subset of
    /// YAML 1.2, content detection is necessarily heuristic.
    pub(crate) fn resolve(self, input: Option<&Path>, data_prefix: &[u8]) -> Self {
        if self != Self::Auto {
            return self;
        }

        format_from_extension(input)
            .or_else(|| format_from_prefix(data_prefix))
            .unwrap_or(Self::Yaml)
    }
}

fn format_from_extension(input: Option<&Path>) -> Option<InputFormat> {
    let extension = input?.extension()?.to_str()?;

    if ["json", "jsonl", "ndjson"]
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
    {
        Some(InputFormat::Json)
    } else if ["yaml", "yml"]
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
    {
        Some(InputFormat::Yaml)
    } else {
        None
    }
}

fn format_from_prefix(data: &[u8]) -> Option<InputFormat> {
    // Keep auto-detection cheap even if a caller already has the entire input
    // in memory. Callers only need to provide a prefix this large at most.
    const SNIFF_LIMIT: usize = 8 * 1024;

    let data = &data[..data.len().min(SNIFF_LIMIT)];
    let data = data.strip_prefix(b"\xef\xbb\xbf").unwrap_or(data);
    let data = data
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .map(|start| &data[start..])?;

    if data.starts_with(b"---") || data.starts_with(b"%YAML") {
        return Some(InputFormat::Yaml);
    }

    let is_json = match data {
        [b'{' | b'[' | b'"', ..] => true,
        [b'-', digit, ..] if digit.is_ascii_digit() => true,
        [digit, ..] if digit.is_ascii_digit() => true,
        _ => [b"true".as_slice(), b"false", b"null"]
            .iter()
            .any(|keyword| starts_with_json_scalar(data, keyword)),
    };

    Some(if is_json {
        InputFormat::Json
    } else {
        InputFormat::Yaml
    })
}

fn starts_with_json_scalar(data: &[u8], keyword: &[u8]) -> bool {
    let Some(rest) = data.strip_prefix(keyword) else {
        return false;
    };

    rest.first()
        .is_none_or(|byte| byte.is_ascii_whitespace() || matches!(byte, b',' | b']' | b'}'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_an_explicit_format() {
        assert_eq!(
            InputFormat::Yaml.resolve(Some(Path::new("events.json")), b"[1, 2]"),
            InputFormat::Yaml
        );
    }

    #[test]
    fn detects_known_extensions_case_insensitively() {
        for path in ["events.json", "events.JSONL", "events.ndjson"] {
            assert_eq!(
                InputFormat::Auto.resolve(Some(Path::new(path)), b"key: value"),
                InputFormat::Json
            );
        }
        for path in ["events.yaml", "events.YML"] {
            assert_eq!(
                InputFormat::Auto.resolve(Some(Path::new(path)), b"{}"),
                InputFormat::Yaml
            );
        }
    }

    #[test]
    fn detects_json_from_a_bounded_prefix() {
        for data in [
            b"\xef\xbb\xbf \n {\"key\": 1}".as_slice(),
            b"[1, 2]",
            b"\"value\"",
            b"-12.5",
            b"true\n",
            b"null]",
        ] {
            assert_eq!(InputFormat::Auto.resolve(None, data), InputFormat::Json);
        }
    }

    #[test]
    fn treats_yaml_syntax_and_ambiguous_input_as_yaml() {
        for data in [
            b"---\nkey: value".as_slice(),
            b"%YAML 1.2\n---",
            b"key: value",
            b"- item",
            b"true: value",
            b"",
        ] {
            assert_eq!(
                InputFormat::Auto.resolve(Some(Path::new("input.txt")), data),
                InputFormat::Yaml
            );
        }
    }

    #[test]
    fn does_not_scan_past_the_sniff_limit() {
        let mut data = vec![b' '; 8 * 1024];
        data.push(b'{');

        assert_eq!(InputFormat::Auto.resolve(None, &data), InputFormat::Yaml);
    }
}
