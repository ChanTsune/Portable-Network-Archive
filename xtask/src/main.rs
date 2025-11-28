use std::{fs, path::PathBuf, process};

use clap::{CommandFactory, Parser, builder::Resettable};

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

    // Create parent directory if it doesn't exist
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Get the CLI command and rename to match the binary name
    let mut cmd = portable_network_archive::cli::Cli::command().name("pna");

    // Build the command to propagate global arguments to subcommands.
    // This is necessary because clap-markdown iterates through subcommands
    // and global argument references need to be resolved first.
    cmd.build();

    // After build(), clap sets display_name and bin_name on subcommands to include
    // the parent name (e.g., display_name="pna-create", bin_name="pna create").
    // This causes two problems in clap-markdown:
    // 1. Section headers show "pna pna-create" instead of "pna create"
    //    (because display_name is used in path building)
    // 2. Usage lines show "pna pna create" instead of "pna create"
    //    (because render_usage() uses bin_name, then clap-markdown prepends parent path)
    //
    // We need to clear display_name on all subcommands so clap-markdown
    // uses the command name instead of display_name for section headers.
    clear_display_names(&mut cmd);

    // Generate markdown documentation
    let markdown = clap_markdown::help_markdown_command(&cmd);

    // Post-process to fix duplicate command paths.
    // After build(), clap's render_usage() includes the full path (e.g., "pna create"),
    // but clap-markdown also prepends the parent path, resulting in duplicates like:
    // - "pna pna create" (for top-level subcommands)
    // - "pna xattr pna xattr get" (for nested subcommands)
    // - "pna-pna create" (in TOC links/anchors)
    let markdown = fix_duplicate_command_paths(&markdown);

    fs::write(out_path, &markdown)?;

    eprintln!("Markdown documentation generated: {}", out_path.display());
    Ok(())
}

/// Recursively clear display_name from all subcommands.
///
/// After `Command::build()` is called, clap automatically sets display_name
/// on subcommands (e.g., "pna-create"). This causes clap-markdown to generate
/// incorrect section headers like "pna pna-create" instead of "pna create".
fn clear_display_names(cmd: &mut clap::Command) {
    for sub in cmd.get_subcommands_mut() {
        let mut owned = std::mem::take(sub);
        owned = owned.display_name(Resettable::Reset);
        clear_display_names(&mut owned);
        *sub = owned;
    }
}

/// Fix duplicate command paths in the generated markdown.
///
/// clap-markdown has a bug where it prepends the parent command path to the
/// usage string, but clap's render_usage() already includes the full path.
/// This results in duplicates like "pna xattr pna xattr get".
///
/// This function removes these duplicated path segments using simple string
/// replacement, iterating until no more changes are made.
fn fix_duplicate_command_paths(markdown: &str) -> String {
    let mut result = markdown.to_string();

    // Keep replacing until no more changes
    loop {
        let prev = result.clone();

        // Fix "pna pna " -> "pna " (simple duplicate)
        result = result.replace("pna pna ", "pna ");

        // Fix "pna X pna X " patterns where X is a subcommand
        // We enumerate known subcommand prefixes that might be duplicated
        let subcommands = [
            "create",
            "append",
            "extract",
            "list",
            "split",
            "concat",
            "strip",
            "xattr",
            "complete",
            "bug-report",
            "experimental",
            "help",
            "stdio",
            "delete",
            "update",
            "chown",
            "chmod",
            "acl",
            "migrate",
            "chunk",
            "sort",
            "diff",
            "get",
            "set",
        ];

        for sub in &subcommands {
            // Fix patterns like "pna xattr pna xattr " -> "pna xattr "
            let dup_pattern = format!("pna {sub} pna {sub} ");
            let replacement = format!("pna {sub} ");
            result = result.replace(&dup_pattern, &replacement);

            // Also handle end-of-command patterns (no trailing space)
            let dup_pattern_end = format!("pna {sub} pna {sub}`");
            let replacement_end = format!("pna {sub}`");
            result = result.replace(&dup_pattern_end, &replacement_end);
        }

        // Fix nested patterns like "pna experimental stdio pna experimental stdio"
        for sub1 in &subcommands {
            for sub2 in &subcommands {
                let dup = format!("pna {sub1} {sub2} pna {sub1} {sub2}");
                let replacement = format!("pna {sub1} {sub2}");
                result = result.replace(&dup, &replacement);
            }
        }

        // Fix 3-level nested patterns
        for sub1 in &["experimental", "help", "xattr", "acl", "chunk"] {
            for sub2 in &subcommands {
                for sub3 in &subcommands {
                    let dup = format!("pna {sub1} {sub2} {sub3} pna {sub1} {sub2} {sub3}");
                    let replacement = format!("pna {sub1} {sub2} {sub3}");
                    result = result.replace(&dup, &replacement);
                }
            }
        }

        // Fix hyphenated anchors: "#pna-pna-" -> "#pna-"
        result = result.replace("#pna-pna-", "#pna-");

        // Fix hyphenated duplicates in anchors
        for sub in &subcommands {
            // Fix "#pna-X-pna-X" -> "#pna-X"
            let dup = format!("#pna-{sub}-pna-{sub}");
            let replacement = format!("#pna-{sub}");
            result = result.replace(&dup, &replacement);
        }

        // Fix 2-level hyphenated patterns
        for sub1 in &subcommands {
            for sub2 in &subcommands {
                let dup = format!("-pna-{sub1}-{sub2}-pna-{sub1}-{sub2}");
                let replacement = format!("-pna-{sub1}-{sub2}");
                result = result.replace(&dup, &replacement);

                let dup2 = format!("#pna-{sub1}-pna-{sub1}-{sub2}");
                let replacement2 = format!("#pna-{sub1}-{sub2}");
                result = result.replace(&dup2, &replacement2);
            }
        }

        if result == prev {
            break;
        }
    }

    result
}
