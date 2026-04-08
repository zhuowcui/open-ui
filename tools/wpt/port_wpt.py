#!/usr/bin/env python3
"""
WPT Test Porter — Converts WPT HTML tests to Rust Document builders.

Usage:
  python3 tools/wpt/port_wpt.py <wpt_dir> <output_rust_file> [--filter-supported]

For each HTML file in wpt_dir:
  1. Parse the DOM structure
  2. Determine if it uses only supported CSS features
  3. If supported, generate a Rust fn that builds the equivalent Document
  4. Also generate the HTML template entry for Chrome rendering

Supports:
  - display: block, inline, inline-block, flow-root, none, flex
  - position: static, relative, absolute, fixed
  - float: left, right, none
  - clear: left, right, both, none
  - margin, padding, border (all sides, shorthand)
  - width, height, min/max variants
  - box-sizing, overflow
  - background-color, color
  - flex properties
  - opacity, visibility, z-index
  - top, right, bottom, left
  - line-height
"""

import os
import re
import sys
import csv
import json
from pathlib import Path
from html.parser import HTMLParser
from collections import OrderedDict


# ─── CSS property support map ──────────────────────────────────────────────
SUPPORTED_PROPERTIES = {
    # Display
    'display',
    # Position
    'position', 'top', 'right', 'bottom', 'left', 'z-index',
    'inset', 'inset-block', 'inset-inline',
    'inset-block-start', 'inset-block-end', 'inset-inline-start', 'inset-inline-end',
    # Float
    'float', 'clear',
    # Box model
    'margin', 'margin-top', 'margin-right', 'margin-bottom', 'margin-left',
    'margin-block', 'margin-block-start', 'margin-block-end',
    'margin-inline', 'margin-inline-start', 'margin-inline-end',
    'padding', 'padding-top', 'padding-right', 'padding-bottom', 'padding-left',
    'padding-block', 'padding-block-start', 'padding-block-end',
    'padding-inline', 'padding-inline-start', 'padding-inline-end',
    'border', 'border-top', 'border-right', 'border-bottom', 'border-left',
    'border-width', 'border-top-width', 'border-right-width', 'border-bottom-width', 'border-left-width',
    'border-style', 'border-top-style', 'border-right-style', 'border-bottom-style', 'border-left-style',
    'border-color', 'border-top-color', 'border-right-color', 'border-bottom-color', 'border-left-color',
    'border-block', 'border-block-start', 'border-block-end',
    'border-block-width', 'border-inline-width',
    'border-block-start-width', 'border-block-end-width',
    'border-inline-start-width', 'border-inline-end-width',
    'border-radius', 'border-top-left-radius', 'border-top-right-radius',
    'border-bottom-left-radius', 'border-bottom-right-radius',
    'box-sizing',
    # Sizing
    'width', 'height', 'min-width', 'max-width', 'min-height', 'max-height',
    # Overflow
    'overflow', 'overflow-x', 'overflow-y',
    # Visual
    'background', 'background-color', 'color', 'opacity', 'visibility',
    # Flex
    'flex', 'flex-direction', 'flex-wrap', 'flex-flow',
    'justify-content', 'align-items', 'align-content', 'align-self',
    'flex-grow', 'flex-shrink', 'flex-basis', 'order',
    'gap', 'row-gap', 'column-gap',
    # Multicol
    'columns', 'column-count', 'column-width', 'column-gap', 'column-rule',
    'column-rule-width', 'column-rule-style', 'column-rule-color',
    'column-span', 'column-fill',
    # Break
    'break-before', 'break-after', 'break-inside',
    'page-break-before', 'page-break-after', 'page-break-inside',
    'widows', 'orphans',
    'box-decoration-break',
    # Sizing - logical
    'block-size', 'inline-size', 'min-block-size', 'max-block-size',
    'min-inline-size', 'max-inline-size',
    # Text (basic)
    'line-height', 'vertical-align', 'text-align',
    # Aspect ratio
    'aspect-ratio',
    # Font (extract font-size)
    'font', 'font-size',
}

UNSUPPORTED_FEATURES = {
    # Grid layout — not implemented
    'grid', 'grid-template', 'grid-template-columns', 'grid-template-rows',
    'grid-column', 'grid-row', 'grid-area', 'grid-gap',
    # Writing modes — changes coordinate system fundamentally
    'writing-mode', 'direction', 'unicode-bidi',
    # Transforms & animation — out of scope
    'transform', 'rotate', 'scale', 'translate',
    'animation', 'transition', 'will-change',
    # Shape/mask/filter — out of scope
    'shape-outside', 'shape-margin', 'shape-image-threshold',
    'clip-path', 'mask', 'filter',
    # Table layout — not implemented
    'table-layout', 'caption-side', 'border-collapse', 'border-spacing',
    # Generated content — out of scope
    'counter-reset', 'counter-increment', 'content',
    # CSS containment — not implemented
    'contain', 'container', 'container-type', 'container-name',
    'contain-intrinsic-size',
    # Line clamp — requires text layout
    'line-clamp',
    # margin-trim — not implemented
    'margin-trim',
}

# Properties we can safely IGNORE (don't affect box layout geometry)
IGNORED_PROPERTIES = {
    'text-decoration', 'text-transform', 'text-indent', 'text-shadow',
    'font-family', 'font-weight', 'font-style',
    'font-variant', 'letter-spacing', 'word-spacing',
    'white-space', 'word-break', 'overflow-wrap', 'hyphens',
    'list-style', 'list-style-type', 'list-style-position',
    'cursor', 'pointer-events', 'user-select',
    'resize', 'outline', 'box-shadow', 'text-overflow',
    'vertical-align', 'line-height', 'visibility',
    'opacity', 'z-index', 'isolation',
    # Background details that don't affect layout
    'background-image', 'background-repeat', 'background-size',
    'background-position', 'background-clip', 'background-origin',
    'background-attachment',
    # Border image (visual only)
    'border-image', 'border-image-source', 'border-image-slice',
    'border-image-width', 'border-image-repeat',
    # Print/page
    'print-color-adjust', 'image-rendering',
    # Scroll
    'scrollbar-gutter', 'scrollbar-width',
    'overflow-clip-margin',
    # Ruby
    'ruby-position',
}

# CSS named colors → Rust Color constants
CSS_COLORS = {
    'red': 'Color::RED',
    'green': 'Color::from_rgba8(0, 128, 0, 255)',
    'lime': 'Color::from_rgba8(0, 255, 0, 255)',
    'blue': 'Color::BLUE',
    'yellow': 'Color::from_rgba8(255, 255, 0, 255)',
    'orange': 'Color::from_rgba8(255, 165, 0, 255)',
    'purple': 'Color::from_rgba8(128, 0, 128, 255)',
    'black': 'Color::BLACK',
    'white': 'Color::WHITE',
    'gray': 'Color::from_rgba8(128, 128, 128, 255)',
    'grey': 'Color::from_rgba8(128, 128, 128, 255)',
    'aqua': 'Color::from_rgba8(0, 255, 255, 255)',
    'cyan': 'Color::from_rgba8(0, 255, 255, 255)',
    'magenta': 'Color::from_rgba8(255, 0, 255, 255)',
    'fuchsia': 'Color::from_rgba8(255, 0, 255, 255)',
    'silver': 'Color::from_rgba8(192, 192, 192, 255)',
    'maroon': 'Color::from_rgba8(128, 0, 0, 255)',
    'olive': 'Color::from_rgba8(128, 128, 0, 255)',
    'navy': 'Color::from_rgba8(0, 0, 128, 255)',
    'teal': 'Color::from_rgba8(0, 128, 128, 255)',
    'transparent': 'Color::TRANSPARENT',
    'pink': 'Color::from_rgba8(255, 192, 203, 255)',
    'lightblue': 'Color::from_rgba8(173, 216, 230, 255)',
    'lightgreen': 'Color::from_rgba8(144, 238, 144, 255)',
    'darkgreen': 'Color::from_rgba8(0, 100, 0, 255)',
    'darkblue': 'Color::from_rgba8(0, 0, 139, 255)',
    'darkred': 'Color::from_rgba8(139, 0, 0, 255)',
}


def parse_color(value: str) -> str | None:
    """Convert a CSS color value to Rust Color expression."""
    value = value.strip().lower()
    if value in CSS_COLORS:
        return CSS_COLORS[value]
    # #RGB
    m = re.match(r'^#([0-9a-f]{3})$', value)
    if m:
        r, g, b = [int(c*2, 16) for c in m.group(1)]
        return f'Color::from_rgba8({r}, {g}, {b}, 255)'
    # #RRGGBB
    m = re.match(r'^#([0-9a-f]{6})$', value)
    if m:
        r = int(m.group(1)[0:2], 16)
        g = int(m.group(1)[2:4], 16)
        b = int(m.group(1)[4:6], 16)
        return f'Color::from_rgba8({r}, {g}, {b}, 255)'
    # #RRGGBBAA
    m = re.match(r'^#([0-9a-f]{8})$', value)
    if m:
        r = int(m.group(1)[0:2], 16)
        g = int(m.group(1)[2:4], 16)
        b = int(m.group(1)[4:6], 16)
        a = int(m.group(1)[6:8], 16)
        return f'Color::from_rgba8({r}, {g}, {b}, {a})'
    # rgb(r, g, b)
    m = re.match(r'^rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)$', value)
    if m:
        return f'Color::from_rgba8({m.group(1)}, {m.group(2)}, {m.group(3)}, 255)'
    # rgba(r, g, b, a)
    m = re.match(r'^rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([\d.]+)\s*\)$', value)
    if m:
        a = int(float(m.group(4)) * 255)
        return f'Color::from_rgba8({m.group(1)}, {m.group(2)}, {m.group(3)}, {a})'
    return None


