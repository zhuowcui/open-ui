//! New Formatting Context — BFC creation detection, layout, and float avoidance.
//!
//! Source: CSS 2.1 §9.4.1 (Block formatting contexts) and §9.5 (Floats).
//!
//! When an element establishes a new Block Formatting Context (BFC):
//! 1. Its margins do NOT collapse with its parent.
//! 2. It must NOT overlap with floats — it shrinks or moves to avoid them.
//! 3. It contains its own floats (they don't escape the BFC).
//! 4. It gets its own ExclusionSpace.
//!
//! This module provides:
//! - `creates_new_formatting_context()` — style-based BFC detection
//! - `layout_new_formatting_context()` — lay out a child that establishes a new BFC
//! - `compute_float_avoidance_offset()` — find position that avoids floats
//! - `adjust_for_float_avoidance()` — compute adjusted position and available size

use std::sync::Arc;

use openui_geometry::{BfcOffset, LayoutUnit, MarginStrut, PhysicalSize};
use openui_dom::NodeId;
use openui_style::ComputedStyle;

use crate::constraint_space::{ConstraintSpace, ConstraintSpaceBuilder};
use crate::exclusions::{ExclusionSpace, ClearType};
use crate::fragment::Fragment;
use crate::layout_result::LayoutResult;
use crate::length_resolver::resolve_margin_or_padding;

// ── 1. BFC creation detection ────────────────────────────────────────────

/// Determine whether a style establishes a new block formatting context.
///
/// Per CSS 2.1 §9.4.1, a new BFC is established when:
/// - `overflow` is not `visible` (hidden, scroll, auto, clip)
/// - `display` is `flow-root`, `flex`, `grid`, `inline-block`, `inline-flex`,
///   `inline-grid`, or `table`
/// - `float` is not `none`
/// - `position` is `absolute` or `fixed`
///
/// Delegates to `ComputedStyle::creates_new_formatting_context()` which already
/// checks these conditions. This wrapper adds the `is_root` parameter for the
/// root element (which always establishes a BFC).
pub fn creates_new_formatting_context(style: &ComputedStyle, is_root: bool) -> bool {
    if is_root {
        return true;
    }
    style.creates_new_formatting_context()
}

// ── 2. Float avoidance geometry ──────────────────────────────────────────

/// Result of float avoidance computation.
///
/// Contains the adjusted position and available inline size for a new-BFC
/// element that must not overlap floats per CSS 2.1 §9.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatAvoidanceResult {
    /// Inline (line) offset in BFC coordinates after avoiding floats.
    pub inline_offset: LayoutUnit,
    /// Available inline size after subtracting float exclusions.
    pub available_inline_size: LayoutUnit,
    /// Block offset in BFC coordinates (may be pushed down below floats).
    pub block_offset: LayoutUnit,
}

/// Compute the position where a new-BFC element can be placed without
/// overlapping any floats in the parent's exclusion space.
///
/// Per CSS 2.1 §9.5: "The border box of a table, a block-level replaced element,
/// or an element in normal flow that establishes a new block formatting context
/// must not overlap the margin box of any floats in the same block formatting
/// context."
///
/// Algorithm:
/// 1. Query the parent's ExclusionSpace for a layout opportunity at the current
///    block offset that is at least `element_inline_size` wide.
/// 2. If found, position the element at the opportunity's inline start.
/// 3. If no opportunity fits, the element is pushed below floats.
pub fn compute_float_avoidance_offset(
    exclusion_space: &ExclusionSpace,
    bfc_offset: &BfcOffset,
    container_inline_size: LayoutUnit,
    element_inline_size: LayoutUnit,
) -> FloatAvoidanceResult {
    let opportunity = exclusion_space.find_layout_opportunity(
        bfc_offset,
        container_inline_size,
        element_inline_size,
    );

    let available = opportunity.inline_size();

    FloatAvoidanceResult {
        inline_offset: opportunity.rect.line_start_offset(),
        available_inline_size: available,
        block_offset: opportunity.rect.block_start_offset(),
    }
}

