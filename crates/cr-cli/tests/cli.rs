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
