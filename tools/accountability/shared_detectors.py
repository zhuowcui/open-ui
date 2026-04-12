"""Shared cross-SP dependency detectors.

Single source of truth for classifying WPT test HTML by its feature dependencies.
Used by both generate_wpt_mapping.py and generate_sp12_5_csv.py to ensure
consistent classification across all tracking artifacts.
"""

import re


def strip_style_blocks(html: str) -> str:
    """Remove all <style>...</style> blocks from HTML."""
    return re.sub(r"<style[^>]*>.*?</style>", "", html, flags=re.DOTALL | re.IGNORECASE)


def has_visible_text(html: str) -> bool:
    """Detect visible text content between tags after stripping style blocks.

    Uses tag-boundary approach: finds text runs between > and <,
    requires length > 1 and at least one alphanumeric character.
    HTML comments are stripped first to avoid false positives.
    Also detects <br> tags (line breaks) and <span> (inline elements)
    which require inline/text layout.
    """
    stripped = strip_style_blocks(html)
    # Strip HTML comments before checking for text
    stripped = re.sub(r"<!--.*?-->", "", stripped, flags=re.DOTALL)
    # <br> tags require line-break handling (text layout)
    if re.search(r"<br\s*/?\s*>", stripped, re.IGNORECASE):
        return True
    # <span> elements are inline-level and require inline layout
    if re.search(r"<span[\s>]", stripped, re.IGNORECASE):
        return True
    for m in re.finditer(r">([^<]+)<", stripped):
        text = m.group(1).strip()
        if text and not text.isspace() and text != "{":
            if re.search(r"[A-Za-z0-9]", text):
                return True
    return False


def has_font_metrics(html: str) -> bool:
    """Detect dependency on font metrics: ch, ex units, font shorthand, or
    font-dependent line-height.

    ch and ex genuinely depend on the font's glyph metrics.
    font: shorthand (e.g., font: 1.25em/1 Ahem) sets both font-size and
    line-height, requiring font metrics.
    line-height only depends on font metrics when set to 'normal' or a unitless
    number (e.g., '1.5'). Absolute values like '20px' or '0' do NOT depend on
    font metrics.
    """
    # ch/ex units always depend on font glyph metrics
    if re.search(r"[\d.]+(?:ch|ex)\b", html, re.IGNORECASE):
        return True
    # font: shorthand sets font-size/family/line-height (requires font metrics)
    if re.search(r"(?<![a-zA-Z-])font\s*:\s*(?!inherit|initial|unset|revert)", html, re.IGNORECASE):
        return True
    # line-height: normal depends on font metrics
    if re.search(r"line-height\s*:\s*normal", html, re.IGNORECASE):
        return True
    # line-height with unitless number (e.g., 1.5) depends on font metrics
    # (multiplied by font-size which comes from font metrics for 'normal')
    if re.search(r"line-height\s*:\s*\d+\.?\d*\s*[;\}]", html):
        return True
    return False


def has_image_ref(html: str) -> bool:
    """Detect url() near background or border-image properties."""
    return bool(re.search(
        r"(background|border-image)[^;]*url\s*\(", html, re.IGNORECASE
    ))


def has_containment(html: str) -> bool:
    """Detect CSS contain or content-visibility property."""
    return bool(
        re.search(r"(?<![a-zA-Z-])contain\s*:", html, re.IGNORECASE)
        or re.search(r"content-visibility\s*:", html, re.IGNORECASE)
    )


def has_gradient(html: str) -> bool:
    """Detect gradient() function."""
    return bool(re.search(r"gradient\s*\(", html, re.IGNORECASE))


def has_margin_trim(html: str) -> bool:
    """Detect margin-trim CSS property."""
    return bool(re.search(r"margin-trim\s*:", html, re.IGNORECASE))


def has_inline_block(html: str) -> bool:
    """Detect display:inline-block which requires inline layout (SP11)."""
    return bool(re.search(r"display\s*:\s*inline-block", html, re.IGNORECASE))


def has_box_shadow(html: str) -> bool:
    """Detect box-shadow property (not yet implemented)."""
    return bool(re.search(r"box-shadow\s*:", html, re.IGNORECASE))


def has_sticky_position(html: str) -> bool:
    """Detect position:sticky (requires scroll container integration)."""
    return bool(re.search(r"position\s*:\s*sticky", html, re.IGNORECASE))


