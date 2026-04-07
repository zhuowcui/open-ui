#!/usr/bin/env python3
"""Generate minimal HTML test files for pixel comparison with Chromium.

Reads feature matrix CSVs (sp11, sp12, sp13) and produces one self-contained
HTML file per feature variant under
    tools/accountability/data/pixel_comparison/html_tests/{sp11,sp12,sp13}/
"""

import csv
import os
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
MATRIX_DIR = REPO_ROOT / "tools" / "accountability" / "data" / "feature_matrix"
OUTPUT_BASE = REPO_ROOT / "tools" / "accountability" / "data" / "pixel_comparison" / "html_tests"

SAMPLE_TEXT = "The quick brown fox jumps over the lazy dog"
LONG_TEXT = (
    "The quick brown fox jumps over the lazy dog. "
    "Pack my box with five dozen liquor jugs. "
    "How vexingly quick daft zebras jump."
)

BASE_STYLE = "body { margin: 0; padding: 20px; font-family: Arial, sans-serif; font-size: 16px; }"

CSV_FILES = {
    "sp11": MATRIX_DIR / "sp11_text_features.csv",
    "sp12": MATRIX_DIR / "sp12_block_features.csv",
    "sp13": MATRIX_DIR / "sp13_inline_features.csv",
}


def sanitize_filename(name: str) -> str:
    """Replace non-alphanumeric characters (except hyphens/underscores) with underscores."""
    name = re.sub(r"[^a-zA-Z0-9_\-]", "_", name)
    name = re.sub(r"_+", "_", name)
    return name.strip("_").lower()


def wrap_html(title: str, body: str, extra_style: str = "") -> str:
    """Wrap body content in a minimal HTML document."""
    return (
        "<!DOCTYPE html>\n"
        f"<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n"
        f"<title>{title}</title>\n"
        f"<style>{BASE_STYLE}{extra_style}</style>\n"
        f"</head>\n<body>\n{body}\n</body>\n</html>\n"
    )


def placeholder(feature: str, sub_feature: str, variant: str) -> str:
    """Generate a visible placeholder for variants that can't be auto-mapped."""
    title = f"{feature} / {sub_feature} / {variant}"
    body = (
        f"<!-- PLACEHOLDER: auto-generation could not produce a precise test for\n"
        f"     {feature} / {sub_feature} / {variant}\n"
        f"     Replace this with a hand-crafted test. -->\n"
        f'<div style="padding:20px; border:2px dashed #cc0000; background:#fff3f3; '
        f'color:#333; max-width:600px;">\n'
        f'  <h2 style="margin:0 0 10px; color:#cc0000;">Placeholder Test</h2>\n'
        f"  <p><strong>Feature:</strong> {feature}</p>\n"
        f"  <p><strong>Sub-feature:</strong> {sub_feature}</p>\n"
        f"  <p><strong>Variant:</strong> {variant}</p>\n"
        f'  <p style="margin-top:12px;">{SAMPLE_TEXT}</p>\n'
        f"</div>"
    )
    return wrap_html(title, body)


# ---------------------------------------------------------------------------
# Helpers used by multiple generators
# ---------------------------------------------------------------------------

def _text_prop_test(css_prop: str, css_value: str, label: str = "") -> str:
    """A paragraph demonstrating a single text CSS property."""
    label = label or f"{css_prop}: {css_value}"
    title = label
    body = (
        f'<p style="{css_prop}:{css_value}; background:#f0f0f0; padding:8px;">'
        f"{SAMPLE_TEXT}</p>\n"
        f'<p style="margin-top:12px;">Normal reference text for comparison.</p>'
    )
    return wrap_html(title, body)


def _box_test(extra_style: str, label: str, inner: str = "") -> str:
    """A coloured box demonstrating a block-level CSS property."""
    title = label
    inner_html = inner or ""
    body = (
        f'<div style="width:200px; height:100px; background:#e74c3c; '
        f'border:1px solid #222; {extra_style}">{inner_html}</div>\n'
        f'<div style="margin-top:10px; width:200px; height:50px; '
        f'background:#3498db;">Reference block</div>'
    )
    return wrap_html(title, body)


# ===================================================================
# SP11 – TEXT FEATURES
# ===================================================================

