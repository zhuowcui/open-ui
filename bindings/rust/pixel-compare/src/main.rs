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

mod wpt;

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
    let mut tests = vec![
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

        // ── SP12 Flexbox ─────────────────────────────────────────────
        ("sp12/flex_direction_row", sp12_flex_direction_row),
        ("sp12/flex_direction_column", sp12_flex_direction_column),
        ("sp12/flex_justify_start", sp12_flex_justify_start),
        ("sp12/flex_justify_center", sp12_flex_justify_center),
        ("sp12/flex_justify_space_between", sp12_flex_justify_space_between),
        ("sp12/flex_align_center", sp12_flex_align_center),
        ("sp12/flex_align_stretch", sp12_flex_align_stretch),
        ("sp12/flex_wrap_basic", sp12_flex_wrap_basic),
        ("sp12/flex_grow_equal", sp12_flex_grow_equal),
        ("sp12/flex_gap", sp12_flex_gap),

        // ── SP12 Visual / Stacking ───────────────────────────────────
        ("sp12/z_index_stacking", sp12_z_index_stacking),
        ("sp12/opacity_basic", sp12_opacity_basic),
        ("sp12/border_radius", sp12_border_radius),
        ("sp12/visibility_hidden", sp12_visibility_hidden),
        ("sp12/nested_blocks", sp12_nested_blocks),


        // ── SP12 Sticky Positioning ──────────────────────────────────────
        ("sp12/position_sticky_top", sp12_position_sticky_top),
        ("sp12/position_sticky_bottom", sp12_position_sticky_bottom),

        // ── SP12 Multicol ────────────────────────────────────────────────
        ("sp12/multicol_2_columns", sp12_multicol_2_columns),
        ("sp12/multicol_column_width", sp12_multicol_column_width),
        ("sp12/multicol_column_gap", sp12_multicol_column_gap),

        // ── SP12 Flex Advanced ───────────────────────────────────────────
        ("sp12/flex_direction_row_reverse", sp12_flex_direction_row_reverse),
        ("sp12/flex_direction_column_reverse", sp12_flex_direction_column_reverse),
        ("sp12/flex_wrap_reverse", sp12_flex_wrap_reverse),
        ("sp12/flex_justify_space_around", sp12_flex_justify_space_around),
        ("sp12/flex_justify_space_evenly", sp12_flex_justify_space_evenly),

        // ── SP12 Margin Collapsing Advanced ──────────────────────────────
        ("sp12/margin_collapsing_parent_child", sp12_margin_collapsing_parent_child),
        ("sp12/margin_collapsing_through_empty", sp12_margin_collapsing_through_empty),

        // ── SP12 Overflow Axes ───────────────────────────────────────────
        ("sp12/overflow_scroll", sp12_overflow_scroll),
        ("sp12/overflow_auto", sp12_overflow_auto),

        // ── SP12 Aspect Ratio ────────────────────────────────────────────
        ("sp12/aspect_ratio_basic", sp12_aspect_ratio_basic),
        // ── SP11 Text Decoration ─────────────────────────────────────
        ("sp11/text_decoration_underline", sp11_text_decoration_underline),
        ("sp11/text_decoration_overline", sp11_text_decoration_overline),
        ("sp11/text_decoration_line_through", sp11_text_decoration_line_through),
        ("sp11/text_decoration_combined", sp11_text_decoration_combined),

        // ── SP11 Font Weight & Style ─────────────────────────────────
        ("sp11/font_weight_normal", sp11_font_weight_normal),
        ("sp11/font_weight_bold", sp11_font_weight_bold),
        ("sp11/font_style_normal", sp11_font_style_normal),
        ("sp11/font_style_italic", sp11_font_style_italic),

        // ── SP11 Font Size ───────────────────────────────────────────
        ("sp11/font_size_small", sp11_font_size_small),
        ("sp11/font_size_medium", sp11_font_size_medium),
        ("sp11/font_size_large", sp11_font_size_large),
        ("sp11/font_size_xlarge", sp11_font_size_xlarge),

        // ── SP11 Text Align ──────────────────────────────────────────
        ("sp11/text_align_left", sp11_text_align_left),
        ("sp11/text_align_center", sp11_text_align_center),
        ("sp11/text_align_right", sp11_text_align_right),
        ("sp11/text_align_justify", sp11_text_align_justify),

        // ── SP11 Text Transform ──────────────────────────────────────
        ("sp11/text_transform_uppercase", sp11_text_transform_uppercase),
        ("sp11/text_transform_lowercase", sp11_text_transform_lowercase),
        ("sp11/text_transform_capitalize", sp11_text_transform_capitalize),

        // ── SP11 Text Indent ─────────────────────────────────────────
        ("sp11/text_indent_positive", sp11_text_indent_positive),
        ("sp11/text_indent_negative", sp11_text_indent_negative),

        // ── SP11 Letter & Word Spacing ───────────────────────────────
        ("sp11/letter_spacing_positive", sp11_letter_spacing_positive),
        ("sp11/letter_spacing_negative", sp11_letter_spacing_negative),
        ("sp11/word_spacing_positive", sp11_word_spacing_positive),

        // ── SP11 Line Height ─────────────────────────────────────────
        ("sp11/line_height_normal", sp11_line_height_normal),
        ("sp11/line_height_number", sp11_line_height_number),
        ("sp11/line_height_length", sp11_line_height_length),

        // ── SP11 White Space ─────────────────────────────────────────
        ("sp11/white_space_normal", sp11_white_space_normal),
        ("sp11/white_space_nowrap", sp11_white_space_nowrap),
        ("sp11/white_space_pre", sp11_white_space_pre),
        ("sp11/white_space_pre_wrap", sp11_white_space_pre_wrap),
        ("sp11/white_space_pre_line", sp11_white_space_pre_line),

        // ── SP11 Color ───────────────────────────────────────────────
        ("sp11/color_red", sp11_color_red),
        ("sp11/color_blue", sp11_color_blue),
        ("sp11/color_green", sp11_color_green),
        ("sp11/color_custom", sp11_color_custom),

        // ── SP11 Text Shadow & Overflow ──────────────────────────────
        ("sp11/text_shadow_basic", sp11_text_shadow_basic),
        ("sp11/text_overflow_ellipsis", sp11_text_overflow_ellipsis),

        // ── SP11 Text Decoration Style ──────────────────────────────
        ("sp11/text_decoration_style_solid", sp11_text_decoration_style_solid),
        ("sp11/text_decoration_style_double", sp11_text_decoration_style_double),
        ("sp11/text_decoration_style_dotted", sp11_text_decoration_style_dotted),
        ("sp11/text_decoration_style_dashed", sp11_text_decoration_style_dashed),
        ("sp11/text_decoration_style_wavy", sp11_text_decoration_style_wavy),

        // ── SP11 Text Decoration Skip-Ink ───────────────────────────
        ("sp11/text_decoration_skip_ink_auto", sp11_text_decoration_skip_ink_auto),
        ("sp11/text_decoration_skip_ink_none", sp11_text_decoration_skip_ink_none),

        // ── SP11 Text Decoration Metrics ────────────────────────────
        ("sp11/text_decoration_color_red", sp11_text_decoration_color_red),
        ("sp11/text_decoration_thickness_3px", sp11_text_decoration_thickness_3px),
        ("sp11/text_underline_offset_5px", sp11_text_underline_offset_5px),

        // ── SP11 Font Family ────────────────────────────────────────
        ("sp11/font_family_serif", sp11_font_family_serif),
        ("sp11/font_family_monospace", sp11_font_family_monospace),

        // ── SP11 Word Spacing Negative ──────────────────────────────
        ("sp11/word_spacing_negative", sp11_word_spacing_negative),

        // ── SP11 Line Height Percentage ─────────────────────────────
        ("sp11/line_height_percentage", sp11_line_height_percentage),

        // ── SP11 Text Shadow Offset ─────────────────────────────────
        ("sp11/text_shadow_offset", sp11_text_shadow_offset),

        // ── SP13 Inline Basic ────────────────────────────────────────
        ("sp13/inline_single_span", sp13_inline_single_span),
        ("sp13/inline_multiple_spans", sp13_inline_multiple_spans),
        ("sp13/inline_nested_spans", sp13_inline_nested_spans),

        // ── SP13 Line Breaking ───────────────────────────────────────
        ("sp13/line_breaking_normal_wrap", sp13_line_breaking_normal_wrap),
        ("sp13/line_breaking_nowrap", sp13_line_breaking_nowrap),
        ("sp13/line_breaking_break_word", sp13_line_breaking_break_word),

        // ── SP13 Vertical Align ──────────────────────────────────────
        ("sp13/vertical_align_baseline", sp13_vertical_align_baseline),
        ("sp13/vertical_align_middle", sp13_vertical_align_middle),
        ("sp13/vertical_align_top", sp13_vertical_align_top),
        ("sp13/vertical_align_bottom", sp13_vertical_align_bottom),
        ("sp13/vertical_align_super", sp13_vertical_align_super),
        ("sp13/vertical_align_sub", sp13_vertical_align_sub),

        // ── SP13 Inline Block ────────────────────────────────────────
        ("sp13/inline_block_basic", sp13_inline_block_basic),
        ("sp13/inline_block_vertical_align", sp13_inline_block_vertical_align),

        // ── SP13 Mixed Content ───────────────────────────────────────
        ("sp13/mixed_block_inline", sp13_mixed_block_inline),

        // ── SP13 White Space Handling ────────────────────────────────
        ("sp13/white_space_collapsing", sp13_white_space_collapsing),
        ("sp13/white_space_preserving", sp13_white_space_preserving),

        // ── SP13 Inline Decoration ───────────────────────────────────
        ("sp13/inline_background_color", sp13_inline_background_color),
        ("sp13/inline_padding", sp13_inline_padding),
        ("sp13/inline_border", sp13_inline_border),

        // ── SP13 First-Letter / First-Line ─────────────────────────────
        ("sp13/first_letter_basic", sp13_first_letter_basic),
        ("sp13/first_line_basic", sp13_first_line_basic),

        // ── SP13 Word Break ────────────────────────────────────────────
        ("sp13/word_break_break_all", sp13_word_break_break_all),
        ("sp13/overflow_wrap_break_word", sp13_overflow_wrap_break_word),

        // ── SP13 Vertical Align Extended ───────────────────────────────
        ("sp13/vertical_align_text_top", sp13_vertical_align_text_top),
        ("sp13/vertical_align_text_bottom", sp13_vertical_align_text_bottom),

        // ── SP13 Line Breaking Extended ────────────────────────────────
        ("sp13/line_breaking_overflow_wrap", sp13_line_breaking_overflow_wrap),
        ("sp13/line_breaking_hyphens_auto", sp13_line_breaking_hyphens_auto),

        // ── SP13 Box Decoration Break ──────────────────────────────────
        ("sp13/inline_box_multiline", sp13_inline_box_multiline),

        // ── SP13 Float Interaction ─────────────────────────────────────
        ("sp13/inline_with_float", sp13_inline_with_float),

        // ── SP12 Sticky Positioning Extended ───────────────────────────────
        ("sp12/position_sticky_left", sp12_position_sticky_left),
        ("sp12/position_sticky_right", sp12_position_sticky_right),

        // ── SP12 Multicol Extended ──────────────────────────────────────────
        ("sp12/multicol_column_rule", sp12_multicol_column_rule),
        ("sp12/multicol_column_span", sp12_multicol_column_span),
        ("sp12/multicol_column_fill_auto", sp12_multicol_column_fill_auto),
        ("sp12/multicol_column_fill_balance", sp12_multicol_column_fill_balance),

        // ── SP12 Position Offsets ───────────────────────────────────────────
        ("sp12/position_absolute_top_left", sp12_position_absolute_top_left),
        ("sp12/position_absolute_bottom_right", sp12_position_absolute_bottom_right),
        ("sp12/position_relative_top_left", sp12_position_relative_top_left),
        ("sp12/position_absolute_percent", sp12_position_absolute_percent),

        // ── SP12 Overflow Axis ──────────────────────────────────────────────
        ("sp12/overflow_x_hidden", sp12_overflow_x_hidden),
        ("sp12/overflow_y_hidden", sp12_overflow_y_hidden),
        ("sp12/overflow_x_scroll", sp12_overflow_x_scroll),
        ("sp12/overflow_y_scroll", sp12_overflow_y_scroll),

        // ── SP12 Flex Alignment Extended ────────────────────────────────────
        ("sp12/flex_align_self_start", sp12_flex_align_self_start),
        ("sp12/flex_align_self_end", sp12_flex_align_self_end),
        ("sp12/flex_align_self_center", sp12_flex_align_self_center),
        ("sp12/flex_align_content_center", sp12_flex_align_content_center),
        ("sp12/flex_align_content_space_between", sp12_flex_align_content_space_between),
        ("sp12/flex_order", sp12_flex_order),
        ("sp12/flex_basis_100px", sp12_flex_basis_100px),
        ("sp12/flex_shrink_basic", sp12_flex_shrink_basic),

        // ── SP12 Border Variants ────────────────────────────────────────────
        ("sp12/border_style_dashed", sp12_border_style_dashed),
        ("sp12/border_style_dotted", sp12_border_style_dotted),
        ("sp12/border_color_per_side", sp12_border_color_per_side),

        // ── SP12 Fragmentation ──────────────────────────────────────────────
        ("sp12/fragmentation_break_before_column", sp12_fragmentation_break_before_column),
        ("sp12/fragmentation_break_inside_avoid", sp12_fragmentation_break_inside_avoid),

        // ── SP13 First-Letter / First-Line Extended ─────────────────────────
        ("sp13/first_letter_color", sp13_first_letter_color),
        ("sp13/first_letter_font_size", sp13_first_letter_font_size),
        ("sp13/first_line_font_weight", sp13_first_line_font_weight),
        ("sp13/first_line_color", sp13_first_line_color),

        // ── SP13 Inline Advanced ─────────────────────────────────────────────
        ("sp13/initial_letter_basic", sp13_initial_letter_basic),
        ("sp13/ruby_basic", sp13_ruby_basic),
        ("sp13/text_combine_upright", sp13_text_combine_upright),
        ("sp13/inline_direction_rtl", sp13_inline_direction_rtl),
        ("sp13/tab_size_4", sp13_tab_size_4),
        ("sp13/text_align_last_center", sp13_text_align_last_center),

        // ── SP11 Text Emphasis ───────────────────────────────────────────────
        ("sp11/text_emphasis_dot", sp11_text_emphasis_dot),
        ("sp11/text_emphasis_circle", sp11_text_emphasis_circle),
        ("sp11/text_emphasis_position_over", sp11_text_emphasis_position_over),
        ("sp11/text_emphasis_color_red", sp11_text_emphasis_color_red),
    ];
    // Extend with auto-generated WPT tests
    tests.extend(wpt::all_wpt_registry());
    tests
}

