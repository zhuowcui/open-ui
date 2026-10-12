//! HTML fieldset wrapper and its anonymous formatting/scroll content box.
//!
//! Chromium147 LayoutFieldset delegates padding, display, columns and overflow
//! to a real anonymous child. The rendered legend is its sibling. This module
//! runs the ordinary layout algorithms over a borrowed layout-box context;
//! the native document, semantic identities and authored effect owner remain
//! unchanged.
use crate::layout_context::{FieldsetContentContext, LayoutDocument};
use crate::{ConstraintSpace, Fragment};
use openui_dom::NodeId;
use openui_geometry::{
    LayoutUnit, LogicalOffset, LogicalSize, PhysicalOffset, PhysicalRect, PhysicalSize,
    WritingModeConverter, INDEFINITE_SIZE,
};
use openui_style::{BoxSizing, ComputedStyle, Display, Position, TextAlign};

pub(crate) fn content_style(style: &ComputedStyle) -> ComputedStyle {
    ComputedStyle::for_anonymous_box(style).derive(|content| {
        content.display = if style.display.is_flex() {
            Display::Flex
        } else if style.display.is_grid() {
            Display::Grid
        } else {
            Display::FlowRoot
        };
        content.align_content = style.align_content;
        content.align_items = style.align_items;
        content.justify_content = style.justify_content;
        content.justify_items = style.justify_items;
        content.padding_top = style.padding_top.clone();
        content.padding_right = style.padding_right.clone();
        content.padding_bottom = style.padding_bottom.clone();
        content.padding_left = style.padding_left.clone();
        content.border_top_left_radius = style.border_top_left_radius;
        content.border_top_right_radius = style.border_top_right_radius;
        content.border_bottom_left_radius = style.border_bottom_left_radius;
        content.border_bottom_right_radius = style.border_bottom_right_radius;
        content.border_radius_percent = style.border_radius_percent;
        content.box_decoration_break = style.box_decoration_break;
        content.column_count = style.column_count;
        content.column_width = style.column_width.clone();
        content.column_height = style.column_height.clone();
        content.column_wrap = style.column_wrap;
        content.column_fill = style.column_fill;
        content.column_gap = style.column_gap.clone();
        content.column_rule_color = style.column_rule_color;
        content.column_rule_style = style.column_rule_style;
        content.column_rule_width = style.column_rule_width;
        content.row_gap = style.row_gap.clone();
        content.flex_direction = style.flex_direction;
        content.flex_wrap = style.flex_wrap;
        content.grid_template_columns = style.grid_template_columns.clone();
        content.grid_template_rows = style.grid_template_rows.clone();
        content.grid_template_areas = style.grid_template_areas.clone();
        content.grid_auto_columns = style.grid_auto_columns.clone();
        content.grid_auto_rows = style.grid_auto_rows.clone();
        content.grid_auto_flow = style.grid_auto_flow;
        content.grid_column = style.grid_column.clone();
        content.grid_row = style.grid_row.clone();
        content.overflow_x = style.overflow_x;
        content.overflow_y = style.overflow_y;
        content.unicode_bidi = style.unicode_bidi;
    })
}

pub(crate) fn intrinsic_sizes(
    doc: &dyn LayoutDocument,
    owner: NodeId,
) -> crate::intrinsic_sizing::IntrinsicSizes {
    let style = &doc.node(owner).style;
    let legend = doc.fieldset_rendered_legend(owner);
    let context = FieldsetContentContext::new(doc, owner, content_style(style), legend);
    let mut sizes =
        crate::intrinsic_sizing::compute_intrinsic_block_sizes_with_layout(&context, owner);
    let border = crate::block::resolve_border(style);
    let padding = crate::block::resolve_padding(style, LayoutUnit::zero());
    if !style.has_inline_size_containment() {
        if let Some(legend) = legend {
            let contribution =
                crate::intrinsic_sizing::compute_child_intrinsic_contribution_with_layout(
                    doc, legend,
                );
            sizes.min_content_inline_size = sizes
                .min_content_inline_size
                .max_of(contribution.min_content_inline_size + padding.inline_sum());
            sizes.max_content_inline_size = sizes
                .max_content_inline_size
                .max_of(contribution.max_content_inline_size + padding.inline_sum());
        }
    }
    sizes.min_content_inline_size += border.inline_sum();
    sizes.max_content_inline_size += border.inline_sum();
    sizes.min_content_block_size += border.block_sum();
    sizes.max_content_block_size += border.block_sum();
    sizes
}

