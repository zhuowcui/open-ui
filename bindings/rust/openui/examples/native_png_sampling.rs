//! Real PNG resources, typed image styles, and mutation from a native callback.

use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let source = PathBuf::from(args.next().ok_or("PNG path required")?);
    let sha256 = args.next().ok_or("PNG SHA-256 required")?;
    let intrinsic_width: f32 = args.next().ok_or("intrinsic width required")?.parse()?;
    let intrinsic_height: f32 = args.next().ok_or("intrinsic height required")?.parse()?;
    let width: f32 = args.next().ok_or("width required")?.parse()?;
    let height: f32 = args.next().ok_or("height required")?.parse()?;
    let phase: f32 = args.next().ok_or("phase required")?.parse()?;
    let kind = args.next().ok_or("image kind required")?;
    let opacity: f32 = args.next().ok_or("opacity required")?.parse()?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let resource = document.register_image_resource(
        source.to_string_lossy(),
        "image/png",
        sha256,
        std::fs::read(&source)?,
    )?;
    let image = Element::create(&document, if kind == "replaced" { "img" } else { "div" })?;
    image.set_display(Display::Block)?;
    image.set_position(Position::Absolute)?;
    image.set_left(Length::px(27.0 + phase))?;
    image.set_top(Length::px(22.0 + phase))?;
    image.set_width(LengthValue::px(width + 4.0))?;
    image.set_height(LengthValue::px(height + 4.0))?;
    image.set_background_color(Color::from_rgba8(204, 204, 204, 255))?;
    match kind.as_str() {
        "replaced" => {
            image.set_image_resource(resource, Some((intrinsic_width, intrinsic_height)))?
        }
        "background" => {
            let mut layer = BackgroundLayer::new(CssImage::Raster(resource));
            layer.repeat_x = BackgroundRepeat::NoRepeat;
            layer.repeat_y = BackgroundRepeat::NoRepeat;
            layer.size = BackgroundSize::Explicit(Length::percent(100.0), Length::percent(100.0));
            image.set_background_layers(vec![layer])?;
        }
        "border" => {
            let border = |width| Border {
                width,
                style: BorderStyle::Solid,
                color: Color::RED,
            };
            image.set_border_top(border(15.0))?;
            image.set_border_right(border(20.0))?;
            image.set_border_bottom(border(35.0))?;
            image.set_border_left(border(30.0))?;
            image.set_border_image(Some(BorderImage {
                source: CssImage::Raster(resource),
                slice: [5.0, 10.0, 15.0, 20.0].map(BorderImageLength::Number),
                fill: false,
                width: std::array::from_fn(|_| BorderImageLength::Number(1.0)),
                outset: std::array::from_fn(|_| BorderImageLength::Number(0.0)),
                repeat_x: BorderImageRepeat::Repeat,
                repeat_y: BorderImageRepeat::Repeat,
            }))?;
        }
        _ => return Err("unknown image kind".into()),
    }
    root.append_child(&image)?;
    // Force the initial layout and paint before mutating through the same
    // callback path a consuming native application uses.
    let _before = document.render_to_png_buffer()?;
    let calls = Rc::new(Cell::new(0));
    let callback_calls = calls.clone();
    let target = image.downgrade();
    image.on("click", move |_| {
        let image = target.upgrade().expect("native image remains attached");
        image
            .set_width(LengthValue::px(width))
            .expect("native width mutation");
        image
            .set_height(LengthValue::px(height))
            .expect("native height mutation");
        image.set_opacity(opacity).expect("native opacity mutation");
        callback_calls.set(callback_calls.get() + 1);
    })?;
    image.click()?;
    if calls.get() != 1 {
        return Err("native callback did not run exactly once".into());
    }
    let rect = image.bounding_rect()?.ok_or("image has no owned bounds")?;
    std::fs::create_dir_all(&output)?;
    std::fs::write(
        output.join("geometry.json"),
        format!(
            "{{\"callback_count\":{},\"nodes\":{{\"image\":{{\"bounds\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}}}}}}}\n",
            calls.get(), rect.x, rect.y, rect.width, rect.height
        ),
    )?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    let target = image.downgrade();
    drop(image);
    drop(root);
    drop(document);
    if target.upgrade().is_some() {
        return Err("native callback retained the document after teardown".into());
    }
    Ok(())
}
