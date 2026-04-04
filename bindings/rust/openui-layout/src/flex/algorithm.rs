//! Flex layout algorithm — the main entry point.
//!
//! Extracted from Blink's `FlexLayoutAlgorithm::LayoutInternal()` and
//! `PlaceFlexItems()` (flex_layout_algorithm.cc:1229, 1394).
//!
//! Orchestrates: item collection → line breaking → flexing → alignment → positioning.

use openui_dom::{Document, NodeId};
use openui_geometry::{BoxStrut, LayoutUnit, MinMaxSizes, PhysicalOffset, PhysicalSize};
use openui_style::{
    ContentAlignment, ContentDistribution, ContentPosition,
    ItemPosition,
};
use openui_geometry::Length;

use crate::block::{resolve_border, resolve_padding, resolve_margins};
use crate::constraint_space::ConstraintSpace;
use crate::fragment::Fragment;
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
        style, space, border_padding_inline,
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
        resolve_container_block_size_for_flex(style, space, border_padding_block)
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
        // Container has explicit height → use it as percentage base
        let raw = resolve_length(&style.height, space.percentage_resolution_block_size, LayoutUnit::zero(), LayoutUnit::zero());
        let content = if style.box_sizing == openui_style::BoxSizing::BorderBox {
            (raw - border_padding_block).clamp_negative_to_zero()
        } else {
            raw
        };
        content
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
            // Use the container's own resolved content-box height when explicit,
            // or available_block_size - bp when auto height with definite available.
            if !style.height.is_auto() {
                // Explicit height — use resolved content-box value
                let raw = resolve_length(&style.height, space.percentage_resolution_block_size, LayoutUnit::zero(), LayoutUnit::zero());
                if style.box_sizing == openui_style::BoxSizing::BorderBox {
                    (raw - border_padding_block).clamp_negative_to_zero()
                } else {
                    raw
                }
            } else if !space.available_block_size.is_indefinite() {
                space.available_block_size - border_padding_block
            } else {
                flex_lines[0].line_cross_size // keep computed size
            }
        };
        flex_lines[0].line_cross_size = container_cross;
    }

    // ── Compute total block size ─────────────────────────────────────
    let intrinsic_block_size = compute_intrinsic_block_size(
        &flex_lines, is_column, gap_between_lines, border_padding_block,
    );

    let total_block_size = resolve_total_block_size(
        style, space, intrinsic_block_size, border_padding_block,
    );

    // ── Step 6: Apply reversals (Blink line 1265) ────────────────────
    if is_wrap_reverse {
        flex_lines.reverse();
    }
    if is_reverse {
        for line in &mut flex_lines {
            line.item_indices.reverse();
        }
    }

    // ── Step 7: Final positioning (Blink line 1271) ──────────────────
    let content_cross_size = if is_column {
        content_inline_size
    } else {
        total_block_size - border_padding_block
    };

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
    let mut fragment = Fragment::new_box(
        node_id,
        PhysicalSize::new(container_inline_size, total_block_size),
    );
    fragment.padding = padding;
    fragment.border = border;
    fragment.children = children;
    fragment
}

/// Resolve the container's inline size (width for horizontal writing mode).
fn resolve_container_inline_size(
    style: &openui_style::ComputedStyle,
    space: &ConstraintSpace,
    border_padding_inline: LayoutUnit,
) -> LayoutUnit {
    let resolved = if style.width.is_auto() {
        space.available_inline_size
    } else {
        let raw = resolve_length(&style.width, space.percentage_resolution_inline_size, LayoutUnit::zero(), LayoutUnit::zero());
        if style.box_sizing == openui_style::BoxSizing::BorderBox {
            raw
        } else {
            raw + border_padding_inline
        }
    };

    // Clamp to min/max
    clamp_inline_size(style, space, resolved, border_padding_inline)
}

