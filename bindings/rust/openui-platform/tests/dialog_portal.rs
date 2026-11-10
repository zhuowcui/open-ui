//! Linux file dialogs speak the XDG desktop portal's file chooser protocol,
//! and fall back to zenity where no portal answers.
//!
//! A stand-in portal on the session bus records each request and answers it.
//! Run it on a private bus (`dbus-run-session`) so it cannot meet a real
//! portal; without a free bus it reports a skip, unless
//! `OPENUI_REQUIRE_SESSION_BUS` is set.

#[cfg(target_os = "linux")]
mod linux {
    use openui_platform::dialog::{show, DialogKind, FileDialog, FileFilter};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use zbus::message::Header;
    use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

    #[derive(Debug)]
    struct Call {
        method: &'static str,
        parent: String,
        title: String,
        options: HashMap<String, OwnedValue>,
    }

    struct Portal {
        calls: Arc<Mutex<Vec<Call>>>,
    }

    impl Portal {
        /// Record the call and answer it as its title asks.
        async fn answer(
            &self,
            method: &'static str,
            header: Header<'_>,
            connection: &zbus::Connection,
            parent: String,
            title: String,
            options: HashMap<String, OwnedValue>,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            if title == "no chooser" {
                return Err(zbus::fdo::Error::UnknownMethod(
                    "no FileChooser here".into(),
                ));
            }
            let sender = header.sender().expect("calls have a sender").to_owned();
            let token: String = options["handle_token"]
                .try_clone()
                .unwrap()
                .try_into()
                .unwrap();
            let path = format!(
                "/org/freedesktop/portal/desktop/request/{}/{token}",
                sender.trim_start_matches(':').replace('.', "_")
            );
            let (code, uris): (u32, Vec<&str>) = match title.as_str() {
                "cancel" => (1, vec![]),
                "end" => (2, vec![]),
                "remote" => (0, vec!["sftp://host/x"]),
                _ => (
                    0,
                    match method {
                        "SaveFile" => vec!["file:///tmp/saved%20%C3%BC.txt"],
                        _ => vec!["file:///tmp/a.txt", "file://localhost/tmp/b%23.md"],
                    },
                ),
            };
            self.calls.lock().unwrap().push(Call {
                method,
                parent,
                title,
                options,
            });
            let mut results: HashMap<&str, Value> = HashMap::new();
            results.insert("uris", Value::from(uris));
            connection
                .emit_signal(
                    Some(sender.clone()),
                    path.as_str(),
                    "org.freedesktop.portal.Request",
                    "Response",
                    &(code, results),
                )
                .await
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
            Ok(OwnedObjectPath::try_from(path).unwrap())
        }
    }

    #[zbus::interface(name = "org.freedesktop.portal.FileChooser")]
    impl Portal {
        async fn open_file(
            &self,
            #[zbus(header)] header: Header<'_>,
            #[zbus(connection)] connection: &zbus::Connection,
            parent_window: String,
            title: String,
            options: HashMap<String, OwnedValue>,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            self.answer(
                "OpenFile",
                header,
                connection,
                parent_window,
                title,
                options,
            )
            .await
        }

        async fn save_file(
            &self,
            #[zbus(header)] header: Header<'_>,
            #[zbus(connection)] connection: &zbus::Connection,
            parent_window: String,
            title: String,
            options: HashMap<String, OwnedValue>,
        ) -> zbus::fdo::Result<OwnedObjectPath> {
            self.answer(
                "SaveFile",
                header,
                connection,
                parent_window,
                title,
                options,
            )
            .await
        }
    }

    fn option<T>(call: &Call, name: &str) -> T
    where
        T: TryFrom<OwnedValue>,
        T::Error: std::fmt::Debug,
    {
        call.options[name].try_clone().unwrap().try_into().unwrap()
    }

    fn dialog(kind: DialogKind, title: &str) -> FileDialog {
        let mut dialog = FileDialog::new(kind);
        dialog.title = Some(title.into());
        dialog
    }

    pub fn run() -> Result<(), String> {
        if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
            return Err("no session bus".into());
        }
        let calls = Arc::new(Mutex::new(Vec::new()));
        let _portal = zbus::blocking::connection::Builder::session()
            .and_then(|builder| builder.name("org.freedesktop.portal.Desktop"))
            .and_then(|builder| {
                builder.serve_at(
                    "/org/freedesktop/portal/desktop",
                    Portal {
                        calls: calls.clone(),
                    },
                )
            })
            .and_then(|builder| builder.build())
            .map_err(|error| format!("the portal name is not free ({error})"))?;

