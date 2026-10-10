/home/nero/code/open-ui/bindings/rust/openui/examples/native_shaping_wrap.rs:

//! Native app for line wrapping across font shaping boundaries.
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
            let bounds = element.bounding_rect()?.ok_or("missing text bounds")?;
            Ok(serde_json::json!({"name":name,"bounds":{
                "x":bounds.x,"y":bounds.y,"width":bounds.width,"height":bounds.height
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
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    root.set_font_size(LengthValue::px(16.0))?;
    root.set_font_family(FontFamilyList {
        families: [
            "Ahem",
            "Droid Sans Fallback",
            "Noto Sans Devanagari",
            "Noto Color Emoji",
            "DejaVu Sans",
        ]
        .into_iter()
        .map(|name| FontFamily::Named(name.into()))
        .collect(),
    })?;
    let cases = [
        ("quote", "XX 'XX", 10.0, 10.0, 64.0, 96.0),
        ("double", "XX \"XX\"", 150.0, 10.0, 64.0, 96.0),
        ("contraction", "XX can't", 10.0, 110.0, 64.0, 96.0),
        ("parenthesis", "XX (XX)", 150.0, 110.0, 64.0, 96.0),
        ("suffix", "XX XX'XX", 10.0, 210.0, 64.0, 96.0),
        ("currency", "XX X€X", 150.0, 210.0, 64.0, 96.0),
        (
            "paragraph",
            "CSS3 UAs should ignore border-radius properties applied to internal table elements when 'border-collapse' is 'collapse'.",
            10.0,
            330.0,
            335.0,
            355.0,
        ),
    ];
    let mut nodes = Vec::new();
    for &(name, text, x, y, before, _) in &cases {
        let element = Element::create(&document, "div")?;
        root.append_child(&element)?;
        element.set_position(Position::Absolute)?;
        element.set_left(Length::px(x))?;
        element.set_top(Length::px(y))?;
        element.set_width(LengthValue::px(before))?;
        element.set_text(text)?;
        nodes.push((name.to_owned(), element));
    }
    let targets: Vec<_> = nodes
        .iter()
        .zip(cases.iter())
        .map(|((_, element), case)| (element.downgrade(), case.5))
        .collect();
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    root.on("click", move |_| {
        for (target, width) in &targets {
            target
                .upgrade()
                .expect("mounted text")
                .set_width(LengthValue::px(*width))
                .expect("native width mutation");
        }
        called.set(called.get() + 1);
    })?;
    std::fs::create_dir_all(&output)?;
    snapshot(&document, &nodes, &output, "before")?;
    root.click()?;
    snapshot(&document, &nodes, &output, "after")?;
    assert_eq!(calls.get(), 1);
    let weak = nodes[0].1.downgrade();
    drop(nodes);
    drop(root);
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
