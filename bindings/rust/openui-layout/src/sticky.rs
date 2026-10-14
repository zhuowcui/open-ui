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
    let resolve = |len: &Length, cb_size: LayoutUnit| -> Option<LayoutUnit> {
        if len.is_auto() {
            None
        } else {
            Some(resolve_sticky_length(len, cb_size))
        }
    };

    StickyConstraintRect {
        top: resolve(&style.top, containing_block_block_size),
        bottom: resolve(&style.bottom, containing_block_block_size),
        left: resolve(&style.left, containing_block_inline_size),
        right: resolve(&style.right, containing_block_inline_size),
    }
}

/// Chromium MinimumValueForLength, shared by sticky insets and margins.
pub(crate) fn resolve_sticky_length(length: &Length, basis: LayoutUnit) -> LayoutUnit {
    if length.is_percent() {
        LayoutUnit::from_f32(basis.to_f32() * length.value() / 100.0)
    } else {
        resolve_length(length, basis, LayoutUnit::zero(), LayoutUnit::zero())
    }
}

// ---------------------------------------------------------------------------
// compute_sticky_offset  — core algorithm (CSS Positioned Layout L3 §3)
// ---------------------------------------------------------------------------

/// Compute the visual offset to apply to a sticky element given the current
/// scroll position.
///
/// End movement is computed and applied first, then start movement. Both
/// requests use the same normal sticky range and are limited by the containing
/// block. The retained layout pass supplies the writing-direction-adjusted
/// insets and the two ancestor channels before calling this shared operation.
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

    // Blink computes both movements from the unchanged normal sticky range,
    // after the independent ancestor shifts. Scrolling moves the constraining
    // range, so a nonzero viewport origin remains part of the threshold.
    let mut end = inset_end;
    if let (Some(start), Some(maximum)) = (inset_start, end.as_mut()) {
        let free = vp_extent - element_extent - start - *maximum;
        if free < zero {
            // The retained layout caller already adjusts the end corresponding
            // to its containing block's writing direction. This default is the
            // unflipped physical axis for standalone callers.
            *maximum = *maximum + free;
        }
    }
    let end_delta = end.map_or(zero, |inset| {
        let requested = min_lu(
            vp_start + scroll + vp_extent - inset - normal_pos - element_extent,
            zero,
        );
        max_lu(requested, min_lu(cb_start - normal_pos, zero))
    });
    let start_delta = inset_start.map_or(zero, |inset| {
        let requested = max_lu(vp_start + scroll + inset - normal_pos, zero);
        min_lu(
            requested,
            max_lu(cb_start + cb_extent - normal_pos - element_extent, zero),
        )
    });
    end_delta + start_delta
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
    if a <= b {
        a
    } else {
        b
    }
}

#[inline]
fn max_lu(a: LayoutUnit, b: LayoutUnit) -> LayoutUnit {
    if a >= b {
        a
    } else {
        b
    }
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
            lu(100),
            lu(0),
            lu(0),
            lu(600),
            None,
            None,
            lu(50),
            lu(0),
            lu(1000),
        );
        assert_eq!(off, lu(0));
    }

    #[test]
    fn axis_start_inset_sticks() {
        // Element at y=200, viewport 600px, scroll=250, inset-top=10.
        // el_in_vp = 200 - 250 = -50.  start_stick = (0+10) - (-50) = 60.
        // max(0, 60) = 60.
        let off = compute_axis_offset(
            lu(200),
            lu(250),
            lu(0),
            lu(600),
            Some(lu(10)),
            None,
            lu(50),
            lu(0),
            lu(1000),
        );
        assert_eq!(off, lu(60));
    }

    #[test]
    fn axis_end_inset_sticks() {
        // Element at y=500, height=50, viewport 600, scroll=0, inset-bottom=10.
        // el_in_vp = 500. end_stick = (600-10) - (500+50) = 40.
        // min(0, 40) = 0  → no sticking needed (already inside viewport).
        let off = compute_axis_offset(
            lu(500),
            lu(0),
            lu(0),
            lu(600),
            None,
            Some(lu(10)),
            lu(50),
            lu(0),
            lu(1000),
        );
        assert_eq!(off, lu(0));
    }

    #[test]
    fn clamp_within_containing_block() {
        // Element at y=900, CB is [0..950], element height=50.
        // max_positive = (950 - 50) - 900 = 0.
        // Any positive offset should be clamped to 0.
        let off = compute_axis_offset(
            lu(900),
            lu(950),
            lu(0),
            lu(600),
            Some(lu(0)),
            None,
            lu(50),
            lu(0),
            lu(950),
        );
        assert_eq!(off, lu(0));
    }

    #[test]
    fn zero_sticky_movement_does_not_reposition_outside_containing_block() {
        let before_start = compute_axis_offset(
            lu(-100),
            lu(0),
            lu(0),
            lu(100),
            None,
            Some(lu(0)),
            lu(100),
            lu(0),
            lu(100),
        );
        let after_end = compute_axis_offset(
            lu(0),
            lu(0),
            lu(0),
            lu(100),
            None,
            Some(lu(0)),
            lu(100),
            lu(-100),
            lu(100),
        );
        assert_eq!(before_start, lu(0));
        assert_eq!(after_end, lu(0));
    }
}
