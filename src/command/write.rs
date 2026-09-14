use std::{
    fs::File,
    io::Write as _,
    path::{Path, PathBuf},
};

use anyhow::Context;

use super::{
    CompletionState,
    argument::document_path::{self, DocumentPath, PathCompleter},
    argument::file_path,
};
use crate::document_source::DocumentSource;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Write {
    pub(crate) destination: PathBuf,
    pub(crate) target: Option<DocumentPath>,
}

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Write> {
    let parse = || {
        let (destination, target) = file_path::split_prefix(arguments);
        Ok(Write {
            destination: file_path::parse(destination)?,
            target: document_path::parse_optional(target)?,
        })
    };
    parse().map_err(|error: anyhow::Error| anyhow::anyhow!("write: {error}"))
}

/// Completes the destination first, then an optional document target.
pub(crate) fn complete(
    arguments: &str,
    cursor: usize,
    path_completer: &mut PathCompleter,
) -> CompletionState {
    let leading = arguments.chars().count() - arguments.trim_start().chars().count();
    let Some(cursor) = cursor.checked_sub(leading) else {
        return CompletionState::None;
    };
    let (destination, target) = file_path::split_prefix(arguments);
    let destination_end = destination.chars().count();
    let (mut state, start) = if cursor <= destination_end {
        (file_path::complete(destination, cursor), leading)
    } else if file_path::parse(destination).is_ok() {
        (
            path_completer.complete(target, cursor - destination_end),
            leading + destination_end,
        )
    } else {
        return CompletionState::None;
    };
    if let CompletionState::Ready(request) = &mut state {
        request.replacement = start + request.replacement.start..start + request.replacement.end;
    }
    state
}