def parse_length(value: str) -> str | None:
    """Convert a CSS length value to Rust Length expression."""
    value = value.strip()
    if value == '0' or value == '0px':
        return 'Length::px(0.0)'
    if value == 'auto':
        return 'Length::auto()'
    if value == 'none':
        return 'Length::none()'
    m = re.match(r'^(-?[\d.]+)px$', value)
    if m:
        return f'Length::px({float(m.group(1))})'
    m = re.match(r'^(-?[\d.]+)%$', value)
    if m:
        return f'Length::percent({float(m.group(1))})'
    # em → convert to px assuming 16px base font-size (our default)
    m = re.match(r'^(-?[\d.]+)em$', value)
    if m:
        px_val = float(m.group(1)) * 16.0
        return f'Length::px({px_val})'
    # rem → same as em for root element (base 16px)
    m = re.match(r'^(-?[\d.]+)rem$', value)
    if m:
        px_val = float(m.group(1)) * 16.0
        return f'Length::px({px_val})'
    # Keyword lengths
    if value == 'min-content':
        return 'Length::min_content()'
    if value == 'max-content':
        return 'Length::max_content()'
    if value == 'fit-content':
        return 'Length::fit_content()'
    if value == 'stretch' or value == '-webkit-fill-available':
        return 'Length::stretch()'
    # vw/vh — approximate as % of 800x600 viewport
    m = re.match(r'^(-?[\d.]+)vw$', value)
    if m:
        px_val = float(m.group(1)) * 8.0  # 800px viewport
        return f'Length::px({px_val})'
    m = re.match(r'^(-?[\d.]+)vh$', value)
    if m:
        px_val = float(m.group(1)) * 6.0  # 600px viewport
        return f'Length::px({px_val})'
    return None


def parse_border_width(value: str) -> str | None:
    """Convert a CSS border-width value to Rust i32 expression (pixels)."""
    value = value.strip()
    if value == '0' or value == '0px':
        return '0'
    m = re.match(r'^(-?[\d.]+)px$', value)
    if m:
        return str(int(float(m.group(1))))
    # Named widths
    mapping = {'thin': '1', 'medium': '3', 'thick': '5'}
    if value in mapping:
        return mapping[value]
    return None


def parse_inline_styles(style_str: str) -> dict:
    """Parse a CSS style string into property:value dict."""
    result = OrderedDict()
    if not style_str:
        return result
    for decl in style_str.split(';'):
        decl = decl.strip()
        if ':' not in decl:
            continue
        prop, val = decl.split(':', 1)
        result[prop.strip().lower()] = val.strip()
    return result


def check_supported(styles: dict) -> tuple[bool, str]:
    """Check if all CSS properties in styles are supported. Returns (supported, reason)."""
    for prop in styles:
        if prop in UNSUPPORTED_FEATURES:
            return False, f"unsupported property: {prop}"
        if prop in IGNORED_PROPERTIES:
            continue  # Safe to ignore — doesn't affect box layout
        if prop.startswith('grid') or prop.startswith('-webkit') or prop.startswith('-moz'):
            return False, f"vendor/unsupported prefix: {prop}"
    return True, ""


# ─── DOM tree parser ───────────────────────────────────────────────────────

class DomNode:
    """Represents a parsed DOM element."""
    def __init__(self, tag: str, attrs: dict, styles: dict):
        self.tag = tag
        self.attrs = attrs
        self.styles = styles
        self.children: list = []  # DomNode or str (text)
        self.is_text = False

    def __repr__(self):
        return f"<{self.tag} style={self.styles}>"


def parse_simple_css_rules(css_text: str) -> list:
    """Parse simple CSS rules from a <style> block.
    Returns list of (selector, styles_dict) tuples.
    Only handles: tag selectors, .class selectors, #id selectors, and combinations.
    """
    rules = []
    # Remove comments
    css_text = re.sub(r'/\*.*?\*/', '', css_text, flags=re.DOTALL)
    # Remove CDATA wrapper
    css_text = re.sub(r'<!\[CDATA\[|\]\]>', '', css_text)

    # Split into rule blocks
    blocks = re.findall(r'([^{]+)\{([^}]*)\}', css_text)
    for selector_text, declarations in blocks:
        selector_text = selector_text.strip()
        styles = parse_inline_styles(declarations)

        # Handle comma-separated selectors
        for sel in selector_text.split(','):
            sel = sel.strip()
            if sel:
                rules.append((sel, styles))

    return rules


def _match_simple_selector(selector: str, tag: str, classes: list, id_val: str) -> bool:
    """Check if a simple (non-compound) CSS selector matches an element."""
    parts = re.findall(r'[.#]?[a-zA-Z0-9_-]+|\*', selector)
    if not parts:
        return False
    for part in parts:
        if part == '*':
            continue
        elif part.startswith('.'):
            if part[1:] not in classes:
                return False
        elif part.startswith('#'):
            if part[1:] != id_val:
                return False
        else:
            if part.lower() != tag.lower():
                return False
    return True


def match_selector(selector: str, tag: str, classes: list, id_val: str,
                   ancestors: list = None) -> bool:
    """Check if a CSS selector matches an element.
    
    Supports simple selectors, descendant combinators (space), and child combinator (>).
    ancestors is a list of (tag, classes, id_val) tuples from outermost to innermost.
    """
    selector = selector.strip()
    if not selector:
        return False

    # Strip pseudo-classes for matching purposes
    selector = re.sub(r':(?:root|first-child|last-child|nth-child\([^)]+\)|only-child|empty|not\([^)]+\))', '', selector)
    selector = selector.strip()

    # After stripping, if selector is empty (e.g., bare ":root"), match everything
    if not selector:
        return True

    # Sibling combinators not supported
    if '+' in selector or '~' in selector:
        return False

    # Tokenize: split on child combinator and whitespace while preserving combinator type
    tokens = []
    combinators = []
    # Normalize whitespace around >
    normalized = re.sub(r'\s*>\s*', ' > ', selector).strip()
    parts = normalized.split()
    
    current_parts = []
    for p in parts:
        if p == '>':
            if current_parts:
                tokens.append(' '.join(current_parts))
                current_parts = []
            combinators.append('child')
        else:
            if current_parts:
                tokens.append(' '.join(current_parts))
                combinators.append('descendant')
                current_parts = []
            current_parts.append(p)
    if current_parts:
        tokens.append(' '.join(current_parts))

    if len(tokens) == 1:
        return _match_simple_selector(tokens[0], tag, classes, id_val)

    # Last token must match current element
    if not _match_simple_selector(tokens[-1], tag, classes, id_val):
        return False

    if not ancestors:
        return False

    # Walk ancestor list (innermost first) matching remaining tokens
    remaining_tokens = tokens[:-1]
    remaining_combinators = combinators[:]  # combinators[i] is between tokens[i] and tokens[i+1]
    ri = len(remaining_tokens) - 1
    
    for idx, (anc_tag, anc_classes, anc_id) in enumerate(reversed(ancestors)):
        if ri < 0:
            break
        if _match_simple_selector(remaining_tokens[ri], anc_tag, anc_classes, anc_id):
            ri -= 1
        elif ri < len(remaining_combinators) and remaining_combinators[ri] == 'child':
            # Child combinator requires IMMEDIATE parent match
            return False

    return ri < 0


def apply_css_rules(rules: list, node: 'DomNode', ancestors: list = None):
    """Apply CSS rules to a DOM node and its descendants (recursively).

    CSS cascade: later rules override earlier rules for the same property.
    Inline styles (already in node.styles) take highest precedence.
    """
    if node.is_text:
        return

    if ancestors is None:
        ancestors = []

    classes = node.attrs.get('class', '').split()
    id_val = node.attrs.get('id', '')

    # Save inline styles (highest precedence)
    inline_styles = OrderedDict(node.styles)

    # Apply stylesheet rules in order (later wins)
    cascade = OrderedDict()
    for selector, styles in rules:
        if match_selector(selector, node.tag, classes, id_val, ancestors):
            cascade.update(styles)

    # Inline styles override stylesheet rules
    cascade.update(inline_styles)
    node.styles = cascade

    child_ancestors = ancestors + [(node.tag, classes, id_val)]
    for child in node.children:
        apply_css_rules(rules, child, child_ancestors)