def generate_sp11(feature: str, sub: str, variant: str) -> str:
    key = (feature, sub, variant)

    # --- text-decoration-line ---
    if feature == "text-decoration-line" and sub == "line":
        val_map = {
            "none": "none",
            "underline": "underline",
            "overline": "overline",
            "line-through": "line-through",
            "underline+overline": "underline overline",
            "underline+line-through": "underline line-through",
            "overline+line-through": "overline line-through",
            "underline+overline+line-through": "underline overline line-through",
        }
        val = val_map.get(variant)
        if val is not None:
            return _text_prop_test("text-decoration-line", val)

    # --- text-decoration-style ---
    if feature == "text-decoration-style" and sub == "style":
        return wrap_html(
            f"text-decoration-style: {variant}",
            f'<p style="text-decoration:underline; text-decoration-style:{variant}; '
            f'text-decoration-color:#e74c3c; font-size:24px;">{SAMPLE_TEXT}</p>\n'
            f'<p style="margin-top:12px;">Normal reference text.</p>',
        )

    # --- text-decoration-color ---
    if feature == "text-decoration-color" and sub == "color":
        color_map = {"currentColor": "currentColor", "explicit-color": "#e74c3c", "transparent": "transparent"}
        c = color_map.get(variant)
        if c:
            return wrap_html(
                f"text-decoration-color: {variant}",
                f'<p style="text-decoration:underline; text-decoration-color:{c}; '
                f'font-size:24px; color:#2c3e50;">{SAMPLE_TEXT}</p>',
            )

    # --- text-decoration-thickness ---
    if feature == "text-decoration-thickness" and sub == "thickness":
        thick_map = {"auto": "auto", "from-font": "from-font", "length": "3px"}
        v = thick_map.get(variant)
        if v:
            return wrap_html(
                f"text-decoration-thickness: {variant}",
                f'<p style="text-decoration:underline; text-decoration-thickness:{v}; '
                f'font-size:24px;">{SAMPLE_TEXT}</p>',
            )

    # --- text-decoration-skip-ink ---
    if feature == "text-decoration-skip-ink" and sub == "skip-ink":
        skip_map = {
            "auto": "auto", "all": "all", "none": "none",
            "auto-cjk-exclusion": "auto", "intercept-merging": "auto",
            "line-through-no-skip": "none",
        }
        v = skip_map.get(variant)
        if v:
            return wrap_html(
                f"text-decoration-skip-ink: {variant}",
                f'<p style="text-decoration:underline; text-decoration-skip-ink:{v}; '
                f'font-size:28px;">Typography glyph pqjy</p>',
            )

    # --- text-underline-offset ---
    if feature == "text-underline-offset" and sub == "offset":
        off_map = {"auto": "auto", "length-px": "5px", "percentage": "20%"}
        v = off_map.get(variant)
        if v:
            return _text_prop_test("text-underline-offset", v, f"text-underline-offset: {variant}")

    # --- text-underline-position ---
    if feature == "text-underline-position" and sub == "position":
        return _text_prop_test("text-underline-position", variant, f"text-underline-position: {variant}")

    # --- text-emphasis-mark ---
    if feature == "text-emphasis-mark" and sub == "mark":
        mark_map = {
            "dot": "dot", "circle": "circle", "double-circle": "double-circle",
            "triangle": "triangle", "sesame": "sesame", "none": "none",
            "custom-char": "'★'",
        }
        v = mark_map.get(variant)
        if v:
            return wrap_html(
                f"text-emphasis: {variant}",
                f'<p style="text-emphasis:filled {v}; -webkit-text-emphasis:filled {v}; '
                f'font-size:24px;">{SAMPLE_TEXT}</p>',
            )

    # --- text-emphasis-fill ---
    if feature == "text-emphasis-fill" and sub == "fill":
        return wrap_html(
            f"text-emphasis-fill: {variant}",
            f'<p style="text-emphasis:{variant} dot; -webkit-text-emphasis:{variant} dot; '
            f'font-size:24px;">{SAMPLE_TEXT}</p>',
        )

    # --- text-emphasis-color ---
    if feature == "text-emphasis-color" and sub == "color":
        c = "#e74c3c" if variant == "explicit-color" else "currentColor"
        return wrap_html(
            f"text-emphasis-color: {variant}",
            f'<p style="text-emphasis:filled dot; -webkit-text-emphasis:filled dot; '
            f'text-emphasis-color:{c}; -webkit-text-emphasis-color:{c}; '
            f'font-size:24px;">{SAMPLE_TEXT}</p>',
        )

    # --- text-emphasis-position ---
    if feature == "text-emphasis-position" and sub == "position":
        pos = variant.replace("-", " ")
        return wrap_html(
            f"text-emphasis-position: {variant}",
            f'<p style="text-emphasis:filled dot; -webkit-text-emphasis:filled dot; '
            f'text-emphasis-position:{pos}; font-size:24px; line-height:2.5;">'
            f'{SAMPLE_TEXT}</p>',
        )

    # --- text-emphasis-paint ---
    if feature == "text-emphasis-paint" and sub == "paint":
        return wrap_html(
            f"text-emphasis paint: {variant}",
            f'<p style="text-emphasis:filled dot; -webkit-text-emphasis:filled dot; '
            f'font-size:24px; line-height:2.5;">{SAMPLE_TEXT}</p>',
        )

    # --- text-shadow ---
    if feature == "text-shadow" and sub == "shadow":
        shadow_map = {
            "none": "none",
            "single-offset": "2px 2px #e74c3c",
            "single-blur": "2px 2px 4px #e74c3c",
            "shadow-color": "2px 2px 0 #3498db",
            "zero-blur": "2px 2px 0 #e74c3c",
            "multiple-shadows": "2px 2px 0 #e74c3c, -2px -2px 0 #3498db",
        }
        v = shadow_map.get(variant)
        if v:
            return wrap_html(
                f"text-shadow: {variant}",
                f'<p style="text-shadow:{v}; font-size:28px;">{SAMPLE_TEXT}</p>',
            )

    # --- font-weight ---
    if feature == "font-weight" and sub == "weight":
        return _text_prop_test("font-weight", variant, f"font-weight: {variant}")

    # --- font-style ---
    if feature == "font-style" and sub == "style":
        val = "oblique 14deg" if variant == "oblique-angle" else variant
        return _text_prop_test("font-style", val, f"font-style: {variant}")

    # --- font-size ---
    if feature == "font-size" and sub == "size":
        size_map = {"default-16px": "16px", "explicit-px": "24px", "large": "large", "zero": "0"}
        v = size_map.get(variant)
        if v:
            return _text_prop_test("font-size", v, f"font-size: {variant}")

    # --- font-size-adjust ---
    if feature == "font-size-adjust" and sub == "size-adjust":
        val = "none" if variant == "none" else "0.5"
        return _text_prop_test("font-size-adjust", val, f"font-size-adjust: {variant}")

    # --- font-family ---
    if feature == "font-family":
        if sub == "family":
            family_map = {
                "generic-serif": "serif",
                "generic-sans-serif": "sans-serif",
                "generic-monospace": "monospace",
                "generic-cursive": "cursive",
                "generic-fantasy": "fantasy",
                "generic-system-ui": "system-ui",
                "generic-emoji": "emoji",
                "generic-math": "math",
                "generic-fangsong": "fangsong",
                "generic-ui-serif": "ui-serif",
                "generic-ui-sans-serif": "ui-sans-serif",
                "generic-ui-monospace": "ui-monospace",
                "generic-ui-rounded": "ui-rounded",
                "named-single": "Arial",
                "named-multiple": "Georgia, 'Times New Roman', serif",
            }
            v = family_map.get(variant)
            if v:
                return _text_prop_test("font-family", v, f"font-family: {variant}")
        if sub == "fallback":
            return wrap_html(
                "font-family: fallback chain",
                f'<p style="font-family: \'NonExistentFont42\', Arial, sans-serif;">'
                f'{SAMPLE_TEXT}</p>',
            )

    # --- font-stretch ---
    if feature == "font-stretch" and sub == "stretch":
        return _text_prop_test("font-stretch", variant, f"font-stretch: {variant}")

    # --- font-variant-caps ---
    if feature == "font-variant-caps" and sub == "caps":
        return _text_prop_test("font-variant-caps", variant, f"font-variant-caps: {variant}")

    # --- font-variant-ligatures ---
    if feature == "font-variant-ligatures" and sub == "ligatures":
        return _text_prop_test("font-variant-ligatures", variant, f"font-variant-ligatures: {variant}")

    # --- font-variant-numeric ---
    if feature == "font-variant-numeric" and sub == "numeric":
        text = "0123456789 1/2 3/4 1st 2nd 3rd"
        return wrap_html(
            f"font-variant-numeric: {variant}",
            f'<p style="font-variant-numeric:{variant}; font-size:24px; background:#f0f0f0; '
            f'padding:8px;">{text}</p>\n'
            f'<p style="margin-top:12px;">Reference: {text}</p>',
        )

    # --- font-variant-east-asian ---
    if feature == "font-variant-east-asian" and sub == "east-asian":
        return _text_prop_test("font-variant-east-asian", variant, f"font-variant-east-asian: {variant}")

    # --- font-variant-alternates ---
    if feature == "font-variant-alternates" and sub == "alternates":
        return _text_prop_test("font-variant-alternates", variant, f"font-variant-alternates: {variant}")

    # --- font-variant-position ---
    if feature == "font-variant-position" and sub == "position":
        return _text_prop_test("font-variant-position", variant, f"font-variant-position: {variant}")

    # --- font-feature-settings ---
    if feature == "font-feature-settings" and sub == "feature-settings":
        feat_map = {"normal": "normal", "explicit-tag": "'smcp'", "multiple-tags": "'smcp', 'liga'"}
        v = feat_map.get(variant)
        if v:
            return _text_prop_test("font-feature-settings", v, f"font-feature-settings: {variant}")

    # --- font-variation-settings ---
    if feature == "font-variation-settings" and sub == "variation-settings":
        var_map = {
            "normal": "normal",
            "wght-axis": "'wght' 700",
            "wdth-axis": "'wdth' 75",
            "explicit-axis": "'wght' 600",
        }
        v = var_map.get(variant)
        if v:
            return _text_prop_test("font-variation-settings", v, f"font-variation-settings: {variant}")

    # --- font-optical-sizing ---
    if feature == "font-optical-sizing" and sub == "optical-sizing":
        return _text_prop_test("font-optical-sizing", variant, f"font-optical-sizing: {variant}")

    # --- font-synthesis-weight / font-synthesis-style ---
    if feature in ("font-synthesis-weight", "font-synthesis-style") and sub == "synthesis":
        return _text_prop_test(feature, variant, f"{feature}: {variant}")

    # --- font-smoothing ---
    if feature == "font-smoothing" and sub == "smoothing":
        smooth_map = {
            "auto": "auto",
            "none": "none",
            "antialiased": "antialiased",
            "subpixel-antialiased": "subpixel-antialiased",
        }
        v = smooth_map.get(variant)
        if v:
            return wrap_html(
                f"font-smoothing: {variant}",
                f'<p style="-webkit-font-smoothing:{v}; font-size:24px;">{SAMPLE_TEXT}</p>',
            )

    # --- font-palette ---
    if feature == "font-palette" and sub == "palette":
        val = variant if variant != "custom" else "--custom"
        return _text_prop_test("font-palette", val, f"font-palette: {variant}")

    # --- text-align ---
    if feature == "text-align" and sub == "align":
        return wrap_html(
            f"text-align: {variant}",
            f'<div style="width:400px; background:#f0f0f0; padding:8px; text-align:{variant};">'
            f'<p>{SAMPLE_TEXT}</p></div>',
        )

    # --- text-align-last ---
    if feature == "text-align-last" and sub == "align-last":
        return wrap_html(
            f"text-align-last: {variant}",
            f'<div style="width:300px; background:#f0f0f0; padding:8px; text-align:justify; '
            f'text-align-last:{variant};">'
            f'<p>{LONG_TEXT}</p></div>',
        )

    # --- text-transform ---
    if feature == "text-transform":
        if sub == "transform":
            return _text_prop_test("text-transform", variant, f"text-transform: {variant}")
        if sub == "locale":
            locale_map = {
                "turkish-uppercase": ("tr", "text-transform:uppercase"),
                "turkish-lowercase": ("tr", "text-transform:lowercase"),
                "greek-uppercase": ("el", "text-transform:uppercase"),
                "dutch-capitalize": ("nl", "text-transform:capitalize"),
                "lithuanian-lowercase": ("lt", "text-transform:lowercase"),
            }
            info = locale_map.get(variant)
            if info:
                lang, css = info
                return wrap_html(
                    f"text-transform locale: {variant}",
                    f'<p lang="{lang}" style="{css}; background:#f0f0f0; padding:8px;">'
                    f'{SAMPLE_TEXT}</p>',
                )
        if sub == "edge-case":
            text = "straße" if variant == "german-eszett" else "ΟΔΌΣ Σίγμα"
            return wrap_html(
                f"text-transform edge-case: {variant}",
                f'<p style="text-transform:uppercase; background:#f0f0f0; padding:8px;">{text}</p>',
            )

    # --- text-indent ---
    if feature == "text-indent" and sub == "indent":
        indent_map = {
            "positive-px": "40px",
            "negative-px": "-20px",
            "zero": "0",
            "percentage": "10%",
            "first-line-only": "40px",
        }
        v = indent_map.get(variant)
        if v:
            return wrap_html(
                f"text-indent: {variant}",
                f'<div style="width:400px; background:#f0f0f0; padding:8px; text-indent:{v};">'
                f'<p>{LONG_TEXT}</p></div>',
            )

    # --- text-justify ---
    if feature == "text-justify" and sub == "justify":
        return wrap_html(
            f"text-justify: {variant}",
            f'<div style="width:300px; background:#f0f0f0; padding:8px; text-align:justify; '
            f'text-justify:{variant};">'
            f'<p>{LONG_TEXT}</p></div>',
        )

    # --- text-overflow ---
    if feature == "text-overflow":
        if sub == "overflow":
            return wrap_html(
                f"text-overflow: {variant}",
                f'<div style="width:200px; white-space:nowrap; overflow:hidden; '
                f'text-overflow:{variant}; background:#f0f0f0; padding:8px;">'
                f'{SAMPLE_TEXT}</div>',
            )
        if sub == "behavior":
            return wrap_html(
                f"text-overflow behavior: {variant}",
                f'<div style="width:200px; white-space:nowrap; overflow:hidden; '
                f'text-overflow:ellipsis; background:#f0f0f0; padding:8px;">'
                f'{SAMPLE_TEXT}</div>',
            )

    # --- text-rendering ---
    if feature == "text-rendering" and sub == "rendering":
        return _text_prop_test("text-rendering", variant, f"text-rendering: {variant}")

    # --- text-wrap ---
    if feature == "text-wrap" and sub == "wrap":
        return wrap_html(
            f"text-wrap: {variant}",
            f'<div style="width:300px; background:#f0f0f0; padding:8px; text-wrap:{variant};">'
            f'<p>{LONG_TEXT}</p></div>',
        )

    # --- text-combine-upright ---
    if feature == "text-combine-upright":
        if sub == "combine":
            return wrap_html(
                f"text-combine-upright: {variant}",
                f'<div style="writing-mode:vertical-rl; height:300px; background:#f0f0f0; '
                f'padding:8px;">'
                f'<p>令和<span style="text-combine-upright:{variant};">12</span>年</p></div>',
            )
        # paint sub-features
        return wrap_html(
            f"text-combine-upright paint: {variant}",
            f'<div style="writing-mode:vertical-rl; height:300px; background:#f0f0f0; padding:8px;">'
            f'<p>令和<span style="text-combine-upright:all;">12</span>年</p></div>',
        )

    # --- text-orientation ---
    if feature == "text-orientation" and sub == "orientation":
        return wrap_html(
            f"text-orientation: {variant}",
            f'<div style="writing-mode:vertical-rl; text-orientation:{variant}; '
            f'height:300px; background:#f0f0f0; padding:8px;">'
            f'<p>{SAMPLE_TEXT}</p></div>',
        )

    # --- writing-mode ---
    if feature == "writing-mode":
        if sub == "writing-mode":
            return wrap_html(
                f"writing-mode: {variant}",
                f'<div style="writing-mode:{variant}; height:300px; background:#f0f0f0; '
                f'padding:8px;">'
                f'<p>{SAMPLE_TEXT}</p></div>',
            )
        # helpers sub-features
        return wrap_html(
            f"writing-mode helper: {variant}",
            f'<div style="writing-mode:vertical-rl; height:300px; background:#f0f0f0; '
            f'padding:8px;">'
            f'<p>{SAMPLE_TEXT}</p></div>',
        )

    # --- direction ---
    if feature == "direction" and sub == "direction":
        return wrap_html(
            f"direction: {variant}",
            f'<div style="direction:{variant}; width:400px; background:#f0f0f0; padding:8px;">'
            f'<p>{SAMPLE_TEXT}</p></div>',
        )

    # --- unicode-bidi ---
    if feature == "unicode-bidi":
        if sub == "bidi":
            return wrap_html(
                f"unicode-bidi: {variant}",
                f'<div style="direction:rtl; width:400px; background:#f0f0f0; padding:8px;">'
                f'<span style="unicode-bidi:{variant}; direction:ltr;">{SAMPLE_TEXT}</span></div>',
            )
        # algorithm sub-features
        return wrap_html(
            f"unicode-bidi algorithm: {variant}",
            f'<div style="direction:rtl; width:400px; background:#f0f0f0; padding:8px;">'
            f'<span style="unicode-bidi:isolate; direction:ltr;">{SAMPLE_TEXT}</span></div>',
        )

    # --- line-height ---
    if feature == "line-height" and sub == "line-height":
        lh_map = {"normal": "normal", "number": "1.8", "length-px": "30px", "percentage": "180%", "zero": "0"}
        v = lh_map.get(variant)
        if v:
            return wrap_html(
                f"line-height: {variant}",
                f'<div style="width:400px; background:#f0f0f0; padding:8px; line-height:{v};">'
                f'<p>{LONG_TEXT}</p></div>',
            )

    # --- letter-spacing ---
    if feature == "letter-spacing" and sub == "spacing":
        ls_map = {"normal-0": "normal", "positive-px": "3px", "negative-px": "-1px"}
        v = ls_map.get(variant)
        if v:
            return _text_prop_test("letter-spacing", v, f"letter-spacing: {variant}")

    # --- word-spacing ---
    if feature == "word-spacing" and sub == "spacing":
        ws_map = {"normal-0": "normal", "positive-px": "10px", "rtl-handling": "8px"}
        v = ws_map.get(variant)
        if v:
            extra_style = " direction:rtl;" if variant == "rtl-handling" else ""
            return wrap_html(
                f"word-spacing: {variant}",
                f'<p style="word-spacing:{v};{extra_style} background:#f0f0f0; padding:8px;">'
                f'{SAMPLE_TEXT}</p>',
            )

    # --- white-space ---
    if feature == "white-space" and sub == "white-space":
        text = "  The   quick   brown\n  fox   jumps   over\n  the   lazy   dog  "
        return wrap_html(
            f"white-space: {variant}",
            f'<div style="width:300px; background:#f0f0f0; padding:8px; '
            f'white-space:{variant}; border:1px solid #ccc;">{text}</div>',
        )

    # --- word-break ---
    if feature == "word-break" and sub == "word-break":
        text = "Supercalifragilisticexpialidocious and Pneumonoultramicroscopicsilicovolcanoconiosis"
        return wrap_html(
            f"word-break: {variant}",
            f'<div style="width:200px; background:#f0f0f0; padding:8px; '
            f'word-break:{variant}; border:1px solid #ccc;">{text}</div>',
        )

    # --- overflow-wrap ---
    if feature == "overflow-wrap" and sub == "overflow-wrap":
        text = "Averylongwordthatwillnotfitinthecontainer and normal text"
        return wrap_html(
            f"overflow-wrap: {variant}",
            f'<div style="width:200px; background:#f0f0f0; padding:8px; '
            f'overflow-wrap:{variant}; border:1px solid #ccc;">{text}</div>',
        )

    # --- line-break ---
    if feature == "line-break" and sub == "line-break":
        return wrap_html(
            f"line-break: {variant}",
            f'<div style="width:200px; background:#f0f0f0; padding:8px; '
            f'line-break:{variant}; border:1px solid #ccc;">'
            f'日本語テスト。テキスト、折り返し！テスト。</div>',
        )

    # --- hyphens ---
    if feature == "hyphens":
        if sub == "hyphens":
            return wrap_html(
                f"hyphens: {variant}",
                f'<div lang="en" style="width:150px; background:#f0f0f0; padding:8px; '
                f'hyphens:{variant}; border:1px solid #ccc;">'
                f'Incomprehensibilities and antidisestablishmentarianism</div>',
            )
        # hyphenation sub-features (algorithmic details)
        return wrap_html(
            f"hyphens: {variant}",
            f'<div lang="en" style="width:150px; background:#f0f0f0; padding:8px; '
            f'hyphens:auto; border:1px solid #ccc;">'
            f'Incomprehensibilities and antidisestablishmentarianism</div>',
        )

    # --- tab-size ---
    if feature == "tab-size" and sub == "tab-size":
        tab_map = {"spaces-default": "8", "spaces-custom": "4", "length": "40px"}
        v = tab_map.get(variant)
        if v:
            return wrap_html(
                f"tab-size: {variant}",
                f'<pre style="tab-size:{v}; background:#f0f0f0; padding:8px; '
                f'border:1px solid #ccc;">No tab\n\tOne tab\n\t\tTwo tabs</pre>',
            )

    # --- hanging-punctuation ---
    if feature == "hanging-punctuation" and sub == "hanging":
        return wrap_html(
            f"hanging-punctuation: {variant}",
            f'<div style="width:300px; background:#f0f0f0; padding:8px; '
            f'hanging-punctuation:{variant}; border:1px solid #ccc;">'
            f'"The quick brown fox" said the narrator.</div>',
        )

    # --- vertical-align ---
    if feature == "vertical-align" and sub == "align":
        va_map = {
            "baseline": "baseline", "sub": "sub", "super": "super",
            "text-top": "text-top", "text-bottom": "text-bottom",
            "middle": "middle", "top": "top", "bottom": "bottom",
            "length": "10px", "percentage": "50%",
        }
        v = va_map.get(variant)
        if v:
            return wrap_html(
                f"vertical-align: {variant}",
                f'<p style="font-size:24px; line-height:3; background:#f0f0f0; padding:8px;">'
                f'Normal <span style="vertical-align:{v}; background:#e74c3c; '
                f'color:white; padding:2px 4px;">aligned</span> text</p>',
            )

    # --- initial-letter ---
    if feature == "initial-letter":
        if sub == "initial-letter":
            il_map = {
                "none": "normal",
                "drop-cap": "3",
                "raised-cap": "3 1",
                "sunken-cap": "3 5",
            }
            v = il_map.get(variant)
            if v:
                return wrap_html(
                    f"initial-letter: {variant}",
                    f'<div style="width:400px; background:#f0f0f0; padding:8px;">'
                    f'<p><span style="initial-letter:{v}; color:#e74c3c; font-size:48px;">T</span>'
                    f'he quick brown fox jumps over the lazy dog. {LONG_TEXT}</p></div>',
                )
        if sub == "layout":
            return wrap_html(
                f"initial-letter layout: {variant}",
                f'<div style="width:400px; background:#f0f0f0; padding:8px;">'
                f'<p><span style="initial-letter:3; color:#e74c3c; font-size:48px;">T</span>'
                f'he quick brown fox jumps over the lazy dog. {LONG_TEXT}</p></div>',
            )

    # --- box-decoration-break ---
    if feature == "box-decoration-break" and sub == "decoration-break":
        return wrap_html(
            f"box-decoration-break: {variant}",
            f'<div style="width:200px; line-height:2;">'
            f'<span style="box-decoration-break:{variant}; -webkit-box-decoration-break:{variant}; '
            f'background:#e74c3c; color:white; padding:4px 8px;">'
            f'{LONG_TEXT}</span></div>',
        )

    # --- font-metrics ---
    if feature == "font-metrics" and sub == "metrics":
        return wrap_html(
            f"font-metrics: {variant}",
            f'<p style="font-size:48px; text-decoration:underline; background:#f0f0f0; '
            f'padding:8px;">Tpgyjq</p>\n'
            f'<p style="margin-top:8px;">Metric tested: {variant}</p>',
        )

    # --- font-cache ---
    if feature == "font-cache" and sub == "cache":
        return wrap_html(
            f"font-cache: {variant}",
            f'<p style="font-family:serif; font-size:24px;">{SAMPLE_TEXT}</p>\n'
            f'<p style="font-family:sans-serif; font-size:24px;">{SAMPLE_TEXT}</p>',
        )

    # --- font-orientation ---
    if feature == "font-orientation":
        if sub == "orientation":
            mode = "vertical-rl" if "vertical" in variant else "horizontal-tb"
            return wrap_html(
                f"font-orientation: {variant}",
                f'<div style="writing-mode:{mode}; height:250px; background:#f0f0f0; '
                f'padding:8px;">{SAMPLE_TEXT}</div>',
            )
        if sub == "resolution":
            return wrap_html(
                f"font-orientation resolution: {variant}",
                f'<div style="writing-mode:vertical-rl; height:250px; background:#f0f0f0; '
                f'padding:8px;">{SAMPLE_TEXT}</div>',
            )

    # --- char-orientation ---
    if feature == "char-orientation":
        return wrap_html(
            f"char-orientation: {variant}",
            f'<div style="writing-mode:vertical-rl; text-orientation:upright; height:300px; '
            f'background:#f0f0f0; padding:8px;">ABCabc漢字</div>',
        )

    # --- font-platform ---
    if feature == "font-platform":
        return wrap_html(
            f"font-platform: {variant}",
            f'<p style="font-size:24px;">{SAMPLE_TEXT}</p>',
        )

    # --- color-fonts / emoji ---
    if feature in ("color-fonts", "emoji"):
        return wrap_html(
            f"{feature}: {variant}",
            f'<p style="font-size:48px;">😀🎉🔥💯🌈✨🎨</p>\n'
            f'<p style="font-size:24px;">{SAMPLE_TEXT}</p>',
        )

    # --- text-shaping ---
    if feature == "text-shaping" and sub == "shaping":
        return wrap_html(
            f"text-shaping: {variant}",
            f'<p style="font-size:24px;">ffi ffl fi fl — "quotes" \'apostrophes\'</p>\n'
            f'<p style="font-size:24px; font-family:serif;">ffi ffl fi fl</p>',
        )

    return ""  # signal: not handled


