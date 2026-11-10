//! Native open, save and folder dialogs.
//!
//! [`show`] blocks until the user answers, so call it off the event-loop
//! thread. On Windows it shows the system's common item dialog. On Linux it
//! asks the XDG desktop portal's file chooser over D-Bus and, where no
//! portal is running, falls back to `zenity`.

use crate::{NativeWindow, PlatformError};
use std::path::PathBuf;

/// What a dialog asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    /// Existing files.
    Open,
    /// A file name to save to; the file need not exist.
    Save,
    /// Existing folders.
    Folder,
}

/// A file type the user may choose: a name and its extensions without dots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

/// A dialog to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDialog {
    pub kind: DialogKind,
    pub title: Option<String>,
    /// The folder the dialog starts in.
    pub directory: Option<PathBuf>,
    /// The file name the dialog suggests.
    pub file_name: Option<String>,
    pub filters: Vec<FileFilter>,
    /// Whether the user may choose several items. Ignored when saving.
    pub multiple: bool,
}

impl FileDialog {
    pub fn new(kind: DialogKind) -> Self {
        Self {
            kind,
            title: None,
            directory: None,
            file_name: None,
            filters: Vec::new(),
            multiple: false,
        }
    }
}

/// Show `dialog` above `parent` and wait for the user. Returns the chosen
/// paths, or none if the user cancelled.
pub fn show(
    dialog: &FileDialog,
    parent: Option<NativeWindow>,
) -> Result<Vec<PathBuf>, PlatformError> {
    #[cfg(target_os = "windows")]
    {
        windows::show(dialog, parent)
    }
    #[cfg(target_os = "linux")]
    {
        linux::show(dialog, parent)
    }
}

/// `*.txt;*.md`-style patterns for a filter's extensions.
fn patterns(filter: &FileFilter) -> Vec<String> {
    if filter.extensions.is_empty() {
        vec!["*".to_owned()]
    } else {
        filter
            .extensions
            .iter()
            .map(|extension| format!("*.{}", extension.trim_start_matches('.')))
            .collect()
    }
}

/// The path a save dialog suggests: its folder joined with its file name.
#[cfg(any(target_os = "linux", test))]
fn suggested_path(dialog: &FileDialog) -> Option<PathBuf> {
    match (&dialog.directory, &dialog.file_name) {
        (Some(directory), Some(name)) => Some(directory.join(name)),
        (Some(directory), None) => Some(directory_with_separator(directory)),
        (None, Some(name)) => Some(PathBuf::from(name)),
        (None, None) => None,
    }
}

/// A folder path that ends in a separator, so tools treat it as a folder.
#[cfg(any(target_os = "linux", test))]
fn directory_with_separator(directory: &std::path::Path) -> PathBuf {
    let mut path = directory.as_os_str().to_owned();
    if !path.is_empty() && !directory.as_os_str().to_string_lossy().ends_with('/') {
        path.push(std::path::MAIN_SEPARATOR_STR);
    }
    PathBuf::from(path)
}

#[cfg(target_os = "windows")]
mod windows {
    use super::{patterns, DialogKind, FileDialog};
    use crate::{NativeWindow, PlatformError};
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use windows::core::{Interface, HRESULT, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
    };
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{
        FileOpenDialog, FileSaveDialog, IFileDialog, IFileOpenDialog, IFileSaveDialog, IShellItem,
        SHCreateItemFromParsingName, FOS_ALLOWMULTISELECT, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR,
        FOS_OVERWRITEPROMPT, FOS_PICKFOLDERS, SIGDN_FILESYSPATH,
    };

    fn failed(error: windows::core::Error) -> PlatformError {
        PlatformError::Dialog(error.to_string())
    }

