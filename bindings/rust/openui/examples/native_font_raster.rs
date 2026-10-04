//! Explicit native font raster choices, mutation and owned geometry.

use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn bounds_json(element: &Element) -> Result<String, Box<dyn std::error::Error>> {
    let bounds = element.bounding_rect()?.ok_or("text box required")?;
    Ok(format!(
        "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        f64::from(bounds.x),
        f64::from(bounds.y),
        f64::from(bounds.width),
        f64::from(bounds.height),
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("device scale required")?.parse()?;
    let policy = args.next().ok_or("font raster policy required")?;
    let family = args.next().unwrap_or_else(|| "Ahem".into());
    let size: f32 = args.next().unwrap_or_else(|| "20".into()).parse()?;
    let phase: u8 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let width: f64 = args.next().unwrap_or_else(|| "320".into()).parse()?;
    let height: f64 = args.next().unwrap_or_else(|| "160".into()).parse()?;
    if args.next().is_some()
        || !matches!(
            family.as_str(),
            "Ahem" | "DejaVu Sans" | "DejaVu Serif" | "DejaVu Sans Mono"
        )
        || !size.is_finite()
        || size <= 0.0
        || phase > 63
    {
        return Err("invalid native font arguments".into());
    }
    let raster_configuration = match policy.as_str() {
        "freetype" => RasterConfiguration::chromium_linux_lcd(),
        "fontations" => RasterConfiguration::chromium_linux_fontations_lcd(),
        _ => return Err("expected freetype or fontations".into()),
    };
    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration,
        },
    )?;
    assert_eq!(document.raster_configuration()?, raster_configuration);
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    let text = Element::create(&document, "div")?;
    text.set_position(Position::Absolute)?;
    text.set_left(LengthValue::px(20.0 + f32::from(phase) / 64.0))?;
    text.set_top(LengthValue::px(20.0))?;
    text.set_font_family(FontFamilyList::single(family))?;
    text.set_font_size(LengthValue::px(size))?;
    text.set_line_height(LineHeight::Number(1.0))?;
    text.set_color(Color::BLACK)?;
    text.set_text("X")?;
    root.append_child(&text)?;
    std::fs::create_dir_all(&output)?;
    let before = bounds_json(&text)?;
    let owned_before = text.bounding_rect()?.ok_or("owned text bounds required")?;
    std::fs::write(output.join("before.png"), document.render_to_png_buffer()?)?;

    let calls = Rc::new(Cell::new(0));
    let observed = Rc::clone(&calls);
    let target = text.downgrade();
    text.on("click", move |_| {
        let text = target.upgrade().expect("mounted native text");
        text.set_text("XX").unwrap();
        text.set_color(Color::BLUE).unwrap();
        observed.set(observed.get() + 1);
    })?;
    text.click()?;
    if calls.get() != 1 || text.text_content()? != "XX" {
        return Err("native font callback failed".into());
    }
    assert_eq!(document.raster_configuration()?, raster_configuration);
    let after = bounds_json(&text)?;
    assert_eq!(
        before,
        format!(
            "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
            f64::from(owned_before.x),
            f64::from(owned_before.y),
            f64::from(owned_before.width),
            f64::from(owned_before.height),
        )
    );
    std::fs::write(output.join("after.png"), document.render_to_png_buffer()?)?;
    std::fs::write(
        output.join("geometry.json"),
        format!("{{\"before\":{before},\"after\":{after}}}\n"),
    )?;
    let weak = text.downgrade();
    drop(text);
    drop(root);
    drop(document);
    if weak.upgrade().is_some() {
        return Err("native font callback retained document".into());
    }
    Ok(())
}
