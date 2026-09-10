//! CSS table formatting model.
//!
//! The table algorithm owns structural fixup, track measurement, spanning-cell
//! placement, row baselines, captions, and the layered column/section/row/cell
//! fragment tree.  It intentionally consumes typed DOM/style data only; HTML
//! parsing and CSS grammar remain porter responsibilities.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{
    BoxStrut, LayoutUnit, LengthType, PhysicalOffset, PhysicalRect, PhysicalSize,
    WritingDirectionMode, INDEFINITE_SIZE,
};
use openui_style::{
    BorderCollapse, BorderStyle, BoxSizing, CaptionSide, Display, LineHeight, TableLayout,
    VerticalAlign,
};

use crate::block::{
    algorithm_box_from_logical_strut, block_layout, normalize_multicol_child_outer_box,
    project_logical_fragment_tree_to_physical, resolve_border, resolve_margins, resolve_padding,
};
use crate::constraint_space::ConstraintSpace;
use crate::fragment::{CollapsedBorderSegment, Fragment};
use crate::intrinsic_sizing::{
    compute_intrinsic_block_sizes, compute_intrinsic_inline_sizes, IntrinsicSizes,
};
use crate::length_resolver::resolve_length;
use crate::out_of_flow::OutOfFlowCandidate;

#[derive(Clone)]
struct TableRow {
    node_id: NodeId,
    group_id: Option<NodeId>,
    cells: Vec<NodeId>,
    anonymous: bool,
}

#[derive(Clone, Copy)]
struct CellSlot {
    node_id: NodeId,
    row: usize,
    column: usize,
    row_span: usize,
    column_span: usize,
}

#[derive(Default)]
struct TableModel {
    captions: Vec<NodeId>,
    columns: Vec<NodeId>,
    rows: Vec<TableRow>,
}

#[derive(Clone, Copy, Default)]
struct TrackContribution {
    min: LayoutUnit,
    max: LayoutUnit,
}

fn logical_inline_length<'a>(
    style: &'a openui_style::ComputedStyle,
    writing_direction: WritingDirectionMode,
) -> &'a openui_geometry::Length {
    if writing_direction.is_horizontal() {
        &style.width
    } else {
        &style.height
    }
}

fn logical_min_inline_length<'a>(
    style: &'a openui_style::ComputedStyle,
    writing_direction: WritingDirectionMode,
) -> &'a openui_geometry::Length {
    if writing_direction.is_horizontal() {
        &style.min_width
    } else {
        &style.min_height
    }
}

fn logical_max_inline_length<'a>(
    style: &'a openui_style::ComputedStyle,
    writing_direction: WritingDirectionMode,
) -> &'a openui_geometry::Length {
    if writing_direction.is_horizontal() {
        &style.max_width
    } else {
        &style.max_height
    }
}

fn logical_block_length<'a>(
    style: &'a openui_style::ComputedStyle,
    writing_direction: WritingDirectionMode,
) -> &'a openui_geometry::Length {
    if writing_direction.is_horizontal() {
        &style.height
    } else {
        &style.width
    }
}

fn logical_min_block_length<'a>(
    style: &'a openui_style::ComputedStyle,
    writing_direction: WritingDirectionMode,
) -> &'a openui_geometry::Length {
    if writing_direction.is_horizontal() {
        &style.min_height
    } else {
        &style.min_width
    }
}

fn logical_max_block_length<'a>(
    style: &'a openui_style::ComputedStyle,
    writing_direction: WritingDirectionMode,
) -> &'a openui_geometry::Length {
    if writing_direction.is_horizontal() {
        &style.max_height
    } else {
        &style.max_width
    }
}

fn logical_box_strut(strut: BoxStrut, writing_direction: WritingDirectionMode) -> BoxStrut {
    if writing_direction.is_horizontal() {
        strut
    } else {
        algorithm_box_from_logical_strut(strut.to_logical(writing_direction))
    }
}

fn logical_border(
    style: &openui_style::ComputedStyle,
    writing_direction: WritingDirectionMode,
) -> BoxStrut {
    logical_box_strut(resolve_border(style), writing_direction)
}

fn logical_padding(
    style: &openui_style::ComputedStyle,
    base: LayoutUnit,
    writing_direction: WritingDirectionMode,
) -> BoxStrut {
    logical_box_strut(resolve_padding(style, base), writing_direction)
}

fn logical_margins(
    style: &openui_style::ComputedStyle,
    base: LayoutUnit,
    writing_direction: WritingDirectionMode,
) -> BoxStrut {
    logical_box_strut(resolve_margins(style, base), writing_direction)
}

fn authored_borders_match(
    first: &openui_style::ComputedStyle,
    second: &openui_style::ComputedStyle,
) -> bool {
    first.border_top_width == second.border_top_width
        && first.border_right_width == second.border_right_width
        && first.border_bottom_width == second.border_bottom_width
        && first.border_left_width == second.border_left_width
        && first.border_top_style == second.border_top_style
        && first.border_right_style == second.border_right_style
        && first.border_bottom_style == second.border_bottom_style
        && first.border_left_style == second.border_left_style
        && first.border_top_color == second.border_top_color
        && first.border_right_color == second.border_right_color
        && first.border_bottom_color == second.border_bottom_color
        && first.border_left_color == second.border_left_color
}

fn collapsed_border_style_priority(style: BorderStyle) -> u8 {
    match style {
        BorderStyle::Hidden => 10,
        BorderStyle::Double => 9,
        BorderStyle::Solid => 8,
        BorderStyle::Dashed => 7,
        BorderStyle::Dotted => 6,
        BorderStyle::Ridge => 5,
        BorderStyle::Outset => 4,
        BorderStyle::Groove => 3,
        BorderStyle::Inset => 2,
        BorderStyle::None => 1,
    }
}

fn collapsed_border_wins(
    first_width: LayoutUnit,
    first_style: BorderStyle,
    second_width: LayoutUnit,
    second_style: BorderStyle,
) -> bool {
    first_style == BorderStyle::Hidden
        || (second_style != BorderStyle::Hidden
            && (first_width > second_width
                || (first_width == second_width
                    && collapsed_border_style_priority(first_style)
                        >= collapsed_border_style_priority(second_style))))
}

/// Resolve a structural table border against a cell border. Width and style
/// precedence are considered first; on an exact tie the cell wins because it
/// has the more specific collapsed-border origin.
fn collapsed_structural_border_wins(
    structural_width: LayoutUnit,
    structural_style: BorderStyle,
    cell_width: LayoutUnit,
    cell_style: BorderStyle,
) -> bool {
    structural_style == BorderStyle::Hidden
        || (cell_style != BorderStyle::Hidden
            && (structural_width > cell_width
                || (structural_width == cell_width
                    && collapsed_border_style_priority(structural_style)
                        > collapsed_border_style_priority(cell_style))))
}

fn collect_row_cells(doc: &Document, row_id: NodeId, cells: &mut Vec<NodeId>) {
    for child in doc.children(row_id) {
        let display = doc.node(child).style.display;
        if display == Display::None {
            continue;
        }
        if display == Display::Contents {
            collect_row_cells(doc, child, cells);
        } else if display == Display::TableCell || !display.is_table_internal() {
            cells.push(child);
        }
    }
}

fn collect_group_rows(
    doc: &Document,
    group_id: NodeId,
    rows: &mut Vec<TableRow>,
    captions: &mut Vec<NodeId>,
) {
    let direct_children: Vec<NodeId> = doc.children(group_id).collect();
    if direct_children.len() > 1
        && direct_children.iter().all(|child| {
            let display = doc.node(*child).style.display;
            display == Display::None || !display.is_table_internal()
        })
    {
        // A run of improper children in a row group is wrapped together in
        // one anonymous cell, not converted into one cell per DOM node. In
        // particular, `<br>` remains an inline forced break inside that cell
        // and therefore contributes another row of line boxes.
        rows.push(TableRow {
            node_id: group_id,
            group_id: Some(group_id),
            cells: vec![group_id],
            anonymous: true,
        });
        return;
    }

    fn flush_anonymous(
        group_id: NodeId,
        rows: &mut Vec<TableRow>,
        anonymous_cells: &mut Vec<NodeId>,
    ) {
        if anonymous_cells.is_empty() {
            return;
        }
        rows.push(TableRow {
            node_id: group_id,
            group_id: Some(group_id),
            cells: std::mem::take(anonymous_cells),
            anonymous: true,
        });
    }

    fn visit(
        doc: &Document,
        group_id: NodeId,
        child: NodeId,
        rows: &mut Vec<TableRow>,
        captions: &mut Vec<NodeId>,
        anonymous_cells: &mut Vec<NodeId>,
    ) {
        let display = doc.node(child).style.display;
        if display == Display::None {
            return;
        }
        if display == Display::Contents {
            for descendant in doc.children(child) {
                visit(doc, group_id, descendant, rows, captions, anonymous_cells);
            }
        } else if display == Display::TableCaption {
            // HTML parser fixup may leave a trailing caption inside an
            // implicit row group. It still generates a table-caption box at
            // the wrapper level.
            flush_anonymous(group_id, rows, anonymous_cells);
            captions.push(child);
        } else if display == Display::TableRow {
            flush_anonymous(group_id, rows, anonymous_cells);
            let mut cells = Vec::new();
            collect_row_cells(doc, child, &mut cells);
            rows.push(TableRow {
                node_id: child,
                group_id: Some(group_id),
                cells,
                anonymous: false,
            });
        } else if display == Display::TableCell || !display.is_table_internal() {
            anonymous_cells.push(child);
        }
    }

    let mut anonymous_cells = Vec::new();
    for child in doc.children(group_id) {
        visit(doc, group_id, child, rows, captions, &mut anonymous_cells);
    }
    flush_anonymous(group_id, rows, &mut anonymous_cells);
}

fn collect_columns(doc: &Document, node_id: NodeId, columns: &mut Vec<NodeId>) {
    let node = doc.node(node_id);
    if node.style.display == Display::None {
        return;
    }
    if node.style.display == Display::TableColumn {
        let span = node.table_col_span.max(1) as usize;
        columns.extend(std::iter::repeat_n(node_id, span));
        return;
    }
    let before = columns.len();
    for child in doc.children(node_id) {
        collect_columns(doc, child, columns);
    }
    if before == columns.len() && node.style.display == Display::TableColumnGroup {
        let span = node.table_col_span.max(1) as usize;
        columns.extend(std::iter::repeat_n(node_id, span));
    }
}

fn collect_table_model(doc: &Document, table_id: NodeId) -> TableModel {
    let mut model = TableModel::default();
    let mut headers = Vec::new();
    let mut bodies = Vec::new();
    let mut footers = Vec::new();
    let mut anonymous_cells = Vec::new();
    let mut saw_header = false;
    let mut saw_footer = false;

    fn flush_anonymous_cells(
        table_id: NodeId,
        bodies: &mut Vec<TableRow>,
        anonymous_cells: &mut Vec<NodeId>,
    ) {
        if anonymous_cells.is_empty() {
            return;
        }
        bodies.push(TableRow {
            node_id: table_id,
            group_id: None,
            cells: std::mem::take(anonymous_cells),
            anonymous: true,
        });
    }

    fn visit(
        doc: &Document,
        table_id: NodeId,
        node_id: NodeId,
        model: &mut TableModel,
        headers: &mut Vec<TableRow>,
        bodies: &mut Vec<TableRow>,
        footers: &mut Vec<TableRow>,
        anonymous_cells: &mut Vec<NodeId>,
        saw_header: &mut bool,
        saw_footer: &mut bool,
    ) {
        let display = doc.node(node_id).style.display;
        if display == Display::None {
            return;
        }
        match display {
            Display::Contents => {
                for child in doc.children(node_id) {
                    visit(
                        doc,
                        table_id,
                        child,
                        model,
                        headers,
                        bodies,
                        footers,
                        anonymous_cells,
                        saw_header,
                        saw_footer,
                    );
                }
            }
            Display::TableCaption => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells);
                model.captions.push(node_id);
            }
            Display::TableColumn | Display::TableColumnGroup => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells);
                collect_columns(doc, node_id, &mut model.columns);
                if display == Display::TableColumnGroup {
                    // Malformed source can make an HTML parser retain later
                    // rows/captions under a colgroup. Columns remain column
                    // metadata, while those later structural boxes still
                    // participate in the table grid/wrapper.
                    for child in doc.children(node_id) {
                        if !matches!(
                            doc.node(child).style.display,
                            Display::TableColumn | Display::TableColumnGroup
                        ) {
                            visit(
                                doc,
                                table_id,
                                child,
                                model,
                                headers,
                                bodies,
                                footers,
                                anonymous_cells,
                                saw_header,
                                saw_footer,
                            );
                        }
                    }
                }
            }
            Display::TableHeaderGroup => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells);
                if *saw_header {
                    collect_group_rows(doc, node_id, bodies, &mut model.captions);
                } else {
                    *saw_header = true;
                    collect_group_rows(doc, node_id, headers, &mut model.captions);
                }
            }
            Display::TableFooterGroup => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells);
                if *saw_footer {
                    collect_group_rows(doc, node_id, bodies, &mut model.captions);
                } else {
                    *saw_footer = true;
                    collect_group_rows(doc, node_id, footers, &mut model.captions);
                }
            }
            Display::TableRowGroup => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells);
                collect_group_rows(doc, node_id, bodies, &mut model.captions)
            }
            Display::TableRow => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells);
                let mut cells = Vec::new();
                collect_row_cells(doc, node_id, &mut cells);
                bodies.push(TableRow {
                    node_id,
                    group_id: None,
                    cells,
                    anonymous: false,
                });
            }
            Display::TableCell => anonymous_cells.push(node_id),
            _ => anonymous_cells.push(node_id),
        }
    }

    for child in doc.children(table_id) {
        visit(
            doc,
            table_id,
            child,
            &mut model,
            &mut headers,
            &mut bodies,
            &mut footers,
            &mut anonymous_cells,
            &mut saw_header,
            &mut saw_footer,
        );
    }
    flush_anonymous_cells(table_id, &mut bodies, &mut anonymous_cells);
    model.rows.extend(headers);
    model.rows.extend(bodies);
    model.rows.extend(footers);
    model
}