pub(crate) fn layout(doc: &dyn LayoutDocument, owner: NodeId, space: &ConstraintSpace) -> Fragment {
    let style = &doc.node(owner).style;
    let direction = space.writing_direction;
    let border = space
        .used_border_for(owner)
        .unwrap_or_else(|| crate::block::resolve_border(style));
    let padding = crate::block::resolve_padding(style, space.percentage_resolution_inline_size);
    let logical_border = border.to_logical(direction);
    let logical_padding = padding.to_logical(direction);
    let bp_inline = logical_border.inline_sum() + logical_padding.inline_sum();
    let bp_block = logical_border.block_sum() + logical_padding.block_sum();
    let projected = crate::block::style_with_logical_dimensions(style, direction);
    let logical_style = projected.as_ref().unwrap_or(style);
    let used_inline =
        crate::block::resolve_inline_size(doc, owner, logical_style, space, bp_inline, bp_block);
    let inline = if style.box_sizing == BoxSizing::BorderBox {
        used_inline
    } else {
        used_inline + bp_inline
    };
    let content_inline = (inline - bp_inline).clamp_negative_to_zero();
    let initial_block = crate::block::resolve_block_size(
        doc,
        owner,
        logical_style,
        space,
        crate::containment::logical_block_fallback(style)
            .map_or(bp_block, |fallback| fallback + bp_block),
        bp_block,
        bp_inline,
        used_inline,
        false,
    );
    let automatic_block = (logical_style.height.is_auto()
        || logical_style.height.is_percent()
            && space.percentage_resolution_block_size.is_indefinite())
        && logical_style.aspect_ratio.is_none()
        && !space.is_fixed_block_size
        && !space.stretch_block_size
        && !style.has_block_size_containment();
    let legend_id = doc.fieldset_rendered_legend(owner);
    let mut legend_fragment = legend_id.map(|legend| {
        let legend_style = &doc.node(legend).style;
        let margin =
            crate::block::resolve_margins(legend_style, content_inline).to_logical(direction);
        let available_inline = (content_inline - margin.inline_sum()).clamp_negative_to_zero();
        let child_direction = legend_style
            .direction
            .writing_direction(legend_style.writing_mode);
        let available_block = if automatic_block {
            INDEFINITE_SIZE
        } else {
            (initial_block - bp_block).clamp_negative_to_zero()
        };
        let mut child_space = ConstraintSpace::for_block_child_from_parent(
            space,
            available_inline,
            available_block,
            content_inline,
            available_block,
            true,
            child_direction,
        );
        // A rendered legend is monolithic and auto inline size is fit-content.
        child_space.fragmentainer_block_size = LayoutUnit::zero();
        child_space.outer_fragmentainer_block_size = LayoutUnit::zero();
        if crate::logical_geometry::ResolvedLogicalBox::from_style(legend_style)
            .sizes
            .inline_size
            .is_auto()
        {
            let sizes =
                crate::intrinsic_sizing::compute_logical_intrinsic_inline_sizes(doc, legend);
            child_space.available_inline_size = sizes
                .min
                .max_of(child_space.available_inline_size.min_of(sizes.max));
            child_space.is_fixed_inline_size = true;
        }
        let fragment = crate::block_layout_with_layout(doc, legend, &child_space);
        (fragment, margin)
    });
    let mut content_start = logical_border.block_start;
    let mut legend_position = LogicalOffset::zero();
    if let Some((legend, margin)) = &legend_fragment {
        let size =
            WritingModeConverter::new(direction, PhysicalSize::zero()).to_logical_size(legend.size);
        let block_offset =
            ((logical_border.block_start - size.block_size) / 2).clamp_negative_to_zero();
        content_start = content_start.max_of(block_offset + size.block_size + margin.block_end);
        let mut inline_start =
            logical_border.inline_start + logical_padding.inline_start + margin.inline_start;
        let free = content_inline - size.inline_size;
        if free > LayoutUnit::zero() {
            let legend_style = &doc.node(legend.node_id).style;
            let margin_lengths = crate::logical_geometry::logical_length_sides(
                &legend_style.margin_top,
                &legend_style.margin_right,
                &legend_style.margin_bottom,
                &legend_style.margin_left,
                direction,
            );
            let start_auto = margin_lengths.inline_start.is_auto();
            let end_auto = margin_lengths.inline_end.is_auto();
            // Exact .50 ComputeLegendBlockAlignment: auto margins take
            // precedence; only Left/Right/Center affect legacy text alignment.
            let center = if start_auto || end_auto {
                start_auto && end_auto
            } else {
                legend_style.text_align == TextAlign::Center
            };
            let end = if start_auto || end_auto {
                start_auto && !end_auto
            } else {
                matches!(legend_style.text_align,
                    TextAlign::Right if !direction.is_rtl())
                    || matches!(legend_style.text_align,
                        TextAlign::Left if direction.is_rtl())
            };
            if center {
                inline_start += free / 2;
            } else if end {
                inline_start += free - margin.inline_end;
            }
        }
        legend_position = LogicalOffset::new(inline_start, block_offset);
    }
    let reservation = content_start - logical_border.block_start;
    let helper_style = content_style(style);
    let context = FieldsetContentContext::new(doc, owner, helper_style.clone(), legend_id);
    let helper_inline = (inline - logical_border.inline_sum()).clamp_negative_to_zero();
    let mut helper_space = ConstraintSpace::for_block_child_from_parent(
        space,
        helper_inline,
        if automatic_block {
            INDEFINITE_SIZE
        } else {
            (initial_block - logical_border.block_sum() - reservation)
                .max_of(logical_padding.block_sum())
        },
        space.percentage_resolution_inline_size,
        space.percentage_resolution_block_size,
        true,
        direction,
    );
    helper_space.stretch_inline_size = true;
    helper_space.is_fixed_block_size = !automatic_block;
    helper_space.is_initial_block_size_indefinite = automatic_block;
    helper_space.block_offset_in_fragmentainer += content_start;
    let mut helper = crate::block_layout_with_layout(&context, owner, &helper_space);
    let mut helper_block = WritingModeConverter::new(direction, PhysicalSize::zero())
        .to_logical_size(helper.size)
        .block_size;
    let intrinsic = crate::containment::logical_block_fallback(style).map_or(
        content_start + helper_block + logical_border.block_end,
        |fallback| fallback + bp_block,
    );
    let mut block = crate::block::resolve_block_size(
        doc,
        owner,
        logical_style,
        space,
        intrinsic,
        bp_block,
        bp_inline,
        used_inline,
        false,
    );
    if !style.has_block_size_containment() {
        block = block.max_of(bp_block + reservation);
    }
    // An indefinite fieldset with a constraining max-block-size re-lays out
    // its real content box, so percentages, tracks and scrollbars see that size.
    if automatic_block && block < intrinsic {
        helper_space.available_block_size =
            (block - logical_border.block_sum() - reservation).max_of(logical_padding.block_sum());
        helper_space.is_fixed_block_size = true;
        helper_space.is_initial_block_size_indefinite = false;
        helper = crate::block_layout_with_layout(&context, owner, &helper_space);
        helper_block = WritingModeConverter::new(direction, PhysicalSize::zero())
            .to_logical_size(helper.size)
            .block_size;
        block = block.max_of(content_start + helper_block + logical_border.block_end);
    }
    let size = WritingModeConverter::new(direction, PhysicalSize::zero())
        .to_physical_size(LogicalSize::new(inline, block));
    let converter = WritingModeConverter::new(direction, size);
    helper.offset = converter.to_physical_offset(
        LogicalOffset::new(logical_border.inline_start, content_start),
        helper.size,
    );
    let mut wrapper = Fragment::new_box(owner, size);
    wrapper.border = border;
    wrapper.padding = padding;
    wrapper.margin = crate::block::resolve_margins(style, space.percentage_resolution_inline_size);
    wrapper.first_baseline = helper
        .first_baseline
        .map(|baseline| baseline + content_start);
    wrapper.last_baseline = helper
        .last_baseline
        .map(|baseline| baseline + content_start);
    if let Some((mut legend, _)) = legend_fragment.take() {
        legend.offset = converter.to_physical_offset(legend_position, legend.size);
        // Preserve bubbled candidates in wrapper coordinates; the ordinary
        // parent algorithm resolves candidates owned outside this fieldset.
        for mut candidate in std::mem::take(&mut legend.oof_candidates) {
            candidate.static_position.left += legend.offset.left;
            candidate.static_position.top += legend.offset.top;
            wrapper.oof_candidates.push(candidate);
        }
        wrapper.children.push(legend);
    }
    for mut candidate in std::mem::take(&mut helper.oof_candidates) {
        candidate.static_position.left += helper.offset.left;
        candidate.static_position.top += helper.offset.top;
        wrapper.oof_candidates.push(candidate);
    }
    wrapper.children.push(helper);
    finish_out_of_flow(doc, owner, style, &mut wrapper);
    let mut overflow = PhysicalRect::new(PhysicalOffset::zero(), size);
    for child in &wrapper.children {
        overflow = overflow.unite(&PhysicalRect::new(child.offset, child.size));
    }
    if overflow.size != size || overflow.offset != PhysicalOffset::zero() {
        wrapper.overflow_rect = Some(overflow);
    }
    wrapper
}

