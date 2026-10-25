//! The shell's Open dialog, for a package and for an installation's
//! folder.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows::Win32::Foundation::ERROR_CANCELLED;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH,
};
use windows::core::{HRESULT, HSTRING, PCWSTR};

use super::Owner;

/// Asks for a file, offering the types of `filters` (a name and the
/// patterns, such as `*.xpx;*.xpt`); `None` when the user cancels.
pub fn open_file(
    owner: Owner,
    title: &str,
    filters: &[(String, String)],
) -> windows::core::Result<Option<PathBuf>> {
    let strings: Vec<(HSTRING, HSTRING)> = filters
        .iter()
        .map(|(name, spec)| (HSTRING::from(name.as_str()), HSTRING::from(spec.as_str())))
        .collect();
    let specs: Vec<COMDLG_FILTERSPEC> = strings
        .iter()
        .map(|(name, spec)| COMDLG_FILTERSPEC {
            pszName: PCWSTR(name.as_ptr()),
            pszSpec: PCWSTR(spec.as_ptr()),
        })
        .collect();
    open(owner, title, |dialog| unsafe {
        dialog.SetFileTypes(&specs)
    })
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
