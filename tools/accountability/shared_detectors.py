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
    """
    stripped = strip_style_blocks(html)
    # Strip HTML comments before checking for text
    stripped = re.sub(r"<!--.*?-->", "", stripped, flags=re.DOTALL)
    for m in re.finditer(r">([^<]+)<", stripped):
        text = m.group(1).strip()
        if text and not text.isspace() and text != "{":
            if re.search(r"[A-Za-z0-9]", text):
                return True
    return False


def has_font_metrics(html: str) -> bool:
    """Detect dependency on font metrics: ch, ex units or font-dependent line-height.

    ch and ex genuinely depend on the font's glyph metrics.
    line-height only depends on font metrics when set to 'normal' or a unitless
    number (e.g., '1.5'). Absolute values like '20px' or '0' do NOT depend on
    font metrics.
    """
    # ch/ex units always depend on font glyph metrics
    if re.search(r"[\d.]+(?:ch|ex)\b", html, re.IGNORECASE):
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


# Ordered list of (key, label, owning_sp, detector)
DEPENDENCY_DEFS = [
    ("text_rendering",  "SP11/SP13: Text Rendering",  "SP11,SP13", has_visible_text),
    ("font_metrics",    "SP11: Font Metrics",          "SP11",      has_font_metrics),
    ("image_rendering", "SP13: Image Rendering",       "SP13",      has_image_ref),
    ("css_containment", "Future SP: CSS Containment",  "Future",    has_containment),
    ("gradient",        "SP13: Gradient Rendering",    "SP13",      has_gradient),
    ("margin_trim",     "Future SP: margin-trim",      "Future",    has_margin_trim),
]

# Category names used in wpt_mapping.csv (maps dependency key → category name)
CATEGORY_FOR_DEP = {
    "text_rendering": "needs_text",
    "font_metrics": "needs_font_metrics",
    "image_rendering": "needs_image",
    "css_containment": "needs_containment",
    "gradient": "needs_gradient",
    "margin_trim": "needs_margin_trim",
}


def classify_dependencies(html: str) -> list[str]:
    """Return list of dependency keys that apply to this test's HTML.

    Multi-label: returns ALL matching dependencies, not just the first.
    """
    return [key for key, _label, _sp, detector in DEPENDENCY_DEFS if detector(html)]


def classify_failure_categories(html: str) -> tuple[str, str]:
    """Classify a failing test's cross-SP dependencies for wpt_mapping.csv.

    Returns (failure_category, dependency) where category may be comma-separated.
    If no cross-SP dependencies detected, returns ("sp12_layout_bug", "").
    """
    deps = classify_dependencies(html)
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
