//! Intrinsic block sizing — min-content, max-content, shrink-to-fit.
//!
//! CSS Intrinsic & Extrinsic Sizing Module Level 3.
//!
//! This module computes the natural width/height of block-level elements
//! based on their content, ignoring available space from the parent.
//! Used for:
//! - Auto-width determination
//! - Shrink-to-fit contexts (floats, abs pos, inline-blocks)
//! - `min-content` / `max-content` CSS values
//! - Table cell sizing
//!
//! Source: CSS Sizing 3 §4-5, CSS 2.1 §10.3.5-7, §10.6.7.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{
    LayoutUnit, Length, LengthType, MinMaxSizes, PhysicalSize, WritingDirectionMode,
    WritingModeConverter,
};
use openui_style::{
    BoxSizing, Clear, ColumnSpan, ComputedStyle, FontFamily, LineBreak, WhiteSpace, WordBreak,
};

use crate::block::{resolve_border, resolve_margins, resolve_padding};
use crate::inline::items::InlineItemType;
use crate::inline::items_builder::{
    preprocess_text_for_shaping, style_to_font_description, InlineItemsBuilder, InlineItemsData,
};
use crate::length_resolver::{resolve_length, resolve_margin_or_padding};

/// Check if a style represents an inline-level element.
fn is_inline_level(style: &ComputedStyle) -> bool {
    style.display.is_inline_level()
}

fn uses_deterministic_text_profile(style: &ComputedStyle) -> bool {
    style.font_family.families.iter().any(|family| {
        matches!(family, FontFamily::Named(name) if name.eq_ignore_ascii_case("Droid Sans Fallback"))
    })
}

fn is_interior_collapsible_whitespace(doc: &Document, siblings: &[NodeId], index: usize) -> bool {
    let node = doc.node(siblings[index]);
    if node.tag != ElementTag::Text
        || !matches!(
            node.style.white_space,
            WhiteSpace::Normal | WhiteSpace::Nowrap
        )
        || !node
            .text
            .as_deref()
            .is_some_and(|text| !text.is_empty() && text.chars().all(char::is_whitespace))
    {
        return false;
    }

    let is_effective_inline = |node_id: NodeId| -> Option<bool> {
        let sibling = doc.node(node_id);
        if sibling.style.display == openui_style::Display::None
            || sibling.style.position.is_absolutely_positioned()
        {
            None
        } else {
            Some(
                sibling.tag != ElementTag::Break
                    && (sibling.tag == ElementTag::Text || is_inline_level(&sibling.style)),
            )
        }
    };
    let previous_inline = siblings[..index]
        .iter()
        .rev()
        .find_map(|node_id| is_effective_inline(*node_id))
        .unwrap_or(false);
    let next_inline = siblings[index + 1..]
        .iter()
        .find_map(|node_id| is_effective_inline(*node_id))
        .unwrap_or(false);
    previous_inline && next_inline
}

/// Private projection between the logical axes used by intrinsic/flex
/// algorithms and the physical size properties retained by ComputedStyle.
#[derive(Clone, Copy)]
struct IntrinsicAxisMapping {
    writing_direction: WritingDirectionMode,
}

impl IntrinsicAxisMapping {
    fn for_style(style: &ComputedStyle) -> Self {
        Self {
            writing_direction: style.writing_mode.to_writing_direction(style.direction),
        }
    }

    fn inline_size<'a>(self, style: &'a ComputedStyle) -> &'a Length {
        if self.writing_direction.is_horizontal() {
            &style.width
        } else {
            &style.height
        }
    }

    fn block_size<'a>(self, style: &'a ComputedStyle) -> &'a Length {
        if self.writing_direction.is_horizontal() {
            &style.height
        } else {
            &style.width
        }
    }

    fn min_inline_size<'a>(self, style: &'a ComputedStyle) -> &'a Length {
        if self.writing_direction.is_horizontal() {
            &style.min_width
        } else {
            &style.min_height
        }
    }

    fn max_inline_size<'a>(self, style: &'a ComputedStyle) -> &'a Length {
        if self.writing_direction.is_horizontal() {
            &style.max_width
        } else {
            &style.max_height
        }
    }

    fn min_block_size<'a>(self, style: &'a ComputedStyle) -> &'a Length {
        if self.writing_direction.is_horizontal() {
            &style.min_height
        } else {
            &style.min_width
        }
    }

    fn max_block_size<'a>(self, style: &'a ComputedStyle) -> &'a Length {
        if self.writing_direction.is_horizontal() {
            &style.max_height
        } else {
            &style.max_width
        }
    }

    fn main_size<'a>(self, style: &'a ComputedStyle, is_column: bool) -> &'a Length {
        if is_column {
            self.block_size(style)
        } else {
            self.inline_size(style)
        }
    }

    fn cross_size<'a>(self, style: &'a ComputedStyle, is_column: bool) -> &'a Length {
        if is_column {
            self.inline_size(style)
        } else {
            self.block_size(style)
        }
    }

    fn min_main_size<'a>(self, style: &'a ComputedStyle, is_column: bool) -> &'a Length {
        if is_column {
            self.min_block_size(style)
        } else {
            self.min_inline_size(style)
        }
    }

    fn max_main_size<'a>(self, style: &'a ComputedStyle, is_column: bool) -> &'a Length {
        if is_column {
            self.max_block_size(style)
        } else {
            self.max_inline_size(style)
        }
    }
}

fn has_spanner_descendant_through_transparent_wrappers(doc: &Document, node_id: NodeId) -> bool {
    let style = &doc.node(node_id).style;
    if style.display == openui_style::Display::None || style.position.is_absolutely_positioned() {
        return false;
    }
    if style.column_span == ColumnSpan::All {
        return true;
    }
    if !style.display.is_block_level()
        || style.creates_new_formatting_context()
        || style.overflow_x.is_scrollable()
        || style.overflow_y.is_scrollable()
        || crate::multicol::ColumnLayoutAlgorithm::from_style(style).is_some()
    {
        return false;
    }

    doc.children(node_id)
        .any(|child_id| has_spanner_descendant_through_transparent_wrappers(doc, child_id))
}

// ── Result struct ────────────────────────────────────────────────────────

/// Intrinsic sizes for an element in both axes.
///
/// CSS Sizing 3 §4: every box has a min-content and max-content size in
/// both the inline and block dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntrinsicSizes {
    /// The narrowest the element can be without overflow (inline axis).
    pub min_content_inline_size: LayoutUnit,
    /// The widest the element would be given infinite available space (inline axis).
    pub max_content_inline_size: LayoutUnit,
    /// Min-content contribution in the block axis.
    pub min_content_block_size: LayoutUnit,
    /// Max-content contribution in the block axis.
    pub max_content_block_size: LayoutUnit,
}

impl IntrinsicSizes {
    pub fn zero() -> Self {
        Self {
            min_content_inline_size: LayoutUnit::zero(),
            max_content_inline_size: LayoutUnit::zero(),
            min_content_block_size: LayoutUnit::zero(),
            max_content_block_size: LayoutUnit::zero(),
        }
    }
}

impl Default for IntrinsicSizes {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Clone, Copy)]
struct IntrinsicInlineBox {
    node_id: NodeId,
    start_edge: LayoutUnit,
    end_edge: LayoutUnit,
    clone_edges: bool,
    active_on_segment: bool,
}

fn inline_start_edge(style: &ComputedStyle) -> LayoutUnit {
    if style.direction == openui_style::Direction::Rtl {
        resolve_margin_or_padding(&style.margin_right, LayoutUnit::zero())
            + LayoutUnit::from_i32(style.effective_border_right())
            + resolve_margin_or_padding(&style.padding_right, LayoutUnit::zero())
    } else {
        resolve_margin_or_padding(&style.margin_left, LayoutUnit::zero())
            + LayoutUnit::from_i32(style.effective_border_left())
            + resolve_margin_or_padding(&style.padding_left, LayoutUnit::zero())
    }
}

fn inline_end_edge(style: &ComputedStyle) -> LayoutUnit {
    if style.direction == openui_style::Direction::Rtl {
        resolve_margin_or_padding(&style.padding_left, LayoutUnit::zero())
            + LayoutUnit::from_i32(style.effective_border_left())
            + resolve_margin_or_padding(&style.margin_left, LayoutUnit::zero())
    } else {
        resolve_margin_or_padding(&style.padding_right, LayoutUnit::zero())
            + LayoutUnit::from_i32(style.effective_border_right())
            + resolve_margin_or_padding(&style.margin_right, LayoutUnit::zero())
    }
}

fn active_clone_start_edges(stack: &[IntrinsicInlineBox]) -> LayoutUnit {
    stack
        .iter()
        .filter(|entry| entry.active_on_segment && entry.clone_edges)
        .fold(LayoutUnit::zero(), |sum, entry| sum + entry.start_edge)
}

fn active_clone_end_edges(stack: &[IntrinsicInlineBox]) -> LayoutUnit {
    stack
        .iter()
        .rev()
        .filter(|entry| entry.active_on_segment && entry.clone_edges)
        .fold(LayoutUnit::zero(), |sum, entry| sum + entry.end_edge)
}

/// Convert a shaped advance to Blink's 1/64px intrinsic-size grid.
///
/// Blink's ShapeResult::SnappedWidth preserves the shaped advance and
/// ceil-converts it. Even a small positive remainder is observable through
/// native bounds and must not be rounded away before this conversion.
fn intrinsic_text_width(width: f32) -> LayoutUnit {
    LayoutUnit::from_f32_ceil(width)
}

fn intrinsic_close_rounding_excess(data: &InlineItemsData, close_item_index: usize) -> LayoutUnit {
    let next_starts_non_whitespace = data.items[close_item_index + 1..]
        .iter()
        .find_map(|candidate| match candidate.item_type {
            InlineItemType::OpenTag | InlineItemType::CloseTag => None,
            InlineItemType::Text => Some(
                data.text[candidate.text_range.clone()]
                    .chars()
                    .next()
                    .is_some_and(|character| !character.is_whitespace()),
            ),
            InlineItemType::AtomicInline
            | InlineItemType::Control
            | InlineItemType::BlockInInline => Some(false),
        })
        .unwrap_or(false);
    if !next_starts_non_whitespace || close_item_index == 0 {
        return LayoutUnit::zero();
    }

    let Some(last_item) = data.items[..close_item_index]
        .last()
        .filter(|item| item.item_type == InlineItemType::Text)
    else {
        return LayoutUnit::zero();
    };
    let node_id = last_item.node_id;
    let mut exact_width = 0.0f32;
    let mut allocated_width = LayoutUnit::zero();
    let mut run_count = 0usize;
    for item in data.items[..close_item_index].iter().rev() {
        if item.item_type != InlineItemType::Text
            || item.node_id != node_id
            || data.text[item.text_range.clone()]
                .chars()
                .any(char::is_whitespace)
        {
            break;
        }
        let Some(shape_result) = item.shape_result.as_ref() else {
            break;
        };
        exact_width += shape_result.width;
        allocated_width = allocated_width + intrinsic_text_width(shape_result.width);
        run_count += 1;
    }
    if run_count > 1 {
        (allocated_width - intrinsic_text_width(exact_width)).clamp_negative_to_zero()
    } else {
        LayoutUnit::zero()
    }
}

