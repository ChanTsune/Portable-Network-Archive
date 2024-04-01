use crate::{cli::Verbosity, command::Command};
use clap::{Args, Parser, Subcommand, ValueHint};
use std::path::PathBuf;

mod create;
mod extract;

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct StdioCommand {
    #[command(subcommand)]
    command: StdioCommands,
}

impl Command for StdioCommand {
    fn execute(self, verbosity: Verbosity) -> std::io::Result<()> {
        match self.command {
            StdioCommands::Create(cmd) => cmd.execute(verbosity),
            StdioCommands::Extract(cmd) => cmd.execute(verbosity),
        }
    }
}

#[derive(Subcommand, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum StdioCommands {
    #[command(about = "Create archive")]
    Create(create::CreateCommand),
    #[command(about = "Extract archive")]
    Extract(extract::ExtractCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct FileArgs {
    #[arg(value_hint = ValueHint::FilePath)]
    pub(crate) files: Vec<PathBuf>,
}
