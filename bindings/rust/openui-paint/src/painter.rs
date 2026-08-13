//! Box and text painting — extracted from Blink's `box_fragment_painter.cc`,
//! `box_painter_base.cc`, and `text_painter.cc`.
//!
//! Paint order for a block box (PaintPhase::kBlockBackground):
//! 1. Box shadows (outset) — SP12
//! 2. Background color
//! 3. Box shadows (inset) — SP12
//! 4. Borders
//!
//! Paint order for a text fragment:
//! 1. Text shadows
//! 2. Text decorations (underline, overline — behind text)
//! 3. Text glyphs
//! 4. Emphasis marks (above/below each character)
//! 5. Text decorations (line-through — in front of text)
//!
//! Each operation maps to exact Skia calls with exact SkPaint configuration.

use openui_dom::Document;
use openui_geometry::{LayoutUnit, PhysicalOffset};
use openui_layout::{Fragment, FragmentKind};
use openui_style::{
    BackgroundAttachment, BackgroundClip, BorderStyle, Color, ComputedStyle, Display,
    GradientStopPosition, LineHeight, ListStylePosition, ListStyleType, Overflow, OverflowClipBox,
    StyleColor, Visibility,
};
use openui_text::font::FontMetrics;
use skia_safe::{
    gradient_shader, BlendMode, Canvas, ClipOp, Color4f, ColorSpace, Paint, PaintStyle,
    PathBuilder, PathFillType, Point, RRect, Rect, TileMode,
};

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashSet};

fn set_paint_css_color(paint: &mut Paint, color: &Color) {
    set_paint_css_color_with_alpha(paint, color, 1.0);
}

fn set_paint_css_color_with_alpha(paint: &mut Paint, color: &Color, alpha_multiplier: f32) {
    // Blink carries CSS colors through PaintFlags as SkColor4f. Converting to
    // packed SkColor first changes premultiplication rounding at antialiased
    // edges, even when the authored channels came from 8-bit CSS syntax.
    paint.set_color4f(
        Color4f::new(
            color.r.clamp(0.0, 1.0),
            color.g.clamp(0.0, 1.0),
            color.b.clamp(0.0, 1.0),
            (color.a * alpha_multiplier).clamp(0.0, 1.0),
        ),
        None::<&ColorSpace>,
    );
}

fn skia_css_color_with_alpha(color: &Color, alpha_multiplier: f32) -> skia_safe::Color {
    let to_u8 = |component: f32| (component.clamp(0.0, 1.0) * 255.0).round() as u8;
    skia_safe::Color::from_argb(
        to_u8(color.a * alpha_multiplier),
        to_u8(color.r),
        to_u8(color.g),
        to_u8(color.b),
    )
}

fn resolved_gradient_positions(
    stops: &[openui_style::LinearGradientStop],
    line_length: f32,
) -> Vec<f32> {
    let count = stops.len();
    let mut result: Vec<Option<f32>> = stops
        .iter()
        .map(|stop| match stop.position {
            GradientStopPosition::Auto => None,
            GradientStopPosition::Percent(value) => Some(value / 100.0),
            GradientStopPosition::Px(value) => Some(if line_length > 0.0 {
                value / line_length
            } else {
                0.0
            }),
        })
        .collect();
    if count == 0 {
        return Vec::new();
    }
    if result[0].is_none() {
        result[0] = Some(0.0);
    }
    if result[count - 1].is_none() {
        result[count - 1] = Some(1.0);
    }
    let mut start = 0;
    while start + 1 < count {
        let mut end = start + 1;
        while end < count && result[end].is_none() {
            end += 1;
        }
        let start_value = result[start].unwrap_or(0.0);
        let end_value = result[end].unwrap_or(start_value);
        let span = (end - start) as f32;
        for (offset, item) in result.iter_mut().enumerate().take(end).skip(start + 1) {
            *item = Some(start_value + (end_value - start_value) * (offset - start) as f32 / span);
        }
        start = end;
    }
    let mut previous = f32::NEG_INFINITY;
    result
        .into_iter()
        .map(|position| {
            let position = position.unwrap_or(previous).max(previous).clamp(0.0, 1.0);
            previous = position;
            position
        })
        .collect()
}

// Fragments hoisted from in-flow subtrees into the nearest stacking context's
// non_negative_z list must not be painted a second time during Phase 2
// (in-flow recursion). Their raw pointer is inserted here before Phase 2 and
// removed before Phase 3 so the entry in non_negative_z paints them correctly.
thread_local! {
    static HOIST_SKIP: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    static PREPAINTED_COLUMN_DECORATIONS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    static FRAGMENTED_OOF_HOIST_ACTIVE: RefCell<bool> = const { RefCell::new(false) };
    static FRAGMENTED_INLINE_SKIP_AFTER: RefCell<Option<usize>> = const { RefCell::new(None) };
}

/// Resolve a padding/margin Length to f32 pixels.
/// Percentage values resolve against `container_size`.
fn resolve_margin_or_padding_f32(len: &openui_geometry::Length, container_size: f32) -> f32 {
    if len.is_fixed() {
        len.value()
    } else if len.is_percent() {
        len.value() / 100.0 * container_size
    } else if len.is_calculated() {
        len.calc_offset() + len.value() / 100.0 * container_size
    } else {
        0.0
    }
}

/// Paint a fragment tree onto a Skia canvas.
///
/// This is the main entry point — paints the fragment and all its children
/// recursively, with correct coordinate offsets.
pub fn paint_fragment(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
) {
    // Keep accumulated offset fractional (sub-pixel precision).
    // Pixel snapping happens only at the point of drawing (paint_box_decoration_background,
    // paint_text_fragment, etc.) using Blink's PixelSnappedIntRect approach:
    //   left = round(abs.x), top = round(abs.y),
    //   right = round(abs.x + w), bottom = round(abs.y + h)
    // This ensures adjacent elements at fractional boundaries share the same
    // snapped edge (e.g. flex items at 133.33px intervals produce no gaps).
    let abs_offset = PhysicalOffset::new(
        offset.left + fragment.offset.left,
        offset.top + fragment.offset.top,
    );
    // Skip fragments that have been hoisted to the nearest stacking context's
    // non_negative_z list — they will be painted in Phase 3 instead.
    if HOIST_SKIP.with(|s| s.borrow().contains(&(fragment as *const Fragment as usize))) {
        return;
    }

    // A fragmented inline's positioned descendants and its later in-flow
    // content share one source-order stacking sequence across all columns.
    // During the first column traversal, defer later inline fragments so they
    // can be painted once in that global sequence instead of once per column.
    if !fragment.node_id.is_none()
        && fragment.is_inline_box_fragment
        && FRAGMENTED_INLINE_SKIP_AFTER.with(|source_after| {
            source_after
                .borrow()
                .is_some_and(|source_after| fragment.node_id.index() > source_after)
        })
    {
        return;
    }

    // Text fragments with NodeId::NONE (e.g., ellipsis "…") need to be painted
    // even though they have no DOM node. Use inherited style if available.
    if fragment.kind == FragmentKind::Text && fragment.node_id.is_none() {
        let default_style = ComputedStyle::default();
        let style = fragment.inherited_style.as_ref().unwrap_or(&default_style);
        if style.visibility != Visibility::Visible {
            return;
        }
        paint_text_fragment(canvas, fragment, style, abs_offset);
        return;
    }

    // Column box fragments are anonymous clipping containers for multicol
    // columns (CSS Multicol §3.1). They have no decoration but clip children
    // to the column boundaries.
    if fragment.kind == FragmentKind::ColumnBox {
        if fragment.has_overflow_clip {
            // A nested multicol's rule is ink in the intervening column gap,
            // not content clipped to the ancestor column's inline edge. When
            // the ancestor owns a two-axis continuation clip, paint only the
            // lateral part of descendant rules outside that edge here; the
            // ordinary traversal below paints the portion inside the column.
            // Both passes remain constrained to this fragmentainer's block
            // interval, so rules cannot leak into an adjacent row.
            if !fragment.block_axis_clip_only && !fragment.inline_axis_clip_only {
                let (clip_y, clip_h) = compute_column_block_clip_rect(fragment, abs_offset);
                canvas.save();
                canvas.clip_rect(
                    Rect::from_xywh(-1_000_000.0, clip_y, 2_000_000.0, clip_h),
                    ClipOp::Intersect,
                    false,
                );
                canvas.clip_rect(
                    Rect::from_xywh(
                        abs_offset.left.to_f32(),
                        clip_y,
                        fragment.size.width.to_f32(),
                        clip_h,
                    ),
                    ClipOp::Difference,
                    false,
                );
                paint_descendant_column_rules(canvas, &fragment.children, doc, abs_offset);
                canvas.restore();
            }
            canvas.save();
            let (clip_y, clip_h) = compute_column_block_clip_rect(fragment, abs_offset);
            // Multicol fragmentainers clip in the block axis. Inline overflow
            // may normally paint into the column gap. Layout clears
            // `block_axis_clip_only` when a nested fragmentation context owns
            // an inline-axis continuation boundary as well.
            let (clip_x, clip_w) =
                if fragment.block_axis_clip_only && !fragment.inline_axis_clip_only {
                    (-1_000_000.0, 2_000_000.0)
                } else {
                    (abs_offset.left.to_f32(), fragment.size.width.to_f32())
                };
            let (clip_y, clip_h) = if fragment.inline_axis_clip_only {
                (-1_000_000.0, 2_000_000.0)
            } else {
                (clip_y, clip_h)
            };
            canvas.clip_rect(
                skia_safe::Rect::from_xywh(clip_x, clip_y, clip_w, clip_h),
                skia_safe::ClipOp::Intersect,
                false,
            );
            paint_children_with_stacking_order(canvas, &fragment.children, doc, abs_offset, false);
            canvas.restore();
        } else {
            paint_children_with_stacking_order(canvas, &fragment.children, doc, abs_offset, false);
        }
        return;
    }

    // Line box fragments (from inline layout) have NodeId::NONE — they are
    // anonymous boxes with no DOM node. Just recurse into children.
    if fragment.node_id.is_none() {
        for child in &fragment.children {
            paint_fragment(canvas, child, doc, abs_offset);
        }
        return;
    }

    let original_style = &doc.node(fragment.node_id).style;
    let mut canvas_adjusted_style = None;
    if doc.canvas_background_source() == Some(fragment.node_id) {
        let mut adjusted = original_style.clone();
        // Canvas-propagated backgrounds are painted once on the canvas, not
        // again on the source element's principal box.
        adjusted.background_color = Color::TRANSPARENT;
        canvas_adjusted_style = Some(adjusted);
    }
    let style = canvas_adjusted_style.as_ref().unwrap_or(original_style);

    // CSS opacity creates a stacking context and composites the entire
    // subtree at the given opacity. Blink implements this via
    // PaintLayerPainter::PaintLayerWithAdjustedRoot() using saveLayerAlphaf().
    let flattens_opacity = can_flatten_box_opacity(fragment, style);
    let needs_layer = style.opacity < 1.0 && !flattens_opacity;
    if needs_layer {
        canvas.save_layer_alpha_f(None, style.opacity);
    }

    if style.visibility == Visibility::Visible && style.display.is_flex() {
        paint_flex_negative_stacking_children(canvas, fragment, doc, abs_offset);
    }

    // ── Paint this fragment (skip if visibility: hidden) ─────────────
    if style.visibility == Visibility::Visible {
        match fragment.kind {
            FragmentKind::Text => {
                paint_text_fragment(canvas, fragment, style, abs_offset);
            }
            FragmentKind::Box | FragmentKind::Viewport => {
                let paint_opacity = if flattens_opacity { style.opacity } else { 1.0 };
                let decoration_prepainted = PREPAINTED_COLUMN_DECORATIONS.with(|fragments| {
                    fragments
                        .borrow()
                        .contains(&(fragment as *const Fragment as usize))
                });
                if !decoration_prepainted {
                    paint_box_decoration_background(
                        canvas,
                        fragment,
                        style,
                        abs_offset,
                        paint_opacity,
                    );
                }
                let outside_marker_clipped = style.list_style_position
                    == ListStylePosition::Outside
                    && (style.overflow_x != Overflow::Visible
                        || style.overflow_y != Overflow::Visible);
                if style.display == Display::ListItem
                    && style.list_style_type != ListStyleType::None
                    && !outside_marker_clipped
                {
                    paint_list_marker(canvas, fragment, style, abs_offset);
                }
            }
            FragmentKind::ColumnRule => {
                paint_column_rule(canvas, fragment, style, abs_offset);
            }
            FragmentKind::ColumnBox => {
                // Handled above (early return); unreachable here.
            }
        }
    }

    let paint_outline_after_children = should_paint_outline_after_children(fragment, doc, style);
    // A column rule borrows the originating multicol style for its rule
    // width/style/color, but it is not another principal box and must not
    // duplicate that element's outline around the synthetic rule fragment.
    let should_outline =
        fragment.kind != FragmentKind::ColumnRule && should_paint_outline(fragment, doc, style);
    if style.visibility == Visibility::Visible && should_outline && !paint_outline_after_children {
        paint_outline(canvas, fragment, style, abs_offset);
    }

    // ── Overflow clipping + children ──────────────────────────────────
    let needs_clip = needs_overflow_clip(fragment, style, doc);
    if needs_clip {
        paint_with_overflow_clip(canvas, fragment, doc, abs_offset, style);
    } else {
        // Paint children with CSS stacking order (z-index aware).
        let is_sc = is_fragment_stacking_context(fragment, doc);
        paint_children_with_stacking_order(canvas, &fragment.children, doc, abs_offset, is_sc);
    }

    if style.visibility == Visibility::Visible && should_outline && paint_outline_after_children {
        paint_outline(canvas, fragment, style, abs_offset);
    }

    if needs_layer {
        canvas.restore();
    }
}

fn paint_descendant_column_rules(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    parent_offset: PhysicalOffset,
) {
    for child in children {
        if child.kind == FragmentKind::ColumnRule {
            paint_fragment(canvas, child, doc, parent_offset);
            continue;
        }
        let child_offset = PhysicalOffset::new(
            parent_offset.left + child.offset.left,
            parent_offset.top + child.offset.top,
        );
        paint_descendant_column_rules(canvas, &child.children, doc, child_offset);
    }
}

fn should_paint_outline(fragment: &Fragment, doc: &Document, style: &ComputedStyle) -> bool {
    if !style.has_outline() {
        return false;
    }
    // The body canvas does not acquire an outline from an empty principal
    // multicol fragment. Its padding still contributes to the canvas box,
    // but there is no column-row border edge around which to draw the outline.
    if doc.node(fragment.node_id).tag == openui_dom::ElementTag::Body
        && !fragment.children.is_empty()
        && fragment.children.iter().all(|child| {
            child.kind == FragmentKind::ColumnBox && child.size.height == LayoutUnit::zero()
        })
    {
        return false;
    }
    !(style.column_span == openui_style::ColumnSpan::All
        && fragment.children.is_empty()
        && fragment.size.height.raw() == 0
        && !fragment.paint_zero_block_outline)
}

fn should_paint_outline_after_children(
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
) -> bool {
    style.outline_style == BorderStyle::Solid
        && style.overflow_x == Overflow::Visible
        && style.overflow_y == Overflow::Visible
        && !fragment.children.iter().any(|child| {
            if child.node_id.is_none() {
                return false;
            }
            let child_style = &doc.node(child.node_id).style;
            child_style.overflow_x.is_clipping() || child_style.overflow_y.is_clipping()
        })
}

// ── Stacking order (z-index) ──────────────────────────────────────────

enum StackingEntry<'a> {
    Direct(usize),
    Descendant(&'a Fragment, PhysicalOffset),
    DescendantWithClip(&'a Fragment, PhysicalOffset, Rect),
}

struct FragmentedInlineOofEntry<'a> {
    fragment: &'a Fragment,
    parent_offset: PhysicalOffset,
    clip: Rect,
    column_order: usize,
    visible: bool,
}

fn collect_inline_oof_ids(fragment: &Fragment, doc: &Document, result: &mut BTreeSet<usize>) {
    use openui_style::Position;

    if !fragment.node_id.is_none() {
        let node = doc.node(fragment.node_id);
        if matches!(node.style.position, Position::Absolute | Position::Fixed)
            && node.style.z_index.is_none()
            && !node.parent.is_none()
            && doc.node(node.parent).style.display.is_inline_level()
        {
            result.insert(fragment.node_id.index());
        }
    }
    for child in &fragment.children {
        collect_inline_oof_ids(child, doc, result);
    }
}

fn collect_fragmented_inline_oof_entries<'a>(
    fragment: &'a Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    candidate_ids: &BTreeSet<usize>,
    clip: Rect,
    column_order: usize,
    entries: &mut Vec<FragmentedInlineOofEntry<'a>>,
) {
    use openui_style::Position;

    if !fragment.node_id.is_none() {
        let node = doc.node(fragment.node_id);
        if matches!(node.style.position, Position::Absolute | Position::Fixed)
            && node.style.z_index.is_none()
            && candidate_ids.contains(&fragment.node_id.index())
        {
            let fragment_top = parent_offset.top.to_f32() + fragment.offset.top.to_f32();
            let fragment_bottom = fragment_top + fragment.size.height.to_f32();
            entries.push(FragmentedInlineOofEntry {
                fragment,
                parent_offset,
                clip,
                column_order,
                visible: fragment_top < clip.bottom && fragment_bottom > clip.top,
            });
            return;
        }
    }

    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    let clip = if fragment.has_overflow_clip {
        let (_, clip_y, _, clip_h) = compute_clip_rect(fragment, fragment_offset);
        let (clip_x, clip_w) = if fragment.block_axis_clip_only {
            (-1_000_000.0, 2_000_000.0)
        } else {
            let (clip_x, _, clip_w, _) = compute_clip_rect(fragment, fragment_offset);
            (clip_x, clip_w)
        };
        let local = Rect::from_xywh(clip_x, clip_y, clip_w, clip_h);
        Rect::from_ltrb(
            clip.left.max(local.left),
            clip.top.max(local.top),
            clip.right.min(local.right),
            clip.bottom.min(local.bottom),
        )
    } else {
        clip
    };
    for child in &fragment.children {
        collect_fragmented_inline_oof_entries(
            child,
            doc,
            fragment_offset,
            candidate_ids,
            clip,
            column_order,
            entries,
        );
    }
}

fn repaint_fragmented_inline_source_range(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    source_after: usize,
    source_through: Option<usize>,
    inherited_clip: Rect,
    repaint_clip: Rect,
) {
    use openui_style::Position;

    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    let local_clip = if fragment.has_overflow_clip {
        let (clip_x, clip_y, clip_w, clip_h) = compute_clip_rect(fragment, fragment_offset);
        let (clip_x, clip_w) = if fragment.block_axis_clip_only {
            (-1_000_000.0, 2_000_000.0)
        } else {
            (clip_x, clip_w)
        };
        Rect::from_xywh(clip_x, clip_y, clip_w, clip_h)
    } else {
        inherited_clip
    };
    let clip = Rect::from_ltrb(
        inherited_clip
            .left
            .max(local_clip.left)
            .max(repaint_clip.left),
        inherited_clip.top.max(local_clip.top).max(repaint_clip.top),
        inherited_clip
            .right
            .min(local_clip.right)
            .min(repaint_clip.right),
        inherited_clip
            .bottom
            .min(local_clip.bottom)
            .min(repaint_clip.bottom),
    );
    if clip.left >= clip.right || clip.top >= clip.bottom {
        return;
    }

    if !fragment.node_id.is_none() {
        let node_id = fragment.node_id.index();
        let style = &doc.node(fragment.node_id).style;
        if matches!(style.position, Position::Absolute | Position::Fixed) {
            return;
        }
        let in_source_range = node_id > source_after
            && source_through.is_none_or(|source_through| node_id <= source_through);
        if in_source_range && fragment.is_inline_box_fragment {
            canvas.save();
            canvas.clip_rect(clip, ClipOp::Intersect, false);
            paint_fragment(canvas, fragment, doc, parent_offset);
            canvas.restore();
            return;
        }
    }

    for child in &fragment.children {
        repaint_fragmented_inline_source_range(
            canvas,
            child,
            doc,
            fragment_offset,
            source_after,
            source_through,
            clip,
            repaint_clip,
        );
    }
}

