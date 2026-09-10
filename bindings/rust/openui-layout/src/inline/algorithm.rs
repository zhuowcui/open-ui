//! Inline layout algorithm — entry point for inline formatting context.
//!
//! Mirrors Blink's `InlineLayoutAlgorithm` from
//! `third_party/blink/renderer/core/layout/inline/inline_layout_algorithm.cc`.
//!
//! Takes a block node that has inline children and produces a Fragment
//! containing positioned line box fragments with text fragments inside.
//!
//! The algorithm follows CSS 2.2 §10.6.1 (inline formatting context),
//! §10.8 (line height calculations), and §16.2 (text alignment).

use openui_dom::{Document, ElementTag, NodeId, PseudoElementKind};
use openui_geometry::{BoxStrut, LayoutUnit, PhysicalOffset, PhysicalSize, WritingModeConverter};
use openui_style::{
    BoxDecorationBreak, Clear, ComputedStyle, Direction, Display, Float, FontFamily, LineHeight,
    TextAlign, TextAlignLast, TextJustify, VerticalAlign, WhiteSpace,
};
use openui_text::{
    used_line_height, used_line_height_metrics, Font, FontMetrics, ShapeResult, TextShaper,
    UsedLineHeightMetrics,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

use crate::constraint_space::ConstraintSpace;
use crate::exclusions::{ClearType, ExclusionSpace};
use crate::fragment::{resolve_text_run_orientation, Fragment, FragmentKind};
use crate::length_resolver::{resolve_length, resolve_margin_or_padding};
use crate::out_of_flow::OutOfFlowCandidate;

use super::items::{InlineItemResult, InlineItemType};
use super::items_builder::{
    style_to_font_description, FloatPlaceholder, InlineItemsBuilder, InlineItemsData,
};
use super::line_breaker::{byte_to_char_offset, find_break_opportunities, LineBreaker};
use super::line_info::LineInfo;
use super::line_width::{
    compute_line_availability, compute_line_availability_for_block_size, next_float_bottom,
};

fn uses_deterministic_text_profile(style: &ComputedStyle) -> bool {
    style.font_family.families.iter().any(|family| {
        matches!(family, FontFamily::Named(name) if name.eq_ignore_ascii_case("Droid Sans Fallback"))
    })
}

fn configure_line_breaker(
    breaker: &mut LineBreaker<'_>,
    style: &ComputedStyle,
    space: &ConstraintSpace,
    first_float_text_offset: Option<usize>,
) {
    breaker.set_first_float_text_offset(first_float_text_offset);
    breaker.set_writing_direction(space.writing_direction);
    breaker.set_text_align(style.text_align);
    breaker.set_container_white_space(style.white_space);
    breaker.set_preserve_leading_inline_fragment(
        style.line_clamp != openui_style::LineClamp::None
            || style.text_wrap == openui_style::TextWrap::Balance
            || space.line_clamp_context.is_some(),
    );
    breaker.set_hyphens(style.hyphens, style.hyphenate_limit_chars);
}

fn line_count_at_width(
    items_data: &InlineItemsData,
    containing_width: LayoutUnit,
    line_width: LayoutUnit,
    style: &ComputedStyle,
    space: &ConstraintSpace,
    first_float_text_offset: Option<usize>,
) -> usize {
    let mut breaker = LineBreaker::new(items_data, containing_width);
    configure_line_breaker(&mut breaker, style, space, first_float_text_offset);
    let mut count = 0;
    while !breaker.is_finished() && count <= 7 {
        if breaker.next_line(line_width).is_none() {
            break;
        }
        count += 1;
    }
    count
}

/// Blink balances short paragraphs by finding the narrowest greedy line
/// width that preserves the paragraph's line count. The actual line boxes
/// still occupy the containing block; only their wrap constraint is reduced.
fn balanced_wrap_width(
    items_data: &InlineItemsData,
    available_width: LayoutUnit,
    style: &ComputedStyle,
    space: &ConstraintSpace,
    first_float_text_offset: Option<usize>,
) -> Option<LayoutUnit> {
    if style.text_wrap != openui_style::TextWrap::Balance
        || style.first_line_style.is_some()
        || space.exclusion_space.is_some()
        || matches!(style.line_clamp, openui_style::LineClamp::Lines(_))
        || space
            .line_clamp_context
            .as_ref()
            .is_some_and(|context| context.remaining_block_size().is_none())
    {
        return None;
    }
    let line_count = line_count_at_width(
        items_data,
        available_width,
        available_width,
        style,
        space,
        first_float_text_offset,
    );
    if !(2..=6).contains(&line_count) {
        return None;
    }

    let mut low = 1;
    let mut high = available_width.raw().max(1);
    while low < high {
        let mid = low + (high - low) / 2;
        let candidate = LayoutUnit::from_raw(mid);
        let candidate_lines = line_count_at_width(
            items_data,
            available_width,
            candidate,
            style,
            space,
            first_float_text_offset,
        );
        if candidate_lines <= line_count {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    Some(LayoutUnit::from_raw(low))
}

fn node_follows_float_with_in_flow_text(
    doc: &Document,
    node_id: NodeId,
    saw_float: &mut bool,
) -> bool {
    let node = doc.node(node_id);
    if node.style.display == Display::None {
        return false;
    }
    if node.style.float != Float::None {
        *saw_float = true;
        return false;
    }
    if node.style.is_out_of_flow() {
        return false;
    }
    if node.tag == ElementTag::Text {
        return *saw_float
            && node
                .text
                .as_deref()
                .is_some_and(|text| text.chars().any(|ch| !ch.is_whitespace()));
    }
    doc.children(node_id)
        .any(|child_id| node_follows_float_with_in_flow_text(doc, child_id, saw_float))
}

/// Provisional source positions for floats in one IFC.
///
/// Lines are broken without exclusions first so a float encountered after
/// already-laid text starts at that line's insertion boundary. The owning BFC
/// then decides whether the float still fits beside the preceding inline
/// advance or must move to the following line.
#[derive(Clone, Copy, Debug)]
pub(crate) struct InlineFloatSourcePosition {
    pub block_offset: LayoutUnit,
    pub preceding_inline_size: LayoutUnit,
    pub line_height: LayoutUnit,
}

fn float_line_break_reservations(
    doc: &Document,
    floats: &[FloatPlaceholder],
    available_inline_size: LayoutUnit,
    space: &ConstraintSpace,
) -> HashMap<NodeId, LayoutUnit> {
    floats
        .iter()
        .filter_map(|float| {
            let style = &doc.node(float.node_id).style;
            let inline_length = if space.writing_direction.is_horizontal() {
                &style.width
            } else {
                &style.height
            };
            let border_box = if inline_length.is_auto() {
                crate::out_of_flow::compute_shrink_to_fit_width(
                    doc,
                    float.node_id,
                    available_inline_size,
                )
            } else {
                resolve_length(
                    inline_length,
                    available_inline_size,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                )
            };
            let (margin_start, margin_end) = if space.writing_direction.is_horizontal() {
                (
                    resolve_margin_or_padding(&style.margin_left, available_inline_size),
                    resolve_margin_or_padding(&style.margin_right, available_inline_size),
                )
            } else {
                (
                    resolve_margin_or_padding(&style.margin_top, available_inline_size),
                    resolve_margin_or_padding(&style.margin_bottom, available_inline_size),
                )
            };
            let margin_box = (margin_start + border_box + margin_end).clamp_negative_to_zero();
            (margin_box < border_box).then_some((float.node_id, border_box))
        })
        .collect()
}

fn line_reaches_float_source(
    line: &LineInfo,
    data: &InlineItemsData,
    float: &FloatPlaceholder,
) -> bool {
    if float.text_offset == 0 && float.inline_ancestor.is_none() {
        return true;
    }
    line.items.iter().any(|result| {
        let item = &data.items[result.item_index];
        match result.item_type {
            InlineItemType::Text | InlineItemType::AtomicInline => {
                item.text_range.start <= float.text_offset
                    && float.text_offset <= item.text_range.end
            }
            InlineItemType::OpenTag => {
                float.inline_ancestor == Some(item.node_id)
                    && item.text_range.start == float.inline_ancestor_text_offset
            }
            InlineItemType::CloseTag | InlineItemType::Control | InlineItemType::BlockInInline => {
                false
            }
        }
    })
}

fn pending_float_line_reservation(
    floats: &[FloatPlaceholder],
    reservations: &HashMap<NodeId, LayoutUnit>,
    consumed: &HashSet<NodeId>,
) -> LayoutUnit {
    floats
        .iter()
        .find(|float| !consumed.contains(&float.node_id))
        .and_then(|float| reservations.get(&float.node_id).copied())
        .unwrap_or_default()
}

fn consume_pending_float_line_reservation(
    floats: &[FloatPlaceholder],
    reservations: &HashMap<NodeId, LayoutUnit>,
    consumed: &mut HashSet<NodeId>,
) {
    if let Some(float) = floats.iter().find(|float| {
        !consumed.contains(&float.node_id) && reservations.contains_key(&float.node_id)
    }) {
        consumed.insert(float.node_id);
    }
}

pub(crate) fn inline_float_source_positions(
    doc: &Document,
    node_id: NodeId,
    items_data: &InlineItemsData,
    floats: &[FloatPlaceholder],
    available_inline_size: LayoutUnit,
    space: &ConstraintSpace,
) -> HashMap<NodeId, InlineFloatSourcePosition> {
    if floats.is_empty() {
        return HashMap::new();
    }
    let style = &doc.node(node_id).style;
    let mut data = items_data.clone();
    let base_direction = if style.unicode_bidi == openui_style::UnicodeBidi::Plaintext {
        None
    } else if style.direction == Direction::Rtl {
        Some(openui_text::TextDirection::Rtl)
    } else {
        Some(openui_text::TextDirection::Ltr)
    };
    data.apply_bidi(base_direction);
    data.split_shaping_runs();
    data.shape_text();

    let mut breaker = LineBreaker::new(&data, available_inline_size);
    configure_line_breaker(&mut breaker, style, space, None);
    let reservations = float_line_break_reservations(doc, floats, available_inline_size, space);
    let mut consumed = HashSet::new();
    let font = Font::new(style_to_font_description(style));
    let metrics = font.font_metrics().copied().unwrap_or_default();
    let line_metrics = compute_line_height_metrics(&metrics, &style.line_height, style.font_size);
    let line_height = LayoutUnit::from_f32(line_metrics.ascent + line_metrics.descent);
    let mut block_offset = LayoutUnit::zero();
    let mut lines = Vec::new();
    let mut last_line = InlineFloatSourcePosition {
        block_offset,
        preceding_inline_size: LayoutUnit::zero(),
        line_height,
    };

    while !breaker.is_finished() {
        let reservation = pending_float_line_reservation(floats, &reservations, &consumed);
        let line_width = (available_inline_size - reservation).clamp_negative_to_zero();
        let Some(line) = breaker.next_line(line_width) else {
            break;
        };
        if reservation > LayoutUnit::zero() {
            consume_pending_float_line_reservation(floats, &reservations, &mut consumed);
        }
        for float in floats {
            if line_reaches_float_source(&line, &data, float) {
                consumed.insert(float.node_id);
            }
        }
        last_line = InlineFloatSourcePosition {
            block_offset,
            preceding_inline_size: line.used_width,
            line_height,
        };
        lines.push((block_offset, line));
        block_offset = block_offset + line_height;
    }
    let mut result = HashMap::new();
    for placeholder in floats {
        // A float immediately after <br> starts at the following line's
        // block position. The line breaker keeps the forced-break control on
        // the preceding line and does not materialize an empty successor, so
        // recover that insertion boundary explicitly from the control item.
        let forced_break_line = data
            .items
            .iter()
            .enumerate()
            .find(|(_, item)| {
                item.item_type == InlineItemType::Control
                    && doc.node(item.node_id).tag == ElementTag::Break
                    && item.is_end_collapsible_newline
                    && item.text_range.end == placeholder.text_offset
            })
            // A clearing <br> already moves subsequent floats below the
            // exclusion space. Adding a synthetic successor-line offset as
            // well would double-count the break's block progression.
            .filter(|(_, item)| doc.node(item.node_id).style.clear == Clear::None)
            .and_then(|(_, break_item)| {
                // Forced-break controls are consumed by LineBreaker and may
                // not appear in LineInfo::items. Locate the last materialized
                // line whose content ends at or before the control boundary.
                lines
                    .iter()
                    .rev()
                    .find(|(_, line)| {
                        line.items
                            .iter()
                            .any(|result| result.text_range.end <= break_item.text_range.start)
                    })
                    .or_else(|| lines.first())
            })
            .or_else(|| {
                // Older static ports represent `<br>` as a preserved newline
                // on a text item. It establishes the same successor-line
                // source boundary as a structural Break control.
                let has_preserved_text_break = data.items.iter().any(|item| {
                    item.item_type == InlineItemType::Text
                        && item.is_end_collapsible_newline
                        && item.text_range.end == placeholder.text_offset
                        && matches!(
                            data.styles[item.style_index].white_space,
                            openui_style::WhiteSpace::Pre
                                | openui_style::WhiteSpace::PreWrap
                                | openui_style::WhiteSpace::PreLine
                                | openui_style::WhiteSpace::BreakSpaces
                        )
                });
                has_preserved_text_break.then(|| {
                    lines
                        .iter()
                        .rev()
                        .find(|(_, line)| {
                            line.items.iter().any(|result| {
                                result.has_forced_break
                                    && result.text_range.end <= placeholder.text_offset
                            })
                        })
                        .or_else(|| lines.first())
                })?
            });
        let has_text_inside_inline = placeholder.inline_ancestor.is_none()
            || placeholder.text_offset > placeholder.inline_ancestor_text_offset;
        let boundary_preserves_newlines = data.items.iter().rev().any(|item| {
            item.item_type == InlineItemType::Text
                && item.text_range.start < placeholder.text_offset
                && placeholder.text_offset <= item.text_range.end
                && matches!(
                    data.styles[item.style_index].white_space,
                    openui_style::WhiteSpace::Pre
                        | openui_style::WhiteSpace::PreWrap
                        | openui_style::WhiteSpace::PreLine
                        | openui_style::WhiteSpace::BreakSpaces
                )
        });
        let source_line = if placeholder.text_offset == 0 && placeholder.inline_ancestor.is_none() {
            lines.first()
        } else if has_text_inside_inline {
            lines
                .iter()
                .find(|(_, line)| {
                    line.items.iter().any(|item| {
                        let source_range = &data.items[item.item_index].text_range;
                        let line_range = &item.text_range;
                        let range = if boundary_preserves_newlines {
                            line_range
                        } else {
                            source_range
                        };
                        item.item_type == InlineItemType::Text
                            && range.start < placeholder.text_offset
                            && placeholder.text_offset <= range.end
                    })
                })
                // A float immediately after a forced break has no preceding
                // text item whose range contains its boundary. Anchor it to
                // the first following text line instead of falling back to
                // the final line of the IFC.
                .or_else(|| {
                    lines.iter().find(|(_, line)| {
                        line.items.iter().any(|item| {
                            let source_range = &data.items[item.item_index].text_range;
                            let line_range = &item.text_range;
                            let range = if boundary_preserves_newlines {
                                line_range
                            } else {
                                source_range
                            };
                            item.item_type == InlineItemType::Text
                                && range.start >= placeholder.text_offset
                        })
                    })
                })
        } else {
            placeholder.inline_ancestor.and_then(|ancestor| {
                lines.iter().find(|(_, line)| {
                    line.items.iter().any(|result| {
                        let item = &data.items[result.item_index];
                        item.item_type == InlineItemType::OpenTag && item.node_id == ancestor
                    })
                })
            })
        };
        let mut position = forced_break_line.map_or_else(
            || {
                source_line.map_or(last_line, |(source_block, line)| {
                    let preceding_inline_size = line
                        .items
                        .iter()
                        .filter(|result| {
                            let item = &data.items[result.item_index];
                            match result.item_type {
                                InlineItemType::Text => {
                                    result.text_range.end <= placeholder.text_offset
                                }
                                InlineItemType::OpenTag | InlineItemType::CloseTag => {
                                    item.text_range.start <= placeholder.text_offset
                                }
                                InlineItemType::AtomicInline => {
                                    item.text_range.end <= placeholder.text_offset
                                }
                                InlineItemType::Control | InlineItemType::BlockInInline => false,
                            }
                        })
                        .fold(LayoutUnit::zero(), |sum, item| sum + item.inline_size);
                    InlineFloatSourcePosition {
                        block_offset: *source_block,
                        preceding_inline_size,
                        line_height,
                    }
                })
            },
            |(source_block, _)| InlineFloatSourcePosition {
                block_offset: *source_block + line_height,
                preceding_inline_size: LayoutUnit::zero(),
                line_height,
            },
        );
        // A shaped text item may span several preserved newline boundaries.
        // The source-position probe's line association can then resolve the
        // float against the first split result for that item. Source order
        // still gives us a strict lower bound: every preserved newline before
        // the placeholder advances by one line strut. Keep any larger offset
        // found by actual wrapping, but never place the float above that
        // forced-break floor.
        let preserved_break_count = data
            .items
            .iter()
            .filter(|item| item.item_type == InlineItemType::Text)
            .filter(|item| {
                matches!(
                    data.styles[item.style_index].white_space,
                    openui_style::WhiteSpace::Pre
                        | openui_style::WhiteSpace::PreWrap
                        | openui_style::WhiteSpace::PreLine
                        | openui_style::WhiteSpace::BreakSpaces
                )
            })
            .map(|item| {
                let start = item.text_range.start.min(placeholder.text_offset);
                let end = item.text_range.end.min(placeholder.text_offset);
                data.text[start..end]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
            })
            .sum::<usize>();
        let forced_break_floor = line_height * preserved_break_count as i32;
        if forced_break_floor > position.block_offset {
            position.block_offset = forced_break_floor;
            if data
                .text
                .as_bytes()
                .get(placeholder.text_offset.saturating_sub(1))
                == Some(&b'\n')
            {
                position.preceding_inline_size = LayoutUnit::zero();
            }
        }
        result.insert(placeholder.node_id, position);
    }
    result
}

/// Lay out one HTML ruby container as an atomic inline object.
///
/// Base content and `<rt>` content each establish an internal inline
/// formatting context. They share the wider inline size and stack in the
/// block direction. Exporting the base's baseline lets adjacent over/under
/// ruby objects expand one common line box on opposite sides of that base.
fn ruby_internal_content_height(fragment: &Fragment) -> LayoutUnit {
    fragment
        .children
        .iter()
        .map(|line| {
            let line_content_height = line
                .children
                .iter()
                .map(|child| child.offset.top + child.size.height)
                .max()
                .unwrap_or(line.size.height);
            line.offset.top + line_content_height
        })
        .max()
        .unwrap_or(fragment.size.height)
}

fn fragment_text_ink_bounds(
    fragment: &Fragment,
    parent_block_offset: LayoutUnit,
) -> Option<(LayoutUnit, LayoutUnit)> {
    let block_offset = parent_block_offset + fragment.offset.top;
    if fragment.kind == FragmentKind::Text {
        return Some((block_offset, block_offset + fragment.size.height));
    }
    fragment
        .children
        .iter()
        .filter_map(|child| fragment_text_ink_bounds(child, block_offset))
        .fold(None, |bounds, child_bounds| {
            Some(bounds.map_or(child_bounds, |(start, end)| {
                (start.min_of(child_bounds.0), end.max_of(child_bounds.1))
            }))
        })
}

fn fragment_text_ink_height(fragment: &Fragment) -> LayoutUnit {
    fragment_text_ink_bounds(fragment, LayoutUnit::zero())
        .map_or(LayoutUnit::zero(), |(start, end)| end - start)
}

fn fragment_descendant_text_ink_end(fragment: &Fragment) -> Option<LayoutUnit> {
    fragment
        .children
        .iter()
        .filter_map(|child| fragment_text_ink_bounds(child, LayoutUnit::zero()))
        .map(|(_, end)| end)
        .max()
}

fn fragment_contains_tag(doc: &Document, fragment: &Fragment, tag: ElementTag) -> bool {
    (!fragment.node_id.is_none() && doc.node(fragment.node_id).tag == tag)
        || fragment
            .children
            .iter()
            .any(|child| fragment_contains_tag(doc, child, tag))
}

/// CSS 2.1 exposes an inline-block's last in-flow line-box baseline. A block
/// child's synthesized baseline (for example a table row baseline) is useful
/// to flex/grid alignment, but it must not turn a line-less inline-block into
/// one with a line baseline.
fn inline_block_last_line_baseline(fragment: &Fragment) -> Option<LayoutUnit> {
    fn contains_anonymous_line(fragment: &Fragment) -> bool {
        fragment.children.iter().any(|child| {
            (child.node_id.is_none()
                && (child.last_baseline.is_some()
                    || child.first_baseline.is_some()
                    || (child.kind == FragmentKind::Box && child.baseline_offset > 0.0)))
                || contains_anonymous_line(child)
        })
    }

    if !contains_anonymous_line(fragment) {
        return None;
    }
    // The formatting context's exported baseline is already projected onto
    // the parent's physical baseline axis. This matters for vertical writing,
    // where summing a descendant line's top offset reconstructs the wrong
    // physical coordinate.
    if let Some(baseline) = fragment.last_baseline.or(fragment.first_baseline) {
        return Some(baseline);
    }
    for child in fragment.children.iter().rev() {
        if child.node_id.is_none() {
            if let Some(baseline) = child.last_baseline.or(child.first_baseline) {
                return Some(child.offset.top + baseline);
            }
            // Anonymous IFC line fragments expose their baseline through the
            // line-local offset. They are not formatting-context roots, so
            // first_baseline/last_baseline are intentionally unset here.
            if child.kind == FragmentKind::Box && child.baseline_offset > 0.0 {
                return Some(child.offset.top + LayoutUnit::from_f32(child.baseline_offset));
            }
        }
        if let Some(baseline) = inline_block_last_line_baseline(child) {
            return Some(child.offset.top + baseline);
        }
    }
    None
}

fn align_ruby_internal_content(fragment: &mut Fragment, inline_size: LayoutUnit) {
    for line in &mut fragment.children {
        let Some(start) = line.children.iter().map(|child| child.offset.left).min() else {
            continue;
        };
        let end = line
            .children
            .iter()
            .map(|child| child.offset.left + child.size.width)
            .max()
            .unwrap_or(start);
        let shift = ((inline_size - (end - start)).clamp_negative_to_zero()
            / LayoutUnit::from_i32(2))
            - start;
        for child in &mut line.children {
            child.offset.left = child.offset.left + shift;
        }
    }
}

fn layout_ruby_atomic(
    doc: &Document,
    node_id: NodeId,
    inline_size: LayoutUnit,
    percentage_block_size: LayoutUnit,
    compact_for_clamp: bool,
) -> Fragment {
    let mut base_children = Vec::new();
    let mut annotation_parent = None;
    let mut annotation_children = Vec::new();

    for child_id in doc.children(node_id) {
        if doc.node(child_id).tag == ElementTag::RubyText {
            if annotation_parent.is_none() {
                annotation_parent = Some(child_id);
            }
            annotation_children.extend(doc.children(child_id));
        } else {
            base_children.push(child_id);
        }
    }
    let internal_space = ConstraintSpace::for_block_child(
        inline_size,
        openui_geometry::INDEFINITE_SIZE,
        inline_size,
        percentage_block_size,
        true,
    );
    let mut base = inline_layout_for_children(doc, node_id, &base_children, &internal_space);
    let mut annotation = annotation_parent.map(|parent| {
        inline_layout_for_children(doc, parent, &annotation_children, &internal_space)
    });
    if compact_for_clamp
        && matches!(
            doc.node(node_id).style.ruby_align,
            openui_style::RubyAlign::SpaceAround | openui_style::RubyAlign::Center
        )
    {
        align_ruby_internal_content(&mut base, inline_size);
        if let Some(annotation) = annotation.as_mut() {
            align_ruby_internal_content(annotation, inline_size);
        }
    }

    // A ruby base/annotation line area is sized to its contents. Ordinary
    // inline formatting retains the parent strut and its below-baseline
    // leading, but that leading must not become a gap between the two ruby
    // levels (CSS Ruby Layout §3.3).
    let base_height = ruby_internal_content_height(&base);
    base.size.height = base_height;
    let annotation_height = annotation
        .as_ref()
        .map(ruby_internal_content_height)
        .unwrap_or(LayoutUnit::zero());
    if let Some(annotation) = annotation.as_mut() {
        annotation.size.height = annotation_height;
    }
    let base_baseline = base.first_baseline.unwrap_or(base_height);
    if !compact_for_clamp {
        let ruby_position = doc.node(node_id).style.ruby_position;
        let mut children = Vec::with_capacity(2);
        let exported_baseline = if ruby_position.is_over() {
            if let Some(mut annotation) = annotation.take() {
                annotation.offset.top = LayoutUnit::zero();
                children.push(annotation);
            }
            base.offset.top = annotation_height;
            children.push(base);
            annotation_height + base_baseline
        } else {
            base.offset.top = LayoutUnit::zero();
            children.push(base);
            if let Some(mut annotation) = annotation.take() {
                annotation.offset.top = base_height;
                children.push(annotation);
            }
            base_baseline
        };
        let mut fragment = Fragment::new_box(
            node_id,
            PhysicalSize::new(inline_size, base_height + annotation_height),
        );
        fragment.children = children;
        fragment.first_baseline = Some(exported_baseline);
        fragment.last_baseline = Some(exported_baseline);
        fragment.baseline_offset = exported_baseline.to_f32();
        return fragment;
    }
    let base_leading_start = base
        .children
        .first()
        .and_then(|line| line.children.iter().map(|child| child.offset.top).min())
        .unwrap_or(LayoutUnit::zero());
    let annotation_leading_start = annotation
        .as_ref()
        .and_then(|fragment| fragment.children.first())
        .and_then(|line| line.children.iter().map(|child| child.offset.top).min())
        .unwrap_or(LayoutUnit::zero());
    let annotation_leading_trim = if annotation_parent
        .is_some_and(|parent| doc.node(parent).style.font_size < doc.node(node_id).style.font_size)
    {
        annotation_leading_start.ceil()
    } else {
        annotation_leading_start
    };
    let annotation_position_trim = annotation_leading_trim;
    let over_annotation_offset = if annotation_parent
        .is_some_and(|parent| doc.node(parent).style.font_size < doc.node(node_id).style.font_size)
    {
        (base_leading_start - LayoutUnit::from_i32(1)).clamp_negative_to_zero()
    } else {
        base_leading_start
    };
    let ruby_position = doc.node(node_id).style.ruby_position;

    let mut children = Vec::with_capacity(2);
    let exported_baseline = if ruby_position.is_over() {
        if let Some(mut annotation) = annotation.take() {
            // Align the annotation ink with the base ink, allowing it to use
            // the base line's start-side half-leading instead of leaving an
            // artificial gap between the two ruby levels.
            annotation.offset.top = over_annotation_offset;
            children.push(annotation);
        }
        base.offset.top = annotation_height;
        children.push(base);
        annotation_height + base_baseline
    } else {
        base.offset.top = LayoutUnit::zero();
        children.push(base);
        if let Some(mut annotation) = annotation.take() {
            // Adjacent ruby levels share their half-leading. Position the
            // annotation's ink immediately after the base ink instead of
            // inserting the annotation line's start-side leading as a gap.
            annotation.offset.top =
                (base_height - annotation_position_trim).clamp_negative_to_zero();
            children.push(annotation);
        }
        base_baseline
    };

    let ruby_height = if ruby_position.is_over() {
        base_height + annotation_height
    } else {
        base_height + (annotation_height - annotation_leading_trim).clamp_negative_to_zero()
    };

    let mut fragment = Fragment::new_box(node_id, PhysicalSize::new(inline_size, ruby_height));
    fragment.children = children;
    fragment.first_baseline = Some(exported_baseline);
    fragment.last_baseline = Some(exported_baseline);
    fragment.baseline_offset = exported_baseline.to_f32();
    fragment
}

// ── Inline box state tracking (CSS Fragmentation §4.4) ──────────────────

/// Tracks an open inline box for line-splitting state.
///
/// When an inline element (e.g. `<span>`) spans multiple lines, its
/// border/padding/margin must be split across fragments:
/// - First fragment: gets inline-start MBP
/// - Last fragment: gets inline-end MBP
/// - Middle fragments: no inline MBP
///
/// With `box-decoration-break: clone`, every fragment gets full MBP.
///
/// Blink: `InlineBoxState` in `inline_box_state.cc`.
#[derive(Debug, Clone)]
pub struct InlineBoxState {
    /// Index into the styles array for this inline element.
    pub style_index: usize,
    /// DOM node of the inline element.
    pub node_id: NodeId,
    /// `box-decoration-break` mode from the element's style.
    pub box_decoration_break: BoxDecorationBreak,
}

#[derive(Debug)]
enum InlineLineChild {
    Fragment(usize),
    InlineBox(usize),
}

#[derive(Debug)]
struct InlineLineBox {
    node_id: NodeId,
    style_index: usize,
    border_start: LayoutUnit,
    border_end: LayoutUnit,
    is_first: bool,
    is_last: bool,
    has_left_edge: bool,
    has_right_edge: bool,
    children: Vec<InlineLineChild>,
}

fn attach_inline_line_child(
    stack: &[usize],
    boxes: &mut [InlineLineBox],
    roots: &mut Vec<InlineLineChild>,
    child: InlineLineChild,
) {
    if let Some(&parent) = stack.last() {
        boxes[parent].children.push(child);
    } else {
        roots.push(child);
    }
}

fn inline_decoration_content_bounds(
    baseline: LayoutUnit,
    metrics: &FontMetrics,
    snap_block_edges: bool,
) -> (LayoutUnit, LayoutUnit) {
    // CSS 2.2 §10.6.1: the content area of a non-replaced inline box is
    // established by that box's font, independently of descendant ink. Blink
    // uses its rounded fixed ascent/descent for these decoration bounds. Glyph
    // ink and nested inline boxes may overflow them, but must not enlarge the
    // box's background or vertical border/padding area.
    let mut content_top = baseline - LayoutUnit::from_f32(metrics.int_ascent());
    let mut content_bottom = baseline + LayoutUnit::from_f32(metrics.int_descent());
    if snap_block_edges {
        content_top = content_top.floor();
        content_bottom = content_bottom.floor();
    }
    (content_top, content_bottom)
}

fn materialize_inline_line_child(
    child: InlineLineChild,
    boxes: &[InlineLineBox],
    flat_fragments: &mut [Option<Fragment>],
    items_data: &InlineItemsData,
    percentage_base: LayoutUnit,
    baseline: LayoutUnit,
    line_height: LayoutUnit,
    parent_metrics: &FontMetrics,
    parent_font_size: f32,
    snap_block_edges: bool,
) -> Fragment {
    match child {
        InlineLineChild::Fragment(index) => flat_fragments[index]
            .take()
            .expect("inline line child materialized once"),
        InlineLineChild::InlineBox(index) => {
            let record = &boxes[index];
            let style = &items_data.styles[record.style_index];
            let font = Font::new(style_to_font_description(style));
            let metrics = font.font_metrics().copied().unwrap_or_default();
            let mut children: Vec<Fragment> = record
                .children
                .iter()
                .map(|child| match child {
                    InlineLineChild::Fragment(index) => flat_fragments[*index]
                        .take()
                        .expect("inline fragment belongs to one box"),
                    InlineLineChild::InlineBox(index) => materialize_inline_line_child(
                        InlineLineChild::InlineBox(*index),
                        boxes,
                        flat_fragments,
                        items_data,
                        percentage_base,
                        baseline,
                        line_height,
                        &metrics,
                        style.font_size,
                        snap_block_edges,
                    ),
                })
                .collect();

            let (content_top, content_bottom) =
                inline_decoration_content_bounds(baseline, &metrics, snap_block_edges);

            let border = BoxStrut::new(
                LayoutUnit::from_i32(style.effective_border_top()),
                if record.has_right_edge {
                    LayoutUnit::from_i32(style.effective_border_right())
                } else {
                    LayoutUnit::zero()
                },
                LayoutUnit::from_i32(style.effective_border_bottom()),
                if record.has_left_edge {
                    LayoutUnit::from_i32(style.effective_border_left())
                } else {
                    LayoutUnit::zero()
                },
            );
            let padding = BoxStrut::new(
                resolve_margin_or_padding(&style.padding_top, percentage_base),
                if record.has_right_edge {
                    resolve_margin_or_padding(&style.padding_right, percentage_base)
                } else {
                    LayoutUnit::zero()
                },
                resolve_margin_or_padding(&style.padding_bottom, percentage_base),
                if record.has_left_edge {
                    resolve_margin_or_padding(&style.padding_left, percentage_base)
                } else {
                    LayoutUnit::zero()
                },
            );
            let margin = BoxStrut::new(
                resolve_margin_or_padding(&style.margin_top, percentage_base),
                if record.has_right_edge {
                    resolve_margin_or_padding(&style.margin_right, percentage_base)
                } else {
                    LayoutUnit::zero()
                },
                resolve_margin_or_padding(&style.margin_bottom, percentage_base),
                if record.has_left_edge {
                    resolve_margin_or_padding(&style.margin_left, percentage_base)
                } else {
                    LayoutUnit::zero()
                },
            );
            let box_top = content_top - padding.top - border.top;
            let box_bottom = content_bottom + padding.bottom + border.bottom;
            let item_lh =
                compute_line_height_metrics(&metrics, &style.line_height, style.font_size);
            let element_line_height =
                used_line_height(&metrics, &style.line_height, style.font_size);
            let keyword_shift = compute_baseline_shift(
                &style.vertical_align,
                parent_font_size,
                parent_metrics.ascent,
                parent_metrics.descent,
                parent_metrics.x_height,
                item_lh.ascent,
                item_lh.descent,
                element_line_height,
            );
            let block_shift = match style.vertical_align {
                VerticalAlign::Top => -box_top.to_f32(),
                VerticalAlign::Bottom => (line_height - box_bottom).to_f32(),
                _ => keyword_shift,
            };
            let shifted_box_top = box_top + LayoutUnit::from_f32(block_shift);
            for child in &mut children {
                child.offset.left = child.offset.left - record.border_start;
                child.offset.top = child.offset.top - box_top;
            }
            let mut fragment = Fragment::new_box(
                record.node_id,
                PhysicalSize::new(
                    (record.border_end - record.border_start).clamp_negative_to_zero(),
                    (box_bottom - box_top).clamp_negative_to_zero(),
                ),
            );
            fragment.offset = PhysicalOffset::new(record.border_start, shifted_box_top);
            fragment.border = border;
            fragment.padding = padding;
            fragment.margin = margin;
            fragment.children = children;
            fragment.is_first_for_node = record.is_first;
            fragment.is_last_for_node = record.is_last;
            fragment.is_inline_box_fragment = true;
            crate::relative::apply_relative_offset(
                &mut fragment,
                style,
                percentage_base,
                LayoutUnit::zero(),
            );
            fragment
        }
    }
}

/// Move an inline-end decoration off a trailing, content-empty continuation.
///
/// A terminal `<br>` can leave the element's close tag on a final empty line.
/// The line box remains (the break still contributes block advance), but the
/// inline-end border/padding belongs to the preceding content-bearing
/// continuation. Painting it on the zero-width tail puts the border at the
/// inline-start edge on the following line.
fn coalesce_trailing_empty_inline_end(line_fragments: &mut [Fragment]) {
    fn take_empty_ends(fragment: &mut Fragment, ends: &mut Vec<Fragment>) {
        let mut retained = Vec::with_capacity(fragment.children.len());
        for mut child in std::mem::take(&mut fragment.children) {
            take_empty_ends(&mut child, ends);
            if child.is_inline_box_fragment
                && !child.is_first_for_node
                && child.is_last_for_node
                && child.children.is_empty()
            {
                ends.push(child);
            } else {
                retained.push(child);
            }
        }
        fragment.children = retained;
    }

    fn last_inline_for_node_mut(fragment: &mut Fragment, node_id: NodeId) -> Option<&mut Fragment> {
        if fragment.is_inline_box_fragment && fragment.node_id == node_id {
            return Some(fragment);
        }
        fragment
            .children
            .iter_mut()
            .rev()
            .find_map(|child| last_inline_for_node_mut(child, node_id))
    }

    if line_fragments.len() < 2 {
        return;
    }
    let (preceding, final_line) = line_fragments.split_at_mut(line_fragments.len() - 1);
    let mut empty_ends = Vec::new();
    take_empty_ends(&mut final_line[0], &mut empty_ends);
    for terminal in empty_ends {
        let Some(previous) = preceding
            .iter_mut()
            .rev()
            .find_map(|line| last_inline_for_node_mut(line, terminal.node_id))
        else {
            continue;
        };
        previous.size.width = previous.size.width + terminal.size.width;
        previous.border.right = terminal.border.right;
        previous.padding.right = terminal.padding.right;
        previous.margin.right = terminal.margin.right;
        previous.is_last_for_node = true;
    }
}

fn line_break_clear(line: &LineInfo, items: &InlineItemsData) -> Clear {
    let mut result = Clear::None;
    for item_result in &line.items {
        if item_result.item_type != InlineItemType::Control {
            continue;
        }
        let item = &items.items[item_result.item_index];
        let clear = items.styles[item.style_index].clear;
        result = match (result, clear) {
            (Clear::None, value) | (value, Clear::None) => value,
            (a, b) if a == b => a,
            _ => Clear::Both,
        };
    }
    result
}

fn line_break_follows_float_at_same_boundary(line: &LineInfo, floats: &[FloatPlaceholder]) -> bool {
    line.items
        .iter()
        .filter(|item| item.item_type == InlineItemType::Control)
        .any(|item| {
            floats
                .iter()
                .any(|float| float.item_index == item.item_index)
        })
}

fn clear_type(clear: Clear) -> ClearType {
    match clear {
        Clear::None => ClearType::None,
        Clear::Left => ClearType::Left,
        Clear::Right => ClearType::Right,
        Clear::Both => ClearType::Both,
    }
}

// ── Line height metrics (CSS 2.2 §10.8.1 half-leading model) ────────────

/// Compute line height metrics using the CSS 2.2 §10.8.1 half-leading model.
///
/// The computed line-height determines total height, and extra space (leading)
/// is distributed equally above and below the font's ascent/descent.
///
/// Blink puts floor on ascent side, ceil on descent side, so the total
/// exactly equals the computed line-height.
fn compute_line_height_metrics(
    metrics: &FontMetrics,
    line_height: &LineHeight,
    font_size: f32,
) -> UsedLineHeightMetrics {
    used_line_height_metrics(metrics, line_height, font_size)
}

fn unite_text_run_metrics<'a>(
    primary: FontMetrics,
    fallback_metrics: impl Iterator<Item = &'a FontMetrics>,
) -> FontMetrics {
    let mut united = primary;
    for metrics in fallback_metrics {
        united.ascent = united.ascent.max(metrics.ascent);
        united.descent = united.descent.max(metrics.descent);
        united.line_gap = united.line_gap.max(metrics.line_gap);
    }
    united.line_spacing = united.ascent + united.descent + united.line_gap;
    united
}

/// Return the font metrics that establish this text portion's normal line box.
///
/// Fallback glyph runs share the inline baseline but may have taller ascent or
/// descent than the primary face. Blink unites those run metrics for
/// `line-height: normal`; explicit line heights keep using the specified
/// primary-font half-leading model.
fn text_line_metrics(
    primary: FontMetrics,
    style: &ComputedStyle,
    item: &super::items::InlineItem,
    item_result: &super::items::InlineItemResult,
    items_data: &InlineItemsData,
) -> FontMetrics {
    if !matches!(style.line_height, LineHeight::Normal) {
        return primary;
    }
    let Some(shape_result) = item_result.shape_result.as_ref() else {
        return primary;
    };
    let item_char_start = byte_to_char_offset(&items_data.text, item.text_range.start);
    let line_char_start =
        byte_to_char_offset(&items_data.text, item_result.text_range.start) - item_char_start;
    let line_char_end =
        byte_to_char_offset(&items_data.text, item_result.text_range.end) - item_char_start;
    unite_text_run_metrics(
        primary,
        shape_result
            .runs
            .iter()
            .filter(|run| {
                run.start_index < line_char_end
                    && run.start_index + run.num_characters > line_char_start
            })
            .map(|run| run.font_data.metrics()),
    )
}

// ── Vertical alignment (CSS 2.2 §10.8) ──────────────────────────────────

fn keyword_baseline_shift(parent_font_size: f32, divisor: i32) -> f32 {
    let fixed_font_size = LayoutUnit::from_f32(parent_font_size);
    (LayoutUnit::from_raw(fixed_font_size.raw() / divisor) + LayoutUnit::from_i32(1)).to_f32()
}

/// Compute baseline shift for vertical-align.
///
/// Returns a float offset where positive = downward shift from parent baseline.
/// Blink: `InlineBoxState::ComputeTextMetrics` and related code in
/// `inline_box_state.cc`.
fn compute_baseline_shift(
    vertical_align: &VerticalAlign,
    parent_font_size: f32,
    parent_ascent: f32,
    parent_descent: f32,
    parent_x_height: f32,
    item_ascent: f32,
    item_descent: f32,
    element_line_height: f32,
) -> f32 {
    match vertical_align {
        VerticalAlign::Baseline => 0.0,
        // Blink's used values include a one-pixel keyword offset in
        // addition to the font-relative component. These keywords are
        // deliberately UA-defined rather than fixed by CSS Values.
        VerticalAlign::Sub => keyword_baseline_shift(parent_font_size, 5),
        VerticalAlign::Super => -keyword_baseline_shift(parent_font_size, 3),
        VerticalAlign::Middle => (item_ascent - item_descent) / 2.0 - parent_x_height / 2.0,
        VerticalAlign::TextTop => item_ascent - parent_ascent,
        VerticalAlign::TextBottom => parent_descent - item_descent,
        VerticalAlign::Length(px) => -px,
        VerticalAlign::Percentage(pct) => {
            // CSS 2.2 §10.8.1: percentage is of the element's own line-height.
            -(element_line_height * pct / 100.0)
        }
        // Top/Bottom need deferred resolution after full line height is known.
        // Return 0.0 here; resolved in a second pass.
        VerticalAlign::Top | VerticalAlign::Bottom => 0.0,
    }
}

// ── Text alignment (CSS 2.2 §16.2) ──────────────────────────────────────

/// Compute the inline-start offset for text-align.
///
/// Blink: `InlineLayoutAlgorithm::ApplyTextAlign`.
///
/// The effective content width is `used_width` (which already has trailing
/// space widths subtracted). For `pre-wrap` the spaces "hang" past the line
/// box — their width is recorded in `hang_width` and *not* included in
/// `used_width`, so the alignment math naturally excludes them.
fn compute_text_align_offset(
    line_info: &LineInfo,
    available_width: LayoutUnit,
    direction: Direction,
    text_align_last: TextAlignLast,
) -> LayoutUnit {
    let remaining = available_width - line_info.used_width;

    // On the last line or forced-break line, always check text-align-last
    // first. Per CSS Text Level 3 §7.3, text-align-last overrides the
    // last line's alignment regardless of text-align's value (unless
    // text-align-last is `auto`).
    let effective_align = if line_info.is_last_line || line_info.has_forced_break {
        match text_align_last {
            TextAlignLast::Auto => match line_info.text_align {
                TextAlign::Justify => TextAlign::Start,
                other => other,
            },
            TextAlignLast::Start => TextAlign::Start,
            TextAlignLast::End => TextAlign::End,
            TextAlignLast::Left => TextAlign::Left,
            TextAlignLast::Right => TextAlign::Right,
            TextAlignLast::Center => TextAlign::Center,
            TextAlignLast::Justify => TextAlign::Justify,
        }
    } else {
        line_info.text_align
    };

    match effective_align {
        TextAlign::Left => LayoutUnit::zero(),
        TextAlign::Right => remaining,
        TextAlign::Center => {
            // CSS Text's default overflow alignment is safe: an overlong
            // centered line falls back to inline-start so its start remains
            // reachable. This is especially visible for a two-glyph RTL
            // scroll-marker label in a one-glyph-wide marker box.
            if remaining < LayoutUnit::zero() {
                if direction == Direction::Rtl {
                    remaining
                } else {
                    LayoutUnit::zero()
                }
            } else {
                LayoutUnit::from_raw(remaining.raw() / 2)
            }
        }
        TextAlign::Justify => {
            // Justification is handled by expanding spaces; offset is 0.
            LayoutUnit::zero()
        }
        TextAlign::Start => {
            if direction == Direction::Rtl {
                remaining
            } else {
                LayoutUnit::zero()
            }
        }
        TextAlign::End => {
            if direction == Direction::Rtl {
                LayoutUnit::zero()
            } else {
                remaining
            }
        }
    }
}

/// Physical offset contributed by a first-line text indent. The line's
/// available inline size has already been shortened by the indent. For RTL
/// that shortening moves the aligned run toward physical left, so adding the
/// indent again would cancel it instead of indenting from inline-start.
fn physical_text_indent_offset(direction: Direction, indent: LayoutUnit) -> LayoutUnit {
    if direction == Direction::Rtl {
        LayoutUnit::zero()
    } else {
        indent
    }
}

/// Count expansion opportunities (spaces between words) for justification.
///
/// Excludes trailing spaces, which are already stripped from width measurement
/// and should not be counted as expansion opportunities.
fn count_expansion_opportunities(line_info: &LineInfo, items_data: &InlineItemsData) -> usize {
    let mut count = 0;
    for item_result in &line_info.items {
        if item_result.item_type == InlineItemType::Text {
            let text = &items_data.text[item_result.text_range.clone()];
            count += text.chars().filter(|c| *c == ' ').count();
        }
    }
    // Exclude trailing spaces: the last text item's trailing spaces were already
    // stripped from width and should not be expansion opportunities.
    // In pre-wrap mode there can be multiple trailing spaces.
    if count > 0 {
        for item_result in line_info.items.iter().rev() {
            if item_result.item_type == InlineItemType::Text {
                let text = &items_data.text[item_result.text_range.clone()];
                let trailing_spaces = text.chars().rev().take_while(|c| *c == ' ').count();
                count = count.saturating_sub(trailing_spaces);
                break;
            }
            if item_result.item_type != InlineItemType::CloseTag
                && item_result.item_type != InlineItemType::OpenTag
            {
                break;
            }
        }
    }
    count
}

/// Count inter-character expansion opportunities for `text-justify: inter-character`.
///
/// Every character boundary (excluding trailing spaces) is an expansion point.
/// Returns the number of gaps between characters (char_count - 1 for non-empty text).
///
/// Only counts boundaries between text items that are logically adjacent
/// (separated only by OpenTag/CloseTag). AtomicInline or Control items
/// break the adjacency, so no boundary gap is counted across them.
fn count_inter_character_opportunities(
    line_info: &LineInfo,
    items_data: &InlineItemsData,
) -> usize {
    // Collect character counts per contiguous text segment, where segments
    // are separated by AtomicInline or Control items.
    let mut segments: Vec<usize> = Vec::new();
    let mut current_segment_chars = 0usize;
    for item_result in &line_info.items {
        match item_result.item_type {
            InlineItemType::Text => {
                let text = &items_data.text[item_result.text_range.clone()];
                current_segment_chars += text.chars().count();
            }
            InlineItemType::OpenTag | InlineItemType::CloseTag => {
                // Tags don't break adjacency — continue accumulating.
            }
            InlineItemType::AtomicInline
            | InlineItemType::Control
            | InlineItemType::BlockInInline => {
                // Non-text items break adjacency.
                if current_segment_chars > 0 {
                    segments.push(current_segment_chars);
                    current_segment_chars = 0;
                }
            }
        }
    }
    if current_segment_chars > 0 {
        segments.push(current_segment_chars);
    }

    // Exclude trailing spaces from the last segment.
    if let Some(last_seg) = segments.last_mut() {
        for item_result in line_info.items.iter().rev() {
            if item_result.item_type == InlineItemType::Text {
                let text = &items_data.text[item_result.text_range.clone()];
                let trailing_spaces = text.chars().rev().take_while(|c| *c == ' ').count();
                *last_seg = last_seg.saturating_sub(trailing_spaces);
                break;
            }
            if item_result.item_type != InlineItemType::CloseTag
                && item_result.item_type != InlineItemType::OpenTag
            {
                break;
            }
        }
    }

    // Total gaps = sum of (segment_chars - 1) for each segment.
    segments.iter().map(|&c| c.saturating_sub(1)).sum()
}

/// Detect whether the current line contains CJK characters.
///
/// Used by `text-justify: auto` to select inter-character justification
/// for CJK text instead of inter-word justification. Checks the actual
/// text content of items on the current line.
///
/// CJK character ranges checked:
/// - CJK Unified Ideographs: U+4E00–U+9FFF
/// - CJK Extension A: U+3400–U+4DBF
/// - Katakana: U+30A0–U+30FF
/// - Hiragana: U+3040–U+309F
/// - Hangul Syllables: U+AC00–U+D7AF
/// - CJK Compatibility Ideographs: U+F900–U+FAFF
fn detect_cjk_content(line_info: &LineInfo, items_data: &InlineItemsData) -> bool {
    for item_result in &line_info.items {
        if item_result.item_type == InlineItemType::Text {
            let text = &items_data.text[item_result.text_range.clone()];
            for ch in text.chars() {
                if is_cjk_character(ch) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a character is in a CJK Unicode range.
#[inline]
fn is_cjk_character(ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Extension A
        | '\u{30A0}'..='\u{30FF}' // Katakana
        | '\u{3040}'..='\u{309F}' // Hiragana
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        | '\u{F900}'..='\u{FAFF}' // CJK Compatibility Ideographs
    )
}

// ── Inline start/end resolution for open/close tag items ─────────────────

// Inline start/end decoration is resolved when a concrete continuation
// fragment is materialized, where first/last slice metadata is available.

// ── Main entry point ─────────────────────────────────────────────────────

/// Vertical RTL line construction uses visual inline coordinates. Keep the
/// legacy physical path for monolithic direct flow, but normalize to logical
/// start-relative coordinates when an owning fragmentation context will map
/// the line through multicol geometry.
fn needs_logical_positioned_inline_geometry(
    doc: &Document,
    node_id: NodeId,
    space: &ConstraintSpace,
) -> bool {
    if space.has_block_fragmentation() {
        return true;
    }
    let mut ancestor = doc.node(node_id).parent;
    while !ancestor.is_none() {
        let ancestor_style = &doc.node(ancestor).style;
        if crate::multicol::ColumnLayoutAlgorithm::from_style(ancestor_style).is_some() {
            let owner_block_size = if ancestor_style.writing_mode.is_horizontal() {
                &ancestor_style.height
            } else {
                &ancestor_style.width
            };
            let node_style = &doc.node(node_id).style;
            let node_block_size = if ancestor_style.writing_mode.is_horizontal() {
                &node_style.height
            } else {
                &node_style.width
            };
            return node_block_size.is_fixed()
                && owner_block_size.is_fixed()
                && node_block_size.value() > owner_block_size.value();
        }
        ancestor = doc.node(ancestor).parent;
    }
    false
}

/// Perform inline layout for a block node that has inline children.
///
/// This is the inline formatting context (IFC) layout algorithm.
/// Returns a Fragment containing line box fragments as children.
///
/// Blink: `InlineLayoutAlgorithm::Layout()` in `inline_layout_algorithm.cc`.
pub fn inline_layout(doc: &Document, node_id: NodeId, space: &ConstraintSpace) -> Fragment {
    let mut items_data = InlineItemsBuilder::collect(doc, node_id);
    let style = &doc.node(node_id).style;
    let base_direction = if style.unicode_bidi == openui_style::UnicodeBidi::Plaintext {
        None
    } else if style.direction == Direction::Rtl {
        Some(openui_text::TextDirection::Rtl)
    } else {
        Some(openui_text::TextDirection::Ltr)
    };
    items_data.apply_bidi(base_direction);
    items_data.split_shaping_runs();
    items_data.shape_text();
    inline_layout_from_items(doc, node_id, space, &items_data, 0, items_data.items.len())
}

/// Perform inline layout using pre-collected items over a specific item range.
///
/// Lays out items in `[item_start..item_end)` from the given `InlineItemsData`.
/// BlockInInline items within the range are skipped (they should have been
/// handled by the caller via block-in-inline splitting).
///
/// Used by block_layout's block-in-inline path (CSS 2.2 §9.2.1.1) to lay
/// out inline segments between block-level interruptions.
pub fn inline_layout_from_items(
    doc: &Document,
    node_id: NodeId,
    space: &ConstraintSpace,
    items_data: &InlineItemsData,
    item_start: usize,
    item_end: usize,
) -> Fragment {
    let style = &doc.node(node_id).style;
    let normalize_positioned_vertical_rtl =
        needs_logical_positioned_inline_geometry(doc, node_id, space);

    let available_inline_size = space.available_inline_size.clamp_negative_to_zero();

    // Build a sub-view of items for the requested range.
    // If processing a subset, create a filtered InlineItemsData with only
    // the items in [item_start..item_end). The line breaker and layout
    // functions work on item indices relative to the data's items array.
    let working_items_data = if item_start == 0 && item_end == items_data.items.len() {
        items_data.clone()
    } else {
        // Create a filtered copy with only the items in the requested range.
        let mut filtered = items_data.clone();
        filtered.items = items_data.items[item_start..item_end].to_vec();
        // Re-index item indices for OOF children within this range.
        filtered.oof_children = items_data
            .oof_children
            .iter()
            .filter(|o| o.item_index >= item_start && o.item_index < item_end)
            .map(|o| super::items_builder::OofPlaceholder {
                node_id: o.node_id,
                item_index: o.item_index - item_start,
                inline_containing_block: o.inline_containing_block,
            })
            .collect();
        filtered.block_in_inline = Vec::new(); // already handled by caller
        filtered
    };
    // Ordinary vertical RTL lines cross the logical-to-physical boundary in
    // their owning block. Keep the established positioned-inline path when
    // out-of-flow placeholders are present: their static positions already
    // share the legacy physical coordinate space.
    let normalize_vertical_rtl = normalize_positioned_vertical_rtl
        || (!space.writing_direction.is_horizontal()
            && space.writing_direction.is_rtl()
            && working_items_data.oof_children.is_empty());

    // Create line breaker from the (possibly filtered) items.
    let inline_float_placeholders = InlineItemsBuilder::collect_with_floats(doc, node_id).1;
    let inline_float_reservations = float_line_break_reservations(
        doc,
        &inline_float_placeholders,
        available_inline_size,
        space,
    );
    let mut consumed_inline_floats = HashSet::new();
    let first_float_text_offset = inline_float_placeholders
        .iter()
        .filter(|float| !inline_float_reservations.contains_key(&float.node_id))
        .map(|float| float.text_offset)
        .min();
    let balanced_width = balanced_wrap_width(
        &working_items_data,
        available_inline_size,
        style,
        space,
        first_float_text_offset,
    );
    let mut line_breaker = LineBreaker::new(&working_items_data, available_inline_size);
    configure_line_breaker(&mut line_breaker, style, space, first_float_text_offset);
    let inherited_first_line_style = space
        .first_line_context
        .as_ref()
        .and_then(|context| context.snapshot());
    let first_line_pseudo = style
        .first_line_style
        .as_deref()
        .or(inherited_first_line_style.as_ref());
    let first_line_items_data = first_line_pseudo.map(|pseudo| {
        let mut data = working_items_data.clone();
        data.apply_first_line_style(style, pseudo);
        data.shape_text();
        data
    });
    let mut first_line_breaker = first_line_items_data.as_ref().map(|data| {
        let mut breaker = LineBreaker::new(data, available_inline_size);
        configure_line_breaker(&mut breaker, style, space, first_float_text_offset);
        breaker
    });

    // Step 3b: Resolve text-indent for the first line.
    let text_indent = crate::length_resolver::resolve_length(
        &style.text_indent,
        available_inline_size,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
    );

    // Get block's font metrics for the strut.
    let block_font_desc = style_to_font_description(style);
    let block_font = Font::new(block_font_desc);
    let block_metrics = block_font.font_metrics().copied().unwrap_or_default();

    // Step 4: Layout each line.
    // Line offsets are relative to the content box (0-based). The caller
    // (block_layout) adds border+padding offsets when positioning.
    //
    // CSS 2.1 §9.5.1: Line boxes flow alongside floats. Each line's
    // available width may differ depending on float exclusion areas at
    // that line's block offset. We query the ExclusionSpace per-line.
    let mut line_fragments: Vec<Fragment> = Vec::new();
    let mut line_item_bounds: Vec<Option<(usize, usize)>> = Vec::new();
    let mut line_static_inline_data: Vec<(LineInfo, LayoutUnit)> = Vec::new();
    let mut block_offset = LayoutUnit::zero();
    let mut is_first_line = true;
    let (clamp_budget, clamp_block_budget, clamp_ellipsis) = effective_line_clamp(style, space);

    // Track which inline boxes (by style_index) are open at the start of
    // each line. Carried forward across lines for inline box decoration
    // splitting (CSS Fragmentation §4.4).
    let mut boxes_open_at_line_start: Vec<InlineBoxState> = Vec::new();

    // Dereference the exclusion space once for the entire line loop.
    let exclusion_ref = space.exclusion_space.as_deref();
    // BFC block offset of this inline content's start within the exclusion space.
    let bfc_block_start = space.bfc_offset.block_offset;

    while clamp_budget != Some(0)
        && if is_first_line {
            first_line_breaker
                .as_ref()
                .map_or(!line_breaker.is_finished(), |breaker| {
                    !breaker.is_finished()
                })
        } else {
            !line_breaker.is_finished()
        }
    {
        // Query float exclusions at this line's block offset.
        // The exclusion space uses content-edge-relative coordinates; add the
        // BFC start offset so we query at the correct absolute position.
        let line_avail = compute_line_availability(
            exclusion_ref,
            bfc_block_start + block_offset,
            available_inline_size,
            LayoutUnit::zero(),
        );
        if line_avail.block_offset > bfc_block_start + block_offset {
            block_offset = line_avail.block_offset - bfc_block_start;
            continue;
        }

        // Apply text-indent: reduce available width on first line only.
        let line_available = if is_first_line && text_indent != LayoutUnit::zero() {
            (line_avail.available_inline_size - text_indent).clamp_negative_to_zero()
        } else {
            line_avail.available_inline_size
        };
        let float_reservation = pending_float_line_reservation(
            &inline_float_placeholders,
            &inline_float_reservations,
            &consumed_inline_floats,
        );
        let wrap_width = (balanced_width
            .map(|balanced| balanced.min_of(line_available))
            .unwrap_or(line_available)
            - float_reservation)
            .clamp_negative_to_zero();

        if let Some(mut line_info) = {
            let use_first_line_breaker = is_first_line && first_line_breaker.is_some();
            let checkpoint = if use_first_line_breaker {
                first_line_breaker.as_ref().unwrap().checkpoint()
            } else {
                line_breaker.checkpoint()
            };
            let mut produced = if use_first_line_breaker {
                first_line_breaker.as_mut().unwrap().next_line(wrap_width)
            } else {
                line_breaker.next_line(wrap_width)
            };
            // CSS 2.1 §9.5.1: if floats shortened this line and its content
            // doesn't fit in the shortened width, shift the line box down to
            // the next float bottom (where more width is available) and
            // re-break. Repeats via the outer loop until content fits or no
            // floats remain at the line's offset.
            if let Some(ref li) = produced {
                let float_follows_text_on_line = inline_float_placeholders.iter().any(|float| {
                    li.items.iter().any(|result| {
                        let item = &working_items_data.items[result.item_index];
                        match result.item_type {
                            InlineItemType::Text => {
                                item.text_range.start < float.text_offset
                                    && float.text_offset <= item.text_range.end
                                    && result.text_range.start < float.text_offset
                            }
                            InlineItemType::AtomicInline => result.item_index < float.item_index,
                            _ => false,
                        }
                    })
                });
                let prevents_rewind = matches!(
                    style.white_space,
                    openui_style::WhiteSpace::Nowrap | openui_style::WhiteSpace::Pre
                ) || float_follows_text_on_line;
                if !prevents_rewind
                    && line_avail.available_inline_size < available_inline_size
                    && li.used_width - li.hang_width > line_available
                {
                    if let Some(shift_to) =
                        next_float_bottom(exclusion_ref, bfc_block_start + block_offset)
                    {
                        if use_first_line_breaker {
                            first_line_breaker.as_mut().unwrap().restore(checkpoint);
                        } else {
                            line_breaker.restore(checkpoint);
                        }
                        block_offset = shift_to - bfc_block_start;
                        produced = None;
                    }
                }
            }
            produced
        } {
            if float_reservation > LayoutUnit::zero() {
                consume_pending_float_line_reservation(
                    &inline_float_placeholders,
                    &inline_float_reservations,
                    &mut consumed_inline_floats,
                );
            }
            for float in &inline_float_placeholders {
                if line_reaches_float_source(&line_info, &working_items_data, float) {
                    consumed_inline_floats.insert(float.node_id);
                }
            }
            let line_items_data = if is_first_line {
                first_line_items_data
                    .as_ref()
                    .unwrap_or(&working_items_data)
            } else {
                &working_items_data
            };
            if is_first_line && first_line_breaker.is_some() {
                line_breaker.seek_after_line(&line_info);
            }
            line_item_bounds.push(line_info.items.iter().map(|item| item.item_index).fold(
                None,
                |bounds, index| {
                    Some(match bounds {
                        Some((first, last)) => (first.min(index), last.max(index)),
                        None => (index, index),
                    })
                },
            ));
            // Step 4b: BiDi reorder items on this line for visual display.
            bidi_reorder_line(&mut line_info.items, line_items_data);

            // Apply text-overflow: ellipsis if configured on the block style.
            if style.text_overflow == openui_style::TextOverflow::Ellipsis
                && style.overflow_x == openui_style::Overflow::Hidden
            {
                apply_text_overflow_ellipsis(
                    &mut line_info,
                    line_available,
                    line_items_data,
                    style,
                );
            }
            let clamp_after_this_line = clamp_budget.is_some_and(|budget| {
                line_fragments.len() + 1 >= budget
                    && (line_breaker.has_remaining_visible_content()
                        || (style.legacy_webkit_line_clamp
                            && line_breaker.has_remaining_forced_break()))
            });
            if clamp_after_this_line {
                apply_line_clamp_marker(
                    &mut line_info,
                    line_available,
                    line_items_data,
                    style,
                    &clamp_ellipsis,
                );
            }

            let line_indent = if is_first_line {
                text_indent
            } else {
                LayoutUnit::zero()
            };
            let break_clear = line_break_clear(&line_info, line_items_data);
            let clearance_target = if break_clear == Clear::None {
                None
            } else {
                exclusion_ref.map(|exclusions| {
                    exclusions.clearance_offset(clear_type(break_clear)) - bfc_block_start
                })
            };
            let clearance_advances = clearance_target.is_some_and(|target| target > block_offset);
            let clearance_at_float_boundary = clearance_target
                .is_some_and(|target| target == block_offset)
                && line_break_follows_float_at_same_boundary(
                    &line_info,
                    &inline_float_placeholders,
                );
            let clearance_only_break = exclusion_ref.is_some_and(ExclusionSpace::has_floats)
                && (clearance_advances || clearance_at_float_boundary);
            let clearance_only_extent = (clearance_only_break && clearance_advances).then(|| {
                let strut = compute_line_height_metrics(
                    &block_metrics,
                    &style.line_height,
                    style.font_size,
                );
                block_offset + LayoutUnit::from_f32(strut.ascent + strut.descent)
            });

            let line_start_boxes = boxes_open_at_line_start.clone();
            let mut line_fragment = create_line_box(
                doc,
                space,
                line_items_data,
                &line_info,
                line_avail.available_inline_size,
                block_offset,
                style,
                &block_metrics,
                space.percentage_resolution_inline_size,
                if is_first_line {
                    text_indent
                } else {
                    LayoutUnit::zero()
                },
                space.percentage_resolution_block_size,
                &line_start_boxes,
                normalize_vertical_rtl,
                clearance_only_break,
            );

            // A line box must avoid every float shelf that intersects its
            // block extent, not only the shelf at its block-start. This is
            // especially visible with tall atomic inlines and consecutive
            // floats: a later float can begin halfway through the line. Use
            // the already-computed line height and occupied inline measure to
            // find a continuous opportunity, then rebuild alignment at that
            // opportunity without changing the selected line content.
            let has_atomic_inline = line_info
                .items
                .iter()
                .any(|item| item.item_type == InlineItemType::AtomicInline);
            let float_follows_atomic_on_line = inline_float_placeholders.iter().any(|float| {
                line_info.items.iter().any(|result| {
                    result.item_type == InlineItemType::AtomicInline
                        && result.item_index < float.item_index
                })
            });
            let full_height_line_avail = if has_atomic_inline && !float_follows_atomic_on_line {
                let occupied_inline_size = (line_info.used_width - line_info.hang_width
                    + line_indent)
                    .clamp_negative_to_zero();
                compute_line_availability_for_block_size(
                    exclusion_ref,
                    bfc_block_start + block_offset,
                    available_inline_size,
                    occupied_inline_size,
                    line_fragment.size.height,
                )
            } else {
                line_avail
            };
            if full_height_line_avail.block_offset > bfc_block_start + block_offset
                || full_height_line_avail.inline_start != line_avail.inline_start
                || full_height_line_avail.available_inline_size != line_avail.available_inline_size
            {
                block_offset = full_height_line_avail.block_offset - bfc_block_start;
                line_fragment = create_line_box(
                    doc,
                    space,
                    line_items_data,
                    &line_info,
                    full_height_line_avail.available_inline_size,
                    block_offset,
                    style,
                    &block_metrics,
                    space.percentage_resolution_inline_size,
                    line_indent,
                    space.percentage_resolution_block_size,
                    &line_start_boxes,
                    normalize_vertical_rtl,
                    clearance_only_break,
                );
            }
            let line_avail = full_height_line_avail;
            let static_inline_origin = line_avail.inline_start
                + compute_text_align_offset(
                    &line_info,
                    line_avail.available_inline_size - line_indent,
                    style.direction,
                    style.text_align_last,
                )
                + physical_text_indent_offset(style.direction, line_indent);
            line_static_inline_data.push((line_info.clone(), static_inline_origin));

            // Update open inline box state for the next line:
            // replay OpenTag/CloseTag items on this line to determine which
            // inline boxes remain open at line end.
            let mut current_open = boxes_open_at_line_start.clone();
            for item_result in &line_info.items {
                let item = &line_items_data.items[item_result.item_index];
                match item_result.item_type {
                    InlineItemType::OpenTag => {
                        let s = &line_items_data.styles[item.style_index];
                        current_open.push(InlineBoxState {
                            style_index: item.style_index,
                            node_id: item.node_id,
                            box_decoration_break: s.box_decoration_break,
                        });
                    }
                    InlineItemType::CloseTag => {
                        current_open.pop();
                    }
                    _ => {}
                }
            }
            boxes_open_at_line_start = current_open;

            // Offset the line box inline-start when floats intrude from the left.
            let mut positioned_line = line_fragment;
            if line_avail.inline_start > LayoutUnit::zero() {
                positioned_line.offset.left = positioned_line.offset.left + line_avail.inline_start;
            }

            let mut next_block_offset = block_offset + positioned_line.size.height;
            if let Some(target) = clearance_target {
                next_block_offset = next_block_offset.max_of(target);
            }
            if let Some(extent) = clearance_only_extent {
                next_block_offset = next_block_offset.max_of(extent);
            }
            let under_ruby_ink_end =
                line_has_under_ruby(doc, &line_info, line_items_data).then(|| {
                    block_offset
                        + fragment_descendant_text_ink_end(&positioned_line)
                            .unwrap_or_else(|| fragment_relative_block_end(&positioned_line))
                });
            let ruby_padding_allowance = resolve_margin_or_padding(
                &style.padding_bottom,
                space.percentage_resolution_inline_size,
            );
            let ruby_ink_exceeds_clamp = clamp_block_budget.is_some_and(|budget| {
                under_ruby_ink_end.is_some_and(|end| {
                    end > budget + ruby_padding_allowance + LayoutUnit::from_i32(1)
                })
            });
            // `line-clamp:auto` is constrained by the used physical block
            // size, not by a line count derived from the root strut. This is
            // observable when descendants, first-line styling, ruby, or
            // atomic inline items make individual line boxes taller.
            let root_line_height = {
                let metrics = compute_line_height_metrics(
                    &block_metrics,
                    &style.line_height,
                    style.font_size,
                );
                LayoutUnit::from_f32(metrics.ascent + metrics.descent)
            };
            let tall_line_reaches_clamp = clamp_block_budget.is_some_and(|budget| {
                next_block_offset >= budget
                    && !line_breaker.is_finished()
                    && positioned_line.size.height > root_line_height
            });
            if clamp_block_budget.is_some_and(|budget| next_block_offset > budget)
                || tall_line_reaches_clamp
                || ruby_ink_exceeds_clamp
            {
                let mut trimmed_line = line_info.clone();
                let mut accepted = None;
                while !ruby_ink_exceeds_clamp
                    && remove_last_content_item_for_tall_clamp(&mut trimmed_line, line_items_data)
                {
                    let mut marked_line = trimmed_line.clone();
                    apply_line_clamp_marker(
                        &mut marked_line,
                        line_available,
                        line_items_data,
                        style,
                        &clamp_ellipsis,
                    );
                    let mut candidate = create_line_box(
                        doc,
                        space,
                        line_items_data,
                        &marked_line,
                        line_avail.available_inline_size,
                        block_offset,
                        style,
                        &block_metrics,
                        space.percentage_resolution_inline_size,
                        if is_first_line {
                            text_indent
                        } else {
                            LayoutUnit::zero()
                        },
                        space.percentage_resolution_block_size,
                        &line_start_boxes,
                        normalize_vertical_rtl,
                        clearance_only_break,
                    );
                    if line_avail.inline_start > LayoutUnit::zero() {
                        candidate.offset.left = candidate.offset.left + line_avail.inline_start;
                    }
                    let candidate_end = block_offset + candidate.size.height;
                    if candidate_end
                        <= clamp_block_budget.unwrap_or(openui_geometry::LayoutUnit::max())
                    {
                        accepted = Some((marked_line, candidate, candidate_end));
                        break;
                    }
                }
                if let Some((marked_line, candidate, candidate_end)) = accepted {
                    if let Some((stored, _)) = line_static_inline_data.last_mut() {
                        *stored = marked_line;
                    }
                    block_offset = candidate_end;
                    line_fragments.push(candidate);
                    break;
                }
                line_item_bounds.pop();
                line_static_inline_data.pop();
                append_clamp_marker_to_last_line(doc, &mut line_fragments, style, &clamp_ellipsis);
                break;
            }
            let physical_clamp_after_this_line = clamp_block_budget.is_some_and(|budget| {
                (next_block_offset >= budget
                    || under_ruby_ink_end.is_some_and(|end| end > next_block_offset))
                    && !line_breaker.is_finished()
            });
            if physical_clamp_after_this_line && !clamp_after_this_line {
                apply_line_clamp_marker(
                    &mut line_info,
                    line_available,
                    line_items_data,
                    style,
                    &clamp_ellipsis,
                );
                if let Some((stored, _)) = line_static_inline_data.last_mut() {
                    *stored = line_info.clone();
                }
                positioned_line = create_line_box(
                    doc,
                    space,
                    line_items_data,
                    &line_info,
                    line_avail.available_inline_size,
                    block_offset,
                    style,
                    &block_metrics,
                    space.percentage_resolution_inline_size,
                    if is_first_line {
                        text_indent
                    } else {
                        LayoutUnit::zero()
                    },
                    space.percentage_resolution_block_size,
                    &line_start_boxes,
                    normalize_vertical_rtl,
                    clearance_only_break,
                );
                if line_avail.inline_start > LayoutUnit::zero() {
                    positioned_line.offset.left =
                        positioned_line.offset.left + line_avail.inline_start;
                }
            }
            if physical_clamp_after_this_line || clamp_after_this_line {
                if let Some(ink_end) = under_ruby_ink_end {
                    let integer_clamp_rounding = (clamp_after_this_line
                        && !physical_clamp_after_this_line
                        && ruby_padding_allowance == LayoutUnit::zero())
                    .then_some(LayoutUnit::from_i32(1))
                    .unwrap_or(LayoutUnit::zero());
                    next_block_offset = next_block_offset.max_of(
                        (ink_end - ruby_padding_allowance).floor() + integer_clamp_rounding,
                    );
                }
            }
            block_offset = next_block_offset;
            line_fragments.push(positioned_line);
            if clamp_after_this_line || physical_clamp_after_this_line {
                if is_first_line {
                    if let Some(context) = &space.first_line_context {
                        context.consume();
                    }
                }
                break;
            }
            if is_first_line {
                if let Some(context) = &space.first_line_context {
                    context.consume();
                }
            }
            is_first_line = false;
        }
    }

    if let Some(context) = &space.line_clamp_context {
        context.consume_layout(line_fragments.len(), block_offset);
        if line_static_inline_data
            .last()
            .is_some_and(|(line, _)| line.has_ellipsis)
        {
            // The IFC already marked the line where its own continuation was
            // discarded. A following OOF-only wrapper is therefore beyond
            // the clamp boundary, rather than a sibling at an exact fit.
            context.suppress_marker();
        }
    }
    let intrinsic_block_size = block_offset;

    if space.writing_direction.is_horizontal() {
        coalesce_trailing_empty_inline_end(&mut line_fragments);
    }

    // Compute first and last baselines from line boxes.
    // CSS Inline 3 §3: The first baseline of a block container with inline
    // content is the baseline of its first line box. The last baseline is
    // the baseline of its last line box.
    // Blink: InlineLayoutAlgorithm::Layout() — first/last_baseline computation.
    let first_baseline = line_fragments
        .first()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    let last_baseline = line_fragments
        .last()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));

    // Build the container fragment.
    let border_box_inline = space.available_inline_size;
    let border_box_size = PhysicalSize::new(border_box_inline, intrinsic_block_size);

    let mut fragment = Fragment::new_box(node_id, border_box_size);
    fragment.children = line_fragments;
    fragment.first_baseline = first_baseline;
    fragment.last_baseline = last_baseline;

    collect_atomic_inline_oof_candidates(doc, node_id, space, border_box_size, &mut fragment);

    append_positioned_inline_candidates(
        doc,
        node_id,
        space,
        &working_items_data,
        &line_item_bounds,
        &line_static_inline_data,
        intrinsic_block_size,
        border_box_size,
        normalize_vertical_rtl,
        &mut fragment,
    );

    // A block-in-inline interruption is laid out by the block caller, but its
    // out-of-flow descendants still belong to this IFC's positioned inline
    // containing block. Preserve those candidates here so multicol's direct
    // IFC path cannot drop them when it fragments the surrounding line.
    for block_info in &items_data.block_in_inline {
        if block_info.item_index < item_start || block_info.item_index >= item_end {
            continue;
        }
        let block_space = crate::block_child_constraint_space(
            space,
            &doc.node(block_info.node_id).style,
            available_inline_size,
            space.available_block_size,
            available_inline_size,
            space.percentage_resolution_block_size,
            false,
        );
        let mut block_fragment = crate::block::block_layout(doc, block_info.node_id, &block_space);
        if block_fragment.oof_candidates.is_empty() {
            continue;
        }
        let prefix_has_content = items_data.items[item_start..block_info.item_index]
            .iter()
            .any(|item| match item.item_type {
                InlineItemType::Text => !item.text_range.is_empty(),
                InlineItemType::AtomicInline | InlineItemType::Control => true,
                _ => false,
            });
        let static_block = if prefix_has_content {
            inline_layout_from_items(
                doc,
                node_id,
                space,
                items_data,
                item_start,
                block_info.item_index,
            )
            .size
            .height
        } else {
            LayoutUnit::zero()
        };
        let inline_cb = block_info.inline_containing_block.and_then(|cb_node_id| {
            inline_containing_block_geometry(
                cb_node_id,
                &fragment.children,
                doc.node(cb_node_id).style.direction,
                space.writing_direction,
                border_box_size.width,
                normalize_vertical_rtl,
                positioned_inline_starts_with_out_of_flow(doc, cb_node_id),
            )
            .map(|(offset, size)| {
                let (offset, size) = normalize_fragmented_inline_containing_block(
                    doc,
                    node_id,
                    space,
                    border_box_size,
                    offset,
                    size,
                );
                (cb_node_id, offset, size)
            })
        });
        for mut candidate in std::mem::take(&mut block_fragment.oof_candidates) {
            candidate.static_position.top = candidate.static_position.top + static_block;
            if let Some((cb_node_id, offset, size)) = inline_cb {
                candidate.containing_block_offset = offset;
                candidate.containing_block_size = size;
                candidate.containing_block_border = openui_geometry::BoxStrut::zero();
                candidate.containing_block_direction = doc.node(cb_node_id).style.direction;
                candidate.containing_block_node = cb_node_id;
                candidate.has_inline_containing_block = true;
                candidate.inline_containing_block_node = Some(cb_node_id);
            }
            fragment.oof_candidates.push(candidate);
        }
    }

    fragment
}

/// Shared logical geometry path for positioned children discovered by either
/// inline layout entry point. Static anchors, first/last-fragment inline
/// containing blocks, and relative translations are resolved together before
/// the owning block or multicol projects them to the physical public API.
#[allow(clippy::too_many_arguments)]
fn append_positioned_inline_candidates(
    doc: &Document,
    node_id: NodeId,
    space: &ConstraintSpace,
    items_data: &InlineItemsData,
    line_item_bounds: &[Option<(usize, usize)>],
    line_static_inline_data: &[(LineInfo, LayoutUnit)],
    intrinsic_block_size: LayoutUnit,
    border_box_size: PhysicalSize,
    normalize_vertical_rtl: bool,
    fragment: &mut Fragment,
) {
    let retained_clamp_boundary = line_static_inline_data
        .last()
        .filter(|(line, _)| line.has_ellipsis)
        .and_then(|_| {
            line_static_inline_data
                .iter()
                .flat_map(|(line, _)| line.items.iter())
                .map(|item| item.item_index + 1)
                .max()
        });
    for oof in &items_data.oof_children {
        if !doc.node(node_id).style.legacy_webkit_line_clamp
            && retained_clamp_boundary.is_some_and(|boundary| oof.item_index > boundary)
        {
            continue;
        }
        let oof_style = doc.node(oof.node_id).style.clone();
        let mut static_block = find_static_block_for_item_index(
            oof.item_index,
            &fragment.children,
            line_item_bounds,
            intrinsic_block_size,
            oof_style.display.is_block_level()
                && items_data.items[..oof.item_index]
                    .iter()
                    .any(|item| match item.item_type {
                        InlineItemType::Text => !items_data.text[item.text_range.clone()]
                            .trim_matches(char::is_whitespace)
                            .is_empty(),
                        InlineItemType::AtomicInline => true,
                        _ => false,
                    }),
        );
        let inline_cb = oof.inline_containing_block.and_then(|cb_node_id| {
            inline_containing_block_geometry(
                cb_node_id,
                &fragment.children,
                doc.node(cb_node_id).style.direction,
                space.writing_direction,
                border_box_size.width,
                normalize_vertical_rtl,
                positioned_inline_starts_with_out_of_flow(doc, cb_node_id),
            )
            .map(|(offset, size)| {
                let (offset, size) = normalize_fragmented_inline_containing_block(
                    doc,
                    node_id,
                    space,
                    border_box_size,
                    offset,
                    size,
                );
                (cb_node_id, offset, size)
            })
        });
        let unresolved_transform_inline_cb = (inline_cb.is_none())
            .then_some(oof.inline_containing_block)
            .flatten()
            .filter(|target| {
                doc.node(*target)
                    .style
                    .establishes_transform_containing_block
            });
        let retained_inline_cb = inline_cb
            .map(|(target, _, _)| target)
            .or(unresolved_transform_inline_cb);
        let (containing_block_offset, containing_block_size, containing_block_direction) =
            inline_cb.map_or(
                (
                    PhysicalOffset::zero(),
                    border_box_size,
                    retained_inline_cb.map_or(doc.node(node_id).style.direction, |target| {
                        doc.node(target).style.direction
                    }),
                ),
                |(cb_node_id, offset, size)| (offset, size, doc.node(cb_node_id).style.direction),
            );
        let mut static_inline = find_static_inline_for_item_index(
            oof.item_index,
            line_static_inline_data,
            doc.node(node_id).style.direction,
            oof_style.display.is_block_level(),
            border_box_size.width,
        );
        static_inline = logical_inline_axis_offset(
            static_inline,
            LayoutUnit::zero(),
            border_box_size.width,
            space.writing_direction,
            normalize_vertical_rtl,
        );
        if !space.writing_direction.is_horizontal() {
            if let Some(cb_node_id) = oof.inline_containing_block {
                // Horizontal inline fragments already carry their relative
                // visual translation. Vertical IFC geometry is normalized
                // back to logical coordinates before the block projection,
                // so reconstruct that translation once on the retained
                // logical anchor here.
                let relative = crate::relative::compute_relative_offset(
                    &doc.node(cb_node_id).style,
                    space.percentage_resolution_inline_size,
                    LayoutUnit::zero(),
                );
                static_inline += relative.left;
                static_block += relative.top;
            }
        }
        fragment.oof_candidates.push(OutOfFlowCandidate {
            node_id: oof.node_id,
            style: oof_style,
            static_position: PhysicalOffset::new(static_inline, static_block),
            static_position_horizontal_edge: crate::out_of_flow::StaticPositionEdge::Start,
            static_position_vertical_edge: crate::out_of_flow::StaticPositionEdge::Start,
            containing_block_offset,
            containing_block_node: retained_inline_cb.unwrap_or(NodeId::NONE),
            containing_block_size,
            containing_block_border: openui_geometry::BoxStrut::zero(),
            containing_block_direction,
            static_position_direction: doc.node(node_id).style.direction,
            has_inline_containing_block: retained_inline_cb.is_some(),
            inline_containing_block_node: oof.inline_containing_block,
        });
    }
}

/// Resolve the containing block formed by a positioned inline's first and
/// last line fragments (CSS 2.1 §10.1).
fn positioned_inline_starts_with_out_of_flow(doc: &Document, target: NodeId) -> bool {
    for child_id in doc.children(target) {
        let child = doc.node(child_id);
        if child.style.display == Display::None {
            continue;
        }
        if child.style.position.is_absolutely_positioned() {
            return true;
        }
        if child.tag == ElementTag::Text
            && child
                .text
                .as_deref()
                .is_some_and(|text| text.trim_matches(char::is_whitespace).is_empty())
            && matches!(
                child.style.white_space,
                WhiteSpace::Normal | WhiteSpace::Nowrap
            )
        {
            continue;
        }
        return false;
    }
    false
}

fn inline_containing_block_geometry(
    target: NodeId,
    roots: &[Fragment],
    direction: Direction,
    writing_direction: openui_geometry::WritingDirectionMode,
    container_inline_size: LayoutUnit,
    normalize_vertical_rtl: bool,
    preserve_empty_first: bool,
) -> Option<(PhysicalOffset, PhysicalSize)> {
    fn collect(
        target: NodeId,
        fragment: &Fragment,
        parent_offset: PhysicalOffset,
        fragments: &mut Vec<(PhysicalOffset, PhysicalSize, bool)>,
    ) {
        fn has_nonempty_in_flow_content(fragment: &Fragment) -> bool {
            fragment.children.iter().any(|child| {
                if child.positioned_fragmentation.is_some() {
                    return false;
                }
                if child.kind == FragmentKind::Text {
                    return child.size.width > LayoutUnit::zero()
                        || child.size.height > LayoutUnit::zero();
                }
                if child.node_id.is_none() || child.is_inline_box_fragment {
                    return has_nonempty_in_flow_content(child);
                }
                child.size.width > LayoutUnit::zero() || child.size.height > LayoutUnit::zero()
            })
        }

        let offset = PhysicalOffset::new(
            parent_offset.left + fragment.offset.left,
            parent_offset.top + fragment.offset.top,
        );
        if fragment.node_id == target && fragment.is_inline_box_fragment {
            fragments.push((
                offset,
                fragment.size,
                has_nonempty_in_flow_content(fragment),
            ));
        }
        for child in &fragment.children {
            collect(target, child, offset, fragments);
        }
    }

    let mut fragments = Vec::new();
    for root in roots {
        collect(target, root, PhysicalOffset::zero(), &mut fragments);
    }
    // CSS 2.1 forms a positioned inline's containing block from the padding
    // edges of its first and last generated boxes.  An empty first box at a
    // normal line boundary is still the authoritative start edge. During
    // block fragmentation, however, the IFC also emits empty continuation
    // shells at fragmentainer boundaries; those do not replace the first or
    // last in-flow box used by positioned descendants.
    let nonempty: Vec<_> = fragments
        .iter()
        .filter(|(_, size, has_content)| size.width > LayoutUnit::zero() || *has_content)
        .collect();
    let endpoints = if preserve_empty_first || nonempty.is_empty() {
        fragments.first().zip(fragments.last())
    } else {
        nonempty.first().copied().zip(nonempty.last().copied())
    };
    endpoints.map(
        |((first_offset, first_size, _), (last_offset, last_size, _))| {
            let (left, right) = if direction == Direction::Rtl {
                (last_offset.left, first_offset.left + first_size.width)
            } else {
                (first_offset.left, last_offset.left + last_size.width)
            };
            let (top, bottom) = if writing_direction.is_horizontal() {
                // Preserve the established horizontal continuation geometry;
                // its fragment list already carries CSS 2.1 first/last-box
                // affinity used by horizontal multicol fragmentation.
                (first_offset.top, last_offset.top + last_size.height)
            } else {
                // Vertical line fragments are still collected before their
                // owning block/multicol projection. Retain their logical line
                // endpoints here so that block-axis column affinity survives
                // the later transpose.
                let (block_first_offset, _, _) = fragments.first().unwrap();
                let (block_last_offset, block_last_size, _) = fragments.last().unwrap();
                (
                    block_first_offset.top,
                    block_last_offset.top + block_last_size.height,
                )
            };
            let size = PhysicalSize::new(
                (right - left).clamp_negative_to_zero(),
                (bottom - top).clamp_negative_to_zero(),
            );
            (
                PhysicalOffset::new(
                    logical_inline_axis_offset(
                        left,
                        size.width,
                        container_inline_size,
                        writing_direction,
                        normalize_vertical_rtl,
                    ),
                    top,
                ),
                size,
            )
        },
    )
}

/// Inline layout places runs in visual start/end order so text alignment and
/// bidi stay local to the line. Positioned geometry crosses a different
/// boundary: it must be expressed as a start-relative logical coordinate
/// before the owning block projects it to physical fragment storage.
fn logical_inline_axis_offset(
    visual_offset: LayoutUnit,
    inline_extent: LayoutUnit,
    container_inline_size: LayoutUnit,
    writing_direction: openui_geometry::WritingDirectionMode,
    normalize_vertical_rtl: bool,
) -> LayoutUnit {
    // Line construction stores visual inline coordinates in both horizontal
    // and vertical modes.  The owning block consumes start-relative logical
    // coordinates, so RTL must reverse the cursor before either a horizontal
    // or transposed physical boundary consumes it.
    if writing_direction.is_rtl() && (writing_direction.is_horizontal() || normalize_vertical_rtl) {
        container_inline_size - visual_offset - inline_extent
    } else {
        visual_offset
    }
}

/// Convert the visual inline coordinates produced while constructing a line
/// into the logical start-relative coordinates consumed by vertical block
/// layout. Horizontal IFC fragments remain in their established physical
/// storage path; their positioned geometry is normalized separately above.
fn normalize_vertical_line_inline_axis(
    fragment: &mut Fragment,
    writing_direction: openui_geometry::WritingDirectionMode,
) {
    if writing_direction.is_horizontal() || !writing_direction.is_rtl() {
        return;
    }

    fn normalize_children(fragment: &mut Fragment) {
        let inline_size = fragment.size.width;
        for child in &mut fragment.children {
            child.offset.left = inline_size - child.offset.left - child.size.width;
            if child.node_id.is_none() || child.is_inline_box_fragment {
                normalize_children(child);
            }
        }
    }

    normalize_children(fragment);
    for candidate in &mut fragment.oof_candidates {
        candidate.static_position.left = fragment.size.width - candidate.static_position.left;
        if candidate.has_inline_containing_block {
            candidate.containing_block_offset.left = fragment.size.width
                - candidate.containing_block_offset.left
                - candidate.containing_block_size.width;
        }
    }
}

fn normalize_fragmented_inline_containing_block(
    doc: &Document,
    block_node_id: NodeId,
    space: &ConstraintSpace,
    block_size: PhysicalSize,
    offset: PhysicalOffset,
    mut size: PhysicalSize,
) -> (PhysicalOffset, PhysicalSize) {
    let block_style = &doc.node(block_node_id).style;
    if size.width > block_size.width && block_style.height.is_fixed() {
        // The inline breaker may retain a single logical inline strip before
        // its containing block is fragmented. Its positioned descendants use
        // the definite surrounding block as that strip's fragmentation
        // extent, rather than the unfragmented horizontal ink width.
        size.width = block_size.width;
        let definite_block_size = resolve_length(
            &block_style.height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        size.height = (definite_block_size - offset.top).clamp_negative_to_zero();
    }
    (offset, size)
}

/// Bubble unresolved positioned descendants from atomic inlines to the IFC.
/// Their nearest positioned inline ancestor cannot be resolved while a single
/// line is being built because CSS 2.1 defines that containing block from the
/// ancestor's first and last line fragments.
fn collect_atomic_inline_oof_candidates(
    doc: &Document,
    block_node_id: NodeId,
    space: &ConstraintSpace,
    block_size: PhysicalSize,
    fragment: &mut Fragment,
) {
    let mut candidates = Vec::new();
    for line in &mut fragment.children {
        for mut candidate in std::mem::take(&mut line.oof_candidates) {
            candidate.static_position.left = candidate.static_position.left + line.offset.left;
            candidate.static_position.top = candidate.static_position.top + line.offset.top;
            if candidate.has_inline_containing_block {
                candidate.containing_block_offset.left =
                    candidate.containing_block_offset.left + line.offset.left;
                candidate.containing_block_offset.top =
                    candidate.containing_block_offset.top + line.offset.top;
            }
            candidates.push(candidate);
        }
    }

    for mut candidate in candidates {
        if let Some(cb_node_id) = candidate.inline_containing_block_node {
            if let Some((offset, size)) = inline_containing_block_geometry(
                cb_node_id,
                &fragment.children,
                doc.node(cb_node_id).style.direction,
                space.writing_direction,
                block_size.width,
                needs_logical_positioned_inline_geometry(doc, block_node_id, space),
                positioned_inline_starts_with_out_of_flow(doc, cb_node_id),
            ) {
                let (offset, size) = normalize_fragmented_inline_containing_block(
                    doc,
                    block_node_id,
                    space,
                    block_size,
                    offset,
                    size,
                );
                candidate.containing_block_offset = offset;
                candidate.containing_block_size = size;
                candidate.containing_block_border = BoxStrut::zero();
                candidate.containing_block_direction = doc.node(cb_node_id).style.direction;
                candidate.containing_block_node = cb_node_id;
                candidate.has_inline_containing_block = true;
            }
        }
        fragment.oof_candidates.push(candidate);
    }
}

/// Find the static block position for an OOF placeholder at a given item index.
///
/// The placeholder's item index is the insertion boundary immediately before
/// the next in-flow item.  Use the last line containing an earlier item.  This
/// remains exact when one text item wraps onto several lines; proportional
/// item/line mapping cannot distinguish those continuations.
fn find_static_block_for_item_index(
    item_index: usize,
    line_fragments: &[Fragment],
    line_item_bounds: &[Option<(usize, usize)>],
    intrinsic_block_size: LayoutUnit,
    block_level_after_inline_content: bool,
) -> LayoutUnit {
    if line_fragments.is_empty() {
        return intrinsic_block_size;
    }
    if line_fragments.len() == 1 {
        return line_fragments[0].offset.top;
    }
    let mut preceding_line = None;
    for (line_index, bounds) in line_item_bounds.iter().enumerate() {
        if let Some((first, _)) = bounds {
            if *first < item_index {
                preceding_line = Some(line_index);
            }
        }
    }
    let line = &line_fragments[preceding_line.unwrap_or(0).min(line_fragments.len() - 1)];
    line.offset.top
        + if block_level_after_inline_content && preceding_line.is_some() {
            line.size.height
        } else {
            LayoutUnit::zero()
        }
}

/// Return the inline-axis static position at an out-of-flow insertion
/// boundary.  The placeholder does not create an inline item of its own, so
/// reconstruct its zero-width advance from the line items that precede it.
fn find_static_inline_for_item_index(
    item_index: usize,
    lines: &[(LineInfo, LayoutUnit)],
    direction: Direction,
    block_level_hypothetical_box: bool,
    formatting_context_inline_size: LayoutUnit,
) -> LayoutUnit {
    let line_index = lines
        .iter()
        .enumerate()
        .filter(|(_, (line, _))| line.items.iter().any(|item| item.item_index < item_index))
        .map(|(index, _)| index)
        .next_back()
        .unwrap_or(0)
        .min(lines.len().saturating_sub(1));
    let Some((line, origin)) = lines.get(line_index) else {
        return LayoutUnit::zero();
    };
    let preceding = if block_level_hypothetical_box {
        // CSS 2.1 §10.3.7: static position is based on the hypothetical
        // position of the element's first box.  A block-level abspos child
        // interrupts the inline run and its hypothetical block begins at the
        // inline edge; it does not begin at the preceding text cursor.
        LayoutUnit::zero()
    } else {
        line.items
            .iter()
            .filter(|item| item.item_index < item_index)
            .fold(LayoutUnit::zero(), |sum, item| sum + item.inline_size)
    };
    if block_level_hypothetical_box {
        // Text alignment moves the line's glyph run, not the hypothetical
        // block box. Its static inline edge remains the formatting context's
        // start edge.
        if direction == Direction::Rtl {
            formatting_context_inline_size
        } else {
            LayoutUnit::zero()
        }
    } else if direction == Direction::Rtl {
        *origin + (line.used_width - preceding).clamp_negative_to_zero()
    } else {
        *origin + preceding
    }
}
/// Apply inline fragmentation to a laid-out inline formatting context.
///
/// Takes a fragment produced by `inline_layout` / `inline_layout_from_items`
/// and splits it at fragmentainer boundaries. Returns the fragment for the
/// current fragmentainer, with a break token if content continues.
///
/// CSS Break 3 §3: Line boxes are class-B break points — a break can occur
/// between any two line boxes. Orphans/widows constraints (§4.1) may shift
/// the break point to satisfy minimum line counts.
///
/// Blink: `InlineLayoutAlgorithm::BreakLine()` and `BreakBeforeLine()` in
/// `inline_layout_algorithm.cc`.
///
/// # Parameters
/// - `fragment`: The fully laid-out inline fragment (all lines).
/// - `fragmentainer_block_size`: Height of the current fragmentainer.
/// - `block_offset_in_fragmentainer`: How far into the fragmentainer this
///   inline content starts.
/// - `lines_already_consumed`: Number of lines consumed in previous
///   fragmentainers (from an `InlineBreakToken`).
/// - `orphans`: CSS `orphans` property value (minimum lines before break).
/// - `widows`: CSS `widows` property value (minimum lines after break).
pub fn apply_inline_fragmentation(
    mut fragment: Fragment,
    fragmentainer_block_size: LayoutUnit,
    block_offset_in_fragmentainer: LayoutUnit,
    lines_already_consumed: usize,
    orphans: u32,
    widows: u32,
) -> Fragment {
    use crate::fragmentation::{BreakToken, InlineBreakToken};

    let has_block_end_decoration = fragment
        .children
        .last()
        .is_some_and(|child| child.is_block_end_decoration_marker);
    let total_lines = fragment.children.len() - usize::from(has_block_end_decoration);

    // No line boxes means this is a trailing-decoration continuation. It is
    // an authoritative fragment in its own right even though it paints no
    // child; retain the remaining decoration extent computed by resumption.
    if total_lines == 0 {
        if has_block_end_decoration {
            let fills_fragmentainer = fragment
                .children
                .last()
                .is_some_and(|marker| marker.fills_fragmentainer_block_end_decoration);
            fragment.children.clear();
            if fills_fragmentainer {
                fragment.size.height = fragmentainer_block_size;
            }
        }
        return fragment;
    }

    // No fragmentation context — return as-is.
    if fragmentainer_block_size <= LayoutUnit::zero() {
        return fragment;
    }

    // Available block space in this fragmentainer for inline content.
    let available_block = fragmentainer_block_size - block_offset_in_fragmentainer;
    if available_block <= LayoutUnit::zero() {
        // No space at all — produce empty fragment with break token at line 0.
        fragment.children.clear();
        fragment.size.height = LayoutUnit::zero();
        fragment.first_baseline = None;
        fragment.last_baseline = None;
        fragment.break_token = Some(BreakToken::Inline(InlineBreakToken::new(
            lines_already_consumed,
            LayoutUnit::zero(),
        )));
        return fragment;
    }

    // Determine how many lines fit in the available block space.
    // Walk line fragments (children) accumulating their block sizes.
    let mut lines_that_fit = 0usize;
    for child in fragment.children.iter().take(total_lines) {
        let line_bottom = child.offset.top + child.size.height;
        if line_bottom > available_block {
            break;
        }
        lines_that_fit += 1;
    }

    let decoration_fits = !has_block_end_decoration
        || fragment
            .children
            .last()
            .is_some_and(|marker| marker.offset.top + marker.size.height <= available_block);

    // All lines and the trailing block-end decoration fit — no break needed.
    if lines_that_fit >= total_lines && decoration_fits {
        return fragment;
    }

    // All line boxes fit exactly, but block-end decoration does not. Break at
    // the class-B point after the final line and leave the decoration marker
    // for the continuation token. Widows/orphans constrain line-to-line
    // breaks, not this trailing decoration boundary.
    if lines_that_fit >= total_lines {
        // The fragmentainer also consumes the leading portion of the
        // decoration marker. Resume from its physical break edge rather than
        // from the last line edge, otherwise the full marker is painted again
        // in the continuation.
        let consumed_block_size = available_block;
        fragment.children.truncate(total_lines);
        fragment.size.height = available_block;
        fragment.break_token = Some(BreakToken::Inline(InlineBreakToken::new(
            lines_already_consumed + total_lines,
            consumed_block_size,
        )));
        return fragment;
    }

    // A line can start beyond the current fragmentainer because a float
    // exclusion consumed all available inline space above it.  That is an
    // empty continuation, not an oversized line that should be forced into
    // this fragmentainer.  Preserve the consumed block coordinate in the
    // break token so the same exclusion resumes at the correct offset in the
    // next column.  If the line starts within this fragmentainer but is
    // monolithic and fits in a fresh one, take the class-B break immediately
    // before it instead.
    if lines_that_fit == 0 {
        let first_line = &fragment.children[0];
        if first_line.offset.top > LayoutUnit::zero()
            && (first_line.offset.top >= available_block
                || first_line.size.height <= fragmentainer_block_size)
        {
            let consumed_block_size = if first_line.offset.top >= available_block {
                available_block
            } else {
                first_line.offset.top
            };
            fragment.children.clear();
            fragment.size.height = LayoutUnit::zero();
            fragment.first_baseline = None;
            fragment.last_baseline = None;
            fragment.break_token = Some(BreakToken::Inline(InlineBreakToken::new(
                lines_already_consumed,
                consumed_block_size,
            )));
            return fragment;
        }
    }

    // Apply orphans and widows constraints.
    // CSS Break 3 §4.1:
    // - "orphans" = minimum lines that must remain before the break.
    // - "widows" = minimum lines that must remain after the break.
    let lines_remaining = total_lines - lines_that_fit;
    let orphans = orphans as usize;
    let widows = widows as usize;

    // If orphans constraint is violated (too few lines before break),
    // reduce lines_that_fit to satisfy it... but we can't go below 0.
    // Actually orphans says we need AT LEAST `orphans` lines before break.
    // If lines_that_fit < orphans, that's okay if that's all we have space for;
    // orphans is a soft constraint that may be violated if there's no room.
    // But if we have room for more than orphans, we shouldn't reduce.
    // The key scenario: if we have room for many lines but widows would be
    // violated, we reduce lines_that_fit so the next fragmentainer gets enough.

    // Widows check: the next fragmentainer needs at least `widows` lines.
    // If lines_remaining < widows, steal lines from this fragmentainer.
    let mut adjusted_fit = lines_that_fit;
    if lines_remaining < widows && adjusted_fit > 0 {
        let needed = widows - lines_remaining;
        if adjusted_fit > needed {
            adjusted_fit -= needed;
        } else {
            // Can't satisfy widows without violating orphans to 0.
            // Keep at least 1 line (or orphans, whichever is smaller).
            adjusted_fit = adjusted_fit.min(1);
        }
    }

    // Orphans check: this fragmentainer needs at least `orphans` lines.
    // If adjusted_fit < orphans and there are enough total lines, try to
    // fit at least `orphans` lines (if they physically fit).
    if adjusted_fit < orphans && lines_that_fit >= orphans {
        adjusted_fit = orphans;
    }

    // Ensure we don't exceed what physically fits.
    adjusted_fit = adjusted_fit.min(lines_that_fit);

    // If adjusted_fit is 0 and there are lines, keep at least 1 line
    // (last resort — a line box is unbreakable content).
    if adjusted_fit == 0 && total_lines > 0 {
        adjusted_fit = 1;
    }

    // If all lines now fit after adjustment, no break needed.
    if adjusted_fit >= total_lines {
        return fragment;
    }

    // Split: keep first `adjusted_fit` lines, produce break token for rest.
    let consumed_block_size = if adjusted_fit > 0 {
        let last_kept = &fragment.children[adjusted_fit - 1];
        last_kept.offset.top + last_kept.size.height
    } else {
        LayoutUnit::zero()
    };

    fragment.children.truncate(adjusted_fit);
    fragment.size.height = consumed_block_size;

    // Update baselines for the truncated set.
    fragment.first_baseline = fragment
        .children
        .first()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    fragment.last_baseline = fragment
        .children
        .last()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));

    // Produce break token so the next fragmentainer can resume.
    let total_consumed = lines_already_consumed + adjusted_fit;
    fragment.break_token = Some(BreakToken::Inline(InlineBreakToken::new(
        total_consumed,
        consumed_block_size,
    )));

    fragment
}

/// Resume inline layout from a break token, producing a fragment for the
/// next fragmentainer.
///
/// Takes the full set of line fragments (from a complete inline layout),
/// skips lines already consumed, re-offsets the remaining lines to start
/// at block offset 0, and optionally applies fragmentation again if the
/// remaining lines still don't fit.
///
/// # Parameters
/// - `full_fragment`: The complete inline fragment with ALL lines.
/// - `break_token`: The inline break token from the previous fragmentainer.
/// - `fragmentainer_block_size`: Height of the new fragmentainer.
/// - `orphans`: CSS `orphans` value.
/// - `widows`: CSS `widows` value.
pub fn resume_inline_from_break_token(
    full_fragment: Fragment,
    break_token: &crate::fragmentation::InlineBreakToken,
    fragmentainer_block_size: LayoutUnit,
    orphans: u32,
    widows: u32,
) -> Fragment {
    let lines_to_skip = break_token.lines_consumed;
    let has_block_end_decoration = full_fragment
        .children
        .last()
        .is_some_and(|child| child.is_block_end_decoration_marker);
    let total_lines = full_fragment.children.len() - usize::from(has_block_end_decoration);

    if lines_to_skip >= total_lines && !has_block_end_decoration {
        // All lines consumed — return empty fragment.
        let mut empty = Fragment::new_box(
            full_fragment.node_id,
            PhysicalSize::new(full_fragment.size.width, LayoutUnit::zero()),
        );
        empty.first_baseline = None;
        empty.last_baseline = None;
        return empty;
    }

    // Take the remaining lines and re-offset them to start from 0.
    let mut resumed_fragment = full_fragment;
    let remaining_children: Vec<Fragment> = resumed_fragment
        .children
        .drain(..)
        .skip(lines_to_skip)
        .collect();

    let consumed_block_size = break_token.consumed_block_size;

    let adjusted_children: Vec<Fragment> = remaining_children
        .into_iter()
        .map(|mut f| {
            f.offset.top = f.offset.top - consumed_block_size;
            f
        })
        .collect();

    // Compute the total block size of remaining lines.
    let total_block = if let Some(last) = adjusted_children.last() {
        last.offset.top + last.size.height
    } else {
        LayoutUnit::zero()
    };

    resumed_fragment.size.height = total_block;
    resumed_fragment.children = adjusted_children;
    resumed_fragment.break_token = None;

    // Update baselines.
    resumed_fragment.first_baseline = resumed_fragment
        .children
        .first()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    resumed_fragment.last_baseline = resumed_fragment
        .children
        .last()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));

    // Apply fragmentation again if this fragmentainer also can't hold
    // all remaining lines.
    let mut fragment = apply_inline_fragmentation(
        resumed_fragment,
        fragmentainer_block_size,
        LayoutUnit::zero(),
        lines_to_skip,
        orphans,
        widows,
    );
    if let Some(crate::fragmentation::BreakToken::Inline(token)) = &mut fragment.break_token {
        token.consumed_block_size = token.consumed_block_size + consumed_block_size;
    }
    fragment
}

