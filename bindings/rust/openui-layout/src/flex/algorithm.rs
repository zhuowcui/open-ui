//! Flex layout algorithm — the main entry point.
//!
//! Extracted from Blink's `FlexLayoutAlgorithm::LayoutInternal()` and
//! `PlaceFlexItems()` (flex_layout_algorithm.cc:1229, 1394).
//!
//! Orchestrates: item collection → line breaking → flexing → alignment → positioning.

use openui_dom::{Document, NodeId};
use openui_geometry::{BoxStrut, LayoutUnit, LengthType, MinMaxSizes, PhysicalOffset, PhysicalSize};
use openui_style::{
    ContentAlignment, ContentDistribution, ContentPosition,
    ItemPosition,
};
use openui_geometry::Length;

use crate::block::{resolve_border, resolve_padding, resolve_margins};
use crate::constraint_space::ConstraintSpace;
use crate::fragment::Fragment;
use crate::intrinsic_sizing::compute_intrinsic_block_sizes;
use crate::length_resolver::resolve_length;

use super::alignment::{
    resolve_align_self, resolve_content_alignment, resolve_cross_auto_margins,
    resolve_main_auto_margins,
};
use super::item::{FlexItem, FlexerState};
use super::line::FlexLine;
use super::line_breaker::break_into_lines;
use super::line_flexer::LineFlexer;

