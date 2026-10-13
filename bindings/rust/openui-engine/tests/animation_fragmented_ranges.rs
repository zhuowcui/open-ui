//! Native retained layout owns fragmented View geometry; no HTML or script.
//! Source expectations are exact147 LayoutBox::ComputeSize and
//! BoxFragmentBuilder::UpdateBorderPaddingForClonedBoxDecorations.
use openui_compositor::SoftwareCompositor;
use openui_dom::ElementTag;
use openui_engine::{AnimationTimelineSource, Engine, NodeHandle};
use openui_geometry::{Length, ViewportMetrics};
use openui_style::{
    AnimationAttachmentRange, AnimationOptions, BorderStyle, BoxDecorationBreak, Color, ColumnFill,
    Display, FillMode, Keyframes, LengthValue, LineHeight, Overflow, PropertyKeyframes,
    RendererStyleValue, StyleProperty, StyleValue, TimelineAxis, VerticalAlign, WritingMode,
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
fn fixture(scale: f64) -> (Engine, NodeHandle, NodeHandle, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap()).unwrap();
    let root = engine.root();
    let source = box_(&mut engine, root, 500.0, 80.0);
    set(
        &mut engine,
        source,
        StyleProperty::Overflow,
        Overflow::Hidden,
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
    set(
        &mut engine,
        subject,
        StyleProperty::Width,
        LengthValue::auto(),
    );
    let target = box_(&mut engine, root, 10.0, 10.0);
    (engine, source, subject, target)
}
fn timeline(
    engine: &mut Engine,
    subject: NodeHandle,
    target: NodeHandle,
) -> openui_engine::AnimationId {
    engine
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
        .unwrap()
}
#[test]
fn sliced_and_cloned_principal_boxes_include_their_actual_continuation_edges() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, subject, target) = fixture(scale);
        set(
            &mut engine,
            subject,
            StyleProperty::BorderTopStyle,
            StyleValue::Renderer(RendererStyleValue::BorderTopStyle(BorderStyle::Solid)),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::BorderBottomStyle,
            StyleValue::Renderer(RendererStyleValue::BorderBottomStyle(BorderStyle::Solid)),
        );
        set(&mut engine, subject, StyleProperty::BorderTopWidth, 2_i32);
        set(
            &mut engine,
            subject,
            StyleProperty::BorderBottomWidth,
            2_i32,
        );
        let id = timeline(&mut engine, subject, target);
        for (decoration, height) in [
            (BoxDecorationBreak::Slice, 184.0),
            (BoxDecorationBreak::Clone, 188.0),
            (BoxDecorationBreak::Slice, 184.0),
        ] {
            set(
                &mut engine,
                subject,
                StyleProperty::BoxDecorationBreak,
                StyleValue::Renderer(RendererStyleValue::BoxDecorationBreak(decoration)),
            );
            let bounds = engine.bounds(subject).unwrap().unwrap();
            let rects = engine.client_rects(subject).unwrap();
            assert!(rects.len() >= 2);
            let owned = engine
                .animation_attachment_snapshot(id)
                .unwrap()
                .unwrap()
                .resolved
                .unwrap();
            assert_eq!(owned.named.cover.start, 40.0);
            assert_eq!(owned.named.cover.end, 120.0 + height);
            // Timeline queries do not replace the element's real continued boxes.
            assert_eq!(engine.bounds(subject).unwrap().unwrap(), bounds);
            assert_eq!(engine.client_rects(subject).unwrap(), rects);
            for scroll in [0.0, 50.0, 150.0, 0.0] {
                engine.scroll_to(source, 0.0, scroll).unwrap();
                assert_eq!(
                    engine
                        .animation_attachment_snapshot(id)
                        .unwrap()
                        .unwrap()
                        .resolved
                        .unwrap(),
                    owned
                );
            }
        }
    }
}
#[test]
fn overflow_only_continuations_do_not_extend_a_definite_subject_box() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, subject, target) = fixture(scale);
        set(
            &mut engine,
            subject,
            StyleProperty::Height,
            LengthValue::px(40.0),
        );
        box_(&mut engine, subject, 100.0, 180.0);
        let id = timeline(&mut engine, subject, target);
        let owned = engine
            .animation_attachment_snapshot(id)
            .unwrap()
            .unwrap()
            .resolved
            .unwrap();
        assert_eq!(owned.named.cover.start, 40.0);
        assert_eq!(owned.named.cover.end, 160.0);
        for scroll in [0.0, 70.0, 150.0, 0.0] {
            engine.scroll_to(source, 0.0, scroll).unwrap();
            assert_eq!(
                engine
                    .animation_attachment_snapshot(id)
                    .unwrap()
                    .unwrap()
                    .resolved
                    .unwrap(),
                owned
            );
        }
    }
}