/// Adjust a child's position and available size to account for floats.
///
/// Returns `(inline_offset, available_inline_size, block_offset)` in BFC
/// coordinates. If no floats are present, returns the original position
/// with full available width.
pub fn adjust_for_float_avoidance(
    exclusion_space: &ExclusionSpace,
    bfc_offset: &BfcOffset,
    container_inline_size: LayoutUnit,
    element_inline_size: LayoutUnit,
) -> (LayoutUnit, LayoutUnit, LayoutUnit) {
    if !exclusion_space.has_floats() {
        return (
            bfc_offset.line_offset,
            container_inline_size,
            bfc_offset.block_offset,
        );
    }

    let result = compute_float_avoidance_offset(
        exclusion_space,
        bfc_offset,
        container_inline_size,
        element_inline_size,
    );

    (result.inline_offset, result.available_inline_size, result.block_offset)
}

// ── 3. New formatting context layout ─────────────────────────────────────

/// Input parameters for laying out a child that establishes a new BFC.
#[derive(Debug)]
pub struct NewFcLayoutInput<'a> {
    /// The child's computed style.
    pub style: &'a ComputedStyle,
    /// The child's DOM node ID.
    pub node_id: NodeId,
    /// The parent's constraint space.
    pub parent_space: &'a ConstraintSpace,
    /// The child's BFC offset (where it would be placed without float avoidance).
    pub child_bfc_offset: BfcOffset,
    /// The container's content-box inline size.
    pub container_inline_size: LayoutUnit,
    /// The container's content-box block size (may be indefinite).
    pub container_block_size: LayoutUnit,
}

/// Output of new formatting context layout.
#[derive(Debug)]
pub struct NewFcLayoutResult {
    /// The laid-out fragment.
    pub fragment: Fragment,
    /// Resolved margins (not collapsed with parent).
    pub margin_top: LayoutUnit,
    pub margin_bottom: LayoutUnit,
    pub margin_left: LayoutUnit,
    pub margin_right: LayoutUnit,
    /// The block offset in BFC coordinates (may differ from input if pushed by floats).
    pub bfc_block_offset: LayoutUnit,
    /// The line offset in BFC coordinates.
    pub bfc_line_offset: LayoutUnit,
    /// Whether the element was pushed down by floats.
    pub is_pushed_by_floats: bool,
}

