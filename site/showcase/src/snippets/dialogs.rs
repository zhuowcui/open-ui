use openui::dialog;
use openui::prelude::*;
use openui::task::{spawn_blocking, spawn_local};

fn open_button() -> ViewNode {
    let text = create_signal(String::new());
    let status = create_signal(String::from("No file open."));
    let open = move |_: &Event| {
        // Build the dialog in the handler: it belongs to the current window.
        let chosen = dialog::open_file()
            .title("Open a text file")
            .filter("Text", &["txt", "md"])
            .filter("All files", &[])
            .pick();
        spawn_local(async move {
            match chosen.await {
                Ok(Some(path)) => {
                    let read = spawn_blocking(move || std::fs::read_to_string(path)).await;
                    match read {
                        Ok(contents) => text.set(contents),
                        Err(error) => status.set(error.to_string()),
                    }
                }
                Ok(None) => status.set("Cancelled.".into()),
                Err(error) => status.set(error.to_string()),
            }
        })
        .detach();
    };
    view! {
        <div>
            <button on:click={open}>"Open…"</button>
            <p>{status.get()}</p>
            <p>{text.get()}</p>
        </div>
    }
}

async fn save_as() -> Result<Option<std::path::PathBuf>, Error> {
    dialog::save_file()
        .title("Save note")
        .file_name("note.txt")
        .filter("Text", &["txt"])
        .pick()
        .await
}