///
/// Used by block_layout for CSS 2.2 §9.2.1.1 anonymous block boxes when
/// mixed inline+block content is present. Lays out only the given children
/// as an inline formatting context.
pub fn inline_layout_for_children(
    doc: &Document,
    node_id: NodeId,
    children: &[NodeId],
    space: &ConstraintSpace,
) -> Fragment {
    let style = &doc.node(node_id).style;
    let normalize_positioned_vertical_rtl =
        needs_logical_positioned_inline_geometry(doc, node_id, space);

    let available_inline_size = space.available_inline_size.clamp_negative_to_zero();

    // Collect inline items only from the specified children.
    let (mut items_data, inline_float_placeholders) =
        InlineItemsBuilder::collect_for_children_with_floats(doc, node_id, children);
    let normalize_vertical_rtl = normalize_positioned_vertical_rtl
        || (!space.writing_direction.is_horizontal()
            && space.writing_direction.is_rtl()
            && items_data.oof_children.is_empty());

    let base_direction = if style.unicode_bidi == openui_style::UnicodeBidi::Plaintext {
        None
    } else if style.direction == Direction::Rtl {
        Some(openui_text::TextDirection::Rtl)
    } else {
        Some(openui_text::TextDirection::Ltr)
    };
    items_data.apply_bidi(base_direction);
    items_data.split_shaping_runs();
    items_data.shape_text();

    let mut line_breaker = LineBreaker::new(&items_data, available_inline_size);
    let mut saw_float = false;
    let float_precedes_text = children
        .iter()
        .any(|child_id| node_follows_float_with_in_flow_text(doc, *child_id, &mut saw_float));
    line_breaker.set_float_precedes_in_flow_text(float_precedes_text);
    line_breaker.set_writing_direction(space.writing_direction);
    line_breaker.set_text_align(style.text_align);
    line_breaker.set_container_white_space(style.white_space);

    let text_indent = crate::length_resolver::resolve_length(
        &style.text_indent,
        available_inline_size,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
    );

    let block_font_desc = style_to_font_description(style);
    let block_font = Font::new(block_font_desc);
    let block_metrics = block_font.font_metrics().copied().unwrap_or_default();

    let mut line_fragments: Vec<Fragment> = Vec::new();
    let mut line_item_bounds: Vec<Option<(usize, usize)>> = Vec::new();
    let mut line_static_inline_data: Vec<(LineInfo, LayoutUnit)> = Vec::new();
    let mut block_offset = LayoutUnit::zero();
    let mut is_first_line = true;
    let (clamp_budget, clamp_block_budget, clamp_ellipsis) = effective_line_clamp(style, space);

    // Track which inline boxes are open at the start of each line.
    let mut boxes_open_at_line_start: Vec<InlineBoxState> = Vec::new();

    // Dereference the exclusion space once for the entire line loop.
    let exclusion_ref = space.exclusion_space.as_deref();
    // BFC block offset of this anonymous wrapper's start within the exclusion space.
    let bfc_block_start = space.bfc_offset.block_offset;

    while clamp_budget != Some(0) && !line_breaker.is_finished() {
        // Query float exclusions at this line's block offset.
        let line_avail = compute_line_availability(
            exclusion_ref,
            bfc_block_start + block_offset,
            available_inline_size,
            LayoutUnit::zero(),
        );
        if line_avail.block_offset > bfc_block_start + block_offset {
            block_offset = line_avail.block_offset - bfc_block_start;
            continue;
        }

        let line_available = if is_first_line && text_indent != LayoutUnit::zero() {
            (line_avail.available_inline_size - text_indent).clamp_negative_to_zero()
        } else {
            line_avail.available_inline_size
        };

        if let Some(mut line_info) = {
            let checkpoint = line_breaker.checkpoint();
            let mut produced = line_breaker.next_line(line_available);
            // CSS 2.1 §9.5.1: shift unfittable float-shortened lines down
            // (see the identical logic in inline_layout above).
            if let Some(ref li) = produced {
                if line_avail.available_inline_size < available_inline_size
                    && li.used_width - li.hang_width > line_available
                {
                    if let Some(shift_to) =
                        next_float_bottom(exclusion_ref, bfc_block_start + block_offset)
                    {
                        line_breaker.restore(checkpoint);
                        block_offset = shift_to - bfc_block_start;
                        produced = None;
                    }
                }
            }
            produced
        } {
            line_item_bounds.push(line_info.items.iter().map(|item| item.item_index).fold(
                None,
                |bounds, index| {
                    Some(match bounds {
                        Some((first, last)) => (first.min(index), last.max(index)),
                        None => (index, index),
                    })
                },
            ));
            bidi_reorder_line(&mut line_info.items, &items_data);

            if style.text_overflow == openui_style::TextOverflow::Ellipsis
                && style.overflow_x == openui_style::Overflow::Hidden
            {
                apply_text_overflow_ellipsis(&mut line_info, line_available, &items_data, style);
            }
            let clamp_after_this_line = clamp_budget.is_some_and(|budget| {
                line_fragments.len() + 1 >= budget
                    && (line_breaker.has_remaining_visible_content()
                        || (style.legacy_webkit_line_clamp
                            && line_breaker.has_remaining_forced_break()))
            });
            if clamp_after_this_line {
                apply_line_clamp_marker(
                    &mut line_info,
                    line_available,
                    &items_data,
                    style,
                    &clamp_ellipsis,
                );
            }

            let line_indent = if is_first_line {
                text_indent
            } else {
                LayoutUnit::zero()
            };
            let static_inline_origin = line_avail.inline_start
                + compute_text_align_offset(
                    &line_info,
                    line_avail.available_inline_size - line_indent,
                    style.direction,
                    style.text_align_last,
                )
                + physical_text_indent_offset(style.direction, line_indent);
            line_static_inline_data.push((line_info.clone(), static_inline_origin));

            let break_clear = line_break_clear(&line_info, &items_data);
            let clearance_target = if break_clear == Clear::None {
                None
            } else {
                exclusion_ref.map(|exclusions| {
                    exclusions.clearance_offset(clear_type(break_clear)) - bfc_block_start
                })
            };
            let clearance_advances = clearance_target.is_some_and(|target| target > block_offset);
            let clearance_at_float_boundary = clearance_target
                .is_some_and(|target| target == block_offset)
                && line_break_follows_float_at_same_boundary(
                    &line_info,
                    &inline_float_placeholders,
                );
            let clearance_only_break = exclusion_ref.is_some_and(ExclusionSpace::has_floats)
                && (clearance_advances || clearance_at_float_boundary);
            let clearance_only_extent = (clearance_only_break && clearance_advances).then(|| {
                let strut = compute_line_height_metrics(
                    &block_metrics,
                    &style.line_height,
                    style.font_size,
                );
                block_offset + LayoutUnit::from_f32(strut.ascent + strut.descent)
            });

            let line_start_boxes = boxes_open_at_line_start.clone();
            let line_fragment = create_line_box(
                doc,
                space,
                &items_data,
                &line_info,
                line_avail.available_inline_size,
                block_offset,
                style,
                &block_metrics,
                space.percentage_resolution_inline_size,
                if is_first_line {
                    text_indent
                } else {
                    LayoutUnit::zero()
                },
                space.percentage_resolution_block_size,
                &line_start_boxes,
                normalize_vertical_rtl,
                clearance_only_break,
            );

            // Update open inline box state for the next line.
            let mut current_open = boxes_open_at_line_start.clone();
            for item_result in &line_info.items {
                let item = &items_data.items[item_result.item_index];
                match item_result.item_type {
                    InlineItemType::OpenTag => {
                        let s = &items_data.styles[item.style_index];
                        current_open.push(InlineBoxState {
                            style_index: item.style_index,
                            node_id: item.node_id,
                            box_decoration_break: s.box_decoration_break,
                        });
                    }
                    InlineItemType::CloseTag => {
                        current_open.pop();
                    }
                    _ => {}
                }
            }
            boxes_open_at_line_start = current_open;

            // Offset the line box inline-start when floats intrude from the left.
            let mut positioned_line = line_fragment;
            if line_avail.inline_start > LayoutUnit::zero() {
                positioned_line.offset.left = positioned_line.offset.left + line_avail.inline_start;
            }

            let mut next_block_offset = block_offset + positioned_line.size.height;
            if let Some(target) = clearance_target {
                next_block_offset = next_block_offset.max_of(target);
            }
            if let Some(extent) = clearance_only_extent {
                next_block_offset = next_block_offset.max_of(extent);
            }
            let under_ruby_ink_end = line_has_under_ruby(doc, &line_info, &items_data).then(|| {
                block_offset
                    + fragment_descendant_text_ink_end(&positioned_line)
                        .unwrap_or_else(|| fragment_relative_block_end(&positioned_line))
            });
            let ruby_padding_allowance = resolve_margin_or_padding(
                &style.padding_bottom,
                space.percentage_resolution_inline_size,
            );
            let ruby_ink_exceeds_clamp = clamp_block_budget.is_some_and(|budget| {
                under_ruby_ink_end.is_some_and(|end| {
                    end > budget + ruby_padding_allowance + LayoutUnit::from_i32(1)
                })
            });
            let root_line_height = {
                let metrics = compute_line_height_metrics(
                    &block_metrics,
                    &style.line_height,
                    style.font_size,
                );
                LayoutUnit::from_f32(metrics.ascent + metrics.descent)
            };
            let tall_line_reaches_clamp = clamp_block_budget.is_some_and(|budget| {
                next_block_offset >= budget
                    && !line_breaker.is_finished()
                    && positioned_line.size.height > root_line_height
            });
            if clamp_block_budget.is_some_and(|budget| next_block_offset > budget)
                || tall_line_reaches_clamp
                || ruby_ink_exceeds_clamp
            {
                let mut trimmed_line = line_info.clone();
                let mut accepted = None;
                while !ruby_ink_exceeds_clamp
                    && remove_last_content_item_for_tall_clamp(&mut trimmed_line, &items_data)
                {
                    let mut marked_line = trimmed_line.clone();
                    apply_line_clamp_marker(
                        &mut marked_line,
                        line_available,
                        &items_data,
                        style,
                        &clamp_ellipsis,
                    );
                    let mut candidate = create_line_box(
                        doc,
                        space,
                        &items_data,
                        &marked_line,
                        line_avail.available_inline_size,
                        block_offset,
                        style,
                        &block_metrics,
                        space.percentage_resolution_inline_size,
                        if is_first_line {
                            text_indent
                        } else {
                            LayoutUnit::zero()
                        },
                        space.percentage_resolution_block_size,
                        &line_start_boxes,
                        normalize_vertical_rtl,
                        clearance_only_break,
                    );
                    if line_avail.inline_start > LayoutUnit::zero() {
                        candidate.offset.left = candidate.offset.left + line_avail.inline_start;
                    }
                    let candidate_end = block_offset + candidate.size.height;
                    if candidate_end
                        <= clamp_block_budget.unwrap_or(openui_geometry::LayoutUnit::max())
                    {
                        accepted = Some((marked_line, candidate, candidate_end));
                        break;
                    }
                }
                if let Some((marked_line, candidate, candidate_end)) = accepted {
                    if let Some((stored, _)) = line_static_inline_data.last_mut() {
                        *stored = marked_line;
                    }
                    block_offset = candidate_end;
                    line_fragments.push(candidate);
                    break;
                }
                line_item_bounds.pop();
                line_static_inline_data.pop();
                append_clamp_marker_to_last_line(doc, &mut line_fragments, style, &clamp_ellipsis);
                break;
            }
            let physical_clamp_after_this_line = clamp_block_budget.is_some_and(|budget| {
                (next_block_offset >= budget
                    || under_ruby_ink_end.is_some_and(|end| end > next_block_offset))
                    && !line_breaker.is_finished()
            });
            if physical_clamp_after_this_line && !clamp_after_this_line {
                apply_line_clamp_marker(
                    &mut line_info,
                    line_available,
                    &items_data,
                    style,
                    &clamp_ellipsis,
                );
                if let Some((stored, _)) = line_static_inline_data.last_mut() {
                    *stored = line_info.clone();
                }
                positioned_line = create_line_box(
                    doc,
                    space,
                    &items_data,
                    &line_info,
                    line_avail.available_inline_size,
                    block_offset,
                    style,
                    &block_metrics,
                    space.percentage_resolution_inline_size,
                    if is_first_line {
                        text_indent
                    } else {
                        LayoutUnit::zero()
                    },
                    space.percentage_resolution_block_size,
                    &line_start_boxes,
                    normalize_vertical_rtl,
                    clearance_only_break,
                );
                if line_avail.inline_start > LayoutUnit::zero() {
                    positioned_line.offset.left =
                        positioned_line.offset.left + line_avail.inline_start;
                }
            }
            if physical_clamp_after_this_line || clamp_after_this_line {
                if let Some(ink_end) = under_ruby_ink_end {
                    let integer_clamp_rounding = (clamp_after_this_line
                        && !physical_clamp_after_this_line
                        && ruby_padding_allowance == LayoutUnit::zero())
                    .then_some(LayoutUnit::from_i32(1))
                    .unwrap_or(LayoutUnit::zero());
                    next_block_offset = next_block_offset.max_of(
                        (ink_end - ruby_padding_allowance).floor() + integer_clamp_rounding,
                    );
                }
            }
            block_offset = next_block_offset;
            line_fragments.push(positioned_line);
            if clamp_after_this_line || physical_clamp_after_this_line {
                break;
            }
            is_first_line = false;
        }
    }

    if let Some(context) = &space.line_clamp_context {
        context.consume_layout(line_fragments.len(), block_offset);
    }
    let intrinsic_block_size = block_offset;

    if space.writing_direction.is_horizontal() {
        coalesce_trailing_empty_inline_end(&mut line_fragments);
    }

    let first_baseline = line_fragments
        .first()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    let last_baseline = line_fragments
        .last()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));

    let border_box_inline = space.available_inline_size;
    let border_box_size = PhysicalSize::new(border_box_inline, intrinsic_block_size);

    let mut fragment = Fragment::new_box(node_id, border_box_size);
    fragment.children = line_fragments;
    fragment.first_baseline = first_baseline;
    fragment.last_baseline = last_baseline;

    collect_atomic_inline_oof_candidates(doc, node_id, space, border_box_size, &mut fragment);

    append_positioned_inline_candidates(
        doc,
        node_id,
        space,
        &items_data,
        &line_item_bounds,
        &line_static_inline_data,
        intrinsic_block_size,
        border_box_size,
        normalize_vertical_rtl,
        &mut fragment,
    );

    fragment
}

