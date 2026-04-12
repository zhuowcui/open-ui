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

use openui_geometry::{LayoutUnit, BfcOffset, BoxStrut, Length, LengthType, PhysicalOffset, PhysicalRect, PhysicalSize, MarginStrut};
use openui_style::{ComputedStyle, Display, BoxSizing, Overflow, Float, Clear, Position, Direction, BreakValue};
use openui_dom::{Document, NodeId};

use crate::constraint_space::ConstraintSpace;
use crate::exclusions::{ExclusionSpace, ClearType};
use crate::exclusions::float_utils::{UnpositionedFloat, position_float};
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
        let width_is_auto_like = style.width.is_auto() || style.width.is_stretch()
            || style.width.is_content_or_intrinsic();
        if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 && style.height.is_auto() && width_is_auto_like {
            let border_box_w = if style.box_sizing == BoxSizing::BorderBox {
                content_inline_size.max_of(border_padding_inline)
            } else {
                content_inline_size + border_padding_inline
            };

            // Compute tentative AR-derived height
            let (content_w, bp_i, bp_b) = if ar.auto_flag || style.box_sizing != BoxSizing::BorderBox {
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

            let tentative_h = LayoutUnit::from_f32(content_w.to_f32() * ar.ratio.1 / ar.ratio.0) + bp_b;

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
                    let new_w = LayoutUnit::from_f32(new_content_h.to_f32() * ar.ratio.0 / ar.ratio.1) + bp_i;
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
        return layout_multicol(doc, node_id, style, space, &algo,
            &border, &padding, border_padding_inline, border_padding_block,
            child_available_inline, content_inline_size, border_box_inline);
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
    let child_percentage_block_size = if space.is_fixed_block_size || space.stretch_block_size {
        // Definite block size from external constraint — use it directly.
        // Content-box: subtract border+padding.
        (space.available_block_size - border_padding_block).clamp_negative_to_zero()
    } else if !style.height.is_auto() {
        // A percentage height against an indefinite basis is itself indefinite.
        if style.height.is_percent()
            && space.percentage_resolution_block_size.is_indefinite()
        {
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
                let width_definite = !style.width.is_auto()
                    || space.available_inline_size.raw() > 0;
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
                            avail, avail,
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
    } else if !style.height.is_auto() && !style.height.is_stretch()
        && !style.height.is_content_or_intrinsic()
    {
        // Explicit height (px, %, etc.) — resolve it
        if style.height.is_percent()
            && space.percentage_resolution_block_size.is_indefinite()
        {
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
    let establishes_cb_for_abspos = style.position.is_positioned() || is_root;
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
        let mut exclusion_space_inline = ExclusionSpace::new();

        for child_id in doc.children(node_id) {
            let child_style = &doc.node(child_id).style;
            if child_style.display == Display::None {
                continue;
            }
            if child_style.position.is_absolutely_positioned() {
                let candidate = OutOfFlowCandidate {
                    node_id: child_id,
                    style: child_style.clone(),
                    static_position: PhysicalOffset::new(
                        border.left + padding.left,
                        block_offset,
                    ),
                    containing_block_size,
                    containing_block_border: border.clone(),
                    containing_block_direction: style.direction,
                    static_position_direction: style.direction,
                };
                let captures = if child_style.position == Position::Fixed {
                    is_root
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
                    doc, child_id, space,
                    child_available_inline, child_percentage_block_size,
                    &border, &padding, content_edge,
                    &block_offset, &mut exclusion_space_inline,
                    &mut child_fragments,
                    &mut oof_candidates, &mut bubbled_oof_candidates,
                    establishes_cb_for_abspos, is_root,
                    &mut max_float_bottom,
                );
            }
        }

        // Pre-collect inline items to detect block-in-inline (CSS 2.2 §9.2.1.1).
        let mut items_data = crate::inline::items_builder::InlineItemsBuilder::collect(doc, node_id);

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
                            matches!(item.item_type, InlineItemType::Text | InlineItemType::AtomicInline)
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
                            doc, node_id, &seg_space, &items_data,
                            segment_start_item, segment_end_item,
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
                    &block_child_style.margin_top, child_available_inline);
                let bm_bottom = resolve_margin_or_padding(
                    &block_child_style.margin_bottom, child_available_inline);

                current_block_offset = current_block_offset + bm_top;
                let mut positioned_block = block_child_frag;
                positioned_block.offset = PhysicalOffset::new(
                    border.left + padding.left,
                    current_block_offset,
                );
                current_block_offset = current_block_offset + positioned_block.size.height + bm_bottom;
                child_fragments.push(positioned_block);

                segment_start_item = segment_end_item + 1;
            }

            // Lay out remaining inline items after the last block-in-inline.
            if segment_start_item < items_data.items.len() {
                let has_content = items_data.items[segment_start_item..]
                    .iter()
                    .any(|item| {
                        use crate::inline::items::InlineItemType;
                        matches!(item.item_type, InlineItemType::Text | InlineItemType::AtomicInline)
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
                        doc, node_id, &seg_space, &items_data,
                        segment_start_item, items_data.items.len(),
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
                inline_space.exclusion_space =
                    Some(std::sync::Arc::new(exclusion_space_inline));
            }
            let inline_fragment = crate::inline::algorithm::inline_layout(
                doc, node_id, &inline_space,
            );

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
                intrinsic_block_size = intrinsic_block_size.max_of(
                    positioned_line.offset.top + line_height,
                );
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
        let mut exclusion_space_mixed = ExclusionSpace::new();

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
                    is_root
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
                    doc, child_id, space,
                    child_available_inline, child_percentage_block_size,
                    &border, &padding, content_edge,
                    &block_offset, &mut exclusion_space_mixed,
                    &mut child_fragments,
                    &mut oof_candidates, &mut bubbled_oof_candidates,
                    establishes_cb_for_abspos, is_root,
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
                            is_root
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
                    inline_space.bfc_offset = BfcOffset::new(
                        LayoutUnit::zero(),
                        block_offset - content_edge,
                    );
                }
                let anon_fragment = crate::inline::algorithm::inline_layout_for_children(
                    doc, node_id, inline_run, &inline_space,
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
                    intrinsic_block_size = intrinsic_block_size.max_of(
                        positioned_line.offset.top + line_height,
                    );
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
                    let child_top_margin = resolve_margins(child_style, child_available_inline).top;
                    let mut hyp_strut = margin_strut;
                    hyp_strut.append_normal(child_top_margin);
                    let hypothetical = block_offset + hyp_strut.sum();
                    let clearance_target = exclusion_space_mixed.clearance_offset(
                        clear_type_from_style(child_style.clear),
                    ) + content_edge;
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
                let (float_inline_offset, adjusted_available) = if exclusion_space_mixed.has_floats()
                    && child_is_new_fc_caller
                {
                    let child_top_margin = resolve_margins(child_style, child_available_inline).top;
                    let mut temp_strut = margin_strut;
                    temp_strut.append_normal(child_top_margin);
                    let resolved_offset = block_offset + temp_strut.sum();
                    let content_block_offset = resolved_offset - content_edge;
                    let min_inline_size = new_fc_min_inline_size(child_style, child_available_inline);

                    let opp = exclusion_space_mixed.find_layout_opportunity(
                        &BfcOffset::new(LayoutUnit::zero(), content_block_offset),
                        child_available_inline,
                        min_inline_size,
                    );

                    let pushed_bfc = opp.rect.block_start_offset();
                    if pushed_bfc > content_block_offset {
                        let push_amount = pushed_bfc - content_block_offset;
                        block_offset = block_offset + push_amount;
                        margin_strut = MarginStrut::new();
                        start_margin_resolved = true;
                    }

                    (opp.rect.line_start_offset(), opp.inline_size())
                } else {
                    (LayoutUnit::zero(), child_available_inline)
                };

                let oof_count_before = oof_candidates.len();
                let bubbled_count_before = bubbled_oof_candidates.len();

                layout_block_child(
                    doc, child_id, space,
                    adjusted_available, child_percentage_block_size,
                    children_available_block_size,
                    &border, &padding, content_edge,
                    &mut block_offset, &mut margin_strut,
                    &mut intrinsic_block_size, &mut child_fragments,
                    &mut oof_candidates, &mut bubbled_oof_candidates,
                    establishes_cb_for_abspos, is_root,
                    &mut start_margin_resolved,
                    &mut saved_start_strut,
                    style.direction,
                    &mut pending_self_collapsing,
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
    let mut exclusion_space = ExclusionSpace::new();

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
                is_root
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
        if child_style.display.is_inline_level() {
            continue;
        }

        // Handle floated children — they are positioned in the exclusion
        // space and do not advance the block offset.
        if child_style.float != Float::None {
            // CSS 2.1: Floats force BFC offset resolution.
            if !start_margin_resolved {
                block_offset += margin_strut.sum();
                margin_strut = MarginStrut::new();
                start_margin_resolved = true;
                float_resolved_bfc = true;
            }
            handle_float(
                doc, child_id, space,
                child_available_inline, child_percentage_block_size,
                &border, &padding, content_edge,
                &block_offset, &mut exclusion_space,
                &mut child_fragments,
                &mut oof_candidates, &mut bubbled_oof_candidates,
                establishes_cb_for_abspos, is_root,
                &mut max_float_bottom,
            );
            continue;
        }

        // Handle clear property — CSS 2.1 §8.3.1 / §9.5.2.
        // Compute hypothetical position (with margin collapsing) first,
        // then determine clearance as additional distance needed.
        // Clearance inhibits margin collapsing.
        if child_style.clear != Clear::None {
            let child_top_margin = resolve_margins(child_style, child_available_inline).top;
            let mut hyp_strut = margin_strut;
            hyp_strut.append_normal(child_top_margin);
            let hypothetical = block_offset + hyp_strut.sum();
            let clearance_target = exclusion_space.clearance_offset(
                clear_type_from_style(child_style.clear),
            ) + content_edge;
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
        let (float_inline_offset, adjusted_available) = if exclusion_space.has_floats()
            && child_is_new_fc_caller
        {
            // Tentative BFC block offset including margin collapsing.
            let child_top_margin = resolve_margins(child_style, child_available_inline).top;
            let mut temp_strut = margin_strut;
            temp_strut.append_normal(child_top_margin);
            let resolved_offset = block_offset + temp_strut.sum();
            let content_block_offset = resolved_offset - content_edge;
            let min_inline_size = new_fc_min_inline_size(child_style, child_available_inline);

            let opp = exclusion_space.find_layout_opportunity(
                &BfcOffset::new(LayoutUnit::zero(), content_block_offset),
                child_available_inline,
                min_inline_size,
            );

            // Push-down: if the layout opportunity starts below the child's
            // tentative position, push the child down.
            let pushed_bfc = opp.rect.block_start_offset();
            if pushed_bfc > content_block_offset {
                let push_amount = pushed_bfc - content_block_offset;
                block_offset = block_offset + push_amount;
                margin_strut = MarginStrut::new();
                start_margin_resolved = true;
            }

            (opp.rect.line_start_offset(), opp.inline_size())
        } else {
            (LayoutUnit::zero(), child_available_inline)
        };

        let oof_count_before = oof_candidates.len();
        let bubbled_count_before = bubbled_oof_candidates.len();

        layout_block_child(
            doc, child_id, space,
            adjusted_available, child_percentage_block_size,
            children_available_block_size,
            &border, &padding, content_edge,
            &mut block_offset, &mut margin_strut,
            &mut intrinsic_block_size, &mut child_fragments,
            &mut oof_candidates, &mut bubbled_oof_candidates,
            establishes_cb_for_abspos, is_root,
            &mut start_margin_resolved,
            &mut saved_start_strut,
            style.direction,
            &mut pending_self_collapsing,
        );

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
            && space.percentage_resolution_block_size.is_indefinite());
    let has_non_auto_height = !height_is_effectively_auto
        || space.is_fixed_block_size
        || space.stretch_block_size;
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
            || has_min_height);
    if end_margin_resolved {
        intrinsic_block_size += margin_strut.sum();
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
        let actual_content_height = (resolved_block_size - border_padding_block).clamp_negative_to_zero();
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
                    && child_style.top.length_type() == openui_geometry::LengthType::Percent;
                let has_pct_bottom = !child_style.bottom.is_auto()
                    && child_style.bottom.length_type() == openui_geometry::LengthType::Percent;
                if !has_pct_top && !has_pct_bottom {
                    continue;
                }
                let zero = LayoutUnit::zero();
                let old_offset = if has_pct_top {
                    if old_basis.is_indefinite() { zero }
                    else { resolve_length(&child_style.top, old_basis, zero, zero) }
                } else {
                    if old_basis.is_indefinite() { zero }
                    else { -resolve_length(&child_style.bottom, old_basis, zero, zero) }
                };
                let new_offset = if has_pct_top {
                    if target_basis.is_indefinite() { zero }
                    else { resolve_length(&child_style.top, target_basis, zero, zero) }
                } else {
                    if target_basis.is_indefinite() { zero }
                    else { -resolve_length(&child_style.bottom, target_basis, zero, zero) }
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
            let oof_fragments = crate::out_of_flow::layout_out_of_flow_children(
                doc,
                &pending,
            );
            pending = Vec::new();
            for mut frag in oof_fragments {
                // Collect nested OOF candidates from the just-laid-out fragment
                // and translate their static positions into parent coordinates.
                let nested = std::mem::take(&mut frag.oof_candidates);
                for mut c in nested {
                    c.static_position.left = c.static_position.left + frag.offset.left;
                    c.static_position.top = c.static_position.top + frag.offset.top;
                    let captures = if c.style.position == Position::Fixed {
                        is_root
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
    fragment.has_overflow_clip = style.overflow_x != Overflow::Visible
        || style.overflow_y != Overflow::Visible;

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
            fragment.start_margin_strut = saved;
        }
    } else if !start_margin_resolved {
        fragment.start_margin_strut = margin_strut;
    }
    fragment.end_margin_strut = final_end_margin_strut;
    fragment.float_resolved_bfc = float_resolved_bfc;

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
                fragment.last_baseline = Some(child.offset.top +
                    child.last_baseline.unwrap_or(child_first));
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
    is_root: bool,
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
            let ml = if child_style.margin_left.is_auto() { LayoutUnit::zero() } else { child_margin.left };
            let mr = if child_style.margin_right.is_auto() { LayoutUnit::zero() } else { child_margin.right };
            ml + mr
        };
        let stf_available = (child_available_inline - margin_inline).clamp_negative_to_zero();
        crate::out_of_flow::compute_shrink_to_fit_width(
            doc, child_id, stf_available,
        )
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
        let clearance = exclusion_space.clearance_offset(
            clear_type_from_style(child_style.clear),
        );
        if clearance > content_block_offset {
            content_block_offset = clearance;
        }
    }

    let unpositioned = UnpositionedFloat {
        node_id: child_id,
        available_size: child_available_inline,
        origin_bfc_offset: BfcOffset::new(LayoutUnit::zero(), content_block_offset),
        margins: child_margin,
        inline_size: child_fragment.size.width,
        block_size: child_fragment.size.height,
        is_left,
    };

    let (positioned, exclusion) = position_float(&unpositioned, exclusion_space);
    exclusion_space.add(exclusion);

    // CSS 2.1 §10.6.7: Track float bottom margin edge (BFC coordinates)
    // for auto-height BFC roots that must include float descendants.
    let float_bottom_bfc = positioned.bfc_offset.block_offset
        + child_fragment.size.height
        + child_margin.bottom;
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
                is_root
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
    is_root: bool,
    start_margin_resolved: &mut bool,
    saved_start_strut: &mut Option<MarginStrut>,
    containing_block_direction: Direction,
    pending_self_collapsing: &mut Vec<usize>,
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
    let child_space = ConstraintSpace::for_block_child(
        child_constrained_inline,
        children_available_block_size,
        child_available_inline,
        child_percentage_block_size,
        child_is_new_fc,
    );

    let mut child_fragment = block_layout(doc, child_id, &child_space);

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
    if child_fragment.float_resolved_bfc && !*start_margin_resolved {
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
                is_root
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

    // CSS 2.1 §8.3.1: Determine if this child is self-collapsing.
    // A box is self-collapsing if:
    //   - zero computed height (or auto resolving to zero)
    //   - no top/bottom border or padding
    //   - does not establish a new BFC
    //   - does not contain any line boxes or in-flow children
    // Check the DOM tree for in-flow children (not the fragment, which may
    // include float fragments). Floats/abspos are out-of-flow and don't
    // prevent self-collapsing per §8.3.1.
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
        && !child_has_in_flow_content;

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
        // Exception: if a descendant float forced BFC resolution (cascaded
        // via float_resolved_bfc), the margin boundary IS known and final.
        // Don't rollback and don't defer — position is at block_offset.
        if child_fragment.float_resolved_bfc {
            // Float cascade resolved the position — it's final.
            // Start a new strut from this boundary with the child's bottom margin.
            *margin_strut = MarginStrut::new();
            margin_strut.append_normal(child_margin.bottom);
            if !child_fragment.end_margin_strut.is_empty() {
                let child_end = child_fragment.end_margin_strut;
                margin_strut.append_normal(child_end.positive_margin);
                if child_end.negative_margin < LayoutUnit::zero() {
                    margin_strut.append_normal(child_end.negative_margin);
                }
            }
            // Also flush any previously pending self-collapsing blocks to
            // the current margin boundary (block_offset before height add).
            let boundary = child_fragment.offset.top;
            for &idx in pending_self_collapsing.iter() {
                child_fragments[idx].offset.top = boundary;
            }
            pending_self_collapsing.clear();
        } else {
            // Normal self-collapsing handling: rollback tentative resolution.
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
/// The min_inline_size is the start margin + border-box width. The end margin
/// can extend past the opportunity (it doesn't cause float overlap).
///
/// For auto-width: start margin + border + padding (content can shrink to 0).
/// For explicit width: start margin + border-box width.
fn new_fc_min_inline_size(
    style: &ComputedStyle,
    containing_inline: LayoutUnit,
) -> LayoutUnit {
    let margin = resolve_margins(style, containing_inline);
    // Only the start margin matters — it shifts the border box within the
    // opportunity. The end margin can overflow past the opportunity edge.
    // TODO: handle RTL (use margin_right as start margin)
    let margin_start = margin.left;
    let bp_left = LayoutUnit::from_i32(style.effective_border_left())
        + resolve_margin_or_padding(&style.padding_left, containing_inline);
    let bp_right = LayoutUnit::from_i32(style.effective_border_right())
        + resolve_margin_or_padding(&style.padding_right, containing_inline);
    let bp = bp_left + bp_right;

    if style.width.is_auto() {
        // Auto-width: can shrink to 0 content, so just start margin + border + padding
        margin_start + bp
    } else {
        // Explicit width: resolve and compute border-box width
        let w = resolve_length(
            &style.width, containing_inline, containing_inline, containing_inline,
        );
        let border_box_w = match style.box_sizing {
            BoxSizing::ContentBox => w + bp,
            BoxSizing::BorderBox => w,
        };
        margin_start + border_box_w
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
    // Intrinsic sizes include border+padding, convert to content-box
    let min_content = (intrinsic.min - border_padding).clamp_negative_to_zero();
    let max_content = (intrinsic.max - border_padding).clamp_negative_to_zero();
    match length.length_type() {
        LengthType::MinContent => min_content,
        LengthType::MaxContent => max_content,
        LengthType::FitContent => {
            // fit-content = clamp(min-content, available, max-content)
            let avail = (available - border_padding).clamp_negative_to_zero();
            avail.clamp(min_content, max_content)
        }
        LengthType::Stretch => (available - border_padding).clamp_negative_to_zero(),
        _ => (available - border_padding).clamp_negative_to_zero(), // fallback
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
        return (available - border_padding).clamp_negative_to_zero();
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
                if h.is_indefinite() { None } else {
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
                    Some(if style.box_sizing == BoxSizing::BorderBox && box_sizing_for_ar != BoxSizing::BorderBox {
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
        };
        if let Some(w) = ar_override {
            w
        } else {
            resolve_intrinsic_inline(doc, node_id, &style.width, available, border_padding)
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
        // aspect-ratio (that are not scroll containers), min-width:auto
        // resolves to the content-based minimum size (min-content width),
        // clamped from above by the maximum size (if definite).
        // Without AR, CSS 2.2 §10.4 applies: min-width:auto = 0.
        if style.aspect_ratio.is_some() && !style.is_scroll_container() {
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
    let (min, max) = if style.width.is_auto() || style.width.is_stretch()
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
        // CSS Sizing 4: height: stretch
        // Resolve to available block size minus margins.
        let avail = space.available_block_size;
        if !avail.is_indefinite() {
            let margin_block = resolve_margin_or_padding(&style.margin_top, space.available_inline_size)
                + resolve_margin_or_padding(&style.margin_bottom, space.available_inline_size);
            (avail - margin_block).clamp_negative_to_zero()
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
                    resolve_intrinsic_block(doc, node_id, &style.height, intrinsic_block_size, border_padding_block)
                }
            } else {
                resolve_intrinsic_block(doc, node_id, &style.height, intrinsic_block_size, border_padding_block)
            }
        } else {
            resolve_intrinsic_block(doc, node_id, &style.height, intrinsic_block_size, border_padding_block)
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
        let percentage_resolved_to_auto = style.height.length_type() == openui_geometry::LengthType::Percent
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
    let apply_automatic_min_size = style.min_height.is_auto()
        && has_ar
        && !style.is_scroll_container()
        && style.height.is_auto();
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
        return resolved.max_of(resolve_intrinsic_block(
            doc, node_id, &style.min_height, intrinsic_block_size, border_padding_block,
        ));
    } else if style.min_height.is_stretch() {
        let avail = space.available_block_size;
        if !avail.is_indefinite() {
            let margin_block = resolve_margin_or_padding(&style.margin_top, space.available_inline_size)
                + resolve_margin_or_padding(&style.margin_bottom, space.available_inline_size);
            (avail - margin_block).clamp_negative_to_zero()
        } else { LayoutUnit::zero() }
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
        let max_bb = resolve_intrinsic_block(
            doc, node_id, &style.max_height, intrinsic_block_size, border_padding_block,
        );
        return resolved.min_of(max_bb).max_of(min);
    } else if style.max_height.is_stretch() {
        let avail = space.available_block_size;
        if !avail.is_indefinite() {
            let margin_block = resolve_margin_or_padding(&style.margin_top, space.available_inline_size)
                + resolve_margin_or_padding(&style.margin_bottom, space.available_inline_size);
            (avail - margin_block).clamp_negative_to_zero()
        } else { LayoutUnit::max() }
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
    use crate::multicol::{resolve_column_count_and_width, compute_column_positions,
                          balance_columns, balance_columns_with_margins};
    use openui_style::{ColumnFill, ColumnSpan};

    let resolved = resolve_column_count_and_width(
        if algo.column_count > 0 { Some(algo.column_count) } else { None },
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
        resolved.count, column_width, algo.column_gap,
        child_available_inline, false,
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
        /// Height allocated to this portion from the container's budget.
        portion_height: LayoutUnit,
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
            oof_children.push(OofChild { node_id: child_id, flow_index: flow_count });
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

        // Check for nested spanner extraction: if this non-spanner child
        // is a normal block container (not BFC-triggering), check whether
        // any of its immediate children have column-span: all.
        let has_nested_spanners = !child_style.overflow_x.is_scrollable()
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
                        child_style.effective_border_top()
                            + child_style.effective_border_bottom())
                        + LayoutUnit::from_f32(child_style.padding_top.value())
                        + LayoutUnit::from_f32(child_style.padding_bottom.value());
                    (raw - bp).clamp_negative_to_zero()
                } else {
                    raw
                }
            } else {
                // Auto height: use sum of children as budget
                LayoutUnit::from_raw(i32::MAX / 2)
            };

            let mut remaining_height = container_height;
            let mut current_group: Vec<NodeId> = Vec::new();
            // Include container's padding-top in the first group's content.
            // When a container with padding is split at spanner boundaries,
            // the padding contributes to the column content before/after the spanner.
            let container_pad_top = resolve_margin_or_padding(&child_style.padding_top, child_available_inline)
                + LayoutUnit::from_i32(child_style.effective_border_top());
            let container_pad_bottom = resolve_margin_or_padding(&child_style.padding_bottom, child_available_inline)
                + LayoutUnit::from_i32(child_style.effective_border_bottom());
            let mut current_group_content = container_pad_top;
            let mut is_first_group = true;

            for gc_id in doc.children(child_id) {
                let gc_style = &doc.node(gc_id).style;
                if gc_style.display == Display::None {
                    continue;
                }
                if gc_style.position.is_absolutely_positioned() {
                    continue;
                }
                if gc_style.column_span == ColumnSpan::All {
                    // Flush the current group as a split portion.
                    // Also flush if group is empty but has padding content
                    // (container padding-top contributes to pre-spanner space).
                    if !current_group.is_empty() || current_group_content > LayoutUnit::zero() {
                        let portion_h = current_group_content.min_of(remaining_height);
                        remaining_height = (remaining_height - portion_h).clamp_negative_to_zero();
                        children_info.push(ChildInfo {
                            id: child_id,
                            is_spanner: false,
                            split_portion: Some(SplitPortion {
                                container_id: child_id,
                                child_ids: std::mem::take(&mut current_group),
                                portion_height: portion_h,
                            }),
                        });
                        current_group_content = LayoutUnit::zero();
                        is_first_group = false;
                    }
                    // Add the nested spanner as a multicol-level spanner.
                    children_info.push(ChildInfo {
                        id: gc_id,
                        is_spanner: true,
                        split_portion: None,
                    });
                    // Spanner height doesn't consume the container's budget
                    // because it's extracted from the container.
                } else {
                    // Estimate child height for budget allocation.
                    let gc_h = if !gc_style.height.is_auto() {
                        let raw = resolve_length(
                            &gc_style.height,
                            container_height,
                            LayoutUnit::zero(),
                            LayoutUnit::zero(),
                        );
                        raw
                    } else {
                        // Auto height: lay out to determine
                        let gc_is_new_fc = establishes_new_fc(&doc.node(gc_id).style);
                        let gc_space = ConstraintSpace::for_block_child(
                            child_available_inline,
                            remaining_height,
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
            // Flush any trailing group (add container's padding-bottom).
            if !current_group.is_empty() || current_group_content > LayoutUnit::zero() {
                let final_content = current_group_content + container_pad_bottom;
                let portion_h = final_content.min_of(remaining_height);
                children_info.push(ChildInfo {
                    id: child_id,
                    is_spanner: false,
                    split_portion: Some(SplitPortion {
                        container_id: child_id,
                        child_ids: current_group,
                        portion_height: portion_h,
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
        if content_h.raw() > 0 { Some(content_h) } else { None }
    } else {
        None
    };

    // Process children in groups separated by spanners.
    let mut group_start = 0;
    // Track OOF static positions: for each OOF child, record the column
    // position where it would have appeared in normal flow.
    let mut oof_static_positions: Vec<(NodeId, PhysicalOffset)> = Vec::new();
    let mut global_flow_idx: usize = 0;
    while group_start < children_info.len() {
        // Collect the next group of columnar children (until a spanner or end).
        let mut group_end = group_start;
        while group_end < children_info.len() && !children_info[group_end].is_spanner {
            group_end += 1;
        }

        // Lay out columnar group.
        if group_end > group_start {
            // Determine the available block size for this column group.
            // For explicit-height containers, use remaining space after previous
            // groups/spanners. For auto-height, use max-height or indefinite.
            let group_available_block = if has_explicit_height {
                remaining_available_block
            } else if let Some(mh) = resolved_max_height {
                // column-fill:auto with max-height: use remaining of max-height
                if remaining_available_block.is_indefinite() {
                    mh
                } else {
                    remaining_available_block
                }
            } else {
                openui_geometry::INDEFINITE_SIZE
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
                    let container_style = &doc.node(split.container_id).style;
                    let c_border_top = LayoutUnit::from_i32(container_style.effective_border_top());
                    let c_border_bottom = LayoutUnit::from_i32(container_style.effective_border_bottom());
                    let c_pad_top = resolve_margin_or_padding(&container_style.padding_top, column_width);
                    let c_pad_bottom = resolve_margin_or_padding(&container_style.padding_bottom, column_width);
                    let c_border_left = LayoutUnit::from_i32(container_style.effective_border_left());
                    let c_border_right = LayoutUnit::from_i32(container_style.effective_border_right());
                    let c_pad_left = resolve_margin_or_padding(&container_style.padding_left, column_width);
                    let c_pad_right = resolve_margin_or_padding(&container_style.padding_right, column_width);
                    let inner_width = (column_width - c_border_left - c_border_right
                        - c_pad_left - c_pad_right).clamp_negative_to_zero();

                    let mut wrapper_children: Vec<Fragment> = Vec::new();
                    let mut block_off = c_border_top + c_pad_top;
                    for &gc_id in &split.child_ids {
                        let gc_is_new_fc = establishes_new_fc(&doc.node(gc_id).style);
                        let gc_space = ConstraintSpace::for_block_child(
                            inner_width,
                            split.portion_height,
                            inner_width,
                            split.portion_height,
                            gc_is_new_fc,
                        );
                        let mut gc_frag = block_layout(doc, gc_id, &gc_space);
                        gc_frag.offset = PhysicalOffset::new(
                            c_border_left + c_pad_left, block_off);
                        block_off = block_off + gc_frag.size.height;
                        wrapper_children.push(gc_frag);
                    }
                    let content_h = block_off - c_border_top - c_pad_top;
                    let wrapper_h = split.portion_height.min_of(content_h)
                        + c_border_top + c_pad_top + c_border_bottom + c_pad_bottom;
                    let mut wrapper = Fragment::new_box(
                        split.container_id,
                        PhysicalSize::new(column_width, wrapper_h),
                    );
                    wrapper.children = wrapper_children;
                    wrapper.border = BoxStrut {
                        top: c_border_top, right: c_border_right,
                        bottom: c_border_bottom, left: c_border_left,
                    };
                    wrapper.padding = BoxStrut {
                        top: c_pad_top, right: c_pad_right,
                        bottom: c_pad_bottom, left: c_pad_left,
                    };

                    let child_margin_top = resolve_margin_or_padding(
                        &container_style.margin_top, column_width);
                    let child_margin_bottom = resolve_margin_or_padding(
                        &container_style.margin_bottom, column_width);

                    col_block_sizes.push(wrapper.size.height);
                    col_margins_top.push(child_margin_top);
                    col_margins_bottom.push(child_margin_bottom);
                    col_avoid_break.push(container_style.break_inside.is_avoid());
                    col_avoid_break_after.push(propagated_break_after(doc, child_node_id).is_avoid()
                        || container_style.break_after.is_avoid());
                    col_forced_break_before.push(forced);
                    col_fragments.push(wrapper);

                    let prop_ba = propagated_break_after(doc, child_node_id);
                    prev_break_after_forced_fp = prop_ba.is_forced();
                } else {
                    let child_style = &doc.node(info.id).style;
                    let child_margin_top = resolve_margin_or_padding(
                        &child_style.margin_top, column_width);
                    let child_margin_bottom = resolve_margin_or_padding(
                        &child_style.margin_bottom, column_width);
                    let child_is_new_fc = establishes_new_fc(child_style);
                    let mut child_space = ConstraintSpace::for_block_child(
                        column_width,
                        group_available_block,
                        column_width,
                        first_pass_pct_basis,
                        child_is_new_fc,
                    );
                    // Tell children about the fragmentainer (column) height so
                    // flex containers know the wrapping boundary.
                    if !group_available_block.is_indefinite() {
                        child_space.fragmentainer_block_size = group_available_block;
                    }
                    let child_frag = block_layout(doc, info.id, &child_space);
                    col_block_sizes.push(child_frag.size.height);
                    col_margins_top.push(child_margin_top);
                    col_margins_bottom.push(child_margin_bottom);
                    col_avoid_break.push(doc.node(info.id).style.break_inside.is_avoid());
                    col_avoid_break_after.push(propagated_break_after(doc, child_node_id).is_avoid()
                        || doc.node(info.id).style.break_after.is_avoid());
                    col_forced_break_before.push(forced);
                    col_fragments.push(child_frag);

                    let prop_ba = propagated_break_after(doc, child_node_id);
                    prev_break_after_forced_fp = prop_ba.is_forced();
                }
            }

            // Compute effective block sizes including collapsed margins for balancing.
            // Margins between siblings collapse (CSS 2.1 §8.3.1), so we take max(prev_bottom, cur_top).
            let compute_effective = |sizes: &[LayoutUnit], m_top: &[LayoutUnit], m_bot: &[LayoutUnit]| -> Vec<LayoutUnit> {
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

            let mut _effective_sizes = compute_effective(&col_block_sizes, &col_margins_top, &col_margins_bottom);

            // Determine column height for this group.
            let group_max = if !group_available_block.is_indefinite() {
                group_available_block
            } else {
                space.available_block_size
            };

            // CSS Multicol §7.2: Content preceding a column-span:all element
            // is always balanced, even when column-fill is auto.
            let has_spanner_after = group_end < children_info.len()
                && children_info[group_end].is_spanner;

            let column_height = if has_spanner_after
                && algo.column_fill == ColumnFill::Auto
            {
                // Force balance before spanner per §7.2, regardless of
                // whether the container has a definite height/max-height.
                balance_columns_with_margins(&col_block_sizes, &col_margins_top, &col_margins_bottom, &col_avoid_break, resolved.count, group_max, &col_forced_break_before, &col_avoid_break_after)
            } else {
                match algo.column_fill {
                    ColumnFill::Balance | ColumnFill::BalanceAll => {
                        balance_columns_with_margins(&col_block_sizes, &col_margins_top, &col_margins_bottom, &col_avoid_break, resolved.count, group_max, &col_forced_break_before, &col_avoid_break_after)
                    }
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
                            balance_columns_with_margins(&col_block_sizes, &col_margins_top, &col_margins_bottom, &col_avoid_break, resolved.count, group_max, &col_forced_break_before, &col_avoid_break_after)
                        }
                    }
                }
            };

            // Cap column_height at the group's available block size so that
            // a group never claims more vertical space than the container
            // allocated to it.  balance_columns() can return a value
            // exceeding group_available_block when content overflows.
            // CSS Multicol §3: even when remaining space is zero, columns
            // must be at least 1px to allow overflow content to be visible.
            let column_height = if !group_available_block.is_indefinite() {
                column_height.min_of(group_available_block)
                    .max_of(LayoutUnit::from_i32(1))
            } else {
                column_height
            };

            // Second pass: if column_height differs from first_pass_pct_basis
            // and any child has a percentage-based height, re-lay out those
            // children so available_block_size matches the actual column height.
            let needs_relayout = column_height.raw() != first_pass_pct_basis.raw()
                && !column_height.is_indefinite()
                && children_info[group_start..group_end].iter().any(|info| {
                    if info.split_portion.is_some() { return false; }
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
                        let c_border_top = LayoutUnit::from_i32(container_style.effective_border_top());
                        let c_border_bottom = LayoutUnit::from_i32(container_style.effective_border_bottom());
                        let c_pad_top = resolve_margin_or_padding(&container_style.padding_top, column_width);
                        let c_pad_bottom = resolve_margin_or_padding(&container_style.padding_bottom, column_width);
                        let c_border_left = LayoutUnit::from_i32(container_style.effective_border_left());
                        let c_border_right = LayoutUnit::from_i32(container_style.effective_border_right());
                        let c_pad_left = resolve_margin_or_padding(&container_style.padding_left, column_width);
                        let c_pad_right = resolve_margin_or_padding(&container_style.padding_right, column_width);
                        let inner_width = (column_width - c_border_left - c_border_right
                            - c_pad_left - c_pad_right).clamp_negative_to_zero();

                        let mut wrapper_children: Vec<Fragment> = Vec::new();
                        let mut block_off = c_border_top + c_pad_top;
                        for &gc_id in &split.child_ids {
                            let gc_is_new_fc = establishes_new_fc(&doc.node(gc_id).style);
                            let gc_space = ConstraintSpace::for_block_child(
                                inner_width, split.portion_height,
                                inner_width, split.portion_height, gc_is_new_fc,
                            );
                            let mut gc_frag = block_layout(doc, gc_id, &gc_space);
                            gc_frag.offset = PhysicalOffset::new(
                                c_border_left + c_pad_left, block_off);
                            block_off = block_off + gc_frag.size.height;
                            wrapper_children.push(gc_frag);
                        }
                        let content_h = block_off - c_border_top - c_pad_top;
                        let wrapper_h = split.portion_height.min_of(content_h)
                            + c_border_top + c_pad_top + c_border_bottom + c_pad_bottom;
                        let mut wrapper = Fragment::new_box(
                            split.container_id,
                            PhysicalSize::new(column_width, wrapper_h),
                        );
                        wrapper.children = wrapper_children;
                        wrapper.border = BoxStrut {
                            top: c_border_top, right: c_border_right,
                            bottom: c_border_bottom, left: c_border_left,
                        };
                        wrapper.padding = BoxStrut {
                            top: c_pad_top, right: c_pad_right,
                            bottom: c_pad_bottom, left: c_pad_left,
                        };

                        let child_margin_top = resolve_margin_or_padding(
                            &container_style.margin_top, column_width);
                        let child_margin_bottom = resolve_margin_or_padding(
                            &container_style.margin_bottom, column_width);

                        col_block_sizes.push(wrapper.size.height);
                        col_margins_top.push(child_margin_top);
                        col_margins_bottom.push(child_margin_bottom);
                        col_avoid_break.push(container_style.break_inside.is_avoid());
                        col_avoid_break_after.push(propagated_break_after(doc, child_node_id).is_avoid()
                            || container_style.break_after.is_avoid());
                        col_forced_break_before.push(forced);
                        col_fragments.push(wrapper);

                        let prop_ba = propagated_break_after(doc, child_node_id);
                        prev_break_after_forced_rp = prop_ba.is_forced();
                    } else {
                        let child_style = &doc.node(info.id).style;
                        let child_margin_top = resolve_margin_or_padding(
                            &child_style.margin_top, column_width);
                        let child_margin_bottom = resolve_margin_or_padding(
                            &child_style.margin_bottom, column_width);
                        let child_is_new_fc = establishes_new_fc(child_style);
                        let mut child_space = ConstraintSpace::for_block_child(
                            column_width,
                            column_height,
                            column_width,
                            child_percentage_block_size,
                            child_is_new_fc,
                        );
                        if !column_height.is_indefinite() {
                            child_space.fragmentainer_block_size = column_height;
                        }
                        let child_frag = block_layout(doc, info.id, &child_space);
                        col_block_sizes.push(child_frag.size.height);
                        col_margins_top.push(child_margin_top);
                        col_margins_bottom.push(child_margin_bottom);
                        col_avoid_break.push(doc.node(info.id).style.break_inside.is_avoid());
                        col_avoid_break_after.push(propagated_break_after(doc, child_node_id).is_avoid()
                            || doc.node(info.id).style.break_after.is_avoid());
                        col_forced_break_before.push(forced);
                        col_fragments.push(child_frag);

                        let prop_ba = propagated_break_after(doc, child_node_id);
                        prev_break_after_forced_rp = prop_ba.is_forced();
                    }
                }

                _effective_sizes = compute_effective(&col_block_sizes, &col_margins_top, &col_margins_bottom);
            }

            // Distribute children across columns (with fragmentation support).
            // CSS Multicol §3.4: overflow content creates additional columns
            // in the inline direction. col_idx may exceed positions.len().
            let col_inline_offset_for = |idx: usize| -> LayoutUnit {
                if idx < positions.len() {
                    positions[idx].inline_offset
                } else if let Some(last) = positions.last() {
                    // Overflow column: extend rightward from last defined column
                    let overflow = (idx - positions.len()) as i32;
                    let stride = column_width + algo.column_gap;
                    last.inline_offset + last.width + algo.column_gap
                        + stride * LayoutUnit::from_i32(overflow)
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
            // Track whether the current column was started by a forced break.
            // CSS Fragmentation: margins at the top of a non-first column are
            // preserved after forced breaks but truncated after unforced breaks.
            let mut col_started_by_forced_break = false;
            // Track actual tallest column content for auto-height containers.
            let mut max_col_content = LayoutUnit::zero();

            // Record static positions for OOF children that appear before
            // the first in-flow child in this group.
            for oof in &oof_children {
                if oof.flow_index == global_flow_idx {
                    oof_static_positions.push((oof.node_id, PhysicalOffset::new(
                        content_edge_x + col_inline_offset_for(0),
                        content_edge_y + total_block_offset,
                    )));
                }
            }

            for (i, child_frag) in col_fragments.into_iter().enumerate() {
                let child_height = col_block_sizes[i];
                let child_margin_top = col_margins_top[i];
                let child_margin_bottom = col_margins_bottom[i];
                let child_node_id = children_info[group_start + i].id;
                let child_style = &doc.node(child_node_id).style;

                // CSS Fragmentation §3.1: Forced breaks —
                // break-before: column/page/always forces a break before this child.
                // break-after on the *previous* child forces a break before this one.
                // §3.1 also says break values propagate from first/last in-flow children.
                let prop_break_before = propagated_break_before(doc, child_node_id);
                let forced_break = prop_break_before.is_forced()
                    || prev_break_after_forces;

                if forced_break {
                    // CSS Multicol §3.4: Forced breaks always create a new
                    // column, even if that means creating an overflow column
                    // beyond the declared column-count.  Overflow columns are
                    // positioned by col_inline_offset_for() which already
                    // handles indices ≥ positions.len().
                    if col_block_offset > LayoutUnit::zero() {
                        max_col_content = max_col_content.max_of(col_block_offset);
                        col_idx += 1;
                        col_block_offset = LayoutUnit::zero();
                        col_remaining = column_height;
                        prev_margin_bottom = LayoutUnit::zero();
                        col_started_by_forced_break = true;
                    } else if i > 0 {
                        // Column is empty but this isn't the first child overall.
                        // A forced break still moves to the next column.
                        col_idx += 1;
                        col_block_offset = LayoutUnit::zero();
                        col_remaining = column_height;
                        prev_margin_bottom = LayoutUnit::zero();
                        col_started_by_forced_break = true;
                    }
                }

                // Compute collapsed margin between siblings.
                // CSS 2.1 §8.3.1: adjoining margins collapse to max.
                // CSS Fragmentation §5.4: margin at top of non-first column
                // truncated for unforced breaks, preserved for forced breaks.
                let margin_space = if col_block_offset > LayoutUnit::zero() {
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
                let avoid_break_inside = child_style.break_inside.is_avoid();
                // CSS Fragmentation §3.1: break-before/after: avoid —
                // This child or the previous child wants to avoid a break here.
                let avoid_break_before = prop_break_before.is_avoid()
                    || child_style.break_before.is_avoid()
                    || prev_break_after_avoids;
                if avoid_break_inside
                    && col_remaining.raw() < total_child_space.raw()
                    && col_block_offset > LayoutUnit::zero()
                    && child_height.raw() <= column_height.raw()
                    && (algo.column_fill != ColumnFill::Auto
                        || col_idx + 1 < resolved.count as usize)
                {
                    max_col_content = max_col_content.max_of(col_block_offset);
                    col_idx += 1;
                    col_block_offset = LayoutUnit::zero();
                    col_remaining = column_height;
                    prev_margin_bottom = LayoutUnit::zero();
                    col_started_by_forced_break = false;
                }

                // Recalculate margin for potentially new column context.
                // CSS Fragmentation §5.4: truncate at non-first column top
                // only for unforced breaks.
                let actual_margin = if col_block_offset > LayoutUnit::zero() {
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
                let can_advance_col = algo.column_fill != ColumnFill::Auto
                    || col_idx + 1 < resolved.count as usize;
                if col_remaining.raw() < needed.raw() && col_block_offset > LayoutUnit::zero()
                    && !avoid_break_before && can_advance_col
                {
                    max_col_content = max_col_content.max_of(col_block_offset);
                    col_idx += 1;
                    col_block_offset = LayoutUnit::zero();
                    col_remaining = column_height;
                    prev_margin_bottom = LayoutUnit::zero();
                    col_started_by_forced_break = false;
                }

                let prop_break_after = propagated_break_after(doc, child_node_id);
                prev_break_after_forces = prop_break_after.is_forced();
                prev_break_after_avoids = prop_break_after.is_avoid()
                    || child_style.break_after.is_avoid();

                // Final margin for positioning.
                // CSS Fragmentation §5.4: margins at the top of a non-first
                // column are truncated for UNFORCED breaks but preserved for
                // FORCED breaks (break-before/break-after: column).
                let pos_margin = if col_block_offset > LayoutUnit::zero() {
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

                if col_remaining.raw() >= effective_needed.raw() {
                    // Child fits in current column.
                    let col_inline_offset = col_inline_offset_for(col_idx);

                    col_block_offset = col_block_offset + pos_margin;

                    let mut positioned = child_frag;
                    positioned.offset = PhysicalOffset::new(
                        content_edge_x + col_inline_offset,
                        content_edge_y + total_block_offset + col_block_offset,
                    );
                    // Apply relative positioning (CSS 2.1 §9.4.3).
                    crate::relative::apply_relative_offset(
                        &mut positioned,
                        child_style,
                        column_width,
                        column_height,
                    );
                    col_block_offset = col_block_offset + child_height;
                    col_remaining = col_remaining - pos_margin - child_height;
                    prev_margin_bottom = child_margin_bottom;
                    result_children.push(positioned);
                } else {
                    // Child must be fragmented across multiple columns.
                    // CSS Multicol §3.4: overflow creates additional columns
                    // in the inline direction (col_idx may exceed positions.len()).
                    col_block_offset = col_block_offset + pos_margin;
                    col_remaining = col_remaining - pos_margin;
                    let mut consumed = LayoutUnit::zero();
                    // Guard: if column_height is zero or negative, place
                    // everything in the current column to avoid infinite loop.
                    if column_height.raw() <= 0 {
                        let mut part = child_frag.clone();
                        part.size.height = child_height;
                        part.has_overflow_clip = true;
                        part.offset = PhysicalOffset::new(
                            content_edge_x + col_inline_offset_for(col_idx),
                            content_edge_y + total_block_offset + col_block_offset,
                        );
                        result_children.push(part);
                        col_block_offset = col_block_offset + child_height;
                    } else {
                        while consumed.raw() < child_height.raw() {
                            let col_inline_offset = col_inline_offset_for(col_idx);

                            let avail = if col_block_offset > LayoutUnit::zero() {
                                col_remaining
                            } else {
                                column_height
                            };
                            let remaining_child = child_height - consumed;
                            let part_height = avail.min_of(remaining_child);

                            // Safety: if part_height is zero, break to avoid
                            // infinite loop (degenerate column height).
                            if part_height.raw() <= 0 {
                                break;
                            }

                            let mut part = child_frag.clone();
                            part.size.height = part_height;
                            part.has_overflow_clip = true;
                            part.offset = PhysicalOffset::new(
                                content_edge_x + col_inline_offset,
                                content_edge_y + total_block_offset + col_block_offset,
                            );
                            // Apply relative positioning (CSS 2.1 §9.4.3).
                            crate::relative::apply_relative_offset(
                                &mut part,
                                child_style,
                                column_width,
                                column_height,
                            );
                            // Shift child content up by the amount already consumed
                            if consumed > LayoutUnit::zero() {
                                for c in &mut part.children {
                                    c.offset.top = c.offset.top - consumed;
                                }
                            }
                            result_children.push(part);

                            consumed = consumed + part_height;
                            col_block_offset = col_block_offset + part_height;
                            col_remaining = col_remaining - part_height;

                            // Move to next column if this one is full and there's
                            // more content. Overflow columns are created as needed,
                            // but column-fill:auto caps at the resolved column count.
                            if consumed.raw() < child_height.raw() {
                                let can_advance = algo.column_fill != ColumnFill::Auto
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
                    }
                    prev_margin_bottom = child_margin_bottom;
                }

                // After placing this child, record static positions for
                // any OOF children that appear after it in DOM order.
                global_flow_idx += 1;
                for oof in &oof_children {
                    if oof.flow_index == global_flow_idx {
                        oof_static_positions.push((oof.node_id, PhysicalOffset::new(
                            content_edge_x + col_inline_offset_for(col_idx),
                            content_edge_y + total_block_offset + col_block_offset,
                        )));
                    }
                }
            }

            // Finalize: include the last column's content height.
            max_col_content = max_col_content.max_of(col_block_offset);
            // For auto-height containers, use the balanced column_height (which
            // IS the desired visual height) for balanced columns. For
            // column-fill:auto without a spanner, use actual content height.
            // §7.2: before a spanner, content is always balanced, so use
            // the balanced column_height even with column-fill:auto.
            let actual_group_height = if style.height.is_auto() {
                if algo.column_fill == ColumnFill::Auto && !has_spanner_after {
                    max_col_content
                } else {
                    column_height
                }
            } else {
                column_height
            };

            // Column rules for this group — derive positions from actual
            // per-column positions so that rules stay centered in the gap
            // even when column widths vary due to cumulative rounding.
            if let Some(ref rule) = algo.column_rule {
                let half_gap = LayoutUnit::from_raw(algo.column_gap.raw() / 2);
                for i in 0..positions.len().saturating_sub(1) {
                    let rp = positions[i].inline_offset + positions[i].width + half_gap;
                    let mut rule_frag = Fragment::new_box(node_id,
                        PhysicalSize::new(rule.width, actual_group_height));
                    rule_frag.offset = PhysicalOffset::new(
                        content_edge_x + rp - rule.width / LayoutUnit::from_i32(2),
                        content_edge_y + total_block_offset,
                    );
                    rule_frag.kind = FragmentKind::ColumnRule;
                    result_children.push(rule_frag);
                }
            }

            total_block_offset = total_block_offset + actual_group_height;

            // Update remaining available block for subsequent groups.
            if !remaining_available_block.is_indefinite() {
                remaining_available_block = (remaining_available_block - actual_group_height).clamp_negative_to_zero();
            }
        }

        // Handle spanner (if current item is one).
        if group_end < children_info.len() && children_info[group_end].is_spanner {
            let spanner_id = children_info[group_end].id;
            let spanner_style = &doc.node(spanner_id).style;
            let spanner_margin_top = resolve_margin_or_padding(
                &spanner_style.margin_top, child_available_inline);
            let spanner_margin_bottom = resolve_margin_or_padding(
                &spanner_style.margin_bottom, child_available_inline);
            let spanner_space = ConstraintSpace::for_block_child(
                child_available_inline,
                space.available_block_size,
                child_available_inline,
                child_percentage_block_size,
                false,
            );
            let mut spanner_frag = block_layout(doc, spanner_id, &spanner_space);
            total_block_offset = total_block_offset + spanner_margin_top;
            spanner_frag.offset = PhysicalOffset::new(
                content_edge_x,
                content_edge_y + total_block_offset,
            );
            let spanner_total = spanner_margin_top + spanner_frag.size.height + spanner_margin_bottom;
            total_block_offset = total_block_offset + spanner_frag.size.height + spanner_margin_bottom;
            result_children.push(spanner_frag);

            // Update remaining available block after spanner.
            if !remaining_available_block.is_indefinite() {
                remaining_available_block = (remaining_available_block - spanner_total).clamp_negative_to_zero();
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
    let container_block_size = explicit_block_size
        .unwrap_or(total_block_offset + border_padding_block);

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
            let min_border = if style.box_sizing == BoxSizing::ContentBox && min_raw > LayoutUnit::zero() {
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

    // Layout out-of-flow children (absolute/fixed positioned).
    // These are positioned relative to the multicol container's padding box.
    // CSS 2.1 §10.1: The containing block for abspos is the padding edge of
    // the nearest positioned ancestor.
    if !oof_children.is_empty() {
        let cb_height = container_block_size - border.top - border.bottom;
        let cb_width = child_available_inline + padding.left + padding.right;
        let mut oof_candidates: Vec<crate::out_of_flow::OutOfFlowCandidate> = Vec::new();
        for (oof_node_id, static_pos) in &oof_static_positions {
            let child_style = doc.node(*oof_node_id).style.clone();
            oof_candidates.push(crate::out_of_flow::OutOfFlowCandidate {
                node_id: *oof_node_id,
                style: child_style,
                static_position: *static_pos,
                containing_block_size: PhysicalSize::new(cb_width, cb_height),
                containing_block_border: border.clone(),
                containing_block_direction: style.direction,
                static_position_direction: style.direction,
            });
        }
        // Also handle OOF children that were not matched during distribution
        // (e.g., the only child is OOF, or OOF appears after all in-flow children
        // that were in a different group due to spanners).
        for oof in &oof_children {
            if !oof_static_positions.iter().any(|(id, _)| *id == oof.node_id) {
                let child_style = doc.node(oof.node_id).style.clone();
                oof_candidates.push(crate::out_of_flow::OutOfFlowCandidate {
                    node_id: oof.node_id,
                    style: child_style,
                    static_position: PhysicalOffset::new(content_edge_x, content_edge_y),
                    containing_block_size: PhysicalSize::new(cb_width, cb_height),
                    containing_block_border: border.clone(),
                    containing_block_direction: style.direction,
                    static_position_direction: style.direction,
                });
            }
        }
        let oof_fragments = crate::out_of_flow::layout_out_of_flow_children(doc, &oof_candidates);
        for frag in oof_fragments {
            result_children.push(frag);
        }
    }

    let mut container = Fragment::new_box(node_id,
        PhysicalSize::new(border_box_inline, container_block_size));
    container.children = result_children;
    container
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::Length;
    use openui_style::*;
    use openui_dom::ElementTag;

    #[test]
    fn single_div_fills_width() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let outer_frag = &fragment.children[0];

        // Outer should be 800px wide (fills parent)
        assert_eq!(outer_frag.size.width.to_i32(), 800);
        // Outer height = 2 (border-top) + 10 (padding-top) + 50 (child) + 10 (padding-bottom) + 2 (border-bottom) = 74
        assert_eq!(outer_frag.size.height.to_i32(), 74);

        // Inner child should be offset by border+padding
        let inner_frag = &outer_frag.children[0];
        assert_eq!(inner_frag.offset.left.to_i32(), 22); // 2 (border) + 20 (padding)
        assert_eq!(inner_frag.offset.top.to_i32(), 12);  // 2 (border) + 10 (padding)

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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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
        let has_30px_child = container_frag.children.iter().any(|f| f.size.height.to_i32() == 30);
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let container_frag = &fragment.children[0];

        // Should produce 2 fragments: 1 anonymous inline wrapper (one IFC
        // for both text nodes) + 1 block child. Before the fix, the
        // display:none span split it into 3 fragments.
        assert_eq!(
            container_frag.children.len(), 2,
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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
            fragment.children.len(), 2,
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];
        let child_frag = &parent_frag.children[0];

        // With auto-height parent, 50% height should resolve to 0 (indefinite).
        // Before the fix, it resolved to 50% of 600 = 300.
        assert_eq!(
            child_frag.size.height.to_i32(), 0,
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

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
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
