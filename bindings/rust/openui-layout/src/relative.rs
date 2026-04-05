//! Relative positioning — CSS 2.1 §9.4.3.
//!
//! A relatively positioned box is offset from its normal flow position without
//! affecting subsequent siblings. This module resolves the `top`, `right`,
//! `bottom`, and `left` offsets and applies them to a fragment.

use openui_geometry::{LayoutUnit, PhysicalOffset};
use openui_style::{ComputedStyle, Direction, Position};

use crate::fragment::Fragment;
use crate::length_resolver::resolve_length;

/// Apply relative positioning offsets to a fragment.
///
/// Per CSS 2.1 §9.4.3: A relatively positioned box is offset from its normal
/// flow position. The box retains its dimensions and does not affect the
/// positions of subsequent siblings.
///
/// - `top` and `bottom` are mutually exclusive; if both specified, `bottom` is ignored.
/// - `left` and `right` are mutually exclusive; if both specified, `right` is ignored (LTR)
///    or `left` is ignored (RTL).
/// - Percentages resolve against the containing block's dimensions.
pub fn apply_relative_offset(
    fragment: &mut Fragment,
    style: &ComputedStyle,
    containing_block_inline_size: LayoutUnit,
    containing_block_block_size: LayoutUnit,
) {
    if style.position != Position::Relative {
        return;
    }

    let offset = compute_relative_offset(
        style,
        containing_block_inline_size,
        containing_block_block_size,
    );

    fragment.offset.left += offset.left;
    fragment.offset.top += offset.top;
}

