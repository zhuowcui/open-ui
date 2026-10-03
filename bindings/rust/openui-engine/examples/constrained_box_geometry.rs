//! Native Engine geometry probe for a constrained box with visible child overflow.
//! The states correspond to preserved static Chromium inputs; no script is run.
//!
//! cargo run -p openui-engine --example constrained_box_geometry -- OUTPUT_DIR CASE [SCALE]

use openui_compositor::{SceneRect, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, ViewportMetrics};
use openui_geometry::Length;
use openui_layout::Fragment;
use openui_style::{
    Border, BorderStyle, BoxDecorationBreak, Color, Display, FlexDirection, LengthValue, Overflow,
    RendererStyleValue, StyleProperty, StyleValue, WritingMode,
};
use std::path::PathBuf;

fn rect_json(rect: SceneRect) -> String {
    format!(
        "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
        rect.x, rect.y, rect.width, rect.height
    )
}

fn fragment_dump(fragment: &Fragment, depth: usize, output: &mut String) {
    use std::fmt::Write;
    writeln!(
        output,
        "{}node={:?} kind={:?} offset={:?} size={:?} principal_box={:?} slice={:?} paint_limit={:?} writing_direction={:?} first={} last={} overflow_clip={} block_clip={}",
        " ".repeat(depth * 2), fragment.node_id, fragment.kind, fragment.offset, fragment.size,
        fragment.principal_box_rect,
        fragment.decoration_slice, fragment.decoration_paint_block_size,
        fragment.fragmentation_writing_direction, fragment.is_first_for_node, fragment.is_last_for_node,
        fragment.has_overflow_clip, fragment.block_axis_clip_only,
    ).unwrap();
    for child in &fragment.children {
        fragment_dump(child, depth + 1, output);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=6).contains(&arguments.len()) {
        return Err(
            "expected OUTPUT_DIR CASE [SCALE] [CLIPPED_DESCENDANT_HEIGHT] [COLUMN_MAX_HEIGHT] [WRAPPER_PADDING_TOP]"
                .into(),
        );
    }
    let output = PathBuf::from(&arguments[0]);
    let case = arguments[1].as_str();
    let scale: f64 = arguments.get(2).map_or(Ok(1.0), |value| value.parse())?;
    let descendant_height: f32 = arguments.get(3).map_or(Ok(20.0), |value| value.parse())?;
    if !descendant_height.is_finite()
        || descendant_height < 0.0
        || (arguments.len() >= 4 && case != "clipped-descendant")
    {
        return Err(
            "clipped descendant height requires that case and a finite nonnegative value".into(),
        );
    }
    let column_max_height: Option<f32> = arguments.get(4).map(|value| value.parse()).transpose()?;
    if column_max_height.is_some_and(|height| !height.is_finite() || height < 0.0) {
        return Err("column maximum height must be finite and nonnegative".into());
    }
    let wrapper_padding_top: Option<f32> =
        arguments.get(5).map(|value| value.parse()).transpose()?;
    if wrapper_padding_top.is_some_and(|padding| !padding.is_finite() || padding < 0.0) {
        return Err("wrapper padding must be finite and nonnegative".into());
    }
    let vertical = matches!(case, "vertical-rl" | "vertical-lr");
    let mut engine = Engine::new(ViewportMetrics::from_logical_size(320.0, 340.0, scale)?)?;
    let root = engine.root();
    engine.set_property(root, StyleProperty::BackgroundColor, Color::WHITE.into())?;
    let columns = engine.create_element(ElementTag::Div)?;
    engine.append_child(root, columns)?;
    engine.set_property(columns, StyleProperty::Display, Display::Block.into())?;
    if let Some(maximum) = column_max_height {
        engine.set_property(
            columns,
            StyleProperty::MaxHeight,
            LengthValue::px(maximum).into(),
        )?;
    }
    engine.set_property(
        columns,
        if vertical {
            StyleProperty::Height
        } else {
            StyleProperty::Width
        },
        LengthValue::px(300.0).into(),
    )?;
    if vertical {
        engine.set_property(
            columns,
            StyleProperty::WritingMode,
            StyleValue::Renderer(RendererStyleValue::WritingMode(if case == "vertical-rl" {
                WritingMode::VerticalRl
            } else {
                WritingMode::VerticalLr
            })),
        )?;
    }
    engine.set_property(
        columns,
        StyleProperty::ColumnCount,
        StyleValue::Renderer(RendererStyleValue::ColumnCount(Some(3))),
    )?;
    engine.set_property(
        columns,
        StyleProperty::ColumnGap,
        LengthValue::px(24.0).into(),
    )?;
    engine.set_property(
        columns,
        StyleProperty::BackgroundColor,
        Color::from_rgba8(128, 128, 128, 255).into(),
    )?;
    let wrapper = engine.create_element(ElementTag::Div)?;
    engine.append_child(columns, wrapper)?;
    engine.set_property(wrapper, StyleProperty::Display, Display::Block.into())?;
    if vertical {
        engine.set_property(
            wrapper,
            StyleProperty::WritingMode,
            StyleValue::Renderer(RendererStyleValue::WritingMode(if case == "vertical-rl" {
                WritingMode::VerticalRl
            } else {
                WritingMode::VerticalLr
            })),
        )?;
    }
    let (property, maximum) = match case {
        "max160"
        | "clipped-descendant"
        | "border-padding"
        | "clone-border-padding"
        | "overflow-hidden"
        | "min-overrides-max"
        | "flow-root"
        | "flex-column" => (StyleProperty::MaxHeight, 160.0),
        "max120" => (StyleProperty::MaxHeight, 120.0),
        "max0" => (StyleProperty::MaxHeight, 0.0),
        "height160" => (StyleProperty::Height, 160.0),
        "height120" => (StyleProperty::Height, 120.0),
        "vertical-rl" | "vertical-lr" => (StyleProperty::MaxWidth, 160.0),
        _ => return Err("unknown case".into()),
    };
    engine.set_property(wrapper, property, LengthValue::px(maximum).into())?;
    if let Some(padding) = wrapper_padding_top {
        engine.set_property(
            wrapper,
            StyleProperty::PaddingTop,
            LengthValue::px(padding).into(),
        )?;
    }
    engine.set_property(
        wrapper,
        StyleProperty::BackgroundColor,
        Color::from_rgba8(255, 255, 0, 255).into(),
    )?;
    if matches!(case, "border-padding" | "clone-border-padding") {
        for property in [
            StyleProperty::BorderTop,
            StyleProperty::BorderRight,
            StyleProperty::BorderBottom,
            StyleProperty::BorderLeft,
        ] {
            engine.set_property(
                wrapper,
                property,
                Border {
                    width: 2.0,
                    style: BorderStyle::Solid,
                    color: Color::BLUE,
                }
                .into(),
            )?;
        }
        for property in [
            StyleProperty::PaddingTop,
            StyleProperty::PaddingRight,
            StyleProperty::PaddingBottom,
            StyleProperty::PaddingLeft,
        ] {
            engine.set_property(wrapper, property, LengthValue::px(3.0).into())?;
        }
    }
    if case == "clone-border-padding" {
        engine.set_property(
            wrapper,
            StyleProperty::BoxDecorationBreak,
            StyleValue::Renderer(RendererStyleValue::BoxDecorationBreak(
                BoxDecorationBreak::Clone,
            )),
        )?;
    }
    if case == "overflow-hidden" {
        engine.set_property(wrapper, StyleProperty::Overflow, Overflow::Hidden.into())?;
    }
    if case == "min-overrides-max" {
        engine.set_property(
            wrapper,
            StyleProperty::MinHeight,
            LengthValue::px(180.0).into(),
        )?;
    }
    if case == "flow-root" {
        engine.set_property(wrapper, StyleProperty::Display, Display::FlowRoot.into())?;
    }
    if case == "flex-column" {
        engine.set_property(wrapper, StyleProperty::Display, Display::Flex.into())?;
        engine.set_property(
            wrapper,
            StyleProperty::FlexDirection,
            FlexDirection::Column.into(),
        )?;
    }
    let child = engine.create_element(ElementTag::Div)?;
    engine.append_child(wrapper, child)?;
    engine.set_property(child, StyleProperty::Display, Display::Block.into())?;
    engine.set_property(
        child,
        StyleProperty::Width,
        LengthValue::px(if vertical { 200.0 } else { 50.0 }).into(),
    )?;
    engine.set_property(
        child,
        StyleProperty::Height,
        LengthValue::px(if vertical { 50.0 } else { 200.0 }).into(),
    )?;
    if vertical {
        engine.set_property(
            child,
            StyleProperty::WritingMode,
            StyleValue::Renderer(RendererStyleValue::WritingMode(if case == "vertical-rl" {
                WritingMode::VerticalRl
            } else {
                WritingMode::VerticalLr
            })),
        )?;
    }
    engine.set_property(child, StyleProperty::FlexShrink, 0.0_f32.into())?;
    engine.set_property(
        child,
        StyleProperty::FlexBasis,
        StyleValue::Renderer(RendererStyleValue::FlexBasis(Length::auto())),
    )?;
    for property in [
        StyleProperty::BorderTop,
        StyleProperty::BorderRight,
        StyleProperty::BorderBottom,
        StyleProperty::BorderLeft,
    ] {
        engine.set_property(
            child,
            property,
            Border {
                width: 3.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            }
            .into(),
        )?;
    }
    if case == "clipped-descendant" {
        let descendant = engine.create_element(ElementTag::Div)?;
        engine.append_child(child, descendant)?;
        engine.set_property(descendant, StyleProperty::Display, Display::Block.into())?;
        engine.set_property(
            descendant,
            StyleProperty::Width,
            LengthValue::px(20.0).into(),
        )?;
        engine.set_property(
            descendant,
            StyleProperty::Height,
            LengthValue::px(descendant_height).into(),
        )?;
        engine.set_property(descendant, StyleProperty::Overflow, Overflow::Hidden.into())?;
    }
    let scene = engine.scene()?;
    std::fs::create_dir_all(output.parent().unwrap())?;
    std::fs::create_dir(&output)?;
    let mut nodes = Vec::new();
    for (name, handle) in [("columns", columns), ("limit", wrapper), ("border", child)] {
        let rects = engine
            .client_rects(handle)?
            .into_iter()
            .map(rect_json)
            .collect::<Vec<_>>()
            .join(",");
        let bounds = engine
            .bounds(handle)?
            .map(rect_json)
            .unwrap_or_else(|| "null".into());
        nodes.push(format!(
            "\"{name}\":{{\"bounds\":{bounds},\"rects\":[{rects}]}}"
        ));
    }
    let json = format!(
        "{{\"case\":\"{case}\",\"viewport\":[320,340,{scale}],\"nodes\":{{{}}}}}\n",
        nodes.join(",")
    );
    std::fs::write(output.join("geometry.json"), &json)?;
    let mut fragments = String::new();
    fragment_dump(scene.fragments(), 0, &mut fragments);
    std::fs::write(output.join("fragments.txt"), fragments)?;
    std::fs::write(
        output.join("openui.png"),
        SoftwareCompositor::default().render_png(&scene)?,
    )?;
    print!("{json}");
    Ok(())
}