class WptHtmlParser(HTMLParser):
    """Parse WPT HTML into a DOM tree, extracting only body content."""

    SKIP_TAGS = {'head', 'link', 'meta', 'title', 'script', 'noscript'}
    LAYOUT_TAGS = {'div', 'span', 'p', 'section', 'article', 'main', 'header',
                   'footer', 'nav', 'aside', 'figure', 'figcaption', 'br',
                   'strong', 'em', 'b', 'i', 'u', 'a', 'img'}

    def __init__(self):
        super().__init__()
        self.root = DomNode('body', {}, {})
        self.stack = [self.root]
        self.in_body = False
        self.skip_depth = 0
        self.in_style = False
        self.style_content = ""
        self.has_script = False
        self.has_style_block = False
        self.css_rules = []  # Parsed CSS rules from <style>
        self.all_styles = []  # all style dicts encountered
        self.ref_path = None

    # Void elements that never have closing tags
    VOID_TAGS = {'link', 'meta', 'br', 'hr', 'img', 'input', 'col', 'area',
                 'base', 'embed', 'param', 'source', 'track', 'wbr'}

    def handle_starttag(self, tag, attrs):
        attrs_dict = dict(attrs)

        # Track reference
        if tag == 'link' and attrs_dict.get('rel') == 'match':
            self.ref_path = attrs_dict.get('href', '')

        if tag == 'script':
            self.has_script = True
            self.skip_depth += 1
            return

        if tag == 'style':
            self.has_style_block = True
            self.in_style = True
            self.style_content = ""
            return

        if tag in ('head', 'html', 'body'):
            if tag == 'body':
                self.in_body = True
                if 'style' in attrs_dict:
                    self.root.styles = parse_inline_styles(attrs_dict['style'])
            return

        # Skip void elements in SKIP_TAGS without incrementing depth
        if tag in self.SKIP_TAGS:
            if tag in self.VOID_TAGS:
                return  # Void: no closing tag, don't increment depth
            self.skip_depth += 1
            return

        if self.skip_depth > 0:
            if tag not in self.VOID_TAGS:
                self.skip_depth += 1
            return

        if not self.in_body:
            self.in_body = True

        styles = parse_inline_styles(attrs_dict.get('style', ''))
        self.all_styles.append(styles)

        node = DomNode(tag, attrs_dict, styles)
        self.stack[-1].children.append(node)
        if tag not in self.VOID_TAGS:
            self.stack.append(node)

    def handle_endtag(self, tag):
        if tag == 'style':
            self.in_style = False
            self.css_rules = parse_simple_css_rules(self.style_content)
            return
        if tag == 'script':
            if self.skip_depth > 0:
                self.skip_depth -= 1
            return
        if tag in ('head', 'html', 'body'):
            return
        if self.skip_depth > 0:
            self.skip_depth -= 1
            return
        if len(self.stack) > 1:
            self.stack.pop()

    def handle_data(self, data):
        if self.in_style:
            self.style_content += data
            return
        if self.skip_depth > 0:
            return
        text = data.strip()
        if text and self.in_body:
            node = DomNode('#text', {}, {})
            node.is_text = True
            node.text_content = text
            self.stack[-1].children.append(node)

    def handle_comment(self, data):
        pass

    def finalize(self):
        """Apply CSS rules to the DOM tree after parsing."""
        if self.css_rules:
            apply_css_rules(self.css_rules, self.root)
            # Re-collect all styles
            self.all_styles = []
            def collect(node):
                if not node.is_text:
                    self.all_styles.append(node.styles)
                    for c in node.children:
                        collect(c)
            collect(self.root)


def parse_wpt_html(html_path: str) -> WptHtmlParser:
    """Parse a WPT HTML file and return the parser with DOM tree."""
    with open(html_path, 'r', encoding='utf-8', errors='replace') as f:
        content = f.read()

    parser = WptHtmlParser()
    parser.feed(content)
    parser.finalize()
    return parser


# ─── Portability analysis ──────────────────────────────────────────────────

def analyze_portability(parser: WptHtmlParser) -> tuple[bool, str]:
    """Determine if a WPT test can be ported to our engine.
    Returns (portable, reason_if_not).
    """
    if parser.has_script:
        return False, "uses_javascript"

    # Pseudo-classes/pseudo-elements we can handle
    SAFE_PSEUDO_PATTERN = re.compile(
        r':(?:root|first-child|last-child|nth-child\([^)]+\)|only-child|empty|not\([^)]+\))'
    )

    # Check CSS rules from <style> blocks for unsupported properties
    if parser.has_style_block:
        for selector, styles in parser.css_rules:
            # Strip safe pseudo-classes before checking for unsupported ones
            stripped = SAFE_PSEUDO_PATTERN.sub('', selector)
            # After stripping safe pseudos, reject remaining pseudo-classes/elements
            if '::' in stripped:
                return False, f"complex_css_selector: {selector}"
            # Allow remaining ':' only if it was fully consumed by safe pattern
            remaining_colons = stripped.replace('::', '')
            if ':' in remaining_colons:
                return False, f"complex_css_selector: {selector}"
            # Reject sibling combinators (+, ~) but ALLOW child combinator (>)
            if '+' in stripped or '~' in stripped:
                return False, f"complex_css_selector: {selector}"
            supported, reason = check_supported(styles)
            if not supported:
                return False, f"style_block_{reason}"

    # Check all inline styles for unsupported properties
    for styles in parser.all_styles:
        supported, reason = check_supported(styles)
        if not supported:
            return False, reason

    # Check for display values we don't support
    all_style_dicts = list(parser.all_styles)
    for _, styles in parser.css_rules:
        all_style_dicts.append(styles)

    for styles in all_style_dicts:
        display = styles.get('display', '')
        if display in ('table', 'table-row', 'table-cell', 'table-column',
                        'table-row-group', 'table-column-group', 'table-header-group',
                        'table-footer-group', 'table-caption', 'grid', 'inline-grid',
                        'ruby', 'ruby-text'):
            return False, f"unsupported display: {display}"

    # Check for unsupported elements
    def check_tree(node):
        if node.is_text:
            return True, ""
        if node.tag in ('table', 'tr', 'td', 'th', 'thead', 'tbody', 'tfoot',
                        'caption', 'col', 'colgroup', 'img', 'svg', 'canvas',
                        'video', 'audio', 'iframe', 'object', 'embed',
                        'input', 'select', 'textarea', 'button', 'form',
                        'fieldset', 'legend', 'details', 'summary', 'dialog',
                        'template', 'slot'):
            return False, f"unsupported element: <{node.tag}>"
        for child in node.children:
            ok, reason = check_tree(child)
            if not ok:
                return False, reason
        return True, ""

    return check_tree(parser.root)


# ─── Rust code generation ─────────────────────────────────────────────────

def sanitize_fn_name(name: str) -> str:
    """Convert a filename to a valid Rust function name."""
    name = re.sub(r'[^a-zA-Z0-9]', '_', name)
    name = re.sub(r'_+', '_', name)
    name = name.strip('_').lower()
    if name[0:1].isdigit():
        name = 'test_' + name
    return name


def generate_style_code(styles: dict, var_name: str) -> list[str]:
    """Generate Rust code lines to set style properties on a node."""
    lines = []
    s = f"doc.node_mut({var_name}).style"

    for prop, val in styles.items():
        code = generate_single_style(prop, val, s)
        if code:
            lines.extend(code if isinstance(code, list) else [code])

    return lines


