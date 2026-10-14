//! Public native transform-owner consumer; usage: OUTPUT SCALE.
use openui::prelude::*;
use openui::typed_style::{
    AnimationOptions, FillMode, Keyframes, Position, StyleProperty, TransformList,
    TransformOperation,
};
use std::{cell::Cell, path::Path, rc::Rc};
fn snapshot(
    document: &Document,
    owner: &Element,
    child: &Element,
    output: &Path,
    state: &str,
    callbacks: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let own = owner.bounding_rect()?.ok_or("missing owner box")?;
    let child_box = child.bounding_rect()?.ok_or("missing positioned child")?;
    let rects = child.client_rects()?;
    let bitmap = document.render_to_bitmap()?;
    assert_eq!(
        bitmap,
        document.render_to_bitmap()?,
        "unchanged retained frame"
    );
    std::fs::write(output.join(format!("{state}.rgba")), bitmap.pixels())?;
    std::fs::write(
        output.join(format!("{state}.png")),
        document.render_to_png_buffer()?,
    )?;
    let hit = document
        .hit_test(
            child_box.x + child_box.width / 2.0,
            child_box.y + child_box.height / 2.0,
        )?
        .map(|v| v.get_attribute("id"))
        .transpose()?
        .flatten();
    let metrics = owner.scroll_metrics()?.ok_or("missing owned metrics")?;
    let child_rects = rects
        .iter()
        .map(|r| format!("[{},{},{},{}]", r.x, r.y, r.width, r.height))
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("{{\"state\":\"{state}\",\"owner\":[{},{},{},{}],\"child\":[{},{},{},{}],\"child_rects\":[{child_rects}],\"hit\":{},\"dimensions\":[{},{},{},{}],\"raw_metrics\":[{},{},{},{}],\"scroll\":[{},{}],\"callbacks\":{callbacks},\"repeat_exact\":true}}",own.x,own.y,own.width,own.height,child_box.x,child_box.y,child_box.width,child_box.height,hit.map_or("null".to_owned(),|id|format!("\"{id}\"")),owner.client_width()?,owner.client_height()?,owner.scroll_width()?,owner.scroll_height()?,metrics.client_width,metrics.client_height,metrics.scroll_width,metrics.scroll_height,owner.scroll_left()?,owner.scroll_top()?))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("output required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    std::fs::create_dir_all(&output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(160.0, 120.0, scale)?)?;
    document.body().set_background_color(Color::WHITE)?;
    let owner = Element::create(&document, "div")?;
    owner.set_attribute("id", "owner")?;
    owner.set_width(LengthValue::px(80.0))?;
    owner.set_height(LengthValue::px(60.0))?;
    owner.set_background_color(Color::GREEN)?;
    owner.set_property(StyleProperty::MarginLeft, LengthValue::px(20.0).into())?;
    owner.set_property(StyleProperty::MarginTop, LengthValue::px(30.0).into())?;
    document.body().append_child(&owner)?;
    let child = Element::create(&document, "button")?;
    child.set_attribute("id", "fixed-child")?;
    child.set_width(LengthValue::px(8.0))?;
    child.set_height(LengthValue::px(8.0))?;
    child.set_background_color(Color::RED)?;
    child.set_property(StyleProperty::Position, Position::Fixed.into())?;
    child.set_property(StyleProperty::Top, LengthValue::px(0.0).into())?;
    child.set_property(StyleProperty::Left, LengthValue::px(0.0).into())?;
    owner.append_child(&child)?;
    let callbacks = Rc::new(Cell::new(0_u32));
    let observed = callbacks.clone();
    let weak = child.downgrade();
    owner.on("animationcancel", move |_| {
        observed.set(observed.get() + 1);
        weak.upgrade()
            .expect("owned callback target")
            .set_attribute("data-cancel", "observed")
            .expect("native callback after borrows");
    })?;
    let mut records = vec![snapshot(
        &document,
        &owner,
        &child,
        &output,
        "viewport-fixed",
        callbacks.get(),
    )?];
    let identity = TransformList(vec![TransformOperation::Translate(
        LengthValue::percent(0.0),
        LengthValue::Em(0.0),
    )]);
    owner.set_property(StyleProperty::Transform, identity.clone().into())?;
    records.push(snapshot(
        &document,
        &owner,
        &child,
        &output,
        "specified-identity",
        callbacks.get(),
    )?);
    let none = TransformList::default();
    let animation = owner.animate(
        StyleProperty::Transform,
        Keyframes::from_values(none.clone(), none.clone()),
        AnimationOptions {
            duration_ms: 100.0,
            fill: FillMode::Both,
            ..AnimationOptions::default()
        },
    )?;
    document.advance_time(0.0)?;
    records.push(snapshot(
        &document,
        &owner,
        &child,
        &output,
        "none-replace",
        callbacks.get(),
    )?);
    document.cancel_animation(animation)?;
    records.push(snapshot(
        &document,
        &owner,
        &child,
        &output,
        "revealed-identity",
        callbacks.get(),
    )?);
    owner.set_property(StyleProperty::Transform, none.into())?;
    records.push(snapshot(
        &document,
        &owner,
        &child,
        &output,
        "removed-source",
        callbacks.get(),
    )?);
    assert_eq!(callbacks.get(), 1);
    assert_eq!(
        child.get_attribute("data-cancel")?.as_deref(),
        Some("observed")
    );
    owner.remove()?;
    assert!(child.downgrade().upgrade().is_none());
    std::fs::write(output.join("metadata.json"),format!("{{\"schema\":1,\"qualification\":false,\"scale\":{scale},\"states\":[{}],\"callbacks_after_borrows\":true,\"cleanup\":true}}\n",records.join(",")))?;
    Ok(())
}
