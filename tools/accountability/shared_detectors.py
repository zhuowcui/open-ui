"""Shared cross-SP dependency detectors.

Single source of truth for classifying WPT test HTML by its feature dependencies.
Used by both generate_wpt_mapping.py and generate_sp12_5_csv.py to ensure
consistent classification across all tracking artifacts.
"""

import re
from html.parser import HTMLParser


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


def has_positioned_inline_layout(html: str) -> bool:
    """Detect relative/fixed positioning through an inline ancestor.

    This combination needs inline-fragment containing-block propagation and,
    when a block descendant is present, block-in-inline splitting that keeps
    the positioned ancestor's coordinate space.
    """
    return bool(
        re.search(r"<span[\s>]", html, re.IGNORECASE)
        and re.search(r"position\s*:\s*(?:relative|absolute|fixed)", html, re.IGNORECASE)
        and re.search(r"<div[\s>]", html, re.IGNORECASE)
    )


def has_abspos_flex_static_position(html: str) -> bool:
    """Detect out-of-flow children whose static position comes from flex."""
    return bool(
        re.search(r"display\s*:\s*(?:inline-)?flex\b", html, re.IGNORECASE)
        and re.search(r"position\s*:\s*absolute\b", html, re.IGNORECASE)
    )


def has_float_descendant_of_inline(html: str) -> bool:
    """Detect floats nested beneath an inline ``span`` ancestor.

    Direct floated children are handled by block layout. A float encountered
    while flattening an inline subtree instead needs the inline collector to
    preserve an out-of-flow float placeholder and feed it back to the ancestor
    block formatting context.
    """
    if not re.search(r"float\s*:\s*(?:left|right)\b", html, re.IGNORECASE):
        return False

    float_classes: set[str] = set()
    for css in re.findall(r"<style[^>]*>(.*?)</style>", html, re.DOTALL | re.IGNORECASE):
        for selector, declarations in re.findall(r"([^{}]+)\{([^{}]*)\}", css):
            if re.search(r"float\s*:\s*(?:left|right)\b", declarations, re.IGNORECASE):
                float_classes.update(re.findall(r"\.([\w-]+)", selector))

    class InlineFloatParser(HTMLParser):
        def __init__(self):
            super().__init__()
            self.span_depth = 0
            self.found = False

        def handle_starttag(self, tag, attrs):
            attrs_d = dict(attrs)
            if self.span_depth > 0:
                inline_style = attrs_d.get("style", "")
                classes = set(attrs_d.get("class", "").split())
                if re.search(
                    r"float\s*:\s*(?:left|right)\b", inline_style, re.IGNORECASE
                ) or classes.intersection(float_classes):
                    self.found = True
            if tag.lower() == "span":
                self.span_depth += 1

        def handle_endtag(self, tag):
            if tag.lower() == "span" and self.span_depth > 0:
                self.span_depth -= 1

    parser = InlineFloatParser()
    parser.feed(html)
    return parser.found


def has_float_bfc_phantom_margin_separation(html: str) -> bool:
    """Detect the float/BFC margin-separation case with a phantom line.

    An empty inline between a nested float and a later new formatting context
    creates a zero-height phantom line. Margins may collapse through that line,
    but moving the BFC below the float must make the float non-adjoining first.
    """
    return bool(
        re.search(r"float\s*:\s*(?:left|right)\b", html, re.IGNORECASE)
        and re.search(r"overflow\s*:\s*(?:hidden|auto|scroll)\b", html, re.IGNORECASE)
        and re.search(r"margin-top\s*:\s*[^;\"']+", html, re.IGNORECASE)
        and re.search(r"<span\b[^>]*>\s*</span\s*>", html, re.IGNORECASE)
    )


def has_box_shadow(html: str) -> bool:
    """Detect box-shadow property (not yet implemented)."""
    return bool(re.search(r"box-shadow\s*:", html, re.IGNORECASE))


def has_sticky_position(html: str) -> bool:
    """Detect position:sticky (requires scroll container integration)."""
    return bool(re.search(r"position\s*:\s*sticky", html, re.IGNORECASE))