/// Main entry point for flex layout.
///
/// Blink: `FlexLayoutAlgorithm::Layout()` → `LayoutInternal()` → `PlaceFlexItems()`.
///
/// Takes a flex container node and its constraint space, returns a positioned Fragment.
pub fn flex_layout(doc: &Document, node_id: NodeId, space: &ConstraintSpace) -> Fragment {
    let style = &doc.node(node_id).style;

    // ── Axis orientation (Blink constructor, line 170-191) ───────────
    let is_column = style.flex_direction.is_column();
    let is_reverse = style.flex_direction.is_reverse();
    let is_wrap_reverse = style.flex_wrap.is_wrap_reverse();
    let is_multi_line = style.flex_wrap.is_wrap();

    // For horizontal writing mode (our only mode for now):
    // Row: main=inline, cross=block. Column: main=block, cross=inline.
    let is_horizontal_flow = !is_column; // horizontal writing mode assumed

    // ── Resolve container border + padding ───────────────────────────
    let border = resolve_border(style);
    let padding = resolve_padding(style, space.percentage_resolution_inline_size);
    let border_padding_inline = border.inline_sum() + padding.inline_sum();
    let border_padding_block = border.block_sum() + padding.block_sum();

    // ── Resolve container inline size (width for row, used for percentage base) ──
    let container_inline_size = resolve_container_inline_size(
        doc, node_id, style, space, border_padding_inline,
    );

    // Content-box sizes
    let content_inline_size = container_inline_size - border_padding_inline;

    // ── Resolve gaps (Blink line 187-191) ────────────────────────────
    let percentage_base = content_inline_size;
    let gap_between_items = resolve_gap(
        if is_column { &style.row_gap } else { &style.column_gap },
        percentage_base,
    );
    let gap_between_lines = resolve_gap(
        if is_column { &style.column_gap } else { &style.row_gap },
        percentage_base,
    );

    // ── Main axis inner size ─────────────────────────────────────────
    let main_axis_inner_size = if is_column {
        // Column: main axis = block, may be indefinite
        let resolved = resolve_container_block_size_for_flex(style, space, border_padding_block);
        // If resolved to indefinite but parent provided a fixed height, use it for
        // wrapping decisions (CSS Flexbox §9.2: definite size from containing block)
        if resolved.is_indefinite() && (space.is_fixed_block_size || space.stretch_block_size) {
            (space.available_block_size - border_padding_block).clamp_negative_to_zero()
        } else {
            resolved
        }
    } else {
        // Row: main axis = inline
        content_inline_size
    };

    // Child percentage resolution sizes.
    // Percentage heights on flex items resolve against the flex container's
    // own content-box height (CSS §9.8). When the container has auto height,
    // its height is indefinite until layout completes, so percentages are
    // indefinite regardless of parent's available block size.
    let child_percentage_inline = content_inline_size;
    let child_percentage_block = if !style.height.is_auto() {
        if style.height.is_content_or_intrinsic() {
            // Intrinsic keyword on container height → treat as indefinite for child percentages
            LayoutUnit::from_raw(-64) // indefinite
        } else {
            // Container has explicit height → use it as percentage base
            let raw = resolve_length(&style.height, space.percentage_resolution_block_size, LayoutUnit::zero(), LayoutUnit::zero());
            let content = if style.box_sizing == openui_style::BoxSizing::BorderBox {
                (raw - border_padding_block).clamp_negative_to_zero()
            } else {
                raw
            };
            content
        }
    } else if space.is_fixed_block_size || space.stretch_block_size {
        // CSS Flexbox §9.8: Parent flex has set a definite block size for this
        // container (e.g. stretch or fixed). Treat it as the percentage base
        // so that percentage-height children resolve correctly.
        (space.available_block_size - border_padding_block).clamp_negative_to_zero()
    } else {
        // Container height is auto → percentages are indefinite
        LayoutUnit::from_raw(-64) // indefinite
    };

    // ── Step A: Collect items (Blink line 801) ───────────────────────
    let mut flex_items = construct_flex_items(
        doc,
        node_id,
        is_column,
        is_horizontal_flow,
        child_percentage_inline,
        child_percentage_block,
        main_axis_inner_size,
        space,
    );

    // ── Step B: Break into lines (Blink line_breaker.cc) ─────────────
    let mut flex_lines = break_into_lines(
        &flex_items,
        main_axis_inner_size,
        gap_between_items,
        is_multi_line,
    );

    // ── Step C: Flex each line (CSS §9.7) ────────────────────────────
    for line in &mut flex_lines {
        let sum_hyp: LayoutUnit = line.item_indices.iter()
            .map(|&idx| flex_items[idx].hypothetical_main_axis_margin_box_size())
            .fold(LayoutUnit::zero(), |acc, s| acc + s);

        // When main axis is indefinite (auto-height column), skip grow/shrink.
        // Items stay at their hypothetical sizes.
        if !main_axis_inner_size.is_indefinite() {
            let mut flexer = LineFlexer::new(
                &mut flex_items,
                &line.item_indices,
                main_axis_inner_size,
                sum_hyp,
                gap_between_items,
            );
            flexer.run();
        } else {
            // Freeze all items at their hypothetical sizes
            for &idx in &line.item_indices {
                flex_items[idx].flexed_content_size = flex_items[idx].hypothetical_content_size;
                flex_items[idx].state = super::item::FlexerState::Frozen;
            }
        }

        // Compute free space after flexing
        let total_flexed: LayoutUnit = line.item_indices.iter()
            .map(|&idx| flex_items[idx].flexed_margin_box_size())
            .fold(LayoutUnit::zero(), |acc, s| acc + s);

        let num_gaps = if line.item_count() > 1 { line.item_count() as i32 - 1 } else { 0 };
        let total_gap = gap_between_items * num_gaps;
        if !main_axis_inner_size.is_indefinite() {
            line.main_axis_free_space = main_axis_inner_size - total_flexed - total_gap;
        } else {
            line.main_axis_free_space = LayoutUnit::zero();
        }
        line.main_axis_used_size = total_flexed + total_gap;

        // Count auto margins on main axis
        line.main_axis_auto_margin_count = line.item_indices.iter()
            .map(|&idx| flex_items[idx].main_axis_auto_margin_count as u32)
            .sum();
    }

    // ── Step D: Compute line cross sizes (Blink line 1470) ───────────
    compute_line_cross_sizes(
        doc,
        &flex_items,
        &mut flex_lines,
        is_column,
        is_horizontal_flow,
        child_percentage_inline,
        child_percentage_block,
    );

    // For single-line: use container cross size if definite
    if !is_multi_line && flex_lines.len() == 1 {
        let container_cross = if is_column {
            content_inline_size
        } else {
            // Use the container's own resolved content-box height when explicit.
            // CSS Flexbox §9.4: Only use container cross size if it is definite.
            // height:auto means the cross size is NOT definite, even with definite available space.
            // Exception: if the parent flex has set a definite block size (via
            // is_fixed_block_size or stretch_block_size), the cross size IS definite.
            if space.is_fixed_block_size || space.stretch_block_size {
                (space.available_block_size - border_padding_block).clamp_negative_to_zero()
            } else if !style.height.is_auto() && !style.height.is_content_or_intrinsic() {
                // Explicit height — use resolved content-box value
                let raw = resolve_length(&style.height, space.percentage_resolution_block_size, LayoutUnit::zero(), LayoutUnit::zero());
                if style.box_sizing == openui_style::BoxSizing::BorderBox {
                    (raw - border_padding_block).clamp_negative_to_zero()
                } else {
                    raw
                }
            } else {
                flex_lines[0].line_cross_size // keep computed size
            }
        };

        // Clamp cross size to container's min/max constraints (CSS Flexbox §9.4)
        let clamped_cross = if !is_column {
            let pct_base = space.percentage_resolution_block_size;
            let min_h = if !style.min_height.is_auto() {
                if style.min_height.is_content_or_intrinsic() {
                    let sizes = compute_intrinsic_block_sizes(doc, node_id);
                    let intrinsic = match style.min_height.length_type() {
                        LengthType::MinContent => sizes.min_content_block_size,
                        _ => sizes.max_content_block_size,
                    };
                    (intrinsic - border_padding_block).clamp_negative_to_zero()
                } else if !pct_base.is_indefinite() || style.min_height.is_fixed() {
                    let min_raw = resolve_length(&style.min_height, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
                    if style.box_sizing == openui_style::BoxSizing::BorderBox {
                        (min_raw - border_padding_block).clamp_negative_to_zero()
                    } else {
                        min_raw
                    }
                } else {
                    LayoutUnit::zero()
                }
            } else {
                LayoutUnit::zero()
            };
            let max_h = if !style.max_height.is_none() {
                if style.max_height.is_content_or_intrinsic() {
                    let sizes = compute_intrinsic_block_sizes(doc, node_id);
                    let intrinsic = match style.max_height.length_type() {
                        LengthType::MinContent => sizes.min_content_block_size,
                        _ => sizes.max_content_block_size,
                    };
                    (intrinsic - border_padding_block).clamp_negative_to_zero()
                } else if !pct_base.is_indefinite() || style.max_height.is_fixed() {
                    let max_raw = resolve_length(&style.max_height, pct_base, LayoutUnit::zero(), LayoutUnit::from_i32(33554431));
                    if style.box_sizing == openui_style::BoxSizing::BorderBox {
                        (max_raw - border_padding_block).clamp_negative_to_zero()
                    } else {
                        max_raw
                    }
                } else {
                    LayoutUnit::from_i32(33554431)
                }
            } else {
                LayoutUnit::from_i32(33554431)
            };
            container_cross.clamp(min_h, max_h)
        } else {
            container_cross
        };

        flex_lines[0].line_cross_size = clamped_cross;
    }

    // ── Compute total block size ─────────────────────────────────────
    let intrinsic_block_size = compute_intrinsic_block_size(
        &flex_lines, is_column, gap_between_lines, border_padding_block,
    );

    let total_block_size = resolve_total_block_size(
        doc, node_id, style, space, intrinsic_block_size, border_padding_block,
    );

    // ── Fix: Recalculate main-axis free space for column flex ────────
    // When the main axis was initially indefinite (auto-height column),
    // items were frozen at hypothetical sizes with free_space=0. But
    // total_block_size may be larger than intrinsic_block_size (due to
    // min-height, stretch from parent, etc.). Recalculate so that
    // justify-content and column-reverse positioning use the real free space.
    // Blink: LayoutColumnReverse() recalculates offsets using the resolved size.
    if is_column && main_axis_inner_size.is_indefinite() {
        let resolved_main = (total_block_size - border_padding_block).clamp_negative_to_zero();
        for line in &mut flex_lines {
            let num_gaps = if line.item_count() > 1 { line.item_count() as i32 - 1 } else { 0 };
            let total_gap = gap_between_items * num_gaps;
            line.main_axis_free_space = resolved_main - line.main_axis_used_size - total_gap;
        }
    }

    // ── Step 5b: Stretch cross-axis lines BEFORE reversal ────────────
    // Blink performs align-content:stretch before reversing lines, so
    // remainder pixels go to the first lines in original order.
    let content_cross_size = if is_column {
        content_inline_size
    } else {
        total_block_size - border_padding_block
    };
    {
        let total_line_cross: LayoutUnit = flex_lines.iter()
            .map(|l| l.line_cross_size)
            .fold(LayoutUnit::zero(), |acc, s| acc + s);
        let num_line_gaps = if flex_lines.len() > 1 { flex_lines.len() as i32 - 1 } else { 0 };
        let total_line_gap = gap_between_lines * num_line_gaps;
        let cross_free_space = content_cross_size - total_line_cross - total_line_gap;

        let should_stretch_lines = style.align_content.distribution == ContentDistribution::Stretch
            || (style.align_content.distribution == ContentDistribution::Default
                && style.align_content.position == ContentPosition::Normal);
        if should_stretch_lines && cross_free_space > LayoutUnit::zero() && flex_lines.len() > 0 {
            let n = flex_lines.len() as i32;
            let extra_per_line = LayoutUnit::from_raw(cross_free_space.raw() / n);
            let remainder = cross_free_space.raw() % n;
            for (i, line) in flex_lines.iter_mut().enumerate() {
                let bonus = if (i as i32) < remainder { LayoutUnit::from_raw(1) } else { LayoutUnit::zero() };
                line.line_cross_size = line.line_cross_size + extra_per_line + bonus;
            }
        }
    }

    // ── Step 6: Apply reversals (Blink line 1265) ────────────────────
    if is_wrap_reverse {
        flex_lines.reverse();
    }
    if is_reverse {
        for line in &mut flex_lines {
            line.item_indices.reverse();
        }
    }

    let children = give_items_final_position(
        doc,
        &mut flex_items,
        &mut flex_lines,
        is_column,
        is_reverse,
        is_wrap_reverse,
        is_horizontal_flow,
        main_axis_inner_size,
        content_cross_size,
        gap_between_items,
        gap_between_lines,
        &style.justify_content,
        &style.align_content,
        &border,
        &padding,
        child_percentage_inline,
        child_percentage_block,
        space,
    );

    // ── Build fragment ───────────────────────────────────────────────
    // When the container inline size is indefinite (e.g. auto-width flex container
    // being measured for intrinsic sizing), shrink-wrap to the actual content.
    // CSS Flexbox §9.2: auto main size → fit-content.
    let final_inline_size = if container_inline_size.is_indefinite() || container_inline_size < LayoutUnit::zero() {
        // Compute from actual item sizes: max of all lines' used sizes
        let max_line_main = flex_lines.iter()
            .map(|line| line.main_axis_used_size)
            .fold(LayoutUnit::zero(), |acc, s| acc.max_of(s));
        let result = if is_column {
            let max_child_width = children.iter()
                .map(|c| c.offset.left + c.width())
                .fold(LayoutUnit::zero(), |acc, w| acc.max_of(w));
            max_child_width + border.right + padding.right
        } else {
            max_line_main + border_padding_inline
        };
        result
    } else {
        container_inline_size
    };
    let mut fragment = Fragment::new_box(
        node_id,
        PhysicalSize::new(final_inline_size, total_block_size),
    );
    fragment.padding = padding.clone();
    fragment.border = border.clone();
    fragment.children = children;

    // ── Lay out out-of-flow (absolute/fixed) children ────────────────
    // Flex containers establish a containing block for abspos descendants.
    let content_width = (final_inline_size - border_padding_inline).clamp_negative_to_zero();
    let content_height = (total_block_size - border_padding_block).clamp_negative_to_zero();
    let cb_size = PhysicalSize::new(content_width, content_height);

    let mut oof_candidates = Vec::new();
    for child_id in doc.children(node_id) {
        let child_style = &doc.node(child_id).style;
        if child_style.position.is_absolutely_positioned() {
            // CSS Flexbox §4.1: The static position of an abspos child of a
            // flex container is determined as if the child were the sole flex
            // item in the container, using the container's alignment properties.
            let (sp_x, sp_y) = compute_abspos_static_position(
                doc, child_id, child_style, style,
                content_width, content_height, is_column,
                &border, &padding,
            );
            oof_candidates.push(crate::out_of_flow::OutOfFlowCandidate {
                node_id: child_id,
                style: child_style.clone(),
                static_position: PhysicalOffset::new(sp_x, sp_y),
                containing_block_size: cb_size,
                containing_block_border: border.clone(),
                containing_block_direction: style.direction,
                static_position_direction: style.direction,
            });
        }
    }
    if !oof_candidates.is_empty() {
        let oof_fragments = crate::out_of_flow::layout_out_of_flow_children(doc, &oof_candidates);
        fragment.children.extend(oof_fragments);
    }

    fragment
}

/// Resolve the container's inline size (width for horizontal writing mode).
fn resolve_container_inline_size(
    doc: &Document,
    node_id: NodeId,
    style: &openui_style::ComputedStyle,
    space: &ConstraintSpace,
    border_padding_inline: LayoutUnit,
) -> LayoutUnit {
    let resolved = if style.width.is_auto() {
        if space.available_inline_size.is_indefinite() {
            // Shrink-to-fit: compute max-content inline size from children
            let sizes = compute_intrinsic_block_sizes(doc, node_id);
            sizes.max_content_inline_size.max_of(border_padding_inline)
        } else {
            space.available_inline_size
        }
    } else if style.width.is_content_or_intrinsic() {
        // Resolve intrinsic sizing keywords for flex container width
        let sizes = compute_intrinsic_block_sizes(doc, node_id);
        let bp = border_padding_inline;
        match style.width.length_type() {
            LengthType::MinContent => sizes.min_content_inline_size.max_of(bp),
            LengthType::MaxContent => sizes.max_content_inline_size.max_of(bp),
            LengthType::FitContent => {
                let avail = space.available_inline_size;
                let min = sizes.min_content_inline_size;
                let max = sizes.max_content_inline_size;
                avail.clamp(min, max).max_of(bp)
            }
            _ => space.available_inline_size,
        }
    } else {
        let raw = resolve_length(&style.width, space.percentage_resolution_inline_size, LayoutUnit::zero(), LayoutUnit::zero());
        if style.box_sizing == openui_style::BoxSizing::BorderBox {
            raw
        } else {
            raw + border_padding_inline
        }
    };

    // Clamp to min/max
    clamp_inline_size(doc, node_id, style, space, resolved, border_padding_inline)
}

/// Clamp inline size to min-width/max-width.
fn clamp_inline_size(
    doc: &Document,
    node_id: NodeId,
    style: &openui_style::ComputedStyle,
    space: &ConstraintSpace,
    size: LayoutUnit,
    border_padding_inline: LayoutUnit,
) -> LayoutUnit {
    let pct_base = space.percentage_resolution_inline_size;

    let min = if !style.min_width.is_auto() {
        if style.min_width.is_content_or_intrinsic() {
            let sizes = compute_intrinsic_block_sizes(doc, node_id);
            match style.min_width.length_type() {
                LengthType::MinContent => sizes.min_content_inline_size,
                _ => sizes.max_content_inline_size,
            }
        } else {
            let min_raw = resolve_length(&style.min_width, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
            if style.box_sizing == openui_style::BoxSizing::BorderBox {
                min_raw
            } else {
                min_raw + border_padding_inline
            }
        }
    } else {
        LayoutUnit::zero()
    };

    let max = if !style.max_width.is_none() {
        if style.max_width.is_content_or_intrinsic() {
            let sizes = compute_intrinsic_block_sizes(doc, node_id);
            match style.max_width.length_type() {
                LengthType::MinContent => sizes.min_content_inline_size,
                LengthType::MaxContent | LengthType::FitContent => sizes.max_content_inline_size,
                _ => sizes.max_content_inline_size,
            }
        } else {
            let max_raw = resolve_length(&style.max_width, pct_base, LayoutUnit::zero(), LayoutUnit::from_i32(33554431));
            if style.box_sizing == openui_style::BoxSizing::BorderBox {
                max_raw
            } else {
                max_raw + border_padding_inline
            }
        }
    } else {
        LayoutUnit::from_i32(33554431) // nearly max
    };

    size.clamp(min, max)
}

/// Resolve the container's block size for flex layout.
/// For column flex, this is the main-axis size. For row flex, just the cross size.
fn resolve_container_block_size_for_flex(
    style: &openui_style::ComputedStyle,
    space: &ConstraintSpace,
    border_padding_block: LayoutUnit,
) -> LayoutUnit {
    // When the parent flex has set a definite block size for this container,
    // use it as the main axis size (e.g. nested column flex with stretch).
    if space.is_fixed_block_size {
        return (space.available_block_size - border_padding_block).clamp_negative_to_zero();
    }
    if space.stretch_block_size {
        return (space.available_block_size - border_padding_block).clamp_negative_to_zero();
    }

    if style.height.is_auto() {
        // Auto height → indefinite main axis for column flex.
        // Items stay at hypothetical sizes; container shrink-wraps.
        // CSS Flexbox §9.2: auto height means intrinsic sizing.
        LayoutUnit::from_raw(-64) // INDEFINITE_SIZE sentinel
    } else {
        let raw = resolve_length(&style.height, space.percentage_resolution_block_size, LayoutUnit::zero(), LayoutUnit::zero());
        let content = if style.box_sizing == openui_style::BoxSizing::BorderBox {
            raw - border_padding_block
        } else {
            raw
        };
        content.clamp_negative_to_zero()
    }
}

/// Resolve the total block size of the container.
fn resolve_total_block_size(
    doc: &Document,
    node_id: NodeId,
    style: &openui_style::ComputedStyle,
    space: &ConstraintSpace,
    intrinsic_block_size: LayoutUnit,
    border_padding_block: LayoutUnit,
) -> LayoutUnit {
    let resolved = if space.is_fixed_block_size {
        // Parent (e.g., column flex) has set a definite block size for this child
        space.available_block_size
    } else if space.stretch_block_size {
        // Parent flex is stretching this container on the cross axis
        space.available_block_size
    } else if style.height.is_auto() {
        intrinsic_block_size
    } else if style.height.is_content_or_intrinsic() {
        let sizes = compute_intrinsic_block_sizes(doc, node_id);
        match style.height.length_type() {
            LengthType::MinContent => sizes.min_content_block_size.max_of(border_padding_block),
            LengthType::MaxContent => sizes.max_content_block_size.max_of(border_padding_block),
            _ => intrinsic_block_size,
        }
    } else {
        let raw = resolve_length(&style.height, space.percentage_resolution_block_size, LayoutUnit::zero(), LayoutUnit::zero());
        if style.box_sizing == openui_style::BoxSizing::BorderBox {
            raw
        } else {
            raw + border_padding_block
        }
    };

    // Clamp min/max
    let pct_base = space.percentage_resolution_block_size;

    let min = if !style.min_height.is_auto() && (!pct_base.is_indefinite() || style.min_height.is_fixed() || style.min_height.is_content_or_intrinsic()) {
        if style.min_height.is_content_or_intrinsic() {
            let sizes = compute_intrinsic_block_sizes(doc, node_id);
            match style.min_height.length_type() {
                LengthType::MinContent => sizes.min_content_block_size.max_of(border_padding_block),
                _ => sizes.max_content_block_size.max_of(border_padding_block),
            }
        } else {
            let min_raw = resolve_length(&style.min_height, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
            if style.box_sizing == openui_style::BoxSizing::BorderBox {
                min_raw
            } else {
                min_raw + border_padding_block
            }
        }
    } else {
        LayoutUnit::zero()
    };

    let max = if !style.max_height.is_none() && (!pct_base.is_indefinite() || style.max_height.is_fixed() || style.max_height.is_content_or_intrinsic()) {
        if style.max_height.is_content_or_intrinsic() {
            let sizes = compute_intrinsic_block_sizes(doc, node_id);
            match style.max_height.length_type() {
                LengthType::MinContent => sizes.min_content_block_size.max_of(border_padding_block),
                LengthType::MaxContent | LengthType::FitContent => sizes.max_content_block_size.max_of(border_padding_block),
                _ => sizes.max_content_block_size.max_of(border_padding_block),
            }
        } else {
            let max_raw = resolve_length(&style.max_height, pct_base, LayoutUnit::zero(), LayoutUnit::from_i32(33554431));
            if style.box_sizing == openui_style::BoxSizing::BorderBox {
                max_raw
            } else {
                max_raw + border_padding_block
            }
        }
    } else {
        LayoutUnit::from_i32(33554431)
    };

    resolved.clamp(min, max)
}

/// Resolve a gap value (row-gap or column-gap).
fn resolve_gap(gap: &Option<Length>, percentage_base: LayoutUnit) -> LayoutUnit {
    match gap {
        Some(length) => resolve_length(length, percentage_base, LayoutUnit::zero(), LayoutUnit::zero()),
        None => LayoutUnit::zero(), // normal = 0px for flex
    }
}

/// Construct flex items from in-flow children.
/// Blink: `ConstructAndAppendFlexItems()` at line 801.
fn construct_flex_items(
    doc: &Document,
    container_id: NodeId,
    is_column: bool,
    is_horizontal_flow: bool,
    child_percentage_inline: LayoutUnit,
    child_percentage_block: LayoutUnit,
    main_axis_inner_size: LayoutUnit,
    space: &ConstraintSpace,
) -> Vec<FlexItem> {
    let container_style = &doc.node(container_id).style;

    // Collect children with their order values, then stable sort
    let mut children_with_order: Vec<(NodeId, i32)> = Vec::new();
    for child_id in doc.children(container_id) {
        let child_style = &doc.node(child_id).style;
        // Skip out-of-flow and display:none
        if child_style.is_out_of_flow() || child_style.display == openui_style::Display::None {
            continue;
        }
        children_with_order.push((child_id, child_style.order));
    }

    // Stable sort by order (Blink: FlexChildIterator)
    children_with_order.sort_by_key(|&(_, order)| order);

    let mut items = Vec::with_capacity(children_with_order.len());

    for (item_index, &(child_id, _)) in children_with_order.iter().enumerate() {
        let child_style = &doc.node(child_id).style;

        // Read flex properties — CSS spec requires non-negative values.
        let flex_grow = child_style.flex_grow.max(0.0);
        let flex_shrink = child_style.flex_shrink.max(0.0);

        // Resolve alignment (Blink: ResolvedAlignSelf, line 261)
        let alignment = resolve_item_alignment(child_style, container_style);

        // Compute margins
        let margin_pct_base = child_percentage_inline;
        let margin = resolve_margins(child_style, margin_pct_base);

        // Compute border + padding
        let child_border = resolve_border(child_style);
        let child_padding = resolve_padding(child_style, child_percentage_inline);

        let main_axis_border_padding = if is_column {
            child_border.block_sum() + child_padding.block_sum()
        } else {
            child_border.inline_sum() + child_padding.inline_sum()
        };

        // Count auto margins on main axis
        let main_axis_auto_margin_count = if is_column {
            (if child_style.margin_top.is_auto() { 1u8 } else { 0 })
                + (if child_style.margin_bottom.is_auto() { 1 } else { 0 })
        } else {
            (if child_style.margin_left.is_auto() { 1u8 } else { 0 })
                + (if child_style.margin_right.is_auto() { 1 } else { 0 })
        };

        // ── Resolve flex-basis (Blink lines 942-1024) ────────────────
        let (base_content_size, is_used_flex_basis_indefinite) = resolve_flex_basis(
            doc,
            child_id,
            child_style,
            is_column,
            main_axis_border_padding,
            child_percentage_inline,
            child_percentage_block,
            main_axis_inner_size,
            space,
            alignment,
        );

        // ── Resolve min/max on main axis (Blink lines 1145-1157) ─────
        let main_axis_min_max = resolve_main_axis_min_max(
            doc,
            child_id,
            child_style,
            is_column,
            main_axis_border_padding,
            child_percentage_inline,
            child_percentage_block,
            base_content_size,
            is_used_flex_basis_indefinite,
        );

        // Hypothetical = clamp base to min/max
        let hypothetical_content_size = main_axis_min_max.clamp(base_content_size);

        items.push(FlexItem {
            node_id: child_id,
            item_index,
            flex_grow,
            flex_shrink,
            base_content_size,
            hypothetical_content_size,
            main_axis_min_max,
            main_axis_border_padding,
            margin,
            main_axis_auto_margin_count,
            alignment,
            flexed_content_size: LayoutUnit::zero(),
            state: FlexerState::None,
            free_space_fraction: 0.0,
            is_used_flex_basis_indefinite,
            is_horizontal_flow,
        });
    }

    items
}

/// Resolve the effective alignment for a flex item.
/// Blink: `ResolvedAlignSelf()` at flex_layout_algorithm.cc:261.
fn resolve_item_alignment(
    child_style: &openui_style::ComputedStyle,
    parent_style: &openui_style::ComputedStyle,
) -> ItemPosition {
    let mut position = child_style.align_self.position;

    // auto → inherit from parent's align-items
    if position == ItemPosition::Auto {
        position = parent_style.align_items.position;
    }

    // normal → stretch in flex context
    if position == ItemPosition::Normal {
        position = ItemPosition::Stretch;
    }

    // Coerce start/end variants to flex-start/flex-end
    match position {
        ItemPosition::Start | ItemPosition::SelfStart => ItemPosition::FlexStart,
        ItemPosition::End | ItemPosition::SelfEnd => ItemPosition::FlexEnd,
        other => other,
    }
}

/// Resolve flex-basis for a flex item.
/// Blink: lines 942-1024 of flex_layout_algorithm.cc.
///
/// Returns (base_content_size, is_used_flex_basis_indefinite).
fn resolve_flex_basis(
    doc: &Document,
    child_id: NodeId,
    child_style: &openui_style::ComputedStyle,
    is_column: bool,
    main_axis_border_padding: LayoutUnit,
    child_percentage_inline: LayoutUnit,
    child_percentage_block: LayoutUnit,
    main_axis_inner_size: LayoutUnit,
    space: &ConstraintSpace,
    resolved_alignment: ItemPosition,
) -> (LayoutUnit, bool) {
    let flex_basis = &child_style.flex_basis;

    // Step 1: If flex-basis is not auto, try to resolve it
    if !flex_basis.is_auto() {
        // Handle intrinsic sizing keywords (min-content, max-content, fit-content)
        if flex_basis.is_content_or_intrinsic() {
            let sizes = compute_intrinsic_block_sizes(doc, child_id);
            let intrinsic = if is_column {
                match flex_basis.length_type() {
                    LengthType::MinContent => sizes.min_content_block_size,
                    _ => sizes.max_content_block_size,
                }
            } else {
                match flex_basis.length_type() {
                    LengthType::MinContent => sizes.min_content_inline_size,
                    _ => sizes.max_content_inline_size,
                }
            };
            let content = (intrinsic - main_axis_border_padding).clamp_negative_to_zero();
            return (content, false);
        }

        // CSS Flexbox §9.2: percentage flex-basis resolves against the flex
        // container's main size. For column flex, use main_axis_inner_size
        // (which accounts for parent-imposed sizes), not child_percentage_block.
        let pct_base = if is_column {
            main_axis_inner_size
        } else {
            child_percentage_inline
        };

        // CSS Flexbox §9.2 step E: a percentage flex-basis with an indefinite
        // containing block falls back to content-based sizing.
        // Exception: flex-basis: 0% resolves to 0 even with indefinite containers,
        // matching Chromium/Blink behavior for pixel parity (0% of anything is 0).
        if !pct_base.is_indefinite() || flex_basis.is_fixed()
            || (flex_basis.is_percent() && flex_basis.value() == 0.0)
        {
            let resolved = resolve_length(flex_basis, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
            let content = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                (resolved - main_axis_border_padding).clamp_negative_to_zero()
            } else {
                // CSS Flexbox §4.2: negative flex-basis clamps to 0
                resolved.clamp_negative_to_zero()
            };
            return (content, false);
        }

        // Percentage with indefinite base → content-based
        return (resolve_content_based_size(
            doc, child_id, child_style, is_column,
            main_axis_border_padding,
            child_percentage_inline, child_percentage_block, space,
            resolved_alignment,
        ), true);
    }

    // Step 2: flex-basis: auto → use width/height in main axis direction
    let main_length = if is_column {
        &child_style.height
    } else {
        &child_style.width
    };

    if !main_length.is_auto() {
        // Handle intrinsic sizing keywords on width/height
        if main_length.is_content_or_intrinsic() {
            let sizes = compute_intrinsic_block_sizes(doc, child_id);
            let intrinsic = if is_column {
                match main_length.length_type() {
                    LengthType::MinContent => sizes.min_content_block_size,
                    _ => sizes.max_content_block_size,
                }
            } else {
                match main_length.length_type() {
                    LengthType::MinContent => sizes.min_content_inline_size,
                    _ => sizes.max_content_inline_size,
                }
            };
            let content = (intrinsic - main_axis_border_padding).clamp_negative_to_zero();
            return (content, false);
        }

        let pct_base = if is_column {
            child_percentage_block
        } else {
            child_percentage_inline
        };

        if !pct_base.is_indefinite() || main_length.is_fixed() {
            let resolved = resolve_length(main_length, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
            let content = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                (resolved - main_axis_border_padding).clamp_negative_to_zero()
            } else {
                resolved
            };
            return (content, false);
        }
    }

    // Step 3: Content-based sizing (max-content)
    (resolve_content_based_size(
        doc, child_id, child_style, is_column,
        main_axis_border_padding,
        child_percentage_inline, child_percentage_block, space,
        resolved_alignment,
    ), true)
}

/// Resolve content-based (intrinsic) size for a flex item.
/// This runs a child layout to determine the item's natural size.
/// Also handles aspect-ratio: if the item has an aspect ratio and one dimension
/// is known, derive the main-axis size from the cross-axis size.
fn resolve_content_based_size(
    doc: &Document,
    child_id: NodeId,
    child_style: &openui_style::ComputedStyle,
    is_column: bool,
    main_axis_border_padding: LayoutUnit,
    child_percentage_inline: LayoutUnit,
    child_percentage_block: LayoutUnit,
    space: &ConstraintSpace,
    resolved_alignment: ItemPosition,
) -> LayoutUnit {
    // Check if aspect-ratio can resolve the main-axis size from a known cross-axis size
    if let Some(ref ar) = child_style.aspect_ratio {
        let ratio = ar.ratio;
        if ratio.0 > 0.0 && ratio.1 > 0.0 {
            let (cross_prop, cross_pct) = if is_column {
                (&child_style.width, child_percentage_inline)
            } else {
                (&child_style.height, child_percentage_block)
            };

            if !cross_prop.is_auto() && (!cross_pct.is_indefinite() || cross_prop.is_fixed()) {
                let cross_val = resolve_length(cross_prop, cross_pct, LayoutUnit::zero(), LayoutUnit::zero());
                // For border-box, cross_val is the border-box size. We need
                // content size to apply the ratio, then return content main size.
                let content_cross = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                    let b = resolve_border(child_style);
                    let p = resolve_padding(child_style, LayoutUnit::zero());
                    let cross_bp = if is_column {
                        b.left + b.right + p.left + p.right
                    } else {
                        b.top + b.bottom + p.top + p.bottom
                    };
                    (cross_val - cross_bp).clamp_negative_to_zero()
                } else {
                    cross_val
                };
                // Derive main-axis content size from cross-axis content size
                let main_val = if is_column {
                    // Column: main=block, cross=inline. main = cross * (h/w)
                    LayoutUnit::from_f32(content_cross.to_f32() * ratio.1 / ratio.0)
                } else {
                    // Row: main=inline, cross=block. main = cross * (w/h)
                    LayoutUnit::from_f32(content_cross.to_f32() * ratio.0 / ratio.1)
                };
                return main_val;
            }

            // CSS Flexbox §9.2 step 3(B): If cross-size is auto but the item
            // will stretch (align-self: stretch + definite container cross),
            // the cross-size is definite and equals the container cross minus margins.
            // NOTE: Must resolve align-self against parent's align-items, since
            // auto/normal inherit from the parent.
            if cross_prop.is_auto() {
                let cross_container = if is_column {
                    space.available_inline_size
                } else {
                    space.available_block_size
                };
                if !cross_container.is_indefinite() {
                    // Use the resolved alignment (which already accounts for
                    // parent's align-items when align-self is auto/normal).
                    let would_stretch = resolved_alignment == ItemPosition::Stretch;
                    if would_stretch {
                        let margin = resolve_margins(child_style, LayoutUnit::zero());
                        let cross_margin = if is_column {
                            margin.left + margin.right
                        } else {
                            margin.top + margin.bottom
                        };
                        let cross_bp = {
                            let b = resolve_border(child_style);
                            let p = resolve_padding(child_style, LayoutUnit::zero());
                            if is_column {
                                b.left + b.right + p.left + p.right
                            } else {
                                b.top + b.bottom + p.top + p.bottom
                            }
                        };
                        let stretched_bb = (cross_container - cross_margin).clamp_negative_to_zero();
                        let content_cross = (stretched_bb - cross_bp).clamp_negative_to_zero();
                        let main_val = if is_column {
                            LayoutUnit::from_f32(content_cross.to_f32() * ratio.1 / ratio.0)
                        } else {
                            LayoutUnit::from_f32(content_cross.to_f32() * ratio.0 / ratio.1)
                        };
                        return main_val;
                    }
                }
            }
        }
    }

    // For content-based sizing, lay out the child with unconstrained main axis.
    // CSS Flexbox §9.2: When computing the flex base size from content, the cross-
    // axis available size depends on alignment and the item's own constraints.
    // If the item will stretch, use the container's cross-axis size.
    // If not stretching, use the item's own cross-axis constraints (max-width
    // for column, max-height for row) to bound the layout, since the item
    // will shrink-wrap to its content.
    let would_stretch_cross = resolved_alignment == ItemPosition::Stretch;
    let child_space = if is_column {
        let cross_inline = if would_stretch_cross {
            child_percentage_inline
        } else if child_style.aspect_ratio.is_some() && child_style.width.is_auto() {
            // AR items with auto cross-size: use indefinite so AR derives
            // main from content, not from container width.
            LayoutUnit::from_raw(-64)
        } else if !child_style.max_width.is_none() && !child_style.max_width.is_auto() {
            // Item has explicit max-width: use it as constraint
            let max_w = resolve_length(
                &child_style.max_width, child_percentage_inline,
                LayoutUnit::zero(), LayoutUnit::zero(),
            );
            max_w
        } else {
            child_percentage_inline
        };
        ConstraintSpace::for_block_child(
            cross_inline,
            LayoutUnit::from_raw(-64), // indefinite block (main axis)
            child_percentage_inline,
            child_percentage_block,
            child_style.creates_new_formatting_context(),
        )
    } else {
        let cross_block = if would_stretch_cross {
            space.available_block_size
        } else if child_style.aspect_ratio.is_some() && child_style.height.is_auto() {
            LayoutUnit::from_raw(-64)
        } else {
            space.available_block_size
        };
        // Row flex: use indefinite inline size for max-content measurement
        ConstraintSpace::for_block_child(
            LayoutUnit::from_raw(-64), // indefinite → child gets intrinsic width
            cross_block,
            child_percentage_inline,
            child_percentage_block,
            child_style.creates_new_formatting_context(),
        )
    };

    // CSS Flexbox §9.2 step E: "size the item into the available space using
    // its used flex basis in place of its main size, treating a value of content
    // as max-content." For column flex, the main axis is block — block_layout
    // would use the item's own `height` property (even with indefinite available
    // space, a fixed height still resolves). Instead, compute the intrinsic
    // max-content block size, which ignores the item's height property and
    // measures purely from content.
    if is_column {
        // For aspect-ratio items, try deriving main from cross first
        if let Some(ref ar) = child_style.aspect_ratio {
            let ratio = ar.ratio;
            if ratio.0 > 0.0 && ratio.1 > 0.0 {
                let child_fragment = crate::block::block_layout(doc, child_id, &child_space);
                let cross_size = child_fragment.width();
                if !cross_size.is_indefinite() && cross_size > LayoutUnit::zero() {
                    let derived_main = LayoutUnit::from_f32(
                        cross_size.to_f32() * ratio.1 / ratio.0
                    );
                    return (derived_main - main_axis_border_padding).clamp_negative_to_zero();
                }
            }
        }

        let intrinsic = compute_intrinsic_block_sizes(doc, child_id);
        return (intrinsic.max_content_block_size - main_axis_border_padding).clamp_negative_to_zero();
    }

    let child_fragment = crate::block::block_layout(doc, child_id, &child_space);

    let main_size = child_fragment.width();

    // If block_layout returned indefinite (empty element with unconstrained axis),
    // treat as zero content size.
    let main_size = main_size.clamp_indefinite_to_zero();

    // Apply aspect-ratio for row flex: derive main (inline) from cross (block)
    if let Some(ref ar) = child_style.aspect_ratio {
        let ratio = ar.ratio;
        if ratio.0 > 0.0 && ratio.1 > 0.0 {
            let cross_size = child_fragment.height();
            if !cross_size.is_indefinite() && cross_size > LayoutUnit::zero() {
                let derived_main = LayoutUnit::from_f32(
                    cross_size.to_f32() * ratio.0 / ratio.1
                );
                if derived_main > main_size {
                    return (derived_main - main_axis_border_padding).clamp_negative_to_zero();
                }
            }
        }
    }

    // Convert from border-box to content-box
    (main_size - main_axis_border_padding).clamp_negative_to_zero()
}

/// Resolve min/max constraints on the main axis.
/// Blink: lines 1034-1157.
fn resolve_main_axis_min_max(
    doc: &Document,
    child_id: NodeId,
    child_style: &openui_style::ComputedStyle,
    is_column: bool,
    main_axis_border_padding: LayoutUnit,
    pct_inline: LayoutUnit,
    pct_block: LayoutUnit,
    _base_content_size: LayoutUnit,
    _is_basis_from_content: bool,
) -> MinMaxSizes {
    let (min_prop, max_prop, pct_base) = if is_column {
        (&child_style.min_height, &child_style.max_height, pct_block)
    } else {
        (&child_style.min_width, &child_style.max_width, pct_inline)
    };

    // ── Resolve min ──────────────────────────────────────────────────
    let min = if min_prop.is_auto() {
        // CSS Flexbox §4.5: Automatic Minimum Size
        // If overflow is not visible, auto min = 0.
        // Otherwise, auto min = min(content_size, specified_size).
        let overflow_visible = if is_column {
            child_style.overflow_y == openui_style::Overflow::Visible
        } else {
            child_style.overflow_x == openui_style::Overflow::Visible
        };

        if !overflow_visible {
            LayoutUnit::zero()
        } else {
            // CSS Flexbox §4.5: Content size suggestion is the min-content
            // size in the main axis. Always compute min-content intrinsic
            // size — base_content_size is max-content when flex-basis was
            // content-based, which is NOT the right value here.
            let content_size = if is_column {
                let intrinsic = crate::intrinsic_sizing::compute_intrinsic_block_sizes(doc, child_id);
                (intrinsic.min_content_block_size - main_axis_border_padding).clamp_negative_to_zero()
            } else {
                let min_max = crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, child_id);
                (min_max.min - main_axis_border_padding).clamp_negative_to_zero()
            };

            // Specified size suggestion (CSS Flexbox §4.5):
            // Uses the main-size property (width/height), NOT flex-basis.
            let main_size_prop = if is_column {
                &child_style.height
            } else {
                &child_style.width
            };
            let has_specified_main = !main_size_prop.is_auto()
                && (!pct_base.is_indefinite() || main_size_prop.is_fixed());

            if has_specified_main {
                let resolved = resolve_length(
                    main_size_prop, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
                let specified = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                    (resolved - main_axis_border_padding).clamp_negative_to_zero()
                } else {
                    resolved
                };
                content_size.min_of(specified)
            } else {
                content_size
            }
        }
    } else if min_prop.is_none() || *min_prop == Length::zero() {
        LayoutUnit::zero()
    } else if min_prop.is_content_or_intrinsic() {
        // CSS Sizing 3: min-width/min-height: min-content/max-content
        // Resolve via intrinsic sizing in the main axis.
        if is_column {
            let intrinsic = crate::intrinsic_sizing::compute_intrinsic_block_sizes(doc, child_id);
            let val = match min_prop.length_type() {
                LengthType::MinContent => intrinsic.min_content_block_size,
                _ => intrinsic.max_content_block_size,
            };
            (val - main_axis_border_padding).clamp_negative_to_zero()
        } else {
            let min_max = crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, child_id);
            let val = match min_prop.length_type() {
                LengthType::MinContent => min_max.min,
                _ => min_max.max,
            };
            (val - main_axis_border_padding).clamp_negative_to_zero()
        }
    } else {
        let resolved = resolve_length(min_prop, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
        if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
            (resolved - main_axis_border_padding).clamp_negative_to_zero()
        } else {
            resolved
        }
    };

    // ── Resolve max ──────────────────────────────────────────────────
    let max = if max_prop.is_none() {
        LayoutUnit::from_i32(33554431) // ~LayoutUnit::Max()
    } else if max_prop.is_content_or_intrinsic() {
        // CSS Sizing 3: max-width/max-height: min-content/max-content
        if is_column {
            let intrinsic = crate::intrinsic_sizing::compute_intrinsic_block_sizes(doc, child_id);
            let val = match max_prop.length_type() {
                LengthType::MinContent => intrinsic.min_content_block_size,
                _ => intrinsic.max_content_block_size,
            };
            (val - main_axis_border_padding).clamp_negative_to_zero()
        } else {
            let min_max = crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, child_id);
            let val = match max_prop.length_type() {
                LengthType::MinContent => min_max.min,
                _ => min_max.max,
            };
            (val - main_axis_border_padding).clamp_negative_to_zero()
        }
    } else if !pct_base.is_indefinite() || max_prop.is_fixed() {
        let resolved = resolve_length(max_prop, pct_base, LayoutUnit::zero(), LayoutUnit::from_i32(33554431));
        if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
            (resolved - main_axis_border_padding).clamp_negative_to_zero()
        } else {
            resolved
        }
    } else {
        LayoutUnit::from_i32(33554431)
    };

    // NOTE: CSS Sizing 4 §5.2 transferred min/max constraints are NOT
    // applied for flex items. In flex layout, the main axis size is determined
    // by flex-basis/grow/shrink, and the cross axis size is derived separately
    // (via AR if present) then clamped by cross min/max. Applying transferred
    // constraints here would incorrectly constrain the main axis — e.g. a flex
    // item with AR 1/2 + max-height:100px + flex:1 in a 100px container should
    // have width=100 (flex), height=min(200, 100)=100, not width=50.

    MinMaxSizes::new(min, max)
}

