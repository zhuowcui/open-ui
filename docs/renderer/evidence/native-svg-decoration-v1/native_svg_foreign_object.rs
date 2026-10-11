//! SVG viewport decoration and native child layout through public Rust APIs.
use openui::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let phase: f32 = args.next().ok_or("phase required")?.parse()?;
    let width: f32 = args.next().ok_or("width required")?.parse()?;
    let height: f32 = args.next().ok_or("height required")?.parse()?;
    let border_width: f32 = args.next().ok_or("border width required")?.parse()?;
    let border_style = match args.next().ok_or("border style required")?.as_str() {
        "solid" => BorderStyle::Solid,
        "double" => BorderStyle::Double,
        _ => return Err("unknown border style".into()),
    };
    let radius: f32 = args.next().ok_or("radius required")?.parse()?;
    let padding: f32 = args.next().ok_or("padding required")?.parse()?;
    let sizing = match args.next().ok_or("box sizing required")?.as_str() {
        "content-box" => BoxSizing::ContentBox,
        "border-box" => BoxSizing::BorderBox,
        _ => return Err("unknown box sizing".into()),
    };
    let opacity: f32 = args.next().ok_or("opacity required")?.parse()?;
    let child_kind = args.next().ok_or("child kind required")?;
    let writing_mode = match args.next().ok_or("writing mode required")?.as_str() {
        "horizontal-tb" => WritingMode::HorizontalTb,
        "vertical-rl" => WritingMode::VerticalRl,
        _ => return Err("unknown writing mode".into()),
    };
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let svg = Element::create(&document, "svg")?;
    svg.set_position(Position::Absolute)?;
    svg.set_left(Length::px(20.0 + phase))?;
    svg.set_top(Length::px(20.0 + phase))?;
    svg.set_width(LengthValue::px(240.0))?;
    svg.set_height(LengthValue::px(180.0))?;
    svg.set_overflow(Overflow::Hidden)?;
    svg.set_overflow_clip_box(OverflowClipBox::ContentBox)?;
    root.append_child(&svg)?;
    let foreign = Element::create(&document, "foreignObject")?;
    foreign.set_width(LengthValue::px(width))?;
    foreign.set_height(LengthValue::px(height))?;
    foreign.set_border_left(Border {
        width: border_width,
        style: border_style,
        color: Color::BLACK,
    })?;
    foreign.set_border_radius(CornerRadii(Edges::all(LengthValue::px(radius))))?;
    foreign.set_padding(Edges::all(LengthValue::px(padding)))?;
    foreign.set_box_sizing(sizing)?;
    foreign.set_opacity(opacity)?;
    foreign.set_writing_mode(writing_mode)?;
    svg.append_child(&foreign)?;
    let child = match child_kind.as_str() {
        "none" => None,
        "red" => {
            let child = Element::create(&document, "div")?;
            child.set_width(LengthValue::percent(100.0))?;
            child.set_height(LengthValue::percent(100.0))?;
            child.set_background_color(Color::RED)?;
            foreign.append_child(&child)?;
            Some(child)
        }
        _ => return Err("unknown child kind".into()),
    };
    let foreign_rect = foreign.bounding_rect()?.ok_or("viewport has no bounds")?;
    let bounds = |r: Rect| {
        format!(
            "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
            r.x, r.y, r.width, r.height
        )
    };
    let mut geometry = format!(
        "{{\"nodes\":{{\"foreign\":{{\"bounds\":{}}}",
        bounds(foreign_rect)
    );
    if let Some(child) = child {
        let rect = child.bounding_rect()?.ok_or("child has no bounds")?;
        geometry.push_str(&format!(",\"content\":{{\"bounds\":{}}}", bounds(rect)));
    }
    geometry.push_str("}}\n");
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("geometry.json"), geometry)?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    Ok(())
}
