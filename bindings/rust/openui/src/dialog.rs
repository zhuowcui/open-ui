//! Native open, save and folder dialogs.
//!
//! Build a dialog, then await [`FileDialog::pick`] or
//! [`FileDialog::pick_many`] in a task. The dialog runs on a background
//! thread, so the app keeps drawing while it is open. It is owned by the
//! window that was current when it was built, or the one passed to
//! [`FileDialog::parent`].
//!
//! On Windows this is the system's common item dialog. On Linux it is the
//! XDG desktop portal's file chooser, or `zenity` where no portal runs.
//!
//! ```no_run
//! use openui::{dialog, task};
//!
//! task::spawn_local(async {
//!     let chosen = dialog::open_file()
//!         .title("Open notes")
//!         .filter("Text", &["txt", "md"])
//!         .pick()
//!         .await;
//!     if let Ok(Some(path)) = chosen {
//!         println!("open {}", path.display());
//!     }
//! })
//! .detach();
//! ```

use crate::style::Error;
use crate::window::{self, WindowHandle};
use openui_platform::dialog::{DialogKind, FileFilter};
use openui_platform::NativeWindow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;

/// A dialog that chooses existing files.
pub fn open_file() -> FileDialog {
    FileDialog::new(DialogKind::Open)
}

/// A dialog that chooses where to save a file. It asks before replacing an
/// existing file.
pub fn save_file() -> FileDialog {
    FileDialog::new(DialogKind::Save)
}

/// A dialog that chooses existing folders.
pub fn pick_folder() -> FileDialog {
    FileDialog::new(DialogKind::Folder)
}

/// A native file dialog to show. Build it with [`open_file`],
/// [`save_file`] or [`pick_folder`].
#[derive(Debug, Clone)]
#[must_use = "a dialog appears only when `pick` or `pick_many` is awaited"]
pub struct FileDialog {
    request: openui_platform::dialog::FileDialog,
    parent: Option<WindowHandle>,
}

impl FileDialog {
    fn new(kind: DialogKind) -> Self {
        Self {
            request: openui_platform::dialog::FileDialog::new(kind),
            parent: window::current(),
        }
    }

    /// The dialog's title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.request.title = Some(title.into());
        self
    }

    /// The folder the dialog starts in.
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.request.directory = Some(directory.into());
        self
    }

    /// The file name the dialog suggests, for example when saving.
    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.request.file_name = Some(name.into());
        self
    }

    /// Offer a file type: a name and its extensions, such as
    /// `filter("Images", &["png", "jpg"])`. The first filter is selected
    /// first, and when saving its first extension is added to a bare name.
    /// Folder dialogs ignore filters.
    pub fn filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        self.request.filters.push(FileFilter {
            name: name.into(),
            extensions: extensions
                .iter()
                .map(|extension| extension.to_string())
                .collect(),
        });
        self
    }

    /// Own the dialog by `window` instead of the current window.
    pub fn parent(mut self, window: &WindowHandle) -> Self {
        self.parent = Some(window.clone());
        self
    }

    /// Show the dialog and wait for one choice; `None` if the user
    /// cancelled.
    pub fn pick(self) -> impl Future<Output = Result<Option<PathBuf>, Error>> {
        let chosen = self.show(false);
        async move { Ok(chosen.await?.into_iter().next()) }
    }

    /// Show the dialog and wait for any number of choices; empty if the
    /// user cancelled. A save dialog chooses at most one.
    pub fn pick_many(self) -> impl Future<Output = Result<Vec<PathBuf>, Error>> {
        self.show(true)
    }

    /// Start the dialog now, while the parent window is known, and return
    /// its result.
    fn show(mut self, multiple: bool) -> impl Future<Output = Result<Vec<PathBuf>, Error>> {
        self.request.multiple = multiple;
        let parent = self.parent.as_ref().and_then(native_of);
        let request = self.request;
        let chosen =
            crate::task::spawn_blocking(move || openui_platform::dialog::show(&request, parent));
        async move {
            chosen
                .await
                .map_err(|error| Error::Platform(error.to_string()))
        }
    }
}

thread_local! {
    /// Native handles of this thread's open windows, by runtime window id.
    static NATIVE: RefCell<HashMap<u64, NativeWindow>> = RefCell::new(HashMap::new());
}

pub(crate) fn window_opened(id: u64, native: NativeWindow) {
    NATIVE.with(|windows| windows.borrow_mut().insert(id, native));
}

pub(crate) fn window_closed(id: u64) {
    NATIVE.with(|windows| windows.borrow_mut().remove(&id));
}

pub(crate) fn reset() {
    NATIVE.with(|windows| windows.borrow_mut().clear());
}

fn native_of(window: &WindowHandle) -> Option<NativeWindow> {
    let id = window::id_of(window.key())?;
    NATIVE.with(|windows| windows.borrow().get(&id).copied())
}
