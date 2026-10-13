//! Target-aware native AX pages and stock Chromium fractional legal endpoints.
//! Browser DOM witnesses measure clamps, not live AX action delivery.
use openui_dom::ElementTag;
use openui_engine::{
    AccessibilityAction, AccessibilityPlatformAction as Action,
    AccessibilityScrollDirection as Direction, Engine, FontCollection, NodeHandle,
    ScrollButtonDirection,
};
use openui_geometry::ViewportMetrics;
use openui_style::{
    Border, BorderStyle, Color, Direction as TextDirection, Display, Easing, Edges, LengthValue,
    Overflow, RendererStyleValue, ScrollSnapAlign, ScrollSnapAxis, ScrollSnapStrictness,
    ScrollbarWidth, StyleProperty, StyleValue, TransformList, TransformOperation, WritingMode,
};
const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
fn engine(scale: f64) -> Engine {
    Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap()
}
fn set(e: &mut Engine, n: NodeHandle, p: StyleProperty, v: impl Into<StyleValue>) {
    e.set_property(n, p, v.into()).unwrap();
}
fn box_(e: &mut Engine, parent: NodeHandle, w: f32, h: f32) -> NodeHandle {
    let n = e.create_native_element(ElementTag::Div).unwrap();
    set(e, n, StyleProperty::Display, Display::Block);
    set(e, n, StyleProperty::Width, LengthValue::px(w));
    set(e, n, StyleProperty::Height, LengthValue::px(h));
    e.append_child(parent, n).unwrap();
    n
}
fn port(e: &mut Engine, w: f32, h: f32, cw: f32, ch: f32) -> NodeHandle {
    let root = e.root();
    let p = box_(e, root, w, h);
    set(e, p, StyleProperty::Overflow, Overflow::Auto);
    set(
        e,
        p,
        StyleProperty::ScrollbarWidth,
        StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None)),
    );
    box_(e, p, cw, ch);
    p
}
fn page(e: &mut Engine, n: NodeHandle, d: Direction) {
    e.perform_accessibility_action(n, AccessibilityAction::ScrollPage { direction: d })
        .unwrap();
}
#[test]
fn physical_pages_clamp_and_queue_only_changed_targets() {
    for scale in SCALES {
        let mut e = engine(scale);
        let p = port(&mut e, 100.0, 80.0, 160.0, 240.0);
        assert_eq!(
            AccessibilityAction::directional_scroll(Action::ScrollDown),
            Some(AccessibilityAction::ScrollPage {
                direction: Direction::Down
            })
        );
        for (d, expected) in [
            (Direction::Down, (0.0, 64.0)),
            (Direction::Down, (0.0, 128.0)),
            (Direction::Down, (0.0, 160.0)),
            (Direction::Right, (60.0, 160.0)),
            (Direction::Up, (60.0, 96.0)),
            (Direction::Left, (0.0, 96.0)),
        ] {
            page(&mut e, p, d);
            assert_eq!(e.scroll_offset(p).unwrap(), expected);
            assert_eq!(e.drain_scroll_events(), vec![p]);
        }
        e.scroll_to(p, 60.0, 160.0).unwrap();
        e.update().unwrap();
        e.drain_scroll_events();
        let before = e.stats();
        page(&mut e, p, Direction::Down);
        page(&mut e, p, Direction::Right);
        assert_eq!(e.scroll_offset(p).unwrap(), (60.0, 160.0));
        assert!(e.drain_scroll_events().is_empty());
        assert_eq!(e.stats(), before);
        // Explicit application deltas keep their pre-existing native semantics.
        e.scroll_to(p, 0.0, 0.0).unwrap();
        e.perform_accessibility_action(
            p,
            AccessibilityAction::ScrollBy {
                delta_x: 0.0,
                delta_y: 40.0,
            },
        )
        .unwrap();
        assert_eq!(e.scroll_offset(p).unwrap(), (0.0, 40.0));
        let mut e = engine(scale);
        let tiny = port(&mut e, 1.0, 1.0, 10.0, 10.0);
        page(&mut e, tiny, Direction::Right);
        page(&mut e, tiny, Direction::Down);
        assert_eq!(e.scroll_offset(tiny).unwrap(), (1.0, 1.0));
    }
}
#[test]
fn fractional_legal_bounds_match_the_stock_chromium_dom_witnesses() {
    for scale in SCALES {
        for (width, content, ltr, rtl) in [
            (100.75, 180.25, (0.0, 79.0), (-79.0, 0.0)),
            (100.25, 180.75, (0.0, 81.0), (-80.0, 1.0)),
        ] {
            for reversed in [false, true] {
                let mut e = engine(scale);
                let p = port(&mut e, width, 80.75, content, 160.25);
                if reversed {
                    set(&mut e, p, StyleProperty::Direction, TextDirection::Rtl);
                }
                e.scroll_to(p, 1e6, 1e6).unwrap();
                e.update().unwrap();
                let expected = if reversed { rtl } else { ltr };
                assert_eq!(e.scroll_offset(p).unwrap(), (expected.1, 79.0));
                let metrics = e.scroll_metrics(p).unwrap().unwrap();
                assert_eq!(metrics.client_width, width as f64);
                assert_eq!(metrics.scroll_width, content as f64);
                let id = e.accessibility_node_id(p).unwrap();
                let tree = e.accessibility_snapshot().unwrap();
                let owned = &tree
                    .iter()
                    .find(|(candidate, _)| *candidate == id)
                    .unwrap()
                    .1;
                assert_eq!(
                    (owned.scroll_x_min(), owned.scroll_x_max()),
                    (Some(expected.0), Some(expected.1))
                );
                e.scroll_to(p, -1e6, -1e6).unwrap();
                e.update().unwrap();
                assert_eq!(e.scroll_offset(p).unwrap(), (expected.0, 0.0));
                if reversed {
                    page(&mut e, p, Direction::Right);
                    assert_eq!(
                        e.scroll_offset(p).unwrap(),
                        ((expected.0 + 80.0).min(expected.1), 0.0)
                    );
                }
            }
        }
    }
}
#[test]
fn authored_border_padding_and_transforms_do_not_reduce_or_enlarge_pages() {
    for scale in SCALES {
        for transform in [
            TransformList::default(),
            TransformList(vec![TransformOperation::Scale(2.0, 3.0)]),
            TransformList(vec![
                TransformOperation::Rotate(12.0),
                TransformOperation::Translate(LengthValue::px(0.25), LengthValue::px(0.5)),
            ]),
        ] {
            let mut e = engine(scale);
            let p = port(&mut e, 100.0, 80.0, 420.0, 500.0);
            set(
                &mut e,
                p,
                StyleProperty::Padding,
                Edges::all(LengthValue::px(5.0)),
            );
            set(
                &mut e,
                p,
                StyleProperty::Border,
                Border {
                    width: 2.0,
                    style: BorderStyle::Solid,
                    color: Color::BLACK,
                },
            );
            set(&mut e, p, StyleProperty::Transform, transform);
            assert_eq!(
                (e.client_width(p).unwrap(), e.client_height(p).unwrap()),
                (110, 90)
            );
            page(&mut e, p, Direction::Right);
            page(&mut e, p, Direction::Down);
            assert_eq!(e.scroll_offset(p).unwrap(), (91.0, 75.0));
        }
        let mut e = engine(scale);
        let p = port(&mut e, 100.625, 80.625, 400.0, 400.0);
        page(&mut e, p, Direction::Right);
        page(&mut e, p, Direction::Down);
        assert_eq!(e.scroll_offset(p).unwrap(), (81.0, 65.0));
    }
}
#[test]
fn endpoint_noop_preserves_animation_but_changed_page_cancels_it() {
    for scale in SCALES {
        let mut e = engine(scale);
        let p = port(&mut e, 100.0, 80.0, 160.0, 240.0);
        e.scroll_to(p, 60.0, 160.0).unwrap();
        e.drain_scroll_events();
        e.smooth_scroll_to(p, 0.0, 0.0, 100.0, Easing::Linear)
            .unwrap();
        assert!(e.is_animating());
        page(&mut e, p, Direction::Down);
        assert!(e.is_animating());
        assert!(e.drain_scroll_events().is_empty());
        page(&mut e, p, Direction::Up);
        assert!(!e.is_animating());
        assert_eq!(e.scroll_offset(p).unwrap(), (60.0, 96.0));
        assert_eq!(e.drain_scroll_events(), vec![p]);
    }
}
#[test]
fn ax_instant_page_does_not_choose_a_new_mandatory_snap_target() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let p = box_(&mut e, root, 100.0, 80.0);
        set(&mut e, p, StyleProperty::Overflow, Overflow::Auto);
        set(
            &mut e,
            p,
            StyleProperty::ScrollbarWidth,
            StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None)),
        );
        set(
            &mut e,
            p,
            StyleProperty::ScrollSnapAxis,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapAxis(ScrollSnapAxis::Y)),
        );
        set(
            &mut e,
            p,
            StyleProperty::ScrollSnapStrictness,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapStrictness(
                ScrollSnapStrictness::Mandatory,
            )),
        );
        for _ in 0..3 {
            let area = box_(&mut e, p, 100.0, 80.0);
            set(
                &mut e,
                area,
                StyleProperty::ScrollSnapAlign,
                StyleValue::Renderer(RendererStyleValue::ScrollSnapAlign(ScrollSnapAlign::Start)),
            );
        }
        e.update().unwrap();
        page(&mut e, p, Direction::Down);
        e.update().unwrap();
        assert_eq!(e.scroll_offset(p).unwrap(), (0.0, 64.0));
        e.drain_scroll_events();
        let generation = e.scene().unwrap().generation();
        // scrollBy has no requested direction when both deltas are zero.
        e.scroll_by(p, 0.0, 0.0).unwrap();
        assert_eq!(e.scroll_offset(p).unwrap(), (0.0, 64.0));
        assert!(e.drain_scroll_events().is_empty());
        assert_eq!(e.scene().unwrap().generation(), generation);
        // Both supplied scrollTo axes still select an end-position snap.
        e.scroll_to(p, 0.0, 64.0).unwrap();
        assert_eq!(e.scroll_offset(p).unwrap(), (0.0, 80.0));
    }
}
#[test]
fn programmatic_displacement_retains_direction_and_snap_stop_at_five_scales() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let p = box_(&mut e, root, 100.0, 80.0);
        set(&mut e, p, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut e,
            p,
            StyleProperty::ScrollSnapAxis,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapAxis(ScrollSnapAxis::Y)),
        );
        set(
            &mut e,
            p,
            StyleProperty::ScrollSnapStrictness,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapStrictness(
                ScrollSnapStrictness::Mandatory,
            )),
        );
        let mut areas = Vec::new();
        for _ in 0..3 {
            let area = box_(&mut e, p, 100.0, 80.0);
            set(
                &mut e,
                area,
                StyleProperty::ScrollSnapAlign,
                StyleValue::Renderer(RendererStyleValue::ScrollSnapAlign(ScrollSnapAlign::Start)),
            );
            areas.push(area);
        }
        e.update().unwrap();
        e.scroll_by(p, 0.0, 1.0).unwrap();
        assert_eq!(e.scroll_offset(p).unwrap(), (0.0, 80.0));
        e.scroll_by(p, 0.0, -1.0).unwrap();
        assert_eq!(e.scroll_offset(p).unwrap(), (0.0, 0.0));
        set(
            &mut e,
            areas[1],
            StyleProperty::ScrollSnapStop,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapStop(
                openui_style::ScrollSnapStop::Always,
            )),
        );
        e.scroll_by(p, 0.0, 150.0).unwrap();
        assert_eq!(e.scroll_offset(p).unwrap(), (0.0, 80.0));
        // Absolute end-position selection does not apply the directional stop.
        e.scroll_to(p, 0.0, 150.0).unwrap();
        assert_eq!(e.scroll_offset(p).unwrap(), (0.0, 160.0));
        let before = e.scroll_offset(p).unwrap();
        assert!(e.scroll_by(p, f64::NAN, 0.0).is_err());
        assert_eq!(e.scroll_offset(p).unwrap(), before);
    }
}

