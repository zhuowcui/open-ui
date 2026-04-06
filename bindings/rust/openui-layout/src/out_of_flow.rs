//! Out-of-flow layout — CSS 2.1 §10.3.7 / §10.6.4 (absolute & fixed positioning).
//!
//! Ported from Blink's `OutOfFlowLayoutPart` (core/layout/out_of_flow_layout_part.cc).
//!
//! Absolutely and fixed positioned elements are removed from normal flow.
//! Their position and size are resolved against their containing block using
//! the constraint equations from CSS 2.1 §10.3.7 (horizontal) and §10.6.4
//! (vertical).

use openui_geometry::{LayoutUnit, BoxStrut, PhysicalOffset, PhysicalSize};
use openui_style::{ComputedStyle, Direction, BoxSizing};
use openui_dom::{Document, NodeId};

use crate::block::{block_layout, resolve_border, resolve_padding};
use crate::constraint_space::ConstraintSpace;
use crate::fragment::Fragment;
use crate::intrinsic_sizing::compute_intrinsic_block_sizes;
use crate::length_resolver::{resolve_length, resolve_margin_or_padding};

/// A candidate for out-of-flow layout, collected during the in-flow pass.
///
/// When block layout encounters an absolutely or fixed positioned child, it
/// records this struct instead of laying out the child inline. After the
/// in-flow pass completes, `layout_out_of_flow_children()` processes these.
#[derive(Debug, Clone)]
pub struct OutOfFlowCandidate {
    /// The DOM node of the out-of-flow element.
    pub node_id: NodeId,
    /// The computed style snapshot (includes position, insets, size).
    pub style: ComputedStyle,
    /// The static position — where this element would appear if `position: static`.
    pub static_position: PhysicalOffset,
    /// The size of the containing block (for resolving percentages and insets).
    pub containing_block_size: PhysicalSize,
    /// The border of the containing block. Used to convert from padding-edge
    /// coordinates (where abs-pos insets are measured) to the parent's
    /// border-box coordinates (where fragment offsets are stored).
    pub containing_block_border: BoxStrut,
}

/// Layout all out-of-flow candidates and return positioned fragments.
///
/// This is the main entry point, equivalent to Blink's
/// `OutOfFlowLayoutPart::Run()`. For each candidate it:
/// 1. Resolves horizontal position and width (CSS 2.1 §10.3.7)
/// 2. Resolves vertical position and height (CSS 2.1 §10.6.4)
/// 3. Lays out the child with a constraint space derived from the resolved size
/// 4. Positions the resulting fragment
pub fn layout_out_of_flow_children(
    doc: &Document,
    candidates: &[OutOfFlowCandidate],
) -> Vec<Fragment> {
    let mut fragments = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        let fragment = layout_out_of_flow_child(doc, candidate);
        fragments.push(fragment);
    }

    fragments
}

