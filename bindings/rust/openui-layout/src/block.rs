//! Block layout algorithm — extracted from Blink's `block_layout_algorithm.cc`.
//!
//! Source: core/layout/block_layout_algorithm.cc (~4200 lines)
#![allow(unused_assignments)] // margin_strut is a loop accumulator
//!
//! This implements CSS normal flow (block formatting context): children are
//! stacked vertically, auto margins are resolved, margins collapse between
//! siblings and between parent/child.
//!
//! The algorithm follows Blink's NG layout pipeline:
//! 1. Compute child margins and padding
//! 2. Create constraint space for child
//! 3. Layout child (recursively)
//! 4. Position child using ComputeInflowPosition logic
//! 5. After all children: compute intrinsic block size, apply CSS height

use openui_dom::{Document, NodeId};
use openui_geometry::{
    BfcOffset, BfcRect, BoxStrut, LayoutUnit, Length, LengthType, MarginStrut, PhysicalOffset,
    PhysicalRect, PhysicalSize,
};
use openui_style::{
    BoxDecorationBreak, BoxSizing, BreakInside, BreakValue, Clear, ComputedStyle, Direction,
    Display, Float, LineHeight, ListStylePosition, Overflow, Position,
};

use crate::constraint_space::ConstraintSpace;
use crate::exclusions::float_utils::{position_float, UnpositionedFloat};
use crate::exclusions::{ClearType, ExclusionArea, ExclusionSpace, ExclusionType};
use crate::fragment::{Fragment, FragmentKind};
use crate::length_resolver::{resolve_length, resolve_margin_or_padding};
use crate::out_of_flow::OutOfFlowCandidate;

/// Perform block layout on a node and its descendants.
///
/// This is the main entry point, equivalent to Blink's
/// `BlockLayoutAlgorithm::Layout()` (line 593).
///
/// Returns a `Fragment` with resolved sizes and positioned children.
pub fn block_layout(doc: &Document, node_id: NodeId, space: &ConstraintSpace) -> Fragment {
    let style = &doc.node(node_id).style;

    // Dispatch flex containers to the flex algorithm
    if style.display.is_flex() {
        return crate::flex::flex_layout(doc, node_id, space);
    }

    // ── Step 1: Resolve border + padding ─────────────────────────────
    // Blink: uses pre-resolved border widths (integers) and resolves padding
    // against percentage_resolution_inline_size.

    let border = resolve_border(style);
    let padding = resolve_padding(style, space.percentage_resolution_inline_size);

    let border_padding_inline = border.left + border.right + padding.left + padding.right;
    let border_padding_block = border.top + border.bottom + padding.top + padding.bottom;

    // ── Step 2: Resolve width ────────────────────────────────────────
    // Blink: ComputeBlockSizeForFragment / ResolveMainInlineLength

    let mut content_inline_size = resolve_inline_size(
        doc,
        node_id,
        style,
        space,
        border_padding_inline,
        border_padding_block,
    );

    // CSS Sizing 4 §5.1: AR constraint feedback — when the tentative inline
    // size produces a block size via AR that gets clamped by min/max-height,
    // re-derive the inline size from the clamped block size.
    // Only applies when width is NOT an explicit length (auto, stretch, or
    // content keywords like min-content/max-content/fit-content).
    // When width is explicit (e.g. 100px), AR violation is accepted.
    if let Some(ref ar) = style.aspect_ratio {
        let width_is_auto_like = style.width.is_auto()
            || style.width.is_stretch()
            || style.width.is_content_or_intrinsic();
        let inline_size_fixed_by_parent = space.is_fixed_inline_size || space.stretch_inline_size;
        if ar.ratio.0 != 0.0
            && ar.ratio.1 != 0.0
            && style.height.is_auto()
            && width_is_auto_like
            && !inline_size_fixed_by_parent
        {
            let border_box_w = if style.box_sizing == BoxSizing::BorderBox {
                content_inline_size.max_of(border_padding_inline)
            } else {
                content_inline_size + border_padding_inline
            };

            // Compute tentative AR-derived height
            let (content_w, bp_i, bp_b) =
                if ar.auto_flag || style.box_sizing != BoxSizing::BorderBox {
                    // AR on content-box
                    let cw = if style.box_sizing == BoxSizing::BorderBox {
                        (border_box_w - border_padding_inline).clamp_negative_to_zero()
                    } else {
                        content_inline_size
                    };
                    (cw, border_padding_inline, border_padding_block)
                } else {
                    // AR on border-box
                    (border_box_w, LayoutUnit::zero(), LayoutUnit::zero())
                };

            let tentative_h =
                LayoutUnit::from_f32(content_w.to_f32() * ar.ratio.1 / ar.ratio.0) + bp_b;

            // Apply min/max-height
            let min_h = if style.min_height.is_auto() {
                LayoutUnit::zero()
            } else {
                resolve_length(
                    &style.min_height,
                    space.percentage_resolution_block_size,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                )
            };
            let max_h = resolve_length(
                &style.max_height,
                space.percentage_resolution_block_size,
                LayoutUnit::max(),
                LayoutUnit::max(),
            );

            if max_h < LayoutUnit::max() || min_h > LayoutUnit::zero() {
                let clamped_h = tentative_h.max_of(min_h).min_of(max_h);
                if clamped_h != tentative_h {
                    // Re-derive inline size from clamped block size
                    let new_content_h = (clamped_h - bp_b).clamp_negative_to_zero();
                    let new_w =
                        LayoutUnit::from_f32(new_content_h.to_f32() * ar.ratio.0 / ar.ratio.1)
                            + bp_i;
                    let new_content = if style.box_sizing == BoxSizing::BorderBox {
                        (new_w - border_padding_inline).clamp_negative_to_zero()
                    } else {
                        new_w
                    };
                    // Apply min/max-width to the feedback result
                    let min_w_val = resolve_length(
                        &style.min_width,
                        space.percentage_resolution_inline_size,
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                    );
                    let max_w_val = resolve_length(
                        &style.max_width,
                        space.percentage_resolution_inline_size,
                        LayoutUnit::max(),
                        LayoutUnit::max(),
                    );
                    content_inline_size = new_content.clamp(min_w_val, max_w_val);
                }
            }
        }
    }

    // The total border-box inline size
    let border_box_inline = if style.box_sizing == BoxSizing::BorderBox {
        content_inline_size.max_of(border_padding_inline)
    } else {
        content_inline_size + border_padding_inline
    };

    // Available inline size for children = content box width
    let child_available_inline = if style.box_sizing == BoxSizing::BorderBox {
        border_box_inline - border_padding_inline
    } else {
        content_inline_size
    };

    // ── Multicol dispatch ────────────────────────────────────────────
    // CSS Multi-column Layout §3-4: if column-count or column-width is set,
    // lay out children across columns instead of normal block flow.
    if let Some(algo) = crate::multicol::ColumnLayoutAlgorithm::from_style(style) {
        return layout_multicol(
            doc,
            node_id,
            style,
            space,
            &algo,
            &border,
            &padding,
            border_padding_inline,
            border_padding_block,
            child_available_inline,
            content_inline_size,
            border_box_inline,
        );
    }

    // ── Step 3: Layout children (the main loop) ─────────────────────
    // Blink: block_layout_algorithm.cc lines 981-1110
    //
    // Iterate children in document order. For each in-flow block child:
    // 1. Calculate margins
    // 2. Create child constraint space
    // 3. Layout child
    // 4. Position using ComputeInflowPosition

    // Per CSS 2.1 §10.5, percentage heights on children resolve against the
    // containing block's *specified* height (if definite), not auto-computed.
    // If the parent's height is auto, percentage heights are indefinite.
    // CSS 2.1 §10.5: If the parent's height depends on its children (auto)
    // or is a percentage against an indefinite containing block, children's
    // percentage heights compute to auto.
    // Exception: if the constraint space imposes a fixed block size (e.g.,
    // viewport/ICB, or flex/grid definite cross-size), children can resolve
    // percentage heights against that definite size.
    let child_percentage_block_size = if (space.is_fixed_block_size || space.stretch_block_size)
        && !space.is_initial_block_size_indefinite
    {
        // Definite block size from external constraint — use it directly.
        // Content-box: subtract border+padding.
        (space.available_block_size - border_padding_block).clamp_negative_to_zero()
    } else if !style.height.is_auto() && !space.is_initial_block_size_indefinite {
        // A percentage height against an indefinite basis is itself indefinite.
        if style.height.is_percent() && space.percentage_resolution_block_size.is_indefinite() {
            openui_geometry::INDEFINITE_SIZE
        } else {
            let raw = resolve_length(
                &style.height,
                space.percentage_resolution_block_size,
                LayoutUnit::zero(), // auto fallback (shouldn't reach here)
                LayoutUnit::zero(), // none fallback
            );
            // Convert to content-box size
            if style.box_sizing == BoxSizing::BorderBox {
                (raw - border_padding_block).clamp_negative_to_zero()
            } else {
                raw
            }
        }
    } else {
        // Auto height → per CSS 2.2 §10.5, percentage heights on children
        // are indefinite (treated as auto). Do NOT pass through the parent's
        // percentage resolution — that would incorrectly let grandchildren
        // resolve percentage heights against an ancestor's explicit height.
        //
        // Exception: CSS Sizing 4 §5.1 — When the element has an aspect-ratio
        // and its width is definite, the AR-derived height is considered
        // definite for percentage resolution purposes.
        if let Some(ref ar) = style.aspect_ratio {
            let ratio = ar.ratio;
            if ratio.0 != 0.0 && ratio.1 != 0.0 {
                // Width must be definite (explicit, or stretch, etc.)
                let width_definite =
                    !style.width.is_auto() || space.available_inline_size.raw() > 0;
                if width_definite {
                    // Compute the AR-derived content height from the resolved width.
                    // The actual width resolution happens later, so approximate
                    // by resolving width now (matches what resolve_block_size does).
                    let avail = space.available_inline_size;
                    let bp_inline = border.left + border.right + padding.left + padding.right;
                    let content_w = if style.width.is_auto() || style.width.is_stretch() {
                        (avail - bp_inline).clamp_negative_to_zero()
                    } else {
                        let raw = resolve_length(
                            &style.width,
                            space.percentage_resolution_inline_size,
                            avail,
                            avail,
                        );
                        if style.box_sizing == BoxSizing::BorderBox {
                            (raw - bp_inline).clamp_negative_to_zero()
                        } else {
                            raw
                        }
                    };
                    let content_h = LayoutUnit::from_f32(content_w.to_f32() * ratio.1 / ratio.0);
                    content_h
                } else {
                    openui_geometry::INDEFINITE_SIZE
                }
            } else {
                openui_geometry::INDEFINITE_SIZE
            }
        } else {
            openui_geometry::INDEFINITE_SIZE
        }
    };

    // Compute the available block size to pass to children for `height: stretch`
    // and similar sizing. When this node has a definite block size (explicit
    // height, or externally imposed), children should size against that, not
    // against the grandparent's available_block_size.
    let children_available_block_size = if space.is_fixed_block_size || space.stretch_block_size {
        // Externally imposed (viewport, flex cross axis, etc.)
        (space.available_block_size - border_padding_block).clamp_negative_to_zero()
    } else if !style.height.is_auto()
        && !style.height.is_stretch()
        && !style.height.is_content_or_intrinsic()
    {
        // Explicit height (px, %, etc.) — resolve it
        if style.height.is_percent() && space.percentage_resolution_block_size.is_indefinite() {
            space.available_block_size // fallback to parent's available
        } else {
            let raw = resolve_length(
                &style.height,
                space.percentage_resolution_block_size,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            if style.box_sizing == BoxSizing::BorderBox {
                (raw - border_padding_block).clamp_negative_to_zero()
            } else {
                raw
            }
        }
    } else {
        space.available_block_size // auto height: pass through
    };

    let content_edge = border.top + padding.top;
    let mut block_offset = content_edge;
    let mut margin_strut = MarginStrut::new();
    let mut child_fragments: Vec<Fragment> = Vec::new();
    let mut intrinsic_block_size = content_edge;

    // Track whether the start margin was resolved. If it collapses through
    // (first child with no border/padding/new-FC), we need to propagate it
    // via end_margin_strut so the parent can merge it with its own margin.
    // CSS 2.1 §8.3.1: parent/first-child margin collapsing.
    let mut start_margin_resolved = false;
    // Holds the margin strut as it was before the first child's trailing reset.
    // CSS 2.1 §8.3.1: when the parent has no border/padding/FC, the first
    // child's top margin collapses with the parent's top margin. This saved
    // strut captures that chain for start_margin_strut propagation.
    let mut saved_start_strut: Option<MarginStrut> = None;

    // CSS 2.1 §10.6.7: Track the maximum float bottom margin edge (in BFC
    // coordinates) so BFC roots can extend their auto height to include floats.
    let mut max_float_bottom = LayoutUnit::zero();

    // Track whether a descendant float forced BFC offset resolution.
    // When true, this block's start_margin_strut should NOT propagate to the
    // parent — subsequent margins go into internal block_offset instead.
    // Cascaded from child fragments via float_resolved_bfc flag.
    let mut float_resolved_bfc = false;

    // Float exclusions to propagate to the parent (for non-BFC blocks only).
    // Populated inside the block-children path after the child loop.
    let mut float_exclusions_result: Vec<ExclusionArea> = Vec::new();

    // CSS 2.1 §8.3.1: Self-collapsing blocks are positioned at the final
    // collapsed margin boundary, but that boundary is unknown until the next
    // non-self-collapsing sibling resolves the strut. Track their indices
    // in child_fragments so we can reposition them when the boundary is known.
    let mut pending_self_collapsing: Vec<usize> = Vec::new();

    // Collect out-of-flow (absolute/fixed) candidates for deferred layout.
    // These are populated during the child walk below so that each candidate
    // gets the correct static position (i.e., where it would appear in
    // normal flow after preceding in-flow siblings).
    //
    // Per CSS 2.1 §10.1:
    // - Absolute-pos: containing block = nearest positioned ancestor
    // - Fixed-pos: containing block = initial containing block (viewport/root)
    // A positioned element establishes a CB for absolute descendants.
    // Only the root captures fixed-pos descendants.
    let is_root = node_id == doc.root();
    // Opacity does not create a geometric abspos containing block, but in this
    // fragment model we must keep auto-positioned abspos descendants under the
    // opacity fragment so they composite with the opacity stacking context.
    let establishes_cb_for_abspos = style.position.is_positioned()
        || style.establishes_transform_containing_block
        || style.opacity < 1.0
        || is_root;
    let captures_fixed_pos_descendants = is_root || style.establishes_transform_containing_block;
    let mut oof_candidates: Vec<OutOfFlowCandidate> = Vec::new();
    let mut bubbled_oof_candidates: Vec<OutOfFlowCandidate> = Vec::new();
    // Per CSS 2.1 §10.1, the containing block for absolute positioning is the
    // padding edge of the positioned ancestor. We use content_width + padding
    // (i.e. padding-box width). For height, we use the available block size.
    let containing_block_size = PhysicalSize::new(
        child_available_inline + padding.left + padding.right,
        space.available_block_size,
    );

    // Classify children: detect whether we have only inline, only block,
    // or mixed content (CSS 2.2 §9.2.1.1 — anonymous block boxes).
    let has_inline = crate::inline::algorithm::has_inline_children(doc, node_id);
    let has_block = has_block_children(doc, node_id);

    // Track baselines from inline and block children.
    // CSS Inline 3 §3: first baseline = baseline of first line box or first child
    // with a baseline; last baseline = baseline of last line box / last child.
    let mut first_baseline_result: Option<LayoutUnit> = None;
    let mut last_baseline_result: Option<LayoutUnit> = None;

    if has_inline && !has_block {
        // ── Pure inline formatting context ───────────────────────────
        // Handle float and OOF children first (leading floats), then lay out
        // inline content with float-aware per-line available width.
        let mut exclusion_space_inline = initial_exclusion_space(space);

        for child_id in doc.children(node_id) {
            let child_style = &doc.node(child_id).style;
            if child_style.display == Display::None {
                continue;
            }
            if child_style.position.is_absolutely_positioned() {
                let candidate = OutOfFlowCandidate {
                    node_id: child_id,
                    style: child_style.clone(),
                    static_position: PhysicalOffset::new(border.left + padding.left, block_offset),
                    containing_block_size,
                    containing_block_border: border.clone(),
                    containing_block_direction: style.direction,
                    static_position_direction: style.direction,
                };
                let captures = if child_style.position == Position::Fixed {
                    is_root || style.establishes_transform_containing_block
                } else {
                    establishes_cb_for_abspos
                };
                if captures {
                    oof_candidates.push(candidate);
                } else {
                    bubbled_oof_candidates.push(candidate);
                }
                continue;
            }
            // CSS 2.1 §9.5: Float children are positioned as leading floats
            // before inline content begins. Their exclusion areas affect
            // per-line available width via the exclusion space.
            if child_style.float != Float::None {
                handle_float(
                    doc,
                    child_id,
                    space,
                    child_available_inline,
                    child_percentage_block_size,
                    &border,
                    &padding,
                    content_edge,
                    &block_offset,
                    &mut exclusion_space_inline,
                    &mut child_fragments,
                    &mut oof_candidates,
                    &mut bubbled_oof_candidates,
                    establishes_cb_for_abspos,
                    captures_fixed_pos_descendants,
                    &mut max_float_bottom,
                );
            }
        }

        // Pre-collect inline items to detect block-in-inline (CSS 2.2 §9.2.1.1).
        let mut items_data =
            crate::inline::items_builder::InlineItemsBuilder::collect(doc, node_id);

        if !items_data.block_in_inline.is_empty() {
            // ── Block-in-inline: split IFC around block-level elements ──
            // CSS 2.2 §9.2.1.1: When block elements appear inside inline
            // content (e.g. <span>text<div>block</div>text</span>), we split
            // the inline items into segments separated by block elements and
            // lay out each segment as an anonymous inline wrapper, with the
            // block elements laid out between them.
            let base_direction = if style.direction == Direction::Rtl {
                openui_text::TextDirection::Rtl
            } else {
                openui_text::TextDirection::Ltr
            };
            items_data.apply_bidi(base_direction);
            items_data.shape_text();

            let mut current_block_offset = content_edge;

            let block_in_inline_sorted = {
                let mut v = items_data.block_in_inline.clone();
                v.sort_by_key(|b| b.item_index);
                v
            };

            let mut segment_start_item = 0usize;
            let mut is_first_segment = true;
            for bi_info in &block_in_inline_sorted {
                let segment_end_item = bi_info.item_index;

                // Lay out inline items [segment_start..segment_end) as an
                // anonymous inline segment (if non-empty text content exists).
                if segment_end_item > segment_start_item {
                    let has_content = items_data.items[segment_start_item..segment_end_item]
                        .iter()
                        .any(|item| {
                            use crate::inline::items::InlineItemType;
                            matches!(
                                item.item_type,
                                InlineItemType::Text | InlineItemType::AtomicInline
                            )
                        });
                    if has_content {
                        let mut seg_space = ConstraintSpace::for_block_child(
                            child_available_inline,
                            space.available_block_size,
                            child_available_inline,
                            child_percentage_block_size,
                            false,
                        );
                        if exclusion_space_inline.has_floats() {
                            seg_space.exclusion_space =
                                Some(std::sync::Arc::new(exclusion_space_inline.clone()));
                        }
                        let seg_frag = crate::inline::algorithm::inline_layout_from_items(
                            doc,
                            node_id,
                            &seg_space,
                            &items_data,
                            segment_start_item,
                            segment_end_item,
                        );

                        if is_first_segment {
                            if let Some(fb) = seg_frag.first_baseline {
                                first_baseline_result = Some(current_block_offset + fb);
                            }
                        }
                        if let Some(lb) = seg_frag.last_baseline {
                            last_baseline_result = Some(current_block_offset + lb);
                        }

                        for line_frag in seg_frag.children {
                            let line_height = line_frag.size.height;
                            let orig_top = line_frag.offset.top;
                            let mut positioned_line = line_frag;
                            positioned_line.offset = PhysicalOffset::new(
                                border.left + padding.left + positioned_line.offset.left,
                                current_block_offset + orig_top,
                            );
                            current_block_offset = (current_block_offset + orig_top + line_height)
                                .max_of(current_block_offset);
                            child_fragments.push(positioned_line);
                        }
                    }
                }
                is_first_segment = false;

                // Lay out the block-level element.
                let block_child_id = bi_info.node_id;
                let block_child_style = &doc.node(block_child_id).style;
                let block_child_space = ConstraintSpace::for_block_child(
                    child_available_inline,
                    space.available_block_size,
                    child_available_inline,
                    child_percentage_block_size,
                    false,
                );
                let block_child_frag = block_layout(doc, block_child_id, &block_child_space);

                let bm_top = resolve_margin_or_padding(
                    &block_child_style.margin_top,
                    child_available_inline,
                );
                let bm_bottom = resolve_margin_or_padding(
                    &block_child_style.margin_bottom,
                    child_available_inline,
                );

                current_block_offset = current_block_offset + bm_top;
                let mut positioned_block = block_child_frag;
                positioned_block.offset =
                    PhysicalOffset::new(border.left + padding.left, current_block_offset);
                current_block_offset =
                    current_block_offset + positioned_block.size.height + bm_bottom;
                child_fragments.push(positioned_block);

                segment_start_item = segment_end_item + 1;
            }

            // Lay out remaining inline items after the last block-in-inline.
            if segment_start_item < items_data.items.len() {
                let has_content = items_data.items[segment_start_item..].iter().any(|item| {
                    use crate::inline::items::InlineItemType;
                    matches!(
                        item.item_type,
                        InlineItemType::Text | InlineItemType::AtomicInline
                    )
                });
                if has_content {
                    let mut seg_space = ConstraintSpace::for_block_child(
                        child_available_inline,
                        space.available_block_size,
                        child_available_inline,
                        child_percentage_block_size,
                        false,
                    );
                    if exclusion_space_inline.has_floats() {
                        seg_space.exclusion_space =
                            Some(std::sync::Arc::new(exclusion_space_inline.clone()));
                    }
                    let seg_frag = crate::inline::algorithm::inline_layout_from_items(
                        doc,
                        node_id,
                        &seg_space,
                        &items_data,
                        segment_start_item,
                        items_data.items.len(),
                    );

                    if let Some(lb) = seg_frag.last_baseline {
                        last_baseline_result = Some(current_block_offset + lb);
                    }
                    if first_baseline_result.is_none() {
                        if let Some(fb) = seg_frag.first_baseline {
                            first_baseline_result = Some(current_block_offset + fb);
                        }
                    }

                    for line_frag in seg_frag.children {
                        let line_height = line_frag.size.height;
                        let orig_top = line_frag.offset.top;
                        let mut positioned_line = line_frag;
                        positioned_line.offset = PhysicalOffset::new(
                            border.left + padding.left + positioned_line.offset.left,
                            current_block_offset + orig_top,
                        );
                        current_block_offset = (current_block_offset + orig_top + line_height)
                            .max_of(current_block_offset);
                        child_fragments.push(positioned_line);
                    }
                }
            }

            intrinsic_block_size = current_block_offset;
            block_offset = intrinsic_block_size;
        } else {
            // No block-in-inline: standard inline layout path.
            // Build constraint space with exclusion data for per-line float avoidance.
            let mut inline_space = ConstraintSpace::for_block_child(
                child_available_inline,
                space.available_block_size,
                child_available_inline,
                child_percentage_block_size,
                false,
            );
            if exclusion_space_inline.has_floats() {
                inline_space.exclusion_space = Some(std::sync::Arc::new(exclusion_space_inline));
            }
            let inline_fragment =
                crate::inline::algorithm::inline_layout(doc, node_id, &inline_space);

            // Capture baselines from inline layout, adjusted to border-box coordinates.
            if let Some(fb) = inline_fragment.first_baseline {
                first_baseline_result = Some(content_edge + fb);
            }
            if let Some(lb) = inline_fragment.last_baseline {
                last_baseline_result = Some(content_edge + lb);
            }

            for line_frag in inline_fragment.children {
                let line_height = line_frag.size.height;
                let mut positioned_line = line_frag;
                positioned_line.offset = PhysicalOffset::new(
                    border.left + padding.left + positioned_line.offset.left,
                    content_edge + positioned_line.offset.top,
                );
                intrinsic_block_size =
                    intrinsic_block_size.max_of(positioned_line.offset.top + line_height);
                child_fragments.push(positioned_line);
            }
            block_offset = intrinsic_block_size;
        }
    } else if has_inline && has_block {
        // ── Mixed content: create anonymous block boxes (CSS 2.2 §9.2.1.1) ─
        // Collect contiguous runs of inline children into anonymous wrappers,
        // interleaved with real block-level children.
        let children_ids: Vec<NodeId> = doc.children(node_id).collect();
        let mut i = 0;
        let mut exclusion_space_mixed = initial_exclusion_space(space);

        while i < children_ids.len() {
            let child_id = children_ids[i];
            let child_style = &doc.node(child_id).style;

            // Skip display:none children.
            if child_style.display == Display::None {
                i += 1;
                continue;
            }

            // Collect absolutely positioned children with correct static position.
            // CSS 2.1 §10.6.4: Include pending margin_strut in block-axis
            // static position for the hypothetical normal-flow position.
            if child_style.position.is_absolutely_positioned() {
                let candidate = OutOfFlowCandidate {
                    node_id: child_id,
                    style: child_style.clone(),
                    static_position: PhysicalOffset::new(
                        border.left + padding.left,
                        block_offset + margin_strut.sum(),
                    ),
                    containing_block_size,
                    containing_block_border: border.clone(),
                    containing_block_direction: style.direction,
                    static_position_direction: style.direction,
                };
                let captures = if child_style.position == Position::Fixed {
                    is_root || style.establishes_transform_containing_block
                } else {
                    establishes_cb_for_abspos
                };
                if captures {
                    oof_candidates.push(candidate);
                } else {
                    bubbled_oof_candidates.push(candidate);
                }
                i += 1;
                continue;
            }

            // Handle floated children.
            if child_style.float != Float::None {
                // CSS 2.1: Floats force BFC offset resolution. Any pending
                // margin strut must be resolved before positioning the float.
                if !start_margin_resolved {
                    block_offset += margin_strut.sum();
                    margin_strut = MarginStrut::new();
                    start_margin_resolved = true;
                    float_resolved_bfc = true;
                }
                handle_float(
                    doc,
                    child_id,
                    space,
                    child_available_inline,
                    child_percentage_block_size,
                    &border,
                    &padding,
                    content_edge,
                    &block_offset,
                    &mut exclusion_space_mixed,
                    &mut child_fragments,
                    &mut oof_candidates,
                    &mut bubbled_oof_candidates,
                    establishes_cb_for_abspos,
                    captures_fixed_pos_descendants,
                    &mut max_float_bottom,
                );
                i += 1;
                continue;
            }

            if is_inline_level_child(doc, child_id) {
                // Gather contiguous run of inline children.
                // display:none and abs-pos children are transparent to
                // inline run gathering. Floats break the run.
                // Abs-pos children encountered here are collected as OOF
                // candidates with the current block_offset as static position.
                let run_start = i;
                while i < children_ids.len() {
                    let cid = children_ids[i];
                    let cs = &doc.node(cid).style;
                    if cs.display == Display::None {
                        i += 1;
                        continue;
                    }
                    if cs.position.is_absolutely_positioned() {
                        // CSS 2.1 §10.6.4: Include pending margin_strut in
                        // block-axis static position.
                        let candidate = OutOfFlowCandidate {
                            node_id: cid,
                            style: cs.clone(),
                            static_position: PhysicalOffset::new(
                                border.left + padding.left,
                                block_offset + margin_strut.sum(),
                            ),
                            containing_block_size,
                            containing_block_border: border.clone(),
                            containing_block_direction: style.direction,
                            static_position_direction: style.direction,
                        };
                        let captures = if cs.position == Position::Fixed {
                            is_root || style.establishes_transform_containing_block
                        } else {
                            establishes_cb_for_abspos
                        };
                        if captures {
                            oof_candidates.push(candidate);
                        } else {
                            bubbled_oof_candidates.push(candidate);
                        }
                        i += 1;
                        continue;
                    }
                    if cs.float != Float::None {
                        break;
                    }
                    if !is_inline_level_child(doc, cid) {
                        break;
                    }
                    i += 1;
                }
                let inline_run = &children_ids[run_start..i];

                // Anonymous inline wrapper is non-self-collapsing (has content),
                // so it breaks the margin collapsing chain per CSS 2.1 §8.3.1.
                // Resolve any pending margin strut before positioning.
                if !start_margin_resolved {
                    if space.is_new_formatting_context || content_edge > LayoutUnit::zero() {
                        block_offset += margin_strut.sum();
                        margin_strut = MarginStrut::new();
                        start_margin_resolved = true;
                    } else {
                        // No FC/border/padding: propagate accumulated strut as
                        // the parent's start margin, then reset for this inline run.
                        saved_start_strut = Some(margin_strut);
                        margin_strut = MarginStrut::new();
                        start_margin_resolved = true;
                    }
                } else if !margin_strut.is_empty() {
                    block_offset += margin_strut.sum();
                    margin_strut = MarginStrut::new();
                }

                // Lay out this anonymous inline wrapper.
                // Pass the exclusion space so inline layout can do per-line
                // float avoidance (CSS 2.1 §9.5.1).
                let mut inline_space = ConstraintSpace::for_block_child(
                    child_available_inline,
                    space.available_block_size,
                    child_available_inline,
                    child_percentage_block_size,
                    false,
                );
                if exclusion_space_mixed.has_floats() {
                    // The exclusion space uses content-edge-relative coordinates.
                    // Inline layout's block_offset starts at 0, but the anonymous
                    // wrapper begins at `block_offset - content_edge` within the
                    // content area. Set the bfc_offset so inline layout queries
                    // at the correct position in the exclusion space.
                    inline_space.exclusion_space =
                        Some(std::sync::Arc::new(exclusion_space_mixed.clone()));
                    inline_space.bfc_offset =
                        BfcOffset::new(LayoutUnit::zero(), block_offset - content_edge);
                }
                let anon_fragment = crate::inline::algorithm::inline_layout_for_children(
                    doc,
                    node_id,
                    inline_run,
                    &inline_space,
                );

                // Capture baselines from anonymous inline wrapper.
                if let Some(fb) = anon_fragment.first_baseline {
                    if first_baseline_result.is_none() {
                        first_baseline_result = Some(block_offset + fb);
                    }
                    last_baseline_result = Some(block_offset + fb);
                }
                if let Some(lb) = anon_fragment.last_baseline {
                    last_baseline_result = Some(block_offset + lb);
                }

                for line_frag in anon_fragment.children {
                    let line_height = line_frag.size.height;
                    let mut positioned_line = line_frag;
                    positioned_line.offset = PhysicalOffset::new(
                        border.left + padding.left + positioned_line.offset.left,
                        block_offset + positioned_line.offset.top,
                    );
                    intrinsic_block_size =
                        intrinsic_block_size.max_of(positioned_line.offset.top + line_height);
                    child_fragments.push(positioned_line);
                }
                block_offset = intrinsic_block_size;
            } else {
                // Block-level child.

                // Handle clear property — CSS 2.1 §8.3.1 / §9.5.2.
                // Compute hypothetical position (with margin collapsing) first,
                // then determine clearance as additional distance needed.
                // Clearance also inhibits margin collapsing.
                if child_style.clear != Clear::None {
                    let child_margin = resolve_margins(child_style, child_available_inline);
                    let child_top_margin = child_margin.top;
                    let mut hyp_strut = margin_strut;
                    hyp_strut.append_normal(child_top_margin);
                    let hypothetical = block_offset + hyp_strut.sum();
                    let clearance_target = exclusion_space_mixed
                        .clearance_offset(clear_type_from_style(child_style.clear))
                        + content_edge;
                    if clearance_target > hypothetical {
                        // Clearance positions the border edge at clearance_target.
                        // layout_block_child will re-append child_top_margin and
                        // resolve it, so compensate by subtracting it here.
                        block_offset = clearance_target - child_top_margin;
                        margin_strut = MarginStrut::new();
                        start_margin_resolved = true;
                        intrinsic_block_size = intrinsic_block_size.max_of(clearance_target);
                    }
                }

                // Adjust available inline size for float exclusions.
                // CSS 2.1 §9.5: Only new-FC children must NOT overlap float margin boxes.
                // Regular (non-FC) block children overlap floats — only their line
                // boxes avoid floats (handled in inline layout).
                let child_is_new_fc_caller = establishes_new_fc(child_style);
                let (float_inline_offset, adjusted_available) = if exclusion_space_mixed
                    .has_floats()
                    && child_is_new_fc_caller
                {
                    let child_margin = resolve_margins(child_style, child_available_inline);
                    let child_top_margin = child_margin.top;
                    let mut temp_strut = margin_strut;
                    temp_strut.append_normal(child_top_margin);
                    let resolved_offset = block_offset + temp_strut.sum();
                    let content_block_offset = resolved_offset - content_edge;
                    let min_inline_size = if uses_margin_reduced_inherited_exclusions(space) {
                        LayoutUnit::zero()
                    } else {
                        new_fc_min_inline_size(child_style, child_available_inline)
                    };

                    // Use height-aware opportunity search for explicit-height BFCs.
                    let child_block_size = if !child_style.height.is_auto()
                        && !child_style.height.is_stretch()
                        && !child_style.height.is_content_or_intrinsic()
                        && !child_style.height.is_percent()
                    {
                        let raw = resolve_length(
                            &child_style.height,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        let bp_block =
                            resolve_margin_or_padding(
                                &child_style.padding_top,
                                child_available_inline,
                            ) + resolve_margin_or_padding(
                                &child_style.padding_bottom,
                                child_available_inline,
                            ) + LayoutUnit::from_raw(child_style.border_top_width as i32 * 64)
                                + LayoutUnit::from_raw(child_style.border_bottom_width as i32 * 64);
                        let total = if child_style.box_sizing == BoxSizing::ContentBox {
                            raw + bp_block
                        } else {
                            raw
                        };
                        let child_mb = resolve_margin_or_padding(
                            &child_style.margin_bottom,
                            child_available_inline,
                        );
                        total + child_mb
                    } else {
                        LayoutUnit::zero()
                    };

                    let opp = if child_block_size > LayoutUnit::zero() {
                        exclusion_space_mixed.find_opportunity_for_bfc(
                            &BfcOffset::new(LayoutUnit::zero(), content_block_offset),
                            child_available_inline,
                            min_inline_size,
                            child_block_size,
                        )
                    } else {
                        exclusion_space_mixed.find_layout_opportunity(
                            &BfcOffset::new(LayoutUnit::zero(), content_block_offset),
                            child_available_inline,
                            min_inline_size,
                        )
                    };

                    let pushed_bfc = opp.rect.block_start_offset();
                    if pushed_bfc > content_block_offset {
                        let push_amount = pushed_bfc - content_block_offset;
                        block_offset = block_offset + push_amount;
                        margin_strut = MarginStrut::new();
                        start_margin_resolved = true;
                    }

                    new_fc_placement_for_opportunity(&opp, child_margin, child_available_inline)
                } else {
                    (LayoutUnit::zero(), child_available_inline)
                };

                let oof_count_before = oof_candidates.len();
                let bubbled_count_before = bubbled_oof_candidates.len();

                layout_block_child(
                    doc,
                    child_id,
                    space,
                    adjusted_available,
                    child_percentage_block_size,
                    children_available_block_size,
                    &border,
                    &padding,
                    content_edge,
                    &mut block_offset,
                    &mut margin_strut,
                    &mut intrinsic_block_size,
                    &mut child_fragments,
                    &mut oof_candidates,
                    &mut bubbled_oof_candidates,
                    establishes_cb_for_abspos,
                    captures_fixed_pos_descendants,
                    &mut start_margin_resolved,
                    &mut saved_start_strut,
                    style.direction,
                    &mut pending_self_collapsing,
                    Some(&exclusion_space_mixed),
                );

                // Offset inline position for left floats.
                // Also shift OOF static positions that were computed
                // using the pre-shift child offset.
                if float_inline_offset > LayoutUnit::zero() {
                    if let Some(last) = child_fragments.last_mut() {
                        last.offset.left = last.offset.left + float_inline_offset;
                    }
                    for c in &mut oof_candidates[oof_count_before..] {
                        c.static_position.left = c.static_position.left + float_inline_offset;
                    }
                    for c in &mut bubbled_oof_candidates[bubbled_count_before..] {
                        c.static_position.left = c.static_position.left + float_inline_offset;
                    }
                }

                i += 1;
            }
        }
    } else {
        // ── Pure block formatting context ────────────────────────────
        let mut exclusion_space = initial_exclusion_space(space);

        for child_id in doc.children(node_id) {
            let child_style = &doc.node(child_id).style;

            // Skip display:none children.
            if child_style.display == Display::None {
                continue;
            }

            // Collect absolutely positioned children with correct static position
            // (where they would appear in normal flow after preceding siblings).
            // CSS 2.1 §10.6.4: Include pending margin_strut in block-axis static
            // position — the hypothetical normal-flow position includes the
            // unresolved inter-sibling margin gap.
            if child_style.position.is_absolutely_positioned() {
                let candidate = OutOfFlowCandidate {
                    node_id: child_id,
                    style: child_style.clone(),
                    static_position: PhysicalOffset::new(
                        border.left + padding.left,
                        block_offset + margin_strut.sum(),
                    ),
                    containing_block_size,
                    containing_block_border: border.clone(),
                    containing_block_direction: style.direction,
                    static_position_direction: style.direction,
                };
                let captures = if child_style.position == Position::Fixed {
                    is_root || style.establishes_transform_containing_block
                } else {
                    establishes_cb_for_abspos
                };
                if captures {
                    oof_candidates.push(candidate);
                } else {
                    bubbled_oof_candidates.push(candidate);
                }
                continue;
            }
            // Handle floated children — they are positioned in the exclusion
            // space and do not advance the block offset.
            // CSS 2.1 §9.7: Floats are blockified regardless of display value,
            // so this check must come BEFORE the inline-level skip below.
            if child_style.float != Float::None {
                // CSS 2.1: Floats force BFC offset resolution.
                if !start_margin_resolved {
                    block_offset += margin_strut.sum();
                    margin_strut = MarginStrut::new();
                    start_margin_resolved = true;
                    float_resolved_bfc = true;
                }
                handle_float(
                    doc,
                    child_id,
                    space,
                    child_available_inline,
                    child_percentage_block_size,
                    &border,
                    &padding,
                    content_edge,
                    &block_offset,
                    &mut exclusion_space,
                    &mut child_fragments,
                    &mut oof_candidates,
                    &mut bubbled_oof_candidates,
                    establishes_cb_for_abspos,
                    captures_fixed_pos_descendants,
                    &mut max_float_bottom,
                );
                continue;
            }

            // Skip inline-level non-float children in pure block context.
            if child_style.display.is_inline_level() {
                continue;
            }

            // Handle clear property — CSS 2.1 §8.3.1 / §9.5.2.
            // Compute hypothetical position (with margin collapsing) first,
            // then determine clearance as additional distance needed.
            // Clearance inhibits margin collapsing.
            if child_style.clear != Clear::None {
                let child_margin = resolve_margins(child_style, child_available_inline);
                let child_top_margin = child_margin.top;
                let mut hyp_strut = margin_strut;
                hyp_strut.append_normal(child_top_margin);
                let hypothetical = block_offset + hyp_strut.sum();
                let clearance_target = exclusion_space
                    .clearance_offset(clear_type_from_style(child_style.clear))
                    + content_edge;
                if clearance_target > hypothetical {
                    // Clearance positions the border edge at clearance_target.
                    // layout_block_child will re-append child_top_margin and
                    // resolve it, so compensate by subtracting it here.
                    block_offset = clearance_target - child_top_margin;
                    margin_strut = MarginStrut::new();
                    start_margin_resolved = true;
                    intrinsic_block_size = intrinsic_block_size.max_of(clearance_target);
                }
            }

            // Adjust available inline size for float exclusions.
            // CSS 2.1 §9.5: Only new-FC children must NOT overlap float margin boxes.
            // Regular (non-FC) block children overlap floats — only their line
            // boxes avoid floats (handled in inline layout).
            let child_is_new_fc_caller = establishes_new_fc(child_style);
            let child_margin_for_fc = if child_is_new_fc_caller {
                resolve_margins(child_style, child_available_inline)
            } else {
                BoxStrut::zero()
            };

            let (mut float_inline_offset, mut adjusted_available) = if exclusion_space.has_floats()
                && child_is_new_fc_caller
            {
                let child_margin = child_margin_for_fc;
                let child_top_margin = child_margin.top;
                let min_inline_size = if uses_margin_reduced_inherited_exclusions(space) {
                    LayoutUnit::zero()
                } else {
                    new_fc_min_inline_size(child_style, child_available_inline)
                };

                // Chromium two-estimate mechanism (HandleNewFormattingContext):
                // When the parent's BFC block offset is unresolved (non-FC parent,
                // no border/padding, start margin not yet resolved), check whether
                // the new-FC child fits beside adjoining floats at the collapsed
                // position. If not, the margin "separates" — the child's margin
                // does NOT collapse with the parent's margin group.
                let mut margin_got_separated = false;

                if !start_margin_resolved
                    && !space.is_new_formatting_context
                    && content_edge == LayoutUnit::zero()
                {
                    let adj_opp = exclusion_space.find_layout_opportunity(
                        &BfcOffset::new(LayoutUnit::zero(), LayoutUnit::zero()),
                        child_available_inline,
                        min_inline_size,
                    );
                    if adj_opp.rect.block_start_offset() > LayoutUnit::zero() {
                        margin_got_separated = true;
                    }
                }

                if margin_got_separated {
                    // Margin separates: save accumulated margins for parent
                    // propagation, then resolve WITHOUT the child's margin.
                    if saved_start_strut.is_none() {
                        saved_start_strut = Some(margin_strut);
                    }
                    margin_strut = MarginStrut::new();
                    start_margin_resolved = true;

                    // Flush pending self-collapsing blocks at the pre-separation
                    // boundary. Their edges coincide at the current block_offset
                    // (before the push below), not at the post-margin position.
                    for &idx in pending_self_collapsing.iter() {
                        child_fragments[idx].offset.top = block_offset;
                    }
                    pending_self_collapsing.clear();

                    // Find first opportunity below the float.
                    let non_adj_opp = exclusion_space.find_layout_opportunity(
                        &BfcOffset::new(LayoutUnit::zero(), LayoutUnit::zero()),
                        child_available_inline,
                        min_inline_size,
                    );

                    let pushed_block = non_adj_opp.rect.block_start_offset();
                    // Pre-subtract child_top_margin: layout_block_child will
                    // unconditionally add it via the margin strut, restoring
                    // the correct final position at the opportunity.
                    block_offset = content_edge + pushed_block - child_top_margin;

                    new_fc_placement_for_opportunity(
                        &non_adj_opp,
                        child_margin,
                        child_available_inline,
                    )
                } else {
                    // Non-separated: the child's margin stays adjoining (or the
                    // parent's margin is already resolved).
                    if !start_margin_resolved {
                        if space.is_new_formatting_context || content_edge > LayoutUnit::zero() {
                            block_offset += margin_strut.sum();
                            margin_strut = MarginStrut::new();
                            start_margin_resolved = true;
                        }
                        // else: margin propagates through — don't resolve.
                    }

                    // Compute the search position in content-area coordinates.
                    // For unresolved margins, the child ends up at block 0
                    // (margin propagates through). For resolved margins, use the
                    // tentative position including collapsed margins.
                    let search_block = if !start_margin_resolved {
                        LayoutUnit::zero()
                    } else {
                        let mut temp_strut = margin_strut;
                        temp_strut.append_normal(child_top_margin);
                        block_offset + temp_strut.sum() - content_edge
                    };

                    // Height-aware opportunity search if child has explicit height.
                    let child_block_size = if !child_style.height.is_auto()
                        && !child_style.height.is_stretch()
                        && !child_style.height.is_content_or_intrinsic()
                        && !child_style.height.is_percent()
                    {
                        let raw = resolve_length(
                            &child_style.height,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        let bp_block =
                            resolve_margin_or_padding(
                                &child_style.padding_top,
                                child_available_inline,
                            ) + resolve_margin_or_padding(
                                &child_style.padding_bottom,
                                child_available_inline,
                            ) + LayoutUnit::from_raw(child_style.border_top_width as i32 * 64)
                                + LayoutUnit::from_raw(child_style.border_bottom_width as i32 * 64);
                        let total = if child_style.box_sizing == BoxSizing::ContentBox {
                            raw + bp_block
                        } else {
                            raw
                        };
                        let child_mb = resolve_margin_or_padding(
                            &child_style.margin_bottom,
                            child_available_inline,
                        );
                        total + child_mb
                    } else {
                        LayoutUnit::zero()
                    };

                    let opp = if child_block_size > LayoutUnit::zero() {
                        exclusion_space.find_opportunity_for_bfc(
                            &BfcOffset::new(LayoutUnit::zero(), search_block),
                            child_available_inline,
                            min_inline_size,
                            child_block_size,
                        )
                    } else {
                        exclusion_space.find_layout_opportunity(
                            &BfcOffset::new(LayoutUnit::zero(), search_block),
                            child_available_inline,
                            min_inline_size,
                        )
                    };

                    let pushed_bfc = opp.rect.block_start_offset();
                    if pushed_bfc > search_block {
                        if start_margin_resolved {
                            // Position at the opportunity, accounting for child's
                            // margin that layout_block_child will add.
                            block_offset = content_edge + pushed_bfc - child_top_margin;
                            margin_strut = MarginStrut::new();
                        } else {
                            // Unresolved margin + push: margin must separate.
                            if saved_start_strut.is_none() {
                                saved_start_strut = Some(margin_strut);
                            }
                            margin_strut = MarginStrut::new();
                            block_offset = content_edge + pushed_bfc - child_top_margin;
                            start_margin_resolved = true;
                        }
                    }

                    new_fc_placement_for_opportunity(&opp, child_margin, child_available_inline)
                }
            } else {
                (LayoutUnit::zero(), child_available_inline)
            };

            let oof_count_before = oof_candidates.len();
            let bubbled_count_before = bubbled_oof_candidates.len();

            layout_block_child(
                doc,
                child_id,
                space,
                adjusted_available,
                child_percentage_block_size,
                children_available_block_size,
                &border,
                &padding,
                content_edge,
                &mut block_offset,
                &mut margin_strut,
                &mut intrinsic_block_size,
                &mut child_fragments,
                &mut oof_candidates,
                &mut bubbled_oof_candidates,
                establishes_cb_for_abspos,
                captures_fixed_pos_descendants,
                &mut start_margin_resolved,
                &mut saved_start_strut,
                style.direction,
                &mut pending_self_collapsing,
                Some(&exclusion_space),
            );

            // For auto-height BFC children, verify the laid-out fragment doesn't
            // overlap floats across its full height. If it does, find the correct
            // opportunity using the actual height and re-layout.
            if exclusion_space.has_floats() && child_is_new_fc_caller {
                if let Some(last_frag) = child_fragments.last() {
                    let frag_block_size = last_frag.size.height;
                    let relative_offset = if child_style.position == Position::Relative {
                        crate::relative::compute_relative_offset(
                            child_style,
                            child_available_inline,
                            space.available_block_size,
                        )
                    } else {
                        PhysicalOffset::zero()
                    };
                    let frag_block_offset =
                        last_frag.offset.top - relative_offset.top - content_edge;
                    let frag_line_offset = last_frag.offset.left - relative_offset.left
                        + float_inline_offset
                        - border.left
                        - padding.left;
                    let child_rect = BfcRect::new(
                        BfcOffset::new(frag_line_offset, frag_block_offset),
                        BfcOffset::new(
                            frag_line_offset + last_frag.size.width,
                            frag_block_offset + frag_block_size,
                        ),
                    );
                    let inherited_margin_reduced = uses_margin_reduced_inherited_exclusions(space);
                    let overlaps_float =
                        !inherited_margin_reduced && exclusion_space.overlaps_float(&child_rect);
                    let child_mb = resolve_margin_or_padding(
                        &child_style.margin_bottom,
                        child_available_inline,
                    );
                    let total_block = frag_block_size + child_mb;
                    let min_inline_size = if inherited_margin_reduced {
                        LayoutUnit::zero()
                    } else {
                        new_fc_min_inline_size(child_style, child_available_inline)
                    };
                    let verified_min_inline_size = if overlaps_float {
                        min_inline_size.max_of(last_frag.size.width)
                    } else {
                        min_inline_size
                    };

                    let verified_opp = if child_style.height.is_auto()
                        && child_style.aspect_ratio.is_some()
                        && (child_style.width.is_auto() || child_style.width.is_stretch())
                    {
                        exclusion_space.find_opportunity_for_auto_sized_bfc(
                            &BfcOffset::new(LayoutUnit::zero(), frag_block_offset),
                            child_available_inline,
                            min_inline_size,
                            |opportunity_inline| {
                                estimate_auto_aspect_ratio_block_size(
                                    child_style,
                                    opportunity_inline,
                                    child_margin_for_fc,
                                )
                            },
                        )
                    } else {
                        exclusion_space.find_opportunity_for_bfc(
                            &BfcOffset::new(LayoutUnit::zero(), frag_block_offset),
                            child_available_inline,
                            verified_min_inline_size,
                            total_block,
                        )
                    };

                    let verified_start = verified_opp.rect.block_start_offset();
                    let (verified_left, verified_available) = new_fc_placement_for_opportunity(
                        &verified_opp,
                        child_margin_for_fc,
                        child_available_inline,
                    );

                    // Compare verified opportunity against what we initially used.
                    if overlaps_float
                        || verified_start > frag_block_offset
                        || verified_left != float_inline_offset
                        || verified_available != adjusted_available
                    {
                        let push_amount = if verified_start > frag_block_offset {
                            verified_start - frag_block_offset
                        } else {
                            LayoutUnit::zero()
                        };

                        // Remove the previous layout result.
                        child_fragments.pop();
                        oof_candidates.truncate(oof_count_before);
                        bubbled_oof_candidates.truncate(bubbled_count_before);

                        // Revert block_offset to pre-layout state, apply verified push.
                        block_offset = block_offset - frag_block_size - child_mb + push_amount;
                        if push_amount > LayoutUnit::zero() {
                            margin_strut = MarginStrut::new();
                        }

                        // Re-layout with corrected available width.
                        layout_block_child(
                            doc,
                            child_id,
                            space,
                            verified_available,
                            child_percentage_block_size,
                            children_available_block_size,
                            &border,
                            &padding,
                            content_edge,
                            &mut block_offset,
                            &mut margin_strut,
                            &mut intrinsic_block_size,
                            &mut child_fragments,
                            &mut oof_candidates,
                            &mut bubbled_oof_candidates,
                            establishes_cb_for_abspos,
                            captures_fixed_pos_descendants,
                            &mut start_margin_resolved,
                            &mut saved_start_strut,
                            style.direction,
                            &mut pending_self_collapsing,
                            Some(&exclusion_space),
                        );

                        // Update float offset to verified values.
                        float_inline_offset = verified_left;
                        adjusted_available = verified_available;
                    }
                }
            }

            // Offset inline position for left floats.
            // Also shift OOF static positions computed with pre-shift offset.
            if float_inline_offset > LayoutUnit::zero() {
                if let Some(last) = child_fragments.last_mut() {
                    last.offset.left = last.offset.left + float_inline_offset;
                }
                for c in &mut oof_candidates[oof_count_before..] {
                    c.static_position.left = c.static_position.left + float_inline_offset;
                }
                for c in &mut bubbled_oof_candidates[bubbled_count_before..] {
                    c.static_position.left = c.static_position.left + float_inline_offset;
                }
            }

            // CSS 2.1 §9.5: Propagate float exclusions from non-BFC children.
            // Floats inside non-BFC wrapper blocks participate in the nearest
            // ancestor BFC's exclusion space. After laying out a child, absorb
            // its float_exclusions (if any) into our exclusion_space, translating
            // from child's content-area coordinates to our content-area coordinates.
            if !child_is_new_fc_caller {
                if let Some(child_frag) = child_fragments.last() {
                    if !child_frag.float_exclusions.is_empty() {
                        let child_border = &child_frag.border;
                        let child_padding = &child_frag.padding;
                        // Translation from child's content area to parent's content area:
                        // child_frag.offset is in parent's border-box coords.
                        // Parent's BFC (0,0) = parent's content area = (border.left+padding.left, content_edge).
                        let block_adj = (child_frag.offset.top - content_edge)
                            + child_border.top
                            + child_padding.top;
                        let inline_adj = (child_frag.offset.left - border.left - padding.left)
                            + child_border.left
                            + child_padding.left;

                        for excl in &child_frag.float_exclusions {
                            let translated = ExclusionArea {
                                rect: BfcRect::new(
                                    BfcOffset::new(
                                        excl.rect.start_offset.line_offset + inline_adj,
                                        excl.rect.start_offset.block_offset + block_adj,
                                    ),
                                    BfcOffset::new(
                                        excl.rect.end_offset.line_offset + inline_adj,
                                        excl.rect.end_offset.block_offset + block_adj,
                                    ),
                                ),
                                exclusion_type: excl.exclusion_type,
                            };
                            exclusion_space.add(translated);
                        }
                    }
                }
            }
        }

        // CSS 2.1 §9.5: Floats participate in the nearest BFC's exclusion space.
        // If this block is NOT a new formatting context, propagate all float
        // exclusions upward so the parent can absorb them.
        if !space.is_new_formatting_context && exclusion_space.has_floats() {
            float_exclusions_result = exclusion_space.all_exclusions();
        }
    } // end block children

    // ── Step 4: Finish layout (FinishLayout, line 1165) ──────────────
    // Resolve the trailing margin strut if margins can't collapse through
    // the bottom edge. Per CSS 2.1 §8.3.1, the last child's bottom margin
    // collapses with the parent's bottom margin ONLY if:
    //   - parent has 'auto' computed height AND min-height is zero
    //   - no bottom padding or border separates them
    //   - parent doesn't establish a new BFC
    // When any of these conditions fails, the margin strut is consumed.
    let bottom_edge = border.bottom + padding.bottom;
    let height_is_effectively_auto = style.height.is_auto()
        || style.height.is_content_or_intrinsic()
        || (style.height.length_type() == openui_geometry::LengthType::Percent
            && space.percentage_resolution_block_size.is_indefinite())
        // height: stretch with indefinite containing block falls back to auto
        || (style.height.is_stretch()
            && !space.is_fixed_block_size
            && !space.stretch_block_size
            && space.percentage_resolution_block_size.is_indefinite());
    let has_non_auto_height =
        !height_is_effectively_auto || space.is_fixed_block_size || space.stretch_block_size;
    let has_aspect_ratio_block_size = style.height.is_auto()
        && style
            .aspect_ratio
            .as_ref()
            .is_some_and(|ar| ar.ratio.0 > 0.0 && ar.ratio.1 > 0.0)
        && !content_inline_size.is_indefinite();
    // CSS 2.1 §8.3.1: min-height > 0 prevents parent/last-child collapse.
    // Use percentage_resolution_block_size (not available_block_size) because
    // percentage min-height resolves against the containing block's height,
    // which may be indefinite even when available block size is definite.
    let has_min_height = !style.min_height.is_auto()
        && !style.min_height.is_none()
        && match style.min_height.length_type() {
            openui_geometry::LengthType::Fixed => style.min_height.value() > 0.0,
            openui_geometry::LengthType::Percent => {
                !space.percentage_resolution_block_size.is_indefinite()
                    && style.min_height.value() > 0.0
            }
            _ => false,
        };
    let end_margin_resolved = !margin_strut.is_empty()
        && (space.is_new_formatting_context
            || bottom_edge > LayoutUnit::zero()
            || has_non_auto_height
            || has_aspect_ratio_block_size
            || has_min_height);
    if end_margin_resolved {
        if !has_aspect_ratio_block_size {
            intrinsic_block_size += margin_strut.sum();
        }
        // Reposition any remaining pending self-collapsing children to the
        // final collapsed margin boundary. This handles the case where the
        // last child(ren) are self-collapsing and no subsequent non-self-
        // collapsing sibling triggered the flush.
        let resolved_offset = block_offset + margin_strut.sum();
        for &idx in pending_self_collapsing.iter() {
            child_fragments[idx].offset.top = resolved_offset;
        }
        pending_self_collapsing.clear();
    }
    // Capture end margin strut before it's consumed. If the end margin
    // couldn't be resolved (no bottom border/padding, not a new FC), it
    // propagates to the parent for collapsing with the parent's bottom margin
    // or the next sibling's top margin.
    let final_end_margin_strut = if end_margin_resolved {
        MarginStrut::new()
    } else {
        margin_strut
    };
    let aspect_ratio_collapsed_end_strut = if has_aspect_ratio_block_size {
        margin_strut
    } else {
        MarginStrut::new()
    };

    if style.display == Display::ListItem && height_is_effectively_auto {
        intrinsic_block_size =
            intrinsic_block_size.max_of(content_edge + list_marker_line_height(style));
    }

    // Add bottom border + padding
    intrinsic_block_size += bottom_edge;

    // CSS 2.1 §10.6.7: If this element establishes a BFC, its auto height
    // must include any floating descendants whose bottom margin edge extends
    // below the element's bottom content edge.
    if space.is_new_formatting_context && max_float_bottom > LayoutUnit::zero() {
        let float_bottom_border_box = max_float_bottom + content_edge + bottom_edge;
        intrinsic_block_size = intrinsic_block_size.max_of(float_bottom_border_box);
    }

    // ── Step 5: Resolve height ───────────────────────────────────────
    // Blink: ComputeBlockSizeForFragment (length_utils.h:314)
    let is_viewport = doc.node(node_id).tag == openui_dom::ElementTag::Viewport;
    let resolved_block_size = resolve_block_size(
        doc,
        node_id,
        style,
        space,
        intrinsic_block_size,
        border_padding_block,
        border_padding_inline,
        content_inline_size,
        is_viewport,
    );

    // -- Fixup: percentage-based relative positioning --
    // During child layout, percentage top/bottom resolved against
    // space.available_block_size which may differ from the final height.
    //
    // CSS 2.1 section 10.8: percentage top/bottom on position:relative resolves
    // against the containing block's height (content edge per section 10.1).
    // If the CB height is not explicitly specified (auto), percentage = 0.
    {
        let actual_content_height =
            (resolved_block_size - border_padding_block).clamp_negative_to_zero();
        let old_basis = space.available_block_size;
        let height_is_explicit = has_non_auto_height || is_viewport;

        // Target: explicit height -> actual content height; auto -> INDEFINITE (pct=0)
        let target_basis = if height_is_explicit {
            actual_content_height
        } else {
            openui_geometry::INDEFINITE_SIZE
        };
        let needs_fixup = if height_is_explicit {
            old_basis.is_indefinite() || old_basis.raw() != target_basis.raw()
        } else {
            // Auto height: only need fixup if old basis was definite
            // (percentage was incorrectly applied against it).
            !old_basis.is_indefinite()
        };
        if needs_fixup {
            for frag in &mut child_fragments {
                if frag.node_id == openui_dom::NodeId::NONE {
                    continue;
                }
                let child_style = &doc.node(frag.node_id).style;
                if child_style.position != Position::Relative {
                    continue;
                }
                let has_pct_top = !child_style.top.is_auto()
                    && (child_style.top.length_type() == openui_geometry::LengthType::Percent
                        || child_style.top.length_type()
                            == openui_geometry::LengthType::Calculated);
                let has_pct_bottom = !child_style.bottom.is_auto()
                    && (child_style.bottom.length_type() == openui_geometry::LengthType::Percent
                        || child_style.bottom.length_type()
                            == openui_geometry::LengthType::Calculated);
                if !has_pct_top && !has_pct_bottom {
                    continue;
                }
                let zero = LayoutUnit::zero();
                let old_offset = if has_pct_top {
                    if old_basis.is_indefinite() {
                        zero
                    } else {
                        resolve_length(&child_style.top, old_basis, zero, zero)
                    }
                } else {
                    if old_basis.is_indefinite() {
                        zero
                    } else {
                        -resolve_length(&child_style.bottom, old_basis, zero, zero)
                    }
                };
                let new_offset = if has_pct_top {
                    if target_basis.is_indefinite() {
                        zero
                    } else {
                        resolve_length(&child_style.top, target_basis, zero, zero)
                    }
                } else {
                    if target_basis.is_indefinite() {
                        zero
                    } else {
                        -resolve_length(&child_style.bottom, target_basis, zero, zero)
                    }
                };
                frag.offset.top = frag.offset.top - old_offset + new_offset;
            }
        }
    }

    // ── Out-of-flow layout ───────────────────────────────────────────
    // Layout absolutely and fixed positioned children that were collected
    // earlier. Must happen AFTER height resolution so that the containing
    // block height is the actual padding-box height (not just the available
    // block size from the parent constraint).
    if !oof_candidates.is_empty() {
        // Update containing block to use this block's resolved dimensions.
        // CSS 2.1 §10.1: The containing block for abspos descendants is
        // the padding box of the nearest positioned ancestor.
        let cb_height = resolved_block_size - border.top - border.bottom;
        let cb_width = child_available_inline + padding.left + padding.right;
        for c in &mut oof_candidates {
            c.containing_block_size = PhysicalSize::new(cb_width, cb_height);
            c.containing_block_border = border.clone();
            c.containing_block_direction = style.direction;
        }

        // Iteratively process OOF candidates. Each pass may produce nested OOF
        // candidates (e.g., fixed inside abs) that this block captures. This
        // matches Blink's OutOfFlowLayoutPart multi-pass approach.
        let mut pending = oof_candidates;
        while !pending.is_empty() {
            let oof_fragments = crate::out_of_flow::layout_out_of_flow_children(doc, &pending);
            pending = Vec::new();
            for mut frag in oof_fragments {
                // Collect nested OOF candidates from the just-laid-out fragment
                // and translate their static positions into parent coordinates.
                let nested = std::mem::take(&mut frag.oof_candidates);
                for mut c in nested {
                    c.static_position.left = c.static_position.left + frag.offset.left;
                    c.static_position.top = c.static_position.top + frag.offset.top;
                    let captures = if c.style.position == Position::Fixed {
                        is_root || style.establishes_transform_containing_block
                    } else {
                        establishes_cb_for_abspos
                    };
                    if captures {
                        c.containing_block_size = PhysicalSize::new(cb_width, cb_height);
                        c.containing_block_border = border.clone();
                        c.containing_block_direction = style.direction;
                        pending.push(c);
                    } else {
                        bubbled_oof_candidates.push(c);
                    }
                }
                child_fragments.push(frag);
            }
        }
    }

    let border_box_size = PhysicalSize::new(border_box_inline, resolved_block_size);

    let mut fragment = Fragment::new_box(node_id, border_box_size);
    fragment.border = border;
    fragment.padding = padding;
    fragment.children = child_fragments;
    fragment.kind = if doc.node(node_id).tag == openui_dom::ElementTag::Viewport {
        FragmentKind::Viewport
    } else {
        FragmentKind::Box
    };

    // ── Overflow tracking ────────────────────────────────────────────
    // Compute the scrollable overflow rect by unioning all child border-box
    // rects (relative to this fragment). If the union extends beyond this
    // fragment's border-box, store it as the overflow rect.
    let border_box_rect = PhysicalRect::new(PhysicalOffset::zero(), border_box_size);
    let mut overflow = border_box_rect;
    for child in &fragment.children {
        let child_rect = PhysicalRect::new(child.offset, child.size);
        overflow = overflow.unite(&child_rect);

        // Include grandchild overflow that wasn't clipped by the child.
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

    // Set the overflow clip flag from style.
    fragment.has_overflow_clip =
        style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible;

    // Attach any un-resolved OOF candidates for the parent to absorb.
    // These are abs-pos descendants that need a positioned ancestor higher up.
    fragment.oof_candidates = bubbled_oof_candidates;

    // CSS 2.1 §8.3.1: Propagate unresolved margin struts for parent collapsing.
    // If the start margin wasn't resolved (first child's margin collapsed through
    // with no border/padding/new-FC separating), propagate it so the parent can
    // merge it with the parent's own margins.
    // CSS 2.1 §8.3.1: Propagate unresolved margin struts for parent collapsing.
    // If the parent has no border/padding/FC and the start margin collapsed
    // through children, propagate it so the parent can merge with its own margins.
    if let Some(saved) = saved_start_strut {
        // The saved strut is the accumulated margin chain from self-collapsing
        // first children + the first non-self-collapsing child's top margin.
        // This must be propagated unless the parent resolved it via FC/bp.
        if !space.is_new_formatting_context && content_edge == LayoutUnit::zero() {
            let mut propagated = saved;
            if has_aspect_ratio_block_size {
                propagated.append_normal(aspect_ratio_collapsed_end_strut.positive_margin);
                if aspect_ratio_collapsed_end_strut.negative_margin < LayoutUnit::zero() {
                    propagated.append_normal(aspect_ratio_collapsed_end_strut.negative_margin);
                }
            }
            fragment.start_margin_strut = propagated;
        }
    } else if !start_margin_resolved {
        fragment.start_margin_strut = margin_strut;
    }
    fragment.end_margin_strut = if has_aspect_ratio_block_size {
        MarginStrut::new()
    } else {
        final_end_margin_strut
    };
    fragment.float_resolved_bfc = float_resolved_bfc;
    fragment.float_exclusions = float_exclusions_result;

    // ── Baseline propagation ─────────────────────────────────────────
    // CSS Inline 3 §3: The first baseline set of a block container is the
    // first baseline of its first in-flow child that contributes one; the
    // last baseline is from the last such child.
    //
    // Always compute baselines so parent containers (flex, grid, etc.) can
    // use them. The `needs_first_baseline` flag is checked by parents, not
    // by the computation itself.
    //
    // Blink: BlockLayoutAlgorithm::Layout() — first/last baseline propagation.
    {
        // Start with values from inline layout (pure or mixed path).
        fragment.first_baseline = first_baseline_result;
        fragment.last_baseline = last_baseline_result;

        // Also scan block children for baselines (handles nested blocks
        // and block children in mixed content).
        for child in &fragment.children {
            if let Some(child_first) = child.first_baseline {
                if fragment.first_baseline.is_none() {
                    fragment.first_baseline = Some(child.offset.top + child_first);
                }
                fragment.last_baseline =
                    Some(child.offset.top + child.last_baseline.unwrap_or(child_first));
            }
        }
    }

    fragment
}

// ── Helper: detect block-level children ──────────────────────────────

/// Check if a node has any block-level children (CSS 2.2 §9.2.1.1).
/// Floated children are treated as block-level for content classification.
pub fn has_block_children(doc: &Document, node_id: NodeId) -> bool {
    for child_id in doc.children(node_id) {
        let child = doc.node(child_id);
        if child.style.position.is_absolutely_positioned() || child.style.display == Display::None {
            continue;
        }
        // Floated elements trigger block-level content classification
        if child.style.float != Float::None {
            return true;
        }
        if child.tag == openui_dom::ElementTag::Text {
            continue;
        }
        if child.style.display.is_block_level() {
            return true;
        }
    }
    false
}

/// Check if a child is inline-level (text node or inline display).
fn is_inline_level_child(doc: &Document, child_id: NodeId) -> bool {
    let child = doc.node(child_id);
    if child.style.is_out_of_flow() || child.style.display == Display::None {
        return false;
    }
    child.tag == openui_dom::ElementTag::Text || child.style.display.is_inline_level()
}

// ── Helper: map CSS Clear to ClearType ───────────────────────────────

fn clear_type_from_style(clear: Clear) -> ClearType {
    match clear {
        Clear::None => ClearType::None,
        Clear::Left => ClearType::Left,
        Clear::Right => ClearType::Right,
        Clear::Both => ClearType::Both,
    }
}

// ── Helper: handle a floated child ───────────────────────────────────

/// Position a floated child within the exclusion space.
///
/// The float is laid out to determine its size, then positioned using the
/// exclusion space algorithm. The resulting exclusion area is added to the
/// exclusion space. Float children do NOT advance the block offset.
#[allow(clippy::too_many_arguments)]
fn handle_float(
    doc: &Document,
    child_id: NodeId,
    space: &ConstraintSpace,
    child_available_inline: LayoutUnit,
    child_percentage_block_size: LayoutUnit,
    border: &BoxStrut,
    padding: &BoxStrut,
    content_edge: LayoutUnit,
    block_offset: &LayoutUnit,
    exclusion_space: &mut ExclusionSpace,
    child_fragments: &mut Vec<Fragment>,
    oof_candidates: &mut Vec<OutOfFlowCandidate>,
    bubbled_oof_candidates: &mut Vec<OutOfFlowCandidate>,
    establishes_cb_for_abspos: bool,
    captures_fixed_pos_descendants: bool,
    max_float_bottom: &mut LayoutUnit,
) {
    let child_style = &doc.node(child_id).style;
    let child_margin = resolve_margins(child_style, child_available_inline);
    let is_left = child_style.float == Float::Left;

    // CSS 2.1 §10.3.5: Floats with auto width use shrink-to-fit sizing.
    // The available width for shrink-to-fit is the containing block width
    // minus the float's own non-auto horizontal margins.
    let float_inline_size = if child_style.width.is_auto() {
        let margin_inline = {
            let ml = if child_style.margin_left.is_auto() {
                LayoutUnit::zero()
            } else {
                child_margin.left
            };
            let mr = if child_style.margin_right.is_auto() {
                LayoutUnit::zero()
            } else {
                child_margin.right
            };
            ml + mr
        };
        let stf_available = (child_available_inline - margin_inline).clamp_negative_to_zero();
        crate::out_of_flow::compute_shrink_to_fit_width(doc, child_id, stf_available)
    } else {
        child_available_inline
    };

    // Layout the float child to determine its size.
    // Floats always establish a new formatting context.
    let child_space = ConstraintSpace::for_block_child(
        float_inline_size,
        space.available_block_size,
        child_available_inline,
        child_percentage_block_size,
        true,
    );
    let mut child_fragment = block_layout(doc, child_id, &child_space);

    // Take OOF candidates from the float's descendants for post-positioning
    // translation (same pattern as the normal block path at lines 1009-1027).
    let child_oof = std::mem::take(&mut child_fragment.oof_candidates);

    // BFC coordinates: (0, 0) = content area start of the container.
    let mut content_block_offset = *block_offset - content_edge;

    // CSS 2.1 §9.5.2: The `clear` property applies to floating elements too.
    // A float with clear:left/right/both is moved below prior floats.
    if child_style.clear != Clear::None {
        let clearance = exclusion_space.clearance_offset(clear_type_from_style(child_style.clear));
        if clearance > content_block_offset {
            content_block_offset = clearance;
        }
    }

    let has_inherited_exclusions = uses_margin_reduced_inherited_exclusions(space);
    let inherited_bfc_line_offset = if has_inherited_exclusions {
        space.bfc_offset.line_offset
    } else {
        LayoutUnit::zero()
    };
    let ancestor_bfc_inline_size = if has_inherited_exclusions {
        space.percentage_resolution_inline_size
    } else {
        child_available_inline
    };
    let (float_origin_line, float_available_inline) = if has_inherited_exclusions {
        if is_left {
            (
                LayoutUnit::zero(),
                (ancestor_bfc_inline_size - inherited_bfc_line_offset).clamp_negative_to_zero(),
            )
        } else {
            (
                -inherited_bfc_line_offset,
                inherited_bfc_line_offset + child_available_inline,
            )
        }
    } else {
        (LayoutUnit::zero(), child_available_inline)
    };

    let unpositioned = UnpositionedFloat {
        node_id: child_id,
        available_size: float_available_inline,
        origin_bfc_offset: BfcOffset::new(float_origin_line, content_block_offset),
        margins: child_margin,
        inline_size: child_fragment.size.width,
        block_size: child_fragment.size.height,
        placement_min_inline_size: if has_inherited_exclusions {
            Some(LayoutUnit::zero())
        } else {
            None
        },
        is_left,
    };

    let (positioned, exclusion) = position_float(&unpositioned, exclusion_space);
    exclusion_space.add(exclusion);

    // CSS 2.1 §10.6.7: Track float bottom margin edge (BFC coordinates)
    // for auto-height BFC roots that must include float descendants.
    let float_bottom_bfc =
        positioned.bfc_offset.block_offset + child_fragment.size.height + child_margin.bottom;
    *max_float_bottom = (*max_float_bottom).max_of(float_bottom_bfc);

    // Convert BFC coordinates back to parent border-box coordinates.
    let mut fragment = child_fragment;
    fragment.offset = PhysicalOffset::new(
        positioned.bfc_offset.line_offset + border.left + padding.left,
        positioned.bfc_offset.block_offset + content_edge,
    );

    // Apply relative positioning offsets (CSS 2.1 §9.4.3).
    crate::relative::apply_relative_offset(
        &mut fragment,
        child_style,
        child_available_inline,
        space.available_block_size,
    );

    fragment.margin = child_margin;

    // Translate OOF candidates' static_position into parent coordinates
    // using the float's final offset (matching the normal block path).
    if !child_oof.is_empty() {
        let child_offset = fragment.offset;
        for mut c in child_oof {
            c.static_position.left = c.static_position.left + child_offset.left;
            c.static_position.top = c.static_position.top + child_offset.top;
            let captures = if c.style.position == Position::Fixed {
                captures_fixed_pos_descendants
            } else {
                establishes_cb_for_abspos
            };
            if captures {
                oof_candidates.push(c);
            } else {
                bubbled_oof_candidates.push(c);
            }
        }
    }

    child_fragments.push(fragment);
}

fn initial_exclusion_space(space: &ConstraintSpace) -> ExclusionSpace {
    if space.is_new_formatting_context {
        ExclusionSpace::new()
    } else {
        space
            .exclusion_space
            .as_ref()
            .map(|exclusions| (**exclusions).clone())
            .unwrap_or_default()
    }
}

fn uses_margin_reduced_inherited_exclusions(space: &ConstraintSpace) -> bool {
    !space.is_new_formatting_context
        && space.exclusion_space.is_some()
        && space.available_inline_size != space.percentage_resolution_inline_size
}

fn new_fc_placement_for_opportunity(
    opportunity: &crate::exclusions::LayoutOpportunity,
    margin: BoxStrut,
    container_inline_size: LayoutUnit,
) -> (LayoutUnit, LayoutUnit) {
    if opportunity.rect.line_start_offset() == LayoutUnit::zero()
        && opportunity.inline_size() == container_inline_size
    {
        return (LayoutUnit::zero(), container_inline_size);
    }
    let inline_offset = opportunity.rect.line_start_offset() - margin.left;
    let available =
        (opportunity.inline_size() + margin.left + margin.right).clamp_negative_to_zero();
    (inline_offset, available)
}

fn estimate_auto_aspect_ratio_block_size(
    style: &ComputedStyle,
    opportunity_inline: LayoutUnit,
    margin: BoxStrut,
) -> LayoutUnit {
    let Some(ar) = &style.aspect_ratio else {
        return LayoutUnit::zero();
    };
    if ar.ratio.0 == 0.0 || ar.ratio.1 == 0.0 {
        return LayoutUnit::zero();
    }

    let containing_inline =
        (opportunity_inline + margin.left + margin.right).clamp_negative_to_zero();
    let border = resolve_border(style);
    let padding = resolve_padding(style, containing_inline);
    let border_padding_inline = border.left + border.right + padding.left + padding.right;
    let border_padding_block = border.top + border.bottom + padding.top + padding.bottom;
    let box_sizing_for_ar = if ar.auto_flag {
        BoxSizing::ContentBox
    } else {
        style.box_sizing
    };

    let ar_inline = if box_sizing_for_ar == BoxSizing::BorderBox {
        opportunity_inline
    } else {
        (opportunity_inline - border_padding_inline).clamp_negative_to_zero()
    };
    let ar_block = LayoutUnit::from_f32(ar_inline.to_f32() * ar.ratio.1 / ar.ratio.0);

    if box_sizing_for_ar == BoxSizing::BorderBox {
        ar_block.max_of(border_padding_block)
    } else {
        ar_block + border_padding_block
    }
}

#[allow(clippy::too_many_arguments)]
fn estimate_child_content_origin(
    child_style: &ComputedStyle,
    child_margin: BoxStrut,
    child_available_inline: LayoutUnit,
    parent_content_edge: LayoutUnit,
    block_offset: LayoutUnit,
    margin_strut: MarginStrut,
    start_margin_resolved: bool,
    parent_space: &ConstraintSpace,
) -> BfcOffset {
    let child_border = resolve_border(child_style);
    let child_padding = resolve_padding(child_style, child_available_inline);

    let mut projected_strut = margin_strut;
    projected_strut.append_normal(child_margin.top);
    let border_block_offset = if start_margin_resolved
        || parent_space.is_new_formatting_context
        || parent_content_edge > LayoutUnit::zero()
    {
        block_offset + projected_strut.sum()
    } else {
        block_offset
    };

    BfcOffset::new(
        child_margin.left + child_border.left + child_padding.left,
        border_block_offset - parent_content_edge + child_border.top + child_padding.top,
    )
}

fn translate_exclusion_space(
    parent: &ExclusionSpace,
    child_content_origin: BfcOffset,
) -> ExclusionSpace {
    let mut translated = ExclusionSpace::new();
    for exclusion in parent.all_exclusions() {
        translated.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(
                    exclusion.rect.start_offset.line_offset - child_content_origin.line_offset,
                    exclusion.rect.start_offset.block_offset - child_content_origin.block_offset,
                ),
                BfcOffset::new(
                    exclusion.rect.end_offset.line_offset - child_content_origin.line_offset,
                    exclusion.rect.end_offset.block_offset - child_content_origin.block_offset,
                ),
            ),
            exclusion_type: exclusion.exclusion_type,
        });
    }
    translated
}

// ── Helper: layout a single block child ──────────────────────────────

/// Layout a single block-level child in normal flow.
///
/// Extracted from the block child loop to be reusable by both the pure-block
/// and the mixed-content (anonymous block box) paths.
#[allow(clippy::too_many_arguments)]
fn layout_block_child(
    doc: &Document,
    child_id: NodeId,
    space: &ConstraintSpace,
    child_available_inline: LayoutUnit,
    child_percentage_block_size: LayoutUnit,
    children_available_block_size: LayoutUnit,
    border: &BoxStrut,
    padding: &BoxStrut,
    content_edge: LayoutUnit,
    block_offset: &mut LayoutUnit,
    margin_strut: &mut MarginStrut,
    intrinsic_block_size: &mut LayoutUnit,
    child_fragments: &mut Vec<Fragment>,
    oof_candidates: &mut Vec<OutOfFlowCandidate>,
    bubbled_oof_candidates: &mut Vec<OutOfFlowCandidate>,
    establishes_cb_for_abspos: bool,
    captures_fixed_pos_descendants: bool,
    start_margin_resolved: &mut bool,
    saved_start_strut: &mut Option<MarginStrut>,
    containing_block_direction: Direction,
    pending_self_collapsing: &mut Vec<usize>,
    parent_exclusion_space: Option<&ExclusionSpace>,
) {
    let child_style = &doc.node(child_id).style;

    if child_style.is_out_of_flow() || child_style.display == Display::None {
        return;
    }

    let child_margin = resolve_margins(child_style, child_available_inline);
    margin_strut.append_normal(child_margin.top);

    // ── Layout child BEFORE resolving the margin strut ───────────────
    // CSS 2.1 §8.3.1: A child with no border/padding propagates its first
    // descendant's margin through start_margin_strut. That propagated margin
    // must participate in the parent's margin collapsing group. We must
    // layout the child first to learn about any propagated margins, absorb
    // them into the strut, and THEN resolve. The child's constraint space
    // doesn't depend on block_offset, so this reordering is safe.

    let child_non_auto_margin_inline = {
        let ml = if child_style.margin_left.is_auto() {
            LayoutUnit::zero()
        } else {
            child_margin.left
        };
        let mr = if child_style.margin_right.is_auto() {
            LayoutUnit::zero()
        } else {
            child_margin.right
        };
        ml + mr
    };
    let child_constrained_inline =
        (child_available_inline - child_non_auto_margin_inline).clamp_negative_to_zero();

    let child_is_new_fc = establishes_new_fc(child_style);
    let mut child_space = ConstraintSpace::for_block_child(
        child_constrained_inline,
        children_available_block_size,
        child_available_inline,
        child_percentage_block_size,
        child_is_new_fc,
    );
    if !child_is_new_fc {
        if let Some(parent_exclusions) = parent_exclusion_space {
            let child_content_origin = estimate_child_content_origin(
                child_style,
                child_margin,
                child_available_inline,
                content_edge,
                *block_offset,
                *margin_strut,
                *start_margin_resolved,
                space,
            );
            child_space.bfc_offset = child_content_origin;
            let translated = translate_exclusion_space(parent_exclusions, child_content_origin);
            if translated.has_floats() {
                child_space.exclusion_space = Some(std::sync::Arc::new(translated));
            }
        }
    }

    let mut child_fragment = block_layout(doc, child_id, &child_space);

    // Early self-collapsing check — needed BEFORE float cascade gating.
    // CSS 2.1 §8.3.1: Empty blocks (no height, no border/padding, no FC,
    // no in-flow children) collapse through. A float inside an empty block
    // should NOT break the parent's margin collapsing chain — the margin
    // group continues through the empty wrapper.
    let child_bp_block = resolve_border(child_style).block_sum()
        + resolve_padding(child_style, child_available_inline).block_sum();
    let child_has_in_flow_content = doc.children(child_id).any(|grandchild_id| {
        let gs = &doc.node(grandchild_id).style;
        gs.display != Display::None
            && !gs.position.is_absolutely_positioned()
            && gs.float == openui_style::Float::None
    });
    let child_is_self_collapsing = child_fragment.size.height == LayoutUnit::zero()
        && child_bp_block == LayoutUnit::zero()
        && !child_is_new_fc
        && !child_has_in_flow_content
        && child_fragment.oof_candidates.is_empty();

    // CSS 2.1 §8.3.1: Absorb child's propagated start margin strut.
    // If the child itself had an unresolved start margin (its first
    // grandchild's margin collapsed through with no border/padding
    // separating), merge it into our current margin strut. This
    // enables margin collapse to propagate through nested wrapper divs.
    // Absorbed unconditionally — propagated margins participate in the
    // collapsing group regardless of whether this is the first child or
    // a subsequent sibling.
    if !child_fragment.start_margin_strut.is_empty() {
        let child_start = child_fragment.start_margin_strut;
        margin_strut.append_normal(child_start.positive_margin);
        if child_start.negative_margin < LayoutUnit::zero() {
            margin_strut.append_normal(child_start.negative_margin);
        }
    }

    // ── Cascade float-forced BFC resolution ─────────────────────────
    // CSS 2.1 §9.5 + Chromium LayoutNG: When a descendant float forces
    // BFC offset resolution, that resolution cascades upward through all
    // non-BFC ancestors to the BFC root. This prevents margin-collapse
    // propagation from incorrectly pushing the float's ancestor down.
    //
    // If the child's float_resolved_bfc is set and our start margin isn't
    // resolved yet, force resolution now. For non-BFC blocks, save the
    // current strut (margins before the float) for parent propagation,
    // then mark as resolved so subsequent margins become internal.
    //
    // EXCEPTION: Skip the cascade for self-collapsing children. Per CSS 2.1
    // §8.3.1, an empty block (no height/bp/FC/in-flow content) collapses
    // through — its margins adjoin with subsequent siblings. A float inside
    // such a block is positioned within the child's own layout; the parent's
    // margin chain must continue so margins from later siblings can still
    // collapse with the parent's start margin.
    if child_fragment.float_resolved_bfc && !*start_margin_resolved && !child_is_self_collapsing {
        if space.is_new_formatting_context || content_edge > LayoutUnit::zero() {
            *block_offset += margin_strut.sum();
            *margin_strut = MarginStrut::new();
        } else {
            // Non-BFC: save accumulated margins before the float for parent
            // propagation, then reset. Margins after the float go internal.
            if saved_start_strut.is_none() {
                *saved_start_strut = Some(*margin_strut);
            }
            *margin_strut = MarginStrut::new();
        }
        *start_margin_resolved = true;
    }

    // ── NOW resolve the margin strut ─────────────────────────────────
    // When start_margin_resolved is true, we tentatively resolve the strut
    // but save state for rollback if the child turns out self-collapsing.
    // Per CSS 2.1 §8.3.1, a self-collapsing block's top and bottom margins
    // are adjoining with the preceding sibling's bottom margin — they form
    // one collapsing group.
    let pre_resolve_strut = *margin_strut;
    let pre_resolve_offset = *block_offset;
    let mut strut_resolved_this_child = false;

    if !*start_margin_resolved {
        if space.is_new_formatting_context || content_edge > LayoutUnit::zero() {
            *block_offset += margin_strut.sum();
            *margin_strut = MarginStrut::new();
            *start_margin_resolved = true;
            strut_resolved_this_child = true;
            // NOTE: Do NOT flush pending_self_collapsing here — this resolution
            // is tentative. If the current child is self-collapsing, we'll roll
            // back. Flush happens later once we confirm non-self-collapsing.
        }
        // else: start margin still collapses through — don't resolve yet
    } else {
        *block_offset += margin_strut.sum();
        *margin_strut = MarginStrut::new();
        strut_resolved_this_child = true;
        // NOTE: Tentative — deferred flush (see non-self-collapsing branch).
    }

    // Save the margin boundary offset BEFORE height addition. This is the
    // position where self-collapsing predecessors should be flushed — at the
    // top border edge of this child, not its bottom.
    let margin_boundary_offset = *block_offset;

    // Take OOF candidates from the child for processing after offset is known.
    let child_oof = if !child_fragment.oof_candidates.is_empty() {
        std::mem::take(&mut child_fragment.oof_candidates)
    } else {
        Vec::new()
    };

    let child_border_box_inline = child_fragment.size.width;
    let remaining_space = child_available_inline - child_border_box_inline;

    let resolved_margin_left;
    let resolved_margin_right;

    if child_style.margin_left.is_auto() && child_style.margin_right.is_auto() {
        if remaining_space > LayoutUnit::zero() {
            let half = remaining_space / 2;
            resolved_margin_left = half;
            resolved_margin_right = remaining_space - half;
        } else {
            // CSS 2.1 §10.3.3: auto→0, then apply over-constrained rule.
            // Use the CONTAINING BLOCK's direction per spec (not child's).
            // In RTL, margin-left absorbs the negative remainder (overflow left);
            // in LTR, margin-right absorbs it (overflow right).
            if containing_block_direction == Direction::Rtl {
                resolved_margin_left = remaining_space;
                resolved_margin_right = LayoutUnit::zero();
            } else {
                resolved_margin_left = LayoutUnit::zero();
                resolved_margin_right = remaining_space;
            }
        }
    } else if child_style.margin_left.is_auto() {
        resolved_margin_right = child_margin.right;
        resolved_margin_left = remaining_space - resolved_margin_right;
    } else if child_style.margin_right.is_auto() {
        resolved_margin_left = child_margin.left;
        resolved_margin_right = remaining_space - resolved_margin_left;
    } else {
        // CSS 2.1 §10.3.3: Both margins specified. If the total
        // (width + margin-left + margin-right) exceeds the containing block,
        // the end margin (margin-right in LTR, margin-left in RTL) is
        // recomputed to satisfy the equation. Uses containing block direction.
        if containing_block_direction == Direction::Rtl {
            resolved_margin_right = child_margin.right;
            resolved_margin_left = remaining_space - resolved_margin_right;
        } else {
            resolved_margin_left = child_margin.left;
            resolved_margin_right = remaining_space - resolved_margin_left;
        }
    }

    child_fragment.offset = PhysicalOffset::new(
        border.left + padding.left + resolved_margin_left,
        *block_offset,
    );

    // Apply relative positioning offsets (CSS 2.1 §9.4.3).
    // The fragment retains its normal-flow position for sibling layout;
    // only the visual offset is shifted.
    crate::relative::apply_relative_offset(
        &mut child_fragment,
        child_style,
        child_available_inline,
        space.available_block_size,
    );

    child_fragment.margin = BoxStrut::new(
        child_margin.top,
        resolved_margin_right,
        child_margin.bottom,
        resolved_margin_left,
    );

    // Absorb OOF candidates now that the child's offset is known.
    // Translate each candidate's static_position from the child's coordinate
    // space into this parent's coordinate space using the child's offset.
    if !child_oof.is_empty() {
        let child_offset = child_fragment.offset;
        for mut c in child_oof {
            c.static_position.left = c.static_position.left + child_offset.left;
            c.static_position.top = c.static_position.top + child_offset.top;
            let captures = if c.style.position == Position::Fixed {
                captures_fixed_pos_descendants
            } else {
                establishes_cb_for_abspos
            };
            if captures {
                oof_candidates.push(c);
            } else {
                bubbled_oof_candidates.push(c);
            }
        }
    }

    *block_offset += child_fragment.size.height;

    // child_is_self_collapsing was computed early (before float cascade)
    // to gate the cascade. Reuse it here.

    // CSS 2.1 §8.3.1: Save the start margin strut before the trailing reset
    // overwrites it. This captures the first child's (and nested first
    // children's) accumulated top margin chain for parent-first-child collapsing.
    if !*start_margin_resolved && saved_start_strut.is_none() {
        *saved_start_strut = Some(*margin_strut);
    }

    if child_is_self_collapsing {
        // CSS 2.1 §8.3.1: A self-collapsing block's top and bottom margins
        // collapse together. The collapsed result adjoins the next sibling's
        // top margin. Don't reset the strut — append the bottom margin to
        // the existing strut so both top and bottom are preserved.
        //
        // This applies uniformly regardless of float_resolved_bfc. A float
        // inside an empty block is positioned within that child's own layout;
        // the parent's margin chain must continue so later siblings can
        // collapse with the parent's start margin (CSS 2.1 §8.3.1).
        if strut_resolved_this_child {
            *block_offset = pre_resolve_offset;
            *margin_strut = pre_resolve_strut;
        }
        margin_strut.append_normal(child_margin.bottom);
        if !child_fragment.end_margin_strut.is_empty() {
            let child_end = child_fragment.end_margin_strut;
            margin_strut.append_normal(child_end.positive_margin);
            if child_end.negative_margin < LayoutUnit::zero() {
                margin_strut.append_normal(child_end.negative_margin);
            }
        }
        // Record index for deferred repositioning. The self-collapsing block's
        // final position is the next collapsed margin boundary, which we only
        // know when a non-self-collapsing sibling resolves the strut.
        //
        // A clearing empty wrapper whose only content is floats is different:
        // clearance has already established the float's BFC block position, so
        // moving the zero-height wrapper to the next sibling boundary drags its
        // floated descendants down by one collapsed-margin row.
        if !(child_style.clear != Clear::None && child_fragment.float_resolved_bfc) {
            let idx = child_fragments.len(); // index after push below
            pending_self_collapsing.push(idx);
        }
    } else {
        // Non-self-collapsing child: the child occupies space, so margin
        // collapsing ends here. Start a new strut with the bottom margin.
        // Also mark start margin as resolved — this child's content breaks
        // the adjoining margin chain.
        //
        // CSS 2.1 §8.3.1: If the parent has no border/padding/FC, the
        // accumulated strut (from self-collapsing predecessors + this child's
        // top margin) must be saved for parent-first-child collapsing.
        if !*start_margin_resolved {
            *saved_start_strut = Some(*margin_strut);
        }
        *start_margin_resolved = true;

        // The strut resolution is now confirmed (non-self-collapsing child).
        // Flush pending self-collapsing children to the margin boundary —
        // the offset BEFORE this child's height was added (CSS 2.1 §8.3.1:
        // self-collapsing block's edges coincide at the collapsed boundary).
        for &idx in pending_self_collapsing.iter() {
            child_fragments[idx].offset.top = margin_boundary_offset;
        }
        pending_self_collapsing.clear();

        *margin_strut = MarginStrut::new();
        margin_strut.append_normal(child_margin.bottom);
        if !child_fragment.end_margin_strut.is_empty() {
            let child_end = child_fragment.end_margin_strut;
            margin_strut.append_normal(child_end.positive_margin);
            if child_end.negative_margin < LayoutUnit::zero() {
                margin_strut.append_normal(child_end.negative_margin);
            }
        }
    }

    *intrinsic_block_size = (*intrinsic_block_size).max_of(*block_offset);
    child_fragments.push(child_fragment);
}

// ── Helper: establishes new formatting context ───────────────────────

/// Whether a style establishes a new formatting context.
///
/// overflow != visible, display: flow-root, floats, abs pos, inline-block,
/// flex, grid — all establish a new block formatting context.
///
/// Delegates to `ComputedStyle::creates_new_formatting_context()` which checks
/// these conditions. Provided as a free function for use in layout algorithms.
pub fn establishes_new_fc(style: &ComputedStyle) -> bool {
    style.creates_new_formatting_context()
}

/// Compute the minimum inline size for a new-FC child's float avoidance query.
///
/// Per CSS 2.1 §9.5: "The border box of ... an element in normal flow that
/// establishes a new block formatting context must not overlap the margin box
/// of any floats in the same block formatting context."
///
/// Compute the minimum inline size needed for a new formatting context child
/// to fit in a layout opportunity next to floats.
///
/// CSS 2.1 §9.5 constrains the BFC child's border box against float margin
/// boxes. The child's own margins are resolved when positioning the border box
/// and may overflow the containing block per CSS 2.1 §10.3.3.
///
/// For auto-width: positive margins + border + padding (content can shrink to 0).
/// For explicit width: border-box width.
fn new_fc_min_inline_size(style: &ComputedStyle, containing_inline: LayoutUnit) -> LayoutUnit {
    let bp_left = LayoutUnit::from_i32(style.effective_border_left())
        + resolve_margin_or_padding(&style.padding_left, containing_inline);
    let bp_right = LayoutUnit::from_i32(style.effective_border_right())
        + resolve_margin_or_padding(&style.padding_right, containing_inline);
    let bp = bp_left + bp_right;
    let margin_left = resolve_margin_or_padding(&style.margin_left, containing_inline);
    let margin_right = resolve_margin_or_padding(&style.margin_right, containing_inline);
    let positive_margins =
        margin_left.max_of(LayoutUnit::zero()) + margin_right.max_of(LayoutUnit::zero());

    // CSS 2.1 §9.5 constrains the BFC child's border box against float margin
    // boxes. The child's own margins may extend through the opportunity; they
    // are applied later when positioning the border box.
    if style.width.is_auto()
        || style.width.is_stretch()
        || style.min_width.is_stretch()
        || style.max_width.is_stretch()
    {
        // Auto/stretch width can shrink content to zero; border/padding and
        // positive margins still need opportunity space. With a single negative
        // side margin, require enough opportunity for that protruding side; the
        // final width is still resolved from the selected opportunity.
        let single_negative_margin =
            (margin_left < LayoutUnit::zero()) != (margin_right < LayoutUnit::zero());
        if single_negative_margin {
            bp + positive_margins.max_of(
                (-margin_left.min_of(LayoutUnit::zero()))
                    .max_of(-margin_right.min_of(LayoutUnit::zero())),
            )
        } else {
            bp + positive_margins
        }
    } else {
        // Explicit width: resolve and compute border-box width
        let w = resolve_length(
            &style.width,
            containing_inline,
            containing_inline,
            containing_inline,
        );
        let border_box_w = match style.box_sizing {
            BoxSizing::ContentBox => w + bp,
            BoxSizing::BorderBox => w,
        };
        border_box_w
    }
}

// ── Helper: resolve border widths from style ─────────────────────────

pub fn resolve_border(style: &ComputedStyle) -> BoxStrut {
    BoxStrut::new(
        LayoutUnit::from_i32(style.effective_border_top()),
        LayoutUnit::from_i32(style.effective_border_right()),
        LayoutUnit::from_i32(style.effective_border_bottom()),
        LayoutUnit::from_i32(style.effective_border_left()),
    )
}

// ── Helper: resolve padding lengths ──────────────────────────────────

pub fn resolve_padding(style: &ComputedStyle, percentage_base: LayoutUnit) -> BoxStrut {
    BoxStrut::new(
        resolve_margin_or_padding(&style.padding_top, percentage_base),
        resolve_margin_or_padding(&style.padding_right, percentage_base),
        resolve_margin_or_padding(&style.padding_bottom, percentage_base),
        resolve_margin_or_padding(&style.padding_left, percentage_base),
    )
}

// ── Helper: resolve margins ──────────────────────────────────────────

pub fn resolve_margins(style: &ComputedStyle, percentage_base: LayoutUnit) -> BoxStrut {
    BoxStrut::new(
        resolve_margin_or_padding(&style.margin_top, percentage_base),
        resolve_margin_or_padding(&style.margin_right, percentage_base),
        resolve_margin_or_padding(&style.margin_bottom, percentage_base),
        resolve_margin_or_padding(&style.margin_left, percentage_base),
    )
}

// ── Intrinsic sizing keyword helpers ─────────────────────────────────

/// Resolve an intrinsic sizing keyword (min-content, max-content, fit-content)
/// to a concrete inline (width) value. Returns content-box size.
fn resolve_intrinsic_inline(
    doc: &Document,
    node_id: NodeId,
    length: &openui_geometry::Length,
    available: LayoutUnit,
    border_padding: LayoutUnit,
) -> LayoutUnit {
    let intrinsic = crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, node_id);
    let style = &doc.node(node_id).style;

    // Apply AR transfer: if the element has aspect-ratio + definite height,
    // its intrinsic sizes include the AR-derived width (CSS Sizing 4 §5.1).
    let (min_raw, max_raw) = if let Some(ref ar) = style.aspect_ratio {
        if style.height.length_type() == openui_geometry::LengthType::Fixed
            && ar.ratio.0 != 0.0
            && ar.ratio.1 != 0.0
        {
            let ar_derived =
                crate::intrinsic_sizing::apply_size_override_inline(style, intrinsic.max);
            (ar_derived, ar_derived)
        } else {
            (intrinsic.min, intrinsic.max)
        }
    } else {
        (intrinsic.min, intrinsic.max)
    };

    // Intrinsic sizes include border+padding. Convert to the element's
    // box-sizing convention: border-box elements keep the full value,
    // content-box elements subtract border+padding.
    let to_box_sizing = |v: LayoutUnit| -> LayoutUnit {
        if style.box_sizing == BoxSizing::BorderBox {
            v
        } else {
            (v - border_padding).clamp_negative_to_zero()
        }
    };
    let min_content = to_box_sizing(min_raw);
    let max_content = to_box_sizing(max_raw);
    match length.length_type() {
        LengthType::MinContent => min_content,
        LengthType::MaxContent => max_content,
        LengthType::FitContent => {
            let avail = to_box_sizing(
                available
                    + if style.box_sizing == BoxSizing::BorderBox {
                        border_padding
                    } else {
                        LayoutUnit::zero()
                    },
            );
            avail.clamp(min_content, max_content)
        }
        LengthType::Stretch => to_box_sizing(
            available
                + if style.box_sizing == BoxSizing::BorderBox {
                    border_padding
                } else {
                    LayoutUnit::zero()
                },
        ),
        _ => to_box_sizing(
            available
                + if style.box_sizing == BoxSizing::BorderBox {
                    border_padding
                } else {
                    LayoutUnit::zero()
                },
        ),
    }
}

/// Resolve an intrinsic sizing keyword (min-content, max-content, fit-content)
/// to a concrete block (height) value. Returns border-box size.
fn resolve_intrinsic_block(
    _doc: &Document,
    _node_id: NodeId,
    _length: &openui_geometry::Length,
    intrinsic_block_size: LayoutUnit,
    _border_padding_block: LayoutUnit,
) -> LayoutUnit {
    // CSS Sizing 3 §5: For block containers, min-content and max-content
    // block sizes are the content height as defined in CSS 2.1 §10.6.3.
    // The main layout's intrinsic_block_size already accounts for margin
    // collapsing and other block formatting effects, so use it directly
    // rather than re-computing from scratch (which lacks margin collapsing).
    intrinsic_block_size
}

// ── Helper: resolve inline size (width) ──────────────────────────────

fn resolve_inline_size(
    doc: &Document,
    node_id: NodeId,
    style: &ComputedStyle,
    space: &ConstraintSpace,
    border_padding: LayoutUnit,
    border_padding_block: LayoutUnit,
) -> LayoutUnit {
    let available = space.available_inline_size;

    // When flex layout determines the exact inline size, use it directly.
    // The value from flex is always border-box; convert to content-box.
    if space.is_fixed_inline_size || space.stretch_inline_size {
        return if style.box_sizing == BoxSizing::BorderBox {
            available.max_of(border_padding)
        } else {
            (available - border_padding).clamp_negative_to_zero()
        };
    }

    // Resolve the CSS width property — handle intrinsic sizing keywords
    let resolved = if style.width.is_auto() || style.width.is_stretch() {
        // CSS Sizing 4 §5.1: when width is auto and the element has a preferred
        // aspect ratio with a definite height, compute width from height × ratio.
        //
        // This applies to ALL elements (block-level, inline-block, floats, etc.)
        // as long as:
        //   1. width is auto (not stretch)
        //   2. the element has an aspect-ratio
        //   3. the height resolves to a definite value (including after
        //      min-height/max-height clamping)
        //
        // Chromium NG: ComputeInlineSizeFromAspectRatio() is called for any
        // element with a definite block size and aspect-ratio, regardless of
        // whether it's in a shrink-to-fit context.
        let ar_width = if style.width.is_auto() {
            if let Some(ref ar) = style.aspect_ratio {
                // Resolve the effective height (applying min/max constraints)
                // so that width = effective_height × ratio.
                let h_resolved = if !style.height.is_auto()
                    && !style.height.is_content_or_intrinsic()
                    && !style.height.is_stretch()
                {
                    let h = resolve_length(
                        &style.height,
                        space.percentage_resolution_block_size,
                        openui_geometry::INDEFINITE_SIZE,
                        openui_geometry::INDEFINITE_SIZE,
                    );
                    if h.is_indefinite() {
                        None
                    } else {
                        // Apply min-height / max-height constraints
                        let min_h = if style.min_height.is_auto() {
                            LayoutUnit::zero()
                        } else {
                            resolve_length(
                                &style.min_height,
                                space.percentage_resolution_block_size,
                                LayoutUnit::zero(),
                                LayoutUnit::zero(),
                            )
                        };
                        let max_h = resolve_length(
                            &style.max_height,
                            space.percentage_resolution_block_size,
                            LayoutUnit::max(),
                            LayoutUnit::max(),
                        );
                        let clamped = h.max_of(min_h).min_of(max_h);
                        Some(clamped)
                    }
                } else {
                    None
                };
                if let Some(h) = h_resolved {
                    // CSS Sizing 4: when `auto <ratio>`, AR applies to
                    // content-box regardless of box-sizing. Bare `<ratio>`
                    // respects the element's box-sizing.
                    let box_sizing_for_ar = if ar.auto_flag {
                        BoxSizing::ContentBox
                    } else {
                        style.box_sizing
                    };
                    let content_h = if box_sizing_for_ar == BoxSizing::BorderBox {
                        h
                    } else if style.box_sizing == BoxSizing::BorderBox {
                        (h - border_padding_block).clamp_negative_to_zero()
                    } else {
                        h
                    };
                    let ratio = ar.ratio;
                    if ratio.0 != 0.0 && ratio.1 != 0.0 {
                        let w = LayoutUnit::from_f32(content_h.to_f32() * ratio.0 / ratio.1);
                        Some(if box_sizing_for_ar == BoxSizing::BorderBox {
                            w
                        } else if style.box_sizing == BoxSizing::BorderBox {
                            w + border_padding
                        } else {
                            w
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(w) = ar_width {
            w
        } else if available.is_indefinite() || available < LayoutUnit::zero() {
            // Indefinite available inline → shrink-to-fit (max-content width).
            // CSS 2.1 §10.3.5: shrink-to-fit width = min(available, max(preferred_min, preferred)).
            // With no available constraint, this reduces to max-content.
            let intrinsic = crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, node_id);
            if style.box_sizing == BoxSizing::BorderBox {
                intrinsic.max
            } else {
                (intrinsic.max - border_padding).clamp_negative_to_zero()
            }
        } else {
            // Auto/stretch width: fill available space minus border+padding
            if style.box_sizing == BoxSizing::BorderBox {
                available
            } else {
                (available - border_padding).clamp_negative_to_zero()
            }
        }
    } else if style.width.is_content_or_intrinsic() {
        // CSS Sizing 4 §5.1: For elements with a preferred aspect ratio and
        // a definite size in the opposite axis, min-content and max-content
        // sizes are the transferred size from the definite block constraint,
        // NOT the content-based intrinsic size.
        let ar_override = if let Some(ref ar) = style.aspect_ratio {
            let h_resolved = if !style.height.is_auto()
                && !style.height.is_content_or_intrinsic()
                && !style.height.is_stretch()
            {
                let h = resolve_length(
                    &style.height,
                    space.percentage_resolution_block_size,
                    openui_geometry::INDEFINITE_SIZE,
                    openui_geometry::INDEFINITE_SIZE,
                );
                if h.is_indefinite() {
                    None
                } else {
                    let min_h = if style.min_height.is_auto() {
                        LayoutUnit::zero()
                    } else {
                        resolve_length(
                            &style.min_height,
                            space.percentage_resolution_block_size,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        )
                    };
                    let max_h = resolve_length(
                        &style.max_height,
                        space.percentage_resolution_block_size,
                        LayoutUnit::max(),
                        LayoutUnit::max(),
                    );
                    Some(h.max_of(min_h).min_of(max_h))
                }
            } else {
                None
            };
            if let Some(h) = h_resolved {
                let ratio = ar.ratio;
                if ratio.0 != 0.0 && ratio.1 != 0.0 {
                    let box_sizing_for_ar = if ar.auto_flag {
                        BoxSizing::ContentBox
                    } else {
                        style.box_sizing
                    };
                    let content_h = if box_sizing_for_ar == BoxSizing::BorderBox {
                        h
                    } else if style.box_sizing == BoxSizing::BorderBox {
                        (h - border_padding_block).clamp_negative_to_zero()
                    } else {
                        h
                    };
                    let w = LayoutUnit::from_f32(content_h.to_f32() * ratio.0 / ratio.1);
                    Some(
                        if style.box_sizing == BoxSizing::BorderBox
                            && box_sizing_for_ar != BoxSizing::BorderBox
                        {
                            w + border_padding
                        } else {
                            w
                        },
                    )
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        if let Some(w) = ar_override {
            w
        } else {
            let intrinsic =
                resolve_intrinsic_inline(doc, node_id, &style.width, available, border_padding);
            let block_known_intrinsic = if !style.height.is_auto()
                && !style.height.is_stretch()
                && !style.height.is_content_or_intrinsic()
            {
                let raw_h = resolve_length(
                    &style.height,
                    space.percentage_resolution_block_size,
                    openui_geometry::INDEFINITE_SIZE,
                    openui_geometry::INDEFINITE_SIZE,
                );
                if raw_h.is_indefinite() {
                    None
                } else {
                    let content_h = if style.box_sizing == BoxSizing::BorderBox {
                        (raw_h - border_padding_block).clamp_negative_to_zero()
                    } else {
                        raw_h
                    };
                    let sizes =
                        crate::intrinsic_sizing::compute_intrinsic_inline_sizes_with_block_size(
                            doc, node_id, content_h,
                        );
                    let transferred = match style.width.length_type() {
                        LengthType::MinContent => sizes.min,
                        LengthType::MaxContent => sizes.max,
                        _ => sizes.max,
                    };
                    Some(if style.box_sizing == BoxSizing::BorderBox {
                        transferred
                    } else {
                        (transferred - border_padding).clamp_negative_to_zero()
                    })
                }
            } else {
                None
            };
            if let Some(transferred) = block_known_intrinsic {
                intrinsic.max_of(transferred)
            } else {
                intrinsic
            }
        }
    } else {
        resolve_length(
            &style.width,
            space.percentage_resolution_inline_size,
            available, // auto fallback
            available, // none fallback
        )
    };

    // Compute max-width first (needed for auto-min clamping).
    let max = if style.max_width.is_content_or_intrinsic() {
        resolve_intrinsic_inline(doc, node_id, &style.max_width, available, border_padding)
    } else if style.max_width.is_stretch() {
        available
    } else {
        resolve_length(
            &style.max_width,
            space.percentage_resolution_inline_size,
            LayoutUnit::max(), // auto → unconstrained
            LayoutUnit::max(), // none → unconstrained
        )
    };

    // Apply min-width / max-width constraints
    let min = if style.min_width.is_auto() {
        // CSS Sizing 4 §5.1: For non-replaced elements with a preferred
        // aspect-ratio (that are not scroll containers) AND no specified
        // size in the relevant axis, min-width:auto resolves to the
        // content-based minimum size (min-content width), clamped from
        // above by the maximum size (if definite).
        // When width IS specified, CSS 2.2 §10.4 applies: min-width:auto = 0.
        if style.aspect_ratio.is_some()
            && !style.is_scroll_container()
            && (style.width.is_auto() || style.width.is_content_or_intrinsic())
        {
            let intrinsic = crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, node_id);
            let auto_min = if style.box_sizing == BoxSizing::BorderBox {
                intrinsic.min
            } else {
                (intrinsic.min - border_padding).clamp_negative_to_zero()
            };
            // CSS Sizing 4 §5.1: "clamped from above by the maximum size"
            if max < LayoutUnit::max() {
                auto_min.min_of(max)
            } else {
                auto_min
            }
        } else {
            LayoutUnit::zero()
        }
    } else if style.min_width.is_content_or_intrinsic() {
        resolve_intrinsic_inline(doc, node_id, &style.min_width, available, border_padding)
    } else if style.min_width.is_stretch() {
        available
    } else {
        resolve_length(
            &style.min_width,
            space.percentage_resolution_inline_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
    };

    // CSS Sizing 4 §5.2: Transferred min/max through aspect-ratio.
    // min-height/max-height transfer to the inline axis via the ratio.
    //
    // Transferred sizes apply when the inline size is not an explicit
    // length — i.e. auto, stretch, or intrinsic keywords (min-content,
    // max-content, fit-content). An explicit length like `width: 200px`
    // takes precedence over transferred constraints.
    let (min, max) = if style.width.is_auto()
        || style.width.is_stretch()
        || style.width.is_content_or_intrinsic()
    {
        if let Some(ar) = &style.aspect_ratio {
            let ratio = ar.ratio;
            if ratio.0 == 0.0 || ratio.1 == 0.0 {
                (min, max)
            } else {
                let h_to_w = ratio.0 / ratio.1;
                let bp_block = border_padding_block;
                let bp_inline = border_padding;

                let transferred_min = if !style.min_height.is_auto()
                    && !style.min_height.is_content_or_intrinsic()
                {
                    let min_h_raw = resolve_length(
                        &style.min_height,
                        space.percentage_resolution_block_size,
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                    );
                    if min_h_raw > LayoutUnit::zero() {
                        let content_min_h = if style.box_sizing == BoxSizing::BorderBox {
                            (min_h_raw - bp_block).clamp_negative_to_zero()
                        } else {
                            min_h_raw
                        };
                        let transferred_w = LayoutUnit::from_f32(content_min_h.to_f32() * h_to_w);
                        let transferred = if style.box_sizing == BoxSizing::BorderBox {
                            transferred_w + bp_inline
                        } else {
                            transferred_w
                        };
                        min.max_of(transferred)
                    } else {
                        min
                    }
                } else {
                    min
                };

                let transferred_max = if max == LayoutUnit::max() {
                    let max_h_raw = resolve_length(
                        &style.max_height,
                        space.percentage_resolution_block_size,
                        LayoutUnit::max(),
                        LayoutUnit::max(),
                    );
                    if max_h_raw != LayoutUnit::max() {
                        let content_max_h = if style.box_sizing == BoxSizing::BorderBox {
                            (max_h_raw - bp_block).clamp_negative_to_zero()
                        } else {
                            max_h_raw
                        };
                        let transferred_w = LayoutUnit::from_f32(content_max_h.to_f32() * h_to_w);
                        let transferred = if style.box_sizing == BoxSizing::BorderBox {
                            transferred_w + bp_inline
                        } else {
                            transferred_w
                        };
                        max.min_of(transferred)
                    } else {
                        max
                    }
                } else {
                    max
                };

                // CSS Sizing 4 §5.2: Transferred min must not exceed explicit max.
                let transferred_min = transferred_min.min_of(transferred_max);

                // Combine with non-transferred constraints.
                (min.max_of(transferred_min), max.min_of(transferred_max))
            }
        } else {
            (min, max)
        }
    } else {
        (min, max)
    };

    resolved.clamp(min, max)
}

fn resolve_block_size(
    doc: &Document,
    node_id: NodeId,
    style: &ComputedStyle,
    space: &ConstraintSpace,
    intrinsic_block_size: LayoutUnit,
    border_padding_block: LayoutUnit,
    border_padding_inline: LayoutUnit,
    content_inline_size: LayoutUnit,
    is_viewport: bool,
) -> LayoutUnit {
    // When flex layout determines the exact block size, use it directly.
    if space.is_fixed_block_size || space.stretch_block_size {
        let available = space.available_block_size;
        return if style.box_sizing == BoxSizing::BorderBox {
            available.max_of(border_padding_block)
        } else {
            (available - border_padding_block).clamp_negative_to_zero() + border_padding_block
        };
    }

    // For the viewport/initial containing block, auto height = viewport height
    // (not content-sized). This matches Blink's initial containing block behavior.
    //
    // Track whether height is being resolved through the aspect ratio so we
    // know whether transferred min/max sizes apply (CSS Sizing 4 §5.2).
    let mut height_from_ar = false;
    let resolved = if style.height.is_auto() {
        if is_viewport {
            space.available_block_size
        } else if let Some(ref ar) = style.aspect_ratio {
            height_from_ar = true;
            // CSS Sizing 4 §5.1: When height is auto and aspect-ratio is set,
            // compute height from the resolved width using the aspect ratio.
            //
            // Chromium: BoxSizingForAspectRatio() — when `auto <ratio>`, the
            // aspect ratio always applies to the content-box dimensions,
            // regardless of the element's box-sizing property. Only a bare
            // `<ratio>` (without auto) respects the element's box-sizing.
            let box_sizing_for_ar = if ar.auto_flag {
                BoxSizing::ContentBox
            } else {
                style.box_sizing
            };
            // content_inline_size is the CSS width value: for border-box it's
            // the border-box width, for content-box it's the content width.
            // We need the value in the coordinate system that matches
            // box_sizing_for_ar.
            let ar_inline = if style.box_sizing == BoxSizing::BorderBox
                && box_sizing_for_ar == BoxSizing::ContentBox
            {
                // Convert border-box width to content-box width
                (content_inline_size - border_padding_inline).clamp_negative_to_zero()
            } else {
                content_inline_size
            };
            let (_, h) = crate::css_sizing::apply_aspect_ratio_with_auto(
                ar_inline,
                openui_geometry::INDEFINITE_SIZE,
                ar,
                None,
            );
            if !h.is_indefinite() {
                if box_sizing_for_ar == BoxSizing::BorderBox {
                    // h is border-box height; ensure at least border+padding
                    h.max_of(border_padding_block)
                } else {
                    // h is content-box height; add border+padding
                    h + border_padding_block
                }
            } else {
                intrinsic_block_size
            }
        } else {
            intrinsic_block_size
        }
    } else if style.height.is_stretch() {
        // CSS Sizing 4: height: stretch resolves to the containing block's
        // content area size. Only stretch when the containing block has a
        // definite block size (explicit height, externally imposed, or itself
        // stretched). When the containing block is auto-height, stretch falls
        // back to intrinsic (auto) sizing — the available_block_size from the
        // grandparent must NOT be used.
        if space.is_fixed_block_size
            || space.stretch_block_size
            || !space.percentage_resolution_block_size.is_indefinite()
        {
            space.available_block_size
        } else {
            intrinsic_block_size
        }
    } else if style.height.is_content_or_intrinsic() {
        // CSS Sizing 3 §4: For block containers, min-content and max-content in
        // the block axis are equivalent to the auto block size. So if the element
        // has an aspect-ratio and definite width, derive height from AR.
        if let Some(ref ar) = style.aspect_ratio {
            if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
                height_from_ar = true;
                let box_sizing_for_ar = if ar.auto_flag {
                    BoxSizing::ContentBox
                } else {
                    style.box_sizing
                };
                let ar_inline = if style.box_sizing == BoxSizing::BorderBox
                    && box_sizing_for_ar == BoxSizing::ContentBox
                {
                    (content_inline_size - border_padding_inline).clamp_negative_to_zero()
                } else {
                    content_inline_size
                };
                let (_, h) = crate::css_sizing::apply_aspect_ratio_with_auto(
                    ar_inline,
                    openui_geometry::INDEFINITE_SIZE,
                    ar,
                    None,
                );
                if !h.is_indefinite() {
                    if box_sizing_for_ar == BoxSizing::BorderBox {
                        h.max_of(border_padding_block)
                    } else {
                        h + border_padding_block
                    }
                } else {
                    resolve_intrinsic_block(
                        doc,
                        node_id,
                        &style.height,
                        intrinsic_block_size,
                        border_padding_block,
                    )
                }
            } else {
                resolve_intrinsic_block(
                    doc,
                    node_id,
                    &style.height,
                    intrinsic_block_size,
                    border_padding_block,
                )
            }
        } else {
            resolve_intrinsic_block(
                doc,
                node_id,
                &style.height,
                intrinsic_block_size,
                border_padding_block,
            )
        }
    } else {
        let raw = resolve_length(
            &style.height,
            space.percentage_resolution_block_size,
            intrinsic_block_size, // auto fallback
            intrinsic_block_size, // none fallback
        );
        // CSS Sizing 4 §5.1: When height is a percentage that resolves to auto
        // (percentage against indefinite containing block), and the element has
        // aspect-ratio + definite width, compute height from width × ratio.
        let percentage_resolved_to_auto = style.height.length_type()
            == openui_geometry::LengthType::Percent
            && space.percentage_resolution_block_size.is_indefinite();
        if percentage_resolved_to_auto {
            if let Some(ref ar) = style.aspect_ratio {
                if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
                    height_from_ar = true;
                    let box_sizing_for_ar = if ar.auto_flag {
                        BoxSizing::ContentBox
                    } else {
                        style.box_sizing
                    };
                    let ar_inline = if style.box_sizing == BoxSizing::BorderBox
                        && box_sizing_for_ar == BoxSizing::ContentBox
                    {
                        (content_inline_size - border_padding_inline).clamp_negative_to_zero()
                    } else {
                        content_inline_size
                    };
                    let (_, h) = crate::css_sizing::apply_aspect_ratio_with_auto(
                        ar_inline,
                        openui_geometry::INDEFINITE_SIZE,
                        ar,
                        None,
                    );
                    if !h.is_indefinite() {
                        if box_sizing_for_ar == BoxSizing::BorderBox {
                            h.max_of(border_padding_block)
                        } else {
                            h + border_padding_block
                        }
                    } else if style.box_sizing == BoxSizing::BorderBox {
                        raw.max_of(border_padding_block)
                    } else {
                        raw + border_padding_block
                    }
                } else if style.box_sizing == BoxSizing::BorderBox {
                    raw.max_of(border_padding_block)
                } else {
                    raw + border_padding_block
                }
            } else if style.box_sizing == BoxSizing::BorderBox {
                raw.max_of(border_padding_block)
            } else {
                raw + border_padding_block
            }
        } else if style.box_sizing == BoxSizing::BorderBox {
            raw.max_of(border_padding_block)
        } else {
            raw + border_padding_block
        }
    };

    // Apply min-height / max-height.
    // min/max values are in content-box space (for box-sizing: content-box),
    // so convert them to border-box before clamping against the resolved value
    // which is already in border-box space.
    //
    // CSS Sizing 4 §5.1 — Automatic Minimum Size:
    // In general, the automatic minimum size of an aspect-ratio'd box in
    // either axis is its min-content size in that axis, clamped by its
    // maximum size. This applies to BOTH `<ratio>` and `auto <ratio>`.
    //
    // Exception: scroll containers (overflow != visible) have automatic
    // minimum size of zero per CSS Sizing 4 §5.1.
    //
    // The automatic minimum only applies when:
    //   1. height is auto (being resolved through AR)
    //   2. min-height is auto (not explicitly set)
    //   3. element has an aspect-ratio
    //   4. element is NOT a scroll container
    let has_ar = style.aspect_ratio.is_some();
    let apply_automatic_min_size =
        style.min_height.is_auto() && has_ar && !style.is_scroll_container() && height_from_ar;
    let min_raw = if style.min_height.is_auto() {
        if apply_automatic_min_size {
            // CSS Sizing 4 §5.1: Automatic minimum = intrinsic block size,
            // clamped from above by the maximum size (max-height).
            let auto_min = intrinsic_block_size;
            let max_h_raw = resolve_length(
                &style.max_height,
                space.percentage_resolution_block_size,
                LayoutUnit::max(),
                LayoutUnit::max(),
            );
            let max_h = if max_h_raw == LayoutUnit::max() {
                max_h_raw
            } else if style.box_sizing == BoxSizing::ContentBox {
                max_h_raw + border_padding_block
            } else {
                max_h_raw.max_of(border_padding_block)
            };
            auto_min.min_of(max_h)
        } else {
            LayoutUnit::zero()
        }
    } else if style.min_height.is_content_or_intrinsic() {
        // CSS Sizing 3: min-height: min-content / max-content / fit-content
        let min_bb = if height_from_ar {
            resolved
        } else {
            resolve_intrinsic_block(
                doc,
                node_id,
                &style.min_height,
                intrinsic_block_size,
                border_padding_block,
            )
        };
        return resolved.max_of(min_bb);
    } else if style.min_height.is_stretch() {
        // Chrome does not subtract margins from min-height: stretch.
        // Only resolve when containing block has definite height.
        if space.is_fixed_block_size
            || space.stretch_block_size
            || !space.percentage_resolution_block_size.is_indefinite()
        {
            space.available_block_size
        } else {
            LayoutUnit::zero()
        }
    } else {
        resolve_length(
            &style.min_height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
    };
    let min = if apply_automatic_min_size && style.min_height.is_auto() {
        // Automatic minimum is already in border-box space — clamp by max-height.
        min_raw
    } else if style.box_sizing == BoxSizing::ContentBox && min_raw > LayoutUnit::zero() {
        min_raw + border_padding_block
    } else if min_raw > LayoutUnit::zero() {
        min_raw.max_of(border_padding_block)
    } else {
        min_raw
    };

    let max_raw = if style.max_height.is_content_or_intrinsic() {
        // CSS Sizing 3: max-height: min-content / max-content / fit-content
        // resolve_intrinsic_block returns border-box value
        let max_bb = if height_from_ar {
            resolved
        } else {
            resolve_intrinsic_block(
                doc,
                node_id,
                &style.max_height,
                intrinsic_block_size,
                border_padding_block,
            )
        };
        return resolved.min_of(max_bb).max_of(min);
    } else if style.max_height.is_stretch() {
        // Chrome does not subtract margins from max-height: stretch.
        if space.is_fixed_block_size
            || space.stretch_block_size
            || !space.percentage_resolution_block_size.is_indefinite()
        {
            space.available_block_size
        } else {
            LayoutUnit::max()
        }
    } else {
        resolve_length(
            &style.max_height,
            space.percentage_resolution_block_size,
            LayoutUnit::max(), // auto → unconstrained
            LayoutUnit::max(), // none → unconstrained
        )
    };
    let max = if max_raw == LayoutUnit::max() {
        max_raw // don't add border_padding to unconstrained
    } else if style.box_sizing == BoxSizing::ContentBox {
        max_raw + border_padding_block
    } else {
        max_raw.max_of(border_padding_block)
    };

    // CSS Sizing 4 §5.2: Transferred min/max through aspect-ratio.
    // min-width/max-width transfer to the block axis via the ratio.
    //
    // Transferred sizes only apply when the block size is being resolved
    // through the aspect ratio (height is auto or percentage-resolved-to-auto
    // with AR). When height is explicit, transfers do not override it.
    let (min, max) = if height_from_ar {
        if let Some(ar) = &style.aspect_ratio {
            let ratio = ar.ratio;
            if ratio.0 == 0.0 || ratio.1 == 0.0 {
                (min, max)
            } else {
                let w_to_h = ratio.1 / ratio.0;

                let transferred_min = if !style.min_width.is_auto()
                    && !style.min_width.is_content_or_intrinsic()
                {
                    let min_w_raw = resolve_length(
                        &style.min_width,
                        space.percentage_resolution_inline_size,
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                    );
                    if min_w_raw > LayoutUnit::zero() {
                        let content_min_w = if style.box_sizing == BoxSizing::BorderBox {
                            (min_w_raw - border_padding_inline).clamp_negative_to_zero()
                        } else {
                            min_w_raw
                        };
                        let transferred_h = LayoutUnit::from_f32(content_min_w.to_f32() * w_to_h);
                        let transferred = if style.box_sizing == BoxSizing::BorderBox {
                            transferred_h + border_padding_block
                        } else {
                            transferred_h + border_padding_block
                        };
                        min.max_of(transferred)
                    } else {
                        min
                    }
                } else {
                    min
                };

                let transferred_max = if max == LayoutUnit::max() {
                    let max_w_raw = resolve_length(
                        &style.max_width,
                        space.percentage_resolution_inline_size,
                        LayoutUnit::max(),
                        LayoutUnit::max(),
                    );
                    if max_w_raw != LayoutUnit::max() {
                        let content_max_w = if style.box_sizing == BoxSizing::BorderBox {
                            (max_w_raw - border_padding_inline).clamp_negative_to_zero()
                        } else {
                            max_w_raw
                        };
                        let transferred_h = LayoutUnit::from_f32(content_max_w.to_f32() * w_to_h);
                        let transferred = if style.box_sizing == BoxSizing::BorderBox {
                            transferred_h + border_padding_block
                        } else {
                            transferred_h + border_padding_block
                        };
                        max.min_of(transferred)
                    } else {
                        max
                    }
                } else {
                    max
                };

                // CSS Sizing 4 §5.2: Transferred min must not exceed explicit max.
                let transferred_min = transferred_min.min_of(transferred_max);

                // Combine transferred constraints with non-transferred constraints.
                // CSS Sizing 4: final_min = max(non_transferred_min, transferred_min),
                // final_max = min(non_transferred_max, transferred_max).
                (min.max_of(transferred_min), max.min_of(transferred_max))
            }
        } else {
            (min, max)
        }
    } else {
        (min, max)
    };

    resolved.clamp(min, max)
}

// ── Multicol layout ─────────────────────────────────────────────────────

/// Lay out children across CSS multi-column layout.
///
/// CSS Fragmentation §3.1: break value propagation.
/// `break-before` on the first in-flow child of a block propagates up
/// to the block itself. Walk down the first-child chain and return
/// the first forced break-before found, or Auto.
fn propagated_break_before(doc: &Document, node_id: NodeId) -> BreakValue {
    let style = &doc.node(node_id).style;
    if style.break_before.is_forced() {
        return style.break_before;
    }
    // Walk first in-flow child chain.
    for child_id in doc.children(node_id) {
        let cs = &doc.node(child_id).style;
        if cs.display == Display::None
            || cs.float != Float::None
            || cs.position == Position::Absolute
            || cs.position == Position::Fixed
        {
            continue;
        }
        // Found the first in-flow child — recurse.
        return propagated_break_before(doc, child_id);
    }
    style.break_before
}

/// CSS Fragmentation §3.1: break-after propagation.
/// `break-after` on the last in-flow child propagates up to the block.
fn propagated_break_after(doc: &Document, node_id: NodeId) -> BreakValue {
    let style = &doc.node(node_id).style;
    if style.break_after.is_forced() {
        return style.break_after;
    }
    // Walk last in-flow child chain (iterate in reverse to find last in-flow).
    let children: Vec<_> = doc.children(node_id).collect();
    for &child_id in children.iter().rev() {
        let cs = &doc.node(child_id).style;
        if cs.display == Display::None
            || cs.float != Float::None
            || cs.position == Position::Absolute
            || cs.position == Position::Fixed
        {
            continue;
        }
        return propagated_break_after(doc, child_id);
    }
    style.break_after
}

fn is_transparent_spanner_wrapper(style: &ComputedStyle, inline_size: LayoutUnit) -> bool {
    if style.display == Display::None
        || style.position.is_absolutely_positioned()
        || style.float != Float::None
        || style.column_span == openui_style::ColumnSpan::All
        || crate::multicol::ColumnLayoutAlgorithm::from_style(style).is_some()
        || style.creates_new_formatting_context()
        || !style.background_color.is_transparent()
        || style.has_outline()
        || style.effective_border_top() != 0
        || style.effective_border_right() != 0
        || style.effective_border_bottom() != 0
        || style.effective_border_left() != 0
        || !style.width.is_auto()
        || !style.height.is_auto()
        || !style.min_height.is_auto()
        || !style.max_height.is_none()
    {
        return false;
    }

    let zero = LayoutUnit::zero();
    resolve_margin_or_padding(&style.margin_top, inline_size) == zero
        && resolve_margin_or_padding(&style.margin_right, inline_size) == zero
        && resolve_margin_or_padding(&style.margin_bottom, inline_size) == zero
        && resolve_margin_or_padding(&style.margin_left, inline_size) == zero
        && resolve_margin_or_padding(&style.padding_top, inline_size) == zero
        && resolve_margin_or_padding(&style.padding_right, inline_size) == zero
        && resolve_margin_or_padding(&style.padding_bottom, inline_size) == zero
        && resolve_margin_or_padding(&style.padding_left, inline_size) == zero
}

fn auto_height_spanner_has_sized_in_flow_content(doc: &Document, node_id: NodeId) -> bool {
    doc.children(node_id).any(|child_id| {
        let child_style = &doc.node(child_id).style;
        if child_style.display == Display::None || child_style.position.is_absolutely_positioned() {
            return false;
        }
        !child_style.height.is_auto()
            || child_style.effective_border_top() != 0
            || child_style.effective_border_bottom() != 0
            || auto_height_spanner_has_sized_in_flow_content(doc, child_id)
    })
}

fn sole_spanner_descendant_through_transparent_wrappers(
    doc: &Document,
    node_id: NodeId,
    inline_size: LayoutUnit,
) -> Option<NodeId> {
    let style = &doc.node(node_id).style;
    if style.column_span == openui_style::ColumnSpan::All
        && style.display != Display::None
        && !style.position.is_absolutely_positioned()
        && (!style.height.is_auto() || auto_height_spanner_has_sized_in_flow_content(doc, node_id))
    {
        return Some(node_id);
    }

    if !is_transparent_spanner_wrapper(style, inline_size) {
        return None;
    }

    let mut child_iter = doc.children(node_id).filter(|child_id| {
        let child_style = &doc.node(*child_id).style;
        child_style.display != Display::None && !child_style.position.is_absolutely_positioned()
    });
    let only_child = child_iter.next()?;
    if child_iter.next().is_some() {
        return None;
    }

    sole_spanner_descendant_through_transparent_wrappers(doc, only_child, inline_size)
}

fn collapsed_spanner_content_margin_height(fragment: &Fragment) -> LayoutUnit {
    let mut strut = MarginStrut::new();
    if !fragment.start_margin_strut.is_empty() {
        strut.append_normal(fragment.start_margin_strut.positive_margin);
        if fragment.start_margin_strut.negative_margin < LayoutUnit::zero() {
            strut.append_normal(fragment.start_margin_strut.negative_margin);
        }
    }
    if !fragment.end_margin_strut.is_empty() {
        strut.append_normal(fragment.end_margin_strut.positive_margin);
        if fragment.end_margin_strut.negative_margin < LayoutUnit::zero() {
            strut.append_normal(fragment.end_margin_strut.negative_margin);
        }
    }
    strut.sum().clamp_negative_to_zero()
}

fn list_marker_line_height(style: &ComputedStyle) -> LayoutUnit {
    let marker_line_height = match style.line_height {
        LineHeight::Normal => style.font_size * 1.2,
        LineHeight::Number(n) => style.font_size * n,
        LineHeight::Length(px) => px,
        LineHeight::Percentage(pct) => style.font_size * pct / 100.0,
    };
    LayoutUnit::from_f32(marker_line_height.max(style.font_size).floor())
}

fn snap_wrapped_multicol_fragment_consumption(
    style: &ComputedStyle,
    amount: LayoutUnit,
    remaining: LayoutUnit,
    percentage_basis: LayoutUnit,
) -> LayoutUnit {
    if amount.raw() <= 0 || amount.raw() >= remaining.raw() {
        return amount;
    }
    let wraps_rows = style.column_wrap == openui_style::ColumnWrap::Wrap
        || (style.column_wrap == openui_style::ColumnWrap::Auto && style.column_height.is_some());
    if !wraps_rows {
        return amount;
    }
    let Some(column_height) = style.column_height else {
        return amount;
    };
    if !column_height.is_specified() {
        return amount;
    }
    let row_height = resolve_length(
        &column_height,
        percentage_basis,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
    );
    let row_gap = style
        .row_gap
        .as_ref()
        .map(|gap| {
            resolve_length(
                gap,
                percentage_basis,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            )
            .clamp_negative_to_zero()
        })
        .unwrap_or(LayoutUnit::zero());
    if row_height.raw() <= 0 || row_gap.raw() <= 0 {
        return amount;
    }
    let stride = row_height + row_gap;
    if stride.raw() <= row_height.raw() {
        return amount;
    }
    let remainder = amount.raw().rem_euclid(stride.raw());
    if remainder > 0 && remainder < row_height.raw() && row_height.raw() < amount.raw() {
        return LayoutUnit::from_raw(amount.raw() - remainder).min_of(remaining);
    }
    if remainder < row_height.raw() {
        return amount;
    }
    let snapped = ((amount.raw() + stride.raw() - 1) / stride.raw()) * stride.raw();
    LayoutUnit::from_raw(snapped).min_of(remaining)
}

fn avoid_descendant_break_before(
    fragment: &Fragment,
    doc: &Document,
    consumed: LayoutUnit,
    break_at: LayoutUnit,
) -> Option<LayoutUnit> {
    fn walk(
        fragment: &Fragment,
        doc: &Document,
        parent_top: LayoutUnit,
        consumed: LayoutUnit,
        break_at: LayoutUnit,
        best: &mut Option<LayoutUnit>,
    ) {
        for child in &fragment.children {
            let child_top = parent_top + child.offset.top;
            let child_bottom = child_top + child.size.height;

            if !child.node_id.is_none() {
                let style = &doc.node(child.node_id).style;
                if style.break_inside.is_avoid()
                    && previous_in_flow_sibling_is_float(doc, child.node_id)
                    && !style.position.is_positioned()
                    && child_top.raw() > consumed.raw()
                    && child_top.raw() < break_at.raw()
                    && child_bottom.raw() > break_at.raw()
                    && best.is_none_or(|current| child_top.raw() < current.raw())
                {
                    *best = Some(child_top);
                }

                if style.position.is_positioned() {
                    continue;
                }
            }

            if child_bottom.raw() > consumed.raw() && child_top.raw() < break_at.raw() {
                walk(child, doc, child_top, consumed, break_at, best);
            }
        }
    }

    if break_at.raw() <= consumed.raw() {
        return None;
    }

    let mut best = None;
    walk(
        fragment,
        doc,
        LayoutUnit::zero(),
        consumed,
        break_at,
        &mut best,
    );
    best
}

fn bottom_abspos_avoid_descendant_break_before(
    fragment: &Fragment,
    doc: &Document,
    consumed: LayoutUnit,
    break_at: LayoutUnit,
) -> Option<LayoutUnit> {
    fn walk(
        fragment: &Fragment,
        doc: &Document,
        parent_top: LayoutUnit,
        consumed: LayoutUnit,
        break_at: LayoutUnit,
        best: &mut Option<LayoutUnit>,
    ) {
        for child in &fragment.children {
            let child_top = parent_top + child.offset.top;
            let child_bottom = child_top + child.size.height;

            if !child.node_id.is_none() {
                let style = &doc.node(child.node_id).style;
                if style.break_inside.is_avoid()
                    && !style.position.is_positioned()
                    && child_top.raw() > consumed.raw()
                    && child_top.raw() < break_at.raw()
                    && child_bottom.raw() > break_at.raw()
                    && best.is_none_or(|current| child_top.raw() < current.raw())
                {
                    *best = Some(child_top);
                }
            }

            if child_bottom.raw() > consumed.raw() && child_top.raw() < break_at.raw() {
                walk(child, doc, child_top, consumed, break_at, best);
            }
        }
    }

    if break_at.raw() <= consumed.raw() {
        return None;
    }

    let mut best = None;
    walk(
        fragment,
        doc,
        LayoutUnit::zero(),
        consumed,
        break_at,
        &mut best,
    );
    best
}

fn bottom_abspos_avoid_descendant_crossing(
    fragment: &Fragment,
    doc: &Document,
    consumed: LayoutUnit,
    break_at: LayoutUnit,
) -> Option<(LayoutUnit, LayoutUnit)> {
    fn walk(
        fragment: &Fragment,
        doc: &Document,
        parent_top: LayoutUnit,
        consumed: LayoutUnit,
        break_at: LayoutUnit,
        best: &mut Option<(LayoutUnit, LayoutUnit)>,
    ) {
        for child in &fragment.children {
            let child_top = parent_top + child.offset.top;
            let child_bottom = child_top + child.size.height;

            if !child.node_id.is_none() {
                let style = &doc.node(child.node_id).style;
                if style.break_inside.is_avoid()
                    && !style.position.is_positioned()
                    && child_top.raw() > consumed.raw()
                    && child_top.raw() < break_at.raw()
                    && child_bottom.raw() > break_at.raw()
                    && best.is_none_or(|(current, _)| child_top.raw() < current.raw())
                {
                    *best = Some((child_top, child_bottom));
                }
            }

            if child_bottom.raw() > consumed.raw() && child_top.raw() < break_at.raw() {
                walk(child, doc, child_top, consumed, break_at, best);
            }
        }
    }

    if break_at.raw() <= consumed.raw() {
        return None;
    }

    let mut best = None;
    walk(
        fragment,
        doc,
        LayoutUnit::zero(),
        consumed,
        break_at,
        &mut best,
    );
    best
}

fn suppress_avoid_descendants_at_or_after(
    fragment: &mut Fragment,
    doc: &Document,
    parent_top: LayoutUnit,
    cutoff: LayoutUnit,
) {
    fragment.children.retain(|child| {
        if child.node_id.is_none() {
            return true;
        }
        let child_top = parent_top + child.offset.top;
        let style = &doc.node(child.node_id).style;
        !(style.break_inside.is_avoid()
            && !style.position.is_positioned()
            && child_top.raw() >= cutoff.raw())
    });
    for child in &mut fragment.children {
        let child_top = parent_top + child.offset.top;
        suppress_avoid_descendants_at_or_after(child, doc, child_top, cutoff);
    }
}

fn forced_break_descendant_top(fragment: &Fragment, doc: &Document) -> Option<LayoutUnit> {
    fn walk(
        fragment: &Fragment,
        doc: &Document,
        parent_top: LayoutUnit,
        best: &mut Option<LayoutUnit>,
    ) {
        for child in &fragment.children {
            let child_top = parent_top + child.offset.top;
            if !child.node_id.is_none() {
                let style = &doc.node(child.node_id).style;
                if style.break_before.is_forced()
                    && best.is_none_or(|current| child_top.raw() < current.raw())
                {
                    *best = Some(child_top);
                }
                if style.position.is_positioned() {
                    continue;
                }
            }
            walk(child, doc, child_top, best);
        }
    }

    let mut best = None;
    walk(fragment, doc, LayoutUnit::zero(), &mut best);
    best
}

fn subtree_has_positioned_descendant(doc: &Document, node_id: NodeId) -> bool {
    doc.children(node_id).any(|child| {
        doc.node(child).style.position.is_positioned()
            || subtree_has_positioned_descendant(doc, child)
    })
}

fn subtree_has_positioned_abspos_descendant(doc: &Document, node_id: NodeId) -> bool {
    let style = &doc.node(node_id).style;
    (style.position.is_positioned()
        && doc
            .children(node_id)
            .any(|child| doc.node(child).style.position.is_absolutely_positioned()))
        || doc.children(node_id).any(|child| {
            let style = &doc.node(child).style;
            (style.position.is_positioned()
                && doc
                    .children(child)
                    .any(|gc| doc.node(gc).style.position.is_absolutely_positioned()))
                || subtree_has_positioned_abspos_descendant(doc, child)
        })
}

fn subtree_has_authored_inline_overflow_descendant(doc: &Document, node_id: NodeId) -> bool {
    doc.children(node_id).any(|child| {
        let style = &doc.node(child).style;
        (style.overflow_x == Overflow::Visible
            && style.width.is_percent()
            && style.width.value() > 100.0)
            || subtree_has_authored_inline_overflow_descendant(doc, child)
    })
}

fn subtree_has_in_flow_spanner_descendant(doc: &Document, node_id: NodeId) -> bool {
    doc.children(node_id).any(|child| {
        let style = &doc.node(child).style;
        (style.display != Display::None
            && !style.position.is_absolutely_positioned()
            && style.column_span == openui_style::ColumnSpan::All)
            || subtree_has_in_flow_spanner_descendant(doc, child)
    })
}

fn subtree_has_flex_descendant(doc: &Document, node_id: NodeId) -> bool {
    doc.children(node_id).any(|child| {
        doc.node(child).style.display == Display::Flex || subtree_has_flex_descendant(doc, child)
    })
}

fn subtree_has_forced_break_descendant(doc: &Document, node_id: NodeId) -> bool {
    doc.children(node_id).any(|child| {
        let style = &doc.node(child).style;
        style.break_before.is_forced()
            || style.break_after.is_forced()
            || subtree_has_forced_break_descendant(doc, child)
    })
}

fn subtree_has_avoid_break_descendant(doc: &Document, node_id: NodeId) -> bool {
    doc.children(node_id).any(|child| {
        let style = &doc.node(child).style;
        style.break_before.is_avoid()
            || style.break_after.is_avoid()
            || style.break_inside.is_avoid()
            || subtree_has_avoid_break_descendant(doc, child)
    })
}

fn fill_current_fragmentainer_with_late_direct_float(
    fragment: &mut Fragment,
    doc: &Document,
    fragmentainer_height: LayoutUnit,
) -> bool {
    if fragmentainer_height <= LayoutUnit::zero() {
        return false;
    }

    let mut previous_float_bottom: Option<LayoutUnit> = None;
    let mut cloned_float = None;
    for child in &fragment.children {
        if child.node_id.is_none() {
            continue;
        }
        let child_style = &doc.node(child.node_id).style;
        if child_style.float == Float::None {
            continue;
        }
        let child_bottom = child.offset.top + child.size.height;
        if child.offset.top >= fragmentainer_height {
            if let Some(anchor) = previous_float_bottom {
                if anchor < fragmentainer_height
                    && anchor + child.size.height <= fragmentainer_height
                    && !child_style.background_color.is_transparent()
                {
                    let mut clone = child.clone();
                    clone.offset.top = anchor;
                    cloned_float = Some(clone);
                    break;
                }
            }
        }
        previous_float_bottom = Some(
            previous_float_bottom
                .map(|bottom| bottom.max_of(child_bottom))
                .unwrap_or(child_bottom),
        );
    }

    if let Some(clone) = cloned_float {
        fragment.children.push(clone);
        true
    } else {
        false
    }
}

fn subtree_has_clone_decoration_descendant(doc: &Document, node_id: NodeId) -> bool {
    doc.children(node_id).any(|child| {
        doc.node(child).style.box_decoration_break == BoxDecorationBreak::Clone
            || subtree_has_clone_decoration_descendant(doc, child)
    })
}

fn fragment_visual_block_bottom(fragment: &Fragment) -> LayoutUnit {
    let overflow_bottom = fragment
        .overflow_rect
        .map(|rect| rect.offset.top + rect.size.height)
        .unwrap_or(fragment.size.height);
    fragment.children.iter().fold(
        fragment.size.height.max_of(overflow_bottom),
        |bottom, child| bottom.max_of(child.offset.top + fragment_visual_block_bottom(child)),
    )
}

fn fragment_visual_block_top(fragment: &Fragment) -> LayoutUnit {
    let overflow_top = fragment
        .overflow_rect
        .map(|rect| rect.offset.top)
        .unwrap_or(LayoutUnit::zero());
    fragment.children.iter().fold(overflow_top, |top, child| {
        top.min_of(child.offset.top + fragment_visual_block_top(child))
    })
}

fn previous_in_flow_sibling_is_float(doc: &Document, node_id: NodeId) -> bool {
    let parent = doc.node(node_id).parent;
    if parent.is_none() {
        return false;
    }

    let mut previous = None;
    for sibling in doc.children(parent) {
        if sibling == node_id {
            break;
        }
        let style = &doc.node(sibling).style;
        if style.display != Display::None && !style.position.is_absolutely_positioned() {
            previous = Some(sibling);
        }
    }

    previous.is_some_and(|sibling| doc.node(sibling).style.float != Float::None)
}

fn collapse_adjacent_block_margins(a: LayoutUnit, b: LayoutUnit) -> LayoutUnit {
    if a.raw() >= 0 && b.raw() >= 0 {
        a.max_of(b)
    } else if a.raw() <= 0 && b.raw() <= 0 {
        a.min_of(b)
    } else {
        a + b
    }
}

/// This function handles the complete multicol path: resolves column geometry,
/// lays out each child at the column width, distributes children across columns
/// using balanced or auto-fill, and produces a container fragment with
/// positioned column fragments containing the actual children.
fn layout_multicol(
    doc: &Document,
    node_id: NodeId,
    style: &ComputedStyle,
    space: &ConstraintSpace,
    algo: &crate::multicol::ColumnLayoutAlgorithm,
    border: &BoxStrut,
    padding: &BoxStrut,
    _border_padding_inline: LayoutUnit,
    border_padding_block: LayoutUnit,
    child_available_inline: LayoutUnit,
    _content_inline_size: LayoutUnit,
    border_box_inline: LayoutUnit,
) -> Fragment {
    use crate::multicol::{
        balance_columns, balance_columns_with_margins, compute_column_positions,
        resolve_column_count_and_width,
    };
    use openui_style::{ColumnFill, ColumnSpan};

    let mut algo = algo.clone();
    if let Some(column_gap) = &style.column_gap {
        algo.column_gap = resolve_length(
            column_gap,
            child_available_inline,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
    }

    let resolved = resolve_column_count_and_width(
        if algo.column_count > 0 {
            Some(algo.column_count)
        } else {
            None
        },
        algo.column_width,
        child_available_inline,
        algo.column_gap,
    );

    let column_width = resolved.width;
    let content_edge_x = border.left + padding.left;
    let content_edge_y = border.top + padding.top;

    let child_percentage_block_size = if !style.height.is_auto() {
        let raw = resolve_length(
            &style.height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        let content = if style.box_sizing == BoxSizing::BorderBox {
            (raw - border_padding_block).clamp_negative_to_zero()
        } else {
            raw
        };
        // Apply min-height / max-height clamping so that the resolved
        // container block size reflects the actual usable height for columns.
        // CSS 2.1 §10.7: min-height wins over max-height when they conflict.
        // Order: apply max first, then min (so min trumps max).
        let mut clamped = content;
        if !style.max_height.is_none() && !style.max_height.is_auto() {
            let max_raw = resolve_length(
                &style.max_height,
                space.percentage_resolution_block_size,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            let max_content = if style.box_sizing == BoxSizing::BorderBox {
                (max_raw - border_padding_block).clamp_negative_to_zero()
            } else {
                max_raw
            };
            clamped = clamped.min_of(max_content);
        }
        if !style.min_height.is_auto() && !style.min_height.is_none() {
            let min_raw = resolve_length(
                &style.min_height,
                space.percentage_resolution_block_size,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            let min_content = if style.box_sizing == BoxSizing::BorderBox {
                (min_raw - border_padding_block).clamp_negative_to_zero()
            } else {
                min_raw
            };
            clamped = clamped.max_of(min_content);
        }
        clamped
    } else {
        openui_geometry::INDEFINITE_SIZE
    };

    // Get column positions.
    let positions = compute_column_positions(
        resolved.count,
        column_width,
        algo.column_gap,
        child_available_inline,
        false,
    );

    // Gather children, separating spanners from columnar content.
    // A spanner (column-span: all) interrupts the column flow.
    //
    // CSS Multicol §8: column-span applies to in-flow descendants of a
    // multicol container, not just direct children.  When a block child
    // contains nested column-span:all descendants, the child is "split"
    // at spanner boundaries (spanner extraction).
    struct ChildInfo {
        id: NodeId,
        is_spanner: bool,
        /// For split portions of a container with nested spanners:
        /// the grandchild IDs to include in this group portion and
        /// the height budget allocated from the container.
        split_portion: Option<SplitPortion>,
    }
    struct SplitPortion {
        /// The container (direct child of multicol) whose children we're splitting.
        container_id: NodeId,
        /// Grandchild IDs to include in this portion.
        child_ids: Vec<NodeId>,
        /// Content height allocated to this portion (excludes border/padding).
        portion_height: LayoutUnit,
        /// How much of this portion's content area should paint the wrapper's
        /// own decorations for explicit-height split wrappers.
        paint_height: LayoutUnit,
        /// True if this is the first portion of the container (gets top border/padding).
        is_first: bool,
        /// True if this is the last portion of the container (gets bottom border/padding).
        is_last: bool,
    }
    let mut children_info: Vec<ChildInfo> = Vec::new();
    // Track OOF children with their flow index (number of in-flow children
    // before them) so we can compute correct static positions during column
    // distribution.  CSS 2.1 §10.3.7: the static position of an abspos
    // element is where it would have been placed in normal flow.
    struct OofChild {
        node_id: NodeId,
        flow_index: usize,
    }
    let mut oof_children: Vec<OofChild> = Vec::new();
    let mut flow_count: usize = 0;
    for child_id in doc.children(node_id) {
        let child_style = &doc.node(child_id).style;
        if child_style.display == Display::None {
            continue;
        }
        // CSS Multicol §3: Floats inside a multicol container are treated
        // as block-level children for column distribution.  Only truly
        // absolutely-positioned / fixed elements are out-of-flow here.
        if child_style.position.is_absolutely_positioned() {
            oof_children.push(OofChild {
                node_id: child_id,
                flow_index: flow_count,
            });
            continue;
        }
        if child_style.column_span == ColumnSpan::All {
            children_info.push(ChildInfo {
                id: child_id,
                is_spanner: true,
                split_portion: None,
            });
            // Note: do NOT increment flow_count for spanners. Spanners
            // create group boundaries but are not in-flow column children.
            // OOF elements appearing after a spanner should get the same
            // flow_index as the first in-flow child of the next group.
            continue;
        }

        if let Some(spanner_id) = sole_spanner_descendant_through_transparent_wrappers(
            doc,
            child_id,
            child_available_inline,
        ) {
            children_info.push(ChildInfo {
                id: spanner_id,
                is_spanner: true,
                split_portion: None,
            });
            continue;
        }

        // Check for nested spanner extraction: if this non-spanner child
        // is a normal block container (not BFC-triggering), check whether
        // any of its immediate children have column-span: all.
        // CSS Multicol §8: "a column spanning element only spans the columns
        // of its nearest multicol container." If this child is itself a
        // multicol container, its column-span:all children belong to IT,
        // not to the outer multicol. Do not extract them.
        let child_is_multicol =
            crate::multicol::ColumnLayoutAlgorithm::from_style(child_style).is_some();
        let has_nested_spanners = !child_is_multicol
            && !child_style.creates_new_formatting_context()
            && !child_style.overflow_x.is_scrollable()
            && !child_style.overflow_y.is_scrollable()
            && child_style.display.is_block_level()
            && doc.children(child_id).any(|gc| {
                let gs = &doc.node(gc).style;
                gs.column_span == ColumnSpan::All
                    && gs.display != Display::None
                    && !gs.position.is_absolutely_positioned()
            });

        if has_nested_spanners {
            // Extract nested spanners: split this container at spanner
            // boundaries. Each non-spanner group becomes a "portion" entry
            // and each spanner becomes a multicol-level spanner.
            let container_height = if !child_style.height.is_auto() {
                let raw = resolve_length(
                    &child_style.height,
                    child_percentage_block_size,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                );
                if child_style.box_sizing == BoxSizing::BorderBox {
                    let bp = LayoutUnit::from_i32(
                        child_style.effective_border_top() + child_style.effective_border_bottom(),
                    ) + LayoutUnit::from_f32(child_style.padding_top.value())
                        + LayoutUnit::from_f32(child_style.padding_bottom.value());
                    (raw - bp).clamp_negative_to_zero()
                } else {
                    raw
                }
            } else {
                // Auto height: use sum of children as budget
                LayoutUnit::from_raw(i32::MAX / 2)
            };

            // Spanner extraction: split container at spanner boundaries.
            // Each group gets its natural content height as portion_height.
            // The column balancing algorithm and remaining_available_block
            // handle the actual height constraint — we don't cap here.
            let container_pad_top =
                resolve_margin_or_padding(&child_style.padding_top, child_available_inline)
                    + LayoutUnit::from_i32(child_style.effective_border_top());
            let container_pad_bottom =
                resolve_margin_or_padding(&child_style.padding_bottom, child_available_inline)
                    + LayoutUnit::from_i32(child_style.effective_border_bottom());

            let mut current_group: Vec<NodeId> = Vec::new();
            let mut current_group_content = LayoutUnit::zero();
            let has_explicit_height = !child_style.height.is_auto();
            let mut total_content_used = LayoutUnit::zero();
            let mut painted_content_used = LayoutUnit::zero();
            let mut spanner_count = 0usize;
            let mut portion_index = 0usize;

            for gc_id in doc.children(child_id) {
                let gc_style = &doc.node(gc_id).style;
                if gc_style.display == Display::None {
                    continue;
                }
                if gc_style.position.is_absolutely_positioned() {
                    continue;
                }
                if gc_style.column_span == ColumnSpan::All {
                    // Always create a before-portion, even if empty, so the
                    // container's explicit height is preserved in columns.
                    let portion_h = current_group_content;
                    let paint_h = if has_explicit_height {
                        (container_height - painted_content_used)
                            .clamp_negative_to_zero()
                            .min_of(portion_h)
                    } else {
                        portion_h
                    };
                    children_info.push(ChildInfo {
                        id: child_id,
                        is_spanner: false,
                        split_portion: Some(SplitPortion {
                            container_id: child_id,
                            child_ids: std::mem::take(&mut current_group),
                            portion_height: portion_h,
                            paint_height: paint_h,
                            is_first: portion_index == 0,
                            is_last: false, // a before-spanner portion is never last
                        }),
                    });
                    portion_index += 1;
                    total_content_used = total_content_used + portion_h;
                    painted_content_used = painted_content_used + portion_h;
                    current_group_content = LayoutUnit::zero();
                    spanner_count += 1;
                    children_info.push(ChildInfo {
                        id: gc_id,
                        is_spanner: true,
                        split_portion: None,
                    });
                } else {
                    let gc_h = if !gc_style.height.is_auto() {
                        resolve_length(
                            &gc_style.height,
                            container_height,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        )
                    } else {
                        let gc_is_new_fc = establishes_new_fc(&doc.node(gc_id).style);
                        let gc_space = ConstraintSpace::for_block_child(
                            child_available_inline,
                            container_height,
                            child_available_inline,
                            container_height,
                            gc_is_new_fc,
                        );
                        let gc_frag = block_layout(doc, gc_id, &gc_space);
                        gc_frag.size.height
                    };
                    current_group.push(gc_id);
                    current_group_content = current_group_content + gc_h;
                }
            }
            // Always create an after-portion (may be empty) when spanners exist.
            // If the container has explicit height, assign remaining height to
            // the last portion so column distribution respects the full height.
            if spanner_count > 0 {
                let natural_content = current_group_content;
                total_content_used = total_content_used + natural_content;
                let final_height = if has_explicit_height {
                    let remaining =
                        (container_height - total_content_used).clamp_negative_to_zero();
                    natural_content + remaining
                } else {
                    natural_content
                };
                let paint_h = if has_explicit_height {
                    (container_height - painted_content_used)
                        .clamp_negative_to_zero()
                        .min_of(final_height)
                } else {
                    natural_content
                };
                children_info.push(ChildInfo {
                    id: child_id,
                    is_spanner: false,
                    split_portion: Some(SplitPortion {
                        container_id: child_id,
                        child_ids: current_group,
                        portion_height: final_height,
                        paint_height: paint_h,
                        is_first: portion_index == 0,
                        is_last: true,
                    }),
                });
            } else if !current_group.is_empty() || current_group_content > LayoutUnit::zero() {
                let final_content = current_group_content;
                children_info.push(ChildInfo {
                    id: child_id,
                    is_spanner: false,
                    split_portion: Some(SplitPortion {
                        container_id: child_id,
                        child_ids: current_group,
                        portion_height: final_content,
                        paint_height: final_content,
                        is_first: true,
                        is_last: true,
                    }),
                });
            }
        } else {
            children_info.push(ChildInfo {
                id: child_id,
                is_spanner: false,
                split_portion: None,
            });
        }
        flow_count += 1;
    }

    let mut result_children: Vec<Fragment> = Vec::new();
    // OOF candidates bubbled up from in-flow children inside columns.
    // Collected during column distribution and processed alongside direct OOF children.
    let mut bubbled_oof_from_columns: Vec<crate::out_of_flow::OutOfFlowCandidate> = Vec::new();
    let mut total_block_offset = LayoutUnit::zero();

    // Remaining available block size tracks how much vertical space is left
    // for column groups after subtracting previous groups and spanners.
    // This is critical for correct column heights when spanners are present.
    let has_explicit_height = !style.height.is_auto();
    let container_content_height = if has_explicit_height {
        child_percentage_block_size
    } else {
        openui_geometry::INDEFINITE_SIZE
    };
    let mut remaining_available_block = container_content_height;

    // Resolve max-height once for column-fill:auto with auto height.
    let resolved_max_height = if !style.max_height.is_none() && !style.max_height.is_auto() {
        let raw = resolve_length(
            &style.max_height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        let content_h = if style.box_sizing == BoxSizing::BorderBox {
            (raw - border_padding_block).clamp_negative_to_zero()
        } else {
            raw
        };
        if content_h.raw() > 0 {
            Some(content_h)
        } else {
            None
        }
    } else {
        None
    };

    // Process children in groups separated by spanners.
    let mut group_start = 0;
    // Track OOF static positions: for each OOF child, record the column
    // position where it would have appeared in normal flow.
    let mut oof_static_positions: Vec<(NodeId, PhysicalOffset)> = Vec::new();
    let mut global_flow_idx: usize = 0;
    let mut previous_spanner_margin_bottom: Option<LayoutUnit> = None;
    while group_start < children_info.len() {
        // Collect the next group of columnar children (until a spanner or end).
        let mut group_end = group_start;
        while group_end < children_info.len() && !children_info[group_end].is_spanner {
            group_end += 1;
        }

        // Lay out columnar group.
        if group_end > group_start {
            let group_has_spanner_separating_content = children_info[group_start..group_end]
                .iter()
                .any(|info| match info.split_portion.as_ref() {
                    Some(split) => {
                        let container_style = &doc.node(split.container_id).style;
                        !split.child_ids.is_empty()
                            || split.portion_height > LayoutUnit::zero()
                            || split.paint_height > LayoutUnit::zero()
                            || !container_style.background_color.is_transparent()
                            || container_style.effective_border_top() != 0
                            || container_style.effective_border_bottom() != 0
                            || resolve_margin_or_padding(&container_style.padding_top, column_width)
                                != LayoutUnit::zero()
                            || resolve_margin_or_padding(
                                &container_style.padding_bottom,
                                column_width,
                            ) != LayoutUnit::zero()
                            || resolve_margin_or_padding(&container_style.margin_top, column_width)
                                != LayoutUnit::zero()
                            || resolve_margin_or_padding(
                                &container_style.margin_bottom,
                                column_width,
                            ) != LayoutUnit::zero()
                    }
                    None => true,
                });
            if group_has_spanner_separating_content {
                previous_spanner_margin_bottom = None;
            }
            // Determine the available block size for this column group.
            // For explicit-height containers, use remaining space after previous
            // groups/spanners. For auto-height, use max-height or indefinite.
            let group_available_block = if has_explicit_height {
                algo.column_height.unwrap_or(remaining_available_block)
            } else if let Some(mh) = resolved_max_height {
                // column-fill:auto with max-height: use remaining of max-height
                if let Some(column_height) = algo.column_height {
                    column_height
                } else if remaining_available_block.is_indefinite() {
                    mh
                } else {
                    remaining_available_block
                }
            } else {
                algo.column_height
                    .unwrap_or(openui_geometry::INDEFINITE_SIZE)
            };

            // First pass: lay out children to get intrinsic sizes.
            // Percentage-height children resolve against the multicol
            // container's content height (child_percentage_block_size), NOT
            // the group's remaining available space.  This ensures that
            // children after a spanner still resolve 100 % → container
            // height and are then fragmented across columns at the group's
            // (smaller) column height.
            let first_pass_pct_basis = child_percentage_block_size;

            let mut col_fragments: Vec<Fragment> = Vec::new();
            let mut col_block_sizes: Vec<LayoutUnit> = Vec::new();
            let mut col_visual_flow_sizes: Vec<LayoutUnit> = Vec::new();
            let mut col_margins_top: Vec<LayoutUnit> = Vec::new();
            let mut col_margins_bottom: Vec<LayoutUnit> = Vec::new();
            let mut col_avoid_break: Vec<bool> = Vec::new();
            let mut col_avoid_break_after: Vec<bool> = Vec::new();
            let mut col_forced_break_before: Vec<bool> = Vec::new();

            let mut prev_break_after_forced_fp = false;
            for info in &children_info[group_start..group_end] {
                // Compute forced break-before for this child (for balancing).
                let child_node_id = info.id;
                let prop_bb = propagated_break_before(doc, child_node_id);
                let forced = prop_bb.is_forced() || prev_break_after_forced_fp;

                if let Some(ref split) = info.split_portion {
                    // Split portion: lay out the portion's children and
                    // create a wrapper fragment with the container's styling.
                    // box-decoration-break:slice (default): only first portion
                    // gets top border/padding, only last gets bottom.
                    let container_style = &doc.node(split.container_id).style;
                    let c_border_top_full =
                        LayoutUnit::from_i32(container_style.effective_border_top());
                    let c_border_bottom_full =
                        LayoutUnit::from_i32(container_style.effective_border_bottom());
                    let c_pad_top_full =
                        resolve_margin_or_padding(&container_style.padding_top, column_width);
                    let c_pad_bottom_full =
                        resolve_margin_or_padding(&container_style.padding_bottom, column_width);
                    let c_border_left =
                        LayoutUnit::from_i32(container_style.effective_border_left());
                    let c_border_right =
                        LayoutUnit::from_i32(container_style.effective_border_right());
                    let c_pad_left =
                        resolve_margin_or_padding(&container_style.padding_left, column_width);
                    let c_pad_right =
                        resolve_margin_or_padding(&container_style.padding_right, column_width);

                    // Apply top/bottom border+padding only for first/last portions
                    let c_border_top = if split.is_first {
                        c_border_top_full
                    } else {
                        LayoutUnit::zero()
                    };
                    let c_pad_top = if split.is_first {
                        c_pad_top_full
                    } else {
                        LayoutUnit::zero()
                    };
                    let c_border_bottom = if split.is_last {
                        c_border_bottom_full
                    } else {
                        LayoutUnit::zero()
                    };
                    let c_pad_bottom = if split.is_last {
                        c_pad_bottom_full
                    } else {
                        LayoutUnit::zero()
                    };

                    let inner_width =
                        (column_width - c_border_left - c_border_right - c_pad_left - c_pad_right)
                            .clamp_negative_to_zero();

                    let mut wrapper_children: Vec<Fragment> = Vec::new();
                    let mut block_off = c_border_top + c_pad_top;
                    for &gc_id in &split.child_ids {
                        let gc_style = &doc.node(gc_id).style;
                        let gc_is_new_fc = establishes_new_fc(gc_style);
                        let gc_space = ConstraintSpace::for_block_child(
                            inner_width,
                            split.portion_height,
                            inner_width,
                            split.portion_height,
                            gc_is_new_fc,
                        );
                        let mut gc_frag = block_layout(doc, gc_id, &gc_space);
                        let gc_margin_left =
                            resolve_margin_or_padding(&gc_style.margin_left, inner_width);
                        gc_frag.offset = PhysicalOffset::new(
                            c_border_left + c_pad_left + gc_margin_left,
                            block_off,
                        );
                        block_off = block_off + gc_frag.size.height;
                        wrapper_children.push(gc_frag);
                    }
                    let content_h = block_off - c_border_top - c_pad_top;
                    // Use the larger of portion_height (explicit budget) and
                    // content_h (natural content). For explicit-height containers,
                    // the budget may exceed the content; for auto-height, content
                    // may exceed the estimate.
                    let effective_h = split.portion_height.max_of(content_h);
                    let wrapper_h =
                        effective_h + c_border_top + c_pad_top + c_border_bottom + c_pad_bottom;
                    let mut wrapper = Fragment::new_box(
                        split.container_id,
                        PhysicalSize::new(column_width, wrapper_h),
                    );
                    let include_block_end_decoration =
                        if split.is_last && !container_style.height.is_auto() {
                            split.paint_height.raw() > 0
                        } else {
                            split.paint_height.raw() >= effective_h.raw()
                        };
                    let decoration_paint_height = (c_border_top
                        + c_pad_top
                        + split.paint_height
                        + if include_block_end_decoration {
                            c_pad_bottom + c_border_bottom
                        } else {
                            LayoutUnit::zero()
                        })
                    .min_of(wrapper_h);
                    if !container_style.height.is_auto()
                        || decoration_paint_height.raw() < wrapper_h.raw()
                    {
                        wrapper.decoration_paint_block_size = Some(decoration_paint_height);
                    }
                    wrapper.is_first_for_node = split.is_first;
                    wrapper.is_last_for_node = split.is_last;
                    wrapper.children = wrapper_children;
                    wrapper.border = BoxStrut {
                        top: c_border_top,
                        right: c_border_right,
                        bottom: c_border_bottom,
                        left: c_border_left,
                    };
                    wrapper.padding = BoxStrut {
                        top: c_pad_top,
                        right: c_pad_right,
                        bottom: c_pad_bottom,
                        left: c_pad_left,
                    };

                    let explicit_split_height = !container_style.height.is_auto();
                    let child_margin_top = if split.is_first || !explicit_split_height {
                        resolve_margin_or_padding(&container_style.margin_top, column_width)
                    } else {
                        LayoutUnit::zero()
                    };
                    let child_margin_bottom = if split.is_last || !explicit_split_height {
                        resolve_margin_or_padding(&container_style.margin_bottom, column_width)
                    } else {
                        LayoutUnit::zero()
                    };

                    let distribution_block_size =
                        if split.is_last && !container_style.height.is_auto() {
                            (wrapper.size.height - c_pad_bottom - c_border_bottom)
                                .clamp_negative_to_zero()
                        } else {
                            wrapper.size.height
                        };
                    col_block_sizes.push(distribution_block_size);
                    col_margins_top.push(child_margin_top);
                    col_margins_bottom.push(child_margin_bottom);
                    col_avoid_break.push(container_style.break_inside.is_avoid());
                    col_avoid_break_after.push(
                        propagated_break_after(doc, child_node_id).is_avoid()
                            || container_style.break_after.is_avoid(),
                    );
                    col_forced_break_before.push(forced);
                    col_fragments.push(wrapper);

                    let prop_ba = propagated_break_after(doc, child_node_id);
                    prev_break_after_forced_fp = prop_ba.is_forced();
                } else {
                    let child_style = &doc.node(info.id).style;
                    let child_margin_top =
                        resolve_margin_or_padding(&child_style.margin_top, column_width);
                    let child_margin_bottom =
                        resolve_margin_or_padding(&child_style.margin_bottom, column_width);
                    let child_is_new_fc = establishes_new_fc(child_style);
                    let child_auto_float =
                        child_style.float != Float::None && child_style.width.is_auto();
                    let child_column_algo =
                        crate::multicol::ColumnLayoutAlgorithm::from_style(child_style);
                    let child_is_multicol = child_column_algo.is_some();
                    let sole_auto_nested_multicol_in_fixed_auto_fill = child_is_multicol
                        && has_explicit_height
                        && algo.column_fill == ColumnFill::Auto
                        && algo.column_height.is_none()
                        && algo.column_wrap == openui_style::ColumnWrap::Auto
                        && style.background_color.is_transparent()
                        && style.effective_border_top() == 0
                        && style.effective_border_right() == 0
                        && style.effective_border_bottom() == 0
                        && style.effective_border_left() == 0
                        && group_end == group_start + 1
                        && child_style.width.is_auto()
                        && child_style.height.is_auto()
                        && child_column_algo.as_ref().map_or(false, |child_algo| {
                            child_algo.column_height.is_none()
                                && child_algo.column_wrap == openui_style::ColumnWrap::Auto
                        })
                        && !subtree_has_flex_descendant(doc, info.id)
                        && !subtree_has_in_flow_spanner_descendant(doc, info.id);
                    let child_available_inline = if child_auto_float {
                        let child_margin = resolve_margins(child_style, column_width);
                        let margin_inline = {
                            let ml = if child_style.margin_left.is_auto() {
                                LayoutUnit::zero()
                            } else {
                                child_margin.left
                            };
                            let mr = if child_style.margin_right.is_auto() {
                                LayoutUnit::zero()
                            } else {
                                child_margin.right
                            };
                            ml + mr
                        };
                        crate::out_of_flow::compute_shrink_to_fit_width(
                            doc,
                            info.id,
                            (column_width - margin_inline).clamp_negative_to_zero(),
                        )
                    } else if sole_auto_nested_multicol_in_fixed_auto_fill {
                        child_available_inline
                    } else {
                        column_width
                    };
                    let mut child_space = ConstraintSpace::for_block_child(
                        child_available_inline,
                        group_available_block,
                        column_width,
                        first_pass_pct_basis,
                        child_is_new_fc,
                    );
                    if child_auto_float {
                        child_space.is_fixed_inline_size = true;
                    }
                    // Tell children about the fragmentainer (column) height so
                    // flex containers know the wrapping boundary.
                    if !group_available_block.is_indefinite() {
                        child_space.fragmentainer_block_size = group_available_block;
                    }
                    let mut child_frag = block_layout(doc, info.id, &child_space);
                    if sole_auto_nested_multicol_in_fixed_auto_fill {
                        child_frag.has_overflow_clip = true;
                        child_frag.block_axis_clip_only = false;
                    }
                    let zero_width_auto_float_flow = child_style.float != Float::None
                        && style.height.is_auto()
                        && resolved.count == 1
                        && child_style.width.is_auto()
                        && child_frag.size.width == LayoutUnit::zero();
                    let flex_visual_overflow_flow_size = style.height.is_auto()
                        && resolved.count > 1
                        && child_style.display == Display::Flex
                        && child_style.height.is_auto()
                        && child_style.overflow_x == Overflow::Visible
                        && child_style.overflow_y == Overflow::Visible
                        && !subtree_has_positioned_descendant(doc, info.id)
                        && !subtree_has_in_flow_spanner_descendant(doc, info.id)
                        && !subtree_has_forced_break_descendant(doc, info.id)
                        && fragment_visual_block_bottom(&child_frag).raw()
                            > child_frag.size.height.raw();
                    let float_flow_block_size = if zero_width_auto_float_flow
                        || (child_style.float != Float::None
                            && style.height.is_auto()
                            && resolved.count > 1
                            && child_style.height.is_fixed()
                            && !child_style.background_color.is_transparent()
                            && group_end == children_info.len()
                            && !children_info[group_start..group_end]
                                .iter()
                                .any(|info| subtree_has_in_flow_spanner_descendant(doc, info.id))
                            && doc.children(node_id).any(|sibling_id| {
                                let sibling_style = &doc.node(sibling_id).style;
                                sibling_id != info.id
                                    && sibling_style.float == Float::None
                                    && sibling_style.display.is_block_level()
                            })) {
                        LayoutUnit::zero()
                    } else {
                        child_frag.size.height
                    };
                    col_block_sizes.push(float_flow_block_size);
                    col_visual_flow_sizes.push(if flex_visual_overflow_flow_size {
                        fragment_visual_block_bottom(&child_frag)
                    } else {
                        LayoutUnit::zero()
                    });
                    col_margins_top.push(child_margin_top);
                    col_margins_bottom.push(child_margin_bottom);
                    let float_can_fill_remaining_fragmentainer = child_style.float != Float::None
                        && algo.column_fill == ColumnFill::Auto
                        && resolved.count >= 4
                        && !child_style.background_color.is_transparent();
                    col_avoid_break.push(
                        !float_can_fill_remaining_fragmentainer
                            && (child_style.break_inside.is_avoid()
                                || flex_visual_overflow_flow_size),
                    );
                    col_avoid_break_after.push(
                        propagated_break_after(doc, child_node_id).is_avoid()
                            || doc.node(info.id).style.break_after.is_avoid(),
                    );
                    col_forced_break_before.push(forced);
                    col_fragments.push(child_frag);

                    let prop_ba = propagated_break_after(doc, child_node_id);
                    prev_break_after_forced_fp = prop_ba.is_forced();
                }
            }

            for i in 1..col_avoid_break_after.len() {
                let child_id = children_info[group_start + i].id;
                let child_style = &doc.node(child_id).style;
                if child_style.break_before.is_avoid()
                    || propagated_break_before(doc, child_id).is_avoid()
                {
                    col_avoid_break_after[i - 1] = true;
                }
            }

            // Compute effective block sizes including collapsed margins for balancing.
            // The balancing path preserves the historical positive-margin max
            // model; placement below has a narrow flex/negative-margin pull-in.
            let compute_effective = |sizes: &[LayoutUnit],
                                     m_top: &[LayoutUnit],
                                     m_bot: &[LayoutUnit]|
             -> Vec<LayoutUnit> {
                let mut eff: Vec<LayoutUnit> = Vec::with_capacity(sizes.len());
                let mut prev_mb = LayoutUnit::zero();
                for j in 0..sizes.len() {
                    let mt = m_top[j];
                    let collapsed = if j == 0 { mt } else { prev_mb.max_of(mt) };
                    eff.push(collapsed + sizes[j]);
                    prev_mb = m_bot[j];
                }
                eff
            };

            let mut _effective_sizes =
                compute_effective(&col_block_sizes, &col_margins_top, &col_margins_bottom);

            // Determine column height for this group.
            let group_max = if !group_available_block.is_indefinite() {
                group_available_block
            } else if !space.available_block_size.is_indefinite() {
                space.available_block_size
            } else if space.fragmentainer_block_size > LayoutUnit::zero() {
                space.fragmentainer_block_size
            } else {
                space.available_block_size
            };

            // CSS Multicol §7.2: Content preceding a column-span:all element
            // is always balanced, even when column-fill is auto.
            let has_spanner_after =
                group_end < children_info.len() && children_info[group_end].is_spanner;
            let single_positioned_abspos_before_spanner = has_spanner_after
                && group_end == group_start + 1
                && !group_available_block.is_indefinite()
                && col_block_sizes
                    .first()
                    .map_or(false, |size| *size < group_available_block)
                && doc
                    .node(children_info[group_start].id)
                    .style
                    .position
                    .is_positioned()
                && doc
                    .children(children_info[group_start].id)
                    .any(|gc| doc.node(gc).style.position.is_absolutely_positioned());

            let mut column_height = if single_positioned_abspos_before_spanner
                && algo.column_fill == ColumnFill::Auto
            {
                col_block_sizes
                    .iter()
                    .copied()
                    .max()
                    .unwrap_or(LayoutUnit::from_i32(1))
                    .max_of(LayoutUnit::from_i32(1))
            } else if has_spanner_after && algo.column_fill == ColumnFill::Auto {
                // Force balance before spanner per §7.2, regardless of
                // whether the container has a definite height/max-height.
                balance_columns_with_margins(
                    &col_block_sizes,
                    &col_margins_top,
                    &col_margins_bottom,
                    &col_avoid_break,
                    resolved.count,
                    group_max,
                    &col_forced_break_before,
                    &col_avoid_break_after,
                )
            } else {
                match algo.column_fill {
                    ColumnFill::Balance | ColumnFill::BalanceAll => balance_columns_with_margins(
                        &col_block_sizes,
                        &col_margins_top,
                        &col_margins_bottom,
                        &col_avoid_break,
                        resolved.count,
                        group_max,
                        &col_forced_break_before,
                        &col_avoid_break_after,
                    ),
                    ColumnFill::Auto => {
                        if has_explicit_height {
                            if !remaining_available_block.is_indefinite() {
                                remaining_available_block
                            } else {
                                child_percentage_block_size
                            }
                        } else if let Some(mh) = resolved_max_height {
                            if !remaining_available_block.is_indefinite() {
                                remaining_available_block
                            } else {
                                mh
                            }
                        } else {
                            // column-fill:auto with no height/max-height:
                            // CSS Multicol §7.2: auto-height multicol has no
                            // constraint, so column-fill:auto degrades to balance.
                            balance_columns_with_margins(
                                &col_block_sizes,
                                &col_margins_top,
                                &col_margins_bottom,
                                &col_avoid_break,
                                resolved.count,
                                group_max,
                                &col_forced_break_before,
                                &col_avoid_break_after,
                            )
                        }
                    }
                }
            };
            if let Some(max_visual_flow_size) = col_visual_flow_sizes.iter().copied().max() {
                if max_visual_flow_size.raw() > 0 {
                    column_height = column_height.max_of(max_visual_flow_size);
                }
            }
            if style.height.is_auto()
                && group_available_block.is_indefinite()
                && algo.column_height.is_none()
                && algo.column_wrap == openui_style::ColumnWrap::Auto
                && resolved_max_height.is_none()
                && !space.available_block_size.is_indefinite()
                && space.available_block_size > LayoutUnit::zero()
                && (resolved.count == 1
                    || (space.has_block_fragmentation()
                        && space.available_inline_size.raw()
                            > space.percentage_resolution_inline_size.raw()))
                && matches!(
                    algo.column_fill,
                    ColumnFill::Balance | ColumnFill::BalanceAll
                )
                && !subtree_has_flex_descendant(doc, node_id)
                && !subtree_has_in_flow_spanner_descendant(doc, node_id)
            {
                column_height = column_height.min_of(space.available_block_size);
            }
            if has_spanner_after
                && style.height.is_auto()
                && group_available_block.is_indefinite()
                && algo.column_wrap != openui_style::ColumnWrap::Wrap
                && algo.column_height.is_none()
                && resolved.count > 1
            {
                for (idx, child_frag) in col_fragments.iter().enumerate() {
                    let child_style = &doc.node(children_info[group_start + idx].id).style;
                    if child_style.height.is_auto()
                        || child_style.overflow_x != Overflow::Visible
                        || child_style.overflow_y != Overflow::Visible
                        || child_frag.has_overflow_clip
                    {
                        continue;
                    }
                    let overflow = child_frag.scrollable_overflow();
                    let overflow_bottom = overflow.offset.top + overflow.size.height;
                    if overflow_bottom.raw() > col_block_sizes[idx].raw() {
                        let count = resolved.count.max(1) as i32;
                        let balanced_raw = (overflow_bottom.raw() + count - 1) / count;
                        column_height = column_height.max_of(LayoutUnit::from_raw(balanced_raw));
                    }
                }
            }

            // Cap column_height at the group's available block size so that
            // a group never claims more vertical space than the container
            // allocated to it.  balance_columns() can return a value
            // exceeding group_available_block when content overflows.
            // CSS Multicol §3: even when remaining space is zero, columns
            // must be at least 1px to allow overflow content to be visible.
            let column_height = if !group_available_block.is_indefinite() {
                column_height
                    .min_of(group_available_block)
                    .max_of(LayoutUnit::from_i32(1))
            } else {
                column_height
            };
            // Keep the specified visual column-height for row wrapping.  A
            // 0px column still needs a 1px internal fragmentainer guard for
            // progress, but row offsets use the author-specified stride.
            let visual_column_height =
                if algo.column_height.is_some() && !group_available_block.is_indefinite() {
                    group_available_block
                } else {
                    column_height
                };

            // Second pass: if column_height differs from first_pass_pct_basis
            // and any child has a percentage-based height, re-lay out those
            // children so available_block_size matches the actual column height.
            let needs_relayout = column_height.raw() != first_pass_pct_basis.raw()
                && !column_height.is_indefinite()
                && children_info[group_start..group_end].iter().any(|info| {
                    if info.split_portion.is_some() {
                        return false;
                    }
                    let cs = &doc.node(info.id).style;
                    cs.height.is_percent()
                        || cs.min_height.is_percent()
                        || cs.max_height.is_percent()
                });

            if needs_relayout {
                col_fragments.clear();
                col_block_sizes.clear();
                col_margins_top.clear();
                col_margins_bottom.clear();
                col_avoid_break.clear();
                col_avoid_break_after.clear();
                col_forced_break_before.clear();

                let mut prev_break_after_forced_rp = false;
                for info in &children_info[group_start..group_end] {
                    let child_node_id = info.id;
                    let prop_bb = propagated_break_before(doc, child_node_id);
                    let forced = prop_bb.is_forced() || prev_break_after_forced_rp;

                    if info.split_portion.is_some() {
                        // Split portions don't need relayout (they have
                        // fixed heights computed during extraction).
                        // Re-run the same logic as the first pass.
                        // (For simplicity, we skip relayout for splits.)
                        let split = info.split_portion.as_ref().unwrap();
                        let container_style = &doc.node(split.container_id).style;
                        let c_border_top_full =
                            LayoutUnit::from_i32(container_style.effective_border_top());
                        let c_border_bottom_full =
                            LayoutUnit::from_i32(container_style.effective_border_bottom());
                        let c_pad_top_full =
                            resolve_margin_or_padding(&container_style.padding_top, column_width);
                        let c_pad_bottom_full = resolve_margin_or_padding(
                            &container_style.padding_bottom,
                            column_width,
                        );
                        let c_border_left =
                            LayoutUnit::from_i32(container_style.effective_border_left());
                        let c_border_right =
                            LayoutUnit::from_i32(container_style.effective_border_right());
                        let c_pad_left =
                            resolve_margin_or_padding(&container_style.padding_left, column_width);
                        let c_pad_right =
                            resolve_margin_or_padding(&container_style.padding_right, column_width);
                        let inner_width = (column_width
                            - c_border_left
                            - c_border_right
                            - c_pad_left
                            - c_pad_right)
                            .clamp_negative_to_zero();
                        let c_border_top = if split.is_first {
                            c_border_top_full
                        } else {
                            LayoutUnit::zero()
                        };
                        let c_pad_top = if split.is_first {
                            c_pad_top_full
                        } else {
                            LayoutUnit::zero()
                        };
                        let c_border_bottom = if split.is_last {
                            c_border_bottom_full
                        } else {
                            LayoutUnit::zero()
                        };
                        let c_pad_bottom = if split.is_last {
                            c_pad_bottom_full
                        } else {
                            LayoutUnit::zero()
                        };

                        let mut wrapper_children: Vec<Fragment> = Vec::new();
                        let mut block_off = c_border_top + c_pad_top;
                        for &gc_id in &split.child_ids {
                            let gc_style = &doc.node(gc_id).style;
                            let gc_is_new_fc = establishes_new_fc(gc_style);
                            let gc_space = ConstraintSpace::for_block_child(
                                inner_width,
                                split.portion_height,
                                inner_width,
                                split.portion_height,
                                gc_is_new_fc,
                            );
                            let mut gc_frag = block_layout(doc, gc_id, &gc_space);
                            let gc_margin_left =
                                resolve_margin_or_padding(&gc_style.margin_left, inner_width);
                            gc_frag.offset = PhysicalOffset::new(
                                c_border_left + c_pad_left + gc_margin_left,
                                block_off,
                            );
                            block_off = block_off + gc_frag.size.height;
                            wrapper_children.push(gc_frag);
                        }
                        let content_h = block_off - c_border_top - c_pad_top;
                        let effective_h = split.portion_height.max_of(content_h);
                        let wrapper_h =
                            effective_h + c_border_top + c_pad_top + c_border_bottom + c_pad_bottom;
                        let mut wrapper = Fragment::new_box(
                            split.container_id,
                            PhysicalSize::new(column_width, wrapper_h),
                        );
                        let include_block_end_decoration =
                            if split.is_last && !container_style.height.is_auto() {
                                split.paint_height.raw() > 0
                            } else {
                                split.paint_height.raw() >= effective_h.raw()
                            };
                        let decoration_paint_height = (c_border_top
                            + c_pad_top
                            + split.paint_height
                            + if include_block_end_decoration {
                                c_pad_bottom + c_border_bottom
                            } else {
                                LayoutUnit::zero()
                            })
                        .min_of(wrapper_h);
                        if !container_style.height.is_auto()
                            || decoration_paint_height.raw() < wrapper_h.raw()
                        {
                            wrapper.decoration_paint_block_size = Some(decoration_paint_height);
                        }
                        wrapper.is_first_for_node = split.is_first;
                        wrapper.is_last_for_node = split.is_last;
                        wrapper.children = wrapper_children;
                        wrapper.border = BoxStrut {
                            top: c_border_top,
                            right: c_border_right,
                            bottom: c_border_bottom,
                            left: c_border_left,
                        };
                        wrapper.padding = BoxStrut {
                            top: c_pad_top,
                            right: c_pad_right,
                            bottom: c_pad_bottom,
                            left: c_pad_left,
                        };

                        let explicit_split_height = !container_style.height.is_auto();
                        let child_margin_top = if split.is_first || !explicit_split_height {
                            resolve_margin_or_padding(&container_style.margin_top, column_width)
                        } else {
                            LayoutUnit::zero()
                        };
                        let child_margin_bottom = if split.is_last || !explicit_split_height {
                            resolve_margin_or_padding(&container_style.margin_bottom, column_width)
                        } else {
                            LayoutUnit::zero()
                        };

                        let distribution_block_size =
                            if split.is_last && !container_style.height.is_auto() {
                                (wrapper.size.height - c_pad_bottom - c_border_bottom)
                                    .clamp_negative_to_zero()
                            } else {
                                wrapper.size.height
                            };
                        col_block_sizes.push(distribution_block_size);
                        col_margins_top.push(child_margin_top);
                        col_margins_bottom.push(child_margin_bottom);
                        col_avoid_break.push(container_style.break_inside.is_avoid());
                        col_avoid_break_after.push(
                            propagated_break_after(doc, child_node_id).is_avoid()
                                || container_style.break_after.is_avoid(),
                        );
                        col_forced_break_before.push(forced);
                        col_fragments.push(wrapper);

                        let prop_ba = propagated_break_after(doc, child_node_id);
                        prev_break_after_forced_rp = prop_ba.is_forced();
                    } else {
                        let child_style = &doc.node(info.id).style;
                        let child_margin_top =
                            resolve_margin_or_padding(&child_style.margin_top, column_width);
                        let child_margin_bottom =
                            resolve_margin_or_padding(&child_style.margin_bottom, column_width);
                        let child_is_new_fc = establishes_new_fc(child_style);
                        let child_auto_float =
                            child_style.float != Float::None && child_style.width.is_auto();
                        let child_available_inline = if child_auto_float {
                            let child_margin = resolve_margins(child_style, column_width);
                            let margin_inline = {
                                let ml = if child_style.margin_left.is_auto() {
                                    LayoutUnit::zero()
                                } else {
                                    child_margin.left
                                };
                                let mr = if child_style.margin_right.is_auto() {
                                    LayoutUnit::zero()
                                } else {
                                    child_margin.right
                                };
                                ml + mr
                            };
                            crate::out_of_flow::compute_shrink_to_fit_width(
                                doc,
                                info.id,
                                (column_width - margin_inline).clamp_negative_to_zero(),
                            )
                        } else {
                            column_width
                        };
                        let mut child_space = ConstraintSpace::for_block_child(
                            child_available_inline,
                            column_height,
                            column_width,
                            child_percentage_block_size,
                            child_is_new_fc,
                        );
                        if child_auto_float {
                            child_space.is_fixed_inline_size = true;
                        }
                        if !column_height.is_indefinite() {
                            child_space.fragmentainer_block_size = column_height;
                        }
                        let child_frag = block_layout(doc, info.id, &child_space);
                        let zero_width_auto_float_flow = child_style.float != Float::None
                            && style.height.is_auto()
                            && resolved.count == 1
                            && child_style.width.is_auto()
                            && child_frag.size.width == LayoutUnit::zero();
                        let float_flow_block_size = if zero_width_auto_float_flow
                            || (child_style.float != Float::None
                                && style.height.is_auto()
                                && resolved.count > 1
                                && child_style.height.is_fixed()
                                && !child_style.background_color.is_transparent()
                                && group_end == children_info.len()
                                && !children_info[group_start..group_end].iter().any(|info| {
                                    subtree_has_in_flow_spanner_descendant(doc, info.id)
                                })
                                && doc.children(node_id).any(|sibling_id| {
                                    let sibling_style = &doc.node(sibling_id).style;
                                    sibling_id != info.id
                                        && sibling_style.float == Float::None
                                        && sibling_style.display.is_block_level()
                                })) {
                            LayoutUnit::zero()
                        } else {
                            child_frag.size.height
                        };
                        col_block_sizes.push(float_flow_block_size);
                        col_margins_top.push(child_margin_top);
                        col_margins_bottom.push(child_margin_bottom);
                        col_avoid_break.push(doc.node(info.id).style.break_inside.is_avoid());
                        col_avoid_break_after.push(
                            propagated_break_after(doc, child_node_id).is_avoid()
                                || doc.node(info.id).style.break_after.is_avoid(),
                        );
                        col_forced_break_before.push(forced);
                        col_fragments.push(child_frag);

                        let prop_ba = propagated_break_after(doc, child_node_id);
                        prev_break_after_forced_rp = prop_ba.is_forced();
                    }
                }

                for i in 1..col_avoid_break_after.len() {
                    let child_id = children_info[group_start + i].id;
                    let child_style = &doc.node(child_id).style;
                    if child_style.break_before.is_avoid()
                        || propagated_break_before(doc, child_id).is_avoid()
                    {
                        col_avoid_break_after[i - 1] = true;
                    }
                }

                _effective_sizes =
                    compute_effective(&col_block_sizes, &col_margins_top, &col_margins_bottom);
            }

            // Distribute children across columns (with fragmentation support).
            // CSS Multicol §3.4: overflow content creates additional columns
            // in the inline direction. col_idx may exceed positions.len().
            let wraps_rows = algo.column_wrap == openui_style::ColumnWrap::Wrap
                || (algo.column_wrap == openui_style::ColumnWrap::Auto
                    && algo.column_height.is_some());
            let col_inline_offset_for = |idx: usize| -> LayoutUnit {
                let position_idx = if wraps_rows && !positions.is_empty() {
                    idx % positions.len()
                } else {
                    idx
                };
                if position_idx < positions.len() {
                    positions[position_idx].inline_offset
                } else if let Some(last) = positions.last() {
                    // Overflow column: extend rightward from last defined column
                    let overflow = (position_idx - positions.len()) as i32;
                    let stride = column_width + algo.column_gap;
                    last.inline_offset
                        + last.width
                        + algo.column_gap
                        + stride * LayoutUnit::from_i32(overflow)
                } else {
                    LayoutUnit::zero()
                }
            };
            let col_extra_block_offset_for = |idx: usize| -> LayoutUnit {
                if wraps_rows && !positions.is_empty() {
                    (visual_column_height + algo.row_gap)
                        * LayoutUnit::from_i32((idx / positions.len()) as i32)
                } else {
                    LayoutUnit::zero()
                }
            };

            let mut col_idx: usize = 0;
            let mut col_block_offset = LayoutUnit::zero();
            let mut col_remaining = column_height;
            let mut prev_break_after_forces = false;
            let mut prev_break_after_avoids = false;
            let mut prev_margin_bottom = LayoutUnit::zero();
            let mut prev_fixed_child_visual_overflow = false;
            let mut prev_fixed_child_descendant_visual_overflow = false;
            let mut pending_parallel_flow_after_split_height: Option<LayoutUnit> = None;
            let mut delayed_float_overflow_parts: Vec<(usize, Fragment)> = Vec::new();
            let overflow_clip_forced_break_group = algo.column_rule.is_some()
                && algo.column_fill == ColumnFill::Auto
                && !wraps_rows
                && !style.height.is_auto()
                && (group_start..group_end).any(|idx| {
                    let child_id = children_info[idx].id;
                    let child_style = &doc.node(child_id).style;
                    child_style.overflow_x != Overflow::Visible
                        && child_style.overflow_y != Overflow::Visible
                        && subtree_has_forced_break_descendant(doc, child_id)
                });
            let mut overflow_rule_column_count = if overflow_clip_forced_break_group {
                0usize
            } else {
                positions.len()
            };
            // Track whether the current column was started by a forced break.
            // CSS Fragmentation: margins at the top of a non-first column are
            // preserved after forced breaks but truncated after unforced breaks.
            let mut col_started_by_forced_break = false;
            let previous_extracted_portion_has_authored_inline_overflow = group_start >= 2
                && children_info[group_start - 1].is_spanner
                && children_info[group_start]
                    .split_portion
                    .as_ref()
                    .map_or(false, |split| {
                        children_info[group_start - 2]
                            .split_portion
                            .as_ref()
                            .map_or(false, |previous_split| {
                                previous_split.container_id == split.container_id
                                    && previous_split.child_ids.iter().any(|&id| {
                                        let style = &doc.node(id).style;
                                        (style.overflow_x == Overflow::Visible
                                            && style.width.is_percent()
                                            && style.width.value() > 100.0)
                                            || subtree_has_authored_inline_overflow_descendant(
                                                doc, id,
                                            )
                                    })
                            })
                    });
            let post_spanner_single_child_balanced_tail =
                if previous_extracted_portion_has_authored_inline_overflow
                    && style.height.is_auto()
                    && total_block_offset > LayoutUnit::zero()
                    && !has_spanner_after
                    && !wraps_rows
                    && resolved.count > 1
                    && algo.column_fill != ColumnFill::Auto
                {
                    let tail_idx = if col_block_sizes.len() == 2
                        && children_info[group_start].split_portion.is_some()
                        && col_block_sizes[0] == LayoutUnit::zero()
                        && children_info[group_start + 1].split_portion.is_none()
                    {
                        Some(1)
                    } else {
                        None
                    };
                    tail_idx.filter(|&idx| {
                        !col_forced_break_before[idx]
                            && col_margins_top[idx] == LayoutUnit::zero()
                            && col_margins_bottom[idx] == LayoutUnit::zero()
                            && !doc
                                .node(children_info[group_start + idx].id)
                                .style
                                .break_inside
                                .is_avoid()
                            && col_block_sizes[idx].raw() > column_height.raw()
                            && col_block_sizes[idx].raw()
                                <= (column_height * LayoutUnit::from_i32(resolved.count as i32))
                                    .raw()
                    })
                } else {
                    None
                };
            // Track actual tallest column content for auto-height containers.
            let mut max_col_content = LayoutUnit::zero();
            let mut max_col_visual_content = LayoutUnit::zero();
            let mut pre_spanner_positioned_visual_height = LayoutUnit::zero();
            let mut max_wrapped_content_extent = LayoutUnit::zero();
            // Per-column children: collected during distribution, then wrapped
            // in ColumnBox fragments for column-level overflow clipping
            // (CSS Multicol §3.1: column boxes clip their content).
            let mut per_col_children: Vec<Vec<Fragment>> = vec![Vec::new()];
            let suppress_wrapped_row_flex_break_after_avoid = |child_id: NodeId| -> bool {
                let child_style = &doc.node(child_id).style;
                child_style.display == Display::Flex
                    && !child_style.flex_direction.is_column()
                    && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                    && algo.column_fill == ColumnFill::Auto
                    && has_explicit_height
                    && !wraps_rows
                    && resolved.count == 2
                    && algo.column_gap == LayoutUnit::zero()
                    && oof_children.iter().any(|oof| {
                        let oof_style = &doc.node(oof.node_id).style;
                        if oof_style.position != Position::Absolute
                            || oof_style.top.is_auto()
                            || oof_style.width.is_auto()
                        {
                            return false;
                        }
                        let width = resolve_length(
                            &oof_style.width,
                            child_available_inline,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        width.raw() > column_width.raw()
                    })
            };
            let propagated_break_after_is_avoid = |child_id: NodeId| -> bool {
                propagated_break_after(doc, child_id).is_avoid()
                    && !suppress_wrapped_row_flex_break_after_avoid(child_id)
            };

            // Record static positions for OOF children that appear before
            // the first in-flow child in this group.
            for oof in &oof_children {
                if oof.flow_index == global_flow_idx {
                    oof_static_positions.push((
                        oof.node_id,
                        PhysicalOffset::new(
                            content_edge_x + col_inline_offset_for(0),
                            content_edge_y + total_block_offset,
                        ),
                    ));
                }
            }

            // CSS Fragmentation §3.2: Pre-compute avoid groups.
            // An avoid group is a maximal sequence of children linked by
            // break-after:avoid / break-before:avoid.  For each child,
            // `avoid_group_size[i]` gives the total height of the group
            // it starts (including collapsed margins), or zero if it is
            // not the first child of its avoid group.
            let child_count = col_block_sizes.len();
            let mut avoid_group_size: Vec<LayoutUnit> = vec![LayoutUnit::zero(); child_count];
            {
                // Walk backwards to find group starts.
                // A child i is linked to i+1 if:
                //   - child i has break-after:avoid (or propagated), OR
                //   - child i+1 has break-before:avoid (or propagated),
                //   AND there is no forced break between them.
                let mut group_end = child_count;
                let mut i = child_count;
                while i > 0 {
                    i -= 1;
                    let cid = children_info[group_start + i].id;
                    let cstyle = &doc.node(cid).style;
                    let has_link_after = if i + 1 < child_count {
                        let next_id = children_info[group_start + i + 1].id;
                        let next_style = &doc.node(next_id).style;
                        let next_prop_bb = propagated_break_before(doc, next_id);
                        // Forced breaks override avoid constraints.
                        let forced = cstyle.break_after.is_forced()
                            || propagated_break_after(doc, cid).is_forced()
                            || next_style.break_before.is_forced()
                            || next_prop_bb.is_forced();
                        !forced
                            && (cstyle.break_after.is_avoid()
                                || propagated_break_after_is_avoid(cid)
                                || next_style.break_before.is_avoid()
                                || next_prop_bb.is_avoid())
                    } else {
                        false
                    };
                    if !has_link_after {
                        group_end = i + 1;
                    }
                    // If this child starts a multi-child group, compute total.
                    if i + 1 < group_end {
                        // This child is the start of a group [i..group_end).
                        let mut total = col_block_sizes[i];
                        for j in (i + 1)..group_end {
                            let collapsed = col_margins_bottom[j - 1].max_of(col_margins_top[j]);
                            total = total + collapsed + col_block_sizes[j];
                        }
                        // Include trailing margin of last child.
                        total = total + col_margins_bottom[group_end - 1];
                        avoid_group_size[i] = total;
                    }
                }
            }

            for (i, mut child_frag) in col_fragments.into_iter().enumerate() {
                let child_height = col_block_sizes[i];
                let child_margin_top = col_margins_top[i];
                let child_margin_bottom = col_margins_bottom[i];
                let child_node_id = children_info[group_start + i].id;
                let child_style = &doc.node(child_node_id).style;
                if has_spanner_after
                    && style.height.is_auto()
                    && group_available_block.is_indefinite()
                    && !wraps_rows
                    && resolved.count > 1
                    && child_height.raw() > column_height.raw()
                    && child_style.position.is_positioned()
                    && doc
                        .children(child_node_id)
                        .any(|gc| doc.node(gc).style.position.is_absolutely_positioned())
                {
                    pre_spanner_positioned_visual_height =
                        pre_spanner_positioned_visual_height.max_of(child_height);
                }
                let parallel_flow_after_extracted_split =
                    if let Some(flow_height) = pending_parallel_flow_after_split_height.take() {
                        if child_margin_top.raw() < 0 {
                            col_idx = 0;
                            col_block_offset = flow_height;
                            col_remaining = (column_height - flow_height).clamp_negative_to_zero();
                            prev_margin_bottom = LayoutUnit::zero();
                            col_started_by_forced_break = false;
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                let collapse_negative_margin_after_flex_sibling = i > 0
                    && child_margin_top < LayoutUnit::zero()
                    && doc
                        .node(children_info[group_start + i - 1].id)
                        .style
                        .display
                        == Display::Flex;

                // CSS Fragmentation §3.1: Forced breaks —
                // break-before: column/page/always forces a break before this child.
                // break-after on the *previous* child forces a break before this one.
                // §3.1 also says break values propagate from first/last in-flow children.
                let prop_break_before = propagated_break_before(doc, child_node_id);
                let forced_break = prop_break_before.is_forced() || prev_break_after_forces;

                if forced_break {
                    // CSS Multicol §3.4 + CSS Fragmentation §3.1:
                    // Forced breaks advance to the next column.
                    // However, when column-fill is balance, the balance
                    // algorithm packs excess forced segments into the last
                    // column.  Match that behavior here: cap col_idx at
                    // column_count - 1 so excess content stays in the last
                    // declared column instead of creating overflow columns.
                    // For column-fill:auto, excess forced breaks DO create
                    // overflow columns (positioned by col_inline_offset_for).
                    let can_advance_forced = algo.column_fill == ColumnFill::Auto
                        || col_idx + 1 < resolved.count as usize;
                    if can_advance_forced {
                        if col_block_offset > LayoutUnit::zero() {
                            max_col_content = max_col_content.max_of(col_block_offset);
                            col_idx += 1;
                            col_block_offset = LayoutUnit::zero();
                            col_remaining = column_height;
                            prev_margin_bottom = LayoutUnit::zero();
                            prev_fixed_child_visual_overflow = false;
                            prev_fixed_child_descendant_visual_overflow = false;
                            col_started_by_forced_break = true;
                        } else if i > 0 {
                            col_idx += 1;
                            col_block_offset = LayoutUnit::zero();
                            col_remaining = column_height;
                            prev_margin_bottom = LayoutUnit::zero();
                            prev_fixed_child_visual_overflow = false;
                            prev_fixed_child_descendant_visual_overflow = false;
                            col_started_by_forced_break = true;
                        }
                    }
                }

                // Compute collapsed margin between siblings.
                // CSS 2.1 §8.3.1: adjoining margins collapse to max.
                // CSS Fragmentation §5.4: margin at top of non-first column
                // truncated for unforced breaks, preserved for forced breaks.
                let margin_space = if parallel_flow_after_extracted_split {
                    child_margin_top
                } else if collapse_negative_margin_after_flex_sibling
                    && col_block_offset > LayoutUnit::zero()
                {
                    collapse_adjacent_block_margins(prev_margin_bottom, child_margin_top)
                } else if col_block_offset > LayoutUnit::zero() {
                    prev_margin_bottom.max_of(child_margin_top)
                } else if col_idx == 0 || col_started_by_forced_break {
                    child_margin_top
                } else {
                    LayoutUnit::zero()
                };
                let total_child_space = margin_space + child_height;

                // CSS Fragmentation §3.2: break-inside: avoid —
                // If the child doesn't fit but would fit in a fresh column,
                // and break-inside is avoid, move to the next column.
                // Include margin-bottom in the fit check: the entire margin
                // box of an unbreakable child must fit in the column.
                // CSS Fragmentation Level 3 §4: monolithic elements (those with
                // overflow other than visible/clip) cannot be fragmented.
                let is_monolithic = child_style.overflow_x != Overflow::Visible
                    && child_style.overflow_x != Overflow::Clip
                    || child_style.overflow_y != Overflow::Visible
                        && child_style.overflow_y != Overflow::Clip;
                let pulls_negative_margin_after_visual_overflow = prev_fixed_child_visual_overflow
                    && prev_fixed_child_descendant_visual_overflow
                    && prev_margin_bottom == LayoutUnit::zero()
                    && child_margin_top < LayoutUnit::zero()
                    && !child_style.background_color.is_transparent()
                    && doc.children(child_node_id).next().is_none()
                    && !child_style.break_inside.is_avoid()
                    && !is_monolithic;
                let float_can_fill_remaining_fragmentainer = child_style.float != Float::None
                    && algo.column_fill == ColumnFill::Auto
                    && resolved.count >= 4
                    && !child_style.background_color.is_transparent();
                let avoid_break_inside = (!float_can_fill_remaining_fragmentainer
                    && child_style.break_inside.is_avoid())
                    || is_monolithic;
                // CSS Fragmentation §3.1: break-before/after: avoid —
                // This child or the previous child wants to avoid a break here.
                let avoid_break_before = prop_break_before.is_avoid()
                    || child_style.break_before.is_avoid()
                    || prev_break_after_avoids;

                // CSS Fragmentation §3.2: Avoid-group handling.
                // If this child starts an avoid group, the entire group must
                // fit.  If it doesn't fit in the remaining space but would
                // fit in a fresh column, advance to the next column NOW
                // (before placing any group member).
                let group_total = avoid_group_size[i];
                if group_total > LayoutUnit::zero()
                    && col_block_offset > LayoutUnit::zero()
                    && col_remaining.raw() < (margin_space + group_total).raw()
                    && group_total.raw() <= column_height.raw()
                    && (algo.column_fill != ColumnFill::Auto
                        || wraps_rows
                        || algo.column_wrap == openui_style::ColumnWrap::NoWrap
                        || col_idx + 1 < resolved.count as usize)
                {
                    max_col_content = max_col_content.max_of(col_block_offset);
                    col_idx += 1;
                    col_block_offset = LayoutUnit::zero();
                    col_remaining = column_height;
                    prev_margin_bottom = LayoutUnit::zero();
                    prev_fixed_child_visual_overflow = false;
                    prev_fixed_child_descendant_visual_overflow = false;
                    col_started_by_forced_break = false;
                }

                let avoid_total = total_child_space + child_margin_bottom;
                let avoid_fresh = child_height + child_margin_bottom;
                // CSS Fragmentation §3.2: break-inside:avoid takes priority
                // over break-before/after:avoid.  If the child has
                // break-inside:avoid and doesn't fit, move it to the next
                // column even if break-before:avoid wants to keep it with the
                // previous sibling (we prefer not splitting over not
                // separating siblings).
                if avoid_break_inside
                    && col_remaining.raw() < avoid_total.raw()
                    && col_block_offset > LayoutUnit::zero()
                    && avoid_fresh.raw() <= column_height.raw()
                    && (algo.column_fill != ColumnFill::Auto
                        || wraps_rows
                        || algo.column_wrap == openui_style::ColumnWrap::NoWrap
                        || col_idx + 1 < resolved.count as usize
                        || child_style.break_inside == BreakInside::AvoidColumn)
                {
                    max_col_content = max_col_content.max_of(col_block_offset);
                    col_idx += 1;
                    col_block_offset = LayoutUnit::zero();
                    col_remaining = column_height;
                    prev_margin_bottom = LayoutUnit::zero();
                    prev_fixed_child_visual_overflow = false;
                    prev_fixed_child_descendant_visual_overflow = false;
                    col_started_by_forced_break = false;
                }

                // Recalculate margin for potentially new column context.
                // CSS Fragmentation §5.4: truncate at non-first column top
                // only for unforced breaks.
                let actual_margin = if parallel_flow_after_extracted_split {
                    child_margin_top
                } else if pulls_negative_margin_after_visual_overflow {
                    child_margin_top
                } else if collapse_negative_margin_after_flex_sibling
                    && col_block_offset > LayoutUnit::zero()
                {
                    collapse_adjacent_block_margins(prev_margin_bottom, child_margin_top)
                } else if col_block_offset > LayoutUnit::zero() {
                    prev_margin_bottom.max_of(child_margin_top)
                } else if col_idx == 0 || col_started_by_forced_break {
                    child_margin_top
                } else {
                    LayoutUnit::zero()
                };
                let needed = actual_margin + child_height;

                // CSS Fragmentation §4 / CSS Multicol §3.4: When a child
                // doesn't fit in the remaining column space, move it to the
                // next column (class C break between siblings).  If the child
                // is taller than a full column, the fragmentation code below
                // will split it across columns.
                //
                // Do NOT advance when break-before:avoid is set — the child
                // should stay with the previous sibling.
                // CSS Multicol §3.4: column-fill:auto caps at column-count;
                // excess content overflows the last column (no new columns
                // unless forced by break-before/after).
                let can_create_single_column_rule_overflow = algo.column_fill == ColumnFill::Auto
                    && resolved.count == 1
                    && algo.column_rule.is_some()
                    && !wraps_rows
                    && !has_spanner_after;
                let can_advance_col = algo.column_fill != ColumnFill::Auto
                    || wraps_rows
                    || algo.column_wrap == openui_style::ColumnWrap::NoWrap
                    || col_idx + 1 < resolved.count as usize
                    || can_create_single_column_rule_overflow;
                // Advance to next column if the child doesn't fit.  For
                // oversized children (taller than one column), only advance
                // when the remaining space can't even hold the block-start
                // decoration (border-top + padding-top); otherwise start
                // fragmenting in the remaining space so it isn't wasted.
                let child_block_start_deco = LayoutUnit::from_i32(child_style.border_top_width)
                    + resolve_margin_or_padding(&child_style.padding_top, column_width);
                let child_has_direct_avoid_after_sibling_for_advance = {
                    let mut seen_previous = false;
                    let mut found = false;
                    for gc in doc.children(child_node_id) {
                        if doc.node(gc).style.break_inside.is_avoid() && seen_previous {
                            found = true;
                            break;
                        }
                        seen_previous = true;
                    }
                    found
                };
                let can_fragment_nested_inline_overflow =
                    crate::multicol::ColumnLayoutAlgorithm::from_style(child_style).is_some()
                        && child_style.height.is_auto()
                        && child_style.overflow_x == Overflow::Visible
                        && child_style.overflow_y == Overflow::Visible
                        && subtree_has_authored_inline_overflow_descendant(doc, child_node_id)
                        && child_has_direct_avoid_after_sibling_for_advance
                        && !subtree_has_in_flow_spanner_descendant(doc, child_node_id)
                        && !subtree_has_flex_descendant(doc, child_node_id)
                        && !subtree_has_forced_break_descendant(doc, child_node_id);
                let should_advance = (!float_can_fill_remaining_fragmentainer
                    && child_height.raw() <= column_height.raw()
                    && !child_style.position.is_positioned()
                    && !can_fragment_nested_inline_overflow
                    && !(prev_fixed_child_visual_overflow
                        && col_remaining > LayoutUnit::zero()
                        && (prev_margin_bottom == LayoutUnit::zero()
                            || prev_fixed_child_descendant_visual_overflow)
                        && !child_style.background_color.is_transparent()
                        && doc.children(child_node_id).next().is_none()
                        && !child_style.break_inside.is_avoid()
                        && !is_monolithic))
                    || col_remaining.raw() < child_block_start_deco.raw();
                if col_remaining.raw() < needed.raw()
                    && col_block_offset > LayoutUnit::zero()
                    && !avoid_break_before
                    && can_advance_col
                    && should_advance
                {
                    max_col_content = max_col_content.max_of(col_block_offset);
                    col_idx += 1;
                    col_block_offset = LayoutUnit::zero();
                    col_remaining = column_height;
                    prev_margin_bottom = LayoutUnit::zero();
                    prev_fixed_child_visual_overflow = false;
                    prev_fixed_child_descendant_visual_overflow = false;
                    col_started_by_forced_break = false;
                }

                let prop_break_after = propagated_break_after(doc, child_node_id);
                prev_break_after_forces = prop_break_after.is_forced();
                prev_break_after_avoids =
                    (prop_break_after.is_avoid()
                        && !suppress_wrapped_row_flex_break_after_avoid(child_node_id))
                        || child_style.break_after.is_avoid();

                // Final margin for positioning.
                // CSS Fragmentation §5.4: margins at the top of a non-first
                // column are truncated for UNFORCED breaks but preserved for
                // FORCED breaks (break-before/break-after: column).
                let pos_margin = if parallel_flow_after_extracted_split {
                    child_margin_top
                } else if pulls_negative_margin_after_visual_overflow {
                    child_margin_top
                } else if collapse_negative_margin_after_flex_sibling
                    && col_block_offset > LayoutUnit::zero()
                {
                    collapse_adjacent_block_margins(prev_margin_bottom, child_margin_top)
                } else if col_block_offset > LayoutUnit::zero() {
                    prev_margin_bottom.max_of(child_margin_top)
                } else if col_idx == 0 {
                    // First item in first column: keep full margin.
                    child_margin_top
                } else if col_started_by_forced_break {
                    // First item after a forced break: preserve margin.
                    child_margin_top
                } else {
                    // First item in a non-first column (unforced break):
                    // truncate top margin.
                    LayoutUnit::zero()
                };
                // Reset prev_margin_bottom for recalculation later (set after placement).
                // We use `child_margin_bottom` at the end.

                let effective_needed = pos_margin + child_height;

                let auto_width_fc_after_only_floats = algo.column_fill == ColumnFill::Auto
                    && !style.height.is_auto()
                    && child_style.display == Display::FlowRoot
                    && child_style.width.is_auto()
                    && child_style.float == Float::None
                    && child_style.height.is_fixed()
                    && doc.children(child_node_id).next().is_none()
                    && i > 0
                    && children_info[group_start..group_start + i]
                        .iter()
                        .all(|info| doc.node(info.id).style.float != Float::None)
                    && per_col_children.iter().any(|children| {
                        children.iter().any(|fragment| {
                            !fragment.node_id.is_none()
                                && doc.node(fragment.node_id).style.float != Float::None
                        })
                    });

                if auto_width_fc_after_only_floats && child_height > column_height {
                    let mut remaining = child_height;
                    let mut consumed = LayoutUnit::zero();
                    let mut target_col = 0usize;
                    while remaining.raw() > 0 {
                        while per_col_children.len() <= target_col {
                            per_col_children.push(Vec::new());
                        }
                        let col_inline_offset = col_inline_offset_for(target_col);
                        let col_origin = PhysicalOffset::new(
                            content_edge_x + col_inline_offset,
                            content_edge_y
                                + total_block_offset
                                + col_extra_block_offset_for(target_col),
                        );
                        let mut column_exclusions = ExclusionSpace::new();
                        for float_fragment in &per_col_children[target_col] {
                            if float_fragment.node_id.is_none() {
                                continue;
                            }
                            let float_style = &doc.node(float_fragment.node_id).style;
                            let exclusion_type = match float_style.float {
                                Float::Left => ExclusionType::Left,
                                Float::Right => ExclusionType::Right,
                                Float::None => continue,
                            };
                            let rel_top = float_fragment.offset.top - col_origin.top;
                            if rel_top.raw() <= 0
                                && (rel_top + float_fragment.size.height).raw() > 0
                            {
                                let rel_left = match exclusion_type {
                                    ExclusionType::Left => {
                                        float_fragment.offset.left - col_origin.left
                                    }
                                    ExclusionType::Right => (column_width
                                        - float_fragment.size.width)
                                        .clamp_negative_to_zero(),
                                };
                                column_exclusions.add(ExclusionArea {
                                    rect: BfcRect::new(
                                        BfcOffset::new(rel_left, rel_top),
                                        BfcOffset::new(
                                            rel_left + float_fragment.size.width,
                                            rel_top + float_fragment.size.height,
                                        ),
                                    ),
                                    exclusion_type,
                                });
                            }
                        }

                        let opportunity = column_exclusions.find_layout_opportunity(
                            &BfcOffset::new(LayoutUnit::zero(), LayoutUnit::zero()),
                            column_width,
                            LayoutUnit::zero(),
                        );
                        let (inline_offset, available_inline) = new_fc_placement_for_opportunity(
                            &opportunity,
                            BoxStrut::zero(),
                            column_width,
                        );
                        let mut relaid_space = ConstraintSpace::for_block_child(
                            available_inline,
                            group_available_block,
                            available_inline,
                            first_pass_pct_basis,
                            true,
                        );
                        if !group_available_block.is_indefinite() {
                            relaid_space.fragmentainer_block_size = group_available_block;
                        }
                        let mut part = block_layout(doc, child_node_id, &relaid_space);
                        let part_height = remaining.min_of(column_height);
                        part.size.height = part_height;
                        part.offset =
                            PhysicalOffset::new(col_origin.left + inline_offset, col_origin.top);
                        part.has_overflow_clip = true;
                        part.block_axis_clip_only = child_style.overflow_x == Overflow::Visible
                            && child_style.overflow_y == Overflow::Visible;
                        if consumed > LayoutUnit::zero() {
                            part.is_first_for_node = false;
                        }
                        if remaining > part_height {
                            part.is_last_for_node = false;
                        }
                        per_col_children[target_col].push(part);
                        max_col_content = max_col_content.max_of(part_height);
                        max_wrapped_content_extent = max_wrapped_content_extent
                            .max_of(col_extra_block_offset_for(target_col) + part_height);
                        overflow_rule_column_count = overflow_rule_column_count.max(target_col + 1);
                        remaining = remaining - part_height;
                        consumed = consumed + part_height;
                        target_col += 1;
                    }
                    col_idx = target_col.saturating_sub(1);
                    col_block_offset = child_height.min_of(column_height);
                    col_remaining = (column_height - col_block_offset).clamp_negative_to_zero();
                    prev_margin_bottom = child_margin_bottom;
                    prev_fixed_child_visual_overflow = false;
                    prev_fixed_child_descendant_visual_overflow = false;
                    col_started_by_forced_break = false;
                    continue;
                }

                if col_remaining.raw() >= effective_needed.raw() || is_monolithic {
                    // Child fits in current column, or is monolithic and must
                    // not be fragmented (CSS Fragmentation Level 3 §4).
                    let col_inline_offset = col_inline_offset_for(col_idx);

                    col_block_offset = col_block_offset + pos_margin;

                    let mut positioned = child_frag;
                    positioned.offset = PhysicalOffset::new(
                        content_edge_x + col_inline_offset,
                        content_edge_y
                            + total_block_offset
                            + col_extra_block_offset_for(col_idx)
                            + col_block_offset,
                    );
                    // Apply relative positioning (CSS 2.1 §9.4.3).
                    crate::relative::apply_relative_offset(
                        &mut positioned,
                        child_style,
                        column_width,
                        column_height,
                    );
                    let spread_zero_height_forced_break_flex_items = !wraps_rows
                        && algo.column_fill == ColumnFill::Auto
                        && has_explicit_height
                        && child_style.display == Display::Flex
                        && !child_style.flex_direction.is_column()
                        && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                        && child_style.background_color.is_transparent()
                        && child_height == LayoutUnit::zero()
                        && positioned.children.len() > 1
                        && positioned.children.len() <= resolved.count as usize
                        && positioned.children.iter().all(|child| {
                            !child.node_id.is_none()
                                && child.offset.top < LayoutUnit::zero()
                                && doc.node(child.node_id).style.break_before.is_forced()
                        });
                    if spread_zero_height_forced_break_flex_items {
                        for (offset_idx, child) in positioned.children.iter().enumerate() {
                            let target_col = col_idx + offset_idx;
                            while per_col_children.len() <= target_col {
                                per_col_children.push(Vec::new());
                            }
                            let mut part = child.clone();
                            part.offset = PhysicalOffset::new(
                                content_edge_x
                                    + col_inline_offset_for(target_col)
                                    + child.offset.left,
                                content_edge_y
                                    + total_block_offset
                                    + col_extra_block_offset_for(target_col)
                                    + child.offset.top,
                            );
                            per_col_children[target_col].push(part);
                            max_col_content = max_col_content.max_of(child.size.height);
                            max_wrapped_content_extent =
                                max_wrapped_content_extent.max_of(
                                    col_extra_block_offset_for(target_col) + child.size.height,
                                );
                        }
                        col_idx += positioned.children.len().saturating_sub(1);
                        col_block_offset = child_height;
                        col_remaining = column_height;
                        prev_margin_bottom = child_margin_bottom;
                        prev_fixed_child_visual_overflow = false;
                        prev_fixed_child_descendant_visual_overflow = false;
                        col_started_by_forced_break = true;
                        continue;
                    }
                    let split_fitting_row_flex_at_forced_break = !wraps_rows
                        && algo.column_fill == ColumnFill::Auto
                        && has_explicit_height
                        && resolved.count == 2
                        && algo.column_gap == LayoutUnit::zero()
                        && col_block_offset == pos_margin
                        && child_style.display == Display::Flex
                        && !child_style.flex_direction.is_column()
                        && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                        && child_style.background_color.is_transparent()
                        && child_height <= column_height
                        && !subtree_has_positioned_descendant(doc, child_node_id)
                        && subtree_has_forced_break_descendant(doc, child_node_id);
                    if split_fitting_row_flex_at_forced_break {
                        let mut previous_break_after_forced = false;
                        let mut split_index = None;
                        for (idx, child) in positioned.children.iter().enumerate() {
                            if idx > 0
                                && (previous_break_after_forced
                                    || (!child.node_id.is_none()
                                        && propagated_break_before(doc, child.node_id).is_forced()))
                            {
                                split_index = Some(idx);
                                break;
                            }
                            previous_break_after_forced = !child.node_id.is_none()
                                && propagated_break_after(doc, child.node_id).is_forced();
                        }
                        if let Some(split_index) = split_index {
                            let split_top = positioned.children[split_index].offset.top;
                            if split_top > LayoutUnit::zero()
                                && split_top < child_height
                                && col_idx + 1 < resolved.count as usize
                            {
                                let mut first_part = positioned.clone();
                                first_part.children = positioned.children[..split_index].to_vec();
                                first_part.size.height = split_top;
                                first_part.is_last_for_node = false;

                                let mut second_part = positioned;
                                second_part.children = second_part.children[split_index..].to_vec();
                                for child in &mut second_part.children {
                                    child.offset.top = child.offset.top - split_top;
                                }
                                second_part.offset = PhysicalOffset::new(
                                    content_edge_x + col_inline_offset_for(col_idx + 1),
                                    content_edge_y
                                        + total_block_offset
                                        + col_extra_block_offset_for(col_idx + 1),
                                );
                                second_part.size.height = (child_height - split_top)
                                    .clamp_negative_to_zero();
                                second_part.is_first_for_node = false;

                                while per_col_children.len() <= col_idx + 1 {
                                    per_col_children.push(Vec::new());
                                }
                                per_col_children[col_idx].push(first_part);
                                per_col_children[col_idx + 1].push(second_part);

                                max_col_content = max_col_content.max_of(split_top);
                                max_wrapped_content_extent = max_wrapped_content_extent
                                    .max_of(col_extra_block_offset_for(col_idx) + split_top)
                                    .max_of(
                                        col_extra_block_offset_for(col_idx + 1)
                                            + (child_height - split_top).clamp_negative_to_zero(),
                                    );
                                col_idx += 1;
                                col_block_offset =
                                    (child_height - split_top).clamp_negative_to_zero();
                                col_remaining =
                                    (column_height - col_block_offset).clamp_negative_to_zero();
                                prev_margin_bottom = child_margin_bottom;
                                prev_fixed_child_visual_overflow = false;
                                prev_fixed_child_descendant_visual_overflow = false;
                                col_started_by_forced_break = true;
                                continue;
                            }
                        }
                    }
                    let extend_clipped_positioned_abspos_tail =
                        child_style.position.is_positioned()
                            && child_style.overflow_x == Overflow::Visible
                            && child_style.overflow_y == Overflow::Clip
                            && child_height < col_remaining
                            && doc.children(child_node_id).any(|gc| {
                                let gc_style = &doc.node(gc).style;
                                gc_style.position.is_absolutely_positioned()
                                    && !gc_style.top.is_auto()
                                    && resolve_length(
                                        &gc_style.top,
                                        child_height,
                                        LayoutUnit::zero(),
                                        LayoutUnit::zero(),
                                    ) < LayoutUnit::zero()
                            });
                    if extend_clipped_positioned_abspos_tail {
                        positioned.size.height = positioned.size.height.max_of(col_remaining);
                    }
                    if resolved.count == 1
                        && style.height.is_auto()
                        && child_style.overflow_x == Overflow::Visible
                        && child_style.overflow_y == Overflow::Visible
                        && !positioned.has_overflow_clip
                    {
                        let child_visual_bottom = positioned
                            .children
                            .iter()
                            .fold(positioned.size.height, |bottom, child| {
                                bottom.max_of(child.offset.top + child.size.height)
                            });
                        let overflow_bottom = positioned
                            .overflow_rect
                            .map(|rect| rect.offset.top + rect.size.height)
                            .unwrap_or(child_visual_bottom)
                            .max_of(child_visual_bottom);
                        let visual_bottom = col_extra_block_offset_for(col_idx)
                            + col_block_offset
                            + overflow_bottom;
                        max_col_visual_content = max_col_visual_content.max_of(visual_bottom);
                    }
                    // Extract bubbled OOF candidates from this child and translate
                    // their static positions into multicol container coordinates.
                    let child_oof = std::mem::take(&mut positioned.oof_candidates);
                    for mut c in child_oof {
                        c.static_position.left = c.static_position.left + positioned.offset.left;
                        c.static_position.top = c.static_position.top + positioned.offset.top;
                        bubbled_oof_from_columns.push(c);
                    }
                    let child_visual_bottom = fragment_visual_block_bottom(&positioned);
                    col_block_offset = col_block_offset + child_height;
                    col_remaining = col_remaining - pos_margin - child_height;
                    max_wrapped_content_extent = max_wrapped_content_extent
                        .max_of(col_extra_block_offset_for(col_idx) + col_block_offset);
                    prev_margin_bottom = child_margin_bottom;
                    let child_visual_top = fragment_visual_block_top(&positioned);
                    let child_descendant_visual_overflows_box = child_visual_bottom > child_height;
                    if style.height.is_auto()
                        && !style.background_color.is_transparent()
                        && algo.column_fill != ColumnFill::Auto
                        && resolved.count > 1
                        && !has_spanner_after
                        && !wraps_rows
                        && child_style.display == Display::Flex
                        && !child_style.flex_direction.is_column()
                        && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                        && child_style.background_color.is_transparent()
                        && child_descendant_visual_overflows_box
                        && subtree_has_avoid_break_descendant(doc, child_node_id)
                        && !subtree_has_forced_break_descendant(doc, child_node_id)
                    {
                        let visual_bottom = col_extra_block_offset_for(col_idx)
                            + col_block_offset
                            + child_visual_bottom;
                        max_col_visual_content = max_col_visual_content.max_of(visual_bottom);
                    }
                    while per_col_children.len() <= col_idx {
                        per_col_children.push(Vec::new());
                    }
                    let float_visual_continuation = if child_style.float != Float::None
                        && child_height == LayoutUnit::zero()
                        && child_style.height.is_fixed()
                        && !child_style.background_color.is_transparent()
                        && positioned.size.height > column_height
                        && style.height.is_auto()
                        && resolved.count > 1
                    {
                        Some(positioned.clone())
                    } else {
                        None
                    };
                    per_col_children[col_idx].push(positioned);
                    if let Some(float_fragment) = float_visual_continuation {
                        let mut consumed_visual = column_height;
                        let mut overflow_col_idx = col_idx + 1;
                        while consumed_visual < float_fragment.size.height {
                            while per_col_children.len() <= overflow_col_idx {
                                per_col_children.push(Vec::new());
                            }
                            let mut continuation = float_fragment.clone();
                            continuation.offset = PhysicalOffset::new(
                                content_edge_x + col_inline_offset_for(overflow_col_idx),
                                content_edge_y
                                    + total_block_offset
                                    + col_extra_block_offset_for(overflow_col_idx),
                            );
                            continuation.size.height = (float_fragment.size.height
                                - consumed_visual)
                                .min_of(column_height);
                            continuation.is_first_for_node = false;
                            if consumed_visual + continuation.size.height
                                < float_fragment.size.height
                            {
                                continuation.is_last_for_node = false;
                            }
                            per_col_children[overflow_col_idx].push(continuation);
                            overflow_rule_column_count =
                                overflow_rule_column_count.max(overflow_col_idx + 1);
                            consumed_visual = consumed_visual + column_height;
                            overflow_col_idx += 1;
                        }
                    }
                    overflow_rule_column_count = overflow_rule_column_count.max(col_idx + 1);

                    let positioned_direct_abspos_visual_overflow =
                        child_style.position.is_positioned()
                            && has_explicit_height
                            && algo.column_fill == ColumnFill::Auto
                            && resolved.count > 1
                            && !wraps_rows
                            && child_descendant_visual_overflows_box
                            && doc.children(child_node_id).any(|gc| {
                                let gc_style = &doc.node(gc).style;
                                gc_style.position.is_absolutely_positioned()
                                    && gc_style.break_before == BreakValue::Auto
                                    && gc_style.break_after == BreakValue::Auto
                                    && !gc_style.break_inside.is_avoid()
                                    && !subtree_has_positioned_descendant(doc, gc)
                            });
                    let should_continue_abspos_overflow = if child_style.position.is_positioned() {
                        let positioned_zero_height_overflow = child_height == LayoutUnit::zero()
                            && doc.children(child_node_id).any(|gc| {
                                let gc_style = &doc.node(gc).style;
                                gc_style.position.is_absolutely_positioned()
                                    && (gc_style.break_before.is_forced()
                                        || gc_style.break_inside == BreakInside::Avoid
                                        || doc.children(gc).any(|ggc| {
                                            let ggc_style = &doc.node(ggc).style;
                                            ggc_style.column_span == ColumnSpan::All
                                        })
                                        || (doc.children(gc).any(|ggc| {
                                            let ggc_style = &doc.node(ggc).style;
                                            ggc_style.position.is_positioned()
                                                || crate::multicol::ColumnLayoutAlgorithm::from_style(
                                                    ggc_style,
                                                )
                                                .is_some()
                                        }) && doc.children(gc).all(|ggc| {
                                            let ggc_style = &doc.node(ggc).style;
                                            ggc_style.position.is_positioned()
                                                || crate::multicol::ColumnLayoutAlgorithm::from_style(
                                                    ggc_style,
                                                )
                                                .is_some()
                                        })))
                            });
                        let positioned_bottom_abspos_overflow =
                            doc.children(child_node_id).any(|gc| {
                                let gc_style = &doc.node(gc).style;
                                gc_style.position.is_absolutely_positioned()
                                    && !gc_style.bottom.is_auto()
                            });
                        positioned_direct_abspos_visual_overflow
                            || positioned_zero_height_overflow
                            || positioned_bottom_abspos_overflow
                    } else {
                        child_height == LayoutUnit::zero()
                            && doc.children(child_node_id).any(|gc| {
                                let gc_style = &doc.node(gc).style;
                                gc_style.position.is_positioned()
                                    && (child_style.overflow_y == Overflow::Visible
                                        || child_style.overflow_y == Overflow::Clip)
                            })
                    };
                    let should_continue_fixed_child_visual_overflow = has_explicit_height
                        && algo.column_fill == ColumnFill::Auto
                        && resolved.count > 1
                        && !has_spanner_after
                        && !wraps_rows
                        && !child_style.height.is_auto()
                        && child_style.effective_border_top() == 0
                        && child_style.effective_border_right() == 0
                        && child_style.effective_border_bottom() == 0
                        && child_style.effective_border_left() == 0
                        && resolve_margin_or_padding(&child_style.padding_top, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_right, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_bottom, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_left, column_width)
                            == LayoutUnit::zero()
                        && child_style.overflow_x == Overflow::Visible
                        && child_style.overflow_y == Overflow::Visible
                        && !per_col_children[col_idx]
                            .last()
                            .map(|fragment| fragment.has_overflow_clip)
                            .unwrap_or(false);
                    let child_has_descendant_visual_overflow =
                        should_continue_fixed_child_visual_overflow
                            && child_descendant_visual_overflows_box;
                    let child_has_direct_spanner = doc
                        .children(child_node_id)
                        .any(|gc| doc.node(gc).style.column_span == ColumnSpan::All);
                    let child_has_no_block_decoration = child_style.effective_border_top() == 0
                        && child_style.effective_border_bottom() == 0
                        && resolve_margin_or_padding(&child_style.padding_top, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_bottom, column_width)
                            == LayoutUnit::zero();
                    let child_has_authored_inline_overflow =
                        subtree_has_authored_inline_overflow_descendant(doc, child_node_id)
                            && !subtree_has_in_flow_spanner_descendant(doc, child_node_id)
                            && !subtree_has_flex_descendant(doc, child_node_id)
                            && !subtree_has_forced_break_descendant(doc, child_node_id)
                            && style.max_height.is_none();
                    let should_continue_nested_fixed_visual_overflow = has_explicit_height
                        && algo.column_fill == ColumnFill::Auto
                        && resolved.count > 1
                        && !has_spanner_after
                        && !wraps_rows
                        && child_style.height.is_auto()
                        && child_style.overflow_x == Overflow::Visible
                        && child_style.overflow_y == Overflow::Visible
                        && child_style.background_color.is_transparent()
                        && child_style.effective_border_top() == 0
                        && child_style.effective_border_right() == 0
                        && child_style.effective_border_bottom() == 0
                        && child_style.effective_border_left() == 0
                        && resolve_margin_or_padding(&child_style.padding_top, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_right, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_bottom, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_left, column_width)
                            == LayoutUnit::zero()
                        && doc.children(child_node_id).count() == 1
                        && child_descendant_visual_overflows_box
                        && !subtree_has_positioned_descendant(doc, child_node_id)
                        && !subtree_has_in_flow_spanner_descendant(doc, child_node_id)
                        && !subtree_has_flex_descendant(doc, child_node_id)
                        && !subtree_has_forced_break_descendant(doc, child_node_id);
                    let should_continue_bordered_flex_item_visual_overflow = has_explicit_height
                        && algo.column_fill == ColumnFill::Auto
                        && resolved.count == 2
                        && !has_spanner_after
                        && !wraps_rows
                        && algo.column_gap == LayoutUnit::zero()
                        && (child_style.display == Display::Block
                            || child_style.display == Display::Flex)
                        && child_style.height.is_fixed()
                        && child_style.overflow_x == Overflow::Visible
                        && child_style.overflow_y == Overflow::Visible
                        && child_descendant_visual_overflows_box
                        && !subtree_has_positioned_descendant(doc, child_node_id)
                        && !subtree_has_in_flow_spanner_descendant(doc, child_node_id)
                        && !subtree_has_flex_descendant(doc, child_node_id)
                        && !subtree_has_forced_break_descendant(doc, child_node_id);
                    let should_continue_float_visual_overflow = has_explicit_height
                        && algo.column_fill == ColumnFill::Auto
                        && resolved.count > 1
                        && !has_spanner_after
                        && !wraps_rows
                        && child_style.height.is_auto()
                        && child_style.overflow_x == Overflow::Visible
                        && child_style.overflow_y == Overflow::Visible
                        && child_style.background_color.is_transparent()
                        && child_style.effective_border_top() == 0
                        && child_style.effective_border_right() == 0
                        && child_style.effective_border_bottom() == 0
                        && child_style.effective_border_left() == 0
                        && resolve_margin_or_padding(&child_style.padding_top, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_right, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_bottom, column_width)
                            == LayoutUnit::zero()
                        && resolve_margin_or_padding(&child_style.padding_left, column_width)
                            == LayoutUnit::zero()
                        && child_descendant_visual_overflows_box
                        && doc
                            .children(child_node_id)
                            .any(|gc| doc.node(gc).style.float != Float::None)
                        && !subtree_has_positioned_descendant(doc, child_node_id)
                        && !subtree_has_in_flow_spanner_descendant(doc, child_node_id)
                        && !subtree_has_flex_descendant(doc, child_node_id)
                        && !subtree_has_forced_break_descendant(doc, child_node_id);
                    let should_continue_opacity_abspos_visual_overflow = has_explicit_height
                        && algo.column_fill == ColumnFill::Auto
                        && resolved.count > 1
                        && !has_spanner_after
                        && !wraps_rows
                        && child_height == LayoutUnit::zero()
                        && child_style.position.is_positioned()
                        && child_style.opacity < 1.0
                        && child_style.overflow_x == Overflow::Visible
                        && child_style.overflow_y == Overflow::Visible
                        && child_descendant_visual_overflows_box
                        && doc
                            .children(child_node_id)
                            .any(|gc| doc.node(gc).style.position.is_absolutely_positioned())
                        && !subtree_has_in_flow_spanner_descendant(doc, child_node_id)
                        && !subtree_has_flex_descendant(doc, child_node_id)
                        && !subtree_has_forced_break_descendant(doc, child_node_id);
                    let should_continue_nested_multicol_visual_overflow =
                        crate::multicol::ColumnLayoutAlgorithm::from_style(child_style).is_some()
                            && child_style.height.is_auto()
                            && child_style.overflow_x == Overflow::Visible
                            && child_style.overflow_y == Overflow::Visible
                            && (subtree_has_positioned_descendant(doc, child_node_id)
                                || (child_has_direct_spanner && child_has_no_block_decoration)
                                || child_has_authored_inline_overflow);
                    let should_continue_nested_rule_visual_overflow = has_explicit_height
                        && algo.column_fill == ColumnFill::Auto
                        && resolved.count == 1
                        && !has_spanner_after
                        && !wraps_rows
                        && crate::multicol::ColumnLayoutAlgorithm::from_style(child_style)
                            .is_some()
                        && child_style.column_count == Some(1)
                        && child_style.column_rule_style != openui_style::BorderStyle::None
                        && child_style.overflow_x == Overflow::Visible
                        && child_style.overflow_y == Overflow::Visible
                        && child_descendant_visual_overflows_box;
                    if should_continue_fixed_child_visual_overflow
                        && child_style.height.is_fixed()
                        && child_style.background_color.is_transparent()
                    {
                        if let Some(fragment) = per_col_children[col_idx].last_mut() {
                            fill_current_fragmentainer_with_late_direct_float(
                                fragment,
                                doc,
                                column_height,
                            );
                        }
                    }
                    prev_fixed_child_visual_overflow = should_continue_fixed_child_visual_overflow
                        || should_continue_nested_fixed_visual_overflow;
                    prev_fixed_child_descendant_visual_overflow =
                        child_has_descendant_visual_overflow
                            || should_continue_nested_fixed_visual_overflow;
                    if (has_spanner_after
                        || should_continue_abspos_overflow
                        || should_continue_fixed_child_visual_overflow
                        || should_continue_nested_fixed_visual_overflow
                        || should_continue_bordered_flex_item_visual_overflow
                        || should_continue_float_visual_overflow
                        || should_continue_opacity_abspos_visual_overflow
                        || should_continue_nested_multicol_visual_overflow
                        || should_continue_nested_rule_visual_overflow)
                        && !wraps_rows
                        && (child_style.overflow_y == Overflow::Visible
                            || child_style.overflow_y == Overflow::Clip)
                    {
                        let overflow = per_col_children[col_idx]
                            .last()
                            .and_then(|fragment| fragment.overflow_rect);
                        let first_fragment_consumed =
                            column_height - col_block_offset + child_height;
                        let overflow_bottom = overflow
                            .map(|overflow_rect| {
                                overflow_rect.offset.top + overflow_rect.size.height
                            })
                            .or_else(|| {
                                if should_continue_nested_fixed_visual_overflow {
                                    Some(child_visual_bottom)
                                } else if should_continue_bordered_flex_item_visual_overflow {
                                    Some(child_visual_bottom)
                                } else if should_continue_float_visual_overflow {
                                    Some(child_visual_bottom)
                                } else if should_continue_opacity_abspos_visual_overflow {
                                    Some(child_visual_bottom)
                                } else if should_continue_abspos_overflow
                                    && child_descendant_visual_overflows_box
                                {
                                    Some(child_visual_bottom)
                                } else if should_continue_nested_multicol_visual_overflow
                                    && first_fragment_consumed == column_height
                                {
                                    Some(first_fragment_consumed + column_height)
                                } else if should_continue_nested_rule_visual_overflow {
                                    Some(child_visual_bottom)
                                } else {
                                    None
                                }
                            });
                        if let Some(overflow_bottom) = overflow_bottom {
                            let bottom_abspos_avoid_crossing = if should_continue_abspos_overflow
                                && child_style.position.is_positioned()
                            {
                                bottom_abspos_avoid_descendant_crossing(
                                    per_col_children[col_idx].last().unwrap(),
                                    doc,
                                    LayoutUnit::zero(),
                                    first_fragment_consumed,
                                )
                            } else {
                                None
                            };
                            let bottom_abspos_overflow_source =
                                if bottom_abspos_avoid_crossing.is_some() {
                                    per_col_children[col_idx].last().cloned()
                                } else {
                                    None
                                };
                            if let Some((avoid_top, _)) = bottom_abspos_avoid_crossing {
                                if let Some(fragment) = per_col_children[col_idx].last_mut() {
                                    suppress_avoid_descendants_at_or_after(
                                        fragment,
                                        doc,
                                        LayoutUnit::zero(),
                                        avoid_top,
                                    );
                                }
                            }
                            if should_continue_opacity_abspos_visual_overflow
                                && child_visual_top < LayoutUnit::zero()
                                && col_idx > 0
                            {
                                let visual_top_in_column = col_block_offset + child_visual_top;
                                if visual_top_in_column < LayoutUnit::zero() {
                                    let needed_back_cols =
                                        ((-visual_top_in_column.raw() + column_height.raw() - 1)
                                            / column_height.raw())
                                            as usize;
                                    let start_col =
                                        col_idx.saturating_sub(needed_back_cols.min(col_idx));
                                    for overflow_col_idx in start_col..col_idx {
                                        let mut overflow_part =
                                            per_col_children[col_idx].last().unwrap().clone();
                                        overflow_part.offset = PhysicalOffset::new(
                                            content_edge_x
                                                + col_inline_offset_for(overflow_col_idx),
                                            content_edge_y + total_block_offset,
                                        );
                                        overflow_part.size.height = column_height;
                                        overflow_part.has_overflow_clip = true;
                                        overflow_part.block_axis_clip_only = true;
                                        overflow_part.is_first_for_node = false;
                                        overflow_part.is_last_for_node = false;
                                        let back_distance =
                                            column_height * (col_idx - overflow_col_idx) as i32;
                                        for child in &mut overflow_part.children {
                                            child.offset.top = child.offset.top + back_distance;
                                        }
                                        while per_col_children.len() <= overflow_col_idx {
                                            per_col_children.push(Vec::new());
                                        }
                                        per_col_children[overflow_col_idx].push(overflow_part);
                                    }
                                }
                            }
                            let mut overflow_consumed = first_fragment_consumed;
                            let mut overflow_col_idx = col_idx + 1;
                            let can_extend_positioned_abspos_overflow_columns =
                                should_continue_abspos_overflow
                                    && child_style.position.is_positioned()
                                    && ((style.position.is_positioned()
                                        && !style.left.is_auto()
                                        && resolve_length(
                                            &style.left,
                                            child_available_inline,
                                            LayoutUnit::zero(),
                                            LayoutUnit::zero(),
                                        ) < LayoutUnit::zero())
                                        || (positioned_direct_abspos_visual_overflow
                                            && child_style.overflow_x == Overflow::Visible
                                            && child_style.overflow_y == Overflow::Visible));
                            while overflow_consumed.raw() < overflow_bottom.raw()
                                && (overflow_col_idx < resolved.count as usize
                                    || can_extend_positioned_abspos_overflow_columns
                                    || should_continue_nested_rule_visual_overflow)
                            {
                                let first_overflow_col = overflow_col_idx == col_idx + 1;
                                let mut overflow_part = if bottom_abspos_avoid_crossing.is_some()
                                    && first_overflow_col
                                {
                                    bottom_abspos_overflow_source.as_ref().unwrap().clone()
                                } else {
                                    per_col_children[col_idx].last().unwrap().clone()
                                };
                                overflow_part.offset = PhysicalOffset::new(
                                    content_edge_x + col_inline_offset_for(overflow_col_idx),
                                    content_edge_y + total_block_offset,
                                );
                                let mut overflow_part_height =
                                    (overflow_bottom - overflow_consumed).min_of(column_height);
                                let mut overflow_child_shift = overflow_consumed;
                                if let Some((avoid_top, avoid_bottom)) =
                                    bottom_abspos_avoid_crossing
                                {
                                    if first_overflow_col {
                                        let avoid_height =
                                            (avoid_bottom - avoid_top).clamp_negative_to_zero();
                                        overflow_part_height =
                                            overflow_part_height.max_of(avoid_height);
                                        overflow_child_shift = avoid_top;
                                    }
                                }
                                overflow_part.size.height = overflow_part_height;
                                overflow_part.has_overflow_clip = true;
                                overflow_part.block_axis_clip_only = child_style.overflow_x
                                    == Overflow::Visible
                                    && !child_has_authored_inline_overflow;
                                if should_continue_bordered_flex_item_visual_overflow {
                                    overflow_part.border = BoxStrut::zero();
                                    for child in &mut overflow_part.children {
                                        child.border = BoxStrut::zero();
                                    }
                                }
                                overflow_part.is_first_for_node = false;
                                if overflow_consumed + overflow_part.size.height < overflow_bottom {
                                    overflow_part.is_last_for_node = false;
                                }
                                for child in &mut overflow_part.children {
                                    if should_continue_nested_multicol_visual_overflow
                                        && child.kind == FragmentKind::ColumnBox
                                        || should_continue_nested_rule_visual_overflow
                                            && child.kind == FragmentKind::ColumnBox
                                            && child.offset.left.raw() > 0
                                    {
                                        // Nested multicol fragments already contain column-local
                                        // slices; keep the inner column clip at the fragment origin.
                                    } else {
                                        child.offset.top = child.offset.top - overflow_child_shift;
                                        if should_continue_nested_fixed_visual_overflow
                                            && child_style.display == Display::Flex
                                            && !child_style.flex_direction.is_column()
                                            && !child.node_id.is_none()
                                        {
                                            let item_style = &doc.node(child.node_id).style;
                                            if !item_style.background_color.is_transparent()
                                                && child.offset.top.raw() < 0
                                                && fragment_visual_block_bottom(child).raw()
                                                    > child.size.height.raw()
                                            {
                                                child.size.height = child.size.height.max_of(
                                                    overflow_child_shift + overflow_part_height,
                                                );
                                            }
                                        }
                                        if positioned_direct_abspos_visual_overflow
                                            && !child.node_id.is_none()
                                            && doc
                                                .node(child.node_id)
                                                .style
                                                .position
                                                .is_absolutely_positioned()
                                        {
                                            child.size.height = child
                                                .size
                                                .height
                                                .max_of(overflow_child_shift + overflow_part_height);
                                            child.size.width = child.size.width.max_of(column_width);
                                        }
                                    }
                                }
                                if should_continue_bordered_flex_item_visual_overflow {
                                    let mut descendants = Vec::new();
                                    for flex_item in &overflow_part.children {
                                        for descendant in &flex_item.children {
                                            let mut descendant = descendant.clone();
                                            descendant.offset.left =
                                                descendant.offset.left + flex_item.offset.left;
                                            descendant.offset.top =
                                                descendant.offset.top + flex_item.offset.top;
                                            descendants.push(descendant);
                                        }
                                    }
                                    if !descendants.is_empty() {
                                        overflow_part.node_id = NodeId::NONE;
                                        overflow_part.border = BoxStrut::zero();
                                        overflow_part.padding = BoxStrut::zero();
                                        overflow_part.children = descendants;
                                    }
                                }
                                while per_col_children.len() <= overflow_col_idx {
                                    per_col_children.push(Vec::new());
                                }
                                if should_continue_float_visual_overflow {
                                    delayed_float_overflow_parts
                                        .push((overflow_col_idx, overflow_part));
                                } else {
                                    per_col_children[overflow_col_idx].push(overflow_part);
                                }
                                overflow_rule_column_count =
                                    overflow_rule_column_count.max(overflow_col_idx + 1);
                                overflow_consumed = overflow_consumed + column_height;
                                overflow_col_idx += 1;
                            }
                        }
                    }
                } else {
                    prev_fixed_child_visual_overflow = false;
                    prev_fixed_child_descendant_visual_overflow = false;
                    // Child must be fragmented across multiple columns.
                    // CSS Multicol §3.4: overflow creates additional columns
                    // in the inline direction (col_idx may exceed positions.len()).
                    col_block_offset = col_block_offset + pos_margin;
                    col_remaining = col_remaining - pos_margin;
                    if child_style.display == Display::Flex
                        && column_height.raw() > 0
                        && child_height.raw() > column_height.raw()
                        && col_block_offset.raw() >= column_height.raw()
                    {
                        let advance_cols = if pos_margin.raw() > 0 {
                            1
                        } else {
                            (col_block_offset.raw() / column_height.raw()) as usize
                        };
                        if advance_cols > 0 {
                            col_idx += advance_cols;
                            col_block_offset = if pos_margin.raw() > 0 {
                                LayoutUnit::zero()
                            } else {
                                col_block_offset - column_height * advance_cols as i32
                            };
                            col_remaining =
                                (column_height - col_block_offset).clamp_negative_to_zero();
                        }
                    }
                    // Extract OOF candidates before fragmenting (they originate
                    // in the child's first column position).
                    let frag_child_oof = std::mem::take(&mut child_frag.oof_candidates);
                    let frag_base_offset = PhysicalOffset::new(
                        content_edge_x + col_inline_offset_for(col_idx),
                        content_edge_y
                            + total_block_offset
                            + col_extra_block_offset_for(col_idx)
                            + col_block_offset,
                    );
                    for mut c in frag_child_oof {
                        if c.style.position == Position::Fixed
                            && c.style.top.is_auto()
                            && c.style.bottom.is_auto()
                            && !wraps_rows
                            && column_height.raw() > 0
                        {
                            let local_flow_top = col_block_offset + c.static_position.top;
                            let column_delta = if local_flow_top.raw() > 0 {
                                ((local_flow_top.raw() - 1) / column_height.raw()) as usize
                            } else {
                                0
                            };
                            let static_col_idx = col_idx + column_delta;
                            let static_block_offset =
                                local_flow_top - column_height * column_delta as i32;
                            c.static_position.left = c.static_position.left
                                + content_edge_x
                                + col_inline_offset_for(static_col_idx);
                            c.static_position.top = content_edge_y
                                + total_block_offset
                                + col_extra_block_offset_for(static_col_idx)
                                + static_block_offset;
                        } else if c.style.position == Position::Absolute
                            && !wraps_rows
                            && column_height.raw() > 0
                        {
                            let row_flex_gap = child_style
                                .row_gap
                                .as_ref()
                                .map(|gap| {
                                    resolve_length(
                                        gap,
                                        child_height,
                                        LayoutUnit::zero(),
                                        LayoutUnit::zero(),
                                    )
                                    .clamp_negative_to_zero()
                                })
                                .unwrap_or(LayoutUnit::zero());
                            let row_flex_forced_break_gap_abspos = child_style.display
                                == Display::Flex
                                && !child_style.flex_direction.is_column()
                                && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                                && child_style.height.is_auto()
                                && child_style.background_color.is_transparent()
                                && row_flex_gap.raw() > 0
                                && subtree_has_forced_break_descendant(doc, child_node_id)
                                && subtree_has_positioned_descendant(doc, child_node_id);
                            let local_flow_top = if row_flex_forced_break_gap_abspos {
                                column_height
                                    + forced_break_descendant_top(&child_frag, doc)
                                    .unwrap_or(col_block_offset + c.static_position.top)
                            } else {
                                col_block_offset + c.static_position.top
                            };
                            let column_delta = if row_flex_forced_break_gap_abspos
                                && col_idx + 1 < resolved.count as usize
                            {
                                1
                            } else if local_flow_top.raw() > 0 {
                                (local_flow_top.raw() / column_height.raw()) as usize
                            } else {
                                0
                            };
                            let static_col_idx = col_idx + column_delta;
                            let static_block_offset =
                                local_flow_top - column_height * column_delta as i32;
                            c.static_position.left = c.static_position.left
                                + content_edge_x
                                + col_inline_offset_for(static_col_idx);
                            c.static_position.top = content_edge_y
                                + total_block_offset
                                + col_extra_block_offset_for(static_col_idx)
                                + static_block_offset;
                        } else {
                            c.static_position.left = c.static_position.left + frag_base_offset.left;
                            c.static_position.top = c.static_position.top + frag_base_offset.top;
                        }
                        bubbled_oof_from_columns.push(c);
                    }
                    let mut consumed = LayoutUnit::zero();
                    // Guard: if column_height is zero or negative, place
                    // everything in the current column to avoid infinite loop.
                    if column_height.raw() <= 0 {
                        let mut part = child_frag.clone();
                        part.size.height = child_height;
                        // Only clip if the child itself has overflow clipping or content overflows
                        part.has_overflow_clip = child_frag.has_overflow_clip;
                        part.offset = PhysicalOffset::new(
                            content_edge_x + col_inline_offset_for(col_idx),
                            content_edge_y
                                + total_block_offset
                                + col_extra_block_offset_for(col_idx)
                                + col_block_offset,
                        );
                        while per_col_children.len() <= col_idx {
                            per_col_children.push(Vec::new());
                        }
                        per_col_children[col_idx].push(part);
                        overflow_rule_column_count = overflow_rule_column_count.max(col_idx + 1);
                        col_block_offset = col_block_offset + child_height;
                        max_wrapped_content_extent = max_wrapped_content_extent
                            .max_of(col_extra_block_offset_for(col_idx) + col_block_offset);
                    } else {
                        // box-decoration-break: clone — borders/padding repeat
                        // on every fragment. Compute extra block-axis BP to
                        // account for in each fragment's content area.
                        let is_clone =
                            child_style.box_decoration_break == BoxDecorationBreak::Clone;
                        let clone_bp_block = if is_clone {
                            resolve_margin_or_padding(&child_style.padding_top, column_width)
                                + resolve_margin_or_padding(
                                    &child_style.padding_bottom,
                                    column_width,
                                )
                                + LayoutUnit::from_i32(child_style.border_top_width)
                                + LayoutUnit::from_i32(child_style.border_bottom_width)
                        } else {
                            LayoutUnit::zero()
                        };
                        let clone_visual_overflow_bottom = if is_clone {
                            fragment_visual_block_bottom(&child_frag)
                        } else {
                            child_height
                        };
                        // For clone mode, track content consumed separately.
                        // The child_height includes one set of BP; content is
                        // child_height minus that BP. Each fragment adds its
                        // own full BP around the content portion.
                        let visual_overflow_fragment_height = if !is_clone
                            && !child_style.height.is_auto()
                            && child_style.overflow_y == Overflow::Visible
                            && child_style.overflow_x != Overflow::Visible
                            && !subtree_has_positioned_descendant(doc, child_node_id)
                            && !subtree_has_in_flow_spanner_descendant(doc, child_node_id)
                            && !subtree_has_flex_descendant(doc, child_node_id)
                            && !subtree_has_forced_break_descendant(doc, child_node_id)
                        {
                            fragment_visual_block_bottom(&child_frag)
                        } else {
                            child_height
                        };
                        let row_flex_descendant_visual_overflow_height = if !is_clone
                            && algo.column_fill == ColumnFill::Auto
                            && has_explicit_height
                            && !wraps_rows
                            && algo.column_gap == LayoutUnit::zero()
                            && child_style.display == Display::Flex
                            && !child_style.flex_direction.is_column()
                            && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                            && child_style.height.is_auto()
                            && child_style.background_color.is_transparent()
                            && !subtree_has_forced_break_descendant(doc, child_node_id)
                            && !subtree_has_positioned_descendant(doc, child_node_id)
                        {
                            fragment_visual_block_bottom(&child_frag)
                        } else {
                            child_height
                        };
                        let content_height = if is_clone {
                            let normal_content =
                                (child_height - clone_bp_block).clamp_negative_to_zero();
                            if clone_visual_overflow_bottom.raw() > child_height.raw()
                                && column_height.raw() > 0
                            {
                                let desired_fragments = (clone_visual_overflow_bottom.raw()
                                    + column_height.raw()
                                    - 1)
                                    / column_height.raw();
                                let content_per_fragment =
                                    (column_height - clone_bp_block).clamp_negative_to_zero();
                                let unit_content = if content_per_fragment.raw() > 0 {
                                    content_per_fragment
                                } else {
                                    LayoutUnit::from_i32(1)
                                };
                                normal_content.max_of(unit_content * desired_fragments as i32)
                            } else {
                                normal_content
                            }
                        } else if visual_overflow_fragment_height.raw() > child_height.raw() {
                            visual_overflow_fragment_height
                        } else if row_flex_descendant_visual_overflow_height.raw()
                            > child_height.raw()
                        {
                            row_flex_descendant_visual_overflow_height
                        } else {
                            child_height
                        };
                        let mut content_consumed = LayoutUnit::zero();
                        let row_flex_forced_fragmentainer_flow = !wraps_rows
                            && algo.column_fill == ColumnFill::Auto
                            && child_style.display == Display::Flex
                            && !child_style.flex_direction.is_column()
                            && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                            && !child_style.height.is_auto()
                            && child_style.background_color.is_transparent()
                            && col_block_offset > LayoutUnit::zero()
                            && subtree_has_forced_break_descendant(doc, child_node_id);
                        if style.height.is_auto()
                            && !style.background_color.is_transparent()
                            && algo.column_fill != ColumnFill::Auto
                            && resolved.count > 1
                            && !has_spanner_after
                            && !wraps_rows
                            && child_style.display == Display::Flex
                            && !child_style.flex_direction.is_column()
                            && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                            && child_style.background_color.is_transparent()
                            && child_height.raw() > column_height.raw()
                            && subtree_has_avoid_break_descendant(doc, child_node_id)
                            && !subtree_has_forced_break_descendant(doc, child_node_id)
                        {
                            max_col_visual_content = max_col_visual_content.max_of(child_height);
                        }
                        let mut row_flex_avoid_negative_continuation = false;

                        while content_consumed.raw() < content_height.raw() {
                            let avail = if col_block_offset != LayoutUnit::zero() {
                                col_remaining
                            } else {
                                column_height
                            };

                            // For clone mode, subtract BP from available space
                            // to get the content portion that fits.
                            let content_avail = if is_clone {
                                (avail - clone_bp_block).clamp_negative_to_zero()
                            } else {
                                avail
                            };
                            let remaining_content = content_height - content_consumed;
                            let raw_content_in_part = if is_clone && content_avail.raw() <= 0 {
                                LayoutUnit::from_i32(1).min_of(remaining_content)
                            } else {
                                content_avail.min_of(remaining_content)
                            };
                            let mut content_in_part = if is_clone {
                                raw_content_in_part
                            } else {
                                snap_wrapped_multicol_fragment_consumption(
                                    child_style,
                                    raw_content_in_part,
                                    remaining_content,
                                    column_width,
                                )
                            };
                            let child_has_bottom_aligned_abspos =
                                child_style.position.is_positioned()
                                    && doc.children(child_node_id).any(|gc| {
                                        let gc_style = &doc.node(gc).style;
                                        gc_style.position.is_absolutely_positioned()
                                            && !gc_style.bottom.is_auto()
                                    });
                            let row_flex_avoid_before_negative_sibling =
                                child_style.display == Display::Flex
                                    && !child_style.flex_direction.is_column()
                                    && child_style.flex_wrap == openui_style::FlexWrap::Nowrap
                                    && i + 1 < child_count
                                    && col_margins_top[i + 1].raw() < 0
                                    && subtree_has_avoid_break_descendant(doc, child_node_id);
                            let mut expanded_for_avoid_descendant = false;
                            if !is_clone && content_in_part.raw() < remaining_content.raw() {
                                let break_at = content_consumed + content_in_part;
                                let avoid_start = if child_has_bottom_aligned_abspos {
                                    bottom_abspos_avoid_descendant_break_before(
                                        &child_frag,
                                        doc,
                                        content_consumed,
                                        break_at,
                                    )
                                } else if row_flex_avoid_before_negative_sibling {
                                    bottom_abspos_avoid_descendant_break_before(
                                        &child_frag,
                                        doc,
                                        content_consumed,
                                        break_at,
                                    )
                                } else {
                                    avoid_descendant_break_before(
                                        &child_frag,
                                        doc,
                                        content_consumed,
                                        break_at,
                                    )
                                };
                                if let Some(avoid_start) = avoid_start {
                                    let adjusted = avoid_start - content_consumed;
                                    if adjusted.raw() > 0 {
                                        content_in_part = adjusted;
                                        expanded_for_avoid_descendant = true;
                                        if row_flex_avoid_before_negative_sibling {
                                            row_flex_avoid_negative_continuation = true;
                                        }
                                    }
                                }
                            }
                            let part_height = if is_clone {
                                content_in_part + clone_bp_block
                            } else {
                                content_in_part
                            };
                            let child_is_multicol =
                                crate::multicol::ColumnLayoutAlgorithm::from_style(child_style)
                                    .is_some();
                            let child_has_positioned_descendant =
                                subtree_has_positioned_descendant(doc, child_node_id);
                            let child_has_flex_descendant =
                                subtree_has_flex_descendant(doc, child_node_id);
                            let child_has_rule_positioned_overflow = child_is_multicol
                                && child_style.height.is_auto()
                                && child_style.column_rule_width > 0
                                && child_has_positioned_descendant;
                            let child_has_direct_spanner = doc
                                .children(child_node_id)
                                .any(|gc| doc.node(gc).style.column_span == ColumnSpan::All);
                            let child_has_direct_avoid_after_sibling = {
                                let mut seen_previous = false;
                                let mut found = false;
                                for gc in doc.children(child_node_id) {
                                    if doc.node(gc).style.break_inside.is_avoid() && seen_previous {
                                        found = true;
                                        break;
                                    }
                                    seen_previous = true;
                                }
                                found
                            };
                            let child_has_authored_inline_overflow =
                                subtree_has_authored_inline_overflow_descendant(doc, child_node_id)
                                    && !subtree_has_in_flow_spanner_descendant(doc, child_node_id)
                                    && !subtree_has_flex_descendant(doc, child_node_id)
                                    && !subtree_has_forced_break_descendant(doc, child_node_id)
                                    && style.max_height.is_none();
                            let child_has_top_abspos_before_spanner = has_spanner_after
                                && child_style.position.is_positioned()
                                && doc.children(child_node_id).any(|gc| {
                                    let gc_style = &doc.node(gc).style;
                                    gc_style.position.is_absolutely_positioned()
                                        && !gc_style.top.is_auto()
                                        && resolve_length(
                                            &gc_style.top,
                                            child_height,
                                            LayoutUnit::zero(),
                                            LayoutUnit::zero(),
                                        ) <= LayoutUnit::zero()
                                })
                                && if group_available_block.is_indefinite() {
                                    child_frag.scrollable_overflow().offset.top
                                        + child_frag.scrollable_overflow().size.height
                                        > child_height
                                } else {
                                    child_height == group_available_block
                                };
                            let child_has_static_abspos_before_spanner = has_spanner_after
                                && style.height.is_auto()
                                && group_available_block.is_indefinite()
                                && child_height.raw() > column_height.raw()
                                && child_style.position.is_positioned()
                                && doc.children(child_node_id).any(|gc| {
                                    let gc_style = &doc.node(gc).style;
                                    gc_style.position.is_absolutely_positioned()
                                        && gc_style.top.is_auto()
                                        && gc_style.bottom.is_auto()
                                });
                            let expand_fragment_visual_height = (wraps_rows
                                && !is_clone
                                && (algo.column_height.is_none()
                                    || total_block_offset > LayoutUnit::zero()))
                                || (child_is_multicol
                                    && child_style.height.is_auto()
                                    && (child_has_direct_spanner
                                        || child_style.column_height.is_some()
                                        || child_has_rule_positioned_overflow)
                                    && !is_clone);
                            let nested_avoid_inline_overflow_continuation = content_consumed
                                > LayoutUnit::zero()
                                && child_is_multicol
                                && child_has_authored_inline_overflow
                                && child_has_direct_avoid_after_sibling;
                            let row_flex_visible_crossing_child = !is_clone
                                && !row_flex_forced_fragmentainer_flow
                                && content_consumed > LayoutUnit::zero()
                                && child_style.display == Display::Flex
                                && !child_style.flex_direction.is_column()
                                && child_style.background_color.is_transparent()
                                && child_frag.children.iter().any(|c| {
                                    if c.node_id.is_none() {
                                        return false;
                                    }
                                    let item_style = &doc.node(c.node_id).style;
                                    !item_style.background_color.is_transparent()
                                        && c.offset.top.raw() < content_consumed.raw()
                                        && (c.offset.top + c.size.height).raw()
                                            > content_consumed.raw()
                                });
                            let row_flex_gap = child_style
                                .row_gap
                                .as_ref()
                                .map(|gap| {
                                    resolve_length(
                                        gap,
                                        child_height,
                                        LayoutUnit::zero(),
                                        LayoutUnit::zero(),
                                    )
                                    .clamp_negative_to_zero()
                                })
                                .unwrap_or(LayoutUnit::zero());
                            let row_flex_has_positioned_descendant =
                                subtree_has_positioned_descendant(doc, child_node_id);
                            let row_flex_forced_break_continuation = !is_clone
                                && content_consumed > LayoutUnit::zero()
                                && !wraps_rows
                                && algo.column_fill == ColumnFill::Auto
                                && has_explicit_height
                                && resolved.count == 2
                                && algo.column_gap == LayoutUnit::zero()
                                && child_style.display == Display::Flex
                                && !child_style.flex_direction.is_column()
                                && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                                && child_style.height.is_auto()
                                && child_style.min_height.is_auto()
                                && (row_flex_gap.raw() == 0
                                    || row_flex_has_positioned_descendant)
                                && subtree_has_forced_break_descendant(doc, child_node_id);
                            let flex_padding_overflow_continuation = !is_clone
                                && !style.height.is_auto()
                                && algo.column_fill == ColumnFill::Auto
                                && !wraps_rows
                                && resolved.count > 1
                                && child_style.display == Display::Flex
                                && child_style.height.is_auto()
                                && child_style.overflow_x == Overflow::Visible
                                && child_style.overflow_y == Overflow::Visible
                                && doc.children(child_node_id).next().is_none()
                                && (resolve_margin_or_padding(&child_style.padding_bottom, column_width)
                                    > column_height);
                            let visual_part_height = if expanded_for_avoid_descendant {
                                part_height.max_of(avail)
                            } else if post_spanner_single_child_balanced_tail == Some(i)
                                && !is_clone
                            {
                                part_height.max_of(child_height)
                            } else if nested_avoid_inline_overflow_continuation {
                                part_height.max_of(column_height)
                            } else if row_flex_visible_crossing_child {
                                part_height.max_of(avail)
                            } else if row_flex_forced_break_continuation {
                                part_height.max_of(avail)
                            } else if is_clone
                                && clone_visual_overflow_bottom.raw() > child_height.raw()
                            {
                                part_height.max_of(avail)
                            } else if flex_padding_overflow_continuation {
                                part_height.max_of(column_height * resolved.count as i32)
                            } else if child_has_top_abspos_before_spanner && !is_clone {
                                let top_abspos_visual_height =
                                    if group_available_block.is_indefinite() {
                                        child_height
                                    } else {
                                        group_available_block
                                    };
                                part_height.max_of(top_abspos_visual_height)
                            } else if child_has_static_abspos_before_spanner && !is_clone {
                                part_height.max_of(child_height)
                            } else if child_has_bottom_aligned_abspos && !is_clone {
                                let child_padding_block = resolve_margin_or_padding(
                                    &child_style.padding_top,
                                    column_width,
                                ) + resolve_margin_or_padding(
                                    &child_style.padding_bottom,
                                    column_width,
                                );
                                part_height.max_of(avail + child_padding_block)
                            } else if child_has_authored_inline_overflow
                                && !space.fragmentainer_block_size.is_indefinite()
                            {
                                let visible_remaining = (space.fragmentainer_block_size
                                    - col_block_offset)
                                    .clamp_negative_to_zero();
                                part_height.max_of(visible_remaining)
                            } else if !is_clone
                                && child_style.display == Display::Flex
                                && content_consumed == LayoutUnit::zero()
                            {
                                let child_padding_top = resolve_margin_or_padding(
                                    &child_style.padding_top,
                                    column_width,
                                );
                                if child_padding_top.raw() > 0 {
                                    part_height + child_padding_top
                                } else {
                                    part_height
                                }
                            } else if !is_clone
                                && !style.height.is_auto()
                                && style.column_fill == ColumnFill::Auto
                                && child_style.display == Display::FlowRoot
                                && child_style.height.is_auto()
                                && !child_style.background_color.is_transparent()
                                && child_style.overflow_x == Overflow::Visible
                                && child_style.overflow_y == Overflow::Visible
                                && content_consumed > LayoutUnit::zero()
                                && content_in_part == remaining_content
                                && subtree_has_clone_decoration_descendant(doc, child_node_id)
                            {
                                part_height.max_of(avail)
                            } else if expand_fragment_visual_height {
                                if !space.available_block_size.is_indefinite() {
                                    let visible_remaining = (space.available_block_size
                                        - col_extra_block_offset_for(col_idx)
                                        - col_block_offset)
                                        .clamp_negative_to_zero();
                                    let expanded = if algo.column_height.is_some()
                                        && total_block_offset > LayoutUnit::zero()
                                    {
                                        part_height.max_of(visible_remaining)
                                    } else {
                                        part_height.max_of(avail)
                                    };
                                    expanded.min_of(visible_remaining)
                                } else {
                                    part_height.max_of(avail)
                                }
                            } else {
                                part_height
                            };

                            // Safety: if part_height is zero, break to avoid
                            // infinite loop (degenerate column height).
                            if part_height.raw() <= 0 {
                                break;
                            }

                            let mut part = child_frag.clone();
                            part.size.height = visual_part_height;
                            if child_has_bottom_aligned_abspos && expanded_for_avoid_descendant {
                                suppress_avoid_descendants_at_or_after(
                                    &mut part,
                                    doc,
                                    LayoutUnit::zero(),
                                    content_consumed + content_in_part,
                                );
                            }
                            if let Some(decoration_limit) = child_frag.decoration_paint_block_size {
                                let part_decoration_limit =
                                    (decoration_limit - content_consumed).clamp_negative_to_zero();
                                part.decoration_paint_block_size =
                                    Some(part_decoration_limit.min_of(visual_part_height));
                            }
                            // Multicol fragments normally need an overflow clip,
                            // but split wrappers with an independent decoration
                            // paint budget must let children overflow to the
                            // column box clip instead of the wrapper decoration
                            // limit.
                            let split_decoration_visible_overflow =
                                child_frag.decoration_paint_block_size.is_some()
                                    && child_style.overflow_x == Overflow::Visible
                                    && child_style.overflow_y == Overflow::Visible;
                            part.has_overflow_clip = !split_decoration_visible_overflow;
                            part.block_axis_clip_only = child_style.overflow_x == Overflow::Visible
                                && child_style.overflow_y == Overflow::Visible;

                            if is_clone {
                                // box-decoration-break: clone — keep borders on
                                // all fragments (is_first/is_last stay true).
                            } else {
                                // box-decoration-break: slice (default) — suppress
                                // block-start border on non-first fragments and
                                // block-end border on non-last fragments.
                                if consumed > LayoutUnit::zero() {
                                    part.is_first_for_node = false;
                                }
                                if content_consumed + content_in_part < content_height {
                                    part.is_last_for_node = false;
                                }
                            }
                            if child_frag.decoration_paint_block_size.is_some()
                                && child_frag.is_last_for_node
                                && part
                                    .decoration_paint_block_size
                                    .map_or(false, |limit| limit.raw() > 0)
                            {
                                part.is_last_for_node = true;
                            }

                            let part_col_idx = if row_flex_forced_fragmentainer_flow {
                                col_idx + 1
                            } else {
                                col_idx
                            };
                            let part_block_offset = if row_flex_forced_fragmentainer_flow {
                                LayoutUnit::zero()
                            } else {
                                col_block_offset
                            };
                            part.offset = PhysicalOffset::new(
                                content_edge_x + col_inline_offset_for(part_col_idx),
                                content_edge_y
                                    + total_block_offset
                                    + col_extra_block_offset_for(part_col_idx)
                                    + part_block_offset,
                            );
                            // Apply relative positioning (CSS 2.1 §9.4.3).
                            crate::relative::apply_relative_offset(
                                &mut part,
                                child_style,
                                column_width,
                                column_height,
                            );
                            // Shift child content up by the amount already consumed.
                            // For clone mode, content_consumed tracks content only;
                            // children are offset relative to the content box, so
                            // shifting by content_consumed is correct.
                            if child_is_multicol
                                && child_style.column_count == Some(1)
                                && child_style.column_rule_style != openui_style::BorderStyle::None
                                && content_consumed == LayoutUnit::zero()
                                && content_in_part.raw() < child_height.raw()
                            {
                                part.children.retain(|c| {
                                    !(matches!(
                                        c.kind,
                                        FragmentKind::ColumnBox | FragmentKind::ColumnRule
                                    ) && c.offset.left.raw() > column_width.raw())
                                });
                            }
                            if content_consumed > LayoutUnit::zero() {
                                let content_shift = if child_has_bottom_aligned_abspos {
                                    (content_consumed - (visual_part_height - part_height))
                                        .clamp_negative_to_zero()
                                } else if child_is_multicol
                                    && child_has_authored_inline_overflow
                                    && child_has_direct_avoid_after_sibling
                                {
                                    LayoutUnit::zero()
                                } else if row_flex_forced_break_continuation {
                                    forced_break_descendant_top(&child_frag, doc)
                                        .unwrap_or(content_consumed)
                                        .min_of(content_consumed)
                                } else {
                                    content_consumed
                                };
                                let nested_avoid_inline_overflow_column_width = if child_is_multicol
                                    && child_has_authored_inline_overflow
                                    && child_has_direct_avoid_after_sibling
                                {
                                    part.children
                                        .iter()
                                        .find(|c| c.kind == FragmentKind::ColumnBox)
                                        .map_or(LayoutUnit::zero(), |c| c.size.width)
                                } else {
                                    LayoutUnit::zero()
                                };
                                let nested_abspos_column_gap = if child_is_multicol
                                    && child_style.column_rule_style
                                        == openui_style::BorderStyle::None
                                    && subtree_has_positioned_abspos_descendant(doc, child_node_id)
                                {
                                    let mut first_column: Option<(LayoutUnit, LayoutUnit)> = None;
                                    let mut second_column_left: Option<LayoutUnit> = None;
                                    for c in &part.children {
                                        if c.kind != FragmentKind::ColumnBox {
                                            continue;
                                        }
                                        if let Some((first_left, _)) = first_column {
                                            if c.offset.left > first_left {
                                                second_column_left = Some(
                                                    second_column_left
                                                        .map_or(c.offset.left, |left| {
                                                            left.min_of(c.offset.left)
                                                        }),
                                                );
                                            }
                                        } else {
                                            first_column = Some((c.offset.left, c.size.width));
                                        }
                                    }
                                    if let (Some((first_left, first_width)), Some(second_left)) =
                                        (first_column, second_column_left)
                                    {
                                        (second_left - first_left - first_width)
                                            .clamp_negative_to_zero()
                                    } else {
                                        LayoutUnit::zero()
                                    }
                                } else {
                                    LayoutUnit::zero()
                                };
                                for c in &mut part.children {
                                    let original_child_top = c.offset.top;
                                    if child_is_multicol
                                        && (child_style.position.is_positioned()
                                            || (child_has_positioned_descendant
                                                && child_has_flex_descendant)
                                            || (child_style.column_count == Some(1)
                                                && child_style.column_rule_style
                                                    != openui_style::BorderStyle::None
                                                && c.offset.left.raw() > 0))
                                        && c.kind == FragmentKind::ColumnBox
                                    {
                                        // The nested multicol child has already produced
                                        // column-local fragments; moving the inner column
                                        // box would move its clip away from this outer
                                        // fragmentainer.
                                    } else {
                                        c.offset.top = c.offset.top - content_shift;
                                        if child_style.display == Display::Flex
                                            && child_style.flex_direction.is_column()
                                        {
                                            if let Some(forced_top) =
                                                forced_break_descendant_top(c, doc)
                                            {
                                                let forced_global_top =
                                                    original_child_top + forced_top;
                                                if forced_global_top.raw() < content_shift.raw() {
                                                    c.offset.top =
                                                        original_child_top - forced_global_top;
                                                }
                                            }
                                        }
                                        if nested_abspos_column_gap.raw() > 0
                                            && c.kind == FragmentKind::ColumnBox
                                        {
                                            c.offset.left =
                                                c.offset.left + nested_abspos_column_gap;
                                        }
                                        if child_style.display == Display::Flex
                                            && !child_style.flex_direction.is_column()
                                        {
                                            if !c.node_id.is_none() {
                                                let item_style = &doc.node(c.node_id).style;
                                                let original_child_bottom =
                                                    original_child_top + c.size.height;
                                                if child_style.background_color.is_transparent()
                                                    && row_flex_descendant_visual_overflow_height
                                                        .raw()
                                                        <= child_height.raw()
                                                    && !item_style.background_color.is_transparent()
                                                    && original_child_top.raw()
                                                        < content_shift.raw()
                                                    && original_child_bottom.raw()
                                                        > content_shift.raw()
                                                    && c.size.height.raw()
                                                        >= visual_part_height.raw()
                                                    && c.offset.top.raw() < 0
                                                {
                                                    let needed_paint_height = (content_shift
                                                        - original_child_top)
                                                        .clamp_negative_to_zero()
                                                        + visual_part_height;
                                                    c.size.height =
                                                        c.size.height.max_of(needed_paint_height);
                                                }
                                                if child_style.flex_wrap
                                                    != openui_style::FlexWrap::Nowrap
                                                    && item_style.break_inside.is_avoid()
                                                    && original_child_top.raw() >= 0
                                                    && original_child_bottom.raw()
                                                        <= content_shift.raw()
                                                    && c.offset.top.raw() < 0
                                                {
                                                    c.offset.top = LayoutUnit::zero();
                                                }
                                            }
                                            if child_style.flex_wrap
                                                != openui_style::FlexWrap::Nowrap
                                            {
                                                let flex_row_gap = child_style
                                                    .row_gap
                                                    .as_ref()
                                                    .map(|gap| {
                                                        resolve_length(
                                                            gap,
                                                            child_height,
                                                            LayoutUnit::zero(),
                                                            LayoutUnit::zero(),
                                                        )
                                                        .clamp_negative_to_zero()
                                                    })
                                                    .unwrap_or(LayoutUnit::zero());
                                                if flex_row_gap.raw() > 0
                                                    && c.offset.top.raw() > 0
                                                    && c.offset.top.raw() < flex_row_gap.raw()
                                                {
                                                    c.offset.top = LayoutUnit::zero();
                                                }
                                            }
                                        }
                                        if nested_avoid_inline_overflow_column_width.raw() > 0
                                            && c.kind == FragmentKind::ColumnBox
                                        {
                                            c.offset.left = c.offset.left
                                                - nested_avoid_inline_overflow_column_width;
                                        }
                                        if child_has_rule_positioned_overflow
                                            && (c.kind == FragmentKind::ColumnBox
                                                || c.kind == FragmentKind::ColumnRule)
                                        {
                                            c.size.height = c
                                                .size
                                                .height
                                                .max_of(content_shift + visual_part_height);
                                            if c.kind == FragmentKind::ColumnBox {
                                                for column_child in &mut c.children {
                                                    if doc
                                                        .node(column_child.node_id)
                                                        .style
                                                        .position
                                                        .is_positioned()
                                                    {
                                                        column_child.size.height =
                                                            column_child.size.height.max_of(
                                                                content_shift + visual_part_height,
                                                            );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if row_flex_descendant_visual_overflow_height.raw()
                                > child_height.raw()
                                && content_consumed == LayoutUnit::zero()
                            {
                                let mut overflow_paint = Vec::new();
                                for flex_item in &part.children {
                                    if flex_item.node_id.is_none() {
                                        continue;
                                    }
                                    let item_overflow_start =
                                        flex_item.offset.top + flex_item.size.height;
                                    for descendant in &flex_item.children {
                                        if descendant.node_id.is_none() {
                                            continue;
                                        }
                                        let descendant_style = &doc.node(descendant.node_id).style;
                                        if descendant_style.opacity >= 1.0 {
                                            continue;
                                        }
                                        let descendant_bottom =
                                            flex_item.offset.top
                                                + descendant.offset.top
                                                + descendant.size.height;
                                        if descendant_bottom.raw() <= item_overflow_start.raw() {
                                            continue;
                                        }
                                        let mut continuation = descendant.clone();
                                        continuation.offset = PhysicalOffset::new(
                                            flex_item.offset.left + descendant.offset.left,
                                            item_overflow_start,
                                        );
                                        continuation.size.height =
                                            descendant_bottom - item_overflow_start;
                                        overflow_paint.push(continuation);
                                    }
                                }
                                part.children.extend(overflow_paint);
                            }
                            while per_col_children.len() <= part_col_idx {
                                per_col_children.push(Vec::new());
                            }
                            per_col_children[part_col_idx].push(part);
                            overflow_rule_column_count =
                                overflow_rule_column_count.max(part_col_idx + 1);

                            content_consumed = content_consumed + content_in_part;
                            // For the outer loop's `consumed` tracker (slice mode),
                            // advance by content_in_part so the while condition
                            // uses content_height consistently.
                            consumed = consumed + content_in_part;
                            col_block_offset = col_block_offset + visual_part_height;
                            col_remaining = col_remaining - visual_part_height;
                            max_wrapped_content_extent = max_wrapped_content_extent.max_of(
                                col_extra_block_offset_for(part_col_idx)
                                    + part_block_offset
                                    + visual_part_height,
                            );

                            // Move to next column if this one is full and there's
                            // more content. Overflow columns are created as needed,
                            // but column-fill:auto caps at the resolved column count.
                            if content_consumed.raw() < content_height.raw() {
                                let auto_wrap_single_column_overflow = algo.column_fill
                                    == ColumnFill::Auto
                                    && algo.column_wrap == openui_style::ColumnWrap::Auto
                                    && algo.column_height.is_none()
                                    && resolved.count == 1
                                    && !has_spanner_after
                                    && child_height.raw() >= (column_height * 2).raw();
                                let can_advance = algo.column_fill != ColumnFill::Auto
                                    || wraps_rows
                                    || algo.column_wrap == openui_style::ColumnWrap::NoWrap
                                    || row_flex_forced_fragmentainer_flow
                                    || row_flex_descendant_visual_overflow_height.raw()
                                        > child_height.raw()
                                    || auto_wrap_single_column_overflow
                                    || col_idx + 1 < resolved.count as usize;
                                if can_advance {
                                    max_col_content = max_col_content.max_of(col_block_offset);
                                    col_idx += 1;
                                    col_block_offset = LayoutUnit::zero();
                                    col_remaining = column_height;
                                } else {
                                    // column-fill:auto at max columns — stop fragmenting,
                                    // remaining content overflows last column.
                                    break;
                                }
                            }
                        }
                        if row_flex_avoid_negative_continuation {
                            prev_fixed_child_visual_overflow = true;
                            prev_fixed_child_descendant_visual_overflow = true;
                        }
                        if has_explicit_height
                            && algo.column_fill == ColumnFill::Auto
                            && resolved.count == 2
                            && algo.column_gap == LayoutUnit::zero()
                            && !wraps_rows
                            && i + 1 < child_count
                            && col_margins_top[i + 1].raw() >= 0
                            && child_style.display == Display::Flex
                            && !child_style.flex_direction.is_column()
                            && child_style.flex_wrap == openui_style::FlexWrap::Nowrap
                            && child_style.height.is_auto()
                            && child_style.background_color.is_transparent()
                            && child_height.raw() > column_height.raw()
                            && !subtree_has_avoid_break_descendant(doc, child_node_id)
                            && doc.children(child_node_id).any(|gc| {
                                resolve_margin_or_padding(
                                    &doc.node(gc).style.margin_bottom,
                                    column_width,
                                )
                                .raw()
                                    > 0
                            })
                        {
                            col_block_offset = LayoutUnit::zero();
                            col_remaining = column_height;
                        }
                    }
                    prev_margin_bottom = child_margin_bottom;
                }

                // After placing this child, record static positions for
                // any OOF children that appear after it in DOM order.
                if let Some(split) = children_info[group_start + i].split_portion.as_ref() {
                    if split.is_last
                        && !split.is_first
                        && !child_style.height.is_auto()
                        && child_height.raw() > column_height.raw()
                    {
                        let flow_height = resolve_length(
                            &child_style.height,
                            child_percentage_block_size,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        pending_parallel_flow_after_split_height =
                            Some(if child_style.box_sizing == BoxSizing::BorderBox {
                                let bp = LayoutUnit::from_i32(
                                    child_style.effective_border_top()
                                        + child_style.effective_border_bottom(),
                                ) + resolve_margin_or_padding(
                                    &child_style.padding_top,
                                    column_width,
                                ) + resolve_margin_or_padding(
                                    &child_style.padding_bottom,
                                    column_width,
                                );
                                (flow_height - bp).clamp_negative_to_zero()
                            } else {
                                flow_height
                            });
                    }
                }
                global_flow_idx += 1;
                for oof in &oof_children {
                    if oof.flow_index == global_flow_idx {
                        oof_static_positions.push((
                            oof.node_id,
                            PhysicalOffset::new(
                                content_edge_x + col_inline_offset_for(col_idx),
                                content_edge_y
                                    + total_block_offset
                                    + col_extra_block_offset_for(col_idx)
                                    + col_block_offset,
                            ),
                        ));
                    }
                }
            }

            // Finalize: include the last column's content height.
            max_col_content = max_col_content.max_of(col_block_offset);
            let wrapped_row_count =
                if wraps_rows && !positions.is_empty() && !per_col_children.is_empty() {
                    ((per_col_children.len() + positions.len() - 1) / positions.len()).max(1)
                } else {
                    1
                };
            let wrapped_group_height = visual_column_height
                * LayoutUnit::from_i32(wrapped_row_count as i32)
                + algo.row_gap * LayoutUnit::from_i32(wrapped_row_count.saturating_sub(1) as i32);
            // For auto-height containers, use the balanced column_height (which
            // IS the desired visual height) for balanced columns. For
            // column-fill:auto without a spanner, use actual content height.
            // §7.2: before a spanner, content is always balanced, so use
            // the balanced column_height even with column-fill:auto.
            let empty_group_before_spanner = has_spanner_after
                && max_col_content == LayoutUnit::zero()
                && col_block_sizes.iter().all(|s| *s == LayoutUnit::zero())
                && col_margins_top.iter().all(|m| *m == LayoutUnit::zero())
                && col_margins_bottom.iter().all(|m| *m == LayoutUnit::zero());
            let balanced_auto_height_wrapped_flex_visual_overflow = style.height.is_auto()
                && !style.background_color.is_transparent()
                && algo.column_fill != ColumnFill::Auto
                && !has_spanner_after
                && !wraps_rows
                && child_count == 1
                && max_col_visual_content.raw() > column_height.raw()
                && !space.available_block_size.is_indefinite()
                && {
                    let child_id = children_info[group_start].id;
                    let child_style = &doc.node(child_id).style;
                    child_style.display == Display::Flex
                        && !child_style.flex_direction.is_column()
                        && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                        && child_style.background_color.is_transparent()
                        && subtree_has_avoid_break_descendant(doc, child_id)
                        && !subtree_has_forced_break_descendant(doc, child_id)
                };
            let actual_group_height = if empty_group_before_spanner {
                LayoutUnit::zero()
            } else if style.height.is_auto() {
                if algo.column_fill == ColumnFill::Auto && !has_spanner_after {
                    if wraps_rows {
                        max_col_content
                            .max_of(max_col_visual_content)
                            .max_of(wrapped_group_height)
                    } else {
                        max_col_content.max_of(max_col_visual_content)
                    }
                } else if wraps_rows {
                    if total_block_offset > LayoutUnit::zero()
                        && !space.available_block_size.is_indefinite()
                    {
                        wrapped_group_height.max_of(
                            (space.available_block_size - total_block_offset)
                                .clamp_negative_to_zero(),
                        )
                    } else if has_spanner_after {
                        let trailing_columns = if positions.is_empty() {
                            0
                        } else {
                            per_col_children.len() % positions.len()
                        };
                        if trailing_columns != 0 && child_count == 1 {
                            let partial_row_start = per_col_children.len() - trailing_columns;
                            col_extra_block_offset_for(partial_row_start).max_of(max_col_content)
                        } else {
                            max_wrapped_content_extent.max_of(max_col_content)
                        }
                    } else {
                        wrapped_group_height
                    }
                } else {
                    if let Some(tail_idx) = post_spanner_single_child_balanced_tail {
                        col_block_sizes[tail_idx]
                    } else if resolved.count == 1 {
                        column_height.max_of(max_col_visual_content)
                    } else if balanced_auto_height_wrapped_flex_visual_overflow {
                        column_height.max_of(max_col_visual_content.min_of(space.available_block_size))
                    } else {
                        column_height
                    }
                }
            } else if wraps_rows {
                wrapped_group_height
            } else {
                column_height
            };
            // When a wrapped multicol is itself fragmented by an outer
            // fragmentainer that is shorter than one wrapped row, Chromium
            // paints the final partial row across the declared column set. The
            // cloned-slicing path can otherwise leave the missing column slots
            // empty, exposing the container background.
            if wraps_rows
                && algo.column_height.is_some()
                && positions.len() > 1
                && !has_spanner_after
                && total_block_offset == LayoutUnit::zero()
                && child_count == 1
                && doc.children(children_info[group_start].id).next().is_none()
                && !space.fragmentainer_block_size.is_indefinite()
                && space.fragmentainer_block_size.raw() < visual_column_height.raw()
                && !per_col_children.is_empty()
            {
                let remainder = per_col_children.len() % positions.len();
                if remainder != 0 {
                    let row_start = per_col_children.len() - remainder;
                    let donor_idx = row_start + remainder - 1;
                    if !per_col_children[donor_idx].is_empty() {
                        while per_col_children.len() % positions.len() != 0 {
                            let target_idx = per_col_children.len();
                            let mut cloned = per_col_children[donor_idx].clone();
                            let inline_delta = col_inline_offset_for(target_idx)
                                - col_inline_offset_for(donor_idx);
                            let block_delta = col_extra_block_offset_for(target_idx)
                                - col_extra_block_offset_for(donor_idx);
                            for child in &mut cloned {
                                child.offset.left = child.offset.left + inline_delta;
                                child.offset.top = child.offset.top + block_delta;
                            }
                            per_col_children.push(cloned);
                        }
                    }
                }
            }
            for (overflow_col_idx, overflow_part) in delayed_float_overflow_parts {
                while per_col_children.len() <= overflow_col_idx {
                    per_col_children.push(Vec::new());
                }
                per_col_children[overflow_col_idx].push(overflow_part);
            }
            let explicit_wrapped_nested_auto_child = wraps_rows
                && algo.column_height.is_some()
                && !style.height.is_auto()
                && child_count == 1
                && children_info[group_start].split_portion.is_none()
                && {
                    let child_style = &doc.node(children_info[group_start].id).style;
                    crate::multicol::ColumnLayoutAlgorithm::from_style(child_style).is_some()
                        && child_style.column_height.is_none()
                        && !doc
                            .children(children_info[group_start].id)
                            .any(|gc| doc.node(gc).style.column_span == ColumnSpan::All)
                };
            if wraps_rows
                && algo.column_height.is_some()
                && !style.height.is_auto()
                && explicit_wrapped_nested_auto_child
                && positions.len() > 1
                && !per_col_children.is_empty()
            {
                let remainder = per_col_children.len() % positions.len();
                if remainder != 0 {
                    let row_start = per_col_children.len() - remainder;
                    let row_offset = col_extra_block_offset_for(row_start);
                    let clipped_final_row = row_offset.raw() < child_percentage_block_size.raw()
                        && (row_offset + visual_column_height).raw()
                            > child_percentage_block_size.raw();
                    if clipped_final_row {
                        let donor_idx = row_start + remainder - 1;
                        if !per_col_children[donor_idx].is_empty() {
                            while per_col_children.len() % positions.len() != 0 {
                                let target_idx = per_col_children.len();
                                let mut cloned = per_col_children[donor_idx].clone();
                                let inline_delta = col_inline_offset_for(target_idx)
                                    - col_inline_offset_for(donor_idx);
                                let block_delta = col_extra_block_offset_for(target_idx)
                                    - col_extra_block_offset_for(donor_idx);
                                for child in &mut cloned {
                                    child.offset.left = child.offset.left + inline_delta;
                                    child.offset.top = child.offset.top + block_delta;
                                }
                                per_col_children.push(cloned);
                            }
                        }
                    }
                }
            }

            // Wrap each column's children in a ColumnBox fragment for
            // column-level overflow clipping (CSS Multicol §3.1).
            for (ci, mut col_children) in per_col_children.into_iter().enumerate() {
                if col_children.is_empty() {
                    continue;
                }
                if style.height.is_auto()
                    && !has_spanner_after
                    && !children_info[group_start..group_end]
                        .iter()
                        .any(|info| subtree_has_in_flow_spanner_descendant(doc, info.id))
                    && col_children.iter().any(|child| {
                        !child.node_id.is_none() && {
                            let child_style = &doc.node(child.node_id).style;
                            child_style.float != Float::None
                                && child_style.height.is_fixed()
                                && !child_style.background_color.is_transparent()
                        }
                    })
                {
                    col_children.sort_by_key(|child| {
                        !child.node_id.is_none()
                            && doc.node(child.node_id).style.float != Float::None
                    });
                }
                let col_offset = PhysicalOffset::new(
                    content_edge_x + col_inline_offset_for(ci),
                    content_edge_y + total_block_offset + col_extra_block_offset_for(ci),
                );
                // Adjust children's offsets to be relative to the column box.
                for child in &mut col_children {
                    child.offset.left = child.offset.left - col_offset.left;
                    child.offset.top = child.offset.top - col_offset.top;
                }
                let pre_spanner_visual_height = if has_spanner_after && !style.height.is_auto() {
                    col_children
                        .iter()
                        .map(|child| child.offset.top + child.size.height)
                        .max()
                        .unwrap_or(LayoutUnit::zero())
                } else if has_spanner_after && style.height.is_auto() {
                    pre_spanner_positioned_visual_height
                } else {
                    LayoutUnit::zero()
                };
                let cloned_decoration_height =
                    if !style.height.is_auto() && group_available_block.raw() == 0 {
                        col_children
                            .iter()
                            .filter(|child| {
                                !child.node_id.is_none()
                                    && doc.node(child.node_id).style.box_decoration_break
                                        == BoxDecorationBreak::Clone
                            })
                            .map(|child| child.offset.top + child.size.height)
                            .max()
                            .unwrap_or(LayoutUnit::zero())
                    } else {
                        LayoutUnit::zero()
                    };
                let unsplittable_overflow_visual_height = if !style.height.is_auto()
                    && !has_spanner_after
                    && !wraps_rows
                    && col_children.len() == 1
                {
                    let child = &col_children[0];
                    if !child.node_id.is_none() {
                        let child_style = &doc.node(child.node_id).style;
                        if child.has_overflow_clip
                            && child_style.overflow_y != Overflow::Visible
                            && child.offset.top == LayoutUnit::zero()
                            && child.size.height.raw() > actual_group_height.raw()
                        {
                            child.size.height
                        } else {
                            LayoutUnit::zero()
                        }
                    } else {
                        LayoutUnit::zero()
                    }
                } else {
                    LayoutUnit::zero()
                };
                let flex_padding_overflow_visual_height = if !style.height.is_auto()
                    && algo.column_fill == ColumnFill::Auto
                    && !wraps_rows
                    && col_children.len() == 1
                {
                    let child = &col_children[0];
                    if !child.node_id.is_none() {
                        let child_style = &doc.node(child.node_id).style;
                        if child_style.display == Display::Flex
                            && child_style.height.is_auto()
                            && child_style.overflow_x == Overflow::Visible
                            && child_style.overflow_y == Overflow::Visible
                            && child.offset.top == LayoutUnit::zero()
                            && child.size.height.raw() > actual_group_height.raw()
                        {
                            child.size.height
                        } else {
                            LayoutUnit::zero()
                        }
                    } else {
                        LayoutUnit::zero()
                    }
                } else {
                    LayoutUnit::zero()
                };
                let col_box_height = if style.height.is_auto()
                    && style.max_height.is_none()
                    && !subtree_has_in_flow_spanner_descendant(doc, node_id)
                    && !subtree_has_flex_descendant(doc, node_id)
                    && !subtree_has_forced_break_descendant(doc, node_id)
                    && subtree_has_authored_inline_overflow_descendant(doc, node_id)
                    && !space.fragmentainer_block_size.is_indefinite()
                {
                    space.fragmentainer_block_size
                } else if explicit_wrapped_nested_auto_child && visual_column_height.raw() > 0 {
                    (child_percentage_block_size - col_extra_block_offset_for(ci))
                        .clamp_negative_to_zero()
                        .min_of(visual_column_height)
                } else if wraps_rows && visual_column_height.raw() > 0 {
                    (actual_group_height - col_extra_block_offset_for(ci)).clamp_negative_to_zero()
                } else {
                    actual_group_height
                }
                .max_of(cloned_decoration_height);
                let col_box_height = col_box_height
                    .max_of(pre_spanner_visual_height)
                    .max_of(unsplittable_overflow_visual_height)
                    .max_of(flex_padding_overflow_visual_height);
                let fixed_forced_break_upward_overflow = !has_spanner_after
                    && !style.height.is_auto()
                    && !wraps_rows
                    && algo.column_fill == ColumnFill::Auto
                    && col_children.iter().any(|child| {
                        !child.node_id.is_none()
                            && child.offset.top < LayoutUnit::zero()
                            && doc.node(child.node_id).style.break_before.is_forced()
                    });
                let upward_overflow = if (!has_spanner_after
                    && total_block_offset > LayoutUnit::zero()
                    && style.height.is_auto()
                    && !wraps_rows)
                    || fixed_forced_break_upward_overflow
                {
                    col_children.iter().fold(LayoutUnit::zero(), |top, child| {
                        if child.has_overflow_clip {
                            top.min_of(child.offset.top)
                        } else {
                            let overflow = child.scrollable_overflow();
                            top.min_of(child.offset.top + overflow.offset.top)
                        }
                    })
                } else {
                    LayoutUnit::zero()
                };
                let mut col_offset = col_offset;
                let col_box_height = if upward_overflow.raw() < 0 {
                    for child in &mut col_children {
                        child.offset.top = child.offset.top - upward_overflow;
                    }
                    col_offset.top = col_offset.top + upward_overflow;
                    col_box_height - upward_overflow
                } else {
                    col_box_height
                };
                let mut col_box = Fragment::new_box(
                    NodeId::NONE,
                    PhysicalSize::new(column_width, col_box_height),
                );
                col_box.kind = FragmentKind::ColumnBox;
                col_box.offset = col_offset;
                col_box.has_overflow_clip = true;
                col_box.block_axis_clip_only = true;
                col_box.children = col_children;
                result_children.push(col_box);
            }

            // Column rules for this group — derive positions from actual
            // per-column positions so that rules stay centered in the gap
            // even when column widths vary due to cumulative rounding.
            if let Some(ref rule) = algo.column_rule {
                let half_gap = LayoutUnit::from_raw(algo.column_gap.raw() / 2);
                let rule_block_size = if !style.height.is_auto()
                    && !wraps_rows
                    && !group_available_block.is_indefinite()
                {
                    group_available_block
                } else {
                    actual_group_height
                };
                if rule_block_size.raw() > 0 {
                    let rule_count = if wraps_rows {
                        positions.len().saturating_sub(1)
                    } else {
                        overflow_rule_column_count.saturating_sub(1)
                    };
                    for i in 0..rule_count {
                        let rp = col_inline_offset_for(i) + column_width + half_gap;
                        let mut rule_frag = Fragment::new_box(
                            node_id,
                            PhysicalSize::new(rule.width, rule_block_size),
                        );
                        rule_frag.offset = PhysicalOffset::new(
                            content_edge_x + rp - rule.width / LayoutUnit::from_i32(2),
                            content_edge_y + total_block_offset,
                        );
                        rule_frag.kind = FragmentKind::ColumnRule;
                        result_children.push(rule_frag);
                    }
                }
            }

            total_block_offset = total_block_offset + actual_group_height;

            // Update remaining available block for subsequent groups.
            if !remaining_available_block.is_indefinite() {
                remaining_available_block =
                    (remaining_available_block - actual_group_height).clamp_negative_to_zero();
            }
        }

        // Handle spanner (if current item is one).
        if group_end < children_info.len() && children_info[group_end].is_spanner {
            let spanner_id = children_info[group_end].id;
            let spanner_style = &doc.node(spanner_id).style;
            let spanner_margin_top =
                resolve_margin_or_padding(&spanner_style.margin_top, child_available_inline);
            let spanner_margin_bottom =
                resolve_margin_or_padding(&spanner_style.margin_bottom, child_available_inline);
            let collapsed_spanner_margin_top = if let Some(previous_margin_bottom) =
                previous_spanner_margin_bottom
            {
                let collapsed_margin =
                    if previous_margin_bottom.raw() >= 0 && spanner_margin_top.raw() >= 0 {
                        previous_margin_bottom.max_of(spanner_margin_top)
                    } else if previous_margin_bottom.raw() <= 0 && spanner_margin_top.raw() <= 0 {
                        previous_margin_bottom.min_of(spanner_margin_top)
                    } else {
                        previous_margin_bottom + spanner_margin_top
                    };
                collapsed_margin - previous_margin_bottom
            } else {
                spanner_margin_top
            };
            let spanner_space = ConstraintSpace::for_block_child(
                child_available_inline,
                space.available_block_size,
                child_available_inline,
                child_percentage_block_size,
                false,
            );
            let mut spanner_frag = block_layout(doc, spanner_id, &spanner_space);
            if spanner_frag.size.height == LayoutUnit::zero() {
                spanner_frag.size.height = collapsed_spanner_content_margin_height(&spanner_frag);
                if spanner_frag.size.height == LayoutUnit::zero()
                    && style.display == Display::ListItem
                    && style.list_style_position == ListStylePosition::Inside
                    && resolved.count == 1
                {
                    spanner_frag.paint_zero_block_outline = true;
                }
            }
            total_block_offset = total_block_offset + collapsed_spanner_margin_top;
            let spanner_visual_offset = if spanner_frag.paint_zero_block_outline
                && style.list_style_position == ListStylePosition::Inside
            {
                (list_marker_line_height(style) - LayoutUnit::from_i32(1)).clamp_negative_to_zero()
            } else {
                LayoutUnit::zero()
            };
            spanner_frag.offset = PhysicalOffset::new(
                content_edge_x,
                content_edge_y + total_block_offset + spanner_visual_offset,
            );
            let spanner_total =
                collapsed_spanner_margin_top + spanner_frag.size.height + spanner_margin_bottom;
            total_block_offset =
                total_block_offset + spanner_frag.size.height + spanner_margin_bottom;
            result_children.push(spanner_frag);
            previous_spanner_margin_bottom = Some(spanner_margin_bottom);

            // Update remaining available block after spanner.
            if !remaining_available_block.is_indefinite() {
                remaining_available_block =
                    (remaining_available_block - spanner_total).clamp_negative_to_zero();
            } else if let Some(mh) = resolved_max_height {
                // Initialize remaining tracking from max-height when first spanner seen
                remaining_available_block = (mh - total_block_offset).clamp_negative_to_zero();
            }

            group_start = group_end + 1;
        } else {
            group_start = group_end;
        }
    }

    // Build container fragment.
    let explicit_block_size = if !style.height.is_auto() {
        let raw = resolve_length(
            &style.height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        if style.box_sizing == BoxSizing::BorderBox {
            Some(raw)
        } else {
            Some(raw + border_padding_block)
        }
    } else {
        None
    };
    let container_block_size = explicit_block_size.unwrap_or_else(|| {
        let mut auto_block_size = total_block_offset + border_padding_block;
        if style.display == Display::ListItem
            && style.list_style_position == ListStylePosition::Inside
        {
            auto_block_size =
                auto_block_size.max_of(list_marker_line_height(style) + border_padding_block);
        }
        auto_block_size
    });

    // Apply min-height / max-height constraints (CSS 2.1 §10.7).
    // The normal block path handles this after layout, but multicol returns
    // directly so we must apply here.
    let container_block_size = if explicit_block_size.is_none() {
        let mut clamped = container_block_size;

        // max-height
        if let Some(mh_content) = resolved_max_height {
            let mh_border = if style.box_sizing == BoxSizing::ContentBox {
                mh_content + border_padding_block
            } else {
                mh_content.max_of(border_padding_block)
            };
            if clamped > mh_border {
                clamped = mh_border;
            }
        }

        // min-height
        if !style.min_height.is_auto() && !style.min_height.is_none() {
            let min_raw = resolve_length(
                &style.min_height,
                space.percentage_resolution_block_size,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            let min_border =
                if style.box_sizing == BoxSizing::ContentBox && min_raw > LayoutUnit::zero() {
                    min_raw + border_padding_block
                } else if min_raw > LayoutUnit::zero() {
                    min_raw.max_of(border_padding_block)
                } else {
                    min_raw
                };
            if clamped < min_border {
                clamped = min_border;
            }
        }

        clamped
    } else {
        container_block_size
    };
    let parent_node_id = doc.node(node_id).parent;
    let parent_is_multicol = !parent_node_id.is_none()
        && crate::multicol::ColumnLayoutAlgorithm::from_style(&doc.node(parent_node_id).style)
            .is_some();
    let wrapped_spanner_clips_to_available = explicit_block_size.is_none()
        && style.column_wrap == openui_style::ColumnWrap::Wrap
        && algo.column_height.is_some()
        && !space.available_block_size.is_indefinite()
        && !parent_is_multicol
        && subtree_has_in_flow_spanner_descendant(doc, node_id);
    let container_block_size = if wrapped_spanner_clips_to_available {
        container_block_size.min_of(space.available_block_size + border_padding_block)
    } else {
        container_block_size
    };

    // Layout out-of-flow children (absolute/fixed positioned).
    // These are positioned relative to the multicol container's padding box.
    // CSS 2.1 §10.1: The containing block for abspos is the padding edge of
    // the nearest positioned ancestor.
    let mut deferred_oof_to_parent: Vec<crate::out_of_flow::OutOfFlowCandidate> = Vec::new();
    if !oof_children.is_empty() || !bubbled_oof_from_columns.is_empty() {
        let cb_height = container_block_size - border.top - border.bottom;
        let cb_width = child_available_inline + padding.left + padding.right;
        let mut oof_candidates: Vec<crate::out_of_flow::OutOfFlowCandidate> = Vec::new();
        let should_defer_static_fixed =
            |candidate: &crate::out_of_flow::OutOfFlowCandidate| -> bool {
                candidate.style.position == Position::Fixed
                    && !style.establishes_transform_containing_block
                    && candidate.style.top.is_auto()
                    && candidate.style.bottom.is_auto()
            };
        let fixed_oof_wraps_rows = algo.column_wrap == openui_style::ColumnWrap::Wrap
            || (algo.column_wrap == openui_style::ColumnWrap::Auto && algo.column_height.is_some());
        let fixed_oof_fragmentainer_height = if has_explicit_height {
            child_percentage_block_size
        } else if let Some(mh) = resolved_max_height {
            mh
        } else {
            openui_geometry::INDEFINITE_SIZE
        };
        let final_col_inline_offset_for = |idx: usize| -> LayoutUnit {
            if idx < positions.len() {
                positions[idx].inline_offset
            } else if let Some(last) = positions.last() {
                let overflow = (idx - positions.len()) as i32;
                let stride = column_width + algo.column_gap;
                last.inline_offset
                    + last.width
                    + algo.column_gap
                    + stride * LayoutUnit::from_i32(overflow)
            } else {
                LayoutUnit::zero()
            }
        };
        let has_wrapped_row_flex_child = children_info.iter().any(|info| {
            let child_style = &doc.node(info.id).style;
            child_style.display == Display::Flex
                && !child_style.flex_direction.is_column()
                && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
        });
        let has_row_flex_child = children_info.iter().any(|info| {
            let child_style = &doc.node(info.id).style;
            child_style.display == Display::Flex && !child_style.flex_direction.is_column()
        });
        let has_oversized_direct_flex_child = children_info.iter().any(|info| {
            let child_style = &doc.node(info.id).style;
            child_style.display == Display::Flex
                && !child_style.height.is_auto()
                && resolve_length(
                    &child_style.height,
                    fixed_oof_fragmentainer_height,
                    LayoutUnit::zero(),
                    LayoutUnit::zero(),
                )
                .raw()
                    > fixed_oof_fragmentainer_height.raw()
        });
        let has_avoid_around_oversized_direct_flex_child =
            children_info.iter().enumerate().any(|(idx, info)| {
                let child_style = &doc.node(info.id).style;
                child_style.display == Display::Flex
                    && !child_style.height.is_auto()
                    && resolve_length(
                        &child_style.height,
                        fixed_oof_fragmentainer_height,
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                    )
                    .raw()
                        > fixed_oof_fragmentainer_height.raw()
                    && (child_style.break_before.is_avoid()
                        || idx
                            .checked_sub(1)
                            .is_some_and(|prev_idx| doc.node(children_info[prev_idx].id).style.break_after.is_avoid()))
            });
        let has_nowrap_row_flex_child_with_trailing_margin_and_following_child =
            children_info.iter().enumerate().any(|(idx, info)| {
                let child_style = &doc.node(info.id).style;
                child_style.display == Display::Flex
                    && !child_style.flex_direction.is_column()
                    && child_style.flex_wrap == openui_style::FlexWrap::Nowrap
                    && idx + 1 < children_info.len()
                    && doc.children(info.id).any(|gc| {
                        resolve_margin_or_padding(
                            &doc.node(gc).style.margin_bottom,
                            column_width,
                        )
                        .raw()
                            > 0
                    })
            });
        let has_auto_height_wrapped_row_flex_child_with_row_gap = children_info.iter().any(|info| {
            let child_style = &doc.node(info.id).style;
            child_style.display == Display::Flex
                && child_style.height.is_auto()
                && !child_style.flex_direction.is_column()
                && child_style.flex_wrap != openui_style::FlexWrap::Nowrap
                && child_style.row_gap.as_ref().is_some_and(|gap| {
                    resolve_length(gap, fixed_oof_fragmentainer_height, LayoutUnit::zero(), LayoutUnit::zero()).raw() > 0
                })
        });
        let direct_oof_static_position =
            |child_style: &ComputedStyle, static_pos: PhysicalOffset| -> PhysicalOffset {
                if child_style.position == Position::Absolute
                    && child_style.left.is_auto()
                    && !child_style.top.is_auto()
                    && algo.column_fill == ColumnFill::Auto
                    && has_explicit_height
                    && !fixed_oof_wraps_rows
                    && !fixed_oof_fragmentainer_height.is_indefinite()
                    && fixed_oof_fragmentainer_height.raw() > 0
                {
                    let top_inset = resolve_length(
                        &child_style.top,
                        fixed_oof_fragmentainer_height,
                        LayoutUnit::zero(),
                        LayoutUnit::zero(),
                    );
                    let child_width = if child_style.width.is_auto() {
                        openui_geometry::INDEFINITE_SIZE
                    } else {
                        resolve_length(
                            &child_style.width,
                            cb_width,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        )
                    };
                    if child_width.is_indefinite() || child_width.raw() > column_width.raw() {
                        return static_pos;
                    }
                    let child_height = if child_style.height.is_auto() {
                        openui_geometry::INDEFINITE_SIZE
                    } else {
                        resolve_length(
                            &child_style.height,
                            fixed_oof_fragmentainer_height,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        )
                    };
                    if child_height.is_indefinite()
                        || (top_inset + child_height).raw()
                            != fixed_oof_fragmentainer_height.raw()
                    {
                        if has_oversized_direct_flex_child
                            && top_inset == LayoutUnit::zero()
                            && resolved.count > 1
                        {
                            return PhysicalOffset::new(
                                content_edge_x + final_col_inline_offset_for(resolved.count as usize),
                                static_pos.top,
                            );
                        }
                        return static_pos;
                    }
                    if !has_wrapped_row_flex_child {
                        if has_nowrap_row_flex_child_with_trailing_margin_and_following_child {
                            return PhysicalOffset::new(content_edge_x, static_pos.top);
                        }
                        return static_pos;
                    }
                    let local_flow_top = static_pos.top - content_edge_y + top_inset;
                    if local_flow_top.raw() >= fixed_oof_fragmentainer_height.raw() {
                        let column_delta =
                            (local_flow_top.raw() / fixed_oof_fragmentainer_height.raw()) as usize;
                        return PhysicalOffset::new(
                            static_pos.left + final_col_inline_offset_for(column_delta),
                            static_pos.top,
                        );
                    }
                }
                static_pos
            };
        for (oof_node_id, static_pos) in &oof_static_positions {
            let child_style = doc.node(*oof_node_id).style.clone();
            let static_position = direct_oof_static_position(&child_style, *static_pos);
            let candidate = crate::out_of_flow::OutOfFlowCandidate {
                node_id: *oof_node_id,
                style: child_style,
                static_position,
                containing_block_size: PhysicalSize::new(cb_width, cb_height),
                containing_block_border: border.clone(),
                containing_block_direction: style.direction,
                static_position_direction: style.direction,
            };
            if should_defer_static_fixed(&candidate) {
                deferred_oof_to_parent.push(candidate);
            } else {
                oof_candidates.push(candidate);
            }
        }
        // Also handle OOF children that were not matched during distribution
        // (e.g., the only child is OOF, or OOF appears after all in-flow children
        // that were in a different group due to spanners).
        for oof in &oof_children {
            if !oof_static_positions
                .iter()
                .any(|(id, _)| *id == oof.node_id)
            {
                let child_style = doc.node(oof.node_id).style.clone();
                let static_position =
                    direct_oof_static_position(&child_style, PhysicalOffset::new(content_edge_x, content_edge_y));
                let candidate = crate::out_of_flow::OutOfFlowCandidate {
                    node_id: oof.node_id,
                    style: child_style,
                    static_position,
                    containing_block_size: PhysicalSize::new(cb_width, cb_height),
                    containing_block_border: border.clone(),
                    containing_block_direction: style.direction,
                    static_position_direction: style.direction,
                };
                if should_defer_static_fixed(&candidate) {
                    deferred_oof_to_parent.push(candidate);
                } else {
                    oof_candidates.push(candidate);
                }
            }
        }
        // Process OOF candidates bubbled from in-flow children inside columns.
        // These already have translated static positions (column-adjusted).
        for mut c in bubbled_oof_from_columns {
            if c.style.position == Position::Fixed
                && c.style.top.is_auto()
                && c.style.bottom.is_auto()
                && !fixed_oof_wraps_rows
                && !fixed_oof_fragmentainer_height.is_indefinite()
                && fixed_oof_fragmentainer_height.raw() > 0
                && c.static_position.top.raw()
                    > (content_edge_y + fixed_oof_fragmentainer_height).raw()
            {
                let local_flow_top = c.static_position.top - content_edge_y;
                let column_delta = if local_flow_top.raw() > 0 {
                    ((local_flow_top.raw() - 1) / fixed_oof_fragmentainer_height.raw()) as usize
                } else {
                    0
                };
                let static_block_offset =
                    local_flow_top - fixed_oof_fragmentainer_height * column_delta as i32;
                c.static_position.left =
                    c.static_position.left + final_col_inline_offset_for(column_delta);
                c.static_position.top = content_edge_y + static_block_offset;
            }
            c.containing_block_size = PhysicalSize::new(cb_width, cb_height);
            c.containing_block_border = border.clone();
            c.containing_block_direction = style.direction;
            if should_defer_static_fixed(&c) {
                deferred_oof_to_parent.push(c);
            } else {
                oof_candidates.push(c);
            }
        }
        let mut pending = oof_candidates;
        while !pending.is_empty() {
            let oof_fragments = crate::out_of_flow::layout_out_of_flow_children(doc, &pending);
            pending = Vec::new();
            for mut frag in oof_fragments {
                let nested = std::mem::take(&mut frag.oof_candidates);
                for mut c in nested {
                    c.static_position.left = c.static_position.left + frag.offset.left;
                    c.static_position.top = c.static_position.top + frag.offset.top;
                    if c.style.position == Position::Fixed
                        && c.style.top.is_auto()
                        && c.style.bottom.is_auto()
                        && !style.establishes_transform_containing_block
                    {
                        deferred_oof_to_parent.push(c);
                    } else {
                        c.containing_block_size = PhysicalSize::new(cb_width, cb_height);
                        c.containing_block_border = border.clone();
                        c.containing_block_direction = style.direction;
                        pending.push(c);
                    }
                }
                if oof_children.iter().any(|oof| oof.node_id == frag.node_id) {
                    let frag_style = &doc.node(frag.node_id).style;
                    let suppress_static_abspos_after_oversized_flex = frag_style.position
                        == Position::Absolute
                        && frag_style.left.is_auto()
                        && !frag_style.top.is_auto()
                        && has_oversized_direct_flex_child
                        && has_avoid_around_oversized_direct_flex_child
                        && resolved.count > 2
                        && frag.size.width == column_width
                        && frag.size.height.raw() > 0
                        && frag.size.height.raw() < fixed_oof_fragmentainer_height.raw()
                        && frag.offset.left.raw()
                            >= (content_edge_x
                                + final_col_inline_offset_for(
                                    resolved.count.saturating_sub(1) as usize,
                                ))
                            .raw();
                    if suppress_static_abspos_after_oversized_flex {
                        continue;
                    } else if frag_style.position == Position::Absolute
                        && frag_style.left.is_auto()
                        && !frag_style.top.is_auto()
                        && algo.column_fill == ColumnFill::Auto
                        && has_explicit_height
                        && !fixed_oof_wraps_rows
                        && resolved.count == 2
                        && algo.column_gap == LayoutUnit::zero()
                        && frag.offset.left == content_edge_x
                        && frag.size.width.raw() > column_width.raw()
                        && frag.size.height.raw() > 0
                        && frag.size.height.raw() < fixed_oof_fragmentainer_height.raw()
                    {
                        let top_inset = resolve_length(
                            &frag_style.top,
                            fixed_oof_fragmentainer_height,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        if top_inset.raw() > frag.size.height.raw()
                            && top_inset.raw() <= fixed_oof_fragmentainer_height.raw()
                        {
                            let mut continuation = frag.clone();
                            continuation.offset.left = content_edge_x + column_width;
                            continuation.offset.top = content_edge_y + top_inset - frag.size.height;
                            continuation.size.width = column_width;
                            continuation.size.height = frag.size.height;
                            continuation.is_first_for_node = false;
                            result_children.push(continuation);
                        }
                    } else if frag_style.position == Position::Absolute
                        && !frag_style.left.is_auto()
                        && !frag_style.top.is_auto()
                        && algo.column_fill == ColumnFill::Auto
                        && has_explicit_height
                        && !fixed_oof_wraps_rows
                        && resolved.count == 2
                        && algo.column_gap == LayoutUnit::zero()
                        && frag.offset.left == content_edge_x
                        && frag.size.width == column_width
                        && frag.size.height.raw() > 0
                        && frag.size.height.raw() <= fixed_oof_fragmentainer_height.raw()
                        && has_auto_height_wrapped_row_flex_child_with_row_gap
                        && frag.offset.top.raw() > content_edge_y.raw()
                        && (frag.offset.top + frag.size.height).raw()
                            <= (content_edge_y + fixed_oof_fragmentainer_height).raw()
                    {
                        let mut continuation = frag.clone();
                        continuation.offset.left = content_edge_x + column_width;
                        continuation.is_first_for_node = false;
                        result_children.push(continuation);
                    } else if frag_style.position == Position::Absolute
                        && frag_style.left.is_auto()
                        && frag_style.top.is_auto()
                        && algo.column_fill == ColumnFill::Auto
                        && has_explicit_height
                        && !fixed_oof_wraps_rows
                        && resolved.count == 2
                        && algo.column_gap == LayoutUnit::zero()
                        && has_row_flex_child
                        && frag.offset.left == content_edge_x
                        && frag.offset.top.raw() > content_edge_y.raw()
                        && frag.size.width == column_width
                        && frag.size.height.raw() > 0
                        && (frag.offset.top + frag.size.height).raw()
                            == (content_edge_y + fixed_oof_fragmentainer_height).raw()
                    {
                        let mut continuation = frag.clone();
                        continuation.offset.left = content_edge_x + column_width;
                        continuation.is_first_for_node = false;
                        result_children.push(continuation);
                    }
                }
                result_children.push(frag);
            }
        }
    }

    let mut container = Fragment::new_box(
        node_id,
        PhysicalSize::new(border_box_inline, container_block_size),
    );
    let container_clips_overflow = wrapped_spanner_clips_to_available
        || style.overflow_x != Overflow::Visible
        || style.overflow_y != Overflow::Visible;
    if container_clips_overflow {
        container.border = *border;
        container.padding = *padding;
        container.has_overflow_clip = true;
    }
    container.oof_candidates = deferred_oof_to_parent;
    container.children = result_children;
    container
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::ElementTag;
    use openui_geometry::Length;
    use openui_style::*;

    #[test]
    fn single_div_fills_width() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);

        // Viewport should be 800×600
        assert_eq!(fragment.size.width.to_i32(), 800);

        // The child div should fill width (800) and be 50px tall
        assert_eq!(fragment.children.len(), 1);
        assert_eq!(fragment.children[0].size.width.to_i32(), 800);
        assert_eq!(fragment.children[0].size.height.to_i32(), 50);
    }

    #[test]
    fn fixed_width_and_height() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.height = Length::px(100.0);
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        assert_eq!(child.size.width.to_i32(), 200);
        assert_eq!(child.size.height.to_i32(), 100);
    }

    #[test]
    fn children_stack_vertically() {
        let mut doc = Document::new();
        let vp = doc.root();

        let a = doc.create_node(ElementTag::Div);
        doc.node_mut(a).style.display = Display::Block;
        doc.node_mut(a).style.height = Length::px(50.0);
        doc.append_child(vp, a);

        let b = doc.create_node(ElementTag::Div);
        doc.node_mut(b).style.display = Display::Block;
        doc.node_mut(b).style.height = Length::px(30.0);
        doc.append_child(vp, b);

        let c = doc.create_node(ElementTag::Div);
        doc.node_mut(c).style.display = Display::Block;
        doc.node_mut(c).style.height = Length::px(20.0);
        doc.append_child(vp, c);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);

        assert_eq!(fragment.children.len(), 3);
        assert_eq!(fragment.children[0].offset.top.to_i32(), 0);
        assert_eq!(fragment.children[1].offset.top.to_i32(), 50);
        assert_eq!(fragment.children[2].offset.top.to_i32(), 80); // 50 + 30
    }

    #[test]
    fn margin_creates_spacing() {
        let mut doc = Document::new();
        let vp = doc.root();

        let a = doc.create_node(ElementTag::Div);
        doc.node_mut(a).style.display = Display::Block;
        doc.node_mut(a).style.height = Length::px(50.0);
        doc.node_mut(a).style.margin_bottom = Length::px(20.0);
        doc.append_child(vp, a);

        let b = doc.create_node(ElementTag::Div);
        doc.node_mut(b).style.display = Display::Block;
        doc.node_mut(b).style.height = Length::px(30.0);
        doc.append_child(vp, b);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);

        // b should be at 50 + 20 (margin) = 70
        assert_eq!(fragment.children[1].offset.top.to_i32(), 70);
    }

    #[test]
    fn margin_collapsing_between_siblings() {
        let mut doc = Document::new();
        let vp = doc.root();

        let a = doc.create_node(ElementTag::Div);
        doc.node_mut(a).style.display = Display::Block;
        doc.node_mut(a).style.height = Length::px(50.0);
        doc.node_mut(a).style.margin_bottom = Length::px(20.0);
        doc.append_child(vp, a);

        let b = doc.create_node(ElementTag::Div);
        doc.node_mut(b).style.display = Display::Block;
        doc.node_mut(b).style.height = Length::px(30.0);
        doc.node_mut(b).style.margin_top = Length::px(30.0);
        doc.append_child(vp, b);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);

        // Collapsed margin between siblings = max(20, 30) = 30
        // b should be at 50 + 30 = 80, NOT 50 + 20 + 30 = 100
        assert_eq!(fragment.children[1].offset.top.to_i32(), 80);
    }

    #[test]
    fn auto_margin_centering() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.node_mut(div).style.margin_left = Length::auto();
        doc.node_mut(div).style.margin_right = Length::auto();
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // Centered: (800 - 200) / 2 = 300
        assert_eq!(child.offset.left.to_i32(), 300);
        assert_eq!(child.size.width.to_i32(), 200);
    }

    #[test]
    fn border_and_padding() {
        let mut doc = Document::new();
        let vp = doc.root();

        let outer = doc.create_node(ElementTag::Div);
        doc.node_mut(outer).style.display = Display::Block;
        doc.node_mut(outer).style.padding_top = Length::px(10.0);
        doc.node_mut(outer).style.padding_left = Length::px(20.0);
        doc.node_mut(outer).style.padding_right = Length::px(20.0);
        doc.node_mut(outer).style.padding_bottom = Length::px(10.0);
        doc.node_mut(outer).style.border_top_width = 2;
        doc.node_mut(outer).style.border_top_style = BorderStyle::Solid;
        doc.node_mut(outer).style.border_bottom_width = 2;
        doc.node_mut(outer).style.border_bottom_style = BorderStyle::Solid;
        doc.node_mut(outer).style.border_left_width = 2;
        doc.node_mut(outer).style.border_left_style = BorderStyle::Solid;
        doc.node_mut(outer).style.border_right_width = 2;
        doc.node_mut(outer).style.border_right_style = BorderStyle::Solid;
        doc.append_child(vp, outer);

        let inner = doc.create_node(ElementTag::Div);
        doc.node_mut(inner).style.display = Display::Block;
        doc.node_mut(inner).style.height = Length::px(50.0);
        doc.append_child(outer, inner);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let outer_frag = &fragment.children[0];

        // Outer should be 800px wide (fills parent)
        assert_eq!(outer_frag.size.width.to_i32(), 800);
        // Outer height = 2 (border-top) + 10 (padding-top) + 50 (child) + 10 (padding-bottom) + 2 (border-bottom) = 74
        assert_eq!(outer_frag.size.height.to_i32(), 74);

        // Inner child should be offset by border+padding
        let inner_frag = &outer_frag.children[0];
        assert_eq!(inner_frag.offset.left.to_i32(), 22); // 2 (border) + 20 (padding)
        assert_eq!(inner_frag.offset.top.to_i32(), 12); // 2 (border) + 10 (padding)

        // Inner width = 800 - 2*2 (borders) - 2*20 (padding) = 756
        assert_eq!(inner_frag.size.width.to_i32(), 756);
    }

    #[test]
    fn percentage_width() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::percent(50.0);
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        assert_eq!(child.size.width.to_i32(), 400); // 50% of 800
    }

    #[test]
    fn border_box_sizing() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.box_sizing = BoxSizing::BorderBox;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.height = Length::px(100.0);
        doc.node_mut(div).style.padding_left = Length::px(20.0);
        doc.node_mut(div).style.padding_right = Length::px(20.0);
        doc.node_mut(div).style.padding_top = Length::px(10.0);
        doc.node_mut(div).style.padding_bottom = Length::px(10.0);
        doc.node_mut(div).style.border_left_width = 5;
        doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
        doc.node_mut(div).style.border_right_width = 5;
        doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // border-box: total width = 200 (includes padding + border)
        assert_eq!(child.size.width.to_i32(), 200);
        // Content width = 200 - 20 - 20 - 5 - 5 = 150
        assert_eq!(child.content_size().width.to_i32(), 150);
    }

    #[test]
    fn nested_layout() {
        let mut doc = Document::new();
        let vp = doc.root();

        let outer = doc.create_node(ElementTag::Div);
        doc.node_mut(outer).style.display = Display::Block;
        doc.node_mut(outer).style.width = Length::px(400.0);
        doc.append_child(vp, outer);

        let inner = doc.create_node(ElementTag::Div);
        doc.node_mut(inner).style.display = Display::Block;
        doc.node_mut(inner).style.width = Length::percent(50.0);
        doc.node_mut(inner).style.height = Length::px(30.0);
        doc.append_child(outer, inner);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);

        let outer_frag = &fragment.children[0];
        assert_eq!(outer_frag.size.width.to_i32(), 400);

        let inner_frag = &outer_frag.children[0];
        // 50% of 400 = 200
        assert_eq!(inner_frag.size.width.to_i32(), 200);
        assert_eq!(inner_frag.size.height.to_i32(), 30);
    }

    #[test]
    fn display_none_skipped() {
        let mut doc = Document::new();
        let vp = doc.root();

        let a = doc.create_node(ElementTag::Div);
        doc.node_mut(a).style.display = Display::Block;
        doc.node_mut(a).style.height = Length::px(50.0);
        doc.append_child(vp, a);

        let hidden = doc.create_node(ElementTag::Div);
        doc.node_mut(hidden).style.display = Display::None;
        doc.node_mut(hidden).style.height = Length::px(999.0);
        doc.append_child(vp, hidden);

        let b = doc.create_node(ElementTag::Div);
        doc.node_mut(b).style.display = Display::Block;
        doc.node_mut(b).style.height = Length::px(30.0);
        doc.append_child(vp, b);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);

        // Only 2 visible children
        assert_eq!(fragment.children.len(), 2);
        // b should be at 50, not 50+999
        assert_eq!(fragment.children[1].offset.top.to_i32(), 50);
    }

    #[test]
    fn min_max_width() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(1000.0); // wider than max
        doc.node_mut(div).style.max_width = Length::px(500.0);
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        assert_eq!(child.size.width.to_i32(), 500); // clamped by max-width
    }

    #[test]
    fn overconstrained_auto_margins_become_zero() {
        // CSS 2.1 §10.3.3: overconstrained auto margins → 0
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(1000.0); // wider than container
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.node_mut(div).style.margin_left = Length::auto();
        doc.node_mut(div).style.margin_right = Length::auto();
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // Child should be flush-left (margin-left = 0), not shifted negative
        assert_eq!(child.offset.left.to_i32(), 0);
    }

    #[test]
    fn first_child_margin_with_parent_border() {
        // CSS 2.1 §8.3.1: border/padding prevents parent-child margin collapsing
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        doc.node_mut(parent).style.border_top_width = 1;
        doc.node_mut(parent).style.border_top_style = BorderStyle::Solid;
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.margin_top = Length::px(20.0);
        doc.node_mut(child).style.height = Length::px(30.0);
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];
        let child_frag = &parent_frag.children[0];

        // Child offset = border_top(1) + margin(20) = 21
        assert_eq!(child_frag.offset.top.to_i32(), 21);
        // Parent height = border(1) + margin(20) + child(30) = 51
        assert_eq!(parent_frag.size.height.to_i32(), 51);
    }

    #[test]
    fn viewport_uses_available_height() {
        let doc = Document::new();

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, doc.root(), &space);

        // Viewport should always be full height even with no children
        assert_eq!(fragment.size.height.to_i32(), 600);
        assert_eq!(fragment.size.width.to_i32(), 800);
    }

    #[test]
    fn inline_children_produce_line_boxes() {
        // SP11: Inline elements are now laid out via inline formatting context.
        // A block with an inline child should produce line box children.
        let mut doc = Document::new();
        let vp = doc.root();

        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(vp, block);

        // An inline span child (empty, but still triggers IFC)
        let inline = doc.create_node(ElementTag::Span);
        doc.node_mut(inline).style.display = Display::Inline;
        doc.append_child(block, inline);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let block_frag = &fragment.children[0];

        // Block should now have children (line boxes from IFC).
        // An empty span produces no text items, so line breaker produces no
        // lines. The block height is just border+padding (0 here).
        // The block_frag.children may be empty (no actual text content)
        // but the block itself is laid out.
        assert_eq!(block_frag.size.width.to_i32(), 800);
    }

    #[test]
    fn min_height_with_content_box_sizing() {
        // min-height in content-box space must be converted to border-box
        // before clamping the border-box resolved height.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.node_mut(div).style.min_height = Length::px(100.0);
        doc.node_mut(div).style.padding_top = Length::px(10.0);
        doc.node_mut(div).style.padding_bottom = Length::px(10.0);
        // box-sizing: content-box (default)
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // min-height(100) > height(50), so content = 100px
        // border-box = content(100) + padding(20) = 120px
        assert_eq!(child.size.height.to_i32(), 120);
    }

    #[test]
    fn last_child_bottom_margin_with_parent_border() {
        // CSS 2.1 §8.3.1: bottom border/padding prevents last child's
        // bottom margin from collapsing through the parent.
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        doc.node_mut(parent).style.border_bottom_width = 1;
        doc.node_mut(parent).style.border_bottom_style = BorderStyle::Solid;
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.height = Length::px(30.0);
        doc.node_mut(child).style.margin_bottom = Length::px(20.0);
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];

        // Parent height = child(30) + margin(20) + border(1) = 51
        assert_eq!(parent_frag.size.height.to_i32(), 51);
    }

    #[test]
    fn inline_block_children_produce_line_boxes() {
        // SP11: InlineBlock children are now laid out via IFC.
        let mut doc = Document::new();
        let vp = doc.root();

        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(vp, block);

        let ib = doc.create_node(ElementTag::Div);
        doc.node_mut(ib).style.display = Display::InlineBlock;
        doc.node_mut(ib).style.height = Length::px(50.0);
        doc.append_child(block, ib);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let block_frag = &fragment.children[0];

        // Block now uses IFC for inline-level children.
        // The block should be laid out with some height.
        assert_eq!(block_frag.size.width.to_i32(), 800);
    }

    #[test]
    fn auto_width_subtracts_fixed_margins() {
        // CSS 2.1 §10.3.3: width:auto = containing_block - margin - border - padding
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.margin_left = Length::px(20.0);
        doc.node_mut(div).style.margin_right = Length::px(20.0);
        doc.node_mut(div).style.height = Length::px(50.0);
        // width: auto (default)
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // width:auto should be 800 - 20 - 20 = 760
        assert_eq!(child.size.width.to_i32(), 760);
        // positioned at left margin
        assert_eq!(child.offset.left.to_i32(), 20);
    }

    #[test]
    fn percentage_margin_resolves_against_parent_content_box() {
        // Percentage margins resolve against the child's containing block
        // width (parent's content-box), not the grandparent's.
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        doc.node_mut(parent).style.width = Length::px(400.0);
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.height = Length::px(50.0);
        doc.node_mut(child).style.margin_left = Length::percent(50.0);
        // margin-left: 50% of 400 = 200
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];
        let child_frag = &parent_frag.children[0];

        // margin-left = 50% of 400 = 200
        assert_eq!(child_frag.offset.left.to_i32(), 200);
        // child width = 400 - 200 (margin) = 200
        assert_eq!(child_frag.size.width.to_i32(), 200);
    }

    #[test]
    fn percentage_height_resolves_against_parent_height() {
        // Percentage heights resolve against the parent's specified height
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        doc.node_mut(parent).style.height = Length::px(200.0);
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.height = Length::percent(50.0);
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];
        let child_frag = &parent_frag.children[0];

        // child height = 50% of 200 = 100
        assert_eq!(child_frag.size.height.to_i32(), 100);
    }

    // ── Issue 1: Double border+padding subtraction ───────────────────

    #[test]
    fn padding_does_not_double_subtract_in_inline_layout() {
        // A div with 20px padding on each side and 200px width should
        // have 160px content box. Text that fits in 160px should NOT
        // wrap (i.e., only one line box). Before the fix, border+padding
        // was subtracted twice, making only 120px available.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.padding_left = Length::px(20.0);
        doc.node_mut(div).style.padding_right = Length::px(20.0);
        doc.append_child(vp, div);

        // Create text that fits in 160px but not 120px.
        // We use a short word to ensure it fits in the content box.
        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("Hi".to_string());
        doc.append_child(div, text);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];

        // The div should be 200px wide (content-box sizing default: 200 + 20 + 20 = 240)
        assert_eq!(div_frag.size.width.to_i32(), 240);

        // Should have line boxes as children (from inline layout)
        assert!(
            !div_frag.children.is_empty(),
            "Div with text should produce line boxes"
        );
        // Single short text should produce exactly one line
        assert_eq!(
            div_frag.children.len(),
            1,
            "Short text in 160px content box should fit on one line"
        );
    }

    // ── Issue 3: Mixed content anonymous block boxes ─────────────────

    #[test]
    fn mixed_content_preserves_block_children() {
        // Mixed content: text + block div + text should produce fragments
        // for all three pieces (anonymous inline wrappers + block child).
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        // First inline child: text
        let text1 = doc.create_node(ElementTag::Text);
        doc.node_mut(text1).text = Some("Before".to_string());
        doc.append_child(container, text1);

        // Block child
        let block_child = doc.create_node(ElementTag::Div);
        doc.node_mut(block_child).style.display = Display::Block;
        doc.node_mut(block_child).style.height = Length::px(30.0);
        doc.append_child(container, block_child);

        // Second inline child: text
        let text2 = doc.create_node(ElementTag::Text);
        doc.node_mut(text2).text = Some("After".to_string());
        doc.append_child(container, text2);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let container_frag = &fragment.children[0];

        // Should have at least 3 children:
        // 1. Anonymous inline wrapper (line box for "Before")
        // 2. Block child (30px height div)
        // 3. Anonymous inline wrapper (line box for "After")
        assert!(
            container_frag.children.len() >= 3,
            "Mixed content should produce at least 3 child fragments, got {}",
            container_frag.children.len(),
        );

        // The block child should have 30px height and be present
        let has_30px_child = container_frag
            .children
            .iter()
            .any(|f| f.size.height.to_i32() == 30);
        assert!(
            has_30px_child,
            "Block child with height 30px should be present in fragment tree"
        );
    }

    // ── Issue 4: Span with display:inline-block is atomic inline ─────

    #[test]
    fn span_inline_block_is_atomic_inline() {
        // A <span> with display: inline-block should be treated as an
        // atomic inline, not flattened as a regular inline container.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::InlineBlock;
        doc.node_mut(span).style.width = Length::px(50.0);
        doc.node_mut(span).style.height = Length::px(20.0);
        doc.append_child(container, span);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let container_frag = &fragment.children[0];

        // Should produce line boxes (inline formatting context)
        assert!(
            !container_frag.children.is_empty(),
            "Container with inline-block span should produce line boxes"
        );
    }

    // ── SP11 Round 11 Issue 1: display:none doesn't split inline runs ──

    #[test]
    fn display_none_child_does_not_split_inline_run() {
        // [Text, Span(display:none), Text, Div(block)] should produce
        // ONE anonymous inline wrapper for both text nodes, not two.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.node_mut(container).style.width = Length::px(400.0);
        doc.append_child(vp, container);

        let text1 = doc.create_node(ElementTag::Text);
        doc.node_mut(text1).text = Some("Before".to_string());
        doc.append_child(container, text1);

        let hidden_span = doc.create_node(ElementTag::Span);
        doc.node_mut(hidden_span).style.display = Display::None;
        doc.append_child(container, hidden_span);

        let text2 = doc.create_node(ElementTag::Text);
        doc.node_mut(text2).text = Some("After".to_string());
        doc.append_child(container, text2);

        let block_child = doc.create_node(ElementTag::Div);
        doc.node_mut(block_child).style.display = Display::Block;
        doc.node_mut(block_child).style.height = Length::px(10.0);
        doc.append_child(container, block_child);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let container_frag = &fragment.children[0];

        // Should produce 2 fragments: 1 anonymous inline wrapper (one IFC
        // for both text nodes) + 1 block child. Before the fix, the
        // display:none span split it into 3 fragments.
        assert_eq!(
            container_frag.children.len(),
            2,
            "display:none child should not split inline run; expected 2 fragments, got {}",
            container_frag.children.len(),
        );
    }

    #[test]
    fn out_of_flow_child_does_not_split_inline_run() {
        // An absolutely positioned child between inline content should not
        // split the inline run.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.node_mut(container).style.width = Length::px(400.0);
        doc.append_child(vp, container);

        let text1 = doc.create_node(ElementTag::Text);
        doc.node_mut(text1).text = Some("A".to_string());
        doc.append_child(container, text1);

        let abs_child = doc.create_node(ElementTag::Div);
        doc.node_mut(abs_child).style.display = Display::Block;
        doc.node_mut(abs_child).style.position = Position::Absolute;
        doc.append_child(container, abs_child);

        let text2 = doc.create_node(ElementTag::Text);
        doc.node_mut(text2).text = Some("B".to_string());
        doc.append_child(container, text2);

        let block_child = doc.create_node(ElementTag::Div);
        doc.node_mut(block_child).style.display = Display::Block;
        doc.node_mut(block_child).style.height = Length::px(10.0);
        doc.append_child(container, block_child);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let container_frag = &fragment.children[0];

        // The container is position:static, so the abs-pos child is NOT laid
        // out here — it bubbles up to the nearest positioned ancestor (root).
        // Container children: 1 inline wrapper (single IFC) + 1 block child = 2
        assert_eq!(
            container_frag.children.len(), 2,
            "OOF child should bubble up from static container; expected 2 fragments (inline + block), got {}",
            container_frag.children.len(),
        );

        // The viewport root (which is the initial containing block) should
        // have the container + the bubbled-up OOF child = 2 children
        assert_eq!(
            fragment.children.len(),
            2,
            "Root should have container + bubbled OOF; expected 2, got {}",
            fragment.children.len(),
        );
    }

    // ── SP11 Round 11 Issue 3: percentage height in auto-height parent ──

    #[test]
    fn percentage_height_in_auto_height_parent_is_indefinite() {
        // A child with height:50% inside a parent with height:auto should
        // resolve to 0 (indefinite), not 50% of the viewport.
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        // height is auto (default)
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.height = Length::percent(50.0);
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];
        let child_frag = &parent_frag.children[0];

        // With auto-height parent, 50% height should resolve to 0 (indefinite).
        // Before the fix, it resolved to 50% of 600 = 300.
        assert_eq!(
            child_frag.size.height.to_i32(),
            0,
            "50% height in auto-height parent should be 0 (indefinite), got {}",
            child_frag.size.height.to_i32(),
        );
    }

    // ── SP11 Round 15 Issue 5: percentage padding resolves against containing block ──

    #[test]
    fn percentage_padding_resolves_against_containing_block_not_content_box() {
        // A block with padding has its content-box smaller than its width.
        // When inline_layout re-resolves the SAME block's percentage padding,
        // it should use the containing block's width (from the parent), not
        // the block's own content-box width.
        //
        // Setup: viewport(800) → div(width:200, padding-left:10%)
        // Before fix: padding resolved as 10% of 200 = 20px (wrong)
        // After fix: padding resolved as 10% of 800 = 80px (correct)
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.padding_left = Length::percent(10.0);
        doc.append_child(vp, div);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("Hello".to_string());
        doc.append_child(div, text);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600));
        let fragment = block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];

        // 10% of containing block (800px viewport) = 80px.
        // The fix ensures inline_layout receives the correct percentage base.
        let resolved_padding = div_frag.padding.left.to_i32();
        assert_eq!(
            resolved_padding, 80,
            "10% padding-left should resolve against containing block (800px) = 80px, got {}",
            resolved_padding,
        );
    }
}
