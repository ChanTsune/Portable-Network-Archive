use clap::{Args, Subcommand};

#[derive(Args, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true)]
pub(crate) struct ExperimentalArgs {
    #[command(subcommand)]
    pub(crate) command: ExperimentalCommands,
}

#[derive(Subcommand, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum ExperimentalCommands {}