#[test]
fn partial_two_axis_proximity_result_retains_the_live_other_target() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let p = box_(&mut e, root, 100.0, 80.0);
        set(&mut e, p, StyleProperty::Overflow, Overflow::Hidden);
        set(
            &mut e,
            p,
            StyleProperty::ScrollSnapAxis,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapAxis(ScrollSnapAxis::Both)),
        );
        set(
            &mut e,
            p,
            StyleProperty::ScrollSnapStrictness,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapStrictness(
                ScrollSnapStrictness::Proximity,
            )),
        );
        let content = box_(&mut e, p, 300.0, 300.0);
        set(
            &mut e,
            content,
            StyleProperty::Position,
            openui_style::Position::Relative,
        );
        let both = box_(&mut e, content, 20.0, 20.0);
        set(
            &mut e,
            both,
            StyleProperty::Position,
            openui_style::Position::Absolute,
        );
        set(&mut e, both, StyleProperty::Top, LengthValue::px(0.0));
        set(&mut e, both, StyleProperty::Left, LengthValue::px(0.0));
        set(
            &mut e,
            both,
            StyleProperty::ScrollSnapAlign,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapAlign(ScrollSnapAlign::Start)),
        );
        let inline = box_(&mut e, content, 20.0, 100.0);
        set(
            &mut e,
            inline,
            StyleProperty::Position,
            openui_style::Position::Absolute,
        );
        set(&mut e, inline, StyleProperty::Top, LengthValue::px(0.0));
        set(&mut e, inline, StyleProperty::Left, LengthValue::px(80.0));
        set(
            &mut e,
            inline,
            StyleProperty::ScrollSnapAlignInline,
            StyleValue::Renderer(RendererStyleValue::ScrollSnapAlignInline(Some(
                ScrollSnapAlign::Start,
            ))),
        );
        let button = e
            .create_scroll_button(p, ScrollButtonDirection::Down)
            .unwrap();
        e.update().unwrap();
        assert_eq!(
            e.scroll_button_state(button).unwrap().current_snap_targets,
            [Some(both), Some(both)]
        );
        // X80 selects the inline-only box; Y50 has no proximity candidate.
        e.scroll_to(p, 80.0, 50.0).unwrap();
        assert_eq!(e.scroll_offset(p).unwrap(), (80.0, 50.0));
        assert_eq!(
            e.scroll_button_state(button).unwrap().current_snap_targets,
            [Some(inline), Some(both)]
        );
        e.remove_node(both).unwrap();
        assert_eq!(
            e.scroll_button_state(button).unwrap().current_snap_targets[1],
            None
        );
    }
}

