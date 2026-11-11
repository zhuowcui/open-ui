use openui::clipboard;
use openui::prelude::*;

fn share_link() -> ViewNode {
    let pasted = create_signal(String::new());
    view! {
        <div>
            <button on:click={move |_| clipboard::write_text("https://example.com/invite/42")}>
                "Copy invite link"
            </button>
            <button on:click={move |_| {
                pasted.set(clipboard::read_text().unwrap_or_else(|error| error.to_string()));
            }}>
                "Paste"
            </button>
            <p>{pasted.get()}</p>
        </div>
    }
}
