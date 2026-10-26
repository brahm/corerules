//! Scrolls a list of the window to its selected item, through UI
//! Automation, which windows-reactor's ListView has nothing for: the
//! book's contents follow the text, and the library's list shows the
//! entry selected.

use std::mem::ManuallyDrop;
use std::time::Duration;

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::Variant::{VARIANT, VT_BSTR, VariantClear};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationScrollItemPattern,
    IUIAutomationSelectionPattern, IUIAutomationVirtualizedItemPattern, TreeScope_Descendants,
    UIA_AutomationIdPropertyId, UIA_ScrollItemPatternId, UIA_SelectionPatternId,
    UIA_VirtualizedItemPatternId,
};
use windows::core::{BSTR, Result};

use super::Owner;

/// How many times, and how far apart, the list is looked for: it may
/// not be laid out yet when its selection changes.
const TRIES: u32 = 5;
const PAUSE: Duration = Duration::from_millis(100);

/// Scrolls the list with the automation id `id`, in the window `owner`,
/// to its selected item, on a thread of its own: UI Automation must not
/// be asked about a window on the window's thread.
pub fn show_selected(owner: Owner, id: &'static str) {
    if owner.0 == 0 {
        return;
    }
    std::thread::spawn(move || {
        // SAFETY: the apartment is entered and left on this thread only.
        let entered = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        for _ in 0..TRIES {
            std::thread::sleep(PAUSE);
            if scroll(owner, id).is_ok() {
                break;
            }
        }
        if entered.is_ok() {
            unsafe { CoUninitialize() };
        }
    });
}

fn scroll(owner: Owner, id: &str) -> Result<()> {
    // SAFETY: COM calls on interfaces this thread made, with arguments
    // that outlive them; the VARIANT is cleared once the condition has
    // it.
    unsafe {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
        let window = automation.ElementFromHandle(owner.hwnd())?;
        let mut value = text(id);
        let condition = automation.CreatePropertyCondition(UIA_AutomationIdPropertyId, &value);
        let _ = VariantClear(&mut value);
        let list = window.FindFirst(TreeScope_Descendants, &condition?)?;
        let selection: IUIAutomationSelectionPattern =
            list.GetCurrentPatternAs(UIA_SelectionPatternId)?;
        let selected = selection.GetCurrentSelection()?;
        if selected.Length()? < 1 {
            return Ok(());
        }
        let item: IUIAutomationElement = selected.GetElement(0)?;
        // A list makes only the items near what it shows.
        if let Ok(virtualized) = item.GetCurrentPatternAs::<IUIAutomationVirtualizedItemPattern>(
            UIA_VirtualizedItemPatternId,
        ) {
            let _ = virtualized.Realize();
        }
        let scroll: IUIAutomationScrollItemPattern =
            item.GetCurrentPatternAs(UIA_ScrollItemPatternId)?;
        scroll.ScrollIntoView()
    }
}

/// `value` as a VARIANT, which the caller clears.
fn text(value: &str) -> VARIANT {
    let mut variant = VARIANT::default();
    // SAFETY: the VARIANT is written as a BSTR, its type said with it.
    unsafe {
        let inner = &mut *variant.Anonymous.Anonymous;
        inner.vt = VT_BSTR;
        inner.Anonymous.bstrVal = ManuallyDrop::new(BSTR::from(value));
    }
    variant
}
