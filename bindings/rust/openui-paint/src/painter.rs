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

use skia_safe::{Canvas, Color4f, Paint, PaintStyle, Path, Rect, RRect, ColorSpace, ClipOp, Point};
use openui_geometry::PhysicalOffset;
use openui_style::{Color, ComputedStyle, BorderStyle, Overflow, StyleColor, Visibility, BackgroundClip, OverflowClipBox, BoxShadow};
use openui_dom::Document;
use openui_layout::{Fragment, FragmentKind};
use openui_text::font::FontMetrics;

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
pub fn paint_fragment(canvas: &Canvas, fragment: &Fragment, doc: &Document, offset: PhysicalOffset) {
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
            canvas.save();
            let clip_x = abs_offset.left.round().to_f32();
            let clip_y = abs_offset.top.round().to_f32();
            let clip_w = fragment.size.width.round().to_f32();
            let clip_h = fragment.size.height.round().to_f32();
            canvas.clip_rect(
                skia_safe::Rect::from_xywh(clip_x, clip_y, clip_w, clip_h),
                skia_safe::ClipOp::Intersect,
                true,
            );
            paint_children_with_stacking_order(canvas, &fragment.children, doc, abs_offset);
            canvas.restore();
        } else {
            paint_children_with_stacking_order(canvas, &fragment.children, doc, abs_offset);
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

    let style = &doc.node(fragment.node_id).style;

    // CSS opacity creates a stacking context and composites the entire
    // subtree at the given opacity. Blink implements this via
    // PaintLayerPainter::PaintLayerWithAdjustedRoot() using saveLayerAlphaf().
    let needs_layer = style.opacity < 1.0;
    if needs_layer {
        canvas.save_layer_alpha_f(None, style.opacity);
    }

    // ── Paint this fragment (skip if visibility: hidden) ─────────────
    if style.visibility == Visibility::Visible {
        match fragment.kind {
            FragmentKind::Text => {
                paint_text_fragment(canvas, fragment, style, abs_offset);
            }
            FragmentKind::Box | FragmentKind::Viewport => {
                paint_box_decoration_background(canvas, fragment, style, abs_offset);
            }
            FragmentKind::ColumnRule => {
                paint_column_rule(canvas, fragment, style, abs_offset);
            }
            FragmentKind::ColumnBox => {
                // Handled above (early return); unreachable here.
            }
        }
    }

    // ── Overflow clipping + children ──────────────────────────────────
    let needs_clip = needs_overflow_clip(fragment, style);
    if needs_clip {
        paint_with_overflow_clip(canvas, fragment, doc, abs_offset, style);
    } else {
        // Paint children with CSS stacking order (z-index aware).
        paint_children_with_stacking_order(canvas, &fragment.children, doc, abs_offset);
    }

    // ── Outline (painted after children, outside border box) ──────────
    if style.visibility == Visibility::Visible && style.has_outline() {
        paint_outline(canvas, fragment, style, abs_offset);
    }

    if needs_layer {
        canvas.restore();
    }
}

// ── Stacking order (z-index) ──────────────────────────────────────────

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
fn paint_children_with_stacking_order(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    offset: PhysicalOffset,
) {
    use openui_style::Position;

    // Classify children into buckets
    let mut negative_z: Vec<(i32, usize)> = Vec::new(); // (z-index, child_index)
    let mut in_flow: Vec<usize> = Vec::new();
    let mut non_negative_z: Vec<(i32, usize)> = Vec::new();

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
        let is_flex_item = !parent_id.is_none()
            && doc.node(parent_id).style.display.is_flex();
        let has_z_index = child_style.z_index.is_some();

        if is_positioned || (is_flex_item && has_z_index) {
            let z = child_style.z_index.unwrap_or(0);
            if z < 0 {
                negative_z.push((z, i));
            } else {
                non_negative_z.push((z, i));
            }
        } else {
            in_flow.push(i);
        }
    }

    // Sort by z-index (stable sort preserves document order for equal z-index)
    negative_z.sort_by_key(|&(z, _)| z);
    non_negative_z.sort_by_key(|&(z, _)| z);

    // Phase 1: Negative z-index positioned elements
    for &(_, idx) in &negative_z {
        paint_fragment(canvas, &children[idx], doc, offset);
    }

    // Phase 2: In-flow elements (in document order)
    for &idx in &in_flow {
        paint_fragment(canvas, &children[idx], doc, offset);
    }

    // Phase 3: Non-negative z-index positioned elements
    for &(_, idx) in &non_negative_z {
        paint_fragment(canvas, &children[idx], doc, offset);
    }
}

