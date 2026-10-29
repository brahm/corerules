//! Help topics as web pages, made by `cr_app::web::help`, and the `file:`
//! URIs the WebView2 opens pages from.

use std::fmt::Write;

pub use cr_app::web::help::*;

/// A path of Windows (`C:\Temp\Core Rules\a.html`) as a `file:` URI,
/// what the WebView2 opens a page from.
pub fn file_uri(path: &str) -> String {
    let mut uri = String::from("file:///");
    for byte in path.replace('\\', "/").bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                uri.push(char::from(byte))
            }
            _ => {
                let _ = write!(uri, "%{byte:02X}");
            }
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_make_file_uris() {
        assert_eq!(
            file_uri(r"C:\Users\Zoë\AppData\Local\Temp\Core Rules\MonBk.html"),
            "file:///C:/Users/Zo%C3%AB/AppData/Local/Temp/Core%20Rules/MonBk.html"
        );
    }
}
