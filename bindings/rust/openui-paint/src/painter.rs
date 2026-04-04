//! Box painting — extracted from Blink's `box_fragment_painter.cc` and
//! `box_painter_base.cc`.
//!
//! Paint order for a block box (PaintPhase::kBlockBackground):
//! 1. Box shadows (outset) — SP12
//! 2. Background color
//! 3. Box shadows (inset) — SP12
//! 4. Borders
//!
//! Each operation maps to exact Skia calls with exact SkPaint configuration.

use skia_safe::{Canvas, Color4f, Paint, PaintStyle, Rect, ColorSpace};
use openui_geometry::PhysicalOffset;
use openui_style::{Color, ComputedStyle, BorderStyle, StyleColor};
use openui_dom::Document;
use openui_layout::Fragment;

/// Paint a fragment tree onto a Skia canvas.
///
/// This is the main entry point — paints the fragment and all its children
/// recursively, with correct coordinate offsets.
pub fn paint_fragment(canvas: &Canvas, fragment: &Fragment, doc: &Document, offset: PhysicalOffset) {
    let abs_offset = offset + fragment.offset;
    let style = &doc.node(fragment.node_id).style;

    // ── Paint this box ───────────────────────────────────────────────
    paint_box_decoration_background(canvas, fragment, style, abs_offset);

    // ── Paint children in document order ─────────────────────────────
    // Blink paints children at their offset relative to the parent fragment.
    for child in &fragment.children {
        paint_fragment(canvas, child, doc, abs_offset);
    }
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
        paint.set_color4f(Color4f::new(c.r, c.g, c.b, c.a * style.opacity), None::<&ColorSpace>);

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
            Color4f::new(resolved.r, resolved.g, resolved.b, resolved.a * style.opacity),
            None::<&ColorSpace>,
        );

        canvas.draw_rect(stroke_rect, &paint);
    } else {
        // Per-side border painting (filled rectangles for each side)
        // This handles different widths/colors per side.
        paint_border_side(canvas, style.border_top_style, &style.border_top_color,
            inherited_color, style.opacity, bt,
            Rect::from_xywh(x, y, w, bt));
        paint_border_side(canvas, style.border_right_style, &style.border_right_color,
            inherited_color, style.opacity, br,
            Rect::from_xywh(x + w - br, y, br, h));
        paint_border_side(canvas, style.border_bottom_style, &style.border_bottom_color,
            inherited_color, style.opacity, bb,
            Rect::from_xywh(x, y + h - bb, w, bb));
        paint_border_side(canvas, style.border_left_style, &style.border_left_color,
            inherited_color, style.opacity, bl,
            Rect::from_xywh(x, y, bl, h));
    }
}

/// Paint a single border side as a filled rectangle.
fn paint_border_side(
    canvas: &Canvas,
    border_style: BorderStyle,
    border_color: &StyleColor,
    inherited_color: &Color,
    opacity: f32,
    width: f32,
    rect: Rect,
) {
    if width <= 0.0 || !border_style.has_visible_border() {
        return;
    }

    let resolved = border_color.resolve(inherited_color);
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);
    paint.set_color4f(
        Color4f::new(resolved.r, resolved.g, resolved.b, resolved.a * opacity),
        None::<&ColorSpace>,
    );

    match border_style {
        BorderStyle::Solid => {
            canvas.draw_rect(rect, &paint);
        }
        BorderStyle::Dashed | BorderStyle::Dotted => {
            // TODO SP12: implement dash/dot patterns with PathEffect
            canvas.draw_rect(rect, &paint);
        }
        _ => {
            // Other styles (double, groove, ridge, inset, outset) — SP12
            canvas.draw_rect(rect, &paint);
        }
    }
}

// ── StyleColor PartialEq needed for border comparison ────────────────
// Already derived in the style crate.
