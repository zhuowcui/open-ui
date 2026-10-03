//! Native application reproducing nowrap overflow and nested sticky scrolling.
use openui::prelude::*;
use std::{cell::Cell, path::Path, rc::Rc};

fn inline_box(document: &Document, width: f32) -> Result<Element, Error> {
    let element = Element::create(document, "div")?;
    element.set_display(Display::InlineBlock)?;
    element.set_width(LengthValue::px(width))?;
    element.set_height(LengthValue::px(150.0))?;
    Ok(element)
}

fn snapshot(
    document: &Document,
    viewport: &Element,
    content: &Element,
    sticky: &Element,
    calls: u32,
    name: &str,
    output: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let metrics = viewport.scroll_metrics()?.ok_or("metrics missing")?;
    let rect = |element: &Element| -> Result<String, Error> {
        let r = element
            .bounding_rect()?
            .ok_or(Error::InvalidArgument("bounds missing"))?;
        Ok(format!(
            "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
            r.x, r.y, r.width, r.height
        ))
    };
    std::fs::write(
        output.join(format!("{name}.png")),
        document.render_to_png_buffer()?,
    )?;
    Ok(format!("{{\"name\":\"{name}\",\"callback_count\":{calls},\"scroll\":{{\"x\":{},\"y\":{}}},\"client\":{{\"width\":{},\"height\":{}}},\"extent\":{{\"width\":{},\"height\":{}}},\"nodes\":{{\"viewport\":{},\"content\":{},\"sticky\":{}}}}}", viewport.scroll_left()?, viewport.scroll_top()?, metrics.client_width, metrics.client_height, metrics.scroll_width, metrics.scroll_height, rect(viewport)?, rect(content)?, rect(sticky)?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    std::fs::create_dir_all(&output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(800.0, 600.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_scrollbar_width(ScrollbarWidth::None)?;
    let viewport = Element::create(&document, "div")?;
    viewport.set_position(Position::Absolute)?;
    viewport.set_left(Length::px(20.0))?;
    viewport.set_top(Length::px(20.0))?;
    viewport.set_width(LengthValue::px(250.0))?;
    viewport.set_height(LengthValue::px(150.0))?;
    viewport.set_overflow_y(Overflow::Hidden)?;
    viewport.set_scrollbar_width(ScrollbarWidth::None)?;
    viewport.set_white_space(WhiteSpaceShorthand {
        collapse: WhiteSpaceCollapse::Collapse,
        wrap: TextWrapMode::Nowrap,
    })?;
    root.append_child(&viewport)?;
    let first = inline_box(&document, 100.0)?;
    let content = inline_box(&document, 300.0)?;
    let last = inline_box(&document, 100.0)?;
    for child in [&first, &content, &last] {
        viewport.append_child(child)?;
    }
    let leading = inline_box(&document, 100.0)?;
    let sticky = inline_box(&document, 100.0)?;
    sticky.set_height(LengthValue::px(100.0))?;
    sticky.set_vertical_align(VerticalAlign::Top)?;
    sticky.set_position(Position::Sticky)?;
    sticky.set_right(Length::px(100.0))?;
    sticky.set_background_color(Color::from_rgba8(0, 128, 0, 255))?;
    let trailing = inline_box(&document, 100.0)?;
    for child in [&leading, &sticky, &trailing] {
        content.append_child(child)?;
    }
    let calls = Rc::new(Cell::new(0u32));
    let stage = Rc::new(Cell::new(0u8));
    let callback_calls = calls.clone();
    let callback_stage = stage.clone();
    let weak_viewport = viewport.downgrade();
    let weak_content = content.downgrade();
    let weak_spacers = [&first, &last, &leading, &trailing].map(Element::downgrade);
    sticky.on("click", move |_| {
        let viewport = weak_viewport.upgrade().expect("attached viewport");
        match callback_stage.get() {
            1 => viewport.scroll_to(25.0, 0.0).expect("scroll to 25"),
            2 => viewport.scroll_to(100.0, 0.0).expect("scroll to 100"),
            3 => viewport.scroll_to(200.0, 0.0).expect("scroll to 200"),
            4 => viewport.scroll_to(f64::MAX, 0.0).expect("positive limit"),
            5 => viewport.scroll_by(-75.0, 0.0).expect("relative scroll"),
            6 => {
                viewport
                    .smooth_scroll_to(200.0, 0.0, 100.0, Easing::Linear)
                    .expect("smooth sticky scroll");
            }
            7 => {
                for weak in &weak_spacers {
                    weak.upgrade()
                        .expect("attached spacer")
                        .set_width(LengthValue::px(0.0))
                        .expect("shrink spacer");
                }
                weak_content
                    .upgrade()
                    .expect("attached content")
                    .set_width(LengthValue::px(100.0))
                    .expect("shrink content");
            }
            8 => viewport
                .set_width(LengthValue::px(600.0))
                .expect("resize viewport"),
            _ => panic!("unknown callback stage"),
        }
        callback_calls.set(callback_calls.get() + 1);
    })?;
    let owned = viewport
        .scroll_metrics()?
        .ok_or("initial metrics missing")?;
    assert_eq!((owned.client_width, owned.scroll_width), (250.0, 500.0));
    let mut records = vec![snapshot(
        &document,
        &viewport,
        &content,
        &sticky,
        calls.get(),
        "initial",
        &output,
    )?];
    for (index, name) in [
        (1, "scroll-25"),
        (2, "scroll-100"),
        (3, "scroll-200"),
        (4, "maximum"),
        (5, "by"),
    ] {
        stage.set(index);
        sticky.click()?;
        records.push(snapshot(
            &document,
            &viewport,
            &content,
            &sticky,
            calls.get(),
            name,
            &output,
        )?);
    }
    stage.set(6);
    sticky.click()?;
    for (time, name) in [(50.0, "smooth-half"), (100.0, "smooth-end")] {
        document.advance_time(time)?;
        records.push(snapshot(
            &document,
            &viewport,
            &content,
            &sticky,
            calls.get(),
            name,
            &output,
        )?);
    }
    assert!(!document.is_animating()?);
    for (index, name) in [(7, "shrink"), (8, "resize")] {
        stage.set(index);
        sticky.click()?;
        records.push(snapshot(
            &document,
            &viewport,
            &content,
            &sticky,
            calls.get(),
            name,
            &output,
        )?);
    }
    assert_eq!(calls.get(), 8);
    assert_eq!(owned.scroll_width, 500.0);
    assert_eq!(viewport.scroll_left()?, 0.0);
    std::fs::write(
        output.join("geometry.json"),
        format!("[{}]\n", records.join(",")),
    )?;
    let weak = viewport.downgrade();
    drop((
        first, last, leading, trailing, sticky, content, viewport, root, document,
    ));
    assert!(weak.upgrade().is_none(), "callback retained document");
    Ok(())
}
