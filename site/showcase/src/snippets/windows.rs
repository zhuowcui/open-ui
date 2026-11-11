use openui::prelude::*;
use openui::window;

fn main() -> Result<(), Error> {
    App::new("com.example.editor")
        .name("Editor")
        .window(
            WindowSpec::new("main")
                .title("Editor")
                .size(900.0, 600.0)
                .min_size(480.0, 320.0),
            editor,
        )
        .run()
}

fn editor() -> ViewNode {
    // Signals are shared by every window of the app.
    let unsaved = create_signal(true);
    view! {
        <div style:padding="16px">
            <button on:click={move |_| open_settings(unsaved)}>"Settings"</button>
        </div>
    }
}

fn open_settings(unsaved: Signal<bool>) {
    // Opening a key that is already open focuses that window instead.
    window::open(
        WindowSpec::new("settings")
            .title("Settings")
            .size(360.0, 240.0)
            .resizable(false)
            .on_close_requested(move || {
                if unsaved.get() {
                    Close::Prevent
                } else {
                    Close::Allow
                }
            }),
        move || {
            view! {
                <div style:padding="16px">
                    <button on:click={move |_| unsaved.set(false)}>"Save"</button>
                    <button on:click={move |_| {
                        // close() skips the on_close_requested handler.
                        if let Some(this) = window::current() {
                            this.close();
                        }
                    }}>
                        "Close"
                    </button>
                </div>
            }
        },
    );
}
