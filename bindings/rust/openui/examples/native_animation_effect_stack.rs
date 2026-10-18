//! Real public native consumer for effect ownership, app mutations and callbacks.
//! Usage: native_animation_effect_stack OUTPUT SCALE
use openui::prelude::*;
use openui::typed_style::{
    AnimationOptions, CompositeOperation, FillMode, Keyframes, StyleProperty,
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
fn snapshot(
    document: &Document,
    node: &Element,
    output: &Path,
    state: &str,
    calls: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let bounds = node.bounding_rect()?.ok_or("missing authored bounds")?;
    let rects = node.client_rects()?;
    let metrics = node.scroll_metrics()?.ok_or("missing owned metrics")?;
    let style = node.computed_style()?;
    let bitmap = document.render_to_bitmap()?;
    let repeat = document.render_to_bitmap()?;
    assert_eq!(bitmap, repeat, "unchanged public frame pixels");
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
        .map(|r| format!("[{},{},{},{}]", r.x, r.y, r.width, r.height))
        .collect::<Vec<_>>()
        .join(",");
    let events = document.drain_animation_events()?;
    Ok(format!("{{\"state\":\"{state}\",\"callbacks\":{calls},\"width\":{},\"opacity\":{},\"background_color\":[{},{},{},{}],\"bounds\":[{},{},{},{}],\"rects\":[{rects}],\"hit\":{},\"dimensions\":[{},{},{},{}],\"raw_metrics\":[{},{},{},{}],\"scroll\":[{},{}],\"event_count\":{},\"repeat_exact\":true}}",
        style.width.value(),style.opacity,style.background_color.r,style.background_color.g,style.background_color.b,style.background_color.a,bounds.x,bounds.y,bounds.width,bounds.height,
        hit.map_or("null".to_owned(),|id| format!("\"{id}\"")),
        node.client_width()?,node.client_height()?,node.scroll_width()?,node.scroll_height()?,
        metrics.client_width,metrics.client_height,metrics.scroll_width,metrics.scroll_height,
        node.scroll_left()?,node.scroll_top()?,events.len()))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    std::fs::create_dir_all(&output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(160.0, 120.0, scale)?)?;
    document.body().set_background_color(Color::WHITE)?;
    let node = Element::create(&document, "div")?;
    node.set_attribute("id", "target")?;
    node.set_width(LengthValue::px(10.0))?;
    node.set_height(LengthValue::px(20.0))?;
    node.set_background_color(Color::RED)?;
    document.body().append_child(&node)?;
    let calls = Rc::new(Cell::new(0_u32));
    let callback_calls = calls.clone();
    let weak = node.downgrade();
    node.on("animationcancel", move |_| {
        let node = weak.upgrade().expect("callback target remains owned");
        callback_calls.set(callback_calls.get() + 1);
        // This public setter proves callbacks run after engine borrows end,
        // and an unrelated authored value survives the active width stack.
        node.set_height(LengthValue::px(20.0 + callback_calls.get() as f32))
            .expect("native callback mutation");
    })?;
    let lower = node.animate(
        StyleProperty::Width,
        Keyframes::from_values(LengthValue::px(5.0), LengthValue::px(5.0)),
        options(CompositeOperation::Add),
    )?;
    let survivor = node.animate(
        StyleProperty::Width,
        Keyframes::from_values(LengthValue::px(7.0), LengthValue::px(7.0)),
        options(CompositeOperation::Add),
    )?;
    document.advance_time(0.0)?;
    assert_eq!(node.computed_style()?.width.value(), 22.0);
    let initial_owned = node.client_rects()?;
    let mut records = vec![snapshot(&document, &node, &output, "two-add", calls.get())?];
    node.set_width(LengthValue::px(20.0))?;
    assert_eq!(node.computed_style()?.width.value(), 32.0);
    records.push(snapshot(
        &document,
        &node,
        &output,
        "authored-change",
        calls.get(),
    )?);
    let top = node.animate(
        StyleProperty::Width,
        Keyframes::from_values(LengthValue::px(40.0), LengthValue::px(40.0)),
        options(CompositeOperation::Replace),
    )?;
    document.advance_time(0.0)?;
    assert_eq!(node.computed_style()?.width.value(), 40.0);
    records.push(snapshot(
        &document,
        &node,
        &output,
        "top-replace",
        calls.get(),
    )?);
    document.cancel_animation(lower)?;
    assert_eq!(node.computed_style()?.width.value(), 40.0);
    records.push(snapshot(
        &document,
        &node,
        &output,
        "lower-cancel",
        calls.get(),
    )?);
    document.cancel_animation(top)?;
    assert_eq!(node.computed_style()?.width.value(), 27.0);
    records.push(snapshot(
        &document,
        &node,
        &output,
        "survivor",
        calls.get(),
    )?);
    let clone = node.clone_subtree()?;
    assert_eq!(clone.computed_style()?.width.value(), 20.0);
    document.cancel_animation(survivor)?;
    assert_eq!(node.computed_style()?.width.value(), 20.0);
    assert_eq!(calls.get(), 3);
    records.push(snapshot(
        &document,
        &node,
        &output,
        "current-base",
        calls.get(),
    )?);
    assert_eq!(
        initial_owned[0].width, 22.0,
        "owned geometry changed after later mutations"
    );
    let accumulate = node.animate(
        StyleProperty::Width,
        Keyframes::from_values(LengthValue::px(3.0), LengthValue::px(3.0)),
        AnimationOptions {
            iterations: IterationCount::Number(3.0),
            ..options(CompositeOperation::Accumulate)
        },
    )?;
    document.advance_time(150.0)?;
    assert_eq!(node.computed_style()?.width.value(), 23.0);
    records.push(snapshot(
        &document,
        &node,
        &output,
        "effect-accumulate",
        calls.get(),
    )?);
    document.cancel_animation(accumulate)?;
    assert_eq!(node.computed_style()?.width.value(), 20.0);
    records.push(snapshot(
        &document,
        &node,
        &output,
        "accumulate-cancel",
        calls.get(),
    )?);
    // Native Color retains the same premultiplied working stack as Engine.
    node.set_background_color(Color::from_rgba_f32(1.0, 0.0, 0.0, 0.75))?;
    let blue = node.animate(
        StyleProperty::BackgroundColor,
        Keyframes::from_values(
            Color::from_rgba_f32(0.0, 0.0, 1.0, 0.5),
            Color::from_rgba_f32(0.0, 0.0, 1.0, 0.5),
        ),
        options(CompositeOperation::Add),
    )?;
    let green = node.animate(
        StyleProperty::BackgroundColor,
        Keyframes::from_values(
            Color::from_rgba_f32(0.0, 1.0, 0.0, 0.25),
            Color::from_rgba_f32(0.0, 1.0, 0.0, 0.25),
        ),
        options(CompositeOperation::Accumulate),
    )?;
    document.advance_time(0.0)?;
    assert_eq!(
        node.computed_style()?.background_color,
        Color::from_rgba_f32(0.75, 0.25, 0.5, 1.0)
    );
    records.push(snapshot(
        &document,
        &node,
        &output,
        "color-stack",
        calls.get(),
    )?);
    document.cancel_animation(blue)?;
    assert_eq!(
        node.computed_style()?.background_color,
        Color::from_rgba_f32(0.75, 0.25, 0.0, 1.0)
    );
    records.push(snapshot(
        &document,
        &node,
        &output,
        "color-survivor",
        calls.get(),
    )?);
    document.cancel_animation(green)?;
    assert_eq!(
        node.computed_style()?.background_color,
        Color::from_rgba_f32(1.0, 0.0, 0.0, 0.75)
    );
    records.push(snapshot(
        &document,
        &node,
        &output,
        "color-base",
        calls.get(),
    )?);
    node.remove()?;
    clone.remove()?;
    assert!(node.computed_style().is_err());
    assert!(clone.computed_style().is_err());
    let cleaned = document.render_to_bitmap()?;
    assert_eq!(cleaned, document.render_to_bitmap()?);
    std::fs::write(output.join("cleanup.rgba"), cleaned.pixels())?;
    std::fs::write(output.join("metadata.json"),format!("{{\"schema\":1,\"runtime_qualification\":false,\"scale\":{scale},\"states\":[{}],\"cleanup\":true,\"final_callbacks\":{}}}\n",records.join(","),calls.get()))?;
    Ok(())
}