def generate_single_style(prop: str, val: str, s: str) -> list[str] | str | None:
    """Generate Rust code for a single CSS property:value."""
    val = val.strip().rstrip(';').strip()

    # ── display ──
    if prop == 'display':
        mapping = {
            'block': 'Display::Block',
            'inline': 'Display::Inline',
            'inline-block': 'Display::InlineBlock',
            'none': 'Display::None',
            'flow-root': 'Display::FlowRoot',
            'flex': 'Display::Flex',
            'inline-flex': 'Display::InlineFlex',
        }
        if val in mapping:
            return f"{s}.display = {mapping[val]};"

    # ── position ──
    if prop == 'position':
        mapping = {
            'static': 'Position::Static',
            'relative': 'Position::Relative',
            'absolute': 'Position::Absolute',
            'fixed': 'Position::Fixed',
            'sticky': 'Position::Sticky',
        }
        if val in mapping:
            return f"{s}.position = {mapping[val]};"

    # ── float ──
    if prop == 'float':
        mapping = {'left': 'Float::Left', 'right': 'Float::Right', 'none': 'Float::None'}
        if val in mapping:
            return f"{s}.float = {mapping[val]};"

    # ── clear ──
    if prop == 'clear':
        mapping = {
            'left': 'Clear::Left', 'right': 'Clear::Right',
            'both': 'Clear::Both', 'none': 'Clear::None',
        }
        if val in mapping:
            return f"{s}.clear = {mapping[val]};"

    # ── top/right/bottom/left ──
    if prop in ('top', 'right', 'bottom', 'left'):
        length = parse_length(val)
        if length:
            return f"{s}.{prop} = {length};"

    # ── z-index ──
    if prop == 'z-index':
        if val == 'auto':
            return f"{s}.z_index = None;"
        try:
            return f"{s}.z_index = Some({int(val)});"
        except ValueError:
            pass

    # ── width/height ──
    if prop in ('width', 'height', 'min-width', 'max-width', 'min-height', 'max-height'):
        length = parse_length(val)
        if length:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {length};"

    # ── margin shorthand ──
    if prop == 'margin':
        return generate_shorthand_4(val, s, 'margin')

    # ── margin sides ──
    if prop in ('margin-top', 'margin-right', 'margin-bottom', 'margin-left'):
        length = parse_length(val)
        if length:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {length};"

    # ── padding shorthand ──
    if prop == 'padding':
        return generate_shorthand_4(val, s, 'padding')

    # ── padding sides ──
    if prop in ('padding-top', 'padding-right', 'padding-bottom', 'padding-left'):
        length = parse_length(val)
        if length:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {length};"

    # ── border shorthand (e.g. "1px solid red") ──
    if prop == 'border':
        return generate_border_shorthand(val, s, ['top', 'right', 'bottom', 'left'])

    if prop in ('border-top', 'border-right', 'border-bottom', 'border-left'):
        side = prop.split('-')[1]
        return generate_border_shorthand(val, s, [side])

    # ── border-width shorthand ──
    if prop == 'border-width':
        return generate_border_width_shorthand(val, s)

    # ── border-width sides ──
    if prop in ('border-top-width', 'border-right-width', 'border-bottom-width', 'border-left-width'):
        px_val = parse_border_width(val)
        if px_val is not None:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {px_val};"

    # ── border-style shorthand ──
    if prop == 'border-style':
        return generate_border_style_shorthand(val, s)

    # ── border-style sides ──
    if prop in ('border-top-style', 'border-right-style', 'border-bottom-style', 'border-left-style'):
        style_code = border_style_to_rust(val)
        if style_code:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {style_code};"

    # ── border-color shorthand ──
    if prop == 'border-color':
        return generate_border_color_shorthand(val, s)

    # ── border-color sides ──
    if prop in ('border-top-color', 'border-right-color', 'border-bottom-color', 'border-left-color'):
        color = parse_color(val)
        if color:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = StyleColor::Resolved({color});"

    # ── border-radius ──
    if prop == 'border-radius':
        m = re.match(r'^(-?[\d.]+)(px|em|rem|%)$', val.strip())
        if m:
            num = float(m.group(1))
            unit = m.group(2)
            if unit == 'em' or unit == 'rem':
                num = num * 16.0
            elif unit == '%':
                pass  # stored as-is, layout resolves
            v = num
            return [
                f"{s}.border_top_left_radius = ({v}_f32, {v}_f32);",
                f"{s}.border_top_right_radius = ({v}_f32, {v}_f32);",
                f"{s}.border_bottom_left_radius = ({v}_f32, {v}_f32);",
                f"{s}.border_bottom_right_radius = ({v}_f32, {v}_f32);",
            ]

    if prop in ('border-top-left-radius', 'border-top-right-radius',
                'border-bottom-left-radius', 'border-bottom-right-radius'):
        m = re.match(r'^(-?[\d.]+)px$', val.strip())
        if m:
            v = float(m.group(1))
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = ({v}_f32, {v}_f32);"

    # ── box-sizing ──
    if prop == 'box-sizing':
        mapping = {'content-box': 'BoxSizing::ContentBox', 'border-box': 'BoxSizing::BorderBox'}
        if val in mapping:
            return f"{s}.box_sizing = {mapping[val]};"

    # ── overflow ──
    if prop == 'overflow':
        mapping = {
            'visible': 'Overflow::Visible', 'hidden': 'Overflow::Hidden',
            'scroll': 'Overflow::Scroll', 'auto': 'Overflow::Auto',
            'clip': 'Overflow::Clip',
        }
        if val in mapping:
            return [
                f"{s}.overflow_x = {mapping[val]};",
                f"{s}.overflow_y = {mapping[val]};",
            ]

    if prop in ('overflow-x', 'overflow-y'):
        mapping = {
            'visible': 'Overflow::Visible', 'hidden': 'Overflow::Hidden',
            'scroll': 'Overflow::Scroll', 'auto': 'Overflow::Auto',
            'clip': 'Overflow::Clip',
        }
        if val in mapping:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {mapping[val]};"

    # ── background-color ──
    if prop == 'background-color':
        color = parse_color(val)
        if color:
            return f"{s}.background_color = {color};"

    # ── background shorthand — extract color component ──
    if prop == 'background':
        # Try parsing entire value as color first (simplest case)
        color = parse_color(val)
        if color:
            return f"{s}.background_color = {color};"
        # Try extracting color from complex shorthand
        # background: <color> url(...) ... or <color> <other>
        parts = val.split()
        for part in parts:
            part = part.strip()
            if part.startswith('url(') or part.startswith('no-repeat') or part.startswith('repeat'):
                continue
            if '/' in part or part in ('top', 'left', 'right', 'bottom', 'center',
                                        'cover', 'contain', 'fixed', 'scroll', 'local',
                                        'no-repeat', 'repeat-x', 'repeat-y', 'repeat',
                                        'padding-box', 'border-box', 'content-box'):
                continue
            color = parse_color(part)
            if color:
                return f"{s}.background_color = {color};"

    # ── color ──
    if prop == 'color':
        color = parse_color(val)
        if color:
            return f"{s}.color = {color};"

    # ── opacity ──
    if prop == 'opacity':
        try:
            return f"{s}.opacity = {float(val)};"
        except ValueError:
            pass

    # ── visibility ──
    if prop == 'visibility':
        mapping = {'visible': 'Visibility::Visible', 'hidden': 'Visibility::Hidden'}
        if val in mapping:
            return f"{s}.visibility = {mapping[val]};"

    # ── flex properties ──
    if prop == 'flex-direction':
        mapping = {
            'row': 'FlexDirection::Row', 'row-reverse': 'FlexDirection::RowReverse',
            'column': 'FlexDirection::Column', 'column-reverse': 'FlexDirection::ColumnReverse',
        }
        if val in mapping:
            return f"{s}.flex_direction = {mapping[val]};"

    if prop == 'flex-wrap':
        mapping = {
            'nowrap': 'FlexWrap::Nowrap', 'wrap': 'FlexWrap::Wrap',
            'wrap-reverse': 'FlexWrap::WrapReverse',
        }
        if val in mapping:
            return f"{s}.flex_wrap = {mapping[val]};"

    if prop == 'justify-content':
        mapping = {
            'flex-start': 'ContentAlignment::new(ContentPosition::FlexStart)',
            'start': 'ContentAlignment::new(ContentPosition::FlexStart)',
            'flex-end': 'ContentAlignment::new(ContentPosition::FlexEnd)',
            'end': 'ContentAlignment::new(ContentPosition::FlexEnd)',
            'center': 'ContentAlignment::new(ContentPosition::Center)',
            'space-between': 'ContentAlignment::with_distribution(ContentDistribution::SpaceBetween)',
            'space-around': 'ContentAlignment::with_distribution(ContentDistribution::SpaceAround)',
            'space-evenly': 'ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly)',
        }
        if val in mapping:
            return f"{s}.justify_content = {mapping[val]};"

    if prop == 'align-items':
        mapping = {
            'flex-start': 'ItemAlignment::new(ItemPosition::FlexStart)',
            'start': 'ItemAlignment::new(ItemPosition::FlexStart)',
            'flex-end': 'ItemAlignment::new(ItemPosition::FlexEnd)',
            'end': 'ItemAlignment::new(ItemPosition::FlexEnd)',
            'center': 'ItemAlignment::new(ItemPosition::Center)',
            'stretch': 'ItemAlignment::new(ItemPosition::Stretch)',
            'baseline': 'ItemAlignment::new(ItemPosition::Baseline)',
            'self-start': 'ItemAlignment::new(ItemPosition::FlexStart)',
            'self-end': 'ItemAlignment::new(ItemPosition::FlexEnd)',
        }
        if val in mapping:
            return f"{s}.align_items = {mapping[val]};"

    if prop == 'align-self':
        mapping = {
            'auto': 'ItemAlignment::INITIAL_SELF',
            'flex-start': 'ItemAlignment::new(ItemPosition::FlexStart)',
            'start': 'ItemAlignment::new(ItemPosition::FlexStart)',
            'flex-end': 'ItemAlignment::new(ItemPosition::FlexEnd)',
            'end': 'ItemAlignment::new(ItemPosition::FlexEnd)',
            'center': 'ItemAlignment::new(ItemPosition::Center)',
            'stretch': 'ItemAlignment::new(ItemPosition::Stretch)',
            'baseline': 'ItemAlignment::new(ItemPosition::Baseline)',
            'self-start': 'ItemAlignment::new(ItemPosition::FlexStart)',
            'self-end': 'ItemAlignment::new(ItemPosition::FlexEnd)',
        }
        if val in mapping:
            return f"{s}.align_self = {mapping[val]};"

    if prop == 'align-content':
        mapping = {
            'flex-start': 'ContentAlignment::new(ContentPosition::FlexStart)',
            'start': 'ContentAlignment::new(ContentPosition::FlexStart)',
            'flex-end': 'ContentAlignment::new(ContentPosition::FlexEnd)',
            'end': 'ContentAlignment::new(ContentPosition::FlexEnd)',
            'center': 'ContentAlignment::new(ContentPosition::Center)',
            'stretch': 'ContentAlignment::with_distribution(ContentDistribution::Stretch)',
            'space-between': 'ContentAlignment::with_distribution(ContentDistribution::SpaceBetween)',
            'space-around': 'ContentAlignment::with_distribution(ContentDistribution::SpaceAround)',
            'space-evenly': 'ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly)',
        }
        if val in mapping:
            return f"{s}.align_content = {mapping[val]};"

    if prop in ('flex-grow', 'flex-shrink'):
        try:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {float(val)};"
        except ValueError:
            pass

    if prop == 'flex-basis':
        length = parse_length(val)
        if length:
            return f"{s}.flex_basis = {length};"

    if prop == 'order':
        try:
            return f"{s}.order = {int(val)};"
        except ValueError:
            pass

    if prop in ('gap', 'row-gap', 'column-gap'):
        length = parse_length(val)
        if length:
            if prop == 'gap':
                return [
                    f"{s}.row_gap = Some({length});",
                    f"{s}.column_gap = Some({length});",
                ]
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = Some({length});"

    # ── multicol ──
    if prop == 'column-count':
        if val == 'auto':
            return f"{s}.column_count = None;"
        try:
            v = int(val)
            if v < 1:
                return None  # Invalid: column-count must be >= 1
            return f"{s}.column_count = Some({v});"
        except ValueError:
            pass

    if prop == 'column-width':
        if val == 'auto':
            return f"{s}.column_width = None;"
        length = parse_length(val)
        if length:
            return f"{s}.column_width = Some({length});"

    if prop == 'column-fill':
        mapping = {'balance': 'ColumnFill::Balance', 'auto': 'ColumnFill::Auto'}
        if val in mapping:
            return f"{s}.column_fill = {mapping[val]};"

    if prop == 'column-span':
        mapping = {'none': 'ColumnSpan::None', 'all': 'ColumnSpan::All'}
        if val in mapping:
            return f"{s}.column_span = {mapping[val]};"

    # ── line-height ──
    if prop == 'line-height':
        if val == 'normal':
            return f"{s}.line_height = LineHeight::Normal;"
        # line-height: Length takes f32 (px value), not Length type
        m = re.match(r'^(-?[\d.]+)px$', val.strip())
        if m:
            return f"{s}.line_height = LineHeight::Length({float(m.group(1))});"
        m = re.match(r'^(-?[\d.]+)%$', val.strip())
        if m:
            return f"{s}.line_height = LineHeight::Percentage({float(m.group(1))});"
        try:
            return f"{s}.line_height = LineHeight::Number({float(val)});"
        except ValueError:
            pass

    # ── break ──
    if prop in ('break-before', 'break-after'):
        mapping = {
            'auto': 'BreakValue::Auto', 'avoid': 'BreakValue::Avoid',
            'avoid-page': 'BreakValue::AvoidPage', 'avoid-column': 'BreakValue::AvoidColumn',
            'column': 'BreakValue::Column', 'page': 'BreakValue::Page',
            'left': 'BreakValue::Left', 'right': 'BreakValue::Right',
            'always': 'BreakValue::Always',
        }
        if val in mapping:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {mapping[val]};"

    if prop == 'break-inside':
        mapping = {
            'auto': 'BreakInside::Auto', 'avoid': 'BreakInside::Avoid',
            'avoid-column': 'BreakInside::AvoidColumn',
        }
        if val in mapping:
            return f"{s}.break_inside = {mapping[val]};"

    # ── text-align ──
    if prop == 'text-align':
        mapping = {
            'left': 'TextAlign::Left', 'right': 'TextAlign::Right',
            'center': 'TextAlign::Center', 'justify': 'TextAlign::Justify',
        }
        if val in mapping:
            return f"{s}.text_align = {mapping[val]};"

    # ── vertical-align ──
    if prop == 'vertical-align':
        mapping = {
            'baseline': 'VerticalAlign::Baseline', 'top': 'VerticalAlign::Top',
            'bottom': 'VerticalAlign::Bottom', 'middle': 'VerticalAlign::Middle',
            'text-top': 'VerticalAlign::TextTop', 'text-bottom': 'VerticalAlign::TextBottom',
            'sub': 'VerticalAlign::Sub', 'super': 'VerticalAlign::Super',
        }
        if val in mapping:
            return f"{s}.vertical_align = {mapping[val]};"

    # ── columns shorthand (column-count + column-width) ──
    if prop == 'columns':
        parts = val.split()
        lines = []
        for part in parts:
            part = part.strip()
            if part == 'auto':
                continue
            m = re.match(r'^(\d+)$', part)
            if m:
                lines.append(f"{s}.column_count = Some({int(m.group(1))});")
                continue
            length = parse_length(part)
            if length:
                lines.append(f"{s}.column_width = Some({length});")
        return lines if lines else None

    # ── column-rule shorthand ──
    if prop == 'column-rule':
        parts = val.split()
        lines = []
        for part in parts:
            part = part.strip()
            if not part:
                continue
            bstyle = border_style_to_rust(part)
            if bstyle:
                lines.append(f"{s}.column_rule_style = {bstyle};")
            elif parse_border_width(part) is not None:
                lines.append(f"{s}.column_rule_width = {parse_border_width(part)};")
            elif parse_color(part):
                lines.append(f"{s}.column_rule_color = StyleColor::Resolved({parse_color(part)});")
        return lines if lines else None

    if prop == 'column-rule-width':
        px_val = parse_border_width(val)
        if px_val is not None:
            return f"{s}.column_rule_width = {px_val};"

    if prop == 'column-rule-style':
        style_code = border_style_to_rust(val)
        if style_code:
            return f"{s}.column_rule_style = {style_code};"

    if prop == 'column-rule-color':
        color = parse_color(val)
        if color:
            return f"{s}.column_rule_color = StyleColor::Resolved({color});"

    # ── logical properties: block-size/inline-size → height/width (horizontal writing mode) ──
    if prop in ('block-size', 'min-block-size', 'max-block-size'):
        length = parse_length(val)
        if length:
            physical = prop.replace('block-size', 'height').replace('-', '_')
            return f"{s}.{physical} = {length};"

    if prop in ('inline-size', 'min-inline-size', 'max-inline-size'):
        length = parse_length(val)
        if length:
            physical = prop.replace('inline-size', 'width').replace('-', '_')
            return f"{s}.{physical} = {length};"

    # ── inset (shorthand for top/right/bottom/left) ──
    if prop == 'inset':
        parts = val.split()
        props = ['top', 'right', 'bottom', 'left']
        if len(parts) == 1:
            length = parse_length(parts[0])
            if length:
                return [f"{s}.{p} = {length};" for p in props]
        elif len(parts) == 2:
            tb, lr = parse_length(parts[0]), parse_length(parts[1])
            if tb and lr:
                return [f"{s}.top = {tb};", f"{s}.right = {lr};", f"{s}.bottom = {tb};", f"{s}.left = {lr};"]
        elif len(parts) == 4:
            lengths = [parse_length(p) for p in parts]
            if all(lengths):
                return [f"{s}.{props[i]} = {lengths[i]};" for i in range(4)]

    # ── inset-block / inset-inline (logical shorthands) ──
    if prop == 'inset-block':
        length = parse_length(val)
        if length:
            return [f"{s}.top = {length};", f"{s}.bottom = {length};"]

    if prop == 'inset-inline':
        length = parse_length(val)
        if length:
            return [f"{s}.left = {length};", f"{s}.right = {length};"]

    # ── inset-block-start/end, inset-inline-start/end (individual logical inset) ──
    _inset_logical_map = {
        'inset-block-start': 'top', 'inset-block-end': 'bottom',
        'inset-inline-start': 'left', 'inset-inline-end': 'right',
    }
    if prop in _inset_logical_map:
        length = parse_length(val)
        if length:
            physical = _inset_logical_map[prop]
            return f"{s}.{physical} = {length};"

    # ── margin-block / margin-inline (logical margin shorthands) ──
    if prop == 'margin-block':
        parts = val.split()
        if len(parts) == 1:
            length = parse_length(parts[0]) if parts[0] != 'auto' else 'LengthPercentageAuto::Auto'
            if length:
                return [f"{s}.margin_top = {length};", f"{s}.margin_bottom = {length};"]
        elif len(parts) == 2:
            start = parse_length(parts[0]) if parts[0] != 'auto' else 'LengthPercentageAuto::Auto'
            end = parse_length(parts[1]) if parts[1] != 'auto' else 'LengthPercentageAuto::Auto'
            if start and end:
                return [f"{s}.margin_top = {start};", f"{s}.margin_bottom = {end};"]

    if prop == 'margin-inline':
        parts = val.split()
        if len(parts) == 1:
            length = parse_length(parts[0]) if parts[0] != 'auto' else 'LengthPercentageAuto::Auto'
            if length:
                return [f"{s}.margin_left = {length};", f"{s}.margin_right = {length};"]
        elif len(parts) == 2:
            start = parse_length(parts[0]) if parts[0] != 'auto' else 'LengthPercentageAuto::Auto'
            end = parse_length(parts[1]) if parts[1] != 'auto' else 'LengthPercentageAuto::Auto'
            if start and end:
                return [f"{s}.margin_left = {start};", f"{s}.margin_right = {end};"]

    # ── margin-block-start/end, margin-inline-start/end (individual logical margins) ──
    _margin_logical_map = {
        'margin-block-start': 'margin_top', 'margin-block-end': 'margin_bottom',
        'margin-inline-start': 'margin_left', 'margin-inline-end': 'margin_right',
    }
    if prop in _margin_logical_map:
        physical = _margin_logical_map[prop]
        if val.strip() == 'auto':
            return f"{s}.{physical} = LengthPercentageAuto::Auto;"
        length = parse_length(val)
        if length:
            return f"{s}.{physical} = {length};"

    # ── padding-block / padding-inline (logical padding shorthands) ──
    if prop == 'padding-block':
        parts = val.split()
        if len(parts) == 1:
            length = parse_length(parts[0])
            if length:
                return [f"{s}.padding_top = {length};", f"{s}.padding_bottom = {length};"]
        elif len(parts) == 2:
            start, end = parse_length(parts[0]), parse_length(parts[1])
            if start and end:
                return [f"{s}.padding_top = {start};", f"{s}.padding_bottom = {end};"]

    if prop == 'padding-inline':
        parts = val.split()
        if len(parts) == 1:
            length = parse_length(parts[0])
            if length:
                return [f"{s}.padding_left = {length};", f"{s}.padding_right = {length};"]
        elif len(parts) == 2:
            start, end = parse_length(parts[0]), parse_length(parts[1])
            if start and end:
                return [f"{s}.padding_left = {start};", f"{s}.padding_right = {end};"]

    # ── padding-block-start/end, padding-inline-start/end ──
    _padding_logical_map = {
        'padding-block-start': 'padding_top', 'padding-block-end': 'padding_bottom',
        'padding-inline-start': 'padding_left', 'padding-inline-end': 'padding_right',
    }
    if prop in _padding_logical_map:
        length = parse_length(val)
        if length:
            physical = _padding_logical_map[prop]
            return f"{s}.{physical} = {length};"

    # ── border-block / border-inline (logical border shorthands) ──
    if prop in ('border-block', 'border-block-start', 'border-block-end'):
        sides = {'border-block': ['top', 'bottom'],
                 'border-block-start': ['top'], 'border-block-end': ['bottom']}[prop]
        lines = []
        for part in val.split():
            bw = parse_border_width(part)
            if bw:
                for side in sides:
                    lines.append(f"{s}.border_{side}_width = {bw};")
            else:
                bstyle = border_style_to_rust(part)
                if bstyle:
                    for side in sides:
                        lines.append(f"{s}.border_{side}_style = {bstyle};")
                else:
                    bc = parse_color(part)
                    if bc:
                        for side in sides:
                            lines.append(f"{s}.border_{side}_color = StyleColor::Resolved({bc});")
        if lines:
            return lines

    # ── border-block-*-width, border-inline-*-width (individual logical border widths) ──
    _border_width_logical_map = {
        'border-block-width': ['border_top_width', 'border_bottom_width'],
        'border-inline-width': ['border_left_width', 'border_right_width'],
        'border-block-start-width': ['border_top_width'],
        'border-block-end-width': ['border_bottom_width'],
        'border-inline-start-width': ['border_left_width'],
        'border-inline-end-width': ['border_right_width'],
    }
    if prop in _border_width_logical_map:
        bw = parse_border_width(val.strip())
        if bw:
            return [f"{s}.{p} = {bw};" for p in _border_width_logical_map[prop]]

    # ── font shorthand (extract font-size) ──
    if prop == 'font':
        # font: <size>/<line-height> <family> or <size> <family> etc.
        m = re.match(r'(?:(?:normal|italic|oblique|bold|bolder|lighter|\d{3})\s+)*'
                     r'(-?[\d.]+)(px|em|rem)(?:\s*/\s*[\d.]+(?:px|em|rem|%)?)?', val.strip())
        if m:
            size_val = float(m.group(1))
            unit = m.group(2)
            if unit == 'px':
                return f"{s}.font_size = {size_val};"
            elif unit in ('em', 'rem'):
                return f"{s}.font_size = {size_val * 16.0};"

    # ── font-size ──
    if prop == 'font-size':
        v = val.strip()
        m = re.match(r'^(-?[\d.]+)px$', v)
        if m:
            return f"{s}.font_size = {float(m.group(1))};"
        m = re.match(r'^(-?[\d.]+)(em|rem)$', v)
        if m:
            return f"{s}.font_size = {float(m.group(1)) * 16.0};"
        m = re.match(r'^(-?[\d.]+)pt$', v)
        if m:
            return f"{s}.font_size = {float(m.group(1)) * 4.0 / 3.0};"

    # ── aspect-ratio ──
    if prop == 'aspect-ratio':
        if val.strip() == 'auto':
            return f"{s}.aspect_ratio = None;"
        # "16 / 9" or "16/9" or "2"
        m = re.match(r'^([\d.]+)\s*/\s*([\d.]+)$', val.strip())
        if m:
            w, h = float(m.group(1)), float(m.group(2))
            return f"{s}.aspect_ratio = Some(AspectRatio {{ ratio: ({w}_f32, {h}_f32), auto_flag: false }});"
        # "auto 16 / 9"
        m = re.match(r'^auto\s+([\d.]+)\s*/\s*([\d.]+)$', val.strip())
        if m:
            w, h = float(m.group(1)), float(m.group(2))
            return f"{s}.aspect_ratio = Some(AspectRatio {{ ratio: ({w}_f32, {h}_f32), auto_flag: true }});"
        try:
            r = float(val.strip())
            return f"{s}.aspect_ratio = Some(AspectRatio {{ ratio: ({r}_f32, 1.0_f32), auto_flag: false }});"
        except ValueError:
            pass

    # ── contain — not yet implemented in our style system ──

    # ── margin-block / margin-inline (logical) ──
    if prop == 'margin-block':
        length = parse_length(val)
        if length:
            return [f"{s}.margin_top = {length};", f"{s}.margin_bottom = {length};"]

    if prop == 'margin-inline':
        length = parse_length(val)
        if length:
            return [f"{s}.margin_left = {length};", f"{s}.margin_right = {length};"]

    # ── padding-block / padding-inline (logical) ──
    if prop == 'padding-block':
        length = parse_length(val)
        if length:
            return [f"{s}.padding_top = {length};", f"{s}.padding_bottom = {length};"]

    if prop == 'padding-inline':
        length = parse_length(val)
        if length:
            return [f"{s}.padding_left = {length};", f"{s}.padding_right = {length};"]

    # ── widows / orphans ──
    if prop in ('widows', 'orphans'):
        try:
            return f"{s}.{prop} = {int(val)}_u32;"
        except ValueError:
            pass

    # ── page-break-before / page-break-after (legacy) ──
    if prop in ('page-break-before', 'page-break-after'):
        mapping = {
            'auto': 'BreakValue::Auto', 'always': 'BreakValue::Page',
            'avoid': 'BreakValue::Avoid',
        }
        if val in mapping:
            # Map page-break-* to break-*
            side = prop.replace('page-break-', '')
            return f"{s}.break_{side} = {mapping[val]};"

    # ── page-break-inside (legacy) ──
    if prop == 'page-break-inside':
        mapping = {'auto': 'BreakInside::Auto', 'avoid': 'BreakInside::Avoid'}
        if val in mapping:
            return f"{s}.break_inside = {mapping[val]};"

    # ── box-decoration-break ──
    if prop == 'box-decoration-break':
        mapping = {'slice': 'BoxDecorationBreak::Slice', 'clone': 'BoxDecorationBreak::Clone'}
        if val.strip() in mapping:
            return f"{s}.box_decoration_break = {mapping[val.strip()]};"

    # ── flex shorthand ──
    if prop == 'flex':
        parts = val.split()
        if len(parts) == 1:
            if val == 'none':
                return [f"{s}.flex_grow = 0.0;", f"{s}.flex_shrink = 0.0;"]
            if val == 'auto':
                return [f"{s}.flex_grow = 1.0;", f"{s}.flex_shrink = 1.0;", f"{s}.flex_basis = Length::auto();"]
            if val == 'initial':
                # flex: initial = flex: 0 1 auto (CSS default)
                return [f"{s}.flex_grow = 0.0;", f"{s}.flex_shrink = 1.0;", f"{s}.flex_basis = Length::auto();"]
            try:
                g = float(val)
                return [f"{s}.flex_grow = {g};", f"{s}.flex_shrink = 1.0;", f"{s}.flex_basis = Length::px(0.0);"]
            except ValueError:
                pass
        elif len(parts) == 2:
            # flex: <grow> <shrink> | <grow> <basis>
            try:
                g = float(parts[0])
                # Try second as shrink factor
                try:
                    sh = float(parts[1])
                    return [f"{s}.flex_grow = {g};", f"{s}.flex_shrink = {sh};", f"{s}.flex_basis = Length::px(0.0);"]
                except ValueError:
                    # Second is basis
                    basis = parse_length(parts[1])
                    if basis:
                        return [f"{s}.flex_grow = {g};", f"{s}.flex_shrink = 1.0;", f"{s}.flex_basis = {basis};"]
            except ValueError:
                pass
        elif len(parts) == 3:
            # flex: <grow> <shrink> <basis>
            try:
                g = float(parts[0])
                sh = float(parts[1])
                basis = parse_length(parts[2])
                if basis:
                    return [f"{s}.flex_grow = {g};", f"{s}.flex_shrink = {sh};", f"{s}.flex_basis = {basis};"]
            except ValueError:
                pass

    # ── flex-flow shorthand ──
    if prop == 'flex-flow':
        lines = []
        for part in val.split():
            part = part.strip()
            dir_map = {'row': 'FlexDirection::Row', 'row-reverse': 'FlexDirection::RowReverse',
                       'column': 'FlexDirection::Column', 'column-reverse': 'FlexDirection::ColumnReverse'}
            wrap_map = {'nowrap': 'FlexWrap::Nowrap', 'wrap': 'FlexWrap::Wrap',
                        'wrap-reverse': 'FlexWrap::WrapReverse'}
            if part in dir_map:
                lines.append(f"{s}.flex_direction = {dir_map[part]};")
            elif part in wrap_map:
                lines.append(f"{s}.flex_wrap = {wrap_map[part]};")
        return lines if lines else None

    return None


