use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::Path,
};

use clap::Parser;
use filedescriptor::{FileDescriptor, StdioDescriptor};
use promkit::{TerminalModes, TerminalSession};

use crate::{
    catalog::{CatalogLoader, FlattenLoader},
    cli::Cli,
    command::argument::document_path::PathCompleter,
    config::{Config, ConfigFile},
    document_source::DocumentSource,
    viewer::{DocumentComponent, DocumentDisplayConfig, Viewer, ViewerOutcome, ViewerSources},
};

mod catalog;
mod cli;
mod command;
mod config;
mod document_source;
mod history;
mod input_format;
mod utils;
mod viewer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config_file = ConfigFile::new(cli.config_file.clone());
    let (config, config_load_error) = match config_file.load_initial() {
        Ok(config) => (config, None),
        Err(error) => (Config::default(), Some(error)),
    };
    let mut reader: Box<dyn BufRead> = match cli.input.as_deref() {
        Some(path) if path != Path::new("-") => Box::new(BufReader::new(File::open(path)?)),
        _ => Box::new(BufReader::new(io::stdin())),
    };

    let format = {
        // `fill_buf` reads only the reader's bounded buffer and does not consume
        // it, so the parser still reads the document from the beginning.
        let data_prefix = reader.fill_buf()?;
        cli.format.resolve(cli.input.as_deref(), data_prefix)
    };
    let scroll_lines = config.scroll_lines.get();
    let keybinds = config.keybinds.clone();
    let theme = config.theme.clone();
    let display_config = DocumentDisplayConfig::from_config(format, &config);
    let document_source = DocumentSource::from_reader(format, reader)?;
    let catalog = CatalogLoader::spawn(document_source.clone());
    let flatten = FlattenLoader::spawn(catalog.clone());
    let path_completer = PathCompleter::new(catalog);
    let document = DocumentComponent::parse(document_source.content(), display_config)?;

    // promkit renders and manages terminal modes through stdout. Route those writes to
    // stderr for the interactive session, keeping the original stdout solely for :print.
    io::stdout().flush()?;
    let mut output = FileDescriptor::redirect_stdio(&io::stderr(), StdioDescriptor::Stdout)?;
    let modes = TerminalModes::RAW_MODE
        | TerminalModes::ALTERNATE_SCREEN
        | TerminalModes::HIDDEN_CURSOR
        | TerminalModes::MOUSE_CAPTURE;
    let mut terminal_session = Some(TerminalSession::try_new(modes)?);
    let mut viewer = Viewer::new(
        document,
        path_completer,
        ViewerSources::new(document_source, flatten, config_file),
        config_load_error,
        keybinds,
        theme,
        scroll_lines,
    )
    .await?;

    loop {
        match viewer.run().await? {
            ViewerOutcome::Exit => return Ok(()),
            ViewerOutcome::Print(source) => {
                drop(viewer);
                if let Some(mut session) = terminal_session.take() {
                    session.restore()?;
                }
                output.write_all(source.content())?;
                if !source.content().ends_with(b"\n") {
                    output.write_all(b"\n")?;
                }
                output.flush()?;
                return Ok(());
            }
            ViewerOutcome::EditConfig => {
                if let Some(mut session) = terminal_session.take() {
                    session.restore()?;
                }
                let edit_result = viewer.edit_config();
                terminal_session = Some(TerminalSession::try_new(modes)?);
                viewer.finish_config_edit(edit_result);
            }
        }
    }
}
