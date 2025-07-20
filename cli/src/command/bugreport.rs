use crate::command::Command;
use bugreport::{bugreport, collector::*, format::Markdown};
use clap::Parser;

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct BugReportCommand;

impl Command for BugReportCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        bugreport!()
            .info(SoftwareVersion::default())
            .info(OperatingSystem::default())
            .info(CommandLine::default())
            .info(CompileTimeInformation::default())
            .print::<Markdown>();
        Ok(())
    }
}