/// Layout a single out-of-flow child.
fn layout_out_of_flow_child(
    doc: &Document,
    candidate: &OutOfFlowCandidate,
) -> Fragment {
    let style = &candidate.style;
    let cb_width = candidate.containing_block_size.width;
    let cb_height = candidate.containing_block_size.height;
    let cb_border = &candidate.containing_block_border;

    // Static position is in parent border-box coordinates. Convert to
    // padding-edge coordinates for the constraint equations.
    let static_left = candidate.static_position.left - cb_border.left;
    let static_top = candidate.static_position.top - cb_border.top;

    let border = resolve_border(style);
    let padding = resolve_padding(style, cb_width);

    let border_padding_h = border.left + border.right + padding.left + padding.right;
    let border_padding_v = border.top + border.bottom + padding.top + padding.bottom;

    // Compute shrink-to-fit width from intrinsic sizes (CSS 2.1 §10.3.7).
    // shrink-to-fit = min(max-content, max(min-content, available))
    // intrinsic sizes include border+padding, so convert to content-box.
    let intrinsic = compute_intrinsic_block_sizes(doc, candidate.node_id);
    let shrink_to_fit_max = (intrinsic.max_content_inline_size - border_padding_h)
        .clamp_negative_to_zero();
    let shrink_to_fit_min = (intrinsic.min_content_inline_size - border_padding_h)
        .clamp_negative_to_zero();

    // Resolve horizontal axis (CSS 2.1 §10.3.7)
    let (resolved_left, resolved_width, resolved_margin_left, resolved_margin_right) =
        resolve_horizontal(style, cb_width, static_left, &border, &padding,
                          shrink_to_fit_min, shrink_to_fit_max);

    // Resolve vertical axis (CSS 2.1 §10.6.4)
    let (resolved_top, resolved_height, resolved_margin_top, resolved_margin_bottom) =
        resolve_vertical(style, cb_width, cb_height, static_top, &border, &padding);

    // The content-box width for the child constraint space
    let content_width = (resolved_width - border_padding_h).clamp_negative_to_zero();
    let content_height = (resolved_height - border_padding_v).clamp_negative_to_zero();

    // Create constraint space and lay out the child
    let child_space = ConstraintSpace::for_block_child(
        content_width,
        content_height,
        content_width,
        content_height,
        true, // Abs-pos elements establish new formatting contexts
    );

    let mut child_fragment = block_layout(doc, candidate.node_id, &child_space);

    // For auto-width or auto-height, use the actual laid-out size
    // UNLESS the dimension was fully resolved from the constraint equation
    // (i.e., left+right or top+bottom were both specified).
    let width_resolved_from_constraints = style.width.is_auto()
        && !style.left.is_auto() && !style.right.is_auto();
    let height_resolved_from_constraints = style.height.is_auto()
        && !style.top.is_auto() && !style.bottom.is_auto();

    let final_width = if style.width.is_auto() && !width_resolved_from_constraints {
        child_fragment.size.width
    } else {
        resolved_width
    };
    let final_height = if style.height.is_auto() && !height_resolved_from_constraints {
        child_fragment.size.height
    } else {
        resolved_height
    };

    // Recompute left when width changed after layout (e.g., max-width constraint).
    // If left:auto and right:specified, left depends on the final width.
    let final_left = if final_width != resolved_width && style.left.is_auto() && !style.right.is_auto() {
        let zero = LayoutUnit::zero();
        let right_val = resolve_length(&style.right, cb_width, zero, zero);
        let ml = resolved_margin_left;
        let mr = resolved_margin_right;
        cb_width - right_val - mr - final_width - ml
    } else {
        resolved_left
    };

    // Recompute top when height was auto-sized and affects the constraint
    // equation. Per CSS 2.1 §10.6.4, when top is auto and height was
    // auto-sized (content-determined), we must use the final height to
    // correctly position the element.
    let final_top = if style.height.is_auto() && !height_resolved_from_constraints {
        if style.top.is_auto() && !style.bottom.is_auto() {
            let zero = LayoutUnit::zero();
            let bottom_val = resolve_length(&style.bottom, cb_height, zero, zero);
            let mt = resolved_margin_top;
            let mb = resolved_margin_bottom;
            cb_height - bottom_val - mb - final_height - mt
        } else {
            resolved_top
        }
    } else {
        resolved_top
    };

    child_fragment.size = PhysicalSize::new(final_width, final_height);
    // CSS 2.1 §10.3.7/§10.6.4: insets are measured from the containing block's
    // padding edge. Fragment offsets are in the parent's border-box coordinates.
    // Add the containing block's border to convert from padding-edge to border-box.
    let cb_border = &candidate.containing_block_border;
    child_fragment.offset = PhysicalOffset::new(
        final_left + cb_border.left,
        final_top + cb_border.top,
    );
    child_fragment.border = border;
    child_fragment.padding = padding;
    child_fragment.margin = BoxStrut::new(
        resolved_margin_top,
        resolved_margin_right,
        resolved_margin_bottom,
        resolved_margin_left,
    );

    child_fragment
}