def has_complex_border_style(html: str) -> bool:
    """Detect dashed/dotted/double/groove/ridge/inset/outset border styles (paint quality)."""
    return bool(re.search(
        r"border(?:-(?:top|right|bottom|left))?-style\s*:\s*(?:dashed|dotted|double|groove|ridge|inset|outset)",
        html, re.IGNORECASE
    ))


def has_scrollbar_gutter(html: str) -> bool:
    """Detect scrollbar-gutter property (not implemented)."""
    return bool(re.search(r"scrollbar-gutter\s*:", html, re.IGNORECASE))


def is_print_layout(html: str, test_id: str = "") -> bool:
    """Detect print-specific layout tests."""
    return "-print" in test_id.split("/")[-1]


def is_reference_test(html: str, test_id: str = "") -> bool:
    """Detect WPT reference files (-ref suffix) which are comparison targets, not standalone tests."""
    name = test_id.split("/")[-1] if test_id else ""
    return "-ref" in name


# Ordered list of (key, label, owning_sp, detector)
# Detectors that need test_id have a special flag.
DEPENDENCY_DEFS = [
    ("reference_test",     "Reference Test (not standalone)", "N/A",       is_reference_test),
    ("print_layout",       "Print Layout Test",               "Future",    is_print_layout),
    ("text_rendering",     "SP11/SP13: Text Rendering",       "SP11,SP13", has_visible_text),
    ("font_metrics",       "SP11: Font Metrics",              "SP11",      has_font_metrics),
    ("image_rendering",    "SP13: Image Rendering",           "SP13",      has_image_ref),
    ("css_containment",    "Future SP: CSS Containment",      "Future",    has_containment),
    ("gradient",           "SP13: Gradient Rendering",        "SP13",      has_gradient),
    ("margin_trim",        "Future SP: margin-trim",          "Future",    has_margin_trim),
    ("inline_block",       "SP11: Inline Block Layout",       "SP11",      has_inline_block),
    ("box_shadow",         "Future SP: Box Shadow",           "Future",    has_box_shadow),
    ("sticky_position",    "Future SP: Sticky Position",      "Future",    has_sticky_position),
    ("complex_border",     "Paint Quality: Complex Borders",  "Future",    has_complex_border_style),
    ("scrollbar_gutter",   "Future SP: Scrollbar Gutter",     "Future",    has_scrollbar_gutter),
]

# Category names used in wpt_mapping.csv (maps dependency key → category name)
CATEGORY_FOR_DEP = {
    "reference_test": "reference_test",
    "print_layout": "print_layout",
    "text_rendering": "needs_text",
    "font_metrics": "needs_font_metrics",
    "image_rendering": "needs_image",
    "css_containment": "needs_containment",
    "gradient": "needs_gradient",
    "margin_trim": "needs_margin_trim",
    "inline_block": "needs_inline_block",
    "box_shadow": "needs_box_shadow",
    "sticky_position": "needs_sticky",
    "complex_border": "needs_complex_border",
    "scrollbar_gutter": "needs_scrollbar_gutter",
}


def classify_dependencies(html: str, test_id: str = "") -> list[str]:
    """Return list of dependency keys that apply to this test's HTML.

    Multi-label: returns ALL matching dependencies, not just the first.
    """
    result = []
    for key, _label, _sp, detector in DEPENDENCY_DEFS:
        import inspect
        params = inspect.signature(detector).parameters
        if 'test_id' in params:
            if detector(html, test_id=test_id):
                result.append(key)
        else:
            if detector(html):
                result.append(key)
    return result


def classify_failure_categories(html: str, test_id: str = "") -> tuple[str, str]:
    """Classify a failing test's cross-SP dependencies for wpt_mapping.csv.

    Returns (failure_category, dependency) where category may be comma-separated.
    If no cross-SP dependencies detected, returns ("sp12_layout_bug", "").
    """
    deps = classify_dependencies(html, test_id=test_id)
    if deps:
        categories = [CATEGORY_FOR_DEP[d] for d in deps]
        labels = []
        for key in deps:
            for k, label, _sp, _det in DEPENDENCY_DEFS:
                if k == key:
                    labels.append(label)
                    break
        return ",".join(categories), "; ".join(labels)
    return "sp12_layout_bug", ""
