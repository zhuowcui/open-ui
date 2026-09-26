//! A small application that exercises the public Open UI framework.
//!
//! The default headless run renders a PNG after sending a real pointer click
//! through the document. Build with `--features linux` and pass `--window` to
//! use the same view as an interactive desktop application.

use openui::prelude::*;
use std::path::Path;

fn application_view(count: Signal<i32>) -> ViewNode {
    view! {
        <div
            style:min-height="100vh"
            style:padding="32px"
            style:background-color="#eef2f7"
            style:font-family="sans-serif"
        >
            <h1 style:margin="0" style:font-size="32px" style:color="#1e293b">
                "Open UI test app"
            </h1>
            <p style:margin="12px 0 0" style:font-size="16px" style:color="#475569">
                "A reactive counter rendered with the public Rust API."
            </p>
            <div style:display="flex" style:gap="12px" style:margin-top="28px">
                <button
                    id="increment"
                    style:width="160px"
                    style:height="52px"
                    style:background-color="#2563eb"
                    style:color="white"
                    style:border="none"
                    style:border-radius="8px"
                    style:font-size="16px"
                    style:cursor="pointer"
                    on:click={move |_| count.update(|value| *value += 1)}
                >
                    "Add one"
                </button>
                <button
                    style:width="100px"
                    style:height="52px"
                    style:background-color="#cbd5e1"
                    style:color="#1e293b"
                    style:border="none"
                    style:border-radius="8px"
                    style:font-size="16px"
                    style:cursor="pointer"
                    on:click={move |_| count.set(0)}
                >
                    "Reset"
                </button>
            </div>
            <div
                style:width="320px"
                style:margin-top="18px"
                style:padding="24px"
                style:background-color="white"
                style:border="1px solid #cbd5e1"
                style:border-radius="10px"
            >
                <p style:margin="0" style:font-size="14px" style:color="#475569">
                    "Current count"
                </p>
                <p style:margin="0" style:font-size="48px" style:color="#1e293b">
                    {count.get()}
                </p>
            </div>
        </div>
    }
}

fn find_by_id(root: Element, id: &str) -> Result<Option<Element>, Error> {
    if root.get_attribute("id")?.as_deref() == Some(id) {
        return Ok(Some(root));
    }
    let mut child = root.first_child()?;
    while let Some(node) = child {
        if let Some(found) = find_by_id(node.clone(), id)? {
            return Ok(Some(found));
        }
        child = node.next_sibling()?;
    }
    Ok(None)
}

fn run_headless(path: &Path) -> Result<(), Error> {
    let count = create_signal(0_i32);
    let mut app = HeadlessApp::new(ViewportMetrics::from_logical_size(800.0, 600.0, 1.0)?)?;
    app.mount(move || application_view(count))?;

    let before = app.render_at(0.0)?;
    let button = find_by_id(app.document().body(), "increment")?
        .ok_or(Error::InvalidArgument("increment button was not mounted"))?;
    let bounds = button
        .bounding_rect()?
        .ok_or(Error::InvalidArgument("increment button has no bounds"))?;
    app.document().dispatch_mouse_event(
        MouseEventType::Up,
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
        MouseButton::Left,
        Modifiers::NONE,
    )?;
    let after = app.render_at(0.0)?;
    if count.get() != 1 || before.pixels() == after.pixels() {
        return Err(Error::InvalidArgument(
            "headless click did not update the rendered counter",
        ));
    }
    app.render_png_to(0.0, path)?;
    println!(
        "Click verified; rendered {} (800x600, count = 1)",
        path.display()
    );
    Ok(())
}

#[cfg(feature = "linux")]
fn run_window() -> Result<(), Error> {
    let count = create_signal(0_i32);
    let backend = match std::env::var("OUI_BACKEND").as_deref() {
        Ok("opengl") => BackendPreference::OpenGl,
        Ok("software") => BackendPreference::Software,
        _ => BackendPreference::Auto,
    };
    App::builder()
        .title("Open UI test app")
        .size(LogicalSize::new(800.0, 600.0))
        .backend(backend)
        .build()?
        .run(move || application_view(count))
}

fn main() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (None, None) => run_headless(Path::new("framework-test.png")),
        (Some("--headless"), Some(path)) => run_headless(Path::new(&path)),
        #[cfg(feature = "linux")]
        (Some("--window"), None) => run_window(),
        _ => Err(Error::InvalidArgument(
            "usage: framework-test [--headless PNG_PATH | --window (with linux feature)]",
        )),
    }
}