/// Resolve horizontal position and width per CSS 2.1 §10.3.7.
///
/// The constraint equation for absolutely positioned, non-replaced elements:
///
///   left + margin_left + border_left + padding_left + width +
///   padding_right + border_right + margin_right + right = CB_width
///
/// `shrink_to_fit_min` and `shrink_to_fit_max` are content-box values
/// (intrinsic sizes with border+padding subtracted).
///
/// Returns `(left, border_box_width, margin_left, margin_right)`.
fn resolve_horizontal(
    style: &ComputedStyle,
    cb_width: LayoutUnit,
    static_left: LayoutUnit,
    border: &BoxStrut,
    padding: &BoxStrut,
    shrink_to_fit_min: LayoutUnit,
    shrink_to_fit_max: LayoutUnit,
) -> (LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit) {
    let zero = LayoutUnit::zero();

    let border_padding_h = border.left + border.right + padding.left + padding.right;

    // Resolve specified values (auto remains as a sentinel)
    let left_auto = style.left.is_auto();
    let right_auto = style.right.is_auto();
    let width_auto = style.width.is_auto();

    let left_val = if left_auto { zero } else {
        resolve_length(&style.left, cb_width, zero, zero)
    };
    let right_val = if right_auto { zero } else {
        resolve_length(&style.right, cb_width, zero, zero)
    };
    // Resolve width and convert to border-box, accounting for box-sizing.
    let (_width_val, border_box_from_specified) = if width_auto {
        (zero, zero)
    } else {
        let raw = resolve_length(&style.width, cb_width, zero, zero);
        if style.box_sizing == BoxSizing::BorderBox {
            // width is already border-box — content-width = width - bp
            let content = (raw - border_padding_h).clamp_negative_to_zero();
            (content, raw.max_of(border_padding_h))
        } else {
            (raw, raw + border_padding_h)
        }
    };

    // Resolve margins — auto margins are handled specially below
    let margin_left_auto = style.margin_left.is_auto();
    let margin_right_auto = style.margin_right.is_auto();
    let margin_left_val = if margin_left_auto { zero } else {
        resolve_margin_or_padding(&style.margin_left, cb_width)
    };
    let margin_right_val = if margin_right_auto { zero } else {
        resolve_margin_or_padding(&style.margin_right, cb_width)
    };

    // CSS 2.1 §10.3.7: Determine which values are auto and solve the equation.

    if !left_auto && !width_auto && !right_auto {
        // ── Case 1: None are auto — possibly over-constrained ────────
        let border_box_width = border_box_from_specified;

        if margin_left_auto && margin_right_auto {
            // Auto margins absorb remaining space (centering)
            let remaining = cb_width - left_val - right_val - border_box_width;
            if remaining >= zero {
                let half = remaining / 2;
                return (left_val + half, border_box_width, half, remaining - half);
            } else {
                // Negative available space: per CSS 2.1 §10.3.7,
                // LTR → margin-left=0, margin-right absorbs the deficit.
                // RTL → margin-right=0, margin-left absorbs the deficit.
                if style.direction == Direction::Rtl {
                    return (left_val + remaining, border_box_width, remaining, zero);
                } else {
                    return (left_val, border_box_width, zero, remaining);
                }
            }
        }

        if margin_left_auto {
            let ml = cb_width - left_val - right_val - border_box_width - margin_right_val;
            return (left_val + ml, border_box_width, ml, margin_right_val);
        }

        if margin_right_auto {
            let mr = cb_width - left_val - right_val - border_box_width - margin_left_val;
            return (left_val + margin_left_val, border_box_width, margin_left_val, mr);
        }

        // Over-constrained: all specified including margins
        // LTR: ignore right. RTL: ignore left.
        if style.direction == Direction::Rtl {
            // Ignore left, recompute it
            let new_left = cb_width - right_val - margin_left_val - border_box_width - margin_right_val;
            return (new_left + margin_left_val, border_box_width, margin_left_val, margin_right_val);
        } else {
            // Ignore right (LTR default)
            return (left_val + margin_left_val, border_box_width, margin_left_val, margin_right_val);
        }
    }

    // For the remaining cases, treat auto margins as zero
    let ml = if margin_left_auto { zero } else { margin_left_val };
    let mr = if margin_right_auto { zero } else { margin_right_val };

    if width_auto && left_auto && right_auto {
        // ── All three auto: use static position for left, shrink-to-fit for width
        let left = static_left;
        let available = (cb_width - left - ml - mr - border_padding_h).clamp_negative_to_zero();
        let width = shrink_to_fit_max.min_of(shrink_to_fit_min.max_of(available));
        let border_box_width = width + border_padding_h;
        return (left + ml, border_box_width, ml, mr);
    }

    if width_auto && left_auto {
        // left and width auto, right specified
        // Shrink-to-fit width, then left = CB - right - margins - width
        let available = (cb_width - right_val - ml - mr - border_padding_h).clamp_negative_to_zero();
        let width = shrink_to_fit_max.min_of(shrink_to_fit_min.max_of(available));
        let border_box_width = width + border_padding_h;
        let left = cb_width - right_val - mr - border_box_width - ml;
        return (left + ml, border_box_width, ml, mr);
    }

    if width_auto && right_auto {
        // width and right auto, left specified
        // Shrink-to-fit width, right is computed
        let available = (cb_width - left_val - ml - mr - border_padding_h).clamp_negative_to_zero();
        let width = shrink_to_fit_max.min_of(shrink_to_fit_min.max_of(available));
        let border_box_width = width + border_padding_h;
        return (left_val + ml, border_box_width, ml, mr);
    }

    if left_auto && right_auto {
        // left and right auto, width specified
        // Use static position for left
        let border_box_width = border_box_from_specified;
        let left = static_left;
        return (left + ml, border_box_width, ml, mr);
    }

    if left_auto {
        // Only left is auto
        let border_box_width = border_box_from_specified;
        let left = cb_width - right_val - mr - border_box_width - ml;
        return (left + ml, border_box_width, ml, mr);
    }

    if right_auto {
        // Only right is auto
        let border_box_width = border_box_from_specified;
        return (left_val + ml, border_box_width, ml, mr);
    }

    if width_auto {
        // Only width is auto
        let width = (cb_width - left_val - right_val - ml - mr - border_padding_h)
            .clamp_negative_to_zero();
        let border_box_width = width + border_padding_h;
        return (left_val + ml, border_box_width, ml, mr);
    }

    // Fallback (shouldn't reach here)
    (static_left, zero, zero, zero)
}