#[test]
fn timeline_geometry_queries_preserve_static_scene_pixels_and_native_hits() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, source, subject, target) = fixture(scale);
        set(
            &mut engine,
            subject,
            StyleProperty::BackgroundColor,
            Color::BLUE,
        );
        set(
            &mut engine,
            subject,
            StyleProperty::BoxDecorationBreak,
            StyleValue::Renderer(RendererStyleValue::BoxDecorationBreak(
                BoxDecorationBreak::Clone,
            )),
        );
        engine.scroll_to(source, 0.0, 80.0).unwrap();
        let mut compositor = SoftwareCompositor::default();
        let before = compositor.render(&engine.scene().unwrap()).unwrap();
        assert_eq!(engine.hit_test(5.0, 45.0).unwrap(), Some(subject));
        let id = timeline(&mut engine, subject, target);
        let resolved = engine
            .animation_attachment_snapshot(id)
            .unwrap()
            .unwrap()
            .resolved
            .unwrap();
        assert_eq!(resolved.named.cover.end, 300.0);
        let after = compositor.render(&engine.scene().unwrap()).unwrap();
        assert_eq!(after.pixels, before.pixels);
        assert_eq!(engine.hit_test(5.0, 45.0).unwrap(), Some(subject));
    }
}
#[test]
fn vertical_block_fragmentation_keeps_logical_consumption_and_physical_stitched_size() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap()).unwrap();
        let root = engine.root();
        let source = box_(&mut engine, root, 80.0, 500.0);
        set(
            &mut engine,
            source,
            StyleProperty::WritingMode,
            WritingMode::VerticalLr,
        );
        set(
            &mut engine,
            source,
            StyleProperty::Overflow,
            Overflow::Hidden,
        );
        let content = box_(&mut engine, source, 400.0, 200.0);
        box_(&mut engine, content, 120.0, 200.0);
        let columns = box_(&mut engine, content, 100.0, 200.0);
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
        let subject = box_(&mut engine, columns, 180.0, 100.0);
        set(
            &mut engine,
            subject,
            StyleProperty::Height,
            LengthValue::auto(),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::BorderLeftStyle,
            StyleValue::Renderer(RendererStyleValue::BorderLeftStyle(BorderStyle::Solid)),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::BorderRightStyle,
            StyleValue::Renderer(RendererStyleValue::BorderRightStyle(BorderStyle::Solid)),
        );
        set(&mut engine, subject, StyleProperty::BorderLeftWidth, 2_i32);
        set(&mut engine, subject, StyleProperty::BorderRightWidth, 2_i32);
        set(
            &mut engine,
            subject,
            StyleProperty::BoxDecorationBreak,
            StyleValue::Renderer(RendererStyleValue::BoxDecorationBreak(
                BoxDecorationBreak::Clone,
            )),
        );
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
                    axis: TimelineAxis::Block,
                },
                AnimationAttachmentRange::default(),
            )
            .unwrap();
        let owned = engine
            .animation_attachment_snapshot(id)
            .unwrap()
            .unwrap()
            .resolved
            .unwrap();
        assert_eq!(owned.named.cover.start, 40.0);
        assert_eq!(owned.named.cover.end, 308.0);
        for offset in [0.0, 50.0, 150.0, 0.0] {
            engine.scroll_to(source, offset, 0.0).unwrap();
            assert_eq!(
                engine
                    .animation_attachment_snapshot(id)
                    .unwrap()
                    .unwrap()
                    .resolved
                    .unwrap(),
                owned
            );
        }
    }
}

