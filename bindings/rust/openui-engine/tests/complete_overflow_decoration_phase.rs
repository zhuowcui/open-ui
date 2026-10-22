//! Complete source decorations remain owned by their ordinary column replay.
//! Native semantic guards; no new Chromium reference or pixel tolerance.
use openui_compositor::{Frame, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineOptions, NodeHandle, ViewportMetrics};
use openui_geometry::{Length, RasterConfiguration};
use openui_style::*;

fn set(e: &mut Engine, n: NodeHandle, value: RendererStyleValue, authored: bool) {
    if authored {
        e.set_property(n, value.property(), StyleValue::Renderer(value))
            .unwrap();
    } else {
        e.set_renderer_style(n, value).unwrap();
    }
}

fn element(e: &mut Engine, parent: NodeHandle, tag: ElementTag, authored: bool) -> NodeHandle {
    let node = e.create_native_element(tag).unwrap();
    e.append_child(parent, node).unwrap();
    for value in [
        RendererStyleValue::Display(Display::Block),
        RendererStyleValue::BoxSizing(BoxSizing::ContentBox),
        RendererStyleValue::MarginTop(Length::px(0.0)),
        RendererStyleValue::MarginRight(Length::px(0.0)),
        RendererStyleValue::MarginBottom(Length::px(0.0)),
        RendererStyleValue::MarginLeft(Length::px(0.0)),
        RendererStyleValue::PaddingTop(Length::px(0.0)),
        RendererStyleValue::PaddingRight(Length::px(0.0)),
        RendererStyleValue::PaddingBottom(Length::px(0.0)),
        RendererStyleValue::PaddingLeft(Length::px(0.0)),
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
    ] {
        set(e, node, value, authored);
    }
    node
}

fn fixture(scale: f64, authored: bool) -> (Engine, NodeHandle) {
    let mut e = Engine::new_with_font_collection_and_options(
        ViewportMetrics::from_logical_size(400.0, 240.0, scale).unwrap(),
        openui_text::FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )
    .unwrap();
    let root = e.root();
    set(
        &mut e,
        root,
        RendererStyleValue::BackgroundColor(Color::WHITE),
        authored,
    );
    set(
        &mut e,
        root,
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
        authored,
    );
    let body = element(&mut e, root, ElementTag::Div, authored);
    for value in [
        RendererStyleValue::Display(Display::FlowRoot),
        RendererStyleValue::PaddingTop(Length::px(20.0)),
        RendererStyleValue::PaddingLeft(Length::px(20.0)),
    ] {
        set(&mut e, body, value, authored);
    }
    let columns = element(&mut e, body, ElementTag::Div, authored);
    for value in [
        RendererStyleValue::Width(Length::px(100.0)),
        RendererStyleValue::Height(Length::px(30.0)),
        RendererStyleValue::ColumnCount(Some(2)),
        RendererStyleValue::ColumnGap(Some(Length::px(16.0))),
        RendererStyleValue::ColumnFill(ColumnFill::Auto),
        RendererStyleValue::BorderTopWidth(2),
        RendererStyleValue::BorderRightWidth(2),
        RendererStyleValue::BorderBottomWidth(2),
        RendererStyleValue::BorderLeftWidth(2),
        RendererStyleValue::BorderTopStyle(BorderStyle::Solid),
        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
        RendererStyleValue::BorderLeftStyle(BorderStyle::Solid),
    ] {
        set(&mut e, columns, value, authored);
    }
    let source = element(&mut e, columns, ElementTag::Div, authored);
    for value in [
        RendererStyleValue::MaxHeight(Length::px(40.0)),
        RendererStyleValue::BackgroundColor(Color::from_rgba8(0, 128, 128, 255)),
        RendererStyleValue::BorderRightWidth(5),
        RendererStyleValue::BorderBottomWidth(5),
        RendererStyleValue::BorderLeftWidth(5),
        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
        RendererStyleValue::BorderLeftStyle(BorderStyle::Solid),
        RendererStyleValue::BorderRightColor(StyleColor::Resolved(Color::BLACK)),
        RendererStyleValue::BorderBottomColor(StyleColor::Resolved(Color::BLACK)),
        RendererStyleValue::BorderLeftColor(StyleColor::Resolved(Color::BLACK)),
    ] {
        set(&mut e, source, value, authored);
    }
    // Canvas is an indivisible source, without the missing-image icon/text
    // paint of an unavailable Image resource.
    let child = element(&mut e, source, ElementTag::Canvas, authored);
    for value in [
        RendererStyleValue::Width(Length::percent(50.0)),
        RendererStyleValue::Height(Length::px(80.0)),
        RendererStyleValue::BackgroundColor(Color::from_rgba8(250, 128, 114, 255)),
    ] {
        set(&mut e, child, value, authored);
    }
    let after = element(&mut e, source, ElementTag::Div, authored);
    set(
        &mut e,
        after,
        RendererStyleValue::Height(Length::px(80.0)),
        authored,
    );
    (e, child)
}

