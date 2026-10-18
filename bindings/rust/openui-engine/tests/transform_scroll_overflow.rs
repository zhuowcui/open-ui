//! Native mutation guards for retained overflow, not screenshot compensations.
use openui_dom::ElementTag;
use openui_engine::{
    AnimationTimeline, Engine, NodeHandle, ScrollButtonDirection, ViewportMetrics,
};
use openui_style::{
    parse_literal, AnimationOptions, Direction, Display, Edges, FillMode, Keyframes, LengthValue,
    Overflow, Position, PropertyKeyframes, RendererStyleValue, ScrollSnapAlign, ScrollSnapAxis,
    ScrollSnapStrictness, ScrollbarWidth, StyleProperty, StyleValue, TimelineAxis, TimelineRange,
};
const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
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
fn no_bars(engine: &mut Engine, node: NodeHandle) {
    set(
        engine,
        node,
        StyleProperty::ScrollbarWidth,
        StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None)),
    );
}
fn transform(engine: &mut Engine, node: NodeHandle, input: &str) {
    set(
        engine,
        node,
        StyleProperty::Transform,
        parse_literal(StyleProperty::Transform, input).unwrap(),
    );
}
fn fixture(scale: f64) -> (Engine, NodeHandle, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap()).unwrap();
    let root = engine.root();
    no_bars(&mut engine, root);
    let container = box_(&mut engine, root, 200.0, 150.0);
    set(
        &mut engine,
        container,
        StyleProperty::Overflow,
        Overflow::Hidden,
    );
    no_bars(&mut engine, container);
    let child = box_(&mut engine, container, 100.0, 50.0);
    (engine, container, child)
}
fn dimensions(engine: &mut Engine, node: NodeHandle) -> [i32; 4] {
    [
        engine.client_width(node).unwrap(),
        engine.client_height(node).unwrap(),
        engine.scroll_width(node).unwrap(),
        engine.scroll_height(node).unwrap(),
    ]
}

#[test]
fn transform_growth_and_shrink_refresh_ancestors_clamps_buttons_and_zero_work() {
    for scale in SCALES {
        let (mut engine, container, child) = fixture(scale);
        let button = engine
            .create_scroll_button(container, ScrollButtonDirection::Right)
            .unwrap();
        assert_eq!(dimensions(&mut engine, container), [200, 150, 200, 150]);
        assert!(!engine.scroll_button_state(button).unwrap().enabled);
        let initial = engine.stats();
        // Actual reference center50,25: scale4 reaches250,125, not400,200.
        transform(&mut engine, child, "scale(4)");
        assert_eq!(dimensions(&mut engine, container), [200, 150, 250, 150]);
        assert_eq!(engine.stats().layouts, initial.layouts);
        assert!(engine.scroll_button_state(button).unwrap().enabled);
        assert_eq!(engine.bounds(child).unwrap().unwrap().width, 400.0);
        engine.scroll_to(container, 50.0, 0.0).unwrap();
        engine.scene().unwrap();
        assert_eq!(engine.drain_scroll_events(), [container]);
        transform(&mut engine, child, "none");
        assert_eq!(dimensions(&mut engine, container), [200, 150, 200, 150]);
        assert_eq!(engine.scroll_offset(container).unwrap(), (0.0, 0.0));
        assert_eq!(engine.drain_scroll_events(), [container]);
        assert!(!engine.scroll_button_state(button).unwrap().enabled);
        assert_eq!(engine.stats().layouts, initial.layouts);
        let idle = engine.stats();
        transform(&mut engine, child, "none");
        assert_eq!(dimensions(&mut engine, container), [200, 150, 200, 150]);
        engine.scene().unwrap();
        assert_eq!(engine.stats(), idle);
        assert!(engine.drain_scroll_events().is_empty());
    }
}

#[test]
fn non_scroll_owner_propagates_changed_transforms_through_nested_boxes() {
    for scale in SCALES {
        let (mut engine, container, wrapper) = fixture(scale);
        let child = box_(&mut engine, wrapper, 100.0, 50.0);
        assert_eq!(engine.scroll_width(container).unwrap(), 200);
        let layouts = engine.stats().layouts;
        transform(&mut engine, child, "translateX(150px)");
        assert_eq!(engine.scroll_width(wrapper).unwrap(), 250);
        assert_eq!(engine.scroll_width(container).unwrap(), 250);
        assert_eq!(engine.stats().layouts, layouts);
        transform(&mut engine, child, "translateX(-300px)");
        assert_eq!(engine.scroll_width(container).unwrap(), 200);
        assert_eq!(engine.stats().layouts, layouts);
    }
}

