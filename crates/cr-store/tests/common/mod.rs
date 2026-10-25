//! What the integration tests share.

use std::io;
use std::path::Path;

/// Makes `link` a symbolic link to the file or folder `original`, so a
/// scratch installation uses the original's data without copying it.
#[cfg(unix)]
pub fn symlink(original: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

/// Makes `link` a symbolic link to the file or folder `original`, so a
/// scratch installation uses the original's data without copying it.
/// Windows needs Developer Mode or an administrator for it, as on CI.
#[cfg(windows)]
pub fn symlink(original: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
    if original.as_ref().is_dir() {
        std::os::windows::fs::symlink_dir(original, link)
    } else {
        std::os::windows::fs::symlink_file(original, link)
    }
}
