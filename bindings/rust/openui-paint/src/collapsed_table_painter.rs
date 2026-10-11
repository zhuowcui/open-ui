//! Retained collapsed-table edge painting.
//!
//! Geometry and ordered operations follow Chromium 147.0.7727.50
//! core/paint/table_painters.cc:35-332,518-727. DrawBoxSide follows
//! core/paint/box_border_painter.cc:410-925,2465-2476, with zero adjacent
//! widths. Colors follow platform/graphics/color.cc:1138-1181 and
//! ui/gfx/color_utils.cc:58-63,392-412. No output pixels choose this path.

use crate::paint_record::PaintCanvas;
use openui_dom::Document;
use openui_geometry::{
    LayoutUnit, LogicalOffset, LogicalRect, LogicalSize, PhysicalOffset, PhysicalRect,
    WritingModeConverter,
};
use openui_layout::{
    BorderCandidate, CollapsedTableBorderData, Fragment, PhysicalBorderSide,
    ResolvedCollapsedBorders, ResolvedEdge,
};
use openui_style::{BorderStyle, Color};
use skia_safe::{Color4f, ColorSpace, Paint, PaintStyle, PathEffect, Point, Rect};

type Canvas = dyn PaintCanvas;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Inline,
    Block,
}

#[derive(Clone, Copy)]
struct Edge {
    identity: Option<usize>,
    candidate: Option<BorderCandidate>,
}

impl Edge {
    const EMPTY: Self = Self {
        identity: None,
        candidate: None,
    };
    fn width(self) -> LayoutUnit {
        self.candidate.map_or(LayoutUnit::zero(), |c| c.width)
    }
    fn can_paint(self) -> bool {
        self.candidate
            .is_some_and(|c| c.style.has_visible_border() && c.width != LayoutUnit::zero())
    }
}

fn edge(borders: &ResolvedCollapsedBorders, axis: Axis, row: isize, col: isize) -> Edge {
    if row < 0 || col < 0 {
        return Edge::EMPTY;
    }
    let (row, col) = (row as usize, col as usize);
    let (resolved, identity) = match axis {
        Axis::Inline if row <= borders.row_count() && col < borders.column_count() => (
            borders.block_edge(row, col),
            row * (2 * (borders.column_count() + 1)) + 2 * col + 1,
        ),
        Axis::Block if row < borders.row_count() && col <= borders.column_count() => (
            borders.inline_edge(row, col),
            row * (2 * (borders.column_count() + 1)) + 2 * col,
        ),
        _ => return Edge::EMPTY,
    };
    Edge {
        identity: Some(identity),
        candidate: match resolved {
            ResolvedEdge::Winner(c) => Some(c),
            _ => None,
        },
    }
}

fn style_priority(style: BorderStyle) -> u8 {
    match style {
        BorderStyle::None => 0,
        BorderStyle::Hidden => 1,
        BorderStyle::Inset => 2,
        BorderStyle::Groove => 3,
        BorderStyle::Outset => 4,
        BorderStyle::Ridge => 5,
        BorderStyle::Dotted => 6,
        BorderStyle::Dashed => 7,
        BorderStyle::Solid => 8,
        BorderStyle::Double => 9,
    }
}

