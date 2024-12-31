use crate::{cli::Cli, command::Command};
use clap::{Args, CommandFactory};
use clap_complete::{generate, Generator, Shell};
use std::{env, io, path::PathBuf};

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct CompleteCommand {
    #[arg(help = "shell")]
    shell: Shell,
}

impl Command for CompleteCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        let cmd = &mut Cli::command();
        print_completions(self.shell, cmd);
        Ok(())
    }
}

fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
    let name = env::args().next().map(PathBuf::from).unwrap();
    generate(
        generator,
        cmd,
        name.file_name().unwrap().to_string_lossy(),
        &mut io::stdout().lock(),
    );
}
