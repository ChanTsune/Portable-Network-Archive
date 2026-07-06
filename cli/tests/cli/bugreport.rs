//! Smoke test for bug-report generation.
//!
//! The report body is assembled by the `bugreport` crate from its built-in
//! collectors, so this module intentionally does not duplicate that library's
//! formatting and collector assertions. This test only pins the CLI wiring:
//! `pna bug-report` invokes the report pipeline and exits successfully.

use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: None.
/// Action: Run `pna bug-report`.
/// Expectation: Command exits successfully with exit code 0.
#[test]
fn bug_report_exits_successfully() {
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["bug-report"]).assert().success();
}
