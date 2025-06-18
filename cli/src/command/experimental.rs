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
    fn execute(self) -> anyhow::Result<()> {
        match self.command {
            ExperimentalCommands::Stdio(cmd) => cmd.execute(),
            ExperimentalCommands::Delete(cmd) => cmd.execute(),
            ExperimentalCommands::Update(cmd) => cmd.execute(),
            ExperimentalCommands::Chown(cmd) => cmd.execute(),
            ExperimentalCommands::Chmod(cmd) => cmd.execute(),
            ExperimentalCommands::Xattr(cmd) => {
                log::warn!(
                    "`{0} experimental xattr` subcommand was stabilized, use `{0} xattr` instead.",
                    std::env::current_exe()
                        .ok()
                        .and_then(|it| it.file_name().map(|n| n.to_os_string()))
                        .unwrap_or_default()
                        .to_string_lossy()
                );
                cmd.execute()
            }
            ExperimentalCommands::Acl(cmd) => cmd.execute(),
            ExperimentalCommands::Migrate(cmd) => cmd.execute(),
            ExperimentalCommands::Chunk(cmd) => cmd.execute(),
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
    #[command(about = "Manipulate extended attributes")]
    Xattr(command::xattr::XattrCommand),
    #[command(about = "Manipulate ACLs of entries")]
    Acl(command::acl::AclCommand),
    #[command(about = "Migrate old format to latest format")]
    Migrate(command::migrate::MigrateCommand),
    #[command(about = "Chunk level operation")]
    Chunk(command::chunk::ChunkCommand),
}