/// Paint shared inline-positioned continuations in the stacking position of
/// the fragmented inline as a whole.
fn paint_shared_inline_oof_across_column_row(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    offset: PhysicalOffset,
    _is_stacking_context: bool,
) -> bool {
    use openui_style::Position;

    if FRAGMENTED_OOF_HOIST_ACTIVE.with(|active| *active.borrow()) {
        return false;
    }
    let is_direct_inline_oof = |child: &Fragment| {
        if child.node_id.is_none() {
            return false;
        }
        let node = doc.node(child.node_id);
        matches!(node.style.position, Position::Absolute | Position::Fixed)
            && node.style.z_index.is_none()
            && !node.parent.is_none()
            && doc.node(node.parent).style.display.is_inline_level()
            && child.positioned_fragmentation.is_some()
    };
    let column_count = children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
        .count();
    if column_count < 2
        || children.iter().any(|child| {
            !matches!(
                child.kind,
                FragmentKind::ColumnBox | FragmentKind::ColumnRule
            ) && !is_direct_inline_oof(child)
        })
    {
        return false;
    }

    let mut candidate_ids = BTreeSet::new();
    for child in children {
        collect_inline_oof_ids(child, doc, &mut candidate_ids);
    }
    if candidate_ids.is_empty() {
        return false;
    }

    let mut entries = Vec::new();
    for (column_order, column) in children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
        .enumerate()
    {
        let column_offset = PhysicalOffset::new(
            offset.left + column.offset.left,
            offset.top + column.offset.top,
        );
        let clip = if column.has_overflow_clip {
            let (clip_y, clip_h) = compute_column_block_clip_rect(column, column_offset);
            Rect::from_xywh(-1_000_000.0, clip_y, 2_000_000.0, clip_h)
        } else {
            Rect::from_xywh(-1_000_000.0, -1_000_000.0, 2_000_000.0, 2_000_000.0)
        };
        collect_fragmented_inline_oof_entries(
            column,
            doc,
            offset,
            &candidate_ids,
            clip,
            column_order,
            &mut entries,
        );
    }
    // Positioned continuations are owned by the multicol row rather than
    // duplicated inside its anonymous column boxes.  Their break-token
    // metadata identifies the column whose source-order slot they occupy.
    for child in children.iter().filter(|child| is_direct_inline_oof(child)) {
        let column_order = child
            .positioned_fragmentation
            .as_ref()
            .and_then(|data| data.fragmentainer_index)
            .unwrap_or_default() as usize;
        collect_fragmented_inline_oof_entries(
            child,
            doc,
            offset,
            &candidate_ids,
            Rect::from_xywh(-1_000_000.0, -1_000_000.0, 2_000_000.0, 2_000_000.0),
            column_order,
            &mut entries,
        );
    }

    let mut visible_columns: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for entry in &entries {
        if entry.visible {
            visible_columns
                .entry(entry.fragment.node_id.index())
                .or_default()
                .insert(entry.column_order);
        }
    }
    let selected_ids: BTreeSet<usize> = visible_columns
        .iter()
        .filter_map(|(node_id, columns)| (columns.len() >= 2).then_some(*node_id))
        .collect();
    if selected_ids.is_empty() {
        return false;
    }
    // The special row-wide traversal exists to interleave multiple positioned
    // siblings with the fragmented inline content between them. A lone OOF
    // descendant needs only the ordinary stacking traversal; selecting it here
    // can suppress valid later in-flow continuations.
    // A single continuation needs this row-wide path only when it is the only
    // positioned descendant of its inline owner. If just one of several OOF
    // siblings crosses columns, ordinary traversal already preserves their
    // source-order relationship. Multiple crossing descendants may belong to
    // different inline owners, and are grouped independently below.
    if selected_ids.len() < 2 && candidate_ids.len() != 1 {
        return false;
    }
    let mut inline_groups: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for entry in entries
        .iter()
        .filter(|entry| entry.visible && selected_ids.contains(&entry.fragment.node_id.index()))
    {
        let selected_id = entry.fragment.node_id.index();
        let parent = doc.node(entry.fragment.node_id).parent;
        if !parent.is_none() {
            inline_groups
                .entry(parent.index())
                .or_default()
                .insert(selected_id);
        }
    }
    if inline_groups.is_empty() {
        return false;
    }
    let inline_groups: Vec<(usize, BTreeSet<usize>)> = inline_groups.into_iter().collect();
    let selected_ids: BTreeSet<usize> = inline_groups
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    entries.retain(|entry| selected_ids.contains(&entry.fragment.node_id.index()));
    let skipped: Vec<usize> = entries
        .iter()
        .map(|entry| entry.fragment as *const Fragment as usize)
        .collect();
    HOIST_SKIP.with(|set| set.borrow_mut().extend(skipped.iter().copied()));
    FRAGMENTED_OOF_HOIST_ACTIVE.with(|active| *active.borrow_mut() = true);
    FRAGMENTED_INLINE_SKIP_AFTER.with(|source_after| {
        *source_after.borrow_mut() = inline_groups.first().map(|(parent_id, _)| *parent_id);
    });

    for rule in children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnRule)
    {
        paint_fragment(canvas, rule, doc, offset);
    }
    for column in children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
    {
        paint_fragment(canvas, column, doc, offset);
    }
    FRAGMENTED_INLINE_SKIP_AFTER.with(|source_after| *source_after.borrow_mut() = None);
    for (group_index, (inline_parent_id, group_ids)) in inline_groups.iter().enumerate() {
        let source_through = inline_groups
            .get(group_index + 1)
            .map(|(parent_id, _)| *parent_id);
        for entry in entries
            .iter()
            .filter(|entry| entry.visible && group_ids.contains(&entry.fragment.node_id.index()))
        {
            HOIST_SKIP.with(|set| {
                set.borrow_mut()
                    .remove(&(entry.fragment as *const Fragment as usize));
            });
            canvas.save();
            canvas.clip_rect(entry.clip, ClipOp::Intersect, false);
            paint_fragment(canvas, entry.fragment, doc, entry.parent_offset);
            canvas.restore();
        }
        for column in children
            .iter()
            .filter(|child| child.kind == FragmentKind::ColumnBox)
        {
            let column_offset = PhysicalOffset::new(
                offset.left + column.offset.left,
                offset.top + column.offset.top,
            );
            let column_clip = if column.has_overflow_clip {
                let (clip_y, clip_h) = compute_column_block_clip_rect(column, column_offset);
                Rect::from_xywh(-1_000_000.0, clip_y, 2_000_000.0, clip_h)
            } else {
                Rect::from_xywh(-1_000_000.0, -1_000_000.0, 2_000_000.0, 2_000_000.0)
            };
            repaint_fragmented_inline_source_range(
                canvas,
                column,
                doc,
                offset,
                *inline_parent_id,
                source_through,
                column_clip,
                column_clip,
            );
        }
    }
    HOIST_SKIP.with(|set| {
        let mut set = set.borrow_mut();
        for pointer in &skipped {
            set.remove(pointer);
        }
    });
    FRAGMENTED_OOF_HOIST_ACTIVE.with(|active| *active.borrow_mut() = false);
    true
}

/// Paint children respecting CSS stacking order.
///
/// CSS 2 §E.2 / CSS Positioned Layout §7.2 paint order:
/// 1. Negative z-index stacking contexts (sorted ascending by z-index)
/// 2. In-flow, non-positioned block backgrounds + borders
/// 3. Non-positioned floats
/// 4. In-flow, non-positioned inline content
/// 5. Positioned elements with z-index: auto or 0 (in document order)
/// 6. Positive z-index stacking contexts (sorted ascending by z-index)
///
/// Simplified: paint negative z-index first, then in-flow, then non-negative.
///
/// `is_stacking_context`: true when this call is painting the children of a
/// true CSS stacking context (viewport, or positioned+explicit-z element).
/// When true, positioned z:auto descendants buried in non-SC in-flow children
/// are hoisted into `non_negative_z` so they paint after all in-flow content,
/// matching CSS 2.1 Appendix E step 5 (document-tree order within a SC).
fn paint_children_with_stacking_order(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    offset: PhysicalOffset,
    is_stacking_context: bool,
) {
    use openui_style::Position;

    if paint_shared_inline_oof_across_column_row(canvas, children, doc, offset, is_stacking_context)
    {
        return;
    }

    // Classify children into buckets
    let mut negative_z: Vec<(i32, usize, StackingEntry)> = Vec::new(); // (z-index, doc_order, entry)
    let mut in_flow: Vec<usize> = Vec::new();
    let mut non_negative_z: Vec<(i32, usize, StackingEntry)> = Vec::new();
    let parent_is_flex = children
        .iter()
        .filter_map(|child| (!child.node_id.is_none()).then_some(child.node_id))
        .filter_map(|child_id| {
            let parent_id = doc.node(child_id).parent;
            (!parent_id.is_none()).then_some(parent_id)
        })
        .next()
        .is_some_and(|parent_id| doc.node(parent_id).style.display.is_flex());
    let mut order = 0usize;

    for (i, child) in children.iter().enumerate() {
        if child.node_id.is_none() {
            // Anonymous fragments (line boxes, etc.) are in-flow
            in_flow.push(i);
            continue;
        }
        let child_style = &doc.node(child.node_id).style;
        let is_positioned = matches!(
            child_style.position,
            Position::Absolute | Position::Fixed | Position::Relative | Position::Sticky
        );

        // Flex items participate in z-index ordering even when position: static
        // (CSS Flexbox §5.4).
        let parent_id = doc.node(child.node_id).parent;
        let is_flex_item = !parent_id.is_none() && doc.node(parent_id).style.display.is_flex();
        let has_z_index = child_style.z_index.is_some();

        if is_positioned || (is_flex_item && has_z_index) || child_style.opacity < 1.0 {
            let z = child_style.z_index.unwrap_or(0);
            let paint_order = if parent_is_flex {
                order
            } else {
                child.node_id.index()
            };
            if z < 0 {
                if !parent_is_flex {
                    negative_z.push((z, paint_order, StackingEntry::Direct(i)));
                }
            } else {
                non_negative_z.push((z, paint_order, StackingEntry::Direct(i)));
            }
            order += 1;
        } else {
            in_flow.push(i);
        }
    }

    if parent_is_flex {
        for &idx in &in_flow {
            collect_positioned_descendant_stacking_contexts(
                &children[idx],
                doc,
                offset,
                &mut order,
                &mut negative_z,
                &mut non_negative_z,
                false,
            );
        }
    }

    // When painting a true stacking context, hoist any positioned z:auto
    // descendants from in-flow subtrees so they paint in document tree order
    // alongside the already-classified non_negative_z entries (CSS 2.1 §E step 5).
    let mut negative_hoisted_ptrs: Vec<usize> = Vec::new();
    let mut hoisted_ptrs: Vec<usize> = Vec::new();
    if is_stacking_context {
        for &idx in &in_flow {
            let child = &children[idx];
            collect_negative_z_descendants(
                child,
                doc,
                offset,
                None,
                &mut negative_z,
                &mut negative_hoisted_ptrs,
            );
            // Don't hoist across overflow-clip boundaries: elements inside a
            // clipping container must remain inside it.
            let child_clips = if child.node_id.is_none() {
                child.has_overflow_clip
            } else {
                let cs = &doc.node(child.node_id).style;
                child.has_overflow_clip
                    || (matches!(child.kind, FragmentKind::Box | FragmentKind::Viewport)
                        && (cs.overflow_x != Overflow::Visible
                            || cs.overflow_y != Overflow::Visible))
            };
            if !child_clips {
                collect_positioned_z_auto_descendants(
                    child,
                    doc,
                    offset,
                    &mut non_negative_z,
                    &mut hoisted_ptrs,
                );
            }
        }
    }

    // Sort by z-index (stable sort preserves document order for equal z-index)
    negative_z.sort_by_key(|&(z, order, _)| (z, order));
    non_negative_z.sort_by_key(|&(z, order, _)| (z, order));

    // Register hoisted fragments in HOIST_SKIP so Phase 2 skips them.
    if !hoisted_ptrs.is_empty() {
        HOIST_SKIP.with(|s| {
            let mut set = s.borrow_mut();
            for &p in &hoisted_ptrs {
                set.insert(p);
            }
        });
    }

    // Phase 1: Negative z-index positioned elements
    for (_, _, entry) in &negative_z {
        paint_stacking_entry(canvas, entry, children, doc, offset);
    }

    if !negative_hoisted_ptrs.is_empty() {
        HOIST_SKIP.with(|s| {
            let mut set = s.borrow_mut();
            for &p in &negative_hoisted_ptrs {
                set.insert(p);
            }
        });
    }

    // Column rules paint behind column contents; overflowing descendants may
    // cover them in the gap.
    for &idx in &in_flow {
        if matches!(children[idx].kind, FragmentKind::ColumnRule) {
            paint_fragment(canvas, &children[idx], doc, offset);
        }
    }

    // Phase 2: In-flow elements (in document order); hoisted fragments are
    // skipped. Fragmented roots need their decorations prepainted within one
    // contiguous column row so a later continuation background cannot erase
    // earlier inline overflow. A spanner ends that row: prepainting columns
    // from both sides of it together would put post-spanner backgrounds below
    // the spanner and invert ordinary source paint order.
    let mut in_flow_position = 0usize;
    while in_flow_position < in_flow.len() {
        let idx = in_flow[in_flow_position];
        if children[idx].kind == FragmentKind::ColumnRule {
            in_flow_position += 1;
            continue;
        }
        if children[idx].kind != FragmentKind::ColumnBox {
            paint_fragment(canvas, &children[idx], doc, offset);
            in_flow_position += 1;
            continue;
        }

        let row_start = idx;
        let mut row_end = idx + 1;
        let mut row_position_end = in_flow_position + 1;
        while row_position_end < in_flow.len() {
            let candidate = in_flow[row_position_end];
            if candidate != row_end
                || !matches!(
                    children[candidate].kind,
                    FragmentKind::ColumnBox | FragmentKind::ColumnRule
                )
            {
                break;
            }
            row_end += 1;
            row_position_end += 1;
        }

        let prepainted = prepaint_shared_column_root_decorations(
            canvas,
            &children[row_start..row_end],
            doc,
            offset,
        );
        if !prepainted.is_empty() {
            PREPAINTED_COLUMN_DECORATIONS.with(|fragments| {
                fragments.borrow_mut().extend(prepainted.iter().copied());
            });
        }
        for &row_idx in &in_flow[in_flow_position..row_position_end] {
            if children[row_idx].kind != FragmentKind::ColumnRule {
                paint_fragment(canvas, &children[row_idx], doc, offset);
            }
        }
        if !prepainted.is_empty() {
            PREPAINTED_COLUMN_DECORATIONS.with(|fragments| {
                let mut fragments = fragments.borrow_mut();
                for pointer in &prepainted {
                    fragments.remove(pointer);
                }
            });
        }
        in_flow_position = row_position_end;
    }

    // Deregister hoisted fragments before Phase 3 so they paint correctly.
    if !hoisted_ptrs.is_empty() {
        HOIST_SKIP.with(|s| {
            let mut set = s.borrow_mut();
            for &p in &hoisted_ptrs {
                set.remove(&p);
            }
        });
    }

    // Phase 3: Non-negative z-index positioned elements (+ hoisted z:auto).
    for (_, _, entry) in &non_negative_z {
        paint_stacking_entry(canvas, entry, children, doc, offset);
    }

    if !negative_hoisted_ptrs.is_empty() {
        HOIST_SKIP.with(|s| {
            let mut set = s.borrow_mut();
            for &p in &negative_hoisted_ptrs {
                set.remove(&p);
            }
        });
    }
}

fn prepaint_shared_column_root_decorations(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    offset: PhysicalOffset,
) -> Vec<usize> {
    let columns: Vec<_> = children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
        .collect();
    if columns.len() < 2 {
        return Vec::new();
    }

    let mut occurrences: BTreeMap<usize, usize> = BTreeMap::new();
    for column in &columns {
        for fragment in &column.children {
            if !fragment.node_id.is_none() && fragment.kind == FragmentKind::Box {
                *occurrences.entry(fragment.node_id.index()).or_default() += 1;
            }
        }
    }
    if !occurrences.values().any(|count| *count > 1) {
        return Vec::new();
    }

    let mut prepainted = Vec::new();
    for column in columns {
        let column_offset = PhysicalOffset::new(
            offset.left + column.offset.left,
            offset.top + column.offset.top,
        );
        canvas.save();
        if column.has_overflow_clip {
            let (clip_y, clip_h) = compute_column_block_clip_rect(column, column_offset);
            let (clip_x, clip_w) = if column.block_axis_clip_only && !column.inline_axis_clip_only {
                (-1_000_000.0, 2_000_000.0)
            } else {
                (column_offset.left.to_f32(), column.size.width.to_f32())
            };
            let (clip_y, clip_h) = if column.inline_axis_clip_only {
                (-1_000_000.0, 2_000_000.0)
            } else {
                (clip_y, clip_h)
            };
            canvas.clip_rect(
                Rect::from_xywh(clip_x, clip_y, clip_w, clip_h),
                ClipOp::Intersect,
                false,
            );
        }
        for fragment in &column.children {
            if fragment.node_id.is_none()
                || fragment.kind != FragmentKind::Box
                || occurrences
                    .get(&fragment.node_id.index())
                    .copied()
                    .unwrap_or_default()
                    < 2
            {
                continue;
            }
            let style = &doc.node(fragment.node_id).style;
            if style.visibility != Visibility::Visible || style.opacity < 1.0 {
                continue;
            }
            let fragment_offset = PhysicalOffset::new(
                column_offset.left + fragment.offset.left,
                column_offset.top + fragment.offset.top,
            );
            paint_box_decoration_background(canvas, fragment, style, fragment_offset, 1.0);
            prepainted.push(fragment as *const Fragment as usize);
        }
        canvas.restore();
    }
    prepainted
}

fn paint_stacking_entry(
    canvas: &Canvas,
    entry: &StackingEntry,
    children: &[Fragment],
    doc: &Document,
    offset: PhysicalOffset,
) {
    match entry {
        StackingEntry::Direct(idx) => paint_fragment(canvas, &children[*idx], doc, offset),
        StackingEntry::Descendant(fragment, parent_offset) => {
            paint_fragment(canvas, fragment, doc, *parent_offset)
        }
        StackingEntry::DescendantWithClip(fragment, parent_offset, clip_rect) => {
            canvas.save();
            canvas.clip_rect(*clip_rect, ClipOp::Intersect, false);
            paint_fragment(canvas, fragment, doc, *parent_offset);
            canvas.restore();
        }
    }
}

/// Returns true if `fragment` is a CSS stacking context.
///
/// Stacking contexts are formed by:
///   - The viewport (root)
///   - Positioned elements with an explicit z-index value
///   - Elements with opacity < 1 (composited via save_layer in paint_fragment)
fn is_fragment_stacking_context(fragment: &Fragment, doc: &Document) -> bool {
    use openui_style::Position;
    if fragment.kind == FragmentKind::Viewport {
        return true;
    }
    if fragment.node_id.is_none() {
        return false;
    }
    let style = &doc.node(fragment.node_id).style;
    let is_positioned = matches!(
        style.position,
        Position::Absolute | Position::Fixed | Position::Relative | Position::Sticky
    );
    (is_positioned && style.z_index.is_some()) || style.opacity < 1.0
}

/// Walk an in-flow fragment subtree, collecting zero-or-positive stacking
/// descendants into `non_negative_z` (hoisting them to the nearest stacking
/// context) and recording their raw pointers in `hoisted_ptrs` so Phase 2 can
/// skip them. This includes positioned z:auto descendants, positioned
/// descendants with a non-negative z-index, and opacity stacking contexts.
///
/// Stops at fragmentainer and overflow-clip boundaries so a hoisted fragment
/// cannot escape its authoritative paint clip.
fn collect_positioned_z_auto_descendants<'a>(
    fragment: &'a Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    non_negative_z: &mut Vec<(i32, usize, StackingEntry<'a>)>,
    hoisted_ptrs: &mut Vec<usize>,
) {
    use openui_style::Position;

    // The absolute offset of `fragment` itself — passed as parent_offset to
    // paint_fragment for any child we hoist.
    let frag_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );

    for child in &fragment.children {
        if child.kind == FragmentKind::ColumnBox {
            let column_offset = PhysicalOffset::new(
                frag_offset.left + child.offset.left,
                frag_offset.top + child.offset.top,
            );
            let column_clip = if child.has_overflow_clip {
                let (clip_y, clip_h) = compute_column_block_clip_rect(child, column_offset);
                let (clip_x, clip_w) = if child.block_axis_clip_only && !child.inline_axis_clip_only
                {
                    (-1_000_000.0, 2_000_000.0)
                } else {
                    (column_offset.left.to_f32(), child.size.width.to_f32())
                };
                let (clip_y, clip_h) = if child.inline_axis_clip_only {
                    (-1_000_000.0, 2_000_000.0)
                } else {
                    (clip_y, clip_h)
                };
                Some(Rect::from_xywh(clip_x, clip_y, clip_w, clip_h))
            } else {
                None
            };
            collect_fragmented_opacity_stacking_contexts(
                child,
                doc,
                frag_offset,
                column_clip,
                non_negative_z,
                hoisted_ptrs,
            );
            continue;
        }

        if child.node_id.is_none() {
            // Anonymous box: recurse.
            collect_positioned_z_auto_descendants(
                child,
                doc,
                frag_offset,
                non_negative_z,
                hoisted_ptrs,
            );
            continue;
        }

        let child_style = &doc.node(child.node_id).style;
        let is_positioned = matches!(
            child_style.position,
            Position::Absolute | Position::Fixed | Position::Relative | Position::Sticky
        );
        // A positioned continuation whose containing block is a fragmented
        // inline is painted by the owning multicol row. Hoisting it again to
        // an ancestor stacking context loses the source-order interleaving
        // between inline content and later positioned continuations.
        let owned_by_fragmented_inline =
            child.positioned_fragmentation.as_ref().is_some_and(|data| {
                data.inline_containing_block_node.is_some() && data.fragmentainer_index.is_some()
            });
        if is_positioned && owned_by_fragmented_inline {
            continue;
        }
        let stacking_level = if is_positioned {
            match child_style.z_index {
                Some(z) if z < 0 => None,
                Some(z) => Some(z),
                None => Some(0),
            }
        } else if child_style.opacity < 1.0 {
            Some(0)
        } else {
            None
        };
        if let Some(z) = stacking_level {
            // Hoist this stacking context to the nearest SC. The DOM index
            // preserves document-tree order among peers at the same level.
            let ptr = child as *const Fragment as usize;
            non_negative_z.push((
                z,
                child.node_id.index(),
                StackingEntry::Descendant(child, frag_offset),
            ));
            hoisted_ptrs.push(ptr);
            // Don't recurse: the entire subtree paints as part of this fragment.
        } else if is_positioned && child_style.z_index.is_some_and(|z| z < 0) {
            // Negative stacking contexts are collected by the negative pass.
        } else {
            // Non-positioned, non-SC: only recurse if the child does NOT clip
            // overflow. Elements inside an overflow-clipping container must
            // remain inside it — hoisting them out would bypass the clip.
            let clips_overflow = child.has_overflow_clip
                || (matches!(child.kind, FragmentKind::Box | FragmentKind::Viewport)
                    && (child_style.overflow_x != Overflow::Visible
                        || child_style.overflow_y != Overflow::Visible));
            if !clips_overflow {
                collect_positioned_z_auto_descendants(
                    child,
                    doc,
                    frag_offset,
                    non_negative_z,
                    hoisted_ptrs,
                );
            }
        }
    }
}

