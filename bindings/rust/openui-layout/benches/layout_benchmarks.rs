//! Criterion benchmarks for Open UI layout performance.
//!
//! Measures block, float, position, flex, inline, and complex layout scenarios
//! to compare with Chromium's layout times.

use criterion::{criterion_group, criterion_main, Criterion};
use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block_layout, ConstraintSpace};
use openui_style::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn root_space() -> ConstraintSpace {
    ConstraintSpace::for_root(LayoutUnit::from_i32(800), LayoutUnit::from_i32(600))
}

/// Create a block child, set its display to Block, and append it to `parent`.
fn add_block(doc: &mut Document, parent: NodeId) -> NodeId {
    let id = doc.create_node(ElementTag::Div);
    doc.node_mut(id).style.display = Display::Block;
    doc.append_child(parent, id);
    id
}

/// Create a sized block child and append it to `parent`.
fn add_sized_block(doc: &mut Document, parent: NodeId, w: f32, h: f32) -> NodeId {
    let id = add_block(doc, parent);
    doc.node_mut(id).style.width = Length::px(w);
    doc.node_mut(id).style.height = Length::px(h);
    id
}

/// Create an inline span child and append it to `parent`.
fn add_inline(doc: &mut Document, parent: NodeId) -> NodeId {
    let id = doc.create_node(ElementTag::Span);
    doc.node_mut(id).style.display = Display::Inline;
    doc.append_child(parent, id);
    id
}

/// Create a text node and append it to `parent`.
fn add_text(doc: &mut Document, parent: NodeId, text: &str) -> NodeId {
    let id = doc.create_node(ElementTag::Text);
    doc.node_mut(id).style.display = Display::Inline;
    doc.node_mut(id).text = Some(text.to_string());
    doc.append_child(parent, id);
    id
}

// ===========================================================================
// 1. Block Layout Benchmarks
// ===========================================================================

