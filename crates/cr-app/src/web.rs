//! The help topics and the rule books as web pages, which the Windows app
//! shows in a WebView2 and the Mac app in a WKWebView, both from the same
//! HTML. A page posts what happens in it (a link followed, a key pressed)
//! to the app through whichever of the two it runs in.

/// The name of the WKWebView message handler a page posts to on the Mac
/// (`window.webkit.messageHandlers.corerules`).
pub const HANDLER: &str = "corerules";

/// The script's `post(message)`, which hands `message` to the app: through
/// `window.chrome.webview` in a WebView2, else through the message handler
/// [`HANDLER`] of a WKWebView. A macro, so that the pages' scripts can
/// `concat!` it.
macro_rules! post_script {
    () => {
        "function post(m){const w=window.chrome&&window.chrome.webview;if(w)w.postMessage(m);\
         else window.webkit.messageHandlers.corerules.postMessage(m)}"
    };
}

pub mod book;
pub mod help;

#[cfg(test)]
mod tests {
    #[test]
    fn the_script_posts_to_the_handler_named() {
        assert!(post_script!().contains(&format!("messageHandlers.{}.", super::HANDLER)));
    }
}