fn group_end(rows: &[TableRow], row: usize) -> usize {
    let group = rows[row].group_id;
    let mut end = row + 1;
    while end < rows.len() && rows[end].group_id == group {
        end += 1;
    }
    end
}

fn place_cells(doc: &Document, rows: &[TableRow]) -> (Vec<CellSlot>, usize) {
    let mut occupied_until: Vec<usize> = Vec::new();
    let mut slots = Vec::new();
    let mut column_count = 0usize;
    for (row_index, row) in rows.iter().enumerate() {
        let mut column = 0usize;
        for &cell_id in &row.cells {
            while occupied_until
                .get(column)
                .is_some_and(|&end| end > row_index)
            {
                column += 1;
            }
            let cell = doc.node(cell_id);
            let column_span = cell.table_col_span.max(1) as usize;
            let row_span = if cell.table_row_span == 0 {
                group_end(rows, row_index) - row_index
            } else {
                (cell.table_row_span as usize).min(rows.len() - row_index)
            }
            .max(1);
            if occupied_until.len() < column + column_span {
                occupied_until.resize(column + column_span, 0);
            }
            for occupied in &mut occupied_until[column..column + column_span] {
                *occupied = (*occupied).max(row_index + row_span);
            }
            slots.push(CellSlot {
                node_id: cell_id,
                row: row_index,
                column,
                row_span,
                column_span,
            });
            column += column_span;
            column_count = column_count.max(column);
        }
        column_count = column_count.max(occupied_until.len());
    }
    (slots, column_count)
}

fn collapsed_inline_boundary_for_cell(
    doc: &Document,
    slots: &[CellSlot],
    cell: CellSlot,
    boundary: usize,
    writing_direction: WritingDirectionMode,
) -> LayoutUnit {
    slots
        .iter()
        .filter(|candidate| {
            candidate.row < cell.row + cell.row_span
                && cell.row < candidate.row + candidate.row_span
        })
        .fold(LayoutUnit::zero(), |winner, candidate| {
            let border = logical_border(&doc.node(candidate.node_id).style, writing_direction);
            let edge = if candidate.column == boundary {
                border.left
            } else if candidate.column + candidate.column_span == boundary {
                border.right
            } else {
                LayoutUnit::zero()
            };
            winner.max_of(edge)
        })
}

fn add_spanning_contribution(
    tracks: &mut [TrackContribution],
    start: usize,
    span: usize,
    min: LayoutUnit,
    max: LayoutUnit,
    inline_spacing: LayoutUnit,
) {
    if tracks.is_empty() || start >= tracks.len() {
        return;
    }
    let end = (start + span).min(tracks.len());
    let count = (end - start) as i32;
    let internal_spacing =
        inline_spacing * LayoutUnit::from_i32((end - start).saturating_sub(1) as i32);
    let min = (min - internal_spacing).clamp_negative_to_zero();
    let max = (max - internal_spacing).clamp_negative_to_zero();
    let current_min = tracks[start..end]
        .iter()
        .fold(LayoutUnit::zero(), |sum, track| sum + track.min);
    let current_max = tracks[start..end]
        .iter()
        .fold(LayoutUnit::zero(), |sum, track| sum + track.max);
    if min > current_min {
        let deficit = (min - current_min).raw();
        let share = LayoutUnit::from_raw(deficit / count);
        for track in &mut tracks[start..end] {
            track.min = track.min + share;
        }
        tracks[end - 1].min = tracks[end - 1].min + LayoutUnit::from_raw(deficit % count);
    }
    if max > current_max {
        let deficit = (max - current_max).raw();
        let share = LayoutUnit::from_raw(deficit / count);
        for track in &mut tracks[start..end] {
            track.max = track.max + share;
        }
        tracks[end - 1].max = tracks[end - 1].max + LayoutUnit::from_raw(deficit % count);
    }
    for track in &mut tracks[start..end] {
        track.max = track.max.max_of(track.min);
    }
}

