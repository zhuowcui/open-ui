//! Native Engine reproducer for a hollow rectangular border at device scale.
//! Writes its native render and equivalent static Chromium input.
//!
//! cargo run -p openui-engine --example border_raster_reproducer -- \
//!   OUTPUT_DIR SCALE BORDER_WIDTH OFFSET VIEWPORT_WIDTH VIEWPORT_HEIGHT [plain|clip|columns]

use openui_compositor::SoftwareCompositor;
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineOptions, ViewportMetrics};
use openui_geometry::{Length, RasterConfiguration};
use openui_style::{
    Border, BorderStyle, Color, Display, LengthValue, Overflow, Position, RendererStyleValue,
    StyleProperty, StyleValue,
};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.len() != 6 && arguments.len() != 7 {
        return Err(
            "expected OUTPUT_DIR SCALE BORDER_WIDTH OFFSET VIEWPORT_WIDTH VIEWPORT_HEIGHT [plain|clip|columns]".into(),
        );
    }
    let output = PathBuf::from(&arguments[0]);
    let scale: f64 = arguments[1].parse()?;
    let border_width: f32 = arguments[2].parse()?;
    let offset: f32 = arguments[3].parse()?;
    let width: f64 = arguments[4].parse()?;
    let height: f64 = arguments[5].parse()?;
    let mode = arguments.get(6).map_or("plain", String::as_str);
    if !border_width.is_finite() || border_width < 0.0 || !offset.is_finite() {
        return Err("border width must be finite and nonnegative; offset must be finite".into());
    }
    if !matches!(mode, "plain" | "clip" | "columns") {
        return Err("mode must be plain, clip, or columns".into());
    }
    let viewport = ViewportMetrics::from_logical_size(width, height, scale)?;
    let mut engine = Engine::new_with_font_collection_and_options(
        viewport,
        openui_text::FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )?;
    let root = engine.root();
    engine.set_property(
        root,
        StyleProperty::BackgroundColor,
        if mode == "plain" {
            Color::from_rgba8(255, 255, 0, 255)
        } else {
            Color::WHITE
        }
        .into(),
    )?;
    let mut parent = root;
    let mut opening = String::new();
    let mut closing = String::new();
    let mut parent_css = String::new();
    if mode == "clip" {
        let clip = engine.create_element(ElementTag::Div)?;
        engine.append_child(root, clip)?;
        engine.set_property(clip, StyleProperty::Display, Display::Block.into())?;
        engine.set_property(clip, StyleProperty::Position, Position::Relative.into())?;
        engine.set_property(clip, StyleProperty::Width, LengthValue::px(84.0).into())?;
        engine.set_property(
            clip,
            StyleProperty::Height,
            LengthValue::px(68.671875).into(),
        )?;
        engine.set_property(clip, StyleProperty::Overflow, Overflow::Hidden.into())?;
        engine.set_property(
            clip,
            StyleProperty::BackgroundColor,
            Color::from_rgba8(255, 255, 0, 255).into(),
        )?;
        parent = clip;
        opening.push_str("<div class=clip>");
        closing.push_str("</div>");
        parent_css.push_str(".clip{position:relative;width:84px;height:68.671875px;overflow:hidden;background:yellow}");
    } else if mode == "columns" {
        let columns = engine.create_element(ElementTag::Div)?;
        engine.append_child(root, columns)?;
        engine.set_property(columns, StyleProperty::Display, Display::Block.into())?;
        engine.set_property(columns, StyleProperty::Width, LengthValue::px(300.0).into())?;
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
        let limit = engine.create_element(ElementTag::Div)?;
        engine.append_child(columns, limit)?;
        engine.set_property(limit, StyleProperty::Display, Display::Block.into())?;
        engine.set_property(
            limit,
            StyleProperty::MaxHeight,
            LengthValue::px(160.0).into(),
        )?;
        engine.set_property(
            limit,
            StyleProperty::BackgroundColor,
            Color::from_rgba8(255, 255, 0, 255).into(),
        )?;
        parent = limit;
        opening.push_str("<div class=columns><div class=limit>");
        closing.push_str("</div></div>");
        parent_css.push_str(".columns{width:300px;column-count:3;column-gap:24px;background:grey}.limit{max-height:160px;background:yellow}");
    }
    let border = engine.create_element(ElementTag::Div)?;
    engine.append_child(parent, border)?;
    engine.set_property(border, StyleProperty::Display, Display::Block.into())?;
    let position_css = if mode == "columns" {
        engine.set_property(
            border,
            StyleProperty::MarginLeft,
            LengthValue::px(offset).into(),
        )?;
        engine.set_property(
            border,
            StyleProperty::MarginTop,
            LengthValue::px(offset).into(),
        )?;
        format!("margin-left:{offset}px;margin-top:{offset}px")
    } else {
        engine.set_property(border, StyleProperty::Position, Position::Absolute.into())?;
        engine.set_property(
            border,
            StyleProperty::Left,
            StyleValue::Renderer(RendererStyleValue::Left(Length::px(offset))),
        )?;
        engine.set_property(
            border,
            StyleProperty::Top,
            StyleValue::Renderer(RendererStyleValue::Top(Length::px(offset))),
        )?;
        format!("position:absolute;left:{offset}px;top:{offset}px")
    };
    engine.set_property(border, StyleProperty::Width, LengthValue::px(50.0).into())?;
    engine.set_property(border, StyleProperty::Height, LengthValue::px(200.0).into())?;
    for property in [
        StyleProperty::BorderTop,
        StyleProperty::BorderRight,
        StyleProperty::BorderBottom,
        StyleProperty::BorderLeft,
    ] {
        engine.set_property(
            border,
            property,
            Border {
                width: border_width,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            }
            .into(),
        )?;
    }
    let scene = engine.scene()?;
    println!(
        "native_border={:?} native_parent={:?}",
        engine.bounds(border)?,
        engine.bounds(parent)?,
    );
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir(&output)?;
    std::fs::write(
        output.join("openui.png"),
        SoftwareCompositor::default().render_png(&scene)?,
    )?;
    let background = if mode == "plain" { "yellow" } else { "white" };
    std::fs::write(output.join("test.html"), format!(
        "<!DOCTYPE html><html><head><style>html,body{{margin:0;padding:0;height:100%;background:{background};overflow:hidden}}{parent_css}.border{{{position_css};width:50px;height:200px;border:{border_width}px solid black}}</style></head><body>{opening}<div class=border></div>{closing}</body></html>"
    ))?;
    println!(
        "viewport={width}x{height}@{scale} border={border_width} offset={offset} mode={mode} output={}",
        output.display()
    );
    Ok(())
}
