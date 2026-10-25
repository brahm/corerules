//! Puts Microsoft.Web.WebView2.Core.dll next to the Windows app. WinUI's
//! WebView2 control, which shows the help and the books, loads it when it
//! starts, and neither Windows nor the Windows App Runtime has it: without
//! it the control stays blank. It comes from the Microsoft.Web.WebView2
//! NuGet package, checked against its SHA-256, with the curl and tar of
//! Windows, as windows-reactor's own setup does for self-contained apps.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const PACKAGE: &str = "microsoft.web.webview2";
const VERSION: &str = "1.0.4078.44";
const SHA256: &str = "dc4d1d9168df26b830398303e50210b6e1729f6ce5a7ac69d2c766852f489962";
const DLL: &str = "Microsoft.Web.WebView2.Core.dll";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Only a build for Windows on Windows needs the file and has the tools.
    if !cfg!(windows) || env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    if let Err(e) = deploy() {
        println!("cargo:warning=No {DLL} next to the app, so WebView2 will not start: {e}");
    }
}

fn deploy() -> Result<(), String> {
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("no OUT_DIR")?);
    let package = out.join(format!("{PACKAGE}.{VERSION}.nupkg"));
    if !sha256_is(&package, SHA256) {
        let url = format!(
            "https://api.nuget.org/v3-flatcontainer/{PACKAGE}/{VERSION}/{PACKAGE}.{VERSION}.nupkg"
        );
        run(tool("curl.exe")?
            .args(["-s", "-f", "-L", "-o"])
            .arg(&package)
            .arg(&url))?;
        if !sha256_is(&package, SHA256) {
            let _ = std::fs::remove_file(&package);
            return Err(format!("{url} is not the package expected"));
        }
    }
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86") => "x86",
        _ => "x64",
    };
    let inside = format!("runtimes/win-{arch}/native_uap/{DLL}");
    let extracted = out.join("webview2");
    std::fs::create_dir_all(&extracted).map_err(|e| e.to_string())?;
    run(tool("tar.exe")?
        .arg("-xf")
        .arg(&package)
        .arg("-C")
        .arg(&extracted)
        .arg(&inside))?;
    let dest = target_dir(&out).join(DLL);
    std::fs::copy(extracted.join(&inside), &dest)
        .map(|_| ())
        .map_err(|e| format!("{}: {e}", dest.display()))
}

/// One of the tools in System32, which every Windows 10 and 11 has.
fn tool(name: &str) -> Result<Command, String> {
    let root = env::var_os("SystemRoot").ok_or("no SystemRoot")?;
    Ok(Command::new(
        PathBuf::from(root).join("System32").join(name),
    ))
}

fn run(command: &mut Command) -> Result<(), String> {
    match command.output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()),
        Err(e) => Err(e.to_string()),
    }
}

/// Whether the file at `path` has the SHA-256 `expected`, by certutil.
fn sha256_is(path: &Path, expected: &str) -> bool {
    if !path.is_file() {
        return false;
    }
    let Ok(output) = tool("certutil.exe").and_then(|mut c| {
        c.arg("-hashfile")
            .arg(path)
            .arg("SHA256")
            .output()
            .map_err(|e| e.to_string())
    }) else {
        return false;
    };
    // The hash is the second line, with spaces between its bytes on
    // older versions of Windows.
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .nth(1)
        .is_some_and(|line| line.replace(' ', "").eq_ignore_ascii_case(expected))
}

/// Where the app is built: the profile's folder above `out`.
fn target_dir(out: &Path) -> PathBuf {
    env::var_os("PROFILE")
        .and_then(|profile| {
            out.ancestors()
                .find(|path| path.file_name() == Some(&profile))
                .map(Path::to_path_buf)
        })
        .unwrap_or_else(|| out.ancestors().nth(3).unwrap_or(out).to_path_buf())
}
