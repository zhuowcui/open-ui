//! Grid columns with an intrinsic maximum grow toward their growth limits
//! (CSS Grid's maximize step) before flexible columns take the rest.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset};
use openui_layout::intrinsic_sizing::compute_child_intrinsic_contribution;
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_style::{
    Display, GridTrackBreadth, GridTrackComponent, GridTrackList, GridTrackSize, LineHeight,
};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn fragment_for(parent: &Fragment, node_id: NodeId) -> &Fragment {
    parent
        .children
        .iter()
        .find(|fragment| fragment.node_id == node_id)
        .unwrap_or_else(|| panic!("missing fragment for {node_id:?}"))
}

fn track(breadth: GridTrackBreadth) -> GridTrackComponent {
    GridTrackComponent::Track(GridTrackSize::Breadth(breadth))
}

/// A block holding wrappable text at a 20px font and line height.
fn text_item(doc: &mut Document, parent: NodeId, content: &str) -> NodeId {
    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.display = Display::Block;
        style.font_size = 20.0;
        style.line_height = LineHeight::Length(20.0);
    });
    doc.append_child(parent, item);
    let text = doc.create_node(ElementTag::Text);
    doc.update_resolved_style(text, |style| {
        style.font_size = 20.0;
        style.line_height = LineHeight::Length(20.0);
    });
    doc.node_mut(text).text = Some(content.to_string());
    doc.append_child(item, text);
    item
}

fn grid(doc: &mut Document, columns: Vec<GridTrackComponent>) -> NodeId {
    let grid = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(grid, |style| {
        style.display = Display::Grid;
        style.width = Length::px(300.0);
        style.column_gap = Some(Length::px(8.0));
        style.grid_template_columns = GridTrackList::Tracks(columns);
    });
    doc.append_child(doc.root(), grid);
    grid
}

fn layout(doc: &Document, grid: NodeId) -> Fragment {
    block_layout(
        doc,
        grid,
        &ConstraintSpace::for_block_child(lu(300), lu(600), lu(300), lu(600), false),
    )
}

/// The components track's card header: `grid-template-columns: 1fr auto`
/// in a definite width. Chromium sizes the `auto` column to its item's
/// max-content width, so the text stays on one line; the `1fr` column takes
/// the rest.
#[test]
fn an_auto_column_beside_a_flexible_column_takes_its_max_content_width() {
    let mut doc = Document::new();
    let grid = grid(
        &mut doc,
        vec![
            track(GridTrackBreadth::Flex(1.0)),
            track(GridTrackBreadth::Auto),
        ],
    );
    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| style.display = Display::Block);
    doc.append_child(grid, first);
    let action = text_item(&mut doc, grid, "XX XX");

    let intrinsic = compute_child_intrinsic_contribution(&doc, action);
    assert!(
        intrinsic.min_content_inline_size < intrinsic.max_content_inline_size,
        "the text must be able to wrap for this test to mean anything"
    );
    let max_content = intrinsic.max_content_inline_size;

    let fragment = layout(&doc, grid);
    let action_fragment = fragment_for(&fragment, action);
    assert_eq!(action_fragment.size.width, max_content);
    assert_eq!(
        action_fragment.offset,
        PhysicalOffset::new(lu(300) - max_content, LayoutUnit::zero())
    );
    assert_eq!(action_fragment.size.height, lu(20));
    assert_eq!(
        fragment_for(&fragment, first).size.width,
        lu(292) - max_content
    );
}

/// Two `auto` columns: the maximize step fills each to its max-content
/// width, then the remaining space is shared equally, so the wider item's
/// column stays wider. Content alignment alone would make them equal.
#[test]
fn auto_columns_grow_to_max_content_before_stretching_equally() {
    let mut doc = Document::new();
    let grid = grid(
        &mut doc,
        vec![track(GridTrackBreadth::Auto), track(GridTrackBreadth::Auto)],
    );
    let narrow = text_item(&mut doc, grid, "XX");
    let wide = text_item(&mut doc, grid, "XX XX XX");
    let narrow_max = compute_child_intrinsic_contribution(&doc, narrow).max_content_inline_size;
    let wide_max = compute_child_intrinsic_contribution(&doc, wide).max_content_inline_size;
    assert!(narrow_max + wide_max + lu(8) < lu(300));

    let fragment = layout(&doc, grid);
    let leftover = lu(300) - lu(8) - narrow_max - wide_max;
    let narrow_width = fragment_for(&fragment, narrow).size.width;
    let wide_width = fragment_for(&fragment, wide).size.width;
    assert_eq!(narrow_width, narrow_max + leftover / 2);
    assert_eq!(narrow_width + wide_width, lu(292));
}