    pub(super) fn show(
        dialog: &FileDialog,
        parent: Option<NativeWindow>,
    ) -> Result<Vec<PathBuf>, PlatformError> {
        // SAFETY: the dialog's COM objects are created, used and released on
        // this thread between this initialization and the matching
        // uninitialization.
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)
                .ok()
                .map_err(|error| {
                    PlatformError::Dialog(format!(
                        "file dialogs need a single-threaded COM apartment: {error}"
                    ))
                })?;
            let result = show_in_apartment(dialog, parent);
            CoUninitialize();
            result
        }
    }

    unsafe fn show_in_apartment(
        dialog: &FileDialog,
        parent: Option<NativeWindow>,
    ) -> Result<Vec<PathBuf>, PlatformError> {
        let (file_dialog, open): (IFileDialog, Option<IFileOpenDialog>) = match dialog.kind {
            DialogKind::Save => {
                let save: IFileSaveDialog =
                    CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
                        .map_err(failed)?;
                (save.cast().map_err(failed)?, None)
            }
            DialogKind::Open | DialogKind::Folder => {
                let open: IFileOpenDialog =
                    CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
                        .map_err(failed)?;
                (open.cast().map_err(failed)?, Some(open))
            }
        };

        let mut options = file_dialog.GetOptions().map_err(failed)?;
        options |= FOS_FORCEFILESYSTEM | FOS_NOCHANGEDIR;
        match dialog.kind {
            DialogKind::Save => options |= FOS_OVERWRITEPROMPT,
            DialogKind::Folder => options |= FOS_PICKFOLDERS,
            DialogKind::Open => {}
        }
        if dialog.multiple && dialog.kind != DialogKind::Save {
            options |= FOS_ALLOWMULTISELECT;
        }
        file_dialog.SetOptions(options).map_err(failed)?;

        if let Some(title) = &dialog.title {
            file_dialog
                .SetTitle(&HSTRING::from(title.as_str()))
                .map_err(failed)?;
        }
        // The strings must outlive SetFileTypes, which copies them.
        let filters: Vec<(HSTRING, HSTRING)> = dialog
            .filters
            .iter()
            .map(|filter| {
                (
                    HSTRING::from(filter.name.as_str()),
                    HSTRING::from(patterns(filter).join(";")),
                )
            })
            .collect();
        if !filters.is_empty() && dialog.kind != DialogKind::Folder {
            let specs: Vec<COMDLG_FILTERSPEC> = filters
                .iter()
                .map(|(name, spec)| COMDLG_FILTERSPEC {
                    pszName: PCWSTR(name.as_ptr()),
                    pszSpec: PCWSTR(spec.as_ptr()),
                })
                .collect();
            file_dialog.SetFileTypes(&specs).map_err(failed)?;
            if dialog.kind == DialogKind::Save {
                if let Some(extension) = dialog.filters[0].extensions.first() {
                    file_dialog
                        .SetDefaultExtension(&HSTRING::from(extension.trim_start_matches('.')))
                        .map_err(failed)?;
                }
            }
        }
        if let Some(directory) = &dialog.directory {
            // A folder that does not exist leaves the system's default.
            if let Ok(folder) = SHCreateItemFromParsingName::<_, _, IShellItem>(
                &HSTRING::from(directory.as_path()),
                None,
            ) {
                file_dialog.SetFolder(&folder).map_err(failed)?;
            }
        }
        if let Some(name) = &dialog.file_name {
            file_dialog
                .SetFileName(&HSTRING::from(name.as_str()))
                .map_err(failed)?;
        }

        let owner = parent
            .and_then(|parent| parent.win32())
            .map(|hwnd| HWND(hwnd as _));
        if let Err(error) = file_dialog.Show(owner) {
            if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Ok(Vec::new());
            }
            return Err(failed(error));
        }

        match open {
            Some(open) if dialog.multiple => {
                let items = open.GetResults().map_err(failed)?;
                let count = items.GetCount().map_err(failed)?;
                (0..count)
                    .map(|index| path_of(&items.GetItemAt(index).map_err(failed)?))
                    .collect()
            }
            _ => Ok(vec![path_of(&file_dialog.GetResult().map_err(failed)?)?]),
        }
    }

    unsafe fn path_of(item: &IShellItem) -> Result<PathBuf, PlatformError> {
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).map_err(failed)?;
        let path = PathBuf::from(OsString::from_wide(name.as_wide()));
        CoTaskMemFree(Some(name.0 as _));
        Ok(path)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{patterns, suggested_path, DialogKind, FileDialog};
    use crate::{NativeWindow, PlatformError};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

    pub(super) fn show(
        dialog: &FileDialog,
        parent: Option<NativeWindow>,
    ) -> Result<Vec<PathBuf>, PlatformError> {
        match portal(dialog, parent) {
            Ok(paths) => Ok(paths),
            Err(PortalError::Unavailable(reason)) => zenity(dialog).map_err(|error| {
                PlatformError::Dialog(format!(
                    "no file chooser is available: the desktop portal failed ({reason}) and {error}"
                ))
            }),
            Err(PortalError::Failed(error)) => Err(PlatformError::Dialog(error)),
        }
    }

    enum PortalError {
        /// No portal answered; another chooser may still work.
        Unavailable(String),
        /// The portal showed a chooser, or tried to, and failed.
        Failed(String),
    }

    const DESTINATION: &str = "org.freedesktop.portal.Desktop";

    /// The portal's `parent_window` for `parent`. Wayland needs an exported
    /// surface handle, which winit does not offer, so Wayland parents are
    /// left empty and the chooser opens unparented.
    pub(super) fn parent_window(parent: Option<NativeWindow>) -> String {
        parent
            .and_then(|parent| parent.x11())
            .map(|window| format!("x11:{window:x}"))
            .unwrap_or_default()
    }

    /// The portal method and its options for `dialog`.
    pub(super) fn request(
        dialog: &FileDialog,
        token: &str,
    ) -> (&'static str, HashMap<&'static str, Value<'static>>) {
        let mut options: HashMap<&'static str, Value<'static>> = HashMap::new();
        options.insert("handle_token", Value::from(token.to_owned()));
        options.insert("modal", Value::from(true));
        let method = match dialog.kind {
            DialogKind::Save => {
                if let Some(name) = &dialog.file_name {
                    options.insert("current_name", Value::from(name.clone()));
                }
                "SaveFile"
            }
            DialogKind::Open | DialogKind::Folder => {
                options.insert("multiple", Value::from(dialog.multiple));
                options.insert("directory", Value::from(dialog.kind == DialogKind::Folder));
                "OpenFile"
            }
        };
        if let Some(directory) = &dialog.directory {
            // A null-terminated byte string, as the portal expects.
            let mut bytes = std::os::unix::ffi::OsStrExt::as_bytes(directory.as_os_str()).to_vec();
            bytes.push(0);
            options.insert("current_folder", Value::from(bytes));
        }
        if !dialog.filters.is_empty() && dialog.kind != DialogKind::Folder {
            let filters: Vec<(String, Vec<(u32, String)>)> = dialog
                .filters
                .iter()
                .map(|filter| {
                    let globs = patterns(filter).into_iter().map(|glob| (0, glob)).collect();
                    (filter.name.clone(), globs)
                })
                .collect();
            options.insert("filters", Value::from(filters));
        }
        (method, options)
    }

    fn portal(
        dialog: &FileDialog,
        parent: Option<NativeWindow>,
    ) -> Result<Vec<PathBuf>, PortalError> {
        static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
        let unavailable = |error: zbus::Error| PortalError::Unavailable(error.to_string());
        let failed = |error: zbus::Error| PortalError::Failed(error.to_string());

        let connection = zbus::blocking::Connection::session().map_err(unavailable)?;
        let sender = connection
            .unique_name()
            .ok_or_else(|| PortalError::Unavailable("the session bus gave no name".into()))?
            .trim_start_matches(':')
            .replace('.', "_");
        let token = format!(
            "openui_{}_{}",
            std::process::id(),
            NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
        );
        // Listen for the answer before asking, so it cannot be missed.
        let expected = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
        let mut responses = response_stream(&connection, &expected).map_err(unavailable)?;

        let chooser = zbus::blocking::Proxy::new(
            &connection,
            DESTINATION,
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.FileChooser",
        )
        .map_err(unavailable)?;
        let (method, options) = request(dialog, &token);
        let title = dialog.title.clone().unwrap_or_default();
        let handle: OwnedObjectPath = chooser
            .call(method, &(parent_window(parent), title, options))
            .map_err(|error| {
                if missing(&error) {
                    unavailable(error)
                } else {
                    failed(error)
                }
            })?;
        // Portals older than version 0.9 pick their own request path.
        if handle.as_str() != expected {
            responses = response_stream(&connection, handle.as_str()).map_err(failed)?;
        }

        let message = responses
            .next()
            .ok_or_else(|| PortalError::Failed("the portal closed without answering".into()))?;
        let (code, results): (u32, HashMap<String, OwnedValue>) =
            message.body().deserialize().map_err(failed)?;
        match code {
            0 => {
                let uris: Vec<String> = match results.get("uris") {
                    Some(uris) => uris
                        .try_clone()
                        .map_err(|error| PortalError::Failed(error.to_string()))?
                        .try_into()
                        .map_err(|error: zbus::zvariant::Error| {
                            PortalError::Failed(error.to_string())
                        })?,
                    None => Vec::new(),
                };
                uris.iter()
                    .map(|uri| {
                        path_from_uri(uri).ok_or_else(|| {
                            PortalError::Failed(format!("the portal chose a non-local file: {uri}"))
                        })
                    })
                    .collect()
            }
            1 => Ok(Vec::new()),
            code => Err(PortalError::Failed(format!(
                "the portal ended the request (response {code})"
            ))),
        }
    }

    /// Whether `error` means no portal file chooser is there to answer.
    fn missing(error: &zbus::Error) -> bool {
        use zbus::fdo::Error as Fdo;
        match error {
            zbus::Error::MethodError(name, ..) => matches!(
                name.as_str(),
                "org.freedesktop.DBus.Error.ServiceUnknown"
                    | "org.freedesktop.DBus.Error.NameHasNoOwner"
                    | "org.freedesktop.DBus.Error.UnknownMethod"
                    | "org.freedesktop.DBus.Error.UnknownInterface"
                    | "org.freedesktop.DBus.Error.UnknownObject"
            ),
            zbus::Error::FDO(error) => matches!(
                **error,
                Fdo::ServiceUnknown(_)
                    | Fdo::NameHasNoOwner(_)
                    | Fdo::UnknownMethod(_)
                    | Fdo::UnknownInterface(_)
                    | Fdo::UnknownObject(_)
            ),
            _ => false,
        }
    }

    fn response_stream(
        connection: &zbus::blocking::Connection,
        path: &str,
    ) -> zbus::Result<zbus::blocking::proxy::SignalIterator<'static>> {
        let request = zbus::blocking::Proxy::new_owned(
            connection.clone(),
            DESTINATION,
            path.to_owned(),
            "org.freedesktop.portal.Request",
        )?;
        request.receive_signal("Response")
    }

    /// The local path of a `file://` URI.
    pub(super) fn path_from_uri(uri: &str) -> Option<PathBuf> {
        let rest = uri.strip_prefix("file://")?;
        // Skip an authority such as `localhost`; the path starts at `/`.
        let path = &rest[rest.find('/')?..];
        let mut bytes = Vec::with_capacity(path.len());
        let mut input = path.bytes();
        while let Some(byte) = input.next() {
            if byte == b'%' {
                let high = (input.next()? as char).to_digit(16)?;
                let low = (input.next()? as char).to_digit(16)?;
                bytes.push((high * 16 + low) as u8);
            } else {
                bytes.push(byte);
            }
        }
        Some(PathBuf::from(
            <std::ffi::OsString as std::os::unix::ffi::OsStringExt>::from_vec(bytes),
        ))
    }

    /// The `zenity` arguments for `dialog`.
    pub(super) fn zenity_arguments(dialog: &FileDialog) -> Vec<String> {
        let mut arguments = vec!["--file-selection".to_owned()];
        if let Some(title) = &dialog.title {
            arguments.push(format!("--title={title}"));
        }
        match dialog.kind {
            DialogKind::Save => arguments.push("--save".to_owned()),
            DialogKind::Folder => arguments.push("--directory".to_owned()),
            DialogKind::Open => {}
        }
        if dialog.multiple && dialog.kind != DialogKind::Save {
            arguments.push("--multiple".to_owned());
            arguments.push("--separator=\n".to_owned());
        }
        if let Some(path) = suggested_path(dialog) {
            arguments.push(format!("--filename={}", path.display()));
        }
        if dialog.kind != DialogKind::Folder {
            for filter in &dialog.filters {
                arguments.push(format!(
                    "--file-filter={} | {}",
                    filter.name,
                    patterns(filter).join(" ")
                ));
            }
        }
        arguments
    }

    fn zenity(dialog: &FileDialog) -> Result<Vec<PathBuf>, String> {
        let output = std::process::Command::new("zenity")
            .args(zenity_arguments(dialog))
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|error| format!("zenity could not run: {error}"))?;
        match output.status.code() {
            Some(0) => Ok(String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect()),
            // Cancelled or closed.
            Some(1) => Ok(Vec::new()),
            _ => Err(format!(
                "zenity failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn text_filter() -> FileFilter {
        FileFilter {
            name: "Text".into(),
            extensions: vec!["txt".into(), ".md".into()],
        }
    }

    #[test]
    fn filters_become_glob_patterns() {
        assert_eq!(patterns(&text_filter()), ["*.txt", "*.md"]);
        let any = FileFilter {
            name: "All".into(),
            extensions: Vec::new(),
        };
        assert_eq!(patterns(&any), ["*"]);
    }

    #[test]
    fn a_save_dialog_suggests_its_folder_and_name() {
        let mut dialog = FileDialog::new(DialogKind::Save);
        assert_eq!(suggested_path(&dialog), None);
        dialog.file_name = Some("notes.txt".into());
        assert_eq!(suggested_path(&dialog), Some(PathBuf::from("notes.txt")));
        dialog.directory = Some(PathBuf::from("base"));
        assert_eq!(
            suggested_path(&dialog),
            Some(Path::new("base").join("notes.txt"))
        );
        dialog.file_name = None;
        let folder = suggested_path(&dialog).unwrap();
        assert!(folder
            .as_os_str()
            .to_string_lossy()
            .ends_with(std::path::MAIN_SEPARATOR));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn portal_uris_decode_to_local_paths() {
        use super::linux::path_from_uri;
        assert_eq!(
            path_from_uri("file:///home/a%20b/%C3%BC.txt"),
            Some(PathBuf::from("/home/a b/ü.txt"))
        );
        assert_eq!(
            path_from_uri("file://localhost/tmp/x"),
            Some(PathBuf::from("/tmp/x"))
        );
        assert_eq!(path_from_uri("https://example.com/x"), None);
        assert_eq!(path_from_uri("file:///bad%2"), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn portal_requests_carry_the_dialog_options() {
        use super::linux::{parent_window, request};
        use zbus::zvariant::Value;
        let mut dialog = FileDialog::new(DialogKind::Open);
        dialog.multiple = true;
        dialog.directory = Some(PathBuf::from("/tmp"));
        dialog.filters = vec![text_filter()];
        let (method, options) = request(&dialog, "token");
        assert_eq!(method, "OpenFile");
        assert_eq!(options["handle_token"], Value::from("token"));
        assert_eq!(options["multiple"], Value::from(true));
        assert_eq!(options["directory"], Value::from(false));
        assert_eq!(options["current_folder"], Value::from(b"/tmp\0".to_vec()));
        let filters = vec![(
            "Text".to_owned(),
            vec![(0u32, "*.txt".to_owned()), (0u32, "*.md".to_owned())],
        )];
        assert_eq!(options["filters"], Value::from(filters));

        let mut dialog = FileDialog::new(DialogKind::Save);
        dialog.file_name = Some("a.txt".into());
        let (method, options) = request(&dialog, "token");
        assert_eq!(method, "SaveFile");
        assert_eq!(options["current_name"], Value::from("a.txt"));
        assert!(!options.contains_key("multiple"));

        let (_, options) = request(&FileDialog::new(DialogKind::Folder), "token");
        assert_eq!(options["directory"], Value::from(true));

        assert_eq!(parent_window(None), "");
        assert_eq!(
            parent_window(Some(NativeWindow::x11_for_test(0x2a))),
            "x11:2a"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn zenity_arguments_follow_the_dialog() {
        use super::linux::zenity_arguments;
        let mut dialog = FileDialog::new(DialogKind::Save);
        dialog.title = Some("Save notes".into());
        dialog.directory = Some(PathBuf::from("/tmp"));
        dialog.file_name = Some("a.txt".into());
        dialog.filters = vec![text_filter()];
        assert_eq!(
            zenity_arguments(&dialog),
            [
                "--file-selection",
                "--title=Save notes",
                "--save",
                "--filename=/tmp/a.txt",
                "--file-filter=Text | *.txt *.md",
            ]
        );
        let mut dialog = FileDialog::new(DialogKind::Folder);
        dialog.multiple = true;
        dialog.filters = vec![text_filter()];
        assert_eq!(
            zenity_arguments(&dialog),
            [
                "--file-selection",
                "--directory",
                "--multiple",
                "--separator=\n"
            ]
        );
    }
}
