//! Native viewport scrolling through a Rust callback and normalized wheel input.

use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let width: f32 = args.next().ok_or("content width required")?.parse()?;
    let height: f32 = args.next().ok_or("content height required")?.parse()?;
    let overflow = match args.next().ok_or("overflow required")?.as_str() {
        "visible" => Overflow::Visible,
        "auto" => Overflow::Auto,
        "hidden" => Overflow::Hidden,
        "clip" => Overflow::Clip,
        "scroll" => Overflow::Scroll,
        _ => return Err("unknown overflow".into()),
    };
    let requested_x: f64 = args.next().ok_or("scroll x required")?.parse()?;
    let requested_y: f64 = args.next().ok_or("scroll y required")?.parse()?;
    let wheel_y: f32 = args.next().ok_or("wheel y required")?.parse()?;
    let writing_mode = match args.next().as_deref().unwrap_or("horizontal-tb") {
        "horizontal-tb" => WritingMode::HorizontalTb,
        "vertical-rl" => WritingMode::VerticalRl,
        "vertical-lr" => WritingMode::VerticalLr,
        "sideways-rl" => WritingMode::SidewaysRl,
        "sideways-lr" => WritingMode::SidewaysLr,
        _ => return Err("unknown writing mode".into()),
    };
    let direction = match args.next().as_deref().unwrap_or("ltr") {
        "ltr" => Direction::Ltr,
        "rtl" => Direction::Rtl,
        _ => return Err("unknown direction".into()),
    };
    let left: f32 = args.next().unwrap_or_else(|| "27".into()).parse()?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow_x(overflow)?;
    root.set_overflow_y(overflow)?;
    root.set_writing_mode(writing_mode)?;
    root.set_direction(direction)?;
    let child = Element::create(&document, "div")?;
    child.set_position(Position::Absolute)?;
    child.set_display(Display::Block)?;
    child.set_left(Length::px(left))?;
    child.set_top(Length::px(22.0))?;
    child.set_width(LengthValue::px(width))?;
    child.set_height(LengthValue::px(height))?;
    child.set_background_color(Color::GREEN)?;
    root.append_child(&child)?;
    let _initial = document.render_to_png_buffer()?;
    let calls = Rc::new(Cell::new(0));
    let callback_calls = calls.clone();
    let target = root.downgrade();
    child.on("click", move |_| {
        target
            .upgrade()
            .expect("viewport remains owned")
            .scroll_to(requested_x, requested_y)
            .expect("native viewport scroll");
        callback_calls.set(callback_calls.get() + 1);
    })?;
    child.click()?;
    if calls.get() != 1 {
        return Err("native callback did not run exactly once".into());
    }
    if wheel_y != 0.0 {
        document.dispatch_wheel_event(30.0, 30.0, 0.0, wheel_y, Modifiers::NONE)?;
    }
    let rect = child
        .bounding_rect()?
        .ok_or("content has no owned bounds")?;
    let scroll_x = root.scroll_left()?;
    let scroll_y = root.scroll_top()?;
    let metrics = root
        .scroll_metrics()?
        .ok_or("viewport has no scroll metrics")?;
    std::fs::create_dir_all(&output)?;
    std::fs::write(
        output.join("geometry.json"),
        format!(
            "{{\"callback_count\":{},\"scroll\":{{\"x\":{},\"y\":{}}},\"client\":{{\"width\":{},\"height\":{}}},\"extent\":{{\"width\":{},\"height\":{}}},\"nodes\":{{\"content\":{{\"bounds\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}}}}}}}\n",
            calls.get(), scroll_x, scroll_y, metrics.client_width, metrics.client_height,
            metrics.scroll_width, metrics.scroll_height, rect.x, rect.y, rect.width, rect.height,
        ),
    )?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    let target = root.downgrade();
    drop(child);
    drop(root);
    drop(document);
    if target.upgrade().is_some() {
        return Err("scroll callback retained the document after teardown".into());
    }
    Ok(())
}
