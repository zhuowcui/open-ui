//! Consuming Rust app: nested reveal through native callbacks and owned queries.
use openui::prelude::*;
use openui::{ScrollAlignment, ScrollIntoViewContainer, ScrollIntoViewOptions};
use std::{cell::Cell, path::Path, rc::Rc};

fn block(document: &Document, parent: &Element, width: f32, height: f32) -> Result<Element, Error> {
    let element = Element::create(document, "div")?;
    element.set_width(LengthValue::px(width))?;
    element.set_height(LengthValue::px(height))?;
    element.set_scrollbar_width(ScrollbarWidth::None)?;
    parent.append_child(&element)?;
    Ok(element)
}

fn snapshot(
    document: &Document,
    nodes: [&Element; 3],
    output: &Path,
    state: &str,
    calls: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut records = Vec::new();
    for (name, element) in ["outer", "inner", "target"].into_iter().zip(nodes) {
        let bounds = element.bounding_rect()?.ok_or("missing native bounds")?;
        let metrics = element.scroll_metrics()?.ok_or("missing native metrics")?;
        records.push(format!("\"{name}\":{{\"bounds\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\"scroll\":{{\"x\":{},\"y\":{}}},\"client\":{{\"width\":{},\"height\":{}}},\"extent\":{{\"width\":{},\"height\":{}}}}}",bounds.x,bounds.y,bounds.width,bounds.height,element.scroll_left()?,element.scroll_top()?,metrics.client_width,metrics.client_height,metrics.scroll_width,metrics.scroll_height));
    }
    std::fs::write(
        output.join(format!("{state}.png")),
        document.render_to_png_buffer()?,
    )?;
    Ok(format!(
        "{{\"state\":\"{state}\",\"callback_count\":{calls},\"nodes\":{{{}}}}}",
        records.join(",")
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let overflow = match args.next().as_deref() {
        Some("hidden") => Overflow::Hidden,
        Some("clip") => Overflow::Clip,
        _ => return Err("expected hidden or clip".into()),
    };
    std::fs::create_dir_all(&output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_scrollbar_width(ScrollbarWidth::None)?;
    let outer = block(&document, &root, 100.0, 80.0)?;
    outer.set_position(Position::Absolute)?;
    outer.set_left(Length::px(40.0))?;
    outer.set_top(Length::px(30.0))?;
    outer.set_overflow(Overflow::Hidden)?;
    let spacer = block(&document, &outer, 200.0, 200.0)?;
    let inner = block(&document, &outer, 200.0, 100.0)?;
    inner.set_overflow(overflow)?;
    let target = block(&document, &inner, 20.0, 20.0)?;
    target.set_margin_left(LengthValue::px(150.0))?;
    target.set_margin_top(LengthValue::px(120.0))?;
    target.set_background_color(Color::from_rgba8(0, 128, 0, 255))?;
    let options = ScrollIntoViewOptions {
        block: ScrollAlignment::Nearest,
        inline: ScrollAlignment::Nearest,
        container: ScrollIntoViewContainer::All,
    };
    let calls = Rc::new(Cell::new(0));
    let stage = Rc::new(Cell::new(0));
    let callback_calls = calls.clone();
    let callback_stage = stage.clone();
    let weak_target = target.downgrade();
    target.on("click", move |_| {
        let target = weak_target.upgrade().expect("live native target");
        if callback_stage.get() == 0 {
            target.scroll_into_view(options).expect("native reveal");
        } else {
            target
                .smooth_scroll_into_view(options, 100.0)
                .expect("native smooth reveal");
        }
        callback_calls.set(callback_calls.get() + 1);
    })?;
    let owned = outer.scroll_metrics()?.ok_or("missing initial metrics")?;
    let mut records = vec![snapshot(
        &document,
        [&outer, &inner, &target],
        &output,
        "initial",
        calls.get(),
    )?];
    target.click()?;
    records.push(snapshot(
        &document,
        [&outer, &inner, &target],
        &output,
        "reveal",
        calls.get(),
    )?);
    outer.scroll_to(0.0, 0.0)?;
    inner.scroll_to(0.0, 0.0)?;
    stage.set(1);
    target.click()?;
    document.advance_time(100.0)?;
    records.push(snapshot(
        &document,
        [&outer, &inner, &target],
        &output,
        "smooth-end",
        calls.get(),
    )?);
    assert_eq!(calls.get(), 2);
    assert_eq!(owned.client_width, 100.0);
    assert!(!document.is_animating()?);
    std::fs::write(
        output.join("geometry.json"),
        format!("[{}]\n", records.join(",")),
    )?;
    let weak = target.downgrade();
    drop((target, inner, spacer, outer, root, document));
    assert!(
        weak.upgrade().is_none(),
        "native callback retained document"
    );
    Ok(())
}
