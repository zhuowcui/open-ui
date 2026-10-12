//! Native consumers of Chromium147 sticky constraints. No markup or script.
//! Source-only until compiled and executed with the full renderer gates.
use openui_compositor::SoftwareCompositor;
use openui_dom::ElementTag;
use openui_engine::{AnimationTimelineSource, Engine, NodeHandle};
use openui_geometry::{Length, ViewportMetrics};
use openui_style::{
    AnimationAttachmentRange, AnimationOptions, Border, BorderStyle, Color, ColumnFill, Display,
    Edges, FillMode, Keyframes, LengthValue, Overflow, Position, PropertyKeyframes,
    RendererStyleValue, StyleProperty, StyleValue, TimelineAxis,
};
fn set(
    engine: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    value: impl Into<StyleValue>,
) {
    engine.set_property(node, property, value.into()).unwrap();
}
fn box_(engine: &mut Engine, parent: NodeHandle, width: f32, height: f32) -> NodeHandle {
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    set(engine, node, StyleProperty::Display, Display::Block);
    set(engine, node, StyleProperty::Width, LengthValue::px(width));
    set(engine, node, StyleProperty::Height, LengthValue::px(height));
    engine.append_child(parent, node).unwrap();
    node
}
fn fixture(scale: f64) -> (Engine, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap()).unwrap();
    let root = engine.root();
    let source = box_(&mut engine, root, 100.0, 80.0);
    set(
        &mut engine,
        source,
        StyleProperty::Overflow,
        Overflow::Hidden,
    );
    (engine, source)
}
fn sticky(engine: &mut Engine, node: NodeHandle, top: f32) {
    set(engine, node, StyleProperty::Position, Position::Sticky);
    set(engine, node, StyleProperty::Top, LengthValue::px(top));
}
fn y(engine: &mut Engine, node: NodeHandle) -> f32 {
    engine.bounds(node).unwrap().unwrap().y
}
#[test]
fn bordered_padded_scrollport_keeps_its_origin_and_real_block_end_after_native_mutation() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source) = fixture(scale);
        set(
            &mut engine,
            source,
            StyleProperty::Border,
            Border {
                width: 10.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            },
        );
        set(
            &mut engine,
            source,
            StyleProperty::Padding,
            Edges::all(LengthValue::px(5.0)),
        );
        let block = box_(&mut engine, source, 100.0, 60.0);
        let subject = box_(&mut engine, block, 100.0, 20.0);
        sticky(&mut engine, subject, 0.0);
        box_(&mut engine, source, 100.0, 200.0);
        let origin = y(&mut engine, source);
        for (scroll, expected) in [
            (0.0, 15.0),
            (20.0, 15.0),
            (60.0, -5.0),
            (20.0, 15.0),
            (0.0, 15.0),
        ] {
            engine.scroll_to(source, 0.0, scroll).unwrap();
            assert_eq!(y(&mut engine, subject), origin + expected);
        }
        engine.scroll_to(source, 0.0, 20.0).unwrap();
        set(
            &mut engine,
            subject,
            StyleProperty::Position,
            Position::Relative,
        );
        assert_eq!(y(&mut engine, subject), origin - 5.0);
        sticky(&mut engine, subject, 0.0);
        assert_eq!(y(&mut engine, subject), origin + 15.0);
        let stats = engine.stats();
        assert_eq!(y(&mut engine, subject), origin + 15.0);
        assert_eq!(engine.stats(), stats);
    }
}
#[test]
fn sticky_containing_block_excludes_its_padding_and_the_targets_margins() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source) = fixture(scale);
        let block = box_(&mut engine, source, 100.0, 60.0);
        set(
            &mut engine,
            block,
            StyleProperty::PaddingTop,
            LengthValue::px(7.0),
        );
        set(
            &mut engine,
            block,
            StyleProperty::PaddingBottom,
            LengthValue::px(11.0),
        );
        let subject = box_(&mut engine, block, 100.0, 20.0);
        set(
            &mut engine,
            subject,
            StyleProperty::MarginTop,
            LengthValue::px(3.0),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::MarginBottom,
            LengthValue::px(4.0),
        );
        sticky(&mut engine, subject, 0.0);
        box_(&mut engine, source, 100.0, 200.0);
        let origin = y(&mut engine, source);
        for (scroll, expected) in [(0.0, 10.0), (20.0, 0.0), (80.0, -37.0), (0.0, 10.0)] {
            engine.scroll_to(source, 0.0, scroll).unwrap();
            assert_eq!(y(&mut engine, subject), origin + expected);
        }
    }
}
#[test]
fn nested_sticky_blocks_keep_containing_translation_separate_from_their_own_request() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source) = fixture(scale);
        let content = box_(&mut engine, source, 100.0, 500.0);
        box_(&mut engine, content, 100.0, 40.0);
        let parent = box_(&mut engine, content, 100.0, 100.0);
        sticky(&mut engine, parent, 0.0);
        box_(&mut engine, parent, 100.0, 20.0);
        let subject = box_(&mut engine, parent, 100.0, 20.0);
        sticky(&mut engine, subject, 40.0);
        let origin = y(&mut engine, source);
        for (scroll, parent_y, child_y) in
            [(0.0, 40.0, 60.0), (100.0, 0.0, 40.0), (0.0, 40.0, 60.0)]
        {
            engine.scroll_to(source, 0.0, scroll).unwrap();
            assert_eq!(y(&mut engine, parent), origin + parent_y);
            assert_eq!(y(&mut engine, subject), origin + child_y);
        }
    }
}
#[test]
fn anonymous_column_pieces_do_not_supply_smaller_sticky_containing_blocks() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source) = fixture(scale);
        set(
            &mut engine,
            source,
            StyleProperty::Width,
            LengthValue::px(500.0),
        );
        let content = box_(&mut engine, source, 200.0, 400.0);
        box_(&mut engine, content, 200.0, 120.0);
        let columns = box_(&mut engine, content, 200.0, 100.0);
        set(
            &mut engine,
            columns,
            StyleProperty::ColumnCount,
            StyleValue::Renderer(RendererStyleValue::ColumnCount(Some(2))),
        );
        set(
            &mut engine,
            columns,
            StyleProperty::ColumnFill,
            StyleValue::Renderer(RendererStyleValue::ColumnFill(ColumnFill::Auto)),
        );
        set(
            &mut engine,
            columns,
            StyleProperty::ColumnGap,
            StyleValue::Renderer(RendererStyleValue::ColumnGap(Some(Length::px(0.0)))),
        );
        let subject = box_(&mut engine, columns, 100.0, 180.0);
        sticky(&mut engine, subject, 0.0);
        let before = engine.client_rects(subject).unwrap();
        assert_eq!(before.len(), 2);
        for scroll in [150.0, 0.0, 150.0] {
            engine.scroll_to(source, 0.0, scroll).unwrap();
            let after = engine.client_rects(subject).unwrap();
            for (a, b) in after.iter().zip(&before) {
                assert_eq!(a.y, b.y - scroll as f32);
                assert_eq!((a.x, a.width, a.height), (b.x, b.width, b.height));
            }
        }
    }
}
#[test]
fn no_inset_sticky_queries_preserve_pixels_hits_and_the_unchanged_frame_path() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source) = fixture(scale);
        let subject = box_(&mut engine, source, 100.0, 20.0);
        set(
            &mut engine,
            subject,
            StyleProperty::BackgroundColor,
            Color::BLUE,
        );
        box_(&mut engine, source, 100.0, 200.0);
        set(
            &mut engine,
            subject,
            StyleProperty::Position,
            Position::Relative,
        );
        let mut compositor = SoftwareCompositor::default();
        let before = compositor.render(&engine.scene().unwrap()).unwrap();
        let rect = engine.bounds(subject).unwrap().unwrap();
        let hit = engine.hit_test(5.0, 5.0).unwrap();
        set(
            &mut engine,
            subject,
            StyleProperty::Position,
            Position::Sticky,
        );
        let after = compositor.render(&engine.scene().unwrap()).unwrap();
        assert_eq!(before.pixels, after.pixels);
        assert_eq!(engine.bounds(subject).unwrap().unwrap(), rect);
        assert_eq!(engine.hit_test(5.0, 5.0).unwrap(), hit);
        let stats = engine.stats();
        compositor.render(&engine.scene().unwrap()).unwrap();
        assert_eq!(engine.stats(), stats);
    }
}

