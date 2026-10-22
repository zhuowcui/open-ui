//! Actual source-sliced nested multicol decoration ownership and retained overflow.
//! These are native API/paint guards, not new Chromium reference pixels.
use openui_compositor::{Frame, SceneSnapshot, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineOptions, NodeHandle, ViewportMetrics};
use openui_geometry::{LayoutUnit, RasterConfiguration};
use openui_layout::{Fragment, FragmentKind};
use openui_style::*;

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn set(e: &mut Engine, n: NodeHandle, v: RendererStyleValue) {
    e.set_renderer_style(n, v).unwrap();
}
fn element(e: &mut Engine, parent: NodeHandle) -> NodeHandle {
    let n = e.create_native_element(ElementTag::Div).unwrap();
    e.append_child(parent, n).unwrap();
    for v in [
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
        set(e, n, v);
    }
    n
}
fn fixture(scale: f64) -> (Engine, NodeHandle) {
    let mut e = Engine::new_with_font_collection_and_options(
        ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap(),
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
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
    );
    set(
        &mut e,
        root,
        RendererStyleValue::BackgroundColor(Color::WHITE),
    );
    let body = element(&mut e, root);
    for v in [
        RendererStyleValue::PaddingTop(Length::px(20.0)),
        RendererStyleValue::PaddingRight(Length::px(20.0)),
        RendererStyleValue::PaddingBottom(Length::px(20.0)),
        RendererStyleValue::PaddingLeft(Length::px(20.0)),
    ] {
        set(&mut e, body, v);
    }
    let outer = element(&mut e, body);
    for v in [
        RendererStyleValue::Width(Length::px(400.0)),
        RendererStyleValue::Height(Length::px(110.0)),
        RendererStyleValue::ColumnCount(Some(2)),
        RendererStyleValue::ColumnRuleWidth(6),
        RendererStyleValue::ColumnRuleStyle(BorderStyle::Solid),
    ] {
        set(&mut e, outer, v);
    }
    let inner = element(&mut e, outer);
    for v in [
        RendererStyleValue::Height(Length::px(270.0)),
        RendererStyleValue::ColumnCount(Some(2)),
        RendererStyleValue::ColumnRuleWidth(3),
        RendererStyleValue::ColumnRuleStyle(BorderStyle::Solid),
        RendererStyleValue::BackgroundColor(Color::from_rgba8(144, 238, 144, 255)),
        RendererStyleValue::BorderTopWidth(10),
        RendererStyleValue::BorderRightWidth(10),
        RendererStyleValue::BorderBottomWidth(10),
        RendererStyleValue::BorderLeftWidth(10),
        RendererStyleValue::BorderTopStyle(BorderStyle::Solid),
        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
        RendererStyleValue::BorderLeftStyle(BorderStyle::Solid),
    ] {
        set(&mut e, inner, v);
    }
    let before = element(&mut e, inner);
    set(
        &mut e,
        before,
        RendererStyleValue::Height(Length::px(200.0)),
    );
    set(
        &mut e,
        before,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 255, 0, 255)),
    );
    let span = element(&mut e, inner);
    set(
        &mut e,
        span,
        RendererStyleValue::ColumnSpan(ColumnSpan::All),
    );
    set(&mut e, span, RendererStyleValue::Height(Length::px(50.0)));
    set(
        &mut e,
        span,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(173, 216, 230, 255)),
    );
    let after = element(&mut e, inner);
    set(&mut e, after, RendererStyleValue::Height(Length::px(240.0)));
    set(
        &mut e,
        after,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 255, 0, 255)),
    );
    (e, span)
}
fn raster(scene: &SceneSnapshot) -> Frame {
    SoftwareCompositor::default().render(scene).unwrap()
}
fn projections<'a>(
    fragment: &'a Fragment,
    e: &Engine,
    parent: Option<&'a Fragment>,
    out: &mut Vec<(&'a Fragment, &'a Fragment)>,
) {
    if fragment.kind == FragmentKind::Box
        && fragment.owns_element_box()
        && !fragment.node_id.is_none()
        && e.native_document().node(fragment.node_id).style.column_span == ColumnSpan::All
    {
        out.push((parent.expect("retained direct spanner owner"), fragment));
    }
    for child in &fragment.children {
        projections(child, e, Some(fragment), out);
    }
}
fn observed(e: &mut Engine) -> (Frame, Vec<(i32, i32, i32, bool, usize)>) {
    e.set_layered_paint_capture_enabled(false);
    let ordinary = e.scene().unwrap();
    let frame = raster(&ordinary);
    let mut spans = Vec::new();
    projections(ordinary.fragments(), e, None, &mut spans);
    assert!(
        !spans.is_empty(),
        "actual retained source spanners required"
    );
    let values = spans
        .iter()
        .map(|(parent, span)| {
            let slice = parent
                .decoration_slice
                .expect("actual ancestor source slice");
            assert_eq!(span.decoration_slice, None);
            assert_eq!(
                span.fragmentation_visual_offset,
                openui_geometry::PhysicalOffset::zero()
            );
            (
                slice.source_block_offset.raw(),
                span.offset.top.raw(),
                span.size.height.raw(),
                span.skip_box_decoration,
                span.children.len(),
            )
        })
        .collect();
    e.set_layered_paint_capture_enabled(true);
    let captured = e.scene().unwrap();
    let instrumented = raster(&captured);
    assert_eq!(
        (frame.width, frame.height, frame.stride),
        (instrumented.width, instrumented.height, instrumented.stride)
    );
    assert!(
        frame.pixels == instrumented.pixels,
        "capture changed complete canonical frame"
    );
    let stats = e.stats();
    let generation = captured.generation();
    assert_eq!(e.scene().unwrap().generation(), generation);
    assert_eq!(e.stats(), stats);
    (frame, values)
}