fn observed(e: &mut Engine) -> Frame {
    e.set_layered_paint_capture_enabled(false);
    let scene = e.scene().unwrap();
    let ordinary = SoftwareCompositor::default().render(&scene).unwrap();
    e.set_layered_paint_capture_enabled(true);
    let scene = e.scene().unwrap();
    let captured = SoftwareCompositor::default().render(&scene).unwrap();
    assert_eq!(
        (ordinary.width, ordinary.height, ordinary.stride),
        (captured.width, captured.height, captured.stride)
    );
    assert_eq!(ordinary.pixels.len(), captured.pixels.len());
    assert!(
        ordinary.pixels == captured.pixels,
        "capture changed complete frame"
    );
    let stats = e.stats();
    let generation = scene.generation();
    assert_eq!(e.scene().unwrap().generation(), generation);
    assert_eq!(e.stats(), stats);
    ordinary
}

fn cell(frame: &Frame, scale: f64, x: f64, y: f64) -> &[u8] {
    let px = (x * scale).floor() as usize;
    let py = (y * scale).floor() as usize;
    assert!(px < frame.width as usize && py < frame.height as usize);
    &frame.pixels[py * frame.stride as usize + px * 4..][..4]
}

#[test]
fn complete_source_background_and_border_survive_normal_prepaint() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for authored in [false, true] {
            let (mut e, child) = fixture(scale, authored);
            let original = observed(&mut e);
            // Authored outer starts20+border2; its first column ends52.
            // The max-constrained source owns45px including its bottom
            // border, so complete decoration continues to67 behind the
            // independently replayed80px child. These samples are interiors.
            assert_eq!(
                cell(&original, scale, 50.0, 60.0),
                &[0, 128, 128, 255],
                "complete source background lost: scale={scale} authored={authored}"
            );
            assert_eq!(
                cell(&original, scale, 62.0, 60.0),
                &[0, 0, 0, 255],
                "complete source border lost: scale={scale} authored={authored}"
            );
            assert_eq!(
                cell(&original, scale, 30.0, 80.0),
                &[250, 128, 114, 255],
                "indivisible child ink lost: scale={scale} authored={authored}"
            );

            set(
                &mut e,
                child,
                RendererStyleValue::BackgroundColor(Color::BLUE),
                authored,
            );
            let changed = observed(&mut e);
            assert_eq!(cell(&changed, scale, 30.0, 80.0), &[0, 0, 255, 255]);
            assert_eq!(cell(&changed, scale, 50.0, 60.0), &[0, 128, 128, 255]);
            set(
                &mut e,
                child,
                RendererStyleValue::BackgroundColor(Color::from_rgba8(250, 128, 114, 255)),
                authored,
            );
            let restored = observed(&mut e);
            assert_eq!(
                (restored.width, restored.height, restored.stride),
                (original.width, original.height, original.stride)
            );
            assert!(
                restored.pixels == original.pixels,
                "complete color restore changed frame"
            );
        }
    }
}
