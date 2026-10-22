//! Accessibility smoke — native controls that a platform accessibility API
//! can find, operate and observe.
//!
//! A button counts its activations in its label, so an assistive client can
//! observe the effect of an invoke through the button's accessible name. A
//! named checkbox and text field expose toggle and value state.

use openui::prelude::*;

fn application_view() -> ViewNode {
    let presses = create_signal(0_u32);

    view! {
        <div
            style:display="flex"
            style:flex-direction="column"
            style:align-items="flex-start"
            style:gap="12px"
            style:padding="24px"
            style:font-family="sans-serif"
        >
            <button on:click={move |_| presses.set(presses.get() + 1)}>
                {format!("Pressed {} times", presses.get())}
            </button>
            <input type="checkbox" aria-label="Remember choice" />
            <input type="text" aria-label="Name" />
        </div>
    }
}

#[cfg(any(feature = "linux", feature = "windows"))]
fn main() -> Result<(), Error> {
    App::builder()
        .title("Open UI accessibility")
        .size(LogicalSize::new(480.0, 320.0))
        .build()?
        .run(application_view)
}

#[cfg(not(any(feature = "linux", feature = "windows")))]
fn main() -> Result<(), Error> {
    let mut app = HeadlessApp::new(ViewportMetrics::from_logical_size(480.0, 320.0, 1.0)?)?;
    app.mount(application_view)?;
    app.render_png_to(0.0, "accessibility.png")?;
    println!("Rendered accessibility.png");
    Ok(())
}