# ===================================================================
# SP12 – BLOCK FEATURES
# ===================================================================

def generate_sp12(feature: str, sub: str, variant: str) -> str:
    key = (feature, sub, variant)

    # --- display ---
    if feature == "display":
        if sub == "outer":
            disp_map = {
                "none": ("display:none;", "This should be hidden: <div style='display:none; background:red; width:100px; height:100px;'></div><p>Only this text should show.</p>"),
                "block": ("", "<div style='display:block; width:200px; height:100px; background:#e74c3c;'></div>"),
                "inline": ("", "<span style='display:inline; background:#e74c3c; padding:4px;'>Inline element</span> <span style='background:#3498db; padding:4px;'>Another</span>"),
                "inline-block": ("", "<span style='display:inline-block; width:100px; height:60px; background:#e74c3c;'></span> <span style='display:inline-block; width:80px; height:80px; background:#3498db;'></span>"),
                "flow-root": ("", "<div style='display:flow-root; background:#f0f0f0; padding:10px;'><div style='float:left; width:80px; height:80px; background:#e74c3c;'></div><p>Text next to float inside flow-root.</p></div>"),
                "list-item": ("", "<div style='display:list-item; list-style:disc inside; background:#f0f0f0; padding:8px;'>List item one</div><div style='display:list-item; list-style:disc inside; background:#f0f0f0; padding:8px;'>List item two</div>"),
                "table": ("", "<div style='display:table; border-collapse:collapse;'><div style='display:table-row;'><div style='display:table-cell; border:1px solid #222; padding:8px; background:#e74c3c; color:white;'>Cell A</div><div style='display:table-cell; border:1px solid #222; padding:8px; background:#3498db; color:white;'>Cell B</div></div></div>"),
            }
            entry = disp_map.get(variant)
            if entry:
                return wrap_html(f"display: {variant}", entry[1])
        if sub == "inner":
            inner_map = {
                "flex": "<div style='display:flex; gap:10px;'><div style='width:80px; height:80px; background:#e74c3c;'></div><div style='width:80px; height:80px; background:#3498db;'></div><div style='width:80px; height:80px; background:#2ecc71;'></div></div>",
                "grid": "<div style='display:grid; grid-template-columns:1fr 1fr; gap:10px;'><div style='height:80px; background:#e74c3c;'></div><div style='height:80px; background:#3498db;'></div><div style='height:80px; background:#2ecc71;'></div><div style='height:80px; background:#f39c12;'></div></div>",
                "inline-flex": "<span style='display:inline-flex; gap:5px; background:#f0f0f0; padding:5px;'><span style='width:40px; height:40px; background:#e74c3c;'></span><span style='width:40px; height:40px; background:#3498db;'></span></span> surrounding text",
                "inline-grid": "<span style='display:inline-grid; grid-template-columns:1fr 1fr; gap:5px; background:#f0f0f0; padding:5px;'><span style='width:40px; height:40px; background:#e74c3c;'></span><span style='width:40px; height:40px; background:#3498db;'></span></span> surrounding text",
            }
            body = inner_map.get(variant)
            if body:
                return wrap_html(f"display inner: {variant}", body)

    # --- position ---
    if feature == "position":
        if sub == "value":
            pos_bodies = {
                "static": "<div style='position:static; width:150px; height:100px; background:#e74c3c;'></div>",
                "relative": "<div style='position:relative; top:20px; left:30px; width:150px; height:100px; background:#3498db;'></div><div style='width:150px; height:50px; background:#95a5a6;'>Reference</div>",
                "absolute": "<div style='position:relative; width:300px; height:200px; background:#f0f0f0; border:1px solid #ccc;'><div style='position:absolute; top:20px; left:30px; width:100px; height:80px; background:#e74c3c;'></div></div>",
                "fixed": "<div style='position:relative; width:300px; height:200px; background:#f0f0f0; border:1px solid #ccc;'><div style='position:absolute; top:10px; right:10px; width:80px; height:80px; background:#9b59b6;'></div></div>",
                "sticky": "<div style='height:200px; overflow:auto; background:#f0f0f0;'><div style='height:50px; background:#ddd;'>Before sticky</div><div style='position:sticky; top:0; background:#e74c3c; color:white; padding:10px;'>Sticky header</div><div style='height:400px; padding:10px;'>Scroll content…</div></div>",
            }
            body = pos_bodies.get(variant)
            if body:
                return wrap_html(f"position: {variant}", body)
        # abspos / sticky sub-features
        if sub in ("abspos", "sticky"):
            return wrap_html(
                f"position {sub}: {variant}",
                f"<div style='position:relative; width:300px; height:200px; background:#f0f0f0; "
                f"border:1px solid #ccc;'>"
                f"<div style='position:absolute; top:20px; left:20px; width:100px; height:80px; "
                f"background:#e74c3c;'></div></div>",
            )

    # --- float ---
    if feature == "float":
        if sub == "value":
            float_map = {
                "left": "float:left",
                "right": "float:right",
                "none": "float:none",
            }
            v = float_map.get(variant)
            if v:
                return wrap_html(
                    f"float: {variant}",
                    f"<div style='{v}; width:100px; height:100px; background:#2ecc71; margin:5px;'></div>"
                    f"<p>Text wrapping around the floated element. {SAMPLE_TEXT}</p>",
                )
        # clearance / exclusion sub-features
        return wrap_html(
            f"float {sub}: {variant}",
            f"<div style='float:left; width:100px; height:100px; background:#2ecc71; margin:5px;'></div>"
            f"<div style='float:right; width:80px; height:80px; background:#e74c3c; margin:5px;'></div>"
            f"<p style='clear:both;'>Text after clearing floats. {SAMPLE_TEXT}</p>",
        )

    # --- clear ---
    if feature == "clear" and sub == "value":
        return wrap_html(
            f"clear: {variant}",
            f"<div style='float:left; width:80px; height:80px; background:#2ecc71;'></div>"
            f"<div style='float:right; width:80px; height:80px; background:#e74c3c;'></div>"
            f"<div style='clear:{variant}; background:#3498db; padding:10px; color:white;'>"
            f"Cleared element</div>",
        )

    # --- margin ---
    if feature == "margin":
        if sub == "value":
            m_map = {"px": "20px", "auto": "auto", "%": "10%", "negative": "-10px"}
            v = m_map.get(variant)
            if v:
                extra = "width:200px;" if variant == "auto" else ""
                return _box_test(f"margin:{v}; {extra}", f"margin: {variant}")
        if sub == "collapsing":
            collapse_bodies = {
                "collapsing-siblings": (
                    "<div style='margin-bottom:20px; background:#e74c3c; height:60px; width:200px;'></div>"
                    "<div style='margin-top:30px; background:#3498db; height:60px; width:200px;'></div>"
                    "<p style='margin-top:10px; font-size:12px;'>Gap should be 30px (not 50px)</p>"
                ),
                "collapsing-parent-child": (
                    "<div style='background:#f0f0f0; border:0;'>"
                    "<div style='margin-top:30px; background:#e74c3c; height:60px; width:200px;'></div>"
                    "</div>"
                    "<div style='background:#3498db; height:60px; width:200px;'></div>"
                ),
                "collapsing-through-empty": (
                    "<div style='margin-bottom:20px; background:#e74c3c; height:60px; width:200px;'></div>"
                    "<div style='margin-top:30px; margin-bottom:10px;'></div>"
                    "<div style='margin-top:15px; background:#3498db; height:60px; width:200px;'></div>"
                ),
            }
            body = collapse_bodies.get(variant)
            if body:
                return wrap_html(f"margin {sub}: {variant}", body)

    # --- margin-collapsing ---
    if feature == "margin-collapsing" and sub == "rule":
        rule_bodies = {
            "positive-positive": (
                "<div style='margin-bottom:30px; background:#e74c3c; height:50px; width:200px;'></div>"
                "<div style='margin-top:20px; background:#3498db; height:50px; width:200px;'></div>"
            ),
            "positive-negative": (
                "<div style='margin-bottom:30px; background:#e74c3c; height:50px; width:200px;'></div>"
                "<div style='margin-top:-10px; background:#3498db; height:50px; width:200px;'></div>"
            ),
            "negative-negative": (
                "<div style='margin-bottom:-20px; background:#e74c3c; height:50px; width:200px;'></div>"
                "<div style='margin-top:-30px; background:#3498db; height:50px; width:200px;'></div>"
            ),
            "bfc-prevents": (
                "<div style='overflow:hidden; background:#f0f0f0;'>"
                "<div style='margin-bottom:30px; background:#e74c3c; height:50px; width:200px;'></div>"
                "</div>"
                "<div style='margin-top:20px; background:#3498db; height:50px; width:200px;'></div>"
            ),
            "clearance-prevents": (
                "<div style='float:left; width:50px; height:50px; background:#2ecc71;'></div>"
                "<div style='clear:left; margin-top:20px; background:#e74c3c; height:50px; width:200px;'></div>"
                "<div style='margin-top:20px; background:#3498db; height:50px; width:200px;'></div>"
            ),
            "padding-border-prevent": (
                "<div style='padding-top:1px; background:#f0f0f0;'>"
                "<div style='margin-top:30px; background:#e74c3c; height:50px; width:200px;'></div>"
                "</div>"
                "<div style='margin-top:20px; background:#3498db; height:50px; width:200px;'></div>"
            ),
        }
        body = rule_bodies.get(variant)
        if body:
            return wrap_html(f"margin-collapsing: {variant}", body)

    # --- padding ---
    if feature == "padding" and sub == "value":
        p_map = {"px": "20px", "%": "5%"}
        v = p_map.get(variant)
        if v:
            return wrap_html(
                f"padding: {variant}",
                f"<div style='padding:{v}; background:#3498db; width:200px;'>"
                f"<div style='background:#e74c3c; height:80px;'></div></div>",
            )

    # --- width / height ---
    for prop in ("width", "height"):
        if feature == prop and sub == "value":
            dim_map = {
                "px": "150px", "auto": "auto", "%": "50%",
                "min-content": "min-content", "max-content": "max-content",
                "fit-content": "fit-content",
            }
            v = dim_map.get(variant)
            if v:
                other_dim = "height:100px;" if prop == "width" else "width:200px;"
                return wrap_html(
                    f"{prop}: {variant}",
                    f"<div style='{prop}:{v}; {other_dim} background:#e74c3c; border:1px solid #222;'>"
                    f"{SAMPLE_TEXT}</div>",
                )

    # --- min-width / max-width / min-height / max-height ---
    for prop in ("min-width", "max-width", "min-height", "max-height"):
        if feature == prop and sub == "value":
            dim_map = {"px": "150px", "%": "50%", "none": "none", "min-content": "min-content", "max-content": "max-content"}
            v = dim_map.get(variant)
            if v:
                base = "width:300px; height:150px;" if "width" in prop else "width:200px; height:300px;"
                return wrap_html(
                    f"{prop}: {variant}",
                    f"<div style='{base} {prop}:{v}; background:#e74c3c; border:1px solid #222;'>"
                    f"{SAMPLE_TEXT}</div>",
                )

    # --- box-sizing ---
    if feature == "box-sizing" and sub == "value":
        return wrap_html(
            f"box-sizing: {variant}",
            f"<div style='box-sizing:{variant}; width:200px; height:100px; padding:20px; "
            f"border:5px solid #222; background:#e74c3c;'></div>"
            f"<div style='margin-top:10px; width:200px; height:20px; background:#3498db;'>"
            f"200px reference</div>",
        )

    # --- overflow / overflow-x / overflow-y ---
    if feature in ("overflow", "overflow-x", "overflow-y") and sub == "value":
        return wrap_html(
            f"{feature}: {variant}",
            f"<div style='width:150px; height:100px; {feature}:{variant}; "
            f"background:#f0f0f0; border:1px solid #ccc;'>"
            f"<div style='width:300px; height:200px; background:linear-gradient(135deg, #e74c3c, #3498db);'>"
            f"Overflow content</div></div>",
        )

    # --- top / right / bottom / left ---
    if feature in ("top", "right", "bottom", "left") and sub == "offset":
        off_map = {"px": "30px", "%": "10%", "auto": "auto"}
        v = off_map.get(variant)
        if v:
            return wrap_html(
                f"{feature}: {variant}",
                f"<div style='position:relative; width:300px; height:200px; background:#f0f0f0; "
                f"border:1px solid #ccc;'>"
                f"<div style='position:absolute; {feature}:{v}; width:80px; height:60px; "
                f"background:#e74c3c;'></div></div>",
            )

    # --- z-index ---
    if feature == "z-index" and sub == "value":
        zi_map = {"auto": "auto", "positive": "10", "negative": "-1"}
        v = zi_map.get(variant)
        if v:
            return wrap_html(
                f"z-index: {variant}",
                f"<div style='position:relative; width:300px; height:200px;'>"
                f"<div style='position:absolute; top:0; left:0; width:120px; height:120px; "
                f"background:#e74c3c; z-index:1;'></div>"
                f"<div style='position:absolute; top:40px; left:40px; width:120px; height:120px; "
                f"background:#3498db; z-index:{v};'></div></div>",
            )

    # --- border-width ---
    if feature == "border-width" and sub == "value":
        bw_map = {"thin": "thin", "medium": "medium", "thick": "thick", "px": "3px"}
        v = bw_map.get(variant)
        if v:
            return wrap_html(
                f"border-width: {variant}",
                f"<div style='width:200px; height:100px; border:{v} solid #e74c3c; "
                f"background:#f0f0f0;'></div>",
            )

    # --- aspect-ratio ---
    if feature == "aspect-ratio" and sub == "value":
        ar_map = {"auto": "auto", "ratio": "16 / 9", "auto-with-ratio": "auto 16 / 9"}
        v = ar_map.get(variant)
        if v:
            return wrap_html(
                f"aspect-ratio: {variant}",
                f"<div style='width:200px; aspect-ratio:{v}; background:#e74c3c; "
                f"border:1px solid #222;'></div>",
            )

    # --- flex properties ---
    if feature == "flex-direction" and sub == "value":
        return wrap_html(
            f"flex-direction: {variant}",
            f"<div style='display:flex; flex-direction:{variant}; gap:8px; "
            f"width:300px; background:#f0f0f0; padding:10px;'>"
            f"<div style='width:60px; height:60px; background:#e74c3c; color:white; "
            f"display:flex; align-items:center; justify-content:center;'>1</div>"
            f"<div style='width:60px; height:60px; background:#3498db; color:white; "
            f"display:flex; align-items:center; justify-content:center;'>2</div>"
            f"<div style='width:60px; height:60px; background:#2ecc71; color:white; "
            f"display:flex; align-items:center; justify-content:center;'>3</div></div>",
        )

    if feature == "flex-wrap" and sub == "value":
        return wrap_html(
            f"flex-wrap: {variant}",
            f"<div style='display:flex; flex-wrap:{variant}; width:200px; gap:5px; "
            f"background:#f0f0f0; padding:10px;'>"
            + "".join(
                f"<div style='width:80px; height:50px; background:hsl({i*60},70%,50%); "
                f"color:white; display:flex; align-items:center; justify-content:center;'>{i+1}</div>"
                for i in range(4)
            )
            + "</div>",
        )

    if feature == "flex-grow" and sub == "value":
        fg_map = {"0": "0", "1": "1", "custom": "2"}
        v = fg_map.get(variant)
        if v:
            return wrap_html(
                f"flex-grow: {variant}",
                f"<div style='display:flex; width:400px; background:#f0f0f0; padding:5px; gap:5px;'>"
                f"<div style='flex-grow:{v}; height:60px; background:#e74c3c; min-width:50px;'></div>"
                f"<div style='flex-grow:1; height:60px; background:#3498db; min-width:50px;'></div></div>",
            )

    if feature == "flex-shrink" and sub == "value":
        fs_map = {"0": "0", "1": "1", "custom": "3"}
        v = fs_map.get(variant)
        if v:
            return wrap_html(
                f"flex-shrink: {variant}",
                f"<div style='display:flex; width:200px; background:#f0f0f0; padding:5px; gap:5px;'>"
                f"<div style='flex-shrink:{v}; width:150px; height:60px; background:#e74c3c;'></div>"
                f"<div style='flex-shrink:1; width:150px; height:60px; background:#3498db;'></div></div>",
            )

    if feature == "flex-basis" and sub == "value":
        fb_map = {"auto": "auto", "0": "0", "px": "100px", "%": "50%", "content": "content"}
        v = fb_map.get(variant)
        if v:
            return wrap_html(
                f"flex-basis: {variant}",
                f"<div style='display:flex; width:400px; background:#f0f0f0; padding:5px; gap:5px;'>"
                f"<div style='flex-basis:{v}; height:60px; background:#e74c3c;'>Content</div>"
                f"<div style='flex-basis:100px; height:60px; background:#3498db;'>100px</div></div>",
            )

    if feature == "flex" and sub == "algorithm":
        return wrap_html(
            f"flex algorithm: {variant}",
            f"<div style='display:flex; width:300px; background:#f0f0f0; padding:5px; gap:5px;'>"
            f"<div style='flex:1 1 0; min-width:50px; height:60px; background:#e74c3c;'></div>"
            f"<div style='flex:2 1 0; min-width:50px; height:60px; background:#3498db;'></div>"
            f"<div style='flex:1 1 0; min-width:50px; height:60px; background:#2ecc71;'></div></div>",
        )

    # --- align / justify ---
    for prop in ("align-content", "align-items", "align-self", "justify-content"):
        if feature == prop and sub == "value":
            container_style = "display:flex; flex-wrap:wrap; width:300px; height:250px; background:#f0f0f0; padding:5px; gap:5px;"
            if prop == "align-self":
                return wrap_html(
                    f"{prop}: {variant}",
                    f"<div style='display:flex; width:300px; height:200px; background:#f0f0f0; "
                    f"padding:5px; gap:5px; align-items:flex-start;'>"
                    f"<div style='width:60px; height:60px; background:#e74c3c;'></div>"
                    f"<div style='width:60px; height:60px; background:#3498db; align-self:{variant};'></div>"
                    f"<div style='width:60px; height:60px; background:#2ecc71;'></div></div>",
                )
            return wrap_html(
                f"{prop}: {variant}",
                f"<div style='{container_style} {prop}:{variant};'>"
                + "".join(
                    f"<div style='width:60px; height:60px; background:hsl({i*50},70%,50%);'></div>"
                    for i in range(5)
                )
                + "</div>",
            )

    # --- order ---
    if feature == "order" and sub == "value":
        o_map = {"0": "0", "positive": "2", "negative": "-1"}
        v = o_map.get(variant)
        if v:
            return wrap_html(
                f"order: {variant}",
                f"<div style='display:flex; gap:5px; background:#f0f0f0; padding:10px;'>"
                f"<div style='order:0; width:60px; height:60px; background:#e74c3c; color:white; "
                f"display:flex; align-items:center; justify-content:center;'>A(0)</div>"
                f"<div style='order:{v}; width:60px; height:60px; background:#3498db; color:white; "
                f"display:flex; align-items:center; justify-content:center;'>B({v})</div>"
                f"<div style='order:1; width:60px; height:60px; background:#2ecc71; color:white; "
                f"display:flex; align-items:center; justify-content:center;'>C(1)</div></div>",
            )

    # --- column-* ---
    if feature == "column-count" and sub == "value":
        cc_map = {"auto": "auto", "integer": "3"}
        v = cc_map.get(variant)
        if v:
            return wrap_html(
                f"column-count: {variant}",
                f"<div style='column-count:{v}; width:500px; background:#f0f0f0; padding:10px;'>"
                f"<p>{LONG_TEXT} {LONG_TEXT}</p></div>",
            )

    if feature == "column-width" and sub == "value":
        cw_map = {"auto": "auto", "length": "150px"}
        v = cw_map.get(variant)
        if v:
            return wrap_html(
                f"column-width: {variant}",
                f"<div style='column-width:{v}; width:500px; background:#f0f0f0; padding:10px;'>"
                f"<p>{LONG_TEXT} {LONG_TEXT}</p></div>",
            )

    if feature == "column-gap" and sub == "value":
        cg_map = {"normal": "normal", "length": "30px"}
        v = cg_map.get(variant)
        if v:
            return wrap_html(
                f"column-gap: {variant}",
                f"<div style='column-count:3; column-gap:{v}; width:500px; background:#f0f0f0; "
                f"padding:10px;'><p>{LONG_TEXT} {LONG_TEXT}</p></div>",
            )

    if feature == "column-rule":
        if sub == "style":
            return wrap_html(
                f"column-rule style: {variant}",
                f"<div style='column-count:3; column-rule:2px {variant} #e74c3c; width:500px; "
                f"background:#f0f0f0; padding:10px;'><p>{LONG_TEXT} {LONG_TEXT}</p></div>",
            )
        if sub == "width":
            cw = {"thin": "thin", "medium": "medium", "thick": "thick"}
            v = cw.get(variant, "medium")
            return wrap_html(
                f"column-rule width: {variant}",
                f"<div style='column-count:3; column-rule:{v} solid #e74c3c; width:500px; "
                f"background:#f0f0f0; padding:10px;'><p>{LONG_TEXT} {LONG_TEXT}</p></div>",
            )

    if feature == "column-span" and sub == "value":
        return wrap_html(
            f"column-span: {variant}",
            f"<div style='column-count:3; width:500px; background:#f0f0f0; padding:10px;'>"
            f"<p>{SAMPLE_TEXT}</p>"
            f"<h2 style='column-span:{variant}; background:#e74c3c; color:white; padding:8px;'>"
            f"Spanning heading</h2>"
            f"<p>{LONG_TEXT}</p></div>",
        )

    # --- break-before / break-after / break-inside ---
    if feature in ("break-before", "break-after", "break-inside") and sub == "value":
        return wrap_html(
            f"{feature}: {variant}",
            f"<div style='column-count:2; width:400px; background:#f0f0f0; padding:10px;'>"
            f"<div style='background:#e74c3c; padding:10px; color:white; margin-bottom:5px;'>Block A</div>"
            f"<div style='{feature}:{variant}; background:#3498db; padding:10px; color:white; "
            f"margin-bottom:5px;'>Block B</div>"
            f"<div style='background:#2ecc71; padding:10px; color:white;'>Block C</div></div>",
        )

    # --- orphans / widows ---
    if feature in ("orphans", "widows") and sub == "value":
        return wrap_html(
            f"{feature}: {variant}",
            f"<div style='column-count:2; width:400px; height:200px; background:#f0f0f0; "
            f"padding:10px; {feature}:3;'><p>{LONG_TEXT} {LONG_TEXT} {LONG_TEXT}</p></div>",
        )

    # --- length-resolution ---
    if feature == "length-resolution":
        if sub == "unit":
            unit_map = {"px": "100px", "em": "6em", "%": "50%", "rem": "6rem", "vw-vh": "20vw"}
            v = unit_map.get(variant)
            if v:
                return wrap_html(
                    f"length unit: {variant}",
                    f"<div style='width:{v}; height:80px; background:#e74c3c; border:1px solid #222;'></div>"
                    f"<div style='margin-top:10px; width:100px; height:20px; background:#3498db;'>100px ref</div>",
                )
        if sub == "keyword" and variant == "auto":
            return wrap_html(
                "length keyword: auto",
                "<div style='width:auto; height:80px; background:#e74c3c; border:1px solid #222;'></div>",
            )

    # --- formatting-context ---
    if feature == "formatting-context":
        if sub == "bfc-creation":
            bfc_map = {
                "overflow-triggers-bfc": "overflow:hidden",
                "float-triggers-bfc": "float:left; width:300px",
                "flex-item-bfc": "display:flex",
                "flow-root-triggers-bfc": "display:flow-root",
                "abspos-triggers-bfc": "position:absolute; width:300px",
            }
            css = bfc_map.get(variant, "overflow:hidden")
            return wrap_html(
                f"BFC creation: {variant}",
                f"<div style='{css}; background:#f0f0f0; padding:10px;'>"
                f"<div style='float:left; width:80px; height:80px; background:#e74c3c;'></div>"
                f"<p>Content inside BFC. {SAMPLE_TEXT}</p></div>",
            )
        if sub == "bfc-resolution":
            return wrap_html(
                f"BFC resolution: {variant}",
                f"<div style='overflow:hidden; background:#f0f0f0; padding:10px;'>"
                f"<div style='float:left; width:80px; height:80px; background:#e74c3c;'></div>"
                f"<p>BFC offset resolution. {SAMPLE_TEXT}</p></div>",
            )

    # --- intrinsic-sizing / layout-infra / fragmentation / multicol ---
    if feature in ("intrinsic-sizing", "layout-infra", "fragmentation", "multicol"):
        algo_bodies = {
            "intrinsic-sizing": (
                "<div style='display:flex; gap:10px;'>"
                "<div style='width:min-content; background:#e74c3c; padding:10px;'>Min content width test</div>"
                "<div style='width:max-content; background:#3498db; padding:10px;'>Max content width test</div></div>"
            ),
            "layout-infra": (
                "<div style='width:50%; background:#e74c3c; padding:10px; height:100px;'>"
                "50% width container</div>"
            ),
            "fragmentation": (
                "<div style='column-count:2; width:400px; background:#f0f0f0; padding:10px;'>"
                f"<p>{LONG_TEXT} {LONG_TEXT} {LONG_TEXT}</p></div>"
            ),
            "multicol": (
                "<div style='column-count:3; column-gap:20px; width:500px; background:#f0f0f0; "
                "padding:10px;'>"
                f"<p>{LONG_TEXT} {LONG_TEXT} {LONG_TEXT}</p></div>"
            ),
        }
        return wrap_html(
            f"{feature} {sub}: {variant}",
            algo_bodies.get(feature, f"<p>{feature} {sub}: {variant}</p>"),
        )

    return ""  # not handled


