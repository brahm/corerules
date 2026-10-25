//! The shell's Open dialog, for a package and for an installation's
//! folder, and its Save dialog.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows::Win32::Foundation::ERROR_CANCELLED;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, FileOpenDialog, FileSaveDialog, IFileDialog,
    IFileOpenDialog, IFileSaveDialog, SIGDN_FILESYSPATH,
};
use windows::core::{HRESULT, HSTRING, Interface, PCWSTR};

use super::Owner;

/// Asks for a file, offering the types of `filters` (a name and the
/// patterns, such as `*.xpx;*.xpt`); `None` when the user cancels.
pub fn open_file(
    owner: Owner,
    title: &str,
    filters: &[(String, String)],
) -> windows::core::Result<Option<PathBuf>> {
    let strings = filter_strings(filters);
    let specs = filter_specs(&strings);
    open(owner, title, |dialog| unsafe {
        dialog.SetFileTypes(&specs)
    })
}

/// Asks where to save a file, offering `name` with the extension
/// `extension` and the types of `filters`; `None` when the user cancels.
/// The dialog asks before it replaces a file.
pub fn save_file(
    owner: Owner,
    title: &str,
    name: &str,
    extension: &str,
    filters: &[(String, String)],
) -> windows::core::Result<Option<PathBuf>> {
    let strings = filter_strings(filters);
    let specs = filter_specs(&strings);
    // SAFETY: as in `open`; the filter strings outlive the dialog.
    unsafe {
        let dialog: IFileSaveDialog =
            CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)?;
        dialog.SetTitle(&HSTRING::from(title))?;
        dialog.SetFileTypes(&specs)?;
        dialog.SetDefaultExtension(&HSTRING::from(extension))?;
        dialog.SetFileName(&HSTRING::from(name))?;
        show(&dialog.cast()?, owner)
    }
}

fn filter_strings(filters: &[(String, String)]) -> Vec<(HSTRING, HSTRING)> {
    filters
        .iter()
        .map(|(name, spec)| (HSTRING::from(name.as_str()), HSTRING::from(spec.as_str())))
        .collect()
}

fn filter_specs(strings: &[(HSTRING, HSTRING)]) -> Vec<COMDLG_FILTERSPEC> {
    strings
        .iter()
        .map(|(name, spec)| COMDLG_FILTERSPEC {
            pszName: PCWSTR(name.as_ptr()),
            pszSpec: PCWSTR(spec.as_ptr()),
        })
        .collect()
}

/// Asks for a folder; `None` when the user cancels.
pub fn open_folder(owner: Owner, title: &str) -> windows::core::Result<Option<PathBuf>> {
    open(owner, title, |dialog| unsafe {
        dialog.SetOptions(dialog.GetOptions()? | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)
    })
}

fn open(
    owner: Owner,
    title: &str,
    setup: impl FnOnce(&IFileOpenDialog) -> windows::core::Result<()>,
) -> windows::core::Result<Option<PathBuf>> {
    // SAFETY: plain COM calls on this thread's apartment; the path the
    // dialog returns is copied before it is freed.
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        dialog.SetTitle(&HSTRING::from(title))?;
        setup(&dialog)?;
        show(&dialog.cast()?, owner)
    }
}

/// Shows the dialog over `owner` and returns the path chosen.
///
/// # Safety
/// Called on the dialog's thread, in its apartment.
unsafe fn show(dialog: &IFileDialog, owner: Owner) -> windows::core::Result<Option<PathBuf>> {
    // SAFETY: the path the dialog returns is copied before it is freed.
    unsafe {
        match dialog.Show(Some(owner.hwnd())) {
            Ok(()) => {}
            Err(e) if e.code() == HRESULT::from_win32(ERROR_CANCELLED.0) => return Ok(None),
            Err(e) => return Err(e),
        }
        let name = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = PathBuf::from(OsString::from_wide(name.as_wide()));
        CoTaskMemFree(Some(name.0 as *const _));
        Ok(Some(path))
    }
}
