use super::argument::document_path::{self, DocumentPath};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Print {
    pub(crate) target: Option<DocumentPath>,
}

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Print> {
    let target = document_path::parse_optional(arguments)
        .map_err(|error| anyhow::anyhow!("print: {error}"))?;
    Ok(Print { target })
}
