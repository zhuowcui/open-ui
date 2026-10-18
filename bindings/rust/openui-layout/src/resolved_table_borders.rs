//! Resolved collapsed table edges, separate from a cell's used border strut.
//!
//! Chromium 147.0.7727.50 table_borders.cc:24-47,115-250,335-386,
//! 433-588 is the pinned conflict/half-width arithmetic source. Each segment
//! retains its actual source side for painting; used cell widths scan only
//! that cell's resolved perimeter before intrinsic and final sizing.

use openui_dom::NodeId;
use openui_geometry::{
    LayoutUnit, LogicalBoxStrut, LogicalOffset, LogicalRect, LogicalSize, WritingDirectionMode,
};
use openui_style::{BorderStyle, ComputedStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalBorderSide {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderSourceKind {
    Cell,
    Row,
    Section,
    Column,
    ColumnGroup,
    Table,
}

#[derive(Debug, Clone, Copy)]
pub struct BorderCandidate {
    // A style source, not an additional authored element box. Paint can resolve
    // this side's color from the same node without mutating any cell style.
    pub node_id: NodeId,
    pub physical_side: PhysicalBorderSide,
    pub source_kind: BorderSourceKind,
    pub box_order: usize,
    pub style: BorderStyle,
    pub width: LayoutUnit,
}

impl BorderCandidate {
    fn paintable_width(self) -> LayoutUnit {
        if self.style.has_visible_border() {
            self.width
        } else {
            LayoutUnit::zero()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LogicalBorderCandidates {
    pub inline_start: BorderCandidate,
    pub inline_end: BorderCandidate,
    pub block_start: BorderCandidate,
    pub block_end: BorderCandidate,
}

impl LogicalBorderCandidates {
    pub fn none(box_order: usize) -> Self {
        let candidate = |physical_side| BorderCandidate {
            node_id: NodeId::NONE,
            physical_side,
            source_kind: BorderSourceKind::Cell,
            box_order,
            style: BorderStyle::None,
            width: LayoutUnit::zero(),
        };
        Self {
            inline_start: candidate(PhysicalBorderSide::Left),
            inline_end: candidate(PhysicalBorderSide::Right),
            block_start: candidate(PhysicalBorderSide::Top),
            block_end: candidate(PhysicalBorderSide::Bottom),
        }
    }

    pub fn from_style(
        node_id: NodeId,
        style: &ComputedStyle,
        source_kind: BorderSourceKind,
        box_order: usize,
        table_direction: WritingDirectionMode,
    ) -> Self {
        let candidate = |physical_side| {
            let (border_style, width) = match physical_side {
                PhysicalBorderSide::Top => (style.border_top_style, style.border_top_width),
                PhysicalBorderSide::Right => (style.border_right_style, style.border_right_width),
                PhysicalBorderSide::Bottom => {
                    (style.border_bottom_style, style.border_bottom_width)
                }
                PhysicalBorderSide::Left => (style.border_left_style, style.border_left_width),
            };
            BorderCandidate {
                node_id,
                physical_side,
                source_kind,
                box_order,
                style: collapsed_border_style(border_style),
                width: if border_style.has_visible_border() {
                    LayoutUnit::from_i32(width.max(0))
                } else {
                    LayoutUnit::zero()
                },
            }
        };
        // Same physical/logical mapping as LogicalBoxStrut. A cell's own
        // writing direction must not replace the table direction here.
        let (inline_start, inline_end, block_start, block_end) = if table_direction.is_horizontal()
        {
            let (start, end) = if table_direction.is_rtl() {
                (PhysicalBorderSide::Right, PhysicalBorderSide::Left)
            } else {
                (PhysicalBorderSide::Left, PhysicalBorderSide::Right)
            };
            (
                start,
                end,
                PhysicalBorderSide::Top,
                PhysicalBorderSide::Bottom,
            )
        } else {
            let (start, end) = if table_direction.is_flipped_lines() ^ table_direction.is_rtl() {
                (PhysicalBorderSide::Bottom, PhysicalBorderSide::Top)
            } else {
                (PhysicalBorderSide::Top, PhysicalBorderSide::Bottom)
            };
            let (before, after) = if table_direction.is_flipped_blocks() {
                (PhysicalBorderSide::Right, PhysicalBorderSide::Left)
            } else {
                (PhysicalBorderSide::Left, PhysicalBorderSide::Right)
            };
            (start, end, before, after)
        };
        Self {
            inline_start: candidate(inline_start),
            inline_end: candidate(inline_end),
            block_start: candidate(block_start),
            block_end: candidate(block_end),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridRegion {
    pub row: usize,
    pub column: usize,
    pub row_span: usize,
    pub column_span: usize,
}

impl GridRegion {
    fn clamp(self, rows: usize, columns: usize) -> Option<Self> {
        if self.row >= rows || self.column >= columns {
            return None;
        }
        let row_span = self.row_span.min(rows - self.row);
        let column_span = self.column_span.min(columns - self.column);
        (row_span != 0 && column_span != 0).then_some(Self {
            row_span,
            column_span,
            ..self
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BorderBoxInput {
    pub region: GridRegion,
    pub borders: LogicalBorderCandidates,
    pub source_kind: BorderSourceKind,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ResolvedEdge {
    #[default]
    Empty,
    SpanInterior,
    Winner(BorderCandidate),
}

// Exact .50 computed_style.h:583-594 normalizes these styles before
// collapsed conflict selection and painting, not after comparing priorities.
fn collapsed_border_style(style: BorderStyle) -> BorderStyle {
    match style {
        BorderStyle::Inset => BorderStyle::Ridge,
        BorderStyle::Outset => BorderStyle::Groove,
        _ => style,
    }
}

// Do not compare the native BorderStyle discriminant: its ordering differs
// from collapsed border specificity. This explicit ordering is the current
// collapsed ordering verified by the exact .50 css_properties.json5
// generated enum input at lines 2742-2752 (receipt saved here).
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

impl ResolvedEdge {
    fn merge(&mut self, candidate: BorderCandidate) {
        // table_borders.cc:509 and :534 skip `none` before conflict resolution.
        if candidate.style == BorderStyle::None {
            return;
        }
        let replace = match *self {
            Self::SpanInterior => false,
            Self::Empty => true,
            Self::Winner(current) => {
                // Newly encountered hidden wins even over an existing hidden.
                // A visible candidate cannot replace hidden. Width is compared
                // before style, and equal width/style retains prior traversal.
                candidate.style == BorderStyle::Hidden
                    || (current.style != BorderStyle::Hidden
                        && (candidate.width > current.width
                            || (candidate.width == current.width
                                && style_priority(candidate.style)
                                    > style_priority(current.style))))
            }
        };
        if replace {
            *self = Self::Winner(candidate);
        }
    }

    fn block_if_empty(&mut self) {
        if matches!(self, Self::Empty) {
            *self = Self::SpanInterior;
        }
    }

    pub fn paintable_width(self) -> LayoutUnit {
        match self {
            Self::Winner(candidate) => candidate.paintable_width(),
            Self::Empty | Self::SpanInterior => LayoutUnit::zero(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedCollapsedBorders {
    rows: usize,
    columns: usize,
    // inline_edges[row, boundary_column]; block_edges[boundary_row, column].
    // These remain individual resolved segments: do not reduce them to a
    // global maximum width or to border sides on one winning cell fragment.
    inline_edges: Vec<ResolvedEdge>,
    block_edges: Vec<ResolvedEdge>,
}

impl ResolvedCollapsedBorders {
    pub fn row_count(&self) -> usize {
        self.rows
    }
    pub fn column_count(&self) -> usize {
        self.columns
    }

    pub(crate) fn build(
        rows: usize,
        columns: usize,
        cells: &[BorderBoxInput],
        row_boxes: &[BorderBoxInput],
        sections: &[BorderBoxInput],
        column_boxes: &[BorderBoxInput],
        column_groups: &[BorderBoxInput],
        table: Option<BorderBoxInput>,
    ) -> Option<Self> {
        let inline_count = rows.checked_mul(columns.checked_add(1)?)?;
        let block_count = rows.checked_add(1)?.checked_mul(columns)?;
        let mut result = Self {
            rows,
            columns,
            inline_edges: vec![ResolvedEdge::Empty; inline_count],
            block_edges: vec![ResolvedEdge::Empty; block_count],
        };
        // Caller supplies logical row/cell traversal after header/body/footer
        // grouping; no geometry deduplication or NodeId deduplication is used.
        // Origin precedence comes from this traversal and strict tie retention.
        for source in cells {
            result.merge_box(*source);
        }
        for source in row_boxes {
            result.merge_box(*source);
        }
        for source in sections {
            result.merge_box(*source);
        }
        for source in column_boxes {
            result.merge_box(*source);
        }
        for source in column_groups {
            result.merge_box(*source);
        }
        if let Some(source) = table {
            result.merge_box(source);
        }
        Some(result)
    }

    fn inline_index(&self, row: usize, boundary_column: usize) -> usize {
        row * (self.columns + 1) + boundary_column
    }

    fn block_index(&self, boundary_row: usize, column: usize) -> usize {
        boundary_row * self.columns + column
    }

    fn merge_box(&mut self, source: BorderBoxInput) {
        let Some(region) = source.region.clamp(self.rows, self.columns) else {
            return;
        };
        let row_end = region.row + region.row_span;
        let column_end = region.column + region.column_span;
        for row in region.row..row_end {
            let start = self.inline_index(row, region.column);
            let end = self.inline_index(row, column_end);
            self.inline_edges[start].merge(source.borders.inline_start);
            self.inline_edges[end].merge(source.borders.inline_end);
        }
        for column in region.column..column_end {
            let start = self.block_index(region.row, column);
            let end = self.block_index(row_end, column);
            self.block_edges[start].merge(source.borders.block_start);
            self.block_edges[end].merge(source.borders.block_end);
        }
        if source.source_kind == BorderSourceKind::Cell {
            // Mark each originating cell's interior immediately after its
            // perimeter, as pinned MergeBorders does. Existing winners from
            // malformed overlapping cells are preserved; subsequent structural
            // candidates cannot draw a row/column border through the span.
            for row in region.row..row_end {
                for column in region.column + 1..column_end {
                    let index = self.inline_index(row, column);
                    self.inline_edges[index].block_if_empty();
                }
            }
            for row in region.row + 1..row_end {
                for column in region.column..column_end {
                    let index = self.block_index(row, column);
                    self.block_edges[index].block_if_empty();
                }
            }
        }
    }

    pub fn inline_edge(&self, row: usize, boundary_column: usize) -> ResolvedEdge {
        self.inline_edges[self.inline_index(row, boundary_column)]
    }

    pub fn block_edge(&self, boundary_row: usize, column: usize) -> ResolvedEdge {
        self.block_edges[self.block_index(boundary_row, column)]
    }

    pub fn used_cell_border(&self, region: GridRegion) -> LogicalBoxStrut {
        let Some(region) = region.clamp(self.rows, self.columns) else {
            return LogicalBoxStrut::zero();
        };
        let row_end = region.row + region.row_span;
        let column_end = region.column + region.column_span;
        let mut result = LogicalBoxStrut::zero();
        for row in region.row..row_end {
            result.inline_start = result
                .inline_start
                .max_of(self.inline_edge(row, region.column).paintable_width());
            result.inline_end = result
                .inline_end
                .max_of(self.inline_edge(row, column_end).paintable_width());
        }
        for column in region.column..column_end {
            result.block_start = result
                .block_start
                .max_of(self.block_edge(region.row, column).paintable_width());
            result.block_end = result
                .block_end
                .max_of(self.block_edge(row_end, column).paintable_width());
        }
        // Each side is divided independently, including odd raw widths.
        // Never assign the other half `width - half`: .50 explicitly accepts
        // a one-raw-unit shortfall for widths not divisible by two.
        result.inline_start = result.inline_start / 2;
        result.inline_end = result.inline_end / 2;
        result.block_start = result.block_start / 2;
        result.block_end = result.block_end / 2;
        result
    }

    pub fn used_table_border(&self) -> LogicalBoxStrut {
        self.used_cell_border(GridRegion {
            row: 0,
            column: 0,
            row_span: self.rows,
            column_span: self.columns,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanonicalColumnLocation {
    pub offset: LayoutUnit,
    pub size: LayoutUnit,
    pub is_collapsed: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CanonicalRowSize {
    pub size: LayoutUnit,
    pub is_collapsed: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CanonicalCellGeometry {
    // Relative to the starting row's logical origin; table/section/caption
    // ancestors and authored relative offsets are applied once by callers.
    pub rect_in_starting_row: LogicalRect,
    pub hidden_for_paint: bool,
}

pub fn canonical_cell_geometry(
    region: GridRegion,
    columns: &[CanonicalColumnLocation],
    rows: &[CanonicalRowSize],
    inline_spacing: LayoutUnit,
    block_spacing: LayoutUnit,
) -> Option<CanonicalCellGeometry> {
    let region = region.clamp(rows.len(), columns.len())?;
    let last_column = region.column + region.column_span - 1;
    let first = columns[region.column];
    let last = columns[last_column];
    let inline_size = last.offset + last.size - first.offset;
    let mut block_size = LayoutUnit::zero();
    // Same rule as pinned ComputeCellBlockSize: a cell originating in a
    // collapsed row has zero block size, even if it spans visible rows.
    if !rows[region.row].is_collapsed {
        for offset in 0..region.row_span {
            let row = rows[region.row + offset];
            if row.is_collapsed {
                continue;
            }
            block_size = block_size + row.size;
            if offset != 0 {
                block_size = block_size + block_spacing;
            }
        }
    }
    Some(CanonicalCellGeometry {
        rect_in_starting_row: LogicalRect::new(
            LogicalOffset::new(first.offset - inline_spacing, LayoutUnit::zero()),
            LogicalSize::new(inline_size, block_size),
        ),
        hidden_for_paint: columns[region.column..=last_column]
            .iter()
            .all(|c| c.is_collapsed),
    })
}

/// The rows actually materialized in one table-section continuation.
///
/// `row_offsets` contains one section-local logical block boundary per row,
/// followed by the terminal boundary. Source row identities survive section
/// repetition and slicing; paint must not recover them from DOM order or the
/// table wrapper's decoration offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollapsedTableSectionData {
    pub global_start_row: usize,
    pub row_offsets: Vec<LayoutUnit>,
    pub is_start_row_fragmented: bool,
    pub is_end_row_fragmented: bool,
}

impl CollapsedTableSectionData {
    /// Project this continuation's existing rows into a local block slice.
    /// The immutable table graph and its source row numbering do not change.
    pub fn project_block_slice(&mut self, consumed: LayoutUnit, size: LayoutUnit) {
        let row_count = self.row_offsets.len().saturating_sub(1);
        if row_count == 0 {
            return;
        }
        let start = consumed.clamp_negative_to_zero();
        let end = start + size.clamp_negative_to_zero();
        if start == LayoutUnit::zero()
            && size > LayoutUnit::zero()
            && end >= *self.row_offsets.last().expect("section terminal boundary")
        {
            return;
        }
        if size <= LayoutUnit::zero() {
            self.row_offsets.clear();
            self.is_start_row_fragmented = false;
            self.is_end_row_fragmented = false;
            return;
        }
        let first = (0..row_count).find(|&row| {
            self.row_offsets[row + 1] > start
                || (self.row_offsets[row] == self.row_offsets[row + 1]
                    && self.row_offsets[row] >= start
                    && self.row_offsets[row] < end)
        });
        let Some(first) = first else {
            self.global_start_row += row_count;
            self.row_offsets.clear();
            self.is_start_row_fragmented = false;
            self.is_end_row_fragmented = false;
            return;
        };
        let last = (first..row_count)
            .take_while(|&row| self.row_offsets[row] < end)
            .last();
        let Some(last) = last else {
            self.row_offsets.clear();
            self.is_start_row_fragmented = false;
            self.is_end_row_fragmented = false;
            return;
        };
        let source_start = self.row_offsets[first];
        let source_end = self.row_offsets[last + 1];
        let starts_in_row = start > source_start;
        let ends_in_row = end < source_end;
        let previous_start_fragmented = self.is_start_row_fragmented;
        let previous_end_fragmented = self.is_end_row_fragmented;
        self.row_offsets = self.row_offsets[first..=last + 1]
            .iter()
            .map(|&offset| offset.max_of(start).min_of(end) - start)
            .collect();
        self.global_start_row += first;
        self.is_start_row_fragmented = starts_in_row || (first == 0 && previous_start_fragmented);
        self.is_end_row_fragmented =
            ends_in_row || (last + 1 == row_count && previous_end_fragmented);
    }
}

/// Immutable logical edge winners and canonical grid tracks for the table
/// paint owner. Cell layout uses independent half struts; painting must keep
/// these full-width source segments and junction orders separate.
#[derive(Debug, Clone)]
pub struct CollapsedTableBorderData {
    pub borders: ResolvedCollapsedBorders,
    pub columns: Vec<CanonicalColumnLocation>,
    pub rows: Vec<CanonicalRowSize>,
    pub grid_offset: LogicalOffset,
    pub grid_size: LogicalSize,
    pub writing_direction: WritingDirectionMode,
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::{Document, ElementTag};

    #[test]
    fn section_continuations_preserve_source_rows_and_partial_row_flags() {
        let original = CollapsedTableSectionData {
            global_start_row: 7,
            row_offsets: [0, 10, 20, 30].map(LayoutUnit::from_i32).to_vec(),
            is_start_row_fragmented: false,
            is_end_row_fragmented: false,
        };
        let repeated_header = original.clone();
        let mut body = original.clone();
        body.project_block_slice(LayoutUnit::from_i32(5), LayoutUnit::from_i32(20));
        assert_eq!(body.global_start_row, 7);
        assert_eq!(body.row_offsets, [0, 5, 15, 20].map(LayoutUnit::from_i32));
        assert!(body.is_start_row_fragmented);
        assert!(body.is_end_row_fragmented);
        assert_eq!(repeated_header, original);

        // A nested fragmentainer consumes local continuation coordinates.
        // It must neither advance the source row twice nor retain partial-row
        // flags belonging to rows which have now been completely consumed.
        body.project_block_slice(LayoutUnit::from_i32(5), LayoutUnit::from_i32(10));
        let mut direct = original.clone();
        direct.project_block_slice(LayoutUnit::from_i32(10), LayoutUnit::from_i32(10));
        assert_eq!(body, direct);
        assert_eq!(body.global_start_row, 8);
        assert_eq!(body.row_offsets, [0, 10].map(LayoutUnit::from_i32));
        assert!(!body.is_start_row_fragmented);
        assert!(!body.is_end_row_fragmented);
    }

    #[test]
    fn section_projection_retains_collapsed_row_identity_and_clears_empty_slices() {
        let mut section = CollapsedTableSectionData {
            global_start_row: 4,
            row_offsets: [0, 10, 10, 20, 20].map(LayoutUnit::from_i32).to_vec(),
            is_start_row_fragmented: false,
            is_end_row_fragmented: false,
        };
        let full = section.clone();
        section.project_block_slice(LayoutUnit::zero(), LayoutUnit::from_i32(20));
        assert_eq!(section, full);
        section.project_block_slice(LayoutUnit::from_i32(10), LayoutUnit::from_i32(10));
        assert_eq!(section.global_start_row, 5);
        assert_eq!(section.row_offsets, [0, 0, 10].map(LayoutUnit::from_i32));
        assert!(!section.is_start_row_fragmented);
        assert!(!section.is_end_row_fragmented);
        section.project_block_slice(LayoutUnit::zero(), LayoutUnit::zero());
        assert!(section.row_offsets.is_empty());
        assert!(!section.is_start_row_fragmented);
        assert!(!section.is_end_row_fragmented);
    }

    fn box_input(
        node: NodeId,
        region: GridRegion,
        kind: BorderSourceKind,
        order: usize,
        style: BorderStyle,
        width: LayoutUnit,
    ) -> BorderBoxInput {
        let candidate = |physical_side| BorderCandidate {
            node_id: node,
            physical_side,
            source_kind: kind,
            box_order: order,
            style,
            width,
        };
        BorderBoxInput {
            region,
            source_kind: kind,
            borders: LogicalBorderCandidates {
                inline_start: candidate(PhysicalBorderSide::Left),
                inline_end: candidate(PhysicalBorderSide::Right),
                block_start: candidate(PhysicalBorderSide::Top),
                block_end: candidate(PhysicalBorderSide::Bottom),
            },
        }
    }

    #[test]
    fn winning_perimeter_is_local_to_each_span_and_odd_halves_are_independent() {
        let mut doc = Document::new();
        let first = doc.create_node(ElementTag::TableCell);
        let neighbor = doc.create_node(ElementTag::TableCell);
        let region = GridRegion {
            row: 0,
            column: 0,
            row_span: 2,
            column_span: 1,
        };
        let cells = [
            box_input(
                first,
                region,
                BorderSourceKind::Cell,
                1,
                BorderStyle::Solid,
                LayoutUnit::from_i32(1),
            ),
            box_input(
                neighbor,
                GridRegion {
                    row: 0,
                    column: 1,
                    row_span: 1,
                    column_span: 1,
                },
                BorderSourceKind::Cell,
                2,
                BorderStyle::Solid,
                LayoutUnit::from_i32(3),
            ),
            box_input(
                neighbor,
                GridRegion {
                    row: 1,
                    column: 1,
                    row_span: 1,
                    column_span: 1,
                },
                BorderSourceKind::Cell,
                3,
                BorderStyle::Solid,
                LayoutUnit::from_i32(7),
            ),
            box_input(
                neighbor,
                GridRegion {
                    row: 2,
                    column: 1,
                    row_span: 1,
                    column_span: 1,
                },
                BorderSourceKind::Cell,
                4,
                BorderStyle::Solid,
                LayoutUnit::from_i32(9),
            ),
        ];
        let grid = ResolvedCollapsedBorders::build(3, 2, &cells, &[], &[], &[], &[], None).unwrap();
        assert_eq!(
            grid.used_cell_border(region).inline_end,
            LayoutUnit::from_f32(3.5)
        );
        let odd = box_input(
            first,
            GridRegion {
                row: 0,
                column: 0,
                row_span: 1,
                column_span: 1,
            },
            BorderSourceKind::Cell,
            1,
            BorderStyle::Solid,
            LayoutUnit::from_raw(65),
        );
        let grid = ResolvedCollapsedBorders::build(1, 1, &[odd], &[], &[], &[], &[], None).unwrap();
        let used = grid.used_table_border();
        assert_eq!(used.inline_start.raw(), 32);
        assert_eq!(used.inline_end.raw(), 32);
    }

    #[test]
    fn hidden_winners_keep_source_identity_but_contribute_no_used_width() {
        let mut doc = Document::new();
        let first = doc.create_node(ElementTag::TableCell);
        let last = doc.create_node(ElementTag::TableCell);
        let region = GridRegion {
            row: 0,
            column: 0,
            row_span: 1,
            column_span: 1,
        };
        let visible = box_input(
            first,
            region,
            BorderSourceKind::Cell,
            1,
            BorderStyle::Solid,
            LayoutUnit::from_i32(9),
        );
        let hidden = box_input(
            first,
            region,
            BorderSourceKind::Cell,
            2,
            BorderStyle::Hidden,
            LayoutUnit::zero(),
        );
        let later = box_input(
            last,
            region,
            BorderSourceKind::Cell,
            3,
            BorderStyle::Hidden,
            LayoutUnit::zero(),
        );
        let grid = ResolvedCollapsedBorders::build(
            1,
            1,
            &[visible, hidden, later],
            &[],
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();
        let ResolvedEdge::Winner(edge) = grid.inline_edge(0, 0) else {
            panic!("hidden winner");
        };
        assert_eq!(edge.node_id, last);
        assert_eq!(edge.style, BorderStyle::Hidden);
        assert_eq!(grid.used_table_border(), LogicalBoxStrut::zero());
    }

    #[test]
    fn span_interiors_block_later_structural_edges_and_equal_ties_keep_first_source() {
        let mut doc = Document::new();
        let cell = doc.create_node(ElementTag::TableCell);
        let row = doc.create_node(ElementTag::TableRow);
        let span = BorderBoxInput {
            region: GridRegion {
                row: 0,
                column: 0,
                row_span: 2,
                column_span: 2,
            },
            borders: LogicalBorderCandidates::none(1),
            source_kind: BorderSourceKind::Cell,
        };
        let row_input = box_input(
            row,
            GridRegion {
                row: 1,
                column: 0,
                row_span: 1,
                column_span: 2,
            },
            BorderSourceKind::Row,
            2,
            BorderStyle::Solid,
            LayoutUnit::from_i32(7),
        );
        let grid =
            ResolvedCollapsedBorders::build(2, 2, &[span], &[row_input], &[], &[], &[], None)
                .unwrap();
        assert!(matches!(grid.block_edge(1, 0), ResolvedEdge::SpanInterior));
        assert!(matches!(grid.inline_edge(0, 1), ResolvedEdge::SpanInterior));
        let region = GridRegion {
            row: 0,
            column: 0,
            row_span: 1,
            column_span: 1,
        };
        let a = box_input(
            cell,
            region,
            BorderSourceKind::Cell,
            1,
            BorderStyle::Solid,
            LayoutUnit::from_i32(3),
        );
        let b = box_input(
            row,
            region,
            BorderSourceKind::Row,
            2,
            BorderStyle::Solid,
            LayoutUnit::from_i32(3),
        );
        let grid = ResolvedCollapsedBorders::build(1, 1, &[a], &[b], &[], &[], &[], None).unwrap();
        let ResolvedEdge::Winner(edge) = grid.inline_edge(0, 0) else {
            panic!("first winner");
        };
        assert_eq!(edge.node_id, cell);
    }

    #[test]
    fn collapsed_style_normalization_precedes_priority_comparison() {
        let mut doc = Document::new();
        let node = doc.create_node(ElementTag::TableCell);
        doc.update_resolved_style(node, |style| {
            style.border_left_style = BorderStyle::Inset;
            style.border_left_width = 3;
            style.border_right_style = BorderStyle::Outset;
            style.border_right_width = 3;
        });
        let borders = LogicalBorderCandidates::from_style(
            node,
            &doc.node(node).style,
            BorderSourceKind::Cell,
            1,
            WritingDirectionMode::horizontal_ltr(),
        );
        assert_eq!(borders.inline_start.style, BorderStyle::Ridge);
        assert_eq!(borders.inline_end.style, BorderStyle::Groove);
    }
}
