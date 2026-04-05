//! Sticky positioning — CSS Positioned Layout Module Level 3 §3.
//!
//! A sticky-positioned element is positioned relative to its normal flow
//! position, but "sticks" to certain edges of its scroll container as the
//! user scrolls. The sticky offset is constrained by the containing block's
//! content box (the "sticky-constraint rectangle").

use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_style::{ComputedStyle, Position};

use crate::fragment::Fragment;
use crate::length_resolver::resolve_length;

// ---------------------------------------------------------------------------
// StickyConstraintRect
// ---------------------------------------------------------------------------

/// The rectangle within which a sticky element is allowed to move.
///
/// Each edge is `Some(value)` when the corresponding CSS inset property
/// (`top`, `right`, `bottom`, `left`) is not `auto`, representing the
/// minimum distance the element must maintain from that edge of the scroll
/// container viewport while within the constraint rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickyConstraintRect {
    /// Top inset (distance from viewport top edge).
    pub top: Option<LayoutUnit>,
    /// Bottom inset (distance from viewport bottom edge).
    pub bottom: Option<LayoutUnit>,
    /// Left inset (distance from viewport left edge).
    pub left: Option<LayoutUnit>,
    /// Right inset (distance from viewport right edge).
    pub right: Option<LayoutUnit>,
}

impl StickyConstraintRect {
    /// All edges are `None` (no sticking).
    pub const fn none() -> Self {
        Self {
            top: None,
            bottom: None,
            left: None,
            right: None,
        }
    }
}

// ---------------------------------------------------------------------------
// StickyPositionData — collected during layout
// ---------------------------------------------------------------------------

/// Per-element data that the block layout algorithm captures during normal
/// flow so that a later scroll-driven pass can compute the sticky offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickyPositionData {
    /// The element's offset in its parent, as determined by normal flow.
    pub normal_flow_offset: PhysicalOffset,
    /// Resolved sticky insets from the CSS `top`/`right`/`bottom`/`left` properties.
    pub insets: StickyConstraintRect,
    /// Border-box size of the sticky element.
    pub element_size: PhysicalSize,
    /// The containing block's content-box rectangle (in scroll-container
    /// coordinates). This is the outer boundary that clamps the sticky offset.
    pub containing_block_rect: PhysicalRect,
}

// ---------------------------------------------------------------------------
// compute_sticky_constraint_rect
// ---------------------------------------------------------------------------

/// Build a [`StickyConstraintRect`] from computed style values.
///
/// Each CSS inset property that is **not** `auto` is resolved against the
/// appropriate containing-block dimension and stored as `Some(value)`.
/// `auto` insets become `None`.
pub fn compute_sticky_constraint_rect(
    style: &ComputedStyle,
    containing_block_inline_size: LayoutUnit,
    containing_block_block_size: LayoutUnit,
) -> StickyConstraintRect {
    let zero = LayoutUnit::zero();

    let resolve = |len: &Length, cb_size: LayoutUnit| -> Option<LayoutUnit> {
        if len.is_auto() {
            None
        } else {
            Some(resolve_length(len, cb_size, zero, zero))
        }
    };

    StickyConstraintRect {
        top: resolve(&style.top, containing_block_block_size),
        bottom: resolve(&style.bottom, containing_block_block_size),
        left: resolve(&style.left, containing_block_inline_size),
        right: resolve(&style.right, containing_block_inline_size),
    }
}

// ---------------------------------------------------------------------------
// compute_sticky_offset  — core algorithm (CSS Positioned Layout L3 §3)
// ---------------------------------------------------------------------------