def has_complex_border_style(html: str) -> bool:
    """Detect border paint-quality cases that are not SP12 layout bugs."""
    complex_style = bool(re.search(
        r"(?:border(?:-(?:top|right|bottom|left))?-style|border)\s*:"
        r"[^;]*(?:dashed|dotted|double|groove|ridge|inset|outset)",
        html, re.IGNORECASE
    ))
    translucent_rounded_border = bool(
        re.search(r"border[^;]*rgba\s*\(", html, re.IGNORECASE)
        and re.search(r"border-radius\s*:", html, re.IGNORECASE)
    )
    return complex_style or translucent_rounded_border


def has_rounded_border_paint(html: str) -> bool:
    """Detect rounded-border rasterization as a distinct paint dependency.

    Solid rounded borders are not complex border-style cases, but their
    curved outer and inner edges still require Chromium-compatible coverage
    and compositing. Keep that dependency precise instead of folding it into
    a generic paint bucket.
    """
    return bool(re.search(
        r"border(?:-(?:top-left|top-right|bottom-right|bottom-left))?-radius\s*:",
        html,
        re.IGNORECASE,
    ))


def has_inline_box_decoration_break(html: str) -> bool:
    """Detect sliced/cloned decoration across inline fragments.

    Inline borders/backgrounds must be split or cloned at forced/soft line
    breaks. This needs fragment-aware inline decoration geometry rather than
    ordinary block border painting.
    """
    return bool(
        re.search(r"(?:-webkit-)?box-decoration-break\s*:", html, re.IGNORECASE)
        and re.search(r"<span[\s>]", html, re.IGNORECASE)
    )


def has_clearing_break_after_floats(html: str) -> bool:
    """Detect a clearing ``br`` whose line box must interact with floats."""
    if not re.search(r"float\s*:\s*(?:left|right)\b", html, re.IGNORECASE):
        return False
    css_clearing_break = re.search(
        r"br\s*\{[^}]*clear\s*:\s*(?:left|right|both)\b",
        html,
        re.DOTALL | re.IGNORECASE,
    )
    inline_clearing_break = re.search(
        r"<br\b[^>]*(?:clear\s*=|style\s*=\s*[^>]*clear\s*:)",
        html,
        re.IGNORECASE,
    )
    return bool(css_clearing_break or inline_clearing_break)


def has_display_contents_style_element(html: str) -> bool:
    """Detect author-visible style text produced by ``display:contents``."""
    return bool(re.search(
        r"<style[^>]*>.*?\*\s*\{[^}]*display\s*:\s*contents\b",
        html,
        re.DOTALL | re.IGNORECASE,
    ))


def has_display_contents_list_layout(html: str) -> bool:
    """Detect linked-CSS ``display:contents`` participation in list layout."""
    return bool(
        re.search(r"<link\b[^>]*rel\s*=\s*[\"']stylesheet[\"']", html, re.IGNORECASE)
        and re.search(r"<(?:ul|ol|li)[\s>]", html, re.IGNORECASE)
        and re.search(
            r"class\s*=\s*[\"'][^\"']*\bcontents\b",
            html,
            re.IGNORECASE,
        )
    )


def has_scrollbar_gutter(html: str) -> bool:
    """Detect scrollbar-gutter property (not implemented)."""
    return bool(re.search(r"scrollbar-gutter\s*:", html, re.IGNORECASE))


def has_javascript(html: str) -> bool:
    """Detect tests that require script execution or test harness behavior."""
    # Historical XHTML tests often retain disabled harness snippets inside
    # comments. They do not execute and therefore are not a JavaScript
    # dependency of the visual result.
    uncommented = re.sub(r"<!--.*?-->", "", html, flags=re.DOTALL)
    return bool(re.search(r"<script[\s>]", uncommented, re.IGNORECASE))


def has_grid_layout(html: str) -> bool:
    """Detect CSS Grid dependencies."""
    return bool(re.search(
        r"display\s*:\s*(?:inline-)?grid\b|grid(?:-[a-z-]+)?\s*:",
        html,
        re.IGNORECASE,
    ))


def has_table_layout(html: str) -> bool:
    """Detect table layout dependencies."""
    return bool(
        re.search(r"<(?:table|thead|tbody|tfoot|tr|td|th|caption)[\s>]", html, re.IGNORECASE)
        or re.search(
            r"display\s*:\s*(?:inline-)?table(?:-[a-z-]+)?\b|"
            r"(?:table-layout|border-collapse|border-spacing|caption-side)\s*:",
            html,
            re.IGNORECASE,
        )
    )


