//! Native origin-owned geometry barriers; no script or fixture-specific path.
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ScrollButtonDirection};
use openui_geometry::ViewportMetrics;
use openui_style::{
    Display, LengthValue, Overflow, RendererStyleValue, ScrollbarWidth, StyleProperty, StyleValue,
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
    set(engine, node, StyleProperty::Width, LengthValue::px(width));
    set(engine, node, StyleProperty::Height, LengthValue::px(height));
    engine.append_child(parent, node).unwrap();
    node
}
fn fixture(scale: f64) -> (Engine, NodeHandle, NodeHandle, NodeHandle, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap()).unwrap();
    let root = engine.root();
    let ancestor = box_(&mut engine, root, 100.0, 100.0);
    let origin = box_(&mut engine, ancestor, 80.0, 60.0);
    set(
        &mut engine,
        origin,
        StyleProperty::Overflow,
        Overflow::Scroll,
    );
    set(
        &mut engine,
        origin,
        StyleProperty::ScrollbarWidth,
        StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None)),
    );
    box_(&mut engine, origin, 160.0, 120.0);
    let button = engine
        .create_scroll_button(origin, ScrollButtonDirection::Right)
        .unwrap();
    set(
        &mut engine,
        button,
        StyleProperty::Width,
        LengthValue::px(24.0),
    );
    set(
        &mut engine,
        button,
        StyleProperty::Height,
        LengthValue::px(20.0),
    );
    let descendant = box_(&mut engine, button, 12.0, 6.0);
    (engine, ancestor, origin, button, descendant)
}

#[test]
fn first_geometry_barrier_restores_hidden_owner_and_owned_descendants() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for hidden_ancestor in [false, true] {
            for rects_first in [false, true] {
                for query_descendant in [false, true] {
                    let (mut engine, ancestor, origin, button, descendant) = fixture(scale);
                    let target = if query_descendant { descendant } else { button };
                    let original = engine.bounds(target).unwrap().unwrap();
                    let hidden = if hidden_ancestor { ancestor } else { origin };
                    set(&mut engine, hidden, StyleProperty::Display, Display::None);
                    assert!(engine.is_connected(button).unwrap());
                    assert!(engine.is_connected(descendant).unwrap());
                    assert!(engine.bounds(target).unwrap().is_none());
                    let hidden_stats = engine.stats();
                    set(&mut engine, hidden, StyleProperty::Display, Display::Block);
                    // No state/update call can repair presentation before this query.
                    if rects_first {
                        assert_eq!(engine.client_rects(target).unwrap(), vec![original]);
                    } else {
                        assert_eq!(engine.bounds(target).unwrap(), Some(original));
                    }
                    assert!(engine.stats().layouts > hidden_stats.layouts);
                    let restored = engine.stats();
                    assert_eq!(engine.bounds(target).unwrap(), Some(original));
                    assert_eq!(engine.stats(), restored);
                }
            }
        }
    }
}

#[test]
fn authored_button_display_is_preserved_across_owner_visibility_changes() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, _, origin, button, descendant) = fixture(scale);
        let original = engine.bounds(button).unwrap().unwrap();
        set(&mut engine, button, StyleProperty::Display, Display::None);
        set(&mut engine, origin, StyleProperty::Display, Display::None);
        assert!(engine.bounds(button).unwrap().is_none());
        set(&mut engine, origin, StyleProperty::Display, Display::Block);
        assert!(engine.bounds(button).unwrap().is_none());
        assert!(engine.client_rects(descendant).unwrap().is_empty());
        assert!(engine.is_connected(button).unwrap());
        set(
            &mut engine,
            button,
            StyleProperty::Display,
            Display::InlineBlock,
        );
        assert_eq!(engine.bounds(button).unwrap(), Some(original));
        assert!(engine.bounds(descendant).unwrap().is_some());
    }
}

