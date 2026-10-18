//! Public native application states for image fallback in constrained columns.

use openui::prelude::*;
use std::path::PathBuf;

fn block(document: &Document, parent: &Element) -> Result<Element, openui::Error> {
    let element = Element::create(document, "div")?;
    element.set_display(Display::Block)?;
    parent.append_child(&element)?;
    Ok(element)
}

fn rectangles(element: &Element) -> Result<String, openui::Error> {
    let rectangle = |rect: Rect| {
        format!(
            "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
            rect.x, rect.y, rect.width, rect.height
        )
    };
    let bounds = element
        .bounding_rect()?
        .map(rectangle)
        .unwrap_or_else(|| "null".into());
    let rects = element
        .client_rects()?
        .into_iter()
        .map(rectangle)
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("{{\"bounds\":{bounds},\"rects\":[{rects}]}}"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().unwrap_or_else(|| "1".into()).parse()?;
    let case = args.next().unwrap_or_else(|| "max-image".into());
    if !matches!(
        case.as_str(),
        "max-image"
            | "max-image-flow-root"
            | "max-image-author-styles"
            | "max-image-deterministic-font"
            | "max-image-gradient-clear"
            | "height-image"
            | "max-div"
            | "height-div"
            | "max-src-less"
            | "max-width100"
            | "max-width25"
            | "max-fixed16"
            | "max-fixed20"
    ) {
        return Err("unknown native column image state".into());
    }
    let viewport_metrics = ViewportMetrics::from_logical_size(320.0, 340.0, scale)?;
    let document = if matches!(
        case.as_str(),
        "max-image-author-styles" | "max-image-deterministic-font"
    ) {
        Document::with_font_collection(viewport_metrics, FontCollection::deterministic_test())?
    } else {
        Document::with_viewport_metrics(viewport_metrics)?
    };
    let viewport = document.body();
    viewport.set_background_color(if case == "max-image-author-styles" {
        Color::TRANSPARENT
    } else {
        Color::WHITE
    })?;
    let root = if matches!(
        case.as_str(),
        "max-image-flow-root" | "max-image-author-styles"
    ) {
        let body = block(&document, &viewport)?;
        body.set_display(Display::FlowRoot)?;
        body
    } else {
        viewport
    };
    root.set_padding(Edges::all(LengthValue::px(20.0)))?;
    let columns = block(&document, &root)?;
    columns.add_class("columns")?;
    columns.set_width(LengthValue::px(100.0))?;
    columns.set_height(LengthValue::px(30.0))?;
    columns.set_column_width(Some(Length::px(30.0)))?;
    columns.set_column_fill(ColumnFill::Auto)?;
    columns.set_border(Border {
        width: 2.0,
        style: BorderStyle::Solid,
        color: Color::from_rgba8(255, 165, 0, 255),
    })?;
    let wrapper = block(&document, &columns)?;
    wrapper.add_class("limit")?;
    wrapper.set_background_color(Color::from_rgba8(0, 128, 128, 255))?;
    wrapper.set_border(Border {
        width: 5.0,
        style: BorderStyle::Solid,
        color: Color::BLACK,
    })?;
    wrapper.set_border_top_width(0)?;
    if case.starts_with("height-") {
        wrapper.set_height(LengthValue::px(40.0))?;
    } else {
        wrapper.set_max_height(LengthValue::px(40.0))?;
    }
    let image = Element::create(
        &document,
        if case.ends_with("-div") { "div" } else { "img" },
    )?;
    image.set_display(Display::Block)?;
    image.add_class("image")?;
    image.set_width(match case.as_str() {
        "max-width100" => LengthValue::percent(100.0),
        "max-width25" => LengthValue::percent(25.0),
        "max-fixed16" => LengthValue::px(16.0),
        "max-fixed20" => LengthValue::px(20.0),
        _ => LengthValue::percent(50.0),
    })?;
    image.set_height(LengthValue::px(80.0))?;
    image.set_background_color(Color::from_rgba8(250, 128, 114, 255))?;
    if !case.ends_with("-div") && case != "max-src-less" {
        image.set_attribute("src", "")?;
    }
    wrapper.append_child(&image)?;
    let following = block(&document, &wrapper)?;
    following.add_class("following")?;
    following.set_width(LengthValue::percent(50.0))?;
    following.set_height(LengthValue::px(80.0))?;
    following.set_background_color(Color::from_rgba8(128, 0, 128, 255))?;
    if case == "max-image-gradient-clear" {
        image.set_background_linear_gradient(None)?;
    }
    if matches!(
        case.as_str(),
        "max-image-author-styles" | "max-image-deterministic-font"
    ) {
        for element in [&root, &columns, &wrapper, &image, &following] {
            element.set_font_family(FontFamilyList {
                families: vec![FontFamily::Named(
                    if case == "max-image-deterministic-font" {
                        "Droid Sans Fallback".into()
                    } else {
                        "Ahem".into()
                    },
                )],
            })?;
            element.set_box_sizing(BoxSizing::ContentBox)?;
            element.set_list_style(ListStyleType::None)?;
        }
        image.set_background_linear_gradient(None)?;
        following.set_background_linear_gradient(None)?;
    }
    let nodes = [
        ("columns", columns),
        ("limit", wrapper),
        ("image", image),
        ("following", following),
    ]
    .iter()
    .map(|(name, element)| rectangles(element).map(|value| format!("\"{name}\":{value}")))
    .collect::<Result<Vec<_>, _>>()?
    .join(",");
    std::fs::create_dir(&output)?;
    std::fs::write(
        output.join("geometry.json"),
        format!("{{\"nodes\":{{{nodes}}}}}\n"),
    )?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    Ok(())
}