/// Compute min/max-content sizes over the flattened contents of one IFC.
///
/// The ordinary recursive accumulator is correct for block children, but an
/// IFC needs line-break state across DOM boundaries. This compact scanner is
/// deliberately fed by `InlineItemsBuilder`, which has already performed CSS
/// white-space processing and emitted explicit open/close decoration items.
fn compute_inline_sequence_intrinsic_sizes(doc: &Document, node_id: NodeId) -> MinMaxSizes {
    let container_style = &doc.node(node_id).style;
    let mut data = InlineItemsBuilder::collect_for_intrinsic_sizes(doc, node_id);
    let base_direction = if container_style.unicode_bidi == openui_style::UnicodeBidi::Plaintext {
        None
    } else if container_style.direction == openui_style::Direction::Rtl {
        Some(openui_text::TextDirection::Rtl)
    } else {
        Some(openui_text::TextDirection::Ltr)
    };
    data.apply_bidi(base_direction);
    data.split_shaping_runs();
    // Indentation belongs to the first forced line, rather than to every
    // recursive child contribution. Percentage indentation is cyclic during
    // intrinsic sizing, so its basis is zero while calc's fixed part remains.
    // Empty content has no line and therefore no indentation contribution.
    let has_line_content = data.items.iter().any(|item| match item.item_type {
        InlineItemType::Text => !item.text_range.is_empty(),
        InlineItemType::Control | InlineItemType::AtomicInline => true,
        InlineItemType::OpenTag => {
            inline_start_edge(&data.styles[item.style_index]) != LayoutUnit::zero()
        }
        InlineItemType::CloseTag => {
            inline_end_edge(&data.styles[item.style_index]) != LayoutUnit::zero()
        }
        InlineItemType::BlockInInline => false,
    });
    let first_line_indent = if has_line_content {
        resolve_length(
            &container_style.text_indent,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
    } else {
        LayoutUnit::zero()
    };
    data.shape_text();

    let mut stack: Vec<IntrinsicInlineBox> = Vec::new();
    let mut pending_open_edges = LayoutUnit::zero();
    let mut min_segment = first_line_indent;
    let mut min_content = LayoutUnit::zero();
    let mut soft_break_pending = false;
    let mut soft_break_after_pending_edges = false;
    let mut last_content_was_text = false;

    let mut max_line = first_line_indent;
    let mut max_content = LayoutUnit::zero();
    // Indentation and decoration edges do not make line-start whitespace
    // interior. Only text or an atomic object does.
    let mut line_has_content = false;
    let mut pending_collapsible_space = LayoutUnit::zero();
    let mut pending_min_collapsible_space = LayoutUnit::zero();

    let commit_soft_segment = |min_segment: &mut LayoutUnit,
                               min_content: &mut LayoutUnit,
                               pending_open_edges: &mut LayoutUnit,
                               stack: &mut [IntrinsicInlineBox]| {
        *min_content = (*min_content).max_of(*min_segment + active_clone_end_edges(stack));
        *min_segment = active_clone_start_edges(stack) + *pending_open_edges;
        *pending_open_edges = LayoutUnit::zero();
        for entry in stack {
            entry.active_on_segment = true;
        }
    };

    let commit_forced_line =
        |min_segment: &mut LayoutUnit,
         min_content: &mut LayoutUnit,
         pending_open_edges: &mut LayoutUnit,
         stack: &mut [IntrinsicInlineBox],
         max_line: &mut LayoutUnit,
         max_content: &mut LayoutUnit,
         pending_collapsible_space: &mut LayoutUnit| {
            *min_segment = *min_segment + *pending_open_edges;
            *pending_open_edges = LayoutUnit::zero();
            for entry in stack.iter_mut() {
                entry.active_on_segment = true;
            }
            *min_content = (*min_content).max_of(*min_segment + active_clone_end_edges(stack));
            *min_segment = active_clone_start_edges(stack);
            *max_content = (*max_content).max_of(*max_line);
            *max_line = LayoutUnit::zero();
            *pending_collapsible_space = LayoutUnit::zero();
        };

    for (item_index, item) in data.items.iter().enumerate() {
        let style = &data.styles[item.style_index];
        match item.item_type {
            InlineItemType::OpenTag => {
                // In max-content mode an inline edge proves that preceding
                // whitespace is interior rather than trailing. In min-content
                // mode retain the edge separately until the following content
                // decides which side of a soft break owns it.
                max_line = max_line + pending_collapsible_space;
                pending_collapsible_space = LayoutUnit::zero();
                let start_edge = inline_start_edge(style);
                max_line = max_line + start_edge;
                pending_open_edges = pending_open_edges + start_edge;
                stack.push(IntrinsicInlineBox {
                    node_id: item.node_id,
                    start_edge,
                    end_edge: inline_end_edge(style),
                    clone_edges: style.box_decoration_break
                        == openui_style::BoxDecorationBreak::Clone,
                    active_on_segment: false,
                });
            }
            InlineItemType::CloseTag => {
                let shaping_run_rounding_excess =
                    intrinsic_close_rounding_excess(&data, item_index);
                let position = stack
                    .iter()
                    .rposition(|entry| entry.node_id == item.node_id);
                let entry = position.map(|position| stack.remove(position));
                if let Some(entry) = entry {
                    if !entry.active_on_segment {
                        min_segment = min_segment + pending_open_edges;
                        pending_open_edges = LayoutUnit::zero();
                    }
                    min_segment = min_segment + entry.end_edge;
                    max_line = max_line + entry.end_edge;
                } else {
                    let edge = inline_end_edge(style);
                    min_segment = min_segment + pending_open_edges + edge;
                    pending_open_edges = LayoutUnit::zero();
                    max_line = max_line + edge;
                }
                min_segment = min_segment - shaping_run_rounding_excess;
                max_line = max_line - shaping_run_rounding_excess;
                last_content_was_text = false;
            }
            InlineItemType::Control => {
                line_has_content = false;
                soft_break_after_pending_edges = false;
                last_content_was_text = false;
                pending_min_collapsible_space = LayoutUnit::zero();
                commit_forced_line(
                    &mut min_segment,
                    &mut min_content,
                    &mut pending_open_edges,
                    &mut stack,
                    &mut max_line,
                    &mut max_content,
                    &mut pending_collapsible_space,
                );
            }
            InlineItemType::AtomicInline => {
                min_segment = min_segment + pending_min_collapsible_space;
                pending_min_collapsible_space = LayoutUnit::zero();
                if soft_break_pending {
                    commit_soft_segment(
                        &mut min_segment,
                        &mut min_content,
                        &mut pending_open_edges,
                        &mut stack,
                    );
                } else {
                    min_segment = min_segment + pending_open_edges;
                    pending_open_edges = LayoutUnit::zero();
                    for entry in &mut stack {
                        entry.active_on_segment = true;
                    }
                }
                soft_break_after_pending_edges = false;
                last_content_was_text = false;

                // An atomic child's contribution includes its preferred size,
                // min/max constraints, decorations and margins. The content
                // endpoints stored on an inline item omit those constraints;
                // using them here can reserve 40px for a box whose own layout
                // produces a 200px minimum. Share the block/flex contribution
                // calculation so enclosing intrinsic sizes retain the same
                // constrained box that normal layout will place.
                let intrinsic = compute_child_intrinsic_contribution(doc, item.node_id);
                min_segment = min_segment + intrinsic.min_content_inline_size;
                max_line = max_line + pending_collapsible_space + intrinsic.max_content_inline_size;
                line_has_content = true;
                pending_collapsible_space = LayoutUnit::zero();
                // Atomic inline-level boxes establish a soft wrap
                // opportunity at their boundary. Adjacent inline-blocks
                // therefore contribute their combined width to max-content,
                // but only the widest box to min-content.
                soft_break_pending =
                    !matches!(style.white_space, WhiteSpace::Nowrap | WhiteSpace::Pre)
                        && !matches!(
                            container_style.white_space,
                            WhiteSpace::Nowrap | WhiteSpace::Pre
                        );
            }
            InlineItemType::Text => {
                let Some(shape) = item.shape_result.as_ref() else {
                    continue;
                };
                let text = &data.text[item.text_range.clone()];
                let wraps = !matches!(style.white_space, WhiteSpace::Nowrap | WhiteSpace::Pre)
                    && !matches!(
                        container_style.white_space,
                        WhiteSpace::Nowrap | WhiteSpace::Pre
                    );
                let collapses = matches!(
                    style.white_space,
                    WhiteSpace::Normal | WhiteSpace::Nowrap | WhiteSpace::PreLine
                );
                let forces_newlines = matches!(
                    style.white_space,
                    WhiteSpace::Pre
                        | WhiteSpace::PreWrap
                        | WhiteSpace::PreLine
                        | WhiteSpace::BreakSpaces
                );

                let mut run_start = 0usize;
                let mut run_start_char = 0usize;
                // A forced newline starts a separately snapped line. Soft
                // breaks retain cumulative advance rounding within that line.
                let mut line_start_char = 0usize;
                let chars: Vec<(usize, char)> = text.char_indices().collect();
                for (index, &(byte_index, ch)) in chars.iter().enumerate() {
                    let char_end = index + 1;
                    let byte_end = byte_index + ch.len_utf8();
                    let is_forced = forces_newlines && ch == '\n';
                    let is_space = ch.is_whitespace() && !is_forced;
                    let is_break_all_character = wraps
                        && (style.word_break == WordBreak::BreakAll
                            || style.line_break == LineBreak::Anywhere)
                        && !is_space
                        && !is_forced;
                    if collapses && is_space && !line_has_content && byte_index == run_start {
                        // Leading spaces contribute no width or soft break.
                        // Snap advances after them so the first word retains
                        // both its font's remainder and the first-line indent.
                        run_start = byte_end;
                        run_start_char = char_end;
                        line_start_char = char_end;
                        continue;
                    }
                    if !is_space && !is_forced && !is_break_all_character {
                        continue;
                    }

                    if byte_index > run_start {
                        let max_width =
                            intrinsic_text_width(shape.width_for_range(line_start_char, index))
                                - intrinsic_text_width(
                                    shape.width_for_range(line_start_char, run_start_char),
                                );
                        let width = max_width;
                        min_segment = min_segment + pending_min_collapsible_space;
                        pending_min_collapsible_space = LayoutUnit::zero();
                        if soft_break_pending {
                            if soft_break_after_pending_edges {
                                min_segment = min_segment + pending_open_edges;
                                pending_open_edges = LayoutUnit::zero();
                                for entry in &mut stack {
                                    entry.active_on_segment = true;
                                }
                            }
                            commit_soft_segment(
                                &mut min_segment,
                                &mut min_content,
                                &mut pending_open_edges,
                                &mut stack,
                            );
                        } else {
                            min_segment = min_segment + pending_open_edges;
                            pending_open_edges = LayoutUnit::zero();
                            for entry in &mut stack {
                                entry.active_on_segment = true;
                            }
                        }
                        soft_break_pending = false;
                        soft_break_after_pending_edges = false;
                        last_content_was_text = true;
                        min_segment = min_segment + width;
                        max_line = max_line + pending_collapsible_space + max_width;
                        pending_collapsible_space = LayoutUnit::zero();
                        line_has_content = true;
                    }

                    if is_forced {
                        soft_break_pending = false;
                        soft_break_after_pending_edges = false;
                        last_content_was_text = false;
                        commit_forced_line(
                            &mut min_segment,
                            &mut min_content,
                            &mut pending_open_edges,
                            &mut stack,
                            &mut max_line,
                            &mut max_content,
                            &mut pending_collapsible_space,
                        );
                        line_start_char = char_end;
                        line_has_content = false;
                    } else if is_break_all_character {
                        let max_width =
                            intrinsic_text_width(shape.width_for_range(line_start_char, char_end))
                                - intrinsic_text_width(
                                    shape.width_for_range(line_start_char, index),
                                );
                        let width = max_width;
                        min_segment = min_segment + pending_min_collapsible_space;
                        pending_min_collapsible_space = LayoutUnit::zero();
                        if soft_break_pending {
                            commit_soft_segment(
                                &mut min_segment,
                                &mut min_content,
                                &mut pending_open_edges,
                                &mut stack,
                            );
                        } else {
                            min_segment = min_segment + pending_open_edges;
                            pending_open_edges = LayoutUnit::zero();
                            for entry in &mut stack {
                                entry.active_on_segment = true;
                            }
                        }
                        min_segment = min_segment + width;
                        max_line = max_line + pending_collapsible_space + max_width;
                        pending_collapsible_space = LayoutUnit::zero();
                        line_has_content = true;
                        soft_break_pending = true;
                        soft_break_after_pending_edges = false;
                        last_content_was_text = true;
                    } else {
                        let max_width =
                            intrinsic_text_width(shape.width_for_range(line_start_char, char_end))
                                - intrinsic_text_width(
                                    shape.width_for_range(line_start_char, index),
                                );
                        let width = max_width;
                        if collapses {
                            pending_collapsible_space = max_width;
                        } else {
                            max_line = max_line + pending_collapsible_space + max_width;
                            pending_collapsible_space = LayoutUnit::zero();
                            line_has_content = true;
                        }
                        if wraps {
                            soft_break_pending = true;
                            soft_break_after_pending_edges = byte_index == 0
                                && pending_open_edges > LayoutUnit::zero()
                                && last_content_was_text;
                        } else if collapses {
                            // A collapsible space in nowrap contributes only
                            // if later content makes it interior. Keep it
                            // pending so terminal whitespace does not inflate
                            // the min/max-equivalent intrinsic width.
                            pending_min_collapsible_space = width;
                        } else {
                            min_segment = min_segment + pending_open_edges + width;
                            pending_open_edges = LayoutUnit::zero();
                            for entry in &mut stack {
                                entry.active_on_segment = true;
                            }
                        }
                    }
                    run_start = byte_end;
                    run_start_char = char_end;
                }

                if run_start < text.len() {
                    let max_width = intrinsic_text_width(
                        shape.width_for_range(line_start_char, shape.num_characters),
                    ) - intrinsic_text_width(
                        shape.width_for_range(line_start_char, run_start_char),
                    );
                    let width = max_width;
                    min_segment = min_segment + pending_min_collapsible_space;
                    pending_min_collapsible_space = LayoutUnit::zero();
                    if soft_break_pending {
                        if soft_break_after_pending_edges {
                            min_segment = min_segment + pending_open_edges;
                            pending_open_edges = LayoutUnit::zero();
                            for entry in &mut stack {
                                entry.active_on_segment = true;
                            }
                        }
                        commit_soft_segment(
                            &mut min_segment,
                            &mut min_content,
                            &mut pending_open_edges,
                            &mut stack,
                        );
                    } else {
                        min_segment = min_segment + pending_open_edges;
                        pending_open_edges = LayoutUnit::zero();
                        for entry in &mut stack {
                            entry.active_on_segment = true;
                        }
                    }
                    soft_break_pending = false;
                    soft_break_after_pending_edges = false;
                    last_content_was_text = true;
                    min_segment = min_segment + width;
                    max_line = max_line + pending_collapsible_space + max_width;
                    pending_collapsible_space = LayoutUnit::zero();
                    line_has_content = true;
                }
            }
            InlineItemType::BlockInInline => {}
        }
    }

    min_segment = min_segment + pending_open_edges;
    min_content = min_content.max_of(min_segment + active_clone_end_edges(&stack));
    max_content = max_content.max_of(max_line);
    // With a negative first-line indent, a later unbreakable segment may
    // contribute more to min-content than the forced line does to max-content.
    MinMaxSizes::new(min_content, max_content.max_of(min_content))
}

// ── Block container intrinsic sizing ─────────────────────────────────────

/// Compute intrinsic inline sizes for a block container.
///
/// CSS Sizing 3 §5.1: For a block container, the min-content inline size
/// is the maximum of all children's min-content inline-size contributions.
/// The max-content inline size is the maximum of all children's max-content
/// inline-size contributions.
///
/// Border and padding of the container are added to the result.
pub fn compute_intrinsic_block_sizes(doc: &Document, node_id: NodeId) -> IntrinsicSizes {
    let node = doc.node(node_id);
    let style = &node.style;
    let tag = node.tag;

    // Replaced elements use their own intrinsic dimensions.
    if node.replaced.is_some() || is_replaced_element(tag) {
        return apply_size_containment(
            style,
            compute_replaced_intrinsic_sizes_for_node(doc, node_id),
        );
    }

    // Flex containers have their own intrinsic sizing algorithm.
    // CSS Flexbox §9.9: Flex container intrinsic sizes.
    if style.display.is_flex() {
        return apply_size_containment(
            style,
            compute_flex_intrinsic_sizes(doc, node_id, style, None),
        );
    }

    if style.display.is_grid() {
        return apply_size_containment(
            style,
            crate::grid::compute_grid_intrinsic_sizes(doc, node_id),
        );
    }

    if style.display.is_table_wrapper() {
        return apply_size_containment(
            style,
            crate::table::compute_table_intrinsic_sizes(doc, node_id),
        );
    }

    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let bp_inline = border.inline_sum() + padding.inline_sum();
    let bp_block = border.block_sum() + padding.block_sum();

    let mut min_inline = LayoutUnit::zero();
    // CSS Sizing 3 §4.1: Max-content inline size must accommodate all floats
    // side-by-side (summed), plus the widest non-float child (maxed).
    let mut non_float_max_inline = LayoutUnit::zero();
    // For inline-only containers: max-content is the SUM of inline children
    // (they all sit on one line in max-content mode).
    let mut inline_children_max_sum = LayoutUnit::zero();
    // Keep the inline contribution separate from block-level children. Floats
    // and the widest unbroken inline line can coexist on the same row, so a
    // shrink-to-fit container must reserve their combined width. Block-level
    // children, on the other hand, continue to contribute through a maximum.
    let mut float_max_inline_sum = LayoutUnit::zero();
    let mut widest_float_inline_line = LayoutUnit::zero();
    // Track min-content and max-content block sizes separately.
    // CSS Sizing 3 §5: min-content uses each child's min-content contribution,
    // max-content uses each child's max-content contribution.
    let mut min_content_block = LayoutUnit::zero();
    let mut max_content_block = LayoutUnit::zero();
    // CSS Sizing 3 §5: Float block-size contributions differ between modes.
    let mut float_block_sum = LayoutUnit::zero(); // for min-content
    let mut float_block_max = LayoutUnit::zero(); // for max-content
    let is_bfc = style.creates_new_formatting_context();
    let multicol_algo = crate::multicol::ColumnLayoutAlgorithm::from_style(style);
    let mut has_multicol_columnar_content = false;
    let mut multicol_columnar_min_inline = LayoutUnit::zero();
    let mut multicol_columnar_max_inline = LayoutUnit::zero();
    let mut multicol_spanner_min_inline = LayoutUnit::zero();
    let mut multicol_spanner_max_inline = LayoutUnit::zero();
    let mut has_multicol_spanner = false;

    fn append_intrinsic_children(
        doc: &Document,
        parent: NodeId,
        output: &mut Vec<(usize, NodeId, i32)>,
        source_index: &mut usize,
    ) {
        if doc.node(parent).pseudo_kind == Some(openui_dom::PseudoElementKind::ScrollMarkerGroup) {
            let mut markers = Vec::new();
            crate::block::collect_scroll_marker_group_items(
                doc,
                doc.node(parent).pseudo_origin,
                &mut markers,
            );
            for child_id in markers {
                output.push((*source_index, child_id, doc.node(child_id).style.order));
                *source_index += 1;
            }
            return;
        }
        for child_id in doc.children(parent) {
            let child = doc.node(child_id);
            if child.style.display == openui_style::Display::Contents
                && !child.style.position.is_absolutely_positioned()
            {
                append_intrinsic_children(doc, child_id, output, source_index);
            } else {
                output.push((*source_index, child_id, child.style.order));
                *source_index += 1;
            }
        }
    }

    let mut ordered_children = Vec::new();
    let mut source_index = 0;
    append_intrinsic_children(doc, node_id, &mut ordered_children, &mut source_index);
    ordered_children.sort_by_key(|&(index, _, order)| (order, index));

    let ordered_child_ids: Vec<NodeId> = ordered_children
        .iter()
        .map(|(_, child_id, _)| *child_id)
        .collect();
    for (ordered_index, child_id) in ordered_child_ids.iter().copied().enumerate() {
        let child_style = &doc.node(child_id).style;

        // Skip absolutely positioned and display:none children.
        if child_style.display == openui_style::Display::None
            || child_style.position.is_absolutely_positioned()
        {
            continue;
        }

        let mut child_sizes = compute_child_intrinsic_contribution(doc, child_id);

        // A percentage-sized replaced child can acquire a definite block size
        // from this container while the container's auto inline size is being
        // measured. Transfer that resolved block size through the replaced
        // element's preferred ratio. Treating the percentage as indefinite
        // here leaves shrink-to-fit floats at the resource's natural width
        // even though their definite height makes the used width larger.
        let child_node = doc.node(child_id);
        if IntrinsicAxisMapping::for_style(style)
            .writing_direction
            .is_horizontal()
            && IntrinsicAxisMapping::for_style(child_style)
                .writing_direction
                .is_horizontal()
            && child_node.replaced.is_some()
            && child_style.width.is_auto()
            && tag != ElementTag::Fieldset
            && matches!(
                child_style.height.length_type(),
                LengthType::Percent | LengthType::Calculated
            )
            && style.height.is_fixed()
        {
            let parent_content_height = if style.box_sizing == BoxSizing::BorderBox {
                (LayoutUnit::from_f32(style.height.value()) - bp_block).clamp_negative_to_zero()
            } else {
                LayoutUnit::from_f32(style.height.value())
            };
            let resolved_height = resolve_length(
                &child_style.height,
                parent_content_height,
                parent_content_height,
                parent_content_height,
            );
            let natural_ratio = child_node
                .replaced
                .and_then(|content| content.intrinsic_ratio);
            let effective_ratio = child_style
                .aspect_ratio
                .as_ref()
                .and_then(|ratio| {
                    if ratio.auto_flag {
                        natural_ratio.or_else(|| {
                            (ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0).then_some(ratio.ratio)
                        })
                    } else {
                        (ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0).then_some(ratio.ratio)
                    }
                })
                .or(natural_ratio)
                .filter(|(width, height)| *width > 0.0 && *height > 0.0);
            if let Some((ratio_width, ratio_height)) = effective_ratio {
                let child_border = resolve_border(child_style);
                let child_padding = resolve_padding(child_style, LayoutUnit::zero());
                let child_inline_edges = child_border.inline_sum() + child_padding.inline_sum();
                let child_block_edges = child_border.block_sum() + child_padding.block_sum();
                let content_height = if child_style.box_sizing == BoxSizing::BorderBox {
                    (resolved_height - child_block_edges).clamp_negative_to_zero()
                } else {
                    resolved_height
                };
                let content_width =
                    LayoutUnit::from_f32(content_height.to_f32() * ratio_width / ratio_height);
                let child_margin = resolve_margins(child_style, LayoutUnit::zero());
                let transferred_inline =
                    content_width + child_inline_edges + child_margin.inline_sum();
                let transferred_block =
                    content_height + child_block_edges + child_margin.block_sum();
                child_sizes.min_content_inline_size = transferred_inline;
                child_sizes.max_content_inline_size = transferred_inline;
                child_sizes.min_content_block_size = transferred_block;
                child_sizes.max_content_block_size = transferred_block;
            }
        }
        if is_interior_collapsible_whitespace(doc, &ordered_child_ids, ordered_index) {
            let text = doc
                .node(child_id)
                .text
                .as_deref()
                .expect("whitespace child must be text");
            let text_sizes = compute_text_intrinsic_sizes_impl(doc, text, child_style, true);
            child_sizes.min_content_inline_size = text_sizes.min;
            child_sizes.max_content_inline_size = text_sizes.max;
        }
        let child_is_inline =
            is_inline_level(child_style) || doc.node(child_id).tag == ElementTag::Text;

        if multicol_algo.is_some() {
            if has_spanner_descendant_through_transparent_wrappers(doc, child_id) {
                has_multicol_spanner = true;
                multicol_spanner_min_inline =
                    multicol_spanner_min_inline.max_of(child_sizes.min_content_inline_size);
                multicol_spanner_max_inline =
                    multicol_spanner_max_inline.max_of(child_sizes.max_content_inline_size);
            } else {
                has_multicol_columnar_content = true;
                multicol_columnar_min_inline =
                    multicol_columnar_min_inline.max_of(child_sizes.min_content_inline_size);
                multicol_columnar_max_inline =
                    multicol_columnar_max_inline.max_of(child_sizes.max_content_inline_size);
            }
        }

        if child_style.float != openui_style::Float::None {
            // Clearance forces this float below the preceding float row.  A
            // shrink-to-fit ancestor therefore needs the widest row, not the
            // sum of floats that can never be side by side.
            if child_style.clear != Clear::None && float_max_inline_sum > LayoutUnit::zero() {
                widest_float_inline_line =
                    widest_float_inline_line.max_of(float_max_inline_sum + inline_children_max_sum);
                float_max_inline_sum = LayoutUnit::zero();
                inline_children_max_sum = LayoutUnit::zero();
            }
            min_inline = min_inline.max_of(child_sizes.min_content_inline_size);
            float_max_inline_sum = float_max_inline_sum + child_sizes.max_content_inline_size;
            float_block_sum = float_block_sum + child_sizes.min_content_block_size;
            float_block_max = float_block_max.max_of(child_sizes.max_content_block_size);
        } else if child_is_inline {
            // CSS Sizing 3 §4.1: Inline-level children share a line.
            // min-content: widest individual inline item.
            // max-content: inline items are summed within each explicit line;
            // a BR commits that line and starts the next one.
            min_inline = min_inline.max_of(child_sizes.min_content_inline_size);
            let child_node = doc.node(child_id);
            let is_preserved_newline_control = child_node.tag == ElementTag::Text
                && matches!(
                    child_style.white_space,
                    WhiteSpace::Pre
                        | WhiteSpace::PreWrap
                        | WhiteSpace::PreLine
                        | WhiteSpace::BreakSpaces
                )
                && child_node.text.as_deref().is_some_and(|text| {
                    !text.is_empty() && text.chars().all(|ch| matches!(ch, '\n' | '\r'))
                });
            if child_node.tag == ElementTag::Break || is_preserved_newline_control {
                widest_float_inline_line =
                    widest_float_inline_line.max_of(float_max_inline_sum + inline_children_max_sum);
                non_float_max_inline = non_float_max_inline.max_of(inline_children_max_sum);
                inline_children_max_sum = LayoutUnit::zero();
                // A forced inline break commits the current intrinsic float
                // row. Floats encountered after it form a later placement
                // segment; summing both segments makes a shrink-to-fit flex
                // item fill the available width even though each segment is
                // independently only as wide as its largest float row.
                float_max_inline_sum = LayoutUnit::zero();
            } else {
                inline_children_max_sum =
                    inline_children_max_sum + child_sizes.max_content_inline_size;
            }
            // Block-size for inline children is computed via inline layout below
            // (not by summing individual contributions, which would double-count).
        } else {
            // CSS Sizing 3 §4.1: non-float block children contribute via max.
            min_inline = min_inline.max_of(child_sizes.min_content_inline_size);
            non_float_max_inline = non_float_max_inline.max_of(child_sizes.max_content_inline_size);
            min_content_block = min_content_block + child_sizes.min_content_block_size;
            max_content_block = max_content_block + child_sizes.max_content_block_size;
        }
    }

    // Inline children on one line contribute their sum as max-content.
    widest_float_inline_line =
        widest_float_inline_line.max_of(float_max_inline_sum + inline_children_max_sum);
    non_float_max_inline = non_float_max_inline.max_of(inline_children_max_sum);

    // Max-content inline: floats and an unbroken inline line may share a row,
    // while block-level children occupy their own rows.
    let mut max_inline = non_float_max_inline.max_of(widest_float_inline_line);

    // BFC roots include float bottom margin edge in auto height (§10.6.7).
    if is_bfc {
        min_content_block = min_content_block.max_of(float_block_sum);
        max_content_block = max_content_block.max_of(float_block_max);
    }

    // CSS 2.1 §10.6.3 / CSS Sizing 3 §5: Inline content contributes to
    // the block-size by running inline layout at the given width and
    // measuring the resulting height. Individual inline items report zero
    // block-size; the container must compute it from line layout.
    //
    // Blink: BlockNode::ComputeMinMaxSizes → runs inline layout to
    // determine block-size contribution from inline formatting contexts.
    let has_inline_children = crate::inline::algorithm::has_inline_children(doc, node_id);
    let has_block_children = crate::block::has_block_children(doc, node_id);

    if has_inline_children && !has_block_children {
        // Intrinsic inline sizing is a property of the flattened inline
        // sequence, not of each direct child in isolation. In particular,
        // adjacent inline boxes do not introduce a soft wrap opportunity:
        // `<span>one</span><span>two</span>` is one min-content segment.
        // Conversely, a collapsible space can introduce an opportunity even
        // when it lives just inside an inline boundary. Use the same flattened
        // item stream as line layout so decoration edges, forced breaks, and
        // cross-node whitespace collapsing all participate in the result.
        // The recursive float walk above also tracks clearance-separated
        // rows. The inline item stream excludes float placeholders, so it
        // cannot replace that row accumulator. Ordinary IFCs use the complete
        // shaped sequence, including undecorated native text children.
        let has_float_children = ordered_child_ids
            .iter()
            .any(|child| doc.node(*child).style.float != openui_style::Float::None);
        if !has_float_children {
            let inline_sizes = compute_inline_sequence_intrinsic_sizes(doc, node_id);
            min_inline = inline_sizes.min;
            max_inline = inline_sizes.max;
        }

        let content_inline_min = min_inline;
        let content_inline_max = max_inline;

        // Run inline layout at min-content width to get the block-size when
        // text wraps as tightly as possible.
        let min_space = crate::ConstraintSpace::for_block_child(
            content_inline_min,
            LayoutUnit::max(),
            content_inline_min,
            LayoutUnit::zero(),
            false,
        );
        let min_frag = crate::inline::algorithm::inline_layout(doc, node_id, &min_space);
        min_content_block = min_content_block + min_frag.size.height;

        // Run inline layout at max-content width to get the block-size when
        // text is laid out as wide as possible (single line if fits).
        let max_space = crate::ConstraintSpace::for_block_child(
            content_inline_max,
            LayoutUnit::max(),
            content_inline_max,
            LayoutUnit::zero(),
            false,
        );
        let max_frag = crate::inline::algorithm::inline_layout(doc, node_id, &max_space);
        max_content_block = max_content_block + max_frag.size.height;
    }

    if let Some(algo) = multicol_algo {
        let column_count = algo.column_count.max(1);
        let count = if algo.column_count > 0 {
            LayoutUnit::from_i32(column_count as i32)
        } else {
            LayoutUnit::from_i32(1)
        };
        let gaps = if has_multicol_columnar_content {
            algo.column_gap * LayoutUnit::from_i32(algo.column_count.saturating_sub(1) as i32)
        } else {
            LayoutUnit::zero()
        };
        let specified_column_width = algo.column_width.unwrap_or(LayoutUnit::zero());
        let min_content_gaps =
            if algo.column_width.is_some() && specified_column_width == LayoutUnit::zero() {
                // A zero authored column measure contributes no hypothetical
                // columns to min-content sizing. The used layout measure still
                // receives its 1px progress floor and paints gaps between columns
                // that are actually generated, but those gaps cannot force an
                // auto-width float wider than its zero-width containing block.
                LayoutUnit::zero()
            } else {
                gaps
            };
        // CSS Multicol §3: a specified column-width is the preferred
        // fragmentainer measure and therefore the columnar contribution to
        // the multicol min-content size. Oversized descendants may visibly
        // overflow that fragmentainer; they do not inflate it. With
        // column-width:auto, the columnar min-content contribution supplies
        // the measure instead.
        let min_column_width = if !has_multicol_columnar_content {
            // An empty multicol has no min-content floor, but its authored
            // column width remains its preferred (max-content) measure. This
            // lets an auto-width float shrink to the available inline space
            // while still filling that space instead of collapsing to zero.
            LayoutUnit::zero()
        } else if algo.column_width.is_some() {
            specified_column_width
        } else {
            multicol_columnar_min_inline
        };
        // Max-content measurement keeps an uninterrupted row of floats
        // side-by-side even when final layout will place them into a narrow
        // authored column. This is observable when an auto-width floated
        // multicol is itself shrink-to-fit: its minimum contribution remains
        // the column measure, while its preferred contribution includes the
        // complete float row.
        let max_column_width = specified_column_width
            .max_of(multicol_columnar_max_inline)
            .max_of(widest_float_inline_line);
        min_inline =
            multicol_spanner_min_inline.max_of(min_column_width * count + min_content_gaps);
        max_inline = multicol_spanner_max_inline.max_of(max_column_width * count + gaps);

        // An unconstrained balanced multicol's intrinsic block contribution is
        // the height of one balanced column, not the unfragmented linear flow.
        // This value is consumed by flex/grid intrinsic sizing before final
        // multicol layout; reporting the linear height makes a multicol flex
        // item reserve all of its pre-fragmentation content below the columns.
        if column_count > 1 && has_multicol_columnar_content && !has_multicol_spanner {
            let divide_ceil = |size: LayoutUnit| {
                let raw = size.raw();
                let count = column_count as i32;
                LayoutUnit::from_raw((raw + count - 1) / count)
            };
            min_content_block = divide_ceil(min_content_block);
            max_content_block = divide_ceil(max_content_block);
        }
    }

    // Percentage margins resolve against the containing block's logical
    // inline size. The legacy block intrinsic accumulator above stores
    // physical width/height in its inline/block slots; for a vertical
    // container that means top/bottom percentages are cyclic in physical
    // height, while left/right percentages add to physical width from that
    // resolved height. Preserve the physical public contract here and solve
    // those contributions before adding this container's decorations.
    if !IntrinsicAxisMapping::for_style(style)
        .writing_direction
        .is_horizontal()
    {
        let percent = |length: &Length| -> f32 {
            if matches!(
                length.length_type(),
                LengthType::Percent | LengthType::Calculated
            ) {
                length.value() / 100.0
            } else {
                0.0
            }
        };
        let mut inline_axis_margin_fraction = 0.0f32;
        let mut block_axis_margin_fraction = 0.0f32;
        for child_id in doc.children(node_id) {
            let child = doc.node(child_id);
            if child.style.display == openui_style::Display::None
                || child.style.position.is_absolutely_positioned()
            {
                continue;
            }
            inline_axis_margin_fraction = inline_axis_margin_fraction
                .max(percent(&child.style.margin_top) + percent(&child.style.margin_bottom));
            block_axis_margin_fraction = block_axis_margin_fraction
                .max(percent(&child.style.margin_left) + percent(&child.style.margin_right));
        }
        let solve_inline_axis = |base: LayoutUnit| {
            if inline_axis_margin_fraction < 1.0 {
                LayoutUnit::from_f32(base.to_f32() / (1.0 - inline_axis_margin_fraction))
            } else {
                base
            }
        };
        min_content_block = solve_inline_axis(min_content_block);
        max_content_block = solve_inline_axis(max_content_block);
        min_inline = min_inline
            + LayoutUnit::from_f32(min_content_block.to_f32() * block_axis_margin_fraction);
        max_inline = max_inline
            + LayoutUnit::from_f32(max_content_block.to_f32() * block_axis_margin_fraction);
    }

    // Add container border + padding.
    // CSS Sizing 3 §4: an inline-axis min-content contribution cannot
    // exceed the corresponding max-content contribution. Negative margins
    // and other post-processing above may otherwise invert the endpoints.
    if IntrinsicAxisMapping::for_style(style)
        .writing_direction
        .is_horizontal()
    {
        max_inline = max_inline.max_of(min_inline);
    }

    apply_size_containment(
        style,
        IntrinsicSizes {
            min_content_inline_size: min_inline + bp_inline,
            max_content_inline_size: max_inline + bp_inline,
            min_content_block_size: min_content_block + bp_block,
            max_content_block_size: max_content_block + bp_block,
        },
    )
}

fn apply_size_containment(style: &ComputedStyle, mut sizes: IntrinsicSizes) -> IntrinsicSizes {
    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    if crate::containment::physical_width_is_contained(style) {
        let width = crate::containment::physical_width_fallback(style)
            + border.inline_sum()
            + padding.inline_sum();
        sizes.min_content_inline_size = width;
        sizes.max_content_inline_size = width;
    }
    if crate::containment::physical_height_is_contained(style) {
        let height = crate::containment::physical_height_fallback(style)
            + border.block_sum()
            + padding.block_sum();
        sizes.min_content_block_size = height;
        sizes.max_content_block_size = height;
    }
    sizes
}

/// Compute intrinsic sizes for flex containers.
///
/// CSS Flexbox §9.9: Flex container intrinsic main size is determined by
/// item flex-base sizes; cross size by item cross sizes.
///
/// For row flex (is_column = false):
///   - min-content inline: largest item min-content contribution (single-line)
///     or sum if no wrapping possible
///   - max-content inline: sum of all item max-content contributions + gaps
///   - min/max-content block: max of item cross sizes
///
/// For column flex (is_column = true): swap axes.
fn append_intrinsic_flex_children(doc: &Document, parent: NodeId, output: &mut Vec<NodeId>) {
    for child_id in doc.children(parent) {
        let child = doc.node(child_id);
        if child.style.display == openui_style::Display::Contents
            && !child.style.position.is_absolutely_positioned()
        {
            append_intrinsic_flex_children(doc, child_id, output);
        } else {
            output.push(child_id);
        }
    }
}

fn compute_flex_intrinsic_sizes(
    doc: &Document,
    node_id: NodeId,
    style: &ComputedStyle,
    definite_border_box_block_size: Option<LayoutUnit>,
) -> IntrinsicSizes {
    let axes = IntrinsicAxisMapping::for_style(style);
    let writing_direction = axes.writing_direction;
    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let logical_border = border.to_logical(writing_direction);
    let logical_padding = padding.to_logical(writing_direction);
    let bp_inline = logical_border.inline_sum() + logical_padding.inline_sum();
    let bp_block = logical_border.block_sum() + logical_padding.block_sum();

    let is_column = style.flex_direction == openui_style::FlexDirection::Column
        || style.flex_direction == openui_style::FlexDirection::ColumnReverse;
    let main_axis_is_horizontal = writing_direction.is_horizontal() ^ is_column;
    let is_wrap = style.flex_wrap != openui_style::FlexWrap::Nowrap;

    // Resolve gaps
    let main_gap = if is_column {
        style
            .row_gap
            .as_ref()
            .map(|g| {
                resolve_length(
                    g,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                )
            })
            .unwrap_or(LayoutUnit::zero())
    } else {
        style
            .column_gap
            .as_ref()
            .map(|g| {
                resolve_length(
                    g,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                )
            })
            .unwrap_or(LayoutUnit::zero())
    };
    let cross_gap = if is_column {
        style
            .column_gap
            .as_ref()
            .map(|g| {
                resolve_length(
                    g,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                )
            })
            .unwrap_or(LayoutUnit::zero())
    } else {
        style
            .row_gap
            .as_ref()
            .map(|g| {
                resolve_length(
                    g,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                )
            })
            .unwrap_or(LayoutUnit::zero())
    };

    // Per-item data needed for column-wrap wrapping simulation.
    struct ItemData {
        main_min: LayoutUnit,
        main_max: LayoutUnit,
        cross_min: LayoutUnit,
        cross_max: LayoutUnit,
    }

    let mut items: Vec<ItemData> = Vec::new();
    let mut sum_main_min = LayoutUnit::zero();
    let mut sum_main_max = LayoutUnit::zero();
    let mut max_main_min = LayoutUnit::zero();
    let mut max_cross_min = LayoutUnit::zero();
    let mut max_cross_max = LayoutUnit::zero();

    // Resolve the container's definite cross size (if any) for aspect-ratio children.
    // Row flex: cross = block (height); Column flex: cross = inline (width).
    let definite_content_block =
        definite_border_box_block_size.map(|size| (size - bp_block).clamp_negative_to_zero());
    let container_definite_cross = {
        if !is_column {
            if let Some(size) = definite_content_block {
                size
            } else {
                let cross_prop = axes.cross_size(style, is_column);
                if !cross_prop.is_auto() && cross_prop.is_fixed() {
                    let val = resolve_length(
                        cross_prop,
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                    );
                    if style.box_sizing == BoxSizing::BorderBox {
                        (val - bp_block).clamp_negative_to_zero()
                    } else {
                        val
                    }
                } else {
                    LayoutUnit::zero()
                }
            }
        } else {
            let cross_prop = axes.cross_size(style, is_column);
            if !cross_prop.is_auto() && cross_prop.is_fixed() {
                let val = resolve_length(
                    cross_prop,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
                let content_val = if style.box_sizing == BoxSizing::BorderBox {
                    if is_column {
                        (val - bp_inline).clamp_negative_to_zero()
                    } else {
                        (val - bp_block).clamp_negative_to_zero()
                    }
                } else {
                    val
                };
                content_val
            } else {
                LayoutUnit::zero()
            }
        }
    };
    let container_definite_main = {
        if is_column {
            if let Some(size) = definite_content_block {
                size
            } else {
                let main_prop = axes.main_size(style, is_column);
                if main_prop.is_fixed() {
                    let value = resolve_length(
                        main_prop,
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                    );
                    if style.box_sizing == BoxSizing::BorderBox {
                        (value - bp_block).clamp_negative_to_zero()
                    } else {
                        value
                    }
                } else {
                    LayoutUnit::zero()
                }
            }
        } else {
            let main_prop = axes.main_size(style, is_column);
            if main_prop.is_fixed() {
                let value = resolve_length(
                    main_prop,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
                if style.box_sizing == BoxSizing::BorderBox {
                    (value - if is_column { bp_block } else { bp_inline }).clamp_negative_to_zero()
                } else {
                    value
                }
            } else {
                LayoutUnit::zero()
            }
        }
    };

    let mut flattened_children = Vec::new();
    append_intrinsic_flex_children(doc, node_id, &mut flattened_children);
    let mut ordered_children: Vec<(usize, NodeId, i32)> = flattened_children
        .into_iter()
        .enumerate()
        .map(|(index, child_id)| (index, child_id, doc.node(child_id).style.order))
        .collect();
    ordered_children.sort_by_key(|&(index, _, order)| (order, index));

    // Contiguous text exposed through `display:contents` forms one anonymous
    // flex item. Keep that same grouping during shrink-to-fit sizing as final
    // flex layout, otherwise an inline column container is measured from only
    // one generated character and wraps the run vertically.
    let mut grouped_children: Vec<Vec<NodeId>> = Vec::new();
    for (_, child_id, _) in ordered_children {
        let is_text = matches!(
            doc.node(child_id).tag,
            openui_dom::ElementTag::Text | openui_dom::ElementTag::Break
        );
        if is_text
            && grouped_children.last().is_some_and(|group| {
                group.iter().all(|id| {
                    matches!(
                        doc.node(*id).tag,
                        openui_dom::ElementTag::Text | openui_dom::ElementTag::Break
                    )
                })
            })
        {
            grouped_children.last_mut().unwrap().push(child_id);
        } else {
            grouped_children.push(vec![child_id]);
        }
    }

    for child_group in grouped_children {
        let child_id = child_group[0];
        let child_style = &doc.node(child_id).style;

        if child_style.display == openui_style::Display::None
            || child_style.position.is_absolutely_positioned()
        {
            continue;
        }

        let child_axes = IntrinsicAxisMapping::for_style(child_style);
        let child_border = resolve_border(child_style);
        let child_padding = resolve_padding(child_style, LayoutUnit::zero());
        let child_margin = resolve_margins(child_style, LayoutUnit::zero());

        // Preserve the established horizontal contribution path exactly.
        // Non-horizontal flex containers need an explicit logical projection:
        // strip the legacy physical edge additions, restore each child's
        // logical decorations, then project them to the container axes.
        let mut child_sizes = compute_child_intrinsic_contribution(doc, child_id);
        for additional_id in child_group.iter().skip(1) {
            let additional = compute_child_intrinsic_contribution(doc, *additional_id);
            child_sizes.min_content_inline_size =
                child_sizes.min_content_inline_size + additional.min_content_inline_size;
            child_sizes.max_content_inline_size =
                child_sizes.max_content_inline_size + additional.max_content_inline_size;
            child_sizes.min_content_block_size = child_sizes
                .min_content_block_size
                .max_of(additional.min_content_block_size);
            child_sizes.max_content_block_size = child_sizes
                .max_content_block_size
                .max_of(additional.max_content_block_size);
        }
        if child_group.len() == 1 && child_style.display.is_table_wrapper() {
            // A table's specified inline size is a minimum for its used
            // wrapper width. Generic child contribution code treats a fixed
            // width as replacing intrinsic sizes, so restore the table-grid
            // min-content floor before summing an intrinsic flex container.
            let table_sizes = compute_intrinsic_block_sizes(doc, child_id);
            child_sizes.min_content_inline_size = child_sizes
                .min_content_inline_size
                .max_of(table_sizes.min_content_inline_size);
            child_sizes.max_content_inline_size = child_sizes
                .max_content_inline_size
                .max_of(table_sizes.min_content_inline_size);
        }
        if !writing_direction.is_horizontal() {
            let old_inline_edges =
                child_border.inline_sum() + child_padding.inline_sum() + child_margin.inline_sum();
            let old_block_edges =
                child_border.block_sum() + child_padding.block_sum() + child_margin.block_sum();
            child_sizes.min_content_inline_size =
                (child_sizes.min_content_inline_size - old_inline_edges).clamp_negative_to_zero();
            child_sizes.max_content_inline_size =
                (child_sizes.max_content_inline_size - old_inline_edges).clamp_negative_to_zero();
            child_sizes.min_content_block_size =
                (child_sizes.min_content_block_size - old_block_edges).clamp_negative_to_zero();
            child_sizes.max_content_block_size =
                (child_sizes.max_content_block_size - old_block_edges).clamp_negative_to_zero();

            let child_logical_border = child_border.to_logical(child_axes.writing_direction);
            let child_logical_padding = child_padding.to_logical(child_axes.writing_direction);
            let child_bp_inline =
                child_logical_border.inline_sum() + child_logical_padding.inline_sum();
            let child_bp_block =
                child_logical_border.block_sum() + child_logical_padding.block_sum();
            child_sizes.min_content_inline_size =
                child_sizes.min_content_inline_size + child_bp_inline;
            child_sizes.max_content_inline_size =
                child_sizes.max_content_inline_size + child_bp_inline;
            child_sizes.min_content_block_size =
                child_sizes.min_content_block_size + child_bp_block;
            child_sizes.max_content_block_size =
                child_sizes.max_content_block_size + child_bp_block;

            if child_axes.writing_direction.is_horizontal() != writing_direction.is_horizontal() {
                child_sizes = IntrinsicSizes {
                    min_content_inline_size: child_sizes.min_content_block_size,
                    max_content_inline_size: child_sizes.max_content_block_size,
                    min_content_block_size: child_sizes.min_content_inline_size,
                    max_content_block_size: child_sizes.max_content_inline_size,
                };
            }

            let logical_margin = child_margin.to_logical(writing_direction);
            let margin_inline = logical_margin.inline_sum();
            let margin_block = logical_margin.block_sum();
            child_sizes.min_content_inline_size =
                child_sizes.min_content_inline_size + margin_inline;
            child_sizes.max_content_inline_size =
                child_sizes.max_content_inline_size + margin_inline;
            child_sizes.min_content_block_size = child_sizes.min_content_block_size + margin_block;
            child_sizes.max_content_block_size = child_sizes.max_content_block_size + margin_block;

            // A definite physical size overrides the contribution in whichever
            // logical container axis owns that physical dimension.
            let fixed_border_box = |size: &Length, bp: LayoutUnit| -> Option<LayoutUnit> {
                size.is_fixed().then(|| {
                    let value = resolve_length(
                        size,
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                    );
                    if child_style.box_sizing == BoxSizing::BorderBox {
                        value.max_of(bp)
                    } else {
                        value + bp
                    }
                })
            };
            let child_physical_bp_inline = child_border.inline_sum() + child_padding.inline_sum();
            let child_physical_bp_block = child_border.block_sum() + child_padding.block_sum();
            let physical_width = fixed_border_box(&child_style.width, child_physical_bp_inline);
            let physical_height = fixed_border_box(&child_style.height, child_physical_bp_block);
            let (fixed_inline, fixed_block) = (physical_height, physical_width);
            if let Some(size) = fixed_inline {
                child_sizes.min_content_inline_size = size + margin_inline;
                child_sizes.max_content_inline_size = size + margin_inline;
            }
            if let Some(size) = fixed_block {
                child_sizes.min_content_block_size = size + margin_block;
                child_sizes.max_content_block_size = size + margin_block;
            }
        }

        // A row flex item's used cross size can be definite while the
        // container is being intrinsically measured: from its own percentage
        // height, a percentage min/max constraint, or stretch. Propagate that
        // border-box size through the item's descendant chain so percentage-
        // sized replaced content contributes its transferred ratio width
        // instead of its unrelated natural width.
        let resolved_alignment = {
            let mut position = child_style.align_self.position;
            if position == openui_style::ItemPosition::Auto {
                position = style.align_items.position;
            }
            if position == openui_style::ItemPosition::Normal {
                position = openui_style::ItemPosition::Stretch;
            }
            position
        };
        let has_cross_auto_margin = if is_column {
            child_style.margin_left.is_auto() || child_style.margin_right.is_auto()
        } else {
            child_style.margin_top.is_auto() || child_style.margin_bottom.is_auto()
        };
        if !is_column
            && writing_direction.is_horizontal()
            && container_definite_cross > LayoutUnit::zero()
            && has_block_dependent_replaced_descendant(doc, child_id)
        {
            let child_block_edges = child_border.block_sum() + child_padding.block_sum();
            let to_border_box = |raw: LayoutUnit| {
                if child_style.box_sizing == BoxSizing::BorderBox {
                    raw.max_of(child_block_edges)
                } else {
                    raw + child_block_edges
                }
            };
            let resolve_cross_bound = |length: &Length| {
                (!length.is_auto() && !length.is_none() && !length.is_content_or_intrinsic())
                    .then(|| {
                        resolve_length(
                            length,
                            container_definite_cross,
                            openui_geometry::INDEFINITE_SIZE,
                            openui_geometry::INDEFINITE_SIZE,
                        )
                    })
                    .filter(|value| !value.is_indefinite())
                    .map(to_border_box)
            };
            let specified = resolve_cross_bound(&child_style.height);
            let stretched = (specified.is_none()
                && child_style.height.is_auto()
                && resolved_alignment == openui_style::ItemPosition::Stretch
                && !has_cross_auto_margin)
                .then(|| {
                    (container_definite_cross - child_margin.block_sum()).clamp_negative_to_zero()
                });
            let constrained = if specified.is_none() && stretched.is_none() {
                let natural = (child_sizes.max_content_block_size - child_margin.block_sum())
                    .clamp_negative_to_zero();
                let min =
                    resolve_cross_bound(&child_style.min_height).unwrap_or(LayoutUnit::zero());
                let max = resolve_cross_bound(&child_style.max_height).unwrap_or(LayoutUnit::max());
                (min > LayoutUnit::zero() || max < LayoutUnit::max())
                    .then(|| natural.clamp(min, max))
            } else {
                None
            };
            if let Some(item_border_box) = specified.or(stretched).or(constrained) {
                let measured = compute_intrinsic_inline_sizes_with_block_size(
                    doc,
                    child_id,
                    item_border_box,
                    LayoutUnit::zero(),
                );
                child_sizes.min_content_inline_size = measured.min + child_margin.inline_sum();
                child_sizes.max_content_inline_size = measured.max + child_margin.inline_sum();
            }
        }
        if is_column
            && writing_direction.is_horizontal()
            && container_definite_main > LayoutUnit::zero()
            && crate::intrinsic_sizing::has_block_dependent_replaced_descendant(doc, child_id)
        {
            let child_block_edges = child_border.block_sum() + child_padding.block_sum();
            let child_block_margin = child_margin.block_sum();
            let child_block_size = if child_style.height.is_percent()
                || child_style.height.length_type() == LengthType::Calculated
            {
                let resolved = resolve_length(
                    &child_style.height,
                    container_definite_main,
                    LayoutUnit::zero(),
                    container_definite_main,
                );
                if child_style.box_sizing == BoxSizing::BorderBox {
                    resolved.max_of(child_block_edges)
                } else {
                    resolved + child_block_edges
                }
            } else {
                (container_definite_main - child_block_margin).clamp_negative_to_zero()
            };
            let measured = compute_intrinsic_inline_sizes_with_block_size(
                doc,
                child_id,
                child_block_size,
                LayoutUnit::zero(),
            );
            child_sizes.min_content_inline_size = measured.min + child_margin.inline_sum();
            child_sizes.max_content_inline_size = measured.max + child_margin.inline_sum();
        }

        // Determine main-axis and cross-axis contributions
        let (mut main_min, mut main_max, cross_min, cross_max) = if is_column {
            (
                child_sizes.min_content_block_size,
                child_sizes.max_content_block_size,
                child_sizes.min_content_inline_size,
                child_sizes.max_content_inline_size,
            )
        } else {
            (
                child_sizes.min_content_inline_size,
                child_sizes.max_content_inline_size,
                child_sizes.min_content_block_size,
                child_sizes.max_content_block_size,
            )
        };

        // CSS Flexbox §9.9.1: When the flex container has a definite cross size
        // and a child has aspect-ratio with auto main size, the child's main-axis
        // contribution should be derived from the definite cross size via AR.
        // This handles cases like `inline-flex; height:100px` with child `aspect-ratio:1/1`.
        let natural_ratio = doc
            .node(child_id)
            .replaced
            .and_then(|content| content.intrinsic_ratio)
            .filter(|(width, height)| *width > 0.0 && *height > 0.0);
        let effective_ratio = child_style
            .aspect_ratio
            .as_ref()
            .and_then(|ratio| {
                if ratio.auto_flag {
                    natural_ratio.or_else(|| {
                        (ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0).then_some(ratio.ratio)
                    })
                } else {
                    (ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0).then_some(ratio.ratio)
                }
            })
            .or(natural_ratio);
        if let Some(ratio) = effective_ratio {
            if ratio.0 != 0.0 && ratio.1 != 0.0 {
                let cross_size_prop = axes.cross_size(child_style, is_column);
                let main_size_prop = axes.main_size(child_style, is_column);
                // Only apply when cross size is definite (from container or child)
                // and main size is auto
                if main_size_prop.is_auto() {
                    let definite_cross = if !cross_size_prop.is_auto() && cross_size_prop.is_fixed()
                    {
                        Some(resolve_length(
                            cross_size_prop,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        ))
                    } else if container_definite_cross > LayoutUnit::zero() {
                        // Container's definite cross size (stretch)
                        Some(container_definite_cross)
                    } else {
                        None
                    };
                    if let Some(cross_val) = definite_cross {
                        let child_bp = {
                            let b = resolve_border(child_style);
                            let p = resolve_padding(child_style, LayoutUnit::zero());
                            if main_axis_is_horizontal {
                                (
                                    b.top + b.bottom + p.top + p.bottom,
                                    b.left + b.right + p.left + p.right,
                                )
                            } else {
                                (
                                    b.left + b.right + p.left + p.right,
                                    b.top + b.bottom + p.top + p.bottom,
                                )
                            }
                        };
                        let content_cross = (cross_val - child_bp.0).clamp_negative_to_zero();
                        let transferred = if main_axis_is_horizontal {
                            LayoutUnit::from_f32(content_cross.to_f32() * ratio.0 / ratio.1)
                        } else {
                            LayoutUnit::from_f32(content_cross.to_f32() * ratio.1 / ratio.0)
                        };
                        let ar_main = transferred + child_bp.1;
                        main_min = main_min.max_of(ar_main);
                        main_max = main_max.max_of(ar_main);
                    }
                }
            }
        }

        // Check for explicit flex-basis. CSS Flexbox 9.9.1 compares the
        // item's *outer* flex base size with its outer min/max-content
        // contribution, then moves toward that contribution only when the
        // corresponding grow/shrink factor permits it. A fixed basis is not
        // itself the intrinsic contribution, and `min-size:auto` does not
        // clamp this intrinsic flex-fraction calculation.
        let flex_basis = &child_style.flex_basis;
        let main_contribution_min;
        let main_contribution_max;

        if writing_direction.is_horizontal() {
            let logical_child_border = child_border.to_logical(writing_direction);
            let logical_child_padding = child_padding.to_logical(writing_direction);
            let logical_child_margin = child_margin.to_logical(writing_direction);
            let (main_bp, main_margin) = if is_column {
                (
                    logical_child_border.block_sum() + logical_child_padding.block_sum(),
                    logical_child_margin.block_sum(),
                )
            } else {
                (
                    logical_child_border.inline_sum() + logical_child_padding.inline_sum(),
                    logical_child_margin.inline_sum(),
                )
            };
            let to_outer = |length: &Length| {
                let value = resolve_length(
                    length,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
                if child_style.box_sizing == BoxSizing::BorderBox {
                    value.max_of(main_bp) + main_margin
                } else {
                    value + main_bp + main_margin
                }
            };

            // Recursive block intrinsic sizing deliberately measures the
            // contents before the child's preferred size is applied. A
            // definite preferred main size replaces that axis's content
            // contribution; otherwise the raw content contribution is used.
            let replaced_with_cyclic_max = doc.node(child_id).replaced.is_some()
                && !is_column
                && child_style.width.is_auto()
                && child_style.min_width.is_auto()
                && (child_style.max_width.is_percent()
                    || child_style.max_width.length_type() == LengthType::Calculated);
            let (raw_min, raw_max) = if doc.node(child_id).tag == ElementTag::Text {
                // Anonymous flex text already has its correct min/max-content
                // pair in `child_sizes`. Re-entering block intrinsic sizing
                // collapses max-content to the longest word.
                (main_min, main_max)
            } else {
                let raw = compute_intrinsic_block_sizes(doc, child_id);
                if is_column {
                    (
                        raw.min_content_block_size + main_margin,
                        raw.max_content_block_size + main_margin,
                    )
                } else {
                    (
                        if replaced_with_cyclic_max {
                            // The percentage maximum is cyclic while the flex
                            // container is intrinsically sized. Preserve the
                            // zero min-content endpoint computed above instead
                            // of replacing it with the resource's natural width.
                            main_min
                        } else {
                            raw.min_content_inline_size + main_margin
                        },
                        raw.max_content_inline_size + main_margin,
                    )
                }
            };
            let preferred = axes.main_size(child_style, is_column);
            let preferred_outer = preferred.is_fixed().then(|| to_outer(preferred));
            let flex_grow = child_style.flex_grow.max(0.0);
            let flex_shrink = child_style.flex_shrink.max(0.0);
            let target_min = if is_column && flex_basis.is_fixed() {
                preferred_outer.map_or(raw_min, |size| raw_min.min_of(size))
            } else {
                preferred_outer.unwrap_or(raw_min)
            };
            let target_max = if is_column && flex_basis.is_fixed() {
                preferred_outer.map_or(raw_max, |size| raw_max.min_of(size))
            } else {
                preferred_outer.unwrap_or(raw_max)
            };
            let outer_flex_base = if flex_basis.is_fixed() {
                to_outer(flex_basis)
            } else if let Some(preferred) = preferred_outer {
                preferred
            } else {
                raw_max
            };
            let flex_toward = |target: LayoutUnit| {
                if (target > outer_flex_base && flex_grow > 0.0)
                    || (target < outer_flex_base
                        && flex_shrink > 0.0
                        && !(is_column && flex_basis.is_fixed()))
                {
                    target
                } else {
                    outer_flex_base
                }
            };
            main_contribution_min = if flex_basis.is_auto() {
                target_min
            } else {
                flex_toward(target_min)
            };
            main_contribution_max = flex_toward(target_max);
        } else if flex_basis.is_fixed() {
            let basis = resolve_length(
                flex_basis,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            main_contribution_min = basis;
            main_contribution_max = basis;
        } else {
            main_contribution_min = main_min;
            main_contribution_max = main_max;
        }

        // CSS Flexbox §9.9.1: Clamp item contributions by main-axis min/max constraints.
        let min_main_prop = axes.min_main_size(child_style, is_column);
        let max_main_prop = axes.max_main_size(child_style, is_column);
        let logical_margin = child_margin.to_logical(writing_direction);
        let main_margin = if is_column {
            logical_margin.block_sum()
        } else {
            logical_margin.inline_sum()
        };
        // Contributions are outer margin-box sizes. The border box cannot
        // become negative, but its margin-box contribution can when an item
        // has a negative main-axis margin.
        let clamp_border_box =
            |outer: LayoutUnit| (outer - main_margin).clamp_negative_to_zero() + main_margin;
        let clamped_min_val = if !min_main_prop.is_auto() && min_main_prop.is_fixed() {
            resolve_length(
                min_main_prop,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            )
        } else if min_main_prop.is_auto() && !child_style.is_scroll_container() {
            if flex_basis.is_fixed() {
                // The automatic minimum is the content-based minimum, capped
                // by a specified size suggestion. A definite preferred width
                // on an empty item must not override `flex-basis: 0` during a
                // shrink-to-fit flex container's intrinsic sizing.
                let content_sizes = compute_intrinsic_block_sizes(doc, child_id);
                let logical_margin = child_margin.to_logical(writing_direction);
                let content_main = if is_column {
                    content_sizes.min_content_block_size + logical_margin.block_sum()
                } else {
                    content_sizes.min_content_inline_size + logical_margin.inline_sum()
                };
                clamp_border_box(main_min.min_of(content_main))
            } else {
                clamp_border_box(main_min)
            }
        } else {
            main_margin
        };
        let clamped_max_val = if !max_main_prop.is_none() && max_main_prop.is_fixed() {
            resolve_length(
                max_main_prop,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            )
        } else {
            LayoutUnit::from_i32(33554431) // Max
        };
        let main_contribution_min = main_contribution_min
            .max_of(clamped_min_val)
            .min_of(clamped_max_val);
        let main_contribution_max = main_contribution_max
            .max_of(clamped_min_val)
            .min_of(clamped_max_val);

        sum_main_min = sum_main_min + main_contribution_min;
        sum_main_max = sum_main_max + main_contribution_max;
        max_main_min = max_main_min.max_of(main_contribution_min);
        max_cross_min = max_cross_min.max_of(cross_min);
        max_cross_max = max_cross_max.max_of(cross_max);

        items.push(ItemData {
            main_min: main_contribution_min,
            main_max: main_contribution_max,
            cross_min,
            cross_max,
        });
    }

    let item_count = items.len();

    // Add gaps between items
    let gap_count = if item_count > 1 { item_count - 1 } else { 0 };
    let total_main_gap = main_gap * gap_count as i32;

    // CSS Flexbox §9.9.1:
    // For single-line: min-content = sum of clamped flex-base sizes
    // For multi-line (wrap): min-content = largest item min-content contribution
    let (min_main, max_main) = if is_wrap && is_column {
        (sum_main_min + total_main_gap, sum_main_max + total_main_gap)
    } else if is_wrap {
        (max_main_min, sum_main_max + total_main_gap)
    } else {
        (sum_main_min + total_main_gap, sum_main_max + total_main_gap)
    };
    // Negative item margins may pull the summed max-content contribution
    // below the min-content contribution. Intrinsic max-content still cannot
    // be smaller than min-content (as in Blink's flex intrinsic sizing).
    let max_main = max_main.max_of(min_main);

    // For column+wrap, the cross-axis (inline) size depends on wrapping.
    // When the container has a definite main-axis constraint (height/max-height),
    // simulate wrapping to determine the sum of column widths.
    let (min_cross_total, max_cross_total) = if is_column && is_wrap && !items.is_empty() {
        let main_constraint = {
            let mut c = definite_content_block.unwrap_or_else(|| LayoutUnit::from_i32(33554431));
            let main_size = axes.main_size(style, true);
            let max_main_size = axes.max_main_size(style, true);
            if main_size.is_fixed() {
                let h = resolve_length(
                    main_size,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
                let ch = if style.box_sizing == BoxSizing::BorderBox {
                    (h - bp_block).clamp_negative_to_zero()
                } else {
                    h
                };
                c = c.min_of(ch);
            }
            if !max_main_size.is_none() && max_main_size.is_fixed() {
                let mh = resolve_length(
                    max_main_size,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
                let cmh = if style.box_sizing == BoxSizing::BorderBox {
                    (mh - bp_block).clamp_negative_to_zero()
                } else {
                    mh
                };
                c = c.min_of(cmh);
            }
            c
        };

        if main_constraint < LayoutUnit::from_i32(33554431) {
            // Simulate wrapping to compute cross-axis totals
            let simulate = |use_max: bool| -> LayoutUnit {
                let mut total_cross = LayoutUnit::zero();
                let mut line_main = LayoutUnit::zero();
                let mut line_cross = LayoutUnit::zero();
                let mut line_count: usize = 0;
                let mut items_in_line: usize = 0;

                for item in &items {
                    let im = if use_max {
                        item.main_max
                    } else {
                        item.main_min
                    };
                    let ic = if use_max {
                        item.cross_max
                    } else {
                        item.cross_min
                    };
                    let new_main = if items_in_line > 0 {
                        line_main + main_gap + im
                    } else {
                        im
                    };

                    if items_in_line > 0 && new_main > main_constraint {
                        total_cross = total_cross + line_cross;
                        line_count += 1;
                        line_main = im;
                        line_cross = ic;
                        items_in_line = 1;
                    } else {
                        line_main = new_main;
                        line_cross = line_cross.max_of(ic);
                        items_in_line += 1;
                    }
                }
                if items_in_line > 0 {
                    total_cross = total_cross + line_cross;
                    line_count += 1;
                }
                if line_count > 1 {
                    total_cross = total_cross + cross_gap * (line_count - 1) as i32;
                }
                total_cross
            };
            (max_cross_min, simulate(true))
        } else {
            (max_cross_min, max_cross_max)
        }
    } else {
        (max_cross_min, max_cross_max)
    };

    // Convert back to inline/block coordinates
    let (min_inline, max_inline, min_block, max_block) = if is_column {
        (min_cross_total, max_cross_total, min_main, max_main)
    } else {
        (min_main, max_main, max_cross_min, max_cross_max)
    };

    IntrinsicSizes {
        min_content_inline_size: min_inline + bp_inline,
        max_content_inline_size: max_inline + bp_inline,
        min_content_block_size: min_block + bp_block,
        max_content_block_size: max_block + bp_block,
    }
}

/// Compute the intrinsic size contribution of a single child.
///
/// This accounts for the child's own intrinsic sizes plus its margin box.
/// For inline-level children (text, inline), uses inline intrinsic sizing.
pub fn compute_child_intrinsic_contribution(doc: &Document, child_id: NodeId) -> IntrinsicSizes {
    let child_style = &doc.node(child_id).style;
    let child_tag = doc.node(child_id).tag;

    // HTML's WBR is a soft wrap opportunity, not a principal CSS box. Even
    // when float blockification changes its computed display, it contributes
    // no intrinsic width or height of its own.
    if child_tag == ElementTag::WordBreak {
        return IntrinsicSizes::zero();
    }

    // Resolve child margins (percentages resolve to zero for intrinsic sizing).
    let margin = resolve_margins(child_style, LayoutUnit::zero());
    let margin_inline = margin.inline_sum();
    let margin_block = margin.block_sum();

    // For text nodes and inline-level elements, use inline intrinsic sizing.
    // Block-size contribution requires running inline layout at the given
    // width to determine how many lines wrap.
    let child_intrinsic = if doc.node(child_id).replaced.is_some() || is_replaced_element(child_tag)
    {
        // Replaced elements are atomic in both axes even when their computed
        // outer display is inline-level. Their natural block size therefore
        // participates in Grid row sizing; treating an inline-block control
        // as an ordinary inline item incorrectly erases that contribution.
        compute_replaced_intrinsic_sizes_for_node(doc, child_id)
    } else if child_tag == ElementTag::Text || is_inline_level(child_style) {
        let inline_sizes = compute_intrinsic_inline_sizes(doc, child_id);
        IntrinsicSizes {
            min_content_inline_size: inline_sizes.min,
            max_content_inline_size: inline_sizes.max,
            // Block-size is zero for individual inline items at this level;
            // the container's block-size is computed from inline layout
            // in compute_intrinsic_block_sizes.
            min_content_block_size: LayoutUnit::zero(),
            max_content_block_size: LayoutUnit::zero(),
        }
    } else {
        // Block-level children: recursive block intrinsic sizing.
        compute_intrinsic_block_sizes(doc, child_id)
    };

    // Apply explicit width if set (CSS Sizing 3 §5.1 — definite sizes override).
    let min_inline =
        apply_size_override_inline(child_style, child_intrinsic.min_content_inline_size);
    let max_inline =
        apply_size_override_inline(child_style, child_intrinsic.max_content_inline_size);

    // Apply min-width / max-width clamping.
    // Intrinsic keywords in min/max-width (e.g. `max-width: max-content`) must
    // resolve against the CONTENT-BASED intrinsic sizes, not the specified width.
    // CSS Sizing 3 §4: intrinsic sizes are determined by the content.
    // However, for AR-derived widths (auto width + aspect-ratio + fixed height),
    // the transferred size IS the intrinsic size per CSS Sizing 4 §5.1.
    let intrinsic_for_keywords =
        if child_style.width.length_type() == openui_geometry::LengthType::Fixed {
            // Explicit width: intrinsic keywords resolve against content-based sizes.
            (
                child_intrinsic.min_content_inline_size,
                child_intrinsic.max_content_inline_size,
            )
        } else {
            // Auto/intrinsic width (possibly AR-derived): use post-override values.
            (min_inline, max_inline)
        };
    let min_inline = apply_min_max_inline(child_style, min_inline, intrinsic_for_keywords, false);
    let max_inline = apply_min_max_inline(child_style, max_inline, intrinsic_for_keywords, true);

    // CSS Sizing 4 §5.1: For elements with AR and min-width:auto, the
    // automatic minimum in the ratio-dependent axis is the content-based
    // min-content. apply_size_override_inline may have replaced the
    // content-based intrinsic with the AR-derived size (which can be
    // smaller). Ensure the content min-content acts as a floor.
    let min_inline = if child_style.min_width.is_auto()
        && child_style.aspect_ratio.is_some()
        && (child_style.width.is_auto() || child_style.width.is_content_or_intrinsic())
    {
        min_inline.max_of(child_intrinsic.min_content_inline_size)
    } else {
        min_inline
    };
    let max_inline = if child_style.min_width.is_auto()
        && child_style.aspect_ratio.is_some()
        && (child_style.width.is_auto() || child_style.width.is_content_or_intrinsic())
    {
        max_inline.max_of(child_intrinsic.min_content_inline_size)
    } else {
        max_inline
    };

    // Apply explicit height if set. Compute separately for min-content
    // and max-content modes: at min-content width, wrapping content may
    // be taller than at max-content width (CSS Sizing 3 §5).
    let min_block_size =
        apply_size_override_block(child_style, child_intrinsic.min_content_block_size);
    let mut min_block_size = apply_min_max_block(child_style, min_block_size);
    let max_block_size =
        apply_size_override_block(child_style, child_intrinsic.max_content_block_size);
    let mut max_block_size = apply_min_max_block(child_style, max_block_size);
    if child_style.min_height.is_content_or_intrinsic() {
        if let Some(transferred) = aspect_ratio_block_from_fixed_width(child_style) {
            min_block_size = min_block_size.max_of(transferred);
            max_block_size = max_block_size.max_of(transferred);
        }
    }

    // CSS Sizing 4 §5.1: When height is auto and the element has aspect-ratio,
    // the intrinsic block size is the transferred size from the resolved inline
    // size. apply_size_override_block only handles Fixed width; when width is
    // auto/intrinsic, derive block sizes from the already-resolved inline sizes.
    let (min_block_size, max_block_size) =
        if child_style.height.is_auto() && !child_style.width.is_fixed() {
            if let Some(ref ar) = child_style.aspect_ratio {
                if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
                    let b = resolve_border(child_style);
                    let p = resolve_padding(child_style, LayoutUnit::zero());
                    let bp_inline = b.left + b.right + p.left + p.right;
                    let bp_block = b.top + b.bottom + p.top + p.bottom;

                    // Transfer inline → block through AR.
                    // CSS Sizing 4: bare ratio respects box-sizing; auto ratio
                    // maps through content-box.
                    let ar_uses_border_box =
                        !ar.auto_flag && child_style.box_sizing == BoxSizing::BorderBox;

                    let (transferred_min, transferred_max) = if ar_uses_border_box {
                        // AR maps border-box inline → border-box block
                        (
                            LayoutUnit::from_f32(min_inline.to_f32() * ar.ratio.1 / ar.ratio.0),
                            LayoutUnit::from_f32(max_inline.to_f32() * ar.ratio.1 / ar.ratio.0),
                        )
                    } else {
                        // AR maps content-box: content_w → content_h, then add bp
                        let content_min_w = (min_inline - bp_inline).clamp_negative_to_zero();
                        let content_max_w = (max_inline - bp_inline).clamp_negative_to_zero();
                        (
                            LayoutUnit::from_f32(content_min_w.to_f32() * ar.ratio.1 / ar.ratio.0)
                                + bp_block,
                            LayoutUnit::from_f32(content_max_w.to_f32() * ar.ratio.1 / ar.ratio.0)
                                + bp_block,
                        )
                    };

                    // Transferred size replaces content-based size, then clamp.
                    (
                        apply_min_max_block(child_style, transferred_min),
                        apply_min_max_block(child_style, transferred_max),
                    )
                } else {
                    (min_block_size, max_block_size)
                }
            } else {
                (min_block_size, max_block_size)
            }
        } else {
            (min_block_size, max_block_size)
        };

    // CSS Sizing 4 §5.1: Reverse AR transfer — when min-height (or explicit height)
    // inflates the block size beyond what was derived from inline, transfer back
    // through AR to inflate inline size.
    // The transferred values are still subject to the element's explicit max-width.
    let (min_inline, max_inline) = if let Some(ref ar) = child_style.aspect_ratio {
        if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 && child_style.width.is_auto() {
            let b = resolve_border(child_style);
            let p = resolve_padding(child_style, LayoutUnit::zero());
            let bp_inline = b.left + b.right + p.left + p.right;
            let bp_block = b.top + b.bottom + p.top + p.bottom;

            let ar_uses_border_box =
                !ar.auto_flag && child_style.box_sizing == BoxSizing::BorderBox;

            let reverse_transfer = |block_bb: LayoutUnit| -> LayoutUnit {
                if ar_uses_border_box {
                    // AR maps border-box block → border-box inline
                    LayoutUnit::from_f32(block_bb.to_f32() * ar.ratio.0 / ar.ratio.1)
                } else {
                    let content_h = (block_bb - bp_block).clamp_negative_to_zero();
                    LayoutUnit::from_f32(content_h.to_f32() * ar.ratio.0 / ar.ratio.1) + bp_inline
                }
            };

            let transferred_min = reverse_transfer(min_block_size);
            let transferred_max = reverse_transfer(max_block_size);

            // Clamp transferred sizes by a fixed max-width constraint (not intrinsic
            // keywords — those are already handled by apply_min_max_inline using the
            // post-AR intrinsic sizes).
            let max_w_cap = if !child_style.max_width.is_none()
                && !child_style.max_width.is_auto()
                && !child_style.max_width.is_content_or_intrinsic()
            {
                let max_w_raw = resolve_length(
                    &child_style.max_width,
                    openui_geometry::INDEFINITE_SIZE,
                    LayoutUnit::max(),
                    LayoutUnit::max(),
                );
                if max_w_raw < LayoutUnit::max() {
                    if child_style.box_sizing == BoxSizing::ContentBox {
                        max_w_raw + bp_inline
                    } else {
                        max_w_raw.max_of(bp_inline)
                    }
                } else {
                    LayoutUnit::max()
                }
            } else {
                LayoutUnit::max()
            };

            let clamped_min = min_inline.max_of(transferred_min).min_of(max_w_cap);
            let clamped_max = max_inline.max_of(transferred_max).min_of(max_w_cap);

            (clamped_min, clamped_max)
        } else {
            (min_inline, max_inline)
        }
    } else {
        (min_inline, max_inline)
    };

    let min_inline = if doc.node(child_id).replaced.is_some()
        && doc.node(child_id).form_control != Some(openui_dom::FormControlRole::Range)
        && child_style
            .direction
            .writing_direction(child_style.writing_mode)
            .is_horizontal()
        && child_style.width.is_auto()
        && child_style.min_width.is_auto()
        && (child_style.max_width.is_percent()
            || child_style.max_width.length_type() == LengthType::Calculated)
    {
        // A percentage maximum is cyclic in the inline intrinsic axis. The
        // replaced content therefore contributes only its decorations to the
        // min-content endpoint, while its natural size remains max-content.
        let border = resolve_border(child_style);
        let padding = resolve_padding(child_style, LayoutUnit::zero());
        border.inline_sum() + padding.inline_sum()
    } else if child_style.width.is_auto()
        && child_style.min_width.is_auto()
        && child_style.aspect_ratio.is_some()
        && child_style.is_scroll_container()
    {
        // The aspect-ratio reverse-transfer above contributes to max-content,
        // but a scroll container's automatic minimum contribution is zero.
        LayoutUnit::zero()
    } else {
        min_inline
    };

    IntrinsicSizes {
        min_content_inline_size: min_inline + margin_inline,
        max_content_inline_size: max_inline + margin_inline,
        min_content_block_size: min_block_size + margin_block,
        max_content_block_size: max_block_size + margin_block,
    }
}

// ── Inline-level intrinsic sizing ────────────────────────────────────────

/// Compute intrinsic inline sizes for inline-level content.
///
/// - Text: min-content = widest word, max-content = entire text line width.
/// - Replaced: intrinsic width from the element.
/// - Inline-block: recursive intrinsic sizing.
pub fn compute_intrinsic_inline_sizes(doc: &Document, node_id: NodeId) -> MinMaxSizes {
    let node = doc.node(node_id);
    let tag = node.tag;

    match tag {
        ElementTag::Text => {
            if let Some(ref text) = node.text {
                compute_text_intrinsic_sizes(doc, text, &node.style)
            } else {
                MinMaxSizes::zero()
            }
        }
        _ if node.replaced.is_some() || is_replaced_element(tag) => {
            let sizes = compute_replaced_intrinsic_sizes_for_node(doc, node_id);
            MinMaxSizes::new(sizes.min_content_inline_size, sizes.max_content_inline_size)
        }
        _ => {
            // Inline-block or other: recursive sizing.
            let sizes = compute_intrinsic_block_sizes(doc, node_id);
            MinMaxSizes::new(sizes.min_content_inline_size, sizes.max_content_inline_size)
        }
    }
}

/// Compute an element's intrinsic inline contribution in its own writing
/// mode. The older intrinsic-sizing entry points store physical width/height
/// in their inline/block slots; flex and atomic-inline layout need a stable
/// logical value before they cross back into physical fragment storage.
pub(crate) fn compute_logical_intrinsic_inline_sizes(
    doc: &Document,
    node_id: NodeId,
) -> MinMaxSizes {
    compute_logical_intrinsic_inline_sizes_impl(doc, node_id, None)
}

/// Compute an element's intrinsic inline contribution when its own logical
/// block-size is already definite. This is the orthogonal counterpart of
/// `compute_intrinsic_inline_sizes_with_block_size`: percentage-sized
/// replaced children can resolve in the block axis and transfer that used
/// size through their natural ratio into the container's inline contribution.
pub(crate) fn compute_logical_intrinsic_inline_sizes_with_block_size(
    doc: &Document,
    node_id: NodeId,
    border_box_block_size: LayoutUnit,
) -> MinMaxSizes {
    compute_logical_intrinsic_inline_sizes_impl(doc, node_id, Some(border_box_block_size))
}

/// Whether a descendant replaced box has a percentage/calc constraint in its
/// logical block axis. Only these subtrees need a definite-block intrinsic
/// probe; applying that probe to ordinary inline content would collapse a
/// max-content row into the maximum single child contribution.
pub(crate) fn has_block_dependent_replaced_descendant(doc: &Document, node_id: NodeId) -> bool {
    doc.children(node_id).any(|child_id| {
        let child = doc.node(child_id);
        if child.style.display == openui_style::Display::None
            || child.style.position.is_absolutely_positioned()
        {
            return false;
        }
        let direction = child
            .style
            .direction
            .writing_direction(child.style.writing_mode);
        let block_lengths = if direction.is_horizontal() {
            [
                &child.style.height,
                &child.style.min_height,
                &child.style.max_height,
            ]
        } else {
            [
                &child.style.width,
                &child.style.min_width,
                &child.style.max_width,
            ]
        };
        let depends_on_block = block_lengths
            .iter()
            .any(|length| length.is_percent() || length.length_type() == LengthType::Calculated);
        ((child.replaced.is_some() || is_replaced_element(child.tag)) && depends_on_block)
            || has_block_dependent_replaced_descendant(doc, child_id)
    })
}

/// Whether the block-dependent replaced box is below a non-replaced child.
///
/// A directly contained replaced box keeps its ordinary intrinsic inline
/// contribution while its percentage block size resolves during layout. A
/// nested box, however, participates in the containing block's intrinsic
/// walk and can transfer the now-definite block size through its ratio.
pub(crate) fn has_nested_block_dependent_replaced_descendant(
    doc: &Document,
    node_id: NodeId,
) -> bool {
    fn branch_uses_transferred_block_size(doc: &Document, child_id: NodeId) -> bool {
        let child = doc.node(child_id);
        if child.style.display == openui_style::Display::None
            || child.style.position.is_absolutely_positioned()
            || child.replaced.is_some()
            || is_replaced_element(child.tag)
        {
            return false;
        }
        let direction = child
            .style
            .direction
            .writing_direction(child.style.writing_mode);
        let block_lengths = if direction.is_horizontal() {
            [
                &child.style.height,
                &child.style.min_height,
                &child.style.max_height,
            ]
        } else {
            [
                &child.style.width,
                &child.style.min_width,
                &child.style.max_width,
            ]
        };
        let owns_definite_transfer = block_lengths
            .iter()
            .any(|length| length.is_percent() || length.length_type() == LengthType::Calculated)
            && has_block_dependent_replaced_descendant(doc, child_id);
        owns_definite_transfer
            || ((is_inline_level(&child.style)
                || child.style.display == openui_style::Display::Contents)
                && doc
                    .children(child_id)
                    .any(|nested| branch_uses_transferred_block_size(doc, nested)))
    }

    doc.children(node_id)
        .any(|child_id| branch_uses_transferred_block_size(doc, child_id))
}

fn compute_logical_intrinsic_inline_sizes_impl(
    doc: &Document,
    node_id: NodeId,
    definite_border_box_block_size: Option<LayoutUnit>,
) -> MinMaxSizes {
    let node = doc.node(node_id);
    let logical_contained_size = || {
        crate::containment::logical_inline_fallback(&node.style).map(|fallback| {
            let direction = IntrinsicAxisMapping::for_style(&node.style).writing_direction;
            let border = resolve_border(&node.style).to_logical(direction);
            let padding = resolve_padding(&node.style, LayoutUnit::zero()).to_logical(direction);
            let border_box = fallback + border.inline_sum() + padding.inline_sum();
            MinMaxSizes::new(border_box, border_box)
        })
    };
    if node.tag == ElementTag::Text {
        return node
            .text
            .as_deref()
            .map(|text| compute_text_intrinsic_sizes(doc, text, &node.style))
            .unwrap_or_else(MinMaxSizes::zero);
    }
    if node.replaced.is_some() || is_replaced_element(node.tag) {
        let physical = compute_replaced_intrinsic_sizes_for_node(doc, node_id);
        let direction = IntrinsicAxisMapping::for_style(&node.style).writing_direction;
        return if direction.is_horizontal() {
            MinMaxSizes::new(
                physical.min_content_inline_size,
                physical.max_content_inline_size,
            )
        } else {
            MinMaxSizes::new(
                physical.min_content_block_size,
                physical.max_content_block_size,
            )
        };
    }
    let direction = IntrinsicAxisMapping::for_style(&node.style).writing_direction;
    if direction.is_horizontal() {
        return compute_intrinsic_inline_sizes(doc, node_id);
    }
    if let Some(contained) = logical_contained_size() {
        return contained;
    }
    if node.style.display.is_table_wrapper() {
        // The table algorithm already reports contributions in the table's
        // own logical axes and includes border spacing, captions, and track
        // fixup. Falling through to the generic vertical block walk loses
        // the inline-axis border-spacing edges around the table grid.
        let sizes = crate::table::compute_table_intrinsic_sizes(doc, node_id);
        return MinMaxSizes::new(sizes.min_content_inline_size, sizes.max_content_inline_size);
    }

    let border = resolve_border(&node.style).to_logical(direction);
    let padding = resolve_padding(&node.style, LayoutUnit::zero()).to_logical(direction);
    let container_edges = border.inline_sum() + padding.inline_sum();
    let container_block_edges = border.block_sum() + padding.block_sum();
    let definite_content_block_size = definite_border_box_block_size
        .map(|size| (size - container_block_edges).clamp_negative_to_zero());

    let mut min_inline = LayoutUnit::zero();
    let mut max_inline = LayoutUnit::zero();
    let mut current_line_max = LayoutUnit::zero();

    let child_ids: Vec<NodeId> = doc.children(node_id).collect();
    for (child_index, child_id) in child_ids.iter().copied().enumerate() {
        let child = doc.node(child_id);
        let child_style = &child.style;
        if child_style.display == openui_style::Display::None
            || child_style.position.is_absolutely_positioned()
        {
            continue;
        }

        let is_preserved_newline_control = child.tag == ElementTag::Text
            && matches!(
                child_style.white_space,
                WhiteSpace::Pre
                    | WhiteSpace::PreWrap
                    | WhiteSpace::PreLine
                    | WhiteSpace::BreakSpaces
            )
            && child.text.as_deref().is_some_and(|text| {
                !text.is_empty() && text.chars().all(|ch| matches!(ch, '\n' | '\r'))
            });
        if child.tag == ElementTag::Break || is_preserved_newline_control {
            max_inline = max_inline.max_of(current_line_max);
            current_line_max = LayoutUnit::zero();
            continue;
        }

        let physical_border = resolve_border(child_style);
        let physical_padding = resolve_padding(child_style, LayoutUnit::zero());
        let logical_border = physical_border.to_logical(direction);
        let logical_padding = physical_padding.to_logical(direction);
        let child_edges = logical_border.inline_sum() + logical_padding.inline_sum();
        let logical_margin = resolve_margins(child_style, LayoutUnit::zero()).to_logical(direction);
        let margin_inline = logical_margin.inline_sum();

        let contribution = if child.tag == ElementTag::Text {
            child
                .text
                .as_deref()
                .map(|text| {
                    compute_text_intrinsic_sizes_impl(
                        doc,
                        text,
                        child_style,
                        is_interior_collapsible_whitespace(doc, &child_ids, child_index),
                    )
                })
                .unwrap_or_else(MinMaxSizes::zero)
        } else if let Some(content_block_size) = definite_content_block_size
            .filter(|_| child.replaced.is_some() || is_replaced_element(child.tag))
        {
            // The intrinsic walk normally measures a replaced child without a
            // containing-block opportunity. Once this vertical container's
            // physical width (logical block-size) is definite, a percentage
            // physical width on the child resolves and its natural ratio can
            // change the physical height (the parent's logical inline-size).
            let parent_space = crate::ConstraintSpace::for_block_child_with_writing_direction(
                openui_geometry::INDEFINITE_SIZE,
                content_block_size,
                openui_geometry::INDEFINITE_SIZE,
                content_block_size,
                true,
                direction,
            );
            let child_space = crate::block_child_constraint_space(
                &parent_space,
                child_style,
                openui_geometry::INDEFINITE_SIZE,
                content_block_size,
                openui_geometry::INDEFINITE_SIZE,
                content_block_size,
                true,
            );
            let fragment = crate::block::block_layout(doc, child_id, &child_space);
            let logical = WritingModeConverter::new(direction, PhysicalSize::zero())
                .to_logical_size(fragment.size);
            MinMaxSizes::new(logical.inline_size, logical.inline_size)
        } else if child_style.height.is_fixed() {
            let specified = resolve_length(
                &child_style.height,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            let border_box = if child_style.box_sizing == BoxSizing::BorderBox {
                specified.max_of(child_edges)
            } else {
                specified + child_edges
            };
            MinMaxSizes::new(border_box, border_box)
        } else {
            let child_direction = IntrinsicAxisMapping::for_style(child_style).writing_direction;
            if child_direction.is_horizontal() == direction.is_horizontal() {
                compute_logical_intrinsic_inline_sizes(doc, child_id)
            } else {
                let physical = compute_intrinsic_block_sizes(doc, child_id);
                MinMaxSizes::new(
                    physical.min_content_block_size,
                    physical.max_content_block_size,
                )
            }
        };

        let child_min = contribution.min + margin_inline;
        let child_max = contribution.max + margin_inline;
        min_inline = min_inline.max_of(child_min);
        if child_style.display.is_block_level() && child_style.float == openui_style::Float::None {
            // Normal-flow block children occupy separate block-axis rows. Their
            // inline intrinsic contribution is therefore the maximum child,
            // not the sum used by consecutive inline-level children on one
            // max-content line. This distinction is axis-independent: in a
            // vertical container, two vertical flex children still stack in
            // the horizontal block direction.
            max_inline = max_inline.max_of(current_line_max).max_of(child_max);
            current_line_max = LayoutUnit::zero();
        } else {
            current_line_max = current_line_max + child_max;
        }
    }

    max_inline = max_inline.max_of(current_line_max);
    MinMaxSizes::new(min_inline + container_edges, max_inline + container_edges)
}

/// Compute intrinsic inline sizes when the element has a definite border-box
/// block-size. This lets percentage-height descendants with aspect-ratio
/// contribute their transferred inline size to a min/max-content ancestor.
pub fn compute_intrinsic_inline_sizes_with_block_size(
    doc: &Document,
    node_id: NodeId,
    border_box_block_size: LayoutUnit,
    containing_inline_size: LayoutUnit,
) -> MinMaxSizes {
    let style = &doc.node(node_id).style;
    if doc.node(node_id).replaced.is_some() {
        // Replaced boxes have no descendant contribution to walk. Measure the
        // principal box itself with the definite block percentage basis so a
        // percentage height can transfer through its natural aspect ratio.
        let has_ratio = doc
            .node(node_id)
            .replaced
            .and_then(|content| content.intrinsic_ratio)
            .is_some_and(|(width, height)| width > 0.0 && height > 0.0)
            || style
                .aspect_ratio
                .as_ref()
                .is_some_and(|ratio| ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0);
        if !has_ratio {
            let intrinsic = compute_replaced_intrinsic_sizes_for_node(doc, node_id);
            return MinMaxSizes::new(
                intrinsic.min_content_inline_size,
                intrinsic.max_content_inline_size,
            );
        }
        let direction = style.direction.writing_direction(style.writing_mode);
        let probe_space = crate::ConstraintSpace::for_block_child_with_writing_direction(
            openui_geometry::INDEFINITE_SIZE,
            border_box_block_size,
            containing_inline_size,
            border_box_block_size,
            true,
            direction,
        );
        let fragment = crate::block::block_layout(doc, node_id, &probe_space);
        let logical = WritingModeConverter::new(direction, PhysicalSize::zero())
            .to_logical_size(fragment.size);
        return MinMaxSizes::new(logical.inline_size, logical.inline_size);
    }
    if style.display.is_flex()
        && style
            .direction
            .writing_direction(style.writing_mode)
            .is_horizontal()
    {
        let intrinsic =
            compute_flex_intrinsic_sizes(doc, node_id, style, Some(border_box_block_size));
        return MinMaxSizes::new(
            intrinsic.min_content_inline_size,
            intrinsic.max_content_inline_size,
        );
    }
    let border = resolve_border(style);
    let padding = resolve_padding(style, containing_inline_size);
    let bp_inline = border.inline_sum() + padding.inline_sum();
    let bp_block = border.block_sum() + padding.block_sum();
    let child_block_size = (border_box_block_size - bp_block).clamp_negative_to_zero();

    let mut min_inline = LayoutUnit::zero();
    let mut max_inline = LayoutUnit::zero();
    for child_id in doc.children(node_id) {
        let child_style = &doc.node(child_id).style;
        if child_style.display == openui_style::Display::None
            || child_style.position.is_absolutely_positioned()
        {
            continue;
        }
        let contribution =
            compute_child_intrinsic_contribution_with_block_size(doc, child_id, child_block_size);
        min_inline = min_inline.max_of(contribution.min);
        max_inline = max_inline.max_of(contribution.max);
    }

    MinMaxSizes::new(min_inline + bp_inline, max_inline + bp_inline)
}

fn compute_child_intrinsic_contribution_with_block_size(
    doc: &Document,
    child_id: NodeId,
    parent_content_block_size: LayoutUnit,
) -> MinMaxSizes {
    let child_style = &doc.node(child_id).style;
    let margin = resolve_margins(child_style, LayoutUnit::zero());
    let margin_inline = margin.inline_sum();

    let child_border = resolve_border(child_style);
    let child_padding = resolve_padding(child_style, LayoutUnit::zero());
    let child_bp_inline = child_border.inline_sum() + child_padding.inline_sum();
    let child_bp_block = child_border.block_sum() + child_padding.block_sum();

    let specified_child_block_bb = if child_style.height.is_percent()
        || child_style.height.is_fixed()
        || child_style.height.length_type() == LengthType::Calculated
    {
        let raw = resolve_length(
            &child_style.height,
            parent_content_block_size,
            openui_geometry::INDEFINITE_SIZE,
            openui_geometry::INDEFINITE_SIZE,
        );
        if raw.is_indefinite() {
            None
        } else if child_style.box_sizing == BoxSizing::BorderBox {
            Some(raw.max_of(child_bp_block))
        } else {
            Some(raw + child_bp_block)
        }
    } else {
        None
    };
    let resolve_block_bound = |length: &Length| {
        (!length.is_auto() && !length.is_none() && !length.is_content_or_intrinsic())
            .then(|| {
                resolve_length(
                    length,
                    parent_content_block_size,
                    openui_geometry::INDEFINITE_SIZE,
                    openui_geometry::INDEFINITE_SIZE,
                )
            })
            .filter(|value| !value.is_indefinite())
            .map(|raw| {
                if child_style.box_sizing == BoxSizing::BorderBox {
                    raw.max_of(child_bp_block)
                } else {
                    raw + child_bp_block
                }
            })
    };
    let block_min = resolve_block_bound(&child_style.min_height).unwrap_or(LayoutUnit::zero());
    let block_max = resolve_block_bound(&child_style.max_height).unwrap_or(LayoutUnit::max());
    let known_child_block_bb = if let Some(specified) = specified_child_block_bb {
        // A preferred block size still participates in min/max sizing before
        // its used value transfers through a replaced aspect ratio.
        Some(specified.clamp(block_min, block_max))
    } else if block_min > LayoutUnit::zero() || block_max < LayoutUnit::max() {
        let natural = (compute_child_intrinsic_contribution(doc, child_id).max_content_block_size
            - resolve_margins(child_style, LayoutUnit::zero()).block_sum())
        .clamp_negative_to_zero();
        Some(natural.clamp(block_min, block_max))
    } else {
        None
    };

    if let Some(block_bb) = known_child_block_bb {
        let natural_ratio = doc
            .node(child_id)
            .replaced
            .and_then(|content| content.intrinsic_ratio)
            .filter(|(width, height)| *width > 0.0 && *height > 0.0);
        let effective_ratio = child_style
            .aspect_ratio
            .as_ref()
            .and_then(|ratio| {
                if ratio.auto_flag {
                    natural_ratio.or_else(|| {
                        (ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0).then_some(ratio.ratio)
                    })
                } else {
                    (ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0).then_some(ratio.ratio)
                }
            })
            .or(natural_ratio);
        if let Some((ratio_width, ratio_height)) = effective_ratio {
            if child_style.width.is_auto()
                || child_style.width.is_content_or_intrinsic()
                || child_style.width.is_percent()
                || child_style.width.length_type() == LengthType::Calculated
            {
                // A percentage inline size is cyclic while computing the
                // parent's intrinsic inline contribution. When the opposite
                // axis is definite, the replaced element's preferred ratio
                // supplies that contribution just like an auto inline size.
                let ar_uses_border_box = child_style
                    .aspect_ratio
                    .as_ref()
                    .is_some_and(|ratio| !ratio.auto_flag)
                    && child_style.box_sizing == BoxSizing::BorderBox;
                let transferred = if ar_uses_border_box {
                    LayoutUnit::from_f32(block_bb.to_f32() * ratio_width / ratio_height)
                } else {
                    let content_h = (block_bb - child_bp_block).clamp_negative_to_zero();
                    LayoutUnit::from_f32(content_h.to_f32() * ratio_width / ratio_height)
                        + child_bp_inline
                };
                let clamped = apply_min_max_inline(
                    child_style,
                    transferred,
                    (transferred, transferred),
                    false,
                );
                return MinMaxSizes::new(clamped + margin_inline, clamped + margin_inline);
            }
        }

        if doc.children(child_id).next().is_some() {
            let nested = compute_intrinsic_inline_sizes_with_block_size(
                doc,
                child_id,
                block_bb,
                LayoutUnit::zero(),
            );
            let min =
                apply_min_max_inline(child_style, nested.min, (nested.min, nested.max), false);
            let max = apply_min_max_inline(child_style, nested.max, (nested.min, nested.max), true);
            return MinMaxSizes::new(min + margin_inline, max + margin_inline);
        }
    }

    // Inline wrappers do not establish a containing block for an inner block
    // that was split out by block-in-inline layout. Carry the definite block
    // basis through that transparent wrapper so a nested percentage-sized
    // replaced box can transfer its used block size through its ratio.
    if is_inline_level(child_style) && has_block_dependent_replaced_descendant(doc, child_id) {
        let nested = compute_intrinsic_inline_sizes_with_block_size(
            doc,
            child_id,
            parent_content_block_size,
            LayoutUnit::zero(),
        );
        let min = apply_min_max_inline(child_style, nested.min, (nested.min, nested.max), false);
        let max = apply_min_max_inline(child_style, nested.max, (nested.min, nested.max), true);
        return MinMaxSizes::new(min + margin_inline, max + margin_inline);
    }

    let intrinsic = compute_child_intrinsic_contribution(doc, child_id);
    MinMaxSizes::new(
        intrinsic.min_content_inline_size,
        intrinsic.max_content_inline_size,
    )
}

/// Compute text intrinsic sizes.
///
/// min-content = widest soft-wrap opportunity; max-content = the longest
/// forced line. Both values use the same font resolution and shaping path as
/// inline layout so shrink-to-fit boxes do not acquire an unrelated 8px-per-
/// character approximation.
/// max-content = full text width.
fn compute_text_intrinsic_sizes(doc: &Document, text: &str, style: &ComputedStyle) -> MinMaxSizes {
    compute_text_intrinsic_sizes_impl(doc, text, style, false)
}

fn is_css_collapsible_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{000C}')
}

fn compute_text_intrinsic_sizes_impl(
    doc: &Document,
    text: &str,
    style: &ComputedStyle,
    preserve_boundary_whitespace: bool,
) -> MinMaxSizes {
    let processed = preprocess_text_for_shaping(text, style, doc.font_collection());
    // Collapsible white space at an intrinsic line boundary has no advance.
    // In particular, source indentation around a text-only shrink-to-fit
    // float must not inflate its max-content width. Inline layout already
    // applies the same start/end collapsing when it constructs the line.
    let deterministic_text_profile = uses_deterministic_text_profile(style);
    let processed = if deterministic_text_profile
        && !preserve_boundary_whitespace
        && matches!(
            style.white_space,
            WhiteSpace::Normal | WhiteSpace::Nowrap | WhiteSpace::PreLine
        ) {
        processed.trim_matches(is_css_collapsible_whitespace)
    } else {
        processed.as_str()
    };
    if processed.is_empty() {
        return MinMaxSizes::zero();
    }

    let font = doc.resolve_font(style_to_font_description(style));
    let shaper = openui_text::TextShaper::new();
    let direction = if style.direction == openui_style::Direction::Rtl {
        openui_text::TextDirection::Rtl
    } else {
        openui_text::TextDirection::Ltr
    };
    let measure = |run: &str| intrinsic_text_width(shaper.shape(run, &font, direction).width);
    let forced_lines = processed.split('\n');
    let max_content = forced_lines
        .clone()
        .map(measure)
        .fold(LayoutUnit::zero(), |acc, width| acc.max_of(width));

    let permits_soft_wrap = !matches!(style.white_space, WhiteSpace::Nowrap | WhiteSpace::Pre);
    let min_content = if permits_soft_wrap
        && (style.word_break == WordBreak::BreakAll || style.line_break == LineBreak::Anywhere)
    {
        // `break-all` and `line-break: anywhere` introduce a soft wrap
        // opportunity between typographic character units. For the text
        // handled by this engine, measuring each scalar through the same font
        // path gives the min-content width needed by shrink-to-fit floats and
        // inline blocks.
        forced_lines
            .flat_map(|line| line.chars())
            .map(|ch| {
                let mut encoded = [0; 4];
                measure(ch.encode_utf8(&mut encoded))
            })
            .fold(LayoutUnit::zero(), |acc, width| acc.max_of(width))
    } else if permits_soft_wrap && deterministic_text_profile {
        use unicode_linebreak::{linebreaks, BreakOpportunity};

        forced_lines.fold(LayoutUnit::zero(), |widest, line| {
            let mut segment_start = 0;
            let mut line_widest = LayoutUnit::zero();
            for (segment_end, opportunity) in linebreaks(line) {
                if !matches!(
                    opportunity,
                    BreakOpportunity::Allowed | BreakOpportunity::Mandatory
                ) {
                    continue;
                }
                let segment =
                    line[segment_start..segment_end].trim_matches(is_css_collapsible_whitespace);
                if !segment.is_empty() {
                    line_widest = line_widest.max_of(measure(segment));
                }
                segment_start = segment_end;
            }
            if segment_start < line.len() {
                line_widest = line_widest.max_of(measure(&line[segment_start..]));
            }
            widest.max_of(line_widest)
        })
    } else if permits_soft_wrap {
        forced_lines
            .flat_map(|line| line.split(is_css_collapsible_whitespace))
            .filter(|segment| !segment.is_empty())
            .map(measure)
            .fold(LayoutUnit::zero(), |acc, width| acc.max_of(width))
    } else {
        max_content
    };

    MinMaxSizes::new(min_content, max_content)
}

// ── Auto block size from content ─────────────────────────────────────────

/// Compute the auto block size of an element from its children.
///
/// CSS 2.1 §10.6.3: the height of a block-level element with `height: auto`
/// is the distance between the top content edge and:
/// - the bottom edge of the last in-flow child's margin box, or
/// - the bottom edge of the last line box, or
/// - zero if there are no children.
///
/// Margins between children collapse per CSS 2.1 §8.3.1.
///
/// The result is clamped by min-height / max-height.
pub fn compute_block_size_from_content(
    doc: &Document,
    node_id: NodeId,
    child_margin_boxes: &[LayoutUnit],
) -> LayoutUnit {
    let style = &doc.node(node_id).style;

    // Sum of all children's margin-box block sizes.
    let mut content_height = LayoutUnit::zero();
    for &child_block in child_margin_boxes {
        content_height = content_height + child_block;
    }

    // Apply simple margin collapsing between adjacent siblings.
    // For intrinsic sizing purposes, we apply a simplified version:
    // adjacent positive margins collapse (take the larger).
    content_height = collapse_adjacent_margins(doc, node_id, content_height);

    // Clamp by min-height / max-height.
    let min_height = resolve_length(
        &style.min_height,
        LayoutUnit::zero(), // percentage resolves to 0 when containing block is auto
        LayoutUnit::zero(), // auto min-height = 0
        LayoutUnit::zero(), // none = 0
    );
    let max_height = resolve_length(
        &style.max_height,
        LayoutUnit::zero(),
        LayoutUnit::max(), // auto = unconstrained
        LayoutUnit::max(), // none = unconstrained
    );

    content_height.clamp(min_height, max_height)
}

/// Simplified margin collapsing for block size computation.
///
/// Looks at adjacent children's margins and collapses them per CSS 2.1 §8.3.1.
/// Returns the adjusted total block size.
fn collapse_adjacent_margins(doc: &Document, node_id: NodeId, raw_sum: LayoutUnit) -> LayoutUnit {
    // Only consider in-flow children (skip display:none and absolutely positioned).
    let children: Vec<NodeId> = doc
        .children(node_id)
        .filter(|&id| {
            let s = &doc.node(id).style;
            s.display != openui_style::Display::None
                && !s.position.is_absolutely_positioned()
                && s.float == openui_style::Float::None
        })
        .collect();
    if children.len() < 2 {
        return raw_sum;
    }

    let mut collapsed_reduction = LayoutUnit::zero();

    for i in 0..children.len() - 1 {
        let current_style = &doc.node(children[i]).style;
        let next_style = &doc.node(children[i + 1]).style;

        let current_bottom = resolve_length(
            &current_style.margin_bottom,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        let next_top = resolve_length(
            &next_style.margin_top,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );

        // Only collapse when both are non-negative (simple case).
        if current_bottom.raw() >= 0 && next_top.raw() >= 0 {
            let smaller = current_bottom.min_of(next_top);
            collapsed_reduction = collapsed_reduction + smaller;
        } else if current_bottom.raw() < 0 && next_top.raw() < 0 {
            // Both negative: keep the more negative, remove the other.
            let less_negative = current_bottom.max_of(next_top);
            collapsed_reduction = collapsed_reduction + less_negative;
        }
        // Mixed positive/negative: they sum (no collapsing reduction).
    }

    raw_sum - collapsed_reduction
}

// ── Shrink-to-fit ────────────────────────────────────────────────────────

/// CSS 2.1 §10.3.5: shrink-to-fit inline size.
///
/// `min(max(min_content, available), max_content)`
///
/// Used for floats, absolutely positioned elements with `width: auto`,
/// inline-block elements, and table cells.
#[inline]
pub fn shrink_to_fit_inline_size(
    min_content: LayoutUnit,
    max_content: LayoutUnit,
    available: LayoutUnit,
) -> LayoutUnit {
    // preferred minimum width = min_content
    // preferred width = max_content
    // available width = available
    // Result = min(max(preferred minimum, available), preferred)
    let lower = min_content.max_of(available);
    lower.min_of(max_content)
}

// ── Replaced element intrinsic sizes ─────────────────────────────────────

/// Compute intrinsic sizes for replaced elements (img, video, canvas, etc.).
///
/// CSS 2.1 §10.3.2, CSS Sizing 3 §5.2:
/// - Use intrinsic width/height if specified (CSS `width`/`height` on the element).
/// - If only one dimension is specified and the element has an aspect ratio,
///   derive the other from the ratio.
/// - Default to 300×150 for objects with no intrinsic size (CSS 2.1 §10.3.2).
pub fn compute_replaced_intrinsic_sizes(style: &ComputedStyle) -> IntrinsicSizes {
    compute_replaced_intrinsic_sizes_with_natural(style, 300.0, 150.0, Some((300.0, 150.0)))
}

/// Compute intrinsic contributions from the resource metadata stored on a DOM
/// node. Percentage preferred sizes remain indefinite during intrinsic sizing;
/// the element's natural dimensions are used until a definite containing-block
/// size exists in normal layout.
pub fn compute_replaced_intrinsic_sizes_for_node(
    doc: &Document,
    node_id: NodeId,
) -> IntrinsicSizes {
    let node = doc.node(node_id);
    let replaced = node.replaced;
    let missing_image = node.tag == ElementTag::Image && replaced.is_none();
    let source_less_alt_width = missing_image
        .then(|| doc.attribute(node_id, "alt"))
        .flatten()
        .filter(|alt| !alt.is_empty())
        .map(|alt| {
            let font = doc.resolve_font(style_to_font_description(&node.style));
            font.width(alt)
        })
        .unwrap_or(0.0);
    let natural_width = replaced
        .and_then(|content| content.intrinsic_width)
        .unwrap_or(if missing_image {
            16.0 + source_less_alt_width
        } else {
            300.0
        });
    let natural_height = replaced
        .and_then(|content| content.intrinsic_height)
        .unwrap_or(if missing_image { 16.0 } else { 150.0 });
    let natural_ratio = replaced.and_then(|content| content.intrinsic_ratio);
    let svg_ratio_only = node.tag == ElementTag::Svg
        && replaced.is_some_and(|content| {
            !content
                .intrinsic_width
                .is_some_and(|dimension| dimension > 0.0)
                && !content
                    .intrinsic_height
                    .is_some_and(|dimension| dimension > 0.0)
        })
        && (node.style.width.is_stretch()
            || (node.style.width.is_auto()
                && node
                    .style
                    .aspect_ratio
                    .as_ref()
                    .is_some_and(|ratio| ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0)));
    let mut sizes = compute_replaced_intrinsic_sizes_with_natural(
        &node.style,
        if svg_ratio_only { 0.0 } else { natural_width },
        if svg_ratio_only { 0.0 } else { natural_height },
        natural_ratio,
    );
    let style = &node.style;
    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let inline_edges = border.inline_sum() + padding.inline_sum();
    let block_edges = border.block_sum() + padding.block_sum();
    let svg_cyclic_percentage = node.tag == ElementTag::Svg
        && replaced.is_some_and(|content| {
            !content
                .intrinsic_width
                .is_some_and(|dimension| dimension > 0.0)
                && !content
                    .intrinsic_height
                    .is_some_and(|dimension| dimension > 0.0)
        })
        && (node.style.width.is_percent()
            || node.style.width.length_type() == LengthType::Calculated
            || node.style.height.is_percent()
            || node.style.height.length_type() == LengthType::Calculated);
    if svg_cyclic_percentage {
        // Cyclic percentages do not contribute a minimum, but retain the
        // resource's default maximum contribution. This lets a ratio-only SVG
        // establish a flex base size and subsequently shrink to the available
        // space instead of either overflowing at 300px or disappearing at 0px.
        sizes.min_content_inline_size = inline_edges;
        sizes.min_content_block_size = block_edges;
    }
    let intrinsic_bound = |length: &Length,
                           min_content: LayoutUnit,
                           max_content: LayoutUnit,
                           edges: LayoutUnit,
                           is_minimum: bool| {
        if length.is_auto() || length.is_none() || length.is_percent() {
            return None;
        }
        let raw = match length.length_type() {
            LengthType::MinContent => min_content,
            LengthType::MaxContent | LengthType::FitContent | LengthType::Content => max_content,
            LengthType::Calculated => LayoutUnit::from_f32(length.calc_offset()),
            LengthType::Fixed => LayoutUnit::from_f32(length.value()),
            _ => return None,
        };
        Some(if length.is_content_or_intrinsic() {
            raw
        } else if style.box_sizing == BoxSizing::ContentBox {
            raw + edges
        } else if is_minimum {
            raw.max_of(edges)
        } else {
            raw.max_of(edges)
        })
    };
    let min_inline = intrinsic_bound(
        &style.min_width,
        sizes.min_content_inline_size,
        sizes.max_content_inline_size,
        inline_edges,
        true,
    )
    .unwrap_or(LayoutUnit::zero());
    let mut max_inline = intrinsic_bound(
        &style.max_width,
        sizes.min_content_inline_size,
        sizes.max_content_inline_size,
        inline_edges,
        false,
    )
    .unwrap_or(LayoutUnit::max());
    let min_block = intrinsic_bound(
        &style.min_height,
        sizes.min_content_block_size,
        sizes.max_content_block_size,
        block_edges,
        true,
    )
    .unwrap_or(LayoutUnit::zero());
    let mut max_block = intrinsic_bound(
        &style.max_height,
        sizes.min_content_block_size,
        sizes.max_content_block_size,
        block_edges,
        false,
    )
    .unwrap_or(LayoutUnit::max());
    max_inline = max_inline.max_of(min_inline);
    max_block = max_block.max_of(min_block);

    let preserves_ratio = style
        .aspect_ratio
        .as_ref()
        .is_some_and(|ratio| ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0)
        || natural_ratio.is_some_and(|(width, height)| width > 0.0 && height > 0.0);
    let independently_sized = style.width.is_fixed() && style.height.is_fixed();
    let constrain_pair = |mut inline: LayoutUnit, mut block: LayoutUnit| {
        if preserves_ratio && !independently_sized {
            let inline_float = inline.to_f32();
            let block_float = block.to_f32();
            let mut downscale = 1.0_f32;
            if inline_float > 0.0 && inline > max_inline {
                downscale = downscale.min(max_inline.to_f32() / inline_float);
            }
            if block_float > 0.0 && block > max_block {
                downscale = downscale.min(max_block.to_f32() / block_float);
            }
            if downscale < 1.0 {
                inline = LayoutUnit::from_f32(inline_float * downscale);
                block = LayoutUnit::from_f32(block_float * downscale);
            }

            let inline_float = inline.to_f32();
            let block_float = block.to_f32();
            let mut upscale = 1.0_f32;
            if inline_float > 0.0 && inline < min_inline {
                upscale = upscale.max(min_inline.to_f32() / inline_float);
            }
            if block_float > 0.0 && block < min_block {
                upscale = upscale.max(min_block.to_f32() / block_float);
            }
            if upscale > 1.0 {
                inline = LayoutUnit::from_f32(inline_float * upscale);
                block = LayoutUnit::from_f32(block_float * upscale);
            }
        }
        (
            inline.clamp(min_inline, max_inline),
            block.clamp(min_block, max_block),
        )
    };
    let min_pair = constrain_pair(sizes.min_content_inline_size, sizes.min_content_block_size);
    let max_pair = constrain_pair(sizes.max_content_inline_size, sizes.max_content_block_size);
    sizes.min_content_inline_size = min_pair.0;
    sizes.min_content_block_size = min_pair.1;
    sizes.max_content_inline_size = max_pair.0;
    sizes.max_content_block_size = max_pair.1;

    if node.form_control == Some(openui_dom::FormControlRole::Range) {
        // CSS Sizing 3 §5.2.1: a cyclic percentage preferred size contributes
        // zero to min-content while the control's natural size remains its
        // max-content contribution. This distinction is what lets a range
        // shrink inside a definite wrapper without erasing its auto size.
        if matches!(
            node.style.width.length_type(),
            openui_geometry::LengthType::Percent | openui_geometry::LengthType::Calculated
        ) {
            sizes.min_content_inline_size = border.inline_sum() + padding.inline_sum();
        }
        if matches!(
            node.style.height.length_type(),
            openui_geometry::LengthType::Percent | openui_geometry::LengthType::Calculated
        ) || (node.style.min_height.is_auto()
            && matches!(
                node.style.max_height.length_type(),
                openui_geometry::LengthType::Percent | openui_geometry::LengthType::Calculated
            ))
        {
            sizes.min_content_block_size = border.block_sum() + padding.block_sum();
        }
    }
    sizes
}

fn compute_replaced_intrinsic_sizes_with_natural(
    style: &ComputedStyle,
    natural_width: f32,
    natural_height: f32,
    natural_ratio: Option<(f32, f32)>,
) -> IntrinsicSizes {
    // Default replaced element size (CSS 2.1 §10.3.2).
    let default_width = LayoutUnit::from_f32(natural_width);
    let default_height = LayoutUnit::from_f32(natural_height);

    // Determine the effective aspect ratio for deriving the missing dimension.
    // CSS Sizing 4: If a CSS `aspect-ratio` is specified (without `auto`), it
    // overrides the natural ratio. With `auto <ratio>`, the natural ratio
    // (from the element's intrinsic dimensions) takes priority.
    let effective_ratio = if let Some(ref ar) = style.aspect_ratio {
        if ar.auto_flag {
            // `auto <ratio>`: prefer the natural ratio.
            natural_ratio
                .filter(|(width, height)| *width > 0.0 && *height > 0.0)
                .map(|(width, height)| (LayoutUnit::from_f32(width), LayoutUnit::from_f32(height)))
                .or_else(|| {
                    (ar.ratio.0 > 0.0 && ar.ratio.1 > 0.0).then(|| {
                        (
                            LayoutUnit::from_f32(ar.ratio.0),
                            LayoutUnit::from_f32(ar.ratio.1),
                        )
                    })
                })
        } else if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
            // Bare `<ratio>`: override natural ratio with specified one.
            Some((
                LayoutUnit::from_f32(ar.ratio.0),
                LayoutUnit::from_f32(ar.ratio.1),
            ))
        } else {
            natural_ratio
                .filter(|(width, height)| *width > 0.0 && *height > 0.0)
                .map(|(width, height)| (LayoutUnit::from_f32(width), LayoutUnit::from_f32(height)))
        }
    } else {
        natural_ratio
            .filter(|(width, height)| *width > 0.0 && *height > 0.0)
            .map(|(width, height)| (LayoutUnit::from_f32(width), LayoutUnit::from_f32(height)))
    };

    let has_width = style.width.length_type() == openui_geometry::LengthType::Fixed;
    let has_height = style.height.length_type() == openui_geometry::LengthType::Fixed;

    let (width, height) = match (has_width, has_height) {
        (true, true) => {
            let w = LayoutUnit::from_f32(style.width.value());
            let h = LayoutUnit::from_f32(style.height.value());
            (w, h)
        }
        (true, false) => {
            let w = LayoutUnit::from_f32(style.width.value());
            // Derive height only when the replaced resource has a natural or
            // preferred aspect ratio; iframe/object default dimensions do
            // not imply one.
            let h = effective_ratio
                .map(|(ratio_w, ratio_h)| apply_aspect_ratio(w, ratio_w, ratio_h))
                .unwrap_or(default_height);
            (w, h)
        }
        (false, true) => {
            let h = LayoutUnit::from_f32(style.height.value());
            let w = effective_ratio
                .map(|(ratio_w, ratio_h)| apply_aspect_ratio_inverse(h, ratio_w, ratio_h))
                .unwrap_or(default_width);
            (w, h)
        }
        (false, false) => (default_width, default_height),
    };

    // Add border + padding.
    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let bp_inline = border.inline_sum() + padding.inline_sum();
    let bp_block = border.block_sum() + padding.block_sum();

    let total_inline = width + bp_inline;
    let total_block = height + bp_block;

    IntrinsicSizes {
        min_content_inline_size: total_inline,
        max_content_inline_size: total_inline,
        min_content_block_size: total_block,
        max_content_block_size: total_block,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// True if this element tag represents a replaced element.
///
/// In the current DOM model, only `Viewport` and `Text` are special;
/// all others are generic containers. We treat none as replaced for now,
/// but this function provides the extension point. The caller can mark
/// elements as replaced through the style (e.g., explicit width+height
/// on an img-like element).
pub(crate) fn is_replaced_element(tag: ElementTag) -> bool {
    matches!(
        tag,
        ElementTag::Image
            | ElementTag::Canvas
            | ElementTag::Svg
            | ElementTag::Audio
            | ElementTag::Video
    )
}

/// Derive height from width using the default aspect ratio.
///
/// `height = width * (intrinsic_height / intrinsic_width)`
fn apply_aspect_ratio(
    known: LayoutUnit,
    intrinsic_width: LayoutUnit,
    intrinsic_height: LayoutUnit,
) -> LayoutUnit {
    if intrinsic_width.raw() == 0 {
        return intrinsic_height;
    }
    known.mul_div(intrinsic_height, intrinsic_width)
}

/// Derive width from height using the default aspect ratio.
///
/// `width = height * (intrinsic_width / intrinsic_height)`
fn apply_aspect_ratio_inverse(
    known: LayoutUnit,
    intrinsic_width: LayoutUnit,
    intrinsic_height: LayoutUnit,
) -> LayoutUnit {
    if intrinsic_height.raw() == 0 {
        return intrinsic_width;
    }
    known.mul_div(intrinsic_width, intrinsic_height)
}

/// If the element has an explicit fixed width, use it (content-box);
/// otherwise return the intrinsic size.
/// If the element has an explicit fixed width, use it (as border-box);
/// otherwise return the intrinsic value (already border-box).
///
/// The returned value must be border-box because the intrinsic sizes from
/// `compute_intrinsic_block_sizes` include border+padding. Converting the
/// explicit width to border-box ensures consistent units throughout the
/// intrinsic sizing pipeline.
pub fn apply_size_override_inline(style: &ComputedStyle, intrinsic: LayoutUnit) -> LayoutUnit {
    if style.width.length_type() == openui_geometry::LengthType::Fixed {
        let raw = LayoutUnit::from_f32(style.width.value());
        let bp_val = {
            let b = resolve_border(style);
            let p = resolve_padding(style, LayoutUnit::zero());
            b.left + b.right + p.left + p.right
        };
        if style.box_sizing == BoxSizing::BorderBox {
            raw.max_of(bp_val)
        } else {
            raw + bp_val
        }
    } else if style.width.is_auto()
        || style.width.is_content_or_intrinsic()
        || style.width.is_percent()
    {
        // CSS Sizing 4 §5.1: When width is auto (or an intrinsic keyword like
        // min-content/max-content) and the element has aspect-ratio + definite
        // height, compute width from height × ratio. For intrinsic keywords,
        // the transferred size replaces the content-based intrinsic size.
        let transferred = if let Some(ref ar) = style.aspect_ratio {
            if style.height.length_type() == openui_geometry::LengthType::Fixed
                && ar.ratio.0 != 0.0
                && ar.ratio.1 != 0.0
            {
                let b = resolve_border(style);
                let p = resolve_padding(style, LayoutUnit::zero());
                let bp_inline = b.left + b.right + p.left + p.right;
                let bp_block = b.top + b.bottom + p.top + p.bottom;

                // CSS Sizing 4: when `auto <ratio>`, AR maps through
                // content-box. Bare `<ratio>` respects box-sizing.
                let ar_uses_border_box = !ar.auto_flag && style.box_sizing == BoxSizing::BorderBox;

                // Get raw height and clamp by min-height / max-height.
                let h_raw = LayoutUnit::from_f32(style.height.value());
                let indefinite = openui_geometry::INDEFINITE_SIZE;
                let min_h = resolve_length(
                    &style.min_height,
                    indefinite,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
                let max_h = resolve_length(
                    &style.max_height,
                    indefinite,
                    LayoutUnit::max(),
                    LayoutUnit::max(),
                );
                let h_clamped = if style.box_sizing == BoxSizing::BorderBox {
                    let h_bb = h_raw.max_of(bp_block);
                    let min_h_bb = if min_h > LayoutUnit::zero() {
                        min_h.max_of(bp_block)
                    } else {
                        LayoutUnit::zero()
                    };
                    let max_h_bb = if max_h < LayoutUnit::max() {
                        max_h.max_of(bp_block)
                    } else {
                        LayoutUnit::max()
                    };
                    h_bb.clamp(min_h_bb, max_h_bb)
                } else {
                    h_raw.clamp(min_h, max_h)
                };

                if ar_uses_border_box {
                    // AR applies to border-box: w_bb = h_bb × ratio
                    LayoutUnit::from_f32(h_clamped.to_f32() * ar.ratio.0 / ar.ratio.1)
                } else {
                    // AR applies to content-box: content_w = content_h × ratio
                    let content_h = if style.box_sizing == BoxSizing::BorderBox {
                        (h_clamped - bp_block).clamp_negative_to_zero()
                    } else {
                        h_clamped
                    };
                    LayoutUnit::from_f32(content_h.to_f32() * ar.ratio.0 / ar.ratio.1) + bp_inline
                }
            } else {
                intrinsic
            }
        } else {
            intrinsic
        };
        // A cyclic percentage preferred size is treated as auto for
        // intrinsic sizing. Ratio transfer is one size suggestion, while
        // descendant content may still establish a larger contribution.
        if style.width.is_percent() {
            transferred.max_of(intrinsic)
        } else {
            transferred
        }
    } else {
        intrinsic
    }
}

/// If the element has an explicit fixed height, use it (as border-box);
/// otherwise return the intrinsic value (already border-box).
fn aspect_ratio_block_from_fixed_width(style: &ComputedStyle) -> Option<LayoutUnit> {
    let ar = style.aspect_ratio.as_ref()?;
    if !style.width.is_fixed() || ar.ratio.0 <= 0.0 || ar.ratio.1 <= 0.0 {
        return None;
    }
    let b = resolve_border(style);
    let p = resolve_padding(style, LayoutUnit::zero());
    let bp_inline = b.left + b.right + p.left + p.right;
    let bp_block = b.top + b.bottom + p.top + p.bottom;
    let width = LayoutUnit::from_f32(style.width.value());
    let content_width = if style.box_sizing == BoxSizing::BorderBox {
        (width - bp_inline).clamp_negative_to_zero()
    } else {
        width
    };
    Some(LayoutUnit::from_f32(content_width.to_f32() * ar.ratio.1 / ar.ratio.0) + bp_block)
}

fn apply_size_override_block(style: &ComputedStyle, intrinsic: LayoutUnit) -> LayoutUnit {
    if style.height.length_type() == openui_geometry::LengthType::Fixed {
        let raw = LayoutUnit::from_f32(style.height.value());
        let bp_val = {
            let b = resolve_border(style);
            let p = resolve_padding(style, LayoutUnit::zero());
            b.top + b.bottom + p.top + p.bottom
        };
        if style.box_sizing == BoxSizing::BorderBox {
            raw.max_of(bp_val)
        } else {
            raw + bp_val
        }
    } else if style.height.is_auto() || style.height.is_content_or_intrinsic() {
        // CSS Sizing 4 §5.1: When height is auto and the element has
        // aspect-ratio + definite width, compute height from width × ratio.
        aspect_ratio_block_from_fixed_width(style).unwrap_or(intrinsic)
    } else {
        intrinsic
    }
}

/// Clamp a resolved border-box inline size by min-width / max-width.
///
/// The `size` parameter is in border-box units (from `apply_size_override_inline`
/// or from `compute_intrinsic_block_sizes` which includes border+padding).
/// Min/max values must be converted to border-box before clamping when
/// box-sizing is content-box.
fn apply_min_max_inline(
    style: &ComputedStyle,
    size: LayoutUnit,
    base_intrinsic: (LayoutUnit, LayoutUnit), // (min-content, max-content) in border-box
    is_max_content_contribution: bool,
) -> LayoutUnit {
    let zero = LayoutUnit::zero();

    // Percentage min/max resolve against the containing block's inline size.
    // In intrinsic sizing there is no containing block, so use INDEFINITE_SIZE
    // to trigger the auto fallback in resolve_length (CSS Sizing 3 §5.1:
    // percentage sizes against indefinite bases are treated as auto).
    let indefinite = openui_geometry::INDEFINITE_SIZE;
    let resolve_intrinsic_constraint =
        |length: &Length, auto_value: LayoutUnit, none_value: LayoutUnit| {
            if length.length_type() == LengthType::Calculated {
                // During intrinsic sizing a cyclic percentage contributes
                // zero, but the definite term of calc() remains. Treating the
                // entire expression as auto drops e.g. calc(160px + 0%).
                LayoutUnit::from_f32(length.calc_offset())
            } else {
                resolve_length(length, indefinite, auto_value, none_value)
            }
        };

    // Resolve min-width, handling intrinsic keywords.
    let min_raw = if style.min_width.is_content_or_intrinsic() {
        match style.min_width.length_type() {
            LengthType::MinContent => base_intrinsic.0,
            LengthType::MaxContent => base_intrinsic.1,
            // A fit-content minimum contributes the corresponding intrinsic
            // endpoint. In particular, a definite preferred width must not
            // collapse its max-content contribution to min-content.
            _ if is_max_content_contribution => base_intrinsic.1,
            _ => base_intrinsic.0,
        }
    } else {
        resolve_intrinsic_constraint(
            &style.min_width,
            zero, // auto min-width = 0
            zero,
        )
    };
    // Resolve max-width, handling intrinsic keywords.
    let max_raw = if style.max_width.is_content_or_intrinsic() {
        match style.max_width.length_type() {
            LengthType::MinContent => base_intrinsic.0,
            LengthType::MaxContent => base_intrinsic.1,
            _ => base_intrinsic.1, // fit-content → max-content for max sizing
        }
    } else if style.max_width.length_type() == LengthType::Calculated {
        // A percentage-dependent maximum is cyclic while the containing
        // inline size is being intrinsically measured. It behaves as an
        // unconstrained maximum, including calc() expressions whose authored
        // percentage term happens to be zero.
        LayoutUnit::max()
    } else {
        resolve_intrinsic_constraint(
            &style.max_width,
            LayoutUnit::max(), // auto = unconstrained
            LayoutUnit::max(), // none = unconstrained
        )
    };

    // Compute border+padding for conversion and floor.
    let bp_val = {
        let b = resolve_border(style);
        let p = resolve_padding(style, zero);
        b.left + b.right + p.left + p.right
    };

    // Convert to border-box units to match the size being clamped.
    // For content-box: add bp. For border-box: floor at bp.
    let min_bb = if min_raw > zero {
        if style.box_sizing == BoxSizing::ContentBox {
            min_raw + bp_val
        } else {
            min_raw.max_of(bp_val)
        }
    } else {
        zero
    };
    let max_bb = if max_raw == LayoutUnit::max() {
        max_raw
    } else if style.box_sizing == BoxSizing::ContentBox {
        max_raw + bp_val
    } else {
        max_raw.max_of(bp_val)
    };

    // CSS Sizing 4 §5.2: transferred min/max through aspect-ratio.
    // If min-height or max-height is definite and AR is present, transfer to inline axis.
    let (mut min_bb, mut max_bb) = (min_bb, max_bb);
    if let Some(ref ar) = style.aspect_ratio {
        if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
            let bp_block = {
                let b = resolve_border(style);
                let p = resolve_padding(style, zero);
                b.top + b.bottom + p.top + p.bottom
            };

            // CSS Sizing 4: bare ratio respects box-sizing; auto ratio
            // always maps through content-box.
            let ar_uses_border_box = !ar.auto_flag && style.box_sizing == BoxSizing::BorderBox;

            // Transfer min-height → min-width (only if min-width is auto/0)
            let min_h_raw = resolve_length(&style.min_height, indefinite, zero, zero);
            if min_h_raw > zero && min_bb == zero {
                if ar_uses_border_box {
                    min_bb = LayoutUnit::from_f32(min_h_raw.to_f32() * ar.ratio.0 / ar.ratio.1);
                } else {
                    let content_min_h = if style.box_sizing == BoxSizing::BorderBox {
                        (min_h_raw - bp_block).clamp_negative_to_zero()
                    } else {
                        min_h_raw
                    };
                    min_bb = LayoutUnit::from_f32(content_min_h.to_f32() * ar.ratio.0 / ar.ratio.1)
                        + bp_val;
                }
            }

            // Transfer max-height → max-width (only if max-width is unconstrained)
            let max_h_raw = resolve_length(
                &style.max_height,
                indefinite,
                LayoutUnit::max(),
                LayoutUnit::max(),
            );
            if max_h_raw < LayoutUnit::max() && max_bb == LayoutUnit::max() {
                if ar_uses_border_box {
                    max_bb = LayoutUnit::from_f32(max_h_raw.to_f32() * ar.ratio.0 / ar.ratio.1);
                } else {
                    let content_max_h = if style.box_sizing == BoxSizing::BorderBox {
                        (max_h_raw - bp_block).clamp_negative_to_zero()
                    } else {
                        max_h_raw
                    };
                    max_bb = LayoutUnit::from_f32(content_max_h.to_f32() * ar.ratio.0 / ar.ratio.1)
                        + bp_val;
                }
            }
        }
    }

    size.clamp(min_bb, max_bb)
}

/// Clamp a resolved border-box block size by min-height / max-height.
fn apply_min_max_block(style: &ComputedStyle, size: LayoutUnit) -> LayoutUnit {
    let zero = LayoutUnit::zero();

    // Same as apply_min_max_inline: use INDEFINITE_SIZE for percentage base
    // so percentage min/max-height resolves to auto values (CSS Sizing 3 §5.1).
    let indefinite = openui_geometry::INDEFINITE_SIZE;

    let min_raw = resolve_length(&style.min_height, indefinite, zero, zero);
    let max_raw = resolve_length(
        &style.max_height,
        indefinite,
        LayoutUnit::max(),
        LayoutUnit::max(),
    );

    let bp_val = {
        let b = resolve_border(style);
        let p = resolve_padding(style, zero);
        b.top + b.bottom + p.top + p.bottom
    };

    let min_bb = if min_raw > zero {
        if style.box_sizing == BoxSizing::ContentBox {
            min_raw + bp_val
        } else {
            min_raw.max_of(bp_val)
        }
    } else {
        zero
    };
    let max_bb = if max_raw == LayoutUnit::max() {
        max_raw
    } else if style.box_sizing == BoxSizing::ContentBox {
        max_raw + bp_val
    } else {
        max_raw.max_of(bp_val)
    };

    // CSS Sizing 4 §5.2: transferred min/max through aspect-ratio.
    // If min-width or max-width is definite and AR is present, transfer to block axis.
    let (mut min_bb, mut max_bb) = (min_bb, max_bb);
    if let Some(ref ar) = style.aspect_ratio {
        if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
            let bp_inline = {
                let b = resolve_border(style);
                let p = resolve_padding(style, zero);
                b.left + b.right + p.left + p.right
            };
            let ar_uses_border_box = !ar.auto_flag && style.box_sizing == BoxSizing::BorderBox;

            // Transfer min-width → min-height (only if min-height is auto/0)
            let min_w_raw = resolve_length(&style.min_width, indefinite, zero, zero);
            if min_w_raw > zero && min_bb == zero {
                if ar_uses_border_box {
                    // AR applies to border-box: border_box_h = border_box_w * h/w
                    min_bb = LayoutUnit::from_f32(min_w_raw.to_f32() * ar.ratio.1 / ar.ratio.0);
                } else {
                    let content_min_w = if style.box_sizing == BoxSizing::BorderBox {
                        (min_w_raw - bp_inline).clamp_negative_to_zero()
                    } else {
                        min_w_raw
                    };
                    let transferred_min_h =
                        LayoutUnit::from_f32(content_min_w.to_f32() * ar.ratio.1 / ar.ratio.0);
                    min_bb = transferred_min_h + bp_val;
                }
            }

            // Transfer max-width → max-height (only if max-height is unconstrained)
            let max_w_raw = resolve_length(
                &style.max_width,
                indefinite,
                LayoutUnit::max(),
                LayoutUnit::max(),
            );
            if max_w_raw < LayoutUnit::max() && max_bb == LayoutUnit::max() {
                if ar_uses_border_box {
                    max_bb = LayoutUnit::from_f32(max_w_raw.to_f32() * ar.ratio.1 / ar.ratio.0);
                } else {
                    let content_max_w = if style.box_sizing == BoxSizing::BorderBox {
                        (max_w_raw - bp_inline).clamp_negative_to_zero()
                    } else {
                        max_w_raw
                    };
                    let transferred_max_h =
                        LayoutUnit::from_f32(content_max_w.to_f32() * ar.ratio.1 / ar.ratio.0);
                    max_bb = transferred_max_h + bp_val;
                }
            }
        }
    }

    size.clamp(min_bb, max_bb)
}
mod tests {
    use super::*;

    #[test]
    fn inline_flex_intrinsic_size_includes_negative_item_margin() {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(container, |style| {
            style.display = openui_style::Display::InlineFlex;
        });
        doc.append_child(doc.root(), container);

        for (width, margin_left) in [(40.0, 0.0), (20.0, -40.0), (20.0, 0.0)] {
            let item = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(item, |style| {
                style.width = Length::px(width);
                style.margin_left = Length::px(margin_left);
            });
            doc.append_child(container, item);
        }

        let sizes = compute_logical_intrinsic_inline_sizes(&doc, container);
        assert_eq!(sizes.max, LayoutUnit::from_i32(40));
    }

    #[test]
    fn wrapped_flex_max_content_cannot_shrink_below_min_content() {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(container, |style| {
            style.display = openui_style::Display::Flex;
            style.flex_wrap = openui_style::FlexWrap::Wrap;
        });
        doc.append_child(doc.root(), container);

        for (width, margin_left) in [(100.0, 0.0), (0.0, -10.0)] {
            let item = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(item, |style| {
                style.width = Length::px(width);
                style.margin_left = Length::px(margin_left);
            });
            doc.append_child(container, item);
        }

        let sizes = compute_logical_intrinsic_inline_sizes(&doc, container);
        assert_eq!(sizes.min, LayoutUnit::from_i32(100));
        assert_eq!(sizes.max, sizes.min);
    }

    #[test]
    fn intrinsic_text_width_preserves_positive_remainders_at_layout_unit_boundaries() {
        assert_eq!(
            intrinsic_text_width(208.000_396_729),
            LayoutUnit::from_raw(208 * 64 + 1)
        );
        assert_eq!(
            intrinsic_text_width(48.01),
            LayoutUnit::from_raw(48 * 64 + 1)
        );
    }

    #[test]
    fn degenerate_preferred_ratio_preserves_replaced_natural_ratio() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| {
            computed.aspect_ratio = Some(openui_style::AspectRatio {
                ratio: (0.0, 1.0),
                auto_flag: false,
            })
        });

        let sizes =
            compute_replaced_intrinsic_sizes_with_natural(&style, 1.0, 1.0, Some((1.0, 1.0)));
        assert_eq!(sizes.max_content_inline_size, LayoutUnit::from_i32(100));
        assert_eq!(sizes.max_content_block_size, LayoutUnit::from_i32(100));
    }

    #[test]
    fn intrinsic_sizes_zero_default() {
        let sizes = IntrinsicSizes::zero();
        assert_eq!(sizes.min_content_inline_size, LayoutUnit::zero());
        assert_eq!(sizes.max_content_inline_size, LayoutUnit::zero());
        assert_eq!(sizes.min_content_block_size, LayoutUnit::zero());
        assert_eq!(sizes.max_content_block_size, LayoutUnit::zero());
    }

    #[test]
    fn shrink_to_fit_uses_max_when_available_exceeds() {
        let min = LayoutUnit::from_i32(50);
        let max = LayoutUnit::from_i32(200);
        let available = LayoutUnit::from_i32(300);
        // min(max(50, 300), 200) = min(300, 200) = 200
        assert_eq!(shrink_to_fit_inline_size(min, max, available), max);
    }

    #[test]
    fn shrink_to_fit_uses_available_in_between() {
        let min = LayoutUnit::from_i32(50);
        let max = LayoutUnit::from_i32(200);
        let available = LayoutUnit::from_i32(150);
        // min(max(50, 150), 200) = min(150, 200) = 150
        assert_eq!(
            shrink_to_fit_inline_size(min, max, available),
            LayoutUnit::from_i32(150)
        );
    }

    #[test]
    fn shrink_to_fit_uses_min_when_available_too_small() {
        let min = LayoutUnit::from_i32(100);
        let max = LayoutUnit::from_i32(200);
        let available = LayoutUnit::from_i32(50);
        // min(max(100, 50), 200) = min(100, 200) = 100
        assert_eq!(shrink_to_fit_inline_size(min, max, available), min);
    }

    #[test]
    fn intrinsic_calc_min_width_keeps_definite_term() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| computed.min_width = Length::calc_percent_px(0.0, 160.0));
        let size = apply_min_max_inline(
            &style,
            LayoutUnit::from_i32(60),
            (LayoutUnit::from_i32(60), LayoutUnit::from_i32(60)),
            false,
        );
        assert_eq!(size, LayoutUnit::from_i32(160));
    }

    #[test]
    fn text_min_content_widest_word() {
        let sizes = compute_text_intrinsic_sizes(
            &Document::new(),
            "hello world",
            &ComputedStyle::default(),
        );
        assert!(sizes.min > LayoutUnit::zero());
        assert!(sizes.max > sizes.min);
    }

    #[test]
    fn text_single_word_min_equals_max() {
        let sizes = compute_text_intrinsic_sizes(
            &Document::new(),
            "indivisible",
            &ComputedStyle::default(),
        );
        // Both min and max are the full word
        assert_eq!(sizes.min, sizes.max);
    }

    #[test]
    fn collapsible_boundary_whitespace_does_not_inflate_intrinsic_width() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| {
            computed.font_family.families = vec![FontFamily::Named("Droid Sans Fallback".into())]
        });
        assert_eq!(
            compute_text_intrinsic_sizes(&Document::new(), "\n  vertical-rl:\n  ", &style),
            compute_text_intrinsic_sizes(&Document::new(), "vertical-rl:", &style)
        );
    }

    #[test]
    fn collapsible_interior_whitespace_retains_one_advance() {
        let style = ComputedStyle::default();
        let sizes = compute_text_intrinsic_sizes_impl(&Document::new(), "\n  ", &style, true);
        assert_eq!(sizes.min, LayoutUnit::zero());
        assert!(sizes.max > LayoutUnit::zero());
    }

    #[test]
    fn non_breaking_space_is_not_trimmed_as_css_whitespace() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| {
            computed.font_family.families = vec![FontFamily::Named("Droid Sans Fallback".into())]
        });
        let sizes = compute_text_intrinsic_sizes(&Document::new(), "\u{00a0}", &style);
        assert!(sizes.min > LayoutUnit::zero());
        assert_eq!(sizes.min, sizes.max);
    }

    #[test]
    fn fit_content_minimum_preserves_both_intrinsic_endpoints() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| computed.width = Length::px(10.0));
        style.update_derived(|computed| computed.min_width = Length::fit_content());
        let endpoints = (LayoutUnit::from_i32(50), LayoutUnit::from_i32(100));
        assert_eq!(
            apply_min_max_inline(&style, LayoutUnit::from_i32(10), endpoints, false),
            endpoints.0,
        );
        assert_eq!(
            apply_min_max_inline(&style, LayoutUnit::from_i32(10), endpoints, true),
            endpoints.1,
        );
    }

    #[test]
    fn negative_inline_margin_cannot_invert_intrinsic_endpoints() {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.append_child(doc.root(), container);

        let fixed = doc.create_node(ElementTag::Span);
        doc.update_resolved_style(fixed, |style| {
            style.display = openui_style::Display::InlineBlock
        });
        doc.update_resolved_style(fixed, |style| style.width = Length::px(100.0));
        doc.append_child(container, fixed);

        let negative = doc.create_node(ElementTag::Span);
        doc.update_resolved_style(negative, |style| {
            style.display = openui_style::Display::InlineBlock
        });
        doc.update_resolved_style(negative, |style| style.margin_right = Length::px(-50.0));
        doc.append_child(container, negative);

        let sizes = compute_intrinsic_block_sizes(&doc, container);
        assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_i32(100));
        assert_eq!(sizes.max_content_inline_size, LayoutUnit::from_i32(100));
    }

    #[test]
    fn intrinsic_calc_max_width_with_percentage_is_unconstrained() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| computed.max_width = Length::calc_percent_px(0.0, 40.0));
        let size = apply_min_max_inline(
            &style,
            LayoutUnit::from_i32(80),
            (LayoutUnit::from_i32(80), LayoutUnit::from_i32(80)),
            false,
        );
        assert_eq!(size, LayoutUnit::from_i32(80));
    }

    #[test]
    fn adjacent_inline_blocks_wrap_for_min_content_but_not_max_content() {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.append_child(doc.root(), container);
        for _ in 0..5 {
            let atomic = doc.create_node(ElementTag::Span);
            doc.update_resolved_style(atomic, |style| {
                style.display = openui_style::Display::InlineBlock;
                style.width = Length::px(25.0);
            });
            doc.append_child(container, atomic);
        }

        assert_eq!(
            compute_inline_sequence_intrinsic_sizes(&doc, container),
            MinMaxSizes::new(LayoutUnit::from_i32(25), LayoutUnit::from_i32(125))
        );
    }

    #[test]
    fn max_content_combines_float_and_unbroken_inline_line() {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.append_child(doc.root(), container);

        let float = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(float, |float_style| {
            float_style.float = openui_style::Float::Right;
            float_style.width = Length::px(100.0);
            float_style.height = Length::px(200.0);
        });
        doc.append_child(container, float);

        let atomic = doc.create_node(ElementTag::Span);
        doc.update_resolved_style(atomic, |atomic_style| {
            atomic_style.display = openui_style::Display::InlineBlock;
            atomic_style.width = Length::px(100.0);
            atomic_style.height = Length::px(200.0);
        });
        doc.append_child(container, atomic);

        let sizes = compute_intrinsic_block_sizes(&doc, container);
        assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_i32(100));
        assert_eq!(sizes.max_content_inline_size, LayoutUnit::from_i32(200));
    }

    #[test]
    fn forced_break_separates_float_intrinsic_rows() {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.append_child(doc.root(), container);

        let first = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(first, |first_style| {
            first_style.float = openui_style::Float::Left;
            first_style.width = Length::px(100.0);
        });
        doc.append_child(container, first);

        let br = doc.create_node(ElementTag::Break);
        doc.append_child(container, br);

        let second = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(second, |second_style| {
            second_style.float = openui_style::Float::Left;
            second_style.width = Length::px(60.0);
        });
        doc.append_child(container, second);

        let sizes = compute_intrinsic_block_sizes(&doc, container);
        assert_eq!(sizes.min_content_inline_size, LayoutUnit::from_i32(100));
        assert_eq!(sizes.max_content_inline_size, LayoutUnit::from_i32(100));
    }

    #[test]
    fn cleared_float_starts_a_new_intrinsic_row() {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.append_child(doc.root(), container);

        let first = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(first, |first_style| {
            first_style.float = openui_style::Float::Left;
            first_style.width = Length::px(68.0);
        });
        doc.append_child(container, first);

        let second = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(second, |second_style| {
            second_style.float = openui_style::Float::Left;
            second_style.clear = Clear::Both;
            second_style.width = Length::px(44.0);
        });
        doc.append_child(container, second);

        let sizes = compute_intrinsic_block_sizes(&doc, container);
        assert_eq!(sizes.max_content_inline_size, LayoutUnit::from_i32(68));
    }

    #[test]
    fn vertical_inline_intrinsic_uses_atomic_height_and_forced_lines() {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(container, |style| {
            style.writing_mode = openui_style::WritingMode::VerticalLr
        });
        doc.append_child(doc.root(), container);

        for index in 0..2 {
            let atomic = doc.create_node(ElementTag::Span);
            doc.update_resolved_style(atomic, |style| {
                style.display = openui_style::Display::InlineBlock;
                style.writing_mode = openui_style::WritingMode::VerticalLr;
                style.width = Length::px(15.0);
                style.height = Length::px(45.0);
            });
            doc.append_child(container, atomic);

            if index == 0 {
                let line_break = doc.create_node(ElementTag::Break);
                doc.update_resolved_style(line_break, |style| {
                    style.writing_mode = openui_style::WritingMode::VerticalLr
                });
                doc.append_child(container, line_break);
            }
        }

        let sizes = compute_logical_intrinsic_inline_sizes(&doc, container);
        assert_eq!(
            sizes,
            MinMaxSizes::new(LayoutUnit::from_i32(45), LayoutUnit::from_i32(45))
        );
    }

    #[test]
    fn vertical_inline_intrinsic_maxes_normal_flow_block_children() {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(container, |style| {
            style.writing_mode = openui_style::WritingMode::VerticalLr
        });
        doc.append_child(doc.root(), container);

        for _ in 0..2 {
            let child = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(child, |style| {
                style.display = openui_style::Display::Flex;
                style.writing_mode = openui_style::WritingMode::VerticalLr;
                style.height = Length::px(48.0);
            });
            doc.append_child(container, child);
        }

        let sizes = compute_logical_intrinsic_inline_sizes(&doc, container);
        assert_eq!(
            sizes,
            MinMaxSizes::new(LayoutUnit::from_i32(48), LayoutUnit::from_i32(48))
        );
    }

    #[test]
    fn line_break_anywhere_uses_character_min_content_opportunities() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| {
            computed.font_family.families = vec![FontFamily::Named("Droid Sans Fallback".into())]
        });
        let normal = compute_text_intrinsic_sizes(&Document::new(), "fragmentation", &style);

        style.update_derived(|computed| computed.line_break = LineBreak::Anywhere);
        let anywhere = compute_text_intrinsic_sizes(&Document::new(), "fragmentation", &style);

        assert!(anywhere.min < normal.min);
        assert_eq!(anywhere.max, normal.max);
    }

    #[test]
    fn zero_column_width_does_not_add_hypothetical_min_content_gaps() {
        let mut doc = Document::new();
        let multicol = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(multicol, |multicol_style| {
            multicol_style.column_count = Some(3);
            multicol_style.column_width = Some(Length::px(0.0));
            multicol_style.column_gap = Some(Length::px(20.0));
        });
        doc.append_child(doc.root(), multicol);

        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(child, |style| style.width = Length::px(100.0));
        doc.append_child(multicol, child);

        let sizes = compute_intrinsic_block_sizes(&doc, multicol);
        assert_eq!(sizes.min_content_inline_size, LayoutUnit::zero());
        assert!(sizes.max_content_inline_size >= LayoutUnit::from_i32(100));
    }
}