/// Create a positioned line box fragment from a LineInfo.
///
/// This is the core of the inline layout algorithm:
/// 1. Compute line height using the half-leading model
/// 2. Apply vertical alignment
/// 3. Apply text alignment (horizontal offset)
/// 4. Position each item within the line box
///
/// Blink: `InlineLayoutAlgorithm::CreateLine()`.
fn snap_line_baseline(
    line_ascent: f32,
    line_height: &LineHeight,
    is_horizontal_writing_direction: bool,
) -> LayoutUnit {
    if *line_height == LineHeight::Normal || !is_horizontal_writing_direction {
        LayoutUnit::from_f32_ceil(line_ascent)
    } else {
        LayoutUnit::from_f32(line_ascent).floor()
    }
}

fn create_line_box(
    doc: &Document,
    space: &ConstraintSpace,
    items_data: &InlineItemsData,
    line_info: &LineInfo,
    available_width: LayoutUnit,
    block_offset: LayoutUnit,
    block_style: &ComputedStyle,
    block_metrics: &openui_text::FontMetrics,
    percentage_base: LayoutUnit,
    text_indent: LayoutUnit,
    percentage_block_base: LayoutUnit,
    boxes_open_at_line_start: &[InlineBoxState],
    normalize_vertical_rtl: bool,
    clearance_only_break: bool,
) -> Fragment {
    // === STEP 0: Check if line has content (CSS 2.1 §9.4.2) ===
    // "Line boxes that contain no text, no preserved white space, no inline
    // elements with a non-zero margin, padding, or border, and no other
    // in-flow content must be treated as zero-height line boxes."
    // A forced break (<br> or preserved newline) establishes a strut even
    // when it is the only item on the line. Without this, `<p><br></p>` has
    // zero height and following block content overlaps it.
    let line_has_content = line_info.has_ellipsis
        || (line_info.has_forced_break && !clearance_only_break)
        || line_info
            .items
            .iter()
            .any(|item_result| match item_result.item_type {
                InlineItemType::Text => !item_result.text_range.is_empty(),
                InlineItemType::AtomicInline => true,
                InlineItemType::OpenTag => {
                    let item = &items_data.items[item_result.item_index];
                    let s = &items_data.styles[item.style_index];
                    s.effective_border_left() > 0
                        || s.effective_border_right() > 0
                        || s.effective_border_top() > 0
                        || s.effective_border_bottom() > 0
                        || (s.padding_left.is_fixed() && s.padding_left.value() != 0.0)
                        || (s.padding_right.is_fixed() && s.padding_right.value() != 0.0)
                        || (s.padding_top.is_fixed() && s.padding_top.value() != 0.0)
                        || (s.padding_bottom.is_fixed() && s.padding_bottom.value() != 0.0)
                        || (s.margin_left.is_fixed() && s.margin_left.value() != 0.0)
                        || (s.margin_right.is_fixed() && s.margin_right.value() != 0.0)
                        || (s.margin_top.is_fixed() && s.margin_top.value() != 0.0)
                        || (s.margin_bottom.is_fixed() && s.margin_bottom.value() != 0.0)
                }
                _ => false,
            });

    // === STEP 1: Compute strut (minimum line height from block's font) ===
    let strut = compute_line_height_metrics(
        block_metrics,
        &block_style.line_height,
        block_style.font_size,
    );

    let mut line_ascent = if line_has_content { strut.ascent } else { 0.0 };
    let mut line_descent = if line_has_content { strut.descent } else { 0.0 };

    // Track items that need deferred vertical-align resolution (top/bottom).
    struct DeferredItem {
        item_ascent: f32,
        item_descent: f32,
        is_top: bool,
    }
    let mut deferred_items: Vec<DeferredItem> = Vec::new();

    // Stack of parent inline font metrics for vertical-align resolution.
    // When inside a nested inline (e.g., <span style="font-size:30px">),
    // text-top/text-bottom/middle should align to the parent inline's font
    // metrics, not the block container's. The stack is pushed on OpenTag
    // and popped on CloseTag.
    let mut inline_metrics_stack: Vec<FontMetrics> = Vec::new();
    let mut inline_font_size_stack: Vec<f32> = Vec::new();

    // === PRE-STEP: Run block_layout for atomic inlines ===
    // Per CSS 2.1 §10.6.1, inline-block/inline-flex/inline-grid establish a new
    // block formatting context. We pre-compute their layout so the result's
    // dimensions are authoritative for both line metrics and fragment creation.
    let mut atomic_layout_results: Vec<Option<Fragment>> =
        (0..line_info.items.len()).map(|_| None).collect();
    for (idx, item_result) in line_info.items.iter().enumerate() {
        if item_result.item_type == InlineItemType::AtomicInline {
            let item = &items_data.items[item_result.item_index];
            if !item.node_id.is_none() {
                let item_width = item_result.inline_size;
                let style = &items_data.styles[item.style_index];
                let logical_block_size = if space.writing_direction.is_horizontal() {
                    &style.height
                } else {
                    &style.width
                };
                let available_block = match logical_block_size.length_type() {
                    openui_geometry::LengthType::Fixed => {
                        LayoutUnit::from_f32(logical_block_size.value())
                    }
                    // An auto outer block-size on an atomic inline remains
                    // indefinite.  Passing the numeric maximum here makes an
                    // orthogonal child treat it as a definite inline measure
                    // after axis conversion and stretch to the viewport.
                    _ => openui_geometry::INDEFINITE_SIZE,
                };
                // Use the containing block's actual block size for percentage
                // resolution so that `height: 50%` etc. resolve correctly.
                // When the containing block height is indefinite or LayoutUnit::max(),
                // keep it indefinite so shrink-to-fit/intrinsic sizing runs.
                let percentage_block = if !percentage_block_base.is_indefinite()
                    && percentage_block_base > LayoutUnit::zero()
                    && percentage_block_base < LayoutUnit::max()
                {
                    percentage_block_base
                } else {
                    available_block
                };
                let child_space = crate::block_child_constraint_space(
                    // Atomic inline sizing is resolved in this IFC's logical
                    // axes. Derive the child's writing direction at the
                    // boundary so direction-only and orthogonal descendants
                    // consume the same authoritative constraint conversion as
                    // normal block and flex children.
                    space,
                    style,
                    item_width,
                    available_block,
                    percentage_base,
                    percentage_block,
                    true,
                );
                let mut result = if doc.node(item.node_id).tag == ElementTag::Ruby {
                    layout_ruby_atomic(
                        doc,
                        item.node_id,
                        item_width,
                        percentage_block,
                        block_style.line_clamp != openui_style::LineClamp::None,
                    )
                } else {
                    crate::block::block_layout(doc, item.node_id, &child_space)
                };
                // Inline layout stores coordinates as (inline, block). A child
                // layout returns a physical fragment, so normalize the atomic
                // box at this boundary before line metrics and placement use it.
                if !space.writing_direction.is_horizontal() {
                    let logical_size =
                        WritingModeConverter::new(space.writing_direction, PhysicalSize::zero())
                            .to_logical_size(result.size);
                    result.size =
                        PhysicalSize::new(logical_size.inline_size, logical_size.block_size);
                }
                // The line breaker resolves atomic inline outer sizing to a
                // border-box measure (including authored inline-axis edges).
                // Keep that used inline size authoritative after child block
                // layout; the latter receives a content constraint and can
                // otherwise return a box smaller by its own borders.
                if uses_deterministic_text_profile(style)
                    && doc.node(item.node_id).replaced.is_none()
                {
                    result.size.width = item_width;
                }
                atomic_layout_results[idx] = Some(result);
            }
        }
    }

    // === STEP 2: Compute per-item metrics and unite ===
    for (step2_idx, item_result) in line_info.items.iter().enumerate() {
        match item_result.item_type {
            InlineItemType::Text => {
                let item = &items_data.items[item_result.item_index];
                let style = &items_data.styles[item.style_index];
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let primary_metrics = font.font_metrics().copied().unwrap_or_default();
                let metrics =
                    text_line_metrics(primary_metrics, style, item, item_result, items_data);
                let item_lh =
                    compute_line_height_metrics(&metrics, &style.line_height, style.font_size);

                let element_line_height =
                    used_line_height(&metrics, &style.line_height, style.font_size);

                let effective_vertical_align = style.vertical_align;
                let alignment_font_size = style.font_size;
                let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);

                let baseline_shift = compute_baseline_shift(
                    &effective_vertical_align,
                    alignment_font_size,
                    parent_metrics.ascent,
                    parent_metrics.descent,
                    parent_metrics.x_height,
                    item_lh.ascent,
                    item_lh.descent,
                    element_line_height,
                );

                match effective_vertical_align {
                    VerticalAlign::Top => {
                        deferred_items.push(DeferredItem {
                            item_ascent: item_lh.ascent,
                            item_descent: item_lh.descent,
                            is_top: true,
                        });
                    }
                    VerticalAlign::Bottom => {
                        deferred_items.push(DeferredItem {
                            item_ascent: item_lh.ascent,
                            item_descent: item_lh.descent,
                            is_top: false,
                        });
                    }
                    _ => {
                        line_ascent = line_ascent.max(item_lh.ascent - baseline_shift);
                        line_descent = line_descent.max(item_lh.descent + baseline_shift);
                    }
                }
            }
            InlineItemType::AtomicInline => {
                // Atomic inline contributes its margin-box height to line metrics.
                // CSS 2.1 §10.8.1: inline-block margin boxes affect line box height.
                let item = &items_data.items[item_result.item_index];
                let style = &items_data.styles[item.style_index];

                let ruby_base_metrics = atomic_layout_results[step2_idx]
                    .as_ref()
                    .filter(|_| {
                        doc.node(item.node_id).tag == ElementTag::Ruby
                            && block_style.line_clamp != openui_style::LineClamp::None
                    })
                    .and_then(|result| {
                        let (base, annotation) = if style.ruby_position.is_over() {
                            (result.children.last()?, result.children.first())
                        } else {
                            (result.children.first()?, result.children.get(1))
                        };
                        let base_line = base.children.first()?;
                        let base_ink = fragment_text_ink_height(base_line);
                        let mut annotation_ink = annotation
                            .map(fragment_text_ink_height)
                            .unwrap_or(LayoutUnit::zero());
                        if annotation.is_some_and(|fragment| {
                            fragment_contains_tag(doc, fragment, ElementTag::Ruby)
                        }) {
                            annotation_ink += LayoutUnit::from_f32(
                                doc.children(item.node_id)
                                    .find(|child_id| {
                                        doc.node(*child_id).tag == ElementTag::RubyText
                                    })
                                    .map(|child_id| doc.node(child_id).style.font_size)
                                    .unwrap_or(0.0),
                            );
                        }
                        let over_expansion = if style.ruby_position.is_over() {
                            (annotation_ink - base_ink).clamp_negative_to_zero()
                        } else {
                            LayoutUnit::zero()
                        };
                        Some((
                            (base_line.size.height + over_expansion).to_f32(),
                            base_line.baseline_offset + over_expansion.to_f32(),
                        ))
                    });
                let item_height = if let Some((base_height, _)) = ruby_base_metrics {
                    // Ruby annotations are ink overflow around the base line;
                    // they do not replace the originating line-height strut.
                    // Keeping only the base area in line metrics lets leading
                    // and authored padding absorb over/under annotations while
                    // the full atomic fragment still paints them.
                    base_height
                } else if let Some(ref result) = atomic_layout_results[step2_idx] {
                    result.size.height.to_f32()
                } else {
                    let logical_block_size = if space.writing_direction.is_horizontal() {
                        &style.height
                    } else {
                        &style.width
                    };
                    match logical_block_size.length_type() {
                        openui_geometry::LengthType::Fixed => logical_block_size.value(),
                        _ => {
                            let font_desc = style_to_font_description(style);
                            let font = Font::new(font_desc);
                            let metrics = font.font_metrics().copied().unwrap_or_default();
                            metrics.ascent + metrics.descent
                        }
                    }
                };

                // Resolve vertical margins for line box contribution.
                let (block_start_margin, block_end_margin) =
                    if space.writing_direction.is_horizontal() {
                        (&style.margin_top, &style.margin_bottom)
                    } else if space.writing_direction.is_flipped_blocks() {
                        (&style.margin_right, &style.margin_left)
                    } else {
                        (&style.margin_left, &style.margin_right)
                    };
                let margin_top =
                    resolve_margin_or_padding(block_start_margin, percentage_base).to_f32();
                let margin_bottom =
                    resolve_margin_or_padding(block_end_margin, percentage_base).to_f32();
                let margin_box_height = item_height + margin_top + margin_bottom;
                let baseline_from_top =
                    ruby_base_metrics.map(|(_, baseline)| baseline).or_else(|| {
                        atomic_layout_results[step2_idx]
                            .as_ref()
                            .and_then(|result| {
                                let child_direction =
                                    style.direction.writing_direction(style.writing_mode);
                                if style.legacy_webkit_box {
                                    // The compatibility -webkit-box used by legacy
                                    // line clamping does not expose a descendant
                                    // line baseline when it is atomic inline-level.
                                    // Use the synthesized margin-box baseline below.
                                    None
                                } else if child_direction.is_horizontal()
                                    != space.writing_direction.is_horizontal()
                                {
                                    Some(result.size.height)
                                } else if style.display == Display::InlineBlock
                                    && uses_deterministic_text_profile(style)
                                {
                                    // CSS 2.1 §10.8.1: an inline-block exports the
                                    // baseline of its last in-flow line box.
                                    inline_block_last_line_baseline(result).or_else(|| {
                                        // Replaced form controls have their own
                                        // synthesized baseline even though they
                                        // contain no author-visible line box.
                                        (!result.node_id.is_none()
                                            && doc.node(result.node_id).form_control.is_some())
                                        .then_some(result.first_baseline)
                                        .flatten()
                                    })
                                } else {
                                    result.first_baseline
                                }
                            })
                            .map(|baseline| baseline.to_f32())
                    });

                match style.vertical_align {
                    VerticalAlign::Top => {
                        deferred_items.push(DeferredItem {
                            item_ascent: margin_box_height,
                            item_descent: 0.0,
                            is_top: true,
                        });
                    }
                    VerticalAlign::Bottom => {
                        deferred_items.push(DeferredItem {
                            item_ascent: margin_box_height,
                            item_descent: 0.0,
                            is_top: false,
                        });
                    }
                    VerticalAlign::Middle => {
                        // CSS defines "middle" relative to the parent inline's
                        // x-height, not the block's.
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        let x_height = parent_metrics.x_height;
                        let above_baseline = margin_box_height / 2.0 + x_height / 2.0;
                        let below_baseline = (margin_box_height / 2.0 - x_height / 2.0).max(0.0);
                        line_ascent = line_ascent.max(above_baseline);
                        line_descent = line_descent.max(below_baseline);
                    }
                    VerticalAlign::Length(px) => {
                        // Shift from baseline by px (negative = down).
                        let shifted_ascent = (margin_box_height + px).max(0.0);
                        let shifted_descent = (-px).max(0.0);
                        line_ascent = line_ascent.max(shifted_ascent);
                        line_descent = line_descent.max(shifted_descent);
                    }
                    VerticalAlign::Percentage(pct) => {
                        // Compute element's own line-height for percentage basis
                        let font_desc = style_to_font_description(style);
                        let font = Font::new(font_desc);
                        let metrics = font.font_metrics().copied().unwrap_or_default();
                        let element_line_height =
                            used_line_height(&metrics, &style.line_height, style.font_size);
                        let shift = element_line_height * pct / 100.0;
                        let shifted_ascent = (margin_box_height + shift).max(0.0);
                        let shifted_descent = (-shift).max(0.0);
                        line_ascent = line_ascent.max(shifted_ascent);
                        line_descent = line_descent.max(shifted_descent);
                    }
                    VerticalAlign::TextTop => {
                        // Margin-top of item aligns with parent inline's font ascent.
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        let font_ascent = parent_metrics.ascent;
                        line_ascent = line_ascent.max(font_ascent);
                        line_descent = line_descent.max((margin_box_height - font_ascent).max(0.0));
                    }
                    VerticalAlign::TextBottom => {
                        // Margin-bottom of item aligns with parent inline's font descent.
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        let font_descent = parent_metrics.descent;
                        line_ascent = line_ascent.max((margin_box_height - font_descent).max(0.0));
                        line_descent = line_descent.max(font_descent);
                    }
                    VerticalAlign::Sub => {
                        // Lowered by sub_offset below the baseline.
                        let parent_font_size = inline_font_size_stack
                            .last()
                            .copied()
                            .unwrap_or(block_style.font_size);
                        let sub_offset = keyword_baseline_shift(parent_font_size, 5);
                        line_ascent = line_ascent.max((margin_box_height - sub_offset).max(0.0));
                        line_descent = line_descent.max(sub_offset);
                    }
                    VerticalAlign::Super => {
                        // Raised by super_offset above the baseline.
                        let parent_font_size = inline_font_size_stack
                            .last()
                            .copied()
                            .unwrap_or(block_style.font_size);
                        let super_offset = keyword_baseline_shift(parent_font_size, 3);
                        line_ascent = line_ascent.max(margin_box_height + super_offset);
                        // Item bottom is at super_offset above baseline → 0 descent.
                        line_descent = line_descent.max(0.0);
                    }
                    _ => {
                        if let Some(baseline) = baseline_from_top {
                            line_ascent = line_ascent.max(margin_top + baseline);
                            line_descent =
                                line_descent.max((item_height - baseline).max(0.0) + margin_bottom);
                        } else {
                            // Baseline-aligned: margin-box bottom sits on baseline.
                            // For empty inline-blocks: baseline = bottom margin edge.
                            line_ascent = line_ascent.max(margin_box_height);
                            line_descent = line_descent.max(0.0);
                        }
                    }
                }
            }
            // OpenTag: push this inline element's font metrics onto the stack
            // so nested content uses the parent inline's metrics for vertical-align.
            InlineItemType::OpenTag => {
                let item = &items_data.items[item_result.item_index];
                let style = &items_data.styles[item.style_index];
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let metrics = font.font_metrics().copied().unwrap_or_default();
                let item_lh =
                    compute_line_height_metrics(&metrics, &style.line_height, style.font_size);
                let element_line_height =
                    used_line_height(&metrics, &style.line_height, style.font_size);
                let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                let parent_font_size = inline_font_size_stack
                    .last()
                    .copied()
                    .unwrap_or(block_style.font_size);
                let baseline_shift = compute_baseline_shift(
                    &style.vertical_align,
                    parent_font_size,
                    parent_metrics.ascent,
                    parent_metrics.descent,
                    parent_metrics.x_height,
                    item_lh.ascent,
                    item_lh.descent,
                    element_line_height,
                );
                // Font metrics from an inline box only enlarge an established
                // line. A line made solely of empty, undecorated inline boxes
                // is zero-height under CSS 2.1 §9.4.2.
                if line_has_content {
                    match style.vertical_align {
                        VerticalAlign::Top => deferred_items.push(DeferredItem {
                            item_ascent: item_lh.ascent,
                            item_descent: item_lh.descent,
                            is_top: true,
                        }),
                        VerticalAlign::Bottom => deferred_items.push(DeferredItem {
                            item_ascent: item_lh.ascent,
                            item_descent: item_lh.descent,
                            is_top: false,
                        }),
                        _ => {
                            line_ascent = line_ascent.max(item_lh.ascent - baseline_shift);
                            line_descent = line_descent.max(item_lh.descent + baseline_shift);
                        }
                    }
                }
                inline_metrics_stack.push(metrics);
                inline_font_size_stack.push(style.font_size);
            }
            // CloseTag: pop the parent inline's font metrics.
            InlineItemType::CloseTag => {
                inline_metrics_stack.pop();
                inline_font_size_stack.pop();
            }
            InlineItemType::Control => {
                // A semantic <br> has no inline-size or painted fragment, but
                // its computed font and line-height establish the strut for
                // the forced-break line. This matters when the break inherits
                // metrics from an inline ancestor that differ from the block.
                // A clearing break that actually advances past a float keeps
                // the existing clearance-only zero-height line contract.
                if !clearance_only_break {
                    let item = &items_data.items[item_result.item_index];
                    let style = &items_data.styles[item.style_index];
                    let initial = ComputedStyle::default();
                    // Raw Document callers do not run the CSS inheritance
                    // pass. Treat a completely initial break metric set as
                    // inherited from its block; generated retained breaks
                    // carry any non-initial inherited values explicitly.
                    let uses_initial_metrics = style.font_family == initial.font_family
                        && style.font_size == initial.font_size
                        && style.font_weight == initial.font_weight
                        && style.font_style == initial.font_style
                        && style.font_stretch == initial.font_stretch
                        && style.line_height == initial.line_height;
                    let item_lh = if uses_initial_metrics {
                        compute_line_height_metrics(
                            block_metrics,
                            &block_style.line_height,
                            block_style.font_size,
                        )
                    } else {
                        let font_desc = style_to_font_description(style);
                        let font = Font::new(font_desc);
                        let metrics = font.font_metrics().copied().unwrap_or_default();
                        compute_line_height_metrics(&metrics, &style.line_height, style.font_size)
                    };
                    line_ascent = line_ascent.max(item_lh.ascent);
                    line_descent = line_descent.max(item_lh.descent);
                }
            }
            // BlockInInline: handled separately in block layout.
            InlineItemType::BlockInInline => {}
        }
    }

    // === STEP 2b: Resolve deferred top/bottom items ===
    // Top/bottom aligned items may expand the line box but use the already-
    // computed line height from other items.
    for deferred in &deferred_items {
        let item_total = deferred.item_ascent + deferred.item_descent;
        let line_total = line_ascent + line_descent;
        if item_total > line_total {
            // Expand the line to fit this item.
            let extra = item_total - line_total;
            if deferred.is_top {
                // Top-aligned: extra goes to descent side.
                line_descent += extra;
            } else {
                // Bottom-aligned: extra goes to ascent side.
                line_ascent += extra;
            }
        }
    }

    // An explicit line height remains authoritative for a line made solely
    // from atomic inlines that all fit inside that strut.
    // A synthesized inline-block baseline can otherwise add a fractional
    // descent to every line (for example 25.5px for a 25px atomic), causing
    // the clamp edge to drift even though no item is taller than the line.
    let atomic_only_explicit_line = block_style.line_height != LineHeight::Normal
        && line_info
            .items
            .iter()
            .any(|item| item.item_type == InlineItemType::AtomicInline)
        && line_info.items.iter().all(|item| match item.item_type {
            InlineItemType::AtomicInline => true,
            InlineItemType::Text => items_data
                .text
                .get(item.text_range.clone())
                .is_none_or(|text| text.chars().all(char::is_whitespace)),
            _ => false,
        });
    if atomic_only_explicit_line && deferred_items.is_empty() {
        let used_height = used_line_height(
            block_metrics,
            &block_style.line_height,
            block_style.font_size,
        );
        let atomics_fit = line_info
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.item_type == InlineItemType::AtomicInline)
            .all(|(index, item)| {
                let inline_item = &items_data.items[item.item_index];
                let style = &items_data.styles[inline_item.style_index];
                let margins = if space.writing_direction.is_horizontal() {
                    (&style.margin_top, &style.margin_bottom)
                } else if space.writing_direction.is_flipped_blocks() {
                    (&style.margin_right, &style.margin_left)
                } else {
                    (&style.margin_left, &style.margin_right)
                };
                let margin_box_height = atomic_layout_results[index]
                    .as_ref()
                    .map_or(LayoutUnit::zero(), |fragment| fragment.size.height)
                    + resolve_margin_or_padding(margins.0, percentage_base)
                    + resolve_margin_or_padding(margins.1, percentage_base);
                margin_box_height.to_f32() <= used_height
            });
        if atomics_fit && line_ascent <= used_height {
            line_descent = (used_height - line_ascent).max(0.0);
        }
    }

    let line_height = LayoutUnit::from_f32_ceil(line_ascent + line_descent);
    // Blink snaps negative half-leading from an explicit line-height toward
    // the line's block start. A normal line-height retains the font metric's
    // device-pixel ceiling; flooring that case moves tiny Ahem glyphs one row
    // above their line box after fragmentation.
    let baseline = snap_line_baseline(
        line_ascent,
        &block_style.line_height,
        space.writing_direction.is_horizontal(),
    );

    // Pre-shape hyphen to include its width in alignment calculations.
    let hyphen_shape_data = if line_info.has_forced_hyphen && !line_info.has_ellipsis {
        let last_style = line_info
            .items
            .last()
            .map(|r| &items_data.styles[items_data.items[r.item_index].style_index])
            .unwrap_or(block_style);
        let hyphen_font_desc = style_to_font_description(last_style);
        let hyphen_font = Font::new(hyphen_font_desc);
        let shaper = TextShaper::new();
        let hyphen_text = "-";
        let hyphen_sr = shaper.shape(hyphen_text, &hyphen_font, openui_text::TextDirection::Ltr);
        let hyphen_width = LayoutUnit::from_f32(hyphen_sr.width);
        Some((Arc::new(hyphen_sr), hyphen_width, last_style.clone()))
    } else {
        None
    };

    let hyphen_extra_width = hyphen_shape_data
        .as_ref()
        .map(|(_, w, _)| *w)
        .unwrap_or(LayoutUnit::zero());

    // Pre-compute ellipsis width so alignment accounts for it.
    let ellipsis_extra_width = if line_info.has_ellipsis {
        let block_font_desc = style_to_font_description(block_style);
        let ellipsis_font = Font::new(block_font_desc);
        let shaper = TextShaper::new();
        let marker = line_info.ellipsis_text.as_deref().unwrap_or("\u{2026}");
        let sr = shaper.shape(marker, &ellipsis_font, openui_text::TextDirection::Ltr);
        LayoutUnit::from_f32(sr.width)
    } else {
        LayoutUnit::zero()
    };

    // Total extra width from appended glyphs (hyphen or ellipsis, never both).
    let appended_extra_width = hyphen_extra_width + ellipsis_extra_width;

    // === STEP 3: Horizontal positioning (text-align) ===
    // Use the effective available width (after text-indent) for alignment
    // so that text-align: right with text-indent doesn't overflow.
    let align_available = available_width - text_indent;
    let text_align_offset = compute_text_align_offset(
        line_info,
        align_available - appended_extra_width,
        block_style.direction,
        block_style.text_align_last,
    );

    // === STEP 3b: Justification ===
    // Distribute extra space among word gaps (or character gaps) if justified.
    let mut justification_per_space = 0.0f32;
    let mut justification_per_char = 0.0f32;
    let text_justify = block_style.text_justify;

    // Determine if this line should be justified: text-align:justify on
    // non-last, non-forced-break lines, OR text-align-last:justify on last lines.
    let should_justify = if line_info.is_last_line || line_info.has_forced_break {
        block_style.text_align_last == TextAlignLast::Justify
    } else {
        line_info.text_align == TextAlign::Justify
    };

    if should_justify && text_justify != TextJustify::None {
        let remaining = align_available - appended_extra_width - line_info.used_width;
        if remaining > LayoutUnit::zero() {
            match text_justify {
                TextJustify::InterCharacter => {
                    let char_count = count_inter_character_opportunities(line_info, items_data);
                    if char_count > 0 {
                        justification_per_char = remaining.to_f32() / char_count as f32;
                    }
                }
                // Auto: use inter-character for CJK content, inter-word otherwise.
                TextJustify::Auto => {
                    if detect_cjk_content(line_info, items_data) {
                        let char_count = count_inter_character_opportunities(line_info, items_data);
                        if char_count > 0 {
                            justification_per_char = remaining.to_f32() / char_count as f32;
                        }
                    } else {
                        let space_count = count_expansion_opportunities(line_info, items_data);
                        if space_count > 0 {
                            justification_per_space = remaining.to_f32() / space_count as f32;
                        }
                    }
                }
                TextJustify::InterWord => {
                    let space_count = count_expansion_opportunities(line_info, items_data);
                    if space_count > 0 {
                        justification_per_space = remaining.to_f32() / space_count as f32;
                    }
                }
                TextJustify::None => {} // unreachable due to outer guard
            }
        }
    }

    // === STEP 4: Position each item ===
    let mut children: Vec<Fragment> = Vec::new();
    let mut line_oof_candidates: Vec<OutOfFlowCandidate> = Vec::new();
    let mut inline_boxes: Vec<InlineLineBox> = Vec::new();
    let mut inline_box_roots: Vec<InlineLineChild> = Vec::new();
    let mut inline_box_record_stack: Vec<usize> = Vec::new();
    let physical_text_indent = physical_text_indent_offset(block_style.direction, text_indent);
    let mut inline_offset = text_align_offset + physical_text_indent;
    let mut justification_accumulator = 0.0f32;
    // Track how many characters we've seen before this item (for inter-character
    // justification boundary gaps — Issue 4 fix).
    let mut inter_char_chars_before = 0usize;

    // Reset the inline metrics stack for the positioning pass.
    inline_metrics_stack.clear();
    inline_font_size_stack.clear();

    // --- Inline box decoration tracking (CSS Fragmentation §4.4) ---
    // Build a set of style indices for boxes open at line start, for quick lookup.
    let boxes_open_at_start_set: Vec<usize> = boxes_open_at_line_start
        .iter()
        .map(|b| b.style_index)
        .collect();

    // Re-open per-line fragments for inline boxes continued from the previous
    // line. Clone gets fresh inline-start MBP; slice begins directly at the
    // continuation content edge.
    for open_box in boxes_open_at_line_start {
        let style = &items_data.styles[open_box.style_index];
        if open_box.box_decoration_break == BoxDecorationBreak::Clone {
            inline_offset =
                inline_offset + resolve_margin_or_padding(&style.margin_left, percentage_base);
        }
        let border_start = inline_offset;
        if open_box.box_decoration_break == BoxDecorationBreak::Clone {
            inline_offset = inline_offset
                + LayoutUnit::from_i32(style.effective_border_left())
                + resolve_margin_or_padding(&style.padding_left, percentage_base);
        }
        let index = inline_boxes.len();
        inline_boxes.push(InlineLineBox {
            node_id: open_box.node_id,
            style_index: open_box.style_index,
            border_start,
            border_end: border_start,
            is_first: false,
            is_last: false,
            has_left_edge: open_box.box_decoration_break == BoxDecorationBreak::Clone,
            has_right_edge: false,
            children: Vec::new(),
        });
        attach_inline_line_child(
            &inline_box_record_stack,
            &mut inline_boxes,
            &mut inline_box_roots,
            InlineLineChild::InlineBox(index),
        );
        inline_box_record_stack.push(index);
    }

    // Pre-scan to determine which boxes don't close on this line (needed for
    // clone mode inline-end MBP and for is_last_for_node metadata).
    let mut scan_stack: Vec<usize> = boxes_open_at_start_set.clone();
    for item_result in &line_info.items {
        match item_result.item_type {
            InlineItemType::OpenTag => {
                let item = &items_data.items[item_result.item_index];
                scan_stack.push(item.style_index);
            }
            InlineItemType::CloseTag => {
                scan_stack.pop();
            }
            _ => {}
        }
    }
    // scan_stack now contains style indices of boxes still open at line end.
    let boxes_open_at_line_end: Vec<usize> = scan_stack;

    // Track the current inline box stack during positioning for
    // is_first_for_node / is_last_for_node metadata on text fragments.
    // Each entry: (style_index, is_first_fragment_on_this_line)
    let mut inline_box_stack: Vec<(usize, bool)> = boxes_open_at_start_set
        .iter()
        .map(|&si| (si, false)) // boxes from previous line are NOT first
        .collect();

    // A single logical inline can be split into multiple visual fragments by
    // bidi reordering on the same line. Its physical left/right decorations
    // belong only to the first/last visual fragment; repeated synthesized tag
    // pairs carry the shared middle content without duplicating those edges.
    let mut close_tag_totals: HashMap<usize, usize> = HashMap::new();
    for result in &line_info.items {
        match result.item_type {
            InlineItemType::CloseTag => {
                *close_tag_totals.entry(result.item_index).or_default() += 1
            }
            _ => {}
        }
    }
    let mut open_tag_seen: HashMap<usize, usize> = HashMap::new();
    let mut close_tag_seen: HashMap<usize, usize> = HashMap::new();

    for (step4_idx, item_result) in line_info.items.iter().enumerate() {
        let item = &items_data.items[item_result.item_index];
        match item_result.item_type {
            InlineItemType::Text => {
                let style = &items_data.styles[item.style_index];
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let primary_metrics = font.font_metrics().copied().unwrap_or_default();
                let metrics =
                    text_line_metrics(primary_metrics, style, item, item_result, items_data);

                let element_line_height =
                    used_line_height(&metrics, &style.line_height, style.font_size);

                // Compute half-leading-adjusted metrics for baseline shift
                // (CSS 2.2 §10.8.1: text-top/text-bottom/middle use the inline
                // box, not the content area).
                let item_lh =
                    compute_line_height_metrics(&metrics, &style.line_height, style.font_size);

                let effective_vertical_align = style.vertical_align;
                let alignment_font_size = style.font_size;
                let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);

                let baseline_shift = compute_baseline_shift(
                    &effective_vertical_align,
                    alignment_font_size,
                    parent_metrics.ascent,
                    parent_metrics.descent,
                    parent_metrics.x_height,
                    item_lh.ascent,
                    item_lh.descent,
                    element_line_height,
                );

                // Compute vertical offset for top/bottom aligned items.
                let effective_shift = match effective_vertical_align {
                    VerticalAlign::Top => {
                        // Align top of item with top of line box.
                        -(line_ascent - item_lh.ascent)
                    }
                    VerticalAlign::Bottom => {
                        // Align bottom of item with bottom of line box.
                        line_descent - item_lh.descent
                    }
                    _ => baseline_shift,
                };

                // Text top = baseline position - font ascent, adjusted for shift.
                let text_top =
                    baseline - LayoutUnit::from_f32_ceil(metrics.ascent - effective_shift);

                // Compute sub-range shape result for the line portion.
                // When text wraps, the item_result's text_range may be a subset
                // of the full item's text_range. Clip the shape result to only
                // the characters visible on this line (Issue 1 fix).
                let line_shape_result = if let Some(ref sr) = item_result.shape_result {
                    let item_byte_start = item.text_range.start;
                    let line_byte_start = item_result.text_range.start;
                    let line_byte_end = item_result.text_range.end;
                    let full_text = &items_data.text;
                    let char_start = byte_to_char_offset(full_text, line_byte_start)
                        - byte_to_char_offset(full_text, item_byte_start);
                    let char_end = byte_to_char_offset(full_text, line_byte_end)
                        - byte_to_char_offset(full_text, item_byte_start);
                    if char_start == 0 && char_end == sr.num_characters {
                        item_result.shape_result.clone()
                    } else {
                        Some(Arc::new(sr.sub_range(char_start, char_end)))
                    }
                } else {
                    None
                };

                // Compute item width, adding justification if applicable.
                let mut item_width = item_result.inline_size;
                let mut justified_shape: Option<Arc<ShapeResult>> = None;
                if should_justify && (justification_per_space > 0.0 || justification_per_char > 0.0)
                {
                    let text = &items_data.text[item_result.text_range.clone()];
                    // In pre-wrap mode, the last text item's trailing spaces
                    // hang and must not receive justification expansion.
                    let is_last_text = line_info.items[(step4_idx + 1)..]
                        .iter()
                        .all(|ir| ir.item_type != InlineItemType::Text);
                    let style_for_item = &items_data.styles[item.style_index];
                    let trailing_spaces = if is_last_text
                        && matches!(
                            style_for_item.white_space,
                            openui_style::WhiteSpace::PreWrap
                                | openui_style::WhiteSpace::BreakSpaces
                        ) {
                        text.chars().rev().take_while(|c| *c == ' ').count()
                    } else {
                        0
                    };

                    if justification_per_char > 0.0 {
                        // Inter-character justification: distribute extra space
                        // between every character boundary.
                        let char_count = text.chars().count().saturating_sub(trailing_spaces);
                        let internal_gaps = char_count.saturating_sub(1);
                        // Boundary gap: if there are characters before this item,
                        // add one gap for the boundary between the previous text
                        // item's last char and this item's first char (Issue 4 fix).
                        let boundary_gap = if inter_char_chars_before > 0 && char_count > 0 {
                            1
                        } else {
                            0
                        };
                        let total_item_gaps = internal_gaps + boundary_gap;
                        if total_item_gaps > 0 {
                            let extra = justification_per_char * total_item_gaps as f32;
                            let old_acc = justification_accumulator;
                            justification_accumulator += extra;
                            let extra_lu = LayoutUnit::from_f32(justification_accumulator)
                                - LayoutUnit::from_f32(old_acc);
                            item_width = item_width + extra_lu;
                            // Shift x position by boundary gap amount.
                            if boundary_gap > 0 {
                                let boundary_shift = LayoutUnit::from_f32(justification_per_char);
                                inline_offset = inline_offset + boundary_shift;
                                // Reduce item_width by boundary_shift since it's positional.
                                item_width = item_width - boundary_shift;
                            }
                            // Create justified shape result with modified glyph advances.
                            if let Some(ref sr) = line_shape_result {
                                let mut justified_sr = sr.sub_range(0, sr.num_characters);
                                justified_sr
                                    .apply_inter_character_justification(justification_per_char);
                                justified_shape = Some(Arc::new(justified_sr));
                            }
                        }
                        inter_char_chars_before += char_count;
                    } else {
                        let total = text.chars().filter(|c| *c == ' ').count();
                        let space_count = total - trailing_spaces;
                        if space_count > 0 {
                            let extra = justification_per_space * space_count as f32;
                            let old_acc = justification_accumulator;
                            justification_accumulator += extra;
                            let extra_lu = LayoutUnit::from_f32(justification_accumulator)
                                - LayoutUnit::from_f32(old_acc);
                            item_width = item_width + extra_lu;
                            // Also adjust glyph advances in the shape result so that
                            // space glyphs are visually wider — exclude trailing spaces.
                            if let Some(ref sr) = line_shape_result {
                                let mut justified_sr = sr.sub_range(0, sr.num_characters);
                                justified_sr.apply_justification(
                                    justification_per_space,
                                    text,
                                    trailing_spaces,
                                );
                                justified_shape = Some(Arc::new(justified_sr));
                            }
                        }
                    }
                }

                let text_height = LayoutUnit::from_f32_ceil(metrics.ascent + metrics.descent);

                let mut text_fragment =
                    Fragment::new_box(item.node_id, PhysicalSize::new(item_width, text_height));
                text_fragment.kind = FragmentKind::Text;
                // Populate text_content so paint pipeline can use it for
                // emphasis marks and skip-ink CJK filtering.
                let text_content = &items_data.text[item_result.text_range.clone()];
                text_fragment.text_content = Some(text_content.to_string());
                text_fragment.text_run_orientation =
                    resolve_text_run_orientation(style, text_content);
                // First-line/first-letter layout uses a styled clone of the
                // inline item stream. Preserve that resolved style on the
                // fragment; looking the DOM node up again during paint would
                // lose pseudo colors, backgrounds, shadows, and decorations.
                text_fragment.inherited_style = Some(style.clone());
                text_fragment.offset = PhysicalOffset::new(inline_offset, text_top);
                // Store the baseline offset (distance from fragment top to baseline)
                // so paint can use it directly instead of recomputing from metrics.
                text_fragment.baseline_offset = (baseline - text_top).to_f32();
                // Use justified shape result if justification was applied,
                // otherwise use the sub-range shape result for this line portion.
                text_fragment.shape_result = justified_shape.or(line_shape_result);

                // Set inline box decoration metadata for the paint system.
                // If this text is inside an inline box (span), indicate whether
                // it's on the first/last line of that box.
                if let Some(&(style_idx, is_first)) = inline_box_stack.last() {
                    text_fragment.is_first_for_node = is_first;
                    text_fragment.is_last_for_node = !boxes_open_at_line_end.contains(&style_idx);
                }

                let child_index = children.len();
                children.push(text_fragment);
                attach_inline_line_child(
                    &inline_box_record_stack,
                    &mut inline_boxes,
                    &mut inline_box_roots,
                    InlineLineChild::Fragment(child_index),
                );
                inline_offset = inline_offset + item_width;
            }
            InlineItemType::OpenTag => {
                let style = &items_data.styles[item.style_index];
                let seen = open_tag_seen.entry(item_result.item_index).or_default();
                let has_left_edge = *seen == 0;
                *seen += 1;
                if has_left_edge {
                    inline_offset = inline_offset
                        + resolve_margin_or_padding(&style.margin_left, percentage_base);
                }
                let border_start = inline_offset;
                if has_left_edge {
                    inline_offset = inline_offset
                        + LayoutUnit::from_i32(style.effective_border_left())
                        + resolve_margin_or_padding(&style.padding_left, percentage_base);
                }
                let index = inline_boxes.len();
                inline_boxes.push(InlineLineBox {
                    node_id: item.node_id,
                    style_index: item.style_index,
                    border_start,
                    border_end: border_start,
                    is_first: has_left_edge,
                    is_last: false,
                    has_left_edge,
                    has_right_edge: false,
                    children: Vec::new(),
                });
                attach_inline_line_child(
                    &inline_box_record_stack,
                    &mut inline_boxes,
                    &mut inline_box_roots,
                    InlineLineChild::InlineBox(index),
                );
                inline_box_record_stack.push(index);
                // Push this inline element's font metrics for nested content.
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let metrics = font.font_metrics().copied().unwrap_or_default();
                inline_metrics_stack.push(metrics);
                inline_font_size_stack.push(style.font_size);
                // Track that this box opened on this line (is_first = true).
                inline_box_stack.push((item.style_index, true));
            }
            InlineItemType::CloseTag => {
                if let Some(index) = inline_box_record_stack.pop() {
                    let style = &items_data.styles[inline_boxes[index].style_index];
                    let seen = close_tag_seen.entry(item_result.item_index).or_default();
                    *seen += 1;
                    let has_right_edge = *seen
                        == close_tag_totals
                            .get(&item_result.item_index)
                            .copied()
                            .unwrap_or(1);
                    let logical_end_advance = if style.direction == openui_style::Direction::Rtl {
                        resolve_margin_or_padding(&style.padding_left, percentage_base)
                            + LayoutUnit::from_i32(style.effective_border_left())
                            + resolve_margin_or_padding(&style.margin_left, percentage_base)
                    } else {
                        resolve_margin_or_padding(&style.padding_right, percentage_base)
                            + LayoutUnit::from_i32(style.effective_border_right())
                            + resolve_margin_or_padding(&style.margin_right, percentage_base)
                    };
                    let shaping_run_rounding_excess =
                        (logical_end_advance - item_result.inline_size).clamp_negative_to_zero();
                    if has_right_edge {
                        inline_offset = inline_offset
                            + resolve_margin_or_padding(&style.padding_right, percentage_base)
                            + LayoutUnit::from_i32(style.effective_border_right());
                    }
                    // Keep following glyph origins on their independently
                    // quantized advances, but size this decoration fragment
                    // from the cumulatively quantized text extent encoded by
                    // the line breaker on the structural close item.
                    inline_boxes[index].border_end = inline_offset
                        - if has_right_edge {
                            shaping_run_rounding_excess
                        } else {
                            LayoutUnit::zero()
                        };
                    inline_boxes[index].is_last = has_right_edge;
                    inline_boxes[index].has_right_edge = has_right_edge;
                    if has_right_edge {
                        inline_offset = inline_offset
                            + resolve_margin_or_padding(&style.margin_right, percentage_base);
                    }
                }
                inline_metrics_stack.pop();
                inline_font_size_stack.pop();
                inline_box_stack.pop();
            }
            InlineItemType::Control => {
                // <br> — no visual contribution, line break already handled.
                // Reset inter-character tracking so no boundary gap is added
                // between text items separated by a control.
                inter_char_chars_before = 0;
            }
            InlineItemType::AtomicInline | InlineItemType::BlockInInline => {
                // Reset inter-character tracking so no boundary gap is added
                // between text items separated by an atomic inline.
                inter_char_chars_before = 0;
                // Use pre-computed block_layout result as authoritative source
                // for atomic inline dimensions (Issue 2 fix).
                let item_width = item_result.inline_size;
                let style = &items_data.styles[item.style_index];
                let item_height = if let Some(ref result) = atomic_layout_results[step4_idx] {
                    result.size.height
                } else {
                    let logical_block_size = if space.writing_direction.is_horizontal() {
                        &style.height
                    } else {
                        &style.width
                    };
                    match logical_block_size.length_type() {
                        openui_geometry::LengthType::Fixed => {
                            LayoutUnit::from_f32(logical_block_size.value())
                        }
                        _ => {
                            let font_desc = style_to_font_description(style);
                            let font = Font::new(font_desc);
                            let metrics = font.font_metrics().copied().unwrap_or_default();
                            LayoutUnit::from_f32_ceil(metrics.ascent + metrics.descent)
                        }
                    }
                };

                // Resolve vertical margins (CSS 2.1 §10.8.1: margin box participates in line box).
                let (block_start_margin, block_end_margin) =
                    if space.writing_direction.is_horizontal() {
                        (&style.margin_top, &style.margin_bottom)
                    } else if space.writing_direction.is_flipped_blocks() {
                        (&style.margin_right, &style.margin_left)
                    } else {
                        (&style.margin_left, &style.margin_right)
                    };
                let margin_top_lu = resolve_margin_or_padding(block_start_margin, percentage_base);
                let margin_bottom_lu = resolve_margin_or_padding(block_end_margin, percentage_base);
                let margin_box_height_lu = margin_top_lu + item_height + margin_bottom_lu;
                let baseline_from_top =
                    atomic_layout_results[step4_idx]
                        .as_ref()
                        .and_then(|result| {
                            let child_direction =
                                style.direction.writing_direction(style.writing_mode);
                            if style.legacy_webkit_box {
                                // Match the legacy atomic -webkit-box baseline:
                                // synthesize it from the margin-box bottom instead
                                // of propagating an internal clamped line baseline.
                                None
                            } else if child_direction.is_horizontal()
                                != space.writing_direction.is_horizontal()
                            {
                                Some(result.size.height)
                            } else if style.display == Display::InlineBlock
                                && uses_deterministic_text_profile(style)
                            {
                                inline_block_last_line_baseline(result).or_else(|| {
                                    // Replaced form controls have their own
                                    // synthesized baseline even though they
                                    // contain no author-visible line box.
                                    (!result.node_id.is_none()
                                        && doc.node(result.node_id).form_control.is_some())
                                    .then_some(result.first_baseline)
                                    .flatten()
                                })
                            } else {
                                result.first_baseline
                            }
                        });

                let atomic_top = match style.vertical_align {
                    VerticalAlign::Top => {
                        // Top margin edge flush with top of line box.
                        margin_top_lu
                    }
                    VerticalAlign::Bottom => {
                        if space.writing_direction.is_horizontal() {
                            // Bottom margin edge flush with bottom of a
                            // horizontal line box.
                            line_height - item_height - margin_bottom_lu
                        } else {
                            // In a vertical line the physical bottom keyword
                            // addresses the inline-axis end; block-axis
                            // placement remains at the line's start edge.
                            margin_top_lu
                        }
                    }
                    VerticalAlign::Middle => {
                        // Center of margin box at baseline - x_height/2.
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        let x_height = LayoutUnit::from_f32(parent_metrics.x_height);
                        let two = LayoutUnit::from_f32(2.0);
                        // The ascent is device-snapped toward block-start. Do
                        // not let the discarded subpixel half-leading move an
                        // item above the line box that it expanded to contain.
                        (baseline - x_height / two - margin_box_height_lu / two + margin_top_lu)
                            .max_of(LayoutUnit::zero())
                    }
                    VerticalAlign::TextTop => {
                        // Top margin edge aligns with parent font ascent.
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        baseline - LayoutUnit::from_f32_ceil(parent_metrics.ascent) + margin_top_lu
                    }
                    VerticalAlign::TextBottom => {
                        // Bottom margin edge aligns with parent font descent.
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        baseline + LayoutUnit::from_f32_ceil(parent_metrics.descent)
                            - item_height
                            - margin_bottom_lu
                    }
                    VerticalAlign::Sub => {
                        let parent_font_size = inline_font_size_stack
                            .last()
                            .copied()
                            .unwrap_or(block_style.font_size);
                        let shift =
                            LayoutUnit::from_f32(keyword_baseline_shift(parent_font_size, 5));
                        baseline + shift - item_height - margin_bottom_lu
                    }
                    VerticalAlign::Super => {
                        let parent_font_size = inline_font_size_stack
                            .last()
                            .copied()
                            .unwrap_or(block_style.font_size);
                        let shift =
                            LayoutUnit::from_f32(keyword_baseline_shift(parent_font_size, 3));
                        baseline - shift - item_height - margin_bottom_lu
                    }
                    VerticalAlign::Length(px) => {
                        // Positive length shifts up from baseline.
                        let shift = LayoutUnit::from_f32(px);
                        baseline - item_height - margin_bottom_lu - shift
                    }
                    VerticalAlign::Percentage(pct) => {
                        // Percentage of the element's own line-height (CSS 2.2 §10.8.1).
                        let font_desc = style_to_font_description(style);
                        let font = Font::new(font_desc);
                        let metrics = font.font_metrics().copied().unwrap_or_default();
                        let element_line_height =
                            used_line_height(&metrics, &style.line_height, style.font_size);
                        let shift = LayoutUnit::from_f32(element_line_height * pct / 100.0);
                        baseline - item_height - margin_bottom_lu - shift
                    }
                    _ => {
                        // Baseline (default): align the atomic inline's exported
                        // baseline when it has one; otherwise fall back to the
                        // CSS inline-block synthesized bottom-margin baseline.
                        if let Some(item_baseline) = baseline_from_top {
                            baseline - item_baseline
                        } else {
                            baseline - item_height - margin_bottom_lu
                        }
                    }
                };

                // Apply horizontal margins to offset.
                let (inline_start_margin, inline_end_margin) =
                    if space.writing_direction.is_horizontal() {
                        (&style.margin_left, &style.margin_right)
                    } else {
                        (&style.margin_top, &style.margin_bottom)
                    };
                let margin_left_lu =
                    resolve_margin_or_padding(inline_start_margin, percentage_base);
                let margin_right_lu = resolve_margin_or_padding(inline_end_margin, percentage_base);

                // Use the pre-computed block_layout result as the atomic fragment,
                // preserving its computed size, border, padding, margin, and children.
                // Only fall back to a new empty box when block_layout was not run.
                let mut atomic_fragment = if let Some(result) =
                    atomic_layout_results[step4_idx].take()
                {
                    let mut frag = result;
                    // Use block_layout's authoritative width; only override height
                    // and position. block_layout already accounts for border+padding.
                    frag.size.height = item_height;
                    frag.offset = PhysicalOffset::new(inline_offset + margin_left_lu, atomic_top);
                    frag
                } else {
                    let mut frag =
                        Fragment::new_box(item.node_id, PhysicalSize::new(item_width, item_height));
                    frag.offset = PhysicalOffset::new(inline_offset + margin_left_lu, atomic_top);
                    frag
                };
                // block_layout computes atomic geometry before the line
                // breaker assigns its final normal-flow position. Reapply the
                // relative visual offset after that assignment; otherwise a
                // positioned image/control loses left/top when it becomes an
                // atomic inline fragment.
                crate::relative::apply_relative_offset(
                    &mut atomic_fragment,
                    style,
                    percentage_base,
                    space.percentage_resolution_block_size,
                );

                let containing_inline = inline_box_record_stack.iter().rev().find_map(|index| {
                    let inline_box = &inline_boxes[*index];
                    items_data.styles[inline_box.style_index]
                        .position
                        .is_positioned()
                        .then_some(inline_box.node_id)
                });
                for mut candidate in std::mem::take(&mut atomic_fragment.oof_candidates) {
                    candidate.static_position.left =
                        candidate.static_position.left + atomic_fragment.offset.left;
                    candidate.static_position.top =
                        candidate.static_position.top + atomic_fragment.offset.top;
                    if candidate.inline_containing_block_node.is_none() {
                        candidate.inline_containing_block_node = containing_inline;
                    }
                    line_oof_candidates.push(candidate);
                }

                let child_index = children.len();
                children.push(atomic_fragment);
                attach_inline_line_child(
                    &inline_box_record_stack,
                    &mut inline_boxes,
                    &mut inline_box_roots,
                    InlineLineChild::Fragment(child_index),
                );
                // Advance by the full margin-box inline size so subsequent
                // items start at the correct position.
                inline_offset =
                    inline_offset + margin_left_lu + item_result.inline_size + margin_right_lu;
            }
        }
    }

    // For `box-decoration-break: clone`, boxes still open at line end need
    // their inline-end MBP added after all items have been positioned.
    for &index in inline_box_record_stack.iter().rev() {
        let style = &items_data.styles[inline_boxes[index].style_index];
        if style.box_decoration_break == BoxDecorationBreak::Clone {
            inline_offset = inline_offset
                + resolve_margin_or_padding(&style.padding_right, percentage_base)
                + LayoutUnit::from_i32(style.effective_border_right());
            inline_boxes[index].border_end = inline_offset;
            inline_boxes[index].has_right_edge = true;
            inline_offset =
                inline_offset + resolve_margin_or_padding(&style.margin_right, percentage_base);
        } else {
            inline_boxes[index].border_end = inline_offset;
        }
    }

    // === STEP 4b: Append visible hyphen if line was broken at a soft hyphen ===
    if let Some((ref hyphen_sr, hyphen_width, ref hyphen_style)) = hyphen_shape_data {
        let hyphen_metrics = {
            let hyphen_font_desc = style_to_font_description(hyphen_style);
            let hyphen_font = Font::new(hyphen_font_desc);
            hyphen_font.font_metrics().copied().unwrap_or_default()
        };
        let hyphen_height =
            LayoutUnit::from_f32_ceil(hyphen_metrics.ascent + hyphen_metrics.descent);
        let hyphen_top = baseline - LayoutUnit::from_f32_ceil(hyphen_metrics.ascent);

        let mut hyphen_fragment = Fragment::new_text(
            NodeId::NONE,
            PhysicalSize::new(hyphen_width, hyphen_height),
            Arc::clone(hyphen_sr),
            "-".to_string(),
        );
        hyphen_fragment.inherited_style = Some(hyphen_style.clone());
        hyphen_fragment.baseline_offset = (baseline - hyphen_top).to_f32();
        hyphen_fragment.text_run_orientation = resolve_text_run_orientation(hyphen_style, "-");

        if block_style.direction == Direction::Rtl {
            // RTL: place hyphen at visual start (left of content), shift content right.
            for child in &mut children {
                child.offset =
                    PhysicalOffset::new(child.offset.left + hyphen_width, child.offset.top);
            }
            hyphen_fragment.offset =
                PhysicalOffset::new(text_align_offset + physical_text_indent, hyphen_top);
            let child_index = children.len();
            children.push(hyphen_fragment);
            attach_inline_line_child(
                &inline_box_record_stack,
                &mut inline_boxes,
                &mut inline_box_roots,
                InlineLineChild::Fragment(child_index),
            );
        } else {
            hyphen_fragment.offset = PhysicalOffset::new(inline_offset, hyphen_top);
            let child_index = children.len();
            children.push(hyphen_fragment);
            attach_inline_line_child(
                &inline_box_record_stack,
                &mut inline_boxes,
                &mut inline_box_roots,
                InlineLineChild::Fragment(child_index),
            );
            inline_offset = inline_offset + hyphen_width;
        }
    }

    // === STEP 5: Paint ellipsis if text-overflow: ellipsis is active ===
    if line_info.has_ellipsis {
        // Shape the configured clamp marker (U+2026 for text-overflow/auto).
        let block_font_desc = style_to_font_description(block_style);
        let ellipsis_font = Font::new(block_font_desc);
        let shaper = TextShaper::new();
        let ellipsis_text = line_info.ellipsis_text.as_deref().unwrap_or("\u{2026}");
        let ellipsis_sr = shaper.shape(
            ellipsis_text,
            &ellipsis_font,
            openui_text::TextDirection::Ltr,
        );
        let ellipsis_width = LayoutUnit::from_f32(ellipsis_sr.width);
        let ellipsis_metrics = ellipsis_font.font_metrics().copied().unwrap_or_default();
        let ellipsis_height =
            LayoutUnit::from_f32_ceil(ellipsis_metrics.ascent + ellipsis_metrics.descent);
        let ellipsis_top = baseline - LayoutUnit::from_f32_ceil(ellipsis_metrics.ascent);

        if line_info.ellipsis_at_start {
            // RTL: place ellipsis at the left edge, shift content right.
            for child in &mut children {
                child.offset =
                    PhysicalOffset::new(child.offset.left + ellipsis_width, child.offset.top);
            }
            let mut ellipsis_fragment = Fragment::new_text(
                NodeId::NONE,
                PhysicalSize::new(ellipsis_width, ellipsis_height),
                Arc::new(ellipsis_sr),
                ellipsis_text.to_string(),
            );
            ellipsis_fragment.offset =
                PhysicalOffset::new(text_align_offset + physical_text_indent, ellipsis_top);
            ellipsis_fragment.inherited_style = Some(block_style.clone());
            ellipsis_fragment.baseline_offset = (baseline - ellipsis_top).to_f32();
            ellipsis_fragment.text_run_orientation =
                resolve_text_run_orientation(block_style, ellipsis_text);
            let child_index = children.len();
            children.push(ellipsis_fragment);
            attach_inline_line_child(
                &inline_box_record_stack,
                &mut inline_boxes,
                &mut inline_box_roots,
                InlineLineChild::Fragment(child_index),
            );
        } else {
            // LTR: place ellipsis at the right edge (after content).
            let mut ellipsis_fragment = Fragment::new_text(
                NodeId::NONE,
                PhysicalSize::new(ellipsis_width, ellipsis_height),
                Arc::new(ellipsis_sr),
                ellipsis_text.to_string(),
            );
            ellipsis_fragment.offset = PhysicalOffset::new(
                inline_offset + line_info.ellipsis_inline_start_advance,
                ellipsis_top,
            );
            ellipsis_fragment.inherited_style = Some(block_style.clone());
            ellipsis_fragment.baseline_offset = (baseline - ellipsis_top).to_f32();
            ellipsis_fragment.text_run_orientation =
                resolve_text_run_orientation(block_style, ellipsis_text);
            let child_index = children.len();
            children.push(ellipsis_fragment);
            attach_inline_line_child(
                &inline_box_record_stack,
                &mut inline_boxes,
                &mut inline_box_roots,
                InlineLineChild::Fragment(child_index),
            );
        }
    }

    // Replace the flat text/atomic list with paintable per-line inline box
    // fragments. Descendant offsets are converted from line coordinates to
    // their nearest inline fragment's local coordinate space.
    let mut flat_fragments: Vec<Option<Fragment>> = children.into_iter().map(Some).collect();
    let children = inline_box_roots
        .into_iter()
        .map(|child| {
            materialize_inline_line_child(
                child,
                &inline_boxes,
                &mut flat_fragments,
                items_data,
                percentage_base,
                baseline,
                line_height,
                block_metrics,
                block_style.font_size,
                block_style.line_clamp == openui_style::LineClamp::None
                    && space.line_clamp_context.is_none(),
            )
        })
        .collect();

    // Build the line box fragment.
    let mut line_fragment = Fragment::new_box(
        NodeId::NONE,
        PhysicalSize::new(available_width, line_height),
    );
    line_fragment.offset = PhysicalOffset::new(LayoutUnit::zero(), block_offset);
    line_fragment.baseline_offset = baseline.to_f32();
    line_fragment.children = children;
    line_fragment.oof_candidates = line_oof_candidates;
    if normalize_vertical_rtl {
        normalize_vertical_line_inline_axis(&mut line_fragment, space.writing_direction);
    }
    line_fragment
}