def has_writing_mode(html: str) -> bool:
    """Detect writing-mode / bidi coordinate-system dependencies."""
    return bool(re.search(r"(?:writing-mode|unicode-bidi|direction)\s*:", html, re.IGNORECASE))


def has_generated_content(html: str) -> bool:
    """Detect generated content and pseudo-element selectors."""
    return bool(
        re.search(r"::?(?:before|after|first-letter|first-line)\b", html, re.IGNORECASE)
        or re.search(r"content\s*:", html, re.IGNORECASE)
        or re.search(r"(?:counter-reset|counter-increment)\s*:", html, re.IGNORECASE)
    )


def has_advanced_selectors(html: str) -> bool:
    """Detect selector features beyond the simple porter rule subset."""
    # Only selector preludes inside CSS are relevant. Scanning the whole HTML
    # makes dotted author/help URLs look like compound class selectors.
    css = "\n".join(re.findall(
        r"<style[^>]*>(.*?)</style>", html, re.DOTALL | re.IGNORECASE
    ))
    selectors = "\n".join(re.findall(r"([^{}]+)\{", css))
    return bool(re.search(
        r"(?:^|[,{])[^{}]*(?:[#.][\w-]+){2,}|"
        r":(?:has|is|where|not|nth-|column|modal|popover)",
        selectors,
        re.IGNORECASE,
    ))


def has_visual_effects(html: str) -> bool:
    """Detect transform/filter/clip/mask/animation dependencies."""
    return bool(re.search(
        r"(?:transform|rotate|scale|translate|filter|clip-path|mask|animation|transition)\s*:",
        html,
        re.IGNORECASE,
    ))


def has_form_controls(html: str) -> bool:
    """Detect native form control layout/painting dependencies."""
    return bool(re.search(
        r"<(?:button|input|select|textarea|fieldset|legend|form|details|summary|dialog|audio|video)[\s>]",
        html,
        re.IGNORECASE,
    ))


def has_canvas_svg(html: str) -> bool:
    """Detect canvas/SVG rendering dependencies."""
    return bool(re.search(r"<(?:canvas|svg)[\s>]", html, re.IGNORECASE))


def has_line_clamp(html: str) -> bool:
    """Detect line-clamp / WebKit box line-clamp dependencies."""
    return bool(re.search(r"(?:-webkit-)?line-clamp\s*:|-webkit-box-orient\s*:", html, re.IGNORECASE))


def has_no_layout_content(html: str) -> bool:
    """Detect harness/crash tests with no visual DOM content to compare."""
    stripped = re.sub(r"<style[^>]*>.*?</style>", "", html, flags=re.DOTALL | re.IGNORECASE)
    stripped = re.sub(r"<script[^>]*>.*?</script>", "", stripped, flags=re.DOTALL | re.IGNORECASE)
    return not bool(re.search(r"<(?:div|span|p|section|article|main|body|table|img|canvas|svg|button|input)[\s>]", stripped, re.IGNORECASE))


def is_print_layout(html: str, test_id: str = "") -> bool:
    """Detect print-specific layout tests."""
    return "-print" in test_id.split("/")[-1]


def is_reference_test(html: str, test_id: str = "") -> bool:
    """Detect WPT reference files (-ref suffix) which are comparison targets, not standalone tests."""
    name = test_id.split("/")[-1] if test_id else ""
    return "-ref" in name


def is_fragmentation_area(html: str, test_id: str = "") -> bool:
    """Detect tests in SP13-owned fragmentation areas (css_break)."""
    return test_id.startswith("wpt/css_break/")


def is_multicol_area(html: str, test_id: str = "") -> bool:
    """Detect tests in SP13-owned multi-column layout area (css_multicol)."""
    return test_id.startswith("wpt/css_multicol/")


def reason_only_dependency(html: str) -> bool:
    """Dependencies attached from a concrete porter rejection, not HTML alone."""
    return False


