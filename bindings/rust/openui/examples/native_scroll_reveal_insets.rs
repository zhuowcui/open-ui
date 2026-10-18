//! Public native consumer for reveal insets and retained table coordinates.
//! Usage: native_scroll_reveal_insets OUTPUT SCALE block|caption|transformed-caption
use openui::prelude::*;
use openui::{ScrollAlignment, ScrollIntoViewContainer, ScrollIntoViewOptions};
use std::{cell::Cell, path::Path, rc::Rc};

fn block(document: &Document, parent: &Element, width: f32, height: f32) -> Result<Element, Error> {
    let node = Element::create(document, "div")?;
    node.set_width(LengthValue::px(width))?;
    node.set_height(LengthValue::px(height))?;
    node.set_scrollbar_width(ScrollbarWidth::None)?;
    parent.append_child(&node)?;
    Ok(node)
}

fn snapshot(
    document: &Document,
    nodes: &[(&str, &Element)],
    output: &Path,
    state: &str,
    calls: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut records = Vec::new();
    for (name, node) in nodes {
        let bounds = node.bounding_rect()?.ok_or("missing native bounds")?;
        let rects = node.client_rects()?;
        let rects = rects
            .iter()
            .map(|r| format!("[{}, {}, {}, {}]", r.x, r.y, r.width, r.height))
            .collect::<Vec<_>>()
            .join(",");
        let raw = node.scroll_metrics()?.ok_or("missing native metrics")?;
        records.push(format!("\"{name}\":{{\"bounds\":[{},{},{},{}],\"rects\":[{rects}],\"offset\":[{},{}],\"dimensions\":[{},{},{},{}],\"raw_metrics\":[{},{},{},{}]}}", bounds.x,bounds.y,bounds.width,bounds.height,node.scroll_left()?,node.scroll_top()?,node.client_width()?,node.client_height()?,node.scroll_width()?,node.scroll_height()?,raw.client_width,raw.client_height,raw.scroll_width,raw.scroll_height));
    }
    let bitmap = document.render_to_bitmap()?;
    let repeat = document.render_to_bitmap()?;
    assert_eq!(bitmap, repeat, "unchanged public frame bytes differed");
    std::fs::write(output.join(format!("{state}.rgba")), bitmap.pixels())?;
    std::fs::write(
        output.join(format!("{state}.png")),
        document.render_to_png_buffer()?,
    )?;
    let target = nodes
        .iter()
        .find(|(name, _)| *name == "target")
        .ok_or("missing target")?
        .1;
    let bounds = target.bounding_rect()?.ok_or("missing target bounds")?;
    let point = (
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    );
    let hit = match document.hit_test(point.0, point.1)? {
        Some(node) => node.get_attribute("id")?,
        None => None,
    };
    let hit = hit.map_or("null".to_owned(), |id| format!("\"{id}\""));
    Ok(format!("{{\"state\":\"{state}\",\"callbacks\":{calls},\"bitmap\":[{},{},{}],\"repeat_exact\":true,\"hit_point\":[{},{}],\"hit\":{hit},\"nodes\":{{{}}}}}",bitmap.width(),bitmap.height(),bitmap.stride(),point.0,point.1,records.join(",")))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let mode = args.next().ok_or("mode required")?;
    if !matches!(mode.as_str(), "block" | "caption" | "transformed-caption") {
        return Err("unknown reveal mode".into());
    }
    std::fs::create_dir_all(&output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_scrollbar_width(ScrollbarWidth::None)?;
    let mut structural = Vec::new();
    let parent = if mode == "block" {
        root.clone()
    } else {
        let table = Element::create(&document, "table")?;
        table.set_attribute("id", "table")?;
        table.set_width(LengthValue::px(140.0))?;
        table.set_border_spacing((Length::px(0.0), Length::px(0.0)))?;
        root.append_child(&table)?;
        let caption = Element::create(&document, "caption")?;
        caption.set_attribute("id", "caption")?;
        caption.set_height(LengthValue::px(30.0))?;
        table.append_child(&caption)?;
        let group = Element::create(&document, "tbody")?;
        group.set_attribute("id", "group")?;
        table.append_child(&group)?;
        let row = Element::create(&document, "tr")?;
        row.set_attribute("id", "row")?;
        group.append_child(&row)?;
        let cell = Element::create(&document, "td")?;
        cell.set_attribute("id", "cell")?;
        row.append_child(&cell)?;
        if mode == "transformed-caption" {
            table.set_transform(TransformList(vec![TransformOperation::Translate(
                LengthValue::px(17.0),
                LengthValue::px(9.0),
            )]))?;
        }
        structural.extend([table, caption, group, row, cell.clone()]);
        cell
    };
    let port = block(&document, &parent, 100.0, 80.0)?;
    port.set_attribute("id", "port")?;
    port.set_overflow(Overflow::Hidden)?;
    port.set_border(Border {
        width: 4.0,
        style: BorderStyle::Solid,
        color: Color::BLACK,
    })?;
    port.set_scroll_padding_left(LengthValue::percent(20.0))?;
    port.set_scroll_padding_top(LengthValue::px(25.0))?;
    port.set_scroll_padding_right(LengthValue::px(5.0))?;
    port.set_scroll_padding_bottom(LengthValue::px(10.0))?;
    let target = block(&document, &port, 20.0, 20.0)?;
    target.set_attribute("id", "target")?;
    target.set_margin_left(LengthValue::px(150.0))?;
    target.set_margin_top(LengthValue::px(120.0))?;
    target.set_scroll_margin_left(LengthValue::px(10.0))?;
    target.set_scroll_margin_top(LengthValue::px(15.0))?;
    target.set_scroll_margin_right(LengthValue::px(9.0))?;
    target.set_scroll_margin_bottom(LengthValue::px(7.0))?;
    target.set_background_color(Color::from_rgba8(0, 128, 0, 255))?;
    let tail = block(&document, &port, 400.0, 300.0)?;
    let options = ScrollIntoViewOptions {
        block: ScrollAlignment::Start,
        inline: ScrollAlignment::Start,
        container: ScrollIntoViewContainer::Nearest,
    };
    let calls = Rc::new(Cell::new(0));
    let stage = Rc::new(Cell::new(0));
    let weak_target = target.downgrade();
    let callback_calls = calls.clone();
    let callback_stage = stage.clone();
    target.on("click", move |_| {
        let target = weak_target.upgrade().expect("live native target");
        if callback_stage.get() == 0 {
            target
                .scroll_into_view(options)
                .expect("native instant reveal");
        } else {
            target
                .smooth_scroll_into_view(options, 100.0)
                .expect("native smooth reveal");
        }
        callback_calls.set(callback_calls.get() + 1);
    })?;
    let owned_initial_rects = target.client_rects()?;
    let owned_initial_metrics = port.scroll_metrics()?.ok_or("missing owned metrics")?;
    let mut records = Vec::new();
    for state in ["initial", "instant", "smooth-end", "accessibility"] {
        match state {
            "instant" => target.click()?,
            "smooth-end" => {
                port.scroll_to(0.0, 0.0)?;
                stage.set(1);
                target.click()?;
                document.advance_time(100.0)?;
            }
            "accessibility" => {
                port.scroll_to(0.0, 0.0)?;
                target.perform_accessibility_action(AccessibilityAction::ScrollIntoView)?;
            }
            _ => {}
        }
        let mut nodes = vec![("port", &port), ("target", &target)];
        nodes.extend(
            ["table", "caption", "group", "row", "cell"]
                .into_iter()
                .zip(structural.iter()),
        );
        records.push(snapshot(&document, &nodes, &output, state, calls.get())?);
        if state == "instant" || state == "smooth-end" {
            assert_eq!((port.scroll_left()?, port.scroll_top()?), (120.0, 80.0));
        }
        if state == "accessibility" {
            assert_eq!((port.scroll_left()?, port.scroll_top()?), (84.0, 77.0));
        }
    }
    assert_eq!(calls.get(), 2);
    assert!(!document.is_animating()?);
    assert_eq!(owned_initial_metrics.client_width, 100.0);
    assert!(!owned_initial_rects.is_empty());
    let owned_initial_rects = owned_initial_rects
        .iter()
        .map(|rect| format!("[{},{},{},{}]", rect.x, rect.y, rect.width, rect.height))
        .collect::<Vec<_>>()
        .join(",");
    std::fs::write(output.join("geometry.json"), format!("{{\"mode\":\"{mode}\",\"scale\":{scale},\"states\":[{}],\"owned_initial_rects\":[{owned_initial_rects}],\"owned_initial_metrics\":[{},{},{},{}]}}\n",records.join(","),owned_initial_metrics.client_width,owned_initial_metrics.client_height,owned_initial_metrics.scroll_width,owned_initial_metrics.scroll_height))?;
    let weak = target.downgrade();
    drop((target, tail, port, parent, structural, root, document));
    assert!(
        weak.upgrade().is_none(),
        "native callback retained document"
    );
    std::fs::write(
        output.join("cleanup.json"),
        "{\"document_released\":true,\"callbacks\":2}\n",
    )?;
    Ok(())
}