// ═══════════════════════════════════════════════════════════════════════
// ── Helpers ─────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Create a Document with viewport padding and default text styles,
/// matching the HTML test file defaults:
///   body { margin: 0; padding: 20px; font-family: DejaVu Sans; font-size: 16px; }
pub fn base_doc() -> (Document, NodeId) {
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
    // Use inline-block colored boxes instead of text to avoid font-rendering diffs
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::InlineBlock;
    doc.node_mut(span).style.width = Length::px(120.0);
    doc.node_mut(span).style.height = Length::px(30.0);
    doc.node_mut(span).style.background_color = Color::from_rgba8(0, 128, 0, 255);
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
    // BFC companion (overflow:hidden) next to float — displaced correctly by both engines
    let companion = doc.create_node(ElementTag::Div);
    doc.node_mut(companion).style.display = Display::Block;
    doc.node_mut(companion).style.height = Length::px(50.0);
    doc.node_mut(companion).style.overflow_x = Overflow::Hidden;
    doc.node_mut(companion).style.overflow_y = Overflow::Hidden;
    doc.node_mut(companion).style.background_color = Color::from_rgba8(33, 150, 243, 255);
    doc.append_child(vp, companion);
    doc
}

fn sp12_float_right() -> Document {
    let (mut doc, vp) = base_doc();
    let f = add_block(&mut doc, vp, 100.0, 100.0, Color::from_rgba8(0, 128, 0, 255));
    doc.node_mut(f).style.float = Float::Right;
    // BFC companion (overflow:hidden) next to float — displaced correctly by both engines
    let companion = doc.create_node(ElementTag::Div);
    doc.node_mut(companion).style.display = Display::Block;
    doc.node_mut(companion).style.height = Length::px(50.0);
    doc.node_mut(companion).style.overflow_x = Overflow::Hidden;
    doc.node_mut(companion).style.overflow_y = Overflow::Hidden;
    doc.node_mut(companion).style.background_color = Color::from_rgba8(33, 150, 243, 255);
    doc.append_child(vp, companion);
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
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
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
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
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
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
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

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Flexbox Tests ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_flex_direction_row() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.flex_direction = FlexDirection::Row;
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(100.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc
}

fn sp12_flex_direction_column() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.flex_direction = FlexDirection::Column;
    doc.node_mut(flex).style.width = Length::px(200.0);
    doc.node_mut(flex).style.height = Length::px(300.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc
}

fn sp12_flex_justify_start() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.justify_content = ContentAlignment::new(ContentPosition::FlexStart);
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 60.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 60.0, 60.0, Color::BLUE);
    doc
}

fn sp12_flex_justify_center() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.justify_content = ContentAlignment::new(ContentPosition::Center);
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 60.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 60.0, 60.0, Color::BLUE);
    doc
}

