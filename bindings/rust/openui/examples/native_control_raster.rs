//! Native control raster options, retained mutation and owned geometry.

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
    let hinting = match args.next().as_deref() {
        Some("none") => TextHinting::None,
        Some("slight") => TextHinting::Slight,
        Some("normal") => TextHinting::Normal,
        Some("full") => TextHinting::Full,
        _ => return Err("expected none, slight, normal or full".into()),
    };
    if args.next().is_some() {
        return Err("unexpected native control argument".into());
    }
    let mut raster_configuration = match policy.as_str() {
        "default" => RasterConfiguration::default(),
        "freetype" => RasterConfiguration::chromium_linux_lcd(),
        "fontations" => RasterConfiguration::chromium_linux_fontations_lcd(),
        _ => return Err("expected default, freetype or fontations".into()),
    };
    raster_configuration.native_text.hinting = hinting;
    let phases: Vec<u8> = (0..64).collect();
    let (width, height) = (800.0, 600.0);
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
    let mut texts = Vec::new();
    for (index, phase) in phases.iter().enumerate() {
        let control = Element::create(&document, "input")?;
        control.set_attribute("type", "file")?;
        control.set_position(Position::Absolute)?;
        control.set_left(Length::px(
            20.0 + (index % 4) as f32 * 190.0 + f32::from(*phase) / 64.0,
        ))?;
        control.set_top(Length::px(20.0 + (index / 4) as f32 * 32.0))?;
        control.set_width(LengthValue::px(180.0))?;
        control.set_height(LengthValue::px(28.0))?;
        control.set_font_family(FontFamilyList::single("DejaVu Sans"))?;
        control.set_font_size(LengthValue::px(17.0))?;
        control.set_color(Color::BLACK)?;
        root.append_child(&control)?;
        texts.push(control);
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
            text.set_opacity(0.5).unwrap();
        }
        observed.set(observed.get() + 1);
    })?;
    root.click()?;
    if calls.get() != 1 {
        return Err("native control callback failed".into());
    }
    assert_eq!(document.raster_configuration()?, raster_configuration);
    let after = texts
        .iter()
        .map(bounds_json)
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    assert_eq!(before, after);
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
        return Err("native control callback retained document".into());
    }
    Ok(())
}
