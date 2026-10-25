//! The WebView2 of a page, the help's or a book's: Reactor's reference to
//! the control, and its CoreWebView2 once made.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows_reactor::*;

use super::Message;
use crate::native::webview::Web;

/// How many times a page asks again for its control's CoreWebView2 while
/// Reactor has not published the control yet, or has made it anew,
/// before it says the page could not be shown.
const TRIES: u32 = 20;

#[derive(Clone, Default)]
pub struct WebHost {
    pub control: ElementRef<WebView2>,
    web: Rc<RefCell<Option<Web>>>,
    /// The requests in a row that found no control.
    misses: Rc<Cell<u32>>,
}

impl WebHost {
    pub fn web(&self) -> Option<Web> {
        self.web.borrow().clone()
    }

    /// The page is no longer shown: its control and CoreWebView2 go.
    pub fn forget(&self) {
        self.web.borrow_mut().take();
        self.misses.set(0);
    }

    /// Asks for the control's CoreWebView2 and calls `show` with it;
    /// `made` first, once for each CoreWebView2. While the control is not
    /// there, the page is asked to render again ([`Message::WebRetry`]),
    /// which asks anew; a CoreWebView2 that could not be made is said
    /// ([`Message::WebFailed`], with the HRESULT or what was missing).
    pub fn request(
        &self,
        sender: LocalSender<Message>,
        made: impl Fn(&Web) + 'static,
        show: impl Fn(&Web) + 'static,
    ) {
        let misses = self.misses.clone();
        let missed = {
            let sender = sender.clone();
            move || {
                misses.set(misses.get() + 1);
                sender.send(if misses.get() < TRIES {
                    Message::WebRetry
                } else {
                    Message::WebFailed("no control".to_owned())
                });
            }
        };
        let slot = self.web.clone();
        let found = self.misses.clone();
        let completion = {
            let missed = missed.clone();
            move |core: Result<reactor_core::IUnknown, WebView2Error>| match core {
                Ok(core) => {
                    let Some(web) = Web::of(&core) else {
                        sender.send(Message::WebFailed("no ICoreWebView2".to_owned()));
                        return;
                    };
                    found.set(0);
                    let mut current = slot.borrow_mut();
                    if current.as_ref().is_none_or(|w| !w.same(&web)) {
                        made(&web);
                        *current = Some(web.clone());
                    }
                    drop(current);
                    show(&web);
                }
                Err(WebView2Error::Unavailable) => missed(),
                Err(WebView2Error::Native(code)) => {
                    // The HRESULT, as Windows writes it.
                    sender.send(Message::WebFailed(format!(
                        "0x{:08X}",
                        code.cast_unsigned()
                    )));
                }
            }
        };
        if !self.control.request_core_web_view2(completion) {
            missed();
        }
    }
}
