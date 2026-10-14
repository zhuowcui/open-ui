//! A native application consumer for fragmented flow-root geometry and input.

use openui::prelude::*;
use openui::ViewportMetrics;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

fn block(document: &Document, parent: &Element) -> Result<Element, openui::Error> {
    let element = Element::create(document, "div")?;
    element.set_display(Display::Block)?;
    parent.append_child(&element)?;
    Ok(element)
}

fn rectangles(element: &Element) -> Result<String, openui::Error> {
    let rectangle = |rect: Rect| {
        format!(
            "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
            rect.x, rect.y, rect.width, rect.height
        )
    };
    let bounds = element
        .bounding_rect()?
        .map(rectangle)
        .unwrap_or_else(|| "null".into());
    let rects = element
        .client_rects()?
        .into_iter()
        .map(rectangle)
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("{{\"bounds\":{bounds},\"rects\":[{rects}]}}"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().unwrap_or_else(|| "1".into()).parse()?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 340.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let columns = block(&document, &root)?;
    columns.set_width(LengthValue::px(300.0))?;
    columns.set_column_count(Some(3))?;
    columns.set_column_gap(LengthValue::px(24.0))?;
    columns.set_background_color(Color::from_rgba8(128, 128, 128, 255))?;
    let wrapper = block(&document, &columns)?;
    wrapper.set_display(Display::FlowRoot)?;
    wrapper.set_max_height(LengthValue::px(160.0))?;
    wrapper.set_background_color(Color::from_rgba8(255, 255, 0, 255))?;
    let child = block(&document, &wrapper)?;
    child.set_width(LengthValue::px(50.0))?;
    child.set_height(LengthValue::px(200.0))?;
    child.set_border(Border {
        width: 3.0,
        style: BorderStyle::Solid,
        color: Color::BLACK,
    })?;
    child.set_id("overflowing child")?;
    let clicks = Rc::new(Cell::new(0));
    let observed = clicks.clone();
    child.on("click", move |_| observed.set(observed.get() + 1))?;
    assert_eq!(
        child
            .client_rects()?
            .iter()
            .map(|rect| (rect.x, rect.y, rect.width, rect.height))
            .collect::<Vec<_>>(),
        [
            (0.0, 0.0, 56.0, 68.671875),
            (108.0, 0.0, 56.0, 68.671875),
            (216.0, 0.0, 56.0, 68.65625)
        ]
    );
    let hit = document.hit_test(240.0, 40.0)?.ok_or("child not hit")?;
    assert_eq!(
        hit.get_attribute("id")?.as_deref(),
        Some("overflowing child")
    );
    hit.click()?;
    assert_eq!(clicks.get(), 1);
    let geometry = format!(
        "{{\"nodes\":{{\"columns\":{},\"limit\":{},\"border\":{}}}}}\n",
        rectangles(&columns)?,
        rectangles(&wrapper)?,
        rectangles(&child)?
    );
    std::fs::create_dir(&output)?;
    std::fs::write(output.join("geometry.json"), geometry)?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    Ok(())
}
