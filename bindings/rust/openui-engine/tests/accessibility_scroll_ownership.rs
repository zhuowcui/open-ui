//! Native accessibility snapshots consume the same retained scroll ownership
//! and integer dimensions as public scrolling applications. No markup/runtime.
use openui_dom::ElementTag;
use openui_engine::{
    AccessibilityAction, AccessibilityNode, AccessibilityPlatformAction as Action, Engine,
    FontCollection, NodeHandle,
};
use openui_geometry::ViewportMetrics;
use openui_style::{
    Border, BorderStyle, Color, Direction, Display, Edges, LengthValue, Overflow,
    RendererStyleValue, ScrollbarGutter, ScrollbarWidth, StyleProperty, StyleValue, TableLayout,
};

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
fn engine(scale: f64) -> Engine {
    Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap()
}
fn set(
    engine: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    value: impl Into<StyleValue>,
) {
    engine.set_property(node, property, value.into()).unwrap();
}
fn size(engine: &mut Engine, node: NodeHandle, width: f32, height: f32) {
    set(engine, node, StyleProperty::Width, LengthValue::px(width));
    set(engine, node, StyleProperty::Height, LengthValue::px(height));
}
fn element(
    engine: &mut Engine,
    parent: NodeHandle,
    tag: ElementTag,
    width: f32,
    height: f32,
) -> NodeHandle {
    let node = engine.create_native_element(tag).unwrap();
    size(engine, node, width, height);
    for property in [StyleProperty::Margin, StyleProperty::Padding] {
        set(engine, node, property, Edges::all(LengthValue::px(0.0)));
    }
    set(
        engine,
        node,
        StyleProperty::Border,
        Border {
            width: 0.0,
            style: BorderStyle::None,
            color: Color::BLACK,
        },
    );
    set(engine, node, StyleProperty::MinWidth, LengthValue::px(0.0));
    set(engine, node, StyleProperty::MinHeight, LengthValue::px(0.0));
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
fn snapshot(engine: &mut Engine, handle: NodeHandle) -> AccessibilityNode {
    let id = engine.accessibility_node_id(handle).unwrap();
    engine
        .accessibility_snapshot()
        .unwrap()
        .into_iter()
        .find(|(candidate, _)| *candidate == id)
        .unwrap()
        .1
}
fn directions(node: &AccessibilityNode, enabled: bool) {
    for action in [
        Action::ScrollDown,
        Action::ScrollUp,
        Action::ScrollLeft,
        Action::ScrollRight,
    ] {
        assert_eq!(node.supports_action(action), enabled, "{action:?}");
    }
}

#[test]
fn hidden_offsets_and_ranges_are_distinct_from_user_actions_and_mutate_incrementally() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let scroll = element(&mut e, root, ElementTag::Div, 100.0, 80.0);
        let _child = element(&mut e, scroll, ElementTag::Div, 60.0, 240.0);
        no_bars(&mut e, scroll);
        set(&mut e, scroll, StyleProperty::Overflow, Overflow::Auto);
        let initial = snapshot(&mut e, scroll);
        directions(&initial, true);
        assert_eq!(
            (
                initial.scroll_y(),
                initial.scroll_y_min(),
                initial.scroll_y_max()
            ),
            (Some(0.0), Some(0.0), Some(160.0))
        );
        e.accessibility_update().unwrap();
        set(&mut e, scroll, StyleProperty::Overflow, Overflow::Hidden);
        e.scroll_to(scroll, 0.0, 40.0).unwrap();
        let hidden = snapshot(&mut e, scroll);
        directions(&hidden, false);
        assert_eq!(
            (hidden.scroll_y(), hidden.scroll_y_max()),
            (Some(40.0), Some(160.0))
        );
        let id = e.accessibility_node_id(scroll).unwrap();
        assert!(e
            .accessibility_update()
            .unwrap()
            .nodes
            .iter()
            .any(|(changed, _)| *changed == id));
        let before = e.stats();
        set(&mut e, scroll, StyleProperty::Overflow, Overflow::Hidden);
        assert!(e.accessibility_update().unwrap().nodes.is_empty());
        assert_eq!(e.stats(), before);
        set(&mut e, scroll, StyleProperty::Overflow, Overflow::Clip);
        let clipped = snapshot(&mut e, scroll);
        directions(&clipped, false);
        assert_eq!((clipped.scroll_x(), clipped.scroll_y()), (None, None));
        assert_eq!(e.scroll_offset(scroll).unwrap(), (0.0, 0.0));
        set(&mut e, scroll, StyleProperty::Overflow, Overflow::Auto);
        let restored = snapshot(&mut e, scroll);
        directions(&restored, true);
        assert_eq!(restored.scroll_y(), Some(0.0));
        e.perform_accessibility_action(
            scroll,
            AccessibilityAction::directional_scroll(Action::ScrollDown).unwrap(),
        )
        .unwrap();
        assert_eq!(snapshot(&mut e, scroll).scroll_y(), Some(64.0));
        let owned = snapshot(&mut e, scroll);
        e.detach(scroll).unwrap();
        assert!(!e
            .accessibility_snapshot()
            .unwrap()
            .iter()
            .any(|(candidate, _)| *candidate == id));
        drop(e);
        assert_eq!(owned.scroll_y(), Some(64.0));
        directions(&initial, true);
        assert_eq!(initial.scroll_y(), Some(0.0));
    }
}