/// Resolve a cross-axis min or max constraint, handling intrinsic keywords.
fn resolve_cross_min_max(
    doc: &Document,
    node_id: NodeId,
    prop: &openui_geometry::Length,
    is_column: bool,
    pct_base: LayoutUnit,
    is_min: bool,
) -> LayoutUnit {
    if is_min && prop.is_auto() {
        return LayoutUnit::zero();
    }
    if !is_min && prop.is_none() {
        return LayoutUnit::from_i32(33554431);
    }
    if prop.is_content_or_intrinsic() {
        let sizes = if is_column {
            crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, node_id)
        } else {
            let block = crate::intrinsic_sizing::compute_intrinsic_block_sizes(doc, node_id);
            MinMaxSizes::new(block.min_content_block_size, block.max_content_block_size)
        };
        return match prop.length_type() {
            LengthType::MinContent => sizes.min,
            _ => sizes.max,
        };
    }
    let (auto_val, none_val) = if is_min {
        (LayoutUnit::zero(), LayoutUnit::zero())
    } else {
        (LayoutUnit::from_i32(33554431), LayoutUnit::from_i32(33554431))
    };
    resolve_length(prop, pct_base, auto_val, none_val)
}

/// Compute cross-axis sizes for each line.
/// Blink: PlaceFlexItems cross-size computation (line 1470-1558).
fn compute_line_cross_sizes(
    doc: &Document,
    items: &[FlexItem],
    lines: &mut [FlexLine],
    is_column: bool,
    _is_horizontal_flow: bool,
    child_percentage_inline: LayoutUnit,
    child_percentage_block: LayoutUnit,
) {
    for line in lines.iter_mut() {
        let mut max_cross_size = LayoutUnit::zero();

        for &idx in &line.item_indices {
            let item = &items[idx];
            let child_style = &doc.node(item.node_id).style;

            // Compute cross-axis size by laying out the child
            let child_border = resolve_border(child_style);
            let child_padding = resolve_padding(child_style, child_percentage_inline);

            let cross_border_padding = if is_column {
                child_border.inline_sum() + child_padding.inline_sum()
            } else {
                child_border.block_sum() + child_padding.block_sum()
            };

            // Resolve cross-axis size
            let cross_content_size = resolve_cross_size(
                doc, item, child_style, is_column,
                cross_border_padding,
                child_percentage_inline,
                child_percentage_block,
            );

            // Apply cross-axis min/max constraints
            let cross_bb = cross_content_size + cross_border_padding;
            let (cross_min_prop, cross_max_prop) = if is_column {
                (&child_style.min_width, &child_style.max_width)
            } else {
                (&child_style.min_height, &child_style.max_height)
            };
            let cross_pct_base = if is_column { child_percentage_inline } else { child_percentage_block };
            let cross_min_raw = resolve_cross_min_max(doc, item.node_id, cross_min_prop, is_column, cross_pct_base, true);
            let cross_max_raw = resolve_cross_min_max(doc, item.node_id, cross_max_prop, is_column, cross_pct_base, false);
            let cross_min_bb = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                cross_min_raw
            } else if cross_min_raw > LayoutUnit::zero() {
                cross_min_raw + cross_border_padding
            } else {
                cross_min_raw
            };
            let cross_max_bb = if cross_max_raw == LayoutUnit::from_i32(33554431) {
                cross_max_raw
            } else if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                cross_max_raw
            } else {
                cross_max_raw + cross_border_padding
            };
            let clamped_cross_bb = cross_bb.clamp(cross_min_bb, cross_max_bb);

            let cross_margin_box = clamped_cross_bb
                + item.cross_axis_margin_extent();

            max_cross_size = max_cross_size.max_of(cross_margin_box);
        }

        line.line_cross_size = max_cross_size;
    }
}