/// The authored wrapper still owns positioned descendants of its legend.
/// Content descendants use the anonymous box's independently established
/// containing block, as LayoutFieldset::InsertedIntoTree requires.
fn finish_out_of_flow(
    doc: &dyn LayoutDocument,
    owner: NodeId,
    style: &ComputedStyle,
    wrapper: &mut Fragment,
) {
    let captures_absolute = owner == doc.root()
        || style.position.is_positioned()
        || style.has_transform_containing_block()
        || style.has_layout_containment();
    let captures_fixed = owner == doc.root()
        || style.has_transform_containing_block()
        || style.has_layout_containment();
    let size = PhysicalSize::new(
        (wrapper.size.width - wrapper.border.left - wrapper.border.right).clamp_negative_to_zero(),
        (wrapper.size.height - wrapper.border.top - wrapper.border.bottom).clamp_negative_to_zero(),
    );
    let mut candidates = std::mem::take(&mut wrapper.oof_candidates);
    while !candidates.is_empty() {
        let mut local = Vec::new();
        for mut candidate in candidates {
            let captures = if candidate.style.position == Position::Fixed {
                captures_fixed
            } else {
                captures_absolute
            };
            if !captures {
                wrapper.oof_candidates.push(candidate);
                continue;
            }
            if !candidate.has_inline_containing_block {
                candidate.containing_block_node = owner;
                candidate.containing_block_size = size;
                candidate.containing_block_offset = PhysicalOffset::zero();
                candidate.containing_block_border = wrapper.border;
                candidate.containing_block_direction = style.direction;
            }
            local.push(candidate);
        }
        candidates = Vec::new();
        for mut child in crate::out_of_flow::layout_out_of_flow_children_with_layout(doc, &local) {
            for mut candidate in std::mem::take(&mut child.oof_candidates) {
                candidate.static_position.left += child.offset.left;
                candidate.static_position.top += child.offset.top;
                candidates.push(candidate);
            }
            wrapper.children.push(child);
        }
    }
}
