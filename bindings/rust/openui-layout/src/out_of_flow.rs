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
    /// The direction of the containing block. CSS 2.1 §10.3.7 requires this
    /// (not the element's own direction) for auto-margin / over-constrained
    /// resolution.
    pub containing_block_direction: Direction,
    /// The direction of the static-position containing block (the element's
    /// direct parent in normal flow). CSS 2.1 §10.3.7 uses this (not the
    /// actual CB's direction) to decide whether to use the static-left or
    /// static-right position when left/right/width are all auto. This field
    /// is set at candidate creation time and never overwritten during bubbling.
    pub static_position_direction: Direction,
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
    let cb_direction = candidate.containing_block_direction;
    let sp_direction = candidate.static_position_direction;
    let (resolved_left, resolved_width_raw, resolved_margin_left, resolved_margin_right) =
        resolve_horizontal(style, cb_width, static_left, &border, &padding,
                          shrink_to_fit_min, shrink_to_fit_max, cb_direction, sp_direction);

    // Resolve vertical axis (CSS 2.1 §10.6.4)
    let (resolved_top, resolved_height_raw, resolved_margin_top, resolved_margin_bottom) =
        resolve_vertical(style, cb_width, cb_height, static_top, &border, &padding);

    // CSS 2.1 §10.4 / §10.7: Apply min/max constraints to the resolved size.
    // The constraint equation (§10.3.7/§10.6.4) gives a tentative width/height.
    // If that tentative value violates min/max, re-resolve the full constraint
    // equation with the clamped value treated as specified (not auto).
    let resolved_width = apply_min_max_inline(style, cb_width, resolved_width_raw,
                                              &border, &padding);
    let resolved_height = apply_min_max_block(style, cb_height, resolved_height_raw,
                                              &border, &padding);

    // CSS 2.1 §10.4: When min/max changes the width, re-solve §10.3.7 with
    // the clamped width treated as the specified width. This is needed to
    // recompute auto margins (e.g., margin:auto centering with max-width)
    // and auto insets. For the re-solve, shrink_to_fit values are irrelevant
    // since width is now a known quantity.
    let (resolved_left, resolved_margin_left, resolved_margin_right) =
        if resolved_width != resolved_width_raw {
            let (l, _w, ml, mr) = resolve_horizontal_with_known_width(
                style, cb_width, static_left, &border, &padding, resolved_width, cb_direction, sp_direction,
            );
            (l, ml, mr)
        } else {
            (resolved_left, resolved_margin_left, resolved_margin_right)
        };

    // CSS 2.1 §10.7: When min/max changes the height, re-solve §10.6.4 with
    // the clamped height treated as the specified height.
    let (resolved_top, resolved_margin_top, resolved_margin_bottom) =
        if resolved_height != resolved_height_raw {
            let (t, _h, mt, mb) = resolve_vertical_with_known_height(
                style, cb_width, cb_height, static_top, &border, &padding, resolved_height,
            );
            (t, mt, mb)
        } else {
            (resolved_top, resolved_margin_top, resolved_margin_bottom)
        };

    // Detect whether height was fully determined by the constraint equation
    // (both opposing insets specified with auto height).
    let height_resolved_from_constraints = style.height.is_auto()
        && !style.top.is_auto() && !style.bottom.is_auto();

    // The content-box width for the child constraint space
    let content_width = (resolved_width - border_padding_h).clamp_negative_to_zero();
    let content_height = (resolved_height - border_padding_v).clamp_negative_to_zero();

    // CSS 2.1 §10.5: When the containing block's height is determined by the
    // constraint equation (top + bottom specified), it IS definite for percentage
    // resolution in descendants — even though style.height is auto.
    let child_percentage_block_size = if height_resolved_from_constraints {
        content_height
    } else if !style.height.is_auto() {
        content_height
    } else {
        openui_geometry::INDEFINITE_SIZE
    };

    // Create constraint space and lay out the child.
    // Pass border-box resolved_width/resolved_height as available sizes —
    // block_layout subtracts its own border+padding to get the content area.
    // percentage_resolution stays content-box per CSS 2.1 §10.
    // When the height is determined by constraints (top+bottom specified, height auto),
    // signal it as fixed so descendants can resolve percentage heights against it.
    // Similarly, when height is explicitly specified, it's a fixed block size.
    let fixed_block = height_resolved_from_constraints || !style.height.is_auto();
    let available_block = if fixed_block { resolved_height } else { content_height };
    let mut child_space = ConstraintSpace::for_block_child(
        resolved_width,
        available_block,
        content_width,
        child_percentage_block_size,
        true, // Abs-pos elements establish new formatting contexts
    );
    child_space.is_fixed_inline_size = true; // Width pre-determined by constraint equation
    if fixed_block {
        child_space.is_fixed_block_size = true;
    }

    let mut child_fragment = block_layout(doc, candidate.node_id, &child_space);

    // CSS 2.1 §10.3.7: The width is always pre-determined by the constraint
    // equation before child layout, so the fragment width = resolved border-box.
    let final_width = resolved_width;

    // CSS 2.1 §10.7: For auto-height content-sized abspos, the content height
    // must still be clamped by min-height / max-height constraints.
    let final_height = if style.height.is_auto() && !height_resolved_from_constraints {
        let content_height = child_fragment.size.height;
        apply_min_max_block(style, cb_height, content_height, &border, &padding)
    } else {
        resolved_height
    };

    // CSS 2.1 §10.7: When auto-height was clamped by min/max, the clamped
    // height becomes the definite height for percentage-height descendants.
    // Re-layout with the definite block size so children can resolve against it.
    // Save the original unclamped height BEFORE relayout, because relayout
    // will produce a fragment with height == final_height, making the
    // comparison meaningless afterward.
    let original_unclamped = child_fragment.size.height;
    if style.height.is_auto() && !height_resolved_from_constraints {
        if final_height != original_unclamped {
            let clamped_content = (final_height - border_padding_v).clamp_negative_to_zero();
            let mut relayout_space = ConstraintSpace::for_block_child(
                resolved_width,
                final_height,
                content_width,
                clamped_content,
                true,
            );
            relayout_space.is_fixed_inline_size = true;
            relayout_space.is_fixed_block_size = true;
            child_fragment = block_layout(doc, candidate.node_id, &relayout_space);
        }
    }

    // When the auto-height was clamped by min/max, re-solve the vertical
    // constraint equation to correctly recompute auto margins and insets.
    let (resolved_top, resolved_margin_top, resolved_margin_bottom) =
        if style.height.is_auto() && !height_resolved_from_constraints {
            if final_height != original_unclamped {
                let (t, _h, mt, mb) = resolve_vertical_with_known_height(
                    style, cb_width, cb_height, static_top, &border, &padding, final_height,
                );
                (t, mt, mb)
            } else {
                (resolved_top, resolved_margin_top, resolved_margin_bottom)
            }
        } else {
            (resolved_top, resolved_margin_top, resolved_margin_bottom)
        };

    // Recompute left when width changed after layout (e.g., shrink-to-fit).
    // If left:auto and right:specified, left depends on the final width.
    let final_left = resolved_left;

    // Recompute top when height was auto-sized (content-determined) and the
    // vertical constraint equation has a top:auto + bottom:specified pattern.
    // Note: if min/max clamped the auto-height, resolved_top was already
    // re-solved above via resolve_vertical_with_known_height.
    let final_top = if style.height.is_auto() && !height_resolved_from_constraints
        && final_height == child_fragment.size.height  // was NOT clamped by min/max
    {
        if style.top.is_auto() && !style.bottom.is_auto() {
            let zero = LayoutUnit::zero();
            let bottom_val = resolve_length(&style.bottom, cb_height, zero, zero);
            let mb = resolved_margin_bottom;
            // CSS 2.1 §10.6.4: top + mt + height + mb + bottom = cb_height
            // Solving: top = cb_height - bottom - mb - height - mt
            // Border-edge position = top + mt = cb_height - bottom - mb - height
            cb_height - bottom_val - mb - final_height
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
    cb_direction: Direction,
    sp_direction: Direction,
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
                // Use containing block's direction (not element's).
                // LTR → margin-left=0, margin-right absorbs the deficit.
                // RTL → margin-right=0, margin-left absorbs the deficit.
                if cb_direction == Direction::Rtl {
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
        // Use containing block's direction: LTR ignores right, RTL ignores left.
        if cb_direction == Direction::Rtl {
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
        // ── All three auto: use static position, shrink-to-fit for width
        // CSS 2.1 §10.3.7: Use static-position CB direction (not actual CB).
        // In LTR use static position for left; in RTL for right.
        if sp_direction == Direction::Rtl {
            // For a block-level placeholder in RTL normal flow, the right
            // margin edge is at the containing block's right padding edge,
            // so static_right = 0.
            let right = LayoutUnit::zero();
            let available = (cb_width - right - ml - mr - border_padding_h).clamp_negative_to_zero();
            let width = shrink_to_fit_max.min_of(shrink_to_fit_min.max_of(available));
            let border_box_width = width + border_padding_h;
            let left = cb_width - right - mr - border_box_width - ml;
            return (left + ml, border_box_width, ml, mr);
        } else {
            let left = static_left;
            let available = (cb_width - left - ml - mr - border_padding_h).clamp_negative_to_zero();
            let width = shrink_to_fit_max.min_of(shrink_to_fit_min.max_of(available));
            let border_box_width = width + border_padding_h;
            return (left + ml, border_box_width, ml, mr);
        }
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
        // CSS 2.1 §10.3.7: Use static-position CB direction (not actual CB).
        // In LTR use static position for left; in RTL for right.
        let border_box_width = border_box_from_specified;
        if sp_direction == Direction::Rtl {
            // Block-level static position in RTL: right margin edge at CB's
            // right padding edge → static_right = 0.
            let right = LayoutUnit::zero();
            let left = cb_width - right - mr - border_box_width - ml;
            return (left + ml, border_box_width, ml, mr);
        } else {
            let left = static_left;
            return (left + ml, border_box_width, ml, mr);
        }
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

/// Apply min-width / max-width constraints to a resolved border-box width.
///
/// CSS 2.1 §10.4: If the tentative used width is greater than max-width, the
/// rules are applied again with max-width as the computed width. If the
/// resulting width is less than min-width, the rules are applied again with
/// min-width as the computed width.
fn apply_min_max_inline(
    style: &ComputedStyle,
    cb_width: LayoutUnit,
    border_box_width: LayoutUnit,
    border: &BoxStrut,
    padding: &BoxStrut,
) -> LayoutUnit {
    let zero = LayoutUnit::zero();
    let border_padding_h = border.left + border.right + padding.left + padding.right;

    let min_raw = if style.min_width.is_auto() {
        zero
    } else {
        resolve_length(&style.min_width, cb_width, zero, zero)
    };
    let min_bb = if min_raw > zero {
        if style.box_sizing == BoxSizing::ContentBox {
            min_raw + border_padding_h
        } else {
            min_raw.max_of(border_padding_h)
        }
    } else {
        zero
    };

    let max_raw = resolve_length(
        &style.max_width, cb_width, LayoutUnit::max(), LayoutUnit::max(),
    );
    let max_bb = if max_raw == LayoutUnit::max() {
        max_raw
    } else if style.box_sizing == BoxSizing::ContentBox {
        max_raw + border_padding_h
    } else {
        max_raw.max_of(border_padding_h)
    };

    border_box_width.clamp(min_bb, max_bb)
}

/// Apply min-height / max-height constraints to a resolved border-box height.
///
/// CSS 2.1 §10.7: Same logic as §10.4 but for the block axis.
fn apply_min_max_block(
    style: &ComputedStyle,
    cb_height: LayoutUnit,
    border_box_height: LayoutUnit,
    border: &BoxStrut,
    padding: &BoxStrut,
) -> LayoutUnit {
    let zero = LayoutUnit::zero();
    let border_padding_v = border.top + border.bottom + padding.top + padding.bottom;

    let min_raw = if style.min_height.is_auto() {
        zero
    } else {
        resolve_length(&style.min_height, cb_height, zero, zero)
    };
    let min_bb = if min_raw > zero {
        if style.box_sizing == BoxSizing::ContentBox {
            min_raw + border_padding_v
        } else {
            min_raw.max_of(border_padding_v)
        }
    } else {
        zero
    };

    let max_raw = resolve_length(
        &style.max_height, cb_height, LayoutUnit::max(), LayoutUnit::max(),
    );
    let max_bb = if max_raw == LayoutUnit::max() {
        max_raw
    } else if style.box_sizing == BoxSizing::ContentBox {
        max_raw + border_padding_v
    } else {
        max_raw.max_of(border_padding_v)
    };

    border_box_height.clamp(min_bb, max_bb)
}

/// Re-solve the horizontal constraint equation with a known border-box width.
///
/// CSS 2.1 §10.4: When min/max constrains the width, re-run §10.3.7 treating
/// the clamped width as the specified width. This properly recomputes auto
/// margins (e.g., centering via `margin: auto` with `max-width`) and auto
/// insets (e.g., `left: auto` + `right: 50px`).
///
/// Returns `(left, border_box_width, margin_left, margin_right)`.
fn resolve_horizontal_with_known_width(
    style: &ComputedStyle,
    cb_width: LayoutUnit,
    static_left: LayoutUnit,
    _border: &BoxStrut,
    _padding: &BoxStrut,
    border_box_width: LayoutUnit,
    cb_direction: Direction,
    sp_direction: Direction,
) -> (LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit) {
    let zero = LayoutUnit::zero();

    let left_auto = style.left.is_auto();
    let right_auto = style.right.is_auto();

    let left_val = if left_auto { zero } else {
        resolve_length(&style.left, cb_width, zero, zero)
    };
    let right_val = if right_auto { zero } else {
        resolve_length(&style.right, cb_width, zero, zero)
    };

    let margin_left_auto = style.margin_left.is_auto();
    let margin_right_auto = style.margin_right.is_auto();
    let margin_left_val = if margin_left_auto { zero } else {
        resolve_margin_or_padding(&style.margin_left, cb_width)
    };
    let margin_right_val = if margin_right_auto { zero } else {
        resolve_margin_or_padding(&style.margin_right, cb_width)
    };

    // Width is now known (clamped), so the equation has at most two unknowns.
    // Re-solve with width treated as specified.
    if !left_auto && !right_auto {
        // Both insets specified — Case 1: auto margins absorb space
        if margin_left_auto && margin_right_auto {
            let remaining = cb_width - left_val - right_val - border_box_width;
            if remaining >= zero {
                let half = remaining / 2;
                return (left_val + half, border_box_width, half, remaining - half);
            } else {
                if cb_direction == Direction::Rtl {
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
        // Over-constrained: ignore right in LTR, ignore left in RTL
        if cb_direction == Direction::Rtl {
            let new_left = cb_width - right_val - margin_left_val - border_box_width - margin_right_val;
            return (new_left + margin_left_val, border_box_width, margin_left_val, margin_right_val);
        } else {
            return (left_val + margin_left_val, border_box_width, margin_left_val, margin_right_val);
        }
    }

    let ml = if margin_left_auto { zero } else { margin_left_val };
    let mr = if margin_right_auto { zero } else { margin_right_val };

    if left_auto && right_auto {
        // Use static-position CB direction for static position choice
        if sp_direction == Direction::Rtl {
            let right = LayoutUnit::zero();
            let left = cb_width - right - mr - border_box_width - ml;
            return (left + ml, border_box_width, ml, mr);
        } else {
            let left = static_left;
            return (left + ml, border_box_width, ml, mr);
        }
    }

    if left_auto {
        let left = cb_width - right_val - mr - border_box_width - ml;
        return (left + ml, border_box_width, ml, mr);
    }

    if right_auto {
        return (left_val + ml, border_box_width, ml, mr);
    }

    (static_left, border_box_width, zero, zero)
}

/// Re-solve the vertical constraint equation with a known border-box height.
///
/// CSS 2.1 §10.7: When min/max constrains the height, re-run §10.6.4 treating
/// the clamped height as the specified height.
///
/// Returns `(top, border_box_height, margin_top, margin_bottom)`.
fn resolve_vertical_with_known_height(
    style: &ComputedStyle,
    cb_width: LayoutUnit,
    cb_height: LayoutUnit,
    static_top: LayoutUnit,
    _border: &BoxStrut,
    _padding: &BoxStrut,
    border_box_height: LayoutUnit,
) -> (LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit) {
    let zero = LayoutUnit::zero();

    let top_auto = style.top.is_auto();
    let bottom_auto = style.bottom.is_auto();

    let top_val = if top_auto { zero } else {
        resolve_length(&style.top, cb_height, zero, zero)
    };
    let bottom_val = if bottom_auto { zero } else {
        resolve_length(&style.bottom, cb_height, zero, zero)
    };

    // CSS 2.1 §8.3: Percentage margins resolve against CB WIDTH
    let margin_top_auto = style.margin_top.is_auto();
    let margin_bottom_auto = style.margin_bottom.is_auto();
    let margin_top_val = if margin_top_auto { zero } else {
        resolve_margin_or_padding(&style.margin_top, cb_width)
    };
    let margin_bottom_val = if margin_bottom_auto { zero } else {
        resolve_margin_or_padding(&style.margin_bottom, cb_width)
    };

    if !top_auto && !bottom_auto {
        // Both insets specified — auto margins absorb space
        if margin_top_auto && margin_bottom_auto {
            let remaining = cb_height - top_val - bottom_val - border_box_height;
            if remaining >= zero {
                let half = remaining / 2;
                return (top_val + half, border_box_height, half, remaining - half);
            } else {
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
        // Over-constrained: ignore bottom
        return (top_val + margin_top_val, border_box_height, margin_top_val, margin_bottom_val);
    }

    let mt = if margin_top_auto { zero } else { margin_top_val };
    let mb = if margin_bottom_auto { zero } else { margin_bottom_val };

    if top_auto && bottom_auto {
        let top = static_top;
        return (top + mt, border_box_height, mt, mb);
    }

    if top_auto {
        let top = cb_height - bottom_val - mb - border_box_height - mt;
        return (top + mt, border_box_height, mt, mb);
    }

    if bottom_auto {
        return (top_val + mt, border_box_height, mt, mb);
    }

    (static_top, border_box_height, zero, zero)
}
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
            &style, LayoutUnit::from_i32(800), LayoutUnit::zero(), &border, &padding, stf_min, stf_max, Direction::Ltr, Direction::Ltr,
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
            &style, LayoutUnit::from_i32(800), LayoutUnit::zero(), &border, &padding, stf_min, stf_max, Direction::Ltr, Direction::Ltr,
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
