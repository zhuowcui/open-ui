//! Flex layout algorithm — the main entry point.
//!
//! Extracted from Blink's `FlexLayoutAlgorithm::LayoutInternal()` and
//! `PlaceFlexItems()` (flex_layout_algorithm.cc:1229, 1394).
//!
//! Orchestrates: item collection → line breaking → flexing → alignment → positioning.

use openui_dom::{Document, NodeId};
use openui_geometry::Length;
use openui_geometry::{
    BoxStrut, LayoutUnit, LengthType, MinMaxSizes, PhysicalOffset, PhysicalRect, PhysicalSize,
};
use openui_style::{
    ContentAlignment, ContentDistribution, ContentPosition, FlexWrap, ItemPosition,
    OverflowAlignment,
};

use crate::block::{resolve_border, resolve_margins, resolve_padding};
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
    let is_rtl = style.direction == openui_style::Direction::Rtl;
    // CSS Flexbox §4.1: for row flex, direction:rtl reverses the main axis.
    // This is modeled as XOR with the flex-direction reverse flag.
    let is_reverse = if !is_column && is_rtl {
        !style.flex_direction.is_reverse()
    } else {
        style.flex_direction.is_reverse()
    };
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
    let container_inline_size =
        resolve_container_inline_size(doc, node_id, style, space, border_padding_inline);

    // Content-box sizes
    let content_inline_size = container_inline_size - border_padding_inline;

    // ── Resolve gaps (Blink line 187-191) ────────────────────────────
    // CSS Box Alignment §8: row-gap % resolves against block size,
    // column-gap % resolves against inline size.
    let column_gap_pct_base = content_inline_size;
    let row_gap_pct_base = if !style.height.is_auto() && !style.height.is_content_or_intrinsic() {
        let raw = resolve_length(
            &style.height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        if style.box_sizing == openui_style::BoxSizing::BorderBox {
            (raw - border_padding_block).clamp_negative_to_zero()
        } else {
            raw
        }
    } else if space.is_fixed_block_size || space.stretch_block_size {
        (space.available_block_size - border_padding_block).clamp_negative_to_zero()
    } else {
        // Auto height → percentage gap resolves to 0
        LayoutUnit::zero()
    };
    let gap_between_items = resolve_gap(
        if is_column {
            &style.row_gap
        } else {
            &style.column_gap
        },
        if is_column {
            row_gap_pct_base
        } else {
            column_gap_pct_base
        },
    );
    let gap_between_lines = resolve_gap(
        if is_column {
            &style.column_gap
        } else {
            &style.row_gap
        },
        if is_column {
            column_gap_pct_base
        } else {
            row_gap_pct_base
        },
    );

    // ── Main axis inner size ─────────────────────────────────────────
    let main_axis_inner_size = if is_column {
        // Column: main axis = block, may be indefinite
        let resolved = resolve_container_block_size_for_flex(
            style,
            space,
            border_padding_block,
            container_inline_size,
        );
        // If resolved to indefinite but parent provided a fixed height, use it for
        // wrapping decisions (CSS Flexbox §9.2: definite size from containing block)
        if resolved.is_indefinite() && (space.is_fixed_block_size || space.stretch_block_size) {
            (space.available_block_size - border_padding_block).clamp_negative_to_zero()
        } else if resolved.is_indefinite()
            && style.flex_wrap != FlexWrap::Nowrap
            && space.fragmentainer_block_size > LayoutUnit::zero()
        {
            // Inside a fragmentainer (multicol column) with flex-wrap: use the
            // column height as the wrapping boundary so items wrap correctly.
            // Only for wrapping containers — non-wrapping ones should stay
            // indefinite and be fragmented by the multicol distribution pass.
            (space.fragmentainer_block_size - border_padding_block).clamp_negative_to_zero()
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
            let raw = resolve_length(
                &style.height,
                space.percentage_resolution_block_size,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            let content = if style.box_sizing == openui_style::BoxSizing::BorderBox {
                (raw - border_padding_block).clamp_negative_to_zero()
            } else {
                raw
            };
            content
        }
    } else if (space.is_fixed_block_size || space.stretch_block_size)
        && !space.is_initial_block_size_indefinite
    {
        // CSS Flexbox §9.8: Parent flex has set a definite block size for this
        // container (e.g. stretch or fixed). Treat it as the percentage base
        // so that percentage-height children resolve correctly.
        // BUT if is_initial_block_size_indefinite is set, the parent determined
        // its size from content (auto height) so percentages stay indefinite.
        (space.available_block_size - border_padding_block).clamp_negative_to_zero()
    } else if let Some(ar) = &style.aspect_ratio {
        // Aspect-ratio with definite inline size gives a definite block size
        // for percentage resolution (CSS Sizing L4).
        if ar.ratio.0 > 0.0
            && ar.ratio.1 > 0.0
            && container_inline_size.raw() > 0
            && !container_inline_size.is_indefinite()
        {
            let ratio = ar.ratio.0 / ar.ratio.1;
            let ar_block = LayoutUnit::from_f32(container_inline_size.to_f32() / ratio);
            (ar_block - border_padding_block).clamp_negative_to_zero()
        } else {
            LayoutUnit::from_raw(-64) // indefinite
        }
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
    let mut effective_main_axis_inner_size = main_axis_inner_size;
    if is_column
        && style.height.is_auto()
        && !main_axis_inner_size.is_indefinite()
        && !style.max_height.is_auto()
        && !style.max_height.is_none()
    {
        let all_lines_fit_hypothetical = flex_lines.iter().all(|line| {
            let visible_count = line
                .item_indices
                .iter()
                .filter(|&&idx| !flex_items[idx].is_collapsed)
                .count() as i32;
            let gap_count = if visible_count > 1 {
                visible_count - 1
            } else {
                0
            };
            let total_gap = gap_between_items * gap_count;
            let sum_hyp = line
                .item_indices
                .iter()
                .map(|&idx| flex_items[idx].hypothetical_main_axis_margin_box_size())
                .fold(LayoutUnit::zero(), |acc, s| acc + s);
            sum_hyp + total_gap <= main_axis_inner_size
        });
        if all_lines_fit_hypothetical {
            effective_main_axis_inner_size = LayoutUnit::from_raw(-64);
        }
    }

    // ── Step C: Flex each line (CSS §9.7) ────────────────────────────
    for line in &mut flex_lines {
        let sum_hyp: LayoutUnit = line
            .item_indices
            .iter()
            .map(|&idx| flex_items[idx].hypothetical_main_axis_margin_box_size())
            .fold(LayoutUnit::zero(), |acc, s| acc + s);

        // When main axis is indefinite (auto-height column), skip grow/shrink.
        // Items stay at their hypothetical sizes.
        if !effective_main_axis_inner_size.is_indefinite() {
            let mut flexer = LineFlexer::new(
                &mut flex_items,
                &line.item_indices,
                effective_main_axis_inner_size,
                sum_hyp,
                gap_between_items,
            );
            flexer.run();
        } else {
            // CSS Flexbox §9.9.1: For auto-height column flex, each item's
            // max-content contribution is max(outer max-content size, outer
            // hypothetical main size). When an item has a definite main-size
            // property (height for column), its max-content size equals that
            // resolved height, which may exceed the hypothetical size.
            // BUT: this only applies when flex-basis is auto (§9.2 step A),
            // since an explicit flex-basis overrides the main-size property.
            for &idx in &line.item_indices {
                let mut frozen = flex_items[idx].hypothetical_content_size;

                if is_column {
                    let child_style = &doc.node(flex_items[idx].node_id).style;
                    // Only use the height property when flex-basis is auto.
                    // An explicit flex-basis (e.g. 0%) means the item's main
                    // size was intentionally set and should not be overridden.
                    if child_style.flex_basis.is_auto() {
                        let height = &child_style.height;
                        if !height.is_auto() && !height.is_content_or_intrinsic() {
                            if height.is_fixed() || !child_percentage_block.is_indefinite() {
                                let resolved = resolve_length(
                                    height,
                                    child_percentage_block,
                                    LayoutUnit::zero(),
                                    LayoutUnit::zero(),
                                );
                                let content = if child_style.box_sizing
                                    == openui_style::BoxSizing::BorderBox
                                {
                                    (resolved - flex_items[idx].main_axis_border_padding)
                                        .clamp_negative_to_zero()
                                } else {
                                    resolved
                                };
                                let clamped = flex_items[idx].main_axis_min_max.clamp(content);
                                frozen = frozen.max_of(clamped);
                            }
                        }
                    }
                }

                flex_items[idx].flexed_content_size = frozen;
                flex_items[idx].state = super::item::FlexerState::Frozen;
            }
        }

        // Compute free space after flexing
        let total_flexed: LayoutUnit = line
            .item_indices
            .iter()
            .map(|&idx| flex_items[idx].flexed_margin_box_size())
            .fold(LayoutUnit::zero(), |acc, s| acc + s);

        let num_visible = line
            .item_indices
            .iter()
            .filter(|&&idx| !flex_items[idx].is_collapsed)
            .count() as i32;
        let num_gaps = if num_visible > 1 { num_visible - 1 } else { 0 };
        let total_gap = gap_between_items * num_gaps;
        if !effective_main_axis_inner_size.is_indefinite() {
            line.main_axis_free_space = effective_main_axis_inner_size - total_flexed - total_gap;
        } else {
            line.main_axis_free_space = LayoutUnit::zero();
        }
        line.main_axis_used_size = total_flexed + total_gap;

        // Count auto margins on main axis
        line.main_axis_auto_margin_count = line
            .item_indices
            .iter()
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
                let raw = resolve_length(
                    &style.height,
                    space.percentage_resolution_block_size,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
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
                    let min_raw = resolve_length(
                        &style.min_height,
                        pct_base,
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                    );
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
                    let max_raw = resolve_length(
                        &style.max_height,
                        pct_base,
                        LayoutUnit::zero(),
                        LayoutUnit::from_i32(33554431),
                    );
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
        &flex_lines,
        is_column,
        gap_between_lines,
        border_padding_block,
    );

    let total_block_size = resolve_total_block_size(
        doc,
        node_id,
        style,
        space,
        intrinsic_block_size,
        border_padding_block,
        container_inline_size,
    );

    // ── Fix: Recalculate main-axis free space for column flex ────────
    // When the main axis was initially indefinite (auto-height column),
    // items were frozen at hypothetical sizes with free_space=0. But
    // total_block_size may be larger than intrinsic_block_size (due to
    // min-height, stretch from parent, etc.). Recalculate so that
    // justify-content and column-reverse positioning use the real free space.
    // Blink: LayoutColumnReverse() recalculates offsets using the resolved size.
    if is_column && effective_main_axis_inner_size.is_indefinite() {
        let resolved_main = (total_block_size - border_padding_block).clamp_negative_to_zero();
        for line in &mut flex_lines {
            let num_vis = line
                .item_indices
                .iter()
                .filter(|&&idx| !flex_items[idx].is_collapsed)
                .count() as i32;
            let num_gaps = if num_vis > 1 { num_vis - 1 } else { 0 };
            let total_gap = gap_between_items * num_gaps;
            line.main_axis_free_space = resolved_main - line.main_axis_used_size - total_gap;
        }
    }
    if is_column {
        let resolved_main = (total_block_size - border_padding_block).clamp_negative_to_zero();
        let should_reflex_to_resolved_main = resolved_main.raw() > 0
            && (effective_main_axis_inner_size.is_indefinite()
                || resolved_main != effective_main_axis_inner_size);
        if should_reflex_to_resolved_main {
            effective_main_axis_inner_size = resolved_main;
            for line in &mut flex_lines {
                let sum_hyp: LayoutUnit = line
                    .item_indices
                    .iter()
                    .map(|&idx| flex_items[idx].hypothetical_main_axis_margin_box_size())
                    .fold(LayoutUnit::zero(), |acc, s| acc + s);
                let mut flexer = LineFlexer::new(
                    &mut flex_items,
                    &line.item_indices,
                    resolved_main,
                    sum_hyp,
                    gap_between_items,
                );
                flexer.run();

                let total_flexed = line
                    .item_indices
                    .iter()
                    .map(|&idx| flex_items[idx].flexed_margin_box_size())
                    .fold(LayoutUnit::zero(), |acc, s| acc + s);
                let visible_count = line
                    .item_indices
                    .iter()
                    .filter(|&&idx| !flex_items[idx].is_collapsed)
                    .count() as i32;
                let gap_count = if visible_count > 1 {
                    visible_count - 1
                } else {
                    0
                };
                let total_gap = gap_between_items * gap_count;
                line.main_axis_used_size = total_flexed + total_gap;
                line.main_axis_free_space = resolved_main - total_flexed - total_gap;
            }
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
        let total_line_cross: LayoutUnit = flex_lines
            .iter()
            .map(|l| l.line_cross_size)
            .fold(LayoutUnit::zero(), |acc, s| acc + s);
        let num_line_gaps = if flex_lines.len() > 1 {
            flex_lines.len() as i32 - 1
        } else {
            0
        };
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
                let bonus = if (i as i32) < remainder {
                    LayoutUnit::from_raw(1)
                } else {
                    LayoutUnit::zero()
                };
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

    let (children, computed_first_baseline, computed_last_baseline) = give_items_final_position(
        doc,
        &mut flex_items,
        &mut flex_lines,
        is_column,
        is_reverse,
        is_wrap_reverse,
        is_horizontal_flow,
        is_rtl,
        effective_main_axis_inner_size,
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
    let final_inline_size =
        if container_inline_size.is_indefinite() || container_inline_size < LayoutUnit::zero() {
            // Compute from actual item sizes: max of all lines' used sizes
            let max_line_main = flex_lines
                .iter()
                .map(|line| line.main_axis_used_size)
                .fold(LayoutUnit::zero(), |acc, s| acc.max_of(s));
            let result = if is_column {
                let max_child_width = children
                    .iter()
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
    let content_width = (final_inline_size - border_padding_inline).clamp_negative_to_zero();
    let content_height = (total_block_size - border_padding_block).clamp_negative_to_zero();
    let cb_size = PhysicalSize::new(content_width, content_height);

    let mut oof_candidates = Vec::new();
    let mut bubbled_oof_candidates = Vec::new();
    let is_root = node_id == doc.root();
    let establishes_cb_for_abspos = style.position.is_positioned() || is_root;

    // Collect bubbled OOF candidates from flex item fragments.
    for child_frag in &mut fragment.children {
        let bubbled = std::mem::take(&mut child_frag.oof_candidates);
        for mut c in bubbled {
            c.static_position.left = c.static_position.left + child_frag.offset.left;
            c.static_position.top = c.static_position.top + child_frag.offset.top;
            let captures = if c.style.position == openui_style::Position::Fixed {
                is_root
            } else {
                establishes_cb_for_abspos
            };
            if captures {
                c.containing_block_size = cb_size;
                c.containing_block_border = border.clone();
                c.containing_block_direction = style.direction;
                oof_candidates.push(c);
            } else {
                bubbled_oof_candidates.push(c);
            }
        }
    }

    for child_id in flex_abspos_children(doc, node_id) {
        let child_style = &doc.node(child_id).style;
        if child_style.position.is_absolutely_positioned() {
            // CSS Flexbox §4.1: The static position of an abspos child of a
            // flex container is determined as if the child were the sole flex
            // item in the container, using the container's alignment properties.
            let (sp_x, sp_y) = compute_abspos_static_position(
                doc,
                child_id,
                child_style,
                style,
                content_width,
                content_height,
                is_column,
                &border,
                &padding,
            );
            let candidate = crate::out_of_flow::OutOfFlowCandidate {
                node_id: child_id,
                style: child_style.clone(),
                static_position: PhysicalOffset::new(sp_x, sp_y),
                containing_block_size: cb_size,
                containing_block_border: border.clone(),
                containing_block_direction: style.direction,
                static_position_direction: style.direction,
            };
            let captures = if child_style.position == openui_style::Position::Fixed {
                is_root
            } else {
                establishes_cb_for_abspos
            };
            if captures {
                oof_candidates.push(candidate);
            } else {
                bubbled_oof_candidates.push(candidate);
            }
        }
    }
    if !oof_candidates.is_empty() {
        let oof_fragments = crate::out_of_flow::layout_out_of_flow_children(doc, &oof_candidates);
        fragment.children.extend(oof_fragments);
    }
    fragment.oof_candidates = bubbled_oof_candidates;
    finalize_flex_fragment(&mut fragment, style, is_column);
    // Override baselines with values computed from the baseline alignment group.
    // flex_baseline_from_child only examines the first/last child and cannot
    // account for baseline-aligned items (which may be deeper in the list and
    // may produce a baseline that extends beyond the container's block size).
    // This matches Blink's BaselineAccumulator::AccumulateLine behavior where
    // first_major_baseline_ = line.cross_axis_offset + line.major_baseline.
    if computed_first_baseline.is_some() {
        fragment.first_baseline = computed_first_baseline;
    }
    if computed_last_baseline.is_some() {
        fragment.last_baseline = computed_last_baseline;
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
        let raw = resolve_length(
            &style.width,
            space.percentage_resolution_inline_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
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
            let min_raw = resolve_length(
                &style.min_width,
                pct_base,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
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
            let max_raw = resolve_length(
                &style.max_width,
                pct_base,
                LayoutUnit::zero(),
                LayoutUnit::from_i32(33554431),
            );
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
    container_inline_size: LayoutUnit,
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
        // CSS Sizing L4 §5.1: When a box has a preferred aspect ratio and
        // auto height but a definite width, the height is computed from the
        // width divided by the ratio.  For column flex this provides a
        // definite main-axis size enabling correct wrapping.
        if let Some(ar) = &style.aspect_ratio {
            if ar.ratio.1 > 0.0 && ar.ratio.0 > 0.0 {
                // container_inline_size is the border-box width of this flex container.
                // For aspect-ratio, border-box width maps to border-box height:
                //   border-box-height = border-box-width * (ratio.1 / ratio.0)
                // Then content-box height = border-box-height - border_padding_block
                if container_inline_size.raw() > 0 && !container_inline_size.is_indefinite() {
                    let ratio = ar.ratio.0 / ar.ratio.1;
                    let border_box_height =
                        LayoutUnit::from_f32(container_inline_size.to_f32() / ratio);
                    let height_from_ar =
                        (border_box_height - border_padding_block).clamp_negative_to_zero();

                    // Apply min-height / max-height clamping
                    let min_h = if !style.min_height.is_auto() {
                        let raw = resolve_length(
                            &style.min_height,
                            space.percentage_resolution_block_size,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        if style.box_sizing == openui_style::BoxSizing::BorderBox {
                            (raw - border_padding_block).clamp_negative_to_zero()
                        } else {
                            raw
                        }
                    } else {
                        LayoutUnit::zero()
                    };
                    let max_h = if !style.max_height.is_auto() && !style.max_height.is_none() {
                        let raw = resolve_length(
                            &style.max_height,
                            space.percentage_resolution_block_size,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        if style.box_sizing == openui_style::BoxSizing::BorderBox {
                            (raw - border_padding_block).clamp_negative_to_zero()
                        } else {
                            raw
                        }
                    } else {
                        LayoutUnit::from_raw(i32::MAX / 2)
                    };

                    return height_from_ar.max_of(min_h).min_of(max_h);
                }
            }
        }
        // CSS Flexbox §9.7: When the flex container has a definite max main
        // size (max-height for column), use it as the available space for the
        // flex algorithm so that flex-shrink can produce negative free space.
        // Blink: FlexLayoutAlgorithm::ComputeMainAxisAutoFallbackSize().
        if !style.max_height.is_auto() && !style.max_height.is_none() {
            let raw = resolve_length(
                &style.max_height,
                space.percentage_resolution_block_size,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            let content_max = if style.box_sizing == openui_style::BoxSizing::BorderBox {
                (raw - border_padding_block).clamp_negative_to_zero()
            } else {
                raw
            };
            if content_max.raw() > 0 {
                return content_max;
            }
        }
        // Auto height with no max-height → indefinite main axis.
        // Items stay at hypothetical sizes; container shrink-wraps.
        LayoutUnit::from_raw(-64) // INDEFINITE_SIZE sentinel
    } else {
        let raw = resolve_length(
            &style.height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
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
    container_inline_size: LayoutUnit,
) -> LayoutUnit {
    let resolved = if space.is_fixed_block_size {
        // Parent (e.g., column flex) has set a definite block size for this child
        space.available_block_size
    } else if space.stretch_block_size {
        // Parent flex is stretching this container on the cross axis
        space.available_block_size
    } else if style.height.is_auto() {
        // When aspect-ratio is set and width is definite, the block size is
        // determined from the ratio — even if content is smaller.
        if let Some(ar) = &style.aspect_ratio {
            if ar.ratio.0 > 0.0
                && ar.ratio.1 > 0.0
                && container_inline_size.raw() > 0
                && !container_inline_size.is_indefinite()
            {
                let ratio = ar.ratio.0 / ar.ratio.1;
                let ar_block = LayoutUnit::from_f32(container_inline_size.to_f32() / ratio);
                // Use the larger of AR-derived and intrinsic: AR provides the
                // definite size from which wrapping was computed, but content
                // might overflow (single-line or nowrap).
                ar_block.max_of(intrinsic_block_size)
            } else {
                intrinsic_block_size
            }
        } else {
            intrinsic_block_size
        }
    } else if style.height.is_content_or_intrinsic() {
        let sizes = compute_intrinsic_block_sizes(doc, node_id);
        match style.height.length_type() {
            LengthType::MinContent => sizes.min_content_block_size.max_of(border_padding_block),
            LengthType::MaxContent => sizes.max_content_block_size.max_of(border_padding_block),
            _ => intrinsic_block_size,
        }
    } else {
        let raw = resolve_length(
            &style.height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        if style.box_sizing == openui_style::BoxSizing::BorderBox {
            raw
        } else {
            raw + border_padding_block
        }
    };

    // Clamp min/max
    let pct_base = space.percentage_resolution_block_size;

    let min = if !style.min_height.is_auto()
        && (!pct_base.is_indefinite()
            || style.min_height.is_fixed()
            || style.min_height.is_content_or_intrinsic())
    {
        if style.min_height.is_content_or_intrinsic() {
            let sizes = compute_intrinsic_block_sizes(doc, node_id);
            match style.min_height.length_type() {
                LengthType::MinContent => sizes.min_content_block_size.max_of(border_padding_block),
                _ => sizes.max_content_block_size.max_of(border_padding_block),
            }
        } else {
            let min_raw = resolve_length(
                &style.min_height,
                pct_base,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            if style.box_sizing == openui_style::BoxSizing::BorderBox {
                min_raw
            } else {
                min_raw + border_padding_block
            }
        }
    } else {
        LayoutUnit::zero()
    };

    let max = if !style.max_height.is_none()
        && (!pct_base.is_indefinite()
            || style.max_height.is_fixed()
            || style.max_height.is_content_or_intrinsic())
    {
        if style.max_height.is_content_or_intrinsic() {
            let sizes = compute_intrinsic_block_sizes(doc, node_id);
            match style.max_height.length_type() {
                LengthType::MinContent => sizes.min_content_block_size.max_of(border_padding_block),
                LengthType::MaxContent | LengthType::FitContent => {
                    sizes.max_content_block_size.max_of(border_padding_block)
                }
                _ => sizes.max_content_block_size.max_of(border_padding_block),
            }
        } else {
            let max_raw = resolve_length(
                &style.max_height,
                pct_base,
                LayoutUnit::zero(),
                LayoutUnit::from_i32(33554431),
            );
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
        Some(length) => resolve_length(
            length,
            percentage_base,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        ),
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

    // Collect children with their order values, then stable sort.  display:contents
    // nodes do not generate flex item boxes; their box-generating children are
    // promoted into the flex container's child list.
    let mut children_with_order: Vec<(NodeId, i32)> = Vec::new();
    for child_id in flex_box_children(doc, container_id) {
        let child_style = &doc.node(child_id).style;
        children_with_order.push((child_id, child_style.order));
    }

    // Stable sort by order (Blink: FlexChildIterator)
    children_with_order.sort_by_key(|&(_, order)| order);

    let mut items = Vec::with_capacity(children_with_order.len());

    for (item_index, &(child_id, _)) in children_with_order.iter().enumerate() {
        let child_style = &doc.node(child_id).style;

        // CSS Flexbox §4.4: visibility:collapse items participate in layout
        // but with zero main size. They still contribute to line cross size.
        let is_collapsed = child_style.visibility == openui_style::Visibility::Collapse;

        // Read flex properties — CSS spec requires non-negative values.
        let flex_grow = child_style.flex_grow.max(0.0);
        let flex_shrink = child_style.flex_shrink.max(0.0);

        // Resolve alignment (Blink: ResolvedAlignSelf, line 261)
        let (alignment, alignment_overflow) = resolve_item_alignment(child_style, container_style);

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
            (if child_style.margin_top.is_auto() {
                1u8
            } else {
                0
            }) + (if child_style.margin_bottom.is_auto() {
                1
            } else {
                0
            })
        } else {
            (if child_style.margin_left.is_auto() {
                1u8
            } else {
                0
            }) + (if child_style.margin_right.is_auto() {
                1
            } else {
                0
            })
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
            alignment,
        );

        // Hypothetical = clamp base to min/max. Collapsed flex items keep their
        // flex base for line sizing/positioning, but their fragments are not painted.
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
            alignment_overflow,
            flexed_content_size: LayoutUnit::zero(),
            state: FlexerState::None,
            free_space_fraction: 0.0,
            is_used_flex_basis_indefinite,
            is_horizontal_flow,
            is_collapsed,
        });
    }

    items
}

fn flex_box_children(doc: &Document, parent_id: NodeId) -> Vec<NodeId> {
    let mut children = Vec::new();
    append_flex_box_children(doc, parent_id, &mut children);
    children
}

fn append_flex_box_children(doc: &Document, parent_id: NodeId, children: &mut Vec<NodeId>) {
    for child_id in doc.children(parent_id) {
        let child_style = &doc.node(child_id).style;
        if child_style.display == openui_style::Display::None {
            continue;
        }
        if child_style.display == openui_style::Display::Contents
            && !child_style.position.is_absolutely_positioned()
        {
            append_flex_box_children(doc, child_id, children);
            continue;
        }
        if child_style.position.is_absolutely_positioned() {
            continue;
        }
        children.push(child_id);
    }
}

fn flex_abspos_children(doc: &Document, parent_id: NodeId) -> Vec<NodeId> {
    let mut children = Vec::new();
    append_flex_abspos_children(doc, parent_id, &mut children);
    children
}

fn append_flex_abspos_children(doc: &Document, parent_id: NodeId, children: &mut Vec<NodeId>) {
    for child_id in doc.children(parent_id) {
        let child_style = &doc.node(child_id).style;
        if child_style.display == openui_style::Display::None {
            continue;
        }
        if child_style.position.is_absolutely_positioned() {
            children.push(child_id);
            continue;
        }
        if child_style.display == openui_style::Display::Contents {
            append_flex_abspos_children(doc, child_id, children);
        }
    }
}

/// Resolve the effective alignment for a flex item.
/// Blink: `ResolvedAlignSelf()` at flex_layout_algorithm.cc:261.
fn resolve_item_alignment(
    child_style: &openui_style::ComputedStyle,
    parent_style: &openui_style::ComputedStyle,
) -> (ItemPosition, OverflowAlignment) {
    let mut position = child_style.align_self.position;
    let mut overflow = child_style.align_self.overflow;

    // auto → inherit from parent's align-items
    if position == ItemPosition::Auto {
        position = parent_style.align_items.position;
        overflow = parent_style.align_items.overflow;
    }

    // normal → stretch in flex context
    if position == ItemPosition::Normal {
        position = ItemPosition::Stretch;
    }

    // CSS Box Alignment §4: Start/SelfStart and End/SelfEnd are NOT
    // equivalent to FlexStart/FlexEnd — they are axis-relative and must
    // NOT be flipped by flex-wrap:wrap-reverse.  Preserve them so that
    // resolve_align_self can handle them correctly.
    (position, overflow)
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
        // and the `content` keyword.
        if flex_basis.is_content_or_intrinsic() {
            // First try aspect-ratio transfer: if the item has a definite cross
            // size and an aspect ratio, derive the main size from it. This is
            // handled by resolve_content_based_size which already has full AR logic.
            let ar_size = resolve_content_based_size(
                doc,
                child_id,
                child_style,
                is_column,
                main_axis_border_padding,
                child_percentage_inline,
                child_percentage_block,
                space,
                resolved_alignment,
            );
            // resolve_content_based_size returns the AR-derived content size
            // when applicable, or falls back to layout-based sizing.
            // For min-content specifically, we need the min-content value.
            if flex_basis.length_type() == LengthType::MinContent {
                let contrib =
                    crate::intrinsic_sizing::compute_child_intrinsic_contribution(doc, child_id);
                let child_margin = resolve_margins(child_style, LayoutUnit::zero());
                let margin_main = if is_column {
                    child_margin.top + child_margin.bottom
                } else {
                    child_margin.left + child_margin.right
                };
                let intrinsic = if is_column {
                    contrib.min_content_block_size - margin_main
                } else {
                    contrib.min_content_inline_size - margin_main
                };
                let content = (intrinsic - main_axis_border_padding).clamp_negative_to_zero();
                // Use the larger of AR-derived and min-content
                return (content.max_of(ar_size), true);
            }
            return (ar_size, true);
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
        if !pct_base.is_indefinite()
            || flex_basis.is_fixed()
            || (flex_basis.is_percent() && flex_basis.value() == 0.0)
        {
            let resolved =
                resolve_length(flex_basis, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
            let content = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                (resolved - main_axis_border_padding).clamp_negative_to_zero()
            } else {
                // CSS Flexbox §4.2: negative flex-basis clamps to 0
                resolved.clamp_negative_to_zero()
            };
            return (content, false);
        }

        // Percentage with indefinite base → content-based
        return (
            resolve_content_based_size(
                doc,
                child_id,
                child_style,
                is_column,
                main_axis_border_padding,
                child_percentage_inline,
                child_percentage_block,
                space,
                resolved_alignment,
            ),
            true,
        );
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
            let resolved = resolve_length(
                main_length,
                pct_base,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            let content = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                (resolved - main_axis_border_padding).clamp_negative_to_zero()
            } else {
                resolved
            };
            return (content, false);
        }
    }

    // Step 3: Content-based sizing (max-content)
    (
        resolve_content_based_size(
            doc,
            child_id,
            child_style,
            is_column,
            main_axis_border_padding,
            child_percentage_inline,
            child_percentage_block,
            space,
            resolved_alignment,
        ),
        true,
    )
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
    _space: &ConstraintSpace,
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
                let mut cross_val = resolve_length(
                    cross_prop,
                    cross_pct,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
                // Clamp by min/max cross-axis constraints before AR transfer.
                let (min_cross, max_cross) = if is_column {
                    (&child_style.min_width, &child_style.max_width)
                } else {
                    (&child_style.min_height, &child_style.max_height)
                };
                if !max_cross.is_none() && !max_cross.is_auto() {
                    if !max_cross.is_percent() || !cross_pct.is_indefinite() {
                        let max_v = resolve_length(
                            max_cross,
                            cross_pct,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        if cross_val > max_v {
                            cross_val = max_v;
                        }
                    }
                }
                if !min_cross.is_auto() && !min_cross.is_none() {
                    if !min_cross.is_percent() || !cross_pct.is_indefinite() {
                        let min_v = resolve_length(
                            min_cross,
                            cross_pct,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        if cross_val < min_v {
                            cross_val = min_v;
                        }
                    }
                }
                // CSS Sizing 4 §5.1: a bare ratio respects box-sizing; `auto <ratio>`
                // transfers through the content box when no intrinsic ratio exists.
                let b = resolve_border(child_style);
                let p = resolve_padding(child_style, LayoutUnit::zero());
                let cross_bp = if is_column {
                    b.left + b.right + p.left + p.right
                } else {
                    b.top + b.bottom + p.top + p.bottom
                };
                let main_bp = if is_column {
                    b.top + b.bottom + p.top + p.bottom
                } else {
                    b.left + b.right + p.left + p.right
                };
                let box_sizing_for_ar = if ar.auto_flag {
                    openui_style::BoxSizing::ContentBox
                } else {
                    child_style.box_sizing
                };
                let main_val = if box_sizing_for_ar == openui_style::BoxSizing::BorderBox {
                    // AR on border-box: main_bb = cross_bb × ratio
                    let main_bb = if is_column {
                        LayoutUnit::from_f32(cross_val.to_f32() * ratio.1 / ratio.0)
                    } else {
                        LayoutUnit::from_f32(cross_val.to_f32() * ratio.0 / ratio.1)
                    };
                    (main_bb - main_bp).clamp_negative_to_zero()
                } else {
                    // AR on content-box: main = cross × ratio
                    let content_cross =
                        if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                            (cross_val - cross_bp).clamp_negative_to_zero()
                        } else {
                            cross_val
                        };
                    if is_column {
                        LayoutUnit::from_f32(content_cross.to_f32() * ratio.1 / ratio.0)
                    } else {
                        LayoutUnit::from_f32(content_cross.to_f32() * ratio.0 / ratio.1)
                    }
                };
                return main_val;
            }

            if cross_prop.is_auto() {
                let (min_cross, _) = if is_column {
                    (&child_style.min_width, &child_style.max_width)
                } else {
                    (&child_style.min_height, &child_style.max_height)
                };
                if !min_cross.is_auto() && !min_cross.is_none() {
                    if !min_cross.is_percent() || !cross_pct.is_indefinite() {
                        let cross_val = resolve_length(
                            min_cross,
                            cross_pct,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        let b = resolve_border(child_style);
                        let p = resolve_padding(child_style, LayoutUnit::zero());
                        let cross_bp = if is_column {
                            b.left + b.right + p.left + p.right
                        } else {
                            b.top + b.bottom + p.top + p.bottom
                        };
                        let box_sizing_for_ar = if ar.auto_flag {
                            openui_style::BoxSizing::ContentBox
                        } else {
                            child_style.box_sizing
                        };
                        let content_cross =
                            if box_sizing_for_ar == openui_style::BoxSizing::BorderBox {
                                cross_val
                            } else if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                                (cross_val - cross_bp).clamp_negative_to_zero()
                            } else {
                                cross_val
                            };
                        return if is_column {
                            LayoutUnit::from_f32(content_cross.to_f32() * ratio.1 / ratio.0)
                        } else {
                            LayoutUnit::from_f32(content_cross.to_f32() * ratio.0 / ratio.1)
                        };
                    }
                }
            }

            // CSS Flexbox §9.2 step 3(B): If cross-size is auto but the item
            // will stretch (align-self: stretch + definite container cross),
            // the cross-size is definite and equals the container cross minus margins.
            // NOTE: Must resolve align-self against parent's align-items, since
            // auto/normal inherit from the parent.
            if cross_prop.is_auto() {
                // Use the flex container's own resolved cross-axis content size,
                // not the parent's available size (which may be much larger).
                let cross_container = if is_column {
                    child_percentage_inline
                } else {
                    child_percentage_block
                };
                if !cross_container.is_indefinite() {
                    // CSS Flexbox §9.4: Cross-axis auto margins prevent stretching.
                    let has_cross_auto_margin = if is_column {
                        child_style.margin_left.is_auto() || child_style.margin_right.is_auto()
                    } else {
                        child_style.margin_top.is_auto() || child_style.margin_bottom.is_auto()
                    };
                    // Use the resolved alignment (which already accounts for
                    // parent's align-items when align-self is auto/normal).
                    let would_stretch =
                        resolved_alignment == ItemPosition::Stretch && !has_cross_auto_margin;
                    if would_stretch {
                        let margin = resolve_margins(child_style, LayoutUnit::zero());
                        let cross_margin = if is_column {
                            margin.left + margin.right
                        } else {
                            margin.top + margin.bottom
                        };
                        let b = resolve_border(child_style);
                        let p = resolve_padding(child_style, LayoutUnit::zero());
                        let cross_bp = if is_column {
                            b.left + b.right + p.left + p.right
                        } else {
                            b.top + b.bottom + p.top + p.bottom
                        };
                        let stretched_bb =
                            (cross_container - cross_margin).clamp_negative_to_zero();
                        // CSS Sizing 4 §5.1: a bare ratio respects box-sizing; `auto <ratio>`
                        // transfers through the content box when no intrinsic ratio exists.
                        let box_sizing_for_ar = if ar.auto_flag {
                            openui_style::BoxSizing::ContentBox
                        } else {
                            child_style.box_sizing
                        };
                        let main_val = if box_sizing_for_ar == openui_style::BoxSizing::BorderBox {
                            // AR on border-box: main_bb = cross_bb * ratio
                            let main_bb = if is_column {
                                LayoutUnit::from_f32(stretched_bb.to_f32() * ratio.1 / ratio.0)
                            } else {
                                LayoutUnit::from_f32(stretched_bb.to_f32() * ratio.0 / ratio.1)
                            };
                            let main_bp = if is_column {
                                b.top + b.bottom + p.top + p.bottom
                            } else {
                                b.left + b.right + p.left + p.right
                            };
                            (main_bb - main_bp).clamp_negative_to_zero()
                        } else {
                            let content_cross = (stretched_bb - cross_bp).clamp_negative_to_zero();
                            if is_column {
                                LayoutUnit::from_f32(content_cross.to_f32() * ratio.1 / ratio.0)
                            } else {
                                LayoutUnit::from_f32(content_cross.to_f32() * ratio.0 / ratio.1)
                            }
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
    // CSS Flexbox §9.4: Cross-axis auto margins prevent stretching.
    let has_cross_auto_margin_for_stretch = if is_column {
        child_style.margin_left.is_auto() || child_style.margin_right.is_auto()
    } else {
        child_style.margin_top.is_auto() || child_style.margin_bottom.is_auto()
    };
    let would_stretch_cross =
        resolved_alignment == ItemPosition::Stretch && !has_cross_auto_margin_for_stretch;
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
                &child_style.max_width,
                child_percentage_inline,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            max_w
        } else if child_style.width.is_auto() {
            let min_max = crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, child_id);
            if min_max.max > LayoutUnit::zero() {
                min_max.max
            } else {
                LayoutUnit::from_raw(-64)
            }
        } else {
            LayoutUnit::from_raw(-64)
        };
        let pct_inline_for_child = if cross_inline.is_indefinite() {
            child_percentage_inline
        } else {
            cross_inline
        };
        ConstraintSpace::for_block_child(
            cross_inline,
            LayoutUnit::from_raw(-64), // indefinite block (main axis)
            pct_inline_for_child,
            child_percentage_block,
            true,
        )
    } else {
        // Use the flex container's own cross-axis content size, not the parent's.
        let container_cross_block = child_percentage_block;
        let cross_block = if would_stretch_cross && !container_cross_block.is_indefinite() {
            container_cross_block
        } else if child_style.aspect_ratio.is_some() && child_style.height.is_auto() {
            LayoutUnit::from_raw(-64)
        } else if !container_cross_block.is_indefinite() {
            container_cross_block
        } else {
            LayoutUnit::from_raw(-64)
        };
        // Row flex: use indefinite inline size for max-content measurement
        ConstraintSpace::for_block_child(
            LayoutUnit::from_raw(-64), // indefinite → child gets intrinsic width
            cross_block,
            child_percentage_inline,
            child_percentage_block,
            true,
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
                    let derived_main =
                        LayoutUnit::from_f32(cross_size.to_f32() * ratio.1 / ratio.0);
                    return (derived_main - main_axis_border_padding).clamp_negative_to_zero();
                }
            }
        }

        // Lay out child with cross-axis constraint to get content-based height.
        // block_layout with indefinite block (auto height) computes auto height
        // from content, accounting for the actual cross-axis width constraint
        // which affects line breaking of inline children.
        let child_fragment = crate::block::block_layout(doc, child_id, &child_space);
        let main_size = child_fragment.height().clamp_indefinite_to_zero();
        return (main_size - main_axis_border_padding).clamp_negative_to_zero();
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
                let derived_main = LayoutUnit::from_f32(cross_size.to_f32() * ratio.0 / ratio.1);
                let cross_max_is_definite = !child_style.max_height.is_none()
                    && (!child_percentage_block.is_indefinite()
                        || child_style.max_height.is_fixed());
                if derived_main > main_size || cross_max_is_definite {
                    return (derived_main - main_axis_border_padding).clamp_negative_to_zero();
                }
            }
        }
    }

    // Convert from border-box to content-box
    (main_size - main_axis_border_padding).clamp_negative_to_zero()
}

/// CSS Flexbox §4.5: Compute the "transferred size suggestion" for a flex item.
///
/// If the item has an aspect ratio and its flex base size is content-based
/// (not definite), the transferred size suggestion is the size transferred
/// through its AR from its definite cross-axis constraint.
///
/// Returns `Some(content_box_main_size)` if a transferred suggestion exists.
fn compute_transferred_size_suggestion(
    child_style: &openui_style::ComputedStyle,
    is_column: bool,
    is_basis_from_content: bool,
    resolved_alignment: ItemPosition,
    pct_inline: LayoutUnit,
    pct_block: LayoutUnit,
    main_axis_border_padding: LayoutUnit,
) -> Option<LayoutUnit> {
    let ar = child_style.aspect_ratio.as_ref()?;
    if ar.ratio.0 <= 0.0 || ar.ratio.1 <= 0.0 {
        return None;
    }

    // Only applies when flex base size is not definite (content-based).
    if !is_basis_from_content {
        return None;
    }

    // Determine the definite cross-axis size.
    let (cross_prop, cross_pct) = if is_column {
        (&child_style.width, pct_inline)
    } else {
        (&child_style.height, pct_block)
    };

    let b = resolve_border(child_style);
    let p = resolve_padding(child_style, LayoutUnit::zero());
    let cross_bp = if is_column {
        b.left + b.right + p.left + p.right
    } else {
        b.top + b.bottom + p.top + p.bottom
    };
    let main_bp = if is_column {
        b.top + b.bottom + p.top + p.bottom
    } else {
        b.left + b.right + p.left + p.right
    };
    let is_border_box = child_style.box_sizing == openui_style::BoxSizing::BorderBox;
    let ar_uses_border_box = is_border_box && !ar.auto_flag;

    // Get the cross-axis border-box size (for AR transfer).
    let cross_bb = if !cross_prop.is_auto() && (!cross_pct.is_indefinite() || cross_prop.is_fixed())
    {
        // Explicit cross-axis property → use it.
        let resolved = resolve_length(
            cross_prop,
            cross_pct,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        if is_border_box {
            resolved
        } else {
            resolved + cross_bp
        }
    } else if cross_prop.is_auto() {
        // Check if the item would stretch to a definite cross size.
        let has_cross_auto_margin = if is_column {
            child_style.margin_left.is_auto() || child_style.margin_right.is_auto()
        } else {
            child_style.margin_top.is_auto() || child_style.margin_bottom.is_auto()
        };
        let would_stretch = resolved_alignment == ItemPosition::Stretch && !has_cross_auto_margin;
        let cross_container = if is_column { pct_inline } else { pct_block };
        if would_stretch && !cross_container.is_indefinite() {
            let margin = resolve_margins(child_style, LayoutUnit::zero());
            let cross_margin = if is_column {
                margin.left + margin.right
            } else {
                margin.top + margin.bottom
            };
            (cross_container - cross_margin).clamp_negative_to_zero()
        } else {
            return None;
        }
    } else {
        return None;
    };

    // CSS Sizing 4 §5.1: a bare ratio respects box-sizing; `auto <ratio>`
    // transfers through the content box when no intrinsic ratio exists.
    let transferred = if ar_uses_border_box {
        // AR on border-box: main_bb = cross_bb × ratio
        let main_bb = if is_column {
            LayoutUnit::from_f32(cross_bb.to_f32() * ar.ratio.1 / ar.ratio.0)
        } else {
            LayoutUnit::from_f32(cross_bb.to_f32() * ar.ratio.0 / ar.ratio.1)
        };
        (main_bb - main_bp).clamp_negative_to_zero()
    } else {
        // AR on content-box: convert to content, apply ratio
        let cross_content = (cross_bb - cross_bp).clamp_negative_to_zero();
        if is_column {
            LayoutUnit::from_f32(cross_content.to_f32() * ar.ratio.1 / ar.ratio.0)
        } else {
            LayoutUnit::from_f32(cross_content.to_f32() * ar.ratio.0 / ar.ratio.1)
        }
    };

    // Clamp by cross-axis min/max transferred through AR.
    let (cross_min_prop, cross_max_prop) = if is_column {
        (&child_style.min_width, &child_style.max_width)
    } else {
        (&child_style.min_height, &child_style.max_height)
    };
    let cross_min = if !cross_min_prop.is_auto()
        && !cross_min_prop.is_none()
        && (!cross_pct.is_indefinite() || cross_min_prop.is_fixed())
    {
        let r = resolve_length(
            cross_min_prop,
            cross_pct,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        let cross_min_bb = if is_border_box { r } else { r + cross_bp };
        if ar_uses_border_box {
            let main_min_bb = if is_column {
                LayoutUnit::from_f32(cross_min_bb.to_f32() * ar.ratio.1 / ar.ratio.0)
            } else {
                LayoutUnit::from_f32(cross_min_bb.to_f32() * ar.ratio.0 / ar.ratio.1)
            };
            (main_min_bb - main_bp).clamp_negative_to_zero()
        } else {
            let content_min = (cross_min_bb - cross_bp).clamp_negative_to_zero();
            if is_column {
                LayoutUnit::from_f32(content_min.to_f32() * ar.ratio.1 / ar.ratio.0)
            } else {
                LayoutUnit::from_f32(content_min.to_f32() * ar.ratio.0 / ar.ratio.1)
            }
        }
    } else {
        LayoutUnit::zero()
    };
    let cross_max =
        if !cross_max_prop.is_none() && (!cross_pct.is_indefinite() || cross_max_prop.is_fixed()) {
            let r = resolve_length(
                cross_max_prop,
                cross_pct,
                LayoutUnit::zero(),
                LayoutUnit::from_i32(33554431),
            );
            let cross_max_bb = if is_border_box { r } else { r + cross_bp };
            if ar_uses_border_box {
                let main_max_bb = if is_column {
                    LayoutUnit::from_f32(cross_max_bb.to_f32() * ar.ratio.1 / ar.ratio.0)
                } else {
                    LayoutUnit::from_f32(cross_max_bb.to_f32() * ar.ratio.0 / ar.ratio.1)
                };
                (main_max_bb - main_bp).clamp_negative_to_zero()
            } else {
                let content_max = (cross_max_bb - cross_bp).clamp_negative_to_zero();
                if is_column {
                    LayoutUnit::from_f32(content_max.to_f32() * ar.ratio.1 / ar.ratio.0)
                } else {
                    LayoutUnit::from_f32(content_max.to_f32() * ar.ratio.0 / ar.ratio.1)
                }
            }
        } else {
            LayoutUnit::from_i32(33554431)
        };

    // Also clamp by main-axis max if definite.
    let (main_max_prop, main_pct) = if is_column {
        (&child_style.max_height, pct_block)
    } else {
        (&child_style.max_width, pct_inline)
    };
    let main_max =
        if !main_max_prop.is_none() && (!main_pct.is_indefinite() || main_max_prop.is_fixed()) {
            let r = resolve_length(
                main_max_prop,
                main_pct,
                LayoutUnit::zero(),
                LayoutUnit::from_i32(33554431),
            );
            if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                (r - main_axis_border_padding).clamp_negative_to_zero()
            } else {
                r
            }
        } else {
            LayoutUnit::from_i32(33554431)
        };

    let result = transferred.clamp(cross_min, cross_max).min_of(main_max);
    Some(result)
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
    is_basis_from_content: bool,
    resolved_alignment: ItemPosition,
) -> MinMaxSizes {
    let (min_prop, max_prop, pct_base) = if is_column {
        (&child_style.min_height, &child_style.max_height, pct_block)
    } else {
        (&child_style.min_width, &child_style.max_width, pct_inline)
    };

    // ── Resolve min ──────────────────────────────────────────────────
    let min = if min_prop.is_auto() {
        // CSS Flexbox §4.5: Automatic Minimum Size
        // Scroll containers use an automatic minimum size of zero. Non-scrollable
        // overflow values (visible and clip) keep the content-based minimum.
        if child_style.is_scroll_container() {
            LayoutUnit::zero()
        } else {
            // CSS Flexbox §4.5: Content size suggestion is the min-content
            // size in the main axis. Always compute min-content intrinsic
            // size — base_content_size is max-content when flex-basis was
            // content-based, which is NOT the right value here.
            let content_size = if is_column {
                let intrinsic =
                    crate::intrinsic_sizing::compute_intrinsic_block_sizes(doc, child_id);
                let mut cs = (intrinsic.min_content_block_size - main_axis_border_padding)
                    .clamp_negative_to_zero();
                // CSS Flexbox §4.5: When the item has AR and a definite cross
                // size, the content size suggestion is clamped by min/max cross
                // sizes transferred through the AR. Also, if the raw min-content
                // is zero but the cross size is definite, transfer through AR.
                if !is_basis_from_content {
                    if let Some(ref ar) = child_style.aspect_ratio {
                        if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
                            let cross_prop = &child_style.width;
                            if !cross_prop.is_auto() && cross_prop.is_fixed() {
                                let mut cross_raw = resolve_length(
                                    cross_prop,
                                    pct_inline,
                                    LayoutUnit::zero(),
                                    LayoutUnit::zero(),
                                );
                                // Clamp by min/max cross constraints before AR transfer
                                if !child_style.max_width.is_none()
                                    && !child_style.max_width.is_auto()
                                {
                                    if child_style.max_width.is_fixed()
                                        || !pct_inline.is_indefinite()
                                    {
                                        let max_v = resolve_length(
                                            &child_style.max_width,
                                            pct_inline,
                                            LayoutUnit::zero(),
                                            LayoutUnit::zero(),
                                        );
                                        if cross_raw > max_v {
                                            cross_raw = max_v;
                                        }
                                    }
                                }
                                if !child_style.min_width.is_auto()
                                    && !child_style.min_width.is_none()
                                {
                                    if child_style.min_width.is_fixed()
                                        || !pct_inline.is_indefinite()
                                    {
                                        let min_v = resolve_length(
                                            &child_style.min_width,
                                            pct_inline,
                                            LayoutUnit::zero(),
                                            LayoutUnit::zero(),
                                        );
                                        if cross_raw < min_v {
                                            cross_raw = min_v;
                                        }
                                    }
                                }
                                let cross_bp = {
                                    let b = resolve_border(child_style);
                                    let p = resolve_padding(child_style, pct_inline);
                                    b.left + b.right + p.left + p.right
                                };
                                let content_cross = if child_style.box_sizing
                                    == openui_style::BoxSizing::BorderBox
                                {
                                    (cross_raw - cross_bp).clamp_negative_to_zero()
                                } else {
                                    cross_raw
                                };
                                let transferred = LayoutUnit::from_f32(
                                    content_cross.to_f32() * ar.ratio.1 / ar.ratio.0,
                                );
                                cs = cs.max_of(transferred);
                            }
                        }
                    }
                }
                if let Some(ref ar) = child_style.aspect_ratio {
                    if ar.ratio.0 != 0.0
                        && ar.ratio.1 != 0.0
                        && child_style.width.is_auto()
                        && !child_style.min_width.is_auto()
                        && !child_style.min_width.is_none()
                        && (child_style.min_width.is_fixed() || !pct_inline.is_indefinite())
                    {
                        let cross_raw = resolve_length(
                            &child_style.min_width,
                            pct_inline,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        let b = resolve_border(child_style);
                        let p = resolve_padding(child_style, pct_inline);
                        let cross_bp = b.left + b.right + p.left + p.right;
                        let transferred = if child_style.box_sizing
                            == openui_style::BoxSizing::BorderBox
                            && !ar.auto_flag
                        {
                            let main_bb =
                                LayoutUnit::from_f32(cross_raw.to_f32() * ar.ratio.1 / ar.ratio.0);
                            (main_bb - main_axis_border_padding).clamp_negative_to_zero()
                        } else {
                            let content_cross =
                                if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                                    (cross_raw - cross_bp).clamp_negative_to_zero()
                                } else {
                                    cross_raw
                                };
                            LayoutUnit::from_f32(content_cross.to_f32() * ar.ratio.1 / ar.ratio.0)
                        };
                        cs = cs.max_of(transferred);
                    }
                }
                cs
            } else {
                let min_max =
                    crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, child_id);
                let mut cs = (min_max.min - main_axis_border_padding).clamp_negative_to_zero();
                // Same AR transfer for row flex (cross = block)
                if !is_basis_from_content {
                    if let Some(ref ar) = child_style.aspect_ratio {
                        if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
                            let cross_prop = &child_style.height;
                            if !cross_prop.is_auto() && cross_prop.is_fixed() {
                                let mut cross_raw = resolve_length(
                                    cross_prop,
                                    pct_block,
                                    LayoutUnit::zero(),
                                    LayoutUnit::zero(),
                                );
                                // Clamp by min/max cross constraints before AR transfer
                                if !child_style.max_height.is_none()
                                    && !child_style.max_height.is_auto()
                                {
                                    if child_style.max_height.is_fixed()
                                        || !pct_block.is_indefinite()
                                    {
                                        let max_v = resolve_length(
                                            &child_style.max_height,
                                            pct_block,
                                            LayoutUnit::zero(),
                                            LayoutUnit::zero(),
                                        );
                                        if cross_raw > max_v {
                                            cross_raw = max_v;
                                        }
                                    }
                                }
                                if !child_style.min_height.is_auto()
                                    && !child_style.min_height.is_none()
                                {
                                    if child_style.min_height.is_fixed()
                                        || !pct_block.is_indefinite()
                                    {
                                        let min_v = resolve_length(
                                            &child_style.min_height,
                                            pct_block,
                                            LayoutUnit::zero(),
                                            LayoutUnit::zero(),
                                        );
                                        if cross_raw < min_v {
                                            cross_raw = min_v;
                                        }
                                    }
                                }
                                let cross_bp = {
                                    let b = resolve_border(child_style);
                                    let p = resolve_padding(child_style, pct_block);
                                    b.top + b.bottom + p.top + p.bottom
                                };
                                let content_cross = if child_style.box_sizing
                                    == openui_style::BoxSizing::BorderBox
                                {
                                    (cross_raw - cross_bp).clamp_negative_to_zero()
                                } else {
                                    cross_raw
                                };
                                let transferred = LayoutUnit::from_f32(
                                    content_cross.to_f32() * ar.ratio.0 / ar.ratio.1,
                                );
                                cs = cs.max_of(transferred);
                            }
                        }
                    }
                }
                if let Some(ref ar) = child_style.aspect_ratio {
                    if ar.ratio.0 != 0.0
                        && ar.ratio.1 != 0.0
                        && child_style.height.is_auto()
                        && !child_style.min_height.is_auto()
                        && !child_style.min_height.is_none()
                        && (child_style.min_height.is_fixed() || !pct_block.is_indefinite())
                    {
                        let cross_raw = resolve_length(
                            &child_style.min_height,
                            pct_block,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        let b = resolve_border(child_style);
                        let p = resolve_padding(child_style, pct_block);
                        let cross_bp = b.top + b.bottom + p.top + p.bottom;
                        let transferred = if child_style.box_sizing
                            == openui_style::BoxSizing::BorderBox
                            && !ar.auto_flag
                        {
                            let main_bb =
                                LayoutUnit::from_f32(cross_raw.to_f32() * ar.ratio.0 / ar.ratio.1);
                            (main_bb - main_axis_border_padding).clamp_negative_to_zero()
                        } else {
                            let content_cross =
                                if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                                    (cross_raw - cross_bp).clamp_negative_to_zero()
                                } else {
                                    cross_raw
                                };
                            LayoutUnit::from_f32(content_cross.to_f32() * ar.ratio.0 / ar.ratio.1)
                        };
                        cs = cs.max_of(transferred);
                    }
                }
                cs
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
                    main_size_prop,
                    pct_base,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
                let specified = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                    (resolved - main_axis_border_padding).clamp_negative_to_zero()
                } else {
                    resolved
                };
                content_size.min_of(specified)
            } else if let Some(transferred) = compute_transferred_size_suggestion(
                child_style,
                is_column,
                is_basis_from_content,
                resolved_alignment,
                pct_inline,
                pct_block,
                main_axis_border_padding,
            ) {
                // CSS Flexbox §4.5 (CSSWG resolution #6071):
                // When no specified suggestion exists but the item has AR and
                // a definite cross constraint, use the LARGER of the content
                // size suggestion and the transferred size suggestion.
                content_size.max_of(transferred)
            } else if is_basis_from_content {
                content_size
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
        let resolved = resolve_length(
            max_prop,
            pct_base,
            LayoutUnit::zero(),
            LayoutUnit::from_i32(33554431),
        );
        if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
            (resolved - main_axis_border_padding).clamp_negative_to_zero()
        } else {
            resolved
        }
    } else {
        LayoutUnit::from_i32(33554431)
    };

    let min = if min_prop.is_auto() {
        // CSS Flexbox §4.5: the content-based automatic minimum size is
        // clamped by any definite maximum main size.
        min.min_of(max)
    } else {
        min
    };

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
        (
            LayoutUnit::from_i32(33554431),
            LayoutUnit::from_i32(33554431),
        )
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
                doc,
                item,
                child_style,
                is_column,
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
            let cross_pct_base = if is_column {
                child_percentage_inline
            } else {
                child_percentage_block
            };
            let cross_min_raw = resolve_cross_min_max(
                doc,
                item.node_id,
                cross_min_prop,
                is_column,
                cross_pct_base,
                true,
            );
            let cross_max_raw = resolve_cross_min_max(
                doc,
                item.node_id,
                cross_max_prop,
                is_column,
                cross_pct_base,
                false,
            );
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

            let cross_margin_box = clamped_cross_bb + item.cross_axis_margin_extent();

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
        let (cross_min_prop, _) = if is_column {
            (&child_style.min_width, &child_style.max_width)
        } else {
            (&child_style.min_height, &child_style.max_height)
        };
        if !cross_min_prop.is_auto()
            && !cross_min_prop.is_none()
            && (!pct_base.is_indefinite() || cross_min_prop.is_fixed())
        {
            let resolved = resolve_length(
                cross_min_prop,
                pct_base,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            return if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                (resolved - cross_border_padding).clamp_negative_to_zero()
            } else {
                resolved
            };
        }

        // Check if aspect-ratio can derive cross-axis from the flexed main-axis size
        if let Some(ref ar) = child_style.aspect_ratio {
            let ratio = ar.ratio;
            if ratio.0 > 0.0 && ratio.1 > 0.0 {
                let main_size = item.flexed_border_box_size();
                if !main_size.is_indefinite() && main_size > LayoutUnit::zero() {
                    // CSS Sizing 4 §5.1: AR applies to content-box or border-box
                    // depending on box-sizing.
                    let cross_content =
                        if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                            // AR on border-box: cross_bb = main_bb × ratio
                            let cross_bb = if is_column {
                                LayoutUnit::from_f32(main_size.to_f32() * ratio.0 / ratio.1)
                            } else {
                                LayoutUnit::from_f32(main_size.to_f32() * ratio.1 / ratio.0)
                            };
                            (cross_bb - cross_border_padding).clamp_negative_to_zero()
                        } else {
                            // AR on content-box
                            let main_bp = if is_column {
                                let b = resolve_border(child_style);
                                let p = resolve_padding(child_style, child_percentage_inline);
                                b.block_sum() + p.block_sum()
                            } else {
                                let b = resolve_border(child_style);
                                let p = resolve_padding(child_style, child_percentage_inline);
                                b.inline_sum() + p.inline_sum()
                            };
                            let main_content = (main_size - main_bp).clamp_negative_to_zero();
                            if is_column {
                                LayoutUnit::from_f32(main_content.to_f32() * ratio.0 / ratio.1)
                            } else {
                                LayoutUnit::from_f32(main_content.to_f32() * ratio.1 / ratio.0)
                            }
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
            true,
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
            true,
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
    is_rtl: bool,
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
    space: &ConstraintSpace,
) -> (Vec<Fragment>, Option<LayoutUnit>, Option<LayoutUnit>) {
    let content_offset_x = border.left + padding.left;
    let content_offset_y = border.top + padding.top;

    // ── Resolve align-content (cross-axis line offsets) ──────────────
    // NOTE: Line stretching (align-content:stretch/normal) was already
    // performed before reversals in flex_layout(), so lines are already
    // at their final cross sizes here. Just compute remaining free space.
    let total_line_cross: LayoutUnit = lines
        .iter()
        .map(|l| l.line_cross_size)
        .fold(LayoutUnit::zero(), |acc, s| acc + s);

    let num_line_gaps = if lines.len() > 1 {
        lines.len() as i32 - 1
    } else {
        0
    };
    let total_line_gap = gap_between_lines * num_line_gaps;
    let cross_free_after = content_cross_size - total_line_cross - total_line_gap;

    let cross_align = resolve_content_alignment(
        align_content,
        cross_free_after,
        lines.len(),
        is_wrap_reverse, // wrap-reverse flips cross-axis alignment semantics
        !is_column,      // cross axis: row→block(vertical), column→inline(horizontal)
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
    // Two-pass approach per line:
    //   Pass 1: Layout all items, collect fragments and baseline data
    //   Pass 2: Compute line baseline, then position items
    let mut children = Vec::new();
    // Track the flex container's first/last baseline from the baseline
    // alignment group: content_offset_y + line.cross_axis_offset + line_max_ascent.
    // Only set for row flex (not column) and non-wrap-reverse, where cross
    // axis = block axis and "ascent" maps to the physical top.
    let mut container_first_baseline: Option<LayoutUnit> = None;
    let mut container_last_baseline: Option<LayoutUnit> = None;

    for line in lines.iter() {
        // Resolve justify-content for this line
        let effective_free = if line.main_axis_auto_margin_count > 0 {
            LayoutUnit::zero()
        } else {
            line.main_axis_free_space
        };

        // Count only non-collapsed items for justify-content distribution.
        // CSS Flexbox §4.4: collapsed items occupy zero main-axis space.
        let visible_item_count = line
            .item_indices
            .iter()
            .filter(|&&idx| !items[idx].is_collapsed)
            .count();

        let main_align = resolve_content_alignment(
            justify_content,
            effective_free,
            visible_item_count,
            is_reverse,
            is_column,
        );

        // ── Pass 1: Layout all items and collect data ────────────────
        struct ItemLayoutData {
            fragment: Fragment,
            idx: usize,
            cross_margin_box: LayoutUnit,
            has_cross_auto_margins: bool,
            is_start_auto: bool,
            is_end_auto: bool,
            baseline: Option<LayoutUnit>,
            cross_margin_start: LayoutUnit,
            is_baseline_aligned: bool,
        }

        let mut item_data: Vec<ItemLayoutData> = Vec::with_capacity(line.item_count());

        for &idx in line.item_indices.iter() {
            let item = &mut items[idx];
            let child_style = &doc.node(item.node_id).style;

            // ── Resolve main-axis auto margins ───────────────────────
            if item.main_axis_auto_margin_count > 0
                && line.main_axis_free_space > LayoutUnit::zero()
            {
                let is_start_auto = if is_column {
                    child_style.margin_top.is_auto()
                } else if is_rtl {
                    child_style.margin_right.is_auto()
                } else {
                    child_style.margin_left.is_auto()
                };
                let is_end_auto = if is_column {
                    child_style.margin_bottom.is_auto()
                } else if is_rtl {
                    child_style.margin_left.is_auto()
                } else {
                    child_style.margin_right.is_auto()
                };

                let per_margin_space = LayoutUnit::from_raw(
                    line.main_axis_free_space.raw() / line.main_axis_auto_margin_count as i32,
                );

                let (start_margin, end_margin) = resolve_main_auto_margins(
                    per_margin_space * item.main_axis_auto_margin_count as i32,
                    is_start_auto,
                    is_end_auto,
                );

                if is_column {
                    item.margin.top = start_margin;
                    item.margin.bottom = end_margin;
                } else if is_rtl {
                    item.margin.right = start_margin;
                    item.margin.left = end_margin;
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

            let cross_size_is_auto = if is_column {
                child_style.width.is_auto() || child_style.width.is_stretch()
            } else {
                child_style.height.is_auto() || child_style.height.is_stretch()
            };
            let has_cross_auto_margins_for_stretch = if is_column {
                child_style.margin_left.is_auto() || child_style.margin_right.is_auto()
            } else {
                child_style.margin_top.is_auto() || child_style.margin_bottom.is_auto()
            };
            let should_stretch = item.alignment == ItemPosition::Stretch
                && cross_size_is_auto
                && !has_cross_auto_margins_for_stretch;
            let cross_size_for_child = if should_stretch {
                let stretch_size = line.line_cross_size - item.cross_axis_margin_extent();
                let stretch_size = stretch_size.clamp_negative_to_zero();
                let (cross_min_prop, cross_max_prop) = if is_column {
                    (&child_style.min_width, &child_style.max_width)
                } else {
                    (&child_style.min_height, &child_style.max_height)
                };
                let cross_pct_base = if is_column {
                    child_percentage_inline
                } else {
                    child_percentage_block
                };
                let cross_min_raw = resolve_cross_min_max(
                    doc,
                    item.node_id,
                    cross_min_prop,
                    is_column,
                    cross_pct_base,
                    true,
                );
                let cross_max_raw = resolve_cross_min_max(
                    doc,
                    item.node_id,
                    cross_max_prop,
                    is_column,
                    cross_pct_base,
                    false,
                );
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
                let cross_content = resolve_cross_size(
                    doc,
                    item,
                    child_style,
                    is_column,
                    cross_border_padding,
                    child_percentage_inline,
                    child_percentage_block,
                );
                let natural_bb = cross_content + cross_border_padding;
                let (cross_min_prop, cross_max_prop) = if is_column {
                    (&child_style.min_width, &child_style.max_width)
                } else {
                    (&child_style.min_height, &child_style.max_height)
                };
                let cross_pct_base = if is_column {
                    child_percentage_inline
                } else {
                    child_percentage_block
                };
                let cross_min_raw = resolve_cross_min_max(
                    doc,
                    item.node_id,
                    cross_min_prop,
                    is_column,
                    cross_pct_base,
                    true,
                );
                let cross_max_raw = resolve_cross_min_max(
                    doc,
                    item.node_id,
                    cross_max_prop,
                    is_column,
                    cross_pct_base,
                    false,
                );
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

            let mut final_main = flexed_border_box;
            if item.flex_grow == 0.0 {
                if let Some(ar) = &child_style.aspect_ratio {
                    if ar.ratio.0 > 0.0 && ar.ratio.1 > 0.0 {
                        let (cross_max_prop, cross_pct_base) = if is_column {
                            (&child_style.max_width, child_percentage_inline)
                        } else {
                            (&child_style.max_height, child_percentage_block)
                        };
                        if !cross_max_prop.is_none()
                            && (!cross_pct_base.is_indefinite() || cross_max_prop.is_fixed())
                        {
                            let cross_max = resolve_length(
                                cross_max_prop,
                                cross_pct_base,
                                LayoutUnit::zero(),
                                LayoutUnit::from_i32(33554431),
                            );
                            let main_limit = if child_style.box_sizing
                                == openui_style::BoxSizing::BorderBox
                                && !ar.auto_flag
                            {
                                if is_column {
                                    LayoutUnit::from_f32(
                                        cross_max.to_f32() * ar.ratio.1 / ar.ratio.0,
                                    )
                                } else {
                                    LayoutUnit::from_f32(
                                        cross_max.to_f32() * ar.ratio.0 / ar.ratio.1,
                                    )
                                }
                            } else {
                                let content_cross = if child_style.box_sizing
                                    == openui_style::BoxSizing::BorderBox
                                {
                                    (cross_max - cross_border_padding).clamp_negative_to_zero()
                                } else {
                                    cross_max
                                };
                                let content_main = if is_column {
                                    LayoutUnit::from_f32(
                                        content_cross.to_f32() * ar.ratio.1 / ar.ratio.0,
                                    )
                                } else {
                                    LayoutUnit::from_f32(
                                        content_cross.to_f32() * ar.ratio.0 / ar.ratio.1,
                                    )
                                };
                                content_main + item.main_axis_border_padding
                            };
                            final_main = final_main.min_of(main_limit);
                        }
                    }
                }
            }

            let (inline_size, block_size) = if is_column {
                (cross_size_for_child, final_main)
            } else {
                (final_main, cross_size_for_child)
            };

            // CSS Flexbox §9.8: flex item sizes are definite for child
            // percentage resolution only when the flex container has a
            // definite main size. When the container has auto height
            // (even if max-height constrains it), percentage heights on
            // flex item children must resolve as auto.
            let item_pct_block = if is_column {
                if child_percentage_block.is_indefinite() && item.is_used_flex_basis_indefinite {
                    // CSS Flexbox §9.8: Container main size is not definite
                    // AND the item has no definite flex-basis → item height
                    // is not definite for percentage purposes.
                    child_percentage_block // indefinite
                } else {
                    // Either the container has a definite main size, OR the
                    // item has a definite flex-basis — treat the post-flexing
                    // main size as definite for percentage resolution (§9.8).
                    (final_main - item.main_axis_border_padding).clamp_negative_to_zero()
                }
            } else {
                if should_stretch || child_percentage_block.is_indefinite() {
                    let child_style = &doc.node(item.node_id).style;
                    let bp_cross = {
                        let bp = child_style.border_top_width as i32
                            + child_style.border_bottom_width as i32;
                        let pad_t = crate::length_resolver::resolve_margin_or_padding(
                            &child_style.padding_top,
                            child_percentage_inline,
                        );
                        let pad_b = crate::length_resolver::resolve_margin_or_padding(
                            &child_style.padding_bottom,
                            child_percentage_inline,
                        );
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
                // CSS Flexbox §9.8: flex item heights are only definite for
                // child percentage resolution when the container has a definite
                // main size OR the item has a definite flex-basis. Mark
                // indefinite only when BOTH container AND basis are indefinite.
                if child_percentage_block.is_indefinite() && item.is_used_flex_basis_indefinite {
                    child_space.is_initial_block_size_indefinite = true;
                }
                if should_stretch {
                    child_space.stretch_inline_size = true;
                }
            } else {
                child_space.is_fixed_inline_size = true;
                if should_stretch {
                    child_space.stretch_block_size = true;
                } else if !cross_size_is_auto {
                    // Row flex: item has explicit cross size (e.g. height + max-height).
                    // The flex algorithm already resolved the used cross size including
                    // min/max constraints. Mark as fixed so block.rs uses the constraint
                    // space value for child percentage resolution instead of re-resolving
                    // from style.height (which ignores max-height clamping).
                    child_space.is_fixed_block_size = true;
                }
            }

            let child_fragment = crate::block::block_layout(doc, item.node_id, &child_space);

            let item_cross_margin_box = if is_column {
                child_fragment.width() + item.cross_axis_margin_extent()
            } else {
                child_fragment.height() + item.cross_axis_margin_extent()
            };

            let has_cross_auto_margins = if is_column {
                child_style.margin_left.is_auto() || child_style.margin_right.is_auto()
            } else {
                child_style.margin_top.is_auto() || child_style.margin_bottom.is_auto()
            };

            let is_start_auto = if is_column {
                if is_rtl {
                    child_style.margin_right.is_auto()
                } else {
                    child_style.margin_left.is_auto()
                }
            } else {
                child_style.margin_top.is_auto()
            };
            let is_end_auto = if is_column {
                if is_rtl {
                    child_style.margin_left.is_auto()
                } else {
                    child_style.margin_right.is_auto()
                }
            } else {
                child_style.margin_bottom.is_auto()
            };

            let cross_margin_start = if is_column {
                if is_rtl {
                    item.margin.right
                } else {
                    item.margin.left
                }
            } else {
                item.margin.top
            };

            // Baseline alignment: for row flex, baseline = first_baseline of
            // the child fragment (distance from cross-start to baseline).
            // If no baseline from content, synthesize from the cross-end
            // border edge (CSS Flexbox §9.4).
            let is_baseline_aligned = matches!(
                item.alignment,
                ItemPosition::Baseline | ItemPosition::LastBaseline
            ) && !has_cross_auto_margins;

            let baseline = if is_baseline_aligned {
                let frag_baseline = if item.alignment == ItemPosition::LastBaseline {
                    child_fragment
                        .last_baseline
                        .or(child_fragment.first_baseline)
                } else {
                    child_fragment.first_baseline
                };
                // Ascent includes the cross-start margin so baselines align
                // across items with different margins.
                let cross_size = if is_column {
                    child_fragment.width()
                } else {
                    child_fragment.height()
                };
                let baseline_from_top = cross_margin_start + frag_baseline.unwrap_or(cross_size);
                // For wrap-reverse the cross axis runs bottom-to-top, so
                // "ascent" is measured from the physical bottom (= cross-start).
                Some(if is_wrap_reverse {
                    item_cross_margin_box - baseline_from_top
                } else {
                    baseline_from_top
                })
            } else {
                None
            };

            item_data.push(ItemLayoutData {
                fragment: child_fragment,
                idx,
                cross_margin_box: item_cross_margin_box,
                has_cross_auto_margins,
                is_start_auto,
                is_end_auto,
                baseline,
                cross_margin_start,
                is_baseline_aligned,
            });
        }

        // ── Compute line baseline (max ascent among baseline-aligned items) ──
        // Do NOT clamp to zero: for wrap-reverse items negative ascents are
        // valid (baseline extends beyond the cross-end margin edge), and
        // clamping to zero would offset items incorrectly.
        let line_max_ascent = item_data
            .iter()
            .filter_map(|d| d.baseline)
            .max()
            .unwrap_or(LayoutUnit::zero());

        // For row (non-column), non-wrap-reverse flex: export the container's
        // baseline from this line's alignment group.  The baseline is
        // content_offset_y + line.cross_axis_offset + line_max_ascent —
        // the physical distance from the container's border-box top to the
        // shared baseline of all baseline-aligned items in this line.
        // This mirrors Blink's BaselineAccumulator::AccumulateLine which sets
        // first_major_baseline_ = line.cross_axis_offset + line.major_baseline.
        let has_baseline_items = item_data.iter().any(|d| d.is_baseline_aligned);
        if has_baseline_items && !is_column && !is_wrap_reverse {
            let line_baseline = content_offset_y + line.cross_axis_offset + line_max_ascent;
            if container_first_baseline.is_none() {
                container_first_baseline = Some(line_baseline);
            }
            container_last_baseline = Some(line_baseline);
        }

        // ── Pass 2: Position items ───────────────────────────────────
        let mut main_offset = main_align.initial_offset;

        for (item_pos, data) in item_data.into_iter().enumerate() {
            let item = &items[data.idx];
            let child_style = &doc.node(item.node_id).style;
            let cross_space = line.line_cross_size - data.cross_margin_box;

            let cross_item_offset = if data.has_cross_auto_margins {
                let (start, _end) =
                    resolve_cross_auto_margins(cross_space, data.is_start_auto, data.is_end_auto);
                start
            } else if data.is_baseline_aligned {
                // Offset aligns all baselines on the same cross-axis line.
                // For wrap-reverse the cross axis is flipped: cross_item_offset
                // counts from physical top, so items near cross-start (physical
                // bottom) need offset ≈ cross_space.
                let item_ascent = data.baseline.unwrap_or(LayoutUnit::zero());
                if is_wrap_reverse {
                    cross_space - (line_max_ascent - item_ascent)
                } else {
                    line_max_ascent - item_ascent
                }
            } else {
                let alignment_cross_space = if item.alignment_overflow == OverflowAlignment::Safe {
                    let container_cross_space = content_cross_size - data.cross_margin_box;
                    cross_space.min_of(container_cross_space)
                } else {
                    cross_space
                };
                resolve_align_self(
                    item.alignment,
                    alignment_cross_space,
                    item.alignment_overflow,
                    is_wrap_reverse,
                )
            };

            // For RTL row flex, physical right margin is the main-start margin.
            let main_margin_start = if is_column {
                item.margin.top
            } else if is_rtl {
                item.margin.right
            } else {
                item.margin.left
            };
            let cross_margin_start = data.cross_margin_start;

            let item_main_pos = main_offset + main_margin_start;
            let item_cross_pos = line.cross_axis_offset + cross_item_offset + cross_margin_start;

            let (x, y) = if is_column {
                let physical_cross_pos = if is_rtl {
                    let item_cross_size = data.fragment.width();
                    content_cross_size - item_cross_pos - item_cross_size
                } else {
                    item_cross_pos
                };
                (
                    content_offset_x + physical_cross_pos,
                    content_offset_y + item_main_pos,
                )
            } else {
                (
                    content_offset_x + item_main_pos,
                    content_offset_y + item_cross_pos,
                )
            };

            let mut positioned = data.fragment;
            positioned.offset = PhysicalOffset::new(x, y);
            positioned.margin = item.margin.clone();

            crate::relative::apply_relative_offset(
                &mut positioned,
                child_style,
                child_percentage_inline,
                child_percentage_block,
            );

            let main_margin_end = if is_column {
                item.margin.bottom
            } else if is_rtl {
                item.margin.left
            } else {
                item.margin.right
            };
            let item_main_size = if is_column {
                positioned.height()
            } else {
                positioned.width()
            };

            // Blink keeps visibility:collapse flex items in main-axis layout
            // while suppressing their fragment from painting; their cross-axis
            // strut is already accounted for by line cross-size computation.
            if !item.is_collapsed {
                children.push(positioned);
            }

            main_offset = main_offset + main_margin_start + item_main_size + main_margin_end;

            if item_pos < line.item_count() - 1 {
                main_offset = main_offset + gap_between_items + main_align.between_space;
            }
        }
    }

    (children, container_first_baseline, container_last_baseline)
}

fn finalize_flex_fragment(
    fragment: &mut Fragment,
    style: &openui_style::ComputedStyle,
    is_column: bool,
) {
    let border_box_rect = PhysicalRect::new(PhysicalOffset::zero(), fragment.size);
    let mut overflow = border_box_rect;
    for child in &fragment.children {
        let child_rect = PhysicalRect::new(child.offset, child.size);
        overflow = overflow.unite(&child_rect);
        if !child.has_overflow_clip {
            if let Some(child_overflow) = child.overflow_rect {
                let shifted = PhysicalRect::new(
                    PhysicalOffset::new(
                        child.offset.left + child_overflow.offset.left,
                        child.offset.top + child_overflow.offset.top,
                    ),
                    child_overflow.size,
                );
                overflow = overflow.unite(&shifted);
            }
        }
    }
    if overflow != border_box_rect {
        fragment.overflow_rect = Some(overflow);
    }
    fragment.has_overflow_clip = style.overflow_x != openui_style::Overflow::Visible
        || style.overflow_y != openui_style::Overflow::Visible;

    fragment.first_baseline = flex_baseline_from_child(fragment, style, is_column, true);
    fragment.last_baseline = flex_baseline_from_child(fragment, style, is_column, false);
}

fn flex_baseline_from_child(
    fragment: &Fragment,
    style: &openui_style::ComputedStyle,
    is_column: bool,
    first: bool,
) -> Option<LayoutUnit> {
    let child = match if first {
        fragment.children.first()
    } else {
        fragment.children.last()
    } {
        Some(child) => child,
        None => {
            return if is_column {
                Some(fragment.height())
            } else {
                None
            };
        }
    };
    let child_baseline = if first {
        child.first_baseline.or(child.last_baseline)
    } else {
        child.last_baseline.or(child.first_baseline)
    };
    if is_column {
        child_baseline
            .map(|baseline| child.offset.top + baseline)
            .or_else(|| {
                if child.height() == LayoutUnit::zero() {
                    Some(child.offset.top)
                } else {
                    None
                }
            })
    } else {
        child_baseline
            .map(|baseline| child.offset.top + baseline)
            .or_else(|| {
                if style.flex_wrap.is_wrap() {
                    Some(child.offset.top + child.height())
                } else {
                    None
                }
            })
    }
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
    use openui_style::{ContentDistribution, ContentPosition, ItemPosition};

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

    (
        border.left + padding.left + x_off,
        border.top + padding.top + y_off,
    )
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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(300), LayoutUnit::from_i32(100));

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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(300), LayoutUnit::from_i32(100));

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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(400), LayoutUnit::from_i32(100));

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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(400), LayoutUnit::from_i32(100));

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
            s.justify_content =
                ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
        }
        doc.append_child(doc.root(), container);

        let _c1 = add_flex_child(&mut doc, container, 50, 50);
        let _c2 = add_flex_child(&mut doc, container, 50, 50);
        let _c3 = add_flex_child(&mut doc, container, 50, 50);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(400), LayoutUnit::from_i32(100));

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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(200), LayoutUnit::from_i32(300));

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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(200), LayoutUnit::from_i32(300));

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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(400), LayoutUnit::from_i32(100));

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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(300), LayoutUnit::from_i32(100));

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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(300), LayoutUnit::from_i32(100));

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

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(300), LayoutUnit::from_i32(100));

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
