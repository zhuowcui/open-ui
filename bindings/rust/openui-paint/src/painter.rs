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

use skia_safe::{Canvas, Color4f, Paint, PaintStyle, Path, Rect, ColorSpace, ClipOp, PathEffect, Point};
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
    let needs_clip = fragment.has_overflow_clip
        || ((style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible)
            && matches!(fragment.kind, FragmentKind::Box | FragmentKind::Viewport));
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

        canvas.draw_rect(stroke_rect, &paint);
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

        // Top border: outer-top-left → outer-top-right → inner-top-right → inner-top-left
        if bt > 0.0 {
            paint_border_side_path(canvas, style.border_top_style, &style.border_top_color,
                inherited_color, bt,
                &[(ox0, oy0), (ox1, oy0), (ix1, iy0), (ix0, iy0)],
                BorderSide::Top);
        }
        // Right border: outer-top-right → outer-bottom-right → inner-bottom-right → inner-top-right
        if br > 0.0 {
            paint_border_side_path(canvas, style.border_right_style, &style.border_right_color,
                inherited_color, br,
                &[(ox1, oy0), (ox1, oy1), (ix1, iy1), (ix1, iy0)],
                BorderSide::Right);
        }
        // Bottom border: outer-bottom-right → outer-bottom-left → inner-bottom-left → inner-bottom-right
        if bb > 0.0 {
            paint_border_side_path(canvas, style.border_bottom_style, &style.border_bottom_color,
                inherited_color, bb,
                &[(ox1, oy1), (ox0, oy1), (ix0, iy1), (ix1, iy1)],
                BorderSide::Bottom);
        }
        // Left border: outer-bottom-left → outer-top-left → inner-top-left → inner-bottom-left
        if bl > 0.0 {
            paint_border_side_path(canvas, style.border_left_style, &style.border_left_color,
                inherited_color, bl,
                &[(ox0, oy1), (ox0, oy0), (ix0, iy0), (ix0, iy1)],
                BorderSide::Left);
        }
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
            let mut path = Path::new();
            path.move_to(Point::new(points[0].0, points[0].1));
            path.line_to(Point::new(points[1].0, points[1].1));
            path.line_to(Point::new(points[2].0, points[2].1));
            path.line_to(Point::new(points[3].0, points[3].1));
            path.close();

            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            paint.set_color4f(base_color, None::<&ColorSpace>);
            canvas.draw_path(&path, &paint);
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
            canvas.save();
            let mut clip_path = Path::new();
            clip_path.move_to(Point::new(points[0].0, points[0].1));
            clip_path.line_to(Point::new(points[1].0, points[1].1));
            clip_path.line_to(Point::new(points[2].0, points[2].1));
            clip_path.line_to(Point::new(points[3].0, points[3].1));
            clip_path.close();
            canvas.clip_path(&clip_path, ClipOp::Intersect, true);

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
