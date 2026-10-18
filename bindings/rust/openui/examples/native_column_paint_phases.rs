//! Public native Rust consumers of column background and atomic item paint phases.

use openui::prelude::*;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

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
    let case = args.next().unwrap_or_else(|| "max160".into());
    if !matches!(
        case.as_str(),
        "max160"
            | "following-sibling"
            | "next-item"
            | "row"
            | "reverse"
            | "following-relative"
            | "following-opacity"
            | "following-transform"
            | "following-containment"
            | "following-inline-block"
            | "grid-following-sibling"
    ) {
        return Err("unknown native column paint state".into());
    }
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 340.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let columns = block(&document, &root)?;
    columns.add_class("columns")?;
    columns.set_width(LengthValue::px(300.0))?;
    columns.set_column_count(Some(3))?;
    columns.set_column_gap(LengthValue::px(24.0))?;
    columns.set_background_color(Color::from_rgba8(128, 128, 128, 255))?;
    let wrapper = block(&document, &columns)?;
    wrapper.set_display(if case == "grid-following-sibling" {
        Display::Grid
    } else {
        Display::Flex
    })?;
    wrapper.set_flex_direction(match case.as_str() {
        "row" => FlexDirection::Row,
        "reverse" => FlexDirection::ColumnReverse,
        _ => FlexDirection::Column,
    })?;
    wrapper.add_class("limit")?;
    wrapper.set_max_height(LengthValue::px(160.0))?;
    wrapper.set_background_color(Color::from_rgba8(255, 255, 0, 255))?;
    let child = block(&document, &wrapper)?;
    child.set_width(LengthValue::px(50.0))?;
    child.set_height(LengthValue::px(200.0))?;
    child.set_border(Border {
        width: 3.0,
        style: BorderStyle::Solid,
        color: Color::BLACK,
    })?;
    child.set_flex_shrink(0.0)?;
    child.set_flex_basis(LengthValue::auto())?;
    child.set_id("overflowing child")?;
    child.add_class("border")?;
    let mut additional = Vec::new();
    if case.starts_with("following-")
        || matches!(case.as_str(), "next-item" | "grid-following-sibling")
    {
        let following = block(
            &document,
            if case == "next-item" {
                &wrapper
            } else {
                &columns
            },
        )?;
        following.add_class("following")?;
        following.set_width(LengthValue::px(20.0))?;
        following.set_height(LengthValue::px(20.0))?;
        following.set_flex_shrink(0.0)?;
        following.set_flex_basis(LengthValue::auto())?;
        following.set_background_color(Color::BLUE)?;
        match case.as_str() {
            "following-relative" => following.set_position(Position::Relative)?,
            "following-opacity" => following.set_opacity(0.5)?,
            "following-transform" => {
                following.set_transform(TransformList(vec![TransformOperation::Translate(
                    LengthValue::px(10.0),
                    LengthValue::px(0.0),
                )]))?
            }
            "following-containment" => following.set_contain(Containment::PAINT)?,
            "following-inline-block" => following.set_display(Display::InlineBlock)?,
            _ => {}
        }
        additional.push(("following", following));
    }
    let clicks = Rc::new(Cell::new(0));
    let observed = clicks.clone();
    child.on("click", move |_| observed.set(observed.get() + 1))?;
    if case == "max160" {
        assert_eq!(
            child
                .client_rects()?
                .iter()
                .map(|r| (r.x, r.y, r.width, r.height))
                .collect::<Vec<_>>(),
            [
                (0.0, 0.0, 56.0, 68.671875),
                (108.0, 0.0, 56.0, 68.671875),
                (216.0, 0.0, 56.0, 68.65625)
            ]
        );
        let hit = document.hit_test(240.0, 40.0)?.ok_or("child not hit")?;
        assert_eq!(
            hit.get_attribute("id")?.as_deref(),
            Some("overflowing child")
        );
        hit.click()?;
        assert_eq!(clicks.get(), 1);
    }
    let hit_targets = [(217.0, 30.0), (227.0, 30.0)]
        .into_iter()
        .map(|(x, y)| {
            let target = document.hit_test(x, y)?;
            let class = target
                .map(|element| element.get_attribute("class"))
                .transpose()?
                .flatten()
                .unwrap_or_default();
            Ok(format!("\"{x},{y}\":\"{class}\""))
        })
        .collect::<Result<Vec<_>, openui::Error>>()?
        .join(",");
    if case == "following-sibling" {
        let hit = document.hit_test(227.0, 30.0)?.ok_or("overlap not hit")?;
        assert_eq!(hit.get_attribute("class")?.as_deref(), Some("border"));
        hit.click()?;
        assert_eq!(clicks.get(), 1);
        let owned_before = child.client_rects()?;
        let following = &additional[0].1;
        following.set_position(Position::Relative)?;
        assert_eq!(
            document
                .hit_test(227.0, 30.0)?
                .unwrap()
                .get_attribute("class")?
                .as_deref(),
            Some("following")
        );
        following.set_position(Position::Static)?;
        assert_eq!(
            document
                .hit_test(227.0, 30.0)?
                .unwrap()
                .get_attribute("class")?
                .as_deref(),
            Some("border")
        );
        following.set_opacity(0.5)?;
        assert_eq!(
            document
                .hit_test(227.0, 30.0)?
                .unwrap()
                .get_attribute("class")?
                .as_deref(),
            Some("following")
        );
        following.set_opacity(1.0)?;
        assert_eq!(
            document
                .hit_test(227.0, 30.0)?
                .unwrap()
                .get_attribute("class")?
                .as_deref(),
            Some("border")
        );
        assert_eq!(child.client_rects()?, owned_before);
    }
    let mut nodes = vec![("columns", columns), ("limit", wrapper), ("border", child)];
    nodes.extend(additional);
    let nodes = nodes
        .iter()
        .map(|(name, element)| rectangles(element).map(|value| format!("\"{name}\":{value}")))
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    std::fs::create_dir(&output)?;
    std::fs::write(
        output.join("geometry.json"),
        format!(
            "{{\"nodes\":{{{nodes}}},\"hit_targets\":{{{hit_targets}}},\"clicks\":{}}}\n",
            clicks.get()
        ),
    )?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    Ok(())
}
