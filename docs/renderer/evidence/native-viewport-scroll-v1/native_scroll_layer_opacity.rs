use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let background_alpha: f32 = args.next().ok_or("background alpha required")?.parse()?;
    let clip = match args.next().ok_or("background clip required")?.as_str() {
        "border-box" => BackgroundClip::BorderBox,
        "padding-box" => BackgroundClip::PaddingBox,
        "content-box" => BackgroundClip::ContentBox,
        _ => return Err("unknown clip".into()),
    };
    let radius: f32 = args.next().ok_or("radius required")?.parse()?;
    let opacity: f32 = args.next().ok_or("opacity required")?.parse()?;
    let blur: f32 = args.next().ok_or("blur required")?.parse()?;
    let child_kind = args.next().ok_or("child kind required")?;
    let request: f64 = args.next().ok_or("scroll request required")?.parse()?;
    let phase: f32 = args.next().ok_or("phase required")?.parse()?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_scrollbar_width(ScrollbarWidth::None)?;
    let viewport = Element::create(&document, "div")?;
    viewport.set_position(Position::Absolute)?;
    viewport.set_left(Length::px(44.0 + phase))?;
    viewport.set_top(Length::px(44.0 + phase))?;
    viewport.set_display(Display::Flex)?;
    viewport.set_width(LengthValue::px(80.0))?;
    viewport.set_height(LengthValue::px(80.0))?;
    viewport.set_padding(Edges::all(LengthValue::px(10.0)))?;
    viewport.set_border(Border {
        width: 3.0,
        style: BorderStyle::Solid,
        color: Color::from_rgba8(255, 165, 0, 255),
    })?;
    viewport.set_overflow(Overflow::Scroll)?;
    viewport.set_scrollbar_width(ScrollbarWidth::None)?;
    viewport.set_background_color(Color::WHITE)?;
    root.append_child(&viewport)?;
    let child = Element::create(&document, "div")?;
    child.set_flex_shrink(0.0)?;
    child.set_width(LengthValue::px(30.0))?;
    child.set_height(LengthValue::px(30.0))?;
    match child_kind.as_str() {
        "reference" => {
            viewport.set_padding(Edges::all(LengthValue::px(0.0)))?;
            child.set_border(Border {
                width: 30.0,
                style: BorderStyle::Solid,
                color: Color::from_rgba8(0, 255, 255, 255),
            })?;
            child.set_background_color(Color::from_rgba8(0, 128, 128, 255))?;
        }
        "opaque" => {
            child.set_border(Border {
                width: 50.0,
                style: BorderStyle::Solid,
                color: Color::from_rgba8(0, 255, 255, 255),
            })?;
            child.set_background_color(Color::from_rgba8(0, 128, 128, 255))?;
        }
        "translucent" => {
            child.set_border(Border {
                width: 50.0,
                style: BorderStyle::Solid,
                color: Color::from_rgba_f32(0.0, 1.0, 1.0, 0.5),
            })?;
            child.set_background_color(Color::from_rgba_f32(
                0.0,
                128.0 / 255.0,
                128.0 / 255.0,
                0.5,
            ))?;
        }
        "partial" => {
            child.set_background_color(Color::from_rgba8(0, 128, 128, 255))?;
        }
        _ => return Err("unknown child kind".into()),
    }
    viewport.append_child(&child)?;
    let initial = document.render_to_png_buffer()?;
    let calls = Rc::new(Cell::new(0));
    let callback_calls = calls.clone();
    let target = viewport.downgrade();
    child.on("click", move |_| {
        let viewport = target.upgrade().expect("viewport remains owned");
        viewport
            .set_background_color(Color::from_rgba_f32(0.0, 1.0, 1.0, background_alpha))
            .expect("background");
        viewport.set_background_clip(clip).expect("background clip");
        viewport
            .set_border_radius(CornerRadii(Edges::all(LengthValue::px(radius))))
            .expect("radius");
        viewport.set_opacity(opacity).expect("opacity");
        viewport.set_filter_blur(blur).expect("filter");
        viewport.scroll_to(request, request).expect("scroll");
        callback_calls.set(callback_calls.get() + 1);
    })?;
    child.click()?;
    assert_eq!(calls.get(), 1);
    let bounds = viewport.bounding_rect()?.ok_or("viewport bounds missing")?;
    let child_bounds = child.bounding_rect()?.ok_or("child bounds missing")?;
    let metrics = viewport.scroll_metrics()?.ok_or("scroll metrics missing")?;
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("initial.png"), initial)?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    std::fs::write(output.join("geometry.json"), format!(
        "{{\"callback_count\":{},\"scroll\":{{\"x\":{},\"y\":{}}},\"client\":{{\"width\":{},\"height\":{}}},\"extent\":{{\"width\":{},\"height\":{}}},\"nodes\":{{\"viewport\":{{\"bounds\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}}},\"child\":{{\"bounds\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}}}}}}}\n",
        calls.get(), viewport.scroll_left()?, viewport.scroll_top()?,
        metrics.client_width, metrics.client_height, metrics.scroll_width, metrics.scroll_height,
        bounds.x, bounds.y, bounds.width, bounds.height,
        child_bounds.x, child_bounds.y, child_bounds.width, child_bounds.height,
    ))?;
    let target = viewport.downgrade();
    drop(child);
    drop(viewport);
    drop(root);
    drop(document);
    assert!(target.upgrade().is_none(), "callback retained document");
    Ok(())
}