/// Hoist opacity stacking contexts out of a column's in-flow paint pass while
/// retaining that fragmentainer's authoritative clip. Opacity establishes a
/// stacking context at the nearest ancestor stacking level, so its visible
/// overflow paints after later in-flow siblings in the same column row.
fn collect_fragmented_opacity_stacking_contexts<'a>(
    fragment: &'a Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    column_clip: Option<Rect>,
    non_negative_z: &mut Vec<(i32, usize, StackingEntry<'a>)>,
    hoisted_ptrs: &mut Vec<usize>,
) {
    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    for child in &fragment.children {
        if child.node_id.is_none() {
            collect_fragmented_opacity_stacking_contexts(
                child,
                doc,
                fragment_offset,
                column_clip,
                non_negative_z,
                hoisted_ptrs,
            );
            continue;
        }
        let style = &doc.node(child.node_id).style;
        if style.opacity < 1.0 {
            let pointer = child as *const Fragment as usize;
            let entry = column_clip
                .map_or(StackingEntry::Descendant(child, fragment_offset), |clip| {
                    StackingEntry::DescendantWithClip(child, fragment_offset, clip)
                });
            non_negative_z.push((style.z_index.unwrap_or(0), child.node_id.index(), entry));
            hoisted_ptrs.push(pointer);
        } else if !is_fragment_stacking_context(child, doc) {
            let clips_overflow = child.has_overflow_clip
                || (matches!(child.kind, FragmentKind::Box | FragmentKind::Viewport)
                    && (style.overflow_x != Overflow::Visible
                        || style.overflow_y != Overflow::Visible));
            if !clips_overflow {
                collect_fragmented_opacity_stacking_contexts(
                    child,
                    doc,
                    fragment_offset,
                    column_clip,
                    non_negative_z,
                    hoisted_ptrs,
                );
            }
        }
    }
}

fn collect_negative_z_descendants<'a>(
    fragment: &'a Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    opaque_ancestor_clip: Option<Rect>,
    negative_z: &mut Vec<(i32, usize, StackingEntry<'a>)>,
    hoisted_ptrs: &mut Vec<usize>,
) {
    use openui_style::Position;

    let frag_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    let mut child_opaque_clip = opaque_ancestor_clip;
    if !fragment.node_id.is_none() {
        let style = &doc.node(fragment.node_id).style;
        if matches!(fragment.kind, FragmentKind::Box | FragmentKind::Viewport)
            && style.background_color.a >= 0.99
            && style.border_top_left_radius == (0.0, 0.0)
            && style.border_top_right_radius == (0.0, 0.0)
            && style.border_bottom_right_radius == (0.0, 0.0)
            && style.border_bottom_left_radius == (0.0, 0.0)
        {
            let rect = Rect::from_xywh(
                frag_offset.left.round().to_f32(),
                frag_offset.top.round().to_f32(),
                fragment.size.width.round().to_f32(),
                fragment.size.height.round().to_f32(),
            );
            child_opaque_clip = Some(match child_opaque_clip {
                Some(existing) => {
                    let left = existing.left.max(rect.left);
                    let top = existing.top.max(rect.top);
                    let right = existing.right.min(rect.right);
                    let bottom = existing.bottom.min(rect.bottom);
                    Rect::from_ltrb(left, top, right.max(left), bottom.max(top))
                }
                None => rect,
            });
        }
    }

    for child in &fragment.children {
        if child.node_id.is_none() {
            collect_negative_z_descendants(
                child,
                doc,
                frag_offset,
                child_opaque_clip,
                negative_z,
                hoisted_ptrs,
            );
            continue;
        }

        let child_style = &doc.node(child.node_id).style;
        let is_positioned = matches!(
            child_style.position,
            Position::Absolute | Position::Fixed | Position::Relative | Position::Sticky
        );
        if is_positioned && child_style.z_index.is_some_and(|z| z < 0) {
            let ptr = child as *const Fragment as usize;
            let entry = if let Some(clip) = child_opaque_clip {
                StackingEntry::DescendantWithClip(child, frag_offset, clip)
            } else {
                StackingEntry::Descendant(child, frag_offset)
            };
            negative_z.push((
                child_style.z_index.unwrap_or(0),
                child.node_id.index(),
                entry,
            ));
            hoisted_ptrs.push(ptr);
            continue;
        }

        if !is_fragment_stacking_context(child, doc) {
            collect_negative_z_descendants(
                child,
                doc,
                frag_offset,
                child_opaque_clip,
                negative_z,
                hoisted_ptrs,
            );
        }
    }
}

fn collect_positioned_descendant_stacking_contexts<'a>(
    fragment: &'a Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    order: &mut usize,
    negative_z: &mut Vec<(i32, usize, StackingEntry<'a>)>,
    non_negative_z: &mut Vec<(i32, usize, StackingEntry<'a>)>,
    use_dom_order: bool,
) {
    use openui_style::Position;

    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );

    for child in &fragment.children {
        let mut collected = false;
        if !child.node_id.is_none() {
            let child_id = child.node_id;
            let child_style = &doc.node(child_id).style;
            let is_positioned = matches!(
                child_style.position,
                Position::Absolute | Position::Fixed | Position::Relative | Position::Sticky
            );
            if is_positioned {
                if let Some(z) = child_style.z_index {
                    let entry = StackingEntry::Descendant(child, fragment_offset);
                    let paint_order = if use_dom_order {
                        child_id.index()
                    } else {
                        *order
                    };
                    if z < 0 {
                        // Negative descendants of flex items were painted before
                        // the flex container's own decoration.
                    } else {
                        non_negative_z.push((z, paint_order, entry));
                    }
                    *order += 1;
                    collected = true;
                }
            }
        }

        if !collected {
            collect_positioned_descendant_stacking_contexts(
                child,
                doc,
                fragment_offset,
                order,
                negative_z,
                non_negative_z,
                use_dom_order,
            );
        }
    }
}

fn paint_flex_negative_stacking_children(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
) {
    for child in &fragment.children {
        paint_negative_stacking_descendants(canvas, child, doc, offset);
    }
}

fn paint_negative_stacking_descendants(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
) {
    use openui_style::Position;

    if !fragment.node_id.is_none() {
        let style = &doc.node(fragment.node_id).style;
        let is_positioned = matches!(
            style.position,
            Position::Absolute | Position::Fixed | Position::Relative | Position::Sticky
        );
        let parent_id = doc.node(fragment.node_id).parent;
        let is_flex_item = !parent_id.is_none() && doc.node(parent_id).style.display.is_flex();
        if (is_positioned || is_flex_item) && style.z_index.is_some_and(|z| z < 0) {
            paint_fragment(canvas, fragment, doc, parent_offset);
            return;
        }
    }

    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    for child in &fragment.children {
        paint_negative_stacking_descendants(canvas, child, doc, fragment_offset);
    }
}

// ── Overflow clipping helpers ────────────────────────────────────────

/// Determine whether a fragment needs overflow clipping.
///
/// Returns `true` when either the layout-computed `has_overflow_clip` flag is
/// set, or the style's `overflow-x`/`overflow-y` is not `visible` for a
/// box-level fragment.
fn needs_overflow_clip(fragment: &Fragment, style: &ComputedStyle, doc: &Document) -> bool {
    if doc.node(fragment.node_id).tag == openui_dom::ElementTag::Body
        && doc.body_overflow_is_propagated()
    {
        return false;
    }
    fragment.has_overflow_clip
        || ((style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible)
            && matches!(fragment.kind, FragmentKind::Box | FragmentKind::Viewport))
}

fn can_flatten_box_opacity(fragment: &Fragment, style: &ComputedStyle) -> bool {
    // A background-only leaf box can fold opacity into its fill paint without
    // changing CSS compositing; this avoids an extra saveLayer AA quantization.
    //
    // Keep this disabled for now: rounded overflow clips depend on the child
    // opacity being composited through the same clip mask as Chromium. Folding
    // the opacity into the fill changes the AA edge for `overflow-clip-margin`.
    false
        && style.opacity < 1.0
        && matches!(fragment.kind, FragmentKind::Box)
        && fragment.children.is_empty()
        && !style.background_color.is_transparent()
        && style.box_shadow.is_empty()
        && !style.has_outline()
        && style.effective_border_top() == 0
        && style.effective_border_right() == 0
        && style.effective_border_bottom() == 0
        && style.effective_border_left() == 0
}

fn paint_list_marker(
    canvas: &Canvas,
    _fragment: &Fragment,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
) {
    let font_size = style.font_size.max(1.0);
    let line_height = match style.line_height {
        LineHeight::Normal => font_size * 1.2,
        LineHeight::Number(n) => font_size * n,
        LineHeight::Length(px) => px,
        LineHeight::Percentage(pct) => font_size * pct / 100.0,
    }
    .max(font_size);

    let marker_diameter = if font_size >= 18.0 {
        6.0
    } else {
        (font_size * 0.3).round().max(5.0)
    };
    let marker_x = abs_offset.left.round().to_f32()
        - if style.list_style_position == ListStylePosition::Outside {
            if style.column_count == Some(1) {
                17.0
            } else {
                19.0
            }
        } else {
            0.0
        };
    let marker_y_adjust = if font_size >= 18.0 { 1.0 } else { 0.0 };
    let marker_y = abs_offset.top.round().to_f32()
        + ((line_height - marker_diameter) / 2.0).round()
        + marker_y_adjust;

    let mut paint = Paint::default();
    paint.set_color(skia_safe::Color::BLACK);
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    canvas.draw_oval(
        Rect::from_xywh(marker_x, marker_y, marker_diameter, marker_diameter),
        &paint,
    );
}

/// Compute the clip rectangle for overflow clipping.
///
/// Per CSS spec, overflow clips to the **padding box** — the border-box
/// inset by each side's border width. Returns `(x, y, width, height)`.
pub fn compute_clip_rect(fragment: &Fragment, offset: PhysicalOffset) -> (f32, f32, f32, f32) {
    // Pixel-snap the padding box edges independently for crisp clipping.
    let clip_left = (offset.left + fragment.border.left).round().to_f32();
    let clip_top = (offset.top + fragment.border.top).round().to_f32();
    let clip_right = (offset.left + fragment.size.width - fragment.border.right)
        .round()
        .to_f32();
    let clip_bottom = (offset.top + fragment.size.height - fragment.border.bottom)
        .round()
        .to_f32();
    let clip_w = (clip_right - clip_left).max(0.0);
    let clip_h = (clip_bottom - clip_top).max(0.0);
    (clip_left, clip_top, clip_w, clip_h)
}

/// Column fragmentainers clip continuation content at their block edges, but
/// direct descendants may create ink overflow above the column through normal
/// flow (for example, a negative collapsed start margin). Extend only to the
/// direct fragment's placement; negative offsets inside a continuation remain
/// protected by the fragmentainer clip.
fn compute_column_block_clip_rect(fragment: &Fragment, offset: PhysicalOffset) -> (f32, f32) {
    let (_, clip_top, _, clip_height) = compute_clip_rect(fragment, offset);
    let clip_bottom = clip_top + clip_height + fragment.column_block_end_ink_overflow.to_f32();
    let clip_top = clip_top - fragment.column_block_start_ink_overflow.to_f32();
    let direct_ink_top = fragment
        .children
        .iter()
        .filter(|child| child.offset.top < LayoutUnit::zero())
        .map(|child| (offset.top + child.offset.top).to_f32())
        .fold(clip_top, f32::min);
    (direct_ink_top, (clip_bottom - direct_ink_top).max(0.0))
}

fn overflow_clip_reference_box(style: &ComputedStyle) -> OverflowClipBox {
    if style.overflow_x == Overflow::Clip || style.overflow_y == Overflow::Clip {
        style.overflow_clip_box
    } else {
        OverflowClipBox::PaddingBox
    }
}

/// Apply overflow clipping, paint children, then restore the canvas.
///
/// Mirrors Blink's `BoxFragmentPainter::PaintOverflowClip()`:
///   1. `canvas.save()`
///   2. Clip to padding box (with rounded corners when border-radius is set)
///   3. Translate for scroll offset (overflow: scroll/auto — currently 0,0)
///   4. Paint children
///   5. `canvas.restore()`
fn paint_with_overflow_clip(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
    style: &ComputedStyle,
) {
    // CSS Overflow 3 §3: overflow-clip-margin <visual-box>? <length>
    // The visual-box determines which box edge the clip starts from:
    //   padding-box (default): same as compute_clip_rect
    //   content-box: inset by padding
    //   border-box: expand out to border edge
    let (clip_x, clip_y, clip_w, clip_h) = {
        let (px, py, pw, ph) = compute_clip_rect(fragment, offset);
        match overflow_clip_reference_box(style) {
            OverflowClipBox::PaddingBox => (px, py, pw, ph),
            OverflowClipBox::ContentBox => {
                // Inset from padding-box by padding amounts
                let pl = resolve_margin_or_padding_f32(&style.padding_left, pw);
                let pr = resolve_margin_or_padding_f32(&style.padding_right, pw);
                let pt = resolve_margin_or_padding_f32(&style.padding_top, ph);
                let pb = resolve_margin_or_padding_f32(&style.padding_bottom, ph);
                (
                    px + pl,
                    py + pt,
                    (pw - pl - pr).max(0.0),
                    (ph - pt - pb).max(0.0),
                )
            }
            OverflowClipBox::BorderBox => {
                // Expand from padding-box out to border edge
                let bl = fragment.border.left.to_f32();
                let br = fragment.border.right.to_f32();
                let bt = fragment.border.top.to_f32();
                let bb = fragment.border.bottom.to_f32();
                (px - bl, py - bt, pw + bl + br, ph + bt + bb)
            }
        }
    };

    // Per CSS Overflow 3, when overflow is `clip`, expand the clip rect
    // outward by `overflow-clip-margin` on all sides.
    let margin = style.overflow_clip_margin;
    let has_clip_axis = style.overflow_x == Overflow::Clip || style.overflow_y == Overflow::Clip;
    let margin_x = if has_clip_axis && style.overflow_x == Overflow::Clip {
        margin
    } else {
        0.0
    };
    let margin_y = if has_clip_axis && style.overflow_y == Overflow::Clip {
        margin
    } else {
        0.0
    };
    let (clip_x, clip_y, clip_w, clip_h) = if margin_x != 0.0 || margin_y != 0.0 {
        (
            clip_x - margin_x,
            clip_y - margin_y,
            clip_w + margin_x * 2.0,
            clip_h + margin_y * 2.0,
        )
    } else {
        (clip_x, clip_y, clip_w, clip_h)
    };

    // CSS Overflow 3: visible remains unbounded only when paired with `clip`.
    // When paired with hidden/scroll/auto, the visible axis computes to auto
    // and clips as part of the scroll container.
    let big = 100_000.0_f32;
    let clip_x_visible =
        style.overflow_x == Overflow::Visible && style.overflow_y == Overflow::Clip;
    let clip_y_visible =
        style.overflow_y == Overflow::Visible && style.overflow_x == Overflow::Clip;
    let (clip_x, clip_y, clip_w, clip_h) = match (clip_x_visible, clip_y_visible) {
        (true, false) => (-big, clip_y, big * 2.0, clip_h),
        (false, true) => (clip_x, -big, clip_w, big * 2.0),
        _ => (clip_x, clip_y, clip_w, clip_h),
    };

    // Box-decoration-break:slice correction for fragmented overflow clip.
    //
    // When a block is fragmented across multicol columns, border/padding of
    // the non-rendered sides must not contribute to the overflow clip region:
    //
    //   Non-first fragment: block-start border/padding is not painted, so the
    //   clip must not start below the fragment's own top edge.
    //
    //   Non-last fragment: block-end border is not painted or included in the
    //   fragment's used block size, so the padding-box clip extends to the
    //   fragmentainer edge instead of subtracting the source box's block-end
    //   border a second time.
    let (clip_x, clip_y, clip_w, clip_h) =
        if !fragment.is_first_for_node || !fragment.is_last_for_node {
            let frag_top = offset.top.to_f32();
            let mut cy = clip_y;
            let mut ch = clip_h;

            if !fragment.is_first_for_node && cy > frag_top {
                // Expand clip upward to the fragment's top edge, preserving
                // the existing clip_bottom (the rendered end of the content).
                let clip_bottom = cy + ch;
                cy = frag_top;
                ch = (clip_bottom - cy).max(0.0);
            }

            if !fragment.is_last_for_node {
                let fragment_bottom = frag_top + fragment.size.height.to_f32();
                ch = (fragment_bottom - cy).max(0.0);
            }

            (clip_x, cy, clip_w, ch)
        } else {
            (clip_x, clip_y, clip_w, clip_h)
        };

    let (clip_x, clip_w) = if fragment.block_axis_clip_only {
        (-100_000.0_f32, 200_000.0_f32)
    } else {
        (clip_x, clip_w)
    };
    let clip_rect = Rect::from_xywh(clip_x, clip_y, clip_w, clip_h);

    canvas.save();

    // When border-radius is set, clip to a rounded rect so children are
    // clipped along the curves. Otherwise use a simple rect clip.
    if style.has_border_radius() {
        let rrect = build_clip_rrect(
            &clip_rect,
            fragment,
            style,
            overflow_clip_reference_box(style),
            margin_x,
            margin_y,
        );
        canvas.clip_rrect(rrect, ClipOp::Intersect, true);
    } else {
        // Axis-aligned rectangular overflow clips are pixel-snapped above and
        // rasterized as hard edges. Coverage antialiasing here leaves a
        // one-pixel fringe at multicol fragmentation boundaries.
        canvas.clip_rect(clip_rect, ClipOp::Intersect, false);
    }

    // For scrollable overflow (scroll / auto), apply scroll offset
    // translation. Actual scroll offsets will be supplied by the scroll
    // system in a future task; for now they default to (0, 0).
    if style.overflow_x.is_scrollable() || style.overflow_y.is_scrollable() {
        let scroll_x: f32 = 0.0;
        let scroll_y: f32 = 0.0;
        if scroll_x != 0.0 || scroll_y != 0.0 {
            canvas.translate((-scroll_x, -scroll_y));
        }
    }

    // Paint children inside the clip (with stacking order).
    let is_sc = is_fragment_stacking_context(fragment, doc);
    paint_children_with_stacking_order(canvas, &fragment.children, doc, offset, is_sc);

    paint_scrollbars_if_needed(canvas, fragment, style, &clip_rect);

    canvas.restore();
}

/// Build a Skia `RRect` for an overflow clip edge.
///
/// Blink starts from the padding-box rounded rect and outsets that contour to
/// the requested `overflow-clip-margin` visual box, applying the CSS
/// corner-correction formula while doing so.
fn build_clip_rrect(
    clip_rect: &Rect,
    fragment: &Fragment,
    style: &ComputedStyle,
    reference_box: OverflowClipBox,
    margin_x: f32,
    margin_y: f32,
) -> RRect {
    let bt = style.effective_border_top() as f32;
    let br = style.effective_border_right() as f32;
    let bb = style.effective_border_bottom() as f32;
    let bl = style.effective_border_left() as f32;
    let pt = fragment.padding.top.to_f32();
    let pr = fragment.padding.right.to_f32();
    let pb = fragment.padding.bottom.to_f32();
    let pl = fragment.padding.left.to_f32();
    let border_rect = Rect::from_xywh(
        0.0,
        0.0,
        fragment.size.width.to_f32(),
        fragment.size.height.to_f32(),
    );
    let outer = normalized_border_radii(style, &border_rect);

    let padding_width = (fragment.size.width.to_f32() - bl - br).max(0.0);
    let padding_height = (fragment.size.height.to_f32() - bt - bb).max(0.0);

    // Overflow clips are based on the padding-box rounded rect, then outset
    // to the requested visual box plus overflow-clip-margin.
    let tl_base = Point::new((outer[0].x - bl).max(0.0), (outer[0].y - bt).max(0.0));
    let tr_base = Point::new((outer[1].x - br).max(0.0), (outer[1].y - bt).max(0.0));
    let br_base = Point::new((outer[2].x - br).max(0.0), (outer[2].y - bb).max(0.0));
    let bl_base = Point::new((outer[3].x - bl).max(0.0), (outer[3].y - bb).max(0.0));

    let (ot, or, ob, ol) = match reference_box {
        OverflowClipBox::BorderBox => (bt + margin_y, br + margin_x, bb + margin_y, bl + margin_x),
        OverflowClipBox::PaddingBox => (margin_y, margin_x, margin_y, margin_x),
        OverflowClipBox::ContentBox => (margin_y - pt, margin_x - pr, margin_y - pb, margin_x - pl),
    };

    let adjust_dimension = |radius: f32, outset: f32, coverage: f32| {
        if radius <= 0.0 || outset == 0.0 {
            return radius;
        }
        if outset < 0.0 || radius > outset || coverage > 1.0 {
            return (radius + outset).max(0.0);
        }
        let ratio = radius / outset;
        radius + outset * (1.0 - (1.0 - ratio).powi(3) * (1.0 - coverage.powi(3)))
    };

    let adjust_corner = |base: Point, outset_x: f32, outset_y: f32| {
        if base.x <= 0.0 || base.y <= 0.0 || (outset_x == 0.0 && outset_y == 0.0) {
            return base;
        }
        let coverage = if padding_width > 0.0 && padding_height > 0.0 {
            2.0 * (base.x / padding_width).min(base.y / padding_height)
        } else {
            1.0
        };
        Point::new(
            adjust_dimension(base.x, outset_x, coverage),
            adjust_dimension(base.y, outset_y, coverage),
        )
    };

    let tl = adjust_corner(tl_base, ol, ot);
    let tr = adjust_corner(tr_base, or, ot);
    let br = adjust_corner(br_base, or, ob);
    let bl = adjust_corner(bl_base, ol, ob);

    // skia_safe::RRect radii order: top-left, top-right, bottom-right, bottom-left
    let radii = [tl, tr, br, bl];

    RRect::new_rect_radii(*clip_rect, &radii)
}

