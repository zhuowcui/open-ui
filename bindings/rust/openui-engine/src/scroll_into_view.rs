//! Native reveal operations over the retained fragment geometry.

use crate::{Affine, Engine, EngineError, NodeHandle, ScrollAnimationId};
use openui_compositor::SceneRect;
use openui_style::{Direction, Overflow, WritingMode};

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
    use openui_style::{LengthValue, Position, ScrollbarWidth, StyleProperty};

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

fn map_rect(transform: Affine, rect: SceneRect) -> SceneRect {
    transform
        .then(Affine::translate(rect.x, rect.y))
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
            self.scroll_to(ancestor, x, y)?;
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
        if !duration_ms.is_finite() || duration_ms < 0.0 {
            return Err(EngineError::InvalidInput(
                "scroll duration must be finite and non-negative",
            ));
        }
        self.scroll_into_view_plan(target, options)?
            .into_iter()
            .map(|(ancestor, x, y)| {
                self.smooth_scroll_to(
                    ancestor,
                    x,
                    y,
                    duration_ms,
                    openui_style::Easing::CubicBezier {
                        x1: 0.25,
                        y1: 0.1,
                        x2: 0.25,
                        y2: 1.0,
                    },
                )
            })
            .collect()
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
        exposed.width = exposed.width.max(1.0);
        exposed.height = exposed.height.max(1.0);
        let mut plan = Vec::new();
        let mut ancestor = self.parent(target)?;
        while let Some(handle) = ancestor {
            let node = self.resolve(handle)?;
            ancestor = self.parent(handle)?;
            let Some(area) = self.layout_scroll_area(node) else {
                continue;
            };
            if matches!(area.overflow_x, Overflow::Visible | Overflow::Clip)
                && matches!(area.overflow_y, Overflow::Visible | Overflow::Clip)
            {
                continue;
            }
            let Some(world) = self.fragment_box_worlds.get(&node).copied() else {
                continue;
            };
            let Some(inverse) = world.inverse() else {
                continue;
            };
            let local = map_rect(inverse, exposed);
            let client = area.client_rect;
            let (x, y) = self.scroll_offset(handle)?;
            let (new_x, new_y) = area.clamp_offset(
                (x + alignment_delta(
                    local.x,
                    local.x + local.width,
                    client.x().to_f32(),
                    client.right().to_f32(),
                    x_align,
                    x_reversed,
                ))
                .round(),
                (y + alignment_delta(
                    local.y,
                    local.y + local.height,
                    client.y().to_f32(),
                    client.bottom().to_f32(),
                    y_align,
                    y_reversed,
                ))
                .round(),
            );
            // Propagate the final visible rectangle. Outer containers must
            // reveal the part exposed by the inner scrollport, including when
            // the inner move is scheduled as an animation rather than applied.
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
