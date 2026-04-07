//! Criterion benchmarks for Open UI paint performance.
//!
//! Measures text painting, decoration painting, border rendering, and full-page
//! compositing to compare with Blink's paint times.

use criterion::{criterion_group, criterion_main, Criterion};
use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::Length;
use openui_paint::render_to_surface;
use openui_style::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Create a text node and append it to `parent`.
fn add_text(doc: &mut Document, parent: NodeId, text: &str) -> NodeId {
    let id = doc.create_node(ElementTag::Text);
    doc.node_mut(id).style.display = Display::Inline;
    doc.node_mut(id).text = Some(text.to_string());
    doc.append_child(parent, id);
    id
}

/// Apply solid border to all four sides of a node.
fn apply_solid_border(doc: &mut Document, id: NodeId, width: i32, color: Color) {
    let s = &mut doc.node_mut(id).style;
    s.border_top_width = width;
    s.border_right_width = width;
    s.border_bottom_width = width;
    s.border_left_width = width;
    s.border_top_style = BorderStyle::Solid;
    s.border_right_style = BorderStyle::Solid;
    s.border_bottom_style = BorderStyle::Solid;
    s.border_left_style = BorderStyle::Solid;
    s.border_top_color = StyleColor::Resolved(color);
    s.border_right_color = StyleColor::Resolved(color);
    s.border_bottom_color = StyleColor::Resolved(color);
    s.border_left_color = StyleColor::Resolved(color);
}

// ===========================================================================
// 1. Paint / Text Benchmarks
// ===========================================================================