/// Compute the relative positioning offset without mutating a fragment.
///
/// Resolves `top`/`bottom` and `left`/`right` per CSS 2.1 §9.4.3:
///
/// **Block axis (top/bottom):**
/// - If `top` is not `auto`, use it.
/// - Else if `bottom` is not `auto`, use the negation.
/// - Both `auto` → zero offset.
/// - Both specified → `bottom` is ignored (top wins).
///
/// **Inline axis (left/right):**
/// - LTR: if `left` is not `auto`, use it; else negate `right`. Both specified → `right` ignored.
/// - RTL: if `right` is not `auto`, negate it; else use `left`. Both specified → `left` ignored.
fn compute_relative_offset(
    style: &ComputedStyle,
    containing_block_inline_size: LayoutUnit,
    containing_block_block_size: LayoutUnit,
) -> PhysicalOffset {
    let zero = LayoutUnit::zero();

    // ── Block axis (top / bottom) ────────────────────────────────────
    // CSS 2.1 §9.4.3: If both are specified, bottom is ignored.
    let block_offset = if !style.top.is_auto() {
        resolve_length(&style.top, containing_block_block_size, zero, zero)
    } else if !style.bottom.is_auto() {
        -resolve_length(&style.bottom, containing_block_block_size, zero, zero)
    } else {
        zero
    };

    // ── Inline axis (left / right) ───────────────────────────────────
    // CSS 2.1 §9.4.3: In LTR, if both are specified, right is ignored.
    //                  In RTL, if both are specified, left is ignored.
    let inline_offset = if style.direction == Direction::Rtl {
        // RTL: right takes precedence
        if !style.right.is_auto() {
            -resolve_length(&style.right, containing_block_inline_size, zero, zero)
        } else if !style.left.is_auto() {
            resolve_length(&style.left, containing_block_inline_size, zero, zero)
        } else {
            zero
        }
    } else {
        // LTR: left takes precedence
        if !style.left.is_auto() {
            resolve_length(&style.left, containing_block_inline_size, zero, zero)
        } else if !style.right.is_auto() {
            -resolve_length(&style.right, containing_block_inline_size, zero, zero)
        } else {
            zero
        }
    };

    PhysicalOffset::new(inline_offset, block_offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::{Length, PhysicalSize};
    use openui_style::Display;

    /// Helper: create a minimal fragment at a given offset.
    fn make_fragment(left: i32, top: i32) -> Fragment {
        let mut f = Fragment::new_box(
            openui_dom::NodeId::NONE,
            PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(50)),
        );
        f.offset = PhysicalOffset::new(LayoutUnit::from_i32(left), LayoutUnit::from_i32(top));
        f
    }

    /// Helper: create a relative-positioned style with custom offsets.
    fn make_relative_style(
        top: Length,
        right: Length,
        bottom: Length,
        left: Length,
    ) -> ComputedStyle {
        let mut s = ComputedStyle::initial();
        s.display = Display::Block;
        s.position = Position::Relative;
        s.top = top;
        s.right = right;
        s.bottom = bottom;
        s.left = left;
        s
    }

    fn cb_inline() -> LayoutUnit { LayoutUnit::from_i32(800) }
    fn cb_block() -> LayoutUnit { LayoutUnit::from_i32(600) }

    // ── Test 1: top only ─────────────────────────────────────────────

    #[test]
    fn top_only() {
        let mut frag = make_fragment(10, 20);
        let style = make_relative_style(
            Length::px(30.0), Length::auto(), Length::auto(), Length::auto(),
        );
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        assert_eq!(frag.offset.top.to_i32(), 50); // 20 + 30
        assert_eq!(frag.offset.left.to_i32(), 10); // unchanged
    }

    // ── Test 2: left only ────────────────────────────────────────────

    #[test]
    fn left_only() {
        let mut frag = make_fragment(10, 20);
        let style = make_relative_style(
            Length::auto(), Length::auto(), Length::auto(), Length::px(15.0),
        );
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        assert_eq!(frag.offset.left.to_i32(), 25); // 10 + 15
        assert_eq!(frag.offset.top.to_i32(), 20); // unchanged
    }

    // ── Test 3: bottom only ──────────────────────────────────────────

    #[test]
    fn bottom_only() {
        let mut frag = make_fragment(10, 100);
        let style = make_relative_style(
            Length::auto(), Length::auto(), Length::px(40.0), Length::auto(),
        );
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        assert_eq!(frag.offset.top.to_i32(), 60); // 100 - 40
        assert_eq!(frag.offset.left.to_i32(), 10); // unchanged
    }

    // ── Test 4: right only ───────────────────────────────────────────

    #[test]
    fn right_only() {
        let mut frag = make_fragment(50, 20);
        let style = make_relative_style(
            Length::auto(), Length::px(25.0), Length::auto(), Length::auto(),
        );
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        assert_eq!(frag.offset.left.to_i32(), 25); // 50 - 25
        assert_eq!(frag.offset.top.to_i32(), 20); // unchanged
    }

    // ── Test 5: top + left combination ───────────────────────────────

    #[test]
    fn top_and_left() {
        let mut frag = make_fragment(0, 0);
        let style = make_relative_style(
            Length::px(10.0), Length::auto(), Length::auto(), Length::px(20.0),
        );
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        assert_eq!(frag.offset.top.to_i32(), 10);
        assert_eq!(frag.offset.left.to_i32(), 20);
    }

    // ── Test 6: top + bottom (bottom is ignored) ─────────────────────

    #[test]
    fn top_and_bottom_ignores_bottom() {
        let mut frag = make_fragment(0, 0);
        let style = make_relative_style(
            Length::px(10.0), Length::auto(), Length::px(999.0), Length::auto(),
        );
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        // top wins — bottom is ignored per CSS 2.1 §9.4.3
        assert_eq!(frag.offset.top.to_i32(), 10);
    }

    // ── Test 7: left + right in LTR (right ignored) ─────────────────

    #[test]
    fn left_and_right_ltr_ignores_right() {
        let mut frag = make_fragment(0, 0);
        let mut style = make_relative_style(
            Length::auto(), Length::px(999.0), Length::auto(), Length::px(5.0),
        );
        style.direction = Direction::Ltr;
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        // LTR: left wins — right is ignored
        assert_eq!(frag.offset.left.to_i32(), 5);
    }

    // ── Test 8: left + right in RTL (left ignored) ──────────────────

    #[test]
    fn left_and_right_rtl_ignores_left() {
        let mut frag = make_fragment(100, 0);
        let mut style = make_relative_style(
            Length::auto(), Length::px(30.0), Length::auto(), Length::px(999.0),
        );
        style.direction = Direction::Rtl;
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        // RTL: right wins — left is ignored. right: 30px → offset.left -= 30
        assert_eq!(frag.offset.left.to_i32(), 70); // 100 - 30
    }

    // ── Test 9: percentage values ────────────────────────────────────

    #[test]
    fn percentage_offsets() {
        let mut frag = make_fragment(0, 0);
        let style = make_relative_style(
            Length::percent(10.0),  // 10% of 600 = 60
            Length::auto(),
            Length::auto(),
            Length::percent(25.0),  // 25% of 800 = 200
        );
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        assert_eq!(frag.offset.top.to_i32(), 60);
        assert_eq!(frag.offset.left.to_i32(), 200);
    }

    // ── Test 10: position:relative with no offsets (no effect) ───────

    #[test]
    fn relative_no_offsets() {
        let mut frag = make_fragment(42, 77);
        let style = make_relative_style(
            Length::auto(), Length::auto(), Length::auto(), Length::auto(),
        );
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        // All offsets are auto → no movement
        assert_eq!(frag.offset.left.to_i32(), 42);
        assert_eq!(frag.offset.top.to_i32(), 77);
    }

    // ── Test 11: position:static is a no-op ─────────────────────────

    #[test]
    fn static_position_is_noop() {
        let mut frag = make_fragment(10, 20);
        let mut style = ComputedStyle::initial();
        style.position = Position::Static;
        style.top = Length::px(999.0);
        style.left = Length::px(999.0);
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        // position:static → offsets are not applied
        assert_eq!(frag.offset.left.to_i32(), 10);
        assert_eq!(frag.offset.top.to_i32(), 20);
    }

    // ── Test 12: negative offsets ────────────────────────────────────

    #[test]
    fn negative_offsets() {
        let mut frag = make_fragment(50, 100);
        let style = make_relative_style(
            Length::px(-20.0), Length::auto(), Length::auto(), Length::px(-10.0),
        );
        apply_relative_offset(&mut frag, &style, cb_inline(), cb_block());
        assert_eq!(frag.offset.top.to_i32(), 80);   // 100 + (-20)
        assert_eq!(frag.offset.left.to_i32(), 40);  // 50 + (-10)
    }

    // ── Test 13: nested relative positioning ─────────────────────────
    // Relative offsets stack — a child positioned relative inside a parent
    // positioned relative simply receives its own offset on top of normal flow.

    #[test]
    fn nested_relative_positioning() {
        // Simulate parent: normal flow at (0,0), relative offset top:10 left:20
        let mut parent = make_fragment(0, 0);
        let parent_style = make_relative_style(
            Length::px(10.0), Length::auto(), Length::auto(), Length::px(20.0),
        );
        apply_relative_offset(&mut parent, &parent_style, cb_inline(), cb_block());
        assert_eq!(parent.offset.top.to_i32(), 10);
        assert_eq!(parent.offset.left.to_i32(), 20);

        // Child inside parent: normal flow at (5,30) within parent, relative top:3 left:7
        let mut child = make_fragment(5, 30);
        let child_style = make_relative_style(
            Length::px(3.0), Length::auto(), Length::auto(), Length::px(7.0),
        );
        apply_relative_offset(&mut child, &child_style, cb_inline(), cb_block());
        // Child's offset is relative to parent's content box.
        // The relative offset stacks independently.
        assert_eq!(child.offset.top.to_i32(), 33);   // 30 + 3
        assert_eq!(child.offset.left.to_i32(), 12);  // 5 + 7
    }
}
