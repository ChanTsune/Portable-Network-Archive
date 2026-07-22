use crate::{command, command::Command};
use clap::{Parser, Subcommand};

#[derive(Parser, Clone, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct ExperimentalCommand {
    #[command(subcommand)]
    pub(crate) command: ExperimentalCommands,
}

impl Command for ExperimentalCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        match self.command {
            ExperimentalCommands::Update(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Chown(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Chmod(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Acl(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Chunk(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Diff(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Verify(cmd) => cmd.execute(ctx),
        }
    }
}

#[derive(Subcommand, Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum ExperimentalCommands {
    #[command(about = "Update entries in archive")]
    Update(command::update::UpdateCommand),
    #[command(about = "Change owner")]
    Chown(command::chown::ChownCommand),
    #[command(about = "Change mode")]
    Chmod(command::chmod::ChmodCommand),
    #[command(about = "Manipulate ACLs of entries")]
    Acl(command::acl::AclCommand),
    #[command(about = "Chunk level operation")]
    Chunk(command::chunk::ChunkCommand),
    #[command(about = "Compare archive entries with filesystem")]
    Diff(command::diff::DiffCommand),
    #[command(about = "Verify archive integrity")]
    Verify(command::verify::VerifyCommand),
}