fn bench_single_block(c: &mut Criterion) {
    c.bench_function("block/single_200x100", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                add_sized_block(&mut doc, vp, 200.0, 100.0);
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_10_stacked_blocks(c: &mut Criterion) {
    c.bench_function("block/10_stacked", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                for _ in 0..10 {
                    add_sized_block(&mut doc, vp, 200.0, 50.0);
                }
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_100_stacked_blocks(c: &mut Criterion) {
    c.bench_function("block/100_stacked", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                for _ in 0..100 {
                    add_sized_block(&mut doc, vp, 200.0, 50.0);
                }
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_deep_nesting_10(c: &mut Criterion) {
    c.bench_function("block/deep_nesting_10", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                let mut parent = vp;
                for _ in 0..10 {
                    parent = add_sized_block(&mut doc, parent, 400.0, 300.0);
                }
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_deep_nesting_50(c: &mut Criterion) {
    c.bench_function("block/deep_nesting_50", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                let mut parent = vp;
                for _ in 0..50 {
                    parent = add_sized_block(&mut doc, parent, 400.0, 300.0);
                }
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_margin_collapsing(c: &mut Criterion) {
    c.bench_function("block/margin_collapsing_20", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                for _ in 0..20 {
                    let id = add_sized_block(&mut doc, vp, 300.0, 40.0);
                    doc.node_mut(id).style.margin_top = Length::px(20.0);
                    doc.node_mut(id).style.margin_bottom = Length::px(15.0);
                }
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_mixed_sizing(c: &mut Criterion) {
    c.bench_function("block/mixed_sizing", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                // Fixed width
                add_sized_block(&mut doc, vp, 300.0, 50.0);

                // Percentage width
                let pct = add_block(&mut doc, vp);
                doc.node_mut(pct).style.width = Length::percent(50.0);
                doc.node_mut(pct).style.height = Length::px(50.0);

                // Auto width (default)
                let auto = add_block(&mut doc, vp);
                doc.node_mut(auto).style.height = Length::px(50.0);

                // Min/max constrained
                let mm = add_block(&mut doc, vp);
                doc.node_mut(mm).style.width = Length::percent(80.0);
                doc.node_mut(mm).style.min_width = Length::px(100.0);
                doc.node_mut(mm).style.max_width = Length::px(400.0);
                doc.node_mut(mm).style.height = Length::px(50.0);

                // Another percentage
                let pct2 = add_block(&mut doc, vp);
                doc.node_mut(pct2).style.width = Length::percent(75.0);
                doc.node_mut(pct2).style.height = Length::px(50.0);

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 2. Float Layout Benchmarks
// ===========================================================================

fn bench_float_left_simple(c: &mut Criterion) {
    c.bench_function("float/left_5_boxes", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                // Overflow hidden on root to establish BFC for floats
                doc.node_mut(vp).style.overflow_x = Overflow::Hidden;
                doc.node_mut(vp).style.overflow_y = Overflow::Hidden;
                for _ in 0..5 {
                    let id = add_sized_block(&mut doc, vp, 100.0, 80.0);
                    doc.node_mut(id).style.float = Float::Left;
                }
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_float_text_wrap(c: &mut Criterion) {
    c.bench_function("float/text_wrap", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                doc.node_mut(vp).style.overflow_x = Overflow::Hidden;
                doc.node_mut(vp).style.overflow_y = Overflow::Hidden;

                // Left-floated image-like box
                let float_box = add_sized_block(&mut doc, vp, 150.0, 150.0);
                doc.node_mut(float_box).style.float = Float::Left;
                doc.node_mut(float_box).style.margin_right = Length::px(10.0);

                // Paragraph that wraps around the float
                let para = add_block(&mut doc, vp);
                add_text(&mut doc, para, "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                    Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                    Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.");

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_float_complex(c: &mut Criterion) {
    c.bench_function("float/complex_20_alternating", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                doc.node_mut(vp).style.overflow_x = Overflow::Hidden;
                doc.node_mut(vp).style.overflow_y = Overflow::Hidden;
                for i in 0..20 {
                    let id = add_sized_block(&mut doc, vp, 80.0, 60.0);
                    doc.node_mut(id).style.float = if i % 2 == 0 {
                        Float::Left
                    } else {
                        Float::Right
                    };
                    doc.node_mut(id).style.margin_top = Length::px(5.0);
                    doc.node_mut(id).style.margin_bottom = Length::px(5.0);
                }
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 3. Position Layout Benchmarks
// ===========================================================================

fn bench_absolute_positioning(c: &mut Criterion) {
    c.bench_function("position/absolute_10_children", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                // Containing block with position: relative
                let container = add_sized_block(&mut doc, vp, 600.0, 400.0);
                doc.node_mut(container).style.position = Position::Relative;

                for i in 0..10 {
                    let id = add_block(&mut doc, container);
                    doc.node_mut(id).style.position = Position::Absolute;
                    doc.node_mut(id).style.width = Length::px(80.0);
                    doc.node_mut(id).style.height = Length::px(60.0);
                    doc.node_mut(id).style.top = Length::px(i as f32 * 40.0);
                    doc.node_mut(id).style.left = Length::px(i as f32 * 50.0);
                }
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_relative_offsets(c: &mut Criterion) {
    c.bench_function("position/relative_20_blocks", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                for i in 0..20 {
                    let id = add_sized_block(&mut doc, vp, 200.0, 30.0);
                    doc.node_mut(id).style.position = Position::Relative;
                    doc.node_mut(id).style.top = Length::px(((i % 5) as f32) * 2.0);
                    doc.node_mut(id).style.left = Length::px(((i % 3) as f32) * 5.0);
                }
                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 4. Complex Layout Benchmarks
// ===========================================================================

fn bench_blog_layout(c: &mut Criterion) {
    c.bench_function("complex/blog_layout", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                doc.node_mut(vp).style.overflow_x = Overflow::Hidden;
                doc.node_mut(vp).style.overflow_y = Overflow::Hidden;

                // Header
                let header = add_sized_block(&mut doc, vp, 800.0, 60.0);
                doc.node_mut(header).style.background_color = Color::from_rgba8(51, 51, 51, 255);

                // Main wrapper
                let main = add_block(&mut doc, vp);
                doc.node_mut(main).style.width = Length::px(800.0);

                // Sidebar (floated left)
                let sidebar = add_sized_block(&mut doc, main, 200.0, 500.0);
                doc.node_mut(sidebar).style.float = Float::Left;
                doc.node_mut(sidebar).style.margin_right = Length::px(20.0);

                // Add sidebar items
                for _ in 0..5 {
                    let item = add_sized_block(&mut doc, sidebar, 180.0, 40.0);
                    doc.node_mut(item).style.margin_bottom = Length::px(10.0);
                }

                // Content area
                let content = add_block(&mut doc, main);
                doc.node_mut(content).style.margin_left = Length::px(220.0);

                // Article paragraphs
                for _ in 0..8 {
                    let para = add_block(&mut doc, content);
                    doc.node_mut(para).style.margin_bottom = Length::px(16.0);
                    add_text(&mut doc, para,
                        "Sed ut perspiciatis unde omnis iste natus error sit voluptatem \
                         accusantium doloremque laudantium, totam rem aperiam.");
                }

                // Footer
                let footer = add_sized_block(&mut doc, vp, 800.0, 40.0);
                doc.node_mut(footer).style.margin_top = Length::px(20.0);
                doc.node_mut(footer).style.background_color = Color::from_rgba8(51, 51, 51, 255);

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_form_layout(c: &mut Criterion) {
    c.bench_function("complex/form_layout", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let form = add_block(&mut doc, vp);
                doc.node_mut(form).style.width = Length::px(400.0);
                doc.node_mut(form).style.padding_top = Length::px(20.0);
                doc.node_mut(form).style.padding_right = Length::px(20.0);
                doc.node_mut(form).style.padding_bottom = Length::px(20.0);
                doc.node_mut(form).style.padding_left = Length::px(20.0);

                // 10 labeled input rows
                for _ in 0..10 {
                    let row = add_block(&mut doc, form);
                    doc.node_mut(row).style.margin_bottom = Length::px(12.0);
                    doc.node_mut(row).style.height = Length::px(30.0);

                    // Label (inline)
                    let label = add_inline(&mut doc, row);
                    add_text(&mut doc, label, "Field label:");

                    // Input-like box
                    let input = add_sized_block(&mut doc, row, 250.0, 28.0);
                    doc.node_mut(input).style.border_top_width = 1;
                    doc.node_mut(input).style.border_right_width = 1;
                    doc.node_mut(input).style.border_bottom_width = 1;
                    doc.node_mut(input).style.border_left_width = 1;
                    doc.node_mut(input).style.border_top_style = BorderStyle::Solid;
                    doc.node_mut(input).style.border_right_style = BorderStyle::Solid;
                    doc.node_mut(input).style.border_bottom_style = BorderStyle::Solid;
                    doc.node_mut(input).style.border_left_style = BorderStyle::Solid;
                }

                // Submit button
                let btn = add_sized_block(&mut doc, form, 120.0, 36.0);
                doc.node_mut(btn).style.margin_top = Length::px(16.0);
                doc.node_mut(btn).style.background_color = Color::BLUE;

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_grid_of_cards(c: &mut Criterion) {
    c.bench_function("complex/4x4_card_grid", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                doc.node_mut(vp).style.overflow_x = Overflow::Hidden;
                doc.node_mut(vp).style.overflow_y = Overflow::Hidden;

                let container = add_block(&mut doc, vp);
                doc.node_mut(container).style.width = Length::px(800.0);

                // 4x4 grid using floated cards
                for _ in 0..16 {
                    let card = add_sized_block(&mut doc, container, 180.0, 240.0);
                    doc.node_mut(card).style.float = Float::Left;
                    doc.node_mut(card).style.margin_right = Length::px(10.0);
                    doc.node_mut(card).style.margin_bottom = Length::px(10.0);

                    // Card header
                    let card_header = add_sized_block(&mut doc, card, 180.0, 40.0);
                    doc.node_mut(card_header).style.background_color = Color::from_rgba8(0, 120, 215, 255);

                    // Card body text
                    let card_body = add_block(&mut doc, card);
                    doc.node_mut(card_body).style.padding_top = Length::px(8.0);
                    doc.node_mut(card_body).style.padding_right = Length::px(8.0);
                    doc.node_mut(card_body).style.padding_bottom = Length::px(8.0);
                    doc.node_mut(card_body).style.padding_left = Length::px(8.0);
                    add_text(&mut doc, card_body, "Card content here with some description.");

                    // Card footer
                    add_sized_block(&mut doc, card, 180.0, 30.0);
                }

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 5. Flex Layout Benchmarks
// ===========================================================================

fn bench_flex_row_5_items(c: &mut Criterion) {
    c.bench_function("flex/row_5_items", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let flex = add_block(&mut doc, vp);
                doc.node_mut(flex).style.display = Display::Flex;
                doc.node_mut(flex).style.flex_direction = FlexDirection::Row;
                doc.node_mut(flex).style.width = Length::px(600.0);
                doc.node_mut(flex).style.height = Length::px(100.0);

                for _ in 0..5 {
                    let item = add_block(&mut doc, flex);
                    doc.node_mut(item).style.flex_grow = 1.0;
                    doc.node_mut(item).style.height = Length::px(80.0);
                }

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_flex_column_10_items(c: &mut Criterion) {
    c.bench_function("flex/column_10_items", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let flex = add_block(&mut doc, vp);
                doc.node_mut(flex).style.display = Display::Flex;
                doc.node_mut(flex).style.flex_direction = FlexDirection::Column;
                doc.node_mut(flex).style.width = Length::px(400.0);
                doc.node_mut(flex).style.height = Length::px(600.0);

                for _ in 0..10 {
                    let item = add_block(&mut doc, flex);
                    doc.node_mut(item).style.flex_grow = 1.0;
                    doc.node_mut(item).style.width = Length::px(380.0);
                }

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_flex_wrap_20_items(c: &mut Criterion) {
    c.bench_function("flex/wrap_20_items", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let flex = add_block(&mut doc, vp);
                doc.node_mut(flex).style.display = Display::Flex;
                doc.node_mut(flex).style.flex_direction = FlexDirection::Row;
                doc.node_mut(flex).style.flex_wrap = FlexWrap::Wrap;
                doc.node_mut(flex).style.width = Length::px(500.0);

                for _ in 0..20 {
                    let item = add_block(&mut doc, flex);
                    doc.node_mut(item).style.width = Length::px(120.0);
                    doc.node_mut(item).style.height = Length::px(80.0);
                    doc.node_mut(item).style.margin_right = Length::px(5.0);
                    doc.node_mut(item).style.margin_bottom = Length::px(5.0);
                }

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 6. Inline Layout Benchmarks
// ===========================================================================

fn bench_inline_short_text(c: &mut Criterion) {
    c.bench_function("inline/short_text", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(400.0);
                add_text(&mut doc, para, "Hello, world!");

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_inline_long_paragraph(c: &mut Criterion) {
    c.bench_function("inline/long_paragraph", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(600.0);
                add_text(&mut doc, para,
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                     Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                     Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris \
                     nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in \
                     reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla \
                     pariatur. Excepteur sint occaecat cupidatat non proident, sunt in \
                     culpa qui officia deserunt mollit anim id est laborum.");

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_inline_mixed_spans(c: &mut Criterion) {
    c.bench_function("inline/mixed_spans_10", |b| {
        b.iter_batched(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(500.0);

                for i in 0..10 {
                    let span = add_inline(&mut doc, para);
                    if i % 3 == 0 {
                        doc.node_mut(span).style.font_weight = FontWeight::BOLD;
                    }
                    add_text(&mut doc, span, "Inline span content. ");
                }

                doc
            },
            |doc| block_layout(&doc, doc.root(), &root_space()),
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// Criterion groups and main
// ===========================================================================

criterion_group!(
    block_benches,
    bench_single_block,
    bench_10_stacked_blocks,
    bench_100_stacked_blocks,
    bench_deep_nesting_10,
    bench_deep_nesting_50,
    bench_margin_collapsing,
    bench_mixed_sizing,
);

criterion_group!(
    float_benches,
    bench_float_left_simple,
    bench_float_text_wrap,
    bench_float_complex,
);

criterion_group!(
    position_benches,
    bench_absolute_positioning,
    bench_relative_offsets,
);

criterion_group!(
    complex_benches,
    bench_blog_layout,
    bench_form_layout,
    bench_grid_of_cards,
);

criterion_group!(
    flex_benches,
    bench_flex_row_5_items,
    bench_flex_column_10_items,
    bench_flex_wrap_20_items,
);

criterion_group!(
    inline_benches,
    bench_inline_short_text,
    bench_inline_long_paragraph,
    bench_inline_mixed_spans,
);

criterion_main!(
    block_benches,
    float_benches,
    position_benches,
    complex_benches,
    flex_benches,
    inline_benches,
);