fn probe(e: &mut Engine, span: NodeHandle, zero: bool) -> NodeHandle {
    if zero {
        set(e, span, RendererStyleValue::Height(Length::px(0.0)));
    }
    let probe = element(e, span);
    set(e, probe, RendererStyleValue::Width(Length::px(20.0)));
    set(e, probe, RendererStyleValue::Height(Length::px(20.0)));
    set(
        e,
        probe,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 0, 255, 255)),
    );
    if !zero {
        set(
            e,
            probe,
            RendererStyleValue::Transform(Transform2D {
                f: -20.0,
                ..Transform2D::IDENTITY
            }),
        );
    }
    probe
}

fn owned_layer<'a>(
    scene: &'a SceneSnapshot,
    handle: NodeHandle,
) -> &'a openui_compositor::SceneLayerSnapshot {
    scene
        .retained_scene()
        .unwrap()
        .layers()
        .iter()
        .find(|layer| {
            layer.owner.document == handle.document_id()
                && layer.owner.index == handle.index()
                && layer.owner.generation == handle.generation()
        })
        .expect("real authored layer")
}

fn at_path<'a>(root: &'a Fragment, path: &[usize]) -> &'a Fragment {
    path.iter()
        .fold(root, |fragment, index| &fragment.children[*index])
}

fn node_copies<'a>(root: &'a Fragment, node: openui_dom::NodeId, out: &mut Vec<&'a Fragment>) {
    if root.node_id == node && root.owns_element_box() {
        out.push(root);
    }
    for child in &root.children {
        node_copies(child, node, out);
    }
}