def generate_shorthand_4(val: str, s: str, prefix: str, suffix: str = '') -> list[str] | None:
    """Generate 4-side shorthand (margin, padding, border-width)."""
    parts = val.split()
    if len(parts) == 1:
        length = parse_length(parts[0])
        if length:
            sides = ['top', 'right', 'bottom', 'left']
            return [f"{s}.{prefix}_{side}{suffix} = {length};" for side in sides]
    elif len(parts) == 2:
        tb = parse_length(parts[0])
        lr = parse_length(parts[1])
        if tb and lr:
            return [
                f"{s}.{prefix}_top{suffix} = {tb};",
                f"{s}.{prefix}_right{suffix} = {lr};",
                f"{s}.{prefix}_bottom{suffix} = {tb};",
                f"{s}.{prefix}_left{suffix} = {lr};",
            ]
    elif len(parts) == 3:
        top = parse_length(parts[0])
        lr = parse_length(parts[1])
        bot = parse_length(parts[2])
        if top and lr and bot:
            return [
                f"{s}.{prefix}_top{suffix} = {top};",
                f"{s}.{prefix}_right{suffix} = {lr};",
                f"{s}.{prefix}_bottom{suffix} = {bot};",
                f"{s}.{prefix}_left{suffix} = {lr};",
            ]
    elif len(parts) == 4:
        lengths = [parse_length(p) for p in parts]
        if all(lengths):
            sides = ['top', 'right', 'bottom', 'left']
            return [f"{s}.{prefix}_{side}{suffix} = {lengths[i]};" for i, side in enumerate(sides)]
    return None


