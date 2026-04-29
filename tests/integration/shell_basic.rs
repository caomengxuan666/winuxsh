//! Integration tests for the WinSH shell.
//!
//! Tests end-to-end shell functionality using assert_cmd.

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

fn winsh() -> Command {
    Command::cargo_bin("winsh").unwrap()
}

#[test]
fn test_version_flag() {
    let mut cmd = winsh();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("WinSH"));
}

#[test]
fn test_help_flag() {
    let mut cmd = winsh();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("WinSH"));
}

#[test]
fn test_c_flag_echo() {
    let mut cmd = winsh();
    cmd.args(["-c", "echo hello"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn test_c_flag_exit_code() {
    let mut cmd = winsh();
    cmd.args(["-c", "exit 42"]);
    cmd.assert()
        .code(42);
}

#[test]
fn test_c_flag_pwd() {
    let mut cmd = winsh();
    cmd.args(["-c", "pwd"]);
    cmd.assert()
        .success();
}

#[test]
fn test_c_flag_variable_expansion() {
    let mut cmd = winsh();
    cmd.args(["-c", "echo $HOME"]);
    cmd.assert()
        .success();
}

#[test]
fn test_c_flag_and_operator() {
    let mut cmd = winsh();
    cmd.args(["-c", "echo a && echo b"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("a"))
        .stdout(predicate::str::contains("b"));
}

#[test]
fn test_c_flag_or_operator() {
    let mut cmd = winsh();
    cmd.args(["-c", "false || echo ok"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ok"));
}

#[test]
fn test_c_flag_redirect() {
    use std::env;

    let mut cmd = winsh();
    let temp_file = env::temp_dir().join("winsh_test_redirect.txt");
    let path_str = temp_file.to_string_lossy().to_string();

    cmd.args(["-c", &format!("echo test_output > {}", path_str)]);
    cmd.assert().success();

    let content = std::fs::read_to_string(&temp_file).unwrap_or_default();
    assert!(content.contains("test_output"));
    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_c_flag_true() {
    let mut cmd = winsh();
    cmd.args(["-c", "true"]);
    cmd.assert().success();
}

#[test]
fn test_c_flag_false() {
    let mut cmd = winsh();
    cmd.args(["-c", "false"]);
    cmd.assert().code(1);
}

#[test]
fn test_script_execution() {
    use std::env;
    use tempfile::NamedTempFile;

    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "echo script_hello").unwrap();
    writeln!(file, "echo script_world").unwrap();

    let mut cmd = winsh();
    cmd.arg(file.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("script_hello"))
        .stdout(predicate::str::contains("script_world"));
}

#[test]
fn test_unknown_command() {
    let mut cmd = winsh();
    cmd.args(["-c", "this_command_does_not_exist_xyz"]);
    cmd.assert()
        .code(predicate::ne(0));
}