#[test]
fn raw_carriers_keep_source_geometry_but_publish_one_authored_box() {
    for scale in SCALES {
        for zero in [false, true] {
            let (mut e, span) = fixture(scale);
            let probe = probe(&mut e, span, zero);
            let (_, values) = observed(&mut e);
            assert_eq!(values.len(), 3, "scale={scale} zero={zero}");
            assert!(values.iter().all(|value| value.4 == 1));
            if zero {
                assert!(values.iter().all(|value| value.2 == 0 && !value.3));
            }
            let scene = e.scene().unwrap();
            let layer = owned_layer(&scene, probe);
            assert_eq!(layer.projections.len(), 1, "scale={scale} zero={zero}");
            let owned = at_path(scene.fragments(), &layer.projections[0].fragment_path);
            let mut copies = Vec::new();
            node_copies(scene.fragments(), owned.node_id, &mut copies);
            assert_eq!(copies.len(), 3);
            assert_eq!(
                copies
                    .iter()
                    .map(|copy| copy.is_authored_projection_owner())
                    .collect::<Vec<_>>(),
                [false, true, false]
            );
            assert!(copies.iter().all(|copy| copy
                .box_geometry
                .is_some_and(|geometry| !geometry.continuation_record
                    && !geometry.is_repeated
                    && geometry.consumed_block_size == LayoutUnit::zero())));
            assert!(copies.iter().all(|copy| copy
                .authored_projection
                .as_ref()
                .unwrap()
                .is_materialized_copy()));
            assert_eq!(
                copies[1]
                    .authored_projection
                    .as_ref()
                    .unwrap()
                    .source_point(),
                Some(LayoutUnit::from_i32(110))
            );
            assert_eq!(
                e.client_rects(span).unwrap(),
                [openui_compositor::SceneRect {
                    x: 238.0,
                    y: 20.0,
                    width: 172.0,
                    height: if zero { 0.0 } else { 50.0 },
                }]
            );
            let probe_rect = openui_compositor::SceneRect {
                x: 238.0,
                y: if zero { 20.0 } else { 0.0 },
                width: 20.0,
                height: 20.0,
            };
            assert_eq!(e.client_rects(probe).unwrap(), [probe_rect]);
            assert_eq!(e.bounds(probe).unwrap(), Some(probe_rect));
            let stats = e.stats();
            let generation = scene.generation();
            assert_eq!(e.scene().unwrap().generation(), generation);
            assert_eq!(e.stats(), stats);
        }
    }
}

#[test]
fn owning_transform_input_and_capture_cross_only_source_fragmentation_scope() {
    for scale in SCALES {
        let (mut e, span) = fixture(scale);
        let probe = probe(&mut e, span, false);
        let (_, values) = observed(&mut e);
        assert_eq!(
            values.iter().map(|value| value.3).collect::<Vec<_>>(),
            [true, false, true]
        );
        assert_eq!(e.hit_test(248.0, 10.0).unwrap(), Some(probe));
        assert_ne!(e.hit_test(40.0, 120.0).unwrap(), Some(probe));
        let scene = e.scene().unwrap();
        assert_eq!(owned_layer(&scene, probe).projections.len(), 1);
        let stats = e.stats();
        let generation = scene.generation();
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
    }
}

#[test]
fn zero_source_point_keeps_child_and_authored_transform_removal_restores_frame() {
    for scale in SCALES {
        let (mut e, span) = fixture(scale);
        let probe = probe(&mut e, span, true);
        let (original, original_values) = observed(&mut e);
        assert!(original_values
            .iter()
            .all(|value| value.2 == 0 && !value.3 && value.4 == 1));
        let original_rects = e.client_rects(probe).unwrap();
        e.set_property(
            probe,
            StyleProperty::Transform,
            StyleValue::Transform(TransformList(vec![TransformOperation::Matrix(
                Transform2D {
                    f: -20.0,
                    ..Transform2D::IDENTITY
                },
            )])),
        )
        .unwrap();
        let (_, changed_values) = observed(&mut e);
        assert_eq!(changed_values, original_values);
        assert_eq!(
            e.client_rects(probe).unwrap(),
            [openui_compositor::SceneRect {
                x: 238.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            }]
        );
        // This is source/geometry restoration, not an unmeasured zero+transform
        // Chromium visibility expectation. The new Chrome10 gate remains open.
        assert!(e.remove_property(probe, StyleProperty::Transform).unwrap());
        let (restored, restored_values) = observed(&mut e);
        assert_eq!(restored_values, original_values);
        assert_eq!(e.client_rects(probe).unwrap(), original_rects);
        assert_eq!(
            (restored.width, restored.height, restored.stride),
            (original.width, original.height, original.stride)
        );
        assert!(
            restored.pixels == original.pixels,
            "scale={scale} complete restored frame"
        );
        let scene = e.scene().unwrap();
        let layer = owned_layer(&scene, probe);
        let fragment = at_path(scene.fragments(), &layer.projections[0].fragment_path);
        assert!(!e
            .native_document()
            .node(fragment.node_id)
            .style
            .has_transform());
    }
}

