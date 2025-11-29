use std::{fs, path::PathBuf, process};

use clap::{CommandFactory, Parser};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.command {
        Command::Mangen(args) => mangen(args),
        Command::Docgen(args) => docgen(args),
    }
}

#[derive(Parser)]
#[command(name = "xtask", about = "Development tasks for PNA")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Generate man pages for the CLI
    Mangen(MangenArgs),
    /// Generate markdown documentation for the CLI
    Docgen(DocgenArgs),
}

#[derive(Parser)]
struct MangenArgs {
    /// Output directory for man pages
    #[arg(short, long, default_value = "target/man")]
    output: PathBuf,
}

#[derive(Parser)]
struct DocgenArgs {
    /// Output file path for markdown documentation
    #[arg(short, long, default_value = "target/doc/pna.md")]
    output: PathBuf,
}

fn mangen(args: MangenArgs) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = &args.output;
    fs::create_dir_all(out_dir)?;

    // Get the CLI command and rename to match the binary name
    let cmd = portable_network_archive::cli::Cli::command().name("pna");

    // Use clap_mangen::generate_to which properly handles subcommands
    // and global argument propagation
    clap_mangen::generate_to(cmd, out_dir)?;

    eprintln!("Man pages generated in: {}", out_dir.display());
    Ok(())
}

fn docgen(args: DocgenArgs) -> Result<(), Box<dyn std::error::Error>> {
    let out_path = &args.output;

    // Get the CLI command and rename to match the binary name
    let cmd = portable_network_archive::cli::Cli::command().name("pna");

    // Generate markdown documentation
    let markdown = clap_markdown::help_markdown_command(&cmd);

    // Create a parent directory if it doesn't exist
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(out_path, &markdown)?;

    eprintln!("Markdown documentation generated: {}", out_path.display());
    Ok(())
}