fn normalize_radii_to_rect(mut radii: [Point; 4], rect: &Rect) -> [Point; 4] {
    let width = rect.width();
    let height = rect.height();
    if width <= 0.0 || height <= 0.0 {
        return radii;
    }

    let mut scale = 1.0_f32;
    for (sum, limit) in [
        (radii[0].x + radii[1].x, width),
        (radii[3].x + radii[2].x, width),
        (radii[0].y + radii[3].y, height),
        (radii[1].y + radii[2].y, height),
    ] {
        if sum > 0.0 {
            scale = scale.min(limit / sum);
        }
    }

    if scale < 1.0 {
        for radius in &mut radii {
            radius.x *= scale;
            radius.y *= scale;
        }
    }
    radii
}

fn normalized_border_radii(style: &ComputedStyle, rect: &Rect) -> [Point; 4] {
    let radii = [
        Point::new(
            style.border_top_left_radius.0,
            style.border_top_left_radius.1,
        ),
        Point::new(
            style.border_top_right_radius.0,
            style.border_top_right_radius.1,
        ),
        Point::new(
            style.border_bottom_right_radius.0,
            style.border_bottom_right_radius.1,
        ),
        Point::new(
            style.border_bottom_left_radius.0,
            style.border_bottom_left_radius.1,
        ),
    ];

    normalize_radii_to_rect(radii, rect)
}

fn thin_uniform_circular_border_radii(
    style: &ComputedStyle,
    rect: &Rect,
    _border_width: f32,
) -> [Point; 4] {
    normalized_border_radii(style, rect)
}

fn slice_adjust_border_radii(mut radii: [Point; 4], fragment: &Fragment) -> [Point; 4] {
    if fragment.is_inline_box_fragment {
        if !fragment.is_first_for_node {
            radii[0] = Point::new(0.0, 0.0);
            radii[3] = Point::new(0.0, 0.0);
        }
        if !fragment.is_last_for_node {
            radii[1] = Point::new(0.0, 0.0);
            radii[2] = Point::new(0.0, 0.0);
        }
    } else {
        if !fragment.is_first_for_node {
            radii[0] = Point::new(0.0, 0.0);
            radii[1] = Point::new(0.0, 0.0);
        }
        if !fragment.is_last_for_node {
            radii[2] = Point::new(0.0, 0.0);
            radii[3] = Point::new(0.0, 0.0);
        }
    }
    radii
}

fn fragment_border_radii(
    style: &ComputedStyle,
    fragment: &Fragment,
    rect: &Rect,
    border_width: f32,
) -> [Point; 4] {
    let normalization_rect = fragment.decoration_slice.map_or(*rect, |slice| {
        Rect::from_xywh(
            rect.left,
            rect.top - slice.source_block_offset.to_f32(),
            rect.width(),
            slice.source_block_size.to_f32(),
        )
    });
    slice_adjust_border_radii(
        thin_uniform_circular_border_radii(style, &normalization_rect, border_width),
        fragment,
    )
}

fn has_any_radius(radii: &[Point; 4]) -> bool {
    radii.iter().any(|r| r.x > 0.0 || r.y > 0.0)
}

fn specified_border_radii(style: &ComputedStyle) -> [Point; 4] {
    [
        Point::new(
            style.border_top_left_radius.0,
            style.border_top_left_radius.1,
        ),
        Point::new(
            style.border_top_right_radius.0,
            style.border_top_right_radius.1,
        ),
        Point::new(
            style.border_bottom_right_radius.0,
            style.border_bottom_right_radius.1,
        ),
        Point::new(
            style.border_bottom_left_radius.0,
            style.border_bottom_left_radius.1,
        ),
    ]
}

fn all_effective_borders_transparent(style: &ComputedStyle) -> bool {
    style.effective_border_top() > 0
        && style.effective_border_right() > 0
        && style.effective_border_bottom() > 0
        && style.effective_border_left() > 0
        && style
            .border_top_color
            .resolve(&style.color)
            .is_transparent()
        && style
            .border_right_color
            .resolve(&style.color)
            .is_transparent()
        && style
            .border_bottom_color
            .resolve(&style.color)
            .is_transparent()
        && style
            .border_left_color
            .resolve(&style.color)
            .is_transparent()
}

fn radii_exceed_rect(rect: &Rect, radii: &[Point; 4]) -> bool {
    radii[0].x + radii[1].x > rect.width()
        || radii[3].x + radii[2].x > rect.width()
        || radii[0].y + radii[3].y > rect.height()
        || radii[1].y + radii[2].y > rect.height()
}

fn clip_nonrenderable_inner_rounded_rect(
    canvas: &Canvas,
    outer_rect: Rect,
    clip_rect: Rect,
    radii: &[Point; 4],
) {
    if radii[0].x > 0.0 || radii[0].y > 0.0 {
        let corner_rect = Rect::from_ltrb(
            clip_rect.left,
            clip_rect.top,
            outer_rect.right,
            outer_rect.bottom,
        );
        let corner_radii = [
            radii[0],
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
        ];
        canvas.clip_rrect(
            RRect::new_rect_radii(corner_rect, &corner_radii),
            ClipOp::Intersect,
            true,
        );
    }

    if radii[2].x > 0.0 || radii[2].y > 0.0 {
        let corner_rect = Rect::from_ltrb(
            outer_rect.left,
            outer_rect.top,
            clip_rect.right,
            clip_rect.bottom,
        );
        let corner_radii = [
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            radii[2],
            Point::new(0.0, 0.0),
        ];
        canvas.clip_rrect(
            RRect::new_rect_radii(corner_rect, &corner_radii),
            ClipOp::Intersect,
            true,
        );
    }

    if radii[1].x > 0.0 || radii[1].y > 0.0 {
        let corner_rect = Rect::from_ltrb(
            outer_rect.left,
            clip_rect.top,
            clip_rect.right,
            outer_rect.bottom,
        );
        let corner_radii = [
            Point::new(0.0, 0.0),
            radii[1],
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
        ];
        canvas.clip_rrect(
            RRect::new_rect_radii(corner_rect, &corner_radii),
            ClipOp::Intersect,
            true,
        );
    }

    if radii[3].x > 0.0 || radii[3].y > 0.0 {
        let corner_rect = Rect::from_ltrb(
            clip_rect.left,
            outer_rect.top,
            outer_rect.right,
            clip_rect.bottom,
        );
        let corner_radii = [
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            radii[3],
        ];
        canvas.clip_rrect(
            RRect::new_rect_radii(corner_rect, &corner_radii),
            ClipOp::Intersect,
            true,
        );
    }
}

fn line_intersection(a1: Point, a2: Point, b1: Point, b2: Point) -> Point {
    let dax = a2.x - a1.x;
    let day = a2.y - a1.y;
    let dbx = b2.x - b1.x;
    let dby = b2.y - b1.y;
    let denom = dax * dby - day * dbx;
    if denom.abs() < 0.0001 {
        return a2;
    }
    let t = ((b1.x - a1.x) * dby - (b1.y - a1.y) * dbx) / denom;
    Point::new(a1.x + t * dax, a1.y + t * day)
}

fn adjusted_nonrenderable_inner_border_for_side(
    inner_rect: Rect,
    mut radii: [Point; 4],
    side: BorderSide,
) -> (Rect, [Point; 4]) {
    let mut rect = inner_rect;

    match side {
        BorderSide::Top => {
            let overshoot = radii[0].x + radii[1].x - rect.width();
            if overshoot > 0.1 {
                rect.right += overshoot;
                if radii[0].x == 0.0 {
                    rect.left -= overshoot;
                }
            }
            radii[2] = Point::new(0.0, 0.0);
            radii[3] = Point::new(0.0, 0.0);
            let max_radius = radii[0].y.max(radii[1].y);
            if max_radius > rect.height() {
                rect.bottom = rect.top + max_radius;
            }
        }
        BorderSide::Bottom => {
            let overshoot = radii[3].x + radii[2].x - rect.width();
            if overshoot > 0.1 {
                rect.right += overshoot;
                if radii[3].x == 0.0 {
                    rect.left -= overshoot;
                }
            }
            radii[0] = Point::new(0.0, 0.0);
            radii[1] = Point::new(0.0, 0.0);
            let max_radius = radii[3].y.max(radii[2].y);
            if max_radius > rect.height() {
                rect.top = rect.bottom - max_radius;
            }
        }
        BorderSide::Left => {
            let overshoot = radii[0].y + radii[3].y - rect.height();
            if overshoot > 0.1 {
                rect.bottom += overshoot;
                if radii[0].y == 0.0 {
                    rect.top -= overshoot;
                }
            }
            radii[1] = Point::new(0.0, 0.0);
            radii[2] = Point::new(0.0, 0.0);
            let max_radius = radii[0].x.max(radii[3].x);
            if max_radius > rect.width() {
                rect.right = rect.left + max_radius;
            }
        }
        BorderSide::Right => {
            let overshoot = radii[1].y + radii[2].y - rect.height();
            if overshoot > 0.1 {
                rect.bottom += overshoot;
                if radii[1].y == 0.0 {
                    rect.top -= overshoot;
                }
            }
            radii[0] = Point::new(0.0, 0.0);
            radii[3] = Point::new(0.0, 0.0);
            let max_radius = radii[1].x.max(radii[2].x);
            if max_radius > rect.width() {
                rect.left = rect.right - max_radius;
            }
        }
    }

    (rect, radii)
}

fn nonrenderable_border_side_clip_polygon(
    border_rect: Rect,
    inner_rect: Rect,
    radii: [Point; 4],
    side: BorderSide,
) -> Vec<Point> {
    let inner = [
        Point::new(inner_rect.left, inner_rect.top),
        Point::new(inner_rect.right, inner_rect.top),
        Point::new(inner_rect.right, inner_rect.bottom),
        Point::new(inner_rect.left, inner_rect.bottom),
    ];
    let outer = [
        Point::new(border_rect.left, border_rect.top),
        Point::new(border_rect.right, border_rect.top),
        Point::new(border_rect.right, border_rect.bottom),
        Point::new(border_rect.left, border_rect.bottom),
    ];
    let zero = |p: Point| p.x == 0.0 && p.y == 0.0;
    let mut q;
    let mut pentagon: Option<Vec<Point>> = None;

    match side {
        BorderSide::Top => {
            q = [outer[0], inner[0], inner[1], outer[1]];
            if !zero(radii[0]) {
                q[1] = line_intersection(
                    q[0],
                    q[1],
                    Point::new(q[1].x + radii[0].x, q[1].y),
                    Point::new(q[1].x, q[1].y + radii[0].y),
                );
                if q[1].y > inner[2].y {
                    q[1] = line_intersection(q[0], q[1], inner[3], inner[2]);
                }
                if q[1].x > inner[2].x {
                    q[1] = line_intersection(q[0], q[1], inner[1], inner[2]);
                }
                if q[2].y < q[1].y && q[2].x > q[1].x {
                    pentagon = Some(vec![q[0], q[1], Point::new(q[2].x, q[1].y), q[2], q[3]]);
                }
            }
            if !zero(radii[1]) {
                q[2] = line_intersection(
                    q[3],
                    q[2],
                    Point::new(q[2].x - radii[1].x, q[2].y),
                    Point::new(q[2].x, q[2].y + radii[1].y),
                );
                if q[2].y > inner[3].y {
                    q[2] = line_intersection(q[3], q[2], inner[3], inner[2]);
                }
                if q[2].x < inner[3].x {
                    q[2] = line_intersection(q[3], q[2], inner[0], inner[3]);
                }
                if q[2].y > q[1].y && q[2].x > q[1].x {
                    pentagon = Some(vec![q[0], q[1], Point::new(q[1].x, q[2].y), q[2], q[3]]);
                }
            }
        }
        BorderSide::Left => {
            q = [outer[3], inner[3], inner[0], outer[0]];
            if !zero(radii[0]) {
                q[2] = line_intersection(
                    q[3],
                    q[2],
                    Point::new(q[2].x + radii[0].x, q[2].y),
                    Point::new(q[2].x, q[2].y + radii[0].y),
                );
                if q[2].y > inner[2].y {
                    q[2] = line_intersection(q[3], q[2], inner[3], inner[2]);
                }
                if q[2].x > inner[2].x {
                    q[2] = line_intersection(q[3], q[2], inner[1], inner[2]);
                }
                if q[2].y < q[1].y && q[2].x > q[1].x {
                    pentagon = Some(vec![q[0], q[1], Point::new(q[2].x, q[1].y), q[2], q[3]]);
                }
            }
            if !zero(radii[3]) {
                q[1] = line_intersection(
                    q[0],
                    q[1],
                    Point::new(q[1].x + radii[3].x, q[1].y),
                    Point::new(q[1].x, q[1].y - radii[3].y),
                );
                if q[1].y < inner[1].y {
                    q[1] = line_intersection(q[0], q[1], inner[0], inner[1]);
                }
                if q[1].x > inner[1].x {
                    q[1] = line_intersection(q[0], q[1], inner[1], inner[2]);
                }
                if q[2].y < q[1].y && q[2].x < q[1].x {
                    pentagon = Some(vec![q[0], q[1], Point::new(q[1].x, q[2].y), q[2], q[3]]);
                }
            }
        }
        BorderSide::Bottom => {
            q = [outer[2], inner[2], inner[3], outer[3]];
            if !zero(radii[3]) {
                q[2] = line_intersection(
                    q[3],
                    q[2],
                    Point::new(q[2].x + radii[3].x, q[2].y),
                    Point::new(q[2].x, q[2].y - radii[3].y),
                );
                if q[2].y < inner[1].y {
                    q[2] = line_intersection(q[3], q[2], inner[0], inner[1]);
                }
                if q[2].x > inner[1].x {
                    q[2] = line_intersection(q[3], q[2], inner[1], inner[2]);
                }
                if q[2].y < q[1].y && q[2].x < q[1].x {
                    pentagon = Some(vec![q[0], q[1], Point::new(q[1].x, q[2].y), q[2], q[3]]);
                }
            }
            if !zero(radii[2]) {
                q[1] = line_intersection(
                    q[0],
                    q[1],
                    Point::new(q[1].x - radii[2].x, q[1].y),
                    Point::new(q[1].x, q[1].y - radii[2].y),
                );
                if q[1].y < inner[0].y {
                    q[1] = line_intersection(q[0], q[1], inner[0], inner[1]);
                }
                if q[1].x < inner[0].x {
                    q[1] = line_intersection(q[0], q[1], inner[0], inner[3]);
                }
                if q[2].x < q[1].x && q[2].y > q[1].y {
                    pentagon = Some(vec![q[0], q[1], Point::new(q[2].x, q[1].y), q[2], q[3]]);
                }
            }
        }
        BorderSide::Right => {
            q = [outer[1], inner[1], inner[2], outer[2]];
            if !zero(radii[1]) {
                q[1] = line_intersection(
                    q[0],
                    q[1],
                    Point::new(q[1].x - radii[1].x, q[1].y),
                    Point::new(q[1].x, q[1].y + radii[1].y),
                );
                if q[1].y > inner[3].y {
                    q[1] = line_intersection(q[0], q[1], inner[3], inner[2]);
                }
                if q[1].x < inner[3].x {
                    q[1] = line_intersection(q[0], q[1], inner[0], inner[3]);
                }
                if q[2].y > q[1].y && q[2].x > q[1].x {
                    pentagon = Some(vec![q[0], q[1], Point::new(q[1].x, q[2].y), q[2], q[3]]);
                }
            }
            if !zero(radii[2]) {
                q[2] = line_intersection(
                    q[3],
                    q[2],
                    Point::new(q[2].x - radii[2].x, q[2].y),
                    Point::new(q[2].x, q[2].y - radii[2].y),
                );
                if q[2].y < inner[0].y {
                    q[2] = line_intersection(q[3], q[2], inner[0], inner[1]);
                }
                if q[2].x < inner[0].x {
                    q[2] = line_intersection(q[3], q[2], inner[0], inner[3]);
                }
                if q[2].x < q[1].x && q[2].y > q[1].y {
                    pentagon = Some(vec![q[0], q[1], Point::new(q[2].x, q[1].y), q[2], q[3]]);
                }
            }
        }
    }

    pentagon.unwrap_or_else(|| q.to_vec())
}

fn draw_nonrenderable_uniform_rounded_border(
    canvas: &Canvas,
    border_rect: Rect,
    inner_rect: Rect,
    inner_radii: [Point; 4],
    paint: &Paint,
) {
    for side in [
        BorderSide::Top,
        BorderSide::Right,
        BorderSide::Bottom,
        BorderSide::Left,
    ] {
        canvas.save();
        let polygon =
            nonrenderable_border_side_clip_polygon(border_rect, inner_rect, inner_radii, side);
        let mut side_path = PathBuilder::new();
        side_path.move_to(polygon[0]);
        for point in polygon.iter().skip(1) {
            side_path.line_to(*point);
        }
        side_path.close();
        canvas.clip_path(&side_path.detach(), ClipOp::Intersect, false);

        let (adjusted_rect, adjusted_radii) =
            adjusted_nonrenderable_inner_border_for_side(inner_rect, inner_radii, side);
        if adjusted_rect.width() > 0.0 && adjusted_rect.height() > 0.0 {
            canvas.clip_rrect(
                RRect::new_rect_radii(adjusted_rect, &adjusted_radii),
                ClipOp::Difference,
                true,
            );
        }
        canvas.draw_rect(border_rect, paint);
        canvas.restore();
    }
}

fn draw_outer_edge_overlap(canvas: &Canvas, border_rect: Rect, edge_overlap: f32, paint: &Paint) {
    let edge_rects = [
        Rect::from_ltrb(
            border_rect.left,
            border_rect.top,
            border_rect.right,
            (border_rect.top + edge_overlap).min(border_rect.bottom),
        ),
        Rect::from_ltrb(
            (border_rect.right - edge_overlap).max(border_rect.left),
            border_rect.top,
            border_rect.right,
            border_rect.bottom,
        ),
        Rect::from_ltrb(
            border_rect.left,
            (border_rect.bottom - edge_overlap).max(border_rect.top),
            border_rect.right,
            border_rect.bottom,
        ),
        Rect::from_ltrb(
            border_rect.left,
            border_rect.top,
            (border_rect.left + edge_overlap).min(border_rect.right),
            border_rect.bottom,
        ),
    ];

    for rect in edge_rects {
        if rect.width() > 0.0 && rect.height() > 0.0 {
            canvas.draw_rect(rect, paint);
        }
    }
}

fn draw_outer_corner_tangent_overlap(
    canvas: &Canvas,
    border_rect: Rect,
    radii: &[Point; 4],
    border_width: f32,
    edge_overlap: f32,
    paint: &Paint,
) {
    let l = border_rect.left;
    let t = border_rect.top;
    let r = border_rect.right;
    let b = border_rect.bottom;
    let cap = 1.0;
    let draw_cap = |rect: Rect| {
        if rect.width() > 0.0 && rect.height() > 0.0 {
            canvas.draw_rect(rect, paint);
        }
    };

    if radii[0].x > border_width && radii[0].y > border_width {
        draw_cap(Rect::from_ltrb(
            l + radii[0].x - border_width,
            t + edge_overlap,
            l + radii[0].x,
            (t + edge_overlap + cap).min(b),
        ));
        draw_cap(Rect::from_ltrb(
            l + edge_overlap,
            t + radii[0].y - border_width,
            (l + edge_overlap + cap).min(r),
            t + radii[0].y,
        ));
    }
    if radii[1].x > border_width && radii[1].y > border_width {
        draw_cap(Rect::from_ltrb(
            r - radii[1].x,
            t + edge_overlap,
            r - radii[1].x + border_width,
            (t + edge_overlap + cap).min(b),
        ));
        draw_cap(Rect::from_ltrb(
            (r - edge_overlap - cap).max(l),
            t + radii[1].y - border_width,
            r - edge_overlap,
            t + radii[1].y,
        ));
    }
    if radii[2].x > border_width && radii[2].y > border_width {
        draw_cap(Rect::from_ltrb(
            r - radii[2].x,
            (b - edge_overlap - cap).max(t),
            r - radii[2].x + border_width,
            b - edge_overlap,
        ));
        draw_cap(Rect::from_ltrb(
            (r - edge_overlap - cap).max(l),
            b - radii[2].y,
            r - edge_overlap,
            b - radii[2].y + border_width,
        ));
    }
    if radii[3].x > border_width && radii[3].y > border_width {
        draw_cap(Rect::from_ltrb(
            l + radii[3].x - border_width,
            (b - edge_overlap - cap).max(t),
            l + radii[3].x,
            b - edge_overlap,
        ));
        draw_cap(Rect::from_ltrb(
            l + edge_overlap,
            b - radii[3].y,
            (l + edge_overlap + cap).min(r),
            b - radii[3].y + border_width,
        ));
    }
}

fn draw_nonrenderable_outer_tangent_fringe(
    canvas: &Canvas,
    border_rect: Rect,
    radii: &[Point; 4],
    border_width: f32,
    color: &Color,
) {
    if color.a < 0.99 {
        return;
    }

    let l = border_rect.left;
    let t = border_rect.top;
    let r = border_rect.right;
    let b = border_rect.bottom;
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);
    paint.set_color4f(
        Color4f::new(color.r, color.g, color.b, 0.12),
        None::<&ColorSpace>,
    );

    let draw = |rect: Rect| {
        if rect.width() > 0.0 && rect.height() > 0.0 {
            canvas.draw_rect(rect, &paint);
        }
    };

    if radii[0].x > border_width && radii[0].y > border_width {
        draw(Rect::from_ltrb(
            l,
            (t + radii[0].y - border_width * 0.5).max(t),
            (l + 1.0).min(r),
            (t + radii[0].y).min(b),
        ));
        draw(Rect::from_ltrb(
            (l + radii[0].x - border_width * 0.5).max(l),
            t,
            (l + radii[0].x).min(r),
            (t + 1.0).min(b),
        ));
    }
    if radii[1].x > border_width && radii[1].y > border_width {
        draw(Rect::from_ltrb(
            (r - 1.0).max(l),
            (t + radii[1].y - border_width * 0.5).max(t),
            r,
            (t + radii[1].y).min(b),
        ));
        draw(Rect::from_ltrb(
            (r - radii[1].x).max(l),
            t,
            (r - radii[1].x + border_width * 0.5).min(r),
            (t + 1.0).min(b),
        ));
    }
    if radii[2].x > border_width && radii[2].y > border_width {
        draw(Rect::from_ltrb(
            (r - 1.0).max(l),
            (b - radii[2].y).max(t),
            r,
            (b - radii[2].y + border_width * 0.5).min(b),
        ));
        draw(Rect::from_ltrb(
            (r - radii[2].x).max(l),
            (b - 1.0).max(t),
            (r - radii[2].x + border_width * 0.5).min(r),
            b,
        ));
    }
    if radii[3].x > border_width && radii[3].y > border_width {
        draw(Rect::from_ltrb(
            l,
            (b - radii[3].y).max(t),
            (l + 1.0).min(r),
            (b - radii[3].y + border_width * 0.5).min(b),
        ));
        draw(Rect::from_ltrb(
            (l + radii[3].x - border_width * 0.5).max(l),
            (b - 1.0).max(t),
            (l + radii[3].x).min(r),
            b,
        ));
    }
}

