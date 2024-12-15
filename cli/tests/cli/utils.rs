pub mod diff;

pub fn setup() {
    #[cfg(target_os = "wasi")]
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).expect("Failed to set current dir");
}
