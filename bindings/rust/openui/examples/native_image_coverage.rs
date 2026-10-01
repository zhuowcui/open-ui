//! Public native image coverage across backgrounds and subpixel origins.

use openui::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().unwrap_or_else(|| "1".into()).parse()?;
    let width: f32 = args.next().unwrap_or_else(|| "50".into()).parse()?;
    let phase: f32 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let background = args.next().unwrap_or_else(|| "salmon".into());
    let effect = args.next().unwrap_or_else(|| "none".into());
    let mut color = match background.as_str() {
        "salmon" => Color::from_rgba8(250, 128, 114, 255),
        "white" => Color::WHITE,
        "gray" => Color::from_rgba8(128, 128, 128, 255),
        "black" => Color::BLACK,
        _ => return Err("unknown image background".into()),
    };
    if !matches!(width, 50.0 | 100.0) || !matches!(phase, 0.0 | 0.25 | 0.5) {
        return Err("unknown image width or subpixel origin".into());
    }
    let image_opacity = match effect.as_str() {
        "none" | "parent50" | "alpha128" => 1.0,
        "opacity50" => 0.5,
        "opacity75" => 0.75,
        _ => return Err("unknown image effect".into()),
    };
    if effect == "alpha128" {
        color.a = 128.0 / 255.0;
    }
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let container = Element::create(&document, "div")?;
    container.set_display(Display::Block)?;
    container.set_position(Position::Absolute)?;
    container.set_left(Length::px(27.0 + phase))?;
    container.set_top(Length::px(22.0 + phase))?;
    container.set_width(LengthValue::px(32.0))?;
    container.set_height(LengthValue::px(80.0))?;
    if effect == "parent50" {
        container.set_opacity(0.5)?;
    }
    root.append_child(&container)?;
    let image = Element::create(&document, "img")?;
    image.set_display(Display::Block)?;
    image.set_width(LengthValue::percent(width))?;
    image.set_height(LengthValue::px(80.0))?;
    image.set_background_color(color)?;
    image.set_opacity(image_opacity)?;
    image.set_attribute("src", "")?;
    container.append_child(&image)?;
    let rect = image.bounding_rect()?.ok_or("image has no owned box")?;
    std::fs::create_dir_all(&output)?;
    std::fs::write(
        output.join("geometry.json"),
        format!(
            "{{\"nodes\":{{\"image\":{{\"bounds\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}}}}}}}\n",
            rect.x, rect.y, rect.width, rect.height
        ),
    )?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    Ok(())
}