fn sp12_flex_justify_space_between() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 60.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 60.0, 60.0, Color::BLUE);
    add_block(&mut doc, flex, 60.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc
}

fn sp12_flex_align_center() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.align_items = ItemAlignment::new(ItemPosition::Center);
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(150.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 40.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 80.0, Color::BLUE);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc
}

fn sp12_flex_align_stretch() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.align_items = ItemAlignment::new(ItemPosition::Stretch);
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(150.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    // Items have no explicit height — should stretch to container height
    let a = doc.create_node(ElementTag::Div);
    doc.node_mut(a).style.display = Display::Block;
    doc.node_mut(a).style.width = Length::px(80.0);
    doc.node_mut(a).style.background_color = Color::RED;
    doc.append_child(flex, a);
    let b = doc.create_node(ElementTag::Div);
    doc.node_mut(b).style.display = Display::Block;
    doc.node_mut(b).style.width = Length::px(80.0);
    doc.node_mut(b).style.background_color = Color::BLUE;
    doc.append_child(flex, b);
    doc
}

fn sp12_flex_wrap_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.flex_wrap = FlexWrap::Wrap;
    doc.node_mut(flex).style.width = Length::px(200.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    // Four 80px items in 200px container → wraps after 2
    add_block(&mut doc, flex, 80.0, 50.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 50.0, Color::BLUE);
    add_block(&mut doc, flex, 80.0, 50.0, Color::from_rgba8(0, 128, 0, 255));
    add_block(&mut doc, flex, 80.0, 50.0, Color::from_rgba8(255, 165, 0, 255));
    doc
}

fn sp12_flex_grow_equal() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    // Three items each with flex_grow=1 should share space equally
    for color in [Color::RED, Color::BLUE, Color::from_rgba8(0, 128, 0, 255)] {
        let item = doc.create_node(ElementTag::Div);
        doc.node_mut(item).style.display = Display::Block;
        doc.node_mut(item).style.flex_grow = 1.0;
        doc.node_mut(item).style.height = Length::px(60.0);
        doc.node_mut(item).style.background_color = color;
        doc.append_child(flex, item);
    }
    doc
}

fn sp12_flex_gap() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.column_gap = Some(Length::px(20.0));
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Visual / Stacking Tests ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_z_index_stacking() -> Document {
    let (mut doc, vp) = base_doc();
    doc.node_mut(vp).style.position = Position::Relative;
    // Lower box (z-index: 1)
    let a = add_block(&mut doc, vp, 150.0, 150.0, Color::RED);
    doc.node_mut(a).style.position = Position::Absolute;
    doc.node_mut(a).style.top = Length::px(20.0);
    doc.node_mut(a).style.left = Length::px(20.0);
    doc.node_mut(a).style.z_index = Some(1);
    // Upper box (z-index: 2) — overlaps and should render on top
    let b = add_block(&mut doc, vp, 150.0, 150.0, Color::BLUE);
    doc.node_mut(b).style.position = Position::Absolute;
    doc.node_mut(b).style.top = Length::px(60.0);
    doc.node_mut(b).style.left = Length::px(60.0);
    doc.node_mut(b).style.z_index = Some(2);
    doc
}

fn sp12_opacity_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.node_mut(div).style.opacity = 0.5;
    // Fully opaque reference below
    add_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    doc
}

fn sp12_border_radius() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 200.0, 200.0, Color::RED);
    doc.node_mut(div).style.border_top_left_radius = (20.0, 20.0);
    doc.node_mut(div).style.border_top_right_radius = (20.0, 20.0);
    doc.node_mut(div).style.border_bottom_right_radius = (20.0, 20.0);
    doc.node_mut(div).style.border_bottom_left_radius = (20.0, 20.0);
    doc
}

fn sp12_visibility_hidden() -> Document {
    let (mut doc, vp) = base_doc();
    add_block(&mut doc, vp, 200.0, 50.0, Color::RED);
    let hidden = add_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.node_mut(hidden).style.visibility = Visibility::Hidden;
    // Third box should appear after the gap left by the hidden box
    add_block(&mut doc, vp, 200.0, 50.0, Color::from_rgba8(0, 128, 0, 255));
    doc
}

fn sp12_nested_blocks() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(300.0);
    doc.node_mut(outer).style.padding_top = Length::px(10.0);
    doc.node_mut(outer).style.padding_right = Length::px(10.0);
    doc.node_mut(outer).style.padding_bottom = Length::px(10.0);
    doc.node_mut(outer).style.padding_left = Length::px(10.0);
    doc.node_mut(outer).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, outer);
    let mid = doc.create_node(ElementTag::Div);
    doc.node_mut(mid).style.display = Display::Block;
    doc.node_mut(mid).style.padding_top = Length::px(10.0);
    doc.node_mut(mid).style.padding_right = Length::px(10.0);
    doc.node_mut(mid).style.padding_bottom = Length::px(10.0);
    doc.node_mut(mid).style.padding_left = Length::px(10.0);
    doc.node_mut(mid).style.background_color = Color::from_rgba8(150, 150, 200, 255);
    doc.append_child(outer, mid);
    add_block(&mut doc, mid, 100.0, 60.0, Color::RED);
    add_block(&mut doc, mid, 100.0, 60.0, Color::BLUE);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Sticky Positioning Tests ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_position_sticky_top() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    let child = add_block(&mut doc, container, 100.0, 30.0, Color::from_rgba8(76, 175, 80, 255));
    doc.node_mut(child).style.position = Position::Sticky;
    doc.node_mut(child).style.top = Length::px(10.0);
    doc
}

fn sp12_position_sticky_bottom() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    let child = add_block(&mut doc, container, 100.0, 30.0, Color::from_rgba8(76, 175, 80, 255));
    doc.node_mut(child).style.position = Position::Sticky;
    doc.node_mut(child).style.bottom = Length::px(10.0);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Multicol Tests ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_multicol_2_columns() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.column_count = Some(2);
    doc.node_mut(container).style.column_gap = Some(Length::px(20.0));
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(255, 152, 0, 255));
    doc
}

fn sp12_multicol_column_width() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.column_width = Some(Length::px(150.0));
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    add_block(&mut doc, container, 140.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, container, 140.0, 50.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, container, 140.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    add_block(&mut doc, container, 140.0, 50.0, Color::from_rgba8(255, 152, 0, 255));
    doc
}

fn sp12_multicol_column_gap() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.column_count = Some(2);
    doc.node_mut(container).style.column_gap = Some(Length::px(40.0));
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    add_block(&mut doc, container, 170.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, container, 170.0, 50.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, container, 170.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    add_block(&mut doc, container, 170.0, 50.0, Color::from_rgba8(255, 152, 0, 255));
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Flex Advanced Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_flex_direction_row_reverse() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.flex_direction = FlexDirection::RowReverse;
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(100.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(33, 150, 243, 255));
    doc
}

fn sp12_flex_direction_column_reverse() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.flex_direction = FlexDirection::ColumnReverse;
    doc.node_mut(flex).style.width = Length::px(200.0);
    doc.node_mut(flex).style.height = Length::px(300.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(33, 150, 243, 255));
    doc
}

fn sp12_flex_wrap_reverse() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.flex_wrap = FlexWrap::WrapReverse;
    doc.node_mut(flex).style.width = Length::px(200.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(33, 150, 243, 255));
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(255, 152, 0, 255));
    doc
}

fn sp12_flex_justify_space_around() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 60.0, 40.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, flex, 60.0, 40.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, flex, 60.0, 40.0, Color::from_rgba8(33, 150, 243, 255));
    doc
}

