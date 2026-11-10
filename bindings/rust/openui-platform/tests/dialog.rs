//! Native file dialogs return what the user chose.
//!
//! On Windows each test shows the real common item dialog on its own thread
//! and answers it by posting OK or Cancel to it, as a click would. Linux
//! dialogs come from another process (the desktop portal or zenity); the
//! `dialog_portal` test covers them against a stand-in portal.

#[cfg(target_os = "windows")]
mod windows {
    use openui_platform::dialog::{show, DialogKind, FileDialog, FileFilter};
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumThreadWindows, GetClassNameW, IsWindowVisible, PostMessageW, IDCANCEL, IDOK, WM_COMMAND,
    };

    /// The visible dialog window owned by `thread`, if it has one yet.
    fn dialog_of(thread: u32) -> Option<HWND> {
        unsafe extern "system" fn visit(window: HWND, found: LPARAM) -> i32 {
            let mut class = [0u16; 16];
            // SAFETY: `class` holds 16 UTF-16 units and `found` points at the
            // `Option<HWND>` below for the whole enumeration.
            unsafe {
                let length = GetClassNameW(window, class.as_mut_ptr(), class.len() as i32);
                if IsWindowVisible(window) != 0
                    && String::from_utf16_lossy(&class[..length as usize]) == "#32770"
                {
                    *(found as *mut Option<HWND>) = Some(window);
                    return 0;
                }
            }
            1
        }
        let mut found: Option<HWND> = None;
        // SAFETY: `visit` only writes through the pointer to `found`, which
        // outlives the call.
        unsafe { EnumThreadWindows(thread, Some(visit), &mut found as *mut _ as LPARAM) };
        found
    }

    /// Show `dialog` on a new thread, press `button` once it appears, and
    /// return what it chose.
    fn answer(dialog: FileDialog, button: i32) -> Vec<PathBuf> {
        let (thread_sender, thread_receiver) = mpsc::channel();
        let shown = std::thread::spawn(move || {
            // SAFETY: GetCurrentThreadId has no preconditions.
            thread_sender.send(unsafe { GetCurrentThreadId() }).unwrap();
            show(&dialog, None)
        });
        let thread = thread_receiver.recv().unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let window = loop {
            if let Some(window) = dialog_of(thread) {
                break window;
            }
            assert!(Instant::now() < deadline, "the dialog never appeared");
            std::thread::sleep(Duration::from_millis(50));
        };
        // Let the dialog finish populating before answering it.
        std::thread::sleep(Duration::from_millis(500));
        // SAFETY: posting a message to a window has no memory-safety
        // requirements; a stale handle only makes the call fail.
        let posted = unsafe { PostMessageW(window, WM_COMMAND, button as usize, 0) };
        assert_ne!(posted, 0, "posting to the dialog failed");
        shown
            .join()
            .expect("the dialog thread panicked")
            .expect("the dialog ran")
    }

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("openui-dialog-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn same_file(left: &Path, right: &Path) -> bool {
        std::fs::canonicalize(left).ok() == std::fs::canonicalize(right).ok()
    }

    pub fn save_returns_the_suggested_path() {
        let directory = scratch("save");
        let mut dialog = FileDialog::new(DialogKind::Save);
        dialog.title = Some("Open UI save test".into());
        dialog.directory = Some(directory.clone());
        dialog.file_name = Some("notes ü.txt".into());
        dialog.filters = vec![FileFilter {
            name: "Text".into(),
            extensions: vec!["txt".into()],
        }];
        let chosen = answer(dialog, IDOK);
        assert_eq!(chosen.len(), 1, "{chosen:?}");
        assert_eq!(chosen[0].file_name().unwrap(), "notes ü.txt");
        assert!(
            same_file(chosen[0].parent().unwrap(), &directory),
            "{chosen:?}"
        );
        println!("save: ok ({})", chosen[0].display());
    }

    pub fn open_returns_the_named_file() {
        let directory = scratch("open");
        let file = directory.join("existing.md");
        std::fs::write(&file, "x").unwrap();
        let mut dialog = FileDialog::new(DialogKind::Open);
        dialog.directory = Some(directory.clone());
        dialog.file_name = Some("existing.md".into());
        dialog.multiple = true;
        let chosen = answer(dialog, IDOK);
        assert_eq!(chosen.len(), 1, "{chosen:?}");
        assert!(same_file(&chosen[0], &file), "{chosen:?}");
        println!("open: ok ({})", chosen[0].display());
    }

    pub fn folder_returns_the_current_folder() {
        let directory = scratch("folder");
        let mut dialog = FileDialog::new(DialogKind::Folder);
        dialog.directory = Some(directory.clone());
        let chosen = answer(dialog, IDOK);
        assert_eq!(chosen.len(), 1, "{chosen:?}");
        assert!(same_file(&chosen[0], &directory), "{chosen:?}");
        println!("folder: ok ({})", chosen[0].display());
    }

    pub fn cancel_returns_nothing() {
        let chosen = answer(FileDialog::new(DialogKind::Open), IDCANCEL);
        assert!(chosen.is_empty(), "{chosen:?}");
        println!("cancel: ok");
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    {
        windows::save_returns_the_suggested_path();
        windows::open_returns_the_named_file();
        windows::folder_returns_the_current_folder();
        windows::cancel_returns_nothing();
    }
    #[cfg(not(target_os = "windows"))]
    println!("dialog: skipped (Linux dialogs are covered by dialog_portal)");
}
