//! SP17 W1K — safe overflow alignment for flex abspos static positions.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{
    BoxStrut, LayoutUnit, Length, LogicalOffset, PhysicalOffset, PhysicalSize, WritingModeConverter,
};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_style::{
    BorderStyle, ContentAlignment, ContentDistribution, ContentPosition, Direction, Display,
    FlexDirection, FlexWrap, ItemAlignment, ItemPosition, OverflowAlignment, Position, WritingMode,
};

const CONTENT_WIDTH: i32 = 90;
const CONTENT_HEIGHT: i32 = 70;

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn content_alignment(position: ContentPosition, overflow: OverflowAlignment) -> ContentAlignment {
    ContentAlignment {
        position,
        distribution: ContentDistribution::Default,
        overflow,
    }
}

fn layout_root(doc: &Document) -> Fragment {
    block_layout(
        doc,
        doc.root(),
        &ConstraintSpace::for_root(lu(800), lu(600)),
    )
}

fn fragment_for(parent: &Fragment, node_id: NodeId) -> &Fragment {
    parent
        .children
        .iter()
        .find(|fragment| fragment.node_id == node_id)
        .unwrap_or_else(|| panic!("missing fragment for {node_id:?}"))
}

fn container_border() -> BoxStrut {
    BoxStrut::new(lu(11), lu(7), lu(13), lu(3))
}

fn container_padding() -> BoxStrut {
    BoxStrut::new(lu(23), lu(19), lu(29), lu(17))
}

fn child_border() -> BoxStrut {
    BoxStrut::new(lu(3), lu(2), lu(4), lu(1))
}

fn child_padding() -> BoxStrut {
    BoxStrut::new(lu(7), lu(6), lu(8), lu(5))
}

fn child_margin() -> BoxStrut {
    BoxStrut::new(lu(11), lu(10), lu(12), lu(9))
}

fn add_flex_case(
    doc: &mut Document,
    mode: WritingMode,
    direction: Direction,
    flex_direction: FlexDirection,
    child_content_size: PhysicalSize,
) -> (NodeId, NodeId) {
    let container = doc.create_node(ElementTag::Div);
    {
        let style = &mut doc.node_mut(container).style;
        style.display = Display::Flex;
        style.position = Position::Relative;
        style.writing_mode = mode;
        style.direction = direction;
        style.flex_direction = flex_direction;
        style.width = Length::px(CONTENT_WIDTH as f32);
        style.height = Length::px(CONTENT_HEIGHT as f32);
        style.border_left_width = 3;
        style.border_right_width = 7;
        style.border_top_width = 11;
        style.border_bottom_width = 13;
        style.border_left_style = BorderStyle::Solid;
        style.border_right_style = BorderStyle::Solid;
        style.border_top_style = BorderStyle::Solid;
        style.border_bottom_style = BorderStyle::Solid;
        style.padding_left = Length::px(17.0);
        style.padding_right = Length::px(19.0);
        style.padding_top = Length::px(23.0);
        style.padding_bottom = Length::px(29.0);
    }
    doc.append_child(doc.root(), container);

    let child = doc.create_node(ElementTag::Div);
    {
        let style = &mut doc.node_mut(child).style;
        style.display = Display::Block;
        style.position = Position::Absolute;
        style.writing_mode = mode;
        style.direction = direction;
        style.width = Length::px(child_content_size.width.to_f32());
        style.height = Length::px(child_content_size.height.to_f32());
        style.border_left_width = 1;
        style.border_right_width = 2;
        style.border_top_width = 3;
        style.border_bottom_width = 4;
        style.border_left_style = BorderStyle::Solid;
        style.border_right_style = BorderStyle::Solid;
        style.border_top_style = BorderStyle::Solid;
        style.border_bottom_style = BorderStyle::Solid;
        style.padding_left = Length::px(5.0);
        style.padding_right = Length::px(6.0);
        style.padding_top = Length::px(7.0);
        style.padding_bottom = Length::px(8.0);
        style.margin_left = Length::px(9.0);
        style.margin_right = Length::px(10.0);
        style.margin_top = Length::px(11.0);
        style.margin_bottom = Length::px(12.0);
    }
    doc.append_child(container, child);
    (container, child)
}

