use crate::{cli::Verbosity, command, command::Command};
use clap::{Parser, Subcommand};
use std::io;

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct ExperimentalCommand {
    #[command(subcommand)]
    pub(crate) command: ExperimentalCommands,
}

impl Command for ExperimentalCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        match self.command {
            ExperimentalCommands::Stdio(cmd) => cmd.execute(verbosity),
            ExperimentalCommands::Delete(cmd) => cmd.execute(verbosity),
            ExperimentalCommands::Strip(cmd) => cmd.execute(verbosity),
            ExperimentalCommands::Update(cmd) => cmd.execute(verbosity),
        }
    }
}

#[derive(Subcommand, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum ExperimentalCommands {
    #[command(about = "Archive manipulation via stdio")]
    Stdio(command::stdio::StdioCommand),
    #[command(about = "Delete entry from archive")]
    Delete(command::delete::DeleteCommand),
    #[command(about = "Strip entries metadata")]
    Strip(command::strip::StripCommand),
    #[command(about = "Update entries in archive")]
    Update(command::update::UpdateCommand),
}
