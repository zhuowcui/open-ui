//! Hello World — static content with styled layout.
//!
//! Demonstrates the basics of Open UI: creating an application, using the
//! `view!` macro for declarative markup, and rendering to a PNG image.
//! No reactive signals are used here — this is purely static content.

use openui::prelude::*;

fn application_view() -> ViewNode {
    view! {
            <div
                style:display="flex"
                style:flex-direction="column"
                style:align-items="center"
                style:justify-content="center"
                style:min-height="100vh"
                style:background-color="#f0f4f8"
                style:font-family="sans-serif"
            >
                // Main heading
                <h1 style:font-size="48px" style:color="#1a202c" style:margin-bottom="8px">
                    "Hello, Open UI!"
                </h1>

                // Subtitle
                <p style:font-size="20px" style:color="#4a5568" style:margin-top="0">
                    "A reactive UI framework powered by pure Rust"
                </p>

                // Horizontal rule
                <hr style:width="300px" style:border="none"
                    style:border-top="2px solid #cbd5e0" style:margin="24px" />

                // Feature highlights
                <div style:display="flex" style:gap="32px" style:margin-top="8px">
                    <span style:color="#2b6cb0" style:font-size="16px">
                        "Declarative"
                    </span>
                    <span style:color="#2f855a" style:font-size="16px">
                        "Reactive"
                    </span>
                    <span style:color="#9b2c2c" style:font-size="16px">
                        "Fast"
                    </span>
                </div>

                // Footer
                <p style:font-size="12px" style:color="#a0aec0" style:margin-top="48px">
                    "Built with Rust + Skia"
                </p>
            </div>
    }
}

#[cfg(feature = "linux")]
fn main() -> Result<(), Error> {
    let backend = match std::env::var("OUI_BACKEND").as_deref() {
        Ok("opengl") => BackendPreference::OpenGl,
        Ok("software") => BackendPreference::Software,
        _ => BackendPreference::Auto,
    };
    let app = App::builder()
        .title("Hello, Open UI!")
        .size(LogicalSize::new(800.0, 600.0))
        .backend(backend)
        .build()?;
    if std::env::var_os("OUI_SMOKE_EXIT").is_some() {
        app.request_exit();
    }
    app.run(application_view)
}

#[cfg(not(feature = "linux"))]
fn main() -> Result<(), Error> {
    let mut app = HeadlessApp::new(Viewport::new(800, 600)?)?;
    app.mount(application_view)?;
    app.render_png_to(0.0, "hello.png")?;

    println!("Rendered hello.png");
    Ok(())
}