fn child_margin_box_size(child_content_size: PhysicalSize) -> PhysicalSize {
    let border = child_border();
    let padding = child_padding();
    let margin = child_margin();
    PhysicalSize::new(
        child_content_size.width
            + border.left
            + border.right
            + padding.left
            + padding.right
            + margin.left
            + margin.right,
        child_content_size.height
            + border.top
            + border.bottom
            + padding.top
            + padding.bottom
            + margin.top
            + margin.bottom,
    )
}

fn expected_child_offset(
    mode: WritingMode,
    direction: Direction,
    child_content_size: PhysicalSize,
    inline_alignment_offset: LayoutUnit,
    block_alignment_offset: LayoutUnit,
) -> PhysicalOffset {
    let border = container_border();
    let padding = container_padding();
    let container_size = PhysicalSize::new(
        lu(CONTENT_WIDTH) + border.left + border.right + padding.left + padding.right,
        lu(CONTENT_HEIGHT) + border.top + border.bottom + padding.top + padding.bottom,
    );
    let writing_direction = direction.writing_direction(mode);
    let converter = WritingModeConverter::new(writing_direction, container_size);
    let logical_border = border.to_logical(writing_direction);
    let logical_padding = padding.to_logical(writing_direction);
    let margin_box = child_margin_box_size(child_content_size);
    let margin_box_offset = converter.to_physical_offset(
        LogicalOffset::new(
            logical_border.inline_start + logical_padding.inline_start + inline_alignment_offset,
            logical_border.block_start + logical_padding.block_start + block_alignment_offset,
        ),
        margin_box,
    );
    let margin = child_margin();
    PhysicalOffset::new(
        margin_box_offset.left + margin.left,
        margin_box_offset.top + margin.top,
    )
}

fn assert_case_offset(
    doc: &Document,
    container: NodeId,
    child: NodeId,
    expected: PhysicalOffset,
    child_content_size: PhysicalSize,
    label: &str,
) {
    let root = layout_root(doc);
    let container_fragment = fragment_for(&root, container);
    let child_fragment = fragment_for(container_fragment, child);
    assert_eq!(child_fragment.offset, expected, "offset for {label}");
    assert_eq!(
        child_fragment.size,
        PhysicalSize::new(
            child_content_size.width + child_border().inline_sum() + child_padding().inline_sum(),
            child_content_size.height + child_border().block_sum() + child_padding().block_sum(),
        ),
        "border-box size for {label}",
    );
    assert_eq!(child_fragment.margin, child_margin(), "margins for {label}");
}

fn writing_cases() -> impl Iterator<Item = (WritingMode, Direction)> {
    [
        WritingMode::HorizontalTb,
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
    ]
    .into_iter()
    .flat_map(|mode| {
        [Direction::Ltr, Direction::Rtl]
            .into_iter()
            .map(move |direction| (mode, direction))
    })
}

fn flex_directions() -> [FlexDirection; 4] {
    [
        FlexDirection::Row,
        FlexDirection::RowReverse,
        FlexDirection::Column,
        FlexDirection::ColumnReverse,
    ]
}

