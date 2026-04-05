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
//! 4. Text decorations (line-through — in front of text)
//!
//! Each operation maps to exact Skia calls with exact SkPaint configuration.

use skia_safe::{Canvas, Color4f, Paint, PaintStyle, Rect, ColorSpace, ClipOp, PathEffect, Point};
use skia_safe::paint::Cap;
use openui_geometry::PhysicalOffset;
use openui_style::{Color, ComputedStyle, BorderStyle, Overflow, StyleColor, Visibility};
use openui_dom::Document;
use openui_layout::{Fragment, FragmentKind};
use openui_text::font::FontMetrics;

/// Paint a fragment tree onto a Skia canvas.
///
/// This is the main entry point — paints the fragment and all its children
/// recursively, with correct coordinate offsets.
pub fn paint_fragment(canvas: &Canvas, fragment: &Fragment, doc: &Document, offset: PhysicalOffset) {
    let abs_offset = offset + fragment.offset;

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
        }
    }

    // ── Overflow clipping ────────────────────────────────────────────
    // When a box has overflow: hidden (or clip), clip children to the
    // content box so overflowing content is not painted.
    let needs_clip = (style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible)
        && matches!(fragment.kind, FragmentKind::Box | FragmentKind::Viewport);
    if needs_clip {
        canvas.save();
        // Clip to padding box (inset by border widths per CSS spec).
        let clip_x = abs_offset.left.to_f32() + fragment.border.left.to_f32();
        let clip_y = abs_offset.top.to_f32() + fragment.border.top.to_f32();
        let clip_w = fragment.size.width.to_f32() - fragment.border.left.to_f32() - fragment.border.right.to_f32();
        let clip_h = fragment.size.height.to_f32() - fragment.border.top.to_f32() - fragment.border.bottom.to_f32();
        canvas.clip_rect(
            Rect::from_xywh(clip_x, clip_y, clip_w, clip_h),
            ClipOp::Intersect,
            false,
        );
    }

    // ── Paint children in document order ─────────────────────────────
    // Blink paints children at their offset relative to the parent fragment.
    // Note: visibility is inherited, but children can override it,
    // so we always recurse (the child's own visibility check will decide).
    for child in &fragment.children {
        paint_fragment(canvas, child, doc, abs_offset);
    }

    if needs_clip {
        canvas.restore();
    }

    if needs_layer {
        canvas.restore();
    }
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

/// Paint a text fragment — shadows, decorations, and glyphs.
///
/// Extracted from Blink's `TextFragmentPainter::Paint()`.
///
/// Paint order:
/// 1. Text shadows (behind everything)
/// 2. Underline + overline decorations (behind text glyphs)
/// 3. Text glyphs
/// 4. Line-through decoration (in front of text glyphs)
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

    // 1. Text shadows
    crate::text_painter::paint_text_shadows(canvas, shape_result, origin, style);

    // 2. Text decorations (underline + overline, painted behind text)
    crate::decoration_painter::paint_text_decorations(
        canvas, shape_result, origin, style, &metrics,
        crate::decoration_painter::DecorationPhase::BeforeText,
    );

    // 3. Text glyphs
    crate::text_painter::paint_text(canvas, shape_result, origin, style);

    // 4. Line-through decoration (painted in front of text per CSS spec)
    crate::decoration_painter::paint_text_decorations(
        canvas, shape_result, origin, style, &metrics,
        crate::decoration_painter::DecorationPhase::AfterText,
    );
}

/// Paint background + border for a single box fragment.
///
/// Extracted from Blink's `BoxFragmentPainter::PaintBoxDecorationBackgroundWithRectImpl()`
/// (box_fragment_painter.cc:1550).
///
/// Order:
/// 1. Background color (fill the border-box rect)
/// 2. Border (stroke the border-box rect)
fn paint_box_decoration_background(
    canvas: &Canvas,
    fragment: &Fragment,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
) {
    let x = abs_offset.left.to_f32();
    let y = abs_offset.top.to_f32();
    let w = fragment.size.width.to_f32();
    let h = fragment.size.height.to_f32();

    // Skip empty fragments
    if w <= 0.0 || h <= 0.0 {
        return;
    }

    let border_box_rect = Rect::from_xywh(x, y, w, h);

    // ── 1. Background color ──────────────────────────────────────────
    // Blink: box_painter_base.cc:1279 — PaintFillLayerBackground
    // → context.FillRect(background_rect, info.color, ...)
    // → canvas->drawRect(rect, paint) with kFill_Style
    if !style.background_color.is_transparent() {
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(true);
        let c = &style.background_color;
        paint.set_color4f(Color4f::new(c.r, c.g, c.b, c.a), None::<&ColorSpace>);

        canvas.draw_rect(border_box_rect, &paint);
    }

    // ── 2. Borders ───────────────────────────────────────────────────
    // Blink renders borders differently based on complexity:
    // - Uniform solid border with same color: single stroke rect
    // - Different colors/widths per side: four separate trapezoids
    //
    // For SP9 we implement both the simple uniform case and the
    // per-side case.
    paint_borders(canvas, fragment, style, x, y, w, h);
}

