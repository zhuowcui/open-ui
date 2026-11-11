use openui::prelude::*;

fn render_preview() -> Result<(), Error> {
    // A 600x400 page at device scale 2, drawn without a window.
    let mut app = HeadlessApp::new(ViewportMetrics::from_logical_size(600.0, 400.0, 2.0)?)?;
    app.mount(|| view! { <p style:padding="24px">"Hello from a headless render"</p> })?;
    app.render_png_to(0.0, "preview.png")?;
    Ok(())
}