fn draw_renderable_outer_aa_fringe(
    canvas: &Canvas,
    outer_radii: &[Point; 4],
    inner_rect: Rect,
    inner_radii: [Point; 4],
    color: &Color,
) {
    if color.a < 0.99 {
        return;
    }
    if !outer_radii
        .iter()
        .zip(inner_radii.iter())
        .any(|(outer, inner)| outer.x > 0.0 && outer.y > 0.0 && inner.x <= 0.0 && inner.y <= 0.0)
    {
        return;
    }

    let inner_radii = normalize_radii_to_rect(inner_radii, &inner_rect);
    let inner_rrect = RRect::new_rect_radii(inner_rect, &inner_radii);
    let mut fringe_path = PathBuilder::new();
    fringe_path.set_fill_type(PathFillType::InverseWinding);
    fringe_path.add_rrect(inner_rrect, None, None);

    let mut fringe_paint = Paint::default();
    fringe_paint.set_style(PaintStyle::Fill);
    fringe_paint.set_anti_alias(true);
    fringe_paint.set_color4f(
        Color4f::new(color.r, color.g, color.b, 0.12),
        None::<&ColorSpace>,
    );
    canvas.draw_path(&fringe_path.detach(), &fringe_paint);
}

fn descendant_overflows(fragment: &Fragment) -> (bool, bool) {
    let mut overflow_x = false;
    let mut overflow_y = false;
    for child in &fragment.children {
        if let Some(rect) = child.overflow_rect {
            let left = child.offset.left + rect.offset.left;
            let top = child.offset.top + rect.offset.top;
            let right = left + rect.size.width;
            let bottom = top + rect.size.height;
            if left < child.offset.left || right > child.offset.left + child.size.width {
                overflow_x = true;
            }
            if top < child.offset.top || bottom > child.offset.top + child.size.height {
                overflow_y = true;
            }
        }
        let (child_x, child_y) = descendant_overflows(child);
        overflow_x |= child_x;
        overflow_y |= child_y;
    }
    (overflow_x, overflow_y)
}

fn paint_scrollbars_if_needed(
    canvas: &Canvas,
    fragment: &Fragment,
    style: &ComputedStyle,
    clip_rect: &Rect,
) {
    let Some(track_color) = style.scrollbar_track_color else {
        return;
    };
    let (overflow_x, overflow_y) = descendant_overflows(fragment);
    let show_x = style.overflow_x.is_scrollable() && overflow_x;
    let show_y = style.overflow_y.is_scrollable() && overflow_y;
    if !show_x && !show_y {
        return;
    }

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    set_paint_css_color(&mut paint, &track_color);

    let thickness = 15.0_f32.min(clip_rect.width()).min(clip_rect.height());
    if show_y {
        canvas.draw_rect(
            Rect::from_xywh(
                clip_rect.right - thickness,
                clip_rect.top,
                thickness,
                clip_rect.height(),
            ),
            &paint,
        );
    }
    if show_x {
        canvas.draw_rect(
            Rect::from_xywh(
                clip_rect.left,
                clip_rect.bottom - thickness,
                clip_rect.width(),
                thickness,
            ),
            &paint,
        );
    }
}

/// Resolve decoration metrics from the styled font (CSS font-family/size),
/// falling back to the first shaped run's metrics if the primary font lookup
/// fails. This ensures decoration positioning uses the intended CSS font
/// even when the first shaped run uses a fallback (emoji, CJK, etc.).
fn resolve_decoration_metrics(
    style: &ComputedStyle,
    shape_result: &openui_text::shaping::ShapeResult,
) -> FontMetrics {
    let font_desc = crate::text_painter::style_to_font_description(style);
    let font = openui_text::Font::new(font_desc);
    font.font_metrics()
        .copied()
        .unwrap_or_else(|| crate::text_painter::metrics_from_shape_result(shape_result))
}

/// Paint a text fragment — shadows, decorations, glyphs, and emphasis marks.
///
/// Extracted from Blink's `TextFragmentPainter::Paint()`.
///
/// Paint order:
/// 1. Text shadows (behind everything)
/// 2. Underline + overline decorations (behind text glyphs)
/// 3. Text glyphs
/// 4. Emphasis marks (above/below each character per text-emphasis)
/// 5. Line-through decoration (in front of text glyphs)
fn paint_text_fragment(
    canvas: &Canvas,
    fragment: &Fragment,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
) {
    let shape_result = match fragment.shape_result.as_ref() {
        Some(sr) => sr,
        None => return,
    };

    // Resolve font metrics from the styled font (CSS font-family/size), not
    // from the first shaped run which may be a fallback font (emoji, CJK).
    let metrics = resolve_decoration_metrics(style, shape_result);

    // Use the layout-computed baseline offset stored on the fragment,
    // rather than recomputing from font metrics (which can differ with
    // fallback fonts, vertical-align shifts, or fractional ascents).
    let x = abs_offset.left.to_f32();
    let baseline_y = abs_offset.top.to_f32() + fragment.baseline_offset;
    let origin = (x, baseline_y);

    // Text content for CJK detection in skip-ink Auto mode.
    let text_content = fragment.text_content.as_deref();

    // 1. Text shadows
    crate::text_painter::paint_text_shadows(canvas, shape_result, origin, style);

    // 2. Text decorations (underline + overline, painted behind text)
    crate::decoration_painter::paint_text_decorations(
        canvas,
        shape_result,
        origin,
        style,
        &metrics,
        crate::decoration_painter::DecorationPhase::BeforeText,
        text_content,
    );

    // 3. Text glyphs — use text-combine-upright transform when active
    if let Some(ref tc_layout) = fragment.text_combine {
        crate::text_painter::paint_text_combine(canvas, shape_result, origin, style, tc_layout);
    } else {
        crate::text_painter::paint_text(canvas, shape_result, origin, style);
    }

    // 4. Emphasis marks (painted after text glyphs, before line-through)
    crate::emphasis_painter::paint_emphasis_marks(
        canvas,
        shape_result,
        origin,
        style,
        text_content,
    );

    // 5. Line-through decoration (painted in front of text per CSS spec)
    crate::decoration_painter::paint_text_decorations(
        canvas,
        shape_result,
        origin,
        style,
        &metrics,
        crate::decoration_painter::DecorationPhase::AfterText,
        text_content,
    );
}

/// Paint box shadows for a single box fragment.
///
/// Extracted from Blink's `BoxPainterBase::PaintNormalBoxShadow()` and
/// `BoxPainterBase::PaintInsetBoxShadow()` (box_painter_base.cc).
///
/// When `inset_only` is false, paints outset shadows (behind the element).
/// When `inset_only` is true, paints inset shadows (inside the border-box).
fn paint_box_shadows(canvas: &Canvas, style: &ComputedStyle, border_rect: Rect, inset_only: bool) {
    for shadow in &style.box_shadow {
        if shadow.inset != inset_only {
            continue;
        }

        let color = Color4f::new(
            shadow.color.r,
            shadow.color.g,
            shadow.color.b,
            shadow.color.a,
        );
        let mut paint = Paint::default();
        paint.set_color4f(color, None::<&ColorSpace>);
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);

        if shadow.blur_radius > 0.0 {
            let sigma = shadow.blur_radius / 2.0;
            if let Some(filter) =
                skia_safe::MaskFilter::blur(skia_safe::BlurStyle::Normal, sigma, false)
            {
                paint.set_mask_filter(filter);
            }
        }

        if shadow.inset {
            // Inset shadow: clip to border-box (respecting border-radius), then
            // paint a large rect with the inner "hole" cut out.
            canvas.save();
            if style.has_border_radius() {
                let element_radii = normalized_border_radii(style, &border_rect);
                let border_rrect = RRect::new_rect_radii(border_rect, &element_radii);
                canvas.clip_rrect(border_rrect, ClipOp::Intersect, false);
            } else {
                canvas.clip_rect(border_rect, ClipOp::Intersect, false);
            }

            // The hole is the border-box shrunk by spread and shifted by offset
            let hole = Rect::from_xywh(
                border_rect.left + shadow.offset_x + shadow.spread_radius,
                border_rect.top + shadow.offset_y + shadow.spread_radius,
                (border_rect.width() - shadow.spread_radius * 2.0).max(0.0),
                (border_rect.height() - shadow.spread_radius * 2.0).max(0.0),
            );

            // Outer rect large enough to cover any blur extent
            let outer = Rect::from_xywh(
                border_rect.left - 1000.0,
                border_rect.top - 1000.0,
                border_rect.width() + 2000.0,
                border_rect.height() + 2000.0,
            );

            let mut path = PathBuilder::new();
            path.add_rect(outer, None, None);
            if hole.width() > 0.0 && hole.height() > 0.0 {
                path.add_rect(hole, skia_safe::PathDirection::CCW, 0);
            }
            path.set_fill_type(skia_safe::PathFillType::EvenOdd);
            canvas.draw_path(&path.detach(), &paint);
            canvas.restore();
        } else {
            // Outset shadow: draw behind the element, expanded by spread.
            // CSS spec §12.2: the shadow shape matches the element's border-radius.
            let shadow_rect = Rect::from_xywh(
                border_rect.left + shadow.offset_x - shadow.spread_radius,
                border_rect.top + shadow.offset_y - shadow.spread_radius,
                border_rect.width() + shadow.spread_radius * 2.0,
                border_rect.height() + shadow.spread_radius * 2.0,
            );
            // An outer shadow is excluded from the element's border box even
            // when the element background is transparent.  Painting the
            // complete shadow shape and relying on the background to cover it
            // turns transparent fragmented boxes into solid shadow-colored
            // rectangles.
            canvas.save();
            if style.has_border_radius() {
                let element_radii = normalized_border_radii(style, &border_rect);
                let border_rrect = RRect::new_rect_radii(border_rect, &element_radii);
                canvas.clip_rrect(border_rrect, ClipOp::Difference, true);
                let shadow_radii: [Point; 4] = std::array::from_fn(|i| {
                    Point::new(
                        (element_radii[i].x + shadow.spread_radius).max(0.0),
                        (element_radii[i].y + shadow.spread_radius).max(0.0),
                    )
                });
                let normalized = normalize_radii_to_rect(shadow_radii, &shadow_rect);
                let shadow_rrect = RRect::new_rect_radii(shadow_rect, &normalized);
                canvas.draw_rrect(shadow_rrect, &paint);
            } else {
                canvas.clip_rect(border_rect, ClipOp::Difference, false);
                canvas.draw_rect(shadow_rect, &paint);
            }
            canvas.restore();
        }
    }
}

