use clap::ValueEnum;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, ValueEnum)]
pub(crate) enum CipherMode {
    Cbc,
    Ctr,
}

impl Default for CipherMode {
    fn default() -> Self {
        Self::Ctr
    }
}
