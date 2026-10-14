//! Public native Rust diagnostic. No JavaScript or private Engine access.
//! CLI: OUTPUT WIDTH HEIGHT SCALE POLICY MARKERS [TABLE_WIDTH]
//! POLICY: auto|fitted. MARKERS: no|yes. TABLE_WIDTH is CSS px; omitted means auto.
use openui::prelude::*;
use std::{cell::Cell, error::Error, path::PathBuf, rc::Rc};

type ProbeResult<T> = Result<T, Box<dyn Error>>;

fn node(document: &Document, parent: &Element, tag: &str, id: &str) -> ProbeResult<Element> {
    let element = Element::create(document, tag)?;
    element.set_attribute("id", id)?;
    parent.append_child(&element)?;
    Ok(element)
}

fn border(element: &Element) -> ProbeResult<()> {
    element.set_border_top_width(1)?;
    element.set_border_right_width(1)?;
    element.set_border_bottom_width(1)?;
    element.set_border_left_width(1)?;
    element.set_border_top_style(BorderStyle::Solid)?;
    element.set_border_right_style(BorderStyle::Solid)?;
    element.set_border_bottom_style(BorderStyle::Solid)?;
    element.set_border_left_style(BorderStyle::Solid)?;
    element.set_border_top_color(StyleColor::Resolved(Color::BLUE))?;
    element.set_border_right_color(StyleColor::Resolved(Color::BLUE))?;
    element.set_border_bottom_color(StyleColor::Resolved(Color::BLUE))?;
    element.set_border_left_color(StyleColor::Resolved(Color::BLUE))?;
    Ok(())
}

fn marker(document: &Document, parent: &Element, id: &str) -> ProbeResult<Element> {
    let element = node(document, parent, "span", id)?;
    element.set_display(Display::InlineBlock)?;
    element.set_width(LengthValue::px(0.0))?;
    element.set_height(LengthValue::px(0.0))?;
    element.set_margin_top(LengthValue::px(0.0))?;
    element.set_margin_right(LengthValue::px(0.0))?;
    element.set_margin_bottom(LengthValue::px(0.0))?;
    element.set_margin_left(LengthValue::px(0.0))?;
    element.set_padding_top(LengthValue::px(0.0))?;
    element.set_padding_right(LengthValue::px(0.0))?;
    element.set_padding_bottom(LengthValue::px(0.0))?;
    element.set_padding_left(LengthValue::px(0.0))?;
    element.set_border_top_width(0)?;
    element.set_border_right_width(0)?;
    element.set_border_bottom_width(0)?;
    element.set_border_left_width(0)?;
    element.set_vertical_align(VerticalAlign::Baseline)?;
    Ok(element)
}

