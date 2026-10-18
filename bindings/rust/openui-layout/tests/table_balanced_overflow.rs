//! Real table continuations for the immutable .50 empty-row measurement.
//! These cases use no glyph, border, spacing or raster rounding assumptions.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block::block_layout, ConstraintSpace, Fragment, FragmentTablePart};
use openui_style::{BorderCollapse, ColumnFill, Display};

fn collect_owned<'a>(fragment: &'a Fragment, node: NodeId, result: &mut Vec<&'a Fragment>) {
    if fragment.node_id == node && fragment.owns_element_box() {
        result.push(fragment);
    }
    for child in &fragment.children {
        collect_owned(child, node, result);
    }
}

fn row_intersections(
    fragment: &Fragment,
    node: NodeId,
    parent_top: LayoutUnit,
    table_height: LayoutUnit,
    result: &mut Vec<LayoutUnit>,
) {
    let top = parent_top + fragment.offset.top;
    if fragment.node_id == node && fragment.owns_element_box() {
        let start = top.max_of(LayoutUnit::zero());
        let end = (top + fragment.size.height).min_of(table_height);
        if end > start {
            result.push(end - start);
        }
    }
    for child in &fragment.children {
        row_intersections(child, node, top, table_height, result);
    }
}

#[test]
fn balanced_empty_table_rows_use_the_measured_last_resort_split_in_inline_overflow() {
    for collapse in [BorderCollapse::Collapse, BorderCollapse::Separate] {
        for fill in [ColumnFill::Balance, ColumnFill::Auto] {
            for explicit_section in [false, true] {
                let mut doc = Document::new();
                let columns = doc.create_node(ElementTag::Div);
                let table = doc.create_node(ElementTag::Table);
                let column = doc.create_node(ElementTag::TableColumn);
                doc.append_child(doc.root(), columns);
                doc.append_child(columns, table);
                doc.append_child(table, column);
                doc.update_resolved_style(columns, |style| {
                    style.display = Display::Block;
                    style.width = Length::px(80.0);
                    style.height = Length::px(25.0);
                    style.column_fill = fill;
                    style.column_count = Some(2);
                    style.column_gap = Some(Length::zero());
                });
                doc.update_resolved_style(table, |style| {
                    style.display = Display::Table;
                    style.border_collapse = collapse;
                });
                doc.update_resolved_style(column, |style| style.display = Display::TableColumn);
                let parent = if explicit_section {
                    let section = doc.create_node(ElementTag::TableBody);
                    doc.append_child(table, section);
                    doc.update_resolved_style(section, |style| {
                        style.display = Display::TableRowGroup
                    });
                    section
                } else {
                    table
                };
                let mut rows = Vec::new();
                for _ in 0..6 {
                    let row = doc.create_node(ElementTag::TableRow);
                    let cell = doc.create_node(ElementTag::TableCell);
                    doc.append_child(parent, row);
                    doc.append_child(row, cell);
                    doc.update_resolved_style(row, |style| style.display = Display::TableRow);
                    doc.update_resolved_style(cell, |style| {
                        style.display = Display::TableCell;
                        style.width = Length::px(20.0);
                        style.height = Length::px(10.0);
                    });
                    rows.push(row);
                }
                let root = block_layout(
                    &doc,
                    doc.root(),
                    &ConstraintSpace::for_root(
                        LayoutUnit::from_i32(200),
                        LayoutUnit::from_i32(100),
                    ),
                );
                let mut pieces = Vec::new();
                collect_owned(&root, table, &mut pieces);
                assert!(pieces
                    .iter()
                    .all(|part| part.table_part == Some(FragmentTablePart::Table)));
                // Root's retained native and exact .50 oracle diagnostics:
                // native-real-multicol-table-{diagnostic-v4396,oracle-v4390}.
                assert_eq!(
                    pieces
                        .iter()
                        .map(|piece| piece.size.height)
                        .collect::<Vec<_>>(),
                    [
                        LayoutUnit::from_i32(25),
                        LayoutUnit::from_i32(25),
                        LayoutUnit::from_i32(10)
                    ],
                    "{collapse:?}/{fill:?}/section={explicit_section}"
                );
                assert_eq!(
                    pieces
                        .iter()
                        .map(|piece| piece.size.height)
                        .fold(LayoutUnit::zero(), |a, b| a + b),
                    LayoutUnit::from_i32(60)
                );
                for (index, piece) in pieces.iter().enumerate() {
                    let positioning = piece.table_column_background_positioning.unwrap();
                    assert_eq!(positioning.rect.size.height, LayoutUnit::from_i32(60));
                    assert_eq!(
                        positioning.rect.y(),
                        LayoutUnit::from_i32(-25 * index as i32)
                    );
                }
                // Inspect actual retained row intervals against their owning
                // table continuations. The row crossing source y=25 is split
                // 5+5, while the ordinary first-row neighbor remains 10px.
                let mut crossing = Vec::new();
                let mut first = Vec::new();
                for piece in &pieces {
                    for child in &piece.children {
                        row_intersections(
                            child,
                            rows[2],
                            LayoutUnit::zero(),
                            piece.size.height,
                            &mut crossing,
                        );
                        row_intersections(
                            child,
                            rows[0],
                            LayoutUnit::zero(),
                            piece.size.height,
                            &mut first,
                        );
                    }
                }
                assert_eq!(crossing, [LayoutUnit::from_i32(5), LayoutUnit::from_i32(5)]);
                assert_eq!(first, [LayoutUnit::from_i32(10)]);
            }
        }
    }
}
