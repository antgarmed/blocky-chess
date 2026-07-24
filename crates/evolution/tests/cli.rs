use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_blocky-evolution"))
}

#[test]
fn help_succeeds_and_describes_training() {
    let output = binary().arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("blocky-evolution train [OPTIONS]"));
    assert!(stdout.contains("--validation-depths"));
    assert!(output.stderr.is_empty());
}

#[test]
fn invalid_arguments_have_usage_exit_code_and_clear_error() {
    let output = binary()
        .args(["train", "--population-size", "3"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("population size must be even, got 3"));
    assert!(stderr.contains("Usage:"));
}
