//! F1, which opens the help of the page shown, as the original's screens
//! do. windows-reactor's keyboard accelerators have no function keys, so
//! a keyboard hook on the UI thread sees the key before the window does.

use std::cell::RefCell;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_F1;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, SetWindowsHookExW, WH_KEYBOARD,
};

thread_local! {
    static F1: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// Calls `pressed` when F1 goes down in a window of this thread; called
/// once, on the UI thread.
pub fn on_f1(pressed: impl Fn() + 'static) {
    let first = F1.with_borrow_mut(|f1| f1.replace(Box::new(pressed)).is_none());
    if first {
        // SAFETY: the hook procedure lives as long as the program and is
        // only called on this thread.
        let _ = unsafe { SetWindowsHookExW(WH_KEYBOARD, Some(hook), None, GetCurrentThreadId()) };
    }
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // The key going down, not held down (bits 30 and 31 of the flags).
    let down = lparam.0 & (0b11 << 30) == 0;
    if code == HC_ACTION as i32 && wparam.0 == usize::from(VK_F1.0) && down {
        // Not while the callback itself is running, if it pumps messages.
        let called = F1.with(|f1| match f1.try_borrow() {
            Ok(f1) => f1.as_ref().map(|pressed| pressed()).is_some(),
            Err(_) => false,
        });
        if called {
            return LRESULT(1);
        }
    }
    // SAFETY: the hook chain gets the arguments as they came.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