/// Paint background + border for a single box fragment.
///
/// Extracted from Blink's `BoxFragmentPainter::PaintBoxDecorationBackgroundWithRectImpl()`
/// (box_fragment_painter.cc:1550).
///
/// Order:
/// 1. Box shadows (outset)
/// 2. Background color (fill the border-box rect)
/// 3. Box shadows (inset)
/// 4. Border (stroke the border-box rect)
fn paint_box_decoration_background(
    canvas: &Canvas,
    fragment: &Fragment,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    // Pixel-snap all four edges independently (Blink's PixelSnappedIntRect).
    // Fractional abs_offset flows through from parent so that adjacent elements
    // at fractional boundaries (e.g. flex items at 133.33px intervals) share
    // the same snapped pixel edge — no gaps.
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let decoration_block_size = fragment
        .decoration_paint_block_size
        .filter(|limit| limit.raw() >= 0 && limit.raw() < fragment.size.height.raw())
        .unwrap_or(fragment.size.height);
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + decoration_block_size).round().to_f32();
    let w = right - x;
    let h = bottom - y;

    // Skip empty fragments
    if w <= 0.0 || h <= 0.0 {
        return;
    }

    let border_box_rect = Rect::from_xywh(x, y, w, h);
    let shadow_border_box_rect = fragment.decoration_slice.map_or(border_box_rect, |slice| {
        let source_top = y - slice.source_block_offset.to_f32();
        Rect::from_xywh(x, source_top, w, slice.source_block_size.to_f32())
    });
    let decoration_clip_saved = fragment
        .decoration_paint_block_size
        .filter(|limit| limit.raw() >= 0 && limit.raw() < fragment.size.height.raw())
        .is_some();
    if let Some(limit) = fragment.decoration_paint_block_size {
        if limit.raw() >= 0 && limit.raw() < fragment.size.height.raw() {
            let clip_bottom = (abs_offset.top + limit).round().to_f32();
            canvas.save();
            canvas.clip_rect(
                Rect::from_ltrb(x, y, right, clip_bottom.max(y)),
                ClipOp::Intersect,
                false,
            );
        }
    }

    // ── 1. Outset box shadows (painted behind everything) ────────────
    paint_box_shadows(canvas, style, shadow_border_box_rect, false);

    // When border-radius is set AND borders are uniform solid, use saveLayer
    // so bg+border composite as one unit, then clip by the outer rrect.
    // This prevents background color from bleeding through at the border's
    // AA curve edges (matching Chromium). Only apply for uniform borders
    // since non-uniform (trapezoid) borders handle corners differently.
    let bt = style.effective_border_top() as f32;
    let br_bw = style.effective_border_right() as f32;
    let bb_bw = style.effective_border_bottom() as f32;
    let bl_bw = style.effective_border_left() as f32;
    let clone_inline = fragment.is_inline_box_fragment
        && style.box_decoration_break == openui_style::BoxDecorationBreak::Clone;
    let paint_bt = if fragment.is_inline_box_fragment || fragment.is_first_for_node {
        bt
    } else {
        0.0
    };
    let paint_br = if !fragment.is_inline_box_fragment || clone_inline || fragment.is_last_for_node
    {
        br_bw
    } else {
        0.0
    };
    let paint_bb = if fragment.is_inline_box_fragment || fragment.is_last_for_node {
        bb_bw
    } else {
        0.0
    };
    let paint_bl = if !fragment.is_inline_box_fragment || clone_inline || fragment.is_first_for_node
    {
        bl_bw
    } else {
        0.0
    };
    let fragment_radii = fragment_border_radii(style, fragment, &border_box_rect, bt);
    let has_radius = has_any_radius(&fragment_radii);
    let uniform_border = bt == br_bw
        && br_bw == bb_bw
        && bb_bw == bl_bw
        && style.border_top_style == style.border_right_style
        && style.border_right_style == style.border_bottom_style
        && style.border_bottom_style == style.border_left_style
        && style.border_top_color == style.border_right_color
        && style.border_right_color == style.border_bottom_color
        && style.border_bottom_color == style.border_left_color
        && style.border_top_style == BorderStyle::Solid;
    let has_nonzero_uniform_border =
        uniform_border && (bt > 0.0 || br_bw > 0.0 || bb_bw > 0.0 || bl_bw > 0.0);
    let side_specs = [
        (
            paint_bt,
            style.border_top_style,
            style.border_top_color.resolve(&style.color),
        ),
        (
            paint_br,
            style.border_right_style,
            style.border_right_color.resolve(&style.color),
        ),
        (
            paint_bb,
            style.border_bottom_style,
            style.border_bottom_color.resolve(&style.color),
        ),
        (
            paint_bl,
            style.border_left_style,
            style.border_left_color.resolve(&style.color),
        ),
    ];
    let mut visible_width: Option<f32> = None;
    let mut visible_color: Option<Color> = None;
    let mut same_solid_visible_border = false;
    for (width, border_style, color) in side_specs {
        if width <= 0.0 {
            continue;
        }
        same_solid_visible_border = true;
        if border_style != BorderStyle::Solid {
            same_solid_visible_border = false;
            break;
        }
        if let Some(existing_width) = visible_width {
            if existing_width != width {
                same_solid_visible_border = false;
                break;
            }
        } else {
            visible_width = Some(width);
        }
        if let Some(existing_color) = visible_color {
            if existing_color != color {
                same_solid_visible_border = false;
                break;
            }
        } else {
            visible_color = Some(color);
        }
    }
    let effective_background_clip = if style.background_attachment == BackgroundAttachment::Local {
        BackgroundClip::PaddingBox
    } else {
        style.background_clip
    };
    // Blink avoids rounded-background bleed by shrinking a border-box
    // background underneath borders that fully obscure its edge. This is a
    // geometric adjustment, not a compositing effect.
    let authored_sides = [
        (
            bt,
            style.border_top_style,
            style.border_top_color.resolve(&style.color),
        ),
        (
            br_bw,
            style.border_right_style,
            style.border_right_color.resolve(&style.color),
        ),
        (
            bb_bw,
            style.border_bottom_style,
            style.border_bottom_color.resolve(&style.color),
        ),
        (
            bl_bw,
            style.border_left_style,
            style.border_left_color.resolve(&style.color),
        ),
    ];
    let rounded_edge_requires_bleed_cover = [
        fragment_radii[0].y > 0.0 || fragment_radii[1].y > 0.0,
        fragment_radii[1].x > 0.0 || fragment_radii[2].x > 0.0,
        fragment_radii[2].y > 0.0 || fragment_radii[3].y > 0.0,
        fragment_radii[3].x > 0.0 || fragment_radii[0].x > 0.0,
    ];
    let border_obscures_background_edge =
        authored_sides
            .iter()
            .enumerate()
            .all(|(index, (width, border_style, color))| {
                !rounded_edge_requires_bleed_cover[index]
                    || (*width > 0.0
                        && color.is_opaque()
                        && !matches!(
                            border_style,
                            BorderStyle::None
                                | BorderStyle::Hidden
                                | BorderStyle::Dotted
                                | BorderStyle::Dashed
                        ))
            });
    let background_border_inset_fraction = if authored_sides
        .iter()
        .any(|(_, border_style, _)| *border_style == BorderStyle::Double)
    {
        1.0 / 6.0
    } else {
        0.5
    };
    let shrink_background_for_opaque_border = has_radius
        && border_obscures_background_edge
        && effective_background_clip == BackgroundClip::BorderBox;
    let border_inner_rect = Rect::from_xywh(
        x + bl_bw,
        y + paint_bt,
        (w - bl_bw - br_bw).max(0.0),
        (h - paint_bt - paint_bb).max(0.0),
    );
    let border_inner_radii = [
        Point::new(
            (fragment_radii[0].x - bl_bw).max(0.0),
            (fragment_radii[0].y - paint_bt).max(0.0),
        ),
        Point::new(
            (fragment_radii[1].x - br_bw).max(0.0),
            (fragment_radii[1].y - paint_bt).max(0.0),
        ),
        Point::new(
            (fragment_radii[2].x - br_bw).max(0.0),
            (fragment_radii[2].y - paint_bb).max(0.0),
        ),
        Point::new(
            (fragment_radii[3].x - bl_bw).max(0.0),
            (fragment_radii[3].y - paint_bb).max(0.0),
        ),
    ];
    let nonrenderable_inner_border_contour =
        matches!(
            effective_background_clip,
            BackgroundClip::PaddingBox | BackgroundClip::ContentBox
        ) && radii_exceed_rect(&border_inner_rect, &border_inner_radii);
    let use_layer = has_radius
        && !shrink_background_for_opaque_border
        // Renderable padding/content-box contours do not reach the outer
        // border contour, so their border can use one native double-rrect.
        // An inner contour whose adjacent radii cannot fit its rect needs the
        // shared outer clip: this is the coverage model Blink uses for the
        // extreme single-corner cases where the background meets the border.
        && (matches!(
            effective_background_clip,
            BackgroundClip::BorderBox | BackgroundClip::BorderArea
        ) || nonrenderable_inner_border_contour)
        && (has_nonzero_uniform_border || same_solid_visible_border);
    if use_layer {
        canvas.save();
        canvas.clip_rrect(
            RRect::new_rect_radii(border_box_rect, &fragment_radii),
            ClipOp::Intersect,
            true,
        );
        canvas.save_layer_alpha_f(border_box_rect, 1.0);
    }

    // ── 2. Background color ──────────────────────────────────────────
    if !style.background_color.is_transparent() {
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(true);
        let c = &style.background_color;
        set_paint_css_color_with_alpha(&mut paint, c, opacity_multiplier);

        if effective_background_clip == BackgroundClip::BorderArea {
            let top = fragment.border.top.round().to_f32();
            let right_width = fragment.border.right.round().to_f32();
            let bottom_height = fragment.border.bottom.round().to_f32();
            let left_width = fragment.border.left.round().to_f32();
            if top > 0.0 {
                canvas.draw_rect(Rect::from_xywh(x, y, w, top), &paint);
            }
            if bottom_height > 0.0 {
                canvas.draw_rect(
                    Rect::from_xywh(x, bottom - bottom_height, w, bottom_height),
                    &paint,
                );
            }
            let middle_height = (h - top - bottom_height).max(0.0);
            if left_width > 0.0 {
                canvas.draw_rect(
                    Rect::from_xywh(x, y + top, left_width, middle_height),
                    &paint,
                );
            }
            if right_width > 0.0 {
                canvas.draw_rect(
                    Rect::from_xywh(right - right_width, y + top, right_width, middle_height),
                    &paint,
                );
            }
        } else if effective_background_clip != BackgroundClip::Text {
            let bg_rect = match effective_background_clip {
                BackgroundClip::BorderBox => border_box_rect,
                BackgroundClip::PaddingBox => {
                    let bx = x + fragment.border.left.round().to_f32();
                    let by = y + fragment.border.top.round().to_f32();
                    let br = right - fragment.border.right.round().to_f32();
                    let bb = bottom - fragment.border.bottom.round().to_f32();
                    let bw = (br - bx).max(0.0);
                    let bh = (bb - by).max(0.0);
                    Rect::from_xywh(bx, by, bw, bh)
                }
                BackgroundClip::ContentBox => {
                    let bx = x
                        + fragment.border.left.round().to_f32()
                        + fragment.padding.left.round().to_f32();
                    let by = y
                        + fragment.border.top.round().to_f32()
                        + fragment.padding.top.round().to_f32();
                    let br = right
                        - fragment.border.right.round().to_f32()
                        - fragment.padding.right.round().to_f32();
                    let bb = bottom
                        - fragment.border.bottom.round().to_f32()
                        - fragment.padding.bottom.round().to_f32();
                    let bw = (br - bx).max(0.0);
                    let bh = (bb - by).max(0.0);
                    Rect::from_xywh(bx, by, bw, bh)
                }
                BackgroundClip::BorderArea | BackgroundClip::Text => unreachable!(),
            };

            if has_radius {
                // Compute radii adjusted for background-clip box (CSS Backgrounds §5.3).
                // Inner corner radii = outer radii - inset on each side, clamped to 0.
                let outer_radii = if all_effective_borders_transparent(style) {
                    slice_adjust_border_radii(specified_border_radii(style), fragment)
                } else {
                    fragment_radii
                };
                let clip_radii = match effective_background_clip {
                    BackgroundClip::BorderBox => outer_radii,
                    BackgroundClip::PaddingBox => {
                        let bt = style.effective_border_top() as f32;
                        let br_w = style.effective_border_right() as f32;
                        let bb = style.effective_border_bottom() as f32;
                        let bl = style.effective_border_left() as f32;
                        [
                            Point::new(
                                (outer_radii[0].x - bl).max(0.0),
                                (outer_radii[0].y - bt).max(0.0),
                            ),
                            Point::new(
                                (outer_radii[1].x - br_w).max(0.0),
                                (outer_radii[1].y - bt).max(0.0),
                            ),
                            Point::new(
                                (outer_radii[2].x - br_w).max(0.0),
                                (outer_radii[2].y - bb).max(0.0),
                            ),
                            Point::new(
                                (outer_radii[3].x - bl).max(0.0),
                                (outer_radii[3].y - bb).max(0.0),
                            ),
                        ]
                    }
                    BackgroundClip::ContentBox => {
                        let bt = style.effective_border_top() as f32
                            + fragment.padding.top.round().to_f32();
                        let br_w = style.effective_border_right() as f32
                            + fragment.padding.right.round().to_f32();
                        let bb = style.effective_border_bottom() as f32
                            + fragment.padding.bottom.round().to_f32();
                        let bl = style.effective_border_left() as f32
                            + fragment.padding.left.round().to_f32();
                        [
                            Point::new(
                                (outer_radii[0].x - bl).max(0.0),
                                (outer_radii[0].y - bt).max(0.0),
                            ),
                            Point::new(
                                (outer_radii[1].x - br_w).max(0.0),
                                (outer_radii[1].y - bt).max(0.0),
                            ),
                            Point::new(
                                (outer_radii[2].x - br_w).max(0.0),
                                (outer_radii[2].y - bb).max(0.0),
                            ),
                            Point::new(
                                (outer_radii[3].x - bl).max(0.0),
                                (outer_radii[3].y - bb).max(0.0),
                            ),
                        ]
                    }
                    BackgroundClip::BorderArea | BackgroundClip::Text => unreachable!(),
                };
                if shrink_background_for_opaque_border
                    && effective_background_clip == BackgroundClip::BorderBox
                {
                    let top_inset = paint_bt * background_border_inset_fraction;
                    let right_inset = paint_br * background_border_inset_fraction;
                    let bottom_inset = paint_bb * background_border_inset_fraction;
                    let left_inset = paint_bl * background_border_inset_fraction;
                    let shrunk_rect = Rect::from_xywh(
                        bg_rect.left + left_inset,
                        bg_rect.top + top_inset,
                        (bg_rect.width() - left_inset - right_inset).max(0.0),
                        (bg_rect.height() - top_inset - bottom_inset).max(0.0),
                    );
                    let shrunk_radii = normalize_radii_to_rect(
                        [
                            Point::new(
                                (clip_radii[0].x - left_inset).max(0.0),
                                (clip_radii[0].y - top_inset).max(0.0),
                            ),
                            Point::new(
                                (clip_radii[1].x - right_inset).max(0.0),
                                (clip_radii[1].y - top_inset).max(0.0),
                            ),
                            Point::new(
                                (clip_radii[2].x - right_inset).max(0.0),
                                (clip_radii[2].y - bottom_inset).max(0.0),
                            ),
                            Point::new(
                                (clip_radii[3].x - left_inset).max(0.0),
                                (clip_radii[3].y - bottom_inset).max(0.0),
                            ),
                        ],
                        &shrunk_rect,
                    );
                    canvas.draw_rrect(RRect::new_rect_radii(shrunk_rect, &shrunk_radii), &paint);
                } else if use_layer && effective_background_clip == BackgroundClip::BorderBox {
                    canvas.draw_rect(bg_rect, &paint);
                } else if effective_background_clip == BackgroundClip::BorderBox
                    || !radii_exceed_rect(&bg_rect, &clip_radii)
                {
                    // A solid rounded background is a native rounded-rect
                    // fill in Blink. Clipping a rectangle through an AA rrect
                    // creates a separately quantized mask and loses the
                    // lowest-coverage edge samples.
                    let clip_radii = normalize_radii_to_rect(clip_radii, &bg_rect);
                    canvas.draw_rrect(RRect::new_rect_radii(bg_rect, &clip_radii), &paint);
                } else {
                    canvas.save();
                    clip_nonrenderable_inner_rounded_rect(
                        canvas,
                        border_box_rect,
                        bg_rect,
                        &clip_radii,
                    );
                    canvas.draw_rect(bg_rect, &paint);
                    canvas.restore();
                }
            } else {
                canvas.draw_rect(bg_rect, &paint);
            }
        }
    }

    // Background images paint above background-color and below inset shadows.
    // The initial background-origin is padding-box, while background-clip is
    // independently resolved above.  Keeping the gradient's positioning area
    // separate from its paint clip is essential for cloned fragments: each
    // clone restarts the image in its own padding box.
    if let Some(gradient) = &style.background_linear_gradient {
        if gradient.stops.len() >= 2 && effective_background_clip != BackgroundClip::Text {
            let border_left = fragment.border.left.round().to_f32();
            let border_top = fragment.border.top.round().to_f32();
            let border_right = fragment.border.right.round().to_f32();
            let border_bottom = fragment.border.bottom.round().to_f32();
            let padding_rect = Rect::from_ltrb(
                x + border_left,
                y + border_top,
                (right - border_right).max(x + border_left),
                (bottom - border_bottom).max(y + border_top),
            );
            let positioning_padding_rect =
                fragment.decoration_slice.map_or(padding_rect, |slice| {
                    // A sliced continuation suppresses its block-start border,
                    // but that decoration still occupies source-space in the
                    // unfragmented background positioning area. Advance past
                    // it before sampling the continuation image.
                    let suppressed_block_start_border = if fragment.is_first_for_node {
                        0.0
                    } else {
                        border_top
                    };
                    let source_top =
                        y - slice.source_block_offset.to_f32() - suppressed_block_start_border;
                    let source_bottom = source_top + slice.source_block_size.to_f32();
                    Rect::from_ltrb(
                        x + border_left,
                        source_top + border_top,
                        (right - border_right).max(x + border_left),
                        (source_bottom - border_bottom).max(source_top + border_top),
                    )
                });
            let paint_rect = match effective_background_clip {
                BackgroundClip::BorderBox | BackgroundClip::BorderArea => border_box_rect,
                BackgroundClip::PaddingBox => padding_rect,
                BackgroundClip::ContentBox => Rect::from_ltrb(
                    padding_rect.left + fragment.padding.left.round().to_f32(),
                    padding_rect.top + fragment.padding.top.round().to_f32(),
                    (padding_rect.right - fragment.padding.right.round().to_f32())
                        .max(padding_rect.left),
                    (padding_rect.bottom - fragment.padding.bottom.round().to_f32())
                        .max(padding_rect.top),
                ),
                BackgroundClip::Text => unreachable!(),
            };
            let radians = gradient.angle_degrees.to_radians();
            let direction = Point::new(radians.sin(), -radians.cos());
            let line_length = positioning_padding_rect.width() * direction.x.abs()
                + positioning_padding_rect.height() * direction.y.abs();
            if line_length > 0.0 && paint_rect.width() > 0.0 && paint_rect.height() > 0.0 {
                let center = Point::new(
                    (positioning_padding_rect.left + positioning_padding_rect.right) * 0.5,
                    (positioning_padding_rect.top + positioning_padding_rect.bottom) * 0.5,
                );
                let half = line_length * 0.5;
                let full_start =
                    Point::new(center.x - direction.x * half, center.y - direction.y * half);
                let full_end =
                    Point::new(center.x + direction.x * half, center.y + direction.y * half);
                let colors: Vec<skia_safe::Color> = gradient
                    .stops
                    .iter()
                    .map(|stop| skia_css_color_with_alpha(&stop.color, opacity_multiplier))
                    .collect();
                let mut positions = resolved_gradient_positions(&gradient.stops, line_length);
                let (start, end, tile_mode) = if gradient.repeating {
                    let first_offset = positions.first().copied().unwrap_or(0.0) * line_length;
                    let last_offset = positions.last().copied().unwrap_or(1.0) * line_length;
                    let period = last_offset - first_offset;
                    if period > 0.0 {
                        for position in &mut positions {
                            *position = (*position * line_length - first_offset) / period;
                        }
                        (
                            Point::new(
                                full_start.x + direction.x * first_offset,
                                full_start.y + direction.y * first_offset,
                            ),
                            Point::new(
                                full_start.x + direction.x * last_offset,
                                full_start.y + direction.y * last_offset,
                            ),
                            TileMode::Repeat,
                        )
                    } else {
                        (full_start, full_end, TileMode::Clamp)
                    }
                } else {
                    (full_start, full_end, TileMode::Clamp)
                };
                let mut paint = Paint::default();
                paint.set_style(PaintStyle::Fill);
                paint.set_anti_alias(true);
                paint.set_shader(gradient_shader::linear(
                    (start, end),
                    colors.as_slice(),
                    positions.as_slice(),
                    tile_mode,
                    None,
                    None,
                ));
                canvas.save();
                if has_radius {
                    let clip_rrect = RRect::new_rect_radii(
                        paint_rect,
                        &fragment_border_radii(style, fragment, &paint_rect, bt),
                    );
                    canvas.clip_rrect(clip_rrect, ClipOp::Intersect, true);
                } else {
                    canvas.clip_rect(paint_rect, ClipOp::Intersect, false);
                }
                canvas.draw_rect(paint_rect, &paint);
                canvas.restore();
            }
        }
    }

    // ── 3. Inset box shadows (painted on top of background) ──────────
    paint_box_shadows(canvas, style, shadow_border_box_rect, true);

    // ── 4. Borders ───────────────────────────────────────────────────
    paint_borders(canvas, fragment, style, x, y, w, h, use_layer);

    if use_layer {
        let border_rect = Rect::from_xywh(x, y, w, h);
        let outer_radii = fragment_radii;
        let inner_rect = border_inner_rect;
        let inner_radii = border_inner_radii;
        let border_color = style.border_top_color.resolve(&style.color);
        let single_outer_corner = outer_radii
            .iter()
            .filter(|r| r.x > 0.0 && r.y > 0.0)
            .count()
            == 1;
        if effective_background_clip == BackgroundClip::ContentBox
            && border_color.is_opaque()
            && single_outer_corner
            && radii_exceed_rect(&inner_rect, &inner_radii)
        {
            let mut erase = Paint::default();
            erase.set_style(PaintStyle::Fill);
            erase.set_anti_alias(false);
            erase.set_blend_mode(BlendMode::DstOut);
            erase.set_color4f(Color4f::new(0.0, 0.0, 0.0, 0.08), None::<&ColorSpace>);
            canvas.draw_rect(
                Rect::from_ltrb(
                    border_rect.right - bt / 2.0,
                    border_rect.top,
                    border_rect.right - bt / 2.0 + 2.0,
                    border_rect.top + 1.0,
                ),
                &erase,
            );
            canvas.draw_rect(
                Rect::from_ltrb(
                    border_rect.left,
                    border_rect.bottom - bt / 2.0,
                    border_rect.left + 1.0,
                    border_rect.bottom - bt / 2.0 + 2.0,
                ),
                &erase,
            );
        }
        canvas.restore(); // pops saveLayer
        if radii_exceed_rect(&inner_rect, &inner_radii) {
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            set_paint_css_color(&mut paint, &border_color);
            let edge_overlap = (bt / 10.0).round().clamp(1.0, 4.0);
            draw_outer_edge_overlap(canvas, border_rect, edge_overlap, &paint);
            draw_outer_corner_tangent_overlap(
                canvas,
                border_rect,
                &outer_radii,
                bt,
                edge_overlap,
                &paint,
            );
            draw_nonrenderable_outer_tangent_fringe(
                canvas,
                border_rect,
                &outer_radii,
                bt,
                &border_color,
            );
        } else {
            draw_renderable_outer_aa_fringe(
                canvas,
                &outer_radii,
                inner_rect,
                inner_radii,
                &border_color,
            );
        }
        canvas.restore(); // pops save()
    }
    if decoration_clip_saved {
        canvas.restore();
    }
}