#[test]
fn viewport_refresh_keeps_real_reachable_sides_and_root_precision() {
    for scale in SCALES {
        for rtl in [false, true] {
            let mut engine =
                Engine::new(ViewportMetrics::from_logical_size(200.0, 150.0, scale).unwrap())
                    .unwrap();
            let root = engine.root();
            no_bars(&mut engine, root);
            set(&mut engine, root, StyleProperty::Overflow, Overflow::Hidden);
            if rtl {
                set(&mut engine, root, StyleProperty::Direction, Direction::Rtl);
            }
            let child = box_(&mut engine, root, 100.0, 50.0);
            set(
                &mut engine,
                child,
                StyleProperty::Position,
                Position::Absolute,
            );
            set(
                &mut engine,
                child,
                StyleProperty::Left,
                LengthValue::px(0.0),
            );
            set(&mut engine, child, StyleProperty::Top, LengthValue::px(0.0));
            engine.scene().unwrap();
            let layouts = engine.stats().layouts;
            transform(&mut engine, child, "scale(4)");
            assert_eq!(
                dimensions(&mut engine, root),
                [200, 150, if rtl { 350 } else { 250 }, 150]
            );
            engine
                .scroll_to(root, if rtl { -150.0 } else { 50.0 }, 0.0)
                .unwrap();
            assert_eq!(
                engine.scroll_offset(root).unwrap(),
                (if rtl { -150.0 } else { 50.0 }, 0.0)
            );
            transform(&mut engine, child, "none");
            assert_eq!(dimensions(&mut engine, root), [200, 150, 200, 150]);
            assert_eq!(engine.scroll_offset(root).unwrap(), (0.0, 0.0));
            assert_eq!(engine.stats().layouts, layouts);
            transform(
                &mut engine,
                child,
                if rtl {
                    "translateX(300px)"
                } else {
                    "translateX(-300px)"
                },
            );
            assert_eq!(dimensions(&mut engine, root), [200, 150, 200, 150]);
            let idle = engine.stats();
            engine.scene().unwrap();
            assert_eq!(engine.stats(), idle);
        }
    }
}

#[test]
fn auto_gutter_changes_request_layout_but_fixed_gutters_retain_box_measurements() {
    for scale in SCALES {
        for display in [Display::Block, Display::Flex, Display::Grid] {
            let (mut engine, container, child) = fixture(scale);
            set(&mut engine, container, StyleProperty::Display, display);
            set(
                &mut engine,
                container,
                StyleProperty::ScrollbarWidth,
                StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::Auto)),
            );
            set(
                &mut engine,
                container,
                StyleProperty::Overflow,
                Overflow::Auto,
            );
            set(
                &mut engine,
                child,
                StyleProperty::FlexShrink,
                StyleValue::Number(0.0),
            );
            assert_eq!(dimensions(&mut engine, container), [200, 150, 200, 150]);
            let before = engine.stats();
            transform(&mut engine, child, "translateX(150px)");
            assert_eq!(dimensions(&mut engine, container), [200, 135, 250, 135]);
            assert!(engine.stats().layouts > before.layouts);
            transform(&mut engine, child, "none");
            assert_eq!(dimensions(&mut engine, container), [200, 150, 200, 150]);
            set(
                &mut engine,
                container,
                StyleProperty::Overflow,
                Overflow::Scroll,
            );
            assert_eq!(dimensions(&mut engine, container), [185, 135, 185, 135]);
            let fixed = engine.stats();
            transform(&mut engine, child, "translateX(150px)");
            assert_eq!(dimensions(&mut engine, container), [185, 135, 250, 135]);
            assert_eq!(engine.stats().layouts, fixed.layouts);
            let idle = engine.stats();
            engine.scene().unwrap();
            assert_eq!(engine.stats(), idle);
        }
    }
}

#[test]
fn changed_transform_resnaps_live_target_without_relayout_or_repeated_events() {
    for scale in SCALES {
        let (mut engine, container, content) = fixture(scale);
        set(
            &mut engine,
            content,
            StyleProperty::Width,
            LengthValue::px(400.0),
        );
        let target = box_(&mut engine, content, 20.0, 150.0);
        set(
            &mut engine,
            target,
            StyleProperty::ScrollSnapAlign,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapAlign(ScrollSnapAlign::Start)),
        );
        set(
            &mut engine,
            container,
            StyleProperty::ScrollSnapAxis,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapAxis(ScrollSnapAxis::X)),
        );
        engine
            .set_scroll_snap_strictness(container, ScrollSnapStrictness::Mandatory)
            .unwrap();
        let button = engine
            .create_scroll_button(container, ScrollButtonDirection::Right)
            .unwrap();
        assert_eq!(
            engine
                .scroll_button_state(button)
                .unwrap()
                .current_snap_targets[0],
            Some(target)
        );
        let layouts = engine.stats().layouts;
        transform(&mut engine, target, "translateX(50px)");
        engine.scene().unwrap();
        assert_eq!(engine.scroll_offset(container).unwrap().0, 50.0);
        assert_eq!(
            engine
                .scroll_button_state(button)
                .unwrap()
                .current_snap_targets[0],
            Some(target)
        );
        assert_eq!(engine.drain_scroll_events(), [container]);
        assert_eq!(engine.stats().layouts, layouts);
        let idle = engine.stats();
        engine.scene().unwrap();
        assert_eq!(engine.stats(), idle);
        assert!(engine.drain_scroll_events().is_empty());
    }
}