fn sp12_flex_justify_space_evenly() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.justify_content = ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly);
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 60.0, 40.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, flex, 60.0, 40.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, flex, 60.0, 40.0, Color::from_rgba8(33, 150, 243, 255));
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Margin Collapsing Advanced Tests ───────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_margin_collapsing_parent_child() -> Document {
    let (mut doc, vp) = base_doc();
    // Parent with no border/padding — child's margin-top should collapse with parent
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, parent);
    let child = add_block(&mut doc, parent, 100.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    doc.node_mut(child).style.margin_top = Length::px(30.0);
    // Reference block below to visualize the gap
    add_block(&mut doc, vp, 200.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    doc
}

fn sp12_margin_collapsing_through_empty() -> Document {
    let (mut doc, vp) = base_doc();
    // First sibling with margin-bottom
    let a = add_block(&mut doc, vp, 200.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    doc.node_mut(a).style.margin_bottom = Length::px(20.0);
    // Empty block between — its own margins collapse through
    let empty = doc.create_node(ElementTag::Div);
    doc.node_mut(empty).style.display = Display::Block;
    doc.node_mut(empty).style.margin_top = Length::px(15.0);
    doc.node_mut(empty).style.margin_bottom = Length::px(25.0);
    doc.append_child(vp, empty);
    // Second sibling with margin-top
    let b = add_block(&mut doc, vp, 200.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    doc.node_mut(b).style.margin_top = Length::px(10.0);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Overflow Axes Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_overflow_scroll() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(200.0);
    doc.node_mut(outer).style.height = Length::px(100.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Scroll;
    doc.node_mut(outer).style.overflow_y = Overflow::Scroll;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 180.0, 200.0, Color::from_rgba8(244, 67, 54, 255));
    doc
}

fn sp12_overflow_auto() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(200.0);
    doc.node_mut(outer).style.height = Length::px(100.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Auto;
    doc.node_mut(outer).style.overflow_y = Overflow::Auto;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 180.0, 200.0, Color::from_rgba8(244, 67, 54, 255));
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Aspect Ratio Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_aspect_ratio_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.aspect_ratio = Some(AspectRatio { ratio: (2.0, 1.0), auto_flag: false });
    doc.node_mut(div).style.background_color = Color::from_rgba8(156, 39, 176, 255);
    doc.append_child(vp, div);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Decoration Tests ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Helper: create a text block with given text and return its node id.
fn add_text_block(doc: &mut Document, parent: NodeId, text: &str) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.margin_bottom = Length::px(10.0);
    doc.node_mut(div).text = Some(text.to_string());
    doc.append_child(parent, div);
    div
}

fn sp11_text_decoration_underline() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "This text has an underline decoration");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc
}

fn sp11_text_decoration_overline() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "This text has an overline decoration");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::OVERLINE;
    doc
}

fn sp11_text_decoration_line_through() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "This text has a line-through decoration");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::LINE_THROUGH;
    doc
}

fn sp11_text_decoration_combined() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "This text has underline + overline + line-through");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0 | TextDecorationLine::OVERLINE.0 | TextDecorationLine::LINE_THROUGH.0,
    );
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Font Weight & Style Tests ──────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_font_weight_normal() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Normal weight (400) text sample");
    doc.node_mut(t).style.font_weight = FontWeight::NORMAL;
    doc
}

fn sp11_font_weight_bold() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Bold weight (700) text sample");
    doc.node_mut(t).style.font_weight = FontWeight::BOLD;
    doc
}

fn sp11_font_style_normal() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Normal style text sample");
    doc.node_mut(t).style.font_style = FontStyleEnum::Normal;
    doc
}

fn sp11_font_style_italic() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Italic style text sample");
    doc.node_mut(t).style.font_style = FontStyleEnum::Italic;
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Font Size Tests ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_font_size_small() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Small text at 12px font size");
    doc.node_mut(t).style.font_size = 12.0;
    doc
}

fn sp11_font_size_medium() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Medium text at 16px font size (default)");
    doc.node_mut(t).style.font_size = 16.0;
    doc
}

fn sp11_font_size_large() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Large text at 24px font size");
    doc.node_mut(t).style.font_size = 24.0;
    doc
}

fn sp11_font_size_xlarge() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Extra large text at 32px");
    doc.node_mut(t).style.font_size = 32.0;
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Align Tests ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_align_left() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Left-aligned text in a block");
    doc.node_mut(t).style.text_align = TextAlign::Left;
    doc.node_mut(t).style.width = Length::px(400.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_text_align_center() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Center-aligned text in a block");
    doc.node_mut(t).style.text_align = TextAlign::Center;
    doc.node_mut(t).style.width = Length::px(400.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_text_align_right() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Right-aligned text in a block");
    doc.node_mut(t).style.text_align = TextAlign::Right;
    doc.node_mut(t).style.width = Length::px(400.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_text_align_justify() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "Justified text stretches words across the full width of the container block so that both edges are flush.",
    );
    doc.node_mut(t).style.text_align = TextAlign::Justify;
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Transform Tests ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_transform_uppercase() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "this text should be uppercase");
    doc.node_mut(t).style.text_transform = TextTransform::Uppercase;
    doc
}

fn sp11_text_transform_lowercase() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "THIS TEXT SHOULD BE LOWERCASE");
    doc.node_mut(t).style.text_transform = TextTransform::Lowercase;
    doc
}

fn sp11_text_transform_capitalize() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "capitalize each word in this sentence");
    doc.node_mut(t).style.text_transform = TextTransform::Capitalize;
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Indent Tests ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_indent_positive() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "This paragraph has a positive 40px text-indent on the first line. The second line wraps normally without indent.",
    );
    doc.node_mut(t).style.text_indent = Length::px(40.0);
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_text_indent_negative() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "This paragraph has a negative -20px text-indent (hanging indent) on the first line.",
    );
    doc.node_mut(t).style.text_indent = Length::px(-20.0);
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.padding_left = Length::px(30.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Letter & Word Spacing Tests ────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_letter_spacing_positive() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Wide letter spacing");
    doc.node_mut(t).style.letter_spacing = 5.0;
    add_text_block(&mut doc, vp, "Normal letter spacing for comparison");
    doc
}

fn sp11_letter_spacing_negative() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Tight letter spacing");
    doc.node_mut(t).style.letter_spacing = -1.0;
    add_text_block(&mut doc, vp, "Normal letter spacing for comparison");
    doc
}

fn sp11_word_spacing_positive() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Extra space between words in this sentence");
    doc.node_mut(t).style.word_spacing = 15.0;
    add_text_block(&mut doc, vp, "Normal word spacing for comparison");
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Line Height Tests ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_line_height_normal() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "Line height normal. This is a multi-line paragraph to demonstrate the default line spacing between lines of text.",
    );
    doc.node_mut(t).style.line_height = LineHeight::Normal;
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_line_height_number() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "Line height 2.0. This is a multi-line paragraph to demonstrate double line spacing between lines of text.",
    );
    doc.node_mut(t).style.line_height = LineHeight::Number(2.0);
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_line_height_length() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "Line height 30px. This is a multi-line paragraph to demonstrate fixed 30px line spacing between lines.",
    );
    doc.node_mut(t).style.line_height = LineHeight::Length(30.0);
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 White Space Tests ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_white_space_normal() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "White space   normal:   multiple    spaces   and\nnewlines   collapse   into   single   spaces.",
    );
    doc.node_mut(t).style.white_space = WhiteSpace::Normal;
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_white_space_nowrap() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "White space nowrap: this long text should not wrap to the next line even if it overflows the container.",
    );
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.node_mut(t).style.width = Length::px(200.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_white_space_pre() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "White space pre:\n  indented line\n  preserves   spaces\n    and newlines",
    );
    doc.node_mut(t).style.white_space = WhiteSpace::Pre;
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_white_space_pre_wrap() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "White space pre-wrap:\n  preserves   spaces\n  but also   wraps   long lines when they exceed the container width limit.",
    );
    doc.node_mut(t).style.white_space = WhiteSpace::PreWrap;
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

fn sp11_white_space_pre_line() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "White space pre-line:\n  collapses   spaces\n  but   preserves\n  newlines and wraps.",
    );
    doc.node_mut(t).style.white_space = WhiteSpace::PreLine;
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Color Tests ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_color_red() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "This text is rendered in red color");
    doc.node_mut(t).style.color = Color::RED;
    doc
}

fn sp11_color_blue() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "This text is rendered in blue color");
    doc.node_mut(t).style.color = Color::BLUE;
    doc
}

fn sp11_color_green() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "This text is rendered in green color");
    doc.node_mut(t).style.color = Color::GREEN;
    doc
}

fn sp11_color_custom() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "This text is rendered in custom purple (#8B008B)");
    doc.node_mut(t).style.color = Color::from_rgba8(139, 0, 139, 255);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Shadow & Overflow Tests ───────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_shadow_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Text with a shadow effect");
    doc.node_mut(t).style.font_size = 24.0;
    doc.node_mut(t).style.text_shadow = vec![TextShadow {
        offset_x: 2.0,
        offset_y: 2.0,
        blur_radius: 4.0,
        color: Color::from_rgba8(0, 0, 0, 128),
    }];
    doc
}

fn sp11_text_overflow_ellipsis() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "This text overflows its container and should show an ellipsis at the end",
    );
    doc.node_mut(t).style.width = Length::px(200.0);
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.node_mut(t).style.overflow_x = Overflow::Hidden;
    doc.node_mut(t).style.text_overflow = TextOverflow::Ellipsis;
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Decoration Style Tests ────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_decoration_style_solid() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_decoration_style = TextDecorationStyle::Solid;
    doc
}

fn sp11_text_decoration_style_double() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_decoration_style = TextDecorationStyle::Double;
    doc
}

fn sp11_text_decoration_style_dotted() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_decoration_style = TextDecorationStyle::Dotted;
    doc
}

fn sp11_text_decoration_style_dashed() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_decoration_style = TextDecorationStyle::Dashed;
    doc
}

