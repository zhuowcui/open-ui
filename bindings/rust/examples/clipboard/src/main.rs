//! Reading and writing the system clipboard's text.
//!
//! "Copy greeting" writes a line to the clipboard; "Show clipboard" reads
//! whatever text it holds, including text copied in another program.
//!
//! Set `OUI_SMOKE_EXIT=1` to write, read back and restore the clipboard, print
//! the result and exit, as CI does.

use openui::clipboard;
use openui::prelude::*;
use openui::window::WindowSpec;

fn app_view() -> ViewNode {
    let shown = create_signal(String::from("(press Show clipboard)"));
    let copies = create_signal(0_u32);

    if std::env::var_os("OUI_SMOKE_EXIT").is_some() {
        let previous = clipboard::read_text().ok();
        clipboard::write_text("Open UI clipboard example");
        let read = clipboard::read_text();
        if let Some(previous) = previous {
            clipboard::write_text(previous);
        }
        println!("clipboard: read back {read:?}");
        openui::app::exit();
    }

    view! {
        <div style:font-family="sans-serif" style:padding="16px">
            <button style:padding="6px 14px" style:margin="4px" on:click={move |_| {
                copies.update(|count| *count += 1);
                clipboard::write_text(format!("Hello from Open UI ({})", copies.get()));
            }}>"Copy greeting"</button>
            <button style:padding="6px 14px" style:margin="4px" on:click={move |_| {
                shown.set(clipboard::read_text().unwrap_or_else(|error| error.to_string()));
            }}>"Show clipboard"</button>
            <p>{format!("Copied {} time(s).", copies.get())}</p>
            <p style:white-space="pre-wrap">{shown.get()}</p>
        </div>
    }
}

#[cfg(any(feature = "linux", feature = "windows"))]
fn main() -> Result<(), Error> {
    App::new("dev.openui.examples.clipboard")
        .name("Clipboard")
        .window(WindowSpec::new("main").size(420.0, 220.0), app_view)
        .run()
}

#[cfg(not(any(feature = "linux", feature = "windows")))]
fn main() -> Result<(), Error> {
    let mut app = HeadlessApp::new(ViewportMetrics::from_logical_size(420.0, 220.0, 1.0)?)?;
    app.mount(app_view)?;
    app.render_png_to(0.0, "clipboard.png")?;
    println!("Rendered clipboard.png");
    Ok(())
}
