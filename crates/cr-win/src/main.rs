//! Core Rules for Windows 11: a WinUI 3 interface, written with
//! windows-reactor, over the same backend as the GNOME app (`cr-store`,
//! `cr-formats` and `cr-i18n`). On other systems this program only says
//! so; there the GNOME app is `corerules`.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod native;
#[cfg(windows)]
mod prefs;
#[cfg(windows)]
mod ui;

#[cfg(windows)]
fn main() {
    app::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("This is the Windows version of Core Rules; on Linux, run corerules.");
    std::process::exit(1);
}