// ── BiDi visual reordering ──────────────────────────────────────────────

/// Reorder items within a line for visual display per UAX#9 L2.
///
/// After the line breaker produces a line in logical order, this function
/// reorders items so RTL runs are visually reversed.
///
/// Blink: `InlineLayoutAlgorithm::BidiReorder` / `ReorderInlineItems`.
fn bidi_reorder_line(items: &mut Vec<InlineItemResult>, items_data: &InlineItemsData) {
    if items.is_empty() {
        return;
    }

    let max_level = items
        .iter()
        .map(|ir| items_data.items[ir.item_index].bidi_level)
        .max()
        .unwrap_or(0);

    if max_level == 0 {
        return; // All LTR, no reordering needed
    }

    let min_odd = match items
        .iter()
        .map(|ir| items_data.items[ir.item_index].bidi_level)
        .filter(|l| l % 2 == 1)
        .min()
    {
        Some(v) => v,
        None => return, // No odd levels → no reordering needed
    };

    // Inline element boundary tokens are logical tree markers, not UAX#9
    // characters. Reversing them together with text prevents visual runs from
    // crossing element boundaries and can create an invalid tag stack. For a
    // balanced line, attach each paintable item to its logical inline ancestry,
    // reorder only those paintable items, then synthesize a balanced boundary
    // sequence around each contiguous visual fragment.
    let original = std::mem::take(items);
    let has_inline_boundaries = original.iter().any(|item| {
        matches!(
            item.item_type,
            InlineItemType::OpenTag | InlineItemType::CloseTag
        )
    });
    let mut open_stack: Vec<usize> = Vec::new();
    let mut open_results: HashMap<usize, InlineItemResult> = HashMap::new();
    let mut close_results: HashMap<usize, InlineItemResult> = HashMap::new();
    let mut visual_items: Vec<(InlineItemResult, Vec<usize>)> = Vec::new();
    let mut balanced = true;

    if has_inline_boundaries {
        for result in original.iter().cloned() {
            match result.item_type {
                InlineItemType::OpenTag => {
                    open_results.insert(result.item_index, result.clone());
                    open_stack.push(result.item_index);
                }
                InlineItemType::CloseTag => {
                    if let Some(open_index) = open_stack.pop() {
                        close_results.insert(open_index, result);
                    } else {
                        balanced = false;
                        break;
                    }
                }
                _ => visual_items.push((result, open_stack.clone())),
            }
        }
        balanced &= open_stack.is_empty()
            && open_results.len() == close_results.len()
            && !visual_items.is_empty();
    }

    if !has_inline_boundaries || !balanced {
        *items = original;
    } else {
        *items = Vec::new();
    }

    // UAX#9 L2: for each level from max down to min odd level, reverse every
    // maximal contiguous run of paintable items at that level or higher.
    if balanced && has_inline_boundaries {
        for level in (min_odd..=max_level).rev() {
            let mut i = 0;
            while i < visual_items.len() {
                let item_level = items_data.items[visual_items[i].0.item_index].bidi_level;
                if item_level >= level {
                    let start = i;
                    while i < visual_items.len()
                        && items_data.items[visual_items[i].0.item_index].bidi_level >= level
                    {
                        i += 1;
                    }
                    visual_items[start..i].reverse();
                } else {
                    i += 1;
                }
            }
        }

        let mut current_ancestry: Vec<usize> = Vec::new();
        for (result, desired_ancestry) in visual_items {
            let common = current_ancestry
                .iter()
                .zip(&desired_ancestry)
                .take_while(|(left, right)| left == right)
                .count();
            for open_index in current_ancestry[common..].iter().rev() {
                items.push(
                    close_results
                        .get(open_index)
                        .expect("balanced inline boundary has a close result")
                        .clone(),
                );
            }
            for open_index in &desired_ancestry[common..] {
                items.push(
                    open_results
                        .get(open_index)
                        .expect("balanced inline boundary has an open result")
                        .clone(),
                );
            }
            items.push(result);
            current_ancestry = desired_ancestry;
        }
        for open_index in current_ancestry.iter().rev() {
            items.push(
                close_results
                    .get(open_index)
                    .expect("balanced inline boundary has a close result")
                    .clone(),
            );
        }
        return;
    }

    // Lines with an inline continuation from another line are not locally
    // balanced. Preserve their boundary tokens and use the legacy item-level
    // ordering until the continuation ancestry is available to this stage.
    for level in (min_odd..=max_level).rev() {
        let mut i = 0;
        while i < items.len() {
            let item_level = items_data.items[items[i].item_index].bidi_level;
            if item_level >= level {
                let start = i;
                while i < items.len() {
                    let l = items_data.items[items[i].item_index].bidi_level;
                    if l >= level {
                        i += 1;
                    } else {
                        break;
                    }
                }
                items[start..i].reverse();
            } else {
                i += 1;
            }
        }
    }
}

