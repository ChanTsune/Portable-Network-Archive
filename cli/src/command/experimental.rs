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
    fn execute(self, ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        match self.command {
            ExperimentalCommands::Stdio(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Delete(cmd) => {
                log::warn!(
                    "`{0} experimental delete` subcommand was stabilized, use `{0} delete` instead.",
                    std::env::current_exe()
                        .ok()
                        .and_then(|it| it.file_name().map(|n| n.to_os_string()))
                        .unwrap_or_default()
                        .to_string_lossy()
                );
                cmd.execute(ctx)
            }
            ExperimentalCommands::Update(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Chown(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Chmod(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Acl(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Migrate(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Chunk(cmd) => cmd.execute(ctx),
            ExperimentalCommands::Sort(cmd) => {
                log::warn!(
                    "`{0} experimental sort` subcommand was stabilized, use `{0} sort` instead. this command will be removed in the future.",
                    std::env::current_exe()
                        .ok()
                        .and_then(|it| it.file_name().map(|n| n.to_os_string()))
                        .unwrap_or_default()
                        .to_string_lossy()
                );
                cmd.execute(ctx)
            }
            ExperimentalCommands::Diff(cmd) => cmd.execute(ctx),
        }
    }
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum ExperimentalCommands {
    #[command(about = "Archive manipulation via stdio")]
    Stdio(command::stdio::StdioCommand),
    #[command(about = "Delete entry from archive")]
    Delete(command::delete::DeleteCommand),
    #[command(about = "Update entries in archive")]
    Update(command::update::UpdateCommand),
    #[command(about = "Change owner")]
    Chown(command::chown::ChownCommand),
    #[command(about = "Change mode")]
    Chmod(command::chmod::ChmodCommand),
    #[command(about = "Manipulate ACLs of entries")]
    Acl(command::acl::AclCommand),
    #[command(about = "Migrate old format to latest format")]
    Migrate(command::migrate::MigrateCommand),
    #[command(about = "Chunk level operation")]
    Chunk(command::chunk::ChunkCommand),
    #[command(
        about = "Sort entries in archive (stabilized, use `pna sort` command instead. this command will be removed in the future)"
    )]
    Sort(command::sort::SortCommand),
    #[command(about = "Compare archive entries with filesystem")]
    Diff(command::diff::DiffCommand),
}