fn measure_tracks(
    doc: &Document,
    table_id: NodeId,
    model: &TableModel,
    slots: &[CellSlot],
    column_count: usize,
    writing_direction: WritingDirectionMode,
) -> Vec<TrackContribution> {
    let mut tracks = vec![TrackContribution::default(); column_count];
    let inline_spacing = resolved_spacing(doc, table_id, LayoutUnit::zero()).0;
    for (index, &column_id) in model.columns.iter().take(column_count).enumerate() {
        let style = &doc.node(column_id).style;
        let inline_size = logical_inline_length(style, writing_direction);
        if inline_size.length_type() == LengthType::Fixed {
            let width = resolve_length(
                inline_size,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            tracks[index].min = tracks[index].min.max_of(width);
            tracks[index].max = tracks[index].max.max_of(width);
        }
    }
    for slot in slots {
        let mut sizes = compute_intrinsic_block_sizes(doc, slot.node_id);
        if doc.node(slot.node_id).tag == ElementTag::Text {
            let inline = compute_intrinsic_inline_sizes(doc, slot.node_id);
            sizes.min_content_inline_size = inline.min;
            sizes.max_content_inline_size = inline.max;
        }
        let cell_style = &doc.node(slot.node_id).style;
        let specified =
            authored_inline_size(doc, slot.node_id, LayoutUnit::zero(), writing_direction).map(
                |width| {
                    if cell_style.box_sizing == BoxSizing::BorderBox {
                        width
                    } else {
                        width
                            + logical_border(cell_style, writing_direction).inline_sum()
                            + logical_padding(cell_style, LayoutUnit::zero(), writing_direction)
                                .inline_sum()
                    }
                },
            );
        // A non-internal child is wrapped in an anonymous table cell. Its
        // definite outer inline size is the anonymous cell's intrinsic
        // contribution; using the child's unconstrained flex/block max-
        // content measure here can incorrectly expand an auto-width table to
        // the whole containing block.
        let anonymous_definite = (cell_style.display != Display::TableCell)
            .then_some(specified)
            .flatten();
        let min_contribution = anonymous_definite.unwrap_or_else(|| {
            specified.map_or(sizes.min_content_inline_size, |width| {
                sizes.min_content_inline_size.max_of(width)
            })
        });
        let max_contribution = anonymous_definite.unwrap_or_else(|| {
            specified.map_or(sizes.max_content_inline_size, |width| {
                // A definite cell width is its preferred contribution. It
                // still cannot compress below unbreakable content, but soft
                // opportunities between atomic inline children are allowed
                // to wrap instead of forcing the table to their one-line
                // max-content sum.
                sizes.min_content_inline_size.max_of(width)
            })
        });
        add_spanning_contribution(
            &mut tracks,
            slot.column,
            slot.column_span,
            min_contribution,
            max_contribution,
            inline_spacing,
        );
    }
    if doc.node(table_id).style.border_collapse == BorderCollapse::Collapse {
        // Adjacent cell border boxes share one collapsed grid line. Intrinsic
        // measurement initially includes both authored border edges, so remove
        // the winning shared edge once, split evenly between its neighboring
        // tracks. Conflict style/color selection remains a paint concern.
        for boundary in 1..column_count {
            let left = slots
                .iter()
                .filter(|slot| slot.column + slot.column_span == boundary)
                .map(|slot| logical_border(&doc.node(slot.node_id).style, writing_direction).right)
                .max()
                .unwrap_or(LayoutUnit::zero());
            let right = slots
                .iter()
                .filter(|slot| slot.column == boundary)
                .map(|slot| logical_border(&doc.node(slot.node_id).style, writing_direction).left)
                .max()
                .unwrap_or(LayoutUnit::zero());
            // Contributions contain both authored edges. Collapse removes
            // only the duplicated portion: if one side has no border, the
            // other side's winning edge is already counted exactly once.
            let shared = left.min_of(right);
            let left_share = shared / LayoutUnit::from_i32(2);
            let right_share = shared - left_share;
            tracks[boundary - 1].min =
                (tracks[boundary - 1].min - left_share).clamp_negative_to_zero();
            tracks[boundary - 1].max = tracks[boundary - 1].max.max_of(tracks[boundary - 1].min)
                - left_share.min_of(tracks[boundary - 1].max);
            tracks[boundary].min = (tracks[boundary].min - right_share).clamp_negative_to_zero();
            tracks[boundary].max = tracks[boundary].max.max_of(tracks[boundary].min)
                - right_share.min_of(tracks[boundary].max);
        }
    }
    tracks
}

fn sum_tracks(tracks: &[TrackContribution], max_content: bool) -> LayoutUnit {
    tracks.iter().fold(LayoutUnit::zero(), |sum, track| {
        sum + if max_content { track.max } else { track.min }
    })
}

fn resolved_spacing(
    doc: &Document,
    table_id: NodeId,
    base: LayoutUnit,
) -> (LayoutUnit, LayoutUnit) {
    let style = &doc.node(table_id).style;
    if style.border_collapse == BorderCollapse::Collapse {
        return (LayoutUnit::zero(), LayoutUnit::zero());
    }
    (
        resolve_length(
            &style.border_spacing.0,
            base,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
        .clamp_negative_to_zero(),
        resolve_length(
            &style.border_spacing.1,
            base,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
        .clamp_negative_to_zero(),
    )
}

fn single_cell_collapsed_structural_perimeter(
    doc: &Document,
    table_id: NodeId,
    model: &TableModel,
    slots: &[CellSlot],
    column_count: usize,
    writing_direction: WritingDirectionMode,
) -> Option<BoxStrut> {
    if doc.node(table_id).style.border_collapse != BorderCollapse::Collapse
        || model.rows.len() != 1
        || column_count != 1
        || slots.len() != 1
    {
        return None;
    }
    let table_border = logical_border(&doc.node(table_id).style, writing_direction);
    let cell_border = logical_border(&doc.node(slots[0].node_id).style, writing_direction);
    if table_border != BoxStrut::zero() || cell_border != BoxStrut::zero() {
        return None;
    }
    let mut perimeter = BoxStrut::zero();
    let mut include = |node_id: NodeId| {
        let border = logical_border(&doc.node(node_id).style, writing_direction);
        perimeter.top = perimeter.top.max_of(border.top);
        perimeter.right = perimeter.right.max_of(border.right);
        perimeter.bottom = perimeter.bottom.max_of(border.bottom);
        perimeter.left = perimeter.left.max_of(border.left);
    };
    include(model.rows[0].node_id);
    if let Some(group_id) = model.rows[0].group_id {
        include(group_id);
    }
    for &column_id in &model.columns {
        include(column_id);
    }
    (perimeter != BoxStrut::zero()).then_some(perimeter)
}

/// Table min/max contributions, including captions, spacing, table edges, and
/// cell spans. Table cells deliberately use the ordinary block intrinsic
/// algorithm, so nested flex/multicol/replaced content contributes normally.
pub fn compute_table_intrinsic_sizes(doc: &Document, table_id: NodeId) -> IntrinsicSizes {
    let style = &doc.node(table_id).style;
    let writing_direction = style.writing_mode.to_writing_direction(style.direction);
    let model = collect_table_model(doc, table_id);
    let (slots, column_count) = place_cells(doc, &model.rows);
    let tracks = measure_tracks(
        doc,
        table_id,
        &model,
        &slots,
        column_count,
        writing_direction,
    );
    let (inline_spacing, block_spacing) = resolved_spacing(doc, table_id, LayoutUnit::zero());
    let border = logical_border(style, writing_direction);
    let padding = logical_padding(style, LayoutUnit::zero(), writing_direction);
    let margins = logical_margins(style, LayoutUnit::zero(), writing_direction);
    let horizontal_edges = border.inline_sum() + padding.inline_sum();
    let vertical_edges = border.block_sum() + padding.block_sum();
    let gaps_inline = inline_spacing * LayoutUnit::from_i32((column_count + 1) as i32);
    let mut min_inline = sum_tracks(&tracks, false) + gaps_inline + horizontal_edges;
    let mut max_inline = sum_tracks(&tracks, true) + gaps_inline + horizontal_edges;
    let mut caption_min = LayoutUnit::zero();
    let mut caption_max = LayoutUnit::zero();
    let mut caption_block = LayoutUnit::zero();
    for caption in model.captions {
        let caption_style = &doc.node(caption).style;
        let sizes = compute_intrinsic_block_sizes(doc, caption);
        let specified_outer =
            authored_inline_size(doc, caption, LayoutUnit::zero(), writing_direction).map(
                |width| {
                    let border_padding = logical_border(caption_style, writing_direction)
                        .inline_sum()
                        + logical_padding(caption_style, LayoutUnit::zero(), writing_direction)
                            .inline_sum();
                    let border_box = if caption_style.box_sizing == BoxSizing::BorderBox {
                        width
                    } else {
                        width + border_padding
                    };
                    border_box
                        + logical_margins(caption_style, LayoutUnit::zero(), writing_direction)
                            .inline_sum()
                },
            );
        caption_min = caption_min.max_of(
            specified_outer.map_or(sizes.min_content_inline_size, |width| {
                sizes.min_content_inline_size.max_of(width)
            }),
        );
        caption_max = caption_max.max_of(
            specified_outer.map_or(sizes.max_content_inline_size, |width| {
                sizes.max_content_inline_size.max_of(width)
            }),
        );
        caption_block = caption_block + sizes.max_content_block_size;
    }
    min_inline = min_inline.max_of(caption_min);
    max_inline = max_inline.max_of(caption_max);

    let mut row_min = vec![LayoutUnit::zero(); model.rows.len()];
    let mut row_max = vec![LayoutUnit::zero(); model.rows.len()];
    for slot in slots {
        let sizes = compute_intrinsic_block_sizes(doc, slot.node_id);
        let share_min =
            LayoutUnit::from_f32(sizes.min_content_block_size.to_f32() / slot.row_span as f32);
        let share_max =
            LayoutUnit::from_f32(sizes.max_content_block_size.to_f32() / slot.row_span as f32);
        for row in slot.row..slot.row + slot.row_span {
            row_min[row] = row_min[row].max_of(share_min);
            row_max[row] = row_max[row].max_of(share_max);
        }
    }
    let block_gaps = block_spacing * LayoutUnit::from_i32((model.rows.len() + 1) as i32);
    let min_block = row_min.into_iter().fold(LayoutUnit::zero(), |a, b| a + b)
        + block_gaps
        + vertical_edges
        + caption_block;
    let max_block = row_max.into_iter().fold(LayoutUnit::zero(), |a, b| a + b)
        + block_gaps
        + vertical_edges
        + caption_block;
    IntrinsicSizes {
        min_content_inline_size: min_inline + margins.inline_sum(),
        max_content_inline_size: max_inline + margins.inline_sum(),
        min_content_block_size: min_block + margins.block_sum(),
        max_content_block_size: max_block + margins.block_sum(),
    }
}

pub(crate) fn compute_table_caption_block_size(
    doc: &Document,
    table_id: NodeId,
    inline_size: LayoutUnit,
    parent_space: &ConstraintSpace,
) -> LayoutUnit {
    if inline_size.is_indefinite() {
        return LayoutUnit::zero();
    }
    collect_table_model(doc, table_id)
        .captions
        .into_iter()
        .map(|caption_id| {
            let caption_style = &doc.node(caption_id).style;
            let direction = caption_style
                .writing_mode
                .to_writing_direction(caption_style.direction);
            let mut caption_space = ConstraintSpace::for_block_child_from_parent(
                parent_space,
                inline_size,
                INDEFINITE_SIZE,
                inline_size,
                parent_space.percentage_resolution_block_size,
                true,
                direction,
            );
            caption_space.is_fixed_inline_size =
                authored_inline_size(doc, caption_id, inline_size, parent_space.writing_direction)
                    .is_none();
            let mut caption = block_layout(doc, caption_id, &caption_space);
            normalize_multicol_child_outer_box(doc, &mut caption, parent_space.writing_direction);
            let margin =
                logical_margins(caption_style, inline_size, parent_space.writing_direction);
            margin.top + caption.size.height + margin.bottom
        })
        .fold(LayoutUnit::zero(), |sum, size| sum + size)
}

fn distribute_auto_tracks(tracks: &[TrackContribution], target: LayoutUnit) -> Vec<LayoutUnit> {
    if tracks.is_empty() {
        return Vec::new();
    }
    let mut widths: Vec<LayoutUnit> = tracks.iter().map(|track| track.min).collect();
    let min_sum = widths
        .iter()
        .copied()
        .fold(LayoutUnit::zero(), |a, b| a + b);
    if target <= min_sum {
        return widths;
    }
    let max_sum = sum_tracks(tracks, true);
    if max_sum > min_sum {
        let fraction = ((target - min_sum).to_f32() / (max_sum - min_sum).to_f32()).clamp(0.0, 1.0);
        for (width, track) in widths.iter_mut().zip(tracks) {
            *width = *width + LayoutUnit::from_f32((track.max - track.min).to_f32() * fraction);
        }
    }
    let used = widths
        .iter()
        .copied()
        .fold(LayoutUnit::zero(), |a, b| a + b);
    if target > used {
        // Preserve the requested grid width exactly. Converting a subpixel
        // remainder through f32 can truncate every per-track share to zero
        // (for example, one raw 1/64 unit split across two tracks), leaving
        // the table one unit narrower than its resolved width. Distribute in
        // raw fixed-point units and assign the indivisible remainder in track
        // order, matching the stable logical-start bias used by track sizing.
        let remaining = (target - used).raw();
        let count = widths.len() as i32;
        let share = remaining / count;
        let remainder = remaining % count;
        for (index, width) in widths.iter_mut().enumerate() {
            *width = *width + LayoutUnit::from_raw(share + i32::from((index as i32) < remainder));
        }
    }
    widths
}

fn authored_inline_size(
    doc: &Document,
    node_id: NodeId,
    base: LayoutUnit,
    writing_direction: WritingDirectionMode,
) -> Option<LayoutUnit> {
    let style = &doc.node(node_id).style;
    let inline_size = logical_inline_length(style, writing_direction);
    (!inline_size.is_auto() && !inline_size.is_content_or_intrinsic() && !inline_size.is_stretch())
        .then(|| resolve_length(inline_size, base, LayoutUnit::zero(), LayoutUnit::zero()))
}

fn distribute_fixed_tracks(
    doc: &Document,
    model: &TableModel,
    slots: &[CellSlot],
    column_count: usize,
    target: LayoutUnit,
    writing_direction: WritingDirectionMode,
) -> Vec<LayoutUnit> {
    if column_count == 0 {
        return Vec::new();
    }
    let mut widths = vec![None; column_count];
    for (index, &column_id) in model.columns.iter().take(column_count).enumerate() {
        widths[index] = authored_inline_size(doc, column_id, target, writing_direction);
    }
    if let Some(first_row) = model.rows.first() {
        for slot in slots.iter().filter(|slot| slot.row == 0) {
            if !first_row.cells.contains(&slot.node_id) {
                continue;
            }
            if let Some(width) = authored_inline_size(doc, slot.node_id, target, writing_direction)
            {
                let share = LayoutUnit::from_f32(width.to_f32() / slot.column_span as f32);
                for track in
                    &mut widths[slot.column..(slot.column + slot.column_span).min(column_count)]
                {
                    if track.is_none() {
                        *track = Some(share);
                    }
                }
            }
        }
    }
    let specified = widths
        .iter()
        .flatten()
        .copied()
        .fold(LayoutUnit::zero(), |a, b| a + b);
    let unspecified = widths.iter().filter(|width| width.is_none()).count();
    let share = if unspecified == 0 {
        LayoutUnit::zero()
    } else {
        LayoutUnit::from_f32(
            (target - specified).clamp_negative_to_zero().to_f32() / unspecified as f32,
        )
    };
    widths
        .into_iter()
        .map(|width| width.unwrap_or(share))
        .collect()
}

fn resolve_table_grid_width(
    doc: &Document,
    table_id: NodeId,
    space: &ConstraintSpace,
    tracks: &[TrackContribution],
    column_count: usize,
    inline_spacing: LayoutUnit,
    writing_direction: WritingDirectionMode,
) -> LayoutUnit {
    let style = &doc.node(table_id).style;
    let border = logical_border(style, writing_direction);
    let padding = logical_padding(
        style,
        space.percentage_resolution_inline_size,
        writing_direction,
    );
    let edges = border.inline_sum() + padding.inline_sum();
    let gaps = inline_spacing * LayoutUnit::from_i32((column_count + 1) as i32);
    let min_grid = sum_tracks(tracks, false);
    let max_grid = sum_tracks(tracks, true).max_of(min_grid);
    let inline_size = logical_inline_length(style, writing_direction);
    let min_inline_size = logical_min_inline_length(style, writing_direction);
    let max_inline_size = logical_max_inline_length(style, writing_direction);

    let mut grid = if inline_size.length_type() == LengthType::MinContent {
        min_grid
    } else if inline_size.length_type() == LengthType::MaxContent {
        max_grid
    } else if inline_size.length_type() == LengthType::FitContent {
        if space.available_inline_size.is_indefinite() {
            max_grid
        } else {
            let available = (space.available_inline_size - edges - gaps).clamp_negative_to_zero();
            max_grid.min_of(available).max_of(min_grid)
        }
    } else if space.is_fixed_inline_size
        || space.stretch_inline_size
        || (inline_size.is_stretch() && !space.available_inline_size.is_indefinite())
    {
        (space.available_inline_size - edges - gaps).clamp_negative_to_zero()
    } else if let Some(width) = authored_inline_size(
        doc,
        table_id,
        space.percentage_resolution_inline_size,
        writing_direction,
    ) {
        let content = if style.box_sizing == BoxSizing::BorderBox {
            (width - edges).clamp_negative_to_zero()
        } else {
            width
        };
        (content - gaps).clamp_negative_to_zero()
    } else if space.available_inline_size.is_indefinite() {
        max_grid
    } else {
        let available = (space.available_inline_size - edges - gaps).clamp_negative_to_zero();
        max_grid.min_of(available).max_of(min_grid)
    };

    if !min_inline_size.is_auto() {
        let min_width = resolve_length(
            min_inline_size,
            space.percentage_resolution_inline_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        grid = grid.max_of((min_width - edges - gaps).clamp_negative_to_zero());
    }
    if !max_inline_size.is_none() && !max_inline_size.is_auto() {
        let max_width = resolve_length(
            max_inline_size,
            space.percentage_resolution_inline_size,
            LayoutUnit::max(),
            LayoutUnit::max(),
        );
        grid = grid.min_of((max_width - edges - gaps).clamp_negative_to_zero());
    }
    if style.table_layout == TableLayout::Fixed
        && (authored_inline_size(
            doc,
            table_id,
            space.percentage_resolution_inline_size,
            writing_direction,
        )
        .is_some()
            || space.is_fixed_inline_size
            || space.stretch_inline_size)
    {
        grid
    } else {
        grid.max_of(min_grid)
    }
}

struct LaidCell {
    slot: CellSlot,
    fragment: Fragment,
    natural_height: LayoutUnit,
    alignment_height: LayoutUnit,
    baseline: LayoutUnit,
}

fn table_cell_alignment_height(fragment: &Fragment) -> LayoutUnit {
    let content_bottom = fragment
        .children
        .iter()
        .map(|child| child.offset.top + child.size.height)
        .max()
        .unwrap_or(fragment.border.top + fragment.padding.top);
    content_bottom + fragment.padding.bottom + fragment.border.bottom
}

fn layout_cell(
    doc: &Document,
    slot: CellSlot,
    width: LayoutUnit,
    table_space: &ConstraintSpace,
    percentage_block_size: LayoutUnit,
) -> LaidCell {
    let style = &doc.node(slot.node_id).style;
    let direction = style.writing_mode.to_writing_direction(style.direction);
    let mut cell_space = ConstraintSpace::for_block_child_from_parent(
        table_space,
        width,
        INDEFINITE_SIZE,
        width,
        percentage_block_size,
        true,
        direction,
    );
    cell_space.is_fixed_inline_size = true;
    if style.display == Display::TableCell && !percentage_block_size.is_indefinite() {
        // The table's second cell pass owns the final spanned row size. Make
        // that a definite cell block constraint so percentage-sized
        // descendants resolve through the table cell even though CSS treats
        // the cell's authored `height` as a minimum during its natural pass.
        cell_space.available_block_size = percentage_block_size;
        cell_space.is_fixed_block_size = true;
    }
    cell_space.needs_first_baseline = true;
    cell_space.needs_last_baseline = true;
    let mut fragment = if doc.node(slot.node_id).tag == ElementTag::Text {
        // Text directly inside a CSS table is wrapped by an anonymous cell.
        // Format that one-node inline sequence through a detached anonymous
        // block, then remap its text fragment back to the source node so paint
        // and hit testing retain DOM identity.
        let mut anonymous_doc = Document::new();
        let wrapper = anonymous_doc.create_node(ElementTag::Div);
        anonymous_doc.node_mut(wrapper).style = style.clone();
        anonymous_doc.node_mut(wrapper).style.display = Display::Block;
        anonymous_doc.append_child(anonymous_doc.root(), wrapper);
        let text = anonymous_doc.create_node(ElementTag::Text);
        anonymous_doc.node_mut(text).style = style.clone();
        anonymous_doc.node_mut(text).text = doc.node(slot.node_id).text.clone();
        anonymous_doc.append_child(wrapper, text);
        let mut fragment = block_layout(&anonymous_doc, wrapper, &cell_space);
        fn remap(fragment: &mut Fragment, wrapper: NodeId, text: NodeId, source: NodeId) {
            if fragment.node_id == wrapper {
                fragment.node_id = NodeId::NONE;
            } else if fragment.node_id == text {
                fragment.node_id = source;
            }
            for child in &mut fragment.children {
                remap(child, wrapper, text, source);
            }
        }
        remap(&mut fragment, wrapper, text, slot.node_id);
        fragment
    } else {
        block_layout(doc, slot.node_id, &cell_space)
    };
    normalize_multicol_child_outer_box(doc, &mut fragment, table_space.writing_direction);
    if style.display == Display::TableCell
        && crate::inline::algorithm::has_inline_children(doc, slot.node_id)
    {
        if let LineHeight::Number(multiplier) = style.line_height {
            let line_count = fragment.children.len().max(1) as f32;
            let expected = LayoutUnit::from_f32(style.font_size * multiplier * line_count);
            let excess = fragment.size.height - expected;
            if excess > LayoutUnit::zero() && excess <= LayoutUnit::from_i32(1) {
                shift_children(&mut fragment, -excess);
                fragment.size.height = expected;
                if let Some(value) = fragment.first_baseline.as_mut() {
                    *value = *value - excess;
                }
                if let Some(value) = fragment.last_baseline.as_mut() {
                    *value = *value - excess;
                }
            }
        }
    }
    let mut natural_height = fragment.size.height;
    if direction.is_horizontal() != table_space.writing_direction.is_horizontal() {
        // An orthogonal cell is measured in its own logical axes and then
        // projected into the table's axes.  Its auto block-size can therefore
        // be zero even though in-flow contents have a non-zero physical
        // extent in the table row's block axis.  That extent is still a
        // minimum contribution to the row (CSS Tables 3, row height).
        natural_height = natural_height.max_of(table_cell_alignment_height(&fragment));
    }
    // A cell's authored block-size is a minimum for row sizing, but it does
    // not become part of the contents that `vertical-align` positions. Keep
    // the full border-box minimum in `natural_height` while measuring the
    // actual in-flow contents separately for middle/bottom alignment.
    let authored_cell_block_size = logical_block_length(style, table_space.writing_direction);
    let mut alignment_height = if style.display == Display::TableCell
        && !authored_cell_block_size.is_auto()
        && !authored_cell_block_size.is_content_or_intrinsic()
        && !authored_cell_block_size.is_stretch()
    {
        table_cell_alignment_height(&fragment).min_of(natural_height)
    } else {
        natural_height
    };
    let mut baseline = fragment
        .first_baseline
        .unwrap_or(natural_height - fragment.border.bottom - fragment.padding.bottom);
    if style.display != Display::TableCell {
        // Improper table children live inside an anonymous cell. Preserve the
        // source box at its margin-box position rather than stretching its
        // own background/overflow box to the row dimensions.
        let margins = logical_margins(style, width, table_space.writing_direction);
        fragment.offset = PhysicalOffset::new(margins.left, margins.top);
        baseline = baseline + margins.top;
        natural_height = margins.top + natural_height + margins.bottom;
        alignment_height = natural_height;
        let mut anonymous =
            Fragment::new_box(NodeId::NONE, PhysicalSize::new(width, natural_height));
        anonymous.first_baseline = Some(baseline);
        anonymous.last_baseline = fragment.last_baseline.map(|value| value + margins.top);
        anonymous.children.push(fragment);
        anonymous.oof_candidates = adopt_child_oof_candidates(&mut anonymous.children);
        fragment = anonymous;
    }
    LaidCell {
        slot,
        fragment,
        natural_height,
        alignment_height,
        baseline,
    }
}

fn minimum_row_height(
    doc: &Document,
    row: &TableRow,
    base: LayoutUnit,
    writing_direction: WritingDirectionMode,
) -> LayoutUnit {
    if row.anonymous {
        return LayoutUnit::zero();
    }
    let style = &doc.node(row.node_id).style;
    let block_size = logical_block_length(style, writing_direction);
    if block_size.is_auto()
        || block_size.is_content_or_intrinsic()
        || block_size.is_stretch()
        || block_size.length_type() == LengthType::Percent
    {
        // A row percentage resolves against the table grid's eventual
        // definite block size, not the table's containing block. Its natural
        // minimum is therefore zero; surplus distribution supplies the final
        // row size once the table height is known.
        LayoutUnit::zero()
    } else {
        resolve_length(block_size, base, LayoutUnit::zero(), LayoutUnit::zero())
    }
}

fn definite_table_internal_block_base(
    doc: &Document,
    node_id: NodeId,
    percentage_base: LayoutUnit,
    writing_direction: WritingDirectionMode,
) -> LayoutUnit {
    let block_size = logical_block_length(&doc.node(node_id).style, writing_direction);
    if block_size.is_auto() || block_size.is_content_or_intrinsic() || block_size.is_stretch() {
        LayoutUnit::zero()
    } else {
        resolve_length(
            block_size,
            percentage_base,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
    }
}

fn subtree_uses_percentage_block_size(
    doc: &Document,
    node_id: NodeId,
    writing_direction: WritingDirectionMode,
) -> bool {
    matches!(
        logical_block_length(&doc.node(node_id).style, writing_direction).length_type(),
        LengthType::Percent | LengthType::Calculated
    ) || doc
        .children(node_id)
        .any(|child_id| subtree_uses_percentage_block_size(doc, child_id, writing_direction))
}

fn shift_children(fragment: &mut Fragment, amount: LayoutUnit) {
    if amount == LayoutUnit::zero() {
        return;
    }
    for child in &mut fragment.children {
        child.offset.top = child.offset.top + amount;
    }
    for candidate in &mut fragment.oof_candidates {
        candidate.static_position.top = candidate.static_position.top + amount;
    }
}

fn adopt_child_oof_candidates(children: &mut [Fragment]) -> Vec<OutOfFlowCandidate> {
    let mut candidates = Vec::new();
    for child in children {
        let child_offset = child.offset;
        for mut candidate in std::mem::take(&mut child.oof_candidates) {
            candidate.static_position.left = candidate.static_position.left + child_offset.left;
            candidate.static_position.top = candidate.static_position.top + child_offset.top;
            candidates.push(candidate);
        }
    }
    candidates
}

fn resolve_table_box_oof(doc: &Document, fragment: &mut Fragment) {
    if fragment.node_id.is_none() || fragment.oof_candidates.is_empty() {
        return;
    }
    let node_id = fragment.node_id;
    let style = &doc.node(node_id).style;
    let captures_abs = style.position.is_positioned()
        || style.establishes_transform_containing_block
        || style.opacity < 1.0;
    let captures_fixed = style.establishes_transform_containing_block;
    let cb_size = PhysicalSize::new(
        (fragment.size.width - fragment.border.inline_sum()).clamp_negative_to_zero(),
        (fragment.size.height - fragment.border.block_sum()).clamp_negative_to_zero(),
    );
    let mut pending = Vec::new();
    let mut deferred = Vec::new();
    for mut candidate in std::mem::take(&mut fragment.oof_candidates) {
        let captures = candidate.has_inline_containing_block
            || if candidate.style.position == openui_style::Position::Fixed {
                captures_fixed
            } else {
                captures_abs
            };
        if captures {
            if !candidate.has_inline_containing_block {
                candidate.containing_block_size = cb_size;
                candidate.containing_block_border = fragment.border;
                candidate.containing_block_direction = style.direction;
                candidate.containing_block_node = node_id;
            }
            pending.push(candidate);
        } else {
            deferred.push(candidate);
        }
    }

    while !pending.is_empty() {
        let laid_out = crate::out_of_flow::layout_out_of_flow_children(doc, &pending);
        pending = Vec::new();
        for mut positioned in laid_out {
            let positioned_offset = positioned.offset;
            for mut nested in std::mem::take(&mut positioned.oof_candidates) {
                nested.static_position.left = nested.static_position.left + positioned_offset.left;
                nested.static_position.top = nested.static_position.top + positioned_offset.top;
                let captures = nested.has_inline_containing_block
                    || if nested.style.position == openui_style::Position::Fixed {
                        captures_fixed
                    } else {
                        captures_abs
                    };
                if captures {
                    if !nested.has_inline_containing_block {
                        nested.containing_block_size = cb_size;
                        nested.containing_block_border = fragment.border;
                        nested.containing_block_direction = style.direction;
                        nested.containing_block_node = node_id;
                    }
                    pending.push(nested);
                } else {
                    deferred.push(nested);
                }
            }
            fragment.children.push(positioned);
        }
    }
    fragment.oof_candidates = deferred;
}

/// Lay out a table wrapper and its caption/table-grid contents.
pub fn table_layout(doc: &Document, table_id: NodeId, space: &ConstraintSpace) -> Fragment {
    let style = &doc.node(table_id).style;
    let writing_direction = space.writing_direction;
    let model = collect_table_model(doc, table_id);
    let (slots, column_count) = place_cells(doc, &model.rows);
    let use_per_cell_collapsed_edges = style.border_collapse == BorderCollapse::Collapse
        && !model.captions.is_empty()
        && slots.first().is_some_and(|first| {
            let first_border = logical_border(&doc.node(first.node_id).style, writing_direction);
            slots.iter().skip(1).any(|slot| {
                let border = logical_border(&doc.node(slot.node_id).style, writing_direction);
                border.left != first_border.left || border.right != first_border.right
            })
        });
    let mut collapsed_cell_outer_edges = BoxStrut::zero();
    let mut collapsed_structural_outer_edges = BoxStrut::zero();
    if style.border_collapse == BorderCollapse::Collapse {
        collapsed_structural_outer_edges = logical_border(style, writing_direction);
        for slot in &slots {
            let cell_border = logical_border(&doc.node(slot.node_id).style, writing_direction);
            if slot.column == 0 {
                collapsed_cell_outer_edges.left =
                    collapsed_cell_outer_edges.left.max_of(cell_border.left);
            }
            if slot.column + slot.column_span == column_count {
                collapsed_cell_outer_edges.right =
                    collapsed_cell_outer_edges.right.max_of(cell_border.right);
            }
            if slot.row == 0 {
                collapsed_cell_outer_edges.top =
                    collapsed_cell_outer_edges.top.max_of(cell_border.top);
            }
            if slot.row + slot.row_span == model.rows.len() {
                collapsed_cell_outer_edges.bottom =
                    collapsed_cell_outer_edges.bottom.max_of(cell_border.bottom);
            }
        }
        for (index, row) in model.rows.iter().enumerate() {
            if !row.anonymous {
                let row_border = logical_border(&doc.node(row.node_id).style, writing_direction);
                collapsed_structural_outer_edges.left = collapsed_structural_outer_edges
                    .left
                    .max_of(row_border.left);
                collapsed_structural_outer_edges.right = collapsed_structural_outer_edges
                    .right
                    .max_of(row_border.right);
                if index == 0 {
                    collapsed_structural_outer_edges.top =
                        collapsed_structural_outer_edges.top.max_of(row_border.top);
                }
                if index + 1 == model.rows.len() {
                    collapsed_structural_outer_edges.bottom = collapsed_structural_outer_edges
                        .bottom
                        .max_of(row_border.bottom);
                }
            }
            if let Some(group_id) = row.group_id {
                let group_border = logical_border(&doc.node(group_id).style, writing_direction);
                collapsed_structural_outer_edges.left = collapsed_structural_outer_edges
                    .left
                    .max_of(group_border.left);
                collapsed_structural_outer_edges.right = collapsed_structural_outer_edges
                    .right
                    .max_of(group_border.right);
                if index == 0 {
                    collapsed_structural_outer_edges.top = collapsed_structural_outer_edges
                        .top
                        .max_of(group_border.top);
                }
                if index + 1 == model.rows.len() {
                    collapsed_structural_outer_edges.bottom = collapsed_structural_outer_edges
                        .bottom
                        .max_of(group_border.bottom);
                }
            }
        }
    }
    let mut collapsed_structural_outer_excess = BoxStrut {
        top: (collapsed_structural_outer_edges.top - collapsed_cell_outer_edges.top)
            .clamp_negative_to_zero(),
        right: (collapsed_structural_outer_edges.right - collapsed_cell_outer_edges.right)
            .clamp_negative_to_zero(),
        bottom: (collapsed_structural_outer_edges.bottom - collapsed_cell_outer_edges.bottom)
            .clamp_negative_to_zero(),
        left: (collapsed_structural_outer_edges.left - collapsed_cell_outer_edges.left)
            .clamp_negative_to_zero(),
    };
    let structural_perimeter = single_cell_collapsed_structural_perimeter(
        doc,
        table_id,
        &model,
        &slots,
        column_count,
        writing_direction,
    );
    if structural_perimeter.is_some() {
        // The single-cell structural-perimeter path already projects the
        // winning row/group/column edge into the table grid geometry.
        collapsed_structural_outer_excess = BoxStrut::zero();
    }
    let collapsed_structural_inline_track_reduction =
        collapsed_structural_outer_excess.inline_sum() / LayoutUnit::from_i32(2);
    let tracks = measure_tracks(
        doc,
        table_id,
        &model,
        &slots,
        column_count,
        writing_direction,
    );
    let (inline_spacing, block_spacing) =
        resolved_spacing(doc, table_id, space.percentage_resolution_inline_size);
    let grid_width = resolve_table_grid_width(
        doc,
        table_id,
        space,
        &tracks,
        column_count,
        inline_spacing,
        writing_direction,
    );
    let track_grid_width =
        (grid_width - collapsed_structural_inline_track_reduction).clamp_negative_to_zero();
    let track_widths = if style.table_layout == TableLayout::Fixed
        && (authored_inline_size(
            doc,
            table_id,
            space.percentage_resolution_inline_size,
            writing_direction,
        )
        .is_some()
            || space.is_fixed_inline_size
            || space.stretch_inline_size
            || logical_inline_length(style, writing_direction).is_stretch())
    {
        distribute_fixed_tracks(
            doc,
            &model,
            &slots,
            column_count,
            track_grid_width,
            writing_direction,
        )
    } else {
        distribute_auto_tracks(&tracks, track_grid_width)
    };
    let actual_grid_width = if track_widths.is_empty() {
        grid_width
    } else {
        track_widths
            .iter()
            .copied()
            .fold(LayoutUnit::zero(), |a, b| a + b)
    };
    let table_inline_is_definite = space.is_fixed_inline_size
        || space.stretch_inline_size
        || logical_inline_length(style, writing_direction).is_stretch()
        || authored_inline_size(
            doc,
            table_id,
            space.percentage_resolution_inline_size,
            writing_direction,
        )
        .is_some();
    let mut collapsed_inline_boundaries = vec![LayoutUnit::zero(); column_count + 1];
    if style.border_collapse == BorderCollapse::Collapse {
        let table_border = logical_border(style, writing_direction);
        if let Some(first) = collapsed_inline_boundaries.first_mut() {
            *first = table_border.left;
        }
        if let Some(last) = collapsed_inline_boundaries.last_mut() {
            *last = table_border.right;
        }
        for slot in &slots {
            let cell_border = logical_border(&doc.node(slot.node_id).style, writing_direction);
            collapsed_inline_boundaries[slot.column] =
                collapsed_inline_boundaries[slot.column].max_of(cell_border.left);
            let inline_end = slot.column + slot.column_span;
            collapsed_inline_boundaries[inline_end] =
                collapsed_inline_boundaries[inline_end].max_of(cell_border.right);
        }
    }
    let mut laid_cells = Vec::with_capacity(slots.len());
    for slot in slots.iter().copied() {
        let mut width = track_widths[slot.column..slot.column + slot.column_span]
            .iter()
            .copied()
            .fold(LayoutUnit::zero(), |a, b| a + b)
            + inline_spacing * LayoutUnit::from_i32(slot.column_span.saturating_sub(1) as i32);
        if style.border_collapse == BorderCollapse::Collapse && !table_inline_is_definite {
            if slot.column > 0 {
                width = width + collapsed_inline_boundaries[slot.column] / LayoutUnit::from_i32(2);
            }
            let inline_end = slot.column + slot.column_span;
            if inline_end < column_count {
                let boundary = collapsed_inline_boundaries[inline_end];
                width = width + boundary - boundary / LayoutUnit::from_i32(2);
            }
        }
        // Table-cell percentage block sizes are indefinite during the
        // intrinsic/natural row-height pass. They are resolved in a second
        // pass once a definite table grid has distributed its height.
        laid_cells.push(layout_cell(doc, slot, width, space, INDEFINITE_SIZE));
    }

    if style.border_collapse == BorderCollapse::Collapse {
        let table_border = logical_border(style, writing_direction);
        for cell in &mut laid_cells {
            let cell_style = &doc.node(cell.slot.node_id).style;
            let cell_border = logical_border(cell_style, writing_direction);
            if cell.slot.column == 0
                && collapsed_structural_border_wins(
                    table_border.left,
                    style.border_left_style,
                    cell_border.left,
                    cell_style.border_left_style,
                )
            {
                cell.fragment.border.left = LayoutUnit::zero();
            }
            if cell.slot.column + cell.slot.column_span == column_count
                && collapsed_structural_border_wins(
                    table_border.right,
                    style.border_right_style,
                    cell_border.right,
                    cell_style.border_right_style,
                )
            {
                cell.fragment.border.right = LayoutUnit::zero();
            }
            if cell.slot.row == 0
                && collapsed_structural_border_wins(
                    table_border.top,
                    style.border_top_style,
                    cell_border.top,
                    cell_style.border_top_style,
                )
            {
                cell.fragment.border.top = LayoutUnit::zero();
            }
            if cell.slot.row + cell.slot.row_span == model.rows.len()
                && collapsed_structural_border_wins(
                    table_border.bottom,
                    style.border_bottom_style,
                    cell_border.bottom,
                    cell_style.border_bottom_style,
                )
            {
                cell.fragment.border.bottom = LayoutUnit::zero();
            }
            let row = &model.rows[cell.slot.row];
            if !row.anonymous {
                let row_style = &doc.node(row.node_id).style;
                let row_border = logical_border(row_style, writing_direction);
                if collapsed_structural_border_wins(
                    row_border.top,
                    row_style.border_top_style,
                    cell_border.top,
                    cell_style.border_top_style,
                ) {
                    cell.fragment.border.top = LayoutUnit::zero();
                }
                if cell.slot.row_span == 1
                    && collapsed_structural_border_wins(
                        row_border.bottom,
                        row_style.border_bottom_style,
                        cell_border.bottom,
                        cell_style.border_bottom_style,
                    )
                {
                    cell.fragment.border.bottom = LayoutUnit::zero();
                }
            }
        }

        // Resolve shared cell-border ownership before fragments are nested in
        // rows. Equal-strength ties belong to the cell on the logical
        // block/inline-start side; otherwise the wider edge wins. Clearing
        // the losing fragment edge prevents later DOM paint from covering
        // the already-painted winning collapsed border.
        for first in 0..laid_cells.len() {
            for second in first + 1..laid_cells.len() {
                let (leading, trailing) = laid_cells.split_at_mut(second);
                let a = &mut leading[first];
                let b = &mut trailing[0];
                if !use_per_cell_collapsed_edges
                    && authored_borders_match(
                        &doc.node(a.slot.node_id).style,
                        &doc.node(b.slot.node_id).style,
                    )
                {
                    continue;
                }
                let columns_overlap = a.slot.column < b.slot.column + b.slot.column_span
                    && b.slot.column < a.slot.column + a.slot.column_span;
                if columns_overlap && a.slot.row + a.slot.row_span == b.slot.row {
                    if a.fragment.border.bottom >= b.fragment.border.top {
                        b.fragment.border.top = LayoutUnit::zero();
                    } else {
                        a.fragment.border.bottom = LayoutUnit::zero();
                    }
                }
                let rows_overlap = a.slot.row < b.slot.row + b.slot.row_span
                    && b.slot.row < a.slot.row + a.slot.row_span;
                if rows_overlap && a.slot.column + a.slot.column_span == b.slot.column {
                    if a.fragment.border.right >= b.fragment.border.left {
                        b.fragment.border.left = LayoutUnit::zero();
                    } else {
                        a.fragment.border.right = LayoutUnit::zero();
                    }
                }
            }
        }
    }

    // Collapsed grid lines occupy one shared strip centered on their track
    // boundary. Track intrinsic sizing already assigns half of each internal
    // inline line to either neighboring column; keep the winning authored
    // widths here so cell border boxes can overlap on that center line. Block
    // tracks need the analogous half-line subtraction below.
    let mut collapsed_block_boundaries = vec![LayoutUnit::zero(); model.rows.len() + 1];
    let mut collapsed_block_before_present = vec![false; model.rows.len() + 1];
    let mut collapsed_block_after_present = vec![false; model.rows.len() + 1];
    if style.border_collapse == BorderCollapse::Collapse {
        let table_border = logical_border(style, writing_direction);
        if let Some(first) = collapsed_block_boundaries.first_mut() {
            *first = table_border.top;
        }
        if let Some(last) = collapsed_block_boundaries.last_mut() {
            *last = table_border.bottom;
        }
        for slot in &slots {
            let cell_border = logical_border(&doc.node(slot.node_id).style, writing_direction);
            collapsed_block_boundaries[slot.row] =
                collapsed_block_boundaries[slot.row].max_of(cell_border.top);
            collapsed_block_after_present[slot.row] |= cell_border.top > LayoutUnit::zero();
            let block_end = slot.row + slot.row_span;
            collapsed_block_boundaries[block_end] =
                collapsed_block_boundaries[block_end].max_of(cell_border.bottom);
            collapsed_block_before_present[block_end] |= cell_border.bottom > LayoutUnit::zero();
        }
        for (index, row) in model.rows.iter().enumerate() {
            if row.anonymous {
                continue;
            }
            let row_border = logical_border(&doc.node(row.node_id).style, writing_direction);
            if let Some(first) = collapsed_inline_boundaries.first_mut() {
                *first = first.max_of(row_border.left);
            }
            if let Some(last) = collapsed_inline_boundaries.last_mut() {
                *last = last.max_of(row_border.right);
            }
            collapsed_block_boundaries[index] =
                collapsed_block_boundaries[index].max_of(row_border.top);
            collapsed_block_after_present[index] |= row_border.top > LayoutUnit::zero();
            collapsed_block_boundaries[index + 1] =
                collapsed_block_boundaries[index + 1].max_of(row_border.bottom);
            collapsed_block_before_present[index + 1] |= row_border.bottom > LayoutUnit::zero();
        }
    }

    let mut row_heights: Vec<LayoutUnit> = model
        .rows
        .iter()
        .map(|row| {
            minimum_row_height(
                doc,
                row,
                space.percentage_resolution_block_size,
                writing_direction,
            )
        })
        .collect();
    let mut row_baselines = vec![LayoutUnit::zero(); model.rows.len()];
    let mut row_below_baseline = vec![LayoutUnit::zero(); model.rows.len()];
    for cell in &laid_cells {
        if cell.slot.row_span != 1 {
            continue;
        }
        let row = cell.slot.row;
        if matches!(
            doc.node(cell.slot.node_id).style.vertical_align,
            VerticalAlign::Baseline
        ) {
            row_baselines[row] = row_baselines[row].max_of(cell.baseline);
            row_below_baseline[row] =
                row_below_baseline[row].max_of(cell.natural_height - cell.baseline);
        } else {
            row_heights[row] = row_heights[row].max_of(cell.natural_height);
        }
    }
    for row in 0..row_heights.len() {
        row_heights[row] = row_heights[row].max_of(row_baselines[row] + row_below_baseline[row]);
    }
    for cell in &laid_cells {
        let end = cell.slot.row + cell.slot.row_span;
        let used = row_heights[cell.slot.row..end]
            .iter()
            .copied()
            .fold(LayoutUnit::zero(), |a, b| a + b)
            + block_spacing * LayoutUnit::from_i32(cell.slot.row_span.saturating_sub(1) as i32);
        if cell.natural_height > used {
            let extra = LayoutUnit::from_f32(
                (cell.natural_height - used).to_f32() / cell.slot.row_span as f32,
            );
            for height in &mut row_heights[cell.slot.row..end] {
                *height = *height + extra;
            }
        }
    }
    if style.border_collapse == BorderCollapse::Collapse {
        for boundary in 1..row_heights.len() {
            let previous_size = logical_block_length(
                &doc.node(model.rows[boundary - 1].node_id).style,
                writing_direction,
            );
            let next_size = logical_block_length(
                &doc.node(model.rows[boundary].node_id).style,
                writing_direction,
            );
            // A definite row block-size is already the used collapsed-track
            // size. Automatic rows, by contrast, were measured from cell
            // border boxes and therefore contain both halves of their shared
            // line until this adjustment.
            if !previous_size.is_auto() || !next_size.is_auto() {
                continue;
            }
            if !collapsed_block_before_present[boundary] || !collapsed_block_after_present[boundary]
            {
                continue;
            }
            let shared = collapsed_block_boundaries[boundary];
            let before = shared / LayoutUnit::from_i32(2);
            let after = shared - before;
            row_heights[boundary - 1] =
                (row_heights[boundary - 1] - before).clamp_negative_to_zero();
            row_heights[boundary] = (row_heights[boundary] - after).clamp_negative_to_zero();
        }
    }

    // A definite row-group block size is a minimum for the rows it contains,
    // including rows synthesized by table fixup for improper group content.
    // Distribute any group surplus here rather than reading the table's style
    // through an anonymous row node, which would either lose the group height
    // or leak an unrelated table height into the row.
    let mut group_start = 0;
    while group_start < model.rows.len() {
        let Some(group_id) = model.rows[group_start].group_id else {
            group_start += 1;
            continue;
        };
        let mut group_end = group_start + 1;
        while group_end < model.rows.len() && model.rows[group_end].group_id == Some(group_id) {
            group_end += 1;
        }
        let requested = definite_table_internal_block_base(
            doc,
            group_id,
            space.percentage_resolution_block_size,
            writing_direction,
        );
        let spacing =
            block_spacing * LayoutUnit::from_i32(group_end.saturating_sub(group_start + 1) as i32);
        let used = row_heights[group_start..group_end]
            .iter()
            .copied()
            .fold(LayoutUnit::zero(), |sum, height| sum + height)
            + spacing;
        if requested > used {
            let surplus = requested - used;
            let count = (group_end - group_start) as i32;
            let share = LayoutUnit::from_raw(surplus.raw() / count);
            let remainder = surplus.raw() % count;
            for (offset, height) in row_heights[group_start..group_end].iter_mut().enumerate() {
                *height =
                    *height + share + LayoutUnit::from_raw(i32::from((offset as i32) < remainder));
            }
        }
        group_start = group_end;
    }

    let border = logical_border(style, writing_direction);
    let geometry_border = if style.border_collapse == BorderCollapse::Collapse {
        structural_perimeter.unwrap_or_else(BoxStrut::zero)
    } else {
        structural_perimeter.unwrap_or(border)
    };
    let padding = logical_padding(
        style,
        space.percentage_resolution_inline_size,
        writing_direction,
    );
    let margin = logical_margins(
        style,
        space.percentage_resolution_inline_size,
        writing_direction,
    );
    let grid_outer_width = actual_grid_width
        + inline_spacing * LayoutUnit::from_i32((column_count + 1) as i32)
        + collapsed_structural_inline_track_reduction;
    let (caption_min_outer_width, _caption_max_outer_width) = model
        .captions
        .iter()
        .copied()
        .map(|caption_id| {
            let caption_style = &doc.node(caption_id).style;
            let intrinsic = compute_intrinsic_block_sizes(doc, caption_id);
            let caption_border = logical_border(caption_style, writing_direction);
            let caption_padding = logical_padding(
                caption_style,
                space.percentage_resolution_inline_size,
                writing_direction,
            );
            let caption_margin = logical_margins(
                caption_style,
                space.percentage_resolution_inline_size,
                writing_direction,
            );
            let edges = caption_border.inline_sum() + caption_padding.inline_sum();
            let authored_border_box = authored_inline_size(
                doc,
                caption_id,
                space.percentage_resolution_inline_size,
                writing_direction,
            )
            .map(|width| {
                if caption_style.box_sizing == BoxSizing::BorderBox {
                    width
                } else {
                    width + edges
                }
            });
            let minimum = authored_border_box.map_or(intrinsic.min_content_inline_size, |width| {
                intrinsic.min_content_inline_size.max_of(width)
            }) + caption_margin.inline_sum();
            let maximum = authored_border_box.map_or(intrinsic.max_content_inline_size, |width| {
                intrinsic.max_content_inline_size.max_of(width)
            }) + caption_margin.inline_sum();
            (minimum, maximum)
        })
        .fold(
            (LayoutUnit::zero(), LayoutUnit::zero()),
            |(min_width, max_width), (caption_min, caption_max)| {
                (min_width.max_of(caption_min), max_width.max_of(caption_max))
            },
        );
    let collapsed_inline_overhang =
        if style.border_collapse == BorderCollapse::Collapse && table_inline_is_definite {
            (collapsed_inline_boundaries
                .first()
                .copied()
                .unwrap_or_default()
                + collapsed_inline_boundaries
                    .last()
                    .copied()
                    .unwrap_or_default())
                / LayoutUnit::from_i32(2)
        } else {
            LayoutUnit::zero()
        };
    let collapsed_cell_inline_overhang =
        if style.border_collapse == BorderCollapse::Collapse && table_inline_is_definite {
            collapsed_cell_outer_edges.inline_sum() / LayoutUnit::from_i32(2)
        } else {
            LayoutUnit::zero()
        };
    let grid_border_box_width = geometry_border.inline_sum()
        + padding.inline_sum()
        + grid_outer_width
        + collapsed_inline_overhang;
    let border_box_width = if space.is_fixed_inline_size || space.stretch_inline_size {
        grid_border_box_width
    } else {
        // A caption-only table uses CAPMIN plus the table grid's structural
        // edge spacing. Populated auto tables keep their grid preferred width,
        // with CAPMIN acting only as a floor; the caption's max-content width
        // does not stretch the table to the available column measure.
        let caption_floor = if column_count == 0 {
            caption_min_outer_width + grid_border_box_width
        } else {
            caption_min_outer_width
        };
        grid_border_box_width.max_of(caption_floor)
    };

    let mut top_captions = Vec::new();
    let mut bottom_captions = Vec::new();
    for caption_id in model.captions.iter().copied() {
        let caption_style = &doc.node(caption_id).style;
        let direction = caption_style
            .writing_mode
            .to_writing_direction(caption_style.direction);
        let mut caption_space = ConstraintSpace::for_block_child_from_parent(
            space,
            border_box_width,
            INDEFINITE_SIZE,
            border_box_width,
            space.percentage_resolution_block_size,
            true,
            direction,
        );
        caption_space.is_fixed_inline_size =
            authored_inline_size(doc, caption_id, border_box_width, writing_direction).is_none();
        let mut caption = block_layout(doc, caption_id, &caption_space);
        normalize_multicol_child_outer_box(doc, &mut caption, writing_direction);
        if caption_style.caption_side == CaptionSide::Bottom {
            bottom_captions.push(caption);
        } else {
            top_captions.push(caption);
        }
    }
    let mut block_cursor = LayoutUnit::zero();
    let mut children = Vec::new();
    for mut caption in top_captions {
        let caption_margin = logical_margins(
            &doc.node(caption.node_id).style,
            border_box_width,
            writing_direction,
        );
        block_cursor = block_cursor + caption_margin.top;
        caption.offset = PhysicalOffset::new(LayoutUnit::zero(), block_cursor);
        let caption_height = caption.size.height;
        let caption_style = &doc.node(caption.node_id).style;
        crate::relative::apply_relative_offset(
            &mut caption,
            caption_style,
            border_box_width,
            caption_height,
        );
        block_cursor = block_cursor + caption.size.height + caption_margin.bottom;
        children.push(caption);
    }

    // CSS table height applies to the table grid, with captions outside that
    // used size. External fixed/stretch constraints (notably flex sizing)
    // replace the authored size; otherwise height is a minimum and surplus is
    // distributed across rows.
    let grid_edges = geometry_border.block_sum() + padding.block_sum();
    let spacing_count = if row_heights.is_empty() {
        0
    } else {
        row_heights.len() + 1
    };
    let mut natural_grid_height = row_heights
        .iter()
        .copied()
        .fold(LayoutUnit::zero(), |a, b| a + b)
        + block_spacing * LayoutUnit::from_i32(spacing_count as i32)
        + grid_edges
        + collapsed_structural_outer_excess.block_sum();
    let caption_block_size = block_cursor
        + bottom_captions
            .iter()
            .fold(LayoutUnit::zero(), |sum, caption| {
                let margin = logical_margins(
                    &doc.node(caption.node_id).style,
                    border_box_width,
                    writing_direction,
                );
                sum + margin.top + caption.size.height + margin.bottom
            });
    let block_size = logical_block_length(style, writing_direction);
    let min_block_size = logical_min_block_length(style, writing_direction);
    let max_block_size = logical_max_block_length(style, writing_direction);
    let has_definite_min_height = !min_block_size.is_auto()
        && !min_block_size.is_none()
        && !min_block_size.is_content_or_intrinsic();
    let mut requested_grid_height = if (space.is_fixed_block_size || space.stretch_block_size)
        && !space.available_block_size.is_indefinite()
    {
        if style.flex_grow > 0.0 || style.flex_basis.is_auto() {
            Some((space.available_block_size - caption_block_size).clamp_negative_to_zero())
        } else {
            Some(space.available_block_size)
        }
    } else if !block_size.is_auto()
        && !block_size.is_content_or_intrinsic()
        && !block_size.is_stretch()
    {
        let height = resolve_length(
            block_size,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        Some(if style.box_sizing == BoxSizing::ContentBox {
            height + grid_edges
        } else {
            height
        })
    } else {
        style.aspect_ratio.as_ref().and_then(|aspect_ratio| {
            let (inline, block) = if writing_direction.is_horizontal() {
                aspect_ratio.ratio
            } else {
                (aspect_ratio.ratio.1, aspect_ratio.ratio.0)
            };
            (inline > 0.0 && block > 0.0)
                .then(|| LayoutUnit::from_f32(border_box_width.to_f32() * block / inline))
        })
    };
    if has_definite_min_height {
        let minimum = resolve_length(
            min_block_size,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        let minimum = if style.box_sizing == BoxSizing::ContentBox {
            minimum + grid_edges
        } else {
            minimum
        };
        requested_grid_height = Some(requested_grid_height.unwrap_or(minimum).max_of(minimum));
    }
    if !max_block_size.is_none()
        && !max_block_size.is_auto()
        && !max_block_size.is_content_or_intrinsic()
    {
        let maximum = resolve_length(
            max_block_size,
            space.percentage_resolution_block_size,
            LayoutUnit::max(),
            LayoutUnit::max(),
        );
        let maximum = if style.box_sizing == BoxSizing::ContentBox {
            maximum + grid_edges
        } else {
            maximum
        };
        requested_grid_height = Some(requested_grid_height.unwrap_or(maximum).min_of(maximum));
    }
    let table_grid_block_is_definite = requested_grid_height.is_some();
    let requested_grid_height = requested_grid_height.unwrap_or(natural_grid_height);
    if !row_heights.is_empty() && requested_grid_height > natural_grid_height {
        let surplus = requested_grid_height - natural_grid_height;
        // Header and footer groups keep their intrinsic track sizes when a
        // definite table height leaves extra space.  The expandable body row
        // group receives that surplus; this is also what keeps repeated
        // sections stable across page fragments.  Tables without a body
        // group fall back to distributing across every row.
        let mut expandable_rows: Vec<usize> = model
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                let is_body = row.group_id.map_or(true, |group_id| {
                    doc.node(group_id).style.display == Display::TableRowGroup
                });
                is_body.then_some(index)
            })
            .collect();
        if expandable_rows.is_empty() {
            expandable_rows.extend(0..row_heights.len());
        }
        let natural_rows_height = expandable_rows
            .iter()
            .map(|&index| row_heights[index])
            .fold(LayoutUnit::zero(), |sum, height| sum + height);
        let row_count = expandable_rows.len();
        let mut distributed = LayoutUnit::zero();
        let last = row_count - 1;
        for (offset, &index) in expandable_rows.iter().enumerate() {
            let addition = if offset == last {
                surplus - distributed
            } else if natural_rows_height > LayoutUnit::zero() {
                let proportional = LayoutUnit::from_f32(
                    surplus.to_f32() * row_heights[index].to_f32() / natural_rows_height.to_f32(),
                );
                distributed = distributed + proportional;
                proportional
            } else {
                let equal = surplus / LayoutUnit::from_i32(row_count as i32);
                distributed = distributed + equal;
                equal
            };
            row_heights[index] = row_heights[index] + addition;
        }
        natural_grid_height = requested_grid_height;
    }
    let grid_border_box_height = natural_grid_height.max_of(requested_grid_height);

    let grid_top =
        block_cursor + geometry_border.top + padding.top + collapsed_structural_outer_excess.top;
    let mut row_starts = Vec::with_capacity(row_heights.len());
    let mut row_cursor = grid_top
        + if row_heights.is_empty() {
            LayoutUnit::zero()
        } else {
            block_spacing
        };
    for &height in &row_heights {
        row_starts.push(row_cursor);
        row_cursor = row_cursor + height + block_spacing;
    }

    // Table column backgrounds sit below section/row/cell backgrounds.
    let cells_top = grid_top + block_spacing;
    let cells_height = row_heights
        .iter()
        .copied()
        .fold(LayoutUnit::zero(), |a, b| a + b)
        + block_spacing * LayoutUnit::from_i32(model.rows.len().saturating_sub(1) as i32);
    let clip_structural_backgrounds_to_cells = structural_perimeter.is_none()
        && style.border_collapse == BorderCollapse::Separate
        && (inline_spacing > LayoutUnit::zero() || block_spacing > LayoutUnit::zero());
    let grid_border_box_offset = block_cursor;
    if !model.captions.is_empty() {
        // Captions belong to the table wrapper but sit outside the table-grid
        // border/background box. Keep the wrapper as the fragmentation and
        // containing-block owner while painting grid decoration on this
        // structural child at the grid's actual block position.
        let mut grid_decoration = Fragment::new_box(
            table_id,
            PhysicalSize::new(border_box_width, grid_border_box_height),
        );
        grid_decoration.offset = PhysicalOffset::new(LayoutUnit::zero(), block_cursor);
        grid_decoration.border = if style.border_collapse == BorderCollapse::Collapse {
            BoxStrut::zero()
        } else {
            border
        };
        grid_decoration.padding = padding;
        children.push(grid_decoration);
    }
    let mut column_left = geometry_border.left + padding.left + inline_spacing;
    for (index, &column_id) in model.columns.iter().take(column_count).enumerate() {
        let mut column = if structural_perimeter.is_some() {
            Fragment::new_box(
                column_id,
                PhysicalSize::new(grid_border_box_width, grid_border_box_height),
            )
        } else {
            Fragment::new_box(
                column_id,
                PhysicalSize::new(track_widths[index], cells_height),
            )
        };
        column.border = if style.border_collapse == BorderCollapse::Collapse {
            logical_border(&doc.node(column_id).style, writing_direction)
        } else {
            BoxStrut::zero()
        };
        column.padding = logical_padding(
            &doc.node(column_id).style,
            grid_outer_width,
            writing_direction,
        );
        column.offset = if structural_perimeter.is_some() {
            PhysicalOffset::new(LayoutUnit::zero(), block_cursor)
        } else {
            PhysicalOffset::new(column_left, cells_top)
        };
        if clip_structural_backgrounds_to_cells {
            column.decoration_clip_rects = row_starts
                .iter()
                .copied()
                .zip(row_heights.iter().copied())
                .map(|(row_top, row_height)| {
                    PhysicalRect::new(
                        PhysicalOffset::new(LayoutUnit::zero(), row_top - cells_top),
                        PhysicalSize::new(track_widths[index], row_height),
                    )
                })
                .collect();
        }
        column_left = column_left + track_widths[index] + inline_spacing;
        children.push(column);
    }

    let table_content_left =
        if structural_perimeter.is_some() || style.border_collapse == BorderCollapse::Collapse {
            LayoutUnit::zero()
        } else {
            border.left + padding.left
        };
    let mut row_cells: Vec<Vec<Fragment>> = vec![Vec::new(); model.rows.len()];
    for mut cell in laid_cells {
        let collapsed_cell_border = (style.border_collapse == BorderCollapse::Collapse)
            .then(|| logical_border(&doc.node(cell.slot.node_id).style, writing_direction));
        let span_end = cell.slot.row + cell.slot.row_span;
        let span_height = row_heights[cell.slot.row..span_end]
            .iter()
            .copied()
            .fold(LayoutUnit::zero(), |a, b| a + b)
            + block_spacing * LayoutUnit::from_i32(cell.slot.row_span.saturating_sub(1) as i32);
        if table_grid_block_is_definite
            && subtree_uses_percentage_block_size(doc, cell.slot.node_id, writing_direction)
        {
            // Percentage block sizes inside a cell are indefinite during the
            // table's intrinsic pass. Once a definite table height has been
            // distributed to rows, lay out that cell again using its final
            // spanned row height as the percentage basis.
            let mut percentage_space = space.clone();
            percentage_space.percentage_resolution_block_size = span_height;
            percentage_space.available_block_size = span_height;
            cell = layout_cell(
                doc,
                cell.slot,
                cell.fragment.size.width,
                &percentage_space,
                span_height,
            );
        }
        let spare = (span_height - cell.alignment_height).clamp_negative_to_zero();
        let align_offset = match doc.node(cell.slot.node_id).style.vertical_align {
            VerticalAlign::Middle => spare / 2,
            VerticalAlign::Bottom => spare,
            VerticalAlign::Baseline if cell.slot.row_span == 1 => {
                (row_baselines[cell.slot.row] - cell.baseline).clamp_negative_to_zero()
            }
            _ => LayoutUnit::zero(),
        };
        shift_children(&mut cell.fragment, align_offset);
        cell.fragment.table_row_span = cell.slot.row_span;
        let collapsed_inline_start_share =
            if collapsed_cell_border.is_some() && !table_inline_is_definite && cell.slot.column > 0
            {
                collapsed_inline_boundary_for_cell(
                    doc,
                    &slots,
                    cell.slot,
                    cell.slot.column,
                    writing_direction,
                ) / LayoutUnit::from_i32(2)
            } else {
                LayoutUnit::zero()
            };
        let collapsed_block_start_share = if collapsed_cell_border.is_some()
            && cell.slot.row > 0
            && collapsed_block_before_present[cell.slot.row]
            && collapsed_block_after_present[cell.slot.row]
            && logical_block_length(
                &doc.node(model.rows[cell.slot.row - 1].node_id).style,
                writing_direction,
            )
            .is_auto()
            && logical_block_length(
                &doc.node(model.rows[cell.slot.row].node_id).style,
                writing_direction,
            )
            .is_auto()
        {
            collapsed_block_boundaries[cell.slot.row] / LayoutUnit::from_i32(2)
        } else {
            LayoutUnit::zero()
        };
        let cell_block_end = cell.slot.row + cell.slot.row_span;
        let collapsed_block_end_share = if collapsed_cell_border.is_some()
            && cell_block_end < model.rows.len()
            && collapsed_block_before_present[cell_block_end]
            && collapsed_block_after_present[cell_block_end]
            && logical_block_length(
                &doc.node(model.rows[cell_block_end - 1].node_id).style,
                writing_direction,
            )
            .is_auto()
            && logical_block_length(
                &doc.node(model.rows[cell_block_end].node_id).style,
                writing_direction,
            )
            .is_auto()
        {
            let boundary = collapsed_block_boundaries[cell_block_end];
            boundary - boundary / LayoutUnit::from_i32(2)
        } else {
            LayoutUnit::zero()
        };
        let legacy_definite_block_overhang =
            collapsed_cell_border.map_or(LayoutUnit::zero(), |border| {
                if collapsed_block_start_share == LayoutUnit::zero()
                    && collapsed_block_end_share == LayoutUnit::zero()
                    && span_height > cell.natural_height
                {
                    if !use_per_cell_collapsed_edges {
                        (border.top + border.bottom) / LayoutUnit::from_i32(2)
                    } else {
                        border.bottom / LayoutUnit::from_i32(2)
                    }
                } else {
                    LayoutUnit::zero()
                }
            });
        cell.fragment.size.height = span_height
            + collapsed_block_start_share
            + collapsed_block_end_share
            + legacy_definite_block_overhang;
        if let Some(cell_border) = collapsed_cell_border {
            if table_inline_is_definite {
                // Collapsed borders are centered on this cell's grid edges.
                // The painter draws inside a fragment border box, so project
                // the cell's own half-edges into that box instead of applying
                // the table-wide maximum to every row.
                cell.fragment.size.width = cell.fragment.size.width
                    + if !use_per_cell_collapsed_edges {
                        collapsed_cell_inline_overhang
                    } else {
                        (cell_border.left + cell_border.right) / LayoutUnit::from_i32(2)
                    };
            }
            let clip_left = if cell.fragment.border.left == LayoutUnit::zero()
                && cell_border.left > LayoutUnit::zero()
            {
                cell_border.left
            } else {
                LayoutUnit::zero()
            };
            let clip_top = if cell.fragment.border.top == LayoutUnit::zero()
                && cell_border.top > LayoutUnit::zero()
            {
                cell_border.top
            } else {
                LayoutUnit::zero()
            };
            let clip_right = if cell.fragment.border.right == LayoutUnit::zero()
                && cell_border.right > LayoutUnit::zero()
            {
                cell_border.right
            } else {
                LayoutUnit::zero()
            };
            let clip_bottom = if cell.fragment.border.bottom == LayoutUnit::zero()
                && cell_border.bottom > LayoutUnit::zero()
            {
                cell_border.bottom
            } else {
                LayoutUnit::zero()
            };
            if clip_left > LayoutUnit::zero()
                || clip_top > LayoutUnit::zero()
                || clip_right > LayoutUnit::zero()
                || clip_bottom > LayoutUnit::zero()
            {
                cell.fragment.decoration_clip_rects.push(PhysicalRect::new(
                    PhysicalOffset::new(clip_left, clip_top),
                    PhysicalSize::new(
                        (cell.fragment.size.width - clip_left - clip_right)
                            .clamp_negative_to_zero(),
                        (cell.fragment.size.height - clip_top - clip_bottom)
                            .clamp_negative_to_zero(),
                    ),
                ));
            }
            if collapsed_inline_start_share == LayoutUnit::zero()
                && use_per_cell_collapsed_edges
                && (clip_top > LayoutUnit::zero() || clip_bottom > LayoutUnit::zero())
                && (cell.fragment.border.left > LayoutUnit::zero()
                    || cell.fragment.border.right > LayoutUnit::zero())
            {
                // A shared horizontal edge can be owned by the preceding
                // cell, which clips this cell's ordinary decoration along
                // that edge.  The clip must not suppress the perpendicular
                // side borders: those continue through the shared strip and
                // overlay its horizontal winner at the corner.
                cell.fragment
                    .collapsed_border_segments
                    .push(CollapsedBorderSegment {
                        rect: PhysicalRect::new(PhysicalOffset::zero(), cell.fragment.size),
                        // `paint_border_after_children` replaces the ordinary
                        // border pass, so carry the horizontal and inline-end
                        // edges still owned by this cell. Cleared shared edges
                        // remain zero. The inline-start corner is resolved in
                        // its own segment below.
                        border: BoxStrut {
                            top: cell.fragment.border.top,
                            right: cell.fragment.border.right,
                            bottom: cell.fragment.border.bottom,
                            left: LayoutUnit::zero(),
                        },
                    });
                if cell.fragment.border.left > LayoutUnit::zero() {
                    // At an equal-width collapsed corner the preceding
                    // horizontal edge owns the inline-start half; a wider
                    // inline-start edge wins and continues through it.
                    let start_inset =
                        if clip_top > LayoutUnit::zero() && cell.fragment.border.left <= clip_top {
                            clip_top / LayoutUnit::from_i32(2)
                        } else {
                            LayoutUnit::zero()
                        };
                    let end_inset = if clip_bottom > LayoutUnit::zero()
                        && cell.fragment.border.left <= clip_bottom
                    {
                        clip_bottom / LayoutUnit::from_i32(2)
                    } else {
                        LayoutUnit::zero()
                    };
                    cell.fragment
                        .collapsed_border_segments
                        .push(CollapsedBorderSegment {
                            rect: PhysicalRect::new(
                                PhysicalOffset::new(LayoutUnit::zero(), start_inset),
                                PhysicalSize::new(
                                    cell.fragment.size.width,
                                    (cell.fragment.size.height - start_inset - end_inset)
                                        .clamp_negative_to_zero(),
                                ),
                            ),
                            border: BoxStrut {
                                top: LayoutUnit::zero(),
                                right: LayoutUnit::zero(),
                                bottom: LayoutUnit::zero(),
                                left: cell.fragment.border.left,
                            },
                        });
                }
                cell.fragment.paint_border_after_children = true;
            }
            if collapsed_inline_start_share > LayoutUnit::zero()
                && (cell.fragment.border.top > LayoutUnit::zero()
                    || cell.fragment.border.bottom > LayoutUnit::zero())
            {
                cell.fragment
                    .collapsed_border_segments
                    .push(CollapsedBorderSegment {
                        rect: PhysicalRect::new(
                            PhysicalOffset::zero(),
                            PhysicalSize::new(
                                (cell.fragment.size.width
                                    - collapsed_inline_start_share
                                    - clip_right)
                                    .clamp_negative_to_zero(),
                                cell.fragment.size.height,
                            ),
                        ),
                        border: BoxStrut {
                            top: cell.fragment.border.top,
                            right: LayoutUnit::zero(),
                            bottom: cell.fragment.border.bottom,
                            left: LayoutUnit::zero(),
                        },
                    });
                if cell.fragment.border.left > LayoutUnit::zero()
                    || cell.fragment.border.right > LayoutUnit::zero()
                {
                    cell.fragment
                        .collapsed_border_segments
                        .push(CollapsedBorderSegment {
                            rect: PhysicalRect::new(PhysicalOffset::zero(), cell.fragment.size),
                            border: BoxStrut {
                                top: LayoutUnit::zero(),
                                right: cell.fragment.border.right,
                                bottom: LayoutUnit::zero(),
                                left: cell.fragment.border.left,
                            },
                        });
                }
                cell.fragment.paint_border_after_children = true;
            }
        }
        let left = inline_spacing
            + track_widths[..cell.slot.column]
                .iter()
                .copied()
                .fold(LayoutUnit::zero(), |a, b| a + b)
            + inline_spacing * LayoutUnit::from_i32(cell.slot.column as i32);
        cell.fragment.offset = PhysicalOffset::new(
            left + structural_perimeter.map_or(LayoutUnit::zero(), |edge| edge.left)
                + collapsed_structural_outer_excess.left
                + collapsed_cell_border.map_or(LayoutUnit::zero(), |border| {
                    if table_inline_is_definite && use_per_cell_collapsed_edges {
                        (collapsed_cell_outer_edges.left - border.left).clamp_negative_to_zero()
                            / LayoutUnit::from_i32(2)
                    } else {
                        LayoutUnit::zero()
                    }
                })
                - collapsed_inline_start_share,
            structural_perimeter.map_or(LayoutUnit::zero(), |edge| edge.top)
                + if cell.slot.row == 0 {
                    collapsed_structural_outer_excess.top
                } else {
                    LayoutUnit::zero()
                }
                - collapsed_block_start_share,
        );
        let internal_relative_block_base = model.rows[cell.slot.row]
            .group_id
            .map(|group_id| {
                definite_table_internal_block_base(
                    doc,
                    group_id,
                    space.percentage_resolution_block_size,
                    writing_direction,
                )
            })
            .filter(|size| *size > LayoutUnit::zero())
            .or_else(|| {
                let size = definite_table_internal_block_base(
                    doc,
                    model.rows[cell.slot.row].node_id,
                    space.percentage_resolution_block_size,
                    writing_direction,
                );
                (size > LayoutUnit::zero()).then_some(size)
            })
            .unwrap_or_else(|| {
                definite_table_internal_block_base(
                    doc,
                    table_id,
                    space.percentage_resolution_block_size,
                    writing_direction,
                )
            });
        crate::relative::apply_relative_offset(
            &mut cell.fragment,
            &doc.node(cell.slot.node_id).style,
            grid_outer_width,
            // An auto-height table does not establish a definite block-size
            // percentage base for relative insets on internal table boxes.
            // Fixed-length insets still resolve normally against this zero
            // fallback, while percentages compute to the auto/zero offset.
            internal_relative_block_base,
        );
        row_cells[cell.slot.row].push(cell.fragment);
    }

    let mut row_index = 0usize;
    while row_index < model.rows.len() {
        let group_id = model.rows[row_index].group_id;
        let group_start = row_index;
        let mut group_end_index = row_index + 1;
        while group_end_index < model.rows.len() && model.rows[group_end_index].group_id == group_id
        {
            group_end_index += 1;
        }
        let group_top = structural_perimeter.map_or(
            row_starts[group_start]
                - if group_start == 0 {
                    collapsed_structural_outer_excess.top
                } else {
                    LayoutUnit::zero()
                },
            |_| block_cursor,
        );
        let group_bottom = structural_perimeter.map_or(
            row_starts[group_end_index - 1]
                + row_heights[group_end_index - 1]
                + if group_end_index == model.rows.len() {
                    collapsed_structural_outer_excess.bottom
                } else {
                    LayoutUnit::zero()
                },
            |_| block_cursor + grid_border_box_height,
        );
        let mut row_fragments = Vec::new();
        for index in group_start..group_end_index {
            let row_id = model.rows[index].node_id;
            let mut row = Fragment::new_box(
                row_id,
                PhysicalSize::new(
                    if structural_perimeter.is_some()
                        || style.border_collapse == BorderCollapse::Collapse
                    {
                        grid_border_box_width
                    } else {
                        grid_outer_width
                    },
                    if structural_perimeter.is_some() {
                        grid_border_box_height
                    } else {
                        row_heights[index]
                            + if index == 0 {
                                collapsed_structural_outer_excess.top
                            } else {
                                LayoutUnit::zero()
                            }
                            + if index + 1 == model.rows.len() {
                                collapsed_structural_outer_excess.bottom
                            } else {
                                LayoutUnit::zero()
                            }
                    },
                ),
            );
            row.border = if !model.rows[index].anonymous
                && style.border_collapse == BorderCollapse::Collapse
            {
                logical_border(&doc.node(row_id).style, writing_direction)
            } else {
                BoxStrut::zero()
            };
            row.paint_border_after_children = false;
            row.padding = if model.rows[index].anonymous {
                BoxStrut::zero()
            } else {
                logical_padding(&doc.node(row_id).style, grid_outer_width, writing_direction)
            };
            row.offset = PhysicalOffset::new(
                LayoutUnit::zero(),
                if structural_perimeter.is_some() {
                    LayoutUnit::zero()
                } else {
                    row_starts[index]
                        - if index == 0 {
                            collapsed_structural_outer_excess.top
                        } else {
                            LayoutUnit::zero()
                        }
                        - group_top
                },
            );
            row.children = std::mem::take(&mut row_cells[index]);
            if style.border_collapse == BorderCollapse::Collapse
                && !model.rows[index].anonymous
                && writing_direction.is_horizontal()
            {
                let row_style = &doc.node(row_id).style;
                let inset_left = collapsed_border_wins(
                    border.left,
                    style.border_left_style,
                    row.border.left,
                    row_style.border_left_style,
                )
                .then_some(border.left)
                .unwrap_or_default();
                let inset_right = collapsed_border_wins(
                    border.right,
                    style.border_right_style,
                    row.border.right,
                    row_style.border_right_style,
                )
                .then_some(border.right)
                .unwrap_or_default();
                if inset_left > LayoutUnit::zero() || inset_right > LayoutUnit::zero() {
                    row.offset.left = row.offset.left + inset_left;
                    row.size.width =
                        (row.size.width - inset_left - inset_right).clamp_negative_to_zero();
                    row.border.left = LayoutUnit::zero();
                    row.border.right = LayoutUnit::zero();
                    for child in &mut row.children {
                        child.offset.left = child.offset.left - inset_left;
                    }
                }
                let used_row_border = row.border;
                for child in &row.children {
                    let cell_style = &doc.node(child.node_id).style;
                    let cell_border = logical_border(cell_style, writing_direction);
                    let left = child.offset.left.max_of(LayoutUnit::zero());
                    let right = (child.offset.left + child.size.width).min_of(row.size.width);
                    if right <= left {
                        continue;
                    }
                    if collapsed_structural_border_wins(
                        used_row_border.top,
                        row_style.border_top_style,
                        cell_border.top,
                        cell_style.border_top_style,
                    ) {
                        row.collapsed_border_segments.push(CollapsedBorderSegment {
                            rect: PhysicalRect::new(
                                PhysicalOffset::new(left, LayoutUnit::zero()),
                                PhysicalSize::new(right - left, row.size.height),
                            ),
                            border: BoxStrut {
                                top: used_row_border.top,
                                right: LayoutUnit::zero(),
                                bottom: LayoutUnit::zero(),
                                left: LayoutUnit::zero(),
                            },
                        });
                    }
                    if collapsed_structural_border_wins(
                        used_row_border.bottom,
                        row_style.border_bottom_style,
                        cell_border.bottom,
                        cell_style.border_bottom_style,
                    ) {
                        row.collapsed_border_segments.push(CollapsedBorderSegment {
                            rect: PhysicalRect::new(
                                PhysicalOffset::new(left, LayoutUnit::zero()),
                                PhysicalSize::new(right - left, row.size.height),
                            ),
                            border: BoxStrut {
                                top: LayoutUnit::zero(),
                                right: LayoutUnit::zero(),
                                bottom: used_row_border.bottom,
                                left: LayoutUnit::zero(),
                            },
                        });
                    }
                }
                if used_row_border.left > LayoutUnit::zero()
                    || used_row_border.right > LayoutUnit::zero()
                {
                    row.collapsed_border_segments.push(CollapsedBorderSegment {
                        rect: PhysicalRect::new(PhysicalOffset::zero(), row.size),
                        border: BoxStrut {
                            top: LayoutUnit::zero(),
                            right: used_row_border.right,
                            bottom: LayoutUnit::zero(),
                            left: used_row_border.left,
                        },
                    });
                }
                row.paint_border_after_children = !row.collapsed_border_segments.is_empty();
            }
            if clip_structural_backgrounds_to_cells {
                let mut track_left = inline_spacing;
                for &track_width in &track_widths {
                    row.decoration_clip_rects.push(PhysicalRect::new(
                        PhysicalOffset::new(track_left, LayoutUnit::zero()),
                        PhysicalSize::new(track_width, row_heights[index]),
                    ));
                    track_left = track_left + track_width + inline_spacing;
                }
            }
            row.oof_candidates = adopt_child_oof_candidates(&mut row.children);
            row.first_baseline =
                (row_baselines[index] > LayoutUnit::zero()).then_some(row_baselines[index]);
            row.last_baseline = row.first_baseline;
            if !model.rows[index].anonymous {
                crate::relative::apply_relative_offset(
                    &mut row,
                    &doc.node(row_id).style,
                    grid_outer_width,
                    group_id
                        .map(|group_id| {
                            definite_table_internal_block_base(
                                doc,
                                group_id,
                                space.percentage_resolution_block_size,
                                writing_direction,
                            )
                        })
                        .filter(|size| *size > LayoutUnit::zero())
                        .unwrap_or_else(|| {
                            definite_table_internal_block_base(
                                doc,
                                table_id,
                                space.percentage_resolution_block_size,
                                writing_direction,
                            )
                        }),
                );
            }
            resolve_table_box_oof(doc, &mut row);
            row_fragments.push(row);
        }
        if let Some(group_id) = group_id {
            let mut group = Fragment::new_box(
                group_id,
                PhysicalSize::new(
                    if structural_perimeter.is_some()
                        || style.border_collapse == BorderCollapse::Collapse
                    {
                        grid_border_box_width
                    } else {
                        grid_outer_width
                    },
                    group_bottom - group_top,
                ),
            );
            group.border = if style.border_collapse == BorderCollapse::Collapse {
                logical_border(&doc.node(group_id).style, writing_direction)
            } else {
                BoxStrut::zero()
            };
            group.padding = logical_padding(
                &doc.node(group_id).style,
                grid_outer_width,
                writing_direction,
            );
            group.offset = PhysicalOffset::new(table_content_left, group_top);
            group.children = row_fragments;
            if clip_structural_backgrounds_to_cells {
                for row in group_start..group_end_index {
                    let local_top = row_starts[row] - group_top;
                    let mut track_left = inline_spacing;
                    for &track_width in &track_widths {
                        group.decoration_clip_rects.push(PhysicalRect::new(
                            PhysicalOffset::new(track_left, local_top),
                            PhysicalSize::new(track_width, row_heights[row]),
                        ));
                        track_left = track_left + track_width + inline_spacing;
                    }
                }
            }
            group.oof_candidates = adopt_child_oof_candidates(&mut group.children);
            crate::relative::apply_relative_offset(
                &mut group,
                &doc.node(group_id).style,
                grid_outer_width,
                {
                    let own = definite_table_internal_block_base(
                        doc,
                        group_id,
                        space.percentage_resolution_block_size,
                        writing_direction,
                    );
                    if own > LayoutUnit::zero() {
                        own
                    } else {
                        definite_table_internal_block_base(
                            doc,
                            table_id,
                            space.percentage_resolution_block_size,
                            writing_direction,
                        )
                    }
                },
            );
            resolve_table_box_oof(doc, &mut group);
            children.push(group);
        } else {
            for mut row in row_fragments {
                row.offset.left = row.offset.left + table_content_left;
                row.offset.top = row.offset.top + group_top;
                children.push(row);
            }
        }
        row_index = group_end_index;
    }

    block_cursor = block_cursor + grid_border_box_height;
    for mut caption in bottom_captions {
        let caption_margin = logical_margins(
            &doc.node(caption.node_id).style,
            border_box_width,
            writing_direction,
        );
        block_cursor = block_cursor + caption_margin.top;
        caption.offset = PhysicalOffset::new(LayoutUnit::zero(), block_cursor);
        let caption_height = caption.size.height;
        let caption_style = &doc.node(caption.node_id).style;
        crate::relative::apply_relative_offset(
            &mut caption,
            caption_style,
            border_box_width,
            caption_height,
        );
        block_cursor = block_cursor + caption.size.height + caption_margin.bottom;
        children.push(caption);
    }

    let mut table = Fragment::new_box(table_id, PhysicalSize::new(border_box_width, block_cursor));
    // A table with captions has a wrapper box around a separately decorated
    // table grid.  Fragmentation must not reserve the grid's border/padding at
    // the wrapper start, before the top captions; those edges are already
    // represented by `grid_decoration` at the grid's true source position.
    table.border = if model.captions.is_empty() {
        border
    } else {
        BoxStrut::zero()
    };
    table.padding = if model.captions.is_empty() {
        padding
    } else {
        BoxStrut::zero()
    };
    table.margin = margin;
    table.children = children;
    table.skip_box_decoration = !model.captions.is_empty();
    if style.border_collapse == BorderCollapse::Collapse
        && border != BoxStrut::zero()
        && !model.captions.is_empty()
    {
        table.paint_border_after_children = true;
        table
            .collapsed_border_segments
            .push(CollapsedBorderSegment {
                rect: PhysicalRect::new(
                    PhysicalOffset::new(LayoutUnit::zero(), grid_border_box_offset),
                    PhysicalSize::new(border_box_width, grid_border_box_height),
                ),
                border,
            });
    }
    table.oof_candidates = adopt_child_oof_candidates(&mut table.children);
    resolve_table_box_oof(doc, &mut table);
    table.first_baseline = row_baselines
        .first()
        .copied()
        .filter(|baseline| *baseline > LayoutUnit::zero())
        .map(|baseline| grid_top + block_spacing + baseline);
    table.last_baseline = row_baselines
        .last()
        .copied()
        .filter(|baseline| *baseline > LayoutUnit::zero())
        .map(|baseline| row_starts.last().copied().unwrap_or(grid_top) + baseline);
    table.has_overflow_clip = style.overflow_x != openui_style::Overflow::Visible
        || style.overflow_y != openui_style::Overflow::Visible;
    if style.border_collapse == BorderCollapse::Collapse {
        fn suppress_collapsed_table_radii(doc: &Document, fragment: &mut Fragment) {
            if fragment.node_id != NodeId::NONE
                && matches!(
                    doc.node(fragment.node_id).style.display,
                    Display::TableCell
                        | Display::TableRow
                        | Display::TableRowGroup
                        | Display::TableHeaderGroup
                        | Display::TableFooterGroup
                        | Display::TableColumn
                        | Display::TableColumnGroup
                )
            {
                fragment.ignore_border_radius = true;
            }
            for child in &mut fragment.children {
                suppress_collapsed_table_radii(doc, child);
            }
        }
        suppress_collapsed_table_radii(doc, &mut table);
    }
    project_logical_fragment_tree_to_physical(doc, &mut table, writing_direction);
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::ElementTag;
    use openui_geometry::Length;

    #[test]
    fn places_colspan_and_rowspan_without_overlap() {
        let mut doc = Document::new();
        let table = doc.create_node(ElementTag::Table);
        doc.node_mut(table).style.display = Display::Table;
        doc.append_child(doc.root(), table);
        let body = doc.create_node(ElementTag::TableBody);
        doc.node_mut(body).style.display = Display::TableRowGroup;
        doc.append_child(table, body);
        for row_index in 0..2 {
            let row = doc.create_node(ElementTag::TableRow);
            doc.node_mut(row).style.display = Display::TableRow;
            doc.append_child(body, row);
            let cell = doc.create_node(ElementTag::TableCell);
            doc.node_mut(cell).style.display = Display::TableCell;
            if row_index == 0 {
                doc.node_mut(cell).table_row_span = 2;
            }
            doc.append_child(row, cell);
        }
        let model = collect_table_model(&doc, table);
        let (slots, columns) = place_cells(&doc, &model.rows);
        assert_eq!(columns, 2);
        assert_eq!(
            (slots[0].row, slots[0].column, slots[0].row_span),
            (0, 0, 2)
        );
        assert_eq!((slots[1].row, slots[1].column), (1, 1));
    }

    #[test]
    fn table_fixup_preserves_cell_runs_and_reorders_only_first_header_and_footer() {
        let mut doc = Document::new();
        let table = doc.create_node(ElementTag::Div);
        doc.node_mut(table).style.display = Display::Table;
        doc.append_child(doc.root(), table);

        let cell_a = doc.create_node(ElementTag::Div);
        doc.node_mut(cell_a).style.display = Display::TableCell;
        doc.append_child(table, cell_a);

        let footer_a = doc.create_node(ElementTag::Div);
        doc.node_mut(footer_a).style.display = Display::TableFooterGroup;
        doc.append_child(table, footer_a);
        let footer_a_text = doc.create_node(ElementTag::Text);
        doc.append_child(footer_a, footer_a_text);

        let cell_b = doc.create_node(ElementTag::Div);
        doc.node_mut(cell_b).style.display = Display::TableCell;
        doc.append_child(table, cell_b);

        let footer_b = doc.create_node(ElementTag::Div);
        doc.node_mut(footer_b).style.display = Display::TableFooterGroup;
        doc.append_child(table, footer_b);
        let footer_b_text = doc.create_node(ElementTag::Text);
        doc.append_child(footer_b, footer_b_text);

        let cell_c = doc.create_node(ElementTag::Div);
        doc.node_mut(cell_c).style.display = Display::TableCell;
        doc.append_child(table, cell_c);

        let header = doc.create_node(ElementTag::Div);
        doc.node_mut(header).style.display = Display::TableHeaderGroup;
        doc.append_child(table, header);
        let header_text = doc.create_node(ElementTag::Text);
        doc.append_child(header, header_text);

        let model = collect_table_model(&doc, table);
        assert_eq!(model.rows.len(), 6);
        assert_eq!(model.rows[0].group_id, Some(header));
        assert_eq!(model.rows[1].cells, vec![cell_a]);
        assert_eq!(model.rows[2].cells, vec![cell_b]);
        assert_eq!(model.rows[3].group_id, Some(footer_b));
        assert_eq!(model.rows[4].cells, vec![cell_c]);
        assert_eq!(model.rows[5].group_id, Some(footer_a));
    }

    #[test]
    fn fixed_layout_distributes_remaining_track_space() {
        let mut doc = Document::new();
        let table = doc.create_node(ElementTag::Table);
        doc.node_mut(table).style.display = Display::Table;
        doc.append_child(doc.root(), table);
        let col = doc.create_node(ElementTag::TableColumn);
        doc.node_mut(col).style.display = Display::TableColumn;
        doc.node_mut(col).style.width = Length::px(40.0);
        doc.append_child(table, col);
        let model = collect_table_model(&doc, table);
        let widths = distribute_fixed_tracks(
            &doc,
            &model,
            &[],
            2,
            LayoutUnit::from_i32(100),
            WritingDirectionMode::horizontal_ltr(),
        );
        assert_eq!(widths[0].to_i32(), 40);
        assert_eq!(widths[1].to_i32(), 60);
    }

    #[test]
    fn auto_track_distribution_preserves_subpixel_remainder() {
        let tracks = vec![
            TrackContribution {
                min: LayoutUnit::from_raw(2049),
                max: LayoutUnit::from_raw(4100),
            },
            TrackContribution {
                min: LayoutUnit::from_raw(2177),
                max: LayoutUnit::from_raw(3203),
            },
        ];
        let target = LayoutUnit::from_raw(7271);
        let widths = distribute_auto_tracks(&tracks, target);

        assert_eq!(
            widths
                .iter()
                .copied()
                .fold(LayoutUnit::zero(), |sum, width| sum + width),
            target
        );
        assert_eq!(
            widths,
            vec![LayoutUnit::from_raw(4079), LayoutUnit::from_raw(3192)]
        );
    }

    #[test]
    fn auto_table_width_honors_unbreakable_cell_min_content() {
        use openui_style::{FontFamily, FontFamilyList};

        let mut doc = Document::new();
        let table = doc.create_node(ElementTag::Div);
        doc.node_mut(table).style.display = Display::Table;
        doc.node_mut(table).style.width = Length::px(40.0);
        doc.append_child(doc.root(), table);
        let row = doc.create_node(ElementTag::Div);
        doc.node_mut(row).style.display = Display::TableRow;
        doc.append_child(table, row);
        let cell = doc.create_node(ElementTag::Div);
        doc.node_mut(cell).style.display = Display::TableCell;
        doc.append_child(row, cell);
        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).style.font_size = 10.0;
        doc.node_mut(text).style.font_family = FontFamilyList {
            families: vec![FontFamily::Named("Ahem".to_string())],
        };
        doc.node_mut(text).text = Some("stretch".to_string());
        doc.append_child(cell, text);

        let intrinsic = compute_table_intrinsic_sizes(&doc, table);
        assert!(intrinsic.min_content_inline_size > LayoutUnit::from_i32(40));
    }
}