#[test]
fn independent_effect_clone_containment_and_multiple_source_pieces_keep_old_roles() {
    for scale in SCALES {
        for neighbor in 0..5 {
            let (mut e, span) = fixture(scale);
            let probe = probe(&mut e, span, true);
            match neighbor {
                0 => set(&mut e, span, RendererStyleValue::Opacity(0.5)),
                1 => set(
                    &mut e,
                    span,
                    RendererStyleValue::BoxDecorationBreak(BoxDecorationBreak::Clone),
                ),
                2 => set(&mut e, span, RendererStyleValue::Height(Length::px(330.0))),
                3 | 4 => e
                    .set_property(
                        span,
                        StyleProperty::Contain,
                        StyleValue::Renderer(RendererStyleValue::Contain(if neighbor == 3 {
                            Containment::LAYOUT
                        } else {
                            Containment::SIZE
                        })),
                    )
                    .unwrap(),
                _ => unreachable!(),
            }
            let (_, values) = observed(&mut e);
            if neighbor == 3 || neighbor == 4 {
                assert_eq!(
                    e.computed_style(span).unwrap().contain,
                    if neighbor == 3 {
                        Containment::LAYOUT
                    } else {
                        Containment::SIZE
                    },
                );
            }
            assert!(values.iter().all(|value| value.4 == 1));
            let scene = e.scene().unwrap();
            let layer = owned_layer(&scene, probe);
            let node = at_path(scene.fragments(), &layer.projections[0].fragment_path).node_id;
            let mut copies = Vec::new();
            node_copies(scene.fragments(), node, &mut copies);
            assert!(
                copies
                    .iter()
                    .all(|copy| copy.is_authored_projection_owner()),
                "scale={scale} neighbor={neighbor}"
            );
            assert!(copies.iter().all(|copy| !copy
                .authored_projection
                .as_ref()
                .is_some_and(|p| p.is_materialized_copy())));
            assert_eq!(e.client_rects(probe).unwrap().len(), copies.len());
            let stats = e.stats();
            let generation = scene.generation();
            assert_eq!(e.scene().unwrap().generation(), generation);
            assert_eq!(e.stats(), stats);
        }
    }
}