fn main() -> ProbeResult<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("OUTPUT")?);
    let width: f64 = args.next().ok_or("WIDTH")?.parse()?;
    let height: f64 = args.next().ok_or("HEIGHT")?.parse()?;
    let scale: f64 = args.next().ok_or("SCALE")?.parse()?;
    let policy = args.next().ok_or("POLICY")?;
    let markers_arg = args.next().ok_or("MARKERS")?;
    let markers = match markers_arg.as_str() {
        "no" => false,
        "yes" => true,
        _ => return Err("MARKERS must be no or yes".into()),
    };
    let table_width = args.next().map(|v| v.parse::<f32>()).transpose()?;
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    if table_width.is_some_and(|v| !v.is_finite() || v <= 0.0) {
        return Err("TABLE_WIDTH must be positive and finite".into());
    }
    let raster_configuration = match policy.as_str() {
        "auto" => RasterConfiguration::deterministic_aliased(false),
        "fitted" => RasterConfiguration::chromium_linux_monochrome(),
        _ => return Err("POLICY must be auto or fitted".into()),
    };
    std::fs::create_dir_all(&out)?;
    let callbacks = Rc::new(Cell::new(0_u32));
    let weak = {
        let document = Document::with_font_collection_and_options(
            ViewportMetrics::from_logical_size(width, height, scale)?,
            FontCollection::deterministic_test(),
            EngineOptions {
                raster_configuration,
            },
        )?;
        let body = document.body();
        body.set_background_color(Color::WHITE)?;
        body.set_font_family(FontFamilyList {
            families: [
                "Ahem",
                "Droid Sans Fallback",
                "Noto Sans Devanagari",
                "Noto Color Emoji",
                "DejaVu Sans",
            ]
            .into_iter()
            .map(|v| FontFamily::Named(v.to_owned()))
            .collect(),
        })?;
        body.set_font_size(LengthValue::px(16.0))?;
        body.set_font_kerning(FontKerning::None)?;
        body.set_font_variant_ligatures(FontVariantLigatures::none())?;
        body.set_margin_top(LengthValue::px(0.0))?;
        body.set_margin_right(LengthValue::px(0.0))?;
        body.set_margin_bottom(LengthValue::px(0.0))?;
        body.set_margin_left(LengthValue::px(0.0))?;
        body.set_padding_top(LengthValue::px(0.0))?;
        body.set_padding_right(LengthValue::px(0.0))?;
        body.set_padding_bottom(LengthValue::px(0.0))?;
        body.set_padding_left(LengthValue::px(0.0))?;
        let fixture = node(&document, &body, "div", "fixture")?;
        fixture.set_color(Color::RED)?;
        let table = node(&document, &fixture, "div", "table")?;
        table.set_display(Display::Table)?;
        table.set_float(Float::Right)?;
        table.set_border_collapse(BorderCollapse::Collapse)?;
        border(&table)?;
        if let Some(value) = table_width {
            table.set_width(LengthValue::px(value))?;
        }
        let caption = node(&document, &table, "div", "caption")?;
        caption.set_display(Display::TableCaption)?;
        caption.set_color(Color::from_rgba8(255, 192, 203, 255))?;
        caption.append_text_node("1")?;
        let second = node(&document, &caption, "span", "caption-second")?;
        second.set_text("1")?;
        let cell = node(&document, &table, "div", "cell")?;
        cell.set_display(Display::TableCell)?;
        border(&cell)?;
        cell.set_color(Color::from_rgba8(0, 255, 255, 255))?;
        cell.set_text("2222222")?;
        let plain = node(&document, &body, "div", "plain")?;
        plain.set_position(Position::Absolute)?;
        plain.set_top(Length::px(96.0))?;
        plain.set_color(Color::from_rgba8(255, 192, 203, 255))?;
        plain.append_text_node("1")?;
        let plain_second = node(&document, &plain, "span", "plain-second")?;
        plain_second.set_text("1")?;
        let mut nodes = vec![
            ("fixture", fixture.clone()),
            ("table", table.clone()),
            ("caption", caption.clone()),
            ("caption-second", second.clone()),
            ("cell", cell.clone()),
            ("plain", plain.clone()),
            ("plain-second", plain_second.clone()),
        ];
        if markers {
            for (parent, name) in [
                (&caption, "caption-baseline"),
                (&cell, "cell-baseline"),
                (&plain, "plain-baseline"),
            ] {
                nodes.push((name, marker(&document, parent, name)?));
            }
        }
        let caption_box = caption
            .bounding_rect()?
            .ok_or("caption has no layout box")?;
        plain.set_left(Length::px(caption_box.x))?;
        plain.set_width(LengthValue::px(caption_box.width))?;
        let snapshots = nodes.iter().map(|(name, element)| {
            let bounds = element.bounding_rect()?.map(|r| serde_json::json!({"x":r.x,"y":r.y,"width":r.width,"height":r.height}));
            let rects = element.client_rects()?.into_iter().map(|r| serde_json::json!({"x":r.x,"y":r.y,"width":r.width,"height":r.height})).collect::<Vec<_>>();
            let baseline = if name.ends_with("-baseline") {
                element.client_rects()?.first().map(|r| f64::from(r.y))
            } else { None };
            Ok(serde_json::json!({"name":name,"bounds":bounds,"client_rects":rects,"baseline_css_y":baseline,"baseline_physical_y":baseline.map(|y|y*scale)}))
        }).collect::<ProbeResult<Vec<_>>>()?;
        let called = callbacks.clone();
        body.on("click", move |_| {
            called.set(called.get() + 1);
        })?;
        let png = document.render_to_png_buffer()?;
        let unchanged_repeat = document.render_to_png_buffer()?;
        if png != unchanged_repeat {
            return Err("unchanged native frame differs".into());
        }
        body.click()?;
        if callbacks.get() != 1 {
            return Err("native click callback did not run exactly once".into());
        }
        if png != document.render_to_png_buffer()? {
            return Err("count-only native callback changed pixels".into());
        }
        let bitmap = document.render_to_bitmap()?;
        std::fs::write(out.join("frame.png"), png)?;
        std::fs::write(
            out.join("bounds.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version":1,"kind":"native-table-origin-diagnostic","qualified":false,
                "policy":policy,"markers":markers_arg,"table_width_css_px":table_width,
                "raster_configuration_debug":format!("{:?}",raster_configuration),
                "logical_size_css_px":[width,height],"device_scale":scale,
                "physical_size_px":[bitmap.width(),bitmap.height()],
                "plain_left_matched_to_caption_css_px":caption_box.x,
                "plain_width_matched_to_caption_css_px":caption_box.width,
                "marker_neutrality":"unverified; compare no/yes outputs externally",
                "nodes":snapshots,
            }))?,
        )?;
        table.downgrade()
    };
    if weak.upgrade().is_some() {
        return Err("native document retained after teardown".into());
    }
    std::fs::write(
        out.join("lifecycle.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "native_click_callbacks":callbacks.get(),"unchanged_frame_byte_repeat":true,
            "count_only_callback_preserved_pixels":true,"weak_teardown_passed":true,
            "javascript_executed_by_openui":false,"marker_neutrality_qualified":false,
        }))?,
    )?;
    Ok(())
}
