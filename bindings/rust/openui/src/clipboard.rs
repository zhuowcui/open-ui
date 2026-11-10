//! The system clipboard's text.
//!
//! While an app runs on a native runtime these read and write the system
//! clipboard (Win32, X11 or Wayland). Without one, for example in headless
//! tests, they use a clipboard private to the UI thread, so the same code
//! runs in both.
//!
//! Under a native runtime a document's own copy, cut and paste commands use
//! the system clipboard too: paste reads it, and copy and cut write it.
//!
//! ```no_run
//! openui::clipboard::write_text("Hello");
//! assert_eq!(openui::clipboard::read_text().unwrap(), "Hello");
//! ```

use crate::style::Error;
use std::cell::RefCell;

enum Backend {
    /// No native runtime: a clipboard for this thread only.
    Memory(String),
    #[cfg(any(
        all(feature = "linux", target_os = "linux"),
        all(feature = "windows", target_os = "windows")
    ))]
    Native(openui_platform::Clipboard),
}

thread_local! {
    static CLIPBOARD: RefCell<Backend> = const { RefCell::new(Backend::Memory(String::new())) };
}

/// The clipboard's text. Fails if it holds no text or another program is
/// using it.
pub fn read_text() -> Result<String, Error> {
    CLIPBOARD.with(|clipboard| match &*clipboard.borrow() {
        Backend::Memory(text) => Ok(text.clone()),
        #[cfg(any(
            all(feature = "linux", target_os = "linux"),
            all(feature = "windows", target_os = "windows")
        ))]
        Backend::Native(native) => native
            .read_text()
            .map_err(|error| Error::Platform(error.to_string())),
    })
}

/// Replace the clipboard's contents with `text`.
pub fn write_text(text: impl Into<String>) {
    let text = text.into();
    CLIPBOARD.with(|clipboard| match &mut *clipboard.borrow_mut() {
        Backend::Memory(stored) => *stored = text,
        #[cfg(any(
            all(feature = "linux", target_os = "linux"),
            all(feature = "windows", target_os = "windows")
        ))]
        Backend::Native(native) => native.write_text(text),
    });
}

/// Use the native clipboard while a runtime runs on this thread, or go back
/// to the private one.
#[cfg(any(
    all(feature = "linux", target_os = "linux"),
    all(feature = "windows", target_os = "windows")
))]
pub(crate) fn set_native(native: Option<openui_platform::Clipboard>) {
    CLIPBOARD.with(|clipboard| {
        *clipboard.borrow_mut() = match native {
            Some(native) => Backend::Native(native),
            None => Backend::Memory(String::new()),
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn without_a_runtime_text_round_trips_on_this_thread() {
        super::write_text("first");
        super::write_text(String::from("second"));
        assert_eq!(super::read_text().unwrap(), "second");
        let other = std::thread::spawn(super::read_text)
            .join()
            .unwrap()
            .unwrap();
        assert_eq!(other, "", "each thread has its own private clipboard");
    }
}