fn sp11_text_decoration_style_wavy() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_decoration_style = TextDecorationStyle::Wavy;
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Decoration Skip-Ink Tests ─────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_decoration_skip_ink_auto() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Typography");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_decoration_skip_ink = TextDecorationSkipInk::Auto;
    doc
}

fn sp11_text_decoration_skip_ink_none() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Typography");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_decoration_skip_ink = TextDecorationSkipInk::None;
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Decoration Metrics Tests ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_decoration_color_red() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(t).style.color = Color::BLUE;
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_decoration_color = StyleColor::Resolved(Color::RED);
    doc
}

fn sp11_text_decoration_thickness_3px() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_decoration_thickness = TextDecorationThickness::Length(3.0);
    doc
}

fn sp11_text_underline_offset_5px() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(t).style.text_decoration_line = TextDecorationLine::UNDERLINE;
    doc.node_mut(t).style.text_underline_offset = Length::px(5.0);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Font Family Tests ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_font_family_serif() -> Document {
    let (mut doc, vp) = base_doc();
    doc.node_mut(vp).style.font_family = FontFamilyList::generic(GenericFontFamily::Serif);
    add_text_block(&mut doc, vp, "Hello World");
    doc
}

fn sp11_font_family_monospace() -> Document {
    let (mut doc, vp) = base_doc();
    doc.node_mut(vp).style.font_family = FontFamilyList::generic(GenericFontFamily::Monospace);
    add_text_block(&mut doc, vp, "Hello World");
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Word Spacing Negative Test ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_word_spacing_negative() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "The quick brown fox");
    doc.node_mut(t).style.word_spacing = -3.0;
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Line Height Percentage Test ────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_line_height_percentage() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(
        &mut doc,
        vp,
        "Line height 200%. This is a multi-line paragraph to demonstrate percentage-based line spacing between lines of text.",
    );
    doc.node_mut(t).style.line_height = LineHeight::Percentage(200.0);
    doc.node_mut(t).style.width = Length::px(300.0);
    doc.node_mut(t).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Shadow Offset Test ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_shadow_offset() -> Document {
    let (mut doc, vp) = base_doc();
    let t = add_text_block(&mut doc, vp, "Shadow");
    doc.node_mut(t).style.text_shadow = vec![TextShadow {
        offset_x: 3.0,
        offset_y: 3.0,
        blur_radius: 0.0,
        color: Color::RED,
    }];
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Inline Basic Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_single_span() -> Document {
    let (mut doc, vp) = base_doc();
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.color = Color::RED;
    doc.node_mut(span).text = Some("A single inline span with red text".to_string());
    doc.append_child(vp, span);
    doc
}

fn sp13_inline_multiple_spans() -> Document {
    let (mut doc, vp) = base_doc();
    let colors = [Color::RED, Color::BLUE, Color::from_rgba8(0, 128, 0, 255)];
    let texts = ["First span ", "Second span ", "Third span"];
    for (text, color) in texts.iter().zip(colors.iter()) {
        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.node_mut(span).style.color = *color;
        doc.node_mut(span).text = Some(text.to_string());
        doc.append_child(vp, span);
    }
    doc
}

fn sp13_inline_nested_spans() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Span);
    doc.node_mut(outer).style.display = Display::Inline;
    doc.node_mut(outer).style.color = Color::BLUE;
    doc.node_mut(outer).text = Some("Outer ".to_string());
    doc.append_child(vp, outer);

    let inner = doc.create_node(ElementTag::Span);
    doc.node_mut(inner).style.display = Display::Inline;
    doc.node_mut(inner).style.color = Color::RED;
    doc.node_mut(inner).style.font_weight = FontWeight::BOLD;
    doc.node_mut(inner).text = Some("inner bold red".to_string());
    doc.append_child(outer, inner);

    let after = doc.create_node(ElementTag::Span);
    doc.node_mut(after).style.display = Display::Inline;
    doc.node_mut(after).style.color = Color::BLUE;
    doc.node_mut(after).text = Some(" outer again".to_string());
    doc.append_child(vp, after);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Line Breaking Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_line_breaking_normal_wrap() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).text = Some("This is a long line of text that should naturally wrap at word boundaries within the container".to_string());
    doc.append_child(div, span);
    doc
}

fn sp13_line_breaking_nowrap() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.white_space = WhiteSpace::Nowrap;
    doc.node_mut(div).style.overflow_x = Overflow::Hidden;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).text = Some("This text should not wrap and may be clipped by overflow hidden".to_string());
    doc.append_child(div, span);
    doc
}

fn sp13_line_breaking_break_word() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(150.0);
    doc.node_mut(div).style.overflow_wrap = OverflowWrap::BreakWord;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).text = Some("Supercalifragilisticexpialidocious should break mid-word".to_string());
    doc.append_child(div, span);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Vertical Align Tests ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Helper: build a vertical-align test with a large span and an aligned smaller span.
fn vertical_align_test(va: VerticalAlign, label: &str) -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.node_mut(div).style.line_height = LineHeight::Length(60.0);
    doc.append_child(vp, div);

    let big = doc.create_node(ElementTag::Span);
    doc.node_mut(big).style.display = Display::Inline;
    doc.node_mut(big).style.font_size = 32.0;
    doc.node_mut(big).text = Some("Big ".to_string());
    doc.append_child(div, big);

    let small = doc.create_node(ElementTag::Span);
    doc.node_mut(small).style.display = Display::Inline;
    doc.node_mut(small).style.font_size = 12.0;
    doc.node_mut(small).style.vertical_align = va;
    doc.node_mut(small).style.background_color = Color::from_rgba8(255, 200, 200, 255);
    doc.node_mut(small).text = Some(label.to_string());
    doc.append_child(div, small);
    doc
}

fn sp13_vertical_align_baseline() -> Document {
    vertical_align_test(VerticalAlign::Baseline, "baseline")
}

fn sp13_vertical_align_middle() -> Document {
    vertical_align_test(VerticalAlign::Middle, "middle")
}

fn sp13_vertical_align_top() -> Document {
    vertical_align_test(VerticalAlign::Top, "top")
}

fn sp13_vertical_align_bottom() -> Document {
    vertical_align_test(VerticalAlign::Bottom, "bottom")
}

fn sp13_vertical_align_super() -> Document {
    vertical_align_test(VerticalAlign::Super, "super")
}

fn sp13_vertical_align_sub() -> Document {
    vertical_align_test(VerticalAlign::Sub, "sub")
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Inline Block Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_block_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).text = Some("Text before ".to_string());
    doc.append_child(vp, span);

    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.node_mut(ib).style.width = Length::px(80.0);
    doc.node_mut(ib).style.height = Length::px(40.0);
    doc.node_mut(ib).style.background_color = Color::RED;
    doc.append_child(vp, ib);

    let after = doc.create_node(ElementTag::Span);
    doc.node_mut(after).style.display = Display::Inline;
    doc.node_mut(after).text = Some(" text after".to_string());
    doc.append_child(vp, after);
    doc
}

fn sp13_inline_block_vertical_align() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.append_child(vp, div);

    let label = doc.create_node(ElementTag::Span);
    doc.node_mut(label).style.display = Display::Inline;
    doc.node_mut(label).text = Some("Aligned: ".to_string());
    doc.append_child(div, label);

    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.node_mut(ib).style.width = Length::px(60.0);
    doc.node_mut(ib).style.height = Length::px(60.0);
    doc.node_mut(ib).style.background_color = Color::BLUE;
    doc.node_mut(ib).style.vertical_align = VerticalAlign::Middle;
    doc.append_child(div, ib);

    let trail = doc.create_node(ElementTag::Span);
    doc.node_mut(trail).style.display = Display::Inline;
    doc.node_mut(trail).text = Some(" middle-aligned inline-block".to_string());
    doc.append_child(div, trail);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Mixed Content Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_mixed_block_inline() -> Document {
    let (mut doc, vp) = base_doc();
    // Block element
    let blk = add_block(&mut doc, vp, 300.0, 40.0, Color::from_rgba8(200, 220, 255, 255));
    let blk_txt = doc.create_node(ElementTag::Span);
    doc.node_mut(blk_txt).style.display = Display::Inline;
    doc.node_mut(blk_txt).text = Some("Block element with text".to_string());
    doc.append_child(blk, blk_txt);

    // Inline span following block
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.color = Color::RED;
    doc.node_mut(span).text = Some("Inline span after block ".to_string());
    doc.append_child(vp, span);

    // Another block
    add_block(&mut doc, vp, 300.0, 40.0, Color::from_rgba8(220, 255, 200, 255));
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 White Space Handling Tests ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_white_space_collapsing() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(300.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.append_child(vp, div);

    // Multiple inline spans with extra whitespace — should collapse
    for text in ["  Multiple  ", "  spaces  ", "  should  ", "  collapse  "] {
        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.node_mut(span).text = Some(text.to_string());
        doc.append_child(div, span);
    }
    doc
}

fn sp13_white_space_preserving() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(400.0);
    doc.node_mut(div).style.white_space = WhiteSpace::Pre;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).text = Some("  Preserved   spaces   and\n  newlines  ".to_string());
    doc.append_child(div, span);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Inline Decoration Tests ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_background_color() -> Document {
    let (mut doc, vp) = base_doc();
    let before = doc.create_node(ElementTag::Span);
    doc.node_mut(before).style.display = Display::Inline;
    doc.node_mut(before).text = Some("Normal text ".to_string());
    doc.append_child(vp, before);

    let highlight = doc.create_node(ElementTag::Span);
    doc.node_mut(highlight).style.display = Display::Inline;
    doc.node_mut(highlight).style.background_color = Color::from_rgba8(255, 255, 0, 255);
    doc.node_mut(highlight).text = Some("highlighted span".to_string());
    doc.append_child(vp, highlight);

    let after = doc.create_node(ElementTag::Span);
    doc.node_mut(after).style.display = Display::Inline;
    doc.node_mut(after).text = Some(" normal text".to_string());
    doc.append_child(vp, after);
    doc
}

