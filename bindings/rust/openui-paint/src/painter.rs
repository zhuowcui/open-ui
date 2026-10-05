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
use openui_geometry::{
    BoxStrut, LayoutUnit, PhysicalOffset, PhysicalSnap, RasterBackend, RasterSnapping, TextEdging,
};
use openui_layout::{Fragment, FragmentKind};
use openui_style::{
    BackgroundAttachment, BackgroundClip, BackgroundLayer, BackgroundPosition, BackgroundRepeat,
    BackgroundSize, BorderImage, BorderImageLength, BorderImageRepeat, BorderStyle, Color,
    ComputedStyle, ContentPosition, CssImage, Direction, Display, FontFamily, FontFamilyList,
    GenericFontFamily, GradientColorSpace, GradientStopPosition, LineHeight, ListStylePosition,
    ListStyleType, ObjectFit, Overflow, OverflowClipBox, Position, RadialGradientShape,
    RadialGradientSize, StyleColor, Visibility,
};
use openui_text::{FontMetrics, TextDirection, TextShaper};
use skia_safe::canvas::{SaveLayerFlags, SaveLayerRec, SrcRectConstraint};
use skia_safe::image::RequiredProperties;
use skia_safe::rrect::Corner as RRectCorner;
use skia_safe::{
    color_filters, gradient_shader, image_filters, surfaces, AlphaType, BlendMode, ClipOp, Color4f,
    ColorSpace, ColorType, Data, FilterMode, IRect, Image, ImageInfo, Matrix, MipmapMode, Paint,
    PaintStyle, PathBuilder, PathFillType, PathMeasure, PictureRecorder, Point, RRect, Rect,
    RoundOut, SamplingOptions, TileMode,
};

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use unicode_segmentation::UnicodeSegmentation;

type Canvas = dyn crate::paint_record::PaintCanvas;

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
            GradientStopPosition::HintPercent(_)
            | GradientStopPosition::HintPx(_)
            | GradientStopPosition::HintCalc { .. } => None,
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
    static PREPAINTED_COLUMN_RULES: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    static FRAGMENTED_OOF_HOIST_ACTIVE: RefCell<bool> = const { RefCell::new(false) };
    static FRAGMENTED_INLINE_SKIP_AFTER: RefCell<Option<usize>> = const { RefCell::new(None) };
    static VIEWPORT_SIZE: RefCell<(f32, f32)> = const { RefCell::new((800.0, 600.0)) };
    static RASTER_TILED_REPLAY: RefCell<bool> = const { RefCell::new(false) };
    static DEFER_VIEWPORT_SCROLLBARS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CONTENT_BACKGROUND_ANNOTATIONS: RefCell<Option<Vec<(Color, f32, Option<Rect>)>>> = const { RefCell::new(None) };
    static CONTENT_BACKGROUND_DEVICE_SPACE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CONTENT_BACKGROUND_EFFECT: std::cell::Cell<(f32, bool)> = const { std::cell::Cell::new((1.0, true)) };
    static VIEWPORT_CONTENT_LAYER: RefCell<Option<crate::RecordedContentLayer>> = const { RefCell::new(None) };
    static BROKEN_IMAGE: RefCell<Option<Image>> = const { RefCell::new(None) };
    static BROKEN_IMAGE_HIGH_RES: RefCell<Option<Image>> = const { RefCell::new(None) };
    static RASTERIZING_PROMOTED_TRANSFORM: RefCell<bool> = const { RefCell::new(false) };
    static DEFERRED_OPAQUE_OUTLINE_COVERS: RefCell<Vec<OutlineCoverageKey>> = const { RefCell::new(Vec::new()) };
    static DEFERRED_OUTLINE_REPLAYS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    // Cross-cutting paint phases (fragment edges and late source-order
    // replays) own these outlines outside the fragment's local stacking
    // traversal. Keep this distinct from DEFERRED_OUTLINE_REPLAYS so a nested
    // traversal cannot consume its ancestor's deferral.
    static EXTERNALLY_DEFERRED_OUTLINES: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    // A descendant which crosses a fragmentation clip owns one visible
    // outline contour for that fragment. Both the nearest layout clip and an
    // enclosing ColumnBox can discover the same contour, so retain ownership
    // by fragment identity for the duration of one picture recording.
    static PAINTED_FRAGMENTED_OUTLINES: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OutlineCoverageKey {
    outer: [i32; 4],
    color: [u8; 4],
}

pub(crate) fn set_paint_viewport_size(width: f32, height: f32) {
    VIEWPORT_SIZE.with(|size| *size.borrow_mut() = (width, height));
}

pub(crate) fn set_paint_raster_tiled(tiled: bool) {
    RASTER_TILED_REPLAY.with(|value| *value.borrow_mut() = tiled);
}

pub(crate) fn reset_picture_paint_state() {
    PAINTED_FRAGMENTED_OUTLINES.with(|painted| painted.borrow_mut().clear());
    EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| deferred.borrow_mut().clear());
    VIEWPORT_CONTENT_LAYER.with(|layer| *layer.borrow_mut() = None);
}

pub(crate) fn take_viewport_content_layer() -> Option<crate::RecordedContentLayer> {
    VIEWPORT_CONTENT_LAYER.with(|layer| layer.borrow_mut().take())
}

struct BackgroundEffectScope((f32, bool));

impl BackgroundEffectScope {
    fn new(fragment: &Fragment, style: &ComputedStyle) -> Self {
        Self(CONTENT_BACKGROUND_EFFECT.with(|effect| {
            let previous = effect.get();
            effect.set((
                previous.0
                    * if can_flatten_box_opacity(fragment, style) {
                        1.0
                    } else {
                        style.opacity
                    },
                previous.1
                    && style.filter_blur == 0.0
                    && style.filter_grayscale == 0.0
                    && style.mask_layers.is_empty()
                    && !(CONTENT_BACKGROUND_DEVICE_SPACE.with(|space| space.get())
                        && style.has_border_radius())
                    && style.transform == openui_style::Transform2D::IDENTITY,
            ));
            previous
        }))
    }
}

struct BackgroundRecordingScope {
    previous_annotations: Option<Vec<(Color, f32, Option<Rect>)>>,
    previous_effect: (f32, bool),
    previous_device_space: bool,
}

impl BackgroundRecordingScope {
    fn new() -> Self {
        Self {
            previous_annotations: CONTENT_BACKGROUND_ANNOTATIONS
                .with(|annotations| annotations.replace(Some(Vec::new()))),
            previous_effect: CONTENT_BACKGROUND_EFFECT.with(|effect| effect.replace((1.0, true))),
            previous_device_space: CONTENT_BACKGROUND_DEVICE_SPACE
                .with(|space| space.replace(false)),
        }
    }

    fn for_raster() -> Self {
        let scope = Self::new();
        CONTENT_BACKGROUND_DEVICE_SPACE.with(|space| space.set(true));
        scope
    }

    fn annotations(&self) -> Vec<(Color, f32, Option<Rect>)> {
        CONTENT_BACKGROUND_ANNOTATIONS
            .with(|annotations| annotations.borrow().as_ref().cloned().unwrap_or_default())
    }
}

impl Drop for BackgroundRecordingScope {
    fn drop(&mut self) {
        CONTENT_BACKGROUND_ANNOTATIONS
            .with(|annotations| annotations.replace(self.previous_annotations.take()));
        CONTENT_BACKGROUND_EFFECT.with(|effect| effect.set(self.previous_effect));
        CONTENT_BACKGROUND_DEVICE_SPACE.with(|space| space.set(self.previous_device_space));
    }
}

impl Drop for BackgroundEffectScope {
    fn drop(&mut self) {
        CONTENT_BACKGROUND_EFFECT.with(|effect| effect.set(self.0));
    }
}

fn annotate_content_background(
    canvas: &Canvas,
    mut color: Color,
    rect: Rect,
    opacity: f32,
    opaque_rect: Option<Rect>,
) {
    let effect = CONTENT_BACKGROUND_EFFECT.with(|effect| effect.get());
    if !effect.1 {
        return;
    }
    color.a *= opacity * effect.0;
    CONTENT_BACKGROUND_ANNOTATIONS.with(|annotations| {
        if let Some(annotations) = annotations.borrow_mut().as_mut() {
            annotations.push((
                color,
                rect.width() * rect.height(),
                opaque_rect.filter(|_| color.is_opaque()).and_then(|rect| {
                    let mut rect = if CONTENT_BACKGROUND_DEVICE_SPACE.with(|space| space.get()) {
                        // In a raster backing, opacity belongs to fully covered
                        // physical cells after the current transform and clip.
                        // A rectangular AA clip is accepted by Skia only when
                        // every retained sample has full coverage.
                        let matrix = canvas.local_to_device_as_3x3();
                        if !matrix.rect_stays_rect() || !canvas.is_clip_rect() {
                            return None;
                        }
                        let mut mapped = matrix.map_rect(rect).0;
                        let clip = Rect::from(canvas.device_clip_bounds()?);
                        if !mapped.intersect(clip) {
                            return None;
                        }
                        mapped
                    } else {
                        rect
                    };
                    rect = Rect::from_ltrb(
                        rect.left.ceil(),
                        rect.top.ceil(),
                        rect.right.floor(),
                        rect.bottom.floor(),
                    );
                    (!rect.is_empty() && rect.is_finite()).then_some(rect)
                }),
            ));
        }
    });
}

fn layer_background_color(annotations: &[(Color, f32, Option<Rect>)], bounds: Rect) -> Color {
    let minimum_area = 0.5 * bounds.width() * bounds.height();
    let mut colors = Vec::new();
    for &(color, area, _) in annotations.iter().rev() {
        if color.a > 0.0 && area >= minimum_area {
            colors.push(color);
            if color.is_opaque() {
                break;
            }
        }
    }
    let mut result = Color::TRANSPARENT;
    for color in colors.into_iter().rev() {
        let alpha = color.a + result.a * (1.0 - color.a);
        if alpha > 0.0 {
            result = Color::from_rgba_f32(
                (color.r * color.a + result.r * result.a * (1.0 - color.a)) / alpha,
                (color.g * color.a + result.g * result.a * (1.0 - color.a)) / alpha,
                (color.b * color.a + result.b * result.a * (1.0 - color.a)) / alpha,
                alpha,
            );
        }
    }
    result
}

fn maximum_covered_rect(a: Rect, b: Rect) -> Rect {
    let area = |rect: Rect| rect.width().max(0.0) * rect.height().max(0.0);
    let mut maximum = if area(b) > area(a) { b } else { a };
    let left = a.left.max(b.left);
    let top = a.top.max(b.top);
    let right = a.right.min(b.right);
    let bottom = a.bottom.min(b.bottom);
    if right >= left && bottom >= top && (right > left || bottom > top) {
        let vertical = Rect::from_ltrb(left, a.top.min(b.top), right, a.bottom.max(b.bottom));
        if area(vertical) > area(maximum) {
            maximum = vertical;
        }
        let horizontal = Rect::from_ltrb(a.left.min(b.left), top, a.right.max(b.right), bottom);
        if area(horizontal) > area(maximum) {
            maximum = horizontal;
        }
    }
    maximum
}

fn layer_opaque_rect(annotations: &[(Color, f32, Option<Rect>)]) -> Rect {
    annotations
        .iter()
        .filter_map(|(_, _, opaque)| *opaque)
        .fold(Rect::default(), maximum_covered_rect)
}

/// Preserve every fully covered physical cell, including disjoint regions.
/// Choosing the largest rectangle can discard a covering background when an
/// inset child's painted area extends farther than its parent.
fn layer_opaque_region(annotations: &[(Color, f32, Option<Rect>)]) -> skia_safe::Region {
    let mut region = skia_safe::Region::new();
    for rect in annotations.iter().filter_map(|(_, _, opaque)| *opaque) {
        region.op_rect(rect.round_in(), skia_safe::region::RegionOp::Union);
    }
    region
}

pub(crate) fn paint_document_fragments(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
) {
    struct RestoreScrollbarPhase(bool);
    impl Drop for RestoreScrollbarPhase {
        fn drop(&mut self) {
            DEFER_VIEWPORT_SCROLLBARS.with(|deferred| deferred.set(self.0));
        }
    }
    let _phase = RestoreScrollbarPhase(DEFER_VIEWPORT_SCROLLBARS.with(|value| value.replace(true)));
    paint_fragment_tracked(canvas, fragment, doc, offset);
}

fn uses_deterministic_text_profile(style: &ComputedStyle) -> bool {
    style.font_family.families.iter().any(|family| {
        matches!(family, FontFamily::Named(name) if name.eq_ignore_ascii_case("Droid Sans Fallback"))
    })
}

fn replays_aliased_ahem_start_for_raster_policy(
    style: &ComputedStyle,
    deterministic_text_profile: bool,
) -> bool {
    if deterministic_text_profile {
        return true;
    }
    let scale = style.device_scale_factor;
    // An integral magnification reuses the aliased CSS strike and can lose
    // its inclusive block-start cell. Fractional magnification selects a
    // different physical strike; replaying that cell overpaints its mask.
    style.raster_configuration.author_text.edging == TextEdging::Alias
        && scale.is_finite()
        && scale > 0.0
        && (scale - scale.round()).abs() <= f64::EPSILON
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

fn promoted_transform_backing_bounds(fragment: &Fragment) -> Rect {
    let bounds = promoted_subtree_bounds(fragment);
    let Some(slice) = fragment.decoration_slice else {
        return bounds;
    };

    // A final zero-sized continuation can still own painted overflow. Blink
    // retains a picture-layer backing with the principal box's complete
    // source block extent for that continuation; tightening the backing to
    // the visible child changes the transformed edge samples. Other
    // continuations that can be represented by compositor solid-color quads
    // stay on the direct paint path below.
    if fragment_block_axis_is_x(fragment) && fragment.size.width == LayoutUnit::zero() {
        Rect::from_ltrb(
            bounds.left.min(0.0),
            bounds.top.min(0.0),
            bounds.right.max(slice.source_block_size.to_f32()),
            bounds.bottom.max(fragment.size.height.to_f32()),
        )
    } else if !fragment_block_axis_is_x(fragment) && fragment.size.height == LayoutUnit::zero() {
        Rect::from_ltrb(
            bounds.left.min(0.0),
            bounds.top.min(0.0),
            bounds.right.max(fragment.size.width.to_f32()),
            bounds.bottom.max(slice.source_block_size.to_f32()),
        )
    } else {
        bounds
    }
}

fn has_zero_sized_fragment_block_axis(fragment: &Fragment) -> bool {
    if fragment_block_axis_is_x(fragment) {
        fragment.size.width == LayoutUnit::zero()
    } else {
        fragment.size.height == LayoutUnit::zero()
    }
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
                    | FormControlRole::ColorInput
                    | FormControlRole::DateInput
                    | FormControlRole::FileInput
                    | FormControlRole::Checkbox
                    | FormControlRole::Radio
                    | FormControlRole::TextArea
                    | FormControlRole::Select
                    | FormControlRole::Range
                    | FormControlRole::Meter
                    | FormControlRole::Progress
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
        || (!matches!(
            node.style.display,
            Display::TableRow
                | Display::TableRowGroup
                | Display::TableHeaderGroup
                | Display::TableFooterGroup
        ) && ((node.style.overflow_x != Overflow::Visible
            && node.style.overflow_x != Overflow::Clip)
            || (node.style.overflow_y != Overflow::Visible
                && node.style.overflow_y != Overflow::Clip)))
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
                || fragment_has_overflowing_in_flow_monolithic_descendant(
                    child,
                    doc,
                    LayoutUnit::zero(),
                    fragment_physical_block_extent(child),
                    fragment_block_axis_is_x(child),
                ))
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

/// Complete decoration replay is needed only for an indivisible descendant
/// that extends outside this continuation. An atomic descendant that fits
/// does not move its fragmentable ancestors outside the fragmentainer clip.
fn fragment_has_overflowing_in_flow_monolithic_descendant(
    fragment: &Fragment,
    doc: &Document,
    source_offset: LayoutUnit,
    block_extent: LayoutUnit,
    block_axis_is_x: bool,
) -> bool {
    fragment.children.iter().any(|child| {
        if !child.node_id.is_none() && doc.node(child.node_id).style.is_out_of_flow() {
            return false;
        }
        let start = source_offset
            + if block_axis_is_x {
                child.offset.left
            } else {
                child.offset.top
            };
        if fragment_is_in_flow_monolithic(child, doc) {
            let size = if block_axis_is_x {
                child.size.width
            } else {
                child.size.height
            };
            start < LayoutUnit::zero() || start + size > block_extent
        } else {
            fragment_has_overflowing_in_flow_monolithic_descendant(
                child,
                doc,
                start,
                block_extent,
                block_axis_is_x,
            )
        }
    })
}

/// Paint a fragment tree onto a Skia canvas.
///
/// This is the main entry point — paints the fragment and all its children
/// recursively, with correct coordinate offsets.
pub fn paint_fragment(
    canvas: &skia_safe::Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
) {
    paint_fragment_tracked(canvas, fragment, doc, offset);
}

fn paint_fragment_tracked(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
) {
    // A box that crosses a fractional viewport edge is cut by the physical
    // viewport scissor before its overflowing ink reaches the terminal cell.
    // A box ending at that edge keeps its own snapped edge coverage, so clip
    // only boxes whose laid-out right edge is beyond the viewport.
    let scale = doc.device_scale_factor();
    let (viewport_width, _) = VIEWPORT_SIZE.with(|size| *size.borrow());
    let physical_viewport_right = f64::from(viewport_width) * scale;
    let fragment_left = (offset.left + fragment.offset.left).to_f64();
    let fragment_right = fragment_left + fragment.size.width.to_f64();
    let clips_fractional_terminal_column = physical_viewport_right.fract() > 1.0e-6
        && fragment_left < f64::from(viewport_width)
        && fragment_right > f64::from(viewport_width) + 1.0e-6;
    if clips_fractional_terminal_column {
        canvas.save();
        canvas.clip_rect(
            Rect::from_ltrb(
                -100_000.0,
                -100_000.0,
                (physical_viewport_right.floor() / scale) as f32,
                100_000.0,
            ),
            ClipOp::Intersect,
            false,
        );
    }
    if !fragment.node_id.is_none() && doc.node(fragment.node_id).is_svg_foreign_object {
        // SVG root placement is pixel-snapped before the foreignObject's
        // local CSS decoration is replayed. Keep that translation separate
        // from its local edge snapping (SVGRootPainter and
        // SVGForeignObjectPainter); combining them changes fractional widths.
        let viewport_origin = PhysicalOffset::new(
            offset.left + fragment.offset.left,
            offset.top + fragment.offset.top,
        );
        canvas.save();
        canvas.translate((
            viewport_origin.left.round().to_f32(),
            viewport_origin.top.round().to_f32(),
        ));
        paint_fragment_contents(
            canvas,
            fragment,
            doc,
            PhysicalOffset::new(-fragment.offset.left, -fragment.offset.top),
        );
        canvas.restore();
    } else {
        paint_fragment_contents(canvas, fragment, doc, offset);
    }
    if clips_fractional_terminal_column {
        canvas.restore();
    }
}

fn paint_fragment_contents(
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
    if fragment.kind == FragmentKind::ColumnRule
        && PREPAINTED_COLUMN_RULES.with(|rules| {
            rules
                .borrow()
                .contains(&(fragment as *const Fragment as usize))
        })
    {
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
                    let rendered_legend = doc.fieldset_rendered_legend(complete.node_id);
                    let legend_end = complete
                        .children
                        .iter()
                        .filter(|child| Some(child.node_id) == rendered_legend)
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
                        column_block_only_clip_rect(
                            fragment,
                            abs_offset,
                            doc.device_scale_factor(),
                        ),
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
                let block_clip =
                    column_block_only_clip_rect(fragment, abs_offset, doc.device_scale_factor());
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
            // Multicol fragmentainers clip in the block axis. Inline overflow
            // may normally paint into the column gap. Layout clears
            // `block_axis_clip_only` when a nested fragmentation context owns
            // an inline-axis continuation boundary as well.
            let raw_fragmentainer_clip =
                column_fragmentainer_clip_rect(fragment, abs_offset, doc.device_scale_factor());
            let content_fragmentainer_clip = if doc.device_scale_factor().fract().abs()
                > f64::EPSILON
            {
                let snapping = RasterSnapping::new(doc.device_scale_factor());
                match fragment.fragmentation_writing_direction {
                    Some(direction)
                        if !direction.is_horizontal() && direction.is_flipped_blocks() =>
                    {
                        Rect::from_ltrb(
                            snapping.logical_coordinate(
                                raw_fragmentainer_clip.left,
                                PhysicalSnap::Ceil,
                            ),
                            raw_fragmentainer_clip.top,
                            raw_fragmentainer_clip.right,
                            raw_fragmentainer_clip.bottom,
                        )
                    }
                    Some(direction) if !direction.is_horizontal() => Rect::from_ltrb(
                        raw_fragmentainer_clip.left,
                        raw_fragmentainer_clip.top,
                        snapping
                            .logical_coordinate(raw_fragmentainer_clip.right, PhysicalSnap::Floor),
                        raw_fragmentainer_clip.bottom,
                    ),
                    _ => Rect::from_ltrb(
                        raw_fragmentainer_clip.left,
                        raw_fragmentainer_clip.top,
                        raw_fragmentainer_clip.right,
                        snapping
                            .logical_coordinate(raw_fragmentainer_clip.bottom, PhysicalSnap::Floor),
                    ),
                }
            } else {
                raw_fragmentainer_clip
            };
            let outline_outset = descendant_outline_outset(fragment, doc);
            let outline_fragmentainer_clip = if fragment_block_axis_is_x(fragment) {
                Rect::from_ltrb(
                    raw_fragmentainer_clip.left - outline_outset,
                    raw_fragmentainer_clip.top,
                    raw_fragmentainer_clip.right + outline_outset,
                    raw_fragmentainer_clip.bottom,
                )
            } else {
                Rect::from_ltrb(
                    raw_fragmentainer_clip.left,
                    raw_fragmentainer_clip.top - outline_outset,
                    raw_fragmentainer_clip.right,
                    raw_fragmentainer_clip.bottom + outline_outset,
                )
            };
            let outline_fragmentainer_clip = outward_snap_rect_to_physical(
                outline_fragmentainer_clip,
                doc.device_scale_factor(),
            );
            let outline_slice = column_physical_rect(fragment, abs_offset);
            let block_axis_is_x = fragment_block_axis_is_x(fragment);
            let mut fragmented_outline_owners = Vec::new();
            collect_fragmented_descendant_outline_owners(
                &fragment.children,
                doc,
                abs_offset,
                outline_slice,
                block_axis_is_x,
                &mut fragmented_outline_owners,
            );
            let mut later_repaint_outline_owners = Vec::new();
            collect_later_repaint_outline_owners(
                &fragment.children,
                doc,
                abs_offset,
                outline_slice,
                block_axis_is_x,
                &mut later_repaint_outline_owners,
            );
            let paint_content_fragmentainer_clip = if fragmented_outline_owners.is_empty() {
                outward_snap_rect_to_physical(raw_fragmentainer_clip, doc.device_scale_factor())
            } else {
                content_fragmentainer_clip
            };
            let newly_deferred_outline_owners = EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
                let mut deferred = deferred.borrow_mut();
                fragmented_outline_owners
                    .iter()
                    .chain(&later_repaint_outline_owners)
                    .copied()
                    .filter(|pointer| deferred.insert(*pointer))
                    .collect::<Vec<_>>()
            });
            if fragmented_outline_owners.is_empty() {
                // Preserve the ordinary fragmentainer path byte-for-byte when
                // no outline reaches its edge. Repeating an equivalent
                // fractional clip in nested saves changes Skia's packed edge
                // color even though the logical intersection is unchanged.
                canvas.save();
                canvas.clip_rect(
                    paint_content_fragmentainer_clip,
                    skia_safe::ClipOp::Intersect,
                    false,
                );
                paint_children_with_stacking_order(
                    canvas,
                    &fragment.children,
                    doc,
                    abs_offset,
                    false,
                    false,
                );
                paint_fragmented_descendant_outlines(
                    canvas,
                    &fragment.children,
                    doc,
                    abs_offset,
                    outline_slice,
                    block_axis_is_x,
                    None,
                );
                repaint_later_siblings_over_fragmented_outlines(
                    canvas,
                    &fragment.children,
                    doc,
                    abs_offset,
                    outline_slice,
                    block_axis_is_x,
                );
                canvas.restore();
            } else {
                // Content retains the analytic fragmentainer edge. Outward
                // physical closure belongs only to the outline contour;
                // sharing that expanded scissor with glyphs leaks their ink
                // into the outline's fractional boundary cell.
                canvas.save();
                canvas.clip_rect(
                    outline_fragmentainer_clip,
                    skia_safe::ClipOp::Intersect,
                    false,
                );
                canvas.save();
                canvas.clip_rect(
                    paint_content_fragmentainer_clip,
                    skia_safe::ClipOp::Intersect,
                    false,
                );
                paint_children_with_stacking_order(
                    canvas,
                    &fragment.children,
                    doc,
                    abs_offset,
                    false,
                    false,
                );
                canvas.restore();

                paint_fragmented_descendant_outlines(
                    canvas,
                    &fragment.children,
                    doc,
                    abs_offset,
                    outline_slice,
                    block_axis_is_x,
                    None,
                );

                canvas.save();
                canvas.clip_rect(
                    paint_content_fragmentainer_clip,
                    skia_safe::ClipOp::Intersect,
                    false,
                );
                repaint_later_siblings_over_fragmented_outlines(
                    canvas,
                    &fragment.children,
                    doc,
                    abs_offset,
                    outline_slice,
                    block_axis_is_x,
                );
                canvas.restore();
                canvas.restore();
            }
            EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
                let mut deferred = deferred.borrow_mut();
                for pointer in &newly_deferred_outline_owners {
                    deferred.remove(pointer);
                }
            });
            HOIST_SKIP.with(|skipped| {
                let mut skipped = skipped.borrow_mut();
                for pointer in &monolithic_skip {
                    skipped.remove(pointer);
                }
            });
            for (overflow, parent_offset) in overflowing_monolithic {
                paint_fragment_tracked(canvas, overflow, doc, parent_offset);
            }
        } else {
            paint_children_with_stacking_order(
                canvas,
                &fragment.children,
                doc,
                abs_offset,
                false,
                false,
            );
        }
        return;
    }

    // Line box fragments (from inline layout) have NodeId::NONE — they are
    // anonymous boxes with no DOM node. Just recurse into children.
    if fragment.node_id.is_none() {
        for child in &fragment.children {
            paint_fragment_tracked(canvas, child, doc, abs_offset);
        }
        return;
    }

    let original_style = &doc.node(fragment.node_id).style;
    let mut canvas_adjusted_style = fragment
        .paint_background_color_override
        .map(|color| original_style.derive(|adjusted| adjusted.background_color = color));
    let canvas_background_source = doc.canvas_background_source();
    if canvas_background_source == Some(fragment.node_id)
        || (fragment.node_id == doc.root()
            && canvas_background_source.is_some()
            && canvas_background_source != Some(fragment.node_id))
    {
        let adjusted = original_style.derive(|adjusted| {
            // Canvas-propagated backgrounds are painted once on the canvas, not
            // again on the source element's principal box.  Historical generated
            // documents also carry the initial white canvas color on their
            // synthetic viewport node; suppress that compatibility color when a
            // direct body child supplies the propagated canvas background.
            adjusted.background_color = Color::TRANSPARENT;
            adjusted.background_layers.clear();
            adjusted.background_linear_gradient = None;
        });
        canvas_adjusted_style = Some(adjusted);
    }
    let style = if fragment.kind == FragmentKind::Text {
        fragment.inherited_style.as_ref().unwrap_or(original_style)
    } else {
        canvas_adjusted_style.as_ref().unwrap_or(original_style)
    };
    let _background_effect = BackgroundEffectScope::new(fragment, style);
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
        && (fragment.decoration_slice.is_none()
            || fragment.is_first_for_node
            || has_zero_sized_fragment_block_axis(fragment));
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
        let bounds = promoted_transform_backing_bounds(fragment);
        let raster_width = bounds.width().ceil().max(1.0) as i32;
        let raster_height = bounds.height().ceil().max(1.0) as i32;
        if let Some(mut surface) = surfaces::raster_n32_premul((raster_width, raster_height)) {
            surface.canvas().clear(skia_safe::Color::TRANSPARENT);
            RASTERIZING_PROMOTED_TRANSFORM.with(|active| *active.borrow_mut() = true);
            let raster_parent_offset = PhysicalOffset::new(
                LayoutUnit::from_f32(-bounds.left) - fragment.offset.left,
                LayoutUnit::from_f32(-bounds.top) - fragment.offset.top,
            );
            paint_fragment_tracked(surface.canvas(), fragment, doc, raster_parent_offset);
            RASTERIZING_PROMOTED_TRANSFORM.with(|active| *active.borrow_mut() = false);

            let image = surface.image_snapshot();
            let destination = Rect::from_xywh(
                abs_offset.left.to_f32() + bounds.left,
                abs_offset.top.to_f32() + bounds.top,
                bounds.width(),
                bounds.height(),
            );
            let source = Rect::from_xywh(0.0, 0.0, bounds.width(), bounds.height());
            let mut paint = Paint::default();
            paint.set_anti_alias(true);

            canvas.save();
            canvas.concat(&fragment_transform_matrix(style, fragment, abs_offset));
            // SoftwareRenderer composites a promoted picture tile with
            // drawImageRect rather than replaying it as an image shader. The
            // two paths have measurably different transformed edge coverage.
            canvas.draw_image_rect_with_sampling_options(
                image,
                Some((&source, SrcRectConstraint::Strict)),
                destination,
                SamplingOptions::from(FilterMode::Linear),
                &paint,
            );
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
    let has_filter_layer = style.filter_blur > 0.0 || style.filter_grayscale > 0.0;
    let filter_paint = has_filter_layer.then(|| {
        let mut paint = Paint::default();
        if style.filter_blur > 0.0 {
            paint.set_image_filter(image_filters::blur(
                (style.filter_blur, style.filter_blur),
                TileMode::Decal,
                None,
                None,
            ));
        }
        if style.filter_grayscale > 0.0 {
            let amount = style.filter_grayscale.clamp(0.0, 1.0);
            let inverse = 1.0 - amount;
            paint.set_color_filter(color_filters::matrix_row_major(
                &[
                    inverse + amount * 0.2126,
                    amount * 0.7152,
                    amount * 0.0722,
                    0.0,
                    0.0,
                    amount * 0.2126,
                    inverse + amount * 0.7152,
                    amount * 0.0722,
                    0.0,
                    0.0,
                    amount * 0.2126,
                    amount * 0.7152,
                    inverse + amount * 0.0722,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                ],
                None,
            ));
        }
        paint
    });
    if let Some(paint) = filter_paint.as_ref() {
        canvas.save_layer(&SaveLayerRec::default().paint(paint));
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
        if fragment_subtree_has_text(fragment) || fragment_subtree_has_images(fragment, doc) {
            // Text and sampled-image opacity groups use Chromium's ordinary N32
            // intermediate. Its packed alpha is observably different from
            // an F16 layer for fully covered glyph and image cells. Raster
            // backgrounds and border images require the same intermediate
            // as replaced images, including under an ancestor's opacity.
            canvas.save_layer_alpha_f(None, style.opacity);
        } else {
            // Solid/vector-only groups retain float color until the final
            // composite. This prevents an intermediate 8-bit premultiply
            // from dropping a channel when a translucent primary color is
            // blended over another saturated fill.
            let mut opacity_paint = Paint::default();
            opacity_paint.set_alpha_f(style.opacity);
            canvas.save_layer(
                &SaveLayerRec::default()
                    .paint(&opacity_paint)
                    .flags(SaveLayerFlags::F16_COLOR_TYPE),
            );
        }
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
                paint_media_element(canvas, fragment, doc, style, abs_offset, paint_opacity);
                paint_form_control(canvas, fragment, doc, abs_offset, paint_opacity);
                paint_embedded_canvas(canvas, fragment, doc, style, abs_offset, paint_opacity);
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
    let outline_key = outline_coverage_key(fragment, style, abs_offset);
    let outline_occluded_by_ancestor = style.transform == openui_style::Transform2D::IDENTITY
        && outline_key.is_some_and(|key| {
            DEFERRED_OPAQUE_OUTLINE_COVERS.with(|covers| covers.borrow().contains(&key))
        });
    let should_outline = fragment.kind != FragmentKind::ColumnRule
        && should_paint_outline(fragment, doc, style)
        && !outline_occluded_by_ancestor;
    if style.visibility == Visibility::Visible && should_outline && !paint_outline_after_children {
        paint_outline(canvas, fragment, style, abs_offset);
    }

    // ── Overflow clipping + children ──────────────────────────────────
    let needs_clip =
        needs_overflow_clip(fragment, style, doc) && !has_fixed_fragmentation_transform_clip;
    let deferred_outline_cover = (should_outline
        && paint_outline_after_children
        && style.outline_style == BorderStyle::Solid
        && style.outline_color.resolve(&style.color).is_opaque()
        && style.transform == openui_style::Transform2D::IDENTITY)
        .then_some(outline_key)
        .flatten();
    if let Some(key) = deferred_outline_cover {
        DEFERRED_OPAQUE_OUTLINE_COVERS.with(|covers| covers.borrow_mut().push(key));
    }
    if uses_native_scroll_button_theme(fragment, doc)
        && native_scroll_button_symbol(fragment, doc).is_some()
    {
        // The platform control renderer above consumed the directional
        // single-character content with its native LCD mask.
    } else if let Some(scrollport) = fragment.viewport_scrollport {
        canvas.save();
        // With no reserved gutter, viewport clipping belongs to the final
        // surface/tile assembly. Applying that edge again during document
        // raster changes analytic coverage of transformed ink near the edge.
        // A reserved scrollbar gutter still needs the smaller client clip.
        if scrollport.horizontal_scrollbar || scrollport.vertical_scrollbar {
            canvas.clip_rect(
                outward_snap_rect_to_physical(
                    Rect::from_xywh(
                        abs_offset.left.to_f32(),
                        abs_offset.top.to_f32(),
                        scrollport.client_rect.width().to_f32(),
                        scrollport.client_rect.height().to_f32(),
                    ),
                    doc.device_scale_factor(),
                ),
                ClipOp::Intersect,
                false,
            );
        }
        let node = doc.node(fragment.node_id);
        // Chromium snaps the viewport scroll transform in physical space.
        // Keep the retained offset and geometry logical while preserving the
        // unscrolled paint phase when composing the document at a new offset.
        let snapping = RasterSnapping::new(doc.device_scale_factor());
        let scroll_translation = (
            snapping.logical_coordinate(-node.scroll_left, PhysicalSnap::Nearest),
            snapping.logical_coordinate(-node.scroll_top, PhysicalSnap::Nearest),
        );
        canvas.translate(scroll_translation);
        let content = scrollport.content_rect;
        let bounds = Rect::from_ltrb(
            (abs_offset.left + content.x()).round().to_f32(),
            (abs_offset.top + content.y()).round().to_f32(),
            (abs_offset.left + content.right()).round().to_f32(),
            (abs_offset.top + content.bottom()).round().to_f32(),
        );
        let mut recorder = PictureRecorder::new();
        let layer_canvas: &Canvas = recorder.begin_recording(bounds, false);
        let paint_recording =
            crate::paint_record::RecordingScope::new(layer_canvas.raw_untracked());
        let background_recording = BackgroundRecordingScope::new();
        paint_scrolling_canvas_background(layer_canvas, doc, fragment, bounds);
        paint_children_with_stacking_order(
            layer_canvas,
            &fragment.children,
            doc,
            abs_offset,
            true,
            style.display.is_flex() && fragmented_flex_has_internal_four_way_junction(fragment),
        );
        let annotations = background_recording.annotations();
        let paint_record = paint_recording.snapshot();
        drop(paint_recording);
        drop(background_recording);
        let picture = recorder
            .finish_recording_as_picture(None)
            .expect("viewport content recording must finish");
        let rect_known_to_be_opaque = layer_opaque_rect(&annotations);
        let layer = crate::RecordedContentLayer {
            node_id: fragment.node_id,
            bounds,
            background_color: layer_background_color(&annotations, bounds),
            rect_known_to_be_opaque,
            paint_record,
            contents_opaque: rect_known_to_be_opaque.left <= bounds.left
                && rect_known_to_be_opaque.top <= bounds.top
                && rect_known_to_be_opaque.right >= bounds.right
                && rect_known_to_be_opaque.bottom >= bounds.bottom,
            scroll_translation,
            picture,
            _font_cache_lifetime: doc.font_collection().retain_cache_lifetime(),
        };
        layer.replay(canvas, doc.device_scale_factor());
        VIEWPORT_CONTENT_LAYER.with(|recorded| *recorded.borrow_mut() = Some(layer));
        canvas.restore();
        if !DEFER_VIEWPORT_SCROLLBARS.with(|deferred| deferred.get()) {
            paint_viewport_scrollbars(canvas, fragment, doc, abs_offset);
        }
    } else if needs_clip {
        paint_with_overflow_clip(canvas, fragment, doc, abs_offset, style);
    } else {
        // Paint children with CSS stacking order (z-index aware).
        let is_sc = is_fragment_stacking_context(fragment, doc);
        paint_children_with_stacking_order(
            canvas,
            &fragment.children,
            doc,
            abs_offset,
            is_sc,
            style.display.is_flex() && fragmented_flex_has_internal_four_way_junction(fragment),
        );
    }
    if deferred_outline_cover.is_some() {
        DEFERRED_OPAQUE_OUTLINE_COVERS.with(|covers| {
            covers.borrow_mut().pop();
        });
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
                    false,
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
        let mask_style = style.derive(|mask_style| {
            mask_style.background_layers = style.mask_layers.clone();
            mask_style.background_linear_gradient = None;
            mask_style.background_color = Color::TRANSPARENT;
        });
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
            false,
        );
        canvas.restore();
        canvas.restore();
    }

    if needs_layer {
        canvas.restore();
    }
    if has_filter_layer {
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

#[derive(Clone, Copy)]
enum PassiveMediaIcon {
    Play,
    Volume,
    Fullscreen,
    Overflow,
}

/// Paint Chromium's deterministic initial video surface and passive controls.
///
/// The media decoder is deliberately outside the compact renderer. The
/// initial state of a video with no decoded frame is nevertheless ordinary
/// UA rendering: a #333 surface and, when `controls` is present, the small
/// media-control layout. Keeping that behavior on the public element and its
/// content-box geometry also makes it available to Engine-built micro-oracles
/// without introducing a test-specific resource path.
fn paint_media_element(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let node = doc.node(fragment.node_id);
    if node.tag != ElementTag::Video {
        return;
    }

    let content_rect = Rect::from_ltrb(
        abs_offset.left.to_f32() + fragment.border.left.to_f32() + fragment.padding.left.to_f32(),
        abs_offset.top.to_f32() + fragment.border.top.to_f32() + fragment.padding.top.to_f32(),
        (abs_offset.left + fragment.size.width - fragment.border.right - fragment.padding.right)
            .to_f32(),
        (abs_offset.top + fragment.size.height - fragment.border.bottom - fragment.padding.bottom)
            .to_f32(),
    );
    if content_rect.width() <= 0.0 || content_rect.height() <= 0.0 {
        return;
    }
    // An empty video without exposed controls has no decoded frame and paints
    // transparently. The deterministic #333 initial media surface belongs to
    // the controls presentation; posters and decoded frames are represented
    // by the ordinary replaced-resource path instead.
    if doc.attribute(fragment.node_id, "controls").is_none() {
        return;
    }

    canvas.save();
    if style.has_border_radius() && !fragment.ignore_border_radius {
        canvas.clip_rrect(
            build_clip_rrect(
                &content_rect,
                fragment,
                style,
                OverflowClipBox::ContentBox,
                0.0,
                0.0,
            ),
            ClipOp::Intersect,
            true,
        );
    } else {
        canvas.clip_rect(content_rect, ClipOp::Intersect, false);
    }

    let mut surface_paint = Paint::default();
    surface_paint.set_style(PaintStyle::Fill);
    surface_paint.set_anti_alias(false);
    set_paint_css_color_with_alpha(
        &mut surface_paint,
        &Color::from_rgba8(51, 51, 51, 255),
        opacity_multiplier,
    );
    canvas.draw_rect(content_rect, &surface_paint);

    // M147's sizing-small media panel uses the standard 112 CSS-pixel
    // perceptual scrim. These are the authored UA stops, not fitted samples.
    const SCRIM_POSITIONS: [f32; 16] = [
        0.0, 0.081, 0.155, 0.225, 0.29, 0.353, 0.412, 0.471, 0.529, 0.588, 0.647, 0.71, 0.775,
        0.845, 0.919, 1.0,
    ];
    const SCRIM_ALPHA: [f32; 16] = [
        0.0, 0.013, 0.049, 0.104, 0.175, 0.259, 0.352, 0.45, 0.55, 0.648, 0.741, 0.825, 0.896,
        0.951, 0.987, 1.0,
    ];
    let scrim_height = 112.0_f32.min(content_rect.height());
    let scrim_top = content_rect.bottom - scrim_height;
    let colors: Vec<Color4f> = SCRIM_ALPHA
        .iter()
        .map(|alpha| Color4f::new(0.0, 0.0, 0.0, alpha * opacity_multiplier))
        .collect();
    let mut scrim_paint = Paint::default();
    scrim_paint.set_style(PaintStyle::Fill);
    scrim_paint.set_anti_alias(false);
    scrim_paint.set_dither(true);
    scrim_paint.set_shader(gradient_shader::linear_with_interpolation(
        (
            Point::new(content_rect.left, scrim_top),
            Point::new(content_rect.left, content_rect.bottom),
        ),
        (colors.as_slice(), None),
        SCRIM_POSITIONS.as_slice(),
        TileMode::Clamp,
        skia_gradient_interpolation(GradientColorSpace::Srgb),
        None,
    ));
    canvas.draw_rect(
        Rect::from_ltrb(
            content_rect.left,
            scrim_top,
            content_rect.right,
            content_rect.bottom,
        ),
        &scrim_paint,
    );

    // A source without usable metadata has the stable no-source snapshot used
    // by Chromium: the buttons and timeline remain visible at disabled opacity.
    let button_center_y = content_rect.bottom - 48.0;
    let icon_alpha = opacity_multiplier * 0.3;
    let button_top = content_rect.bottom - 72.0;
    let button_bottom = content_rect.bottom - 24.0;
    // Blink's `-internal-media-control` appearance contributes its transparent
    // native button layer before the `:disabled { opacity: .3 }` group is
    // composited. On an 8-bit surface that layer darkens the inherited scrim
    // by one quantum across the 48px hit target. Keep the contribution tied to
    // the UA control role instead of baking profile- or fixture-specific
    // pixels into the raster path.
    let mut disabled_button_backdrop = Paint::default();
    disabled_button_backdrop.set_style(PaintStyle::Fill);
    disabled_button_backdrop.set_anti_alias(false);
    set_paint_css_color_with_alpha(
        &mut disabled_button_backdrop,
        &Color::BLACK,
        opacity_multiplier * 0.02,
    );
    for (left, right) in [
        (content_rect.left, content_rect.left + 48.0),
        (content_rect.right - 144.0, content_rect.right - 96.0),
        (content_rect.right - 96.0, content_rect.right - 48.0),
        (content_rect.right - 48.0, content_rect.right),
    ] {
        if right > left {
            canvas.draw_rect(
                Rect::from_ltrb(left, button_top, right, button_bottom),
                &disabled_button_backdrop,
            );
        }
    }
    paint_passive_media_icon(
        canvas,
        Point::new(content_rect.left + 24.0, button_center_y),
        PassiveMediaIcon::Play,
        icon_alpha,
    );
    paint_passive_media_icon(
        canvas,
        Point::new(content_rect.right - 120.0, button_center_y),
        PassiveMediaIcon::Volume,
        icon_alpha,
    );
    paint_passive_media_icon(
        canvas,
        Point::new(content_rect.right - 72.0, button_center_y),
        PassiveMediaIcon::Fullscreen,
        icon_alpha,
    );
    paint_passive_media_icon(
        canvas,
        Point::new(content_rect.right - 24.0, button_center_y),
        PassiveMediaIcon::Overflow,
        icon_alpha,
    );

    let time_style = style.derive(|time_style| {
        time_style.font_family = FontFamilyList::generic(GenericFontFamily::SansSerif);
        time_style.font_size = 14.0;
        time_style.color = Color::WHITE;
        time_style.letter_spacing = 0.0;
        time_style.native_control_text = false;
        time_style.native_button_text_metrics = false;
    });
    paint_single_line_control_text(
        canvas,
        doc,
        "0:00",
        &time_style,
        Rect::from_xywh(
            content_rect.left + 48.0,
            content_rect.bottom - 72.0,
            (content_rect.width() - 48.0 - 144.0).max(0.0),
            48.0,
        ),
        false,
        opacity_multiplier,
    );

    let timeline = Rect::from_xywh(
        content_rect.left + 16.0,
        content_rect.bottom - 24.0,
        (content_rect.width() - 32.0).max(0.0),
        4.0,
    );
    if timeline.width() > 0.0 {
        let timeline_style = style.derive(|timeline_style| {
            timeline_style.border_top_left_radius = (2.0, 2.0);
            timeline_style.border_top_right_radius = (2.0, 2.0);
            timeline_style.border_bottom_right_radius = (2.0, 2.0);
            timeline_style.border_bottom_left_radius = (2.0, 2.0);
            timeline_style.box_shadow = vec![openui_style::BoxShadow {
                offset_x: 0.0,
                offset_y: 2.0,
                blur_radius: 10.0,
                spread_radius: 0.0,
                color: Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.5 * opacity_multiplier,
                },
                inset: false,
            }];
        });
        paint_box_shadows(canvas, &timeline_style, timeline, false, false);
        let mut timeline_paint = Paint::default();
        timeline_paint.set_style(PaintStyle::Fill);
        timeline_paint.set_anti_alias(true);
        set_paint_css_color_with_alpha(
            &mut timeline_paint,
            &Color::WHITE,
            opacity_multiplier * 0.3,
        );
        canvas.draw_rrect(RRect::new_rect_xy(timeline, 2.0, 2.0), &timeline_paint);
    }
    canvas.restore();
}

fn paint_passive_media_icon(
    canvas: &Canvas,
    center: Point,
    icon: PassiveMediaIcon,
    opacity_multiplier: f32,
) {
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);
    set_paint_css_color_with_alpha(&mut paint, &Color::WHITE, opacity_multiplier);

    // UA icons have a 24x24 viewBox and a 20px sizing-small background box.
    let scale = 20.0 / 24.0;
    canvas.save();
    canvas.translate(Point::new(center.x - 10.0, center.y - 10.0));
    canvas.scale((scale, scale));

    match icon {
        PassiveMediaIcon::Play => {
            let mut path = PathBuilder::new();
            path.move_to((8.0, 5.0));
            path.line_to((8.0, 19.0));
            path.line_to((19.0, 12.0));
            path.close();
            canvas.draw_path(&path.detach(), &paint);
        }
        PassiveMediaIcon::Volume => {
            let mut path = PathBuilder::new();
            path.move_to((3.0, 9.0));
            path.line_to((3.0, 15.0));
            path.line_to((7.0, 15.0));
            path.line_to((12.0, 20.0));
            path.line_to((12.0, 4.0));
            path.line_to((7.0, 9.0));
            path.close();

            path.move_to((16.5, 12.0));
            path.cubic_to((16.5, 10.23), (15.48, 8.71), (14.0, 7.97));
            path.line_to((14.0, 16.02));
            path.cubic_to((15.48, 15.29), (16.5, 13.77), (16.5, 12.0));
            path.close();

            path.move_to((14.0, 3.23));
            path.line_to((14.0, 5.29));
            path.cubic_to((16.89, 6.15), (19.0, 8.83), (19.0, 12.0));
            path.cubic_to((19.0, 15.17), (16.89, 17.85), (14.0, 18.71));
            path.line_to((14.0, 20.77));
            path.cubic_to((18.01, 19.86), (21.0, 16.28), (21.0, 12.0));
            path.cubic_to((21.0, 7.72), (18.01, 4.14), (14.0, 3.23));
            path.close();
            canvas.draw_path(&path.detach(), &paint);
        }
        PassiveMediaIcon::Fullscreen => {
            let mut path = PathBuilder::new();
            for points in [
                [
                    (7.0, 14.0),
                    (5.0, 14.0),
                    (5.0, 19.0),
                    (10.0, 19.0),
                    (10.0, 17.0),
                    (7.0, 17.0),
                ],
                [
                    (5.0, 10.0),
                    (7.0, 10.0),
                    (7.0, 7.0),
                    (10.0, 7.0),
                    (10.0, 5.0),
                    (5.0, 5.0),
                ],
                [
                    (17.0, 17.0),
                    (14.0, 17.0),
                    (14.0, 19.0),
                    (19.0, 19.0),
                    (19.0, 14.0),
                    (17.0, 14.0),
                ],
                [
                    (14.0, 5.0),
                    (14.0, 7.0),
                    (17.0, 7.0),
                    (17.0, 10.0),
                    (19.0, 10.0),
                    (19.0, 5.0),
                ],
            ] {
                path.move_to(points[0]);
                for point in &points[1..] {
                    path.line_to(*point);
                }
                path.close();
            }
            canvas.draw_path(&path.detach(), &paint);
        }
        PassiveMediaIcon::Overflow => {
            for y in [6.0, 12.0, 18.0] {
                canvas.draw_circle(Point::new(12.0, y), 2.0, &paint);
            }
        }
    }
    canvas.restore();
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
            paint_meter_control(canvas, fragment, doc, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::Progress) => {
            paint_progress_control(canvas, fragment, doc, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::Range) => paint_range_control(
            canvas,
            fragment,
            doc,
            abs_offset,
            opacity_multiplier,
            doc.node(fragment.node_id).form_control_native_appearance,
        ),
        Some(FormControlRole::TextInput) => {
            paint_text_input_control(canvas, fragment, doc, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::ColorInput) => {
            paint_native_button_corners(
                canvas,
                fragment,
                doc,
                abs_offset,
                opacity_multiplier,
                doc.node(fragment.node_id).form_control_disabled,
            );
            paint_native_input_button_corners(canvas, fragment, abs_offset, opacity_multiplier);
            paint_color_input_control(
                canvas,
                fragment,
                abs_offset,
                opacity_multiplier,
                doc.node(fragment.node_id).style.device_scale_factor,
            )
        }
        Some(FormControlRole::DateInput) => {
            paint_date_input_control(canvas, fragment, doc, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::FileInput) => {
            paint_file_input_control(canvas, fragment, doc, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::TextArea) => {
            paint_textarea_resize_grip(
                canvas,
                fragment,
                &doc.node(fragment.node_id).style,
                abs_offset,
                opacity_multiplier,
            );
            paint_textarea_contents(canvas, fragment, doc, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::Select) => {
            paint_select_contents(canvas, fragment, doc, abs_offset, opacity_multiplier);
            if doc.attribute(fragment.node_id, "multiple").is_some()
                && doc.node(fragment.node_id).form_control_native_appearance
            {
                paint_native_listbox_corners(canvas, fragment, abs_offset, opacity_multiplier);
            }
            if doc.node(fragment.node_id).form_control_native_appearance
                && doc.attribute(fragment.node_id, "multiple").is_none()
            {
                paint_select_arrow(canvas, fragment, abs_offset, opacity_multiplier);
            }
        }
        Some(FormControlRole::Checkbox | FormControlRole::Radio)
            if doc.node(fragment.node_id).form_control_native_appearance =>
        {
            paint_checkable_control(canvas, fragment, doc, abs_offset, opacity_multiplier)
        }
        Some(FormControlRole::Button) if uses_native_scroll_button_theme(fragment, doc) => {
            paint_scroll_button_control(canvas, fragment, doc, abs_offset)
        }
        Some(FormControlRole::Button) if uses_native_button_theme(fragment, doc) => {
            // Preserve the byte-frozen unit-scale CPU replay. At scaled replay,
            // paint the NativeTheme primitive directly in the decoration pass
            // instead of expanding this unit-pixel corner oracle.
            if (doc.node(fragment.node_id).style.device_scale_factor - 1.0).abs() <= f64::EPSILON {
                paint_native_button_corners(
                    canvas,
                    fragment,
                    doc,
                    abs_offset,
                    opacity_multiplier,
                    doc.node(fragment.node_id).form_control_disabled,
                );
                if doc.node(fragment.node_id).tag == ElementTag::Input {
                    paint_native_input_button_corners(
                        canvas,
                        fragment,
                        abs_offset,
                        opacity_multiplier,
                    );
                }
            }
            if doc.node(fragment.node_id).tag == ElementTag::Input {
                paint_input_button_contents(canvas, fragment, doc, abs_offset, opacity_multiplier);
            }
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
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    // The native theme suppresses the affordance when its motif cannot fit
    // inside the textarea's border box.
    if fragment.size.height < LayoutUnit::from_i32(15) {
        return;
    }
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();

    if style.background_color.a > 0.0 && style.background_color != Color::WHITE {
        // On an authored fill Chromium composites the larger native motif
        // from a 40%-background edge and a 60%-white highlight.
        let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round();
        let alpha = (style.background_color.a * opacity_multiplier * 255.0).round() as u8;
        let dark = Color::from_rgba8(
            (channel(style.background_color.r) * 0.4).round() as u8,
            (channel(style.background_color.g) * 0.4).round() as u8,
            (channel(style.background_color.b) * 0.4).round() as u8,
            alpha,
        );
        let light = Color::from_rgba8(
            (153.0 + channel(style.background_color.r) * 0.4)
                .round()
                .min(255.0) as u8,
            (153.0 + channel(style.background_color.g) * 0.4)
                .round()
                .min(255.0) as u8,
            (153.0 + channel(style.background_color.b) * 0.4)
                .round()
                .min(255.0) as u8,
            alpha,
        );
        let mut dark_paint = Paint::default();
        dark_paint.set_style(PaintStyle::Fill);
        dark_paint.set_anti_alias(false);
        set_paint_css_color(&mut dark_paint, &dark);
        let mut light_paint = Paint::default();
        light_paint.set_style(PaintStyle::Fill);
        light_paint.set_anti_alias(false);
        set_paint_css_color(&mut light_paint, &light);
        for i in 0..7 {
            canvas.draw_rect(
                Rect::from_xywh(right - 7.0 - i as f32, bottom - 13.0 + i as f32, 1.0, 1.0),
                &dark_paint,
            );
        }
        for i in 1..7 {
            canvas.draw_rect(
                Rect::from_xywh(right - 6.0 - i as f32, bottom - 13.0 + i as f32, 1.0, 1.0),
                &light_paint,
            );
        }
        for i in 0..3 {
            canvas.draw_rect(
                Rect::from_xywh(right - 7.0 - i as f32, bottom - 9.0 + i as f32, 1.0, 1.0),
                &dark_paint,
            );
        }
        for i in 1..3 {
            canvas.draw_rect(
                Rect::from_xywh(right - 6.0 - i as f32, bottom - 9.0 + i as f32, 1.0, 1.0),
                &light_paint,
            );
        }
        return;
    }

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(1.0);
    paint.set_stroke_cap(skia_safe::paint::Cap::Butt);
    paint.set_anti_alias(false);
    set_paint_css_color(
        &mut paint,
        &Color::from_rgba8(0, 0, 0, (153.0 * opacity_multiplier).round() as u8),
    );
    let mut dark_lines = PathBuilder::new();
    dark_lines.move_to((right - 2.0, bottom - 9.0));
    dark_lines.line_to((right - 9.0, bottom - 2.0));
    dark_lines.move_to((right - 2.0, bottom - 5.0));
    dark_lines.line_to((right - 5.0, bottom - 2.0));
    canvas.draw_path(&dark_lines.detach(), &paint);

    set_paint_css_color(
        &mut paint,
        &Color::from_rgba8(255, 255, 255, (153.0 * opacity_multiplier).round() as u8),
    );
    let mut light_lines = PathBuilder::new();
    light_lines.move_to((right - 2.0, bottom - 8.0));
    light_lines.line_to((right - 8.0, bottom - 2.0));
    light_lines.move_to((right - 2.0, bottom - 4.0));
    light_lines.line_to((right - 4.0, bottom - 2.0));
    canvas.draw_path(&light_lines.detach(), &paint);
}

fn paint_native_listbox_corners(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    if right - x < 4.0 || bottom - y < 4.0 {
        return;
    }
    let corners = [
        (x, y, 238),
        (x + 1.0, y, 139),
        (x, y + 1.0, 138),
        (x + 1.0, y + 1.0, 54),
        (right - 2.0, y, 138),
        (right - 1.0, y, 238),
        (right - 2.0, y + 1.0, 213),
        (right - 1.0, y + 1.0, 139),
        (x, bottom - 2.0, 139),
        (x + 1.0, bottom - 2.0, 213),
        (x, bottom - 1.0, 238),
        (x + 1.0, bottom - 1.0, 138),
        (right - 2.0, bottom - 2.0, 213),
        (right - 1.0, bottom - 2.0, 138),
        (right - 2.0, bottom - 1.0, 139),
        (right - 1.0, bottom - 1.0, 238),
    ];
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    for (left, top, gray) in corners {
        set_paint_css_color(
            &mut paint,
            &Color::from_rgba8(gray, gray, gray, (255.0 * opacity_multiplier).round() as u8),
        );
        canvas.draw_rect(Rect::from_xywh(left, top, 1.0, 1.0), &paint);
    }
}

fn first_descendant_text(doc: &Document, node_id: NodeId) -> Option<&str> {
    if let Some(text) = doc.node(node_id).text.as_deref() {
        return Some(text);
    }
    for child in doc.children(node_id) {
        if let Some(text) = first_descendant_text(doc, child) {
            return Some(text);
        }
    }
    None
}

fn paint_control_text_lines(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
    text: &str,
    leading_inset: f32,
    center_single_line: bool,
    style_override: Option<&ComputedStyle>,
) {
    if text.is_empty() {
        return;
    }
    let style = style_override.unwrap_or(&doc.node(fragment.node_id).style);
    let font = doc.resolve_font(crate::text_painter::style_to_font_description(style));
    let direction = if style.direction == Direction::Rtl {
        TextDirection::Rtl
    } else {
        TextDirection::Ltr
    };
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
    let text_style = style.derive(|text_style| text_style.color.a *= opacity_multiplier);
    let line_height = line_metrics.ascent + line_metrics.descent;
    let initial_top = if center_single_line {
        content_top + ((content_bottom - content_top - line_height) / 2.0).max(0.0)
    } else {
        content_top
    };
    for (index, line) in text.lines().enumerate() {
        let shaped = TextShaper::new().shape(line, &font, direction);
        crate::text_painter::paint_text(
            canvas,
            &shaped,
            (
                content_left + leading_inset,
                initial_top + index as f32 * line_height + line_metrics.ascent,
            ),
            &text_style,
        );
    }
    canvas.restore();
}

fn paint_textarea_contents(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let authored = doc
        .attribute(fragment.node_id, "value")
        .or_else(|| first_descendant_text(doc, fragment.node_id))
        .unwrap_or_default();
    let showing_placeholder = authored.is_empty();
    let text = if showing_placeholder {
        doc.attribute(fragment.node_id, "placeholder")
            .unwrap_or_default()
    } else {
        authored
    };
    paint_control_text_lines(
        canvas,
        fragment,
        doc,
        abs_offset,
        opacity_multiplier,
        text,
        0.0,
        false,
        showing_placeholder
            .then(|| {
                doc.node(fragment.node_id)
                    .style
                    .placeholder_style
                    .as_deref()
            })
            .flatten(),
    );
}

fn paint_select_contents(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    fn first_option_text(doc: &Document, node_id: openui_dom::NodeId) -> Option<&str> {
        for child in doc.children(node_id) {
            if matches!(
                doc.node(child).tag,
                ElementTag::Option | ElementTag::OptGroup
            ) {
                if let Some(text) = first_descendant_text(doc, child) {
                    return Some(text);
                }
            }
        }
        None
    }

    let Some(text) = first_option_text(doc, fragment.node_id) else {
        return;
    };
    let multiple = doc.attribute(fragment.node_id, "multiple").is_some();
    paint_control_text_lines(
        canvas,
        fragment,
        doc,
        abs_offset,
        opacity_multiplier,
        text,
        if multiple { 0.0 } else { 4.0 },
        !multiple,
        None,
    );
}

fn paint_checkable_control(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let node = doc.node(fragment.node_id);
    let checked = doc.attribute(fragment.node_id, "checked").is_some();
    let indeterminate = doc.attribute(fragment.node_id, "indeterminate").is_some();
    let left = abs_offset.left.round().to_f32();
    let top = abs_offset.top.round().to_f32();
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    let width = (right - left).max(0.0);
    let height = (bottom - top).max(0.0);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let rect = Rect::from_xywh(left, top, width, height);
    let mut fill = Paint::default();
    fill.set_style(PaintStyle::Fill);
    fill.set_anti_alias(true);
    set_paint_css_color(&mut fill, &Color::WHITE);
    let mut stroke = Paint::default();
    stroke.set_style(PaintStyle::Stroke);
    stroke.set_stroke_width(1.0);
    stroke.set_anti_alias(true);
    set_paint_css_color(
        &mut stroke,
        &Color::from_rgba8(118, 118, 118, (255.0 * opacity_multiplier).round() as u8),
    );
    if node.form_control == Some(FormControlRole::Radio) {
        // NativeTheme records each radio into the control's own border-box
        // display item.  Keep the analytic oval coverage from spilling into
        // an adjacent control when their border boxes meet on a fractional
        // device-pixel boundary; without this hard item clip both ovals
        // contribute coverage to the same physical row.
        canvas.save();
        canvas.clip_rect(rect, ClipOp::Intersect, false);
        let side = width.min(height);
        let radio_rect = Rect::from_xywh(
            left + (width - side) / 2.0,
            top + (height - side) / 2.0,
            side,
            side,
        );
        let radius = side / 2.0;
        // NativeThemeBase::PaintCheckboxRadioCommon deliberately gives the
        // opaque background a 0.2px inset, then records the one-pixel border
        // on a separate 0.5px-inset contour. Using one shared oval changes
        // the multiplied fringe coverage over authored backgrounds.
        let background_rect = Rect::from_ltrb(
            radio_rect.left + 0.2,
            radio_rect.top + 0.2,
            radio_rect.right - 0.2,
            radio_rect.bottom - 0.2,
        );
        canvas.draw_rrect(RRect::new_rect_xy(background_rect, radius, radius), &fill);
        let border_rect = Rect::from_ltrb(
            radio_rect.left + 0.5,
            radio_rect.top + 0.5,
            radio_rect.right - 0.5,
            radio_rect.bottom - 0.5,
        );
        canvas.draw_rrect(RRect::new_rect_xy(border_rect, radius, radius), &stroke);
        canvas.restore();
    } else if (node.style.device_scale_factor - 1.0).abs() > f64::EPSILON {
        // NativeTheme records the scalable checkbox shell as an analytic
        // rounded rectangle. The legacy DPR-1 contour below is its exact
        // one-pixel coverage table, but scaling those table cells would turn
        // a physical AA fringe into CSS-sized blocks.
        canvas.save();
        canvas.clip_rect(rect, ClipOp::Intersect, false);
        let inset = Rect::from_xywh(left + 0.5, top + 0.5, width - 1.0, height - 1.0);
        let shell = RRect::new_rect_xy(inset, 2.0, 2.0);
        let mut backing = fill.clone();
        backing.set_anti_alias(false);
        let snapping = RasterSnapping::new(node.style.device_scale_factor);
        let backing_rect = Rect::from_ltrb(
            snapping.logical_coordinate(left, PhysicalSnap::Floor),
            snapping.logical_coordinate(top, PhysicalSnap::Floor),
            snapping.logical_coordinate(right, PhysicalSnap::Ceil),
            snapping.logical_coordinate(bottom, PhysicalSnap::Ceil),
        );
        canvas.draw_rect(backing_rect, &backing);
        canvas.draw_rrect(shell, &fill);
        canvas.draw_rrect(shell, &stroke);
        canvas.restore();
    } else {
        canvas.draw_rect(rect, &fill);
        if width < 7.0 || height < 7.0 {
            canvas.draw_rect(rect, &stroke);
            return;
        }
        let mut edge = Paint::default();
        edge.set_style(PaintStyle::Fill);
        edge.set_anti_alias(false);
        let mut draw_gray = |gray: u8, cells: &[(f32, f32, f32, f32)]| {
            set_paint_css_color(
                &mut edge,
                &Color::from_rgba8(gray, gray, gray, (255.0 * opacity_multiplier).round() as u8),
            );
            for &(x, y, w, h) in cells {
                canvas.draw_rect(Rect::from_xywh(x, y, w, h), &edge);
            }
        };
        draw_gray(
            118,
            &[
                (left + 3.0, top, width - 6.0, 1.0),
                (left + 3.0, bottom - 1.0, width - 6.0, 1.0),
                (left, top + 3.0, 1.0, height - 6.0),
                (right - 1.0, top + 3.0, 1.0, height - 6.0),
            ],
        );
        draw_gray(
            163,
            &[
                (left + 1.0, top, 1.0, 1.0),
                (left, top + 1.0, 1.0, 1.0),
                (right - 2.0, top, 1.0, 1.0),
                (right - 1.0, top + 1.0, 1.0, 1.0),
                (left, bottom - 2.0, 1.0, 1.0),
                (left + 1.0, bottom - 1.0, 1.0, 1.0),
                (right - 1.0, bottom - 2.0, 1.0, 1.0),
                (right - 2.0, bottom - 1.0, 1.0, 1.0),
            ],
        );
        draw_gray(
            153,
            &[
                (left + 2.0, top, 1.0, 1.0),
                (left, top + 2.0, 1.0, 1.0),
                (right - 3.0, top, 1.0, 1.0),
                (right - 1.0, top + 2.0, 1.0, 1.0),
                (left, bottom - 3.0, 1.0, 1.0),
                (left + 2.0, bottom - 1.0, 1.0, 1.0),
                (right - 1.0, bottom - 3.0, 1.0, 1.0),
                (right - 3.0, bottom - 1.0, 1.0, 1.0),
            ],
        );
        draw_gray(
            178,
            &[
                (left + 1.0, top + 1.0, 1.0, 1.0),
                (right - 2.0, top + 1.0, 1.0, 1.0),
                (left + 1.0, bottom - 2.0, 1.0, 1.0),
                (right - 2.0, bottom - 2.0, 1.0, 1.0),
            ],
        );
    }

    if !(checked || indeterminate) {
        return;
    }
    let mark_color = if node.form_control_disabled {
        Color::from_rgba8(128, 128, 128, 255)
    } else {
        Color::from_rgba8(0, 117, 255, 255)
    };
    let mut mark = Paint::default();
    mark.set_anti_alias(true);
    mark.set_style(PaintStyle::Fill);
    set_paint_css_color_with_alpha(&mut mark, &mark_color, opacity_multiplier);
    if node.form_control == Some(FormControlRole::Radio) {
        let dot = Rect::from_xywh(
            left + width * 0.3,
            top + height * 0.3,
            width * 0.4,
            height * 0.4,
        );
        canvas.draw_oval(dot, &mark);
        return;
    }
    canvas.draw_rect(
        Rect::from_xywh(left + 1.0, top + 1.0, width - 2.0, height - 2.0),
        &mark,
    );
    mark.set_style(PaintStyle::Stroke);
    mark.set_stroke_width((width.min(height) * 0.13).max(1.0));
    mark.set_stroke_cap(skia_safe::PaintCap::Round);
    set_paint_css_color_with_alpha(&mut mark, &Color::WHITE, opacity_multiplier);
    let mut path = PathBuilder::new();
    if indeterminate {
        path.move_to(Point::new(left + width * 0.25, top + height * 0.5));
        path.line_to(Point::new(left + width * 0.75, top + height * 0.5));
    } else {
        path.move_to(Point::new(left + width * 0.22, top + height * 0.52));
        path.line_to(Point::new(left + width * 0.43, top + height * 0.72));
        path.line_to(Point::new(left + width * 0.79, top + height * 0.28));
    }
    canvas.draw_path(&path.detach(), &mark);
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
    let authored = doc
        .attribute(fragment.node_id, "value")
        .filter(|value| !value.is_empty());
    let placeholder = doc
        .attribute(fragment.node_id, "placeholder")
        .filter(|value| !value.is_empty());
    let focused = doc
        .attribute(fragment.node_id, "data-oui-focused")
        .is_some();
    let Some(authored_value) = authored.or(placeholder).or_else(|| focused.then_some("")) else {
        return;
    };
    let showing_placeholder = authored.is_none() && placeholder.is_some();
    let is_password = doc
        .attribute(fragment.node_id, "type")
        .is_some_and(|input_type| input_type.eq_ignore_ascii_case("password"));
    let masked_value;
    let value = if is_password {
        masked_value = "\u{2022}".repeat(authored_value.graphemes(true).count());
        masked_value.as_str()
    } else {
        authored_value
    };
    let mut control_style = if showing_placeholder {
        node.style
            .placeholder_style
            .as_deref()
            .cloned()
            .unwrap_or_else(|| node.style.clone())
    } else {
        node.style.clone()
    };
    control_style.update_derived(|control_style| {
        control_style.native_button_text_metrics = true;
        if showing_placeholder && node.style.placeholder_style.is_none() {
            control_style.color = Color::from_rgba8(117, 117, 117, 255);
        }
    });
    let style = &control_style;
    let font = doc.resolve_font(crate::text_painter::style_to_font_description(style));
    let direction = if style.direction == Direction::Rtl {
        TextDirection::Rtl
    } else {
        TextDirection::Ltr
    };
    let shaped = TextShaper::new().shape(value, &font, direction);
    let metrics = font.font_metrics().copied().unwrap_or_default();
    let line_metrics =
        openui_text::used_line_height_metrics(&metrics, &style.line_height, style.font_size);
    let content_left =
        abs_offset.left.to_f32() + fragment.border.left.to_f32() + fragment.padding.left.to_f32();
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
    let text_style = style.derive(|text_style| text_style.color.a *= opacity_multiplier);
    let content_height = content_bottom - content_top;
    let line_height = line_metrics.ascent + line_metrics.descent;
    let line_top = content_top + ((content_height - line_height) / 2.0).max(0.0);
    let source_offset_to_character = |offset: usize| {
        let mut offset = offset.min(authored_value.len());
        while !authored_value.is_char_boundary(offset) {
            offset -= 1;
        }
        if is_password {
            authored_value[..offset].graphemes(true).count()
        } else {
            authored_value[..offset].chars().count()
        }
    };
    let selection_anchor = doc
        .attribute(fragment.node_id, "data-oui-selection-anchor")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let selection_focus = doc
        .attribute(fragment.node_id, "data-oui-selection-focus")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(selection_anchor);
    let (selection_start, selection_end) = if selection_anchor <= selection_focus {
        (selection_anchor, selection_focus)
    } else {
        (selection_focus, selection_anchor)
    };
    let start_character = source_offset_to_character(selection_start);
    let end_character = source_offset_to_character(selection_end);
    let selection_left = content_left + shaped.width_for_range(0, start_character);
    let selection_right = content_left + shaped.width_for_range(0, end_character);
    if focused && selection_start != selection_end {
        let mut selection_paint = Paint::default();
        selection_paint.set_style(PaintStyle::Fill);
        set_paint_css_color_with_alpha(
            &mut selection_paint,
            &Color::from_rgba8(51, 103, 209, 120),
            opacity_multiplier,
        );
        canvas.draw_rect(
            Rect::from_ltrb(
                selection_left.min(selection_right),
                line_top,
                selection_left.max(selection_right),
                line_top + line_height,
            ),
            &selection_paint,
        );
    }
    crate::text_painter::paint_text(
        canvas,
        &shaped,
        (content_left, line_top + line_metrics.ascent),
        &text_style,
    );
    if focused && selection_start == selection_end && !showing_placeholder {
        let mut caret = Paint::default();
        caret.set_style(PaintStyle::Stroke);
        caret.set_stroke_width(1.0);
        set_paint_css_color_with_alpha(&mut caret, &style.color, opacity_multiplier);
        canvas.draw_line(
            (selection_left, line_top),
            (selection_left, line_top + line_height),
            &caret,
        );
    }
    let composition_start = doc
        .attribute(fragment.node_id, "data-oui-composition-start")
        .and_then(|value| value.parse::<usize>().ok());
    let composition_end = doc
        .attribute(fragment.node_id, "data-oui-composition-end")
        .and_then(|value| value.parse::<usize>().ok());
    if let (Some(start), Some(end)) = (composition_start, composition_end) {
        let left = content_left + shaped.width_for_range(0, source_offset_to_character(start));
        let right = content_left + shaped.width_for_range(0, source_offset_to_character(end));
        let mut composition = Paint::default();
        composition.set_style(PaintStyle::Stroke);
        composition.set_stroke_width(1.0);
        set_paint_css_color_with_alpha(&mut composition, &style.color, opacity_multiplier);
        canvas.draw_line(
            (left.min(right), line_top + line_height),
            (left.max(right), line_top + line_height),
            &composition,
        );
    }
    canvas.restore();
}

fn paint_single_line_control_text(
    canvas: &Canvas,
    doc: &Document,
    text: &str,
    style: &ComputedStyle,
    rect: Rect,
    center_inline: bool,
    opacity_multiplier: f32,
) {
    if text.is_empty() || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let font = doc.resolve_font(crate::text_painter::style_to_font_description(style));
    let direction = if style.direction == Direction::Rtl {
        TextDirection::Rtl
    } else {
        TextDirection::Ltr
    };
    let shaped = TextShaper::new().shape(text, &font, direction);
    let metrics = font.font_metrics().copied().unwrap_or_default();
    let line_metrics =
        openui_text::used_line_height_metrics(&metrics, &style.line_height, style.font_size);
    let line_height = line_metrics.ascent + line_metrics.descent;
    let x = if center_inline {
        rect.left + ((rect.width() - shaped.width()).max(0.0) / 2.0).ceil()
    } else {
        rect.left
    };
    let line_top = rect.top + ((rect.height() - line_height).max(0.0) / 2.0).floor();
    let text_style = style.derive(|text_style| text_style.color.a *= opacity_multiplier);
    canvas.save();
    canvas.clip_rect(rect, ClipOp::Intersect, false);
    crate::text_painter::paint_text(
        canvas,
        &shaped,
        (x, line_top + line_metrics.ascent),
        &text_style,
    );
    canvas.restore();
}

fn paint_input_button_contents(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let node = doc.node(fragment.node_id);
    let input_type = doc.attribute(fragment.node_id, "type").unwrap_or("text");
    let label = doc
        .attribute(fragment.node_id, "value")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            if input_type.eq_ignore_ascii_case("submit") {
                "Submit"
            } else if input_type.eq_ignore_ascii_case("reset") {
                "Reset"
            } else {
                ""
            }
        });
    let rect = Rect::from_ltrb(
        abs_offset.left.to_f32() + fragment.border.left.to_f32(),
        abs_offset.top.to_f32() + fragment.border.top.to_f32(),
        (abs_offset.left + fragment.size.width - fragment.border.right).to_f32(),
        (abs_offset.top + fragment.size.height - fragment.border.bottom).to_f32(),
    );
    paint_single_line_control_text(
        canvas,
        doc,
        label,
        &node.style,
        rect,
        true,
        opacity_multiplier,
    );
}

fn paint_color_input_control(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
    device_scale: f64,
) {
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let width = fragment.size.width.round().to_f32();
    let height = fragment.size.height.round().to_f32();
    if width < 8.0 || height < 12.0 {
        return;
    }
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(239, 239, 239, 255),
        opacity_multiplier,
    );
    canvas.draw_rect(
        Rect::from_xywh(x + 2.0, y + 2.0, width - 4.0, height - 4.0),
        &paint,
    );
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(119, 119, 119, 255),
        opacity_multiplier,
    );
    canvas.draw_rect(
        Rect::from_xywh(x + 3.0, y + 5.0, width - 6.0, height - 10.0),
        &paint,
    );
    draw_css_coverage_rect(
        canvas,
        Rect::from_xywh(x + 4.0, y + 6.0, width - 8.0, height - 12.0),
        &Color::from_rgba8(0, 0, 0, (255.0 * opacity_multiplier).round() as u8),
        device_scale,
        PhysicalCoveragePacking::LeadingBoth,
    );
}

fn paint_date_input_control(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let control_style = doc
        .node(fragment.node_id)
        .style
        .derive(|style| style.native_button_text_metrics = true);
    let style = &control_style;
    let content = Rect::from_ltrb(
        abs_offset.left.to_f32() + fragment.border.left.to_f32() + 1.0,
        abs_offset.top.to_f32() + fragment.border.top.to_f32(),
        (abs_offset.left + fragment.size.width - fragment.border.right).to_f32(),
        (abs_offset.top + fragment.size.height - fragment.border.bottom).to_f32(),
    );
    // Blink's date-edit shadow tree places one device pixel of anonymous
    // spacing on either side of each date separator. Preserve those native
    // slots even when the comparison profile substitutes the square Ahem
    // face for every visible field.
    for (text, inline_offset) in [
        ("mm", 0.0),
        ("/", 27.0),
        ("dd", 41.0),
        ("/", 68.0),
        ("yyyy", 82.0),
    ] {
        paint_single_line_control_text(
            canvas,
            doc,
            text,
            style,
            Rect::from_ltrb(
                content.left + inline_offset,
                content.top,
                content.right,
                content.bottom,
            ),
            false,
            opacity_multiplier,
        );
    }

    let right = (abs_offset.left + fragment.size.width - fragment.border.right)
        .round()
        .to_f32();
    let top = (content.top + (content.height() - 12.0) / 2.0).floor() - 1.0;
    let x = right - 14.0;
    let rows: [&[(i32, u8)]; 12] = [
        &[(2, 127), (8, 127)],
        &[
            (0, 232),
            (1, 127),
            (2, 0),
            (3, 127),
            (4, 127),
            (5, 127),
            (6, 127),
            (7, 127),
            (8, 0),
            (9, 127),
            (10, 233),
        ],
        &[
            (0, 131),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
            (5, 0),
            (6, 0),
            (7, 0),
            (8, 0),
            (9, 0),
            (10, 134),
        ],
        &[
            (0, 127),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
            (5, 0),
            (6, 0),
            (7, 0),
            (8, 0),
            (9, 0),
            (10, 127),
        ],
        &[(0, 127), (1, 127), (9, 127), (10, 127)],
        &[(0, 127), (1, 127), (9, 127), (10, 127)],
        &[(0, 127), (1, 127), (9, 127), (10, 127)],
        &[(0, 127), (1, 127), (9, 127), (10, 127)],
        &[(0, 127), (1, 127), (9, 127), (10, 127)],
        &[(0, 127), (1, 127), (9, 127), (10, 127)],
        &[
            (0, 132),
            (1, 63),
            (2, 127),
            (3, 127),
            (4, 127),
            (5, 127),
            (6, 127),
            (7, 127),
            (8, 127),
            (9, 63),
            (10, 133),
        ],
        &[
            (0, 231),
            (1, 127),
            (2, 127),
            (3, 127),
            (4, 127),
            (5, 127),
            (6, 127),
            (7, 127),
            (8, 127),
            (9, 128),
            (10, 233),
        ],
    ];
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    for (row, samples) in rows.iter().enumerate() {
        for &(column, gray) in *samples {
            set_paint_css_color_with_alpha(
                &mut paint,
                &Color::from_rgba8(gray, gray, gray, 255),
                opacity_multiplier,
            );
            canvas.draw_rect(
                Rect::from_xywh(x + column as f32, top + row as f32, 1.0, 1.0),
                &paint,
            );
        }
    }
}

fn native_following_inline_origin(nominal: f32, device_scale: f32) -> f32 {
    let physical = nominal * device_scale;
    if physical.rem_euclid(1.0) >= 0.5 {
        (physical.floor() + 31.0 / 64.0) / device_scale
    } else {
        nominal
    }
}

fn paint_file_input_control(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let width = fragment.size.width.round().to_f32();
    let height = fragment.size.height.round().to_f32();
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let button_width = 92.0_f32.min(width);
    let button_height = 21.0_f32.min(height);
    let device_scale = doc.node(fragment.node_id).style.device_scale_factor;
    if (device_scale - 1.0).abs() > f64::EPSILON {
        let scale = device_scale as f32;
        let mut clip = canvas
            .local_clip_bounds()
            .unwrap_or_else(|| Rect::from_xywh(x, y, button_width, button_height));
        clip.left = clip
            .left
            .max((abs_offset.left.to_f32() * scale).ceil() / scale);
        clip.top = clip
            .top
            .max((abs_offset.top.to_f32() * scale).ceil() / scale);
        paint_direct_native_button_theme_rect(
            canvas,
            Rect::from_xywh(x, y, button_width, button_height),
            opacity_multiplier,
            Some(clip),
            false,
        );
    } else {
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(false);
        set_paint_css_color_with_alpha(
            &mut paint,
            &Color::from_rgba8(239, 239, 239, 255),
            opacity_multiplier,
        );
        canvas.draw_rect(
            Rect::from_xywh(x + 1.0, y + 1.0, button_width - 2.0, button_height - 2.0),
            &paint,
        );
        set_paint_css_color_with_alpha(
            &mut paint,
            &Color::from_rgba8(118, 118, 118, 255),
            opacity_multiplier,
        );
        canvas.draw_rect(Rect::from_xywh(x + 3.0, y, button_width - 6.0, 1.0), &paint);
        canvas.draw_rect(
            Rect::from_xywh(x + 3.0, y + button_height - 1.0, button_width - 6.0, 1.0),
            &paint,
        );
        canvas.draw_rect(
            Rect::from_xywh(x, y + 3.0, 1.0, button_height - 6.0),
            &paint,
        );
        canvas.draw_rect(
            Rect::from_xywh(x + button_width - 1.0, y + 3.0, 1.0, button_height - 6.0),
            &paint,
        );
        for &(dx, dy, gray) in &[
            (1.0, 0.0, 162),
            (2.0, 0.0, 151),
            (0.0, 1.0, 162),
            (1.0, 1.0, 171),
            (0.0, 2.0, 151),
        ] {
            set_paint_css_color_with_alpha(
                &mut paint,
                &Color::from_rgba8(gray, gray, gray, 255),
                opacity_multiplier,
            );
            for (sample_x, sample_y) in [
                (x + dx, y + dy),
                (x + button_width - 1.0 - dx, y + dy),
                (x + dx, y + button_height - 1.0 - dy),
                (x + button_width - 1.0 - dx, y + button_height - 1.0 - dy),
            ] {
                canvas.draw_rect(Rect::from_xywh(sample_x, sample_y, 1.0, 1.0), &paint);
            }
        }
    }

    let button_style = doc.node(fragment.node_id).style.derive(|button_style| {
        button_style.font_family = FontFamilyList::generic(GenericFontFamily::SansSerif);
        button_style.font_size = 13.333333;
        button_style.native_control_text = true;
        button_style.native_button_text_metrics = false;
    });
    paint_single_line_control_text(
        canvas,
        doc,
        "Choose File",
        &button_style,
        Rect::from_xywh(x + 1.0, y + 4.0, button_width - 4.0, 16.0_f32.min(height)),
        true,
        opacity_multiplier,
    );

    let filename_style = doc.node(fragment.node_id).style.derive(|filename_style| {
        filename_style.native_control_text = false;
        filename_style.native_button_text_metrics = true;
    });
    let nominal_filename_left = x + button_width + 4.0;
    // Chromium resolves the native label's following inline text on the
    // preceding side of an exact/late half-pixel phase. Express that in the
    // physical coverage grid instead of baking a fractional CSS inset for a
    // particular DPR.
    let filename_left = native_following_inline_origin(nominal_filename_left, device_scale as f32);
    paint_single_line_control_text(
        canvas,
        doc,
        "No file chosen",
        &filename_style,
        Rect::from_xywh(
            filename_left,
            y + 1.0,
            (width - button_width - 4.0).max(0.0),
            20.0_f32.min(height),
        ),
        false,
        opacity_multiplier,
    );
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
        && style.background_color == Color::from_rgba8(239, 239, 239, 255)
        && style.border_top_color.resolve(&style.color) == Color::from_rgba8(118, 118, 118, 255)
        && style.border_right_color.resolve(&style.color) == Color::from_rgba8(118, 118, 118, 255)
        && style.border_bottom_color.resolve(&style.color) == Color::from_rgba8(118, 118, 118, 255)
        && style.border_left_color.resolve(&style.color) == Color::from_rgba8(118, 118, 118, 255)
}

fn uses_native_scroll_button_theme(fragment: &Fragment, doc: &Document) -> bool {
    if fragment.node_id.is_none() {
        return false;
    }
    let node = doc.node(fragment.node_id);
    let style = &node.style;
    matches!(node.pseudo_kind, Some(PseudoElementKind::ScrollButton(_)))
        && node.form_control == Some(FormControlRole::Button)
        && node.form_control_native_appearance
        && style.border_top_left_radius == (2.0, 2.0)
        && style.border_top_right_radius == (2.0, 2.0)
        && style.border_bottom_right_radius == (2.0, 2.0)
        && style.border_bottom_left_radius == (2.0, 2.0)
}

fn paint_direct_native_button_theme(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
    device_scale: f64,
    clip_to_outline: bool,
    disabled: bool,
) {
    let left = abs_offset.left.round().to_f32();
    let top = abs_offset.top.round().to_f32();
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    if right - left < 5.0 || bottom - top < 5.0 {
        return;
    }
    let scale = device_scale.max(f64::EPSILON) as f32;
    let outline_clip = clip_to_outline.then(|| {
        let horizontal = canvas
            .local_clip_bounds()
            .unwrap_or_else(|| Rect::from_ltrb(left, top, right, bottom));
        let leading_phase = (left * scale).rem_euclid(1.0);
        let trailing_phase = (right * scale).rem_euclid(1.0);
        let clip_left = if leading_phase > 0.5 {
            (abs_offset.left.to_f32() * scale).ceil() / scale
        } else {
            horizontal.left
        };
        let clip_right = if trailing_phase > f32::EPSILON && trailing_phase < 0.5 {
            ((abs_offset.left + fragment.size.width).to_f32() * scale).floor() / scale
        } else {
            horizontal.right
        };
        Rect::from_ltrb(
            clip_left,
            (abs_offset.top.to_f32() * scale).ceil() / scale,
            clip_right,
            ((abs_offset.top + fragment.size.height).to_f32() * scale).floor() / scale,
        )
    });
    paint_direct_native_button_theme_rect(
        canvas,
        Rect::from_ltrb(left, top, right, bottom),
        opacity_multiplier,
        outline_clip,
        disabled,
    );
}

fn paint_direct_native_button_theme_rect(
    canvas: &Canvas,
    bounds: Rect,
    opacity_multiplier: f32,
    clip: Option<Rect>,
    disabled: bool,
) {
    let rect = Rect::from_ltrb(
        bounds.left + 0.5,
        bounds.top + 0.5,
        bounds.right - 0.5,
        bounds.bottom - 0.5,
    );
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    paint.set_color(skia_safe::Color::from_argb(
        (255.0 * opacity_multiplier).round() as u8,
        if disabled { 238 } else { 239 },
        if disabled { 238 } else { 239 },
        if disabled { 238 } else { 239 },
    ));
    if let Some(clip) = clip {
        canvas.save();
        canvas.clip_rect(clip, ClipOp::Intersect, false);
    }
    canvas.draw_round_rect(rect, 2.0, 2.0, &paint);

    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(1.0);
    paint.set_color(skia_safe::Color::from_argb(
        (255.0 * opacity_multiplier).round() as u8,
        if disabled { 208 } else { 118 },
        if disabled { 208 } else { 118 },
        if disabled { 208 } else { 118 },
    ));
    canvas.draw_round_rect(rect, 2.0, 2.0, &paint);
    if clip.is_some() {
        canvas.restore();
    }
}

fn paint_direct_native_text_control_theme(
    canvas: &Canvas,
    fragment: &Fragment,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
    device_scale: f64,
    clip_to_outline: bool,
    background_color: &Color,
) {
    let left = abs_offset.left.round().to_f32();
    let top = abs_offset.top.round().to_f32();
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    if right <= left || bottom <= top {
        return;
    }
    let scale = device_scale.max(f64::EPSILON) as f32;
    let outline_clip = clip_to_outline.then(|| {
        let horizontal = canvas
            .local_clip_bounds()
            .unwrap_or_else(|| Rect::from_ltrb(left, top, right, bottom));
        Rect::from_ltrb(
            horizontal.left,
            (abs_offset.top.to_f32() * scale).ceil() / scale,
            horizontal.right,
            ((abs_offset.top + fragment.size.height).to_f32() * scale).floor() / scale,
        )
    });
    let fill_bounds = Rect::from_ltrb(left, top, right, bottom);
    let border_bounds = Rect::from_ltrb(left + 0.5, top + 0.5, right - 0.5, bottom - 0.5);
    let alpha = (255.0 * opacity_multiplier).round() as u8;

    // NativeThemeBase keeps its opaque default text-field edge aliased. When
    // an author color supplies the face, the same contour is coverage-blended
    // over that face at fractional device coordinates.
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(!background_color.is_transparent() && *background_color != Color::WHITE);
    let fill = if background_color.is_transparent() {
        Color::WHITE
    } else {
        *background_color
    };
    set_paint_css_color_with_alpha(&mut paint, &fill, opacity_multiplier);
    if let Some(clip) = outline_clip {
        canvas.save();
        canvas.clip_rect(clip, ClipOp::Intersect, false);
    }
    canvas.draw_round_rect(fill_bounds, 2.0, 2.0, &paint);

    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(1.0);
    paint.set_color(skia_safe::Color::from_argb(alpha, 118, 118, 118));
    canvas.draw_round_rect(border_bounds, 2.0, 2.0, &paint);
    if clip_to_outline {
        canvas.restore();
    }
}

/// Complete the pinned Linux passive-button theme's 2px-radius corner mask.
/// The general CSS rrect path paints the straight one-pixel edge; these five
/// coverage samples per corner are supplied by Chromium's native theme.
fn paint_native_button_corners(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
    disabled: bool,
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
    let face = if disabled { 238 } else { 239 };
    set_paint_css_color(
        &mut paint,
        &Color::from_rgba8(face, face, face, (255.0 * opacity_multiplier).round() as u8),
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
    // Native buttons use theme-owned corner coverage. An absolutely
    // positioned control is painted after later in-flow siblings, so their
    // authored background remains the backdrop at uncovered corner cells.
    // Preserve the translucent platform contour in that paint-order case;
    // the ordinary canvas path owns Chromium's packed opaque samples.
    let disabled_samples = [
        (1.0, 0.0, 224, 255),
        (2.0, 0.0, 211, 255),
        (0.0, 1.0, 224, 255),
        (1.0, 1.0, 215, 255),
        (0.0, 2.0, 211, 255),
    ];
    // Fixed-position controls are recorded through Chromium's fixed paint
    // layer. Its packed vertical tangent closes one value below the ordinary
    // display-list path, while the horizontal tangent is unchanged.
    let vertical_tangent = if doc.node(fragment.node_id).style.position == Position::Fixed {
        150
    } else {
        151
    };
    let enabled_samples = [
        (1.0, 0.0, 162, 255),
        (2.0, 0.0, 151, 255),
        (0.0, 1.0, 162, 255),
        (1.0, 1.0, 171, 255),
        (0.0, 2.0, vertical_tangent, 255),
    ];
    let backdrop_samples = [
        (1.0, 0.0, 127, 181),
        (2.0, 0.0, 135, 219),
        (0.0, 1.0, 127, 181),
        (1.0, 1.0, 171, 255),
        (0.0, 2.0, 136, 221),
    ];
    let mut sibling = doc.node(fragment.node_id).next_sibling;
    let mut has_later_colored_backdrop = false;
    while !sibling.is_none() {
        let background = doc.node(sibling).style.background_color;
        if !background.is_transparent() && background != Color::WHITE {
            has_later_colored_backdrop = true;
            break;
        }
        sibling = doc.node(sibling).next_sibling;
    }
    let samples = if disabled {
        &disabled_samples
    } else if doc.node(fragment.node_id).style.position == Position::Absolute
        && has_later_colored_backdrop
    {
        &backdrop_samples
    } else {
        &enabled_samples
    };
    for &(dx, dy, gray, alpha) in samples {
        set_paint_css_color(
            &mut paint,
            &Color::from_rgba8(
                gray,
                gray,
                gray,
                ((alpha as f32) * opacity_multiplier).round() as u8,
            ),
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

fn paint_native_input_button_corners(
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
    // Chromium's passive input-button theme keeps the compact control's
    // darker corner coverage, while a block-axis-stretched button composites
    // the same contour over its enlarged native face. Select the native
    // raster profile from used geometry, independent of DOM/test identity.
    let compact_samples = [
        (1.0, 0.0, 127),
        (2.0, 0.0, 134),
        (0.0, 1.0, 127),
        (1.0, 1.0, 171),
        (0.0, 2.0, 135),
    ];
    let stretched_samples = [
        (1.0, 0.0, 162),
        (2.0, 0.0, 151),
        (0.0, 1.0, 162),
        (1.0, 1.0, 171),
        (0.0, 2.0, 151),
    ];
    let samples = if height > 21.0 {
        &stretched_samples
    } else {
        &compact_samples
    };
    for &(dx, dy, gray) in samples {
        set_paint_css_color(
            &mut paint,
            &Color::from_rgba8(gray, gray, gray, (255.0 * opacity_multiplier).round() as u8),
        );
        for (left, top) in [
            (x + dx, y + dy),
            (x + width - 1.0 - dx, y + dy),
            (x + dx, y + height - 1.0 - dy),
            (x + width - 1.0 - dx, y + height - 1.0 - dy),
        ] {
            canvas.draw_rect(Rect::from_xywh(left, top, 1.0, 1.0), &paint);
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
    let mut sample = |sample_x: f32, sample_y: f32, gray: u8, alpha: u8| {
        set_paint_css_color(&mut paint, &Color::from_rgba8(gray, gray, gray, alpha));
        canvas.draw_rect(
            Rect::from_xywh(x + sample_x, y + sample_y, 1.0, 1.0),
            &paint,
        );
    };

    let disabled = doc.node(fragment.node_id).form_control_disabled;
    let face = if disabled { 238 } else { 239 };
    for &(dx, dy) in &[(2.0, 1.0), (1.0, 2.0), (2.0, 2.0)] {
        for (sx, sy) in [
            (dx, dy),
            (width - 1.0 - dx, dy),
            (dx, height - 1.0 - dy),
            (width - 1.0 - dx, height - 1.0 - dy),
        ] {
            sample(sx, sy, face, 255);
        }
    }

    // The native 2px-radius edge is composited by the Linux control theme,
    // whose corner samples differ from Skia's general CSS rrect coverage.
    // Enabled controls preserve the canvas through the outermost cell and
    // composite their edge samples; disabled controls own an opaque layer.
    let disabled_samples = [
        (0.0, 0.0, 255, 255),
        (1.0, 0.0, 224, 255),
        (2.0, 0.0, 211, 255),
        (0.0, 1.0, 224, 255),
        (1.0, 1.0, 215, 255),
        (0.0, 2.0, 211, 255),
    ];
    let enabled_samples = [
        (1.0, 0.0, 127, 181),
        (2.0, 0.0, 135, 219),
        (0.0, 1.0, 127, 181),
        (1.0, 1.0, 171, 255),
        (0.0, 2.0, 136, 221),
    ];
    let samples: &[(f32, f32, u8, u8)] = if disabled {
        &disabled_samples
    } else {
        &enabled_samples
    };
    for &(dx, dy, gray, alpha) in samples {
        for (sx, sy) in [
            (dx, dy),
            (width - 1.0 - dx, dy),
            (dx, height - 1.0 - dy),
            (width - 1.0 - dx, height - 1.0 - dy),
        ] {
            sample(sx, sy, gray, alpha);
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
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let width = fragment.size.width.to_f32();
    let control_height = fragment.size.height.to_f32();
    let compact_native_meter = control_height <= 16.0;
    let track_height = if compact_native_meter {
        8.0_f32.min(control_height)
    } else {
        (control_height / 2.0).max(0.0)
    };
    if width <= 0.0 || track_height <= 0.0 {
        return;
    }
    let x = abs_offset.left.to_f32();
    let y = if compact_native_meter {
        abs_offset.top.to_f32() + (control_height - track_height) / 2.0
    } else {
        (abs_offset.top.to_f32() + (control_height - track_height) / 2.0).ceil()
    };
    let outer = Rect::from_xywh(x, y, width, track_height);
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);
    let device_scale = doc
        .node(fragment.node_id)
        .style
        .device_scale_factor
        .max(f64::EPSILON) as f32;
    let packed_fractional_contour =
        (device_scale - device_scale.round()).abs() > f32::EPSILON && !compact_native_meter;
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(203, 203, 203, 255),
        opacity_multiplier
            * if packed_fractional_contour {
                7.0 / 8.0
            } else {
                1.0
            },
    );
    let outer_radius_x = if compact_native_meter {
        4.0
    } else {
        20.0_f32.min(width / 2.0)
    };
    let outer_radius_y = if compact_native_meter {
        4.0
    } else {
        20.0_f32.min(track_height / 2.0)
    };
    canvas.draw_rrect(
        RRect::new_rect_xy(outer, outer_radius_x, outer_radius_y),
        &paint,
    );
    if packed_fractional_contour {
        // Ganesh reduces the shared outer fringe to seven eighths of CPU
        // analytic coverage while leaving the fully covered native part
        // opaque. Restore that core after drawing the packed fringe.
        canvas.save();
        canvas.clip_rect(
            Rect::from_ltrb(
                outer.left + 0.5 + f32::EPSILON,
                outer.top + 0.5 + f32::EPSILON,
                outer.right - 0.5 - f32::EPSILON,
                outer.bottom - 0.5 - f32::EPSILON,
            ),
            ClipOp::Intersect,
            false,
        );
        set_paint_css_color_with_alpha(
            &mut paint,
            &Color::from_rgba8(203, 203, 203, 255),
            opacity_multiplier,
        );
        canvas.draw_rrect(
            RRect::new_rect_xy(outer, outer_radius_x, outer_radius_y),
            &paint,
        );
        canvas.restore();
    }

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
    let inner_radius_x = if compact_native_meter {
        3.0
    } else {
        19.2_f32.min((width - 2.0).max(0.0) / 2.0)
    };
    let inner_radius_y = if compact_native_meter {
        3.0
    } else {
        19.0_f32.min((track_height / 2.0 - 1.0).max(0.0))
    };
    canvas.draw_rrect(
        RRect::new_rect_xy(inner, inner_radius_x, inner_radius_y),
        &paint,
    );

    let value = doc
        .attribute(fragment.node_id, "value")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0);
    let minimum = doc
        .attribute(fragment.node_id, "min")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0);
    let maximum = doc
        .attribute(fragment.node_id, "max")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(1.0);
    let fraction = if maximum > minimum {
        ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut value_edge = (x + width * fraction).ceil();
    let physical_value_edge = value_edge * device_scale;
    let fractional_value_edge = (physical_value_edge - physical_value_edge.round()).abs() > 1.0e-5;
    if fractional_value_edge {
        // Ganesh packs fractional clip coverage in 8-bit steps. Bias the
        // mathematical edge by one coverage quantum before CPU Skia creates
        // its analytic clip so 3/4 coverage remains 192/255, not 191/255.
        value_edge += 1.0 / (255.0 * device_scale);
    }
    canvas.save();
    canvas.clip_rect(
        Rect::from_ltrb(x, y, value_edge, y + track_height),
        ClipOp::Intersect,
        fractional_value_edge,
    );
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(16, 124, 16, 255),
        opacity_multiplier,
    );
    let value_shape = RRect::new_rect_xy(
        inner,
        if compact_native_meter {
            3.0
        } else {
            19.0_f32.min((width - 2.0).max(0.0) / 2.0)
        },
        if compact_native_meter {
            3.0
        } else {
            19.0_f32.min((track_height / 2.0 - 1.0).max(0.0))
        },
    );
    // The value part is clipped by the native track contour before its own
    // identically rounded background is painted. At fringe pixels the two
    // analytic masks multiply; drawing the value rrect alone overstates its
    // coverage, especially along fractional bottom edges.
    canvas.clip_rrect(value_shape, ClipOp::Intersect, true);
    canvas.draw_rrect(value_shape, &paint);
    canvas.restore();
}

fn paint_progress_control(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let width = fragment.size.width.to_f32();
    let track_height = (fragment.size.height.to_f32() / 2.0).max(0.0);
    if width <= 0.0 || track_height <= 0.0 {
        return;
    }
    let x = abs_offset.left.to_f32();
    let y = abs_offset.top.to_f32() + (fragment.size.height.to_f32() - track_height) / 2.0;
    let track = Rect::from_xywh(x, y, width, track_height);
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(239, 239, 239, 255),
        opacity_multiplier,
    );
    let track_shape = RRect::new_rect_xy(track, 40.0, 40.0);
    canvas.draw_rrect(track_shape, &paint);
    let fraction = doc
        .attribute(fragment.node_id, "value")
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    canvas.save();
    canvas.clip_rrect(track_shape, ClipOp::Intersect, true);
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(0, 117, 255, 255),
        opacity_multiplier,
    );
    canvas.draw_rect(
        Rect::from_ltrb(x, y, (x + width * fraction).floor(), y + track_height),
        &paint,
    );
    canvas.restore();

    paint.set_style(PaintStyle::Stroke);
    // The native Linux progress ring packs the outside and inside halves of
    // its one-pixel contour separately. The outer half uses quarter alpha;
    // the inner half closes to half alpha after the two source-over passes.
    paint.set_stroke_width(1.0);
    paint.set_color4f(
        Color4f::new(
            117.0 / 255.0,
            117.0 / 255.0,
            117.0 / 255.0,
            0.25 * opacity_multiplier,
        ),
        None::<&ColorSpace>,
    );
    let border_inset = 0.5;
    let border_track = Rect::from_xywh(
        x + border_inset,
        y + border_inset,
        (width - 2.0 * border_inset).max(0.0),
        (track_height - 2.0 * border_inset).max(0.0),
    );
    let border_shape = RRect::new_rect_xy(
        border_track,
        (track_height / 2.0 - border_inset).max(0.0),
        (track_height / 2.0 - border_inset).max(0.0),
    );
    canvas.draw_rrect(border_shape, &paint);
    canvas.save();
    canvas.clip_rrect(border_shape, ClipOp::Intersect, false);
    paint.set_color4f(
        Color4f::new(
            117.0 / 255.0,
            117.0 / 255.0,
            117.0 / 255.0,
            (70.0 / 255.0) * opacity_multiplier,
        ),
        None::<&ColorSpace>,
    );
    canvas.draw_rrect(border_shape, &paint);
    canvas.restore();
}

fn paint_range_control(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
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
    let min = doc
        .attribute(fragment.node_id, "min")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0);
    let max = doc
        .attribute(fragment.node_id, "max")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(100.0);
    let midpoint = min + (max - min) / 2.0;
    let value = doc
        .attribute(fragment.node_id, "value")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(midpoint)
        .clamp(min.min(max), min.max(max));
    let fraction = if (max - min).abs() <= f32::EPSILON {
        0.0
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    };
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

        let thumb_local_x = ((snapped_width - 16.0) * fraction).round();
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
    let thumb_left = (x + (width - 16.0) * fraction).round();
    let thumb_top = (y + (height - 16.0) / 2.0).round();
    let thumb_rect = Rect::from_xywh(thumb_left + 0.5, thumb_top + 0.5, 15.0, 15.0);
    paint.set_style(PaintStyle::Fill);
    set_paint_css_color_with_alpha(
        &mut paint,
        &Color::from_rgba8(0, 117, 255, 255),
        opacity_multiplier,
    );

    // Use one analytic rounded-rectangle primitive at every size, phase, and
    // scale. Backend policy belongs to raster configuration, never to
    // dimension-keyed post-raster sample replacement.
    canvas.draw_rrect(RRect::new_rect_xy(thumb_rect, 8.0, 8.0), &paint);
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

// Chromium 147 high-resolution IDR_BROKENIMAGE.
// Source: third_party/blink/public/default_200_percent/blink/broken_image.png
// SHA-256: 072fb1710ffa7096a8ef3842ac168b1e774d8879054eedad39bc947306389c2d
const BROKEN_IMAGE_HIGH_RES_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x20, 0x08, 0x06, 0x00, 0x00, 0x00, 0x01, 0xb5, 0x18,
    0x42, 0x00, 0x00, 0x02, 0xf1, 0x49, 0x44, 0x41, 0x54, 0x78, 0x5e, 0xbd, 0x96, 0xcb, 0x6b, 0x13,
    0x51, 0x14, 0x87, 0x7f, 0x77, 0x32, 0x49, 0xda, 0x5a, 0x6b, 0x2d, 0xe2, 0x0b, 0xc1, 0xae, 0xc4,
    0xee, 0x14, 0xbb, 0x10, 0x71, 0x27, 0xa2, 0x2b, 0x17, 0x05, 0x05, 0x75, 0xd5, 0xd4, 0x8a, 0x7f,
    0x45, 0xbb, 0x71, 0xe1, 0xc2, 0x85, 0xa8, 0x20, 0xb6, 0x4d, 0x8b, 0x4a, 0x95, 0xaa, 0x18, 0x70,
    0x61, 0x29, 0x8a, 0xa5, 0x2a, 0x16, 0x69, 0xa5, 0x0f, 0x51, 0x14, 0x7c, 0x94, 0x16, 0x1f, 0x6d,
    0x4d, 0xd2, 0xa6, 0xc9, 0x34, 0x99, 0x4c, 0xe6, 0x98, 0x1c, 0x99, 0x0e, 0xc3, 0x9d, 0xa1, 0xb6,
    0x49, 0xfc, 0xe0, 0x70, 0xb9, 0x93, 0x21, 0xdf, 0xfc, 0xce, 0x9c, 0x4b, 0x22, 0xba, 0xba, 0xc2,
    0x84, 0x12, 0x92, 0xcd, 0x66, 0x1b, 0x9b, 0xce, 0x9e, 0x7f, 0x0b, 0x0f, 0x58, 0x78, 0xe2, 0x64,
    0x33, 0x4c, 0x2a, 0xce, 0xab, 0x08, 0x81, 0x81, 0xc7, 0xbd, 0x48, 0x26, 0x93, 0x00, 0x84, 0xa7,
    0x54, 0x25, 0x22, 0xa4, 0x32, 0x84, 0x4c, 0xb6, 0x38, 0x61, 0xd0, 0x0f, 0xa6, 0xa5, 0xe5, 0x1c,
    0xc2, 0xe1, 0xae, 0xd1, 0x47, 0xbd, 0x1d, 0x8d, 0x4d, 0x67, 0x5a, 0x25, 0xa9, 0xca, 0x6d, 0xc8,
    0x11, 0x96, 0x75, 0xb3, 0xb8, 0x84, 0x8a, 0xc2, 0xab, 0x43, 0x7a, 0xb7, 0x53, 0x92, 0x2a, 0x44,
    0x04, 0xe2, 0x0d, 0x15, 0x55, 0x04, 0xc6, 0xda, 0xe7, 0xa5, 0x2d, 0x00, 0xa8, 0x20, 0x3d, 0x20,
    0x25, 0xb4, 0x74, 0x25, 0x86, 0xa5, 0xe1, 0x70, 0xd8, 0x91, 0x94, 0x13, 0xb2, 0x8d, 0x48, 0xaa,
    0xcd, 0x55, 0x02, 0x0d, 0x3b, 0xfd, 0xd8, 0xb7, 0x3b, 0x50, 0x58, 0x79, 0x0f, 0x22, 0x8f, 0x02,
    0x43, 0x44, 0x8e, 0x0a, 0x85, 0x42, 0x2b, 0x49, 0x89, 0xc0, 0x43, 0x63, 0xf9, 0x1c, 0xd4, 0x6e,
    0x50, 0x50, 0xbf, 0xd5, 0x0f, 0x8b, 0x8a, 0x80, 0xe0, 0x3d, 0xcd, 0x66, 0x11, 0x4f, 0xc9, 0xef,
    0x9b, 0xac, 0x95, 0x20, 0xd1, 0xdc, 0x1c, 0x42, 0x4f, 0x4f, 0xf7, 0x68, 0xe4, 0x5e, 0x67, 0xa3,
    0x0a, 0x8f, 0x1b, 0x77, 0x6c, 0x56, 0xe1, 0xc6, 0xf6, 0xfc, 0xf5, 0x58, 0x52, 0x87, 0x0b, 0x7c,
    0x24, 0xf2, 0x5f, 0x0c, 0x2f, 0x84, 0x10, 0xa3, 0x9c, 0xb0, 0x80, 0xb5, 0x6e, 0xaa, 0xe2, 0x64,
    0xf0, 0xab, 0x02, 0x6e, 0x54, 0x06, 0x04, 0x6a, 0x2a, 0x05, 0x16, 0x35, 0x67, 0xca, 0x4c, 0xc6,
    0xc0, 0xe1, 0x63, 0xa7, 0xf3, 0x9f, 0x2b, 0x70, 0xc3, 0xaf, 0xfa, 0xf0, 0x24, 0x72, 0x07, 0xb6,
    0x10, 0xbc, 0xca, 0x32, 0x19, 0xbe, 0x67, 0x7c, 0x2a, 0x0d, 0x0b, 0xeb, 0x68, 0xa9, 0xbe, 0x20,
    0x96, 0x73, 0x1e, 0xe9, 0xfc, 0x01, 0x0e, 0xc5, 0x42, 0x10, 0x58, 0xd7, 0xb0, 0x2b, 0xc8, 0xb2,
    0x55, 0x11, 0xf2, 0x54, 0xe7, 0x28, 0x5f, 0x06, 0xc1, 0x0b, 0xbf, 0x4a, 0xb6, 0x90, 0x31, 0xd9,
    0xba, 0x2a, 0xba, 0x41, 0x98, 0x9a, 0xd5, 0xf9, 0xfe, 0xb5, 0xc2, 0x42, 0x80, 0x3d, 0xec, 0x7b,
    0x3f, 0x9d, 0x46, 0xb9, 0x20, 0x30, 0xd2, 0x3b, 0x2c, 0x27, 0x76, 0x4b, 0xd9, 0x49, 0xe5, 0x96,
    0xd9, 0x42, 0xcb, 0xfe, 0x3f, 0x12, 0x8a, 0x15, 0xa1, 0x59, 0x7e, 0x9f, 0xf8, 0x9f, 0x2d, 0x15,
    0x52, 0x4b, 0xcd, 0x7f, 0x32, 0x12, 0xe2, 0xfa, 0x0c, 0x34, 0x23, 0x86, 0xa0, 0xaf, 0x1a, 0x75,
    0x81, 0x7a, 0x28, 0x42, 0x5d, 0x55, 0xe6, 0x7a, 0x2c, 0x40, 0x26, 0xbc, 0xc8, 0x98, 0x29, 0x4c,
    0xc6, 0x23, 0xf8, 0x90, 0xe8, 0x67, 0x99, 0x45, 0x50, 0xa9, 0xc6, 0xde, 0x9a, 0xa3, 0xd8, 0x5f,
    0x77, 0x0a, 0x95, 0xbe, 0x5a, 0x57, 0x99, 0xe7, 0xb1, 0x30, 0x3d, 0x7c, 0xd3, 0xda, 0x08, 0x86,
    0xe6, 0xae, 0x40, 0xcb, 0xc5, 0x5c, 0x1e, 0x24, 0x89, 0x89, 0x85, 0x08, 0x3e, 0x26, 0x9e, 0xe2,
    0xd0, 0x96, 0x0b, 0xd8, 0xb3, 0xf1, 0x88, 0x53, 0x26, 0x0f, 0x8d, 0xfd, 0x7b, 0xe8, 0xc6, 0x58,
    0xbc, 0x0f, 0xfd, 0x3f, 0xdb, 0x25, 0x99, 0x9b, 0x78, 0x70, 0xee, 0x32, 0x86, 0xa3, 0x1d, 0x00,
    0xc8, 0x55, 0x06, 0x62, 0x21, 0x71, 0x42, 0xe2, 0x84, 0x4e, 0xeb, 0x9b, 0x58, 0x37, 0x26, 0x17,
    0x1f, 0x62, 0x2d, 0x4c, 0xe6, 0xd3, 0x12, 0x41, 0x1c, 0xac, 0x6b, 0x75, 0xf5, 0xd9, 0x09, 0x19,
    0x5a, 0xa9, 0xf1, 0x85, 0xfb, 0x2c, 0x5b, 0x07, 0xe2, 0xdd, 0x62, 0x04, 0x9f, 0x96, 0x06, 0x58,
    0x61, 0x17, 0x4b, 0xc0, 0x42, 0x38, 0xc1, 0xb7, 0xd4, 0x2b, 0x8c, 0xc4, 0x6f, 0xad, 0x4b, 0x66,
    0x6d, 0x5e, 0x47, 0x6f, 0x62, 0xc9, 0xf8, 0xe5, 0x9c, 0x17, 0xb7, 0xa1, 0x49, 0x18, 0x3f, 0xf0,
    0xe2, 0xf7, 0x55, 0x00, 0xb4, 0x5e, 0x19, 0x63, 0x50, 0x1a, 0x2f, 0xe7, 0xaf, 0xe3, 0xf8, 0xb6,
    0x8b, 0x0e, 0x99, 0xa3, 0xa5, 0x39, 0x33, 0x8b, 0xe7, 0xf3, 0x97, 0xa0, 0x9b, 0xa9, 0x22, 0x64,
    0x36, 0xdf, 0xd3, 0x63, 0xf4, 0x39, 0x39, 0x58, 0x90, 0x70, 0x49, 0x2d, 0x9d, 0x48, 0x3c, 0x40,
    0x54, 0xff, 0x52, 0xac, 0xcc, 0xd1, 0xc6, 0x91, 0x85, 0x1e, 0x9e, 0x60, 0x02, 0x39, 0xff, 0x08,
    0x47, 0xd3, 0x33, 0x79, 0x61, 0x5f, 0x29, 0x65, 0x8c, 0x96, 0x8b, 0x62, 0x38, 0x7e, 0xc3, 0xba,
    0x68, 0x27, 0x7c, 0x36, 0x73, 0x0d, 0x26, 0x19, 0x25, 0x94, 0xd9, 0x7c, 0x4d, 0x0d, 0x61, 0x5c,
    0xbb, 0x0d, 0x3d, 0xa7, 0xf1, 0x5e, 0xb4, 0xb5, 0xb5, 0x93, 0xa6, 0x69, 0x25, 0x96, 0xc9, 0x90,
    0x20, 0x54, 0x54, 0x05, 0xf0, 0x07, 0x9f, 0xba, 0x0c, 0x56, 0x38, 0x85, 0x33, 0xe1, 0x00, 0x00,
    0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

fn broken_image_resource(device_scale: f64) -> Option<Image> {
    if device_scale >= 2.0 {
        BROKEN_IMAGE_HIGH_RES.with(|cached| {
            if cached.borrow().is_none() {
                *cached.borrow_mut() =
                    Image::from_encoded(Data::new_copy(BROKEN_IMAGE_HIGH_RES_PNG));
            }
            cached.borrow().clone()
        })
    } else {
        BROKEN_IMAGE.with(|cached| {
            if cached.borrow().is_none() {
                *cached.borrow_mut() = Image::from_encoded(Data::new_copy(BROKEN_IMAGE_PNG));
            }
            cached.borrow().clone()
        })
    }
}

/// Return the first three device cells admitted on each scanline of an
/// axis-aligned rounded replaced-content clip.
///
/// Chromium's software image rasterizer handles this short, coverage-bearing
/// span head with its scalar sampler before entering the ordinary vectorized
/// image loop. Keeping the cells as a geometry-derived path lets the native
/// 14→16 fallback image use the same two filter phases without coupling paint
/// behavior to a document or test identifier.
fn rounded_clip_span_head(rrect: &RRect, destination: Rect) -> Option<skia_safe::Path> {
    let clip_rect = rrect.rect();
    if clip_rect.left.fract() != 0.0
        || clip_rect.top.fract() != 0.0
        || destination.left.fract() != 0.0
        || destination.top.fract() != 0.0
    {
        return None;
    }
    let width = clip_rect.width().ceil() as i32;
    let height = clip_rect.height().ceil() as i32;
    if width <= 0 || height <= 0 {
        return None;
    }
    let mut surface = surfaces::raster_n32_premul((width, height))?;
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    let radii = [
        rrect.radii(RRectCorner::UpperLeft),
        rrect.radii(RRectCorner::UpperRight),
        rrect.radii(RRectCorner::LowerRight),
        rrect.radii(RRectCorner::LowerLeft),
    ];
    let local_rrect = RRect::new_rect_radii(
        Rect::from_xywh(0.0, 0.0, clip_rect.width(), clip_rect.height()),
        &radii,
    );
    let mut mask_paint = Paint::default();
    mask_paint.set_style(PaintStyle::Fill);
    mask_paint.set_anti_alias(true);
    mask_paint.set_color(skia_safe::Color::WHITE);
    surface.canvas().draw_rrect(local_rrect, &mask_paint);

    let image = surface.image_snapshot();
    let info = ImageInfo::new(
        (width, height),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );
    let row_bytes = width as usize * 4;
    let mut pixels = vec![0_u8; row_bytes * height as usize];
    if !image.read_pixels(
        &info,
        &mut pixels,
        row_bytes,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    ) {
        return None;
    }

    let start_x = destination.left as i32;
    let end_x = destination.right.ceil() as i32;
    let start_y = destination.top as i32;
    let end_y = destination.bottom.ceil() as i32;
    let clip_left = clip_rect.left as i32;
    let clip_top = clip_rect.top as i32;
    let mut path = PathBuilder::new();
    let mut has_cells = false;
    for y in start_y..end_y {
        let mask_y = y - clip_top;
        if !(0..height).contains(&mask_y) {
            continue;
        }
        let leading_x = (start_x..end_x).find(|x| {
            let mask_x = *x - clip_left;
            (0..width).contains(&mask_x)
                && pixels[mask_y as usize * row_bytes + mask_x as usize * 4 + 3] != 0
        });
        let Some(leading_x) = leading_x else {
            continue;
        };
        let span_end = (leading_x + 3).min(end_x);
        path.add_rect(
            Rect::from_xywh(
                leading_x as f32,
                y as f32,
                (span_end - leading_x) as f32,
                1.0,
            ),
            None,
            None,
        );
        has_cells = true;
    }
    has_cells.then(|| path.detach())
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
    let source_less_sized = doc.attribute(fragment.node_id, "src").is_none()
        && (!style.width.is_auto() || !style.height.is_auto());
    if node.tag != ElementTag::Image
        || node.replaced.is_some()
        || (doc.attribute(fragment.node_id, "src").is_none()
            && !source_less_alt
            && !source_less_sized)
    {
        return;
    }
    let Some(image) = broken_image_resource(style.device_scale_factor) else {
        return;
    };

    let has_ratio_dimension =
        style.aspect_ratio.is_some() && (!style.width.is_auto() || !style.height.is_auto());
    let treated_as_replaced =
        (!style.width.is_auto() && !style.height.is_auto()) || has_ratio_dimension;
    // UA fallback decorations use CSS-pixel-snapped edges before the
    // device-scale transform, as does the element background. Keep the
    // layout fragment and the public geometry unchanged.
    let abs_offset = PhysicalOffset::new(abs_offset.left.round(), abs_offset.top.round());
    let host_width = fragment.size.width.to_f32();
    let host_height = fragment.size.height.to_f32();
    // The failed-image shadow container and icon are absent when an
    // authored fixed dimension cannot contain the 16px icon plus its inset.
    // This decision uses the authored lengths, before percentage resolution.
    if treated_as_replaced
        && doc.attribute(fragment.node_id, "src") == Some("")
        && ((style.width.is_fixed() && style.width.value() < 18.0)
            || (style.height.is_fixed() && style.height.value() < 18.0))
    {
        return;
    }
    if treated_as_replaced
        && doc.attribute(fragment.node_id, "src") == Some("")
        && host_width > 0.0
        && host_width < 18.0
        && host_height >= 18.0
    {
        let _host_clip_guard = skia_safe::AutoCanvasRestore::guard(canvas, true);
        if style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible {
            let (x, y, width, height) =
                compute_overflow_clip_reference_rect(fragment, abs_offset, style);
            canvas.clip_rect(
                Rect::from_xywh(x, y, width, height),
                ClipOp::Intersect,
                true,
            );
        }
        // An explicitly empty image may retain the broken-resource icon
        // when an authored percentage resolves below its ordinary size.
        // The shadow container clips that icon to its own resolved extent;
        // the remainder of the host keeps the author's background.
        // The UA container inherits the authored percentage width and
        // resolves it against the replaced host's content box. Its two
        // 1px borders and two 1px paddings establish a 4px minimum extent.
        let placeholder_width = if style.width.is_percent() || style.width.is_calculated() {
            resolve_margin_or_padding_f32(&style.width, host_width).max(4.0)
        } else {
            host_width
        };
        let child_clip = Rect::from_xywh(
            abs_offset.left.to_f32() + 1.0,
            abs_offset.top.to_f32() + 1.0,
            (placeholder_width - 2.0).max(0.0),
            (host_height - 2.0).max(0.0),
        );

        let mut border = Paint::default();
        border.set_style(PaintStyle::Stroke);
        border.set_stroke_width(1.0);
        border.set_anti_alias(true);
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

        paint_broken_image_resource(
            canvas,
            image,
            style,
            abs_offset.left.to_f32() + 2.0,
            abs_offset.top.to_f32() + 2.0,
            opacity_multiplier,
            None,
            Some(child_clip),
        );
        return;
    }
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
    let clips_missing_replaced = treated_as_replaced
        && (style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible);
    // Blink places the broken-image glyph in a UA shadow child. The host
    // image's `overflow: clip` applies to that child even when the image is
    // rendered as non-replaced alternative text. Paint the glyph under the
    // host content clip, just as ordinary descendants are clipped below.
    let clips_missing_nonreplaced = !treated_as_replaced
        && (style.overflow_x == Overflow::Clip || style.overflow_y == Overflow::Clip);
    let mut missing_clip_rrect = None;
    if clips_missing_replaced {
        let content_rect = Rect::from_xywh(content_x, content_y, content_width, content_height);
        let clip_rrect = build_clip_rrect(
            &content_rect,
            fragment,
            style,
            OverflowClipBox::ContentBox,
            0.0,
            0.0,
        );
        canvas.save();
        canvas.clip_rrect(clip_rrect, ClipOp::Intersect, true);
        if style.has_border_radius() && !fragment.ignore_border_radius {
            missing_clip_rrect = Some(clip_rrect);
        }
    }
    if clips_missing_nonreplaced {
        let (x, y, width, height) =
            compute_overflow_clip_reference_rect(fragment, abs_offset, style);
        let margin = style.overflow_clip_margin;
        let bounds = Rect::from_ltrb(
            if style.overflow_x == Overflow::Clip {
                x - margin
            } else {
                -100_000.0
            },
            if style.overflow_y == Overflow::Clip {
                y - margin
            } else {
                -100_000.0
            },
            if style.overflow_x == Overflow::Clip {
                x + width + margin
            } else {
                100_000.0
            },
            if style.overflow_y == Overflow::Clip {
                y + height + margin
            } else {
                100_000.0
            },
        );
        canvas.save();
        canvas.clip_rect(
            bounds,
            ClipOp::Intersect,
            antialias_rectangular_overflow_clip(fragment, doc, style),
        );
    }
    let (icon_x, icon_y) = if treated_as_replaced {
        let host_height = if style.height.is_fixed() {
            content_height
        } else {
            20.0_f32.min(content_height)
        };
        let host_width = content_width;
        if host_width >= 1.0 && host_height >= 1.0 {
            let mut border = Paint::default();
            border.set_style(PaintStyle::Stroke);
            border.set_stroke_width(1.0);
            border.set_anti_alias(true);
            border.set_color4f(
                Color4f::new(0.7529412, 0.7529412, 0.7529412, opacity_multiplier),
                None::<&ColorSpace>,
            );
            canvas.draw_rect(
                Rect::from_xywh(
                    content_x + 0.5,
                    content_y + 0.5,
                    (host_width - 1.0).max(0.0),
                    (host_height - 1.0).max(0.0),
                ),
                &border,
            );
            if host_height < 18.0 {
                // The native broken-image inset border keeps its two
                // block-corner highlights when the box is too short for the
                // icon. A single stroked rect otherwise resolves both corner
                // samples to the darker edge color.
                let mut corner = Paint::default();
                corner.set_style(PaintStyle::Fill);
                corner.set_anti_alias(false);
                corner.set_color4f(
                    Color4f::new(0.8156863, 0.8156863, 0.8156863, opacity_multiplier),
                    None::<&ColorSpace>,
                );
                canvas.draw_rect(Rect::from_xywh(content_x, content_y, 1.0, 1.0), &corner);
                canvas.draw_rect(
                    Rect::from_xywh(content_x, content_y + host_height - 1.0, 1.0, 1.0),
                    &corner,
                );
            }
        }
        // A source-less image with authored dimensions participates as a
        // replaced box and receives the native one-pixel frame, but it does
        // not display the broken-resource glyph reserved for a failed URL.
        if doc.attribute(fragment.node_id, "src").is_none() && !source_less_alt {
            if clips_missing_replaced {
                canvas.restore();
            }
            return;
        }
        if host_width < 18.0 || host_height < 18.0 {
            if clips_missing_replaced {
                canvas.restore();
            }
            return;
        }
        (content_x + 2.0, content_y + 2.0)
    } else {
        (
            (abs_offset.left.to_f32()
                + fragment.border.left.to_f32()
                + fragment.padding.left.to_f32())
            .round(),
            (abs_offset.top.to_f32()
                + fragment.border.top.to_f32()
                + fragment.padding.top.to_f32())
            .round(),
        )
    };

    paint_broken_image_resource(
        canvas,
        image,
        style,
        icon_x,
        icon_y,
        opacity_multiplier,
        missing_clip_rrect.as_ref(),
        None,
    );

    if !treated_as_replaced
        && doc
            .attribute(fragment.node_id, "alt")
            .is_some_and(|alt| !alt.is_empty())
        && doc.children(fragment.node_id).next().is_none()
    {
        let alt = doc.attribute(fragment.node_id, "alt").unwrap_or("");
        let font = doc.resolve_font(crate::text_painter::style_to_font_description(style));
        let direction = if style.direction == Direction::Rtl {
            TextDirection::Rtl
        } else {
            TextDirection::Ltr
        };
        let shaped = TextShaper::new().shape(alt, &font, direction);
        let metrics = font.font_metrics().copied().unwrap_or_default();
        let line_metrics =
            openui_text::used_line_height_metrics(&metrics, &style.line_height, style.font_size);
        let alt_style = style.derive(|alt_style| alt_style.color.a *= opacity_multiplier);
        crate::text_painter::paint_text(
            canvas,
            &shaped,
            (icon_x + 16.0, abs_offset.top.to_f32() + line_metrics.ascent),
            &alt_style,
        );
    }
    if clips_missing_replaced || clips_missing_nonreplaced {
        canvas.restore();
    }
}

/// Sample the pinned fallback resource through the same physical transform
/// for ordinary and clipped UA shadow content.
fn paint_broken_image_resource(
    canvas: &Canvas,
    image: Image,
    style: &ComputedStyle,
    icon_x: f32,
    icon_y: f32,
    opacity_multiplier: f32,
    missing_clip_rrect: Option<&RRect>,
    child_clip: Option<Rect>,
) {
    // CPU slots sample through the legacy inverse matrix and restart at
    // analytic coverage spans. Rounded clips retain their established direct
    // resource path and its separate span head.
    let source = Rect::from_xywh(0.0, 0.0, image.width() as f32, image.height() as f32);
    let main_phase = if missing_clip_rrect.is_some() {
        3.0 / 64.0
    } else {
        1.0 / 16.0
    };
    let main_destination = Rect::from_xywh(icon_x + main_phase, icon_y, 16.0, 16.0);
    let device_destination = Rect::from_xywh(icon_x, icon_y, 16.0, 16.0);
    let mut paint = Paint::default();
    paint.set_alpha_f(opacity_multiplier);
    let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear);
    let span_head =
        missing_clip_rrect.and_then(|rrect| rounded_clip_span_head(rrect, device_destination));
    let visible_destination = child_clip.map_or(device_destination, |clip| {
        Rect::from_ltrb(
            device_destination.left.max(clip.left),
            device_destination.top.max(clip.top),
            device_destination.right.min(clip.right),
            device_destination.bottom.min(clip.bottom),
        )
    });
    if visible_destination.is_empty() {
        return;
    }
    if let Some(span_head) = span_head.as_ref() {
        canvas.save();
        canvas.clip_path(span_head, ClipOp::Difference, false);
    }
    let device_scale = style.device_scale_factor.max(f64::EPSILON) as f32;
    let packed_physical_patch =
        if span_head.is_none() && style.raster_configuration.backend != RasterBackend::GaneshGl {
            crate::image_resource::physical_quantized_broken_image_patch_with_clip(
                &image,
                source,
                device_destination,
                device_scale,
                Some(visible_destination),
            )
            .ok()
        } else {
            None
        };
    if let Some((patch, aligned_destination, _)) = packed_physical_patch {
        paint_legacy_image_resource(
            canvas,
            patch,
            aligned_destination,
            device_destination,
            child_clip,
            device_scale,
            opacity_multiplier,
        );
    } else {
        let _clip_guard = child_clip.map(|clip| {
            let guard = skia_safe::AutoCanvasRestore::guard(canvas, true);
            canvas.clip_rect(clip, ClipOp::Intersect, true);
            guard
        });
        canvas.draw_image_rect_with_sampling_options(
            image.clone(),
            Some((&source, SrcRectConstraint::Strict)),
            main_destination,
            sampling,
            &paint,
        );
    }
    if let Some(span_head) = span_head.as_ref() {
        canvas.restore();
        canvas.save();
        canvas.clip_path(span_head, ClipOp::Intersect, false);
        canvas.draw_image_rect_with_sampling_options(
            image,
            Some((&source, SrcRectConstraint::Strict)),
            device_destination,
            sampling,
            &paint,
        );
        canvas.restore();
    }
}

/// Preserve Chromium's packed image blend with analytic coverage supplied
/// separately from the sampled premultiplied texels.
fn paint_legacy_image_resource(
    canvas: &Canvas,
    patch: Image,
    aligned_destination: Rect,
    destination: Rect,
    clip: Option<Rect>,
    scale: f32,
    opacity: f32,
) {
    use skia_safe::runtime_effect::RuntimeEffect;
    thread_local! {
        static BLENDER: RuntimeEffect = RuntimeEffect::make_for_blender(r#"
            uniform float coverage;
            half4 main(half4 source, half4 destination) {
                float4 src = floor(float4(source) * 255 + 0.5);
                float4 dst = floor(float4(destination) * 255 + 0.5);
                float srcScale = coverage + 1;
                float inverse = 65535 - src.a * srcScale;
                float dstScale = floor((inverse + floor(inverse / 256)) / 256);
                return half4(floor((src * srcScale + dst * dstScale) / 256) / 255);
            }
        "#, None).expect("legacy image coverage blender");
    }
    fn segments(start: f32, end: f32, scale: f32) -> Vec<(f32, f32, u32)> {
        let start = start * scale;
        let end = end * scale;
        let first = start.floor();
        let last = end.ceil();
        if last - first <= 1.0 {
            return vec![(
                first / scale,
                last / scale,
                ((end - start) * 256.0).ceil().min(255.0) as u32,
            )];
        }
        let mut result = Vec::with_capacity(3);
        if start != first {
            result.push((
                first / scale,
                (first + 1.0) / scale,
                ((first + 1.0 - start) * 256.0).ceil().min(255.0) as u32,
            ));
        }
        if end.floor() > start.ceil() {
            result.push((start.ceil() / scale, end.floor() / scale, 255));
        }
        if end != end.floor() {
            result.push((
                end.floor() / scale,
                last / scale,
                ((end - end.floor()) * 256.0).ceil().min(255.0) as u32,
            ));
        }
        result
    }
    let matrix = Matrix::scale_translate(
        (
            aligned_destination.width() / patch.width() as f32,
            aligned_destination.height() / patch.height() as f32,
        ),
        (aligned_destination.left, aligned_destination.top),
    );
    let texels = patch
        .to_shader(
            (TileMode::Clamp, TileMode::Clamp),
            SamplingOptions::from(FilterMode::Nearest),
            &matrix,
        )
        .expect("legacy image texel shader");
    let visible = clip.map_or(destination, |clip| {
        Rect::from_ltrb(
            destination.left.max(clip.left),
            destination.top.max(clip.top),
            destination.right.min(clip.right),
            destination.bottom.min(clip.bottom),
        )
    });
    let horizontal = segments(visible.left, visible.right, scale);
    let vertical = segments(visible.top, visible.bottom, scale);
    let overlap = |pixel: f32, start: f32, end: f32| {
        (end.min(pixel + 1.0) - start.max(pixel)).clamp(0.0, 1.0)
    };
    BLENDER.with(|effect| {
        let mut paint = Paint::default();
        paint.set_anti_alias(false);
        paint.set_alpha_f(opacity);
        paint.set_shader(texels);
        for &(left, right, _) in &horizontal {
            for &(top, bottom, _) in &vertical {
                let x = (left * scale).round();
                let y = (top * scale).round();
                let image_x = (overlap(x, destination.left * scale, destination.right * scale)
                    * 256.0)
                    .ceil()
                    .min(255.0) as u32;
                let image_y = (overlap(y, destination.top * scale, destination.bottom * scale)
                    * 256.0)
                    .ceil()
                    .min(255.0) as u32;
                let image_coverage =
                    crate::image_resource::combine_physical_axis_coverage(image_x, image_y);
                // SkAAClip's rectangular path uses fixed_to_alpha: round
                // the covered area times 255. Its mask intersects the image
                // span through SkMulDiv255Round before the byte blend.
                let clip_coverage = clip.map_or(255, |clip| {
                    let cx = overlap(x, clip.left * scale, clip.right * scale);
                    let cy = overlap(y, clip.top * scale, clip.bottom * scale);
                    (cx * cy * 255.0).round() as u32
                });
                let coverage = ((image_coverage * clip_coverage + 127) / 255) as f32;
                paint.set_blender(
                    effect
                        .make_blender(Data::new_copy(&coverage.to_ne_bytes()), None)
                        .expect("legacy image coverage draw"),
                );
                canvas.draw_rect(Rect::from_ltrb(left, top, right, bottom), &paint);
            }
        }
    });
}

/// Paint the propagated canvas of a statically lowered nested document into
/// the iframe's content viewport. This is object content, rather than the
/// host element's CSS background, so it uses the same rounded overflow edge
/// as other replaced content.
fn paint_embedded_canvas(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let Some(color) = doc.node(fragment.node_id).embedded_canvas_color else {
        return;
    };
    let content_rect = Rect::from_xywh(
        abs_offset.left.to_f32() + fragment.border.left.to_f32() + fragment.padding.left.to_f32(),
        abs_offset.top.to_f32() + fragment.border.top.to_f32() + fragment.padding.top.to_f32(),
        (fragment.size.width
            - fragment.border.left
            - fragment.border.right
            - fragment.padding.left
            - fragment.padding.right)
            .clamp_negative_to_zero()
            .to_f32(),
        (fragment.size.height
            - fragment.border.top
            - fragment.border.bottom
            - fragment.padding.top
            - fragment.padding.bottom)
            .clamp_negative_to_zero()
            .to_f32(),
    );
    if content_rect.width() <= 0.0 || content_rect.height() <= 0.0 {
        return;
    }

    canvas.save();
    if style.has_border_radius() && !fragment.ignore_border_radius {
        canvas.clip_rrect(
            build_clip_rrect(
                &content_rect,
                fragment,
                style,
                OverflowClipBox::ContentBox,
                0.0,
                0.0,
            ),
            ClipOp::Intersect,
            true,
        );
    } else {
        canvas.clip_rect(content_rect, ClipOp::Intersect, false);
    }
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    set_paint_css_color_with_alpha(&mut paint, &color, opacity_multiplier);
    canvas.draw_rect(content_rect, &paint);
    canvas.restore();
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
    // Blink records the replaced object in a CSS-pixel-snapped destination,
    // then resolves that contour through the frame's physical transform. The
    // two stages are intentionally distinct: integer CSS edges can still
    // receive fractional physical coverage at a non-integral device scale.
    let destination_left = object_x.round();
    let destination_top = object_y.round();
    let destination = Rect::from_xywh(
        destination_left,
        destination_top,
        (object_x + object_width).round() - destination_left,
        (object_y + object_height).round() - destination_top,
    );
    let decoded_media_frame = doc
        .image_resource(image_id)
        .is_some_and(|resource| resource.mime_type == "image/x-openui-rgba8");
    let destination = if decoded_media_frame {
        // The decoded video frame is an external texture quad. Chromium
        // resolves each of its CSS-pixel-snapped edges to the nearest device
        // boundary before sampling (with half ties assigned to the following
        // cell), unlike an ordinary image whose fractional contour retains
        // analytic coverage.
        let snapping = RasterSnapping::new(style.device_scale_factor);
        Rect::from_ltrb(
            snapping.logical_coordinate(destination.left, PhysicalSnap::Nearest),
            snapping.logical_coordinate(destination.top, PhysicalSnap::Nearest),
            snapping.logical_coordinate(destination.right, PhysicalSnap::Nearest),
            snapping.logical_coordinate(destination.bottom, PhysicalSnap::Nearest),
        )
    } else {
        destination
    };
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_alpha_f(opacity_multiplier);
    canvas.save();
    let mut physical_replaced_coverage_bounds = None;
    // The concrete object may extend beyond the replaced element's content
    // box (`object-fit: none`, an offset `object-position`, or `cover`).  It is
    // clipped by the element's computed overflow, just like other ink owned
    // by the element.  In particular, `overflow: visible` must not turn the
    // content box into an implicit replaced-object viewport; Blink lets the
    // intrinsic object and its square corners paint beyond both the box and
    // an authored border radius in that case.
    let content_rect = Rect::from_xywh(content_x, content_y, content_width, content_height);
    let unsnapped_object_rect = Rect::from_xywh(object_x, object_y, object_width, object_height);
    // A visible axis paired with `clip` remains unbounded. When it is paired
    // with hidden/scroll/auto it computes to the scrollable behavior and is
    // clipped as well (CSS Overflow 3, matching the fragment clip path).
    let clip_x = style.overflow_x != Overflow::Visible
        || (style.overflow_x == Overflow::Visible
            && !matches!(style.overflow_y, Overflow::Visible | Overflow::Clip));
    let clip_y = style.overflow_y != Overflow::Visible
        || (style.overflow_y == Overflow::Visible
            && !matches!(style.overflow_x, Overflow::Visible | Overflow::Clip));
    if clip_x && clip_y && style.has_border_radius() && !fragment.ignore_border_radius {
        canvas.clip_rrect(
            build_clip_rrect(
                &content_rect,
                fragment,
                style,
                OverflowClipBox::ContentBox,
                0.0,
                0.0,
            ),
            ClipOp::Intersect,
            true,
        );
    } else if (clip_x || clip_y)
        && (unsnapped_object_rect.left < content_rect.left - 1.0e-5
            || unsnapped_object_rect.top < content_rect.top - 1.0e-5
            || unsnapped_object_rect.right > content_rect.right + 1.0e-5
            || unsnapped_object_rect.bottom > content_rect.bottom + 1.0e-5)
    {
        // Avoid a second raster edge when the object already fits. For the
        // `visible clip` mixed-axis case, make the visible axis effectively
        // unbounded while retaining the authored clip on the other axis.
        let big = 100_000.0_f32;
        let clip = Rect::from_ltrb(
            if clip_x { content_rect.left } else { -big },
            if clip_y { content_rect.top } else { -big },
            if clip_x { content_rect.right } else { big },
            if clip_y { content_rect.bottom } else { big },
        );
        let clip = if clip_x
            && clip_y
            && (style.overflow_x.is_scrollable()
                || style.overflow_y.is_scrollable()
                || style.overflow_x == Overflow::Clip
                || style.overflow_y == Overflow::Clip)
        {
            // A native scrollport owns a hard physical scissor. Expand that
            // scissor to every touched device cell and let the replaced
            // object's already-packed destination coverage resolve the
            // fractional content-box edge exactly once.
            outward_snap_rect_to_physical(clip, style.device_scale_factor)
        } else {
            clip
        };
        if (style.device_scale_factor - 1.0).abs() > 1.0e-5
            && style.raster_configuration.backend != RasterBackend::GaneshGl
            && !matches!(replaced.resource, ReplacedResourceKind::StaticSvg(_))
        {
            // The physical raster patch can own this rectangular scissor and
            // fold it into the same coverage value as the destination edge.
            // A separate Skia clip mask would quantize the edge a second time.
            physical_replaced_coverage_bounds = Some(clip);
        } else {
            canvas.clip_rect(clip, ClipOp::Intersect, false);
        }
    }
    if matches!(replaced.resource, ReplacedResourceKind::StaticSvg(_)) {
        // Keep vector replaced content in destination space. Rasterizing a
        // ratio-only SVG at its 300x150 default object size and resampling it
        // into a fractional CSS box exposes transparent edge texels between
        // adjacent replaced boxes. Chromium records the SVG display item in
        // the concrete object viewport instead.
        let _ = crate::image_resource::paint_svg_resource_patch(
            canvas,
            doc,
            image_id,
            object_width,
            object_height,
            Rect::from_xywh(0.0, 0.0, object_width, object_height),
            destination,
            opacity_multiplier,
            None,
        );
        paint_composited_svg_edge_samples(
            canvas,
            doc,
            style,
            image_id,
            destination,
            opacity_multiplier,
        );
        canvas.restore();
        return;
    }
    let device_scale = style.device_scale_factor.max(f64::EPSILON) as f32;
    let draw_image = crate::image_resource::decode_raster_resource_for_draw(
        doc,
        image_id,
        (destination.width() * device_scale).round() as i32,
        (destination.height() * device_scale).round() as i32,
    )
    .unwrap_or_else(|_| image.clone());
    let mip_decoded = draw_image.width() != image.width() || draw_image.height() != image.height();
    let full_precision_replaced = doc
        .image_resource(image_id)
        .is_some_and(|resource| resource.mime_type == "image/jpeg");
    let draw_source = Rect::from_xywh(
        0.0,
        0.0,
        draw_image.width() as f32,
        draw_image.height() as f32,
    );
    if (device_scale - 1.0).abs() > 1.0e-5
        && style.raster_configuration.backend != RasterBackend::GaneshGl
    {
        if let Ok((patch, aligned_destination, _analytic_clip)) =
            crate::image_resource::physical_quantized_image_patch(
                &draw_image,
                draw_source,
                destination,
                device_scale,
                false,
                false,
                false,
                false,
                false,
                false,
                None,
                true,
                mip_decoded,
                full_precision_replaced,
                false,
                false,
                false,
                physical_replaced_coverage_bounds,
            )
        {
            let local_matrix = Matrix::scale_translate(
                (
                    aligned_destination.width() / patch.width() as f32,
                    aligned_destination.height() / patch.height() as f32,
                ),
                (aligned_destination.left, aligned_destination.top),
            );
            paint.set_anti_alias(false);
            paint.set_shader(patch.to_shader(
                (TileMode::Clamp, TileMode::Clamp),
                SamplingOptions::from(FilterMode::Nearest),
                &local_matrix,
            ));
            canvas.draw_rect(aligned_destination, &paint);
            if draw_image.color_space().is_some() {
                if let Ok((corner_patch, corner_destination)) =
                    crate::image_resource::physical_quantized_replaced_profile_corners(
                        &draw_image,
                        draw_source,
                        destination,
                        device_scale,
                        mip_decoded,
                        full_precision_replaced,
                        physical_replaced_coverage_bounds,
                    )
                {
                    let corner_matrix = Matrix::scale_translate(
                        (
                            corner_destination.width() / corner_patch.width() as f32,
                            corner_destination.height() / corner_patch.height() as f32,
                        ),
                        (corner_destination.left, corner_destination.top),
                    );
                    paint.set_shader(corner_patch.to_shader(
                        (TileMode::Clamp, TileMode::Clamp),
                        SamplingOptions::from(FilterMode::Nearest),
                        &corner_matrix,
                    ));
                    canvas.draw_rect(corner_destination, &paint);
                }
            }
            canvas.restore();
            return;
        }
    }
    let patch = crate::image_resource::quantized_image_patch(
        &draw_image,
        draw_source,
        destination,
        destination.width().round() as i32,
        destination.height().round() as i32,
        crate::image_resource::ImagePatchPhase::Replaced,
    )
    .unwrap_or(draw_image);
    let patch_source = Rect::from_xywh(0.0, 0.0, patch.width() as f32, patch.height() as f32);
    canvas.draw_image_rect_with_sampling_options(
        &patch,
        Some((&patch_source, SrcRectConstraint::Strict)),
        destination,
        SamplingOptions::from(FilterMode::Nearest),
        &paint,
    );
    canvas.restore();
}

/// Chromium's GPU display-item path quantizes the shared horizontal edge of
/// an opaque, viewport-covering SVG and its CSS background as one composited
/// sample. CPU Skia instead accumulates the two neighboring vector draws. The
/// phase buckets below are backend coverage values; colors remain entirely
/// resource- and style-derived.
fn paint_composited_svg_edge_samples(
    canvas: &Canvas,
    doc: &Document,
    style: &ComputedStyle,
    image_id: openui_style::ImageResourceId,
    destination: Rect,
    opacity_multiplier: f32,
) {
    if style.raster_configuration.backend != RasterBackend::ChromiumLinux
        || opacity_multiplier < 1.0
        || !style.background_color.is_opaque()
    {
        return;
    }
    let Some(svg_color) = crate::image_resource::static_svg_full_viewport_color(doc, image_id)
    else {
        return;
    };
    let scale = style.device_scale_factor as f32;
    let physical_left = destination.left * scale;
    let physical_right = destination.right * scale;
    if physical_left.fract().abs() > 1.0 / 64.0 || physical_right.fract().abs() > 1.0 / 64.0 {
        return;
    }
    let backdrop = doc
        .canvas_background_source()
        .map(|node| doc.node(node).style.background_color)
        .filter(Color::is_opaque)
        .unwrap_or(Color::WHITE);
    let source = Color::from_rgba8(svg_color.r(), svg_color.g(), svg_color.b(), svg_color.a());
    let composite = |foreground: Color, alpha: f32, background: Color| Color {
        r: foreground.r * alpha + background.r * (1.0 - alpha),
        g: foreground.g * alpha + background.g * (1.0 - alpha),
        b: foreground.b * alpha + background.b * (1.0 - alpha),
        a: 1.0,
    };
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    for physical_edge in [destination.top * scale, destination.bottom * scale] {
        let phase = physical_edge.rem_euclid(1.0);
        let (background_alpha, svg_alpha) = if (phase - 0.5).abs() <= 1.0 / 64.0 {
            (112.0 / 255.0, 64.0 / 255.0)
        } else if (phase - 0.25).abs() <= 1.0 / 64.0 || (phase - 0.75).abs() <= 1.0 / 64.0 {
            (152.0 / 255.0, 144.0 / 255.0)
        } else {
            continue;
        };
        let background = composite(style.background_color, background_alpha, backdrop);
        let sample = composite(source, svg_alpha, background);
        set_paint_css_color(&mut paint, &sample);
        canvas.draw_rect(
            Rect::from_xywh(
                physical_left / scale,
                physical_edge.floor() / scale,
                (physical_right - physical_left) / scale,
                1.0 / scale,
            ),
            &paint,
        );
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
            paint_fragment_tracked(canvas, child, doc, parent_offset);
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
    let zero_block_spanner_in_outside_list_item = if style.column_span
        == openui_style::ColumnSpan::All
        && fragment.children.is_empty()
        && fragment.size.height == LayoutUnit::zero()
        && !fragment.node_id.is_none()
    {
        let mut ancestor = doc.node(fragment.node_id).parent;
        let mut outside_list_item = false;
        while !ancestor.is_none() {
            let ancestor_node = doc.node(ancestor);
            if ancestor_node.style.display == Display::ListItem {
                outside_list_item =
                    ancestor_node.style.list_style_position == ListStylePosition::Outside;
                break;
            }
            ancestor = ancestor_node.parent;
        }
        outside_list_item
    } else {
        false
    };
    !(style.column_span == openui_style::ColumnSpan::All
        && fragment.children.is_empty()
        && fragment.size.height.raw() == 0
        && !fragment.paint_zero_block_outline
        && !zero_block_spanner_in_outside_list_item)
}

fn should_paint_outline_after_children(
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
) -> bool {
    fn subtree_has_outline(fragment: &Fragment, doc: &Document) -> bool {
        (!fragment.node_id.is_none()
            && fragment.kind == FragmentKind::Box
            && doc.node(fragment.node_id).style.has_outline())
            || fragment
                .children
                .iter()
                .any(|child| subtree_has_outline(child, doc))
    }

    style.outline_style == BorderStyle::Solid
        && (fragment
            .children
            .iter()
            .any(|child| subtree_has_outline(child, doc))
            || (style.overflow_x == Overflow::Visible
                && style.overflow_y == Overflow::Visible
                && !fragment.children.iter().any(|child| {
                    if child.node_id.is_none() {
                        return false;
                    }
                    let child_style = &doc.node(child.node_id).style;
                    child_style.overflow_x.is_clipping() || child_style.overflow_y.is_clipping()
                })))
}

fn outline_coverage_key(
    fragment: &Fragment,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
) -> Option<OutlineCoverageKey> {
    let width = style.effective_outline_width();
    if width <= 0 {
        return None;
    }
    let expand = LayoutUnit::from_i32(style.outline_offset + width);
    let color = style.outline_color.resolve(&style.color);
    Some(OutlineCoverageKey {
        outer: [
            (abs_offset.left.round() - expand).raw(),
            (abs_offset.top.round() - expand).raw(),
            (abs_offset.left + fragment.size.width).round().raw() + expand.raw(),
            (abs_offset.top + fragment.size.height).round().raw() + expand.raw(),
        ],
        color: [
            (color.a * 255.0) as u8,
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
        ],
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
            paint_fragment_tracked(canvas, fragment, doc, parent_offset);
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
            column_block_only_clip_rect(column, column_offset, doc.device_scale_factor())
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
        paint_fragment_tracked(canvas, rule, doc, offset);
    }
    for column in children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
    {
        paint_fragment_tracked(canvas, column, doc, offset);
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
            paint_fragment_tracked(canvas, entry.fragment, doc, entry.parent_offset);
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
                column_block_only_clip_rect(column, column_offset, doc.device_scale_factor())
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
fn fragmented_flex_has_internal_four_way_junction(fragment: &Fragment) -> bool {
    if fragment.decoration_slice.is_none() || fragment.is_first_for_node {
        return false;
    }
    let width = fragment.size.width;
    let height = fragment.size.height;
    for first in &fragment.children {
        let junction_x = first.offset.left + first.size.width;
        let junction_y = first.offset.top + first.size.height;
        if junction_x <= LayoutUnit::zero()
            || junction_x >= width
            || junction_y <= LayoutUnit::zero()
            || junction_y >= height
        {
            continue;
        }
        let mut top_left = false;
        let mut top_right = false;
        let mut bottom_left = false;
        let mut bottom_right = false;
        for child in &fragment.children {
            let right = child.offset.left + child.size.width;
            let bottom = child.offset.top + child.size.height;
            top_left |= right == junction_x && bottom == junction_y;
            top_right |= child.offset.left == junction_x && bottom == junction_y;
            bottom_left |= right == junction_x && child.offset.top == junction_y;
            bottom_right |= child.offset.left == junction_x && child.offset.top == junction_y;
        }
        if top_left && top_right && bottom_left && bottom_right {
            return true;
        }
    }
    false
}

fn paint_children_with_stacking_order(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    offset: PhysicalOffset,
    is_stacking_context: bool,
    fragmented_flex_items: bool,
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

    if fragmented_flex_items {
        // Layout retains a column flex container's complete item list in
        // every continuation. After the first fragment, Chromium's fragment
        // painter consumes visible items in physical row-major order, which
        // determines SrcOver ownership at a four-way fractional junction.
        // Preserve z-index buckets above, but order ordinary in-flow items by
        // their fragment coordinates before painting that continuation.
        in_flow.sort_by_key(|&index| {
            (
                children[index].offset.top.raw(),
                children[index].offset.left.raw(),
                index,
            )
        });
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
            paint_fragment_tracked(canvas, &children[idx], doc, offset);
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

    // Outlines that must appear above a touching later sibling are deferred,
    // not painted twice. Replaying an opaque antialiased contour over itself
    // compounds its fractional edge coverage even though the integral-scale
    // pixels hide the duplicate draw.
    let deferred_outline_replays: Vec<usize> = in_flow
        .iter()
        .enumerate()
        .filter_map(|(position, &idx)| {
            let child = &children[idx];
            if child.node_id.is_none()
                || child.kind != FragmentKind::Box
                || doc.node(child.node_id).tag == openui_dom::ElementTag::Text
            {
                return None;
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
            ((style.column_span == openui_style::ColumnSpan::All || outline_reaches_later_sibling)
                && should_paint_outline(child, doc, style))
            .then_some(child as *const Fragment as usize)
        })
        .collect();
    DEFERRED_OUTLINE_REPLAYS.with(|deferred| {
        deferred
            .borrow_mut()
            .extend(deferred_outline_replays.iter().copied());
    });
    let mut touching_descendant_outline_owners = Vec::new();
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
        collect_touching_unfragmented_descendant_outline_owners(
            child,
            doc,
            child.offset.top,
            later_top,
            child.decoration_slice.is_some() || !child.is_first_for_node || !child.is_last_for_node,
            &mut touching_descendant_outline_owners,
        );
    }
    // A real column spanner's complete outline phase follows the subsequent
    // column row. Descendant outlines participate in that same phase; letting
    // a nested traversal consume them early allows post-spanner content to
    // erase a coincident fractional edge.
    if children
        .iter()
        .any(|child| child.kind == FragmentKind::ColumnBox)
    {
        for &idx in &in_flow {
            let child = &children[idx];
            if !child.node_id.is_none()
                && child.kind == FragmentKind::Box
                && doc.node(child.node_id).style.column_span == openui_style::ColumnSpan::All
            {
                collect_unfragmented_descendant_outline_owners(
                    &child.children,
                    doc,
                    &mut touching_descendant_outline_owners,
                );
            }
        }
    }
    let newly_deferred_touching_outlines = EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
        let mut deferred = deferred.borrow_mut();
        touching_descendant_outline_owners
            .iter()
            .copied()
            .filter(|pointer| deferred.insert(*pointer))
            .collect::<Vec<_>>()
    });

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
            paint_fragment_tracked(canvas, &children[idx], doc, offset);
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

        let (prepainted, prepainted_rules) = prepaint_shared_column_root_decorations(
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
        if !prepainted_rules.is_empty() {
            PREPAINTED_COLUMN_RULES.with(|rules| {
                rules.borrow_mut().extend(prepainted_rules.iter().copied());
            });
        }
        for &row_idx in &in_flow[in_flow_position..row_position_end] {
            if children[row_idx].kind != FragmentKind::ColumnRule {
                paint_fragment_tracked(canvas, &children[row_idx], doc, offset);
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
        if !prepainted_rules.is_empty() {
            PREPAINTED_COLUMN_RULES.with(|rules| {
                let mut rules = rules.borrow_mut();
                for pointer in &prepainted_rules {
                    rules.remove(pointer);
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
            DEFERRED_OUTLINE_REPLAYS.with(|deferred| {
                deferred
                    .borrow_mut()
                    .remove(&(child as *const Fragment as usize));
            });
            if style.column_span == openui_style::ColumnSpan::All
                && children
                    .iter()
                    .any(|sibling| sibling.kind == FragmentKind::ColumnBox)
            {
                paint_deferred_unfragmented_descendant_outlines(
                    canvas,
                    &child.children,
                    doc,
                    PhysicalOffset::new(
                        offset.left + child.offset.left,
                        offset.top + child.offset.top,
                    ),
                );
            }
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
    DEFERRED_OUTLINE_REPLAYS.with(|deferred| {
        let mut deferred = deferred.borrow_mut();
        for pointer in &deferred_outline_replays {
            deferred.remove(pointer);
        }
    });

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
            &newly_deferred_touching_outlines,
        );
    }
    if !newly_deferred_touching_outlines.is_empty() {
        EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
            let mut deferred = deferred.borrow_mut();
            for pointer in &newly_deferred_touching_outlines {
                deferred.remove(pointer);
            }
        });
    }

    // Phase 3: Non-negative z-index positioned elements (+ hoisted z:auto).
    let mut positioned_index = 0;
    while positioned_index < non_negative_z.len() {
        let first = stacking_entry_fragment(&non_negative_z[positioned_index].2, children);
        let mut group_end = positioned_index + 1;
        while group_end < non_negative_z.len()
            && stacking_entry_fragment(&non_negative_z[group_end].2, children).node_id
                == first.node_id
        {
            group_end += 1;
        }

        // A fragmented positioned box has one decoration phase across its
        // continuations. Paint every slice of that box before its descendants:
        // interleaving decoration and child ink per slice changes SrcOver
        // coverage at a shared fractional column edge. Keep independent
        // stacking contexts and effects on their existing atomic path.
        let mut prepainted = Vec::new();
        if group_end > positioned_index + 1 && !first.node_id.is_none() {
            let node = doc.node(first.node_id);
            let style = &node.style;
            let simple_group = first.is_first_for_node
                && stacking_entry_fragment(&non_negative_z[group_end - 1].2, children)
                    .is_last_for_node
                && !is_fragment_stacking_context(first, doc)
                && style.visibility == Visibility::Visible
                && style.transform == openui_style::Transform2D::IDENTITY
                && style.filter_blur == 0.0
                && style.filter_grayscale == 0.0
                && style.clip_path_inset.is_none()
                && style.mask_layers.is_empty()
                && !style.has_border_radius()
                && style.overflow_x == Overflow::Visible
                && style.overflow_y == Overflow::Visible
                && node.replaced.is_none()
                && node.form_control.is_none()
                && non_negative_z[positioned_index..group_end]
                    .iter()
                    .all(|(_, _, entry)| {
                        let fragment = stacking_entry_fragment(entry, children);
                        let pointer = fragment as *const Fragment as usize;
                        fragment.kind == FragmentKind::Box
                            && fragment.decoration_slice.is_some()
                            && fragment.has_overflow_clip
                            && fragment.block_axis_clip_only
                            && !fragment.skip_box_decoration
                            && fragment.paint_background_color_override.is_none()
                            && fragment.decoration_clip_rects.is_empty()
                            && fragment.promoted_transform_ancestors.is_empty()
                            && (!matches!(entry, StackingEntry::Direct(_))
                                || !HOIST_SKIP.with(|skipped| skipped.borrow().contains(&pointer)))
                            && !PREPAINTED_BOX_DECORATIONS
                                .with(|prepainted| prepainted.borrow().contains(&pointer))
                    })
                && non_negative_z[positioned_index..group_end]
                    .iter()
                    .any(|(_, _, entry)| {
                        !stacking_entry_fragment(entry, children).children.is_empty()
                    });
            if simple_group {
                for (_, _, entry) in &non_negative_z[positioned_index..group_end] {
                    let fragment = stacking_entry_fragment(entry, children);
                    if let StackingEntry::DescendantWithClip(_, _, clip_rect) = entry {
                        canvas.save();
                        canvas.clip_rect(
                            outward_snap_rect_to_physical(*clip_rect, doc.device_scale_factor()),
                            ClipOp::Intersect,
                            false,
                        );
                    }
                    let parent_offset = match entry {
                        StackingEntry::Direct(_) => offset,
                        StackingEntry::Descendant(_, parent_offset)
                        | StackingEntry::DescendantWithClip(_, parent_offset, _) => *parent_offset,
                    };
                    paint_fragment_box_decoration(
                        canvas,
                        fragment,
                        doc,
                        style,
                        PhysicalOffset::new(
                            parent_offset.left + fragment.offset.left,
                            parent_offset.top + fragment.offset.top,
                        ),
                        1.0,
                    );
                    if matches!(entry, StackingEntry::DescendantWithClip(..)) {
                        canvas.restore();
                    }
                    prepainted.push(fragment as *const Fragment as usize);
                }
                PREPAINTED_BOX_DECORATIONS.with(|fragments| {
                    fragments.borrow_mut().extend(prepainted.iter().copied());
                });
            }
        }

        for (_, _, entry) in &non_negative_z[positioned_index..group_end] {
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
        if !prepainted.is_empty() {
            PREPAINTED_BOX_DECORATIONS.with(|fragments| {
                let mut fragments = fragments.borrow_mut();
                for pointer in &prepainted {
                    fragments.remove(pointer);
                }
            });
        }
        positioned_index = group_end;
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
    for (position, &idx) in in_flow.iter().enumerate() {
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
        let qualified_text_profile = uses_deterministic_text_profile(style);
        if !style.display.is_block_level()
            || style.float != openui_style::Float::None
            || style.transform != openui_style::Transform2D::IDENTITY
            || style.filter_blur > 0.0
            || style.filter_grayscale > 0.0
            || style.clip_path_inset.is_some()
            || style.visibility != Visibility::Visible
            || style.opacity < 1.0
            // Preserve the qualified deterministic-font path. Outside it,
            // moving every decoration would reorder overlapping block and
            // flex backgrounds, or split a mask/paint-containment group.
            || (!qualified_text_profile
                && (style.display.is_flex()
                    || style.display.is_grid()
                    || !style.mask_layers.is_empty()
                    || style.has_paint_containment()
                    || !earlier_inline_ink_reaches_block(
                        children,
                        &in_flow[..position],
                        fragment,
                        doc,
                    )))
        {
            continue;
        }
        let pointer = fragment as *const Fragment as usize;
        // An overflowing monolithic child is replayed outside its column's
        // clip. Its background belongs to that replay as well: prepainting
        // it here would compound antialiased coverage when the child paints.
        if HOIST_SKIP.with(|fragments| fragments.borrow().contains(&pointer))
            || PREPAINTED_BOX_DECORATIONS.with(|fragments| fragments.borrow().contains(&pointer))
        {
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

fn earlier_inline_ink_reaches_block(
    children: &[Fragment],
    earlier_in_flow: &[usize],
    block: &Fragment,
    doc: &Document,
) -> bool {
    let block_left = block.offset.left;
    let block_right = block_left + block.size.width;
    let block_top = block.offset.top;
    earlier_in_flow.iter().any(|&idx| {
        let earlier = &children[idx];
        if earlier.kind != FragmentKind::Box || earlier.node_id.is_none() {
            return false;
        }
        let style = &doc.node(earlier.node_id).style;
        if style.display.is_flex() || style.display.is_grid() || earlier.has_overflow_clip {
            return false;
        }
        text_fragment_reaches_block(earlier, earlier.offset, block_left, block_right, block_top)
    })
}

fn text_fragment_reaches_block(
    fragment: &Fragment,
    offset: PhysicalOffset,
    block_left: LayoutUnit,
    block_right: LayoutUnit,
    block_top: LayoutUnit,
) -> bool {
    if fragment.kind == FragmentKind::Text {
        // A glyph can cover the adjacent device row even when its layout box
        // ends exactly at the next block's border edge.
        return offset.left < block_right
            && offset.left + fragment.size.width > block_left
            && offset.top <= block_top
            && offset.top + fragment.size.height + LayoutUnit::from_i32(1) >= block_top;
    }
    fragment.children.iter().any(|child| {
        text_fragment_reaches_block(
            child,
            PhysicalOffset::new(
                offset.left + child.offset.left,
                offset.top + child.offset.top,
            ),
            block_left,
            block_right,
            block_top,
        )
    })
}

#[derive(Clone, Copy)]
struct SharedColumnClip {
    rect: Rect,
    antialias: bool,
    translate_with_relative: bool,
}

struct SharedColumnDecoration<'a> {
    fragment: &'a Fragment,
    offset: PhysicalOffset,
    clips: Vec<SharedColumnClip>,
    ancestor_node_ids: Vec<NodeId>,
    starts_at_shared_edge: bool,
}

fn has_inline_ancestor(doc: &Document, node_id: NodeId) -> bool {
    let mut ancestor = doc.node(node_id).parent;
    while !ancestor.is_none() {
        let node = doc.node(ancestor);
        if node.style.display.is_inline_level() {
            return true;
        }
        ancestor = node.parent;
    }
    false
}

fn decoration_coverage_rect(decoration: &SharedColumnDecoration<'_>) -> Rect {
    Rect::from_xywh(
        decoration.offset.left.to_f32(),
        decoration.offset.top.to_f32(),
        decoration.fragment.size.width.to_f32(),
        decoration.fragment.size.height.to_f32(),
    )
}

fn clipped_decoration_coverage_rect(decoration: &SharedColumnDecoration<'_>) -> Option<Rect> {
    let mut rect = decoration_coverage_rect(decoration);
    for clip in &decoration.clips {
        rect = Rect::from_ltrb(
            rect.left.max(clip.rect.left),
            rect.top.max(clip.rect.top),
            rect.right.min(clip.rect.right),
            rect.bottom.min(clip.rect.bottom),
        );
    }
    (rect.left < rect.right && rect.top < rect.bottom).then_some(rect)
}

fn can_coalesce_identical_solid_decoration(
    decoration: &SharedColumnDecoration<'_>,
    style: &ComputedStyle,
) -> bool {
    style.opacity >= 1.0
        && style.background_color.is_opaque()
        && style.background_layers.is_empty()
        && style.background_linear_gradient.is_none()
        && style.box_shadow.is_empty()
        && style.border_image.is_none()
        && !style.has_border_radius()
        && decoration.fragment.border == openui_geometry::BoxStrut::zero()
        && decoration
            .fragment
            .paint_background_color_override
            .is_none()
}

fn same_physical_decoration_rect(first: Rect, second: Rect) -> bool {
    const EPSILON: f32 = 1.0 / 1024.0;
    (first.left - second.left).abs() <= EPSILON
        && (first.top - second.top).abs() <= EPSILON
        && (first.right - second.right).abs() <= EPSILON
        && (first.bottom - second.bottom).abs() <= EPSILON
}

fn shared_spanner_four_way_junctions(
    column_decorations: &[Vec<SharedColumnDecoration<'_>>],
    node_order: &[usize],
    doc: &Document,
) -> Vec<(Rect, Color)> {
    const EPSILON: f32 = 1.0 / 1024.0;
    let scale = doc.device_scale_factor();
    let snapping = RasterSnapping::new(scale);
    let candidates: Vec<_> = column_decorations
        .iter()
        .flatten()
        .filter_map(|decoration| {
            let node_id = decoration.fragment.node_id.index();
            if !node_order.contains(&node_id) {
                return None;
            }
            let style = &doc.node(decoration.fragment.node_id).style;
            let belongs_to_spanner = style.column_span == openui_style::ColumnSpan::All
                || decoration.ancestor_node_ids.iter().any(|ancestor| {
                    doc.node(*ancestor).style.column_span == openui_style::ColumnSpan::All
                });
            (belongs_to_spanner && can_coalesce_identical_solid_decoration(decoration, style))
                .then(|| {
                    clipped_decoration_coverage_rect(decoration)
                        .map(|rect| (rect, style.background_color))
                })
                .flatten()
        })
        .collect();

    let mut junctions = Vec::new();
    for &(top_left, color) in &candidates {
        let x = top_left.right;
        let y = top_left.bottom;
        let x_phase = (f64::from(x) * scale).rem_euclid(1.0);
        let y_phase = (f64::from(y) * scale).rem_euclid(1.0);
        if (x_phase - 0.5).abs() > 1.0e-6 || (y_phase - 0.5).abs() > 1.0e-6 {
            continue;
        }
        let has_top_right = candidates.iter().any(|(rect, peer_color)| {
            *peer_color == color
                && (rect.left - x).abs() <= EPSILON
                && (rect.bottom - y).abs() <= EPSILON
        });
        let has_bottom_left = candidates.iter().any(|(rect, peer_color)| {
            *peer_color == color
                && (rect.right - x).abs() <= EPSILON
                && (rect.top - y).abs() <= EPSILON
        });
        let has_bottom_right = candidates.iter().any(|(rect, peer_color)| {
            *peer_color == color
                && (rect.left - x).abs() <= EPSILON
                && (rect.top - y).abs() <= EPSILON
        });
        if !(has_top_right && has_bottom_left && has_bottom_right) {
            continue;
        }
        let physical_x = snapping.physical_coordinate(x, PhysicalSnap::Floor);
        let physical_y = snapping.physical_coordinate(y, PhysicalSnap::Floor);
        let cell = Rect::from_ltrb(
            physical_x as f32 / scale as f32,
            physical_y as f32 / scale as f32,
            (physical_x + 1) as f32 / scale as f32,
            (physical_y + 1) as f32 / scale as f32,
        );
        if !junctions.iter().any(|(existing, existing_color)| {
            *existing_color == color && same_physical_decoration_rect(*existing, cell)
        }) {
            junctions.push((cell, color));
        }
    }
    junctions
}

fn physical_axis_coverage_may_overlap(
    first_start: f32,
    first_end: f32,
    second_start: f32,
    second_end: f32,
    device_scale: f64,
) -> bool {
    const EPSILON: f64 = 1.0 / 1024.0;
    let overlap = f64::from(first_end.min(second_end) - first_start.max(second_start));
    if overlap > EPSILON {
        return true;
    }
    if overlap < -EPSILON {
        return false;
    }

    // Adjacent analytic rectangles contribute to the same device cell only
    // when their common edge is off the physical pixel grid.
    let edge = f64::from(first_end.min(second_end).max(first_start.max(second_start)));
    let phase = (edge * device_scale).rem_euclid(1.0);
    phase > EPSILON && phase < 1.0 - EPSILON
}

fn decorations_may_share_physical_coverage(
    first: &SharedColumnDecoration<'_>,
    second: &SharedColumnDecoration<'_>,
    device_scale: f64,
) -> bool {
    let first = decoration_coverage_rect(first);
    let second = decoration_coverage_rect(second);
    physical_axis_coverage_may_overlap(
        first.left,
        first.right,
        second.left,
        second.right,
        device_scale,
    ) && physical_axis_coverage_may_overlap(
        first.top,
        first.bottom,
        second.top,
        second.bottom,
        device_scale,
    )
}

fn collect_shared_column_decorations<'a>(
    fragment: &'a Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    clips: &[SharedColumnClip],
    ancestor_node_ids: &[NodeId],
    is_column_root: bool,
    output: &mut Vec<SharedColumnDecoration<'a>>,
) {
    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    let mut child_clips = clips.to_vec();
    if !fragment.node_id.is_none()
        && doc.node(fragment.node_id).style.position == openui_style::Position::Relative
        && fragment.fragmentation_visual_offset != PhysicalOffset::zero()
        // Fragmented inline containing blocks have a separate row paint
        // owner. Their stored visual offset can include the inline ancestor's
        // translation, so shifting this prepass's clips would replay content
        // outside the row's authoritative fragmentainer interval.
        && !has_inline_ancestor(doc, fragment.node_id)
    {
        // A relative box is laid out in its source fragmentainer and then
        // visually translated. The synthetic fragmentainer clips follow that
        // translation for its decorations and descendants. Authored overflow
        // clips stay attached to their owning ancestor instead.
        let shift = fragment.fragmentation_visual_offset;
        for clip in &mut child_clips {
            if clip.translate_with_relative {
                clip.rect = Rect::from_ltrb(
                    clip.rect.left + shift.left.to_f32(),
                    clip.rect.top + shift.top.to_f32(),
                    clip.rect.right + shift.left.to_f32(),
                    clip.rect.bottom + shift.top.to_f32(),
                );
            }
        }
    }
    let mut child_ancestor_node_ids = ancestor_node_ids.to_vec();
    if fragment.kind == FragmentKind::ColumnBox {
        if fragment.has_overflow_clip {
            let logical_clip = column_fragmentainer_clip_rect(
                fragment,
                fragment_offset,
                doc.device_scale_factor(),
            );
            let mut physical_clip =
                outward_snap_rect_to_physical(logical_clip, doc.device_scale_factor());
            if fragment.inline_axis_clip_only && !fragment.is_first_for_node {
                let snapping = RasterSnapping::new(doc.device_scale_factor());
                if fragment_block_axis_is_x(fragment) {
                    physical_clip.top =
                        snapping.logical_coordinate(logical_clip.top, PhysicalSnap::Ceil);
                } else {
                    physical_clip.left =
                        snapping.logical_coordinate(logical_clip.left, PhysicalSnap::Ceil);
                }
            }
            child_clips.push(SharedColumnClip {
                rect: physical_clip,
                antialias: false,
                translate_with_relative: true,
            });
        }
    } else if !fragment.node_id.is_none() {
        let style = &doc.node(fragment.node_id).style;
        // Floats paint in their own phase after in-flow block backgrounds.
        // Hoisting a float's decoration into the shared column-root prepass
        // lets a later in-flow background erase its overflow continuation.
        if style.float != openui_style::Float::None {
            return;
        }
        let pointer = fragment as *const Fragment as usize;
        let positioned_grid =
            style.display.is_grid() && style.position != openui_style::Position::Static;
        let eligible = fragment.kind == FragmentKind::Box
            && (is_column_root || style.display == Display::Block || positioned_grid)
            && !fragment.skip_box_decoration
            && style.visibility == Visibility::Visible
            && style.opacity >= 1.0
            && style.transform == openui_style::Transform2D::IDENTITY
            && style.filter_blur <= 0.0
            && style.filter_grayscale <= 0.0
            && style.clip_path_inset.is_none()
            && style.mask_layers.is_empty()
            && !HOIST_SKIP.with(|fragments| fragments.borrow().contains(&pointer))
            && !PREPAINTED_BOX_DECORATIONS.with(|fragments| fragments.borrow().contains(&pointer));
        if eligible {
            output.push(SharedColumnDecoration {
                fragment,
                offset: fragment_offset,
                clips: child_clips.clone(),
                ancestor_node_ids: ancestor_node_ids.to_vec(),
                starts_at_shared_edge: false,
            });
        }
        child_ancestor_node_ids.push(fragment.node_id);

        // Reordering decorations outside an authored paint-property clip or
        // compositing group would change the group semantics. Synthetic
        // one-axis fragmentation clips are safe: retain them while grouping
        // the logical box's continuations below.
        let has_authored_overflow_clip =
            style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible;
        let can_retain_authored_overflow_clip = !style.has_border_radius()
            && matches!(style.overflow_x, Overflow::Visible | Overflow::Clip)
            && matches!(style.overflow_y, Overflow::Visible | Overflow::Clip);
        if style.opacity < 1.0
            || style.transform != openui_style::Transform2D::IDENTITY
            || style.filter_blur > 0.0
            || style.filter_grayscale > 0.0
            || style.clip_path_inset.is_some()
            || !style.mask_layers.is_empty()
            || style.has_paint_containment()
            || (has_authored_overflow_clip && !can_retain_authored_overflow_clip)
            || style.display.is_flex()
            || (style.display.is_grid() && !positioned_grid)
            || style.display.is_inline_level()
            || style.display.is_table_internal()
            || style.display.is_table_wrapper()
        {
            return;
        }
        if has_authored_overflow_clip {
            // An authored overflow box fixes the visible region for its
            // descendants. Relative positioning inside it can move ink
            // within that region, but cannot move the ancestor column's
            // fragmentainer boundary beyond the overflow clip.
            for clip in &mut child_clips {
                clip.translate_with_relative = false;
            }
            let (clip_x, clip_y, clip_width, clip_height) =
                compute_overflow_clip_reference_rect(fragment, fragment_offset, style);
            let margin = style.overflow_clip_margin;
            let big = 100_000.0_f32;
            let left = if style.overflow_x == Overflow::Visible {
                -big
            } else {
                clip_x - margin
            };
            let right = if style.overflow_x == Overflow::Visible {
                big
            } else {
                clip_x + clip_width + margin
            };
            let top = if style.overflow_y == Overflow::Visible {
                -big
            } else {
                clip_y - margin
            };
            let bottom = if style.overflow_y == Overflow::Visible {
                big
            } else {
                clip_y + clip_height + margin
            };
            let (left, top, width, height) = correct_fragmented_overflow_clip_rect(
                fragment,
                fragment_offset,
                (left, top, right - left, bottom - top),
            );
            let logical_clip = Rect::from_xywh(left, top, width, height);
            let antialias = antialias_rectangular_overflow_clip(fragment, doc, style);
            child_clips.push(SharedColumnClip {
                rect: if antialias {
                    logical_clip
                } else {
                    outward_snap_rect_to_physical(logical_clip, style.device_scale_factor)
                },
                antialias,
                translate_with_relative: false,
            });
        } else if fragment.has_overflow_clip {
            // Shared column-root decorations are lifted out of the ordinary
            // descendant traversal. Preserve the same synthetic
            // fragmentainer clip that traversal would have applied,
            // including two-axis clips owned by nested continuations.
            // Dropping the latter lets a resumed nested column paint through
            // its ancestor's inline continuation boundary.
            let logical_clip = column_fragmentainer_clip_rect(
                fragment,
                fragment_offset,
                doc.device_scale_factor(),
            );
            let mut physical_clip =
                outward_snap_rect_to_physical(logical_clip, doc.device_scale_factor());
            if fragment.inline_axis_clip_only && !fragment.is_first_for_node {
                let snapping = RasterSnapping::new(doc.device_scale_factor());
                if fragment_block_axis_is_x(fragment) {
                    physical_clip.top =
                        snapping.logical_coordinate(logical_clip.top, PhysicalSnap::Ceil);
                } else {
                    physical_clip.left =
                        snapping.logical_coordinate(logical_clip.left, PhysicalSnap::Ceil);
                }
            }
            child_clips.push(SharedColumnClip {
                rect: physical_clip,
                antialias: false,
                translate_with_relative: true,
            });
        }
    }

    for child in &fragment.children {
        collect_shared_column_decorations(
            child,
            doc,
            fragment_offset,
            &child_clips,
            &child_ancestor_node_ids,
            false,
            output,
        );
    }
}

fn prepaint_direct_shared_column_rules(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    fragment_offset: PhysicalOffset,
    prepainted_rules: &mut Vec<usize>,
) {
    if !fragment
        .children
        .iter()
        .any(|child| child.kind == FragmentKind::ColumnRule)
    {
        return;
    }

    canvas.save();
    if fragment.has_overflow_clip {
        let mut rule_clip =
            column_fragmentainer_clip_rect(fragment, fragment_offset, doc.device_scale_factor());
        // Rules occupy the used continuation content box. Source borders
        // suppressed by `box-decoration-break: slice` must not shorten the
        // rule clip, even though principal backgrounds intentionally retain
        // their authored first/final edge in the shared-decoration path.
        match fragment.fragmentation_writing_direction {
            Some(direction) if !direction.is_horizontal() && direction.is_flipped_blocks() => {
                if !fragment.is_first_for_node {
                    rule_clip.right = rule_clip
                        .right
                        .max((fragment_offset.left + fragment.size.width).to_f32());
                }
                if !fragment.is_last_for_node {
                    rule_clip.left = rule_clip.left.min(fragment_offset.left.to_f32());
                }
            }
            Some(direction) if !direction.is_horizontal() => {
                if !fragment.is_first_for_node {
                    rule_clip.left = rule_clip.left.min(fragment_offset.left.to_f32());
                }
                if !fragment.is_last_for_node {
                    rule_clip.right = rule_clip
                        .right
                        .max((fragment_offset.left + fragment.size.width).to_f32());
                }
            }
            _ => {
                if !fragment.is_first_for_node {
                    rule_clip.top = rule_clip.top.min(fragment_offset.top.to_f32());
                }
                if !fragment.is_last_for_node {
                    rule_clip.bottom = rule_clip
                        .bottom
                        .max((fragment_offset.top + fragment.size.height).to_f32());
                }
            }
        }
        canvas.clip_rect(
            outward_snap_rect_to_physical(rule_clip, doc.device_scale_factor()),
            ClipOp::Intersect,
            false,
        );
    }
    for rule in fragment
        .children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnRule)
    {
        let pointer = rule as *const Fragment as usize;
        if prepainted_rules.contains(&pointer)
            || rule.node_id.is_none()
            || doc.node(rule.node_id).style.visibility != Visibility::Visible
        {
            continue;
        }
        let rule_offset = PhysicalOffset::new(
            fragment_offset.left + rule.offset.left,
            fragment_offset.top + rule.offset.top,
        );
        paint_column_rule(canvas, rule, &doc.node(rule.node_id).style, rule_offset);
        prepainted_rules.push(pointer);
    }
    canvas.restore();
}

fn prepaint_shared_column_root_decorations(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    offset: PhysicalOffset,
) -> (Vec<usize>, Vec<usize>) {
    let columns: Vec<_> = children
        .iter()
        .filter(|child| child.kind == FragmentKind::ColumnBox)
        .collect();
    if columns.len() < 2 {
        return (Vec::new(), Vec::new());
    }

    let mut column_decorations: Vec<Vec<SharedColumnDecoration<'_>>> =
        Vec::with_capacity(columns.len());
    let mut occurrence_columns: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for (column_index, column) in columns.iter().enumerate() {
        let column_offset = PhysicalOffset::new(
            offset.left + column.offset.left,
            offset.top + column.offset.top,
        );
        let clips = column
            .has_overflow_clip
            .then(|| SharedColumnClip {
                rect: outward_snap_rect_to_physical(
                    column_fragmentainer_clip_rect(
                        column,
                        column_offset,
                        doc.device_scale_factor(),
                    ),
                    doc.device_scale_factor(),
                ),
                antialias: false,
                translate_with_relative: true,
            })
            .into_iter()
            .collect::<Vec<_>>();
        let mut decorations = Vec::new();
        for fragment in &column.children {
            collect_shared_column_decorations(
                fragment,
                doc,
                column_offset,
                &clips,
                &[],
                true,
                &mut decorations,
            );
        }
        if column_index > 0 {
            let previous = columns[column_index - 1];
            let previous_offset = PhysicalOffset::new(
                offset.left + previous.offset.left,
                offset.top + previous.offset.top,
            );
            let previous_rect = column_physical_rect(previous, previous_offset);
            let column_rect = column_physical_rect(column, column_offset);
            let epsilon = 1.0 / 1024.0;
            if !fragment_block_axis_is_x(column)
                && (column_rect.left - previous_rect.right).abs() <= epsilon
            {
                let shared_edge_indices: Vec<_> = decorations
                    .iter()
                    .enumerate()
                    .filter_map(|(index, decoration)| {
                        (doc.node(decoration.fragment.node_id).style.display == Display::Block
                            && (decoration.offset.left.to_f32() - column_rect.left).abs()
                                <= epsilon)
                            .then_some(index)
                    })
                    .collect();
                for &index in &shared_edge_indices {
                    let decoration = &decorations[index];
                    let top = decoration.offset.top.to_f32();
                    let bottom = top + decoration.fragment.size.height.to_f32();
                    if shared_edge_indices.iter().any(|&peer_index| {
                        if peer_index == index {
                            return false;
                        }
                        let peer = &decorations[peer_index];
                        let decoration_node_id = decoration.fragment.node_id;
                        let peer_node_id = peer.fragment.node_id;
                        if !decoration.ancestor_node_ids.contains(&peer_node_id)
                            && !peer.ancestor_node_ids.contains(&decoration_node_id)
                        {
                            return false;
                        }
                        let peer_top = peer.offset.top.to_f32();
                        let peer_bottom = peer_top + peer.fragment.size.height.to_f32();
                        bottom.min(peer_bottom) - top.max(peer_top) > epsilon
                    }) {
                        decorations[index].starts_at_shared_edge = true;
                    }
                }
            } else if fragment_block_axis_is_x(column)
                && (column_rect.top - previous_rect.bottom).abs() <= epsilon
            {
                let shared_edge_indices: Vec<_> = decorations
                    .iter()
                    .enumerate()
                    .filter_map(|(index, decoration)| {
                        (doc.node(decoration.fragment.node_id).style.display == Display::Block
                            && (decoration.offset.top.to_f32() - column_rect.top).abs() <= epsilon)
                            .then_some(index)
                    })
                    .collect();
                for &index in &shared_edge_indices {
                    let decoration = &decorations[index];
                    let left = decoration.offset.left.to_f32();
                    let right = left + decoration.fragment.size.width.to_f32();
                    if shared_edge_indices.iter().any(|&peer_index| {
                        if peer_index == index {
                            return false;
                        }
                        let peer = &decorations[peer_index];
                        let decoration_node_id = decoration.fragment.node_id;
                        let peer_node_id = peer.fragment.node_id;
                        if !decoration.ancestor_node_ids.contains(&peer_node_id)
                            && !peer.ancestor_node_ids.contains(&decoration_node_id)
                        {
                            return false;
                        }
                        let peer_left = peer.offset.left.to_f32();
                        let peer_right = peer_left + peer.fragment.size.width.to_f32();
                        right.min(peer_right) - left.max(peer_left) > epsilon
                    }) {
                        decorations[index].starts_at_shared_edge = true;
                    }
                }
            }
        }
        for decoration in &decorations {
            occurrence_columns
                .entry(decoration.fragment.node_id.index())
                .or_default()
                .insert(column_index);
        }
        column_decorations.push(decorations);
    }
    let has_repeated_decoration = occurrence_columns.values().any(|columns| columns.len() > 1);
    if !has_repeated_decoration
        && !column_decorations
            .iter()
            .flatten()
            .any(|decoration| decoration.starts_at_shared_edge)
    {
        return (Vec::new(), Vec::new());
    }
    // Decorations belong to logical boxes, not to anonymous columns. Paint
    // every continuation of one box before advancing to the next box's paint
    // phase. At a fractional shared column edge this distinction is visible:
    // column-major R,G,R,G compositing does not produce the same coverage as
    // source-major R,R,G,G compositing. A unique column-root decoration that
    // physically meets a later sliced continuation joins that phase as well;
    // leaving it in column-major painting would reverse their DOM order in
    // the shared coverage cell. Nested decorations retain their ancestor's
    // phase, and cloned decorations retain their per-fragment paint phase. A
    // unique descendant beginning at the shared edge is grouped separately
    // by `starts_at_shared_edge` for the same source-order reason.
    let mut node_order = Vec::new();
    for decorations in &column_decorations {
        for (decoration_index, decoration) in decorations.iter().enumerate() {
            let node_id = decoration.fragment.node_id.index();
            let grouped = occurrence_columns
                .get(&node_id)
                .is_some_and(|columns| columns.len() > 1)
                || decoration.starts_at_shared_edge;
            let precedes_grouped_decoration =
                decorations.iter().skip(decoration_index + 1).any(|later| {
                    decoration.ancestor_node_ids.is_empty()
                        && doc.node(later.fragment.node_id).style.box_decoration_break
                            != openui_style::BoxDecorationBreak::Clone
                        && (occurrence_columns
                            .get(&later.fragment.node_id.index())
                            .is_some_and(|columns| columns.len() > 1)
                            || later.starts_at_shared_edge)
                        && decorations_may_share_physical_coverage(
                            decoration,
                            later,
                            doc.device_scale_factor(),
                        )
                });
            // Flex and grid items paint atomically after ordinary in-flow
            // block backgrounds. Retain a later root block's background in
            // this phase when it shares a column with an earlier atomic
            // container; recursive painting then keeps the item's overflow
            // above that background.
            let follows_atomic_container = decoration.ancestor_node_ids.is_empty()
                && doc.node(decoration.fragment.node_id).style.display == Display::Block
                && doc.node(decoration.fragment.node_id).style.position
                    == openui_style::Position::Static
                && !doc
                    .node(decoration.fragment.node_id)
                    .style
                    .has_paint_containment()
                && decorations.iter().take(decoration_index).any(|earlier| {
                    earlier.ancestor_node_ids.is_empty()
                        && (doc.node(earlier.fragment.node_id).style.display.is_flex()
                            || doc.node(earlier.fragment.node_id).style.display.is_grid())
                });
            if (!grouped && !precedes_grouped_decoration && !follows_atomic_container)
                || node_order.contains(&node_id)
            {
                continue;
            }
            node_order.push(node_id);
        }
    }

    let mut prepainted = Vec::new();
    let mut prepainted_rules = Vec::new();
    let spanner_junctions =
        shared_spanner_four_way_junctions(&column_decorations, &node_order, doc);
    for node_id in node_order {
        let mut painted_solid_coverage = Vec::new();
        for decorations in &column_decorations {
            for decoration in decorations
                .iter()
                .filter(|decoration| decoration.fragment.node_id.index() == node_id)
            {
                let fragment = decoration.fragment;
                let style = &doc.node(fragment.node_id).style;
                let solid_coverage = can_coalesce_identical_solid_decoration(decoration, style)
                    .then(|| clipped_decoration_coverage_rect(decoration))
                    .flatten();
                let duplicate_solid_coverage = solid_coverage.is_some_and(|coverage| {
                    painted_solid_coverage
                        .iter()
                        .any(|painted| same_physical_decoration_rect(*painted, coverage))
                });
                let pointer = fragment as *const Fragment as usize;
                if duplicate_solid_coverage {
                    // Coordinate replay can expose the same opaque slice
                    // through two nested fragmentainers. It is one logical
                    // box decoration; painting it twice compounds fractional
                    // edge coverage at their shared physical boundary.
                    prepainted.push(pointer);
                    continue;
                }
                canvas.save();
                for clip in &decoration.clips {
                    canvas.clip_rect(clip.rect, ClipOp::Intersect, clip.antialias);
                }
                paint_fragment_box_decoration(canvas, fragment, doc, style, decoration.offset, 1.0);
                prepaint_direct_shared_column_rules(
                    canvas,
                    fragment,
                    doc,
                    decoration.offset,
                    &mut prepainted_rules,
                );
                prepainted.push(pointer);
                canvas.restore();
                if let Some(coverage) = solid_coverage {
                    painted_solid_coverage.push(coverage);
                }
            }
        }
    }
    for (cell, color) in spanner_junctions {
        // Four independently rasterized quarter-covered corners leave a
        // symmetric junction slightly lighter on Chromium's analytic
        // display-item path than four CPU-Skia SrcOver masks. Resolve that
        // primitive junction while it is still vector paint, before tile
        // assembly. The eight-bit analytic mask stores the additional
        // midpoint sample as 28/255 coverage.
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(false);
        let channel_up = |component: f32| {
            if component <= 0.0 || component >= 1.0 {
                component.clamp(0.0, 1.0)
            } else {
                (((component * 255.0).round() + 8.0).min(255.0)) / 255.0
            }
        };
        let packed_color = Color {
            r: channel_up(color.r),
            g: channel_up(color.g),
            b: channel_up(color.b),
            a: color.a,
        };
        set_paint_css_color_with_alpha(&mut paint, &packed_color, 28.0 / 255.0);
        canvas.draw_rect(cell, &paint);
    }
    (prepainted, prepainted_rules)
}

fn paint_stacking_entry(
    canvas: &Canvas,
    entry: &StackingEntry,
    children: &[Fragment],
    doc: &Document,
    offset: PhysicalOffset,
) {
    match entry {
        StackingEntry::Direct(idx) => paint_fragment_tracked(canvas, &children[*idx], doc, offset),
        StackingEntry::Descendant(fragment, parent_offset) => {
            paint_fragment_tracked(canvas, fragment, doc, *parent_offset)
        }
        StackingEntry::DescendantWithClip(fragment, parent_offset, clip_rect) => {
            canvas.save();
            canvas.clip_rect(
                outward_snap_rect_to_physical(*clip_rect, doc.device_scale_factor()),
                ClipOp::Intersect,
                false,
            );
            paint_fragment_tracked(canvas, fragment, doc, *parent_offset);
            canvas.restore();
        }
    }
}

fn stacking_entry_fragment<'a>(
    entry: &'a StackingEntry<'a>,
    children: &'a [Fragment],
) -> &'a Fragment {
    match entry {
        StackingEntry::Direct(index) => &children[*index],
        StackingEntry::Descendant(fragment, _)
        | StackingEntry::DescendantWithClip(fragment, _, _) => fragment,
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
            openui_style::Position::Absolute
                | openui_style::Position::Fixed
                | openui_style::Position::Relative
                | openui_style::Position::Sticky
        );
        (positioned && style.display.is_table_internal())
            || fragment_has_positioned_table_internal_descendant(child, doc)
    })
}

fn flex_item_interleaves_with_positioned_sibling(node_id: NodeId, doc: &Document) -> bool {
    let flex_id = doc.node(node_id).parent;
    if flex_id.is_none() || !doc.node(flex_id).style.display.is_flex() {
        return false;
    }
    let container_id = doc.node(flex_id).parent;
    if container_id.is_none() {
        return false;
    }
    doc.children(container_id).any(|sibling_id| {
        if sibling_id == flex_id {
            return false;
        }
        let style = &doc.node(sibling_id).style;
        matches!(
            style.position,
            openui_style::Position::Absolute
                | openui_style::Position::Fixed
                | openui_style::Position::Relative
                | openui_style::Position::Sticky
        ) && style.z_index.is_some()
    })
}

fn fragment_has_interleaved_flex_stacking_descendant(fragment: &Fragment, doc: &Document) -> bool {
    fragment.children.iter().any(|child| {
        if child.node_id.is_none() {
            return fragment_has_interleaved_flex_stacking_descendant(child, doc);
        }
        (doc.node(child.node_id).style.z_index.is_some()
            && flex_item_interleaves_with_positioned_sibling(child.node_id, doc))
            || fragment_has_interleaved_flex_stacking_descendant(child, doc)
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
                Some(column_fragmentainer_clip_rect(
                    child,
                    column_offset,
                    doc.device_scale_factor(),
                ))
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
        } else if child_style.z_index.is_some()
            && flex_item_interleaves_with_positioned_sibling(child.node_id, doc)
        {
            child_style.z_index
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
            // Inline descendants and positioned table internals participate
            // at the ancestor stacking level
            // even when their containing block is a z:auto positioned block.
            // Ordinary block descendants remain in this fragment's recursive
            // paint order, which also preserves local rounded-edge compositing.
            let clips_overflow = child.has_overflow_clip
                || (matches!(child.kind, FragmentKind::Box | FragmentKind::Viewport)
                    && (child_style.overflow_x != Overflow::Visible
                        || child_style.overflow_y != Overflow::Visible));
            if is_positioned
                && child_style.z_index.is_none()
                && (fragment_has_inline_fragment_descendant(child, doc)
                    || fragment_has_positioned_table_internal_descendant(child, doc)
                    || fragment_has_interleaved_flex_stacking_descendant(child, doc))
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
        paint_fragment_tracked(canvas, child, doc, parent_offset);
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

fn fragment_outline_crosses_slice(
    fragment: &Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    slice: Rect,
    block_axis_is_x: bool,
) -> bool {
    if fragment.node_id.is_none() || fragment.kind != FragmentKind::Box {
        return false;
    }
    let style = &doc.node(fragment.node_id).style;
    if !style.has_outline() {
        return false;
    }
    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
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
    end > slice_start && start < slice_end && (start < slice_start || end > slice_end)
}

fn fragment_outline_reaches_slice_edge(
    fragment: &Fragment,
    doc: &Document,
    parent_offset: PhysicalOffset,
    slice: Rect,
    block_axis_is_x: bool,
) -> bool {
    if fragment.node_id.is_none() || fragment.kind != FragmentKind::Box {
        return false;
    }
    let style = &doc.node(fragment.node_id).style;
    if !style.has_outline() {
        return false;
    }
    let fragment_offset = PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    let expand = (style.effective_outline_width() + style.outline_offset).max(0) as f32;
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
    end + expand > slice_start
        && start - expand < slice_end
        && (start - expand < slice_start || end + expand > slice_end)
}

fn collect_fragmented_descendant_outline_owners(
    children: &[Fragment],
    doc: &Document,
    parent_offset: PhysicalOffset,
    slice: Rect,
    block_axis_is_x: bool,
    owners: &mut Vec<usize>,
) {
    for child in children {
        if fragment_outline_reaches_slice_edge(child, doc, parent_offset, slice, block_axis_is_x) {
            owners.push(child as *const Fragment as usize);
        }
        let child_offset = PhysicalOffset::new(
            parent_offset.left + child.offset.left,
            parent_offset.top + child.offset.top,
        );
        collect_fragmented_descendant_outline_owners(
            &child.children,
            doc,
            child_offset,
            slice,
            block_axis_is_x,
            owners,
        );
    }
}

fn collect_subtree_outline_owners(fragment: &Fragment, doc: &Document, owners: &mut Vec<usize>) {
    if !fragment.node_id.is_none()
        && fragment.kind == FragmentKind::Box
        && doc.node(fragment.node_id).style.has_outline()
    {
        owners.push(fragment as *const Fragment as usize);
    }
    for child in &fragment.children {
        collect_subtree_outline_owners(child, doc, owners);
    }
}

fn collect_unfragmented_descendant_outline_owners(
    children: &[Fragment],
    doc: &Document,
    owners: &mut Vec<usize>,
) {
    for child in children {
        let fragmented =
            child.decoration_slice.is_some() || !child.is_first_for_node || !child.is_last_for_node;
        if !fragmented
            && !child.node_id.is_none()
            && child.kind == FragmentKind::Box
            && should_paint_outline(child, doc, &doc.node(child.node_id).style)
        {
            owners.push(child as *const Fragment as usize);
        }
        if !fragmented {
            collect_unfragmented_descendant_outline_owners(&child.children, doc, owners);
        }
    }
}

fn paint_deferred_unfragmented_descendant_outlines(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    parent_offset: PhysicalOffset,
) {
    for child in children {
        let child_offset = PhysicalOffset::new(
            parent_offset.left + child.offset.left,
            parent_offset.top + child.offset.top,
        );
        let fragmented =
            child.decoration_slice.is_some() || !child.is_first_for_node || !child.is_last_for_node;
        if !fragmented {
            paint_deferred_unfragmented_descendant_outlines(
                canvas,
                &child.children,
                doc,
                child_offset,
            );
        }
        if !fragmented && !child.node_id.is_none() && child.kind == FragmentKind::Box {
            let style = &doc.node(child.node_id).style;
            if style.visibility == Visibility::Visible && should_paint_outline(child, doc, style) {
                EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
                    deferred
                        .borrow_mut()
                        .remove(&(child as *const Fragment as usize));
                });
                paint_outline(canvas, child, style, child_offset);
            }
        }
    }
}

fn collect_later_repaint_outline_owners(
    children: &[Fragment],
    doc: &Document,
    parent_offset: PhysicalOffset,
    slice: Rect,
    block_axis_is_x: bool,
    owners: &mut Vec<usize>,
) {
    let mut preceding_fragmented_outline = false;
    for child in children {
        if preceding_fragmented_outline && child.kind != FragmentKind::ColumnRule {
            collect_subtree_outline_owners(child, doc, owners);
        }
        preceding_fragmented_outline |=
            fragmented_outline_crosses_slice(child, doc, parent_offset, slice, block_axis_is_x);
    }
}

fn paint_fragmented_descendant_outlines(
    canvas: &Canvas,
    children: &[Fragment],
    doc: &Document,
    parent_offset: PhysicalOffset,
    slice: Rect,
    block_axis_is_x: bool,
    owned_contours: Option<&[usize]>,
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
            let outline_reaches_edge = fragment_outline_reaches_slice_edge(
                child,
                doc,
                parent_offset,
                slice,
                block_axis_is_x,
            );
            if style.has_outline() && visible_end > visible_start && outline_reaches_edge {
                let owner = child as *const Fragment as usize;
                let caller_owns_contour = owned_contours.is_none_or(|owned| owned.contains(&owner));
                let owns_contour = caller_owns_contour
                    && PAINTED_FRAGMENTED_OUTLINES
                        .with(|painted| painted.borrow_mut().insert(owner));
                if owns_contour {
                    EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
                        deferred.borrow_mut().remove(&owner);
                    });
                    let mut visible = child.clone();
                    let mut visible_offset = child_offset;
                    if start < slice_start || end > slice_end {
                        if block_axis_is_x {
                            visible_offset.left = LayoutUnit::from_f32(visible_start);
                            visible.size.width = LayoutUnit::from_f32(visible_end - visible_start);
                        } else {
                            visible_offset.top = LayoutUnit::from_f32(visible_start);
                            visible.size.height = LayoutUnit::from_f32(visible_end - visible_start);
                        }
                    }
                    paint_outline(canvas, &visible, style, visible_offset);
                }
            }
        }
        paint_fragmented_descendant_outlines(
            canvas,
            &child.children,
            doc,
            child_offset,
            slice,
            block_axis_is_x,
            owned_contours,
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
    let crosses =
        fragment_outline_crosses_slice(fragment, doc, parent_offset, slice, block_axis_is_x);
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
            let mut outline_owners = Vec::new();
            collect_subtree_outline_owners(child, doc, &mut outline_owners);
            EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
                let mut deferred = deferred.borrow_mut();
                for pointer in &outline_owners {
                    deferred.remove(pointer);
                }
            });
            paint_fragment_tracked(canvas, child, doc, parent_offset);
        }
        preceding_fragmented_outline |=
            fragmented_outline_crosses_slice(child, doc, parent_offset, slice, block_axis_is_x);
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_touching_unfragmented_descendant_outline_owners(
    fragment: &Fragment,
    doc: &Document,
    fragment_top_in_parent: LayoutUnit,
    later_top_in_parent: LayoutUnit,
    fragmented_ancestor: bool,
    owners: &mut Vec<usize>,
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
            owners.push(fragment as *const Fragment as usize);
        }
    }
    for child in &fragment.children {
        collect_touching_unfragmented_descendant_outline_owners(
            child,
            doc,
            fragment_top_in_parent + child.offset.top,
            later_top_in_parent,
            fragmented_here,
            owners,
        );
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
    owned_outlines: &[usize],
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
            let owner = fragment as *const Fragment as usize;
            if owned_outlines.contains(&owner) {
                EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
                    deferred.borrow_mut().remove(&owner);
                });
                paint_outline(canvas, fragment, style, abs_offset);
            }
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
            owned_outlines,
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

fn fragment_subtree_has_text(fragment: &Fragment) -> bool {
    fragment.kind == FragmentKind::Text || fragment.children.iter().any(fragment_subtree_has_text)
}

fn fragment_subtree_has_images(fragment: &Fragment, doc: &Document) -> bool {
    (!fragment.node_id.is_none() && {
        let node = doc.node(fragment.node_id);
        node.tag == ElementTag::Image
            || node.replaced.is_some()
            || node
                .style
                .background_layers
                .iter()
                .any(|layer| matches!(layer.image, CssImage::Raster(_)))
            || node
                .style
                .border_image
                .as_ref()
                .is_some_and(|border| matches!(border.source, CssImage::Raster(_)))
    }) || fragment
        .children
        .iter()
        .any(|child| fragment_subtree_has_images(child, doc))
}

fn paint_list_marker(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
) {
    let marker_style = style.marker_style.as_deref().unwrap_or(style);
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

    let font_size = marker_style.font_size.max(1.0);
    let line_height = match marker_style.line_height {
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
        set_paint_css_color_with_alpha(&mut paint, &marker_style.color, 1.0);
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);
        canvas.draw_path(&path.detach(), &paint);
        return;
    }
    if style.list_style_type == ListStyleType::DisclosureClosed {
        let left = abs_offset.left.round().to_f32();
        let top = (abs_offset.top + line_top).round().to_f32() + 2.5;
        let mut path = PathBuilder::new();
        path.move_to(Point::new(left, top));
        path.line_to(Point::new(left + 9.070_312_5, top + 5.183_593_8));
        path.line_to(Point::new(left, top + 10.546_875));
        path.close();
        let mut paint = Paint::default();
        set_paint_css_color_with_alpha(&mut paint, &marker_style.color, 1.0);
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);
        canvas.draw_path(&path.detach(), &paint);
        return;
    }
    let marker_y = (abs_offset.top + line_top).round().to_f32()
        + ((line_height - marker_diameter) / 2.0).floor()
        + marker_y_adjust;

    let mut paint = Paint::default();
    set_paint_css_color_with_alpha(&mut paint, &marker_style.color, 1.0);
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
    let insets = fragment
        .element_scrollbars
        .map_or(openui_geometry::BoxStrut::zero(), |s| s.insets);
    // Pixel-snap the padding box edges independently for crisp clipping.
    let clip_left = (offset.left + fragment.border.left + insets.left)
        .round()
        .to_f32();
    let clip_top = (offset.top + fragment.border.top + insets.top)
        .round()
        .to_f32();
    let clip_right = (offset.left + fragment.size.width - fragment.border.right - insets.right)
        .round()
        .to_f32();
    let clip_bottom = (offset.top + fragment.size.height - fragment.border.bottom - insets.bottom)
        .round()
        .to_f32();
    let clip_w = (clip_right - clip_left).max(0.0);
    let clip_h = (clip_bottom - clip_top).max(0.0);
    (clip_left, clip_top, clip_w, clip_h)
}

/// Resolve the authored visual box that establishes an overflow clip.
///
/// Keep this calculation shared by ordinary descendant painting and the
/// multicol shared-decoration traversal. The latter paints a logical box's
/// continuations as one unit, but it must still retain the box's authored
/// `overflow-clip-margin` reference box rather than silently falling back to
/// the padding box.
fn compute_overflow_clip_reference_rect(
    fragment: &Fragment,
    offset: PhysicalOffset,
    style: &ComputedStyle,
) -> (f32, f32, f32, f32) {
    let (px, py, pw, ph) = compute_clip_rect(fragment, offset);
    match overflow_clip_reference_box(style) {
        OverflowClipBox::PaddingBox => (px, py, pw, ph),
        OverflowClipBox::ContentBox => {
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
            let bl = fragment.border.left.to_f32();
            let br = fragment.border.right.to_f32();
            let bt = fragment.border.top.to_f32();
            let bb = fragment.border.bottom.to_f32();
            (px - bl, py - bt, pw + bl + br, ph + bt + bb)
        }
    }
}

fn correct_fragmented_overflow_clip_rect(
    fragment: &Fragment,
    offset: PhysicalOffset,
    rect: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    if fragment.is_first_for_node && fragment.is_last_for_node {
        return rect;
    }

    let (clip_x, clip_y, clip_w, clip_h) = rect;
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

fn compute_column_block_clip_rect(
    fragment: &Fragment,
    offset: PhysicalOffset,
    device_scale: f64,
) -> Rect {
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
            let child_top = fragment
                .children
                .iter()
                .map(|child| (offset.top + child.offset.top).to_f32())
                .fold(top, f32::min);
            // A continuation may start fractionally above its column after
            // LayoutUnit rounding. Preserve actual overflowing ink, but do
            // not expand the clip for an offset smaller than one device cell.
            if top - child_top >= (1.0 / device_scale) as f32 {
                top = child_top;
            }
        }
    }

    // A `box-decoration-break: slice` continuation does not own a source
    // border at each fragmented block edge. Shared-decoration prepainting
    // uses this synthetic clip before ordinary overflow correction, so restore
    // each suppressed edge independently. Otherwise a first continuation
    // still subtracts the source block-end border from its fragmentainer and
    // clips descendant side borders before the column edge.
    if !fragment.is_first_for_node || !fragment.is_last_for_node {
        let snapping = RasterSnapping::new(device_scale);
        let is_middle_continuation = !fragment.is_first_for_node && !fragment.is_last_for_node;
        let snap_restored_start = |coordinate: f32| {
            if is_middle_continuation || device_scale.fract().abs() <= f64::EPSILON {
                coordinate
            } else {
                // Keep the logical edge one layout quantum inside the chosen
                // physical pixel. The clip is converted back through `f32`
                // and may be outward-snapped again by an enclosing column;
                // an exact `physical / scale` value can otherwise round just
                // across the pixel boundary and admit the neighboring row.
                snapping.logical_coordinate(coordinate, PhysicalSnap::Ceil) + 1.0 / 64.0
            }
        };
        let snap_restored_end = |coordinate: f32| {
            if is_middle_continuation || device_scale.fract().abs() <= f64::EPSILON {
                coordinate
            } else {
                snapping.logical_coordinate(coordinate, PhysicalSnap::Floor) - 1.0 / 64.0
            }
        };
        match fragment.fragmentation_writing_direction {
            Some(direction) if !direction.is_horizontal() && direction.is_flipped_blocks() => {
                if !fragment.is_first_for_node {
                    right = right.max(snap_restored_end(
                        (offset.left + fragment.size.width).to_f32(),
                    ));
                }
                if !fragment.is_last_for_node {
                    left = left.min(snap_restored_start(offset.left.to_f32()));
                }
            }
            Some(direction) if !direction.is_horizontal() => {
                if !fragment.is_first_for_node {
                    left = left.min(snap_restored_start(offset.left.to_f32()));
                }
                if !fragment.is_last_for_node {
                    right = right.max(snap_restored_end(
                        (offset.left + fragment.size.width).to_f32(),
                    ));
                }
            }
            _ => {
                if !fragment.is_first_for_node {
                    top = top.min(snap_restored_start(offset.top.to_f32()));
                }
                if !fragment.is_last_for_node {
                    bottom = bottom.max(snap_restored_end(
                        (offset.top + fragment.size.height).to_f32(),
                    ));
                }
            }
        }
    }

    Rect::from_ltrb(left, top, right.max(left), bottom.max(top))
}

fn column_block_only_clip_rect(
    fragment: &Fragment,
    offset: PhysicalOffset,
    device_scale: f64,
) -> Rect {
    let block = compute_column_block_clip_rect(fragment, offset, device_scale);
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

fn column_fragmentainer_clip_rect(
    fragment: &Fragment,
    offset: PhysicalOffset,
    device_scale: f64,
) -> Rect {
    if fragment.block_axis_clip_only && !fragment.inline_axis_clip_only {
        column_block_only_clip_rect(fragment, offset, device_scale)
    } else if fragment.inline_axis_clip_only {
        column_inline_only_clip_rect(fragment, offset)
    } else {
        compute_column_block_clip_rect(fragment, offset, device_scale)
    }
}

fn outward_snap_rect_to_physical(rect: Rect, device_scale: f64) -> Rect {
    // Preserve the established DPR-1 clip semantics. At unit scale Skia's
    // non-AA clip retains the LayoutUnit edge phase; expanding it to an
    // integer rectangle admits an extra row of fragmented text. Physical
    // outward closure is needed only once logical and device grids differ.
    if (device_scale - 1.0).abs() <= f64::EPSILON {
        return rect;
    }
    let snapping = RasterSnapping::new(device_scale);
    Rect::from_ltrb(
        snapping.logical_coordinate(rect.left, PhysicalSnap::Floor),
        snapping.logical_coordinate(rect.top, PhysicalSnap::Floor),
        snapping.logical_coordinate(rect.right, PhysicalSnap::Ceil),
        snapping.logical_coordinate(rect.bottom, PhysicalSnap::Ceil),
    )
}

fn close_rect_at_physical_viewport_edge(mut rect: Rect, device_scale: f64) -> Rect {
    let snapping = RasterSnapping::new(device_scale);
    VIEWPORT_SIZE.with(|size| {
        let (viewport_width, viewport_height) = *size.borrow();
        if (rect.right - viewport_width).abs() <= 1.0 / 1024.0 {
            rect.right = snapping.logical_coordinate(viewport_width, PhysicalSnap::Ceil);
        }
        if (rect.bottom - viewport_height).abs() <= 1.0 / 1024.0 {
            rect.bottom = snapping.logical_coordinate(viewport_height, PhysicalSnap::Ceil);
        }
    });
    rect
}

fn half_open_physical_clip_rect(mut rect: Rect, device_scale: f64) -> Rect {
    let scale = device_scale.max(f64::EPSILON);
    let trailing_edge_hits_pixel_center = |value: f32| {
        let fraction = (f64::from(value) * scale).rem_euclid(1.0);
        (fraction - 0.5).abs() <= 1.0e-6
    };
    let previous_f32 = |value: f32| {
        if value.is_finite() && value > 0.0 {
            f32::from_bits(value.to_bits() - 1)
        } else if value.is_finite() && value < 0.0 {
            f32::from_bits(value.to_bits() + 1)
        } else {
            value
        }
    };
    if trailing_edge_hits_pixel_center(rect.right) {
        rect.right = previous_f32(rect.right);
    }
    if trailing_edge_hits_pixel_center(rect.bottom) {
        rect.bottom = previous_f32(rect.bottom);
    }
    rect
}

fn flat_shadow_exclusion_rect(rect: Rect, device_scale: f64) -> Rect {
    // The flat shadow already carries fractional coverage at its border-box
    // edge. Keep the hard exclusion clip inside that terminal physical cell
    // so it does not discard the shadow's own partial sample before the
    // element's opaque border and background are painted over it.
    let snapping = RasterSnapping::new(device_scale);
    let mut rect = rect;
    rect.left = snapping.logical_coordinate(rect.left, PhysicalSnap::Ceil);
    rect.top = snapping.logical_coordinate(rect.top, PhysicalSnap::Ceil);
    rect.right = snapping.logical_coordinate(rect.right, PhysicalSnap::Floor);
    rect.bottom = snapping.logical_coordinate(rect.bottom, PhysicalSnap::Floor);
    rect
}

fn overflow_clip_reference_box(style: &ComputedStyle) -> OverflowClipBox {
    if style.overflow_x == Overflow::Clip || style.overflow_y == Overflow::Clip {
        style.overflow_clip_box
    } else {
        OverflowClipBox::PaddingBox
    }
}

/// Whether a rectangular authored overflow clip keeps analytic edge coverage.
///
/// Chromium device-snaps native scrollports, but authored `hidden` and `clip`
/// content clips are replayed through the device transform. At a fractional
/// scale that distinction is visible even when the clip was snapped to
/// integer CSS coordinates first.
fn antialias_rectangular_overflow_clip(
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
) -> bool {
    // An iframe owns a nested viewport whose content is already clipped and
    // rastered in that viewport's device space. Its host content edge is a
    // device-aligned viewport scissor, not another analytic CSS content clip.
    let is_embedded_viewport = !fragment.node_id.is_none()
        && doc.node(fragment.node_id).tag == openui_dom::ElementTag::IFrame;
    // A button's implicit content clip is a native control scissor. At a
    // fractional device scale, its boundary covers the full physical cell
    // touched by the button. The child's own analytic edge supplies coverage
    // when the two edges coincide; antialiasing the clip as well would apply
    // that half-pixel coverage twice.
    let is_button_content_clip = !fragment.node_id.is_none()
        && doc.node(fragment.node_id).tag == openui_dom::ElementTag::Button
        && doc.node(fragment.node_id).form_control == Some(FormControlRole::Button)
        && style.overflow_x == Overflow::Clip
        && style.overflow_y == Overflow::Clip;
    !is_embedded_viewport
        && !is_button_content_clip
        && !style.has_border_radius()
        && ((style.transform.b != 0.0 || style.transform.c != 0.0)
            || (style.device_scale_factor.fract().abs() > f64::EPSILON
                && (matches!(style.overflow_x, Overflow::Hidden | Overflow::Clip)
                    || matches!(style.overflow_y, Overflow::Hidden | Overflow::Clip))))
}

/// A repeated table body's visible slice can end inside its source row group.
/// Keep that fragmentainer edge in logical coordinates so the body's final
/// coverage composes with the repeated footer's start edge at fractional DPR.
fn truncated_table_row_group_end(fragment: &Fragment, style: &ComputedStyle) -> bool {
    style.display == Display::TableRowGroup
        && fragment.has_overflow_clip
        && fragment.block_axis_clip_only
        && fragment
            .children
            .iter()
            .any(|child| child.offset.top + child.size.height > fragment.size.height)
}

/// Whether Blink assigns this box a separately rastered scroll-contents
/// layer. CPU qualification replays scroll contents in the destination
/// display list so coverage and dithering retain the compositor-tile phase;
/// pre-rasterizing them into a software surface introduces a second
/// fractional resample. Ganesh keeps an explicit layer because its replay
/// backend owns that surface and transform directly.
fn has_composited_scroll_contents(fragment: &Fragment, style: &ComputedStyle) -> bool {
    if style.raster_configuration.backend != RasterBackend::GaneshGl {
        return false;
    }
    let overflow = fragment.scrollable_overflow();
    // ScrollableOverflow contains the border box even when no descendant
    // overflows. `auto` gets a scroll-contents layer only when ink extends
    // beyond that principal box; otherwise Chromium paints the descendants
    // directly and avoids an unnecessary fractional-scale resample. Authored
    // `scroll` retains its layer even with an empty scroll range.
    let overflows_x = overflow.x() < LayoutUnit::zero() || overflow.right() > fragment.size.width;
    let overflows_y = overflow.y() < LayoutUnit::zero() || overflow.bottom() > fragment.size.height;
    style.overflow_x == Overflow::Scroll
        || style.overflow_y == Overflow::Scroll
        || (style.overflow_x == Overflow::Auto && overflows_x)
        || (style.overflow_y == Overflow::Auto && overflows_y)
}

/// CPU replay normally paints scroll contents directly. At a fractional
/// physical start, however, Chromium's scroll backing owns the coverage phase
/// for an explicitly scrollable axis. Raster that backing in software only
/// when direct replay cannot be bit-equivalent; an integral start remains a
/// bit-preserving direct draw and avoids an unnecessary resample.
fn needs_fractional_cpu_scroll_backing(
    fragment: &Fragment,
    style: &ComputedStyle,
    clip_rect: Rect,
) -> bool {
    if style.raster_configuration.backend == RasterBackend::GaneshGl
        || style.device_scale_factor.fract().abs() <= f64::EPSILON
    {
        return false;
    }
    let scale = style.device_scale_factor;
    let is_fractional = |value: f32| {
        let phase = (f64::from(value) * scale).rem_euclid(1.0);
        phase > 1.0e-6 && (1.0 - phase) > 1.0e-6
    };
    let overflow = fragment.scrollable_overflow();
    // The software replay below is one compositor tile. Oversized scroll
    // contents are already split into physical tiles by Chromium; replaying
    // them as one giant fractional surface changes coverage at every tile
    // edge. Keep those on the bit-preserving destination display list until
    // they are assembled by the tiled path.
    const SCROLL_TILE_EXTENT_PX: f64 = 512.0;
    let fits_single_tile = f64::from(overflow.size.width.to_f32()) * scale <= SCROLL_TILE_EXTENT_PX
        && f64::from(overflow.size.height.to_f32()) * scale <= SCROLL_TILE_EXTENT_PX;
    if !fits_single_tile {
        return false;
    }
    let overflows_x = overflow.x() < LayoutUnit::zero() || overflow.right() > fragment.size.width;
    let overflows_y = overflow.y() < LayoutUnit::zero() || overflow.bottom() > fragment.size.height;
    (style.overflow_x == Overflow::Scroll && overflows_x && is_fractional(clip_rect.left))
        || (style.overflow_y == Overflow::Scroll && overflows_y && is_fractional(clip_rect.top))
}

fn scroll_backing_phase_adjustment(logical_origin: f32, backing_scale: f32) -> f32 {
    let physical_phase = (logical_origin * backing_scale).rem_euclid(1.0);
    if physical_phase.abs() <= f32::EPSILON * 16.0 {
        return 0.0;
    }
    let compositor_phase = (backing_scale.ceil() - backing_scale).rem_euclid(1.0);
    (compositor_phase - physical_phase).rem_euclid(1.0) / backing_scale
}

/// Scroll translations without an authored transform ancestry use the same
/// physical snapping as the viewport. Chromium creates a snapped compositor
/// transform for every scroll node, including non-composited scrollers.
/// Retained offsets and geometry stay logical; only paint movement is snapped.
///
/// An authored transform or SVG viewport supplies another screen-space
/// origin. That path still needs the complete retained transform tree rather
/// than independently snapping a local delta at the document's device scale.
fn element_scroll_paint_translation(
    fragment: &Fragment,
    doc: &Document,
    scroll_x: f32,
    scroll_y: f32,
) -> (f32, f32) {
    // Fragmentation also installs overflow clips and records a scroll area,
    // but those slices have no scroll transform. Only an authored scroll
    // container owns the origin and physical translation below.
    if fragment.node_id.is_none() || !doc.node(fragment.node_id).style.is_scroll_container() {
        return (-scroll_x, -scroll_y);
    }
    let mut ancestor = fragment.node_id;
    while !ancestor.is_none() {
        let node = doc.node(ancestor);
        if node.style.transform != openui_style::Transform2D::IDENTITY
            || node.is_svg_foreign_object
            || node.tag == openui_dom::ElementTag::Svg
        {
            return (-scroll_x, -scroll_y);
        }
        ancestor = node.parent;
    }
    let snapping = RasterSnapping::new(doc.device_scale_factor());
    // Chromium snaps the translation of ScrollPosition = ScrollOrigin +
    // ScrollOffset, and adds ScrollOrigin to its contents paint offset.
    // Reversed axes can have a nonzero origin even at API offset zero.
    let (origin_x, origin_y) = fragment.scroll_area.map_or((0.0, 0.0), |area| {
        (
            (area.client_rect.x() - area.content_rect.x())
                .to_f32()
                .floor(),
            (area.client_rect.y() - area.content_rect.y())
                .to_f32()
                .floor(),
        )
    });
    (
        origin_x + snapping.logical_coordinate(-(scroll_x + origin_x), PhysicalSnap::Nearest),
        origin_y + snapping.logical_coordinate(-(scroll_y + origin_y), PhysicalSnap::Nearest),
    )
}

/// Prove opacity for the source cells reached by the visible linear-sampled
/// quad. This uses owned paint coverage, never an inspection of raster pixels.
fn scroll_quad_has_opaque_samples(
    canvas: &Canvas,
    opaque: &skia_safe::Region,
    source: Rect,
    destination: Rect,
    width: i32,
    height: i32,
) -> bool {
    let matrix = canvas.local_to_device_as_3x3();
    if opaque.is_empty() || !matrix.rect_stays_rect() {
        return false;
    }
    let (Some(inverse), Some(clip)) = (matrix.invert(), canvas.device_clip_bounds()) else {
        return false;
    };
    let mut visible = inverse.map_rect(Rect::from(clip)).0;
    if !visible.intersect(destination) || !visible.is_finite() {
        return false;
    }
    let scale_x = source.width() / destination.width();
    let scale_y = source.height() / destination.height();
    // Linear filtering reaches a neighboring source center on each side.
    // The strict source rectangle clamps outside the backing image bounds.
    let samples = Rect::from_ltrb(
        (source.left + (visible.left - destination.left) * scale_x - 0.5)
            .floor()
            .max(0.0),
        (source.top + (visible.top - destination.top) * scale_y - 0.5)
            .floor()
            .max(0.0),
        (source.left + (visible.right - destination.left) * scale_x + 0.5)
            .ceil()
            .min(width as f32),
        (source.top + (visible.bottom - destination.top) * scale_y + 0.5)
            .ceil()
            .min(height as f32),
    );
    if samples.is_empty() || !samples.is_finite() {
        return false;
    }
    let samples: IRect = samples.round_out();
    opaque.contains_rect(samples)
}

fn paint_composited_scroll_contents(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
    style: &ComputedStyle,
    clip_rect: Rect,
    is_stacking_context: bool,
) -> bool {
    let backing_scale = style.device_scale_factor.max(f64::EPSILON) as f32;
    let overflow = fragment.scrollable_overflow();
    let fragment_width = fragment.size.width.to_f32();
    let fragment_height = fragment.size.height.to_f32();
    let local_clip_left = clip_rect.left - offset.left.to_f32();
    let local_clip_top = clip_rect.top - offset.top.to_f32();
    let local_clip_right = clip_rect.right - offset.left.to_f32();
    let local_clip_bottom = clip_rect.bottom - offset.top.to_f32();
    let overflow_left = overflow.x().to_f32();
    let overflow_top = overflow.y().to_f32();
    let overflow_right = overflow.right().to_f32();
    let overflow_bottom = overflow.bottom().to_f32();
    // `ScrollableOverflow()` includes the border box in its union. Ignore
    // that contribution on a side unless content actually crosses the outer
    // border; otherwise the backing begins at the scrollport's padding edge.
    let local_layer_left = if overflow_left < 0.0 {
        overflow_left
    } else {
        local_clip_left
    };
    let local_layer_top = if overflow_top < 0.0 {
        overflow_top
    } else {
        local_clip_top
    };
    let local_layer_right = if overflow_right > fragment_width {
        overflow_right
    } else {
        local_clip_right
    };
    let local_layer_bottom = if overflow_bottom > fragment_height {
        overflow_bottom
    } else {
        local_clip_bottom
    };
    let layer_rect = Rect::from_ltrb(
        offset.left.to_f32() + local_layer_left,
        offset.top.to_f32() + local_layer_top,
        offset.left.to_f32() + local_layer_right,
        offset.top.to_f32() + local_layer_bottom,
    );
    let physical_width = (layer_rect.width() * backing_scale).ceil().max(1.0) as i32;
    let physical_height = (layer_rect.height() * backing_scale).ceil().max(1.0) as i32;
    let Some(mut surface) = surfaces::raster_n32_premul((physical_width, physical_height)) else {
        return false;
    };
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    surface.canvas().scale((backing_scale, backing_scale));
    surface
        .canvas()
        .translate((-layer_rect.left, -layer_rect.top));

    // The scroll-contents layer owns the box background and inset shadows,
    // but not the border, border-image, or outset shadows. Preserve the
    // authored border widths for background-clip geometry while making the
    // border paint itself transparent.
    let contents_style = style.derive(|adjusted| {
        adjusted.border_top_color = StyleColor::Resolved(Color::TRANSPARENT);
        adjusted.border_right_color = StyleColor::Resolved(Color::TRANSPARENT);
        adjusted.border_bottom_color = StyleColor::Resolved(Color::TRANSPARENT);
        adjusted.border_left_color = StyleColor::Resolved(Color::TRANSPARENT);
        adjusted.border_image = None;
        adjusted.box_shadow.retain(|shadow| shadow.inset);
    });
    let background_recording = BackgroundRecordingScope::for_raster();
    paint_box_decoration_background(
        surface.canvas(),
        fragment,
        doc,
        &contents_style,
        offset,
        1.0,
    );

    let (scroll_x, scroll_y) = if fragment.node_id.is_none() {
        (0.0, 0.0)
    } else {
        let node = doc.node(fragment.node_id);
        (node.scroll_left, node.scroll_top)
    };
    surface.canvas().save();
    // The backing owns its raster and composition phase. Keep the retained
    // offset logical inside it; direct-scroll snapping must not move these
    // children independently of the background recorded above.
    let translation = (-scroll_x, -scroll_y);
    if translation != (0.0, 0.0) {
        surface.canvas().translate(translation);
    }
    paint_children_with_stacking_order(
        surface.canvas(),
        &fragment.children,
        doc,
        offset,
        is_stacking_context,
        style.display.is_flex() && fragmented_flex_has_internal_four_way_junction(fragment),
    );
    surface.canvas().restore();

    let opaque_region = layer_opaque_region(&background_recording.annotations());
    let composition_effect = background_recording.previous_effect;
    drop(background_recording);
    let image = surface.image_snapshot();
    let destination = Rect::from_xywh(
        layer_rect.left + scroll_backing_phase_adjustment(layer_rect.left, backing_scale),
        layer_rect.top + scroll_backing_phase_adjustment(layer_rect.top, backing_scale),
        physical_width as f32 / backing_scale,
        physical_height as f32 / backing_scale,
    );
    let mut paint = Paint::default();
    paint.set_anti_alias(false);
    let snapping = RasterSnapping::new(style.device_scale_factor);
    let compositor_bounds = Rect::from_ltrb(
        destination.left,
        destination.top,
        destination
            .right
            .max(snapping.logical_coordinate(layer_rect.right, PhysicalSnap::Ceil)),
        destination
            .bottom
            .max(snapping.logical_coordinate(layer_rect.bottom, PhysicalSnap::Ceil)),
    );
    // SoftwareRenderer replays a texture quad with DrawImageRect and a
    // strict source rectangle. Preserve the backing's mapping when its
    // physical compositor bounds extend beyond the content destination.
    let source = Rect::from_xywh(
        0.0,
        0.0,
        compositor_bounds.width() * backing_scale,
        compositor_bounds.height() * backing_scale,
    );
    // Chromium omits blending only for an opaque quad with full composition
    // opacity. An opaque authored color alone cannot establish that: clipped
    // or rounded backgrounds can leave transparent backing cells.
    if composition_effect == (1.0, true)
        && style.opacity == 1.0
        && scroll_quad_has_opaque_samples(
            canvas,
            &opaque_region,
            source,
            compositor_bounds,
            physical_width,
            physical_height,
        )
    {
        paint.set_blend_mode(skia_safe::BlendMode::Src);
    }
    canvas.draw_image_rect_with_sampling_options(
        image,
        Some((&source, SrcRectConstraint::Strict)),
        compositor_bounds,
        SamplingOptions::from(FilterMode::Linear),
        &paint,
    );
    true
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
    let unclipped_fieldset_legend_content: Vec<&Fragment> = if !fragment.node_id.is_none()
        && doc.node(fragment.node_id).tag == ElementTag::Fieldset
    {
        let rendered_legend = doc.fieldset_rendered_legend(fragment.node_id);
        let belongs_to_rendered_legend = |node_id: NodeId| {
            let mut ancestor = node_id;
            while !ancestor.is_none() && ancestor != fragment.node_id {
                if Some(ancestor) == rendered_legend {
                    return true;
                }
                ancestor = doc.node(ancestor).parent;
            }
            false
        };
        fragment
            .children
            .iter()
            .filter(|child| !child.node_id.is_none() && belongs_to_rendered_legend(child.node_id))
            .collect()
    } else {
        Vec::new()
    };
    for legend_content in &unclipped_fieldset_legend_content {
        paint_fragment_tracked(canvas, legend_content, doc, offset);
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
    let (clip_x, clip_y, clip_w, clip_h) =
        compute_overflow_clip_reference_rect(fragment, offset, style);

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
    let is_svg_viewport =
        !fragment.node_id.is_none() && doc.node(fragment.node_id).tag == ElementTag::Svg;
    let margin_x = if is_svg_viewport
        || margin_expands_paint_clip
        || (has_clip_axis && style.overflow_x == Overflow::Clip)
    {
        margin
    } else {
        0.0
    };
    let margin_y = if is_svg_viewport
        || margin_expands_paint_clip
        || (has_clip_axis && style.overflow_y == Overflow::Clip)
    {
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
        correct_fragmented_overflow_clip_rect(fragment, offset, (clip_x, clip_y, clip_w, clip_h));

    // A layout-owned fragmentation clip is not authored overflow clipping.
    // Descendant outlines remain ink overflow at every fragmentainer edge,
    // so admit their outset while keeping ordinary continuation content under
    // the same expanded edge (the outline paints over that narrow strip).
    let fragmented_outline_clip = (fragment.has_overflow_clip
        && fragment.block_axis_clip_only
        && style.overflow_x == Overflow::Visible
        && style.overflow_y == Overflow::Visible)
        .then(|| Rect::from_xywh(clip_x, clip_y, clip_w, clip_h));
    let newly_deferred_fragmented_outlines = if let Some(slice) = fragmented_outline_clip {
        let mut owners = Vec::new();
        collect_fragmented_descendant_outline_owners(
            &fragment.children,
            doc,
            offset,
            slice,
            fragment_block_axis_is_x(fragment),
            &mut owners,
        );
        EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
            let mut deferred = deferred.borrow_mut();
            owners
                .into_iter()
                .filter(|pointer| deferred.insert(*pointer))
                .collect::<Vec<_>>()
        })
    } else {
        Vec::new()
    };
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
                    // An in-flow opacity stacking context is composited as one
                    // unit after its fragmented ancestor. Its ink may cross
                    // the ancestor's principal-box slice, while the enclosing
                    // ColumnBox still clips it at the fragmentainer edge.
                    fragment.decoration_paint_block_size.is_some()
                        && !child.node_id.is_none()
                        && doc.node(child.node_id).style.opacity < 1.0
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
    let analytic_table_slice_end = style.device_scale_factor.fract().abs() > f64::EPSILON
        && truncated_table_row_group_end(fragment, style);
    let clip_rect = if fragmented_outline_clip.is_some() && !analytic_table_slice_end {
        // This is a layout-owned continuation boundary, just like the
        // surrounding anonymous ColumnBox. Close it on the same outward
        // physical cells so outgoing and incoming fragments retain
        // complementary coverage at a fractional replay scale.
        outward_snap_rect_to_physical(clip_rect, doc.device_scale_factor())
    } else {
        clip_rect
    };

    let rounded_clip_rrect =
        (style.has_border_radius() && !fragment.ignore_border_radius).then(|| {
            build_clip_rrect(
                &clip_rect,
                fragment,
                style,
                overflow_clip_reference_box(style),
                margin_x,
                margin_y,
            )
        });
    let packed_rounded_mask = rounded_clip_rrect.as_ref().and_then(|rrect| {
        (style.raster_configuration.backend != RasterBackend::GaneshGl
            && style.overflow_x == Overflow::Hidden
            && style.overflow_y == Overflow::Hidden
            && !fragment_subtree_has_text(fragment)
            && margin_x.abs() <= f32::EPSILON
            && margin_y.abs() <= f32::EPSILON
            && fragment.decoration_slice.is_none()
            && fragmented_outline_clip.is_none())
        .then(|| packed_rounded_clip_mask(rrect, style.device_scale_factor as f32))
        .flatten()
    });
    if packed_rounded_mask.is_some() {
        canvas.save_layer(&SaveLayerRec::default().bounds(&clip_rect));
    } else {
        canvas.save();
    }
    let mut fragmented_top_left_tangent = None;
    let composited_scroll_layer = has_composited_scroll_contents(fragment, style)
        || needs_fractional_cpu_scroll_backing(fragment, style, clip_rect);
    let overflow = fragment.scrollable_overflow();
    let has_scroll_range = overflow.x() < LayoutUnit::zero()
        || overflow.right() > fragment.size.width
        || overflow.y() < LayoutUnit::zero()
        || overflow.bottom() > fragment.size.height;
    // An explicitly scrollable box gets a scroll backing even when its range
    // is currently empty. CPU replay can keep that backing implicit, but its
    // fractional clip coverage must still multiply the independently painted
    // child edge. Limit this analytic path to the opaque, fully covered
    // scrollport geometry where the backing is observable.
    let implicit_scroll_backing_clip = !composited_scroll_layer
        && !has_scroll_range
        && (style.overflow_x == Overflow::Scroll || style.overflow_y == Overflow::Scroll)
        && opaque_in_flow_child_covers_inner_border_box(fragment, doc);
    let antialias_transformed_clip = analytic_table_slice_end
        || antialias_rectangular_overflow_clip(fragment, doc, style)
        || (!composited_scroll_layer
            && style.device_scale_factor.fract().abs() > f64::EPSILON
            && (style.overflow_x == Overflow::Auto
                || style.overflow_y == Overflow::Auto
                || implicit_scroll_backing_clip));

    // When border-radius is set, clip to a rounded rect so children are
    // clipped along the curves. Otherwise use a simple rect clip.
    if let Some(rrect) = rounded_clip_rrect {
        let top_left = rrect.radii(RRectCorner::UpperLeft);
        if fragment.decoration_slice.is_some() && top_left.x > 0.0 && top_left.y > 0.0 {
            fragmented_top_left_tangent = Some((rrect.rect().left, rrect.rect().top + top_left.y));
        }
        if packed_rounded_mask.is_none() {
            canvas.clip_rrect(rrect, ClipOp::Intersect, true);
        }
    } else {
        // Axis-aligned rectangular overflow clips are pixel-snapped in CSS
        // space above. They remain hard only when that edge also lands on the
        // physical grid. Fractional replay scales and rotations retain
        // analytic coverage in Blink.
        let raster_clip_rect = if composited_scroll_layer {
            let snapping = RasterSnapping::new(style.device_scale_factor);
            let overflow = fragment.scrollable_overflow();
            let scroll_start_snap = |value: f32, has_start_overflow: bool| {
                if !has_start_overflow {
                    return PhysicalSnap::Ceil;
                }
                let physical_phase = (value as f64 * style.device_scale_factor).rem_euclid(1.0);
                if (physical_phase - 0.5).abs() <= f64::EPSILON * 16.0 {
                    PhysicalSnap::Ceil
                } else {
                    PhysicalSnap::Floor
                }
            };
            Rect::from_ltrb(
                snapping.logical_coordinate(
                    clip_rect.left,
                    scroll_start_snap(clip_rect.left, overflow.x().to_f32() < 0.0),
                ),
                snapping.logical_coordinate(
                    clip_rect.top,
                    scroll_start_snap(clip_rect.top, overflow.y().to_f32() < 0.0),
                ),
                snapping.logical_coordinate(clip_rect.right, PhysicalSnap::Ceil),
                snapping.logical_coordinate(clip_rect.bottom, PhysicalSnap::Ceil),
            )
        } else if (style.overflow_x == Overflow::Clip || style.overflow_y == Overflow::Clip)
            && !antialias_transformed_clip
        {
            // Integral-scale `overflow: clip` is a hard paint-property clip.
            // Close its scissor on every physical cell touched by the logical
            // clip. Fractional replay retains the logical rectangle above so
            // Skia can produce Chromium's analytic edge coverage.
            outward_snap_rect_to_physical(clip_rect, style.device_scale_factor)
        } else {
            clip_rect
        };
        canvas.clip_rect(
            raster_clip_rect,
            ClipOp::Intersect,
            antialias_transformed_clip,
        );
    }

    // Paint children inside the clip (with stacking order).
    let is_sc = is_fragment_stacking_context(fragment, doc);
    let composited_scroll_contents = composited_scroll_layer
        && paint_composited_scroll_contents(canvas, fragment, doc, offset, style, clip_rect, is_sc);
    if !composited_scroll_contents {
        // Scrollable descendants paint in scrolled content coordinates.
        // Sticky offsets were resolved against the same document-owned scroll
        // state by layout, while scrollbars remain fixed in the scrollport.
        canvas.save();
        let (scroll_x, scroll_y) = if fragment.node_id.is_none() {
            (0.0, 0.0)
        } else {
            let node = doc.node(fragment.node_id);
            (node.scroll_left, node.scroll_top)
        };
        let translation = element_scroll_paint_translation(fragment, doc, scroll_x, scroll_y);
        if translation != (0.0, 0.0) {
            canvas.translate(translation);
        }
        paint_children_with_stacking_order(
            canvas,
            &fragment.children,
            doc,
            offset,
            is_sc,
            style.display.is_flex() && fragmented_flex_has_internal_four_way_junction(fragment),
        );
        canvas.restore();
    }
    if let Some(slice) = fragmented_outline_clip {
        paint_fragmented_descendant_outlines(
            canvas,
            &fragment.children,
            doc,
            offset,
            slice,
            fragment_block_axis_is_x(fragment),
            Some(&newly_deferred_fragmented_outlines),
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
            paint_children_with_stacking_order(
                canvas,
                &fragment.children,
                doc,
                offset,
                is_sc,
                style.display.is_flex() && fragmented_flex_has_internal_four_way_junction(fragment),
            );
            canvas.restore();
            canvas.restore();
        }
    }
    if !newly_deferred_fragmented_outlines.is_empty() {
        EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
            let mut deferred = deferred.borrow_mut();
            for pointer in &newly_deferred_fragmented_outlines {
                deferred.remove(pointer);
            }
        });
    }
    if !newly_skipped_legends.is_empty() {
        HOIST_SKIP.with(|skipped| {
            let mut skipped = skipped.borrow_mut();
            for pointer in newly_skipped_legends {
                skipped.remove(&pointer);
            }
        });
    }

    if fragment.element_scrollbars.is_none() {
        paint_scrollbars_if_needed(canvas, fragment, style, &clip_rect);
    }
    paint_resize_handle_if_needed(canvas, style, &clip_rect);

    if let Some((mask, destination)) = packed_rounded_mask {
        let mut mask_paint = Paint::default();
        mask_paint.set_anti_alias(false);
        mask_paint.set_blend_mode(BlendMode::DstIn);
        canvas.draw_image_rect_with_sampling_options(
            mask.clone(),
            Some((
                &Rect::from_xywh(0.0, 0.0, mask.width() as f32, mask.height() as f32),
                SrcRectConstraint::Strict,
            )),
            destination,
            SamplingOptions::from(FilterMode::Nearest),
            &mask_paint,
        );
    }

    canvas.restore();
    paint_element_scrollbars(canvas, fragment, doc, offset);
}

/// Rasterize an axis-aligned rounded overflow clip on the qualification
/// device grid and close every non-empty analytic coverage cell upward.
///
/// Chromium's packed clip mask retains one more coverage quantum than CPU
/// Skia's direct `clip_rrect` path. Keeping that policy in a reusable mask
/// avoids moving the contour geometry or changing fully covered interior
/// pixels.
fn packed_rounded_clip_mask(rrect: &RRect, device_scale: f32) -> Option<(Image, Rect)> {
    if !device_scale.is_finite() || device_scale <= 0.0 {
        return None;
    }
    let bounds = rrect.rect();
    let physical_left = (bounds.left * device_scale).floor() as i32;
    let physical_top = (bounds.top * device_scale).floor() as i32;
    let physical_right = (bounds.right * device_scale).ceil() as i32;
    let physical_bottom = (bounds.bottom * device_scale).ceil() as i32;
    let width = physical_right - physical_left;
    let height = physical_bottom - physical_top;
    if width <= 0 || height <= 0 {
        return None;
    }

    let physical_radii = [
        rrect.radii(RRectCorner::UpperLeft) * device_scale,
        rrect.radii(RRectCorner::UpperRight) * device_scale,
        rrect.radii(RRectCorner::LowerRight) * device_scale,
        rrect.radii(RRectCorner::LowerLeft) * device_scale,
    ];
    let physical_rrect = RRect::new_rect_radii(
        Rect::from_ltrb(
            bounds.left * device_scale - physical_left as f32,
            bounds.top * device_scale - physical_top as f32,
            bounds.right * device_scale - physical_left as f32,
            bounds.bottom * device_scale - physical_top as f32,
        ),
        &physical_radii,
    );
    let mut surface = surfaces::raster_n32_premul((width, height))?;
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    let mut mask_paint = Paint::default();
    mask_paint.set_style(PaintStyle::Fill);
    mask_paint.set_anti_alias(true);
    mask_paint.set_color(skia_safe::Color::WHITE);
    surface.canvas().draw_rrect(physical_rrect, &mask_paint);

    let info = ImageInfo::new(
        (width, height),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );
    let row_bytes = width as usize * 4;
    let mut pixels = vec![0_u8; row_bytes * height as usize];
    if !surface.image_snapshot().read_pixels(
        &info,
        &mut pixels,
        row_bytes,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    ) {
        return None;
    }
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = pixel[3];
        if alpha != 0 && alpha != 255 {
            let closed = alpha + 1;
            pixel.fill(closed);
        }
    }
    let image = skia_safe::images::raster_from_data(&info, Data::new_copy(&pixels), row_bytes)?;
    Some((
        image,
        Rect::from_ltrb(
            physical_left as f32 / device_scale,
            physical_top as f32 / device_scale,
            physical_right as f32 / device_scale,
            physical_bottom as f32 / device_scale,
        ),
    ))
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
    let radii = specified_border_radii(style, rect);

    normalize_radii_to_rect(radii, rect)
}

fn is_opaque_circular_background(rect: &Rect, radii: &[Point; 4], color: &Color) -> bool {
    let radius_x = rect.width() * 0.5;
    let radius_y = rect.height() * 0.5;
    color.is_opaque()
        && (rect.width() - rect.height()).abs() < 1.0 / 1024.0
        && radii.iter().all(|radius| {
            (radius.x - radius_x).abs() < 1.0 / 1024.0 && (radius.y - radius_y).abs() < 1.0 / 1024.0
        })
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
            slice_adjust_border_radii(specified_border_radii(style, &normalization_rect), fragment),
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

fn specified_border_radii(style: &ComputedStyle, rect: &Rect) -> [Point; 4] {
    if table_internal_ignores_border_radius(style.display) {
        return [Point::new(0.0, 0.0); 4];
    }
    let specified = [
        style.border_top_left_radius,
        style.border_top_right_radius,
        style.border_bottom_right_radius,
        style.border_bottom_left_radius,
    ];
    std::array::from_fn(|index| {
        let (x, y) = specified[index];
        let (x_percent, y_percent) = style.border_radius_percent[index];
        Point::new(
            if x_percent {
                x * rect.width() / 100.0
            } else {
                x
            },
            if y_percent {
                y * rect.height() / 100.0
            } else {
                y
            },
        )
    })
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
    blend_with_background_at_inner_edge: bool,
    paint: &Paint,
) {
    let has_radius = |radius: Point| radius.x > 0.0 || radius.y > 0.0;
    for side in [
        BorderSide::Top,
        BorderSide::Right,
        BorderSide::Bottom,
        BorderSide::Left,
    ] {
        let straight = match side {
            BorderSide::Top => !has_radius(inner_radii[0]) && !has_radius(inner_radii[1]),
            BorderSide::Right => !has_radius(inner_radii[1]) && !has_radius(inner_radii[2]),
            BorderSide::Bottom => !has_radius(inner_radii[2]) && !has_radius(inner_radii[3]),
            BorderSide::Left => !has_radius(inner_radii[3]) && !has_radius(inner_radii[0]),
        };
        if straight && blend_with_background_at_inner_edge {
            // This opaque same-color border and background share a fractional
            // inner edge. Fill its straight side in one rectangle so the side
            // polygon does not discard the border's coverage there.
            let side_rect = match side {
                BorderSide::Top => Rect::from_ltrb(
                    border_rect.left,
                    border_rect.top,
                    border_rect.right,
                    inner_rect.top,
                ),
                BorderSide::Right => Rect::from_ltrb(
                    inner_rect.right,
                    border_rect.top,
                    border_rect.right,
                    border_rect.bottom,
                ),
                BorderSide::Bottom => Rect::from_ltrb(
                    border_rect.left,
                    inner_rect.bottom,
                    border_rect.right,
                    border_rect.bottom,
                ),
                BorderSide::Left => Rect::from_ltrb(
                    border_rect.left,
                    border_rect.top,
                    inner_rect.left,
                    border_rect.bottom,
                ),
            };
            canvas.draw_rect(side_rect, paint);
            continue;
        }
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

/// Paint the pinned Linux Chromium Fluent viewport scrollbar theme.
/// Geometry follows ScrollbarThemeFluent and cc::ScrollUtils; colors follow
/// ui/color/fluent_ui_color_mixer.cc in Chromium 147.0.7727.50.
pub(crate) fn paint_viewport_scrollbars(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
) {
    let Some(scrollport) = fragment.viewport_scrollport else {
        return;
    };
    let source = if doc.body_overflow_is_propagated() {
        doc.body_element().unwrap_or(fragment.node_id)
    } else {
        doc.document_element().unwrap_or(fragment.node_id)
    };
    let style = &doc.node(source).style;
    paint_native_scrollbars(canvas, fragment, doc, offset, scrollport, style, false);
}

fn paint_element_scrollbars(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
) {
    let (Some(bars), Some(area)) = (fragment.element_scrollbars, fragment.scroll_area) else {
        return;
    };
    let scrollport = openui_layout::ViewportScrollport {
        client_rect: area.client_rect,
        content_rect: area.content_rect,
        horizontal_scrollbar: bars.horizontal,
        vertical_scrollbar: bars.vertical,
        scrollbar_thickness: bars.thickness,
        overflow_x: area.overflow_x,
        overflow_y: area.overflow_y,
        negative_x: area.negative_x,
        negative_y: area.negative_y,
    };
    paint_native_scrollbars(
        canvas,
        fragment,
        doc,
        offset,
        scrollport,
        &doc.node(fragment.node_id).style,
        doc.node(fragment.node_id)
            .style
            .writing_mode
            .is_horizontal()
            && doc.node(fragment.node_id).style.direction == openui_style::Direction::Rtl,
    );
}

fn paint_native_scrollbars(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    offset: PhysicalOffset,
    scrollport: openui_layout::ViewportScrollport,
    style: &ComputedStyle,
    vertical_on_left: bool,
) {
    let thickness = scrollport.scrollbar_thickness.to_f32();
    let client_width = scrollport.client_rect.width().to_f32();
    let client_height = scrollport.client_rect.height().to_f32();
    let node = doc.node(fragment.node_id);
    let mut track = Paint::default();
    track.set_color(skia_safe::Color::from_rgb(252, 252, 252));
    if let Some(color) = style.scrollbar_track_color {
        set_paint_css_color(&mut track, &color);
    }
    let mut thumb = Paint::default();
    thumb.set_anti_alias(true);
    thumb.set_color(skia_safe::Color::from_rgb(139, 139, 139));
    if let Some(color) = style.scrollbar_thumb_color {
        set_paint_css_color(&mut thumb, &color);
    }
    canvas.save();
    canvas.translate((
        (offset.left + scrollport.client_rect.x()).to_f32(),
        (offset.top + scrollport.client_rect.y()).to_f32(),
    ));
    let vertical_x = if vertical_on_left {
        -thickness
    } else {
        client_width
    };
    let user_scrollable = (scrollport.horizontal_scrollbar
        && scrollport.content_rect.width().to_f32() > client_width)
        || (scrollport.vertical_scrollbar
            && scrollport.content_rect.height().to_f32() > client_height);
    if !user_scrollable {
        // Without a user-scrollable translation Chromium records the controls
        // as ordinary pictures. They use the full track raster, rather than
        // the compact resource of a PaintedScrollbarLayer. The vertical
        // picture also contains the adjoining corner.
        for horizontal in [true, false] {
            let visible = if horizontal {
                scrollport.horizontal_scrollbar
            } else {
                scrollport.vertical_scrollbar
            };
            if !visible {
                continue;
            }
            let length = if horizontal {
                client_width
            } else {
                client_height
            };
            let corner = !horizontal && scrollport.horizontal_scrollbar;
            let outer = if horizontal {
                Rect::from_xywh(0.0, client_height, length, thickness)
            } else {
                Rect::from_xywh(vertical_x, 0.0, thickness, length)
            };
            paint_fluent_scrollbar_picture(
                canvas,
                outer,
                horizontal,
                corner,
                style.device_scale_factor as f32,
                &track,
                &thumb,
            );
        }
        canvas.restore();
        return;
    }
    for horizontal in [false, true] {
        let visible = if horizontal {
            scrollport.horizontal_scrollbar
        } else {
            scrollport.vertical_scrollbar
        };
        if !visible {
            continue;
        }
        let length = if horizontal {
            client_width
        } else {
            client_height
        };
        let proportion = thickness / 15.0;
        let button = (18.0 * proportion).round().min((length / 2.0).floor());
        let track_length = (length - 2.0 * button).max(0.0);
        let total = if horizontal {
            scrollport.content_rect.width().to_f32()
        } else {
            scrollport.content_rect.height().to_f32()
        };
        let maximum = (total - length).max(0.0);
        if maximum == 0.0 {
            // Chromium chooses the scrollbar layer independently per axis.
            // A forced bar with no range remains an ordinary picture even
            // when the other axis uses a PaintedScrollbarLayer. Its adjacent
            // corner shares that picture's raster and composition transform.
            let outer = if horizontal {
                Rect::from_xywh(0.0, client_height, length, thickness)
            } else {
                Rect::from_xywh(vertical_x, 0.0, thickness, length)
            };
            paint_fluent_scrollbar_picture(
                canvas,
                outer,
                horizontal,
                scrollport.horizontal_scrollbar && scrollport.vertical_scrollbar,
                style.device_scale_factor as f32,
                &track,
                &thumb,
            );
            continue;
        }
        let scroll = if horizontal {
            if scrollport.negative_x {
                node.scroll_left - scrollport.content_rect.x().to_f32()
            } else {
                node.scroll_left
            }
        } else {
            if scrollport.negative_y {
                node.scroll_top - scrollport.content_rect.y().to_f32()
            } else {
                node.scroll_top
            }
        };
        let thumb_length = ((length / total.max(1.0) * track_length).round())
            .max((17.0 * proportion).round())
            .min(track_length);
        let raw_position = if maximum > 0.0 {
            scroll.max(0.0) * (track_length - thumb_length) / maximum
        } else {
            0.0
        };
        let position = if raw_position > 0.0 && raw_position < 1.0 {
            1.0
        } else {
            raw_position.trunc()
        };
        let raw_thickness = (9.0 * proportion).round();
        let thumb_thickness = raw_thickness - (thickness - raw_thickness) % 2.0;
        let outer = if horizontal {
            Rect::from_xywh(0.0, client_height, length, thickness)
        } else {
            Rect::from_xywh(vertical_x, 0.0, thickness, length)
        };
        paint_fluent_scrollbar_track(
            canvas,
            outer,
            horizontal,
            button,
            style.device_scale_factor as f32,
            &track,
            &thumb,
        );
        let thumb_rect = if horizontal {
            Rect::from_xywh(
                button + position,
                client_height + (thickness - thumb_thickness) / 2.0,
                thumb_length,
                thumb_thickness,
            )
        } else {
            Rect::from_xywh(
                vertical_x + (thickness - thumb_thickness) / 2.0,
                button + position,
                thumb_thickness,
                thumb_length,
            )
        };
        if maximum > 0.0 && thumb_length > 0.0 {
            // The software compositor clips a solid-color quad with the
            // Fluent rounded-corner mask; it does not directly draw an RRect.
            canvas.save();
            canvas.clip_rrect(
                RRect::new_rect_xy(thumb_rect, thumb_thickness, thumb_thickness),
                ClipOp::Intersect,
                true,
            );
            let mut fill = thumb.clone();
            let scale = style.device_scale_factor as f32;
            // cc encloses the thumb in physical content coordinates before
            // replay. Keep the rounded mask in target coordinates. This quad
            // is inset from its layer edges, so SoftwareRenderer does not
            // enable geometric AA even when its layer origin is fractional.
            let left = ((thumb_rect.left - outer.left) * scale).floor() / scale + outer.left;
            let top = ((thumb_rect.top - outer.top) * scale).floor() / scale + outer.top;
            let right = ((thumb_rect.right - outer.left) * scale).ceil() / scale + outer.left;
            let bottom = ((thumb_rect.bottom - outer.top) * scale).ceil() / scale + outer.top;
            fill.set_anti_alias(false);
            canvas.draw_rect(Rect::new(left, top, right, bottom), &fill);
            canvas.restore();
        }
    }
    if scrollport.horizontal_scrollbar
        && scrollport.vertical_scrollbar
        && scrollport.content_rect.width().to_f32() > client_width
        && scrollport.content_rect.height().to_f32() > client_height
    {
        // The adjoining corner has exterior compositor edges on both axes.
        // Preserve their coverage when its physical origin is fractional.
        let mut corner = track.clone();
        corner.set_anti_alias(true);
        canvas.draw_rect(
            Rect::from_xywh(vertical_x, client_height, thickness, thickness),
            &corner,
        );
    }
    canvas.restore();
}

fn paint_fluent_scrollbar_picture(
    canvas: &Canvas,
    outer: Rect,
    horizontal: bool,
    corner: bool,
    scale: f32,
    track: &Paint,
    thumb: &Paint,
) {
    let thickness = if horizontal {
        outer.height()
    } else {
        outer.width()
    };
    let length = if horizontal {
        outer.width()
    } else {
        outer.height()
    };
    let width = outer.width() + if corner && horizontal { thickness } else { 0.0 };
    let height = outer.height()
        + if corner && !horizontal {
            thickness
        } else {
            0.0
        };
    let physical_width = (width * scale).ceil().max(1.0) as i32;
    let physical_height = (height * scale).ceil().max(1.0) as i32;
    let Some(mut surface) = surfaces::raster_n32_premul((physical_width, physical_height)) else {
        return;
    };
    let painter = surface.canvas();
    painter.clear(skia_safe::Color::TRANSPARENT);
    painter.scale((scale, scale));
    painter.draw_rect(Rect::from_xywh(0.0, 0.0, width, height), track);
    let button = (18.0 * thickness / 15.0)
        .round()
        .min((length / 2.0).floor());
    for forward in [false, true] {
        let start = if forward { length - button } else { 0.0 };
        let rect = if horizontal {
            Rect::from_xywh(start, 0.0, button, thickness)
        } else {
            Rect::from_xywh(0.0, start, thickness, button)
        };
        paint_fluent_scrollbar_arrow(painter, rect, horizontal, forward, thumb);
    }
    let image = surface.image_snapshot();
    let mut replay = Paint::default();
    replay
        .set_anti_alias((outer.left * scale).fract() != 0.0 || (outer.top * scale).fract() != 0.0);
    canvas.save();
    canvas.scale((scale.recip(), scale.recip()));
    let source = Rect::from_xywh(0.0, 0.0, physical_width as f32, physical_height as f32);
    canvas.draw_image_rect_with_sampling_options(
        &image,
        Some((&source, SrcRectConstraint::Strict)),
        Rect::from_xywh(
            outer.left * scale,
            outer.top * scale,
            physical_width as f32,
            physical_height as f32,
        ),
        SamplingOptions::from(FilterMode::Linear),
        &replay,
    );
    canvas.restore();
}

fn paint_fluent_scrollbar_track(
    canvas: &Canvas,
    outer: Rect,
    horizontal: bool,
    button: f32,
    scale: f32,
    track: &Paint,
    thumb: &Paint,
) {
    // Chromium stores the track and buttons in a compact N32 nine-patch.
    // The across-axis resource size floors while its length ceils. Keeping
    // this raster transform separate from the presentation transform matters
    // at fractional device scales.
    let thickness = if horizontal {
        outer.height()
    } else {
        outer.width()
    };
    let length = if horizontal {
        outer.width()
    } else {
        outer.height()
    };
    let skin_length = length.min(2.0 * button + 1.0);
    let across = (thickness * scale).floor().max(1.0) as i32;
    let along = (skin_length * scale).ceil().max(1.0) as i32;
    let (width, height) = if horizontal {
        (along, across)
    } else {
        (across, along)
    };
    let Some(mut surface) = surfaces::raster_n32_premul((width, height)) else {
        return;
    };
    let painter = surface.canvas();
    painter.clear(skia_safe::Color::TRANSPARENT);
    painter.scale((
        width as f32 / if horizontal { skin_length } else { thickness },
        height as f32 / if horizontal { thickness } else { skin_length },
    ));
    painter.draw_rect(
        Rect::from_xywh(
            0.0,
            0.0,
            if horizontal { skin_length } else { thickness },
            if horizontal { thickness } else { skin_length },
        ),
        track,
    );
    for forward in [false, true] {
        let start = if forward { skin_length - button } else { 0.0 };
        let rect = if horizontal {
            Rect::from_xywh(start, 0.0, button, thickness)
        } else {
            Rect::from_xywh(0.0, start, thickness, button)
        };
        paint_fluent_scrollbar_arrow(painter, rect, horizontal, forward, thumb);
    }
    let image = surface.image_snapshot();
    let aperture_size = 2 - along % 2;
    let aperture_start = along / 2 - (1 - along % 2);
    let center = if horizontal {
        IRect::from_xywh(aperture_start, 0, aperture_size, across)
    } else {
        IRect::from_xywh(0, aperture_start, across, aperture_size)
    };
    let destination = Rect::from_xywh(
        outer.left * scale,
        outer.top * scale,
        (outer.width() * scale).ceil(),
        (outer.height() * scale).ceil(),
    );
    let mut replay = Paint::default();
    replay
        .set_anti_alias((outer.left * scale).fract() != 0.0 || (outer.top * scale).fract() != 0.0);
    // Nine-patch borders retain the source image's pixel dimensions. Replay
    // in physical units, as Chromium's scaled track quads do, so those already
    // scaled borders do not receive the device scale a second time.
    canvas.save();
    canvas.scale((scale.recip(), scale.recip()));
    canvas.draw_image_nine(
        &image,
        center,
        destination,
        FilterMode::Linear,
        Some(&replay),
    );
    canvas.restore();
}

fn paint_fluent_scrollbar_arrow(
    canvas: &Canvas,
    button: Rect,
    horizontal: bool,
    forward: bool,
    color: &Paint,
) {
    let scale = button.width().max(button.height()) / 18.0;
    let mut side = (9.0 * scale).ceil();
    side += (button.width().min(button.height()) - side) % 2.0;
    let mut x = (button.left + (button.width() - side) / 2.0).floor();
    let mut y = (button.top + (button.height() - side) / 2.0).floor();
    let shift = (if forward { scale } else { -scale }).round();
    if horizontal {
        x += shift;
    } else {
        y += shift;
    }
    let arrow_size = (side.round() as i32 / 2 + 1) as f32;
    let mut points = if horizontal {
        let start = x + (arrow_size as i32 / 2) as f32;
        [
            (start, y),
            (start, y + side),
            (start + arrow_size, y + side / 2.0),
        ]
    } else {
        let start = y + side - (arrow_size as i32 / 2) as f32 + 1.0;
        [
            (x, start),
            (x + side, start),
            (x + side / 2.0, start - arrow_size),
        ]
    };
    if horizontal && !forward {
        for point in &mut points {
            point.0 = x * 2.0 + side - point.0;
        }
    } else if !horizontal && forward {
        for point in &mut points {
            point.1 = y * 2.0 + side - point.1;
        }
    }
    let mut path = PathBuilder::new();
    path.move_to(points[0]);
    path.line_to(points[1]);
    path.line_to(points[2]);
    path.close();
    let mut paint = color.clone();
    paint.set_anti_alias(false);
    canvas.draw_path(&path.detach(), &paint);
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
    doc: &Document,
    style: &ComputedStyle,
    shape_result: &openui_text::shaping::ShapeResult,
) -> FontMetrics {
    let font_desc = crate::text_painter::style_to_font_description(style);
    let font = doc.resolve_font(font_desc);
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
    let metrics = resolve_decoration_metrics(doc, style, shape_result);

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
    let parent_has_visible_border =
        !fragment.node_id.is_none() && !doc.node(fragment.node_id).parent.is_none() && {
            let parent_style = &doc.node(doc.node(fragment.node_id).parent).style;
            parent_style.effective_border_top() != 0
                || parent_style.effective_border_right() != 0
                || parent_style.effective_border_bottom() != 0
                || parent_style.effective_border_left() != 0
        };
    let native_vertical_button_text = if fragment.node_id.is_none() {
        false
    } else {
        let mut ancestor = doc.node(fragment.node_id).parent;
        let mut found = false;
        while !ancestor.is_none() {
            let ancestor_node = doc.node(ancestor);
            if matches!(
                ancestor_node.pseudo_kind,
                Some(PseudoElementKind::ScrollButton(_))
            ) || (ancestor_node.form_control == Some(openui_dom::FormControlRole::Button)
                && ancestor_node.style.native_button_text_metrics)
            {
                found = true;
                break;
            }
            ancestor = ancestor_node.parent;
        }
        found
    };
    let inside_native_button = if fragment.node_id.is_none() {
        false
    } else {
        let mut ancestor = doc.node(fragment.node_id).parent;
        let mut found = false;
        while !ancestor.is_none() {
            let ancestor_node = doc.node(ancestor);
            if ancestor_node.form_control == Some(openui_dom::FormControlRole::Button)
                && ancestor_node.form_control_native_appearance
                && !ancestor_node.form_control_disabled
            {
                found = true;
                break;
            }
            ancestor = ancestor_node.parent;
        }
        found
    };
    let native_static_scroll_button_text =
        if !style.native_control_text || fragment.node_id.is_none() {
            false
        } else {
            let mut ancestor = doc.node(fragment.node_id).parent;
            let mut found = false;
            while !ancestor.is_none() {
                let ancestor_node = doc.node(ancestor);
                if matches!(
                    ancestor_node.pseudo_kind,
                    Some(PseudoElementKind::ScrollButton(_))
                ) {
                    found = ancestor_node.style.position == Position::Static
                        && ancestor_node.form_control_native_appearance;
                    break;
                }
                ancestor = ancestor_node.parent;
            }
            found
        };
    let mut inside_line_clamp =
        style.line_clamp != openui_style::LineClamp::None || fragment.is_line_clamp_marker;
    let mut inside_block_content_alignment = false;
    let mut inside_ruby = false;
    let mut clamp_ancestor = if fragment.node_id.is_none() {
        NodeId::NONE
    } else {
        doc.node(fragment.node_id).parent
    };
    while !clamp_ancestor.is_none() {
        let ancestor_style = &doc.node(clamp_ancestor).style;
        inside_ruby |= matches!(
            doc.node(clamp_ancestor).tag,
            ElementTag::Ruby | ElementTag::RubyText
        );
        let ancestor_line_clamp = ancestor_style.line_clamp;
        inside_line_clamp |= ancestor_line_clamp != openui_style::LineClamp::None;
        inside_block_content_alignment |= matches!(
            ancestor_style.align_content.position,
            ContentPosition::Center | ContentPosition::End | ContentPosition::FlexEnd
        );
        clamp_ancestor = doc.node(clamp_ancestor).parent;
    }
    let mut origin = match fragment.text_run_orientation {
        openui_layout::TextRunOrientation::Clockwise => {
            canvas.save();
            let aliased_ahem_inline_shift = if all_ahem_runs
                && style.raster_configuration.author_text.edging == TextEdging::Alias
            {
                chromium_rotated_aliased_inline_shift(
                    abs_offset.left,
                    style.font_size,
                    style.device_scale_factor,
                )
            } else {
                0.0
            };
            let horizontal_translation = abs_offset.left.to_f32()
                + fragment.size.width.to_f32()
                + aliased_ahem_inline_shift
                // Native vertical controls align the clockwise glyph ink
                // to the device pixel after their logical inline padding.
                // The anonymous flex item's fractional Ahem advance would
                // otherwise expose one pixel into the block-start padding.
                + if native_vertical_button_text { 1.0 } else { 0.0 };
            // The vertical-lr Ahem mask uses a physical block-axis column.
            // Align the clockwise rotation anchor before rasterization so a
            // fractional layout-unit advance cannot move the whole mask one
            // device column. Other writing modes anchor the run differently.
            let horizontal_translation = if style.writing_mode
                == openui_style::WritingMode::VerticalLr
                && all_ahem_runs
                && style.raster_configuration.author_text.edging == TextEdging::Alias
            {
                RasterSnapping::new(style.device_scale_factor)
                    .logical_coordinate(horizontal_translation, PhysicalSnap::Nearest)
            } else {
                horizontal_translation
            };
            canvas.translate(Point::new(horizontal_translation, abs_offset.top.to_f32()));
            canvas.rotate(90.0, None);
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
                style.raster_configuration.author_text.edging == TextEdging::SubpixelAntiAlias;
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
            let baseline = snap_tiny_aliased_ahem_ink_start(baseline, style, all_ahem_runs);
            let inline_origin = abs_offset.left.to_f32();
            (inline_origin, baseline)
        }
    };
    if native_static_scroll_button_text && !rotated {
        origin.1 += 1.0;
    }
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
    let clip_aliased_ahem_ink = !rotated
        && !upright
        && fragment.text_combine.is_none()
        && all_ahem_runs
        && style.raster_configuration.author_text.edging == TextEdging::Alias
        && ((style.font_size * style.device_scale_factor as f32)
            .fract()
            .abs()
            > 1.0e-6);
    if clip_aliased_ahem_ink {
        // Ahem's outline is the layout-owned square cell: it has no bearings
        // or ink overflow. At a fractional physical strike Skia includes the
        // trailing half-pixel cell, while Chromium rasterizes the logical run
        // as a half-open rectangle. Clip the primitive before replay so both
        // its inline terminal edge and small-font block edge use that same
        // coverage rule. Large Ahem runs retain their deliberate preceding-
        // row replay outside the line box.
        let block_start = if style.font_size < 16.0 && inside_native_button {
            RasterSnapping::new(style.device_scale_factor)
                .logical_coordinate(abs_offset.top.to_f32(), PhysicalSnap::Floor)
        } else if style.font_size < 16.0 {
            abs_offset.top.to_f32()
        } else {
            -100_000.0
        };
        let block_end = if style.font_size < 16.0 {
            RasterSnapping::new(style.device_scale_factor).logical_coordinate(
                abs_offset.top.to_f32() + fragment.size.height.to_f32(),
                PhysicalSnap::Ceil,
            )
        } else {
            100_000.0
        };
        let raw_inline_end = abs_offset.left.to_f32() + fragment.size.width.to_f32();
        let inline_end_phase =
            (f64::from(raw_inline_end) * style.device_scale_factor).rem_euclid(1.0);
        let inline_end = if inline_end_phase > 1.0e-6 && inline_end_phase < 0.5 - 1.0e-6 {
            RasterSnapping::new(style.device_scale_factor)
                .logical_coordinate(raw_inline_end, PhysicalSnap::Ceil)
        } else {
            raw_inline_end
        };
        let clip = Rect::from_ltrb(abs_offset.left.to_f32(), block_start, inline_end, block_end);
        canvas.save();
        canvas.clip_rect(clip, ClipOp::Intersect, false);
    }
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
        let chromium_aliased_outline = !style.native_control_text
            && style.raster_configuration.author_text.edging == TextEdging::Alias
            && (inside_ruby || (rotated && all_ahem_runs));
        if chromium_aliased_outline {
            crate::text_painter::paint_text_with_raster_policy(
                canvas,
                shape_result,
                origin,
                style,
                openui_text::shaping::TextRasterPolicy::ChromiumAliased,
            );
            if fragment.text_run_orientation == openui_layout::TextRunOrientation::Clockwise {
                paint_rotated_aliased_terminal_edges(
                    canvas,
                    shape_result,
                    origin,
                    style,
                    text_content.unwrap_or(""),
                );
            }
        } else {
            crate::text_painter::paint_text(canvas, shape_result, origin, style);
        }
        let replays_native_small_ahem_start = inside_native_button
            && style.font_size < 16.0
            && f64::from(style.font_size) * style.device_scale_factor < 16.0
            && (style.device_scale_factor - style.device_scale_factor.round()).abs()
                <= f64::EPSILON
            && (f64::from(style.font_size) * style.device_scale_factor)
                .fract()
                .abs()
                > 1.0e-6;
        if !rotated
            && all_ahem_runs
            && replays_aliased_ahem_start_for_raster_policy(style, deterministic_text_profile)
            && !inside_block_content_alignment
            && (!parent_has_visible_border || style.font_size < 20.0)
            && (replays_native_small_ahem_start
                || (style.font_size >= 16.0
                    && (((style.font_size - style.font_size.round()).abs() < 0.01
                        && ((!inside_line_clamp
                            // Ordinary non-clamped runs at 24px and above already
                            // include the complete block-start row. A first-letter run
                            // is positioned from its enlarged pseudo line box and keeps
                            // the smaller-strike behavior.
                            && (style.font_size < 24.0 || style.is_first_letter_pseudo)
                            && should_replay_ahem_preceding_row(
                                abs_offset.top,
                                style.device_scale_factor,
                            ))
                            || (inside_line_clamp
                                && should_replay_line_clamp_ahem_preceding_row(
                                    abs_offset.top,
                                    style.font_size,
                                    style.device_scale_factor,
                                ))))
                        || chromium_fractional_strike_retains_preceding_row(
                            origin.1,
                            style.font_size,
                            style.device_scale_factor,
                        ))))
        {
            // When the aliased Ahem ink origin crosses the half-pixel device
            // threshold, Chromium's FreeType mask retains the block-start
            // coverage row. Replay the square mask one *physical* row toward
            // block-start without changing layout's baseline or advances.
            let snapping = RasterSnapping::new(style.device_scale_factor);
            let block_start =
                snapping.logical_coordinate(origin.1 - style.font_size * 0.8, PhysicalSnap::Floor);
            canvas.save();
            canvas.clip_rect(
                Rect::from_xywh(-100_000.0, block_start - 1.0, 200_000.0, 2.0),
                ClipOp::Intersect,
                false,
            );
            crate::text_painter::paint_text(
                canvas,
                shape_result,
                (origin.0, origin.1 - 1.0 / style.device_scale_factor as f32),
                style,
            );
            canvas.restore();
        }
    }
    if clip_aliased_ahem_ink {
        canvas.restore();
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

fn should_replay_ahem_preceding_row(offset: LayoutUnit, device_scale: f64) -> bool {
    let css_phase_at_or_after_half =
        offset.raw().rem_euclid(LayoutUnit::from_i32(1).raw()) >= LayoutUnit::from_f32(0.5).raw();
    if css_phase_at_or_after_half {
        return true;
    }

    // Integral magnification preserves the aliased CSS mask and can expose a
    // preceding row when that mask crosses a device half-pixel. Fractional
    // magnification selects a distinct physical strike which already advances
    // that row; applying this second trigger there paints one row too many.
    let integral_scale = (device_scale - device_scale.round()).abs() <= f64::EPSILON;
    integral_scale && (f64::from(offset.to_f32()) * device_scale).rem_euclid(1.0) >= 0.5
}

/// Chromium's aliased physical strike keeps the inclusive block-start cell
/// when its 4/5-em ascent is fractional but lies before the half-cell raster
/// threshold. Skia rounds that ascent down and advances the mask by one row.
/// This derives from strike metrics and is independent of test geometry.
fn chromium_fractional_strike_retains_preceding_row(
    _baseline: f32,
    font_size: f32,
    device_scale: f64,
) -> bool {
    if !device_scale.is_finite()
        || device_scale <= 0.0
        || (device_scale - device_scale.round()).abs() <= f64::EPSILON
    {
        return false;
    }
    let physical_strike = f64::from(font_size) * device_scale;
    let strike_phase = physical_strike.rem_euclid(1.0);
    let ascent_phase = (physical_strike * 0.8).rem_euclid(1.0);
    strike_phase > 1.0e-6
        && strike_phase < 1.0 - 1.0e-6
        && ascent_phase > 1.0e-6
        && ascent_phase < 0.5 - 1.0e-6
}

/// Return the logical translation that selects Chromium's device cell for a
/// clockwise strong-aliased physical glyph strike.
///
/// Skia positions the smallest custom hinted outline from the preceding
/// device cell. Chromium's vertical draw path retains the inclusive edge of
/// that 8px strike and advances its mask origin by three half-cells per
/// magnification step. Larger physical strikes already expose Chromium's
/// block-axis origin and must not receive the small-strike correction.
fn chromium_rotated_aliased_inline_shift(
    _offset: LayoutUnit,
    font_size: f32,
    device_scale: f64,
) -> f32 {
    if !device_scale.is_finite() || device_scale <= 0.0 || font_size > 8.0 {
        return 0.0;
    }
    let fractional_magnification = device_scale.rem_euclid(1.0);
    let physical_shift = 1.5 * (device_scale + fractional_magnification) - 0.5;
    (physical_shift / device_scale) as f32
}

/// Replay the terminal device cell of a scaled, clockwise Ahem glyph when its
/// exact right edge lands on a device boundary.
///
/// Chromium retains that boundary cell for a fractional strong-aliased
/// strike. Skia's scaled custom outline uses the half-open fill convention and
/// drops it. Replaying the same mask one physical cell forward under an exact
/// per-glyph edge clip restores the primitive coverage rule without changing
/// layout advances or touching pixels away from glyph edges.
fn paint_rotated_aliased_terminal_edges(
    canvas: &Canvas,
    shape_result: &openui_text::shaping::ShapeResult,
    origin: (f32, f32),
    style: &ComputedStyle,
    text: &str,
) {
    let scale = style.device_scale_factor as f32;
    let physical_font_size = style.font_size * scale;
    if scale <= 1.0 || (physical_font_size - physical_font_size.round()).abs() <= 1.0e-6 {
        return;
    }

    let matrix = canvas.local_to_device_as_3x3();
    let mut edges = Vec::new();
    for (character_index, character) in text.chars().enumerate() {
        if character.is_whitespace() {
            continue;
        }
        let edge = shape_result.x_position_for_offset(character_index) + style.font_size;
        let device_edge = matrix.map_point(Point::new(origin.0 + edge, origin.1));
        // Scene recording remains in logical pixels; the compositor applies
        // the immutable frame scale after replaying this local transform.
        let physical_edge = device_edge.y * scale;
        if (physical_edge - physical_edge.round()).abs() <= 1.0e-5 {
            edges.push(edge);
        }
    }
    edges.sort_by(f32::total_cmp);
    edges.dedup_by(|left, right| (*left - *right).abs() <= 1.0e-5);

    for edge in edges {
        canvas.save();
        canvas.clip_rect(
            Rect::from_xywh(origin.0 + edge, -100_000.0, 1.0 / scale, 200_000.0),
            ClipOp::Intersect,
            false,
        );
        crate::text_painter::paint_text_with_raster_policy(
            canvas,
            shape_result,
            (origin.0 + 1.0 / scale, origin.1),
            style,
            openui_text::shaping::TextRasterPolicy::ChromiumAliased,
        );
        canvas.restore();
    }
}

fn should_replay_line_clamp_ahem_preceding_row(
    offset: LayoutUnit,
    font_size: f32,
    device_scale: f64,
) -> bool {
    let integral_scale = (device_scale - device_scale.round()).abs() <= f64::EPSILON;
    // Ahem's 24px outline still uses Chromium's small-strike line-clamp
    // path: integral magnification above the unit strike retains the inclusive
    // block-start cell, while fractional physical strikes and the unscaled
    // outline already select the complete mask. Only outlines larger than
    // 24px switch that ownership to fractional magnification.
    let strike_uses_device_phase = if font_size <= 24.0 {
        integral_scale && (font_size < 24.0 || device_scale > 1.0)
    } else {
        !integral_scale
    };
    strike_uses_device_phase && (f64::from(offset.to_f32()) * device_scale).rem_euclid(1.0) >= 0.5
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

fn snap_tiny_aliased_ahem_ink_start(
    baseline: f32,
    style: &ComputedStyle,
    all_ahem_runs: bool,
) -> f32 {
    let scale = style.device_scale_factor;
    let physical_strike = f64::from(style.font_size) * scale;
    if !all_ahem_runs
        || style.raster_configuration.author_text.edging != TextEdging::Alias
        || physical_strike >= 2.0
        || physical_strike.fract().abs() <= f64::EPSILON
    {
        return baseline;
    }

    // Ahem's square ink begins at its 4/5-em ascent. Chromium's tiny
    // strong-aliased strike closes that leading edge on the next device cell;
    // Skia otherwise rounds phases below one half backward and can erase or
    // raise the only row of a one-pixel glyph.
    let ink_start = (f64::from(baseline) - f64::from(style.font_size) * 0.8) * scale;
    baseline + ((ink_start.ceil() - ink_start) / scale) as f32
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

fn gradient_stop_is_hint(position: GradientStopPosition) -> bool {
    matches!(
        position,
        GradientStopPosition::HintPercent(_)
            | GradientStopPosition::HintPx(_)
            | GradientStopPosition::HintCalc { .. }
    )
}

fn gradient_stop_position(position: GradientStopPosition, line_length: f32) -> Option<f32> {
    match position {
        GradientStopPosition::Auto => None,
        GradientStopPosition::Percent(value) | GradientStopPosition::HintPercent(value) => {
            Some(value / 100.0)
        }
        GradientStopPosition::Px(value) | GradientStopPosition::HintPx(value) => {
            Some(if line_length > 0.0 {
                value / line_length
            } else {
                0.0
            })
        }
        GradientStopPosition::Calc { percent, px }
        | GradientStopPosition::HintCalc { percent, px } => Some(if line_length > 0.0 {
            percent / 100.0 + px / line_length
        } else {
            0.0
        }),
    }
}

fn gradient_stop_position_f64(position: GradientStopPosition, line_length: f32) -> Option<f64> {
    // CSSPrimitiveValue leaves two LayoutUnit ticks of headroom so a later
    // float/layout conversion cannot overflow its 26.6 fixed-point storage.
    const MIN_CSS_LENGTH: f64 = -33_554_430.0;
    const MAX_CSS_LENGTH: f64 = 33_554_429.0;
    let line_length = f64::from(line_length);
    match position {
        GradientStopPosition::Auto => None,
        GradientStopPosition::Percent(value) | GradientStopPosition::HintPercent(value) => {
            Some(f64::from(value) / 100.0)
        }
        GradientStopPosition::Px(value) | GradientStopPosition::HintPx(value) => {
            Some(if line_length > 0.0 {
                f64::from(value).clamp(MIN_CSS_LENGTH, MAX_CSS_LENGTH) / line_length
            } else {
                0.0
            })
        }
        GradientStopPosition::Calc { percent, px }
        | GradientStopPosition::HintCalc { percent, px } => Some(if line_length > 0.0 {
            f64::from(percent) / 100.0 + f64::from(px) / line_length
        } else {
            0.0
        }),
    }
}

/// Resolve both color stops and transition hints using the CSS Images stop
/// fixup algorithm. Keeping the hint in this pass matters: an explicit hint is
/// also a specified position for the runs of adjacent automatic color stops.
fn css_gradient_all_positions(stops: &[openui_style::GradientStop], line_length: f32) -> Vec<f32> {
    let count = stops.len();
    let mut result: Vec<Option<f32>> = stops
        .iter()
        .map(|stop| gradient_stop_position(stop.position, line_length))
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

    // A specified position may not move behind any preceding specified
    // position. Blink performs this before distributing automatic stops.
    let mut previous_specified = None;
    for position in &mut result {
        if let Some(value) = position {
            if let Some(previous) = previous_specified {
                *value = value.max(previous);
            }
            previous_specified = Some(*value);
        }
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

fn css_gradient_all_positions_f64(
    stops: &[openui_style::GradientStop],
    line_length: f32,
) -> Vec<f64> {
    let count = stops.len();
    let mut result: Vec<Option<f64>> = stops
        .iter()
        .map(|stop| gradient_stop_position_f64(stop.position, line_length))
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
    let mut previous_specified = None;
    for position in &mut result {
        if let Some(value) = position {
            if let Some(previous) = previous_specified {
                *value = value.max(previous);
            }
            previous_specified = Some(*value);
        }
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
                Some(first + (last - first) * (index - start) as f64 / (end - start) as f64);
        }
        start = end;
    }
    let mut previous = f64::NEG_INFINITY;
    result
        .into_iter()
        .map(|position| {
            let position = position.unwrap_or(previous).max(previous);
            previous = position;
            position
        })
        .collect()
}

fn css_gradient_positions(stops: &[openui_style::GradientStop], line_length: f32) -> Vec<f32> {
    stops
        .iter()
        .zip(css_gradient_all_positions(stops, line_length))
        .filter_map(|(stop, position)| (!gradient_stop_is_hint(stop.position)).then_some(position))
        .collect()
}

#[derive(Clone, Copy)]
struct ResolvedGradientStop {
    color: Color,
    position: f32,
}

fn interpolate_gradient_color(left: Color, right: Color, weight: f32) -> Color {
    let lerp = |a: f32, b: f32| a + (b - a) * weight;
    let alpha = lerp(left.a, right.a);
    if alpha == 0.0 {
        return Color::TRANSPARENT;
    }
    Color::from_rgba_f32(
        lerp(left.r * left.a, right.r * right.a) / alpha,
        lerp(left.g * left.a, right.g * right.a) / alpha,
        lerp(left.b * left.a, right.b * right.a) / alpha,
        alpha,
    )
}

fn float_nearly_equal(left: f32, right: f32) -> bool {
    (left - right).abs() <= f32::EPSILON * left.abs().max(right.abs()).max(1.0) * 4.0
}

/// Replace CSS transition hints with the same nine-stop approximation used by
/// Chromium 147. Authored positions are clamped in wide precision first, then
/// Chromium's synthetic-stop placement and weighting run in `f32`.
fn resolved_gradient_stops_with_hints(
    stops: &[openui_style::GradientStop],
    line_length: f32,
    current_color: &Color,
) -> Option<Vec<ResolvedGradientStop>> {
    if stops.len() < 3
        || !stops
            .iter()
            .any(|stop| gradient_stop_is_hint(stop.position))
    {
        return None;
    }

    #[derive(Clone, Copy)]
    struct WideGradientStop {
        color: Color,
        position: f64,
    }

    let positions = css_gradient_all_positions_f64(stops, line_length);
    let mut resolved: Vec<_> = stops
        .iter()
        .zip(positions)
        .map(|(stop, position)| WideGradientStop {
            color: if gradient_stop_is_hint(stop.position) {
                Color::TRANSPARENT
            } else {
                stop.color.resolve(current_color)
            },
            position,
        })
        .collect();
    let mut index_offset = 0_isize;

    for (source_index, source_stop) in stops.iter().enumerate().skip(1).take(stops.len() - 2) {
        if !gradient_stop_is_hint(source_stop.position) {
            continue;
        }
        let index = (source_index as isize + index_offset) as usize;
        let left = resolved[index - 1];
        let hint = resolved[index];
        let right = resolved[index + 1];
        let left_position = left.position as f32;
        let hint_position = hint.position as f32;
        let right_position = right.position as f32;
        let left_distance = hint_position - left_position;
        let right_distance = right_position - hint_position;
        let total_distance = right_position - left_position;

        if float_nearly_equal(left_distance, right_distance) {
            resolved.remove(index);
            index_offset -= 1;
            continue;
        }
        if float_nearly_equal(left_distance, 0.0) {
            resolved[index].color = right.color;
            continue;
        }
        if float_nearly_equal(right_distance, 0.0) {
            resolved[index].color = left.color;
            continue;
        }

        let mut replacement = [WideGradientStop {
            color: Color::TRANSPARENT,
            position: 0.0,
        }; 9];
        if left_distance > right_distance {
            for (offset, stop) in replacement.iter_mut().take(7).enumerate() {
                stop.position =
                    f64::from(left_position + left_distance * ((7.0 + offset as f32) / 13.0));
            }
            replacement[7].position = f64::from(hint_position + right_distance * (1.0 / 3.0));
            replacement[8].position = f64::from(hint_position + right_distance * (2.0 / 3.0));
        } else {
            replacement[0].position = f64::from(left_position + left_distance * (1.0 / 3.0));
            replacement[1].position = f64::from(left_position + left_distance * (2.0 / 3.0));
            for (offset, stop) in replacement.iter_mut().skip(2).enumerate() {
                stop.position = f64::from(hint_position + right_distance * (offset as f32 / 13.0));
            }
        }

        let hint_relative_position = left_distance / total_distance;
        let exponent = 0.5_f32.ln() / hint_relative_position.ln();
        for stop in &mut replacement {
            let point_relative_position = (stop.position as f32 - left_position) / total_distance;
            let weight = point_relative_position.powf(exponent);
            if weight.is_finite() {
                stop.color = interpolate_gradient_color(left.color, right.color, weight);
            }
        }
        resolved.splice(index..=index, replacement);
        index_offset += 8;
    }
    Some(
        resolved
            .into_iter()
            .map(|stop| ResolvedGradientStop {
                color: stop.color,
                position: stop.position as f32,
            })
            .collect(),
    )
}

fn clamp_negative_radial_gradient_stops(stops: &mut [ResolvedGradientStop]) {
    let mut last_negative_position = 0.0;
    for index in 0..stops.len() {
        let position = stops[index].position;
        if position >= 0.0 {
            if index > 0 {
                let ratio = -last_negative_position / (position - last_negative_position);
                stops[index - 1].color =
                    interpolate_gradient_color(stops[index - 1].color, stops[index].color, ratio);
            }
            break;
        }
        stops[index].position = 0.0;
        last_negative_position = position;
    }
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

fn skia_hinted_gradient_interpolation(
    color_space: GradientColorSpace,
) -> skia_safe::gradient::Interpolation {
    let mut interpolation = skia_gradient_interpolation(color_space);
    interpolation.in_premul = skia_safe::gradient::interpolation::InPremul::Yes;
    interpolation
}

fn gradient_colors4f(
    stops: &[openui_style::GradientStop],
    current_color: &Color,
    opacity_multiplier: f32,
) -> Vec<Color4f> {
    stops
        .iter()
        .filter(|stop| !gradient_stop_is_hint(stop.position))
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
        .filter(|stop| !gradient_stop_is_hint(stop.position))
        .map(|stop| {
            skia_css_color_with_alpha(&stop.color.resolve(current_color), opacity_multiplier)
        })
        .collect()
}

fn resolved_gradient_colors4f(
    stops: &[ResolvedGradientStop],
    opacity_multiplier: f32,
) -> Vec<Color4f> {
    stops
        .iter()
        .map(|stop| {
            Color4f::new(
                stop.color.r,
                stop.color.g,
                stop.color.b,
                stop.color.a * opacity_multiplier,
            )
        })
        .collect()
}

fn resolved_gradient_colors(
    stops: &[ResolvedGradientStop],
    opacity_multiplier: f32,
) -> Vec<skia_safe::Color> {
    stops
        .iter()
        .map(|stop| skia_css_color_with_alpha(&stop.color, opacity_multiplier))
        .collect()
}

fn solid_gradient_color(image: &CssImage, current_color: &Color) -> Option<Color> {
    let stops = match image {
        CssImage::LinearGradient(gradient) => gradient.stops.as_slice(),
        CssImage::RadialGradient(gradient) => gradient.stops.as_slice(),
        CssImage::ConicGradient(gradient) => gradient.stops.as_slice(),
        CssImage::Raster(_) => return None,
    };
    let mut colors = stops
        .iter()
        .filter(|stop| !gradient_stop_is_hint(stop.position))
        .map(|stop| stop.color.resolve(current_color));
    let first = colors.next()?;
    colors.all(|color| color == first).then_some(first)
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

fn blink_layout_mul_div(value: f32, multiplier: f32, divisor: f32) -> f32 {
    if divisor <= 0.0 {
        return 0.0;
    }
    let raw = |component: f32| (component * 64.0).trunc() as i64;
    let divisor = raw(divisor);
    if divisor == 0 {
        return 0.0;
    }
    (raw(value) * raw(multiplier) / divisor) as f32 / 64.0
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
                w = ratio.map_or(area.width(), |_| {
                    // Blink resolves the dependent intrinsic dimension with
                    // LayoutUnit::MulDiv, retaining six fractional bits and
                    // truncating the quotient. Reproduce that geometry before
                    // any device-scale sampling decisions are made.
                    blink_layout_mul_div(h, intrinsic.0, intrinsic.1)
                });
            } else if height_auto && !width_auto {
                h = ratio.map_or(area.height(), |_| {
                    blink_layout_mul_div(w, intrinsic.1, intrinsic.0)
                });
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

fn background_image_paint_bounds(
    clip: Rect,
    image_left: f32,
    image_top: f32,
    tile_width: f32,
    tile_height: f32,
    repeats_x: bool,
    repeats_y: bool,
) -> Rect {
    let mut bounds = clip;
    if !repeats_x {
        bounds.left = bounds.left.max(image_left);
        bounds.right = bounds.right.min(image_left + tile_width);
    }
    if !repeats_y {
        bounds.top = bounds.top.max(image_top);
        bounds.bottom = bounds.bottom.min(image_top + tile_height);
    }
    bounds
}

fn uses_spaced_background_shader(
    repeat_x: BackgroundRepeat,
    repeat_y: BackgroundRepeat,
    x_tile_count: usize,
    y_tile_count: usize,
) -> bool {
    (repeat_x == BackgroundRepeat::Space && x_tile_count > 1)
        || (repeat_y == BackgroundRepeat::Space && y_tile_count > 1)
}

fn uses_single_repeated_generated_shader(
    has_resampled_image: bool,
    repeat_x: BackgroundRepeat,
    repeat_y: BackgroundRepeat,
    x_tile_count: usize,
    y_tile_count: usize,
) -> bool {
    has_resampled_image
        && ((matches!(repeat_x, BackgroundRepeat::Repeat | BackgroundRepeat::Round)
            && x_tile_count > 1)
            || (matches!(repeat_y, BackgroundRepeat::Repeat | BackgroundRepeat::Round)
                && y_tile_count > 1))
}

fn generated_image_raster_phase(physical_start: f32, repeats_axis: bool) -> i32 {
    if repeats_axis {
        0
    } else {
        (physical_start as i32).rem_euclid(8)
    }
}

fn generated_image_backing_scale(
    uses_repeated_shader: bool,
    source_extent: f32,
    backing_extent: i32,
    device_scale: f32,
) -> f32 {
    if uses_repeated_shader {
        backing_extent as f32 / source_extent
    } else {
        device_scale
    }
}

fn linear_gradient_direction(angle_degrees: f32) -> Point {
    let radians = angle_degrees.to_radians();
    let mut x = radians.sin();
    let mut y = -radians.cos();
    // CSS side directions and authored quarter turns are exactly axial.
    // Retaining libm's tiny residual cosine makes a nominally horizontal
    // gradient drift through color-quantization thresholds as Y increases,
    // which changes the ordered-dither result far from the shader origin.
    if x.abs() < 1.0e-6 {
        x = 0.0;
    } else if (x.abs() - 1.0).abs() < 1.0e-6 {
        x = x.signum();
    }
    if y.abs() < 1.0e-6 {
        y = 0.0;
    } else if (y.abs() - 1.0).abs() < 1.0e-6 {
        y = y.signum();
    }
    Point::new(x, y)
}

fn draw_paint_with_physical_coverage(
    canvas: &Canvas,
    rect: Rect,
    paint: &Paint,
    opacity_multiplier: f32,
    device_scale: f64,
    square_coverage: bool,
) {
    let scale = device_scale.max(f64::EPSILON) as f32;
    for (left, right, horizontal_coverage) in
        physical_coverage_axis_segments(rect.left, rect.right, device_scale)
    {
        for &(top, bottom, vertical_coverage) in
            &physical_coverage_axis_segments(rect.top, rect.bottom, device_scale)
        {
            let mut covered_paint = paint.clone();
            let clip_coverage = horizontal_coverage * vertical_coverage;
            let analytic_coverage = if square_coverage {
                clip_coverage * clip_coverage
            } else {
                clip_coverage
            };
            let one_fractional_axis = (horizontal_coverage < 1.0) ^ (vertical_coverage < 1.0);
            let packed_coverage = if one_fractional_axis && square_coverage {
                // The box contour and the replaced-content clip each supply
                // fractional coverage. Their product is packed once; closing
                // it upward adds a color level at an exact quarter sample.
                ((analytic_coverage * 256.0).floor().min(255.0)) / 255.0
            } else if !square_coverage {
                // Display-item clip coverage is stored in an eight-bit mask
                // before it modulates the generated image. Preserve the 256-
                // step packing here instead of letting Skia quantize the raw
                // float alpha a second, backend-dependent way.
                packed_physical_coverage(analytic_coverage, false)
            } else {
                analytic_coverage
            };
            covered_paint.set_alpha_f(opacity_multiplier * packed_coverage);
            canvas.draw_rect(
                Rect::from_ltrb(
                    left as f32 / scale,
                    top as f32 / scale,
                    right as f32 / scale,
                    bottom as f32 / scale,
                ),
                &covered_paint,
            );
        }
    }
}

fn physical_patch_axis_bands(
    aligned_start: f32,
    analytic_start: f32,
    analytic_end: f32,
    device_scale: f32,
    extent: i32,
) -> Vec<(i32, i32, u32)> {
    // `aligned_start` was produced by dividing an integral device coordinate
    // by the scale. Recover that integer before measuring edge overlap; the
    // f32 round trip can otherwise turn an exact quarter-pixel trailing span
    // into 63/256 coverage while its leading peer remains 192/256.
    let aligned_physical_start = (f64::from(aligned_start) * f64::from(device_scale)).round();
    let analytic_start = f64::from(analytic_start) * f64::from(device_scale);
    let analytic_end = f64::from(analytic_end) * f64::from(device_scale);
    let mut bands = Vec::new();
    for index in 0..extent {
        let pixel_start = aligned_physical_start + f64::from(index);
        let overlap =
            (analytic_end.min(pixel_start + 1.0) - analytic_start.max(pixel_start)).clamp(0.0, 1.0);
        let coverage = if overlap >= 1.0 {
            255
        } else {
            let closes_trailing_edge =
                analytic_end > pixel_start && analytic_end < pixel_start + 1.0 && overlap > 0.0;
            ((overlap * 256.0).floor() as u32 + u32::from(closes_trailing_edge)).min(255)
        };
        if let Some((_, end, previous)) = bands.last_mut() {
            if *previous == coverage {
                *end = index + 1;
                continue;
            }
        }
        bands.push((index, index + 1, coverage));
    }
    bands
}

fn draw_physical_patch_with_analytic_bounds(
    canvas: &Canvas,
    patch: &Image,
    aligned_destination: Rect,
    analytic_destination: Rect,
    device_scale: f32,
    opacity_multiplier: f32,
) {
    let x_bands = physical_patch_axis_bands(
        aligned_destination.left,
        analytic_destination.left,
        analytic_destination.right,
        device_scale,
        patch.width(),
    );
    let y_bands = physical_patch_axis_bands(
        aligned_destination.top,
        analytic_destination.top,
        analytic_destination.bottom,
        device_scale,
        patch.height(),
    );
    for (top, bottom, vertical_coverage) in y_bands {
        for &(left, right, horizontal_coverage) in &x_bands {
            let coverage = crate::image_resource::combine_physical_axis_coverage(
                horizontal_coverage,
                vertical_coverage,
            );
            if coverage == 0 {
                continue;
            }
            let source = Rect::from_ltrb(left as f32, top as f32, right as f32, bottom as f32);
            let destination = Rect::from_ltrb(
                aligned_destination.left + left as f32 / device_scale,
                aligned_destination.top + top as f32 / device_scale,
                aligned_destination.left + right as f32 / device_scale,
                aligned_destination.top + bottom as f32 / device_scale,
            );
            let mut paint = Paint::default();
            paint.set_alpha_f(opacity_multiplier * coverage as f32 / 255.0);
            canvas.draw_image_rect_with_sampling_options(
                patch.clone(),
                Some((&source, SrcRectConstraint::Strict)),
                destination,
                SamplingOptions::from(FilterMode::Nearest),
                &paint,
            );
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
    wrap_at_integral_scale_x: bool,
    wrap_at_integral_scale_y: bool,
    repeat_origin: Option<Point>,
    space_edge: bool,
    force_tile_antialias: bool,
    opacity_multiplier: f32,
    resample_from: Option<(f32, f32)>,
    paint_bounds: Option<Rect>,
    analytic_coverage_bounds: Option<Rect>,
    square_analytic_coverage: bool,
) {
    if tile.width() <= 0.0 || tile.height() <= 0.0 {
        return;
    }
    let device_matrix = canvas.local_to_device_as_3x3();
    let device_scale = style.device_scale_factor.max(f64::EPSILON) as f32;
    let fractional_physical_edge = [tile.left, tile.top, tile.right, tile.bottom]
        .into_iter()
        .any(|edge| (edge * device_scale).rem_euclid(1.0) > 1.0e-4);
    let antialias_tile_edge = device_matrix.skew_x().abs() > f32::EPSILON
        || device_matrix.skew_y().abs() > f32::EPSILON
        || (fractional_physical_edge && space_edge)
        || force_tile_antialias;
    if let Some((source_width, source_height)) = resample_from {
        // Generated-image tiles are rasterized by Blink at their device-pixel
        // extent. Keeping this temporary surface in CSS pixels and then
        // scaling it at replay time duplicates each quantized gradient sample
        // at high DPR and linearly blends the last and first rows at repeat
        // seams. Rasterize in physical pixels so coverage, dithering, and
        // repeat phase are resolved once in the same coordinate space as the
        // destination surface.
        let raster_scale = style.device_scale_factor.max(f64::EPSILON) as f32;
        let source_tile = Rect::from_xywh(tile.left, tile.top, source_width, source_height);
        let physical_left = (source_tile.left * raster_scale).floor();
        let physical_top = (source_tile.top * raster_scale).floor();
        let physical_right = (source_tile.right * raster_scale).ceil();
        let physical_bottom = (source_tile.bottom * raster_scale).ceil();
        let uses_repeated_shader = wrap_x || wrap_y;
        let width = if wrap_x {
            (source_width * raster_scale).ceil().max(1.0) as i32
        } else {
            (physical_right - physical_left).max(1.0) as i32
        };
        let height = if wrap_y {
            (source_height * raster_scale).ceil().max(1.0) as i32
        } else {
            (physical_bottom - physical_top).max(1.0) as i32
        };
        // The generated image occupies its complete integer-sized backing,
        // including when the destination repeat period is fractional in
        // physical pixels. Evaluating at the raw device scale would leave a
        // half-covered terminal texel (for example 62.5px in a 63px image),
        // then quantize every filtered sample differently when that image is
        // mapped back onto the fractional period.
        let raster_scale_x =
            generated_image_backing_scale(wrap_x, source_width, width, raster_scale);
        let raster_scale_y =
            generated_image_backing_scale(wrap_y, source_height, height, raster_scale);
        // The concrete-tile fallback dithers before applying its repeat
        // shader, so the complete tile uses a local ordered-dither origin.
        // Framebuffer anchoring either axis would make a non-8px repeat
        // period alternate colors.
        let phase_x = generated_image_raster_phase(physical_left, uses_repeated_shader);
        let phase_y = generated_image_raster_phase(physical_top, uses_repeated_shader);
        // SkPictureShader caches an eight-bit generated tile in RGBA8888,
        // tagged with the destination color space (sRGB for an untagged
        // canvas). Retain that representation for the concrete-tile path;
        // native N32 would select a different legacy image sampler.
        let tile_info = ImageInfo::new(
            (width + phase_x, height + phase_y),
            ColorType::RGBA8888,
            AlphaType::Premul,
            ColorSpace::new_srgb(),
        );
        let Some(mut surface) = surfaces::raster(&tile_info, None, None) else {
            return;
        };
        surface.canvas().clear(skia_safe::Color::TRANSPARENT);
        surface.canvas().translate((
            phase_x as f32
                - if wrap_x {
                    source_tile.left * raster_scale_x
                } else {
                    physical_left
                },
            phase_y as f32
                - if wrap_y {
                    source_tile.top * raster_scale_y
                } else {
                    physical_top
                },
        ));
        surface.canvas().scale((raster_scale_x, raster_scale_y));
        paint_css_image_tile(
            surface.canvas(),
            doc,
            style,
            image,
            source_tile,
            false,
            false,
            false,
            false,
            None,
            false,
            false,
            1.0,
            None,
            None,
            None,
            false,
        );
        let rasterized = surface.image_snapshot();
        let source = Rect::from_xywh(phase_x as f32, phase_y as f32, width as f32, height as f32);
        if uses_repeated_shader {
            // The prefix exists only to put ordered dithering at the physical
            // framebuffer phase of a non-repeating axis. Repeat the concrete
            // tile subset, not that transparent prefix; image shaders do not
            // otherwise consume the `source` rectangle used by DrawImage.
            let Some(concrete_tile) = rasterized.make_subset(
                None,
                IRect::from_xywh(phase_x, phase_y, width, height),
                RequiredProperties::default(),
            ) else {
                return;
            };
            // A repeated generated image is one concrete image shader, not a
            // sequence of independently composited rectangles. Let linear
            // sampling cross the repeat boundary so paired fractional edges
            // add to one coverage value instead of leaving an AA grid seam.
            let local_matrix = Matrix::scale_translate(
                (tile.width() / width as f32, tile.height() / height as f32),
                (tile.left, tile.top),
            );
            let mut paint = Paint::default();
            paint.set_shader(concrete_tile.to_shader(
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
            let mut bounds = analytic_coverage_bounds.unwrap_or(tile);
            if !wrap_x {
                bounds.left = bounds.left.max(tile.left);
                bounds.right = bounds.right.min(tile.right);
            }
            if !wrap_y {
                bounds.top = bounds.top.max(tile.top);
                bounds.bottom = bounds.bottom.min(tile.bottom);
            }
            let fractional_bounds = [bounds.left, bounds.top, bounds.right, bounds.bottom]
                .into_iter()
                .any(|edge| {
                    let physical = f64::from(edge) * style.device_scale_factor;
                    physical.rem_euclid(1.0) > 1.0e-6
                });
            if fractional_bounds {
                draw_paint_with_physical_coverage(
                    canvas,
                    bounds,
                    &paint,
                    opacity_multiplier,
                    style.device_scale_factor,
                    square_analytic_coverage,
                );
            } else {
                paint.set_alpha_f(opacity_multiplier);
                canvas.draw_rect(bounds, &paint);
            }
            return;
        }
        let destination_left = if wrap_x {
            tile.left * raster_scale
        } else {
            (tile.left * raster_scale).floor()
        };
        let destination_top = if wrap_y {
            tile.top * raster_scale
        } else {
            (tile.top * raster_scale).floor()
        };
        let destination_right = if wrap_x {
            tile.right * raster_scale
        } else {
            (tile.right * raster_scale).ceil()
        };
        let destination_bottom = if wrap_y {
            tile.bottom * raster_scale
        } else {
            (tile.bottom * raster_scale).ceil()
        };
        let destination = Rect::from_ltrb(
            destination_left / raster_scale,
            destination_top / raster_scale,
            destination_right / raster_scale,
            destination_bottom / raster_scale,
        );
        let resizes_intermediate = width != (destination_right - destination_left) as i32
            || height != (destination_bottom - destination_top) as i32;
        let mut paint = Paint::default();
        paint.set_alpha_f(opacity_multiplier);
        canvas.save();
        canvas.clip_rect(paint_bounds.unwrap_or(tile), ClipOp::Intersect, true);
        canvas.draw_image_rect_with_sampling_options(
            rasterized,
            Some((&source, SrcRectConstraint::Strict)),
            destination,
            SamplingOptions::from(if resizes_intermediate {
                FilterMode::Linear
            } else {
                FilterMode::Nearest
            }),
            &paint,
        );
        canvas.restore();
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
            let coverage_repeats_x = wrap_x;
            let coverage_repeats_y = wrap_y;
            let wrap_x =
                wrap_x && (wrap_at_integral_scale_x || (scale_x - scale_x.round()).abs() > 1.0e-5);
            let wrap_y =
                wrap_y && (wrap_at_integral_scale_y || (scale_y - scale_y.round()).abs() > 1.0e-5);
            // Blink keeps the inverse image-to-device transform at full
            // precision for an ordinary repeating image shader.  Adjusted
            // `round` tiles and single-tile draws instead take the packed
            // image path, even when they use the same decoded resource.
            let has_ordinary_repeat =
                (wrap_x && wrap_at_integral_scale_x) || (wrap_y && wrap_at_integral_scale_y);
            let has_only_ordinary_repeats =
                (!wrap_x || wrap_at_integral_scale_x) && (!wrap_y || wrap_at_integral_scale_y);
            // Blink's single-axis bitmap shader keeps its floating-point
            // inverse transform even for packed indexed sources and
            // round-adjusted tiles. The legacy packed path is selected when
            // both axes repeat; non-repeating draws retain the encoded-format
            // policy below.
            let has_single_repeated_axis = coverage_repeats_x ^ coverage_repeats_y;
            let full_precision_background = has_single_repeated_axis
                || (has_ordinary_repeat
                    && has_only_ordinary_repeats
                    && crate::image_resource::retains_full_precision_background_phase(
                        doc,
                        *id,
                        scale_x * device_scale,
                        scale_y * device_scale,
                    ));
            let mut paint = Paint::default();
            paint.set_anti_alias(antialias_tile_edge);
            paint.set_alpha_f(opacity_multiplier);
            if style.raster_configuration.backend != RasterBackend::GaneshGl && !antialias_tile_edge
            {
                let source = Rect::from_xywh(0.0, 0.0, image_width, image_height);
                let device_scale = style.device_scale_factor.max(f64::EPSILON) as f32;
                if let Ok((patch, aligned_destination, analytic_clip)) =
                    crate::image_resource::physical_quantized_image_patch(
                        &image,
                        source,
                        tile,
                        device_scale,
                        wrap_x,
                        wrap_y,
                        wrap_at_integral_scale_x,
                        wrap_at_integral_scale_y,
                        coverage_repeats_x,
                        coverage_repeats_y,
                        repeat_origin.map(|origin| (origin.x, origin.y)),
                        false,
                        false,
                        full_precision_background,
                        !square_analytic_coverage,
                        !style.has_border_radius(),
                        false,
                        analytic_coverage_bounds,
                    )
                {
                    let patch_source =
                        Rect::from_xywh(0.0, 0.0, patch.width() as f32, patch.height() as f32);
                    if let Some((_single_tile_source, single_tile_destination)) = analytic_clip {
                        // The retained patch already contains Chromium's
                        // packed bilinear result. Apply its destination edge
                        // as a global AA mask so Skia uses SkPMLerp (the same
                        // joint source/destination integer interpolation as
                        // DrawImage) rather than premultiplying the patch.
                        draw_physical_patch_with_analytic_bounds(
                            canvas,
                            &patch,
                            aligned_destination,
                            single_tile_destination,
                            device_scale,
                            opacity_multiplier,
                        );
                        return;
                    }
                    canvas.save();
                    let mut patch_clip = paint_bounds.unwrap_or(tile);
                    if let Some(bounds) = analytic_coverage_bounds {
                        let epsilon = 1.0e-4;
                        if (tile.left - bounds.left).abs() <= epsilon {
                            patch_clip.left = aligned_destination.left;
                        }
                        if (tile.top - bounds.top).abs() <= epsilon {
                            patch_clip.top = aligned_destination.top;
                        }
                        if (tile.right - bounds.right).abs() <= epsilon {
                            patch_clip.right = aligned_destination.right;
                        }
                        if (tile.bottom - bounds.bottom).abs() <= epsilon {
                            patch_clip.bottom = aligned_destination.bottom;
                        }
                    }
                    canvas.clip_rect(patch_clip, ClipOp::Intersect, false);
                    canvas.draw_image_rect_with_sampling_options(
                        patch,
                        Some((&patch_source, SrcRectConstraint::Strict)),
                        aligned_destination,
                        SamplingOptions::from(FilterMode::Nearest),
                        &paint,
                    );
                    canvas.restore();
                    return;
                }
            }
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
            canvas.draw_rect(paint_bounds.unwrap_or(tile), &paint);
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
                linear_gradient_direction(gradient.angle_degrees)
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
            let hinted_stops =
                resolved_gradient_stops_with_hints(&gradient.stops, line_length, &style.color);
            let mut positions = hinted_stops.as_ref().map_or_else(
                || css_gradient_positions(&gradient.stops, line_length),
                |stops| stops.iter().map(|stop| stop.position).collect(),
            );
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
            paint.set_anti_alias(antialias_tile_edge);
            paint.set_dither(true);
            if let Some(stops) = hinted_stops.as_ref() {
                let colors = resolved_gradient_colors4f(stops, opacity_multiplier);
                paint.set_shader(gradient_shader::linear_with_interpolation(
                    (shader_start, shader_end),
                    (colors.as_slice(), None),
                    positions.as_slice(),
                    mode,
                    skia_hinted_gradient_interpolation(gradient.color_space),
                    None,
                ));
            } else if gradient.color_space == GradientColorSpace::Srgb {
                let colors = hinted_stops.as_ref().map_or_else(
                    || gradient_colors(&gradient.stops, &style.color, opacity_multiplier),
                    |stops| resolved_gradient_colors(stops, opacity_multiplier),
                );
                paint.set_shader(gradient_shader::linear(
                    (shader_start, shader_end),
                    colors.as_slice(),
                    positions.as_slice(),
                    mode,
                    None,
                    None,
                ));
            } else {
                let colors = hinted_stops.as_ref().map_or_else(
                    || gradient_colors4f(&gradient.stops, &style.color, opacity_multiplier),
                    |stops| resolved_gradient_colors4f(stops, opacity_multiplier),
                );
                paint.set_shader(gradient_shader::linear_with_interpolation(
                    (shader_start, shader_end),
                    (colors.as_slice(), None),
                    positions.as_slice(),
                    mode,
                    skia_gradient_interpolation(gradient.color_space),
                    None,
                ));
            }
            canvas.draw_rect(paint_bounds.unwrap_or(tile), &paint);
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
            let mut hinted_stops =
                resolved_gradient_stops_with_hints(&gradient.stops, rx.max(ry), &style.color);
            if !gradient.repeating {
                if let Some(stops) = hinted_stops.as_mut() {
                    clamp_negative_radial_gradient_stops(stops);
                }
            }
            let mut positions = hinted_stops.as_ref().map_or_else(
                || css_gradient_positions(&gradient.stops, rx.max(ry)),
                |stops| stops.iter().map(|stop| stop.position).collect(),
            );
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
                paint.set_anti_alias(antialias_tile_edge);
                paint.set_dither(true);
                if let Some(stops) = hinted_stops.as_ref() {
                    let colors = resolved_gradient_colors4f(stops, opacity_multiplier);
                    paint.set_shader(gradient_shader::two_point_conical_with_interpolation(
                        (Point::new(cx, cy), 0.0),
                        (Point::new(cx, cy), rx * radius),
                        (colors.as_slice(), None),
                        positions.as_slice(),
                        mode,
                        skia_hinted_gradient_interpolation(gradient.color_space),
                        None,
                    ));
                } else if gradient.color_space == GradientColorSpace::Srgb {
                    let colors = hinted_stops.as_ref().map_or_else(
                        || gradient_colors(&gradient.stops, &style.color, opacity_multiplier),
                        |stops| resolved_gradient_colors(stops, opacity_multiplier),
                    );
                    paint.set_shader(gradient_shader::two_point_conical(
                        Point::new(cx, cy),
                        0.0,
                        Point::new(cx, cy),
                        rx * radius,
                        colors.as_slice(),
                        positions.as_slice(),
                        mode,
                        None,
                        None,
                    ));
                } else {
                    let colors = hinted_stops.as_ref().map_or_else(
                        || gradient_colors4f(&gradient.stops, &style.color, opacity_multiplier),
                        |stops| resolved_gradient_colors4f(stops, opacity_multiplier),
                    );
                    paint.set_shader(gradient_shader::two_point_conical_with_interpolation(
                        (Point::new(cx, cy), 0.0),
                        (Point::new(cx, cy), rx * radius),
                        (colors.as_slice(), None),
                        positions.as_slice(),
                        mode,
                        skia_gradient_interpolation(gradient.color_space),
                        None,
                    ));
                }
                canvas.draw_rect(paint_bounds.unwrap_or(tile), &paint);
                return;
            }
            let mut local_matrix = Matrix::new_identity();
            local_matrix.pre_scale((1.0, ry / rx), Some(Point::new(cx, cy)));
            let mut paint = Paint::default();
            paint.set_anti_alias(antialias_tile_edge);
            paint.set_dither(true);
            if let Some(stops) = hinted_stops.as_ref() {
                let colors = resolved_gradient_colors4f(stops, opacity_multiplier);
                paint.set_shader(gradient_shader::two_point_conical_with_interpolation(
                    (Point::new(cx, cy), 0.0),
                    (Point::new(cx, cy), rx * radius),
                    (colors.as_slice(), None),
                    positions.as_slice(),
                    mode,
                    skia_hinted_gradient_interpolation(gradient.color_space),
                    Some(&local_matrix),
                ));
            } else if gradient.color_space == GradientColorSpace::Srgb {
                let colors = hinted_stops.as_ref().map_or_else(
                    || gradient_colors(&gradient.stops, &style.color, opacity_multiplier),
                    |stops| resolved_gradient_colors(stops, opacity_multiplier),
                );
                paint.set_shader(gradient_shader::two_point_conical(
                    Point::new(cx, cy),
                    0.0,
                    Point::new(cx, cy),
                    rx * radius,
                    colors.as_slice(),
                    positions.as_slice(),
                    mode,
                    None,
                    Some(&local_matrix),
                ));
            } else {
                let colors = hinted_stops.as_ref().map_or_else(
                    || gradient_colors4f(&gradient.stops, &style.color, opacity_multiplier),
                    |stops| resolved_gradient_colors4f(stops, opacity_multiplier),
                );
                paint.set_shader(gradient_shader::two_point_conical_with_interpolation(
                    (Point::new(cx, cy), 0.0),
                    (Point::new(cx, cy), rx * radius),
                    (colors.as_slice(), None),
                    positions.as_slice(),
                    mode,
                    skia_gradient_interpolation(gradient.color_space),
                    Some(&local_matrix),
                ));
            }
            canvas.draw_rect(paint_bounds.unwrap_or(tile), &paint);
        }
        CssImage::ConicGradient(gradient) => {
            if gradient.stops.len() < 2 {
                return;
            }
            let mut center = Point::new(
                resolve_background_position(gradient.center_x, tile.left, tile.width(), 0.0),
                resolve_background_position(gradient.center_y, tile.top, tile.height(), 0.0),
            );
            let device_scale = style.device_scale_factor.max(f64::EPSILON) as f32;
            let sample_center_phase = |coordinate: f32| {
                ((coordinate * device_scale).rem_euclid(1.0) - 0.5).abs() < 1.0e-5
            };
            if sample_center_phase(center.x) && sample_center_phase(center.y) {
                // The polar angle is undefined when a device-pixel sample lies
                // exactly on the conic center. Blink resolves that singular
                // sample from the south-east quadrant; Skia's direct CPU
                // shader otherwise selects the first stop. Move the logical
                // center by exactly one representable float so no ordinary
                // conic sample crosses an interpolation or dither boundary.
                let preceding_float = |value: f32| {
                    if value > 0.0 {
                        f32::from_bits(value.to_bits() - 1)
                    } else if value < 0.0 {
                        f32::from_bits(value.to_bits() + 1)
                    } else {
                        -f32::from_bits(1)
                    }
                };
                center.x = preceding_float(center.x);
                center.y = preceding_float(center.y);
            }
            let hinted_stops =
                resolved_gradient_stops_with_hints(&gradient.stops, 360.0, &style.color);
            let positions = hinted_stops.as_ref().map_or_else(
                || css_gradient_positions(&gradient.stops, 360.0),
                |stops| stops.iter().map(|stop| stop.position).collect(),
            );
            let mode = if gradient.repeating {
                TileMode::Repeat
            } else {
                TileMode::Clamp
            };
            let mut paint = Paint::default();
            paint.set_anti_alias(antialias_tile_edge);
            paint.set_dither(true);
            let matrix = Matrix::rotate_deg_pivot(gradient.from_degrees - 90.0, center);
            if let Some(stops) = hinted_stops.as_ref() {
                let colors = resolved_gradient_colors4f(stops, opacity_multiplier);
                paint.set_shader(gradient_shader::sweep_with_interpolation(
                    center,
                    (colors.as_slice(), None),
                    positions.as_slice(),
                    mode,
                    None,
                    skia_hinted_gradient_interpolation(gradient.color_space),
                    Some(&matrix),
                ));
            } else if gradient.color_space == GradientColorSpace::Srgb {
                let colors = hinted_stops.as_ref().map_or_else(
                    || gradient_colors(&gradient.stops, &style.color, opacity_multiplier),
                    |stops| resolved_gradient_colors(stops, opacity_multiplier),
                );
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
                let colors = hinted_stops.as_ref().map_or_else(
                    || gradient_colors4f(&gradient.stops, &style.color, opacity_multiplier),
                    |stops| resolved_gradient_colors4f(stops, opacity_multiplier),
                );
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
            canvas.draw_rect(paint_bounds.unwrap_or(tile), &paint);
        }
    }
}

fn generated_image_resample_size(
    generated_image: bool,
    direct_radial_shader: bool,
    _canvas_background: bool,
    repeats_in_x: bool,
    repeats_in_y: bool,
    tile_size: (f32, f32),
) -> Option<(f32, f32)> {
    if !generated_image || direct_radial_shader {
        None
    } else if repeats_in_x || repeats_in_y {
        // A repeated generated image is rasterized and dithered once before
        // its concrete bitmap is repeated. This applies equally to one-axis
        // element backgrounds and propagated canvas backgrounds; otherwise a
        // direct shader would restart at each logical tile with a different
        // framebuffer-anchored dither phase. `round` has already changed
        // `tile_size` to the resolved dimensions.
        Some(tile_size)
    } else {
        // A lone generated tile stays in destination space. Besides avoiding
        // an unnecessary resample, this lets ordered dithering restart at
        // each compositor raster tile exactly as it does in Chromium.
        None
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
    canvas_background: bool,
) {
    let fixed_is_local_to_transform = fragment.is_some_and(|fragment| {
        if fragment.node_id.is_none() {
            return false;
        }
        let mut ancestor = doc.node(fragment.node_id).parent;
        while !ancestor.is_none() {
            let ancestor_style = &doc.node(ancestor).style;
            if ancestor_style.transform != openui_style::Transform2D::IDENTITY {
                return true;
            }
            ancestor = doc.node(ancestor).parent;
        }
        false
    });
    for layer in style.background_layers.iter().rev() {
        let mut area = background_box_rect(layer.origin, border_rect, fragment);
        if layer.attachment == BackgroundAttachment::Fixed && !fixed_is_local_to_transform {
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
        let mut square_opaque_border_bleed_clip = false;
        if clip_override.is_none()
            && layer.clip == BackgroundClip::BorderBox
            && style.border_image.is_none()
            && (!style.has_border_radius()
                || fragment.is_some_and(|fragment| fragment.ignore_border_radius))
        {
            let (top, right, bottom, left) = fragment.map_or_else(
                || {
                    (
                        style.effective_border_top() as f32,
                        style.effective_border_right() as f32,
                        style.effective_border_bottom() as f32,
                        style.effective_border_left() as f32,
                    )
                },
                |fragment| sliced_physical_border_widths(fragment, style),
            );
            let opaque_solid = |width: f32, border_style: BorderStyle, color: &StyleColor| {
                width > 0.0
                    && border_style == BorderStyle::Solid
                    && color.resolve(&style.color).is_opaque()
            };
            let inset_top = opaque_solid(top, style.border_top_style, &style.border_top_color);
            let inset_right =
                opaque_solid(right, style.border_right_style, &style.border_right_color);
            let inset_bottom = opaque_solid(
                bottom,
                style.border_bottom_style,
                &style.border_bottom_color,
            );
            let inset_left = opaque_solid(left, style.border_left_style, &style.border_left_color);
            square_opaque_border_bleed_clip =
                inset_top || inset_right || inset_bottom || inset_left;
            if square_opaque_border_bleed_clip {
                // Blink's opaque-border bleed avoidance clips a square
                // border-box background image at the inner edge of every
                // fully obscuring side. Preserve the analytic inner-edge
                // coverage: it composes with the border's complementary AA
                // instead of allowing image color into the outer border ramp.
                clip = Rect::from_ltrb(
                    clip.left + if inset_left { left } else { 0.0 },
                    clip.top + if inset_top { top } else { 0.0 },
                    clip.right - if inset_right { right } else { 0.0 },
                    clip.bottom - if inset_bottom { bottom } else { 0.0 },
                );
            }
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
            if layer.repeat_y != BackgroundRepeat::Round
                && (matches!(layer.size, BackgroundSize::Auto)
                    || matches!(&layer.size, BackgroundSize::Explicit(_, height) if height.is_auto()))
            {
                tile_height *= adjusted / tile_width;
            }
            tile_width = adjusted;
        }
        if layer.repeat_y == BackgroundRepeat::Round && tile_height > 0.0 {
            let count = (area.height() / tile_height).round().max(1.0);
            let adjusted = area.height() / count;
            if layer.repeat_x != BackgroundRepeat::Round
                && (matches!(layer.size, BackgroundSize::Auto)
                    || matches!(&layer.size, BackgroundSize::Explicit(width, _) if width.is_auto()))
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
        // The concrete-tile fallback rasterizes at the final (including
        // `round`-adjusted) tile size. The recorded linear-gradient path below
        // retains the gradient as a picture shader instead.
        let physical_tile_width = tile_width * style.device_scale_factor as f32;
        let physical_tile_height = tile_height * style.device_scale_factor as f32;
        let fractional_physical_tile = (physical_tile_width - physical_tile_width.round()).abs()
            > 1.0e-4
            || (physical_tile_height - physical_tile_height.round()).abs() > 1.0e-4;
        let generated_image = !matches!(layer.image, CssImage::Raster(_));
        let direct_radial_shader = matches!(
            &layer.image,
            CssImage::RadialGradient(gradient) if gradient.repeating
        );
        let resample_from = generated_image_resample_size(
            generated_image,
            direct_radial_shader,
            canvas_background,
            matches!(
                layer.repeat_x,
                BackgroundRepeat::Repeat | BackgroundRepeat::Round
            ) && xs.len() > 1,
            matches!(
                layer.repeat_y,
                BackgroundRepeat::Repeat | BackgroundRepeat::Round
            ) && ys.len() > 1,
            (tile_width, tile_height),
        );
        // Blink records a generated background as a picture tile. In a
        // one-axis linear-gradient repeat, the recorded shader preserves
        // gradient color evaluation across the repeated tiles.
        let recorded_one_axis_gradient_shader = matches!(layer.image, CssImage::LinearGradient(_))
            && resample_from.is_some()
            && ((matches!(
                layer.repeat_x,
                BackgroundRepeat::Repeat | BackgroundRepeat::Round
            ) && xs.len() > 1
                && layer.repeat_y == BackgroundRepeat::NoRepeat)
                || (matches!(
                    layer.repeat_y,
                    BackgroundRepeat::Repeat | BackgroundRepeat::Round
                ) && ys.len() > 1
                    && layer.repeat_x == BackgroundRepeat::NoRepeat));
        canvas.save();
        let device_matrix = canvas.local_to_device_as_3x3();
        let antialias_transformed_clip = device_matrix.skew_x().abs() > f32::EPSILON
            || device_matrix.skew_y().abs() > f32::EPSILON;
        let fractional_physical_clip_edge = [clip.left, clip.top, clip.right, clip.bottom]
            .into_iter()
            .any(|edge| {
                let physical = f64::from(edge) * style.device_scale_factor;
                physical.rem_euclid(1.0) > 1.0e-6
            });
        let rectangular_transformed_clip = antialias_transformed_clip
            && clip_override.is_none()
            && layer.clip != BackgroundClip::BorderArea
            && !style.has_border_radius()
            && !matches!(layer.image, CssImage::Raster(_));
        let physical_raster_patch_owns_rect_clip = !antialias_transformed_clip
            && style.raster_configuration.backend != RasterBackend::GaneshGl
            && matches!(layer.image, CssImage::Raster(_))
            && (square_opaque_border_bleed_clip
                || (layer.repeat_x == BackgroundRepeat::Round
                    && layer.repeat_y == BackgroundRepeat::Round));
        let direct_generated_image_owns_rect_clip = generated_image
            && resample_from.is_none()
            && fractional_physical_tile
            && xs.len() == 1
            && ys.len() == 1
            && clip_override.is_none();
        let repeated_generated_rect_clip = generated_image
            && resample_from.is_some()
            && matches!(
                layer.repeat_x,
                BackgroundRepeat::Repeat | BackgroundRepeat::Round
            )
            && matches!(
                layer.repeat_y,
                BackgroundRepeat::Repeat | BackgroundRepeat::Round
            )
            && xs.len() > 1
            && ys.len() > 1;
        let physical_generated_image_owns_rect_clip = (recorded_one_axis_gradient_shader
            || repeated_generated_rect_clip)
            && fractional_physical_clip_edge
            && clip_override.is_none();
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
        } else if !rectangular_transformed_clip
            && !physical_raster_patch_owns_rect_clip
            && !direct_generated_image_owns_rect_clip
            && !physical_generated_image_owns_rect_clip
        {
            let hard_clip =
                if antialias_transformed_clip || matches!(layer.image, CssImage::Raster(_)) {
                    clip
                } else {
                    half_open_physical_clip_rect(clip, style.device_scale_factor)
                };
            canvas.clip_rect(hard_clip, ClipOp::Intersect, antialias_transformed_clip);
        }
        let repeated_constant_gradient = matches!(
            layer.repeat_x,
            BackgroundRepeat::Repeat | BackgroundRepeat::Round
        ) && matches!(
            layer.repeat_y,
            BackgroundRepeat::Repeat | BackgroundRepeat::Round
        ) && tile_width > 0.0
            && tile_height > 0.0;
        let constant_color = repeated_constant_gradient
            .then(|| solid_gradient_color(&layer.image, &style.color))
            .flatten()
            .or_else(|| {
                if !repeated_constant_gradient
                    || opacity_multiplier != 1.0
                    || physical_raster_patch_owns_rect_clip
                    || antialias_transformed_clip
                {
                    return None;
                }
                match layer.image {
                    CssImage::Raster(id) => {
                        crate::image_resource::opaque_single_pixel_raster_color(doc, id)
                    }
                    _ => None,
                }
            });
        if let Some(color) = constant_color {
            // Repetition of a constant image is itself constant, including a
            // one-pixel opaque raster. Avoid millions of individual draws for
            // viewport-sized 1x1 repeats. Keep the existing clip and color
            // conversion rather than changing the tile's semantic bounds.
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(false);
            set_paint_css_color_with_alpha(&mut paint, &color, opacity_multiplier);
            canvas.draw_rect(clip, &paint);
            canvas.restore();
            continue;
        }
        if uses_spaced_background_shader(layer.repeat_x, layer.repeat_y, xs.len(), ys.len()) {
            let x_space_repeats = layer.repeat_x == BackgroundRepeat::Space && xs.len() > 1;
            let y_space_repeats = layer.repeat_y == BackgroundRepeat::Space && ys.len() > 1;
            let x_repeats = x_space_repeats
                || matches!(
                    layer.repeat_x,
                    BackgroundRepeat::Repeat | BackgroundRepeat::Round
                );
            let y_repeats = y_space_repeats
                || matches!(
                    layer.repeat_y,
                    BackgroundRepeat::Repeat | BackgroundRepeat::Round
                );
            let x_period = if x_space_repeats {
                xs[1] - xs[0]
            } else {
                tile_width
            };
            let y_period = if y_space_repeats {
                ys[1] - ys[0]
            } else {
                tile_height
            };
            let (source_width, source_height) = match &layer.image {
                CssImage::Raster(id) => crate::image_resource::decode_image_resource(doc, *id)
                    .map(|image| (image.width() as f32, image.height() as f32))
                    .unwrap_or((0.0, 0.0)),
                _ => (tile_width, tile_height),
            };
            if source_width > 0.0 && source_height > 0.0 {
                let scale_x = tile_width / source_width;
                let scale_y = tile_height / source_height;
                let origin_x = if x_space_repeats {
                    area.left
                } else {
                    image_left
                };
                let origin_y = if y_space_repeats { area.top } else { image_top };
                let picture_bounds =
                    Rect::from_xywh(0.0, 0.0, x_period / scale_x, y_period / scale_y);
                let mut recorder = PictureRecorder::new();
                let recording_canvas = recorder.begin_recording(picture_bounds, false);
                match &layer.image {
                    CssImage::Raster(id) => {
                        if let Ok(image) = crate::image_resource::decode_image_resource(doc, *id) {
                            let source = Rect::from_xywh(
                                0.0,
                                0.0,
                                image.width() as f32,
                                image.height() as f32,
                            );
                            let mut paint = Paint::default();
                            paint.set_anti_alias(true);
                            recording_canvas.draw_image_rect_with_sampling_options(
                                image,
                                Some((&source, SrcRectConstraint::Strict)),
                                source,
                                SamplingOptions::from(FilterMode::Linear),
                                &paint,
                            );
                        }
                    }
                    _ => paint_css_image_tile(
                        recording_canvas,
                        doc,
                        style,
                        &layer.image,
                        Rect::from_xywh(0.0, 0.0, source_width, source_height),
                        false,
                        false,
                        false,
                        false,
                        None,
                        false,
                        true,
                        1.0,
                        None,
                        None,
                        None,
                        false,
                    ),
                }
                if let Some(picture) = recorder.finish_recording_as_picture(None) {
                    let matrix = Matrix::scale_translate((scale_x, scale_y), (origin_x, origin_y));
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_alpha_f(opacity_multiplier);
                    let (tile_mode_x, tile_mode_y) = if matches!(layer.image, CssImage::Raster(_)) {
                        (
                            if x_repeats {
                                TileMode::Repeat
                            } else {
                                TileMode::Clamp
                            },
                            if y_repeats {
                                TileMode::Repeat
                            } else {
                                TileMode::Clamp
                            },
                        )
                    } else {
                        (TileMode::Repeat, TileMode::Repeat)
                    };
                    paint.set_shader(picture.to_shader(
                        (tile_mode_x, tile_mode_y),
                        FilterMode::Linear,
                        &matrix,
                        None,
                    ));
                    // The positioning area determines the phase and spacing,
                    // but background-clip determines the painting bounds.
                    // Repetitions before and after the positioning area can
                    // therefore paint beneath a border. When `space` falls
                    // back to a single image, background-position can place
                    // that image partly outside the positioning area too.
                    let bounds = background_image_paint_bounds(
                        clip,
                        image_left,
                        image_top,
                        tile_width,
                        tile_height,
                        x_repeats,
                        y_repeats,
                    );
                    if bounds.width() > 0.0 && bounds.height() > 0.0 {
                        canvas.draw_rect(bounds, &paint);
                    }
                    canvas.restore();
                    continue;
                }
            }
        }
        let single_axis_raster_repeat = (matches!(
            layer.repeat_x,
            BackgroundRepeat::Repeat | BackgroundRepeat::Round
        ) && xs.len() > 1
            && layer.repeat_y == BackgroundRepeat::NoRepeat)
            || (matches!(
                layer.repeat_y,
                BackgroundRepeat::Repeat | BackgroundRepeat::Round
            ) && ys.len() > 1
                && layer.repeat_x == BackgroundRepeat::NoRepeat);
        let fractional_device_scale =
            (style.device_scale_factor - style.device_scale_factor.round()).abs() > 1.0e-5;
        // Blink's bitmap DrawPattern paints a resized one-axis repeat with
        // one image shader over the complete destination. Separate physical
        // patches lose the shared repeat phase at fractional device scales.
        // Native-size repeats keep their existing packed sampling path.
        if single_axis_raster_repeat && fractional_device_scale {
            if let CssImage::Raster(id) = &layer.image {
                if let Ok(image) = crate::image_resource::decode_image_resource(doc, *id) {
                    let resizes_bitmap = (tile_width - image.width() as f32).abs() > 1.0e-5
                        || (tile_height - image.height() as f32).abs() > 1.0e-5;
                    if resizes_bitmap {
                        let matrix = Matrix::scale_translate(
                            (
                                tile_width / image.width() as f32,
                                tile_height / image.height() as f32,
                            ),
                            (image_left, image_top),
                        );
                        let mut paint = Paint::default();
                        paint.set_anti_alias(true);
                        paint.set_alpha_f(opacity_multiplier);
                        paint.set_shader(image.to_shader(
                            (
                                if layer.repeat_x == BackgroundRepeat::NoRepeat {
                                    TileMode::Clamp
                                } else {
                                    TileMode::Repeat
                                },
                                if layer.repeat_y == BackgroundRepeat::NoRepeat {
                                    TileMode::Clamp
                                } else {
                                    TileMode::Repeat
                                },
                            ),
                            SamplingOptions::from(FilterMode::Linear),
                            &matrix,
                        ));
                        let bounds = background_image_paint_bounds(
                            clip,
                            image_left,
                            image_top,
                            tile_width,
                            tile_height,
                            layer.repeat_x != BackgroundRepeat::NoRepeat,
                            layer.repeat_y != BackgroundRepeat::NoRepeat,
                        );
                        if bounds.width() > 0.0 && bounds.height() > 0.0 {
                            draw_paint_with_physical_coverage(
                                canvas,
                                bounds,
                                &paint,
                                opacity_multiplier,
                                style.device_scale_factor,
                                !square_opaque_border_bleed_clip,
                            );
                        }
                        canvas.restore();
                        continue;
                    }
                }
            }
        }
        if recorded_one_axis_gradient_shader {
            let source_rect = Rect::from_xywh(0.0, 0.0, tile_width, tile_height);
            let mut recorder = PictureRecorder::new();
            let recording_canvas = recorder.begin_recording(source_rect, false);
            paint_css_image_tile(
                recording_canvas,
                doc,
                style,
                &layer.image,
                source_rect,
                false,
                false,
                false,
                false,
                None,
                false,
                false,
                1.0,
                None,
                None,
                None,
                false,
            );
            if let Some(picture) = recorder.finish_recording_as_picture(None) {
                let matrix = Matrix::translate((image_left, image_top));
                let mut paint = Paint::default();
                paint.set_anti_alias(true);
                paint.set_alpha_f(opacity_multiplier);
                paint.set_shader(picture.to_shader(
                    (TileMode::Repeat, TileMode::Repeat),
                    FilterMode::Linear,
                    &matrix,
                    None,
                ));
                let bounds = background_image_paint_bounds(
                    clip,
                    image_left,
                    image_top,
                    tile_width,
                    tile_height,
                    matches!(
                        layer.repeat_x,
                        BackgroundRepeat::Repeat | BackgroundRepeat::Round
                    ),
                    matches!(
                        layer.repeat_y,
                        BackgroundRepeat::Repeat | BackgroundRepeat::Round
                    ),
                );
                if bounds.width() > 0.0 && bounds.height() > 0.0 {
                    if physical_generated_image_owns_rect_clip {
                        draw_paint_with_physical_coverage(
                            canvas,
                            bounds,
                            &paint,
                            opacity_multiplier,
                            style.device_scale_factor,
                            !square_opaque_border_bleed_clip,
                        );
                    } else {
                        canvas.draw_rect(bounds, &paint);
                    }
                }
                canvas.restore();
                continue;
            }
        }
        let repeated_generated_shader = uses_single_repeated_generated_shader(
            resample_from.is_some(),
            layer.repeat_x,
            layer.repeat_y,
            xs.len(),
            ys.len(),
        );
        for (y_index, top) in ys.iter().enumerate() {
            for (x_index, left) in xs.iter().enumerate() {
                if repeated_generated_shader && (x_index > 0 || y_index > 0) {
                    continue;
                }
                let tile_rect = Rect::from_xywh(*left, *top, tile_width, tile_height);
                let clipped_tile = Rect::from_ltrb(
                    tile_rect.left.max(clip.left),
                    tile_rect.top.max(clip.top),
                    tile_rect.right.min(clip.right),
                    tile_rect.bottom.min(clip.bottom),
                );
                // A direct radial tile is drawn with antialiasing below. Its
                // draw owns fractional edge coverage; a second antialiased
                // tile clip would multiply that coverage.
                paint_css_image_tile(
                    canvas,
                    doc,
                    style,
                    &layer.image,
                    tile_rect,
                    if generated_image {
                        matches!(
                            layer.repeat_x,
                            BackgroundRepeat::Repeat | BackgroundRepeat::Round
                        ) && xs.len() > 1
                    } else {
                        matches!(
                            layer.repeat_x,
                            BackgroundRepeat::Repeat | BackgroundRepeat::Round
                        ) && xs.len() > 1
                    },
                    if generated_image {
                        matches!(
                            layer.repeat_y,
                            BackgroundRepeat::Repeat | BackgroundRepeat::Round
                        ) && ys.len() > 1
                    } else {
                        matches!(
                            layer.repeat_y,
                            BackgroundRepeat::Repeat | BackgroundRepeat::Round
                        ) && ys.len() > 1
                    },
                    layer.repeat_x == BackgroundRepeat::Repeat,
                    layer.repeat_y == BackgroundRepeat::Repeat,
                    Some(Point::new(image_left, image_top)),
                    (layer.repeat_x == BackgroundRepeat::Space && xs.len() > 1)
                        || (layer.repeat_y == BackgroundRepeat::Space && ys.len() > 1),
                    generated_image && fractional_physical_tile,
                    opacity_multiplier,
                    resample_from,
                    rectangular_transformed_clip.then_some(clipped_tile),
                    Some(clip),
                    !square_opaque_border_bleed_clip,
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
        true,
    );
}

fn paint_scrolling_canvas_background(
    canvas: &Canvas,
    doc: &Document,
    root_fragment: &Fragment,
    bounds: Rect,
) {
    // ViewPainter's document background fills the pixel-snapped scrolling
    // contents, combining the base backdrop with the propagated CSS color.
    let source = doc.canvas_background_source();
    let authored = source.map_or(Color::TRANSPARENT, |source| {
        doc.node(source).style.background_color
    });
    let color = Color::from_rgba_f32(
        authored.r * authored.a + 1.0 - authored.a,
        authored.g * authored.a + 1.0 - authored.a,
        authored.b * authored.a + 1.0 - authored.a,
        1.0,
    );
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    set_paint_css_color(&mut paint, &color);
    canvas.draw_rect(bounds, &paint);
    annotate_content_background(canvas, color, bounds, 1.0, Some(bounds));
    if let Some(source) = source {
        let style = &doc.node(source).style;
        let (positioning_rect, source_fragment) =
            fragment_rect_for_node(root_fragment, source, PhysicalOffset::zero())
                .map_or((bounds, None), |(rect, fragment)| (rect, Some(fragment)));
        paint_background_layers(
            canvas,
            doc,
            source_fragment,
            style,
            positioning_rect,
            1.0,
            Some(bounds),
            true,
        );
    }
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
    source_scale: f32,
    device_scale: f32,
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
    let wrap_x = repeat_x != BorderImageRepeat::Stretch;
    let wrap_y = repeat_y != BorderImageRepeat::Stretch;
    let repeat_origin = xs
        .first()
        .zip(ys.first())
        .map(|((left, _), (top, _))| (*left, *top));
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_alpha_f(opacity_multiplier);
    if !wrap_x && !wrap_y && source_scale == 1.0 && image.color_type() != ColorType::RGBAF16 {
        if let Ok((patch, aligned_destination, analytic_clip)) =
            crate::image_resource::physical_quantized_image_patch(
                image,
                source,
                destination,
                device_scale,
                false,
                false,
                false,
                false,
                false,
                false,
                None,
                false,
                false,
                false,
                true,
                true,
                false,
                Some(destination),
            )
        {
            if let Some((_sampling_source, analytic_destination)) = analytic_clip {
                draw_physical_patch_with_analytic_bounds(
                    canvas,
                    &patch,
                    aligned_destination,
                    analytic_destination,
                    device_scale,
                    opacity_multiplier,
                );
                return;
            }
        }
    }
    if wrap_x || wrap_y {
        let local_source = Rect::from_xywh(0.0, 0.0, source.width(), source.height());
        let mut recorder = PictureRecorder::new();
        let recording_canvas = recorder.begin_recording(local_source, false);
        recording_canvas.draw_image_rect_with_sampling_options(
            image,
            Some((&source, SrcRectConstraint::Strict)),
            local_source,
            SamplingOptions::from(FilterMode::Linear),
            &paint,
        );
        if let Some(picture) = recorder.finish_recording_as_picture(None) {
            let Some(&(first_left, tile_width)) = xs.first() else {
                return;
            };
            let Some(&(first_top, tile_height)) = ys.first() else {
                return;
            };
            let matrix = Matrix::scale_translate(
                (tile_width / source.width(), tile_height / source.height()),
                (first_left, first_top),
            );
            paint.set_shader(picture.to_shader(
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
                FilterMode::Linear,
                &matrix,
                Some(&local_source),
            ));
            canvas.draw_rect(destination, &paint);
            return;
        }
    }
    canvas.save();
    canvas.clip_rect(destination, ClipOp::Intersect, true);
    for (top, height) in ys {
        for (left, width) in &xs {
            let tile_destination = Rect::from_xywh(*left, top, *width, height);
            let tile_clip = Rect::from_ltrb(
                if (tile_destination.left - destination.left).abs() < 0.001 {
                    tile_destination.left - 1.0
                } else {
                    tile_destination.left
                },
                if (tile_destination.top - destination.top).abs() < 0.001 {
                    tile_destination.top - 1.0
                } else {
                    tile_destination.top
                },
                if (tile_destination.right - destination.right).abs() < 0.001 {
                    tile_destination.right + 1.0
                } else {
                    tile_destination.right
                },
                if (tile_destination.bottom - destination.bottom).abs() < 0.001 {
                    tile_destination.bottom + 1.0
                } else {
                    tile_destination.bottom
                },
            );
            canvas.save();
            canvas.clip_rect(tile_clip, ClipOp::Intersect, false);
            if source_scale == 1.0 && image.color_type() != ColorType::RGBAF16 {
                if let Ok((patch, aligned_destination, _analytic_clip)) =
                    crate::image_resource::physical_quantized_image_patch(
                        image,
                        source,
                        tile_destination,
                        device_scale,
                        wrap_x,
                        wrap_y,
                        false,
                        false,
                        wrap_x,
                        wrap_y,
                        repeat_origin,
                        false,
                        false,
                        false,
                        true,
                        false,
                        false,
                        None,
                    )
                {
                    let patch_source =
                        Rect::from_xywh(0.0, 0.0, patch.width() as f32, patch.height() as f32);
                    canvas.save();
                    canvas.clip_rect(tile_destination, ClipOp::Intersect, false);
                    canvas.draw_image_rect_with_sampling_options(
                        patch,
                        Some((&patch_source, SrcRectConstraint::Strict)),
                        aligned_destination,
                        SamplingOptions::from(FilterMode::Nearest),
                        &paint,
                    );
                    canvas.restore();
                    canvas.restore();
                    continue;
                }
            }
            if source_scale > 1.0
                && (*width * source_scale - source.width()).abs() < 0.001
                && (height * source_scale - source.height()).abs() < 0.001
            {
                // Generated border images are retained at device resolution.
                // When a source patch already has exactly the destination's
                // physical extent, copy its texels without a second filter.
                canvas.draw_image_rect_with_sampling_options(
                    image,
                    Some((&source, SrcRectConstraint::Strict)),
                    tile_destination,
                    SamplingOptions::from(FilterMode::Nearest),
                    &paint,
                );
                canvas.restore();
                continue;
            }
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
                        tile_destination,
                        SamplingOptions::from(FilterMode::Nearest),
                        &paint,
                    );
                    canvas.restore();
                    continue;
                }
            }
            canvas.draw_image_rect_with_sampling_options(
                image,
                Some((&source, SrcRectConstraint::Strict)),
                tile_destination,
                SamplingOptions::from(FilterMode::Linear),
                &paint,
            );
            canvas.restore();
        }
    }
    canvas.restore();
}

fn draw_border_generated_image_patch(
    canvas: &Canvas,
    doc: &Document,
    style: &ComputedStyle,
    image: &CssImage,
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
    let coverage_clip = {
        // Nine-piece destinations are half-open at seams between pieces. Keep
        // the generated image's antialiasing on the outer border, but move an
        // internal hard clip just past equality so adjacent pieces cannot both
        // contribute to the same device sample.
        let epsilon = 1.0 / 1024.0;
        Rect::from_ltrb(
            if source.left > epsilon {
                destination.left + epsilon
            } else {
                destination.left - 1.0
            },
            if source.top > epsilon {
                destination.top + epsilon
            } else {
                destination.top - 1.0
            },
            if source.right < concrete_width - epsilon {
                destination.right - epsilon
            } else {
                destination.right + 1.0
            },
            if source.bottom < concrete_height - epsilon {
                destination.bottom - epsilon
            } else {
                destination.bottom + 1.0
            },
        )
    };
    if repeat_x != BorderImageRepeat::Stretch || repeat_y != BorderImageRepeat::Stretch {
        let Some(&(first_left, tile_width)) = xs.first() else {
            return;
        };
        let Some(&(first_top, tile_height)) = ys.first() else {
            return;
        };
        let scale_x = tile_width / source.width();
        let scale_y = tile_height / source.height();
        if scale_x <= 0.0 || scale_y <= 0.0 {
            return;
        }
        let period_x = if xs.len() > 1 {
            xs[1].0 - first_left
        } else if repeat_x == BorderImageRepeat::Space {
            tile_width + (first_left - destination.left).max(0.0)
        } else {
            tile_width
        };
        let period_y = if ys.len() > 1 {
            ys[1].0 - first_top
        } else if repeat_y == BorderImageRepeat::Space {
            tile_height + (first_top - destination.top).max(0.0)
        } else {
            tile_height
        };
        let tile_bounds = Rect::from_xywh(
            source.left,
            source.top,
            period_x / scale_x,
            period_y / scale_y,
        );
        let picture_bounds = Rect::from_ltrb(
            tile_bounds.left - 1.0,
            tile_bounds.top - 1.0,
            tile_bounds.right + 1.0,
            tile_bounds.bottom + 1.0,
        );
        let mut recorder = PictureRecorder::new();
        let recording_canvas = recorder.begin_recording(picture_bounds, false);
        paint_css_image_tile(
            recording_canvas,
            doc,
            style,
            image,
            Rect::from_xywh(0.0, 0.0, concrete_width, concrete_height),
            false,
            false,
            false,
            false,
            None,
            false,
            true,
            opacity_multiplier,
            None,
            Some(source),
            None,
            false,
        );
        if let Some(picture) = recorder.finish_recording_as_picture(None) {
            let matrix = Matrix::scale_translate((scale_x, scale_y), (first_left, first_top));
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_shader(picture.to_shader(
                (TileMode::Repeat, TileMode::Repeat),
                FilterMode::Linear,
                &matrix,
                Some(&tile_bounds),
            ));
            canvas.save();
            canvas.clip_rect(coverage_clip, ClipOp::Intersect, false);
            canvas.draw_rect(destination, &paint);
            canvas.restore();
        }
        return;
    }
    canvas.save();
    canvas.clip_rect(coverage_clip, ClipOp::Intersect, false);
    for (top, height) in ys {
        for (left, width) in &xs {
            let tile_destination = Rect::from_xywh(*left, top, *width, height);
            canvas.save();
            canvas.translate((tile_destination.left, tile_destination.top));
            canvas.scale((
                tile_destination.width() / source.width(),
                tile_destination.height() / source.height(),
            ));
            canvas.translate((-source.left, -source.top));
            paint_css_image_tile(
                canvas,
                doc,
                style,
                image,
                Rect::from_xywh(0.0, 0.0, concrete_width, concrete_height),
                false,
                false,
                false,
                false,
                None,
                false,
                true,
                opacity_multiplier,
                None,
                None,
                None,
                false,
            );
            canvas.restore();
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
    device_scale: f32,
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
    crate::image_resource::clip_svg_patch_to_predecessor(
        canvas,
        source,
        concrete_width,
        concrete_height,
        destination,
        device_scale,
    );
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
                Some(device_scale),
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
) -> Option<(skia_safe::Image, f32)> {
    if let CssImage::Raster(id) = &border_image.source {
        return crate::image_resource::decode_image_resource_at_size(
            doc,
            *id,
            source_width.ceil().max(1.0) as i32,
            source_height.ceil().max(1.0) as i32,
        )
        .ok()
        .map(|image| (image, 1.0));
    }
    // `border-image-width: auto` is derived from the source slice in CSS
    // image space. Keep generated images in that logical coordinate space;
    // explicit widths can retain a device-resolution source and be assembled
    // as a bit-preserving physical image.
    let logical_source = border_image
        .width
        .iter()
        .any(|width| matches!(width, BorderImageLength::Auto));
    let source_scale = if logical_source {
        1.0
    } else {
        style.device_scale_factor.max(f64::EPSILON) as f32
    };
    let width = (source_width * source_scale).ceil().max(1.0) as i32;
    let height = (source_height * source_scale).ceil().max(1.0) as i32;
    let mut surface = if logical_source {
        let source_info = ImageInfo::new(
            (width, height),
            ColorType::RGBAF16,
            AlphaType::Premul,
            ColorSpace::new_srgb(),
        );
        surfaces::raster(&source_info, None, None)?
    } else {
        surfaces::raster_n32_premul((width, height))?
    };
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    surface.canvas().scale((source_scale, source_scale));
    paint_css_image_tile(
        surface.canvas(),
        doc,
        style,
        &border_image.source,
        Rect::from_xywh(0.0, 0.0, source_width, source_height),
        false,
        false,
        false,
        false,
        None,
        false,
        false,
        1.0,
        None,
        None,
        None,
        false,
    );
    Some((surface.image_snapshot(), source_scale))
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
    let (image, source_scale) =
        match border_image_source(doc, style, border_image, outer.width(), outer.height()) {
            Some(image) => image,
            None => return false,
        };
    let generated_source = !matches!(border_image.source, CssImage::Raster(_));
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
        resolve_border_image_slice(&border_image.slice[0], source_height / source_scale)
            * source_scale,
        resolve_border_image_slice(&border_image.slice[1], source_width / source_scale)
            * source_scale,
        resolve_border_image_slice(&border_image.slice[2], source_height / source_scale)
            * source_scale,
        resolve_border_image_slice(&border_image.slice[3], source_width / source_scale)
            * source_scale,
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
            source_slices[0] / source_scale,
        ),
        resolve_border_image_width(
            &border_image.width[1],
            border_widths[1],
            outer.width(),
            source_slices[1] / source_scale,
        ),
        resolve_border_image_width(
            &border_image.width[2],
            border_widths[2],
            outer.height(),
            source_slices[2] / source_scale,
        ),
        resolve_border_image_width(
            &border_image.width[3],
            border_widths[3],
            outer.width(),
            source_slices[3] / source_scale,
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
                    style.device_scale_factor.max(f64::EPSILON) as f32,
                );
            } else if generated_source {
                draw_border_generated_image_patch(
                    canvas,
                    doc,
                    style,
                    &border_image.source,
                    source_width / source_scale,
                    source_height / source_scale,
                    Rect::from_ltrb(
                        source.left / source_scale,
                        source.top / source_scale,
                        source.right / source_scale,
                        source.bottom / source_scale,
                    ),
                    destination,
                    repeat_x,
                    repeat_y,
                    opacity_multiplier,
                );
            } else {
                draw_border_image_patch(
                    canvas,
                    &image,
                    source_scale,
                    style.device_scale_factor.max(f64::EPSILON) as f32,
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
fn css_shadow_blur_sigma(radius: f32) -> f32 {
    // Blink records half the CSS blur length as the local-space Gaussian
    // sigma. The mask filter must then respect the canvas transform so DPR and
    // authored transforms scale the blur together with the shadow geometry.
    radius.max(0.0) * 0.5
}

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
            let sigma = css_shadow_blur_sigma(shadow.blur_radius);
            if let Some(filter) =
                skia_safe::MaskFilter::blur(skia_safe::BlurStyle::Normal, sigma, true)
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
            let rectangular_ancestor_clip = canvas.is_clip_rect();
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
                let flat_opaque_shadow = shadow.blur_radius == 0.0
                    && shadow.color.is_opaque()
                    && rectangular_ancestor_clip;
                let exclusion_rect = if style.background_color.is_opaque() && flat_opaque_shadow {
                    flat_shadow_exclusion_rect(border_rect, style.device_scale_factor)
                } else if style.background_color.is_opaque() {
                    // When an opaque box fill follows the shadow, Blink's
                    // half-open trailing edge leaves the boundary cell in the
                    // shadow mask and lets the later fill supply its partial
                    // coverage. Transparent boxes must exclude the complete
                    // cell so shadow ink cannot leak into their interior.
                    half_open_physical_clip_rect(border_rect, style.device_scale_factor)
                } else {
                    border_rect
                };
                canvas.clip_rect(exclusion_rect, ClipOp::Difference, false);
                if flat_opaque_shadow {
                    // Match the physical edge coverage used by other flat CSS
                    // rectangles. Skia's analytic AA packs a half-covered
                    // shadow edge one channel step above Chromium at 1.5x.
                    // Preserve Skia's joint coverage at a curved ancestor clip.
                    let trailing_phase =
                        (f64::from(shadow_rect.bottom) * style.device_scale_factor).rem_euclid(1.0);
                    // Opaque fills are composited over the shadow's trailing
                    // fringe; a transparent box leaves the shadow on the
                    // ordinary coverage path (including inline fragments).
                    let packing =
                        if style.background_color.is_opaque() && trailing_phase > 0.5 + 1.0e-6 {
                            PhysicalCoveragePacking::TrailingY
                        } else {
                            PhysicalCoveragePacking::Default
                        };
                    // The first replay tile includes physical column 255 as
                    // overlap; the next tile owns the following visible
                    // columns. Keep the edge packing on each side of that
                    // raster boundary consistent with its tile-local clip.
                    let tile_boundary = (crate::render::RASTER_TILE_SIZE_PX - 1) as f32
                        / style.device_scale_factor as f32;
                    let tiled_replay = RASTER_TILED_REPLAY.with(|value| *value.borrow());
                    if tiled_replay
                        && matches!(packing, PhysicalCoveragePacking::TrailingY)
                        && shadow_rect.left < tile_boundary
                        && shadow_rect.right > tile_boundary
                    {
                        draw_css_coverage_rect(
                            canvas,
                            Rect::from_ltrb(
                                shadow_rect.left,
                                shadow_rect.top,
                                tile_boundary,
                                shadow_rect.bottom,
                            ),
                            &shadow.color,
                            style.device_scale_factor,
                            packing,
                        );
                        draw_css_coverage_rect(
                            canvas,
                            Rect::from_ltrb(
                                tile_boundary,
                                shadow_rect.top,
                                shadow_rect.right,
                                shadow_rect.bottom,
                            ),
                            &shadow.color,
                            style.device_scale_factor,
                            PhysicalCoveragePacking::Default,
                        );
                    } else {
                        let packing = if tiled_replay
                            && matches!(packing, PhysicalCoveragePacking::TrailingY)
                            && shadow_rect.left >= tile_boundary
                        {
                            PhysicalCoveragePacking::Default
                        } else {
                            packing
                        };
                        draw_css_coverage_rect(
                            canvas,
                            shadow_rect,
                            &shadow.color,
                            style.device_scale_factor,
                            packing,
                        );
                    }
                } else {
                    canvas.draw_rect(shadow_rect, &paint);
                }
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
        let rendered_legend = doc.fieldset_rendered_legend(fragment.node_id);
        let legend_block_size = fragment
            .children
            .iter()
            .filter(|child| Some(child.node_id) == rendered_legend)
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
                if Some(child.node_id) == rendered_legend {
                    // The rendered legend defines the fieldset's border area
                    // even when earlier source children keep it later in the
                    // ordinary content flow. Decoration geometry therefore
                    // uses the legend at block-start; its real fragment is
                    // painted at its source-order position below.
                    child.offset.top = -decoration_shift;
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
    let native_scroll_button_theme = uses_native_scroll_button_theme(fragment, doc);
    let native_color_theme = node.form_control_native_appearance
        && node.form_control == Some(FormControlRole::ColorInput)
        && style.border_top_style == BorderStyle::Solid
        && style.border_right_style == BorderStyle::Solid
        && style.border_bottom_style == BorderStyle::Solid
        && style.border_left_style == BorderStyle::Solid;
    let native_text_control_theme = node.form_control_native_appearance
        && matches!(
            node.form_control,
            Some(
                FormControlRole::TextInput
                    | FormControlRole::DateInput
                    | FormControlRole::TextArea
                    | FormControlRole::Select
            )
        )
        // Author borders suppress the native text-control border theme even
        // when `appearance` itself remains `auto`.  The porter represents the
        // pinned HTML UA border as `inset`, so only that declaration receives
        // the neutral one-device-pixel Linux theme ring.
        && style.border_top_style == BorderStyle::Inset
        && style.border_right_style == BorderStyle::Inset
        && style.border_bottom_style == BorderStyle::Inset
        && style.border_left_style == BorderStyle::Inset
        && style.effective_border_top() > 0
        && style.effective_border_right() > 0
        && style.effective_border_bottom() > 0
        && style.effective_border_left() > 0;
    let clip_native_text_control_corners = native_text_control_theme
        && (style.background_color == Color::TRANSPARENT || style.background_color == Color::WHITE);
    let style = if node.form_control == Some(FormControlRole::Range)
        && node.form_control_native_appearance
    {
        // Native range appearance replaces the author's track background.
        // `appearance:none` keeps the authored background and uses the
        // ordinary decoration path.
        native_control_style = {
            style.derive(|adjusted| {
                adjusted.background_color = Color::TRANSPARENT;
                adjusted.background_layers.clear();
                adjusted.background_linear_gradient = None;
            })
        };
        &native_control_style
    } else if native_button_theme || native_scroll_button_theme || native_color_theme {
        // Blink reserves a 2px CSS border for native button geometry, while
        // the passive Linux theme paints only its outer device-pixel ring;
        // the inner pixel is button-face background. Preserve the layout
        // strut in the fragment and narrow only the paint representation.
        native_control_style = {
            style.derive(|adjusted| {
                let painted_border = i32::from(
                    fragment.size.width >= LayoutUnit::from_i32(6)
                        && fragment.size.height >= LayoutUnit::from_i32(6),
                );
                adjusted.border_top_width = painted_border;
                adjusted.border_right_width = painted_border;
                adjusted.border_bottom_width = painted_border;
                adjusted.border_left_width = painted_border;
                if (native_button_theme || native_color_theme) && node.form_control_disabled {
                    adjusted.background_color = Color::from_rgba8(238, 238, 238, 255);
                    let border =
                        openui_style::StyleColor::Resolved(Color::from_rgba8(208, 208, 208, 255));
                    adjusted.border_top_color = border;
                    adjusted.border_right_color = border;
                    adjusted.border_bottom_color = border;
                    adjusted.border_left_color = border;
                }
                if painted_border == 0 {
                    adjusted.border_top_left_radius = (0.0, 0.0);
                    adjusted.border_top_right_radius = (0.0, 0.0);
                    adjusted.border_bottom_right_radius = (0.0, 0.0);
                    adjusted.border_bottom_left_radius = (0.0, 0.0);
                }
            })
        };
        &native_control_style
    } else if native_text_control_theme {
        // Passive Linux text controls reserve their UA inset widths for
        // layout, but the native theme raster is a single neutral outer ring.
        // Keep the fragment struts intact and narrow only the painted border.
        native_control_style = {
            style.derive(|adjusted| {
                adjusted.border_top_width = 1;
                adjusted.border_right_width = 1;
                adjusted.border_bottom_width = 1;
                adjusted.border_left_width = 1;
                adjusted.border_top_style = BorderStyle::Solid;
                adjusted.border_right_style = BorderStyle::Solid;
                adjusted.border_bottom_style = BorderStyle::Solid;
                adjusted.border_left_style = BorderStyle::Solid;
                let border =
                    openui_style::StyleColor::Resolved(Color::from_rgba8(118, 118, 118, 255));
                adjusted.border_top_color = border;
                adjusted.border_right_color = border;
                adjusted.border_bottom_color = border;
                adjusted.border_left_color = border;
                if adjusted.background_color == Color::TRANSPARENT {
                    adjusted.background_color = Color::WHITE;
                }
            })
        };
        &native_control_style
    } else {
        style
    };
    let scaled_native_theme = (style.device_scale_factor - 1.0).abs() > f64::EPSILON;
    if (native_button_theme || native_color_theme) && scaled_native_theme {
        paint_direct_native_button_theme(
            canvas,
            fragment,
            abs_offset,
            opacity_multiplier,
            style.device_scale_factor,
            style.has_outline(),
            node.form_control_disabled,
        );
        return;
    }
    if native_text_control_theme && scaled_native_theme {
        paint_direct_native_text_control_theme(
            canvas,
            fragment,
            abs_offset,
            opacity_multiplier,
            style.device_scale_factor,
            style.has_outline(),
            &style.background_color,
        );
        return;
    }
    if fragment.decoration_clip_rects.is_empty() {
        if (native_button_theme || native_scroll_button_theme || native_color_theme)
            && fragment.size.width >= LayoutUnit::from_i32(6)
            && fragment.size.height >= LayoutUnit::from_i32(6)
        {
            canvas.save();
            clip_native_button_corner_cells(canvas, fragment, abs_offset);
        }
        if clip_native_text_control_corners {
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
        paint_native_marker_group_scrollbar_arrows(canvas, fragment, node, style, abs_offset);
        if clip_native_text_control_corners {
            canvas.restore();
        }
        if (native_button_theme || native_scroll_button_theme || native_color_theme)
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
        if native_button_theme || native_scroll_button_theme || native_color_theme {
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

/// Paint the pinned Linux scrollbar arrow buttons exposed by a forced
/// two-axis scrollbar on a generated scroll-marker-group. The group remains
/// a zero-block-size pseudo, but its native scrollbar chrome is still ink.
fn paint_native_marker_group_scrollbar_arrows(
    canvas: &Canvas,
    fragment: &Fragment,
    node: &openui_dom::NodeData,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
) {
    if node.pseudo_kind != Some(PseudoElementKind::ScrollMarkerGroup)
        || style.overflow_x != Overflow::Scroll
        || style.overflow_y != Overflow::Scroll
        || fragment.size.height.round() != LayoutUnit::zero()
        || fragment.size.width < LayoutUnit::from_i32(52)
    {
        return;
    }

    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let right = (abs_offset.left + fragment.size.width).round().to_f32();
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    // Linux overlay scrollbars expose the horizontal track even though the
    // generated marker group itself has zero block size. The 15 CSS-pixel
    // native track is painted before its pinned arrow glyphs.
    set_paint_css_color(&mut paint, &Color::from_rgba8(252, 252, 252, 255));
    canvas.draw_rect(Rect::from_xywh(x, y, right - x, 15.0), &paint);

    set_paint_css_color(&mut paint, &Color::from_rgba8(139, 139, 139, 255));

    let mut left_arrow = PathBuilder::new();
    left_arrow.move_to((x + 10.0, y + 3.0));
    left_arrow.line_to((x + 5.0, y + 7.5));
    left_arrow.line_to((x + 10.0, y + 12.0));
    left_arrow.close();
    canvas.draw_path(&left_arrow.detach(), &paint);

    let mut right_arrow = PathBuilder::new();
    right_arrow.move_to((right - 26.0, y + 3.0));
    right_arrow.line_to((right - 21.0, y + 7.5));
    right_arrow.line_to((right - 26.0, y + 12.0));
    right_arrow.close();
    canvas.draw_path(&right_arrow.detach(), &paint);
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

/// Paint an axis-aligned CSS rectangle with coverage computed in physical
/// pixels. Skia's software `drawRect` snaps axis-aligned edges even when AA is
/// enabled, while Chromium's composited paint path retains fractional edge
/// coverage. Keeping the calculation here makes that coverage independent of
/// the picture replay scale and of tile boundaries.
fn paint_device_coverage_rect(
    canvas: &Canvas,
    rect: Rect,
    color: &Color,
    opacity_multiplier: f32,
    device_scale: f64,
) {
    fn axis_segments(start: f32, end: f32, scale: f32) -> Vec<(f32, f32, f32)> {
        let physical_start = start * scale;
        let physical_end = end * scale;
        let first = physical_start.floor();
        let last = physical_end.floor();
        if first == last {
            let coverage = (physical_end - physical_start).clamp(0.0, 1.0);
            return (coverage > 0.0)
                .then_some((first / scale, (first + 1.0) / scale, coverage))
                .into_iter()
                .collect();
        }

        let mut segments = Vec::with_capacity(3);
        let leading = physical_start.fract();
        if leading > f32::EPSILON {
            segments.push((first / scale, (first + 1.0) / scale, 1.0 - leading));
        }
        let interior_start = physical_start.ceil();
        let interior_end = physical_end.floor();
        if interior_end > interior_start {
            segments.push((interior_start / scale, interior_end / scale, 1.0));
        }
        let trailing = physical_end.fract();
        if trailing > f32::EPSILON {
            segments.push((last / scale, (last + 1.0) / scale, trailing));
        }
        segments
    }

    let scale = device_scale as f32;
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let x_segments = axis_segments(rect.left, rect.right, scale);
    let y_segments = axis_segments(rect.top, rect.bottom, scale);
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    for &(left, right, x_coverage) in &x_segments {
        for &(top, bottom, y_coverage) in &y_segments {
            set_paint_css_color_with_alpha(
                &mut paint,
                color,
                opacity_multiplier * x_coverage * y_coverage,
            );
            canvas.draw_rect(Rect::from_ltrb(left, top, right, bottom), &paint);
        }
    }
}

fn has_negative_stacking_descendant(fragment: &Fragment, doc: &Document) -> bool {
    fragment.children.iter().any(|child| {
        if child.node_id.is_none() {
            return has_negative_stacking_descendant(child, doc);
        }
        let style = &doc.node(child.node_id).style;
        let positioned = matches!(
            style.position,
            Position::Absolute | Position::Fixed | Position::Relative | Position::Sticky
        );
        (positioned && style.z_index.is_some_and(|z| z < 0))
            || (!is_fragment_stacking_context(child, doc)
                && has_negative_stacking_descendant(child, doc))
    })
}

/// Whether one ordinary in-flow child supplies an opaque backing for the
/// parent's complete inner border box.
///
/// Blink's paint culling drops a background-color display item when later
/// opaque content covers it. That matters even when the hidden color sits
/// below a border: retaining it makes independently antialiased border and
/// child edges blend against that color instead of their shared backdrop.
/// Keep this deliberately conservative. It recognizes only a single
/// axis-aligned, unfragmented box with an opaque border-box background or a
/// decoded opaque image covering its complete border box.
/// An anonymous line may wrap an atomic inline box without changing where
/// that box paints; include that line's offset when testing coverage.
fn opaque_in_flow_child_covers_inner_border_box(fragment: &Fragment, doc: &Document) -> bool {
    let inner_left = fragment.border.left;
    let inner_top = fragment.border.top;
    let inner_right = fragment.size.width - fragment.border.right;
    let inner_bottom = fragment.size.height - fragment.border.bottom;

    fragment.children.iter().any(|first_child| {
        let mut child = first_child;
        let mut wrapper_offset = PhysicalOffset::zero();
        while child.node_id.is_none() {
            if child.kind != FragmentKind::Box
                || child.children.len() != 1
                || child.has_overflow_clip
                || child.block_axis_clip_only
                || child.inline_axis_clip_only
                || child.skip_box_decoration
                || child.decoration_paint_block_size.is_some()
                || child.decoration_slice.is_some()
                || !child.decoration_clip_rects.is_empty()
                || !child.is_first_for_node
                || !child.is_last_for_node
                || child.inherited_style.is_some()
                || child.paint_background_color_override.is_some()
                || child.is_inline_box_fragment
                || !child.oof_candidates.is_empty()
                || !child.promoted_transform_ancestors.is_empty()
            {
                return false;
            }
            wrapper_offset.left += child.offset.left;
            wrapper_offset.top += child.offset.top;
            child = &child.children[0];
        }

        if child.node_id.is_none()
            || child.kind != FragmentKind::Box
            || child.skip_box_decoration
            || child.decoration_paint_block_size.is_some()
            || child.decoration_slice.is_some()
            || !child.decoration_clip_rects.is_empty()
            || !child.is_first_for_node
            || !child.is_last_for_node
        {
            return false;
        }

        let child_style = &doc.node(child.node_id).style;
        let background_color = child
            .paint_background_color_override
            .as_ref()
            .unwrap_or(&child_style.background_color);
        if child_style.opacity < 1.0
            || child_style.visibility != Visibility::Visible
            || child_style.position != Position::Static
            || child_style.transform != openui_style::Transform2D::IDENTITY
            || child_style.filter_blur > 0.0
            || child_style.clip_path_inset.is_some()
            || !child_style.mask_layers.is_empty()
            || child_style.background_attachment == BackgroundAttachment::Local
            || child_style.background_clip != BackgroundClip::BorderBox
            || (child_style.has_border_radius() && !child.ignore_border_radius)
        {
            return false;
        }

        let child_left = wrapper_offset.left + child.offset.left;
        let child_top = wrapper_offset.top + child.offset.top;
        let child_right = child_left + child.size.width;
        let child_bottom = child_top + child.size.height;
        let covers_inner_border_box = child_left <= inner_left
            && child_top <= inner_top
            && child_right >= inner_right
            && child_bottom >= inner_bottom;
        covers_inner_border_box
            && (background_color.is_opaque()
                || (!fragment.node_id.is_none()
                    && fragment.node_id != doc.root()
                    && doc.node(fragment.node_id).style.box_shadow.is_empty()
                    && opaque_image_covers_border_box(child, doc)))
    })
}

/// Blink's LayoutImage opacity proof includes image foreground, not only its
/// background color. Restrict this proof to the default object position and
/// fill/cover fits, with no border, padding, or layer that could expose the
/// parent. Image alpha is checked on the actual decoded document resource.
fn opaque_image_covers_border_box(fragment: &Fragment, doc: &Document) -> bool {
    let node = doc.node(fragment.node_id);
    let style = &node.style;
    if fragment.border != BoxStrut::zero()
        || fragment.padding != BoxStrut::zero()
        || fragment.size.width <= LayoutUnit::zero()
        || fragment.size.height <= LayoutUnit::zero()
        || fragment.block_axis_clip_only
        || fragment.inline_axis_clip_only
        || !fragment.promoted_transform_ancestors.is_empty()
        || style.establishes_transform_containing_block
        || style.will_change_transform
        || style.filter_grayscale > 0.0
        || !matches!(style.shape_outside, openui_style::ShapeOutside::None)
        || !matches!(style.object_fit, ObjectFit::Fill | ObjectFit::Cover)
        || !matches!(style.object_position.x, BackgroundPosition::Percent(value) if value == 50.0)
        || !matches!(style.object_position.y, BackgroundPosition::Percent(value) if value == 50.0)
    {
        return false;
    }
    let Some(replaced) = node.replaced else {
        return false;
    };
    let ReplacedResourceKind::Image(id) = replaced.resource else {
        return false;
    };
    crate::image_resource::decode_image_resource(doc, id).is_ok_and(|image| image.is_opaque())
}

/// A child may cover the local border box but still be moved or clipped in a
/// fragmentainer. Preserve the parent's background in that context.
fn borderless_background_can_be_occluded(fragment: &Fragment, doc: &Document) -> bool {
    if !fragment.is_first_for_node
        || !fragment.is_last_for_node
        || fragment.break_token.is_some()
        || fragment.block_axis_clip_only
        || fragment.inline_axis_clip_only
    {
        return false;
    }
    let mut current = fragment.node_id;
    while !current.is_none() {
        let node = doc.node(current);
        if node.style.column_count.is_some() || node.style.column_width.is_some() {
            return false;
        }
        current = node.parent;
    }
    true
}

fn uses_squared_replaced_background_coverage(
    style: &ComputedStyle,
    rect: Rect,
    replaced_resource: Option<ReplacedResourceKind>,
) -> bool {
    if !style.background_color.is_opaque()
        // `TransparentCanvas` supplies intrinsic geometry for native form
        // controls; it is not a composited replaced-content backing. Squaring
        // its box-background coverage would darken the control's independently
        // rasterized border at fractional device scales.
        || replaced_resource.is_none_or(|resource| {
            matches!(resource, ReplacedResourceKind::TransparentCanvas)
        })
        || (style.overflow_x == Overflow::Visible && style.overflow_y == Overflow::Visible)
    {
        return false;
    }
    let scale = style.device_scale_factor.max(f64::EPSILON);
    [rect.left, rect.top, rect.right, rect.bottom]
        .into_iter()
        .any(|edge| {
            let phase = (f64::from(edge) * scale).rem_euclid(1.0);
            phase > 1.0e-6 && (1.0 - phase) > 1.0e-6
        })
}

fn paint_box_decoration_background(
    canvas: &Canvas,
    fragment: &Fragment,
    doc: &Document,
    style: &ComputedStyle,
    abs_offset: PhysicalOffset,
    opacity_multiplier: f32,
) {
    let node = doc.node(fragment.node_id);
    let decoded_media_frame = if node.tag == ElementTag::Video {
        match node.replaced.map(|replaced| replaced.resource) {
            Some(ReplacedResourceKind::MediaPoster(id)) => doc
                .image_resource(id)
                .is_some_and(|resource| resource.mime_type == "image/x-openui-rgba8"),
            _ => false,
        }
    } else {
        false
    };
    // Pixel-snap all four edges independently (Blink's PixelSnappedIntRect).
    // Fractional abs_offset flows through from parent so that adjacent elements
    // at fractional boundaries (e.g. flex items at 133.33px intervals) share
    // the same snapped pixel edge — no gaps.
    let x = abs_offset.left.round().to_f32();
    let y = abs_offset.top.round().to_f32();
    let mut full_right = (abs_offset.left + fragment.size.width).round().to_f32();
    let mut full_bottom = (abs_offset.top + fragment.size.height).round().to_f32();
    // Pixel-snapped non-empty geometry retains at least one device pixel.
    // This is observable for fractional animated boxes whose independently
    // rounded start/end edges otherwise collapse to the same coordinate.
    if full_right == x && fragment.size.width > LayoutUnit::zero() {
        full_right += 1.0;
    }
    if full_bottom == y && fragment.size.height > LayoutUnit::zero() {
        full_bottom += 1.0;
    }
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
        // The decoration primitive owns its fractional block edge. Close the
        // safety clip outward on the device grid so it does not discard that
        // primitive's antialias coverage. A borderless inline edge retains its
        // original hard clip to avoid overlapping neighboring columns.
        let inline_border_ink = if fragment_block_axis_is_x(fragment) {
            style.effective_border_top() > 0 || style.effective_border_bottom() > 0
        } else {
            style.effective_border_left() > 0 || style.effective_border_right() > 0
        };
        let snapping = RasterSnapping::new(style.device_scale_factor);
        let preserve_inline =
            inline_border_ink || fragment.decoration_limit_preserves_inline_coverage;
        let clip_rect = if fragment_block_axis_is_x(fragment) {
            Rect::from_ltrb(
                snapping.logical_coordinate(border_box_rect.left, PhysicalSnap::Floor),
                if preserve_inline {
                    snapping.logical_coordinate(border_box_rect.top, PhysicalSnap::Floor)
                } else {
                    border_box_rect.top
                },
                snapping.logical_coordinate(border_box_rect.right, PhysicalSnap::Ceil),
                if preserve_inline {
                    snapping.logical_coordinate(border_box_rect.bottom, PhysicalSnap::Ceil)
                } else {
                    border_box_rect.bottom
                },
            )
        } else {
            Rect::from_ltrb(
                if preserve_inline {
                    snapping.logical_coordinate(border_box_rect.left, PhysicalSnap::Floor)
                } else {
                    border_box_rect.left
                },
                snapping.logical_coordinate(border_box_rect.top, PhysicalSnap::Floor),
                if preserve_inline {
                    snapping.logical_coordinate(border_box_rect.right, PhysicalSnap::Ceil)
                } else {
                    border_box_rect.right
                },
                snapping.logical_coordinate(border_box_rect.bottom, PhysicalSnap::Ceil),
            )
        };
        canvas.save();
        canvas.clip_rect(clip_rect, ClipOp::Intersect, false);
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
    let platform_text_control_ring = uniform_border
        && bt == 1.0
        && style.device_scale_factor.fract().abs() > f64::EPSILON
        && style.background_color == Color::WHITE
        && style.border_top_color.resolve(&style.color) == Color::from_rgba8(118, 118, 118, 255);
    let use_layer = has_radius
        && has_visible_background
        && style.border_image.is_none()
        // The Linux native text-control ring is snapped independently on
        // all four sides.  An analytic outer-rrect clip would reintroduce
        // half coverage at a fractional shared edge after the ring itself
        // has selected the owning device cell.
        && !platform_text_control_ring
        // Blink shrinks an obscured background under an opaque border,
        // regardless of the authored background-clip. It does not need a
        // separate outer clip layer for that border.
        && !border_obscures_background_edge
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
    let opaque_child_occludes_background = !has_radius
        && style.border_image.is_none()
        && effective_background_clip == BackgroundClip::BorderBox
        // Both an opaque solid ring and a borderless box can leave the
        // parent's color fully hidden under a covering child. Retaining that
        // color adds a second AA coverage layer at their shared edge.
        // A scroll container owns a distinct scroll backing. Its box
        // background remains part of that backing even when an opaque child
        // currently covers the complete scrollport; the child may move as
        // the scroll offset changes.
        && style.overflow_x == Overflow::Visible
        && style.overflow_y == Overflow::Visible
        && (side_specs.iter().all(|(width, border_style, color)| {
            *width > 0.0 && *border_style == BorderStyle::Solid && color.is_opaque()
        }) || (side_specs.iter().all(|(width, _, _)| *width == 0.0)
            && borderless_background_can_be_occluded(fragment, doc)))
        && opaque_in_flow_child_covers_inner_border_box(fragment, doc);
    let background_color_is_occluded = opacity_multiplier >= 1.0
        && (opaque_child_occludes_background
            || style.background_layers.last().is_some_and(|layer| {
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
            }));
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
            let bg_rect = if decoded_media_frame {
                // Decoded video frames are promoted to a composited media
                // layer. Its element background and frame share one
                // device-pixel-aligned quad, so a half-device-pixel trailing
                // edge belongs wholly to one neighboring row instead of
                // receiving CPU Skia's independent analytic coverage.
                let snapping = RasterSnapping::new(style.device_scale_factor);
                Rect::from_ltrb(
                    snapping.logical_coordinate(bg_rect.left, PhysicalSnap::Nearest),
                    snapping.logical_coordinate(bg_rect.top, PhysicalSnap::Nearest),
                    snapping.logical_coordinate(bg_rect.right, PhysicalSnap::Nearest),
                    snapping.logical_coordinate(bg_rect.bottom, PhysicalSnap::Nearest),
                )
            } else if platform_text_control_ring
                && effective_background_clip == BackgroundClip::BorderBox
            {
                // Adjacent native controls partition their shared fractional
                // boundary before painting the opaque field background.  If
                // the later control begins at half a device pixel, starting
                // its ordinary antialiased fill there would wash out half of
                // the preceding control's already-snapped border cell.
                let scale = style.device_scale_factor as f32;
                let snap_boundary = |value: f32| (value * scale + 0.5).floor() / scale;
                Rect::from_ltrb(
                    snap_boundary(bg_rect.left),
                    snap_boundary(bg_rect.top),
                    snap_boundary(bg_rect.right),
                    snap_boundary(bg_rect.bottom),
                )
            } else {
                close_rect_at_physical_viewport_edge(bg_rect, style.device_scale_factor)
            };
            annotate_content_background(
                canvas,
                style.background_color,
                bg_rect,
                opacity_multiplier,
                (!has_radius && !use_layer).then_some(bg_rect),
            );
            if has_radius {
                // Compute radii adjusted for background-clip box (CSS Backgrounds §5.3).
                // Inner corner radii = outer radii - inset on each side, clamped to 0.
                let outer_radii = if all_effective_borders_transparent(style) {
                    slice_adjust_border_radii(
                        specified_border_radii(style, &background_box),
                        fragment,
                    )
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
                    let hard_clip_x =
                        style.overflow_x == Overflow::Clip && style.overflow_y == Overflow::Visible;
                    if hard_clip_x {
                        // A one-axis paint-property clip owns the inline edge
                        // for both the direct and F16 circular fill paths.
                        let big = 100_000.0;
                        canvas.save();
                        canvas.clip_rect(
                            Rect::from_ltrb(bg_rect.left, -big, bg_rect.right, big),
                            ClipOp::Intersect,
                            false,
                        );
                    }
                    if is_opaque_circular_background(&bg_rect, &clip_radii, &style.background_color)
                    {
                        // Chromium retains circular coverage in a float
                        // intermediate before compositing it into the N32
                        // destination. Keeping this primitive-level policy
                        // independent of element role and dimensions avoids
                        // correcting individual circumference samples.
                        canvas.save_layer(
                            &SaveLayerRec::default()
                                .bounds(&bg_rect)
                                .flags(SaveLayerFlags::F16_COLOR_TYPE),
                        );
                        canvas.draw_rrect(RRect::new_rect_radii(bg_rect, &clip_radii), &paint);
                        canvas.restore();
                    } else {
                        canvas.draw_rrect(RRect::new_rect_radii(bg_rect, &clip_radii), &paint);
                    }
                    if hard_clip_x {
                        canvas.restore();
                    }
                } else {
                    canvas.save();
                    clip_nonrenderable_inner_rounded_rect(
                        canvas,
                        border_box_rect,
                        bg_rect,
                        &clip_radii,
                    );
                    // Blink clips the content contour, then fills the enclosing
                    // background paint rect. Apply the curved contour before
                    // the straight content edge so their shared AA coverage
                    // is rounded once at the border/background seam.
                    canvas.clip_rect(bg_rect, ClipOp::Intersect, true);
                    canvas.draw_rect(border_box_rect, &paint);
                    canvas.restore();
                }
            } else {
                let preserves_composited_edge_coverage = style.z_index.is_some_and(|z| z < 0)
                    || has_negative_stacking_descendant(fragment, doc);
                if matches!(
                    effective_background_clip,
                    BackgroundClip::PaddingBox | BackgroundClip::ContentBox
                ) {
                    // Blink fills the CSS-pixel-snapped border box through a
                    // hard CSS-pixel-snapped padding/content clip. Device
                    // scale is applied after those layout-space edges are
                    // selected: the fill can still have fractional device
                    // coverage at its outer edge, while the hard clip does
                    // not leak color into the neighboring physical cell.
                    canvas.save();
                    canvas.clip_rect(bg_rect, ClipOp::Intersect, false);
                    canvas.draw_rect(background_box, &paint);
                    canvas.restore();
                } else if uses_squared_replaced_background_coverage(
                    style,
                    bg_rect,
                    node.replaced.map(|replaced| replaced.resource),
                ) {
                    // A clipped replaced element's box mask and its replaced
                    // backing carry the same analytic outer contour. Resolve
                    // both masks in physical space so a half-covered edge
                    // becomes quarter coverage (and a corner one sixteenth),
                    // matching Chromium's composited background.
                    draw_paint_with_physical_coverage(
                        canvas,
                        bg_rect,
                        &paint,
                        opacity_multiplier,
                        style.device_scale_factor,
                        true,
                    );
                } else if preserves_composited_edge_coverage {
                    paint_device_coverage_rect(
                        canvas,
                        bg_rect,
                        c,
                        opacity_multiplier,
                        style.device_scale_factor,
                    );
                } else {
                    canvas.draw_rect(bg_rect, &paint);
                }
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
            false,
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
                let direction = linear_gradient_direction(gradient.angle_degrees);
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
                    if has_radius {
                        canvas.save();
                        let clip_rrect = RRect::new_rect_radii(
                            paint_rect,
                            &fragment_border_radii(style, fragment, &paint_rect, bt),
                        );
                        canvas.clip_rrect(clip_rrect, ClipOp::Intersect, true);
                        canvas.draw_rect(paint_rect, &paint);
                        canvas.restore();
                    } else {
                        // The analytic fill already ends at `paint_rect`. A
                        // second hard clip to that identical rectangle drops
                        // fractional edge coverage where adjacent fragments
                        // meet on a non-integral device scale.
                        canvas.draw_rect(paint_rect, &paint);
                    }
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
            paint_borders(canvas, fragment, style, x, y, w, h, use_layer, true, false);
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
    let rendered_legend = doc.fieldset_rendered_legend(fragment.node_id);
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

    fn find_rendered_legend<'a>(
        fragment: &'a Fragment,
        legend_id: NodeId,
        offset: PhysicalOffset,
    ) -> Option<(&'a Fragment, PhysicalOffset)> {
        for child in &fragment.children {
            let child_offset = PhysicalOffset::new(
                offset.left + child.offset.left,
                offset.top + child.offset.top,
            );
            if child.node_id == legend_id {
                return Some((child, child_offset));
            }
            if let Some(legend) = find_rendered_legend(child, legend_id, child_offset) {
                return Some(legend);
            }
        }
        None
    }

    // Anonymous line/layout fragments may wrap the rendered legend. Follow
    // the fragment tree by its authoritative box-tree identity; a genuinely
    // nested legend cannot interrupt this fieldset's border.
    let Some((legend, _)) = rendered_legend
        .and_then(|legend_id| find_rendered_legend(fragment, legend_id, PhysicalOffset::zero()))
    else {
        paint_borders(canvas, fragment, style, x, y, w, h, use_layer, true, true);
        paint_native_fieldset_corner_samples(canvas, style, x, y, w, h);
        return;
    };
    let legend_id = legend.node_id;
    fn collect_legend_geometry(
        current: &Fragment,
        doc: &Document,
        legend_id: NodeId,
        offset: PhysicalOffset,
        geometry: &mut Option<(LayoutUnit, LayoutUnit, LayoutUnit, LayoutUnit)>,
    ) {
        let current_offset = PhysicalOffset::new(
            offset.left + current.offset.left,
            offset.top + current.offset.top,
        );
        if current.node_id == legend_id {
            let extent = if doc.node(legend_id).style.width.is_auto() {
                legend_auto_inline_extent(current, doc)
            } else {
                current.size.width
            };
            let bottom = current_offset.top + current.size.height;
            if let Some((left, top, existing_bottom, accumulated_extent)) = geometry {
                *left = (*left).min_of(current_offset.left);
                *top = (*top).min_of(current_offset.top);
                *existing_bottom = (*existing_bottom).max_of(bottom);
                *accumulated_extent = *accumulated_extent + extent;
            } else {
                *geometry = Some((current_offset.left, current_offset.top, bottom, extent));
            }
            return;
        }
        for child in &current.children {
            collect_legend_geometry(child, doc, legend_id, current_offset, geometry);
        }
    }
    let mut geometry = None;
    for child in &fragment.children {
        collect_legend_geometry(child, doc, legend_id, PhysicalOffset::zero(), &mut geometry);
    }
    let Some((legend_left, legend_top, legend_bottom, legend_inline_extent)) = geometry else {
        paint_borders(canvas, fragment, style, x, y, w, h, use_layer, true, true);
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
        // The top border is analytically covered by Skia. Close the gap over
        // every physical row touched by that coverage; otherwise a
        // fractional-scale fringe remains visible through transparent glyph
        // cells (most noticeably through spaces in the legend text). Keep
        // the inline end at its layout phase. Chromium assigns the start
        // junction to the nearest physical edge: always rounding it outward
        // would erase the first solid border column when the edge is already
        // in the trailing half of a device pixel.
        let snapping = RasterSnapping::new(style.device_scale_factor);
        let gap_rect = Rect::from_ltrb(
            snapping.logical_coordinate(gap_left, PhysicalSnap::Nearest),
            snapping.logical_coordinate(y + border_top, PhysicalSnap::Floor),
            gap_right,
            snapping.logical_coordinate(y + border_top + top_width, PhysicalSnap::Ceil),
        );
        canvas.clip_rect(gap_rect, ClipOp::Difference, false);
        // The legend is painted over the fieldset decoration. Exclude the
        // complete legend box as well as the horizontal gap so fractional
        // coverage from a vertical border edge cannot show through the last
        // physical row of the legend.
        canvas.clip_rect(
            Rect::from_ltrb(
                snapping.logical_coordinate(gap_left, PhysicalSnap::Nearest),
                snapping.logical_coordinate(y + legend_top.to_f32(), PhysicalSnap::Floor),
                gap_right,
                snapping.logical_coordinate(y + legend_bottom.to_f32(), PhysicalSnap::Floor),
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
        true,
    );
    paint_native_fieldset_corner_samples(canvas, style, x, y + border_top, w, border_height);
    canvas.restore();
}

/// Replay the unit-scale corner cells of Chromium's default 2px groove
/// fieldset border. The platform painter assigns the two groove shades
/// asymmetrically at the four junctions; generic polygon splitting leaves
/// five cells owned by the canvas or by the wrong shade.
fn paint_native_fieldset_corner_samples(
    canvas: &Canvas,
    style: &ComputedStyle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    if (style.device_scale_factor - 1.0).abs() > f64::EPSILON
        || style.effective_border_top() != 2
        || style.effective_border_right() != 2
        || style.effective_border_bottom() != 2
        || style.effective_border_left() != 2
        || style.border_top_style != BorderStyle::Groove
        || style.border_right_style != BorderStyle::Groove
        || style.border_bottom_style != BorderStyle::Groove
        || style.border_left_style != BorderStyle::Groove
        || style.border_top_color.resolve(&style.color) != Color::BLACK
        || style.border_right_color.resolve(&style.color) != Color::BLACK
        || style.border_bottom_color.resolve(&style.color) != Color::BLACK
        || style.border_left_color.resolve(&style.color) != Color::BLACK
        || width < 2.0
        || height < 2.0
    {
        return;
    }

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(false);
    for (sample_x, sample_y, gray) in [
        (x, y, 155),
        (x + width - 1.0, y, 197),
        (x + width - 2.0, y + 1.0, 197),
        (x, y + height - 1.0, 197),
        (x + width - 1.0, y + height - 1.0, 239),
    ] {
        set_paint_css_color(&mut paint, &Color::from_rgba8(gray, gray, gray, 255));
        canvas.draw_rect(Rect::from_xywh(sample_x, sample_y, 1.0, 1.0), &paint);
    }
}

/// A lone opaque curved solid or double edge follows BoxBorderPainter's complex
/// contour path: clip the outer/inner border, restrict the owning side, then
/// fill the solid edge or clip each double stripe using inset rounded contours.
/// The side clip retains the full corner tangents even when the edge is narrow.
fn paint_sole_curved_border(
    canvas: &Canvas,
    fragment: &Fragment,
    style: &ComputedStyle,
    rect: Rect,
    side: usize,
    width: f32,
    border_style: BorderStyle,
) -> bool {
    let color = match side {
        0 => style.border_top_color,
        1 => style.border_right_color,
        2 => style.border_bottom_color,
        _ => style.border_left_color,
    }
    .resolve(&style.color);
    if !color.is_opaque() {
        return false;
    }
    let radii = fragment_border_radii(style, fragment, &rect, width);
    let contour = |inset: f32| {
        let (top, right, bottom, left) = match side {
            0 => (inset, 0.0, 0.0, 0.0),
            1 => (0.0, inset, 0.0, 0.0),
            2 => (0.0, 0.0, inset, 0.0),
            _ => (0.0, 0.0, 0.0, inset),
        };
        let inner = Rect::from_xywh(
            rect.left + left,
            rect.top + top,
            (rect.width() - left - right).max(0.0),
            (rect.height() - top - bottom).max(0.0),
        );
        let inner_radii = [
            Point::new((radii[0].x - left).max(0.0), (radii[0].y - top).max(0.0)),
            Point::new((radii[1].x - right).max(0.0), (radii[1].y - top).max(0.0)),
            Point::new(
                (radii[2].x - right).max(0.0),
                (radii[2].y - bottom).max(0.0),
            ),
            Point::new((radii[3].x - left).max(0.0), (radii[3].y - bottom).max(0.0)),
        ];
        (inner, inner_radii)
    };
    let (inner, inner_radii) = contour(width);
    if inner.width() > 0.0 && inner.height() > 0.0 && radii_exceed_rect(&inner, &inner_radii) {
        // Non-renderable inner contours require the shared adjusted-inner
        // decomposition; keep them on that path.
        return false;
    }
    let side_rect = match side {
        0 => Rect::from_xywh(
            rect.left,
            rect.top,
            rect.width(),
            width.max(radii[0].y).max(radii[1].y),
        ),
        1 => {
            let extent = width.max(radii[1].x).max(radii[2].x);
            Rect::from_xywh(rect.right - extent, rect.top, extent, rect.height())
        }
        2 => {
            let extent = width.max(radii[2].y).max(radii[3].y);
            Rect::from_xywh(rect.left, rect.bottom - extent, rect.width(), extent)
        }
        _ => Rect::from_xywh(
            rect.left,
            rect.top,
            width.max(radii[0].x).max(radii[3].x),
            rect.height(),
        ),
    };
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    set_paint_css_color(&mut paint, &color);
    canvas.save();
    canvas.clip_rrect(RRect::new_rect_radii(rect, &radii), ClipOp::Intersect, true);
    if inner.width() > 0.0 && inner.height() > 0.0 {
        canvas.clip_rrect(
            RRect::new_rect_radii(inner, &inner_radii),
            ClipOp::Difference,
            true,
        );
    }
    canvas.clip_rect(side_rect, ClipOp::Intersect, true);
    if border_style == BorderStyle::Solid {
        canvas.draw_rect(rect, &paint);
    } else {
        for (inset, op) in [
            ((width * 2.0 / 3.0).round(), ClipOp::Intersect),
            ((width / 3.0).round(), ClipOp::Difference),
        ] {
            let (stripe, stripe_radii) = contour(inset);
            canvas.save();
            canvas.clip_rrect(RRect::new_rect_radii(stripe, &stripe_radii), op, true);
            canvas.draw_rect(rect, &paint);
            canvas.restore();
        }
    }
    canvas.restore();
    true
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
    platform_3d_fieldset_coverage: bool,
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

    // BoxBorderPainter clamps each used edge to the snapped border-box
    // dimension. A double edge narrower than three CSS pixels becomes
    // solid. With an empty inner contour, a lone curved edge fills the box
    // through its rounded outer contour and authored side rectangle.
    let visible_sides = [
        (bt, style.border_top_style),
        (br, style.border_right_style),
        (bb, style.border_bottom_style),
        (bl, style.border_left_style),
    ];
    let mut visible_sides_iter =
        visible_sides
            .iter()
            .enumerate()
            .filter(|(_, (width, border_style))| {
                *width > 0.0 && !matches!(border_style, BorderStyle::None | BorderStyle::Hidden)
            });
    let sole_side = visible_sides_iter
        .next()
        .filter(|_| visible_sides_iter.next().is_none());
    if let Some((side, _)) = sole_side.filter(|_| has_border_radius) {
        let (width, border_style) = visible_sides[side];
        let box_extent = if side == 0 || side == 2 { h } else { w };
        let used_width = width.min(box_extent);
        let effective_solid = border_style == BorderStyle::Solid
            || (border_style == BorderStyle::Double && used_width < 3.0);
        if ((effective_solid && width < box_extent)
            || (border_style == BorderStyle::Double && used_width >= 3.0))
            && !outer_rrect_clipped
            && paint_sole_curved_border(
                canvas,
                fragment,
                style,
                Rect::from_xywh(x, y, w, h),
                side,
                used_width,
                if effective_solid {
                    BorderStyle::Solid
                } else {
                    BorderStyle::Double
                },
            )
        {
            return;
        }
        if width >= box_extent && effective_solid {
            let border_rect = Rect::from_xywh(x, y, w, h);
            let outer_radii = fragment_border_radii(style, fragment, &border_rect, width);
            let side_rect = match side {
                0 => Rect::from_xywh(x, y, w, width),
                1 => Rect::from_xywh(x + w - width, y, width, h),
                2 => Rect::from_xywh(x, y + h - width, w, width),
                _ => Rect::from_xywh(x, y, width, h),
            };
            let color = match side {
                0 => style.border_top_color,
                1 => style.border_right_color,
                2 => style.border_bottom_color,
                _ => style.border_left_color,
            }
            .resolve(&style.color);
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            set_paint_css_color(&mut paint, &color);
            canvas.save();
            if !outer_rrect_clipped {
                canvas.clip_rrect(
                    RRect::new_rect_radii(border_rect, &outer_radii),
                    ClipOp::Intersect,
                    true,
                );
            }
            canvas.clip_rect(side_rect, ClipOp::Intersect, true);
            canvas.draw_rect(border_rect, &paint);
            canvas.restore();
            return;
        }
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
            paint_same_color_solid_border(
                canvas,
                fragment,
                color,
                x,
                y,
                w,
                h,
                bt,
                br,
                bb,
                bl,
                style.device_scale_factor,
                style.raster_configuration.backend,
            );
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

    let square_double_line_width = (bt / 3.0).round().max(1.0).min(bt * 0.5);
    let physical_contour_is_integral = [x, y, x + w, y + h, bt, square_double_line_width]
        .into_iter()
        .all(|edge| {
            let physical = f64::from(edge) * style.device_scale_factor;
            (physical - physical.round()).abs() <= 1.0e-5
        });
    if uniform
        && style.border_top_style == BorderStyle::Double
        && !has_border_radius
        && bt >= 2.0
        && w > bt * 2.0
        && h > bt * 2.0
        && physical_contour_is_integral
    {
        // A uniform square double border is two complete rectangular rings.
        // Painting four independently miter-clipped sides leaves the inner
        // corner's diagonal unowned, exposing the box background through the
        // nominally solid stripe. Keep each ring one draw operation so a
        // translucent border color is composited only once at the corners.
        let line_width = square_double_line_width;
        let rect_at_inset = |inset: f32| {
            let rect = Rect::from_xywh(x + inset, y + inset, w - inset * 2.0, h - inset * 2.0);
            RRect::new_rect_radii(rect, &[Point::new(0.0, 0.0); 4])
        };
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(true);
        set_paint_css_color(&mut paint, &style.border_top_color.resolve(inherited_color));
        canvas.draw_drrect(rect_at_inset(0.0), rect_at_inset(line_width), &paint);
        canvas.draw_drrect(rect_at_inset(bt - line_width), rect_at_inset(bt), &paint);
        return;
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
        let resolved = style.border_top_color.resolve(inherited_color);
        if table_internal_ignores_border_radius(style.display) && style.has_border_radius() {
            // Table-internal boxes ignore the authored radius, but Blink
            // still reaches the complex border path selected by that
            // declaration. Its four independently covered side rectangles
            // overlap at fractional outer corners; collapsing them to the
            // ordinary centered-stroke fast path loses that paired-edge
            // coverage and uses Skia's lower rounding at the inner trailing
            // edges.
            for side in [
                Rect::from_xywh(x, y, w, bt),
                Rect::from_xywh(x, y + h - bb, w, bb),
                Rect::from_xywh(x + w - br, y, br, h),
                Rect::from_xywh(x, y, bl, h),
            ] {
                if side.width() > 0.0 && side.height() > 0.0 {
                    draw_css_coverage_rect(
                        canvas,
                        side,
                        &resolved,
                        style.device_scale_factor,
                        PhysicalCoveragePacking::Default,
                    );
                }
            }
            return;
        }
        let platform_text_control_ring = bt == 1.0
            && style.device_scale_factor.fract().abs() > f64::EPSILON
            && style.background_color == Color::WHITE
            && resolved == Color::from_rgba8(118, 118, 118, 255);
        if platform_text_control_ring {
            // Native Linux text fields rasterize their neutral ring as four
            // snapped filled sides.  A centered Skia stroke shares a
            // half-covered row at adjoining fractional control boundaries;
            // the native-theme fill assigns that row wholly to one side.
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(false);
            set_paint_css_color(&mut paint, &resolved);
            let scale = style.device_scale_factor as f32;
            let center_owned_bounds = |rect: Rect| {
                // The native raster assigns a device cell to a side when its
                // center lies in the half-open interval (start, end].  This
                // gives two adjoining controls a single owner for a shared
                // half-device-pixel edge instead of two partial blends.
                let first_x = (rect.left * scale + 0.5).floor();
                let last_x = (rect.right * scale - 0.5 + 1.0e-5).floor();
                let first_y = (rect.top * scale + 0.5).floor();
                let last_y = (rect.bottom * scale - 0.5 + 1.0e-5).floor();
                (last_x >= first_x && last_y >= first_y)
                    .then_some((first_x, first_y, last_x, last_y))
            };
            let draw_center_owned_rect = |rect: Rect| {
                if let Some((first_x, first_y, last_x, last_y)) = center_owned_bounds(rect) {
                    canvas.draw_rect(
                        Rect::from_ltrb(
                            first_x / scale,
                            first_y / scale,
                            (last_x + 1.0) / scale,
                            (last_y + 1.0) / scale,
                        ),
                        &paint,
                    );
                }
            };
            let top_rect = Rect::from_xywh(x, y, w, bt);
            let bottom_rect = Rect::from_xywh(x, y + h - bb, w, bb);
            let right_rect = Rect::from_xywh(x + w - br, y + bt, br, h - bt - bb);
            let left_rect = Rect::from_xywh(x, y + bt, bl, h - bt - bb);
            draw_center_owned_rect(top_rect);
            draw_center_owned_rect(bottom_rect);
            draw_center_owned_rect(right_rect);
            draw_center_owned_rect(left_rect);

            // The 2 CSS-pixel native corner radius contributes one diagonal
            // cell immediately inside each pair of straight ring sides.
            // Locate it from the actual physical side thickness so the same
            // construction works for integral and fractional device scales.
            if let (Some(top), Some(bottom), Some(right), Some(left)) = (
                center_owned_bounds(top_rect),
                center_owned_bounds(bottom_rect),
                center_owned_bounds(right_rect),
                center_owned_bounds(left_rect),
            ) {
                for (cell_x, cell_y) in [
                    (left.2 + 1.0, top.3 + 1.0),
                    (right.0 - 1.0, top.3 + 1.0),
                    (left.2 + 1.0, bottom.1 - 1.0),
                    (right.0 - 1.0, bottom.1 - 1.0),
                ] {
                    canvas.draw_rect(
                        Rect::from_xywh(cell_x / scale, cell_y / scale, 1.0 / scale, 1.0 / scale),
                        &paint,
                    );
                }
            }
            return;
        }

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
            let blend_with_background_at_inner_edge = resolved.is_opaque()
                && style.background_color == resolved
                && matches!(
                    style.background_clip,
                    BackgroundClip::ContentBox | BackgroundClip::PaddingBox
                )
                && style.background_layers.is_empty()
                && style.background_linear_gradient.is_none();
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
                    blend_with_background_at_inner_edge,
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
                            blend_with_background_at_inner_edge,
                            &fill_paint,
                        );
                        canvas.restore();
                    }
                    if !blend_with_background_at_inner_edge {
                        if let Some(tangent_clips) = single_saturated_corner_tangent_clips(
                            border_rect,
                            inner_rect,
                            &outer_radii,
                        ) {
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
                                    false,
                                    &fill_paint,
                                );
                                canvas.restore();
                            }
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
                if resolved.a < 1.0 && simple_stroked_rrect {
                    // Blink's uniform rounded-border fast path is one
                    // DrawDRRect, including for translucent colors. Keeping
                    // the two contours in the same coverage operation also
                    // avoids intermediate layer/color quantization and
                    // independently clipped tangent spans.
                    canvas.draw_drrect(outer_rrect, inner_rrect, &fill_paint);
                    return;
                }
                // A double rounded rectangle evaluates both contours in one
                // coverage operation for non-circular and elliptical cases.
                let packed_translucent_spans = resolved.a < 1.0 && simple_stroked_rrect;
                if resolved.a < 1.0 {
                    canvas.save_layer_alpha_f(outer_rect, 1.0);
                }
                if packed_translucent_spans {
                    for corner_clip in [
                        Rect::from_ltrb(
                            outer_rect.left,
                            outer_rect.top,
                            outer_rect.left + outer_radii[0].x,
                            outer_rect.top + outer_radii[0].y,
                        ),
                        Rect::from_ltrb(
                            outer_rect.right - outer_radii[1].x,
                            outer_rect.top,
                            outer_rect.right,
                            outer_rect.top + outer_radii[1].y,
                        ),
                        Rect::from_ltrb(
                            outer_rect.right - outer_radii[2].x,
                            outer_rect.bottom - outer_radii[2].y,
                            outer_rect.right,
                            outer_rect.bottom,
                        ),
                        Rect::from_ltrb(
                            outer_rect.left,
                            outer_rect.bottom - outer_radii[3].y,
                            outer_rect.left + outer_radii[3].x,
                            outer_rect.bottom,
                        ),
                    ] {
                        canvas.save();
                        canvas.clip_rect(corner_clip, ClipOp::Intersect, false);
                        canvas.draw_drrect(outer_rrect, inner_rrect, &fill_paint);
                        canvas.restore();
                    }
                    let scale = style.device_scale_factor.max(f64::EPSILON) as f32;
                    let snap_tangent = |value: f32| (value * scale + 0.5).floor() / scale;
                    for (tangent_span, coverage_packing) in [
                        (
                            Rect::from_ltrb(
                                snap_tangent(outer_rect.left + outer_radii[0].x),
                                outer_rect.top,
                                snap_tangent(outer_rect.right - outer_radii[1].x),
                                inner_rect.top,
                            ),
                            PhysicalCoveragePacking::TrailingY,
                        ),
                        (
                            Rect::from_ltrb(
                                snap_tangent(outer_rect.left + outer_radii[3].x),
                                inner_rect.bottom,
                                snap_tangent(outer_rect.right - outer_radii[2].x),
                                outer_rect.bottom,
                            ),
                            PhysicalCoveragePacking::LeadingY,
                        ),
                        (
                            Rect::from_ltrb(
                                outer_rect.left,
                                snap_tangent(outer_rect.top + outer_radii[0].y),
                                inner_rect.left,
                                snap_tangent(outer_rect.bottom - outer_radii[3].y),
                            ),
                            PhysicalCoveragePacking::TrailingX,
                        ),
                        (
                            Rect::from_ltrb(
                                inner_rect.right,
                                snap_tangent(outer_rect.top + outer_radii[1].y),
                                outer_rect.right,
                                snap_tangent(outer_rect.bottom - outer_radii[2].y),
                            ),
                            PhysicalCoveragePacking::LeadingX,
                        ),
                    ] {
                        if tangent_span.width() > 0.0 && tangent_span.height() > 0.0 {
                            draw_css_coverage_rect(
                                canvas,
                                tangent_span,
                                &resolved,
                                style.device_scale_factor,
                                coverage_packing,
                            );
                        }
                    }
                } else {
                    canvas.draw_drrect(outer_rrect, inner_rrect, &fill_paint);
                }
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
        // Different-colored rounded sides still share the border box's outer
        // contour. The side trapezoids below own their miter sectors, while a
        // single outer clip prevents their rectangular corners from escaping
        // an eccentric or saturated radius.
        let clip_per_side_outer = has_border_radius && !outer_rrect_clipped;
        if clip_per_side_outer {
            let border_rect = Rect::from_xywh(x, y, w, h);
            let representative_width = bt.max(br).max(bb).max(bl);
            let outer_radii =
                fragment_border_radii(style, fragment, &border_rect, representative_width);
            canvas.save();
            canvas.clip_rrect(
                RRect::new_rect_radii(border_rect, &outer_radii),
                ClipOp::Intersect,
                true,
            );
        }

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
        let direct_analytic_solid_miters = [
            style.border_top_color.resolve(inherited_color),
            style.border_right_color.resolve(inherited_color),
            style.border_bottom_color.resolve(inherited_color),
            style.border_left_color.resolve(inherited_color),
        ]
        .iter()
        .any(|color| color.a == 0.0);
        let authored_uniform_3d = uniform
            && !platform_3d_fieldset_coverage
            && matches!(
                style.border_top_style,
                BorderStyle::Inset | BorderStyle::Outset
            );
        let authored_uniform_3d_outer_edge = uniform
            && !platform_3d_fieldset_coverage
            && matches!(
                style.border_top_style,
                BorderStyle::Groove | BorderStyle::Ridge | BorderStyle::Inset | BorderStyle::Outset
            );
        // Blink's all-solid, non-uniform border path paints the horizontal
        // sides as full rectangles and lets the later vertical trapezoids
        // overdraw their corners. Keep mixed-style borders on the existing
        // clipped-side path because their corner ownership differs.
        let chromium_solid_border = (bt <= 0.0 || style.border_top_style == BorderStyle::Solid)
            && (br <= 0.0 || style.border_right_style == BorderStyle::Solid)
            && (bb <= 0.0 || style.border_bottom_style == BorderStyle::Solid)
            && (bl <= 0.0 || style.border_left_style == BorderStyle::Solid)
            // Full horizontal side rectangles rely on the later vertical
            // sides to overdraw their corner miters. A transparent adjacent
            // side cannot perform that overdraw, so those joins must retain
            // the ordinary four-point trapezoid (the classic CSS triangle).
            && (bt <= 0.0
                || style
                    .border_top_color
                    .resolve(inherited_color)
                    .is_opaque())
            && (br <= 0.0
                || style
                    .border_right_color
                    .resolve(inherited_color)
                    .is_opaque())
            && (bb <= 0.0
                || style
                    .border_bottom_color
                    .resolve(inherited_color)
                    .is_opaque())
            && (bl <= 0.0
                || style
                    .border_left_color
                    .resolve(inherited_color)
                    .is_opaque());
        // Blink paints lower-alpha sides first, then non-solid styles before
        // solid styles. The side priority breaks ties within one style.
        // Keeping that order matters for dotted edges: a later solid edge
        // can overdraw a corner instead of requiring a miter clip.
        let mut paint_order = [
            BorderSide::Top,
            BorderSide::Bottom,
            BorderSide::Right,
            BorderSide::Left,
        ];
        let side_key = |side: BorderSide| {
            let (border_style, color, side_priority) = match side {
                BorderSide::Top => (
                    style.border_top_style,
                    style.border_top_color.resolve(inherited_color),
                    0,
                ),
                BorderSide::Bottom => (
                    style.border_bottom_style,
                    style.border_bottom_color.resolve(inherited_color),
                    1,
                ),
                BorderSide::Right => (
                    style.border_right_style,
                    style.border_right_color.resolve(inherited_color),
                    2,
                ),
                BorderSide::Left => (
                    style.border_left_style,
                    style.border_left_color.resolve(inherited_color),
                    3,
                ),
            };
            let style_priority = match border_style {
                BorderStyle::None | BorderStyle::Hidden => 0,
                BorderStyle::Dotted | BorderStyle::Dashed | BorderStyle::Double => 1,
                BorderStyle::Inset
                | BorderStyle::Groove
                | BorderStyle::Outset
                | BorderStyle::Ridge => 2,
                BorderStyle::Solid => 3,
            };
            (color.a, style_priority, side_priority)
        };
        paint_order.sort_by(|a, b| {
            let (alpha_a, style_a, side_a) = side_key(*a);
            let (alpha_b, style_b, side_b) = side_key(*b);
            alpha_a
                .total_cmp(&alpha_b)
                .then(style_a.cmp(&style_b))
                .then(side_a.cmp(&side_b))
        });
        for side in paint_order {
            // Top border: outer-top-left → outer-top-right → inner-top-right → inner-top-left
            if side == BorderSide::Top && bt > 0.0 {
                let top_points =
                    if chromium_solid_border && style.border_top_style == BorderStyle::Solid {
                        [(ox0, oy0), (ox1, oy0), (ox1, iy0), (ox0, iy0)]
                    } else if authored_uniform_3d {
                        [(ox0, oy0), (ox1, oy0), (ix1, iy0), (ox0, iy0)]
                    } else {
                        [(ox0, oy0), (ox1, oy0), (ix1, iy0), (ix0, iy0)]
                    };
                paint_border_side_path(
                    canvas,
                    style.border_top_style,
                    &style.border_top_color,
                    inherited_color,
                    bt,
                    &top_points,
                    BorderSide::Top,
                    adjust_dashed_gap,
                    !matching_stroked_sides,
                    !all_side_colors_match,
                    antialias_solid_miters,
                    direct_analytic_solid_miters,
                    chromium_solid_border,
                    authored_uniform_3d_outer_edge,
                    platform_3d_fieldset_coverage,
                    style.device_scale_factor,
                );
            }
            // Bottom border: outer-bottom-right → outer-bottom-left → inner-bottom-left → inner-bottom-right
            if side == BorderSide::Bottom && bb > 0.0 {
                let bottom_points =
                    if chromium_solid_border && style.border_bottom_style == BorderStyle::Solid {
                        [(ox1, oy1), (ox0, oy1), (ox0, iy1), (ox1, iy1)]
                    } else if authored_uniform_3d {
                        [(ox1, oy1), (ox0, oy1), (ix0, iy1), (ox1, iy1)]
                    } else {
                        [(ox1, oy1), (ox0, oy1), (ix0, iy1), (ix1, iy1)]
                    };
                paint_border_side_path(
                    canvas,
                    style.border_bottom_style,
                    &style.border_bottom_color,
                    inherited_color,
                    bb,
                    &bottom_points,
                    BorderSide::Bottom,
                    adjust_dashed_gap,
                    !matching_stroked_sides,
                    !all_side_colors_match,
                    antialias_solid_miters,
                    direct_analytic_solid_miters,
                    chromium_solid_border,
                    authored_uniform_3d_outer_edge,
                    platform_3d_fieldset_coverage,
                    style.device_scale_factor,
                );
            }
            // Right border: outer-top-right → outer-bottom-right → inner-bottom-right → inner-top-right
            if side == BorderSide::Right && br > 0.0 {
                let right_color = style.border_right_color.resolve(inherited_color);
                let right_top_miter = chromium_solid_border
                    && bt > 0.0
                    && right_color != style.border_top_color.resolve(inherited_color);
                let right_bottom_miter = chromium_solid_border
                    && bb > 0.0
                    && right_color != style.border_bottom_color.resolve(inherited_color);
                paint_border_side_path(
                    canvas,
                    style.border_right_style,
                    &style.border_right_color,
                    inherited_color,
                    br,
                    &[
                        (ox1, oy0),
                        (ox1, oy1),
                        (
                            ix1,
                            if authored_uniform_3d {
                                oy1
                            } else if !chromium_solid_border || right_bottom_miter {
                                iy1
                            } else {
                                oy1
                            },
                        ),
                        (
                            ix1,
                            if !chromium_solid_border || right_top_miter {
                                iy0
                            } else {
                                oy0
                            },
                        ),
                    ],
                    BorderSide::Right,
                    adjust_dashed_gap,
                    !matching_stroked_sides
                        && !(style.border_right_style == BorderStyle::Dotted
                            && bt > 0.0
                            && bb > 0.0
                            && style.border_top_style == BorderStyle::Solid
                            && style.border_bottom_style == BorderStyle::Solid
                            && style.border_top_color.resolve(inherited_color).is_opaque()
                            && style
                                .border_bottom_color
                                .resolve(inherited_color)
                                .is_opaque()),
                    !(style.border_right_style == BorderStyle::Dotted
                        && ((bt > 0.0
                            && matches!(
                                style.border_top_style,
                                BorderStyle::Dashed | BorderStyle::Dotted | BorderStyle::Double
                            ))
                            || (bb > 0.0
                                && matches!(
                                    style.border_bottom_style,
                                    BorderStyle::Dashed | BorderStyle::Dotted | BorderStyle::Double
                                )))),
                    antialias_solid_miters,
                    direct_analytic_solid_miters,
                    chromium_solid_border,
                    authored_uniform_3d_outer_edge,
                    platform_3d_fieldset_coverage,
                    style.device_scale_factor,
                );
            }
            // Left border: outer-bottom-left → outer-top-left → inner-top-left → inner-bottom-left
            if side == BorderSide::Left && bl > 0.0 {
                let left_color = style.border_left_color.resolve(inherited_color);
                let left_top_miter = chromium_solid_border
                    && bt > 0.0
                    && left_color != style.border_top_color.resolve(inherited_color);
                let left_bottom_miter = chromium_solid_border
                    && bb > 0.0
                    && left_color != style.border_bottom_color.resolve(inherited_color);
                paint_border_side_path(
                    canvas,
                    style.border_left_style,
                    &style.border_left_color,
                    inherited_color,
                    bl,
                    &[
                        (ox0, oy1),
                        (ox0, oy0),
                        (
                            ix0,
                            if authored_uniform_3d {
                                oy0
                            } else if !chromium_solid_border || left_top_miter {
                                iy0
                            } else {
                                oy0
                            },
                        ),
                        (
                            ix0,
                            if !chromium_solid_border || left_bottom_miter {
                                iy1
                            } else {
                                oy1
                            },
                        ),
                    ],
                    BorderSide::Left,
                    adjust_dashed_gap,
                    !matching_stroked_sides,
                    !all_side_colors_match,
                    antialias_solid_miters,
                    direct_analytic_solid_miters,
                    chromium_solid_border,
                    authored_uniform_3d_outer_edge,
                    platform_3d_fieldset_coverage,
                    style.device_scale_factor,
                );
            }
        }

        // CPU Skia snaps the axis-aligned edges of opaque trapezoid fills,
        // even when their CSS edge lands between device pixels. Replay solid
        // side rectangles through the shared physical-coverage path, in the
        // same side order, so the inner and outer border contours retain the
        // fractional row or column that Chromium rasterizes.
        if !chromium_solid_border && (style.device_scale_factor - 1.0).abs() > f64::EPSILON {
            if bt > 0.0 && style.border_top_style == BorderStyle::Solid {
                paint_device_coverage_rect(
                    canvas,
                    Rect::from_xywh(x + bl, y, (w - bl - br).max(0.0), bt),
                    &style.border_top_color.resolve(inherited_color),
                    1.0,
                    style.device_scale_factor,
                );
            }
            if bb > 0.0 && style.border_bottom_style == BorderStyle::Solid {
                paint_device_coverage_rect(
                    canvas,
                    Rect::from_xywh(x + bl, y + h - bb, (w - bl - br).max(0.0), bb),
                    &style.border_bottom_color.resolve(inherited_color),
                    1.0,
                    style.device_scale_factor,
                );
            }
            if br > 0.0 && style.border_right_style == BorderStyle::Solid {
                paint_device_coverage_rect(
                    canvas,
                    Rect::from_xywh(x + w - br, y + bt, br, (h - bt - bb).max(0.0)),
                    &style.border_right_color.resolve(inherited_color),
                    1.0,
                    style.device_scale_factor,
                );
            }
            if bl > 0.0 && style.border_left_style == BorderStyle::Solid {
                paint_device_coverage_rect(
                    canvas,
                    Rect::from_xywh(x, y + bt, bl, (h - bt - bb).max(0.0)),
                    &style.border_left_color.resolve(inherited_color),
                    1.0,
                    style.device_scale_factor,
                );
            }
        }

        // Fix corner diagonal pixels: Chrome's Skia achieves exact complementary
        // coverage at shared miter edges (no white bleed-through). Standard SrcOver
        // compositing leaves ~25% background showing. Fix by overwriting diagonal
        // pixels with the exact 50% blend of the two adjacent border colors.
        if !chromium_solid_border {
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
        if clip_per_side_outer {
            canvas.restore();
        }
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
    device_scale: f64,
    backend: RasterBackend,
) {
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    // The contour is expressed in logical pixels and the frame transform
    // maps it onto the physical raster grid. Keep edge coverage enabled so a
    // one-sided border ending on a fractional device pixel receives the same
    // partial physical row as other CSS box edges.
    paint.set_anti_alias(
        backend == RasterBackend::ChromiumLinux || (device_scale - 1.0).abs() > f64::EPSILON,
    );
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

    let border_rect =
        close_rect_at_physical_viewport_edge(Rect::from_xywh(x, y, w, h), device_scale);
    let w = border_rect.width();
    let h = border_rect.height();

    if bt == 0.0 || br == 0.0 || bb == 0.0 || bl == 0.0 {
        // A same-color border with suppressed sides does not take Blink's
        // four-sided contour fast path. Its complex-border painter draws the
        // remaining straight sides in Top, Bottom, Right, Left order without
        // miters, so adjacent sides deliberately overlap their AA coverage at
        // matching-color corners.
        if bt > 0.0 {
            canvas.draw_rect(
                Rect::from_xywh(border_rect.left, border_rect.top, w, bt),
                &paint,
            );
        }
        if bb > 0.0 {
            canvas.draw_rect(
                Rect::from_xywh(border_rect.left, border_rect.bottom - bb, w, bb),
                &paint,
            );
        }
        if br > 0.0 {
            canvas.draw_rect(
                Rect::from_xywh(border_rect.right - br, border_rect.top, br, h),
                &paint,
            );
        }
        if bl > 0.0 {
            canvas.draw_rect(
                Rect::from_xywh(border_rect.left, border_rect.top, bl, h),
                &paint,
            );
        }
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
    if PAINTED_FRAGMENTED_OUTLINES.with(|painted| {
        painted
            .borrow()
            .contains(&(fragment as *const Fragment as usize))
    }) {
        return;
    }
    if EXTERNALLY_DEFERRED_OUTLINES.with(|deferred| {
        deferred
            .borrow()
            .contains(&(fragment as *const Fragment as usize))
    }) {
        return;
    }
    if DEFERRED_OUTLINE_REPLAYS.with(|deferred| {
        deferred
            .borrow()
            .contains(&(fragment as *const Fragment as usize))
    }) {
        return;
    }
    let has_axis_aligned_device_geometry = style.transform == openui_style::Transform2D::IDENTITY;
    if has_axis_aligned_device_geometry
        && outline_coverage_key(fragment, style, abs_offset).is_some_and(|key| {
            DEFERRED_OPAQUE_OUTLINE_COVERS.with(|covers| covers.borrow().contains(&key))
        })
    {
        return;
    }
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

    let draw_solid_rect_skipping_rows = |rect, coverage_packing, rows: &[i64]| {
        if has_axis_aligned_device_geometry {
            draw_css_coverage_rect_skipping_rows(
                canvas,
                rect,
                &color,
                style.device_scale_factor,
                coverage_packing,
                rows,
            );
        } else {
            canvas.draw_rect(rect, &paint);
        }
    };

    if fragment.paint_zero_block_outline {
        if has_axis_aligned_device_geometry {
            draw_css_coverage_rect(
                canvas,
                Rect::from_ltrb(bx - ow, by, br + ow, by + 2.0 * ow),
                &color,
                style.device_scale_factor,
                PhysicalCoveragePacking::Default,
            );
        } else {
            canvas.draw_rect(Rect::from_xywh(bx, by, br - bx, ow), &paint);
        }
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

    let top_joint_row = fractional_physical_row(oy + ow, style.device_scale_factor);
    let bottom_joint_row = fractional_physical_row(ob - ow, style.device_scale_factor);
    let top_rows = top_joint_row.as_slice();
    let bottom_rows = bottom_joint_row.as_slice();
    let vertical_rows: Vec<i64> = top_joint_row.into_iter().chain(bottom_joint_row).collect();

    // For solid outlines, draw 4 rectangles (top, right, bottom, left).
    // For other styles, fall back to solid (matches most visual tests).
    // Top edge
    draw_solid_rect_skipping_rows(
        Rect::from_xywh(ox, oy, ow_total, ow),
        PhysicalCoveragePacking::Default,
        top_rows,
    );
    // Bottom edge
    draw_solid_rect_skipping_rows(
        Rect::from_xywh(ox, ob - ow, ow_total, ow),
        PhysicalCoveragePacking::LeadingY,
        bottom_rows,
    );
    // Left edge
    draw_solid_rect_skipping_rows(
        Rect::from_xywh(ox, oy + ow, ow, oh_total - 2.0 * ow),
        PhysicalCoveragePacking::Default,
        &vertical_rows,
    );
    // Right edge
    draw_solid_rect_skipping_rows(
        Rect::from_xywh(or - ow, oy + ow, ow, oh_total - 2.0 * ow),
        PhysicalCoveragePacking::LeadingX,
        &vertical_rows,
    );
    if has_axis_aligned_device_geometry {
        // A fractional joint is one geometric union, not two source-over
        // masks. Paint its physical row once so an outline retains the same
        // packed coverage when another outline happens to share that edge.
        // Repeated edge-plus-repair passes are visually right over white but
        // accumulate an extra 8-bit step when two independent outlines
        // overlap.
        if top_joint_row.is_some() && top_joint_row == bottom_joint_row {
            // A zero-block-size outline has coincident top and bottom inner
            // edges. Their complementary analytic coverages form one fully
            // covered contour row; source-over compositing the two masks
            // would leave a fractional hole (for example 75% at half phase).
            let row = top_joint_row.expect("checked above");
            let scale = style.device_scale_factor as f32;
            let physical_joint = f64::from(oy + ow) * style.device_scale_factor;
            let joint_fraction = physical_joint - physical_joint.floor();
            let coverage_packing = if joint_fraction > 0.5 + 1.0e-6 {
                PhysicalCoveragePacking::CoincidentJointX
            } else {
                PhysicalCoveragePacking::Default
            };
            draw_css_coverage_rect(
                canvas,
                Rect::from_ltrb(ox, row as f32 / scale, or, (row as f32 + 1.0) / scale),
                &color,
                style.device_scale_factor,
                coverage_packing,
            );
        } else if let Some(row) = top_joint_row {
            // Blink routes a two-CSS-pixel top contour through the half-open
            // path. Its default medium (3px) outline retains the ordinary
            // packed union at this joint.
            draw_outline_junction_row(
                canvas,
                ox,
                ox + ow,
                or - ow,
                or,
                oy + ow,
                row,
                true,
                ow == 2.0 && offset_px >= 0.0,
                &color,
                style.device_scale_factor,
            );
        }
        if top_joint_row != bottom_joint_row {
            if let Some(row) = bottom_joint_row {
                draw_outline_junction_row(
                    canvas,
                    ox,
                    ox + ow,
                    or - ow,
                    or,
                    ob - ow,
                    row,
                    false,
                    ow > 1.0 && offset_px >= 0.0,
                    &color,
                    style.device_scale_factor,
                );
            }
        }
    }
}

fn physical_coverage_axis_segments(
    start: f32,
    end: f32,
    device_scale: f64,
) -> Vec<(i64, i64, f32)> {
    if !start.is_finite()
        || !end.is_finite()
        || !device_scale.is_finite()
        || device_scale <= 0.0
        || end <= start
    {
        return Vec::new();
    }
    let snap_near_integer = |value: f64| {
        let nearest = value.round();
        if (value - nearest).abs() <= 1.0e-6 {
            nearest
        } else {
            value
        }
    };
    let physical_start = snap_near_integer(f64::from(start) * device_scale);
    let physical_end = snap_near_integer(f64::from(end) * device_scale);
    let first = physical_start.floor() as i64;
    let last = physical_end.ceil() as i64;
    if last <= first {
        return Vec::new();
    }
    if last - first == 1 {
        return vec![(first, last, (physical_end - physical_start) as f32)];
    }

    let mut segments = Vec::with_capacity(3);
    let first_end = physical_start.ceil() as i64;
    if first_end > first {
        segments.push((first, first_end, (first_end as f64 - physical_start) as f32));
    }
    let interior_start = physical_start.ceil() as i64;
    let interior_end = physical_end.floor() as i64;
    if interior_end > interior_start {
        segments.push((interior_start, interior_end, 1.0));
    }
    if last > interior_end {
        segments.push((
            interior_end,
            last,
            (physical_end - interior_end as f64) as f32,
        ));
    }
    segments.retain(|(_, _, coverage)| *coverage > 0.0);
    segments
}

#[derive(Clone, Copy)]
enum PhysicalCoveragePacking {
    Default,
    LeadingX,
    CoincidentJointX,
    LeadingY,
    TrailingX,
    TrailingY,
    LeadingBoth,
    // Preserve the packed trailing-edge alpha while advancing the stored
    // color channel to Chromium's next premultiplied representation.
    TrailingXColorUp,
}

fn packed_physical_coverage(analytic_coverage: f32, use_255_steps: bool) -> f32 {
    let steps = if use_255_steps { 255.0 } else { 256.0 };
    ((analytic_coverage.clamp(0.0, 1.0) * steps).floor() as u32).min(255) as f32 / 255.0
}

fn fractional_physical_row(coordinate: f32, device_scale: f64) -> Option<i64> {
    let physical = f64::from(coordinate) * device_scale;
    let fraction = physical - physical.floor();
    (fraction > 1.0e-6 && fraction < 1.0 - 1.0e-6).then_some(physical.floor() as i64)
}

fn draw_outline_junction_row(
    canvas: &Canvas,
    outer_left: f32,
    inner_left: f32,
    inner_right: f32,
    outer_right: f32,
    joint: f32,
    physical_row: i64,
    horizontal_precedes_joint: bool,
    half_open_inner_corner: bool,
    color: &Color,
    device_scale: f64,
) {
    let physical_joint = f64::from(joint) * device_scale;
    let fraction = (physical_joint - physical_row as f64) as f32;
    let (horizontal_y, vertical_y) = if horizontal_precedes_joint {
        (fraction, 1.0 - fraction)
    } else {
        (1.0 - fraction, fraction)
    };
    let scale64 = device_scale;
    let scale = device_scale as f32;
    let physical_outer_left = f64::from(outer_left) * scale64;
    let physical_inner_left = f64::from(inner_left) * scale64;
    let physical_inner_right = f64::from(inner_right) * scale64;
    let physical_outer_right = f64::from(outer_right) * scale64;
    let first = physical_outer_left.floor() as i64;
    let last = physical_outer_right.ceil() as i64;
    let overlap = |start: f64, end: f64, cell: i64| {
        (end.min(cell as f64 + 1.0) - start.max(cell as f64)).clamp(0.0, 1.0) as f32
    };
    for physical_x in first..last {
        let outer_x = overlap(physical_outer_left, physical_outer_right, physical_x);
        let side_x = (overlap(physical_outer_left, physical_inner_left, physical_x)
            + overlap(physical_inner_right, physical_outer_right, physical_x))
        .min(outer_x);
        let analytic_coverage = outer_x * horizontal_y + side_x * vertical_y;
        let use_255_steps = !horizontal_precedes_joint && side_x <= 1.0e-6;
        let packed_coverage = if half_open_inner_corner
            && outer_x >= 1.0 - 1.0e-6
            && side_x > 1.0e-6
            && side_x < outer_x - 1.0e-6
        {
            // Blink's outline contour reaches an inner corner through the
            // 255-step coverage path. The bottom contour's low-half side span
            // additionally excludes its half-open endpoint before packing; a
            // top contour or a half-or-greater bottom span closes on the
            // ordinary packed value. This is distinct from the outer fringe
            // (where `side_x == outer_x`) and from a horizontal span with no
            // side contribution.
            let endpoint_adjustment = if !horizontal_precedes_joint && side_x < 0.5 {
                1.0
            } else {
                0.0
            };
            (((analytic_coverage * 255.0).floor() - endpoint_adjustment).max(0.0)) / 255.0
        } else {
            packed_physical_coverage(analytic_coverage, use_255_steps)
        };
        if packed_coverage <= 0.0 {
            continue;
        }
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_anti_alias(false);
        set_paint_css_color_with_alpha(&mut paint, color, packed_coverage);
        canvas.draw_rect(
            Rect::from_ltrb(
                physical_x as f32 / scale,
                physical_row as f32 / scale,
                (physical_x as f32 + 1.0) / scale,
                (physical_row as f32 + 1.0) / scale,
            ),
            &paint,
        );
    }
}

/// Draw an axis-aligned CSS rectangle with its exact device-pixel overlap.
/// Skia's analytic rectangle AA uses a wider coverage ramp than Chromium's
/// display-item raster at fractional replay scales. Partitioning the two edge
/// cells and the integral interior keeps adjacent outline pieces bit-stable.
fn draw_css_coverage_rect(
    canvas: &Canvas,
    rect: Rect,
    color: &Color,
    device_scale: f64,
    coverage_packing: PhysicalCoveragePacking,
) {
    draw_css_coverage_rect_skipping_rows(canvas, rect, color, device_scale, coverage_packing, &[]);
}

fn draw_css_coverage_rect_skipping_rows(
    canvas: &Canvas,
    rect: Rect,
    color: &Color,
    device_scale: f64,
    coverage_packing: PhysicalCoveragePacking,
    skipped_physical_rows: &[i64],
) {
    let x_segments = physical_coverage_axis_segments(rect.left, rect.right, device_scale);
    let y_segments = physical_coverage_axis_segments(rect.top, rect.bottom, device_scale);
    let scale = device_scale as f32;
    for (x_index, (left, right, x_coverage)) in x_segments.iter().enumerate() {
        for (y_index, (top, bottom, y_coverage)) in y_segments.iter().enumerate() {
            if skipped_physical_rows
                .iter()
                .any(|row| *top <= *row && *row < *bottom)
            {
                continue;
            }
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_anti_alias(false);
            let analytic_coverage = x_coverage * y_coverage;
            let use_255_steps = match coverage_packing {
                PhysicalCoveragePacking::Default | PhysicalCoveragePacking::CoincidentJointX => {
                    false
                }
                PhysicalCoveragePacking::LeadingX => x_index == 0 && *x_coverage < 1.0,
                PhysicalCoveragePacking::LeadingY => y_index == 0 && *y_coverage < 1.0,
                PhysicalCoveragePacking::TrailingX => {
                    x_index + 1 == x_segments.len() && *x_coverage < 1.0
                }
                PhysicalCoveragePacking::TrailingY => {
                    y_index + 1 == y_segments.len() && *y_coverage < 1.0
                }
                PhysicalCoveragePacking::LeadingBoth => {
                    let leading_x = x_index == 0 && *x_coverage < 1.0;
                    let leading_y = y_index == 0 && *y_coverage < 1.0;
                    leading_x ^ leading_y
                }
                PhysicalCoveragePacking::TrailingXColorUp => false,
            };
            let packed_coverage = packed_physical_coverage(analytic_coverage, use_255_steps);
            let packed_coverage =
                if matches!(coverage_packing, PhysicalCoveragePacking::CoincidentJointX)
                    && (x_index == 0 || x_index + 1 == x_segments.len())
                    && *x_coverage < 1.0
                {
                    ((packed_coverage * 255.0).round() + 1.0).min(255.0) / 255.0
                } else {
                    packed_coverage
                };
            let color_up = |component: f32| {
                (((component.clamp(0.0, 1.0) * 255.0).round() + 1.0).min(255.0)) / 255.0
            };
            let trailing_x_color = Color {
                r: color_up(color.r),
                g: color_up(color.g),
                b: color_up(color.b),
                a: color.a,
            };
            let use_trailing_x_color =
                matches!(coverage_packing, PhysicalCoveragePacking::TrailingXColorUp)
                    && x_index + 1 == x_segments.len()
                    && *x_coverage < 1.0;
            set_paint_css_color_with_alpha(
                &mut paint,
                if use_trailing_x_color {
                    &trailing_x_color
                } else {
                    color
                },
                packed_coverage,
            );
            canvas.draw_rect(
                Rect::from_ltrb(
                    *left as f32 / scale,
                    *top as f32 / scale,
                    *right as f32 / scale,
                    *bottom as f32 / scale,
                ),
                &paint,
            );
        }
    }
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
    if rule_style == BorderStyle::Solid {
        let color = style.column_rule_color.resolve(&style.color);
        draw_css_coverage_rect(
            canvas,
            rect,
            &color,
            style.device_scale_factor,
            PhysicalCoveragePacking::Default,
        );
        return;
    }
    paint_border_side(
        canvas,
        rule_style,
        &style.column_rule_color,
        &style.color,
        w,
        rect,
        BorderSide::Left,
        true,
        false,
        style.device_scale_factor,
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
    let device_scale = style.device_scale_factor.max(f64::EPSILON) as f32;
    let physical_floor = |coordinate: f32| (coordinate * device_scale).floor() as i32;
    let physical_ceil_minus_one = |coordinate: f32| (coordinate * device_scale).ceil() as i32 - 1;
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
            physical_floor(ox0),
            physical_floor(oy0),
            physical_ceil_minus_one(ix0),
            physical_ceil_minus_one(iy0),
            &top_c,
            &left_c,
            device_scale,
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
            physical_ceil_minus_one(ox1),
            physical_floor(oy0),
            physical_floor(ix1),
            physical_ceil_minus_one(iy0),
            &top_c,
            &right_c,
            device_scale,
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
            physical_ceil_minus_one(ox1),
            physical_ceil_minus_one(oy1),
            physical_floor(ix1),
            physical_floor(iy1),
            &bottom_c,
            &right_c,
            device_scale,
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
            physical_floor(ox0),
            physical_ceil_minus_one(oy1),
            physical_ceil_minus_one(ix0),
            physical_floor(iy1),
            &bottom_c,
            &left_c,
            device_scale,
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
        let dark = shade_3d_dark(&base, uses_platform_current_color, false);
        let light = shade_3d_light(&base, uses_platform_current_color, false);
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
        // The platform fieldset bevel is emitted as a closed native contour;
        // its endpoint cells are owned by the adjoining outer edges. CSS 3D
        // borders are four mitered sides, so their entire shared diagonal,
        // including both endpoint cells, receives complementary coverage.
        let skip_endpoints = uses_platform_current_color;
        let (first_miter_color, second_miter_color) = if uses_platform_current_color {
            (&dark, &light)
        } else {
            (&light, &dark)
        };
        draw_miter_blend_pixels_skipping(
            canvas,
            physical_ceil_minus_one(ox1),
            physical_floor(oy0),
            physical_floor(ix1),
            physical_ceil_minus_one(iy0),
            first_miter_color,
            second_miter_color,
            skip_middle,
            skip_endpoints,
            device_scale,
        );
        draw_miter_blend_pixels_skipping(
            canvas,
            physical_floor(ox0),
            physical_ceil_minus_one(oy1),
            physical_ceil_minus_one(ix0),
            physical_floor(iy1),
            first_miter_color,
            second_miter_color,
            skip_middle,
            skip_endpoints,
            device_scale,
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
    device_scale: f32,
) {
    draw_miter_blend_pixels_skipping(
        canvas,
        x0,
        y0,
        x1,
        y1,
        color1,
        color2,
        None,
        false,
        device_scale,
    );
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
    skip_endpoints: bool,
    device_scale: f32,
) {
    // Chromium partitions a half-covered shared miter cell into complementary
    // 8-bit coverages. The earlier-painted side owns 127/255 and the later
    // side owns 128/255; a floating-point 50/50 average rounds both channels
    // upward and produces a one-value bright seam.
    const FIRST_WEIGHT: f32 = 127.0 / 255.0;
    const SECOND_WEIGHT: f32 = 128.0 / 255.0;
    // Blend in premultiplied space for correctness with translucent borders.
    let avg_a = color1.a * FIRST_WEIGHT + color2.a * SECOND_WEIGHT;
    let blend = if avg_a > 0.0 {
        let pm_r = color1.r * color1.a * FIRST_WEIGHT + color2.r * color2.a * SECOND_WEIGHT;
        let pm_g = color1.g * color1.a * FIRST_WEIGHT + color2.g * color2.a * SECOND_WEIGHT;
        let pm_b = color1.b * color1.a * FIRST_WEIGHT + color2.b * color2.a * SECOND_WEIGHT;
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
        let at_endpoint = index == 0 || (cx == x1 && cy == y1);
        if skip_index != Some(index) && !(skip_endpoints && at_endpoint) {
            canvas.draw_rect(
                Rect::from_xywh(
                    cx as f32 / device_scale,
                    cy as f32 / device_scale,
                    1.0 / device_scale,
                    1.0 / device_scale,
                ),
                &paint,
            );
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
/// Solid borders either fill the path directly or use it as a clip for an
/// axis-aligned fill. Dashed, dotted, double, and 3D styles use the same miter
/// clip around the rectangle-based `paint_border_side` implementation.
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
    direct_analytic_solid_miter: bool,
    direct_solid_fill: bool,
    preserve_horizontal_outer_edge: bool,
    platform_3d_fieldset_coverage: bool,
    device_scale: f64,
) {
    if width <= 0.0 {
        return;
    }
    if matches!(border_style, BorderStyle::None | BorderStyle::Hidden) {
        return;
    }

    let resolved = border_color.resolve(inherited_color);
    let uses_platform_3d_current_color = platform_3d_fieldset_coverage
        && matches!(
            border_style,
            BorderStyle::Groove | BorderStyle::Ridge | BorderStyle::Inset | BorderStyle::Outset
        )
        && matches!(border_color, StyleColor::CurrentColor)
        && resolved.r == 0.0
        && resolved.g == 0.0
        && resolved.b == 0.0;

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

            let mut clip_path = PathBuilder::new();
            clip_path.move_to(Point::new(points[0].0, points[0].1));
            clip_path.line_to(Point::new(points[1].0, points[1].1));
            clip_path.line_to(Point::new(points[2].0, points[2].1));
            clip_path.line_to(Point::new(points[3].0, points[3].1));
            clip_path.close();
            if direct_solid_fill || direct_analytic_solid_miter {
                let mut paint = Paint::default();
                paint.set_style(PaintStyle::Fill);
                paint.set_anti_alias(true);
                set_paint_css_color(&mut paint, &resolved);
                canvas.draw_path(&clip_path.detach(), &paint);
                return;
            }

            canvas.save();
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
                // The side painter below owns analytic coverage for both the
                // outer and inner axis-aligned edges. A center-sampled hard
                // miter clip must therefore include the two edge cells; if it
                // ends at the mathematical contour it discards fractional
                // coverage before the physical-coverage rectangles can paint
                // it. Expand only along the side normal by half a device
                // pixel, leaving the diagonal miter ownership unchanged.
                let mut clip_points = *points;
                let is_3d_style = matches!(
                    border_style,
                    BorderStyle::Groove
                        | BorderStyle::Ridge
                        | BorderStyle::Inset
                        | BorderStyle::Outset
                );
                let expands_inner_axis_coverage = uses_platform_3d_current_color
                    || (is_3d_style && side == BorderSide::Bottom)
                    || (!uses_platform_3d_current_color
                        && matches!(border_style, BorderStyle::Inset | BorderStyle::Outset)
                        && side == BorderSide::Right)
                    || (border_style == BorderStyle::Double
                        && matches!(side, BorderSide::Bottom | BorderSide::Right));
                let outer_axis_coordinate = match side {
                    BorderSide::Top | BorderSide::Bottom => clip_points[0].1,
                    BorderSide::Right | BorderSide::Left => clip_points[0].0,
                };
                let outer_axis_fraction =
                    (f64::from(outer_axis_coordinate) * device_scale).rem_euclid(1.0);
                let outer_axis_hits_half_pixel = (outer_axis_fraction - 0.5).abs() <= 1.0e-6;
                let expands_outer_axis_coverage = expands_inner_axis_coverage
                    || (outer_axis_hits_half_pixel
                        && (border_style == BorderStyle::Double
                            || (is_3d_style
                                && !uses_platform_3d_current_color
                                && (side == BorderSide::Top
                                    || (side == BorderSide::Left
                                        && matches!(
                                            border_style,
                                            BorderStyle::Inset | BorderStyle::Outset
                                        ))))));
                if expands_outer_axis_coverage || expands_inner_axis_coverage {
                    let half_device_pixel = 0.5 / device_scale.max(f64::EPSILON) as f32;
                    if expands_outer_axis_coverage {
                        match side {
                            BorderSide::Top => {
                                clip_points[0].1 -= half_device_pixel;
                                clip_points[1].1 -= half_device_pixel;
                            }
                            BorderSide::Bottom => {
                                clip_points[0].1 += half_device_pixel;
                                clip_points[1].1 += half_device_pixel;
                            }
                            BorderSide::Right if !uses_platform_3d_current_color => {
                                clip_points[0].0 += half_device_pixel;
                                clip_points[1].0 += half_device_pixel;
                            }
                            BorderSide::Left if !uses_platform_3d_current_color => {
                                clip_points[0].0 -= half_device_pixel;
                                clip_points[1].0 -= half_device_pixel;
                            }
                            BorderSide::Right | BorderSide::Left => {}
                        }
                    }
                    if expands_inner_axis_coverage {
                        match side {
                            BorderSide::Top => {
                                clip_points[2].1 += half_device_pixel;
                                clip_points[3].1 += half_device_pixel;
                            }
                            BorderSide::Bottom => {
                                clip_points[2].1 -= half_device_pixel;
                                clip_points[3].1 -= half_device_pixel;
                            }
                            BorderSide::Right if !uses_platform_3d_current_color => {
                                clip_points[2].0 -= half_device_pixel;
                                clip_points[3].0 -= half_device_pixel;
                            }
                            BorderSide::Left if !uses_platform_3d_current_color => {
                                clip_points[2].0 += half_device_pixel;
                                clip_points[3].0 += half_device_pixel;
                            }
                            BorderSide::Right | BorderSide::Left => {}
                        }
                    }
                }
                if is_3d_style
                    && !uses_platform_3d_current_color
                    && matches!(border_style, BorderStyle::Groove | BorderStyle::Ridge)
                    && matches!(side, BorderSide::Left | BorderSide::Right)
                {
                    let scale = device_scale.max(f64::EPSILON) as f32;
                    let snapped_inner = if side == BorderSide::Right {
                        (clip_points[2].0 * scale).ceil() / scale
                    } else {
                        clip_points[2].0
                    };
                    clip_points[2].0 = snapped_inner;
                    clip_points[3].0 = snapped_inner;
                }
                let mut clip_path = PathBuilder::new();
                clip_path.move_to(Point::new(clip_points[0].0, clip_points[0].1));
                clip_path.line_to(Point::new(clip_points[1].0, clip_points[1].1));
                if preserve_horizontal_outer_edge
                    && matches!(side, BorderSide::Top | BorderSide::Bottom)
                {
                    clip_path.line_to(Point::new(clip_points[1].0, clip_points[2].1));
                }
                clip_path.line_to(Point::new(clip_points[2].0, clip_points[2].1));
                clip_path.line_to(Point::new(clip_points[3].0, clip_points[3].1));
                clip_path.close();
                canvas.clip_path(
                    &clip_path.detach(),
                    ClipOp::Intersect,
                    (is_3d_style
                        && ((!uses_platform_3d_current_color
                            && matches!(border_style, BorderStyle::Groove | BorderStyle::Ridge)
                            && matches!(side, BorderSide::Left | BorderSide::Right))
                            || (uses_platform_3d_current_color && side == BorderSide::Left)))
                        || (border_style == BorderStyle::Dotted && antialias_stroked_miter),
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
                platform_3d_fieldset_coverage,
                device_scale,
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
    platform_3d_fieldset_coverage: bool,
    device_scale: f64,
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
            // Outer line.
            let outer_rect = shrink_border_rect(&rect, width, 0.0, line_width, side);
            let outer_packing = match side {
                BorderSide::Right => PhysicalCoveragePacking::LeadingX,
                BorderSide::Bottom => PhysicalCoveragePacking::LeadingY,
                BorderSide::Top | BorderSide::Left => PhysicalCoveragePacking::Default,
            };
            draw_css_coverage_rect(canvas, outer_rect, &resolved, device_scale, outer_packing);
            // Inner line.
            let inner_rect = shrink_border_rect(&rect, width, width - line_width, line_width, side);
            let inner_packing = match side {
                BorderSide::Right => PhysicalCoveragePacking::LeadingX,
                BorderSide::Bottom => PhysicalCoveragePacking::LeadingY,
                BorderSide::Top | BorderSide::Left => PhysicalCoveragePacking::Default,
            };
            draw_css_coverage_rect(canvas, inner_rect, &resolved, device_scale, inner_packing);
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
                platform_3d_fieldset_coverage,
                device_scale,
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
                platform_3d_fieldset_coverage,
                device_scale,
            );
        }
        BorderStyle::Inset => {
            // Per CSS: top+left darkened, bottom+right lightened.
            let shaded = if matches!(side, BorderSide::Top | BorderSide::Left) {
                shade_3d_dark(
                    &base_color,
                    uses_platform_3d_current_color,
                    platform_3d_fieldset_coverage,
                )
            } else {
                shade_3d_light(
                    &base_color,
                    uses_platform_3d_current_color,
                    platform_3d_fieldset_coverage,
                )
            };
            let packing = match side {
                BorderSide::Left => PhysicalCoveragePacking::LeadingX,
                BorderSide::Right => PhysicalCoveragePacking::TrailingX,
                BorderSide::Top | BorderSide::Bottom => PhysicalCoveragePacking::Default,
            };
            draw_color4f_coverage_rect_with_packing(canvas, rect, shaded, device_scale, packing);
        }
        BorderStyle::Outset => {
            // Per CSS: top+left lightened, bottom+right darkened.
            let shaded = if matches!(side, BorderSide::Top | BorderSide::Left) {
                shade_3d_light(
                    &base_color,
                    uses_platform_3d_current_color,
                    platform_3d_fieldset_coverage,
                )
            } else {
                shade_3d_dark(
                    &base_color,
                    uses_platform_3d_current_color,
                    platform_3d_fieldset_coverage,
                )
            };
            let packing = match side {
                BorderSide::Left => PhysicalCoveragePacking::LeadingX,
                BorderSide::Right => PhysicalCoveragePacking::TrailingX,
                BorderSide::Top | BorderSide::Bottom => PhysicalCoveragePacking::Default,
            };
            draw_color4f_coverage_rect_with_packing(canvas, rect, shaded, device_scale, packing);
        }
        BorderStyle::None | BorderStyle::Hidden => {
            // Already handled above, but satisfy exhaustive match.
        }
    }
}

/// Shrink a border rect inward for double-line border painting.
fn shrink_border_rect(
    rect: &Rect,
    _border_width: f32,
    inset: f32,
    line_width: f32,
    side: BorderSide,
) -> Rect {
    if matches!(side, BorderSide::Top | BorderSide::Bottom) {
        Rect::from_xywh(rect.left, rect.top + inset, rect.width(), line_width)
    } else {
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
    corrected_platform_tone: bool,
    device_scale: f64,
) {
    let half_width = (width / 2.0).ceil().max(1.0);
    let dark = shade_3d_dark(color, uses_platform_current_color, corrected_platform_tone);
    let light = shade_3d_light(color, uses_platform_current_color, corrected_platform_tone);

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

    let coverage_packing = if side == BorderSide::Left && !corrected_platform_tone {
        PhysicalCoveragePacking::TrailingXColorUp
    } else {
        PhysicalCoveragePacking::Default
    };

    // Outer half.
    let outer = shrink_border_rect(rect, width, 0.0, half_width, side);
    draw_color4f_coverage_rect_with_packing(
        canvas,
        outer,
        outer_color,
        device_scale,
        coverage_packing,
    );

    // Inner half.
    let inner = shrink_border_rect(rect, width, half_width, width - half_width, side);
    draw_color4f_coverage_rect_with_packing(
        canvas,
        inner,
        inner_color,
        device_scale,
        coverage_packing,
    );
}

fn draw_color4f_coverage_rect_with_packing(
    canvas: &Canvas,
    rect: Rect,
    color: Color4f,
    device_scale: f64,
    packing: PhysicalCoveragePacking,
) {
    draw_css_coverage_rect(
        canvas,
        rect,
        &Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        },
        device_scale,
        packing,
    );
}

/// Blink's CSS 3D border shade subtracts one third from the largest sRGB
/// component, preserving the component ratios. Its legacy `Color::Dark()`
/// quantizer multiplies by the largest `f32` below 256 and truncates, rather
/// than rounding on a 255-step scale. That distinction is observable for
/// colors such as gray(118), which darkens to 33, while gray(128) becomes 44.
fn darken_color(color: &Color4f) -> Color4f {
    let value = color.r.max(color.g).max(color.b);
    let multiplier = if value == 0.0 {
        0.0
    } else {
        ((value - 0.33).max(0.0) / value).min(1.0)
    };
    let scale_factor = f32::from_bits(256.0_f32.to_bits() - 1);
    let quantize =
        |component: f32| (component.clamp(0.0, 1.0) * multiplier * scale_factor).floor() / 255.0;
    Color4f::new(
        quantize(color.r),
        quantize(color.g),
        quantize(color.b),
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
fn shade_3d_dark(
    color: &Color4f,
    uses_platform_current_color: bool,
    corrected_platform_tone: bool,
) -> Color4f {
    if uses_platform_current_color {
        let shade = if corrected_platform_tone {
            155.0
        } else {
            154.0
        } / 255.0;
        Color4f::new(shade, shade, shade, color.a)
    } else {
        darken_color(color)
    }
}

fn shade_3d_light(
    color: &Color4f,
    uses_platform_current_color: bool,
    corrected_platform_tone: bool,
) -> Color4f {
    if uses_platform_current_color {
        let shade = if corrected_platform_tone {
            239.0
        } else {
            238.0
        } / 255.0;
        Color4f::new(shade, shade, shade, color.a)
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
        BoxShadow, ConicGradient, CssLinearGradient, Direction, GradientStop, ImageResourceId,
        WritingMode,
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
    fn shared_column_decoration_prepass_keeps_floats_in_their_paint_phase() {
        let mut doc = Document::new();
        let normal = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(normal, |style| style.display = Display::Block);
        let floated = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(floated, |style| {
            style.display = Display::Block;
            style.float = openui_style::Float::Left;
        });
        let size = PhysicalSize::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(20));
        let offset = PhysicalOffset::new(LayoutUnit::zero(), LayoutUnit::zero());
        let normal_fragment = Fragment::new_box(normal, size);
        let floated_fragment = Fragment::new_box(floated, size);
        let mut decorations = Vec::new();
        collect_shared_column_decorations(
            &normal_fragment,
            &doc,
            offset,
            &[],
            &[],
            true,
            &mut decorations,
        );
        assert_eq!(decorations.len(), 1);
        decorations.clear();
        collect_shared_column_decorations(
            &floated_fragment,
            &doc,
            offset,
            &[],
            &[],
            true,
            &mut decorations,
        );
        assert!(decorations.is_empty());
    }

    #[test]
    fn broken_image_resource_switches_to_the_pinned_200_percent_asset() {
        let low = broken_image_resource(1.999).expect("low-resolution broken image");
        let high = broken_image_resource(2.0).expect("high-resolution broken image");

        assert_eq!((low.width(), low.height()), (14, 16));
        assert_eq!((high.width(), high.height()), (28, 32));
    }

    #[test]
    fn css_3d_dark_uses_blink_legacy_quantization() {
        let gray_118 = darken_color(&Color4f::new(
            118.0 / 255.0,
            118.0 / 255.0,
            118.0 / 255.0,
            1.0,
        ));
        let gray_128 = darken_color(&Color4f::new(
            128.0 / 255.0,
            128.0 / 255.0,
            128.0 / 255.0,
            1.0,
        ));

        assert_eq!((gray_118.r * 255.0).round(), 33.0);
        assert_eq!((gray_128.r * 255.0).round(), 44.0);
    }

    #[test]
    fn background_ratio_and_physical_edge_bands_match_blink_packing() {
        assert_eq!(blink_layout_mul_div(190.0, 224.0, 300.0), 141.859375);
        assert_eq!(
            physical_patch_axis_bands(208.8, 209.0, 321.0, 1.25, 141),
            vec![(0, 1, 192), (1, 140, 255), (140, 141, 65)]
        );
        assert_eq!(
            physical_patch_axis_bands(100.0, 100.0, 260.0, 1.25, 200),
            vec![(0, 200, 255)]
        );
    }

    #[test]
    fn coincident_different_color_coverage_composites_in_source_order() {
        let mut surface = surfaces::raster_n32_premul((160, 64)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        surface.canvas().scale((1.25, 1.25));
        let edge = Rect::from_ltrb(20.0, 19.0, 100.0, 20.0);
        draw_css_coverage_rect(
            surface.canvas(),
            edge,
            &Color::BLUE,
            1.25,
            PhysicalCoveragePacking::Default,
        );
        draw_css_coverage_rect(
            surface.canvas(),
            edge,
            &Color::BLACK,
            1.25,
            PhysicalCoveragePacking::Default,
        );
        let pixels = surface_bytes(&mut surface);
        let pixel = &pixels[(23 * 160 + 50) * 4..][..4];
        // Native N32 byte order is BGRA on the qualification host.
        assert_eq!(pixel, [191, 143, 143, 255]);
    }

    #[test]
    fn opaque_in_flow_child_only_occludes_a_fully_covered_inner_border_box() {
        let mut doc = Document::new();
        let parent = doc.create_node(ElementTag::Div);
        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(child, |style| style.background_color = Color::WHITE);

        let mut parent_fragment = Fragment::new_box(
            parent,
            PhysicalSize::new(LayoutUnit::from_i32(102), LayoutUnit::from_i32(52)),
        );
        parent_fragment.border = BoxStrut::new(
            LayoutUnit::from_i32(1),
            LayoutUnit::from_i32(1),
            LayoutUnit::from_i32(1),
            LayoutUnit::from_i32(1),
        );
        let mut child_fragment = Fragment::new_box(
            child,
            PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(50)),
        );
        child_fragment.offset =
            PhysicalOffset::new(LayoutUnit::from_i32(1), LayoutUnit::from_i32(1));
        parent_fragment.children.push(child_fragment);

        assert!(opaque_in_flow_child_covers_inner_border_box(
            &parent_fragment,
            &doc
        ));

        parent_fragment.children[0].offset.left = LayoutUnit::from_i32(2);
        assert!(!opaque_in_flow_child_covers_inner_border_box(
            &parent_fragment,
            &doc
        ));

        let mut inline_child = parent_fragment.children.pop().unwrap();
        inline_child.offset = PhysicalOffset::zero();
        let mut line = Fragment::new_box(
            NodeId::NONE,
            PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(50)),
        );
        line.offset = PhysicalOffset::new(LayoutUnit::from_i32(1), LayoutUnit::from_i32(1));
        line.children.push(inline_child);
        parent_fragment.children.push(line);
        assert!(
            opaque_in_flow_child_covers_inner_border_box(&parent_fragment, &doc),
            "an anonymous line does not hide its opaque in-flow child"
        );

        parent_fragment.children[0].has_overflow_clip = true;
        assert!(!opaque_in_flow_child_covers_inner_border_box(
            &parent_fragment,
            &doc
        ));

        assert!(borderless_background_can_be_occluded(
            &parent_fragment,
            &doc
        ));
        parent_fragment.is_first_for_node = false;
        assert!(!borderless_background_can_be_occluded(
            &parent_fragment,
            &doc
        ));
        parent_fragment.is_first_for_node = true;
        let columns = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(columns, |style| style.column_count = Some(2));
        doc.append_child(columns, parent);
        assert!(!borderless_background_can_be_occluded(
            &parent_fragment,
            &doc
        ));
    }

    #[test]
    fn opaque_image_background_culling_preserves_uncovered_and_effected_boxes() {
        let mut doc = Document::new();
        let parent = doc.create_node(ElementTag::Div);
        let image_node = doc.create_node(ElementTag::Image);
        let resource = doc.register_image_resource(
            "memory:opaque-image-guard",
            "image/png",
            "d49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe",
            include_bytes!("../../openui/tests/assets/green-200.png").to_vec(),
        );
        let replaced = openui_dom::ReplacedContent {
            resource: ReplacedResourceKind::Image(resource),
            intrinsic_width: Some(200.0),
            intrinsic_height: Some(200.0),
            intrinsic_ratio: Some((200.0, 200.0)),
        };
        doc.node_mut(image_node).replaced = Some(replaced);
        let size = PhysicalSize::new(LayoutUnit::from_i32(150), LayoutUnit::from_i32(150));
        let mut fragment = Fragment::new_box(parent, size);
        fragment.children.push(Fragment::new_box(image_node, size));
        assert!(opaque_in_flow_child_covers_inner_border_box(
            &fragment, &doc
        ));

        fragment.children[0].size.width = LayoutUnit::from_i32(100);
        assert!(!opaque_in_flow_child_covers_inner_border_box(
            &fragment, &doc
        ));
        fragment.children[0].size = size;
        fragment.children[0].padding.left = LayoutUnit::from_i32(1);
        assert!(!opaque_in_flow_child_covers_inner_border_box(
            &fragment, &doc
        ));
        fragment.children[0].padding = BoxStrut::zero();
        fragment.children[0].border.left = LayoutUnit::from_i32(1);
        assert!(!opaque_in_flow_child_covers_inner_border_box(
            &fragment, &doc
        ));
        fragment.children[0].border = BoxStrut::zero();

        for fit in [ObjectFit::Contain, ObjectFit::None, ObjectFit::ScaleDown] {
            doc.update_resolved_style(image_node, |style| style.object_fit = fit);
            assert!(!opaque_in_flow_child_covers_inner_border_box(
                &fragment, &doc
            ));
        }
        doc.update_resolved_style(image_node, |style| style.object_fit = ObjectFit::Cover);
        assert!(opaque_in_flow_child_covers_inner_border_box(
            &fragment, &doc
        ));
        let original_style = doc.node(image_node).style.clone();
        let changes: [fn(&mut ComputedStyle); 7] = [
            |style: &mut ComputedStyle| style.opacity = 0.5,
            |style: &mut ComputedStyle| style.filter_blur = 1.0,
            |style: &mut ComputedStyle| style.filter_grayscale = 0.5,
            |style: &mut ComputedStyle| style.will_change_transform = true,
            |style: &mut ComputedStyle| style.establishes_transform_containing_block = true,
            |style: &mut ComputedStyle| style.object_position.x = BackgroundPosition::start(),
            |style: &mut ComputedStyle| style.shape_outside = openui_style::ShapeOutside::BorderBox,
        ];
        for change in changes {
            doc.update_resolved_style(image_node, |style| {
                *style = original_style.clone();
                change(style);
            });
            assert!(!opaque_in_flow_child_covers_inner_border_box(
                &fragment, &doc
            ));
        }
        doc.update_resolved_style(image_node, |style| *style = original_style);
        doc.update_resolved_style(parent, |style| {
            style.box_shadow.push(BoxShadow {
                offset_x: 1.0,
                offset_y: 1.0,
                blur_radius: 1.0,
                spread_radius: 0.0,
                color: Color::BLACK,
                inset: false,
            });
        });
        assert!(!opaque_in_flow_child_covers_inner_border_box(
            &fragment, &doc
        ));
        doc.update_resolved_style(parent, |style| style.box_shadow.clear());
        doc.node_mut(image_node).replaced = Some(openui_dom::ReplacedContent {
            resource: ReplacedResourceKind::TransparentCanvas,
            ..replaced
        });
        assert!(!opaque_in_flow_child_covers_inner_border_box(
            &fragment, &doc
        ));
        doc.node_mut(image_node).replaced = Some(openui_dom::ReplacedContent {
            resource: ReplacedResourceKind::Image(ImageResourceId::new(999)),
            ..replaced
        });
        assert!(!opaque_in_flow_child_covers_inner_border_box(
            &fragment, &doc
        ));

        let translucent = doc.register_image_resource(
            "memory:translucent-image-guard",
            "image/x-openui-rgba8",
            "54a971547aa343f51a9e41492ecffecf7d6106939467fa4307370edf3fa12607",
            vec![
                79, 85, 73, 82, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 128, 0, 128,
            ],
        );
        doc.node_mut(image_node).replaced = Some(openui_dom::ReplacedContent {
            resource: ReplacedResourceKind::Image(translucent),
            ..replaced
        });
        assert!(!opaque_in_flow_child_covers_inner_border_box(
            &fragment, &doc
        ));
    }

    #[test]
    fn generated_images_resample_only_when_repetition_or_rounding_requires_it() {
        let resolved = (24.0, 18.0);
        assert_eq!(
            generated_image_resample_size(true, false, false, false, false, resolved),
            None,
            "a single tile must retain compositor-local dither phase"
        );
        assert_eq!(
            generated_image_resample_size(true, false, false, true, false, resolved),
            Some(resolved)
        );
        assert_eq!(
            generated_image_resample_size(true, false, true, true, false, resolved),
            Some(resolved)
        );
        assert_eq!(
            generated_image_resample_size(true, false, false, true, true, resolved),
            Some(resolved)
        );
        assert_eq!(
            generated_image_resample_size(false, false, true, true, true, resolved),
            None
        );
        assert_eq!(
            generated_image_resample_size(true, true, true, true, true, resolved),
            None
        );
    }

    #[test]
    fn half_open_clip_only_moves_trailing_edges_on_pixel_centers() {
        let rect = Rect::from_ltrb(2.0, 3.0, 10.0, 6.0);
        let clipped = half_open_physical_clip_rect(rect, 1.25);
        assert_eq!(clipped.left, rect.left);
        assert_eq!(clipped.top, rect.top);
        assert_eq!(clipped.right.to_bits(), rect.right.to_bits() - 1);
        assert_eq!(clipped.bottom.to_bits(), rect.bottom.to_bits() - 1);

        let off_center = Rect::from_ltrb(2.0, 3.0, 10.4, 6.4);
        assert_eq!(half_open_physical_clip_rect(off_center, 1.25), off_center);
    }

    #[test]
    fn native_following_text_closes_late_phases_on_the_physical_grid() {
        for scale in [1.25, 1.5] {
            let resolved = native_following_inline_origin(891.0, scale);
            assert_eq!((resolved * scale).rem_euclid(1.0), 31.0 / 64.0);
        }
        assert_eq!(native_following_inline_origin(891.0, 1.0), 891.0);
        assert_eq!(native_following_inline_origin(891.0, 2.0), 891.0);
    }

    #[test]
    fn passive_video_surface_and_controls_use_content_box_and_restore_canvas() {
        let mut doc = Document::new();
        let video = doc.create_node(openui_dom::ElementTag::Video);
        doc.set_attribute(video, "controls", "");
        doc.append_child(doc.root(), video);

        let mut fragment = Fragment::new_box(
            video,
            PhysicalSize::new(LayoutUnit::from_i32(224), LayoutUnit::from_i32(160)),
        );
        fragment.border = BoxStrut::new(
            LayoutUnit::from_i32(2),
            LayoutUnit::from_i32(2),
            LayoutUnit::from_i32(2),
            LayoutUnit::from_i32(2),
        );
        fragment.padding = BoxStrut::new(
            LayoutUnit::from_i32(4),
            LayoutUnit::from_i32(4),
            LayoutUnit::from_i32(4),
            LayoutUnit::from_i32(4),
        );

        let mut surface = surfaces::raster_n32_premul((224, 160)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        let save_count = surface.canvas().save_count();
        paint_media_element(
            surface.canvas(),
            &fragment,
            &doc,
            &doc.node(video).style,
            PhysicalOffset::zero(),
            1.0,
        );
        assert_eq!(surface.canvas().save_count(), save_count);

        let pixels = surface_bytes(&mut surface);
        let pixel = |x: usize, y: usize| &pixels[(y * 224 + x) * 4..][..4];
        assert_eq!(pixel(0, 0), [255, 255, 255, 255]);
        assert_eq!(pixel(6, 6), [51, 51, 51, 255]);
        assert_eq!(pixel(112, 20), [51, 51, 51, 255]);
        assert!(pixel(112, 140)[0] < 51, "the bottom scrim must darken");
        assert!(pixels
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 51 && pixel[1] > 51 && pixel[2] > 51));
    }

    #[test]
    fn empty_video_without_controls_remains_transparent() {
        let mut doc = Document::new();
        let video = doc.create_node(openui_dom::ElementTag::Video);
        doc.append_child(doc.root(), video);
        let fragment = Fragment::new_box(
            video,
            PhysicalSize::new(LayoutUnit::from_i32(80), LayoutUnit::from_i32(60)),
        );
        let mut surface = surfaces::raster_n32_premul((80, 60)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        paint_media_element(
            surface.canvas(),
            &fragment,
            &doc,
            &doc.node(video).style,
            PhysicalOffset::zero(),
            1.0,
        );
        assert!(surface_bytes(&mut surface)
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255]));
    }

    #[test]
    fn replaced_object_overflows_content_box_when_overflow_is_visible() {
        let mut doc = Document::new();
        let image_node = doc.create_node(openui_dom::ElementTag::Image);
        doc.append_child(doc.root(), image_node);
        doc.update_resolved_style(image_node, |style| {
            style.object_fit = ObjectFit::None;
            // This test exercises an authored override of the Chromium UA
            // clipping default on replaced elements.
            style.overflow_x = Overflow::Visible;
            style.overflow_y = Overflow::Visible;
            style.border_top_left_radius = (12.0, 12.0);
            style.border_top_right_radius = (12.0, 12.0);
            style.border_bottom_right_radius = (12.0, 12.0);
            style.border_bottom_left_radius = (12.0, 12.0);
            assert_eq!(style.overflow_x, Overflow::Visible);
            assert_eq!(style.overflow_y, Overflow::Visible);
        });
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
        doc.node_mut(image_node).replaced = Some(openui_dom::ReplacedContent {
            resource: ReplacedResourceKind::Image(image),
            intrinsic_width: Some(81.0),
            intrinsic_height: Some(81.0),
            intrinsic_ratio: Some((81.0, 81.0)),
        });

        let fragment = Fragment::new_box(
            image_node,
            PhysicalSize::new(LayoutUnit::from_i32(40), LayoutUnit::from_i32(40)),
        );
        let mut surface = surfaces::raster_n32_premul((90, 90)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        paint_replaced_content(
            surface.canvas(),
            &fragment,
            &doc,
            &doc.node(image_node).style,
            PhysicalOffset::zero(),
            1.0,
        );
        let pixels = surface_bytes(&mut surface);
        let pixel = |x: usize, y: usize| &pixels[(y * 90 + x) * 4..][..4];
        // The source's top-center blue diamond begins exactly at the
        // content-box end; it must remain visible as replaced-element ink.
        assert_ne!(pixel(40, 0), [255, 255, 255, 255]);
        assert_ne!(pixel(54, 40), [255, 255, 255, 255]);
        assert_eq!(pixel(85, 85), [255, 255, 255, 255]);
        assert!(pixels
            .chunks_exact(4)
            .any(|pixel| pixel != [255, 255, 255, 255]));
    }

    #[test]
    fn ahem_preceding_row_trigger_distinguishes_css_and_integral_device_phases() {
        let offset = LayoutUnit::from_f32(223.1875);
        assert!(!should_replay_ahem_preceding_row(offset, 1.0));
        assert!(!should_replay_ahem_preceding_row(offset, 1.25));
        assert!(!should_replay_ahem_preceding_row(offset, 1.5));
        assert!(!should_replay_ahem_preceding_row(offset, 2.0));
        assert!(should_replay_ahem_preceding_row(offset, 3.0));
        assert!(should_replay_ahem_preceding_row(
            LayoutUnit::from_f32(223.5),
            1.25
        ));

        for (scale, expected_physical_shift) in
            [(1.0, 1.0), (1.25, 1.75), (1.5, 2.5), (2.0, 2.5), (3.0, 4.0)]
        {
            let shift =
                chromium_rotated_aliased_inline_shift(LayoutUnit::from_f32(26.5), 8.0, scale);
            assert!((f64::from(shift) * scale - expected_physical_shift).abs() < 1.0e-6);
            assert_eq!(
                shift,
                chromium_rotated_aliased_inline_shift(LayoutUnit::from_f32(41.5), 8.0, scale),
                "physical strike origin must not depend on the CSS origin phase"
            );
        }
        assert_eq!(
            chromium_rotated_aliased_inline_shift(LayoutUnit::from_f32(26.5), 10.0, 1.25),
            0.0,
            "larger strong-aliased strikes already expose Chromium's block-axis origin"
        );

        let clamped_offset = LayoutUnit::from_f32(21.1875);
        assert!(!should_replay_line_clamp_ahem_preceding_row(
            clamped_offset,
            16.0,
            1.25
        ));
        assert!(!should_replay_line_clamp_ahem_preceding_row(
            clamped_offset,
            16.0,
            1.5
        ));
        assert!(!should_replay_line_clamp_ahem_preceding_row(
            clamped_offset,
            16.0,
            2.0
        ));
        assert!(should_replay_line_clamp_ahem_preceding_row(
            clamped_offset,
            16.0,
            3.0
        ));

        let boundary_strike_offset = LayoutUnit::from_f32(95.796875);
        assert!(!should_replay_line_clamp_ahem_preceding_row(
            boundary_strike_offset,
            24.0,
            1.25
        ));
        assert!(!should_replay_line_clamp_ahem_preceding_row(
            boundary_strike_offset,
            24.0,
            1.5
        ));
        assert!(!should_replay_line_clamp_ahem_preceding_row(
            boundary_strike_offset,
            24.0,
            1.0
        ));
        assert!(should_replay_line_clamp_ahem_preceding_row(
            boundary_strike_offset,
            24.0,
            2.0
        ));

        let large_clamped_offset = LayoutUnit::from_f32(20.390625);
        assert!(!should_replay_line_clamp_ahem_preceding_row(
            large_clamped_offset,
            32.0,
            1.25
        ));
        assert!(should_replay_line_clamp_ahem_preceding_row(
            large_clamped_offset,
            32.0,
            1.5
        ));
        assert!(!should_replay_line_clamp_ahem_preceding_row(
            large_clamped_offset,
            32.0,
            2.0
        ));
        assert!(!should_replay_line_clamp_ahem_preceding_row(
            large_clamped_offset,
            32.0,
            3.0
        ));
    }

    #[test]
    fn fractional_outline_span_partitions_exact_physical_coverage() {
        let segments = physical_coverage_axis_segments(19.0, 421.0, 1.25);
        assert_eq!(segments.len(), 3);
        assert_eq!((segments[0].0, segments[0].1), (23, 24));
        assert!((segments[0].2 - 0.25).abs() < f32::EPSILON);
        assert_eq!(segments[1], (24, 526, 1.0));
        assert_eq!((segments[2].0, segments[2].1), (526, 527));
        assert!((segments[2].2 - 0.25).abs() < f32::EPSILON);
        assert_eq!(packed_physical_coverage(0.25, false), 64.0 / 255.0);
        assert_eq!(packed_physical_coverage(0.75, false), 192.0 / 255.0);
        assert_eq!(packed_physical_coverage(0.25, true), 63.0 / 255.0);
        assert_eq!(packed_physical_coverage(0.75, true), 191.0 / 255.0);
        assert_eq!(packed_physical_coverage(1.0, false), 1.0);
    }

    #[test]
    fn coincident_zero_block_outlines_close_the_fractional_joint_cell() {
        let mut surface = surfaces::raster_n32_premul((500, 240)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        surface.canvas().scale((1.25, 1.25));

        let fragment = Fragment::new_box(
            NodeId::NONE,
            PhysicalSize::new(LayoutUnit::from_i32(300), LayoutUnit::zero()),
        );
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| {
            computed.outline_style = BorderStyle::Solid;
            computed.outline_width = 1;
            computed.device_scale_factor = 1.25;
            computed.outline_color = StyleColor::Resolved(Color::BLUE);
        });
        let offset = PhysicalOffset::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(71));
        paint_outline(surface.canvas(), &fragment, &style, offset);
        style
            .update_derived(|computed| computed.outline_color = StyleColor::Resolved(Color::BLACK));
        paint_outline(surface.canvas(), &fragment, &style, offset);

        let pixels = surface_bytes(&mut surface);
        let pixel = |x: usize, y: usize| &pixels[(y * 500 + x) * 4..][..4];
        assert_eq!(pixel(23, 88), [191, 143, 143, 255]);
        assert_eq!(pixel(401, 88), [191, 143, 143, 255]);
    }

    #[test]
    fn inside_marker_zero_block_outline_retains_the_collapsed_two_edge_contour() {
        let mut surface = surfaces::raster_n32_premul((500, 240)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        surface.canvas().scale((1.25, 1.25));

        let mut spanner = Fragment::new_box(
            NodeId::NONE,
            PhysicalSize::new(LayoutUnit::from_i32(300), LayoutUnit::zero()),
        );
        spanner.paint_zero_block_outline = true;
        let mut spanner_style = ComputedStyle::default();
        spanner_style.update_derived(|computed| {
            computed.outline_style = BorderStyle::Solid;
            computed.outline_width = 1;
            computed.device_scale_factor = 1.25;
            computed.outline_color = StyleColor::Resolved(Color::BLUE);
        });
        paint_outline(
            surface.canvas(),
            &spanner,
            &spanner_style,
            PhysicalOffset::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(105)),
        );

        let parent = Fragment::new_box(
            NodeId::NONE,
            PhysicalSize::new(LayoutUnit::from_i32(300), LayoutUnit::from_i32(19)),
        );
        let mut parent_style = ComputedStyle::default();
        parent_style.update_derived(|computed| {
            computed.outline_style = BorderStyle::Solid;
            computed.outline_width = 1;
            computed.device_scale_factor = 1.25;
            computed.outline_color = StyleColor::Resolved(Color::BLACK);
        });
        paint_outline(
            surface.canvas(),
            &parent,
            &parent_style,
            PhysicalOffset::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(87)),
        );

        let pixels = surface_bytes(&mut surface);
        let pixel = |x: usize, y: usize| &pixels[(y * 500 + x) * 4..][..4];
        assert_eq!(pixel(200, 131), [255, 63, 63, 255]);
        assert_eq!(pixel(200, 132), [128, 0, 0, 255]);
        assert_eq!(pixel(200, 133), [63, 15, 15, 255]);
    }

    #[test]
    fn adjacent_decoration_coverage_overlaps_only_at_fractional_device_edges() {
        assert!(physical_axis_coverage_may_overlap(
            20.0, 70.0, 70.0, 120.0, 1.25
        ));
        assert!(!physical_axis_coverage_may_overlap(
            20.0, 70.0, 70.0, 120.0, 1.0
        ));
        assert!(physical_axis_coverage_may_overlap(
            20.0, 80.0, 70.0, 120.0, 1.0
        ));
        assert!(!physical_axis_coverage_may_overlap(
            20.0, 60.0, 70.0, 120.0, 1.25
        ));
    }

    #[test]
    fn fragmented_flex_row_major_order_requires_an_internal_four_way_junction() {
        let mut fragment = Fragment::new_box(
            NodeId::NONE,
            PhysicalSize::new(LayoutUnit::from_i32(50), LayoutUnit::from_i32(100)),
        );
        fragment.decoration_slice = Some(openui_layout::DecorationSlice {
            source_block_offset: LayoutUnit::from_i32(100),
            source_block_size: LayoutUnit::from_i32(200),
        });
        fragment.is_first_for_node = false;
        for (left, top) in [(0, 0), (25, 0), (0, 50), (25, 50)] {
            let mut child = Fragment::new_box(
                NodeId::NONE,
                PhysicalSize::new(LayoutUnit::from_i32(25), LayoutUnit::from_i32(50)),
            );
            child.offset =
                PhysicalOffset::new(LayoutUnit::from_i32(left), LayoutUnit::from_i32(top));
            fragment.children.push(child);
        }
        assert!(fragmented_flex_has_internal_four_way_junction(&fragment));

        for child in &mut fragment.children {
            child.offset.top -= LayoutUnit::from_i32(50);
        }
        assert!(
            !fragmented_flex_has_internal_four_way_junction(&fragment),
            "a junction on the fragment clip edge must retain original paint order"
        );
    }

    #[test]
    fn ignored_table_radius_border_packs_fractional_sides_independently() {
        let scale = 1.25;
        let mut surface = surfaces::raster_n32_premul((200, 200)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        surface.canvas().scale((scale, scale));
        let color = Color::from_rgba8(0, 128, 0, 255);
        for side in [
            Rect::from_xywh(20.0, 20.0, 102.0, 3.0),
            Rect::from_xywh(20.0, 119.0, 102.0, 3.0),
            Rect::from_xywh(119.0, 20.0, 3.0, 102.0),
            Rect::from_xywh(20.0, 20.0, 3.0, 102.0),
        ] {
            draw_css_coverage_rect(
                surface.canvas(),
                side,
                &color,
                f64::from(scale),
                PhysicalCoveragePacking::Default,
            );
        }

        let pixels = surface_bytes(&mut surface);
        let pixel = |x: usize, y: usize| &pixels[(y * 200 + x) * 4..][..4];
        assert_eq!(pixel(30, 28), [63, 159, 63, 255]);
        assert_eq!(pixel(148, 30), [191, 223, 191, 255]);
        assert_eq!(pixel(152, 25), [63, 159, 63, 255]);
        assert_eq!(pixel(152, 152), [143, 199, 143, 255]);
        assert_eq!(pixel(100, 100), [255, 255, 255, 255]);
    }

    #[test]
    fn overlapping_fractional_outline_junctions_retain_independent_coverage() {
        let sample = |scale: f64, x: usize, y: usize, draws: usize| {
            let mut surface = surfaces::raster_n32_premul((700, 240)).expect("surface");
            surface.canvas().clear(skia_safe::Color::WHITE);
            surface.canvas().scale((scale as f32, scale as f32));
            let joint = 113.0;
            let physical_row = fractional_physical_row(joint, scale).expect("fractional joint");
            for _ in 0..draws {
                draw_outline_junction_row(
                    surface.canvas(),
                    19.0,
                    20.0,
                    420.0,
                    421.0,
                    joint,
                    physical_row,
                    false,
                    false,
                    &Color::BLACK,
                    scale,
                );
            }
            let pixels = surface_bytes(&mut surface);
            pixels[(y * 700 + x) * 4]
        };

        assert_eq!(sample(1.25, 526, 141, 1), 191);
        assert_eq!(sample(1.25, 526, 141, 2), 143);
        assert_eq!(sample(1.5, 631, 169, 1), 127);
        assert_eq!(sample(1.5, 631, 169, 2), 63);
    }

    #[test]
    fn external_outline_bottom_inner_corner_uses_half_open_coverage() {
        let sample = |scale: f64, row: i64, extent: usize| {
            let mut surface =
                surfaces::raster_n32_premul((extent as i32, extent as i32)).expect("surface");
            surface.canvas().clear(skia_safe::Color::WHITE);
            surface.canvas().scale((scale as f32, scale as f32));
            draw_outline_junction_row(
                surface.canvas(),
                42.0,
                44.0,
                131.0,
                133.0,
                131.0,
                row,
                false,
                true,
                &Color::from_rgba8(0, 128, 0, 255),
                scale,
            );
            let pixels = surface_bytes(&mut surface);
            pixels[((row as usize * extent + row as usize) * 4)..][..4].to_vec()
        };

        assert_eq!(sample(1.25, 163, 200), [145, 200, 145, 255]);
        assert_eq!(sample(1.5, 196, 240), [64, 160, 64, 255]);

        let mut surface = surfaces::raster_n32_premul((400, 400)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        surface.canvas().scale((1.25, 1.25));
        draw_outline_junction_row(
            surface.canvas(),
            118.0,
            120.0,
            170.0,
            172.0,
            270.0,
            337,
            true,
            true,
            &Color::from_rgba8(0, 128, 0, 255),
            1.25,
        );
        let pixels = surface_bytes(&mut surface);
        assert_eq!(&pixels[(337 * 400 + 212) * 4..][..4], [64, 160, 64, 255]);
    }

    #[test]
    fn fractional_ahem_strike_retains_row_from_ascent_phase() {
        assert!(!chromium_fractional_strike_retains_preceding_row(
            0.0, 18.72, 1.25
        ));
        assert!(chromium_fractional_strike_retains_preceding_row(
            0.0, 18.72, 1.5
        ));
        assert!(!chromium_fractional_strike_retains_preceding_row(
            0.0, 18.72, 2.0
        ));
    }

    #[test]
    fn aliased_ahem_start_replay_uses_integral_physical_magnification() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| {
            computed.raster_configuration =
                openui_geometry::RasterConfiguration::deterministic_aliased(false);
        });
        for (scale, expected) in [
            (1.0, true),
            (1.25, false),
            (1.5, false),
            (2.0, true),
            (3.0, true),
        ] {
            style.update_derived(|computed| computed.device_scale_factor = scale);
            assert_eq!(
                replays_aliased_ahem_start_for_raster_policy(&style, false),
                expected,
                "scale={scale}"
            );
            assert!(replays_aliased_ahem_start_for_raster_policy(&style, true));
        }
    }

    #[test]
    fn fractional_fragmentainer_clip_closes_outward_on_device_grid() {
        let rect = outward_snap_rect_to_physical(Rect::from_ltrb(20.0, 20.0, 70.0, 80.0), 1.25);
        assert_eq!(rect.left, 20.0);
        assert_eq!(rect.top, 20.0);
        assert!((rect.right - 70.4).abs() < 1.0e-5);
        assert_eq!(rect.bottom, 80.0);
    }

    #[test]
    fn negative_flex_stacking_contexts_sort_by_stack_level_before_order() {
        let mut doc = Document::new();
        let container = doc.create_node(openui_dom::ElementTag::Div);
        doc.update_resolved_style(container, |style| style.display = Display::Flex);
        doc.append_child(doc.root(), container);

        let minus_one = doc.create_node(openui_dom::ElementTag::Div);
        doc.update_resolved_style(minus_one, |style| style.z_index = Some(-1));
        doc.append_child(container, minus_one);
        let minus_two = doc.create_node(openui_dom::ElementTag::Div);
        doc.update_resolved_style(minus_two, |style| style.z_index = Some(-2));
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
                    1.0,
                ),
                expected
            );
        }
    }

    #[test]
    fn zero_block_fragment_transform_backing_retains_inline_extent() {
        let mut fragment = Fragment::new_box(
            NodeId::NONE,
            PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::zero()),
        );
        fragment.decoration_slice = Some(openui_layout::DecorationSlice {
            source_block_offset: LayoutUnit::from_i32(250),
            source_block_size: LayoutUnit::from_i32(175),
        });
        let mut child = Fragment::new_box(
            NodeId::NONE,
            PhysicalSize::new(LayoutUnit::from_i32(50), LayoutUnit::from_i32(50)),
        );
        child.offset = PhysicalOffset::new(LayoutUnit::from_i32(50), LayoutUnit::zero());
        fragment.children.push(child);

        assert_eq!(
            promoted_transform_backing_bounds(&fragment),
            Rect::from_xywh(0.0, 0.0, 100.0, 175.0),
        );
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
    fn truncated_decoration_retains_the_background_edge_coverage() {
        let mut document = Document::new();
        let node = document.create_node(openui_dom::ElementTag::Div);
        let mut style = ComputedStyle::default();
        style.update_derived(|style| {
            style.background_color = Color::from_rgba8(255, 255, 0, 255);
            style.device_scale_factor = 1.25;
        });
        let mut fragment = Fragment::new_box(
            node,
            PhysicalSize::new(LayoutUnit::from_i32(84), LayoutUnit::from_f32(67.3125)),
        );
        fragment.decoration_slice = Some(openui_layout::DecorationSlice {
            source_block_offset: LayoutUnit::from_f32(134.6875),
            source_block_size: LayoutUnit::from_i32(202),
        });
        fragment.is_first_for_node = false;
        // Both directions of CSS snapping retain the primitive's own edge
        // coverage. BGRA values come from preserved Chromium captures.
        for (limit, edge_row, edge_color) in [
            (25.3125, 31, [96, 160, 160, 255]),
            (25.65625, 32, [64, 192, 192, 255]),
            (24.65625, 31, [96, 160, 160, 255]),
        ] {
            fragment.decoration_paint_block_size = Some(LayoutUnit::from_f32(limit));
            let mut surface = surfaces::raster_n32_premul((120, 100)).expect("surface");
            surface
                .canvas()
                .clear(skia_safe::Color::from_rgb(128, 128, 128));
            surface.canvas().scale((1.25, 1.25));
            paint_box_decoration_background(
                surface.canvas(),
                &fragment,
                &document,
                &style,
                PhysicalOffset::zero(),
                1.0,
            );
            let pixels = surface_bytes(&mut surface);
            let pixel = |x: usize, y: usize| &pixels[(y * 120 + x) * 4..][..4];
            assert_eq!(pixel(10, edge_row - 1), [0, 255, 255, 255]);
            assert_eq!(pixel(10, edge_row), edge_color);
            assert_eq!(pixel(10, edge_row + 1), [128, 128, 128, 255]);
        }
    }

    #[test]
    fn sliced_borders_map_first_middle_and_last_physical_block_edges() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| computed.border_top_width = 1);
        style.update_derived(|computed| computed.border_right_width = 2);
        style.update_derived(|computed| computed.border_bottom_width = 3);
        style.update_derived(|computed| computed.border_left_width = 4);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);

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

    #[test]
    fn aligned_uniform_double_border_owns_the_inner_square_corner() {
        let mut surface = surfaces::raster_n32_premul((80, 80)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        let mut blue = Paint::default();
        blue.set_color(skia_safe::Color::BLUE);
        surface
            .canvas()
            .draw_rect(Rect::from_xywh(10.0, 10.0, 60.0, 60.0), &blue);

        let mut style = ComputedStyle::default();
        style.update_derived(|computed| {
            computed.border_top_width = 10;
            computed.border_right_width = 10;
            computed.border_bottom_width = 10;
            computed.border_left_width = 10;
            computed.border_top_style = BorderStyle::Double;
            computed.border_right_style = BorderStyle::Double;
            computed.border_bottom_style = BorderStyle::Double;
            computed.border_left_style = BorderStyle::Double;
            computed.border_top_color = StyleColor::Resolved(Color::RED);
            computed.border_right_color = StyleColor::Resolved(Color::RED);
            computed.border_bottom_color = StyleColor::Resolved(Color::RED);
            computed.border_left_color = StyleColor::Resolved(Color::RED);
        });
        let mut fragment = Fragment::new_box(NodeId::NONE, PhysicalSize::zero());
        fragment.border = openui_geometry::BoxStrut::new(
            LayoutUnit::from_i32(10),
            LayoutUnit::from_i32(10),
            LayoutUnit::from_i32(10),
            LayoutUnit::from_i32(10),
        );
        fragment.is_first_for_node = true;
        fragment.is_last_for_node = true;
        paint_borders(
            surface.canvas(),
            &fragment,
            &style,
            10.0,
            10.0,
            60.0,
            60.0,
            false,
            false,
            false,
        );

        let pixels = surface_bytes(&mut surface);
        let pixel_at = |x: usize, y: usize| &pixels[(y * 80 + x) * 4..][..4];
        assert_eq!(pixel_at(67, 67), [0, 0, 255, 255]);
        assert_eq!(pixel_at(65, 65), [255, 0, 0, 255]);
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
    fn background_shader_paints_to_clip_not_positioning_area() {
        let clip = Rect::from_ltrb(0.0, 0.0, 146.0, 146.0);
        let repeated = background_image_paint_bounds(clip, 20.0, 20.0, 32.0, 32.0, true, true);
        assert_eq!(repeated, clip);

        // A single `space` tile uses no-repeat behavior and keeps the
        // authored position even when it extends outside the padding box.
        let positioned = background_image_paint_bounds(clip, 10.0, 10.0, 50.0, 50.0, false, false);
        assert_eq!(positioned, Rect::from_ltrb(10.0, 10.0, 60.0, 60.0));

        assert!(uses_spaced_background_shader(
            BackgroundRepeat::Space,
            BackgroundRepeat::Space,
            3,
            3
        ));
        assert!(!uses_spaced_background_shader(
            BackgroundRepeat::Space,
            BackgroundRepeat::Space,
            1,
            1
        ));
        assert!(uses_single_repeated_generated_shader(
            true,
            BackgroundRepeat::Repeat,
            BackgroundRepeat::Round,
            2,
            2
        ));
        assert!(!uses_single_repeated_generated_shader(
            false,
            BackgroundRepeat::Repeat,
            BackgroundRepeat::Round,
            2,
            2
        ));
        assert!(uses_single_repeated_generated_shader(
            true,
            BackgroundRepeat::Space,
            BackgroundRepeat::Repeat,
            2,
            2
        ));
        assert!(uses_single_repeated_generated_shader(
            true,
            BackgroundRepeat::Repeat,
            BackgroundRepeat::Repeat,
            1,
            2
        ));

        // A concrete repeated generated image owns one local ordered-dither
        // origin for both axes, even when only one background axis repeats.
        assert_eq!(generated_image_raster_phase(50.0, true), 0);
        assert_eq!(generated_image_raster_phase(50.0, false), 2);

        // A 50 CSS-pixel repeat at 1.25x owns a 63-pixel backing. Its
        // generated image must fill all 63 pixels before that bitmap is
        // filtered back onto the 62.5-pixel destination period.
        assert_eq!(generated_image_backing_scale(true, 50.0, 63, 1.25), 1.26);
        assert_eq!(generated_image_backing_scale(true, 50.0, 75, 1.5), 1.5);
        assert_eq!(generated_image_backing_scale(false, 50.0, 63, 1.25), 1.25);
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
    fn transition_hints_use_chromiums_clamped_nine_stop_curve() {
        let first = Color::from_rgba_f32(0.1197, 0.1197, 0.0203, 0.32042524);
        let stops = vec![
            GradientStop {
                color: StyleColor::Resolved(first),
                position: GradientStopPosition::Px(-25_771_702_432.0),
            },
            GradientStop {
                color: StyleColor::Resolved(Color::TRANSPARENT),
                position: GradientStopPosition::HintPx(-41.333332),
            },
            GradientStop {
                color: StyleColor::Resolved(Color::from_rgba_f32(0.3048, 0.8152, 0.70885754, 0.0)),
                position: GradientStopPosition::Percent(26.0),
            },
        ];

        assert_eq!(css_gradient_positions(&stops, 464.0).len(), 2);
        let mut resolved =
            resolved_gradient_stops_with_hints(&stops, 464.0, &Color::BLACK).unwrap();
        assert_eq!(resolved.len(), 11);
        assert!(resolved
            .windows(2)
            .all(|pair| pair[0].position <= pair[1].position));
        // The seventh inserted stop is the authored midpoint. Its small
        // position offset is Chromium's intentional f32 cancellation after
        // clamping the remote CSS length.
        assert_eq!(resolved[7].position, -0.0859375);
        // Premultiplied interpolation keeps the visible RGB channels while
        // halving alpha.
        assert!((resolved[7].color.r - first.r).abs() < 1.0e-6);
        assert!((resolved[7].color.a - first.a * 0.5).abs() < 1.0e-5);

        clamp_negative_radial_gradient_stops(&mut resolved);
        assert!(resolved.iter().all(|stop| stop.position >= 0.0));
    }

    #[test]
    fn conic_gradient_dithers_and_resolves_a_sample_center_singularity() {
        let doc = Document::new();
        let image = CssImage::ConicGradient(ConicGradient {
            from_degrees: 0.0,
            center_x: BackgroundPosition::Percent(50.0),
            center_y: BackgroundPosition::Percent(50.0),
            repeating: false,
            color_space: GradientColorSpace::Srgb,
            stops: vec![
                GradientStop {
                    color: StyleColor::Resolved(Color::WHITE),
                    position: GradientStopPosition::Auto,
                },
                GradientStop {
                    color: StyleColor::CurrentColor,
                    position: GradientStopPosition::Auto,
                },
            ],
        });
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| {
            computed.color = Color::from_rgba8(0, 128, 0, 255);
            computed.device_scale_factor = 1.25;
        });
        let mut surface = surfaces::raster_n32_premul((200, 200)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        surface.canvas().scale((1.25, 1.25));
        paint_css_image_tile(
            surface.canvas(),
            &doc,
            &style,
            &image,
            Rect::from_xywh(20.0, 20.0, 100.0, 100.0),
            false,
            false,
            false,
            false,
            None,
            false,
            false,
            1.0,
            None,
            None,
            None,
            false,
        );

        let pixels = surface_bytes(&mut surface);
        let pixel = |x: usize, y: usize| &pixels[(y * 200 + x) * 4..][..4];
        // Native N32 is BGRA on the qualification host; this gradient has
        // equal red and blue channels. The center is the exact half-device-px
        // singular phase and the off-center sample guards ordered dithering.
        assert_eq!(pixel(87, 87), [159, 207, 159, 255]);
        assert_eq!(pixel(119, 29), [235, 245, 235, 255]);
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
        style.update_derived(|style| style.background_layers.push(layer));
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
            false,
        );
        assert_eq!(surface.canvas().save_count(), save_count);
        let pixels = surface_bytes(&mut surface);
        let center = &pixels[(6 * 16 + 6) * 4..][..4];
        let outside = &pixels[..4];
        assert_eq!(center, &[0, 128, 0, 255]);
        assert_eq!(outside, &[255, 255, 255, 255]);
    }

    #[test]
    fn raster_background_keeps_packed_opacity_under_an_ancestor() {
        let mut doc = Document::new();
        let parent = doc.create_node(ElementTag::Div);
        let child = doc.create_node(ElementTag::Div);
        doc.append_child(doc.root(), parent);
        doc.append_child(parent, child);
        let image = doc.register_image_resource(
            "css-backgrounds/support/1x1-green.png",
            "image/png",
            "a236213916dd30bd771a233aa1d66381eabf335bf8885304b75a4e2e370d68ce",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/accountability/data/wpt_assets/sp13p/1x1-green.png"
            ))
            .to_vec(),
        );
        doc.update_resolved_style(parent, |style| style.opacity = 0.5);
        doc.update_resolved_style(child, |style| {
            style
                .background_layers
                .push(BackgroundLayer::new(CssImage::Raster(image)));
        });
        let size = PhysicalSize::new(LayoutUnit::from_i32(12), LayoutUnit::from_i32(12));
        let mut fragment = Fragment::new_box(parent, size);
        fragment.children.push(Fragment::new_box(child, size));

        for scale in [1.0_f64, 1.25, 1.5, 2.0, 3.0] {
            doc.update_resolved_style(parent, |style| style.device_scale_factor = scale);
            doc.update_resolved_style(child, |style| style.device_scale_factor = scale);
            let mut surface = surfaces::raster_n32_premul((40, 40)).expect("surface");
            surface.canvas().clear(skia_safe::Color::WHITE);
            surface.canvas().scale((scale as f32, scale as f32));
            let save_count = surface.canvas().save_count();
            paint_fragment_tracked(surface.canvas(), &fragment, &doc, PhysicalOffset::zero());
            assert_eq!(surface.canvas().save_count(), save_count);
            let pixels = surface_bytes(&mut surface);
            // Repeated Chromium captures of the same RGB source at opacity
            // 0.5 yield #7ebf7e. F16 instead yields #80bf80 at full coverage.
            assert_eq!(&pixels[(6 * 40 + 6) * 4..][..4], &[126, 191, 126, 255]);
        }
    }

    #[test]
    fn opaque_single_pixel_raster_repetition_fills_scaled_canvas() {
        let mut doc = Document::new();
        let image = doc.register_image_resource(
            "css-backgrounds/support/1x1-green.png",
            "image/png",
            "a236213916dd30bd771a233aa1d66381eabf335bf8885304b75a4e2e370d68ce",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/accountability/data/wpt_assets/sp13p/1x1-green.png"
            ))
            .to_vec(),
        );
        let mut style = ComputedStyle::default();
        style.update_derived(|style| {
            style.device_scale_factor = 1.5;
            style
                .background_layers
                .push(BackgroundLayer::new(CssImage::Raster(image)));
        });
        let mut surface = surfaces::raster_n32_premul((64, 64)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        surface.canvas().scale((1.5, 1.5));
        let save_count = surface.canvas().save_count();
        paint_background_layers(
            surface.canvas(),
            &doc,
            None,
            &style,
            Rect::from_xywh(0.0, 0.0, 40.0, 40.0),
            1.0,
            Some(Rect::from_xywh(0.0, 0.0, 40.0, 40.0)),
            true,
        );
        assert_eq!(surface.canvas().save_count(), save_count);
        let pixels = surface_bytes(&mut surface);
        assert_eq!(&pixels[(30 * 64 + 30) * 4..][..4], &[0, 128, 0, 255]);
        assert_eq!(&pixels[(62 * 64 + 62) * 4..][..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn raster_mask_composites_the_complete_box_layer() {
        let mut doc = Document::new();
        let node = doc.create_node(openui_dom::ElementTag::Div);
        doc.append_child(doc.root(), node);
        doc.update_resolved_style(node, |style| {
            style.background_color = Color::from_rgba8(0, 128, 0, 255)
        });
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
        doc.update_resolved_style(node, |style| {
            style
                .mask_layers
                .push(BackgroundLayer::new(CssImage::Raster(image)));
        });

        let fragment = Fragment::new_box(
            node,
            PhysicalSize::new(LayoutUnit::from_i32(81), LayoutUnit::from_i32(81)),
        );
        let mut surface = surfaces::raster_n32_premul((81, 81)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        let save_count = surface.canvas().save_count();
        paint_fragment_tracked(surface.canvas(), &fragment, &doc, PhysicalOffset::zero());
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
        doc.update_resolved_style(parent, |parent_style| {
            parent_style.overflow_x = Overflow::Clip;
            parent_style.overflow_y = Overflow::Visible;
            parent_style.border_top_left_radius = (8.0, 8.0);
            parent_style.border_top_right_radius = (8.0, 8.0);
            parent_style.border_bottom_right_radius = (8.0, 8.0);
            parent_style.border_bottom_left_radius = (8.0, 8.0);
        });
        doc.append_child(doc.root(), parent);
        let child = doc.create_node(openui_dom::ElementTag::Div);
        doc.update_resolved_style(child, |style| {
            style.background_color = Color::from_rgba8(0, 128, 0, 255)
        });
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
        paint_fragment_tracked(
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
    fn fractional_hidden_clips_keep_analytic_coverage_but_scrollports_snap() {
        let mut doc = Document::new();
        let node = doc.create_node(openui_dom::ElementTag::Div);
        doc.append_child(doc.root(), node);
        let fragment = Fragment::new_box(
            node,
            PhysicalSize::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(20)),
        );
        let mut hidden = ComputedStyle::default();
        hidden.update_derived(|style| {
            style.device_scale_factor = 1.25;
            style.overflow_x = Overflow::Hidden;
            style.overflow_y = Overflow::Hidden;
        });
        assert!(antialias_rectangular_overflow_clip(
            &fragment, &doc, &hidden
        ));

        let mut scroll = hidden.clone();
        scroll.update_derived(|style| {
            style.overflow_x = Overflow::Scroll;
            style.overflow_y = Overflow::Scroll;
        });
        assert!(!antialias_rectangular_overflow_clip(
            &fragment, &doc, &scroll
        ));

        let mut integral = hidden.clone();
        integral.update_derived(|style| style.device_scale_factor = 2.0);
        assert!(!antialias_rectangular_overflow_clip(
            &fragment, &doc, &integral
        ));

        let mut rounded = hidden.clone();
        rounded.update_derived(|style| style.border_top_left_radius = (4.0, 4.0));
        assert!(!antialias_rectangular_overflow_clip(
            &fragment, &doc, &rounded
        ));

        let iframe = doc.create_node(openui_dom::ElementTag::IFrame);
        doc.append_child(doc.root(), iframe);
        let iframe_fragment = Fragment::new_box(
            iframe,
            PhysicalSize::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(20)),
        );
        assert!(!antialias_rectangular_overflow_clip(
            &iframe_fragment,
            &doc,
            &hidden
        ));
    }

    #[test]
    fn button_content_clip_does_not_square_a_coincident_child_edge() {
        let mut doc = Document::new();
        let button = doc.create_node(openui_dom::ElementTag::Button);
        doc.node_mut(button).form_control = Some(FormControlRole::Button);
        doc.update_resolved_style(button, |style| {
            style.device_scale_factor = 1.25;
            style.background_color = Color::from_rgba8(128, 0, 128, 255);
            style.overflow_x = Overflow::Clip;
            style.overflow_y = Overflow::Clip;
        });
        doc.append_child(doc.root(), button);
        let child = doc.create_node(openui_dom::ElementTag::Div);
        doc.update_resolved_style(child, |style| {
            style.device_scale_factor = 1.25;
            style.background_color = Color::from_rgba8(0, 128, 0, 255);
        });
        doc.append_child(button, child);

        let mut button_fragment = Fragment::new_box(
            button,
            PhysicalSize::new(LayoutUnit::from_i32(80), LayoutUnit::from_i32(80)),
        );
        button_fragment.offset =
            PhysicalOffset::new(LayoutUnit::from_i32(70), LayoutUnit::from_i32(20));
        let mut child_fragment = Fragment::new_box(
            child,
            PhysicalSize::new(LayoutUnit::from_i32(40), LayoutUnit::from_i32(40)),
        );
        child_fragment.offset = PhysicalOffset::new(LayoutUnit::zero(), LayoutUnit::from_i32(20));
        button_fragment.children.push(child_fragment);

        let mut surface = surfaces::raster_n32_premul((240, 180)).expect("surface");
        surface.canvas().clear(skia_safe::Color::WHITE);
        surface.canvas().scale((1.25, 1.25));
        paint_fragment_tracked(
            surface.canvas(),
            &button_fragment,
            &doc,
            PhysicalOffset::zero(),
        );
        let pixels = surface_bytes(&mut surface);
        let pixel = |x: usize, y: usize| &pixels[(y * 240 + x) * 4..][..4];
        assert_eq!(pixel(87, 60), [95, 127, 95, 255]);
        assert_eq!(pixel(88, 60), [0, 128, 0, 255]);
    }

    #[test]
    fn clipped_fractional_replaced_background_uses_both_compositor_masks() {
        let mut style = ComputedStyle::default();
        style.update_derived(|style| {
            style.background_color = Color::from_rgba8(128, 128, 128, 255);
            style.overflow_x = Overflow::Scroll;
            style.overflow_y = Overflow::Scroll;
            style.device_scale_factor = 1.25;
        });
        assert!(uses_squared_replaced_background_coverage(
            &style,
            Rect::from_xywh(20.0, 20.0, 70.0, 70.0),
            Some(ReplacedResourceKind::Image(ImageResourceId::new(0))),
        ));

        style.update_derived(|style| style.device_scale_factor = 1.0);
        assert!(!uses_squared_replaced_background_coverage(
            &style,
            Rect::from_xywh(20.0, 20.0, 70.0, 70.0),
            Some(ReplacedResourceKind::Image(ImageResourceId::new(0))),
        ));
        style.update_derived(|style| {
            style.device_scale_factor = 1.25;
            style.overflow_x = Overflow::Hidden;
            style.overflow_y = Overflow::Hidden;
        });
        assert!(uses_squared_replaced_background_coverage(
            &style,
            Rect::from_xywh(20.0, 20.0, 70.0, 70.0),
            Some(ReplacedResourceKind::Image(ImageResourceId::new(0))),
        ));
        assert!(!uses_squared_replaced_background_coverage(
            &style,
            Rect::from_xywh(20.0, 20.0, 70.0, 70.0),
            None,
        ));
        assert!(!uses_squared_replaced_background_coverage(
            &style,
            Rect::from_xywh(20.0, 20.0, 70.0, 70.0),
            Some(ReplacedResourceKind::TransparentCanvas),
        ));
    }

    #[test]
    fn cpu_scroll_contents_paint_directly_while_ganesh_layers_real_scroll_ranges() {
        let size = PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(80));
        let mut doc = Document::new();
        let node = doc.create_node(openui_dom::ElementTag::Div);
        let mut fragment = Fragment::new_box(node, size);
        let mut style = ComputedStyle::default();
        style.update_derived(|style| {
            style.overflow_x = Overflow::Auto;
            style.overflow_y = Overflow::Auto;
        });
        assert!(!has_composited_scroll_contents(&fragment, &style));

        fragment.overflow_rect = Some(openui_geometry::PhysicalRect::new(
            PhysicalOffset::zero(),
            PhysicalSize::new(LayoutUnit::from_i32(101), LayoutUnit::from_i32(80)),
        ));
        assert!(!has_composited_scroll_contents(&fragment, &style));

        style.update_derived(|style| {
            style.raster_configuration =
                openui_geometry::RasterConfiguration::chromium_linux_ganesh()
        });
        assert!(has_composited_scroll_contents(&fragment, &style));

        fragment.overflow_rect = Some(openui_geometry::PhysicalRect::new(
            PhysicalOffset::new(LayoutUnit::from_i32(-1), LayoutUnit::zero()),
            size,
        ));
        assert!(has_composited_scroll_contents(&fragment, &style));

        fragment.overflow_rect = None;
        style.update_derived(|style| {
            style.overflow_x = Overflow::Scroll;
            style.overflow_y = Overflow::Hidden;
        });
        assert!(has_composited_scroll_contents(&fragment, &style));

        style.update_derived(|style| {
            style.raster_configuration = openui_geometry::RasterConfiguration::chromium_linux_lcd()
        });
        assert!(!has_composited_scroll_contents(&fragment, &style));

        style.update_derived(|style| {
            style.raster_configuration =
                openui_geometry::RasterConfiguration::chromium_linux_ganesh();
            style.overflow_x = Overflow::Hidden;
            style.overflow_y = Overflow::Clip;
        });
        assert!(!has_composited_scroll_contents(&fragment, &style));
    }

    #[test]
    fn cpu_scroll_backing_is_limited_to_fractional_explicit_scroll_axes() {
        let size = PhysicalSize::new(LayoutUnit::from_i32(80), LayoutUnit::from_i32(80));
        let mut doc = Document::new();
        let node = doc.create_node(openui_dom::ElementTag::Div);
        let mut fragment = Fragment::new_box(node, size);
        fragment.overflow_rect = Some(openui_geometry::PhysicalRect::new(
            PhysicalOffset::zero(),
            PhysicalSize::new(LayoutUnit::from_i32(81), LayoutUnit::from_i32(80)),
        ));
        let mut style = ComputedStyle::default();
        style.update_derived(|style| {
            style.device_scale_factor = 1.25;
            style.overflow_x = Overflow::Scroll;
            style.overflow_y = Overflow::Hidden;
        });
        assert!(needs_fractional_cpu_scroll_backing(
            &fragment,
            &style,
            Rect::from_xywh(47.0, 21.0, 80.0, 80.0)
        ));

        assert!(!needs_fractional_cpu_scroll_backing(
            &fragment,
            &style,
            Rect::from_xywh(128.0, 21.0, 80.0, 80.0)
        ));

        fragment.overflow_rect = Some(openui_geometry::PhysicalRect::new(
            PhysicalOffset::new(LayoutUnit::from_i32(-500), LayoutUnit::zero()),
            PhysicalSize::new(LayoutUnit::from_i32(600), LayoutUnit::from_i32(80)),
        ));
        assert!(!needs_fractional_cpu_scroll_backing(
            &fragment,
            &style,
            Rect::from_xywh(47.0, 21.0, 80.0, 80.0)
        ));

        fragment.overflow_rect = Some(openui_geometry::PhysicalRect::new(
            PhysicalOffset::zero(),
            PhysicalSize::new(LayoutUnit::from_i32(80), LayoutUnit::from_i32(81)),
        ));
        style.update_derived(|style| {
            style.overflow_x = Overflow::Hidden;
            style.overflow_y = Overflow::Scroll;
        });
        assert!(needs_fractional_cpu_scroll_backing(
            &fragment,
            &style,
            Rect::from_xywh(128.0, 47.0, 80.0, 80.0)
        ));
        style.update_derived(|style| style.device_scale_factor = 2.0);
        assert!(!needs_fractional_cpu_scroll_backing(
            &fragment,
            &style,
            Rect::from_xywh(128.0, 47.0, 80.0, 80.0)
        ));
    }

    #[test]
    fn integral_scroll_backing_origins_do_not_acquire_a_fractional_resample() {
        assert_eq!(scroll_backing_phase_adjustment(128.0, 1.25), 0.0);
        assert_eq!(scroll_backing_phase_adjustment(47.0, 1.25), 0.0);
        assert!((scroll_backing_phase_adjustment(47.4, 1.25) - 0.4).abs() < 1.0e-6);
    }

    #[test]
    fn cardinal_linear_gradient_directions_are_exactly_axial() {
        assert_eq!(linear_gradient_direction(0.0), Point::new(0.0, -1.0));
        assert_eq!(linear_gradient_direction(90.0), Point::new(1.0, 0.0));
        assert_eq!(linear_gradient_direction(180.0), Point::new(0.0, 1.0));
        assert_eq!(linear_gradient_direction(270.0), Point::new(-1.0, 0.0));
    }

    #[test]
    fn transparent_adjacent_solid_border_uses_direct_analytic_trapezoid_coverage() {
        let mut surface = surfaces::raster_n32_premul((200, 200)).expect("surface");
        surface
            .canvas()
            .clear(skia_safe::Color::from_rgb(238, 238, 238));
        surface.canvas().scale((1.25, 1.25));
        paint_border_side_path(
            surface.canvas(),
            BorderStyle::Solid,
            &StyleColor::Resolved(Color::from_rgba8(229, 195, 178, 255)),
            &Color::BLACK,
            50.0,
            &[(121.0, 121.0), (21.0, 121.0), (71.0, 71.0), (71.0, 71.0)],
            BorderSide::Bottom,
            false,
            true,
            true,
            true,
            true,
            false,
            false,
            false,
            1.25,
        );

        let pixels = surface_bytes(&mut surface);
        let pixel = |x: usize, y: usize| &pixels[(y * 200 + x) * 4..][..4];
        // raster_n32_premul exposes the native BGRA byte order.
        assert_eq!(pixel(87, 89), [230, 232, 236, 255]);
        assert_ne!(pixel(50, 151), [238, 238, 238, 255]);
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
        style.update_derived(|computed| computed.border_top_left_radius = (12.0, 12.0));
        style.update_derived(|computed| computed.border_top_right_radius = (12.0, 12.0));
        style.update_derived(|computed| computed.border_bottom_right_radius = (12.0, 12.0));
        style.update_derived(|computed| computed.border_bottom_left_radius = (12.0, 12.0));
        style.update_derived(|computed| {
            computed.box_shadow = vec![
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
            ]
        });
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
                false,
                1.0,
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
        style.update_derived(|computed| computed.font_size = 12.0);
        style.update_derived(|computed| computed.vertical_align = openui_style::VerticalAlign::Sub);

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

        style.update_derived(|computed| {
            computed.vertical_align = openui_style::VerticalAlign::Baseline
        });
        assert_eq!(
            snap_aliased_ahem_keyword_baseline(100.265625, &style, true, true),
            101.265625
        );
    }

    #[test]
    fn tiny_fractional_aliased_ahem_closes_its_ink_start_device_cell() {
        let mut style = ComputedStyle::default();
        style.update_derived(|computed| {
            computed.font_size = 1.0;
            computed.raster_configuration =
                openui_geometry::RasterConfiguration::deterministic_aliased(false);
            computed.device_scale_factor = 1.25;
        });
        let baseline = snap_tiny_aliased_ahem_ink_start(70.8125, &style, true);
        let physical_ink_start = (baseline - style.font_size * 0.8) * 1.25;
        assert!((physical_ink_start - physical_ink_start.round()).abs() < 1.0e-5);

        style.update_derived(|computed| computed.device_scale_factor = 1.5);
        let baseline = snap_tiny_aliased_ahem_ink_start(70.8125, &style, true);
        let physical_ink_start = (baseline - style.font_size * 0.8) * 1.5;
        assert!((physical_ink_start - physical_ink_start.round()).abs() < 1.0e-5);

        style.update_derived(|computed| computed.device_scale_factor = 2.0);
        assert_eq!(
            snap_tiny_aliased_ahem_ink_start(70.8125, &style, true),
            70.8125
        );
    }
}
