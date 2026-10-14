//! Native reveal operations over the retained fragment geometry.

use crate::{Engine, EngineError, NodeHandle, ScrollAnimationId, WorldTransform};
use openui_compositor::SceneRect;
use openui_dom::NodeId;
use openui_geometry::PhysicalOffset;
use openui_layout::{Fragment, ScrollArea};
use openui_style::{Direction, Display, Position, WritingMode};

/// Alignment along a logical writing axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAlignment {
    Start,
    Center,
    End,
    Nearest,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessibilityAction, ViewportMetrics};
    use openui_dom::ElementTag;
    use openui_style::{LengthValue, Overflow, ScrollbarWidth, StyleProperty};

    fn box_node(engine: &mut Engine, parent: NodeHandle, width: f32, height: f32) -> NodeHandle {
        let node = engine.create_native_element(ElementTag::Div).unwrap();
        engine
            .set_property(node, StyleProperty::Width, LengthValue::px(width).into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Height, LengthValue::px(height).into())
            .unwrap();
        engine
            .set_property(
                node,
                StyleProperty::ScrollbarWidth,
                openui_style::StyleValue::Renderer(
                    openui_style::RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
                ),
            )
            .unwrap();
        engine.append_child(parent, node).unwrap();
        node
    }

    fn nested() -> (Engine, NodeHandle, NodeHandle, NodeHandle) {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        engine
            .set_property(
                root,
                StyleProperty::ScrollbarWidth,
                openui_style::StyleValue::Renderer(
                    openui_style::RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
                ),
            )
            .unwrap();
        let outer = box_node(&mut engine, root, 100.0, 80.0);
        engine
            .set_property(outer, StyleProperty::Position, Position::Absolute.into())
            .unwrap();
        engine
            .set_property(outer, StyleProperty::Left, LengthValue::px(40.0).into())
            .unwrap();
        engine
            .set_property(outer, StyleProperty::Top, LengthValue::px(30.0).into())
            .unwrap();
        engine
            .set_property(outer, StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        let _spacer = box_node(&mut engine, outer, 200.0, 200.0);
        let inner = box_node(&mut engine, outer, 200.0, 100.0);
        engine
            .set_property(inner, StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        let target = box_node(&mut engine, inner, 20.0, 20.0);
        engine
            .set_property(
                target,
                StyleProperty::MarginLeft,
                LengthValue::px(150.0).into(),
            )
            .unwrap();
        engine
            .set_property(
                target,
                StyleProperty::MarginTop,
                LengthValue::px(120.0).into(),
            )
            .unwrap();
        (engine, outer, inner, target)
    }

    #[test]
    fn owned_reveal_curve_uses_the_shared_scroll_clock_in_every_container() {
        use openui_style::{Easing, LinearStop};
        let (mut engine, outer, inner, target) = nested();
        engine.scroll_into_view(target, nearest()).unwrap();
        let destinations = [outer, inner].map(|node| engine.scroll_offset(node).unwrap());
        for node in [engine.root(), outer, inner] {
            engine.scroll_to(node, 0.0, 0.0).unwrap();
        }
        let curve = Easing::linear_stops(vec![
            LinearStop {
                input: 0.0,
                output: 0.0,
            },
            LinearStop {
                input: 0.5,
                output: 0.25,
            },
            LinearStop {
                input: 1.0,
                output: 1.0,
            },
        ])
        .unwrap();
        let ids = engine
            .smooth_scroll_into_view_with_easing(target, nearest(), 100.0, curve)
            .unwrap();
        assert!(!ids.is_empty());
        engine.set_animation_time(50.0).unwrap();
        for (node, destination) in [outer, inner].into_iter().zip(destinations) {
            let halfway = engine.scroll_offset(node).unwrap();
            // Chromium exposes integer scroll offsets after sampling the curve
            // (ScrollableArea::GetWebExposedScrollOffset).
            assert_eq!(
                halfway,
                (
                    (destination.0 * 0.25).round(),
                    (destination.1 * 0.25).round()
                )
            );
        }
        engine.set_animation_time(100.0).unwrap();
        for (node, destination) in [outer, inner].into_iter().zip(destinations) {
            assert_eq!(engine.scroll_offset(node).unwrap(), destination);
        }
    }

    #[test]
    fn reveal_rejects_invalid_owned_curve_before_starting_any_container() {
        let (mut engine, outer, inner, target) = nested();
        let before = [outer, inner].map(|node| engine.scroll_offset(node).unwrap());
        assert!(engine
            .smooth_scroll_into_view_with_easing(
                target,
                nearest(),
                100.0,
                openui_style::Easing::LinearStopsPercent(Vec::new())
            )
            .is_err());
        assert!(!engine.is_animating());
        for (node, offset) in [outer, inner].into_iter().zip(before) {
            assert_eq!(engine.scroll_offset(node).unwrap(), offset);
        }
    }

    fn nearest() -> ScrollIntoViewOptions {
        ScrollIntoViewOptions {
            block: ScrollAlignment::Nearest,
            inline: ScrollAlignment::Nearest,
            ..Default::default()
        }
    }

    #[test]
    fn reveal_nested_hidden_scrollports_and_preserve_unchanged_frame() {
        let (mut engine, outer, inner, target) = nested();
        engine.scroll_into_view(target, nearest()).unwrap();
        assert_eq!(engine.scroll_offset(inner).unwrap(), (0.0, 40.0));
        assert_eq!(engine.scroll_offset(outer).unwrap(), (70.0, 220.0));
        let bounds = engine.bounds(target).unwrap().unwrap();
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (120.0, 90.0, 20.0, 20.0)
        );
        let scene = engine.scene().unwrap();
        let stats = engine.stats();
        engine.scroll_into_view(target, nearest()).unwrap();
        assert_eq!(scene.generation(), engine.scene().unwrap().generation());
        assert_eq!(stats, engine.stats());
    }

    #[test]
    fn nearest_container_does_not_scroll_its_parent() {
        let (mut engine, outer, inner, target) = nested();
        let mut options = nearest();
        options.container = ScrollIntoViewContainer::Nearest;
        engine.scroll_into_view(target, options).unwrap();
        assert_eq!(engine.scroll_offset(inner).unwrap(), (0.0, 40.0));
        assert_eq!(engine.scroll_offset(outer).unwrap(), (0.0, 0.0));
    }

    #[test]
    fn accessibility_reveal_uses_the_same_nested_nearest_operation() {
        let (mut engine, outer, inner, target) = nested();
        engine
            .perform_accessibility_action(target, AccessibilityAction::ScrollIntoView)
            .unwrap();
        assert_eq!(engine.scroll_offset(inner).unwrap(), (0.0, 40.0));
        assert_eq!(engine.scroll_offset(outer).unwrap(), (70.0, 220.0));
    }

    #[test]
    fn clip_is_not_a_scroll_container() {
        let (mut engine, outer, inner, target) = nested();
        engine
            .set_property(inner, StyleProperty::Overflow, Overflow::Clip.into())
            .unwrap();
        engine.scroll_into_view(target, nearest()).unwrap();
        assert_eq!(engine.scroll_offset(inner).unwrap(), (0.0, 0.0));
        // Clip does not establish a formatting context: the 120px target
        // margin collapses through the inner box. Pinned Chromium confirms
        // a 420px outer extent and this 260px nearest reveal at all five scales.
        assert_eq!(engine.scroll_offset(outer).unwrap(), (70.0, 260.0));
    }

    #[test]
    fn smooth_reveal_plans_outer_destination_from_final_inner_view() {
        let (mut engine, outer, inner, target) = nested();
        let ids = engine
            .smooth_scroll_into_view(target, nearest(), 100.0)
            .unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(engine.scroll_offset(inner).unwrap(), (0.0, 0.0));
        assert_eq!(engine.scroll_offset(outer).unwrap(), (0.0, 0.0));
        engine.set_animation_time(100.0).unwrap();
        assert_eq!(engine.scroll_offset(inner).unwrap(), (0.0, 40.0));
        assert_eq!(engine.scroll_offset(outer).unwrap(), (70.0, 220.0));
        assert!(!engine.is_animating());
    }

    #[test]
    fn reduced_motion_and_zero_duration_settle_and_invalid_duration_is_atomic() {
        for reduced in [false, true] {
            let (mut engine, outer, inner, target) = nested();
            engine.set_prefers_reduced_motion(reduced);
            assert!(engine
                .smooth_scroll_into_view(target, nearest(), f64::NAN)
                .is_err());
            assert_eq!(engine.scroll_offset(outer).unwrap(), (0.0, 0.0));
            assert_eq!(engine.scroll_offset(inner).unwrap(), (0.0, 0.0));
            engine
                .smooth_scroll_into_view(target, nearest(), if reduced { 100.0 } else { 0.0 })
                .unwrap();
            assert_eq!(engine.scroll_offset(outer).unwrap(), (70.0, 220.0));
            assert_eq!(engine.scroll_offset(inner).unwrap(), (0.0, 40.0));
            assert!(!engine.is_animating());
        }
    }

    #[test]
    fn detached_foreign_and_stale_targets_keep_handle_rules() {
        let (mut engine, _, _, target) = nested();
        let detached = engine.create_native_element(ElementTag::Div).unwrap();
        let stats = engine.stats();
        engine.scroll_into_view(detached, nearest()).unwrap();
        assert_eq!(engine.stats(), stats);
        let other =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
        assert_eq!(
            engine.scroll_into_view(other.root(), nearest()),
            Err(EngineError::WrongDocument)
        );
        engine.remove(target).unwrap();
        assert!(engine.scroll_into_view(target, nearest()).is_err());
    }
    fn insets_fixture() -> (Engine, NodeHandle, NodeHandle) {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        let port = box_node(&mut engine, root, 100.0, 80.0);
        engine
            .set_property(port, StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        let target = box_node(&mut engine, port, 20.0, 20.0);
        for (node, property, value) in [
            (target, StyleProperty::MarginLeft, 150.0),
            (target, StyleProperty::MarginTop, 120.0),
            (target, StyleProperty::ScrollMarginLeft, 10.0),
            (target, StyleProperty::ScrollMarginTop, 15.0),
            (port, StyleProperty::ScrollPaddingLeft, 20.0),
            (port, StyleProperty::ScrollPaddingTop, 25.0),
        ] {
            engine
                .set_property(node, property, LengthValue::px(value).into())
                .unwrap();
        }
        // Leave enough reachable content for start alignment to avoid clamping.
        let _tail = box_node(&mut engine, port, 400.0, 300.0);
        (engine, port, target)
    }

    fn absolute_target(engine: &mut Engine, port: NodeHandle, target: NodeHandle) {
        engine
            .set_property(port, StyleProperty::Position, Position::Relative.into())
            .unwrap();
        engine
            .set_property(target, StyleProperty::Position, Position::Absolute.into())
            .unwrap();
        for (property, value) in [
            (StyleProperty::Left, 150.0),
            (StyleProperty::Top, 120.0),
            (StyleProperty::MarginLeft, 0.0),
            (StyleProperty::MarginTop, 0.0),
        ] {
            engine
                .set_property(target, property, LengthValue::px(value).into())
                .unwrap();
        }
    }

    #[test]
    fn reveal_start_uses_native_margin_and_padding_with_border_and_transform() {
        for transformed in [false, true] {
            let (mut engine, port, target) = insets_fixture();
            engine
                .set_property(
                    port,
                    StyleProperty::Border,
                    openui_style::Border {
                        width: 4.0,
                        style: openui_style::BorderStyle::Solid,
                        color: openui_style::Color::BLACK,
                    }
                    .into(),
                )
                .unwrap();
            if transformed {
                engine
                    .set_property(
                        port,
                        StyleProperty::Transform,
                        openui_style::TransformList(vec![
                            openui_style::TransformOperation::Translate(
                                LengthValue::px(17.0),
                                LengthValue::px(9.0),
                            ),
                            openui_style::TransformOperation::Scale(1.25, 1.5),
                        ])
                        .into(),
                    )
                    .unwrap();
            }
            engine
                .scroll_into_view(
                    target,
                    ScrollIntoViewOptions {
                        block: ScrollAlignment::Start,
                        inline: ScrollAlignment::Start,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(engine.scroll_offset(port).unwrap(), (120.0, 80.0));
        }
    }

    #[test]
    fn reveal_nearest_and_accessibility_share_insets_and_minimum_move() {
        for accessibility in [false, true] {
            let (mut engine, port, target) = insets_fixture();
            for (node, property, value) in [
                (port, StyleProperty::ScrollPaddingRight, 5.0),
                (port, StyleProperty::ScrollPaddingBottom, 10.0),
                (target, StyleProperty::ScrollMarginRight, 9.0),
                (target, StyleProperty::ScrollMarginBottom, 7.0),
            ] {
                engine
                    .set_property(node, property, LengthValue::px(value).into())
                    .unwrap();
            }
            if accessibility {
                engine
                    .perform_accessibility_action(target, AccessibilityAction::ScrollIntoView)
                    .unwrap();
            } else {
                engine.scroll_into_view(target, nearest()).unwrap();
            }
            assert_eq!(engine.scroll_offset(port).unwrap(), (84.0, 77.0));
            let generation = engine.scene().unwrap().generation();
            let stats = engine.stats();
            engine.scroll_into_view(target, nearest()).unwrap();
            assert_eq!(generation, engine.scene().unwrap().generation());
            assert_eq!(stats, engine.stats());
        }
    }

    #[test]
    fn reveal_percentage_padding_and_degenerate_snapport_keep_layout_unit_policy() {
        let (mut engine, port, target) = insets_fixture();
        engine
            .set_property(
                port,
                StyleProperty::ScrollPaddingLeft,
                LengthValue::percent(20.0).into(),
            )
            .unwrap();
        engine
            .scroll_into_view(
                target,
                ScrollIntoViewOptions {
                    block: ScrollAlignment::Start,
                    inline: ScrollAlignment::Start,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(engine.scroll_offset(port).unwrap().0, 120.0);
        engine.scroll_to(port, 0.0, 0.0).unwrap();
        for property in [
            StyleProperty::ScrollPaddingLeft,
            StyleProperty::ScrollPaddingRight,
        ] {
            engine
                .set_property(port, property, LengthValue::px(80.0).into())
                .unwrap();
        }
        engine
            .scroll_into_view(
                target,
                ScrollIntoViewOptions {
                    block: ScrollAlignment::Start,
                    inline: ScrollAlignment::Center,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(engine.scroll_offset(port).unwrap().0, 75.0);
    }

    #[test]
    fn reveal_negative_margin_and_logical_end_use_physical_insets() {
        let (mut engine, port, target) = insets_fixture();
        absolute_target(&mut engine, port, target);
        engine
            .set_property(
                target,
                StyleProperty::ScrollMarginLeft,
                LengthValue::px(-5.0).into(),
            )
            .unwrap();
        engine
            .scroll_into_view(
                target,
                ScrollIntoViewOptions {
                    block: ScrollAlignment::Start,
                    inline: ScrollAlignment::Start,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(engine.scroll_offset(port).unwrap().0, 135.0);
        engine.scroll_to(port, 0.0, 0.0).unwrap();
        engine
            .set_property(target, StyleProperty::Direction, Direction::Rtl.into())
            .unwrap();
        engine
            .set_property(
                target,
                StyleProperty::ScrollMarginRight,
                LengthValue::px(9.0).into(),
            )
            .unwrap();
        engine
            .set_property(
                port,
                StyleProperty::ScrollPaddingRight,
                LengthValue::px(5.0).into(),
            )
            .unwrap();
        engine
            .scroll_into_view(
                target,
                ScrollIntoViewOptions {
                    block: ScrollAlignment::Start,
                    inline: ScrollAlignment::Start,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(engine.scroll_offset(port).unwrap().0, 84.0);
    }

    #[test]
    fn reveal_logical_alignment_uses_target_writing_mode_and_direction() {
        for (mode, direction, expected) in [
            (WritingMode::VerticalRl, Direction::Ltr, (84.0, 80.0)),
            (WritingMode::VerticalLr, Direction::Ltr, (120.0, 80.0)),
            (WritingMode::SidewaysLr, Direction::Ltr, (120.0, 77.0)),
            (WritingMode::VerticalRl, Direction::Rtl, (84.0, 77.0)),
        ] {
            let (mut engine, port, target) = insets_fixture();
            absolute_target(&mut engine, port, target);
            engine
                .set_property(
                    target,
                    StyleProperty::WritingMode,
                    openui_style::StyleValue::Renderer(
                        openui_style::RendererStyleValue::WritingMode(mode),
                    ),
                )
                .unwrap();
            engine
                .set_property(target, StyleProperty::Direction, direction.into())
                .unwrap();
            engine
                .set_property(
                    target,
                    StyleProperty::ScrollMarginRight,
                    LengthValue::px(9.0).into(),
                )
                .unwrap();
            engine
                .set_property(
                    target,
                    StyleProperty::ScrollMarginBottom,
                    LengthValue::px(7.0).into(),
                )
                .unwrap();
            engine
                .set_property(
                    port,
                    StyleProperty::ScrollPaddingRight,
                    LengthValue::px(5.0).into(),
                )
                .unwrap();
            engine
                .set_property(
                    port,
                    StyleProperty::ScrollPaddingBottom,
                    LengthValue::px(10.0).into(),
                )
                .unwrap();
            engine
                .scroll_into_view(
                    target,
                    ScrollIntoViewOptions {
                        block: ScrollAlignment::Start,
                        inline: ScrollAlignment::Start,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(engine.scroll_offset(port).unwrap(), expected);
        }
    }

    fn snap_corridor_fixture(
        candidate_height: f32,
    ) -> (Engine, NodeHandle, NodeHandle, NodeHandle) {
        let (mut engine, outer, inner, target) = nested();
        engine
            .set_property(inner, StyleProperty::Width, LengthValue::px(100.0).into())
            .unwrap();
        engine
            .set_property(inner, StyleProperty::Position, Position::Relative.into())
            .unwrap();
        let _tail = box_node(&mut engine, inner, 400.0, 300.0);
        for left in [0.0, 200.0] {
            let candidate = box_node(&mut engine, inner, 10.0, candidate_height);
            engine
                .set_property(
                    candidate,
                    StyleProperty::Position,
                    Position::Absolute.into(),
                )
                .unwrap();
            engine
                .set_property(candidate, StyleProperty::Left, LengthValue::px(left).into())
                .unwrap();
            engine
                .set_property(candidate, StyleProperty::Top, LengthValue::px(0.0).into())
                .unwrap();
            engine
                .set_property(
                    candidate,
                    StyleProperty::ScrollSnapAlign,
                    openui_style::StyleValue::Renderer(
                        openui_style::RendererStyleValue::ScrollSnapAlign(
                            openui_style::ScrollSnapAlign::Start,
                        ),
                    ),
                )
                .unwrap();
        }
        engine
            .set_property(
                inner,
                StyleProperty::ScrollSnapAxis,
                openui_style::StyleValue::Renderer(
                    openui_style::RendererStyleValue::ScrollSnapAxis(
                        openui_style::ScrollSnapAxis::X,
                    ),
                ),
            )
            .unwrap();
        engine
            .set_property(
                inner,
                StyleProperty::ScrollSnapStrictness,
                openui_style::StyleValue::Renderer(
                    openui_style::RendererStyleValue::ScrollSnapStrictness(
                        openui_style::ScrollSnapStrictness::Mandatory,
                    ),
                ),
            )
            .unwrap();
        (engine, outer, inner, target)
    }

    #[test]
    fn reveal_outer_geometry_uses_the_inner_snapped_endpoint() {
        // Both X snap areas span the content's Y corridor, so they remain
        // eligible after revealing the target at Y=40. Pinned Chromium .50
        // SnapSearchResult::visible_range requires this mutual visibility.
        let (mut engine, outer, inner, target) = snap_corridor_fixture(300.0);
        engine.scroll_into_view(target, nearest()).unwrap();
        assert_eq!(engine.scroll_offset(inner).unwrap().0, 0.0);
        assert_eq!(engine.scroll_offset(outer).unwrap().0, 70.0);
    }

    #[test]
    fn reveal_does_not_select_snap_areas_outside_its_cross_axis_corridor() {
        // The original 10px-high areas end at Y=10. Reveal's Y=40 endpoint
        // cannot select those X areas, even with mandatory X snapping.
        let (mut engine, outer, inner, target) = snap_corridor_fixture(10.0);
        engine.scroll_into_view(target, nearest()).unwrap();
        assert_eq!(engine.scroll_offset(inner).unwrap(), (70.0, 40.0));
        assert_eq!(engine.scroll_offset(outer).unwrap(), (0.0, 220.0));
    }

    #[test]
    fn reveal_nested_margin_survives_bubbling_and_padding_does_not_clip_target() {
        let (mut engine, outer, inner, target) = nested();
        let _tail = box_node(&mut engine, inner, 200.0, 300.0);
        for (node, property, value) in [
            (inner, StyleProperty::ScrollPaddingBottom, 15.0),
            (outer, StyleProperty::ScrollPaddingBottom, 10.0),
            (target, StyleProperty::ScrollMarginBottom, 7.0),
        ] {
            engine
                .set_property(node, property, LengthValue::px(value).into())
                .unwrap();
        }
        engine.scroll_into_view(target, nearest()).unwrap();
        assert_eq!(engine.scroll_offset(inner).unwrap().1, 62.0);
        assert_eq!(engine.scroll_offset(outer).unwrap().1, 215.0);
    }

    #[test]
    fn reveal_viewport_uses_retained_client_and_native_insets() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 80.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        engine
            .set_property(
                root,
                StyleProperty::ScrollbarWidth,
                openui_style::StyleValue::Renderer(
                    openui_style::RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
                ),
            )
            .unwrap();
        engine
            .set_property(root, StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        let target = box_node(&mut engine, root, 20.0, 20.0);
        absolute_target(&mut engine, root, target);
        let _tail = box_node(&mut engine, root, 400.0, 500.0);
        for (node, property, value) in [
            (root, StyleProperty::ScrollPaddingLeft, 20.0),
            (root, StyleProperty::ScrollPaddingTop, 25.0),
            (target, StyleProperty::ScrollMarginLeft, 10.0),
            (target, StyleProperty::ScrollMarginTop, 15.0),
        ] {
            engine
                .set_property(node, property, LengthValue::px(value).into())
                .unwrap();
        }
        engine
            .scroll_into_view(
                target,
                ScrollIntoViewOptions {
                    block: ScrollAlignment::Start,
                    inline: ScrollAlignment::Start,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(engine.scroll_offset(root).unwrap(), (120.0, 80.0));
    }

    #[test]
    fn reveal_smooth_and_instant_share_inset_destinations() {
        let (mut engine, port, target) = insets_fixture();
        let options = ScrollIntoViewOptions {
            block: ScrollAlignment::Start,
            inline: ScrollAlignment::Start,
            ..Default::default()
        };
        let ids = engine
            .smooth_scroll_into_view(target, options, 100.0)
            .unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(engine.scroll_offset(port).unwrap(), (0.0, 0.0));
        engine.set_animation_time(100.0).unwrap();
        assert_eq!(engine.scroll_offset(port).unwrap(), (120.0, 80.0));
        assert!(!engine.is_animating());
        engine.scroll_to(port, 0.0, 0.0).unwrap();
        engine.scroll_into_view(target, options).unwrap();
        assert_eq!(engine.scroll_offset(port).unwrap(), (120.0, 80.0));
    }

    #[test]
    fn reveal_fixed_and_absolute_targets_follow_used_containing_blocks() {
        for position in [Position::Fixed, Position::Absolute] {
            for captures in [false, true] {
                let (mut engine, outer, _, target) = nested();
                engine.append_or_move_child(outer, target).unwrap();
                engine
                    .set_property(target, StyleProperty::Position, position.into())
                    .unwrap();
                for (property, value) in [
                    (StyleProperty::Left, 150.0),
                    (StyleProperty::Top, 120.0),
                    (StyleProperty::MarginLeft, 0.0),
                    (StyleProperty::MarginTop, 0.0),
                ] {
                    engine
                        .set_property(target, property, LengthValue::px(value).into())
                        .unwrap();
                }
                engine
                    .set_property(outer, StyleProperty::Position, Position::Static.into())
                    .unwrap();
                if captures {
                    engine
                        .set_property(
                            outer,
                            StyleProperty::Transform,
                            openui_style::TransformList(vec![
                                openui_style::TransformOperation::Translate(
                                    LengthValue::px(7.0),
                                    LengthValue::px(3.0),
                                ),
                            ])
                            .into(),
                        )
                        .unwrap();
                }
                engine.scroll_into_view(target, nearest()).unwrap();
                assert_eq!(
                    engine.scroll_offset(outer).unwrap(),
                    if captures { (70.0, 60.0) } else { (0.0, 0.0) }
                );
                assert_eq!(engine.scroll_offset(engine.root()).unwrap(), (0.0, 0.0));
            }
        }
    }

    #[test]
    fn reveal_preserves_positive_fractional_target_size_and_oversized_nearest() {
        for (left, width) in [(99.75, 0.25), (0.0, 150.0)] {
            let (mut engine, port, target) = insets_fixture();
            for (node, property, value) in [
                (target, StyleProperty::Width, width),
                (target, StyleProperty::MarginLeft, left),
                (target, StyleProperty::MarginTop, 0.0),
                (target, StyleProperty::ScrollMarginLeft, 0.0),
                (target, StyleProperty::ScrollMarginTop, 0.0),
                (port, StyleProperty::ScrollPaddingLeft, 0.0),
                (port, StyleProperty::ScrollPaddingTop, 0.0),
            ] {
                engine
                    .set_property(node, property, LengthValue::px(value).into())
                    .unwrap();
            }
            engine.scroll_into_view(target, nearest()).unwrap();
            assert_eq!(engine.scroll_offset(port).unwrap(), (0.0, 0.0));
        }
    }

    #[test]
    fn reveal_captioned_table_preserves_offsets_under_authored_transform() {
        let (mut engine, port, target) = insets_fixture();
        let root = engine.root();
        let table = engine.create_native_element(ElementTag::Table).unwrap();
        engine
            .set_property(table, StyleProperty::Width, LengthValue::px(140.0).into())
            .unwrap();
        engine.append_child(root, table).unwrap();
        let caption = box_node(&mut engine, table, 140.0, 30.0);
        engine
            .set_property(
                caption,
                StyleProperty::Display,
                Display::TableCaption.into(),
            )
            .unwrap();
        let row = engine.create_native_element(ElementTag::Div).unwrap();
        engine
            .set_property(row, StyleProperty::Display, Display::TableRow.into())
            .unwrap();
        engine.append_child(table, row).unwrap();
        let cell = engine.create_native_element(ElementTag::Div).unwrap();
        engine
            .set_property(cell, StyleProperty::Display, Display::TableCell.into())
            .unwrap();
        engine.append_child(row, cell).unwrap();
        engine.append_or_move_child(cell, port).unwrap();
        let options = ScrollIntoViewOptions {
            block: ScrollAlignment::Start,
            inline: ScrollAlignment::Start,
            container: ScrollIntoViewContainer::Nearest,
        };
        engine.scroll_into_view(target, options).unwrap();
        let offset = engine.scroll_offset(port).unwrap();
        let bounds = engine.bounds(target).unwrap().unwrap();
        assert_eq!(offset, (120.0, 80.0));
        engine.scroll_to(port, 0.0, 0.0).unwrap();
        engine
            .set_property(
                table,
                StyleProperty::Transform,
                openui_style::TransformList(vec![openui_style::TransformOperation::Translate(
                    LengthValue::px(17.0),
                    LengthValue::px(9.0),
                )])
                .into(),
            )
            .unwrap();
        engine.scroll_into_view(target, options).unwrap();
        let translated = engine.bounds(target).unwrap().unwrap();
        assert_eq!(engine.scroll_offset(port).unwrap(), offset);
        assert_eq!(
            (translated.x - bounds.x, translated.y - bounds.y),
            (17.0, 9.0)
        );
    }
}

/// Which enclosing scroll containers a reveal operation may move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollIntoViewContainer {
    #[default]
    All,
    Nearest,
}

/// Native element reveal policy. The default aligns block-start and reveals
/// the nearest inline edge through all enclosing containers and the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollIntoViewOptions {
    pub block: ScrollAlignment,
    pub inline: ScrollAlignment,
    pub container: ScrollIntoViewContainer,
}

impl Default for ScrollIntoViewOptions {
    fn default() -> Self {
        Self {
            block: ScrollAlignment::Start,
            inline: ScrollAlignment::Nearest,
            container: ScrollIntoViewContainer::All,
        }
    }
}

fn map_rect(transform: WorldTransform, rect: SceneRect) -> SceneRect {
    transform
        .then(WorldTransform::translate(rect.x as f64, rect.y as f64))
        .map_rect(rect.width, rect.height)
}

fn alignment_delta(
    start: f32,
    end: f32,
    client_start: f32,
    client_end: f32,
    alignment: ScrollAlignment,
    reversed: bool,
) -> f64 {
    let low = (start - client_start) as f64;
    let high = (end - client_end) as f64;
    match alignment {
        ScrollAlignment::Start => {
            if reversed {
                high
            } else {
                low
            }
        }
        ScrollAlignment::End => {
            if reversed {
                low
            } else {
                high
            }
        }
        ScrollAlignment::Center => (low + high) / 2.0,
        ScrollAlignment::Nearest => {
            // Fully contained and oversized rectangles spanning both edges
            // already expose as much as this scrollport can show.
            if (low >= 0.0 && high <= 0.0) || (low < 0.0 && high > 0.0) {
                0.0
            } else if low < 0.0 {
                if end - start <= client_end - client_start {
                    low
                } else {
                    high
                }
            } else if high > 0.0 {
                if end - start <= client_end - client_start {
                    high
                } else {
                    low
                }
            } else {
                0.0
            }
        }
    }
}

impl Engine {
    /// Reveal an element using owned viewport and scrollport geometry. Pending
    /// layout is resolved before planning. Detached and unboxed elements are
    /// no-ops; stale and foreign handles remain errors.
    pub fn scroll_into_view(
        &mut self,
        target: NodeHandle,
        options: ScrollIntoViewOptions,
    ) -> Result<(), EngineError> {
        for (ancestor, x, y) in self.scroll_into_view_plan(target, options)? {
            self.scroll_to_resolved(ancestor, x as f32, y as f32)?;
        }
        Ok(())
    }

    /// Reveal through the same scroll plan using the retained animation clock
    /// and CSS ease curve. Reduced motion and a zero duration settle immediately.
    /// Returns owned animation IDs in inner-to-outer container order.
    pub fn smooth_scroll_into_view(
        &mut self,
        target: NodeHandle,
        options: ScrollIntoViewOptions,
        duration_ms: f64,
    ) -> Result<Vec<ScrollAnimationId>, EngineError> {
        self.smooth_scroll_into_view_with_easing(
            target,
            options,
            duration_ms,
            openui_style::Easing::CubicBezier {
                x1: 0.25,
                y1: 0.1,
                x2: 0.25,
                y2: 1.0,
            },
        )
    }

    /// Reveal through the shared plan with a caller-owned native easing curve.
    /// Validate timing and curve before planning or starting any animation.
    pub fn smooth_scroll_into_view_with_easing(
        &mut self,
        target: NodeHandle,
        options: ScrollIntoViewOptions,
        duration_ms: f64,
        easing: openui_style::Easing,
    ) -> Result<Vec<ScrollAnimationId>, EngineError> {
        if !duration_ms.is_finite() || duration_ms < 0.0 {
            return Err(EngineError::InvalidInput("invalid reveal duration"));
        }
        easing
            .validate()
            .map_err(|_| EngineError::InvalidInput("invalid reveal easing curve"))?;
        self.scroll_into_view_plan(target, options)?
            .into_iter()
            .map(|(ancestor, x, y)| {
                self.smooth_scroll_to(ancestor, x, y, duration_ms, easing.clone())
            })
            .collect()
    }

    fn reveal_fragment(&self, node: NodeId) -> Option<&Fragment> {
        fn find(fragment: &Fragment, node: NodeId) -> Option<&Fragment> {
            if fragment.owns_element_box() && fragment.node_id == node {
                return Some(fragment);
            }
            fragment.children.iter().find_map(|child| find(child, node))
        }
        self.latest_fragment
            .as_ref()
            .and_then(|root| find(root, node))
    }

    /// Client rectangles include the principal border-box origin, while the
    /// input world map already starts at that origin. Recover fragment space
    /// before comparing either the client rect or its padding-reduced snapport.
    fn reveal_scroll_geometry(&self, node: NodeId) -> Option<(ScrollArea, WorldTransform)> {
        let fragment = self.latest_fragment.as_ref()?.find_scroll_box(node)?;
        let area = fragment.scroll_area?;
        let authored = self.reveal_fragment(node)?;
        let principal = authored.border_box_rect().offset;
        let anonymous_offset = if fragment.anonymous_layout_box.is_some() {
            fragment.offset
        } else {
            PhysicalOffset::zero()
        };
        let world = self
            .fragment_box_worlds
            .get(&node)?
            .then(WorldTransform::translate(
                -principal.left.to_f64() + anonymous_offset.left.to_f64(),
                -principal.top.to_f64() + anonymous_offset.top.to_f64(),
            ));
        Some((area, world))
    }

    /// Blink bubbles reveal along containing blocks. Positioned descendants
    /// can bypass DOM ancestors that scrolling would not move; a viewport-fixed
    /// box must not request scrolling of that viewport.
    fn reveal_parent(&self, node: NodeId) -> Option<NodeId> {
        let source = self.document.node(node);
        if source.style.position.is_absolutely_positioned() {
            if let Some(positioned) = self
                .reveal_fragment(node)
                .and_then(|fragment| fragment.positioned_fragmentation)
            {
                let mut container = positioned.containing_block_node;
                if source.style.position == Position::Fixed && container == self.document.root() {
                    return None;
                }
                // A positioned inline supplies positioning geometry, but
                // InclusiveContainingBlock bubbles through its enclosing block.
                while !container.is_none()
                    && matches!(
                        self.document.node(container).style.display,
                        Display::Inline | Display::Contents
                    )
                {
                    container = self.document.node(container).parent;
                }
                if !container.is_none() && container != node {
                    return Some(container);
                }
            }
        }
        (!source.parent.is_none()).then_some(source.parent)
    }

    fn scroll_into_view_plan(
        &mut self,
        target: NodeHandle,
        options: ScrollIntoViewOptions,
    ) -> Result<Vec<(NodeHandle, f64, f64)>, EngineError> {
        let target_node = self.resolve(target)?;
        if !self.node_is_connected(target_node) {
            return Ok(Vec::new());
        }
        self.update()?;
        let Some(mut exposed) = self.node_bounds(target_node) else {
            return Ok(Vec::new());
        };
        // Blink resolves logical alignment using the target's writing mode.
        let style = &self.document.node(target_node).style;
        let margin = crate::scroll_button::scroll_margin(style);
        let horizontal = style.writing_mode.is_horizontal();
        let inline_reversed =
            (style.direction == Direction::Rtl) ^ (style.writing_mode == WritingMode::SidewaysLr);
        let (x_align, y_align, x_reversed, y_reversed) = if horizontal {
            (options.inline, options.block, inline_reversed, false)
        } else {
            (
                options.block,
                options.inline,
                style.writing_mode.is_flipped_blocks(),
                inline_reversed,
            )
        };
        // Chromium gives degenerate expose rectangles one logical pixel.
        if exposed.width <= 0.0 {
            exposed.width = 1.0;
        }
        if exposed.height <= 0.0 {
            exposed.height = 1.0;
        }
        let mut plan = Vec::new();
        let mut ancestor = self.reveal_parent(target_node);
        while let Some(node) = ancestor {
            ancestor = self.reveal_parent(node);
            let Some(index) = self.node_slots.get(&node).copied() else {
                continue;
            };
            let handle = self.handle_for_slot(index);
            if self
                .latest_fragment
                .as_ref()
                .and_then(|root| root.find_scroll_box(node))
                .is_some_and(|box_| {
                    box_.anonymous_layout_box.is_some() && !box_.contains_element_box(target_node)
                })
            {
                continue;
            }
            let Some((area, world)) = self.reveal_scroll_geometry(node) else {
                continue;
            };
            if !area.is_scroll_container() {
                continue;
            }
            // A previous intersection can also be degenerate.
            if exposed.width <= 0.0 {
                exposed.width = 1.0;
            }
            if exposed.height <= 0.0 {
                exposed.height = 1.0;
            }
            // Blink's inverse TransformState uses ProjectQuad, not a depth=0
            // forward MapQuad. This intersects the transformed source plane.
            // Failed inversion leaves the planar quad unchanged in Blink.
            let local = crate::hit_geometry::enclosing_layout_rect(
                world
                    .inverse()
                    .map_or(exposed, |inverse| inverse.project_rect(exposed)),
            );
            let client = area.client_rect;
            let mut snapport =
                crate::scroll_button::scroll_snapport(self.retained_scroll_style(node), area);
            // GetScrollOffsetToExpose uses one raw LayoutUnit, rather than one
            // CSS pixel, when scroll-padding leaves no positive snapport size.
            snapport[2] = snapport[2].max(1.0 / 64.0);
            snapport[3] = snapport[3].max(1.0 / 64.0);
            let (x, y) = self.scroll_offset(handle)?;
            let (new_x, new_y) = area.clamp_offset(
                (x + alignment_delta(
                    local.x - margin[0],
                    local.x + local.width + margin[2],
                    snapport[0],
                    snapport[0] + snapport[2],
                    x_align,
                    x_reversed,
                ))
                .round(),
                (y + alignment_delta(
                    local.y - margin[1],
                    local.y + local.height + margin[3],
                    snapport[1],
                    snapport[1] + snapport[3],
                    y_align,
                    y_reversed,
                ))
                .round(),
            );
            // Native endpoint snapping is part of the reveal destination, so
            // outer containers must see its final position too.
            let (new_x, new_y) = self.resolve_native_reveal_endpoint(handle, new_x, new_y)?;
            // Propagate the final visible rectangle. Outer containers must
            // reveal the part exposed by the inner scrollport, including when
            // the inner move is scheduled as an animation rather than applied.
            // Padding affects alignment, but neither it nor the target margin
            // clips this unexpanded rectangle passed to the next scrollport.
            let shifted = SceneRect {
                x: local.x - (new_x as f64 - x) as f32,
                y: local.y - (new_y as f64 - y) as f32,
                ..local
            };
            let left = shifted.x.max(client.x().to_f32());
            let top = shifted.y.max(client.y().to_f32());
            let right = (shifted.x + shifted.width).min(client.right().to_f32());
            let bottom = (shifted.y + shifted.height).min(client.bottom().to_f32());
            exposed = map_rect(
                world,
                if right > left && bottom > top {
                    SceneRect {
                        x: left,
                        y: top,
                        width: right - left,
                        height: bottom - top,
                    }
                } else {
                    shifted
                },
            );
            // Include an unchanged target when a smooth scroll is already
            // running so an explicit reveal replaces that pending destination.
            if x != new_x as f64
                || y != new_y as f64
                || self
                    .scroll_animations
                    .values()
                    .any(|animation| animation.target == handle)
            {
                plan.push((handle, new_x as f64, new_y as f64));
            }
            if options.container == ScrollIntoViewContainer::Nearest {
                break;
            }
        }
        Ok(plan)
    }
}
