use crate::{cli::value::ChunkType, command::Command};
use clap::{Parser, ValueHint};
use pna::prelude::*;
use std::{collections::HashSet, fs, path::PathBuf};
use tabled::{builder::Builder as TableBuilder, settings::Style as TableStyle};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct ChunkCommand {
    #[command(subcommand)]
    command: ChunkCommands,
}

impl Command for ChunkCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        match self.command {
            ChunkCommands::List(cmd) => cmd.execute(ctx),
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum ChunkCommands {
    #[command(about = "List chunks")]
    List(ListCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[clap(disable_help_flag = true)]
pub(crate) struct ListCommand {
    #[arg(short, long, help = "Display chunk body")]
    pub(crate) long: bool,
    #[arg(short, long, help = "Add a header row to each column")]
    pub(crate) header: bool,
    #[arg(
        long = "type",
        value_name = "TYPE",
        help = "Only list chunks of the specified type (repeatable)"
    )]
    ty: Vec<ChunkType>,
    #[arg(
        long = "exclude-type",
        value_name = "TYPE",
        help = "Do not list chunks of the specified type (repeatable)"
    )]
    exclude_ty: Vec<ChunkType>,
    #[arg(short = 'f', long = "file", value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(long, action = clap::ArgAction::Help, help = "Print help")]
    help: (),
}

impl Command for ListCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        list_archive_chunks(self)
    }
}

#[hooq::hooq(anyhow)]
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
    let include: HashSet<pna::ChunkType> = args.ty.iter().map(|ty| ty.0).collect();
    let exclude: HashSet<pna::ChunkType> = args.exclude_ty.iter().map(|ty| ty.0).collect();
    let mut offset = pna::PNA_HEADER.len();
    let mut idx = 0;
    for chunk in pna::read_as_chunks(archive)? {
        let chunk = chunk?;
        let ty = chunk.ty();
        idx += 1;
        let accepted = (include.is_empty() || include.contains(&ty)) && !exclude.contains(&ty);
        if accepted {
            builder.push_record(
                [
                    idx.to_string(),
                    ty.to_string(),
                    chunk.length().to_string(),
                    format!("{offset:#06x}"),
                ]
                .into_iter()
                .chain(args.long.then(|| {
                    let data = chunk.data();
                    match std::str::from_utf8(data) {
                        Ok(s) => s.to_string(),
                        Err(_) => format!("{:#x}", const_hex::display(data)),
                    }
                })),
            );
        }
        offset += chunk.length() as usize + std::mem::size_of::<u32>() * 3;
    }
    let mut table = builder.build();
    table.with(TableStyle::empty());
    println!("{table}");
    Ok(())
}
