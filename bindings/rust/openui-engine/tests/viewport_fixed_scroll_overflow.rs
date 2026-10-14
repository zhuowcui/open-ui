//! Actual viewport-fixed ownership and scroll geometry; no renderer selection.
use openui_compositor::SceneSnapshot;
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_paint::{PaintLayerNodeIdentity, SourcePositioningReference};
use openui_style::{
    parse_literal, Color, Direction, Display, Edges, LengthValue, Overflow, Position,
    StyleProperty, StyleValue,
};

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
fn set(e: &mut Engine, n: NodeHandle, p: StyleProperty, value: impl Into<StyleValue>) {
    e.set_property(n, p, value.into()).unwrap();
}
fn box_(e: &mut Engine, parent: NodeHandle, width: f32, height: f32) -> NodeHandle {
    let n = e.create_native_element(ElementTag::Div).unwrap();
    e.append_child(parent, n).unwrap();
    set(e, n, StyleProperty::Display, Display::Block);
    set(e, n, StyleProperty::Width, LengthValue::px(width));
    set(e, n, StyleProperty::Height, LengthValue::px(height));
    n
}
fn transform(e: &mut Engine, n: NodeHandle, value: &str) {
    set(
        e,
        n,
        StyleProperty::Transform,
        parse_literal(StyleProperty::Transform, value).unwrap(),
    );
}
fn fixture(scale: f64, width: f64, height: f64) -> (Engine, NodeHandle) {
    let mut e =
        Engine::new(ViewportMetrics::from_logical_size(width, height, scale).unwrap()).unwrap();
    e.set_layered_paint_capture_enabled(true);
    let root = e.root();
    let fixed = box_(&mut e, root, width as f32, 50.0);
    set(&mut e, fixed, StyleProperty::Position, Position::Fixed);
    set(&mut e, fixed, StyleProperty::Left, LengthValue::px(0.0));
    set(&mut e, fixed, StyleProperty::Top, LengthValue::px(0.0));
    set(&mut e, fixed, StyleProperty::BackgroundColor, Color::BLUE);
    (e, fixed)
}
fn dimensions(e: &mut Engine) -> [i32; 4] {
    let root = e.root();
    [
        e.client_width(root).unwrap(),
        e.client_height(root).unwrap(),
        e.scroll_width(root).unwrap(),
        e.scroll_height(root).unwrap(),
    ]
}
fn identity(n: NodeHandle) -> PaintLayerNodeIdentity {
    PaintLayerNodeIdentity {
        document: n.document_id(),
        index: n.index(),
        generation: n.generation(),
    }
}
fn position(scene: &SceneSnapshot, n: NodeHandle) -> SourcePositioningReference {
    scene
        .source_layer_paint_capture()
        .unwrap()
        .recording_programs()
        .flat_map(|p| p.contexts())
        .find(|context| context.binding.owner == Some(identity(n)) && context.owns_element_box)
        .unwrap()
        .positioning
}
fn scene(e: &mut Engine) -> SceneSnapshot {
    e.scene().unwrap().with_source_layer_diagnostic().unwrap()
}

#[test]
fn actual_tall_sibling_has_one_viewport_bar_and_fixed_owner_at_both_widths() {
    for scale in SCALES {
        for (width, height) in [(800.0, 600.0), (1280.0, 720.0)] {
            for rtl in [false, true] {
                let (mut e, fixed) = fixture(scale, width, height);
                let root = e.root();
                if rtl {
                    set(&mut e, root, StyleProperty::Direction, Direction::Rtl);
                }
                box_(&mut e, root, 10.0, 1000.0);
                assert_eq!(
                    dimensions(&mut e),
                    [
                        (width - 15.0) as i32,
                        height as i32,
                        (width - 15.0) as i32,
                        1000
                    ]
                );
                let captured = scene(&mut e);
                assert!(matches!(
                    position(&captured, fixed),
                    SourcePositioningReference::Viewport { .. }
                ));
                let bounds = e.bounds(fixed).unwrap().unwrap();
                assert_eq!((bounds.width, bounds.height), (width as f32, 50.0));
                let idle = e.stats();
                let generation = e.scene().unwrap().generation();
                assert_eq!(
                    dimensions(&mut e),
                    [
                        (width - 15.0) as i32,
                        height as i32,
                        (width - 15.0) as i32,
                        1000
                    ]
                );
                assert_eq!(e.scene().unwrap().generation(), generation);
                assert_eq!(e.stats(), idle);
            }
        }
    }
}

#[test]
fn transformed_containing_block_fixed_children_keep_real_both_bar_overflow() {
    for scale in SCALES {
        for spacer in [false, true] {
            let (mut e, fixed) = fixture(scale, 800.0, 600.0);
            let root = e.root();
            if spacer {
                box_(&mut e, root, 10.0, 1000.0);
            }
            let cb = box_(&mut e, root, 800.0, 600.0);
            transform(&mut e, cb, "scale(1)");
            e.append_or_move_child(cb, fixed).unwrap();
            assert_eq!(
                dimensions(&mut e),
                if spacer {
                    [785, 585, 800, 1600]
                } else {
                    [800, 600, 800, 600]
                }
            );
            let captured = scene(&mut e);
            assert!(matches!(
                position(&captured, fixed),
                SourcePositioningReference::ContainingBox { .. }
            ));
            let bounds = e.bounds(fixed).unwrap().unwrap();
            assert_eq!(
                (bounds.y, bounds.width, bounds.height),
                (if spacer { 1000.0 } else { 0.0 }, 800.0, 50.0)
            );
        }
    }
}

