mod complete;

pub(crate) use complete::complete;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Path,
    Value,
    Subtree,
}

impl Target {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Value => "value",
            Self::Subtree => "subtree",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Copy {
    pub(crate) target: Target,
}

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Copy> {
    let mut arguments = arguments.split_whitespace();
    let target = match arguments.next() {
        Some("path") => Target::Path,
        Some("value") => Target::Value,
        Some("subtree") => Target::Subtree,
        Some(unknown) => return Err(anyhow::anyhow!("unknown copy target: {unknown}")),
        None => return Err(anyhow::anyhow!("copy requires a target")),
    };
    if arguments.next().is_some() {
        return Err(anyhow::anyhow!("copy accepts exactly one target"));
    }
    Ok(Copy { target })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_targets() {
        assert_eq!(parse("path").unwrap().target, Target::Path);
        assert_eq!(parse("value").unwrap().target, Target::Value);
        assert_eq!(parse("subtree").unwrap().target, Target::Subtree);
    }

    #[test]
    fn rejects_missing_unknown_and_extra_targets() {
        assert!(parse("").is_err());
        assert!(parse("node").is_err());
        assert!(parse("path extra").is_err());
    }
}
