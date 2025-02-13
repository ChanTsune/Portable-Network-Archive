use std::{borrow::Cow, path::Path};

pub(crate) fn temp_dir_or_else<'p>(default: impl Fn() -> &'p Path) -> Cow<'p, Path> {
    if cfg!(target_os = "wasi") {
        default().into()
    } else {
        std::env::temp_dir().into()
    }
}
