use crate::utils::setup;
use portable_network_archive::cli;

#[test]
fn extract_command_accepts_o_flag() {
    setup();
    cli::Cli::try_parse_from([
        "pna",
        "extract",
        "--file",
        "test.pna",
        "-o",
    ])
    .unwrap();
}