#[test]
fn transform_overflow_activates_and_retires_the_real_scroll_timeline_range() {
    for scale in SCALES {
        let (mut engine, container, child) = fixture(scale);
        let root = engine.root();
        let target = box_(&mut engine, root, 10.0, 10.0);
        let id = engine
            .animate(
                target,
                PropertyKeyframes::typed(
                    StyleProperty::Opacity,
                    Keyframes::from_values(0.0_f32, 1.0_f32),
                )
                .unwrap(),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::Both,
                    ..AnimationOptions::default()
                },
                AnimationTimeline::Scroll {
                    source: container,
                    axis: TimelineAxis::X,
                    range: TimelineRange::new(0.0, 50.0).unwrap(),
                },
            )
            .unwrap();
        assert!(
            !engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .active
        );
        let layouts = engine.stats().layouts;
        transform(&mut engine, child, "scale(4)");
        let active = engine.animation_timeline_snapshot(id).unwrap().unwrap();
        assert!(active.active);
        assert_eq!(active.cover_end, Some(50.0));
        assert_eq!(
            active.duration_ms,
            Some(((50.0_f64 * 16.0 / 1000.0) / 1000.0) * 1000.0)
        );
        engine.scroll_to(container, 25.0, 0.0).unwrap();
        assert_eq!(engine.computed_style(target).unwrap().opacity, 0.5);
        transform(&mut engine, child, "none");
        assert!(
            !engine
                .animation_timeline_snapshot(id)
                .unwrap()
                .unwrap()
                .active
        );
        assert_eq!(engine.scroll_offset(container).unwrap(), (0.0, 0.0));
        assert_eq!(engine.stats().layouts, layouts);
    }
}

#[test]
fn containing_block_topology_still_uses_layout_for_real_fixed_descendants() {
    for scale in SCALES {
        let (mut engine, container, child) = fixture(scale);
        set(
            &mut engine,
            container,
            StyleProperty::Position,
            Position::Relative,
        );
        set(
            &mut engine,
            container,
            StyleProperty::Top,
            LengthValue::px(30.0),
        );
        set(&mut engine, child, StyleProperty::Position, Position::Fixed);
        set(&mut engine, child, StyleProperty::Top, LengthValue::px(0.0));
        engine.scene().unwrap();
        assert_eq!(engine.bounds(child).unwrap().unwrap().y, 0.0);
        let before = engine.stats();
        transform(&mut engine, container, "scale(1)");
        assert_eq!(engine.bounds(child).unwrap().unwrap().y, 30.0);
        assert!(engine.stats().layouts > before.layouts);
        let transformed = engine.stats();
        transform(&mut engine, container, "none");
        assert_eq!(engine.bounds(child).unwrap().unwrap().y, 0.0);
        assert!(engine.stats().layouts > transformed.layouts);
    }
}

#[test]
fn shrinking_transform_clamps_before_rebuilding_owned_sticky_geometry() {
    for scale in SCALES {
        let (mut engine, container, child) = fixture(scale);
        let sticky = box_(&mut engine, container, 10.0, 20.0);
        set(
            &mut engine,
            sticky,
            StyleProperty::Position,
            Position::Sticky,
        );
        set(
            &mut engine,
            sticky,
            StyleProperty::Left,
            LengthValue::px(0.0),
        );
        transform(&mut engine, child, "scale(4)");
        assert_eq!(engine.scroll_width(container).unwrap(), 250);
        engine.scroll_to(container, 50.0, 0.0).unwrap();
        let original = engine.bounds(sticky).unwrap().unwrap();
        let before = engine.stats();
        engine.drain_scroll_events();
        transform(&mut engine, child, "none");
        assert_eq!(engine.scroll_width(container).unwrap(), 200);
        assert_eq!(engine.scroll_offset(container).unwrap(), (0.0, 0.0));
        assert_eq!(engine.bounds(sticky).unwrap().unwrap(), original);
        assert!(engine.stats().layouts > before.layouts);
        assert_eq!(engine.drain_scroll_events(), [container]);
        let idle = engine.stats();
        engine.scene().unwrap();
        assert_eq!(engine.stats(), idle);
    }
}