#[test]
fn actual_root_transform_changes_fixed_owner_and_restores_viewport_extent() {
    for scale in SCALES {
        let (mut e, fixed) = fixture(scale, 800.0, 600.0);
        let root = e.root();
        box_(&mut e, root, 10.0, 1000.0);
        assert_eq!(dimensions(&mut e), [785, 600, 785, 1000]);
        assert!(matches!(
            position(&scene(&mut e), fixed),
            SourcePositioningReference::Viewport { .. }
        ));
        transform(&mut e, root, "scale(1)");
        assert_eq!(dimensions(&mut e), [785, 585, 800, 1000]);
        assert!(matches!(
            position(&scene(&mut e), fixed),
            SourcePositioningReference::ContainingBox { .. }
        ));
        transform(&mut e, root, "none");
        assert_eq!(dimensions(&mut e), [785, 600, 785, 1000]);
        assert!(matches!(
            position(&scene(&mut e), fixed),
            SourcePositioningReference::Viewport { .. }
        ));
    }
}

#[test]
fn fixed_size_and_composite_transform_do_not_expand_viewport_and_shrink_clamps() {
    for scale in SCALES {
        let (mut e, fixed) = fixture(scale, 800.0, 600.0);
        let root = e.root();
        let tall = box_(&mut e, root, 10.0, 1000.0);
        assert_eq!(dimensions(&mut e), [785, 600, 785, 1000]);
        set(&mut e, fixed, StyleProperty::Width, LengthValue::px(1600.0));
        set(
            &mut e,
            fixed,
            StyleProperty::Height,
            LengthValue::px(2000.0),
        );
        assert_eq!(dimensions(&mut e), [785, 600, 785, 1000]);
        let layouts = e.stats().layouts;
        transform(&mut e, fixed, "scale(2)");
        assert_eq!(dimensions(&mut e), [785, 600, 785, 1000]);
        assert_eq!(e.stats().layouts, layouts);
        assert!(matches!(
            position(&scene(&mut e), fixed),
            SourcePositioningReference::Viewport { .. }
        ));
        e.scroll_to(root, 0.0, 200.0).unwrap();
        e.scene().unwrap();
        assert_eq!(e.scroll_offset(root).unwrap(), (0.0, 200.0));
        assert_eq!(e.drain_scroll_events(), [root]);
        set(&mut e, tall, StyleProperty::Height, LengthValue::px(10.0));
        assert_eq!(dimensions(&mut e), [800, 600, 800, 600]);
        assert_eq!(e.scroll_offset(root).unwrap(), (0.0, 0.0));
        assert_eq!(e.drain_scroll_events(), [root]);
        let idle = e.stats();
        e.scene().unwrap();
        assert_eq!(e.stats(), idle);
    }
}

#[test]
fn absolute_and_normal_flow_overflow_remain_included() {
    for scale in SCALES {
        for position_value in [Position::Absolute, Position::Static] {
            let (mut e, target) = fixture(scale, 800.0, 600.0);
            let root = e.root();
            set(&mut e, target, StyleProperty::Position, position_value);
            box_(&mut e, root, 10.0, 1000.0);
            assert_eq!(
                dimensions(&mut e),
                [
                    785,
                    585,
                    800,
                    if position_value == Position::Static {
                        1050
                    } else {
                        1000
                    }
                ]
            );
            let context_position = position(&scene(&mut e), target);
            if position_value == Position::Static {
                assert_eq!(context_position, SourcePositioningReference::InFlow);
            } else {
                assert!(matches!(
                    context_position,
                    SourcePositioningReference::ContainingBox { .. }
                ));
            }
        }
    }
}

#[test]
fn html_body_overflow_propagation_preserves_normal_extent_and_viewport_fixed_owner() {
    for scale in SCALES {
        let mut e =
            Engine::new(ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap()).unwrap();
        e.set_layered_paint_capture_enabled(true);
        let root = e.root();
        let html = e.create_native_element(ElementTag::Html).unwrap();
        e.append_child(root, html).unwrap();
        let body = e.create_native_element(ElementTag::Body).unwrap();
        e.append_child(html, body).unwrap();
        for n in [html, body] {
            set(&mut e, n, StyleProperty::Display, Display::Block);
            set(
                &mut e,
                n,
                StyleProperty::Margin,
                Edges::all(LengthValue::px(0.0)),
            );
        }
        set(&mut e, body, StyleProperty::Overflow, Overflow::Auto);
        let fixed = box_(&mut e, body, 800.0, 50.0);
        set(&mut e, fixed, StyleProperty::Position, Position::Fixed);
        set(&mut e, fixed, StyleProperty::Top, LengthValue::px(0.0));
        set(&mut e, fixed, StyleProperty::Left, LengthValue::px(0.0));
        set(&mut e, fixed, StyleProperty::BackgroundColor, Color::BLUE);
        box_(&mut e, body, 10.0, 1000.0);
        assert_eq!(dimensions(&mut e), [785, 600, 785, 1000]);
        assert!(matches!(
            position(&scene(&mut e), fixed),
            SourcePositioningReference::Viewport { .. }
        ));
    }
}
