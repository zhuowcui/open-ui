//! Float positioning utilities — extracted from Blink's `floats_utils.h/cc`.
//!
//! Source: `core/layout/floats_utils.h`, `core/layout/floats_utils.cc`,
//!         `core/layout/unpositioned_float.h`
//!
//! This module provides the data structures and algorithms for positioning
//! CSS floats within a block formatting context (BFC). A float starts as
//! an [`UnpositionedFloat`] (its intrinsic size and origin are known but it
//! has no final BFC position yet). The [`position_float`] function queries
//! the [`ExclusionSpace`] for a suitable layout opportunity, then places
//! the float and returns a [`PositionedFloat`] together with the
//! [`ExclusionArea`] that the caller should add to the exclusion space.

use openui_geometry::{BfcOffset, BfcRect, BoxStrut, LayoutUnit, PhysicalSize};
use openui_dom::NodeId;

use crate::fragment::Fragment;
use super::{ExclusionArea, ExclusionSpace, ExclusionType, LayoutOpportunity};

// ─────────────────────────────────────────────────────────────────────────────
// UnpositionedFloat
// ─────────────────────────────────────────────────────────────────────────────

/// A float whose intrinsic size has been computed but whose final BFC
/// position has not yet been determined.
///
/// Corresponds to Blink's `UnpositionedFloat` struct in
/// `core/layout/unpositioned_float.h`. The block formatting context
/// algorithm creates an `UnpositionedFloat` when it encounters a floated
/// child, then later calls [`position_float`] to resolve its position.
#[derive(Debug)]
pub struct UnpositionedFloat {
    /// The DOM node that generated this float.
    pub node_id: NodeId,

    /// Available inline size in the float's BFC at the point of origin.
    /// This is the container's content-box inline size — the maximum
    /// inline space within which the float may be placed.
    pub available_size: LayoutUnit,

    /// The BFC offset where this float originates — typically the current
    /// block offset of the formatting context plus any accumulated margins.
    pub origin_bfc_offset: BfcOffset,

    /// Resolved margin edges of the float element (top, right, bottom, left).
    pub margins: BoxStrut,

    /// Border-box inline size of the float (width in horizontal-tb).
    pub inline_size: LayoutUnit,

    /// Border-box block size of the float (height in horizontal-tb).
    pub block_size: LayoutUnit,

    /// `true` for `float: left`, `false` for `float: right`.
    pub is_left: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// PositionedFloat
// ─────────────────────────────────────────────────────────────────────────────

/// A float that has been placed at a concrete BFC offset and whose layout
/// fragment has been generated.
///
/// Corresponds to the output of Blink's `PositionFloat()` in
/// `floats_utils.cc`. After positioning, the caller adds the accompanying
/// [`ExclusionArea`] to the [`ExclusionSpace`] and appends the fragment
/// to the parent's child list.
#[derive(Debug)]
pub struct PositionedFloat {
    /// The DOM node that generated this float.
    pub node_id: NodeId,

    /// Final position of the float's border-box in BFC coordinates.
    pub bfc_offset: BfcOffset,

    /// The fully laid-out fragment for this float.
    pub fragment: Fragment,

