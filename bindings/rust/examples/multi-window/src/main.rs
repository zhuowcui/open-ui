//! Several windows sharing one signal.
//!
//! The main window opens note windows (each with a fresh key) and a single
//! settings window (opening it again focuses it). Every window reads and
//! writes the same `edits` signal, and the settings window can refuse to
//! close.
//!
//! Set `OUI_SMOKE_EXIT=1` to open a note and the settings window and exit at
//! once, as CI does.

use openui::prelude::*;
use openui::window::{self, Close, WindowKey, WindowSpec};

fn main_view(edits: Signal<u32>) -> ViewNode {
    let notes = create_signal(0_u32);
    let open_note = move || {
        notes.set(notes.get() + 1);
        let number = notes.get();
        window::open(
            WindowSpec::new(WindowKey::unique())
                .title(format!("Note {number}"))
                .size(360.0, 220.0),
            move || note_view(number, edits),
        );
    };
    let open_settings = move || {
        let allow_close = create_signal(true);
        window::open(
            WindowSpec::new("settings")
                .title("Settings")
                .size(320.0, 200.0)
                .resizable(false)
                .on_close_requested(move || {
                    if allow_close.get() {
                        Close::Allow
                    } else {
                        Close::Prevent
                    }
                }),
            move || settings_view(allow_close),
        );
    };
    if std::env::var_os("OUI_SMOKE_EXIT").is_some() {
        open_note();
        open_settings();
        openui::app::exit();
    }
    view! {
        <div style:font-family="sans-serif" style:padding="16px">
            <h1 style:font-size="20px">"Windows"</h1>
            <p>{format!("Notes opened: {}. Edits in all windows: {}", notes.get(), edits.get())}</p>
            <button style:padding="6px 14px" style:margin="4px" style:font-size="14px" on:click={move |_| open_note()}>"Open a note"</button>
            <button style:padding="6px 14px" style:margin="4px" style:font-size="14px" on:click={move |_| open_settings()}>"Open settings"</button>
        </div>
    }
}

fn note_view(number: u32, edits: Signal<u32>) -> ViewNode {
    view! {
        <div style:font-family="sans-serif" style:padding="16px">
            <h2 style:font-size="18px">{format!("Note {number}")}</h2>
            <p>{format!("Edits in all windows: {}", edits.get())}</p>
            <button style:padding="6px 14px" style:margin="4px" style:font-size="14px" on:click={move |_| edits.set(edits.get() + 1)}>"Edit"</button>
            <button style:padding="6px 14px" style:margin="4px" style:font-size="14px" on:click={move |_| {
                if let Some(this) = window::current() {
                    this.close();
                }
            }}>"Close"</button>
        </div>
    }
}

fn settings_view(allow_close: Signal<bool>) -> ViewNode {
    view! {
        <div style:font-family="sans-serif" style:padding="16px">
            <p>{if allow_close.get() { "This window may close." } else { "This window refuses to close." }}</p>
            <button style:padding="6px 14px" style:margin="4px" style:font-size="14px" on:click={move |_| allow_close.set(!allow_close.get())}>
                "Toggle"
            </button>
        </div>
    }
}

#[cfg(any(feature = "linux", feature = "windows"))]
fn main() -> Result<(), Error> {
    let edits = create_signal(0_u32);
    App::new("dev.openui.examples.multi-window")
        .name("Windows")
        .window(WindowSpec::new("main").size(480.0, 240.0), move || {
            main_view(edits)
        })
        .run()
}

#[cfg(not(any(feature = "linux", feature = "windows")))]
fn main() -> Result<(), Error> {
    let mut app = HeadlessApp::new(ViewportMetrics::from_logical_size(480.0, 240.0, 1.0)?)?;
    let edits = create_signal(0_u32);
    app.mount(move || main_view(edits))?;
    app.render_png_to(0.0, "multi-window.png")?;
    println!("Rendered multi-window.png");
    Ok(())
}
