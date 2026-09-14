mod complete;

pub(crate) use complete::{complete, set_target};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct View;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Get {
    pub(crate) key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Set {
    pub(crate) key: String,
    pub(crate) value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Edit;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Config {
    View(View),
    Get(Get),
    Set(Set),
    Edit(Edit),
}

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Config> {
    let arguments = arguments.trim();
    let (subcommand, arguments) = arguments
        .split_once(char::is_whitespace)
        .map_or((arguments, ""), |(subcommand, arguments)| {
            (subcommand, arguments.trim())
        });

    match subcommand {
        "view" => parse_view(arguments).map(Config::View),
        "get" => parse_get(arguments).map(Config::Get),
        "set" => parse_set(arguments).map(Config::Set),
        "edit" => parse_edit(arguments).map(Config::Edit),
        "" => Err(anyhow::anyhow!("config requires a subcommand")),
        unknown => Err(anyhow::anyhow!("unknown config subcommand: {unknown}")),
    }
}

fn parse_view(arguments: &str) -> anyhow::Result<View> {
    if arguments.is_empty() {
        Ok(View)
    } else {
        Err(anyhow::anyhow!("config view does not accept arguments"))
    }
}

fn parse_get(arguments: &str) -> anyhow::Result<Get> {
    if arguments.is_empty() {
        return Err(anyhow::anyhow!("config get requires a key"));
    }
    if arguments.chars().any(char::is_whitespace) {
        return Err(anyhow::anyhow!("config get accepts exactly one key"));
    }
    validate_key(arguments)?;
    Ok(Get {
        key: arguments.to_owned(),
    })
}

fn parse_set(arguments: &str) -> anyhow::Result<Set> {
    let Some((key, value)) = arguments.split_once(char::is_whitespace) else {
        return Err(anyhow::anyhow!("config set requires a key and value"));
    };
    validate_key(key)?;
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow::anyhow!("config set requires a value"));
    }
    Ok(Set {
        key: key.to_owned(),
        value: value.to_owned(),
    })
}

fn parse_edit(arguments: &str) -> anyhow::Result<Edit> {
    if arguments.is_empty() {
        Ok(Edit)
    } else {
        Err(anyhow::anyhow!("config edit does not accept arguments"))
    }
}

fn validate_key(key: &str) -> anyhow::Result<()> {
    if key.is_empty() || key.split('.').any(str::is_empty) {
        Err(anyhow::anyhow!("invalid configuration key: {key}"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_subcommands() {
        assert!(matches!(parse("view"), Ok(Config::View(_))));
        assert!(matches!(parse("edit"), Ok(Config::Edit(_))));
        assert_eq!(
            parse("get json.indent").unwrap(),
            Config::Get(Get {
                key: "json.indent".into()
            })
        );
        assert_eq!(
            parse(r#"set keybinds.view.browse.input.exit ["q", "Esc"]"#).unwrap(),
            Config::Set(Set {
                key: "keybinds.view.browse.input.exit".into(),
                value: r#"["q", "Esc"]"#.into()
            })
        );
    }

    #[test]
    fn rejects_missing_or_unknown_subcommands() {
        assert!(parse("").is_err());
        assert!(parse("unknown").is_err());
    }

    #[test]
    fn rejects_arguments_for_argumentless_subcommands() {
        assert!(parse("view unexpected").is_err());
        assert!(parse("edit unexpected").is_err());
    }

    #[test]
    fn rejects_missing_or_invalid_get_arguments() {
        for arguments in ["get", "get json.indent extra", "get json..indent"] {
            assert!(parse(arguments).is_err(), "arguments: {arguments}");
        }
    }

    #[test]
    fn rejects_missing_or_invalid_set_arguments() {
        for arguments in ["set", "set json.indent", "set json..indent 4"] {
            assert!(parse(arguments).is_err(), "arguments: {arguments}");
        }
    }
}
