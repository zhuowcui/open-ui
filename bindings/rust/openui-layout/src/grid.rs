//! CSS Grid layout for the deterministic WPT surface.
//!
//! The implementation keeps the authored track grammar until layout, expands
//! repeats against the used grid-container size, places explicit and automatic
//! items into one occupancy map, and then runs intrinsic/flexible track sizing.
//! It deliberately shares child layout, positioned layout, writing-mode
//! projection, and paint fragments with the other formatting contexts.

use std::collections::{HashMap, HashSet};

use openui_dom::{Document, ElementTag, NodeId, PseudoElementKind};
use openui_geometry::{
    BoxStrut, LayoutUnit, Length, LengthType, LogicalOffset, LogicalSize, PhysicalOffset,
    PhysicalRect, PhysicalSize, WritingModeConverter, INDEFINITE_SIZE,
};
use openui_style::{
    BoxSizing, ContentAlignment, ContentDistribution, ContentPosition, Display,
    GridAutoFlowDirection, GridLine, GridPlacement, GridRepetition, GridTrackBreadth,
    GridTrackComponent, GridTrackList, GridTrackSize, ItemAlignment, ItemPosition, Overflow,
    OverflowAlignment,
};

use crate::block::{block_layout, resolve_border, resolve_margins, resolve_padding};
use crate::constraint_space::ConstraintSpace;
use crate::fragment::Fragment;
use crate::intrinsic_sizing::{compute_child_intrinsic_contribution, IntrinsicSizes};
use crate::length_resolver::resolve_length;
use crate::out_of_flow::{layout_out_of_flow_children, OutOfFlowCandidate, StaticPositionEdge};

#[derive(Clone, Copy)]
struct Track {
    sizing: GridTrackSize,
    base: LayoutUnit,
    /// Fragmentainer remainder inserted after this track. It contributes to
    /// subsequent line positions and the grid's fragmented used size, but it
    /// is not part of an item whose grid area ends on this track.
    fragmentation_clearance: LayoutUnit,
    flexible: f32,
    stretchable: bool,
    intrinsic_minimum: bool,
    auto_fit: bool,
}

impl Track {
    fn new(sizing: GridTrackSize, percentage_base: LayoutUnit, auto_fit: bool) -> Self {
        let (base, flexible, stretchable) = initial_track_values(sizing, percentage_base);
        let intrinsic_minimum = match sizing {
            GridTrackSize::Breadth(GridTrackBreadth::Length(length)) => {
                fixed_track_length(length, percentage_base).is_none()
            }
            GridTrackSize::Breadth(_) => true,
            GridTrackSize::MinMax { min, .. } => !matches!(
                min,
                GridTrackBreadth::Length(length)
                    if fixed_track_length(length, percentage_base).is_some()
            ),
        };
        Self {
            sizing,
            base,
            fragmentation_clearance: LayoutUnit::zero(),
            flexible,
            stretchable,
            intrinsic_minimum,
            auto_fit,
        }
    }

    fn is_definite(self) -> bool {
        self.flexible == 0.0 && !self.stretchable && !self.intrinsic_minimum
    }
}

#[derive(Clone)]
struct AxisTracks {
    tracks: Vec<Track>,
    line_names: Vec<Vec<String>>,
}

#[derive(Clone)]
struct GridItem {
    node_id: NodeId,
    source_index: usize,
    column_start: usize,
    column_span: usize,
    row_start: usize,
    row_span: usize,
}

#[derive(Clone, Copy)]
struct AxisRange {
    start: usize,
    span: usize,
}

fn fixed_track_length(length: Length, percentage_base: LayoutUnit) -> Option<LayoutUnit> {
    match length.length_type() {
        LengthType::Fixed => Some(LayoutUnit::from_f32(length.value())),
        LengthType::Percent | LengthType::Calculated if !percentage_base.is_indefinite() => {
            Some(resolve_length(
                &length,
                percentage_base,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            ))
        }
        _ => None,
    }
}

fn initial_track_values(
    sizing: GridTrackSize,
    percentage_base: LayoutUnit,
) -> (LayoutUnit, f32, bool) {
    match sizing {
        GridTrackSize::Breadth(GridTrackBreadth::Length(length)) => {
            fixed_track_length(length, percentage_base)
                .map_or((LayoutUnit::zero(), 0.0, true), |value| (value, 0.0, false))
        }
        GridTrackSize::Breadth(GridTrackBreadth::Flex(value)) => {
            (LayoutUnit::zero(), value.max(0.0), false)
        }
        GridTrackSize::Breadth(GridTrackBreadth::FitContent(length)) => {
            fixed_track_length(length, percentage_base)
                .map_or((LayoutUnit::zero(), 0.0, false), |_| {
                    (LayoutUnit::zero(), 0.0, false)
                })
        }
        GridTrackSize::Breadth(GridTrackBreadth::Auto) => (LayoutUnit::zero(), 0.0, true),
        GridTrackSize::Breadth(_) => (LayoutUnit::zero(), 0.0, false),
        GridTrackSize::MinMax { min, max } => {
            let base = match min {
                GridTrackBreadth::Length(length) => {
                    fixed_track_length(length, percentage_base).unwrap_or(LayoutUnit::zero())
                }
                _ => LayoutUnit::zero(),
            };
            let flexible = match max {
                GridTrackBreadth::Flex(value) => value.max(0.0),
                _ => 0.0,
            };
            let stretchable = flexible == 0.0 && matches!(max, GridTrackBreadth::Auto);
            (base, flexible, stretchable)
        }
    }
}

fn component_fixed_minimum(
    component: &GridTrackComponent,
    percentage_base: LayoutUnit,
) -> LayoutUnit {
    match component {
        GridTrackComponent::LineNames(_) => LayoutUnit::zero(),
        GridTrackComponent::Track(size) => initial_track_values(*size, percentage_base).0,
        GridTrackComponent::Repeat { tracks, .. } => tracks
            .iter()
            .map(|component| component_fixed_minimum(component, percentage_base))
            .fold(LayoutUnit::zero(), |sum, value| sum + value),
    }
}

fn append_components(
    components: &[GridTrackComponent],
    percentage_base: LayoutUnit,
    gap: LayoutUnit,
    output: &mut AxisTracks,
    inherited_auto_fit: bool,
) {
    for component in components {
        match component {
            GridTrackComponent::LineNames(names) => {
                output
                    .line_names
                    .last_mut()
                    .unwrap()
                    .extend(names.iter().cloned());
            }
            GridTrackComponent::Track(size) => {
                output
                    .tracks
                    .push(Track::new(*size, percentage_base, inherited_auto_fit));
                output.line_names.push(Vec::new());
            }
            GridTrackComponent::Repeat { repetition, tracks } => {
                let track_count = tracks
                    .iter()
                    .filter(|component| matches!(component, GridTrackComponent::Track(_)))
                    .count()
                    .max(1);
                let repetitions = match repetition {
                    GridRepetition::Count(count) => (*count as usize).max(1),
                    GridRepetition::AutoFill | GridRepetition::AutoFit => {
                        let group = component_fixed_minimum(
                            &GridTrackComponent::Repeat {
                                repetition: GridRepetition::Count(1),
                                tracks: tracks.clone(),
                            },
                            percentage_base,
                        );
                        if percentage_base.is_indefinite() || group <= LayoutUnit::zero() {
                            1
                        } else {
                            let group_with_internal_gaps =
                                group + gap * (track_count.saturating_sub(1) as i32);
                            let stride = group_with_internal_gaps + gap;
                            (((percentage_base + gap).to_f32() / stride.to_f32()).floor() as usize)
                                .max(1)
                        }
                    }
                };
                let auto_fit = inherited_auto_fit || matches!(repetition, GridRepetition::AutoFit);
                for _ in 0..repetitions {
                    append_components(tracks, percentage_base, gap, output, auto_fit);
                }
            }
        }
    }
}

fn expand_tracks(
    list: &GridTrackList,
    percentage_base: LayoutUnit,
    gap: LayoutUnit,
    fallback: GridTrackSize,
) -> AxisTracks {
    let mut output = AxisTracks {
        tracks: Vec::new(),
        line_names: vec![Vec::new()],
    };
    match list {
        GridTrackList::Tracks(components) => {
            append_components(components, percentage_base, gap, &mut output, false)
        }
        GridTrackList::Subgrid(_) | GridTrackList::None => {}
    }
    if output.tracks.is_empty() {
        output
            .tracks
            .push(Track::new(fallback, percentage_base, false));
        output.line_names.push(Vec::new());
    }
    output
}

fn collect_grid_children(doc: &Document, parent: NodeId, output: &mut Vec<NodeId>) {
    if doc.node(parent).pseudo_kind == Some(PseudoElementKind::ScrollMarkerGroup) {
        fn collect_markers(doc: &Document, origin: NodeId, output: &mut Vec<NodeId>) {
            for child_id in doc.children(origin) {
                match doc.node(child_id).pseudo_kind {
                    Some(PseudoElementKind::ScrollMarker)
                    | Some(PseudoElementKind::ColumnScrollMarker) => output.push(child_id),
                    Some(PseudoElementKind::ScrollMarkerGroup) => {}
                    _ => collect_markers(doc, child_id, output),
                }
            }
        }

        let origin = doc.node(parent).pseudo_origin;
        if !origin.is_none() {
            collect_markers(doc, origin, output);
        }
        return;
    }

    for child_id in doc.children(parent) {
        let child = doc.node(child_id);
        if child.style.display == Display::None {
            continue;
        }
        if child.style.display == Display::Contents {
            collect_grid_children(doc, child_id, output);
            continue;
        }
        if child.tag == ElementTag::Text
            && child
                .text
                .as_deref()
                .is_none_or(|text| text.chars().all(char::is_whitespace))
        {
            continue;
        }
        output.push(child_id);
    }
}

fn named_line_index(names: &[Vec<String>], name: &str, occurrence: i32) -> Option<usize> {
    let matching: Vec<usize> = names
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.iter().any(|value| value == name).then_some(index))
        .collect();
    if occurrence < 0 {
        matching
            .len()
            .checked_sub((-occurrence) as usize)
            .and_then(|index| matching.get(index).copied())
    } else {
        matching.get(occurrence.max(1) as usize - 1).copied()
    }
}