/// Resolve the cross-axis size of a single flex item.
fn resolve_cross_size(
    doc: &Document,
    item: &FlexItem,
    child_style: &openui_style::ComputedStyle,
    is_column: bool,
    cross_border_padding: LayoutUnit,
    child_percentage_inline: LayoutUnit,
    child_percentage_block: LayoutUnit,
) -> LayoutUnit {
    let (cross_prop, pct_base) = if is_column {
        (&child_style.width, child_percentage_inline)
    } else {
        (&child_style.height, child_percentage_block)
    };

    if !cross_prop.is_auto() && (!pct_base.is_indefinite() || cross_prop.is_fixed()) {
        let resolved = resolve_length(cross_prop, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
        if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
            (resolved - cross_border_padding).clamp_negative_to_zero()
        } else {
            resolved
        }
    } else if cross_prop.is_auto() {
        // Check if aspect-ratio can derive cross-axis from the flexed main-axis size
        if let Some(ref ar) = child_style.aspect_ratio {
            let ratio = ar.ratio;
            if ratio.0 > 0.0 && ratio.1 > 0.0 {
                let main_size = item.flexed_border_box_size();
                if !main_size.is_indefinite() && main_size > LayoutUnit::zero() {
                    // AR applies to content-box dimensions. Convert the border-box
                    // main size to content-box before applying the ratio.
                    let main_bp = if is_column {
                        // Column: main=block → block border+padding
                        let b = crate::block::resolve_border(child_style);
                        let p = crate::block::resolve_padding(child_style, child_percentage_inline);
                        b.block_sum() + p.block_sum()
                    } else {
                        // Row: main=inline → inline border+padding
                        let b = crate::block::resolve_border(child_style);
                        let p = crate::block::resolve_padding(child_style, child_percentage_inline);
                        b.inline_sum() + p.inline_sum()
                    };
                    let main_content = (main_size - main_bp).clamp_negative_to_zero();
                    let cross_content = if is_column {
                        // Column: main=block, cross=inline. cross = main * (w/h)
                        LayoutUnit::from_f32(main_content.to_f32() * ratio.0 / ratio.1)
                    } else {
                        // Row: main=inline, cross=block. cross = main * (h/w)
                        LayoutUnit::from_f32(main_content.to_f32() * ratio.1 / ratio.0)
                    };
                    return cross_content;
                }
            }
        }

        // Auto cross size → lay out child to get intrinsic size.
        // For row flex: we know the inline size (main axis), need intrinsic block (cross).
        // For column flex: we know the block size (main axis), need intrinsic inline (cross).
        let child_space = ConstraintSpace::for_block_child(
            if is_column {
                LayoutUnit::from_raw(-64) // indefinite inline to find intrinsic width
            } else {
                item.flexed_border_box_size() // known inline size (main axis)
            },
            if is_column {
                item.flexed_border_box_size() // known block size (main axis)
            } else {
                LayoutUnit::from_raw(-64) // indefinite block to find intrinsic height
            },
            child_percentage_inline,
            child_percentage_block,
            child_style.creates_new_formatting_context(),
        );

        let child_fragment = crate::block::block_layout(doc, item.node_id, &child_space);
        let cross_size = if is_column {
            child_fragment.width()
        } else {
            child_fragment.height()
        };

        (cross_size - cross_border_padding).clamp_negative_to_zero()
    } else {
        // Intrinsic keyword or other non-auto, non-fixed cross size
        let child_space = ConstraintSpace::for_block_child(
            if is_column {
                LayoutUnit::from_raw(-64)
            } else {
                item.flexed_border_box_size()
            },
            if is_column {
                item.flexed_border_box_size()
            } else {
                LayoutUnit::from_raw(-64)
            },
            child_percentage_inline,
            child_percentage_block,
            child_style.creates_new_formatting_context(),
        );

        let child_fragment = crate::block::block_layout(doc, item.node_id, &child_space);
        let cross_size = if is_column {
            child_fragment.width()
        } else {
            child_fragment.height()
        };

        (cross_size - cross_border_padding).clamp_negative_to_zero()
    }
}

