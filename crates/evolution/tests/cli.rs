use std::{fs, process::Command};

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

fn minimal_training(command: &mut Command) {
    command.args([
        "train",
        "--generations",
        "1",
        "--population-size",
        "2",
        "--swiss-rounds",
        "1",
        "--elite-count",
        "0",
        "--parent-candidate-count",
        "1",
        "--search-depth",
        "1",
        "--max-game-plies",
        "1",
        "--opening-min-plies",
        "0",
        "--opening-max-plies",
        "0",
        "--validation-depths",
        "1",
        "--validation-openings",
        "1",
        "--validation-max-game-plies",
        "1",
        "--validation-opening-min-plies",
        "0",
        "--validation-opening-max-plies",
        "0",
    ]);
}

#[test]
fn checkpoint_resume_and_report_are_wired_through_the_binary() {
    let directory = std::env::temp_dir();
    let checkpoint = directory.join(format!("blocky-cli-{}-checkpoint.json", std::process::id()));
    let first_report = directory.join(format!("blocky-cli-{}-first.json", std::process::id()));
    let resumed_report = directory.join(format!("blocky-cli-{}-resumed.json", std::process::id()));

    let mut first = binary();
    minimal_training(&mut first);
    let output = first
        .arg("--checkpoint")
        .arg(&checkpoint)
        .arg("--report")
        .arg(&first_report)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut resumed = binary();
    minimal_training(&mut resumed);
    let output = resumed
        .arg("--resume")
        .arg(&checkpoint)
        .arg("--report")
        .arg(&resumed_report)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&first_report).unwrap(),
        fs::read(&resumed_report).unwrap()
    );

    fs::remove_file(checkpoint).unwrap();
    fs::remove_file(first_report).unwrap();
    fs::remove_file(resumed_report).unwrap();
}
