//! Full CSS 2.1 §8.3.1 margin collapsing — orchestration layer.
//!
//! This module provides the stateful logic that drives margin collapsing
//! during block layout. It builds on top of the low-level `MarginStrut`
//! (in `openui-geometry`) which tracks the largest positive and most-negative
//! margins. This module decides *when* and *how* those struts get combined,
//! implementing every rule from CSS 2.1 §8.3.1:
//!
//! - Adjacent sibling collapsing
//! - Parent–first-child collapsing
//! - Parent–last-child collapsing
//! - Empty block collapse-through
//! - BFC / float / clearance exclusions
//! - Negative margin arithmetic
//!
//! Source: CSS 2.1 §8.3.1 + Blink's `BlockLayoutAlgorithm::HandleMargin*`
//! and `MarginStrut`.

use openui_geometry::{BoxStrut, LayoutUnit, MarginStrut};
use openui_style::{Clear, Display, Float, Overflow, Position};

// ── Core state ──────────────────────────────────────────────────────────

/// Tracks margin collapsing state during a single block layout pass.
///
/// Created at the start of `block_layout`, mutated as each child is laid
/// out, and consumed at the end to produce the final margin strut that
/// propagates to the parent.
#[derive(Debug, Clone)]
pub struct MarginCollapsingState {
    /// Current margin strut being accumulated. As we walk through children,
    /// margins are appended here. When we hit a non-collapsible boundary
    /// the strut is resolved and a new one begins.
    pub margin_strut: MarginStrut,

    /// Once the BFC block-offset is resolved (e.g. because a non-empty
    /// child or a float forced it), margin propagation to the parent stops.
    pub bfc_offset_resolved: bool,

    /// True when the current element has been determined to be an "empty
    /// block" — no height, no border, no padding, no in-flow content.
    /// In that case its top and bottom margins collapse through.
    pub is_empty_block: bool,

    /// True when the most recently processed child collapsed through.
    /// When set, the next sibling's top margin adjoins the collapsed-through
    /// child's margins — the strut must NOT be resolved between them.
    pub previous_child_collapsed_through: bool,
}

impl MarginCollapsingState {
    #[inline]
    pub fn new() -> Self {
        Self {
            margin_strut: MarginStrut::new(),
            bfc_offset_resolved: false,
            is_empty_block: false,
            previous_child_collapsed_through: false,
        }
    }

    /// Create a state whose BFC offset is already resolved (e.g. root
    /// element, or a block that itself establishes a new BFC).
    #[inline]
    pub fn new_resolved() -> Self {
        Self {
            margin_strut: MarginStrut::new(),
            bfc_offset_resolved: true,
            is_empty_block: false,
            previous_child_collapsed_through: false,
        }
    }
}

impl Default for MarginCollapsingState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Resolve a MarginStrut into a single collapsed value ─────────────────

/// Compute the collapsed margin value from a `MarginStrut`.
///
/// CSS 2.1 §8.3.1 rules:
/// - Both positive → max(positive)           (tracked by MarginStrut)
/// - Both negative → min(negative)           (tracked by MarginStrut)
/// - Mixed         → max_positive + min_negative (they add; negative is stored
///   as a negative value already)
///
/// `MarginStrut::sum()` already implements this, but we provide a free function
/// for clarity in the collapsing pipeline.
#[inline]
pub fn collapse_margins(strut: &MarginStrut) -> LayoutUnit {
    strut.sum()
}

/// Same as `collapse_margins` — named to match the task specification.
#[inline]
pub fn adjoining_margin_resolve(strut: &MarginStrut) -> LayoutUnit {
    if strut.discard_margins {
        return LayoutUnit::zero();
    }
    let positive = if strut.quirky_positive_margin > strut.positive_margin {
        strut.quirky_positive_margin
    } else {
        strut.positive_margin
    };
    // negative_margin is already ≤ 0
    positive + strut.negative_margin
}

// ── Collapse-through detection ──────────────────────────────────────────

