#!/usr/bin/env python3
"""
run_all_pixel_comparisons.py — Run pixel comparisons for ALL test patterns.

For each test in the pixel_compare binary:
1. Render with our engine → openui.png
2. Generate matching HTML → test.html
3. Render with Chrome → chromium.png
4. Diff → result.json + diff.png
"""

import json
import os
import subprocess
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(os.path.dirname(SCRIPT_DIR))
RESULTS_DIR = os.path.join(SCRIPT_DIR, "data", "pixel_comparison", "results")
PIXEL_COMPARE = os.path.join(PROJECT_ROOT, "bindings", "rust", "target", "release", "pixel_compare")
PIXEL_DIFF = os.path.join(SCRIPT_DIR, "pixel_diff.py")

# Chrome binary detection
CHROME_DIRS = [
    os.path.join(PROJECT_ROOT, "chrome", "linux-147.0.7727.50", "chrome-linux64"),
    os.path.join(PROJECT_ROOT, "chrome"),
]

BODY_STYLE = "* { margin: 0; padding: 0; box-sizing: content-box; } body { margin: 0; padding: 20px; font-family: DejaVu Sans, sans-serif; font-size: 16px; color: black; background-color: white; }"

# HTML templates for each test pattern.
# Each template is the inner <body> content.  BODY_STYLE provides the CSS reset
# (* { margin:0; padding:0; box-sizing:content-box }) and body defaults that
# mirror base_doc() in pixel-compare/src/main.rs.
#
# Rules applied when generating these templates:
#   - add_block(doc, parent, w, h, color) → <div style='width:Wpx;height:Hpx;background-color:COLOR;'>
#   - add_text_block(doc, parent, text)   → <div style='margin-bottom:10px;'>TEXT</div>
#   - ElementTag::Div  → <div>   (display:block by default)
#   - ElementTag::Span → <span>  (display:inline by default)
#   - Only use <p>/<pre> if the Rust builder explicitly creates them (none do).
#   - When the viewport (vp) needs extra styles (e.g. position:relative), a
#     <style>body{…}</style> override is prepended to the template.

