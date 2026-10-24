//! The `cr` binary as a shell pipeline runs it.

use std::fs;
use std::process::{Command, Stdio};

/// Printing into a pipe whose reader has gone away (`cr … | head`) ends the
/// program quietly with success instead of panicking.
#[test]
fn closed_pipe_is_not_an_error() {
    let root = std::env::temp_dir().join(format!("cr-cli-pipe-{}", std::process::id()));
    fs::create_dir_all(root.join("Database")).unwrap();
    fs::create_dir_all(root.join("UserDbas")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_cr"))
        .arg("survey")
        .arg(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Close the reading end before the program writes anything.
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    let _ = fs::remove_dir_all(&root);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{}: {stderr}", output.status);
    assert!(stderr.is_empty(), "{stderr}");
}

/// Runs `cr` with `args`, with no data to find: a home and Wine prefix
/// of nothing.
fn cr_without_data(args: &[&str]) -> std::process::Output {
    let home = std::env::temp_dir().join(format!("cr-cli-home-{}", std::process::id()));
    fs::create_dir_all(&home).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cr"))
        .args(args)
        .env("HOME", &home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("WINEPREFIX", home.join("wine"))
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&home);
    output
}

#[test]
fn help_needs_no_data_and_mistakes_fail() {
    let help = cr_without_data(&["--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).starts_with("usage:"));

    let unknown = cr_without_data(&["foo"]);
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown command foo"));

    let missing = cr_without_data(&["chars", "/nao/existe"]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("/nao/existe"));
}
