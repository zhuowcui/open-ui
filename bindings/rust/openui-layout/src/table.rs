//! CSS table formatting model.
//!
//! The table algorithm owns structural fixup, track measurement, spanning-cell
//! placement, row baselines, captions, and the layered column/section/row/cell
//! fragment tree.  It intentionally consumes typed DOM/style data only; HTML
//! parsing and CSS grammar remain porter responsibilities.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{
    BoxStrut, LayoutUnit, LengthType, LogicalOffset, LogicalSize, PhysicalOffset, PhysicalRect,
    PhysicalSize, WritingDirectionMode, INDEFINITE_SIZE,
};
use openui_style::{
    BorderCollapse, BoxSizing, CaptionSide, Display, TableLayout, VerticalAlign, Visibility,
};

use crate::block::{
    algorithm_box_from_logical_strut, block_layout, normalize_multicol_child_outer_box,
    project_logical_fragment_tree_to_physical, resolve_border, resolve_margins, resolve_padding,
};
use crate::constraint_space::ConstraintSpace;
use crate::fragment::{Fragment, FragmentBoxOwnership};
use crate::intrinsic_sizing::{
    compute_intrinsic_block_sizes, compute_intrinsic_block_sizes_with_used_border,
    compute_intrinsic_inline_sizes, IntrinsicSizes,
};
use crate::length_resolver::resolve_length;
use crate::out_of_flow::OutOfFlowCandidate;
use crate::resolved_table_borders::{
    canonical_cell_geometry, BorderBoxInput, BorderSourceKind, CanonicalColumnLocation,
    CanonicalRowSize, CollapsedTableBorderData, GridRegion, LogicalBorderCandidates,
    ResolvedCollapsedBorders,
};

#[derive(Clone)]
struct TableRow {
    node_id: NodeId,
    group_id: Option<NodeId>,
    // Identity of the structural section after anonymous table fixup. Direct
    // row runs use their first source node; an intervening structural box
    // ends that run even when both adjacent groups have group_id == None.
    section_id: NodeId,
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

#[derive(Clone, Copy)]
struct ColumnSource {
    node_id: NodeId,
    start: usize,
    span: usize,
}

#[derive(Default)]
struct TableModel {
    captions: Vec<NodeId>,
    columns: Vec<NodeId>,
    column_sources: Vec<ColumnSource>,
    column_group_sources: Vec<ColumnSource>,
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

fn row_owns_authored_element_box(doc: &Document, row: &TableRow) -> bool {
    !row.anonymous
        && doc.node(row.node_id).tag != ElementTag::Text
        && doc.node(row.node_id).style.display == Display::TableRow
}

fn is_authored_row_group(doc: &Document, node_id: NodeId) -> bool {
    doc.node(node_id).tag != ElementTag::Text
        && matches!(
            doc.node(node_id).style.display,
            Display::TableRowGroup | Display::TableHeaderGroup | Display::TableFooterGroup
        )
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
            section_id: group_id,
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
            section_id: group_id,
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
                section_id: group_id,
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

fn collect_columns(doc: &Document, node_id: NodeId, model: &mut TableModel) {
    let node = doc.node(node_id);
    if node.style.display == Display::None {
        return;
    }
    let start = model.columns.len();
    if node.style.display == Display::TableColumn {
        let span = node.table_col_span.max(1) as usize;
        model.columns.extend(std::iter::repeat_n(node_id, span));
        model.column_sources.push(ColumnSource {
            node_id,
            start,
            span,
        });
        return;
    }
    for child in doc.children(node_id) {
        collect_columns(doc, child, model);
    }
    if node.style.display == Display::TableColumnGroup {
        if start == model.columns.len() {
            let span = node.table_col_span.max(1) as usize;
            model.columns.extend(std::iter::repeat_n(node_id, span));
        }
        model.column_group_sources.push(ColumnSource {
            node_id,
            start,
            span: model.columns.len() - start,
        });
    }
}

fn collect_table_model(doc: &Document, table_id: NodeId) -> TableModel {
    let mut model = TableModel::default();
    let mut headers = Vec::new();
    let mut bodies = Vec::new();
    let mut footers = Vec::new();
    let mut anonymous_cells = Vec::new();
    let mut anonymous_section = None;
    let mut saw_header = false;
    let mut saw_footer = false;

    fn flush_anonymous_cells(
        table_id: NodeId,
        bodies: &mut Vec<TableRow>,
        anonymous_cells: &mut Vec<NodeId>,
        section_id: Option<NodeId>,
    ) {
        if anonymous_cells.is_empty() {
            return;
        }
        bodies.push(TableRow {
            node_id: table_id,
            group_id: None,
            section_id: section_id.expect("anonymous cell run has a section identity"),
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
        anonymous_section: &mut Option<NodeId>,
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
                        anonymous_section,
                        saw_header,
                        saw_footer,
                    );
                }
            }
            Display::TableCaption => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells, *anonymous_section);
                *anonymous_section = None;
                model.captions.push(node_id);
            }
            Display::TableColumn | Display::TableColumnGroup => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells, *anonymous_section);
                *anonymous_section = None;
                collect_columns(doc, node_id, model);
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
                                anonymous_section,
                                saw_header,
                                saw_footer,
                            );
                        }
                    }
                }
            }
            Display::TableHeaderGroup => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells, *anonymous_section);
                *anonymous_section = None;
                if *saw_header {
                    collect_group_rows(doc, node_id, bodies, &mut model.captions);
                } else {
                    *saw_header = true;
                    collect_group_rows(doc, node_id, headers, &mut model.captions);
                }
            }
            Display::TableFooterGroup => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells, *anonymous_section);
                *anonymous_section = None;
                if *saw_footer {
                    collect_group_rows(doc, node_id, bodies, &mut model.captions);
                } else {
                    *saw_footer = true;
                    collect_group_rows(doc, node_id, footers, &mut model.captions);
                }
            }
            Display::TableRowGroup => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells, *anonymous_section);
                *anonymous_section = None;
                collect_group_rows(doc, node_id, bodies, &mut model.captions)
            }
            Display::TableRow => {
                flush_anonymous_cells(table_id, bodies, anonymous_cells, *anonymous_section);
                let section_id = *anonymous_section.get_or_insert(node_id);
                let mut cells = Vec::new();
                collect_row_cells(doc, node_id, &mut cells);
                bodies.push(TableRow {
                    node_id,
                    group_id: None,
                    section_id,
                    cells,
                    anonymous: false,
                });
            }
            _ => {
                anonymous_section.get_or_insert(node_id);
                anonymous_cells.push(node_id);
            }
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
            &mut anonymous_section,
            &mut saw_header,
            &mut saw_footer,
        );
    }
    flush_anonymous_cells(
        table_id,
        &mut bodies,
        &mut anonymous_cells,
        anonymous_section,
    );
    model.rows.extend(headers);
    model.rows.extend(bodies);
    model.rows.extend(footers);
    model
}

