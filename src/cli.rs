use std::path::PathBuf;

use clap::{Parser, ValueHint};

use crate::input_format::InputFormat;

/// Explore and query JSON and YAML, right in your terminal.
#[derive(Debug, Parser)]
#[command(name = "vy", version, about)]
pub(crate) struct Cli {
    /// Input file to open, or '-' for standard input.
    #[arg(value_name = "INPUT", value_hint = ValueHint::FilePath)]
    pub(crate) input: Option<PathBuf>,

    /// Path to the configuration file.
    #[arg(short = 'c', long = "config", value_name = "CONFIG", value_hint = ValueHint::FilePath)]
    pub(crate) config_file: Option<PathBuf>,

    /// Input format.
    #[arg(short, long, value_enum, default_value_t)]
    pub(crate) format: InputFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod cli {
        use super::*;

        #[test]
        fn uses_defaults() {
            let cli = Cli::try_parse_from(["vy"]).unwrap();

            assert_eq!(cli.input, None);
            assert_eq!(cli.config_file, None);
            assert_eq!(cli.format, InputFormat::Auto);
        }

        #[test]
        fn parses_input_and_format() {
            let cli = Cli::try_parse_from(["vy", "events.jsonl", "--format", "json"]).unwrap();

            assert_eq!(cli.input, Some(PathBuf::from("events.jsonl")));
            assert_eq!(cli.format, InputFormat::Json);
        }

        #[test]
        fn parses_config_file() {
            let cli = Cli::try_parse_from(["vy", "--config", "vy.toml"]).unwrap();

            assert_eq!(cli.config_file, Some(PathBuf::from("vy.toml")));
        }

        #[test]
        fn rejects_jsonl_as_a_separate_format() {
            let error = Cli::try_parse_from(["vy", "--format", "jsonl"]).unwrap_err();

            assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
        }
    }
}
