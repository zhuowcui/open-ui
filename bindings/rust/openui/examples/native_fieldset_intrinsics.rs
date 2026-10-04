//! Native fieldset and image sizing, including mutation from a Rust callback.

use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn bounds_json(element: &Element) -> Result<String, Box<dyn std::error::Error>> {
    let bounds = element.bounding_rect()?.ok_or("native box required")?;
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
    let tag = args.next().ok_or("container kind required")?;
    let padding: f32 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let sizing = args.next().unwrap_or_else(|| "content-box".into());
    let width: f64 = args.next().unwrap_or_else(|| "800".into()).parse()?;
    let height: f64 = args.next().unwrap_or_else(|| "600".into()).parse()?;
    if args.next().is_some() || !matches!(tag.as_str(), "div" | "fieldset") {
        return Err("unexpected container or arguments".into());
    }
    if !matches!(padding, 0.0 | 10.0) {
        return Err("padding must be zero or ten logical pixels".into());
    }
    let box_sizing = match sizing.as_str() {
        "content-box" => BoxSizing::ContentBox,
        "border-box" => BoxSizing::BorderBox,
        _ => return Err("unknown box sizing".into()),
    };
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        FontCollection::deterministic_test(),
    )?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_font_family(FontFamilyList::single("Ahem"))?;
    root.set_font_size(LengthValue::px(16.0))?;
    root.set_line_height(LineHeight::Number(1.0))?;
    root.set_padding_top(LengthValue::px(20.0))?;
    root.set_padding_right(LengthValue::px(20.0))?;
    root.set_padding_bottom(LengthValue::px(20.0))?;
    root.set_padding_left(LengthValue::px(20.0))?;
    let resource = document.register_image_resource(
        "native-green-image",
        "image/png",
        "d49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/accountability/data/wpt_assets/sp20/",
            "d49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe.png"
        ))
        .to_vec(),
    )?;
    let container = Element::create(&document, &tag)?;
    container.set_display(Display::Block)?;
    container.set_box_sizing(box_sizing)?;
    container.set_width(LengthValue::Computed(Length::fit_content()))?;
    container.set_height(LengthValue::px(100.0))?;
    container.set_background_color(Color::RED)?;
    let border = || Border {
        width: 0,
        style: BorderStyle::None,
        color: Color::BLACK,
    };
    container.set_border_top(border())?;
    container.set_border_right(border())?;
    container.set_border_bottom(border())?;
    container.set_border_left(border())?;
    container.set_margin_top(LengthValue::px(0.0))?;
    container.set_margin_right(LengthValue::px(0.0))?;
    container.set_margin_bottom(LengthValue::px(0.0))?;
    container.set_margin_left(LengthValue::px(0.0))?;
    container.set_padding_top(LengthValue::px(padding))?;
    container.set_padding_right(LengthValue::px(padding))?;
    container.set_padding_bottom(LengthValue::px(padding))?;
    container.set_padding_left(LengthValue::px(padding))?;
    root.append_child(&container)?;
    let image = Element::create(&document, "img")?;
    image.set_display(Display::Inline)?;
    image.set_height(LengthValue::percent(100.0))?;
    image.set_image_resource(resource, Some((200.0, 200.0)))?;
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
    let target = container.downgrade();
    container.on("click", move |_| {
        target
            .upgrade()
            .expect("native container remains attached")
            .set_height(LengthValue::px(150.0))
            .expect("native height mutation");
        observed.set(observed.get() + 1);
    })?;
    container.click()?;
    if calls.get() != 1 {
        return Err("native click callback did not run once".into());
    }
    let after = format!(
        "{{\"container\":{},\"image\":{}}}",
        bounds_json(&container)?,
        bounds_json(&image)?
    );
    std::fs::write(output.join("after.png"), document.render_to_png_buffer()?)?;
    std::fs::write(
        output.join("geometry.json"),
        format!("{{\"before\":{before},\"after\":{after}}}\n"),
    )?;
    let weak = image.downgrade();
    drop(image);
    drop(container);
    drop(root);
    drop(document);
    if weak.upgrade().is_some() {
        return Err("native callback retained document after teardown".into());
    }
    Ok(())
}
