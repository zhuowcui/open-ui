//! The grid auto-placement cursor follows Blink's
//! `GridPlacement::RunAutoPlacementAlgorithm`: items placed in steps 1 and 2
//! do not move the step 4 cursor, a definite column behind the cursor moves
//! it to the next row, and items locked to a row pack after the last one
//! placed in that row.
//!
//! Expected boxes are pinned Chromium 147's, recorded by
//! `tools/qualification/capture_grid_placement_oracle.py` in
//! `docs/progress/evidence/grid-placement-cursor-v1/chromium-windows.json`.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_style::{
    Display, GridAutoFlow, GridAutoFlowDirection, GridLine, GridPlacement, GridTrackBreadth,
    GridTrackComponent, GridTrackList, GridTrackSize,
};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn fixed(px: f32) -> GridTrackSize {
    GridTrackSize::Breadth(GridTrackBreadth::Length(Length::px(px)))
}

fn tracks(sizes: Vec<GridTrackSize>) -> GridTrackList {
    GridTrackList::Tracks(sizes.into_iter().map(GridTrackComponent::Track).collect())
}

fn line(index: i32) -> GridLine {
    GridLine::Line { index, name: None }
}

fn span(count: u32) -> GridLine {
    GridLine::Span { count, name: None }
}

fn placement(start: GridLine, end: GridLine) -> GridPlacement {
    GridPlacement { start, end }
}

struct Item {
    column: GridPlacement,
    row: GridPlacement,
    width: Option<f32>,
    height: f32,
}

fn item(height: f32) -> Item {
    Item {
        column: GridPlacement::default(),
        row: GridPlacement::default(),
        width: None,
        height,
    }
}

fn layout_grid(
    width: f32,
    configure: impl FnOnce(&mut openui_style::ComputedStyleFields),
    items: Vec<Item>,
) -> Vec<(i32, i32, i32)> {
    let mut doc = Document::new();
    let grid = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(grid, |style| {
        style.display = Display::Grid;
        style.width = Length::px(width);
        configure(style);
    });
    doc.append_child(doc.root(), grid);
    let ids: Vec<NodeId> = items
        .into_iter()
        .map(|spec| {
            let child = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(child, |style| {
                style.display = Display::Block;
                style.grid_column = spec.column;
                style.grid_row = spec.row;
                if let Some(width) = spec.width {
                    style.width = Length::px(width);
                }
                style.height = Length::px(spec.height);
            });
            doc.append_child(grid, child);
            child
        })
        .collect();
    let fragment = block_layout(
        &doc,
        grid,
        &ConstraintSpace::for_block_child(lu(400), lu(400), lu(400), lu(400), false),
    );
    ids.iter()
        .map(|id| {
            let child: &Fragment = fragment
                .children
                .iter()
                .find(|child| child.node_id == *id)
                .expect("grid item fragment");
            (
                child.offset.left.to_f32() as i32,
                child.offset.top.to_f32() as i32,
                child.size.width.to_f32() as i32,
            )
        })
        .collect()
}

/// The components track's card header: title, description and an action
/// at `grid-column: 2; grid-row: 1 / span 2`. The action is placed first
/// and must not push the title into the second row.
#[test]
fn a_definite_item_does_not_move_the_auto_placement_cursor() {
    let boxes = layout_grid(
        300.0,
        |style| {
            style.grid_template_columns = tracks(vec![
                GridTrackSize::Breadth(GridTrackBreadth::Flex(1.0)),
                GridTrackSize::Breadth(GridTrackBreadth::Auto),
            ]);
            style.grid_template_rows = tracks(vec![
                GridTrackSize::Breadth(GridTrackBreadth::Auto),
                GridTrackSize::Breadth(GridTrackBreadth::Auto),
            ]);
            style.grid_auto_rows = vec![GridTrackSize::Breadth(GridTrackBreadth::MinContent)];
            style.row_gap = Some(Length::px(8.0));
            style.column_gap = Some(Length::px(8.0));
        },
        vec![
            item(16.0),
            item(20.0),
            Item {
                column: placement(line(2), GridLine::Auto),
                row: placement(line(1), span(2)),
                width: Some(50.0),
                height: 16.0,
            },
        ],
    );
    assert_eq!(boxes, vec![(0, 0, 242), (0, 24, 242), (250, 0, 50)]);
}

/// Row auto-flow: an item with a definite column behind the cursor starts
/// on the next row.
#[test]
fn a_definite_column_behind_the_cursor_moves_to_the_next_row() {
    let boxes = layout_grid(
        150.0,
        |style| {
            style.grid_template_columns = tracks(vec![fixed(50.0), fixed(50.0), fixed(50.0)]);
            style.grid_auto_rows = vec![fixed(20.0)];
        },
        vec![
            Item {
                column: placement(line(2), GridLine::Auto),
                ..item(20.0)
            },
            Item {
                column: placement(line(1), GridLine::Auto),
                ..item(20.0)
            },
        ],
    );
    assert_eq!(boxes, vec![(50, 0, 50), (0, 20, 50)]);
}

/// Sparse packing: an item locked to a row starts after the last item
/// locked to that row, not at the first free column.
#[test]
fn row_locked_items_pack_after_the_previous_one_in_their_row() {
    let boxes = layout_grid(
        300.0,
        |style| {
            style.grid_template_columns = tracks(vec![fixed(50.0), fixed(50.0), fixed(50.0)]);
            style.grid_auto_columns = vec![fixed(50.0)];
            style.grid_auto_rows = vec![fixed(20.0)];
        },
        vec![
            Item {
                column: placement(line(2), GridLine::Auto),
                row: placement(line(1), GridLine::Auto),
                ..item(20.0)
            },
            Item {
                column: placement(span(2), GridLine::Auto),
                row: placement(line(1), GridLine::Auto),
                ..item(20.0)
            },
            Item {
                row: placement(line(1), GridLine::Auto),
                ..item(20.0)
            },
        ],
    );
    assert_eq!(boxes, vec![(50, 0, 50), (100, 0, 100), (200, 0, 50)]);
}

/// Column auto-flow, the transpose of the first case.
#[test]
fn column_flow_auto_items_start_from_the_first_column() {
    let boxes = layout_grid(
        300.0,
        |style| {
            style.grid_auto_flow = GridAutoFlow {
                direction: GridAutoFlowDirection::Column,
                dense: false,
            };
            style.grid_template_rows = tracks(vec![fixed(20.0), fixed(20.0)]);
            style.grid_auto_columns = vec![fixed(50.0)];
        },
        vec![
            Item {
                column: placement(line(1), GridLine::Auto),
                row: placement(line(2), GridLine::Auto),
                ..item(20.0)
            },
            item(20.0),
            item(20.0),
        ],
    );
    assert_eq!(boxes, vec![(0, 20, 50), (0, 0, 50), (50, 0, 50)]);
}
