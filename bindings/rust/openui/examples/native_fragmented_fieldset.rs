//! Native diagnostic application; not release qualification evidence.
//! Native app for fractional-scale fieldset decoration and opacity.
//! Inputs vary geometry; no expected pixels or WPT-ID branches.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn reset(element: &Element) -> Result<(), Box<dyn std::error::Error>> {
    element.set_margin(Edges::all(LengthValue::px(0.0)))?;
    element.set_padding(Edges::all(LengthValue::px(0.0)))?;
    element.set_color(Color::BLACK)?;
    element.set_background_color(Color::WHITE)?;
    element.set_font_size(LengthValue::px(16.0))?;
    element.set_line_height(LineHeight::Number(1.0))?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let phase: f32 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let alpha: f32 = args.next().unwrap_or_else(|| "0.5".into()).parse()?;
    let mode = args.next().unwrap_or_else(|| "all".into());
    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(1280.0, 720.0, scale)?,
        FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(true),
        },
    )?;
    let root = document.body();
    reset(&root)?;
    root.set_font_family(FontFamilyList {
        families: [
            "Ahem",
            "Droid Sans Fallback",
            "Noto Sans Devanagari",
            "Noto Color Emoji",
            "DejaVu Sans",
        ]
        .into_iter()
        .map(|name| FontFamily::Named(name.into()))
        .collect(),
    })?;
    root.set_width(LengthValue::px(600.0))?;
    root.set_height(LengthValue::px(600.0))?;
    let mut nodes = vec![("body".to_owned(), root.clone())];
    let calls = Rc::new(Cell::new(0));
    for (index, (columns, height, sized)) in [
        (3, 80.0, false),
        (2, 120.0, false),
        (3, 80.0, true),
        (2, 120.0, true),
    ]
    .into_iter()
    .enumerate()
    {
        if mode == "single" && index != 3 {
            continue;
        }
        let group = Element::create(&document, "div")?;
        reset(&group)?;
        group.set_column_count(Some(columns))?;
        group.set_column_fill(ColumnFill::Auto)?;
        group.set_height(LengthValue::px(height))?;
        group.set_background_color(Color::from_rgba8(128, 128, 128, 255))?;
        group.set_margin_bottom(LengthValue::px(10.0))?;
        group.set_position(Position::Relative)?;
        group.set_left(Length::px(phase))?;
        group.set_top(Length::px(phase))?;
        root.append_child(&group)?;
        let fieldset = Element::create(&document, "fieldset")?;
        let legend = Element::create(&document, "legend")?;
        let fill = Element::create(&document, "div")?;
        for element in [&fieldset, &legend, &fill] {
            reset(element)?;
        }
        for element in [&fieldset, &legend] {
            element.set_background_clip(BackgroundClip::ContentBox)?;
            element.set_padding_top(LengthValue::px(10.0))?;
            element.set_padding_right(LengthValue::px(7.0))?;
            element.set_padding_bottom(LengthValue::px(20.0))?;
            element.set_padding_left(LengthValue::px(3.0))?;
        }
        fieldset.set_background_color(Color::from_rgba8(211, 211, 211, 255))?;
        fieldset.set_border_top(Border {
            width: 6.0,
            style: BorderStyle::Solid,
            color: Color::BLACK,
        })?;
        fieldset.set_border_right(Border {
            width: 3.0,
            style: BorderStyle::Solid,
            color: Color::BLACK,
        })?;
        fieldset.set_border_bottom(Border {
            width: 10.0,
            style: BorderStyle::Solid,
            color: Color::BLACK,
        })?;
        fieldset.set_border_left(Border {
            width: 7.0,
            style: BorderStyle::Solid,
            color: Color::BLACK,
        })?;
        fieldset.set_margin_top(LengthValue::px(3.0))?;
        fieldset.set_margin_right(LengthValue::px(10.0))?;
        fieldset.set_margin_bottom(LengthValue::px(7.0))?;
        fieldset.set_margin_left(LengthValue::px(5.0))?;
        if sized {
            fieldset.set_height(LengthValue::px(108.0))?;
        }
        legend.set_background_color(Color::BLACK)?;
        legend.set_width(LengthValue::px(100.0))?;
        legend.set_height(LengthValue::px(19.0))?;
        fill.set_display(Display::Block)?;
        fill.set_background_color(Color::BLUE)?;
        fill.set_height(LengthValue::px(108.0))?;
        fill.set_opacity(0.5)?;
        group.append_child(&fieldset)?;
        fieldset.append_child(&legend)?;
        fieldset.append_child(&fill)?;
        let callback_calls = calls.clone();
        fill.on("click", move |event| {
            event
                .target()
                .expect("live target")
                .set_opacity(alpha)
                .expect("native opacity mutation");
            callback_calls.set(callback_calls.get() + 1);
        })?;
        fill.click()?;
        for (name, element) in [
            ("group", group),
            ("fieldset", fieldset),
            ("legend", legend),
            ("fill", fill),
        ] {
            nodes.push((format!("{name}-{index}"), element));
        }
    }
    std::fs::create_dir_all(&output)?;
    let geometry = nodes.iter().map(|(name, e)| {
        let r = e.bounding_rect()?.ok_or("missing owned bounds")?;
        Ok(serde_json::json!({"name": name, "bounds": {"x":r.x,"y":r.y,"width":r.width,"height":r.height}}))
    }).collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let rects = nodes
        .iter()
        .map(|(name, e)| {
            let rectangles = e
                .client_rects()?
                .into_iter()
                .map(|r| serde_json::json!({"x":r.x,"y":r.y,"width":r.width,"height":r.height}))
                .collect::<Vec<_>>();
            Ok(serde_json::json!({"name": name, "rects": rectangles}))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let png = document.render_to_png_buffer()?;
    assert_eq!(png, document.render_to_png_buffer()?);
    std::fs::write(output.join("openui.png"), png)?;
    std::fs::write(
        output.join("geometry.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"callbacks": calls.get(), "nodes": geometry, "rects": rects}),
        )?,
    )?;
    let weak = nodes.last().unwrap().1.downgrade();
    drop(nodes);
    drop(root);
    drop(document);
    assert!(weak.upgrade().is_none());
    Ok(())
}
