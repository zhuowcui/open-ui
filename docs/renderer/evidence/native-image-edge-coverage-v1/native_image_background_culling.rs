//! Image coverage and neighboring styles through public Rust methods.

use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn bounds_json(element: &Element) -> Result<String, Box<dyn std::error::Error>> {
    let rect = element
        .bounding_rect()?
        .ok_or("native image bounds required")?;
    Ok(format!(
        "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        rect.x, rect.y, rect.width, rect.height
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("device scale required")?.parse()?;
    let display = args.next().ok_or("image display required")?;
    let variant = args.next().ok_or("image coverage variant required")?;
    let phase: f32 = args
        .next()
        .ok_or("logical origin phase required")?
        .parse()?;
    if args.next().is_some() || !matches!(display.as_str(), "block" | "inline") {
        return Err("unexpected display or arguments".into());
    }
    if !matches!(
        variant.as_str(),
        "opaque"
            | "partial"
            | "opacity"
            | "padding"
            | "contain"
            | "offset"
            | "blur"
            | "shadow"
            | "white"
            | "white-opacity"
            | "transparent"
            | "clip"
    ) {
        return Err("unknown image coverage variant".into());
    }
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(320.0, 240.0, scale)?,
        FontCollection::deterministic_test(),
    )?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_font_family(FontFamilyList::single("Ahem"))?;
    root.set_font_size(LengthValue::px(16.0))?;
    root.set_line_height(LineHeight::Number(1.0))?;
    let (bytes, hash, intrinsic): (&[u8], &str, (f32, f32)) = match variant.as_str() {
        "white" | "white-opacity" => (
            include_bytes!("../tests/assets/1x1-white.png"),
            "b31782b0ecaa71394f1bccf3cc4647ba70b7208464244546b48521a71e1f1dd0",
            (1.0, 1.0),
        ),
        "transparent" => (
            include_bytes!("../tests/assets/green-transparent-200x200.png"),
            "77e8da29ee253660e7a650f43241d96e636b8e2cec5547cb99fd160e95419422",
            (200.0, 200.0),
        ),
        _ => (
            include_bytes!("../tests/assets/green-200.png"),
            "d49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe",
            (200.0, 200.0),
        ),
    };
    let resource =
        document.register_image_resource("native-image", "image/png", hash, bytes.to_vec())?;
    let container = Element::create(&document, "div")?;
    container.set_display(Display::Block)?;
    container.set_position(Position::Absolute)?;
    container.set_left(Length::px(20.0 + phase))?;
    container.set_top(Length::px(20.0 + phase))?;
    container.set_width(LengthValue::px(150.0))?;
    container.set_height(LengthValue::px(100.0))?;
    container.set_background_color(Color::RED)?;
    if variant == "shadow" {
        container.set_box_shadow(vec![BoxShadow {
            offset_x: 1.0,
            offset_y: 1.0,
            blur_radius: 1.0,
            spread_radius: 0.0,
            color: Color::BLACK,
            inset: false,
        }])?;
    }
    root.append_child(&container)?;
    let image = Element::create(&document, "img")?;
    image.set_display(if display == "block" {
        Display::Block
    } else {
        Display::Inline
    })?;
    image.set_width(LengthValue::px(if variant == "partial" {
        100.0
    } else {
        150.0
    }))?;
    image.set_height(LengthValue::px(100.0))?;
    image.set_image_resource(resource, Some(intrinsic))?;
    match variant.as_str() {
        "opacity" | "white-opacity" => image.set_opacity(0.5)?,
        "padding" => image.set_padding(Edges::all(LengthValue::px(1.0)))?,
        "contain" => image.set_object_fit(ObjectFit::Contain)?,
        "offset" => image.set_object_position(ObjectPosition {
            x: BackgroundPosition::start(),
            y: BackgroundPosition::center(),
        })?,
        "blur" => image.set_filter_blur(1.0)?,
        "clip" => {
            image.set_object_fit(ObjectFit::None)?;
            image.set_overflow_x(Overflow::Hidden)?;
            image.set_overflow_y(Overflow::Hidden)?;
        }
        _ => {}
    }
    container.append_child(&image)?;
    let before = format!(
        "{{\"container\":{},\"image\":{}}}",
        bounds_json(&container)?,
        bounds_json(&image)?
    );
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("before.png"), document.render_to_png_buffer()?)?;
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let container_target = container.downgrade();
    let image_target = image.downgrade();
    container.on("click", move |_| {
        container_target
            .upgrade()
            .expect("native container remains attached")
            .set_height(LengthValue::px(150.0))
            .expect("native container height mutation");
        image_target
            .upgrade()
            .expect("native image remains attached")
            .set_height(LengthValue::px(150.0))
            .expect("native image height mutation");
        observed.set(observed.get() + 1);
    })?;
    container.click()?;
    if calls.get() != 1 {
        return Err("native Rust callback did not run exactly once".into());
    }
    let after = format!(
        "{{\"container\":{},\"image\":{}}}",
        bounds_json(&container)?,
        bounds_json(&image)?
    );
    std::fs::write(output.join("after.png"), document.render_to_png_buffer()?)?;
    std::fs::write(
        output.join("geometry.json"),
        format!(
            "{{\"before\":{before},\"after\":{after},\"callback_count\":{}}}\n",
            calls.get()
        ),
    )?;
    let weak = image.downgrade();
    drop(image);
    drop(container);
    drop(root);
    drop(document);
    if weak.upgrade().is_some() {
        return Err("native callback retained the document after teardown".into());
    }
    Ok(())
}
