//! pixel_compare — Renders test patterns to PNG for comparison with Chromium.
//!
//! Usage:
//!   pixel_compare list                              # List all test IDs
//!   pixel_compare render <test_id> <output.png>     # Render one test to PNG
//!   pixel_compare render-all <output_dir>           # Render all tests
//!
//! Each test ID corresponds to a specific CSS feature variant that has
//! a matching HTML file rendered by Chrome for pixel-by-pixel comparison.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::Length;
use openui_paint::render_to_png;
use openui_style::*;

const W: i32 = 800;
const H: i32 = 600;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("list") => {
            for (id, _) in registry() {
                println!("{}", id);
            }
        }
        Some("render") => {
            let test_id = args.get(2).expect("Usage: pixel_compare render <test_id> <output.png>");
            let output = args.get(3).expect("Usage: pixel_compare render <test_id> <output.png>");
            render_test(test_id, output);
        }
        Some("render-all") => {
            let dir = args.get(2).expect("Usage: pixel_compare render-all <output_dir>");
            std::fs::create_dir_all(dir).unwrap();
            for (id, _) in registry() {
                let path = format!("{}/{}.png", dir, id);
                render_test(id, &path);
            }
            println!("Rendered {} tests to {}/", registry().len(), dir);
        }
        _ => {
            eprintln!("Usage: pixel_compare <list|render|render-all> ...");
            std::process::exit(1);
        }
    }
}

fn render_test(test_id: &str, output: &str) {
    let tests = registry();
    if let Some((_, builder)) = tests.iter().find(|(id, _)| *id == test_id) {
        let doc = builder();
        render_to_png(&doc, W, H, output).expect("render failed");
        println!("OK: {} → {}", test_id, output);
    } else {
        eprintln!("Unknown test ID: {}", test_id);
        eprintln!("Use 'pixel_compare list' to see available tests.");
        std::process::exit(1);
    }
}

type TestBuilder = fn() -> Document;

/// Returns the full registry of test IDs → Document builders.
fn registry() -> Vec<(&'static str, TestBuilder)> {
    vec![
        // ── SP12 Display ─────────────────────────────────────────────
        ("sp12/display_outer_block", sp12_display_outer_block as TestBuilder),
        ("sp12/display_outer_inline", sp12_display_outer_inline),
        ("sp12/display_outer_inline_block", sp12_display_outer_inline_block),
        ("sp12/display_outer_none", sp12_display_outer_none),
        ("sp12/display_inner_flow_root", sp12_display_inner_flow_root),

        // ── SP12 Position ────────────────────────────────────────────
        ("sp12/position_static", sp12_position_static),
        ("sp12/position_relative", sp12_position_relative),
        ("sp12/position_absolute", sp12_position_absolute),
        ("sp12/position_fixed", sp12_position_fixed),

        // ── SP12 Float ───────────────────────────────────────────────
        ("sp12/float_left", sp12_float_left),
        ("sp12/float_right", sp12_float_right),
        ("sp12/float_none", sp12_float_none),
        ("sp12/clear_left", sp12_clear_left),
        ("sp12/clear_right", sp12_clear_right),
        ("sp12/clear_both", sp12_clear_both),

        // ── SP12 Box Model ───────────────────────────────────────────
        ("sp12/margin_positive", sp12_margin_positive),
        ("sp12/margin_negative", sp12_margin_negative),
        ("sp12/margin_auto", sp12_margin_auto),
        ("sp12/margin_collapsing_siblings", sp12_margin_collapsing_siblings),
        ("sp12/padding_basic", sp12_padding_basic),
        ("sp12/border_basic", sp12_border_basic),
        ("sp12/box_sizing_content_box", sp12_box_sizing_content_box),
        ("sp12/box_sizing_border_box", sp12_box_sizing_border_box),

        // ── SP12 Sizing ──────────────────────────────────────────────
        ("sp12/width_fixed_px", sp12_width_fixed_px),
        ("sp12/height_fixed_px", sp12_height_fixed_px),
        ("sp12/width_percent", sp12_width_percent),
        ("sp12/min_width", sp12_min_width),
        ("sp12/max_width", sp12_max_width),
        ("sp12/min_height", sp12_min_height),
        ("sp12/max_height", sp12_max_height),

        // ── SP12 Overflow ────────────────────────────────────────────
        ("sp12/overflow_visible", sp12_overflow_visible),
        ("sp12/overflow_hidden", sp12_overflow_hidden),
    ]
}

