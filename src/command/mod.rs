use std::{ops::Range, sync::Arc};

pub(crate) mod argument;
pub(crate) mod config;
pub(crate) mod copy;
pub(crate) mod flatten;
pub(crate) mod focus;
pub(crate) mod goto;
pub(crate) mod grep;
pub(crate) mod help;
pub(crate) mod hstr;
pub(crate) mod jaq;
pub(crate) mod preview;
pub(crate) mod print;
pub(crate) mod quit;
pub(crate) mod write;

/// A completion result shared by candidate generation and the completion UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompletionCandidate {
    pub(crate) display: String,
    pub(crate) replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompletionRequest {
    pub(crate) replacement: Range<usize>,
    pub(crate) candidates: Vec<CompletionCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CompletionState {
    None,
    Loading,
    Ready(CompletionRequest),
    Failed(Arc<str>),
}

/// Canonical command names shown by command completion.
///
/// Parser-only aliases such as `h` and `j` intentionally remain out of this list.
pub(crate) const COMMAND_SUGGESTIONS: &[&str] = &[
    "config", "copy", "flatten", "focus", "goto", "help", "hstr", "jaq", "preview", "print",
    "quit", "write",
];

/// Identifies a command even while its arguments are incomplete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandName {
    Config,
    Copy,
    Flatten,
    Focus,
    Goto,
    Help,
    Hstr,
    Jaq,
    Preview,
    Print,
    Quit,
    Write,
}

impl CommandName {
    pub(crate) fn parse(name: &str) -> anyhow::Result<Self> {
        match name {
            "config" => Ok(Self::Config),
            "copy" | "cp" => Ok(Self::Copy),
            "flatten" => Ok(Self::Flatten),
            "focus" => Ok(Self::Focus),
            "goto" => Ok(Self::Goto),
            "help" | "h" => Ok(Self::Help),
            "hstr" => Ok(Self::Hstr),
            "jaq" | "j" => Ok(Self::Jaq),
            "preview" => Ok(Self::Preview),
            "print" => Ok(Self::Print),
            "quit" => Ok(Self::Quit),
            "write" => Ok(Self::Write),
            "" => Err(anyhow::anyhow!("command is empty")),
            unknown => Err(anyhow::anyhow!("unknown command: {unknown}")),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Config(config::Config),
    Copy(copy::Copy),
    Flatten(flatten::Flatten),
    Focus(focus::Focus),
    Goto(goto::Goto),
    Help(help::Help),
    Hstr(hstr::Hstr),
    Jaq(jaq::Jaq),
    Preview(preview::Preview),
    Print(print::Print),
    Quit(quit::Quit),
    Write(write::Write),
}

impl Command {
    pub(crate) fn parse(input: &str) -> anyhow::Result<Self> {
        let input = input.trim();
        let (name, arguments) = input
            .split_once(char::is_whitespace)
            .map_or((input, ""), |(name, arguments)| (name, arguments.trim()));

        match CommandName::parse(name)? {
            CommandName::Config => config::parse(arguments).map(Self::Config),
            CommandName::Copy => copy::parse(arguments).map(Self::Copy),
            CommandName::Flatten => flatten::parse(arguments).map(Self::Flatten),
            CommandName::Focus => focus::parse(arguments).map(Self::Focus),
            CommandName::Goto => goto::parse(arguments).map(Self::Goto),
            CommandName::Help => help::parse(arguments).map(Self::Help),
            CommandName::Hstr => hstr::parse(arguments).map(Self::Hstr),
            CommandName::Jaq => jaq::parse(arguments).map(Self::Jaq),
            CommandName::Preview => preview::parse(arguments).map(Self::Preview),
            CommandName::Print => print::parse(arguments).map(Self::Print),
            CommandName::Quit => quit::parse(arguments).map(Self::Quit),
            CommandName::Write => write::parse(arguments).map(Self::Write),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quit_without_arguments() {
        assert_eq!(
            Command::parse(" quit \n").unwrap(),
            Command::Quit(quit::Quit)
        );
        assert!(Command::parse("quit now").is_err());
    }

    #[test]
    fn parses_focus_without_arguments() {
        assert_eq!(
            Command::parse(" focus ").unwrap(),
            Command::Focus(focus::Focus { target: None })
        );
        assert!(Command::parse("focus .items").is_err());
    }

    #[test]
    fn parses_preview_as_a_separate_command() {
        assert_eq!(
            Command::parse(" preview ").unwrap(),
            Command::Preview(preview::Preview { target: None })
        );
        assert!(Command::parse("focus text").is_err());
        assert!(Command::parse("preview text").is_err());
        assert!(Command::parse("preview .message").is_err());
    }

    #[test]
    fn path_commands_share_arguments() {
        for name in ["goto", "focus", "preview", "print"] {
            let target = argument::document_path::DocumentPath {
                document_index: 1,
                path: r#".items[0]["first name"]"#.into(),
            };
            let expected = if name == "goto" {
                Command::Goto(goto::Goto { target })
            } else if name == "focus" {
                Command::Focus(focus::Focus {
                    target: Some(target),
                })
            } else if name == "preview" {
                Command::Preview(preview::Preview {
                    target: Some(target),
                })
            } else {
                Command::Print(print::Print {
                    target: Some(target),
                })
            };
            assert_eq!(
                Command::parse(&format!(r#"{name} @1 .items[0]["first name"]"#)).unwrap(),
                expected
            );
            for arguments in ["@1", "1 .name", "@invalid .name"] {
                let error = Command::parse(&format!("{name} {arguments}")).unwrap_err();
                assert!(error.to_string().starts_with(name));
            }
        }
        assert!(Command::parse("goto").is_err());
        assert_eq!(
            Command::parse(" print ").unwrap(),
            Command::Print(print::Print { target: None })
        );
        assert!(Command::parse("print .items").is_err());
    }

    #[test]
    fn parses_jaq_command_and_alias() {
        assert!(matches!(
            Command::parse("jaq .items[]"),
            Ok(Command::Jaq(_))
        ));
        assert!(matches!(Command::parse("j .items[]"), Ok(Command::Jaq(_))));
        assert!(Command::parse("jq .items[]").is_err());
        assert!(Command::parse("unknown").is_err());
    }

    #[test]
    fn parses_help_command_and_alias() {
        assert!(matches!(Command::parse("help"), Ok(Command::Help(_))));
        assert!(matches!(Command::parse("h"), Ok(Command::Help(_))));
    }

    #[test]
    fn parses_hstr_without_arguments() {
        assert!(matches!(Command::parse("hstr"), Ok(Command::Hstr(_))));
        assert!(Command::parse("hstr jaq").is_err());
    }

    #[test]
    fn parses_flatten_without_arguments() {
        assert!(matches!(Command::parse("flatten"), Ok(Command::Flatten(_))));
        assert!(Command::parse("flatten now").is_err());
    }

    #[test]
    fn parses_config_subcommands() {
        assert!(matches!(
            Command::parse("config view"),
            Ok(Command::Config(config::Config::View(_)))
        ));
        assert!(Command::parse("config").is_err());
    }

    #[test]
    fn parses_copy_targets() {
        assert!(matches!(
            Command::parse("copy subtree"),
            Ok(Command::Copy(copy::Copy {
                target: copy::Target::Subtree
            }))
        ));
        assert!(matches!(
            Command::parse("cp path"),
            Ok(Command::Copy(copy::Copy {
                target: copy::Target::Path
            }))
        ));
        assert!(Command::parse("copy node").is_err());
    }
}
