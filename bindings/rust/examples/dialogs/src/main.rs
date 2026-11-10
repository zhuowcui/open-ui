//! Native open, save and folder dialogs.
//!
//! "Open..." reads a text file into the note, "Save as..." writes the note
//! to a file, and "Choose folders..." lists the folders chosen. The window
//! keeps drawing while a dialog is open; the clock shows it.
//!
//! Set `OUI_SMOKE_EXIT=1` to start and exit without showing a dialog, as CI
//! does; the `openui-platform` dialog tests answer real dialogs.

use openui::prelude::*;

#[cfg(any(feature = "linux", feature = "windows"))]
fn app_view() -> ViewNode {
    use openui::dialog;
    use openui::task::{spawn_blocking, spawn_local, time};
    use std::time::Duration;

    let note = create_signal(String::from("Hello from Open UI.\n"));
    let status = create_signal(String::from("Choose a file or folder."));
    let seconds = create_signal(0_u64);
    spawn_local(async move {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            seconds.update(|s| *s += 1);
        }
    })
    .detach();

    if std::env::var_os("OUI_SMOKE_EXIT").is_some() {
        println!("dialogs: started");
        openui::app::exit();
    }

    let open = move |_: &Event| {
        let chosen = dialog::open_file()
            .title("Open a text file")
            .filter("Text", &["txt", "md"])
            .filter("All files", &[])
            .pick();
        spawn_local(async move {
            match chosen.await {
                Ok(Some(path)) => {
                    let read = spawn_blocking({
                        let path = path.clone();
                        move || std::fs::read_to_string(path)
                    })
                    .await;
                    match read {
                        Ok(text) => {
                            note.set(text);
                            status.set(format!("Opened {}", path.display()));
                        }
                        Err(error) => status.set(format!("Could not read it: {error}")),
                    }
                }
                Ok(None) => status.set("Cancelled.".into()),
                Err(error) => status.set(error.to_string()),
            }
        })
        .detach();
    };
    let save = move |_: &Event| {
        let chosen = dialog::save_file()
            .title("Save the note")
            .file_name("note.txt")
            .filter("Text", &["txt"])
            .pick();
        spawn_local(async move {
            match chosen.await {
                Ok(Some(path)) => {
                    let text = note.get();
                    let written = spawn_blocking({
                        let path = path.clone();
                        move || std::fs::write(path, text)
                    })
                    .await;
                    status.set(match written {
                        Ok(()) => format!("Saved {}", path.display()),
                        Err(error) => format!("Could not save it: {error}"),
                    });
                }
                Ok(None) => status.set("Cancelled.".into()),
                Err(error) => status.set(error.to_string()),
            }
        })
        .detach();
    };
    let folders = move |_: &Event| {
        let chosen = dialog::pick_folder().title("Choose folders").pick_many();
        spawn_local(async move {
            status.set(match chosen.await {
                Ok(paths) if paths.is_empty() => "Cancelled.".into(),
                Ok(paths) => paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
                Err(error) => error.to_string(),
            });
        })
        .detach();
    };

    view! {
        <div style:font-family="sans-serif" style:padding="16px">
            <button style:padding="6px 14px" style:margin="4px" on:click={open}>"Open..."</button>
            <button style:padding="6px 14px" style:margin="4px" on:click={save}>"Save as..."</button>
            <button style:padding="6px 14px" style:margin="4px" on:click={folders}>"Choose folders..."</button>
            <p style:white-space="pre-wrap">{status.get()}</p>
            <p style:color="#666">{format!("Running for {} s", seconds.get())}</p>
            <p style:white-space="pre-wrap" style:font-family="monospace" style:border="1px solid #ccc" style:padding="8px">{note.get()}</p>
        </div>
    }
}

#[cfg(any(feature = "linux", feature = "windows"))]
fn main() -> Result<(), Error> {
    App::new("dev.openui.examples.dialogs")
        .name("Dialogs")
        .window(
            openui::window::WindowSpec::new("main").size(520.0, 360.0),
            app_view,
        )
        .run()
}

#[cfg(not(any(feature = "linux", feature = "windows")))]
fn main() {
    println!("dialogs: build with --features linux or --features windows");
}
