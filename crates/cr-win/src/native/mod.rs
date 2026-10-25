//! What the app asks of Windows directly, where windows-reactor has no
//! control for it: the file dialogs, printing and the paper size. Each runs on a thread
//! of its own in a single-threaded COM apartment, as the shell's dialogs
//! need, with the app's window as owner so that the dialog stays modal to
//! it while the window keeps drawing.

pub mod dialogs;
pub mod print;

use cr_app::pdf::Paper;
use windows::Win32::Foundation::HWND;
use windows::Win32::Globalization::{GetLocaleInfoEx, LOCALE_IPAPERSIZE};
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;

/// The window that owns a dialog. It is taken on the UI thread, where the
/// app's window is the active one, and handed to the dialog's thread as a
/// number, since a window handle may not cross threads in Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Owner(isize);

impl Owner {
    /// The active window of the calling thread; called on the UI thread.
    pub fn active() -> Self {
        // SAFETY: GetActiveWindow only reads the thread's input state.
        Self(unsafe { GetActiveWindow() }.0 as isize)
    }

    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut _)
    }
}

/// Runs `work` on a new thread in a single-threaded apartment and waits
/// for it; called from background work, never on the UI thread.
pub fn on_dialog_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let thread = std::thread::spawn(move || {
        // SAFETY: the apartment is entered and left on this thread only.
        let entered =
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        let result = work();
        if entered.is_ok() {
            unsafe { CoUninitialize() };
        }
        result
    });
    match thread.join() {
        Ok(result) => result,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// The paper of the user's region: Letter where Windows says Letter or
/// Legal, as in the United States, A4 elsewhere.
pub fn paper() -> Paper {
    let mut value = [0u16; 8];
    // SAFETY: the buffer is passed with its length; no locale name means
    // the user's own.
    let n = unsafe { GetLocaleInfoEx(None, LOCALE_IPAPERSIZE, Some(&mut value)) };
    let value = String::from_utf16_lossy(&value[..(n.max(1) - 1) as usize]);
    // DMPAPER_LETTER and DMPAPER_LEGAL.
    if matches!(value.as_str(), "1" | "5") {
        Paper::Letter
    } else {
        Paper::A4
    }
}
