use super::argument::document_path::{self, DocumentPath};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Focus {
    pub(crate) target: Option<DocumentPath>,
}

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Focus> {
    let target = document_path::parse_optional(arguments)
        .map_err(|error| anyhow::anyhow!("focus: {error}"))?;
    Ok(Focus { target })
}
