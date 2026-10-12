//! Public native Rust application for retained flex/grid gutter geometry.
//! Requires the additive integer dimension APIs; runtime/Chromium qualification
//! is performed by the release owner after the frozen census is audited.

use openui::prelude::*;
use serde_json::{json, Value};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn percent_track() -> GridTrackList {
    GridTrackList::Tracks(vec![GridTrackComponent::Track(GridTrackSize::Breadth(
        GridTrackBreadth::Length(Length::percent(100.0)),
    ))])
}

fn box_node(document: &Document, parent: &Element) -> Result<Element> {
    let node = Element::create(document, "div")?;
    node.set_display(Display::Block)?;
    parent.append_child(&node)?;
    Ok(node)
}

fn dimensions(node: &Element) -> Result<[i32; 4]> {
    Ok([
        node.client_width()?,
        node.client_height()?,
        node.scroll_width()?,
        node.scroll_height()?,
    ])
}

fn rect(node: &Element) -> Result<Value> {
    let bounds = node.bounding_rect()?.ok_or("live box is missing")?;
    Ok(json!({"x":bounds.x, "y":bounds.y, "width":bounds.width, "height":bounds.height}))
}

fn snapshot(
    document: &Document,
    container: &Element,
    child: &Element,
    output: &Path,
    name: &str,
    expected: [i32; 4],
    calls: u32,
) -> Result<Value> {
    let measured = dimensions(container)?;
    assert_eq!(measured, expected, "native application stage {name}");
    let metrics = container
        .scroll_metrics()?
        .ok_or("scroll area is missing")?;
    let png = document.render_to_png_buffer()?;
    std::fs::write(output.join(format!("{name}.png")), &png)?;
    assert_eq!(
        document.render_to_png_buffer()?,
        png,
        "unchanged repeated native frame differs"
    );
    Ok(json!({"stage":name,"dimensions":measured,"raw_metrics":{
        "client_width":metrics.client_width,"client_height":metrics.client_height,
        "scroll_width":metrics.scroll_width,"scroll_height":metrics.scroll_height},
        "container":rect(container)?,"child":rect(child)?,
        "scroll_offset":[container.scroll_left()?,container.scroll_top()?],"callback_count":calls}))
}