// ═══════════════════════════════════════════════════════════════════════
// ── Helpers ─────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Create a Document with viewport padding and default text styles,
/// matching the HTML test file defaults:
///   body { margin: 0; padding: 20px; font-family: DejaVu Sans; font-size: 16px; }
fn base_doc() -> (Document, NodeId) {
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.display = Display::Block;
    doc.node_mut(vp).style.background_color = Color::WHITE;
    doc.node_mut(vp).style.padding_top = Length::px(20.0);
    doc.node_mut(vp).style.padding_right = Length::px(20.0);
    doc.node_mut(vp).style.padding_bottom = Length::px(20.0);
    doc.node_mut(vp).style.padding_left = Length::px(20.0);
    doc.node_mut(vp).style.font_family = FontFamilyList::single("DejaVu Sans");
    doc.node_mut(vp).style.font_size = 16.0;
    doc.node_mut(vp).style.color = Color::BLACK;
    (doc, vp)
}

fn add_block(doc: &mut Document, parent: NodeId, w: f32, h: f32, color: Color) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(w);
    doc.node_mut(div).style.height = Length::px(h);
    doc.node_mut(div).style.background_color = color;
    doc.append_child(parent, div);
    div
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Display Tests ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_display_outer_block() -> Document {
    let (mut doc, vp) = base_doc();
    add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc
}

fn sp12_display_outer_inline() -> Document {
    let (mut doc, vp) = base_doc();
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.background_color = Color::from_rgba8(0, 128, 0, 255);
    doc.node_mut(span).text = Some("Inline element text".to_string());
    doc.append_child(vp, span);
    doc
}

fn sp12_display_outer_inline_block() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::InlineBlock;
    doc.node_mut(div).style.width = Length::px(150.0);
    doc.node_mut(div).style.height = Length::px(80.0);
    doc.node_mut(div).style.background_color = Color::BLUE;
    doc.append_child(vp, div);
    doc
}

fn sp12_display_outer_none() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::None;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.append_child(vp, div);
    doc
}

fn sp12_display_inner_flow_root() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::FlowRoot;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 128, 128, 255);
    doc.append_child(vp, div);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Position Tests ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_position_static() -> Document {
    let (mut doc, vp) = base_doc();
    add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    add_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    doc
}

fn sp12_position_relative() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::BLUE);
    doc.node_mut(div).style.position = Position::Relative;
    doc.node_mut(div).style.top = Length::px(20.0);
    doc.node_mut(div).style.left = Length::px(30.0);
    doc
}

fn sp12_position_absolute() -> Document {
    let (mut doc, vp) = base_doc();
    doc.node_mut(vp).style.position = Position::Relative;
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::RED);
    doc.node_mut(div).style.position = Position::Absolute;
    doc.node_mut(div).style.top = Length::px(50.0);
    doc.node_mut(div).style.left = Length::px(50.0);
    doc
}

fn sp12_position_fixed() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::RED);
    doc.node_mut(div).style.position = Position::Fixed;
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.right = Length::px(10.0);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Float Tests ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_float_left() -> Document {
    let (mut doc, vp) = base_doc();
    let f = add_block(&mut doc, vp, 100.0, 100.0, Color::from_rgba8(0, 128, 0, 255));
    doc.node_mut(f).style.float = Float::Left;
    let txt = doc.create_node(ElementTag::Span);
    doc.node_mut(txt).text = Some("Text wrapping around a left-floated element. The text should flow to the right of the green box.".to_string());
    doc.append_child(vp, txt);
    doc
}

fn sp12_float_right() -> Document {
    let (mut doc, vp) = base_doc();
    let f = add_block(&mut doc, vp, 100.0, 100.0, Color::from_rgba8(0, 128, 0, 255));
    doc.node_mut(f).style.float = Float::Right;
    let txt = doc.create_node(ElementTag::Span);
    doc.node_mut(txt).text = Some("Text wrapping around a right-floated element. The text should flow to the left of the green box.".to_string());
    doc.append_child(vp, txt);
    doc
}

fn sp12_float_none() -> Document {
    let (mut doc, vp) = base_doc();
    let f = add_block(&mut doc, vp, 100.0, 100.0, Color::from_rgba8(0, 128, 0, 255));
    doc.node_mut(f).style.float = Float::None;
    doc
}

fn sp12_clear_left() -> Document {
    let (mut doc, vp) = base_doc();
    let f = add_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    doc.node_mut(f).style.float = Float::Left;
    let div = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.clear = Clear::Left;
    doc
}

fn sp12_clear_right() -> Document {
    let (mut doc, vp) = base_doc();
    let f = add_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    doc.node_mut(f).style.float = Float::Right;
    let div = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.clear = Clear::Right;
    doc
}