/// Parameters describing an element for the "should collapse through" check.
#[derive(Debug, Clone)]
pub struct CollapseCheckParams {
    pub border: BoxStrut,
    pub padding: BoxStrut,
    /// Resolved block-size (height). `None` means `auto`.
    pub block_size: Option<LayoutUnit>,
    /// Resolved min-block-size. Defaults to zero.
    pub min_block_size: LayoutUnit,
    /// Number of in-flow children (block or inline).
    pub in_flow_child_count: u32,
    /// Whether the element has any line boxes (text content).
    pub has_line_boxes: bool,
}

/// Determine whether an element is "empty" for the purpose of margin
/// collapsing (CSS 2.1 §8.3.1 rule 4).
///
/// An empty block collapses its own top and bottom margins together.
/// The element is empty when it has:
/// - No border-top or border-bottom
/// - No padding-top or padding-bottom
/// - No in-flow children
/// - No line boxes
/// - No height (block_size == 0 or auto) with min-height == 0
pub fn should_margins_collapse_through(params: &CollapseCheckParams) -> bool {
    // Border prevents collapse-through.
    if params.border.top.raw() != 0 || params.border.bottom.raw() != 0 {
        return false;
    }

    // Padding prevents collapse-through.
    if params.padding.top.raw() != 0 || params.padding.bottom.raw() != 0 {
        return false;
    }

    // min-height > 0 prevents collapse-through.
    if params.min_block_size.raw() > 0 {
        return false;
    }

    // Explicit non-zero height prevents collapse-through.
    if let Some(h) = params.block_size {
        if h.raw() > 0 {
            return false;
        }
    }

    // In-flow children prevent collapse-through.
    if params.in_flow_child_count > 0 {
        return false;
    }

    // Line boxes prevent collapse-through.
    if params.has_line_boxes {
        return false;
    }

    true
}

// ── Child margin processing ─────────────────────────────────────────────

/// Properties of a child element relevant to margin collapsing decisions.
#[derive(Debug, Clone)]
pub struct ChildMarginInfo {
    /// Resolved top margin of the child.
    pub margin_top: LayoutUnit,
    /// Resolved bottom margin of the child.
    pub margin_bottom: LayoutUnit,
    /// Whether the child establishes a new BFC.
    pub establishes_bfc: bool,
    /// Whether the child is a float.
    pub is_float: bool,
    /// Whether the child has clearance.
    pub has_clearance: bool,
    /// Whether the child collapsed through (empty block).
    pub collapsed_through: bool,
    /// The child's own margin strut (propagated from its layout).
    pub child_margin_strut: MarginStrut,
}

/// Properties of the parent element relevant to margin collapsing.
#[derive(Debug, Clone)]
pub struct ParentMarginInfo {
    /// Parent's border box.
    pub border: BoxStrut,
    /// Parent's padding box.
    pub padding: BoxStrut,
    /// Whether this is the first in-flow child.
    pub is_first_child: bool,
    /// Whether this is the last in-flow child.
    pub is_last_child: bool,
    /// Parent's resolved block-size. `None` means `auto`.
    pub block_size: Option<LayoutUnit>,
}