fn consumer(output: &Path, display: Display, scale: f64) -> Result<()> {
    std::fs::create_dir_all(output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let container = box_node(&document, &root)?;
    container.set_position(Position::Absolute)?;
    container.set_left(LengthValue::px(20.0))?;
    container.set_top(LengthValue::px(20.0))?;
    container.set_display(display)?;
    container.set_width(LengthValue::px(100.0))?;
    container.set_height(LengthValue::px(80.0))?;
    container.set_overflow(Overflow::Scroll)?;
    container.set_flex_direction(FlexDirection::Column)?;
    container.set_grid_template_columns(percent_track())?;
    container.set_grid_template_rows(percent_track())?;
    container.set_background_color(Color::from_rgba8(220, 220, 220, 255))?;
    let child = box_node(&document, &container)?;
    child.set_width(LengthValue::percent(100.0))?;
    child.set_height(LengthValue::percent(100.0))?;
    child.set_min_width(LengthValue::px(0.0))?;
    child.set_min_height(LengthValue::px(0.0))?;
    child.set_flex_shrink(0.0)?;
    child.set_background_color(Color::from_rgba8(0, 128, 128, 255))?;
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let weak_child = child.downgrade();
    child.on("click", move |_| {
        let node = weak_child.upgrade().expect("owned native child is live");
        node.set_width(LengthValue::px(150.0))
            .expect("native callback width");
        node.set_height(LengthValue::px(120.0))
            .expect("native callback height");
        observed.set(observed.get() + 1);
    })?;
    let mut states = vec![snapshot(
        &document,
        &container,
        &child,
        output,
        "forced",
        [85, 65, 85, 65],
        calls.get(),
    )?];
    container.set_scrollbar_width(ScrollbarWidth::None)?;
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "none",
        [100, 80, 100, 80],
        calls.get(),
    )?);
    container.set_scrollbar_width(ScrollbarWidth::Thin)?;
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "thin",
        [90, 70, 90, 70],
        calls.get(),
    )?);
    container.set_scrollbar_width(ScrollbarWidth::Auto)?;
    container.set_padding(Edges::all(LengthValue::px(5.0)))?;
    container.set_border(Border {
        width: 2.0,
        style: BorderStyle::Solid,
        color: Color::BLACK,
    })?;
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "padding-border",
        [95, 75, 95, 75],
        calls.get(),
    )?);
    let bounds = container.bounding_rect()?.ok_or("container missing")?;
    assert_eq!((bounds.width, bounds.height), (114.0, 94.0));
    container.set_padding(Edges::all(LengthValue::px(0.0)))?;
    container.set_border(Border {
        width: 0.0,
        style: BorderStyle::Solid,
        color: Color::BLACK,
    })?;
    container.set_width(LengthValue::px(100.25))?;
    container.set_height(LengthValue::px(80.5))?;
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "fractional",
        [85, 66, 85, 66],
        calls.get(),
    )?);
    let saved_metrics = container
        .scroll_metrics()?
        .ok_or("fractional metrics missing")?;
    assert_eq!(
        (saved_metrics.client_width, saved_metrics.client_height),
        (85.25, 65.5)
    );
    container.set_width(LengthValue::px(100.0))?;
    container.set_height(LengthValue::px(80.0))?;
    let absolute = box_node(&document, &container)?;
    absolute.set_position(Position::Absolute)?;
    absolute.set_left(LengthValue::px(0.0))?;
    absolute.set_top(LengthValue::px(0.0))?;
    absolute.set_width(LengthValue::percent(100.0))?;
    absolute.set_height(LengthValue::percent(100.0))?;
    let bounds = absolute.bounding_rect()?.ok_or("absolute child missing")?;
    assert_eq!((bounds.width, bounds.height), (85.0, 65.0));
    absolute.remove()?;
    container.set_overflow(Overflow::Auto)?;
    child.click()?;
    assert_eq!(calls.get(), 1);
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "native-callback-overflow",
        [85, 65, 150, 120],
        calls.get(),
    )?);
    container.scroll_to(1000.0, 1000.0)?;
    assert_eq!(
        (container.scroll_left()?, container.scroll_top()?),
        (65.0, 55.0)
    );
    child.set_width(LengthValue::px(20.0))?;
    child.set_height(LengthValue::px(20.0))?;
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "shrink-removes-gutters",
        [100, 80, 100, 80],
        calls.get(),
    )?);
    assert_eq!(
        (container.scroll_left()?, container.scroll_top()?),
        (0.0, 0.0)
    );
    child.set_width(LengthValue::percent(100.0))?;
    child.set_height(LengthValue::percent(100.0))?;
    container.set_overflow(Overflow::Hidden)?;
    container.set_direction(Direction::Rtl)?;
    container.set_scrollbar_gutter(ScrollbarGutter::Stable)?;
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "stable-rtl",
        [85, 80, 85, 80],
        calls.get(),
    )?);
    container.set_scrollbar_gutter(ScrollbarGutter::StableBothEdges)?;
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "stable-both",
        [70, 80, 70, 80],
        calls.get(),
    )?);
    container.set_direction(Direction::Ltr)?;
    container.set_writing_mode(WritingMode::VerticalRl)?;
    container.set_scrollbar_gutter(ScrollbarGutter::Stable)?;
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "vertical-stable",
        [100, 65, 100, 65],
        calls.get(),
    )?);
    container.set_scrollbar_gutter(ScrollbarGutter::StableBothEdges)?;
    states.push(snapshot(
        &document,
        &container,
        &child,
        output,
        "vertical-both",
        [100, 50, 100, 50],
        calls.get(),
    )?);
    container.set_display(Display::None)?;
    assert_eq!(dimensions(&container)?, [0; 4]);
    container.set_display(display)?;
    assert_eq!(dimensions(&container)?, [100, 50, 100, 50]);
    assert_eq!(
        (saved_metrics.client_width, saved_metrics.client_height),
        (85.25, 65.5)
    );
    std::fs::write(
        output.join("geometry.json"),
        serde_json::to_vec_pretty(&states)?,
    )?;
    child.remove()?;
    container.remove()?;
    Ok(())
}