fn line_index(line: &GridLine, names: &[Vec<String>], explicit_tracks: usize) -> Option<usize> {
    match line {
        GridLine::Line { index, name } => {
            if let Some(name) = name {
                if let Some(value) = named_line_index(names, name, *index) {
                    return Some(value);
                }
            }
            if *index > 0 {
                Some((*index as usize).saturating_sub(1))
            } else if *index < 0 {
                Some((explicit_tracks as i32 + 1 + *index).max(0) as usize)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn span_count(line: &GridLine) -> Option<usize> {
    match line {
        GridLine::Span { count, .. } => Some((*count as usize).max(1)),
        _ => None,
    }
}

fn area_axis_range(
    areas: &openui_style::GridTemplateAreas,
    name: &str,
    columns: bool,
) -> Option<AxisRange> {
    let mut first = usize::MAX;
    let mut last = 0usize;
    for (row, values) in areas.rows.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            if value.as_deref() == Some(name) {
                let index = if columns { column } else { row };
                first = first.min(index);
                last = last.max(index + 1);
            }
        }
    }
    (first != usize::MAX).then_some(AxisRange {
        start: first,
        span: last - first,
    })
}

fn placement_name(placement: &GridPlacement) -> Option<&str> {
    match (&placement.start, &placement.end) {
        (
            GridLine::Line {
                name: Some(name), ..
            },
            _,
        ) => Some(name),
        (
            _,
            GridLine::Line {
                name: Some(name), ..
            },
        ) => Some(name),
        _ => None,
    }
}

fn resolve_axis_range(
    placement: &GridPlacement,
    names: &[Vec<String>],
    explicit_tracks: usize,
    area: Option<AxisRange>,
) -> Option<AxisRange> {
    if let Some(area) = area {
        return Some(area);
    }
    let start = line_index(&placement.start, names, explicit_tracks);
    let end = line_index(&placement.end, names, explicit_tracks);
    let start_span = span_count(&placement.start);
    let end_span = span_count(&placement.end);
    match (start, end, start_span, end_span) {
        (Some(start), Some(end), _, _) if end > start => Some(AxisRange {
            start,
            span: end - start,
        }),
        (Some(start), _, _, Some(span)) => Some(AxisRange { start, span }),
        (_, Some(end), Some(span), _) => Some(AxisRange {
            start: end.saturating_sub(span),
            span,
        }),
        (Some(start), _, _, _) => Some(AxisRange { start, span: 1 }),
        (_, Some(end), _, _) => Some(AxisRange {
            start: end.saturating_sub(1),
            span: 1,
        }),
        _ => None,
    }
}

fn placement_area(
    doc: &Document,
    child_id: NodeId,
    style: &openui_style::ComputedStyle,
    columns: bool,
) -> Option<AxisRange> {
    let placement = if columns {
        &doc.node(child_id).style.grid_column
    } else {
        &doc.node(child_id).style.grid_row
    };
    placement_name(placement)
        .and_then(|name| area_axis_range(&style.grid_template_areas, name, columns))
}

fn cells_free(occupied: &HashSet<(usize, usize)>, column: AxisRange, row: AxisRange) -> bool {
    (row.start..row.start + row.span)
        .all(|r| (column.start..column.start + column.span).all(|c| !occupied.contains(&(r, c))))
}

fn occupy(occupied: &mut HashSet<(usize, usize)>, column: AxisRange, row: AxisRange) {
    for r in row.start..row.start + row.span {
        for c in column.start..column.start + column.span {
            occupied.insert((r, c));
        }
    }
}

fn place_items(
    doc: &Document,
    node_id: NodeId,
    style: &openui_style::ComputedStyle,
    columns: &AxisTracks,
    rows: &AxisTracks,
    children: &[NodeId],
) -> Vec<GridItem> {
    let mut occupied = HashSet::new();
    let mut result = Vec::new();
    let mut cursor_row = 0usize;
    let mut cursor_column = 0usize;
    let column_flow = style.grid_auto_flow.direction == GridAutoFlowDirection::Column;
    let dense = style.grid_auto_flow.dense;
    let explicit_columns = columns.tracks.len();
    let explicit_rows = rows.tracks.len();

    let mut ordered: Vec<(usize, NodeId)> = children.iter().copied().enumerate().collect();
    // Placement is phased along the auto-flow axis. Fully definite items
    // reserve their cells first. For row auto-flow, only items with a
    // definite row are placed in the second phase; a column-locked item with
    // an automatic row remains in the ordinary source-order scan. Column
    // auto-flow is the transpose of that rule (Grid Placement Algorithm,
    // steps 1, 2, and 4).
    ordered.sort_by_key(|(index, child)| {
        let child_style = &doc.node(*child).style;
        let definite_columns = resolve_axis_range(
            &child_style.grid_column,
            &columns.line_names,
            explicit_columns,
            placement_area(doc, *child, style, true),
        )
        .is_some();
        let definite_rows = resolve_axis_range(
            &child_style.grid_row,
            &rows.line_names,
            explicit_rows,
            placement_area(doc, *child, style, false),
        )
        .is_some();
        let phase = if definite_columns && definite_rows {
            0
        } else if (column_flow && definite_columns) || (!column_flow && definite_rows) {
            1
        } else {
            2
        };
        (phase, doc.node(*child).style.order, *index)
    });

    for (source_index, child_id) in ordered {
        if doc.node(child_id).style.position.is_absolutely_positioned() {
            continue;
        }
        let child_style = &doc.node(child_id).style;
        let column = resolve_axis_range(
            &child_style.grid_column,
            &columns.line_names,
            explicit_columns,
            placement_area(doc, child_id, style, true),
        );
        let row = resolve_axis_range(
            &child_style.grid_row,
            &rows.line_names,
            explicit_rows,
            placement_area(doc, child_id, style, false),
        );
        let column_span = column.map_or_else(
            || {
                span_count(&child_style.grid_column.start)
                    .or_else(|| span_count(&child_style.grid_column.end))
                    .unwrap_or(1)
            },
            |value| value.span,
        );
        let row_span = row.map_or_else(
            || {
                span_count(&child_style.grid_row.start)
                    .or_else(|| span_count(&child_style.grid_row.end))
                    .unwrap_or(1)
            },
            |value| value.span,
        );

        let (column, row) = match (column, row) {
            (Some(column), Some(row)) => (column, row),
            (Some(column), None) => {
                // In column auto-flow this is the dedicated one-axis
                // placement phase: each column-definite item searches from
                // the first row independently. In row auto-flow it remains
                // part of the cursor-driven sparse scan.
                let mut candidate = if column_flow || dense { 0 } else { cursor_row };
                loop {
                    let row = AxisRange {
                        start: candidate,
                        span: row_span,
                    };
                    if cells_free(&occupied, column, row) {
                        break (column, row);
                    }
                    candidate += 1;
                }
            }
            (None, Some(row)) => {
                // Transpose of the case above. Row-definite items in row
                // auto-flow do not inherit the previous item's column
                // cursor; two items in different explicit rows may both use
                // the first column.
                let mut candidate = if !column_flow || dense {
                    0
                } else {
                    cursor_column
                };
                loop {
                    let column = AxisRange {
                        start: candidate,
                        span: column_span,
                    };
                    if cells_free(&occupied, column, row) {
                        break (column, row);
                    }
                    candidate += 1;
                }
            }
            (None, None) => {
                if dense {
                    cursor_row = 0;
                    cursor_column = 0;
                }
                loop {
                    if column_flow {
                        if cursor_row + row_span > explicit_rows.max(1) {
                            cursor_row = 0;
                            cursor_column += 1;
                        }
                    } else if cursor_column + column_span > explicit_columns.max(1) {
                        cursor_column = 0;
                        cursor_row += 1;
                    }
                    let column = AxisRange {
                        start: cursor_column,
                        span: column_span,
                    };
                    let row = AxisRange {
                        start: cursor_row,
                        span: row_span,
                    };
                    if cells_free(&occupied, column, row) {
                        break (column, row);
                    }
                    if column_flow {
                        cursor_row += 1;
                        if cursor_row >= explicit_rows.max(1) {
                            cursor_row = 0;
                            cursor_column += 1;
                        }
                    } else {
                        cursor_column += 1;
                        if cursor_column + column_span > explicit_columns.max(1) {
                            cursor_column = 0;
                            cursor_row += 1;
                        }
                    }
                }
            }
        };
        occupy(&mut occupied, column, row);
        if !dense {
            cursor_column = column.start;
            cursor_row = row.start;
            if column_flow {
                cursor_row += row.span;
            } else {
                cursor_column += column.span;
            }
        }
        result.push(GridItem {
            node_id: child_id,
            source_index,
            column_start: column.start,
            column_span: column.span,
            row_start: row.start,
            row_span: row.span,
        });
    }
    result.sort_by_key(|item| (doc.node(item.node_id).style.order, item.source_index));
    let _ = node_id;
    result
}

fn repeated_auto_track(pattern: &[GridTrackSize], index: usize) -> GridTrackSize {
    pattern
        .get(index % pattern.len().max(1))
        .copied()
        .unwrap_or_else(GridTrackSize::auto)
}

fn ensure_tracks(
    tracks: &mut AxisTracks,
    required: usize,
    auto_pattern: &[GridTrackSize],
    percentage_base: LayoutUnit,
) {
    while tracks.tracks.len() < required.max(1) {
        let index = tracks.tracks.len();
        tracks.tracks.push(Track::new(
            repeated_auto_track(auto_pattern, index),
            percentage_base,
            false,
        ));
        tracks.line_names.push(Vec::new());
    }
}

fn track_limit(track: Track, percentage_base: LayoutUnit) -> Option<LayoutUnit> {
    match track.sizing {
        GridTrackSize::Breadth(GridTrackBreadth::FitContent(length)) => {
            fixed_track_length(length, percentage_base)
        }
        GridTrackSize::MinMax {
            max: GridTrackBreadth::Length(length),
            ..
        } => fixed_track_length(length, percentage_base),
        _ => None,
    }
}

fn grow_track(track: &mut Track, contribution: LayoutUnit, percentage_base: LayoutUnit) {
    if track.is_definite() {
        return;
    }
    // An intrinsic minimum floors the growth limit. In
    // `minmax(min-content, 10px)`, for example, the fixed maximum is raised
    // to the min-content contribution rather than capping the track at 10px.
    let value = if track.intrinsic_minimum {
        contribution
    } else {
        track_limit(*track, percentage_base)
            .map_or(contribution, |limit| contribution.min_of(limit))
    };
    track.base = track.base.max_of(value);
}

fn grow_spanning_tracks(
    tracks: &mut [Track],
    start: usize,
    span: usize,
    contribution: LayoutUnit,
    gap: LayoutUnit,
    percentage_base: LayoutUnit,
) {
    let end = (start + span).min(tracks.len());
    if start >= end {
        return;
    }
    let current = tracks[start..end]
        .iter()
        .map(|track| track.base)
        .fold(LayoutUnit::zero(), |sum, value| sum + value)
        + gap * (end.saturating_sub(start + 1) as i32);
    let deficit = contribution - current;
    if deficit <= LayoutUnit::zero() {
        return;
    }
    let growable: Vec<usize> = (start..end)
        .filter(|index| !tracks[*index].is_definite())
        .collect();
    if growable.is_empty() {
        return;
    }
    let share = deficit / growable.len() as i32;
    for index in growable {
        let target = tracks[index].base + share;
        grow_track(&mut tracks[index], target, percentage_base);
    }
}

fn axis_contribution(intrinsic: IntrinsicSizes, columns: bool, max_content: bool) -> LayoutUnit {
    match (columns, max_content) {
        (true, true) => intrinsic.max_content_inline_size,
        (true, false) => intrinsic.min_content_inline_size,
        (false, true) => intrinsic.max_content_block_size,
        (false, false) => intrinsic.min_content_block_size,
    }
}

/// Resolve the preferred-ratio contribution that survives a cyclic percentage
/// in the axis being intrinsically sized. CSS Grid treats the percentage as
/// auto for the intrinsic pass, but a definite opposite-axis size still
/// transfers through the preferred aspect ratio.
fn cyclic_ratio_contribution_with_opposite_basis(
    doc: &Document,
    node_id: NodeId,
    columns: bool,
    opposite_percentage_basis: Option<LayoutUnit>,
) -> Option<LayoutUnit> {
    let style = &doc.node(node_id).style;
    let ratio = style.aspect_ratio.as_ref()?;
    if ratio.ratio.0 <= 0.0 || ratio.ratio.1 <= 0.0 {
        return None;
    }
    let (cyclic, opposite) = if columns {
        (&style.width, &style.height)
    } else {
        (&style.height, &style.width)
    };
    if !(cyclic.is_percent() || cyclic.length_type() == LengthType::Calculated) {
        return None;
    }

    let opposite_used = if opposite.is_fixed() {
        LayoutUnit::from_f32(opposite.value())
    } else if (opposite.is_percent() || opposite.length_type() == LengthType::Calculated)
        && opposite_percentage_basis.is_some_and(|basis| !basis.is_indefinite())
    {
        let basis = opposite_percentage_basis.unwrap();
        resolve_length(opposite, basis, basis, basis)
    } else {
        return None;
    };

    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let margin = resolve_margins(style, LayoutUnit::zero());
    let opposite_border_padding = if columns {
        border.block_sum() + padding.block_sum()
    } else {
        border.inline_sum() + padding.inline_sum()
    };
    let result_border_padding = if columns {
        border.inline_sum() + padding.inline_sum()
    } else {
        border.block_sum() + padding.block_sum()
    };
    let opposite_border_box = if style.box_sizing == BoxSizing::BorderBox {
        opposite_used.max_of(opposite_border_padding)
    } else {
        opposite_used + opposite_border_padding
    };
    let ratio_uses_border_box = !ratio.auto_flag && style.box_sizing == BoxSizing::BorderBox;
    let transferred = if ratio_uses_border_box {
        if columns {
            LayoutUnit::from_f32(opposite_border_box.to_f32() * ratio.ratio.0 / ratio.ratio.1)
        } else {
            LayoutUnit::from_f32(opposite_border_box.to_f32() * ratio.ratio.1 / ratio.ratio.0)
        }
    } else {
        let opposite_content =
            (opposite_border_box - opposite_border_padding).clamp_negative_to_zero();
        let content = if columns {
            LayoutUnit::from_f32(opposite_content.to_f32() * ratio.ratio.0 / ratio.ratio.1)
        } else {
            LayoutUnit::from_f32(opposite_content.to_f32() * ratio.ratio.1 / ratio.ratio.0)
        };
        content + result_border_padding
    };
    Some(
        transferred
            + if columns {
                margin.inline_sum()
            } else {
                margin.block_sum()
            },
    )
}

fn cyclic_ratio_contribution(doc: &Document, node_id: NodeId, columns: bool) -> Option<LayoutUnit> {
    cyclic_ratio_contribution_with_opposite_basis(doc, node_id, columns, None)
}

/// Return the Grid item's minimum contribution in the physical axis used by
/// this deterministic surface. A definite minimum replaces the content-based
/// minimum contribution when the preferred size is automatic or cyclic. This
/// is what permits an auto track containing `min-width: 0` replaced content
/// to shrink as a flex item's Grid container shrinks, while its max-content
/// contribution continues to expose the resource's natural size.
fn grid_item_min_contribution(
    doc: &Document,
    node_id: NodeId,
    intrinsic: IntrinsicSizes,
    columns: bool,
) -> LayoutUnit {
    let style = &doc.node(node_id).style;
    if !columns
        && doc.node(node_id).replaced.is_some()
        && style.min_height.is_content_or_intrinsic()
        && (style.height.is_percent() || style.height.length_type() == LengthType::Calculated)
    {
        // A cyclic percentage is zero while resolving the replaced item's
        // used min-height, but the explicit intrinsic minimum still exposes
        // its natural max-content block contribution to Grid track sizing.
        // Keeping those two phases distinct lets the finished row resolve the
        // percentage without incorrectly clamping the control back to its
        // natural height.
        return intrinsic.max_content_block_size;
    }
    if let Some(transferred) = cyclic_ratio_contribution(doc, node_id, columns) {
        return transferred;
    }
    let (minimum, preferred, intrinsic_value, border_padding, margin) = if columns {
        let border = resolve_border(style);
        let padding = resolve_padding(style, LayoutUnit::zero());
        let margin = resolve_margins(style, LayoutUnit::zero());
        (
            &style.min_width,
            &style.width,
            intrinsic.min_content_inline_size,
            border.inline_sum() + padding.inline_sum(),
            margin.inline_sum(),
        )
    } else {
        let border = resolve_border(style);
        let padding = resolve_padding(style, LayoutUnit::zero());
        let margin = resolve_margins(style, LayoutUnit::zero());
        (
            &style.min_height,
            &style.height,
            intrinsic.min_content_block_size,
            border.block_sum() + padding.block_sum(),
            margin.block_sum(),
        )
    };
    if minimum.is_fixed()
        && (preferred.is_auto()
            || preferred.is_percent()
            || preferred.length_type() == LengthType::Calculated)
    {
        let raw = LayoutUnit::from_f32(minimum.value());
        let border_box = if style.box_sizing == BoxSizing::BorderBox {
            raw.max_of(border_padding)
        } else {
            raw + border_padding
        };
        border_box + margin
    } else if minimum.is_auto()
        && ((preferred.is_percent() || preferred.length_type() == LengthType::Calculated)
            || (preferred.is_auto()
                && if columns {
                    style.max_width.is_percent()
                        || style.max_width.length_type() == LengthType::Calculated
                } else {
                    style.max_height.is_percent()
                        || style.max_height.length_type() == LengthType::Calculated
                }))
    {
        // Percentages in an intrinsically-sized Grid track are cyclic.  They
        // contribute zero content size while the tracks are sized, then
        // resolve against the finished grid area during item layout.
        border_padding + margin
    } else {
        intrinsic_value
    }
}

fn size_intrinsic_tracks(
    doc: &Document,
    items: &[GridItem],
    tracks: &mut [Track],
    columns: bool,
    gap: LayoutUnit,
    percentage_base: LayoutUnit,
    cache: &mut HashMap<NodeId, IntrinsicSizes>,
) {
    let mut ordered: Vec<&GridItem> = items.iter().collect();
    ordered.sort_by_key(|item| {
        if columns {
            item.column_span
        } else {
            item.row_span
        }
    });
    for item in ordered {
        let (start, span) = if columns {
            (item.column_start, item.column_span)
        } else {
            (item.row_start, item.row_span)
        };
        if start >= tracks.len() {
            continue;
        }
        let intrinsic = *cache
            .entry(item.node_id)
            .or_insert_with(|| compute_child_intrinsic_contribution(doc, item.node_id));
        let min = grid_item_min_contribution(doc, item.node_id, intrinsic, columns);
        let max = cyclic_ratio_contribution(doc, item.node_id, columns)
            .unwrap_or_else(|| axis_contribution(intrinsic, columns, true));
        let slice = &tracks[start..(start + span).min(tracks.len())];
        let fixed_max_cap = slice
            .iter()
            .map(|track| match track.sizing {
                GridTrackSize::MinMax {
                    max: GridTrackBreadth::Length(length),
                    ..
                } => fixed_track_length(length, percentage_base),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(|limits| {
                limits
                    .into_iter()
                    .fold(LayoutUnit::zero(), |sum, value| sum + value)
                    + gap * slice.len().saturating_sub(1) as i32
            });
        let contribution = if let Some(cap) = fixed_max_cap {
            // Under a max-content constraint a fixed max track grows through
            // its growth limit, while an intrinsic minimum may still raise
            // that limit.  This is the maximize-tracks step for
            // `minmax(min-content|auto, <length>)`.
            min.max_of(max.min_of(cap))
        } else if slice.iter().any(|track| {
            matches!(
                track.sizing,
                GridTrackSize::Breadth(GridTrackBreadth::MaxContent)
                    | GridTrackSize::MinMax {
                        max: GridTrackBreadth::MaxContent,
                        ..
                    }
            ) || (percentage_base.is_indefinite()
                && matches!(
                    track.sizing,
                    GridTrackSize::Breadth(GridTrackBreadth::Auto)
                        | GridTrackSize::MinMax {
                            max: GridTrackBreadth::Auto,
                            ..
                        }
                ))
        }) {
            max
        } else {
            min
        };
        grow_spanning_tracks(tracks, start, span, contribution, gap, percentage_base);
    }
}

/// CSS Grid track sizing's maximize step for intrinsic-minimum tracks with a
/// fixed maximum. Grow toward each fixed growth limit, bounded by the
/// container when its size is definite.
fn maximize_fixed_max_tracks(
    tracks: &mut [Track],
    available: LayoutUnit,
    gap: LayoutUnit,
    percentage_base: LayoutUnit,
) {
    let mut free = if available.is_indefinite() {
        LayoutUnit::max()
    } else {
        (available - tracks_used_size(tracks, gap)).clamp_negative_to_zero()
    };
    for track in tracks {
        if free <= LayoutUnit::zero() {
            break;
        }
        if !matches!(
            track.sizing,
            GridTrackSize::MinMax {
                max: GridTrackBreadth::Length(_),
                ..
            }
        ) {
            continue;
        }
        let Some(limit) = track_limit(*track, percentage_base) else {
            continue;
        };
        let growth = (limit - track.base).clamp_negative_to_zero().min_of(free);
        track.base = track.base + growth;
        if !available.is_indefinite() {
            free = free - growth;
        }
    }
}

fn size_row_tracks_from_layout(
    doc: &Document,
    items: &[GridItem],
    rows: &mut [Track],
    columns: &[Track],
    column_gap: LayoutUnit,
    row_gap: LayoutUnit,
    container_inline: LayoutUnit,
    percentage_block_size: LayoutUnit,
    space: &ConstraintSpace,
    writing_direction: openui_geometry::WritingDirectionMode,
) {
    let column_offsets = track_offsets(columns, column_gap, LayoutUnit::zero(), LayoutUnit::zero());
    let mut ordered: Vec<&GridItem> = items.iter().collect();
    ordered.sort_by_key(|item| item.row_span);
    for item in ordered {
        if item.row_start >= rows.len() {
            continue;
        }
        let (_, area_inline) = axis_area(
            &column_offsets,
            item.column_start,
            item.column_span,
            column_gap,
        );
        let margins = resolve_margins(&doc.node(item.node_id).style, container_inline)
            .to_logical(writing_direction);
        let available_inline =
            (area_inline - margins.inline_start - margins.inline_end).clamp_negative_to_zero();
        let row_flow_start = rows[..item.row_start]
            .iter()
            .map(|track| track.base)
            .fold(LayoutUnit::zero(), |sum, value| sum + value)
            + row_gap * item.row_start as i32;
        let mut child = layout_grid_child(
            doc,
            item.node_id,
            space,
            available_inline,
            INDEFINITE_SIZE,
            area_inline,
            percentage_block_size,
            false,
            false,
        );
        if writing_direction.is_horizontal() && space.has_block_fragmentation() {
            crate::block::apply_forced_descendant_offsets_at(
                &mut child,
                doc,
                space.fragmentainer_block_size,
                row_flow_start,
            );
        }
        let child_block = if writing_direction.is_horizontal() {
            child.size.height
        } else {
            child.size.width
        };
        let fragmented_block = if percentage_block_size.is_indefinite()
            && style_axis_size(&doc.node(item.node_id).style, false).is_auto()
        {
            fragmented_inline_content_extent(&child, space.fragmentainer_block_size, row_flow_start)
                .unwrap_or(child_block)
        } else {
            child_block
        };
        let child_block_size = style_axis_size(&doc.node(item.node_id).style, false);
        let cyclic_percentage = child_block_size.is_percent()
            || child_block_size.length_type() == LengthType::Calculated;
        let contribution = if cyclic_percentage {
            if doc
                .node(item.node_id)
                .style
                .min_height
                .is_content_or_intrinsic()
            {
                let intrinsic = compute_child_intrinsic_contribution(doc, item.node_id);
                grid_item_min_contribution(doc, item.node_id, intrinsic, false)
            } else {
                cyclic_ratio_contribution_with_opposite_basis(
                    doc,
                    item.node_id,
                    false,
                    Some(area_inline),
                )
                .unwrap_or(margins.block_start + margins.block_end)
            }
        } else {
            fragmented_block + margins.block_start + margins.block_end
        };
        grow_spanning_tracks(
            rows,
            item.row_start,
            item.row_span,
            contribution,
            row_gap,
            percentage_block_size,
        );
    }
}

/// Return the block extent produced when an inline formatting context resumes
/// whole line boxes in successive fragmentainers. Grid row sizing consumes
/// this post-fragmentation extent for auto-sized grids; otherwise a 50px line
/// followed by a 100px monolithic line is incorrectly treated as 150px of
/// divisible geometry instead of two complete 100px fragmentainers.
fn fragmented_inline_content_extent(
    fragment: &Fragment,
    fragmentainer: LayoutUnit,
    flow_start: LayoutUnit,
) -> Option<LayoutUnit> {
    if fragmentainer <= LayoutUnit::zero() || fragmentainer.is_indefinite() {
        return None;
    }
    let mut lines: Vec<&Fragment> = fragment
        .children
        .iter()
        .filter(|child| child.node_id.is_none() && child.kind == crate::FragmentKind::Box)
        .collect();
    if !lines.is_empty() && lines.len() == fragment.children.len() {
        lines.sort_by_key(|line| line.offset.top.raw());
        let mut cursor = LayoutUnit::zero();
        for line in lines {
            cursor = cursor.max_of(line.offset.top);
            let global = flow_start + cursor;
            let remainder = LayoutUnit::from_raw(global.raw().rem_euclid(fragmentainer.raw()));
            let available = if remainder == LayoutUnit::zero() {
                fragmentainer
            } else {
                fragmentainer - remainder
            };
            if line.size.height <= fragmentainer
                && line.size.height > available
                && remainder > LayoutUnit::zero()
            {
                cursor = cursor + available;
            }
            cursor = cursor + line.size.height;
        }
        return Some(cursor);
    }

    let in_flow: Vec<&Fragment> = fragment
        .children
        .iter()
        .filter(|child| child.positioned_fragmentation.is_none())
        .collect();
    if in_flow.len() == 1 {
        let child = in_flow[0];
        return fragmented_inline_content_extent(
            child,
            fragmentainer,
            flow_start + child.offset.top,
        )
        .map(|extent| child.offset.top + extent);
    }
    None
}

/// Apply the same line-box resumption decisions used for intrinsic row
/// sizing to the final grid-item fragment. Alignment is computed from this
/// fragmented extent: a 120px auto-sized item whose second 60px line resumes
/// at the next 100px fragmentainer occupies 160px, rather than being aligned
/// as an unfragmented 120px box inside a 160px row.
fn fragment_inline_content(
    fragment: &mut Fragment,
    fragmentainer: LayoutUnit,
    flow_start: LayoutUnit,
) -> Option<LayoutUnit> {
    if fragmentainer <= LayoutUnit::zero() || fragmentainer.is_indefinite() {
        return None;
    }
    let mut line_indices: Vec<usize> = fragment
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| {
            (child.node_id.is_none() && child.kind == crate::FragmentKind::Box).then_some(index)
        })
        .collect();
    if !line_indices.is_empty() && line_indices.len() == fragment.children.len() {
        line_indices.sort_by_key(|index| fragment.children[*index].offset.top.raw());
        let mut cursor = LayoutUnit::zero();
        for index in line_indices {
            let line = &mut fragment.children[index];
            cursor = cursor.max_of(line.offset.top);
            let global = flow_start + cursor;
            let remainder = LayoutUnit::from_raw(global.raw().rem_euclid(fragmentainer.raw()));
            let available = if remainder == LayoutUnit::zero() {
                fragmentainer
            } else {
                fragmentainer - remainder
            };
            if line.size.height <= fragmentainer
                && line.size.height > available
                && remainder > LayoutUnit::zero()
            {
                cursor = cursor + available;
            }
            line.offset.top = cursor;
            cursor = cursor + line.size.height;
        }
        return Some(cursor);
    }

    let in_flow: Vec<usize> = fragment
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| child.positioned_fragmentation.is_none().then_some(index))
        .collect();
    if in_flow.len() == 1 {
        let index = in_flow[0];
        let child_offset = fragment.children[index].offset.top;
        return fragment_inline_content(
            &mut fragment.children[index],
            fragmentainer,
            flow_start + child_offset,
        )
        .map(|extent| child_offset + extent);
    }
    None
}

fn distribute_flexible_tracks(tracks: &mut [Track], available: LayoutUnit, gap: LayoutUnit) {
    if available.is_indefinite() {
        return;
    }
    let gaps = gap * (tracks.len().saturating_sub(1) as i32);
    let non_flex = tracks
        .iter()
        .filter(|track| track.flexible <= 0.0)
        .map(|track| track.base)
        .fold(LayoutUnit::zero(), |sum, value| sum + value);
    let flex_space = (available - gaps - non_flex).clamp_negative_to_zero();
    let flex_base_sum = tracks
        .iter()
        .filter(|track| track.flexible > 0.0)
        .map(|track| track.base)
        .fold(LayoutUnit::zero(), |sum, value| sum + value);
    let mut flex_sum: f32 = tracks
        .iter()
        .filter(|track| track.flexible > 0.0)
        .map(|track| track.flexible)
        .sum();
    if flex_sum <= 0.0 || flex_space <= flex_base_sum {
        return;
    }

    // Resolve the flex fraction against the whole space, freezing any track
    // whose intrinsic base is larger than its fraction. Adding an equal share
    // to each base makes equal `1fr` tracks unequal whenever their content
    // minima differ.
    let mut frozen = vec![false; tracks.len()];
    let mut remaining = flex_space;
    loop {
        let fraction = remaining.to_f32() / flex_sum;
        let newly_frozen: Vec<usize> = tracks
            .iter()
            .enumerate()
            .filter_map(|(index, track)| {
                (track.flexible > 0.0
                    && !frozen[index]
                    && track.base.to_f32() > fraction * track.flexible)
                    .then_some(index)
            })
            .collect();
        if newly_frozen.is_empty() {
            for (index, track) in tracks.iter_mut().enumerate() {
                if track.flexible > 0.0 && !frozen[index] {
                    track.base = LayoutUnit::from_f32(fraction * track.flexible).max_of(track.base);
                }
            }
            break;
        }
        for index in newly_frozen {
            frozen[index] = true;
            remaining = (remaining - tracks[index].base).clamp_negative_to_zero();
            flex_sum -= tracks[index].flexible;
        }
        if flex_sum <= 0.0 {
            break;
        }
    }
}

fn shrink_fixed_minmax_tracks(tracks: &mut [Track], available: LayoutUnit, gap: LayoutUnit) {
    if available.is_indefinite() {
        return;
    }
    let used = tracks_used_size(tracks, gap);
    let mut excess = used - available;
    if excess <= LayoutUnit::zero() {
        return;
    }
    let candidates: Vec<usize> = tracks
        .iter()
        .enumerate()
        .filter_map(|(index, track)| {
            matches!(
                track.sizing,
                GridTrackSize::MinMax {
                    min: GridTrackBreadth::Length(_),
                    max: GridTrackBreadth::Auto | GridTrackBreadth::Flex(_),
                }
            )
            .then_some(index)
        })
        .collect();
    for (position, index) in candidates.iter().copied().enumerate() {
        let floor = match tracks[index].sizing {
            GridTrackSize::MinMax {
                min: GridTrackBreadth::Length(length),
                ..
            } => fixed_track_length(length, available).unwrap_or(LayoutUnit::zero()),
            _ => LayoutUnit::zero(),
        };
        let reducible = (tracks[index].base - floor).clamp_negative_to_zero();
        let remaining = candidates.len().saturating_sub(position).max(1) as i32;
        let reduction = reducible.min_of(excess / remaining);
        tracks[index].base = tracks[index].base - reduction;
        excess = excess - reduction;
    }
}

fn collapse_empty_auto_fit_tracks(tracks: &mut [Track], items: &[GridItem], columns: bool) {
    for (index, track) in tracks.iter_mut().enumerate() {
        if !track.auto_fit {
            continue;
        }
        let occupied = items.iter().any(|item| {
            let (start, span) = if columns {
                (item.column_start, item.column_span)
            } else {
                (item.row_start, item.row_span)
            };
            index >= start && index < start + span
        });
        if !occupied {
            track.base = LayoutUnit::zero();
        }
    }
}

/// Insert the fragmentainer clearance required by class-A break constraints
/// between Grid rows. Linked `avoid` rows move as one unit when that unit fits
/// a fresh fragmentainer; a forced break advances to the next fragmentainer.
/// The clearance becomes part of the preceding row's fragmented used extent,
/// which keeps subsequent row offsets and the Grid container's continuation
/// size in the same linear source coordinate space.
fn apply_row_fragmentation_breaks(
    doc: &Document,
    items: &[GridItem],
    rows: &mut [Track],
    row_gap: LayoutUnit,
    fragmentainer: LayoutUnit,
) {
    if rows.is_empty() || fragmentainer <= LayoutUnit::zero() || fragmentainer.is_indefinite() {
        return;
    }
    let mut forced_before = vec![false; rows.len() + 1];
    let mut avoid_boundary = vec![false; rows.len() + 1];
    let mut avoid_row = vec![false; rows.len()];
    let mut unfragmented_row_size: Vec<LayoutUnit> = rows
        .iter()
        .map(|track| {
            if track.is_definite() {
                track.base
            } else {
                LayoutUnit::zero()
            }
        })
        .collect();
    for item in items {
        let start = item.row_start.min(rows.len());
        let end = (item.row_start + item.row_span).min(rows.len());
        let style = &doc.node(item.node_id).style;
        let propagated_before = crate::block::propagated_break_before(doc, item.node_id);
        let propagated_after = crate::block::propagated_break_after(doc, item.node_id);
        let break_before = if propagated_before.is_forced() {
            propagated_before
        } else {
            style.break_before
        };
        let break_after = if propagated_after.is_forced() {
            propagated_after
        } else {
            style.break_after
        };
        if break_before.is_forced() {
            forced_before[start] = true;
        } else if break_before.is_avoid() {
            avoid_boundary[start] = true;
        }
        if break_after.is_forced() {
            forced_before[end] = true;
        } else if break_after.is_avoid() {
            avoid_boundary[end] = true;
        }
        if style.break_inside.is_avoid() {
            for row in avoid_row.iter_mut().take(end).skip(start) {
                *row = true;
            }
            for boundary in avoid_boundary.iter_mut().take(end).skip(start + 1) {
                *boundary = true;
            }
        }
        if end == start + 1 {
            let intrinsic = compute_child_intrinsic_contribution(doc, item.node_id);
            unfragmented_row_size[start] =
                unfragmented_row_size[start].max_of(intrinsic.max_content_block_size);
        }
    }

    let mut cursor = LayoutUnit::zero();
    let mut row = 0usize;
    while row < rows.len() {
        let group_start = row;
        let mut group_end = row + 1;
        while group_end < rows.len() && avoid_boundary[group_end] && !forced_before[group_end] {
            group_end += 1;
        }
        let avoid_group = group_end > group_start + 1
            || avoid_row[group_start..group_end].iter().any(|value| *value);
        let natural_group_size = unfragmented_row_size[group_start..group_end]
            .iter()
            .copied()
            .fold(LayoutUnit::zero(), |sum, value| sum + value)
            + row_gap * group_end.saturating_sub(group_start + 1) as i32;
        if avoid_group && natural_group_size <= fragmentainer {
            for index in group_start..group_end {
                if !rows[index].is_definite() && unfragmented_row_size[index] > LayoutUnit::zero() {
                    rows[index].base = rows[index].base.min_of(unfragmented_row_size[index]);
                }
            }
        }
        let group_size = rows[group_start..group_end]
            .iter()
            .map(|track| track.base)
            .fold(LayoutUnit::zero(), |sum, value| sum + value)
            + row_gap * group_end.saturating_sub(group_start + 1) as i32;
        let remainder = LayoutUnit::from_raw(cursor.raw().rem_euclid(fragmentainer.raw()));
        let available = if remainder == LayoutUnit::zero() {
            fragmentainer
        } else {
            fragmentainer - remainder
        };
        let needs_clearance = remainder > LayoutUnit::zero()
            && (forced_before[group_start]
                || (avoid_group && group_size <= fragmentainer && group_size > available));
        if needs_clearance {
            let clearance = available;
            if group_start > 0 {
                rows[group_start - 1].base = rows[group_start - 1].base + clearance;
                rows[group_start - 1].fragmentation_clearance =
                    rows[group_start - 1].fragmentation_clearance + clearance;
            }
            cursor = cursor + clearance;
        }
        cursor = cursor + group_size;
        if group_end < rows.len() {
            cursor = cursor + row_gap;
        }
        row = group_end;
    }
}

fn resolve_gap(value: &Option<Length>, percentage_base: LayoutUnit) -> LayoutUnit {
    value.as_ref().map_or(LayoutUnit::zero(), |length| {
        resolve_length(
            length,
            percentage_base,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
        .clamp_negative_to_zero()
    })
}

fn tracks_used_size(tracks: &[Track], gap: LayoutUnit) -> LayoutUnit {
    tracks
        .iter()
        .map(|track| track.base)
        .fold(LayoutUnit::zero(), |sum, value| sum + value)
        + gap * (tracks.len().saturating_sub(1) as i32)
}

fn content_alignment(
    alignment: ContentAlignment,
    available: LayoutUnit,
    tracks: &mut [Track],
    gap: LayoutUnit,
) -> (LayoutUnit, LayoutUnit) {
    let raw_free = available - tracks_used_size(tracks, gap);
    let free = raw_free.clamp_negative_to_zero();
    let stretch = alignment.position == ContentPosition::Normal
        || alignment.distribution == ContentDistribution::Stretch;
    if stretch && free > LayoutUnit::zero() {
        let stretchable: Vec<usize> = tracks
            .iter()
            .enumerate()
            .filter_map(|(index, track)| track.stretchable.then_some(index))
            .collect();
        if !stretchable.is_empty() {
            let share = free / stretchable.len() as i32;
            for index in stretchable {
                tracks[index].base = tracks[index].base + share;
            }
            return (LayoutUnit::zero(), LayoutUnit::zero());
        }
    }
    match alignment.distribution {
        ContentDistribution::SpaceBetween if tracks.len() > 1 => {
            (LayoutUnit::zero(), free / (tracks.len() as i32 - 1))
        }
        ContentDistribution::SpaceAround if !tracks.is_empty() => {
            let between = free / tracks.len() as i32;
            (between / 2, between)
        }
        ContentDistribution::SpaceEvenly if !tracks.is_empty() => {
            let between = free / (tracks.len() as i32 + 1);
            (between, between)
        }
        _ => {
            let start = match alignment.position {
                ContentPosition::Center => raw_free / 2,
                ContentPosition::End | ContentPosition::FlexEnd | ContentPosition::Right => {
                    raw_free
                }
                _ => LayoutUnit::zero(),
            };
            (start, LayoutUnit::zero())
        }
    }
}

fn track_offsets(
    tracks: &[Track],
    gap: LayoutUnit,
    initial: LayoutUnit,
    extra_between: LayoutUnit,
) -> Vec<LayoutUnit> {
    let mut output = Vec::with_capacity(tracks.len() + 1);
    let mut cursor = initial;
    output.push(cursor);
    for (index, track) in tracks.iter().enumerate() {
        cursor = cursor + track.base;
        output.push(cursor);
        if index + 1 < tracks.len() {
            cursor = cursor + gap + extra_between;
        }
    }
    output
}

fn resolved_self_alignment(
    item: ItemAlignment,
    parent: ItemAlignment,
    replaced: bool,
) -> ItemPosition {
    let mut position = if item.position == ItemPosition::Auto {
        parent.position
    } else {
        item.position
    };
    if position == ItemPosition::Normal {
        position = if replaced {
            ItemPosition::Start
        } else {
            ItemPosition::Stretch
        };
    }
    position
}

fn item_alignment_offset(
    position: ItemPosition,
    free: LayoutUnit,
    overflow: OverflowAlignment,
) -> LayoutUnit {
    // The default overflow alignment for Grid self-alignment is safe.  An
    // oversized item therefore falls back to start rather than being
    // centered equally outside both edges of its grid area.
    let free = if overflow != OverflowAlignment::Unsafe {
        free.clamp_negative_to_zero()
    } else {
        free
    };
    match position {
        ItemPosition::Center => free / 2,
        ItemPosition::End
        | ItemPosition::SelfEnd
        | ItemPosition::FlexEnd
        | ItemPosition::Right
        | ItemPosition::LastBaseline => free,
        _ => LayoutUnit::zero(),
    }
}

fn axis_area(
    offsets: &[LayoutUnit],
    start: usize,
    span: usize,
    between: LayoutUnit,
) -> (LayoutUnit, LayoutUnit) {
    let start = start.min(offsets.len().saturating_sub(1));
    let end = (start + span).min(offsets.len().saturating_sub(1));
    let track_start = offsets[start]
        + if start == 0 {
            LayoutUnit::zero()
        } else {
            between
        };
    (track_start, offsets[end] - track_start)
}

fn row_area(
    offsets: &[LayoutUnit],
    tracks: &[Track],
    start: usize,
    span: usize,
    between: LayoutUnit,
) -> (LayoutUnit, LayoutUnit) {
    let (track_start, area) = axis_area(offsets, start, span, between);
    let start = start.min(tracks.len());
    let end = (start + span).min(tracks.len());
    let trailing_clearance = end
        .checked_sub(1)
        .filter(|end| *end >= start)
        .map_or(LayoutUnit::zero(), |end| {
            tracks[end].fragmentation_clearance
        });
    (
        track_start,
        (area - trailing_clearance).clamp_negative_to_zero(),
    )
}

fn style_axis_size(style: &openui_style::ComputedStyle, inline_axis: bool) -> &Length {
    let logical = crate::ResolvedLogicalBox::from_style(style);
    if inline_axis {
        logical.sizes.inline_size
    } else {
        logical.sizes.block_size
    }
}

fn layout_grid_child(
    doc: &Document,
    child_id: NodeId,
    parent_space: &ConstraintSpace,
    available_inline: LayoutUnit,
    available_block: LayoutUnit,
    percentage_inline: LayoutUnit,
    percentage_block: LayoutUnit,
    stretch_inline: bool,
    stretch_block: bool,
) -> Fragment {
    let child_style = &doc.node(child_id).style;
    let mut available_inline = if doc.node(child_id).tag == ElementTag::Text && !stretch_inline {
        crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, child_id)
            .max
            .min_of(available_inline)
    } else {
        available_inline
    };
    let child_logical = crate::ResolvedLogicalBox::from_style(child_style);
    let auto_inline_fit = !stretch_inline && child_logical.sizes.inline_size.is_auto();
    if auto_inline_fit && doc.node(child_id).tag != ElementTag::Text {
        // An auto-sized Grid item which is not stretched is fit-content in
        // the inline axis.  When its block size is definite, that measure
        // participates in intrinsic sizing first so percentage-height and
        // aspect-ratio descendants can transfer a width back to the item.
        let intrinsic = if !available_block.is_indefinite()
            && (stretch_block || !child_logical.sizes.block_size.is_auto())
        {
            crate::intrinsic_sizing::compute_intrinsic_inline_sizes_with_block_size(
                doc,
                child_id,
                available_block,
                percentage_inline,
            )
        } else {
            crate::intrinsic_sizing::compute_intrinsic_inline_sizes(doc, child_id)
        };
        if child_style.aspect_ratio.is_none() || !child_logical.sizes.block_size.is_auto() {
            available_inline = crate::intrinsic_sizing::shrink_to_fit_inline_size(
                intrinsic.min,
                intrinsic.max,
                available_inline,
            );
        }
        if stretch_block || !child_logical.sizes.block_size.is_auto() {
            if let Some(ratio) = &child_style.aspect_ratio {
                if ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0 {
                    // A stretched opposite axis is a definite size for the
                    // preferred-ratio transfer.  This transfer happens
                    // before fit-content clamping and may overflow a narrow
                    // grid area (for example a 1:1 item in a 50x100 area).
                    available_inline = if child_logical.writing_direction.is_horizontal() {
                        LayoutUnit::from_f32(
                            available_block.to_f32() * ratio.ratio.0 / ratio.ratio.1,
                        )
                    } else {
                        LayoutUnit::from_f32(
                            available_block.to_f32() * ratio.ratio.1 / ratio.ratio.0,
                        )
                    };
                }
            }
        }
    }
    let child_direction = child_style
        .direction
        .writing_direction(child_style.writing_mode);
    let orthogonal =
        child_direction.is_horizontal() != parent_space.writing_direction.is_horizontal();
    let mut child_space = ConstraintSpace::for_block_child_from_parent(
        parent_space,
        available_inline,
        available_block,
        percentage_inline,
        percentage_block,
        true,
        child_direction,
    );
    if stretch_inline {
        if orthogonal {
            child_space.stretch_block_size = true;
            child_space.is_fixed_block_size = true;
        } else {
            child_space.stretch_inline_size = true;
            child_space.is_fixed_inline_size = true;
        }
    } else if auto_inline_fit {
        if orthogonal {
            child_space.is_fixed_block_size = true;
        } else {
            child_space.is_fixed_inline_size = true;
        }
    }
    if stretch_block {
        if orthogonal {
            child_space.stretch_inline_size = true;
            child_space.is_fixed_inline_size = true;
        } else {
            child_space.stretch_block_size = true;
            child_space.is_fixed_block_size = true;
        }
    }
    if doc.node(child_id).tag == ElementTag::Text {
        // Text directly exposed to a Grid container generates an anonymous
        // grid item whose contents establish an inline formatting context.
        // Laying out the Text node as a block would retain its advance for
        // track sizing but produce an empty, zero-height paint fragment.
        let mut fragment = crate::inline::algorithm::inline_layout_for_children(
            doc,
            child_id,
            &[child_id],
            &child_space,
        );
        if !child_space.writing_direction.is_horizontal() {
            let logical_size = LogicalSize::new(fragment.size.width, fragment.size.height);
            let physical_size = WritingModeConverter::new(
                child_space.writing_direction,
                PhysicalSize::new(LayoutUnit::zero(), LayoutUnit::zero()),
            )
            .to_physical_size(logical_size);
            for child in &mut fragment.children {
                crate::block::project_logical_child_to_physical(
                    child,
                    child_space.writing_direction,
                    physical_size,
                );
            }
            fragment.size = physical_size;
        }
        fragment
    } else {
        block_layout(doc, child_id, &child_space)
    }
}

fn clamp_container_axis(
    size: LayoutUnit,
    min: &Length,
    max: &Length,
    percentage_base: LayoutUnit,
    border_padding: LayoutUnit,
    box_sizing: BoxSizing,
) -> LayoutUnit {
    let resolve_bound = |length: &Length, none: LayoutUnit| {
        let raw = resolve_length(length, percentage_base, LayoutUnit::zero(), none);
        if box_sizing == BoxSizing::BorderBox {
            raw
        } else {
            raw + border_padding
        }
    };
    let min = if min.is_auto() || min.is_content_or_intrinsic() {
        LayoutUnit::zero()
    } else {
        resolve_bound(min, LayoutUnit::zero())
    };
    let max = if max.is_none() || max.is_content_or_intrinsic() {
        LayoutUnit::max()
    } else {
        resolve_bound(max, LayoutUnit::max())
    }
    // CSS Sizing resolves an over-constrained min/max pair by letting the
    // minimum win. Applying the maximum after the minimum without this
    // normalization incorrectly shrinks `min-width: 200px; max-width: 150px`
    // Grid containers back to 150px.
    .max_of(min);
    size.max_of(min).min_of(max)
}

fn resolve_container_inline_size(
    doc: &Document,
    node_id: NodeId,
    style: &openui_style::ComputedStyle,
    space: &ConstraintSpace,
    border_padding: LayoutUnit,
) -> LayoutUnit {
    let logical = crate::ResolvedLogicalBox::from_style(style);
    let preferred = logical.sizes.inline_size;
    let mut size = if space.is_fixed_inline_size || space.stretch_inline_size {
        space.available_inline_size
    } else if preferred.is_auto()
        && !logical.sizes.block_size.is_auto()
        && !logical.sizes.block_size.is_content_or_intrinsic()
        && style
            .aspect_ratio
            .as_ref()
            .is_some_and(|ratio| ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0)
    {
        let raw_block = resolve_length(
            logical.sizes.block_size,
            space.percentage_resolution_block_size,
            INDEFINITE_SIZE,
            INDEFINITE_SIZE,
        );
        let content_block = if style.box_sizing == BoxSizing::BorderBox {
            (raw_block
                - resolve_border(style)
                    .to_logical(logical.writing_direction)
                    .block_sum()
                - resolve_padding(style, space.percentage_resolution_inline_size)
                    .to_logical(logical.writing_direction)
                    .block_sum())
            .clamp_negative_to_zero()
        } else {
            raw_block
        };
        let ratio = style.aspect_ratio.as_ref().unwrap().ratio;
        let content_inline = if logical.writing_direction.is_horizontal() {
            LayoutUnit::from_f32(content_block.to_f32() * ratio.0 / ratio.1)
        } else {
            LayoutUnit::from_f32(content_block.to_f32() * ratio.1 / ratio.0)
        };
        content_inline + border_padding
    } else if preferred.is_content_or_intrinsic()
        || (style.display == Display::InlineGrid && preferred.is_auto())
    {
        let (min, max) = if let Some(fallback) = crate::containment::logical_inline_fallback(style)
        {
            let contained = fallback + border_padding;
            (contained, contained)
        } else {
            let intrinsic = compute_grid_intrinsic_sizes(doc, node_id);
            (
                intrinsic.min_content_inline_size,
                intrinsic.max_content_inline_size,
            )
        };
        match preferred.length_type() {
            LengthType::MinContent => min,
            LengthType::MaxContent => max,
            _ => max.min_of(space.available_inline_size.max_of(min)),
        }
    } else if preferred.is_auto() {
        if space.available_inline_size.is_indefinite() {
            compute_grid_intrinsic_sizes(doc, node_id).max_content_inline_size
        } else {
            space.available_inline_size
        }
    } else {
        let raw = resolve_length(
            preferred,
            space.percentage_resolution_inline_size,
            space.available_inline_size,
            space.available_inline_size,
        );
        if style.box_sizing == BoxSizing::BorderBox {
            raw
        } else {
            raw + border_padding
        }
    };
    size = clamp_container_axis(
        size,
        logical.sizes.min_inline_size,
        logical.sizes.max_inline_size,
        space.percentage_resolution_inline_size,
        border_padding,
        style.box_sizing,
    );
    size.max_of(border_padding)
}

fn resolve_container_block_size(
    style: &openui_style::ComputedStyle,
    space: &ConstraintSpace,
    intrinsic: LayoutUnit,
    border_padding: LayoutUnit,
    inline_size: LayoutUnit,
) -> LayoutUnit {
    let logical = crate::ResolvedLogicalBox::from_style(style);
    let preferred = logical.sizes.block_size;
    let mut size = if space.is_fixed_block_size || space.stretch_block_size {
        space.available_block_size
    } else if preferred.is_auto() || preferred.is_content_or_intrinsic() {
        if let Some(ratio) = &style.aspect_ratio {
            if ratio.ratio.0 > 0.0 && ratio.ratio.1 > 0.0 {
                intrinsic.max_of(LayoutUnit::from_f32(
                    inline_size.to_f32() * ratio.ratio.1 / ratio.ratio.0,
                ))
            } else {
                intrinsic
            }
        } else {
            intrinsic
        }
    } else {
        let raw = resolve_length(
            preferred,
            space.percentage_resolution_block_size,
            intrinsic,
            intrinsic,
        );
        if style.box_sizing == BoxSizing::BorderBox {
            raw
        } else {
            raw + border_padding
        }
    };
    // In the block axis, min-content and max-content both resolve to the
    // Grid container's max-content block size (the sum of its row tracks).
    // Apply those intrinsic constraints before the ordinary fixed-length
    // clamp below.
    if logical.sizes.min_block_size.is_content_or_intrinsic() {
        size = size.max_of(intrinsic);
    }
    if logical.sizes.max_block_size.is_content_or_intrinsic() {
        size = size.min_of(intrinsic);
    }
    size = clamp_container_axis(
        size,
        logical.sizes.min_block_size,
        logical.sizes.max_block_size,
        space.percentage_resolution_block_size,
        border_padding,
        style.box_sizing,
    );
    size.max_of(border_padding)
}

fn overflow_from_children(fragment: &mut Fragment, style: &openui_style::ComputedStyle) {
    let border_box = PhysicalRect::new(PhysicalOffset::zero(), fragment.size);
    let mut overflow = border_box;
    for child in &fragment.children {
        overflow = overflow.unite(&PhysicalRect::new(child.offset, child.size));
        if !child.has_overflow_clip {
            if let Some(child_overflow) = child.overflow_rect {
                overflow = overflow.unite(&PhysicalRect::new(
                    PhysicalOffset::new(
                        child.offset.left + child_overflow.offset.left,
                        child.offset.top + child_overflow.offset.top,
                    ),
                    child_overflow.size,
                ));
            }
        }
    }
    if overflow != border_box {
        fragment.overflow_rect = Some(overflow);
    }
    fragment.has_overflow_clip =
        style.overflow_x != Overflow::Visible || style.overflow_y != Overflow::Visible;
}

/// Lay out one Grid container.
pub fn grid_layout(doc: &Document, node_id: NodeId, space: &ConstraintSpace) -> Fragment {
    let style = &doc.node(node_id).style;
    let logical_box = crate::ResolvedLogicalBox::from_style(style);
    let writing_direction = logical_box.writing_direction;
    let border = resolve_border(style);
    let padding = resolve_padding(style, space.percentage_resolution_inline_size);
    let logical_border = border.to_logical(writing_direction);
    let logical_padding = padding.to_logical(writing_direction);
    let border_padding_inline = logical_border.inline_sum() + logical_padding.inline_sum();
    let border_padding_block = logical_border.block_sum() + logical_padding.block_sum();
    let container_inline =
        resolve_container_inline_size(doc, node_id, style, space, border_padding_inline);
    let content_inline = (container_inline - border_padding_inline).clamp_negative_to_zero();
    let column_gap = resolve_gap(&style.column_gap, content_inline);
    let preliminary_block = if logical_box.sizes.block_size.is_auto() {
        // Size containment substitutes the contain-intrinsic block size for
        // the contents before track sizing. Treat that substituted content
        // box as definite so fr and auto-repeat rows resolve against it.
        crate::containment::logical_block_fallback(style).unwrap_or(INDEFINITE_SIZE)
    } else {
        let raw = resolve_length(
            logical_box.sizes.block_size,
            space.percentage_resolution_block_size,
            INDEFINITE_SIZE,
            INDEFINITE_SIZE,
        );
        if style.box_sizing == BoxSizing::BorderBox {
            (raw - border_padding_block).clamp_negative_to_zero()
        } else {
            raw
        }
    };
    let row_gap = resolve_gap(
        &style.row_gap,
        if preliminary_block.is_indefinite() {
            LayoutUnit::zero()
        } else {
            preliminary_block
        },
    );
    let mut columns = expand_tracks(
        &style.grid_template_columns,
        content_inline,
        column_gap,
        repeated_auto_track(&style.grid_auto_columns, 0),
    );
    let mut rows = expand_tracks(
        &style.grid_template_rows,
        preliminary_block,
        row_gap,
        repeated_auto_track(&style.grid_auto_rows, 0),
    );
    let mut children = Vec::new();
    collect_grid_children(doc, node_id, &mut children);
    let items = place_items(doc, node_id, style, &columns, &rows, &children);
    let required_columns = items
        .iter()
        .map(|item| item.column_start + item.column_span)
        .max()
        .unwrap_or(columns.tracks.len());
    let required_rows = items
        .iter()
        .map(|item| item.row_start + item.row_span)
        .max()
        .unwrap_or(rows.tracks.len());
    ensure_tracks(
        &mut columns,
        required_columns,
        &style.grid_auto_columns,
        content_inline,
    );
    ensure_tracks(
        &mut rows,
        required_rows,
        &style.grid_auto_rows,
        preliminary_block,
    );
    let mut intrinsic_cache = HashMap::new();
    size_intrinsic_tracks(
        doc,
        &items,
        &mut columns.tracks,
        true,
        column_gap,
        content_inline,
        &mut intrinsic_cache,
    );
    maximize_fixed_max_tracks(
        &mut columns.tracks,
        content_inline,
        column_gap,
        content_inline,
    );
    distribute_flexible_tracks(&mut columns.tracks, content_inline, column_gap);
    collapse_empty_auto_fit_tracks(&mut columns.tracks, &items, true);
    // Column-axis content alignment is part of the inline track result seen
    // by row sizing. In particular, `justify-content: normal` stretches an
    // auto column before a wrapping item's block contribution is measured.
    let (column_initial, column_between) = content_alignment(
        style.justify_content,
        content_inline,
        &mut columns.tracks,
        column_gap,
    );

    // Row contributions depend on the sized grid-area inline measure. The
    // intrinsic block contribution remains the stable first pass; child
    // layout below supplies the final used block size inside each area.
    size_row_tracks_from_layout(
        doc,
        &items,
        &mut rows.tracks,
        &columns.tracks,
        column_gap,
        row_gap,
        content_inline,
        preliminary_block,
        space,
        writing_direction,
    );
    maximize_fixed_max_tracks(
        &mut rows.tracks,
        preliminary_block,
        row_gap,
        preliminary_block,
    );
    if space.has_block_fragmentation() && writing_direction.is_horizontal() {
        apply_row_fragmentation_breaks(
            doc,
            &items,
            &mut rows.tracks,
            row_gap,
            space.fragmentainer_block_size,
        );
    }
    shrink_fixed_minmax_tracks(&mut rows.tracks, preliminary_block, row_gap);
    distribute_flexible_tracks(&mut rows.tracks, preliminary_block, row_gap);
    collapse_empty_auto_fit_tracks(&mut rows.tracks, &items, false);

    let intrinsic_content_block = tracks_used_size(&rows.tracks, row_gap);
    let intrinsic_container_block = crate::containment::logical_block_fallback(style)
        .map_or(intrinsic_content_block, |fallback| fallback)
        + border_padding_block;
    let container_block = resolve_container_block_size(
        style,
        space,
        intrinsic_container_block,
        border_padding_block,
        container_inline,
    );
    let content_block = (container_block - border_padding_block).clamp_negative_to_zero();
    distribute_flexible_tracks(&mut rows.tracks, content_block, row_gap);

    let (row_initial, row_between) = content_alignment(
        style.align_content,
        content_block,
        &mut rows.tracks,
        row_gap,
    );
    let column_offsets = track_offsets(&columns.tracks, column_gap, column_initial, column_between);
    let row_offsets = track_offsets(&rows.tracks, row_gap, row_initial, row_between);

    let physical_size =
        logical_box.physical_size(LogicalSize::new(container_inline, container_block));
    let converter = WritingModeConverter::new(writing_direction, physical_size);
    let content_origin = LogicalOffset::new(
        logical_border.inline_start + logical_padding.inline_start,
        logical_border.block_start + logical_padding.block_start,
    );
    let mut fragment = Fragment::new_box(node_id, physical_size);
    fragment.border = border;
    fragment.padding = padding;

    let mut baseline_groups: HashMap<usize, LayoutUnit> = HashMap::new();
    let mut item_fragments = Vec::new();
    for item in &items {
        let child_style = &doc.node(item.node_id).style;
        let (column_start, area_inline) = axis_area(
            &column_offsets,
            item.column_start,
            item.column_span,
            column_gap + column_between,
        );
        let (row_start, area_block) = row_area(
            &row_offsets,
            &rows.tracks,
            item.row_start,
            item.row_span,
            row_gap + row_between,
        );
        let margins = resolve_margins(child_style, content_inline);
        let logical_margins = margins.to_logical(writing_direction);
        let child_logical_box = crate::ResolvedLogicalBox::from_style(child_style);
        let normal_uses_start = doc.node(item.node_id).tag == ElementTag::Image
            || doc.node(item.node_id).replaced.is_some()
            || child_style.aspect_ratio.is_some();
        let justify = resolved_self_alignment(
            child_style.justify_self,
            style.justify_items,
            normal_uses_start,
        );
        let align =
            resolved_self_alignment(child_style.align_self, style.align_items, normal_uses_start);
        let auto_inline_start = child_logical_box.margins.inline_start.is_auto();
        let auto_inline_end = child_logical_box.margins.inline_end.is_auto();
        let auto_block_start = child_logical_box.margins.block_start.is_auto();
        let auto_block_end = child_logical_box.margins.block_end.is_auto();
        let available_inline =
            (area_inline - logical_margins.inline_start - logical_margins.inline_end)
                .clamp_negative_to_zero();
        let available_block =
            (area_block - logical_margins.block_start - logical_margins.block_end)
                .clamp_negative_to_zero();
        let stretch_inline = justify == ItemPosition::Stretch
            && style_axis_size(child_style, true).is_auto()
            && !auto_inline_start
            && !auto_inline_end;
        let stretch_block = align == ItemPosition::Stretch
            && style_axis_size(child_style, false).is_auto()
            && !auto_block_start
            && !auto_block_end;
        let mut child = layout_grid_child(
            doc,
            item.node_id,
            space,
            available_inline,
            available_block,
            area_inline,
            area_block,
            stretch_inline,
            stretch_block,
        );
        let stretched_block_extent = stretch_block.then_some(child.size.height);
        if space.has_block_fragmentation() && writing_direction.is_horizontal() {
            crate::block::apply_forced_descendant_offsets_at(
                &mut child,
                doc,
                space.fragmentainer_block_size,
                content_origin.block_offset + row_start,
            );
            // Forced descendants extend the item's scrollable/ink overflow,
            // not the border box that `align-self: stretch` resolved from its
            // grid area. Keeping the inflated fragment size here causes the
            // item's background to be cloned into later fragmentainers and
            // overlap the next row's background at fractional device scales.
            if let Some(extent) = stretched_block_extent {
                child.size.height = extent;
            }
        }
        if space.has_block_fragmentation()
            && writing_direction.is_horizontal()
            && child_logical_box.sizes.block_size.is_auto()
        {
            if let Some(fragmented_extent) = fragment_inline_content(
                &mut child,
                space.fragmentainer_block_size,
                content_origin.block_offset + row_start,
            ) {
                if !stretch_block {
                    child.size.height = child.size.height.max_of(fragmented_extent);
                }
            }
        }
        let child_logical = converter.to_logical_size(child.size);
        if space.has_block_fragmentation() && writing_direction.is_horizontal() {
            child.decoration_paint_block_size = Some(child.size.height);
        }
        let inline_free = available_inline - child_logical.inline_size;
        let block_free = available_block - child_logical.block_size;
        let inline_auto_count = usize::from(auto_inline_start) + usize::from(auto_inline_end);
        let block_auto_count = usize::from(auto_block_start) + usize::from(auto_block_end);
        let inline_auto_share = if inline_auto_count > 0 {
            inline_free.clamp_negative_to_zero() / inline_auto_count as i32
        } else {
            LayoutUnit::zero()
        };
        let block_auto_share = if block_auto_count > 0 {
            block_free.clamp_negative_to_zero() / block_auto_count as i32
        } else {
            LayoutUnit::zero()
        };
        let inline_offset = column_start
            + logical_margins.inline_start
            + if auto_inline_start {
                inline_auto_share
            } else if inline_auto_count == 0 {
                item_alignment_offset(justify, inline_free, child_style.justify_self.overflow)
            } else {
                LayoutUnit::zero()
            };
        let block_offset = row_start
            + logical_margins.block_start
            + if auto_block_start {
                block_auto_share
            } else if block_auto_count == 0 {
                item_alignment_offset(align, block_free, child_style.align_self.overflow)
            } else {
                LayoutUnit::zero()
            };
        child.offset = converter.to_physical_offset(
            LogicalOffset::new(
                content_origin.inline_offset + inline_offset,
                content_origin.block_offset + block_offset,
            ),
            child.size,
        );
        child.margin = margins;
        crate::relative::apply_relative_offset(
            &mut child,
            child_style,
            content_inline,
            content_block,
        );
        if matches!(align, ItemPosition::Baseline | ItemPosition::LastBaseline) {
            if let Some(baseline) = child.first_baseline.or(child.last_baseline) {
                baseline_groups
                    .entry(item.row_start)
                    .and_modify(|value| *value = (*value).max_of(baseline))
                    .or_insert(baseline);
            }
        }
        item_fragments.push((item.clone(), align, child));
    }
    let mut descendant_oof_candidates = Vec::new();
    for (item, align, mut child) in item_fragments {
        if matches!(align, ItemPosition::Baseline | ItemPosition::LastBaseline) {
            if let (Some(group), Some(baseline)) = (
                baseline_groups.get(&item.row_start),
                child.first_baseline.or(child.last_baseline),
            ) {
                let shift = *group - baseline;
                if writing_direction.is_horizontal() {
                    child.offset.top = child.offset.top + shift;
                } else {
                    child.offset.left = child.offset.left + shift;
                }
            }
        }
        for mut candidate in std::mem::take(&mut child.oof_candidates) {
            candidate.static_position.left = candidate.static_position.left + child.offset.left;
            candidate.static_position.top = candidate.static_position.top + child.offset.top;
            descendant_oof_candidates.push(candidate);
        }
        fragment.children.push(child);
    }

    // Direct abspos grid children use their resolved grid area as the static
    // positioning area without occupying tracks.
    let is_root = node_id == doc.root();
    let establishes_cb = style.position.is_positioned() || is_root;
    let captures_fixed = is_root || style.establishes_transform_containing_block;
    let mut candidates = Vec::new();
    let grid_positioning_geometry = |child_id: NodeId| {
        let child_style = &doc.node(child_id).style;
        let column = resolve_axis_range(
            &child_style.grid_column,
            &columns.line_names,
            columns.tracks.len(),
            placement_area(doc, child_id, style, true),
        )
        .unwrap_or(AxisRange {
            start: 0,
            span: columns.tracks.len(),
        });
        let row = resolve_axis_range(
            &child_style.grid_row,
            &rows.line_names,
            rows.tracks.len(),
            placement_area(doc, child_id, style, false),
        )
        .unwrap_or(AxisRange {
            start: 0,
            span: rows.tracks.len(),
        });
        let (column_start, area_inline) = axis_area(
            &column_offsets,
            column.start,
            column.span,
            column_gap + column_between,
        );
        let (row_start, area_block) = row_area(
            &row_offsets,
            &rows.tracks,
            row.start,
            row.span,
            row_gap + row_between,
        );
        let area_size = logical_box.physical_size(LogicalSize::new(area_inline, area_block));
        let area_offset = converter.to_physical_offset(
            LogicalOffset::new(
                content_origin.inline_offset + column_start,
                content_origin.block_offset + row_start,
            ),
            area_size,
        );
        let justify = resolved_self_alignment(child_style.justify_self, style.justify_items, false);
        let align = resolved_self_alignment(child_style.align_self, style.align_items, false);
        let edge = |position| match position {
            ItemPosition::Center => StaticPositionEdge::Center,
            ItemPosition::End
            | ItemPosition::SelfEnd
            | ItemPosition::FlexEnd
            | ItemPosition::Right => StaticPositionEdge::End,
            _ => StaticPositionEdge::Start,
        };
        let justify_edge = edge(justify);
        let align_edge = edge(align);
        let edge_offset = |edge, size| match edge {
            StaticPositionEdge::Start => LayoutUnit::zero(),
            StaticPositionEdge::Center => size / 2,
            StaticPositionEdge::End => size,
        };
        // The static-position carrier names the margin-box edge represented
        // by its coordinate.  Keep the containing-block origin at the Grid
        // area's physical start, but place the static anchor at the selected
        // start/center/end edge.  Supplying the area origin for an end edge
        // makes the generic positioned solver subtract the child's size from
        // zero, which incorrectly moves end-aligned Grid abspos children
        // before their area (and, in fragmentation, before the first column).
        let static_position = converter.to_physical_offset(
            LogicalOffset::new(
                content_origin.inline_offset
                    + column_start
                    + edge_offset(justify_edge, area_inline),
                content_origin.block_offset + row_start + edge_offset(align_edge, area_block),
            ),
            PhysicalSize::zero(),
        );
        (
            static_position,
            if writing_direction.is_horizontal() {
                justify_edge
            } else {
                align_edge
            },
            if writing_direction.is_horizontal() {
                align_edge
            } else {
                justify_edge
            },
            area_offset,
            area_size,
        )
    };

    // A positioned Grid container also captures positioned descendants that
    // bubbled out of an in-flow Grid item. Their grid-placement properties
    // resolve against this containing Grid just like those of a direct
    // abspos child; the intervening unpositioned item does not establish a
    // containing block or erase that placement information.
    for mut candidate in descendant_oof_candidates {
        let captures = if candidate.style.position == openui_style::Position::Fixed {
            captures_fixed
        } else {
            establishes_cb
        };
        if captures {
            let (static_position, horizontal_edge, vertical_edge, area_offset, area_size) =
                grid_positioning_geometry(candidate.node_id);
            candidate.static_position = static_position;
            candidate.static_position_horizontal_edge = horizontal_edge;
            candidate.static_position_vertical_edge = vertical_edge;
            candidate.containing_block_offset = area_offset;
            candidate.containing_block_node = node_id;
            candidate.containing_block_size = area_size;
            candidate.containing_block_border = BoxStrut::zero();
            candidate.containing_block_direction = style.direction;
            candidates.push(candidate);
        } else {
            fragment.oof_candidates.push(candidate);
        }
    }

    for child_id in children
        .iter()
        .copied()
        .filter(|child| doc.node(*child).style.position.is_absolutely_positioned())
    {
        let child_style = &doc.node(child_id).style;
        let (static_position, horizontal_edge, vertical_edge, area_offset, area_size) =
            grid_positioning_geometry(child_id);
        let captures = if child_style.position == openui_style::Position::Fixed {
            captures_fixed
        } else {
            establishes_cb
        };
        candidates.push(OutOfFlowCandidate {
            node_id: child_id,
            style: child_style.clone(),
            static_position,
            static_position_horizontal_edge: horizontal_edge,
            static_position_vertical_edge: vertical_edge,
            containing_block_offset: area_offset,
            containing_block_node: if captures { node_id } else { NodeId::NONE },
            containing_block_size: area_size,
            containing_block_border: BoxStrut::zero(),
            containing_block_direction: style.direction,
            static_position_direction: style.direction,
            has_inline_containing_block: false,
            inline_containing_block_node: None,
        });
    }
    let (local_candidates, bubbled_candidates): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|candidate| !candidate.containing_block_node.is_none());
    if !local_candidates.is_empty() {
        fragment
            .children
            .extend(layout_out_of_flow_children(doc, &local_candidates));
    }
    fragment.oof_candidates.extend(bubbled_candidates);

    fragment.first_baseline = fragment
        .children
        .first()
        .and_then(|child| child.first_baseline.or(child.last_baseline))
        .map(|baseline| grid_child_baseline_offset(doc, &fragment.children[0], baseline));
    fragment.last_baseline = fragment
        .children
        .last()
        .and_then(|child| child.last_baseline.or(child.first_baseline))
        .map(|baseline| {
            grid_child_baseline_offset(doc, fragment.children.last().unwrap(), baseline)
        });
    crate::block::apply_static_scroll_snap(doc, &mut fragment);
    overflow_from_children(&mut fragment, style);
    fragment
}

fn child_baseline_offset(child: &Fragment, baseline: LayoutUnit) -> LayoutUnit {
    child.offset.top + baseline
}

fn grid_child_baseline_offset(
    _doc: &Document,
    child: &Fragment,
    baseline: LayoutUnit,
) -> LayoutUnit {
    child_baseline_offset(child, baseline)
}

/// Grid min/max-content contributions used by sizing, flex, abspos, and
/// atomic-inline layout. This path never invokes layout on the container
/// itself, which prevents cyclic Grid/block intrinsic probing.
pub fn compute_grid_intrinsic_sizes(doc: &Document, node_id: NodeId) -> IntrinsicSizes {
    let style = &doc.node(node_id).style;
    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let writing_direction = style.direction.writing_direction(style.writing_mode);
    let logical_border = border.to_logical(writing_direction);
    let logical_padding = padding.to_logical(writing_direction);
    let bp_inline = logical_border.inline_sum() + logical_padding.inline_sum();
    let bp_block = logical_border.block_sum() + logical_padding.block_sum();
    let column_gap = resolve_gap(&style.column_gap, LayoutUnit::zero());
    let row_gap = resolve_gap(&style.row_gap, LayoutUnit::zero());
    let mut columns = expand_tracks(
        &style.grid_template_columns,
        INDEFINITE_SIZE,
        column_gap,
        repeated_auto_track(&style.grid_auto_columns, 0),
    );
    let mut rows = expand_tracks(
        &style.grid_template_rows,
        INDEFINITE_SIZE,
        row_gap,
        repeated_auto_track(&style.grid_auto_rows, 0),
    );
    let mut children = Vec::new();
    collect_grid_children(doc, node_id, &mut children);
    let items = place_items(doc, node_id, style, &columns, &rows, &children);
    ensure_tracks(
        &mut columns,
        items
            .iter()
            .map(|item| item.column_start + item.column_span)
            .max()
            .unwrap_or(1),
        &style.grid_auto_columns,
        INDEFINITE_SIZE,
    );
    ensure_tracks(
        &mut rows,
        items
            .iter()
            .map(|item| item.row_start + item.row_span)
            .max()
            .unwrap_or(1),
        &style.grid_auto_rows,
        INDEFINITE_SIZE,
    );
    let mut min_columns = columns.clone();
    let mut min_rows = rows.clone();
    let mut cache = HashMap::new();
    size_intrinsic_tracks(
        doc,
        &items,
        &mut columns.tracks,
        true,
        column_gap,
        INDEFINITE_SIZE,
        &mut cache,
    );
    size_intrinsic_tracks(
        doc,
        &items,
        &mut rows.tracks,
        false,
        row_gap,
        INDEFINITE_SIZE,
        &mut cache,
    );
    for item in &items {
        let intrinsic = *cache
            .entry(item.node_id)
            .or_insert_with(|| compute_child_intrinsic_contribution(doc, item.node_id));
        grow_spanning_tracks(
            &mut min_columns.tracks,
            item.column_start,
            item.column_span,
            grid_item_min_contribution(doc, item.node_id, intrinsic, true),
            column_gap,
            INDEFINITE_SIZE,
        );
        grow_spanning_tracks(
            &mut min_rows.tracks,
            item.row_start,
            item.row_span,
            grid_item_min_contribution(doc, item.node_id, intrinsic, false),
            row_gap,
            INDEFINITE_SIZE,
        );
    }
    IntrinsicSizes {
        min_content_inline_size: tracks_used_size(&min_columns.tracks, column_gap) + bp_inline,
        max_content_inline_size: tracks_used_size(&columns.tracks, column_gap) + bp_inline,
        min_content_block_size: tracks_used_size(&min_rows.tracks, row_gap) + bp_block,
        max_content_block_size: tracks_used_size(&rows.tracks, row_gap) + bp_block,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_style::BreakValue;

    fn fixed_track(px: f32) -> Track {
        Track::new(
            GridTrackSize::Breadth(GridTrackBreadth::Length(Length::px(px))),
            INDEFINITE_SIZE,
            false,
        )
    }

    #[test]
    fn forced_break_clearance_does_not_expand_an_ending_grid_area() {
        let mut doc = Document::new();
        let first = doc.create_node(ElementTag::Div);
        let second = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(second, |style| style.break_before = BreakValue::Column);
        let items = vec![
            GridItem {
                node_id: first,
                source_index: 0,
                column_start: 0,
                column_span: 1,
                row_start: 0,
                row_span: 1,
            },
            GridItem {
                node_id: second,
                source_index: 1,
                column_start: 0,
                column_span: 1,
                row_start: 1,
                row_span: 1,
            },
        ];
        let mut rows = vec![fixed_track(50.0), fixed_track(100.0)];

        apply_row_fragmentation_breaks(
            &doc,
            &items,
            &mut rows,
            LayoutUnit::zero(),
            LayoutUnit::from_i32(100),
        );

        assert_eq!(rows[0].base, LayoutUnit::from_i32(100));
        assert_eq!(rows[0].fragmentation_clearance, LayoutUnit::from_i32(50));
        let offsets = track_offsets(
            &rows,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        assert_eq!(
            row_area(&offsets, &rows, 0, 1, LayoutUnit::zero()),
            (LayoutUnit::zero(), LayoutUnit::from_i32(50),)
        );
        assert_eq!(
            row_area(&offsets, &rows, 1, 1, LayoutUnit::zero()),
            (LayoutUnit::from_i32(100), LayoutUnit::from_i32(100),)
        );
        assert_eq!(
            row_area(&offsets, &rows, 0, 2, LayoutUnit::zero()),
            (LayoutUnit::zero(), LayoutUnit::from_i32(200),)
        );
    }

    #[test]
    fn forced_descendant_overflow_does_not_expand_a_stretched_grid_item() {
        let mut doc = Document::new();
        let grid = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(grid, |style| {
            style.display = Display::Grid;
            style.width = Length::px(50.0);
            style.grid_template_rows = GridTrackList::Tracks(vec![GridTrackComponent::Track(
                GridTrackSize::Breadth(GridTrackBreadth::Length(Length::px(50.0))),
            )]);
        });
        doc.append_child(doc.root(), grid);

        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| style.display = Display::Block);
        doc.append_child(grid, item);
        let first = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(first, |style| {
            style.display = Display::Block;
            style.height = Length::px(25.0);
            style.break_after = BreakValue::Column;
        });
        doc.append_child(item, first);
        let second = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(second, |style| {
            style.display = Display::Block;
            style.height = Length::px(25.0);
        });
        doc.append_child(item, second);

        let mut space = ConstraintSpace::for_block_child(
            LayoutUnit::from_i32(50),
            LayoutUnit::from_i32(600),
            LayoutUnit::from_i32(50),
            LayoutUnit::from_i32(600),
            false,
        );
        space.fragmentainer_block_size = LayoutUnit::from_i32(100);
        let fragment = crate::block::block_layout(&doc, grid, &space);
        let item_fragment = fragment
            .children
            .iter()
            .find(|child| child.node_id == item)
            .expect("stretched grid item");

        assert_eq!(item_fragment.size.height, LayoutUnit::from_i32(50));
        assert_eq!(
            item_fragment.children[1].offset.top,
            LayoutUnit::from_i32(100)
        );
    }
}