def border_style_to_rust(val: str) -> str | None:
    """Convert CSS border-style value to Rust."""
    mapping = {
        'none': 'BorderStyle::None',
        'solid': 'BorderStyle::Solid',
        'dashed': 'BorderStyle::Dashed',
        'dotted': 'BorderStyle::Dotted',
        'double': 'BorderStyle::Double',
        'groove': 'BorderStyle::Groove',
        'ridge': 'BorderStyle::Ridge',
        'inset': 'BorderStyle::Inset',
        'outset': 'BorderStyle::Outset',
        'hidden': 'BorderStyle::Hidden',
    }
    return mapping.get(val.strip())


def generate_border_shorthand(val: str, s: str, sides: list) -> list[str] | None:
    """Parse 'border: 1px solid red' shorthand."""
    parts = val.split()
    width = None
    style = None
    color = None

    for part in parts:
        part = part.strip()
        if not part:
            continue
        if border_style_to_rust(part):
            style = border_style_to_rust(part)
        elif parse_border_width(part) is not None:
            width = parse_border_width(part)
        elif parse_color(part):
            color = parse_color(part)

    lines = []
    for side in sides:
        if width is not None:
            lines.append(f"{s}.border_{side}_width = {width};")
        if style:
            lines.append(f"{s}.border_{side}_style = {style};")
        if color:
            lines.append(f"{s}.border_{side}_color = StyleColor::Resolved({color});")
    return lines if lines else None


