//! Native Rust application reducing reversed scroll origins and backing phases.
use openui::prelude::*;
use std::{cell::Cell, path::Path, rc::Rc};

fn snapshot(
    document: &Document,
    viewport: &Element,
    child: &Element,
    calls: u32,
    name: &str,
    output: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let metrics = viewport.scroll_metrics()?.ok_or("scroll metrics missing")?;
    let bounds = viewport.bounding_rect()?.ok_or("viewport bounds missing")?;
    let child_bounds = child.bounding_rect()?.ok_or("child bounds missing")?;
    std::fs::write(
        output.join(format!("{name}.png")),
        document.render_to_png_buffer()?,
    )?;
    Ok(format!("{{\"name\":\"{name}\",\"callback_count\":{calls},\"scroll\":{{\"x\":{},\"y\":{}}},\"client\":{{\"width\":{},\"height\":{}}},\"extent\":{{\"width\":{},\"height\":{}}},\"nodes\":{{\"viewport\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\"child\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}}}}}",
        viewport.scroll_left()?, viewport.scroll_top()?, metrics.client_width, metrics.client_height,
        metrics.scroll_width, metrics.scroll_height, bounds.x, bounds.y, bounds.width, bounds.height,
        child_bounds.x, child_bounds.y, child_bounds.width, child_bounds.height))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let flow = args.next().ok_or("flow required")?;
    let direction = match args.next().ok_or("direction required")?.as_str() {
        "ltr" => Direction::Ltr,
        "rtl" => Direction::Rtl,
        _ => return Err("unknown direction".into()),
    };
    let background_alpha: f32 = args.next().ok_or("background alpha required")?.parse()?;
    let phase: f32 = args.next().ok_or("phase required")?.parse()?;
    std::fs::create_dir_all(&output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_scrollbar_width(ScrollbarWidth::None)?;
    let viewport = Element::create(&document, "div")?;
    viewport.set_position(Position::Absolute)?;
    viewport.set_left(Length::px(44.0 + phase))?;
    viewport.set_top(Length::px(44.0 + phase))?;
    viewport.set_width(LengthValue::px(80.0))?;
    viewport.set_height(LengthValue::px(80.0))?;
    viewport.set_border(Border {
        width: 3.0,
        style: BorderStyle::Solid,
        color: Color::from_named("OrAnGe").ok_or("named border color missing")?,
    })?;
    viewport.set_writing_mode(WritingMode::HorizontalTb)?;
    viewport.set_direction(direction)?;
    viewport.set_overflow(Overflow::Scroll)?;
    viewport.set_scrollbar_width(ScrollbarWidth::None)?;
    viewport.set_background_color(Color::from_rgba_f32(0.0, 1.0, 1.0, background_alpha))?;
    {
        viewport.set_display(Display::Flex)?;
        viewport.set_flex_direction(match flow.as_str() {
            "row" => FlexDirection::Row,
            "row-reverse" => FlexDirection::RowReverse,
            "column" => FlexDirection::Column,
            "column-reverse" => FlexDirection::ColumnReverse,
            _ => return Err("unknown flow".into()),
        })?;
    }
    root.append_child(&viewport)?;
    let child = Element::create(&document, "div")?;
    child.set_width(LengthValue::px(30.0))?;
    child.set_height(LengthValue::px(30.0))?;
    child.set_border(Border {
        width: 30.0,
        style: BorderStyle::Solid,
        color: Color::from_rgba8(0, 255, 255, 255),
    })?;
    child.set_flex_shrink(0.0)?;
    child.set_background_color(Color::from_rgba8(0, 128, 128, 255))?;
    viewport.append_child(&child)?;
    let calls = Rc::new(Cell::new(0u32));
    let stage = Rc::new(Cell::new(0u8));
    let callback_calls = calls.clone();
    let callback_stage = stage.clone();
    let weak_viewport = viewport.downgrade();
    let weak_child = child.downgrade();
    child.on("click", move |_| {
        let viewport = weak_viewport.upgrade().expect("viewport remains attached");
        match callback_stage.get() {
            1 => viewport
                .scroll_to(f64::MAX, f64::MAX)
                .expect("positive limit"),
            2 => viewport
                .scroll_to(-f64::MAX, -f64::MAX)
                .expect("negative limit"),
            3 => viewport.scroll_by(3.25, 7.25).expect("relative scroll"),
            4 => {
                viewport
                    .smooth_scroll_to(-1000.0, 1000.0, 100.0, Easing::Linear)
                    .expect("smooth scroll");
            }
            5 => {
                let child = weak_child.upgrade().expect("child remains attached");
                child.set_width(LengthValue::px(0.0)).expect("shrink width");
                child
                    .set_height(LengthValue::px(0.0))
                    .expect("shrink height");
            }
            6 => {
                viewport
                    .set_width(LengthValue::px(140.0))
                    .expect("resize width");
                viewport
                    .set_height(LengthValue::px(140.0))
                    .expect("resize height");
            }
            _ => panic!("unknown callback stage"),
        }
        callback_calls.set(callback_calls.get() + 1);
    })?;
    let owned = viewport
        .scroll_metrics()?
        .ok_or("initial metrics missing")?;
    let mut records = vec![snapshot(
        &document,
        &viewport,
        &child,
        calls.get(),
        "initial",
        &output,
    )?];
    for (index, name) in [(1, "positive"), (2, "negative"), (3, "by")] {
        stage.set(index);
        child.click()?;
        records.push(snapshot(
            &document,
            &viewport,
            &child,
            calls.get(),
            name,
            &output,
        )?);
    }
    stage.set(4);
    child.click()?;
    document.advance_time(50.0)?;
    records.push(snapshot(
        &document,
        &viewport,
        &child,
        calls.get(),
        "smooth-half",
        &output,
    )?);
    document.advance_time(100.0)?;
    records.push(snapshot(
        &document,
        &viewport,
        &child,
        calls.get(),
        "smooth-end",
        &output,
    )?);
    assert!(!document.is_animating()?);
    for (index, name) in [(5, "shrink"), (6, "resize")] {
        stage.set(index);
        child.click()?;
        records.push(snapshot(
            &document,
            &viewport,
            &child,
            calls.get(),
            name,
            &output,
        )?);
    }
    assert_eq!(calls.get(), 6);
    assert_eq!((owned.client_width, owned.client_height), (80.0, 80.0));
    assert_eq!((owned.scroll_width, owned.scroll_height), (90.0, 90.0));
    assert_eq!(
        (viewport.scroll_left()?, viewport.scroll_top()?),
        (0.0, 0.0)
    );
    std::fs::write(
        output.join("geometry.json"),
        format!("[{}]\n", records.join(",")),
    )?;
    let weak = viewport.downgrade();
    drop(child);
    drop(viewport);
    drop(root);
    drop(document);
    assert!(weak.upgrade().is_none(), "callback retained document");
    Ok(())
}
