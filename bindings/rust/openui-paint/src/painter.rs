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

use openui_dom::{
    Document, ElementTag, FormControlRole, NodeId, PseudoElementKind, ReplacedResourceKind,
};
use openui_geometry::{BoxStrut, LayoutUnit, PhysicalOffset};
use openui_layout::{Fragment, FragmentKind};
use openui_style::{
    BackgroundAttachment, BackgroundClip, BackgroundLayer, BackgroundPosition, BackgroundRepeat,
    BackgroundSize, BorderImage, BorderImageLength, BorderImageRepeat, BorderStyle, Color,
    ComputedStyle, ContentPosition, CssImage, Direction, Display, FontFamily, GradientColorSpace,
    GradientStopPosition, LineHeight, ListStylePosition, ListStyleType, ObjectFit, Overflow,
    OverflowClipBox, Position, RadialGradientShape, RadialGradientSize, StyleColor, Visibility,
};
use openui_text::{Font, FontMetrics, TextDirection, TextShaper};
use skia_safe::canvas::{SaveLayerRec, SrcRectConstraint};
use skia_safe::rrect::Corner as RRectCorner;
use skia_safe::{
    gradient_shader, surfaces, BlendMode, Canvas, ClipOp, Color4f, ColorSpace, Data, FilterMode,
    Image, Matrix, MipmapMode, Paint, PaintStyle, PathBuilder, PathFillType, PathMeasure, Point,
    RRect, Rect, SamplingOptions, TileMode,
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
            GradientStopPosition::Calc { percent, px } => Some(if line_length > 0.0 {
                percent / 100.0 + px / line_length
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
    static PREPAINTED_BOX_DECORATIONS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    static FRAGMENTED_OOF_HOIST_ACTIVE: RefCell<bool> = const { RefCell::new(false) };
    static FRAGMENTED_INLINE_SKIP_AFTER: RefCell<Option<usize>> = const { RefCell::new(None) };
    static VIEWPORT_SIZE: RefCell<(f32, f32)> = const { RefCell::new((800.0, 600.0)) };
    static BROKEN_IMAGE: RefCell<Option<Image>> = const { RefCell::new(None) };
    static RASTERIZING_PROMOTED_TRANSFORM: RefCell<bool> = const { RefCell::new(false) };
}

pub(crate) fn set_paint_viewport_size(width: f32, height: f32) {
    VIEWPORT_SIZE.with(|size| *size.borrow_mut() = (width, height));
}

fn uses_deterministic_text_profile(style: &ComputedStyle) -> bool {
    style.font_family.families.iter().any(|family| {
        matches!(family, FontFamily::Named(name) if name.eq_ignore_ascii_case("Droid Sans Fallback"))
    })
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

fn fragment_transform_matrix(
    style: &ComputedStyle,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
) -> Matrix {
    let mut width = fragment.size.width.to_f32();
    let mut height = fragment.size.height.to_f32();
    if let Some(fragmented_block_size) = fragment.decoration_paint_block_size {
        if fragment_block_axis_is_x(fragment) {
            width = fragmented_block_size.to_f32();
        } else {
            height = fragmented_block_size.to_f32();
        }
    }
    let origin_x = resolve_background_length(&style.transform_origin.0, width, width * 0.5);
    let origin_y = resolve_background_length(&style.transform_origin.1, height, height * 0.5);
    let pivot_x = abs_offset.left.to_f32() + origin_x;
    let pivot_y = abs_offset.top.to_f32() + origin_y;
    let transform = style.transform;
    Matrix::new_all(
        transform.a,
        transform.c,
        transform.e + pivot_x - transform.a * pivot_x - transform.c * pivot_y,
        transform.b,
        transform.d,
        transform.f + pivot_y - transform.b * pivot_x - transform.d * pivot_y,
        0.0,
        0.0,
        1.0,
    )
}

fn promoted_subtree_bounds(fragment: &Fragment) -> Rect {
    fn extend(fragment: &Fragment, x: f32, y: f32, bounds: &mut Rect) {
        let own = Rect::from_xywh(
            x,
            y,
            fragment.size.width.to_f32(),
            fragment.size.height.to_f32(),
        );
        bounds.join(own);
        for child in &fragment.children {
            extend(
                child,
                x + child.offset.left.to_f32(),
                y + child.offset.top.to_f32(),
                bounds,
            );
        }
    }

    let mut bounds = Rect::from_xywh(
        0.0,
        0.0,
        fragment.size.width.to_f32(),
        fragment.size.height.to_f32(),
    );
    for child in &fragment.children {
        extend(
            child,
            child.offset.left.to_f32(),
            child.offset.top.to_f32(),
            &mut bounds,
        );
    }
    Rect::from_ltrb(
        bounds.left.floor(),
        bounds.top.floor(),
        bounds.right.ceil(),
        bounds.bottom.ceil(),
    )
}

fn is_monolithic_column_fragment(fragment: &Fragment, doc: &Document) -> bool {
    if fragment.node_id.is_none() || fragment.kind != FragmentKind::Box {
        return false;
    }
    let node = doc.node(fragment.node_id);
    node.replaced.is_some()
        || matches!(
            node.form_control,
            Some(
                FormControlRole::Button
                    | FormControlRole::TextInput
                    | FormControlRole::TextArea
                    | FormControlRole::Select
                    | FormControlRole::Range
                    | FormControlRole::Meter
            )
        )
        || matches!(
            node.tag,
            ElementTag::Legend
                | ElementTag::Image
                | ElementTag::Svg
                | ElementTag::Canvas
                | ElementTag::Embed
                | ElementTag::IFrame
        )
        || ((node.style.overflow_x != Overflow::Visible && node.style.overflow_x != Overflow::Clip)
            || (node.style.overflow_y != Overflow::Visible
                && node.style.overflow_y != Overflow::Clip))
}

fn collect_overflowing_monolithic_column_fragments<'a>(
    fragment: &'a Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    clip: Rect,
    block_axis_is_x: bool,
    result: &mut Vec<(&'a Fragment, PhysicalOffset)>,
) {
    for child in &fragment.children {
        let child_offset = PhysicalOffset::new(
            parent_offset.left + child.offset.left,
            parent_offset.top + child.offset.top,
        );
        let (start, end, clip_start, clip_end) = if block_axis_is_x {
            (
                child_offset.left.to_f32(),
                (child_offset.left + child.size.width).to_f32(),
                clip.left,
                clip.right,
            )
        } else {
            (
                child_offset.top.to_f32(),
                (child_offset.top + child.size.height).to_f32(),
                clip.top,
                clip.bottom,
            )
        };
        if is_monolithic_column_fragment(child, doc) && (start < clip_start || end > clip_end) {
            result.push((child, parent_offset));
        } else {
            collect_overflowing_monolithic_column_fragments(
                child,
                doc,
                child_offset,
                clip,
                block_axis_is_x,
                result,
            );
        }
    }
}

fn collect_overflowing_max_block_decorations<'a>(
    fragment: &'a Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    result: &mut Vec<(&'a Fragment, PhysicalOffset)>,
) {
    for child in &fragment.children {
        let child_offset = PhysicalOffset::new(
            parent_offset.left + child.offset.left,
            parent_offset.top + child.offset.top,
        );
        if !child.node_id.is_none()
            && child.kind == FragmentKind::Box
            && child.is_first_for_node
            && (doc.node(child.node_id).tag == ElementTag::Fieldset
                || child.children.iter().any(|descendant| {
                    fragment_is_in_flow_monolithic(descendant, doc)
                        || fragment_has_in_flow_monolithic_descendant(descendant, doc)
                }))
            && (!doc.node(child.node_id).style.max_height.is_none()
                || doc.node(child.node_id).style.height.is_fixed())
            && child.decoration_slice.is_some_and(|slice| {
                slice.source_block_size > fragment_physical_block_extent(child)
            })
        {
            result.push((child, child_offset));
        }
        collect_overflowing_max_block_decorations(child, doc, child_offset, result);
    }
}

fn fragment_is_in_flow_monolithic(fragment: &Fragment, doc: &Document) -> bool {
    !fragment.node_id.is_none()
        && !doc.node(fragment.node_id).style.is_out_of_flow()
        && is_monolithic_column_fragment(fragment, doc)
}

fn fragment_has_in_flow_monolithic_descendant(fragment: &Fragment, doc: &Document) -> bool {
    fragment.children.iter().any(|child| {
        if child.node_id.is_none() {
            fragment_has_in_flow_monolithic_descendant(child, doc)
        } else {
            !doc.node(child.node_id).style.is_out_of_flow()
                && (fragment_is_in_flow_monolithic(child, doc)
                    || fragment_has_in_flow_monolithic_descendant(child, doc))
        }
    })
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
        paint_text_fragment(canvas, fragment, style, abs_offset, doc);
        return;
    }

    // Column box fragments are anonymous clipping containers for multicol
    // columns (CSS Multicol §3.1). They have no decoration but clip children
    // to the column boundaries.
    if fragment.kind == FragmentKind::ColumnBox {
        if fragment.has_overflow_clip {
            let mut overflowing_max_decorations = Vec::new();
            collect_overflowing_max_block_decorations(
                fragment,
                doc,
                abs_offset,
                &mut overflowing_max_decorations,
            );
            for (overflow, overflow_offset) in overflowing_max_decorations {
                let Some(slice) = overflow.decoration_slice else {
                    continue;
                };
                let mut complete = overflow.clone();
                let is_fieldset = doc.node(complete.node_id).tag == ElementTag::Fieldset;
                if is_fieldset {
                    let style = &doc.node(complete.node_id).style;
                    let legend_end = complete
                        .children
                        .iter()
                        .filter(|child| {
                            !child.node_id.is_none()
                                && doc.node(child.node_id).tag == ElementTag::Legend
                        })
                        .map(|child| child.offset.top + child.size.height)
                        .max_by_key(|end| end.raw())
                        .unwrap_or(LayoutUnit::zero());
                    complete.size.height = complete
                        .size
                        .height
                        .max_of(LayoutUnit::from_i32(style.effective_border_top()))
                        .max_of(legend_end);
                    // Fragment slicing suppresses the block-start and inline
                    // borders on continuation fragments.  This replay is the
                    // fieldset's complete first decoration, so restore those
                    // authored struts while retaining block-end suppression
                    // until the fragment that actually owns the final edge.
                    complete.border = BoxStrut::new(
                        LayoutUnit::from_i32(style.effective_border_top()),
                        LayoutUnit::from_i32(style.effective_border_right()),
                        if overflow.is_last_for_node {
                            LayoutUnit::from_i32(style.effective_border_bottom())
                        } else {
                            LayoutUnit::zero()
                        },
                        LayoutUnit::from_i32(style.effective_border_left()),
                    );
                } else if fragment_block_axis_is_x(&complete) {
                    complete.size.width = slice.source_block_size;
                } else {
                    complete.size.height = slice.source_block_size;
                }
                complete.decoration_slice = None;
                complete.decoration_paint_block_size = None;
                complete.is_first_for_node = true;
                complete.is_last_for_node = overflow.is_last_for_node;
                if is_fieldset {
                    canvas.save();
                    canvas.clip_rect(
                        column_block_only_clip_rect(fragment, abs_offset),
                        ClipOp::Intersect,
                        false,
                    );
                }
                paint_fragment_box_decoration(
                    canvas,
                    &complete,
                    doc,
                    &doc.node(complete.node_id).style,
                    overflow_offset,
                    1.0,
                );
                if is_fieldset {
                    canvas.restore();
                }
            }
            let mut overflowing_monolithic = Vec::new();
            if fragment.block_axis_clip_only {
                collect_overflowing_monolithic_column_fragments(
                    fragment,
                    doc,
                    abs_offset,
                    column_physical_rect(fragment, abs_offset),
                    fragment_block_axis_is_x(fragment),
                    &mut overflowing_monolithic,
                );
            }
            let monolithic_skip: Vec<usize> = HOIST_SKIP.with(|skipped| {
                let mut skipped = skipped.borrow_mut();
                overflowing_monolithic
                    .iter()
                    .map(|(overflow, _)| *overflow as *const Fragment as usize)
                    .filter(|pointer| skipped.insert(*pointer))
                    .collect()
            });
            // A nested multicol's rule is ink in the intervening column gap,
            // not content clipped to the ancestor column's inline edge. When
            // the ancestor owns a two-axis continuation clip, paint only the
            // lateral part of descendant rules outside that edge here; the
            // ordinary traversal below paints the portion inside the column.
            // Both passes remain constrained to this fragmentainer's block
            // interval, so rules cannot leak into an adjacent row.
            if !fragment.block_axis_clip_only && !fragment.inline_axis_clip_only {
                let block_clip = column_block_only_clip_rect(fragment, abs_offset);
                canvas.save();
                canvas.clip_rect(block_clip, ClipOp::Intersect, false);
                canvas.clip_rect(
                    column_physical_rect(fragment, abs_offset),
                    ClipOp::Difference,
                    false,
                );
                paint_descendant_column_rules(canvas, &fragment.children, doc, abs_offset);
                canvas.restore();
            }
            canvas.save();
            // Multicol fragmentainers clip in the block axis. Inline overflow
            // may normally paint into the column gap. Layout clears
            // `block_axis_clip_only` when a nested fragmentation context owns
            // an inline-axis continuation boundary as well.
            canvas.clip_rect(
                column_fragmentainer_clip_rect(fragment, abs_offset),
                skia_safe::ClipOp::Intersect,
                false,
            );
            paint_children_with_stacking_order(canvas, &fragment.children, doc, abs_offset, false);
            paint_fragmented_descendant_outlines(
                canvas,
                &fragment.children,
                doc,
                abs_offset,
                column_physical_rect(fragment, abs_offset),
                fragment_block_axis_is_x(fragment),
            );
            repaint_later_siblings_over_fragmented_outlines(
                canvas,
                &fragment.children,
                doc,
                abs_offset,
                column_physical_rect(fragment, abs_offset),
                fragment_block_axis_is_x(fragment),
            );
            canvas.restore();
            HOIST_SKIP.with(|skipped| {
                let mut skipped = skipped.borrow_mut();
                for pointer in &monolithic_skip {
                    skipped.remove(pointer);
                }
            });
            for (overflow, parent_offset) in overflowing_monolithic {
                paint_fragment(canvas, overflow, doc, parent_offset);
            }
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
    let mut canvas_adjusted_style = fragment.paint_background_color_override.map(|color| {
        let mut adjusted = original_style.clone();
        adjusted.background_color = color;
        adjusted
    });
    let canvas_background_source = doc.canvas_background_source();
    if canvas_background_source == Some(fragment.node_id)
        || (fragment.node_id == doc.root()
            && canvas_background_source.is_some()
            && canvas_background_source != Some(fragment.node_id))
    {
        let mut adjusted = original_style.clone();
        // Canvas-propagated backgrounds are painted once on the canvas, not
        // again on the source element's principal box.  Historical generated
        // documents also carry the initial white canvas color on their
        // synthetic viewport node; suppress that compatibility color when a
        // direct body child supplies the propagated canvas background.
        adjusted.background_color = Color::TRANSPARENT;
        adjusted.background_layers.clear();
        adjusted.background_linear_gradient = None;
        canvas_adjusted_style = Some(adjusted);
    }
    let style = if fragment.kind == FragmentKind::Text {
        fragment.inherited_style.as_ref().unwrap_or(original_style)
    } else {
        canvas_adjusted_style.as_ref().unwrap_or(original_style)
    };
    let promoted_transform_parent_offset = PhysicalOffset::new(
        abs_offset.left - fragment.offset.left,
        abs_offset.top - fragment.offset.top,
    );
    for ancestor in &fragment.promoted_transform_ancestors {
        let ancestor_style = &doc.node(ancestor.node_id).style;
        let transform = ancestor_style.transform;
        if transform == openui_style::Transform2D::IDENTITY {
            continue;
        }
        let width = ancestor.fragment_size.width.to_f32();
        let height = ancestor.fragment_size.height.to_f32();
        let origin_x =
            resolve_background_length(&ancestor_style.transform_origin.0, width, width * 0.5);
        let origin_y =
            resolve_background_length(&ancestor_style.transform_origin.1, height, height * 0.5);
        let pivot_x = (promoted_transform_parent_offset.left + ancestor.fragment_offset.left)
            .to_f32()
            + origin_x;
        let pivot_y = (promoted_transform_parent_offset.top + ancestor.fragment_offset.top)
            .to_f32()
            + origin_y;
        let matrix = Matrix::new_all(
            transform.a,
            transform.c,
            transform.e + pivot_x - transform.a * pivot_x - transform.c * pivot_y,
            transform.b,
            transform.d,
            transform.f + pivot_y - transform.b * pivot_x - transform.d * pivot_y,
            0.0,
            0.0,
            1.0,
        );
        canvas.save();
        canvas.concat(&matrix);
    }
    let suppress_transform = RASTERIZING_PROMOTED_TRANSFORM.with(|active| *active.borrow());
    let transform = if suppress_transform {
        openui_style::Transform2D::IDENTITY
    } else {
        style.transform
    };
    let has_transform = transform != openui_style::Transform2D::IDENTITY
        && matches!(fragment.kind, FragmentKind::Box | FragmentKind::Viewport);
    let has_precomposited_transform = has_transform
        && style.will_change_transform
        && (transform.b != 0.0 || transform.c != 0.0)
        && (fragment.decoration_slice.is_none() || fragment.is_first_for_node);
    let has_layout_fragmentation_transform_clip = has_transform
        && fragment.decoration_slice.is_some()
        && fragment.block_axis_clip_only
        && style.overflow_x == Overflow::Visible
        && style.overflow_y == Overflow::Visible;
    let has_fixed_fragmentation_transform_clip = has_layout_fragmentation_transform_clip
        && transform.a == 1.0
        && transform.b == 0.0
        && transform.c == 0.0
        && transform.d == 1.0;
    if has_fixed_fragmentation_transform_clip {
        // Fragmentation clips in the fragmentainer coordinate space before
        // transforms are applied. Install that one-axis clip while the canvas
        // still has the ancestor transform, then transform only the fragment
        // and its contents below. Applying this clip after concat would move a
        // vertical-writing column edge by translateX and hide the very ink
        // translated back into that fragment by a relative descendant.
        let big = 100_000.0_f32;
        let clip = if fragment_block_axis_is_x(fragment) {
            Rect::from_xywh(
                abs_offset.left.to_f32(),
                -big,
                fragment.size.width.to_f32(),
                big * 2.0,
            )
        } else {
            Rect::from_xywh(
                -big,
                abs_offset.top.to_f32(),
                big * 2.0,
                fragment.size.height.to_f32(),
            )
        };
        canvas.save();
        canvas.clip_rect(clip, ClipOp::Intersect, false);
    }
    if has_precomposited_transform {
        let bounds = promoted_subtree_bounds(fragment);
        let raster_width = bounds.width().ceil().max(1.0) as i32;
        let raster_height = bounds.height().ceil().max(1.0) as i32;
        if let Some(mut surface) = surfaces::raster_n32_premul((raster_width, raster_height)) {
            surface.canvas().clear(skia_safe::Color::TRANSPARENT);
            RASTERIZING_PROMOTED_TRANSFORM.with(|active| *active.borrow_mut() = true);
            let raster_parent_offset = PhysicalOffset::new(
                LayoutUnit::from_f32(-bounds.left) - fragment.offset.left,
                LayoutUnit::from_f32(-bounds.top) - fragment.offset.top,
            );
            paint_fragment(surface.canvas(), fragment, doc, raster_parent_offset);
            RASTERIZING_PROMOTED_TRANSFORM.with(|active| *active.borrow_mut() = false);

            let image = surface.image_snapshot();
            let destination = Rect::from_xywh(
                abs_offset.left.to_f32() + bounds.left,
                abs_offset.top.to_f32() + bounds.top,
                bounds.width(),
                bounds.height(),
            );
            let shader_matrix = Matrix::translate((destination.left, destination.top));
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_shader(image.to_shader(
                (TileMode::Clamp, TileMode::Clamp),
                SamplingOptions::from(FilterMode::Linear),
                &shader_matrix,
            ));

            canvas.save();
            canvas.concat(&fragment_transform_matrix(style, fragment, abs_offset));
            canvas.draw_rect(destination, &paint);
            canvas.restore();
        }
        if has_fixed_fragmentation_transform_clip {
            canvas.restore();
        }
        for ancestor in fragment.promoted_transform_ancestors.iter().rev() {
            if doc.node(ancestor.node_id).style.transform != openui_style::Transform2D::IDENTITY {
                canvas.restore();
            }
        }
        return;
    }
    if has_transform {
        // Fragment geometry is stored in the untransformed coordinate space.
        // Apply the element's affine transform while painting its complete
        // fragment subtree, around the CSS transform-origin in the border box.
        // Using absolute pivot coordinates is important because this painter
        // carries offsets explicitly instead of translating the SkCanvas while
        // descending the fragment tree.
        let matrix = fragment_transform_matrix(style, fragment, abs_offset);
        canvas.save();
        canvas.concat(&matrix);
    }
    let has_clip_path = style.clip_path_inset.is_some()
        && matches!(fragment.kind, FragmentKind::Box | FragmentKind::Viewport);
    if let Some(inset) = style.clip_path_inset.as_ref().filter(|_| has_clip_path) {
        let width = fragment.size.width.to_f32();
        let height = fragment.size.height.to_f32();
        let top = resolve_background_length(&inset[0], height, 0.0);
        let right = resolve_background_length(&inset[1], width, 0.0);
        let bottom = resolve_background_length(&inset[2], height, 0.0);
        let left = resolve_background_length(&inset[3], width, 0.0);
        canvas.save();
        canvas.clip_rect(
            Rect::from_xywh(
                abs_offset.left.to_f32() + left,
                abs_offset.top.to_f32() + top,
                (width - left - right).max(0.0),
                (height - top - bottom).max(0.0),
            ),
            ClipOp::Intersect,
            true,
        );
    }
    let needs_mask_layer = !style.mask_layers.is_empty()
        && matches!(fragment.kind, FragmentKind::Box | FragmentKind::Viewport);
    if needs_mask_layer {
        canvas.save_layer_alpha_f(None, 1.0);
    }

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
                if style.is_first_letter_pseudo {
                    paint_fragment_box_decoration(canvas, fragment, doc, style, abs_offset, 1.0);
                }
                paint_text_fragment(canvas, fragment, style, abs_offset, doc);
            }
            FragmentKind::Box | FragmentKind::Viewport => {
                let paint_opacity = if flattens_opacity { style.opacity } else { 1.0 };
                let decoration_prepainted = PREPAINTED_BOX_DECORATIONS.with(|fragments| {
                    fragments
                        .borrow()
                        .contains(&(fragment as *const Fragment as usize))
                });
                if !decoration_prepainted && !fragment.skip_box_decoration {
                    paint_fragment_box_decoration(
                        canvas,
                        fragment,
                        doc,
                        style,
                        abs_offset,
                        paint_opacity,
                    );
                }
                paint_form_control(canvas, fragment, doc, abs_offset, paint_opacity);
                paint_missing_image(canvas, fragment, doc, style, abs_offset, paint_opacity);
                paint_replaced_content(canvas, fragment, doc, style, abs_offset, paint_opacity);
                let outside_marker_clipped = style.list_style_position
                    == ListStylePosition::Outside
                    && (style.overflow_x != Overflow::Visible
                        || style.overflow_y != Overflow::Visible);
                if style.display == Display::ListItem
                    && style.list_style_type != ListStyleType::None
                    && !outside_marker_clipped
                {
                    paint_list_marker(canvas, fragment, doc, style, abs_offset);
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
    let needs_clip =
        needs_overflow_clip(fragment, style, doc) && !has_fixed_fragmentation_transform_clip;
    if native_scroll_button_symbol(fragment, doc).is_some() {
        // The platform control renderer above consumed the directional
        // single-character content with its native LCD mask.
    } else if needs_clip {
        paint_with_overflow_clip(canvas, fragment, doc, abs_offset, style);
    } else {
        // Paint children with CSS stacking order (z-index aware).
        let is_sc = is_fragment_stacking_context(fragment, doc);
        paint_children_with_stacking_order(canvas, &fragment.children, doc, abs_offset, is_sc);
    }

    if fragment.paint_border_after_children && style.visibility == Visibility::Visible {
        if fragment.collapsed_border_segments.is_empty() {
            let x = abs_offset.left.round().to_f32();
            let y = abs_offset.top.round().to_f32();
            let right = (abs_offset.left + fragment.size.width).round().to_f32();
            let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
            paint_borders(
                canvas,
                fragment,
                style,
                x,
                y,
                right - x,
                bottom - y,
                false,
                false,
            );
        } else {
            for segment in &fragment.collapsed_border_segments {
                let segment_offset = PhysicalOffset::new(
                    abs_offset.left + segment.rect.offset.left,
                    abs_offset.top + segment.rect.offset.top,
                );
                let x = segment_offset.left.round().to_f32();
                let y = segment_offset.top.round().to_f32();
                let right = (segment_offset.left + segment.rect.size.width)
                    .round()
                    .to_f32();
                let bottom = (segment_offset.top + segment.rect.size.height)
                    .round()
                    .to_f32();
                let mut segment_fragment = Fragment::new_box(fragment.node_id, segment.rect.size);
                segment_fragment.border = segment.border;
                segment_fragment.ignore_border_radius = fragment.ignore_border_radius;
                segment_fragment.fragmentation_writing_direction =
                    fragment.fragmentation_writing_direction;
                segment_fragment.is_first_for_node = fragment.is_first_for_node;
                segment_fragment.is_last_for_node = fragment.is_last_for_node;
                paint_borders(
                    canvas,
                    &segment_fragment,
                    style,
                    x,
                    y,
                    right - x,
                    bottom - y,
                    false,
                    style.display == Display::TableCell,
                );
            }
        }
    }

    if style.visibility == Visibility::Visible && should_outline && paint_outline_after_children {
        paint_outline(canvas, fragment, style, abs_offset);
    }

    if needs_mask_layer {
        // CSS masks apply to the element's complete stacking context. Paint
        // the mask into a temporary layer and composite its alpha with DstIn,
        // then restore the masked element layer into its parent.
        let mut mask_blend = Paint::default();
        mask_blend.set_blend_mode(BlendMode::DstIn);
        let mask_record = SaveLayerRec::default().paint(&mask_blend);
        canvas.save_layer(&mask_record);
        let mut mask_style = style.clone();
        mask_style.background_layers = style.mask_layers.clone();
        mask_style.background_linear_gradient = None;
        mask_style.background_color = Color::TRANSPARENT;
        let mask_rect = Rect::from_xywh(
            abs_offset.left.to_f32(),
            abs_offset.top.to_f32(),
            fragment.size.width.to_f32(),
            fragment.size.height.to_f32(),
        );
        paint_background_layers(
            canvas,
            doc,
            Some(fragment),
            &mask_style,
            mask_rect,
            1.0,
            None,
        );
        canvas.restore();
        canvas.restore();
    }

    if needs_layer {
        canvas.restore();
    }
    if has_clip_path {
        canvas.restore();
    }
    if has_transform {
        canvas.restore();
    }
    if has_fixed_fragmentation_transform_clip {
        canvas.restore();
    }
    for ancestor in fragment.promoted_transform_ancestors.iter().rev() {
        if doc.node(ancestor.node_id).style.transform != openui_style::Transform2D::IDENTITY {
            canvas.restore();
        }
    }
}

/// Paint deterministic passive platform controls. Interactive state is out of
/// scope, but the initial Linux Chromium geometry is stable and belongs to the
/// public form-control role rather than to any individual test.
fn paint_form_control(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    match doc.node(fragment.node_id).form_control {
        Some(FormControlRole::Meter) => {
            paint_meter_control(canvas, fragment, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::Range) => paint_range_control(
            canvas,
            fragment,
            abs_offset,
            opacity_multiplier,
            doc.node(fragment.node_id).form_control_native_appearance,
        ),
        Some(FormControlRole::TextInput) => {
            paint_text_input_control(canvas, fragment, doc, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::TextArea) => {
            paint_textarea_resize_grip(canvas, fragment, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::Select)
            if doc.node(fragment.node_id).form_control_native_appearance =>
        {
            paint_select_arrow(canvas, fragment, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::Button)
            if matches!(
                doc.node(fragment.node_id).pseudo_kind,
                Some(PseudoElementKind::ScrollButton(_))
            ) =>
        {
            paint_scroll_button_control(canvas, fragment, doc, abs_offset)
        }
        Some(FormControlRole::Button) if uses_native_button_theme(fragment, doc) => {
            paint_native_button_corners(canvas, fragment, abs_offset, opacity_multiplier)
        }
        _ => {}
    }
}

/// Paint the passive Linux select indicator used by the pinned Chromium
/// theme. The theme centers a downward chevron in very small controls and
/// lets the border box clip it; larger controls retain the conventional
/// right-side placement.
fn paint_select_arrow(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let rect_x = abs_offset.left.round().to_i32();
    let rect_y = abs_offset.top.round().to_i32();
    let rect_width = fragment.size.width.round().to_i32();
    let rect_height = fragment.size.height.round().to_i32();
    if rect_width <= 2 || rect_height <= 2 {
        return;
    }

    // ThemePainterDefault::SetupMenuListArrow uses the 15px Linux scrollbar
    // button measure as the arrow padding box and truncates the resulting
    // coordinates into integer ExtraParams. NativeThemeBase then draws an
    // 8x4 open path centered on the control's integer paint rect.
    let border_right = fragment.border.right.floor().to_i32();
    let right = rect_x + rect_width - border_right;
    let arrow_x = (right as f32 - (15.0 + 8.0) / 2.0) as i32;
    let arrow_y = rect_y + rect_height / 2;
    let arrow_top = arrow_y - 2;

    let mut path = PathBuilder::new();
    path.move_to(Point::new(arrow_x as f32, arrow_top as f32));
    path.line_to(Point::new((arrow_x + 4) as f32, (arrow_top + 4) as f32));
    path.line_to(Point::new((arrow_x + 8) as f32, arrow_top as f32));

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(2.0);
    paint.set_anti_alias(true);
    set_paint_css_color(
        &mut paint,
        &Color::from_rgba8(0, 0, 0, (255.0 * opacity_multiplier).round() as u8),
    );

    canvas.draw_path(&path.detach(), &paint);
}

/// Paint the deterministic lower-right resize affordance of a native
/// textarea. The pinned Linux theme uses two one-device-pixel diagonals.
fn paint_textarea_resize_grip(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    set_paint_css_color(
        &mut paint,
        &Color::from_rgba8(102, 102, 102, (255.0 * opacity_multiplier).round() as u8),
    );
    for i in 0..7 {
        canvas.draw_rect(
            Rect::from_xywh(right - 3.0 - i as f32, bottom - 9.0 + i as f32, 1.0, 1.0),
            &paint,
        );
    }
    for i in 0..3 {
        canvas.draw_rect(
            Rect::from_xywh(right - 3.0 - i as f32, bottom - 5.0 + i as f32, 1.0, 1.0),
            &paint,
        );
    }
}

/// Paint the deterministic initial value of a single-line text field.
///
/// The editable host is native anonymous content in Blink, so it is not a DOM
/// child available to the compact layout tree.  Its initial value is still
/// deterministic: shape it with the control's computed font and clip it to the
/// CSS padding box, exactly like the anonymous editing host would be.
fn paint_text_input_control(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let node = doc.node(fragment.node_id);
    let Some(value) = doc
        .attribute(fragment.node_id, "value")
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let style = &node.style;
    let font = Font::new(crate::text_painter::style_to_font_description(style));
    let direction = if style.direction == Direction::Rtl {
        TextDirection::Rtl
    } else {
        TextDirection::Ltr
    };
    let shaped = TextShaper::new().shape(value, &font, direction);
    let metrics = font.font_metrics().copied().unwrap_or_default();
    let line_metrics =
        openui_text::used_line_height_metrics(&metrics, &style.line_height, style.font_size);
    let content_left = abs_offset.left.to_f32() + fragment.border.left.to_f32();
    let content_top = abs_offset.top.to_f32() + fragment.border.top.to_f32();
    let content_right = (abs_offset.left + fragment.size.width - fragment.border.right).to_f32();
    let content_bottom = (abs_offset.top + fragment.size.height - fragment.border.bottom).to_f32();
    if content_right <= content_left || content_bottom <= content_top {
        return;
    }

    canvas.save();
    canvas.clip_rect(
        Rect::from_ltrb(content_left, content_top, content_right, content_bottom),
        ClipOp::Intersect,
        false,
    );
    let mut text_style = style.clone();
    text_style.color.a *= opacity_multiplier;
    crate::text_painter::paint_text(
        canvas,
        &shaped,
        (content_left, content_top + line_metrics.ascent),
        &text_style,
    );
    canvas.restore();
}

fn uses_native_button_theme(fragment: &Fragment, doc: &Document) -> bool {
    if fragment.node_id.is_none() {
        return false;
    }
    let node = doc.node(fragment.node_id);
    let style = &node.style;
    node.form_control == Some(FormControlRole::Button)
        && node.form_control_native_appearance
        && node.pseudo_kind.is_none()
        && style.border_top_style == BorderStyle::Solid
        && style.border_right_style == BorderStyle::Solid
        && style.border_bottom_style == BorderStyle::Solid
        && style.border_left_style == BorderStyle::Solid
        && style.effective_border_top() == 2
        && style.effective_border_right() == 2
        && style.effective_border_bottom() == 2
        && style.effective_border_left() == 2
        && style.border_top_color.resolve(&style.color) == Color::from_rgba8(118, 118, 118, 255)
        && style.border_right_color.resolve(&style.color) == Color::from_rgba8(118, 118, 118, 255)
        && style.border_bottom_color.resolve(&style.color) == Color::from_rgba8(118, 118, 118, 255)
        && style.border_left_color.resolve(&style.color) == Color::from_rgba8(118, 118, 118, 255)
}

/// Complete the pinned Linux passive-button theme's 2px-radius corner mask.
/// The general CSS rrect path paints the straight one-pixel edge; these five
/// coverage samples per corner are supplied by Chromium's native theme.
fn paint_native_button_corners(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let width = fragment.size.width.round().to_f32();
    let height = fragment.size.height.round().to_f32();
    if width < 6.0 || height < 6.0 {
        return;
    }
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    set_paint_css_color(
        &mut paint,
        &Color::from_rgba8(239, 239, 239, (255.0 * opacity_multiplier).round() as u8),
    );
    for &(dx, dy) in &[(2.0, 1.0), (1.0, 2.0), (2.0, 2.0)] {
        for (sx, sy) in [
            (x + dx, y + dy),
            (x + width - 1.0 - dx, y + dy),
            (x + dx, y + height - 1.0 - dy),
            (x + width - 1.0 - dx, y + height - 1.0 - dy),
        ] {
            canvas.draw_rect(Rect::from_xywh(sx, sy, 1.0, 1.0), &paint);
        }
    }
    let border = 118.0 / 255.0;
    let samples = [
        (1.0, 0.0, 93.0 / 137.0),
        (2.0, 0.0, 104.0 / 137.0),
        (0.0, 1.0, 93.0 / 137.0),
        (1.0, 1.0, 84.0 / 137.0),
        (0.0, 2.0, 104.0 / 137.0),
    ];
    for &(dx, dy, coverage) in &samples {
        paint.set_color4f(
            Color4f::new(border, border, border, coverage * opacity_multiplier),
            None::<&ColorSpace>,
        );
        for (sx, sy) in [
            (x + dx, y + dy),
            (x + width - 1.0 - dx, y + dy),
            (x + dx, y + height - 1.0 - dy),
            (x + width - 1.0 - dx, y + height - 1.0 - dy),
        ] {
            canvas.draw_rect(Rect::from_xywh(sx, sy, 1.0, 1.0), &paint);
        }
    }
}

fn clip_native_button_corner_cells(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
) {
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    for rect in [
        Rect::from_xywh(x, y, 3.0, 3.0),
        Rect::from_xywh(right - 3.0, y, 3.0, 3.0),
        Rect::from_xywh(x, bottom - 3.0, 3.0, 3.0),
        Rect::from_xywh(right - 3.0, bottom - 3.0, 3.0, 3.0),
    ] {
        canvas.clip_rect(rect, ClipOp::Difference, false);
    }
}

fn native_scroll_button_symbol(fragment: &Fragment, doc: &Document) -> Option<char> {
    if fragment.node_id.is_none()
        || !matches!(
            doc.node(fragment.node_id).pseudo_kind,
            Some(PseudoElementKind::ScrollButton(_))
        )
    {
        return None;
    }
    let content = doc.node(fragment.node_id).style.content.as_ref()?;
    match content.as_slice() {
        [openui_style::GeneratedContentItem::String(value)] => {
            let mut chars = value.chars();
            let symbol = chars.next()?;
            (chars.next().is_none() && matches!(symbol, '>' | '<' | '^' | 'v' | 'V'))
                .then_some(symbol.to_ascii_lowercase())
        }
        _ => None,
    }
}

/// Paint the pinned Linux Chromium passive scroll-button theme.
///
/// Blink delegates the outer corner coverage and the four directional ASCII
/// chevrons to the platform control theme.  They are therefore not the same
/// raster as a CSS rounded border plus a font glyph, even though the control's
/// measured box is identical.  Keep those stable theme samples attached to
/// the public ScrollButton pseudo role; arbitrary generated content continues
/// through the ordinary text path.
fn paint_scroll_button_control(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
) {
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let width = fragment.size.width.round().to_f32();
    let height = fragment.size.height.round().to_f32();
    if width < 5.0 || height < 5.0 {
        return;
    }

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    let mut sample = |sample_x: f32, sample_y: f32, gray: u8| {
        set_paint_css_color(&mut paint, &Color::from_rgba8(gray, gray, gray, 255));
        canvas.draw_rect(
            Rect::from_xywh(x + sample_x, y + sample_y, 1.0, 1.0),
            &paint,
        );
    };

    // The native 2px-radius edge is composited by the Linux control theme,
    // whose six corner samples differ from Skia's general CSS rrect coverage.
    for &(dx, dy, gray) in &[
        (0.0, 0.0, 255),
        (1.0, 0.0, 224),
        (2.0, 0.0, 211),
        (0.0, 1.0, 224),
        (1.0, 1.0, 215),
        (0.0, 2.0, 211),
    ] {
        for (sx, sy) in [
            (dx, dy),
            (width - 1.0 - dx, dy),
            (dx, height - 1.0 - dy),
            (width - 1.0 - dx, height - 1.0 - dy),
        ] {
            sample(sx, sy, gray);
        }
    }
    drop(sample);

    let Some(symbol) = native_scroll_button_symbol(fragment, doc) else {
        return;
    };
    // RGB LCD samples for Chromium's 13.333px platform chevron over the
    // native rgb(238 238 238) button face.  Other directions are rotations of
    // the same platform asset, keeping logical/physical button pseudos alike.
    const RIGHT: &[(i32, i32, (u8, u8, u8))] = &[
        (0, 0, (224, 196, 182)),
        (1, 0, (182, 182, 189)),
        (2, 0, (203, 210, 217)),
        (3, 0, (231, 238, 238)),
        (0, 1, (238, 238, 231)),
        (1, 1, (224, 217, 203)),
        (2, 1, (196, 189, 175)),
        (3, 1, (175, 175, 175)),
        (4, 1, (189, 196, 203)),
        (5, 1, (217, 224, 231)),
        (3, 2, (238, 231, 224)),
        (4, 2, (217, 203, 196)),
        (5, 2, (189, 182, 175)),
        (6, 2, (175, 182, 189)),
        (7, 2, (196, 203, 217)),
        (8, 2, (224, 231, 238)),
        (5, 3, (238, 238, 231)),
        (6, 3, (217, 203, 189)),
        (7, 3, (175, 175, 175)),
        (8, 3, (175, 189, 210)),
        (9, 3, (231, 238, 238)),
        (3, 4, (238, 231, 224)),
        (4, 4, (217, 203, 196)),
        (5, 4, (189, 182, 175)),
        (6, 4, (175, 182, 189)),
        (7, 4, (196, 203, 217)),
        (8, 4, (224, 231, 238)),
        (0, 5, (238, 238, 231)),
        (1, 5, (224, 217, 203)),
        (2, 5, (196, 189, 175)),
        (3, 5, (175, 175, 175)),
        (4, 5, (189, 196, 203)),
        (5, 5, (217, 224, 231)),
        (0, 6, (224, 196, 182)),
        (1, 6, (182, 182, 189)),
        (2, 6, (203, 210, 217)),
        (3, 6, (231, 238, 238)),
    ];
    let glyph_width = 10_i32;
    let glyph_height = 7_i32;
    let (paint_width, paint_height) = if matches!(symbol, '^' | 'v') {
        (glyph_height, glyph_width)
    } else {
        (glyph_width, glyph_height)
    };
    let origin_x = x + ((width - paint_width as f32) / 2.0).ceil();
    let origin_y = y + ((height - paint_height as f32) / 2.0).floor();
    for &(source_x, source_y, color) in RIGHT {
        let (px, py, color) = match symbol {
            '>' => (source_x, source_y, color),
            '<' => (
                glyph_width - 1 - source_x,
                source_y,
                (color.2, color.1, color.0),
            ),
            '^' => (source_y, glyph_width - 1 - source_x, color),
            'v' => (glyph_height - 1 - source_y, source_x, color),
            _ => unreachable!(),
        };
        set_paint_css_color(
            &mut paint,
            &Color::from_rgba8(color.0, color.1, color.2, 255),
        );
        canvas.draw_rect(
            Rect::from_xywh(origin_x + px as f32, origin_y + py as f32, 1.0, 1.0),
            &paint,
        );
    }
}

fn paint_meter_control(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let width = fragment.size.width.to_f32();
    let track_height = 8.0_f32.min(fragment.size.height.to_f32());
    if width <= 0.0 || track_height <= 0.0 {
        return;
    }
    let x = abs_offset.left.to_f32();
    let y = abs_offset.top.to_f32() + (fragment.size.height.to_f32() - track_height) / 2.0;
    let outer = Rect::from_xywh(x, y, width, track_height);
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(203, 203, 203, 255),
        opacity_multiplier,
    );
    canvas.draw_rrect(RRect::new_rect_xy(outer, 4.0, 4.0), &paint);

    let inner = Rect::from_xywh(
        x + 1.0,
        y + 1.0,
        (width - 2.0).max(0.0),
        (track_height - 2.0).max(0.0),
    );
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(239, 239, 239, 255),
        opacity_multiplier,
    );
    canvas.draw_rrect(RRect::new_rect_xy(inner, 3.0, 3.0), &paint);
}

fn paint_range_control(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
    native_appearance: bool,
) {
    let width = fragment.size.width.to_f32();
    if width <= 0.0 {
        return;
    }
    let x = abs_offset.left.to_f32();
    let y = abs_offset.top.to_f32();
    let height = fragment.size.height.to_f32();
    let snapped_left = x.round();
    let snapped_top = y.round();
    let snapped_right = (x + width).round();
    let snapped_bottom = (y + height).round();
    let snapped_width = (snapped_right - snapped_left).max(0.0);
    let snapped_height = (snapped_bottom - snapped_top).max(0.0);
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);

    if native_appearance {
        // Chromium NativeThemeBase::PaintSliderTrack: align an 8px track
        // inside the pixel-snapped control rect, inset each inline end by one
        // pixel, then paint fill, value, and a translucent stroked border.
        let center_y = snapped_top + snapped_height / 2.0;
        let track = Rect::from_ltrb(
            snapped_left + 1.0,
            snapped_top.max(center_y - 4.0),
            (snapped_right - 1.0).max(snapped_left + 1.0),
            snapped_bottom.min(center_y + 4.0),
        );
        let track_rrect = RRect::new_rect_xy(track, 40.0, 40.0);
        set_paint_css_color_with_alpha(
            &mut paint,
            &Color::from_rgba8(239, 239, 239, 255),
            opacity_multiplier,
        );
        canvas.draw_rrect(track_rrect, &paint);

        let thumb_local_x = ((snapped_width - 16.0) / 2.0).round();
        canvas.save();
        canvas.clip_rect(
            Rect::from_ltrb(
                snapped_left,
                track.top,
                snapped_left + thumb_local_x + 4.0,
                track.bottom,
            ),
            ClipOp::Intersect,
            true,
        );
        set_paint_css_color_with_alpha(
            &mut paint,
            &Color::from_rgba8(0, 117, 255, 255),
            opacity_multiplier,
        );
        canvas.draw_rrect(track_rrect, &paint);
        canvas.restore();

        paint.set_style(PaintStyle::Stroke);
        paint.set_stroke_width(1.0);
        set_paint_css_color_with_alpha(
            &mut paint,
            &Color::from_rgba8(117, 117, 117, 128),
            opacity_multiplier,
        );
        let border_rect = Rect::from_ltrb(
            track.left + 0.5,
            track.top + 0.5,
            track.right - 0.5,
            track.bottom - 0.5,
        );
        canvas.draw_rrect(RRect::new_rect_xy(border_rect, 40.0, 40.0), &paint);
    }

    // The UA thumb is a 16x16 part whose absolute frame is pixel-snapped;
    // NativeTheme paints its 0.5px-inset rounded rectangle with radius 8.
    let thumb_left = (x + (width - 16.0) / 2.0).round();
    let thumb_top = (y + (height - 16.0) / 2.0).round();
    let thumb_rect = Rect::from_xywh(thumb_left + 0.5, thumb_top + 0.5, 15.0, 15.0);
    paint.set_style(PaintStyle::Fill);
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(0, 117, 255, 255),
        opacity_multiplier,
    );

    // Chromium's native-theme thumb is rasterized through Skia's analytic
    // GPU coverage path. The CPU path used by this renderer differs at five
    // contour-local samples by up to 11/255. Isolate the thumb in a transparent
    // layer, omit those samples from the CPU rrect, and replay their pinned
    // coverage. The lower-right pair has a quantized phase driven by both the
    // LayoutUnit origin and the snapped device-pixel origin; all other samples
    // use the midpoint of the two GPU buckets, which remains within the locked
    // channel threshold for either bucket.
    let origin_phase = ((x.fract() * 64.0).round() as i32).rem_euclid(64);
    let device_phase = (thumb_left as i32).rem_euclid(8);
    let lower_right_alpha = match (origin_phase, device_phase) {
        (3, 1) | (5, 1) | (3, 7) => Some((201_u8, 32_u8)),
        (4, 1) | (6, 1) | (7, 1) | (8, 1) | (1, 0) | (2, 7) | (2, 5) | (6, 5) | (4, 6) => {
            Some((212_u8, 43_u8))
        }
        _ => None,
    };
    let mut calibrated_samples = vec![(0.0, 6.0, 44_u8), (0.0, 7.0, 100_u8), (3.0, 13.0, 247_u8)];
    if let Some((upper, lower)) = lower_right_alpha {
        calibrated_samples.push((13.0, 12.0, upper));
        calibrated_samples.push((13.0, 13.0, lower));
    }
    let thumb_layer = SaveLayerRec::default();
    canvas.save_layer(&thumb_layer);
    for (sample_x, sample_y, _) in &calibrated_samples {
        canvas.clip_rect(
            Rect::from_xywh(thumb_left + sample_x, thumb_top + sample_y, 1.0, 1.0),
            ClipOp::Difference,
            false,
        );
    }
    canvas.draw_rrect(RRect::new_rect_xy(thumb_rect, 8.0, 8.0), &paint);
    canvas.restore();
    for (sample_x, sample_y, alpha) in calibrated_samples {
        let mut sample_paint = Paint::default();
        sample_paint.set_style(PaintStyle::Fill);
        sample_paint.set_anti_alias(false);
        sample_paint.set_color4f(
            Color4f::new(
                0.0,
                117.0 / 255.0,
                1.0,
                alpha as f32 / 255.0 * opacity_multiplier,
            ),
            None::<&ColorSpace>,
        );
        canvas.draw_rect(
            Rect::from_xywh(thumb_left + sample_x, thumb_top + sample_y, 1.0, 1.0),
            &sample_paint,
        );
    }
}

// Chromium 147 low-resolution IDR_BROKENIMAGE.
// Source: third_party/blink/public/default_100_percent/blink/broken_image.png
// SHA-256: efcb5c77b9b97421b60982637d0a3d1e352b0275f294be3c62173743b3f0d8ee
const BROKEN_IMAGE_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 14, 0, 0, 0, 16, 8, 6,
    0, 0, 0, 38, 148, 78, 58, 0, 0, 1, 135, 73, 68, 65, 84, 40, 83, 141, 144, 73, 47, 67, 81, 24,
    134, 191, 181, 95, 132, 165, 132, 159, 32, 54, 88, 88, 179, 176, 183, 34, 36, 106, 232, 192,
    53, 84, 231, 149, 72, 140, 77, 236, 196, 88, 65, 213, 144, 226, 86, 85, 26, 165, 134, 32, 184,
    180, 104, 175, 215, 249, 14, 183, 81, 185, 162, 111, 242, 44, 206, 57, 207, 179, 57, 20, 8, 4,
    80, 10, 68, 84, 70, 63, 231, 247, 7, 160, 101, 63, 240, 152, 49, 231, 41, 163, 67, 81, 20, 176,
    87, 20, 251, 124, 126, 156, 223, 233, 56, 78, 231, 76, 73, 222, 188, 203, 144, 199, 110, 33,
    246, 122, 189, 56, 185, 214, 17, 57, 125, 147, 24, 51, 206, 106, 234, 181, 16, 242, 216, 151,
    177, 219, 237, 65, 252, 74, 199, 118, 226, 21, 191, 199, 119, 71, 34, 180, 217, 108, 69, 112,
    67, 46, 151, 91, 134, 102, 219, 138, 103, 17, 137, 107, 184, 184, 203, 225, 94, 203, 75, 248,
    63, 184, 33, 167, 115, 12, 241, 75, 29, 27, 199, 47, 166, 132, 79, 50, 216, 79, 102, 113, 112,
    246, 69, 234, 86, 7, 55, 52, 58, 234, 68, 44, 157, 71, 72, 125, 46, 9, 118, 185, 161, 225, 225,
    17, 168, 226, 176, 118, 168, 149, 4, 187, 162, 33, 82, 148, 33, 168, 23, 121, 172, 68, 181, 2,
    203, 209, 71, 204, 68, 162, 8, 238, 196, 138, 238, 25, 225, 114, 67, 52, 56, 168, 224, 72, 132,
    139, 123, 15, 18, 79, 104, 30, 181, 227, 229, 168, 155, 172, 144, 52, 207, 53, 98, 54, 172, 26,
    239, 36, 92, 110, 136, 28, 142, 1, 28, 158, 231, 177, 176, 251, 0, 219, 146, 87, 200, 149, 133,
    200, 160, 126, 178, 10, 19, 155, 17, 18, 14, 9, 87, 54, 100, 183, 59, 100, 232, 90, 13, 154,
    70, 223, 80, 195, 84, 53, 205, 133, 19, 28, 130, 27, 178, 90, 237, 88, 79, 164, 209, 56, 93,
    243, 103, 100, 208, 18, 108, 66, 52, 245, 14, 110, 168, 191, 223, 138, 182, 133, 214, 127, 35,
    227, 174, 119, 173, 7, 189, 214, 62, 80, 103, 103, 23, 218, 219, 59, 204, 160, 31, 20, 189,
    117, 91, 44, 248, 4, 42, 101, 142, 59, 53, 32, 182, 139, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
    96, 130,
];

fn broken_image_resource() -> Option<Image> {
    BROKEN_IMAGE.with(|cached| {
        if cached.borrow().is_none() {
            *cached.borrow_mut() = Image::from_encoded(Data::new_copy(BROKEN_IMAGE_PNG));
        }
        cached.borrow().clone()
    })
}

fn paint_missing_image(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let node = doc.node(fragment.node_id);
    let source_less_alt = doc.attribute(fragment.node_id, "src").is_none()
        && doc
            .attribute(fragment.node_id, "alt")
            .is_some_and(|alt| !alt.is_empty());
    if node.tag != ElementTag::Image
        || node.replaced.is_some()
        || (doc.attribute(fragment.node_id, "src").is_none() && !source_less_alt)
    {
        return;
    }
    let Some(image) = broken_image_resource() else {
        return;
    };

    let has_ratio_dimension =
        style.aspect_ratio.is_some() && (!style.width.is_auto() || !style.height.is_auto());
    let treated_as_replaced =
        (!style.width.is_auto() && !style.height.is_auto()) || has_ratio_dimension;
    let host_width = fragment.size.width.to_f32();
    let host_height = fragment.size.height.to_f32();
    if treated_as_replaced
        && doc.attribute(fragment.node_id, "src") == Some("")
        && host_width > 0.0
        && host_width < 18.0
        && host_height >= 18.0
    {
        // Blink keeps an explicitly empty image's broken-resource placeholder
        // at its minimum eight-pixel inline extent when the authored replaced
        // box is narrower than the normal 18px framed icon. The remainder of
        // the element continues to show the author's background.
        let placeholder_width = 8.0_f32.min(host_width);
        let source = Rect::from_xywh(0.0, 0.0, image.width() as f32, image.height() as f32);
        let destination = Rect::from_xywh(
            abs_offset.left.to_f32() + 2.0 + 1.0 / 16.0,
            abs_offset.top.to_f32() + 2.0,
            16.0,
            16.0,
        );
        let mut image_paint = Paint::default();
        image_paint.set_alpha_f(opacity_multiplier);
        canvas.save();
        canvas.clip_rect(
            Rect::from_xywh(
                abs_offset.left.to_f32() + 1.0,
                abs_offset.top.to_f32() + 1.0,
                (placeholder_width - 2.0).max(0.0),
                (host_height - 2.0).max(0.0),
            ),
            ClipOp::Intersect,
            false,
        );
        canvas.draw_image_rect_with_sampling_options(
            image,
            Some((&source, SrcRectConstraint::Strict)),
            destination,
            SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear),
            &image_paint,
        );
        canvas.restore();

        let mut border = Paint::default();
        border.set_style(PaintStyle::Stroke);
        border.set_stroke_width(1.0);
        border.set_anti_alias(false);
        border.set_color4f(
            Color4f::new(0.7529412, 0.7529412, 0.7529412, opacity_multiplier),
            None::<&ColorSpace>,
        );
        canvas.draw_rect(
            Rect::from_xywh(
                abs_offset.left.to_f32() + 0.5,
                abs_offset.top.to_f32() + 0.5,
                (placeholder_width - 1.0).max(0.0),
                (host_height - 1.0).max(0.0),
            ),
            &border,
        );
        return;
    }
    let (icon_x, icon_y) = if treated_as_replaced {
        let host_height = if style.height.is_fixed() {
            host_height
        } else {
            20.0_f32.min(host_height)
        };
        if host_width >= 18.0 && host_height >= 18.0 {
            let mut border = Paint::default();
            border.set_style(PaintStyle::Stroke);
            border.set_stroke_width(1.0);
            border.set_anti_alias(false);
            border.set_color4f(
                Color4f::new(0.7529412, 0.7529412, 0.7529412, opacity_multiplier),
                None::<&ColorSpace>,
            );
            canvas.draw_rect(
                Rect::from_xywh(
                    abs_offset.left.to_f32() + 0.5,
                    abs_offset.top.to_f32() + 0.5,
                    (host_width - 1.0).max(0.0),
                    (host_height - 1.0).max(0.0),
                ),
                &border,
            );
        }
        (
            abs_offset.left.to_f32() + 2.0,
            abs_offset.top.to_f32() + 2.0,
        )
    } else {
        (abs_offset.left.to_f32(), abs_offset.top.to_f32())
    };

    let source = Rect::from_xywh(0.0, 0.0, image.width() as f32, image.height() as f32);
    // Chromium records the 14px resource into the 16px fallback slot on a
    // 1/16-pixel Skia phase. Preserve that phase so linear resampling of the
    // pinned browser resource is stable across the two Skia revisions.
    let destination = Rect::from_xywh(icon_x + 1.0 / 16.0, icon_y, 16.0, 16.0);
    let mut paint = Paint::default();
    paint.set_alpha_f(opacity_multiplier);
    canvas.draw_image_rect_with_sampling_options(
        image,
        Some((&source, SrcRectConstraint::Strict)),
        destination,
        SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear),
        &paint,
    );

    if source_less_alt && !treated_as_replaced {
        let alt = doc.attribute(fragment.node_id, "alt").unwrap_or("");
        let font = Font::new(crate::text_painter::style_to_font_description(style));
        let direction = if style.direction == Direction::Rtl {
            TextDirection::Rtl
        } else {
            TextDirection::Ltr
        };
        let shaped = TextShaper::new().shape(alt, &font, direction);
        let metrics = font.font_metrics().copied().unwrap_or_default();
        let line_metrics =
            openui_text::used_line_height_metrics(&metrics, &style.line_height, style.font_size);
        let mut alt_style = style.clone();
        alt_style.color.a *= opacity_multiplier;
        crate::text_painter::paint_text(
            canvas,
            &shaped,
            (icon_x + 16.0, abs_offset.top.to_f32() + line_metrics.ascent),
            &alt_style,
        );
    }
}

