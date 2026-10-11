//! Public native Rust app for atomic constraints and leading whitespace.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn bounds_json(element: &Element) -> Result<String, Box<dyn std::error::Error>> {
    let r = element.bounding_rect()?.ok_or("native box required")?;
    Ok(format!(
        "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        r.x, r.y, r.width, r.height
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("device scale required")?.parse()?;
    let mode = args.next().ok_or("constraints or whitespace required")?;
    let context = args.next().ok_or("layout context required")?;
    let sizing = args.next().unwrap_or_else(|| "border-box".into());
    let minimum: f32 = args.next().unwrap_or_else(|| "200".into()).parse()?;
    let profile = args.next().unwrap_or_else(|| "owned".into());
    if args.next().is_some()
        || !matches!(mode.as_str(), "constraints" | "whitespace")
        || !matches!(sizing.as_str(), "border-box" | "content-box")
        || ![0.0, 60.0, 200.0].contains(&minimum)
        || !matches!(profile.as_str(), "owned" | "installed")
    {
        return Err("invalid native case arguments".into());
    }
    let configuration = if profile == "installed" {
        RasterConfiguration::deterministic_aliased(false)
    } else {
        RasterConfiguration::default()
    };
    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(320.0, 240.0, scale)?,
        FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: configuration,
        },
    )?;
    assert_eq!(document.raster_configuration()?, configuration);
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_font_family(FontFamilyList::single("Ahem"))?;
    root.set_font_size(LengthValue::px(16.0))?;
    root.set_line_height(LineHeight::Number(1.0))?;
    let parent = Element::create(&document, "div")?;
    parent.set_position(Position::Relative)?;
    parent.set_width(LengthValue::px(240.0))?;
    parent.set_height(LengthValue::px(200.0))?;
    root.append_child(&parent)?;
    let outer = Element::create(&document, "div")?;
    match context.as_str() {
        "absolute" => outer.set_position(Position::Absolute)?,
        "inline-block" => outer.set_display(Display::InlineBlock)?,
        "float" => outer.set_float(Float::Left)?,
        "min-content" => outer.set_width(LengthValue::Computed(Length::min_content()))?,
        "max-content" => outer.set_width(LengthValue::Computed(Length::max_content()))?,
        _ => return Err("invalid native layout context".into()),
    }
    outer.set_background_color(Color::RED)?;
    parent.append_child(&outer)?;
    let inner = Element::create(&document, "div")?;
    let content = Element::create(&document, "div")?;
    if mode == "constraints" {
        inner.set_display(Display::InlineBlock)?;
        inner.set_width(LengthValue::Computed(Length::min_content()))?;
        inner.set_min_width(LengthValue::px(minimum))?;
        inner.set_box_sizing(if sizing == "border-box" {
            BoxSizing::BorderBox
        } else {
            BoxSizing::ContentBox
        })?;
        inner.set_padding(Edges::all(LengthValue::px(10.0)))?;
        inner.set_background_color(Color::GREEN)?;
        content.set_width(LengthValue::px(20.0))?;
        content.set_height(LengthValue::px(20.0))?;
        content.set_background_color(Color::BLUE)?;
        inner.append_child(&content)?;
        outer.append_child(&inner)?;
    } else {
        outer.set_text("\n XXXXX\n")?;
    }
    let before_snapshot = outer
        .bounding_rect()?
        .ok_or("owned before bounds required")?;
    let constrained_width = |value: f32| {
        if sizing == "border-box" {
            value.max(40.0)
        } else {
            (value + 20.0).max(40.0)
        }
    };
    let text_width = if profile == "installed" {
        80.0
    } else {
        80.015625
    };
    let expected_before = if mode == "constraints" {
        constrained_width(minimum)
    } else {
        text_width
    };
    assert_eq!(before_snapshot.width, expected_before);
    let before = bounds_json(&outer)?;
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("before.png"), document.render_to_png_buffer()?)?;
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let weak_outer = outer.downgrade();
    let weak_inner = inner.downgrade();
    let target = weak_inner.clone();
    let text_target = weak_outer.clone();
    let constraints = mode == "constraints";
    let next_minimum = if minimum == 0.0 { 200.0 } else { 0.0 };
    outer.on("click", move |_| {
        if constraints {
            let element = target.upgrade().expect("native inner remains alive");
            element
                .set_min_width(LengthValue::px(next_minimum))
                .expect("minimum mutation");
            element
                .set_max_width(LengthValue::px(100.0))
                .expect("maximum mutation");
        } else {
            let element = text_target.upgrade().expect("native target remains alive");
            element.set_text(" XXXXX ").expect("text mutation");
            element
                .set_text_indent(LengthValue::px(20.0))
                .expect("indent mutation");
        }
        observed.set(observed.get() + 1);
    })?;
    outer.click()?;
    assert_eq!(calls.get(), 1);
    let after = bounds_json(&outer)?;
    let after_snapshot = outer
        .bounding_rect()?
        .ok_or("owned after bounds required")?;
    let expected_after = if constraints {
        constrained_width(next_minimum)
    } else {
        text_width + 20.0
    };
    assert_eq!(after_snapshot.width, expected_after);
    assert_eq!(before_snapshot.width, expected_before);
    std::fs::write(output.join("after.png"), document.render_to_png_buffer()?)?;
    std::fs::write(
        output.join("geometry.json"),
        format!("{{\"before\":{before},\"after\":{after}}}\n"),
    )?;
    // Retain owned geometry after native document teardown; callbacks hold
    // only weak elements and must not keep the document alive.
    drop(content);
    drop(inner);
    drop(outer);
    drop(parent);
    drop(root);
    drop(document);
    assert!(weak_outer.upgrade().is_none());
    assert!(weak_inner.upgrade().is_none());
    assert_eq!(before_snapshot.width, expected_before);
    assert_eq!(after_snapshot.width, expected_after);
    println!("native intrinsic constraints: {mode} {context} {sizing} min={minimum} profile={profile} scale={scale} callback=1 owned-bounds=2 teardown=1 passed");
    Ok(())
}