// Exercise the class/viewport rules through public application-owned APIs.
// Captures are diagnostic until independently paired with immutable Chromium.
fn class_policy_consumer(output: &Path, scale: f64) -> Result<()> {
    std::fs::create_dir_all(output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let table = Element::create(&document, "table")?;
    root.append_child(&table)?;
    table.set_width(LengthValue::px(100.0))?;
    table.set_height(LengthValue::px(80.0))?;
    table.set_border_spacing((Length::px(0.0), Length::px(0.0)))?;
    table.set_scrollbar_gutter(ScrollbarGutter::Stable)?;
    let group = Element::create(&document, "tbody")?;
    let row = Element::create(&document, "tr")?;
    let cell = Element::create(&document, "td")?;
    table.append_child(&group)?;
    group.append_child(&row)?;
    row.append_child(&cell)?;
    cell.set_height(LengthValue::px(80.0))?;
    cell.set_background_color(Color::from_rgba8(0, 128, 0, 255))?;
    let mut states = Vec::new();
    for (name, overflow) in [
        ("table-auto", Overflow::Auto),
        ("table-scroll", Overflow::Scroll),
    ] {
        table.set_overflow(overflow)?;
        states.push(snapshot(
            &document,
            &table,
            &cell,
            output,
            name,
            [100, 80, 100, 80],
            0,
        )?);
        table.scroll_to(1000.0, 1000.0)?;
        assert_eq!((table.scroll_left()?, table.scroll_top()?), (0.0, 0.0));
    }
    table.set_overflow(Overflow::Hidden)?;
    states.push(snapshot(
        &document,
        &table,
        &cell,
        output,
        "table-hidden-stable",
        [85, 80, 100, 80],
        0,
    )?);
    let table_rect = table.bounding_rect()?.ok_or("table bounds missing")?;
    let row_rect = row.bounding_rect()?.ok_or("row bounds missing")?;
    let cell_rect = cell.bounding_rect()?.ok_or("cell bounds missing")?;
    assert_eq!((row_rect.width, cell_rect.width), (85.0, 100.0));
    assert_eq!(cell_rect.x - table_rect.x, 0.0);
    table.scroll_to(1000.0, 1000.0)?;
    assert_eq!((table.scroll_left()?, table.scroll_top()?), (15.0, 0.0));
    table.scroll_to(0.0, 0.0)?;
    table.set_direction(Direction::Rtl)?;
    states.push(snapshot(
        &document,
        &table,
        &cell,
        output,
        "table-rtl-stable",
        [85, 80, 100, 80],
        0,
    )?);
    let table_rect = table.bounding_rect()?.ok_or("table bounds missing")?;
    let row_rect = row.bounding_rect()?.ok_or("row bounds missing")?;
    let cell_rect = cell.bounding_rect()?.ok_or("cell bounds missing")?;
    assert_eq!(row_rect.x - table_rect.x, 15.0);
    assert_eq!(cell_rect.x - table_rect.x, 0.0);
    table.set_direction(Direction::Ltr)?;
    let caption = Element::create(&document, "caption")?;
    caption.set_height(LengthValue::px(20.0))?;
    table.append_child(&caption)?;
    states.push(snapshot(
        &document,
        &table,
        &cell,
        output,
        "captioned-table-stable",
        [85, 100, 100, 100],
        0,
    )?);
    table.set_overflow_y(Overflow::Auto)?;
    states.push(snapshot(
        &document,
        &table,
        &cell,
        output,
        "captioned-table-mixed",
        [100, 100, 100, 100],
        0,
    )?);
    table.remove()?;

    let image = Element::create(&document, "img")?;
    root.append_child(&image)?;
    let resource = document.register_image_resource("native-gutter.svg", "image/svg+xml",
        "567d510cd044916f0d5ee55b185054feedfe5caeece0bbda2203ef7b628bc000", br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80"><rect width="100" height="80" fill="#008000"/></svg>"##.to_vec())?;
    image.set_image_resource(resource, Some((100.0, 80.0)))?;
    image.set_width(LengthValue::px(100.0))?;
    image.set_height(LengthValue::px(80.0))?;
    image.set_overflow(Overflow::Scroll)?;
    states.push(snapshot(
        &document,
        &image,
        &image,
        output,
        "image-scroll-no-gutter",
        [100, 80, 100, 80],
        0,
    )?);
    image.set_scrollbar_gutter(ScrollbarGutter::Stable)?;
    states.push(snapshot(
        &document,
        &image,
        &image,
        output,
        "image-scroll-stable",
        [85, 80, 85, 80],
        0,
    )?);
    image.set_overflow(Overflow::Hidden)?;
    image.set_writing_mode(WritingMode::VerticalRl)?;
    image.set_scrollbar_gutter(ScrollbarGutter::StableBothEdges)?;
    states.push(snapshot(
        &document,
        &image,
        &image,
        output,
        "image-vertical-both",
        [100, 50, 100, 50],
        0,
    )?);
    image.scroll_to(-1000.0, 1000.0)?;
    assert_eq!((image.scroll_left()?, image.scroll_top()?), (0.0, 0.0));
    image.remove()?;

    let html = Element::create(&document, "html")?;
    let body = Element::create(&document, "body")?;
    let content = Element::create(&document, "div")?;
    root.append_child(&html)?;
    html.append_child(&body)?;
    body.append_child(&content)?;
    body.set_width(LengthValue::px(100.0))?;
    body.set_height(LengthValue::px(80.0))?;
    content.set_width(LengthValue::px(150.0))?;
    content.set_height(LengthValue::px(120.0))?;
    content.set_background_color(Color::from_rgba8(0, 128, 0, 255))?;
    body.set_overflow(Overflow::Scroll)?;
    states.push(snapshot(
        &document,
        &body,
        &content,
        output,
        "body-propagated",
        [100, 80, 150, 120],
        0,
    )?);
    assert_eq!(dimensions(&root)?, [305, 225, 305, 225]);
    assert_eq!(dimensions(&html)?, dimensions(&root)?);
    body.scroll_to(1000.0, 1000.0)?;
    assert_eq!((body.scroll_left()?, body.scroll_top()?), (0.0, 0.0));
    html.set_overflow(Overflow::Hidden)?;
    states.push(snapshot(
        &document,
        &body,
        &content,
        output,
        "body-local",
        [85, 65, 150, 120],
        0,
    )?);
    body.scroll_to(1000.0, 1000.0)?;
    assert_eq!((body.scroll_left()?, body.scroll_top()?), (65.0, 55.0));
    html.set_overflow(Overflow::Visible)?;
    states.push(snapshot(
        &document,
        &body,
        &content,
        output,
        "body-restored",
        [100, 80, 150, 120],
        0,
    )?);
    assert_eq!((body.scroll_left()?, body.scroll_top()?), (0.0, 0.0));
    std::fs::write(
        output.join("geometry.json"),
        serde_json::to_vec_pretty(&states)?,
    )?;
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for (name, display) in [("flex", Display::Flex), ("grid", Display::Grid)] {
            consumer(&output.join(format!("{name}@{scale}")), display, scale)?;
        }
        class_policy_consumer(&output.join(format!("class-policy@{scale}")), scale)?;
    }
    Ok(())
}
