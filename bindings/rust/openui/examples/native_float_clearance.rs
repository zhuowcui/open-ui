use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};
fn snapshot(
    document: &Document,
    nodes: &[(String, Element)],
    out: &std::path::Path,
    stage: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let values=nodes.iter().map(|(name,e)| {let b=e.bounding_rect()?.ok_or("missing bounds")?;Ok(serde_json::json!({"name":name,"bounds":{"x":b.x,"y":b.y,"width":b.width,"height":b.height}}))}).collect::<Result<Vec<_>,Box<dyn std::error::Error>>>()?;
    let png = document.render_to_png_buffer()?;
    assert_eq!(png, document.render_to_png_buffer()?);
    std::fs::write(out.join(format!("{stage}.png")), png)?;
    std::fs::write(
        out.join(format!("{stage}.json")),
        serde_json::to_vec_pretty(&values)?,
    )?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("output")?);
    let width: f64 = args.next().ok_or("width")?.parse()?;
    let height: f64 = args.next().ok_or("height")?.parse()?;
    let scale: f64 = args.next().ok_or("scale")?.parse()?;
    std::fs::create_dir_all(&out)?;
    let document = Document::with_options(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let mut nodes = Vec::new();
    let mut targets = Vec::new();
    for i in 0..8 {
        let host = Element::create(&document, "div")?;
        root.append_child(&host)?;
        host.set_position(Position::Absolute)?;
        host.set_left(Length::px(20.0 + (i % 4) as f32 * 170.0))?;
        host.set_top(Length::px(20.0 + (i / 4) as f32 * 280.0))?;
        host.set_width(LengthValue::px(150.0))?;
        let outer = Element::create(&document, "div")?;
        host.append_child(&outer)?;
        outer.set_display(Display::InlineBlock)?;
        outer.set_background_color(Color::RED)?;
        nodes.push((format!("outer-{i}"), outer.clone()));
        let float = Element::create(&document, "div")?;
        outer.append_child(&float)?;
        float.set_width(LengthValue::px(100.0))?;
        float.set_height(LengthValue::px(100.0))?;
        float.set_float(Float::Left)?;
        float.set_background_color(Color::from_rgba8(0, 128, 0, 255))?;
        nodes.push((format!("float-{i}"), float));
        let clear = Element::create(&document, "div")?;
        outer.append_child(&clear)?;
        clear.set_clear(Clear::Both)?;
        nodes.push((format!("clear-{i}"), clear.clone()));
        let fit = if i % 4 < 2 {
            let fit = Element::create(&document, "div")?;
            clear.append_child(&fit)?;
            fit
        } else {
            clear.clone()
        };
        fit.set_width(LengthValue::from(Length::fit_content()))?;
        if i >= 4 {
            fit.set_min_width(LengthValue::px(20.0))?;
        }
        nodes.push((format!("fit-{i}"), fit.clone()));
        let padding = Element::create(&document, "div")?;
        fit.append_child(&padding)?;
        padding.set_padding_top(LengthValue::percent(100.0))?;
        padding.set_margin_bottom(LengthValue::px(if i % 2 == 0 { 100.0 } else { 75.0 }))?;
        nodes.push((format!("padding-{i}"), padding.clone()));
        targets.push(padding.downgrade());
    }
    let count = Rc::new(Cell::new(0));
    let called = count.clone();
    root.on("click", move |_| {
        for target in &targets {
            target
                .upgrade()
                .unwrap()
                .set_margin_bottom(LengthValue::px(0.0))
                .unwrap();
        }
        called.set(called.get() + 1);
    })?;
    snapshot(&document, &nodes, &out, "before")?;
    root.click()?;
    assert_eq!(count.get(), 1);
    snapshot(&document, &nodes, &out, "after")?;
    let weak = nodes[0].1.downgrade();
    drop(nodes);
    drop(root);
    drop(document);
    assert!(weak.upgrade().is_none());
    std::fs::write(
        out.join("lifecycle.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"callbacks":count.get(),"weak_teardown_passed":true,"javascript_executed_by_openui":false}),
        )?,
    )?;
    Ok(())
}
