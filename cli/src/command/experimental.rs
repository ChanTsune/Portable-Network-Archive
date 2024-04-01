mod split;

use crate::{cli::Verbosity, command, command::Command};
use clap::{Args, Subcommand};
use std::io;

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct ExperimentalArgs {
    #[command(subcommand)]
    pub(crate) command: ExperimentalCommands,
}

impl Command for ExperimentalArgs {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        match self.command {
            ExperimentalCommands::Split(cmd) => cmd.execute(verbosity),
            ExperimentalCommands::Stdio(cmd) => cmd.execute(verbosity),
        }
    }
}

#[derive(Subcommand, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum ExperimentalCommands {
    #[command(about = "Split archive")]
    Split(split::SplitCommand),
    #[command(about = "Archive manipulation via stdio")]
    Stdio(command::stdio::StdioCommand),
}