HTML_TEMPLATES = {
    # ── SP12 Display ─────────────────────────────────────────────────
    "sp12/display_outer_block": "<div style='width:200px;height:100px;background-color:red;'></div>",
    "sp12/display_outer_inline": "<span style='background-color:rgb(0,128,0);'>Inline element text</span>",
    "sp12/display_outer_inline_block": "<div style='display:inline-block;width:150px;height:80px;background-color:blue;'></div>",
    "sp12/display_outer_none": "<div style='display:none;width:200px;height:100px;background-color:red;'></div>",
    "sp12/display_inner_flow_root": "<div style='display:flow-root;width:200px;height:100px;background-color:teal;'></div>",

    # ── SP12 Position ────────────────────────────────────────────────
    "sp12/position_static": "<div style='width:200px;height:100px;background-color:red;'></div><div style='width:200px;height:100px;background-color:blue;'></div>",
    "sp12/position_relative": "<div style='width:100px;height:100px;background-color:blue;position:relative;top:20px;left:30px;'></div>",
    "sp12/position_absolute": "<style>body{position:relative;}</style><div style='width:100px;height:100px;background-color:red;position:absolute;top:50px;left:50px;'></div>",
    "sp12/position_fixed": "<div style='width:100px;height:100px;background-color:red;position:fixed;top:10px;right:10px;'></div>",

    # ── SP12 Float ───────────────────────────────────────────────────
    "sp12/float_left": "<div style='width:100px;height:100px;background-color:green;float:left;'></div><span>Text wrapping around a left-floated element. The text should flow to the right of the green box.</span>",
    "sp12/float_right": "<div style='width:100px;height:100px;background-color:green;float:right;'></div><span>Text wrapping around a right-floated element. The text should flow to the left of the green box.</span>",
    "sp12/float_none": "<div style='width:100px;height:100px;background-color:green;float:none;'></div>",
    "sp12/clear_left": "<div style='width:100px;height:80px;background-color:red;float:left;'></div><div style='width:200px;height:50px;background-color:blue;clear:left;'></div>",
    "sp12/clear_right": "<div style='width:100px;height:80px;background-color:red;float:right;'></div><div style='width:200px;height:50px;background-color:blue;clear:right;'></div>",
    "sp12/clear_both": "<div style='width:100px;height:80px;background-color:red;float:left;'></div><div style='width:100px;height:60px;background-color:green;float:right;'></div><div style='width:200px;height:50px;background-color:blue;clear:both;'></div>",

    # ── SP12 Box Model ───────────────────────────────────────────────
    "sp12/margin_positive": "<div style='width:100px;height:100px;background-color:red;margin:20px;'></div><div style='width:100px;height:50px;background-color:blue;'></div>",
    "sp12/margin_negative": "<div style='width:200px;height:100px;background-color:red;'></div><div style='width:200px;height:100px;background-color:blue;margin-top:-30px;'></div>",
    "sp12/margin_auto": "<div style='width:200px;height:100px;background-color:red;margin-left:auto;margin-right:auto;'></div>",
    "sp12/margin_collapsing_siblings": "<div style='width:200px;height:50px;background-color:red;margin-bottom:30px;'></div><div style='width:200px;height:50px;background-color:blue;margin-top:20px;'></div>",
    "sp12/padding_basic": "<div style='background-color:rgb(200,200,200);padding:20px 30px;'><div style='width:100px;height:60px;background-color:red;'></div></div>",
    "sp12/border_basic": "<div style='width:150px;height:100px;background-color:rgb(240,240,240);border:3px solid black;'></div>",
    "sp12/box_sizing_content_box": "<div style='width:200px;height:100px;background-color:red;box-sizing:content-box;padding:10px;border:2px solid black;'></div>",
    "sp12/box_sizing_border_box": "<div style='width:200px;height:100px;background-color:red;box-sizing:border-box;padding:10px;border:2px solid black;'></div>",

    # ── SP12 Sizing ──────────────────────────────────────────────────
    "sp12/width_fixed_px": "<div style='width:300px;height:100px;background-color:red;'></div>",
    "sp12/height_fixed_px": "<div style='width:200px;height:150px;background-color:blue;'></div>",
    "sp12/width_percent": "<div style='width:50%;height:100px;background-color:red;'></div>",
    "sp12/min_width": "<div style='width:50px;min-width:200px;height:100px;background-color:red;'></div>",
    "sp12/max_width": "<div style='width:500px;max-width:200px;height:100px;background-color:red;'></div>",
    "sp12/min_height": "<div style='width:200px;height:30px;min-height:100px;background-color:blue;'></div>",
    "sp12/max_height": "<div style='width:200px;height:500px;max-height:100px;background-color:blue;'></div>",

    # ── SP12 Overflow ────────────────────────────────────────────────
    "sp12/overflow_visible": "<div style='width:100px;height:50px;overflow:visible;background-color:rgb(200,200,200);'><div style='width:200px;height:200px;background-color:red;'></div></div>",
    "sp12/overflow_hidden": "<div style='width:100px;height:50px;overflow:hidden;background-color:rgb(200,200,200);'><div style='width:200px;height:200px;background-color:red;'></div></div>",

    # ── SP12 Flexbox ─────────────────────────────────────────────────
    "sp12/flex_direction_row": "<div style='display:flex;flex-direction:row;width:400px;height:100px;background-color:rgb(220,220,220);'><div style='width:80px;height:60px;background-color:red;'></div><div style='width:80px;height:60px;background-color:blue;'></div><div style='width:80px;height:60px;background-color:green;'></div></div>",
    "sp12/flex_direction_column": "<div style='display:flex;flex-direction:column;width:200px;height:300px;background-color:rgb(220,220,220);'><div style='width:80px;height:60px;background-color:red;'></div><div style='width:80px;height:60px;background-color:blue;'></div><div style='width:80px;height:60px;background-color:green;'></div></div>",
    "sp12/flex_justify_start": "<div style='display:flex;justify-content:flex-start;width:400px;height:80px;background-color:rgb(220,220,220);'><div style='width:60px;height:60px;background-color:red;'></div><div style='width:60px;height:60px;background-color:blue;'></div></div>",
    "sp12/flex_justify_center": "<div style='display:flex;justify-content:center;width:400px;height:80px;background-color:rgb(220,220,220);'><div style='width:60px;height:60px;background-color:red;'></div><div style='width:60px;height:60px;background-color:blue;'></div></div>",
    "sp12/flex_justify_space_between": "<div style='display:flex;justify-content:space-between;width:400px;height:80px;background-color:rgb(220,220,220);'><div style='width:60px;height:60px;background-color:red;'></div><div style='width:60px;height:60px;background-color:blue;'></div><div style='width:60px;height:60px;background-color:green;'></div></div>",
    "sp12/flex_align_center": "<div style='display:flex;align-items:center;width:400px;height:150px;background-color:rgb(220,220,220);'><div style='width:80px;height:40px;background-color:red;'></div><div style='width:80px;height:80px;background-color:blue;'></div><div style='width:80px;height:60px;background-color:green;'></div></div>",
    "sp12/flex_align_stretch": "<div style='display:flex;align-items:stretch;width:400px;height:150px;background-color:rgb(220,220,220);'><div style='width:80px;background-color:red;'></div><div style='width:80px;background-color:blue;'></div></div>",
    "sp12/flex_wrap_basic": "<div style='display:flex;flex-wrap:wrap;width:200px;background-color:rgb(220,220,220);'><div style='width:80px;height:50px;background-color:red;'></div><div style='width:80px;height:50px;background-color:blue;'></div><div style='width:80px;height:50px;background-color:green;'></div><div style='width:80px;height:50px;background-color:orange;'></div></div>",
    "sp12/flex_grow_equal": "<div style='display:flex;width:400px;height:80px;background-color:rgb(220,220,220);'><div style='flex-grow:1;height:60px;background-color:red;'></div><div style='flex-grow:1;height:60px;background-color:blue;'></div><div style='flex-grow:1;height:60px;background-color:green;'></div></div>",
    "sp12/flex_gap": "<div style='display:flex;column-gap:20px;width:400px;height:80px;background-color:rgb(220,220,220);'><div style='width:80px;height:60px;background-color:red;'></div><div style='width:80px;height:60px;background-color:blue;'></div><div style='width:80px;height:60px;background-color:green;'></div></div>",

    # ── SP12 Visual / Stacking ───────────────────────────────────────
    "sp12/z_index_stacking": "<style>body{position:relative;}</style><div style='width:150px;height:150px;background-color:red;position:absolute;top:20px;left:20px;z-index:1;'></div><div style='width:150px;height:150px;background-color:blue;position:absolute;top:60px;left:60px;z-index:2;'></div>",
    "sp12/opacity_basic": "<div style='width:200px;height:100px;background-color:red;opacity:0.5;'></div><div style='width:200px;height:100px;background-color:blue;'></div>",
    "sp12/border_radius": "<div style='width:200px;height:200px;background-color:red;border-radius:20px;'></div>",
    "sp12/visibility_hidden": "<div style='width:200px;height:50px;background-color:red;'></div><div style='width:200px;height:50px;background-color:blue;visibility:hidden;'></div><div style='width:200px;height:50px;background-color:green;'></div>",
    "sp12/nested_blocks": "<div style='width:300px;padding:10px;background-color:rgb(200,200,200);'><div style='padding:10px;background-color:rgb(150,150,200);'><div style='width:100px;height:60px;background-color:red;'></div><div style='width:100px;height:60px;background-color:blue;'></div></div></div>",

    # ── SP12 Sticky Positioning ──────────────────────────────────────
    "sp12/position_sticky_top": "<div style='width:400px;height:300px;background-color:#f0f0f0;'><div style='position:sticky;top:10px;width:100px;height:30px;background-color:#4CAF50;'></div></div>",
    "sp12/position_sticky_bottom": "<div style='width:400px;height:300px;background-color:#f0f0f0;'><div style='position:sticky;bottom:10px;width:100px;height:30px;background-color:#4CAF50;'></div></div>",

    # ── SP12 Multicol ────────────────────────────────────────────────
    "sp12/multicol_2_columns": "<div style='column-count:2;column-gap:20px;width:400px;background-color:#f0f0f0;'><div style='width:180px;height:50px;background-color:#F44336;'></div><div style='width:180px;height:50px;background-color:#4CAF50;'></div><div style='width:180px;height:50px;background-color:#2196F3;'></div><div style='width:180px;height:50px;background-color:#FF9800;'></div></div>",
    "sp12/multicol_column_width": "<div style='column-width:150px;width:400px;background-color:#f0f0f0;'><div style='width:140px;height:50px;background-color:#F44336;'></div><div style='width:140px;height:50px;background-color:#4CAF50;'></div><div style='width:140px;height:50px;background-color:#2196F3;'></div><div style='width:140px;height:50px;background-color:#FF9800;'></div></div>",
    "sp12/multicol_column_gap": "<div style='column-count:2;column-gap:40px;width:400px;background-color:#f0f0f0;'><div style='width:170px;height:50px;background-color:#F44336;'></div><div style='width:170px;height:50px;background-color:#4CAF50;'></div><div style='width:170px;height:50px;background-color:#2196F3;'></div><div style='width:170px;height:50px;background-color:#FF9800;'></div></div>",

    # ── SP12 Flex Advanced ───────────────────────────────────────────
    "sp12/flex_direction_row_reverse": "<div style='display:flex;flex-direction:row-reverse;width:400px;height:100px;background-color:rgb(220,220,220);'><div style='width:80px;height:60px;background-color:#F44336;'></div><div style='width:80px;height:60px;background-color:#4CAF50;'></div><div style='width:80px;height:60px;background-color:#2196F3;'></div></div>",
    "sp12/flex_direction_column_reverse": "<div style='display:flex;flex-direction:column-reverse;width:200px;height:300px;background-color:rgb(220,220,220);'><div style='width:80px;height:60px;background-color:#F44336;'></div><div style='width:80px;height:60px;background-color:#4CAF50;'></div><div style='width:80px;height:60px;background-color:#2196F3;'></div></div>",
    "sp12/flex_wrap_reverse": "<div style='display:flex;flex-wrap:wrap-reverse;width:200px;background-color:rgb(220,220,220);'><div style='width:80px;height:60px;background-color:#F44336;'></div><div style='width:80px;height:60px;background-color:#4CAF50;'></div><div style='width:80px;height:60px;background-color:#2196F3;'></div><div style='width:80px;height:60px;background-color:#FF9800;'></div></div>",
    "sp12/flex_justify_space_around": "<div style='display:flex;justify-content:space-around;width:400px;height:80px;background-color:rgb(220,220,220);'><div style='width:60px;height:40px;background-color:#F44336;'></div><div style='width:60px;height:40px;background-color:#4CAF50;'></div><div style='width:60px;height:40px;background-color:#2196F3;'></div></div>",
    "sp12/flex_justify_space_evenly": "<div style='display:flex;justify-content:space-evenly;width:400px;height:80px;background-color:rgb(220,220,220);'><div style='width:60px;height:40px;background-color:#F44336;'></div><div style='width:60px;height:40px;background-color:#4CAF50;'></div><div style='width:60px;height:40px;background-color:#2196F3;'></div></div>",

    # ── SP12 Margin Collapsing Advanced ──────────────────────────────
    "sp12/margin_collapsing_parent_child": "<div style='width:200px;background-color:rgb(200,200,200);'><div style='width:100px;height:50px;background-color:#F44336;margin-top:30px;'></div></div><div style='width:200px;height:50px;background-color:#2196F3;'></div>",
    "sp12/margin_collapsing_through_empty": "<div style='width:200px;height:50px;background-color:#F44336;margin-bottom:20px;'></div><div style='margin-top:15px;margin-bottom:25px;'></div><div style='width:200px;height:50px;background-color:#2196F3;margin-top:10px;'></div>",

    # ── SP12 Overflow Axes ───────────────────────────────────────────
    "sp12/overflow_scroll": "<div style='width:200px;height:100px;overflow:scroll;background-color:#f0f0f0;'><div style='width:180px;height:200px;background-color:#F44336;'></div></div>",
    "sp12/overflow_auto": "<div style='width:200px;height:100px;overflow:auto;background-color:#f0f0f0;'><div style='width:180px;height:200px;background-color:#F44336;'></div></div>",

    # ── SP12 Aspect Ratio ────────────────────────────────────────────
    "sp12/aspect_ratio_basic": "<div style='width:200px;aspect-ratio:2/1;background-color:#9C27B0;'></div>",

    # ── SP11 Text Decoration ─────────────────────────────────────────
    "sp11/text_decoration_underline": "<div style='margin-bottom:10px;text-decoration-line:underline;'>This text has an underline decoration</div>",
    "sp11/text_decoration_overline": "<div style='margin-bottom:10px;text-decoration-line:overline;'>This text has an overline decoration</div>",
    "sp11/text_decoration_line_through": "<div style='margin-bottom:10px;text-decoration-line:line-through;'>This text has a line-through decoration</div>",
    "sp11/text_decoration_combined": "<div style='margin-bottom:10px;text-decoration-line:underline overline line-through;'>This text has underline + overline + line-through</div>",

    # ── SP11 Font Weight & Style ─────────────────────────────────────
    "sp11/font_weight_normal": "<div style='margin-bottom:10px;font-weight:normal;'>Normal weight (400) text sample</div>",
    "sp11/font_weight_bold": "<div style='margin-bottom:10px;font-weight:bold;'>Bold weight (700) text sample</div>",
    "sp11/font_style_normal": "<div style='margin-bottom:10px;font-style:normal;'>Normal style text sample</div>",
    "sp11/font_style_italic": "<div style='margin-bottom:10px;font-style:italic;'>Italic style text sample</div>",

    # ── SP11 Font Size ───────────────────────────────────────────────
    "sp11/font_size_small": "<div style='margin-bottom:10px;font-size:12px;'>Small text at 12px font size</div>",
    "sp11/font_size_medium": "<div style='margin-bottom:10px;font-size:16px;'>Medium text at 16px font size (default)</div>",
    "sp11/font_size_large": "<div style='margin-bottom:10px;font-size:24px;'>Large text at 24px font size</div>",
    "sp11/font_size_xlarge": "<div style='margin-bottom:10px;font-size:32px;'>Extra large text at 32px</div>",

    # ── SP11 Text Align ──────────────────────────────────────────────
    "sp11/text_align_left": "<div style='margin-bottom:10px;text-align:left;width:400px;background-color:rgb(230,230,230);'>Left-aligned text in a block</div>",
    "sp11/text_align_center": "<div style='margin-bottom:10px;text-align:center;width:400px;background-color:rgb(230,230,230);'>Center-aligned text in a block</div>",
    "sp11/text_align_right": "<div style='margin-bottom:10px;text-align:right;width:400px;background-color:rgb(230,230,230);'>Right-aligned text in a block</div>",
    "sp11/text_align_justify": "<div style='margin-bottom:10px;text-align:justify;width:300px;background-color:rgb(230,230,230);'>Justified text stretches words across the full width of the container block so that both edges are flush.</div>",

    # ── SP11 Text Transform ──────────────────────────────────────────
    "sp11/text_transform_uppercase": "<div style='margin-bottom:10px;text-transform:uppercase;'>this text should be uppercase</div>",
    "sp11/text_transform_lowercase": "<div style='margin-bottom:10px;text-transform:lowercase;'>THIS TEXT SHOULD BE LOWERCASE</div>",
    "sp11/text_transform_capitalize": "<div style='margin-bottom:10px;text-transform:capitalize;'>capitalize each word in this sentence</div>",

    # ── SP11 Text Indent ─────────────────────────────────────────────
    "sp11/text_indent_positive": "<div style='margin-bottom:10px;text-indent:40px;width:300px;background-color:rgb(230,230,230);'>This paragraph has a positive 40px text-indent on the first line. The second line wraps normally without indent.</div>",
    "sp11/text_indent_negative": "<div style='margin-bottom:10px;text-indent:-20px;width:300px;padding-left:30px;background-color:rgb(230,230,230);'>This paragraph has a negative -20px text-indent (hanging indent) on the first line.</div>",

    # ── SP11 Letter & Word Spacing ───────────────────────────────────
    "sp11/letter_spacing_positive": "<div style='margin-bottom:10px;letter-spacing:5px;'>Wide letter spacing</div><div style='margin-bottom:10px;'>Normal letter spacing for comparison</div>",
    "sp11/letter_spacing_negative": "<div style='margin-bottom:10px;letter-spacing:-1px;'>Tight letter spacing</div><div style='margin-bottom:10px;'>Normal letter spacing for comparison</div>",
    "sp11/word_spacing_positive": "<div style='margin-bottom:10px;word-spacing:15px;'>Extra space between words in this sentence</div><div style='margin-bottom:10px;'>Normal word spacing for comparison</div>",

    # ── SP11 Line Height ─────────────────────────────────────────────
    "sp11/line_height_normal": "<div style='margin-bottom:10px;line-height:normal;width:300px;background-color:rgb(230,230,230);'>Line height normal. This is a multi-line paragraph to demonstrate the default line spacing between lines of text.</div>",
    "sp11/line_height_number": "<div style='margin-bottom:10px;line-height:2;width:300px;background-color:rgb(230,230,230);'>Line height 2.0. This is a multi-line paragraph to demonstrate double line spacing between lines of text.</div>",
    "sp11/line_height_length": "<div style='margin-bottom:10px;line-height:30px;width:300px;background-color:rgb(230,230,230);'>Line height 30px. This is a multi-line paragraph to demonstrate fixed 30px line spacing between lines.</div>",

    # ── SP11 White Space ─────────────────────────────────────────────
    "sp11/white_space_normal": "<div style='margin-bottom:10px;white-space:normal;width:300px;background-color:rgb(230,230,230);'>White space   normal:   multiple    spaces   and\nnewlines   collapse   into   single   spaces.</div>",
    "sp11/white_space_nowrap": "<div style='margin-bottom:10px;white-space:nowrap;width:200px;background-color:rgb(230,230,230);'>White space nowrap: this long text should not wrap to the next line even if it overflows the container.</div>",
    "sp11/white_space_pre": "<div style='margin-bottom:10px;white-space:pre;background-color:rgb(230,230,230);'>White space pre:\n  indented line\n  preserves   spaces\n    and newlines</div>",
    "sp11/white_space_pre_wrap": "<div style='margin-bottom:10px;white-space:pre-wrap;width:300px;background-color:rgb(230,230,230);'>White space pre-wrap:\n  preserves   spaces\n  but also   wraps   long lines when they exceed the container width limit.</div>",
    "sp11/white_space_pre_line": "<div style='margin-bottom:10px;white-space:pre-line;width:300px;background-color:rgb(230,230,230);'>White space pre-line:\n  collapses   spaces\n  but   preserves\n  newlines and wraps.</div>",

    # ── SP11 Color ───────────────────────────────────────────────────
    "sp11/color_red": "<div style='margin-bottom:10px;color:red;'>This text is rendered in red color</div>",
    "sp11/color_blue": "<div style='margin-bottom:10px;color:blue;'>This text is rendered in blue color</div>",
    "sp11/color_green": "<div style='margin-bottom:10px;color:green;'>This text is rendered in green color</div>",
    "sp11/color_custom": "<div style='margin-bottom:10px;color:rgb(139,0,139);'>This text is rendered in custom purple (#8B008B)</div>",

    # ── SP11 Text Shadow & Overflow ──────────────────────────────────
    "sp11/text_shadow_basic": "<div style='margin-bottom:10px;font-size:24px;text-shadow:2px 2px 4px rgba(0,0,0,0.502);'>Text with a shadow effect</div>",
    "sp11/text_overflow_ellipsis": "<div style='margin-bottom:10px;width:200px;white-space:nowrap;overflow-x:hidden;text-overflow:ellipsis;background-color:rgb(230,230,230);'>This text overflows its container and should show an ellipsis at the end</div>",

    # ── SP11 Text Decoration Style ──────────────────────────────────
    "sp11/text_decoration_style_solid": "<div style='margin-bottom:10px;text-decoration:underline;text-decoration-style:solid;'>Hello World</div>",
    "sp11/text_decoration_style_double": "<div style='margin-bottom:10px;text-decoration:underline;text-decoration-style:double;'>Hello World</div>",
    "sp11/text_decoration_style_dotted": "<div style='margin-bottom:10px;text-decoration:underline;text-decoration-style:dotted;'>Hello World</div>",
    "sp11/text_decoration_style_dashed": "<div style='margin-bottom:10px;text-decoration:underline;text-decoration-style:dashed;'>Hello World</div>",
    "sp11/text_decoration_style_wavy": "<div style='margin-bottom:10px;text-decoration:underline;text-decoration-style:wavy;'>Hello World</div>",

    # ── SP11 Text Decoration Skip-Ink ───────────────────────────────
    "sp11/text_decoration_skip_ink_auto": "<div style='margin-bottom:10px;text-decoration:underline;text-decoration-skip-ink:auto;'>Typography</div>",
    "sp11/text_decoration_skip_ink_none": "<div style='margin-bottom:10px;text-decoration:underline;text-decoration-skip-ink:none;'>Typography</div>",

    # ── SP11 Text Decoration Metrics ────────────────────────────────
    "sp11/text_decoration_color_red": "<div style='margin-bottom:10px;color:blue;text-decoration:underline;text-decoration-color:red;'>Hello World</div>",
    "sp11/text_decoration_thickness_3px": "<div style='margin-bottom:10px;text-decoration:underline;text-decoration-thickness:3px;'>Hello World</div>",
    "sp11/text_underline_offset_5px": "<div style='margin-bottom:10px;text-decoration:underline;text-underline-offset:5px;'>Hello World</div>",

    # ── SP11 Font Family ────────────────────────────────────────────
    "sp11/font_family_serif": "<div style='margin-bottom:10px;font-family:serif;'>Hello World</div>",
    "sp11/font_family_monospace": "<div style='margin-bottom:10px;font-family:monospace;'>Hello World</div>",

    # ── SP11 Word Spacing Negative ──────────────────────────────────
    "sp11/word_spacing_negative": "<div style='margin-bottom:10px;word-spacing:-3px;'>The quick brown fox</div>",

    # ── SP11 Line Height Percentage ─────────────────────────────────
    "sp11/line_height_percentage": "<div style='margin-bottom:10px;line-height:200%;width:300px;background-color:rgb(230,230,230);'>Line height 200%. This is a multi-line paragraph to demonstrate percentage-based line spacing between lines of text.</div>",

    # ── SP11 Text Shadow Offset ─────────────────────────────────────
    "sp11/text_shadow_offset": "<div style='margin-bottom:10px;text-shadow:3px 3px 0 red;'>Shadow</div>",

    # ── SP13 Inline Basic ────────────────────────────────────────────
    "sp13/inline_single_span": "<span style='color:red;'>A single inline span with red text</span>",
    "sp13/inline_multiple_spans": "<span style='color:red;'>First span </span><span style='color:blue;'>Second span </span><span style='color:green;'>Third span</span>",
    "sp13/inline_nested_spans": "<span style='color:blue;'>Outer <span style='color:red;font-weight:bold;'>inner bold red</span></span><span style='color:blue;'> outer again</span>",

    # ── SP13 Line Breaking ───────────────────────────────────────────
    "sp13/line_breaking_normal_wrap": "<div style='width:200px;background-color:rgb(230,230,230);'><span>This is a long line of text that should naturally wrap at word boundaries within the container</span></div>",
    "sp13/line_breaking_nowrap": "<div style='width:200px;white-space:nowrap;overflow-x:hidden;background-color:rgb(230,230,230);'><span>This text should not wrap and may be clipped by overflow hidden</span></div>",
    "sp13/line_breaking_break_word": "<div style='width:150px;overflow-wrap:break-word;background-color:rgb(230,230,230);'><span>Supercalifragilisticexpialidocious should break mid-word</span></div>",

    # ── SP13 Vertical Align ──────────────────────────────────────────
    "sp13/vertical_align_baseline": "<div style='background-color:rgb(230,230,230);line-height:60px;'><span style='font-size:32px;'>Big </span><span style='font-size:12px;vertical-align:baseline;background-color:rgb(255,200,200);'>baseline</span></div>",
    "sp13/vertical_align_middle": "<div style='background-color:rgb(230,230,230);line-height:60px;'><span style='font-size:32px;'>Big </span><span style='font-size:12px;vertical-align:middle;background-color:rgb(255,200,200);'>middle</span></div>",
    "sp13/vertical_align_top": "<div style='background-color:rgb(230,230,230);line-height:60px;'><span style='font-size:32px;'>Big </span><span style='font-size:12px;vertical-align:top;background-color:rgb(255,200,200);'>top</span></div>",
    "sp13/vertical_align_bottom": "<div style='background-color:rgb(230,230,230);line-height:60px;'><span style='font-size:32px;'>Big </span><span style='font-size:12px;vertical-align:bottom;background-color:rgb(255,200,200);'>bottom</span></div>",
    "sp13/vertical_align_super": "<div style='background-color:rgb(230,230,230);line-height:60px;'><span style='font-size:32px;'>Big </span><span style='font-size:12px;vertical-align:super;background-color:rgb(255,200,200);'>super</span></div>",
    "sp13/vertical_align_sub": "<div style='background-color:rgb(230,230,230);line-height:60px;'><span style='font-size:32px;'>Big </span><span style='font-size:12px;vertical-align:sub;background-color:rgb(255,200,200);'>sub</span></div>",

    # ── SP13 Inline Block ────────────────────────────────────────────
    "sp13/inline_block_basic": "<span>Text before </span><div style='display:inline-block;width:80px;height:40px;background-color:red;'></div><span> text after</span>",
    "sp13/inline_block_vertical_align": "<div style='background-color:rgb(230,230,230);'><span>Aligned: </span><div style='display:inline-block;width:60px;height:60px;background-color:blue;vertical-align:middle;'></div><span> middle-aligned inline-block</span></div>",

    # ── SP13 Mixed Content ───────────────────────────────────────────
    "sp13/mixed_block_inline": "<div style='width:300px;height:40px;background-color:rgb(200,220,255);'><span>Block element with text</span></div><span style='color:red;'>Inline span after block </span><div style='width:300px;height:40px;background-color:rgb(220,255,200);'></div>",

    # ── SP13 White Space Handling ────────────────────────────────────
    "sp13/white_space_collapsing": "<div style='width:300px;background-color:rgb(230,230,230);'><span>  Multiple  </span><span>  spaces  </span><span>  should  </span><span>  collapse  </span></div>",
    "sp13/white_space_preserving": "<div style='width:400px;white-space:pre;background-color:rgb(230,230,230);'><span>  Preserved   spaces   and\n  newlines  </span></div>",

    # ── SP13 Inline Decoration ───────────────────────────────────────
    "sp13/inline_background_color": "<span>Normal text </span><span style='background-color:yellow;'>highlighted span</span><span> normal text</span>",
    "sp13/inline_padding": "<span>Before </span><span style='padding:4px 12px;background-color:rgb(200,230,255);'>padded inline</span><span> after</span>",
    "sp13/inline_border": "<span>Before </span><span style='border:2px solid red;padding-left:6px;padding-right:6px;'>bordered inline</span><span> after</span>",

    # ── SP13 First-Letter / First-Line ─────────────────────────────────
    "sp13/first_letter_basic": "<div><span style='font-size:2em;color:#F44336;'>L</span>orem ipsum dolor sit amet</div>",
    "sp13/first_line_basic": "<div style='width:250px;'><p style='width:250px;'><span style='color:#2196F3;font-weight:bold;'>The first line is styled differently</span> and the remaining text uses the default paragraph style for subsequent lines</p></div>",

    # ── SP13 Word Break ────────────────────────────────────────────────
    "sp13/word_break_break_all": "<div style='width:100px;word-break:break-all;background-color:rgb(230,230,230);'>Supercalifragilisticexpialidocious</div>",
    "sp13/overflow_wrap_break_word": "<div style='width:100px;overflow-wrap:break-word;background-color:rgb(230,230,230);'>Supercalifragilisticexpialidocious</div>",

    # ── SP13 Vertical Align Extended ───────────────────────────────────
    "sp13/vertical_align_text_top": "<div style='background-color:rgb(230,230,230);line-height:60px;'><span style='font-size:32px;'>Big </span><span style='font-size:12px;vertical-align:text-top;background-color:rgb(255,200,200);'>text-top</span></div>",
    "sp13/vertical_align_text_bottom": "<div style='background-color:rgb(230,230,230);line-height:60px;'><span style='font-size:32px;'>Big </span><span style='font-size:12px;vertical-align:text-bottom;background-color:rgb(255,200,200);'>text-bottom</span></div>",

    # ── SP13 Line Breaking Extended ────────────────────────────────────
    "sp13/line_breaking_overflow_wrap": "<div style='width:150px;overflow-wrap:anywhere;background-color:rgb(230,230,230);'>https://example.com/very/long/path/to/resource/that/should/break</div>",
    "sp13/line_breaking_hyphens_auto": "<div style='width:120px;hyphens:auto;background-color:rgb(230,230,230);' lang='en'>Incomprehensibilities and internationalization are long words</div>",

    # ── SP13 Box Decoration Break ──────────────────────────────────────
    "sp13/inline_box_multiline": "<div style='width:200px;'><span style='background-color:rgb(200,230,255);padding:4px 8px;box-decoration-break:clone;-webkit-box-decoration-break:clone;'>This inline span has background and padding and wraps to multiple lines</span></div>",

    # ── SP13 Float Interaction ─────────────────────────────────────────
    "sp13/inline_with_float": "<div style='width:300px;overflow:hidden;'><div style='float:left;width:80px;height:80px;background-color:rgb(255,200,200);margin-right:10px;'></div><span>Inline text wraps around the floated box. More text to ensure wrapping below the float.</span></div>",
}


