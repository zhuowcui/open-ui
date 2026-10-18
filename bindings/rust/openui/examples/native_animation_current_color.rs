//! Public native currentcolor/text-fill consumer with retained pixels and callbacks.
//! Usage: native_animation_current_color OUTPUT SCALE
//! This consumer is not a qualification result until executed by the locked matrix.
use openui::prelude::*;
use openui::typed_style::{
    AnimationOptions, Color, CompositeOperation, FillMode, Keyframes, StyleColor, StyleProperty,
    TextDecorationLine,
};
use std::{cell::Cell, path::Path, rc::Rc};

fn options(composite: CompositeOperation) -> AnimationOptions {
    AnimationOptions {
        duration_ms: 100.0,
        fill: FillMode::Both,
        composite,
        ..AnimationOptions::default()
    }
}
fn color_json(color: Color) -> String {
    format!("[{},{},{},{}]", color.r, color.g, color.b, color.a)
}
fn snapshot(
    document: &Document,
    node: &Element,
    output: &Path,
    state: &str,
    callbacks: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let style = node.computed_style()?;
    let bounds = node.bounding_rect()?.ok_or("missing authored bounds")?;
    let rects = node.client_rects()?;
    let metrics = node.scroll_metrics()?.ok_or("missing scroll metrics")?;
    let bitmap = document.render_to_bitmap()?;
    assert_eq!(
        bitmap,
        document.render_to_bitmap()?,
        "unchanged native pixels"
    );
    std::fs::write(output.join(format!("{state}.rgba")), bitmap.pixels())?;
    std::fs::write(
        output.join(format!("{state}.png")),
        document.render_to_png_buffer()?,
    )?;
    let hit = document.hit_test(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )?;
    let hit = hit
        .map(|node| node.get_attribute("id"))
        .transpose()?
        .flatten();
    let rects = rects
        .iter()
        .map(|rect| format!("[{},{},{},{}]", rect.x, rect.y, rect.width, rect.height))
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("{{\"state\":\"{state}\",\"callbacks\":{callbacks},\"own_color\":{},\"fill\":{},\"background\":{},\"decoration\":{},\"bounds\":[{},{},{},{}],\"rects\":[{rects}],\"hit\":{},\"dimensions\":[{},{},{},{}],\"raw_metrics\":[{},{},{},{}],\"scroll\":[{},{}],\"repeat_exact\":true}}",
        color_json(style.color),color_json(style.effective_text_fill_color()),color_json(style.background_color),
        color_json(style.text_decoration_color.resolve(&style.effective_text_fill_color())),
        bounds.x,bounds.y,bounds.width,bounds.height,hit.map_or("null".to_owned(),|id|format!("\"{id}\"")),
        node.client_width()?,node.client_height()?,node.scroll_width()?,node.scroll_height()?,
        metrics.client_width,metrics.client_height,metrics.scroll_width,metrics.scroll_height,
        node.scroll_left()?,node.scroll_top()?))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    std::fs::create_dir_all(&output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(160.0, 120.0, scale)?)?;
    let body = document.body();
    body.set_color(Color::RED)?;
    body.set_background_color(Color::WHITE)?;
    let node = Element::create(&document, "div")?;
    node.set_attribute("id", "currentcolor-target")?;
    node.set_width(LengthValue::px(100.0))?;
    node.set_height(LengthValue::px(60.0))?;
    node.set_color_value(StyleColor::CurrentColor)?;
    node.set_background_color_value(StyleColor::CurrentColor)?;
    node.set_webkit_text_fill_color(StyleColor::CurrentColor)?;
    node.set_text_decoration_line(TextDecorationLine::UNDERLINE)?;
    node.create_text_child("native")?;
    body.append_child(&node)?;
    let calls = Rc::new(Cell::new(0_u32));
    let callback_calls = calls.clone();
    let weak = node.downgrade();
    node.on("animationcancel", move |_| {
        let node = weak.upgrade().expect("owned native callback target");
        callback_calls.set(callback_calls.get() + 1);
        node.set_background_color_value(StyleColor::CurrentColor)
            .expect("callback after borrow release");
    })?;
    let mut rows = vec![snapshot(
        &document,
        &node,
        &output,
        "authored",
        calls.get(),
    )?];
    // Deliberately create the dependent effect first: it still uses final own color.
    let background = node.animate(
        StyleProperty::BackgroundColor,
        Keyframes::from_values(StyleColor::CurrentColor, StyleColor::CurrentColor),
        options(CompositeOperation::Replace),
    )?;
    let own = node.animate(
        StyleProperty::Color,
        Keyframes::from_values(
            StyleColor::Resolved(Color::BLUE),
            StyleColor::Resolved(Color::BLUE),
        ),
        options(CompositeOperation::Replace),
    )?;
    assert_eq!(node.computed_style()?.background_color, Color::BLUE);
    rows.push(snapshot(
        &document,
        &node,
        &output,
        "own-effect",
        calls.get(),
    )?);
    let fill = node.animate(
        StyleProperty::WebkitTextFillColor,
        Keyframes::from_values(
            StyleColor::Resolved(Color::RED),
            StyleColor::Resolved(Color::RED),
        ),
        options(CompositeOperation::Replace),
    )?;
    assert_eq!(
        node.computed_style()?.effective_text_fill_color(),
        Color::RED
    );
    rows.push(snapshot(
        &document,
        &node,
        &output,
        "fill-effect",
        calls.get(),
    )?);
    body.set_color(Color::WHITE)?;
    rows.push(snapshot(
        &document,
        &node,
        &output,
        "parent-mutation",
        calls.get(),
    )?);
    document.cancel_animation(own)?;
    assert_eq!(node.computed_style()?.color, Color::WHITE);
    assert_eq!(node.computed_style()?.background_color, Color::WHITE);
    rows.push(snapshot(
        &document,
        &node,
        &output,
        "own-cancel",
        calls.get(),
    )?);
    document.cancel_animation(fill)?;
    assert_eq!(
        node.computed_style()?.text_fill_color,
        StyleColor::CurrentColor
    );
    rows.push(snapshot(
        &document,
        &node,
        &output,
        "fill-cancel",
        calls.get(),
    )?);
    document.cancel_animation(background)?;
    assert_eq!(calls.get(), 3);
    rows.push(snapshot(
        &document,
        &node,
        &output,
        "all-cancel",
        calls.get(),
    )?);
    let events = document.drain_animation_events()?;
    node.detach()?;
    assert!(node.bounding_rect()?.is_none());
    std::fs::write(output.join("metadata.json"),format!("{{\"qualification\":false,\"scale\":{scale},\"callbacks\":{},\"retained_events\":{},\"detached\":true,\"states\":[{}]}}\n",calls.get(),events.len(),rows.join(",")))?;
    Ok(())
}