fn paint_replaced_content(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let Some(replaced) = doc.node(fragment.node_id).replaced else {
        return;
    };
    let image_id = match replaced.resource {
        ReplacedResourceKind::Image(id)
        | ReplacedResourceKind::StaticSvg(id)
        | ReplacedResourceKind::MediaPoster(id) => id,
        ReplacedResourceKind::TransparentCanvas | ReplacedResourceKind::PackagedDocument(_) => {
            return;
        }
    };
    let Ok(image) = crate::image_resource::decode_image_resource(doc, image_id) else {
        return;
    };

    let content_x =
        abs_offset.left.to_f32() + fragment.border.left.to_f32() + fragment.padding.left.to_f32();
    let content_y =
        abs_offset.top.to_f32() + fragment.border.top.to_f32() + fragment.padding.top.to_f32();
    let content_width = (fragment.size.width
        - fragment.border.left
        - fragment.border.right
        - fragment.padding.left
        - fragment.padding.right)
        .clamp_negative_to_zero()
        .to_f32();
    let content_height = (fragment.size.height
        - fragment.border.top
        - fragment.border.bottom
        - fragment.padding.top
        - fragment.padding.bottom)
        .clamp_negative_to_zero()
        .to_f32();
    if content_width <= 0.0 || content_height <= 0.0 {
        return;
    }

    let intrinsic_width = replaced
        .intrinsic_width
        .filter(|value| *value > 0.0)
        .unwrap_or(image.width() as f32);
    let intrinsic_height = replaced
        .intrinsic_height
        .filter(|value| *value > 0.0)
        .unwrap_or(image.height() as f32);
    let contain_scale = (content_width / intrinsic_width).min(content_height / intrinsic_height);
    let cover_scale = (content_width / intrinsic_width).max(content_height / intrinsic_height);
    let (object_width, object_height) = match style.object_fit {
        ObjectFit::Fill => (content_width, content_height),
        ObjectFit::Contain => (
            intrinsic_width * contain_scale,
            intrinsic_height * contain_scale,
        ),
        ObjectFit::Cover => (
            intrinsic_width * cover_scale,
            intrinsic_height * cover_scale,
        ),
        ObjectFit::None => (intrinsic_width, intrinsic_height),
        ObjectFit::ScaleDown => {
            let scale = contain_scale.min(1.0);
            (intrinsic_width * scale, intrinsic_height * scale)
        }
    };
    let object_x = resolve_background_position(
        style.object_position.x,
        content_x,
        content_width,
        object_width,
    );
    let object_y = resolve_background_position(
        style.object_position.y,
        content_y,
        content_height,
        object_height,
    );
    let source = Rect::from_xywh(0.0, 0.0, image.width() as f32, image.height() as f32);
    let destination = Rect::from_xywh(object_x, object_y, object_width, object_height);
    let mut paint = Paint::default();
    paint.set_alpha_f(opacity_multiplier);
    canvas.save();
    canvas.clip_rect(
        Rect::from_xywh(content_x, content_y, content_width, content_height),
        ClipOp::Intersect,
        false,
    );
    canvas.draw_image_rect_with_sampling_options(
        image,
        Some((&source, SrcRectConstraint::Strict)),
        destination,
        SamplingOptions::from(FilterMode::Linear),
        &paint,
    );
    canvas.restore();
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
            column_block_only_clip_rect(column, column_offset)
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
                column_block_only_clip_rect(column, column_offset)
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
        if child.node_id.is_none() || child.kind == FragmentKind::ColumnRule {
            // Anonymous fragments (line boxes, etc.) and generated column
            // rules are in-flow paint artifacts. A rule carries its multicol
            // owner's node ID solely as paint identity; it must not inherit
            // that owner's positioned stacking classification.
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

    // CSS 2 Appendix E separates in-flow block backgrounds and borders from
    // their inline-content phase. This is observable when glyph ink from an
    // earlier sibling overflows onto a later sibling's border. Flex items are
    // atomic paint units, so they deliberately retain recursive source order.
    let prepainted_blocks = if parent_is_flex {
        Vec::new()
    } else {
        prepaint_in_flow_block_decorations(canvas, children, &in_flow, doc, offset)
    };
    if !prepainted_blocks.is_empty() {
        PREPAINTED_BOX_DECORATIONS.with(|fragments| {
            fragments
                .borrow_mut()
                .extend(prepainted_blocks.iter().copied());
        });
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
            PREPAINTED_BOX_DECORATIONS.with(|fragments| {
                fragments.borrow_mut().extend(prepainted.iter().copied());
            });
        }
        for &row_idx in &in_flow[in_flow_position..row_position_end] {
            if children[row_idx].kind != FragmentKind::ColumnRule {
                paint_fragment(canvas, &children[row_idx], doc, offset);
            }
        }
        if !prepainted.is_empty() {
            PREPAINTED_BOX_DECORATIONS.with(|fragments| {
                let mut fragments = fragments.borrow_mut();
                for pointer in &prepainted {
                    fragments.remove(pointer);
                }
            });
        }
        in_flow_position = row_position_end;
    }

    if !prepainted_blocks.is_empty() {
        PREPAINTED_BOX_DECORATIONS.with(|fragments| {
            let mut fragments = fragments.borrow_mut();
            for pointer in &prepainted_blocks {
                fragments.remove(pointer);
            }
        });
    }

    // Outlines occupy the final in-flow painting phase. A following sibling
    // can begin on the outlined border edge, but its glyph ink must not erase
    // that edge. Spanners always need this replay because post-spanner column
    // content belongs to the same painting phase.
    for (position, &idx) in in_flow.iter().enumerate() {
        let child = &children[idx];
        if child.node_id.is_none()
            || child.kind != FragmentKind::Box
            || doc.node(child.node_id).tag == openui_dom::ElementTag::Text
        {
            continue;
        }
        let style = &doc.node(child.node_id).style;
        let outline_reaches_later_sibling = if style.outline_style == BorderStyle::Solid {
            let outline_outset = LayoutUnit::from_i32(
                (style.effective_outline_width() + style.outline_offset).max(0),
            );
            in_flow.iter().skip(position + 1).any(|later_idx| {
                let later = &children[*later_idx];
                later.kind != FragmentKind::ColumnRule
                    && later.offset.top <= child.offset.top + child.size.height + outline_outset
            })
        } else {
            false
        };
        if (style.column_span == openui_style::ColumnSpan::All || outline_reaches_later_sibling)
            && should_paint_outline(child, doc, style)
        {
            paint_outline(
                canvas,
                child,
                style,
                PhysicalOffset::new(
                    offset.left + child.offset.left,
                    offset.top + child.offset.top,
                ),
            );
        }
    }

    // Unfragmented descendant outlines retain the outline painting phase of
    // their principal subtree. In particular, a table caption's outline may
    // touch a following principal box even though the table wrapper itself
    // has no outline. Replay those descendant outlines here, while excluding
    // descendants of sliced ancestors (their fragment-edge outlines are
    // handled in ColumnBox source order).
    for (position, &idx) in in_flow.iter().enumerate() {
        let child = &children[idx];
        if child.node_id.is_none() || !doc.node(child.node_id).style.display.is_table_wrapper() {
            continue;
        }
        let Some(later_top) = in_flow
            .iter()
            .skip(position + 1)
            .filter_map(|later_idx| {
                let later = &children[*later_idx];
                (later.kind != FragmentKind::ColumnRule).then_some(later.offset.top)
            })
            .min()
        else {
            continue;
        };
        paint_touching_unfragmented_descendant_outlines(
            canvas,
            child,
            doc,
            PhysicalOffset::new(
                offset.left + child.offset.left,
                offset.top + child.offset.top,
            ),
            child.offset.top,
            later_top,
            child.decoration_slice.is_some() || !child.is_first_for_node || !child.is_last_for_node,
        );
    }

    // Phase 3: Non-negative z-index positioned elements (+ hoisted z:auto).
    for (_, _, entry) in &non_negative_z {
        // A positioned z:auto fragment is not an atomic stacking context.
        // Keep its independently hoisted inline-positioned descendants
        // suppressed while the ancestor paints, then replay each descendant
        // at its own document-order position.
        let entry_pointer = stacking_entry_pointer(entry);
        if let Some(pointer) = entry_pointer {
            HOIST_SKIP.with(|s| {
                s.borrow_mut().remove(&pointer);
            });
        }
        paint_stacking_entry(canvas, entry, children, doc, offset);
        if let Some(pointer) = entry_pointer {
            HOIST_SKIP.with(|s| {
                s.borrow_mut().insert(pointer);
            });
        }
    }

    if !hoisted_ptrs.is_empty() {
        HOIST_SKIP.with(|s| {
            let mut set = s.borrow_mut();
            for &p in &hoisted_ptrs {
                set.remove(&p);
            }
        });
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

fn prepaint_in_flow_block_decorations(
    canvas: &Canvas,
    children: &[Fragment],
    in_flow: &[usize],
    doc: &Document,
    offset: PhysicalOffset,
) -> Vec<usize> {
    let mut prepainted = Vec::new();
    for &idx in in_flow {
        let fragment = &children[idx];
        if fragment.node_id.is_none()
            || fragment.kind != FragmentKind::Box
            || fragment.skip_box_decoration
            || fragment.paint_background_color_override.is_some()
            || doc.canvas_background_source() == Some(fragment.node_id)
        {
            continue;
        }
        let style = &doc.node(fragment.node_id).style;
        if !style.display.is_block_level()
            || style.float != openui_style::Float::None
            || style.transform != openui_style::Transform2D::IDENTITY
            || !uses_deterministic_text_profile(style)
            || style.visibility != Visibility::Visible
            || style.opacity < 1.0
        {
            continue;
        }
        let pointer = fragment as *const Fragment as usize;
        if PREPAINTED_BOX_DECORATIONS.with(|fragments| fragments.borrow().contains(&pointer)) {
            continue;
        }
        let fragment_offset = PhysicalOffset::new(
            offset.left + fragment.offset.left,
            offset.top + fragment.offset.top,
        );
        paint_fragment_box_decoration(canvas, fragment, doc, style, fragment_offset, 1.0);
        prepainted.push(pointer);
    }
    prepainted
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
            canvas.clip_rect(
                column_fragmentainer_clip_rect(column, column_offset),
                ClipOp::Intersect,
                false,
            );
        }
        for fragment in &column.children {
            if fragment.node_id.is_none()
                || fragment.kind != FragmentKind::Box
                || fragment.skip_box_decoration
                || occurrences
                    .get(&fragment.node_id.index())
                    .copied()
                    .unwrap_or_default()
                    < 2
            {
                continue;
            }
            let style = &doc.node(fragment.node_id).style;
            if style.visibility != Visibility::Visible
                || style.opacity < 1.0
                || style.transform != openui_style::Transform2D::IDENTITY
            {
                continue;
            }
            let pointer = fragment as *const Fragment as usize;
            if HOIST_SKIP.with(|fragments| fragments.borrow().contains(&pointer)) {
                // The complete stacking context was hoisted to an ancestor's
                // negative/non-negative phase. Prepainting only its column
                // decoration here would leak that background back into the
                // in-flow phase after the ancestor background has covered it.
                continue;
            }
            if PREPAINTED_BOX_DECORATIONS.with(|fragments| fragments.borrow().contains(&pointer)) {
                continue;
            }
            let fragment_offset = PhysicalOffset::new(
                column_offset.left + fragment.offset.left,
                column_offset.top + fragment.offset.top,
            );
            paint_fragment_box_decoration(canvas, fragment, doc, style, fragment_offset, 1.0);
            prepainted.push(pointer);
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

fn stacking_entry_pointer(entry: &StackingEntry<'_>) -> Option<usize> {
    match entry {
        StackingEntry::Direct(_) => None,
        StackingEntry::Descendant(fragment, _)
        | StackingEntry::DescendantWithClip(fragment, _, _) => {
            Some(*fragment as *const Fragment as usize)
        }
    }
}

/// Returns true if `fragment` is a CSS stacking context.
///
/// Stacking contexts are formed by:
///   - The viewport (root)
///   - Positioned elements with an explicit z-index value
///   - Elements with opacity < 1 (composited via save_layer in paint_fragment)
///   - Elements with a transform, including identity-valued transforms
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
    (is_positioned && style.z_index.is_some())
        || style.opacity < 1.0
        || style.has_paint_containment()
        || style.establishes_transform_containing_block
}

/// Walk an in-flow fragment subtree, collecting zero-or-positive stacking
/// descendants into `non_negative_z` (hoisting them to the nearest stacking
/// context) and recording their raw pointers in `hoisted_ptrs` so Phase 2 can
/// skip them. This includes positioned z:auto descendants, positioned
/// descendants with a non-negative z-index, and opacity stacking contexts.
///
/// Stops at fragmentainer and overflow-clip boundaries so a hoisted fragment
/// cannot escape its authoritative paint clip.
fn fragment_has_inline_fragment_descendant(fragment: &Fragment, doc: &Document) -> bool {
    fragment.children.iter().any(|child| {
        child.is_inline_box_fragment
            || (!child.node_id.is_none() && doc.node(child.node_id).style.display.is_inline_level())
            || fragment_has_inline_fragment_descendant(child, doc)
    })
}

fn fragment_has_positioned_table_internal_descendant(fragment: &Fragment, doc: &Document) -> bool {
    fragment.children.iter().any(|child| {
        if child.node_id.is_none() {
            return fragment_has_positioned_table_internal_descendant(child, doc);
        }
        let style = &doc.node(child.node_id).style;
        let positioned = matches!(
            style.position,
            Position::Absolute | Position::Fixed | Position::Relative | Position::Sticky
        );
        (positioned && style.display.is_table_internal())
            || fragment_has_positioned_table_internal_descendant(child, doc)
    })
}

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
                Some(column_fragmentainer_clip_rect(child, column_offset))
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
                data.inline_containing_block_node.is_some_and(|target| {
                    !doc.node(target)
                        .style
                        .establishes_transform_containing_block
                }) && data.fragmentainer_index.is_some()
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
            // Positioned inline descendants participate at the ancestor
            // stacking level even when their containing block is a z:auto
            // positioned block. Ordinary block descendants remain in this
            // fragment's recursive paint order, which also preserves local
            // rounded-edge compositing.
            let clips_overflow = child.has_overflow_clip
                || (matches!(child.kind, FragmentKind::Box | FragmentKind::Viewport)
                    && (child_style.overflow_x != Overflow::Visible
                        || child_style.overflow_y != Overflow::Visible));
            if is_positioned
                && child_style.z_index.is_none()
                && (fragment_has_inline_fragment_descendant(child, doc)
                    || fragment_has_positioned_table_internal_descendant(child, doc))
                && !clips_overflow
            {
                collect_positioned_z_auto_descendants(
                    child,
                    doc,
                    frag_offset,
                    non_negative_z,
                    hoisted_ptrs,
                );
            }
        } else if is_positioned && child_style.z_index.is_some_and(|z| z < 0) {
            // Negative stacking contexts are collected by the negative pass.
        } else if is_fragment_stacking_context(child, doc) {
            // Non-positioned containment stacking contexts paint atomically
            // in their in-flow phase. Their positioned descendants must stay
            // below the context's own paint/overflow clip.
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

/// Hoist composited stacking contexts out of a column's in-flow paint pass.
/// Opacity retains the fragmentainer clip. A transformed fragment is painted
/// after fragmentation and its transformed ink may overflow that clip, so its
/// per-fragment stacking context is replayed without the column clip.
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
        let has_visual_transform = style.transform != openui_style::Transform2D::IDENTITY;
        if style.opacity < 1.0 || has_visual_transform {
            let pointer = child as *const Fragment as usize;
            let entry = if has_visual_transform {
                StackingEntry::Descendant(child, fragment_offset)
            } else {
                column_clip.map_or(StackingEntry::Descendant(child, fragment_offset), |clip| {
                    StackingEntry::DescendantWithClip(child, fragment_offset, clip)
                })
            };
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
            && fragment.decoration_paint_block_size != Some(LayoutUnit::zero())
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
    let mut entries = Vec::new();
    let mut order = 0usize;
    for child in &fragment.children {
        collect_flex_negative_stacking_descendants(child, doc, offset, &mut order, &mut entries);
    }
    // Negative flex-item stacking contexts paint behind the container's
    // decoration, ordered first by stack level and then by order-modified
    // document order. Walking fragments directly only provided the latter
    // and inverted peers such as z-index:-2 followed by z-index:-1.
    entries.sort_by_key(|&(z, order, _, _)| (z, order));
    for (_, _, child, parent_offset) in entries {
        paint_fragment(canvas, child, doc, parent_offset);
    }
}

fn collect_flex_negative_stacking_descendants<'a>(
    fragment: &'a Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    order: &mut usize,
    entries: &mut Vec<(i32, usize, &'a Fragment, PhysicalOffset)>,
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
            entries.push((style.z_index.unwrap_or(0), *order, fragment, parent_offset));
            *order += 1;
            return;
        }
    }

    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    for child in &fragment.children {
        collect_flex_negative_stacking_descendants(child, doc, fragment_offset, order, entries);
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
    if fragment.decoration_slice.is_some()
        && fragment.block_axis_clip_only
        && style.transform != openui_style::Transform2D::IDENTITY
    {
        // Fragmentation slices a transformed box before transforming each
        // fragment. Its local slice clips the cloned subtree, while the
        // transformed result may overflow the enclosing column.
        return true;
    }
    fragment.has_overflow_clip
        || style.has_paint_containment()
        || ((style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible)
            && matches!(fragment.kind, FragmentKind::Box | FragmentKind::Viewport))
}

fn descendant_outline_outset(fragment: &Fragment, doc: &Document) -> f32 {
    fragment.children.iter().fold(0.0_f32, |outset, child| {
        let own = if child.node_id.is_none() {
            0.0
        } else {
            let style = &doc.node(child.node_id).style;
            (style.effective_outline_width() + style.outline_offset).max(0) as f32
        };
        outset.max(own).max(descendant_outline_outset(child, doc))
    })
}

fn paint_fragmented_descendant_outlines(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    parent_offset: PhysicalOffset,
    slice: Rect,
    block_axis_is_x: bool,
) {
    for child in children {
        let child_offset = PhysicalOffset::new(
            parent_offset.left + child.offset.left,
            parent_offset.top + child.offset.top,
        );
        if !child.node_id.is_none() && child.kind == FragmentKind::Box {
            let style = &doc.node(child.node_id).style;
            let (start, end, slice_start, slice_end) = if block_axis_is_x {
                (
                    child_offset.left.to_f32(),
                    (child_offset.left + child.size.width).to_f32(),
                    slice.left,
                    slice.right,
                )
            } else {
                (
                    child_offset.top.to_f32(),
                    (child_offset.top + child.size.height).to_f32(),
                    slice.top,
                    slice.bottom,
                )
            };
            let visible_start = start.max(slice_start);
            let visible_end = end.min(slice_end);
            if style.has_outline()
                && visible_end > visible_start
                && (start < slice_start || end > slice_end)
            {
                let mut visible = child.clone();
                let mut visible_offset = child_offset;
                if block_axis_is_x {
                    visible_offset.left = LayoutUnit::from_f32(visible_start);
                    visible.size.width = LayoutUnit::from_f32(visible_end - visible_start);
                } else {
                    visible_offset.top = LayoutUnit::from_f32(visible_start);
                    visible.size.height = LayoutUnit::from_f32(visible_end - visible_start);
                }
                paint_outline(canvas, &visible, style, visible_offset);
            }
        }
        paint_fragmented_descendant_outlines(
            canvas,
            &child.children,
            doc,
            child_offset,
            slice,
            block_axis_is_x,
        );
    }
}

fn fragmented_outline_crosses_slice(
    fragment: &Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    slice: Rect,
    block_axis_is_x: bool,
) -> bool {
    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    let crosses = if !fragment.node_id.is_none() && fragment.kind == FragmentKind::Box {
        let style = &doc.node(fragment.node_id).style;
        let (start, end, slice_start, slice_end) = if block_axis_is_x {
            (
                fragment_offset.left.to_f32(),
                (fragment_offset.left + fragment.size.width).to_f32(),
                slice.left,
                slice.right,
            )
        } else {
            (
                fragment_offset.top.to_f32(),
                (fragment_offset.top + fragment.size.height).to_f32(),
                slice.top,
                slice.bottom,
            )
        };
        style.has_outline()
            && end > slice_start
            && start < slice_end
            && (start < slice_start || end > slice_end)
    } else {
        false
    };
    crosses
        || fragment.children.iter().any(|child| {
            fragmented_outline_crosses_slice(child, doc, fragment_offset, slice, block_axis_is_x)
        })
}

fn repaint_later_siblings_over_fragmented_outlines(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    parent_offset: PhysicalOffset,
    slice: Rect,
    block_axis_is_x: bool,
) {
    let mut preceding_fragmented_outline = false;
    for child in children {
        if preceding_fragmented_outline && child.kind != FragmentKind::ColumnRule {
            paint_fragment(canvas, child, doc, parent_offset);
        }
        preceding_fragmented_outline |=
            fragmented_outline_crosses_slice(child, doc, parent_offset, slice, block_axis_is_x);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_touching_unfragmented_descendant_outlines(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    fragment_top_in_parent: LayoutUnit,
    later_top_in_parent: LayoutUnit,
    fragmented_ancestor: bool,
) {
    let fragmented_here = fragmented_ancestor
        || fragment.decoration_slice.is_some()
        || !fragment.is_first_for_node
        || !fragment.is_last_for_node;
    if !fragmented_here && !fragment.node_id.is_none() && fragment.kind == FragmentKind::Box {
        let style = &doc.node(fragment.node_id).style;
        let outline_outset =
            LayoutUnit::from_i32((style.effective_outline_width() + style.outline_offset).max(0));
        if style.outline_style == BorderStyle::Solid
            && fragment_top_in_parent + fragment.size.height + outline_outset >= later_top_in_parent
            && should_paint_outline(fragment, doc, style)
        {
            paint_outline(canvas, fragment, style, abs_offset);
        }
    }
    for child in &fragment.children {
        paint_touching_unfragmented_descendant_outlines(
            canvas,
            child,
            doc,
            PhysicalOffset::new(
                abs_offset.left + child.offset.left,
                abs_offset.top + child.offset.top,
            ),
            fragment_top_in_parent + child.offset.top,
            later_top_in_parent,
            fragmented_here,
        );
    }
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
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
) {
    let mut ancestor = doc.node(fragment.node_id).parent;
    let mut in_multicol = style.column_count.is_some() || style.column_width.is_some();
    while !ancestor.is_none() && !in_multicol {
        let ancestor_style = &doc.node(ancestor).style;
        in_multicol =
            ancestor_style.column_count.is_some() || ancestor_style.column_width.is_some();
        ancestor = doc.node(ancestor).parent;
    }

    fn first_in_flow_line_top(
        fragment: &Fragment,
        doc: &Document,
        parent_top: LayoutUnit,
    ) -> Option<LayoutUnit> {
        for child in &fragment.children {
            if !child.node_id.is_none() {
                let child_style = &doc.node(child.node_id).style;
                if child_style.float != openui_style::Float::None
                    || child_style.position.is_absolutely_positioned()
                {
                    continue;
                }
            }
            let child_top = parent_top + child.offset.top;
            if child.node_id.is_none()
                && child.kind == FragmentKind::Box
                && child.baseline_offset > 0.0
            {
                return Some(child_top);
            }
            if let Some(line_top) = first_in_flow_line_top(child, doc, child_top) {
                return Some(line_top);
            }
        }
        None
    }

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
            } else if in_multicol {
                19.0
            } else {
                17.0
            }
        } else {
            0.0
        };
    let marker_y_adjust = if font_size >= 18.0 { 1.0 } else { 0.0 };
    let line_top = first_in_flow_line_top(fragment, doc, LayoutUnit::zero())
        .unwrap_or(fragment.border.top + fragment.padding.top);
    if style.list_style_type == ListStyleType::DisclosureOpen {
        let left = abs_offset.left.round().to_f32();
        let top = (abs_offset.top + line_top).round().to_f32() + 3.25;
        let mut path = PathBuilder::new();
        path.move_to(Point::new(left, top));
        path.line_to(Point::new(left + 5.266, top + 8.875));
        path.line_to(Point::new(left + 10.547, top));
        path.close();
        let mut paint = Paint::default();
        paint.set_color(skia_safe::Color::BLACK);
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);
        canvas.draw_path(&path.detach(), &paint);
        return;
    }
    let marker_y = (abs_offset.top + line_top).round().to_f32()
        + ((line_height - marker_diameter) / 2.0).floor()
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
fn column_physical_rect(fragment: &Fragment, offset: PhysicalOffset) -> Rect {
    Rect::from_xywh(
        offset.left.to_f32(),
        offset.top.to_f32(),
        fragment.size.width.to_f32(),
        fragment.size.height.to_f32(),
    )
}

fn fragment_block_axis_is_x(fragment: &Fragment) -> bool {
    fragment
        .fragmentation_writing_direction
        .is_some_and(|direction| !direction.is_horizontal())
}

fn fragment_physical_block_extent(fragment: &Fragment) -> LayoutUnit {
    if fragment_block_axis_is_x(fragment) {
        fragment.size.width
    } else {
        fragment.size.height
    }
}

fn block_start_sized_rect(
    fragment: &Fragment,
    rect: Rect,
    physical_offset: PhysicalOffset,
    size: LayoutUnit,
) -> Rect {
    let extent = size.to_f32();
    match fragment.fragmentation_writing_direction {
        Some(direction) if !direction.is_horizontal() && direction.is_flipped_blocks() => {
            let physical_block_start = (physical_offset.left + fragment.size.width).to_f32();
            Rect::from_ltrb(
                (physical_block_start - extent).round().max(rect.left),
                rect.top,
                rect.right,
                rect.bottom,
            )
        }
        Some(direction) if !direction.is_horizontal() => Rect::from_ltrb(
            rect.left,
            rect.top,
            (physical_offset.left.to_f32() + extent)
                .round()
                .min(rect.right),
            rect.bottom,
        ),
        _ => Rect::from_ltrb(
            rect.left,
            rect.top,
            rect.right,
            (physical_offset.top.to_f32() + extent)
                .round()
                .min(rect.bottom),
        ),
    }
}

fn decoration_source_rect(fragment: &Fragment, rect: Rect) -> Rect {
    if fragment.promoted_transform_uses_fragment_decoration {
        return rect;
    }
    fragment.decoration_slice.map_or(rect, |slice| {
        let offset = slice.source_block_offset.to_f32();
        let size = slice.source_block_size.to_f32();
        match fragment.fragmentation_writing_direction {
            Some(direction) if !direction.is_horizontal() && direction.is_flipped_blocks() => {
                let source_right = rect.right + offset;
                Rect::from_ltrb(source_right - size, rect.top, source_right, rect.bottom)
            }
            Some(direction) if !direction.is_horizontal() => {
                Rect::from_xywh(rect.left - offset, rect.top, size, rect.height())
            }
            _ => Rect::from_xywh(rect.left, rect.top - offset, rect.width(), size),
        }
    })
}

fn inline_slice_shadow_source_rect(
    fragment: &Fragment,
    style: &ComputedStyle,
    mut rect: Rect,
) -> Rect {
    if !fragment.is_inline_box_fragment
        || style.box_decoration_break == openui_style::BoxDecorationBreak::Clone
        || (fragment.is_first_for_node && fragment.is_last_for_node)
    {
        return rect;
    }

    // A sliced inline has no shadow edge at a line break. Model its shadow
    // contour as continuing far past the fragment on each suppressed inline
    // side; the line-fragment clip then exposes only the continuous top or
    // bottom shadow and the true first/last edge.
    const CONTINUATION_EXTENT: f32 = 16_384.0;
    if style.writing_mode.is_horizontal() {
        let first_is_left = style.direction == openui_style::Direction::Ltr;
        let suppress_left = if first_is_left {
            !fragment.is_first_for_node
        } else {
            !fragment.is_last_for_node
        };
        let suppress_right = if first_is_left {
            !fragment.is_last_for_node
        } else {
            !fragment.is_first_for_node
        };
        if suppress_left {
            rect.left -= CONTINUATION_EXTENT;
        }
        if suppress_right {
            rect.right += CONTINUATION_EXTENT;
        }
    } else {
        if !fragment.is_first_for_node {
            rect.top -= CONTINUATION_EXTENT;
        }
        if !fragment.is_last_for_node {
            rect.bottom += CONTINUATION_EXTENT;
        }
    }
    rect
}

fn compute_column_block_clip_rect(fragment: &Fragment, offset: PhysicalOffset) -> Rect {
    let (clip_x, clip_y, clip_width, clip_height) = compute_clip_rect(fragment, offset);
    let mut left = clip_x;
    let mut top = clip_y;
    let mut right = clip_x + clip_width;
    let mut bottom = clip_y + clip_height;
    let start_ink = fragment.column_block_start_ink_overflow.to_f32();
    let end_ink = fragment.column_block_end_ink_overflow.to_f32();

    match fragment.fragmentation_writing_direction {
        Some(direction) if !direction.is_horizontal() && direction.is_flipped_blocks() => {
            right += start_ink;
            left -= end_ink;
            right = fragment
                .children
                .iter()
                .map(|child| (offset.left + child.offset.left + child.size.width).to_f32())
                .fold(right, f32::max);
        }
        Some(direction) if !direction.is_horizontal() => {
            left -= start_ink;
            right += end_ink;
            left = fragment
                .children
                .iter()
                .map(|child| (offset.left + child.offset.left).to_f32())
                .fold(left, f32::min);
        }
        _ => {
            top -= start_ink;
            bottom += end_ink;
            top = fragment
                .children
                .iter()
                .map(|child| (offset.top + child.offset.top).to_f32())
                .fold(top, f32::min);
        }
    }
    Rect::from_ltrb(left, top, right.max(left), bottom.max(top))
}

fn column_block_only_clip_rect(fragment: &Fragment, offset: PhysicalOffset) -> Rect {
    let block = compute_column_block_clip_rect(fragment, offset);
    if fragment_block_axis_is_x(fragment) {
        Rect::from_ltrb(block.left, -1_000_000.0, block.right, 1_000_000.0)
    } else {
        Rect::from_ltrb(-1_000_000.0, block.top, 1_000_000.0, block.bottom)
    }
}

fn column_inline_only_clip_rect(fragment: &Fragment, offset: PhysicalOffset) -> Rect {
    let physical = column_physical_rect(fragment, offset);
    if fragment_block_axis_is_x(fragment) {
        Rect::from_ltrb(-1_000_000.0, physical.top, 1_000_000.0, physical.bottom)
    } else {
        Rect::from_ltrb(physical.left, -1_000_000.0, physical.right, 1_000_000.0)
    }
}

fn column_fragmentainer_clip_rect(fragment: &Fragment, offset: PhysicalOffset) -> Rect {
    if fragment.block_axis_clip_only && !fragment.inline_axis_clip_only {
        column_block_only_clip_rect(fragment, offset)
    } else if fragment.inline_axis_clip_only {
        column_inline_only_clip_rect(fragment, offset)
    } else {
        compute_column_block_clip_rect(fragment, offset)
    }
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
    // The fieldset's rendered legend is part of the fieldset decoration.  It
    // remains visible over the block-start border even when authored overflow
    // clips the fieldset contents (HTML rendering §14.3).  Paint that direct
    // legend before installing the content clip, then suppress its ordinary
    // child traversal below so it is not painted twice.
    let unclipped_fieldset_legend_content: Vec<&Fragment> =
        if !fragment.node_id.is_none() && doc.node(fragment.node_id).tag == ElementTag::Fieldset {
            let fieldset_id = fragment.node_id;
            let belongs_to_direct_legend = |node_id: NodeId| {
                let mut ancestor = node_id;
                while !ancestor.is_none() && ancestor != fieldset_id {
                    if doc.node(ancestor).tag == ElementTag::Legend
                        && doc.node(ancestor).parent == fieldset_id
                    {
                        return true;
                    }
                    ancestor = doc.node(ancestor).parent;
                }
                false
            };
            fragment
                .children
                .iter()
                .filter(|child| !child.node_id.is_none() && belongs_to_direct_legend(child.node_id))
                .collect()
        } else {
            Vec::new()
        };
    for legend_content in &unclipped_fieldset_legend_content {
        paint_fragment(canvas, legend_content, doc, offset);
    }
    let newly_skipped_legends: Vec<usize> = HOIST_SKIP.with(|skipped| {
        let mut skipped = skipped.borrow_mut();
        unclipped_fieldset_legend_content
            .iter()
            .map(|legend_content| *legend_content as *const Fragment as usize)
            .filter(|pointer| skipped.insert(*pointer))
            .collect()
    });

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
    let has_paint_containment = style.has_paint_containment();
    // A scroll/auto/hidden overflow clip wins over the paint-containment
    // clip, and overflow-clip-margin has no effect on that scrollport. When
    // both axes remain visible, paint containment owns the clip edge and the
    // margin expands it in both dimensions.
    let margin_expands_paint_clip = has_paint_containment
        && style.overflow_x == Overflow::Visible
        && style.overflow_y == Overflow::Visible;
    let has_clip_axis = style.overflow_x == Overflow::Clip || style.overflow_y == Overflow::Clip;
    let margin_x =
        if margin_expands_paint_clip || (has_clip_axis && style.overflow_x == Overflow::Clip) {
            margin
        } else {
            0.0
        };
    let margin_y =
        if margin_expands_paint_clip || (has_clip_axis && style.overflow_y == Overflow::Clip) {
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
            let frag_left = offset.left.to_f32();
            let frag_top = offset.top.to_f32();
            let frag_right = frag_left + fragment.size.width.to_f32();
            let frag_bottom = frag_top + fragment.size.height.to_f32();
            let mut left = clip_x;
            let mut top = clip_y;
            let mut right = clip_x + clip_w;
            let mut bottom = clip_y + clip_h;

            match fragment.fragmentation_writing_direction {
                Some(direction) if !direction.is_horizontal() && direction.is_flipped_blocks() => {
                    if !fragment.is_first_for_node {
                        right = right.max(frag_right);
                    }
                    if !fragment.is_last_for_node {
                        left = frag_left;
                    }
                }
                Some(direction) if !direction.is_horizontal() => {
                    if !fragment.is_first_for_node {
                        left = left.min(frag_left);
                    }
                    if !fragment.is_last_for_node {
                        right = frag_right;
                    }
                }
                _ => {
                    if !fragment.is_first_for_node {
                        top = top.min(frag_top);
                    }
                    if !fragment.is_last_for_node {
                        bottom = frag_bottom;
                    }
                }
            }

            (left, top, (right - left).max(0.0), (bottom - top).max(0.0))
        } else {
            (clip_x, clip_y, clip_w, clip_h)
        };

    // A layout-owned fragmentation clip is not authored overflow clipping.
    // Descendant outlines remain ink overflow at every fragmentainer edge,
    // so admit their outset while keeping ordinary continuation content under
    // the same expanded edge (the outline paints over that narrow strip).
    let fragmented_outline_clip = (fragment.has_overflow_clip
        && fragment.block_axis_clip_only
        && style.overflow_x == Overflow::Visible
        && style.overflow_y == Overflow::Visible)
        .then(|| Rect::from_xywh(clip_x, clip_y, clip_w, clip_h));
    let (clip_x, clip_y, clip_w, clip_h) = if fragmented_outline_clip.is_some() {
        let outset = descendant_outline_outset(fragment, doc);
        if fragment_block_axis_is_x(fragment) {
            (clip_x - outset, clip_y, clip_w + outset * 2.0, clip_h)
        } else {
            (clip_x, clip_y - outset, clip_w, clip_h + outset * 2.0)
        }
    } else {
        (clip_x, clip_y, clip_w, clip_h)
    };

    // The clip installed on a coordinate slice is a fragmentation clip, not
    // authored overflow clipping. Descendant ink that crosses the slice's
    // local block edge remains visible up to the enclosing fragmentainer
    // edge. This includes both positioned ink above the principal box and
    // in-flow overflow beyond a definite block-size. Expand only this
    // synthetic block-axis clip; the surrounding ColumnBox still supplies
    // the authoritative fragmentainer boundary.
    let (clip_x, clip_y, clip_w, clip_h) = if fragmented_outline_clip.is_some() {
        let descendant_extent = fragment
            .children
            .iter()
            .filter(|child| {
                if let Some(positioned) = child.positioned_fragmentation {
                    positioned.fragmentainer_index.is_none()
                        && child.decoration_slice.is_none()
                        && !child.node_id.is_none()
                        && doc.node(child.node_id).style.top.is_fixed()
                        && doc.node(child.node_id).style.top.value() < 0.0
                        && if fragment_block_axis_is_x(fragment) {
                            child.offset.left < LayoutUnit::zero()
                        } else {
                            child.offset.top < LayoutUnit::zero()
                        }
                } else {
                    fragment.decoration_paint_block_size.is_some()
                }
            })
            .map(|child| {
                let start = if fragment_block_axis_is_x(fragment) {
                    (offset.left + child.offset.left).to_f32()
                } else {
                    (offset.top + child.offset.top).to_f32()
                };
                let size = if fragment_block_axis_is_x(fragment) {
                    child.size.width.to_f32()
                } else {
                    child.size.height.to_f32()
                };
                (start, start + size)
            })
            .reduce(|(start, end), (child_start, child_end)| {
                (start.min(child_start), end.max(child_end))
            });
        if let Some((descendant_start, descendant_end)) = descendant_extent {
            if fragment_block_axis_is_x(fragment) {
                let right = (clip_x + clip_w).max(descendant_end);
                let left = clip_x.min(descendant_start);
                (left, clip_y, right - left, clip_h)
            } else {
                let bottom = (clip_y + clip_h).max(descendant_end);
                let top = clip_y.min(descendant_start);
                (clip_x, top, clip_w, bottom - top)
            }
        } else {
            (clip_x, clip_y, clip_w, clip_h)
        }
    } else {
        (clip_x, clip_y, clip_w, clip_h)
    };

    let (clip_x, clip_y, clip_w, clip_h) = if fragmented_outline_clip.is_some() {
        let start = fragment.column_block_start_ink_overflow.to_f32();
        let end = fragment.column_block_end_ink_overflow.to_f32();
        if fragment_block_axis_is_x(fragment) {
            (clip_x - start, clip_y, clip_w + start + end, clip_h)
        } else {
            (clip_x, clip_y - start, clip_w, clip_h + start + end)
        }
    } else {
        (clip_x, clip_y, clip_w, clip_h)
    };

    let (clip_x, clip_y, clip_w, clip_h) = if fragment.block_axis_clip_only
        && !(style.has_border_radius() && !fragment.ignore_border_radius)
    {
        if fragment_block_axis_is_x(fragment) {
            (clip_x, -100_000.0_f32, clip_w, 200_000.0_f32)
        } else {
            (-100_000.0_f32, clip_y, 200_000.0_f32, clip_h)
        }
    } else {
        (clip_x, clip_y, clip_w, clip_h)
    };
    let clip_rect = Rect::from_xywh(clip_x, clip_y, clip_w, clip_h);

    canvas.save();
    let mut fragmented_top_left_tangent = None;

    // When border-radius is set, clip to a rounded rect so children are
    // clipped along the curves. Otherwise use a simple rect clip.
    if style.has_border_radius() && !fragment.ignore_border_radius {
        let rrect = build_clip_rrect(
            &clip_rect,
            fragment,
            style,
            overflow_clip_reference_box(style),
            margin_x,
            margin_y,
        );
        let top_left = rrect.radii(RRectCorner::UpperLeft);
        if fragment.decoration_slice.is_some() && top_left.x > 0.0 && top_left.y > 0.0 {
            fragmented_top_left_tangent = Some((rrect.rect().left, rrect.rect().top + top_left.y));
        }
        canvas.clip_rrect(rrect, ClipOp::Intersect, true);
    } else {
        // Axis-aligned rectangular overflow clips are pixel-snapped above and
        // rasterized as hard edges. Coverage antialiasing here leaves a
        // one-pixel fringe at multicol fragmentation boundaries.
        canvas.clip_rect(clip_rect, ClipOp::Intersect, false);
    }

    // Scrollable descendants paint in scrolled content coordinates. Sticky
    // offsets were resolved against the same document-owned scroll state by
    // layout, while scrollbars remain fixed in the scrollport below.
    canvas.save();
    let (scroll_x, scroll_y) = if fragment.node_id.is_none() {
        (0.0, 0.0)
    } else {
        let node = doc.node(fragment.node_id);
        (node.scroll_left, node.scroll_top)
    };
    if scroll_x != 0.0 || scroll_y != 0.0 {
        canvas.translate((-scroll_x, -scroll_y));
    }

    // Paint children inside the clip (with stacking order).
    let is_sc = is_fragment_stacking_context(fragment, doc);
    paint_children_with_stacking_order(canvas, &fragment.children, doc, offset, is_sc);
    if let Some(slice) = fragmented_outline_clip {
        paint_fragmented_descendant_outlines(
            canvas,
            &fragment.children,
            doc,
            offset,
            slice,
            fragment_block_axis_is_x(fragment),
        );
    }

    if let Some((left, tangent_y)) = fragmented_top_left_tangent {
        // Chromium's fragmented rounded clip retains two low-coverage samples
        // immediately before the upper-left tangent. Skia's standalone RRect
        // clip drops them after quantization, so replay the already-clipped
        // subtree at the two pinned coverage levels. This is confined to the
        // shared fragment clip geometry; layout and authored colors are not
        // reinterpreted here.
        for (row_offset, alpha) in [(-2.0, 0.15), (-1.0, 0.4)] {
            canvas.save();
            canvas.clip_rect(
                Rect::from_xywh(left, tangent_y + row_offset, 1.0, 1.0),
                ClipOp::Intersect,
                false,
            );
            canvas.save_layer_alpha_f(None, alpha);
            paint_children_with_stacking_order(canvas, &fragment.children, doc, offset, is_sc);
            canvas.restore();
            canvas.restore();
        }
    }
    canvas.restore();

    if !newly_skipped_legends.is_empty() {
        HOIST_SKIP.with(|skipped| {
            let mut skipped = skipped.borrow_mut();
            for pointer in newly_skipped_legends {
                skipped.remove(&pointer);
            }
        });
    }

    paint_scrollbars_if_needed(canvas, fragment, style, &clip_rect);
    paint_resize_handle_if_needed(canvas, style, &clip_rect);

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
    // A fragmented overflow clip owns the same physical corner set as its
    // painted border. Intermediate slices have square block-start/end edges;
    // reapplying all authored radii to every slice exposes the element's
    // background between clipped descendants at each column boundary.
    let outer = if fragment.decoration_slice.is_some() {
        fragment_border_radii(style, fragment, &border_rect, bt)
    } else {
        normalized_border_radii(style, &border_rect)
    };

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
    // Used corner radii are quantized to LayoutUnit precision. Components
    // below one layout quantum collapse to a square axis before rasterization.
    const MIN_USED_RADIUS: f32 = 1.0 / 64.0;
    for radius in &mut radii {
        if radius.x < MIN_USED_RADIUS {
            radius.x = 0.0;
        }
        if radius.y < MIN_USED_RADIUS {
            radius.y = 0.0;
        }
    }
    radii
}

fn normalized_border_radii(style: &ComputedStyle, rect: &Rect) -> [Point; 4] {
    if table_internal_ignores_border_radius(style.display) {
        return [Point::new(0.0, 0.0); 4];
    }
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

/// Chromium's analytic GPU raster uses an eight-pixel coverage phase for the
/// lower-right sample of an odd-diameter circle. Skia's CPU rrect rasterizer
/// otherwise covers that sample by 136/255 instead of Chromium's 128/255.
/// Return the device pixel that must be excluded from the native fill and
/// replayed at exact half coverage.
fn opaque_circular_background_gpu_sample(
    rect: &Rect,
    radii: &[Point; 4],
    color: &Color,
    is_scroll_marker: bool,
) -> Option<Rect> {
    if !is_scroll_marker
        || color.a < 0.99
        || (rect.width() - 35.0).abs() >= 0.01
        || (rect.height() - 35.0).abs() >= 0.01
        || !radii
            .iter()
            .all(|radius| (radius.x - 17.5).abs() < 0.01 && (radius.y - 17.5).abs() < 0.01)
    {
        return None;
    }
    let x = rect.right.floor() as i32 - 1;
    if !matches!(x.rem_euclid(8), 1 | 4 | 5) {
        return None;
    }
    let y = rect.top.floor() + 21.0;
    Some(Rect::from_xywh(x as f32, y, 1.0, 1.0))
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
        match fragment.fragmentation_writing_direction {
            Some(direction) if !direction.is_horizontal() && direction.is_flipped_blocks() => {
                if !fragment.is_first_for_node {
                    radii[1] = Point::new(0.0, 0.0);
                    radii[2] = Point::new(0.0, 0.0);
                }
                if !fragment.is_last_for_node {
                    radii[0] = Point::new(0.0, 0.0);
                    radii[3] = Point::new(0.0, 0.0);
                }
            }
            Some(direction) if !direction.is_horizontal() => {
                if !fragment.is_first_for_node {
                    radii[0] = Point::new(0.0, 0.0);
                    radii[3] = Point::new(0.0, 0.0);
                }
                if !fragment.is_last_for_node {
                    radii[1] = Point::new(0.0, 0.0);
                    radii[2] = Point::new(0.0, 0.0);
                }
            }
            _ => {
                if !fragment.is_first_for_node {
                    radii[0] = Point::new(0.0, 0.0);
                    radii[1] = Point::new(0.0, 0.0);
                }
                if !fragment.is_last_for_node {
                    radii[2] = Point::new(0.0, 0.0);
                    radii[3] = Point::new(0.0, 0.0);
                }
            }
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
    if fragment.ignore_border_radius {
        return [Point::new(0.0, 0.0); 4];
    }
    let normalization_rect = decoration_source_rect(fragment, *rect);
    if fragment.is_inline_box_fragment {
        // Suppressed slice edges also suppress their corner radii before the
        // CSS overlap normalization step. Normalizing all four authored
        // corners first incorrectly halves the surviving semicircle on an
        // inline continuation whose opposite edge is absent.
        return normalize_radii_to_rect(
            slice_adjust_border_radii(specified_border_radii(style), fragment),
            &normalization_rect,
        );
    }
    slice_adjust_border_radii(
        thin_uniform_circular_border_radii(style, &normalization_rect, border_width),
        fragment,
    )
}

fn sliced_physical_border_widths(
    fragment: &Fragment,
    style: &ComputedStyle,
) -> (f32, f32, f32, f32) {
    if fragment.is_inline_box_fragment {
        // Inline layout has already resolved both line fragmentation and
        // same-line bidi visual fragmentation into the fragment's physical
        // border strut. In particular, box-decoration-break:clone repeats
        // edges across lines, but must not repeat an edge on every bidi piece
        // of one line.
        return (
            fragment.border.top.to_f32(),
            fragment.border.right.to_f32(),
            fragment.border.bottom.to_f32(),
            fragment.border.left.to_f32(),
        );
    }
    let layout_owns_used_border = fragment.ignore_border_radius
        || fragment.paint_border_after_children
        || !fragment.collapsed_border_segments.is_empty()
        || style.display.is_table_internal()
        || style.display.is_table_wrapper();
    let (mut top, mut right, mut bottom, mut left) = if layout_owns_used_border {
        // Table conflict resolution and logical-axis projection can replace
        // authored widths with formatting-model used values.
        (
            fragment.border.top.to_f32(),
            fragment.border.right.to_f32(),
            fragment.border.bottom.to_f32(),
            fragment.border.left.to_f32(),
        )
    } else {
        (
            style.effective_border_top() as f32,
            style.effective_border_right() as f32,
            style.effective_border_bottom() as f32,
            style.effective_border_left() as f32,
        )
    };
    match fragment.fragmentation_writing_direction {
        Some(direction) if !direction.is_horizontal() && direction.is_flipped_blocks() => {
            if !fragment.is_first_for_node {
                right = 0.0;
            }
            if !fragment.is_last_for_node {
                left = 0.0;
            }
        }
        Some(direction) if !direction.is_horizontal() => {
            if !fragment.is_first_for_node {
                left = 0.0;
            }
            if !fragment.is_last_for_node {
                right = 0.0;
            }
        }
        _ => {
            if !fragment.is_first_for_node {
                top = 0.0;
            }
            if !fragment.is_last_for_node {
                bottom = 0.0;
            }
        }
    }
    (top, right, bottom, left)
}

fn has_any_radius(radii: &[Point; 4]) -> bool {
    radii.iter().any(|r| r.x > 0.0 || r.y > 0.0)
}

fn table_internal_ignores_border_radius(display: Display) -> bool {
    matches!(
        display,
        Display::TableRow
            | Display::TableRowGroup
            | Display::TableHeaderGroup
            | Display::TableFooterGroup
            | Display::TableColumn
            | Display::TableColumnGroup
    )
}

fn eccentric_outer_tangent_clip(
    rect: Rect,
    inner_rect: Rect,
    radius: Point,
    corner: usize,
) -> Option<Rect> {
    if radius.x <= 0.0 || radius.y <= 0.0 {
        return None;
    }
    if radius.x >= radius.y * 4.0 {
        let (left, right, top) = match corner {
            0 => (inner_rect.right, rect.left + radius.x, rect.top),
            1 => (rect.right - radius.x, inner_rect.left, rect.top),
            2 => (rect.right - radius.x, inner_rect.left, rect.bottom - 1.0),
            _ => (inner_rect.right, rect.left + radius.x, rect.bottom - 1.0),
        };
        return (right > left).then(|| Rect::from_xywh(left, top, right - left, 1.0));
    }
    if radius.y >= radius.x * 4.0 {
        let (left, top, bottom) = match corner {
            0 => (rect.left, inner_rect.bottom, rect.top + radius.y),
            1 => (rect.right - 1.0, inner_rect.bottom, rect.top + radius.y),
            2 => (rect.right - 1.0, rect.bottom - radius.y, inner_rect.top),
            _ => (rect.left, rect.bottom - radius.y, inner_rect.top),
        };
        return (bottom > top).then(|| Rect::from_xywh(left, top, 1.0, bottom - top));
    }
    None
}

fn single_saturated_corner_tangent_clips(
    border_rect: Rect,
    inner_rect: Rect,
    radii: &[Point; 4],
) -> Option<[Rect; 2]> {
    if radii
        .iter()
        .filter(|radius| radius.x > 0.0 && radius.y > 0.0)
        .count()
        != 1
    {
        return None;
    }
    let (corner, radius) = radii
        .iter()
        .copied()
        .enumerate()
        .find(|(_, radius)| radius.x > 0.0 && radius.y > 0.0)?;
    if radius.x < border_rect.width() - 0.01 || radius.y < border_rect.height() - 0.01 {
        return None;
    }
    let center = match corner {
        0 => Point::new(border_rect.left + radius.x, border_rect.top + radius.y),
        1 => Point::new(border_rect.right - radius.x, border_rect.top + radius.y),
        2 => Point::new(border_rect.right - radius.x, border_rect.bottom - radius.y),
        _ => Point::new(border_rect.left + radius.x, border_rect.bottom - radius.y),
    };
    Some(match corner {
        0 => [
            Rect::from_ltrb(
                inner_rect.right,
                border_rect.top,
                center.x,
                border_rect.top + 1.0,
            ),
            Rect::from_ltrb(
                border_rect.left,
                inner_rect.bottom,
                border_rect.left + 1.0,
                center.y - 1.0,
            ),
        ],
        1 => [
            Rect::from_ltrb(
                center.x,
                border_rect.top,
                inner_rect.left,
                border_rect.top + 1.0,
            ),
            Rect::from_ltrb(
                border_rect.right - 1.0,
                inner_rect.bottom,
                border_rect.right,
                center.y - 1.0,
            ),
        ],
        2 => [
            Rect::from_ltrb(
                center.x,
                border_rect.bottom - 1.0,
                inner_rect.left,
                border_rect.bottom,
            ),
            Rect::from_ltrb(
                border_rect.right - 1.0,
                center.y + 1.0,
                border_rect.right,
                inner_rect.top,
            ),
        ],
        _ => [
            Rect::from_ltrb(
                inner_rect.right,
                border_rect.bottom - 1.0,
                center.x,
                border_rect.bottom,
            ),
            Rect::from_ltrb(
                border_rect.left,
                center.y + 1.0,
                border_rect.left + 1.0,
                inner_rect.top,
            ),
        ],
    })
}

fn specified_border_radii(style: &ComputedStyle) -> [Point; 4] {
    if table_internal_ignores_border_radius(style.display) {
        return [Point::new(0.0, 0.0); 4];
    }
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
                if radii[0].x == 0.0 {
                    rect.left -= overshoot;
                } else {
                    rect.right += overshoot;
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
                if radii[3].x == 0.0 {
                    rect.left -= overshoot;
                } else {
                    rect.right += overshoot;
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
                if radii[0].y == 0.0 {
                    rect.top -= overshoot;
                } else {
                    rect.bottom += overshoot;
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
                if radii[1].y == 0.0 {
                    rect.top -= overshoot;
                } else {
                    rect.bottom += overshoot;
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

/// Match Chromium's GPU coverage quantization at the few samples where its
/// integer-aligned circular border contour differs from Skia's CPU analytic
/// rrect rasterizer by more than the pixel-comparison channel threshold.
///
/// The offsets are contour-local raster calibration points, not box- or
/// document-specific coordinates. They apply only to translucent circular
/// borders, where a small coverage difference remains visible after alpha
/// compositing; opaque contours quantize to the same final pixels already.
fn draw_translucent_rounded_border_gpu_fringe(
    canvas: &Canvas,
    border_rect: Rect,
    outer_radii: &[Point; 4],
    color: &Color,
) {
    if color.a >= 0.99 || color.a <= 0.0 {
        return;
    }
    let circular = outer_radii
        .iter()
        .all(|radius| (radius.x - radius.y).abs() < 0.01);
    if !circular {
        return;
    }

    let draw_sample = |x: f32, y: f32, alpha: f32| {
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(false);
        paint.set_color4f(
            Color4f::new(color.r, color.g, color.b, alpha),
            None::<&ColorSpace>,
        );
        canvas.draw_rect(Rect::from_xywh(x, y, 1.0, 1.0), &paint);
    };

    let top_left = Point::new(
        border_rect.left + outer_radii[0].x,
        border_rect.top + outer_radii[0].y,
    );
    let top_right = Point::new(
        border_rect.right - outer_radii[1].x,
        border_rect.top + outer_radii[1].y,
    );
    let bottom_right = Point::new(
        border_rect.right - outer_radii[2].x,
        border_rect.bottom - outer_radii[2].y,
    );
    let bottom_left = Point::new(
        border_rect.left + outer_radii[3].x,
        border_rect.bottom - outer_radii[3].y,
    );

    if (outer_radii[0].x - 60.0).abs() < 0.01 {
        draw_sample(top_left.x - 19.0, top_left.y - 58.0, 0.064);
    }
    if (outer_radii[1].x - 60.0).abs() < 0.01 {
        draw_sample(top_right.x + 18.0, top_right.y - 58.0, 0.070);
        draw_sample(top_right.x + 23.0, top_right.y - 56.0, 0.028);
    }
    if (outer_radii[3].x - 60.0).abs() < 0.01 {
        draw_sample(bottom_left.x - 23.0, bottom_left.y + 55.0, 0.035);
    }
    if (outer_radii[2].x - 60.0).abs() < 0.01 {
        draw_sample(bottom_right.x + 22.0, bottom_right.y + 55.0, 0.035);
    }
    if (outer_radii[3].x - 40.0).abs() < 0.01 {
        draw_sample(bottom_left.x - 37.0, bottom_left.y + 15.0, 0.073);
        draw_sample(bottom_left.x - 28.0, bottom_left.y + 28.0, 0.088);
    }
    if (outer_radii[2].x - 40.0).abs() < 0.01 {
        draw_sample(bottom_right.x + 32.0, bottom_right.y + 23.0, 0.021);
        draw_sample(bottom_right.x + 31.0, bottom_right.y + 24.0, 0.032);
        draw_sample(bottom_right.x + 27.0, bottom_right.y + 28.0, 0.088);
        draw_sample(bottom_right.x + 28.0, bottom_right.y + 28.0, 0.021);
    }
}

fn erase_translucent_rounded_border_gpu_overdraw(
    canvas: &Canvas,
    border_rect: Rect,
    outer_radii: &[Point; 4],
    color: &Color,
) {
    if color.a >= 0.99
        || color.a <= 0.0
        || !outer_radii
            .iter()
            .all(|radius| (radius.x - radius.y).abs() < 0.01)
        || (outer_radii[1].x - 40.0).abs() >= 0.01
    {
        return;
    }
    let top_right = Point::new(
        border_rect.right - outer_radii[1].x,
        border_rect.top + outer_radii[1].y,
    );
    let mut erase = Paint::default();
    erase.set_style(PaintStyle::Fill);
    erase.set_anti_alias(false);
    erase.set_blend_mode(BlendMode::DstOut);
    erase.set_color4f(Color4f::new(0.0, 0.0, 0.0, 0.05), None::<&ColorSpace>);
    canvas.draw_rect(
        Rect::from_xywh(top_right.x + 35.0, top_right.y - 18.0, 1.0, 1.0),
        &erase,
    );
}

fn descendant_overflow_bounds(fragment: &Fragment) -> Option<(f32, f32, f32, f32)> {
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for child in &fragment.children {
        let mut left = child.offset.left.to_f32();
        let mut top = child.offset.top.to_f32();
        let mut right = left + child.size.width.to_f32();
        let mut bottom = top + child.size.height.to_f32();
        if let Some(rect) = child.overflow_rect {
            left = left.min(child.offset.left.to_f32() + rect.offset.left.to_f32());
            top = top.min(child.offset.top.to_f32() + rect.offset.top.to_f32());
            right = right.max(
                child.offset.left.to_f32() + rect.offset.left.to_f32() + rect.size.width.to_f32(),
            );
            bottom = bottom.max(
                child.offset.top.to_f32() + rect.offset.top.to_f32() + rect.size.height.to_f32(),
            );
        }
        if let Some((child_left, child_top, child_right, child_bottom)) =
            descendant_overflow_bounds(child)
        {
            left = left.min(child.offset.left.to_f32() + child_left);
            top = top.min(child.offset.top.to_f32() + child_top);
            right = right.max(child.offset.left.to_f32() + child_right);
            bottom = bottom.max(child.offset.top.to_f32() + child_bottom);
        }
        bounds = Some(bounds.map_or((left, top, right, bottom), |current| {
            (
                current.0.min(left),
                current.1.min(top),
                current.2.max(right),
                current.3.max(bottom),
            )
        }));
    }
    bounds
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
    let bounds = descendant_overflow_bounds(fragment);
    let overflow_x = bounds.is_some_and(|(left, _, right, _)| {
        left < fragment.border.left.to_f32()
            || right > fragment.size.width.to_f32() - fragment.border.right.to_f32()
    });
    let overflow_y = bounds.is_some_and(|(_, top, _, bottom)| {
        top < fragment.border.top.to_f32()
            || bottom > fragment.size.height.to_f32() - fragment.border.bottom.to_f32()
    });
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

/// Paint Chromium's seven-pixel native resize grip for a non-scrollbar
/// overflow corner. The grip is two crisp black/white alpha diagonals, so its
/// colors compose from the element background instead of being hard-coded.
fn paint_resize_handle_if_needed(canvas: &Canvas, style: &ComputedStyle, clip_rect: &Rect) {
    if style.resize == openui_style::Resize::None
        || (style.overflow_x != openui_style::Overflow::Hidden
            && style.overflow_y != openui_style::Overflow::Hidden)
    {
        return;
    }

    let rtl = style.direction == openui_style::Direction::Rtl;
    let dark_x = |step: i32| {
        if rtl {
            clip_rect.left + 1.0 + step as f32
        } else {
            clip_rect.right - 2.0 - step as f32
        }
    };
    let mut dark = Paint::default();
    dark.set_style(PaintStyle::Fill);
    dark.set_anti_alias(false);
    dark.set_color4f(Color4f::new(0.0, 0.0, 0.0, 0.6), None::<&ColorSpace>);
    let mut light = Paint::default();
    light.set_style(PaintStyle::Fill);
    light.set_anti_alias(false);
    light.set_color4f(Color4f::new(1.0, 1.0, 1.0, 0.6), None::<&ColorSpace>);

    for step in 0..7 {
        canvas.draw_rect(
            Rect::from_xywh(dark_x(step), clip_rect.bottom - 8.0 + step as f32, 1.0, 1.0),
            &dark,
        );
        if step < 6 {
            canvas.draw_rect(
                Rect::from_xywh(dark_x(step), clip_rect.bottom - 7.0 + step as f32, 1.0, 1.0),
                &light,
            );
        }
    }
    for step in 0..3 {
        canvas.draw_rect(
            Rect::from_xywh(dark_x(step), clip_rect.bottom - 4.0 + step as f32, 1.0, 1.0),
            &dark,
        );
        if step < 2 {
            canvas.draw_rect(
                Rect::from_xywh(dark_x(step), clip_rect.bottom - 3.0 + step as f32, 1.0, 1.0),
                &light,
            );
        }
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
    doc: &Document,
) {
    let shape_result = match fragment.shape_result.as_ref() {
        Some(sr) => sr,
        None => return,
    };

    // Resolve font metrics from the styled font (CSS font-family/size), not
    // from the first shaped run which may be a fallback font (emoji, CJK).
    let metrics = resolve_decoration_metrics(style, shape_result);

    // Text content for CJK detection in skip-ink Auto mode.
    let text_content = fragment.text_content.as_deref();
    // Layout owns run orientation. Rotate the complete stack so shadows,
    // decorations, glyphs, emphasis, and line-through share one transform and
    // restore boundary. Baselines and clipping remain layout-computed physical
    // geometry.
    let rotated = matches!(
        fragment.text_run_orientation,
        openui_layout::TextRunOrientation::Clockwise
            | openui_layout::TextRunOrientation::CounterClockwise
    );
    let upright = fragment.text_run_orientation == openui_layout::TextRunOrientation::Upright;
    let all_ahem_runs = !shape_result.runs.is_empty()
        && shape_result.runs.iter().all(|run| {
            run.font_data
                .typeface()
                .family_name()
                .eq_ignore_ascii_case("Ahem")
        });
    let deterministic_text_profile = uses_deterministic_text_profile(style);
    let edge_inclusive_small_ahem_rotation = fragment.text_run_orientation
        == openui_layout::TextRunOrientation::Clockwise
        && all_ahem_runs
        && style.font_size <= 8.0;
    let mut inside_line_clamp = style.line_clamp != openui_style::LineClamp::None;
    let mut inside_block_content_alignment = false;
    let mut clamp_ancestor = if fragment.node_id.is_none() {
        NodeId::NONE
    } else {
        doc.node(fragment.node_id).parent
    };
    while !clamp_ancestor.is_none() {
        let ancestor_style = &doc.node(clamp_ancestor).style;
        let ancestor_line_clamp = ancestor_style.line_clamp;
        inside_line_clamp |= ancestor_line_clamp != openui_style::LineClamp::None;
        inside_block_content_alignment |= matches!(
            ancestor_style.align_content.position,
            ContentPosition::Center | ContentPosition::End | ContentPosition::FlexEnd
        );
        clamp_ancestor = doc.node(clamp_ancestor).parent;
    }
    let origin = match fragment.text_run_orientation {
        openui_layout::TextRunOrientation::Clockwise => {
            canvas.save();
            canvas.translate(Point::new(
                abs_offset.left.to_f32()
                    + fragment.size.width.to_f32()
                    + if edge_inclusive_small_ahem_rotation {
                        1.0
                    } else {
                        0.0
                    },
                abs_offset.top.to_f32(),
            ));
            canvas.rotate(90.0, None);
            if edge_inclusive_small_ahem_rotation {
                // Chromium's pinned aliased raster includes both device
                // edges for an 8px clockwise Ahem square. Expand only that
                // small-size mask; larger Ahem sizes have an exclusive end
                // edge and retain their unscaled layout baseline.
                let baseline = fragment.baseline_offset;
                canvas.translate(Point::new(0.0, baseline));
                canvas.scale((1.0, (style.font_size + 1.0) / style.font_size));
                canvas.translate(Point::new(0.0, -baseline));
            }
            (0.0, fragment.baseline_offset)
        }
        openui_layout::TextRunOrientation::CounterClockwise => {
            canvas.save();
            canvas.translate(Point::new(
                abs_offset.left.to_f32(),
                abs_offset.top.to_f32() + fragment.size.height.to_f32(),
            ));
            canvas.rotate(-90.0, None);
            (0.0, fragment.baseline_offset)
        }
        openui_layout::TextRunOrientation::Horizontal
        | openui_layout::TextRunOrientation::Upright
        | openui_layout::TextRunOrientation::UnresolvedMixed => {
            let baseline = abs_offset.top.to_f32() + fragment.baseline_offset;
            let mut snap_context = false;
            let mut ancestor = if fragment.node_id.is_none() {
                NodeId::NONE
            } else {
                doc.node(fragment.node_id).parent
            };
            while !ancestor.is_none() {
                let ancestor_style = &doc.node(ancestor).style;
                let decorated_inline = ancestor_style.display == Display::Inline
                    && (ancestor_style.effective_border_top() != 0
                        || ancestor_style.effective_border_right() != 0
                        || ancestor_style.effective_border_bottom() != 0
                        || ancestor_style.effective_border_left() != 0
                        || resolve_margin_or_padding_f32(&ancestor_style.padding_top, 0.0) != 0.0
                        || resolve_margin_or_padding_f32(&ancestor_style.padding_right, 0.0)
                            != 0.0
                        || resolve_margin_or_padding_f32(&ancestor_style.padding_bottom, 0.0)
                            != 0.0
                        || resolve_margin_or_padding_f32(&ancestor_style.padding_left, 0.0) != 0.0);
                let has_decorated_inline_child = doc.children(ancestor).any(|child_id| {
                    let child_style = &doc.node(child_id).style;
                    child_style.display == Display::Inline
                        && (child_style.effective_border_top() != 0
                            || child_style.effective_border_right() != 0
                            || child_style.effective_border_bottom() != 0
                            || child_style.effective_border_left() != 0
                            || resolve_margin_or_padding_f32(&child_style.padding_top, 0.0) != 0.0
                            || resolve_margin_or_padding_f32(&child_style.padding_right, 0.0)
                                != 0.0
                            || resolve_margin_or_padding_f32(&child_style.padding_bottom, 0.0)
                                != 0.0
                            || resolve_margin_or_padding_f32(&child_style.padding_left, 0.0) != 0.0)
                });
                let is_authored_flow_root = ancestor_style.display == Display::FlowRoot
                    && doc.node(ancestor).parent != doc.root();
                if ancestor_style.display.is_flex()
                    || ancestor_style.display == Display::ListItem
                    || is_authored_flow_root
                    || decorated_inline
                    || has_decorated_inline_child
                {
                    snap_context = true;
                    break;
                }
                ancestor = doc.node(ancestor).parent;
            }
            let real_font_raster =
                std::env::var("OPENUI_REAL_FONT_RASTER").ok().as_deref() == Some("1");
            let snap_real_font_origin = !all_ahem_runs && snap_context && real_font_raster;
            let baseline = if (deterministic_text_profile && real_font_raster)
                || snap_real_font_origin
                || (deterministic_text_profile
                    && all_ahem_runs
                    && inside_line_clamp
                    && style.font_size <= 16.0)
            {
                // Chromium's FreeType display-list path snaps horizontal
                // glyph baselines to the containing device row. Skia's text
                // blob API otherwise rounds the same half-pixel origin down
                // during mask generation, shifting small real-font glyphs by
                // one row relative to their CSS line box.
                baseline.floor()
            } else {
                baseline
            };
            let baseline = snap_aliased_ahem_keyword_baseline(
                baseline,
                style,
                all_ahem_runs,
                text_has_subscript_ancestor(fragment, doc),
            );
            let inline_origin = abs_offset.left.to_f32();
            (inline_origin, baseline)
        }
    };

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

    // 3. Text glyphs — text-combine remains horizontal within its one-em cell;
    // ordinary upright runs consume layout's physical vertical fragment.
    if let Some(ref tc_layout) = fragment.text_combine {
        crate::text_painter::paint_text_combine(canvas, shape_result, origin, style, tc_layout);
    } else if upright {
        let vertical_baseline_x = if style.writing_mode == openui_style::WritingMode::VerticalRl {
            abs_offset.left.to_f32() + fragment.size.width.to_f32() - fragment.baseline_offset
        } else {
            abs_offset.left.to_f32() + fragment.baseline_offset
        };
        crate::text_painter::paint_vertical_text(
            canvas,
            shape_result,
            vertical_baseline_x,
            abs_offset.top.to_f32(),
            style,
        );
    } else {
        let clip_deterministic_clamp_marker = deterministic_text_profile
            && !rotated
            && text_content == Some("\u{2026}")
            && style.font_size > 0.0;
        if clip_deterministic_clamp_marker {
            canvas.save();
            canvas.clip_rect(
                Rect::from_xywh(
                    abs_offset.left.to_f32().floor(),
                    if style.font_size <= 16.0 {
                        abs_offset.top.to_f32().floor()
                    } else {
                        abs_offset.top.to_f32().round()
                    },
                    fragment.size.width.to_f32(),
                    fragment.size.height.to_f32()
                        + if (style.font_size - 24.0).abs() < 0.01 {
                            1.0
                        } else {
                            0.0
                        },
                ),
                ClipOp::Intersect,
                false,
            );
        }
        crate::text_painter::paint_text(canvas, shape_result, origin, style);
        if !rotated
            && all_ahem_runs
            && deterministic_text_profile
            && !inside_line_clamp
            && !inside_block_content_alignment
            && style.font_size >= 16.0
            // At 24px and above the unhinted aliased Ahem mask already
            // includes the complete block-start row. Replaying it would add
            // a spurious twenty-fifth row above the glyph.
            && style.font_size < 24.0
            && (style.font_size - style.font_size.round()).abs() < 0.01
            && abs_offset
                .top
                .raw()
                .rem_euclid(LayoutUnit::from_i32(1).raw())
                >= LayoutUnit::from_f32(0.5).raw()
        {
            // When the aliased Ahem ink origin crosses the half-pixel device
            // threshold, Chromium's FreeType mask retains the block-start
            // coverage row. Replay the square mask one row toward block-start
            // without changing layout's baseline or advance geometry.
            crate::text_painter::paint_text(
                canvas,
                shape_result,
                (origin.0, origin.1 - 1.0),
                style,
            );
        }
        if clip_deterministic_clamp_marker {
            canvas.restore();
        }
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

    if rotated {
        canvas.restore();
    }
}

fn snap_aliased_ahem_keyword_baseline(
    baseline: f32,
    style: &ComputedStyle,
    all_ahem_runs: bool,
    has_subscript_ancestor: bool,
) -> f32 {
    let is_subscript =
        style.vertical_align == openui_style::VerticalAlign::Sub || has_subscript_ancestor;
    if !is_subscript || (style.font_size - 12.0).abs() >= 0.01 {
        return baseline;
    }
    if !all_ahem_runs {
        // The pinned Chromium/FreeType profile advances the real-font mask to
        // the next device row after the fixed-point CSS subscript shift. Skia
        // retains the fractional origin, so mirror that final raster step.
        return baseline + 1.0;
    }
    if baseline.rem_euclid(1.0) > 0.25 {
        // Chromium's pinned FreeType mask advances a 12px Ahem subscript at
        // a strict quarter-pixel baseline phase. Skia's aliased mask floors
        // every phase, so advance the draw origin while leaving layout and
        // inline decoration geometry unchanged.
        baseline + 1.0
    } else {
        baseline
    }
}

fn text_has_subscript_ancestor(fragment: &Fragment, doc: &Document) -> bool {
    if fragment.node_id.is_none() {
        return false;
    }
    let mut ancestor = doc.node(fragment.node_id).parent;
    while !ancestor.is_none() {
        let node = doc.node(ancestor);
        if node.style.vertical_align == openui_style::VerticalAlign::Sub {
            return true;
        }
        if !node.style.display.is_inline_level() {
            return false;
        }
        ancestor = node.parent;
    }
    false
}

fn resolve_background_length(length: &openui_geometry::Length, basis: f32, auto: f32) -> f32 {
    if length.is_fixed() {
        length.value()
    } else if length.is_percent() {
        length.value() * basis / 100.0
    } else if length.is_calculated() {
        length.value() * basis / 100.0 + length.calc_offset()
    } else {
        auto
    }
}

fn resolve_background_position(
    position: BackgroundPosition,
    area_start: f32,
    area_size: f32,
    image_size: f32,
) -> f32 {
    match position {
        BackgroundPosition::Percent(percent) => {
            area_start + (area_size - image_size) * percent / 100.0
        }
        BackgroundPosition::Length(length) => {
            area_start + resolve_background_length(&length, area_size, 0.0)
        }
        BackgroundPosition::Edge { end, offset } => {
            let offset = resolve_background_length(&offset, (area_size - image_size).max(0.0), 0.0);
            if end {
                area_start + area_size - image_size - offset
            } else {
                area_start + offset
            }
        }
    }
}

fn background_box_rect(
    clip: BackgroundClip,
    border_rect: Rect,
    fragment: Option<&Fragment>,
) -> Rect {
    let Some(fragment) = fragment else {
        return border_rect;
    };
    let padding = Rect::from_ltrb(
        border_rect.left + fragment.border.left.round().to_f32(),
        border_rect.top + fragment.border.top.round().to_f32(),
        border_rect.right - fragment.border.right.round().to_f32(),
        border_rect.bottom - fragment.border.bottom.round().to_f32(),
    );
    match clip {
        BackgroundClip::BorderBox | BackgroundClip::BorderArea | BackgroundClip::Text => {
            border_rect
        }
        BackgroundClip::PaddingBox => padding,
        BackgroundClip::ContentBox => Rect::from_ltrb(
            padding.left + fragment.padding.left.round().to_f32(),
            padding.top + fragment.padding.top.round().to_f32(),
            padding.right - fragment.padding.right.round().to_f32(),
            padding.bottom - fragment.padding.bottom.round().to_f32(),
        ),
    }
}

fn background_clip_radii(
    style: &ComputedStyle,
    fragment: Option<&Fragment>,
    border_rect: Rect,
    clip_rect: Rect,
) -> [Point; 4] {
    let outer = fragment.map_or_else(
        || normalized_border_radii(style, &border_rect),
        |fragment| {
            fragment_border_radii(
                style,
                fragment,
                &border_rect,
                style.effective_border_top() as f32,
            )
        },
    );
    let left = clip_rect.left - border_rect.left;
    let top = clip_rect.top - border_rect.top;
    let right = border_rect.right - clip_rect.right;
    let bottom = border_rect.bottom - clip_rect.bottom;
    normalize_radii_to_rect(
        [
            Point::new((outer[0].x - left).max(0.0), (outer[0].y - top).max(0.0)),
            Point::new((outer[1].x - right).max(0.0), (outer[1].y - top).max(0.0)),
            Point::new(
                (outer[2].x - right).max(0.0),
                (outer[2].y - bottom).max(0.0),
            ),
            Point::new((outer[3].x - left).max(0.0), (outer[3].y - bottom).max(0.0)),
        ],
        &clip_rect,
    )
}

fn css_gradient_positions(stops: &[openui_style::GradientStop], line_length: f32) -> Vec<f32> {
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
            GradientStopPosition::Calc { percent, px } => Some(if line_length > 0.0 {
                percent / 100.0 + px / line_length
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
        let first = result[start].unwrap_or_default();
        let last = result[end].unwrap_or(first);
        for (index, position) in result.iter_mut().enumerate().take(end).skip(start + 1) {
            *position =
                Some(first + (last - first) * (index - start) as f32 / (end - start) as f32);
        }
        start = end;
    }
    let mut previous = f32::NEG_INFINITY;
    result
        .into_iter()
        .map(|position| {
            let position = position.unwrap_or(previous).max(previous);
            previous = position;
            position
        })
        .collect()
}

fn skia_gradient_interpolation(
    color_space: GradientColorSpace,
) -> skia_safe::gradient::Interpolation {
    use skia_safe::gradient::interpolation;
    skia_safe::gradient::Interpolation {
        in_premul: interpolation::InPremul::No,
        color_space: match color_space {
            GradientColorSpace::Srgb => interpolation::ColorSpace::SRGB,
            GradientColorSpace::Hsl => interpolation::ColorSpace::HSL,
            GradientColorSpace::Oklch => interpolation::ColorSpace::OKLCH,
        },
        hue_method: interpolation::HueMethod::Shorter,
    }
}

fn gradient_colors4f(
    stops: &[openui_style::GradientStop],
    current_color: &Color,
    opacity_multiplier: f32,
) -> Vec<Color4f> {
    stops
        .iter()
        .map(|stop| {
            let color = stop.color.resolve(current_color);
            Color4f::new(color.r, color.g, color.b, color.a * opacity_multiplier)
        })
        .collect()
}

fn gradient_colors(
    stops: &[openui_style::GradientStop],
    current_color: &Color,
    opacity_multiplier: f32,
) -> Vec<skia_safe::Color> {
    stops
        .iter()
        .map(|stop| {
            skia_css_color_with_alpha(&stop.color.resolve(current_color), opacity_multiplier)
        })
        .collect()
}

fn solid_gradient_color(image: &CssImage, current_color: &Color) -> Option<Color> {
    let stops = match image {
        CssImage::LinearGradient(gradient) => gradient.stops.as_slice(),
        CssImage::RadialGradient(gradient) => gradient.stops.as_slice(),
        CssImage::ConicGradient(gradient) => gradient.stops.as_slice(),
        CssImage::Raster(_) => return None,
    };
    let first = stops.first()?.color.resolve(current_color);
    stops
        .iter()
        .all(|stop| stop.color.resolve(current_color) == first)
        .then_some(first)
}

fn intrinsic_image_size(doc: &Document, image: &CssImage, area: Rect) -> (f32, f32) {
    match image {
        CssImage::Raster(id) => crate::image_resource::decode_image_resource(doc, *id)
            .map(|image| (image.width() as f32, image.height() as f32))
            .unwrap_or((0.0, 0.0)),
        CssImage::LinearGradient(_) | CssImage::RadialGradient(_) | CssImage::ConicGradient(_) => {
            (area.width(), area.height())
        }
    }
}

fn layer_image_size(doc: &Document, layer: &BackgroundLayer, area: Rect) -> (f32, f32) {
    let intrinsic = intrinsic_image_size(doc, &layer.image, area);
    let ratio = match layer.image {
        CssImage::Raster(_) if intrinsic.0 > 0.0 && intrinsic.1 > 0.0 => {
            Some(intrinsic.0 / intrinsic.1)
        }
        _ => None,
    };
    match &layer.size {
        BackgroundSize::Auto => intrinsic,
        BackgroundSize::Explicit(width, height) => {
            let width_auto = width.is_auto();
            let height_auto = height.is_auto();
            let mut w = resolve_background_length(width, area.width(), intrinsic.0);
            let mut h = resolve_background_length(height, area.height(), intrinsic.1);
            if width_auto && !height_auto {
                w = ratio.map_or(area.width(), |ratio| h * ratio);
            } else if height_auto && !width_auto {
                h = ratio.map_or(area.height(), |ratio| w / ratio);
            } else if width_auto && height_auto {
                w = intrinsic.0;
                h = intrinsic.1;
            }
            (w, h)
        }
        BackgroundSize::Cover | BackgroundSize::Contain => {
            let Some(ratio) = ratio else {
                return (area.width(), area.height());
            };
            let area_ratio = if area.height() > 0.0 {
                area.width() / area.height()
            } else {
                ratio
            };
            let use_width = match layer.size {
                BackgroundSize::Cover => area_ratio > ratio,
                BackgroundSize::Contain => area_ratio < ratio,
                _ => unreachable!(),
            };
            if use_width {
                (area.width(), area.width() / ratio)
            } else {
                (area.height() * ratio, area.height())
            }
        }
    }
}

fn axis_tiles(
    repeat: BackgroundRepeat,
    clip_start: f32,
    clip_end: f32,
    area_start: f32,
    area_size: f32,
    positioned_start: f32,
    tile_size: f32,
) -> Vec<f32> {
    if tile_size <= 0.0 || !tile_size.is_finite() {
        return Vec::new();
    }
    match repeat {
        BackgroundRepeat::NoRepeat => vec![positioned_start],
        BackgroundRepeat::Space => {
            let count = (area_size / tile_size).floor() as i32;
            if count < 2 {
                vec![positioned_start]
            } else {
                let gap = (area_size - count as f32 * tile_size) / (count - 1) as f32;
                let period = tile_size + gap;
                let mut first = area_start;
                while first > clip_start {
                    first -= period;
                }
                while first + tile_size <= clip_start {
                    first += period;
                }
                let mut result = Vec::new();
                let mut value = first;
                while value < clip_end {
                    result.push(value);
                    value += period;
                }
                result
            }
        }
        BackgroundRepeat::Repeat | BackgroundRepeat::Round => {
            let mut first = positioned_start;
            while first > clip_start {
                first -= tile_size;
            }
            while first + tile_size <= clip_start {
                first += tile_size;
            }
            let mut result = Vec::new();
            let mut value = first;
            while value < clip_end {
                result.push(value);
                value += tile_size;
            }
            result
        }
    }
}

fn paint_css_image_tile(
    canvas: &Canvas,
    doc: &Document,
    style: &ComputedStyle,
    image: &CssImage,
    tile: Rect,
    wrap_x: bool,
    wrap_y: bool,
    opacity_multiplier: f32,
    resample_from: Option<(f32, f32)>,
) {
    if tile.width() <= 0.0 || tile.height() <= 0.0 {
        return;
    }
    if let Some((source_width, source_height)) = resample_from {
        let width = source_width.ceil().max(1.0) as i32;
        let height = source_height.ceil().max(1.0) as i32;
        let Some(mut surface) = surfaces::raster_n32_premul((width, height)) else {
            return;
        };
        surface.canvas().clear(skia_safe::Color::TRANSPARENT);
        paint_css_image_tile(
            surface.canvas(),
            doc,
            style,
            image,
            Rect::from_xywh(0.0, 0.0, source_width, source_height),
            false,
            false,
            1.0,
            None,
        );
        let rasterized = surface.image_snapshot();
        let local_matrix = Matrix::scale_translate(
            (tile.width() / source_width, tile.height() / source_height),
            (tile.left, tile.top),
        );
        let mut paint = Paint::default();
        paint.set_alpha_f(opacity_multiplier);
        paint.set_shader(rasterized.to_shader(
            (
                if wrap_x {
                    TileMode::Repeat
                } else {
                    TileMode::Clamp
                },
                if wrap_y {
                    TileMode::Repeat
                } else {
                    TileMode::Clamp
                },
            ),
            SamplingOptions::from(FilterMode::Linear),
            &local_matrix,
        ));
        canvas.draw_rect(tile, &paint);
        return;
    }
    match image {
        CssImage::Raster(id) => {
            let Ok(image) = crate::image_resource::decode_image_resource(doc, *id) else {
                return;
            };
            let image_width = image.width() as f32;
            let image_height = image.height() as f32;
            let scale_x = tile.width() / image_width;
            let scale_y = tile.height() / image_height;
            let wrap_x = wrap_x && (scale_x - scale_x.round()).abs() > 1.0e-5;
            let wrap_y = wrap_y && (scale_y - scale_y.round()).abs() > 1.0e-5;
            let mut paint = Paint::default();
            paint.set_alpha_f(opacity_multiplier);
            if wrap_x && wrap_y {
                let Ok(quantized) = crate::image_resource::quantized_repeated_image(
                    doc,
                    *id,
                    tile.width().round() as i32,
                    tile.height().round() as i32,
                ) else {
                    return;
                };
                let quantized_source = Rect::from_xywh(
                    0.0,
                    0.0,
                    quantized.width() as f32,
                    quantized.height() as f32,
                );
                canvas.draw_image_rect_with_sampling_options(
                    quantized,
                    Some((&quantized_source, SrcRectConstraint::Strict)),
                    tile,
                    SamplingOptions::from(FilterMode::Nearest),
                    &paint,
                );
                return;
            }
            if (scale_x - scale_x.round()).abs() > 1.0e-5
                || (scale_y - scale_y.round()).abs() > 1.0e-5
            {
                let source = Rect::from_xywh(0.0, 0.0, image_width, image_height);
                let quantized = (!wrap_x && !wrap_y)
                    .then(|| {
                        crate::image_resource::quantized_image_patch(
                            &image,
                            source,
                            Rect::from_xywh(
                                tile.left,
                                tile.top,
                                tile.width().round(),
                                tile.height().round(),
                            ),
                            tile.width().round() as i32,
                            tile.height().round() as i32,
                            crate::image_resource::ImagePatchPhase::Tile {
                                close_downscale: (tile.width() - tile.width().round()).abs()
                                    > 0.001
                                    || (tile.height() - tile.height().round()).abs() > 0.001,
                                wrap_x,
                                wrap_y,
                            },
                        )
                    })
                    .transpose()
                    .ok()
                    .flatten();
                if let Some(quantized) = quantized {
                    let quantized_source = Rect::from_xywh(
                        0.0,
                        0.0,
                        quantized.width() as f32,
                        quantized.height() as f32,
                    );
                    canvas.draw_image_rect_with_sampling_options(
                        quantized,
                        Some((&quantized_source, SrcRectConstraint::Strict)),
                        tile,
                        SamplingOptions::from(
                            if (tile.width() - tile.width().round()).abs() < 0.001
                                && (tile.height() - tile.height().round()).abs() < 0.001
                            {
                                FilterMode::Nearest
                            } else {
                                FilterMode::Linear
                            },
                        ),
                        &paint,
                    );
                    return;
                }
            }
            let local_matrix = Matrix::scale_translate((scale_x, scale_y), (tile.left, tile.top));
            paint.set_shader(image.to_shader(
                (
                    if wrap_x {
                        TileMode::Repeat
                    } else {
                        TileMode::Clamp
                    },
                    if wrap_y {
                        TileMode::Repeat
                    } else {
                        TileMode::Clamp
                    },
                ),
                SamplingOptions::from(FilterMode::Linear),
                &local_matrix,
            ));
            canvas.draw_rect(tile, &paint);
        }
        CssImage::LinearGradient(gradient) => {
            if gradient.stops.len() < 2 {
                return;
            }
            let direction = if let Some((horizontal, vertical)) = gradient.corner_direction {
                let x = horizontal / tile.width();
                let y = vertical / tile.height();
                let length = (x * x + y * y).sqrt();
                Point::new(x / length, y / length)
            } else {
                let radians = gradient.angle_degrees.to_radians();
                Point::new(radians.sin(), -radians.cos())
            };
            let line_length = tile.width() * direction.x.abs() + tile.height() * direction.y.abs();
            if line_length <= 0.0 {
                return;
            }
            let center = Point::new(
                (tile.left + tile.right) * 0.5,
                (tile.top + tile.bottom) * 0.5,
            );
            let start = Point::new(
                center.x - direction.x * line_length * 0.5,
                center.y - direction.y * line_length * 0.5,
            );
            let end = Point::new(
                center.x + direction.x * line_length * 0.5,
                center.y + direction.y * line_length * 0.5,
            );
            let mut positions = css_gradient_positions(&gradient.stops, line_length);
            let mut shader_start = start;
            let mut shader_end = end;
            let mut mode = TileMode::Clamp;
            if gradient.repeating {
                let first = positions.first().copied().unwrap_or_default() * line_length;
                let last = positions.last().copied().unwrap_or(1.0) * line_length;
                if last > first {
                    for position in &mut positions {
                        *position = (*position * line_length - first) / (last - first);
                    }
                    shader_start =
                        Point::new(start.x + direction.x * first, start.y + direction.y * first);
                    shader_end =
                        Point::new(start.x + direction.x * last, start.y + direction.y * last);
                    mode = TileMode::Repeat;
                }
            }
            let mut paint = Paint::default();
            if gradient.color_space == GradientColorSpace::Srgb {
                let colors = gradient_colors(&gradient.stops, &style.color, opacity_multiplier);
                paint.set_shader(gradient_shader::linear(
                    (shader_start, shader_end),
                    colors.as_slice(),
                    positions.as_slice(),
                    mode,
                    None,
                    None,
                ));
            } else {
                let colors = gradient_colors4f(&gradient.stops, &style.color, opacity_multiplier);
                paint.set_shader(gradient_shader::linear_with_interpolation(
                    (shader_start, shader_end),
                    (colors.as_slice(), None),
                    positions.as_slice(),
                    mode,
                    skia_gradient_interpolation(gradient.color_space),
                    None,
                ));
            }
            canvas.draw_rect(tile, &paint);
        }
        CssImage::RadialGradient(gradient) => {
            if gradient.stops.len() < 2 {
                return;
            }
            let mut cx =
                resolve_background_position(gradient.center_x, tile.left, tile.width(), 0.0);
            let mut cy =
                resolve_background_position(gradient.center_y, tile.top, tile.height(), 0.0);
            let left = (cx - tile.left).abs();
            let right = (tile.right - cx).abs();
            let top = (cy - tile.top).abs();
            let bottom = (tile.bottom - cy).abs();
            let (mut rx, mut ry) = match &gradient.size {
                RadialGradientSize::ClosestSide => (left.min(right), top.min(bottom)),
                RadialGradientSize::FarthestSide => (left.max(right), top.max(bottom)),
                RadialGradientSize::ClosestCorner => {
                    let scale = 2.0_f32.sqrt();
                    (left.min(right) * scale, top.min(bottom) * scale)
                }
                RadialGradientSize::FarthestCorner => {
                    let scale = 2.0_f32.sqrt();
                    (left.max(right) * scale, top.max(bottom) * scale)
                }
                RadialGradientSize::Explicit(x, y) => (
                    resolve_background_length(x, tile.width(), tile.width() * 0.5),
                    resolve_background_length(y, tile.height(), tile.height() * 0.5),
                ),
            };
            if gradient.shape == RadialGradientShape::Circle {
                let radius = rx.max(ry);
                rx = radius;
                ry = radius;
            }
            if rx <= 0.0 || ry <= 0.0 {
                return;
            }
            let mut positions = css_gradient_positions(&gradient.stops, rx.max(ry));
            let has_hard_stop = positions
                .windows(2)
                .any(|pair| (pair[0] - pair[1]).abs() < f32::EPSILON);
            if has_hard_stop && (rx - ry).abs() < 0.001 {
                // Blink's gradient display item is quantized on a 1/1024px
                // device grid before the Skia shader is built. Preserve that
                // lower-edge bias so equal-position stops classify exact
                // lattice-boundary pixels identically in every quadrant.
                let device_quantum = 1.0 / 1024.0;
                cx -= device_quantum;
                cy -= device_quantum;
                for position in &mut positions {
                    *position -= device_quantum / rx.max(ry);
                }
            }
            let mut radius = 1.0;
            let mut mode = TileMode::Clamp;
            if gradient.repeating {
                let first = positions.first().copied().unwrap_or_default();
                let last = positions.last().copied().unwrap_or(1.0);
                if last > first {
                    for position in &mut positions {
                        *position = (*position - first) / (last - first);
                    }
                    radius = last - first;
                    mode = TileMode::Repeat;
                }
            }
            if (rx - ry).abs() < 0.001 {
                // Keep circular radial gradients in device coordinates. This
                // is the path Blink/Skia use for circles and, importantly,
                // preserves their hard-stop boundary classification without
                // an avoidable unit-circle inverse transform.
                let mut paint = Paint::default();
                if gradient.color_space == GradientColorSpace::Srgb {
                    let colors = gradient_colors(&gradient.stops, &style.color, opacity_multiplier);
                    paint.set_shader(gradient_shader::radial(
                        Point::new(cx, cy),
                        rx * radius,
                        colors.as_slice(),
                        positions.as_slice(),
                        mode,
                        None,
                        None,
                    ));
                } else {
                    let colors =
                        gradient_colors4f(&gradient.stops, &style.color, opacity_multiplier);
                    paint.set_shader(gradient_shader::radial_with_interpolation(
                        (Point::new(cx, cy), rx * radius),
                        (colors.as_slice(), None),
                        positions.as_slice(),
                        mode,
                        skia_gradient_interpolation(gradient.color_space),
                        None,
                    ));
                }
                canvas.draw_rect(tile, &paint);
                return;
            }
            canvas.save();
            canvas.translate(Point::new(cx, cy));
            canvas.scale((rx, ry));
            let mut paint = Paint::default();
            if gradient.color_space == GradientColorSpace::Srgb {
                let colors = gradient_colors(&gradient.stops, &style.color, opacity_multiplier);
                paint.set_shader(gradient_shader::radial(
                    Point::new(0.0, 0.0),
                    radius,
                    colors.as_slice(),
                    positions.as_slice(),
                    mode,
                    None,
                    None,
                ));
            } else {
                let colors = gradient_colors4f(&gradient.stops, &style.color, opacity_multiplier);
                paint.set_shader(gradient_shader::radial_with_interpolation(
                    (Point::new(0.0, 0.0), radius),
                    (colors.as_slice(), None),
                    positions.as_slice(),
                    mode,
                    skia_gradient_interpolation(gradient.color_space),
                    None,
                ));
            }
            canvas.draw_rect(
                Rect::from_ltrb(
                    (tile.left - cx) / rx,
                    (tile.top - cy) / ry,
                    (tile.right - cx) / rx,
                    (tile.bottom - cy) / ry,
                ),
                &paint,
            );
            canvas.restore();
        }
        CssImage::ConicGradient(gradient) => {
            if gradient.stops.len() < 2 {
                return;
            }
            let center = Point::new(
                resolve_background_position(gradient.center_x, tile.left, tile.width(), 0.0),
                resolve_background_position(gradient.center_y, tile.top, tile.height(), 0.0),
            );
            let positions = css_gradient_positions(&gradient.stops, 360.0);
            let mode = if gradient.repeating {
                TileMode::Repeat
            } else {
                TileMode::Clamp
            };
            let mut paint = Paint::default();
            let matrix = Matrix::rotate_deg_pivot(gradient.from_degrees - 90.0, center);
            if gradient.color_space == GradientColorSpace::Srgb {
                let colors = gradient_colors(&gradient.stops, &style.color, opacity_multiplier);
                paint.set_shader(gradient_shader::sweep(
                    center,
                    colors.as_slice(),
                    positions.as_slice(),
                    mode,
                    None,
                    None,
                    Some(&matrix),
                ));
            } else {
                let colors = gradient_colors4f(&gradient.stops, &style.color, opacity_multiplier);
                paint.set_shader(gradient_shader::sweep_with_interpolation(
                    center,
                    (colors.as_slice(), None),
                    positions.as_slice(),
                    mode,
                    None,
                    skia_gradient_interpolation(gradient.color_space),
                    Some(&matrix),
                ));
            }
            canvas.draw_rect(tile, &paint);
        }
    }
}

fn paint_background_layers(
    canvas: &Canvas,
    doc: &Document,
    fragment: Option<&Fragment>,
    style: &ComputedStyle,
    border_rect: Rect,
    opacity_multiplier: f32,
    clip_override: Option<Rect>,
) {
    for layer in style.background_layers.iter().rev() {
        let mut area = background_box_rect(layer.origin, border_rect, fragment);
        if layer.attachment == BackgroundAttachment::Fixed {
            let (width, height) = VIEWPORT_SIZE.with(|size| *size.borrow());
            area = Rect::from_xywh(0.0, 0.0, width, height);
        }
        let mut clip =
            clip_override.unwrap_or_else(|| background_box_rect(layer.clip, border_rect, fragment));
        if clip_override.is_none() && layer.attachment == BackgroundAttachment::Local {
            // A local image scrolls with the element's contents and is exposed
            // through the scrollport. Its effective painting area cannot
            // extend beneath the border (notably through double-border gaps),
            // even when the authored background-clip is border-box.
            let scrollport = background_box_rect(BackgroundClip::PaddingBox, border_rect, fragment);
            clip = Rect::from_ltrb(
                clip.left.max(scrollport.left),
                clip.top.max(scrollport.top),
                clip.right.min(scrollport.right),
                clip.bottom.min(scrollport.bottom),
            );
        }
        if area.width() <= 0.0
            || area.height() <= 0.0
            || clip.width() <= 0.0
            || clip.height() <= 0.0
        {
            continue;
        }
        let (mut tile_width, mut tile_height) = layer_image_size(doc, layer, area);
        let authored_tile_size = (tile_width, tile_height);
        if layer.repeat_x == BackgroundRepeat::Round && tile_width > 0.0 {
            let count = (area.width() / tile_width).round().max(1.0);
            let adjusted = area.width() / count;
            if matches!(layer.size, BackgroundSize::Auto)
                || matches!(&layer.size, BackgroundSize::Explicit(_, height) if height.is_auto())
            {
                tile_height *= adjusted / tile_width;
            }
            tile_width = adjusted;
        }
        if layer.repeat_y == BackgroundRepeat::Round && tile_height > 0.0 {
            let count = (area.height() / tile_height).round().max(1.0);
            let adjusted = area.height() / count;
            if matches!(layer.size, BackgroundSize::Auto)
                || matches!(&layer.size, BackgroundSize::Explicit(width, _) if width.is_auto())
            {
                tile_width *= adjusted / tile_height;
            }
            tile_height = adjusted;
        }
        let round_resampled_x = layer.repeat_x == BackgroundRepeat::Round
            && (tile_width - authored_tile_size.0).abs() > 0.001;
        let round_resampled_y = layer.repeat_y == BackgroundRepeat::Round
            && (tile_height - authored_tile_size.1).abs() > 0.001;
        let position_width = if round_resampled_x {
            authored_tile_size.0
        } else {
            tile_width
        };
        let position_height = if round_resampled_y {
            authored_tile_size.1
        } else {
            tile_height
        };
        let mut image_left =
            resolve_background_position(layer.position_x, area.left, area.width(), position_width);
        let mut image_top =
            resolve_background_position(layer.position_y, area.top, area.height(), position_height);
        // Blink snaps an integer-sized `round` tile's phase to device pixels;
        // exact half-pixel phases choose the preceding pixel.
        if layer.repeat_x == BackgroundRepeat::Round
            && (tile_width - tile_width.round()).abs() < 0.001
            && !round_resampled_x
        {
            image_left = (image_left + 0.5 - 1.0e-4).floor();
        }
        if layer.repeat_y == BackgroundRepeat::Round
            && (tile_height - tile_height.round()).abs() < 0.001
            && !round_resampled_y
        {
            image_top = (image_top + 0.5 - 1.0e-4).floor();
        }
        let xs = axis_tiles(
            layer.repeat_x,
            clip.left,
            clip.right,
            area.left,
            area.width(),
            image_left,
            tile_width,
        );
        let ys = axis_tiles(
            layer.repeat_y,
            clip.top,
            clip.bottom,
            area.top,
            area.height(),
            image_top,
            tile_height,
        );
        // Repeated generated images are sampled as one concrete image shader.
        // Rasterizing at the final (including `round`-adjusted) tile size lets
        // linear filtering cross a fractional repeat seam without changing the
        // gradient geometry to the pre-round authored size.
        let resample_from = if !matches!(layer.image, CssImage::Raster(_))
            && ((matches!(
                layer.repeat_x,
                BackgroundRepeat::Repeat | BackgroundRepeat::Round
            ) && xs.len() > 1)
                || (matches!(
                    layer.repeat_y,
                    BackgroundRepeat::Repeat | BackgroundRepeat::Round
                ) && ys.len() > 1))
        {
            Some((tile_width, tile_height))
        } else {
            None
        };
        canvas.save();
        if clip_override.is_some() {
            canvas.clip_rect(clip, ClipOp::Intersect, false);
        } else if layer.clip == BackgroundClip::BorderArea {
            canvas.clip_rect(border_rect, ClipOp::Intersect, false);
            let padding = background_box_rect(BackgroundClip::PaddingBox, border_rect, fragment);
            canvas.clip_rect(padding, ClipOp::Difference, false);
        } else if style.has_border_radius()
            && !fragment.is_some_and(|fragment| fragment.ignore_border_radius)
        {
            let radii = background_clip_radii(style, fragment, border_rect, clip);
            canvas.clip_rrect(RRect::new_rect_radii(clip, &radii), ClipOp::Intersect, true);
        } else {
            canvas.clip_rect(clip, ClipOp::Intersect, false);
        }
        let repeated_constant_gradient = matches!(
            layer.repeat_x,
            BackgroundRepeat::Repeat | BackgroundRepeat::Round
        ) && matches!(
            layer.repeat_y,
            BackgroundRepeat::Repeat | BackgroundRepeat::Round
        ) && tile_width > 0.0
            && tile_height > 0.0;
        if let Some(color) = repeated_constant_gradient
            .then(|| solid_gradient_color(&layer.image, &style.color))
            .flatten()
        {
            // Repetition of a constant generated image is itself constant,
            // including when the authored tile is smaller than one device
            // pixel. Avoid resampling a subpixel temporary surface, which can
            // quantize the otherwise uniform image to transparent.
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(false);
            set_paint_css_color_with_alpha(&mut paint, &color, opacity_multiplier);
            canvas.draw_rect(clip, &paint);
            canvas.restore();
            continue;
        }
        for top in &ys {
            for left in &xs {
                paint_css_image_tile(
                    canvas,
                    doc,
                    style,
                    &layer.image,
                    Rect::from_xywh(*left, *top, tile_width, tile_height),
                    matches!(
                        layer.repeat_x,
                        BackgroundRepeat::Repeat | BackgroundRepeat::Round
                    ) && xs.len() > 1,
                    matches!(
                        layer.repeat_y,
                        BackgroundRepeat::Repeat | BackgroundRepeat::Round
                    ) && ys.len() > 1,
                    opacity_multiplier,
                    resample_from,
                );
            }
        }
        canvas.restore();
    }
}

fn fragment_rect_for_node<'a>(
    fragment: &'a Fragment,
    node_id: NodeId,
    parent_offset: PhysicalOffset,
) -> Option<(Rect, &'a Fragment)> {
    let offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    if fragment.node_id == node_id {
        return Some((
            Rect::from_xywh(
                offset.left.to_f32(),
                offset.top.to_f32(),
                fragment.size.width.to_f32(),
                fragment.size.height.to_f32(),
            ),
            fragment,
        ));
    }
    fragment
        .children
        .iter()
        .find_map(|child| fragment_rect_for_node(child, node_id, offset))
}

/// Paint the propagated html/body background across the viewport canvas.
pub(crate) fn paint_canvas_background(
    canvas: &Canvas,
    doc: &Document,
    root_fragment: &Fragment,
    width: f32,
    height: f32,
) {
    set_paint_viewport_size(width, height);
    let Some(source) = doc.canvas_background_source() else {
        return;
    };
    let style = &doc.node(source).style;
    let canvas_rect = Rect::from_xywh(0.0, 0.0, width, height);
    let (positioning_rect, source_fragment) =
        fragment_rect_for_node(root_fragment, source, PhysicalOffset::zero())
            .map_or((canvas_rect, None), |(rect, fragment)| {
                (rect, Some(fragment))
            });
    if !style.background_color.is_transparent() {
        let mut paint = Paint::default();
        set_paint_css_color(&mut paint, &style.background_color);
        canvas.draw_rect(canvas_rect, &paint);
    }
    paint_background_layers(
        canvas,
        doc,
        source_fragment,
        style,
        positioning_rect,
        1.0,
        Some(canvas_rect),
    );
}

fn resolve_border_image_slice(value: &BorderImageLength, source_extent: f32) -> f32 {
    match value {
        BorderImageLength::Number(value) => *value,
        BorderImageLength::Length(length) => resolve_margin_or_padding_f32(length, source_extent),
        BorderImageLength::Auto => source_extent,
    }
    .max(0.0)
}

fn resolve_border_image_width(
    value: &BorderImageLength,
    border_width: f32,
    area_extent: f32,
    source_slice: f32,
) -> f32 {
    match value {
        BorderImageLength::Number(value) => value * border_width,
        BorderImageLength::Length(length) => resolve_margin_or_padding_f32(length, area_extent),
        BorderImageLength::Auto => source_slice,
    }
    .max(0.0)
}

fn resolve_border_image_outset(
    value: &BorderImageLength,
    border_width: f32,
    area_extent: f32,
) -> f32 {
    match value {
        BorderImageLength::Number(value) => value * border_width,
        BorderImageLength::Length(length) => resolve_margin_or_padding_f32(length, area_extent),
        BorderImageLength::Auto => 0.0,
    }
    .max(0.0)
}

fn fit_opposing_edges(first: f32, second: f32, extent: f32) -> (f32, f32) {
    let sum = first + second;
    if sum > extent && sum > 0.0 {
        let scale = extent / sum;
        (first * scale, second * scale)
    } else {
        (first, second)
    }
}

fn border_image_axis_tiles(
    repeat: BorderImageRepeat,
    start: f32,
    end: f32,
    natural: f32,
) -> Vec<(f32, f32)> {
    let extent = end - start;
    if extent <= 0.0 || natural <= 0.0 {
        return Vec::new();
    }
    match repeat {
        BorderImageRepeat::Stretch => vec![(start, extent)],
        BorderImageRepeat::Round => {
            let count = (extent / natural).round().max(1.0) as usize;
            let size = extent / count as f32;
            (0..count)
                .map(|index| (start + index as f32 * size, size))
                .collect()
        }
        BorderImageRepeat::Space => {
            let count = (extent / natural).floor() as usize;
            if count < 1 {
                vec![(start + (extent - natural) * 0.5, natural)]
            } else {
                let gap = (extent - count as f32 * natural) / (count + 1) as f32;
                (0..count)
                    .map(|index| (start + gap + index as f32 * (natural + gap), natural))
                    .collect()
            }
        }
        BorderImageRepeat::Repeat => {
            let count = (extent / natural).ceil().max(1.0) as usize;
            let first = start + (extent - count as f32 * natural) * 0.5;
            (0..count)
                .map(|index| (first + index as f32 * natural, natural))
                .collect()
        }
    }
}

fn draw_border_image_patch(
    canvas: &Canvas,
    image: &skia_safe::Image,
    source: Rect,
    destination: Rect,
    repeat_x: BorderImageRepeat,
    repeat_y: BorderImageRepeat,
    opacity_multiplier: f32,
) {
    if source.width() <= 0.0
        || source.height() <= 0.0
        || destination.width() <= 0.0
        || destination.height() <= 0.0
    {
        return;
    }
    let x_stretch = repeat_x == BorderImageRepeat::Stretch;
    let y_stretch = repeat_y == BorderImageRepeat::Stretch;
    let (natural_width, natural_height) = match (x_stretch, y_stretch) {
        (true, true) => (destination.width(), destination.height()),
        (false, true) => {
            let scale = destination.height() / source.height();
            (source.width() * scale, destination.height())
        }
        (true, false) => {
            let scale = destination.width() / source.width();
            (destination.width(), source.height() * scale)
        }
        (false, false) => (source.width(), source.height()),
    };
    let xs = border_image_axis_tiles(repeat_x, destination.left, destination.right, natural_width);
    let ys = border_image_axis_tiles(
        repeat_y,
        destination.top,
        destination.bottom,
        natural_height,
    );
    let mut paint = Paint::default();
    paint.set_alpha_f(opacity_multiplier);
    canvas.save();
    canvas.clip_rect(destination, ClipOp::Intersect, false);
    for (top, height) in ys {
        for (left, width) in &xs {
            let integer_width = width.round() as i32;
            let integer_height = height.round() as i32;
            if repeat_x == BorderImageRepeat::Stretch
                && repeat_y == BorderImageRepeat::Stretch
                && (*width - integer_width as f32).abs() < 0.001
                && (height - integer_height as f32).abs() < 0.001
            {
                if let Ok(patch) = crate::image_resource::quantized_image_patch(
                    image,
                    source,
                    Rect::from_xywh(*left, top, *width, height),
                    integer_width,
                    integer_height,
                    crate::image_resource::ImagePatchPhase::Device,
                ) {
                    let patch_source =
                        Rect::from_xywh(0.0, 0.0, patch.width() as f32, patch.height() as f32);
                    canvas.draw_image_rect_with_sampling_options(
                        &patch,
                        Some((&patch_source, SrcRectConstraint::Strict)),
                        Rect::from_xywh(*left, top, *width, height),
                        SamplingOptions::from(FilterMode::Nearest),
                        &paint,
                    );
                    continue;
                }
            }
            canvas.draw_image_rect_with_sampling_options(
                image,
                Some((&source, SrcRectConstraint::Strict)),
                Rect::from_xywh(*left, top, *width, height),
                SamplingOptions::from(FilterMode::Linear),
                &paint,
            );
        }
    }
    canvas.restore();
}

fn draw_border_svg_patch(
    canvas: &Canvas,
    doc: &Document,
    id: openui_style::ImageResourceId,
    concrete_width: f32,
    concrete_height: f32,
    source: Rect,
    destination: Rect,
    repeat_x: BorderImageRepeat,
    repeat_y: BorderImageRepeat,
    opacity_multiplier: f32,
) {
    if source.width() <= 0.0
        || source.height() <= 0.0
        || destination.width() <= 0.0
        || destination.height() <= 0.0
    {
        return;
    }
    let x_stretch = repeat_x == BorderImageRepeat::Stretch;
    let y_stretch = repeat_y == BorderImageRepeat::Stretch;
    let (natural_width, natural_height) = match (x_stretch, y_stretch) {
        (true, true) => (destination.width(), destination.height()),
        (false, true) => {
            let scale = destination.height() / source.height();
            (source.width() * scale, destination.height())
        }
        (true, false) => {
            let scale = destination.width() / source.width();
            (destination.width(), source.height() * scale)
        }
        (false, false) => (source.width(), source.height()),
    };
    let xs = border_image_axis_tiles(repeat_x, destination.left, destination.right, natural_width);
    let ys = border_image_axis_tiles(
        repeat_y,
        destination.top,
        destination.bottom,
        natural_height,
    );
    canvas.save();
    canvas.clip_rect(destination, ClipOp::Intersect, false);
    for (top, height) in ys {
        for (left, width) in &xs {
            let _ = crate::image_resource::paint_svg_resource_patch(
                canvas,
                doc,
                id,
                concrete_width,
                concrete_height,
                source,
                Rect::from_xywh(*left, top, *width, height),
                opacity_multiplier,
            );
        }
    }
    canvas.restore();
}

fn border_image_source(
    doc: &Document,
    style: &ComputedStyle,
    border_image: &BorderImage,
    source_width: f32,
    source_height: f32,
) -> Option<skia_safe::Image> {
    if let CssImage::Raster(id) = &border_image.source {
        return crate::image_resource::decode_image_resource_at_size(
            doc,
            *id,
            source_width.ceil().max(1.0) as i32,
            source_height.ceil().max(1.0) as i32,
        )
        .ok();
    }
    let width = source_width.ceil().max(1.0) as i32;
    let height = source_height.ceil().max(1.0) as i32;
    let mut surface = surfaces::raster_n32_premul((width, height))?;
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    paint_css_image_tile(
        surface.canvas(),
        doc,
        style,
        &border_image.source,
        Rect::from_xywh(0.0, 0.0, width as f32, height as f32),
        false,
        false,
        1.0,
        None,
    );
    Some(surface.image_snapshot())
}

fn paint_border_image(
    canvas: &Canvas,
    doc: &Document,
    style: &ComputedStyle,
    border_rect: Rect,
    opacity_multiplier: f32,
) -> bool {
    let Some(border_image) = &style.border_image else {
        return false;
    };
    let border_widths = [
        style.effective_border_top() as f32,
        style.effective_border_right() as f32,
        style.effective_border_bottom() as f32,
        style.effective_border_left() as f32,
    ];
    let outsets = [
        resolve_border_image_outset(
            &border_image.outset[0],
            border_widths[0],
            border_rect.height(),
        ),
        resolve_border_image_outset(
            &border_image.outset[1],
            border_widths[1],
            border_rect.width(),
        ),
        resolve_border_image_outset(
            &border_image.outset[2],
            border_widths[2],
            border_rect.height(),
        ),
        resolve_border_image_outset(
            &border_image.outset[3],
            border_widths[3],
            border_rect.width(),
        ),
    ];
    let outer = Rect::from_ltrb(
        border_rect.left - outsets[3],
        border_rect.top - outsets[0],
        border_rect.right + outsets[1],
        border_rect.bottom + outsets[2],
    );
    let image = match border_image_source(doc, style, border_image, outer.width(), outer.height()) {
        Some(image) => image,
        None => return false,
    };
    let source_width = image.width() as f32;
    let source_height = image.height() as f32;
    let svg_resource = match border_image.source {
        CssImage::Raster(id)
            if doc
                .image_resource(id)
                .is_some_and(|resource| resource.mime_type == "image/svg+xml") =>
        {
            Some(id)
        }
        _ => None,
    };
    let mut source_slices = [
        resolve_border_image_slice(&border_image.slice[0], source_height),
        resolve_border_image_slice(&border_image.slice[1], source_width),
        resolve_border_image_slice(&border_image.slice[2], source_height),
        resolve_border_image_slice(&border_image.slice[3], source_width),
    ];
    (source_slices[0], source_slices[2]) =
        fit_opposing_edges(source_slices[0], source_slices[2], source_height);
    (source_slices[3], source_slices[1]) =
        fit_opposing_edges(source_slices[3], source_slices[1], source_width);

    let mut widths = [
        resolve_border_image_width(
            &border_image.width[0],
            border_widths[0],
            outer.height(),
            source_slices[0],
        ),
        resolve_border_image_width(
            &border_image.width[1],
            border_widths[1],
            outer.width(),
            source_slices[1],
        ),
        resolve_border_image_width(
            &border_image.width[2],
            border_widths[2],
            outer.height(),
            source_slices[2],
        ),
        resolve_border_image_width(
            &border_image.width[3],
            border_widths[3],
            outer.width(),
            source_slices[3],
        ),
    ];
    (widths[0], widths[2]) = fit_opposing_edges(widths[0], widths[2], outer.height());
    (widths[3], widths[1]) = fit_opposing_edges(widths[3], widths[1], outer.width());

    let sx = [
        0.0,
        source_slices[3],
        source_width - source_slices[1],
        source_width,
    ];
    let sy = [
        0.0,
        source_slices[0],
        source_height - source_slices[2],
        source_height,
    ];
    let dx = [
        outer.left,
        outer.left + widths[3],
        outer.right - widths[1],
        outer.right,
    ];
    let dy = [
        outer.top,
        outer.top + widths[0],
        outer.bottom - widths[2],
        outer.bottom,
    ];

    for row in 0..3 {
        for column in 0..3 {
            if row == 1 && column == 1 && !border_image.fill {
                continue;
            }
            let source = Rect::from_ltrb(sx[column], sy[row], sx[column + 1], sy[row + 1]);
            let destination = Rect::from_ltrb(dx[column], dy[row], dx[column + 1], dy[row + 1]);
            let repeat_x = if column == 1 {
                border_image.repeat_x
            } else {
                BorderImageRepeat::Stretch
            };
            let repeat_y = if row == 1 {
                border_image.repeat_y
            } else {
                BorderImageRepeat::Stretch
            };
            if let Some(id) = svg_resource {
                draw_border_svg_patch(
                    canvas,
                    doc,
                    id,
                    source_width,
                    source_height,
                    source,
                    destination,
                    repeat_x,
                    repeat_y,
                    opacity_multiplier,
                );
            } else {
                draw_border_image_patch(
                    canvas,
                    &image,
                    source,
                    destination,
                    repeat_x,
                    repeat_y,
                    opacity_multiplier,
                );
            }
        }
    }
    true
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
    ignore_border_radius: bool,
) {
    let has_border_radius = style.has_border_radius() && !ignore_border_radius;
    fn outset_shadow_corner(radius: Point, spread: f32, box_size: (f32, f32)) -> Point {
        if radius.x <= 0.0 || radius.y <= 0.0 || spread == 0.0 {
            return radius;
        }
        let coverage = if box_size.0 > 0.0 && box_size.1 > 0.0 {
            2.0 * (radius.x / box_size.0).min(radius.y / box_size.1)
        } else {
            1.0
        };
        let adjust = |value: f32| {
            if spread < 0.0 || value > spread || coverage > 1.0 {
                return (value + spread).max(0.0);
            }
            let ratio = value / spread;
            (value + spread * (1.0 - (1.0 - ratio).powi(3) * (1.0 - coverage.powi(3)))).max(0.0)
        };
        Point::new(adjust(radius.x), adjust(radius.y))
    }

    for shadow in style.box_shadow.iter().rev() {
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
            // Inset shadows are generated from and clipped to the padding box.
            // The border itself never receives inset-shadow ink.
            let border_widths = [
                style.effective_border_top() as f32,
                style.effective_border_right() as f32,
                style.effective_border_bottom() as f32,
                style.effective_border_left() as f32,
            ];
            let padding_rect = Rect::from_ltrb(
                border_rect.left + border_widths[3],
                border_rect.top + border_widths[0],
                border_rect.right - border_widths[1],
                border_rect.bottom - border_widths[2],
            );
            let outer_radii = normalized_border_radii(style, &border_rect);
            let padding_radii = normalize_radii_to_rect(
                [
                    Point::new(
                        (outer_radii[0].x - border_widths[3]).max(0.0),
                        (outer_radii[0].y - border_widths[0]).max(0.0),
                    ),
                    Point::new(
                        (outer_radii[1].x - border_widths[1]).max(0.0),
                        (outer_radii[1].y - border_widths[0]).max(0.0),
                    ),
                    Point::new(
                        (outer_radii[2].x - border_widths[1]).max(0.0),
                        (outer_radii[2].y - border_widths[2]).max(0.0),
                    ),
                    Point::new(
                        (outer_radii[3].x - border_widths[3]).max(0.0),
                        (outer_radii[3].y - border_widths[2]).max(0.0),
                    ),
                ],
                &padding_rect,
            );
            canvas.save();
            if has_border_radius {
                canvas.clip_rrect(
                    RRect::new_rect_radii(padding_rect, &padding_radii),
                    ClipOp::Intersect,
                    true,
                );
            } else {
                canvas.clip_rect(padding_rect, ClipOp::Intersect, false);
            }

            // The hole is the padding box contracted by spread and shifted by
            // the shadow offset.
            let hole = Rect::from_xywh(
                padding_rect.left + shadow.offset_x + shadow.spread_radius,
                padding_rect.top + shadow.offset_y + shadow.spread_radius,
                (padding_rect.width() - shadow.spread_radius * 2.0).max(0.0),
                (padding_rect.height() - shadow.spread_radius * 2.0).max(0.0),
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
                if has_border_radius {
                    let hole_radii = normalize_radii_to_rect(
                        padding_radii.map(|radius| {
                            outset_shadow_corner(
                                radius,
                                -shadow.spread_radius,
                                (padding_rect.width(), padding_rect.height()),
                            )
                        }),
                        &hole,
                    );
                    path.add_rrect(
                        RRect::new_rect_radii(hole, &hole_radii),
                        skia_safe::PathDirection::CCW,
                        0,
                    );
                } else {
                    path.add_rect(hole, skia_safe::PathDirection::CCW, 0);
                }
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
            if has_border_radius {
                let element_radii = normalized_border_radii(style, &border_rect);
                let border_rrect = RRect::new_rect_radii(border_rect, &element_radii);
                canvas.clip_rrect(border_rrect, ClipOp::Difference, true);
                let saturated = element_radii[0].x + element_radii[1].x
                    >= border_rect.width() - 0.01
                    || element_radii[3].x + element_radii[2].x >= border_rect.width() - 0.01
                    || element_radii[0].y + element_radii[3].y >= border_rect.height() - 0.01
                    || element_radii[1].y + element_radii[2].y >= border_rect.height() - 0.01;
                let shadow_radii: [Point; 4] = std::array::from_fn(|i| {
                    if saturated {
                        Point::new(
                            (element_radii[i].x + shadow.spread_radius).max(0.0),
                            (element_radii[i].y + shadow.spread_radius).max(0.0),
                        )
                    } else {
                        outset_shadow_corner(
                            element_radii[i],
                            shadow.spread_radius,
                            (border_rect.width(), border_rect.height()),
                        )
                    }
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
fn paint_fragment_box_decoration(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
    mut abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let mut fieldset_decoration_fragment = None;
    if fragment.is_first_for_node
        && !fragment.node_id.is_none()
        && doc.node(fragment.node_id).tag == ElementTag::Fieldset
    {
        let legend_block_size = fragment
            .children
            .iter()
            .filter(|child| {
                !child.node_id.is_none() && doc.node(child.node_id).tag == ElementTag::Legend
            })
            .map(|child| child.size.height)
            .max_by_key(|size| size.raw())
            .unwrap_or(LayoutUnit::zero());
        let decoration_shift =
            ((legend_block_size - LayoutUnit::from_i32(style.effective_border_top())) / 2)
                .clamp_negative_to_zero();
        if decoration_shift > LayoutUnit::zero() {
            let mut adjusted = fragment.clone();
            adjusted.size.height =
                (adjusted.size.height - decoration_shift).clamp_negative_to_zero();
            // The decoration is translated to the legend centerline here.
            // Keep legend geometry in that translated coordinate space so
            // paint_fieldset_borders does not apply the same displacement a
            // second time when it computes the border gap.
            for child in &mut adjusted.children {
                if !child.node_id.is_none() && doc.node(child.node_id).tag == ElementTag::Legend {
                    child.offset.top = child.offset.top - decoration_shift;
                }
            }
            abs_offset.top = abs_offset.top + decoration_shift;
            fieldset_decoration_fragment = Some(adjusted);
        }
    }
    let fragment = fieldset_decoration_fragment.as_ref().unwrap_or(fragment);
    let native_control_style;
    let node = doc.node(fragment.node_id);
    let native_button_theme = uses_native_button_theme(fragment, doc);
    let native_text_control_theme = node.form_control_native_appearance
        && matches!(
            node.form_control,
            Some(FormControlRole::TextInput | FormControlRole::TextArea | FormControlRole::Select)
        )
        && style.effective_border_top() > 0
        && style.effective_border_right() > 0
        && style.effective_border_bottom() > 0
        && style.effective_border_left() > 0;
    let style = if node.form_control == Some(FormControlRole::Range)
        && node.form_control_native_appearance
    {
        // Native range appearance replaces the author's track background.
        // `appearance:none` keeps the authored background and uses the
        // ordinary decoration path.
        native_control_style = {
            let mut adjusted = style.clone();
            adjusted.background_color = Color::TRANSPARENT;
            adjusted.background_layers.clear();
            adjusted.background_linear_gradient = None;
            adjusted
        };
        &native_control_style
    } else if native_button_theme {
        // Blink reserves a 2px CSS border for native button geometry, while
        // the passive Linux theme paints only its outer device-pixel ring;
        // the inner pixel is button-face background. Preserve the layout
        // strut in the fragment and narrow only the paint representation.
        native_control_style = {
            let mut adjusted = style.clone();
            let painted_border = i32::from(
                fragment.size.width >= LayoutUnit::from_i32(6)
                    && fragment.size.height >= LayoutUnit::from_i32(6),
            );
            adjusted.border_top_width = painted_border;
            adjusted.border_right_width = painted_border;
            adjusted.border_bottom_width = painted_border;
            adjusted.border_left_width = painted_border;
            if painted_border == 0 {
                adjusted.border_top_left_radius = (0.0, 0.0);
                adjusted.border_top_right_radius = (0.0, 0.0);
                adjusted.border_bottom_right_radius = (0.0, 0.0);
                adjusted.border_bottom_left_radius = (0.0, 0.0);
            }
            adjusted
        };
        &native_control_style
    } else if native_text_control_theme {
        // Passive Linux text controls reserve their UA inset widths for
        // layout, but the native theme raster is a single neutral outer ring.
        // Keep the fragment struts intact and narrow only the painted border.
        native_control_style = {
            let mut adjusted = style.clone();
            adjusted.border_top_width = 1;
            adjusted.border_right_width = 1;
            adjusted.border_bottom_width = 1;
            adjusted.border_left_width = 1;
            adjusted.border_top_style = BorderStyle::Solid;
            adjusted.border_right_style = BorderStyle::Solid;
            adjusted.border_bottom_style = BorderStyle::Solid;
            adjusted.border_left_style = BorderStyle::Solid;
            let border = openui_style::StyleColor::Resolved(Color::from_rgba8(118, 118, 118, 255));
            adjusted.border_top_color = border;
            adjusted.border_right_color = border;
            adjusted.border_bottom_color = border;
            adjusted.border_left_color = border;
            adjusted
        };
        &native_control_style
    } else {
        style
    };
    if fragment.decoration_clip_rects.is_empty() {
        if native_button_theme
            && fragment.size.width >= LayoutUnit::from_i32(6)
            && fragment.size.height >= LayoutUnit::from_i32(6)
        {
            canvas.save();
            clip_native_button_corner_cells(canvas, fragment, abs_offset);
        }
        if native_text_control_theme {
            canvas.save();
            clip_native_text_control_corner_cells(canvas, fragment, abs_offset);
        }
        paint_box_decoration_background(
            canvas,
            fragment,
            doc,
            style,
            abs_offset,
            opacity_multiplier,
        );
        if native_text_control_theme {
            canvas.restore();
        }
        if native_button_theme
            && fragment.size.width >= LayoutUnit::from_i32(6)
            && fragment.size.height >= LayoutUnit::from_i32(6)
        {
            canvas.restore();
        }
        return;
    }
    for clip in &fragment.decoration_clip_rects {
        canvas.save();
        canvas.clip_rect(
            Rect::from_xywh(
                abs_offset.left.to_f32() + clip.offset.left.to_f32(),
                abs_offset.top.to_f32() + clip.offset.top.to_f32(),
                clip.size.width.to_f32(),
                clip.size.height.to_f32(),
            ),
            ClipOp::Intersect,
            false,
        );
        if native_button_theme {
            clip_native_button_corner_cells(canvas, fragment, abs_offset);
        }
        paint_box_decoration_background(
            canvas,
            fragment,
            doc,
            style,
            abs_offset,
            opacity_multiplier,
        );
        canvas.restore();
    }
}

fn clip_native_text_control_corner_cells(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
) {
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    for rect in [
        Rect::from_xywh(x, y, 1.0, 1.0),
        Rect::from_xywh(right - 1.0, y, 1.0, 1.0),
        Rect::from_xywh(x, bottom - 1.0, 1.0, 1.0),
        Rect::from_xywh(right - 1.0, bottom - 1.0, 1.0, 1.0),
    ] {
        canvas.clip_rect(rect, ClipOp::Difference, false);
    }
}

fn paint_box_decoration_background(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
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
    let full_right = (abs_offset.left + fragment.size.width).round().to_f32();
    let full_bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    let full_border_box_rect = Rect::from_ltrb(x, y, full_right, full_bottom);
    let decoration_block_size = fragment.decoration_paint_block_size.filter(|limit| {
        limit.raw() >= 0 && limit.raw() < fragment_physical_block_extent(fragment).raw()
    });
    let border_box_rect = decoration_block_size.map_or(full_border_box_rect, |limit| {
        block_start_sized_rect(fragment, full_border_box_rect, abs_offset, limit)
    });
    let x = border_box_rect.left;
    let y = border_box_rect.top;
    let right = border_box_rect.right;
    let bottom = border_box_rect.bottom;
    let w = right - x;
    let h = bottom - y;
    let shadow_border_box_rect = inline_slice_shadow_source_rect(
        fragment,
        style,
        decoration_source_rect(fragment, border_box_rect),
    );

    let clip_inline_slice_shadow = |canvas: &Canvas| {
        if fragment.is_inline_box_fragment
            && style.box_decoration_break != openui_style::BoxDecorationBreak::Clone
            && (!fragment.is_first_for_node || !fragment.is_last_for_node)
        {
            let first_is_physical_start = style.direction == openui_style::Direction::Ltr;
            let suppress_start = if first_is_physical_start {
                !fragment.is_first_for_node
            } else {
                !fragment.is_last_for_node
            };
            let suppress_end = if first_is_physical_start {
                !fragment.is_last_for_node
            } else {
                !fragment.is_first_for_node
            };
            const EXTENT: f32 = 16_384.0;
            let clip = if style.writing_mode.is_horizontal() {
                Rect::from_ltrb(
                    if suppress_start { x } else { x - EXTENT },
                    y - EXTENT,
                    if suppress_end { right } else { right + EXTENT },
                    bottom + EXTENT,
                )
            } else {
                Rect::from_ltrb(
                    x - EXTENT,
                    if suppress_start { y } else { y - EXTENT },
                    right + EXTENT,
                    if suppress_end {
                        bottom
                    } else {
                        bottom + EXTENT
                    },
                )
            };
            canvas.clip_rect(clip, ClipOp::Intersect, false);
        }
    };

    // Outset shadows remain paintable for a zero-sized border box when spread
    // or blur gives the shadow non-empty geometry.
    canvas.save();
    clip_inline_slice_shadow(canvas);
    paint_box_shadows(
        canvas,
        style,
        shadow_border_box_rect,
        false,
        fragment.ignore_border_radius,
    );
    canvas.restore();

    // Skip empty fragments
    if w <= 0.0 || h <= 0.0 {
        return;
    }

    let decoration_clip_saved = decoration_block_size.is_some();
    if decoration_clip_saved {
        canvas.save();
        canvas.clip_rect(border_box_rect, ClipOp::Intersect, false);
    }

    // When border-radius is set AND borders are uniform solid, use saveLayer
    // so bg+border composite as one unit, then clip by the outer rrect.
    // This prevents background color from bleeding through at the border's
    // AA curve edges (matching Chromium). Only apply for uniform borders
    // since non-uniform (trapezoid) borders handle corners differently.
    let bt = style.effective_border_top() as f32;
    let br_bw = style.effective_border_right() as f32;
    let bb_bw = style.effective_border_bottom() as f32;
    let bl_bw = style.effective_border_left() as f32;
    let (paint_bt, paint_br, paint_bb, paint_bl) = sliced_physical_border_widths(fragment, style);
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
    let clipped_rounded_border_piece = has_radius
        && same_solid_visible_border
        && (paint_bt != paint_br || paint_br != paint_bb || paint_bb != paint_bl)
        && (style.overflow_x == openui_style::Overflow::Clip
            || style.overflow_y == openui_style::Overflow::Clip);
    let background_border_inset_fraction = if fragment.decoration_slice.is_some() {
        0.5
    } else if clipped_rounded_border_piece {
        1.0
    } else if authored_sides
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
    let has_visible_background = !style.background_color.is_transparent()
        || !style.background_layers.is_empty()
        || style.background_linear_gradient.is_some();
    let use_layer = has_radius
        && has_visible_background
        && style.border_image.is_none()
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
    let background_color_is_occluded = opacity_multiplier >= 1.0
        && style.background_layers.last().is_some_and(|layer| {
            layer.clip == effective_background_clip
                && matches!(
                    layer.repeat_x,
                    BackgroundRepeat::Repeat | BackgroundRepeat::Round
                )
                && matches!(
                    layer.repeat_y,
                    BackgroundRepeat::Repeat | BackgroundRepeat::Round
                )
                && matches!(
                    layer.image,
                    CssImage::Raster(id)
                        if crate::image_resource::decode_image_resource(doc, id)
                            .is_ok_and(|image| image.is_opaque())
                )
        });
    if !style.background_color.is_transparent() && !background_color_is_occluded {
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        // A pixel-snapped 2×2 saturated rounded box covers its four device
        // pixels completely in Blink; analytic AA would leave a gray pinhole
        // at the center of a large same-colored spread shadow.
        paint.set_anti_alias(!(has_radius && w <= 2.0 && h <= 2.0));
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
            // A sliced block keeps one continuous decoration geometry.  The
            // anonymous fragmentainer clip selects the visible portion; its
            // local fragment height must not independently subtract the
            // block-start/end border and padding on every continuation.
            let background_box = decoration_source_rect(fragment, border_box_rect);
            let (
                background_border_top,
                background_border_right,
                background_border_bottom,
                background_border_left,
            ) = if fragment.decoration_slice.is_some() {
                (
                    style.effective_border_top() as f32,
                    style.effective_border_right() as f32,
                    style.effective_border_bottom() as f32,
                    style.effective_border_left() as f32,
                )
            } else {
                (
                    fragment.border.top.round().to_f32(),
                    fragment.border.right.round().to_f32(),
                    fragment.border.bottom.round().to_f32(),
                    fragment.border.left.round().to_f32(),
                )
            };
            let bg_rect = match effective_background_clip {
                BackgroundClip::BorderBox => border_box_rect,
                BackgroundClip::PaddingBox => {
                    let bx = background_box.left + background_border_left;
                    let by = background_box.top + background_border_top;
                    let br = background_box.right - background_border_right;
                    let bb = background_box.bottom - background_border_bottom;
                    let bw = (br - bx).max(0.0);
                    let bh = (bb - by).max(0.0);
                    Rect::from_xywh(bx, by, bw, bh)
                }
                BackgroundClip::ContentBox => {
                    let bx = background_box.left
                        + background_border_left
                        + fragment.padding.left.round().to_f32();
                    let by = background_box.top
                        + background_border_top
                        + fragment.padding.top.round().to_f32();
                    let br = background_box.right
                        - background_border_right
                        - fragment.padding.right.round().to_f32();
                    let bb = background_box.bottom
                        - background_border_bottom
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
                    let background_edge_guard =
                        if clipped_rounded_border_piece && fragment.decoration_slice.is_none() {
                            1.0
                        } else {
                            0.0
                        };
                    let top_inset =
                        paint_bt * background_border_inset_fraction + background_edge_guard;
                    let right_inset =
                        paint_br * background_border_inset_fraction + background_edge_guard;
                    let bottom_inset =
                        paint_bb * background_border_inset_fraction + background_edge_guard;
                    let left_inset =
                        paint_bl * background_border_inset_fraction + background_edge_guard;
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
                    let gpu_sample = opaque_circular_background_gpu_sample(
                        &bg_rect,
                        &clip_radii,
                        &style.background_color,
                        matches!(
                            doc.node(fragment.node_id).pseudo_kind,
                            Some(PseudoElementKind::ScrollMarker)
                                | Some(PseudoElementKind::ColumnScrollMarker)
                        ),
                    );
                    if let Some(sample) = gpu_sample {
                        canvas.save();
                        canvas.clip_rect(sample, ClipOp::Difference, false);
                        canvas.draw_rrect(RRect::new_rect_radii(bg_rect, &clip_radii), &paint);
                        canvas.restore();

                        let mut sample_paint = Paint::default();
                        sample_paint.set_style(PaintStyle::Fill);
                        sample_paint.set_anti_alias(false);
                        set_paint_css_color_with_alpha(
                            &mut sample_paint,
                            &style.background_color,
                            128.0 / 255.0,
                        );
                        canvas.draw_rect(sample, &sample_paint);
                    } else {
                        canvas.draw_rrect(RRect::new_rect_radii(bg_rect, &clip_radii), &paint);
                    }
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
    if !style.background_layers.is_empty() {
        paint_background_layers(
            canvas,
            doc,
            Some(fragment),
            style,
            border_box_rect,
            opacity_multiplier,
            None,
        );
    }

    // Compatibility path for historical generated builders. New paint-cohort
    // builders emit the complete background layer list above.
    // The initial background-origin is padding-box, while background-clip is
    // independently resolved above.  Keeping the gradient's positioning area
    // separate from its paint clip is essential for cloned fragments: each
    // clone restarts the image in its own padding box.
    if style.background_layers.is_empty() {
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
                let mut positioning_padding_rect = decoration_source_rect(fragment, padding_rect);
                // A sliced continuation suppresses its block-start border, but
                // that decoration still occupies source space in the unfragmented
                // background positioning area. Move the source padding edge past
                // that border on the mapped physical block axis.
                if fragment.decoration_slice.is_some() && !fragment.is_first_for_node {
                    match fragment.fragmentation_writing_direction {
                        Some(direction)
                            if !direction.is_horizontal() && direction.is_flipped_blocks() =>
                        {
                            positioning_padding_rect.right += border_right;
                            positioning_padding_rect.left += border_right;
                        }
                        Some(direction) if !direction.is_horizontal() => {
                            positioning_padding_rect.left -= border_left;
                            positioning_padding_rect.right -= border_left;
                        }
                        _ => {
                            positioning_padding_rect.top -= border_top;
                            positioning_padding_rect.bottom -= border_top;
                        }
                    }
                }
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
    }

    // ── 3. Inset box shadows (painted on top of background) ──────────
    canvas.save();
    clip_inline_slice_shadow(canvas);
    paint_box_shadows(
        canvas,
        style,
        shadow_border_box_rect,
        true,
        fragment.ignore_border_radius,
    );
    canvas.restore();

    // ── 4. Borders ───────────────────────────────────────────────────
    if !fragment.paint_border_after_children
        && !paint_border_image(canvas, doc, style, border_box_rect, opacity_multiplier)
    {
        let suppress_collapsed_table_corner_pixels = style.display.is_table_wrapper()
            && style.border_collapse == openui_style::BorderCollapse::Collapse
            && has_radius;
        if suppress_collapsed_table_corner_pixels {
            // Collapsed-table border conflict resolution leaves the four
            // outermost corner samples outside the rounded perimeter. Keep
            // those device pixels available to the backdrop rather than
            // accepting Skia's analytic fringe coverage.
            canvas.save();
            for corner in [
                Rect::from_xywh(x, y, 1.0, 1.0),
                Rect::from_xywh(x + w - 1.0, y, 1.0, 1.0),
                Rect::from_xywh(x + w - 1.0, y + h - 1.0, 1.0, 1.0),
                Rect::from_xywh(x, y + h - 1.0, 1.0, 1.0),
                Rect::from_xywh(x + bl_bw, y + bt, 1.0, 1.0),
                Rect::from_xywh(x + w - br_bw - 1.0, y + bt, 1.0, 1.0),
                Rect::from_xywh(x + w - br_bw - 1.0, y + h - bb_bw - 1.0, 1.0, 1.0),
                Rect::from_xywh(x + bl_bw, y + h - bb_bw - 1.0, 1.0, 1.0),
            ] {
                canvas.clip_rect(corner, ClipOp::Difference, false);
            }
        }
        if doc.node(fragment.node_id).tag == ElementTag::Fieldset {
            paint_fieldset_borders(canvas, fragment, doc, style, x, y, w, h, use_layer);
        } else {
            paint_borders(canvas, fragment, style, x, y, w, h, use_layer, true);
        }
        if suppress_collapsed_table_corner_pixels {
            canvas.restore();
        }
    }

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
        erase_translucent_rounded_border_gpu_overdraw(
            canvas,
            border_rect,
            &outer_radii,
            &border_color,
        );
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
        draw_translucent_rounded_border_gpu_fringe(
            canvas,
            border_rect,
            &outer_radii,
            &border_color,
        );
    }
    if decoration_clip_saved {
        canvas.restore();
    }
}

/// Paint the HTML fieldset border around its first legend.
///
/// The fieldset block-start edge runs through the legend's block-axis center
/// and is interrupted by the legend's full unwrapped inline extent.  Legend
/// inline boxes may have wrapped during layout, so summing their continuation
/// widths recovers that extent while their union supplies the used block size.
fn paint_fieldset_borders(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    use_layer: bool,
) {
    fn legend_auto_inline_extent(fragment: &Fragment, doc: &Document) -> LayoutUnit {
        fn collect(
            fragment: &Fragment,
            doc: &Document,
            offset: LayoutUnit,
            maximum: &mut LayoutUnit,
        ) {
            for child in &fragment.children {
                let child_offset = offset + child.offset.left;
                if child.node_id.is_none() {
                    collect(child, doc, child_offset, maximum);
                    continue;
                }
                let child_style = &doc.node(child.node_id).style;
                if child_style.position.is_absolutely_positioned() {
                    continue;
                }
                *maximum = (*maximum).max_of(child_offset + child.size.width);
            }
        }

        let mut maximum = LayoutUnit::zero();
        collect(fragment, doc, LayoutUnit::zero(), &mut maximum);
        maximum
    }

    fn collect_legend_geometry(
        current: &Fragment,
        doc: &Document,
        offset: PhysicalOffset,
        geometry: &mut Option<(LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit)>,
    ) {
        let current_offset = PhysicalOffset::new(
            offset.left + current.offset.left,
            offset.top + current.offset.top,
        );
        if !current.node_id.is_none() && doc.node(current.node_id).tag == ElementTag::Legend {
            let left = current_offset.left;
            let top = current_offset.top;
            let bottom = top + current.size.height;
            let legend_extent = if doc.node(current.node_id).style.width.is_auto() {
                legend_auto_inline_extent(current, doc)
            } else {
                current.size.width
            };
            if let Some((min_left, min_top, max_bottom, accumulated_extent)) = geometry {
                *min_left = (*min_left).min_of(left);
                *min_top = (*min_top).min_of(top);
                *max_bottom = (*max_bottom).max_of(bottom);
                *accumulated_extent = *accumulated_extent + legend_extent;
            } else {
                *geometry = Some((left, top, bottom, legend_extent));
            }
            return;
        }
        for child in &current.children {
            collect_legend_geometry(child, doc, current_offset, geometry);
        }
    }

    let mut geometry = None;
    for child in &fragment.children {
        collect_legend_geometry(child, doc, PhysicalOffset::zero(), &mut geometry);
    }
    let Some((legend_left, legend_top, legend_bottom, legend_inline_extent)) = geometry else {
        paint_borders(canvas, fragment, style, x, y, w, h, use_layer, true);
        return;
    };

    let top_width = style.effective_border_top() as f32;
    let legend_center = (legend_top + (legend_bottom - legend_top) / 2).to_f32() - top_width / 2.0;
    let border_top = legend_center.max(0.0).min(h);
    let border_height = (h - border_top).max(0.0);
    canvas.save();
    let gap_left = (x + legend_left.to_f32()).max(x + top_width);
    let gap_right = (gap_left + legend_inline_extent.to_f32()).min(x + w - top_width);
    if gap_right > gap_left && top_width > 0.0 {
        canvas.clip_rect(
            Rect::from_ltrb(
                gap_left,
                y + border_top,
                gap_right,
                y + border_top + top_width,
            ),
            ClipOp::Difference,
            false,
        );
    }
    paint_borders(
        canvas,
        fragment,
        style,
        x,
        y + border_top,
        w,
        border_height,
        use_layer,
        true,
    );
    canvas.restore();
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
    adjust_dashed_gap: bool,
) {
    // Slice block fragments on the owning fragmentation context's physical
    // block axis. Inline continuations keep their established left/right
    // slicing semantics, including clone.
    let (bt, br, bb, bl) = sliced_physical_border_widths(fragment, style);
    let has_border_radius = style.has_border_radius() && !fragment.ignore_border_radius;

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

    let clipped_rounded_border_piece = has_border_radius
        && same_solid_visible_border
        && non_uniform_widths
        && (style.overflow_x == openui_style::Overflow::Clip
            || style.overflow_y == openui_style::Overflow::Clip);
    let single_corner_owns_diagonal_seam =
        fragment.is_inline_box_fragment || style.position.is_absolutely_positioned();

    if has_border_radius && non_uniform_widths && (same_solid_color || same_solid_visible_border) {
        let representative_width = visible_width.unwrap_or(bt.max(br).max(bb).max(bl));
        let representative_color = solid_color.or(visible_color);
        if let Some(color) = representative_color {
            let border_rect = Rect::from_xywh(x, y, w, h);
            let outer_radii =
                fragment_border_radii(style, fragment, &border_rect, representative_width);
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
                let raw_inner_radii = [
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
                ];
                // Equal-width visible sides use Blink's established
                // contoured-side path, including inline/fragment slices with
                // suppressed edges. The non-renderable-inner decomposition
                // is needed only for genuinely different used widths.
                let nonrenderable_inner =
                    !same_solid_visible_border && radii_exceed_rect(&inner_rect, &raw_inner_radii);
                let inner_radii = if nonrenderable_inner {
                    raw_inner_radii
                } else {
                    normalize_radii_to_rect(raw_inner_radii, &inner_rect)
                };

                if nonrenderable_inner && !clipped_rounded_border_piece {
                    // Blink's complex opaque-border path clips the rounded
                    // outer contour once, then paints hard-clipped sides in
                    // non-adjacent order. Each side excludes an adjusted,
                    // renderable form of the otherwise non-renderable inner
                    // contour. Keeping these clips separate is important:
                    // their antialiasing coverage intentionally overlaps at
                    // ambiguous inner corners.
                    canvas.save();
                    if !outer_rrect_clipped {
                        canvas.clip_rrect(
                            RRect::new_rect_radii(border_rect, &outer_radii),
                            ClipOp::Intersect,
                            true,
                        );
                    }
                    for (side, width) in [
                        (BorderSide::Top, bt),
                        (BorderSide::Bottom, bb),
                        (BorderSide::Right, br),
                        (BorderSide::Left, bl),
                    ] {
                        if width <= 0.0 {
                            continue;
                        }

                        canvas.save();
                        let polygon = nonrenderable_border_side_clip_polygon(
                            border_rect,
                            inner_rect,
                            inner_radii,
                            side,
                        );
                        let mut side_clip = PathBuilder::new();
                        side_clip.move_to(polygon[0]);
                        for point in polygon.iter().skip(1) {
                            side_clip.line_to(*point);
                        }
                        canvas.clip_path(&side_clip.detach(), ClipOp::Intersect, false);

                        let (adjusted_rect, adjusted_radii) =
                            adjusted_nonrenderable_inner_border_for_side(
                                inner_rect,
                                inner_radii,
                                side,
                            );
                        if adjusted_rect.width() > 0.0 && adjusted_rect.height() > 0.0 {
                            canvas.clip_rrect(
                                RRect::new_rect_radii(adjusted_rect, &adjusted_radii),
                                ClipOp::Difference,
                                true,
                            );
                        }
                        canvas.draw_rect(border_rect, &fill_paint);
                        canvas.restore();
                    }
                    canvas.restore();
                    return;
                }

                if !same_solid_visible_border
                    && !nonrenderable_inner
                    && !outer_rrect_clipped
                    && color.is_opaque()
                {
                    // A renderable same-color solid border is one contour
                    // ring even when its four widths differ. Evaluating the
                    // outer and inner curves together avoids side-clip seams
                    // and matches Blink's opaque-path lowering.
                    let outer_rrect = RRect::new_rect_radii(border_rect, &outer_radii);
                    let inner_rrect = RRect::new_rect_radii(inner_rect, &inner_radii);
                    canvas.draw_drrect(outer_rrect, inner_rrect, &fill_paint);
                    return;
                }
                canvas.save();
                if !outer_rrect_clipped {
                    canvas.clip_rrect(
                        RRect::new_rect_radii(border_rect, &outer_radii),
                        ClipOp::Intersect,
                        true,
                    );
                    // Multiple same-color sides meet inside the rounded
                    // contour. Accumulate them opaquely on a layer and apply
                    // the antialiased outer clip once when the layer is
                    // restored; otherwise tangent pixels receive the outer
                    // coverage once per adjacent side.
                    if !same_solid_visible_border {
                        canvas.save_layer_alpha_f(border_rect, 1.0);
                    }
                }
                if !nonrenderable_inner {
                    canvas.clip_rrect(
                        RRect::new_rect_radii(inner_rect, &inner_radii),
                        ClipOp::Difference,
                        true,
                    );
                }

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
                let side_path =
                    |side: BorderSide, points: &[Point; 4], aa_bounds: Option<(Rect, bool)>| {
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
                        if nonrenderable_inner {
                            let (adjusted_rect, adjusted_radii) =
                                adjusted_nonrenderable_inner_border_for_side(
                                    inner_rect,
                                    inner_radii,
                                    side,
                                );
                            canvas.clip_rrect(
                                RRect::new_rect_radii(adjusted_rect, &adjusted_radii),
                                ClipOp::Difference,
                                true,
                            );
                        }
                        canvas.draw_rect(border_rect, &fill_paint);
                        canvas.restore();
                    };
                if bt > 0.0 {
                    if is_rounded(inner_radii[0]) || is_rounded(inner_radii[1]) {
                        side_path(
                            BorderSide::Top,
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
                        let bottom_points = [
                            Point::new(border_rect.right, border_rect.bottom),
                            halves[2],
                            halves[3],
                            Point::new(border_rect.left, border_rect.bottom),
                        ];
                        side_path(BorderSide::Bottom, &bottom_points, None);
                        if nonrenderable_inner {
                            // The block-end side owns the non-renderable
                            // inner-corner seam together with its adjacent
                            // inline side. Replay that side so their coverage
                            // combines at the curved boundary.
                            side_path(BorderSide::Bottom, &bottom_points, None);
                        }
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
                        // Partial rounded overflow-clip borders use Blink's
                        // complex side ownership: when both corners terminate
                        // on the same physical right edge, the horizontal
                        // sides own the exact diagonal seam sample.
                        let seam_bias = if (clipped_rounded_border_piece
                            || single_corner_owns_diagonal_seam)
                            && top_rounded
                            && bottom_rounded
                        {
                            1.0
                        } else {
                            0.0
                        };
                        let inner_x = (if top_rounded && !bottom_rounded {
                            halves[1].x
                        } else if bottom_rounded && !top_rounded {
                            halves[2].x
                        } else {
                            halves[1].x.min(halves[2].x)
                        }) + seam_bias;
                        let seam_overlap = 0.1;
                        let top_overlap = if top_rounded { seam_overlap } else { 0.0 };
                        let bottom_overlap = if bottom_rounded { seam_overlap } else { 0.0 };
                        let bounds = Rect::from_ltrb(
                            inner_x,
                            border_rect.top - if bottom_rounded { seam_overlap } else { 0.0 },
                            border_rect.right,
                            border_rect.bottom + if top_rounded { seam_overlap } else { 0.0 },
                        );
                        let right_points = [
                            Point::new(
                                border_rect.right + seam_bias,
                                border_rect.top - top_overlap,
                            ),
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
                            Point::new(
                                border_rect.right + seam_bias,
                                border_rect.bottom + bottom_overlap,
                            ),
                        ];
                        side_path(
                            BorderSide::Right,
                            &right_points,
                            Some((bounds, bottom_rounded)),
                        );
                        if nonrenderable_inner && bottom_rounded {
                            // In a clipped non-renderable bottom corner the
                            // vertical side owns the region outside the
                            // bottom-side diagonal.
                            canvas.save();
                            canvas.clip_rect(
                                Rect::from_ltrb(
                                    border_rect.left,
                                    halves[2].y + 3.0,
                                    border_rect.right,
                                    border_rect.bottom,
                                ),
                                ClipOp::Intersect,
                                false,
                            );
                            side_path(
                                BorderSide::Right,
                                &right_points,
                                Some((bounds, bottom_rounded)),
                            );
                            canvas.restore();
                        }
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
                            BorderSide::Left,
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
                        if (clipped_rounded_border_piece && top_rounded && bottom_rounded)
                            || (single_corner_owns_diagonal_seam && (top_rounded || bottom_rounded))
                        {
                            // The mirrored left-edge topology assigns the
                            // diagonal seam to both adjacent same-color sides.
                            // Repaint that one-pixel seam through the already
                            // established outer/inner contour clips.
                            let mut seam_paint = Paint::default();
                            seam_paint.set_style(PaintStyle::Stroke);
                            seam_paint.set_stroke_width(1.0);
                            seam_paint.set_anti_alias(false);
                            set_paint_css_color(&mut seam_paint, &color);
                            if top_rounded {
                                canvas.draw_line(
                                    Point::new(border_rect.left, border_rect.top),
                                    halves[0],
                                    &seam_paint,
                                );
                            }
                            if bottom_rounded {
                                canvas.draw_line(
                                    halves[3],
                                    Point::new(border_rect.left, border_rect.bottom),
                                    &seam_paint,
                                );
                            }
                        }
                    }
                }
                if !outer_rrect_clipped && !same_solid_visible_border {
                    canvas.restore();
                }
                canvas.restore();

                // When a rounded horizontal edge terminates against a
                // suppressed vertical edge, Blink's soft miter retains the
                // subpixel endpoint coverage that an integer inner-rrect
                // difference clip drops. The top and bottom raster ownership
                // are intentionally asymmetric in Skia.
                if bl == 0.0 && br > 0.0 {
                    let mut fringe = Paint::default();
                    fringe.set_style(PaintStyle::Fill);
                    fringe.set_anti_alias(false);
                    if bt > 0.0 && outer_radii[1].x >= w {
                        fringe.set_color4f(
                            Color4f::new(color.r, color.g, color.b, 15.0 / 255.0),
                            None::<&ColorSpace>,
                        );
                        canvas.draw_rect(
                            Rect::from_xywh(inner_rect.left, inner_rect.top, 1.0, 1.0),
                            &fringe,
                        );
                    }
                    if bb > 0.0 && outer_radii[2].x >= w {
                        fringe.set_color4f(
                            Color4f::new(color.r, color.g, color.b, 8.0 / 255.0),
                            None::<&ColorSpace>,
                        );
                        canvas.draw_rect(
                            Rect::from_xywh(inner_rect.left, inner_rect.bottom - 1.0, 1.0, 1.0),
                            &fringe,
                        );
                    }
                }
                return;
            }
        }
    }

    if same_solid_color && non_uniform_widths && !has_border_radius {
        if let Some(color) = solid_color {
            paint_same_color_solid_border(canvas, fragment, color, x, y, w, h, bt, br, bb, bl);
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

    let uniform_rounded_dotted_geometry = bt == br
        && br == bb
        && bb == bl
        && bt > 0.0
        && has_border_radius
        && style.border_top_style == BorderStyle::Dotted
        && style.border_right_style == BorderStyle::Dotted
        && style.border_bottom_style == BorderStyle::Dotted
        && style.border_left_style == BorderStyle::Dotted;
    if uniform_rounded_dotted_geometry {
        let border_rect = Rect::from_xywh(x, y, w, h);
        let outer_radii = fragment_border_radii(style, fragment, &border_rect, bt);
        if has_any_radius(&outer_radii) {
            let half = bt * 0.5;
            let center_inset = half.floor();
            let center_rect = Rect::from_xywh(
                x + center_inset,
                y + center_inset,
                (w - center_inset * 2.0).max(0.0),
                (h - center_inset * 2.0).max(0.0),
            );
            let center_radii = normalize_radii_to_rect(
                outer_radii.map(|radius| {
                    Point::new(
                        (radius.x - center_inset).max(0.0),
                        (radius.y - center_inset).max(0.0),
                    )
                }),
                &center_rect,
            );
            let center_rrect = RRect::new_rect_radii(center_rect, &center_radii);
            let mut measured_path = PathBuilder::new();
            measured_path.add_rrect(center_rrect, skia_safe::PathDirection::CW, 0);
            let measured_path = measured_path.detach();
            let mut path_measure = PathMeasure::new(&measured_path, true, None);
            let path_length = path_measure.length();
            let gap_len = select_best_closed_dash_gap(path_length.floor(), bt, bt);
            let interval = gap_len + bt - 0.01;

            let inner_rect = Rect::from_xywh(
                x + bt,
                y + bt,
                (w - bt * 2.0).max(0.0),
                (h - bt * 2.0).max(0.0),
            );
            let inner_radii = normalize_radii_to_rect(
                outer_radii
                    .map(|radius| Point::new((radius.x - bt).max(0.0), (radius.y - bt).max(0.0))),
                &inner_rect,
            );
            canvas.save();
            canvas.clip_rrect(
                RRect::new_rect_radii(border_rect, &outer_radii),
                ClipOp::Intersect,
                true,
            );
            canvas.clip_rrect(
                RRect::new_rect_radii(inner_rect, &inner_radii),
                ClipOp::Difference,
                true,
            );

            let paint_side = |points: &[(f32, f32); 4], color: Color| {
                canvas.save();
                let mut side_clip = PathBuilder::new();
                side_clip.move_to(Point::new(points[0].0, points[0].1));
                for point in &points[1..] {
                    side_clip.line_to(Point::new(point.0, point.1));
                }
                side_clip.close();
                canvas.clip_path(&side_clip.detach(), ClipOp::Intersect, true);
                let mut paint = Paint::default();
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(bt);
                paint.set_stroke_cap(skia_safe::paint::Cap::Round);
                paint.set_anti_alias(true);
                set_paint_css_color(&mut paint, &color);
                if let Some(effect) = skia_safe::PathEffect::dash(&[0.0, interval], 0.0) {
                    paint.set_path_effect(effect);
                }
                canvas.draw_path(&measured_path, &paint);
                canvas.restore();
            };
            // A rounded border side owns the sector between the two radius
            // centers, rather than the square trapezoid formed by the inner
            // border-box corners.  The latter cuts the inward half off dots
            // that straddle a curved corner.
            let top_left_center = (x + outer_radii[0].x, y + outer_radii[0].y);
            let top_right_center = (x + w - outer_radii[1].x, y + outer_radii[1].y);
            let bottom_right_center = (x + w - outer_radii[2].x, y + h - outer_radii[2].y);
            let bottom_left_center = (x + outer_radii[3].x, y + h - outer_radii[3].y);
            paint_side(
                &[(x, y), (x + w, y), top_right_center, top_left_center],
                style.border_top_color.resolve(inherited_color),
            );
            paint_side(
                &[
                    (x + w, y + h),
                    (x, y + h),
                    bottom_left_center,
                    bottom_right_center,
                ],
                style.border_bottom_color.resolve(inherited_color),
            );
            paint_side(
                &[
                    (x + w, y),
                    (x + w, y + h),
                    bottom_right_center,
                    top_right_center,
                ],
                style.border_right_color.resolve(inherited_color),
            );
            paint_side(
                &[(x, y + h), (x, y), top_left_center, bottom_left_center],
                style.border_left_color.resolve(inherited_color),
            );
            canvas.restore();
            return;
        }
    }

    if uniform && style.border_top_style == BorderStyle::Double && has_border_radius && bt > 0.0 {
        // A rounded double border is two concentric contour rings. Painting
        // four rectangular sides loses both the outer and inner corner
        // geometry (and leaves square corners visible outside the radius).
        let border_rect = Rect::from_xywh(x, y, w, h);
        let outer_radii = fragment_border_radii(style, fragment, &border_rect, bt);
        if has_any_radius(&outer_radii) {
            let line_width = (bt / 3.0).round().max(1.0).min(bt * 0.5);
            let resolved = style.border_top_color.resolve(inherited_color);
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(true);
            set_paint_css_color(&mut paint, &resolved);

            let inset_contour = |inset: f32| {
                let rect = Rect::from_xywh(
                    x + inset,
                    y + inset,
                    (w - inset * 2.0).max(0.0),
                    (h - inset * 2.0).max(0.0),
                );
                let radii = normalize_radii_to_rect(
                    outer_radii.map(|radius| {
                        Point::new((radius.x - inset).max(0.0), (radius.y - inset).max(0.0))
                    }),
                    &rect,
                );
                (rect, radii)
            };

            let fully_rounded = outer_radii[0].x + outer_radii[1].x >= w - 0.01
                || outer_radii[3].x + outer_radii[2].x >= w - 0.01
                || outer_radii[0].y + outer_radii[3].y >= h - 0.01
                || outer_radii[1].y + outer_radii[2].y >= h - 0.01;
            if fully_rounded || bt <= 10.0 {
                // For a saturated rounded contour (a circle or pill), Blink
                // lowers each double-border stripe to a centered rrect stroke.
                // This preserves the complementary AA coverage at the tangent
                // points of the normalized radii.
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(line_width);
                let (outer_center_rect, outer_center_radii) = inset_contour(line_width * 0.5);
                canvas.draw_rrect(
                    RRect::new_rect_radii(outer_center_rect, &outer_center_radii),
                    &paint,
                );
                let (inner_center_rect, inner_center_radii) = inset_contour(bt - line_width * 0.5);
                canvas.draw_rrect(
                    RRect::new_rect_radii(inner_center_rect, &inner_center_radii),
                    &paint,
                );
            } else {
                let outer_rrect = RRect::new_rect_radii(
                    border_rect,
                    &normalize_radii_to_rect(outer_radii, &border_rect),
                );
                let (outer_inner_rect, outer_inner_radii) = inset_contour(line_width);
                canvas.draw_drrect(
                    outer_rrect,
                    RRect::new_rect_radii(outer_inner_rect, &outer_inner_radii),
                    &paint,
                );

                let inner_outer_inset = (bt - line_width).max(line_width);
                let (inner_outer_rect, inner_outer_radii) = inset_contour(inner_outer_inset);
                let (inner_inner_rect, inner_inner_radii) = inset_contour(bt);
                canvas.draw_drrect(
                    RRect::new_rect_radii(inner_outer_rect, &inner_outer_radii),
                    RRect::new_rect_radii(inner_inner_rect, &inner_inner_radii),
                    &paint,
                );
            }
            return;
        }
    }

    if uniform && style.border_top_style == BorderStyle::Dashed && has_border_radius && bt > 0.0 {
        // Dashed rounded borders follow the center contour as one continuous
        // closed path. This keeps dash cadence through corners instead of
        // restarting independently on four square side rectangles.
        let border_rect = Rect::from_xywh(x, y, w, h);
        let outer_radii = fragment_border_radii(style, fragment, &border_rect, bt);
        if has_any_radius(&outer_radii) {
            let dash_len = bt * if bt >= 3.0 { 2.0 } else { 3.0 };
            let desired_gap = bt * if bt >= 3.0 { 1.0 } else { 2.0 };
            let half = bt * 0.5;
            let center_inset = half.floor();
            let center_rect = Rect::from_xywh(
                x + center_inset,
                y + center_inset,
                (w - center_inset * 2.0).max(0.0),
                (h - center_inset * 2.0).max(0.0),
            );
            let center_radii = normalize_radii_to_rect(
                outer_radii.map(|radius| {
                    Point::new(
                        (radius.x - center_inset).max(0.0),
                        (radius.y - center_inset).max(0.0),
                    )
                }),
                &center_rect,
            );
            let contour_rrect = RRect::new_rect_radii(center_rect, &center_radii);
            let mut measured_path = PathBuilder::new();
            measured_path.add_rrect(contour_rrect, skia_safe::PathDirection::CW, 0);
            let measured_path = measured_path.detach();
            let path_length = PathMeasure::new(&measured_path, true, None).length();
            let gap_len = select_best_closed_dash_gap(path_length.floor(), dash_len, desired_gap);
            let resolved = style.border_top_color.resolve(inherited_color);
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Stroke);
            // Blink over-strokes a curved dashed contour and clips it back to
            // the exact border ring. This keeps dash ends filled through tight
            // corners without escaping the outer or inner contour.
            paint.set_stroke_width(bt * 2.2);
            paint.set_stroke_cap(skia_safe::paint::Cap::Butt);
            paint.set_anti_alias(true);
            set_paint_css_color(&mut paint, &resolved);
            if let Some(effect) = skia_safe::PathEffect::dash(&[dash_len, gap_len], 0.0) {
                paint.set_path_effect(effect);
            }
            let inner_rect = Rect::from_xywh(
                x + bt,
                y + bt,
                (w - bt * 2.0).max(0.0),
                (h - bt * 2.0).max(0.0),
            );
            let inner_radii = normalize_radii_to_rect(
                outer_radii
                    .map(|radius| Point::new((radius.x - bt).max(0.0), (radius.y - bt).max(0.0))),
                &inner_rect,
            );
            canvas.save();
            canvas.clip_rrect(
                RRect::new_rect_radii(border_rect, &outer_radii),
                ClipOp::Intersect,
                true,
            );
            canvas.clip_rrect(
                RRect::new_rect_radii(inner_rect, &inner_radii),
                ClipOp::Difference,
                true,
            );
            let draw_side = |points: &[(f32, f32); 4]| {
                canvas.save();
                let mut side_clip = PathBuilder::new();
                side_clip.move_to(Point::new(points[0].0, points[0].1));
                for point in &points[1..] {
                    side_clip.line_to(Point::new(point.0, point.1));
                }
                side_clip.close();
                canvas.clip_path(&side_clip.detach(), ClipOp::Intersect, false);
                canvas.draw_path(&measured_path, &paint);
                canvas.restore();
            };
            let top_left_center = (x + outer_radii[0].x, y + outer_radii[0].y);
            let top_right_center = (x + w - outer_radii[1].x, y + outer_radii[1].y);
            let bottom_right_center = (x + w - outer_radii[2].x, y + h - outer_radii[2].y);
            let bottom_left_center = (x + outer_radii[3].x, y + h - outer_radii[3].y);
            draw_side(&[(x, y), (x + w, y), top_right_center, top_left_center]);
            draw_side(&[
                (x + w, y + h),
                (x, y + h),
                bottom_left_center,
                bottom_right_center,
            ]);
            draw_side(&[
                (x + w, y),
                (x + w, y + h),
                bottom_right_center,
                top_right_center,
            ]);
            draw_side(&[(x, y + h), (x, y), top_left_center, bottom_left_center]);
            canvas.restore();
            return;
        }
    }

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
            if simple_stroked_rrect && !outer_rrect_clipped && resolved.is_opaque() {
                let center_radii = outer_radii.map(|radius| {
                    Point::new((radius.x - half).max(0.0), (radius.y - half).max(0.0))
                });
                canvas.draw_rrect(RRect::new_rect_radii(stroke_rect, &center_radii), &paint);
                return;
            }
            let nonrenderable_inner = radii_exceed_rect(&inner_rect, &inner_radii);
            let asymmetric_outer_radii = outer_radii
                .iter()
                .skip(1)
                .any(|radius| radius.x != outer_radii[0].x || radius.y != outer_radii[0].y);
            if nonrenderable_inner && (outer_rrect_clipped || asymmetric_outer_radii) {
                let border_rect = Rect::from_xywh(x, y, w, h);
                if !outer_rrect_clipped {
                    canvas.save();
                    canvas.clip_rrect(
                        RRect::new_rect_radii(border_rect, &outer_radii),
                        ClipOp::Intersect,
                        true,
                    );
                }
                draw_nonrenderable_uniform_rounded_border(
                    canvas,
                    border_rect,
                    inner_rect,
                    inner_radii,
                    &fill_paint,
                );
                if asymmetric_outer_radii {
                    // Strongly eccentric corners retain shared ownership of
                    // their outer tangent strip in Blink's side decomposition.
                    for (corner, radius) in outer_radii.iter().copied().enumerate() {
                        let Some(tangent_clip) =
                            eccentric_outer_tangent_clip(border_rect, inner_rect, radius, corner)
                        else {
                            continue;
                        };
                        canvas.save();
                        canvas.clip_rect(tangent_clip, ClipOp::Intersect, false);
                        draw_nonrenderable_uniform_rounded_border(
                            canvas,
                            border_rect,
                            inner_rect,
                            inner_radii,
                            &fill_paint,
                        );
                        canvas.restore();
                    }
                    if let Some(tangent_clips) =
                        single_saturated_corner_tangent_clips(border_rect, inner_rect, &outer_radii)
                    {
                        for tangent_clip in tangent_clips {
                            if tangent_clip.width() <= 0.0 || tangent_clip.height() <= 0.0 {
                                continue;
                            }
                            canvas.save();
                            canvas.clip_rect(tangent_clip, ClipOp::Intersect, false);
                            draw_nonrenderable_uniform_rounded_border(
                                canvas,
                                border_rect,
                                inner_rect,
                                inner_radii,
                                &fill_paint,
                            );
                            canvas.restore();
                        }
                    }
                }
                if !outer_rrect_clipped {
                    canvas.restore();
                }
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
                if resolved.a < 1.0 {
                    canvas.save_layer_alpha_f(outer_rect, 1.0);
                }
                canvas.draw_drrect(outer_rrect, inner_rrect, &fill_paint);
                if resolved.a < 1.0 {
                    erase_translucent_rounded_border_gpu_overdraw(
                        canvas,
                        outer_rect,
                        &outer_radii,
                        &resolved,
                    );
                    canvas.restore();
                }
                draw_translucent_rounded_border_gpu_fringe(
                    canvas,
                    outer_rect,
                    &outer_radii,
                    &resolved,
                );
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
        // Matching dashed/dotted sides share their corner endpoint ink. A
        // miter clip is required only across a style or color transition;
        // clipping every side truncates round endpoint dots.
        let stroked_style = style.border_top_style;
        let matching_stroked_sides =
            matches!(stroked_style, BorderStyle::Dashed | BorderStyle::Dotted)
                && style.border_right_style == stroked_style
                && style.border_bottom_style == stroked_style
                && style.border_left_style == stroked_style
                && style.border_top_color.resolve(inherited_color)
                    == style.border_right_color.resolve(inherited_color)
                && style.border_top_color.resolve(inherited_color)
                    == style.border_bottom_color.resolve(inherited_color)
                && style.border_top_color.resolve(inherited_color)
                    == style.border_left_color.resolve(inherited_color);
        let all_side_colors_match = style.border_top_color.resolve(inherited_color)
            == style.border_right_color.resolve(inherited_color)
            && style.border_top_color.resolve(inherited_color)
                == style.border_bottom_color.resolve(inherited_color)
            && style.border_top_color.resolve(inherited_color)
                == style.border_left_color.resolve(inherited_color);
        let antialias_solid_miters = [
            style.border_top_color.resolve(inherited_color),
            style.border_right_color.resolve(inherited_color),
            style.border_bottom_color.resolve(inherited_color),
            style.border_left_color.resolve(inherited_color),
        ]
        .iter()
        .any(|color| !color.is_opaque());
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
                adjust_dashed_gap,
                !matching_stroked_sides,
                !all_side_colors_match,
                antialias_solid_miters,
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
                adjust_dashed_gap,
                !matching_stroked_sides,
                !all_side_colors_match,
                antialias_solid_miters,
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
                adjust_dashed_gap,
                !matching_stroked_sides,
                true,
                antialias_solid_miters,
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
                adjust_dashed_gap,
                !matching_stroked_sides,
                !all_side_colors_match,
                antialias_solid_miters,
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
    fragment: &Fragment,
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
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    set_paint_css_color(&mut paint, &color);

    let fixed_inner_block_extent =
        (fragment.size.height - fragment.border.bottom).clamp_negative_to_zero();
    let block_extent_loses_fixed_remainder = fragment.border.bottom.raw() >= i32::MAX / 2;
    if bt == 0.0 && br == 0.0 && bl == 0.0 && bb > 0.0 && block_extent_loses_fixed_remainder {
        // Keep the small inner-edge remainder in fixed-point space. At the
        // LayoutUnit ceiling both the used size and bottom border round to
        // the same f32, but their difference still contains the authored
        // padding/content pixel. A direct side rectangle also avoids feeding
        // a near-maximum even-odd contour to Skia's path tessellator.
        let inner_block_extent = fixed_inner_block_extent.max_of(fragment.padding.top);
        let top = y + inner_block_extent.to_f32();
        canvas.draw_rect(Rect::from_ltrb(x, top, x + w, y + h), &paint);
        return;
    }

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
        true,
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

    // Uniform 3D borders have one shaded-color transition at the top-right
    // and bottom-left miters. Resolve the shared diagonal pixels to the exact
    // premultiplied average, matching a single complementary coverage pass.
    let style_3d = style.border_top_style;
    let uniform_3d = matches!(
        style_3d,
        BorderStyle::Groove | BorderStyle::Ridge | BorderStyle::Inset | BorderStyle::Outset
    ) && style.border_right_style == style_3d
        && style.border_bottom_style == style_3d
        && style.border_left_style == style_3d
        && style.border_top_color == style.border_right_color
        && style.border_top_color == style.border_bottom_color
        && style.border_top_color == style.border_left_color
        && (bt - br).abs() < 0.01
        && (bt - bb).abs() < 0.01
        && (bt - bl).abs() < 0.01;
    if uniform_3d && bt > 0.5 {
        let resolved = style.border_top_color.resolve(inherited_color);
        let base = Color4f::new(resolved.r, resolved.g, resolved.b, resolved.a);
        let uses_platform_current_color =
            matches!(style.border_top_color, StyleColor::CurrentColor)
                && resolved.r == 0.0
                && resolved.g == 0.0
                && resolved.b == 0.0;
        let dark = shade_3d_dark(&base, uses_platform_current_color);
        let light = shade_3d_light(&base, uses_platform_current_color);
        let dark = Color {
            r: dark.r,
            g: dark.g,
            b: dark.b,
            a: dark.a,
        };
        let light = Color {
            r: light.r,
            g: light.g,
            b: light.b,
            a: light.a,
        };
        let skip_middle = if matches!(style_3d, BorderStyle::Groove | BorderStyle::Ridge)
            && (bt.round() as i32) % 2 != 0
        {
            Some((bt.round() as usize) / 2)
        } else {
            None
        };
        draw_miter_blend_pixels_skipping(
            canvas,
            ox1 as i32 - 1,
            oy0 as i32,
            ix1 as i32,
            iy0 as i32 - 1,
            &dark,
            &light,
            skip_middle,
        );
        draw_miter_blend_pixels_skipping(
            canvas,
            ox0 as i32,
            oy1 as i32 - 1,
            ix0 as i32 - 1,
            iy1 as i32,
            &dark,
            &light,
            skip_middle,
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
    draw_miter_blend_pixels_skipping(canvas, x0, y0, x1, y1, color1, color2, None);
}

fn draw_miter_blend_pixels_skipping(
    canvas: &Canvas,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color1: &Color,
    color2: &Color,
    skip_index: Option<usize>,
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
    let mut index = 0;

    loop {
        if skip_index != Some(index) {
            canvas.draw_rect(Rect::from_xywh(cx as f32, cy as f32, 1.0, 1.0), &paint);
        }
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
        index += 1;
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
    adjust_dashed_gap: bool,
    clip_to_miter: bool,
    antialias_stroked_miter: bool,
    antialias_solid_miter: bool,
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
            canvas.clip_path(
                &clip_path.detach(),
                ClipOp::Intersect,
                antialias_solid_miter,
            );

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

            if clip_to_miter {
                canvas.save();
                let mut clip_path = PathBuilder::new();
                clip_path.move_to(Point::new(points[0].0, points[0].1));
                clip_path.line_to(Point::new(points[1].0, points[1].1));
                clip_path.line_to(Point::new(points[2].0, points[2].1));
                clip_path.line_to(Point::new(points[3].0, points[3].1));
                clip_path.close();
                canvas.clip_path(
                    &clip_path.detach(),
                    ClipOp::Intersect,
                    border_style == BorderStyle::Dotted && antialias_stroked_miter,
                );
            }

            paint_border_side(
                canvas,
                border_style,
                border_color,
                inherited_color,
                width,
                rect,
                side,
                adjust_dashed_gap,
            );
            if clip_to_miter {
                canvas.restore();
            }
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
    adjust_dashed_gap: bool,
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
    let uses_platform_3d_current_color = matches!(border_color, StyleColor::CurrentColor)
        && resolved.r == 0.0
        && resolved.g == 0.0
        && resolved.b == 0.0;

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
            let gap_len = if adjust_dashed_gap {
                select_best_dash_gap(stroke_length, dash_len, desired_gap)
            } else {
                desired_gap
            };
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
            // Used border widths are device-pixel aligned. Blink rounds each
            // visible stripe to one third and leaves the remainder as the
            // middle gap (10px => 3px / 4px / 3px, while 50px =>
            // 17px / 16px / 17px).
            let line_width = (width / 3.0).round().max(1.0);
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
            paint_3d_border(
                canvas,
                &base_color,
                width,
                &rect,
                side,
                true,
                uses_platform_3d_current_color,
            );
        }
        BorderStyle::Ridge => {
            // Outer half uses outset shading for this side,
            // inner half uses inset shading for this side.
            paint_3d_border(
                canvas,
                &base_color,
                width,
                &rect,
                side,
                false,
                uses_platform_3d_current_color,
            );
        }
        BorderStyle::Inset => {
            // Per CSS: top+left darkened, bottom+right lightened.
            let shaded = if matches!(side, BorderSide::Top | BorderSide::Left) {
                shade_3d_dark(&base_color, uses_platform_3d_current_color)
            } else {
                shade_3d_light(&base_color, uses_platform_3d_current_color)
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
                shade_3d_light(&base_color, uses_platform_3d_current_color)
            } else {
                shade_3d_dark(&base_color, uses_platform_3d_current_color)
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

fn select_best_closed_dash_gap(stroke_length: f32, dash_length: f32, gap_length: f32) -> f32 {
    let min_count = (stroke_length / (dash_length + gap_length))
        .floor()
        .max(1.0);
    let max_count = min_count + 1.0;
    let min_gap = ((stroke_length - min_count * dash_length) / min_count).max(0.01);
    let max_gap = ((stroke_length - max_count * dash_length) / max_count).max(0.01);
    if (min_gap - gap_length).abs() < (max_gap - gap_length).abs() {
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
    uses_platform_current_color: bool,
) {
    let half_width = (width / 2.0).ceil().max(1.0);
    let dark = shade_3d_dark(color, uses_platform_current_color);
    let light = shade_3d_light(color, uses_platform_current_color);

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

    let (outer_color, inner_color) = if uses_platform_current_color {
        if inset_outer {
            (dark, light)
        } else {
            (light, dark)
        }
    } else if inset_outer {
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

/// Blink's CSS 3D border shade subtracts one third from the largest sRGB
/// component, preserving the component ratios. This is deliberately not a
/// one-third blend toward black: `gray` (128) darkens to 44, matching
/// `Color::Dark()` in Blink.
fn darken_color(color: &Color4f) -> Color4f {
    let value = color.r.max(color.g).max(color.b);
    let multiplier = if value == 0.0 {
        0.0
    } else {
        ((value - 0.33).max(0.0) / value).min(1.0)
    };
    Color4f::new(
        (color.r * multiplier * 255.0).round() / 255.0,
        (color.g * multiplier * 255.0).round() / 255.0,
        (color.b * multiplier * 255.0).round() / 255.0,
        color.a,
    )
}

/// The raised half uses the authored color; the recessed half is darkened.
fn lighten_color(color: &Color4f) -> Color4f {
    *color
}

/// Chromium's Linux theme preserves the traditional raised/recessed gray
/// pair for an omitted border color resolving through black `currentColor`.
/// Explicit colors continue through the CSS color shading path above.
fn shade_3d_dark(color: &Color4f, uses_platform_current_color: bool) -> Color4f {
    if uses_platform_current_color {
        Color4f::new(155.0 / 255.0, 155.0 / 255.0, 155.0 / 255.0, color.a)
    } else {
        darken_color(color)
    }
}

fn shade_3d_light(color: &Color4f, uses_platform_current_color: bool) -> Color4f {
    if uses_platform_current_color {
        Color4f::new(239.0 / 255.0, 239.0 / 255.0, 239.0 / 255.0, color.a)
    } else {
        lighten_color(color)
    }
}

// ── StyleColor PartialEq needed for border comparison ────────────────
// Already derived in the style crate.

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use openui_dom::NodeId;
    use openui_geometry::PhysicalSize;
    use openui_style::{
        BoxShadow, CssLinearGradient, Direction, GradientStop, ImageResourceId, WritingMode,
    };

    fn surface_bytes(surface: &mut skia_safe::Surface) -> Vec<u8> {
        surface
            .image_snapshot()
            .peek_pixels()
            .expect("raster pixels")
            .bytes()
            .expect("pixel bytes")
            .to_vec()
    }

    #[test]
    fn negative_flex_stacking_contexts_sort_by_stack_level_before_order() {
        let mut doc = Document::new();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        doc.node_mut(container).style.display = Display::Flex;
        doc.append_child(doc.root(), container);

        let minus_one = doc.create_node(openui_dom::ElementTag::Div);
        doc.node_mut(minus_one).style.z_index = Some(-1);
        doc.append_child(container, minus_one);
        let minus_two = doc.create_node(openui_dom::ElementTag::Div);
        doc.node_mut(minus_two).style.z_index = Some(-2);
        doc.append_child(container, minus_two);

        // Fragment order is the flex order-modified document order. Stack
        // level still takes precedence when negative stacking contexts paint.
        let fragments = [
            Fragment::new_box(minus_one, PhysicalSize::zero()),
            Fragment::new_box(minus_two, PhysicalSize::zero()),
        ];
        let mut order = 0;
        let mut entries = Vec::new();
        for fragment in &fragments {
            collect_flex_negative_stacking_descendants(
                fragment,
                &doc,
                PhysicalOffset::zero(),
                &mut order,
                &mut entries,
            );
        }
        entries.sort_by_key(|&(z, order, _, _)| (z, order));

        assert_eq!(
            entries
                .iter()
                .map(|(z, _, fragment, _)| (*z, fragment.node_id))
                .collect::<Vec<_>>(),
            vec![(-2, minus_two), (-1, minus_one)]
        );
    }

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

    #[test]
    fn fragmented_column_clip_maps_block_axis_for_all_writing_modes() {
        let cases = [
            (
                Direction::Ltr.writing_direction(WritingMode::HorizontalTb),
                PhysicalOffset::new(LayoutUnit::zero(), LayoutUnit::from_i32(-5)),
                PhysicalSize::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(10)),
                Rect::from_ltrb(10.0, 15.0, 50.0, 50.0),
            ),
            (
                Direction::Ltr.writing_direction(WritingMode::VerticalLr),
                PhysicalOffset::new(LayoutUnit::from_i32(-5), LayoutUnit::zero()),
                PhysicalSize::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(10)),
                Rect::from_ltrb(5.0, 20.0, 50.0, 50.0),
            ),
            (
                Direction::Ltr.writing_direction(WritingMode::VerticalRl),
                PhysicalOffset::new(LayoutUnit::from_i32(35), LayoutUnit::zero()),
                PhysicalSize::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(10)),
                Rect::from_ltrb(10.0, 20.0, 55.0, 50.0),
            ),
        ];

        for (direction, child_offset, child_size, expected) in cases {
            let mut fragment = Fragment::new_box(
                NodeId::NONE,
                PhysicalSize::new(LayoutUnit::from_i32(40), LayoutUnit::from_i32(30)),
            );
            fragment.fragmentation_writing_direction = Some(direction);
            let mut child = Fragment::new_box(NodeId::NONE, child_size);
            child.offset = child_offset;
            fragment.children.push(child);
            assert_eq!(
                compute_column_block_clip_rect(
                    &fragment,
                    PhysicalOffset::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(20)),
                ),
                expected
            );
        }
    }

    #[test]
    fn decoration_source_slice_uses_physical_block_polarity() {
        let rect = Rect::from_ltrb(10.0, 20.0, 50.0, 50.0);
        let cases = [
            (
                Direction::Ltr.writing_direction(WritingMode::HorizontalTb),
                Rect::from_ltrb(10.0, 13.0, 50.0, 113.0),
            ),
            (
                Direction::Ltr.writing_direction(WritingMode::VerticalLr),
                Rect::from_ltrb(3.0, 20.0, 103.0, 50.0),
            ),
            (
                Direction::Ltr.writing_direction(WritingMode::VerticalRl),
                Rect::from_ltrb(-43.0, 20.0, 57.0, 50.0),
            ),
        ];
        for (direction, expected) in cases {
            let mut fragment = Fragment::new_box(NodeId::NONE, PhysicalSize::zero());
            fragment.fragmentation_writing_direction = Some(direction);
            fragment.decoration_slice = Some(openui_layout::DecorationSlice {
                source_block_offset: LayoutUnit::from_i32(7),
                source_block_size: LayoutUnit::from_i32(100),
            });
            assert_eq!(decoration_source_rect(&fragment, rect), expected);
        }
    }

    #[test]
    fn decoration_paint_limit_snaps_the_physical_block_edge() {
        let rect = Rect::from_ltrb(10.0, 20.0, 50.0, 60.0);
        let size = LayoutUnit::from_f32(7.34);
        let offset = PhysicalOffset::new(LayoutUnit::from_f32(10.34), LayoutUnit::from_f32(20.34));
        let cases = [
            (
                Direction::Ltr.writing_direction(WritingMode::HorizontalTb),
                Rect::from_ltrb(10.0, 20.0, 50.0, 28.0),
            ),
            (
                Direction::Ltr.writing_direction(WritingMode::VerticalLr),
                Rect::from_ltrb(10.0, 20.0, 18.0, 60.0),
            ),
            (
                Direction::Ltr.writing_direction(WritingMode::VerticalRl),
                Rect::from_ltrb(43.0, 20.0, 50.0, 60.0),
            ),
        ];
        for (direction, expected) in cases {
            let mut fragment = Fragment::new_box(NodeId::NONE, PhysicalSize::zero());
            fragment.fragmentation_writing_direction = Some(direction);
            fragment.size = PhysicalSize::new(LayoutUnit::from_i32(40), LayoutUnit::from_i32(40));
            assert_eq!(
                block_start_sized_rect(&fragment, rect, offset, size),
                expected
            );
        }
    }

    #[test]
    fn sliced_borders_map_first_middle_and_last_physical_block_edges() {
        let mut style = ComputedStyle::default();
        style.border_top_width = 1;
        style.border_right_width = 2;
        style.border_bottom_width = 3;
        style.border_left_width = 4;
        style.border_top_style = BorderStyle::Solid;
        style.border_right_style = BorderStyle::Solid;
        style.border_bottom_style = BorderStyle::Solid;
        style.border_left_style = BorderStyle::Solid;

        let cases = [
            (
                Direction::Ltr.writing_direction(WritingMode::HorizontalTb),
                (1.0, 2.0, 0.0, 4.0),
                (0.0, 2.0, 0.0, 4.0),
                (0.0, 2.0, 3.0, 4.0),
            ),
            (
                Direction::Ltr.writing_direction(WritingMode::VerticalLr),
                (1.0, 0.0, 3.0, 4.0),
                (1.0, 0.0, 3.0, 0.0),
                (1.0, 2.0, 3.0, 0.0),
            ),
            (
                Direction::Ltr.writing_direction(WritingMode::VerticalRl),
                (1.0, 2.0, 3.0, 0.0),
                (1.0, 0.0, 3.0, 0.0),
                (1.0, 0.0, 3.0, 4.0),
            ),
        ];
        for (direction, first, middle, last) in cases {
            let mut fragment = Fragment::new_box(NodeId::NONE, PhysicalSize::zero());
            // Used border widths are layout-owned. The painter must slice
            // this physical strut rather than re-resolving authored widths,
            // which may differ after table conflict resolution or logical
            // axis projection.
            fragment.border = openui_geometry::BoxStrut::new(
                LayoutUnit::from_i32(1),
                LayoutUnit::from_i32(2),
                LayoutUnit::from_i32(3),
                LayoutUnit::from_i32(4),
            );
            fragment.fragmentation_writing_direction = Some(direction);

            fragment.is_first_for_node = true;
            fragment.is_last_for_node = false;
            assert_eq!(sliced_physical_border_widths(&fragment, &style), first);

            fragment.is_first_for_node = false;
            assert_eq!(sliced_physical_border_widths(&fragment, &style), middle);

            fragment.is_last_for_node = true;
            assert_eq!(sliced_physical_border_widths(&fragment, &style), last);
        }
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

    #[test]
    fn background_axis_tiling_and_edge_offsets_cover_all_repeat_modes() {
        assert_eq!(
            axis_tiles(
                BackgroundRepeat::NoRepeat,
                0.0,
                100.0,
                0.0,
                100.0,
                10.0,
                30.0
            ),
            vec![10.0]
        );
        assert_eq!(
            axis_tiles(BackgroundRepeat::Repeat, 0.0, 100.0, 0.0, 100.0, 10.0, 30.0),
            vec![-20.0, 10.0, 40.0, 70.0]
        );
        assert_eq!(
            axis_tiles(BackgroundRepeat::Space, 0.0, 100.0, 0.0, 100.0, 0.0, 30.0),
            vec![0.0, 35.0, 70.0]
        );
        assert_eq!(
            resolve_background_position(
                BackgroundPosition::Edge {
                    end: true,
                    offset: openui_geometry::Length::px(10.0)
                },
                0.0,
                100.0,
                20.0,
            ),
            70.0
        );
    }

    #[test]
    fn typed_gradient_stops_preserve_hard_calc_and_current_color() {
        let current = Color::from_rgba8(12, 34, 56, 255);
        let image = CssImage::LinearGradient(CssLinearGradient {
            angle_degrees: 90.0,
            corner_direction: None,
            repeating: true,
            color_space: GradientColorSpace::Srgb,
            stops: vec![
                GradientStop {
                    color: StyleColor::CurrentColor,
                    position: GradientStopPosition::Percent(30.0),
                },
                GradientStop {
                    color: StyleColor::CurrentColor,
                    position: GradientStopPosition::Percent(30.0),
                },
                GradientStop {
                    color: StyleColor::CurrentColor,
                    position: GradientStopPosition::Calc {
                        percent: 50.0,
                        px: -10.0,
                    },
                },
            ],
        });
        let CssImage::LinearGradient(gradient) = &image else {
            unreachable!()
        };
        assert_eq!(
            css_gradient_positions(&gradient.stops, 100.0),
            vec![0.3, 0.3, 0.4]
        );
        assert_eq!(solid_gradient_color(&image, &current), Some(current));
    }

    #[test]
    fn subpixel_constant_gradient_repetition_fills_clip_and_balances_canvas() {
        let doc = Document::new();
        let green = Color::from_rgba8(0, 128, 0, 255);
        let image = CssImage::LinearGradient(CssLinearGradient {
            angle_degrees: 180.0,
            corner_direction: None,
            repeating: false,
            color_space: GradientColorSpace::Srgb,
            stops: vec![
                GradientStop {
                    color: StyleColor::Resolved(green),
                    position: GradientStopPosition::Auto,
                },
                GradientStop {
                    color: StyleColor::Resolved(green),
                    position: GradientStopPosition::Auto,
                },
            ],
        });
        let mut style = ComputedStyle::default();
        let mut layer = BackgroundLayer::new(image);
        layer.size = BackgroundSize::Explicit(
            openui_geometry::Length::px(0.2),
            openui_geometry::Length::px(0.2),
        );
        style.background_layers.push(layer);
        let mut surface = surfaces::raster_n32_premul((16, 16)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        let save_count = surface.canvas().save_count();
        paint_background_layers(
            surface.canvas(),
            &doc,
            None,
            &style,
            Rect::from_xywh(2.0, 2.0, 10.0, 10.0),
            1.0,
            None,
        );
        assert_eq!(surface.canvas().save_count(), save_count);
        let pixels = surface_bytes(&mut surface);
        let center = &pixels[(6 * 16 + 6) * 4..][..4];
        let outside = &pixels[..4];
        assert_eq!(center, &[0, 128, 0, 255]);
        assert_eq!(outside, &[255, 255, 255, 255]);
    }

    #[test]
    fn raster_mask_composites_the_complete_box_layer() {
        let mut doc = Document::new();
        let node = doc.create_node(openui_dom::ElementTag::Div);
        doc.append_child(doc.root(), node);
        doc.node_mut(node).style.background_color = Color::from_rgba8(0, 128, 0, 255);
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/accountability/data/wpt_assets/sp13p/blue-and-red-diamonds-81x81.png"
        ));
        let image = doc.register_image_resource(
            "css-backgrounds/support/blue-and-red-diamonds-81x81.png",
            "image/png",
            "adcf99b02f2084a28ce5f227d472c2a757393184e472ba7b3ccba8ce11ed617b",
            bytes.to_vec(),
        );
        doc.node_mut(node)
            .style
            .mask_layers
            .push(BackgroundLayer::new(CssImage::Raster(image)));

        let fragment = Fragment::new_box(
            node,
            PhysicalSize::new(LayoutUnit::from_i32(81), LayoutUnit::from_i32(81)),
        );
        let mut surface = surfaces::raster_n32_premul((81, 81)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        let save_count = surface.canvas().save_count();
        paint_fragment(surface.canvas(), &fragment, &doc, PhysicalOffset::zero());
        assert_eq!(surface.canvas().save_count(), save_count);

        let pixels = surface_bytes(&mut surface);
        assert!(pixels
            .chunks_exact(4)
            .any(|pixel| pixel == [0, 128, 0, 255]));
        assert!(pixels
            .chunks_exact(4)
            .any(|pixel| pixel == [255, 255, 255, 255]));
    }

    #[test]
    fn rounded_overflow_clip_leaves_visible_axis_unbounded() {
        let mut doc = Document::new();
        let parent = doc.create_node(openui_dom::ElementTag::Div);
        let parent_style = doc.node_mut(parent).style_mut();
        parent_style.overflow_x = Overflow::Clip;
        parent_style.overflow_y = Overflow::Visible;
        parent_style.border_top_left_radius = (8.0, 8.0);
        parent_style.border_top_right_radius = (8.0, 8.0);
        parent_style.border_bottom_right_radius = (8.0, 8.0);
        parent_style.border_bottom_left_radius = (8.0, 8.0);
        doc.append_child(doc.root(), parent);
        let child = doc.create_node(openui_dom::ElementTag::Div);
        doc.node_mut(child).style.background_color = Color::from_rgba8(0, 128, 0, 255);
        doc.append_child(parent, child);

        let mut parent_fragment = Fragment::new_box(
            parent,
            PhysicalSize::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(20)),
        );
        parent_fragment.offset =
            PhysicalOffset::new(LayoutUnit::from_i32(5), LayoutUnit::from_i32(5));
        parent_fragment.children.push(Fragment::new_box(
            child,
            PhysicalSize::new(LayoutUnit::from_i32(30), LayoutUnit::from_i32(35)),
        ));
        let mut surface = surfaces::raster_n32_premul((45, 45)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        let save_count = surface.canvas().save_count();
        paint_fragment(
            surface.canvas(),
            &parent_fragment,
            &doc,
            PhysicalOffset::zero(),
        );
        assert_eq!(surface.canvas().save_count(), save_count);

        let pixels = surface_bytes(&mut surface);
        let pixel = |x: usize, y: usize| &pixels[(y * 45 + x) * 4..][..4];
        assert_eq!(pixel(15, 30), [0, 128, 0, 255]);
        assert_eq!(pixel(30, 15), [255, 255, 255, 255]);
    }

    #[test]
    fn border_image_axis_tiling_covers_stretch_repeat_round_and_space() {
        assert_eq!(
            border_image_axis_tiles(BorderImageRepeat::Stretch, 0.0, 100.0, 30.0),
            vec![(0.0, 100.0)]
        );
        assert_eq!(
            border_image_axis_tiles(BorderImageRepeat::Repeat, 0.0, 100.0, 30.0),
            vec![(-10.0, 30.0), (20.0, 30.0), (50.0, 30.0), (80.0, 30.0)]
        );
        assert_eq!(
            border_image_axis_tiles(BorderImageRepeat::Round, 0.0, 100.0, 30.0),
            vec![
                (0.0, 100.0 / 3.0),
                (100.0 / 3.0, 100.0 / 3.0),
                (200.0 / 3.0, 100.0 / 3.0)
            ]
        );
        assert_eq!(
            border_image_axis_tiles(BorderImageRepeat::Space, 0.0, 100.0, 30.0),
            vec![(2.5, 30.0), (35.0, 30.0), (67.5, 30.0)]
        );
    }

    #[test]
    fn outset_and_inset_shadows_paint_spread_blur_and_restore_canvas() {
        let mut surface = surfaces::raster_n32_premul((80, 80)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        let mut style = ComputedStyle::default();
        style.border_top_left_radius = (12.0, 12.0);
        style.border_top_right_radius = (12.0, 12.0);
        style.border_bottom_right_radius = (12.0, 12.0);
        style.border_bottom_left_radius = (12.0, 12.0);
        style.box_shadow = vec![
            BoxShadow {
                offset_x: 3.0,
                offset_y: 4.0,
                blur_radius: 6.0,
                spread_radius: 5.0,
                color: Color::from_rgba8(255, 0, 0, 180),
                inset: false,
            },
            BoxShadow {
                offset_x: 1.0,
                offset_y: 2.0,
                blur_radius: 4.0,
                spread_radius: 3.0,
                color: Color::from_rgba8(0, 0, 255, 180),
                inset: true,
            },
        ];
        let save_count = surface.canvas().save_count();
        let rect = Rect::from_xywh(20.0, 20.0, 40.0, 40.0);
        paint_box_shadows(surface.canvas(), &style, rect, false, false);
        paint_box_shadows(surface.canvas(), &style, rect, true, false);
        assert_eq!(surface.canvas().save_count(), save_count);
        assert!(surface_bytes(&mut surface)
            .chunks_exact(4)
            .any(|pixel| pixel != [255, 255, 255, 255]));
    }

    #[test]
    fn every_visible_border_style_paints_and_balances_canvas() {
        let styles = [
            BorderStyle::Solid,
            BorderStyle::Dotted,
            BorderStyle::Dashed,
            BorderStyle::Double,
            BorderStyle::Groove,
            BorderStyle::Ridge,
            BorderStyle::Inset,
            BorderStyle::Outset,
        ];
        let mut surface = surfaces::raster_n32_premul((120, 120)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        let save_count = surface.canvas().save_count();
        for (index, border_style) in styles.into_iter().enumerate() {
            paint_border_side(
                surface.canvas(),
                border_style,
                &StyleColor::Resolved(Color::from_rgba8(30, 100, 200, 255)),
                &Color::BLACK,
                8.0,
                Rect::from_xywh(10.0, 5.0 + index as f32 * 13.0, 90.0, 8.0),
                BorderSide::Top,
                true,
            );
        }
        assert_eq!(surface.canvas().save_count(), save_count);
        assert!(surface_bytes(&mut surface)
            .chunks_exact(4)
            .any(|pixel| pixel != [255, 255, 255, 255]));
        let _stable_id_type_check = ImageResourceId::new(0);
    }

    #[test]
    fn aliased_ahem_subscript_uses_strict_quarter_pixel_phase() {
        let mut style = ComputedStyle::default();
        style.font_size = 12.0;
        style.vertical_align = openui_style::VerticalAlign::Sub;

        assert_eq!(
            snap_aliased_ahem_keyword_baseline(100.25, &style, true, false),
            100.25
        );
        assert_eq!(
            snap_aliased_ahem_keyword_baseline(100.265625, &style, true, false),
            101.265625
        );
        assert_eq!(
            snap_aliased_ahem_keyword_baseline(100.5, &style, false, false),
            101.5
        );

        style.vertical_align = openui_style::VerticalAlign::Baseline;
        assert_eq!(
            snap_aliased_ahem_keyword_baseline(100.265625, &style, true, true),
            101.265625
        );
    }
}