fn sp12_clear_both() -> Document {
    let (mut doc, vp) = base_doc();
    let fl = add_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    doc.node_mut(fl).style.float = Float::Left;
    let fr = add_block(&mut doc, vp, 100.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc.node_mut(fr).style.float = Float::Right;
    let div = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.clear = Clear::Both;
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Box Model Tests ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_margin_positive() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::RED);
    doc.node_mut(div).style.margin_top = Length::px(20.0);
    doc.node_mut(div).style.margin_right = Length::px(20.0);
    doc.node_mut(div).style.margin_bottom = Length::px(20.0);
    doc.node_mut(div).style.margin_left = Length::px(20.0);
    // Reference: second box to show spacing
    add_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc
}

fn sp12_margin_negative() -> Document {
    let (mut doc, vp) = base_doc();
    add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    let div2 = add_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    doc.node_mut(div2).style.margin_top = Length::px(-30.0);
    doc
}

fn sp12_margin_auto() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    doc
}

fn sp12_margin_collapsing_siblings() -> Document {
    let (mut doc, vp) = base_doc();
    let a = add_block(&mut doc, vp, 200.0, 50.0, Color::RED);
    doc.node_mut(a).style.margin_bottom = Length::px(30.0);
    let b = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.node_mut(b).style.margin_top = Length::px(20.0);
    // Collapsed margin = max(30, 20) = 30px gap
    doc
}

fn sp12_padding_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.node_mut(outer).style.padding_top = Length::px(20.0);
    doc.node_mut(outer).style.padding_right = Length::px(30.0);
    doc.node_mut(outer).style.padding_bottom = Length::px(20.0);
    doc.node_mut(outer).style.padding_left = Length::px(30.0);
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 100.0, 60.0, Color::RED);
    doc
}

fn sp12_border_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 150.0, 100.0, Color::from_rgba8(240, 240, 240, 255));
    doc.node_mut(div).style.border_top_width = 3.0 as i32;
    doc.node_mut(div).style.border_right_width = 3.0 as i32;
    doc.node_mut(div).style.border_bottom_width = 3.0 as i32;
    doc.node_mut(div).style.border_left_width = 3.0 as i32;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc
}

fn sp12_box_sizing_content_box() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.node_mut(div).style.box_sizing = BoxSizing::ContentBox;
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_right = Length::px(10.0);
    doc.node_mut(div).style.padding_bottom = Length::px(10.0);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    doc.node_mut(div).style.border_top_width = 2.0 as i32;
    doc.node_mut(div).style.border_right_width = 2.0 as i32;
    doc.node_mut(div).style.border_bottom_width = 2.0 as i32;
    doc.node_mut(div).style.border_left_width = 2.0 as i32;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    // Total width: 200 + 20 + 4 = 224px (content-box)
    doc
}

fn sp12_box_sizing_border_box() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.node_mut(div).style.box_sizing = BoxSizing::BorderBox;
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_right = Length::px(10.0);
    doc.node_mut(div).style.padding_bottom = Length::px(10.0);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    doc.node_mut(div).style.border_top_width = 2.0 as i32;
    doc.node_mut(div).style.border_right_width = 2.0 as i32;
    doc.node_mut(div).style.border_bottom_width = 2.0 as i32;
    doc.node_mut(div).style.border_left_width = 2.0 as i32;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    // Total width: 200px exactly (border-box, content = 200-20-4 = 176px)
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Sizing Tests ───────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_width_fixed_px() -> Document {
    let (mut doc, vp) = base_doc();
    add_block(&mut doc, vp, 300.0, 100.0, Color::RED);
    doc
}

fn sp12_height_fixed_px() -> Document {
    let (mut doc, vp) = base_doc();
    add_block(&mut doc, vp, 200.0, 150.0, Color::BLUE);
    doc
}

fn sp12_width_percent() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::percent(50.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.append_child(vp, div);
    doc
}

fn sp12_min_width() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(50.0);
    doc.node_mut(div).style.min_width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.append_child(vp, div);
    doc
}

fn sp12_max_width() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(500.0);
    doc.node_mut(div).style.max_width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.append_child(vp, div);
    doc
}

fn sp12_min_height() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(30.0);
    doc.node_mut(div).style.min_height = Length::px(100.0);
    doc.node_mut(div).style.background_color = Color::BLUE;
    doc.append_child(vp, div);
    doc
}

fn sp12_max_height() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(500.0);
    doc.node_mut(div).style.max_height = Length::px(100.0);
    doc.node_mut(div).style.background_color = Color::BLUE;
    doc.append_child(vp, div);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Overflow Tests ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_overflow_visible() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(100.0);
    doc.node_mut(outer).style.height = Length::px(50.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Visible;
    doc.node_mut(outer).style.overflow_y = Overflow::Visible;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 200.0, 200.0, Color::RED);
    doc
}

fn sp12_overflow_hidden() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(100.0);
    doc.node_mut(outer).style.height = Length::px(50.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Hidden;
    doc.node_mut(outer).style.overflow_y = Overflow::Hidden;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 200.0, 200.0, Color::RED);
    doc
}
