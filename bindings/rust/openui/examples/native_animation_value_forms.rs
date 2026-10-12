//! Public native consumer for shared author/renderer animation values.
//! Usage: native_animation_value_forms OUTPUT SCALE
use openui::prelude::*;
use openui::typed_style::{
    AnimationOptions, CompositeOperation, FillMode, Keyframes, RendererStyleValue, StyleProperty,
    StyleValue,
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
    target: &Element,
    output: &Path,
    state: &str,
    callbacks: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let bounds = target.bounding_rect()?.ok_or("missing authored bounds")?;
    let rects = target.client_rects()?;
    let metrics = target
        .scroll_metrics()?
        .ok_or("missing owned scroll metrics")?;
    let style = target.computed_style()?;
    let bitmap = document.render_to_bitmap()?;
    assert_eq!(
        bitmap,
        document.render_to_bitmap()?,
        "unchanged native frame"
    );
    std::fs::write(output.join(format!("{state}.rgba")), bitmap.pixels())?;
    std::fs::write(
        output.join(format!("{state}.png")),
        document.render_to_png_buffer()?,
    )?;
    let hit = document
        .hit_test(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )?
        .map(|node| node.get_attribute("id"))
        .transpose()?
        .flatten();
    let rects = rects
        .iter()
        .map(|r| format!("[{},{},{},{}]", r.x, r.y, r.width, r.height))
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("{{\"state\":\"{state}\",\"callbacks\":{callbacks},\"width\":{},\"opacity\":{},\"background\":[{},{},{},{}],\"bounds\":[{},{},{},{}],\"rects\":[{rects}],\"hit\":{},\"dimensions\":[{},{},{},{}],\"raw_metrics\":[{},{},{},{}],\"scroll\":[{},{}],\"repeat_exact\":true}}",
        style.width.value(),style.opacity,style.background_color.r,style.background_color.g,style.background_color.b,style.background_color.a,bounds.x,bounds.y,bounds.width,bounds.height,
        hit.map_or("null".to_owned(),|id| format!("\"{id}\"")),target.client_width()?,target.client_height()?,target.scroll_width()?,target.scroll_height()?,metrics.client_width,metrics.client_height,metrics.scroll_width,metrics.scroll_height,target.scroll_left()?,target.scroll_top()?))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("output required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    std::fs::create_dir_all(&output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(160.0, 120.0, scale)?)?;
    document.body().set_background_color(Color::WHITE)?;
    let target = Element::create(&document, "div")?;
    target.set_attribute("id", "target")?;
    target.set_width(LengthValue::px(10.0))?;
    target.set_height(LengthValue::px(20.0))?;
    target.set_background_color(Color::from_rgba_f32(0.25, 0.0, 0.0, 0.5))?;
    target.set_opacity(0.25)?;
    document.body().append_child(&target)?;
    let calls = Rc::new(Cell::new(0_u32));
    let callback_calls = calls.clone();
    let weak = target.downgrade();
    target.on("animationcancel", move |_| {
        callback_calls.set(callback_calls.get() + 1);
        weak.upgrade()
            .expect("owned callback target")
            .set_attribute("callback", "observed")
            .expect("callback after Engine borrow");
    })?;
    let width = target.animate(
        StyleProperty::Width,
        Keyframes::from_values(
            StyleValue::Renderer(RendererStyleValue::Width(openui::typed_style::Length::px(
                5.0,
            ))),
            StyleValue::from(LengthValue::px(5.0)),
        ),
        options(CompositeOperation::Add),
    )?;
    let opacity = target.animate(
        StyleProperty::Opacity,
        Keyframes::from_values(
            StyleValue::Renderer(RendererStyleValue::Opacity(0.375)),
            StyleValue::Number(0.375),
        ),
        options(CompositeOperation::Add),
    )?;
    let blue = Color::from_rgba_f32(0.0, 0.0, 0.25, 0.5);
    let color = target.animate(
        StyleProperty::BackgroundColor,
        Keyframes::from_values(
            StyleValue::Renderer(RendererStyleValue::BackgroundColor(blue)),
            StyleValue::Color(blue),
        ),
        options(CompositeOperation::Add),
    )?;
    document.advance_time(0.0)?;
    assert_eq!(target.computed_style()?.width.value(), 15.0);
    assert_eq!(target.computed_style()?.opacity, 0.625);
    assert_eq!(
        target.computed_style()?.background_color,
        Color::from_rgba_f32(0.25, 0.0, 0.25, 0.5)
    );
    let mut states = vec![snapshot(
        &document,
        &target,
        &output,
        "mixed-add",
        calls.get(),
    )?];
    target.set_width(LengthValue::px(20.0))?;
    target.set_opacity(0.5)?;
    assert_eq!(target.computed_style()?.width.value(), 25.0);
    assert_eq!(target.computed_style()?.opacity, 0.875);
    states.push(snapshot(
        &document,
        &target,
        &output,
        "author-change",
        calls.get(),
    )?);
    document.cancel_animation(width)?;
    document.cancel_animation(opacity)?;
    document.cancel_animation(color)?;
    assert_eq!(calls.get(), 3);
    assert_eq!(
        target.get_attribute("callback")?.as_deref(),
        Some("observed")
    );
    assert_eq!(target.computed_style()?.width.value(), 20.0);
    assert_eq!(target.computed_style()?.opacity, 0.5);
    states.push(snapshot(
        &document,
        &target,
        &output,
        "cancel-restore",
        calls.get(),
    )?);
    std::fs::write(output.join("metadata.json"),format!("{{\"schema\":1,\"qualification\":false,\"native_api\":true,\"scale\":{scale},\"states\":[{}],\"required_open\":[\"compile/runtime\",\"exact Chromium\",\"currentcolor/modern/specialized compound interpolation\"]}}",states.join(",")))?;
    target.remove()?;
    drop(target);
    drop(document);
    Ok(())
}