/// Compute the visual offset to apply to a sticky element given the current
/// scroll position.
///
/// # Algorithm (per axis)
///
/// ```text
/// if (inset-start != auto AND inset-end != auto):
///     offset = max(start_stick, min(0, end_stick))
/// else if (inset-start != auto):
///     offset = max(0, start_stick)
/// else if (inset-end != auto):
///     offset = min(0, end_stick)
/// else:
///     offset = 0
///
/// offset = clamp(offset, -max_negative, max_positive)
/// ```
///
/// * `start_stick` is the amount the element must shift to reach its start-
///   edge sticky threshold.
/// * `end_stick` is the amount it must shift to reach its end-edge threshold.
/// * The clamp ensures the element never overflows its containing block.
pub fn compute_sticky_offset(
    normal_flow_offset: PhysicalOffset,
    scroll_offset: PhysicalOffset,
    viewport_rect: PhysicalRect,
    insets: &StickyConstraintRect,
    element_size: PhysicalSize,
    containing_block_rect: PhysicalRect,
) -> PhysicalOffset {
    let block_offset = compute_axis_offset(
        normal_flow_offset.top,
        scroll_offset.top,
        viewport_rect.y(),
        viewport_rect.height(),
        insets.top,
        insets.bottom,
        element_size.height,
        containing_block_rect.y(),
        containing_block_rect.height(),
    );

    let inline_offset = compute_axis_offset(
        normal_flow_offset.left,
        scroll_offset.left,
        viewport_rect.x(),
        viewport_rect.width(),
        insets.left,
        insets.right,
        element_size.width,
        containing_block_rect.x(),
        containing_block_rect.width(),
    );

    PhysicalOffset::new(inline_offset, block_offset)
}

/// Per-axis sticky offset computation.
///
/// * `normal_pos`   — element's start edge in scroll-container coordinates.
/// * `scroll`       — current scroll offset along this axis.
/// * `vp_start`     — viewport start edge (often 0).
/// * `vp_extent`    — viewport extent (width or height).
/// * `inset_start`  — resolved CSS start inset (top / left), or `None`.
/// * `inset_end`    — resolved CSS end inset (bottom / right), or `None`.
/// * `element_extent` — element dimension along this axis.
/// * `cb_start`     — containing block start edge in scroll-container coords.
/// * `cb_extent`    — containing block extent.
fn compute_axis_offset(
    normal_pos: LayoutUnit,
    scroll: LayoutUnit,
    vp_start: LayoutUnit,
    vp_extent: LayoutUnit,
    inset_start: Option<LayoutUnit>,
    inset_end: Option<LayoutUnit>,
    element_extent: LayoutUnit,
    cb_start: LayoutUnit,
    cb_extent: LayoutUnit,
) -> LayoutUnit {
    let zero = LayoutUnit::zero();

    // Position of the element's start edge relative to the viewport.
    // (positive = below/right of viewport start)
    let el_in_vp = normal_pos - scroll + vp_start;

    // "start_stick": how much to shift so the element's start edge is at
    // `viewport_start + inset_start`.
    let start_stick = inset_start
        .map(|inset| (vp_start + inset) - el_in_vp);

    // "end_stick": how much to shift so the element's end edge is at
    // `viewport_end - inset_end`.
    let end_stick = inset_end
        .map(|inset| (vp_start + vp_extent - inset) - (el_in_vp + element_extent));

    // CSS §3 decision table:
    let raw_offset = match (start_stick, end_stick) {
        (Some(s), Some(e)) => {
            // Both insets specified: stick to start but don't exceed end.
            max_lu(s, min_lu(zero, e))
        }
        (Some(s), None) => {
            // Only start inset: stick to start edge.
            max_lu(zero, s)
        }
        (None, Some(e)) => {
            // Only end inset: stick to end edge.
            min_lu(zero, e)
        }
        (None, None) => zero,
    };

    // Clamp so the element stays within the containing block.
    //
    //   max_positive: how far toward the end the element can shift before
    //                 its end edge hits the CB end edge.
    //   max_negative: how far toward the start the element can shift before
    //                 its start edge hits the CB start edge.
    let cb_end = cb_start + cb_extent;
    let max_positive = (cb_end - element_extent) - normal_pos;
    let max_negative = normal_pos - cb_start;

    clamp_lu(raw_offset, -max_negative, max_positive)
}