# ===================================================================
# SP13 – INLINE FEATURES
# ===================================================================

def generate_sp13(feature: str, sub: str, variant: str) -> str:
    key = (feature, sub, variant)

    # --- inline_formatting_context ---
    if feature == "inline_formatting_context":
        if variant == "single_text_node":
            return wrap_html("IFC: single text node", f"<p>{SAMPLE_TEXT}</p>")
        if variant == "sibling_spans":
            return wrap_html(
                "IFC: sibling spans",
                "<p><span style='color:#e74c3c;'>Red span</span> "
                "<span style='color:#3498db;'>Blue span</span> "
                "<span style='color:#2ecc71;'>Green span</span></p>",
            )
        if variant == "deeply_nested":
            return wrap_html(
                "IFC: deeply nested",
                "<p><span style='color:#e74c3c;'>"
                "<span style='font-weight:bold;'>"
                "<span style='font-style:italic;'>"
                f"{SAMPLE_TEXT}</span></span></span></p>",
            )

    # --- line_breaking ---
    if feature == "line_breaking":
        if variant == "normal_text":
            return wrap_html(
                "line-breaking: normal text",
                f"<div style='width:200px; border:1px solid #222; padding:8px; background:#f0f0f0;'>"
                f"<span>{LONG_TEXT}</span></div>",
            )
        if variant == "br_and_newline":
            return wrap_html(
                "line-breaking: forced breaks",
                "<div style='width:300px; border:1px solid #222; padding:8px; background:#f0f0f0;'>"
                "Line one<br>Line two<br>Line three</div>",
            )
        if variant == "overflow_character_break":
            return wrap_html(
                "line-breaking: emergency break",
                "<div style='width:100px; border:1px solid #222; padding:8px; background:#f0f0f0; "
                "overflow-wrap:break-word;'>Supercalifragilisticexpialidocious</div>",
            )

    # --- text_align ---
    if feature == "text_align" and sub == "inline_justification":
        return wrap_html(
            f"text-align inline: {variant}",
            f"<div style='width:400px; background:#f0f0f0; padding:8px; text-align:{variant}; "
            f"border:1px solid #ccc;'><p>{LONG_TEXT}</p></div>",
        )

    # --- text_align_last ---
    if feature == "text_align_last" and sub == "last_line_alignment":
        return wrap_html(
            f"text-align-last: {variant}",
            f"<div style='width:300px; background:#f0f0f0; padding:8px; text-align:justify; "
            f"text-align-last:{variant}; border:1px solid #ccc;'><p>{LONG_TEXT}</p></div>",
        )

    # --- vertical_align ---
    if feature == "vertical_align" and sub == "inline_positioning":
        va_map = {
            "baseline": "baseline", "sub": "sub", "super": "super",
            "text-top": "text-top", "text-bottom": "text-bottom",
            "middle": "middle", "top": "top", "bottom": "bottom",
            "length": "10px", "percentage": "50%",
        }
        v = va_map.get(variant)
        if v:
            return wrap_html(
                f"vertical-align: {variant}",
                f"<p style='font-size:24px; line-height:3; background:#f0f0f0; padding:8px;'>"
                f"Normal <span style='vertical-align:{v}; background:#e74c3c; color:white; "
                f"padding:2px 4px;'>aligned</span> text</p>",
            )

    # --- white_space ---
    if feature == "white_space" and sub == "inline_processing":
        text = "  The   quick   brown\n  fox   jumps   over\n  the   lazy   dog  "
        return wrap_html(
            f"white-space: {variant}",
            f"<div style='width:300px; background:#f0f0f0; padding:8px; "
            f"white-space:{variant}; border:1px solid #ccc;'>{text}</div>",
        )

    # --- word_break ---
    if feature == "word_break" and sub == "inline_behavior":
        text = "Supercalifragilisticexpialidocious and 日本語テキスト"
        return wrap_html(
            f"word-break: {variant}",
            f"<div style='width:200px; background:#f0f0f0; padding:8px; "
            f"word-break:{variant}; border:1px solid #ccc;'>{text}</div>",
        )

    # --- overflow_wrap ---
    if feature == "overflow_wrap" and sub == "inline_behavior":
        text = "Averylongwordthatwillnotfit and normal text"
        return wrap_html(
            f"overflow-wrap: {variant}",
            f"<div style='width:200px; background:#f0f0f0; padding:8px; "
            f"overflow-wrap:{variant}; border:1px solid #ccc;'>{text}</div>",
        )

    # --- text_transform ---
    if feature == "text_transform" and sub == "inline_processing":
        return wrap_html(
            f"text-transform: {variant}",
            f"<p style='text-transform:{variant}; background:#f0f0f0; padding:8px;'>"
            f"{SAMPLE_TEXT}</p>\n"
            f"<p style='margin-top:10px;'>Normal reference text.</p>",
        )

    # --- text_indent ---
    if feature == "text_indent" and sub == "first_line_indent":
        indent_map = {"px": "40px", "percent": "10%", "hanging": "40px hanging", "each-line": "40px each-line"}
        v = indent_map.get(variant)
        if v:
            return wrap_html(
                f"text-indent: {variant}",
                f"<div style='width:400px; background:#f0f0f0; padding:8px; text-indent:{v}; "
                f"border:1px solid #ccc;'><p>{LONG_TEXT}</p></div>",
            )

    # --- text_justify ---
    if feature == "text_justify" and sub == "justification_method":
        return wrap_html(
            f"text-justify: {variant}",
            f"<div style='width:300px; background:#f0f0f0; padding:8px; text-align:justify; "
            f"text-justify:{variant}; border:1px solid #ccc;'><p>{LONG_TEXT}</p></div>",
        )

    # --- text_overflow ---
    if feature == "text_overflow" and sub == "truncation":
        direction_style = " direction:rtl;" if variant == "ellipsis_rtl" else ""
        return wrap_html(
            f"text-overflow: {variant}",
            f"<div style='width:200px; white-space:nowrap; overflow:hidden; "
            f"text-overflow:ellipsis;{direction_style} background:#f0f0f0; padding:8px; "
            f"border:1px solid #ccc;'>{SAMPLE_TEXT}</div>",
        )

    # --- text_wrap ---
    if feature == "text_wrap" and sub == "line_breaking_mode":
        return wrap_html(
            f"text-wrap: {variant}",
            f"<div style='width:300px; background:#f0f0f0; padding:8px; text-wrap:{variant}; "
            f"border:1px solid #ccc;'><p>{LONG_TEXT}</p></div>",
        )

    # --- text_combine_upright ---
    if feature == "text_combine_upright" and sub == "tate_chu_yoko":
        return wrap_html(
            f"text-combine-upright: {variant}",
            f"<div style='writing-mode:vertical-rl; height:300px; background:#f0f0f0; padding:8px;'>"
            f"<p>令和<span style='text-combine-upright:{variant};'>12</span>年</p></div>",
        )

    # --- letter_spacing ---
    if feature == "letter_spacing" and sub == "inline_spacing":
        return wrap_html(
            "letter-spacing: applied",
            f"<p style='letter-spacing:3px; background:#f0f0f0; padding:8px;'>{SAMPLE_TEXT}</p>",
        )

    # --- word_spacing ---
    if feature == "word_spacing" and sub == "inline_spacing":
        return wrap_html(
            "word-spacing: applied",
            f"<p style='word-spacing:10px; background:#f0f0f0; padding:8px;'>{SAMPLE_TEXT}</p>",
        )

    # --- line_height ---
    if feature == "line_height" and sub == "half_leading_model":
        lh_map = {"normal": "normal", "number": "2", "length": "36px", "percentage": "200%"}
        v = lh_map.get(variant)
        if v:
            return wrap_html(
                f"line-height: {variant}",
                f"<div style='width:400px; background:#f0f0f0; padding:8px; line-height:{v}; "
                f"border:1px solid #ccc;'><p>{LONG_TEXT}</p></div>",
            )

    # --- line_break (CJK rules) ---
    if feature == "line_break" and sub == "cjk_rules":
        return wrap_html(
            f"line-break: {variant}",
            f"<div style='width:200px; background:#f0f0f0; padding:8px; "
            f"line-break:{variant}; border:1px solid #ccc;'>"
            f"日本語テスト。テキスト、折り返し！テスト。</div>",
        )

    # --- hyphens ---
    if feature == "hyphens" and sub == "hyphenation":
        return wrap_html(
            f"hyphens: {variant}",
            f"<div lang='en' style='width:150px; background:#f0f0f0; padding:8px; "
            f"hyphens:{variant}; border:1px solid #ccc;'>"
            f"Incomprehensibilities and antidisestablishmentarianism</div>",
        )

    # --- hyphenate_limit_chars ---
    if feature == "hyphenate_limit_chars":
        return wrap_html(
            f"hyphenate-limit-chars: {variant}",
            f"<div lang='en' style='width:150px; background:#f0f0f0; padding:8px; "
            f"hyphens:auto; hyphenate-limit-chars:6 3 2; border:1px solid #ccc;'>"
            f"Incomprehensibilities and responsibilities</div>",
        )

    # --- soft_hyphen ---
    if feature == "soft_hyphen":
        return wrap_html(
            "soft-hyphen: U+00AD",
            "<div style='width:100px; background:#f0f0f0; padding:8px; border:1px solid #ccc;'>"
            "In\u00ADcom\u00ADpre\u00ADhen\u00ADsi\u00ADbil\u00ADi\u00ADties</div>",
        )

    # --- tab_size ---
    if feature == "tab_size":
        return wrap_html(
            f"tab-size: {variant}",
            "<pre style='tab-size:4; background:#f0f0f0; padding:8px; border:1px solid #ccc;'>"
            "No tab\n\tOne tab\n\t\tTwo tabs</pre>",
        )

    # --- bidi ---
    if feature == "bidi":
        if sub == "inline_direction":
            dir_map = {"ltr": "ltr", "rtl": "rtl", "mixed": "ltr"}
            d = dir_map.get(variant, "ltr")
            text = "Hello مرحبا World" if variant == "mixed" else SAMPLE_TEXT
            return wrap_html(
                f"bidi direction: {variant}",
                f"<div style='direction:{d}; width:400px; background:#f0f0f0; padding:8px; "
                f"border:1px solid #ccc;'>{text}</div>",
            )
        if sub == "unicode_bidi":
            return wrap_html(
                f"unicode-bidi: {variant}",
                f"<div style='direction:rtl; width:400px; background:#f0f0f0; padding:8px;'>"
                f"<span style='unicode-bidi:{variant}; direction:ltr;'>{SAMPLE_TEXT}</span></div>",
            )
        if sub == "visual_reorder":
            return wrap_html(
                f"bidi visual reorder: {variant}",
                "<div style='direction:rtl; width:400px; background:#f0f0f0; padding:8px;'>"
                "<span style='unicode-bidi:isolate; direction:ltr;'>Hello</span> "
                "مرحبا "
                "<span style='unicode-bidi:isolate; direction:ltr;'>World</span></div>",
            )

    # --- inline_block ---
    if feature == "inline_block" and sub == "layout":
        if variant == "basic":
            return wrap_html(
                "inline-block: basic",
                "<p>Text <span style='display:inline-block; width:80px; height:40px; "
                "background:#e74c3c;'></span> more text "
                "<span style='display:inline-block; width:60px; height:60px; "
                "background:#3498db;'></span> end.</p>",
            )
        if variant == "with_overflow":
            return wrap_html(
                "inline-block: with overflow",
                "<p>Text <span style='display:inline-block; width:80px; height:40px; "
                "overflow:hidden; background:#e74c3c;'>"
                "<span style='width:200px; height:200px; display:block; background:#c0392b;'>"
                "</span></span> more text.</p>",
            )
        if variant == "with_vertical_align":
            return wrap_html(
                "inline-block: vertical-align",
                "<p style='font-size:24px;'>Text "
                "<span style='display:inline-block; width:60px; height:60px; "
                "background:#e74c3c; vertical-align:middle;'></span> "
                "<span style='display:inline-block; width:40px; height:40px; "
                "background:#3498db; vertical-align:top;'></span> end.</p>",
            )

    # --- block_in_inline ---
    if feature == "block_in_inline":
        if sub == "anonymous_splitting":
            if variant == "basic":
                return wrap_html(
                    "block-in-inline: basic",
                    "<span style='background:#e74c3c; color:white; padding:2px 4px;'>"
                    "Inline start <div style='background:#3498db; padding:10px; margin:5px 0;'>"
                    "Block inside inline</div> Inline end</span>",
                )
            if variant == "multiple_blocks":
                return wrap_html(
                    "block-in-inline: multiple blocks",
                    "<span style='background:#e74c3c; color:white; padding:2px 4px;'>"
                    "Start <div style='background:#3498db; padding:8px; margin:3px 0;'>Block 1</div>"
                    "Middle <div style='background:#2ecc71; padding:8px; margin:3px 0;'>Block 2</div>"
                    " End</span>",
                )
            if variant == "nested_inline_with_block":
                return wrap_html(
                    "block-in-inline: nested",
                    "<span style='background:#e74c3c; color:white;'>"
                    "<span style='font-weight:bold;'>Nested "
                    "<div style='background:#3498db; padding:8px; margin:3px 0;'>Block</div>"
                    " inline</span></span>",
                )
        if sub == "continuation":
            return wrap_html(
                f"block-in-inline continuation: {variant}",
                "<span style='background:#e74c3c; color:white; padding:4px;'>"
                "Before <div style='background:#3498db; padding:10px; margin:5px 0;'>"
                "Block</div> After</span>",
            )

    # --- first_letter ---
    if feature == "first_letter":
        extra = ""
        if variant == "cjk":
            extra = "\n .test-content { font-family: serif; }"
            text = "漢字テスト文章です。" + SAMPLE_TEXT
        elif variant == "with_punctuation" or variant == "trailing_punctuation":
            text = "\"Hello, world! " + SAMPLE_TEXT
        elif variant == "open_bracket":
            text = "(Hello) " + SAMPLE_TEXT
        elif variant == "no_letter":
            text = "123 456 789"
        elif variant == "multi_codepoint":
            text = "é " + SAMPLE_TEXT
        else:
            text = SAMPLE_TEXT
        return wrap_html(
            f"::first-letter: {variant}",
            f"<div class='test-content' style='width:400px; background:#f0f0f0; padding:10px;'>"
            f"<p style='margin:0;'>{text}</p></div>",
            extra_style=f"\n .test-content::first-letter {{ color:#e74c3c; font-size:2em; font-weight:bold; }}{extra}",
        )

    # --- first_line ---
    if feature == "first_line":
        return wrap_html(
            f"::first-line: {variant}",
            f"<div style='width:400px; background:#f0f0f0; padding:10px;'>"
            f"<p>{LONG_TEXT}</p></div>",
            extra_style="\n p::first-line { color:#e74c3c; font-weight:bold; text-decoration:underline; }",
        )

    # --- float_exclusion ---
    if feature == "float_exclusion" and sub == "inline_context":
        float_map = {
            "left_float": "float:left",
            "right_float": "float:right",
            "both_floats": "",
            "per_line_width_reduction": "float:left",
        }
        if variant == "both_floats":
            return wrap_html(
                "float-exclusion: both floats",
                "<div style='width:400px; background:#f0f0f0; padding:8px;'>"
                "<div style='float:left; width:80px; height:80px; background:#e74c3c; margin:0 8px 8px 0;'></div>"
                "<div style='float:right; width:60px; height:60px; background:#3498db; margin:0 0 8px 8px;'></div>"
                f"<p>{LONG_TEXT}</p></div>",
            )
        fl = float_map.get(variant, "float:left")
        return wrap_html(
            f"float-exclusion: {variant}",
            f"<div style='width:400px; background:#f0f0f0; padding:8px;'>"
            f"<div style='{fl}; width:80px; height:80px; background:#e74c3c; margin:0 8px 8px 0;'></div>"
            f"<p>{LONG_TEXT}</p></div>",
        )

    # --- out_of_flow ---
    if feature == "out_of_flow" and sub == "static_position":
        pos = "fixed" if variant == "fixed_in_inline" else "absolute"
        return wrap_html(
            f"out-of-flow: {variant}",
            f"<div style='position:relative; width:400px; height:200px; background:#f0f0f0; padding:8px;'>"
            f"<p>Text before <span style='position:{pos}; top:50px; left:50px; "
            f"background:#e74c3c; color:white; padding:8px;'>OOF element</span> text after.</p></div>",
        )

    # --- inline_box_state ---
    if feature == "inline_box_state":
        if sub == "mbp_tracking":
            mbp_bodies = {
                "single_line_full_mbp": (
                    "<p><span style='background:#e74c3c; color:white; padding:4px 8px; "
                    "border:2px solid #c0392b; margin:0 4px;'>Single line span with MBP</span></p>"
                ),
                "multi_line_first": (
                    "<div style='width:200px;'><span style='background:#e74c3c; color:white; "
                    "padding:4px 8px; border:2px solid #c0392b;'>"
                    f"{LONG_TEXT}</span></div>"
                ),
                "multi_line_middle": (
                    "<div style='width:200px;'><span style='background:#e74c3c; color:white; "
                    "padding:4px 8px; border:2px solid #c0392b;'>"
                    f"{LONG_TEXT}</span></div>"
                ),
                "multi_line_last": (
                    "<div style='width:200px;'><span style='background:#e74c3c; color:white; "
                    "padding:4px 8px; border:2px solid #c0392b;'>"
                    f"{LONG_TEXT}</span></div>"
                ),
                "nested_spans": (
                    "<p><span style='background:#e74c3c; color:white; padding:2px 6px;'>"
                    "Outer <span style='background:#3498db; padding:2px 6px;'>Inner</span> "
                    "outer</span></p>"
                ),
                "empty_span": (
                    "<p>Before <span style='background:#e74c3c; padding:4px 8px; "
                    "border:2px solid #c0392b;'></span> after</p>"
                ),
                "padding_border_contribution": (
                    "<p><span style='background:#e74c3c; color:white; padding:8px 16px; "
                    "border:4px solid #c0392b; margin:4px;'>Padded bordered span</span></p>"
                ),
            }
            body = mbp_bodies.get(variant)
            if body:
                return wrap_html(f"inline-box MBP: {variant}", body)
        if sub == "fragment_metadata":
            return wrap_html(
                f"inline-box fragment: {variant}",
                "<div style='width:200px;'><span style='background:#e74c3c; color:white; "
                f"padding:2px 6px; border:1px solid #c0392b;'>{LONG_TEXT}</span></div>",
            )

    # --- inline_fragmentation ---
    if feature == "inline_fragmentation":
        return wrap_html(
            f"inline-fragmentation: {variant}",
            "<div style='column-count:2; width:400px; background:#f0f0f0; padding:8px;'>"
            f"<p><span style='background:#e74c3c; color:white; padding:2px 4px;'>"
            f"{LONG_TEXT} {LONG_TEXT}</span></p></div>",
        )

    # --- baseline_propagation ---
    if feature == "baseline_propagation":
        return wrap_html(
            f"baseline-propagation: {variant}",
            "<div style='display:flex; align-items:baseline; gap:10px; "
            "background:#f0f0f0; padding:10px;'>"
            "<span style='font-size:24px; background:#e74c3c; color:white; padding:4px;'>Large</span>"
            "<span style='font-size:12px; background:#3498db; color:white; padding:4px;'>Small</span>"
            "<span style='font-size:18px; background:#2ecc71; color:white; padding:4px;'>Medium</span>"
            "</div>",
        )

    # --- box_decoration_break ---
    if feature == "box_decoration_break" and sub == "inline_context":
        return wrap_html(
            f"box-decoration-break: {variant}",
            f"<div style='width:200px; line-height:2;'>"
            f"<span style='box-decoration-break:{variant}; -webkit-box-decoration-break:{variant}; "
            f"background:#e74c3c; color:white; padding:4px 8px;'>{LONG_TEXT}</span></div>",
        )

    # --- cjk_line_breaking ---
    if feature == "cjk_line_breaking":
        return wrap_html(
            f"CJK line-breaking: {variant}",
            "<div style='width:200px; background:#f0f0f0; padding:8px; border:1px solid #ccc;'>"
            "日本語テスト。テキスト、折り返し！テスト。中文测试。</div>",
        )

    # --- ruby_align ---
    if feature == "ruby_align" and sub == "annotation_alignment":
        return wrap_html(
            f"ruby-align: {variant}",
            f"<div style='font-size:24px; background:#f0f0f0; padding:10px;'>"
            f"<ruby style='ruby-align:{variant};'>漢<rp>(</rp><rt>かん</rt><rp>)</rp></ruby>"
            f"<ruby style='ruby-align:{variant};'>字<rp>(</rp><rt>じ</rt><rp>)</rp></ruby></div>",
        )

    # --- ruby_position ---
    if feature == "ruby_position" and sub == "annotation_placement":
        return wrap_html(
            f"ruby-position: {variant}",
            f"<div style='font-size:24px; background:#f0f0f0; padding:10px;'>"
            f"<ruby style='ruby-position:{variant};'>漢<rp>(</rp><rt>かん</rt><rp>)</rp></ruby>"
            f"<ruby style='ruby-position:{variant};'>字<rp>(</rp><rt>じ</rt><rp>)</rp></ruby></div>",
        )

    # --- initial_letter ---
    if feature == "initial_letter":
        if sub == "drop_caps":
            lines = "3" if variant == "three_line" else "2"
            return wrap_html(
                f"initial-letter drop-cap: {variant}",
                f"<div style='width:400px; background:#f0f0f0; padding:10px;'>"
                f"<p><span style='initial-letter:{lines}; color:#e74c3c; font-size:48px;'>T</span>"
                f"he quick brown fox jumps over the lazy dog. {LONG_TEXT}</p></div>",
            )
        if sub == "raised_caps":
            return wrap_html(
                f"initial-letter raised-cap: {variant}",
                f"<div style='width:400px; background:#f0f0f0; padding:10px;'>"
                f"<p><span style='initial-letter:3 1; color:#e74c3c; font-size:48px;'>T</span>"
                f"he quick brown fox. {LONG_TEXT}</p></div>",
            )
        if sub == "exclusion":
            return wrap_html(
                f"initial-letter exclusion: {variant}",
                f"<div style='width:400px; background:#f0f0f0; padding:10px;'>"
                f"<p><span style='initial-letter:3; color:#e74c3c; font-size:48px;'>T</span>"
                f"he quick brown fox. {LONG_TEXT}</p></div>",
            )
        if sub in ("style", "validation"):
            return wrap_html(
                f"initial-letter {sub}: {variant}",
                f"<div style='width:400px; background:#f0f0f0; padding:10px;'>"
                f"<p><span style='initial-letter:2; color:#e74c3c; font-size:36px;'>H</span>"
                f"ello world. {SAMPLE_TEXT}</p></div>",
            )

    # --- score_line_breaker ---
    if feature == "score_line_breaker":
        return wrap_html(
            f"score line-breaker: {variant}",
            f"<div style='width:300px; text-wrap:pretty; background:#f0f0f0; padding:8px; "
            f"border:1px solid #ccc;'><p>{LONG_TEXT} {LONG_TEXT}</p></div>",
        )

    # --- trailing_space ---
    if feature == "trailing_space":
        return wrap_html(
            f"trailing-space: {variant}",
            "<div style='width:200px; background:#f0f0f0; padding:8px; border:1px solid #ccc;'>"
            f"<span>{SAMPLE_TEXT}   </span></div>",
        )

    # --- writing_mode ---
    if feature == "writing_mode":
        return wrap_html(
            f"writing-mode vertical: {variant}",
            "<div style='writing-mode:vertical-rl; height:300px; background:#f0f0f0; padding:8px;'>"
            f"<p>{SAMPLE_TEXT}</p></div>",
        )

    # --- pixel_rendering ---
    if feature == "pixel_rendering":
        if "inline_block" in sub or "inline_block" in variant:
            bodies = {
                "single_render": "<p>Text <span style='display:inline-block; width:60px; height:40px; background:#e74c3c;'></span> end.</p>",
                "adjacent_blocks": "<p><span style='display:inline-block; width:60px; height:40px; background:#e74c3c;'></span><span style='display:inline-block; width:60px; height:40px; background:#3498db;'></span></p>",
                "with_padding": "<p>Text <span style='display:inline-block; width:60px; height:40px; background:#e74c3c; padding:10px;'></span> end.</p>",
                "with_margin": "<p>Text <span style='display:inline-block; width:60px; height:40px; background:#e74c3c; margin:10px;'></span> end.</p>",
                "with_border": "<p>Text <span style='display:inline-block; width:60px; height:40px; background:#e74c3c; border:3px solid #c0392b;'></span> end.</p>",
                "backgrounds": "<p>Text <span style='display:inline-block; width:60px; height:40px; background:linear-gradient(#e74c3c, #3498db);'></span> end.</p>",
                "different_sizes": "<p><span style='display:inline-block; width:30px; height:30px; background:#e74c3c;'></span><span style='display:inline-block; width:60px; height:60px; background:#3498db;'></span><span style='display:inline-block; width:90px; height:20px; background:#2ecc71;'></span></p>",
                "nesting": "<p>Text <span style='display:inline-block; background:#f0f0f0; padding:5px;'><span style='display:inline-block; width:40px; height:40px; background:#e74c3c;'></span></span> end.</p>",
            }
            body = bodies.get(variant)
            if body:
                return wrap_html(f"pixel-rendering inline-block: {variant}", body)
        if variant == "rendering" or sub == "block_in_inline":
            return wrap_html(
                f"pixel-rendering: {variant}",
                "<span style='background:#e74c3c; color:white; padding:2px 4px;'>"
                "Before <div style='background:#3498db; padding:8px; margin:3px 0;'>Block</div>"
                " After</span>",
            )
        if variant == "exclusion" or sub == "float_with_inline":
            return wrap_html(
                f"pixel-rendering: float+inline {variant}",
                "<div style='width:300px; background:#f0f0f0; padding:8px;'>"
                "<div style='float:left; width:60px; height:60px; background:#e74c3c; margin:0 8px 8px 0;'></div>"
                f"<p>{SAMPLE_TEXT}</p></div>",
            )

    # --- intrinsic_block_size ---
    if feature == "intrinsic_block_size" and sub == "from_inline_content":
        ibs_map = {
            "single_word": "<div style='background:#e74c3c; color:white; padding:8px; display:inline-block;'>Hello</div>",
            "text_content": f"<div style='width:300px; background:#e74c3c; color:white; padding:8px;'>{SAMPLE_TEXT}</div>",
            "empty_block": "<div style='background:#e74c3c; width:200px;'></div><div style='height:2px; background:#3498db; width:200px;'>reference</div>",
            "wrapping_content": f"<div style='width:100px; background:#e74c3c; color:white; padding:8px;'>{LONG_TEXT}</div>",
            "with_border_padding": f"<div style='width:200px; background:#e74c3c; color:white; padding:10px; border:3px solid #c0392b;'>{SAMPLE_TEXT}</div>",
            "larger_font": f"<div style='width:300px; background:#e74c3c; color:white; padding:8px; font-size:32px;'>{SAMPLE_TEXT}</div>",
        }
        body = ibs_map.get(variant)
        if body:
            return wrap_html(f"intrinsic-block-size: {variant}", body)

    # --- integration ---
    if feature == "integration":
        return wrap_html(
            f"integration: {variant}",
            f"<div style='width:300px; background:#f0f0f0; padding:10px; border:1px solid #ccc;'>"
            f"<p>{SAMPLE_TEXT}</p></div>",
        )

    return ""  # not handled