#[test]
fn normalized_table_and_replaced_boxes_do_not_advertise_authored_overflow() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let table = element(&mut e, root, ElementTag::Table, 100.0, 80.0);
        let group = element(&mut e, table, ElementTag::TableBody, 100.0, 80.0);
        let row = element(&mut e, group, ElementTag::TableRow, 100.0, 80.0);
        let cell = element(&mut e, row, ElementTag::TableCell, 100.0, 80.0);
        set(
            &mut e,
            table,
            StyleProperty::BorderSpacing,
            openui_style::parse_literal(StyleProperty::BorderSpacing, "0px").unwrap(),
        );
        set(
            &mut e,
            table,
            StyleProperty::ScrollbarGutter,
            StyleValue::Renderer(RendererStyleValue::ScrollbarGutter(ScrollbarGutter::Stable)),
        );
        for overflow in [Overflow::Auto, Overflow::Scroll] {
            set(&mut e, table, StyleProperty::Overflow, overflow);
            let node = snapshot(&mut e, table);
            directions(&node, false);
            assert_eq!((node.scroll_x(), node.scroll_y()), (None, None));
        }
        set(&mut e, table, StyleProperty::Overflow, Overflow::Hidden);
        e.scroll_to(table, 1000.0, 0.0).unwrap();
        let hidden = snapshot(&mut e, table);
        directions(&hidden, false);
        assert_eq!(
            (hidden.scroll_x(), hidden.scroll_x_max()),
            (Some(15.0), Some(15.0))
        );
        // Structural table sections and rows do not own a scrollport.
        for part in [group, row] {
            set(&mut e, part, StyleProperty::Overflow, Overflow::Scroll);
            let node = snapshot(&mut e, part);
            directions(&node, false);
            assert_eq!((node.scroll_x(), node.scroll_y()), (None, None));
        }
        // A real table cell retains ordinary overflow. An empty cell has
        // programmatic scroll metadata, but no overflowing user-scroll axis.
        no_bars(&mut e, cell);
        set(&mut e, cell, StyleProperty::Overflow, Overflow::Scroll);
        let empty_cell = snapshot(&mut e, cell);
        directions(&empty_cell, false);
        assert_eq!(
            (empty_cell.scroll_x(), empty_cell.scroll_y()),
            (Some(0.0), Some(0.0))
        );
        assert_eq!(
            (empty_cell.scroll_x_max(), empty_cell.scroll_y_max()),
            (Some(0.0), Some(0.0))
        );
        // Fixed native tracks retain the 100px cell while its real child
        // overflows horizontally; this exercises the same owned action route.
        set(
            &mut e,
            table,
            StyleProperty::TableLayout,
            StyleValue::Renderer(RendererStyleValue::TableLayout(TableLayout::Fixed)),
        );
        let oversized = element(&mut e, cell, ElementTag::Div, 180.0, 20.0);
        let overflowing_cell = snapshot(&mut e, cell);
        directions(&overflowing_cell, true);
        assert_eq!(overflowing_cell.scroll_x_max(), Some(80.0));
        e.perform_accessibility_action(
            cell,
            AccessibilityAction::directional_scroll(Action::ScrollRight).unwrap(),
        )
        .unwrap();
        assert_eq!(e.scroll_offset(cell).unwrap(), (80.0, 0.0));
        assert_eq!(snapshot(&mut e, cell).scroll_x(), Some(80.0));
        e.detach(oversized).unwrap();
        directions(&snapshot(&mut e, cell), false);
        assert_eq!(e.scroll_offset(cell).unwrap(), (0.0, 0.0));
        set(
            &mut e,
            table,
            StyleProperty::TableLayout,
            StyleValue::Renderer(RendererStyleValue::TableLayout(TableLayout::Auto)),
        );
        set(&mut e, table, StyleProperty::OverflowY, Overflow::Auto);
        assert_eq!(snapshot(&mut e, table).scroll_x(), None);
        assert_eq!(e.scroll_offset(table).unwrap(), (0.0, 0.0));

        let image = element(&mut e, root, ElementTag::Image, 100.0, 80.0);
        let resource = e.register_image_resource(
            "memory:accessibility-pixel.svg", "image/svg+xml",
            "949c974a51bfed3daa2be2aed981026336857d4cfd494dd120dbb6da66c891f7",
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="black"/></svg>"#.to_vec());
        e.set_image_resource(image, resource, Some((1.0, 1.0)))
            .unwrap();
        for overflow in [Overflow::Hidden, Overflow::Scroll, Overflow::Auto] {
            set(&mut e, image, StyleProperty::Overflow, overflow);
            set(
                &mut e,
                image,
                StyleProperty::ScrollbarGutter,
                StyleValue::Renderer(RendererStyleValue::ScrollbarGutter(ScrollbarGutter::Stable)),
            );
            let node = snapshot(&mut e, image);
            directions(&node, false);
            assert_eq!((node.scroll_x(), node.scroll_y()), (None, None));
            e.scroll_to(image, 1000.0, 1000.0).unwrap();
            assert_eq!(e.scroll_offset(image).unwrap(), (0.0, 0.0));
        }
    }
}