#[test]
fn inline_formatting_principal_consumption_includes_fragmentainer_space_after_lines() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, _, subject, target) = fixture(scale);
        let columns = engine.parent(subject).unwrap().unwrap();
        set(
            &mut engine,
            columns,
            StyleProperty::ColumnCount,
            StyleValue::Renderer(RendererStyleValue::ColumnCount(None)),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Width,
            LengthValue::px(100.0),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Height,
            LengthValue::auto(),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Orphans,
            StyleValue::Renderer(RendererStyleValue::Orphans(1)),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::Widows,
            StyleValue::Renderer(RendererStyleValue::Widows(1)),
        );
        set(
            &mut engine,
            subject,
            StyleProperty::LineHeight,
            StyleValue::Renderer(RendererStyleValue::LineHeight(LineHeight::Length(60.0))),
        );
        for _ in 0..3 {
            let child = box_(&mut engine, subject, 90.0, 60.0);
            set(
                &mut engine,
                child,
                StyleProperty::Display,
                Display::InlineBlock,
            );
            set(
                &mut engine,
                child,
                StyleProperty::VerticalAlign,
                StyleValue::Renderer(RendererStyleValue::VerticalAlign(VerticalAlign::Top)),
            );
        }
        let id = timeline(&mut engine, subject, target);
        let unfragmented = engine
            .animation_attachment_snapshot(id)
            .unwrap()
            .unwrap()
            .resolved
            .unwrap();
        assert_eq!(unfragmented.named.cover.start, 40.0);
        assert_eq!(unfragmented.named.cover.end, 300.0);
        set(
            &mut engine,
            columns,
            StyleProperty::ColumnCount,
            StyleValue::Renderer(RendererStyleValue::ColumnCount(Some(2))),
        );
        for (fragmentainer_height, stitched_height) in
            [(100.0, 260.0), (90.0, 240.0), (100.0, 260.0)]
        {
            set(
                &mut engine,
                columns,
                StyleProperty::Height,
                LengthValue::px(fragmentainer_height),
            );
            let fragmented = engine
                .animation_attachment_snapshot(id)
                .unwrap()
                .unwrap()
                .resolved
                .unwrap();
            let rects = engine.client_rects(subject).unwrap();
            assert_eq!(rects.len(), 3);
            assert_eq!(f64::from(rects[0].height), f64::from(fragmentainer_height));
            assert_eq!(f64::from(rects[1].height), f64::from(fragmentainer_height));
            assert_eq!(f64::from(rects[2].height), 60.0);
            assert_eq!(fragmented.named.cover.start, unfragmented.named.cover.start);
            assert_eq!(fragmented.named.cover.end, 120.0 + stitched_height);
            assert_ne!(fragmented.named.cover.end, unfragmented.named.cover.end);
        }
    }
}