        // Open several files of some types, starting in a folder.
        let mut open = dialog(DialogKind::Open, "open");
        open.multiple = true;
        open.directory = Some(PathBuf::from("/tmp"));
        open.filters = vec![FileFilter {
            name: "Text".into(),
            extensions: vec!["txt".into(), "md".into()],
        }];
        let chosen = show(&open, None).expect("the portal answered");
        assert_eq!(
            chosen,
            [PathBuf::from("/tmp/a.txt"), PathBuf::from("/tmp/b#.md")]
        );
        {
            let calls = calls.lock().unwrap();
            let call = calls.last().unwrap();
            assert_eq!(call.method, "OpenFile");
            assert_eq!(call.parent, "");
            assert_eq!(call.title, "open");
            assert!(option::<bool>(call, "multiple"));
            assert!(!option::<bool>(call, "directory"));
            assert!(option::<bool>(call, "modal"));
            assert_eq!(option::<Vec<u8>>(call, "current_folder"), b"/tmp\0");
            let filters: Vec<(String, Vec<(u32, String)>)> = option(call, "filters");
            assert_eq!(
                filters,
                [(
                    "Text".to_owned(),
                    vec![(0, "*.txt".to_owned()), (0, "*.md".to_owned())]
                )]
            );
        }
        println!("dialog_portal: open ok");

        // Save under a suggested name.
        let mut save = dialog(DialogKind::Save, "save");
        save.file_name = Some("saved ü.txt".into());
        let chosen = show(&save, None).expect("the portal answered");
        assert_eq!(chosen, [PathBuf::from("/tmp/saved ü.txt")]);
        {
            let calls = calls.lock().unwrap();
            let call = calls.last().unwrap();
            assert_eq!(call.method, "SaveFile");
            assert_eq!(option::<String>(call, "current_name"), "saved ü.txt");
            assert!(!call.options.contains_key("multiple"));
        }
        println!("dialog_portal: save ok");

        // Pick a folder.
        show(&dialog(DialogKind::Folder, "folder"), None).expect("the portal answered");
        assert!(option::<bool>(
            calls.lock().unwrap().last().unwrap(),
            "directory"
        ));
        println!("dialog_portal: folder ok");

        // Cancelling chooses nothing; other endings and remote files fail.
        let cancelled = show(&dialog(DialogKind::Open, "cancel"), None).expect("answered");
        assert!(cancelled.is_empty(), "{cancelled:?}");
        let ended = show(&dialog(DialogKind::Open, "end"), None).unwrap_err();
        assert!(ended.to_string().contains("response 2"), "{ended}");
        let remote = show(&dialog(DialogKind::Open, "remote"), None).unwrap_err();
        assert!(remote.to_string().contains("non-local"), "{remote}");
        println!("dialog_portal: cancel and failures ok");

        // Without a file chooser, zenity answers.
        let bin = std::env::temp_dir().join(format!("openui-zenity-{}", std::process::id()));
        std::fs::create_dir_all(&bin).unwrap();
        let log = bin.join("arguments");
        let script = bin.join("zenity");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nprintf '/tmp/one\\n/tmp/two\\n'\n",
                log.display()
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&script).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
        std::fs::set_permissions(&script, permissions).unwrap();
        let path = std::env::var_os("PATH").unwrap_or_default();
        let mut paths = vec![bin.clone()];
        paths.extend(std::env::split_paths(&path));
        // Single-threaded here apart from zbus's own threads, which do not
        // read PATH.
        std::env::set_var("PATH", std::env::join_paths(paths).unwrap());
        let mut fallback = dialog(DialogKind::Open, "no chooser");
        fallback.multiple = true;
        let chosen = show(&fallback, None).expect("zenity answered");
        assert_eq!(
            chosen,
            [PathBuf::from("/tmp/one"), PathBuf::from("/tmp/two")]
        );
        let arguments = std::fs::read_to_string(&log).unwrap();
        assert!(arguments.contains("--file-selection\n--title=no chooser\n--multiple\n"));
        std::env::set_var("PATH", path);
        let _ = std::fs::remove_dir_all(&bin);
        println!("dialog_portal: zenity fallback ok");
        Ok(())
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    if let Err(reason) = linux::run() {
        if std::env::var_os("OPENUI_REQUIRE_SESSION_BUS").is_some() {
            eprintln!("dialog_portal: {reason}, but OPENUI_REQUIRE_SESSION_BUS is set");
            std::process::exit(1);
        }
        println!("dialog_portal: skipped ({reason}; run under dbus-run-session)");
    }
    #[cfg(not(target_os = "linux"))]
    println!("dialog_portal: skipped (Linux only)");
}