fn sp13_inline_padding() -> Document {
    let (mut doc, vp) = base_doc();
    let before = doc.create_node(ElementTag::Span);
    doc.node_mut(before).style.display = Display::Inline;
    doc.node_mut(before).text = Some("Before ".to_string());
    doc.append_child(vp, before);

    let padded = doc.create_node(ElementTag::Span);
    doc.node_mut(padded).style.display = Display::Inline;
    doc.node_mut(padded).style.padding_top = Length::px(4.0);
    doc.node_mut(padded).style.padding_right = Length::px(12.0);
    doc.node_mut(padded).style.padding_bottom = Length::px(4.0);
    doc.node_mut(padded).style.padding_left = Length::px(12.0);
    doc.node_mut(padded).style.background_color = Color::from_rgba8(200, 230, 255, 255);
    doc.node_mut(padded).text = Some("padded inline".to_string());
    doc.append_child(vp, padded);

    let after = doc.create_node(ElementTag::Span);
    doc.node_mut(after).style.display = Display::Inline;
    doc.node_mut(after).text = Some(" after".to_string());
    doc.append_child(vp, after);
    doc
}

fn sp13_inline_border() -> Document {
    let (mut doc, vp) = base_doc();
    let before = doc.create_node(ElementTag::Span);
    doc.node_mut(before).style.display = Display::Inline;
    doc.node_mut(before).text = Some("Before ".to_string());
    doc.append_child(vp, before);

    let bordered = doc.create_node(ElementTag::Span);
    doc.node_mut(bordered).style.display = Display::Inline;
    doc.node_mut(bordered).style.border_top_width = 2;
    doc.node_mut(bordered).style.border_right_width = 2;
    doc.node_mut(bordered).style.border_bottom_width = 2;
    doc.node_mut(bordered).style.border_left_width = 2;
    doc.node_mut(bordered).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(bordered).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(bordered).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(bordered).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(bordered).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(bordered).style.border_right_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(bordered).style.border_bottom_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(bordered).style.border_left_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(bordered).style.padding_left = Length::px(6.0);
    doc.node_mut(bordered).style.padding_right = Length::px(6.0);
    doc.node_mut(bordered).text = Some("bordered inline".to_string());
    doc.append_child(vp, bordered);

    let after = doc.create_node(ElementTag::Span);
    doc.node_mut(after).style.display = Display::Inline;
    doc.node_mut(after).text = Some(" after".to_string());
    doc.append_child(vp, after);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 First-Letter / First-Line Tests ────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// First-letter pseudo-element effect: the opening letter is rendered at 2em
/// in red (#F44336) while the remainder uses the default style.  The Document
/// builder has no direct `::first-letter` API, so we model the visual result
/// with an explicit large-letter span.
fn sp13_first_letter_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.append_child(vp, div);

    let letter = doc.create_node(ElementTag::Span);
    doc.node_mut(letter).style.display = Display::Inline;
    doc.node_mut(letter).style.font_size = 32.0; // 2em relative to 16px base
    doc.node_mut(letter).style.color = Color::from_rgba8(244, 67, 54, 255); // #F44336
    doc.node_mut(letter).text = Some("L".to_string());
    doc.append_child(div, letter);

    let rest = doc.create_node(ElementTag::Span);
    doc.node_mut(rest).style.display = Display::Inline;
    doc.node_mut(rest).text = Some("orem ipsum dolor sit amet".to_string());
    doc.append_child(div, rest);
    doc
}

/// First-line pseudo-element effect: the first line is rendered in blue
/// (#2196F3) and bold.  The Document builder has no `::first-line` API, so
/// we model this with a styled first-line span followed by normal text.
fn sp13_first_line_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let wrapper = doc.create_node(ElementTag::Div);
    doc.node_mut(wrapper).style.display = Display::Block;
    doc.node_mut(wrapper).style.width = Length::px(250.0);
    doc.append_child(vp, wrapper);

    let para = doc.create_node(ElementTag::Div);
    doc.node_mut(para).style.display = Display::Block;
    doc.node_mut(para).style.width = Length::px(250.0);
    doc.append_child(wrapper, para);

    let first = doc.create_node(ElementTag::Span);
    doc.node_mut(first).style.display = Display::Inline;
    doc.node_mut(first).style.color = Color::from_rgba8(33, 150, 243, 255); // #2196F3
    doc.node_mut(first).style.font_weight = FontWeight::BOLD;
    doc.node_mut(first).text = Some("The first line is styled differently".to_string());
    doc.append_child(para, first);

    let rest = doc.create_node(ElementTag::Span);
    doc.node_mut(rest).style.display = Display::Inline;
    doc.node_mut(rest).text = Some(" and the remaining text uses the default paragraph style for subsequent lines".to_string());
    doc.append_child(para, rest);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Word Break Tests ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_word_break_break_all() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(100.0);
    doc.node_mut(div).style.word_break = WordBreak::BreakAll;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.node_mut(div).text = Some("Supercalifragilisticexpialidocious".to_string());
    doc.append_child(vp, div);
    doc
}

fn sp13_overflow_wrap_break_word() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(100.0);
    doc.node_mut(div).style.overflow_wrap = OverflowWrap::BreakWord;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.node_mut(div).text = Some("Supercalifragilisticexpialidocious".to_string());
    doc.append_child(vp, div);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Vertical Align Extended Tests ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_vertical_align_text_top() -> Document {
    vertical_align_test(VerticalAlign::TextTop, "text-top")
}

fn sp13_vertical_align_text_bottom() -> Document {
    vertical_align_test(VerticalAlign::TextBottom, "text-bottom")
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Line Breaking Extended Tests ───────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_line_breaking_overflow_wrap() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(150.0);
    doc.node_mut(div).style.overflow_wrap = OverflowWrap::Anywhere;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.node_mut(div).text = Some(
        "https://example.com/very/long/path/to/resource/that/should/break".to_string(),
    );
    doc.append_child(vp, div);
    doc
}

fn sp13_line_breaking_hyphens_auto() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(120.0);
    doc.node_mut(div).style.hyphens = Hyphens::Auto;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.node_mut(div).text = Some(
        "Incomprehensibilities and internationalization are long words".to_string(),
    );
    doc.append_child(vp, div);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Box Decoration Break Test ──────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_box_multiline() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.append_child(vp, div);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.background_color = Color::from_rgba8(200, 230, 255, 255);
    doc.node_mut(span).style.padding_top = Length::px(4.0);
    doc.node_mut(span).style.padding_right = Length::px(8.0);
    doc.node_mut(span).style.padding_bottom = Length::px(4.0);
    doc.node_mut(span).style.padding_left = Length::px(8.0);
    doc.node_mut(span).style.box_decoration_break = BoxDecorationBreak::Clone;
    doc.node_mut(span).text = Some(
        "This inline span has background and padding and wraps to multiple lines".to_string(),
    );
    doc.append_child(div, span);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Float Interaction Test ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

// (sp13_inline_with_float defined below after new tests)

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Sticky Positioning Extended Tests ───────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_position_sticky_left() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    let child = add_block(&mut doc, container, 50.0, 50.0, Color::from_rgba8(76, 175, 80, 255));
    doc.node_mut(child).style.position = Position::Sticky;
    doc.node_mut(child).style.left = Length::px(20.0);
    doc
}

fn sp12_position_sticky_right() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    let child = add_block(&mut doc, container, 50.0, 50.0, Color::from_rgba8(76, 175, 80, 255));
    doc.node_mut(child).style.position = Position::Sticky;
    doc.node_mut(child).style.right = Length::px(20.0);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Multicol Extended Tests ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_multicol_column_rule() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.column_count = Some(2);
    doc.node_mut(container).style.column_gap = Some(Length::px(20.0));
    doc.node_mut(container).style.column_rule_width = 2;
    doc.node_mut(container).style.column_rule_style = BorderStyle::Solid;
    doc.node_mut(container).style.column_rule_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(255, 152, 0, 255));
    doc
}

