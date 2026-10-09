//! Uncompiled native app draft for disabled control appearance.
//! Parameters select geometry and native state, never test IDs or expected pixels.
use openui::prelude::*;
use serde_json::json;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let phase: f32 = args.next().ok_or("phase required")?.parse()?;
    let disabled = args.next().ok_or("group state required")? == "disabled";
    let checked = args.next().ok_or("checked state required")? == "checked";
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 180.0, scale)?)?;
    let body = document.body();
    body.set_background_color(Color::WHITE)?;
    let fieldset = Element::create(&document, "fieldset")?;
    fieldset.set_id("group")?;
    fieldset.set_display(Display::Contents)?;
    let first = Element::create(&document, "legend")?;
    let second = Element::create(&document, "legend")?;
    first.set_display(Display::Contents)?;
    second.set_display(Display::Contents)?;
    body.append_child(&fieldset)?;
    fieldset.append_child(&first)?;
    fieldset.append_child(&second)?;
    if disabled {
        fieldset.set_attribute("disabled", "")?;
    }
    let mut controls = Vec::new();
    for (row, kind) in ["checkbox", "radio"].into_iter().enumerate() {
        for (column, parent) in [&first, &second, &fieldset, &body].into_iter().enumerate() {
            let element = Element::create(&document, "input")?;
            let id = format!("{kind}-{column}");
            element.set_id(&id)?;
            element.set_attribute("type", kind)?;
            element.set_attribute("name", &id)?;
            element.set_display(Display::Block)?;
            element.set_position(Position::Absolute)?;
            element.set_left(Length::px(20.0 + column as f32 * 50.0 + phase))?;
            element.set_top(Length::px(20.0 + row as f32 * 50.0 + phase))?;
            element.set_width(LengthValue::px(13.0))?;
            element.set_height(LengthValue::px(13.0))?;
            element.set_box_sizing(BoxSizing::BorderBox)?;
            element.set_margin_top(LengthValue::px(0.0))?;
            element.set_margin_right(LengthValue::px(0.0))?;
            element.set_margin_bottom(LengthValue::px(0.0))?;
            element.set_margin_left(LengthValue::px(0.0))?;
            if checked {
                element.set_attribute("checked", "")?;
            }
            if column == 3 {
                element.set_attribute("disabled", "")?;
            }
            parent.append_child(&element)?;
            controls.push(element);
        }
    }
    let toggle = Element::create(&document, "div")?;
    toggle.set_id("toggle")?;
    toggle.set_position(Position::Absolute)?;
    toggle.set_left(Length::px(20.0))?;
    toggle.set_top(Length::px(120.0))?;
    toggle.set_width(LengthValue::px(20.0))?;
    toggle.set_height(LengthValue::px(20.0))?;
    toggle.set_background_color(Color::BLACK)?;
    body.append_child(&toggle)?;
    let count = Rc::new(Cell::new(0u32));
    let callback_count = count.clone();
    let weak_group = fieldset.downgrade();
    toggle.on("click", move |_| {
        let group = weak_group.upgrade().expect("live group");
        if group.is_own_disabled().expect("own disabled query") {
            group
                .remove_attribute("disabled")
                .expect("enable native group");
        } else {
            group
                .set_attribute("disabled", "")
                .expect("disable native group");
        }
        callback_count.set(callback_count.get() + 1);
    })?;
    std::fs::create_dir_all(&output)?;
    document.update_all()?;
    std::fs::write(output.join("before.png"), document.render_to_png_buffer()?)?;
    toggle.click()?;
    document.update_all()?;
    std::fs::write(output.join("after.png"), document.render_to_png_buffer()?)?;
    let mut states = Vec::new();
    for element in &controls {
        let rect = element.bounding_rect()?.ok_or("control missing bounds")?;
        states.push(json!({
            "id":element.get_attribute("id")?.ok_or("missing id")?,
            "checked":element.is_checked()?,
            "checkedAttribute":element.get_attribute("checked")?.is_some(),
            "own":element.is_own_disabled()?,
            "effective":element.is_effectively_disabled()?,
            "bounds":{"x":rect.x,"y":rect.y,"width":rect.width,"height":rect.height}
        }));
    }
    std::fs::write(
        output.join("observations.json"),
        serde_json::to_vec_pretty(
            &json!({"callback_count":count.get(),"group_own":fieldset.is_own_disabled()?,"controls":states}),
        )?,
    )?;
    assert_eq!(count.get(), 1);
    let weak = controls.iter().map(Element::downgrade).collect::<Vec<_>>();
    drop(controls);
    drop(toggle);
    drop(first);
    drop(second);
    drop(fieldset);
    drop(body);
    drop(document);
    assert!(
        weak.iter().all(|e| e.upgrade().is_none()),
        "consumer retained document"
    );
    Ok(())
}
