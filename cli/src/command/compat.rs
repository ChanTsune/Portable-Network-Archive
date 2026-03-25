use crate::{command, command::Command};
use clap::{Parser, Subcommand};

#[derive(Parser, Clone, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct CompatCommand {
    #[command(subcommand)]
    pub(crate) command: CompatCommands,
}

impl Command for CompatCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        match self.command {
            CompatCommands::Bsdtar(cmd) => cmd.execute(ctx),
        }
    }
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum CompatCommands {
    #[command(about = "bsdtar-compatible interface for PNA archives")]
    Bsdtar(command::bsdtar::BsdtarCommand),
}