/// Resolve vertical position and height per CSS 2.1 §10.6.4.
///
/// The constraint equation for absolutely positioned, non-replaced elements:
///
///   top + margin_top + border_top + padding_top + height +
///   padding_bottom + border_bottom + margin_bottom + bottom = CB_height
///
/// Returns `(top, border_box_height, margin_top, margin_bottom)`.
fn resolve_vertical(
    style: &ComputedStyle,
    cb_width: LayoutUnit,
    cb_height: LayoutUnit,
    static_top: LayoutUnit,
    border: &BoxStrut,
    padding: &BoxStrut,
) -> (LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit) {
    let zero = LayoutUnit::zero();

    let border_padding_v = border.top + border.bottom + padding.top + padding.bottom;

    let top_auto = style.top.is_auto();
    let bottom_auto = style.bottom.is_auto();
    let height_auto = style.height.is_auto();

    let top_val = if top_auto { zero } else {
        resolve_length(&style.top, cb_height, zero, zero)
    };
    let bottom_val = if bottom_auto { zero } else {
        resolve_length(&style.bottom, cb_height, zero, zero)
    };
    // Resolve height and convert to border-box, accounting for box-sizing.
    let height_val_bb = if height_auto {
        border_padding_v // auto height → content-sized (0 + bp)
    } else {
        let raw = resolve_length(&style.height, cb_height, zero, zero);
        if style.box_sizing == BoxSizing::BorderBox {
            raw.max_of(border_padding_v)
        } else {
            raw + border_padding_v
        }
    };

    // CSS 2.1 §8.3: Percentage margins resolve against containing block WIDTH,
    // even for vertical margins. This is true for all four margins.
    let margin_top_auto = style.margin_top.is_auto();
    let margin_bottom_auto = style.margin_bottom.is_auto();
    let margin_top_val = if margin_top_auto { zero } else {
        resolve_margin_or_padding(&style.margin_top, cb_width)
    };
    let margin_bottom_val = if margin_bottom_auto { zero } else {
        resolve_margin_or_padding(&style.margin_bottom, cb_width)
    };

    if !top_auto && !height_auto && !bottom_auto {
        // ── Case 1: None are auto — possibly over-constrained ────────
        let border_box_height = height_val_bb;

        if margin_top_auto && margin_bottom_auto {
            let remaining = cb_height - top_val - bottom_val - border_box_height;
            if remaining >= zero {
                let half = remaining / 2;
                return (top_val + half, border_box_height, half, remaining - half);
            } else {
                // Per CSS 2.1 §10.6.4, if margins are negative, top margin = 0
                return (top_val, border_box_height, zero, remaining);
            }
        }

        if margin_top_auto {
            let mt = cb_height - top_val - bottom_val - border_box_height - margin_bottom_val;
            return (top_val + mt, border_box_height, mt, margin_bottom_val);
        }

        if margin_bottom_auto {
            let mb = cb_height - top_val - bottom_val - border_box_height - margin_top_val;
            return (top_val + margin_top_val, border_box_height, margin_top_val, mb);
        }

        // Over-constrained: ignore bottom (always, unlike horizontal)
        return (top_val + margin_top_val, border_box_height, margin_top_val, margin_bottom_val);
    }

    // For remaining cases, treat auto margins as zero
    let mt = if margin_top_auto { zero } else { margin_top_val };
    let mb = if margin_bottom_auto { zero } else { margin_bottom_val };

    if height_auto && top_auto && bottom_auto {
        // All three auto: use static position for top, auto height
        let top = static_top;
        let border_box_height = height_val_bb; // auto height → content-sized (0 for now)
        return (top + mt, border_box_height, mt, mb);
    }

    if height_auto && top_auto {
        // height and top auto, bottom specified
        let border_box_height = height_val_bb; // auto height → content-sized
        let top = cb_height - bottom_val - mb - border_box_height - mt;
        return (top + mt, border_box_height, mt, mb);
    }

    if height_auto && bottom_auto {
        // height and bottom auto, top specified
        let border_box_height = height_val_bb; // auto height → content-sized
        return (top_val + mt, border_box_height, mt, mb);
    }

    if top_auto && bottom_auto {
        // top and bottom auto, height specified
        let border_box_height = height_val_bb;
        let top = static_top;
        return (top + mt, border_box_height, mt, mb);
    }

    if top_auto {
        // Only top is auto
        let border_box_height = height_val_bb;
        let top = cb_height - bottom_val - mb - border_box_height - mt;
        return (top + mt, border_box_height, mt, mb);
    }

    if bottom_auto {
        // Only bottom is auto
        let border_box_height = height_val_bb;
        return (top_val + mt, border_box_height, mt, mb);
    }

    if height_auto {
        // Only height is auto
        let height = (cb_height - top_val - bottom_val - mt - mb - border_padding_v)
            .clamp_negative_to_zero();
        let border_box_height = height + border_padding_v;
        return (top_val + mt, border_box_height, mt, mb);
    }

    // Fallback
    (static_top, zero, zero, zero)
}