#[test]
fn document_pages_use_the_frame_and_endpoint_return_preserves_fractional_offsets() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        set(&mut e, root, StyleProperty::Overflow, Overflow::Auto);
        set(
            &mut e,
            root,
            StyleProperty::ScrollbarWidth,
            StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None)),
        );
        box_(&mut e, root, 600.0, 800.0);
        e.scroll_to(root, 0.25, 0.25).unwrap();
        e.drain_scroll_events();
        page(&mut e, root, Direction::Up);
        assert_eq!(e.scroll_offset(root).unwrap(), (0.25, 0.25));
        assert!(e.drain_scroll_events().is_empty());
        page(&mut e, root, Direction::Down);
        assert_eq!(e.scroll_offset(root).unwrap(), (0.0, 192.0));
        page(&mut e, root, Direction::Right);
        assert_eq!(e.scroll_offset(root).unwrap(), (256.0, 192.0));
    }
}
#[test]
fn vertical_rtl_actions_remain_physical_and_detached_requests_do_no_work() {
    for scale in SCALES {
        let mut e = engine(scale);
        let p = port(&mut e, 100.0, 80.0, 240.0, 240.0);
        set(
            &mut e,
            p,
            StyleProperty::WritingMode,
            StyleValue::Renderer(RendererStyleValue::WritingMode(WritingMode::VerticalRl)),
        );
        set(&mut e, p, StyleProperty::Direction, TextDirection::Rtl);
        page(&mut e, p, Direction::Left);
        page(&mut e, p, Direction::Up);
        assert_eq!(e.scroll_offset(p).unwrap(), (-80.0, -64.0));
        page(&mut e, p, Direction::Right);
        page(&mut e, p, Direction::Down);
        assert_eq!(e.scroll_offset(p).unwrap(), (0.0, 0.0));
        e.detach(p).unwrap();
        e.update().unwrap();
        e.drain_scroll_events();
        let before = e.stats();
        page(&mut e, p, Direction::Down);
        assert_eq!(e.stats(), before);
        assert!(e.drain_scroll_events().is_empty());
        e.remove(p).unwrap();
        assert!(e
            .perform_accessibility_action(
                p,
                AccessibilityAction::ScrollPage {
                    direction: Direction::Down
                }
            )
            .is_err());
    }
}

