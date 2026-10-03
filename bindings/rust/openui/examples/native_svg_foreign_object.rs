//! SVG viewport decoration and native child layout through public Rust APIs.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

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
    let border_side = args.next().unwrap_or_else(|| "left".into());
    if !matches!(border_side.as_str(), "top" | "right" | "bottom" | "left") {
        return Err("unknown border side".into());
    }
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
    let foreign = Element::create_svg_foreign_object(&document)?;
    foreign.set_width(LengthValue::px(width))?;
    foreign.set_height(LengthValue::px(height))?;
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
    let initial = foreign.bounding_rect()?.ok_or("initial viewport bounds")?;
    let calls = Rc::new(Cell::new(0));
    let callback_calls = calls.clone();
    let target = foreign.downgrade();
    foreign.on("click", move |_| {
        let foreign = target.upgrade().expect("viewport remains owned");
        let border = Border {
            width: border_width,
            style: border_style,
            color: Color::BLACK,
        };
        match border_side.as_str() {
            "top" => foreign.set_border_top(border),
            "right" => foreign.set_border_right(border),
            "bottom" => foreign.set_border_bottom(border),
            _ => foreign.set_border_left(border),
        }
        .expect("native border mutation");
        foreign
            .set_border_radius(CornerRadii(Edges::all(LengthValue::px(radius))))
            .expect("native radius mutation");
        foreign
            .set_padding(Edges::all(LengthValue::px(padding)))
            .expect("native padding mutation");
        foreign.set_box_sizing(sizing).expect("native box sizing");
        foreign.set_opacity(opacity).expect("native opacity");
        foreign
            .set_writing_mode(writing_mode)
            .expect("native writing mode");
        callback_calls.set(callback_calls.get() + 1);
    })?;
    foreign.click()?;
    assert_eq!(calls.get(), 1, "native callback must run once");
    let foreign_rect = foreign.bounding_rect()?.ok_or("viewport has no bounds")?;
    assert_eq!(
        foreign_rect, initial,
        "decoration cannot enlarge SVG viewport"
    );
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
    if let Some(child) = child.as_ref() {
        let rect = child.bounding_rect()?.ok_or("child has no bounds")?;
        geometry.push_str(&format!(",\"content\":{{\"bounds\":{}}}", bounds(rect)));
    }
    geometry.push_str("}}\n");
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("geometry.json"), geometry)?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    let target = foreign.downgrade();
    drop(child);
    drop(foreign);
    drop(svg);
    drop(root);
    drop(document);
    assert!(target.upgrade().is_none(), "callback retained document");
    Ok(())
}