/// Save only to a new file, preserving the source format and terminating it with a newline.
pub(crate) fn save(destination: &Path, source: &DocumentSource) -> anyhow::Result<()> {
    let save = || -> std::io::Result<()> {
        let mut output = File::options()
            .write(true)
            .create_new(true)
            .open(destination)?;
        output.write_all(source.content())?;
        if !source.content().ends_with(b"\n") {
            output.write_all(b"\n")?;
        }
        output.flush()
    };
    save().with_context(|| format!("cannot write {}", destination.display()))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Cursor,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{catalog::Indexer, command::CompletionRequest, input_format::InputFormat};

    use super::*;

    struct Directory(PathBuf);

    impl Directory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("vy-write-{}-{nonce}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn source(format: InputFormat, content: &str) -> DocumentSource {
        DocumentSource::from_reader(format, Box::new(Cursor::new(content.as_bytes().to_vec())))
            .unwrap()
    }

    fn completer() -> PathCompleter {
        PathCompleter::from_catalog(
            Indexer::new(source(
                InputFormat::Json,
                r#"{"名前":[{"first name":42}],"users":[1]}"#,
            ))
            .index()
            .unwrap(),
        )
    }

    fn request(input: &str, cursor: usize) -> CompletionRequest {
        let CompletionState::Ready(request) = complete(input, cursor, &mut completer()) else {
            panic!("expected completion for {input}");
        };
        request
    }

    fn replace(input: &str, request: &CompletionRequest, candidate: usize) -> String {
        input
            .chars()
            .take(request.replacement.start)
            .chain(request.candidates[candidate].replacement.chars())
            .chain(input.chars().skip(request.replacement.end))
            .collect()
    }

    #[test]
    fn parses_required_destination_and_optional_target() {
        for file in [
            "./out.json",
            "'./export files/日本語.json'",
            r#""./export files/日本語.json""#,
        ] {
            assert_eq!(parse(file).unwrap().target, None);
            let command = parse(&format!(r#"  {file} @1 .items[0]["first name"]"#)).unwrap();
            assert_eq!(
                command.target,
                Some(DocumentPath {
                    document_index: 1,
                    path: r#".items[0]["first name"]"#.into(),
                })
            );
            assert_eq!(command.destination, file_path::parse(file).unwrap());
        }
        for input in [
            "",
            "  ",
            "\"\"",
            "'unclosed",
            "./out.json @0",
            "./out.json .items",
            "./out.json @invalid .",
            "@0 . ./out.json",
        ] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn completes_document_paths_without_touching_the_destination() {
        for (input, prefix, expected) in [
            (
                "./out.json @0 .us",
                "./out.json @0 .us",
                "./out.json @0 .users",
            ),
            (
                r#"  "./日本語 file.json" @0 ["名前"][0]["first na"]"#,
                r#"  "./日本語 file.json" @0 ["名前"][0]["first na"#,
                r#"  "./日本語 file.json" @0 ["名前"][0]["first name"]"#,
            ),
            (
                "'./out file.json'  ",
                "'./out file.json'  ",
                "'./out file.json'  @0 ",
            ),
        ] {
            let request = request(input, prefix.chars().count());
            assert_eq!(request.candidates.len(), 1);
            assert_eq!(replace(input, &request, 0), expected);
        }
    }

    #[test]
    fn completes_quoted_directories_then_files() {
        let directory = Directory::new();
        let nested = directory.0.join("日本語 files");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("result file.json"), "{}").unwrap();
        let path_prefix = format!("{}/日本語 ", directory.0.display());
        let mut double_quoted_prefix = serde_json::to_string(&path_prefix).unwrap();
        double_quoted_prefix.pop();
        for prefix in [double_quoted_prefix, format!("'{path_prefix}")] {
            assert_eq!(request(&prefix, prefix.chars().count()).candidates.len(), 1);
        }
        let input = format!("{}/日", directory.0.display());
        let directory_request = request(&input, input.chars().count());
        let input = replace(&input, &directory_request, 0);
        assert!(input.ends_with("日本語 files/"));
        let input = format!("{input}res");
        let file_request = request(&input, input.chars().count());
        let completed = replace(&input, &file_request, 0);
        assert_eq!(
            parse(&completed).unwrap().destination,
            nested.join("result file.json")
        );
        assert!(
            request(&completed, completed.chars().count())
                .candidates
                .is_empty()
        );
        let destination = nested.join("not created.json");
        let new_file = serde_json::to_string(&destination).unwrap();
        assert_eq!(parse(&new_file).unwrap().destination, destination);
        assert!(
            request(&new_file, new_file.chars().count())
                .candidates
                .is_empty()
        );
    }

    #[test]
    fn completes_a_file_in_the_middle_and_preserves_the_target() {
        let directory = Directory::new();
        fs::write(directory.0.join("result.json"), "{}").unwrap();
        for suffix in ["  ", r#" @0 ["名前"][0]["first name"]"#] {
            let prefix = format!("  {}/res", directory.0.display());
            let input = format!("{prefix}typo.json{suffix}");
            let request = request(&input, prefix.chars().count());
            assert_eq!(request.candidates.len(), 1);
            let completed = replace(&input, &request, 0);
            assert!(completed.starts_with("  "));
            let (destination, target) = file_path::split_prefix(&completed);
            assert_eq!(
                file_path::parse(destination).unwrap(),
                directory.0.join("result.json")
            );
            assert_eq!(target, suffix);
        }
    }

    #[test]
    fn writes_json_and_yaml_without_overwriting_or_creating_missing_directories() {
        let directory = Directory::new();
        for (format, content, expected) in [
            (
                InputFormat::Json,
                r#"{"z":9007199254740993,"a":1.2300}"#,
                "{\"z\":9007199254740993,\"a\":1.2300}\n",
            ),
            (
                InputFormat::Yaml,
                "value: !kind hello\n",
                "value: !kind hello\n",
            ),
        ] {
            let destination = directory.0.join(format!("{format:?}.out"));
            let source = source(format, content).subtree(0, ".").unwrap();
            save(&destination, &source).unwrap();
            assert_eq!(fs::read_to_string(&destination).unwrap(), expected);
            assert!(save(&destination, &source).is_err());
            assert_eq!(fs::read_to_string(&destination).unwrap(), expected);
            assert!(save(&directory.0.join("missing/out.json"), &source).is_err());
        }
    }
}