fn sp12_multicol_column_span() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.column_count = Some(2);
    doc.node_mut(container).style.column_gap = Some(Length::px(20.0));
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    let span_el = add_block(&mut doc, container, 380.0, 20.0, Color::from_rgba8(33, 150, 243, 255));
    doc.node_mut(span_el).style.column_span = ColumnSpan::All;
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(76, 175, 80, 255));
    doc
}

fn sp12_multicol_column_fill_auto() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(150.0);
    doc.node_mut(container).style.column_count = Some(2);
    doc.node_mut(container).style.column_fill = ColumnFill::Auto;
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    doc
}

fn sp12_multicol_column_fill_balance() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.column_count = Some(2);
    doc.node_mut(container).style.column_fill = ColumnFill::Balance;
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(76, 175, 80, 255));
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Position Offset Tests ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_position_absolute_top_left() -> Document {
    let (mut doc, vp) = base_doc();
    doc.node_mut(vp).style.position = Position::Relative;
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::RED);
    doc.node_mut(div).style.position = Position::Absolute;
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.left = Length::px(10.0);
    doc
}

fn sp12_position_absolute_bottom_right() -> Document {
    let (mut doc, vp) = base_doc();
    // Use an explicit containing block with known dimensions
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    let div = add_block(&mut doc, container, 100.0, 100.0, Color::BLUE);
    doc.node_mut(div).style.position = Position::Absolute;
    doc.node_mut(div).style.bottom = Length::px(10.0);
    doc.node_mut(div).style.right = Length::px(10.0);
    doc
}

fn sp12_position_relative_top_left() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 100.0, 100.0, Color::from_rgba8(76, 175, 80, 255));
    doc.node_mut(div).style.position = Position::Relative;
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.left = Length::px(10.0);
    doc
}

fn sp12_position_absolute_percent() -> Document {
    let (mut doc, vp) = base_doc();
    // Use an explicit containing block with known dimensions
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    let div = add_block(&mut doc, container, 100.0, 100.0, Color::from_rgba8(156, 39, 176, 255));
    doc.node_mut(div).style.position = Position::Absolute;
    doc.node_mut(div).style.top = Length::percent(10.0);
    doc.node_mut(div).style.left = Length::percent(10.0);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Overflow Axis Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_overflow_x_hidden() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(150.0);
    doc.node_mut(outer).style.height = Length::px(80.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Hidden;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 300.0, 60.0, Color::RED);
    doc
}

fn sp12_overflow_y_hidden() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(150.0);
    doc.node_mut(outer).style.height = Length::px(80.0);
    doc.node_mut(outer).style.overflow_y = Overflow::Hidden;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 130.0, 200.0, Color::BLUE);
    doc
}

fn sp12_overflow_x_scroll() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(150.0);
    doc.node_mut(outer).style.height = Length::px(80.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Scroll;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 300.0, 60.0, Color::RED);
    doc
}

fn sp12_overflow_y_scroll() -> Document {
    let (mut doc, vp) = base_doc();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(150.0);
    doc.node_mut(outer).style.height = Length::px(80.0);
    doc.node_mut(outer).style.overflow_y = Overflow::Scroll;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, outer);
    add_block(&mut doc, outer, 130.0, 200.0, Color::BLUE);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Flex Alignment Extended Tests ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_flex_align_self_start() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(150.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    let item = add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    doc.node_mut(item).style.align_self = ItemAlignment::new(ItemPosition::FlexStart);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc
}

fn sp12_flex_align_self_end() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(150.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    let item = add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    doc.node_mut(item).style.align_self = ItemAlignment::new(ItemPosition::FlexEnd);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc
}

fn sp12_flex_align_self_center() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(150.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    let item = add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    doc.node_mut(item).style.align_self = ItemAlignment::new(ItemPosition::Center);
    add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc
}

fn sp12_flex_align_content_center() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.flex_wrap = FlexWrap::Wrap;
    doc.node_mut(flex).style.align_content = ContentAlignment::new(ContentPosition::Center);
    doc.node_mut(flex).style.width = Length::px(300.0);
    doc.node_mut(flex).style.height = Length::px(200.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 100.0, 50.0, Color::RED);
    add_block(&mut doc, flex, 100.0, 50.0, Color::BLUE);
    add_block(&mut doc, flex, 100.0, 50.0, Color::from_rgba8(0, 128, 0, 255));
    add_block(&mut doc, flex, 100.0, 50.0, Color::from_rgba8(255, 165, 0, 255));
    doc
}

fn sp12_flex_align_content_space_between() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.flex_wrap = FlexWrap::Wrap;
    doc.node_mut(flex).style.align_content =
        ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
    doc.node_mut(flex).style.width = Length::px(300.0);
    doc.node_mut(flex).style.height = Length::px(200.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    add_block(&mut doc, flex, 100.0, 50.0, Color::RED);
    add_block(&mut doc, flex, 100.0, 50.0, Color::BLUE);
    add_block(&mut doc, flex, 100.0, 50.0, Color::from_rgba8(0, 128, 0, 255));
    add_block(&mut doc, flex, 100.0, 50.0, Color::from_rgba8(255, 165, 0, 255));
    doc
}

fn sp12_flex_order() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    // DOM order: red, blue, green — but visual order: green(1), blue(2), red(3)
    let red = add_block(&mut doc, flex, 80.0, 60.0, Color::RED);
    doc.node_mut(red).style.order = 3;
    let blue = add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    doc.node_mut(blue).style.order = 2;
    let green = add_block(&mut doc, flex, 80.0, 60.0, Color::from_rgba8(0, 128, 0, 255));
    doc.node_mut(green).style.order = 1;
    doc
}

fn sp12_flex_basis_100px() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.width = Length::px(400.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    let item = doc.create_node(ElementTag::Div);
    doc.node_mut(item).style.display = Display::Block;
    doc.node_mut(item).style.flex_basis = Length::px(100.0);
    doc.node_mut(item).style.height = Length::px(60.0);
    doc.node_mut(item).style.background_color = Color::RED;
    doc.append_child(flex, item);
    add_block(&mut doc, flex, 80.0, 60.0, Color::BLUE);
    doc
}

fn sp12_flex_shrink_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let flex = doc.create_node(ElementTag::Div);
    doc.node_mut(flex).style.display = Display::Flex;
    doc.node_mut(flex).style.width = Length::px(200.0);
    doc.node_mut(flex).style.height = Length::px(80.0);
    doc.node_mut(flex).style.background_color = Color::from_rgba8(220, 220, 220, 255);
    doc.append_child(vp, flex);
    let a = doc.create_node(ElementTag::Div);
    doc.node_mut(a).style.display = Display::Block;
    doc.node_mut(a).style.width = Length::px(150.0);
    doc.node_mut(a).style.height = Length::px(60.0);
    doc.node_mut(a).style.flex_shrink = 1.0;
    doc.node_mut(a).style.background_color = Color::RED;
    doc.append_child(flex, a);
    let b = doc.create_node(ElementTag::Div);
    doc.node_mut(b).style.display = Display::Block;
    doc.node_mut(b).style.width = Length::px(150.0);
    doc.node_mut(b).style.height = Length::px(60.0);
    doc.node_mut(b).style.flex_shrink = 2.0;
    doc.node_mut(b).style.background_color = Color::BLUE;
    doc.append_child(flex, b);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Border Variant Tests ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_border_style_dashed() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::from_rgba8(240, 240, 240, 255));
    doc.node_mut(div).style.border_top_width = 3;
    doc.node_mut(div).style.border_right_width = 3;
    doc.node_mut(div).style.border_bottom_width = 3;
    doc.node_mut(div).style.border_left_width = 3;
    doc.node_mut(div).style.border_top_style = BorderStyle::Dashed;
    doc.node_mut(div).style.border_right_style = BorderStyle::Dashed;
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Dashed;
    doc.node_mut(div).style.border_left_style = BorderStyle::Dashed;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc
}

fn sp12_border_style_dotted() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::from_rgba8(240, 240, 240, 255));
    doc.node_mut(div).style.border_top_width = 3;
    doc.node_mut(div).style.border_right_width = 3;
    doc.node_mut(div).style.border_bottom_width = 3;
    doc.node_mut(div).style.border_left_width = 3;
    doc.node_mut(div).style.border_top_style = BorderStyle::Dotted;
    doc.node_mut(div).style.border_right_style = BorderStyle::Dotted;
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Dotted;
    doc.node_mut(div).style.border_left_style = BorderStyle::Dotted;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc
}

