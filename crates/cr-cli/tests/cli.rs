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

/// Saving into a CD that cannot be written to uses a copy of it in the data
/// folder instead, made the first time.
#[cfg(unix)]
#[test]
fn a_read_only_cd_is_saved_into_a_local_copy() {
    use std::os::unix::fs::PermissionsExt;
    let base = std::env::temp_dir().join(format!("cr-cli-cd-{}", std::process::id()));
    let cd = base.join("cd");
    for dir in ["DATABASE", "USERDBAS", "CHARS", "PROGRAM"] {
        fs::create_dir_all(cd.join(dir)).unwrap();
    }
    fs::write(cd.join("PROGRAM/TSRP3.EXE"), b"").unwrap();
    fs::write(cd.join("USERDBAS/ALLOWEDU.DAT"), b"").unwrap();
    let read_only = |mode| {
        for dir in ["", "DATABASE", "USERDBAS", "CHARS", "PROGRAM"] {
            fs::set_permissions(cd.join(dir), fs::Permissions::from_mode(mode)).unwrap();
        }
    };
    read_only(0o555);
    // Root writes into any folder; the check is then left to CI.
    if fs::write(cd.join("CHARS/probe"), b"").is_ok() {
        read_only(0o755);
        fs::remove_dir_all(&base).unwrap();
        return;
    }
    let data = base.join("data");
    let delete = || {
        Command::new(env!("CARGO_BIN_EXE_cr"))
            .args(["delete", cd.to_str().unwrap(), "Nobody"])
            .env("HOME", &base)
            .env("XDG_DATA_HOME", &data)
            .output()
            .unwrap()
    };
    let first = String::from_utf8_lossy(&delete().stderr).into_owned();
    let again = String::from_utf8_lossy(&delete().stderr).into_owned();
    read_only(0o755);
    let copy = data.join("corerules/data");
    assert!(
        first.contains("cannot be written to; copied its data into"),
        "{first}"
    );
    assert!(first.contains(copy.to_str().unwrap()), "{first}");
    assert!(again.contains("using the copy in"), "{again}");
    assert!(copy.join("UserDbas/ALLOWEDU.DAT").is_file());
    fs::remove_dir_all(&base).unwrap();
}