// ── Text-overflow: ellipsis ─────────────────────────────────────────────

/// Apply text-overflow: ellipsis to a line that overflows.
///
/// Removes trailing items until the line fits within available width
/// minus the ellipsis width. When the last remaining item is text and
/// partially fits, it is trimmed instead of fully removed. The caller
/// is responsible for actually painting the ellipsis character.
///
/// Blink: `NGLineInfo::SetHasEllipsis` / `NGLineTruncator`.
fn apply_text_overflow_ellipsis(
    line_info: &mut LineInfo,
    available_width: LayoutUnit,
    items_data: &InlineItemsData,
    block_style: &ComputedStyle,
) {
    apply_ellipsis_marker(
        line_info,
        available_width,
        items_data,
        block_style,
        "\u{2026}",
        false,
        false,
    );
}

fn ellipsis_item_advance(
    result: &InlineItemResult,
    items_data: &InlineItemsData,
    percentage_base: LayoutUnit,
    block_style: &ComputedStyle,
) -> LayoutUnit {
    if result.item_type != InlineItemType::AtomicInline {
        return result.inline_size;
    }
    let item = &items_data.items[result.item_index];
    let style = &items_data.styles[item.style_index];
    let margin = if block_style.writing_mode == openui_style::WritingMode::HorizontalTb {
        resolve_margin_or_padding(&style.margin_left, percentage_base)
            + resolve_margin_or_padding(&style.margin_right, percentage_base)
    } else {
        resolve_margin_or_padding(&style.margin_top, percentage_base)
            + resolve_margin_or_padding(&style.margin_bottom, percentage_base)
    };
    result.inline_size + margin
}

