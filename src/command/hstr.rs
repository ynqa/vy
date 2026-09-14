#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Hstr;

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Hstr> {
    if arguments.is_empty() {
        Ok(Hstr)
    } else {
        Err(anyhow::anyhow!("hstr does not accept arguments"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_without_arguments() {
        assert_eq!(parse("").unwrap(), Hstr);
        assert!(parse("jaq").is_err());
    }
}