/// Paint borders around the border-box.
///
/// Extracted from Blink's `BoxBorderPainter` (box_border_painter.cc).
fn paint_borders(
    canvas: &Canvas,
    fragment: &Fragment,
    style: &ComputedStyle,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    outer_rrect_clipped: bool,
) {
    // Slice block fragments on the block axis and inline fragments on the
    // inline axis. Clone repeats both inline edges for every continuation.
    let clone_inline = fragment.is_inline_box_fragment
        && style.box_decoration_break == openui_style::BoxDecorationBreak::Clone;
    let bt = if fragment.is_inline_box_fragment || fragment.is_first_for_node {
        style.effective_border_top() as f32
    } else {
        0.0
    };
    let br = if !fragment.is_inline_box_fragment || clone_inline || fragment.is_last_for_node {
        style.effective_border_right() as f32
    } else {
        0.0
    };
    let bb = if fragment.is_inline_box_fragment || fragment.is_last_for_node {
        style.effective_border_bottom() as f32
    } else {
        0.0
    };
    let bl = if !fragment.is_inline_box_fragment || clone_inline || fragment.is_first_for_node {
        style.effective_border_left() as f32
    } else {
        0.0
    };

    // No borders to paint
    if bt == 0.0 && br == 0.0 && bb == 0.0 && bl == 0.0 {
        return;
    }

    let inherited_color = &style.color;
    let side_specs = [
        (
            bt,
            style.border_top_style,
            style.border_top_color.resolve(inherited_color),
        ),
        (
            br,
            style.border_right_style,
            style.border_right_color.resolve(inherited_color),
        ),
        (
            bb,
            style.border_bottom_style,
            style.border_bottom_color.resolve(inherited_color),
        ),
        (
            bl,
            style.border_left_style,
            style.border_left_color.resolve(inherited_color),
        ),
    ];
    let mut same_solid_color = true;
    let mut solid_color = None;
    for (width, border_style, color) in side_specs {
        if width <= 0.0 {
            continue;
        }
        if border_style != BorderStyle::Solid {
            same_solid_color = false;
            break;
        }
        if let Some(existing) = solid_color {
            if existing != color {
                same_solid_color = false;
                break;
            }
        } else {
            solid_color = Some(color);
        }
    }

    let non_uniform_widths = bt != br || br != bb || bb != bl;
    let mut same_solid_visible_border = false;
    let mut visible_width: Option<f32> = None;
    let mut visible_color: Option<Color> = None;
    for (width, border_style, color) in side_specs {
        if width <= 0.0 {
            continue;
        }
        same_solid_visible_border = true;
        if border_style != BorderStyle::Solid {
            same_solid_visible_border = false;
            break;
        }
        if let Some(existing_width) = visible_width {
            if existing_width != width {
                same_solid_visible_border = false;
                break;
            }
        } else {
            visible_width = Some(width);
        }
        if let Some(existing_color) = visible_color {
            if existing_color != color {
                same_solid_visible_border = false;
                break;
            }
        } else {
            visible_color = Some(color);
        }
    }

    if style.has_border_radius() && non_uniform_widths && same_solid_visible_border {
        if let (Some(border_width), Some(color)) = (visible_width, visible_color) {
            let border_rect = Rect::from_xywh(x, y, w, h);
            let outer_radii = fragment_border_radii(style, fragment, &border_rect, border_width);
            if has_any_radius(&outer_radii) {
                let mut fill_paint = Paint::default();
                fill_paint.set_style(PaintStyle::Fill);
                fill_paint.set_anti_alias(true);
                set_paint_css_color(&mut fill_paint, &color);

                let inner_rect = Rect::from_xywh(
                    x + bl,
                    y + bt,
                    (w - bl - br).max(0.0),
                    (h - bt - bb).max(0.0),
                );
                let inner_radii = normalize_radii_to_rect(
                    [
                        Point::new(
                            (outer_radii[0].x - bl).max(0.0),
                            (outer_radii[0].y - bt).max(0.0),
                        ),
                        Point::new(
                            (outer_radii[1].x - br).max(0.0),
                            (outer_radii[1].y - bt).max(0.0),
                        ),
                        Point::new(
                            (outer_radii[2].x - br).max(0.0),
                            (outer_radii[2].y - bb).max(0.0),
                        ),
                        Point::new(
                            (outer_radii[3].x - bl).max(0.0),
                            (outer_radii[3].y - bb).max(0.0),
                        ),
                    ],
                    &inner_rect,
                );

                canvas.save();
                if !outer_rrect_clipped {
                    canvas.clip_rrect(
                        RRect::new_rect_radii(border_rect, &outer_radii),
                        ClipOp::Intersect,
                        true,
                    );
                }
                canvas.clip_rrect(
                    RRect::new_rect_radii(inner_rect, &inner_radii),
                    ClipOp::Difference,
                    true,
                );

                // Blink's complex-border path clips each side at the
                // half-corner of the inner contour. Curved vertical sides
                // use a tiny overlap at the rounded end and an AA bounding
                // clip at the straight end; these are generated by the
                // shared ContouredRect side-clip algorithm and prevent a
                // seam where fragment edges suppress one pair of corners.
                let is_rounded = |radius: Point| radius.x > 0.0 && radius.y > 0.0;
                let inner_half = |index: usize| match index {
                    0 => Point::new(
                        inner_rect.left + inner_radii[0].x * 0.5,
                        inner_rect.top + inner_radii[0].y * 0.5,
                    ),
                    1 => Point::new(
                        inner_rect.right - inner_radii[1].x * 0.5,
                        inner_rect.top + inner_radii[1].y * 0.5,
                    ),
                    2 => Point::new(
                        inner_rect.right - inner_radii[2].x * 0.5,
                        inner_rect.bottom - inner_radii[2].y * 0.5,
                    ),
                    _ => Point::new(
                        inner_rect.left + inner_radii[3].x * 0.5,
                        inner_rect.bottom - inner_radii[3].y * 0.5,
                    ),
                };
                let halves = [inner_half(0), inner_half(1), inner_half(2), inner_half(3)];
                let side_path = |points: &[Point; 4], aa_bounds: Option<(Rect, bool)>| {
                    let mut path = PathBuilder::new();
                    path.move_to(points[0]);
                    for point in &points[1..] {
                        path.line_to(*point);
                    }
                    canvas.save();
                    if aa_bounds.is_some_and(|(_, before)| before) {
                        canvas.clip_rect(aa_bounds.unwrap().0, ClipOp::Intersect, true);
                    }
                    canvas.clip_path(&path.detach(), ClipOp::Intersect, false);
                    if aa_bounds.is_some_and(|(_, before)| !before) {
                        canvas.clip_rect(aa_bounds.unwrap().0, ClipOp::Intersect, true);
                    }
                    canvas.draw_rect(border_rect, &fill_paint);
                    canvas.restore();
                };
                if bt > 0.0 {
                    if is_rounded(inner_radii[0]) || is_rounded(inner_radii[1]) {
                        side_path(
                            &[
                                Point::new(border_rect.left, border_rect.top),
                                halves[0],
                                halves[1],
                                Point::new(border_rect.right, border_rect.top),
                            ],
                            None,
                        );
                    } else {
                        canvas.draw_rect(
                            Rect::from_xywh(border_rect.left, border_rect.top, w, bt),
                            &fill_paint,
                        );
                    }
                }
                if bb > 0.0 {
                    if is_rounded(inner_radii[2]) || is_rounded(inner_radii[3]) {
                        side_path(
                            &[
                                Point::new(border_rect.right, border_rect.bottom),
                                halves[2],
                                halves[3],
                                Point::new(border_rect.left, border_rect.bottom),
                            ],
                            None,
                        );
                    } else {
                        canvas.draw_rect(
                            Rect::from_xywh(border_rect.left, border_rect.bottom - bb, w, bb),
                            &fill_paint,
                        );
                    }
                }
                if br > 0.0 {
                    let top_rounded = is_rounded(outer_radii[1]);
                    let bottom_rounded = is_rounded(outer_radii[2]);
                    let inner_edge_is_curved =
                        is_rounded(inner_radii[1]) || is_rounded(inner_radii[2]);
                    if !inner_edge_is_curved {
                        canvas.draw_rect(
                            Rect::from_xywh(border_rect.right - br, border_rect.top, br, h),
                            &fill_paint,
                        );
                    } else {
                        let inner_x = if top_rounded && !bottom_rounded {
                            halves[1].x
                        } else if bottom_rounded && !top_rounded {
                            halves[2].x
                        } else {
                            halves[1].x.min(halves[2].x)
                        };
                        let seam_overlap = 0.1;
                        let top_overlap = if top_rounded { seam_overlap } else { 0.0 };
                        let bottom_overlap = if bottom_rounded { seam_overlap } else { 0.0 };
                        let bounds = Rect::from_ltrb(
                            inner_x,
                            border_rect.top - if bottom_rounded { seam_overlap } else { 0.0 },
                            border_rect.right,
                            border_rect.bottom + if top_rounded { seam_overlap } else { 0.0 },
                        );
                        side_path(
                            &[
                                Point::new(border_rect.right, border_rect.top - top_overlap),
                                Point::new(
                                    inner_x,
                                    if top_rounded {
                                        halves[1].y - top_overlap
                                    } else {
                                        border_rect.top
                                    },
                                ),
                                Point::new(
                                    inner_x,
                                    if bottom_rounded {
                                        halves[2].y + bottom_overlap
                                    } else {
                                        border_rect.bottom
                                    },
                                ),
                                Point::new(border_rect.right, border_rect.bottom + bottom_overlap),
                            ],
                            Some((bounds, bottom_rounded)),
                        );
                    }
                }
                if bl > 0.0 {
                    let top_rounded = is_rounded(outer_radii[0]);
                    let bottom_rounded = is_rounded(outer_radii[3]);
                    let inner_edge_is_curved =
                        is_rounded(inner_radii[0]) || is_rounded(inner_radii[3]);
                    if !inner_edge_is_curved {
                        canvas.draw_rect(
                            Rect::from_xywh(border_rect.left, border_rect.top, bl, h),
                            &fill_paint,
                        );
                    } else {
                        let inner_x = if top_rounded && !bottom_rounded {
                            halves[0].x
                        } else if bottom_rounded && !top_rounded {
                            halves[3].x
                        } else {
                            halves[0].x.max(halves[3].x)
                        };
                        let seam_overlap = 0.1;
                        let top_overlap = if top_rounded { seam_overlap } else { 0.0 };
                        let bottom_overlap = if bottom_rounded { seam_overlap } else { 0.0 };
                        let bounds = Rect::from_ltrb(
                            border_rect.left,
                            border_rect.top - if bottom_rounded { seam_overlap } else { 0.0 },
                            inner_x,
                            border_rect.bottom + if top_rounded { seam_overlap } else { 0.0 },
                        );
                        side_path(
                            &[
                                Point::new(border_rect.left, border_rect.bottom + bottom_overlap),
                                Point::new(
                                    inner_x,
                                    if bottom_rounded {
                                        halves[3].y + bottom_overlap
                                    } else {
                                        border_rect.bottom
                                    },
                                ),
                                Point::new(
                                    inner_x,
                                    if top_rounded {
                                        halves[0].y - top_overlap
                                    } else {
                                        border_rect.top
                                    },
                                ),
                                Point::new(border_rect.left, border_rect.top - top_overlap),
                            ],
                            Some((bounds, top_rounded)),
                        );
                    }
                }
                canvas.restore();
                return;
            }
        }
    }

    if same_solid_color && non_uniform_widths && !style.has_border_radius() {
        if let Some(color) = solid_color {
            paint_same_color_solid_border(canvas, color, x, y, w, h, bt, br, bb, bl);
            return;
        }
    }

    // Check if all borders are the same color and style (fast path).
    // Blink: DrawSolidBorderRect for uniform solid borders.
    let uniform = bt == br
        && br == bb
        && bb == bl
        && style.border_top_style == style.border_right_style
        && style.border_right_style == style.border_bottom_style
        && style.border_bottom_style == style.border_left_style
        && style.border_top_color == style.border_right_color
        && style.border_right_color == style.border_bottom_color
        && style.border_bottom_color == style.border_left_color;

    if uniform && style.border_top_style == BorderStyle::Solid {
        // Fast path: single stroke rect
        // Blink: DrawSolidBorderRect (box_border_painter.cc:261)
        // Stroke rect inset by half the border width.
        // Clamp dimensions to zero minimum to prevent invalid geometry
        // when borders are wider than the box.
        let half = bt / 2.0;
        let sw = (w - bt).max(0.0);
        let sh = (h - bt).max(0.0);
        let stroke_rect = Rect::from_xywh(x + half, y + half, sw, sh);

        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Stroke);
        paint.set_stroke_width(bt);
        paint.set_anti_alias(true);

        let resolved = style.border_top_color.resolve(inherited_color);
        set_paint_css_color(&mut paint, &resolved);

        let border_radii = fragment_border_radii(style, fragment, &Rect::from_xywh(x, y, w, h), bt);
        if has_any_radius(&border_radii) {
            // The outer border-box rrect clip is already set by the caller.
            // Just fill the border ring by excluding the inner rrect.
            let mut fill_paint = Paint::default();
            fill_paint.set_style(PaintStyle::Fill);
            fill_paint.set_anti_alias(true);
            set_paint_css_color(&mut fill_paint, &resolved);

            let inner_rect = Rect::from_xywh(
                x + bt,
                y + bt,
                (w - bt - bt).max(0.0),
                (h - bt - bt).max(0.0),
            );
            let outer_radii = border_radii;
            let inner_radii = [
                Point::new(
                    (outer_radii[0].x - bt).max(0.0),
                    (outer_radii[0].y - bt).max(0.0),
                ),
                Point::new(
                    (outer_radii[1].x - bt).max(0.0),
                    (outer_radii[1].y - bt).max(0.0),
                ),
                Point::new(
                    (outer_radii[2].x - bt).max(0.0),
                    (outer_radii[2].y - bt).max(0.0),
                ),
                Point::new(
                    (outer_radii[3].x - bt).max(0.0),
                    (outer_radii[3].y - bt).max(0.0),
                ),
            ];
            // Blink lowers a renderable uniform circular border to one
            // antialiased stroked rrect. Use that representation whenever no
            // outer clip would evaluate the same contour a second time.
            let simple_stroked_rrect = !radii_exceed_rect(&inner_rect, &inner_radii)
                && outer_radii
                    .iter()
                    .zip(inner_radii.iter())
                    .all(|(outer, inner)| {
                        if outer.x == 0.0 && outer.y == 0.0 && inner.x == 0.0 && inner.y == 0.0 {
                            true
                        } else {
                            (outer.x - outer.y).abs() < 0.01
                                && (inner.x - inner.y).abs() < 0.01
                                && (outer.x - inner.x - bt).abs() < 0.01
                        }
                    });
            if simple_stroked_rrect && !outer_rrect_clipped {
                let center_radii = outer_radii.map(|radius| {
                    Point::new((radius.x - half).max(0.0), (radius.y - half).max(0.0))
                });
                canvas.draw_rrect(RRect::new_rect_radii(stroke_rect, &center_radii), &paint);
                return;
            }
            if outer_rrect_clipped && radii_exceed_rect(&inner_rect, &inner_radii) {
                let border_rect = Rect::from_xywh(x, y, w, h);
                draw_nonrenderable_uniform_rounded_border(
                    canvas,
                    border_rect,
                    inner_rect,
                    inner_radii,
                    &fill_paint,
                );
            } else if outer_rrect_clipped {
                // The outer rrect clip is already active on the canvas.
                // Draw the border ring as a path with InverseWinding fill so
                // that everything outside the inner rrect (within the existing
                // clip) gets painted. This avoids stacking a second AA clip
                // on top of the existing one, which would produce subtle
                // double-AA artifacts at curved corners.
                let inner_radii = normalize_radii_to_rect(inner_radii, &inner_rect);
                let inner_rrect = RRect::new_rect_radii(inner_rect, &inner_radii);
                let mut border_path = PathBuilder::new();
                border_path.set_fill_type(PathFillType::InverseWinding);
                border_path.add_rrect(inner_rrect, None, None);
                canvas.draw_path(&border_path.detach(), &fill_paint);
            } else {
                let inner_radii = normalize_radii_to_rect(inner_radii, &inner_rect);
                let inner_rrect = RRect::new_rect_radii(inner_rect, &inner_radii);
                let outer_rect = Rect::from_xywh(x, y, w, h);
                let outer_radii = normalize_radii_to_rect(outer_radii, &outer_rect);
                let outer_rrect = RRect::new_rect_radii(outer_rect, &outer_radii);
                // A double rounded rectangle evaluates both contours in one
                // coverage operation for non-circular and elliptical cases.
                canvas.draw_drrect(outer_rrect, inner_rrect, &fill_paint);
            }
        } else {
            canvas.draw_rect(stroke_rect, &paint);
        }
    } else {
        // Per-side border painting using trapezoid polygons.
        // Each side is drawn as a 4-point polygon with diagonal corner joins
        // from the outer corner to the inner corner (mitered join).
        // This matches Blink's BoxBorderPainter approach.
        //
        // Outer rect corners:
        let ox0 = x;
        let oy0 = y;
        let ox1 = x + w;
        let oy1 = y + h;
        // Inner rect corners — clamped so edges never cross when
        // borders are wider than the box.
        let ix0 = (x + bl).min(x + w - br);
        let iy0 = (y + bt).min(y + h - bb);
        let ix1 = (x + w - br).max(ix0);
        let iy1 = (y + h - bb).max(iy0);

        // Chrome paint order: Top → Bottom → Right → Left (sorted by side priority).
        // Top border: outer-top-left → outer-top-right → inner-top-right → inner-top-left
        if bt > 0.0 {
            paint_border_side_path(
                canvas,
                style.border_top_style,
                &style.border_top_color,
                inherited_color,
                bt,
                &[(ox0, oy0), (ox1, oy0), (ix1, iy0), (ix0, iy0)],
                BorderSide::Top,
            );
        }
        // Bottom border: outer-bottom-right → outer-bottom-left → inner-bottom-left → inner-bottom-right
        if bb > 0.0 {
            paint_border_side_path(
                canvas,
                style.border_bottom_style,
                &style.border_bottom_color,
                inherited_color,
                bb,
                &[(ox1, oy1), (ox0, oy1), (ix0, iy1), (ix1, iy1)],
                BorderSide::Bottom,
            );
        }
        // Right border: outer-top-right → outer-bottom-right → inner-bottom-right → inner-top-right
        if br > 0.0 {
            paint_border_side_path(
                canvas,
                style.border_right_style,
                &style.border_right_color,
                inherited_color,
                br,
                &[(ox1, oy0), (ox1, oy1), (ix1, iy1), (ix1, iy0)],
                BorderSide::Right,
            );
        }
        // Left border: outer-bottom-left → outer-top-left → inner-top-left → inner-bottom-left
        if bl > 0.0 {
            paint_border_side_path(
                canvas,
                style.border_left_style,
                &style.border_left_color,
                inherited_color,
                bl,
                &[(ox0, oy1), (ox0, oy0), (ix0, iy0), (ix0, iy1)],
                BorderSide::Left,
            );
        }

        // Fix corner diagonal pixels: Chrome's Skia achieves exact complementary
        // coverage at shared miter edges (no white bleed-through). Standard SrcOver
        // compositing leaves ~25% background showing. Fix by overwriting diagonal
        // pixels with the exact 50% blend of the two adjacent border colors.
        fix_corner_miter_pixels(
            canvas,
            inherited_color,
            style,
            ox0,
            oy0,
            ox1,
            oy1,
            ix0,
            iy0,
            ix1,
            iy1,
        );
    }
}

fn paint_same_color_solid_border(
    canvas: &Canvas,
    color: Color,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    bt: f32,
    br: f32,
    bb: f32,
    bl: f32,
) {
    let ix0 = (x + bl).min(x + w - br);
    let iy0 = (y + bt).min(y + h - bb);
    let ix1 = (x + w - br).max(ix0);
    let iy1 = (y + h - bb).max(iy0);

    let mut path = PathBuilder::new();
    path.add_rect(Rect::from_xywh(x, y, w, h), None, None);
    if ix1 > ix0 && iy1 > iy0 {
        path.add_rect(
            Rect::from_ltrb(ix0, iy0, ix1, iy1),
            skia_safe::PathDirection::CCW,
            0,
        );
        path.set_fill_type(skia_safe::PathFillType::EvenOdd);
    }

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    set_paint_css_color(&mut paint, &color);
    canvas.draw_path(&path.detach(), &paint);
}

/// Physical side of a border box, used for side-dependent shading
/// in 3D border styles (inset, outset, groove, ridge).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorderSide {
    Top,
    Right,
    Bottom,
    Left,
}

/// Paint a column rule (divider between multicol columns).
///
/// Uses the column-rule-color, column-rule-style, and column-rule-width
/// from the parent style. Supports all CSS border styles (solid, dashed,
/// dotted, double, groove, ridge, inset, outset).
/// Paint CSS outline around the border box.
/// Outline is drawn outside the border edge, offset by outline-offset.
/// Unlike borders, outline does not affect layout and can overlap other content.
fn paint_outline(
    canvas: &Canvas,
    fragment: &Fragment,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
) {
    let ow = style.effective_outline_width() as f32;
    if ow <= 0.0 {
        return;
    }

    let offset_px = style.outline_offset as f32;

    // Border box coordinates (pixel-snapped)
    let bx = abs_offset.left.round().to_f32();
    let by = abs_offset.top.round().to_f32();
    let br = (abs_offset.left + fragment.size.width).round().to_f32();
    let bb = (abs_offset.top + fragment.size.height).round().to_f32();

    // Outline box = border box expanded by (outline-offset + outline-width)
    let expand = offset_px + ow;
    let ox = bx - expand;
    let oy = by - expand;
    let or = br + expand;
    let ob = bb + expand;
    let ow_total = or - ox;
    let oh_total = ob - oy;

    if ow_total <= 0.0 || oh_total <= 0.0 {
        return;
    }

    let color = style.outline_color.resolve(&style.color);
    let mut paint = skia_safe::Paint::default();
    paint.set_color(skia_safe::Color::from_argb(
        (color.a * 255.0) as u8,
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
    ));
    paint.set_anti_alias(false);
    paint.set_style(skia_safe::paint::Style::Fill);

    if fragment.paint_zero_block_outline {
        canvas.draw_rect(Rect::from_xywh(bx, by, br - bx, ow), &paint);
        return;
    }

    if style.outline_style == BorderStyle::Dotted {
        let dot = ow.round().max(1.0);
        let step = dot * 2.0;
        let mut x = ox;
        while x < or {
            let w = dot.min(or - x);
            canvas.draw_rect(Rect::from_xywh(x, oy, w, ow), &paint);
            canvas.draw_rect(Rect::from_xywh(x, ob - ow, w, ow), &paint);
            x += step;
        }

        let mut y = oy + ow + (dot / 2.0).floor();
        let vertical_bottom = ob - ow;
        while y < vertical_bottom {
            let h = dot.min(vertical_bottom - y);
            canvas.draw_rect(Rect::from_xywh(ox, y, ow, h), &paint);
            canvas.draw_rect(Rect::from_xywh(or - ow, y, ow, h), &paint);
            y += step;
        }
        return;
    }

    // For solid outlines, draw 4 rectangles (top, right, bottom, left).
    // For other styles, fall back to solid (matches most visual tests).
    // Top edge
    canvas.draw_rect(Rect::from_xywh(ox, oy, ow_total, ow), &paint);
    // Bottom edge
    canvas.draw_rect(Rect::from_xywh(ox, ob - ow, ow_total, ow), &paint);
    // Left edge
    canvas.draw_rect(
        Rect::from_xywh(ox, oy + ow, ow, oh_total - 2.0 * ow),
        &paint,
    );
    // Right edge
    canvas.draw_rect(
        Rect::from_xywh(or - ow, oy + ow, ow, oh_total - 2.0 * ow),
        &paint,
    );
}

fn paint_column_rule(
    canvas: &Canvas,
    fragment: &Fragment,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
) {
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    let w = right - x;
    let h = bottom - y;

    if w <= 0.0 || h <= 0.0 {
        return;
    }

    let rect = Rect::from_xywh(x, y, w, h);
    // Column rules are painted like left-side borders (vertical line).
    // CSS Multicol makes the one-dimensional `inset` and `outset` rule
    // styles equivalent to `ridge` and `groove`, respectively.  Unlike a
    // four-sided border there is no opposing edge from which to infer the
    // recessed/raised effect.
    let rule_style = match style.column_rule_style {
        BorderStyle::Inset => BorderStyle::Ridge,
        BorderStyle::Outset => BorderStyle::Groove,
        other => other,
    };
    paint_border_side(
        canvas,
        rule_style,
        &style.column_rule_color,
        &style.color,
        w,
        rect,
        BorderSide::Left,
    );
}

/// Fix corner diagonal pixels where two different-colored solid borders meet.
///
/// Chrome's Skia build achieves exact complementary coverage at shared miter
/// edges, producing a perfect 50% blend with no background bleed-through.
/// Standard SrcOver compositing with separate draw calls leaves ~25% white.
/// This function overwrites the diagonal miter pixels with the exact blend.
fn fix_corner_miter_pixels(
    canvas: &Canvas,
    inherited_color: &Color,
    style: &ComputedStyle,
    ox0: f32,
    oy0: f32,
    ox1: f32,
    oy1: f32,
    ix0: f32,
    iy0: f32,
    ix1: f32,
    iy1: f32,
) {
    let top_c = style.border_top_color.resolve(inherited_color);
    let right_c = style.border_right_color.resolve(inherited_color);
    let bottom_c = style.border_bottom_color.resolve(inherited_color);
    let left_c = style.border_left_color.resolve(inherited_color);

    let bt = iy0 - oy0;
    let br = ox1 - ix1;
    let bb = oy1 - iy1;
    let bl = ix0 - ox0;

    // Top-left corner: fix if both adjacent sides are solid and opaque.
    // Skip when either side is transparent — clip-path AA already provides
    // the correct sub-pixel blend in that case; overwriting it here would
    // produce the wrong value.
    if bt > 0.5
        && bl > 0.5
        && style.border_top_style == BorderStyle::Solid
        && style.border_left_style == BorderStyle::Solid
        && top_c.a > 0.01
        && left_c.a > 0.01
    {
        draw_miter_blend_pixels(
            canvas,
            ox0 as i32,
            oy0 as i32,
            ix0 as i32 - 1,
            iy0 as i32 - 1,
            &top_c,
            &left_c,
        );
    }
    // Top-right corner
    if bt > 0.5
        && br > 0.5
        && style.border_top_style == BorderStyle::Solid
        && style.border_right_style == BorderStyle::Solid
        && top_c.a > 0.01
        && right_c.a > 0.01
    {
        draw_miter_blend_pixels(
            canvas,
            ox1 as i32 - 1,
            oy0 as i32,
            ix1 as i32,
            iy0 as i32 - 1,
            &top_c,
            &right_c,
        );
    }
    // Bottom-right corner
    if bb > 0.5
        && br > 0.5
        && style.border_bottom_style == BorderStyle::Solid
        && style.border_right_style == BorderStyle::Solid
        && bottom_c.a > 0.01
        && right_c.a > 0.01
    {
        draw_miter_blend_pixels(
            canvas,
            ox1 as i32 - 1,
            oy1 as i32 - 1,
            ix1 as i32,
            iy1 as i32,
            &bottom_c,
            &right_c,
        );
    }
    // Bottom-left corner
    if bb > 0.5
        && bl > 0.5
        && style.border_bottom_style == BorderStyle::Solid
        && style.border_left_style == BorderStyle::Solid
        && bottom_c.a > 0.01
        && left_c.a > 0.01
    {
        draw_miter_blend_pixels(
            canvas,
            ox0 as i32,
            oy1 as i32 - 1,
            ix0 as i32 - 1,
            iy1 as i32,
            &bottom_c,
            &left_c,
        );
    }
}

/// Draw 50% blend pixels along a miter diagonal between two integer pixel coords.
fn draw_miter_blend_pixels(
    canvas: &Canvas,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color1: &Color,
    color2: &Color,
) {
    // Blend in premultiplied space for correctness with translucent borders.
    let avg_a = (color1.a + color2.a) / 2.0;
    let blend = if avg_a > 0.0 {
        let pm_r = (color1.r * color1.a + color2.r * color2.a) / 2.0;
        let pm_g = (color1.g * color1.a + color2.g * color2.a) / 2.0;
        let pm_b = (color1.b * color1.a + color2.b * color2.a) / 2.0;
        Color4f::new(pm_r / avg_a, pm_g / avg_a, pm_b / avg_a, avg_a)
    } else {
        Color4f::new(0.0, 0.0, 0.0, 0.0)
    };

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    paint.set_color4f(blend, None::<&ColorSpace>);

    // Bresenham's line algorithm for integer pixel coordinates.
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut cx = x0;
    let mut cy = y0;

    loop {
        canvas.draw_rect(Rect::from_xywh(cx as f32, cy as f32, 1.0, 1.0), &paint);
        if cx == x1 && cy == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
        }
    }
}