#[test]
fn viewport_retains_own_authored_size_and_padding_without_counting_restored_gutters() {
    for scale in SCALES {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(200.0, 150.0, scale).unwrap()).unwrap();
        let root = engine.root();
        no_bars(&mut engine, root);
        set(&mut engine, root, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut engine,
            root,
            StyleProperty::Width,
            LengthValue::px(240.0),
        );
        set(
            &mut engine,
            root,
            StyleProperty::Height,
            LengthValue::px(300.0),
        );
        set(
            &mut engine,
            root,
            StyleProperty::Padding,
            Edges::all(LengthValue::px(20.0)),
        );
        assert!(engine.children(root).unwrap().is_empty());
        assert_eq!(dimensions(&mut engine, root), [200, 150, 280, 340]);
        let layouts = engine.stats().layouts;
        transform(&mut engine, root, "scale(1)");
        assert_eq!(dimensions(&mut engine, root), [200, 150, 280, 340]);
        assert_eq!(engine.stats().layouts, layouts);
        engine.scroll_to(root, 80.0, 190.0).unwrap();
        assert_eq!(engine.scroll_offset(root).unwrap(), (80.0, 190.0));
        let mut empty =
            Engine::new(ViewportMetrics::from_logical_size(200.0, 150.0, scale).unwrap()).unwrap();
        let view = empty.root();
        set(&mut empty, view, StyleProperty::Overflow, Overflow::Scroll);
        set(
            &mut empty,
            view,
            StyleProperty::ScrollbarWidth,
            StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::Auto)),
        );
        assert_eq!(dimensions(&mut empty, view), [185, 135, 185, 135]);
        let initial = empty.stats();
        transform(&mut empty, view, "scale(1)");
        assert_eq!(dimensions(&mut empty, view), [185, 135, 185, 135]);
        assert_eq!(empty.stats().layouts, initial.layouts);
        empty.scroll_to(view, 1000.0, 1000.0).unwrap();
        assert_eq!(empty.scroll_offset(view).unwrap(), (0.0, 0.0));
        let scene = empty.scene().unwrap();
        let port = scene.fragments().viewport_scrollport.unwrap();
        assert_eq!(port.layout_box_rect.size.width.to_f64(), 185.0);
        assert_eq!(port.layout_box_rect.size.height.to_f64(), 135.0);
        assert_eq!(scene.fragments().size.width.to_f64(), 200.0);
        assert_eq!(scene.fragments().size.height.to_f64(), 150.0);
    }
}

#[test]
fn viewport_principal_html_and_body_extents_survive_transform_refresh() {
    for scale in SCALES {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(200.0, 150.0, scale).unwrap()).unwrap();
        let root = engine.root();
        no_bars(&mut engine, root);
        let html = engine.create_native_element(ElementTag::Html).unwrap();
        engine.append_child(root, html).unwrap();
        no_bars(&mut engine, html);
        set(
            &mut engine,
            html,
            StyleProperty::Margin,
            Edges::all(LengthValue::px(0.0)),
        );
        set(
            &mut engine,
            html,
            StyleProperty::Width,
            LengthValue::px(400.0),
        );
        set(
            &mut engine,
            html,
            StyleProperty::Height,
            LengthValue::px(300.0),
        );
        assert_eq!(dimensions(&mut engine, root), [200, 150, 400, 300]);
        let body = engine.create_native_element(ElementTag::Body).unwrap();
        engine.append_child(html, body).unwrap();
        no_bars(&mut engine, body);
        set(
            &mut engine,
            body,
            StyleProperty::Margin,
            Edges::all(LengthValue::px(0.0)),
        );
        set(
            &mut engine,
            body,
            StyleProperty::Width,
            LengthValue::px(450.0),
        );
        set(
            &mut engine,
            body,
            StyleProperty::Height,
            LengthValue::px(500.0),
        );
        assert_eq!(dimensions(&mut engine, root), [200, 150, 450, 500]);
        let layouts = engine.stats().layouts;
        transform(&mut engine, body, "translateX(50px)");
        assert_eq!(dimensions(&mut engine, root), [200, 150, 500, 500]);
        assert_eq!(engine.stats().layouts, layouts);
        transform(&mut engine, body, "none");
        assert_eq!(dimensions(&mut engine, root), [200, 150, 450, 500]);
        let idle = engine.stats();
        engine.scene().unwrap();
        assert_eq!(engine.stats(), idle);
    }
}
