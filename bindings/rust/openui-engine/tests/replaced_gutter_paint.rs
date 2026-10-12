//! Native resource consumers use retained, layout-owned content geometry.
//! These guards require the shared stable-gutter layout repair. The source
//! checkpoint remains unqualified until the owner compiles and executes it.

use openui_compositor::{Frame, SceneRect, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_geometry::Length;
use openui_style::{
    Border, BorderCollapse, BorderStyle, Color, Containment, Direction, Display, Edges,
    LengthValue, ObjectFit, Overflow, Position, RendererStyleValue, ScrollbarGutter,
    ScrollbarWidth, StyleProperty, StyleValue, WritingMode,
};

const RESOURCE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="#008080"/></svg>"##;
const RESOURCE_SHA256: &str = "809bab3d90547e75eda8d473aa72a4f4f20b9e23ea7885f2e900eb0cf61aa6de";
const TEAL: [u8; 4] = [0, 128, 128, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn renderer(
    engine: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    value: RendererStyleValue,
) {
    engine
        .set_property(node, property, StyleValue::Renderer(value))
        .unwrap();
}

fn fixture(tag: ElementTag, scale: f64) -> (Engine, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap()).unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::BackgroundColor,
            Color::WHITE.into(),
        )
        .unwrap();
    let node = engine.create_native_element(tag).unwrap();
    engine.append_child(engine.root(), node).unwrap();
    engine
        .set_property(node, StyleProperty::Display, Display::Block.into())
        .unwrap();
    engine
        .set_property(node, StyleProperty::Position, Position::Absolute.into())
        .unwrap();
    for property in [StyleProperty::Left, StyleProperty::Top] {
        engine
            .set_property(node, property, LengthValue::px(20.0).into())
            .unwrap();
    }
    for (property, value) in [(StyleProperty::Width, 80.0), (StyleProperty::Height, 60.0)] {
        engine
            .set_property(node, property, LengthValue::px(value).into())
            .unwrap();
    }
    engine
        .set_property(
            node,
            StyleProperty::Padding,
            Edges::all(LengthValue::px(4.0)).into(),
        )
        .unwrap();
    engine
        .set_property(
            node,
            StyleProperty::Border,
            Border {
                width: 2.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            }
            .into(),
        )
        .unwrap();
    engine
        .set_property(node, StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap();
    renderer(
        &mut engine,
        node,
        StyleProperty::ScrollbarGutter,
        RendererStyleValue::ScrollbarGutter(ScrollbarGutter::Stable),
    );
    (engine, node)
}

fn image_fixture(scale: f64) -> (Engine, NodeHandle) {
    let (mut engine, image) = fixture(ElementTag::Image, scale);
    let resource = engine.register_image_resource(
        "memory:native-gutter-resource",
        "image/svg+xml",
        RESOURCE_SHA256,
        RESOURCE.to_vec(),
    );
    engine
        .set_image_resource(image, resource, Some((100.0, 50.0)))
        .unwrap();
    renderer(
        &mut engine,
        image,
        StyleProperty::ObjectFit,
        RendererStyleValue::ObjectFit(ObjectFit::Fill),
    );
    (engine, image)
}

fn pixel(frame: &Frame, x: f32, y: f32) -> &[u8] {
    let scale = frame.viewport.device_scale_factor() as f32;
    let x = (x * scale).floor() as usize;
    let y = (y * scale).floor() as usize;
    let index = y * frame.stride + x * 4;
    &frame.pixels[index..index + 4]
}

fn content(x: f32, y: f32, width: f32, height: f32) -> SceneRect {
    SceneRect {
        x,
        y,
        width,
        height,
    }
}

fn assert_filled_resource(frame: &Frame, rect: SceneRect) {
    let middle_x = rect.x + rect.width * 0.5;
    let middle_y = rect.y + rect.height * 0.5;
    for (x, y) in [
        (rect.x + 2.0, rect.y + 2.0),
        (rect.x + rect.width - 2.0, rect.y + rect.height - 2.0),
        (middle_x, middle_y),
    ] {
        assert_eq!(pixel(frame, x, y), TEAL, "missing resource at ({x}, {y})");
    }
    for (x, y) in [
        (rect.x - 2.0, middle_y),
        (rect.x + rect.width + 2.0, middle_y),
        (middle_x, rect.y - 2.0),
        (middle_x, rect.y + rect.height + 2.0),
    ] {
        assert_eq!(pixel(frame, x, y), WHITE, "resource leaked at ({x}, {y})");
    }
}

#[test]
fn native_image_gutters_move_and_resize_resource_content_without_changing_css_box() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, image) = image_fixture(scale);
        let mut compositor = SoftwareCompositor::default();
        for (direction, writing_mode, gutter, rect) in [
            (
                Direction::Ltr,
                WritingMode::HorizontalTb,
                ScrollbarGutter::Stable,
                content(26.0, 26.0, 65.0, 60.0),
            ),
            (
                Direction::Rtl,
                WritingMode::HorizontalTb,
                ScrollbarGutter::Stable,
                content(41.0, 26.0, 65.0, 60.0),
            ),
            (
                Direction::Ltr,
                WritingMode::HorizontalTb,
                ScrollbarGutter::StableBothEdges,
                content(41.0, 26.0, 50.0, 60.0),
            ),
            (
                Direction::Ltr,
                WritingMode::VerticalRl,
                ScrollbarGutter::Stable,
                content(26.0, 26.0, 80.0, 45.0),
            ),
            (
                Direction::Ltr,
                WritingMode::VerticalLr,
                ScrollbarGutter::StableBothEdges,
                content(26.0, 41.0, 80.0, 30.0),
            ),
        ] {
            engine
                .set_property(image, StyleProperty::Direction, direction.into())
                .unwrap();
            engine
                .set_property(image, StyleProperty::WritingMode, writing_mode.into())
                .unwrap();
            renderer(
                &mut engine,
                image,
                StyleProperty::ScrollbarGutter,
                RendererStyleValue::ScrollbarGutter(gutter),
            );
            let bounds = engine.bounds(image).unwrap().unwrap();
            assert_eq!(
                (bounds.x, bounds.y, bounds.width, bounds.height),
                (20.0, 20.0, 92.0, 72.0)
            );
            let scene = engine.scene().unwrap();
            let stats = engine.stats();
            let unchanged = engine.scene().unwrap();
            assert_eq!(
                engine.stats(),
                stats,
                "unchanged scene performed engine work"
            );
            let frame = compositor.render(&scene).unwrap();
            assert_filled_resource(&frame, rect);
            assert_eq!(compositor.render(&unchanged).unwrap(), frame);
            assert_eq!(engine.scroll_offset(image).unwrap(), (0.0, 0.0));
        }
        engine
            .set_property(
                image,
                StyleProperty::WritingMode,
                WritingMode::HorizontalTb.into(),
            )
            .unwrap();
        for (width, rect) in [
            (ScrollbarWidth::Thin, content(36.0, 26.0, 60.0, 60.0)),
            (ScrollbarWidth::None, content(26.0, 26.0, 80.0, 60.0)),
        ] {
            renderer(
                &mut engine,
                image,
                StyleProperty::ScrollbarWidth,
                RendererStyleValue::ScrollbarWidth(width),
            );
            assert_filled_resource(&compositor.render(&engine.scene().unwrap()).unwrap(), rect);
        }
        renderer(
            &mut engine,
            image,
            StyleProperty::ScrollbarWidth,
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::Auto),
        );
        engine
            .set_property(image, StyleProperty::Overflow, Overflow::Clip.into())
            .unwrap();
        assert_filled_resource(
            &compositor.render(&engine.scene().unwrap()).unwrap(),
            content(26.0, 26.0, 80.0, 60.0),
        );
    }
}