fn group_end(rows: &[TableRow], row: usize) -> usize {
    let section = rows[row].section_id;
    let mut end = row + 1;
    while end < rows.len() && rows[end].section_id == section {
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
                (cell.table_row_span as usize).min(group_end(rows, row_index) - row_index)
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

fn cell_region(slot: CellSlot) -> GridRegion {
    GridRegion {
        row: slot.row,
        column: slot.column,
        row_span: slot.row_span,
        column_span: slot.column_span,
    }
}

fn resolve_collapsed_grid(
    doc: &Document,
    table_id: NodeId,
    model: &TableModel,
    slots: &[CellSlot],
    column_count: usize,
    direction: WritingDirectionMode,
) -> Option<ResolvedCollapsedBorders> {
    if doc.node(table_id).style.border_collapse != BorderCollapse::Collapse {
        return None;
    }
    let source = |node_id, region, kind, order| BorderBoxInput {
        region,
        borders: LogicalBorderCandidates::from_style(
            node_id,
            &doc.node(node_id).style,
            kind,
            order,
            direction,
        ),
        source_kind: kind,
    };
    let mut order = 0;
    let cells: Vec<_> = slots
        .iter()
        .map(|slot| {
            let region = cell_region(*slot);
            let input = if doc.node(slot.node_id).tag != ElementTag::Text
                && doc.node(slot.node_id).style.display == Display::TableCell
            {
                source(slot.node_id, region, BorderSourceKind::Cell, order)
            } else {
                // The wrapper is the cell border source, not its authored child.
                BorderBoxInput {
                    region,
                    borders: LogicalBorderCandidates::none(order),
                    source_kind: BorderSourceKind::Cell,
                }
            };
            order += 1;
            input
        })
        .collect();
    let rows: Vec<_> = model
        .rows
        .iter()
        .enumerate()
        .filter_map(|(row, data)| {
            let input = row_owns_authored_element_box(doc, data).then(|| {
                source(
                    data.node_id,
                    GridRegion {
                        row,
                        column: 0,
                        row_span: 1,
                        column_span: column_count,
                    },
                    BorderSourceKind::Row,
                    order,
                )
            });
            order += 1;
            input
        })
        .collect();
    let mut sections = Vec::new();
    let mut start = 0;
    while start < model.rows.len() {
        let end = group_end(&model.rows, start);
        if let Some(group) = model.rows[start]
            .group_id
            .filter(|id| is_authored_row_group(doc, *id))
        {
            sections.push(source(
                group,
                GridRegion {
                    row: start,
                    column: 0,
                    row_span: end - start,
                    column_span: column_count,
                },
                BorderSourceKind::Section,
                order,
            ));
        }
        order += 1;
        start = end;
    }
    let mut columns = Vec::new();
    order += 1;
    for column in &model.column_sources {
        // Pinned VisitCol merges each generated column of a spanned col.
        for index in column.start..column.start + column.span {
            columns.push(source(
                column.node_id,
                GridRegion {
                    row: 0,
                    column: index,
                    row_span: model.rows.len(),
                    column_span: 1,
                },
                BorderSourceKind::Column,
                order,
            ));
        }
    }
    let mut groups = Vec::new();
    order += 1;
    for group in &model.column_group_sources {
        groups.push(source(
            group.node_id,
            GridRegion {
                row: 0,
                column: group.start,
                row_span: model.rows.len(),
                column_span: group.span,
            },
            BorderSourceKind::ColumnGroup,
            order,
        ));
        order += 1;
    }
    let table = source(
        table_id,
        GridRegion {
            row: 0,
            column: 0,
            row_span: model.rows.len(),
            column_span: column_count,
        },
        BorderSourceKind::Table,
        order,
    );
    ResolvedCollapsedBorders::build(
        model.rows.len(),
        column_count,
        &cells,
        &rows,
        &sections,
        &columns,
        &groups,
        Some(table),
    )
}

fn cell_used_border(
    resolved: Option<&ResolvedCollapsedBorders>,
    slot: CellSlot,
    direction: WritingDirectionMode,
) -> Option<BoxStrut> {
    resolved.map(|grid| {
        grid.used_cell_border(cell_region(slot))
            .to_physical(direction)
    })
}

fn measure_cell_intrinsic_sizes(
    doc: &Document,
    slot: CellSlot,
    used_border: Option<BoxStrut>,
) -> IntrinsicSizes {
    let style = &doc.node(slot.node_id).style;
    let authored_cell =
        doc.node(slot.node_id).tag != ElementTag::Text && style.display == Display::TableCell;
    let mut sizes = if let Some(border) = used_border.filter(|_| authored_cell) {
        compute_intrinsic_block_sizes_with_used_border(doc, slot.node_id, border)
    } else if used_border.is_some() && is_authored_row_group(doc, slot.node_id) {
        compute_intrinsic_block_sizes_with_used_border(doc, slot.node_id, BoxStrut::zero())
    } else {
        compute_intrinsic_block_sizes(doc, slot.node_id)
    };
    if doc.node(slot.node_id).tag == ElementTag::Text {
        let inline = compute_intrinsic_inline_sizes(doc, slot.node_id);
        sizes.min_content_inline_size = inline.min;
        sizes.max_content_inline_size = inline.max;
    }
    if let Some(border) = used_border.filter(|_| !authored_cell) {
        sizes.min_content_inline_size = sizes.min_content_inline_size + border.inline_sum();
        sizes.max_content_inline_size = sizes.max_content_inline_size + border.inline_sum();
        sizes.min_content_block_size = sizes.min_content_block_size + border.block_sum();
        sizes.max_content_block_size = sizes.max_content_block_size + border.block_sum();
    }
    sizes
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
    let internal_spacing =
        inline_spacing * LayoutUnit::from_i32((end - start).saturating_sub(1) as i32);
    let min = (min - internal_spacing).clamp_negative_to_zero();
    let max = (max - internal_spacing).clamp_negative_to_zero();
    for track in &mut tracks[start..end] {
        track.max = track.max.max_of(track.min);
    }
    let distributed_min = distribute_auto_tracks(&tracks[start..end], min);
    for (track, distributed) in tracks[start..end].iter_mut().zip(distributed_min) {
        track.min = track.min.max_of(distributed);
        track.max = track.max.max_of(track.min);
    }
    let distributed_max = distribute_auto_tracks(&tracks[start..end], max);
    for (track, distributed) in tracks[start..end].iter_mut().zip(distributed_max) {
        track.max = track.max.max_of(distributed).max_of(track.min);
    }
}

fn measure_tracks(
    doc: &Document,
    table_id: NodeId,
    model: &TableModel,
    slots: &[CellSlot],
    column_count: usize,
    writing_direction: WritingDirectionMode,
    resolved: Option<&ResolvedCollapsedBorders>,
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
    // Blink establishes originating single-column constraints before it
    // redistributes colspan cells over those columns. DOM-order interleaving
    // changes both the proportional shares and which final column receives
    // the fixed-point rounding remainder.
    for slot in slots
        .iter()
        .filter(|slot| slot.column_span == 1)
        .chain(slots.iter().filter(|slot| slot.column_span > 1))
    {
        let used_border = cell_used_border(resolved, *slot, writing_direction);
        let sizes = measure_cell_intrinsic_sizes(doc, *slot, used_border);
        let cell_style = &doc.node(slot.node_id).style;
        let authored_cell = doc.node(slot.node_id).tag != ElementTag::Text
            && cell_style.display == Display::TableCell;
        let own_border = if authored_cell {
            used_border.map_or_else(
                || logical_border(cell_style, writing_direction),
                |border| logical_box_strut(border, writing_direction),
            )
        } else if used_border.is_some() && is_authored_row_group(doc, slot.node_id) {
            BoxStrut::zero()
        } else {
            logical_border(cell_style, writing_direction)
        };
        let wrapper_edges = used_border
            .filter(|_| !authored_cell)
            .map_or(LayoutUnit::zero(), |border| {
                logical_box_strut(border, writing_direction).inline_sum()
            });
        let own_edges = own_border.inline_sum()
            + logical_padding(cell_style, LayoutUnit::zero(), writing_direction).inline_sum();
        let specified =
            authored_inline_size(doc, slot.node_id, LayoutUnit::zero(), writing_direction).map(
                |width| {
                    if cell_style.box_sizing == BoxSizing::BorderBox {
                        width.max_of(own_edges) + wrapper_edges
                    } else {
                        width + own_edges + wrapper_edges
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

pub fn compute_table_intrinsic_sizes(doc: &Document, table_id: NodeId) -> IntrinsicSizes {
    let style = &doc.node(table_id).style;
    let writing_direction = style.writing_mode.to_writing_direction(style.direction);
    let model = collect_table_model(doc, table_id);
    let (slots, column_count) = place_cells(doc, &model.rows);
    let column_count = column_count.max(model.columns.len());
    let resolved = resolve_collapsed_grid(
        doc,
        table_id,
        &model,
        &slots,
        column_count,
        writing_direction,
    );
    let tracks = measure_tracks(
        doc,
        table_id,
        &model,
        &slots,
        column_count,
        writing_direction,
        resolved.as_ref(),
    );
    let (inline_spacing, block_spacing) = resolved_spacing(doc, table_id, LayoutUnit::zero());
    let border = resolved.as_ref().map_or_else(
        || logical_border(style, writing_direction),
        |grid| {
            logical_box_strut(
                grid.used_table_border().to_physical(writing_direction),
                writing_direction,
            )
        },
    );
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
        let sizes = measure_cell_intrinsic_sizes(
            doc,
            slot,
            cell_used_border(resolved.as_ref(), slot, writing_direction),
        );
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
    let min_sum = tracks
        .iter()
        .fold(LayoutUnit::zero(), |sum, track| sum + track.min);
    if target <= min_sum {
        return tracks.iter().map(|track| track.min).collect();
    }
    let max_sum = sum_tracks(tracks, true);
    if target == max_sum {
        return tracks.iter().map(|track| track.max).collect();
    }
    let (bases, distributable, total_increase): (Vec<_>, _, _) = if target < max_sum {
        (
            tracks.iter().map(|track| track.min).collect(),
            target - min_sum,
            max_sum - min_sum,
        )
    } else {
        (
            tracks.iter().map(|track| track.max).collect(),
            target - max_sum,
            max_sum,
        )
    };
    let mut rounding_error = distributable;
    let mut widths = Vec::with_capacity(tracks.len());
    for (track, base) in tracks.iter().zip(bases) {
        let increase = if target < max_sum {
            track.max - track.min
        } else {
            track.max
        };
        let delta = if total_increase > LayoutUnit::zero() {
            LayoutUnit::from_f32((distributable * increase.to_f32()) / total_increase.to_f32())
        } else {
            distributable / tracks.len() as i32
        };
        rounding_error = rounding_error - delta;
        widths.push(base + delta);
    }
    // Blink assigns the accumulated LayoutUnit conversion error to the last
    // computed auto column for both the max-content interpolation and the
    // above-max growth stage.
    if let Some(last) = widths.last_mut() {
        *last = *last + rounding_error;
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
    resolved: Option<&ResolvedCollapsedBorders>,
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
                let cell_style = &doc.node(slot.node_id).style;
                let border = cell_used_border(resolved, *slot, writing_direction).map_or_else(
                    || logical_border(cell_style, writing_direction),
                    |border| logical_box_strut(border, writing_direction),
                );
                let width = if cell_style.box_sizing == BoxSizing::ContentBox {
                    width
                        + border.inline_sum()
                        + logical_padding(cell_style, target, writing_direction).inline_sum()
                } else {
                    width.max_of(border.inline_sum())
                };
                let share = width / LayoutUnit::from_i32(slot.column_span as i32);
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
    used_table_border: Option<BoxStrut>,
) -> LayoutUnit {
    let style = &doc.node(table_id).style;
    let border = used_table_border.unwrap_or_else(|| logical_border(style, writing_direction));
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
    used_border: Option<BoxStrut>,
) -> LaidCell {
    let style = &doc.node(slot.node_id).style;
    let direction = style.writing_mode.to_writing_direction(style.direction);
    let authored_cell =
        doc.node(slot.node_id).tag != ElementTag::Text && style.display == Display::TableCell;
    let wrapper_border = used_border
        .filter(|_| !authored_cell)
        .map(|border| logical_box_strut(border, table_space.writing_direction))
        .unwrap_or_else(BoxStrut::zero);
    let content_width = (width - wrapper_border.inline_sum()).clamp_negative_to_zero();
    let mut cell_space = ConstraintSpace::for_block_child_from_parent(
        table_space,
        content_width,
        INDEFINITE_SIZE,
        content_width,
        percentage_block_size,
        true,
        direction,
    );
    if direction.is_horizontal() == table_space.writing_direction.is_horizontal() {
        cell_space.is_fixed_inline_size = true;
    } else {
        cell_space.is_fixed_block_size = true;
    }
    if let Some(border) = used_border.filter(|_| authored_cell) {
        cell_space.element_used_border = Some((slot.node_id, border));
    } else if used_border.is_some() && is_authored_row_group(doc, slot.node_id) {
        // Anonymous row-group content carriers must not repeat the group's
        // authored outer border, which belongs to the structural source.
        cell_space.element_used_border = Some((slot.node_id, BoxStrut::zero()));
    }
    if authored_cell && !percentage_block_size.is_indefinite() {
        // The table's second cell pass owns the final spanned row size. Make
        // that a definite cell block constraint so percentage-sized
        // descendants resolve through the table cell even though CSS treats
        // the cell's authored `height` as a minimum during its natural pass.
        if direction.is_horizontal() == table_space.writing_direction.is_horizontal() {
            cell_space.available_block_size = percentage_block_size;
            cell_space.is_fixed_block_size = true;
        } else {
            cell_space.available_inline_size = percentage_block_size;
            cell_space.is_fixed_inline_size = true;
        }
    }
    cell_space.needs_first_baseline = true;
    cell_space.needs_last_baseline = true;
    let mut fragment = if doc.node(slot.node_id).tag == ElementTag::Text {
        // Text directly inside a CSS table is wrapped by an anonymous cell.
        // Format that one-node inline sequence through a detached anonymous
        // block, then remap its text fragment back to the source node so paint
        // and hit testing retain DOM identity.
        let mut anonymous_doc = Document::new_with_raster_configuration(
            doc.font_collection().clone(),
            doc.raster_configuration(),
            doc.device_scale_factor(),
        );
        let wrapper = anonymous_doc.create_node(ElementTag::Div);
        let mut wrapper_style = openui_style::ComputedStyleBuilder::from(style);
        wrapper_style.display = Display::Block;
        anonymous_doc.install_resolved_style(wrapper, wrapper_style.build());
        anonymous_doc.append_child(anonymous_doc.root(), wrapper);
        let text = anonymous_doc.create_node(ElementTag::Text);
        anonymous_doc.install_resolved_style(text, style.clone());
        anonymous_doc.node_mut(text).text = doc.node(slot.node_id).text.clone();
        anonymous_doc.append_child(wrapper, text);
        if used_border.is_some() {
            cell_space.element_used_border = Some((wrapper, BoxStrut::zero()));
        }
        let mut fragment = block_layout(&anonymous_doc, wrapper, &cell_space);
        fn remap(fragment: &mut Fragment, wrapper: NodeId, text: NodeId, source: NodeId) {
            if fragment.node_id == wrapper {
                fragment.node_id = NodeId::NONE;
                fragment.box_ownership = FragmentBoxOwnership::LayoutHelper;
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
    // Improper row-group contents are laid out once inside an anonymous
    // cell. This content carrier reuses the group's style identity; the
    // enclosing authored group owns its element box and effects.
    if matches!(
        style.display,
        Display::TableRowGroup | Display::TableHeaderGroup | Display::TableFooterGroup
    ) {
        fragment.box_ownership = FragmentBoxOwnership::LayoutHelper;
    }
    normalize_multicol_child_outer_box(doc, &mut fragment, table_space.writing_direction);
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
        fragment.offset = PhysicalOffset::new(
            wrapper_border.left + margins.left,
            wrapper_border.top + margins.top,
        );
        baseline = baseline + wrapper_border.top + margins.top;
        natural_height = wrapper_border.top
            + margins.top
            + natural_height
            + margins.bottom
            + wrapper_border.bottom;
        alignment_height = natural_height;
        let mut anonymous =
            Fragment::new_box(NodeId::NONE, PhysicalSize::new(width, natural_height));
        anonymous.box_ownership = FragmentBoxOwnership::LayoutHelper;
        anonymous.border = wrapper_border;
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
    let column_count = column_count.max(model.columns.len());
    let resolved = resolve_collapsed_grid(
        doc,
        table_id,
        &model,
        &slots,
        column_count,
        writing_direction,
    );
    let used_table_border = resolved.as_ref().map(|grid| {
        logical_box_strut(
            grid.used_table_border().to_physical(writing_direction),
            writing_direction,
        )
    });
    // Geometry is now measured with resolved borders. No finished track or
    // cell result is corrected by subtracting authored shared-border widths.
    let structural_perimeter: Option<BoxStrut> = None;
    let collapsed_structural_outer_excess = BoxStrut::zero();
    let collapsed_structural_inline_track_reduction = LayoutUnit::zero();
    let tracks = measure_tracks(
        doc,
        table_id,
        &model,
        &slots,
        column_count,
        writing_direction,
        resolved.as_ref(),
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
        used_table_border,
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
            resolved.as_ref(),
        )
    } else {
        distribute_auto_tracks(&tracks, track_grid_width)
    };
    let mut track_widths = track_widths;
    let collapsed_columns: Vec<_> = (0..column_count)
        .map(|index| {
            model
                .column_sources
                .iter()
                .chain(&model.column_group_sources)
                .any(|source| {
                    index >= source.start
                        && index < source.start + source.span
                        && doc.node(source.node_id).style.visibility == Visibility::Collapse
                })
        })
        .collect();
    for (width, collapsed) in track_widths.iter_mut().zip(&collapsed_columns) {
        if *collapsed {
            *width = LayoutUnit::zero();
        }
    }
    let actual_grid_width = if track_widths.is_empty() {
        grid_width
    } else {
        track_widths
            .iter()
            .copied()
            .fold(LayoutUnit::zero(), |a, b| a + b)
    };
    let mut laid_cells = Vec::with_capacity(slots.len());
    for slot in slots.iter().copied() {
        let width = track_widths[slot.column..slot.column + slot.column_span]
            .iter()
            .copied()
            .fold(LayoutUnit::zero(), |a, b| a + b)
            + inline_spacing * LayoutUnit::from_i32(slot.column_span.saturating_sub(1) as i32);
        laid_cells.push(layout_cell(
            doc,
            slot,
            width,
            space,
            INDEFINITE_SIZE,
            cell_used_border(resolved.as_ref(), slot, writing_direction),
        ));
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
        let group_end = group_end(&model.rows, group_start);
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

    let border = used_table_border.unwrap_or_else(|| logical_border(style, writing_direction));
    let geometry_border = border;
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
    let grid_border_box_width =
        geometry_border.inline_sum() + padding.inline_sum() + grid_outer_width;
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
    let collapsed_rows: Vec<_> = model
        .rows
        .iter()
        .map(|row| {
            (row_owns_authored_element_box(doc, row)
                && doc.node(row.node_id).style.visibility == Visibility::Collapse)
                || row
                    .group_id
                    .is_some_and(|id| doc.node(id).style.visibility == Visibility::Collapse)
        })
        .collect();
    let mut collapsed_height = LayoutUnit::zero();
    for (height, collapsed) in row_heights.iter_mut().zip(&collapsed_rows) {
        if *collapsed {
            collapsed_height = collapsed_height + *height;
            *height = LayoutUnit::zero();
        }
    }
    let grid_border_box_height = (natural_grid_height.max_of(requested_grid_height)
        - collapsed_height)
        .clamp_negative_to_zero();

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
        grid_decoration.box_ownership = FragmentBoxOwnership::LayoutHelper;
        grid_decoration.collapsed_table_border_participant = resolved.is_some();
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
        column.border = BoxStrut::zero();
        column.collapsed_table_border_participant = resolved.is_some();
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

    let table_content_left = geometry_border.left + padding.left;
    let mut column_cursor = inline_spacing;
    let canonical_columns: Vec<_> = track_widths
        .iter()
        .enumerate()
        .map(|(index, &size)| {
            let column = CanonicalColumnLocation {
                offset: column_cursor,
                size,
                is_collapsed: collapsed_columns[index],
            };
            column_cursor = column_cursor + size + inline_spacing;
            column
        })
        .collect();
    let canonical_rows: Vec<_> = row_heights
        .iter()
        .enumerate()
        .map(|(index, &size)| CanonicalRowSize {
            size,
            is_collapsed: collapsed_rows[index],
        })
        .collect();
    let mut row_cells: Vec<Vec<Fragment>> = vec![Vec::new(); model.rows.len()];
    for mut cell in laid_cells {
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
                cell_used_border(resolved.as_ref(), cell.slot, writing_direction),
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
        let geometry = canonical_cell_geometry(
            cell_region(cell.slot),
            &canonical_columns,
            &canonical_rows,
            inline_spacing,
            block_spacing,
        )
        .expect("placed cell has canonical grid tracks");
        cell.fragment.size = PhysicalSize::new(
            geometry.rect_in_starting_row.size.inline_size,
            geometry.rect_in_starting_row.size.block_size,
        );
        let logical_left = geometry.rect_in_starting_row.offset.inline_offset + inline_spacing;
        let left = if writing_direction.is_horizontal() && writing_direction.is_rtl() {
            actual_grid_width - logical_left - cell.fragment.size.width
        } else {
            logical_left
        };
        cell.fragment.offset = PhysicalOffset::new(left, LayoutUnit::zero());
        cell.fragment.collapsed_table_border_participant = resolved.is_some();
        cell.fragment.is_hidden_for_paint |=
            geometry.hidden_for_paint || collapsed_rows[cell.slot.row];
        if cell.fragment.principal_box_rect.is_some() {
            // The originating cell adopts the assigned border box. Real
            // continuation ownership remains unchanged; fragmentation must
            // subsequently slice this box with its actual continuation.
            cell.fragment.principal_box_rect = Some(PhysicalRect::new(
                PhysicalOffset::zero(),
                cell.fragment.size,
            ));
        }
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
        let group_end_index = group_end(&model.rows, row_index);
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
                    if resolved.is_some() {
                        actual_grid_width
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
            if !row_owns_authored_element_box(doc, &model.rows[index]) {
                row.box_ownership = FragmentBoxOwnership::LayoutHelper;
            }
            row.border = BoxStrut::zero();
            row.collapsed_table_border_participant = resolved.is_some();
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
                    if resolved.is_some() {
                        actual_grid_width
                    } else {
                        grid_outer_width
                    },
                    group_bottom - group_top,
                ),
            );
            if !is_authored_row_group(doc, group_id) {
                group.box_ownership = FragmentBoxOwnership::LayoutHelper;
            }
            group.border = BoxStrut::zero();
            group.collapsed_table_border_participant = resolved.is_some();
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
    table.collapsed_table_border_participant = resolved.is_some();
    if let Some(borders) = resolved {
        table.collapsed_table_border_data = Some(std::sync::Arc::new(CollapsedTableBorderData {
            borders,
            columns: canonical_columns,
            rows: canonical_rows,
            grid_offset: LogicalOffset::new(
                if writing_direction.is_horizontal() && writing_direction.is_rtl() {
                    geometry_border.right + padding.right
                } else {
                    geometry_border.left + padding.left
                },
                grid_top,
            ),
            grid_size: LogicalSize::new(actual_grid_width, cells_height),
            writing_direction,
        }));
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
    use openui_style::BorderStyle;

    fn count_owned_boxes(fragment: &Fragment, node: NodeId) -> usize {
        usize::from(fragment.node_id == node && fragment.owns_element_box())
            + fragment
                .children
                .iter()
                .map(|child| count_owned_boxes(child, node))
                .sum::<usize>()
    }

    fn count_style_helpers(fragment: &Fragment, node: NodeId) -> usize {
        usize::from(
            fragment.node_id == node
                && fragment.box_ownership == FragmentBoxOwnership::LayoutHelper,
        ) + fragment
            .children
            .iter()
            .map(|child| count_style_helpers(child, node))
            .sum::<usize>()
    }

    #[test]
    fn collapsed_cells_use_winning_half_widths_before_track_and_content_sizing() {
        let mut doc = Document::new();
        let table = doc.create_node(ElementTag::Table);
        let row = doc.create_node(ElementTag::TableRow);
        let first = doc.create_node(ElementTag::TableCell);
        let second = doc.create_node(ElementTag::TableCell);
        doc.append_child(doc.root(), table);
        doc.append_child(table, row);
        doc.append_child(row, first);
        doc.append_child(row, second);
        doc.update_resolved_style(table, |style| {
            style.display = Display::Table;
            style.border_collapse = BorderCollapse::Collapse;
        });
        doc.update_resolved_style(row, |style| style.display = Display::TableRow);
        for cell in [first, second] {
            doc.update_resolved_style(cell, |style| {
                style.display = Display::TableCell;
                style.width = Length::px(20.0);
                style.border_left_style = BorderStyle::Solid;
                style.border_right_style = BorderStyle::Solid;
                style.border_left_width = 1;
                style.border_right_width = 1;
            });
        }
        doc.update_resolved_style(second, |style| style.border_left_width = 7);
        let model = collect_table_model(&doc, table);
        let (slots, columns) = place_cells(&doc, &model.rows);
        let direction = WritingDirectionMode::horizontal_ltr();
        let grid = resolve_collapsed_grid(&doc, table, &model, &slots, columns, direction)
            .expect("collapsed borders");
        let first_used = cell_used_border(Some(&grid), slots[0], direction).unwrap();
        assert_eq!(first_used.left, LayoutUnit::from_f32(0.5));
        assert_eq!(first_used.right, LayoutUnit::from_f32(3.5));
        let tracks = measure_tracks(&doc, table, &model, &slots, columns, direction, Some(&grid));
        assert_eq!(tracks[0].min, LayoutUnit::from_i32(24));
        assert_eq!(tracks[1].min, LayoutUnit::from_i32(24));
        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(200), LayoutUnit::from_i32(100));
        let cell = layout_cell(
            &doc,
            slots[0],
            LayoutUnit::from_i32(24),
            &space,
            INDEFINITE_SIZE,
            Some(first_used),
        );
        assert_eq!(cell.fragment.border, first_used);
        assert_eq!(cell.fragment.size.width, LayoutUnit::from_i32(24));
        assert!(cell.fragment.owns_element_box());
    }

    #[test]
    fn positive_and_zero_rowspans_stop_at_their_repaired_section() {
        for authored_span in [0, 9] {
            let mut doc = Document::new();
            let table = doc.create_node(ElementTag::Table);
            doc.update_resolved_style(table, |style| style.display = Display::Table);
            doc.append_child(doc.root(), table);
            let mut cells = Vec::new();
            for row_count in [2, 1] {
                let group = doc.create_node(ElementTag::TableBody);
                doc.update_resolved_style(group, |style| style.display = Display::TableRowGroup);
                doc.append_child(table, group);
                for _ in 0..row_count {
                    let row = doc.create_node(ElementTag::TableRow);
                    let cell = doc.create_node(ElementTag::TableCell);
                    doc.update_resolved_style(row, |style| style.display = Display::TableRow);
                    doc.update_resolved_style(cell, |style| style.display = Display::TableCell);
                    doc.append_child(group, row);
                    doc.append_child(row, cell);
                    cells.push(cell);
                }
            }
            doc.node_mut(cells[0]).table_row_span = authored_span;
            let model = collect_table_model(&doc, table);
            let (slots, _) = place_cells(&doc, &model.rows);
            assert_eq!(slots[0].row_span, 2);
            assert_eq!((slots[2].row, slots[2].column), (2, 0));
            assert_ne!(model.rows[1].section_id, model.rows[2].section_id);
        }
    }

    #[test]
    fn columns_keep_distinct_column_and_group_border_sources() {
        let mut doc = Document::new();
        let table = doc.create_node(ElementTag::Table);
        let group = doc.create_node(ElementTag::TableColumnGroup);
        let column = doc.create_node(ElementTag::TableColumn);
        doc.append_child(table, group);
        doc.append_child(group, column);
        doc.update_resolved_style(table, |style| style.display = Display::Table);
        doc.update_resolved_style(group, |style| style.display = Display::TableColumnGroup);
        doc.update_resolved_style(column, |style| style.display = Display::TableColumn);
        doc.node_mut(column).table_col_span = 3;
        let model = collect_table_model(&doc, table);
        assert_eq!(model.columns, [column, column, column]);
        assert_eq!(model.column_sources.len(), 1);
        assert_eq!(model.column_sources[0].span, 3);
        assert_eq!(model.column_group_sources.len(), 1);
        assert_eq!(model.column_group_sources[0].node_id, group);
        assert_eq!(model.column_group_sources[0].span, 3);
    }

    #[test]
    fn table_grid_and_anonymous_row_helpers_do_not_own_the_table_box() {
        let mut doc = Document::new();
        let table = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(table, |style| style.display = Display::Table);
        doc.append_child(doc.root(), table);
        let caption = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(caption, |style| {
            style.display = Display::TableCaption;
            style.height = Length::px(2.0);
        });
        doc.append_child(table, caption);
        let cell = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(cell, |style| {
            style.display = Display::TableCell;
            style.width = Length::px(8.0);
            style.height = Length::px(6.0);
        });
        doc.append_child(table, cell);

        let space = ConstraintSpace::for_root(LayoutUnit::from_i32(200), LayoutUnit::from_i32(100));
        let fragment = table_layout(&doc, table, &space);
        assert_eq!(count_owned_boxes(&fragment, table), 1);
        assert_eq!(count_style_helpers(&fragment, table), 2);
        assert_eq!(count_owned_boxes(&fragment, caption), 1);
        assert_eq!(count_owned_boxes(&fragment, cell), 1);
    }

    #[test]
    fn anonymous_row_group_content_retains_one_authored_group_box() {
        for display in [
            Display::TableRowGroup,
            Display::TableHeaderGroup,
            Display::TableFooterGroup,
        ] {
            let mut doc = Document::new();
            let table = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(table, |style| style.display = Display::Table);
            doc.append_child(doc.root(), table);
            let group = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(group, |style| style.display = display);
            doc.append_child(table, group);
            let mut authored_children = Vec::new();
            for _ in 0..2 {
                let child = doc.create_node(ElementTag::Div);
                doc.update_resolved_style(child, |style| {
                    style.display = Display::Block;
                    style.width = Length::px(12.0);
                    style.height = Length::px(7.0);
                });
                doc.append_child(group, child);
                authored_children.push(child);
            }

            let space =
                ConstraintSpace::for_root(LayoutUnit::from_i32(200), LayoutUnit::from_i32(100));
            let fragment = table_layout(&doc, table, &space);
            assert_eq!(count_owned_boxes(&fragment, group), 1, "{display:?}");
            assert_eq!(count_style_helpers(&fragment, group), 2, "{display:?}");
            for child in authored_children {
                assert_eq!(count_owned_boxes(&fragment, child), 1, "{display:?}");
            }
        }
    }

    #[test]
    fn authored_table_rows_keep_ownership_when_their_tag_is_generic() {
        let mut doc = Document::new();
        let row_id = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(row_id, |style| style.display = Display::TableRow);
        let row = TableRow {
            node_id: row_id,
            group_id: None,
            section_id: row_id,
            cells: Vec::new(),
            anonymous: false,
        };
        assert!(row_owns_authored_element_box(&doc, &row));
        let mut anonymous = row.clone();
        anonymous.anonymous = true;
        assert!(!row_owns_authored_element_box(&doc, &anonymous));

        doc.update_resolved_style(row_id, |style| style.display = Display::TableRowGroup);
        assert!(!row_owns_authored_element_box(&doc, &row));
        assert!(is_authored_row_group(&doc, row_id));
    }

    #[test]
    fn places_colspan_and_rowspan_without_overlap() {
        let mut doc = Document::new();
        let table = doc.create_node(ElementTag::Table);
        doc.update_resolved_style(table, |style| style.display = Display::Table);
        doc.append_child(doc.root(), table);
        let body = doc.create_node(ElementTag::TableBody);
        doc.update_resolved_style(body, |style| style.display = Display::TableRowGroup);
        doc.append_child(table, body);
        for row_index in 0..2 {
            let row = doc.create_node(ElementTag::TableRow);
            doc.update_resolved_style(row, |style| style.display = Display::TableRow);
            doc.append_child(body, row);
            let cell = doc.create_node(ElementTag::TableCell);
            doc.update_resolved_style(cell, |style| style.display = Display::TableCell);
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
        doc.update_resolved_style(table, |style| style.display = Display::Table);
        doc.append_child(doc.root(), table);

        let cell_a = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(cell_a, |style| style.display = Display::TableCell);
        doc.append_child(table, cell_a);

        let footer_a = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(footer_a, |style| style.display = Display::TableFooterGroup);
        doc.append_child(table, footer_a);
        let footer_a_text = doc.create_node(ElementTag::Text);
        doc.append_child(footer_a, footer_a_text);

        let cell_b = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(cell_b, |style| style.display = Display::TableCell);
        doc.append_child(table, cell_b);

        let footer_b = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(footer_b, |style| style.display = Display::TableFooterGroup);
        doc.append_child(table, footer_b);
        let footer_b_text = doc.create_node(ElementTag::Text);
        doc.append_child(footer_b, footer_b_text);

        let cell_c = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(cell_c, |style| style.display = Display::TableCell);
        doc.append_child(table, cell_c);

        let header = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(header, |style| style.display = Display::TableHeaderGroup);
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
        doc.update_resolved_style(table, |style| style.display = Display::Table);
        doc.append_child(doc.root(), table);
        let col = doc.create_node(ElementTag::TableColumn);
        doc.update_resolved_style(col, |style| style.display = Display::TableColumn);
        doc.update_resolved_style(col, |style| style.width = Length::px(40.0));
        doc.append_child(table, col);
        let model = collect_table_model(&doc, table);
        let widths = distribute_fixed_tracks(
            &doc,
            &model,
            &[],
            2,
            LayoutUnit::from_i32(100),
            WritingDirectionMode::horizontal_ltr(),
            None,
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
            vec![LayoutUnit::from_raw(4078), LayoutUnit::from_raw(3193)]
        );
    }

    #[test]
    fn auto_track_distribution_matches_chromium_max_guess_rounding() {
        let max = [7060, 7060, 7060, 7060, 7060, 7068, 13312];
        let cases = [
            (
                [3072, 3072, 3072, 3072, 3072, 3072, 6144],
                [6026, 6026, 6026, 6026, 6026, 6032, 11454],
            ),
            (
                [3072, 3072, 3072, 3072, 3072, 3072, 7168],
                [5990, 5990, 5990, 5990, 5990, 5996, 11670],
            ),
        ];

        for (min, expected) in cases {
            let tracks = min
                .into_iter()
                .zip(max)
                .map(|(min, max)| TrackContribution {
                    min: LayoutUnit::from_raw(min),
                    max: LayoutUnit::from_raw(max),
                })
                .collect::<Vec<_>>();
            assert_eq!(
                distribute_auto_tracks(&tracks, LayoutUnit::from_raw(47616)),
                expected.map(LayoutUnit::from_raw)
            );
        }
    }

    #[test]
    fn auto_table_width_honors_unbreakable_cell_min_content() {
        use openui_style::{FontFamily, FontFamilyList};

        // The assertion depends on Ahem's fixed advances; use the bundled
        // document-owned face instead of the host's font fallback.
        let mut doc =
            Document::new_with_font_collection(openui_text::FontCollection::deterministic_test());
        let table = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(table, |style| style.display = Display::Table);
        doc.update_resolved_style(table, |style| style.width = Length::px(40.0));
        doc.append_child(doc.root(), table);
        let row = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(row, |style| style.display = Display::TableRow);
        doc.append_child(table, row);
        let cell = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(cell, |style| style.display = Display::TableCell);
        doc.append_child(row, cell);
        let text = doc.create_node(ElementTag::Text);
        doc.update_resolved_style(text, |style| style.font_size = 10.0);
        doc.update_resolved_style(text, |style| {
            style.font_family = FontFamilyList {
                families: vec![FontFamily::Named("Ahem".to_string())],
            }
        });
        doc.node_mut(text).text = Some("stretch".to_string());
        doc.append_child(cell, text);

        let intrinsic = compute_table_intrinsic_sizes(&doc, table);
        assert!(intrinsic.min_content_inline_size > LayoutUnit::from_i32(40));
    }
}