// The pinned comment has the opposite sign; the actual implementation gives
// +1 to the winning lhs. Color and source-node identity never break a tie.
fn compare(a: Edge, b: Edge) -> i8 {
    if a.identity == b.identity {
        return 0;
    }
    match (a.can_paint(), b.can_paint()) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => -1,
        (true, true) => {
            let a = a.candidate.unwrap();
            let b = b.candidate.unwrap();
            let ordering = a
                .width
                .cmp(&b.width)
                .then_with(|| style_priority(a.style).cmp(&style_priority(b.style)))
                .then_with(|| b.box_order.cmp(&a.box_order));
            match ordering {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Joint {
    size: LogicalSize,
    wins: bool,
}

fn joint(
    borders: &ResolvedCollapsedBorders,
    axis: Axis,
    row: usize,
    col: usize,
    at_end: bool,
    over_boundary: bool,
    under_boundary: bool,
) -> Joint {
    let row = row as isize + if at_end && axis == Axis::Block { 1 } else { 0 };
    let col = col as isize + if at_end && axis == Axis::Inline { 1 } else { 0 };
    let before = edge(borders, Axis::Inline, row, col - 1);
    let after = edge(borders, Axis::Inline, row, col);
    let omit_over = over_boundary && (!at_end || axis == Axis::Inline);
    let omit_under = under_boundary && (at_end || axis == Axis::Inline);
    let over = if omit_over {
        Edge::EMPTY
    } else {
        edge(borders, Axis::Block, row - 1, col)
    };
    let under = if omit_under {
        Edge::EMPTY
    } else {
        edge(borders, Axis::Block, row, col)
    };
    let inline_compare = compare(before, after);
    let block_compare = compare(over, under);
    let inline_winner = if inline_compare == 1 { before } else { after };
    let block_winner = if block_compare == 1 { over } else { under };
    let cross = compare(inline_winner, block_winner);
    Joint {
        size: LogicalSize::new(
            block_winner.width(),
            if omit_over || omit_under {
                LayoutUnit::zero()
            } else {
                inline_winner.width()
            },
        ),
        wins: match axis {
            Axis::Inline => cross != -1 && inline_compare != if at_end { -1 } else { 1 },
            Axis::Block => cross != 1 && block_compare != if at_end { -1 } else { 1 },
        },
    }
}

// Blink's SnapSizeToPixel also preserves a nonzero size above four raw units
// when paired rounding would erase it. CSS snapping precedes device scale.
fn snap_size(size: LayoutUnit, origin: LayoutUnit) -> i32 {
    let fraction = origin.fraction();
    let snapped = (fraction + size).round_i32() - fraction.round_i32();
    if snapped == 0 && (size.raw() > 4 || size.raw() < -4) {
        size.raw().signum()
    } else {
        snapped
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SnappedRect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl SnappedRect {
    fn from_physical(rect: PhysicalRect) -> Self {
        let x1 = rect.offset.left.round_i32();
        let y1 = rect.offset.top.round_i32();
        Self {
            x1,
            y1,
            x2: x1 + snap_size(rect.size.width, rect.offset.left),
            y2: y1 + snap_size(rect.size.height, rect.offset.top),
        }
    }
    fn rect(self) -> Rect {
        Rect::from_ltrb(
            self.x1 as f32,
            self.y1 as f32,
            self.x2 as f32,
            self.y2 as f32,
        )
    }
}

pub(crate) fn pixel_snapped_rect(rect: PhysicalRect) -> Rect {
    SnappedRect::from_physical(rect).rect()
}

fn source_color(doc: &Document, candidate: BorderCandidate) -> Color {
    let style = &doc.node(candidate.node_id).style;
    let color = match candidate.physical_side {
        PhysicalBorderSide::Top => style.border_top_color,
        PhysicalBorderSide::Right => style.border_right_color,
        PhysicalBorderSide::Bottom => style.border_bottom_color,
        PhysicalBorderSide::Left => style.border_left_color,
    };
    color.resolve(&style.color)
}

/// Consume actual section projections, retaining one immutable winner graph.
/// A continuation without sections contributes no borders; it never replays
/// the full unfragmented graph or reconstructs rows from authored cell boxes.
pub(crate) fn paint(canvas: &Canvas, table: &Fragment, doc: &Document, offset: PhysicalOffset) {
    let Some(data) = table.collapsed_table_border_data.as_ref() else {
        return;
    };
    let sections: Vec<_> = table
        .children
        .iter()
        .filter(|c| {
            c.collapsed_table_section
                .as_ref()
                .is_some_and(|section| section.row_offsets.len() >= 2)
        })
        .collect();
    let mut previous_painted_row = None;
    for (section_index, section) in sections.iter().enumerate() {
        let projection = section.collapsed_table_section.as_ref().unwrap();
        let offsets = &projection.row_offsets;
        if offsets.len() < 2 {
            continue;
        }
        let start_row = projection.global_start_row;
        let previous_content = section_index == 0 && start_row > 0;
        let next_content = section_index + 1 == sections.len()
            && start_row + offsets.len() < data.borders.row_count() + 1;
        let converter = WritingModeConverter::new(data.writing_direction, section.size);
        let physical_offset = PhysicalOffset::new(
            offset.left + section.offset.left,
            offset.top + section.offset.top,
        );
        for local_row in 0..offsets.len() {
            let row = start_row + local_row;
            if row > data.borders.row_count() {
                break;
            }
            for col in 0..=data.borders.column_count() {
                // The pinned serialized edge grid interleaves V then H.
                for axis in [Axis::Block, Axis::Inline] {
                    let edge = edge(&data.borders, axis, row as isize, col as isize);
                    if !edge.can_paint() {
                        continue;
                    }
                    let Some(candidate) = edge.candidate else {
                        continue;
                    };
                    let Some(column_start) = column_boundary(data, col) else {
                        continue;
                    };
                    let first_row = local_row == 0;
                    let start_fragmented = first_row && projection.is_start_row_fragmented;
                    let over_boundary = first_row && previous_content;
                    let last_row = match axis {
                        Axis::Inline => local_row + 1 == offsets.len(),
                        Axis::Block => local_row + 2 == offsets.len(),
                    };
                    let end_fragmented = last_row && projection.is_end_row_fragmented;
                    let under_boundary = last_row && next_content;
                    let width = candidate.width;
                    let mut origin;
                    let mut size;
                    match axis {
                        Axis::Inline => {
                            if previous_painted_row == Some(row)
                                || start_fragmented
                                || end_fragmented
                            {
                                continue;
                            }
                            let Some(next_column) = column_boundary(data, col + 1) else {
                                continue;
                            };
                            origin = LogicalOffset::new(
                                column_start,
                                if over_boundary {
                                    offsets[local_row]
                                } else {
                                    offsets[local_row] - width / 2
                                },
                            );
                            size = LogicalSize::new(
                                next_column - column_start,
                                if over_boundary || under_boundary {
                                    width / 2
                                } else {
                                    width
                                },
                            );
                        }
                        Axis::Block => {
                            if local_row + 1 >= offsets.len() {
                                continue;
                            }
                            origin =
                                LogicalOffset::new(column_start - width / 2, offsets[local_row]);
                            size = LogicalSize::new(
                                width,
                                offsets[local_row + 1] - offsets[local_row],
                            );
                        }
                    }
                    let start = joint(
                        &data.borders,
                        axis,
                        row,
                        col,
                        false,
                        over_boundary,
                        under_boundary,
                    );
                    let end = joint(
                        &data.borders,
                        axis,
                        row,
                        col,
                        true,
                        over_boundary,
                        under_boundary,
                    );
                    match axis {
                        Axis::Inline => {
                            let half = start.size.inline_size / 2;
                            if start.wins {
                                origin.inline_offset = origin.inline_offset - half;
                                size.inline_size = size.inline_size + half;
                            } else {
                                origin.inline_offset = origin.inline_offset + half;
                                size.inline_size = size.inline_size - half;
                            }
                            let half = end.size.inline_size / 2;
                            size.inline_size = if end.wins {
                                size.inline_size + half
                            } else {
                                size.inline_size - half
                            };
                        }
                        Axis::Block => {
                            if !start_fragmented {
                                let half = start.size.block_size / 2;
                                if start.wins {
                                    origin.block_offset = origin.block_offset - half;
                                    size.block_size = size.block_size + half;
                                } else {
                                    origin.block_offset = origin.block_offset + half;
                                    size.block_size = size.block_size - half;
                                }
                            }
                            if !end_fragmented {
                                let half = end.size.block_size / 2;
                                size.block_size = if end.wins {
                                    size.block_size + half
                                } else {
                                    size.block_size - half
                                };
                            }
                        }
                    }
                    let mut rect = converter.to_physical_rect(LogicalRect::new(origin, size));
                    rect.offset.left = rect.offset.left + physical_offset.left;
                    rect.offset.top = rect.offset.top + physical_offset.top;
                    let horizontal =
                        (axis == Axis::Inline) == data.writing_direction.is_horizontal();
                    draw_side(
                        canvas,
                        SnappedRect::from_physical(rect),
                        horizontal,
                        source_color(doc, candidate),
                        candidate.style,
                    );
                }
            }
        }
        previous_painted_row = if projection.is_end_row_fragmented {
            None
        } else {
            Some(start_row + offsets.len() - 1)
        };
    }
}

fn column_boundary(data: &CollapsedTableBorderData, col: usize) -> Option<LayoutUnit> {
    if let Some(column) = data.columns.get(col) {
        Some(column.offset)
    } else if col == data.columns.len() {
        data.columns.last().map(|c| c.offset + c.size)
    } else {
        None
    }
}

fn color_paint(color: Color, aa: bool) -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(aa);
    paint.set_color4f(
        Color4f::new(color.r, color.g, color.b, color.a),
        None::<&ColorSpace>,
    );
    paint
}

fn fill(canvas: &Canvas, rect: SnappedRect, color: Color) {
    canvas.draw_rect(rect.rect(), &color_paint(color, true));
}

fn draw_side(
    canvas: &Canvas,
    rect: SnappedRect,
    horizontal: bool,
    color: Color,
    style: BorderStyle,
) {
    let thickness = if horizontal {
        rect.y2 - rect.y1
    } else {
        rect.x2 - rect.x1
    };
    let length = if horizontal {
        rect.x2 - rect.x1
    } else {
        rect.y2 - rect.y1
    };
    if thickness <= 0 || length <= 0 {
        return;
    }
    let style = match style {
        BorderStyle::Double if thickness < 3 => BorderStyle::Solid,
        BorderStyle::Ridge | BorderStyle::Groove if thickness <= 1 => BorderStyle::Solid,
        other => other,
    };
    match style {
        BorderStyle::None | BorderStyle::Hidden => {}
        BorderStyle::Solid => fill(canvas, rect, color),
        BorderStyle::Double => {
            let third = (thickness + 1) / 3;
            let mut lead = rect;
            let mut trail = rect;
            if horizontal {
                lead.y2 = lead.y1 + third;
                trail.y1 = trail.y2 - third;
            } else {
                lead.x2 = lead.x1 + third;
                trail.x1 = trail.x2 - third;
            }
            fill(canvas, lead, color);
            fill(canvas, trail, color);
        }
        BorderStyle::Ridge | BorderStyle::Groove => {
            let mut lead = rect;
            let mut trail = rect;
            // Absolute midpoint, C++ integer truncation even below zero.
            if horizontal {
                let mid = split_midpoint(rect.y1, rect.y2);
                lead.y2 = mid;
                trail.y1 = mid;
            } else {
                let mid = split_midpoint(rect.x1, rect.x2);
                lead.x2 = mid;
                trail.x1 = mid;
            }
            let (first, second) = if style == BorderStyle::Groove {
                (BorderStyle::Inset, BorderStyle::Outset)
            } else {
                (BorderStyle::Outset, BorderStyle::Inset)
            };
            draw_side(canvas, lead, horizontal, color, first);
            draw_side(canvas, trail, horizontal, color, second);
        }
        // Collapsed edges always draw Top or Left, independently of the
        // source border-color side. Both sides darken for Inset.
        BorderStyle::Inset => fill(canvas, rect, css_dark(color)),
        BorderStyle::Outset => fill(canvas, rect, css_light_side(color)),
        BorderStyle::Dashed | BorderStyle::Dotted => {
            draw_pattern(canvas, rect, horizontal, color, thickness, length, style)
        }
    }
}

fn split_midpoint(start: i32, end: i32) -> i32 {
    ((i64::from(start) + i64::from(end) + 1) / 2) as i32
}

fn quantize8(value: f32) -> f32 {
    ((value * f32::from_bits(256.0f32.to_bits() - 1)) as i32) as f32 / 255.0
}

fn css_dark(color: Color) -> Color {
    if color == Color::WHITE {
        return Color::from_rgba8(171, 171, 171, 255);
    }
    let v = color.r.max(color.g).max(color.b);
    let multiplier = if v == 0.0 {
        0.0
    } else {
        ((v - 0.33) / v).max(0.0)
    };
    Color {
        r: quantize8(color.r * multiplier),
        g: quantize8(color.g * multiplier),
        b: quantize8(color.b * multiplier),
        a: color.a,
    }
}

fn css_light(color: Color) -> Color {
    let v = color.r.max(color.g).max(color.b);
    if v == 0.0 {
        return Color {
            r: 84.0 / 255.0,
            g: 84.0 / 255.0,
            b: 84.0 / 255.0,
            a: color.a,
        };
    }
    let multiplier = (v + 0.33).min(1.0) / v;
    Color {
        r: quantize8(color.r * multiplier),
        g: quantize8(color.g * multiplier),
        b: quantize8(color.b * multiplier),
        a: color.a,
    }
}

fn luminance(color: Color) -> f32 {
    let linearize = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b)
}

fn css_light_side(color: Color) -> Color {
    if color.r >= 150.0 / 255.0 || color.g >= 92.0 / 255.0 {
        return color;
    }
    let l1 = luminance(color) + 0.05;
    let l2 = luminance(css_dark(color)) + 0.05;
    if l1.max(l2) / l1.min(l2) < 1.75 {
        css_light(color)
    } else {
        color
    }
}

// platform/graphics/styled_stroke_data.cc:49-143. Open-line dash setup is
// based on the original integer length, before endpoint-dot adjustments.
fn pattern(length: i32, thickness: i32, style: BorderStyle) -> Option<([f32; 2], bool)> {
    let width = thickness as f32;
    let length = length as f32;
    if style == BorderStyle::Dashed || thickness <= 3 {
        let dash = width
            * if style == BorderStyle::Dashed {
                if width >= 3.0 {
                    2.0
                } else {
                    3.0
                }
            } else {
                1.0
            };
        let gap = width
            * if style == BorderStyle::Dashed {
                if width >= 3.0 {
                    1.0
                } else {
                    2.0
                }
            } else {
                1.0
            };
        if length <= 2.0 * dash {
            return None;
        }
        if length <= 2.0 * dash + gap {
            let m = length / (2.0 * dash + gap);
            return Some(([dash * m, gap * m], false));
        }
        Some((
            [
                dash,
                if style == BorderStyle::Dashed {
                    select_gap(length, dash, gap)
                } else {
                    gap
                },
            ],
            false,
        ))
    } else if length < 2.0 * width {
        Some(([0.0, 2.0 * width], true))
    } else {
        Some(([0.0, select_gap(length, width, width) + width - 0.01], true))
    }
}

fn select_gap(length: f32, dash: f32, gap: f32) -> f32 {
    let min_dashes = ((length + gap) / (dash + gap)).floor();
    let max_dashes = min_dashes + 1.0;
    let min_gap = (length - min_dashes * dash) / (min_dashes - 1.0);
    let max_gap = (length - max_dashes * dash) / (max_dashes - 1.0);
    if max_gap <= 0.0 || (min_gap - gap).abs() < (max_gap - gap).abs() {
        min_gap
    } else {
        max_gap
    }
}

fn draw_pattern(
    canvas: &Canvas,
    rect: SnappedRect,
    horizontal: bool,
    color: Color,
    width: i32,
    length: i32,
    style: BorderStyle,
) {
    let mut p1;
    let mut p2;
    if horizontal {
        let mid = (rect.y1 + width / 2) as f32;
        p1 = Point::new(rect.x1 as f32, mid);
        p2 = Point::new(rect.x2 as f32, mid);
    } else {
        let mid = (rect.x1 + width / 2) as f32;
        p1 = Point::new(mid, rect.y1 as f32);
        p2 = Point::new(mid, rect.y2 as f32);
    }
    let mut paint = color_paint(color, true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(width as f32);
    if let Some((intervals, round)) = pattern(length, width, style) {
        paint.set_path_effect(PathEffect::dash(&intervals, 0.0));
        if round {
            paint.set_stroke_cap(skia_safe::paint::Cap::Round);
        }
    }
    if style == BorderStyle::Dotted {
        if width <= 3 {
            endpoint_dots(canvas, &mut p1, &mut p2, horizontal, color, width, length);
        } else if horizontal {
            p1.x += width as f32 / 2.0;
            p2.x -= width as f32 / 2.0;
        } else {
            p1.y += width as f32 / 2.0;
            p2.y -= width as f32 / 2.0;
        }
    }
    if width % 2 != 0 {
        if horizontal {
            p1.y += 0.5;
            p2.y += 0.5;
        } else {
            p1.x += 0.5;
            p2.x += 0.5;
        }
    }
    canvas.draw_line(p1, p2, &paint);
}

fn endpoint_dots(
    canvas: &Canvas,
    p1: &mut Point,
    p2: &mut Point,
    horizontal: bool,
    color: Color,
    width: i32,
    length: i32,
) {
    let (mod4, mod6) = (length % 4, length % 6);
    let (mut start, mut end, mut start_growth, mut end_growth, mut start_offset) =
        (false, false, 0, 0, 0);
    if (width == 1 && length % 2 == 0) || (width == 3 && mod6 == 0) {
        start = true;
        start_growth = 1;
        start_offset = 1;
    }
    if (width == 2 && (mod4 == 0 || mod4 == 1)) || (width == 3 && (mod6 == 1 || mod6 == 2)) {
        start = true;
        start_offset = -1;
    }
    if (width == 2 && mod4 == 0) || (width == 3 && mod6 == 1) {
        end = true;
    }
    if (width == 2 && mod4 == 3) || (width == 3 && (mod6 == 4 || mod6 == 5)) {
        start = true;
        start_offset = 1;
    }
    if width == 3 && mod6 == 5 {
        end = true;
    } else if width == 3 && mod6 == 0 {
        end = true;
        end_growth = 1;
    }
    let paint = color_paint(color, false); // pinned fresh fill_flags defaults
    let half = (width / 2) as f32;
    if start {
        let rect = if horizontal {
            Rect::from_ltrb(
                p1.x,
                p1.y - half,
                p1.x + (width + start_growth) as f32,
                p1.y + (width - width / 2) as f32,
            )
        } else {
            Rect::from_ltrb(
                p1.x - half,
                p1.y,
                p1.x + (width - width / 2) as f32,
                p1.y + (width + start_growth) as f32,
            )
        };
        canvas.draw_rect(rect, &paint);
        if horizontal {
            p1.x += (2 * width + start_offset) as f32;
        } else {
            p1.y += (2 * width + start_offset) as f32;
        }
    }
    if end {
        let rect = if horizontal {
            Rect::from_ltrb(
                p2.x - (width + end_growth) as f32,
                p2.y - half,
                p2.x,
                p2.y + (width - width / 2) as f32,
            )
        } else {
            Rect::from_ltrb(
                p2.x - half,
                p2.y - (width + end_growth) as f32,
                p2.x + (width - width / 2) as f32,
                p2.y,
            )
        };
        canvas.draw_rect(rect, &paint);
        if horizontal {
            p2.x -= (width + end_growth + 1) as f32;
        } else {
            p2.y -= (width + end_growth + 1) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn css_3d_split_uses_absolute_coordinates_and_cpp_truncation() {
        assert_eq!(split_midpoint(10, 14), 12);
        assert_eq!(split_midpoint(10, 15), 13);
        assert_eq!(split_midpoint(-10, -6), -7);
        assert_eq!(split_midpoint(-10, -5), -7);
    }
    #[test]
    fn css_snap_preserves_short_nonzero_edges_and_signed_fraction() {
        assert_eq!(
            snap_size(LayoutUnit::from_raw(5), LayoutUnit::from_raw(0)),
            1
        );
        assert_eq!(
            snap_size(LayoutUnit::from_raw(4), LayoutUnit::from_raw(0)),
            0
        );
        assert_eq!(
            snap_size(LayoutUnit::from_raw(-5), LayoutUnit::from_raw(0)),
            -1
        );
        assert_eq!(
            snap_size(LayoutUnit::from_raw(64), LayoutUnit::from_raw(-32)),
            1
        );
    }
    #[test]
    fn css_shading_retains_alpha_and_low_contrast_lightens_black() {
        assert_eq!(
            css_dark(Color::WHITE),
            Color::from_rgba8(171, 171, 171, 255)
        );
        assert_eq!(
            css_light_side(Color::BLACK),
            Color::from_rgba8(84, 84, 84, 255)
        );
        let c = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.5,
        };
        assert_eq!(css_light_side(c).a, 0.5);
        assert_eq!(css_dark(c).a, 0.5);
        let c = Color::from_rgba8(150, 0, 0, 128);
        assert_eq!(css_light_side(c), c);
    }
    #[test]
    fn stroke_setup_keeps_short_lines_solid_and_uses_css_integer_thickness() {
        assert_eq!(pattern(12, 3, BorderStyle::Dashed), None);
        assert_eq!(
            pattern(15, 3, BorderStyle::Dashed),
            Some(([6.0, 3.0], false))
        );
        assert_eq!(pattern(7, 4, BorderStyle::Dotted), Some(([0.0, 8.0], true)));
        assert_eq!(pattern(6, 3, BorderStyle::Dotted), None);
    }
}