fn ellipsis_atomic_margin_start(
    result: &InlineItemResult,
    items_data: &InlineItemsData,
    percentage_base: LayoutUnit,
    block_style: &ComputedStyle,
) -> LayoutUnit {
    if result.item_type != InlineItemType::AtomicInline {
        return LayoutUnit::zero();
    }
    let item = &items_data.items[result.item_index];
    let style = &items_data.styles[item.style_index];
    if block_style.writing_mode == openui_style::WritingMode::HorizontalTb {
        resolve_margin_or_padding(&style.margin_left, percentage_base)
    } else {
        resolve_margin_or_padding(&style.margin_top, percentage_base)
    }
}

fn apply_ellipsis_marker(
    line_info: &mut LineInfo,
    available_width: LayoutUnit,
    items_data: &InlineItemsData,
    block_style: &ComputedStyle,
    marker: &str,
    force: bool,
    prefer_soft_wrap: bool,
) {
    if !force && line_info.used_width <= available_width {
        return;
    }

    // Preserved trailing spaces may hang outside the line box, but a clamp
    // marker replaces them at the logical line end. Remove their item
    // geometry before fitting and alignment so inline backgrounds do not
    // paint beyond the marker and right/justified lines position the marker
    // from the non-hanging content width.
    if force && line_info.hang_width > LayoutUnit::zero() {
        let mut hanging = line_info.hang_width;
        while hanging > LayoutUnit::zero() && !line_info.items.is_empty() {
            let last = line_info.items.last().expect("non-empty: loop condition");
            if last.item_type != InlineItemType::Text {
                if last.inline_size <= LayoutUnit::zero() {
                    line_info.items.pop();
                    continue;
                }
                break;
            }
            let trim = hanging.min_of(last.inline_size);
            if trim >= last.inline_size {
                hanging = hanging - last.inline_size;
                line_info.items.pop();
                continue;
            }
            let item = &items_data.items[last.item_index];
            let Some(shape) = item.shape_result.as_ref() else {
                break;
            };
            let keep_width = last.inline_size - trim;
            let text = &items_data.text[last.text_range.clone()];
            let item_char_start = byte_to_char_offset(&items_data.text, item.text_range.start);
            let portion_char_start =
                byte_to_char_offset(&items_data.text, last.text_range.start) - item_char_start;
            let mut kept_byte_end = 0usize;
            let mut kept_width = LayoutUnit::zero();
            for (byte_end, _) in text
                .grapheme_indices(true)
                .map(|(offset, grapheme)| (offset + grapheme.len(), grapheme))
            {
                let char_end = portion_char_start + text[..byte_end].chars().count();
                let width =
                    LayoutUnit::from_f32(shape.width_for_range(portion_char_start, char_end));
                if width > keep_width {
                    break;
                }
                kept_byte_end = byte_end;
                kept_width = width;
            }
            let last = line_info
                .items
                .last_mut()
                .expect("non-empty: guarded above");
            last.text_range.end = last.text_range.start + kept_byte_end;
            last.inline_size = kept_width;
            hanging = LayoutUnit::zero();
        }
        line_info.hang_width = hanging;
    }

    let block_font_desc = style_to_font_description(block_style);
    let block_font = Font::new(block_font_desc);
    let shaper = TextShaper::new();
    let ellipsis_sr = shaper.shape(marker, &block_font, openui_text::TextDirection::Ltr);
    let ellipsis_width = LayoutUnit::from_f32(ellipsis_sr.width);

    let target_width = available_width - ellipsis_width;

    let is_rtl = block_style.direction == Direction::Rtl;
    // An emergency break inside one unbreakable token is not a normal
    // block-ellipsis opportunity. If that token already fills the line,
    // replace it as a unit instead of preserving a first-letter split or a
    // DOM-boundary prefix before the marker.
    let visible_text_end = line_info
        .items
        .iter()
        .filter(|result| result.item_type == InlineItemType::Text)
        .map(|result| result.text_range.end)
        .max();
    let continues_same_word = visible_text_end.is_some_and(|end| {
        let previous = items_data.text[..end].chars().next_back();
        let next = items_data.text[end..].chars().next();
        previous.is_some_and(|character| !character.is_whitespace())
            && next.is_some_and(|character| !character.is_whitespace())
    });
    if prefer_soft_wrap
        && !line_info.has_forced_break
        && (line_info.used_width >= available_width || continues_same_word)
        && !line_info.items.iter().any(|result| {
            result.item_type == InlineItemType::Text
                && result.inline_size > LayoutUnit::zero()
                && items_data.text[result.text_range.clone()]
                    .chars()
                    .any(char::is_whitespace)
        })
    {
        line_info
            .items
            .retain(|result| result.item_type != InlineItemType::Text);
        line_info.used_width = line_info
            .items
            .iter()
            .fold(LayoutUnit::zero(), |width, item| width + item.inline_size);
    }

    if target_width <= LayoutUnit::zero() {
        line_info.items.clear();
        line_info.used_width = LayoutUnit::zero();
        line_info.has_ellipsis = true;
        line_info.ellipsis_text = Some(marker.to_string());
        line_info.has_forced_hyphen = false;
        if is_rtl {
            line_info.ellipsis_at_start = true;
        }
        return;
    }

    if is_rtl {
        // RTL: truncate from the visual left (beginning of items after bidi reorder).
        // Remove items from the front until we have room for the ellipsis.
        while line_info.used_width > target_width && !line_info.items.is_empty() {
            let first_size = ellipsis_item_advance(
                &line_info.items[0],
                items_data,
                available_width,
                block_style,
            );
            if first_size <= LayoutUnit::zero()
                && line_info.items[0].item_type != InlineItemType::Text
            {
                line_info.items.remove(0);
                continue;
            }

            let excess = line_info.used_width - target_width;
            if line_info.items[0].item_type == InlineItemType::Text && first_size > excess {
                // Partial fit — trim from the left side of this text item.
                let item_target = first_size - excess;
                let item = &items_data.items[line_info.items[0].item_index];
                if let Some(ref sr) = item.shape_result {
                    let line_text = &items_data.text[line_info.items[0].text_range.clone()];
                    let item_char_start =
                        byte_to_char_offset(&items_data.text, item.text_range.start);
                    let portion_char_start =
                        byte_to_char_offset(&items_data.text, line_info.items[0].text_range.start);
                    let portion_char_end =
                        byte_to_char_offset(&items_data.text, line_info.items[0].text_range.end);
                    let local_start = portion_char_start - item_char_start;
                    let local_end = portion_char_end - item_char_start;
                    let total_chars = local_end - local_start;

                    // Find the first grapheme-safe trim point from the start
                    // that fits. Use grapheme clusters and safe_to_break_before
                    // to avoid splitting emoji/ZWJ sequences.
                    let mut trim_chars = 0;
                    let mut trim_byte_offset = 0;
                    for (byte_offset, _grapheme) in line_text.grapheme_indices(true).skip(1) {
                        let char_count = line_text[..byte_offset].chars().count();
                        let local_trim = local_start + char_count;
                        // Only trim at shaping-safe positions.
                        if !sr.safe_to_break_before(local_trim) {
                            continue;
                        }
                        let remaining_width =
                            LayoutUnit::from_f32(sr.width_for_range(local_trim, local_end));
                        if remaining_width <= item_target {
                            trim_chars = char_count;
                            trim_byte_offset = byte_offset;
                            break;
                        }
                    }

                    if trim_chars > 0 && trim_chars < total_chars {
                        let trimmed_width = LayoutUnit::from_f32(
                            sr.width_for_range(local_start + trim_chars, local_end),
                        );
                        let new_text_start = line_info.items[0].text_range.start + trim_byte_offset;
                        let old_size = first_size;

                        let first_mut = &mut line_info.items[0];
                        first_mut.inline_size = trimmed_width;
                        first_mut.text_range = new_text_start..first_mut.text_range.end;
                        line_info.used_width = line_info.used_width - old_size + trimmed_width;
                    } else {
                        line_info.used_width = line_info.used_width - first_size;
                        line_info.items.remove(0);
                    }
                } else {
                    line_info.used_width = line_info.used_width - first_size;
                    line_info.items.remove(0);
                }
                break;
            }

            line_info.used_width = line_info.used_width - first_size;
            line_info.items.remove(0);
        }
    } else {
        // LTR: truncate from the visual right (end of items).
        let mut soft_wrap_used = false;
        let mut removed_text_item_indices = Vec::new();
        let mut removed_text_boundary = None;
        let mut removed_atomic_boundary_margin = None;
        while line_info.used_width > target_width && !line_info.items.is_empty() {
            if let Some(last) = line_info.items.last() {
                let last_size =
                    ellipsis_item_advance(last, items_data, available_width, block_style);
                if last_size <= LayoutUnit::zero() && last.item_type != InlineItemType::Text {
                    line_info.items.pop();
                    continue;
                }

                let excess = line_info.used_width - target_width;
                if last.item_type == InlineItemType::Text && last_size > excess {
                    let item_target = last_size - excess;
                    let item = &items_data.items[last.item_index];
                    if let Some(ref sr) = item.shape_result {
                        let line_text = &items_data.text[last.text_range.clone()];
                        let item_char_start =
                            byte_to_char_offset(&items_data.text, item.text_range.start);
                        let portion_char_start =
                            byte_to_char_offset(&items_data.text, last.text_range.start);
                        let local_start = portion_char_start - item_char_start;

                        // block-ellipsis rewinds to a normal soft-wrap
                        // opportunity. Emergency overflow-wrap breaks are
                        // deliberately excluded; text-overflow keeps the
                        // grapheme-safe fallback below.
                        let soft_fit = if prefer_soft_wrap {
                            let style = &items_data.styles[item.style_index];
                            let wrapping_allowed = !matches!(
                                style.white_space,
                                openui_style::WhiteSpace::Pre | openui_style::WhiteSpace::Nowrap
                            );
                            wrapping_allowed
                                .then(|| {
                                    find_break_opportunities(
                                        line_text,
                                        style.word_break,
                                        style.overflow_wrap,
                                        style.line_break,
                                    )
                                    .into_iter()
                                    .rev()
                                    .find_map(|byte_end| {
                                        let visible_byte_end = if matches!(
                                            style.white_space,
                                            openui_style::WhiteSpace::Normal
                                                | openui_style::WhiteSpace::Nowrap
                                                | openui_style::WhiteSpace::PreLine
                                        ) {
                                            line_text[..byte_end]
                                                .trim_end_matches([' ', '\t'])
                                                .len()
                                        } else {
                                            byte_end
                                        };
                                        if visible_byte_end == 0 {
                                            return None;
                                        }
                                        let char_count =
                                            line_text[..visible_byte_end].chars().count();
                                        let local_end = local_start + char_count;
                                        if !sr.safe_to_break_before(local_end) {
                                            return None;
                                        }
                                        let width = LayoutUnit::from_f32(
                                            sr.width_for_range(local_start, local_end),
                                        );
                                        (width <= item_target).then_some((visible_byte_end, width))
                                    })
                                })
                                .flatten()
                        } else {
                            None
                        };

                        if let Some((fit_byte_end, trimmed_width)) = soft_fit {
                            let new_text_end = last.text_range.start + fit_byte_end;
                            let old_size = last_size;
                            let last_mut = line_info
                                .items
                                .last_mut()
                                .expect("non-empty: guarded by while-loop condition above");
                            last_mut.inline_size = trimmed_width;
                            last_mut.text_range = last_mut.text_range.start..new_text_end;
                            line_info.used_width = line_info.used_width - old_size + trimmed_width;
                            soft_wrap_used = true;
                            break;
                        }

                        if prefer_soft_wrap {
                            line_info.used_width = line_info.used_width - last_size;
                            removed_text_item_indices.push(last.item_index);
                            removed_text_boundary = Some(last.text_range.start);
                            line_info.items.pop();
                            continue;
                        }

                        // Walk grapheme boundaries from end to find the
                        // largest safe-to-break prefix that fits.
                        let mut fit_chars = 0;
                        let mut fit_byte_end = 0;
                        let grapheme_offsets: Vec<usize> = line_text
                            .grapheme_indices(true)
                            .skip(1)
                            .map(|(off, _)| off)
                            .collect();
                        for &byte_offset in grapheme_offsets.iter().rev() {
                            let char_count = line_text[..byte_offset].chars().count();
                            let local_trim = local_start + char_count;
                            if !sr.safe_to_break_before(local_trim) {
                                continue;
                            }
                            let w =
                                LayoutUnit::from_f32(sr.width_for_range(local_start, local_trim));
                            if w <= item_target {
                                fit_chars = char_count;
                                fit_byte_end = byte_offset;
                                break;
                            }
                        }

                        if fit_chars > 0 {
                            let trimmed_width = LayoutUnit::from_f32(
                                sr.width_for_range(local_start, local_start + fit_chars),
                            );
                            let new_text_end = last.text_range.start + fit_byte_end;
                            let old_size = last_size;

                            let last_mut = line_info
                                .items
                                .last_mut()
                                .expect("non-empty: guarded by while-loop condition above");
                            last_mut.inline_size = trimmed_width;
                            last_mut.text_range = last_mut.text_range.start..new_text_end;
                            line_info.used_width = line_info.used_width - old_size + trimmed_width;
                        } else {
                            line_info.used_width = line_info.used_width - last_size;
                            line_info.items.pop();
                        }
                    } else {
                        line_info.used_width = line_info.used_width - last_size;
                        line_info.items.pop();
                    }
                    break;
                }

                if last.item_type == InlineItemType::AtomicInline {
                    removed_atomic_boundary_margin = Some(ellipsis_atomic_margin_start(
                        last,
                        items_data,
                        available_width,
                        block_style,
                    ));
                }
                line_info.used_width = line_info.used_width - last_size;
                line_info.items.pop();
            }
        }
        if line_info
            .items
            .iter()
            .any(|item| item.item_type == InlineItemType::AtomicInline)
        {
            let boundary_margin = removed_atomic_boundary_margin.unwrap_or(LayoutUnit::zero());
            line_info.used_width = line_info.used_width + boundary_margin;
            line_info.ellipsis_inline_start_advance = boundary_margin;
        }
        if prefer_soft_wrap && !soft_wrap_used {
            let mut boundary = removed_text_boundary;
            while let Some(index) = line_info
                .items
                .iter()
                .rposition(|item| item.item_type == InlineItemType::Text)
            {
                let retained_item = &line_info.items[index];
                if !removed_text_item_indices.contains(&retained_item.item_index)
                    && boundary == Some(retained_item.text_range.end)
                {
                    let text = &items_data.text[retained_item.text_range.clone()];
                    let visible_len = text.trim_end_matches(char::is_whitespace).len();
                    if visible_len < text.len() {
                        let item = &items_data.items[retained_item.item_index];
                        let new_end = retained_item.text_range.start + visible_len;
                        let new_width =
                            item.shape_result
                                .as_ref()
                                .map_or(LayoutUnit::zero(), |shape| {
                                    let item_char_start = byte_to_char_offset(
                                        &items_data.text,
                                        item.text_range.start,
                                    );
                                    let local_start = byte_to_char_offset(
                                        &items_data.text,
                                        retained_item.text_range.start,
                                    ) - item_char_start;
                                    let local_end = byte_to_char_offset(&items_data.text, new_end)
                                        - item_char_start;
                                    LayoutUnit::from_f32(
                                        shape.width_for_range(local_start, local_end),
                                    )
                                });
                        let old_width = retained_item.inline_size;
                        let retained_item = &mut line_info.items[index];
                        retained_item.text_range.end = new_end;
                        retained_item.inline_size = new_width;
                        line_info.used_width = line_info.used_width - old_width + new_width;
                        break;
                    }
                }
                if !removed_text_item_indices.contains(&line_info.items[index].item_index)
                    && boundary != Some(line_info.items[index].text_range.end)
                {
                    break;
                }
                let removed = line_info.items.remove(index);
                boundary = Some(removed.text_range.start);
                line_info.used_width = line_info.used_width - removed.inline_size;
            }
        }
    }

    line_info.has_ellipsis = true;
    line_info.ellipsis_text = Some(marker.to_string());
    line_info.has_forced_hyphen = false;
    line_info.ellipsis_at_start = is_rtl;
}

