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

BODY_STYLE = "body { margin: 0; padding: 20px; font-family: DejaVu Sans, sans-serif; font-size: 16px; }"

# HTML templates for each test pattern
HTML_TEMPLATES = {
    # ── SP12 Display ─────────────────────────────────────────────────
    "sp12/display_outer_block": "<div style='display:block;width:200px;height:100px;background:red;'></div>",
    "sp12/display_outer_inline": "<span style='display:inline;background:red;'>Hello World</span>",
    "sp12/display_outer_inline_block": "<div style='display:inline-block;width:200px;height:100px;background:red;'></div>",
    "sp12/display_outer_none": "<div style='display:none;width:200px;height:100px;background:red;'></div><div style='width:100px;height:50px;background:blue;'></div>",
    "sp12/display_inner_flow_root": "<div style='display:flow-root;width:200px;height:100px;background:red;'></div>",

    # ── SP12 Position ────────────────────────────────────────────────
    "sp12/position_static": "<div style='position:static;width:200px;height:100px;background:red;'></div>",
    "sp12/position_relative": "<div style='position:relative;top:30px;left:30px;width:200px;height:100px;background:red;'></div>",
    "sp12/position_absolute": "<div style='position:relative;width:400px;height:300px;background:#eee;'><div style='position:absolute;top:50px;left:50px;width:200px;height:100px;background:red;'></div></div>",
    "sp12/position_fixed": "<div style='position:fixed;top:20px;left:20px;width:200px;height:100px;background:red;'></div>",

    # ── SP12 Float ───────────────────────────────────────────────────
    "sp12/float_left": "<div style='float:left;width:100px;height:100px;background:red;'></div><div style='width:300px;height:100px;background:blue;'></div>",
    "sp12/float_right": "<div style='float:right;width:100px;height:100px;background:red;'></div><div style='width:300px;height:100px;background:blue;'></div>",
    "sp12/float_none": "<div style='float:none;width:200px;height:100px;background:red;'></div>",
    "sp12/clear_left": "<div style='float:left;width:100px;height:100px;background:red;'></div><div style='clear:left;width:200px;height:100px;background:blue;'></div>",
    "sp12/clear_right": "<div style='float:right;width:100px;height:100px;background:red;'></div><div style='clear:right;width:200px;height:100px;background:blue;'></div>",
    "sp12/clear_both": "<div style='float:left;width:100px;height:50px;background:red;'></div><div style='float:right;width:100px;height:50px;background:green;'></div><div style='clear:both;width:200px;height:100px;background:blue;'></div>",

    # ── SP12 Margin ──────────────────────────────────────────────────
    "sp12/margin_positive": "<div style='margin:30px;width:200px;height:100px;background:red;'></div>",
    "sp12/margin_negative": "<div style='width:200px;height:50px;background:blue;'></div><div style='margin-top:-20px;width:200px;height:50px;background:red;'></div>",
    "sp12/margin_auto": "<div style='margin:0 auto;width:200px;height:100px;background:red;'></div>",
    "sp12/margin_collapsing_siblings": "<div style='margin-bottom:30px;width:200px;height:50px;background:red;'></div><div style='margin-top:20px;width:200px;height:50px;background:blue;'></div>",

    # ── SP12 Padding/Border/Box ──────────────────────────────────────
    "sp12/padding_basic": "<div style='padding:20px;width:200px;height:100px;background:red;'></div>",
    "sp12/border_basic": "<div style='border:3px solid black;width:200px;height:100px;background:red;'></div>",
    "sp12/box_sizing_content_box": "<div style='box-sizing:content-box;width:200px;height:100px;padding:20px;border:3px solid black;background:red;'></div>",
    "sp12/box_sizing_border_box": "<div style='box-sizing:border-box;width:200px;height:100px;padding:20px;border:3px solid black;background:red;'></div>",

    # ── SP12 Sizing ──────────────────────────────────────────────────
    "sp12/width_fixed_px": "<div style='width:300px;height:100px;background:red;'></div>",
    "sp12/height_fixed_px": "<div style='width:200px;height:200px;background:red;'></div>",
    "sp12/width_percent": "<div style='width:50%;height:100px;background:red;'></div>",
    "sp12/min_width": "<div style='min-width:300px;width:100px;height:100px;background:red;'></div>",
    "sp12/max_width": "<div style='max-width:100px;width:300px;height:100px;background:red;'></div>",
    "sp12/min_height": "<div style='width:200px;min-height:200px;height:50px;background:red;'></div>",
    "sp12/max_height": "<div style='width:200px;max-height:50px;height:200px;background:red;'></div>",

    # ── SP12 Overflow ────────────────────────────────────────────────
    "sp12/overflow_visible": "<div style='width:100px;height:50px;overflow:visible;background:red;'><div style='width:200px;height:100px;background:blue;'></div></div>",
    "sp12/overflow_hidden": "<div style='width:100px;height:50px;overflow:hidden;background:red;'><div style='width:200px;height:100px;background:blue;'></div></div>",

    # ── SP12 Flex ────────────────────────────────────────────────────
    "sp12/flex_direction_row": "<div style='display:flex;flex-direction:row;width:400px;'><div style='width:100px;height:100px;background:red;'></div><div style='width:100px;height:100px;background:blue;'></div><div style='width:100px;height:100px;background:green;'></div></div>",
    "sp12/flex_direction_column": "<div style='display:flex;flex-direction:column;width:200px;'><div style='height:50px;background:red;'></div><div style='height:50px;background:blue;'></div><div style='height:50px;background:green;'></div></div>",
    "sp12/flex_justify_start": "<div style='display:flex;justify-content:flex-start;width:400px;height:100px;background:#eee;'><div style='width:80px;height:80px;background:red;'></div><div style='width:80px;height:80px;background:blue;'></div></div>",
    "sp12/flex_justify_center": "<div style='display:flex;justify-content:center;width:400px;height:100px;background:#eee;'><div style='width:80px;height:80px;background:red;'></div><div style='width:80px;height:80px;background:blue;'></div></div>",
    "sp12/flex_justify_space_between": "<div style='display:flex;justify-content:space-between;width:400px;height:100px;background:#eee;'><div style='width:80px;height:80px;background:red;'></div><div style='width:80px;height:80px;background:blue;'></div><div style='width:80px;height:80px;background:green;'></div></div>",
    "sp12/flex_align_center": "<div style='display:flex;align-items:center;width:400px;height:200px;background:#eee;'><div style='width:80px;height:50px;background:red;'></div><div style='width:80px;height:80px;background:blue;'></div></div>",
    "sp12/flex_align_stretch": "<div style='display:flex;align-items:stretch;width:400px;height:200px;background:#eee;'><div style='width:80px;background:red;'></div><div style='width:80px;background:blue;'></div></div>",
    "sp12/flex_wrap_basic": "<div style='display:flex;flex-wrap:wrap;width:200px;'><div style='width:100px;height:80px;background:red;'></div><div style='width:100px;height:80px;background:blue;'></div><div style='width:100px;height:80px;background:green;'></div></div>",
    "sp12/flex_grow_equal": "<div style='display:flex;width:400px;height:100px;'><div style='flex-grow:1;background:red;'></div><div style='flex-grow:1;background:blue;'></div><div style='flex-grow:1;background:green;'></div></div>",
    "sp12/flex_gap": "<div style='display:flex;gap:10px;width:400px;'><div style='width:100px;height:100px;background:red;'></div><div style='width:100px;height:100px;background:blue;'></div><div style='width:100px;height:100px;background:green;'></div></div>",

    # ── SP12 Misc ────────────────────────────────────────────────────
    "sp12/z_index_stacking": "<div style='position:relative;width:300px;height:200px;'><div style='position:absolute;top:0;left:0;width:150px;height:150px;background:red;z-index:1;'></div><div style='position:absolute;top:50px;left:50px;width:150px;height:150px;background:blue;z-index:2;'></div></div>",
    "sp12/opacity_basic": "<div style='opacity:0.5;width:200px;height:100px;background:red;'></div>",
    "sp12/border_radius": "<div style='border-radius:20px;width:200px;height:100px;background:red;'></div>",
    "sp12/visibility_hidden": "<div style='width:200px;height:50px;background:blue;'></div><div style='visibility:hidden;width:200px;height:50px;background:red;'></div><div style='width:200px;height:50px;background:green;'></div>",
    "sp12/nested_blocks": "<div style='padding:10px;background:#eee;'><div style='padding:10px;background:#ccc;'><div style='width:200px;height:50px;background:red;'></div></div></div>",

    # ── SP11 Text Decoration ─────────────────────────────────────────
    "sp11/text_decoration_underline": "<p style='text-decoration:underline;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/text_decoration_overline": "<p style='text-decoration:overline;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/text_decoration_line_through": "<p style='text-decoration:line-through;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/text_decoration_combined": "<p style='text-decoration:underline overline line-through;'>The quick brown fox jumps over the lazy dog</p>",

    # ── SP11 Font ────────────────────────────────────────────────────
    "sp11/font_weight_normal": "<p style='font-weight:normal;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/font_weight_bold": "<p style='font-weight:bold;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/font_style_normal": "<p style='font-style:normal;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/font_style_italic": "<p style='font-style:italic;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/font_size_small": "<p style='font-size:12px;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/font_size_medium": "<p style='font-size:16px;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/font_size_large": "<p style='font-size:24px;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/font_size_xlarge": "<p style='font-size:32px;'>The quick brown fox jumps over the lazy dog</p>",

    # ── SP11 Text Align ──────────────────────────────────────────────
    "sp11/text_align_left": "<p style='text-align:left;width:400px;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/text_align_center": "<p style='text-align:center;width:400px;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/text_align_right": "<p style='text-align:right;width:400px;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/text_align_justify": "<p style='text-align:justify;width:300px;'>The quick brown fox jumps over the lazy dog. This sentence needs to be long enough to wrap to at least two lines for justify to take effect.</p>",

    # ── SP11 Text Transform ──────────────────────────────────────────
    "sp11/text_transform_uppercase": "<p style='text-transform:uppercase;'>The quick brown fox</p>",
    "sp11/text_transform_lowercase": "<p style='text-transform:lowercase;'>The Quick Brown Fox</p>",
    "sp11/text_transform_capitalize": "<p style='text-transform:capitalize;'>the quick brown fox</p>",

    # ── SP11 Text Indent ─────────────────────────────────────────────
    "sp11/text_indent_positive": "<p style='text-indent:40px;width:300px;'>The quick brown fox jumps over the lazy dog. This text should be indented on the first line only.</p>",
    "sp11/text_indent_negative": "<p style='text-indent:-20px;padding-left:40px;width:300px;'>The quick brown fox jumps over the lazy dog. This text has a negative indent.</p>",

    # ── SP11 Letter/Word Spacing ─────────────────────────────────────
    "sp11/letter_spacing_positive": "<p style='letter-spacing:3px;'>The quick brown fox</p>",
    "sp11/letter_spacing_negative": "<p style='letter-spacing:-1px;'>The quick brown fox</p>",
    "sp11/word_spacing_positive": "<p style='word-spacing:10px;'>The quick brown fox</p>",

    # ── SP11 Line Height ─────────────────────────────────────────────
    "sp11/line_height_normal": "<p style='line-height:normal;width:200px;'>The quick brown fox jumps over the lazy dog. Multiple lines needed.</p>",
    "sp11/line_height_number": "<p style='line-height:2;width:200px;'>The quick brown fox jumps over the lazy dog. Multiple lines needed.</p>",
    "sp11/line_height_length": "<p style='line-height:30px;width:200px;'>The quick brown fox jumps over the lazy dog. Multiple lines needed.</p>",

    # ── SP11 White Space ─────────────────────────────────────────────
    "sp11/white_space_normal": "<p style='white-space:normal;width:200px;'>The   quick   brown   fox\n  jumps over   the lazy dog</p>",
    "sp11/white_space_nowrap": "<p style='white-space:nowrap;width:200px;background:#eee;'>The quick brown fox jumps over the lazy dog</p>",
    "sp11/white_space_pre": "<pre style='white-space:pre;width:400px;background:#eee;'>The   quick   brown   fox\n  jumps over   the lazy dog</pre>",
    "sp11/white_space_pre_wrap": "<p style='white-space:pre-wrap;width:200px;background:#eee;'>The   quick   brown   fox\n  jumps over   the lazy dog</p>",
    "sp11/white_space_pre_line": "<p style='white-space:pre-line;width:200px;background:#eee;'>The   quick   brown   fox\n  jumps over   the lazy dog</p>",

    # ── SP11 Color ───────────────────────────────────────────────────
    "sp11/color_red": "<p style='color:red;'>The quick brown fox</p>",
    "sp11/color_blue": "<p style='color:blue;'>The quick brown fox</p>",
    "sp11/color_green": "<p style='color:green;'>The quick brown fox</p>",
    "sp11/color_custom": "<p style='color:#8B4513;'>The quick brown fox</p>",

    # ── SP11 Text Shadow / Overflow ──────────────────────────────────
    "sp11/text_shadow_basic": "<p style='text-shadow:2px 2px 4px rgba(0,0,0,0.5);font-size:24px;'>The quick brown fox</p>",
    "sp11/text_overflow_ellipsis": "<div style='width:150px;overflow:hidden;white-space:nowrap;text-overflow:ellipsis;background:#eee;'>The quick brown fox jumps over the lazy dog</div>",

    # ── SP13 Inline Basics ───────────────────────────────────────────
    "sp13/inline_single_span": "<p>Hello <span style='color:red;'>world</span></p>",
    "sp13/inline_multiple_spans": "<p><span style='color:red;'>Hello</span> <span style='color:blue;'>beautiful</span> <span style='color:green;'>world</span></p>",
    "sp13/inline_nested_spans": "<p><span style='color:red;'>Hello <span style='font-weight:bold;color:blue;'>beautiful</span> world</span></p>",

    # ── SP13 Line Breaking ───────────────────────────────────────────
    "sp13/line_breaking_normal_wrap": "<p style='width:150px;background:#eee;'>The quick brown fox jumps over the lazy dog and keeps going</p>",
    "sp13/line_breaking_nowrap": "<p style='white-space:nowrap;width:150px;background:#eee;'>The quick brown fox jumps</p>",
    "sp13/line_breaking_break_word": "<p style='overflow-wrap:break-word;width:100px;background:#eee;'>Supercalifragilisticexpialidocious</p>",

    # ── SP13 Vertical Align ──────────────────────────────────────────
    "sp13/vertical_align_baseline": "<p style='font-size:24px;'>Text <span style='font-size:12px;vertical-align:baseline;background:#eee;'>small baseline</span> text</p>",
    "sp13/vertical_align_middle": "<p style='font-size:24px;'>Text <span style='font-size:12px;vertical-align:middle;background:#eee;'>small middle</span> text</p>",
    "sp13/vertical_align_top": "<p style='font-size:24px;'>Text <span style='font-size:12px;vertical-align:top;background:#eee;'>small top</span> text</p>",
    "sp13/vertical_align_bottom": "<p style='font-size:24px;'>Text <span style='font-size:12px;vertical-align:bottom;background:#eee;'>small bottom</span> text</p>",
    "sp13/vertical_align_super": "<p>Normal text<span style='vertical-align:super;font-size:12px;'>superscript</span> text</p>",
    "sp13/vertical_align_sub": "<p>Normal text<span style='vertical-align:sub;font-size:12px;'>subscript</span> text</p>",

    # ── SP13 Inline Block ────────────────────────────────────────────
    "sp13/inline_block_basic": "<p>Text <span style='display:inline-block;width:50px;height:50px;background:red;'></span> more text</p>",
    "sp13/inline_block_vertical_align": "<p style='font-size:24px;'>Text <span style='display:inline-block;width:50px;height:50px;background:red;vertical-align:middle;'></span> middle</p>",

    # ── SP13 Mixed Content ───────────────────────────────────────────
    "sp13/mixed_block_inline": "<div><p>First paragraph</p><span style='color:red;'>Inline text</span><p>Second paragraph</p></div>",

    # ── SP13 White Space ─────────────────────────────────────────────
    "sp13/white_space_collapsing": "<p>Hello     world    how    are    you</p>",
    "sp13/white_space_preserving": "<pre>Hello     world    how    are    you</pre>",

    # ── SP13 Inline Decoration ───────────────────────────────────────
    "sp13/inline_background_color": "<p>Normal <span style='background:yellow;'>highlighted text</span> normal</p>",
    "sp13/inline_padding": "<p>Normal <span style='padding:5px 10px;background:yellow;'>padded text</span> normal</p>",
    "sp13/inline_border": "<p>Normal <span style='border:1px solid red;padding:2px 4px;'>bordered text</span> normal</p>",
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
        print(f"\nWARNING: {len(tests_without_html)} tests without HTML templates:")
        for t in tests_without_html:
            print(f"  - {t}")

    print(f"\nRunning {len(tests_with_html)} pixel comparisons...\n")

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

    print(f"\n{'='*60}")
    print(f"PIXEL COMPARISON RESULTS")
    print(f"{'='*60}")
    print(f"  Total:   {len(tests_with_html)}")
    print(f"  Passed:  {passed}")
    print(f"  Failed:  {failed}")
    print(f"  Errors:  {errors}")
    print(f"  Rate:    {passed}/{passed+failed} ({100*passed/(passed+failed) if passed+failed > 0 else 0:.1f}%)")

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
            "total": len(tests_with_html),
            "passed": passed,
            "failed": failed,
            "errors": errors,
            "tests": [{"id": tid, "status": st, "mismatch_pct": mp} for tid, st, mp in results_summary]
        }, f, indent=2)
    print(f"\nSummary written to {summary_file}")


if __name__ == "__main__":
    main()
