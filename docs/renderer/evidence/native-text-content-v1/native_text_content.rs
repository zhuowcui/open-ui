//! Shared native text replacement, Rust callbacks and owned geometry.

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
    let family = args.next().unwrap_or_else(|| "Ahem".into());
    let size: f32 = args.next().unwrap_or_else(|| "20".into()).parse()?;
    let phases: Vec<u8> = (0..64).collect();
    let width = 800.0;
    let height = 600.0;
    if args.next().is_some()
        || !matches!(
            family.as_str(),
            "Ahem" | "DejaVu Sans" | "DejaVu Serif" | "DejaVu Sans Mono"
        )
        || !size.is_finite()
        || size <= 0.0
    {
        return Err("invalid native text arguments".into());
    }
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        FontCollection::deterministic_test(),
    )?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    let mut texts = Vec::new();
    for (index, phase) in phases.iter().enumerate() {
        let text = Element::create(&document, "div")?;
        text.set_position(Position::Absolute)?;
        text.set_left(Length::px(
            20.0 + (index % 8) as f32 * 92.0 + f32::from(*phase) / 64.0,
        ))?;
        text.set_top(Length::px(20.0 + (index / 8) as f32 * 60.0))?;
        text.set_font_family(FontFamilyList::single(family.clone()))?;
        text.set_font_size(LengthValue::px(size))?;
        text.set_line_height(LineHeight::Number(1.0))?;
        text.set_color(Color::BLACK)?;
        text.set_text("X")?;
        root.append_child(&text)?;
        texts.push(text);
    }
    std::fs::create_dir_all(&output)?;
    let before = texts
        .iter()
        .map(bounds_json)
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    let owned_before = texts
        .iter()
        .map(|text| -> Result<_, Box<dyn std::error::Error>> {
            Ok(text.bounding_rect()?.ok_or("owned text bounds required")?)
        })
        .collect::<Result<Vec<_>, _>>()?;
    std::fs::write(output.join("before.png"), document.render_to_png_buffer()?)?;

    let calls = Rc::new(Cell::new(0));
    let observed = Rc::clone(&calls);
    let targets: Vec<_> = texts.iter().map(Element::downgrade).collect();
    root.on("click", move |_| {
        for target in &targets {
            let text = target.upgrade().expect("mounted native text");
            text.set_text("XX").unwrap();
            text.set_color(Color::BLUE).unwrap();
        }
        observed.set(observed.get() + 1);
    })?;
    root.click()?;
    if calls.get() != 1
        || texts
            .iter()
            .any(|text| !matches!(text.text_content().as_deref(), Ok("XX")))
    {
        return Err("native text callback failed".into());
    }
    let after = texts
        .iter()
        .map(bounds_json)
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    assert_eq!(
        before,
        owned_before
            .iter()
            .map(|bounds| format!(
                "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
                f64::from(bounds.x),
                f64::from(bounds.y),
                f64::from(bounds.width),
                f64::from(bounds.height),
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    std::fs::write(output.join("after.png"), document.render_to_png_buffer()?)?;
    std::fs::write(
        output.join("geometry.json"),
        format!(
            "{{\"before\":[{before}],\"after\":[{after}],\"callback_count\":{}}}\n",
            calls.get()
        ),
    )?;
    let weak: Vec<_> = texts.iter().map(Element::downgrade).collect();
    drop(texts);
    drop(root);
    drop(document);
    if weak.iter().any(|text| text.upgrade().is_some()) {
        return Err("native text callback retained document".into());
    }
    Ok(())
}