    /// Whether this is a left or right exclusion.
    pub exclusion_type: ExclusionType,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Compute the margin-box inline size of an unpositioned float.
///
/// The margin-box inline size is `margin-left + border-box-width + margin-right`.
/// This value is used by the exclusion space algorithm to determine how much
/// inline space the float will consume.
///
/// # Examples
///
/// ```ignore
/// let size = compute_margin_box_inline_size(&unpositioned);
/// ```
#[inline]
pub fn compute_margin_box_inline_size(float: &UnpositionedFloat) -> LayoutUnit {
    float.margins.left + float.inline_size + float.margins.right
}

/// Position a float within a block formatting context.
///
/// This is the core float-placement algorithm, extracted from Blink's
/// `PositionFloat()` in `floats_utils.cc`. It performs the following steps:
///
/// 1. Compute the float's margin-box inline size.
/// 2. Query the [`ExclusionSpace`] for the first layout opportunity at or
///    below the float's origin that is wide enough to contain the float.
/// 3. Determine the float's line offset:
///    - **Left float**: placed at the opportunity's left edge plus the
///      float's left margin.
///    - **Right float**: placed at the opportunity's right edge minus the
///      float's right margin minus its border-box inline size.
/// 4. The block offset is the opportunity's block-start plus the float's
///    top margin.
/// 5. Construct a [`PositionedFloat`] with a box fragment at the resolved
///    position, and an [`ExclusionArea`] covering the float's margin box.
///
/// # Returns
///
/// A tuple of `(PositionedFloat, ExclusionArea)`. The caller must add the
/// exclusion area to the exclusion space via [`ExclusionSpace::add`].
pub fn position_float(
    float: &UnpositionedFloat,
    exclusion_space: &ExclusionSpace,
) -> (PositionedFloat, ExclusionArea) {
    let margin_inline_size = compute_margin_box_inline_size(float);
    let exclusion_type = if float.is_left {
        ExclusionType::Left
    } else {
        ExclusionType::Right
    };

    // Find a layout opportunity wide enough for the float's margin box.
    let opportunity = exclusion_space.find_layout_opportunity(
        &float.origin_bfc_offset,
        float.available_size,
        margin_inline_size,
    );

    // Resolve the float's border-box position within the opportunity.
    let (line_offset, block_offset) =
        resolve_float_position(float, &opportunity);

    let bfc_offset = BfcOffset::new(line_offset, block_offset);

    // Build the exclusion area covering the float's margin box.
    let exclusion_rect = compute_exclusion_rect(float, &opportunity);

    let exclusion = ExclusionArea {
        rect: exclusion_rect,
        exclusion_type,
    };

    // Create the positioned fragment at the resolved BFC offset.
    let mut fragment = Fragment::new_box(
        float.node_id,
        PhysicalSize::new(float.inline_size, float.block_size),
    );
    fragment.margin = float.margins;

    let positioned = PositionedFloat {
        node_id: float.node_id,
        bfc_offset,
        fragment,
        exclusion_type,
    };

    (positioned, exclusion)
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve the border-box line and block offsets for the float within the
/// given layout opportunity.
///
/// For a left float the border box starts at the opportunity's left edge
/// plus the left margin.  For a right float it is flush against the
/// opportunity's right edge (right edge − right margin − inline size).
fn resolve_float_position(
    float: &UnpositionedFloat,
    opportunity: &LayoutOpportunity,
) -> (LayoutUnit, LayoutUnit) {
    let opp_line_start = opportunity.rect.line_start_offset();
    let opp_line_end = opportunity.rect.line_end_offset();
    let opp_block_start = opportunity.rect.block_start_offset();

    let line_offset = if float.is_left {
        opp_line_start + float.margins.left
    } else {
        opp_line_end - float.margins.right - float.inline_size
    };

    let block_offset = opp_block_start + float.margins.top;

    (line_offset, block_offset)
}

/// Compute the BFC rectangle for the float's margin box — this is the
/// exclusion area that prevents other content from overlapping the float.
///
/// The rectangle spans from `(opportunity_start + 0, block_start)` to
/// `(opportunity_start + margin_inline_size, block_end)` for left floats,
/// and from `(opportunity_end − margin_inline_size, block_start)` to
/// `(opportunity_end, block_end)` for right floats.
fn compute_exclusion_rect(
    float: &UnpositionedFloat,
    opportunity: &LayoutOpportunity,
) -> BfcRect {
    let margin_inline_size = compute_margin_box_inline_size(float);
    let margin_block_size = float.margins.top + float.block_size + float.margins.bottom;

    let opp_block_start = opportunity.rect.block_start_offset();

    let (line_start, line_end) = if float.is_left {
        let start = opportunity.rect.line_start_offset();
        (start, start + margin_inline_size)
    } else {
        let end = opportunity.rect.line_end_offset();
        (end - margin_inline_size, end)
    };

    BfcRect::new(
        BfcOffset::new(line_start, opp_block_start),
        BfcOffset::new(line_end, opp_block_start + margin_block_size),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Shorthand: create a LayoutUnit from an integer.
    fn lu(v: i32) -> LayoutUnit {
        LayoutUnit::from_i32(v)
    }

    /// Helper to build a simple left/right float with uniform zero margins.
    fn make_float(
        inline_size: i32,
        block_size: i32,
        is_left: bool,
        available: i32,
        origin_line: i32,
        origin_block: i32,
    ) -> UnpositionedFloat {
        UnpositionedFloat {
            node_id: NodeId::NONE,
            available_size: lu(available),
            origin_bfc_offset: BfcOffset::new(lu(origin_line), lu(origin_block)),
            margins: BoxStrut::zero(),
            inline_size: lu(inline_size),
            block_size: lu(block_size),
            is_left,
        }
    }

    /// Helper to build a float with explicit margins.
    fn make_float_with_margins(
        inline_size: i32,
        block_size: i32,
        is_left: bool,
        available: i32,
        margins: BoxStrut,
    ) -> UnpositionedFloat {
        UnpositionedFloat {
            node_id: NodeId::NONE,
            available_size: lu(available),
            origin_bfc_offset: BfcOffset::zero(),
            margins,
            inline_size: lu(inline_size),
            block_size: lu(block_size),
            is_left,
        }
    }

    // ── compute_margin_box_inline_size ────────────────────────────────

    #[test]
    fn margin_box_inline_size_zero_margins() {
        let f = make_float(200, 100, true, 800, 0, 0);
        assert_eq!(compute_margin_box_inline_size(&f), lu(200));
    }

    #[test]
    fn margin_box_inline_size_with_margins() {
        let f = make_float_with_margins(
            200,
            100,
            true,
            800,
            BoxStrut::new(lu(5), lu(20), lu(5), lu(10)),
        );
        // 10 (left) + 200 + 20 (right) = 230
        assert_eq!(compute_margin_box_inline_size(&f), lu(230));
    }

    // ── position_float: left float in empty space ────────────────────

    #[test]
    fn left_float_empty_space() {
        let space = ExclusionSpace::new();
        let f = make_float(200, 100, true, 800, 0, 0);
        let (pos, excl) = position_float(&f, &space);

        // Left float placed at line_offset=0, block_offset=0.
        assert_eq!(pos.bfc_offset.line_offset, lu(0));
        assert_eq!(pos.bfc_offset.block_offset, lu(0));
        assert_eq!(pos.exclusion_type, ExclusionType::Left);

        // Exclusion covers 0..200 inline, 0..100 block.
        assert_eq!(excl.rect.line_start_offset(), lu(0));
        assert_eq!(excl.rect.line_end_offset(), lu(200));
        assert_eq!(excl.rect.block_start_offset(), lu(0));
        assert_eq!(excl.rect.block_end_offset(), lu(100));
    }

    // ── position_float: right float in empty space ───────────────────

    #[test]
    fn right_float_empty_space() {
        let space = ExclusionSpace::new();
        let f = make_float(200, 100, false, 800, 0, 0);
        let (pos, excl) = position_float(&f, &space);

        // Right float: 800 − 200 = 600.
        assert_eq!(pos.bfc_offset.line_offset, lu(600));
        assert_eq!(pos.bfc_offset.block_offset, lu(0));
        assert_eq!(pos.exclusion_type, ExclusionType::Right);

        // Exclusion: 600..800 inline, 0..100 block.
        assert_eq!(excl.rect.line_start_offset(), lu(600));
        assert_eq!(excl.rect.line_end_offset(), lu(800));
        assert_eq!(excl.rect.block_start_offset(), lu(0));
        assert_eq!(excl.rect.block_end_offset(), lu(100));
    }

    // ── position_float: left float with margins ──────────────────────

    #[test]
    fn left_float_with_margins() {
        let space = ExclusionSpace::new();
        let f = make_float_with_margins(
            200,
            100,
            true,
            800,
            BoxStrut::new(lu(10), lu(20), lu(15), lu(30)),
        );
        let (pos, excl) = position_float(&f, &space);

        // Border-box starts after left margin: line=30, block=10 (top margin).
        assert_eq!(pos.bfc_offset.line_offset, lu(30));
        assert_eq!(pos.bfc_offset.block_offset, lu(10));

        // Exclusion margin box: 0..250 (30+200+20), 0..125 (10+100+15).
        assert_eq!(excl.rect.line_start_offset(), lu(0));
        assert_eq!(excl.rect.line_end_offset(), lu(250));
        assert_eq!(excl.rect.block_start_offset(), lu(0));
        assert_eq!(excl.rect.block_end_offset(), lu(125));
    }

    // ── position_float: right float with margins ─────────────────────

    #[test]
    fn right_float_with_margins() {
        let space = ExclusionSpace::new();
        let f = make_float_with_margins(
            200,
            100,
            false,
            800,
            BoxStrut::new(lu(10), lu(20), lu(15), lu(30)),
        );
        let (pos, excl) = position_float(&f, &space);

        // Right float margin box is 30+200+20=250 wide.
        // Opportunity right edge is 800. Border box starts at 800−20−200=580.
        assert_eq!(pos.bfc_offset.line_offset, lu(580));
        assert_eq!(pos.bfc_offset.block_offset, lu(10));

        // Exclusion: 550..800 inline (800−250=550), 0..125 block.
        assert_eq!(excl.rect.line_start_offset(), lu(550));
        assert_eq!(excl.rect.line_end_offset(), lu(800));
        assert_eq!(excl.rect.block_start_offset(), lu(0));
        assert_eq!(excl.rect.block_end_offset(), lu(125));
    }

    // ── position_float: float drops below existing float ─────────────

    #[test]
    fn left_float_drops_below_existing() {
        let mut space = ExclusionSpace::new();
        // Existing left float: 0..600 inline, 0..100 block.
        space.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(lu(0), lu(0)),
                BfcOffset::new(lu(600), lu(100)),
            ),
            exclusion_type: ExclusionType::Left,
        });

        // New left float needs 300px but only 200px available beside the
        // existing float (800−600=200). It must drop below block=100.
        let f = make_float(300, 50, true, 800, 0, 0);
        let (pos, excl) = position_float(&f, &space);

        assert!(
            pos.bfc_offset.block_offset >= lu(100),
            "Float should drop below the existing float"
        );
        // Once below, full 800px is available, so it sits at line=0.
        assert_eq!(pos.bfc_offset.line_offset, lu(0));

        assert_eq!(excl.rect.line_start_offset(), lu(0));
        assert_eq!(excl.rect.line_end_offset(), lu(300));
    }