#[test]
fn object_fit_uses_resource_content_and_saved_scenes_keep_their_gutter_geometry() {
    let (mut engine, image) = image_fixture(1.25);
    let original = engine.scene().unwrap();
    let original_frame = SoftwareCompositor::default().render(&original).unwrap();
    renderer(
        &mut engine,
        image,
        StyleProperty::ObjectFit,
        RendererStyleValue::ObjectFit(ObjectFit::Contain),
    );
    let contained = engine.scene().unwrap();
    let contained_frame = SoftwareCompositor::default().render(&contained).unwrap();
    assert_eq!(pixel(&contained_frame, 58.0, 38.0), WHITE);
    assert_eq!(pixel(&contained_frame, 58.0, 52.0), TEAL);
    assert_eq!(pixel(&contained_frame, 58.0, 74.0), WHITE);
    assert_ne!(contained_frame.pixels, original_frame.pixels);
    renderer(
        &mut engine,
        image,
        StyleProperty::ScrollbarWidth,
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
    );
    let no_gutter = SoftwareCompositor::default()
        .render(&engine.scene().unwrap())
        .unwrap();
    assert_eq!(pixel(&no_gutter, 58.0, 38.0), TEAL);
    assert_eq!(pixel(&no_gutter, 58.0, 74.0), TEAL);
    drop(engine);
    let (replayed_original, replayed_contained) = std::thread::spawn(move || {
        (
            SoftwareCompositor::default().render(&original).unwrap(),
            SoftwareCompositor::default().render(&contained).unwrap(),
        )
    })
    .join()
    .unwrap();
    assert_eq!(replayed_original, original_frame);
    assert_eq!(replayed_contained, contained_frame);
}

