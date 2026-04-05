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

use skia_safe::{Canvas, Color4f, Paint, PaintStyle, Rect, ColorSpace, ClipOp};
use openui_geometry::PhysicalOffset;
use openui_style::{Color, ComputedStyle, BorderStyle, Overflow, StyleColor, Visibility};
use openui_dom::Document;
use openui_layout::{Fragment, FragmentKind};

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

    // Resolve font metrics from the shape result's first run.
    let metrics = crate::text_painter::metrics_from_shape_result(shape_result);

    // The fragment's offset.top is the top of the line's content area.
    // The baseline is at top + ascent.
    let x = abs_offset.left.to_f32();
    let baseline_y = abs_offset.top.to_f32() + metrics.ascent;

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

/// Paint a single border side as a filled rectangle.
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

    // Only paint solid borders for SP9. Dashed/dotted/double/groove/ridge/
    // inset/outset require specialized path effects or multi-rect drawing
    // that will be implemented in SP12. Skipping them prevents visually
    // wrong output (rendering non-solid styles as solid fills).
    if !matches!(border_style, BorderStyle::Solid) {
        return;
    }

    let resolved = border_color.resolve(inherited_color);
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);
    paint.set_color4f(
        Color4f::new(resolved.r, resolved.g, resolved.b, resolved.a),
        None::<&ColorSpace>,
    );

    canvas.draw_rect(rect, &paint);
}

// ── StyleColor PartialEq needed for border comparison ────────────────
// Already derived in the style crate.