/// Process margins *before* laying out an in-flow child.
///
/// This implements parent–first-child collapsing (CSS 2.1 §8.3.1 rule 2):
/// if the child is the first in-flow child and there is no border or padding
/// separating them, the child's top margin collapses with the parent's top
/// margin (which is stored in `state.margin_strut`).
///
/// Returns the block-offset contribution from the resolved margin strut
/// (zero if margins are still collapsing upward).
pub fn handle_margin_before_child(
    state: &mut MarginCollapsingState,
    child: &ChildMarginInfo,
    parent: &ParentMarginInfo,
) -> LayoutUnit {
    // Floats never participate in margin collapsing.
    if child.is_float {
        return LayoutUnit::zero();
    }

    // New BFC children don't collapse with their parent.
    if child.establishes_bfc {
        // Resolve any pending strut first.
        let resolved = collapse_margins(&state.margin_strut);
        state.margin_strut = MarginStrut::new();
        state.margin_strut.append_normal(child.margin_top);
        state.bfc_offset_resolved = true;
        return resolved;
    }

    // Clearance severs the margin collapsing chain.
    if child.has_clearance {
        let resolved = collapse_margins(&state.margin_strut);
        state.margin_strut = MarginStrut::new();
        state.margin_strut.append_normal(child.margin_top);
        state.bfc_offset_resolved = true;
        return resolved;
    }

    // First child: check parent-child collapsing.
    if parent.is_first_child && !state.bfc_offset_resolved {
        let has_separating_border = parent.border.top.raw() != 0;
        let has_separating_padding = parent.padding.top.raw() != 0;

        if has_separating_border || has_separating_padding {
            // Border or padding breaks the collapsing — resolve the strut.
            let resolved = collapse_margins(&state.margin_strut);
            state.margin_strut = MarginStrut::new();
            state.margin_strut.append_normal(child.margin_top);
            state.bfc_offset_resolved = true;
            return resolved;
        }

        // No separator — child's top margin collapses with parent's.
        state.margin_strut.append_normal(child.margin_top);
        return LayoutUnit::zero();
    }

    // Subsequent siblings: the strut contains the previous sibling's bottom
    // margin. The current child's top margin collapses with it — append
    // first, then resolve.
    //
    // Exception: if the previous sibling collapsed through, its margins are
    // still adjoining — keep accumulating without resolving.
    if state.previous_child_collapsed_through {
        state.margin_strut.append_normal(child.margin_top);
        return LayoutUnit::zero();
    }

    state.margin_strut.append_normal(child.margin_top);
    let resolved = collapse_margins(&state.margin_strut);
    state.margin_strut = MarginStrut::new();
    state.bfc_offset_resolved = true;
    resolved
}

/// Process margins *after* a child has been laid out.
///
/// Handles:
/// - Appending the child's bottom margin to the current strut
/// - Empty block collapse-through (rule 4)
/// - Sibling margin accumulation for the next child
pub fn handle_margin_after_child(
    state: &mut MarginCollapsingState,
    child: &ChildMarginInfo,
) {
    // Floats don't participate.
    if child.is_float {
        return;
    }

    // If the child established a new BFC, its margins don't collapse outward.
    if child.establishes_bfc {
        // Start a new strut with only the child's bottom margin.
        state.margin_strut = MarginStrut::new();
        state.margin_strut.append_normal(child.margin_bottom);
        state.previous_child_collapsed_through = false;
        return;
    }

    if child.collapsed_through {
        // The child is an empty block — its top and bottom margins have
        // already been merged in its own strut. Append the child's combined
        // strut to ours (the child's own strut carries both top+bottom).
        let child_strut = &child.child_margin_strut;
        state.margin_strut.append_normal(child_strut.positive_margin);
        if child_strut.negative_margin.raw() < 0 {
            state.margin_strut.append_normal(child_strut.negative_margin);
        }
        state.previous_child_collapsed_through = true;
        return;
    }

    // Normal child: append its bottom margin.
    state.margin_strut.append_normal(child.margin_bottom);
    state.previous_child_collapsed_through = false;
}

// ── Finalize ────────────────────────────────────────────────────────────

/// Finalize margin collapsing at the end of a block's layout.
///
/// Handles:
/// - Parent–last-child collapsing (CSS 2.1 §8.3.1 rule 3)
/// - Empty block self-collapsing (rule 4)
///
/// Returns a tuple of:
/// - `resolved_bottom`: the margin contribution at the bottom of the block's
///   content box (zero if the margin propagates to the parent)
/// - `propagated_strut`: the margin strut that the parent should incorporate
pub fn finalize_margins(
    state: &mut MarginCollapsingState,
    parent: &ParentMarginInfo,
    parent_margin_bottom: LayoutUnit,
    is_empty: bool,
) -> (LayoutUnit, MarginStrut) {
    if is_empty {
        // Empty block: top and bottom margins collapse through.
        // The strut already has top margins; append the bottom margin.
        state.margin_strut.append_normal(parent_margin_bottom);
        return (LayoutUnit::zero(), state.margin_strut);
    }

    // Check parent–last-child collapsing (rule 3):
    // Parent's bottom margin collapses with last child's bottom margin if
    // there is no border-bottom, padding-bottom, or explicit height separating them.
    let has_separating_border = parent.border.bottom.raw() != 0;
    let has_separating_padding = parent.padding.bottom.raw() != 0;
    let has_explicit_height = match parent.block_size {
        Some(h) if h.raw() > 0 => true,
        _ => false,
    };

    if !has_separating_border && !has_separating_padding && !has_explicit_height {
        // Margins collapse: propagate the strut (including last child's
        // bottom margin) to the parent, appending the parent's own bottom
        // margin.
        state.margin_strut.append_normal(parent_margin_bottom);
        (LayoutUnit::zero(), state.margin_strut)
    } else {
        // Something separates them: resolve the strut here.
        let resolved = collapse_margins(&state.margin_strut);
        let mut propagated = MarginStrut::new();
        propagated.append_normal(parent_margin_bottom);
        (resolved, propagated)
    }
}

