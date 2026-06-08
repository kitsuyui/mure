use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_usage() {
    Command::cargo_bin("mure")
        .expect("mure binary should build")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn completion_lists_mure_function() {
    Command::cargo_bin("mure")
        .expect("mure binary should build")
        .args(["completion", "--shell", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_mure \"$@\""));
}