#[test]
fn propagated_body_scroll_metadata_belongs_to_the_viewport_until_policy_changes() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let html = e.create_native_element(ElementTag::Html).unwrap();
        let body = e.create_native_element(ElementTag::Body).unwrap();
        e.append_child(root, html).unwrap();
        e.append_child(html, body).unwrap();
        size(&mut e, body, 100.0, 80.0);
        let _content = element(&mut e, body, ElementTag::Div, 150.0, 400.0);
        set(&mut e, body, StyleProperty::Overflow, Overflow::Auto);
        let viewport = snapshot(&mut e, root);
        directions(&viewport, true);
        let propagated = snapshot(&mut e, body);
        directions(&propagated, false);
        assert_eq!((propagated.scroll_x(), propagated.scroll_y()), (None, None));
        e.scroll_to(root, 0.0, 40.0).unwrap();
        assert_eq!(snapshot(&mut e, root).scroll_y(), Some(40.0));
        set(&mut e, html, StyleProperty::Overflow, Overflow::Hidden);
        let local = snapshot(&mut e, body);
        directions(&local, true);
        e.scroll_to(body, 0.0, 40.0).unwrap();
        assert_eq!(snapshot(&mut e, body).scroll_y(), Some(40.0));
        directions(&snapshot(&mut e, root), false);
        set(&mut e, html, StyleProperty::Overflow, Overflow::Visible);
        directions(&snapshot(&mut e, body), false);
        assert_eq!(e.scroll_offset(body).unwrap(), (0.0, 0.0));
        directions(&snapshot(&mut e, root), true);
    }
}