def generate_border_width_shorthand(val: str, s: str) -> list[str] | None:
    """Parse 'border-width: 1px 2px 3px 4px' shorthand to i32."""
    parts = val.split()
    if len(parts) == 1:
        w = parse_border_width(parts[0])
        if w is not None:
            return [f"{s}.border_{side}_width = {w};" for side in ['top', 'right', 'bottom', 'left']]
    elif len(parts) == 2:
        tb = parse_border_width(parts[0])
        lr = parse_border_width(parts[1])
        if tb is not None and lr is not None:
            return [
                f"{s}.border_top_width = {tb};",
                f"{s}.border_right_width = {lr};",
                f"{s}.border_bottom_width = {tb};",
                f"{s}.border_left_width = {lr};",
            ]
    elif len(parts) == 4:
        ws = [parse_border_width(p) for p in parts]
        if all(w is not None for w in ws):
            sides = ['top', 'right', 'bottom', 'left']
            return [f"{s}.border_{side}_width = {ws[i]};" for i, side in enumerate(sides)]
    return None


def generate_border_style_shorthand(val: str, s: str) -> list[str] | None:
    """Parse 'border-style: solid dashed' shorthand."""
    parts = val.split()
    if len(parts) == 1:
        code = border_style_to_rust(parts[0])
        if code:
            return [f"{s}.border_{side}_style = {code};" for side in ['top', 'right', 'bottom', 'left']]
    return None


def generate_border_color_shorthand(val: str, s: str) -> list[str] | None:
    """Parse 'border-color: red blue green yellow' shorthand."""
    parts = val.split()
    if len(parts) == 1:
        color = parse_color(parts[0])
        if color:
            return [f"{s}.border_{side}_color = StyleColor::Resolved({color});" for side in ['top', 'right', 'bottom', 'left']]
    elif len(parts) == 4:
        colors = [parse_color(p) for p in parts]
        if all(colors):
            sides = ['top', 'right', 'bottom', 'left']
            return [f"{s}.border_{side}_color = StyleColor::Resolved({colors[i]});" for i, side in enumerate(sides)]
    return None


# ─── Document builder generation ──────────────────────────────────────────

def generate_rust_fn(fn_name: str, root: DomNode) -> str:
    """Generate a Rust function that builds a Document matching the DOM tree."""
    lines = []
    lines.append(f"fn {fn_name}() -> Document {{")
    lines.append("    let (mut doc, vp) = base_doc();")

    counter = [0]

    def gen_node(node: DomNode, parent_var: str, indent: int):
        if node.is_text:
            # Skip text nodes — we're comparing layout only
            return

        if node.tag in ('p', 'strong', 'em', 'b', 'i', 'u', 'a'):
            # Skip instructional text paragraphs
            # Check if this is a layout-significant element (has styles)
            if not node.styles and node.tag == 'p':
                return
            if node.tag in ('strong', 'em', 'b', 'i', 'u', 'a') and not node.styles:
                return

        if node.tag == 'br':
            return

        counter[0] += 1
        var = f"n{counter[0]}"
        ws = "    " * indent

        # Map HTML tag to ElementTag
        inline_tags = {'span', 'a', 'em', 'strong', 'b', 'i', 'u', 'small', 'big', 'sub', 'sup', 'abbr', 'cite', 'code', 'mark', 'q', 's', 'del', 'ins', 'var', 'kbd', 'samp'}
        if node.tag in inline_tags:
            element_tag = "ElementTag::Span"
        else:
            element_tag = "ElementTag::Div"
        lines.append(f"{ws}let {var} = doc.create_node({element_tag});")

        # Set display:block for block-level HTML elements (our engine defaults to inline)
        block_tags = {'div', 'p', 'section', 'article', 'header', 'footer', 'nav', 'main',
                      'aside', 'figure', 'figcaption', 'blockquote', 'pre', 'address',
                      'details', 'summary', 'fieldset', 'form', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
                      'dl', 'dt', 'dd', 'ol', 'ul', 'li', 'hr', 'table'}
        if node.tag in block_tags and 'display' not in node.styles:
            lines.append(f"{ws}doc.node_mut({var}).style.display = Display::Block;")

        # Generate style code
        style_lines = generate_style_code(node.styles, var)
        for sl in style_lines:
            lines.append(f"{ws}{sl}")

        lines.append(f"{ws}doc.append_child({parent_var}, {var});")

        # Process children
        for child in node.children:
            gen_node(child, var, indent + 1)

    # Process body children
    # Apply body-level styles if any
    if root.styles:
        body_styles = generate_style_code(root.styles, 'vp')
        for sl in body_styles:
            lines.append(f"    {sl}")

    for child in root.children:
        gen_node(child, 'vp', 1)

    lines.append("    doc")
    lines.append("}")

    return '\n'.join(lines)