    // ── position_float: second left float stacks beside first ────────

    #[test]
    fn second_left_float_stacks_beside_first() {
        let mut space = ExclusionSpace::new();
        // First left float: 0..200, 0..100.
        space.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(lu(0), lu(0)),
                BfcOffset::new(lu(200), lu(100)),
            ),
            exclusion_type: ExclusionType::Left,
        });

        // Second left float: 150px wide. Fits beside the first (600px free).
        let f = make_float(150, 80, true, 800, 0, 0);
        let (pos, excl) = position_float(&f, &space);

        // Should be placed at line_offset=200 (right edge of first float).
        assert_eq!(pos.bfc_offset.line_offset, lu(200));
        assert_eq!(pos.bfc_offset.block_offset, lu(0));

        assert_eq!(excl.rect.line_start_offset(), lu(200));
        assert_eq!(excl.rect.line_end_offset(), lu(350));
    }

    // ── position_float: fragment correctness ─────────────────────────

    #[test]
    fn positioned_float_fragment_has_correct_size() {
        let space = ExclusionSpace::new();
        let f = make_float(200, 100, true, 800, 0, 0);
        let (pos, _) = position_float(&f, &space);

        assert_eq!(pos.fragment.size.width, lu(200));
        assert_eq!(pos.fragment.size.height, lu(100));
    }

    // ── position_float: non-zero origin ──────────────────────────────

    #[test]
    fn float_with_nonzero_origin() {
        let space = ExclusionSpace::new();
        // Float originates at block_offset=50 (e.g. after some content).
        let f = make_float(200, 100, true, 800, 0, 50);
        let (pos, excl) = position_float(&f, &space);

        assert_eq!(pos.bfc_offset.line_offset, lu(0));
        assert_eq!(pos.bfc_offset.block_offset, lu(50));

        assert_eq!(excl.rect.block_start_offset(), lu(50));
        assert_eq!(excl.rect.block_end_offset(), lu(150));
    }

    // ── position_float: right float next to left float ───────────────

    #[test]
    fn right_float_next_to_left_float() {
        let mut space = ExclusionSpace::new();
        // Left float occupying 0..300 inline, 0..100 block.
        space.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(lu(0), lu(0)),
                BfcOffset::new(lu(300), lu(100)),
            ),
            exclusion_type: ExclusionType::Left,
        });

        // Right float: 200px wide. Available space is 300..800 = 500px.
        let f = make_float(200, 80, false, 800, 0, 0);
        let (pos, excl) = position_float(&f, &space);

        // Right float: line = 800 − 200 = 600.
        assert_eq!(pos.bfc_offset.line_offset, lu(600));
        assert_eq!(pos.bfc_offset.block_offset, lu(0));

        assert_eq!(excl.rect.line_start_offset(), lu(600));
        assert_eq!(excl.rect.line_end_offset(), lu(800));
        assert_eq!(excl.exclusion_type, ExclusionType::Right);
    }

    // ── position_float: float with all margins next to existing float ─

    #[test]
    fn float_with_margins_next_to_existing() {
        let mut space = ExclusionSpace::new();
        // Existing left float: 0..200, 0..100.
        space.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(lu(0), lu(0)),
                BfcOffset::new(lu(200), lu(100)),
            ),
            exclusion_type: ExclusionType::Left,
        });

        // New left float: 100px wide, 50px tall, with 10px margins all around.
        // Margin box = 10+100+10 = 120px inline.
        // Opportunity at line=200 has 600px free — plenty of room.
        let f = make_float_with_margins(
            100,
            50,
            true,
            800,
            BoxStrut::all(lu(10)),
        );
        let (pos, excl) = position_float(&f, &space);

        // Border box placed at 200 + 10 (left margin) = 210 inline.
        assert_eq!(pos.bfc_offset.line_offset, lu(210));
        // Block = 0 + 10 (top margin) = 10.
        assert_eq!(pos.bfc_offset.block_offset, lu(10));

        // Exclusion margin box: 200..320 inline (200 + 120), 0..70 block (10+50+10).
        assert_eq!(excl.rect.line_start_offset(), lu(200));
        assert_eq!(excl.rect.line_end_offset(), lu(320));
        assert_eq!(excl.rect.block_start_offset(), lu(0));
        assert_eq!(excl.rect.block_end_offset(), lu(70));
    }
}