#[test]
fn native_passive_video_uses_the_same_gutter_content_box_and_cleans_up() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, video) = fixture(ElementTag::Video, scale);
        engine.set_attribute(video, "controls", "").unwrap();
        engine
            .set_property(video, StyleProperty::Width, LengthValue::px(224.0).into())
            .unwrap();
        engine
            .set_property(video, StyleProperty::Height, LengthValue::px(160.0).into())
            .unwrap();
        for (direction, gutter, left, width) in [
            (Direction::Ltr, ScrollbarGutter::Stable, 26.0, 209.0),
            (Direction::Rtl, ScrollbarGutter::Stable, 41.0, 209.0),
            (
                Direction::Ltr,
                ScrollbarGutter::StableBothEdges,
                41.0,
                194.0,
            ),
        ] {
            engine
                .set_property(video, StyleProperty::Direction, direction.into())
                .unwrap();
            renderer(
                &mut engine,
                video,
                StyleProperty::ScrollbarGutter,
                RendererStyleValue::ScrollbarGutter(gutter),
            );
            let scene = engine.scene().unwrap();
            let mut compositor = SoftwareCompositor::default();
            let frame = compositor.render(&scene).unwrap();
            assert_eq!(pixel(&frame, left + 4.0, 30.0), [51, 51, 51, 255]);
            assert_eq!(pixel(&frame, left + width - 4.0, 30.0), [51, 51, 51, 255]);
            assert_eq!(pixel(&frame, left - 2.0, 30.0), WHITE);
            assert_eq!(pixel(&frame, left + width + 2.0, 30.0), WHITE);
            assert_eq!(compositor.render(&engine.scene().unwrap()).unwrap(), frame);
            assert_eq!(engine.scroll_offset(video).unwrap(), (0.0, 0.0));
        }
        engine.remove(video).unwrap();
        assert!(!engine.is_connected(video).unwrap());
        let frame = SoftwareCompositor::default()
            .render(&engine.scene().unwrap())
            .unwrap();
        assert!(frame.pixels.chunks_exact(4).all(|pixel| pixel == WHITE));
    }
}

fn overflow_engine(scale: f64) -> Engine {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap()).unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::BackgroundColor,
            Color::WHITE.into(),
        )
        .unwrap();
    engine
}

fn sized_node(engine: &mut Engine, tag: ElementTag, width: f32, height: f32) -> NodeHandle {
    let node = engine.create_native_element(tag).unwrap();
    for (property, size) in [
        (StyleProperty::Width, width),
        (StyleProperty::Height, height),
    ] {
        engine
            .set_property(node, property, LengthValue::px(size).into())
            .unwrap();
    }
    node
}

fn place_at(engine: &mut Engine, node: NodeHandle, left: f32, top: f32) {
    engine
        .set_property(node, StyleProperty::Position, Position::Absolute.into())
        .unwrap();
    engine
        .set_property(node, StyleProperty::Left, LengthValue::px(left).into())
        .unwrap();
    engine
        .set_property(node, StyleProperty::Top, LengthValue::px(top).into())
        .unwrap();
}