def dependency_for_portability_reason(reason: str) -> str:
    """Map a deterministic porter rejection to its functional owner."""
    value = reason.lower()
    if value == "no_layout_content":
        return "root_body_layout"
    if "javascript" in value:
        return "javascript"
    if "line-clamp" in value or "-webkit-box-orient" in value:
        return "line_clamp"
    if "writing-mode" in value or "unicode-bidi" in value:
        return "writing_mode"
    if "margin-trim" in value:
        return "margin_trim"
    if "contain" in value or "container" in value:
        return "css_containment"
    if "grid" in value:
        return "grid_layout"
    if (
        "table" in value
        or "border-collapse" in value
        or "border-spacing" in value
        or "caption-side" in value
    ):
        return "table_layout"
    if any(
        token in value
        for token in (
            "transform",
            "filter",
            "clip-path",
            "shape-outside",
            "mask",
            "animation",
            "transition",
        )
    ):
        return "visual_effects"
    if any(tag in value for tag in ("<img>", "<iframe>", "<video>", "<object>", "<embed>")):
        return "image_rendering"
    if "<canvas>" in value or "<svg>" in value:
        return "canvas_svg"
    if any(
        tag in value
        for tag in (
            "<button>",
            "<input>",
            "<select>",
            "<textarea>",
            "<fieldset>",
            "<legend>",
            "<details>",
            "<form>",
            "<audio>",
            "-webkit-appearance",
        )
    ):
        return "form_controls"
    if any(
        token in value
        for token in (
            "::before",
            "::after",
            "::first-letter",
            "::first-line",
            "content",
            "counter-reset",
            "counter-increment",
        )
    ):
        return "generated_content"
    return "advanced_selectors"


# Ordered list of (key, label, owning_sp, detector)
# Detectors that need test_id have a special flag.
DEPENDENCY_DEFS = [
    ("reference_test",     "Reference Test (not standalone)", "N/A",       is_reference_test),
    ("print_layout",       "Print Layout Test",               "Future",    is_print_layout),
    ("font_metrics",       "SP11: Font Metrics",              "SP11",      has_font_metrics),
    ("image_rendering",    "SP13: Image Rendering",           "SP13",      has_image_ref),
    ("css_containment",    "Future SP: CSS Containment",      "Future",    has_containment),
    ("gradient",           "SP13: Gradient Rendering",        "SP13",      has_gradient),
    ("margin_trim",        "Future SP: margin-trim",          "Future",    has_margin_trim),
    ("inline_block",       "SP11: Inline Block Layout",       "SP11",      has_inline_block),
    ("positioned_inline_layout", "SP13: Positioned Inline Layout", "SP13", has_positioned_inline_layout),
    ("abspos_flex_static_position", "SP12: Abspos Flex Static Position", "SP12", has_abspos_flex_static_position),
    ("float_descendant_of_inline", "SP13: Float Descendant of Inline", "SP13", has_float_descendant_of_inline),
    ("float_bfc_phantom_margin_separation", "SP12: Float/BFC Phantom Margin Separation", "SP12", has_float_bfc_phantom_margin_separation),
    ("box_shadow",         "Future SP: Box Shadow",           "Future",    has_box_shadow),
    ("sticky_position",    "Future SP: Sticky Position",      "Future",    has_sticky_position),
    ("rounded_border_paint", "Paint Quality: Rounded Borders", "Future",   has_rounded_border_paint),
    ("inline_box_decoration_break", "SP15: Inline Box Decoration Break", "SP15", has_inline_box_decoration_break),
    ("clearing_break_after_floats", "SP15: Clearing Break After Floats", "SP15", has_clearing_break_after_floats),
    ("display_contents_style_element", "SP15: display:contents Style Element", "SP15", has_display_contents_style_element),
    ("display_contents_list_layout", "SP15: display:contents List Layout", "SP15", has_display_contents_list_layout),
    ("root_body_layout", "SP15: Root/Body Viewport Propagation", "SP15", reason_only_dependency),
    ("complex_border",     "Paint Quality: Complex Borders",  "Future",    has_complex_border_style),
    ("scrollbar_gutter",   "Future SP: Scrollbar Gutter",     "Future",    has_scrollbar_gutter),
    ("javascript",         "Future SP: JavaScript/Test Harness", "Future",  has_javascript),
    ("grid_layout",        "Future SP: CSS Grid Layout",       "Future",    has_grid_layout),
    ("table_layout",       "Future SP: Table Layout",          "Future",    has_table_layout),
    ("writing_mode",       "Future SP: Writing Modes/Bidi",    "Future",    has_writing_mode),
    ("generated_content",  "Future SP: Generated Content",     "Future",    has_generated_content),
    ("advanced_selectors", "Future SP: Advanced CSS Selectors", "Future",   has_advanced_selectors),
    ("visual_effects",     "Future SP: Transforms/Effects",    "Future",    has_visual_effects),
    ("form_controls",      "Future SP: Native Form Controls",  "Future",    has_form_controls),
    ("canvas_svg",         "Future SP: Canvas/SVG Rendering",  "Future",    has_canvas_svg),
    ("line_clamp",         "Future SP: Line Clamp",            "Future",    has_line_clamp),
    ("non_visual",         "N/A: Non-visual Harness/Crash Test", "N/A",     has_no_layout_content),
    ("fragmentation",      "SP13: Block Fragmentation",       "SP13",      is_fragmentation_area),
    ("multicol",           "SP13: Multi-Column Layout",       "SP13",      is_multicol_area),
]