// ── Predicates: should margins collapse? ────────────────────────────────

/// Check whether an element establishes a new block formatting context,
/// which prevents its margins from collapsing with its children.
///
/// CSS 2.1 §8.3.1: margins of elements that establish new BFCs do not
/// collapse with their in-flow children.
pub fn establishes_new_bfc_for_collapsing(
    display: Display,
    overflow_x: Overflow,
    overflow_y: Overflow,
    float: Float,
    position: Position,
) -> bool {
    // Absolutely positioned elements establish a new BFC.
    if position.is_absolutely_positioned() {
        return true;
    }

    // Floats establish a new BFC.
    if float != Float::None {
        return true;
    }

    // overflow != visible on either axis establishes a new BFC.
    if overflow_x != Overflow::Visible || overflow_y != Overflow::Visible {
        return true;
    }

    // Certain display values always establish a new formatting context.
    if display.is_new_formatting_context() {
        return true;
    }

    false
}

/// Check whether margins between a float and any other box should collapse.
///
/// CSS 2.1 §8.3.1: Margins between a floated box and any other box do not
/// collapse.
#[inline]
pub fn float_prevents_collapsing(float: Float) -> bool {
    float != Float::None
}

/// Check whether clearance prevents margin collapsing.
///
/// CSS 2.1 §8.3.1: Margins of an element that has clearance do not collapse
/// with its parent's bottom margin.
#[inline]
pub fn clearance_prevents_collapsing(clear: Clear) -> bool {
    clear != Clear::None
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Merge two `MarginStrut`s — used when accumulating struts from adjacent
/// siblings or when an empty block's strut needs to be absorbed.
pub fn merge_struts(target: &mut MarginStrut, source: &MarginStrut) {
    target.append_normal(source.positive_margin);
    if source.negative_margin.raw() < 0 {
        target.append_normal(source.negative_margin);
    }
    // Also pick up quirky positive if larger.
    if source.quirky_positive_margin > target.quirky_positive_margin {
        target.quirky_positive_margin = source.quirky_positive_margin;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(v: i32) -> LayoutUnit {
        LayoutUnit::from_i32(v)
    }

    // ── adjoining_margin_resolve / collapse_margins ─────────────────

    #[test]
    fn resolve_positive_only() {
        let mut strut = MarginStrut::new();
        strut.append_normal(lu(10));
        strut.append_normal(lu(20));
        assert_eq!(adjoining_margin_resolve(&strut), lu(20));
    }

    #[test]
    fn resolve_negative_only() {
        let mut strut = MarginStrut::new();
        strut.append_normal(lu(-5));
        strut.append_normal(lu(-15));
        assert_eq!(adjoining_margin_resolve(&strut), lu(-15));
    }

    #[test]
    fn resolve_mixed() {
        let mut strut = MarginStrut::new();
        strut.append_normal(lu(30));
        strut.append_normal(lu(-10));
        // max_positive(30) + min_negative(-10) = 20
        assert_eq!(adjoining_margin_resolve(&strut), lu(20));
    }

    #[test]
    fn resolve_zero() {
        let strut = MarginStrut::new();
        assert_eq!(adjoining_margin_resolve(&strut), lu(0));
    }

    // ── should_margins_collapse_through ─────────────────────────────

    #[test]
    fn empty_block_collapses_through() {
        let params = CollapseCheckParams {
            border: BoxStrut::zero(),
            padding: BoxStrut::zero(),
            block_size: None,
            min_block_size: lu(0),
            in_flow_child_count: 0,
            has_line_boxes: false,
        };
        assert!(should_margins_collapse_through(&params));
    }

    #[test]
    fn empty_block_with_zero_height_collapses() {
        let params = CollapseCheckParams {
            border: BoxStrut::zero(),
            padding: BoxStrut::zero(),
            block_size: Some(lu(0)),
            min_block_size: lu(0),
            in_flow_child_count: 0,
            has_line_boxes: false,
        };
        assert!(should_margins_collapse_through(&params));
    }

    #[test]
    fn block_with_border_top_no_collapse() {
        let mut border = BoxStrut::zero();
        border.top = lu(1);
        let params = CollapseCheckParams {
            border,
            padding: BoxStrut::zero(),
            block_size: None,
            min_block_size: lu(0),
            in_flow_child_count: 0,
            has_line_boxes: false,
        };
        assert!(!should_margins_collapse_through(&params));
    }

    #[test]
    fn block_with_border_bottom_no_collapse() {
        let mut border = BoxStrut::zero();
        border.bottom = lu(2);
        let params = CollapseCheckParams {
            border,
            padding: BoxStrut::zero(),
            block_size: None,
            min_block_size: lu(0),
            in_flow_child_count: 0,
            has_line_boxes: false,
        };
        assert!(!should_margins_collapse_through(&params));
    }

    #[test]
    fn block_with_min_height_no_collapse() {
        let params = CollapseCheckParams {
            border: BoxStrut::zero(),
            padding: BoxStrut::zero(),
            block_size: None,
            min_block_size: lu(10),
            in_flow_child_count: 0,
            has_line_boxes: false,
        };
        assert!(!should_margins_collapse_through(&params));
    }

    #[test]
    fn block_with_children_no_collapse() {
        let params = CollapseCheckParams {
            border: BoxStrut::zero(),
            padding: BoxStrut::zero(),
            block_size: None,
            min_block_size: lu(0),
            in_flow_child_count: 1,
            has_line_boxes: false,
        };
        assert!(!should_margins_collapse_through(&params));
    }

    #[test]
    fn block_with_line_boxes_no_collapse() {
        let params = CollapseCheckParams {
            border: BoxStrut::zero(),
            padding: BoxStrut::zero(),
            block_size: None,
            min_block_size: lu(0),
            in_flow_child_count: 0,
            has_line_boxes: true,
        };
        assert!(!should_margins_collapse_through(&params));
    }

    // ── establishes_new_bfc_for_collapsing ──────────────────────────

    #[test]
    fn normal_block_no_bfc() {
        assert!(!establishes_new_bfc_for_collapsing(
            Display::Block,
            Overflow::Visible,
            Overflow::Visible,
            Float::None,
            Position::Static,
        ));
    }

    #[test]
    fn overflow_hidden_is_bfc() {
        assert!(establishes_new_bfc_for_collapsing(
            Display::Block,
            Overflow::Hidden,
            Overflow::Visible,
            Float::None,
            Position::Static,
        ));
    }

    #[test]
    fn float_is_bfc() {
        assert!(establishes_new_bfc_for_collapsing(
            Display::Block,
            Overflow::Visible,
            Overflow::Visible,
            Float::Left,
            Position::Static,
        ));
    }

    #[test]
    fn absolute_is_bfc() {
        assert!(establishes_new_bfc_for_collapsing(
            Display::Block,
            Overflow::Visible,
            Overflow::Visible,
            Float::None,
            Position::Absolute,
        ));
    }

    #[test]
    fn flex_is_bfc() {
        assert!(establishes_new_bfc_for_collapsing(
            Display::Flex,
            Overflow::Visible,
            Overflow::Visible,
            Float::None,
            Position::Static,
        ));
    }

    #[test]
    fn flow_root_is_bfc() {
        assert!(establishes_new_bfc_for_collapsing(
            Display::FlowRoot,
            Overflow::Visible,
            Overflow::Visible,
            Float::None,
            Position::Static,
        ));
    }

    #[test]
    fn inline_block_is_bfc() {
        assert!(establishes_new_bfc_for_collapsing(
            Display::InlineBlock,
            Overflow::Visible,
            Overflow::Visible,
            Float::None,
            Position::Static,
        ));
    }
}