fn overflow_child(engine: &mut Engine, parent: NodeHandle) -> NodeHandle {
    let child = sized_node(engine, ElementTag::Div, 80.0, 80.0);
    engine
        .set_property(
            child,
            StyleProperty::BackgroundColor,
            Color::from_rgba8(0, 128, 128, 255).into(),
        )
        .unwrap();
    place_at(engine, child, 0.0, 0.0);
    engine.append_child(parent, child).unwrap();
    child
}

#[test]
fn real_tables_share_used_overflow_between_paint_and_native_hit_testing() {
    for scale in [1.0, 1.25, 2.0] {
        for collapse in [BorderCollapse::Separate, BorderCollapse::Collapse] {
            for with_caption in [false, true] {
                let mut engine = overflow_engine(scale);
                let table = sized_node(&mut engine, ElementTag::Table, 40.0, 40.0);
                place_at(&mut engine, table, 20.0, 20.0);
                renderer(
                    &mut engine,
                    table,
                    StyleProperty::BorderCollapse,
                    RendererStyleValue::BorderCollapse(collapse),
                );
                renderer(
                    &mut engine,
                    table,
                    StyleProperty::BorderSpacing,
                    RendererStyleValue::BorderSpacing((Length::px(0.0), Length::px(0.0))),
                );
                engine.append_child(engine.root(), table).unwrap();
                if with_caption {
                    let caption = sized_node(&mut engine, ElementTag::TableCaption, 40.0, 10.0);
                    engine.append_child(table, caption).unwrap();
                }
                let section = engine.create_native_element(ElementTag::TableBody).unwrap();
                let row = engine.create_native_element(ElementTag::TableRow).unwrap();
                let cell = sized_node(&mut engine, ElementTag::TableCell, 40.0, 40.0);
                engine
                    .set_property(cell, StyleProperty::Position, Position::Relative.into())
                    .unwrap();
                engine.append_child(table, section).unwrap();
                engine.append_child(section, row).unwrap();
                engine.append_child(row, cell).unwrap();
                let child = overflow_child(&mut engine, cell);
                for (x, y, right_visible, bottom_visible) in [
                    (Overflow::Auto, Overflow::Auto, true, true),
                    (Overflow::Scroll, Overflow::Scroll, true, true),
                    (Overflow::Hidden, Overflow::Visible, true, true),
                    (Overflow::Hidden, Overflow::Hidden, false, false),
                    (Overflow::Visible, Overflow::Clip, true, false),
                    (Overflow::Clip, Overflow::Visible, false, true),
                ] {
                    engine
                        .set_property(table, StyleProperty::OverflowX, x.into())
                        .unwrap();
                    engine
                        .set_property(table, StyleProperty::OverflowY, y.into())
                        .unwrap();
                    let bounds = engine.bounds(cell).unwrap().unwrap();
                    assert_eq!((bounds.width, bounds.height), (40.0, 40.0));
                    let scene = engine.scene().unwrap();
                    let frame = SoftwareCompositor::default().render(&scene).unwrap();
                    for (px, py, visible) in [
                        (bounds.x + 55.0, bounds.y + 15.0, right_visible),
                        (bounds.x + 15.0, bounds.y + 55.0, bottom_visible),
                    ] {
                        assert_eq!(
                            pixel(&frame, px, py),
                            if visible { TEAL } else { WHITE },
                            "table used overflow {x:?}/{y:?}, caption={with_caption}"
                        );
                        assert_eq!(
                            engine.hit_test(px, py).unwrap() == Some(child),
                            visible,
                            "native hits must share the painted axis policy"
                        );
                    }
                    assert_eq!(engine.scroll_offset(table).unwrap(), (0.0, 0.0));
                    assert_eq!(
                        SoftwareCompositor::default()
                            .render(&engine.scene().unwrap())
                            .unwrap(),
                        frame
                    );
                }
            }
        }
    }
}