# Category names used in wpt_mapping.csv (maps dependency key → category name)
CATEGORY_FOR_DEP = {
    "reference_test": "reference_test",
    "print_layout": "print_layout",
    "font_metrics": "needs_font_metrics",
    "image_rendering": "needs_image",
    "css_containment": "needs_containment",
    "gradient": "needs_gradient",
    "margin_trim": "needs_margin_trim",
    "inline_block": "needs_inline_block",
    "positioned_inline_layout": "needs_positioned_inline_layout",
    "abspos_flex_static_position": "needs_abspos_flex_static_position",
    "float_descendant_of_inline": "needs_float_descendant_of_inline",
    "float_bfc_phantom_margin_separation": "needs_float_bfc_phantom_margin_separation",
    "box_shadow": "needs_box_shadow",
    "sticky_position": "needs_sticky",
    "rounded_border_paint": "needs_rounded_border_paint",
    "inline_box_decoration_break": "needs_inline_box_decoration_break",
    "clearing_break_after_floats": "needs_clearing_break_after_floats",
    "display_contents_style_element": "needs_display_contents_style_element",
    "display_contents_list_layout": "needs_display_contents_list_layout",
    "root_body_layout": "needs_root_body_layout",
    "complex_border": "needs_complex_border",
    "scrollbar_gutter": "needs_scrollbar_gutter",
    "javascript": "needs_javascript",
    "grid_layout": "needs_grid",
    "table_layout": "needs_table_layout",
    "writing_mode": "needs_writing_mode",
    "generated_content": "needs_generated_content",
    "advanced_selectors": "needs_advanced_selectors",
    "visual_effects": "needs_visual_effects",
    "form_controls": "needs_form_controls",
    "canvas_svg": "needs_canvas_svg",
    "line_clamp": "needs_line_clamp",
    "non_visual": "non_visual_test",
    "fragmentation": "sp13_fragmentation",
    "multicol": "sp13_multicol",
}


def classify_dependencies(
    html: str, test_id: str = "", excluded: set[str] | None = None
) -> list[str]:
    """Return list of dependency keys that apply to this test's HTML.

    Multi-label: returns ALL matching dependencies, not just the first.
    """
    result = []
    excluded = excluded or set()
    for key, _label, _sp, detector in DEPENDENCY_DEFS:
        if key in excluded:
            continue
        import inspect
        params = inspect.signature(detector).parameters
        if 'test_id' in params:
            if detector(html, test_id=test_id):
                result.append(key)
        else:
            if detector(html):
                result.append(key)
    return result


def classify_failure_categories(
    html: str, test_id: str = "", excluded: set[str] | None = None
) -> tuple[str, str]:
    """Classify a failing test's cross-SP dependencies for wpt_mapping.csv.

    Returns (failure_category, dependency) where category may be comma-separated.
    If no cross-SP dependencies detected, returns ("sp12_layout_bug", "").
    """
    deps = classify_dependencies(html, test_id=test_id, excluded=excluded)
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
