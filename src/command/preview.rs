use super::argument::document_path::{self, DocumentPath};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Preview {
    pub(crate) target: Option<DocumentPath>,
}

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Preview> {
    let target = document_path::parse_optional(arguments)
        .map_err(|error| anyhow::anyhow!("preview: {error}"))?;
    Ok(Preview { target })
}