pub(crate) fn line_clamp_budget(
    style: &ComputedStyle,
    available_block_size: LayoutUnit,
) -> Option<usize> {
    use openui_style::{LineClamp, WebkitBoxOrient};
    if matches!(&style.block_ellipsis, openui_style::BlockEllipsis::String(value) if value.is_empty())
    {
        return None;
    }
    match style.line_clamp {
        LineClamp::None => None,
        LineClamp::Lines(lines) => {
            let legacy_inactive = style.legacy_webkit_line_clamp
                && (!style.legacy_webkit_box
                    || style.webkit_box_orient != WebkitBoxOrient::Vertical);
            (!legacy_inactive).then_some(lines as usize)
        }
        LineClamp::Auto => {
            line_clamp_auto_block_size(style, available_block_size)?;
            Some(usize::MAX)
        }
    }
}

pub(crate) fn line_clamp_auto_block_size(
    style: &ComputedStyle,
    available_block_size: LayoutUnit,
) -> Option<LayoutUnit> {
    let length = if !style.max_height.is_none() && !style.max_height.is_auto() {
        &style.max_height
    } else if !style.height.is_auto() {
        &style.height
    } else {
        return None;
    };
    let mut block_size = crate::length_resolver::resolve_length(
        length,
        available_block_size,
        available_block_size,
        available_block_size,
    );
    if !style.min_height.is_auto() && !style.min_height.is_none() {
        block_size = block_size.max_of(crate::length_resolver::resolve_length(
            &style.min_height,
            available_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        ));
    }
    Some(block_size)
}

fn effective_line_clamp(
    style: &ComputedStyle,
    space: &ConstraintSpace,
) -> (
    Option<usize>,
    Option<LayoutUnit>,
    openui_style::BlockEllipsis,
) {
    if let Some(context) = &space.line_clamp_context {
        let (remaining, block_ellipsis) = context.snapshot();
        (
            Some(remaining),
            context.remaining_block_size(),
            block_ellipsis,
        )
    } else {
        (
            line_clamp_budget(style, space.available_block_size),
            (style.line_clamp != openui_style::LineClamp::None)
                .then(|| line_clamp_auto_block_size(style, space.available_block_size))
                .flatten(),
            style.block_ellipsis.clone(),
        )
    }
}

fn apply_line_clamp_marker(
    line_info: &mut LineInfo,
    available_width: LayoutUnit,
    items_data: &InlineItemsData,
    block_style: &ComputedStyle,
    block_ellipsis: &openui_style::BlockEllipsis,
) {
    use openui_style::BlockEllipsis;
    let marker = match block_ellipsis {
        BlockEllipsis::NoEllipsis => return,
        BlockEllipsis::Auto => "\u{2026}",
        BlockEllipsis::String(value) => value.as_str(),
    };
    apply_ellipsis_marker(
        line_info,
        available_width,
        items_data,
        block_style,
        marker,
        true,
        true,
    );
}

/// Drop the final content item from an over-tall candidate line while keeping
/// an earlier textual prefix. This lets a clamp replace a large nested inline
/// (or atomic inline) with its marker when the prefix still fits physically.
fn remove_last_content_item_for_tall_clamp(
    line: &mut LineInfo,
    items_data: &InlineItemsData,
) -> bool {
    let content_indices: Vec<usize> = line
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matches!(
                item.item_type,
                InlineItemType::Text | InlineItemType::AtomicInline
            )
            .then_some(index)
        })
        .collect();
    if content_indices.len() < 2 {
        return false;
    }
    let remove_from = *content_indices.last().expect("at least two content items");
    line.items.truncate(remove_from);
    while line.items.last().is_some_and(|item| {
        matches!(
            item.item_type,
            InlineItemType::OpenTag | InlineItemType::CloseTag | InlineItemType::Control
        )
    }) {
        line.items.pop();
    }
    if let Some(last_text) = line
        .items
        .iter_mut()
        .rev()
        .find(|item| item.item_type == InlineItemType::Text)
    {
        let item = &items_data.items[last_text.item_index];
        let style = &items_data.styles[item.style_index];
        if matches!(
            style.white_space,
            openui_style::WhiteSpace::Normal
                | openui_style::WhiteSpace::Nowrap
                | openui_style::WhiteSpace::PreLine
        ) {
            let text = &items_data.text[last_text.text_range.clone()];
            let visible_len = text.trim_end_matches([' ', '\t', '\n', '\r']).len();
            if visible_len < text.len() {
                let new_end = last_text.text_range.start + visible_len;
                if let Some(shape) = item.shape_result.as_ref() {
                    let item_char_start =
                        byte_to_char_offset(&items_data.text, item.text_range.start);
                    let local_start =
                        byte_to_char_offset(&items_data.text, last_text.text_range.start)
                            - item_char_start;
                    let local_end =
                        byte_to_char_offset(&items_data.text, new_end) - item_char_start;
                    last_text.inline_size =
                        LayoutUnit::from_f32(shape.width_for_range(local_start, local_end));
                }
                last_text.text_range.end = new_end;
            }
        }
    }
    line.used_width = line
        .items
        .iter()
        .fold(LayoutUnit::zero(), |width, item| width + item.inline_size);
    line.hang_width = LayoutUnit::zero();
    line.has_forced_break = false;
    line.is_last_line = false;
    true
}

fn line_has_under_ruby(doc: &Document, line: &LineInfo, items_data: &InlineItemsData) -> bool {
    line.items.iter().any(|result| {
        if result.item_type != InlineItemType::AtomicInline {
            return false;
        }
        let item = &items_data.items[result.item_index];
        !item.node_id.is_none()
            && doc.node(item.node_id).tag == ElementTag::Ruby
            && doc.node(item.node_id).style.ruby_position.is_under()
    })
}

fn fragment_relative_block_end(fragment: &Fragment) -> LayoutUnit {
    fragment
        .children
        .iter()
        .fold(fragment.size.height, |end, child| {
            end.max_of(child.offset.top + fragment_relative_block_end(child))
        })
}

fn last_line_fragment_path(fragments: &[Fragment], doc: &Document) -> Option<Vec<usize>> {
    for (index, fragment) in fragments.iter().enumerate().rev() {
        // A clamp marker belongs to the last retained in-flow line. Positioned
        // descendants and floats may be appended after normal-flow children,
        // but their internal lines do not become clamp-marker candidates for
        // the containing formatting context.
        if fragment.positioned_fragmentation.is_some()
            || (!fragment.node_id.is_none()
                && doc.node(fragment.node_id).style.float != openui_style::Float::None)
        {
            continue;
        }
        if let Some(mut path) = last_line_fragment_path(&fragment.children, doc) {
            path.insert(0, index);
            return Some(path);
        }
        if fragment.node_id.is_none()
            && fragment.kind == FragmentKind::Box
            && fragment.baseline_offset > 0.0
        {
            return Some(vec![index]);
        }
    }
    None
}

fn fragment_at_path_mut<'a>(
    fragments: &'a mut [Fragment],
    path: &[usize],
) -> Option<&'a mut Fragment> {
    let (first, rest) = path.split_first()?;
    let fragment = fragments.get_mut(*first)?;
    if rest.is_empty() {
        Some(fragment)
    } else {
        fragment_at_path_mut(&mut fragment.children, rest)
    }
}

fn last_text_style(fragment: &Fragment) -> Option<ComputedStyle> {
    fragment.children.iter().rev().find_map(|child| {
        if child.kind == FragmentKind::Text {
            child.inherited_style.clone()
        } else {
            last_text_style(child)
        }
    })
}

fn trim_last_text_fragment_for_marker(
    fragment: &mut Fragment,
    target_width: LayoutUnit,
) -> LayoutUnit {
    for child in fragment.children.iter_mut().rev() {
        if child.kind == FragmentKind::Text {
            let Some(text) = child.text_content.as_deref() else {
                continue;
            };
            let Some(shape) = child.shape_result.as_ref() else {
                continue;
            };
            let style = child.inherited_style.as_ref();
            let wrapping_allowed = style.is_none_or(|style| {
                !matches!(
                    style.white_space,
                    openui_style::WhiteSpace::Pre | openui_style::WhiteSpace::Nowrap
                )
            });
            let mut selected = None;
            if wrapping_allowed {
                for (byte, character) in text.char_indices().rev() {
                    if !character.is_whitespace() {
                        continue;
                    }
                    let visible = text[..byte].trim_end_matches(char::is_whitespace);
                    if visible.is_empty() {
                        continue;
                    }
                    let chars = visible.chars().count();
                    let width = LayoutUnit::from_f32(shape.width_for_range(0, chars));
                    if width <= target_width {
                        selected = Some((visible.len(), chars, width));
                        break;
                    }
                }
            }
            let Some((byte_end, char_end, width)) = selected else {
                continue;
            };
            let old_width = child.size.width;
            child.text_content = Some(text[..byte_end].to_string());
            child.shape_result = Some(Arc::new(shape.sub_range(0, char_end)));
            child.size.width = width;
            return (old_width - width).clamp_negative_to_zero();
        }
        let child_target = (target_width - child.offset.left).clamp_negative_to_zero();
        let trimmed = trim_last_text_fragment_for_marker(child, child_target);
        if trimmed > LayoutUnit::zero() {
            child.size.width = (child.size.width - trimmed).clamp_negative_to_zero();
            return trimmed;
        }
    }
    LayoutUnit::zero()
}

/// Attach a marker to the last already-formatted line when the next atomic
/// formatting-context child cannot fit inside an ancestor clamp.
pub(crate) fn append_clamp_marker_to_last_line(
    doc: &Document,
    fragments: &mut [Fragment],
    fallback_style: &ComputedStyle,
    block_ellipsis: &openui_style::BlockEllipsis,
) {
    use openui_style::BlockEllipsis;
    let marker = match block_ellipsis {
        BlockEllipsis::NoEllipsis => return,
        BlockEllipsis::Auto => "\u{2026}",
        BlockEllipsis::String(value) if value.is_empty() => return,
        BlockEllipsis::String(value) => value,
    };
    let Some(path) = last_line_fragment_path(fragments, doc) else {
        return;
    };
    let Some(line) = fragment_at_path_mut(fragments, &path) else {
        return;
    };
    if let Some(marker_index) = line.children.iter().position(|child| {
        child.node_id == NodeId::NONE && child.text_content.as_deref() == Some(marker)
    }) {
        let unbreakable_text_overflow = fallback_style.text_overflow
            == openui_style::TextOverflow::Ellipsis
            && line.children.iter().enumerate().all(|(index, child)| {
                index == marker_index
                    || child.kind != FragmentKind::Text
                    || child
                        .text_content
                        .as_deref()
                        .is_none_or(|text| !text.chars().any(char::is_whitespace))
            });
        if unbreakable_text_overflow {
            let mut marker_fragment = line.children.remove(marker_index);
            line.children
                .retain(|child| child.kind != FragmentKind::Text);
            marker_fragment.offset.left = LayoutUnit::zero();
            line.children.push(marker_fragment);
        }
        return;
    }
    let marker_style = last_text_style(line).unwrap_or_else(|| fallback_style.clone());
    let font = Font::new(style_to_font_description(&marker_style));
    let shaper = TextShaper::new();
    let shape = shaper.shape(marker, &font, openui_text::TextDirection::Ltr);
    let width = LayoutUnit::from_f32(shape.width);
    let metrics = font.font_metrics().copied().unwrap_or_default();
    let height = LayoutUnit::from_f32_ceil(metrics.ascent + metrics.descent);
    let baseline = LayoutUnit::from_f32(line.baseline_offset);
    let top = baseline - LayoutUnit::from_f32_ceil(metrics.ascent);
    let inline_end = line
        .children
        .iter()
        .map(|child| child.offset.left + child.size.width)
        .max()
        .unwrap_or(LayoutUnit::zero());
    if inline_end + width > line.size.width {
        let target = (line.size.width - width).clamp_negative_to_zero();
        trim_last_text_fragment_for_marker(line, target);
    }
    let inline_end = line
        .children
        .iter()
        .map(|child| child.offset.left + child.size.width)
        .max()
        .unwrap_or(LayoutUnit::zero());
    let mut marker_fragment = Fragment::new_text(
        NodeId::NONE,
        PhysicalSize::new(width, height),
        Arc::new(shape),
        marker.to_string(),
    );
    marker_fragment.offset = PhysicalOffset::new(inline_end, top);
    marker_fragment.inherited_style = Some(marker_style.clone());
    marker_fragment.baseline_offset = (baseline - top).to_f32();
    marker_fragment.text_run_orientation = resolve_text_run_orientation(&marker_style, marker);
    line.children.push(marker_fragment);
}

