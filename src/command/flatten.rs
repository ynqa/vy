#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Flatten;

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Flatten> {
    if arguments.is_empty() {
        Ok(Flatten)
    } else {
        Err(anyhow::anyhow!("flatten does not accept arguments"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_no_arguments() {
        assert!(matches!(parse(""), Ok(Flatten)));
        assert!(parse("unexpected").is_err());
    }
}
