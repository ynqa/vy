#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Help;

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Help> {
    if arguments.is_empty() {
        Ok(Help)
    } else {
        Err(anyhow::anyhow!("help does not accept arguments"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_no_arguments() {
        assert_eq!(parse("").unwrap(), Help);
    }

    #[test]
    fn rejects_arguments() {
        assert!(parse("unexpected").is_err());
    }
}