def find_chrome():
    """Find Chrome binary."""
    for d in CHROME_DIRS:
        chrome = os.path.join(d, "chrome")
        if os.path.isfile(chrome):
            return chrome, d
    # Try system chrome
    for name in ["google-chrome", "chrome", "chromium-browser"]:
        try:
            path = subprocess.check_output(["which", name], stderr=subprocess.DEVNULL).decode().strip()
            return path, os.path.dirname(path)
        except (subprocess.CalledProcessError, FileNotFoundError):
            pass
    return None, None


def render_chrome(html_file, output_png, chrome_bin, chrome_dir):
    """Render HTML with Chrome headless."""
    env = os.environ.copy()
    if chrome_dir:
        env["LD_LIBRARY_PATH"] = chrome_dir + ":" + env.get("LD_LIBRARY_PATH", "")
    cmd = [
        chrome_bin, "--headless", "--disable-gpu", "--no-sandbox",
        "--force-device-scale-factor=1", "--window-size=800,600",
        f"--screenshot={output_png}", f"file://{html_file}"
    ]
    result = subprocess.run(cmd, env=env, capture_output=True, timeout=30)
    return result.returncode == 0 and os.path.isfile(output_png)


def render_openui(test_id, output_png):
    """Render test pattern with our engine."""
    result = subprocess.run(
        [PIXEL_COMPARE, "render", test_id, output_png],
        capture_output=True, timeout=30
    )
    return result.returncode == 0 and os.path.isfile(output_png)