/// Compute intrinsic block size (sum of line cross sizes + gaps + border/padding).
fn compute_intrinsic_block_size(
    lines: &[FlexLine],
    is_column: bool,
    gap_between_lines: LayoutUnit,
    border_padding_block: LayoutUnit,
) -> LayoutUnit {
    if is_column {
        // Column: intrinsic block = max line main-axis used size.
        let mut max_line_main = LayoutUnit::zero();
        for line in lines {
            max_line_main = max_line_main.max_of(line.main_axis_used_size);
        }
        max_line_main + border_padding_block
    } else {
        // Row: intrinsic block = sum of line cross sizes + gaps
        let mut total = LayoutUnit::zero();
        for (i, line) in lines.iter().enumerate() {
            total = total + line.line_cross_size;
            if i > 0 {
                total = total + gap_between_lines;
            }
        }
        total + border_padding_block
    }
}

/// Position all items at their final locations.
/// Blink: `GiveItemsFinalPositionAndSize()` at line 1834.
#[allow(clippy::too_many_arguments)]
fn give_items_final_position(
    doc: &Document,
    items: &mut [FlexItem],
    lines: &mut [FlexLine],
    is_column: bool,
    is_reverse: bool,
    is_wrap_reverse: bool,
    _is_horizontal_flow: bool,
    _main_axis_inner_size: LayoutUnit,
    content_cross_size: LayoutUnit,
    gap_between_items: LayoutUnit,
    gap_between_lines: LayoutUnit,
    justify_content: &ContentAlignment,
    align_content: &ContentAlignment,
    border: &BoxStrut,
    padding: &BoxStrut,
    child_percentage_inline: LayoutUnit,
    child_percentage_block: LayoutUnit,
    _space: &ConstraintSpace,
) -> Vec<Fragment> {
    let content_offset_x = border.left + padding.left;
    let content_offset_y = border.top + padding.top;

    // ── Resolve align-content (cross-axis line offsets) ──────────────
    // NOTE: Line stretching (align-content:stretch/normal) was already
    // performed before reversals in flex_layout(), so lines are already
    // at their final cross sizes here. Just compute remaining free space.
    let total_line_cross: LayoutUnit = lines.iter()
        .map(|l| l.line_cross_size)
        .fold(LayoutUnit::zero(), |acc, s| acc + s);

    let num_line_gaps = if lines.len() > 1 { lines.len() as i32 - 1 } else { 0 };
    let total_line_gap = gap_between_lines * num_line_gaps;
    let cross_free_after = content_cross_size - total_line_cross - total_line_gap;

    let cross_align = resolve_content_alignment(
        align_content,
        cross_free_after,
        lines.len(),
        is_wrap_reverse, // wrap-reverse flips cross-axis alignment semantics
        !is_column, // cross axis: row→block(vertical), column→inline(horizontal)
    );

    // Assign cross-axis offsets to lines
    let num_lines = lines.len();
    let mut cross_offset = cross_align.initial_offset;
    for (i, line) in lines.iter_mut().enumerate() {
        line.cross_axis_offset = cross_offset;
        cross_offset = cross_offset + line.line_cross_size;
        if i < num_lines - 1 {
            cross_offset = cross_offset + gap_between_lines + cross_align.between_space;
        }
    }

    // ── Position items within each line ──────────────────────────────
    let mut children = Vec::new();

    for line in lines.iter() {
        // Resolve justify-content for this line
        let effective_free = if line.main_axis_auto_margin_count > 0 {
            // Auto margins consume free space
            LayoutUnit::zero()
        } else {
            line.main_axis_free_space
        };

        let main_align = resolve_content_alignment(
            justify_content,
            effective_free,
            line.item_count(),
            is_reverse,
            is_column, // main axis: row→inline(horizontal), column→block(vertical)
        );

        let mut main_offset = main_align.initial_offset;

        for (item_pos, &idx) in line.item_indices.iter().enumerate() {
            let item = &mut items[idx];
            let child_style = &doc.node(item.node_id).style;

            // ── Resolve main-axis auto margins ───────────────────────
            if item.main_axis_auto_margin_count > 0 && line.main_axis_free_space > LayoutUnit::zero() {
                let is_start_auto = if is_column {
                    child_style.margin_top.is_auto()
                } else {
                    child_style.margin_left.is_auto()
                };
                let is_end_auto = if is_column {
                    child_style.margin_bottom.is_auto()
                } else {
                    child_style.margin_right.is_auto()
                };

                let per_margin_space = LayoutUnit::from_raw(
                    line.main_axis_free_space.raw() / line.main_axis_auto_margin_count as i32
                );

                let (start_margin, end_margin) = resolve_main_auto_margins(
                    per_margin_space * item.main_axis_auto_margin_count as i32,
                    is_start_auto,
                    is_end_auto,
                );

                // Apply auto margins
                if is_column {
                    item.margin.top = start_margin;
                    item.margin.bottom = end_margin;
                } else {
                    item.margin.left = start_margin;
                    item.margin.right = end_margin;
                }
            }

            // ── Layout child with final sizes ────────────────────────
            let child_border = resolve_border(child_style);
            let child_padding = resolve_padding(child_style, child_percentage_inline);

            let flexed_border_box = item.flexed_border_box_size();

            let cross_border_padding = if is_column {
                child_border.inline_sum() + child_padding.inline_sum()
            } else {
                child_border.block_sum() + child_padding.block_sum()
            };

            // Determine if item should stretch on cross axis
            // Stretch only applies to items with auto cross size (CSS Flexbox §9.4)
            let cross_size_is_auto = if is_column {
                child_style.width.is_auto()
            } else {
                child_style.height.is_auto()
            };
            let should_stretch = item.alignment == ItemPosition::Stretch && cross_size_is_auto;
            let cross_size_for_child = if should_stretch {
                let stretch_size = line.line_cross_size - item.cross_axis_margin_extent();
                let stretch_size = stretch_size.clamp_negative_to_zero();
                // Clamp against cross-axis min/max (CSS Flexbox §9.4)
                // stretch_size is border-box, so convert min/max to border-box for clamping
                let (cross_min_prop, cross_max_prop) = if is_column {
                    (&child_style.min_width, &child_style.max_width)
                } else {
                    (&child_style.min_height, &child_style.max_height)
                };
                let cross_pct_base = if is_column { child_percentage_inline } else { child_percentage_block };
                let cross_min_raw = resolve_cross_min_max(doc, item.node_id, cross_min_prop, is_column, cross_pct_base, true);
                let cross_max_raw = resolve_cross_min_max(doc, item.node_id, cross_max_prop, is_column, cross_pct_base, false);
                // Convert content-box min/max to border-box for comparison with stretch_size
                let cross_min = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                    cross_min_raw
                } else if cross_min_raw > LayoutUnit::zero() {
                    cross_min_raw + cross_border_padding
                } else {
                    cross_min_raw
                };
                let cross_max = if cross_max_raw == LayoutUnit::from_i32(33554431) {
                    cross_max_raw
                } else if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                    cross_max_raw
                } else {
                    cross_max_raw + cross_border_padding
                };
                stretch_size.clamp(cross_min, cross_max)
            } else {
                // Use child's natural cross size, clamped by min/max constraints
                let cross_content = resolve_cross_size(
                    doc, item, child_style, is_column,
                    cross_border_padding,
                    child_percentage_inline, child_percentage_block,
                );
                let natural_bb = cross_content + cross_border_padding;
                // Apply cross-axis min/max constraints
                let (cross_min_prop, cross_max_prop) = if is_column {
                    (&child_style.min_width, &child_style.max_width)
                } else {
                    (&child_style.min_height, &child_style.max_height)
                };
                let cross_pct_base = if is_column { child_percentage_inline } else { child_percentage_block };
                let cross_min_raw = resolve_cross_min_max(doc, item.node_id, cross_min_prop, is_column, cross_pct_base, true);
                let cross_max_raw = resolve_cross_min_max(doc, item.node_id, cross_max_prop, is_column, cross_pct_base, false);
                let cross_min_bb = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                    cross_min_raw
                } else if cross_min_raw > LayoutUnit::zero() {
                    cross_min_raw + cross_border_padding
                } else {
                    cross_min_raw
                };
                let cross_max_bb = if cross_max_raw == LayoutUnit::from_i32(33554431) {
                    cross_max_raw
                } else if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                    cross_max_raw
                } else {
                    cross_max_raw + cross_border_padding
                };
                natural_bb.clamp(cross_min_bb, cross_max_bb)
            };

            // Build final constraint space.
            // NOTE: We do NOT re-derive main size from AR when the item is
            // stretched. The flex algorithm has already determined the main
            // size (via grow/shrink). The stretched cross size is independent.
            // Chrome: stretched items keep their flexed main size even with AR.
            let final_main = flexed_border_box;

            let (inline_size, block_size) = if is_column {
                (cross_size_for_child, final_main)
            } else {
                (final_main, cross_size_for_child)
            };

            // CSS Flexbox §9.8: When a flex item is stretched, its cross size
            // becomes definite for percentage resolution by its children.
            // Also, the main size is always definite (set by flex sizing).
            let item_pct_block = if is_column {
                // Column: main axis is block. Use main axis content size.
                (final_main - item.main_axis_border_padding).clamp_negative_to_zero()
            } else {
                // Row: main axis is inline, cross is block.
                // If item is stretched or has definite cross size, use it.
                if should_stretch || child_percentage_block.is_indefinite() {
                    // Compute cross-axis border+padding from style
                    let child_style = &doc.node(item.node_id).style;
                    let bp_cross = {
                        let bp = child_style.border_top_width as i32
                            + child_style.border_bottom_width as i32;
                        let pad_t = crate::length_resolver::resolve_margin_or_padding(
                            &child_style.padding_top, child_percentage_inline);
                        let pad_b = crate::length_resolver::resolve_margin_or_padding(
                            &child_style.padding_bottom, child_percentage_inline);
                        LayoutUnit::from_i32(bp) + pad_t + pad_b
                    };
                    (cross_size_for_child - bp_cross).clamp_negative_to_zero()
                } else {
                    child_percentage_block
                }
            };

            let mut child_space = ConstraintSpace::for_flex_child(
                inline_size,
                block_size,
                child_percentage_inline,
                item_pct_block,
            );

            if is_column {
                child_space.is_fixed_block_size = true;
                if should_stretch {
                    child_space.stretch_inline_size = true;
                }
            } else {
                child_space.is_fixed_inline_size = true;
                if should_stretch {
                    child_space.stretch_block_size = true;
                }
            }

            let child_fragment = crate::block::block_layout(doc, item.node_id, &child_space);

            // ── Compute cross-axis offset (align-self) ───────────────
            let item_cross_margin_box = if is_column {
                child_fragment.width() + item.cross_axis_margin_extent()
            } else {
                child_fragment.height() + item.cross_axis_margin_extent()
            };

            let cross_space = line.line_cross_size - item_cross_margin_box;

            // Check for cross-axis auto margins
            let has_cross_auto_margins = if is_column {
                child_style.margin_left.is_auto() || child_style.margin_right.is_auto()
            } else {
                child_style.margin_top.is_auto() || child_style.margin_bottom.is_auto()
            };

            let cross_item_offset = if has_cross_auto_margins {
                let is_start_auto = if is_column {
                    child_style.margin_left.is_auto()
                } else {
                    child_style.margin_top.is_auto()
                };
                let is_end_auto = if is_column {
                    child_style.margin_right.is_auto()
                } else {
                    child_style.margin_bottom.is_auto()
                };
                let (start, _end) = resolve_cross_auto_margins(cross_space, is_start_auto, is_end_auto);
                start
            } else {
                resolve_align_self(
                    item.alignment,
                    cross_space,
                    child_style.align_self.overflow,
                    is_wrap_reverse,
                )
            };

            // ── Compute physical position ────────────────────────────
            let main_margin_start = if is_column { item.margin.top } else { item.margin.left };
            let cross_margin_start = if is_column { item.margin.left } else { item.margin.top };

            let item_main_pos = main_offset + main_margin_start;
            let item_cross_pos = line.cross_axis_offset + cross_item_offset + cross_margin_start;

            let (x, y) = if is_column {
                (content_offset_x + item_cross_pos, content_offset_y + item_main_pos)
            } else {
                (content_offset_x + item_main_pos, content_offset_y + item_cross_pos)
            };

            let mut positioned = child_fragment;
            positioned.offset = PhysicalOffset::new(x, y);
            positioned.margin = item.margin.clone();

            // Apply relative positioning offsets (CSS 2.1 §9.4.3).
            crate::relative::apply_relative_offset(
                &mut positioned,
                child_style,
                child_percentage_inline,
                child_percentage_block,
            );

            // Advance main offset (compute before push moves positioned)
            let main_margin_end = if is_column { item.margin.bottom } else { item.margin.right };
            let item_main_size = if is_column {
                positioned.height()
            } else {
                positioned.width()
            };

            children.push(positioned);

            main_offset = main_offset + main_margin_start + item_main_size + main_margin_end;

            if item_pos < line.item_count() - 1 {
                main_offset = main_offset + gap_between_items + main_align.between_space;
            }
        }
    }

    children
}

