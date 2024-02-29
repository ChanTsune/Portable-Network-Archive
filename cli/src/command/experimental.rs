#[cfg(feature = "unstable-generate")]
mod complete;
#[cfg(feature = "unstable-split")]
mod split;

use crate::{cli::Verbosity, command::Command};
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
            #[cfg(feature = "unstable-split")]
            ExperimentalCommands::Split(cmd) => cmd.execute(verbosity),
            #[cfg(feature = "unstable-generate")]
            ExperimentalCommands::Complete(cmd) => cmd.execute(verbosity),
        }
    }
}

#[derive(Subcommand, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum ExperimentalCommands {
    #[cfg(feature = "unstable-split")]
    #[command(about = "Split archive")]
    Split(split::SplitCommand),
    #[cfg(feature = "unstable-generate")]
    #[command(about = "Generate shell auto complete")]
    Complete(complete::CompleteCommand),
}