#[test]
fn viewport_defining_html_and_body_do_not_acquire_a_second_local_overflow_clip() {
    for scale in [1.0, 1.25, 2.0] {
        let mut engine = overflow_engine(scale);
        let html = sized_node(&mut engine, ElementTag::Html, 40.0, 40.0);
        let body = sized_node(&mut engine, ElementTag::Body, 40.0, 40.0);
        engine.append_child(engine.root(), html).unwrap();
        engine.append_child(html, body).unwrap();
        engine
            .set_property(body, StyleProperty::Position, Position::Relative.into())
            .unwrap();
        let child = overflow_child(&mut engine, body);
        for (html_overflow, body_overflow, visible) in [
            (Overflow::Hidden, Overflow::Visible, true),
            (Overflow::Visible, Overflow::Hidden, true),
            (Overflow::Hidden, Overflow::Hidden, false),
        ] {
            engine
                .set_property(html, StyleProperty::Overflow, html_overflow.into())
                .unwrap();
            engine
                .set_property(body, StyleProperty::Overflow, body_overflow.into())
                .unwrap();
            let bounds = engine.bounds(body).unwrap().unwrap();
            let px = bounds.x + 55.0;
            let py = bounds.y + 15.0;
            let scene = engine.scene().unwrap();
            let frame = SoftwareCompositor::default().render(&scene).unwrap();
            assert_eq!(pixel(&frame, px, py), if visible { TEAL } else { WHITE });
            assert_eq!(engine.hit_test(px, py).unwrap() == Some(child), visible);
            assert_eq!(
                SoftwareCompositor::default()
                    .render(&engine.scene().unwrap())
                    .unwrap(),
                frame
            );
        }
    }
}

#[test]
fn containment_and_real_multicol_slices_keep_their_independent_clips() {
    for scale in [1.0, 1.25, 2.0] {
        let mut engine = overflow_engine(scale);
        let container = sized_node(&mut engine, ElementTag::Div, 40.0, 40.0);
        place_at(&mut engine, container, 20.0, 20.0);
        engine.append_child(engine.root(), container).unwrap();
        let child = overflow_child(&mut engine, container);
        renderer(
            &mut engine,
            container,
            StyleProperty::Contain,
            RendererStyleValue::Contain(Containment::PAINT),
        );
        let frame = SoftwareCompositor::default()
            .render(&engine.scene().unwrap())
            .unwrap();
        assert_eq!(pixel(&frame, 35.0, 35.0), TEAL);
        assert_eq!(pixel(&frame, 75.0, 35.0), WHITE);
        assert_ne!(engine.hit_test(75.0, 35.0).unwrap(), Some(child));
        engine.remove(container).unwrap();

        let columns = sized_node(&mut engine, ElementTag::Div, 100.0, 40.0);
        place_at(&mut engine, columns, 20.0, 20.0);
        engine.append_child(engine.root(), columns).unwrap();
        renderer(
            &mut engine,
            columns,
            StyleProperty::ColumnCount,
            RendererStyleValue::ColumnCount(Some(2)),
        );
        renderer(
            &mut engine,
            columns,
            StyleProperty::ColumnGap,
            RendererStyleValue::ColumnGap(Some(Length::px(20.0))),
        );
        let content = sized_node(&mut engine, ElementTag::Div, 40.0, 80.0);
        engine
            .set_property(
                content,
                StyleProperty::BackgroundColor,
                Color::from_rgba8(0, 128, 128, 255).into(),
            )
            .unwrap();
        engine.append_child(columns, content).unwrap();
        assert_eq!(
            engine.client_rects(content).unwrap().len(),
            2,
            "guard must exercise actual layout continuations"
        );
        let scene = engine.scene().unwrap();
        let frame = SoftwareCompositor::default().render(&scene).unwrap();
        assert_eq!(pixel(&frame, 40.0, 35.0), TEAL);
        assert_eq!(pixel(&frame, 100.0, 35.0), TEAL);
        assert_eq!(pixel(&frame, 70.0, 35.0), WHITE);
        assert_eq!(pixel(&frame, 40.0, 75.0), WHITE);
        assert_eq!(
            SoftwareCompositor::default()
                .render(&engine.scene().unwrap())
                .unwrap(),
            frame
        );
    }
}