// ---------------------------------------------------------------------------
// apply_sticky_offset — fragment mutation
// ---------------------------------------------------------------------------

/// Apply the sticky offset to an already-laid-out fragment.
///
/// If the fragment's style is not `position: sticky`, this is a no-op.
pub fn apply_sticky_offset(
    fragment: &mut Fragment,
    style: &ComputedStyle,
    scroll_offset: PhysicalOffset,
    viewport_rect: PhysicalRect,
    containing_block_inline_size: LayoutUnit,
    containing_block_block_size: LayoutUnit,
    containing_block_rect: PhysicalRect,
) {
    if style.position != Position::Sticky {
        return;
    }

    let insets = compute_sticky_constraint_rect(
        style,
        containing_block_inline_size,
        containing_block_block_size,
    );

    let offset = compute_sticky_offset(
        fragment.offset,
        scroll_offset,
        viewport_rect,
        &insets,
        fragment.size,
        containing_block_rect,
    );

    fragment.offset.left = fragment.offset.left + offset.left;
    fragment.offset.top = fragment.offset.top + offset.top;
}

// ---------------------------------------------------------------------------
// LayoutUnit helpers (min / max / clamp)
// ---------------------------------------------------------------------------

#[inline]
fn min_lu(a: LayoutUnit, b: LayoutUnit) -> LayoutUnit {
    if a <= b { a } else { b }
}

#[inline]
fn max_lu(a: LayoutUnit, b: LayoutUnit) -> LayoutUnit {
    if a >= b { a } else { b }
}

#[inline]
fn clamp_lu(val: LayoutUnit, lo: LayoutUnit, hi: LayoutUnit) -> LayoutUnit {
    // If lo > hi (element larger than CB), prefer lo (start edge wins).
    let effective_hi = max_lu(lo, hi);
    max_lu(lo, min_lu(val, effective_hi))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(v: i32) -> LayoutUnit {
        LayoutUnit::from_i32(v)
    }

    #[test]
    fn constraint_rect_none_is_all_none() {
        let cr = StickyConstraintRect::none();
        assert_eq!(cr.top, None);
        assert_eq!(cr.bottom, None);
        assert_eq!(cr.left, None);
        assert_eq!(cr.right, None);
    }

    #[test]
    fn axis_no_insets_returns_zero() {
        let off = compute_axis_offset(
            lu(100), lu(0), lu(0), lu(600),
            None, None, lu(50), lu(0), lu(1000),
        );
        assert_eq!(off, lu(0));
    }

    #[test]
    fn axis_start_inset_sticks() {
        // Element at y=200, viewport 600px, scroll=250, inset-top=10.
        // el_in_vp = 200 - 250 = -50.  start_stick = (0+10) - (-50) = 60.
        // max(0, 60) = 60.
        let off = compute_axis_offset(
            lu(200), lu(250), lu(0), lu(600),
            Some(lu(10)), None, lu(50), lu(0), lu(1000),
        );
        assert_eq!(off, lu(60));
    }

    #[test]
    fn axis_end_inset_sticks() {
        // Element at y=500, height=50, viewport 600, scroll=0, inset-bottom=10.
        // el_in_vp = 500. end_stick = (600-10) - (500+50) = 40.
        // min(0, 40) = 0  → no sticking needed (already inside viewport).
        let off = compute_axis_offset(
            lu(500), lu(0), lu(0), lu(600),
            None, Some(lu(10)), lu(50), lu(0), lu(1000),
        );
        assert_eq!(off, lu(0));
    }

    #[test]
    fn clamp_within_containing_block() {
        // Element at y=900, CB is [0..950], element height=50.
        // max_positive = (950 - 50) - 900 = 0.
        // Any positive offset should be clamped to 0.
        let off = compute_axis_offset(
            lu(900), lu(950), lu(0), lu(600),
            Some(lu(0)), None, lu(50), lu(0), lu(950),
        );
        assert_eq!(off, lu(0));
    }
}
