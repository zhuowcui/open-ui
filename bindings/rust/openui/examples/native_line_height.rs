//! Consuming native Rust app for retained font-relative declarations.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn snapshot(
    document: &Document,
    nodes: &[(String, Element)],
    output: &std::path::Path,
    stage: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = nodes
        .iter()
        .map(|(name, element)| {
            let rect = element.bounding_rect()?.ok_or("missing owned bounds")?;
            Ok(serde_json::json!({"name":name,"bounds":{
                "x":rect.x,"y":rect.y,"width":rect.width,"height":rect.height
            }}))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let png = document.render_to_png_buffer()?;
    assert_eq!(png, document.render_to_png_buffer()?);
    std::fs::write(output.join(format!("{stage}.png")), png)?;
    std::fs::write(
        output.join(format!("{stage}.json")),
        serde_json::to_vec_pretty(&geometry)?,
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let width: f64 = args.next().ok_or("width required")?.parse()?;
    let height: f64 = args.next().ok_or("height required")?.parse()?;
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )?;
    let canvas = document.body();
    canvas.set_font_size(LengthValue::px(16.0))?;
    canvas.set_background_color(Color::WHITE)?;
    let root = Element::create(&document, "div")?;
    canvas.append_child(&root)?;
    root.set_margin(Edges::all(LengthValue::px(0.0)))?;
    root.set_padding(Edges::all(LengthValue::px(0.0)))?;
    root.set_width(LengthValue::ViewportWidth(100.0))?;
    root.set_height(LengthValue::ViewportHeight(100.0))?;
    root.set_overflow(Overflow::Hidden)?;
    root.set_background_color(Color::WHITE)?;
    root.set_font_family(FontFamilyList::single("Ahem"))?;
    root.set_font_size(LengthValue::px(18.72))?;
    root.set_line_height(LineHeight::Number(1.25))?;
    let mut nodes = Vec::new();
    for (index, (name, line_height)) in [
        ("ch", LengthValue::Ch(2.0)),
        ("ex", LengthValue::Ex(2.0)),
        ("lh", LengthValue::Lh(2.0)),
        ("em", LengthValue::Em(1.5)),
        ("rem", LengthValue::Rem(1.5)),
        ("percentage", LengthValue::percent(127.5)),
    ]
    .into_iter()
    .enumerate()
    {
        let element = Element::create(&document, "div")?;
        root.append_child(&element)?;
        element.set_position(Position::Absolute)?;
        element.set_left(Length::px(10.0))?;
        element.set_top(Length::px(10.0 + index as f32 * 25.0))?;
        element.set_height(LengthValue::px(10.0))?;
        // Retain the declaration before setting the final line-height context.
        element.set_width(LengthValue::Lh(2.5))?;
        element.set_font_size(LengthValue::px(30.41))?;
        element.set_line_height_length(line_height)?;
        element.set_background_color(Color::BLACK)?;
        nodes.push((name.to_owned(), element));
    }
    let target = root.downgrade();
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    nodes[0].1.on("click", move |_| {
        target
            .upgrade()
            .expect("live native root")
            .set_font_size(LengthValue::px(20.37))
            .expect("native font mutation");
        called.set(called.get() + 1);
    })?;
    std::fs::create_dir_all(&output)?;
    snapshot(&document, &nodes, &output, "before")?;
    nodes[0].1.click()?;
    snapshot(&document, &nodes, &output, "after")?;
    assert_eq!(calls.get(), 1);
    let weak = nodes[0].1.downgrade();
    drop(nodes);
    drop(root);
    drop(canvas);
    drop(document);
    assert!(weak.upgrade().is_none());
    std::fs::write(
        output.join("lifecycle.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "callbacks":calls.get(),"weak_teardown_passed":true,
            "javascript_executed_by_openui":false
        }))?,
    )?;
    Ok(())
}
