use clap::Parser;

#[derive(Parser)]
pub struct BsdtarCompatArgs {}

pub fn run(_args: BsdtarCompatArgs) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("bsdtar-compat: not yet implemented");
    Ok(())
}