#[test]
fn transformed_visible_continuations_do_not_become_ax_scroll_owners() {
    for scale in SCALES {
        let mut e = engine(scale);
        let root = e.root();
        let columns = box_(&mut e, root, 200.0, 40.0);
        set(&mut e, columns, StyleProperty::ColumnCount, 2_i32);
        set(
            &mut e,
            columns,
            StyleProperty::ColumnGap,
            LengthValue::px(0.0),
        );
        set(
            &mut e,
            columns,
            StyleProperty::ColumnFill,
            StyleValue::Renderer(RendererStyleValue::ColumnFill(
                openui_style::ColumnFill::Auto,
            )),
        );
        let continued = box_(&mut e, columns, 100.0, 100.0);
        set(
            &mut e,
            continued,
            StyleProperty::Transform,
            TransformList(vec![TransformOperation::Scale(0.5, 0.5)]),
        );
        let rects = e.client_rects(continued).unwrap();
        assert!(rects.len() >= 2);
        assert_eq!(e.client_height(continued).unwrap(), 100);
        let id = e.accessibility_node_id(continued).unwrap();
        let tree = e.accessibility_snapshot().unwrap();
        let owned = &tree
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .unwrap()
            .1;
        assert_eq!(owned.scroll_y(), None);
        assert!(!owned.supports_action(Action::ScrollDown));
        let before = e.stats();
        let scene = e.scene().unwrap();
        page(&mut e, continued, Direction::Down);
        assert_eq!(e.scroll_offset(continued).unwrap(), (0.0, 0.0));
        assert_eq!(e.client_rects(continued).unwrap(), rects);
        assert_eq!(e.scene().unwrap().generation(), scene.generation());
        assert_eq!(e.stats(), before);
        assert!(e.drain_scroll_events().is_empty());
    }
}
