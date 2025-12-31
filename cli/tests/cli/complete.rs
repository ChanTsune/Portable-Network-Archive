use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: None.
/// Action: Run `pna complete bash`.
/// Expectation: Command exits successfully with exit code 0.
#[test]
fn complete_bash_exits_successfully() {
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["complete", "bash"]).assert().success();
}

/// Precondition: None.
/// Action: Run `pna complete zsh`.
/// Expectation: Command exits successfully with exit code 0.
#[test]
fn complete_zsh_exits_successfully() {
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["complete", "zsh"]).assert().success();
}

/// Precondition: None.
/// Action: Run `pna complete fish`.
/// Expectation: Command exits successfully with exit code 0.
#[test]
fn complete_fish_exits_successfully() {
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["complete", "fish"]).assert().success();
}

/// Precondition: None.
/// Action: Run `pna complete powershell`.
/// Expectation: Command exits successfully with exit code 0.
#[test]
fn complete_powershell_exits_successfully() {
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args(["complete", "powershell"]).assert().success();
}
