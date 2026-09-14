mod complete;

pub(crate) use complete::PathCompleter;

/// A location relative to the current browse view's source.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DocumentPath {
    pub(crate) document_index: usize,
    pub(crate) path: String,
}

pub(crate) fn parse_optional(arguments: &str) -> anyhow::Result<Option<DocumentPath>> {
    if arguments.trim().is_empty() {
        Ok(None)
    } else {
        parse(arguments).map(Some)
    }
}

pub(crate) fn parse(arguments: &str) -> anyhow::Result<DocumentPath> {
    let Some((document_index, path)) = arguments.trim_start().split_once(char::is_whitespace)
    else {
        anyhow::bail!("requires a document index and path");
    };
    let Some(document_index) = document_index.strip_prefix('@') else {
        anyhow::bail!("document index must start with @");
    };
    let document_index = document_index
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("invalid document index: @{document_index}"))?;
    let path = path.trim();
    anyhow::ensure!(!path.is_empty(), "requires a path");
    Ok(DocumentPath {
        document_index,
        path: path.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_document_index_and_quoted_path() {
        assert_eq!(
            parse(r#"@1 .items[0]["first name"]"#).unwrap(),
            DocumentPath {
                document_index: 1,
                path: r#".items[0]["first name"]"#.into(),
            }
        );
    }

    #[test]
    fn rejects_incomplete_or_invalid_arguments() {
        for arguments in ["", "@1", "@1 ", "1 .name", "@invalid .name"] {
            assert!(parse(arguments).is_err(), "arguments: {arguments}");
        }
    }
}