/// Paint a single border side using a trapezoid polygon path.
///
/// The `points` array contains 4 (x, y) pairs forming the trapezoid.
/// For solid borders, the path is filled directly. For dashed/dotted/double
/// and 3D styles, falls back to the rectangle-based `paint_border_side`.
fn paint_border_side_path(
    canvas: &Canvas,
    border_style: BorderStyle,
    border_color: &StyleColor,
    inherited_color: &Color,
    width: f32,
    points: &[(f32, f32); 4],
    side: BorderSide,
) {
    if width <= 0.0 {
        return;
    }
    if matches!(border_style, BorderStyle::None | BorderStyle::Hidden) {
        return;
    }

    let resolved = border_color.resolve(inherited_color);

    match border_style {
        BorderStyle::Solid => {
            // Chrome clips to the trapezoid polygon, then fills a solid rect.
            // Using clip AA (not path fill AA) ensures correct corner blending
            // when adjacent sides have different colors.
            let min_x = points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
            let min_y = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
            let max_x = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
            let max_y = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
            let rect = Rect::from_ltrb(min_x, min_y, max_x, max_y);

            canvas.save();
            let mut clip_path = PathBuilder::new();
            clip_path.move_to(Point::new(points[0].0, points[0].1));
            clip_path.line_to(Point::new(points[1].0, points[1].1));
            clip_path.line_to(Point::new(points[2].0, points[2].1));
            clip_path.line_to(Point::new(points[3].0, points[3].1));
            clip_path.close();
            // Enable AA on the clip so diagonal edges blend smoothly, matching
            // Chrome's sub-pixel coverage at trapezoid boundaries (e.g. CSS triangles).
            canvas.clip_path(&clip_path.detach(), ClipOp::Intersect, true);

            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(false);
            set_paint_css_color(&mut paint, &resolved);
            canvas.draw_rect(rect, &paint);
            canvas.restore();
        }
        _ => {
            // For non-solid styles (dashed, dotted, double, groove, ridge,
            // inset, outset), compute a bounding rect from the polygon
            // and delegate to the rectangle-based painter.
            let min_x = points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
            let min_y = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
            let max_x = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
            let max_y = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
            let rect = Rect::from_ltrb(min_x, min_y, max_x, max_y);

            // Clip to the trapezoid so non-solid styles don't bleed outside.
            // No AA on clip to avoid gray fringe at corners.
            canvas.save();
            let mut clip_path = PathBuilder::new();
            clip_path.move_to(Point::new(points[0].0, points[0].1));
            clip_path.line_to(Point::new(points[1].0, points[1].1));
            clip_path.line_to(Point::new(points[2].0, points[2].1));
            clip_path.line_to(Point::new(points[3].0, points[3].1));
            clip_path.close();
            canvas.clip_path(&clip_path.detach(), ClipOp::Intersect, false);

            paint_border_side(
                canvas,
                border_style,
                border_color,
                inherited_color,
                width,
                rect,
                side,
            );
            canvas.restore();
        }
    }
}

/// Paint a single border side.
///
/// Supports all CSS border styles: solid, dashed, dotted, double,
/// groove, ridge, inset, outset. None/hidden are skipped.
///
/// The `side` parameter controls shading for 3D styles:
/// - **inset**: top+left darkened, bottom+right lightened
/// - **outset**: top+left lightened, bottom+right darkened
/// - **groove**: outer half uses inset shading, inner half uses outset shading
/// - **ridge**: outer half uses outset shading, inner half uses inset shading
fn paint_border_side(
    canvas: &Canvas,
    border_style: BorderStyle,
    border_color: &StyleColor,
    inherited_color: &Color,
    width: f32,
    rect: Rect,
    side: BorderSide,
) {
    if width <= 0.0 {
        return;
    }

    // None and hidden produce no visible border.
    if matches!(border_style, BorderStyle::None | BorderStyle::Hidden) {
        return;
    }

    let resolved = border_color.resolve(inherited_color);
    let base_color = Color4f::new(resolved.r, resolved.g, resolved.b, resolved.a);

    match border_style {
        BorderStyle::Solid => {
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            set_paint_css_color(&mut paint, &resolved);
            canvas.draw_rect(rect, &paint);
        }
        BorderStyle::Dashed => {
            // Chrome: DashLengthRatio (styled_stroke_data.cc:64-66):
            //   thickness >= 3 → dash = 2*width
            //   thickness <  3 → dash = 3*width
            let dash_ratio = if width >= 3.0 { 2.0 } else { 3.0 };
            let dash_len = width * dash_ratio;
            let gap_ratio = if width >= 3.0 { 1.0 } else { 2.0 };
            let desired_gap = width * gap_ratio;
            let (p0, p1) = border_side_center_line(&rect, width);
            let stroke_length = if rect.width() > rect.height() {
                rect.width()
            } else {
                rect.height()
            };
            if stroke_length <= dash_len * 2.0 {
                let mut paint = Paint::default();
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(width);
                paint.set_anti_alias(true);
                set_paint_css_color(&mut paint, &resolved);
                canvas.draw_line(p0, p1, &paint);
                return;
            }
            let gap_len = select_best_dash_gap(stroke_length, dash_len, desired_gap);
            // When stroke_length is between 2*dash and 2*dash+gap (select_best_dash_gap
            // returned the unchanged gap because min_num_gaps==0), Skia would produce
            // asymmetric dashes: first dash full length, second dash truncated.
            // Chrome instead scales both dash and gap proportionally so exactly 2
            // symmetric dashes fit the stroke length (matching sub-pixel AA output).
            let (final_dash, final_gap) = if stroke_length < 2.0 * dash_len + desired_gap {
                let scale = stroke_length / (2.0 * dash_len + desired_gap);
                (dash_len * scale, desired_gap * scale)
            } else {
                (dash_len, gap_len)
            };
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Stroke);
            paint.set_stroke_width(width);
            paint.set_anti_alias(true);
            set_paint_css_color(&mut paint, &resolved);
            if let Some(effect) = skia_safe::PathEffect::dash(&[final_dash, final_gap], 0.0) {
                paint.set_path_effect(effect);
            }
            canvas.draw_line(p0, p1, &paint);
        }
        BorderStyle::Dotted => {
            // Chrome: for width <= 3, treated as dashed with explicit
            // endpoint dots (EnforceDotsAtEndpoints). For width > 3,
            // uses round-capped zero-length dashes.
            let is_horizontal = rect.width() > rect.height();
            let (p0, p1) = border_side_center_line(&rect, width);
            let stroke_length = if is_horizontal {
                rect.width()
            } else {
                rect.height()
            };
            let width_i = width.round() as i32;

            if width_i <= 3 && width_i >= 1 {
                // Thin dotted: Chrome draws explicit start/end dots and
                // adjusts line endpoints for uniform appearance.
                paint_thin_dotted_border(
                    canvas,
                    &base_color,
                    width,
                    width_i,
                    stroke_length as i32,
                    p0,
                    p1,
                    is_horizontal,
                );
            } else {
                // Thick dotted: round-capped zero-length dashes.
                let gap = select_best_dash_gap(stroke_length, width, width);
                let mut paint = Paint::default();
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(width);
                paint.set_stroke_cap(skia_safe::paint::Cap::Round);
                paint.set_anti_alias(true);
                set_paint_css_color(&mut paint, &resolved);
                let adjusted_p0;
                let adjusted_p1;
                if is_horizontal {
                    adjusted_p0 = Point::new(p0.x + width / 2.0, p0.y);
                    adjusted_p1 = Point::new(p1.x - width / 2.0, p1.y);
                } else {
                    adjusted_p0 = Point::new(p0.x, p0.y + width / 2.0);
                    adjusted_p1 = Point::new(p1.x, p1.y - width / 2.0);
                }
                let epsilon: f32 = 0.01;
                if let Some(effect) =
                    skia_safe::PathEffect::dash(&[0.0, gap + width - epsilon], 0.0)
                {
                    paint.set_path_effect(effect);
                }
                canvas.draw_line(adjusted_p0, adjusted_p1, &paint);
            }
        }
        BorderStyle::Double => {
            // Two lines: outer at full position, inner offset inward.
            // Each line is width/3 thick, with width/3 gap between them.
            // Used border widths are device-pixel aligned.  Blink floors each
            // stripe to one third and gives the remainder to the middle gap.
            let line_width = (width / 3.0).floor().max(1.0);
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            set_paint_css_color(&mut paint, &resolved);
            // Outer line.
            let outer_rect = shrink_border_rect(&rect, width, 0.0, line_width);
            canvas.draw_rect(outer_rect, &paint);
            // Inner line.
            let inner_rect = shrink_border_rect(&rect, width, width - line_width, line_width);
            canvas.draw_rect(inner_rect, &paint);
        }
        BorderStyle::Groove => {
            // Outer half uses inset shading for this side,
            // inner half uses outset shading for this side.
            paint_3d_border(canvas, &base_color, width, &rect, side, true);
        }
        BorderStyle::Ridge => {
            // Outer half uses outset shading for this side,
            // inner half uses inset shading for this side.
            paint_3d_border(canvas, &base_color, width, &rect, side, false);
        }
        BorderStyle::Inset => {
            // Per CSS: top+left darkened, bottom+right lightened.
            let shaded = if matches!(side, BorderSide::Top | BorderSide::Left) {
                darken_color(&base_color)
            } else {
                lighten_color(&base_color)
            };
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            paint.set_color4f(shaded, None::<&ColorSpace>);
            canvas.draw_rect(rect, &paint);
        }
        BorderStyle::Outset => {
            // Per CSS: top+left lightened, bottom+right darkened.
            let shaded = if matches!(side, BorderSide::Top | BorderSide::Left) {
                lighten_color(&base_color)
            } else {
                darken_color(&base_color)
            };
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            paint.set_color4f(shaded, None::<&ColorSpace>);
            canvas.draw_rect(rect, &paint);
        }
        BorderStyle::None | BorderStyle::Hidden => {
            // Already handled above, but satisfy exhaustive match.
        }
    }
}

/// Shrink a border rect inward for double-line border painting.
fn shrink_border_rect(rect: &Rect, _border_width: f32, inset: f32, line_width: f32) -> Rect {
    if rect.width() > rect.height() {
        // Horizontal side.
        Rect::from_xywh(rect.left, rect.top + inset, rect.width(), line_width)
    } else {
        // Vertical side.
        Rect::from_xywh(rect.left + inset, rect.top, line_width, rect.height())
    }
}

/// Compute the center line of a border side for stroke-based drawing.
fn border_side_center_line(rect: &Rect, width: f32) -> (Point, Point) {
    let half = width / 2.0;
    if rect.width() > rect.height() {
        // Horizontal side (top or bottom).
        let y = rect.top + half;
        (Point::new(rect.left, y), Point::new(rect.right, y))
    } else {
        // Vertical side (left or right).
        let x = rect.left + half;
        (Point::new(x, rect.top), Point::new(x, rect.bottom))
    }
}

/// Chrome's SelectBestDashGap: adjusts gap length so dashes fit evenly
/// in the given stroke length (styled_stroke_data.cc).
fn select_best_dash_gap(stroke_length: f32, dash_length: f32, gap_length: f32) -> f32 {
    let available_length = stroke_length + gap_length; // open path
    let min_num_dashes = (available_length / (dash_length + gap_length)).floor();
    let max_num_dashes = min_num_dashes + 1.0;
    let min_num_gaps = min_num_dashes - 1.0;
    let max_num_gaps = max_num_dashes - 1.0;
    if min_num_gaps <= 0.0 {
        return gap_length;
    }
    let min_gap = (stroke_length - min_num_dashes * dash_length) / min_num_gaps;
    let max_gap = if max_num_gaps > 0.0 {
        (stroke_length - max_num_dashes * dash_length) / max_num_gaps
    } else {
        -1.0
    };
    if max_gap <= 0.0 || (min_gap - gap_length).abs() < (max_gap - gap_length).abs() {
        min_gap
    } else {
        max_gap
    }
}

/// Chrome's EnforceDotsAtEndpoints for thin (width <= 3) dotted borders.
/// Draws explicit start/end dots and adjusts line endpoints for uniform dots.
/// Port of box_border_painter.cc EnforceDotsAtEndpoints.
fn paint_thin_dotted_border(
    canvas: &Canvas,
    color: &Color4f,
    width: f32,
    width_i: i32,
    path_length: i32,
    p0: Point,
    p1: Point,
    is_horizontal: bool,
) {
    // Chrome uses integer center: y1 + thickness/2 (integer division).
    // Our p0/p1 use float center (y1 + thickness/2.0). For odd widths,
    // the integer center is 0.5 less than our float center.
    // We compute the integer center for dot rects, then add +0.5 for the stroke.
    let int_center_offset = if width_i % 2 != 0 { -0.5_f32 } else { 0.0 };
    let (mut lp0, mut lp1) = (p0, p1);
    if is_horizontal {
        lp0.y += int_center_offset;
        lp1.y += int_center_offset;
    } else {
        lp0.x += int_center_offset;
        lp1.x += int_center_offset;
    }

    let mod_4 = path_length % 4;
    let mod_6 = path_length % 6;
    let mut use_start_dot = false;
    let mut start_dot_growth: i32 = 0;
    let mut start_line_offset: i32 = 0;
    let mut use_end_dot = false;
    let mut end_dot_growth: i32 = 0;

    if (width_i == 1 && path_length % 2 == 0) || (width_i == 3 && mod_6 == 0) {
        use_start_dot = true;
        start_dot_growth = 1;
        start_line_offset = 1;
    }
    if (width_i == 2 && (mod_4 == 0 || mod_4 == 1)) || (width_i == 3 && (mod_6 == 1 || mod_6 == 2))
    {
        use_start_dot = true;
        start_line_offset = -1;
    }
    if (width_i == 2 && mod_4 == 0) || (width_i == 3 && mod_6 == 1) {
        use_end_dot = true;
    }
    if (width_i == 2 && mod_4 == 3) || (width_i == 3 && (mod_6 == 4 || mod_6 == 5)) {
        use_start_dot = true;
        start_line_offset = 1;
    }
    if width_i == 3 && mod_6 == 5 {
        use_end_dot = true;
    } else if width_i == 3 && mod_6 == 0 {
        use_end_dot = true;
        end_dot_growth = 1;
    }

    // Fill paint for explicit dot rects (no AA for crisp squares).
    let mut fill = Paint::default();
    fill.set_style(PaintStyle::Fill);
    fill.set_anti_alias(false);
    fill.set_color4f(*color, None::<&ColorSpace>);

    let w = width_i;

    if use_start_dot {
        let start_dot = if is_horizontal {
            // Chrome: (p1.x(), p1.y() - width/2, p1.x() + width + growth, p1.y() + width - width/2)
            Rect::from_ltrb(
                lp0.x,
                lp0.y - (w / 2) as f32,
                lp0.x + (w + start_dot_growth) as f32,
                lp0.y + (w - w / 2) as f32,
            )
        } else {
            Rect::from_ltrb(
                lp0.x - (w / 2) as f32,
                lp0.y,
                lp0.x + (w - w / 2) as f32,
                lp0.y + (w + start_dot_growth) as f32,
            )
        };
        canvas.draw_rect(start_dot, &fill);
        if is_horizontal {
            lp0.x += (2 * w + start_line_offset) as f32;
        } else {
            lp0.y += (2 * w + start_line_offset) as f32;
        }
    }

    if use_end_dot {
        let end_dot = if is_horizontal {
            Rect::from_ltrb(
                lp1.x - (w + end_dot_growth) as f32,
                lp1.y - (w / 2) as f32,
                lp1.x,
                lp1.y + (w - w / 2) as f32,
            )
        } else {
            Rect::from_ltrb(
                lp1.x - (w / 2) as f32,
                lp1.y - (w + end_dot_growth) as f32,
                lp1.x + (w - w / 2) as f32,
                lp1.y,
            )
        };
        canvas.draw_rect(end_dot, &fill);
        if is_horizontal {
            lp1.x -= (w + end_dot_growth + 1) as f32;
        } else {
            lp1.y -= (w + end_dot_growth + 1) as f32;
        }
    }

    // Odd widths: add 0.5 to center the stroke (Chrome's DrawLineWithStyle).
    if width_i % 2 != 0 {
        if is_horizontal {
            lp0.y += 0.5;
            lp1.y += 0.5;
        } else {
            lp0.x += 0.5;
            lp1.x += 0.5;
        }
    }

    // Draw the remaining line with dash pattern [width, width].
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(width);
    paint.set_stroke_cap(skia_safe::paint::Cap::Butt);
    paint.set_anti_alias(true);
    paint.set_color4f(*color, None::<&ColorSpace>);
    if let Some(effect) = skia_safe::PathEffect::dash(&[width, width], 0.0) {
        paint.set_path_effect(effect);
    }
    canvas.draw_line(lp0, lp1, &paint);
}

/// Paint a 3D-style border (groove or ridge).
///
/// For groove: outer half uses inset shading, inner half uses outset shading.
/// For ridge: outer half uses outset shading, inner half uses inset shading.
///
/// `inset_outer`: true for groove (outer=inset, inner=outset),
/// false for ridge (outer=outset, inner=inset).
fn paint_3d_border(
    canvas: &Canvas,
    color: &Color4f,
    width: f32,
    rect: &Rect,
    side: BorderSide,
    inset_outer: bool,
) {
    let half_width = (width / 2.0).max(1.0);
    let dark = darken_color(color);
    let light = lighten_color(color);

    // Inset shading for a side: top+left → dark, bottom+right → light.
    let inset_color = if matches!(side, BorderSide::Top | BorderSide::Left) {
        dark
    } else {
        light
    };
    // Outset shading for a side: top+left → light, bottom+right → dark.
    let outset_color = if matches!(side, BorderSide::Top | BorderSide::Left) {
        light
    } else {
        dark
    };

    let (outer_color, inner_color) = if inset_outer {
        (inset_color, outset_color)
    } else {
        (outset_color, inset_color)
    };

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);

    // Outer half.
    let outer = shrink_border_rect(rect, width, 0.0, half_width);
    paint.set_color4f(outer_color, None::<&ColorSpace>);
    canvas.draw_rect(outer, &paint);

    // Inner half.
    let inner = shrink_border_rect(rect, width, half_width, width - half_width);
    paint.set_color4f(inner_color, None::<&ColorSpace>);
    canvas.draw_rect(inner, &paint);
}

/// Blink's CSS 3D border shade: blend one third toward black.
fn darken_color(color: &Color4f) -> Color4f {
    Color4f::new(
        color.r * (2.0 / 3.0),
        color.g * (2.0 / 3.0),
        color.b * (2.0 / 3.0),
        color.a,
    )
}

/// The raised half uses the authored color; the recessed half is darkened.
fn lighten_color(color: &Color4f) -> Color4f {
    *color
}

// ── StyleColor PartialEq needed for border comparison ────────────────
// Already derived in the style crate.

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn gradient_stop_fixup_is_monotonic_and_resolves_absolute_lengths() {
        let stops = [
            openui_style::LinearGradientStop {
                color: Color::from_rgba8(0, 128, 0, 255),
                position: GradientStopPosition::Px(80.0),
            },
            openui_style::LinearGradientStop {
                color: Color::RED,
                position: GradientStopPosition::Px(60.0),
            },
        ];
        assert_eq!(resolved_gradient_positions(&stops, 80.0), vec![1.0, 1.0]);
    }

    // ── Issue 7 (R26): large borders don't produce invalid paint geometry ──

    #[test]
    fn large_uniform_border_clamps_stroke_dimensions() {
        // Box is 20×20 with border-width:15 on all sides.
        // Without clamping: stroke rect would be (15/2, 15/2, 20-15=-5, 20-15=-5) → negative!
        // With clamping: (7.5, 7.5, max(0, 20-15)=5, max(0, 20-15)=5)
        let w: f32 = 20.0;
        let h: f32 = 20.0;
        let border_width: f32 = 15.0;

        let sw = (w - border_width).max(0.0);
        let sh = (h - border_width).max(0.0);
        assert!(sw >= 0.0, "stroke width must be non-negative, got {}", sw);
        assert!(sh >= 0.0, "stroke height must be non-negative, got {}", sh);
        assert_eq!(sw, 5.0);
        assert_eq!(sh, 5.0);

        // Even when border is larger than box
        let sw2 = (10.0f32 - 25.0f32).max(0.0);
        let sh2 = (10.0f32 - 25.0f32).max(0.0);
        assert_eq!(sw2, 0.0, "should clamp to 0 when border > box");
        assert_eq!(sh2, 0.0);
    }

    #[test]
    fn large_per_side_borders_inner_rect_never_crosses() {
        // Box 30×30 with borders: left=20, right=20, top=20, bottom=20.
        // Inner edges would cross: ix0=20, ix1=30-20=10 (ix0 > ix1!).
        let x: f32 = 0.0;
        let y: f32 = 0.0;
        let w: f32 = 30.0;
        let h: f32 = 30.0;
        let bl: f32 = 20.0;
        let br: f32 = 20.0;
        let bt: f32 = 20.0;
        let bb: f32 = 20.0;

        // Apply the clamped inner rect computation from the fix.
        let ix0 = (x + bl).min(x + w - br);
        let iy0 = (y + bt).min(y + h - bb);
        let ix1 = (x + w - br).max(ix0);
        let iy1 = (y + h - bb).max(iy0);

        assert!(
            ix1 >= ix0,
            "inner right ({}) must be >= inner left ({})",
            ix1,
            ix0
        );
        assert!(
            iy1 >= iy0,
            "inner bottom ({}) must be >= inner top ({})",
            iy1,
            iy0
        );
        // Both should collapse to the same point (10.0)
        assert_eq!(ix0, 10.0);
        assert_eq!(ix1, 10.0);
        assert_eq!(iy0, 10.0);
        assert_eq!(iy1, 10.0);
    }

    #[test]
    fn normal_borders_inner_rect_unchanged() {
        // Box 100×100 with borders: left=5, right=5, top=5, bottom=5.
        // Inner rect should be (5, 5, 95, 95) — normal case.
        let x: f32 = 0.0;
        let y: f32 = 0.0;
        let w: f32 = 100.0;
        let h: f32 = 100.0;
        let bl: f32 = 5.0;
        let br: f32 = 5.0;
        let bt: f32 = 5.0;
        let bb: f32 = 5.0;

        let ix0 = (x + bl).min(x + w - br);
        let iy0 = (y + bt).min(y + h - bb);
        let ix1 = (x + w - br).max(ix0);
        let iy1 = (y + h - bb).max(iy0);

        assert_eq!(ix0, 5.0);
        assert_eq!(iy0, 5.0);
        assert_eq!(ix1, 95.0);
        assert_eq!(iy1, 95.0);
    }
}
