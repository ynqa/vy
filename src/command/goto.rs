use super::argument::document_path::{self, DocumentPath};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Goto {
    pub(crate) target: DocumentPath,
}

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Goto> {
    document_path::parse(arguments)
        .map(|target| Goto { target })
        .map_err(|error| anyhow::anyhow!("goto: {error}"))
}