#[test]
fn detached_logical_origins_and_ordinary_nodes_skip_unrelated_layout() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, ancestor, origin, button, descendant) = fixture(scale);
        let original = engine.bounds(button).unwrap().unwrap();
        let root = engine.root();
        let unrelated = box_(&mut engine, root, 40.0, 20.0);
        engine.update().unwrap();
        engine.detach(ancestor).unwrap();
        set(
            &mut engine,
            unrelated,
            StyleProperty::Width,
            LengthValue::px(70.0),
        );
        let detached = engine.stats();
        for node in [ancestor, origin, button, descendant] {
            assert!(!engine.is_connected(node).unwrap());
            assert!(engine.bounds(node).unwrap().is_none());
            assert!(engine.client_rects(node).unwrap().is_empty());
            assert_eq!(engine.stats(), detached);
        }
        engine.append_child(root, ancestor).unwrap();
        let reattached = engine.bounds(button).unwrap().unwrap();
        assert_eq!(
            (reattached.width, reattached.height),
            (original.width, original.height)
        );
        assert!(engine.bounds(descendant).unwrap().is_some());
        let ordinary = box_(&mut engine, root, 20.0, 10.0);
        engine.update().unwrap();
        engine.detach(ordinary).unwrap();
        set(
            &mut engine,
            unrelated,
            StyleProperty::Width,
            LengthValue::px(90.0),
        );
        let baseline = engine.stats();
        assert!(engine.bounds(ordinary).unwrap().is_none());
        assert!(engine.client_rects(ordinary).unwrap().is_empty());
        assert!(!engine.is_connected(ordinary).unwrap());
        assert_eq!(engine.stats(), baseline);
        engine.remove(origin).unwrap();
        let removed = engine.stats();
        assert!(engine.bounds(button).is_err());
        assert!(engine.client_rects(descendant).is_err());
        assert_eq!(engine.stats(), removed);
    }
}

#[test]
fn page_snap_targets_distinguish_clamped_end_from_reachable_alignment() {
    use openui_style::{ScrollSnapAlign, ScrollSnapAxis, ScrollSnapStrictness};
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap()).unwrap();
        let root = engine.root();
        let origin = box_(&mut engine, root, 80.0, 100.0);
        set(
            &mut engine,
            origin,
            StyleProperty::Overflow,
            Overflow::Scroll,
        );
        set(
            &mut engine,
            origin,
            StyleProperty::ScrollbarWidth,
            StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None)),
        );
        set(
            &mut engine,
            origin,
            StyleProperty::ScrollSnapAxis,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapAxis(ScrollSnapAxis::Y)),
        );
        set(
            &mut engine,
            origin,
            StyleProperty::ScrollSnapStrictness,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapStrictness(
                ScrollSnapStrictness::Mandatory,
            )),
        );
        let mut targets = Vec::new();
        for _ in 0..3 {
            let area = box_(&mut engine, origin, 80.0, 60.0);
            set(
                &mut engine,
                area,
                StyleProperty::ScrollSnapAlign,
                StyleValue::Renderer(RendererStyleValue::ScrollSnapAlign(ScrollSnapAlign::Start)),
            );
            targets.push(area);
        }
        let button = engine
            .create_scroll_button(origin, ScrollButtonDirection::Down)
            .unwrap();
        let clamped = engine.scroll_button_state(button).unwrap();
        assert_eq!(engine.client_height(origin).unwrap(), 100);
        assert_eq!(engine.scroll_height(origin).unwrap(), 180);
        assert_eq!(clamped.target_offset.unwrap().1, 80.0);
        assert_eq!(clamped.snap_targets[1], Some(targets[2]));
        box_(&mut engine, origin, 80.0, 60.0);
        engine.scroll_to(origin, 0.0, 0.0).unwrap();
        let reachable = engine.scroll_button_state(button).unwrap();
        assert_eq!(engine.scroll_height(origin).unwrap(), 240);
        assert_eq!(reachable.target_offset.unwrap().1, 60.0);
        assert_eq!(reachable.snap_targets[1], Some(targets[1]));
    }
}