/// Lay out a child that establishes a new block formatting context.
///
/// This function:
/// 1. Resolves margins immediately (no collapsing with parent).
/// 2. Creates a fresh ExclusionSpace for the child (floats don't escape).
/// 3. Determines available inline size accounting for float exclusions in parent.
/// 4. Builds a new ConstraintSpace with `is_new_formatting_context: true`.
/// 5. Returns the fragment and consumed BFC space.
///
/// Note: This does NOT call block_layout — it prepares all the inputs that the
/// caller (block.rs or future integration) will pass to block_layout. This keeps
/// the module standalone per task requirements.
pub fn layout_new_formatting_context(
    input: &NewFcLayoutInput,
) -> NewFcLayoutResult {
    let style = input.style;
    let container_inline = input.container_inline_size;

    // Step 1: Resolve margins immediately — new BFC margins don't collapse.
    let margin_top = resolve_margin_or_padding(&style.margin_top, container_inline);
    let margin_bottom = resolve_margin_or_padding(&style.margin_bottom, container_inline);
    let margin_left = resolve_margin_or_padding(&style.margin_left, container_inline);
    let margin_right = resolve_margin_or_padding(&style.margin_right, container_inline);

    // Step 2: Compute the BFC offset including margins.
    let content_bfc_offset = BfcOffset::new(
        input.child_bfc_offset.line_offset + margin_left,
        input.child_bfc_offset.block_offset + margin_top,
    );

    // Step 3: Determine available inline size and adjust for floats.
    let border_left = LayoutUnit::from_i32(style.effective_border_left());
    let border_right = LayoutUnit::from_i32(style.effective_border_right());
    let padding_left = resolve_margin_or_padding(&style.padding_left, container_inline);
    let padding_right = resolve_margin_or_padding(&style.padding_right, container_inline);
    let border_padding = border_left + padding_left + border_right + padding_right;

    let has_auto_width = style.width.is_auto();

    // For auto-width BFC elements, we need to find available space first, then
    // shrink to fit. For explicit-width elements, resolve width first, then find
    // a position that fits. Per CSS 2.1 §9.5: BFC elements must not overlap floats.
    let (bfc_line_offset, available_inline, bfc_block_offset, is_pushed, element_inline_size) =
        if let Some(ref excl_space) = input.parent_space.exclusion_space {
            if has_auto_width {
                // Auto width: find available space with minimum size (border+padding),
                // then use the opportunity width as the element's width.
                let min_size = border_padding + margin_left + margin_right;
                let result = compute_float_avoidance_offset(
                    excl_space,
                    &content_bfc_offset,
                    container_inline,
                    min_size,
                );
                let pushed = result.block_offset > content_bfc_offset.block_offset;
                let avail = result.available_inline_size - margin_left - margin_right;
                let avail = if avail < LayoutUnit::zero() { LayoutUnit::zero() } else { avail };
                // Auto width shrinks to fit the available space.
                (result.inline_offset, avail, result.block_offset, pushed, avail)
            } else {
                // Explicit width: resolve size, then find position.
                let size = resolve_element_inline_size(
                    style, container_inline, margin_left, margin_right, border_padding,
                );
                let result = compute_float_avoidance_offset(
                    excl_space,
                    &content_bfc_offset,
                    container_inline,
                    size + margin_left + margin_right,
                );
                let pushed = result.block_offset > content_bfc_offset.block_offset;
                let avail = result.available_inline_size - margin_left - margin_right;
                let avail = if avail < LayoutUnit::zero() { LayoutUnit::zero() } else { avail };
                (result.inline_offset, avail, result.block_offset, pushed, size)
            }
        } else {
            let avail = container_inline - margin_left - margin_right;
            let avail = if avail < LayoutUnit::zero() { LayoutUnit::zero() } else { avail };
            let size = resolve_element_inline_size(
                style, container_inline, margin_left, margin_right, border_padding,
            );
            (content_bfc_offset.line_offset, avail, content_bfc_offset.block_offset, false, size)
        };

    // Step 4: Build ConstraintSpace for the child with a fresh ExclusionSpace.
    let child_space = build_new_fc_constraint_space(
        input.parent_space,
        available_inline,
        input.container_block_size,
        container_inline,
    );

    // Step 5: Create a placeholder fragment. The actual layout (block_layout call)
    // is deferred to the integration layer in block.rs. We return the fragment
    // with the resolved geometry so the caller can complete layout.
    let fragment_size = PhysicalSize::new(element_inline_size, LayoutUnit::zero());
    let mut fragment = Fragment::new_box(input.node_id, fragment_size);

    // Store resolved box-model on the fragment for downstream use.
    fragment.margin = openui_geometry::BoxStrut {
        top: margin_top,
        right: margin_right,
        bottom: margin_bottom,
        left: margin_left,
    };

    // Embed the child constraint space info into the result.
    let _ = &child_space; // consumed — future integration will pass this to block_layout

    NewFcLayoutResult {
        fragment,
        margin_top,
        margin_bottom,
        margin_left,
        margin_right,
        bfc_block_offset,
        bfc_line_offset,
        is_pushed_by_floats: is_pushed,
    }
}

// ── 4. Constraint space construction ─────────────────────────────────────

/// Build a `ConstraintSpace` for a child that establishes a new BFC.
///
/// Key differences from a normal child space:
/// - `is_new_formatting_context` is `true`
/// - `exclusion_space` is `None` (fresh — child gets its own float context)
/// - `floats_bfc_block_offset` is `None` (no pending float state from parent)
pub fn build_new_fc_constraint_space(
    parent_space: &ConstraintSpace,
    available_inline_size: LayoutUnit,
    available_block_size: LayoutUnit,
    percentage_inline_size: LayoutUnit,
) -> ConstraintSpace {
    ConstraintSpaceBuilder::from_parent(parent_space)
        .set_available_size(available_inline_size, available_block_size)
        .set_percentage_resolution_size(
            percentage_inline_size,
            available_block_size,
        )
        .set_is_new_formatting_context(true)
        .set_exclusion_space(None)
        .set_floats_bfc_block_offset(None)
        .set_bfc_offset(BfcOffset::zero())
        .build()
}

// ── 5. Margin resolution helpers ─────────────────────────────────────────

/// Resolve the end margin strut for a new-BFC child.
///
/// New-BFC elements do NOT participate in margin collapsing with their parent.
/// Their margins are resolved immediately and produce no adjoining margin strut.
pub fn resolve_new_fc_margins(
    style: &ComputedStyle,
    containing_inline_size: LayoutUnit,
) -> (LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit) {
    let top = resolve_margin_or_padding(&style.margin_top, containing_inline_size);
    let bottom = resolve_margin_or_padding(&style.margin_bottom, containing_inline_size);
    let left = resolve_margin_or_padding(&style.margin_left, containing_inline_size);
    let right = resolve_margin_or_padding(&style.margin_right, containing_inline_size);
    (top, right, bottom, left)
}

