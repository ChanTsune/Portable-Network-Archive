use crate::command::Command;
use clap::Parser;
use std::io;

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct AclCommand {
    #[command(subcommand)]
    command: XattrCommands,
}

impl Command for AclCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        match self.command {
            XattrCommands::Get(cmd) => cmd.execute(),
            XattrCommands::Set(cmd) => cmd.execute(),
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum XattrCommands {
    #[command(about = "Get acl of entries")]
    Get(GetAclCommand),
    #[command(about = "Set acl of entries")]
    Set(SetAclCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct GetAclCommand {}

impl Command for GetAclCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        todo!()
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SetAclCommand {}

impl Command for SetAclCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        todo!()
    }
}
