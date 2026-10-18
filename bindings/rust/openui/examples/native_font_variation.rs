//! Change a variable-font instance from a consuming Rust click callback.

use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn bounds_json(element: &Element) -> Result<String, Box<dyn std::error::Error>> {
    let bounds = element.bounding_rect()?.ok_or("mounted label required")?;
    Ok(format!(
        "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        bounds.x, bounds.y, bounds.width, bounds.height,
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("device scale required")?.parse()?;
    if args.next().is_some() {
        return Err("only output directory and device scale are accepted".into());
    }
    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(160.0, 160.0, scale)?,
        FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(true),
        },
    )?;
    let bytes: &[u8] = include_bytes!("data/variabletest_box.ttf");
    let handle = document.register_font_face(bytes, FontFaceDescriptor::new("VariableBox"))?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    let label = Element::create(&document, "div")?;
    label.set_position(Position::Absolute)?;
    label.set_left(Length::px(20.0))?;
    label.set_top(Length::px(20.0))?;
    label.set_font_family(FontFamilyList::single("VariableBox"))?;
    label.set_font_size(LengthValue::px(100.0))?;
    label.set_font_optical_sizing(FontOpticalSizing::None)?;
    label.set_line_height(LineHeight::Number(1.0))?;
    label.set_color(Color::BLACK)?;
    label.set_text("A")?;
    root.append_child(&label)?;
    std::fs::create_dir_all(&output)?;
    let before_bounds = bounds_json(&label)?;
    let before = document.render_to_bitmap()?;
    document.render_to_png(output.join("before.png"))?;
    let calls = Rc::new(Cell::new(0));
    let observed = Rc::clone(&calls);
    let target = label.downgrade();
    root.on("click", move |_| {
        target
            .upgrade()
            .expect("mounted native variable label")
            .set_font_variation_settings(FontVariationList(vec![FontVariation {
                tag: *b"UPWD",
                value: 350.0,
            }]))
            .unwrap();
        observed.set(observed.get() + 1);
    })?;
    root.click()?;
    assert_eq!(calls.get(), 1);
    let after_bounds = bounds_json(&label)?;
    let after = document.render_to_bitmap()?;
    document.render_to_png(output.join("after.png"))?;
    // The independently authored font's fixed upper block is the reference
    // glyph for A at UPWD=350; it does not depend on our variation conversion.
    label.set_font_variation_settings(FontVariationList::default())?;
    label.set_text("\u{2580}")?;
    let expected = document.render_to_bitmap()?;
    document.render_to_png(output.join("upper-block-reference.png"))?;
    std::fs::write(
        output.join("geometry.json"),
        format!(
            "{{\"before\":{before_bounds},\"after\":{after_bounds},\"callback_count\":{},\"render_changed\":{},\"matches_font_reference\":{}}}\n",
            calls.get(), before.pixels() != after.pixels(), after.pixels() == expected.pixels(),
        ),
    )?;
    if before.pixels() == after.pixels() || after.pixels() != expected.pixels() {
        return Err("the native variable-font callback painted the wrong glyph instance".into());
    }
    document.unregister_font_face(handle)?;
    let weak = label.downgrade();
    drop(label);
    drop(root);
    drop(document);
    assert!(weak.upgrade().is_none());
    Ok(())
}
