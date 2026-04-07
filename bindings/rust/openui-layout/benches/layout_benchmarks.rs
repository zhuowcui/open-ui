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
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                add_sized_block(&mut doc, vp, 200.0, 100.0);
                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_10_stacked_blocks(c: &mut Criterion) {
    c.bench_function("block/10_stacked", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                for _ in 0..10 {
                    add_sized_block(&mut doc, vp, 200.0, 50.0);
                }
                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_100_stacked_blocks(c: &mut Criterion) {
    c.bench_function("block/100_stacked", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                for _ in 0..100 {
                    add_sized_block(&mut doc, vp, 200.0, 50.0);
                }
                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_deep_nesting_10(c: &mut Criterion) {
    c.bench_function("block/deep_nesting_10", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                let mut parent = vp;
                for _ in 0..10 {
                    parent = add_sized_block(&mut doc, parent, 400.0, 300.0);
                }
                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_deep_nesting_50(c: &mut Criterion) {
    c.bench_function("block/deep_nesting_50", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                let mut parent = vp;
                for _ in 0..50 {
                    parent = add_sized_block(&mut doc, parent, 400.0, 300.0);
                }
                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_margin_collapsing(c: &mut Criterion) {
    c.bench_function("block/margin_collapsing_20", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_mixed_sizing(c: &mut Criterion) {
    c.bench_function("block/mixed_sizing", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 2. Float Layout Benchmarks
// ===========================================================================

fn bench_float_left_simple(c: &mut Criterion) {
    c.bench_function("float/left_5_boxes", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_float_text_wrap(c: &mut Criterion) {
    c.bench_function("float/text_wrap", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_float_complex(c: &mut Criterion) {
    c.bench_function("float/complex_20_alternating", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 3. Position Layout Benchmarks
// ===========================================================================

fn bench_absolute_positioning(c: &mut Criterion) {
    c.bench_function("position/absolute_10_children", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_relative_offsets(c: &mut Criterion) {
    c.bench_function("position/relative_20_blocks", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 4. Complex Layout Benchmarks
// ===========================================================================

fn bench_blog_layout(c: &mut Criterion) {
    c.bench_function("complex/blog_layout", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_form_layout(c: &mut Criterion) {
    c.bench_function("complex/form_layout", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_grid_of_cards(c: &mut Criterion) {
    c.bench_function("complex/4x4_card_grid", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 5. Flex Layout Benchmarks
// ===========================================================================

fn bench_flex_row_5_items(c: &mut Criterion) {
    c.bench_function("flex/row_5_items", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_flex_column_10_items(c: &mut Criterion) {
    c.bench_function("flex/column_10_items", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_flex_wrap_20_items(c: &mut Criterion) {
    c.bench_function("flex/wrap_20_items", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 6. Inline Layout Benchmarks
// ===========================================================================

fn bench_inline_short_text(c: &mut Criterion) {
    c.bench_function("inline/short_text", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(400.0);
                add_text(&mut doc, para, "Hello, world!");

                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_inline_long_paragraph(c: &mut Criterion) {
    c.bench_function("inline/long_paragraph", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_inline_mixed_spans(c: &mut Criterion) {
    c.bench_function("inline/mixed_spans_10", |b| {
        b.iter_batched_ref(
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
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 7. Sizing Benchmarks
// ===========================================================================

fn bench_sizing_min_content_nested(c: &mut Criterion) {
    c.bench_function("sizing/min_content_nested", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                // 5 levels of nested blocks, each auto-width, to exercise
                // min-content intrinsic sizing propagation.
                let mut parent = vp;
                for _ in 0..5 {
                    let child = add_block(&mut doc, parent);
                    doc.node_mut(child).style.padding_top = Length::px(4.0);
                    doc.node_mut(child).style.padding_right = Length::px(4.0);
                    doc.node_mut(child).style.padding_bottom = Length::px(4.0);
                    doc.node_mut(child).style.padding_left = Length::px(4.0);
                    parent = child;
                }
                add_text(&mut doc, parent, "Min-content sizing text inside nested blocks");

                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_sizing_max_content_text(c: &mut Criterion) {
    c.bench_function("sizing/max_content_text", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                // Auto-width block containing a paragraph — exercises
                // max-content sizing when the container has no explicit width.
                let para = add_block(&mut doc, vp);
                add_text(&mut doc, para,
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                     Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                     Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.");

                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_sizing_min_max_constraints(c: &mut Criterion) {
    c.bench_function("sizing/min_max_constraints", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let block = add_block(&mut doc, vp);
                doc.node_mut(block).style.width = Length::percent(50.0);
                doc.node_mut(block).style.min_width = Length::px(200.0);
                doc.node_mut(block).style.max_width = Length::px(500.0);
                doc.node_mut(block).style.height = Length::percent(40.0);
                doc.node_mut(block).style.min_height = Length::px(100.0);
                doc.node_mut(block).style.max_height = Length::px(300.0);
                add_text(&mut doc, block,
                    "Content inside a block with min-width, max-width, \
                     min-height, and max-height constraints applied.");

                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 8. Multi-Column Benchmarks
// ===========================================================================

fn bench_multicol_3_columns_20_blocks(c: &mut Criterion) {
    c.bench_function("multicol/3_columns_20_blocks", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let multicol = add_block(&mut doc, vp);
                doc.node_mut(multicol).style.width = Length::px(600.0);
                doc.node_mut(multicol).style.column_count = Some(3);

                for _ in 0..20 {
                    let child = add_block(&mut doc, multicol);
                    doc.node_mut(child).style.height = Length::px(30.0);
                    doc.node_mut(child).style.margin_bottom = Length::px(8.0);
                }

                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 9. Fragmentation Benchmarks
// ===========================================================================

fn bench_fragmentation_break_token_chain(c: &mut Criterion) {
    c.bench_function("fragmentation/break_token_chain", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                // 10 blocks with alternating break-before/break-after values
                // to stress fragmentation break-token chain logic.
                for i in 0..10 {
                    let child = add_sized_block(&mut doc, vp, 400.0, 50.0);
                    if i % 2 == 0 {
                        doc.node_mut(child).style.break_after = BreakValue::Column;
                    } else {
                        doc.node_mut(child).style.break_before = BreakValue::Column;
                    }
                }

                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 10. Text / Inline Stress Benchmarks
// ===========================================================================

fn bench_text_inline_long_paragraph(c: &mut Criterion) {
    c.bench_function("text/inline_long_paragraph", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(500.0);

                // ~200 words of lorem ipsum text
                add_text(&mut doc, para,
                    "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do \
                     eiusmod tempor incididunt ut labore et dolore magna aliqua Ut enim \
                     ad minim veniam quis nostrud exercitation ullamco laboris nisi ut \
                     aliquip ex ea commodo consequat Duis aute irure dolor in reprehenderit \
                     in voluptate velit esse cillum dolore eu fugiat nulla pariatur Excepteur \
                     sint occaecat cupidatat non proident sunt in culpa qui officia deserunt \
                     mollit anim id est laborum Sed ut perspiciatis unde omnis iste natus \
                     error sit voluptatem accusantium doloremque laudantium totam rem aperiam \
                     eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae \
                     vitae dicta sunt explicabo Nemo enim ipsam voluptatem quia voluptas sit \
                     aspernatur aut odit aut fugit sed quia consequuntur magni dolores eos \
                     qui ratione voluptatem sequi nesciunt Neque porro quisquam est qui dolorem \
                     ipsum quia dolor sit amet consectetur adipisci velit sed quia non numquam \
                     eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat \
                     voluptatem Ut enim ad minima veniam quis nostrum exercitationem ullam \
                     corporis suscipit laboriosam nisi ut aliquid ex ea commodi consequatur \
                     Quis autem vel eum iure reprehenderit qui in ea voluptate velit esse \
                     quam nihil molestiae consequatur vel illum qui dolorem eum fugiat quo \
                     voluptas nulla pariatur");

                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_text_inline_mixed_bidi(c: &mut Criterion) {
    c.bench_function("text/inline_mixed_bidi", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(400.0);

                // LTR text
                let ltr = add_inline(&mut doc, para);
                add_text(&mut doc, ltr, "English text before ");

                // RTL span
                let rtl = add_inline(&mut doc, para);
                doc.node_mut(rtl).style.direction = Direction::Rtl;
                doc.node_mut(rtl).style.unicode_bidi = UnicodeBidi::Embed;
                add_text(&mut doc, rtl, "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627} \u{0628}\u{0627}\u{0644}\u{0639}\u{0627}\u{0644}\u{0645}");

                // Back to LTR
                let ltr2 = add_inline(&mut doc, para);
                add_text(&mut doc, ltr2, " and English after ");

                // Another RTL span
                let rtl2 = add_inline(&mut doc, para);
                doc.node_mut(rtl2).style.direction = Direction::Rtl;
                doc.node_mut(rtl2).style.unicode_bidi = UnicodeBidi::Embed;
                add_text(&mut doc, rtl2, "\u{0634}\u{0643}\u{0631}\u{0627}");

                let end = add_inline(&mut doc, para);
                add_text(&mut doc, end, " end of line.");

                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_text_inline_line_breaking_stress(c: &mut Criterion) {
    c.bench_function("text/inline_line_breaking_stress", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                // Very narrow container to force many line breaks
                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(80.0);

                // 50 short words — each 3-6 chars — crammed into 80px
                let words: Vec<&str> = vec![
                    "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
                    "and", "then", "runs", "back", "again", "with", "great", "speed",
                    "down", "long", "road", "past", "old", "farm", "near", "big",
                    "red", "barn", "next", "wide", "blue", "lake", "into", "deep",
                    "dark", "wood", "full", "tall", "pine", "trees", "soft", "green",
                    "moss", "grew", "upon", "each", "flat", "gray", "rock", "all",
                    "day", "long",
                ];
                add_text(&mut doc, para, &words.join(" "));

                doc
            },
            |doc| { block_layout(doc, doc.root(), &root_space()); },
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

criterion_group!(
    sizing_benches,
    bench_sizing_min_content_nested,
    bench_sizing_max_content_text,
    bench_sizing_min_max_constraints,
);

criterion_group!(
    multicol_benches,
    bench_multicol_3_columns_20_blocks,
);

criterion_group!(
    fragmentation_benches,
    bench_fragmentation_break_token_chain,
);

criterion_group!(
    text_benches,
    bench_text_inline_long_paragraph,
    bench_text_inline_mixed_bidi,
    bench_text_inline_line_breaking_stress,
);

criterion_main!(
    block_benches,
    float_benches,
    position_benches,
    complex_benches,
    flex_benches,
    inline_benches,
    sizing_benches,
    multicol_benches,
    fragmentation_benches,
    text_benches,
);
