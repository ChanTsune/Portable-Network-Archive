use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: None.
/// Action: Run `pna bug-report`.
/// Expectation: Command exits successfully with exit code 0.
#[test]
fn bug_report_exits_successfully() {
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["bug-report"]).assert().success();
}