/// Compute shrink-to-fit width for auto-width elements.
///
/// CSS 2.1 §10.3.5/7: shrink-to-fit = min(preferred, max(minimum, available))
/// where preferred = max-content width, minimum = min-content width.
///
/// For float layout, call this with the child's intrinsic sizes and available width.
pub fn compute_shrink_to_fit_width(
    doc: &Document,
    node_id: NodeId,
    available: LayoutUnit,
) -> LayoutUnit {
    let intrinsic = compute_intrinsic_block_sizes(doc, node_id);
    let preferred = intrinsic.max_content_inline_size;
    let minimum = intrinsic.min_content_inline_size;
    // shrink-to-fit = min(preferred, max(minimum, available))
    preferred.min_of(minimum.max_of(available)).clamp_negative_to_zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::Length;
    use openui_style::{Display, Position};

    fn make_abs_style() -> ComputedStyle {
        let mut s = ComputedStyle::initial();
        s.display = Display::Block;
        s.position = Position::Absolute;
        s
    }

    fn cb_size(w: i32, h: i32) -> PhysicalSize {
        PhysicalSize::new(LayoutUnit::from_i32(w), LayoutUnit::from_i32(h))
    }

    fn static_pos(l: i32, t: i32) -> PhysicalOffset {
        PhysicalOffset::new(LayoutUnit::from_i32(l), LayoutUnit::from_i32(t))
    }

    #[test]
    fn resolve_horizontal_all_specified() {
        let mut style = make_abs_style();
        style.left = Length::px(10.0);
        style.right = Length::px(20.0);
        style.width = Length::px(100.0);
        let border = BoxStrut::zero();
        let padding = BoxStrut::zero();
        let stf_min = LayoutUnit::from_i32(800);
        let stf_max = LayoutUnit::from_i32(800);
        let (left, width, ml, mr) = resolve_horizontal(
            &style, LayoutUnit::from_i32(800), LayoutUnit::zero(), &border, &padding, stf_min, stf_max,
        );
    }

    #[test]
    fn resolve_horizontal_auto_margins_center() {
        let mut style = make_abs_style();
        style.left = Length::px(0.0);
        style.right = Length::px(0.0);
        style.width = Length::px(200.0);
        style.margin_left = Length::auto();
        style.margin_right = Length::auto();
        let border = BoxStrut::zero();
        let padding = BoxStrut::zero();
        let stf_min = LayoutUnit::from_i32(800);
        let stf_max = LayoutUnit::from_i32(800);
        let (left, width, ml, mr) = resolve_horizontal(
            &style, LayoutUnit::from_i32(800), LayoutUnit::zero(), &border, &padding, stf_min, stf_max,
        );
        assert_eq!(ml.to_i32(), 300);
        assert_eq!(mr.to_i32(), 300);
        assert_eq!(left.to_i32(), 300);
        assert_eq!(width.to_i32(), 200);
    }

    #[test]
    fn resolve_vertical_all_specified() {
        let mut style = make_abs_style();
        style.top = Length::px(50.0);
        style.bottom = Length::px(30.0);
        style.height = Length::px(200.0);
        let border = BoxStrut::zero();
        let padding = BoxStrut::zero();
        let (top, height, mt, mb) = resolve_vertical(
            &style, LayoutUnit::from_i32(800), LayoutUnit::from_i32(600), LayoutUnit::zero(), &border, &padding,
        );
        assert_eq!(top.to_i32(), 50);
        assert_eq!(height.to_i32(), 200);
    }

    #[test]
    fn resolve_vertical_auto_height() {
        let mut style = make_abs_style();
        style.top = Length::px(10.0);
        style.bottom = Length::px(20.0);
        // height is auto
        let border = BoxStrut::zero();
        let padding = BoxStrut::zero();
        let (top, height, _, _) = resolve_vertical(
            &style, LayoutUnit::from_i32(800), LayoutUnit::from_i32(600), LayoutUnit::zero(), &border, &padding,
        );
        assert_eq!(top.to_i32(), 10);
        // height = 600 - 10 - 20 = 570
        assert_eq!(height.to_i32(), 570);
    }

    #[test]
    fn shrink_to_fit_empty_doc() {
        // An empty document node has zero intrinsic size, so shrink-to-fit = 0
        let doc = Document::new();
        let root = doc.root();
        let result = compute_shrink_to_fit_width(&doc, root, LayoutUnit::from_i32(300));
        // Result is min(max_content=0, max(min_content=0, 300)) = 0
        assert_eq!(result.to_i32(), 0);
    }

    #[test]
    fn shrink_to_fit_negative_available_clamped() {
        let doc = Document::new();
        let root = doc.root();
        let result = compute_shrink_to_fit_width(&doc, root, LayoutUnit::from_i32(-10));
        assert_eq!(result.to_i32(), 0);
    }
}