def generate_html_template(html_path: str) -> str:
    """Read an HTML file and extract body content + style blocks for Chrome rendering.
    The template must include <style> blocks so Chrome applies the same CSS rules
    that the Rust code generator parsed and encoded into Document builder code.
    """
    with open(html_path, 'r', encoding='utf-8', errors='replace') as f:
        content = f.read()

    # Extract <style> blocks from anywhere in the document (head or body)
    style_blocks = re.findall(r'<style[^>]*>.*?</style>', content, re.DOTALL | re.IGNORECASE)
    style_prefix = '\n'.join(style_blocks)

    # Extract body content
    body_match = re.search(r'<body[^>]*>(.*?)</body>', content, re.DOTALL | re.IGNORECASE)
    if body_match:
        body = body_match.group(1)
    else:
        # No explicit body — use content after meta/link tags
        body = content
        # Remove DOCTYPE, html, head, meta, link, title, script tags (NOT style)
        body = re.sub(r'<!DOCTYPE[^>]*>', '', body, flags=re.IGNORECASE)
        body = re.sub(r'<html[^>]*>|</html>', '', body, flags=re.IGNORECASE)
        # Remove <head> but preserve <style> blocks (already extracted above)
        body = re.sub(r'<head[^>]*>.*?</head>', '', body, flags=re.DOTALL | re.IGNORECASE)
        body = re.sub(r'<link[^>]*>', '', body, flags=re.IGNORECASE)
        body = re.sub(r'<meta[^>]*>', '', body, flags=re.IGNORECASE)
        body = re.sub(r'<title[^>]*>.*?</title>', '', body, flags=re.DOTALL | re.IGNORECASE)
        body = re.sub(r'<script[^>]*>.*?</script>', '', body, flags=re.DOTALL | re.IGNORECASE)
        # Remove style blocks from body (they're already in style_prefix)
        body = re.sub(r'<style[^>]*>.*?</style>', '', body, flags=re.DOTALL | re.IGNORECASE)

    # Strip instructional <p> tags (contain "Test passes if")
    body = re.sub(r'<p[^>]*>.*?Test passes.*?</p>', '', body, flags=re.DOTALL | re.IGNORECASE)

    # Combine: style blocks first, then body content
    template = style_prefix + '\n' + body.strip() if style_prefix else body.strip()
    return template


# ─── Batch processing ─────────────────────────────────────────────────────

def process_directory(wpt_dir: str, prefix: str = "wpt") -> dict:
    """Process all HTML files in a WPT directory.
    Returns dict with results per file.
    """
    results = {
        'portable': [],      # (filename, fn_name, rust_code, html_template)
        'not_portable': [],   # (filename, reason)
        'errors': [],         # (filename, error_msg)
    }

    html_files = sorted(Path(wpt_dir).glob('*.html'))
    print(f"Found {len(html_files)} HTML files in {wpt_dir}")

    for html_path in html_files:
        filename = html_path.stem
        try:
            parser = parse_wpt_html(str(html_path))
            portable, reason = analyze_portability(parser)

            if not portable:
                results['not_portable'].append((filename, reason))
                continue

            # Check if DOM tree has any layout children
            # Skip only <p> that contains "Test passes if" instruction text
            layout_children = []
            for c in parser.root.children:
                if c.is_text:
                    continue
                if c.tag == 'p' and not c.styles:
                    # Check if it's instruction text
                    has_test_text = False
                    for sub in c.children:
                        if sub.is_text and 'test passes' in getattr(sub, 'text_content', '').lower():
                            has_test_text = True
                    if has_test_text:
                        continue
                if c.tag == 'br':
                    continue
                layout_children.append(c)

            if not layout_children:
                results['not_portable'].append((filename, "no_layout_content"))
                continue

            fn_name = f"{prefix}_{sanitize_fn_name(filename)}"
            rust_code = generate_rust_fn(fn_name, parser.root)
            html_template = generate_html_template(str(html_path))

            results['portable'].append((filename, fn_name, rust_code, html_template))

        except Exception as e:
            results['errors'].append((filename, str(e)))

    return results


def write_rust_module(results: dict, output_path: str, module_name: str):
    """Write a Rust module file with all portable test builders."""
    lines = []
    lines.append(f"//! WPT tests: {module_name}")
    lines.append(f"//! Auto-generated by tools/wpt/port_wpt.py")
    lines.append(f"//! DO NOT EDIT MANUALLY")
    lines.append("")
    lines.append("use openui_dom::{Document, ElementTag, NodeId};")
    lines.append("use openui_geometry::Length;")
    lines.append("use openui_style::*;")
    lines.append("")
    lines.append("use crate::base_doc;")
    lines.append("")

    # Write test builder functions
    for filename, fn_name, rust_code, _ in results['portable']:
        lines.append(f"// Source: {filename}.html")
        lines.append(rust_code)
        lines.append("")

    # Write registry function
    lines.append(f"pub fn {module_name}_registry() -> Vec<(&'static str, fn() -> Document)> {{")
    lines.append("    vec![")
    for filename, fn_name, _, _ in results['portable']:
        test_id = f"wpt/{module_name}/{filename}"
        lines.append(f'        ("{test_id}", {fn_name} as fn() -> Document),')
    lines.append("    ]")
    lines.append("}")

    with open(output_path, 'w') as f:
        f.write('\n'.join(lines) + '\n')

    print(f"Wrote {len(results['portable'])} test builders to {output_path}")


def write_html_templates(results: dict, output_path: str, module_name: str):
    """Write HTML templates JSON for the pipeline."""
    templates = {}
    for filename, fn_name, _, html_template in results['portable']:
        test_id = f"wpt/{module_name}/{filename}"
        templates[test_id] = html_template

    with open(output_path, 'w') as f:
        json.dump(templates, f, indent=2)

    print(f"Wrote {len(templates)} HTML templates to {output_path}")


def write_report(results: dict, output_path: str):
    """Write a CSV report of porting results."""
    with open(output_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['filename', 'status', 'fn_name', 'reason'])

        for filename, fn_name, _, _ in results['portable']:
            writer.writerow([filename, 'ported', fn_name, ''])

        for filename, reason in results['not_portable']:
            writer.writerow([filename, 'not_portable', '', reason])

        for filename, error in results['errors']:
            writer.writerow([filename, 'error', '', error])

    total = len(results['portable']) + len(results['not_portable']) + len(results['errors'])
    print(f"Report: {len(results['portable'])}/{total} portable, "
          f"{len(results['not_portable'])} not portable, "
          f"{len(results['errors'])} errors")


# ─── Main ─────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 3:
        print("Usage: python3 tools/wpt/port_wpt.py <wpt_dir> <module_name> [--output-dir <dir>]")
        print("")
        print("Example:")
        print("  python3 tools/wpt/port_wpt.py ~/chromium/src/.../CSS2/floats/ css2_floats")
        sys.exit(1)

    wpt_dir = sys.argv[1]
    module_name = sys.argv[2]
    output_dir = "."

    if '--output-dir' in sys.argv:
        idx = sys.argv.index('--output-dir')
        output_dir = sys.argv[idx + 1]

    os.makedirs(output_dir, exist_ok=True)

    results = process_directory(wpt_dir, prefix=module_name)

    # Write outputs
    rust_path = os.path.join(output_dir, f"wpt_{module_name}.rs")
    html_path = os.path.join(output_dir, f"wpt_{module_name}_templates.json")
    report_path = os.path.join(output_dir, f"wpt_{module_name}_report.csv")

    write_rust_module(results, rust_path, module_name)
    write_html_templates(results, html_path, module_name)
    write_report(results, report_path)

    # Summary
    print(f"\n{'='*60}")
    print(f"WPT Porting Summary: {module_name}")
    print(f"{'='*60}")
    print(f"  Total files:    {len(results['portable']) + len(results['not_portable']) + len(results['errors'])}")
    print(f"  Ported:         {len(results['portable'])}")
    print(f"  Not portable:   {len(results['not_portable'])}")
    print(f"  Errors:         {len(results['errors'])}")

    if results['not_portable']:
        # Group by reason
        reasons = {}
        for _, reason in results['not_portable']:
            reasons[reason] = reasons.get(reason, 0) + 1
        print(f"\n  Not-portable breakdown:")
        for reason, count in sorted(reasons.items(), key=lambda x: -x[1]):
            print(f"    {reason}: {count}")


if __name__ == '__main__':
    main()
