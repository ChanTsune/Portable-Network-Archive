use clap::ValueEnum;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, ValueEnum)]
pub(crate) enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}

impl From<ColorChoice> for anstream::ColorChoice {
    #[inline]
    fn from(value: ColorChoice) -> Self {
        match value {
            ColorChoice::Auto => anstream::ColorChoice::Auto,
            ColorChoice::Always => anstream::ColorChoice::Always,
            ColorChoice::Never => anstream::ColorChoice::Never,
        }
    }
}
