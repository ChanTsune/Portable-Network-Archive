use crate::{
    cli::{Cli, Verbosity},
    command::Command,
};
use clap::{Args, CommandFactory};
use clap_complete::{generate, Generator, Shell};
use std::{env, io, path::PathBuf};

#[derive(Args, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct CompleteCommand {
    #[arg(help = "shell")]
    shell: Shell,
}

impl Command for CompleteCommand {
    fn execute(self, _: Verbosity) -> io::Result<()> {
        let cmd = &mut Cli::command();
        print_completions(self.shell, cmd);
        Ok(())
    }
}

fn print_completions<G: Generator>(gen: G, cmd: &mut clap::Command) {
    let name = env::args().next().map(PathBuf::from).unwrap();
    generate(
        gen,
        cmd,
        name.file_name().unwrap().to_string_lossy(),
        &mut io::stdout().lock(),
    );
}