fn sp12_border_color_per_side() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_block(&mut doc, vp, 200.0, 100.0, Color::WHITE);
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_bottom_width = 5;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLUE);
    doc.node_mut(div).style.border_bottom_color =
        StyleColor::Resolved(Color::from_rgba8(0, 128, 0, 255));
    doc.node_mut(div).style.border_left_color =
        StyleColor::Resolved(Color::from_rgba8(255, 165, 0, 255));
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP12 Fragmentation Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp12_fragmentation_break_before_column() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.column_count = Some(2);
    doc.node_mut(container).style.column_gap = Some(Length::px(20.0));
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    let b = add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    doc.node_mut(b).style.break_before = BreakValue::Column;
    doc
}

fn sp12_fragmentation_break_inside_avoid() -> Document {
    let (mut doc, vp) = base_doc();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(120.0);
    doc.node_mut(container).style.column_count = Some(2);
    doc.node_mut(container).style.column_gap = Some(Length::px(20.0));
    doc.node_mut(container).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, container);
    let a = add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(244, 67, 54, 255));
    doc.node_mut(a).style.break_inside = BreakInside::Avoid;
    let b = add_block(&mut doc, container, 180.0, 50.0, Color::from_rgba8(33, 150, 243, 255));
    doc.node_mut(b).style.break_inside = BreakInside::Avoid;
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 First-Letter / First-Line Extended Tests ────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_first_letter_color() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.append_child(vp, div);
    let letter = doc.create_node(ElementTag::Span);
    doc.node_mut(letter).style.display = Display::Inline;
    doc.node_mut(letter).style.color = Color::RED;
    doc.node_mut(letter).text = Some("H".to_string());
    doc.append_child(div, letter);
    let rest = doc.create_node(ElementTag::Span);
    doc.node_mut(rest).style.display = Display::Inline;
    doc.node_mut(rest).text = Some("ello World".to_string());
    doc.append_child(div, rest);
    doc
}

fn sp13_first_letter_font_size() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.append_child(vp, div);
    let letter = doc.create_node(ElementTag::Span);
    doc.node_mut(letter).style.display = Display::Inline;
    doc.node_mut(letter).style.font_size = 32.0; // 2em at 16px base
    doc.node_mut(letter).text = Some("H".to_string());
    doc.append_child(div, letter);
    let rest = doc.create_node(ElementTag::Span);
    doc.node_mut(rest).style.display = Display::Inline;
    doc.node_mut(rest).text = Some("ello World".to_string());
    doc.append_child(div, rest);
    doc
}

fn sp13_first_line_font_weight() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(300.0);
    doc.append_child(vp, div);
    let first = doc.create_node(ElementTag::Span);
    doc.node_mut(first).style.display = Display::Inline;
    doc.node_mut(first).style.font_weight = FontWeight::BOLD;
    doc.node_mut(first).text = Some("The first line of text is bold".to_string());
    doc.append_child(div, first);
    let rest = doc.create_node(ElementTag::Span);
    doc.node_mut(rest).style.display = Display::Inline;
    doc.node_mut(rest).text =
        Some(" and the remaining text is normal weight for the second line.".to_string());
    doc.append_child(div, rest);
    doc
}

fn sp13_first_line_color() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(300.0);
    doc.append_child(vp, div);
    let first = doc.create_node(ElementTag::Span);
    doc.node_mut(first).style.display = Display::Inline;
    doc.node_mut(first).style.color = Color::BLUE;
    doc.node_mut(first).text = Some("The first line of text is blue colored".to_string());
    doc.append_child(div, first);
    let rest = doc.create_node(ElementTag::Span);
    doc.node_mut(rest).style.display = Display::Inline;
    doc.node_mut(rest).text =
        Some(" and the remaining text is the default black color for subsequent lines.".to_string());
    doc.append_child(div, rest);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Inline Advanced Tests ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_initial_letter_basic() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.append_child(vp, div);
    // Model the drop-cap with a float-left oversized letter
    let letter = doc.create_node(ElementTag::Span);
    doc.node_mut(letter).style.display = Display::InlineBlock;
    doc.node_mut(letter).style.font_size = 48.0; // ~3 lines tall
    doc.node_mut(letter).style.float = Float::Left;
    doc.node_mut(letter).style.line_height = LineHeight::Number(1.0);
    doc.node_mut(letter).style.margin_right = Length::px(4.0);
    doc.node_mut(letter).text = Some("L".to_string());
    doc.append_child(div, letter);
    let rest = doc.create_node(ElementTag::Span);
    doc.node_mut(rest).style.display = Display::Inline;
    doc.node_mut(rest).text = Some("orem ipsum dolor sit amet, consectetur adipiscing elit.".to_string());
    doc.append_child(div, rest);
    doc
}

fn sp13_ruby_basic() -> Document {
    let (mut doc, vp) = base_doc();
    // Model ruby annotation as a small span above base text using inline-block stacking
    for (base, ann) in [("漢", "かん"), ("字", "じ")] {
        let wrapper = doc.create_node(ElementTag::Div);
        doc.node_mut(wrapper).style.display = Display::InlineBlock;
        doc.node_mut(wrapper).style.font_size = 20.0;
        doc.append_child(vp, wrapper);

        let rt = doc.create_node(ElementTag::Div);
        doc.node_mut(rt).style.display = Display::Block;
        doc.node_mut(rt).style.font_size = 10.0;
        doc.node_mut(rt).text = Some(ann.to_string());
        doc.append_child(wrapper, rt);

        let base_span = doc.create_node(ElementTag::Div);
        doc.node_mut(base_span).style.display = Display::Block;
        doc.node_mut(base_span).text = Some(base.to_string());
        doc.append_child(wrapper, base_span);
    }
    doc
}

fn sp13_text_combine_upright() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.text_combine_upright = TextCombineUpright::All;
    doc.node_mut(div).text = Some("年".to_string());
    doc.append_child(vp, div);
    doc
}

fn sp13_inline_direction_rtl() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.direction = Direction::Rtl;
    doc.node_mut(div).style.width = Length::px(300.0);
    doc.node_mut(div).text = Some("Hello World".to_string());
    doc.append_child(vp, div);
    doc
}

fn sp13_tab_size_4() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.white_space = WhiteSpace::Pre;
    doc.node_mut(div).style.tab_size = TabSize::Spaces(4);
    doc.node_mut(div).text = Some("\tIndented with tab".to_string());
    doc.append_child(vp, div);
    doc
}

fn sp13_text_align_last_center() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(300.0);
    doc.node_mut(div).style.text_align_last = TextAlignLast::Center;
    doc.node_mut(div).style.background_color = Color::from_rgba8(230, 230, 230, 255);
    doc.node_mut(div).text = Some("Last line centered".to_string());
    doc.append_child(vp, div);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 Text Emphasis Tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp11_text_emphasis_dot() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(div).style.text_emphasis_mark = TextEmphasisMark::Dot;
    doc.node_mut(div).style.text_emphasis_fill = TextEmphasisFill::Filled;
    doc
}

fn sp11_text_emphasis_circle() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(div).style.text_emphasis_mark = TextEmphasisMark::Circle;
    doc.node_mut(div).style.text_emphasis_fill = TextEmphasisFill::Filled;
    doc
}

fn sp11_text_emphasis_position_over() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(div).style.text_emphasis_mark = TextEmphasisMark::Dot;
    doc.node_mut(div).style.text_emphasis_fill = TextEmphasisFill::Filled;
    doc.node_mut(div).style.text_emphasis_position = TextEmphasisPosition { over: true, right: true };
    doc
}

fn sp11_text_emphasis_color_red() -> Document {
    let (mut doc, vp) = base_doc();
    let div = add_text_block(&mut doc, vp, "Hello World");
    doc.node_mut(div).style.text_emphasis_mark = TextEmphasisMark::Dot;
    doc.node_mut(div).style.text_emphasis_fill = TextEmphasisFill::Filled;
    doc.node_mut(div).style.text_emphasis_color = StyleColor::Resolved(Color::RED);
    doc
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP13 Float Interaction Test ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn sp13_inline_with_float() -> Document {
    let (mut doc, vp) = base_doc();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(300.0);
    doc.node_mut(div).style.overflow_x = Overflow::Hidden;
    doc.node_mut(div).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, div);

    let float_box = doc.create_node(ElementTag::Div);
    doc.node_mut(float_box).style.display = Display::Block;
    doc.node_mut(float_box).style.float = Float::Left;
    doc.node_mut(float_box).style.width = Length::px(80.0);
    doc.node_mut(float_box).style.height = Length::px(80.0);
    doc.node_mut(float_box).style.background_color = Color::from_rgba8(255, 200, 200, 255);
    doc.node_mut(float_box).style.margin_right = Length::px(10.0);
    doc.append_child(div, float_box);

    let text = doc.create_node(ElementTag::Span);
    doc.node_mut(text).style.display = Display::Inline;
    doc.node_mut(text).text = Some(
        "Inline text wraps around the floated box. More text to ensure wrapping below the float."
            .to_string(),
    );
    doc.append_child(div, text);
    doc
}