#[test]
fn vertical_clone_border_consumption_is_block_relative() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for mode in [WritingMode::VerticalLr, WritingMode::VerticalRl] {
            let mut e =
                Engine::new(ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap())
                    .unwrap();
            let root = e.root();
            let source = box_(&mut e, root, 80.0, 500.0);
            set(&mut e, source, StyleProperty::WritingMode, mode);
            set(&mut e, source, StyleProperty::Overflow, Overflow::Hidden);
            let columns = box_(&mut e, source, 100.0, 200.0);
            set(
                &mut e,
                columns,
                StyleProperty::ColumnCount,
                StyleValue::Renderer(RendererStyleValue::ColumnCount(Some(2))),
            );
            set(
                &mut e,
                columns,
                StyleProperty::ColumnFill,
                StyleValue::Renderer(RendererStyleValue::ColumnFill(ColumnFill::Auto)),
            );
            set(
                &mut e,
                columns,
                StyleProperty::ColumnGap,
                StyleValue::Renderer(RendererStyleValue::ColumnGap(Some(Length::px(0.0)))),
            );
            let subject = box_(&mut e, columns, 180.0, 100.0);
            set(&mut e, subject, StyleProperty::Height, LengthValue::auto());
            set(
                &mut e,
                subject,
                StyleProperty::BorderLeftStyle,
                StyleValue::Renderer(RendererStyleValue::BorderLeftStyle(BorderStyle::Solid)),
            );
            set(
                &mut e,
                subject,
                StyleProperty::BorderRightStyle,
                StyleValue::Renderer(RendererStyleValue::BorderRightStyle(BorderStyle::Solid)),
            );
            set(&mut e, subject, StyleProperty::BorderLeftWidth, 2_i32);
            set(&mut e, subject, StyleProperty::BorderRightWidth, 2_i32);
            let target = box_(&mut e, root, 10.0, 10.0);
            let id = e
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
                        axis: TimelineAxis::Block,
                    },
                    AnimationAttachmentRange::default(),
                )
                .unwrap();
            for (decoration, extent) in [
                (BoxDecorationBreak::Slice, 184.0),
                (BoxDecorationBreak::Clone, 188.0),
                (BoxDecorationBreak::Slice, 184.0),
            ] {
                set(
                    &mut e,
                    subject,
                    StyleProperty::BoxDecorationBreak,
                    StyleValue::Renderer(RendererStyleValue::BoxDecorationBreak(decoration)),
                );
                let resolved = e
                    .animation_attachment_snapshot(id)
                    .unwrap()
                    .unwrap()
                    .resolved
                    .unwrap();
                // Cover span removes source-direction/origin variation and exposes
                // only the real stitched block extent plus the client block extent.
                assert_eq!(
                    resolved.named.cover.end - resolved.named.cover.start,
                    extent + 80.0
                );
                assert!(e.client_rects(subject).unwrap().len() >= 2);
                let scene = e.scene().unwrap();
                let stats = e.stats();
                assert_eq!(e.scene().unwrap().generation(), scene.generation());
                assert_eq!(e.stats(), stats);
            }
            // Physical top/bottom edges are inline decoration here. They
            // must not become cloned block decoration merely because the
            // source style uses physical edge names.
            set(
                &mut e,
                subject,
                StyleProperty::BorderLeftStyle,
                StyleValue::Renderer(RendererStyleValue::BorderLeftStyle(BorderStyle::None)),
            );
            set(
                &mut e,
                subject,
                StyleProperty::BorderRightStyle,
                StyleValue::Renderer(RendererStyleValue::BorderRightStyle(BorderStyle::None)),
            );
            set(
                &mut e,
                subject,
                StyleProperty::BorderTopStyle,
                StyleValue::Renderer(RendererStyleValue::BorderTopStyle(BorderStyle::Solid)),
            );
            set(
                &mut e,
                subject,
                StyleProperty::BorderBottomStyle,
                StyleValue::Renderer(RendererStyleValue::BorderBottomStyle(BorderStyle::Solid)),
            );
            set(&mut e, subject, StyleProperty::BorderTopWidth, 2_i32);
            set(&mut e, subject, StyleProperty::BorderBottomWidth, 2_i32);
            for decoration in [BoxDecorationBreak::Slice, BoxDecorationBreak::Clone] {
                set(
                    &mut e,
                    subject,
                    StyleProperty::BoxDecorationBreak,
                    StyleValue::Renderer(RendererStyleValue::BoxDecorationBreak(decoration)),
                );
                let resolved = e
                    .animation_attachment_snapshot(id)
                    .unwrap()
                    .unwrap()
                    .resolved
                    .unwrap();
                assert_eq!(resolved.named.cover.end - resolved.named.cover.start, 260.0);
            }
        }
    }
}