/// Clamp inline size to min-width/max-width.
fn clamp_inline_size(
    style: &openui_style::ComputedStyle,
    space: &ConstraintSpace,
    size: LayoutUnit,
    border_padding_inline: LayoutUnit,
) -> LayoutUnit {
    let pct_base = space.percentage_resolution_inline_size;

    let min = if !style.min_width.is_auto() {
        let min_raw = resolve_length(&style.min_width, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
        if style.box_sizing == openui_style::BoxSizing::BorderBox {
            min_raw
        } else {
            min_raw + border_padding_inline
        }
    } else {
        LayoutUnit::zero()
    };

    let max = if !style.max_width.is_none() {
        let max_raw = resolve_length(&style.max_width, pct_base, LayoutUnit::zero(), LayoutUnit::from_i32(33554431));
        if style.box_sizing == openui_style::BoxSizing::BorderBox {
            max_raw
        } else {
            max_raw + border_padding_inline
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
    if style.height.is_auto() {
        if !space.available_block_size.is_indefinite() {
            space.available_block_size - border_padding_block
        } else {
            // Indefinite main axis for column flex — signal with INDEFINITE
            LayoutUnit::from_raw(-64) // INDEFINITE_SIZE sentinel
        }
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
    style: &openui_style::ComputedStyle,
    space: &ConstraintSpace,
    intrinsic_block_size: LayoutUnit,
    border_padding_block: LayoutUnit,
) -> LayoutUnit {
    let resolved = if style.height.is_auto() {
        intrinsic_block_size
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

    let min = if !style.min_height.is_auto() && !pct_base.is_indefinite() {
        let min_raw = resolve_length(&style.min_height, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
        if style.box_sizing == openui_style::BoxSizing::BorderBox {
            min_raw
        } else {
            min_raw + border_padding_block
        }
    } else {
        LayoutUnit::zero()
    };

    let max = if !style.max_height.is_none() && !pct_base.is_indefinite() {
        let max_raw = resolve_length(&style.max_height, pct_base, LayoutUnit::zero(), LayoutUnit::from_i32(33554431));
        if style.box_sizing == openui_style::BoxSizing::BorderBox {
            max_raw
        } else {
            max_raw + border_padding_block
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

        // Read flex properties
        let flex_grow = child_style.flex_grow;
        let flex_shrink = child_style.flex_shrink;

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
        );

        // ── Resolve min/max on main axis (Blink lines 1145-1157) ─────
        let main_axis_min_max = resolve_main_axis_min_max(
            child_style,
            is_column,
            main_axis_border_padding,
            child_percentage_inline,
            child_percentage_block,
            base_content_size,
            flex_shrink,
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
    _main_axis_inner_size: LayoutUnit,
    space: &ConstraintSpace,
) -> (LayoutUnit, bool) {
    let flex_basis = &child_style.flex_basis;

    // Step 1: If flex-basis is not auto, try to resolve it
    if !flex_basis.is_auto() {
        let pct_base = if is_column {
            child_percentage_block
        } else {
            child_percentage_inline
        };

        if !pct_base.is_indefinite() || flex_basis.is_fixed() {
            let resolved = resolve_length(flex_basis, pct_base, LayoutUnit::zero(), LayoutUnit::zero());
            let content = if child_style.box_sizing == openui_style::BoxSizing::BorderBox {
                (resolved - main_axis_border_padding).clamp_negative_to_zero()
            } else {
                resolved
            };
            return (content, false);
        }

        // Percentage with indefinite base → content-based
        return (resolve_content_based_size(
            doc, child_id, child_style, is_column,
            main_axis_border_padding,
            child_percentage_inline, child_percentage_block, space,
        ), true);
    }

    // Step 2: flex-basis: auto → use width/height in main axis direction
    let main_length = if is_column {
        &child_style.height
    } else {
        &child_style.width
    };

    if !main_length.is_auto() {
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
    ), true)
}

/// Resolve content-based (intrinsic) size for a flex item.
/// This runs a child layout to determine the item's natural size.
fn resolve_content_based_size(
    doc: &Document,
    child_id: NodeId,
    child_style: &openui_style::ComputedStyle,
    is_column: bool,
    main_axis_border_padding: LayoutUnit,
    child_percentage_inline: LayoutUnit,
    child_percentage_block: LayoutUnit,
    space: &ConstraintSpace,
) -> LayoutUnit {
    // For content-based sizing, lay out the child with unconstrained main axis
    let child_space = if is_column {
        ConstraintSpace::for_block_child(
            child_percentage_inline,
            LayoutUnit::from_raw(-64), // indefinite block
            child_percentage_inline,
            child_percentage_block,
            child_style.creates_new_formatting_context(),
        )
    } else {
        // Row flex: use indefinite inline size for max-content measurement
        ConstraintSpace::for_block_child(
            LayoutUnit::from_raw(-64), // indefinite → child gets intrinsic width
            space.available_block_size,
            child_percentage_inline,
            child_percentage_block,
            child_style.creates_new_formatting_context(),
        )
    };

    let child_fragment = crate::block::block_layout(doc, child_id, &child_space);

    let main_size = if is_column {
        child_fragment.height()
    } else {
        child_fragment.width()
    };

    // If block_layout returned indefinite (empty element with unconstrained axis),
    // treat as zero content size.
    let main_size = main_size.clamp_indefinite_to_zero();

    // Convert from border-box to content-box
    (main_size - main_axis_border_padding).clamp_negative_to_zero()
}

/// Resolve min/max constraints on the main axis.
/// Blink: lines 1034-1157.
fn resolve_main_axis_min_max(
    child_style: &openui_style::ComputedStyle,
    is_column: bool,
    main_axis_border_padding: LayoutUnit,
    pct_inline: LayoutUnit,
    pct_block: LayoutUnit,
    base_content_size: LayoutUnit,
    flex_shrink: f32,
) -> MinMaxSizes {
    let (min_prop, max_prop, pct_base) = if is_column {
        (&child_style.min_height, &child_style.max_height, pct_block)
    } else {
        (&child_style.min_width, &child_style.max_width, pct_inline)
    };

    // ── Resolve min ──────────────────────────────────────────────────
    let min = if min_prop.is_auto() {
        // Auto minimum size (CSS Flexbox §4.5)
        // Approximation: base_content_size if item can't shrink (flex_shrink == 0),
        // else 0. Full implementation needs min-content sizing (content_suggestion
        // vs specified_suggestion).
        if flex_shrink == 0.0 {
            base_content_size
        } else {
            LayoutUnit::zero()
        }
    } else if min_prop.is_none() || *min_prop == Length::zero() {
        LayoutUnit::zero()
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

    MinMaxSizes::new(min, max)
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

            let cross_margin_box = cross_content_size
                + cross_border_padding
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
    } else {
        // Auto cross size → lay out child to get intrinsic size
        let child_space = ConstraintSpace::for_block_child(
            if is_column {
                // Column flex: cross is inline, use flexed size as available inline
                item.flexed_border_box_size()
            } else {
                // Row flex: use item's flexed width as available inline
                // so child content (e.g. text) wraps at the correct width
                item.flexed_border_box_size()
            },
            if is_column {
                LayoutUnit::from_raw(-64) // indefinite
            } else {
                // Row flex: cross is block, indefinite since we're measuring height
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
    let total_line_cross: LayoutUnit = lines.iter()
        .map(|l| l.line_cross_size)
        .fold(LayoutUnit::zero(), |acc, s| acc + s);

    let num_line_gaps = if lines.len() > 1 { lines.len() as i32 - 1 } else { 0 };
    let total_line_gap = gap_between_lines * num_line_gaps;
    let cross_free_space = content_cross_size - total_line_cross - total_line_gap;

    // Stretch lines if align-content: stretch or normal (CSS Flexbox §9.4)
    // In flex context, align-content: normal behaves like stretch for multi-line.
    let should_stretch_lines = align_content.distribution == ContentDistribution::Stretch
        || (align_content.distribution == ContentDistribution::Default
            && align_content.position == ContentPosition::Normal);
    if should_stretch_lines && cross_free_space > LayoutUnit::zero() && lines.len() > 0 {
        let extra_per_line = LayoutUnit::from_raw(cross_free_space.raw() / lines.len() as i32);
        for line in lines.iter_mut() {
            line.line_cross_size = line.line_cross_size + extra_per_line;
        }
    }

    // Recalculate cross free space after stretch
    let total_line_cross_after: LayoutUnit = lines.iter()
        .map(|l| l.line_cross_size)
        .fold(LayoutUnit::zero(), |acc, s| acc + s);
    let cross_free_after = content_cross_size - total_line_cross_after - total_line_gap;

    let cross_align = resolve_content_alignment(
        align_content,
        cross_free_after,
        lines.len(),
        is_wrap_reverse, // wrap-reverse flips cross-axis alignment semantics
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
                let (cross_min_prop, cross_max_prop) = if is_column {
                    (&child_style.min_width, &child_style.max_width)
                } else {
                    (&child_style.min_height, &child_style.max_height)
                };
                let cross_pct_base = if is_column { child_percentage_inline } else { child_percentage_block };
                let cross_min = if cross_min_prop.is_auto() {
                    LayoutUnit::zero()
                } else {
                    resolve_length(cross_min_prop, cross_pct_base, LayoutUnit::zero(), LayoutUnit::zero())
                };
                let cross_max = if cross_max_prop.is_none() {
                    LayoutUnit::from_i32(33554431)
                } else {
                    resolve_length(cross_max_prop, cross_pct_base, LayoutUnit::from_i32(33554431), LayoutUnit::from_i32(33554431))
                };
                stretch_size.clamp(cross_min, cross_max)
            } else {
                // Use child's natural cross size
                let cross_content = resolve_cross_size(
                    doc, item, child_style, is_column,
                    cross_border_padding,
                    child_percentage_inline, child_percentage_block,
                );
                cross_content + cross_border_padding
            };

            // Build final constraint space
            let (inline_size, block_size) = if is_column {
                (cross_size_for_child, flexed_border_box)
            } else {
                (flexed_border_box, cross_size_for_child)
            };

            let mut child_space = ConstraintSpace::for_flex_child(
                inline_size,
                block_size,
                child_percentage_inline,
                child_percentage_block,
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
