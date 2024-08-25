use crate::{command, command::Command};
use clap::{Parser, Subcommand};
use std::io;

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct ExperimentalCommand {
    #[command(subcommand)]
    pub(crate) command: ExperimentalCommands,
}

impl Command for ExperimentalCommand {
    fn execute(self) -> io::Result<()> {
        match self.command {
            ExperimentalCommands::Stdio(cmd) => cmd.execute(),
            ExperimentalCommands::Delete(cmd) => cmd.execute(),
            ExperimentalCommands::Update(cmd) => cmd.execute(),
            ExperimentalCommands::Chown(cmd) => cmd.execute(),
            ExperimentalCommands::Chmod(cmd) => cmd.execute(),
            ExperimentalCommands::Xattr(cmd) => cmd.execute(),
        }
    }
}

#[derive(Subcommand, Clone, Eq, PartialEq, Hash, Debug)]
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
}