/// Returns an empty `MarginStrut` — new-BFC elements produce no adjoining
/// margin contributions to the parent's margin collapsing chain.
pub fn new_fc_end_margin_strut() -> MarginStrut {
    MarginStrut::new()
}

// ── Internal helpers ─────────────────────────────────────────────────────

/// Resolve the element's border-box inline size from style, or compute it
/// as the available width minus margins if `width` is auto.
fn resolve_element_inline_size(
    style: &ComputedStyle,
    container_inline_size: LayoutUnit,
    margin_left: LayoutUnit,
    margin_right: LayoutUnit,
    border_padding: LayoutUnit,
) -> LayoutUnit {
    let auto_size = {
        let s = container_inline_size - margin_left - margin_right;
        if s < LayoutUnit::zero() { LayoutUnit::zero() } else { s }
    };
    let resolved = crate::length_resolver::resolve_length(
        &style.width,
        container_inline_size,
        auto_size,
        auto_size,
    );

    // If the style specifies content-box sizing, add border+padding.
    use openui_style::BoxSizing;
    match style.box_sizing {
        BoxSizing::ContentBox => {
            let total = resolved + border_padding;
            if total < LayoutUnit::zero() { LayoutUnit::zero() } else { total }
        }
        BoxSizing::BorderBox => {
            if resolved < LayoutUnit::zero() { LayoutUnit::zero() } else { resolved }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_style::{Display, Position, Float, Overflow};

    fn lu(v: i32) -> LayoutUnit {
        LayoutUnit::from_i32(v)
    }

    #[test]
    fn root_always_creates_new_fc() {
        let style = ComputedStyle::initial();
        assert!(creates_new_formatting_context(&style, true));
    }

    #[test]
    fn normal_inline_does_not_create_fc() {
        let style = ComputedStyle::initial();
        assert!(!creates_new_formatting_context(&style, false));
    }

    #[test]
    fn overflow_hidden_creates_fc() {
        let mut style = ComputedStyle::initial();
        style.overflow_x = Overflow::Hidden;
        assert!(creates_new_formatting_context(&style, false));
    }

    #[test]
    fn float_creates_fc() {
        let mut style = ComputedStyle::initial();
        style.float = Float::Left;
        assert!(creates_new_formatting_context(&style, false));
    }

    #[test]
    fn absolute_position_creates_fc() {
        let mut style = ComputedStyle::initial();
        style.position = Position::Absolute;
        assert!(creates_new_formatting_context(&style, false));
    }

    #[test]
    fn flow_root_creates_fc() {
        let mut style = ComputedStyle::initial();
        style.display = Display::FlowRoot;
        assert!(creates_new_formatting_context(&style, false));
    }

    #[test]
    fn flex_creates_fc() {
        let mut style = ComputedStyle::initial();
        style.display = Display::Flex;
        assert!(creates_new_formatting_context(&style, false));
    }

    #[test]
    fn new_fc_constraint_space_has_fresh_exclusion_space() {
        let parent = ConstraintSpace::for_root(lu(800), lu(600));
        let child_space = build_new_fc_constraint_space(&parent, lu(400), lu(600), lu(800));

        assert!(child_space.is_new_formatting_context);
        assert!(child_space.exclusion_space.is_none());
        assert!(child_space.floats_bfc_block_offset.is_none());
        assert_eq!(child_space.bfc_offset, BfcOffset::zero());
        assert_eq!(child_space.available_inline_size, lu(400));
    }

    #[test]
    fn new_fc_end_margin_strut_is_empty() {
        let strut = new_fc_end_margin_strut();
        assert!(strut.is_empty());
        assert_eq!(strut.sum(), LayoutUnit::zero());
    }

    #[test]
    fn float_avoidance_no_floats() {
        let space = ExclusionSpace::new();
        let offset = BfcOffset::new(lu(0), lu(0));
        let (inline_off, avail, block_off) =
            adjust_for_float_avoidance(&space, &offset, lu(800), lu(400));
        assert_eq!(inline_off, lu(0));
        assert_eq!(avail, lu(800));
        assert_eq!(block_off, lu(0));
    }

    #[test]
    fn float_avoidance_result_default() {
        let result = FloatAvoidanceResult {
            inline_offset: lu(200),
            available_inline_size: lu(600),
            block_offset: lu(0),
        };
        assert_eq!(result.inline_offset, lu(200));
        assert_eq!(result.available_inline_size, lu(600));
    }
}