/// Check if a block node has any inline children (text or inline-level elements).
///
/// Used by block_layout to detect when to dispatch to inline layout.
pub fn has_inline_children(doc: &Document, node_id: NodeId) -> bool {
    for child_id in doc.children(node_id) {
        let child = doc.node(child_id);
        if matches!(
            child.pseudo_kind,
            Some(PseudoElementKind::ScrollMarker) | Some(PseudoElementKind::ColumnScrollMarker)
        ) {
            continue;
        }
        // display:none and out-of-flow children don't participate in layout.
        if child.style.display == Display::None || child.style.is_out_of_flow() {
            continue;
        }
        if matches!(
            child.tag,
            openui_dom::ElementTag::Text | openui_dom::ElementTag::Break
        ) {
            return true;
        }
        if child.style.display == Display::Contents && has_inline_children(doc, child_id) {
            return true;
        }
        if child.style.display.is_inline_level() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::ElementTag;
    use openui_style::{Color, Direction, Display, Position, WhiteSpace, WritingMode};

    /// Helper to build a FontMetrics with specific ascent, descent, and line_gap.
    fn test_metrics(ascent: f32, descent: f32, line_gap: f32) -> FontMetrics {
        FontMetrics {
            ascent,
            descent,
            line_gap,
            line_spacing: ascent + descent + line_gap,
            ..FontMetrics::zero()
        }
    }

    #[test]
    fn float_after_forced_break_uses_the_following_line_source_position() {
        let mut doc = Document::new();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.font_size = 5.0;
        doc.append_child(doc.root(), block);

        let leading = doc.create_node(ElementTag::Div);
        doc.node_mut(leading).style.float = Float::Left;
        doc.node_mut(leading).style.width = openui_geometry::Length::px(100.0);
        doc.node_mut(leading).style.height = openui_geometry::Length::px(100.0);
        doc.append_child(block, leading);
        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("H".into());
        doc.append_child(block, text);
        let forced_break = doc.create_node(ElementTag::Break);
        doc.append_child(block, forced_break);
        let following = doc.create_node(ElementTag::Div);
        doc.node_mut(following).style.float = Float::Left;
        doc.node_mut(following).style.width = openui_geometry::Length::px(100.0);
        doc.node_mut(following).style.height = openui_geometry::Length::px(100.0);
        doc.append_child(block, following);

        let (items, floats) = InlineItemsBuilder::collect_with_floats(&doc, block);
        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(200), LayoutUnit::from_i32(200));
        let positions = inline_float_source_positions(
            &doc,
            block,
            &items,
            &floats,
            LayoutUnit::from_i32(200),
            &space,
        );
        let leading_position = positions[&leading];
        let following_position = positions[&following];
        assert_eq!(leading_position.block_offset, LayoutUnit::zero());
        assert!(following_position.line_height > LayoutUnit::zero());
        assert_eq!(
            following_position.block_offset,
            following_position.line_height
        );
        assert_eq!(following_position.preceding_inline_size, LayoutUnit::zero());
    }

    #[test]
    fn positioned_inline_containing_block_keeps_empty_first_fragment() {
        let mut doc = Document::new();
        let target = doc.create_node(ElementTag::Span);
        let mut first = Fragment::new_box(
            target,
            PhysicalSize::new(LayoutUnit::zero(), LayoutUnit::from_i32(16)),
        );
        first.is_inline_box_fragment = true;
        first.offset = PhysicalOffset::new(LayoutUnit::from_i32(40), LayoutUnit::zero());
        let mut last = Fragment::new_box(
            target,
            PhysicalSize::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(16)),
        );
        last.is_inline_box_fragment = true;
        last.offset = PhysicalOffset::new(LayoutUnit::from_i32(5), LayoutUnit::from_i32(16));

        let (offset, size) = inline_containing_block_geometry(
            target,
            &[first, last],
            Direction::Ltr,
            Direction::Ltr.writing_direction(WritingMode::HorizontalTb),
            LayoutUnit::from_i32(100),
            false,
            true,
        )
        .expect("positioned inline geometry");

        assert_eq!(offset.left, LayoutUnit::from_i32(40));
        assert_eq!(offset.top, LayoutUnit::zero());
        assert_eq!(size.width, LayoutUnit::zero());
        assert_eq!(size.height, LayoutUnit::from_i32(32));
    }

    #[test]
    fn fragmented_positioned_inline_ignores_empty_continuation_shell() {
        let mut doc = Document::new();
        let target = doc.create_node(ElementTag::Span);
        let mut continuation = Fragment::new_box(
            target,
            PhysicalSize::new(LayoutUnit::zero(), LayoutUnit::from_i32(16)),
        );
        continuation.is_inline_box_fragment = true;
        continuation.offset = PhysicalOffset::new(LayoutUnit::from_i32(40), LayoutUnit::zero());
        let mut content = Fragment::new_box(
            target,
            PhysicalSize::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(16)),
        );
        content.is_inline_box_fragment = true;
        content.offset = PhysicalOffset::new(LayoutUnit::from_i32(5), LayoutUnit::from_i32(16));
        let mut text = Fragment::new_box(
            target,
            PhysicalSize::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(16)),
        );
        text.kind = FragmentKind::Text;
        content.children.push(text);

        let (offset, size) = inline_containing_block_geometry(
            target,
            &[continuation, content],
            Direction::Ltr,
            Direction::Ltr.writing_direction(WritingMode::HorizontalTb),
            LayoutUnit::from_i32(100),
            false,
            false,
        )
        .expect("fragmented positioned inline geometry");

        assert_eq!(offset.left, LayoutUnit::from_i32(5));
        assert_eq!(offset.top, LayoutUnit::from_i32(16));
        assert_eq!(size.width, LayoutUnit::from_i32(10));
        assert_eq!(size.height, LayoutUnit::from_i32(16));
    }

    #[test]
    fn leading_out_of_flow_distinguishes_generated_box_from_continuation() {
        let mut doc = Document::new();
        let leading_oof = doc.create_node(ElementTag::Span);
        let abs = doc.create_node(ElementTag::Div);
        doc.node_mut(abs).style.position = Position::Absolute;
        doc.append_child(leading_oof, abs);
        let following_text = doc.create_node(ElementTag::Text);
        doc.node_mut(following_text).text = Some("content".to_string());
        doc.append_child(leading_oof, following_text);
        assert!(positioned_inline_starts_with_out_of_flow(&doc, leading_oof));

        let leading_text = doc.create_node(ElementTag::Span);
        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("content".to_string());
        doc.append_child(leading_text, text);
        let later_abs = doc.create_node(ElementTag::Div);
        doc.node_mut(later_abs).style.position = Position::Absolute;
        doc.append_child(leading_text, later_abs);
        assert!(!positioned_inline_starts_with_out_of_flow(
            &doc,
            leading_text
        ));
    }

    #[test]
    fn float_after_ported_preserved_newline_uses_the_following_line_source_position() {
        let mut doc = Document::new();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.font_size = 5.0;
        doc.append_child(doc.root(), block);

        let leading = doc.create_node(ElementTag::Div);
        doc.node_mut(leading).style.float = Float::Left;
        doc.node_mut(leading).style.width = openui_geometry::Length::px(100.0);
        doc.node_mut(leading).style.height = openui_geometry::Length::px(100.0);
        doc.append_child(block, leading);
        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).style.white_space = WhiteSpace::PreLine;
        doc.node_mut(text).text = Some("H\n".into());
        doc.append_child(block, text);
        let following = doc.create_node(ElementTag::Div);
        doc.node_mut(following).style.float = Float::Left;
        doc.node_mut(following).style.width = openui_geometry::Length::px(100.0);
        doc.node_mut(following).style.height = openui_geometry::Length::px(100.0);
        doc.append_child(block, following);

        let (items, floats) = InlineItemsBuilder::collect_with_floats(&doc, block);
        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(200), LayoutUnit::from_i32(200));
        let positions = inline_float_source_positions(
            &doc,
            block,
            &items,
            &floats,
            LayoutUnit::from_i32(200),
            &space,
        );
        let following_position = positions[&following];
        assert!(following_position.line_height > LayoutUnit::zero());
        assert_eq!(
            following_position.block_offset,
            following_position.line_height
        );
        assert_eq!(following_position.preceding_inline_size, LayoutUnit::zero());
    }

    #[test]
    fn float_after_multiple_preserved_newlines_uses_the_final_source_line() {
        let mut doc = Document::new();
        let block = doc.create_node(ElementTag::Div);
        let block_style = doc.node_mut(block).style_mut();
        block_style.display = Display::Block;
        block_style.font_size = 16.0;
        block_style.line_height = openui_style::LineHeight::Length(32.0);
        block_style.white_space = WhiteSpace::PreWrap;
        doc.append_child(doc.root(), block);

        let text = doc.create_node(ElementTag::Text);
        let text_style = doc.node_mut(text).style_mut();
        text_style.font_size = 16.0;
        text_style.line_height = openui_style::LineHeight::Length(32.0);
        text_style.white_space = WhiteSpace::PreWrap;
        doc.node_mut(text).text = Some("Line 1\nLine 2\nLine 3\nLine 4\n".into());
        doc.append_child(block, text);

        let following = doc.create_node(ElementTag::Div);
        doc.node_mut(following).style.float = Float::Left;
        doc.node_mut(following).style.width = openui_geometry::Length::px(300.0);
        doc.append_child(block, following);

        let (items, floats) = InlineItemsBuilder::collect_with_floats(&doc, block);
        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(200), LayoutUnit::from_i32(160));
        let positions = inline_float_source_positions(
            &doc,
            block,
            &items,
            &floats,
            LayoutUnit::from_i32(200),
            &space,
        );
        let following_position = positions[&following];
        assert_eq!(following_position.line_height, LayoutUnit::from_i32(32));
        assert_eq!(following_position.block_offset, LayoutUnit::from_i32(128));
        assert_eq!(following_position.preceding_inline_size, LayoutUnit::zero());
    }

    #[test]
    fn float_after_clearing_break_defers_block_progression_to_clearance() {
        let mut doc = Document::new();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.font_size = 16.0;
        doc.append_child(doc.root(), block);

        let leading = doc.create_node(ElementTag::Div);
        doc.node_mut(leading).style.float = Float::Left;
        doc.node_mut(leading).style.width = openui_geometry::Length::px(16.0);
        doc.node_mut(leading).style.height = openui_geometry::Length::px(10.0);
        doc.append_child(block, leading);
        let forced_break = doc.create_node(ElementTag::Break);
        doc.node_mut(forced_break).style.clear = Clear::Both;
        doc.append_child(block, forced_break);
        let following = doc.create_node(ElementTag::Div);
        doc.node_mut(following).style.float = Float::Left;
        doc.node_mut(following).style.width = openui_geometry::Length::px(16.0);
        doc.node_mut(following).style.height = openui_geometry::Length::px(10.0);
        doc.append_child(block, following);

        let (items, floats) = InlineItemsBuilder::collect_with_floats(&doc, block);
        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(200), LayoutUnit::from_i32(200));
        let positions = inline_float_source_positions(
            &doc,
            block,
            &items,
            &floats,
            LayoutUnit::from_i32(200),
            &space,
        );
        assert_eq!(positions[&following].block_offset, LayoutUnit::zero());
    }

    #[test]
    fn baseline_snapping_distinguishes_normal_and_explicit_line_height() {
        let ascent = 0.8125;
        assert_eq!(
            snap_line_baseline(ascent, &LineHeight::Normal, true),
            LayoutUnit::from_f32_ceil(ascent)
        );
        assert_eq!(
            snap_line_baseline(ascent, &LineHeight::Number(1.0), true),
            LayoutUnit::zero()
        );
        assert_eq!(
            snap_line_baseline(ascent, &LineHeight::Number(1.0), false),
            LayoutUnit::from_f32_ceil(ascent)
        );
    }

    #[test]
    fn inline_decoration_uses_rounded_font_content_bounds() {
        let metrics = test_metrics(9.296_875, 2.343_75, 0.0);
        let (top, bottom) =
            inline_decoration_content_bounds(LayoutUnit::from_i32(8), &metrics, true);
        assert_eq!(top, LayoutUnit::from_i32(-1));
        assert_eq!(bottom, LayoutUnit::from_i32(10));
    }

    #[test]
    fn line_height_metrics_normal() {
        // Normal line height uses int_line_spacing from font metrics.
        // ascent=10, descent=4, gap=2 → int_line_spacing = round(16) = 16
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(
            &metrics,
            &LineHeight::Normal,
            16.0, // font_size
        );
        // leading = 16 - 14 = 2, half_leading = 1, rest = 1
        assert_eq!(m.ascent, 11.0);
        assert_eq!(m.descent, 5.0);
    }

    #[test]
    fn fallback_run_metrics_expand_the_normal_line_box() {
        let primary = test_metrics(12.8, 3.2, 0.0);
        let fallback = test_metrics(14.851_562_5, 3.773_437_5, 0.0);
        let united = unite_text_run_metrics(primary, std::iter::once(&fallback));
        let used = compute_line_height_metrics(&united, &LineHeight::Normal, 16.0);

        assert_eq!(used.line_height, 19.0);
        assert_eq!(used.ascent, 15.0);
        assert_eq!(used.descent, 4.0);
    }

    #[test]
    fn line_height_metrics_number() {
        // line-height: 2.0 doubles line height
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(
            &metrics,
            &LineHeight::Number(2.0),
            16.0, // font_size
        );
        // computed = 16 * 2 = 32, leading = 32 - 14 = 18
        // half_leading = 9, rest = 9
        assert_eq!(m.ascent, 19.0);
        assert_eq!(m.descent, 13.0);
    }

    #[test]
    fn line_height_metrics_length() {
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(&metrics, &LineHeight::Length(24.0), 16.0);
        // leading = 24 - 14 = 10, half = 5, rest = 5
        assert_eq!(m.ascent, 15.0);
        assert_eq!(m.descent, 9.0);
    }

    #[test]
    fn line_height_metrics_percentage() {
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(&metrics, &LineHeight::Percentage(150.0), 16.0);
        // computed = 16 * 150 / 100 = 24, leading = 10
        assert_eq!(m.ascent, 15.0);
        assert_eq!(m.descent, 9.0);
    }

    #[test]
    fn line_height_half_leading_odd() {
        // Odd leading: sub-pixel precision preserved (no floor/ceil rounding)
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(&metrics, &LineHeight::Length(25.0), 16.0);
        // leading = 25 - 14 = 11, half = 5.5, rest = 5.5
        assert_eq!(m.ascent, 15.5);
        assert_eq!(m.descent, 9.5);
    }

    #[test]
    fn baseline_shift_baseline() {
        let shift = compute_baseline_shift(
            &VerticalAlign::Baseline,
            16.0,
            10.0,
            4.0,
            8.0,
            10.0,
            4.0,
            16.0,
        );
        assert_eq!(shift, 0.0);
    }

    #[test]
    fn baseline_shift_sub() {
        let shift =
            compute_baseline_shift(&VerticalAlign::Sub, 16.0, 10.0, 4.0, 8.0, 10.0, 4.0, 16.0);
        assert_eq!(shift, keyword_baseline_shift(16.0, 5));
    }

    #[test]
    fn baseline_shift_super() {
        let shift =
            compute_baseline_shift(&VerticalAlign::Super, 16.0, 10.0, 4.0, 8.0, 10.0, 4.0, 16.0);
        assert_eq!(shift, -keyword_baseline_shift(16.0, 3));
    }

    #[test]
    fn baseline_shift_percentage_uses_element_line_height() {
        // CSS 2.2 §10.8.1: percentage is of the element's own line-height.
        // element_line_height = 40px, 50% => shift = -(40 * 50 / 100) = -20
        let shift = compute_baseline_shift(
            &VerticalAlign::Percentage(50.0),
            16.0,
            10.0,
            4.0,
            8.0,
            10.0,
            4.0,
            40.0,
        );
        assert_eq!(shift, -20.0);
    }

    #[test]
    fn baseline_shift_percentage_with_normal_line_height() {
        // When line-height is normal, element_line_height = font line_spacing.
        // Use line_spacing = 18.0, 50% => shift = -(18 * 50 / 100) = -9
        let shift = compute_baseline_shift(
            &VerticalAlign::Percentage(50.0),
            16.0,
            10.0,
            4.0,
            8.0,
            10.0,
            4.0,
            18.0,
        );
        assert_eq!(shift, -9.0);
    }

    #[test]
    fn text_align_left() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Left, false);
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_right() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Right, false);
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::from_i32(40));
    }

    #[test]
    fn text_align_center() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Center, false);
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        assert_eq!(offset.to_i32(), 20);
    }

    #[test]
    fn text_align_start_ltr() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Start, false);
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_start_rtl() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Start, false);
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Rtl,
            TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::from_i32(40));
    }

    #[test]
    fn text_align_end_ltr() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::End, false);
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::from_i32(40));
    }

    #[test]
    fn text_align_end_rtl() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::End, false);
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Rtl,
            TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_justify_last_line_falls_back() {
        // Justify on the last line falls back to start alignment.
        let mut line = make_test_line_info(100.0, 60.0, TextAlign::Justify, false);
        line.is_last_line = true;
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_right_overflow_extends_toward_line_left() {
        // Right alignment preserves the right edge even when the content is
        // wider than the line, so overflow extends toward physical left.
        let line = make_test_line_info(100.0, 150.0, TextAlign::Right, false);
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::from_i32(-50));
    }

    #[test]
    fn text_align_start_rtl_overflow_extends_toward_inline_end() {
        let line = make_test_line_info(100.0, 150.0, TextAlign::Start, false);
        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Rtl,
            TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::from_i32(-50));
    }

    #[test]
    fn has_inline_children_text_node() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(root, block);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("hello".to_string());
        doc.append_child(block, text);

        assert!(has_inline_children(&doc, block));
    }

    #[test]
    fn has_inline_children_span() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(root, block);

        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.append_child(block, span);

        assert!(has_inline_children(&doc, block));
    }

    #[test]
    fn has_inline_children_block_only() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(root, block);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.append_child(block, child);

        assert!(!has_inline_children(&doc, block));
    }

    #[test]
    fn empty_undecorated_inline_descendants_do_not_create_line_height() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(root, block);

        for _ in 0..3 {
            let span = doc.create_node(ElementTag::Span);
            doc.node_mut(span).style.display = Display::Inline;
            doc.node_mut(span).style.font_size = 40.0;
            doc.append_child(block, span);
        }

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = crate::block::block_layout(&doc, root, &space);
        let block_fragment = &fragment.children[0];
        assert_eq!(block_fragment.size.height, LayoutUnit::zero());
        assert_eq!(block_fragment.children[0].size.height, LayoutUnit::zero());
    }

    // ── Helper ───────────────────────────────────────────────────────────

    fn make_test_line_info(available: f32, used: f32, align: TextAlign, is_last: bool) -> LineInfo {
        let mut info = LineInfo::new(LayoutUnit::from_f32(available));
        info.used_width = LayoutUnit::from_f32(used);
        info.text_align = align;
        info.is_last_line = is_last;
        info
    }

    #[test]
    fn ellipsis_uses_block_font_for_measurement() {
        // Verify that apply_text_overflow_ellipsis measures the ellipsis using
        // the block_style's font (consistent with create_line_box), not the
        // first visible item's font.
        let block_style = ComputedStyle::default();
        let block_font_desc = style_to_font_description(&block_style);
        let block_font = Font::new(block_font_desc);
        let shaper = TextShaper::new();

        // Measure the ellipsis width with the block font.
        let ellipsis_sr = shaper.shape("\u{2026}", &block_font, openui_text::TextDirection::Ltr);
        let expected_width = LayoutUnit::from_f32(ellipsis_sr.width);

        // The ellipsis width should be a positive, reasonable value.
        assert!(
            expected_width > LayoutUnit::zero(),
            "Ellipsis measured with block font should have positive width",
        );

        // Create a minimal overflowing line with no items (edge case).
        let mut line_info = LineInfo::new(LayoutUnit::from_f32(100.0));
        line_info.used_width = LayoutUnit::from_f32(150.0);
        let items_data = InlineItemsData {
            text: String::new(),
            items: Vec::new(),
            styles: Vec::new(),
            oof_children: Vec::new(),
            block_in_inline: Vec::new(),
        };

        // Should not panic even with empty items.
        apply_text_overflow_ellipsis(
            &mut line_info,
            LayoutUnit::from_f32(100.0),
            &items_data,
            &block_style,
        );

        // Line should be flagged with ellipsis.
        assert!(line_info.has_ellipsis);
    }

    // ── Issue 2: Half-leading-adjusted metrics for baseline shift ────

    #[test]
    fn text_top_uses_half_leading_adjusted_metrics() {
        // With a large line-height, text-top should use half-leading-adjusted
        // ascent (not raw font ascent). compute_baseline_shift with text-top
        // returns item_ascent - parent_ascent. When we pass half-leading-
        // adjusted metrics, the shift is different from raw metrics.
        //
        // Raw font: ascent=10, descent=4 (total=14)
        // line-height: 24px → leading=10, half=5
        // half-leading ascent = 10 + 5 = 15
        //
        // text-top shift = item_ascent - parent_ascent
        // With raw: 10 - 10 = 0
        // With half-leading: 15 - 10 = 5
        let shift_with_half_leading = compute_baseline_shift(
            &VerticalAlign::TextTop,
            16.0,
            10.0, // parent_ascent
            4.0,  // parent_descent
            8.0,  // parent_x_height
            15.0, // item_ascent (half-leading adjusted)
            9.0,  // item_descent (half-leading adjusted)
            24.0, // element_line_height
        );
        // text-top: item_ascent - parent_ascent = 15 - 10 = 5
        assert_eq!(shift_with_half_leading, 5.0);

        let shift_with_raw = compute_baseline_shift(
            &VerticalAlign::TextTop,
            16.0,
            10.0,
            4.0,
            8.0,
            10.0, // raw ascent
            4.0,  // raw descent
            24.0,
        );
        // With raw metrics: 10 - 10 = 0 (old behavior, now different from above)
        assert_eq!(shift_with_raw, 0.0);
        assert_ne!(
            shift_with_half_leading, shift_with_raw,
            "Half-leading metrics should produce different shift than raw metrics"
        );
    }

    // ── Issue 5: RTL ellipsis truncates from left ────────────────────

    #[test]
    fn rtl_ellipsis_truncates_from_left() {
        // For RTL, ellipsis should be placed at the start (left) and
        // content should be truncated from the left side.
        let mut block_style = ComputedStyle::default();
        block_style.direction = Direction::Rtl;

        let mut line_info = LineInfo::new(LayoutUnit::from_f32(100.0));
        line_info.used_width = LayoutUnit::from_f32(150.0);

        let items_data = InlineItemsData {
            text: String::new(),
            items: Vec::new(),
            styles: Vec::new(),
            oof_children: Vec::new(),
            block_in_inline: Vec::new(),
        };

        apply_text_overflow_ellipsis(
            &mut line_info,
            LayoutUnit::from_f32(100.0),
            &items_data,
            &block_style,
        );

        assert!(line_info.has_ellipsis);
        assert!(
            line_info.ellipsis_at_start,
            "RTL ellipsis should be placed at start (left)"
        );
    }

    #[test]
    fn ltr_ellipsis_not_at_start() {
        // For LTR, ellipsis_at_start should be false.
        let block_style = ComputedStyle::default();

        let mut line_info = LineInfo::new(LayoutUnit::from_f32(100.0));
        line_info.used_width = LayoutUnit::from_f32(150.0);

        let items_data = InlineItemsData {
            text: String::new(),
            items: Vec::new(),
            styles: Vec::new(),
            oof_children: Vec::new(),
            block_in_inline: Vec::new(),
        };

        apply_text_overflow_ellipsis(
            &mut line_info,
            LayoutUnit::from_f32(100.0),
            &items_data,
            &block_style,
        );

        assert!(line_info.has_ellipsis);
        assert!(
            !line_info.ellipsis_at_start,
            "LTR ellipsis should not be at start"
        );
    }

    // ── SP11 Round 11 Issue 4: baseline_offset stored on text fragments ──

    #[test]
    fn text_fragment_has_nonzero_baseline_offset() {
        // Layout a simple text node and verify that the resulting text
        // fragment has a positive baseline_offset (ascent-based).
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = openui_geometry::Length::px(200.0);
        doc.append_child(vp, div);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("Hello".to_string());
        doc.append_child(div, text);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];

        // Line box → text fragment
        assert!(!div_frag.children.is_empty(), "Should have line boxes");
        let line = &div_frag.children[0];
        assert!(!line.children.is_empty(), "Line should have text fragments");

        let text_frag = &line.children[0];
        assert!(
            text_frag.baseline_offset > 0.0,
            "Text fragment baseline_offset should be positive (ascent), got {}",
            text_frag.baseline_offset,
        );
    }

    // ── SP11 Round 15 Issue 1: vertical-align uses parent inline metrics ──

    #[test]
    fn vertical_align_text_top_uses_parent_inline_metrics() {
        // Verify that compute_baseline_shift correctly receives parent inline
        // metrics. With a parent ascent of 25.0, text-top should produce
        // item_ascent - parent_ascent = 10 - 25 = -15.
        // With block metrics (ascent=12), it would be 10 - 12 = -2.
        let shift_parent = compute_baseline_shift(
            &VerticalAlign::TextTop,
            12.0,
            25.0, // parent inline ascent (larger font)
            8.0,  // parent inline descent
            10.0, // parent inline x_height
            10.0, // item ascent
            3.0,  // item descent
            14.0, // element_line_height
        );
        // text-top: item_ascent - parent_ascent = 10 - 25 = -15
        assert_eq!(shift_parent, -15.0);

        let shift_block = compute_baseline_shift(
            &VerticalAlign::TextTop,
            12.0,
            12.0, // block ascent (smaller)
            4.0,
            6.0,
            10.0,
            3.0,
            14.0,
        );
        // text-top: item_ascent - parent_ascent = 10 - 12 = -2
        assert_eq!(shift_block, -2.0);

        // The shift differs when using parent inline vs block metrics.
        assert_ne!(
            shift_parent, shift_block,
            "text-top shift should differ between parent inline (30px) and block (16px)"
        );
    }

    #[test]
    fn vertical_align_middle_uses_parent_x_height() {
        // Verify that middle alignment uses the parent's x_height.
        // With parent x_height=10: (item_ascent - item_descent)/2 - 10/2
        let shift = compute_baseline_shift(
            &VerticalAlign::Middle,
            12.0,
            20.0, // parent ascent
            5.0,  // parent descent
            10.0, // parent x_height
            8.0,  // item ascent
            3.0,  // item descent
            14.0,
        );
        // middle: (8 - 3)/2 - 10/2 = 2.5 - 5.0 = -2.5
        assert_eq!(shift, -2.5);
    }

    #[test]
    fn inline_metrics_stack_affects_layout() {
        // Build: <div font-size=16><span font-size=30><text font-size=12 v-align=text-top>X</text></span></div>
        // The text-top aligned text should be positioned based on the 30px
        // span's metrics, not the 16px block's metrics.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        doc.append_child(vp, div);

        // Outer span with 30px font
        let outer_span = doc.create_node(ElementTag::Span);
        doc.node_mut(outer_span).style.display = Display::Inline;
        doc.node_mut(outer_span).style.font_size = 30.0;
        doc.append_child(div, outer_span);

        let outer_text = doc.create_node(ElementTag::Text);
        doc.node_mut(outer_text).text = Some("A".to_string());
        doc.node_mut(outer_text).style.font_size = 30.0;
        doc.append_child(outer_span, outer_text);

        // Text with text-top inside the outer span (no intermediate span)
        let inner_text = doc.create_node(ElementTag::Text);
        doc.node_mut(inner_text).text = Some("X".to_string());
        doc.node_mut(inner_text).style.font_size = 12.0;
        doc.node_mut(inner_text).style.vertical_align = VerticalAlign::TextTop;
        doc.append_child(outer_span, inner_text);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        assert!(!div_frag.children.is_empty(), "Should have line boxes");
        let line = &div_frag.children[0];

        fn collect_text_tops(
            fragment: &Fragment,
            parent_top: LayoutUnit,
            tops: &mut Vec<LayoutUnit>,
        ) {
            let top = parent_top + fragment.offset.top;
            if fragment.kind == FragmentKind::Text {
                tops.push(top);
            }
            for child in &fragment.children {
                collect_text_tops(child, top, tops);
            }
        }

        let mut text_tops = Vec::new();
        collect_text_tops(line, LayoutUnit::zero(), &mut text_tops);
        assert!(
            text_tops.len() >= 2,
            "Line should have at least 2 descendant text fragments, got {}",
            text_tops.len(),
        );

        // text-top: the top of the inner item aligns with the parent inline's
        // text top. Since the parent inline has 30px font, the inner item's top
        // should be near the outer item's top.
        let outer_top = text_tops[0].to_f32();
        let inner_top = text_tops[1].to_f32();

        // With block metrics (16px), text-top would align to the block's ascent,
        // producing a larger offset difference. With parent inline metrics (30px),
        // the inner text's top should be closer to the outer text's top.
        let diff = (inner_top - outer_top).abs();
        assert!(
            diff < 5.0,
            "text-top inner text should align near outer span's text top; outer_top={}, inner_top={}, diff={}",
            outer_top, inner_top, diff,
        );
    }

    // ── Issue 4 (R24): detect_cjk_content ──────────────────────────────

    #[test]
    fn is_cjk_character_detects_han_ideographs() {
        assert!(is_cjk_character('\u{4E00}')); // CJK Unified start
        assert!(is_cjk_character('\u{9FFF}')); // CJK Unified end
        assert!(is_cjk_character('\u{5927}')); // 大
        assert!(!is_cjk_character('A'));
        assert!(!is_cjk_character(' '));
    }

    #[test]
    fn is_cjk_character_detects_kana_and_hangul() {
        assert!(is_cjk_character('\u{3042}')); // Hiragana あ
        assert!(is_cjk_character('\u{30A2}')); // Katakana ア
        assert!(is_cjk_character('\u{AC00}')); // Hangul 가
        assert!(!is_cjk_character('Z'));
    }

    // ── Issue 2 (R26): atomic fragment uses block_layout width ──────────

    #[test]
    fn atomic_inline_block_layout_width_preserved() {
        // An inline-block child with width:80px inside a 400px container.
        // The fragment width should come from block_layout (80px), not from
        // the line breaker's item_result.inline_size which previously
        // overwrote the block_layout result.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        doc.append_child(vp, div);

        let inline_block = doc.create_node(ElementTag::Div);
        doc.node_mut(inline_block).style.display = Display::InlineBlock;
        doc.node_mut(inline_block).style.width = openui_geometry::Length::px(80.0);
        doc.node_mut(inline_block).style.height = openui_geometry::Length::px(30.0);
        doc.append_child(div, inline_block);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(400), LayoutUnit::from_i32(600));
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        assert!(!div_frag.children.is_empty(), "should have line boxes");
        let line = &div_frag.children[0];
        assert!(
            !line.children.is_empty(),
            "line should have atomic inline child"
        );
        let atomic = &line.children[0];
        // Width should be 80px from block_layout, not overridden.
        assert!(
            (atomic.size.width.to_f32() - 80.0).abs() < 1.0,
            "atomic inline width should be ~80px from block_layout, got {}",
            atomic.size.width.to_f32(),
        );
    }

    #[test]
    fn atomic_inline_block_layout_width_not_overridden_with_border() {
        // An inline-block with width:60px and 5px border on each side.
        // block_layout should produce border-box width of 70px.
        // The fragment should preserve that, not override it.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        doc.append_child(vp, div);

        let inline_block = doc.create_node(ElementTag::Div);
        doc.node_mut(inline_block).style.display = Display::InlineBlock;
        doc.node_mut(inline_block).style.width = openui_geometry::Length::px(60.0);
        doc.node_mut(inline_block).style.height = openui_geometry::Length::px(30.0);
        doc.node_mut(inline_block).style.border_left_width = 5;
        doc.node_mut(inline_block).style.border_right_width = 5;
        doc.node_mut(inline_block).style.border_left_style = openui_style::BorderStyle::Solid;
        doc.node_mut(inline_block).style.border_right_style = openui_style::BorderStyle::Solid;
        doc.append_child(div, inline_block);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(400), LayoutUnit::from_i32(600));
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        let line = &div_frag.children[0];
        let atomic = &line.children[0];
        // Width should be content(60) + border(10) = 70.
        assert!(
            (atomic.size.width.to_f32() - 70.0).abs() < 1.0,
            "atomic inline width should be ~70 (60+border 10), got {}",
            atomic.size.width.to_f32(),
        );
    }

    // ── Issue 3 (R26): percentage heights on atomic inlines resolve ──────

    #[test]
    fn atomic_inline_percentage_height_resolves_against_container() {
        // Container has fixed height 200px. Inline-block has height:50%.
        // Should resolve to 100px, not auto/0.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        doc.node_mut(div).style.height = openui_geometry::Length::px(200.0);
        doc.append_child(vp, div);

        let inline_block = doc.create_node(ElementTag::Div);
        doc.node_mut(inline_block).style.display = Display::InlineBlock;
        doc.node_mut(inline_block).style.width = openui_geometry::Length::px(50.0);
        doc.node_mut(inline_block).style.height = openui_geometry::Length::percent(50.0);
        doc.append_child(div, inline_block);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(400), LayoutUnit::from_i32(600));
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        assert!(!div_frag.children.is_empty(), "should have line boxes");
        let line = &div_frag.children[0];
        assert!(
            !line.children.is_empty(),
            "line should have the inline-block"
        );
        let atomic = &line.children[0];
        // Height should be 50% of 200 = 100px.
        let h = atomic.size.height.to_f32();
        assert!(
            h > 50.0 && h <= 100.0 + 1.0,
            "atomic inline height should be ~100px (50% of 200), got {}",
            h,
        );
    }

    #[test]
    fn atomic_inline_percentage_height_indefinite_container_uses_auto() {
        // Container has auto height (indefinite). Inline-block has height:50%.
        // Should behave as auto (not crash, and produce some reasonable height).
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        // height: auto (default)
        doc.append_child(vp, div);

        let inline_block = doc.create_node(ElementTag::Div);
        doc.node_mut(inline_block).style.display = Display::InlineBlock;
        doc.node_mut(inline_block).style.width = openui_geometry::Length::px(50.0);
        doc.node_mut(inline_block).style.height = openui_geometry::Length::percent(50.0);
        doc.append_child(div, inline_block);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(400), LayoutUnit::max());
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        assert!(!div_frag.children.is_empty(), "should have line boxes");
        // Just verify it doesn't crash/panic with indefinite containing block.
        let line = &div_frag.children[0];
        assert!(
            !line.children.is_empty(),
            "line should have the inline-block"
        );
    }

    // ── Hang width + alignment tests ─────────────────────────────────

    #[test]
    fn text_align_right_with_hang_width() {
        // When hang_width > 0, the remaining space for alignment should be
        // based on used_width (which already excludes hung spaces), not on
        // used_width + hang_width.
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::Right, false);
        // Simulate 20px of hung trailing spaces: used_width is 100 (after
        // subtraction), and hang_width records what was subtracted.
        line.hang_width = LayoutUnit::from_f32(20.0);

        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(200),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        // remaining = 200 - 100 = 100 → right-align offset = 100
        assert_eq!(
            offset,
            LayoutUnit::from_i32(100),
            "right-align should use used_width (not used_width + hang_width)"
        );
    }

    #[test]
    fn text_align_center_with_hang_width() {
        let mut line = make_test_line_info(200.0, 80.0, TextAlign::Center, false);
        line.hang_width = LayoutUnit::from_f32(10.0);

        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(200),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        // remaining = 200 - 80 = 120 → center offset = 60
        assert_eq!(
            offset.to_i32(),
            60,
            "center-align should use used_width (excluding hang_width)"
        );
    }

    #[test]
    fn text_align_left_unaffected_by_hang_width() {
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::Left, false);
        line.hang_width = LayoutUnit::from_f32(30.0);

        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(200),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        assert_eq!(
            offset,
            LayoutUnit::zero(),
            "left-align offset is always 0 regardless of hang_width"
        );
    }

    #[test]
    fn text_align_start_rtl_with_hang_width() {
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::Start, false);
        line.hang_width = LayoutUnit::from_f32(15.0);

        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(200),
            Direction::Rtl,
            TextAlignLast::Auto,
        );
        // RTL start = right-align: remaining = 200 - 100 = 100
        assert_eq!(
            offset,
            LayoutUnit::from_i32(100),
            "RTL start-align with hang_width"
        );
    }

    #[test]
    fn text_align_end_ltr_with_hang_width() {
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::End, false);
        line.hang_width = LayoutUnit::from_f32(25.0);

        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(200),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        // LTR end = right-align: remaining = 200 - 100 = 100
        assert_eq!(
            offset,
            LayoutUnit::from_i32(100),
            "LTR end-align with hang_width"
        );
    }

    #[test]
    fn text_align_justify_with_hang_width() {
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::Justify, false);
        line.hang_width = LayoutUnit::from_f32(20.0);

        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(200),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        // Justify offset is always 0 (justification done by space expansion).
        assert_eq!(
            offset,
            LayoutUnit::zero(),
            "justify offset is 0 regardless of hang_width"
        );
    }

    #[test]
    fn text_align_last_center_with_hang_width() {
        let mut line = make_test_line_info(200.0, 80.0, TextAlign::Justify, false);
        line.is_last_line = true;
        line.hang_width = LayoutUnit::from_f32(10.0);

        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(200),
            Direction::Ltr,
            TextAlignLast::Center,
        );
        // Last line with text-align-last: center; remaining = 200 - 80 = 120; offset = 60
        assert_eq!(
            offset.to_i32(),
            60,
            "text-align-last: center with hang_width on last line"
        );
    }

    #[test]
    fn hang_width_does_not_cause_overflow_alignment() {
        // If used_width < available but used_width + hang_width > available,
        // alignment should still work (hang_width is excluded from overflow check).
        let mut line = make_test_line_info(100.0, 90.0, TextAlign::Right, false);
        line.hang_width = LayoutUnit::from_f32(20.0);

        let offset = compute_text_align_offset(
            &line,
            LayoutUnit::from_i32(100),
            Direction::Ltr,
            TextAlignLast::Auto,
        );
        // remaining = 100 - 90 = 10 → still positive → offset = 10
        assert_eq!(
            offset,
            LayoutUnit::from_i32(10),
            "hang_width overflow should not affect alignment"
        );
    }

    #[test]
    fn decorated_inline_produces_a_real_box_fragment() {
        let mut doc = Document::new();
        let vp = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.width = openui_geometry::Length::px(200.0);
        doc.append_child(vp, block);

        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.node_mut(span).style.padding_left = openui_geometry::Length::px(4.0);
        doc.node_mut(span).style.padding_right = openui_geometry::Length::px(6.0);
        doc.node_mut(span).style.background_color = Color::RED;
        doc.append_child(block, span);
        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("XX".to_string());
        doc.append_child(span, text);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let inline = &fragment.children[0].children[0].children[0];
        assert_eq!(inline.node_id, span);
        assert!(inline.is_inline_box_fragment);
        assert!(inline.is_first_for_node && inline.is_last_for_node);
        assert_eq!(inline.padding.left, LayoutUnit::from_i32(4));
        assert_eq!(inline.padding.right, LayoutUnit::from_i32(6));
        assert!(!inline.children.is_empty());
    }

    #[test]
    fn wrapped_inline_fragments_have_first_and_last_continuations() {
        let mut doc = Document::new();
        let vp = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.width = openui_geometry::Length::px(18.0);
        doc.append_child(vp, block);
        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.node_mut(span).style.background_color = Color::BLUE;
        doc.append_child(block, span);
        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("X X X".to_string());
        doc.append_child(span, text);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let mut continuations = Vec::new();
        for line in &fragment.children[0].children {
            for child in &line.children {
                if child.node_id == span && child.is_inline_box_fragment {
                    continuations.push(child);
                }
            }
        }
        assert!(continuations.len() >= 2);
        assert!(continuations.first().unwrap().is_first_for_node);
        assert!(!continuations.first().unwrap().is_last_for_node);
        assert!(!continuations.last().unwrap().is_first_for_node);
        assert!(continuations.last().unwrap().is_last_for_node);
    }

    #[test]
    fn bidi_reordering_splits_and_rebalances_inline_fragments() {
        let mut doc = Document::new();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(doc.root(), block);
        for content in ["\u{202e}a\u{202d}bc", "d\u{202e}e\u{202d}f"] {
            let span = doc.create_node(ElementTag::Span);
            doc.node_mut(span).style.display = Display::Inline;
            doc.append_child(block, span);
            let text = doc.create_node(ElementTag::Text);
            doc.node_mut(text).text = Some(content.to_string());
            doc.append_child(span, text);
        }

        let mut data = InlineItemsBuilder::collect(&doc, block);
        data.apply_bidi(openui_text::TextDirection::Ltr);
        data.shape_text();
        let mut breaker = LineBreaker::new(&data, LayoutUnit::from_i32(800));
        let mut line = breaker.next_line(LayoutUnit::from_i32(800)).unwrap();
        bidi_reorder_line(&mut line.items, &data);
        assert_eq!(
            line.items
                .iter()
                .filter(|result| result.item_type == InlineItemType::Text)
                .map(|result| data.text[result.text_range.clone()].to_string())
                .collect::<Vec<_>>(),
            vec!["\u{202e}", "bc", "d\u{202e}", "f", "e\u{202d}", "a\u{202d}"]
        );
        let boundary_types: Vec<_> = line
            .items
            .iter()
            .filter(|result| {
                matches!(
                    result.item_type,
                    InlineItemType::OpenTag | InlineItemType::CloseTag
                )
            })
            .map(|result| result.item_type)
            .collect();
        assert_eq!(
            boundary_types,
            vec![
                InlineItemType::OpenTag,
                InlineItemType::CloseTag,
                InlineItemType::OpenTag,
                InlineItemType::CloseTag,
                InlineItemType::OpenTag,
                InlineItemType::CloseTag,
            ]
        );
    }
}