/// Paint borders around the border-box.
///
/// Extracted from Blink's `BoxBorderPainter` (box_border_painter.cc).
fn paint_borders(
    canvas: &Canvas,
    _fragment: &Fragment,
    style: &ComputedStyle,
    x: f32, y: f32, w: f32, h: f32,
) {
    let bt = style.effective_border_top() as f32;
    let br = style.effective_border_right() as f32;
    let bb = style.effective_border_bottom() as f32;
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
        // Stroke rect inset by half the border width
        let half = bt / 2.0;
        let stroke_rect = Rect::from_xywh(x + half, y + half, w - bt, h - bt);

        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Stroke);
        paint.set_stroke_width(bt);
        paint.set_anti_alias(true);

        let resolved = style.border_top_color.resolve(inherited_color);
        paint.set_color4f(
            Color4f::new(resolved.r, resolved.g, resolved.b, resolved.a),
            None::<&ColorSpace>,
        );

        canvas.draw_rect(stroke_rect, &paint);
    } else {
        // Per-side border painting (filled rectangles for each side).
        // NOTE: Corner regions overlap when sides have different colors.
        // Blink uses trapezoid/polygon drawing for correct diagonal splits.
        // This will be fixed in SP12 (advanced paint). For SP9, corners
        // show the later-drawn side's color — acceptable for uniform colors.
        paint_border_side(canvas, style.border_top_style, &style.border_top_color,
            inherited_color, bt,
            Rect::from_xywh(x, y, w, bt));
        paint_border_side(canvas, style.border_right_style, &style.border_right_color,
            inherited_color, br,
            Rect::from_xywh(x + w - br, y, br, h));
        paint_border_side(canvas, style.border_bottom_style, &style.border_bottom_color,
            inherited_color, bb,
            Rect::from_xywh(x, y + h - bb, w, bb));
        paint_border_side(canvas, style.border_left_style, &style.border_left_color,
            inherited_color, bl,
            Rect::from_xywh(x, y, bl, h));
    }
}

/// Paint a single border side.
///
/// Supports all CSS border styles: solid, dashed, dotted, double,
/// groove, ridge, inset, outset. None/hidden are skipped.
fn paint_border_side(
    canvas: &Canvas,
    border_style: BorderStyle,
    border_color: &StyleColor,
    inherited_color: &Color,
    width: f32,
    rect: Rect,
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
            // Dash length = 3 * border-width, gap = border-width.
            let dash_len = width * 3.0;
            let gap_len = width;
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Stroke);
            paint.set_stroke_width(width);
            paint.set_anti_alias(true);
            paint.set_color4f(base_color, None::<&ColorSpace>);
            if let Some(effect) = PathEffect::dash(&[dash_len, gap_len], 0.0) {
                paint.set_path_effect(effect);
            }
            // Draw along the center of the border side.
            let (p0, p1) = border_side_center_line(&rect, width);
            canvas.draw_line(p0, p1, &paint);
        }
        BorderStyle::Dotted => {
            // Dot = border-width, gap = border-width, with round caps.
            let dot_len = 0.01; // near-zero dash to produce dots with round caps
            let gap_len = width * 2.0;
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Stroke);
            paint.set_stroke_width(width);
            paint.set_stroke_cap(Cap::Round);
            paint.set_anti_alias(true);
            paint.set_color4f(base_color, None::<&ColorSpace>);
            if let Some(effect) = PathEffect::dash(&[dot_len, gap_len], 0.0) {
                paint.set_path_effect(effect);
            }
            let (p0, p1) = border_side_center_line(&rect, width);
            canvas.draw_line(p0, p1, &paint);
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
            // Top/left half darkened, bottom/right half lightened.
            paint_3d_border(canvas, &base_color, width, &rect, true);
        }
        BorderStyle::Ridge => {
            // Opposite of groove.
            paint_3d_border(canvas, &base_color, width, &rect, false);
        }
        BorderStyle::Inset => {
            // Darken the color for inset effect.
            let dark = darken_color(&base_color);
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            paint.set_color4f(dark, None::<&ColorSpace>);
            canvas.draw_rect(rect, &paint);
        }
        BorderStyle::Outset => {
            // Lighten the color for outset effect.
            let light = lighten_color(&base_color);
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            paint.set_color4f(light, None::<&ColorSpace>);
            canvas.draw_rect(rect, &paint);
        }
        BorderStyle::None | BorderStyle::Hidden => {
            // Already handled above, but satisfy exhaustive match.
        }
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

/// Paint a 3D-style border (groove or ridge).
///
/// `darken_first`: true for groove (outer half dark, inner half light),
/// false for ridge (outer half light, inner half dark).
fn paint_3d_border(canvas: &Canvas, color: &Color4f, width: f32, rect: &Rect, darken_first: bool) {
    let half_width = (width / 2.0).max(1.0);
    let dark = darken_color(color);
    let light = lighten_color(color);

    let (first_color, second_color) = if darken_first {
        (dark, light)
    } else {
        (light, dark)
    };

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);

    // Outer half.
    let outer = shrink_border_rect(rect, width, 0.0, half_width);
    paint.set_color4f(first_color, None::<&ColorSpace>);
    canvas.draw_rect(outer, &paint);

    // Inner half.
    let inner = shrink_border_rect(rect, width, half_width, width - half_width);
    paint.set_color4f(second_color, None::<&ColorSpace>);
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