#[test]
fn fieldset_anonymous_content_keeps_one_semantic_owner_and_a_fixed_legend() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let fieldset = element(&mut e, root, ElementTag::Fieldset, 100.0, 80.0);
        no_bars(&mut e, fieldset);
        set(
            &mut e,
            fieldset,
            StyleProperty::Border,
            Border {
                width: 2.0,
                style: BorderStyle::Solid,
                color: Color::BLACK,
            },
        );
        set(&mut e, fieldset, StyleProperty::Overflow, Overflow::Auto);
        let legend = element(&mut e, fieldset, ElementTag::Legend, 30.0, 10.0);
        let content = element(&mut e, fieldset, ElementTag::Div, 100.0, 200.0);
        let fixed_legend = e.bounds(legend).unwrap().unwrap();
        let fixed_wrapper = e.bounds(fieldset).unwrap().unwrap();
        let content_before = e.bounds(content).unwrap().unwrap();
        let initial = snapshot(&mut e, fieldset);
        // Blink's AX node queries the authored LayoutFieldset, which does not
        // respect CSS overflow. Its anonymous content remains programmatically
        // scrollable through the public element, without an AX alias.
        directions(&initial, false);
        assert_eq!((initial.scroll_x(), initial.scroll_y()), (None, None));
        assert_eq!(snapshot(&mut e, legend).scroll_y(), None);
        e.perform_accessibility_action(
            fieldset,
            AccessibilityAction::directional_scroll(Action::ScrollDown).unwrap(),
        )
        .unwrap();
        assert_eq!(e.scroll_offset(fieldset).unwrap(), (0.0, 0.0));
        assert_eq!(e.bounds(content).unwrap().unwrap(), content_before);
        // This explicit native delta keeps its existing programmatic meaning.
        e.perform_accessibility_action(
            fieldset,
            AccessibilityAction::ScrollBy {
                delta_x: 0.0,
                delta_y: 40.0,
            },
        )
        .unwrap();
        assert_eq!(e.scroll_offset(fieldset).unwrap(), (0.0, 40.0));
        assert_eq!(e.bounds(legend).unwrap().unwrap(), fixed_legend);
        assert_eq!(e.bounds(fieldset).unwrap().unwrap(), fixed_wrapper);
        assert_eq!(
            e.bounds(content).unwrap().unwrap().y,
            content_before.y - 40.0
        );
        let id = e.accessibility_node_id(fieldset).unwrap();
        assert_eq!(
            e.accessibility_snapshot()
                .unwrap()
                .iter()
                .filter(|(candidate, _)| *candidate == id)
                .count(),
            1
        );
        set(&mut e, fieldset, StyleProperty::Overflow, Overflow::Hidden);
        e.scroll_to(fieldset, 0.0, 80.0).unwrap();
        let hidden = snapshot(&mut e, fieldset);
        directions(&hidden, false);
        assert_eq!(hidden.scroll_y(), None);
        assert_eq!(e.scroll_offset(fieldset).unwrap(), (0.0, 80.0));
        assert_eq!(e.bounds(legend).unwrap().unwrap(), fixed_legend);
    }
}

#[test]
fn dimensions_gate_empty_and_subpixel_boxes_and_preserve_negative_scroll_origins() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let scroll = element(&mut e, root, ElementTag::Div, 100.0, 80.0);
        no_bars(&mut e, scroll);
        set(&mut e, scroll, StyleProperty::Overflow, Overflow::Auto);
        directions(&snapshot(&mut e, scroll), false);
        let child = element(&mut e, scroll, ElementTag::Div, 100.25, 80.25);
        assert_eq!(
            (
                e.client_width(scroll).unwrap(),
                e.scroll_width(scroll).unwrap()
            ),
            (100, 100)
        );
        directions(&snapshot(&mut e, scroll), false);
        size(&mut e, child, 160.0, 80.0);
        set(&mut e, scroll, StyleProperty::Direction, Direction::Rtl);
        let reversed = snapshot(&mut e, scroll);
        directions(&reversed, true);
        assert_eq!(
            (reversed.scroll_x_min(), reversed.scroll_x_max()),
            (Some(-60.0), Some(0.0))
        );
        e.scroll_to(scroll, -40.0, 0.0).unwrap();
        assert_eq!(snapshot(&mut e, scroll).scroll_x(), Some(-40.0));
        set(
            &mut e,
            scroll,
            StyleProperty::Transform,
            openui_style::parse_literal(StyleProperty::Transform, "scale(2,3)").unwrap(),
        );
        let transformed = snapshot(&mut e, scroll);
        directions(&transformed, true);
        assert_eq!(transformed.scroll_x_min(), Some(-60.0));
        assert_eq!(
            (
                e.client_width(scroll).unwrap(),
                e.scroll_width(scroll).unwrap()
            ),
            (100, 160)
        );
        let ordinary = element(&mut e, root, ElementTag::Button, 80.0, 24.0);
        let button = snapshot(&mut e, ordinary);
        assert!(button.supports_action(Action::Click));
        directions(&button, false);
        assert_eq!((button.scroll_x(), button.scroll_y()), (None, None));
        let inline = element(&mut e, root, ElementTag::Span, 100.0, 80.0);
        set(&mut e, inline, StyleProperty::Display, Display::Inline);
        set(&mut e, inline, StyleProperty::Overflow, Overflow::Scroll);
        directions(&snapshot(&mut e, inline), false);
    }
}