#[test]
fn nested_sticky_view_snapshot_uses_pinned_first_sticky_constraints_and_normal_geometry() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source) = fixture(scale);
        let content = box_(&mut engine, source, 100.0, 500.0);
        box_(&mut engine, content, 100.0, 40.0);
        let parent = box_(&mut engine, content, 100.0, 100.0);
        sticky(&mut engine, parent, 0.0);
        box_(&mut engine, parent, 100.0, 20.0);
        let subject = box_(&mut engine, parent, 100.0, 20.0);
        sticky(&mut engine, subject, 40.0);
        let root = engine.root();
        let target = box_(&mut engine, root, 10.0, 10.0);
        let id = engine
            .animate_with_attachment_range(
                target,
                PropertyKeyframes::typed(
                    StyleProperty::Opacity,
                    Keyframes::from_values(1.0_f32, 1.0_f32),
                )
                .unwrap(),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::Both,
                    ..AnimationOptions::default()
                },
                AnimationTimelineSource::View {
                    subject,
                    axis: TimelineAxis::Y,
                },
                AnimationAttachmentRange::default(),
            )
            .unwrap();
        for scroll in [0.0, 100.0, 0.0] {
            engine.scroll_to(source, 0.0, scroll).unwrap();
            let before = engine.bounds(subject).unwrap().unwrap();
            let named = engine
                .animation_attachment_snapshot(id)
                .unwrap()
                .unwrap()
                .resolved
                .unwrap()
                .named;
            assert_eq!(named.cover.start, -20.0);
            assert_eq!(named.cover.end, 140.0);
            assert_eq!(engine.bounds(subject).unwrap().unwrap(), before);
        }
    }
}