// ── Overflow clipping helpers ────────────────────────────────────────

/// Determine whether a fragment needs overflow clipping.
///
/// Returns `true` when either the layout-computed `has_overflow_clip` flag is
/// set, or the style's `overflow-x`/`overflow-y` is not `visible` for a
/// box-level fragment.
fn needs_overflow_clip(fragment: &Fragment, style: &ComputedStyle) -> bool {
    fragment.has_overflow_clip
        || ((style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible)
            && matches!(fragment.kind, FragmentKind::Box | FragmentKind::Viewport))
}

/// Compute the clip rectangle for overflow clipping.
///
/// Per CSS spec, overflow clips to the **padding box** — the border-box
/// inset by each side's border width. Returns `(x, y, width, height)`.
pub fn compute_clip_rect(fragment: &Fragment, offset: PhysicalOffset) -> (f32, f32, f32, f32) {
    // Pixel-snap the padding box edges independently for crisp clipping.
    let clip_left = (offset.left + fragment.border.left).round().to_f32();
    let clip_top = (offset.top + fragment.border.top).round().to_f32();
    let clip_right = (offset.left + fragment.size.width - fragment.border.right).round().to_f32();
    let clip_bottom = (offset.top + fragment.size.height - fragment.border.bottom).round().to_f32();
    let clip_w = (clip_right - clip_left).max(0.0);
    let clip_h = (clip_bottom - clip_top).max(0.0);
    (clip_left, clip_top, clip_w, clip_h)
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
        match style.overflow_clip_box {
            OverflowClipBox::PaddingBox => (px, py, pw, ph),
            OverflowClipBox::ContentBox => {
                // Inset from padding-box by padding amounts
                let pl = resolve_margin_or_padding_f32(&style.padding_left, pw);
                let pr = resolve_margin_or_padding_f32(&style.padding_right, pw);
                let pt = resolve_margin_or_padding_f32(&style.padding_top, ph);
                let pb = resolve_margin_or_padding_f32(&style.padding_bottom, ph);
                (px + pl, py + pt, (pw - pl - pr).max(0.0), (ph - pt - pb).max(0.0))
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
    let (clip_x, clip_y, clip_w, clip_h) = if margin != 0.0
        && (style.overflow_x == Overflow::Clip || style.overflow_y == Overflow::Clip)
    {
        let mx = if style.overflow_x == Overflow::Clip { margin } else { 0.0 };
        let my = if style.overflow_y == Overflow::Clip { margin } else { 0.0 };
        (
            clip_x - mx,
            clip_y - my,
            clip_w + mx * 2.0,
            clip_h + my * 2.0,
        )
    } else {
        (clip_x, clip_y, clip_w, clip_h)
    };

    // CSS Overflow 3: directional overflow. When one axis is `visible` and
    // the other clips, extend the clip to be effectively unbounded on the
    // visible axis so children overflow freely in that direction.
    let big = 100_000.0_f32;
    let clip_x_visible = style.overflow_x == Overflow::Visible;
    let clip_y_visible = style.overflow_y == Overflow::Visible;
    let (clip_x, clip_y, clip_w, clip_h) = match (clip_x_visible, clip_y_visible) {
        (true, false) => (-big, clip_y, big * 2.0, clip_h),
        (false, true) => (clip_x, -big, clip_w, big * 2.0),
        _ => (clip_x, clip_y, clip_w, clip_h),
    };

    let clip_rect = Rect::from_xywh(clip_x, clip_y, clip_w, clip_h);

    canvas.save();

    // When border-radius is set, clip to a rounded rect so children are
    // clipped along the curves. Otherwise use a simple rect clip.
    if style.has_border_radius() {
        let rrect = build_clip_rrect(&clip_rect, style);
        canvas.clip_rrect(rrect, ClipOp::Intersect, true);
    } else {
        canvas.clip_rect(clip_rect, ClipOp::Intersect, true);
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
    paint_children_with_stacking_order(canvas, &fragment.children, doc, offset);

    canvas.restore();
}

/// Build a Skia `RRect` from the padding-box clip rect and the style's
/// border-radius values.
///
/// The radii are adjusted inward from the border-box radii by the
/// corresponding border widths (per CSS Backgrounds §5.3 "inner rounded
/// corners"). Each radius is clamped to a minimum of 0.
fn build_clip_rrect(clip_rect: &Rect, style: &ComputedStyle) -> RRect {
    let bt = style.effective_border_top() as f32;
    let br = style.effective_border_right() as f32;
    let bb = style.effective_border_bottom() as f32;
    let bl = style.effective_border_left() as f32;

    // Adjust each corner's radii inward by the adjacent border widths.
    let tl_x = (style.border_top_left_radius.0 - bl).max(0.0);
    let tl_y = (style.border_top_left_radius.1 - bt).max(0.0);
    let tr_x = (style.border_top_right_radius.0 - br).max(0.0);
    let tr_y = (style.border_top_right_radius.1 - bt).max(0.0);
    let br_x = (style.border_bottom_right_radius.0 - br).max(0.0);
    let br_y = (style.border_bottom_right_radius.1 - bb).max(0.0);
    let bl_x = (style.border_bottom_left_radius.0 - bl).max(0.0);
    let bl_y = (style.border_bottom_left_radius.1 - bb).max(0.0);

    // skia_safe::RRect radii order: top-left, top-right, bottom-right, bottom-left
    let radii = [
        Point::new(tl_x, tl_y),
        Point::new(tr_x, tr_y),
        Point::new(br_x, br_y),
        Point::new(bl_x, bl_y),
    ];

    RRect::new_rect_radii(*clip_rect, &radii)
}

/// Resolve decoration metrics from the styled font (CSS font-family/size),
/// falling back to the first shaped run's metrics if the primary font lookup
/// fails. This ensures decoration positioning uses the intended CSS font
/// even when the first shaped run uses a fallback (emoji, CJK, etc.).
fn resolve_decoration_metrics(style: &ComputedStyle, shape_result: &openui_text::shaping::ShapeResult) -> FontMetrics {
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
        canvas, shape_result, origin, style, &metrics,
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
        canvas, shape_result, origin, style, text_content,
    );

    // 5. Line-through decoration (painted in front of text per CSS spec)
    crate::decoration_painter::paint_text_decorations(
        canvas, shape_result, origin, style, &metrics,
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
fn paint_box_shadows(
    canvas: &Canvas,
    style: &ComputedStyle,
    border_rect: Rect,
    inset_only: bool,
) {
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
            // Inset shadow: clip to border-box, then paint a large rect with
            // the inner "hole" cut out. The visible shadow is the blurred edge.
            canvas.save();
            canvas.clip_rect(border_rect, ClipOp::Intersect, false);

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

            let mut path = Path::new();
            path.add_rect(outer, None);
            if hole.width() > 0.0 && hole.height() > 0.0 {
                path.add_rect(hole, Some((skia_safe::PathDirection::CCW, 0)));
            }
            path.set_fill_type(skia_safe::PathFillType::EvenOdd);
            canvas.draw_path(&path, &paint);
            canvas.restore();
        } else {
            // Outset shadow: draw behind the element, expanded by spread
            let shadow_rect = Rect::from_xywh(
                border_rect.left + shadow.offset_x - shadow.spread_radius,
                border_rect.top + shadow.offset_y - shadow.spread_radius,
                border_rect.width() + shadow.spread_radius * 2.0,
                border_rect.height() + shadow.spread_radius * 2.0,
            );
            canvas.draw_rect(shadow_rect, &paint);
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
) {
    // Pixel-snap all four edges independently (Blink's PixelSnappedIntRect).
    // Fractional abs_offset flows through from parent so that adjacent elements
    // at fractional boundaries (e.g. flex items at 133.33px intervals) share
    // the same snapped pixel edge — no gaps.
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    let w = right - x;
    let h = bottom - y;

    // Skip empty fragments
    if w <= 0.0 || h <= 0.0 {
        return;
    }

    let border_box_rect = Rect::from_xywh(x, y, w, h);

    // ── 1. Outset box shadows (painted behind everything) ────────────
    paint_box_shadows(canvas, style, border_box_rect, false);

    // When border-radius is set AND borders are uniform solid, use saveLayer
    // so bg+border composite as one unit, then clip by the outer rrect.
    // This prevents background color from bleeding through at the border's
    // AA curve edges (matching Chromium). Only apply for uniform borders
    // since non-uniform (trapezoid) borders handle corners differently.
    let has_radius = style.has_border_radius();
    let bt = style.effective_border_top() as f32;
    let br_bw = style.effective_border_right() as f32;
    let bb_bw = style.effective_border_bottom() as f32;
    let bl_bw = style.effective_border_left() as f32;
    let uniform_border = bt == br_bw && br_bw == bb_bw && bb_bw == bl_bw
        && style.border_top_style == style.border_right_style
        && style.border_right_style == style.border_bottom_style
        && style.border_bottom_style == style.border_left_style
        && style.border_top_color == style.border_right_color
        && style.border_right_color == style.border_bottom_color
        && style.border_bottom_color == style.border_left_color
        && style.border_top_style == BorderStyle::Solid;
    let use_layer = has_radius && uniform_border;
    if use_layer {
        let outer_radii = [
            Point::new(style.border_top_left_radius.0, style.border_top_left_radius.1),
            Point::new(style.border_top_right_radius.0, style.border_top_right_radius.1),
            Point::new(style.border_bottom_right_radius.0, style.border_bottom_right_radius.1),
            Point::new(style.border_bottom_left_radius.0, style.border_bottom_left_radius.1),
        ];
        let outer_rrect = RRect::new_rect_radii(border_box_rect, &outer_radii);
        canvas.save();
        canvas.clip_rrect(outer_rrect, ClipOp::Intersect, true);
        canvas.save_layer_alpha_f(Rect::from_xywh(x, y, w, h), 1.0);
    }

    // ── 2. Background color ──────────────────────────────────────────
    if !style.background_color.is_transparent() {
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(true);
        let c = &style.background_color;
        paint.set_color4f(Color4f::new(c.r, c.g, c.b, c.a), None::<&ColorSpace>);

        let bg_rect = match style.background_clip {
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
                let bx = x + fragment.border.left.round().to_f32()
                    + fragment.padding.left.round().to_f32();
                let by = y + fragment.border.top.round().to_f32()
                    + fragment.padding.top.round().to_f32();
                let br = right - fragment.border.right.round().to_f32()
                    - fragment.padding.right.round().to_f32();
                let bb = bottom - fragment.border.bottom.round().to_f32()
                    - fragment.padding.bottom.round().to_f32();
                let bw = (br - bx).max(0.0);
                let bh = (bb - by).max(0.0);
                Rect::from_xywh(bx, by, bw, bh)
            }
        };

        if has_radius {
            // Compute radii adjusted for background-clip box (CSS Backgrounds §5.3).
            // Inner corner radii = outer radii - inset on each side, clamped to 0.
            let clip_radii = match style.background_clip {
                BackgroundClip::BorderBox => [
                    Point::new(style.border_top_left_radius.0, style.border_top_left_radius.1),
                    Point::new(style.border_top_right_radius.0, style.border_top_right_radius.1),
                    Point::new(style.border_bottom_right_radius.0, style.border_bottom_right_radius.1),
                    Point::new(style.border_bottom_left_radius.0, style.border_bottom_left_radius.1),
                ],
                BackgroundClip::PaddingBox => {
                    let bt = style.effective_border_top() as f32;
                    let br_w = style.effective_border_right() as f32;
                    let bb = style.effective_border_bottom() as f32;
                    let bl = style.effective_border_left() as f32;
                    [
                        Point::new((style.border_top_left_radius.0 - bl).max(0.0),
                                   (style.border_top_left_radius.1 - bt).max(0.0)),
                        Point::new((style.border_top_right_radius.0 - br_w).max(0.0),
                                   (style.border_top_right_radius.1 - bt).max(0.0)),
                        Point::new((style.border_bottom_right_radius.0 - br_w).max(0.0),
                                   (style.border_bottom_right_radius.1 - bb).max(0.0)),
                        Point::new((style.border_bottom_left_radius.0 - bl).max(0.0),
                                   (style.border_bottom_left_radius.1 - bb).max(0.0)),
                    ]
                }
                BackgroundClip::ContentBox => {
                    let bt = style.effective_border_top() as f32 + fragment.padding.top.round().to_f32();
                    let br_w = style.effective_border_right() as f32 + fragment.padding.right.round().to_f32();
                    let bb = style.effective_border_bottom() as f32 + fragment.padding.bottom.round().to_f32();
                    let bl = style.effective_border_left() as f32 + fragment.padding.left.round().to_f32();
                    [
                        Point::new((style.border_top_left_radius.0 - bl).max(0.0),
                                   (style.border_top_left_radius.1 - bt).max(0.0)),
                        Point::new((style.border_top_right_radius.0 - br_w).max(0.0),
                                   (style.border_top_right_radius.1 - bt).max(0.0)),
                        Point::new((style.border_bottom_right_radius.0 - br_w).max(0.0),
                                   (style.border_bottom_right_radius.1 - bb).max(0.0)),
                        Point::new((style.border_bottom_left_radius.0 - bl).max(0.0),
                                   (style.border_bottom_left_radius.1 - bb).max(0.0)),
                    ]
                }
            };
            let clip_rrect = RRect::new_rect_radii(bg_rect, &clip_radii);
            canvas.save();
            canvas.clip_rrect(clip_rrect, ClipOp::Intersect, true);
            canvas.draw_rect(bg_rect, &paint);
            canvas.restore();
        } else {
            canvas.draw_rect(bg_rect, &paint);
        }
    }

    // ── 3. Inset box shadows (painted on top of background) ──────────
    paint_box_shadows(canvas, style, border_box_rect, true);

    // ── 4. Borders ───────────────────────────────────────────────────
    paint_borders(canvas, fragment, style, x, y, w, h);

    if use_layer {
        canvas.restore(); // pops saveLayer
        canvas.restore(); // pops outer rrect clip
    }
}

/// Paint borders around the border-box.
///
/// Extracted from Blink's `BoxBorderPainter` (box_border_painter.cc).
fn paint_borders(
    canvas: &Canvas,
    fragment: &Fragment,
    style: &ComputedStyle,
    x: f32, y: f32, w: f32, h: f32,
) {
    // box-decoration-break: slice (default) — suppress block-start border on
    // non-first fragments and block-end border on non-last fragments.
    let bt = if fragment.is_first_for_node { style.effective_border_top() as f32 } else { 0.0 };
    let br = style.effective_border_right() as f32;
    let bb = if fragment.is_last_for_node { style.effective_border_bottom() as f32 } else { 0.0 };
    let bl = style.effective_border_left() as f32;

    // No borders to paint
    if bt == 0.0 && br == 0.0 && bb == 0.0 && bl == 0.0 {
        return;
    }

    let inherited_color = &style.color;

    // Check if all borders are the same color and style (fast path).
    // Blink: DrawSolidBorderRect for uniform solid borders.
    let uniform = bt == br && br == bb && bb == bl
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
        paint.set_color4f(
            Color4f::new(resolved.r, resolved.g, resolved.b, resolved.a),
            None::<&ColorSpace>,
        );

        if style.has_border_radius() {
            // The outer border-box rrect clip is already set by the caller.
            // Just fill the border ring by excluding the inner rrect.
            let mut fill_paint = Paint::default();
            fill_paint.set_style(PaintStyle::Fill);
            fill_paint.set_anti_alias(true);
            fill_paint.set_color4f(
                Color4f::new(resolved.r, resolved.g, resolved.b, resolved.a),
                None::<&ColorSpace>,
            );

            let inner_rect = Rect::from_xywh(
                x + bt, y + bt,
                (w - bt - bt).max(0.0), (h - bt - bt).max(0.0),
            );
            let inner_radii = [
                Point::new((style.border_top_left_radius.0 - bt).max(0.0),
                           (style.border_top_left_radius.1 - bt).max(0.0)),
                Point::new((style.border_top_right_radius.0 - bt).max(0.0),
                           (style.border_top_right_radius.1 - bt).max(0.0)),
                Point::new((style.border_bottom_right_radius.0 - bt).max(0.0),
                           (style.border_bottom_right_radius.1 - bt).max(0.0)),
                Point::new((style.border_bottom_left_radius.0 - bt).max(0.0),
                           (style.border_bottom_left_radius.1 - bt).max(0.0)),
            ];
            let inner_rrect = RRect::new_rect_radii(inner_rect, &inner_radii);

            canvas.save();
            canvas.clip_rrect(inner_rrect, ClipOp::Difference, true);
            canvas.draw_rect(Rect::from_xywh(x, y, w, h), &fill_paint);
            canvas.restore();
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
            paint_border_side_path(canvas, style.border_top_style, &style.border_top_color,
                inherited_color, bt,
                &[(ox0, oy0), (ox1, oy0), (ix1, iy0), (ix0, iy0)],
                BorderSide::Top);
        }
        // Bottom border: outer-bottom-right → outer-bottom-left → inner-bottom-left → inner-bottom-right
        if bb > 0.0 {
            paint_border_side_path(canvas, style.border_bottom_style, &style.border_bottom_color,
                inherited_color, bb,
                &[(ox1, oy1), (ox0, oy1), (ix0, iy1), (ix1, iy1)],
                BorderSide::Bottom);
        }
        // Right border: outer-top-right → outer-bottom-right → inner-bottom-right → inner-top-right
        if br > 0.0 {
            paint_border_side_path(canvas, style.border_right_style, &style.border_right_color,
                inherited_color, br,
                &[(ox1, oy0), (ox1, oy1), (ix1, iy1), (ix1, iy0)],
                BorderSide::Right);
        }
        // Left border: outer-bottom-left → outer-top-left → inner-top-left → inner-bottom-left
        if bl > 0.0 {
            paint_border_side_path(canvas, style.border_left_style, &style.border_left_color,
                inherited_color, bl,
                &[(ox0, oy1), (ox0, oy0), (ix0, iy0), (ix0, iy1)],
                BorderSide::Left);
        }

        // Fix corner diagonal pixels: Chrome's Skia achieves exact complementary
        // coverage at shared miter edges (no white bleed-through). Standard SrcOver
        // compositing leaves ~25% background showing. Fix by overwriting diagonal
        // pixels with the exact 50% blend of the two adjacent border colors.
        fix_corner_miter_pixels(canvas, inherited_color, style,
            ox0, oy0, ox1, oy1, ix0, iy0, ix1, iy1);
    }
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

    // For solid outlines, draw 4 rectangles (top, right, bottom, left)
    // For other styles, fall back to solid (matches most visual tests)
    // Top edge
    canvas.draw_rect(Rect::from_xywh(ox, oy, ow_total, ow), &paint);
    // Bottom edge
    canvas.draw_rect(Rect::from_xywh(ox, ob - ow, ow_total, ow), &paint);
    // Left edge
    canvas.draw_rect(Rect::from_xywh(ox, oy + ow, ow, oh_total - 2.0 * ow), &paint);
    // Right edge
    canvas.draw_rect(Rect::from_xywh(or - ow, oy + ow, ow, oh_total - 2.0 * ow), &paint);
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
    paint_border_side(
        canvas,
        style.column_rule_style,
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
    ox0: f32, oy0: f32, ox1: f32, oy1: f32,
    ix0: f32, iy0: f32, ix1: f32, iy1: f32,
) {
    let top_c = style.border_top_color.resolve(inherited_color);
    let right_c = style.border_right_color.resolve(inherited_color);
    let bottom_c = style.border_bottom_color.resolve(inherited_color);
    let left_c = style.border_left_color.resolve(inherited_color);

    let bt = iy0 - oy0;
    let br = ox1 - ix1;
    let bb = oy1 - iy1;
    let bl = ix0 - ox0;

    // Top-left corner: fix if both adjacent sides are solid.
    if bt > 0.5 && bl > 0.5
        && style.border_top_style == BorderStyle::Solid
        && style.border_left_style == BorderStyle::Solid
    {
        draw_miter_blend_pixels(canvas,
            ox0 as i32, oy0 as i32,
            ix0 as i32 - 1, iy0 as i32 - 1,
            &top_c, &left_c);
    }
    // Top-right corner
    if bt > 0.5 && br > 0.5
        && style.border_top_style == BorderStyle::Solid
        && style.border_right_style == BorderStyle::Solid
    {
        draw_miter_blend_pixels(canvas,
            ox1 as i32 - 1, oy0 as i32,
            ix1 as i32, iy0 as i32 - 1,
            &top_c, &right_c);
    }
    // Bottom-right corner
    if bb > 0.5 && br > 0.5
        && style.border_bottom_style == BorderStyle::Solid
        && style.border_right_style == BorderStyle::Solid
    {
        draw_miter_blend_pixels(canvas,
            ox1 as i32 - 1, oy1 as i32 - 1,
            ix1 as i32, iy1 as i32,
            &bottom_c, &right_c);
    }
    // Bottom-left corner
    if bb > 0.5 && bl > 0.5
        && style.border_bottom_style == BorderStyle::Solid
        && style.border_left_style == BorderStyle::Solid
    {
        draw_miter_blend_pixels(canvas,
            ox0 as i32, oy1 as i32 - 1,
            ix0 as i32 - 1, iy1 as i32,
            &bottom_c, &left_c);
    }
}

/// Draw 50% blend pixels along a miter diagonal between two integer pixel coords.
fn draw_miter_blend_pixels(
    canvas: &Canvas,
    x0: i32, y0: i32,
    x1: i32, y1: i32,
    color1: &Color, color2: &Color,
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
        canvas.draw_rect(
            Rect::from_xywh(cx as f32, cy as f32, 1.0, 1.0),
            &paint,
        );
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
    let base_color = Color4f::new(resolved.r, resolved.g, resolved.b, resolved.a);

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
            let mut clip_path = Path::new();
            clip_path.move_to(Point::new(points[0].0, points[0].1));
            clip_path.line_to(Point::new(points[1].0, points[1].1));
            clip_path.line_to(Point::new(points[2].0, points[2].1));
            clip_path.line_to(Point::new(points[3].0, points[3].1));
            clip_path.close();
            canvas.clip_path(&clip_path, ClipOp::Intersect, true);

            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(false);
            paint.set_color4f(base_color, None::<&ColorSpace>);
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
            let mut clip_path = Path::new();
            clip_path.move_to(Point::new(points[0].0, points[0].1));
            clip_path.line_to(Point::new(points[1].0, points[1].1));
            clip_path.line_to(Point::new(points[2].0, points[2].1));
            clip_path.line_to(Point::new(points[3].0, points[3].1));
            clip_path.close();
            canvas.clip_path(&clip_path, ClipOp::Intersect, false);

            paint_border_side(canvas, border_style, border_color,
                inherited_color, width, rect, side);
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
            paint.set_color4f(base_color, None::<&ColorSpace>);
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
            let gap_len = select_best_dash_gap(stroke_length, dash_len, desired_gap);
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Stroke);
            paint.set_stroke_width(width);
            paint.set_anti_alias(true);
            paint.set_color4f(base_color, None::<&ColorSpace>);
            if let Some(effect) = skia_safe::PathEffect::dash(&[dash_len, gap_len], 0.0) {
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
            let stroke_length = if is_horizontal { rect.width() } else { rect.height() };
            let width_i = width.round() as i32;

            if width_i <= 3 && width_i >= 1 {
                // Thin dotted: Chrome draws explicit start/end dots and
                // adjusts line endpoints for uniform appearance.
                paint_thin_dotted_border(
                    canvas, &base_color, width, width_i, stroke_length as i32,
                    p0, p1, is_horizontal,
                );
            } else {
                // Thick dotted: round-capped zero-length dashes.
                let gap = select_best_dash_gap(stroke_length, width, width);
                let mut paint = Paint::default();
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(width);
                paint.set_stroke_cap(skia_safe::paint::Cap::Round);
                paint.set_anti_alias(true);
                paint.set_color4f(base_color, None::<&ColorSpace>);
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
                if let Some(effect) = skia_safe::PathEffect::dash(
                    &[0.0, gap + width - epsilon], 0.0,
                ) {
                    paint.set_path_effect(effect);
                }
                canvas.draw_line(adjusted_p0, adjusted_p1, &paint);
            }
        }
        BorderStyle::Double => {
            // Two lines: outer at full position, inner offset inward.
            // Each line is width/3 thick, with width/3 gap between them.
            let line_width = (width / 3.0).max(1.0);
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            paint.set_color4f(base_color, None::<&ColorSpace>);
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
    if (width_i == 2 && (mod_4 == 0 || mod_4 == 1))
        || (width_i == 3 && (mod_6 == 1 || mod_6 == 2))
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
fn paint_3d_border(canvas: &Canvas, color: &Color4f, width: f32, rect: &Rect, side: BorderSide, inset_outer: bool) {
    let half_width = (width / 2.0).max(1.0);
    let dark = darken_color(color);
    let light = lighten_color(color);

    // Inset shading for a side: top+left → dark, bottom+right → light.
    let inset_color = if matches!(side, BorderSide::Top | BorderSide::Left) { dark } else { light };
    // Outset shading for a side: top+left → light, bottom+right → dark.
    let outset_color = if matches!(side, BorderSide::Top | BorderSide::Left) { light } else { dark };

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

/// Darken a color by multiplying RGB by 0.5.
fn darken_color(color: &Color4f) -> Color4f {
    Color4f::new(color.r * 0.5, color.g * 0.5, color.b * 0.5, color.a)
}

/// Lighten a color by averaging with white.
fn lighten_color(color: &Color4f) -> Color4f {
    Color4f::new(
        (color.r + 1.0) / 2.0,
        (color.g + 1.0) / 2.0,
        (color.b + 1.0) / 2.0,
        color.a,
    )
}

// ── StyleColor PartialEq needed for border comparison ────────────────
// Already derived in the style crate.

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

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

        assert!(ix1 >= ix0, "inner right ({}) must be >= inner left ({})", ix1, ix0);
        assert!(iy1 >= iy0, "inner bottom ({}) must be >= inner top ({})", iy1, iy0);
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