fn bench_paint_text_simple(c: &mut Criterion) {
    c.bench_function("paint/text_simple", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(600.0);
                add_text(
                    &mut doc,
                    para,
                    "The quick brown fox jumps over the lazy dog.",
                );
                doc
            },
            |doc| {
                render_to_surface(doc, 800, 600).unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_paint_text_styled(c: &mut Criterion) {
    c.bench_function("paint/text_styled", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(600.0);

                // Bold, colored heading
                let heading = add_block(&mut doc, para);
                doc.node_mut(heading).style.font_weight = FontWeight::BOLD;
                doc.node_mut(heading).style.font_size = 24.0;
                doc.node_mut(heading).style.color = Color::from_rgba8(20, 80, 180, 255);
                add_text(&mut doc, heading, "Styled Heading");

                // Italic body text
                let body = add_block(&mut doc, para);
                doc.node_mut(body).style.font_style = FontStyleEnum::Italic;
                doc.node_mut(body).style.color = Color::from_rgba8(50, 50, 50, 255);
                add_text(
                    &mut doc,
                    body,
                    "This paragraph uses italic styling with a dark gray color.",
                );

                // Small colored note
                let note = add_block(&mut doc, para);
                doc.node_mut(note).style.font_size = 12.0;
                doc.node_mut(note).style.color = Color::from_rgba8(150, 100, 0, 255);
                add_text(&mut doc, note, "Note: colors applied via computed style.");

                doc
            },
            |doc| {
                render_to_surface(doc, 800, 600).unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 2. Paint / Decoration Benchmarks
// ===========================================================================

fn bench_paint_decoration_underline(c: &mut Criterion) {
    c.bench_function("paint/decoration_underline", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(600.0);
                doc.node_mut(para).style.text_decoration_line = TextDecorationLine::UNDERLINE;
                doc.node_mut(para).style.text_decoration_style = TextDecorationStyle::Solid;
                add_text(
                    &mut doc,
                    para,
                    "Underlined text for decoration benchmark testing.",
                );
                doc
            },
            |doc| {
                render_to_surface(doc, 800, 600).unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_paint_decoration_wavy(c: &mut Criterion) {
    c.bench_function("paint/decoration_wavy", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(600.0);
                doc.node_mut(para).style.text_decoration_line = TextDecorationLine::UNDERLINE;
                doc.node_mut(para).style.text_decoration_style = TextDecorationStyle::Wavy;
                doc.node_mut(para).style.text_decoration_color =
                    StyleColor::Resolved(Color::from_rgba8(200, 50, 50, 255));
                add_text(
                    &mut doc,
                    para,
                    "Wavy underline decoration — path-based rendering via Skia.",
                );
                doc
            },
            |doc| {
                render_to_surface(doc, 800, 600).unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 3. Paint / Emphasis Benchmarks
// ===========================================================================

fn bench_paint_emphasis_dot(c: &mut Criterion) {
    c.bench_function("paint/emphasis_dot", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                let para = add_block(&mut doc, vp);
                doc.node_mut(para).style.width = Length::px(600.0);
                doc.node_mut(para).style.text_emphasis_mark = TextEmphasisMark::Dot;
                doc.node_mut(para).style.text_emphasis_fill = TextEmphasisFill::Filled;
                add_text(&mut doc, para, "Text with emphasis dot marks above each character.");
                doc
            },
            |doc| {
                render_to_surface(doc, 800, 600).unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 4. Paint / Border Benchmarks
// ===========================================================================

fn bench_paint_border_solid(c: &mut Criterion) {
    c.bench_function("paint/border_solid", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                for _ in 0..5 {
                    let id = add_sized_block(&mut doc, vp, 200.0, 80.0);
                    doc.node_mut(id).style.background_color =
                        Color::from_rgba8(240, 240, 255, 255);
                    apply_solid_border(&mut doc, id, 2, Color::from_rgba8(0, 80, 200, 255));
                    doc.node_mut(id).style.margin_bottom = Length::px(8.0);
                }
                doc
            },
            |doc| {
                render_to_surface(doc, 800, 600).unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_paint_border_radius(c: &mut Criterion) {
    c.bench_function("paint/border_radius", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();
                for _ in 0..5 {
                    let id = add_sized_block(&mut doc, vp, 200.0, 80.0);
                    doc.node_mut(id).style.background_color =
                        Color::from_rgba8(230, 240, 255, 255);
                    apply_solid_border(&mut doc, id, 2, Color::from_rgba8(60, 60, 200, 255));
                    // Rounded corners
                    doc.node_mut(id).style.border_top_left_radius = (8.0, 8.0);
                    doc.node_mut(id).style.border_top_right_radius = (8.0, 8.0);
                    doc.node_mut(id).style.border_bottom_left_radius = (8.0, 8.0);
                    doc.node_mut(id).style.border_bottom_right_radius = (8.0, 8.0);
                    doc.node_mut(id).style.margin_bottom = Length::px(8.0);
                }
                doc
            },
            |doc| {
                render_to_surface(doc, 800, 600).unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 5. Paint / Full-Page Benchmark
// ===========================================================================

fn bench_paint_full_page(c: &mut Criterion) {
    c.bench_function("paint/full_page", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                // Header
                let header = add_sized_block(&mut doc, vp, 800.0, 60.0);
                doc.node_mut(header).style.background_color = Color::from_rgba8(30, 30, 80, 255);
                let htitle = add_block(&mut doc, header);
                doc.node_mut(htitle).style.color = Color::WHITE;
                doc.node_mut(htitle).style.font_size = 20.0;
                add_text(&mut doc, htitle, "Open UI Performance Test Page");

                // Content area with multiple paragraphs
                let content = add_block(&mut doc, vp);
                doc.node_mut(content).style.width = Length::px(760.0);
                doc.node_mut(content).style.padding_top = Length::px(10.0);
                doc.node_mut(content).style.padding_left = Length::px(20.0);

                for i in 0..8 {
                    let para = add_block(&mut doc, content);
                    doc.node_mut(para).style.margin_bottom = Length::px(12.0);
                    if i % 2 == 0 {
                        apply_solid_border(
                            &mut doc,
                            para,
                            1,
                            Color::from_rgba8(180, 180, 200, 255),
                        );
                        doc.node_mut(para).style.background_color =
                            Color::from_rgba8(248, 248, 255, 255);
                    }
                    add_text(
                        &mut doc,
                        para,
                        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
                    );
                }

                // Footer
                let footer = add_sized_block(&mut doc, vp, 800.0, 40.0);
                doc.node_mut(footer).style.background_color =
                    Color::from_rgba8(240, 240, 240, 255);
                apply_solid_border(&mut doc, footer, 1, Color::from_rgba8(180, 180, 180, 255));
                let ftext = add_block(&mut doc, footer);
                doc.node_mut(ftext).style.font_size = 12.0;
                doc.node_mut(ftext).style.color = Color::from_rgba8(100, 100, 100, 255);
                add_text(&mut doc, ftext, "© Open UI Project");

                doc
            },
            |doc| {
                render_to_surface(doc, 800, 600).unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// 6. Paint / PNG Encoding Benchmark
// ===========================================================================

fn bench_paint_render_to_png(c: &mut Criterion) {
    c.bench_function("paint/render_to_png", |b| {
        b.iter_batched_ref(
            || {
                let mut doc = Document::new();
                let vp = doc.root();

                // A representative page with text and colored boxes
                let header = add_sized_block(&mut doc, vp, 800.0, 50.0);
                doc.node_mut(header).style.background_color = Color::from_rgba8(50, 80, 150, 255);

                let body = add_block(&mut doc, vp);
                doc.node_mut(body).style.width = Length::px(780.0);
                doc.node_mut(body).style.padding_top = Length::px(10.0);
                doc.node_mut(body).style.padding_left = Length::px(10.0);
                for _ in 0..5 {
                    let row = add_sized_block(&mut doc, body, 760.0, 60.0);
                    doc.node_mut(row).style.background_color =
                        Color::from_rgba8(230, 235, 255, 255);
                    doc.node_mut(row).style.margin_bottom = Length::px(8.0);
                    apply_solid_border(&mut doc, row, 1, Color::from_rgba8(150, 160, 200, 255));
                    add_text(
                        &mut doc,
                        row,
                        "Row content with some text for PNG encoding benchmark.",
                    );
                }
                doc
            },
            |doc| {
                // Render to surface AND encode to PNG bytes (not written to disk)
                let mut surface = render_to_surface(doc, 800, 600).unwrap();
                let image = surface.image_snapshot();
                let _data = image.encode(
                    None,
                    skia_safe::EncodedImageFormat::PNG,
                    None,
                );
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// ===========================================================================
// Criterion groups and main
// ===========================================================================

criterion_group!(
    text_benches,
    bench_paint_text_simple,
    bench_paint_text_styled,
);

criterion_group!(
    decoration_benches,
    bench_paint_decoration_underline,
    bench_paint_decoration_wavy,
);

criterion_group!(
    emphasis_benches,
    bench_paint_emphasis_dot,
);

criterion_group!(
    border_benches,
    bench_paint_border_solid,
    bench_paint_border_radius,
);

criterion_group!(
    page_benches,
    bench_paint_full_page,
    bench_paint_render_to_png,
);

criterion_main!(
    text_benches,
    decoration_benches,
    emphasis_benches,
    border_benches,
    page_benches,
);
