//! Native app guards for indented inline and block static positions.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("device scale required")?.parse()?;
    let case = args.next().ok_or("case required")?;
    let raster_name = args.next().unwrap_or_else(|| "default".to_owned());
    let raster_configuration = match raster_name.as_str() {
        "default" => RasterConfiguration::default(),
        "chromium-linux-lcd" => RasterConfiguration::chromium_linux_lcd(),
        "chromium-linux-fontations-lcd" => RasterConfiguration::chromium_linux_fontations_lcd(),
        _ => return Err("invalid raster configuration".into()),
    };
    if args.next().is_some()
        || !matches!(
            case.as_str(),
            "inline-reset"
                | "block-reset"
                | "inline-inherit"
                | "block-inherit"
                | "inline-relative-reset"
                | "block-relative-reset"
        )
    {
        return Err("invalid native static-position arguments".into());
    }
    let is_block = case.starts_with("block");
    let reset = case.ends_with("reset");
    let relative = case.contains("relative");
    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(320.0, 160.0, scale)?,
        FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration,
        },
    )?;
    assert_eq!(document.raster_configuration()?, raster_configuration);
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    root.set_padding_top(LengthValue::px(20.0))?;
    root.set_padding_left(LengthValue::px(20.0))?;
    let container = Element::create(&document, "div")?;
    container.set_position(Position::Relative)?;
    container.set_width(LengthValue::px(240.0))?;
    container.set_background_color(Color::GREEN)?;
    container.set_color(Color::GREEN)?;
    container.set_font_family(FontFamilyList::single("Ahem"))?;
    container.set_font_size(LengthValue::px(16.0))?;
    container.set_line_height(LineHeight::Number(1.0))?;
    container.set_text_indent(LengthValue::px(20.0))?;
    container.set_border_top_width(3)?;
    container.set_border_right_width(3)?;
    container.set_border_bottom_width(3)?;
    container.set_border_left_width(3)?;
    container.set_border_top_style(BorderStyle::Solid)?;
    container.set_border_right_style(BorderStyle::Solid)?;
    container.set_border_bottom_style(BorderStyle::Solid)?;
    container.set_border_left_style(BorderStyle::Solid)?;
    container.set_border_top_color(StyleColor::Resolved(Color::BLACK))?;
    container.set_border_right_color(StyleColor::Resolved(Color::BLACK))?;
    container.set_border_bottom_color(StyleColor::Resolved(Color::BLACK))?;
    container.set_border_left_color(StyleColor::Resolved(Color::BLACK))?;
    container.set_text("XXX")?;
    root.append_child(&container)?;
    let span = Element::create(&document, "span")?;
    span.set_text("XX")?;
    if reset {
        span.set_text_indent(LengthValue::px(0.0))?;
    }
    if relative {
        span.set_position(Position::Relative)?;
    }
    container.append_child(&span)?;
    let absolute = Element::create(&document, "div")?;
    absolute.set_display(if is_block {
        Display::Block
    } else {
        Display::Inline
    })?;
    absolute.set_position(Position::Absolute)?;
    if reset {
        absolute.set_text_indent(LengthValue::px(0.0))?;
    }
    absolute.set_text("XXXXX")?;
    span.append_child(&absolute)?;
    let line_break = if is_block {
        let node = Element::create(&document, "br")?;
        if reset {
            node.set_text_indent(LengthValue::px(0.0))?;
        }
        span.append_child(&node)?;
        Some(node)
    } else {
        None
    };
    let red = Element::create(&document, "span")?;
    red.set_color(Color::RED)?;
    if reset {
        red.set_text_indent(LengthValue::px(0.0))?;
    }
    red.set_text("XXXXX")?;
    span.append_child(&red)?;
    let before_style = absolute.computed_style()?;
    assert_eq!(
        before_style.text_indent.value(),
        if reset { 0.0 } else { 20.0 }
    );
    let before_bounds = absolute
        .bounding_rect()?
        .ok_or("native absolute bounds required")?;
    assert_eq!(
        before_bounds.width,
        if reset { 80.015625 } else { 100.015625 }
    );
    let before = document.render_to_png_buffer()?;
    let calls = Rc::new(Cell::new(0));
    let callback_calls = calls.clone();
    let weak_container = container.downgrade();
    absolute.on("click", move |_| {
        weak_container
            .upgrade()
            .expect("live native container")
            .set_text_indent(LengthValue::px(28.0))
            .expect("native indentation mutation");
        callback_calls.set(callback_calls.get() + 1);
    })?;
    absolute.click()?;
    assert_eq!(calls.get(), 1);
    let after_style = absolute.computed_style()?;
    assert_eq!(
        after_style.text_indent.value(),
        if reset { 0.0 } else { 28.0 }
    );
    let after_bounds = absolute
        .bounding_rect()?
        .ok_or("native absolute bounds required")?;
    assert_eq!(
        after_bounds.width,
        if reset { 80.015625 } else { 108.015625 }
    );
    let after = document.render_to_png_buffer()?;
    // Exercise the public typed line-height setter as well as the generic
    // property transport. Percentages compute on the parent before inheritance.
    let percent_parent = Element::create(&document, "div")?;
    percent_parent.set_display(Display::None)?;
    percent_parent.set_font_size(LengthValue::px(24.0))?;
    percent_parent.set_line_height(LineHeight::Percentage(150.0))?;
    let percent_child = Element::create(&document, "span")?;
    percent_child.set_font_size(LengthValue::px(40.0))?;
    percent_parent.append_child(&percent_child)?;
    root.append_child(&percent_parent)?;
    let percent_style = percent_child.computed_style()?;
    assert_eq!(percent_style.line_height, LineHeight::Length(36.0));
    let weak_absolute = absolute.downgrade();
    drop(percent_child);
    drop(percent_parent);
    drop(line_break);
    drop(red);
    drop(absolute);
    drop(span);
    drop(container);
    drop(root);
    drop(document);
    assert!(weak_absolute.upgrade().is_none());
    assert_eq!(
        before_style.text_indent.value(),
        if reset { 0.0 } else { 20.0 }
    );
    assert_eq!(percent_style.line_height, LineHeight::Length(36.0));
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("before.png"), before)?;
    std::fs::write(output.join("after.png"), after)?;
    let rectangle = |rect: Rect| {
        format!(
            "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
            rect.x, rect.y, rect.width, rect.height
        )
    };
    std::fs::write(
        output.join("geometry.json"),
        format!(
            "{{\"before\":{},\"after\":{}}}\n",
            rectangle(before_bounds),
            rectangle(after_bounds)
        ),
    )?;
    println!(
        "native static positions: case={case} scale={scale} raster={raster_name} callback=1 owned-snapshots=2 passed"
    );
    Ok(())
}