def pixel_diff(img_a, img_b, diff_out, result_out):
    """Compare two images pixel-by-pixel."""
    result = subprocess.run(
        ["python3", PIXEL_DIFF, img_a, img_b, diff_out, result_out],
        capture_output=True, timeout=30
    )
    # pixel_diff.py exits 0 for pass, 1 for fail — both produce valid JSON
    if os.path.isfile(result_out):
        with open(result_out) as f:
            return json.load(f)
    return None


def main():
    chrome_bin, chrome_dir = find_chrome()
    if not chrome_bin:
        print("ERROR: Chrome binary not found", file=sys.stderr)
        sys.exit(1)
    print(f"Chrome: {chrome_bin}")

    if not os.path.isfile(PIXEL_COMPARE):
        print(f"ERROR: pixel_compare binary not found at {PIXEL_COMPARE}", file=sys.stderr)
        print("Build with: cd bindings/rust && cargo build --release --package pixel-compare", file=sys.stderr)
        sys.exit(1)

    # Get all test IDs
    result = subprocess.run([PIXEL_COMPARE, "list"], capture_output=True, text=True)
    all_tests = result.stdout.strip().split("\n")
    print(f"Total tests: {len(all_tests)}")

    # Filter to only tests with HTML templates
    tests_with_html = [t for t in all_tests if t in HTML_TEMPLATES]
    tests_without_html = [t for t in all_tests if t not in HTML_TEMPLATES]

    if tests_without_html:
        print(f"\nERROR: {len(tests_without_html)} tests WITHOUT HTML templates:")
        for t in tests_without_html:
            print(f"  - {t}")
        print("\nEvery registered test MUST have a matching HTML template.")
        print("Add missing templates to HTML_TEMPLATES dict.")
        sys.exit(1)

    print(f"\nRunning {len(all_tests)} pixel comparisons...\n")

    passed = 0
    failed = 0
    errors = 0
    results_summary = []

    for test_id in tests_with_html:
        sp, name = test_id.split("/", 1)
        test_dir = os.path.join(RESULTS_DIR, sp, name)
        os.makedirs(test_dir, exist_ok=True)

        openui_png = os.path.join(test_dir, "openui.png")
        chromium_png = os.path.join(test_dir, "chromium.png")
        diff_png = os.path.join(test_dir, "diff.png")
        result_json = os.path.join(test_dir, "result.json")
        html_file = os.path.join(test_dir, "test.html")

        # Write HTML
        html_content = f"<!DOCTYPE html><html><head><style>{BODY_STYLE}</style></head><body>{HTML_TEMPLATES[test_id]}</body></html>"
        with open(html_file, "w") as f:
            f.write(html_content)

        # Render our engine
        if not render_openui(test_id, openui_png):
            print(f"  ERROR  {test_id} — openui render failed")
            errors += 1
            results_summary.append((test_id, "error", 0.0))
            continue

        # Render Chrome
        if not render_chrome(html_file, chromium_png, chrome_bin, chrome_dir):
            print(f"  ERROR  {test_id} — chrome render failed")
            errors += 1
            results_summary.append((test_id, "error", 0.0))
            continue

        # Compare
        diff_result = pixel_diff(chromium_png, openui_png, diff_png, result_json)
        if diff_result is None:
            print(f"  ERROR  {test_id} — diff failed")
            errors += 1
            results_summary.append((test_id, "error", 0.0))
            continue

        status = diff_result.get("status", "fail")
        mismatch = diff_result.get("mismatch_pct", 100.0)

        if status == "pass":
            print(f"  ✅ PASS  {test_id}")
            passed += 1
        else:
            print(f"  ❌ FAIL  {test_id} — {diff_result.get('mismatched_pixels', '?')}px ({mismatch:.2f}%)")
            failed += 1

        results_summary.append((test_id, status, mismatch))

    total = len(all_tests)
    print(f"\n{'='*60}")
    print(f"PIXEL COMPARISON RESULTS")
    print(f"{'='*60}")
    print(f"  Total:   {total}")
    print(f"  Passed:  {passed}")
    print(f"  Failed:  {failed}")
    print(f"  Errors:  {errors}")
    print(f"  Rate:    {passed}/{total} ({100*passed/total if total > 0 else 0:.1f}%)")

    if failed > 0:
        print(f"\nFailing tests:")
        for tid, status, mismatch in results_summary:
            if status == "fail":
                print(f"  {tid} — {mismatch:.2f}% mismatch")

    if errors > 0:
        print(f"\nError tests:")
        for tid, status, mismatch in results_summary:
            if status == "error":
                print(f"  {tid}")

    # Write summary JSON
    summary_file = os.path.join(RESULTS_DIR, "summary.json")
    with open(summary_file, "w") as f:
        json.dump({
            "total": total,
            "passed": passed,
            "failed": failed,
            "errors": errors,
            "tests": [{"id": tid, "status": st, "mismatch_pct": mp} for tid, st, mp in results_summary]
        }, f, indent=2)
    print(f"\nSummary written to {summary_file}")

    # Validate summary.json
    with open(summary_file) as f:
        written = json.load(f)
    written_ids = {t["id"] for t in written.get("tests", [])}
    expected_ids = set(all_tests)
    if len(written.get("tests", [])) != len(all_tests):
        print(f"\nERROR: summary.json has {len(written.get('tests', []))} entries "
              f"but expected {len(all_tests)}", file=sys.stderr)
        sys.exit(1)
    missing = expected_ids - written_ids
    if missing:
        print(f"\nERROR: summary.json is missing {len(missing)} test IDs:", file=sys.stderr)
        for m in sorted(missing):
            print(f"  - {m}", file=sys.stderr)
        sys.exit(1)
    print("summary.json validated OK")


if __name__ == "__main__":
    main()