/// Compute the static position for an abspos child of a flex container.
///
/// Per CSS Flexbox §4.1, the child is positioned as if it were the sole flex
/// item, applying the container's `justify-content` (main axis) and
/// `align-items` (cross axis) alignment.
fn compute_abspos_static_position(
    doc: &Document,
    child_id: NodeId,
    child_style: &openui_style::ComputedStyle,
    container_style: &openui_style::ComputedStyle,
    content_width: LayoutUnit,
    content_height: LayoutUnit,
    is_column: bool,
    border: &BoxStrut,
    padding: &BoxStrut,
) -> (LayoutUnit, LayoutUnit) {
    use openui_style::{ContentPosition, ContentDistribution, ItemPosition};

    // Layout the abspos child to determine its hypothetical size.
    let child_space = crate::constraint_space::ConstraintSpace::for_block_child(
        content_width,
        content_height,
        content_width,
        content_height,
        false,
    );
    let child_fragment = crate::block::block_layout(doc, child_id, &child_space);
    let child_margins = resolve_margins(child_style, content_width);
    let child_w = child_fragment.size.width + child_margins.left + child_margins.right;
    let child_h = child_fragment.size.height + child_margins.top + child_margins.bottom;

    let (main_size, cross_size, child_main, child_cross) = if is_column {
        (content_height, content_width, child_h, child_w)
    } else {
        (content_width, content_height, child_w, child_h)
    };

    // Main axis: apply justify-content
    let jc = &container_style.justify_content;
    let main_free = (main_size - child_main).clamp_negative_to_zero();
    let main_offset = match jc.position {
        ContentPosition::Center => main_free / 2,
        ContentPosition::End | ContentPosition::FlexEnd => {
            if container_style.flex_direction.is_reverse() {
                LayoutUnit::zero()
            } else {
                main_free
            }
        }
        ContentPosition::Start | ContentPosition::FlexStart | ContentPosition::Normal => {
            if container_style.flex_direction.is_reverse() {
                main_free
            } else {
                LayoutUnit::zero()
            }
        }
        _ => LayoutUnit::zero(),
    };

    // Cross axis: resolve align-self (auto → container's align-items)
    let ai_pos = {
        let self_pos = child_style.align_self.position;
        if self_pos == ItemPosition::Auto || self_pos == ItemPosition::Normal {
            let items_pos = container_style.align_items.position;
            if items_pos == ItemPosition::Normal {
                ItemPosition::Stretch
            } else {
                items_pos
            }
        } else {
            self_pos
        }
    };
    let cross_free = (cross_size - child_cross).clamp_negative_to_zero();
    let cross_offset = match ai_pos {
        ItemPosition::Center => cross_free / 2,
        ItemPosition::End | ItemPosition::FlexEnd => cross_free,
        _ => LayoutUnit::zero(),
    };

    let (x_off, y_off) = if is_column {
        (cross_offset, main_offset)
    } else {
        (main_offset, cross_offset)
    };

    (border.left + padding.left + x_off, border.top + padding.top + y_off)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::Document;
    use openui_geometry::{LayoutUnit, Length};
    use openui_style::{Display, FlexDirection, FlexWrap, ItemAlignment, ItemPosition};

    fn make_flex_container(doc: &mut Document, width: i32, height: i32) -> NodeId {
        let root = doc.root();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        {
            let style = doc.node_mut(container).style_mut();
            style.display = Display::Flex;
            style.width = Length::px(width as f32);
            style.height = Length::px(height as f32);
        }
        doc.append_child(root, container);
        container
    }

    fn add_flex_child(doc: &mut Document, parent: NodeId, width: i32, height: i32) -> NodeId {
        let child = doc.create_node(openui_dom::ElementTag::Div);
        {
            let style = doc.node_mut(child).style_mut();
            style.display = Display::Block;
            style.width = Length::px(width as f32);
            style.height = Length::px(height as f32);
        }
        doc.append_child(parent, child);
        child
    }

    #[test]
    fn basic_row_flex_three_items() {
        let mut doc = Document::new();
        let container = make_flex_container(&mut doc, 300, 100);
        let _c1 = add_flex_child(&mut doc, container, 50, 50);
        let _c2 = add_flex_child(&mut doc, container, 80, 50);
        let _c3 = add_flex_child(&mut doc, container, 70, 50);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(300),
            LayoutUnit::from_i32(100),
        );

        let fragment = flex_layout(&doc, container, &space);

        assert_eq!(fragment.children.len(), 3);
        // Items should be placed left-to-right: 0, 50, 130
        assert_eq!(fragment.children[0].offset.left, LayoutUnit::zero());
        assert_eq!(fragment.children[1].offset.left, LayoutUnit::from_i32(50));
        assert_eq!(fragment.children[2].offset.left, LayoutUnit::from_i32(130));

        // Heights remain 50px (explicit height overrides stretch)
        assert_eq!(fragment.children[0].height(), LayoutUnit::from_i32(50));
        assert_eq!(fragment.children[1].height(), LayoutUnit::from_i32(50));
    }

    #[test]
    fn flex_grow_equal() {
        let mut doc = Document::new();
        let container = make_flex_container(&mut doc, 300, 100);
        let c1 = doc.create_node(openui_dom::ElementTag::Div);
        let c2 = doc.create_node(openui_dom::ElementTag::Div);
        {
            let s = doc.node_mut(c1).style_mut();
            s.display = Display::Block;
            s.flex_grow = 1.0;
            s.height = Length::px(50.0);
        }
        {
            let s = doc.node_mut(c2).style_mut();
            s.display = Display::Block;
            s.flex_grow = 1.0;
            s.height = Length::px(50.0);
        }
        doc.append_child(container, c1);
        doc.append_child(container, c2);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(300),
            LayoutUnit::from_i32(100),
        );

        let fragment = flex_layout(&doc, container, &space);

        // Each should get 150px width (300 / 2)
        assert_eq!(fragment.children[0].width(), LayoutUnit::from_i32(150));
        assert_eq!(fragment.children[1].width(), LayoutUnit::from_i32(150));
    }

    #[test]
    fn flex_grow_weighted() {
        let mut doc = Document::new();
        let container = make_flex_container(&mut doc, 400, 100);

        for (grow, _) in [(1.0, 50), (2.0, 50), (1.0, 50)] {
            let child = doc.create_node(openui_dom::ElementTag::Div);
            {
                let s = doc.node_mut(child).style_mut();
                s.display = Display::Block;
                s.flex_grow = grow;
                s.height = Length::px(50.0);
            }
            doc.append_child(container, child);
        }

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(400),
            LayoutUnit::from_i32(100),
        );

        let fragment = flex_layout(&doc, container, &space);

        // Proportions: 1:2:1 of 400 = 100:200:100
        assert_eq!(fragment.children[0].width(), LayoutUnit::from_i32(100));
        assert_eq!(fragment.children[1].width(), LayoutUnit::from_i32(200));
        assert_eq!(fragment.children[2].width(), LayoutUnit::from_i32(100));
    }

    #[test]
    fn justify_content_center() {
        let mut doc = Document::new();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        {
            let s = doc.node_mut(container).style_mut();
            s.display = Display::Flex;
            s.width = Length::px(400.0);
            s.height = Length::px(100.0);
            s.justify_content = ContentAlignment::new(openui_style::ContentPosition::Center);
        }
        doc.append_child(doc.root(), container);

        let _c1 = add_flex_child(&mut doc, container, 50, 50);
        let _c2 = add_flex_child(&mut doc, container, 50, 50);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(400),
            LayoutUnit::from_i32(100),
        );

        let fragment = flex_layout(&doc, container, &space);

        // Total items = 100, free space = 300, offset = 150
        assert_eq!(fragment.children[0].offset.left, LayoutUnit::from_i32(150));
        assert_eq!(fragment.children[1].offset.left, LayoutUnit::from_i32(200));
    }

    #[test]
    fn justify_content_space_between() {
        let mut doc = Document::new();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        {
            let s = doc.node_mut(container).style_mut();
            s.display = Display::Flex;
            s.width = Length::px(400.0);
            s.height = Length::px(100.0);
            s.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
        }
        doc.append_child(doc.root(), container);

        let _c1 = add_flex_child(&mut doc, container, 50, 50);
        let _c2 = add_flex_child(&mut doc, container, 50, 50);
        let _c3 = add_flex_child(&mut doc, container, 50, 50);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(400),
            LayoutUnit::from_i32(100),
        );

        let fragment = flex_layout(&doc, container, &space);

        // Free space = 250, 2 gaps → 125 each
        // Positions: 0, 50+125=175, 175+50+125=350
        assert_eq!(fragment.children[0].offset.left, LayoutUnit::from_i32(0));
        assert_eq!(fragment.children[1].offset.left, LayoutUnit::from_i32(175));
        assert_eq!(fragment.children[2].offset.left, LayoutUnit::from_i32(350));
    }

    #[test]
    fn flex_wrap_basic() {
        let mut doc = Document::new();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        {
            let s = doc.node_mut(container).style_mut();
            s.display = Display::Flex;
            s.width = Length::px(200.0);
            s.flex_wrap = FlexWrap::Wrap;
        }
        doc.append_child(doc.root(), container);

        // 3 items of 100px each → wrap after 2
        add_flex_child(&mut doc, container, 100, 50);
        add_flex_child(&mut doc, container, 100, 50);
        add_flex_child(&mut doc, container, 100, 50);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(200),
            LayoutUnit::from_i32(300),
        );

        let fragment = flex_layout(&doc, container, &space);

        assert_eq!(fragment.children.len(), 3);
        // First two on line 1, third on line 2
        assert_eq!(fragment.children[0].offset.top, LayoutUnit::zero());
        assert_eq!(fragment.children[1].offset.top, LayoutUnit::zero());
        assert_eq!(fragment.children[2].offset.top, LayoutUnit::from_i32(50));
    }

    #[test]
    fn flex_direction_column() {
        let mut doc = Document::new();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        {
            let s = doc.node_mut(container).style_mut();
            s.display = Display::Flex;
            s.width = Length::px(200.0);
            s.height = Length::px(300.0);
            s.flex_direction = FlexDirection::Column;
        }
        doc.append_child(doc.root(), container);

        add_flex_child(&mut doc, container, 50, 50);
        add_flex_child(&mut doc, container, 50, 80);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(200),
            LayoutUnit::from_i32(300),
        );

        let fragment = flex_layout(&doc, container, &space);

        // Column: items stack vertically
        assert_eq!(fragment.children[0].offset.top, LayoutUnit::zero());
        assert_eq!(fragment.children[1].offset.top, LayoutUnit::from_i32(50));
        // Cross axis (x): explicit width=50 doesn't stretch
        assert_eq!(fragment.children[0].width(), LayoutUnit::from_i32(50));
    }

    #[test]
    fn gap_between_items() {
        let mut doc = Document::new();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        {
            let s = doc.node_mut(container).style_mut();
            s.display = Display::Flex;
            s.width = Length::px(400.0);
            s.height = Length::px(100.0);
            s.column_gap = Some(Length::px(20.0));
        }
        doc.append_child(doc.root(), container);

        add_flex_child(&mut doc, container, 50, 50);
        add_flex_child(&mut doc, container, 50, 50);
        add_flex_child(&mut doc, container, 50, 50);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(400),
            LayoutUnit::from_i32(100),
        );

        let fragment = flex_layout(&doc, container, &space);

        // Positions: 0, 50+20=70, 70+50+20=140
        assert_eq!(fragment.children[0].offset.left, LayoutUnit::from_i32(0));
        assert_eq!(fragment.children[1].offset.left, LayoutUnit::from_i32(70));
        assert_eq!(fragment.children[2].offset.left, LayoutUnit::from_i32(140));
    }

    #[test]
    fn order_property() {
        let mut doc = Document::new();
        let container = make_flex_container(&mut doc, 300, 100);

        let c1 = add_flex_child(&mut doc, container, 50, 50);
        let c2 = add_flex_child(&mut doc, container, 50, 50);
        let c3 = add_flex_child(&mut doc, container, 50, 50);

        doc.node_mut(c1).style_mut().order = 3;
        doc.node_mut(c2).style_mut().order = 1;
        doc.node_mut(c3).style_mut().order = 2;

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(300),
            LayoutUnit::from_i32(100),
        );

        let fragment = flex_layout(&doc, container, &space);

        // Order: c2(1), c3(2), c1(3)
        // c2 should be first (leftmost), c1 should be last (rightmost)
        assert_eq!(fragment.children[0].node_id, c2);
        assert_eq!(fragment.children[1].node_id, c3);
        assert_eq!(fragment.children[2].node_id, c1);
    }

    #[test]
    fn align_items_center() {
        let mut doc = Document::new();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        {
            let s = doc.node_mut(container).style_mut();
            s.display = Display::Flex;
            s.width = Length::px(300.0);
            s.height = Length::px(100.0);
            s.align_items = ItemAlignment::new(ItemPosition::Center);
        }
        doc.append_child(doc.root(), container);

        add_flex_child(&mut doc, container, 50, 40);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(300),
            LayoutUnit::from_i32(100),
        );

        let fragment = flex_layout(&doc, container, &space);

        // Cross space = 100 - 40 = 60, center offset = 30
        assert_eq!(fragment.children[0].offset.top, LayoutUnit::from_i32(30));
        assert_eq!(fragment.children[0].height(), LayoutUnit::from_i32(40));
    }

    #[test]
    fn flex_direction_row_reverse() {
        let mut doc = Document::new();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        {
            let s = doc.node_mut(container).style_mut();
            s.display = Display::Flex;
            s.width = Length::px(300.0);
            s.height = Length::px(100.0);
            s.flex_direction = FlexDirection::RowReverse;
        }
        doc.append_child(doc.root(), container);

        let c1 = add_flex_child(&mut doc, container, 50, 50);
        let c2 = add_flex_child(&mut doc, container, 80, 50);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(300),
            LayoutUnit::from_i32(100),
        );

        let fragment = flex_layout(&doc, container, &space);

        // Row-reverse: items reversed, justify-content flex-start means right side
        // free_space = 300 - 130 = 170
        // With reverse, initial offset should be at the end
        // Items appear: c2 first (at higher x), c1 second (at lower x)
        assert_eq!(fragment.children.len(), 2);
        // c2 should come before c1 in the children (reversed)
        assert_eq!(fragment.children[0].node_id, c2);
        assert_eq!(fragment.children[1].node_id, c1);
    }
}
