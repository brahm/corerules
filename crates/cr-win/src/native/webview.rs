//! The WebView2 the help shows its topics in and the Book page its book:
//! the CoreWebView2 behind windows-reactor's control, which Reactor hands
//! over as a COM object of its own windows-core, and the few calls the
//! pages make on it.

use std::path::Path;

use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM, ICoreWebView2, ICoreWebView2_16, ICoreWebView2Settings3,
};
use webview2_com::{WebMessageReceivedEventHandler, take_pwstr};
use windows::core::{GUID, HRESULT, HSTRING, IUnknown, IUnknown_Vtbl, Interface, PWSTR};

/// The most NavigateToString takes; a longer page is written to a file
/// and opened from there.
const MOST: usize = 1_500_000;

/// `ICoreWebView2Interop2` of the SDK's WebView2Interop.idl, by which the
/// WinRT CoreWebView2 of WinUI's control gives the COM one behind it.
const INTEROP2: GUID = GUID::from_u128(0xB151AD7C_CFB0_4ECF_B9B2_AFCA868581A6);

#[repr(C)]
struct Interop2Vtbl {
    base: IUnknown_Vtbl,
    get_com_icorewebview2:
        unsafe extern "system" fn(*mut std::ffi::c_void, *mut *mut std::ffi::c_void) -> HRESULT,
}

#[derive(Clone)]
pub struct Web {
    core: ICoreWebView2,
}

impl Web {
    /// The CoreWebView2 Reactor created for its control.
    pub fn of(unknown: &reactor_core::IUnknown) -> Option<Self> {
        let raw = reactor_core::Interface::as_raw(unknown);
        // SAFETY: `raw` is the live IUnknown Reactor holds for the call;
        // it is borrowed only to query the interface, which adds its own
        // reference.
        let unknown = unsafe { IUnknown::from_raw_borrowed(&raw) }?;
        let core = unknown
            .cast::<ICoreWebView2>()
            .ok()
            .or_else(|| com(unknown))?;
        // The page is the app's: no developer tools and no status bar.
        // SAFETY: plain property calls on a live CoreWebView2.
        unsafe {
            if let Ok(settings) = core.Settings() {
                let _ = settings.SetAreDevToolsEnabled(false);
                let _ = settings.SetIsStatusBarEnabled(false);
            }
        }
        Some(Self { core })
    }

    /// Whether both are the same CoreWebView2, so the same control.
    pub fn same(&self, other: &Self) -> bool {
        self.core.as_raw() == other.core.as_raw()
    }

    /// Calls `received` with each message the page posts.
    pub fn on_message(&self, mut received: impl FnMut(String) + 'static) {
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            if let Some(args) = args {
                let mut message = PWSTR::null();
                // SAFETY: the out parameter gets a string the call
                // allocates, which `take_pwstr` frees.
                unsafe { args.TryGetWebMessageAsString(&mut message) }?;
                received(take_pwstr(message));
            }
            Ok(())
        }));
        let mut token = 0;
        // SAFETY: the handler and the token outlive the call.
        let _ = unsafe { self.core.add_WebMessageReceived(&handler, &mut token) };
    }

    /// The page takes the keys it is given: the browser's own (Find,
    /// Reload, Print) and its zoom do nothing, so that the page can say
    /// which of the app's keys were pressed.
    pub fn keep_keys(&self) {
        // SAFETY: plain property calls on a live CoreWebView2.
        unsafe {
            if let Ok(settings) = self.core.Settings() {
                let _ = settings.SetIsZoomControlEnabled(false);
                if let Ok(settings) = settings.cast::<ICoreWebView2Settings3>() {
                    let _ = settings.SetAreBrowserAcceleratorKeysEnabled(false);
                }
            }
        }
    }

    /// Shows the page `html`.
    pub fn show(&self, html: &str) {
        if html.len() <= MOST {
            // SAFETY: a plain call on a live CoreWebView2 with a string
            // that outlives it.
            let _ = unsafe { self.core.NavigateToString(&HSTRING::from(html)) };
            return;
        }
        let path = std::env::temp_dir().join("Core Rules").join("help.html");
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, html));
        if written.is_ok() {
            self.open(&path);
        }
    }

    /// Shows the page in the file at `path`.
    pub fn open(&self, path: &Path) {
        let uri = cr_win::html::file_uri(&path.display().to_string());
        // SAFETY: a plain call on a live CoreWebView2 with a string that
        // outlives it.
        let _ = unsafe { self.core.Navigate(&HSTRING::from(uri)) };
    }

    /// Runs `script` in the page.
    pub fn run(&self, script: &str) {
        // SAFETY: the script outlives the call, and no handler is asked
        // for its result.
        let _ = unsafe { self.core.ExecuteScript(&HSTRING::from(script), None) };
    }

    /// Opens the Windows print dialog on the page.
    pub fn print(&self) {
        if let Ok(core) = self.core.cast::<ICoreWebView2_16>() {
            // SAFETY: a plain call on a live CoreWebView2.
            let _ = unsafe { core.ShowPrintUI(COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM) };
        }
    }
}

/// The COM CoreWebView2 behind the WinRT one `unknown`.
fn com(unknown: &IUnknown) -> Option<ICoreWebView2> {
    let mut interop = std::ptr::null_mut();
    // SAFETY: a plain QueryInterface.
    unsafe { unknown.query(&INTEROP2, &mut interop) }
        .ok()
        .ok()?;
    // SAFETY: on success `interop` holds a reference, which the IUnknown
    // made from it releases.
    let interop = unsafe { IUnknown::from_raw(interop) };
    let mut core = std::ptr::null_mut();
    // SAFETY: `interop` is an ICoreWebView2Interop2, whose vtable is
    // IUnknown's followed by GetComICoreWebView2; the out parameter gets
    // a referenced ICoreWebView2, which `from_raw` takes over.
    unsafe {
        let vtable = &**interop.as_raw().cast::<*const Interop2Vtbl>();
        (vtable.get_com_icorewebview2)(interop.as_raw(), &mut core)
            .ok()
            .ok()?;
    }
    // SAFETY: as above.
    (!core.is_null()).then(|| unsafe { ICoreWebView2::from_raw(core) })
}
