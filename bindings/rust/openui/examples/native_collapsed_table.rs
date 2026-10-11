//! Public native Rust consumer of reviewed collapsed-table DOM/style inputs.
//! No JavaScript, HTML/CSS parser, private Engine access or pixel replacement.
//! CLI: OUTPUT WIDTH HEIGHT SCALE FIXTURES_JSON CASE auto|fitted
//! Fixtures are lowered offline; every runtime mutation uses a typed native API.
use openui::prelude::*;
use serde_json::{json, Value};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    error::Error,
    path::PathBuf,
    rc::Rc,
};
type ProbeResult<T> = Result<T, Box<dyn Error>>;

fn number(value: &Value) -> ProbeResult<f32> {
    let number = value.as_f64().ok_or("numeric style value required")? as f32;
    if !number.is_finite() {
        return Err("nonfinite style value".into());
    }
    Ok(number)
}
fn string(value: &Value) -> ProbeResult<&str> {
    Ok(value.as_str().ok_or("enum string required")?)
}
fn color(value: &Value) -> ProbeResult<Color> {
    let v = value.as_array().ok_or("RGBA array required")?;
    if v.len() != 4 {
        return Err("four RGBA components required".into());
    }
    Ok(Color::from_rgba_f32(
        number(&v[0])? / 255.0,
        number(&v[1])? / 255.0,
        number(&v[2])? / 255.0,
        number(&v[3])?,
    ))
}
fn style_color(value: &Value) -> ProbeResult<StyleColor> {
    Ok(if value.as_str() == Some("currentColor") {
        StyleColor::CurrentColor
    } else {
        StyleColor::Resolved(color(value)?)
    })
}
fn border_style(value: &Value) -> ProbeResult<BorderStyle> {
    Ok(match string(value)? {
        "none" => BorderStyle::None,
        "hidden" => BorderStyle::Hidden,
        "solid" => BorderStyle::Solid,
        "double" => BorderStyle::Double,
        "dotted" => BorderStyle::Dotted,
        "dashed" => BorderStyle::Dashed,
        "ridge" => BorderStyle::Ridge,
        "groove" => BorderStyle::Groove,
        "inset" => BorderStyle::Inset,
        "outset" => BorderStyle::Outset,
        _ => return Err("unsupported border style".into()),
    })
}
fn apply(element: &Element, properties: &Value) -> ProbeResult<()> {
    for (property, value) in properties.as_object().ok_or("style object required")? {
        match property.as_str() {
            "display" => element.set_display(match string(value)? {
                "block" => Display::Block,
                "table" => Display::Table,
                "table-caption" => Display::TableCaption,
                "table-column-group" => Display::TableColumnGroup,
                "table-column" => Display::TableColumn,
                "table-header-group" => Display::TableHeaderGroup,
                "table-row-group" => Display::TableRowGroup,
                "table-footer-group" => Display::TableFooterGroup,
                "table-row" => Display::TableRow,
                "table-cell" => Display::TableCell,
                _ => return Err("unsupported display".into()),
            })?,
            "width" => element.set_width(LengthValue::px(number(value)?))?,
            "height" => element.set_height(LengthValue::px(number(value)?))?,
            "box-sizing" => element.set_box_sizing(match string(value)? {
                "border-box" => BoxSizing::BorderBox,
                _ => return Err("unsupported box sizing".into()),
            })?,
            "text-indent" => element.set_text_indent(LengthValue::px(number(value)?))?,
            "margin-top" => element.set_margin_top(LengthValue::px(number(value)?))?,
            "margin-right" => element.set_margin_right(LengthValue::px(number(value)?))?,
            "margin-bottom" => element.set_margin_bottom(LengthValue::px(number(value)?))?,
            "margin-left" => element.set_margin_left(LengthValue::px(number(value)?))?,
            "padding-top" => element.set_padding_top(LengthValue::px(number(value)?))?,
            "padding-right" => element.set_padding_right(LengthValue::px(number(value)?))?,
            "padding-bottom" => element.set_padding_bottom(LengthValue::px(number(value)?))?,
            "padding-left" => element.set_padding_left(LengthValue::px(number(value)?))?,
            "font-family" => element.set_font_family(match string(value)? {
                "monospace" => FontFamilyList::generic(GenericFontFamily::Monospace),
                "Times New Roman" => FontFamilyList::single("Times New Roman"),
                _ => return Err("unsupported font family".into()),
            })?,
            "font-size" => element.set_font_size(LengthValue::px(number(value)?))?,
            "font-weight" => element.set_font_weight(FontWeight(number(value)?))?,
            "line-height" => {
                if value.as_str() == Some("normal") {
                    element.set_line_height(LineHeight::Normal)?
                } else {
                    element.set_line_height_length(LengthValue::px(number(value)?))?
                }
            }
            "vertical-align" => element.set_vertical_align(match string(value)? {
                "middle" => VerticalAlign::Middle,
                _ => return Err("unsupported vertical alignment".into()),
            })?,
            "text-align" => element.set_text_align(match string(value)? {
                "center" => TextAlign::Center,
                _ => return Err("unsupported text alignment".into()),
            })?,
            "color" => element.set_color(color(value)?)?,
            "background-color" => element.set_background_color(color(value)?)?,
            "border-collapse" => element.set_border_collapse(match string(value)? {
                "collapse" => BorderCollapse::Collapse,
                _ => return Err("unsupported border collapse".into()),
            })?,
            "border-spacing" => {
                let v = value.as_array().ok_or("spacing pair required")?;
                if v.len() != 2 {
                    return Err("spacing pair required".into());
                }
                element
                    .set_border_spacing((Length::px(number(&v[0])?), Length::px(number(&v[1])?)))?;
            }
            "border-top-width" => element.set_border_top_width(number(value)? as i32)?,
            "border-right-width" => element.set_border_right_width(number(value)? as i32)?,
            "border-bottom-width" => element.set_border_bottom_width(number(value)? as i32)?,
            "border-left-width" => element.set_border_left_width(number(value)? as i32)?,
            "border-top-style" => element.set_border_top_style(border_style(value)?)?,
            "border-right-style" => element.set_border_right_style(border_style(value)?)?,
            "border-bottom-style" => element.set_border_bottom_style(border_style(value)?)?,
            "border-left-style" => element.set_border_left_style(border_style(value)?)?,
            "border-top-color" => element.set_border_top_color(style_color(value)?)?,
            "border-right-color" => element.set_border_right_color(style_color(value)?)?,
            "border-bottom-color" => element.set_border_bottom_color(style_color(value)?)?,
            "border-left-color" => element.set_border_left_color(style_color(value)?)?,
            "position" => element.set_position(match string(value)? {
                "relative" => Position::Relative,
                _ => return Err("unsupported position".into()),
            })?,
            "left" => element.set_left(Length::px(number(value)?))?,
            "top" => element.set_top(Length::px(number(value)?))?,
            "opacity" => element.set_opacity(number(value)?)?,
            "transform" => {
                let v = value.as_array().ok_or("translation pair required")?;
                if v.len() != 2 {
                    return Err("translation pair required".into());
                }
                element.set_transform(TransformList(vec![TransformOperation::Translate(
                    LengthValue::px(number(&v[0])?),
                    LengthValue::px(number(&v[1])?),
                )]))?;
            }
            "writing-mode" => element.set_writing_mode(match string(value)? {
                "horizontal-tb" => WritingMode::HorizontalTb,
                "vertical-rl" => WritingMode::VerticalRl,
                "vertical-lr" => WritingMode::VerticalLr,
                "sideways-rl" => WritingMode::SidewaysRl,
                "sideways-lr" => WritingMode::SidewaysLr,
                _ => return Err("unsupported writing mode".into()),
            })?,
            "direction" => element.set_direction(match string(value)? {
                "ltr" => Direction::Ltr,
                "rtl" => Direction::Rtl,
                _ => return Err("unsupported direction".into()),
            })?,
            "visibility" => element.set_visibility(match string(value)? {
                "collapse" => Visibility::Collapse,
                _ => return Err("unsupported visibility".into()),
            })?,
            "column-count" => element.set_column_count(Some(
                value.as_u64().ok_or("column count required")?.try_into()?,
            ))?,
            "column-gap" => element.set_column_gap(LengthValue::px(number(value)?))?,
            "break-inside" => element.set_break_inside(match string(value)? {
                "avoid" => BreakInside::Avoid,
                _ => return Err("unsupported break policy".into()),
            })?,
            _ => return Err(format!("unlowered property {property}").into()),
        }
    }
    Ok(())
}
fn snapshot(
    document: &Document,
    nodes: &[(String, String, Element)],
    points: &Value,
) -> ProbeResult<Value> {
    let rectangle = |r: openui::Rect| json!({"x":r.x,"y":r.y,"width":r.width,"height":r.height});
    let mut records = Vec::new();
    for (name, tag, element) in nodes {
        let metrics=element.scroll_metrics()?.map(|m|json!({"client_width":m.client_width,"client_height":m.client_height,"scroll_width":m.scroll_width,"scroll_height":m.scroll_height}));
        let column_span = if matches!(tag.as_str(), "td" | "th" | "col" | "colgroup") {
            Some(element.column_span()?)
        } else {
            None
        };
        let row_span = if matches!(tag.as_str(), "td" | "th") {
            Some(element.row_span()?)
        } else {
            None
        };
        records.push(json!({"name":name,"tag":tag,"parent":element.parent()?.and_then(|p|p.get_attribute("id").ok().flatten()),"bounds":element.bounding_rect()?.map(rectangle),"client_rects":element.client_rects()?.into_iter().map(rectangle).collect::<Vec<_>>(),"scroll_left":element.scroll_left()?,"scroll_top":element.scroll_top()?,"scroll_metrics":metrics,"column_span":column_span,"row_span":row_span}));
    }
    let mut hits = Vec::new();
    for point in points.as_array().ok_or("hit point array required")? {
        let point = point.as_array().ok_or("hit pair required")?;
        if point.len() != 2 {
            return Err("hit pair required".into());
        }
        let x = point[0].as_f64().ok_or("hit x required")?;
        let y = point[1].as_f64().ok_or("hit y required")?;
        hits.push(json!({"x":x,"y":y,"id":document.hit_test(x as f32,y as f32)?.map(|node|node.get_attribute("id")).transpose()?.flatten()}));
    }
    Ok(json!({"nodes":records,"hits":hits}))
}
fn write_json(path: PathBuf, value: &Value) -> ProbeResult<()> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn main() -> ProbeResult<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("OUTPUT")?);
    let width: f64 = args.next().ok_or("WIDTH")?.parse()?;
    let height: f64 = args.next().ok_or("HEIGHT")?.parse()?;
    let scale: f64 = args.next().ok_or("SCALE")?.parse()?;
    let fixture_path = PathBuf::from(args.next().ok_or("FIXTURES_JSON")?);
    let name = args.next().ok_or("CASE")?;
    let policy = args.next().ok_or("POLICY")?;
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    let fixtures: Value = serde_json::from_slice(&std::fs::read(&fixture_path)?)?;
    if fixtures["schema_version"] != json!(1) {
        return Err("unsupported fixture schema".into());
    }
    let fixture = fixtures["cases"]
        .as_array()
        .ok_or("case array required")?
        .iter()
        .find(|case| case["name"].as_str() == Some(name.as_str()))
        .ok_or("unknown case")?;
    let raster_configuration = match policy.as_str() {
        "auto" => RasterConfiguration::deterministic_aliased(false),
        "fitted" => RasterConfiguration::chromium_linux_monochrome(),
        _ => return Err("POLICY must be auto or fitted".into()),
    };
    std::fs::create_dir(&out)?;
    let callbacks = Rc::new(Cell::new(0_u32));
    let callback_errors = Rc::new(RefCell::new(Vec::<String>::new()));
    let callback_thread_ok = Rc::new(Cell::new(true));
    let callback_target_ids = Rc::new(RefCell::new(Vec::<Option<String>>::new()));
    let mut failures = Vec::<String>::new();
    let weak_nodes = {
        let document = Document::with_font_collection_and_options(
            ViewportMetrics::from_logical_size(width, height, scale)?,
            FontCollection::system(),
            EngineOptions {
                raster_configuration,
            },
        )?;
        let mut by_id = BTreeMap::new();
        by_id.insert("body".to_owned(), document.body());
        let mut nodes = Vec::new();
        for node in fixture["nodes"].as_array().ok_or("node array required")? {
            let id = string(&node["id"])?;
            let tag = string(&node["tag"])?;
            let element = if tag == "body" {
                document.body()
            } else {
                let element = Element::create(&document, tag)?;
                element.set_id(id)?;
                let parent = by_id
                    .get(string(&node["parent"])?)
                    .ok_or("parent must precede child")?;
                parent.append_child(&element)?;
                by_id.insert(id.to_owned(), element.clone());
                element
            };
            for (attribute, value) in node["attributes"]
                .as_object()
                .ok_or("attribute object required")?
            {
                let value = string(value)?;
                match attribute.as_str() {
                    "colspan" | "span" => element.set_column_span(value.parse()?)?,
                    "rowspan" => element.set_row_span(value.parse()?)?,
                    _ => element.set_attribute(attribute, value)?,
                }
            }
            apply(&element, &node["styles"])?;
            let text = string(&node["text"])?;
            if !text.is_empty() {
                element.append_text_node(text)?;
            }
            if node["probe"].as_bool() == Some(true) {
                nodes.push((id.to_owned(), tag.to_owned(), element));
            }
        }
        document.update_all()?;
        let initial = snapshot(&document, &nodes, &fixture["hit_points"])?;
        let frame = document.render_to_png_buffer()?;
        let repeat = document.render_to_png_buffer()?;
        std::fs::write(out.join("frame.png"), &frame)?;
        std::fs::write(out.join("unchanged-frame.png"), &repeat)?;
        write_json(out.join("bounds.json"), &initial)?;
        if frame != repeat {
            failures.push("unchanged frame differs".into());
        }
        let continuation_nodes = initial["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| {
                matches!(node["tag"].as_str(), Some("table" | "tr" | "td"))
                    && node["client_rects"]
                        .as_array()
                        .is_some_and(|rects| rects.len() > 1)
            })
            .map(|node| node["name"].clone())
            .collect::<Vec<_>>();
        if fixture["requires_continuations"].as_bool() == Some(true)
            && continuation_nodes.is_empty()
        {
            failures.push("required real table continuations are absent".into());
        }
        let table = nodes
            .iter()
            .find(|(_, tag, _)| tag == "table")
            .ok_or("missing table")?
            .2
            .clone();
        let target = nodes
            .iter()
            .find(|(_, tag, _)| tag == "td")
            .ok_or("missing cell")?
            .2
            .clone();
        let count = callbacks.clone();
        let errors = callback_errors.clone();
        let threads = callback_thread_ok.clone();
        let targets = callback_target_ids.clone();
        let owner_thread = std::thread::current().id();
        let weak = table.downgrade();
        table.on("click", move |event| {
            threads.set(threads.get() && std::thread::current().id() == owner_thread);
            targets.borrow_mut().push(
                event
                    .target()
                    .and_then(|target| target.get_attribute("id").ok().flatten()),
            );
            let opacity = if count.get() == 0 { 0.5 } else { 1.0 };
            count.set(count.get() + 1);
            match weak.upgrade() {
                Some(table) => {
                    if let Err(error) = table.set_opacity(opacity) {
                        errors.borrow_mut().push(error.to_string());
                    }
                }
                None => errors
                    .borrow_mut()
                    .push("table expired during callback".into()),
            }
        })?;
        target.dispatch_click_event()?;
        document.update_all()?;
        let callback_frame = document.render_to_png_buffer()?;
        let callback_snapshot = snapshot(&document, &nodes, &fixture["hit_points"])?;
        std::fs::write(out.join("callback-frame.png"), &callback_frame)?;
        write_json(out.join("callback-bounds.json"), &callback_snapshot)?;
        target.dispatch_click_event()?;
        document.update_all()?;
        let restored = document.render_to_png_buffer()?;
        let restored_snapshot = snapshot(&document, &nodes, &fixture["hit_points"])?;
        std::fs::write(out.join("restored-frame.png"), &restored)?;
        write_json(out.join("restored-bounds.json"), &restored_snapshot)?;
        if callbacks.get() != 2 || !callback_errors.borrow().is_empty() || !callback_thread_ok.get()
        {
            failures.push("native callback lifecycle failed".into());
        }
        let expected_target = target.get_attribute("id")?;
        if *callback_target_ids.borrow() != vec![expected_target.clone(), expected_target] {
            failures.push("native callback target differs from the dispatched cell".into());
        }
        if callback_frame == frame {
            failures.push("native callback opacity mutation produced no visible change".into());
        }
        if restored != frame || restored_snapshot != initial {
            failures.push("native callback restoration changed pixels or geometry".into());
        }
        write_json(
            out.join("lifecycle.json"),
            &json!({"case":name,"policy":policy,"native_click_callbacks":callbacks.get(),"native_callback_target_ids":callback_target_ids.borrow().clone(),"native_callback_errors":callback_errors.borrow().clone(),"callbacks_on_ui_thread":callback_thread_ok.get(),"unchanged_frame_byte_repeat":frame==repeat,"callback_mutated_pixels":callback_frame!=frame,"callback_restored_pixels":restored==frame,"callback_restored_geometry":restored_snapshot==initial,"continuation_nodes":continuation_nodes,"required_continuations":fixture["requires_continuations"],"javascript_executed_by_openui":false,"private_engine_access":false,"runtime_html_css_parser":false,"failures":failures,"weak_teardown_passed":false}),
        )?;
        nodes
            .iter()
            .map(|(_, _, node)| node.downgrade())
            .collect::<Vec<_>>()
    };
    let teardown = weak_nodes.iter().all(|node| node.upgrade().is_none());
    if !teardown {
        failures.push("native retained document leaked".into());
    }
    let mut lifecycle: Value = serde_json::from_slice(&std::fs::read(out.join("lifecycle.json"))?)?;
    lifecycle["weak_teardown_passed"] = json!(teardown);
    lifecycle["failures"] = json!(failures);
    write_json(out.join("lifecycle.json"), &lifecycle)?;
    if !failures.is_empty() {
        return Err(format!("native guard failed: {}", failures.join("; ")).into());
    }
    Ok(())
}
