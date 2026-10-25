//! The keys the app acts on wherever the focus is in the window: F1,
//! which opens the help of the page shown, as the original's screens do,
//! and the book reader's, as the GNOME reader has them. windows-reactor's
//! keyboard accelerators have no function keys and few letters, so a
//! keyboard hook on the UI thread sees the key before the window does.
//! Keys pressed in a WebView2 go to its own process, so a page says which
//! were pressed in it.

use std::cell::RefCell;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VIRTUAL_KEY, VK_ADD, VK_CONTROL, VK_F1, VK_F9, VK_MENU, VK_NUMPAD0, VK_OEM_MINUS,
    VK_OEM_PLUS, VK_SUBTRACT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, SetWindowsHookExW, WH_KEYBOARD,
};

/// A key the app acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// F1.
    Help,
    /// F9.
    Contents,
    /// Ctrl+F.
    Find,
    /// Ctrl+plus or Ctrl+equals.
    ZoomIn,
    /// Ctrl+minus.
    ZoomOut,
    /// Ctrl+0.
    ZoomReset,
    /// Ctrl+W.
    Close,
}

impl Key {
    fn of(key: VIRTUAL_KEY, control: bool) -> Option<Self> {
        Some(match key {
            VK_F1 if !control => Self::Help,
            VK_F9 if !control => Self::Contents,
            _ if !control => return None,
            VIRTUAL_KEY(0x46) => Self::Find,
            VK_OEM_PLUS | VK_ADD => Self::ZoomIn,
            VK_OEM_MINUS | VK_SUBTRACT => Self::ZoomOut,
            VIRTUAL_KEY(0x30) | VK_NUMPAD0 => Self::ZoomReset,
            VIRTUAL_KEY(0x57) => Self::Close,
            _ => return None,
        })
    }
}

/// What the app does with a key, and whether it took it.
type Pressed = Box<dyn Fn(Key) -> bool>;

thread_local! {
    static PRESSED: RefCell<Option<Pressed>> = const { RefCell::new(None) };
}

/// Calls `pressed` when one of the keys goes down in a window of this
/// thread; the key goes no further when it returns true. Called once, on
/// the UI thread.
pub fn on_key(pressed: impl Fn(Key) -> bool + 'static) {
    let first = PRESSED.with_borrow_mut(|p| p.replace(Box::new(pressed)).is_none());
    if first {
        // SAFETY: the hook procedure lives as long as the program and is
        // only called on this thread.
        let _ = unsafe { SetWindowsHookExW(WH_KEYBOARD, Some(hook), None, GetCurrentThreadId()) };
    }
}

/// Whether `key` is held down.
fn held(key: VIRTUAL_KEY) -> bool {
    // SAFETY: GetKeyState only reads the thread's keyboard state.
    unsafe { GetKeyState(i32::from(key.0)) < 0 }
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // The key going down, not held down (bits 30 and 31 of the flags).
    let down = lparam.0 & (0b11 << 30) == 0;
    if code == HC_ACTION as i32
        && down
        && !held(VK_MENU)
        && let Ok(vk) = u16::try_from(wparam.0)
        && let Some(key) = Key::of(VIRTUAL_KEY(vk), held(VK_CONTROL))
    {
        // Not while the callback itself is running, if it pumps messages.
        let taken = PRESSED.with(|p| match p.try_borrow() {
            Ok(p) => p.as_ref().is_some_and(|pressed| pressed(key)),
            Err(_) => false,
        });
        if taken {
            return LRESULT(1);
        }
    }
    // SAFETY: the hook chain gets the arguments as they came.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