# ===================================================================
# Main driver
# ===================================================================

GENERATORS = {
    "sp11": generate_sp11,
    "sp12": generate_sp12,
    "sp13": generate_sp13,
}


def main() -> None:
    total = 0
    generated = 0
    placeholders = 0

    for sp_key, csv_path in CSV_FILES.items():
        if not csv_path.exists():
            print(f"WARNING: {csv_path} not found, skipping.")
            continue

        out_dir = OUTPUT_BASE / sp_key
        out_dir.mkdir(parents=True, exist_ok=True)

        gen_func = GENERATORS[sp_key]

        with open(csv_path, newline="", encoding="utf-8") as fh:
            reader = csv.DictReader(fh)
            for row in reader:
                feature = row["feature"].strip()
                sub_feature = row["sub_feature"].strip()
                variant = row["variant"].strip()
                total += 1

                fname = sanitize_filename(f"{feature}_{sub_feature}_{variant}") + ".html"
                fpath = out_dir / fname

                html = gen_func(feature, sub_feature, variant)
                if not html:
                    html = placeholder(feature, sub_feature, variant)
                    placeholders += 1

                fpath.write_text(html, encoding="utf-8")
                generated += 1

    print(f"\nHTML test generation complete.")
    print(f"  Total rows processed:  {total}")
    print(f"  Files generated:       {generated}")
    print(f"  Mapped tests:          {generated - placeholders}")
    print(f"  Placeholders:          {placeholders}")
    for sp_key in CSV_FILES:
        d = OUTPUT_BASE / sp_key
        if d.exists():
            count = len(list(d.glob("*.html")))
            print(f"  {sp_key}: {count} files in {d.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
