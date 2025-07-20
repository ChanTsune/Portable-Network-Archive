use crate::command::Command;
use clap::Parser;
use pna::prelude::*;
use std::{fs, path::PathBuf};
use tabled::{builder::Builder as TableBuilder, settings::Style as TableStyle};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct ChunkCommand {
    #[command(subcommand)]
    command: ChunkCommands,
}

impl Command for ChunkCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        match self.command {
            ChunkCommands::List(cmd) => cmd.execute(),
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum ChunkCommands {
    #[command(about = "List chunks")]
    List(ListCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[clap(disable_help_flag = true)]
pub(crate) struct ListCommand {
    #[arg(short, long, help = "Display chunk body")]
    pub(crate) long: bool,
    #[arg(short, long, help = "Add a header row to each column")]
    pub(crate) header: bool,
    #[arg()]
    pub(crate) archive: PathBuf,
    #[arg(long, action = clap::ArgAction::Help)]
    help: Option<bool>,
}

impl Command for ListCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        list_archive_chunks(self)
    }
}

fn list_archive_chunks(args: ListCommand) -> anyhow::Result<()> {
    let archive = fs::File::open(args.archive)?;
    let mut builder = TableBuilder::new();
    if args.header {
        builder.push_record(
            ["Index", "Type", "Size", "Offset"]
                .into_iter()
                .chain(args.long.then_some("Body")),
        )
    }
    let mut offset = pna::PNA_HEADER.len();
    let mut idx = 0;
    for chunk in pna::read_as_chunks(archive)? {
        let chunk = chunk?;
        idx += 1;
        builder.push_record(
            [
                idx.to_string(),
                chunk.ty().to_string(),
                chunk.length().to_string(),
                format!("{offset:#06x}"),
            ]
            .into_iter()
            .chain(args.long.then(|| {
                std::str::from_utf8(chunk.data())
                    .unwrap_or_default()
                    .to_string()
            })),
        );
        offset += chunk.length() as usize + std::mem::size_of::<u32>() * 3;
    }
    let mut table = builder.build();
    table.with(TableStyle::empty());
    println!("{table}");
    Ok(())
}