#[test]
fn real_css_clip_and_fractional_ancestor_transform_keep_source_owner_and_restore() {
    for scale in SCALES {
        let (mut e, span) = fixture(scale);
        let probe = probe(&mut e, span, false);
        let inner = e.parent(span).unwrap().unwrap();
        let outer = e.parent(inner).unwrap().unwrap();
        let (original, original_values) = observed(&mut e);
        assert_eq!(e.hit_test(248.0, 10.0).unwrap(), Some(probe));
        for property in [StyleProperty::OverflowX, StyleProperty::OverflowY] {
            e.set_property(outer, property, parse_literal(property, "hidden").unwrap())
                .unwrap();
        }
        let (_, clipped_values) = observed(&mut e);
        assert_eq!(clipped_values, original_values);
        assert_eq!(
            e.client_rects(probe).unwrap(),
            [openui_compositor::SceneRect {
                x: 238.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            }]
        );
        assert_ne!(
            e.hit_test(248.0, 10.0).unwrap(),
            Some(probe),
            "real CSS clip was crossed"
        );
        for property in [StyleProperty::OverflowX, StyleProperty::OverflowY] {
            assert!(e.remove_property(outer, property).unwrap());
        }
        e.set_property(
            outer,
            StyleProperty::Transform,
            StyleValue::Transform(TransformList(vec![TransformOperation::Matrix(
                Transform2D {
                    e: 0.25,
                    f: 0.5,
                    ..Transform2D::IDENTITY
                },
            )])),
        )
        .unwrap();
        let (_, moved_values) = observed(&mut e);
        assert_eq!(moved_values, original_values);
        assert_eq!(
            e.client_rects(probe).unwrap(),
            [openui_compositor::SceneRect {
                x: 238.25,
                y: 0.5,
                width: 20.0,
                height: 20.0,
            }]
        );
        assert_eq!(e.hit_test(248.25, 10.5).unwrap(), Some(probe));
        assert!(e.remove_property(outer, StyleProperty::Transform).unwrap());
        let (restored, restored_values) = observed(&mut e);
        assert_eq!(restored_values, original_values);
        assert_eq!(
            (restored.width, restored.height, restored.stride),
            (original.width, original.height, original.stride)
        );
        assert!(
            restored.pixels == original.pixels,
            "scale={scale} restored real CSS ancestor state"
        );
    }
}

#[test]
fn source_policy_transition_returns_to_producer_and_restores_complete_frame() {
    for scale in SCALES {
        let (mut e, span) = fixture(scale);
        let probe = probe(&mut e, span, false);
        let (original, original_values) = observed(&mut e);
        let original_rects = e.client_rects(probe).unwrap();
        assert_eq!(original_rects.len(), 1);

        e.set_property(
            span,
            StyleProperty::Opacity,
            parse_literal(StyleProperty::Opacity, "0.5").unwrap(),
        )
        .unwrap();
        let (_, fallback_values) = observed(&mut e);
        // Opacity declines source-window decoration suppression; retained
        // source operands and child topology must still remain exact.
        assert_eq!(
            fallback_values
                .iter()
                .map(|value| (value.0, value.1, value.2, value.4))
                .collect::<Vec<_>>(),
            original_values
                .iter()
                .map(|value| (value.0, value.1, value.2, value.4))
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            original_values
                .iter()
                .map(|value| value.3)
                .collect::<Vec<_>>(),
            vec![true, false, true],
        );
        assert!(fallback_values.iter().all(|value| !value.3));
        let fallback_scene = e.scene().unwrap();
        let fallback_layer = owned_layer(&fallback_scene, probe);
        let node = at_path(
            fallback_scene.fragments(),
            &fallback_layer.projections[0].fragment_path,
        )
        .node_id;
        let mut fallback_copies = Vec::new();
        node_copies(fallback_scene.fragments(), node, &mut fallback_copies);
        assert_eq!(fallback_copies.len(), 3);
        assert!(fallback_copies.iter().all(|copy| {
            copy.is_authored_projection_owner()
                && !copy
                    .authored_projection
                    .as_ref()
                    .is_some_and(|record| record.is_materialized_copy())
        }));
        assert_eq!(e.client_rects(probe).unwrap().len(), fallback_copies.len());

        assert!(e.remove_property(span, StyleProperty::Opacity).unwrap());
        let (restored, restored_values) = observed(&mut e);
        assert_eq!(restored_values, original_values);
        assert_eq!(e.client_rects(probe).unwrap(), original_rects);
        assert_eq!(
            (restored.width, restored.height, restored.stride),
            (original.width, original.height, original.stride)
        );
        assert!(
            restored.pixels == original.pixels,
            "scale={scale} complete source-policy restoration"
        );
        let restored_scene = e.scene().unwrap();
        assert_eq!(owned_layer(&restored_scene, probe).projections.len(), 1);
    }
}
