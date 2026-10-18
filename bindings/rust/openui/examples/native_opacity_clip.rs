//! Native opacity and clipping controls for boxes and failed images.
use openui::prelude::*;
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let kind = args.next().ok_or("kind required")?;
    let clip = args.next().ok_or("clip required")?;
    let effect = args.next().ok_or("effect required")?;
    let phase: f32 = args.next().ok_or("phase required")?.parse()?;
    let width: f32 = args.next().unwrap_or_else(|| "24".into()).parse()?;
    let height: f32 = args.next().unwrap_or_else(|| "80".into()).parse()?;
    let variant = args.next().unwrap_or_else(|| "plain".into());
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let parent = Element::create(&document, "div")?;
    parent.set_position(Position::Absolute)?;
    parent.set_left(Length::px(27.0 + phase))?;
    parent.set_top(Length::px(22.0 + phase))?;
    parent.set_width(LengthValue::px(width))?;
    parent.set_height(LengthValue::px(40.0))?;
    match clip.as_str() {
        "parent" => parent.set_overflow(Overflow::Hidden)?,
        "none" => {}
        _ => return Err("unknown clip".into()),
    }
    if effect == "parent50" {
        parent.set_opacity(0.5)?;
    }
    root.append_child(&parent)?;
    let child = Element::create(&document, if kind == "box" { "div" } else { "img" })?;
    child.set_display(Display::Block)?;
    child.set_width(LengthValue::percent(100.0))?;
    child.set_height(LengthValue::px(height))?;
    child.set_background_color(Color::from_rgba8(128, 128, 128, 255))?;
    match variant.as_str() {
        "plain" | "tiny" => {}
        "padded" => child.set_padding(Edges::all(LengthValue::px(6.0)))?,
        "bordered" => child.set_border(Border {
            width: 2.0,
            style: BorderStyle::Solid,
            color: Color::BLACK,
        })?,
        "rounded" => child.set_border_radius(CornerRadii(Edges::all(LengthValue::px(7.0))))?,
        "visible" => child.set_overflow(Overflow::Visible)?,
        "hidden" => child.set_overflow(Overflow::Hidden)?,
        "clip" => child.set_overflow(Overflow::Clip)?,
        "alpha128" => child.set_background_color(Color::from_rgba8(128, 128, 128, 128))?,
        "transparent" => child.set_background_color(Color::TRANSPARENT)?,
        _ => return Err("unknown image decoration".into()),
    }
    match effect.as_str() {
        "self50" => child.set_opacity(0.5)?,
        "parent50" | "none" => {}
        _ => return Err("unknown opacity".into()),
    }
    match kind.as_str() {
        "empty" => child.set_attribute("src", "")?,
        "box" | "no-source" => {}
        _ => return Err("unknown child kind".into()),
    }
    parent.append_child(&child)?;
    let rect = child.bounding_rect()?.ok_or("child has no owned bounds")?;
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("geometry.json"),format!("{{\"nodes\":{{\"image\":{{\"bounds\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}}}}}}}\n",rect.x,rect.y,rect.width,rect.height))?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    Ok(())
}