#[test]
fn oversized_safe_center_falls_back_to_logical_start_for_every_flex_axis() {
    let child_size = PhysicalSize::new(lu(100), lu(80));
    for (mode, direction) in writing_cases() {
        for flex_direction in flex_directions() {
            let mut doc = Document::new();
            let (container, child) =
                add_flex_case(&mut doc, mode, direction, flex_direction, child_size);
            doc.node_mut(container).style.justify_content =
                content_alignment(ContentPosition::Center, OverflowAlignment::Safe);
            doc.node_mut(child).style.align_self =
                ItemAlignment::with_overflow(ItemPosition::Center, OverflowAlignment::Safe);
            let expected = expected_child_offset(
                mode,
                direction,
                child_size,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            assert_case_offset(
                &doc,
                container,
                child,
                expected,
                child_size,
                &format!("safe center {mode:?}/{direction:?}/{flex_direction:?}"),
            );
        }
    }
}

#[test]
fn oversized_default_and_unsafe_center_retain_signed_offsets() {
    let child_size = PhysicalSize::new(lu(100), lu(80));
    for overflow in [OverflowAlignment::Default, OverflowAlignment::Unsafe] {
        for (mode, direction) in writing_cases() {
            let writing_direction = direction.writing_direction(mode);
            let converter = WritingModeConverter::new(
                writing_direction,
                PhysicalSize::new(lu(CONTENT_WIDTH), lu(CONTENT_HEIGHT)),
            );
            let content_logical =
                converter.to_logical_size(PhysicalSize::new(lu(CONTENT_WIDTH), lu(CONTENT_HEIGHT)));
            let child_logical = converter.to_logical_size(child_margin_box_size(child_size));
            let inline_center = LayoutUnit::from_raw(
                (content_logical.inline_size - child_logical.inline_size).raw() / 2,
            );
            let block_center = LayoutUnit::from_raw(
                (content_logical.block_size - child_logical.block_size).raw() / 2,
            );
            assert!(inline_center < LayoutUnit::zero());
            assert!(block_center < LayoutUnit::zero());

            for flex_direction in flex_directions() {
                let mut doc = Document::new();
                let (container, child) =
                    add_flex_case(&mut doc, mode, direction, flex_direction, child_size);
                doc.node_mut(container).style.justify_content =
                    content_alignment(ContentPosition::Center, overflow);
                doc.node_mut(child).style.align_self =
                    ItemAlignment::with_overflow(ItemPosition::Center, overflow);
                let expected =
                    expected_child_offset(mode, direction, child_size, inline_center, block_center);
                assert_case_offset(
                    &doc,
                    container,
                    child,
                    expected,
                    child_size,
                    &format!("{overflow:?} center {mode:?}/{direction:?}/{flex_direction:?}"),
                );
            }
        }
    }
}

#[test]
fn fitting_safe_end_and_safe_flex_end_preserve_end_and_reverse_semantics() {
    let child_size = PhysicalSize::new(lu(20), lu(10));
    for (mode, direction) in writing_cases() {
        let writing_direction = direction.writing_direction(mode);
        let converter = WritingModeConverter::new(
            writing_direction,
            PhysicalSize::new(lu(CONTENT_WIDTH), lu(CONTENT_HEIGHT)),
        );
        let content_logical =
            converter.to_logical_size(PhysicalSize::new(lu(CONTENT_WIDTH), lu(CONTENT_HEIGHT)));
        let child_logical = converter.to_logical_size(child_margin_box_size(child_size));
        let inline_free = content_logical.inline_size - child_logical.inline_size;
        let block_free = content_logical.block_size - child_logical.block_size;
        assert!(inline_free > LayoutUnit::zero());
        assert!(block_free > LayoutUnit::zero());

        for flex_direction in flex_directions() {
            for main_position in [ContentPosition::End, ContentPosition::FlexEnd] {
                let mut doc = Document::new();
                let (container, child) =
                    add_flex_case(&mut doc, mode, direction, flex_direction, child_size);
                doc.node_mut(container).style.justify_content =
                    content_alignment(main_position, OverflowAlignment::Safe);
                doc.node_mut(child).style.align_self =
                    ItemAlignment::with_overflow(ItemPosition::End, OverflowAlignment::Safe);

                let main_free = if flex_direction.is_column() {
                    block_free
                } else {
                    inline_free
                };
                let main_offset =
                    if main_position == ContentPosition::FlexEnd && flex_direction.is_reverse() {
                        LayoutUnit::zero()
                    } else {
                        main_free
                    };
                let cross_offset = if flex_direction.is_column() {
                    inline_free
                } else {
                    block_free
                };
                let (inline_offset, block_offset) = if flex_direction.is_column() {
                    (cross_offset, main_offset)
                } else {
                    (main_offset, cross_offset)
                };
                let expected =
                    expected_child_offset(mode, direction, child_size, inline_offset, block_offset);
                assert_case_offset(
                    &doc,
                    container,
                    child,
                    expected,
                    child_size,
                    &format!(
                        "safe {main_position:?}/end {mode:?}/{direction:?}/{flex_direction:?}"
                    ),
                );
            }
        }
    }
}

#[test]
fn align_self_auto_inherits_position_and_overflow_from_align_items() {
    let child_size = PhysicalSize::new(lu(100), lu(80));
    for parent_overflow in [OverflowAlignment::Safe, OverflowAlignment::Unsafe] {
        for (mode, direction) in writing_cases() {
            let writing_direction = direction.writing_direction(mode);
            let converter = WritingModeConverter::new(
                writing_direction,
                PhysicalSize::new(lu(CONTENT_WIDTH), lu(CONTENT_HEIGHT)),
            );
            let content_logical =
                converter.to_logical_size(PhysicalSize::new(lu(CONTENT_WIDTH), lu(CONTENT_HEIGHT)));
            let child_logical = converter.to_logical_size(child_margin_box_size(child_size));

            for flex_direction in flex_directions() {
                let mut doc = Document::new();
                let (container, child) =
                    add_flex_case(&mut doc, mode, direction, flex_direction, child_size);
                {
                    let style = &mut doc.node_mut(container).style;
                    style.justify_content =
                        content_alignment(ContentPosition::Start, OverflowAlignment::Safe);
                    style.align_items =
                        ItemAlignment::with_overflow(ItemPosition::Center, parent_overflow);
                }
                doc.node_mut(child).style.align_self = ItemAlignment::with_overflow(
                    ItemPosition::Auto,
                    if parent_overflow == OverflowAlignment::Safe {
                        OverflowAlignment::Unsafe
                    } else {
                        OverflowAlignment::Safe
                    },
                );

                let cross_free = if flex_direction.is_column() {
                    content_logical.inline_size - child_logical.inline_size
                } else {
                    content_logical.block_size - child_logical.block_size
                };
                let cross_offset = if parent_overflow == OverflowAlignment::Safe {
                    LayoutUnit::zero()
                } else {
                    LayoutUnit::from_raw(cross_free.raw() / 2)
                };
                let (inline_offset, block_offset) = if flex_direction.is_column() {
                    (cross_offset, LayoutUnit::zero())
                } else {
                    (LayoutUnit::zero(), cross_offset)
                };
                let expected =
                    expected_child_offset(mode, direction, child_size, inline_offset, block_offset);
                assert_case_offset(
                    &doc,
                    container,
                    child,
                    expected,
                    child_size,
                    &format!(
                        "auto inherits {parent_overflow:?} {mode:?}/{direction:?}/{flex_direction:?}"
                    ),
                );
            }
        }
    }
}

#[test]
fn wrap_reverse_safe_flex_start_overflow_falls_back_before_projection() {
    let child_size = PhysicalSize::new(lu(100), lu(80));
    for (mode, direction) in writing_cases() {
        for flex_direction in flex_directions() {
            let mut doc = Document::new();
            let (container, child) =
                add_flex_case(&mut doc, mode, direction, flex_direction, child_size);
            {
                let style = &mut doc.node_mut(container).style;
                style.flex_wrap = FlexWrap::WrapReverse;
                style.justify_content =
                    content_alignment(ContentPosition::Start, OverflowAlignment::Safe);
            }
            doc.node_mut(child).style.align_self =
                ItemAlignment::with_overflow(ItemPosition::FlexStart, OverflowAlignment::Safe);
            let expected = expected_child_offset(
                mode,
                direction,
                child_size,
                LayoutUnit::zero(),
                LayoutUnit::zero(),
            );
            assert_case_offset(
                &doc,
                container,
                child,
                expected,
                child_size,
                &format!("wrap-reverse {mode:?}/{direction:?}/{flex_direction:?}"),
            );
        }
    }
}
