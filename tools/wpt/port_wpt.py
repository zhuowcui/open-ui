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
import copy
import colorsys
import base64
import hashlib
import html as html_module
import math
import mimetypes
import urllib.parse
from enum import Enum
from pathlib import Path
from html.parser import HTMLParser
from collections import OrderedDict

try:
    # Package import used by the repository's unittest modules.
    from tools.wpt.engine_fixture_codegen import engineify_module
except ModuleNotFoundError:
    # Direct-script execution puts tools/wpt, rather than the repository root,
    # on sys.path.
    from engine_fixture_codegen import engineify_module


class PorterProfile(Enum):
    """Explicit generation environments with intentionally isolated output."""

    LEGACY_BOX_ONLY = "legacy-box-only"
    DETERMINISTIC_AHEM = "deterministic-ahem"
    REAL_FONT = "real-font"


ACTIVE_PORTER_PROFILE = PorterProfile.LEGACY_BOX_ONLY
EMIT_PAINT_LAYERS = False
MODERN_LINE_CLAMP_ENABLED = True

PROJECT_ROOT = Path(__file__).resolve().parents[2]
WPT_SOURCE_ROOT = Path(os.environ.get(
    'CHROMIUM_WPT_ROOT',
    os.path.expanduser('~/chromium/src/third_party/blink/web_tests/external/wpt'),
)).resolve()
SP20_ASSET_DIR = PROJECT_ROOT / 'tools' / 'accountability' / 'data' / 'wpt_assets' / 'sp20'
MEDIA_FIRST_FRAME_MANIFEST = (
    PROJECT_ROOT / 'docs' / 'renderer' / 'generated' / 'media-first-frames-v1.json'
)
JAVASCRIPT_MUTATION_AUDIT = (
    PROJECT_ROOT / 'docs' / 'renderer' / 'generated' / 'javascript-mutation-audit-v2.json'
)
_ACTIVE_RESOURCE_BASE: Path | None = None
_MUTATION_AUDIT_BY_TEST_ID: dict[str, dict] | None = None


def _load_media_first_frames() -> dict[str, dict]:
    manifest = json.loads(MEDIA_FIRST_FRAME_MANIFEST.read_text(encoding='utf-8'))
    if manifest.get('schema_version') != 1:
        raise ValueError('unsupported media first-frame manifest schema')
    fixtures = manifest.get('fixtures', [])
    if manifest.get('fixture_count') != len(fixtures):
        raise ValueError('media first-frame fixture count does not match')
    by_source_hash = {}
    for fixture in fixtures:
        transport = bytes.fromhex(fixture['transport_hex'])
        if hashlib.sha256(transport).hexdigest() != fixture['output_sha256']:
            raise ValueError('media first-frame output hash does not match')
        width, height = fixture['coded_size']
        expected_header = (
            b'OUIR' + bytes((1, 0, 0, 0))
            + int(width).to_bytes(4, 'little')
            + int(height).to_bytes(4, 'little')
        )
        if (
            not transport.startswith(expected_header)
            or len(transport) != 16 + int(width) * int(height) * 4
            or fixture['output_byte_length'] != len(transport)
        ):
            raise ValueError('invalid media first-frame RGBA8 transport')
        source_hash = fixture['source_sha256']
        if source_hash in by_source_hash:
            raise ValueError('duplicate media first-frame source hash')
        by_source_hash[source_hash] = fixture
    return by_source_hash


MEDIA_FIRST_FRAMES = _load_media_first_frames()


def set_porter_profile(profile: PorterProfile, *, retain_text: bool | None = None) -> None:
    """Select one porter profile without changing historical defaults.

    The SP16 real-font profile may retain or strip text per manifest membership,
    so its text switch is explicit at each transaction boundary.
    """
    global ACTIVE_PORTER_PROFILE, EMIT_TEXT_NODES, RETAIN_TEXT
    ACTIVE_PORTER_PROFILE = profile
    if profile is PorterProfile.LEGACY_BOX_ONLY:
        EMIT_TEXT_NODES = False
        RETAIN_TEXT = False
    elif profile is PorterProfile.DETERMINISTIC_AHEM:
        EMIT_TEXT_NODES = True
        RETAIN_TEXT = True
    else:
        keep = bool(retain_text)
        EMIT_TEXT_NODES = keep
        RETAIN_TEXT = keep


def is_real_font_profile() -> bool:
    return ACTIVE_PORTER_PROFILE is PorterProfile.REAL_FONT


def set_paint_layer_emission(enabled: bool) -> None:
    """Opt into the SP13-P production image/layer metadata.

    The default remains false so every historical full-directory generation
    retains byte-identical legacy output.
    """
    global EMIT_PAINT_LAYERS
    EMIT_PAINT_LAYERS = bool(enabled)


def set_modern_line_clamp_enabled(enabled: bool) -> None:
    """Mirror Chromium's runtime-gated modern line-clamp grammar.

    Legacy ``-webkit-line-clamp`` remains available independently.  The WPT
    splice driver scopes this switch to the same frozen feature-profile
    manifest used by the Chromium comparison runner.
    """
    global MODERN_LINE_CLAMP_ENABLED
    MODERN_LINE_CLAMP_ENABLED = bool(enabled)


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
    'border-inline', 'border-inline-start', 'border-inline-end',
    'border-block-width', 'border-inline-width',
    'border-block-style', 'border-inline-style',
    'border-block-color', 'border-inline-color',
    'border-block-start-width', 'border-block-end-width',
    'border-inline-start-width', 'border-inline-end-width',
    'border-block-start-style', 'border-block-end-style',
    'border-inline-start-style', 'border-inline-end-style',
    'border-block-start-color', 'border-block-end-color',
    'border-inline-start-color', 'border-inline-end-color',
    'border-start-start-radius', 'border-start-end-radius',
    'border-end-start-radius', 'border-end-end-radius',
    'border-radius', 'border-top-left-radius', 'border-top-right-radius',
    'border-bottom-left-radius', 'border-bottom-right-radius',
    'box-sizing',
    # Sizing
    'width', 'height', 'min-width', 'max-width', 'min-height', 'max-height',
    # Overflow
    'overflow', 'overflow-x', 'overflow-y', 'overflow-clip-margin', 'scrollbar-color',
    'resize', 'scrollbar-width', 'scrollbar-gutter',
    # Visual
    'background', 'background-color', 'background-image', 'background-repeat',
    'background-size', 'background-position', 'background-origin',
    'background-clip', 'background-attachment', 'color', 'opacity', 'visibility',
    'mask-image', 'mask-repeat', 'mask-size', 'mask-position',
    '-webkit-mask-image', '-webkit-mask-repeat', '-webkit-mask-size',
    '-webkit-mask-position',
    'zoom',
    # Flex
    'flex', 'flex-direction', 'flex-wrap', 'flex-flow',
    'justify-content', 'align-items', 'align-content', 'align-self',
    'flex-grow', 'flex-shrink', 'flex-basis', 'order',
    'gap', 'row-gap', 'column-gap',
    # Tables
    'table-layout', 'caption-side', 'border-collapse', 'border-spacing',
    'empty-cells',
    # Grid
    'grid', 'grid-template', 'grid-template-columns', 'grid-template-rows',
    'grid-template-areas', 'grid-auto-columns', 'grid-auto-rows',
    'grid-auto-flow', 'grid-column', 'grid-column-start', 'grid-column-end',
    'grid-row', 'grid-row-start', 'grid-row-end', 'grid-area', 'grid-gap',
    'place-items', 'place-content', 'place-self', 'justify-items',
    'justify-self',
    # Containment and static container queries
    'contain', 'content-visibility', 'contain-intrinsic-size',
    'contain-intrinsic-width', 'contain-intrinsic-height',
    'contain-intrinsic-inline-size', 'contain-intrinsic-block-size',
    'container', 'container-type', 'container-name', 'margin-trim',
    'scroll-marker-group', 'scroll-target-group',
    'scroll-snap-align', 'scroll-snap-type', 'scroll-snap-stop',
    # Replaced content and deterministic effects
    'object-fit', 'object-position', 'transform', 'transform-origin',
    'filter',
    'appearance', '-webkit-appearance', '-moz-appearance',
    '-webkit-box-align', '-webkit-box-pack',
    'shape-outside', 'shape-margin', 'shape-image-threshold',
    'animation', 'animation-name', 'animation-duration', 'animation-delay',
    'animation-fill-mode', 'animation-timing-function',
    # Multicol
    'columns', 'column-count', 'column-width', 'column-height', 'column-gap', 'column-rule',
    'column-rule-width', 'column-rule-style', 'column-rule-color', 'column-wrap',
    'column-span', 'column-fill',
    # Break
    'break-before', 'break-after', 'break-inside',
    'page-break-before', 'page-break-after', 'page-break-inside',
    'widows', 'orphans',
    'box-decoration-break',
    # Sizing - logical
    'block-size', 'inline-size', 'min-block-size', 'max-block-size',
    'min-inline-size', 'max-inline-size',
    # Writing modes and bidi
    'writing-mode', 'direction', 'unicode-bidi', 'text-orientation',
    'text-combine-upright',
    # Text (basic)
    'line-height', 'vertical-align', 'text-align', 'white-space', 'text-wrap',
    'ruby-position', 'text-overflow', 'text-shadow',
    'text-emphasis-style', 'text-emphasis-position', 'text-emphasis-color',
    # Generated content and line clamping (SP18)
    'content', 'counter-reset', 'counter-set', 'counter-increment', 'quotes',
    'line-clamp', 'block-ellipsis', '-webkit-line-clamp',
    '-webkit-box-orient',
    # Aspect ratio
    'aspect-ratio',
    # Font (extract font-size)
    'font', 'font-size',
    # Outline
    'outline', 'outline-style', 'outline-width', 'outline-color', 'outline-offset',
    # No-op properties (safe to accept, no visual effect or default-only)
    'will-change',
    # Visual-only properties that don't affect layout
    'box-shadow', 'isolation',
    'border-image', 'border-image-source', 'border-image-slice',
    'border-image-width', 'border-image-outset', 'border-image-repeat',
}

UNSUPPORTED_FEATURES = {
    # 3D/individual transforms and live transitions remain outside SP19.
    'rotate', 'scale', 'transition',
    # General path/mask syntax stays unsupported. ``clip-path: inset()`` is
    # accepted by the SP20 typed parser below.
    'mask',
}

# Properties we can safely IGNORE (don't affect box layout geometry)
IGNORED_PROPERTIES = {
    'text-decoration', 'text-transform', 'text-indent',
    'font-family', 'font-weight', 'font-style',
    'font-variant', 'font-kerning', 'font-feature-settings',
    'letter-spacing', 'word-spacing',
    'word-break', 'overflow-wrap', 'line-break', 'hyphens',
    'list-style', 'list-style-type', 'list-style-position',
    'cursor', 'pointer-events', 'user-select',
    # `scroll-snap-stop` only changes traversal through intermediate snap
    # positions; static snap geometry is emitted separately.
    'scroll-snap-stop',
    # Print/page
    'print-color-adjust', 'image-rendering', 'size',
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
    'hotpink': 'Color::from_rgba8(255, 105, 180, 255)',
    'mediumaquamarine': 'Color::from_rgba8(102, 205, 170, 255)',
    'limegreen': 'Color::from_rgba8(50, 205, 50, 255)',
    'turquoise': 'Color::from_rgba8(64, 224, 208, 255)',
    'coral': 'Color::from_rgba8(255, 127, 80, 255)',
    'violet': 'Color::from_rgba8(238, 130, 238, 255)',
    'bisque': 'Color::from_rgba8(255, 228, 196, 255)',
    'dodgerblue': 'Color::from_rgba8(30, 144, 255, 255)',
    'blueviolet': 'Color::from_rgba8(138, 43, 226, 255)',
    'papayawhip': 'Color::from_rgba8(255, 239, 213, 255)',
    'tomato': 'Color::from_rgba8(255, 99, 71, 255)',
    'skyblue': 'Color::from_rgba8(135, 206, 235, 255)',
    'gold': 'Color::from_rgba8(255, 215, 0, 255)',
    'lightgray': 'Color::from_rgba8(211, 211, 211, 255)',
    'lightgrey': 'Color::from_rgba8(211, 211, 211, 255)',
    'darkgray': 'Color::from_rgba8(169, 169, 169, 255)',
    'darkgrey': 'Color::from_rgba8(169, 169, 169, 255)',
    'indianred': 'Color::from_rgba8(205, 92, 92, 255)',
    'chocolate': 'Color::from_rgba8(210, 105, 30, 255)',
    'brown': 'Color::from_rgba8(165, 42, 42, 255)',
    'khaki': 'Color::from_rgba8(240, 230, 140, 255)',
    'crimson': 'Color::from_rgba8(220, 20, 60, 255)',
    'salmon': 'Color::from_rgba8(250, 128, 114, 255)',
    'deepskyblue': 'Color::from_rgba8(0, 191, 255, 255)',
    'royalblue': 'Color::from_rgba8(65, 105, 225, 255)',
    'steelblue': 'Color::from_rgba8(70, 130, 180, 255)',
    'tan': 'Color::from_rgba8(210, 180, 140, 255)',
    'wheat': 'Color::from_rgba8(245, 222, 179, 255)',
    'plum': 'Color::from_rgba8(221, 160, 221, 255)',
    'orchid': 'Color::from_rgba8(218, 112, 214, 255)',
    'mediumpurple': 'Color::from_rgba8(147, 112, 219, 255)',
    'slateblue': 'Color::from_rgba8(106, 90, 205, 255)',
    'cadetblue': 'Color::from_rgba8(95, 158, 160, 255)',
    'springgreen': 'Color::from_rgba8(0, 255, 127, 255)',
    'mediumseagreen': 'Color::from_rgba8(60, 179, 113, 255)',
    'seagreen': 'Color::from_rgba8(46, 139, 87, 255)',
    'forestgreen': 'Color::from_rgba8(34, 139, 34, 255)',
    'olivedrab': 'Color::from_rgba8(107, 142, 35, 255)',
    'darkorange': 'Color::from_rgba8(255, 140, 0, 255)',
    'orangered': 'Color::from_rgba8(255, 69, 0, 255)',
    'sienna': 'Color::from_rgba8(160, 82, 45, 255)',
    'peru': 'Color::from_rgba8(205, 133, 63, 255)',
    'goldenrod': 'Color::from_rgba8(218, 165, 32, 255)',
    'cornflowerblue': 'Color::from_rgba8(100, 149, 237, 255)',
    'midnightblue': 'Color::from_rgba8(25, 25, 112, 255)',
    'slategray': 'Color::from_rgba8(112, 128, 144, 255)',
    'slategrey': 'Color::from_rgba8(112, 128, 144, 255)',
    'dimgray': 'Color::from_rgba8(105, 105, 105, 255)',
    'dimgrey': 'Color::from_rgba8(105, 105, 105, 255)',
    'whitesmoke': 'Color::from_rgba8(245, 245, 245, 255)',
    'ivory': 'Color::from_rgba8(255, 255, 240, 255)',
    'beige': 'Color::from_rgba8(245, 245, 220, 255)',
    'linen': 'Color::from_rgba8(250, 240, 230, 255)',
    'cornsilk': 'Color::from_rgba8(255, 248, 220, 255)',
    'antiquewhite': 'Color::from_rgba8(250, 235, 215, 255)',
    'lavender': 'Color::from_rgba8(230, 230, 250, 255)',
    'mistyrose': 'Color::from_rgba8(255, 228, 225, 255)',
    'peachpuff': 'Color::from_rgba8(255, 218, 185, 255)',
    'moccasin': 'Color::from_rgba8(255, 228, 181, 255)',
    'navajowhite': 'Color::from_rgba8(255, 222, 173, 255)',
    'powderblue': 'Color::from_rgba8(176, 224, 230, 255)',
    'lightyellow': 'Color::from_rgba8(255, 255, 224, 255)',
    'lightcyan': 'Color::from_rgba8(224, 255, 255, 255)',
    'lightsalmon': 'Color::from_rgba8(255, 160, 122, 255)',
    'lightcoral': 'Color::from_rgba8(240, 128, 128, 255)',
    'lightpink': 'Color::from_rgba8(255, 182, 193, 255)',
    'lightseagreen': 'Color::from_rgba8(32, 178, 170, 255)',
    'lightskyblue': 'Color::from_rgba8(135, 206, 250, 255)',
    'lightsteelblue': 'Color::from_rgba8(176, 196, 222, 255)',
    'lightslategray': 'Color::from_rgba8(119, 136, 153, 255)',
    'lightslategrey': 'Color::from_rgba8(119, 136, 153, 255)',
    'mediumblue': 'Color::from_rgba8(0, 0, 205, 255)',
    'mediumorchid': 'Color::from_rgba8(186, 85, 211, 255)',
    'mediumslateblue': 'Color::from_rgba8(123, 104, 238, 255)',
    'mediumspringgreen': 'Color::from_rgba8(0, 250, 154, 255)',
    'mediumturquoise': 'Color::from_rgba8(72, 209, 204, 255)',
    'mediumvioletred': 'Color::from_rgba8(199, 21, 133, 255)',
    'darkcyan': 'Color::from_rgba8(0, 139, 139, 255)',
    'darkgoldenrod': 'Color::from_rgba8(184, 134, 11, 255)',
    'darkkhaki': 'Color::from_rgba8(189, 183, 107, 255)',
    'darkmagenta': 'Color::from_rgba8(139, 0, 139, 255)',
    'darkolivegreen': 'Color::from_rgba8(85, 107, 47, 255)',
    'darkorchid': 'Color::from_rgba8(153, 50, 204, 255)',
    'darksalmon': 'Color::from_rgba8(233, 150, 122, 255)',
    'darkseagreen': 'Color::from_rgba8(143, 188, 143, 255)',
    'darkslateblue': 'Color::from_rgba8(72, 61, 139, 255)',
    'darkslategray': 'Color::from_rgba8(47, 79, 79, 255)',
    'darkslategrey': 'Color::from_rgba8(47, 79, 79, 255)',
    'darkturquoise': 'Color::from_rgba8(0, 206, 209, 255)',
    'darkviolet': 'Color::from_rgba8(148, 0, 211, 255)',
    'deeppink': 'Color::from_rgba8(255, 20, 147, 255)',
    'firebrick': 'Color::from_rgba8(178, 34, 34, 255)',
    'greenyellow': 'Color::from_rgba8(173, 255, 47, 255)',
    'honeydew': 'Color::from_rgba8(240, 255, 240, 255)',
    'lawngreen': 'Color::from_rgba8(124, 252, 0, 255)',
    'chartreuse': 'Color::from_rgba8(127, 255, 0, 255)',
    'palegreen': 'Color::from_rgba8(152, 251, 152, 255)',
    'paleturquoise': 'Color::from_rgba8(175, 238, 238, 255)',
    'palevioletred': 'Color::from_rgba8(219, 112, 147, 255)',
    'rosybrown': 'Color::from_rgba8(188, 143, 143, 255)',
    'sandybrown': 'Color::from_rgba8(244, 164, 96, 255)',
    'burlywood': 'Color::from_rgba8(222, 184, 135, 255)',
    'thistle': 'Color::from_rgba8(216, 191, 216, 255)',
    'snow': 'Color::from_rgba8(255, 250, 250, 255)',
    'mintcream': 'Color::from_rgba8(245, 255, 250, 255)',
    'azure': 'Color::from_rgba8(240, 255, 255, 255)',
    'ghostwhite': 'Color::from_rgba8(248, 248, 255, 255)',
    'floralwhite': 'Color::from_rgba8(255, 250, 240, 255)',
    'aliceblue': 'Color::from_rgba8(240, 248, 255, 255)',
    'seashell': 'Color::from_rgba8(255, 245, 238, 255)',
    'oldlace': 'Color::from_rgba8(253, 245, 230, 255)',
    'gainsboro': 'Color::from_rgba8(220, 220, 220, 255)',
    'indigo': 'Color::from_rgba8(75, 0, 130, 255)',
    'aquamarine': 'Color::from_rgba8(127, 255, 212, 255)',
    'yellowgreen': 'Color::from_rgba8(154, 205, 50, 255)',
    'darkcoral': 'Color::from_rgba8(205, 91, 69, 255)',
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
        alpha = min(1.0, float(m.group(4)))
        alpha_literal = f'{alpha:.9g}'
        if '.' not in alpha_literal:
            alpha_literal += '.0'
        return (
            f'Color::from_rgba_f32({m.group(1)}.0 / 255.0, '
            f'{m.group(2)}.0 / 255.0, {m.group(3)}.0 / 255.0, '
            f'{alpha_literal})'
        )
    # CSS Color 4 whitespace syntax: rgb(R G B[ / A]). Components may be
    # numbers or percentages, and rgba() is retained as a legacy alias.
    m = re.match(
        r'^rgba?\(\s*([\d.]+%?)\s+([\d.]+%?)\s+([\d.]+%?)'
        r'(?:\s*/\s*([\d.]+%?))?\s*\)$',
        value,
    )
    if m:
        def float_literal(value: float) -> str:
            literal = f'{value:.9g}'
            return literal if '.' in literal else literal + '.0'

        def component(token: str) -> int:
            if token.endswith('%'):
                return round(min(100.0, float(token[:-1])) * 255.0 / 100.0)
            return round(min(255.0, float(token)))

        def component_expr(token: str) -> str:
            if token.endswith('%'):
                return float_literal(min(100.0, float(token[:-1])) / 100.0)
            return f'{float_literal(min(255.0, float(token)))} / 255.0'

        def alpha_expr(token: str) -> str:
            if token.endswith('%'):
                return float_literal(min(100.0, float(token[:-1])) / 100.0)
            return float_literal(min(1.0, float(token)))

        r, g, b = (component(m.group(index)) for index in range(1, 4))
        alpha_token = m.group(4)
        components = [m.group(index) for index in range(1, 4)]
        if alpha_token is None and all('%' not in token and '.' not in token for token in components):
            return f'Color::from_rgba8({r}, {g}, {b}, 255)'
        return (
            'Color::from_rgba_f32('
            + ', '.join(component_expr(token) for token in components)
            + f', {alpha_expr(alpha_token) if alpha_token is not None else "1.0"})'
        )
    # CSS Color 4 HSL syntax, including legacy comma-separated hsla() and
    # angle units. Resolve it to the style model's straight-alpha sRGB floats
    # at fixture-generation time.
    hsl = re.match(r'^hsla?\((.*)\)$', value)
    if hsl:
        content = hsl.group(1).strip()
        alpha_token = None
        if ',' in content:
            components = [part.strip() for part in content.split(',')]
            if len(components) not in (3, 4):
                return None
            if len(components) == 4:
                alpha_token = components.pop()
        else:
            color_part, slash, alpha_part = content.partition('/')
            components = color_part.split()
            if len(components) != 3:
                return None
            if slash:
                alpha_token = alpha_part.strip()

        hue_token, saturation_token, lightness_token = components
        hue_match = re.fullmatch(r'([-+]?[\d.]+)(deg|grad|rad|turn)?', hue_token)
        saturation_match = re.fullmatch(r'([-+]?[\d.]+)%', saturation_token)
        lightness_match = re.fullmatch(r'([-+]?[\d.]+)%', lightness_token)
        if not hue_match or not saturation_match or not lightness_match:
            return None
        hue = float(hue_match.group(1))
        hue_unit = hue_match.group(2) or 'deg'
        hue_degrees = {
            'deg': hue,
            'grad': hue * 0.9,
            'rad': hue * 180.0 / math.pi,
            'turn': hue * 360.0,
        }[hue_unit]
        saturation = min(100.0, max(0.0, float(saturation_match.group(1)))) / 100.0
        lightness = min(100.0, max(0.0, float(lightness_match.group(1)))) / 100.0
        red, green, blue = colorsys.hls_to_rgb(
            (hue_degrees % 360.0) / 360.0, lightness, saturation
        )
        if alpha_token is None:
            alpha = 1.0
        elif alpha_token.endswith('%'):
            alpha = min(100.0, max(0.0, float(alpha_token[:-1]))) / 100.0
        else:
            alpha = min(1.0, max(0.0, float(alpha_token)))

        def hsl_float(value: float) -> str:
            literal = f'{value:.12g}'
            return literal if '.' in literal else literal + '.0'

        return (
            'Color::from_rgba_f32('
            f'{hsl_float(red)}, {hsl_float(green)}, {hsl_float(blue)}, '
            f'{hsl_float(alpha)})'
        )
    return None


_ACTIVE_FONT_RELATIVE_RESOLVER: str | None = None
_ACTIVE_CSS_ZOOM = 1.0
_ACTIVE_LINE_HEIGHT_PX = 19.2


def _zoomed_px(value: float) -> float:
    """Apply the current cumulative CSS zoom to a fixed CSS-pixel value."""
    return round(value * _ACTIVE_CSS_ZOOM, 12)


def _effective_css_zoom(styles: dict, parent_zoom: float = 1.0) -> float:
    """Resolve the cumulative used zoom for a generated element subtree."""
    value = str(styles.get('zoom', 'normal')).strip().lower()
    local_zoom = 1.0
    try:
        if value.endswith('%'):
            candidate = float(value[:-1]) / 100.0
            if candidate > 0.0:
                local_zoom = candidate
        elif value not in ('', 'normal', 'reset'):
            candidate = float(value)
            if candidate > 0.0:
                local_zoom = candidate
    except ValueError:
        pass
    return parent_zoom * local_zoom


def _used_line_height_px(styles: dict, font_size: float) -> float:
    """Resolve the deterministic used line height for ``lh`` lengths."""
    value = styles.get('line-height')
    if value is None and 'font' in styles:
        value = _line_height_from_font_shorthand(styles['font'])
    value = str(value or 'normal').strip().lower()
    if value == 'normal':
        return font_size if not is_real_font_profile() else font_size * 1.2
    match = re.match(r'^(-?[\d.]+)px$', value)
    if match:
        return float(match.group(1))
    match = re.match(r'^(-?[\d.]+)(em|rem)$', value)
    if match:
        basis = 16.0 if match.group(2) == 'rem' else font_size
        return float(match.group(1)) * basis
    match = re.match(r'^(-?[\d.]+)%$', value)
    if match:
        return float(match.group(1)) * font_size / 100.0
    try:
        return float(value) * font_size
    except ValueError:
        return font_size if not is_real_font_profile() else font_size * 1.2


def parse_length(value: str, font_size: float = 16.0) -> str | None:
    """Convert a CSS length value to Rust Length expression.
    
    font_size: current font-size in pixels for em resolution (default 16px).
    """
    value = value.strip()
    if value == '0' or value == '0px':
        return 'Length::px(0.0)'
    if value == 'auto':
        return 'Length::auto()'
    if value == 'none':
        return 'Length::none()'
    m = re.match(r'^(-?[\d.]+)px$', value)
    if m:
        return f'Length::px({_zoomed_px(float(m.group(1)))})'
    m = re.match(r'^(-?[\d.]+)%$', value)
    if m:
        return f'Length::percent({float(m.group(1))})'
    m = re.match(r'^(-?[\d.]+)(ch|ex|lh)$', value, re.IGNORECASE)
    if m and is_real_font_profile() and _ACTIVE_FONT_RELATIVE_RESOLVER:
        unit = {"ch": "Ch", "ex": "Ex", "lh": "Lh"}[m.group(2).lower()]
        zoom = f" * {_ACTIVE_CSS_ZOOM}" if _ACTIVE_CSS_ZOOM != 1.0 else ""
        return (
            "Length::px("
            f"{_ACTIVE_FONT_RELATIVE_RESOLVER}.resolve({float(m.group(1))}, "
            f"openui_text::FontRelativeUnit::{unit}){zoom})"
        )
    if m and m.group(2).lower() == 'lh':
        return f'Length::px({_zoomed_px(float(m.group(1)) * _ACTIVE_LINE_HEIGHT_PX)})'
    # The deterministic Ahem text profile has a square zero glyph, making
    # 1ch exactly 1em. Real-font profiles are resolved from font metrics by
    # the branch above.
    if m and m.group(2).lower() == 'ch':
        return f'Length::px({_zoomed_px(float(m.group(1)) * font_size)})'
    if m and m.group(2).lower() == 'ex':
        return f'Length::px({_zoomed_px(float(m.group(1)) * font_size * 0.5)})'
    # em → convert to px using current font-size context
    m = re.match(r'^(-?[\d.]+)em$', value)
    if m:
        px_val = _zoomed_px(float(m.group(1)) * font_size)
        return f'Length::px({px_val})'
    # rem → always relative to root font-size (16px)
    m = re.match(r'^(-?[\d.]+)rem$', value)
    if m:
        px_val = _zoomed_px(float(m.group(1)) * 16.0)
        return f'Length::px({px_val})'
    # in → inches (1in = 96px)
    m = re.match(r'^(-?[\d.]+)in$', value)
    if m:
        px_val = _zoomed_px(float(m.group(1)) * 96.0)
        return f'Length::px({px_val})'
    # cm → centimeters (1cm = 37.795px)
    m = re.match(r'^(-?[\d.]+)cm$', value)
    if m:
        px_val = _zoomed_px(float(m.group(1)) * 37.7952755906)
        return f'Length::px({round(px_val, 4)})'
    # mm → millimeters (1mm = 3.7795px)
    m = re.match(r'^(-?[\d.]+)mm$', value)
    if m:
        px_val = _zoomed_px(float(m.group(1)) * 3.77952755906)
        return f'Length::px({round(px_val, 4)})'
    # Q → quarter-millimeters (1Q = 96px / 101.6)
    m = re.match(r'^(-?[\d.]+)q$', value, re.IGNORECASE)
    if m:
        px_val = _zoomed_px(float(m.group(1)) * (96.0 / 101.6))
        return f'Length::px({round(px_val, 4)})'
    # pt → points (1pt = 1.333px)
    m = re.match(r'^(-?[\d.]+)pt$', value)
    if m:
        px_val = _zoomed_px(float(m.group(1)) * (96.0 / 72.0))
        return f'Length::px({round(px_val, 4)})'
    # pc → picas (1pc = 16px)
    m = re.match(r'^(-?[\d.]+)pc$', value)
    if m:
        px_val = _zoomed_px(float(m.group(1)) * 16.0)
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
    if value == 'content':
        return 'Length::content()'
    # Keep viewport-relative lengths semantic until style computation. This
    # lets one generated fixture run against every qualification viewport.
    m = re.match(r'^(-?[\d.]+)vw$', value)
    if m:
        return f'crate::fixture_viewport_length(LengthValue::ViewportWidth({_zoomed_px(float(m.group(1)))}))'
    m = re.match(r'^(-?[\d.]+)vh$', value)
    if m:
        return f'crate::fixture_viewport_length(LengthValue::ViewportHeight({_zoomed_px(float(m.group(1)))}))'
    m = re.match(r'^(-?[\d.]+)vmin$', value)
    if m:
        return f'crate::fixture_viewport_length(LengthValue::ViewportMin({_zoomed_px(float(m.group(1)))}))'
    m = re.match(r'^(-?[\d.]+)vmax$', value)
    if m:
        return f'crate::fixture_viewport_length(LengthValue::ViewportMax({_zoomed_px(float(m.group(1)))}))'
    # calc() — pre-evaluate pure-px expressions
    m = re.match(r'^calc\((.+)\)$', value)
    if m:
        return _try_eval_calc(m.group(1), font_size)
    return None


def _grid_tokens(value: str) -> list[str] | None:
    """Tokenize a Grid track list, preserving functions and line-name lists."""
    tokens = []
    current = []
    parens = 0
    brackets = 0
    quote = None
    escaped = False
    for char in value.strip():
        if escaped:
            current.append(char)
            escaped = False
            continue
        if quote is not None:
            current.append(char)
            if char == '\\':
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in ('"', "'"):
            quote = char
            current.append(char)
        elif char == '(':
            parens += 1
            current.append(char)
        elif char == ')':
            parens -= 1
            if parens < 0:
                return None
            current.append(char)
        elif char == '[':
            brackets += 1
            current.append(char)
        elif char == ']':
            brackets -= 1
            if brackets < 0:
                return None
            current.append(char)
        elif char.isspace() and parens == 0 and brackets == 0:
            if current:
                tokens.append(''.join(current))
                current = []
        else:
            current.append(char)
    if quote is not None or parens or brackets:
        return None
    if current:
        tokens.append(''.join(current))
    return tokens


def _grid_track_breadth_rust(value: str, font_size: float) -> str | None:
    value = value.strip().lower()
    keyword = {
        'auto': 'GridTrackBreadth::Auto',
        'min-content': 'GridTrackBreadth::MinContent',
        'max-content': 'GridTrackBreadth::MaxContent',
    }.get(value)
    if keyword:
        return keyword
    match = re.fullmatch(r'([+]?(?:\d+(?:\.\d*)?|\.\d+))fr', value)
    if match:
        return f'GridTrackBreadth::Flex({float(match.group(1))})'
    match = re.fullmatch(r'fit-content\((.*)\)', value, re.DOTALL)
    if match:
        length = parse_length(match.group(1), font_size)
        if length:
            return f'GridTrackBreadth::FitContent({length})'
        return None
    length = parse_length(value, font_size)
    if length and value not in ('none', 'content'):
        return f'GridTrackBreadth::Length({length})'
    return None


def _grid_track_size_rust(value: str, font_size: float) -> str | None:
    value = value.strip()
    match = re.fullmatch(r'minmax\((.*)\)', value, re.I | re.DOTALL)
    if match:
        parts = _split_css_layers(match.group(1))
        if len(parts) != 2:
            return None
        minimum = _grid_track_breadth_rust(parts[0], font_size)
        maximum = _grid_track_breadth_rust(parts[1], font_size)
        if minimum and maximum:
            return f'GridTrackSize::MinMax {{ min: {minimum}, max: {maximum} }}'
        return None
    breadth = _grid_track_breadth_rust(value, font_size)
    return f'GridTrackSize::Breadth({breadth})' if breadth else None


def _grid_track_components_rust(value: str, font_size: float) -> list[str] | None:
    tokens = _grid_tokens(value)
    if not tokens:
        return None
    result = []
    for token in tokens:
        if token.startswith('[') and token.endswith(']'):
            names = token[1:-1].split()
            if any(not re.fullmatch(r'[-_a-zA-Z][-_a-zA-Z0-9]*', name) for name in names):
                return None
            rust_names = ', '.join(
                f'"{_rust_escape_string(name)}".to_string()' for name in names
            )
            result.append(f'GridTrackComponent::LineNames(vec![{rust_names}])')
            continue
        repeat = re.fullmatch(r'repeat\((.*)\)', token, re.I | re.DOTALL)
        if repeat:
            parts = _split_css_layers(repeat.group(1))
            if len(parts) != 2:
                return None
            repetition_value = parts[0].strip().lower()
            if repetition_value == 'auto-fill':
                repetition = 'GridRepetition::AutoFill'
            elif repetition_value == 'auto-fit':
                repetition = 'GridRepetition::AutoFit'
            elif repetition_value.isdigit() and int(repetition_value) > 0:
                repetition = f'GridRepetition::Count({int(repetition_value)})'
            else:
                return None
            repeated = _grid_track_components_rust(parts[1], font_size)
            if not repeated:
                return None
            result.append(
                'GridTrackComponent::Repeat { '
                f'repetition: {repetition}, tracks: vec![{", ".join(repeated)}] }}'
            )
            continue
        track = _grid_track_size_rust(token, font_size)
        if not track:
            return None
        result.append(f'GridTrackComponent::Track({track})')
    return result


def parse_grid_track_list(value: str, font_size: float = 16.0) -> str | None:
    value = value.strip()
    if value.lower() == 'none':
        return 'GridTrackList::None'
    if value.lower().startswith('subgrid'):
        suffix = value[len('subgrid'):].strip()
        groups = []
        if suffix:
            tokens = _grid_tokens(suffix)
            if tokens is None or any(
                not (token.startswith('[') and token.endswith(']')) for token in tokens
            ):
                return None
            for token in tokens:
                names = token[1:-1].split()
                rust_names = ', '.join(
                    f'"{_rust_escape_string(name)}".to_string()' for name in names
                )
                groups.append(f'vec![{rust_names}]')
        return f'GridTrackList::Subgrid(vec![{", ".join(groups)}])'
    components = _grid_track_components_rust(value, font_size)
    if not components:
        return None
    return f'GridTrackList::Tracks(vec![{", ".join(components)}])'


def parse_grid_line(value: str) -> str | None:
    tokens = value.strip().split()
    if not tokens or tokens == ['auto']:
        return 'GridLine::Auto'
    is_span = tokens[0].lower() == 'span'
    if is_span:
        tokens = tokens[1:]
    if not tokens or len(tokens) > 2:
        return None
    number = None
    name = None
    for token in tokens:
        if re.fullmatch(r'[+-]?\d+', token):
            number = int(token)
        elif re.fullmatch(r'[-_a-zA-Z][-_a-zA-Z0-9]*', token):
            name = token
        else:
            return None
    rust_name = (
        f'Some("{_rust_escape_string(name)}".to_string())' if name else 'None'
    )
    if is_span:
        count = number if number is not None else 1
        if count <= 0:
            return None
        return f'GridLine::Span {{ count: {count}, name: {rust_name} }}'
    index = number if number is not None else 1
    return f'GridLine::Line {{ index: {index}, name: {rust_name} }}'


def parse_grid_placement(value: str) -> str | None:
    parts = [part.strip() for part in value.split('/')]
    if len(parts) > 2 or any(not part for part in parts):
        return None
    start = parse_grid_line(parts[0])
    end = parse_grid_line(parts[1]) if len(parts) == 2 else 'GridLine::Auto'
    if start and end:
        return f'GridPlacement {{ start: {start}, end: {end} }}'
    return None


def parse_grid_template_areas(value: str) -> str | None:
    rows = re.findall(r'(["\'])(.*?)\1', value, re.DOTALL)
    if not rows:
        return None
    parsed = []
    width = None
    for _, row in rows:
        cells = row.split()
        if not cells or width is not None and len(cells) != width:
            return None
        width = len(cells)
        rust_cells = []
        for cell in cells:
            if set(cell) == {'.'}:
                rust_cells.append('None')
            elif re.fullmatch(r'[-_a-zA-Z][-_a-zA-Z0-9]*', cell):
                rust_cells.append(
                    f'Some("{_rust_escape_string(cell)}".to_string())'
                )
            else:
                return None
        parsed.append(f'vec![{", ".join(rust_cells)}]')
    return f'GridTemplateAreas {{ rows: vec![{", ".join(parsed)}] }}'


def _split_top_level_slash(value: str) -> list[str] | None:
    parts = []
    current = []
    depth = 0
    bracket = 0
    quote = None
    escaped = False
    for char in value:
        if escaped:
            current.append(char)
            escaped = False
            continue
        if quote is not None:
            current.append(char)
            if char == '\\':
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in ('"', "'"):
            quote = char
            current.append(char)
        elif char == '(':
            depth += 1
            current.append(char)
        elif char == ')':
            depth -= 1
            current.append(char)
        elif char == '[':
            bracket += 1
            current.append(char)
        elif char == ']':
            bracket -= 1
            current.append(char)
        elif char == '/' and depth == 0 and bracket == 0:
            parts.append(''.join(current).strip())
            current = []
        else:
            current.append(char)
    if quote is not None or depth or bracket:
        return None
    parts.append(''.join(current).strip())
    return parts


def _item_alignment_rust(value: str, *, self_value: bool = False) -> str | None:
    value = ' '.join(value.lower().split())
    mapping = {
        'auto': 'ItemAlignment::INITIAL_SELF',
        'normal': 'ItemAlignment::new(ItemPosition::Normal)',
        'stretch': 'ItemAlignment::new(ItemPosition::Stretch)',
        'baseline': 'ItemAlignment::new(ItemPosition::Baseline)',
        'first baseline': 'ItemAlignment::new(ItemPosition::Baseline)',
        'last baseline': 'ItemAlignment::new(ItemPosition::LastBaseline)',
        'center': 'ItemAlignment::new(ItemPosition::Center)',
        'start': 'ItemAlignment::new(ItemPosition::Start)',
        'end': 'ItemAlignment::new(ItemPosition::End)',
        'self-start': 'ItemAlignment::new(ItemPosition::SelfStart)',
        'self-end': 'ItemAlignment::new(ItemPosition::SelfEnd)',
        'flex-start': 'ItemAlignment::new(ItemPosition::FlexStart)',
        'flex-end': 'ItemAlignment::new(ItemPosition::FlexEnd)',
        'left': 'ItemAlignment::new(ItemPosition::Left)',
        'right': 'ItemAlignment::new(ItemPosition::Right)',
    }
    if value in mapping and (self_value or value != 'auto'):
        return mapping[value]
    parts = value.split()
    if len(parts) == 2 and parts[0] in ('safe', 'unsafe'):
        base = _item_alignment_rust(parts[1], self_value=self_value)
        position = re.search(r'ItemPosition::([A-Za-z]+)', base or '')
        if position:
            overflow = (
                'OverflowAlignment::Safe'
                if parts[0] == 'safe' else 'OverflowAlignment::Unsafe'
            )
            return (
                'ItemAlignment::with_overflow('
                f'ItemPosition::{position.group(1)}, {overflow})'
            )
    return None


def _content_alignment_rust(value: str) -> str | None:
    value = ' '.join(value.lower().split())
    positions = {
        'normal': 'ContentPosition::Normal',
        'baseline': 'ContentPosition::Baseline',
        'first baseline': 'ContentPosition::Baseline',
        'last baseline': 'ContentPosition::LastBaseline',
        'center': 'ContentPosition::Center',
        'start': 'ContentPosition::Start',
        'end': 'ContentPosition::End',
        'flex-start': 'ContentPosition::FlexStart',
        'flex-end': 'ContentPosition::FlexEnd',
        'left': 'ContentPosition::Left',
        'right': 'ContentPosition::Right',
    }
    distributions = {
        'stretch': 'ContentDistribution::Stretch',
        'space-between': 'ContentDistribution::SpaceBetween',
        'space-around': 'ContentDistribution::SpaceAround',
        'space-evenly': 'ContentDistribution::SpaceEvenly',
    }
    if value in positions:
        return f'ContentAlignment::new({positions[value]})'
    if value in distributions:
        return f'ContentAlignment::with_distribution({distributions[value]})'
    parts = value.split()
    if len(parts) == 2 and parts[0] in ('safe', 'unsafe') and parts[1] in positions:
        overflow = (
            'OverflowAlignment::Safe'
            if parts[0] == 'safe' else 'OverflowAlignment::Unsafe'
        )
        return (
            'ContentAlignment { '
            f'position: {positions[parts[1]]}, '
            'distribution: ContentDistribution::Default, '
            f'overflow: {overflow} }}'
        )
    return None


def _contain_intrinsic_length_rust(value: str, font_size: float) -> str | None:
    tokens = _grid_tokens(value.lower())
    if not tokens:
        return None
    if tokens == ['none']:
        return 'ContainIntrinsicLength::NONE'
    if len(tokens) == 2 and tokens[0] == 'auto':
        length = parse_length(tokens[1], font_size)
        return (
            f'ContainIntrinsicLength::auto_length({length})' if length else None
        )
    if len(tokens) == 1:
        length = parse_length(tokens[0], font_size)
        return f'ContainIntrinsicLength::length({length})' if length else None
    return None


def _viewport_px_rust(value: str) -> str | None:
    """Return a runtime CSS-pixel expression for one viewport-relative unit."""
    viewport = re.fullmatch(
        r'(-?[\d.]+)(vw|vh|vmin|vmax)', value.strip().lower()
    )
    if not viewport:
        return None
    variants = {
        'vw': 'ViewportWidth',
        'vh': 'ViewportHeight',
        'vmin': 'ViewportMin',
        'vmax': 'ViewportMax',
    }
    return (
        'crate::fixture_viewport_px('
        f'LengthValue::{variants[viewport.group(2)]}'
        f'({_zoomed_px(float(viewport.group(1)))})'
        ')'
    )


def _transform_2d_rust(
    value: str,
    font_size: float,
    reference_box: tuple[float, float] | None = None,
) -> str | None:
    value = value.strip().lower()
    if value == 'none':
        return 'Transform2D::IDENTITY'
    functions = list(re.finditer(r'([a-z0-9]+)\(([^()]*)\)', value))
    if not functions or ''.join(match.group(0) for match in functions).replace(' ', '') != value.replace(' ', ''):
        return None

    def number(token: str) -> float | None:
        try:
            return float(token)
        except ValueError:
            return None

    def length(token: str, percentage_basis: float | None = None) -> float | None:
        token = token.strip()
        if token == '0':
            return 0.0
        percentage = re.fullmatch(r'(-?[\d.]+)%', token)
        if percentage:
            amount = float(percentage.group(1))
            if amount == 0.0:
                return 0.0
            if percentage_basis is None:
                return None
            return amount / 100.0 * percentage_basis
        match = re.fullmatch(r'(-?[\d.]+)(px|em|rem)', token)
        if not match:
            return None
        scale = {
            'px': 1.0,
            'em': font_size,
            'rem': 16.0,
        }[match.group(2)]
        return _zoomed_px(float(match.group(1)) * scale)

    # Translation-only lists can retain viewport terms symbolically because
    # composing translations is simple addition. Percentage terms still use
    # the element's known reference box, exactly as in the numeric path.
    translation_x: list[str] = []
    translation_y: list[str] = []
    has_viewport_translation = False
    translations_only = True
    for function in functions:
        name = function.group(1)
        args = [
            part
            for part in re.split(r'\s*,\s*|\s+', function.group(2).strip())
            if part
        ]
        if name not in ('translate', 'translatex', 'translatey'):
            translations_only = False
            break
        if (name == 'translate' and len(args) not in (1, 2)) or (
            name in ('translatex', 'translatey') and len(args) != 1
        ):
            translations_only = False
            break

        def translation_term(token: str, basis: float | None) -> str | None:
            nonlocal has_viewport_translation
            viewport = _viewport_px_rust(token)
            if viewport:
                has_viewport_translation = True
                return viewport
            numeric = length(token, basis)
            if numeric is None:
                return None
            literal = f'{numeric:.9g}'
            return literal if any(marker in literal for marker in '.eE') else literal + '.0'

        if name in ('translate', 'translatex'):
            x = translation_term(
                args[0], reference_box[0] if reference_box else None
            )
            if x is None:
                translations_only = False
                break
            translation_x.append(x)
        if name == 'translate' and len(args) == 2:
            y = translation_term(
                args[1], reference_box[1] if reference_box else None
            )
            if y is None:
                translations_only = False
                break
            translation_y.append(y)
        elif name == 'translatey':
            y = translation_term(
                args[0], reference_box[1] if reference_box else None
            )
            if y is None:
                translations_only = False
                break
            translation_y.append(y)
    if translations_only and has_viewport_translation:
        x = ' + '.join(f'({term})' for term in translation_x) or '0.0'
        y = ' + '.join(f'({term})' for term in translation_y) or '0.0'
        return (
            'Transform2D { a: 1.0, b: 0.0, c: 0.0, d: 1.0, '
            f'e: {x}, f: {y} }}'
        )

    def angle(token: str) -> float | None:
        match = re.fullmatch(r'(-?[\d.]+)(deg|rad|grad|turn)', token.strip())
        if not match:
            return None
        amount = float(match.group(1))
        return {
            'deg': math.radians(amount),
            'rad': amount,
            'grad': amount * math.pi / 200.0,
            'turn': amount * 2.0 * math.pi,
        }[match.group(2)]

    def multiply(left, right):
        a, b, c, d, e, f = left
        g, h, i, j, k, l = right
        return (
            a * g + c * h, b * g + d * h,
            a * i + c * j, b * i + d * j,
            a * k + c * l + e, b * k + d * l + f,
        )

    matrix = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    for function in functions:
        name = function.group(1)
        args = [part for part in re.split(r'\s*,\s*|\s+', function.group(2).strip()) if part]
        operation = None
        if name == 'matrix' and len(args) == 6:
            values = [number(part) for part in args]
            if all(part is not None for part in values):
                operation = tuple(values)
        elif name in ('translate', 'translatex', 'translatey'):
            reference_width = reference_box[0] if reference_box else None
            reference_height = reference_box[1] if reference_box else None
            if name == 'translate' and len(args) in (1, 2):
                x = length(args[0], reference_width)
                y = length(args[1], reference_height) if len(args) == 2 else 0.0
                if x is not None and y is not None:
                    operation = (1.0, 0.0, 0.0, 1.0, x, y)
            elif name == 'translatex' and len(args) == 1:
                x = length(args[0], reference_width)
                if x is not None:
                    operation = (1.0, 0.0, 0.0, 1.0, x, 0.0)
            elif name == 'translatey' and len(args) == 1:
                y = length(args[0], reference_height)
                if y is not None:
                    operation = (1.0, 0.0, 0.0, 1.0, 0.0, y)
        elif name in ('scale', 'scalex', 'scaley'):
            values = [number(part) for part in args]
            if all(part is not None for part in values):
                if name == 'scale' and len(values) in (1, 2):
                    operation = (values[0], 0.0, 0.0, values[-1], 0.0, 0.0)
                elif name == 'scalex' and len(values) == 1:
                    operation = (values[0], 0.0, 0.0, 1.0, 0.0, 0.0)
                elif name == 'scaley' and len(values) == 1:
                    operation = (1.0, 0.0, 0.0, values[0], 0.0, 0.0)
        elif name == 'rotate' and len(args) == 1:
            radians = angle(args[0])
            if radians is not None:
                sine, cosine = math.sin(radians), math.cos(radians)
                operation = (cosine, sine, -sine, cosine, 0.0, 0.0)
        elif name in ('skew', 'skewx', 'skewy'):
            values = [angle(part) for part in args]
            if all(part is not None for part in values):
                if name == 'skew' and len(values) in (1, 2):
                    operation = (1.0, math.tan(values[-1]) if len(values) == 2 else 0.0, math.tan(values[0]), 1.0, 0.0, 0.0)
                elif name == 'skewx' and len(values) == 1:
                    operation = (1.0, 0.0, math.tan(values[0]), 1.0, 0.0, 0.0)
                elif name == 'skewy' and len(values) == 1:
                    operation = (1.0, math.tan(values[0]), 0.0, 1.0, 0.0, 0.0)
        if operation is None:
            return None
        matrix = multiply(matrix, operation)
    def rust_f32(value: float) -> str:
        literal = f'{value:.9g}'
        return literal if any(marker in literal for marker in '.eE') else literal + '.0'

    values = ', '.join(
        f'{name}: {rust_f32(value)}' for name, value in zip('abcdef', matrix)
    )
    return f'Transform2D {{ {values} }}'


def _shape_outside_rust(value: str, font_size: float) -> str | None:
    lower = ' '.join(value.lower().split())
    boxes = {
        'margin-box': 'ShapeOutside::MarginBox',
        'border-box': 'ShapeOutside::BorderBox',
        'padding-box': 'ShapeOutside::PaddingBox',
        'content-box': 'ShapeOutside::ContentBox',
    }
    if lower == 'none':
        return 'ShapeOutside::None'
    if lower in boxes:
        return boxes[lower]
    inset = re.fullmatch(r'inset\(\s*(.*?)\s*\)', lower)
    if inset:
        inset_value = inset.group(1)
        offset_value = inset_value.split(' round ', 1)[0].strip()
        offset_tokens = _split_respecting_parens(offset_value)
        if 1 <= len(offset_tokens) <= 4:
            parsed_offsets = [parse_length(token, font_size) for token in offset_tokens]
            if all(parsed_offsets):
                if len(parsed_offsets) == 1:
                    top = right = bottom = left = parsed_offsets[0]
                elif len(parsed_offsets) == 2:
                    top = bottom = parsed_offsets[0]
                    right = left = parsed_offsets[1]
                elif len(parsed_offsets) == 3:
                    top, right, bottom = parsed_offsets
                    left = right
                else:
                    top, right, bottom, left = parsed_offsets
                return (
                    'ShapeOutside::Inset { offsets: '
                    f'[{top}, {right}, {bottom}, {left}], '
                    'radii: [(0.0, 0.0); 4] }'
                )
    circle = re.search(r'circle\(\s*([^)]*?)\s*\)', lower)
    if circle:
        radius_token = circle.group(1).split(' at ', 1)[0].strip() or '50%'
        radius = parse_length(radius_token, font_size)
        if radius:
            return (
                'ShapeOutside::Circle { '
                f'radius: {radius}, center_x: BackgroundPosition::center(), '
                'center_y: BackgroundPosition::center() }'
            )
    return None


def _parse_calc_token(tok: str, font_size: float = 16.0):
    """Parse a single calc token, returning (value, unit) where unit is '%' or 'px'.
    Returns None if token can't be parsed."""
    m = re.match(r'^(-?[\d.]+)%$', tok)
    if m:
        return (float(m.group(1)), '%')
    m = re.match(r'^(-?[\d.]+)px$', tok)
    if m:
        return (float(m.group(1)), 'px')
    m = re.match(r'^(-?[\d.]+)rem$', tok)
    if m:
        return (float(m.group(1)) * 16.0, 'px')
    m = re.match(r'^(-?[\d.]+)em$', tok)
    if m:
        return (float(m.group(1)) * font_size, 'px')
    m = re.match(r'^(-?[\d.]+)lh$', tok)
    if m:
        return (float(m.group(1)) * _ACTIVE_LINE_HEIGHT_PX, 'px')
    # Mixed-unit calc expressions remain unsupported instead of being baked
    # against the historical 800x600 porter viewport.
    if re.match(r'^-?[\d.]+(?:vw|vh|vmin|vmax)$', tok):
        return None
    m = re.match(r'^(-?[\d.]+)$', tok)
    if m:
        return (float(m.group(1)), 'num')
    return None


def _try_eval_calc(expr: str, font_size: float = 16.0) -> str | None:
    """Try to pre-evaluate a calc() expression to a Length.
    Handles pure-px arithmetic, * / operators, and percentage+px combos."""
    expr = expr.strip()
    if 'var(' in expr:
        return None

    viewport_calc = re.fullmatch(
        r'(-?[\d.]+)(vw|vh|vmin|vmax)\s*([+-])\s*([\d.]+)px', expr
    )
    if viewport_calc:
        variants = {
            'vw': 'ViewportWidth',
            'vh': 'ViewportHeight',
            'vmin': 'ViewportMin',
            'vmax': 'ViewportMax',
        }
        amount = _zoomed_px(float(viewport_calc.group(1)))
        offset = _zoomed_px(float(viewport_calc.group(4)))
        if viewport_calc.group(3) == '-':
            offset = -offset
        return (
            'crate::fixture_viewport_calc('
            f'LengthValue::{variants[viewport_calc.group(2)]}({amount}), '
            f'{offset:.6f})'
        )
    percentage_viewport_calc = re.fullmatch(
        r'(-?[\d.]+)%\s*([+-])\s*(-?[\d.]+(?:vw|vh|vmin|vmax))', expr
    )
    if percentage_viewport_calc:
        viewport = _viewport_px_rust(percentage_viewport_calc.group(3))
        assert viewport is not None
        if percentage_viewport_calc.group(2) == '-':
            viewport = f'-({viewport})'
        return (
            f'Length::calc_percent_px({float(percentage_viewport_calc.group(1)):.6f}, '
            f'{viewport})'
        )
    # Try pure-px evaluation first: convert em/rem to px, then evaluate
    pure = re.sub(r'(\d+\.?\d*)rem', lambda m: str(float(m.group(1)) * 16.0), expr)
    pure = re.sub(r'(\d+\.?\d*)em', lambda m: str(float(m.group(1)) * font_size), pure)
    pure = re.sub(
        r'(\d+\.?\d*)lh',
        lambda m: str(float(m.group(1)) * _ACTIVE_LINE_HEIGHT_PX),
        pure,
    )
    pure = re.sub(r'(\d+\.?\d*)px', r'\1', pure)
    if not any(unit in pure for unit in ('%', 'vw', 'vh', 'vmin', 'vmax')):
        try:
            result = eval(pure, {"__builtins__": {}}, {})
            return f'Length::px({_zoomed_px(float(result)):.6f})'
        except:
            pass

    # Normalize spacing around +/- operators (but not inside numbers like -5px)
    normalized = re.sub(r'\s*([+\-])\s*', r' \1 ', expr)
    normalized = re.sub(r'\s*([*/])\s*', r' \1 ', normalized)
    tokens = normalized.split()

    # Phase 1: resolve * and / (higher precedence)
    # CSS calc spec: * and / must have one dimensionless operand
    resolved = []
    i = 0
    while i < len(tokens):
        tok = tokens[i]
        if tok in ('+', '-'):
            resolved.append(tok)
            i += 1
            continue
        parsed = _parse_calc_token(tok, font_size)
        if parsed is None:
            return None
        val, unit = parsed
        # Look ahead for * or /
        while i + 2 < len(tokens) and tokens[i + 1] in ('*', '/'):
            op = tokens[i + 1]
            next_parsed = _parse_calc_token(tokens[i + 2], font_size)
            if next_parsed is None:
                return None
            nval, nunit = next_parsed
            if op == '*':
                if unit == 'num' and nunit != 'num':
                    val, unit = val * nval, nunit
                elif nunit == 'num' and unit != 'num':
                    val = val * nval
                elif unit == 'num' and nunit == 'num':
                    val = val * nval
                else:
                    return None  # can't multiply two dimensions
            else:  # /
                if nunit != 'num' or nval == 0:
                    return None  # can only divide by dimensionless number
                val = val / nval
            i += 2
        if unit == 'num':
            unit = 'px'  # bare numbers treated as px
        resolved.append((val, unit))
        i += 1

    # Phase 2: sum up % and px terms with + and - signs
    pct_total = 0.0
    px_total = 0.0
    has_percentage_term = False
    sign = 1.0
    for item in resolved:
        if item == '+':
            sign = 1.0
        elif item == '-':
            sign = -1.0
        else:
            val, unit = item
            if unit == '%':
                has_percentage_term = True
                pct_total += sign * val
            else:
                px_total += sign * val
            sign = 1.0

    if pct_total == 0.0 and not has_percentage_term:
        return f'Length::px({_zoomed_px(px_total):.6f})'
    if px_total == 0.0 and pct_total != 0.0:
        return f'Length::percent({pct_total:.6f})'
    return f'Length::calc_percent_px({pct_total:.6f}, {_zoomed_px(px_total):.6f})'


def _resolve_css_vars(value: str, custom_props: dict[str, str]) -> str:
    """Resolve simple inherited CSS custom properties in generated WPT values."""
    if 'var(' not in value:
        return value

    def repl(match: re.Match) -> str:
        name = match.group(1).strip()
        fallback = match.group(2)
        if name in custom_props:
            return custom_props[name]
        if fallback is not None:
            return fallback.strip()
        return match.group(0)

    previous = None
    resolved = value
    # A small fixed point loop handles variables that reference variables.
    for _ in range(8):
        if resolved == previous or 'var(' not in resolved:
            break
        previous = resolved
        resolved = re.sub(r'var\(\s*(--[-\w]+)\s*(?:,\s*([^)]+))?\)', repl, resolved)
    return resolved


def _split_respecting_parens(value: str) -> list[str]:
    """Split a CSS value by top-level whitespace, respecting parentheses."""
    parts = []
    current = []
    depth = 0
    for ch in value:
        if ch == '(':
            depth += 1
            current.append(ch)
        elif ch == ')':
            depth -= 1
            current.append(ch)
        elif ch == ' ' and depth == 0:
            if current:
                parts.append(''.join(current))
                current = []
        else:
            current.append(ch)
    if current:
        parts.append(''.join(current))
    return parts


def parse_border_width(value: str, font_size: float = 16.0) -> str | None:
    """Convert a CSS border-width value to Rust i32 expression (pixels).
    
    Handles px, em, rem, in, cm, mm, pt, pc units and named widths.
    Blink floors fractional positive border widths to used device pixels,
    with a minimum 1px for non-zero values.
    """
    import math
    value = value.strip()
    if value == '0' or value == '0px':
        return '0'
    # Named widths
    mapping = {'thin': '1', 'medium': '3', 'thick': '5'}
    if value in mapping:
        return mapping[value]
    # Try to resolve to a raw pixel value
    raw = None
    m = re.match(r'^(-?[\d.]+)px$', value)
    if m:
        raw = float(m.group(1))
    if raw is None:
        m = re.match(r'^(-?[\d.]+)em$', value)
        if m:
            raw = float(m.group(1)) * font_size
    if raw is None:
        m = re.match(r'^(-?[\d.]+)rem$', value)
        if m:
            raw = float(m.group(1)) * 16.0
    if raw is None:
        m = re.match(r'^(-?[\d.]+)in$', value)
        if m:
            raw = float(m.group(1)) * 96.0
    if raw is None:
        m = re.match(r'^(-?[\d.]+)cm$', value)
        if m:
            raw = float(m.group(1)) * 37.7952755906
    if raw is None:
        m = re.match(r'^(-?[\d.]+)mm$', value)
        if m:
            raw = float(m.group(1)) * 3.77952755906
    if raw is None:
        m = re.match(r'^(-?[\d.]+)pt$', value)
        if m:
            raw = float(m.group(1)) * (96.0 / 72.0)
    if raw is None:
        m = re.match(r'^(-?[\d.]+)pc$', value)
        if m:
            raw = float(m.group(1)) * 16.0
    if raw is not None:
        if raw < 0:
            return None
        if raw > 0:
            rounded = max(1, math.floor(raw))
        else:
            rounded = 0
        # Computed border widths ultimately become Blink-style LayoutUnits
        # (26 integer bits plus 6 fractional bits). Clamp at the largest
        # whole-pixel value that the fixed-point representation can hold.
        rounded = min(rounded, (2 ** 31 - 1) // 64)
        return str(rounded)
    return None


_FONT_SIZE_TOKEN = re.compile(
    r"^(?:xx-small|x-small|small|medium|large|x-large|xx-large|xxx-large|"
    r"smaller|larger|[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:px|pt|pc|in|cm|mm|em|rem|%))$",
    re.IGNORECASE,
)
_FONT_LINE_HEIGHT_TOKEN = re.compile(
    r"^(?:normal|[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:px|pt|pc|in|cm|mm|em|rem|%)?)$",
    re.IGNORECASE,
)
_FONT_STRETCH_KEYWORDS = {
    "ultra-condensed", "extra-condensed", "condensed", "semi-condensed",
    "normal", "semi-expanded", "expanded", "extra-expanded", "ultra-expanded",
}


class UnsupportedFontShorthand(ValueError):
    pass


def _font_tokens(value: str) -> list[str]:
    """Split the pre-family shorthand while preserving quotes and `/`."""
    tokens = []
    current = []
    quote = None
    for ch in value:
        if quote:
            current.append(ch)
            if ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
            current.append(ch)
        elif ch == '/':
            if current:
                tokens.append(''.join(current))
                current = []
            tokens.append('/')
        elif ch.isspace():
            if current:
                tokens.append(''.join(current))
                current = []
        else:
            current.append(ch)
    if quote:
        raise UnsupportedFontShorthand("unterminated family quote")
    if current:
        tokens.append(''.join(current))
    return tokens


def _valid_family_list(value: str) -> bool:
    if not value:
        return False
    parts = []
    current = []
    quote = None
    for ch in value:
        if quote:
            current.append(ch)
            if ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
            current.append(ch)
        elif ch == ',':
            parts.append(''.join(current).strip())
            current = []
        else:
            current.append(ch)
    parts.append(''.join(current).strip())
    return quote is None and bool(parts) and all(parts)


def parse_font_shorthand(value: str) -> OrderedDict:
    """Expand the corpus-used CSS `font` grammar into computed longhands.

    Unsupported or ambiguous forms raise instead of partially applying a size;
    the splice transaction therefore remains all-or-nothing.
    """
    original = value
    value = value.strip().rstrip(';').strip()
    lower = value.lower()
    longhands = OrderedDict()
    if lower in {"inherit", "initial"}:
        if lower == "inherit":
            inherited = "inherit"
            for prop in (
                "font-style", "font-variant-caps", "font-weight", "font-stretch",
                "font-size", "line-height", "font-family",
            ):
                longhands[prop] = inherited
        else:
            longhands.update((
                ("font-style", "normal"),
                ("font-variant-caps", "normal"),
                ("font-weight", "normal"),
                ("font-stretch", "normal"),
                ("font-size", "medium"),
                ("line-height", "normal"),
                ("font-family", "sans-serif"),
            ))
        return longhands
    if lower in {"caption", "icon", "menu", "message-box", "small-caption", "status-bar"}:
        raise UnsupportedFontShorthand(f"unsupported system font: {original}")

    tokens = _font_tokens(value)
    size_index = next((i for i, token in enumerate(tokens) if _FONT_SIZE_TOKEN.match(token)), None)
    if size_index is None:
        raise UnsupportedFontShorthand(f"missing font size: {original}")

    style = "normal"
    variant = "normal"
    weight = "normal"
    stretch = "normal"
    seen = set()
    index = 0
    while index < size_index:
        token = tokens[index].lower()
        if token == "normal":
            index += 1
            continue
        if token in {"italic", "oblique"}:
            if "style" in seen:
                raise UnsupportedFontShorthand(f"duplicate font style: {original}")
            style = token
            seen.add("style")
            if token == "oblique" and index + 1 < size_index and re.match(
                r"^[-+]?(?:\d+(?:\.\d*)?|\.\d+)deg$", tokens[index + 1], re.I
            ):
                style += " " + tokens[index + 1]
                index += 1
        elif token == "small-caps":
            if "variant" in seen:
                raise UnsupportedFontShorthand(f"duplicate font variant: {original}")
            variant = token
            seen.add("variant")
        elif token in {"bold", "bolder", "lighter"} or re.match(r"^\d{1,4}$", token):
            if "weight" in seen:
                raise UnsupportedFontShorthand(f"duplicate font weight: {original}")
            if token.isdigit() and not 1 <= int(token) <= 1000:
                raise UnsupportedFontShorthand(f"invalid font weight: {original}")
            weight = token
            seen.add("weight")
        elif token in _FONT_STRETCH_KEYWORDS - {"normal"}:
            if "stretch" in seen:
                raise UnsupportedFontShorthand(f"duplicate font stretch: {original}")
            stretch = token
            seen.add("stretch")
        else:
            raise UnsupportedFontShorthand(f"unsupported font prefix {token!r}: {original}")
        index += 1

    size = tokens[size_index]
    index = size_index + 1
    line_height = "normal"
    if index < len(tokens) and tokens[index] == '/':
        index += 1
        if index >= len(tokens) or not _FONT_LINE_HEIGHT_TOKEN.match(tokens[index]):
            raise UnsupportedFontShorthand(f"invalid line height: {original}")
        line_height = tokens[index]
        index += 1
    family = ' '.join(tokens[index:]).strip()
    if not _valid_family_list(family):
        raise UnsupportedFontShorthand(f"missing or invalid font family: {original}")

    longhands.update((
        ("font-style", style),
        ("font-variant-caps", variant),
        ("font-weight", weight),
        ("font-stretch", stretch),
        ("font-size", size),
        ("line-height", line_height),
        ("font-family", family),
    ))
    return longhands


_CSS_WIDE = {'inherit', 'initial', 'unset', 'revert', 'revert-layer'}
_COLUMN_RULE_STYLES = {
    'none', 'hidden', 'dotted', 'dashed', 'solid', 'double',
    'groove', 'ridge', 'inset', 'outset',
}

_SP17_PROPERTY_VALUES = {
    'writing-mode': {
        'horizontal-tb', 'vertical-rl', 'vertical-lr', 'sideways-rl', 'sideways-lr',
    },
    'direction': {'ltr', 'rtl'},
    'unicode-bidi': {
        'normal', 'embed', 'bidi-override', 'isolate', 'isolate-override',
        'plaintext',
    },
    'text-orientation': {'mixed', 'upright', 'sideways'},
    'text-combine-upright': {'none', 'all'},
}
_SP17_INITIAL_VALUES = {
    'writing-mode': 'horizontal-tb',
    'direction': 'ltr',
    'unicode-bidi': 'normal',
    'text-orientation': 'mixed',
    'text-combine-upright': 'none',
}
_SP17_INHERITED_PROPERTIES = {
    'writing-mode', 'direction', 'text-orientation', 'text-combine-upright',
    'text-emphasis-style', 'text-emphasis-position', 'text-emphasis-color',
}


class CssDeclarations(OrderedDict):
    """Ordered declarations plus the winning CSS cascade priority per property.

    The generated Rust DOM stores computed physical fields. Logical and physical
    declarations can therefore converge on one field only after writing-mode
    and direction are known. Keeping priority metadata lets that late mapping
    preserve importance, specificity, and declaration order without changing
    the dict interface used by the historical porter.
    """

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.cascade_priority: dict[str, tuple] = {}
        self.important: dict[str, bool] = {}


def _copy_declarations(styles: dict) -> CssDeclarations:
    copied = CssDeclarations(styles)
    if isinstance(styles, CssDeclarations):
        copied.cascade_priority.update(styles.cascade_priority)
        copied.important.update(styles.important)
    return copied


def _set_declaration(
    styles: dict,
    prop: str,
    value: str,
    *,
    priority: tuple | None = None,
    important: bool = False,
) -> None:
    if isinstance(styles, CssDeclarations) and priority is not None:
        existing = styles.cascade_priority.get(prop)
        if existing is not None and priority < existing:
            return
    styles[prop] = value
    if isinstance(styles, CssDeclarations):
        if priority is not None:
            styles.cascade_priority[prop] = priority
        styles.important[prop] = important


def _split_important(value: str) -> tuple[str, bool]:
    match = re.search(r'\s*!\s*important\s*$', value, re.IGNORECASE)
    if not match:
        return value.strip(), False
    return value[:match.start()].strip(), True


def _normalized_sp17_value(prop: str, value: str) -> str | None:
    """Return a transactional computed token, or None for an invalid value."""
    value = value.strip().lower()
    if value in _SP17_PROPERTY_VALUES[prop]:
        return value
    if value not in _CSS_WIDE:
        return None
    if value == 'inherit':
        return value
    if value == 'unset' and prop in _SP17_INHERITED_PROPERTIES:
        return 'inherit'
    return _SP17_INITIAL_VALUES[prop]


def _nonnegative_css_length(value: str, *, strictly_positive: bool = False) -> bool:
    """Validate the corpus-used non-negative <length-percentage> grammar."""
    value = value.strip().lower()
    if value in _CSS_WIDE:
        return True
    if value.startswith(('calc(', 'min(', 'max(', 'clamp(')):
        # Functional values are validated by Chromium before reaching the
        # generated snapshot.  Reject only a statically evident negative.
        return not re.match(r'^(?:calc|min|max|clamp)\(\s*-', value)
    match = re.fullmatch(
        r'([+-]?(?:\d+(?:\.\d*)?|\.\d+))'
        r'(px|em|rem|%|pt|pc|in|cm|mm|q|vw|vh|vmin|vmax)?',
        value,
    )
    if not match:
        return False
    number = float(match.group(1))
    if match.group(2) is None and number != 0:
        return False
    return number > 0 if strictly_positive else number >= 0


def _parse_columns_shorthand(value: str) -> OrderedDict | None:
    """Return transactional longhands for a valid `columns` declaration."""
    value = value.strip().lower()
    if value in _CSS_WIDE:
        computed = 'auto' if value in {'initial', 'unset', 'revert', 'revert-layer'} else value
        return OrderedDict((('column-width', computed), ('column-count', computed)))

    height = None
    if '/' in value:
        if value.count('/') != 1:
            return None
        value, height = (part.strip() for part in value.split('/', 1))
        if not _nonnegative_css_length(height):
            return None
    parts = value.split()
    if not 1 <= len(parts) <= 2:
        return None
    count = None
    width = None
    auto_count = 0
    for part in parts:
        if part == 'auto':
            auto_count += 1
        elif re.fullmatch(r'\+?\d+', part) and int(part) >= 1:
            if count is not None:
                return None
            count = str(int(part))
        elif '%' not in part and _nonnegative_css_length(part):
            if width is not None:
                return None
            width = part
        else:
            return None
    if count is not None and width is not None and auto_count:
        return None
    if auto_count > 1 or (auto_count and len(parts) == 2 and count is None and width is None):
        return None
    longhands = OrderedDict((('column-width', width or 'auto'), ('column-count', count or 'auto')))
    if height is not None:
        longhands['column-height'] = height
    return longhands


def _parse_column_rule_shorthand(value: str) -> OrderedDict | None:
    """Return reset-and-set longhands for a valid `column-rule` declaration."""
    value = value.strip().lower()
    if value in _CSS_WIDE:
        if value in {'initial', 'unset', 'revert', 'revert-layer'}:
            return OrderedDict((
                ('column-rule-width', 'medium'),
                ('column-rule-style', 'none'),
                ('column-rule-color', 'currentcolor'),
            ))
        return OrderedDict((
            ('column-rule-width', value),
            ('column-rule-style', value),
            ('column-rule-color', value),
        ))
    width = style = color = None
    for token in value.split():
        if token in _COLUMN_RULE_STYLES:
            if style is not None:
                return None
            style = token
        elif parse_border_width(token) is not None:
            if width is not None:
                return None
            width = token
        elif parse_color(token) is not None or token == 'currentcolor':
            if color is not None:
                return None
            color = token
        else:
            return None
    if not value or (width is None and style is None and color is None):
        return None
    return OrderedDict((
        ('column-rule-width', width or 'medium'),
        ('column-rule-style', style or 'none'),
        ('column-rule-color', color or 'currentcolor'),
    ))


def _valid_multicol_longhand(prop: str, value: str) -> bool:
    value = value.strip().lower()
    if value in _CSS_WIDE:
        return True
    if prop == 'column-count':
        return value == 'auto' or bool(re.fullmatch(r'\+?\d+', value) and int(value) >= 1)
    if prop == 'column-width':
        # CSS Multicol defines <column-width> as a <length>, not a
        # <length-percentage>. Invalid percentages must leave the prior
        # cascaded value untouched.
        return value == 'auto' or (
            '%' not in value and _nonnegative_css_length(value)
        )
    if prop == 'column-height':
        return value == 'auto' or _nonnegative_css_length(value)
    if prop in {'column-gap', 'row-gap'}:
        return value == 'normal' or _nonnegative_css_length(value)
    if prop == 'column-fill':
        return value in {'auto', 'balance', 'balance-all'}
    if prop == 'column-span':
        return value in {'none', 'all'}
    if prop == 'column-wrap':
        return value in {'auto', 'wrap', 'nowrap'}
    if prop == 'column-rule-width':
        return parse_border_width(value) is not None
    if prop == 'column-rule-style':
        return value in _COLUMN_RULE_STYLES
    if prop == 'column-rule-color':
        return value == 'currentcolor' or parse_color(value) is not None
    return True


def _valid_zoom(value: str) -> bool:
    """Validate the corpus-used CSS zoom grammar without disturbing cascade."""
    value = value.strip().lower()
    if value in _CSS_WIDE | {'normal', 'reset'}:
        return True
    match = re.fullmatch(r'([+]?(?:\d+(?:\.\d*)?|\.\d+))(%?)', value)
    return bool(match and float(match.group(1)) > 0.0)


def _multicol_initial_value(prop: str) -> str:
    return {
        'column-count': 'auto',
        'column-width': 'auto',
        'column-height': 'auto',
        'column-gap': 'normal',
        'row-gap': 'normal',
        'column-fill': 'balance',
        'column-span': 'none',
        'column-wrap': 'auto',
        'column-rule-width': 'medium',
        'column-rule-style': 'none',
        'column-rule-color': 'currentcolor',
    }[prop]


def _split_css_declarations(style_str: str) -> list[str]:
    """Split a declaration block without breaking quoted/data-URL semicolons."""
    result = []
    current = []
    depth = 0
    quote = None
    escaped = False
    for char in style_str:
        if escaped:
            current.append(char)
            escaped = False
            continue
        if char == '\\' and quote is not None:
            current.append(char)
            escaped = True
            continue
        if quote is not None:
            current.append(char)
            if char == quote:
                quote = None
            continue
        if char in ('"', "'"):
            quote = char
            current.append(char)
        elif char == '(':
            depth += 1
            current.append(char)
        elif char == ')':
            depth = max(0, depth - 1)
            current.append(char)
        elif char == ';' and depth == 0:
            result.append(''.join(current))
            current = []
        else:
            current.append(char)
    if current:
        result.append(''.join(current))
    return result


def parse_inline_styles(style_str: str) -> dict:
    """Parse declarations transactionally, preserving the valid cascade."""
    result = CssDeclarations()
    if not style_str:
        return result
    for declaration_index, decl in enumerate(_split_css_declarations(style_str)):
        decl = decl.strip()
        if ':' not in decl:
            continue
        prop, val = decl.split(':', 1)
        prop = prop.strip().lower()
        val, important = _split_important(val)
        priority = (int(important), 1, 0, 0, 0, 0, declaration_index)
        if prop == 'columns':
            expanded = _parse_columns_shorthand(val)
            if expanded is not None:
                for longhand, value in expanded.items():
                    _set_declaration(
                        result, longhand, value, priority=priority,
                        important=important,
                    )
        elif prop == 'column-rule':
            expanded = _parse_column_rule_shorthand(val)
            if expanded is not None:
                for longhand, value in expanded.items():
                    _set_declaration(
                        result, longhand, value, priority=priority,
                        important=important,
                    )
        elif prop.startswith('column-') or prop == 'row-gap':
            # Invalid declarations do not replace an earlier valid declaration
            # in the same block (CSS Cascade §5).
            if _valid_multicol_longhand(prop, val):
                value = (
                    _multicol_initial_value(prop)
                    if val.strip().lower() in {'initial', 'unset', 'revert', 'revert-layer'}
                    else val
                )
                _set_declaration(
                    result, prop, value, priority=priority, important=important
                )
        elif prop in _SP17_PROPERTY_VALUES:
            value = _normalized_sp17_value(prop, val)
            if value is not None:
                _set_declaration(
                    result, prop, value, priority=priority, important=important
                )
        elif prop == 'zoom':
            if _valid_zoom(val):
                value = (
                    'normal'
                    if val.strip().lower() in {'initial', 'unset', 'revert', 'revert-layer'}
                    else val
                )
                _set_declaration(
                    result, prop, value, priority=priority, important=important
                )
        elif prop == 'box-shadow':
            layers = _split_css_layers(val)
            # `none` is exclusive with the shadow-list grammar. Invalid later
            # declarations must not replace the earlier valid cascaded value.
            if not (
                len(layers) > 1
                and any(layer.strip().lower() == 'none' for layer in layers)
            ):
                _set_declaration(
                    result, prop, val, priority=priority, important=important
                )
        elif prop == "font" and is_real_font_profile():
            for longhand, value in parse_font_shorthand(val).items():
                _set_declaration(
                    result, longhand, value, priority=priority, important=important
                )
        else:
            _set_declaration(
                result, prop, val, priority=priority, important=important
            )
    return result


def check_supported(styles: dict) -> tuple[bool, str]:
    """Check if all CSS properties in styles are supported. Returns (supported, reason)."""
    for prop in styles:
        if prop == 'clip-path' and not re.fullmatch(
            r'(?is)\s*(?:none|inset\([^{}]*\))\s*', styles[prop]
        ):
            return False, f"unsupported clip-path: {styles[prop]}"
        if prop in UNSUPPORTED_FEATURES:
            return False, f"unsupported property: {prop}"
        if prop in IGNORED_PROPERTIES:
            continue  # Safe to ignore — doesn't affect box layout
        if prop in (
            '-webkit-line-clamp', '-webkit-box-orient', '-webkit-box-align',
            '-webkit-box-pack', '-webkit-appearance', '-moz-appearance',
            '-webkit-column-break-before', '-webkit-column-break-after',
        ):
            continue
        if prop.startswith('-webkit-') or prop.startswith('-moz-'):
            # Skip vendor-prefixed properties if the unprefixed version is present
            unprefixed = prop.split('-', 2)[2] if prop.count('-') >= 2 else ''
            if unprefixed and unprefixed in styles:
                continue
            return False, f"vendor/unsupported prefix: {prop}"
        # Unknown declarations are invalid CSS and do not participate in the
        # cascade. Explicitly recognized-but-unimplemented features are listed
        # in UNSUPPORTED_FEATURES above; a typo such as `xcolor` is ignored.
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
        self.pseudo_styles = {
            'before': CssDeclarations(),
            'after': CssDeclarations(),
            'first-line': CssDeclarations(),
            'first-letter': CssDeclarations(),
            'marker': CssDeclarations(),
            'scroll-marker': CssDeclarations(),
            'scroll-marker-target-current': CssDeclarations(),
            'scroll-marker-group': CssDeclarations(),
            'scroll-button-up': CssDeclarations(),
            'scroll-button-right': CssDeclarations(),
            'scroll-button-down': CssDeclarations(),
            'scroll-button-left': CssDeclarations(),
            'scroll-button-block-start': CssDeclarations(),
            'scroll-button-block-end': CssDeclarations(),
            'scroll-button-inline-start': CssDeclarations(),
            'scroll-button-inline-end': CssDeclarations(),
            'column': CssDeclarations(),
            'column-scroll-marker': CssDeclarations(),
            'column-scroll-marker-target-current': CssDeclarations(),
            'details-content': CssDeclarations(),
        }
        self.pseudo_priorities = {name: {} for name in self.pseudo_styles}

    def __repr__(self):
        return f"<{self.tag} style={self.styles}>"


def _candidate_mutation_ir(test_id: str) -> dict | None:
    """Return the checked-in AST mutation IR for one candidate, if lowerable."""
    global _MUTATION_AUDIT_BY_TEST_ID
    if _MUTATION_AUDIT_BY_TEST_ID is None:
        audit = json.loads(JAVASCRIPT_MUTATION_AUDIT.read_text(encoding='utf-8'))
        if audit.get('schema_version') != 2:
            raise ValueError('unsupported JavaScript mutation audit schema')
        _MUTATION_AUDIT_BY_TEST_ID = {
            entry['test_id']: entry
            for entry in audit['entries']
        }
    entry = _MUTATION_AUDIT_BY_TEST_ID.get(test_id)
    if entry is None or entry.get('disposition') not in {
        'ast-lowered-pending-exact', 'lowered-exact',
    }:
        return None
    mutation_ir = entry.get('mutation_ir')
    if not isinstance(mutation_ir, dict) or not mutation_ir.get('lowerable'):
        raise ValueError(f'invalid lowerable mutation IR for {test_id}')
    return mutation_ir


def _apply_ast_mutation_ir(parser: 'WptHtmlParser') -> None:
    """Apply bounded synchronous mutation IR to the parsed final-state DOM.

    The operation list comes from Acorn, not JavaScript text matching. Layout
    reads are explicit barriers in the ledger; because their values cannot
    control subsequent behavior, the static builder retains their order while
    materializing the same final DOM and cascade state.
    """
    mutation_ir = parser.mutation_ir
    if mutation_ir is None:
        return

    html_target = object()
    document_target = object()
    bindings: dict[str, object] = {
        'document': document_target,
        'window': document_target,
    }
    created_by_offset: dict[int, DomNode] = {}

    class StyleElementTarget:
        def __init__(self, element_id):
            self.element_id = element_id

    def walk(node):
        yield node
        if not node.is_text:
            for child in node.children:
                yield from walk(child)

    def find_parent(needle):
        for candidate in walk(parser.root):
            if not candidate.is_text and needle in candidate.children:
                return candidate
        return None

    def find_id(identifier):
        if not isinstance(identifier, str):
            return None
        node = next(
            (
                node for node in walk(parser.root)
                if not node.is_text and node.attrs.get('id') == identifier
            ),
            None,
        )
        if node is not None:
            return node
        if any(attrs.get('id') == identifier for attrs, _text, _prefix in parser.author_style_blocks):
            return StyleElementTarget(identifier)
        return None

    def query(selector, *, all_matches=False):
        if not isinstance(selector, str):
            return [] if all_matches else None
        selector = selector.strip()
        matches = []
        for node in walk(parser.root):
            if node.is_text:
                continue
            if selector.startswith('#'):
                matched = node.attrs.get('id') == selector[1:]
            elif selector.startswith('.'):
                matched = selector[1:] in node.attrs.get('class', '').split()
            else:
                matched = node.tag.lower() == selector.lower()
            if matched:
                if not all_matches:
                    return node
                matches.append(node)
        return matches if all_matches else None

    def resolve(description):
        if not isinstance(description, dict):
            return None
        kind = description.get('kind')
        if kind == 'binding':
            name = description.get('name')
            if name in bindings:
                return bindings[name]
            resolved = parser.root if name == 'body' else find_id(name)
            if resolved is not None:
                bindings[name] = resolved
            return resolved
        if kind == 'dom-query':
            method = description.get('method')
            argument = description.get('argument')
            if method == 'getElementById':
                return find_id(argument)
            if method == 'querySelector':
                return query(argument)
            if method in {'querySelectorAll', 'getElementsByClassName', 'getElementsByTagName'}:
                selector = argument
                if method == 'getElementsByClassName' and isinstance(argument, str):
                    selector = '.' + argument
                return query(selector, all_matches=True)
            return None
        if kind == 'created-node':
            return created_by_offset.get(description.get('source_offset'))
        if kind == 'member':
            owner = resolve(description.get('object'))
            member = description.get('property')
            if owner is document_target:
                if member == 'body':
                    return parser.root
                if member == 'documentElement':
                    return html_target
                return None
            if member == 'classList':
                return owner
            if member in {'parentNode', 'parentElement'} and isinstance(owner, DomNode):
                return find_parent(owner)
            if member == 'children' and isinstance(owner, DomNode):
                return [child for child in owner.children if not child.is_text]
            if isinstance(owner, list) and isinstance(member, int):
                return owner[member] if 0 <= member < len(owner) else None
            return None
        return None

    def detach(node):
        parent = find_parent(node)
        if parent is not None:
            parent.children.remove(node)

    def append(parent, child):
        if isinstance(parent, StyleElementTarget) and isinstance(child, DomNode):
            detach(child)
            return True
        if not isinstance(parent, DomNode) or not isinstance(child, DomNode):
            return False
        detach(child)
        if child.tag == '#document-fragment':
            fragment_children = list(child.children)
            child.children.clear()
            for fragment_child in fragment_children:
                append(parent, fragment_child)
        else:
            parent.children.append(child)
        return True

    def replace_text(node, value):
        if not isinstance(node, DomNode) or not isinstance(value, str):
            return False
        if node.is_text:
            node.text_content = value
            return True
        node.children.clear()
        if value:
            text = DomNode('#text', {}, CssDeclarations())
            text.is_text = True
            text.text_content = value
            node.children.append(text)
        return True

    def replace_markup(node, markup):
        if not isinstance(node, DomNode) or not isinstance(markup, str):
            return False
        fragment = WptHtmlParser(root_aware=parser.root_aware)
        fragment.in_body = True
        fragment.feed(markup)
        node.children = fragment.root.children
        return True

    def style_target(target):
        if target is html_target:
            return parser.html_styles
        return target.styles if isinstance(target, DomNode) and not target.is_text else None

    def attributes_target(target):
        if target is html_target:
            return parser.html_attrs
        return target.attrs if isinstance(target, DomNode) and not target.is_text else None

    unsupported = []
    barriers = []
    for operation in mutation_ir.get('operations', []):
        kind = operation.get('kind')
        if kind == 'layout-barrier':
            barriers.append({
                'order': operation.get('order'),
                'property': operation.get('property'),
                'method': operation.get('method'),
            })
            continue
        if kind == 'bind-target':
            target = resolve(operation.get('target'))
            if target is None:
                unsupported.append((kind, operation.get('order'), 'unresolved target'))
            else:
                bindings[operation['binding']] = target
            continue
        if kind in {'create-element', 'create-fragment', 'create-text'}:
            if kind == 'create-text':
                node = DomNode('#text', {}, CssDeclarations())
                node.is_text = True
                node.text_content = operation.get('text') or ''
            else:
                tag = '#document-fragment' if kind == 'create-fragment' else operation.get('tag_name')
                node = DomNode(str(tag).lower(), {}, CssDeclarations())
            created_by_offset[operation['source_offset']] = node
            if operation.get('binding'):
                bindings[operation['binding']] = node
            continue
        if kind == 'clone-target':
            source = resolve(operation.get('target'))
            if not isinstance(source, DomNode):
                unsupported.append((kind, operation.get('order'), 'invalid clone target'))
                continue
            node = copy.deepcopy(source)
            if not operation.get('deep'):
                node.children.clear()
            created_by_offset[operation['source_offset']] = node
            if operation.get('binding'):
                bindings[operation['binding']] = node
            continue

        target = resolve(operation.get('target'))
        if kind == 'set-style':
            styles = style_target(target)
            delta = operation.get('cascade_delta') or {}
            name = delta.get('css_name')
            value = delta.get('value')
            if styles is None or not isinstance(name, str) or not isinstance(value, str):
                unsupported.append((kind, operation.get('order'), 'invalid cascade delta'))
            else:
                styles[name] = value
        elif kind == 'set-style-text':
            styles = style_target(target)
            if styles is None or not isinstance(operation.get('value'), str):
                unsupported.append((kind, operation.get('order'), 'invalid style text'))
            else:
                parsed = parse_inline_styles(operation['value'])
                styles.clear()
                styles.update(parsed)
        elif kind == 'set-attribute':
            attrs = attributes_target(target)
            name = operation.get('name')
            value = operation.get('value')
            if attrs is None or not isinstance(name, str):
                unsupported.append((kind, operation.get('order'), 'invalid attribute target'))
            elif name.lower() == 'style':
                styles = style_target(target)
                parsed = parse_inline_styles(value if isinstance(value, str) else '')
                styles.clear()
                styles.update(parsed)
                attrs[name] = value if isinstance(value, str) else ''
            else:
                attrs[name] = '' if value is True else str(value)
        elif kind == 'remove-attribute':
            attrs = attributes_target(target)
            name = operation.get('name')
            if attrs is None or not isinstance(name, str):
                unsupported.append((kind, operation.get('order'), 'invalid attribute target'))
            else:
                attrs.pop(name, None)
                if name.lower() == 'style':
                    styles = style_target(target)
                    styles.clear()
        elif kind == 'class-list':
            attrs = attributes_target(target)
            if attrs is None:
                unsupported.append((kind, operation.get('order'), 'invalid class target'))
                continue
            classes = attrs.get('class', '').split()
            action = operation.get('action')
            tokens = [token for token in operation.get('tokens', []) if isinstance(token, str)]
            if action == 'add':
                classes.extend(token for token in tokens if token not in classes)
            elif action == 'remove':
                classes = [token for token in classes if token not in tokens]
            elif action == 'toggle' and tokens:
                token = tokens[0]
                classes = [item for item in classes if item != token] if token in classes else classes + [token]
            elif action == 'replace' and len(tokens) == 2:
                classes = [tokens[1] if item == tokens[0] else item for item in classes]
            else:
                unsupported.append((kind, operation.get('order'), 'invalid class operation'))
                continue
            attrs['class'] = ' '.join(classes)
        elif kind == 'replace-text':
            if not replace_text(target, operation.get('value')):
                unsupported.append((kind, operation.get('order'), 'invalid text target'))
        elif kind == 'replace-children-markup':
            if not replace_markup(target, operation.get('markup')):
                unsupported.append((kind, operation.get('order'), 'invalid markup target'))
        elif kind == 'append':
            children = [resolve(child) for child in operation.get('children', [])]
            if not children or any(not append(target, child) for child in children):
                unsupported.append((kind, operation.get('order'), 'invalid append target'))
        elif kind == 'insert-before':
            child = resolve(operation.get('child'))
            before = resolve(operation.get('before'))
            if not isinstance(target, DomNode) or not isinstance(child, DomNode) or before not in target.children:
                unsupported.append((kind, operation.get('order'), 'invalid insertion target'))
            else:
                detach(child)
                target.children.insert(target.children.index(before), child)
        elif kind == 'remove':
            if isinstance(target, StyleElementTarget):
                removed = False
                retained = []
                for attrs, text, prefix in parser.author_style_blocks:
                    if not removed and attrs.get('id') == target.element_id:
                        removed = True
                        for rule in parse_simple_css_rules(text):
                            try:
                                parser.css_rules.remove(rule)
                            except ValueError:
                                pass
                    else:
                        retained.append((attrs, text, prefix))
                parser.author_style_blocks = retained
                if not removed:
                    unsupported.append((kind, operation.get('order'), 'missing style element'))
            elif not isinstance(target, DomNode) or find_parent(target) is None:
                unsupported.append((kind, operation.get('order'), 'invalid removal target'))
            else:
                detach(target)
        elif kind in {'scroll-axis', 'scroll-to'}:
            if not isinstance(target, DomNode):
                unsupported.append((kind, operation.get('order'), 'invalid scroll target'))
            elif kind == 'scroll-axis':
                field = 'scroll_top' if operation.get('axis') == 'scrollTop' else 'scroll_left'
                setattr(target, field, float(operation.get('value') or 0.0))
            else:
                arguments = operation.get('arguments', [])
                if len(arguments) >= 1:
                    target.scroll_left = float(arguments[0])
                if len(arguments) >= 2:
                    target.scroll_top = float(arguments[1])
        else:
            unsupported.append((kind, operation.get('order'), 'unknown operation'))

    parser.lowered_layout_barriers = barriers
    parser.root.lowered_layout_barriers = list(barriers)
    if unsupported:
        detail = ', '.join(f'{kind}@{order}: {reason}' for kind, order, reason in unsupported)
        raise ValueError(f'AST mutation IR could not be lowered: {detail}')


def _serialize_mutation_styles(styles) -> str:
    """Serialize the authored inline declarations before the final cascade."""
    if styles is None:
        return ''
    declarations = []
    for name, value in styles.items():
        important = bool(
            isinstance(styles, CssDeclarations)
            and styles.important.get(name, False)
        )
        declarations.append(
            f"{name}: {value}{' !important' if important else ''}"
        )
    return "; ".join(declarations)


def _serialize_mutation_attrs(
    attrs: dict,
    styles,
    *,
    extra: dict[str, str] | None = None,
) -> str:
    """Serialize inert final-state attributes, excluding executable handlers."""
    values = {
        str(name): value
        for name, value in attrs.items()
        if not re.fullmatch(r'on[a-z]+', str(name), re.IGNORECASE)
    }
    if styles is not None:
        inline = _serialize_mutation_styles(styles)
        if inline:
            values['style'] = inline
        else:
            values.pop('style', None)
    if extra:
        values.update(extra)
    encoded = []
    for name, value in values.items():
        if value is None:
            encoded.append(f" {name}")
        else:
            encoded.append(
                f' {name}="{html_module.escape(str(value), quote=True)}"'
            )
    return ''.join(encoded)


def _mutation_final_state(parser: 'WptHtmlParser') -> dict:
    """Capture static post-mutation markup before stylesheet cascading.

    The renderer builder consumes the same mutated ``DomNode`` tree.  The
    Chromium side receives a script-free serialization of that tree so the
    comparison does not depend on a browser JavaScript runtime.
    """
    scrolls = []

    def serialize(node) -> str:
        if node.is_text:
            return html_module.escape(node.text_content, quote=False)
        if node.tag == '#document-fragment':
            return ''.join(serialize(child) for child in node.children)
        extra = None
        scroll_left = float(getattr(node, 'scroll_left', 0.0))
        scroll_top = float(getattr(node, 'scroll_top', 0.0))
        if scroll_left or scroll_top:
            marker = str(len(scrolls))
            extra = {'data-openui-final-scroll': marker}
            scrolls.append({
                'marker': marker,
                'left': scroll_left,
                'top': scroll_top,
            })
        attrs = _serialize_mutation_attrs(node.attrs, node.styles, extra=extra)
        start = f'<{node.tag}{attrs}>'
        if node.tag in WptHtmlParser.VOID_TAGS:
            return start
        return start + ''.join(serialize(child) for child in node.children) + f'</{node.tag}>'

    return {
        'body_markup': ''.join(serialize(child) for child in parser.root.children),
        'body_attrs': {
            name: value for name, value in parser.root.attrs.items()
            if not re.fullmatch(r'on[a-z]+', str(name), re.IGNORECASE)
        },
        'body_style': _serialize_mutation_styles(parser.root.styles),
        'html_attrs': {
            name: value for name, value in parser.html_attrs.items()
            if not re.fullmatch(r'on[a-z]+', str(name), re.IGNORECASE)
        },
        'html_style': _serialize_mutation_styles(parser.html_styles),
        'author_style_blocks': copy.deepcopy(parser.author_style_blocks),
        'scrolls': scrolls,
    }


_TERMINAL_PSEUDO_RE = re.compile(
    r'^(.*?)'
    r'(?:::|:)('
    r'column\s*::\s*scroll-marker|'
    r'scroll-button\(\s*(?:\*|up|right|down|left|block-start|block-end|inline-start|inline-end)\s*\)|'
    r'scroll-marker-group|scroll-marker|'
    r'before|after|first-line|first-letter|marker|column|details-content'
    r')'
    r'(?::(target-current|enabled|disabled))?\s*$',
    re.IGNORECASE,
)


def _terminal_pseudo(selector: str) -> tuple[str, str, str | None] | None:
    """Return the origin selector, generated-box key, and optional state."""
    match = _TERMINAL_PSEUDO_RE.match(selector.strip())
    if not match:
        return None
    name = re.sub(r'\s+', '', match.group(2).lower())
    if name == 'column::scroll-marker':
        name = 'column-scroll-marker'
    elif name.startswith('scroll-button('):
        name = 'scroll-button-' + name[len('scroll-button('):-1]
    state = match.group(3).lower() if match.group(3) else None
    return (match.group(1).strip() or '*', name, state)


def _split_selector_list(value: str) -> list[str]:
    result = []
    current = []
    depth = 0
    bracket = 0
    quote = None
    escaped = False
    for char in value:
        if escaped:
            current.append(char)
            escaped = False
            continue
        if quote is not None:
            current.append(char)
            if char == '\\':
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in ('"', "'"):
            quote = char
            current.append(char)
        elif char == '(':
            depth += 1
            current.append(char)
        elif char == ')':
            depth = max(0, depth - 1)
            current.append(char)
        elif char == '[':
            bracket += 1
            current.append(char)
        elif char == ']':
            bracket = max(0, bracket - 1)
            current.append(char)
        elif char == ',' and depth == 0 and bracket == 0:
            if current:
                result.append(''.join(current).strip())
            current = []
        else:
            current.append(char)
    if current:
        result.append(''.join(current).strip())
    return [item for item in result if item]


def _expand_is_selectors(selector: str) -> list[str]:
    """Expand the non-forgiving `:is()` list into equivalent flat selectors."""
    match = re.search(r':is\(([^()]*)\)', selector)
    if not match:
        return [selector]
    result = []
    for alternative in _split_selector_list(match.group(1)):
        replaced = selector[:match.start()] + alternative + selector[match.end():]
        result.extend(_expand_is_selectors(replaced))
    return result


def _matching_css_brace(text: str, opening: int) -> int:
    depth = 1
    quote = None
    escaped = False
    for index in range(opening + 1, len(text)):
        char = text[index]
        if escaped:
            escaped = False
            continue
        if quote is not None:
            if char == '\\':
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in "\"'":
            quote = char
        elif char == '{':
            depth += 1
        elif char == '}':
            depth -= 1
            if depth == 0:
                return index
    return len(text)


def _resolve_nested_selectors(parent: str, nested: str) -> str:
    resolved = []
    for parent_selector in _split_selector_list(parent):
        for nested_selector in _split_selector_list(nested):
            if '&' in nested_selector:
                value = nested_selector.replace('&', parent_selector)
            else:
                value = f'{parent_selector} {nested_selector}'
            resolved.append(value.strip())
    return ', '.join(resolved)


def _flatten_css_rule(selector: str, body: str) -> list[tuple[str, str]]:
    """Flatten one CSS Nesting rule while retaining its outer declarations."""
    declarations = []
    children = []
    cursor = 0
    while True:
        opening = body.find('{', cursor)
        if opening < 0:
            declarations.append(body[cursor:])
            break
        closing = _matching_css_brace(body, opening)
        prefix = body[cursor:opening]
        boundary = prefix.rfind(';')
        if boundary >= 0:
            declarations.append(prefix[:boundary + 1])
            nested_selector = prefix[boundary + 1:].strip()
        else:
            nested_selector = prefix.strip()
        nested_body = body[opening + 1:closing]
        if nested_selector:
            if nested_selector.lower().startswith('@container'):
                children.extend(_flatten_css_rule(selector, nested_body))
            else:
                children.extend(_flatten_css_rule(
                    _resolve_nested_selectors(selector, nested_selector),
                    nested_body,
                ))
        cursor = closing + 1
    result = []
    authored = ''.join(declarations).strip()
    if authored:
        result.append((selector.strip(), authored))
    result.extend(children)
    return result


def _flatten_css_rules(css_text: str) -> list[tuple[str, str]]:
    """Return top-level and nested style rules with balanced-brace parsing."""
    result = []
    cursor = 0
    while True:
        opening = css_text.find('{', cursor)
        if opening < 0:
            break
        closing = _matching_css_brace(css_text, opening)
        selector = css_text[cursor:opening].strip()
        body = css_text[opening + 1:closing]
        lower_selector = ' '.join(selector.lower().split())
        if lower_selector.startswith('@supports'):
            condition = lower_selector[len('@supports'):].strip()
            supported_selector_features = {
                '::column',
                '::details-content',
                '::scroll-button(*)',
            }
            negated = condition.startswith('not ')
            if negated:
                condition = condition[4:].strip()
            selector_match = re.fullmatch(r'selector\((.*)\)', condition)
            supported = bool(
                selector_match
                and selector_match.group(1).strip() in supported_selector_features
            )
            enabled = not supported if negated else supported
            if enabled:
                result.extend(_flatten_css_rules(body))
        elif lower_selector.startswith(('@container', '@layer')):
            result.extend(_flatten_css_rules(body))
        elif not selector.startswith('@'):
            result.extend(_flatten_css_rule(selector, body))
        cursor = closing + 1
    return result


def _strip_top_level_statement_at_rules(css_text: str) -> str:
    """Remove non-block at-rules without consuming the following selector.

    A stylesheet may begin with ``@import`` (or ``@charset``/``@namespace``).
    Those statements have no declarations for the generated document, but
    leaving them in the balanced-brace input makes the next style rule's
    selector begin with ``@`` and causes that entire rule to be discarded.
    Scan only at brace depth zero so semicolons in declarations are retained.
    """
    statement_names = {"charset", "import", "namespace"}
    output = []
    index = 0
    brace_depth = 0
    quote = None
    escaped = False
    while index < len(css_text):
        char = css_text[index]
        if escaped:
            output.append(char)
            escaped = False
            index += 1
            continue
        if quote is not None:
            output.append(char)
            if char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            index += 1
            continue
        if char in "\"'":
            quote = char
            output.append(char)
            index += 1
            continue
        if char == "{":
            brace_depth += 1
        elif char == "}":
            brace_depth = max(0, brace_depth - 1)
        if char == "@" and brace_depth == 0:
            match = re.match(r"@([-_a-zA-Z][-_a-zA-Z0-9]*)", css_text[index:])
            if match and match.group(1).lower() in statement_names:
                cursor = index + match.end()
                local_quote = None
                local_escaped = False
                parentheses = 0
                while cursor < len(css_text):
                    current = css_text[cursor]
                    if local_escaped:
                        local_escaped = False
                    elif local_quote is not None:
                        if current == "\\":
                            local_escaped = True
                        elif current == local_quote:
                            local_quote = None
                    elif current in "\"'":
                        local_quote = current
                    elif current == "(":
                        parentheses += 1
                    elif current == ")":
                        parentheses = max(0, parentheses - 1)
                    elif current == ";" and parentheses == 0:
                        cursor += 1
                        break
                    cursor += 1
                output.append(" ")
                index = cursor
                continue
        output.append(char)
        index += 1
    return "".join(output)


def parse_simple_css_rules(css_text: str) -> list:
    """Parse simple CSS rules from a <style> block.
    Returns list of (selector, styles_dict) tuples.
    Only handles: tag selectors, .class selectors, #id selectors, and combinations.
    """
    rules = []
    # Remove comments
    css_text = re.sub(r'/\*.*?\*/', '', css_text, flags=re.DOTALL)
    # CSS tokenizes the legacy HTML wrappers as top-level CDO/CDC tokens; it
    # does not turn everything between them into a CSS comment.  Prose between
    # the tokens becomes an invalid qualified-rule prelude and consumes the
    # first following declaration block.  Mark that prelude so the compact
    # parser can discard the same block without mistaking prose for a complex
    # selector.  A traditional ``<!-- .x{} -->`` wrapper exposes its enclosed
    # CSS rule normally.
    invalid_cdo_prelude = '__OPENUI_INVALID_CDO_PRELUDE__'

    def lower_cdo(match):
        content = match.group(1)
        if '{' in content or '}' in content:
            return f' {content} '
        return f' {invalid_cdo_prelude} '

    css_text = re.sub(r'<!--(.*?)-->', lower_cdo, css_text, flags=re.DOTALL)
    # Remove CDATA wrapper
    css_text = re.sub(r'<!\[CDATA\[|\]\]>', '', css_text)
    css_text = _strip_top_level_statement_at_rules(css_text)
    # Keyframe declarations are not ordinary selector rules. The comparison
    # runner freezes document time at 0 ms; generated styles record that
    # snapshot separately instead of letting nested percentage blocks leak
    # into the flat declaration parser.
    while True:
        keyframes = re.search(r'@(?:-webkit-)?keyframes\b[^\{]*\{', css_text, re.I)
        if not keyframes:
            break
        depth = 1
        end = keyframes.end()
        while end < len(css_text) and depth:
            if css_text[end] == '{':
                depth += 1
            elif css_text[end] == '}':
                depth -= 1
            end += 1
        css_text = css_text[:keyframes.start()] + css_text[end:]
    # The pixel-comparison harness renders screen media. Declarations inside
    # print/projection media blocks must not affect generated screen styles.
    while True:
        m = re.search(r'@media\b[^{]*\{', css_text)
        if not m:
            break
        depth = 1
        i = m.end()
        while i < len(css_text) and depth:
            if css_text[i] == '{':
                depth += 1
            elif css_text[i] == '}':
                depth -= 1
            i += 1
        css_text = css_text[:m.start()] + css_text[i:]

    blocks = _flatten_css_rules(css_text)
    for selector_text, declarations in blocks:
        selector_text = selector_text.strip()
        if invalid_cdo_prelude in selector_text:
            continue
        styles = parse_inline_styles(declarations)

        # Handle comma-separated selectors without splitting inside :is(),
        # then flatten :is() into the selector subset matched below.
        selectors = []
        for selector in _split_selector_list(selector_text):
            selectors.extend(_expand_is_selectors(selector))
        for sel in selectors:
            sel = sel.strip()
            # The headless renderer has no native scrollbar widget. Chromium
            # fixtures commonly hide that widget with this vendor pseudo; the
            # corresponding rule has no generated box in OpenUI.
            if re.search(r'::-webkit-scrollbar(?:-[a-z-]+)?\b', sel, re.I):
                continue
            # The comparison snapshot has no pointer/focus interaction.
            if re.search(r':(?:hover|active|focus(?:-visible|-within)?)\b', sel):
                continue
            if sel:
                rules.append((sel, styles))

    return rules


def parse_static_keyframes(css_text: str) -> dict[str, list[tuple[float, CssDeclarations]]]:
    """Extract deterministic keyframe declarations without treating them as selectors."""
    result = {}
    source = re.sub(r'/\*.*?\*/', '', css_text, flags=re.DOTALL)
    cursor = 0
    pattern = re.compile(r'@(?:-webkit-)?keyframes\s+([-_a-zA-Z][-_a-zA-Z0-9]*)\s*\{', re.I)
    while match := pattern.search(source, cursor):
        depth = 1
        end = match.end()
        while end < len(source) and depth:
            if source[end] == '{':
                depth += 1
            elif source[end] == '}':
                depth -= 1
            end += 1
        if depth:
            break
        frames = []
        for selector, declarations in _flatten_css_rules(source[match.end():end - 1]):
            parsed = parse_inline_styles(declarations)
            for component in selector.split(','):
                token = component.strip().lower()
                if token == 'from':
                    offset = 0.0
                elif token == 'to':
                    offset = 1.0
                elif token.endswith('%'):
                    try:
                        offset = float(token[:-1]) / 100.0
                    except ValueError:
                        continue
                else:
                    continue
                frames.append((min(1.0, max(0.0, offset)), parsed))
        if frames:
            result[match.group(1)] = sorted(frames, key=lambda item: item[0])
        cursor = end
    return result


def _css_rgba8(value: str) -> tuple[int, int, int, int] | None:
    functional = re.fullmatch(
        r'rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)'
        r'(?:\s*,\s*([\d.]+))?\s*\)',
        value.strip(),
        re.I,
    )
    if functional:
        alpha = round(float(functional.group(4) or '1') * 255.0)
        return tuple(map(int, functional.groups()[:3])) + (alpha,)
    rust = parse_color(value)
    constants = {
        'Color::RED': (255, 0, 0, 255),
        'Color::GREEN': (0, 128, 0, 255),
        'Color::BLUE': (0, 0, 255, 255),
        'Color::BLACK': (0, 0, 0, 255),
        'Color::WHITE': (255, 255, 255, 255),
        'Color::TRANSPARENT': (0, 0, 0, 0),
    }
    if rust in constants:
        return constants[rust]
    match = re.fullmatch(
        r'Color::from_rgba8\((\d+),\s*(\d+),\s*(\d+),\s*(\d+)\)',
        rust or '',
    )
    return tuple(map(int, match.groups())) if match else None


def _keyframe_background_color(declarations: CssDeclarations) -> tuple[int, int, int, int] | None:
    if 'background-color' in declarations:
        return _css_rgba8(declarations['background-color'])
    background = declarations.get('background', '').strip()
    return _css_rgba8(background) if background else None


def _cubic_bezier_progress(progress: float, values: tuple[float, float, float, float]) -> float:
    x1, y1, x2, y2 = values
    def sample(t, first, second):
        return 3.0 * (1.0 - t) ** 2 * t * first + 3.0 * (1.0 - t) * t ** 2 * second + t ** 3
    low, high = 0.0, 1.0
    for _ in range(24):
        middle = (low + high) * 0.5
        if sample(middle, x1, x2) < progress:
            low = middle
        else:
            high = middle
    return sample((low + high) * 0.5, y1, y2)


def apply_static_animation_snapshot(
    styles: CssDeclarations,
    keyframes: dict[str, list[tuple[float, CssDeclarations]]],
) -> None:
    shorthand = styles.get('animation', '').strip()
    if not shorthand:
        return
    name = next((candidate for candidate in keyframes if re.search(
        rf'(?<![-_a-zA-Z0-9]){re.escape(candidate)}(?![-_a-zA-Z0-9])', shorthand
    )), None)
    if name is None:
        return
    times = re.findall(r'(?<![-_a-zA-Z0-9])(-?(?:\d+(?:\.\d*)?|\.\d+))(ms|s)\b', shorthand, re.I)
    if not times:
        return
    to_ms = lambda item: float(item[0]) * (1000.0 if item[1].lower() == 's' else 1.0)
    duration = max(0.0, to_ms(times[0]))
    delay = to_ms(times[1]) if len(times) > 1 else 0.0
    progress = 0.0 if duration == 0.0 else min(1.0, max(0.0, -delay / duration))
    timing = re.search(
        r'cubic-bezier\(\s*([+-]?[\d.]+)\s*,\s*([+-]?[\d.]+)\s*,\s*'
        r'([+-]?[\d.]+)\s*,\s*([+-]?[\d.]+)\s*\)',
        shorthand,
        re.I,
    )
    if timing:
        progress = _cubic_bezier_progress(progress, tuple(map(float, timing.groups())))
    frames = keyframes[name]
    before = max((frame for frame in frames if frame[0] <= progress), default=frames[0], key=lambda item: item[0])
    after = min((frame for frame in frames if frame[0] >= progress), default=frames[-1], key=lambda item: item[0])
    before_color = _keyframe_background_color(before[1])
    after_color = _keyframe_background_color(after[1])
    if before_color is None or after_color is None:
        return
    fraction = 0.0 if after[0] == before[0] else (progress - before[0]) / (after[0] - before[0])
    rgba = tuple(round(start + (end - start) * fraction) for start, end in zip(before_color, after_color))
    styles['background-color'] = f'rgba({rgba[0]}, {rgba[1]}, {rgba[2]}, {rgba[3] / 255.0})'


def compute_specificity(selector: str) -> tuple:
    """Compute CSS specificity as (ids, classes, elements) tuple.

    * → (0,0,0)
    tag → (0,0,1)
    .class → (0,1,0)
    #id → (1,0,0)
    Compound/descendant selectors sum their parts.
    """
    # Pseudo-elements contribute to the element column. Pseudo-classes are
    # handled in the class column below/through their arguments.
    pseudo_elements = len(re.findall(
        r'::(?:before|after|first-line|first-letter|marker|scroll-marker-group|scroll-marker|column|details-content)\b|'
        r'::scroll-button\([^)]*\)|'
        r':(?:before|after)\b', selector, re.IGNORECASE
    ))
    # Strip pseudo-classes/elements for the remaining token count.
    sel = re.sub(r':not\(([^)]*)\)', r' \1 ', selector)
    sel = re.sub(r'::?[a-zA-Z-]+', '', sel)

    ids = len(re.findall(r'#[a-zA-Z0-9_-]+', sel))
    classes = len(re.findall(r'\.[a-zA-Z0-9_-]+', sel))
    classes += len(re.findall(r'\[[^\]]+\]', sel))
    classes += len(re.findall(
        r':(?!:|before\b|after\b|not\b)[a-zA-Z-]+(?:\([^)]*\))?',
        selector,
    ))
    # Count tag names — word boundaries that aren't preceded by . or #
    tags = 0
    for part in re.findall(r'(?:^|[\s>+~])([a-zA-Z][a-zA-Z0-9]*)', sel):
        if part.lower() != 'not':
            tags += 1
    # Standalone tag at start without combinator
    m = re.match(r'^([a-zA-Z][a-zA-Z0-9]*)', sel.strip())
    if m and m.group(1).lower() != 'not':
        # Already counted if it matched the regex above, avoid double-count
        pass

    return (ids, classes, tags + pseudo_elements)


# BODY_STYLE rules mirroring run_all_pixel_comparisons.py BODY_STYLE.
# These provide the same CSS reset + body styling that Chrome sees.
BODY_STYLE_RULES = parse_simple_css_rules(
    '* { margin: 0; padding: 0; box-sizing: content-box; } '
    'body { margin: 0; padding: 20px; }'
)

# Root-aware ports keep the comparison harness reset, but deliberately do not
# force ``html { overflow:hidden }`` or an opaque body background.  Those two
# declarations erase the root/body propagation behavior these ports exercise.
ROOT_BODY_STYLE_RULES = parse_simple_css_rules(
    '* { margin: 0; padding: 0; box-sizing: content-box; } '
    'body { margin: 0; padding: 20px; font-family: DejaVu Sans, sans-serif; }'
)

# A statically lowered embedded document does not inherit the outer comparison
# harness reset. Preserve the HTML body viewport's stable UA margin; ordinary
# element UA defaults are materialized by the Rust generator below.
EMBEDDED_DOCUMENT_RULES = parse_simple_css_rules(
    'body { margin: 8px; padding: 0; box-sizing: content-box; '
    'color: black; font-size: 16px; line-height: normal; direction: ltr; '
    'writing-mode: horizontal-tb; visibility: visible; '
    'font-family: DejaVu Sans, sans-serif; }'
)


def _match_attribute_selector(expression: str, attrs: dict) -> bool:
    match = re.fullmatch(
        r'''\s*([-_a-zA-Z][-_a-zA-Z0-9]*)\s*'''
        r'''(?:(~=|\|=|\^=|\$=|\*=|=)\s*'''
        r'''(?:"([^"]*)"|'([^']*)'|([^\s]+?))\s*([isIS])?)?\s*''',
        expression,
    )
    if not match:
        return False
    name = match.group(1).lower()
    if name not in attrs:
        return False
    operator = match.group(2)
    if operator is None:
        return True
    expected = next(
        value for value in (match.group(3), match.group(4), match.group(5))
        if value is not None
    )
    actual = str(attrs[name])
    if (match.group(6) or '').lower() == 'i':
        actual = actual.casefold()
        expected = expected.casefold()
    return {
        '=': actual == expected,
        '~=': expected in actual.split(),
        '|=': actual == expected or actual.startswith(expected + '-'),
        '^=': actual.startswith(expected),
        '$=': actual.endswith(expected),
        '*=': expected in actual,
    }[operator]


def _match_simple_selector(selector: str, tag: str, classes: list, id_val: str,
                           attrs: dict = None) -> bool:
    """Check if a simple (non-compound) CSS selector matches an element."""
    attrs = attrs or {}
    for expression in re.findall(r'\[([^\]]+)\]', selector):
        if not _match_attribute_selector(expression, attrs):
            return False
    selector = re.sub(r'\[[^\]]+\]', '', selector)
    if not selector.strip():
        return True
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


def _eval_nth_expr(expr: str, index: int) -> bool:
    """Evaluate an :nth-child() expression against a 1-based sibling index.
    
    Supports: integer (e.g. '3'), 'odd', 'even', 'An+B' forms.
    """
    expr = expr.strip().lower()
    if expr == 'odd':
        return index % 2 == 1
    if expr == 'even':
        return index % 2 == 0
    # Try plain integer
    m = re.match(r'^(-?\d+)$', expr)
    if m:
        return index == int(m.group(1))
    # Try An+B form (e.g. '2n+1', '-n+3', 'n', '3n')
    m = re.match(r'^(-?\d*)n\s*([+-]\s*\d+)?$', expr)
    if m:
        a_str = m.group(1)
        if a_str in ('', '+'):
            a = 1
        elif a_str == '-':
            a = -1
        else:
            a = int(a_str)
        b = int(m.group(2).replace(' ', '')) if m.group(2) else 0
        if a == 0:
            return index == b
        # index = a*n + b for some non-negative integer n
        diff = index - b
        if a > 0:
            return diff >= 0 and diff % a == 0
        else:
            return diff <= 0 and diff % a == 0
    return False


def match_selector(selector: str, tag: str, classes: list, id_val: str,
                   ancestors: list = None, sibling_index: int = 0,
                   sibling_count: int = 0,
                   preceding_siblings: list = None, attrs: dict = None,
                   sibling_type_index: int = None,
                   sibling_type_count: int = None) -> bool:
    """Check if a CSS selector matches an element.
    
    Supports simple selectors, combinators (space, >, +, ~), and structural pseudo-classes.
    ancestors: list of (tag, classes, id_val[, attrs]) tuples from outermost to innermost.
    sibling_index: 1-based index among element siblings.
    sibling_count: total number of element siblings.
    preceding_siblings: list of (tag, classes, id_val) for preceding element siblings.
    """
    selector = selector.strip()
    original_selector = selector
    if not selector:
        return False
    # Keep the selector API compatible with the historical three-field DOM
    # tuples while allowing SP19 attribute selectors to carry a fourth field.
    # Normalizing once also keeps recursive selector matching consistent.
    ancestors = [
        (*entry, {}, []) if len(entry) == 3 else
        (*entry, []) if len(entry) == 4 else entry
        for entry in (ancestors or [])
    ]
    preceding_siblings = [
        (*entry, {}) if len(entry) == 3 else entry
        for entry in (preceding_siblings or [])
    ]
    type_index = sibling_index if sibling_type_index is None else sibling_type_index
    type_count = sibling_count if sibling_type_count is None else sibling_type_count

    # Evaluate :not() pseudo-class — handle both simple selectors and
    # structural pseudo-classes inside :not(). Use regex that handles
    # one level of nested parens (e.g. :not(:nth-child(2))).
    for m in re.finditer(r':not\(([^()]*(?:\([^)]*\)[^()]*)*)\)', selector):
        inner = m.group(1).strip()
        # Check structural pseudo-classes inside :not()
        if inner == ':first-child':
            if sibling_index == 1:
                return False
            continue
        if inner in (':last-child', ':last-of-type'):
            index, count = (
                (type_index, type_count)
                if inner == ':last-of-type' else (sibling_index, sibling_count)
            )
            if index == count:
                return False
            continue
        if inner in (':first-of-type',):
            if type_index == 1:
                return False
            continue
        if inner == ':only-of-type':
            if type_count == 1:
                return False
            continue
        if inner == ':only-child':
            if sibling_count == 1:
                return False
            continue
        nth_m = re.match(r':nth-child\(([^)]+)\)', inner)
        if nth_m:
            if _eval_nth_expr(nth_m.group(1), sibling_index):
                return False
            continue
        # Simple selector (tag, class, id)
        if _match_simple_selector(inner, tag, classes, id_val, attrs):
            return False

    # Evaluate bare structural pseudo-classes (not inside :not())
    # Strip :not(...) content first to avoid double-matching
    bare_selector = re.sub(r':not\([^()]*(?:\([^)]*\)[^()]*)*\)', '', selector)
    if ':root' in bare_selector and ancestors:
        return False
    if ':first-child' in bare_selector and sibling_index != 1:
        return False
    if ':last-child' in bare_selector and sibling_index != sibling_count:
        return False
    if ':last-of-type' in bare_selector and type_index != type_count:
        return False
    if ':first-of-type' in bare_selector and type_index != 1:
        return False
    if ':only-of-type' in bare_selector and type_count != 1:
        return False
    if ':only-child' in bare_selector and sibling_count != 1:
        return False
    for nth_m in re.finditer(r':nth-child\(([^)]+)\)', bare_selector):
        if not _eval_nth_expr(nth_m.group(1), sibling_index):
            return False

    # Strip pseudo-classes after evaluation (including nested-paren :not())
    selector = re.sub(r':not\([^()]*(?:\([^)]*\)[^()]*)*\)', '', selector)
    selector = re.sub(r':(?:root|first-child|last-child|first-of-type|last-of-type|only-of-type|nth-child\([^)]+\)|only-child|empty)', '', selector)
    stripped_with_combinators = selector
    selector = selector.strip()

    if not selector:
        return True

    # Selectors such as ".flexbox > :nth-child(1)" target the current
    # element solely via a structural pseudo-class. After evaluating and
    # stripping the pseudo-class above, only the ancestor-side selector remains.
    # Match that remaining selector against the appropriate ancestor rather than
    # incorrectly requiring the current element to match ".flexbox".
    if re.search(r'>\s*$', stripped_with_combinators):
        prefix = re.sub(r'>\s*$', '', stripped_with_combinators).strip()
        if not prefix or not ancestors:
            return False
        parent_tag, parent_classes, parent_id, parent_attrs, parent_preceding = ancestors[-1]
        return match_selector(
            prefix,
            parent_tag,
            parent_classes,
            parent_id,
            ancestors[:-1],
            1,
            1,
            parent_preceding,
            parent_attrs,
        )

    pseudo_only_descendant = re.match(
        r'^(.*?)\s+:(?:root|first-child|last-child|first-of-type|last-of-type|only-of-type|nth-child\([^)]+\)|only-child|empty|not\([^)]*\))\s*$',
        original_selector,
    )
    if pseudo_only_descendant:
        prefix = pseudo_only_descendant.group(1).strip()
        if not prefix or not ancestors:
            return False
        for idx in range(len(ancestors) - 1, -1, -1):
            anc_tag, anc_classes, anc_id, anc_attrs, anc_preceding = ancestors[idx]
            if match_selector(
                prefix, anc_tag, anc_classes, anc_id, ancestors[:idx], 1, 1,
                anc_preceding, anc_attrs
            ):
                return True
        return False

    # Tokenize: split on combinators (>, +, ~, whitespace) preserving type.
    # Protect attribute selectors first: their operators and quoted whitespace
    # are part of a compound selector, not combinators.
    tokens = []
    combinators = []
    protected_attributes = []

    def protect_attribute(match):
        protected_attributes.append(match.group(0))
        return f'__OPENUI_ATTR_{len(protected_attributes) - 1}__'

    protected_selector = re.sub(r'\[[^\]]+\]', protect_attribute, selector)
    normalized = re.sub(r'\s*([>+~])\s*', r' \1 ', protected_selector).strip()
    parts = normalized.split()

    def restore_attributes(part):
        for index, attribute in enumerate(protected_attributes):
            part = part.replace(f'__OPENUI_ATTR_{index}__', attribute)
        return part

    current_parts = []
    for p in parts:
        if p in ('>', '+', '~'):
            if current_parts:
                tokens.append(' '.join(current_parts))
                current_parts = []
            combinators.append({'>' : 'child', '+': 'adjacent', '~': 'general'}[p])
        else:
            if current_parts:
                tokens.append(' '.join(current_parts))
                combinators.append('descendant')
                current_parts = []
            current_parts.append(restore_attributes(p))
    if current_parts:
        tokens.append(' '.join(current_parts))

    if len(tokens) == 1:
        return _match_simple_selector(tokens[0], tag, classes, id_val, attrs)

    # Last token must match current element
    if not _match_simple_selector(tokens[-1], tag, classes, id_val, attrs):
        return False

    # Process remaining tokens right-to-left
    remaining_tokens = tokens[:-1]
    ri = len(remaining_tokens) - 1
    # combinators[i] connects tokens[i] to tokens[i+1]
    # Consume the complete rightmost sibling chain.  The previous
    # implementation stopped after one combinator, so `A + B + C` was treated
    # like `B + C` and lost the more-specific declaration in the cascade.
    sibling_cursor = len(preceding_siblings or []) - 1
    while ri >= 0:
        combinator = combinators[ri] if ri < len(combinators) else 'descendant'
        if combinator not in ('adjacent', 'general'):
            break
        if sibling_cursor < 0:
            return False
        if combinator == 'adjacent':
            prev = preceding_siblings[sibling_cursor]
            if not _match_simple_selector(
                    remaining_tokens[ri], prev[0], prev[1], prev[2], prev[3]):
                return False
            sibling_cursor -= 1
        else:
            matched_at = None
            for index in range(sibling_cursor, -1, -1):
                prev = preceding_siblings[index]
                if _match_simple_selector(
                        remaining_tokens[ri], prev[0], prev[1], prev[2], prev[3]):
                    matched_at = index
                    break
            if matched_at is None:
                return False
            sibling_cursor = matched_at - 1
        ri -= 1
    if ri < 0:
        return True

    if not ancestors:
        return ri < 0

    # Walk ancestor list matching remaining tokens with descendant/child combinators
    for anc_tag, anc_classes, anc_id, anc_attrs, anc_preceding in reversed(ancestors):
        if ri < 0:
            break
        comb = combinators[ri] if ri < len(combinators) else 'descendant'
        if _match_simple_selector(
            remaining_tokens[ri], anc_tag, anc_classes, anc_id, anc_attrs
        ):
            ri -= 1
            # A sibling combinator may connect the matched ancestor to the
            # next token on the left (for example ``p ~ div span``).  The
            # sibling list belongs to that ancestor, not to the current
            # element, so consume the complete chain here before continuing
            # upward through common ancestors.
            sibling_cursor = len(anc_preceding) - 1
            while ri >= 0 and combinators[ri] in ('adjacent', 'general'):
                if sibling_cursor < 0:
                    return False
                sibling_comb = combinators[ri]
                if sibling_comb == 'adjacent':
                    prev = anc_preceding[sibling_cursor]
                    if not _match_simple_selector(
                        remaining_tokens[ri], prev[0], prev[1], prev[2], prev[3]
                    ):
                        return False
                    sibling_cursor -= 1
                else:
                    matched_at = None
                    for index in range(sibling_cursor, -1, -1):
                        prev = anc_preceding[index]
                        if _match_simple_selector(
                            remaining_tokens[ri], prev[0], prev[1], prev[2], prev[3]
                        ):
                            matched_at = index
                            break
                    if matched_at is None:
                        return False
                    sibling_cursor = matched_at - 1
                ri -= 1
        elif comb == 'child':
            return False

    return ri < 0


def apply_css_rules(rules: list, node: 'DomNode', ancestors: list = None,
                    sibling_index: int = 1, sibling_count: int = 1,
                    preceding_siblings: list = None,
                    leading_element_siblings: int = 0,
                    trailing_element_siblings: int = 0,
                    sibling_type_index: int = 1,
                    sibling_type_count: int = 1):
    """Apply CSS rules to a DOM node and its descendants (recursively).

    CSS cascade: later rules override earlier rules for the same property.
    Inline styles (already in node.styles) take highest precedence.
    sibling_index: 1-based index among element siblings.
    sibling_count: total number of element siblings.
    preceding_siblings: list of (tag, classes, id_val, attrs) for preceding element siblings.
    """
    if node.is_text:
        return

    if ancestors is None:
        ancestors = []

    classes = node.attrs.get('class', '').split()
    id_val = node.attrs.get('id', '')

    # Save inline styles (highest precedence)
    inline_styles = _copy_declarations(node.styles)

    # Apply stylesheet rules respecting specificity.
    # Higher-specificity rules win; within same specificity, later wins.
    cascade = CssDeclarations()   # prop -> value plus winning priority
    cascade_priority = {}

    # HTML dir is an author-origin presentational hint with zero specificity.
    # It precedes stylesheet declarations, so any matching author rule wins.
    dir_hint = node.attrs.get('dir', '').strip().lower()
    if dir_hint in {'ltr', 'rtl'}:
        hint_priority = (0, 0, 0, 0, 0, -1, -1)
        _set_declaration(
            cascade, 'direction', dir_hint, priority=hint_priority
        )
        _set_declaration(
            cascade, 'unicode-bidi', 'isolate', priority=hint_priority
        )
        cascade_priority['direction'] = hint_priority
        cascade_priority['unicode-bidi'] = hint_priority

    for rule_index, (selector, styles) in enumerate(rules):
        pseudo_match = _terminal_pseudo(selector)
        match_target = pseudo_match[0] if pseudo_match else selector
        if match_selector(match_target, node.tag, classes, id_val, ancestors,
                          sibling_index, sibling_count, preceding_siblings,
                          node.attrs, sibling_type_index, sibling_type_count):
            spec = compute_specificity(selector)
            if pseudo_match:
                wildcard_scroll_button = pseudo_match[1] == 'scroll-button-*'
                pseudo_names = [pseudo_match[1]]
                if wildcard_scroll_button:
                    # The wildcard styles every scroll-button pseudo. It does
                    # not create a box by itself: directions without authored
                    # `content` remain unmaterialized below.
                    pseudo_names = [
                        'scroll-button-up', 'scroll-button-right',
                        'scroll-button-down', 'scroll-button-left',
                        'scroll-button-block-start', 'scroll-button-block-end',
                        'scroll-button-inline-start', 'scroll-button-inline-end',
                    ]
                for pseudo_name in pseudo_names:
                    if pseudo_match[2] == 'target-current':
                        state_name = f'{pseudo_name}-target-current'
                        if state_name in node.pseudo_styles:
                            pseudo_name = state_name
                    pseudo_cascade = node.pseudo_styles[pseudo_name]
                    pseudo_priorities = node.pseudo_priorities[pseudo_name]
                    for declaration_index, (prop, val) in enumerate(styles.items()):
                        if wildcard_scroll_button and prop == 'content' and pseudo_name in {
                            'scroll-button-block-start', 'scroll-button-block-end',
                            'scroll-button-inline-start', 'scroll-button-inline-end',
                        }:
                            # A wildcard-only rule generates the four physical
                            # buttons. Its non-content declarations still
                            # cascade onto separately-created logical buttons.
                            continue
                        important = (
                            styles.important.get(prop, False)
                            if isinstance(styles, CssDeclarations) else False
                        )
                        if isinstance(styles, CssDeclarations):
                            declaration_index = styles.cascade_priority.get(
                                prop, (0, 0, 0, 0, 0, 0, declaration_index)
                            )[-1]
                        priority = (
                            int(important), 0, spec[0], spec[1], spec[2],
                            rule_index, declaration_index,
                        )
                        if prop not in pseudo_priorities or priority >= pseudo_priorities[prop]:
                            _set_declaration(
                                pseudo_cascade, prop, val, priority=priority,
                                important=important,
                            )
                            pseudo_priorities[prop] = priority
                continue
            for declaration_index, (prop, val) in enumerate(styles.items()):
                important = (
                    styles.important.get(prop, False)
                    if isinstance(styles, CssDeclarations) else False
                )
                if isinstance(styles, CssDeclarations):
                    declaration_index = styles.cascade_priority.get(
                        prop, (0, 0, 0, 0, 0, 0, declaration_index)
                    )[-1]
                priority = (
                    int(important), 0, spec[0], spec[1], spec[2],
                    rule_index, declaration_index,
                )
                if prop not in cascade_priority or priority >= cascade_priority[prop]:
                    _set_declaration(
                        cascade, prop, val, priority=priority, important=important
                    )
                    cascade_priority[prop] = priority

    # Inline styles override stylesheet rules (highest precedence)
    for declaration_index, (prop, val) in enumerate(inline_styles.items()):
        important = inline_styles.important.get(prop, False)
        declaration_index = inline_styles.cascade_priority.get(
            prop, (0, 0, 0, 0, 0, 0, declaration_index)
        )[-1]
        priority = (
            int(important), 1, 0, 0, 0, len(rules), declaration_index,
        )
        if prop not in cascade_priority or priority >= cascade_priority[prop]:
            _set_declaration(
                cascade, prop, val, priority=priority, important=important
            )
            cascade_priority[prop] = priority
    node.styles = cascade

    child_ancestors = ancestors + [(
        node.tag, classes, id_val, node.attrs, list(preceding_siblings or [])
    )]
    # Compute sibling indices and preceding siblings for element children
    element_children = [c for c in node.children if not c.is_text]
    total_elements = (
        leading_element_siblings + len(element_children) + trailing_element_siblings
    )
    type_counts = {}
    for child in element_children:
        type_counts[child.tag] = type_counts.get(child.tag, 0) + 1
    type_seen = {}
    elem_idx = leading_element_siblings
    preceding = [
        ('style', [], '', {'style': 'display:none!important'})
        for _ in range(leading_element_siblings)
    ]
    for child in node.children:
        if not child.is_text:
            elem_idx += 1
            type_seen[child.tag] = type_seen.get(child.tag, 0) + 1
            child_classes = child.attrs.get('class', '').split()
            child_id = child.attrs.get('id', '')
            apply_css_rules(rules, child, child_ancestors, elem_idx, total_elements,
                            list(preceding), sibling_type_index=type_seen[child.tag],
                            sibling_type_count=type_counts[child.tag])
            preceding.append((child.tag, child_classes, child_id, child.attrs))
        else:
            apply_css_rules(rules, child, child_ancestors)


def apply_static_target_current_rules(
    rules: list,
    root: 'DomNode',
    ancestors: list = None,
    leading_element_siblings: int = 0,
    trailing_element_siblings: int = 0,
):
    """Resolve the initial, zero-scroll ``:target-current`` state.

    A scroll target group selects one of its descendant links.  The SP19
    comparison contract freezes scroll offsets at zero, so the first link in
    document order is the deterministic current target.  This second cascade
    phase runs only after ``scroll-target-group`` itself has computed and keeps
    the original rule indices/specificities, including normal ``!important``
    and inline-style precedence.
    """
    target_rules = [
        (rule_index, selector, styles)
        for rule_index, (selector, styles) in enumerate(rules)
        if ':target-current' in selector.lower() and _terminal_pseudo(selector) is None
    ]
    if not target_rules:
        return

    current_nodes = set()

    def first_target_link(node):
        for child in node.children:
            if child.is_text:
                continue
            if child.tag == 'a' and 'href' in child.attrs:
                return child
            match = first_target_link(child)
            if match is not None:
                return match
        return None

    def find_groups(node):
        if node.is_text:
            return
        if node.styles.get('scroll-target-group', '').strip().lower() == 'auto':
            current = first_target_link(node)
            if current is not None:
                current_nodes.add(id(current))
        for child in node.children:
            find_groups(child)

    find_groups(root)
    if not current_nodes:
        return

    def cascade(node, node_ancestors, sibling_index=1, sibling_count=1,
                preceding_siblings=None, sibling_type_index=1,
                sibling_type_count=1, leading_siblings=0, trailing_siblings=0):
        if node.is_text:
            return
        classes = node.attrs.get('class', '').split()
        id_val = node.attrs.get('id', '')
        if id(node) in current_nodes:
            for rule_index, selector, styles in target_rules:
                base_selector = re.sub(
                    r':target-current\b', '', selector, flags=re.IGNORECASE
                )
                if not match_selector(
                    base_selector, node.tag, classes, id_val, node_ancestors,
                    sibling_index, sibling_count, preceding_siblings,
                    node.attrs, sibling_type_index, sibling_type_count,
                ):
                    continue
                spec = compute_specificity(selector)
                for declaration_index, (prop, val) in enumerate(styles.items()):
                    important = (
                        styles.important.get(prop, False)
                        if isinstance(styles, CssDeclarations) else False
                    )
                    if isinstance(styles, CssDeclarations):
                        declaration_index = styles.cascade_priority.get(
                            prop, (0, 0, 0, 0, 0, 0, declaration_index)
                        )[-1]
                    priority = (
                        int(important), 0, spec[0], spec[1], spec[2],
                        rule_index, declaration_index,
                    )
                    old_priority = node.styles.cascade_priority.get(prop)
                    if old_priority is None or priority >= old_priority:
                        _set_declaration(
                            node.styles, prop, val, priority=priority,
                            important=important,
                        )

        child_ancestors = node_ancestors + [
            (node.tag, classes, id_val, node.attrs)
        ]
        element_children = [child for child in node.children if not child.is_text]
        total_elements = leading_siblings + len(element_children) + trailing_siblings
        type_counts = {}
        for child in element_children:
            type_counts[child.tag] = type_counts.get(child.tag, 0) + 1
        type_seen = {}
        preceding = [
            ('style', [], '', {'style': 'display:none!important'})
            for _ in range(leading_siblings)
        ]
        element_index = leading_siblings
        for child in node.children:
            if child.is_text:
                continue
            element_index += 1
            type_seen[child.tag] = type_seen.get(child.tag, 0) + 1
            cascade(
                child, child_ancestors, element_index, total_elements,
                list(preceding), type_seen[child.tag], type_counts[child.tag],
            )
            preceding.append((
                child.tag,
                child.attrs.get('class', '').split(),
                child.attrs.get('id', ''),
                child.attrs,
            ))

    cascade(
        root,
        ancestors or [],
        leading_siblings=leading_element_siblings,
        trailing_siblings=trailing_element_siblings,
    )


class WptHtmlParser(HTMLParser):
    """Parse WPT HTML into a DOM tree, extracting only body content."""

    SKIP_TAGS = {'head', 'link', 'meta', 'title', 'script', 'noscript'}
    LAYOUT_TAGS = {'div', 'span', 'p', 'section', 'article', 'main', 'header',
                   'footer', 'nav', 'aside', 'figure', 'figcaption', 'br',
                   'strong', 'em', 'b', 'i', 'u', 'a', 'img', 'table',
                   'caption', 'colgroup', 'col', 'thead', 'tbody', 'tfoot',
                   'tr', 'td', 'th', 'canvas', 'svg', 'iframe', 'object',
                   'audio', 'video', 'input', 'button', 'meter', 'fieldset',
                   'legend', 'textarea', 'select', 'option', 'optgroup',
                   'form', 'embed'}

    def __init__(
        self,
        *,
        root_aware: bool = False,
        harness_rules: list | None = None,
        mutation_ir: dict | None = None,
    ):
        super().__init__()
        self.root_aware = root_aware
        self.harness_rules = harness_rules
        self.mutation_ir = mutation_ir
        self.lowered_layout_barriers = []
        self.lowered_final_state = None
        self.root = DomNode('body', {}, {})
        self.stack = [self.root]
        self.in_body = False
        self.skip_depth = 0
        self.in_style = False
        self.style_content = ""
        self.current_style_attrs = {}
        self.current_style_prefix = ""
        self.pending_pre_body_whitespace = ""
        self.author_style_blocks = []
        self.has_script = False
        self.script_elements = []
        self.current_script = None
        self.event_handlers = []
        self.has_style_block = False
        self.css_rules = []  # Parsed CSS rules from <style>
        self.external_css_rules = []  # Parsed CSS rules from external stylesheets
        self.keyframes = {}
        self.external_stylesheet_hrefs = []  # hrefs of <link rel="stylesheet">
        self.all_styles = []  # all style dicts encountered
        self.ref_path = None
        self.html_dir = ''  # Set by parse_wpt_html for resolving relative paths
        self.html_styles = CssDeclarations()  # styles applied to <html> (root element)
        self.html_attrs = {}
        self.html_pseudo_styles = DomNode('html', {}, {}).pseudo_styles
        self.document_closed = False

    # Void elements that never have closing tags
    VOID_TAGS = {'link', 'meta', 'br', 'hr', 'img', 'input', 'col', 'area',
                 'base', 'bgsound', 'embed', 'param', 'source', 'track', 'wbr'}

    # Starting one of these elements implicitly closes an open paragraph in
    # the HTML tree builder.  Python's HTMLParser is only a tokenizer and does
    # not perform that repair itself, so without it valid WPT markup such as
    # ``<p>instructions<div id=test>`` incorrectly nests the entire test in
    # the instructional paragraph that the porter later prunes.
    P_IMPLICIT_END_TAGS = {
        'address', 'article', 'aside', 'blockquote', 'div', 'dl', 'fieldset',
        'footer', 'form', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'header',
        'hgroup', 'hr', 'main', 'menu', 'nav', 'ol', 'p', 'pre', 'section',
        'table', 'ul',
    }

    def _close_open_paragraph(self):
        for index in range(len(self.stack) - 1, 0, -1):
            if self.stack[index].tag == 'p':
                del self.stack[index:]
                return

    def _close_nearest(self, tags: set[str]) -> bool:
        for index in range(len(self.stack) - 1, 0, -1):
            if self.stack[index].tag in tags:
                del self.stack[index:]
                return True
            if self.stack[index].tag == 'table':
                break
        return False

    def _insert_implied_table_element(self, tag: str) -> None:
        node = DomNode(tag, {}, CssDeclarations())
        self.stack[-1].children.append(node)
        self.stack.append(node)

    def _table_foster_parent(self):
        """Return the HTML foster-parent insertion point, when active.

        Python's ``HTMLParser`` tokenizes markup without the tree builder's
        "in table" insertion modes. Character data and ordinary elements
        directly inside table structure therefore need to be inserted just
        before the nearest table, matching the browser DOM that CSS sees.
        Content inside a cell or caption uses the normal insertion mode.
        """
        table_index = None
        for index in range(len(self.stack) - 1, 0, -1):
            if self.stack[index].tag == 'table':
                table_index = index
                break
        if table_index is None:
            return None
        current = self.stack[-1].tag
        if current not in {
            'table', 'thead', 'tbody', 'tfoot', 'tr', 'colgroup',
        }:
            return None
        parent = self.stack[table_index - 1]
        table = self.stack[table_index]
        return parent, table

    def _append_with_table_foster_parenting(self, node) -> None:
        target = self._table_foster_parent()
        if target is None:
            self.stack[-1].children.append(node)
            return
        parent, table = target
        parent.children.insert(parent.children.index(table), node)

    def _fixup_table_start(self, tag: str) -> None:
        """Apply the optional-end-tag subset used by static table WPTs."""
        if tag in ('td', 'th'):
            self._close_nearest({'td', 'th'})
            if self.stack[-1].tag in ('table', 'thead', 'tbody', 'tfoot'):
                self._insert_implied_table_element('tr')
            return
        if tag == 'tr':
            self._close_nearest({'td', 'th'})
            self._close_nearest({'tr'})
            if self.stack[-1].tag == 'table':
                self._insert_implied_table_element('tbody')
            return
        if tag in ('thead', 'tbody', 'tfoot'):
            self._close_nearest({'td', 'th'})
            self._close_nearest({'tr'})
            self._close_nearest({'thead', 'tbody', 'tfoot'})
            return
        if tag in ('caption', 'colgroup'):
            self._close_nearest({'caption', 'colgroup'})
            return
        if tag == 'col' and self.stack[-1].tag == 'table':
            self._insert_implied_table_element('colgroup')

    def handle_starttag(self, tag, attrs):
        if self.document_closed:
            return
        # The HTML tokenizer keeps the first occurrence of a duplicate
        # attribute and drops later occurrences. ``dict(attrs)`` did the
        # reverse, which changes selector matching and generated pseudo boxes.
        attrs_dict = {}
        for name, value in attrs:
            attrs_dict.setdefault(name, value)

        for name, value in attrs:
            if re.fullmatch(r'on[a-z]+', name, re.IGNORECASE):
                self.event_handlers.append((tag, name.lower(), value or ''))

        # Track reference
        if tag == 'link' and attrs_dict.get('rel') == 'match':
            self.ref_path = attrs_dict.get('href', '')

        # Track external stylesheets
        if tag == 'link' and attrs_dict.get('rel') == 'stylesheet':
            href = attrs_dict.get('href', '')
            if href:
                self.external_stylesheet_hrefs.append(href)

        if tag == 'script':
            self.has_script = True
            self.current_script = {
                'attrs': attrs_dict,
                'content': '',
            }
            self.script_elements.append(self.current_script)
            self.skip_depth += 1
            return

        if tag == 'style':
            self.has_style_block = True
            self.in_style = True
            self.style_content = ""
            self.current_style_attrs = attrs_dict
            self.current_style_prefix = self.pending_pre_body_whitespace
            self.pending_pre_body_whitespace = ""
            return

        if tag in ('head', 'html', 'body'):
            if tag == 'html':
                self.html_attrs = attrs_dict
                if 'style' in attrs_dict:
                    self.html_styles = parse_inline_styles(attrs_dict['style'])
            if tag == 'body':
                self.in_body = True
                self.root.attrs = attrs_dict
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

        if tag in self.P_IMPLICIT_END_TAGS:
            self._close_open_paragraph()

        if tag in {
            'caption', 'colgroup', 'col', 'thead', 'tbody', 'tfoot',
            'tr', 'td', 'th',
        }:
            # The HTML tree builder ignores table-structure start tags that
            # occur without a table element in scope, while retaining their
            # character data in the current parent. This matters for flexbox
            # fixup tests whose source contains orphan `<td>`/`<tbody>` tags.
            if not any(node.tag == 'table' for node in self.stack[1:]):
                return
            self._fixup_table_start(tag)

        styles = parse_inline_styles(attrs_dict.get('style', ''))
        # HTML's legacy ``clear=all`` spelling computes to ``clear:both``.
        # Preserve it as style so the semantic Break node participates in the
        # same float-clearance path as author CSS on ``br``.
        if tag == 'br' and attrs_dict.get('clear', '').strip().lower() in (
            'all', 'both', 'left', 'right'
        ):
            clear = attrs_dict['clear'].strip().lower()
            styles['clear'] = 'both' if clear == 'all' else clear
        self.all_styles.append(styles)

        node = DomNode(tag, attrs_dict, styles)
        table_structure = {
            'caption', 'colgroup', 'col', 'thead', 'tbody', 'tfoot',
            'tr', 'td', 'th',
        }
        if tag in table_structure:
            self.stack[-1].children.append(node)
        else:
            self._append_with_table_foster_parenting(node)
        if tag not in self.VOID_TAGS:
            self.stack.append(node)

    def handle_startendtag(self, tag, attrs):
        """Handle self-closing tags like <div/>.

        Many WPT tests are XHTML (.xhtml) where <div/> is genuinely
        self-closing.  Treat all self-closing tags as open+close so that
        sibling elements remain siblings instead of being incorrectly nested.
        """
        self.handle_starttag(tag, attrs)
        if tag not in self.VOID_TAGS:
            self.handle_endtag(tag)

    def handle_endtag(self, tag):
        if self.document_closed:
            return
        if tag == 'style':
            self.in_style = False
            self.css_rules.extend(parse_simple_css_rules(self.style_content))
            self.keyframes.update(parse_static_keyframes(self.style_content))
            self.author_style_blocks.append(
                (
                    dict(self.current_style_attrs),
                    self.style_content,
                    self.current_style_prefix,
                )
            )
            self.current_style_attrs = {}
            self.current_style_prefix = ""
            return
        if tag == 'script':
            self.current_script = None
            if self.skip_depth > 0:
                self.skip_depth -= 1
            return
        if tag == 'html':
            self.document_closed = True
            return
        if tag in ('head', 'body'):
            return
        if self.skip_depth > 0:
            self.skip_depth -= 1
            return
        # HTML §13.2.6.4.7: an end tag for `p` with no paragraph in button
        # scope first inserts an empty paragraph, then closes it.  This is
        # observable in layout because the implied block splits the
        # surrounding inline formatting context.
        if tag == 'p' and not any(node.tag == 'p' for node in self.stack[1:]):
            self.handle_starttag('p', [])
        # Pop through the matching element.  A blind single pop corrupts the
        # tree after an implicit paragraph close or any other recoverable HTML
        # end-tag mismatch.
        for index in range(len(self.stack) - 1, 0, -1):
            if self.stack[index].tag == tag:
                del self.stack[index:]
                break

    def handle_data(self, data):
        if self.document_closed:
            return
        if self.in_style:
            self.style_content += data
            return
        if self.current_script is not None:
            self.current_script['content'] += data
            return
        if self.skip_depth > 0:
            return
        if RETAIN_TEXT:
            # Preserve source whitespace verbatim. The inline engine applies
            # the effective `white-space` mode to the Text node, just as Chrome
            # does; eagerly collapsing here would corrupt pre/pre-wrap text.
            # Whitespace-only nodes are contextually filtered during generation
            # so source indentation between blocks cannot create line boxes.
            # Non-whitespace character data after head metadata implicitly
            # opens HTML's body even when the source omits a <body> tag.
            if not self.in_body:
                if data and data.strip():
                    self.in_body = True
                else:
                    self.pending_pre_body_whitespace += data
                    return
            if data and self.in_body:
                node = DomNode('#text', {}, {})
                node.is_text = True
                node.text_content = data
                if data.strip(' \t\n\r\f'):
                    self._append_with_table_foster_parenting(node)
                else:
                    self.stack[-1].children.append(node)
            return
        # HTML's collapsible source whitespace excludes U+00A0.  Python's
        # generic `str.strip()` includes it, which erased a layout-bearing
        # non-breaking space in the legacy box profile.
        text = data.strip(' \t\n\r\f')
        if text and self.in_body:
            node = DomNode('#text', {}, {})
            node.is_text = True
            node.text_content = text
            self._append_with_table_foster_parenting(node)

    def handle_comment(self, data):
        pass

    def finalize(self):
        """Apply CSS rules to the DOM tree after parsing.
        BODY_STYLE_RULES come first (matching Chrome's BODY_STYLE wrapper),
        then external stylesheet rules, then inline <style> rules.
        Specificity-based cascade ensures body { padding:20px } beats * { padding:0 }."""
        if self.mutation_ir is not None:
            _apply_ast_mutation_ir(self)
            self.lowered_final_state = _mutation_final_state(self)
        operations = _static_onload_operations(self) if self.mutation_ir is None else None
        if operations is not None:
            for operation, _node_id, _name, value in operations:
                if operation == 'class':
                    authored = self.root.attrs.get('class', '').split()
                    if value not in authored:
                        authored.append(value)
                    self.root.attrs['class'] = ' '.join(authored)

        harness_rules = self.harness_rules
        if harness_rules is None:
            harness_rules = ROOT_BODY_STYLE_RULES if self.root_aware else BODY_STYLE_RULES
        all_rules = harness_rules + self.external_css_rules + self.css_rules
        # Keep the Rust builder's pruning decisions synchronized with the
        # Chromium template pass.  A universal rule does not turn an empty,
        # otherwise unstyled instructional heading into retained structure;
        # an explicit tag selector does.
        self.css_targeted_tags = set()
        for selector, _ in self.external_css_rules + self.css_rules:
            for match in re.finditer(
                r'(?:^|[\s>+~])([a-zA-Z][a-zA-Z0-9-]*)', selector
            ):
                self.css_targeted_tags.add(match.group(1).lower())
        self.root.css_targeted_tags = set(self.css_targeted_tags)
        if all_rules:
            # In root-aware mode the parsed tree starts at <body>, but CSS
            # selectors still see the real <html> ancestor. Seeding that
            # ancestor prevents :root rules from being misapplied to body and
            # lets html-descendant selectors retain their browser semantics.
            root_ancestors = [('html', [], '', self.html_attrs)] if self.root_aware else None
            # The comparison template appends a hidden deterministic-font
            # stylesheet after the source body. It is absent from the Rust
            # DOM but still participates in structural selector matching in
            # Chromium, so account for that trailing element when cascading
            # over the source body children.
            # The deterministic-font stylesheet is appended after the source
            # markup.  In malformed HTML the browser inserts it into the
            # still-open source element rather than as a direct body child;
            # only count it for body-child structural selectors when the
            # tokenizer stack has actually returned to the synthetic body.
            trailing_harness_style = int(
                RETAIN_TEXT and not is_real_font_profile() and len(self.stack) == 1
            )
            leading_author_styles = len(self.author_style_blocks)
            apply_css_rules(
                all_rules,
                self.root,
                ancestors=root_ancestors,
                leading_element_siblings=leading_author_styles,
                trailing_element_siblings=trailing_harness_style,
            )
            apply_static_target_current_rules(
                all_rules,
                self.root,
                ancestors=root_ancestors,
                leading_element_siblings=leading_author_styles,
                trailing_element_siblings=trailing_harness_style,
            )
            def sample_animations(node):
                if node.is_text:
                    return
                apply_static_animation_snapshot(node.styles, self.keyframes)
                for child in node.children:
                    sample_animations(child)

            sample_animations(self.root)
            # Apply html-targeted rules to self.html_styles for body-bg propagation logic.
            html_cascade = CssDeclarations()
            html_cascade_priority = {}
            html_dir = self.html_attrs.get('dir', '').strip().lower()
            if html_dir in {'ltr', 'rtl'}:
                hint_priority = (0, 0, 0, 0, 0, -1, -1)
                _set_declaration(
                    html_cascade, 'direction', html_dir, priority=hint_priority
                )
                _set_declaration(
                    html_cascade, 'unicode-bidi', 'isolate', priority=hint_priority
                )
                html_cascade_priority['direction'] = hint_priority
                html_cascade_priority['unicode-bidi'] = hint_priority
            for rule_index, (selector, styles) in enumerate(all_rules):
                # Match selectors targeting the html root element only.
                html_classes = self.html_attrs.get('class', '').split()
                html_id = self.html_attrs.get('id', '')
                if match_selector(
                    selector,
                    'html',
                    html_classes,
                    html_id,
                    ancestors=[],
                    sibling_index=1,
                    sibling_count=1,
                    preceding_siblings=[],
                    attrs=self.html_attrs,
                ):
                    spec = compute_specificity(selector)
                    for declaration_index, (prop, val) in enumerate(styles.items()):
                        important = (
                            styles.important.get(prop, False)
                            if isinstance(styles, CssDeclarations) else False
                        )
                        if isinstance(styles, CssDeclarations):
                            declaration_index = styles.cascade_priority.get(
                                prop, (0, 0, 0, 0, 0, 0, declaration_index)
                            )[-1]
                        priority = (
                            int(important), 0, spec[0], spec[1], spec[2],
                            rule_index, declaration_index,
                        )
                        if (
                            prop not in html_cascade_priority
                            or priority >= html_cascade_priority[prop]
                        ):
                            _set_declaration(
                                html_cascade, prop, val, priority=priority,
                                important=important,
                            )
                            html_cascade_priority[prop] = priority
            # Inline html styles take highest precedence
            inline = _copy_declarations(self.html_styles)
            for declaration_index, (prop, val) in enumerate(inline.items()):
                important = inline.important.get(prop, False)
                declaration_index = inline.cascade_priority.get(
                    prop, (0, 0, 0, 0, 0, 0, declaration_index)
                )[-1]
                priority = (
                    int(important), 1, 0, 0, 0, len(all_rules), declaration_index,
                )
                if (
                    prop not in html_cascade_priority
                    or priority >= html_cascade_priority[prop]
                ):
                    _set_declaration(
                        html_cascade, prop, val, priority=priority,
                        important=important,
                    )
                    html_cascade_priority[prop] = priority
            self.html_styles = html_cascade

            # Generated boxes whose originating element is the document
            # element are not descendants of the parsed synthetic body.
            # Cascade them against a detached, real ``html`` element so
            # selectors such as ``:root::scroll-button(*)`` retain their
            # proper origin even in the historical two-node builder shape.
            html_pseudo_source = DomNode('html', self.html_attrs, CssDeclarations())
            apply_css_rules(
                all_rules,
                html_pseudo_source,
                ancestors=[],
                sibling_index=1,
                sibling_count=1,
                preceding_siblings=[],
            )
            self.html_pseudo_styles = html_pseudo_source.pseudo_styles

            # A style element normally has UA ``display:none``. If author CSS
            # changes that computed display, its raw stylesheet text becomes
            # ordinary renderable text. Whitespace before the first style in
            # the HTML head is ignored by the tree builder, even when that
            # style later becomes visible; it must not create a body line box.
            visible_style_nodes = []
            style_sibling_count = trailing_harness_style + len(self.author_style_blocks) + sum(
                1 for child in self.root.children if not child.is_text
            )
            for style_index, (attrs, text, _prefix) in enumerate(
                self.author_style_blocks, 1
            ):
                style_node = DomNode(
                    'style', attrs, parse_inline_styles(attrs.get('style', ''))
                )
                apply_css_rules(
                    all_rules,
                    style_node,
                    ancestors=[('html', [], '', self.html_attrs), ('body', [], '', {})]
                    if self.root_aware else None,
                    sibling_index=style_index,
                    sibling_count=style_sibling_count,
                    sibling_type_index=style_index,
                    sibling_type_count=len(self.author_style_blocks)
                        + trailing_harness_style,
                )
                display = style_node.styles.get('display', '').strip().lower()
                if display and display != 'none':
                    text_node = DomNode('#text', {}, {})
                    text_node.is_text = True
                    text_node.text_content = text
                    style_node.children.append(text_node)
                    visible_style_nodes.append(style_node)
            if visible_style_nodes:
                self.root.children = visible_style_nodes + self.root.children
            # Re-collect all styles
            self.all_styles = []
            def collect(node):
                if not node.is_text:
                    self.all_styles.append(node.styles)
                    for c in node.children:
                        collect(c)
            collect(self.root)

        # Lower the deterministic scroll assignments used by paint/sticky WPT
        # fixtures into document state. These handlers contain no DOM mutation
        # beyond assigning a numeric scrollTop/scrollLeft value.
        nodes_by_id = {}
        def collect_ids(node):
            node_id = node.attrs.get('id', '') if not node.is_text else ''
            if node_id:
                nodes_by_id[node_id] = node
            for child in node.children:
                collect_ids(child)
        collect_ids(self.root)
        if operations is not None:
            for operation, node_id, name, value in operations:
                if operation == 'class':
                    continue
                node = nodes_by_id.get(node_id)
                if node is None:
                    continue
                if operation == 'scroll':
                    setattr(node, name, float(value))
                else:
                    # A CSSOM style assignment is an author inline declaration
                    # and therefore wins the stylesheet cascade.
                    node.styles[name] = value
        self.root.html_pseudo_styles = self.html_pseudo_styles


def _parse_wpt_markup(
    content: str,
    html_dir: str,
    *,
    root_aware: bool = False,
    harness_rules: list | None = None,
    mutation_ir: dict | None = None,
) -> WptHtmlParser:
    """Parse one already-decoded HTML document under an explicit harness."""
    parser = WptHtmlParser(
        root_aware=root_aware,
        harness_rules=harness_rules,
        mutation_ir=mutation_ir,
    )
    parser.html_dir = html_dir
    parser.root.resource_base = parser.html_dir
    parser.feed(content)

    # Resolve and load external stylesheets
    for href in parser.external_stylesheet_hrefs:
        css_path = os.path.join(parser.html_dir, href)
        if os.path.isfile(css_path):
            with open(css_path, 'r', encoding='utf-8-sig', errors='replace') as f:
                css_text = f.read()
            rules = parse_simple_css_rules(css_text)
            parser.external_css_rules.extend(rules)
            parser.keyframes.update(parse_static_keyframes(css_text))

    parser.finalize()
    return parser


def parse_wpt_html(
    html_path: str,
    *,
    root_aware: bool = False,
    mutation_ir: dict | None = None,
) -> WptHtmlParser:
    """Parse a WPT HTML file and return the parser with DOM tree."""
    # Decode a leading UTF-8 BOM as an encoding signature.  Leaving U+FEFF in
    # the character stream makes HTMLParser synthesize body text before the
    # doctype, which later creates a spurious line box in generated fixtures.
    with open(html_path, 'r', encoding='utf-8-sig', errors='replace') as f:
        content = f.read()

    return _parse_wpt_markup(
        content,
        os.path.dirname(os.path.abspath(html_path)),
        root_aware=root_aware,
        mutation_ir=mutation_ir,
    )


# ─── Portability analysis ──────────────────────────────────────────────────

_ASSERTION_ONLY_CHECK_LAYOUT_SCRIPTS = [
    '/resources/testharness.js',
    '/resources/testharnessreport.js',
    '/resources/check-layout-th.js',
]


def _is_assertion_only_check_layout(parser: WptHtmlParser) -> bool:
    """Whether script usage is the inert WPT check-layout assertion harness.

    The generated builders do not execute JavaScript. This narrow exception
    is safe because ``check-layout-th.js`` only verifies authored
    ``data-expected-*`` geometry after layout; stripping it does not change the
    DOM or computed style represented by the builder.
    """
    if len(parser.script_elements) != len(_ASSERTION_ONLY_CHECK_LAYOUT_SCRIPTS):
        return False
    script_sources = []
    for script in parser.script_elements:
        attrs = script['attrs']
        if set(attrs) != {'src'} or script['content'].strip():
            return False
        script_sources.append(attrs['src'])
    if script_sources != _ASSERTION_ONLY_CHECK_LAYOUT_SCRIPTS:
        return False
    if len(parser.event_handlers) != 1:
        return False
    tag, name, body = parser.event_handlers[0]
    if tag != 'body' or name != 'onload':
        return False
    return re.fullmatch(
        r'''\s*checkLayout\(\s*(?:"[^"\\]*"|'[^'\\]*')?\s*\)\s*;?\s*''',
        body,
    ) is not None


def _static_onload_operations(parser: WptHtmlParser) -> list[tuple[str, str, str, str]] | None:
    """Parse deterministic body-onload style/scroll/class assignments."""
    if parser.has_script or not parser.event_handlers:
        return None
    target = (
        r'''(?:document\.getElementById\(\s*(["'])([^"']+)\1\s*\)'''
        r'''|([_a-zA-Z][_a-zA-Z0-9]*))'''
    )
    scroll = re.compile(
        rf'''^{target}\.scroll(Top|Left)\s*=\s*(-?[\d.]+)$'''
    )
    style = re.compile(
        rf'''^{target}\.style\.([_a-zA-Z][_a-zA-Z0-9]*)\s*=\s*'''
        r'''(["'])(.*?)\5$'''
    )
    class_add = re.compile(
        r'''^document\.body\.classList\.add\(\s*(["'])([-_a-zA-Z][-_a-zA-Z0-9]*)\1\s*\)$'''
    )
    operations = []
    for tag, name, body in parser.event_handlers:
        if tag != 'body' or name != 'onload':
            return None
        statements = [statement.strip() for statement in body.split(';') if statement.strip()]
        if not statements:
            return None
        for statement in statements:
            match = class_add.fullmatch(statement)
            if match:
                operations.append(('class', '', 'class', match.group(2)))
                continue
            match = scroll.fullmatch(statement)
            if match:
                node_id = match.group(2) or match.group(3)
                field = 'scroll_top' if match.group(4) == 'Top' else 'scroll_left'
                operations.append(('scroll', node_id, field, match.group(5)))
                continue
            match = style.fullmatch(statement)
            if match:
                node_id = match.group(2) or match.group(3)
                js_name = match.group(4)
                css_name = re.sub(r'([A-Z])', lambda item: '-' + item.group(1).lower(), js_name)
                declarations = parse_inline_styles(f'{css_name}: {match.group(6)}')
                if css_name not in declarations or not check_supported(declarations)[0]:
                    return None
                operations.append(('style', node_id, css_name, declarations[css_name]))
                continue
            return None
    return operations


def _has_only_static_onload_assignments(parser: WptHtmlParser) -> bool:
    return _static_onload_operations(parser) is not None

def analyze_portability(parser: WptHtmlParser) -> tuple[bool, str]:
    """Determine if a WPT test can be ported to our engine.
    Returns (portable, reason_if_not).
    """
    has_lowered_ast_mutations = bool(
        parser.mutation_ir is not None and parser.mutation_ir.get('lowerable')
    )
    if (
        parser.has_script or parser.event_handlers
    ) and not has_lowered_ast_mutations and not _is_assertion_only_check_layout(parser) and not _has_only_static_onload_assignments(parser):
        return False, "uses_javascript"

    # Pseudo-classes/pseudo-elements we can handle
    SAFE_PSEUDO_PATTERN = re.compile(
        r':(?:root|first-child|last-child|first-of-type|last-of-type|only-of-type|nth-child\([^)]+\)|only-child|empty|not\([^)]+\)|target-current)'
    )

    # Check CSS rules from <style> blocks for unsupported properties
    if parser.has_style_block:
        for selector, styles in parser.css_rules:
            # Strip safe pseudo-classes before checking for unsupported ones
            pseudo = _terminal_pseudo(selector)
            stripped = pseudo[0] if pseudo else selector
            stripped = SAFE_PSEUDO_PATTERN.sub('', stripped)
            # After stripping safe pseudos, reject remaining pseudo-classes/elements
            if '::' in stripped:
                return False, f"complex_css_selector: {selector}"
            # Allow remaining ':' only if it was fully consumed by safe pattern
            remaining_colons = stripped.replace('::', '')
            if ':' in remaining_colons:
                return False, f"complex_css_selector: {selector}"
            # Reject sibling combinators only if they appear in attribute selectors
            # (not as CSS combinators — we now support + and ~ combinators)
            # Note: + and ~ as combinators have whitespace around them in normalized CSS,
            # but we check the raw selector. We need a smarter check.
            # For now, allow all selectors through since match_selector handles them.
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
        if display in ('ruby', 'ruby-text'):
            return False, f"unsupported display: {display}"

    # Check for unsupported elements
    def check_tree(node):
        if node.is_text:
            return True, ""
        if node.tag in ('dialog',
                        'template', 'slot'):
            return False, f"unsupported element: <{node.tag}>"
        for child in node.children:
            ok, reason = check_tree(child)
            if not ok:
                return False, reason
        return True, ""

    ok, reason = check_tree(parser.root)
    if not ok:
        return ok, reason

    if RETAIN_TEXT:
        # SP14 deterministic text mode accepts only the repertoire exercised by
        # the opted-in corpus. Ahem covers Latin-1 and the ellipsis; directional
        # controls are non-painting. Chromium-pinned Droid/Noto faces cover the
        # CJK/fullwidth, complex-script, and emoji ranges below before the
        # explicitly pinned DejaVu Sans terminal fallback on both renderers.
        # Unknown code points remain a hard transactional failure rather than
        # silently depending on ambient font fallback.
        extra_codepoints = {0x2026, 0x2190, 0x2193, 0xFEFF}

        def deterministic_text_char(ch):
            cp = ord(ch)
            return (
                ch in '\t\n\r\f'
                or 0x20 <= cp <= 0x7E
                or 0xA0 <= cp <= 0xFF
                or cp in extra_codepoints
                or 0x0300 <= cp <= 0x036F  # combining diacritics
                or 0x0600 <= cp <= 0x06FF  # Arabic
                or 0x0750 <= cp <= 0x077F  # Arabic Supplement
                or 0x08A0 <= cp <= 0x08FF  # Arabic Extended-A
                or 0x0900 <= cp <= 0x097F  # Devanagari
                or cp == 0x1680  # Ogham space mark
                or 0x2000 <= cp <= 0x206F  # Unicode spaces/general punctuation
                or 0x200C <= cp <= 0x200D  # ZWNJ / ZWJ
                or 0x3000 <= cp <= 0x30FF  # CJK punctuation / kana
                or 0x3400 <= cp <= 0x4DBF  # CJK Extension A
                or 0x4E00 <= cp <= 0x9FFF  # CJK Unified Ideographs
                or 0xFE00 <= cp <= 0xFE0F  # variation selectors
                or 0xFF00 <= cp <= 0xFFEF  # fullwidth / halfwidth forms
                or 0x1F000 <= cp <= 0x1FAFF  # emoji and pictographs
                or 0x202A <= cp <= 0x202E
                or 0x2066 <= cp <= 0x2069
            )

        def check_text_ascii(node):
            if getattr(node, 'is_text', False):
                text = getattr(node, 'text_content', '') or ''
                return all(deterministic_text_char(ch) for ch in text)
            return all(check_text_ascii(c) for c in node.children)
        if not check_text_ascii(parser.root):
            return False, "text_non_ascii"

    return True, ""


def has_layout_content(parser: WptHtmlParser) -> bool:
    """Return whether the parsed body contains a renderable test node.

    Instruction-only paragraphs and forced breaks are not sufficient on their
    own.  Text nodes are intentionally ignored here: a body/root-only test
    needs separate viewport propagation support and must not be promoted merely
    because deterministic text retention made its prose visible.
    """
    for child in parser.root.children:
        if child.is_text or child.tag == "br":
            continue
        if child.tag == "p" and not child.styles:
            if any(
                sub.is_text
                and "test passes" in getattr(sub, "text_content", "").lower()
                for sub in child.children
            ):
                continue
        return True
    return False


# ─── Rust code generation ─────────────────────────────────────────────────

def sanitize_fn_name(name: str) -> str:
    """Convert a filename to a valid Rust function name."""
    name = re.sub(r'[^a-zA-Z0-9]', '_', name)
    name = re.sub(r'_+', '_', name)
    name = name.strip('_').lower()
    if name[0:1].isdigit():
        name = 'test_' + name
    return name


def _css_length_px(value: str, font_size: float = 16.0) -> float | None:
    """Resolve fixed CSS lengths to px for static WPT geometry."""
    value = value.strip()
    if value in ('0', '0px'):
        return 0.0
    m = re.match(r'^(-?[\d.]+)(px|em|rem|in|cm|mm|q|pt|pc)$', value, re.IGNORECASE)
    if not m:
        return None
    num = float(m.group(1))
    unit = m.group(2)
    factors = {
        'px': 1.0,
        'em': font_size,
        'rem': 16.0,
        'in': 96.0,
        'cm': 96.0 / 2.54,
        'mm': 96.0 / 25.4,
        'q': 96.0 / 101.6,
        'pt': 96.0 / 72.0,
        'pc': 16.0,
    }
    return _zoomed_px(num * factors[unit])


def _split_css_layers(value: str) -> list[str]:
    """Split a comma-separated CSS layer list, respecting parentheses."""
    layers = []
    depth = 0
    current = []
    for ch in value:
        if ch == '(':
            depth += 1
        elif ch == ')' and depth > 0:
            depth -= 1
        if ch == ',' and depth == 0:
            layers.append(''.join(current).strip())
            current = []
        else:
            current.append(ch)
    if current:
        layers.append(''.join(current).strip())
    return layers


def _linear_gradient_function(value: str) -> tuple[str, str, bool] | None:
    """Return a linear-gradient argument list and the remaining shorthand."""
    match = re.search(
        r'(?P<repeating>repeating-)?linear-gradient\s*\(',
        value,
        re.IGNORECASE,
    )
    if not match:
        return None
    depth = 1
    index = match.end()
    while index < len(value) and depth:
        if value[index] == '(':
            depth += 1
        elif value[index] == ')':
            depth -= 1
        index += 1
    if depth:
        return None
    args = value[match.end():index - 1]
    remainder = (value[:match.start()] + ' ' + value[index:]).strip()
    return args, remainder, match.group('repeating') is not None


def _linear_gradient_rust(value: str) -> str | None:
    """Parse the ordinary linear-gradient subset into a computed-style value."""
    extracted = _linear_gradient_function(value)
    if extracted is None:
        return None
    args, _, repeating = extracted
    parts = _split_css_layers(args)
    if len(parts) < 2:
        return None

    angle = 180.0
    direction = parts[0].strip().lower()
    if direction.endswith('deg'):
        try:
            angle = float(direction[:-3]) % 360.0
            parts = parts[1:]
        except ValueError:
            return None
    elif direction.startswith('to '):
        directions = {
            'to top': 0.0,
            'to right': 90.0,
            'to bottom': 180.0,
            'to left': 270.0,
        }
        if direction not in directions:
            return None
        angle = directions[direction]
        parts = parts[1:]

    stops = []
    for part in parts:
        tokens = _split_respecting_parens(part.strip())
        if not tokens:
            return None
        color = parse_color(tokens[0])
        if color is None:
            return None
        position = 'GradientStopPosition::Auto'
        if len(tokens) >= 2:
            raw_position = tokens[1].strip().lower()
            try:
                if raw_position.endswith('%'):
                    position = (
                        'GradientStopPosition::Percent('
                        f'{float(raw_position[:-1])})'
                    )
                elif raw_position.endswith('px'):
                    position = (
                        'GradientStopPosition::Px('
                        f'{float(raw_position[:-2])})'
                    )
                elif raw_position.endswith('lh'):
                    position = (
                        'GradientStopPosition::Px('
                        f'{float(raw_position[:-2]) * _ACTIVE_LINE_HEIGHT_PX})'
                    )
                elif raw_position == '0':
                    position = 'GradientStopPosition::Px(0.0)'
                else:
                    return None
            except ValueError:
                return None
        stops.append(
            'LinearGradientStop { '
            f'color: {color}, position: {position} '
            '}'
        )
    if len(stops) < 2:
        return None
    return (
        'Some(LinearGradient { '
        f'angle_degrees: {angle}, repeating: {str(repeating).lower()}, '
        f'stops: vec![{", ".join(stops)}] '
        '})'
    )


def _resource_mime(path: Path, data: bytes) -> str:
    if data.startswith(b'\x89PNG\r\n\x1a\n'):
        return 'image/png'
    if data.startswith(b'\xff\xd8'):
        return 'image/jpeg'
    if data[:6] in (b'GIF87a', b'GIF89a'):
        return 'image/gif'
    if b'<svg' in data[:1024].lower():
        return 'image/svg+xml'
    overrides = {
        '.svg': 'image/svg+xml', '.xht': 'text/html',
        '.xhtml': 'application/xhtml+xml', '.webm': 'video/webm',
        '.mp4': 'video/mp4', '.ttf': 'font/ttf',
    }
    return overrides.get(path.suffix.lower()) or mimetypes.guess_type(path.name)[0] or 'application/octet-stream'


def _svg_number(value: str | None) -> float | None:
    if value is None:
        return None
    match = re.fullmatch(r'\s*([0-9]+(?:\.[0-9]+)?)\s*(?:px)?\s*', value)
    return float(match.group(1)) if match else None


def _resource_dimensions(path: Path, data: bytes, mime: str) -> tuple[float, float] | None:
    width = height = None
    if mime == 'image/png' and len(data) >= 24:
        width = float(int.from_bytes(data[16:20], 'big'))
        height = float(int.from_bytes(data[20:24], 'big'))
    elif mime == 'image/gif' and len(data) >= 10:
        width = float(int.from_bytes(data[6:8], 'little'))
        height = float(int.from_bytes(data[8:10], 'little'))
    elif mime == 'image/jpeg':
        offset = 2
        while offset + 9 <= len(data):
            if data[offset] != 0xff:
                offset += 1
                continue
            marker = data[offset + 1]
            if marker in {0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf}:
                height = float(int.from_bytes(data[offset + 5:offset + 7], 'big'))
                width = float(int.from_bytes(data[offset + 7:offset + 9], 'big'))
                break
            if marker in {0xd8, 0xd9}:
                offset += 2
            elif offset + 4 <= len(data):
                offset += 2 + int.from_bytes(data[offset + 2:offset + 4], 'big')
            else:
                break
        if 'orientation-6' in path.name and width is not None:
            width, height = height, width
    elif mime == 'image/svg+xml':
        text = data.decode('utf-8', errors='ignore')
        root = re.search(r'<svg\b([^>]*)>', text, re.IGNORECASE)
        attrs = root.group(1) if root else ''
        width_match = re.search(r'\bwidth\s*=\s*[\'\"]([^\'\"]+)', attrs, re.IGNORECASE)
        height_match = re.search(r'\bheight\s*=\s*[\'\"]([^\'\"]+)', attrs, re.IGNORECASE)
        width = _svg_number(width_match.group(1)) if width_match else None
        height = _svg_number(height_match.group(1)) if height_match else None
        viewbox = re.search(r'\bviewBox\s*=\s*[\'\"]([^\'\"]+)', attrs, re.IGNORECASE)
        if viewbox:
            parts = re.split(r'[\s,]+', viewbox.group(1).strip())
            try:
                view_width, view_height = float(parts[2]), float(parts[3])
                if width is None and height is not None:
                    width = height * view_width / view_height
                elif height is None and width is not None:
                    height = width * view_height / view_width
            except (IndexError, ValueError, ZeroDivisionError):
                pass
    elif mime.startswith('video/'):
        named = re.search(r'(?:^|[^0-9])(\d+)x(\d+)(?:[^0-9]|$)', path.name)
        if named:
            width, height = float(named.group(1)), float(named.group(2))
    return (width, height) if width is not None and height is not None else None


def _resource_intrinsic_ratio(
    data: bytes, mime: str, dimensions: tuple[float, float] | None,
) -> tuple[float, float] | None:
    if mime != 'image/svg+xml':
        return (
            dimensions
            if dimensions is not None and dimensions[0] > 0.0 and dimensions[1] > 0.0
            else None
        )
    text = data.decode('utf-8', errors='ignore')
    root = re.search(r'<svg\b([^>]*)>', text, re.IGNORECASE)
    attrs = root.group(1) if root else ''
    viewbox = re.search(r'\bviewBox\s*=\s*[\'\"]([^\'\"]+)', attrs, re.IGNORECASE)
    if viewbox:
        parts = re.split(r'[\s,]+', viewbox.group(1).strip())
        try:
            width, height = float(parts[2]), float(parts[3])
            if width > 0.0 and height > 0.0:
                return width, height
        except (IndexError, ValueError):
            pass
    width_match = re.search(r'\bwidth\s*=\s*[\'\"]([^\'\"]+)', attrs, re.IGNORECASE)
    height_match = re.search(r'\bheight\s*=\s*[\'\"]([^\'\"]+)', attrs, re.IGNORECASE)
    width = _svg_number(width_match.group(1)) if width_match else None
    height = _svg_number(height_match.group(1)) if height_match else None
    return (width, height) if width and height else None


def _resolve_local_resource(source: str, base: Path | None = None) -> Path | None:
    source = html_module.unescape(source.strip())
    if not source or source.startswith(('data:', 'about:', 'blob:', 'javascript:')):
        return None
    base = (base or _ACTIVE_RESOURCE_BASE)
    if base is None:
        return None
    if '{{location[path]}}/../' in source:
        source = source.split('{{location[path]}}/../', 1)[1]
        candidate = base / source
    elif source.startswith('//') or re.match(r'^[a-zA-Z][a-zA-Z0-9+.-]*:', source):
        return None
    else:
        clean = urllib.parse.unquote(source.split('?', 1)[0].split('#', 1)[0])
        candidate = WPT_SOURCE_ROOT / clean.lstrip('/') if clean.startswith('/') else base / clean
    resolved = candidate.resolve()
    try:
        resolved.relative_to(WPT_SOURCE_ROOT)
    except ValueError:
        return None
    return resolved if resolved.is_file() else None


def _packaged_resource(source: str, base: Path | None = None) -> tuple[str, str, str, str, tuple[float, float] | None] | None:
    path = _resolve_local_resource(source, base)
    if path is None:
        return None
    data = path.read_bytes()
    sha = hashlib.sha256(data).hexdigest()
    mime = _resource_mime(path, data)
    filename = f'{sha}{path.suffix.lower() or ".bin"}'
    package = SP20_ASSET_DIR / filename
    if not package.is_file() or package.read_bytes() != data:
        return None
    label = path.relative_to(WPT_SOURCE_ROOT).as_posix()
    return filename, label, mime, sha, _resource_dimensions(path, data, mime)


def _data_url_resource(source: str) -> tuple[str, str, str, tuple[float, float] | None, bytes] | None:
    if not source.startswith('data:'):
        return None
    header, comma, payload = source.partition(',')
    if not comma:
        return None
    mime = header[5:].split(';', 1)[0] or 'text/plain'
    try:
        import base64
        data = base64.b64decode(payload) if ';base64' in header else urllib.parse.unquote_to_bytes(payload)
    except Exception:
        return None
    sha = hashlib.sha256(data).hexdigest()
    return f'data:{mime};sha256={sha}', mime, sha, _resource_dimensions(Path('inline'), data, mime), data


def _packaged_bytes_expr(filename: str) -> str:
    return (
        'include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), '
        f'"/../../../tools/accountability/data/wpt_assets/sp20/{filename}"))'
        '.as_slice().to_vec()'
    )


_PAINT_ASSETS = {
    '60x60-green.png': ('60x60-green.png', 'css-backgrounds/support/60x60-green.png', 'image/png', '38a9a0ea560a60b9ce79be68126b1e57bbbbcab0c013b9893f4f43fce7ebc3c4'),
    'cat.png': ('cat.png', 'support/cat.png', 'image/png', '18ca1a3f23c106c4b31f0faec34a24cb17d4f2cb31abd1a51df814ba3f58ed7d'),
    'aqua-yellow-32x32.png': ('aqua-yellow-32x32.png', 'css-backgrounds/support/aqua-yellow-32x32.png', 'image/png', '5652f8bc47b6dbb03a1bd16dfc571a61f8e1e3eebbb746f2559a2838d649a5ed'),
    '1x1-green.png': ('1x1-green.png', 'css-backgrounds/support/1x1-green.png', 'image/png', 'a236213916dd30bd771a233aa1d66381eabf335bf8885304b75a4e2e370d68ce'),
    'green-100.png': ('green-100.png', 'css-backgrounds/resources/green-100.png', 'image/png', '750da204219f837c5b8a25f3587b82b05ca841431be990a1a13a50df483e53a4'),
    'css3.png': ('css3.png', 'css-backgrounds/support/css3.png', 'image/png', '404cf10151727f8165e24ff2c964073511fb857ebbf9e4422f0572c7ddf141ef'),
    'green.png': ('green.png', 'css-backgrounds/support/green.png', 'image/png', 'a48b88602c40120ef8d508bd56a1731d204cbb7701749651d206ad7374819b00'),
    'red.png': ('red.png', 'css-backgrounds/support/red.png', 'image/png', '07557d92effc78121f8a48acacfa535d3d4cf368a647124de041b5bba12144ae'),
    '40px-wide-20px-tall-green-rect.png': ('40px-wide-20px-tall-green-rect.png', 'css-backgrounds/support/40px-wide-20px-tall-green-rect.png', 'image/png', 'e9b1bf6e42430928746a02061391ad742ad258cd1392684066391df30eb95c14'),
    'swatch-green.png': ('swatch-green.png', 'css-backgrounds/support/swatch-green.png', 'image/png', 'bfdf34690a36ddebb5f08029df544183f3d5a3e9e21dbff0f4d4315f862236c0'),
    'aqua-yellow-37x37.png': ('aqua-yellow-37x37.png', 'css-backgrounds/support/aqua-yellow-37x37.png', 'image/png', 'd279fe78b42445636c667020249169b35bbf039b57468604dbd0f63978144c60'),
    '9-colored-areas-40-30-20-10.svg': ('9-colored-areas-40-30-20-10.svg', 'css-backgrounds/support/9-colored-areas-40-30-20-10.svg', 'image/svg+xml', 'bde621c5c23b189c6ac29fbccc168a46ad19f61233d12be22f67626cd15e1714'),
    'blue-and-red-diamonds-81x81.png': ('blue-and-red-diamonds-81x81.png', 'css-backgrounds/support/blue-and-red-diamonds-81x81.png', 'image/png', 'adcf99b02f2084a28ce5f227d472c2a757393184e472ba7b3ccba8ce11ed617b'),
    '9grid40-30-20-10-red-old.png': ('9grid40-30-20-10-red-old.png', 'css-backgrounds/support/9grid40-30-20-10-red-old.png', 'image/png', '1f60051612d5f926d6302d69557116ef18518b45dc992d4395c4680e1cf0a134'),
    '9grid40-30-20-10-red.png': ('9grid40-30-20-10-red.png', 'css-backgrounds/support/9grid40-30-20-10-red.png', 'image/png', 'c43d860aa7387ad2bc7b1a847c6fc3b0d2416d18a8a204b929ab86d4a19479c9'),
    '9grid40-30-20-10-green.png': ('9grid40-30-20-10-green.png', 'css-backgrounds/support/9grid40-30-20-10-green.png', 'image/png', '886b528f342e8f918f6c5ccc301da05d71149e61ecd2619e1da8c991b1abe595'),
    'outline-5px-10px-15px-20px-green.png': ('outline-5px-10px-15px-20px-green.png', 'css-backgrounds/support/outline-5px-10px-15px-20px-green.png', 'image/png', 'fea0d9ccac4281eb5836bcd1a0339d3d2ee5187ea8b8286dbc7090e926b6f4ba'),
    'swatch-red.png': ('swatch-red.png', 'css-backgrounds/support/swatch-red.png', 'image/png', 'e42df70647347f5eedb984a611549d962ee362fb73f2135c9af05875b7681784'),
    '100x100-red.png': ('100x100-red.png', 'css-position/sticky/support/100x100-red.png', 'image/png', '0ca8457abb56c0b5df03ed741bfec7b54c0bd90b2dd8b8c3f6e47f9a691d3a58'),
    'stripes-100.png': (None, 'css-backgrounds/resources/stripes-100.png', 'image/png', 'cd8087c9a2e4825f5d6e4807bb739584a21ea1a8f4ed4cb76cecfbdcf797b903'),
    'black20x20.png': (None, 'css-multicol/support/black20x20.png', 'image/png', '3f08031eb1f4aa651ed4b94e383920d3dde660efa5172ce9db4508c17d7ab301'),
    'swatch-blue.png': (None, 'css-multicol/support/swatch-blue.png', 'image/png', 'a2cb741be17fd94f176247f4dca28146b237f34a6df1d46c813d4e0072e41578'),
    'swatch-orange.png': (None, 'css-multicol/support/swatch-orange.png', 'image/png', '8f6d8b04d0f5f7dd0d153f9e5dfc0c54ed822ae9623410baf71759ec4ea02692'),
    'exif-orientation-6-ru.jpg': (None, 'css-images/support/exif-orientation-6-ru.jpg', 'image/jpeg', 'f28b2be684a08d716d846f1f9fbc932f030436acbd5b80c8028d107c25886275'),
}

_INLINE_PAINT_ASSETS = {
    'stripes-100.png': 'iVBORw0KGgoAAAANSUhEUgAAAGQAAABkAQMAAABKLAcXAAAACXBIWXMAAAsTAAALEwEAmpwYAAAABlBMVEUAAABfyc6kY1PyAAAAAnRSTlMA/iyWEiMAAAAcSURBVDhPY2Cw/8D8H0YwjPJGeaO8Ud4oj8Y8APNILMPXB0e9AAAAAElFTkSuQmCC',
    'black20x20.png': 'iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAIAAAAC64paAAAAAXNSR0IArs4c6QAAAAlwSFlzAAAOxAAADsQBlSsOGwAAAAd0SU1FB9wEFxcoF1aBIYoAAAAZdEVYdENvbW1lbnQAQ3JlYXRlZCB3aXRoIEdJTVBXgQ4XAAAAEklEQVQ4y2NgGAWjYBSMgqELAATEAAHkcNQ6AAAAAElFTkSuQmCC',
    'swatch-blue.png': 'iVBORw0KGgoAAAANSUhEUgAAAA8AAAAPAQMAAAABGAcJAAAAA1BMVEUAAP+KeNJXAAAADElEQVR42mNgIAEAAAAtAAH7KhMqAAAAAElFTkSuQmCC',
    'swatch-orange.png': 'iVBORw0KGgoAAAANSUhEUgAAAA8AAAAPAQMAAAABGAcJAAAAA1BMVEX/pQDKkkGbAAAADElEQVR42mNgIAEAAAAtAAH7KhMqAAAAAElFTkSuQmCC',
    'exif-orientation-6-ru.jpg': '/9j/4AAQSkZJRgABAQEASABIAAD/4QDGRXhpZgAASUkqAAgAAAAHABIBAwABAAAABgAAABoBBQABAAAAYgAAABsBBQABAAAAagAAACgBAwABAAAAAgAAADEBAgANAAAAcgAAADIBAgAUAAAAgAAAAGmHBAABAAAAlAAAAAAAAABIAAAAAQAAAEgAAAABAAAAR0lNUCAyLjEwLjE0AAAyMDIwOjAyOjEzIDExOjMyOjQ4AAMAAaADAAEAAAABAAAAAqAEAAEAAABkAAAAA6AEAAEAAAAyAAAAAAAAAP/bAEMAAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAf/bAEMBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAf/CABEIADIAZAMBEQACEQEDEQH/xAAYAAEBAQEBAAAAAAAAAAAAAAAACAYJCv/EABgBAQEBAQEAAAAAAAAAAAAAAAAJCAoH/9oADAMBAAIQAxAAAAGK8r4nAAHcWfu4aX8p9LAAHmjqxNMAAdxZ+7hpfyn0sAAeaOrE0wAB3Fn7uGl/KfSwAB5Au6fIwAArSENod5l/RYAA58dafPIAAKNgDbzX5V0oAAJD6qOa8AAUbAG3mvyrpQAASH1Uc14AAo2ANvNflXSgAA//xAAeEAAABQUBAAAAAAAAAAAAAAAABAYVFgUHIDA1N//aAAgBAQABBQLKz/nWyz/nWyz/AJ1k4Hw4Hw4Hw4Hw4Hw4HwmVWqClEmixE0WImixE0WImixE0WOdB5Wyg8rZQeVl//8QAMBEAAAIECgsBAQAAAAAAAAAABQYAAgQHAwgVF1VWlJbT1AESEyAwNziFhrW2ERb/2gAIAQMBAT8B3nP8ui73b3onxXP8ui73b3onxXP8ui73b3onvyQFUYH2Jmw0kgKowPsTNhpJAVRgfYmbDSSAqjA+xM2GkkBVGB9iZsNJICqMD7EzYaRTiERhCL+QWxvJZTbmuF/qtq1NZdB2lohdmdTHBKbSGhmNeEX1IJRSDU1ltOqooqpo/FVdGhJtndVBJV1gPIpNs7qoJKusB5FJtndVBJV1gPIpNs7qoJKusB5FJtndVBJV1gPIpNs7qoJKusB5HfihdOzvfLPuDNxYoXTs73yz7gzcWKF07O98s+4M2/8A/8QALxEAAAIECgsBAAAAAAAAAAAABgcAAgMFBAgWF1ZXlpfU1QESEyAwNziFhra3Ef/aAAgBAgEBPwHeO7meJ+y+vOnindzPE/ZfXnTxTu5nifsvrzp35iiRqcKu70I5QkxRI1OFXd6EcoSYokanCru9COUJMUSNThV3ehHKEmKJGpwq7vQjlCTFEjU4Vd3oRyhI5hbF07YyZjwJ2gEFQCBMZH7GCQIKuOCwVltAEF2rTZMGEBUZM9dquu1X1VdGs0XXXW/VltOnTIQEUNCtnnRg0kICKGhWzzowaSEBFDQrZ50YNJCAihoVs86MGkhARQ0K2edGDSQgIoaFbPOjB78drqdMzwz58FOLHa6nTM8M+fBTix2up0zPDPnwU3//xAAqEAAAAwQJBQEBAAAAAAAAAAACAwQABQY0ARIgMDaUldLUIYOFtLURFP/aAAgBAQAGPwK1DvlvuvO9h3y33Xnew75b7rztzqvMnb2nVeZO3tOq8ydvadV5k7e06rzJ29p1XmTt7Ik6SJH+mTl/01CE74eBJIKys8YqhZagIA1hiEMX5R1EKkVPWmlsWRLrr05TYsiXXXpymxZEuuvTlNiyJddenKbFkS669OU2LIl116cq2l7/ALJ16l7/ALJ16l7/ALJ1v//EABcQAAMBAAAAAAAAAAAAAAAAAAEhUGH/2gAIAQEAAT8hm6aaGDBgwYMZeUTv3AG9CDNGDBgwYMYYY//aAAwDAQACAAMAAAAQAAAAAAAAAAAAAAAAAAADbbYSSSAAAAAAAAAAAAAAAAAAAH//xAAVEQEBAAAAAAAAAAAAAAAAAABQgf/aAAgBAwEBPxA1RRRy5cuXLmRI1sVvUlRG9+/fv373XXf/xAAVEQEBAAAAAAAAAAAAAAAAAABQAf/aAAgBAgEBPxA3DDB69evXr2OhTnbZ954BFixYsWLCSSf/xAAVEAEBAAAAAAAAAAAAAAAAAABQQf/aAAgBAQABPxA3XXUGDBgwYOhRVRROd5xGgYMGDBg8MMP/2Q==',
}


_REPLACED_ASSET_DIMENSIONS = {
    'black20x20.png': (20.0, 20.0),
    'swatch-blue.png': (15.0, 15.0),
    'swatch-orange.png': (15.0, 15.0),
    # The encoded raster is 100×50 with EXIF orientation 6. CSS Images uses
    # the orientation-adjusted natural dimensions.
    'exif-orientation-6-ru.jpg': (50.0, 100.0),
}


def _embed_paint_asset_urls(template: str, html_dir: str | None = None) -> str:
    """Make focused Chrome templates independent of an ambient WPT server."""
    import base64

    asset_dir = os.path.join(
        os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
        'tools', 'accountability', 'data', 'wpt_assets', 'sp13p',
    )

    def encoded_asset(source: str) -> tuple[str, str] | None:
        if source.startswith('data:'):
            return None
        packaged = _packaged_resource(source, Path(html_dir) if html_dir else None)
        if packaged is not None:
            filename, _, mime, _, _ = packaged
            # Font URLs are supplied by the pinned Chromium harness. Inlining
            # them into every template is wasteful and changes font origins;
            # all visual and embedded resources remain self-contained.
            if not (mime.startswith('image/') or mime.startswith('video/') or mime in ('text/html', 'application/xhtml+xml')):
                return None
            encoded = base64.b64encode((SP20_ASSET_DIR / filename).read_bytes()).decode('ascii')
            return mime, encoded
        asset = _PAINT_ASSETS.get(source.rsplit('/', 1)[-1])
        if asset is None:
            return None
        filename, _, mime, _ = asset
        if filename is None:
            encoded = _INLINE_PAINT_ASSETS[source.rsplit('/', 1)[-1]]
        else:
            with open(os.path.join(asset_dir, filename), 'rb') as asset_file:
                encoded = base64.b64encode(asset_file.read()).decode('ascii')
        return mime, encoded

    def replace_url(match: re.Match) -> str:
        source = match.group(1).strip().strip('"\'')
        encoded = encoded_asset(source)
        if encoded is None:
            return match.group(0)
        mime, payload = encoded
        return f'url("data:{mime};base64,{payload}")'

    def replace_src(match: re.Match) -> str:
        source = match.group(3).strip()
        encoded = encoded_asset(source)
        if encoded is None:
            return match.group(0)
        mime, payload = encoded
        return f'{match.group(1)}{match.group(2)}data:{mime};base64,{payload}{match.group(2)}'

    def replace_srcset(match: re.Match) -> str:
        candidates = []
        for raw_candidate in match.group(3).split(','):
            parts = raw_candidate.strip().split()
            if len(parts) not in (1, 2):
                return match.group(0)
            encoded = encoded_asset(parts[0])
            if encoded is None:
                return match.group(0)
            mime, payload = encoded
            candidate = f'data:{mime};base64,{payload}'
            if len(parts) == 2:
                candidate += f' {parts[1]}'
            candidates.append(candidate)
        return (
            f'{match.group(1)}{match.group(2)}'
            f'{", ".join(candidates)}{match.group(2)}'
        )

    template = re.sub(r'(?is)url\(\s*([^)]*?)\s*\)', replace_url, template)
    template = re.sub(
        r'(?is)(\bsrcset\s*=\s*)(["\'])([^"\']+)\2', replace_srcset, template
    )
    template = re.sub(
        r'(?is)(\b(?:src|poster|data)\s*=\s*)(["\'])([^"\']+)\2', replace_src, template
    )
    return template


def _extract_css_image(value: str) -> tuple[str, str] | None:
    match = re.search(
        r'(?i)(?:repeating-)?(?:linear|radial|conic)-gradient\s*\(|url\s*\(',
        value,
    )
    if not match:
        return None
    depth = 0
    quote = None
    escaped = False
    index = value.find('(', match.start())
    for end in range(index, len(value)):
        char = value[end]
        if escaped:
            escaped = False
            continue
        if char == '\\' and quote:
            escaped = True
            continue
        if quote:
            if char == quote:
                quote = None
            continue
        if char in ('"', "'"):
            quote = char
        elif char == '(':
            depth += 1
        elif char == ')':
            depth -= 1
            if depth == 0:
                image = value[match.start():end + 1]
                remainder = (value[:match.start()] + ' ' + value[end + 1:]).strip()
                return image, remainder
    return None


def _gradient_position_rust(value: str, font_size: float = 16.0) -> str | None:
    value = value.strip().lower()
    if value in ('0', '0px'):
        return 'GradientStopPosition::Px(0.0)'
    match = re.fullmatch(r'(-?[\d.]+)%', value)
    if match:
        return f'GradientStopPosition::Percent({float(match.group(1))})'
    match = re.fullmatch(r'(-?[\d.]+)px', value)
    if match:
        return f'GradientStopPosition::Px({_zoomed_px(float(match.group(1)))})'
    absolute = _css_length_px(value, font_size)
    if absolute is not None:
        return f'GradientStopPosition::Px({absolute})'
    match = re.fullmatch(
        r'calc\(\s*(-?[\d.]+)%\s*([+-])\s*([\d.]+)px\s*\)', value
    )
    if match:
        px = float(match.group(3)) * (-1 if match.group(2) == '-' else 1)
        return (
            'GradientStopPosition::Calc { '
            f'percent: {float(match.group(1))}, px: {_zoomed_px(px)} '
            '}'
        )
    return None


def _style_color_rust(value: str) -> str | None:
    if value.strip().lower() == 'currentcolor':
        return 'StyleColor::CurrentColor'
    color = parse_color(value)
    return f'StyleColor::Resolved({color})' if color else None


def _gradient_stops_rust(parts: list[str], font_size: float = 16.0) -> list[str] | None:
    stops = []
    for part in parts:
        tokens = _split_respecting_parens(part.strip())
        if not tokens:
            return None
        color = _style_color_rust(tokens[0])
        if color is None:
            position = (
                _gradient_position_rust(tokens[0], font_size)
                if len(tokens) == 1
                else None
            )
            if position:
                hint_position = position.replace(
                    'GradientStopPosition::',
                    'GradientStopPosition::Hint',
                    1,
                )
                stops.append(
                    'GradientStop { '
                    'color: StyleColor::Resolved(Color::TRANSPARENT), '
                    f'position: {hint_position} '
                    '}'
                )
                continue
            return None
        positions = [_gradient_position_rust(token, font_size) for token in tokens[1:]]
        if any(position is None for position in positions):
            return None
        if not positions:
            positions = ['GradientStopPosition::Auto']
        for position in positions:
            stops.append(
                'GradientStop { '
                f'color: {color}, position: {position} '
                '}'
            )
    return stops if len(stops) >= 2 else None


def _gradient_color_space(prelude: str) -> tuple[str, str]:
    match = re.search(r'(?i)(?:^|\s)in\s+(srgb|hsl|oklch)(?:\s|$)', prelude)
    if not match:
        return prelude.strip(), 'GradientColorSpace::Srgb'
    mapping = {
        'srgb': 'GradientColorSpace::Srgb',
        'hsl': 'GradientColorSpace::Hsl',
        'oklch': 'GradientColorSpace::Oklch',
    }
    cleaned = (prelude[:match.start()] + ' ' + prelude[match.end():]).strip()
    return cleaned, mapping[match.group(1).lower()]


def _position_component_rust(token: str, *, end: bool | None = None) -> str | None:
    token = token.strip().lower()
    if end is None and token in ('left', 'top'):
        return 'BackgroundPosition::Percent(0.0)'
    if end is None and token == 'center':
        return 'BackgroundPosition::Percent(50.0)'
    if end is None and token in ('right', 'bottom'):
        return 'BackgroundPosition::Percent(100.0)'
    length = parse_length(token)
    if length is None:
        return None
    if end is None and token.endswith('%'):
        return f'BackgroundPosition::Percent({float(token[:-1])})'
    if end is None:
        return f'BackgroundPosition::Length({length})'
    return f'BackgroundPosition::Edge {{ end: {str(end).lower()}, offset: {length} }}'


def _background_position_rust(value: str) -> tuple[str, str] | None:
    tokens = _split_respecting_parens(value.strip())
    if not tokens:
        return None
    zero = 'BackgroundPosition::Percent(0.0)'
    center = 'BackgroundPosition::Percent(50.0)'

    # The two-value grammar is axis-based: in `left 75px`, `left` is the
    # horizontal position and `75px` is the vertical position.  An offset
    # paired with an edge belongs to the three/four-value grammar instead.
    if len(tokens) <= 2:
        lower = [token.lower() for token in tokens]
        horizontal_keywords = {'left', 'right'}
        vertical_keywords = {'top', 'bottom'}
        if len(tokens) == 1:
            token = lower[0]
            if token in horizontal_keywords:
                return _position_component_rust(token), center
            if token in vertical_keywords:
                return center, _position_component_rust(token)
            component = _position_component_rust(tokens[0])
            return (component, center) if component else None

        first, second = lower
        if first in horizontal_keywords:
            x = _position_component_rust(first)
            y = _position_component_rust(tokens[1])
        elif first in vertical_keywords:
            x = _position_component_rust(tokens[1])
            y = _position_component_rust(first)
        elif second in vertical_keywords:
            x = _position_component_rust(tokens[0])
            y = _position_component_rust(second)
        elif second in horizontal_keywords:
            x = _position_component_rust(second)
            y = _position_component_rust(tokens[0])
        else:
            x = _position_component_rust(tokens[0])
            y = _position_component_rust(tokens[1])
        return (x, y) if x is not None and y is not None else None

    horizontal = None
    vertical = None
    index = 0
    while index < len(tokens):
        token = tokens[index].lower()
        if token in ('left', 'right'):
            offset = tokens[index + 1] if index + 1 < len(tokens) and tokens[index + 1].lower() not in ('left', 'right', 'top', 'bottom', 'center') else '0'
            horizontal = _position_component_rust(offset, end=token == 'right')
            index += 2 if offset != '0' else 1
        elif token in ('top', 'bottom'):
            offset = tokens[index + 1] if index + 1 < len(tokens) and tokens[index + 1].lower() not in ('left', 'right', 'top', 'bottom', 'center') else '0'
            vertical = _position_component_rust(offset, end=token == 'bottom')
            index += 2 if offset != '0' else 1
        elif token == 'center':
            if horizontal is None:
                horizontal = 'BackgroundPosition::Percent(50.0)'
            elif vertical is None:
                vertical = 'BackgroundPosition::Percent(50.0)'
            index += 1
        else:
            component = _position_component_rust(tokens[index])
            if horizontal is None:
                horizontal = component
            elif vertical is None:
                vertical = component
            index += 1
    return horizontal or zero, vertical or zero


def _css_image_rust(
    value: str,
    resource_var: str,
    font_size: float = 16.0,
) -> tuple[list[str], str] | None:
    raw = value.strip()
    function = re.match(
        r'(?is)^(?P<repeating>repeating-)?(?P<kind>linear|radial|conic)-gradient\((.*)\)$',
        raw,
    )
    if function:
        repeating = function.group('repeating') is not None
        kind = function.group('kind').lower()
        parts = _split_css_layers(function.group(3))
        if not parts:
            return None
        prelude, color_space = _gradient_color_space(parts[0])
        prelude_tokens = _split_respecting_parens(prelude)
        has_prelude = _style_color_rust(prelude_tokens[0] if prelude_tokens else '') is None
        if kind == 'linear':
            angle = 180.0
            corner_direction = 'None'
            direction = prelude.lower()
            directions = {
                'to top': 0.0, 'to top right': 45.0, 'to right top': 45.0,
                'to right': 90.0, 'to bottom right': 135.0, 'to right bottom': 135.0,
                'to bottom': 180.0, 'to bottom left': 225.0, 'to left bottom': 225.0,
                'to left': 270.0, 'to top left': 315.0, 'to left top': 315.0,
            }
            if direction.endswith('deg'):
                try:
                    angle = float(direction[:-3]) % 360.0
                except ValueError:
                    return None
            elif direction in directions:
                angle = directions[direction]
                corners = {
                    'to top right': (1.0, -1.0), 'to right top': (1.0, -1.0),
                    'to bottom right': (1.0, 1.0), 'to right bottom': (1.0, 1.0),
                    'to bottom left': (-1.0, 1.0), 'to left bottom': (-1.0, 1.0),
                    'to top left': (-1.0, -1.0), 'to left top': (-1.0, -1.0),
                }
                if direction in corners:
                    x, y = corners[direction]
                    corner_direction = f'Some(({x}, {y}))'
            elif direction:
                has_prelude = False
            stop_parts = parts[1:] if has_prelude else parts
            stops = _gradient_stops_rust(stop_parts, font_size)
            if stops is None:
                return None
            return [], (
                'CssImage::LinearGradient(CssLinearGradient { '
                f'angle_degrees: {angle}, corner_direction: {corner_direction}, '
                f'repeating: {str(repeating).lower()}, '
                f'color_space: {color_space}, stops: vec![{", ".join(stops)}] '
                '})'
            )
        if kind == 'radial':
            shape = 'RadialGradientShape::Ellipse'
            size = 'RadialGradientSize::FarthestCorner'
            center_x = 'BackgroundPosition::Percent(50.0)'
            center_y = 'BackgroundPosition::Percent(50.0)'
            if has_prelude:
                before_at, separator, after_at = prelude.partition(' at ')
                words = before_at.split()
                if 'circle' in words:
                    shape = 'RadialGradientShape::Circle'
                for keyword, rust in {
                    'closest-side': 'RadialGradientSize::ClosestSide',
                    'closest-corner': 'RadialGradientSize::ClosestCorner',
                    'farthest-side': 'RadialGradientSize::FarthestSide',
                    'farthest-corner': 'RadialGradientSize::FarthestCorner',
                }.items():
                    if keyword in words:
                        size = rust
                lengths = [
                    parse_length(word, font_size)
                    for word in words
                    if parse_length(word, font_size)
                ]
                if lengths:
                    second = lengths[1] if len(lengths) > 1 else lengths[0]
                    size = f'RadialGradientSize::Explicit({lengths[0]}, {second})'
                if separator:
                    position = _background_position_rust(after_at)
                    if position:
                        center_x, center_y = position
            stops = _gradient_stops_rust(
                parts[1:] if has_prelude else parts, font_size
            )
            if stops is None:
                return None
            return [], (
                'CssImage::RadialGradient(RadialGradient { '
                f'repeating: {str(repeating).lower()}, color_space: {color_space}, '
                f'shape: {shape}, size: {size}, center_x: {center_x}, center_y: {center_y}, '
                f'stops: vec![{", ".join(stops)}] '
                '})'
            )
        from_degrees = 0.0
        center_x = 'BackgroundPosition::Percent(50.0)'
        center_y = 'BackgroundPosition::Percent(50.0)'
        if has_prelude:
            match = re.search(r'from\s+(-?[\d.]+)deg', prelude)
            if match:
                from_degrees = float(match.group(1))
            if ' at ' in prelude:
                position = _background_position_rust(prelude.split(' at ', 1)[1])
                if position:
                    center_x, center_y = position
        stops = _gradient_stops_rust(parts[1:] if has_prelude else parts, font_size)
        if stops is None:
            return None
        return [], (
            'CssImage::ConicGradient(ConicGradient { '
            f'from_degrees: {from_degrees}, center_x: {center_x}, center_y: {center_y}, '
            f'repeating: {str(repeating).lower()}, color_space: {color_space}, '
            f'stops: vec![{", ".join(stops)}] '
            '})'
        )

    url = re.match(r'(?is)^url\(\s*(.*?)\s*\)$', raw)
    if not url:
        return None
    source = url.group(1).strip()
    if len(source) >= 2 and source[0] == source[-1] and source[0] in ('"', "'"):
        source = source[1:-1]
    if source.startswith('data:'):
        header, comma, payload = source.partition(',')
        if not comma:
            return None
        mime = header[5:].split(';', 1)[0] or 'text/plain'
        if ';base64' in header:
            import base64
            data = base64.b64decode(payload)
        else:
            data = urllib.parse.unquote_to_bytes(payload)
        if mime == 'image/svg+xml':
            # WPT frequently uses a size-less SVG whose root background is a
            # solid color as a self-contained CSS image. Skia's raster codec
            # cannot decode SVG bytes directly; lower that general solid-SVG
            # form to an equivalent two-stop CSS gradient.
            try:
                svg_text = data.decode('utf-8')
            except UnicodeDecodeError:
                svg_text = ''
            background = re.search(
                r'(?i)\bbackground(?:-color)?\s*:\s*([^;\'"}]+)', svg_text
            )
            if background and parse_color(background.group(1).strip()):
                color = background.group(1).strip()
                return _css_image_rust(
                    f'linear-gradient({color}, {color})', resource_var
                )
        sha = hashlib.sha256(data).hexdigest()
        byte_expr = f'vec![{", ".join(str(byte) for byte in data)}]'
        source_label = f'data:{mime};sha256={sha}'
    else:
        packaged = _packaged_resource(source)
        if packaged is not None:
            filename, source_label, mime, sha, _ = packaged
            byte_expr = _packaged_bytes_expr(filename)
        else:
            name = source.rsplit('/', 1)[-1]
            asset = _PAINT_ASSETS.get(name)
            if asset is None:
                return None
            filename, source_label, mime, sha = asset
            if filename is None:
                import base64
                data = base64.b64decode(_INLINE_PAINT_ASSETS[name])
                byte_expr = f'vec![{", ".join(str(byte) for byte in data)}]'
            else:
                byte_expr = (
                    'include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), '
                    f'"/../../../tools/accountability/data/wpt_assets/sp13p/{filename}"))'
                    '.as_slice().to_vec()'
                )
    line = (
        f'let {resource_var} = doc.register_image_resource('
        f'{json.dumps(source_label)}, {json.dumps(mime)}, {json.dumps(sha)}, {byte_expr});'
    )
    return [line], f'CssImage::Raster({resource_var})'


def _split_css_slash(value: str) -> tuple[str, str | None]:
    depth = 0
    quote = None
    for index, char in enumerate(value):
        if quote:
            if char == quote and (index == 0 or value[index - 1] != '\\'):
                quote = None
        elif char in ('"', "'"):
            quote = char
        elif char == '(':
            depth += 1
        elif char == ')':
            depth = max(0, depth - 1)
        elif char == '/' and depth == 0:
            return value[:index].strip(), value[index + 1:].strip()
    return value.strip(), None


def _background_repeat_rust(value: str) -> tuple[str, str] | None:
    parts = value.strip().lower().split()
    aliases = {
        'repeat-x': ('repeat', 'no-repeat'),
        'repeat-y': ('no-repeat', 'repeat'),
    }
    if len(parts) == 1 and parts[0] in aliases:
        parts = list(aliases[parts[0]])
    elif len(parts) == 1:
        parts *= 2
    if len(parts) != 2 or any(
        part not in ('repeat', 'no-repeat', 'round', 'space') for part in parts
    ):
        return None
    mapping = {
        'repeat': 'BackgroundRepeat::Repeat',
        'no-repeat': 'BackgroundRepeat::NoRepeat',
        'round': 'BackgroundRepeat::Round',
        'space': 'BackgroundRepeat::Space',
    }
    return mapping[parts[0]], mapping[parts[1]]


def _background_size_rust(value: str, font_size: float) -> str | None:
    value = value.strip().lower()
    if value == 'auto':
        return 'BackgroundSize::Auto'
    if value == 'cover':
        return 'BackgroundSize::Cover'
    if value == 'contain':
        return 'BackgroundSize::Contain'
    parts = _split_respecting_parens(value)
    if len(parts) == 1:
        parts.append('auto')
    if len(parts) != 2:
        return None
    lengths = [parse_length(part, font_size) for part in parts]
    if any(length is None for length in lengths):
        return None
    return f'BackgroundSize::Explicit({lengths[0]}, {lengths[1]})'


def _background_box_rust(value: str) -> str | None:
    return {
        'border-box': 'BackgroundClip::BorderBox',
        'padding-box': 'BackgroundClip::PaddingBox',
        'content-box': 'BackgroundClip::ContentBox',
        'text': 'BackgroundClip::Text',
    }.get(value.strip().lower())


def _new_background_layer(image_raw: str | None) -> dict[str, str | None]:
    return {
        'image': image_raw,
        'repeat_x': 'BackgroundRepeat::Repeat',
        'repeat_y': 'BackgroundRepeat::Repeat',
        'position_x': 'BackgroundPosition::Percent(0.0)',
        'position_y': 'BackgroundPosition::Percent(0.0)',
        'size': 'BackgroundSize::Auto',
        'origin': 'BackgroundClip::PaddingBox',
        'clip': 'BackgroundClip::BorderBox',
        'attachment': 'BackgroundAttachment::Scroll',
    }


def _parse_background_shorthand_layer(value: str, font_size: float) -> dict[str, str | None]:
    extracted = _extract_css_image(value)
    image = extracted[0] if extracted else None
    remainder = extracted[1] if extracted else value
    layer = _new_background_layer(image)

    before_slash, after_slash = _split_css_slash(remainder)
    before_tokens = _split_respecting_parens(before_slash)
    after_tokens = _split_respecting_parens(after_slash or '')

    repeat_tokens = []
    boxes = []
    position_tokens = []
    for token in before_tokens:
        lower = token.lower()
        if lower in ('repeat', 'no-repeat', 'round', 'space', 'repeat-x', 'repeat-y'):
            repeat_tokens.append(lower)
        elif lower in ('border-box', 'padding-box', 'content-box', 'text'):
            boxes.append(lower)
        elif lower in ('scroll', 'fixed', 'local'):
            layer['attachment'] = {
                'scroll': 'BackgroundAttachment::Scroll',
                'fixed': 'BackgroundAttachment::Fixed',
                'local': 'BackgroundAttachment::Local',
            }[lower]
        elif parse_color(token) is None and lower != 'currentcolor':
            position_tokens.append(token)
    if repeat_tokens:
        repeat = _background_repeat_rust(' '.join(repeat_tokens[:2]))
        if repeat:
            layer['repeat_x'], layer['repeat_y'] = repeat
    if position_tokens:
        position = _background_position_rust(' '.join(position_tokens))
        if position:
            layer['position_x'], layer['position_y'] = position
    if boxes:
        layer['origin'] = _background_box_rust(boxes[0])
        layer['clip'] = _background_box_rust(boxes[1] if len(boxes) > 1 else boxes[0])

    if after_slash:
        size_tokens = []
        trailing_tokens = []
        for token in after_tokens:
            lower = token.lower()
            if lower in ('repeat', 'no-repeat', 'round', 'space', 'repeat-x', 'repeat-y',
                         'scroll', 'fixed', 'local', 'border-box', 'padding-box',
                         'content-box', 'text'):
                trailing_tokens.append(token)
            elif not trailing_tokens:
                size_tokens.append(token)
            else:
                trailing_tokens.append(token)
        size = _background_size_rust(' '.join(size_tokens), font_size)
        if size:
            layer['size'] = size
        if trailing_tokens:
            repeat = _background_repeat_rust(' '.join(
                token for token in trailing_tokens
                if token.lower() in ('repeat', 'no-repeat', 'round', 'space', 'repeat-x', 'repeat-y')
            ))
            if repeat:
                layer['repeat_x'], layer['repeat_y'] = repeat
            trailing_boxes = [token for token in trailing_tokens if _background_box_rust(token)]
            if trailing_boxes:
                layer['origin'] = _background_box_rust(trailing_boxes[0])
                layer['clip'] = _background_box_rust(
                    trailing_boxes[1] if len(trailing_boxes) > 1 else trailing_boxes[0]
                )
            for token in trailing_tokens:
                if token.lower() in ('scroll', 'fixed', 'local'):
                    layer['attachment'] = {
                        'scroll': 'BackgroundAttachment::Scroll',
                        'fixed': 'BackgroundAttachment::Fixed',
                        'local': 'BackgroundAttachment::Local',
                    }[token.lower()]
    return layer


def _background_layers_rust(styles: dict, s: str, font_size: float) -> list[str]:
    layers = []
    if 'background' in styles:
        layers = [
            _parse_background_shorthand_layer(value, font_size)
            for value in _split_css_layers(styles['background'])
        ]
    if 'background-image' in styles:
        layers = [
            _new_background_layer(None if value.strip().lower() == 'none' else value.strip())
            for value in _split_css_layers(styles['background-image'])
        ]
    if not layers:
        return []

    def apply_list(prop: str, apply):
        if prop not in styles:
            return
        values = _split_css_layers(styles[prop])
        if not values:
            return
        for index, layer in enumerate(layers):
            apply(layer, values[index % len(values)])

    def apply_repeat(layer, value):
        repeat = _background_repeat_rust(value)
        if repeat:
            layer['repeat_x'], layer['repeat_y'] = repeat

    def apply_position(layer, value):
        position = _background_position_rust(value)
        if position:
            layer['position_x'], layer['position_y'] = position

    def apply_size(layer, value):
        size = _background_size_rust(value, font_size)
        if size:
            layer['size'] = size

    def apply_origin(layer, value):
        box = _background_box_rust(value)
        if box:
            layer['origin'] = box

    def apply_clip(layer, value):
        box = _background_box_rust(value)
        if box:
            layer['clip'] = box

    def apply_attachment(layer, value):
        attachment = {
            'scroll': 'BackgroundAttachment::Scroll',
            'fixed': 'BackgroundAttachment::Fixed',
            'local': 'BackgroundAttachment::Local',
        }.get(value.strip().lower())
        if attachment:
            layer['attachment'] = attachment

    apply_list('background-repeat', apply_repeat)
    apply_list('background-position', apply_position)
    apply_list('background-size', apply_size)
    apply_list('background-origin', apply_origin)
    apply_list('background-clip', apply_clip)
    apply_list('background-attachment', apply_attachment)

    node = re.search(r'node_mut\(([^)]+)\)', s)
    prefix = re.sub(r'\W+', '_', node.group(1) if node else 'style')
    prereqs = []
    emitted = []
    for index, layer in enumerate(layers):
        if not layer['image']:
            continue
        parsed = _css_image_rust(
            layer['image'], f'{prefix}_background_image_{index}', font_size
        )
        if parsed is None:
            continue
        image_prereqs, image = parsed
        prereqs.extend(image_prereqs)
        emitted.append(
            'BackgroundLayer { '
            f'image: {image}, repeat_x: {layer["repeat_x"]}, repeat_y: {layer["repeat_y"]}, '
            f'position_x: {layer["position_x"]}, position_y: {layer["position_y"]}, '
            f'size: {layer["size"]}, origin: {layer["origin"]}, clip: {layer["clip"]}, '
            f'attachment: {layer["attachment"]} '
            '}'
        )
    return prereqs + [
        f'{s}.background_layers = vec![{", ".join(emitted)}];',
        f'{s}.background_linear_gradient = None;',
    ]


def _mask_layers_rust(styles: dict, s: str, font_size: float) -> list[str]:
    image_value = styles.get('mask-image', styles.get('-webkit-mask-image'))
    if not image_value:
        return []
    layers = [
        _new_background_layer(None if value.strip().lower() == 'none' else value.strip())
        for value in _split_css_layers(image_value)
    ]
    for layer in layers:
        # CSS Masking uses the border box as its default positioning area.
        layer['origin'] = 'BackgroundClip::BorderBox'
        layer['clip'] = 'BackgroundClip::BorderBox'

    def apply_list(standard: str, prefixed: str, apply):
        value = styles.get(standard, styles.get(prefixed))
        if not value:
            return
        values = _split_css_layers(value)
        for index, layer in enumerate(layers):
            apply(layer, values[index % len(values)])

    def apply_repeat(layer, value):
        repeat = _background_repeat_rust(value)
        if repeat:
            layer['repeat_x'], layer['repeat_y'] = repeat

    def apply_size(layer, value):
        size = _background_size_rust(value, font_size)
        if size:
            layer['size'] = size

    def apply_position(layer, value):
        position = _background_position_rust(value)
        if position:
            layer['position_x'], layer['position_y'] = position

    apply_list('mask-repeat', '-webkit-mask-repeat', apply_repeat)
    apply_list('mask-size', '-webkit-mask-size', apply_size)
    apply_list('mask-position', '-webkit-mask-position', apply_position)

    node = re.search(r'node_mut\(([^)]+)\)', s)
    prefix = re.sub(r'\W+', '_', node.group(1) if node else 'style')
    prereqs = []
    emitted = []
    for index, layer in enumerate(layers):
        if not layer['image']:
            continue
        parsed = _css_image_rust(
            layer['image'], f'{prefix}_mask_image_{index}', font_size
        )
        if parsed is None:
            continue
        image_prereqs, image = parsed
        prereqs.extend(image_prereqs)
        emitted.append(
            'BackgroundLayer { '
            f'image: {image}, repeat_x: {layer["repeat_x"]}, repeat_y: {layer["repeat_y"]}, '
            f'position_x: {layer["position_x"]}, position_y: {layer["position_y"]}, '
            f'size: {layer["size"]}, origin: {layer["origin"]}, clip: {layer["clip"]}, '
            'attachment: BackgroundAttachment::Scroll '
            '}'
        )
    return prereqs + [f'{s}.mask_layers = vec![{", ".join(emitted)}];']


def _split_all_css_slashes(value: str) -> list[str]:
    result = []
    current = []
    depth = 0
    quote = None
    for index, char in enumerate(value):
        if quote:
            current.append(char)
            if char == quote and (index == 0 or value[index - 1] != '\\'):
                quote = None
        elif char in ('"', "'"):
            quote = char
            current.append(char)
        elif char == '(':
            depth += 1
            current.append(char)
        elif char == ')':
            depth = max(0, depth - 1)
            current.append(char)
        elif char == '/' and depth == 0:
            result.append(''.join(current).strip())
            current = []
        else:
            current.append(char)
    result.append(''.join(current).strip())
    return result


def _expand_four(values: list[str]) -> list[str] | None:
    if len(values) == 1:
        return values * 4
    if len(values) == 2:
        return [values[0], values[1], values[0], values[1]]
    if len(values) == 3:
        return [values[0], values[1], values[2], values[1]]
    if len(values) == 4:
        return values
    return None


def _border_image_length_rust(value: str, font_size: float) -> str | None:
    value = value.strip().lower()
    if value == 'auto':
        return 'BorderImageLength::Auto'
    if re.fullmatch(r'[+]?[\d.]+', value):
        return f'BorderImageLength::Number({float(value)})'
    length = parse_length(value, font_size)
    return f'BorderImageLength::Length({length})' if length else None


def _border_image_repeat_rust(value: str) -> tuple[str, str] | None:
    parts = value.strip().lower().split()
    if len(parts) == 1:
        parts *= 2
    if len(parts) != 2 or any(part not in ('stretch', 'repeat', 'round', 'space') for part in parts):
        return None
    mapping = {
        'stretch': 'BorderImageRepeat::Stretch',
        'repeat': 'BorderImageRepeat::Repeat',
        'round': 'BorderImageRepeat::Round',
        'space': 'BorderImageRepeat::Space',
    }
    return mapping[parts[0]], mapping[parts[1]]


def _border_image_rust(styles: dict, s: str, font_size: float) -> list[str]:
    source = None
    slice_values = ['BorderImageLength::Length(Length::percent(100.0))'] * 4
    fill = False
    width_values = ['BorderImageLength::Number(1.0)'] * 4
    outset_values = ['BorderImageLength::Number(0.0)'] * 4
    repeat_x = repeat_y = 'BorderImageRepeat::Stretch'
    shorthand_priority = (
        styles.cascade_priority.get('border-image')
        if isinstance(styles, CssDeclarations) else None
    )

    def longhand_wins(prop: str) -> bool:
        if prop not in styles:
            return False
        if shorthand_priority is None or not isinstance(styles, CssDeclarations):
            return True
        return styles.cascade_priority.get(prop, shorthand_priority) >= shorthand_priority

    if 'border-image' in styles:
        extracted = _extract_css_image(styles['border-image'])
        if extracted:
            source, remainder = extracted
            slash_parts = _split_all_css_slashes(remainder)
            first_tokens = _split_respecting_parens(slash_parts[0])
            repeat_tokens = [token for token in first_tokens if token.lower() in ('stretch', 'repeat', 'round', 'space')]
            slice_tokens = [token for token in first_tokens if token.lower() not in ('stretch', 'repeat', 'round', 'space', 'fill')]
            fill = 'fill' in [token.lower() for token in first_tokens]
            expanded = _expand_four(slice_tokens)
            if expanded:
                parsed = [_border_image_length_rust(token, font_size) for token in expanded]
                if all(parsed):
                    slice_values = parsed
            if len(slash_parts) >= 2 and slash_parts[1]:
                width_tokens = _split_respecting_parens(slash_parts[1])
                trailing = [token for token in width_tokens if token.lower() in ('stretch', 'repeat', 'round', 'space')]
                repeat_tokens.extend(trailing)
                width_tokens = [token for token in width_tokens if token not in trailing]
                expanded = _expand_four(width_tokens)
                if expanded:
                    parsed = [_border_image_length_rust(token, font_size) for token in expanded]
                    if all(parsed):
                        width_values = parsed
            if len(slash_parts) >= 3 and slash_parts[2]:
                outset_tokens = _split_respecting_parens(slash_parts[2])
                trailing = [token for token in outset_tokens if token.lower() in ('stretch', 'repeat', 'round', 'space')]
                repeat_tokens.extend(trailing)
                outset_tokens = [token for token in outset_tokens if token not in trailing]
                expanded = _expand_four(outset_tokens)
                if expanded:
                    parsed = [_border_image_length_rust(token, font_size) for token in expanded]
                    if all(parsed):
                        outset_values = parsed
            if repeat_tokens:
                repeat = _border_image_repeat_rust(' '.join(repeat_tokens[:2]))
                if repeat:
                    repeat_x, repeat_y = repeat

    if longhand_wins('border-image-source'):
        source = styles['border-image-source'].strip()
    if source is None or source.lower() == 'none':
        return []

    if longhand_wins('border-image-slice'):
        tokens = _split_respecting_parens(styles['border-image-slice'])
        fill = 'fill' in [token.lower() for token in tokens]
        tokens = [token for token in tokens if token.lower() != 'fill']
        expanded = _expand_four(tokens)
        if expanded:
            parsed = [_border_image_length_rust(token, font_size) for token in expanded]
            if all(parsed):
                slice_values = parsed
    if longhand_wins('border-image-width'):
        expanded = _expand_four(_split_respecting_parens(styles['border-image-width']))
        if expanded:
            parsed = [_border_image_length_rust(token, font_size) for token in expanded]
            if all(parsed):
                width_values = parsed
    if longhand_wins('border-image-outset'):
        expanded = _expand_four(_split_respecting_parens(styles['border-image-outset']))
        if expanded:
            parsed = [_border_image_length_rust(token, font_size) for token in expanded]
            if all(parsed):
                outset_values = parsed
    if longhand_wins('border-image-repeat'):
        repeat = _border_image_repeat_rust(styles['border-image-repeat'])
        if repeat:
            repeat_x, repeat_y = repeat

    node = re.search(r'node_mut\(([^)]+)\)', s)
    prefix = re.sub(r'\W+', '_', node.group(1) if node else 'style')
    parsed_source = _css_image_rust(source, f'{prefix}_border_image')
    if parsed_source is None:
        return []
    prereqs, image = parsed_source
    return prereqs + [
        f'{s}.border_image = Some(BorderImage {{ source: {image}, '
        f'slice: [{", ".join(slice_values)}], fill: {str(fill).lower()}, '
        f'width: [{", ".join(width_values)}], outset: [{", ".join(outset_values)}], '
        f'repeat_x: {repeat_x}, repeat_y: {repeat_y} }});'
    ]


def _select_background_clip_layer(styles: dict, value: str) -> str:
    """Choose the background-clip layer that applies to background-color."""
    clips = _split_css_layers(value)
    valid_clips = {'border-box', 'padding-box', 'content-box'}
    if any(clip not in valid_clips for clip in clips):
        return value.strip()
    if len(clips) <= 1:
        return value.strip()

    images = _split_css_layers(styles.get('background-image', ''))
    image_count = len(images) if images and images[0] else 1
    index = min(max(image_count - 1, 0), len(clips) - 1)
    return clips[index]


def _edge_lengths_from_styles(styles: dict, prefix: str, font_size: float) -> tuple[float, float, float, float]:
    """Resolve top/right/bottom/left fixed lengths from CSS shorthand and sides."""
    top = right = bottom = left = 0.0

    def assign_4(value: str):
        nonlocal top, right, bottom, left
        parts = _split_respecting_parens(value)
        vals = [_css_length_px(p, font_size) for p in parts]
        if not vals or any(v is None for v in vals):
            return
        if len(vals) == 1:
            top = right = bottom = left = vals[0]
        elif len(vals) == 2:
            top = bottom = vals[0]
            right = left = vals[1]
        elif len(vals) == 3:
            top = vals[0]
            right = left = vals[1]
            bottom = vals[2]
        elif len(vals) == 4:
            top, right, bottom, left = vals

    for prop, val in styles.items():
        if prop == prefix:
            assign_4(val)
        elif prop == f'{prefix}-top':
            v = _css_length_px(val, font_size)
            if v is not None:
                top = v
        elif prop == f'{prefix}-right':
            v = _css_length_px(val, font_size)
            if v is not None:
                right = v
        elif prop == f'{prefix}-bottom':
            v = _css_length_px(val, font_size)
            if v is not None:
                bottom = v
        elif prop == f'{prefix}-left':
            v = _css_length_px(val, font_size)
            if v is not None:
                left = v

    return top, right, bottom, left


def _border_widths_from_styles(styles: dict, font_size: float) -> tuple[float, float, float, float]:
    """Resolve effective static border widths from CSS shorthand and sides."""
    widths = {'top': 0.0, 'right': 0.0, 'bottom': 0.0, 'left': 0.0}
    styles_by_side = {'top': None, 'right': None, 'bottom': None, 'left': None}

    def parse_bw(v: str) -> float | None:
        bw = parse_border_width(v, font_size)
        return float(bw) if bw is not None else None

    def assign_width_4(value: str):
        parts = _split_respecting_parens(value)
        vals = [parse_bw(p) for p in parts]
        if not vals or any(v is None for v in vals):
            return
        if len(vals) == 1:
            for side in widths:
                widths[side] = vals[0]
        elif len(vals) == 2:
            widths['top'] = widths['bottom'] = vals[0]
            widths['right'] = widths['left'] = vals[1]
        elif len(vals) == 3:
            widths['top'] = vals[0]
            widths['right'] = widths['left'] = vals[1]
            widths['bottom'] = vals[2]
        elif len(vals) == 4:
            for side, val in zip(['top', 'right', 'bottom', 'left'], vals):
                widths[side] = val

    def apply_border_shorthand(value: str, sides: list[str]):
        parts = _split_respecting_parens(value)
        width = None
        style = None
        for part in parts:
            if width is None:
                width = parse_bw(part)
            if style is None and border_style_to_rust(part):
                style = part.strip()
        for side in sides:
            if width is not None:
                widths[side] = width
            if style is not None:
                styles_by_side[side] = style

    for prop, val in styles.items():
        if prop == 'border':
            apply_border_shorthand(val, ['top', 'right', 'bottom', 'left'])
        elif prop in ('border-top', 'border-right', 'border-bottom', 'border-left'):
            apply_border_shorthand(val, [prop.split('-')[1]])
        elif prop == 'border-width':
            assign_width_4(val)
        elif prop in ('border-top-width', 'border-right-width', 'border-bottom-width', 'border-left-width'):
            side = prop.split('-')[1]
            width = parse_bw(val)
            if width is not None:
                widths[side] = width
        elif prop == 'border-style':
            parts = _split_respecting_parens(val)
            if len(parts) == 1:
                sides_vals = [parts[0]] * 4
            elif len(parts) == 2:
                sides_vals = [parts[0], parts[1], parts[0], parts[1]]
            elif len(parts) == 3:
                sides_vals = [parts[0], parts[1], parts[2], parts[1]]
            elif len(parts) == 4:
                sides_vals = parts
            else:
                sides_vals = []
            for side, style in zip(['top', 'right', 'bottom', 'left'], sides_vals):
                styles_by_side[side] = style
        elif prop in ('border-top-style', 'border-right-style', 'border-bottom-style', 'border-left-style'):
            styles_by_side[prop.split('-')[1]] = val.strip()

    for side in widths:
        if styles_by_side[side] in (None, 'none', 'hidden'):
            widths[side] = 0.0

    return widths['top'], widths['right'], widths['bottom'], widths['left']


def _border_radius_basis(styles: dict, font_size: float) -> tuple[float, float] | None:
    """Return border-box width/height for resolving percentage border radii."""
    width = _css_length_px(styles.get('width', ''), font_size)
    height = _css_length_px(styles.get('height', ''), font_size)
    if width is None or height is None:
        return None

    if styles.get('box-sizing', '').strip() == 'border-box':
        return width, height

    pt, pr, pb, pl = _edge_lengths_from_styles(styles, 'padding', font_size)
    bt, br, bb, bl = _border_widths_from_styles(styles, font_size)
    return width + pl + pr + bl + br, height + pt + pb + bt + bb


def _computed_font_size(value: str, inherited_font_size: float) -> float:
    value = value.strip().rstrip(';').strip().lower()
    keyword_sizes = {
        'xx-small': 9.0, 'x-small': 10.0, 'small': 13.333, 'medium': 16.0,
        'large': 18.0, 'x-large': 24.0, 'xx-large': 32.0, 'xxx-large': 48.0,
    }
    if value in keyword_sizes:
        return keyword_sizes[value]
    if value == 'smaller':
        return inherited_font_size * 0.833
    if value == 'larger':
        return inherited_font_size * 1.2
    if value == '0':
        return 0.0
    factors = {
        'px': 1.0, 'pt': 96.0 / 72.0, 'pc': 16.0, 'in': 96.0,
        'cm': 96.0 / 2.54, 'mm': 96.0 / 25.4, 'em': inherited_font_size,
        'rem': 16.0, '%': inherited_font_size / 100.0,
    }
    m = re.match(r'^(-?[\d.]+)(px|pt|pc|in|cm|mm|em|rem|%)$', value)
    if m:
        return float(m.group(1)) * factors[m.group(2)]
    if value in {'inherit', 'unset'}:
        return inherited_font_size
    raise UnsupportedFontShorthand(f"unsupported font-size value: {value}")


def _computed_inherited_line_height(value: str, font_size: float) -> str:
    """Serialize the computed value that descendants inherit."""
    value = value.strip().lower()
    if value == 'normal':
        return value
    if re.match(r'^[-+]?[\d.]+$', value):
        return value  # unitless numbers inherit as numbers
    m = re.match(r'^([-+]?[\d.]+)%$', value)
    if m:
        return f"{float(m.group(1)) * font_size / 100.0}px"
    m = re.match(r'^([-+]?[\d.]+)(px|pt|pc|in|cm|mm|em|rem)$', value)
    if m:
        unit = m.group(2)
        factors = {
            'px': 1.0, 'pt': 96.0 / 72.0, 'pc': 16.0, 'in': 96.0,
            'cm': 96.0 / 2.54, 'mm': 96.0 / 25.4,
            'em': font_size, 'rem': 16.0,
        }
        return f"{float(m.group(1)) * factors[unit]}px"
    raise UnsupportedFontShorthand(f"unsupported inherited line-height: {value}")


def _sp17_logical_sides(writing_mode: str, direction: str) -> dict[str, str]:
    """Resolve logical start/end edges to physical sides for one element."""
    if writing_mode == 'horizontal-tb':
        block_start, block_end = 'top', 'bottom'
        inline_start, inline_end = (
            ('right', 'left') if direction == 'rtl' else ('left', 'right')
        )
    elif writing_mode in {'vertical-rl', 'sideways-rl'}:
        block_start, block_end = 'right', 'left'
        inline_start, inline_end = (
            ('bottom', 'top') if direction == 'rtl' else ('top', 'bottom')
        )
    elif writing_mode == 'vertical-lr':
        block_start, block_end = 'left', 'right'
        inline_start, inline_end = (
            ('bottom', 'top') if direction == 'rtl' else ('top', 'bottom')
        )
    elif writing_mode == 'sideways-lr':
        block_start, block_end = 'left', 'right'
        inline_start, inline_end = (
            ('top', 'bottom') if direction == 'rtl' else ('bottom', 'top')
        )
    else:
        raise ValueError(f"unsupported computed writing-mode: {writing_mode!r}")
    return {
        'block-start': block_start,
        'block-end': block_end,
        'inline-start': inline_start,
        'inline-end': inline_end,
    }


def _sp17_declarations_in_cascade_order(styles: dict) -> list[tuple[str, str]]:
    indexed = list(enumerate(styles.items()))
    has_logical_box_property = any(
        re.fullmatch(r'(?:min-|max-)?(?:block|inline)-size', prop)
        or re.fullmatch(r'contain-intrinsic-(?:block|inline)-size', prop)
        or re.fullmatch(
            r'(?:margin|padding|inset)-(?:block|inline)(?:-(?:start|end))?',
            prop,
        )
        or re.fullmatch(
            r'border-(?:block|inline)(?:-(?:start|end))?'
            r'(?:-(?:width|style|color))?',
            prop,
        )
        or re.fullmatch(
            r'border-(?:start|end)-(?:start|end)-radius', prop
        )
        for _, (prop, _) in indexed
    )
    if not isinstance(styles, CssDeclarations) or not has_logical_box_property:
        return [item for _, item in indexed]

    # Low-priority declarations are materialized first. A later declaration
    # that maps to the same physical field therefore wins naturally.
    def key(item):
        index, (prop, _) = item
        return styles.cascade_priority.get(
            prop, (0, 0, 0, 0, 0, 0, index)
        ), index

    return [item for _, item in sorted(indexed, key=key)]


def _assign_resolved_declaration(
    result: OrderedDict, prop: str, value: str
) -> None:
    # Reinsert an overwritten field at the winning declaration's location so
    # a following physical shorthand still observes the correct cascade order.
    if prop in result:
        del result[prop]
    result[prop] = value


def _expand_sp17_logical_declaration(
    prop: str, value: str, sides: dict[str, str], writing_mode: str,
) -> list[tuple[str, str]] | None:
    contain_size_match = re.fullmatch(
        r'contain-intrinsic-(block|inline)-size', prop
    )
    if contain_size_match:
        axis = contain_size_match.group(1)
        physical_axis = (
            'height' if (axis == 'block') == (writing_mode == 'horizontal-tb')
            else 'width'
        )
        return [(f'contain-intrinsic-{physical_axis}', value)]

    size_match = re.fullmatch(r'(min-|max-)?(block|inline)-size', prop)
    if size_match:
        prefix = size_match.group(1) or ''
        axis = size_match.group(2)
        physical_axis = (
            'height' if (axis == 'block') == (writing_mode == 'horizontal-tb')
            else 'width'
        )
        return [(f'{prefix}{physical_axis}', value)]

    side_match = re.fullmatch(
        r'(margin|padding|inset)-(block|inline)(?:-(start|end))?', prop
    )
    if side_match:
        family, axis, edge = side_match.groups()

        def physical_name(logical_edge: str) -> str:
            side = sides[f'{axis}-{logical_edge}']
            return side if family == 'inset' else f'{family}-{side}'

        if edge:
            return [(physical_name(edge), value)]
        parts = _split_respecting_parens(value)
        if len(parts) == 1:
            parts *= 2
        if len(parts) != 2:
            return None
        return [
            (physical_name('start'), parts[0]),
            (physical_name('end'), parts[1]),
        ]

    border_match = re.fullmatch(
        r'border-(block|inline)(?:-(start|end))?(?:-(width|style|color))?',
        prop,
    )
    if border_match:
        axis, edge, component = border_match.groups()

        def physical_name(logical_edge: str) -> str:
            base = f"border-{sides[f'{axis}-{logical_edge}']}"
            return f'{base}-{component}' if component else base

        if edge:
            return [(physical_name(edge), value)]
        parts = (
            _split_respecting_parens(value)
            if component in {'width', 'style', 'color'} else [value]
        )
        if len(parts) == 1:
            parts *= 2
        if len(parts) != 2:
            return None
        return [
            (physical_name('start'), parts[0]),
            (physical_name('end'), parts[1]),
        ]

    corner_match = re.fullmatch(
        r'border-(start|end)-(start|end)-radius', prop
    )
    if corner_match:
        block_edge, inline_edge = corner_match.groups()
        physical = {
            sides[f'block-{block_edge}'], sides[f'inline-{inline_edge}']
        }
        corners = {
            frozenset({'top', 'left'}): 'border-top-left-radius',
            frozenset({'top', 'right'}): 'border-top-right-radius',
            frozenset({'bottom', 'right'}): 'border-bottom-right-radius',
            frozenset({'bottom', 'left'}): 'border-bottom-left-radius',
        }
        return [(corners[frozenset(physical)], value)]
    return None


def resolve_sp17_logical_properties(styles: dict) -> CssDeclarations:
    """Resolve logical CSS only after this element's writing direction exists.

    ComputedStyle currently stores physical box fields. This transaction keeps
    the authored cascade intact through parsing and selector matching, derives
    the winning writing-mode/direction, then converts each logical declaration
    exactly once. It is the porter-side computed-value boundary used until W1
    teaches layout to consume authoritative logical geometry directly.
    """
    writing_mode = styles.get('writing-mode', 'horizontal-tb').strip().lower()
    direction = styles.get('direction', 'ltr').strip().lower()
    if writing_mode == 'inherit':
        writing_mode = 'horizontal-tb'
    if direction == 'inherit':
        direction = 'ltr'
    sides = _sp17_logical_sides(writing_mode, direction)
    result = CssDeclarations()
    for prop, value in _sp17_declarations_in_cascade_order(styles):
        priority = (
            styles.cascade_priority.get(prop)
            if isinstance(styles, CssDeclarations) else None
        )
        important = (
            styles.important.get(prop, False)
            if isinstance(styles, CssDeclarations) else False
        )
        expanded = _expand_sp17_logical_declaration(
            prop, value, sides, writing_mode
        )
        if expanded is None:
            _assign_resolved_declaration(result, prop, value)
            if priority is not None:
                result.cascade_priority[prop] = priority
            result.important[prop] = important
            continue
        for physical_prop, physical_value in expanded:
            _assign_resolved_declaration(result, physical_prop, physical_value)
            if priority is not None:
                result.cascade_priority[physical_prop] = priority
            result.important[physical_prop] = important
    return result


def generate_style_code(
    styles: dict,
    var_name: str,
    inherited_font_size: float = 16.0,
    effective_zoom: float = 1.0,
) -> list[str]:
    """Generate Rust code lines to set style properties on a node.
    
    inherited_font_size: font-size in px inherited from parent for em resolution.
    """
    styles = resolve_sp17_logical_properties(styles)
    lines = []
    s = f"doc.node_mut({var_name}).style"

    # Determine the effective font-size for this node (used for em resolution).
    # Process font-size first so other properties can use the correct em base.
    font_size = inherited_font_size
    fs_val = styles.get('font-size', '')
    if not fs_val and 'font' in styles:
        # The `font` shorthand also carries font-size; extract the size token so
        # the effective font-size threads correctly to descendants (incl. text).
        m = re.search(r'(-?[\d.]+(?:px|pt|em|rem|%))', styles['font'])
        if m:
            fs_val = m.group(1)
    if fs_val:
        if is_real_font_profile():
            font_size = _computed_font_size(fs_val, inherited_font_size)
        else:
            fs_val = fs_val.strip().rstrip(';').strip()
            m = re.match(r'^(-?[\d.]+)px$', fs_val)
            if m:
                font_size = float(m.group(1))
            else:
                m = re.match(r'^(-?[\d.]+)em$', fs_val)
                if m:
                    font_size = float(m.group(1)) * inherited_font_size
                else:
                    m = re.match(r'^(-?[\d.]+)rem$', fs_val)
                    if m:
                        font_size = float(m.group(1)) * 16.0
                    elif fs_val in ('small',):
                        font_size = 13.333
                    elif fs_val in ('smaller',):
                        font_size = inherited_font_size * 0.833
                    elif fs_val in ('larger',):
                        font_size = inherited_font_size * 1.2
                    elif fs_val in ('large',):
                        font_size = 18.0
                    elif fs_val in ('x-large',):
                        font_size = 24.0
                    elif fs_val in ('xx-large',):
                        font_size = 32.0
                    elif fs_val in ('x-small',):
                        font_size = 10.0
                    elif fs_val in ('xx-small',):
                        font_size = 9.0
                    else:
                        m = re.match(r'^(-?[\d.]+)pt$', fs_val)
                        if m:
                            font_size = float(m.group(1)) * 4.0 / 3.0
                        else:
                            m = re.match(r'^(-?[\d.]+)(pc|in|cm|mm)$', fs_val)
                            if m:
                                factors = {
                                    'pc': 16.0,
                                    'in': 96.0,
                                    'cm': 96.0 / 2.54,
                                    'mm': 96.0 / 25.4,
                                }
                                font_size = float(m.group(1)) * factors[m.group(2)]
                            else:
                                m = re.match(r'^(-?[\d.]+)%$', fs_val)
                                if m:
                                    font_size = float(m.group(1)) / 100.0 * inherited_font_size
                                elif fs_val == '0':
                                    font_size = 0.0

    global _ACTIVE_CSS_ZOOM, _ACTIVE_FONT_RELATIVE_RESOLVER, _ACTIVE_LINE_HEIGHT_PX
    previous_zoom = _ACTIVE_CSS_ZOOM
    previous_resolver = _ACTIVE_FONT_RELATIVE_RESOLVER
    previous_line_height = _ACTIVE_LINE_HEIGHT_PX
    _ACTIVE_CSS_ZOOM = effective_zoom
    _ACTIVE_LINE_HEIGHT_PX = _used_line_height_px(styles, font_size)
    try:
        radius_basis = _border_radius_basis(styles, font_size)

        font_props = {
            'font-family', 'font-size', 'font-weight', 'font-style', 'font-stretch',
            'font-variant-caps', 'line-height',
        }
        if is_real_font_profile():
            for prop in (
                'font-family', 'font-size', 'font-weight', 'font-style', 'font-stretch',
                'font-variant-caps', 'line-height',
            ):
                if prop in styles:
                    code = generate_single_style(prop, styles[prop], s, font_size, radius_basis)
                    if code:
                        lines.extend(code if isinstance(code, list) else [code])

        has_relative_lengths = is_real_font_profile() and any(
            re.search(
                r'(?<![\w-])[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:ch|ex|lh)\b',
                str(value),
                re.I,
            )
            for value in styles.values()
        )
        if has_relative_lengths:
            resolver = f"{var_name}_font_relative"
            lines.append(
                f"let {resolver} = openui_text::FontRelativeLengthResolver::from_style_in_collection(&doc.node({var_name}).style, std::sync::Arc::clone(doc.font_collection()));"
            )
            _ACTIVE_FONT_RELATIVE_RESOLVER = resolver
        for prop, val in styles.items():
            if prop == 'zoom':
                continue
            if is_real_font_profile() and prop in font_props:
                continue
            if prop == 'background-clip':
                val = _select_background_clip_layer(styles, val)
            if prop in ('background', 'background-color', 'box-shadow') and 'currentcolor' in val.lower():
                current = styles.get('color', 'black')
                if parse_color(current) is None:
                    current = 'black'
                val = re.sub(r'(?i)currentcolor', current, val)
            if prop == 'box-shadow' and val.strip().lower() != 'none':
                current = styles.get('color', 'black')
                if parse_color(current) is None:
                    current = 'black'
                layers = _split_css_layers(val)
                val = ', '.join(
                    layer
                    if any(
                        parse_color(token) is not None
                        for token in _split_respecting_parens(layer)
                    )
                    else f'{layer} {current}'
                    for layer in layers
                )
            code = generate_single_style(prop, val, s, font_size, radius_basis)
            if code:
                lines.extend(code if isinstance(code, list) else [code])

        if any(
            prop in styles
            for prop in (
                'mask-image', 'mask-repeat', 'mask-size', 'mask-position',
                '-webkit-mask-image', '-webkit-mask-repeat',
                '-webkit-mask-size', '-webkit-mask-position',
            )
        ):
            lines.extend(_mask_layers_rust(styles, s, font_size))

        if EMIT_PAINT_LAYERS:
            lines.extend(_background_layers_rust(styles, s, font_size))
            lines.extend(_border_image_rust(styles, s, font_size))

        # CSS display blockification is part of the computed value of floated
        # boxes. Generated builders store computed styles, so materialize it
        # after author declarations rather than teaching layout about authored
        # inline display values.
        if styles.get('float', '').strip() in ('left', 'right'):
            display = styles.get('display', 'inline').strip()
            if display in ('', 'inline', 'inline-block'):
                lines.append(f"{s}.display = Display::Block;")
            elif display == 'inline-flex':
                lines.append(f"{s}.display = Display::Flex;")

        # The Rust style model intentionally has no CSS zoom field. Lower the
        # cumulative used zoom into fixed geometry and the physical font size.
        # Font inheritance continues to use the unzoomed computed size returned
        # below, while every box in the zoomed subtree receives the same scale.
        if effective_zoom != 1.0:
            lines.append(f"{s}.font_size = {_zoomed_px(font_size)};")
    finally:
        _ACTIVE_FONT_RELATIVE_RESOLVER = previous_resolver
        _ACTIVE_CSS_ZOOM = previous_zoom
        _ACTIVE_LINE_HEIGHT_PX = previous_line_height

    return lines, font_size


def _decode_css_string(token: str) -> str | None:
    """Decode one CSS quoted string, including hexadecimal escapes."""
    token = token.strip()
    if len(token) < 2 or token[0] not in "\"'" or token[-1] != token[0]:
        return None
    value = token[1:-1]

    def replace_escape(match):
        escaped = match.group(1)
        if re.fullmatch(r'[0-9a-fA-F]{1,6}\s?', escaped):
            codepoint = int(escaped.strip(), 16)
            if codepoint == 0 or codepoint > 0x10ffff:
                return '\ufffd'
            return chr(codepoint)
        if escaped in ('\n', '\r', '\r\n', '\f'):
            return ''
        return escaped[-1]

    return re.sub(r'\\([0-9a-fA-F]{1,6}\s?|\r\n|[^\n\r\f])', replace_escape, value)


def _css_value_tokens(value: str) -> list[str] | None:
    """Tokenize the small generated-content grammar without losing strings."""
    tokens = []
    i = 0
    while i < len(value):
        if value[i].isspace():
            i += 1
            continue
        if value[i] in "\"'":
            quote = value[i]
            start = i
            i += 1
            while i < len(value):
                if value[i] == '\\':
                    i += 2
                elif value[i] == quote:
                    i += 1
                    break
                else:
                    i += 1
            else:
                return None
            tokens.append(value[start:i])
            continue
        start = i
        depth = 0
        quote = None
        while i < len(value):
            ch = value[i]
            if quote:
                if ch == '\\':
                    i += 2
                    continue
                if ch == quote:
                    quote = None
            elif ch in "\"'":
                quote = ch
            elif ch == '(':
                depth += 1
            elif ch == ')':
                depth -= 1
            elif ch.isspace() and depth == 0:
                break
            i += 1
        if depth != 0 or quote:
            return None
        tokens.append(value[start:i])
    return tokens


def _counter_style_rust(value: str) -> str | None:
    return {
        'decimal': 'CounterStyle::Decimal',
        'decimal-leading-zero': 'CounterStyle::DecimalLeadingZero',
        'lower-alpha': 'CounterStyle::LowerAlpha',
        'lower-latin': 'CounterStyle::LowerAlpha',
        'upper-alpha': 'CounterStyle::UpperAlpha',
        'upper-latin': 'CounterStyle::UpperAlpha',
        'lower-roman': 'CounterStyle::LowerRoman',
        'upper-roman': 'CounterStyle::UpperRoman',
    }.get(value.strip().lower())


def parse_generated_content(value: str) -> list[str] | None:
    if value.strip().lower() in ('normal', 'none'):
        return []
    tokens = _css_value_tokens(value)
    if not tokens:
        return None
    output = []
    for token in tokens:
        decoded = _decode_css_string(token)
        if decoded is not None:
            output.append(
                f'GeneratedContentItem::String("{_rust_escape_string(decoded)}".to_string())'
            )
            continue
        lower = token.lower()
        quote_items = {
            'open-quote': 'GeneratedContentItem::OpenQuote',
            'close-quote': 'GeneratedContentItem::CloseQuote',
            'no-open-quote': 'GeneratedContentItem::NoOpenQuote',
            'no-close-quote': 'GeneratedContentItem::NoCloseQuote',
        }
        if lower in quote_items:
            output.append(quote_items[lower])
            continue
        attr = re.fullmatch(r'attr\(\s*([\w:-]+)(?:\s+[^)]*)?\s*\)', token, re.I)
        if attr:
            output.append(
                'GeneratedContentItem::Attribute('
                f'"{_rust_escape_string(attr.group(1))}".to_string())'
            )
            continue
        counter = re.fullmatch(r'counter\(\s*([\w-]+)(?:\s*,\s*([\w-]+))?\s*\)', token, re.I)
        if counter:
            style = _counter_style_rust(counter.group(2) or 'decimal')
            if not style:
                return None
            output.append(
                'GeneratedContentItem::Counter { '
                f'name: "{_rust_escape_string(counter.group(1))}".to_string(), style: {style} }}'
            )
            continue
        counters = re.fullmatch(
            r'counters\(\s*([\w-]+)\s*,\s*((?:"(?:\\.|[^"\\])*")|(?:\'(?:\\.|[^\'\\])*\'))'
            r'(?:\s*,\s*([\w-]+))?\s*\)', token, re.I,
        )
        if counters:
            separator = _decode_css_string(counters.group(2))
            style = _counter_style_rust(counters.group(3) or 'decimal')
            if separator is None or not style:
                return None
            output.append(
                'GeneratedContentItem::Counters { '
                f'name: "{_rust_escape_string(counters.group(1))}".to_string(), '
                f'separator: "{_rust_escape_string(separator)}".to_string(), style: {style} }}'
            )
            continue
        # Replaced generated content remains image-batch-owned.
        return None
    return output


def parse_counter_operations(value: str, default_value: int) -> list[str] | None:
    tokens = value.split()
    if not tokens or value.strip().lower() == 'none':
        return []
    output = []
    i = 0
    while i < len(tokens):
        name = tokens[i]
        if not re.fullmatch(r'(?!none$)[_a-zA-Z][\w-]*', name, re.I):
            return None
        i += 1
        number = default_value
        if i < len(tokens) and re.fullmatch(r'[+-]?\d+', tokens[i]):
            number = int(tokens[i])
            i += 1
        output.append(
            'CounterOperation { '
            f'name: "{_rust_escape_string(name)}".to_string(), value: {number} }}'
        )
    return output


def parse_quotes(value: str) -> list[str] | None:
    if value.strip().lower() in ('auto', 'none'):
        return []
    tokens = _css_value_tokens(value)
    if not tokens or len(tokens) % 2:
        return None
    decoded = [_decode_css_string(token) for token in tokens]
    if any(item is None for item in decoded):
        return None
    return [
        'QuotePair { '
        f'open: "{_rust_escape_string(decoded[i])}".to_string(), '
        f'close: "{_rust_escape_string(decoded[i + 1])}".to_string() }}'
        for i in range(0, len(decoded), 2)
    ]


def parse_block_ellipsis(value: str) -> str | None:
    lower = value.strip().lower()
    if lower == 'auto':
        return 'BlockEllipsis::Auto'
    if lower == 'no-ellipsis':
        return 'BlockEllipsis::NoEllipsis'
    decoded = _decode_css_string(value)
    if decoded is not None:
        return f'BlockEllipsis::String("{_rust_escape_string(decoded)}".to_string())'
    return None


def generate_line_clamp_style(value: str, s: str, *, legacy: bool) -> list[str] | None:
    tokens = _css_value_tokens(value)
    if not tokens:
        return None
    first = tokens[0].lower()
    if first == 'none':
        clamp = 'LineClamp::None'
    elif first == 'auto' and not legacy:
        clamp = 'LineClamp::Auto'
    elif first.isdigit() and int(first) > 0:
        clamp = f'LineClamp::Lines({int(first)})'
    else:
        return None
    lines = [f'{s}.line_clamp = {clamp};']
    if legacy:
        lines.append(f'{s}.legacy_webkit_line_clamp = true;')
    if len(tokens) > 1:
        ellipsis = parse_block_ellipsis(' '.join(tokens[1:]))
        if ellipsis is None:
            return None
        lines.append(f'{s}.block_ellipsis = {ellipsis};')
    return lines


def generate_text_shadow_style(value: str, s: str, font_size: float) -> str | None:
    if value.strip().lower() == 'none':
        return f'{s}.text_shadow.clear();'
    layers = _split_css_layers(value)
    shadows = []
    for layer in layers:
        tokens = _split_respecting_parens(layer)
        lengths = []
        color = None
        uses_current_color = False
        for token in tokens:
            parsed_color = parse_color(token)
            if parsed_color:
                color = parsed_color
                continue
            if token.strip().lower() == 'currentcolor':
                uses_current_color = True
                continue
            length = _css_length_px(token, font_size)
            if length is None:
                return None
            lengths.append(float(length))
        if len(lengths) not in (2, 3) or lengths[2:] and lengths[2] < 0:
            return None
        if color is None:
            uses_current_color = True
        color_expr = 'current_color' if uses_current_color else color
        shadows.append(
            'TextShadow { '
            f'offset_x: {lengths[0]}, offset_y: {lengths[1]}, '
            f'blur_radius: {lengths[2] if len(lengths) == 3 else 0.0}, '
            f'color: {color_expr} }}'
        )
    if not shadows:
        return None
    return (
        f'{{ let current_color = {s}.color; '
        f'{s}.text_shadow = vec![{", ".join(shadows)}]; }}'
    )


def generate_single_style(
    prop: str,
    val: str,
    s: str,
    font_size: float = 16.0,
    radius_basis: tuple[float, float] | None = None,
) -> list[str] | str | None:
    """Generate Rust code for a single CSS property:value."""
    val = val.strip().rstrip(';').strip()
    if prop == 'grid-gap':
        prop = 'gap'

    # ── display ──
    if prop == 'display':
        mapping = {
            'block': 'Display::Block',
            'inline': 'Display::Inline',
            'inline-block': 'Display::InlineBlock',
            'none': 'Display::None',
            'flow-root': 'Display::FlowRoot',
            'contents': 'Display::Contents',
            'list-item': 'Display::ListItem',
            'flex': 'Display::Flex',
            'inline-flex': 'Display::InlineFlex',
            'grid': 'Display::Grid',
            'inline-grid': 'Display::InlineGrid',
            'table': 'Display::Table',
            'inline-table': 'Display::InlineTable',
            'table-row-group': 'Display::TableRowGroup',
            'table-header-group': 'Display::TableHeaderGroup',
            'table-footer-group': 'Display::TableFooterGroup',
            'table-row': 'Display::TableRow',
            'table-cell': 'Display::TableCell',
            'table-column-group': 'Display::TableColumnGroup',
            'table-column': 'Display::TableColumn',
            'table-caption': 'Display::TableCaption',
            # The layout core does not expose a MathML formatting context.
            # For generated textual pseudos, preserve the CSS outer display:
            # `math` is inline-level and `block math` is block-level.
            'math': 'Display::Inline',
            'block math': 'Display::Block',
            'math block': 'Display::Block',
        }
        if val == '-webkit-box':
            return [
                f"{s}.display = Display::FlowRoot;",
                f"{s}.legacy_webkit_box = true;",
            ]
        if val == '-webkit-inline-box':
            return [
                f"{s}.display = Display::InlineBlock;",
                f"{s}.legacy_webkit_box = true;",
            ]
        if val in {'flow-root list-item', 'list-item flow-root'}:
            return [
                f"{s}.display = Display::ListItem;",
                f"{s}.list_item_is_flow_root = true;",
            ]
        if val in mapping:
            return f"{s}.display = {mapping[val]};"

    # ── table model ──
    if prop == 'table-layout':
        mapping = {'auto': 'TableLayout::Auto', 'fixed': 'TableLayout::Fixed'}
        if val in mapping:
            return f"{s}.table_layout = {mapping[val]};"

    if prop == 'border-collapse':
        mapping = {
            'separate': 'BorderCollapse::Separate',
            'collapse': 'BorderCollapse::Collapse',
        }
        if val in mapping:
            return f"{s}.border_collapse = {mapping[val]};"

    if prop == 'border-spacing':
        parts = _grid_tokens(val)
        if parts and len(parts) in (1, 2):
            horizontal = parse_length(parts[0], font_size)
            vertical = parse_length(parts[-1], font_size)
            if horizontal and vertical:
                return f"{s}.border_spacing = ({horizontal}, {vertical});"

    if prop == 'caption-side':
        mapping = {'top': 'CaptionSide::Top', 'bottom': 'CaptionSide::Bottom'}
        if val in mapping:
            return f"{s}.caption_side = {mapping[val]};"

    if prop == 'empty-cells':
        mapping = {'show': 'EmptyCells::Show', 'hide': 'EmptyCells::Hide'}
        if val in mapping:
            return f"{s}.empty_cells = {mapping[val]};"

    # ── Grid model ──
    if prop in ('grid-template-columns', 'grid-template-rows'):
        tracks = parse_grid_track_list(val, font_size)
        if tracks:
            field = prop.replace('-', '_')
            return f"{s}.{field} = {tracks};"

    if prop in ('grid-auto-columns', 'grid-auto-rows'):
        tokens = _grid_tokens(val)
        tracks = [
            _grid_track_size_rust(token, font_size) for token in (tokens or [])
        ]
        if tracks and all(tracks):
            field = prop.replace('-', '_')
            return f"{s}.{field} = vec![{', '.join(tracks)}];"

    if prop == 'grid-auto-flow':
        tokens = set(val.lower().split())
        if tokens <= {'row', 'column', 'dense'} and not ({'row', 'column'} <= tokens):
            direction = (
                'GridAutoFlowDirection::Column'
                if 'column' in tokens else 'GridAutoFlowDirection::Row'
            )
            dense = 'true' if 'dense' in tokens else 'false'
            return (
                f"{s}.grid_auto_flow = GridAutoFlow {{ "
                f"direction: {direction}, dense: {dense} }};"
            )

    if prop in ('grid-column', 'grid-row'):
        placement = parse_grid_placement(val)
        if placement:
            field = prop.replace('-', '_')
            return f"{s}.{field} = {placement};"

    if prop in ('grid-column-start', 'grid-column-end', 'grid-row-start', 'grid-row-end'):
        line = parse_grid_line(val)
        if line:
            axis, edge = prop.split('-')[1:]
            return f"{s}.grid_{axis}.{edge} = {line};"

    if prop == 'grid-area':
        parts = _split_top_level_slash(val)
        if parts and 1 <= len(parts) <= 4:
            lines = [parse_grid_line(part) for part in parts]
            if all(lines):
                row_start = lines[0]
                # A single custom identifier names a template area on both
                # axes. Numeric/special grid lines retain the shorthand's
                # `auto` default for the omitted column start.
                single_named_area = (
                    len(parts) == 1
                    and re.fullmatch(r'[-_a-zA-Z][-_a-zA-Z0-9]*', parts[0].strip())
                    and parts[0].strip().lower() not in {'auto', 'span'}
                )
                column_start = (
                    lines[0]
                    if single_named_area
                    else lines[1] if len(lines) > 1 else 'GridLine::Auto'
                )
                row_end = lines[2] if len(lines) > 2 else 'GridLine::Auto'
                column_end = lines[3] if len(lines) > 3 else 'GridLine::Auto'
                return [
                    f"{s}.grid_row = GridPlacement {{ start: {row_start}, end: {row_end} }};",
                    f"{s}.grid_column = GridPlacement {{ start: {column_start}, end: {column_end} }};",
                ]

    if prop == 'grid-template-areas':
        areas = parse_grid_template_areas(val)
        if areas:
            return f"{s}.grid_template_areas = {areas};"

    if prop in ('grid-template', 'grid'):
        parts = _split_top_level_slash(val)
        if parts and len(parts) == 2:
            rows = parse_grid_track_list(parts[0], font_size)
            columns = parse_grid_track_list(parts[1], font_size)
            if rows and columns:
                return [
                    f"{s}.grid_template_rows = {rows};",
                    f"{s}.grid_template_columns = {columns};",
                ]

    if prop in ('justify-items', 'justify-self'):
        alignment = _item_alignment_rust(val, self_value=prop.endswith('self'))
        if alignment:
            field = prop.replace('-', '_')
            return f"{s}.{field} = {alignment};"

    if prop in ('place-items', 'place-self'):
        values = _grid_tokens(val)
        if values and len(values) in (1, 2):
            align = _item_alignment_rust(
                values[0], self_value=prop.endswith('self')
            )
            justify = _item_alignment_rust(
                values[-1], self_value=prop.endswith('self')
            )
            if align and justify:
                align_field = 'align_self' if prop.endswith('self') else 'align_items'
                justify_field = 'justify_self' if prop.endswith('self') else 'justify_items'
                return [
                    f"{s}.{align_field} = {align};",
                    f"{s}.{justify_field} = {justify};",
                ]

    if prop == 'place-content':
        values = _grid_tokens(val)
        if values and len(values) in (1, 2):
            align = _content_alignment_rust(values[0])
            justify = _content_alignment_rust(values[-1])
            if align and justify:
                return [
                    f"{s}.align_content = {align};",
                    f"{s}.justify_content = {justify};",
                ]

    if prop == 'margin-trim':
        tokens = val.lower().split()
        if tokens == ['none']:
            return f"{s}.margin_trim = MarginTrim::NONE;"
        mapping = {
            'block': 'MarginTrim::BLOCK',
            'block-start': 'MarginTrim::BLOCK_START',
            'block-end': 'MarginTrim::BLOCK_END',
            'inline': 'MarginTrim::INLINE',
            'inline-start': 'MarginTrim::INLINE_START',
            'inline-end': 'MarginTrim::INLINE_END',
        }
        if tokens and all(token in mapping for token in tokens):
            return f"{s}.margin_trim = {' | '.join(mapping[token] for token in tokens)};"

    # ── containment ──
    if prop == 'contain':
        tokens = val.lower().split()
        if tokens == ['none']:
            return f"{s}.contain = Containment::NONE;"
        if tokens == ['strict']:
            return f"{s}.contain = Containment::STRICT;"
        if tokens == ['content']:
            return f"{s}.contain = Containment::CONTENT;"
        mapping = {
            'size': 'Containment::SIZE',
            'inline-size': 'Containment::INLINE_SIZE',
            'layout': 'Containment::LAYOUT',
            'style': 'Containment::STYLE',
            'paint': 'Containment::PAINT',
        }
        if tokens and all(token in mapping for token in tokens):
            return f"{s}.contain = {' | '.join(mapping[token] for token in tokens)};"

    if prop == 'content-visibility':
        mapping = {
            'visible': 'ContentVisibility::Visible',
            'auto': 'ContentVisibility::Auto',
            'hidden': 'ContentVisibility::Hidden',
        }
        if val in mapping:
            return f"{s}.content_visibility = {mapping[val]};"

    if prop in ('contain-intrinsic-width', 'contain-intrinsic-height'):
        intrinsic = _contain_intrinsic_length_rust(val, font_size)
        if intrinsic:
            field = prop.replace('-', '_')
            return f"{s}.{field} = {intrinsic};"

    if prop == 'contain-intrinsic-size':
        intrinsic = _contain_intrinsic_length_rust(val, font_size)
        if intrinsic:
            return [
                f"{s}.contain_intrinsic_width = {intrinsic};",
                f"{s}.contain_intrinsic_height = {intrinsic};",
            ]
        values = _grid_tokens(val)
        auto = bool(values and values[0] == 'auto')
        if auto:
            values = values[1:]
        if values and len(values) in (1, 2):
            width_length = parse_length(values[0], font_size)
            height_length = parse_length(values[-1], font_size)
            constructor = (
                'ContainIntrinsicLength::auto_length'
                if auto else 'ContainIntrinsicLength::length'
            )
            width = f'{constructor}({width_length})' if width_length else None
            height = f'{constructor}({height_length})' if height_length else None
            if width and height:
                return [
                    f"{s}.contain_intrinsic_width = {width};",
                    f"{s}.contain_intrinsic_height = {height};",
                ]

    if prop == 'container-type':
        mapping = {
            'normal': 'ContainerType::Normal',
            'inline-size': 'ContainerType::InlineSize',
            'size': 'ContainerType::Size',
        }
        if val in mapping:
            return f"{s}.container_type = {mapping[val]};"

    if prop == 'container-name':
        if val == 'none':
            return f"{s}.container_names = Vec::new();"
        names = val.split()
        if names and all(re.fullmatch(r'[-_a-zA-Z][-_a-zA-Z0-9]*', name) for name in names):
            rust_names = ', '.join(
                f'"{_rust_escape_string(name)}".to_string()' for name in names
            )
            return f"{s}.container_names = vec![{rust_names}];"

    if prop == 'container':
        parts = _split_top_level_slash(val)
        if parts and len(parts) in (1, 2):
            result = []
            names = parts[0].split()
            if names == ['none']:
                result.append(f"{s}.container_names = Vec::new();")
            elif names and all(re.fullmatch(r'[-_a-zA-Z][-_a-zA-Z0-9]*', name) for name in names):
                rust_names = ', '.join(
                    f'"{_rust_escape_string(name)}".to_string()' for name in names
                )
                result.append(f"{s}.container_names = vec![{rust_names}];")
            else:
                return None
            if len(parts) == 2:
                type_code = generate_single_style(
                    'container-type', parts[1], s, font_size, radius_basis
                )
                if not type_code:
                    return None
                result.extend(type_code if isinstance(type_code, list) else [type_code])
            return result

    if prop == 'scroll-marker-group':
        mapping = {
            'none': 'ScrollMarkerGroup::None',
            'before': 'ScrollMarkerGroup::Before',
            'after': 'ScrollMarkerGroup::After',
        }
        if val in mapping:
            return f"{s}.scroll_marker_group = {mapping[val]};"

    if prop == 'scroll-target-group':
        mapping = {
            'none': 'ScrollTargetGroup::None',
            'auto': 'ScrollTargetGroup::Auto',
        }
        if val in mapping:
            return f"{s}.scroll_target_group = {mapping[val]};"

    if prop == 'scroll-snap-align':
        tokens = val.split()
        if len(tokens) in (1, 2):
            # SP19 stores the cohort's shared/inline alignment point.  All
            # selected rows use identical one-value axis alignment.
            mapping = {
                'none': 'ScrollSnapAlign::None',
                'start': 'ScrollSnapAlign::Start',
                'center': 'ScrollSnapAlign::Center',
                'end': 'ScrollSnapAlign::End',
            }
            if tokens[0] in mapping and (len(tokens) == 1 or tokens[1] == tokens[0]):
                return f"{s}.scroll_snap_align = {mapping[tokens[0]]};"

    if prop == 'scroll-snap-type':
        tokens = val.split()
        if tokens:
            mapping = {
                'none': 'ScrollSnapAxis::None',
                'x': 'ScrollSnapAxis::X',
                'y': 'ScrollSnapAxis::Y',
                'both': 'ScrollSnapAxis::Both',
                'inline': 'ScrollSnapAxis::Inline',
                'block': 'ScrollSnapAxis::Block',
            }
            if tokens[0] in mapping:
                return f"{s}.scroll_snap_axis = {mapping[tokens[0]]};"

    # ── replaced content and deterministic effects ──
    if prop == 'object-fit':
        mapping = {
            'fill': 'ObjectFit::Fill',
            'contain': 'ObjectFit::Contain',
            'cover': 'ObjectFit::Cover',
            'none': 'ObjectFit::None',
            'scale-down': 'ObjectFit::ScaleDown',
        }
        if val in mapping:
            return f"{s}.object_fit = {mapping[val]};"

    if prop == 'object-position':
        position = _background_position_rust(val)
        if position:
            return (
                f"{s}.object_position = ObjectPosition {{ x: {position[0]}, "
                f"y: {position[1]} }};"
            )

    if prop == 'transform':
        transform = _transform_2d_rust(val, font_size, radius_basis)
        if transform:
            return [
                f"{s}.transform = {transform};",
                f"{s}.establishes_transform_containing_block = true;",
            ]

    if prop == 'transform-origin':
        values = val.split()
        if len(values) in (1, 2):
            horizontal = {'left': '0%', 'center': '50%', 'right': '100%'}
            vertical = {'top': '0%', 'center': '50%', 'bottom': '100%'}
            if len(values) == 1:
                token = values[0]
                if token in ('top', 'bottom'):
                    x_token, y_token = '50%', vertical[token]
                else:
                    x_token, y_token = horizontal.get(token, token), '50%'
            else:
                first, second = values
                if first in ('top', 'bottom') or second in ('left', 'right'):
                    first, second = second, first
                x_token = horizontal.get(first, first)
                y_token = vertical.get(second, second)
            x = parse_length(x_token, font_size)
            y = parse_length(y_token, font_size)
            if x and y:
                return f"{s}.transform_origin = ({x}, {y});"

    if prop == 'translate':
        values = val.split()
        if len(values) in (1, 2):
            x = _css_length_px(values[0], font_size)
            y = _css_length_px(values[-1], font_size) if len(values) == 2 else 0.0
            if x is not None and y is not None:
                return [
                    f"{s}.transform = Transform2D {{ a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: {x}, f: {y} }};",
                    f"{s}.establishes_transform_containing_block = true;",
                ]

    if prop == 'clip-path':
        if val == 'none':
            return f"{s}.clip_path_inset = None;"
        match = re.fullmatch(r'inset\(\s*(.*?)\s*\)', val, re.IGNORECASE)
        if match:
            tokens = _split_respecting_parens(match.group(1))
            if 'round' in [token.lower() for token in tokens]:
                tokens = tokens[:[token.lower() for token in tokens].index('round')]
            if 1 <= len(tokens) <= 4:
                expanded = (
                    [tokens[0]] * 4 if len(tokens) == 1 else
                    [tokens[0], tokens[1], tokens[0], tokens[1]] if len(tokens) == 2 else
                    [tokens[0], tokens[1], tokens[2], tokens[1]] if len(tokens) == 3 else
                    tokens
                )
                lengths = [parse_length(token, font_size) for token in expanded]
                if all(lengths):
                    return f"{s}.clip_path_inset = Some([{', '.join(lengths)}]);"

    if prop == 'shape-outside':
        shape = _shape_outside_rust(val, font_size)
        if shape:
            return f"{s}.shape_outside = {shape};"

    if prop == 'shape-margin':
        length = parse_length(val, font_size)
        if length:
            return f"{s}.shape_margin = {length};"

    if prop == 'shape-image-threshold':
        try:
            threshold = min(1.0, max(0.0, float(val)))
            return f"{s}.shape_image_threshold = {threshold};"
        except ValueError:
            pass

    if prop == 'list-style-position':
        mapping = {
            'outside': 'ListStylePosition::Outside',
            'inside': 'ListStylePosition::Inside',
        }
        if val in mapping:
            return f"{s}.list_style_position = {mapping[val]};"

    # ── position ──
    if prop == 'anchor-name':
        if val == 'none':
            return f"{s}.anchor_name = None;"
        names = [token for token in val.split() if token.startswith('--')]
        if names:
            return f'{s}.anchor_name = Some("{_rust_escape_string(names[0])}".to_string());'

    if prop == 'position-anchor':
        if val in {'auto', 'none'}:
            return f"{s}.position_anchor = None;"
        if val.startswith('--') and len(val.split()) == 1:
            return f'{s}.position_anchor = Some("{_rust_escape_string(val)}".to_string());'

    if prop == 'position-area':
        normalized = ' '.join(val.lower().split())
        mapping = {
            'top left': 'TopLeft',
            'top center': 'TopCenter',
            'top right': 'TopRight',
            'left center': 'LeftCenter',
            'center': 'Center',
            'right center': 'RightCenter',
            'bottom left': 'BottomLeft',
            'bottom center': 'BottomCenter',
            'bottom right': 'BottomRight',
        }
        if normalized in mapping:
            return f"{s}.position_area = PositionArea::{mapping[normalized]};"

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

    if prop == 'will-change':
        if any(part.strip() == 'transform' for part in val.split(',')):
            return (
                f"{s}.establishes_transform_containing_block = true;\n"
                f"{s}.will_change_transform = true;"
            )

    if prop == 'filter' and val != 'none':
        emitted = [f"{s}.establishes_transform_containing_block = true;"]
        blur = re.search(r'blur\(\s*([^)]*?)\s*\)', val, re.IGNORECASE)
        if blur:
            blur_px = _css_length_px(blur.group(1), font_size)
            if blur_px is not None:
                emitted.append(f"{s}.filter_blur = {max(0.0, blur_px)};")
        grayscale = re.search(r'grayscale\(\s*([^)]*?)\s*\)', val, re.IGNORECASE)
        if grayscale:
            token = grayscale.group(1).strip()
            try:
                amount = (
                    float(token[:-1]) / 100.0
                    if token.endswith('%') else float(token)
                )
            except ValueError:
                amount = None
            if amount is not None:
                emitted.append(
                    f"{s}.filter_grayscale = {min(1.0, max(0.0, amount))};"
                )
        return emitted

    if prop.startswith('animation'):
        # SP19 comparisons freeze the document timeline at 0 ms. The layout
        # contract retains the sampled state even when the relevant keyframe
        # does not alter a property represented by this compact renderer.
        return (
            f"{s}.animation_snapshot = Some(AnimationSnapshot {{ "
            "document_time_ms: 0.0, progress: 0.0 });"
        )

    # ── generated content / counters ──
    if prop == 'content':
        items = parse_generated_content(val)
        if items is not None:
            return f"{s}.content = Some(vec![{', '.join(items)}]);"

    if prop in ('counter-reset', 'counter-set', 'counter-increment'):
        default_value = 1 if prop == 'counter-increment' else 0
        operations = parse_counter_operations(val, default_value)
        if operations is not None:
            field = prop.replace('-', '_')
            return f"{s}.{field} = vec![{', '.join(operations)}];"

    if prop == 'quotes':
        pairs = parse_quotes(val)
        if pairs is not None:
            return f"{s}.quotes = vec![{', '.join(pairs)}];"

    # ── line clamp compatibility ──
    if prop == 'line-clamp' and not MODERN_LINE_CLAMP_ENABLED:
        return None
    if prop in ('line-clamp', '-webkit-line-clamp'):
        return generate_line_clamp_style(val, s, legacy=prop.startswith('-webkit-'))

    if prop == 'block-ellipsis':
        ellipsis = parse_block_ellipsis(val)
        if ellipsis:
            return f"{s}.block_ellipsis = {ellipsis};"

    if prop == '-webkit-box-orient':
        mapping = {
            'horizontal': ('WebkitBoxOrient::Horizontal', 'FlexDirection::Row'),
            'vertical': ('WebkitBoxOrient::Vertical', 'FlexDirection::Column'),
            'inline-axis': ('WebkitBoxOrient::Horizontal', 'FlexDirection::Row'),
            'block-axis': ('WebkitBoxOrient::Vertical', 'FlexDirection::Column'),
        }
        if val.lower() in mapping:
            orient, direction = mapping[val.lower()]
            return [
                f"{s}.webkit_box_orient = {orient};",
                f"{s}.flex_direction = {direction};",
            ]

    if prop == 'text-shadow':
        return generate_text_shadow_style(val, s, font_size)

    # ── float ──
    if prop == 'float':
        mapping = {
            'left': 'Float::Left',
            'right': 'Float::Right',
            # Logical floats resolve against the inherited default horizontal
            # LTR direction used by the deterministic WPT profile.
            'inline-start': 'Float::Left',
            'inline-end': 'Float::Right',
            'none': 'Float::None',
        }
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
        length = parse_length(val, font_size)
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
        length = parse_length(val, font_size)
        if length:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {length};"

    # ── margin shorthand ──
    if prop == 'margin':
        return generate_shorthand_4(val, s, 'margin', font_size=font_size)

    # ── margin sides ──
    if prop in ('margin-top', 'margin-right', 'margin-bottom', 'margin-left'):
        length = parse_length(val, font_size)
        if length:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {length};"

    # ── padding shorthand ──
    if prop == 'padding':
        return generate_shorthand_4(val, s, 'padding', font_size=font_size)

    # ── padding sides ──
    if prop in ('padding-top', 'padding-right', 'padding-bottom', 'padding-left'):
        length = parse_length(val, font_size)
        if length:
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {length};"

    # ── border shorthand (e.g. "1px solid red") ──
    if prop == 'border':
        return generate_border_shorthand(
            val, s, ['top', 'right', 'bottom', 'left'], font_size
        )

    if prop in ('border-top', 'border-right', 'border-bottom', 'border-left'):
        side = prop.split('-')[1]
        return generate_border_shorthand(val, s, [side], font_size)

    # ── border-width shorthand ──
    if prop == 'border-width':
        return generate_border_width_shorthand(val, s, font_size)

    # ── border-width sides ──
    if prop in ('border-top-width', 'border-right-width', 'border-bottom-width', 'border-left-width'):
        px_val = parse_border_width(val, font_size)
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
    # Unit conversion factors to px
    _UNIT_TO_PX = {
        'px': 1.0, 'em': None, 'rem': None, '%': None,
        'in': 96.0, 'cm': 96.0 / 2.54, 'mm': 96.0 / 25.4,
        'pt': 96.0 / 72.0, 'pc': 96.0 / 6.0,
    }

    def _parse_radius_component(token, axis: str, fs=font_size):
        if token == '0':
            return 0.0
        m = re.match(r'^(-?[\d.]+)(px|em|rem|%|in|cm|mm|pt|pc)$', token)
        if not m:
            return None
        num = float(m.group(1))
        # CSS Backgrounds §3: negative corner radii invalidate the complete
        # declaration; they are not clamped independently to zero.
        if num < 0.0:
            return None
        unit = m.group(2)
        if unit in ('em', 'rem'):
            num = num * fs
        elif unit == '%':
            # Keep percentages unresolved until paint, when the used border
            # box is available (notably for auto-height boxes).
            pass
        elif unit in _UNIT_TO_PX and _UNIT_TO_PX[unit] is not None:
            num = num * _UNIT_TO_PX[unit]
        return num

    def _expand_shorthand(values):
        """CSS shorthand expansion: 1→all, 2→TL/BR TR/BL, 3→TL TR/BL BR, 4→TL TR BR BL"""
        if len(values) == 1:
            return values[0], values[0], values[0], values[0]
        elif len(values) == 2:
            return values[0], values[1], values[0], values[1]
        elif len(values) == 3:
            return values[0], values[1], values[2], values[1]
        elif len(values) == 4:
            return values[0], values[1], values[2], values[3]
        return None, None, None, None

    if prop == 'border-radius':
        slash_parts = val.strip().split('/')
        if len(slash_parts) > 2:
            return None
        horiz_tokens = slash_parts[0].strip().split()
        h_vals = [_parse_radius_component(p, 'x') for p in horiz_tokens]
        if all(v is not None for v in h_vals):
            htl, htr, hbr, hbl = _expand_shorthand(h_vals)
            if htl is not None:
                # Parse vertical radii (after slash) if present
                if len(slash_parts) > 1:
                    vert_tokens = slash_parts[1].strip().split()
                    v_vals = [_parse_radius_component(p, 'y') for p in vert_tokens]
                    if all(v is not None for v in v_vals):
                        vtl, vtr, vbr, vbl = _expand_shorthand(v_vals)
                    else:
                        return None
                else:
                    v_vals = [_parse_radius_component(p, 'y') for p in horiz_tokens]
                    if all(v is not None for v in v_vals):
                        vtl, vtr, vbr, vbl = _expand_shorthand(v_vals)
                    else:
                        vtl, vtr, vbr, vbl = htl, htr, hbr, hbl
                ht = _expand_shorthand(horiz_tokens)
                vt = _expand_shorthand(
                    slash_parts[1].strip().split()
                    if len(slash_parts) > 1 else horiz_tokens
                )
                percentage_flags = [
                    (ht[index].strip().endswith('%'), vt[index].strip().endswith('%'))
                    for index in range(4)
                ]
                percentage_flags_rust = '[' + ', '.join(
                    f"({str(x).lower()}, {str(y).lower()})"
                    for x, y in percentage_flags
                ) + ']'
                return [
                    f"{s}.border_top_left_radius = ({htl}_f32, {vtl}_f32);",
                    f"{s}.border_top_right_radius = ({htr}_f32, {vtr}_f32);",
                    f"{s}.border_bottom_right_radius = ({hbr}_f32, {vbr}_f32);",
                    f"{s}.border_bottom_left_radius = ({hbl}_f32, {vbl}_f32);",
                    f"{s}.border_radius_percent = {percentage_flags_rust};",
                ]

    if prop in ('border-top-left-radius', 'border-top-right-radius',
                'border-bottom-left-radius', 'border-bottom-right-radius'):
        parts = val.strip().split()
        if len(parts) == 1:
            vx = _parse_radius_component(parts[0], 'x')
            vy = _parse_radius_component(parts[0], 'y')
            if vx is not None and vy is not None:
                rust_prop = prop.replace('-', '_')
                corner = {
                    'border-top-left-radius': 0,
                    'border-top-right-radius': 1,
                    'border-bottom-right-radius': 2,
                    'border-bottom-left-radius': 3,
                }[prop]
                flag = parts[0].endswith('%')
                return [
                    f"{s}.{rust_prop} = ({vx}_f32, {vy}_f32);",
                    f"{s}.border_radius_percent[{corner}] = ({str(flag).lower()}, {str(flag).lower()});",
                ]
        elif len(parts) == 2:
            vx = _parse_radius_component(parts[0], 'x')
            vy = _parse_radius_component(parts[1], 'y')
            if vx is not None and vy is not None:
                rust_prop = prop.replace('-', '_')
                corner = {
                    'border-top-left-radius': 0,
                    'border-top-right-radius': 1,
                    'border-bottom-right-radius': 2,
                    'border-bottom-left-radius': 3,
                }[prop]
                return [
                    f"{s}.{rust_prop} = ({vx}_f32, {vy}_f32);",
                    f"{s}.border_radius_percent[{corner}] = ({str(parts[0].endswith('%')).lower()}, {str(parts[1].endswith('%')).lower()});",
                ]

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
        # Two-value form: overflow: <x> <y>
        parts = val.split()
        if len(parts) == 2 and parts[0] in mapping and parts[1] in mapping:
            return [
                f"{s}.overflow_x = {mapping[parts[0]]};",
                f"{s}.overflow_y = {mapping[parts[1]]};",
            ]
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

    if prop == 'resize':
        mapping = {
            'none': 'Resize::None',
            'both': 'Resize::Both',
            'horizontal': 'Resize::Horizontal',
            'vertical': 'Resize::Vertical',
            'block': 'Resize::Block',
            'inline': 'Resize::Inline',
        }
        if val in mapping:
            return f"{s}.resize = {mapping[val]};"

    if prop == 'scrollbar-width':
        mapping = {
            'auto': 'ScrollbarWidth::Auto',
            'thin': 'ScrollbarWidth::Thin',
            'none': 'ScrollbarWidth::None',
        }
        if val in mapping:
            return f"{s}.scrollbar_width = {mapping[val]};"

    if prop == 'scrollbar-gutter':
        normalized = ' '.join(val.split())
        mapping = {
            'auto': 'ScrollbarGutter::Auto',
            'stable': 'ScrollbarGutter::Stable',
            'stable both-edges': 'ScrollbarGutter::StableBothEdges',
        }
        if normalized in mapping:
            return f"{s}.scrollbar_gutter = {mapping[normalized]};"

    # ── overflow-clip-margin ──
    if prop == 'overflow-clip-margin':
        # CSS Overflow 3: overflow-clip-margin: <visual-box>? <length>
        # <visual-box> = content-box | padding-box | border-box
        parts = val.split()
        box_val = None
        length_val = None
        for p in parts:
            if p in ('content-box', 'padding-box', 'border-box'):
                box_val = p
            else:
                length_val = _css_length_px(p, font_size)
        lines = []
        if length_val is not None:
            lines.append(f"{s}.overflow_clip_margin = {length_val};")
        if box_val:
            box_map = {
                'content-box': 'OverflowClipBox::ContentBox',
                'padding-box': 'OverflowClipBox::PaddingBox',
                'border-box': 'OverflowClipBox::BorderBox',
            }
            lines.append(f"{s}.overflow_clip_box = {box_map[box_val]};")
        if lines:
            return '\n'.join(lines)

    # ── scrollbar-color ──
    if prop == 'scrollbar-color':
        parts = val.split()
        if len(parts) >= 2 and parts[0] != 'auto':
            thumb = parse_color(parts[0])
            track = parse_color(parts[1])
            if thumb and track:
                return [
                    f"{s}.scrollbar_thumb_color = Some({thumb});",
                    f"{s}.scrollbar_track_color = Some({track});",
                ]

    # ── background-color ──
    if prop == 'background-color':
        color = parse_color(val)
        if color:
            return f"{s}.background_color = {color};"

    # ── background-clip ──
    if prop == 'background-clip':
        mapping = {
            'border-box': 'BackgroundClip::BorderBox',
            'padding-box': 'BackgroundClip::PaddingBox',
            'content-box': 'BackgroundClip::ContentBox',
            'text': 'BackgroundClip::Text',
        }
        if val.strip() in mapping:
            return f"{s}.background_clip = {mapping[val.strip()]};"

    if prop == 'background-attachment':
        mapping = {
            'scroll': 'BackgroundAttachment::Scroll',
            'fixed': 'BackgroundAttachment::Fixed',
            'local': 'BackgroundAttachment::Local',
        }
        if val.strip() in mapping:
            return f"{s}.background_attachment = {mapping[val.strip()]};"

    if prop == 'background-image':
        if val.strip() == 'none':
            return f"{s}.background_linear_gradient = None;"
        gradient = _linear_gradient_rust(val)
        if gradient:
            return f"{s}.background_linear_gradient = {gradient};"

    # ── background shorthand — extract color component ──
    if prop == 'background':
        # background: none → treat as transparent
        if val.strip() == 'none' or val.strip() == 'none, none':
            return [
                f"{s}.background_color = Color::TRANSPARENT;",
                f"{s}.background_linear_gradient = None;",
            ]
        # Try parsing entire value as color first (simplest case)
        color = parse_color(val)
        if color:
            return [
                f"{s}.background_color = {color};",
                f"{s}.background_linear_gradient = None;",
            ]
        lines = []
        shorthand_layers = _split_css_layers(val)
        if shorthand_layers:
            color_layer = _parse_background_shorthand_layer(
                shorthand_layers[-1], font_size
            )
            if color_layer.get('clip'):
                lines.append(f"{s}.background_clip = {color_layer['clip']};")
        gradient = _linear_gradient_rust(val)
        gradient_function = _linear_gradient_function(val)
        if gradient:
            lines.append(f"{s}.background_linear_gradient = {gradient};")
        # Try extracting color from complex shorthand
        # background: <color> url(...) ... or <color> <other>
        extracted_image = _extract_css_image(val)
        remainder = (
            extracted_image[1]
            if extracted_image
            else (gradient_function[1] if gradient_function else val)
        )
        parts = remainder.split()
        for part in parts:
            part = part.strip()
            if part.startswith('url(') or part.startswith('no-repeat') or part.startswith('repeat'):
                continue
            if '/' in part or part in ('top', 'left', 'right', 'bottom', 'center',
                                        'cover', 'contain', 'fixed', 'scroll', 'local',
                                        'no-repeat', 'repeat-x', 'repeat-y', 'repeat',
                                        'padding-box', 'border-box', 'content-box',
                                        'none'):
                continue
            color = parse_color(part)
            if color:
                lines.append(f"{s}.background_color = {color};")
                break
        if lines:
            return lines

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

    # ── box-shadow ──
    if prop == 'box-shadow':
        if val == 'none':
            return f"{s}.box_shadow = Vec::new();"
        # Split by comma for multiple shadows, but respect parentheses (rgb/rgba)
        shadows_raw = []
        depth = 0
        current = []
        for ch in val:
            if ch == '(':
                depth += 1
                current.append(ch)
            elif ch == ')':
                depth -= 1
                current.append(ch)
            elif ch == ',' and depth == 0:
                shadows_raw.append(''.join(current).strip())
                current = []
            else:
                current.append(ch)
        if current:
            shadows_raw.append(''.join(current).strip())

        shadow_strs = []
        for shadow in shadows_raw:
            tokens = _split_respecting_parens(shadow)
            inset = False
            clean_tokens = []
            for t in tokens:
                if t == 'inset':
                    inset = True
                else:
                    clean_tokens.append(t)
            tokens = clean_tokens

            color_str = None
            numeric_tokens = []
            for token in tokens:
                color = parse_color(token)
                if color is not None and color_str is None:
                    color_str = color
                else:
                    numeric_tokens.append(token)
            if color_str is None:
                color_str = 'Color::BLACK'

            # Parse numeric values (offset-x, offset-y, blur, spread)
            vals = []
            valid_lengths = True
            for t in numeric_tokens:
                m_px = re.match(r'^(-?[\d.]+)px$', t)
                if m_px:
                    vals.append(float(m_px.group(1)))
                elif t == '0':
                    vals.append(0.0)
                else:
                    m_em = re.match(r'^(-?[\d.]+)em$', t)
                    if m_em:
                        vals.append(float(m_em.group(1)) * font_size)
                    else:
                        m_rem = re.match(r'^(-?[\d.]+)rem$', t)
                        if m_rem:
                            vals.append(float(m_rem.group(1)) * 16.0)
                        elif t.startswith('calc(') and t.endswith(')'):
                            total = 0.0
                            inner = t[5:-1].replace('-', '+-')
                            for term in inner.split('+'):
                                term = term.strip()
                                if not term:
                                    continue
                                match = re.fullmatch(r'(-?[\d.]+)(px|em|rem|pt)', term)
                                if not match:
                                    valid_lengths = False
                                    break
                                scale = {
                                    'px': 1.0, 'em': font_size,
                                    'rem': 16.0, 'pt': 4.0 / 3.0,
                                }[match.group(2)]
                                total += float(match.group(1)) * scale
                            vals.append(total)
                        else:
                            valid_lengths = False

            if valid_lengths and 2 <= len(vals) <= 4 and (len(vals) < 3 or vals[2] >= 0.0):
                ox, oy = vals[0], vals[1]
                blur = vals[2] if len(vals) > 2 else 0.0
                spread = vals[3] if len(vals) > 3 else 0.0
                inset_str = 'true' if inset else 'false'
                shadow_strs.append(
                    f'BoxShadow {{ offset_x: {ox:.1f}, offset_y: {oy:.1f}, '
                    f'blur_radius: {blur:.1f}, spread_radius: {spread:.1f}, '
                    f'color: {color_str}, inset: {inset_str} }}'
                )

        if shadow_strs:
            return f"{s}.box_shadow = vec![{', '.join(shadow_strs)}];"

    # ── visibility ──
    if prop == 'visibility':
        mapping = {'visible': 'Visibility::Visible', 'hidden': 'Visibility::Hidden', 'collapse': 'Visibility::Collapse'}
        if val in mapping:
            return f"{s}.visibility = {mapping[val]};"

    # ── direction ──
    if prop == 'direction':
        mapping = {'ltr': 'Direction::Ltr', 'rtl': 'Direction::Rtl'}
        if val in mapping:
            return f"{s}.direction = {mapping[val]};"

    if prop == 'writing-mode':
        mapping = {
            'horizontal-tb': 'WritingMode::HorizontalTb',
            'vertical-rl': 'WritingMode::VerticalRl',
            'vertical-lr': 'WritingMode::VerticalLr',
            'sideways-rl': 'WritingMode::SidewaysRl',
            'sideways-lr': 'WritingMode::SidewaysLr',
        }
        if val in mapping:
            return f"{s}.writing_mode = {mapping[val]};"

    if prop == 'unicode-bidi':
        mapping = {
            'normal': 'UnicodeBidi::Normal',
            'embed': 'UnicodeBidi::Embed',
            'bidi-override': 'UnicodeBidi::Override',
            'isolate': 'UnicodeBidi::Isolate',
            'isolate-override': 'UnicodeBidi::IsolateOverride',
            'plaintext': 'UnicodeBidi::Plaintext',
        }
        if val in mapping:
            return f"{s}.unicode_bidi = {mapping[val]};"

    if prop == 'text-orientation':
        mapping = {
            'mixed': 'TextOrientation::Mixed',
            'upright': 'TextOrientation::Upright',
            'sideways': 'TextOrientation::Sideways',
        }
        if val in mapping:
            return f"{s}.text_orientation = {mapping[val]};"

    if prop == 'text-combine-upright':
        mapping = {
            'none': 'TextCombineUpright::None',
            'all': 'TextCombineUpright::All',
        }
        if val in mapping:
            return f"{s}.text_combine_upright = {mapping[val]};"

    if prop == 'text-emphasis-style':
        token = val.strip()
        if token == 'none':
            return f"{s}.text_emphasis_mark = TextEmphasisMark::None;"
        if len(token) >= 2 and token[0] == token[-1] and token[0] in {'\"', "'"}:
            custom = token[1:-1]
            if len(custom) == 1:
                return (
                    f"{s}.text_emphasis_mark = TextEmphasisMark::Custom("
                    f"'\\u{{{ord(custom):x}}}');"
                )
        pieces = token.split()
        lines = []
        if 'open' in pieces:
            lines.append(f"{s}.text_emphasis_fill = TextEmphasisFill::Open;")
        elif 'filled' in pieces:
            lines.append(f"{s}.text_emphasis_fill = TextEmphasisFill::Filled;")
        mark_names = {
            'dot': 'Dot',
            'circle': 'Circle',
            'double-circle': 'DoubleCircle',
            'triangle': 'Triangle',
            'sesame': 'Sesame',
        }
        for piece in pieces:
            if piece in mark_names:
                lines.append(
                    f"{s}.text_emphasis_mark = TextEmphasisMark::{mark_names[piece]};"
                )
                break
        return lines or None

    if prop == 'text-emphasis-position':
        pieces = val.strip().split()
        if any(piece in {'over', 'under'} for piece in pieces):
            over = 'false' if 'under' in pieces else 'true'
            right = 'false' if 'left' in pieces else 'true'
            return (
                f"{s}.text_emphasis_position = TextEmphasisPosition {{ "
                f"over: {over}, right: {right} }};"
            )

    if prop == 'text-emphasis-color':
        color = parse_color(val)
        if color:
            return f"{s}.text_emphasis_color = StyleColor::Resolved({color});"

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

    if prop in ('justify-content', '-webkit-box-pack'):
        if prop == '-webkit-box-pack':
            val = {
                'start': 'flex-start',
                'end': 'flex-end',
                'justify': 'space-between',
            }.get(val, val)
        mapping = {
            'flex-start': 'ContentAlignment::new(ContentPosition::FlexStart)',
            'start': 'ContentAlignment::new(ContentPosition::Start)',
            'flex-end': 'ContentAlignment::new(ContentPosition::FlexEnd)',
            'end': 'ContentAlignment::new(ContentPosition::End)',
            'center': 'ContentAlignment::new(ContentPosition::Center)',
            'left': 'ContentAlignment::new(ContentPosition::Left)',
            'right': 'ContentAlignment::new(ContentPosition::Right)',
            'normal': 'ContentAlignment::new(ContentPosition::Normal)',
            'space-between': 'ContentAlignment::with_distribution(ContentDistribution::SpaceBetween)',
            'space-around': 'ContentAlignment::with_distribution(ContentDistribution::SpaceAround)',
            'space-evenly': 'ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly)',
            'stretch': 'ContentAlignment::with_distribution(ContentDistribution::Stretch)',
        }
        if val in mapping:
            return f"{s}.justify_content = {mapping[val]};"
        parts = val.split()
        if len(parts) == 2 and parts[0] in ('safe', 'unsafe'):
            overflow = 'OverflowAlignment::Safe' if parts[0] == 'safe' else 'OverflowAlignment::Unsafe'
            pos_map = {
                'flex-start': 'ContentPosition::FlexStart', 'start': 'ContentPosition::Start',
                'flex-end': 'ContentPosition::FlexEnd', 'end': 'ContentPosition::End',
                'center': 'ContentPosition::Center', 'left': 'ContentPosition::Left',
                'right': 'ContentPosition::Right',
            }
            if parts[1] in pos_map:
                return f"{s}.justify_content = ContentAlignment {{ position: {pos_map[parts[1]]}, distribution: ContentDistribution::Default, overflow: {overflow} }};"

    if prop in ('align-items', '-webkit-box-align'):
        if prop == '-webkit-box-align':
            val = {
                'start': 'flex-start',
                'end': 'flex-end',
            }.get(val, val)
        mapping = {
            'flex-start': 'ItemAlignment::new(ItemPosition::FlexStart)',
            'start': 'ItemAlignment::new(ItemPosition::Start)',
            'flex-end': 'ItemAlignment::new(ItemPosition::FlexEnd)',
            'end': 'ItemAlignment::new(ItemPosition::End)',
            'center': 'ItemAlignment::new(ItemPosition::Center)',
            'stretch': 'ItemAlignment::new(ItemPosition::Stretch)',
            'baseline': 'ItemAlignment::new(ItemPosition::Baseline)',
            'first baseline': 'ItemAlignment::new(ItemPosition::Baseline)',
            'last baseline': 'ItemAlignment::new(ItemPosition::LastBaseline)',
            'normal': 'ItemAlignment::new(ItemPosition::Normal)',
            'self-start': 'ItemAlignment::new(ItemPosition::SelfStart)',
            'self-end': 'ItemAlignment::new(ItemPosition::SelfEnd)',
            'left': 'ItemAlignment::new(ItemPosition::Left)',
            'right': 'ItemAlignment::new(ItemPosition::Right)',
        }
        if val in mapping:
            return f"{s}.align_items = {mapping[val]};"
        parts = val.split()
        if len(parts) == 2 and parts[0] in ('safe', 'unsafe'):
            overflow = 'OverflowAlignment::Safe' if parts[0] == 'safe' else 'OverflowAlignment::Unsafe'
            pos_map = {
                'flex-start': 'ItemPosition::FlexStart', 'start': 'ItemPosition::Start',
                'flex-end': 'ItemPosition::FlexEnd', 'end': 'ItemPosition::End',
                'center': 'ItemPosition::Center', 'stretch': 'ItemPosition::Stretch',
                'baseline': 'ItemPosition::Baseline', 'self-start': 'ItemPosition::SelfStart',
                'self-end': 'ItemPosition::SelfEnd',
            }
            if parts[1] in pos_map:
                return f"{s}.align_items = ItemAlignment::with_overflow({pos_map[parts[1]]}, {overflow});"

    if prop == 'align-self':
        mapping = {
            'auto': 'ItemAlignment::INITIAL_SELF',
            'flex-start': 'ItemAlignment::new(ItemPosition::FlexStart)',
            'start': 'ItemAlignment::new(ItemPosition::Start)',
            'flex-end': 'ItemAlignment::new(ItemPosition::FlexEnd)',
            'end': 'ItemAlignment::new(ItemPosition::End)',
            'center': 'ItemAlignment::new(ItemPosition::Center)',
            'stretch': 'ItemAlignment::new(ItemPosition::Stretch)',
            'baseline': 'ItemAlignment::new(ItemPosition::Baseline)',
            'first baseline': 'ItemAlignment::new(ItemPosition::Baseline)',
            'last baseline': 'ItemAlignment::new(ItemPosition::LastBaseline)',
            'normal': 'ItemAlignment::new(ItemPosition::Normal)',
            'self-start': 'ItemAlignment::new(ItemPosition::SelfStart)',
            'self-end': 'ItemAlignment::new(ItemPosition::SelfEnd)',
            'left': 'ItemAlignment::new(ItemPosition::Left)',
            'right': 'ItemAlignment::new(ItemPosition::Right)',
        }
        if val in mapping:
            return f"{s}.align_self = {mapping[val]};"
        parts = val.split()
        if len(parts) == 2 and parts[0] in ('safe', 'unsafe'):
            overflow = 'OverflowAlignment::Safe' if parts[0] == 'safe' else 'OverflowAlignment::Unsafe'
            pos_map = {
                'flex-start': 'ItemPosition::FlexStart', 'start': 'ItemPosition::Start',
                'flex-end': 'ItemPosition::FlexEnd', 'end': 'ItemPosition::End',
                'center': 'ItemPosition::Center', 'stretch': 'ItemPosition::Stretch',
                'baseline': 'ItemPosition::Baseline', 'self-start': 'ItemPosition::SelfStart',
                'self-end': 'ItemPosition::SelfEnd',
            }
            if parts[1] in pos_map:
                return f"{s}.align_self = ItemAlignment::with_overflow({pos_map[parts[1]]}, {overflow});"

    if prop == 'align-content':
        mapping = {
            'flex-start': 'ContentAlignment::new(ContentPosition::FlexStart)',
            'start': 'ContentAlignment::new(ContentPosition::Start)',
            'flex-end': 'ContentAlignment::new(ContentPosition::FlexEnd)',
            'end': 'ContentAlignment::new(ContentPosition::End)',
            'center': 'ContentAlignment::new(ContentPosition::Center)',
            'normal': 'ContentAlignment::new(ContentPosition::Normal)',
            'baseline': 'ContentAlignment::new(ContentPosition::Baseline)',
            'last baseline': 'ContentAlignment::new(ContentPosition::LastBaseline)',
            'stretch': 'ContentAlignment::with_distribution(ContentDistribution::Stretch)',
            'space-between': 'ContentAlignment::with_distribution(ContentDistribution::SpaceBetween)',
            'space-around': 'ContentAlignment::with_distribution(ContentDistribution::SpaceAround)',
            'space-evenly': 'ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly)',
        }
        if val in mapping:
            return f"{s}.align_content = {mapping[val]};"
        parts = val.split()
        if len(parts) == 2 and parts[0] in ('safe', 'unsafe'):
            overflow = 'OverflowAlignment::Safe' if parts[0] == 'safe' else 'OverflowAlignment::Unsafe'
            pos_map = {
                'flex-start': 'ContentPosition::FlexStart', 'start': 'ContentPosition::Start',
                'flex-end': 'ContentPosition::FlexEnd', 'end': 'ContentPosition::End',
                'center': 'ContentPosition::Center',
            }
            if parts[1] in pos_map:
                return f"{s}.align_content = ContentAlignment {{ position: {pos_map[parts[1]]}, distribution: ContentDistribution::Default, overflow: {overflow} }};"

    if prop in ('flex-grow', 'flex-shrink'):
        try:
            v = float(val)
            if v < 0.0:
                return None  # CSS spec: negative values make declaration invalid
            rust_prop = prop.replace('-', '_')
            return f"{s}.{rust_prop} = {v};"
        except ValueError:
            pass

    if prop == 'flex-basis':
        if val == 'content':
            return f"{s}.flex_basis = Length::max_content();"
        length = parse_length(val, font_size)
        if length:
            # CSS spec: flex-basis does not accept negative lengths
            if 'Length::px(-' in length:
                pass  # invalid, skip
            else:
                return f"{s}.flex_basis = {length};"

    if prop == 'order':
        try:
            return f"{s}.order = {int(val)};"
        except ValueError:
            pass

    if prop in ('gap', 'row-gap', 'column-gap'):
        if prop == 'gap':
            # Split respecting parentheses (don't split inside calc())
            gap_parts = _split_respecting_parens(val)
            if len(gap_parts) == 2:
                row_len = parse_length(gap_parts[0], font_size)
                col_len = parse_length(gap_parts[1], font_size)
                if row_len and col_len:
                    return [
                        f"{s}.row_gap = Some({row_len});",
                        f"{s}.column_gap = Some({col_len});",
                    ]
            elif len(gap_parts) == 1:
                length = parse_length(gap_parts[0], font_size)
                if length:
                    return [
                        f"{s}.row_gap = Some({length});",
                        f"{s}.column_gap = Some({length});",
                    ]
            # Fallback: try whole value as single length
            length = parse_length(val, font_size)
            if length:
                return [
                    f"{s}.row_gap = Some({length});",
                    f"{s}.column_gap = Some({length});",
                ]
        else:
            length = parse_length(val, font_size)
            if length:
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
        length = parse_length(val, font_size)
        if length:
            return f"{s}.column_width = Some({length});"

    if prop == 'column-height':
        if val == 'auto':
            return f"{s}.column_height = None;"
        length = parse_length(val, font_size)
        if length:
            return f"{s}.column_height = Some({length});"

    if prop == 'column-fill':
        mapping = {
            'balance': 'ColumnFill::Balance',
            'balance-all': 'ColumnFill::BalanceAll',
            'auto': 'ColumnFill::Auto',
        }
        if val in mapping:
            return f"{s}.column_fill = {mapping[val]};"

    if prop == 'column-wrap':
        mapping = {'auto': 'ColumnWrap::Auto', 'wrap': 'ColumnWrap::Wrap', 'nowrap': 'ColumnWrap::NoWrap'}
        if val in mapping:
            return f"{s}.column_wrap = {mapping[val]};"

    if prop == 'column-span':
        mapping = {'none': 'ColumnSpan::None', 'all': 'ColumnSpan::All'}
        if val in mapping:
            return f"{s}.column_span = {mapping[val]};"

    if prop == 'ruby-position':
        mapping = {'over': 'RubyPosition::Over', 'under': 'RubyPosition::Under'}
        if val in mapping:
            return f"{s}.ruby_position = {mapping[val]};"

    # ── line-height ──
    if prop == 'line-height':
        if val == 'normal':
            return f"{s}.line_height = LineHeight::Normal;"
        # line-height: Length takes f32 (px value), not Length type
        m = re.match(r'^(-?[\d.]+)px$', val.strip())
        if m:
            return f"{s}.line_height = LineHeight::Length({float(m.group(1))});"
        m = re.match(r'^(-?[\d.]+)(em|rem)$', val.strip())
        if m:
            basis = 16.0 if m.group(2) == 'rem' else font_size
            return f"{s}.line_height = LineHeight::Length({float(m.group(1)) * basis});"
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

    # ── page-break-before/after (legacy → break-before/after) ──
    if prop in ('page-break-before', 'page-break-after'):
        # CSS 2.1 page-break maps to CSS3 break properties
        legacy_mapping = {
            'auto': 'BreakValue::Auto', 'avoid': 'BreakValue::Avoid',
            'always': 'BreakValue::Page', 'left': 'BreakValue::Left',
            'right': 'BreakValue::Right',
        }
        if val in legacy_mapping:
            rust_prop = prop.replace('page-break', 'break').replace('-', '_')
            return f"{s}.{rust_prop} = {legacy_mapping[val]};"

    # Blink's legacy column-break aliases remain observable in old paged and
    # multicol content. ``always`` forces a column boundary rather than the
    # page boundary represented by page-break-*.
    if prop in ('-webkit-column-break-before', '-webkit-column-break-after'):
        legacy_mapping = {
            'auto': 'BreakValue::Auto',
            'avoid': 'BreakValue::AvoidColumn',
            'always': 'BreakValue::Column',
        }
        if val in legacy_mapping:
            rust_prop = prop.removeprefix('-webkit-column-').replace('-', '_')
            return f"{s}.{rust_prop} = {legacy_mapping[val]};"

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

    # ── white-space ──
    if prop == 'white-space':
        mapping = {
            'normal': 'WhiteSpace::Normal', 'nowrap': 'WhiteSpace::Nowrap',
            'pre': 'WhiteSpace::Pre', 'pre-wrap': 'WhiteSpace::PreWrap',
            'pre-line': 'WhiteSpace::PreLine', 'break-spaces': 'WhiteSpace::BreakSpaces',
        }
        if val in mapping:
            return f"{s}.white_space = {mapping[val]};"

    if prop == 'text-wrap':
        mapping = {
            'wrap': 'TextWrap::Wrap', 'nowrap': 'TextWrap::Nowrap',
            'balance': 'TextWrap::Balance', 'pretty': 'TextWrap::Pretty',
            'stable': 'TextWrap::Stable',
        }
        if val in mapping:
            return f"{s}.text_wrap = {mapping[val]};"

    # ── word-break ──
    if prop == 'word-break':
        mapping = {
            'normal': 'WordBreak::Normal', 'break-all': 'WordBreak::BreakAll',
            'keep-all': 'WordBreak::KeepAll', 'break-word': 'WordBreak::BreakWord',
        }
        if val in mapping:
            return f"{s}.word_break = {mapping[val]};"

    if prop == 'overflow-wrap':
        mapping = {
            'normal': 'OverflowWrap::Normal',
            'break-word': 'OverflowWrap::BreakWord',
            'anywhere': 'OverflowWrap::Anywhere',
        }
        if val in mapping:
            return f"{s}.overflow_wrap = {mapping[val]};"

    if prop == 'line-break':
        mapping = {
            'auto': 'LineBreak::Auto', 'loose': 'LineBreak::Loose',
            'normal': 'LineBreak::Normal', 'strict': 'LineBreak::Strict',
            'anywhere': 'LineBreak::Anywhere',
        }
        if val in mapping:
            return f"{s}.line_break = {mapping[val]};"

    if prop == 'hyphens':
        mapping = {
            'none': 'Hyphens::None', 'manual': 'Hyphens::Manual',
            'auto': 'Hyphens::Auto',
        }
        if val in mapping:
            return f"{s}.hyphens = {mapping[val]};"

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
        if val.endswith('%'):
            try:
                return f"{s}.vertical_align = VerticalAlign::Percentage({float(val[:-1])});"
            except ValueError:
                return None
        px = _css_length_px(val, font_size)
        if px is not None:
            return f"{s}.vertical_align = VerticalAlign::Length({float(px)});"

    # ── SP14 text-mode-only properties ──
    # These affect layout only when text is present. Emitted exclusively for
    # text-retaining ports so box-only output stays byte-identical (they are
    # in IGNORED_PROPERTIES for the legacy corpus).
    if RETAIN_TEXT:
        if prop == 'text-overflow':
            mapping = {
                'clip': 'TextOverflow::Clip',
                'ellipsis': 'TextOverflow::Ellipsis',
            }
            if val in mapping:
                return f"{s}.text_overflow = {mapping[val]};"
        if prop == 'text-indent':
            length = parse_length(val, font_size)
            if length:
                return f"{s}.text_indent = {length};"
        if prop in ('letter-spacing', 'word-spacing'):
            field = prop.replace('-', '_')
            if val == 'normal':
                return f"{s}.{field} = 0.0;"
            px = _css_length_px(val, font_size)
            if px is not None:
                return f"{s}.{field} = {float(px)};"
        if prop == 'text-transform':
            mapping = {
                'none': 'TextTransform::None', 'capitalize': 'TextTransform::Capitalize',
                'uppercase': 'TextTransform::Uppercase', 'lowercase': 'TextTransform::Lowercase',
            }
            if val in mapping:
                return f"{s}.text_transform = {mapping[val]};"

    # ── columns shorthand (column-count + column-width) ──
    if prop == 'columns':
        lines = []
        if '/' in val:
            before, after = val.split('/', 1)
            height = parse_length(after.strip(), font_size)
            if height:
                lines.append(f"{s}.column_height = Some({height});")
            val = before.strip()
        parts = val.split()
        for part in parts:
            part = part.strip()
            if part == 'auto':
                continue
            m = re.match(r'^(\d+)$', part)
            if m:
                v = int(m.group(1))
                if v == 0:
                    lines.append(f"{s}.column_width = Some(Length::px(0.0));")
                else:
                    lines.append(f"{s}.column_count = Some({v});")
                continue
            length = parse_length(part, font_size)
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
            elif parse_border_width(part, font_size) is not None:
                lines.append(
                    f"{s}.column_rule_width = {parse_border_width(part, font_size)};"
                )
            elif parse_color(part):
                lines.append(f"{s}.column_rule_color = StyleColor::Resolved({parse_color(part)});")
        return lines if lines else None

    if prop == 'column-rule-width':
        px_val = parse_border_width(val, font_size)
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

    # ── outline shorthand ──
    if prop == 'outline':
        parts = val.split()
        lines = []
        for part in parts:
            part = part.strip()
            if not part:
                continue
            bstyle = border_style_to_rust(part)
            if bstyle:
                lines.append(f"{s}.outline_style = {bstyle};")
            elif parse_border_width(part) is not None:
                lines.append(f"{s}.outline_width = {parse_border_width(part)};")
            elif parse_color(part):
                lines.append(f"{s}.outline_color = StyleColor::Resolved({parse_color(part)});")
        return lines if lines else None

    if prop == 'outline-width':
        px_val = parse_border_width(val)
        if px_val is not None:
            return f"{s}.outline_width = {px_val};"

    if prop == 'outline-style':
        style_code = border_style_to_rust(val)
        if style_code:
            return f"{s}.outline_style = {style_code};"

    if prop == 'outline-color':
        color = parse_color(val)
        if color:
            return f"{s}.outline_color = StyleColor::Resolved({color});"

    if prop == 'outline-offset':
        length = parse_length(val, font_size)
        if length:
            # outline-offset is stored as i32 pixels
            m = re.match(r'^(-?[\d.]+)(px|em|rem)?$', val.strip())
            if m:
                num = float(m.group(1))
                unit = m.group(2) or 'px'
                if unit in ('em', 'rem'):
                    num = num * font_size
                return f"{s}.outline_offset = {int(num)};"

    # ── logical properties: block-size/inline-size → height/width (horizontal writing mode) ──
    if prop in ('block-size', 'min-block-size', 'max-block-size'):
        length = parse_length(val, font_size)
        if length:
            physical = prop.replace('block-size', 'height').replace('-', '_')
            return f"{s}.{physical} = {length};"

    if prop in ('inline-size', 'min-inline-size', 'max-inline-size'):
        length = parse_length(val, font_size)
        if length:
            physical = prop.replace('inline-size', 'width').replace('-', '_')
            return f"{s}.{physical} = {length};"

    # ── inset (shorthand for top/right/bottom/left) ──
    if prop == 'inset':
        parts = val.split()
        props = ['top', 'right', 'bottom', 'left']
        if len(parts) == 1:
            length = parse_length(parts[0], font_size)
            if length:
                return [f"{s}.{p} = {length};" for p in props]
        elif len(parts) == 2:
            tb, lr = parse_length(parts[0], font_size), parse_length(parts[1], font_size)
            if tb and lr:
                return [f"{s}.top = {tb};", f"{s}.right = {lr};", f"{s}.bottom = {tb};", f"{s}.left = {lr};"]
        elif len(parts) == 4:
            lengths = [parse_length(p, font_size) for p in parts]
            if all(lengths):
                return [f"{s}.{props[i]} = {lengths[i]};" for i in range(4)]

    # ── inset-block / inset-inline (logical shorthands) ──
    if prop == 'inset-block':
        length = parse_length(val, font_size)
        if length:
            return [f"{s}.top = {length};", f"{s}.bottom = {length};"]

    if prop == 'inset-inline':
        length = parse_length(val, font_size)
        if length:
            return [f"{s}.left = {length};", f"{s}.right = {length};"]

    # ── inset-block-start/end, inset-inline-start/end (individual logical inset) ──
    _inset_logical_map = {
        'inset-block-start': 'top', 'inset-block-end': 'bottom',
        'inset-inline-start': 'left', 'inset-inline-end': 'right',
    }
    if prop in _inset_logical_map:
        length = parse_length(val, font_size)
        if length:
            physical = _inset_logical_map[prop]
            return f"{s}.{physical} = {length};"

    # ── margin-block / margin-inline (logical margin shorthands) ──
    if prop == 'margin-block':
        parts = val.split()
        if len(parts) == 1:
            length = parse_length(parts[0], font_size) if parts[0] != 'auto' else 'LengthPercentageAuto::Auto'
            if length:
                return [f"{s}.margin_top = {length};", f"{s}.margin_bottom = {length};"]
        elif len(parts) == 2:
            start = parse_length(parts[0], font_size) if parts[0] != 'auto' else 'LengthPercentageAuto::Auto'
            end = parse_length(parts[1], font_size) if parts[1] != 'auto' else 'LengthPercentageAuto::Auto'
            if start and end:
                return [f"{s}.margin_top = {start};", f"{s}.margin_bottom = {end};"]

    if prop == 'margin-inline':
        parts = val.split()
        if len(parts) == 1:
            length = parse_length(parts[0], font_size) if parts[0] != 'auto' else 'LengthPercentageAuto::Auto'
            if length:
                return [f"{s}.margin_left = {length};", f"{s}.margin_right = {length};"]
        elif len(parts) == 2:
            start = parse_length(parts[0], font_size) if parts[0] != 'auto' else 'LengthPercentageAuto::Auto'
            end = parse_length(parts[1], font_size) if parts[1] != 'auto' else 'LengthPercentageAuto::Auto'
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
        length = parse_length(val, font_size)
        if length:
            return f"{s}.{physical} = {length};"

    # ── padding-block / padding-inline (logical padding shorthands) ──
    if prop == 'padding-block':
        parts = val.split()
        if len(parts) == 1:
            length = parse_length(parts[0], font_size)
            if length:
                return [f"{s}.padding_top = {length};", f"{s}.padding_bottom = {length};"]
        elif len(parts) == 2:
            start, end = parse_length(parts[0], font_size), parse_length(parts[1], font_size)
            if start and end:
                return [f"{s}.padding_top = {start};", f"{s}.padding_bottom = {end};"]

    if prop == 'padding-inline':
        parts = val.split()
        if len(parts) == 1:
            length = parse_length(parts[0], font_size)
            if length:
                return [f"{s}.padding_left = {length};", f"{s}.padding_right = {length};"]
        elif len(parts) == 2:
            start, end = parse_length(parts[0], font_size), parse_length(parts[1], font_size)
            if start and end:
                return [f"{s}.padding_left = {start};", f"{s}.padding_right = {end};"]

    # ── padding-block-start/end, padding-inline-start/end ──
    _padding_logical_map = {
        'padding-block-start': 'padding_top', 'padding-block-end': 'padding_bottom',
        'padding-inline-start': 'padding_left', 'padding-inline-end': 'padding_right',
    }
    if prop in _padding_logical_map:
        length = parse_length(val, font_size)
        if length:
            physical = _padding_logical_map[prop]
            return f"{s}.{physical} = {length};"

    # ── border-block / border-inline (logical border shorthands) ──
    if prop in ('border-block', 'border-block-start', 'border-block-end'):
        sides = {'border-block': ['top', 'bottom'],
                 'border-block-start': ['top'], 'border-block-end': ['bottom']}[prop]
        lines = []
        for part in val.split():
            bw = parse_border_width(part, font_size)
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
        bw = parse_border_width(val.strip(), font_size)
        if bw:
            return [f"{s}.{p} = {bw};" for p in _border_width_logical_map[prop]]

    # ── real-font computed longhands ──
    if is_real_font_profile() and prop == 'font-family':
        family = _font_family_to_rust(val)
        if family:
            return f"{s}.font_family = {family};"
        raise UnsupportedFontShorthand(f"unsupported font-family: {val}")

    if is_real_font_profile() and prop == 'font-weight':
        weights = {
            'normal': 400.0, 'bold': 700.0, 'bolder': 700.0, 'lighter': 300.0,
        }
        if val in weights:
            return f"{s}.font_weight = FontWeight({weights[val]});"
        if re.match(r'^\d{1,4}$', val) and 1 <= int(val) <= 1000:
            return f"{s}.font_weight = FontWeight({float(val)});"
        raise UnsupportedFontShorthand(f"unsupported font-weight: {val}")

    if is_real_font_profile() and prop == 'font-style':
        if val == 'normal':
            return f"{s}.font_style = FontStyleEnum::Normal;"
        if val == 'italic':
            return f"{s}.font_style = FontStyleEnum::Italic;"
        m = re.match(r'^oblique(?:\s+([-+]?[\d.]+)deg)?$', val)
        if m:
            angle = float(m.group(1)) if m.group(1) else 14.0
            return f"{s}.font_style = FontStyleEnum::Oblique({angle});"
        raise UnsupportedFontShorthand(f"unsupported font-style: {val}")

    if is_real_font_profile() and prop == 'font-stretch':
        values = {
            'ultra-condensed': 50.0, 'extra-condensed': 62.5, 'condensed': 75.0,
            'semi-condensed': 87.5, 'normal': 100.0, 'semi-expanded': 112.5,
            'expanded': 125.0, 'extra-expanded': 150.0, 'ultra-expanded': 200.0,
        }
        if val in values:
            return f"{s}.font_stretch = FontStretch({values[val]});"
        raise UnsupportedFontShorthand(f"unsupported font-stretch: {val}")

    if is_real_font_profile() and prop == 'font-variant-caps':
        values = {'normal': 'Normal', 'small-caps': 'SmallCaps'}
        if val in values:
            return f"{s}.font_variant_caps = FontVariantCaps::{values[val]};"
        raise UnsupportedFontShorthand(f"unsupported font variant: {val}")

    # ── font shorthand (extract font-size) ──
    if prop == 'font':
        # font: <size>/<line-height> <family> or <size> <family> etc.
        m = re.match(r'(?:(?:normal|italic|oblique|bold|bolder|lighter|\d{3})\s+)*'
                     r'(-?[\d.]+)(px|em|rem)(?:\s*/\s*[\d.]+(?:px|em|rem|%)?)?', val.strip())
        if m:
            size_val = float(m.group(1))
            unit = m.group(2)
            if unit == 'px':
                size_code = f"{s}.font_size = {size_val};"
            elif unit in ('em', 'rem'):
                size_code = f"{s}.font_size = {size_val * 16.0};"
            else:
                return None
            line_height = _line_height_from_font_shorthand(val)
            line_height_code = generate_single_style(
                'line-height', line_height, s, font_size
            )
            return [size_code, line_height_code] if line_height_code else size_code

    # ── font-size ──
    if prop == 'font-size':
        if is_real_font_profile():
            return f"{s}.font_size = {float(font_size)};"
        v = val.strip()
        if v == '0':
            return f"{s}.font_size = 0.0;"
        viewport = _viewport_px_rust(v)
        if viewport:
            return f"{s}.font_size = {viewport};"
        m = re.match(r'^(-?[\d.]+)px$', v)
        if m:
            return f"{s}.font_size = {float(m.group(1))};"
        m = re.match(r'^(-?[\d.]+)(em|rem)$', v)
        if m:
            return f"{s}.font_size = {float(m.group(1)) * 16.0};"
        m = re.match(r'^(-?[\d.]+)%$', v)
        if m:
            # `font_size` is the already-computed used value threaded by
            # generate_style_code, including the inherited percentage basis.
            return f"{s}.font_size = {float(font_size)};"
        m = re.match(r'^(-?[\d.]+)pt$', v)
        if m:
            return f"{s}.font_size = {float(m.group(1)) * 4.0 / 3.0};"
        m = re.match(r'^(-?[\d.]+)(pc|in|cm|mm)$', v)
        if m:
            factors = {
                'pc': 16.0,
                'in': 96.0,
                'cm': 96.0 / 2.54,
                'mm': 96.0 / 25.4,
            }
            return f"{s}.font_size = {float(m.group(1)) * factors[m.group(2)]};"

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
        length = parse_length(val, font_size)
        if length:
            return [f"{s}.margin_top = {length};", f"{s}.margin_bottom = {length};"]

    if prop == 'margin-inline':
        length = parse_length(val, font_size)
        if length:
            return [f"{s}.margin_left = {length};", f"{s}.margin_right = {length};"]

    # ── padding-block / padding-inline (logical) ──
    if prop == 'padding-block':
        length = parse_length(val, font_size)
        if length:
            return [f"{s}.padding_top = {length};", f"{s}.padding_bottom = {length};"]

    if prop == 'padding-inline':
        length = parse_length(val, font_size)
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
                return [f"{s}.flex_grow = {g};", f"{s}.flex_shrink = 1.0;", f"{s}.flex_basis = Length::percent(0.0);"]
            except ValueError:
                basis = parse_length(val, font_size)
                if basis and 'Length::px(-' not in basis:
                    # A lone <flex-basis> expands to `1 1 <flex-basis>`.
                    return [f"{s}.flex_grow = 1.0;", f"{s}.flex_shrink = 1.0;", f"{s}.flex_basis = {basis};"]
        elif len(parts) == 2:
            # flex: <grow> <shrink> | <grow> <basis>
            try:
                g = float(parts[0])
                # Try second as shrink factor
                try:
                    sh = float(parts[1])
                    return [f"{s}.flex_grow = {g};", f"{s}.flex_shrink = {sh};", f"{s}.flex_basis = Length::percent(0.0);"]
                except ValueError:
                    # Second is basis
                    basis = parse_length(parts[1], font_size)
                    if basis:
                        return [f"{s}.flex_grow = {g};", f"{s}.flex_shrink = 1.0;", f"{s}.flex_basis = {basis};"]
            except ValueError:
                pass
        elif len(parts) == 3:
            # flex: <grow> <shrink> <basis>
            try:
                g = float(parts[0])
                sh = float(parts[1])
                try:
                    bv = float(parts[2])
                    # A unitless zero is accepted in this grammar and the
                    # shorthand's omitted/zero basis computes as 0%, while a
                    # unitless non-zero basis invalidates the declaration.
                    if bv != 0:
                        return None
                    basis = 'Length::percent(0.0)'
                except ValueError:
                    basis = parse_length(parts[2], font_size)
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


def generate_shorthand_4(val: str, s: str, prefix: str, suffix: str = '', font_size: float = 16.0) -> list[str] | None:
    """Generate 4-side shorthand (margin, padding, border-width)."""
    parts = val.split()
    if len(parts) == 1:
        length = parse_length(parts[0], font_size)
        if length:
            sides = ['top', 'right', 'bottom', 'left']
            return [f"{s}.{prefix}_{side}{suffix} = {length};" for side in sides]
    elif len(parts) == 2:
        tb = parse_length(parts[0], font_size)
        lr = parse_length(parts[1], font_size)
        if tb and lr:
            return [
                f"{s}.{prefix}_top{suffix} = {tb};",
                f"{s}.{prefix}_right{suffix} = {lr};",
                f"{s}.{prefix}_bottom{suffix} = {tb};",
                f"{s}.{prefix}_left{suffix} = {lr};",
            ]
    elif len(parts) == 3:
        top = parse_length(parts[0], font_size)
        lr = parse_length(parts[1], font_size)
        bot = parse_length(parts[2], font_size)
        if top and lr and bot:
            return [
                f"{s}.{prefix}_top{suffix} = {top};",
                f"{s}.{prefix}_right{suffix} = {lr};",
                f"{s}.{prefix}_bottom{suffix} = {bot};",
                f"{s}.{prefix}_left{suffix} = {lr};",
            ]
    elif len(parts) == 4:
        lengths = [parse_length(p, font_size) for p in parts]
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


def generate_border_shorthand(
    val: str, s: str, sides: list, font_size: float = 16.0
) -> list[str] | None:
    """Parse 'border: 1px solid red' shorthand."""
    parts = _split_respecting_parens(val)
    width = None
    style = None
    color = None

    for part in parts:
        part = part.strip()
        if not part:
            continue
        if border_style_to_rust(part):
            style = border_style_to_rust(part)
        elif parse_border_width(part, font_size) is not None:
            width = parse_border_width(part, font_size)
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


def generate_border_width_shorthand(
    val: str, s: str, font_size: float = 16.0
) -> list[str] | None:
    """Parse 'border-width: 1px 2px 3px 4px' shorthand to i32."""
    parts = val.split()
    if len(parts) == 1:
        w = parse_border_width(parts[0], font_size)
        if w is not None:
            return [f"{s}.border_{side}_width = {w};" for side in ['top', 'right', 'bottom', 'left']]
    elif len(parts) == 2:
        tb = parse_border_width(parts[0], font_size)
        lr = parse_border_width(parts[1], font_size)
        if tb is not None and lr is not None:
            return [
                f"{s}.border_top_width = {tb};",
                f"{s}.border_right_width = {lr};",
                f"{s}.border_bottom_width = {tb};",
                f"{s}.border_left_width = {lr};",
            ]
    elif len(parts) == 3:
        top = parse_border_width(parts[0], font_size)
        lr = parse_border_width(parts[1], font_size)
        bottom = parse_border_width(parts[2], font_size)
        if top is not None and lr is not None and bottom is not None:
            return [
                f"{s}.border_top_width = {top};",
                f"{s}.border_right_width = {lr};",
                f"{s}.border_bottom_width = {bottom};",
                f"{s}.border_left_width = {lr};",
            ]
    elif len(parts) == 4:
        ws = [parse_border_width(p, font_size) for p in parts]
        if all(w is not None for w in ws):
            sides = ['top', 'right', 'bottom', 'left']
            return [f"{s}.border_{side}_width = {ws[i]};" for i, side in enumerate(sides)]
    return None


def generate_border_style_shorthand(val: str, s: str) -> list[str] | None:
    """Parse 'border-style: solid dashed' shorthand."""
    parts = val.split()
    sides = ['top', 'right', 'bottom', 'left']
    if len(parts) == 1:
        code = border_style_to_rust(parts[0])
        if code:
            return [f"{s}.border_{side}_style = {code};" for side in sides]
    elif len(parts) == 2:
        codes = [border_style_to_rust(p) for p in parts]
        if all(codes):
            return [
                f"{s}.border_top_style = {codes[0]};",
                f"{s}.border_right_style = {codes[1]};",
                f"{s}.border_bottom_style = {codes[0]};",
                f"{s}.border_left_style = {codes[1]};",
            ]
    elif len(parts) == 3:
        codes = [border_style_to_rust(p) for p in parts]
        if all(codes):
            return [
                f"{s}.border_top_style = {codes[0]};",
                f"{s}.border_right_style = {codes[1]};",
                f"{s}.border_bottom_style = {codes[2]};",
                f"{s}.border_left_style = {codes[1]};",
            ]
    elif len(parts) == 4:
        codes = [border_style_to_rust(p) for p in parts]
        if all(codes):
            return [f"{s}.border_{sides[i]}_style = {codes[i]};" for i in range(4)]
    return None


def generate_border_color_shorthand(val: str, s: str) -> list[str] | None:
    """Parse 'border-color: red blue green yellow' shorthand."""
    parts = val.split()
    sides = ['top', 'right', 'bottom', 'left']
    if len(parts) == 1:
        color = parse_color(parts[0])
        if color:
            return [f"{s}.border_{side}_color = StyleColor::Resolved({color});" for side in sides]
    elif len(parts) == 2:
        colors = [parse_color(p) for p in parts]
        if all(colors):
            return [
                f"{s}.border_top_color = StyleColor::Resolved({colors[0]});",
                f"{s}.border_right_color = StyleColor::Resolved({colors[1]});",
                f"{s}.border_bottom_color = StyleColor::Resolved({colors[0]});",
                f"{s}.border_left_color = StyleColor::Resolved({colors[1]});",
            ]
    elif len(parts) == 3:
        colors = [parse_color(p) for p in parts]
        if all(colors):
            return [
                f"{s}.border_top_color = StyleColor::Resolved({colors[0]});",
                f"{s}.border_right_color = StyleColor::Resolved({colors[1]});",
                f"{s}.border_bottom_color = StyleColor::Resolved({colors[2]});",
                f"{s}.border_left_color = StyleColor::Resolved({colors[1]});",
            ]
    elif len(parts) == 4:
        colors = [parse_color(p) for p in parts]
        if all(colors):
            return [f"{s}.border_{sides[i]}_color = StyleColor::Resolved({colors[i]});" for i in range(4)]
    return None


# ─── Document builder generation ──────────────────────────────────────────

# ── SP14: text-node emission (gated) ─────────────────────────────────────
# Historically the porter skipped text nodes ("comparing layout only"), which
# is why every text-containing WPT test is classified `needs_text`. SP14 emits
# real `ElementTag::Text` nodes so text can be pixel-compared against Chromium.
#
# This is OFF by default so regenerating the existing corpus produces
# byte-identical box-only builders (no churn / zero regression to the passing
# set). SP14 enables it per-pilot: either set EMIT_TEXT_NODES = True for a
# scoped run, or add specific test source paths to EMIT_TEXT_FOR (an allowlist
# keyed by the porter's per-test gating — see callers).
EMIT_TEXT_NODES = False

# SP14: retain text in the Chrome HTML template (skip the TextStripper) and
# force the deterministic Ahem font on BOTH sides. Must be enabled together
# with EMIT_TEXT_NODES for symmetric text-retaining ports. OFF by default:
# box-only output stays byte-identical. Enabled by tools/wpt/splice_text_port.py.
RETAIN_TEXT = False

# Keep display:contents elements in the generated DOM. They produce no
# principal layout box, but their computed style remains the inheritance and
# custom-property boundary for descendants. Layout engines flatten the node
# when constructing their formatting trees.
PRESERVE_DISPLAY_CONTENTS_NODES = True

# CSS override appended to text-retaining Chrome templates. Forces Ahem with
# an explicit ordered set of Chromium-pinned CJK, complex-script, emoji, and
# DejaVu terminal fallbacks. Every family is registered by the manifest-scoped
# fontconfig and OpenUI's in-process manager in the same order.
# everywhere (deterministic glyph boxes, zero-AA via ahem_noaa.conf) and
# neutralizes UA styling our engine does not replicate (synthetic bold/italic,
# underlines, list markers). The Rust side mirrors this by forcing Ahem on
# every emitted Text node and ignoring font-weight/style/text-decoration.
TEXT_TEMPLATE_OVERRIDE = (
    '<style style="display:none!important">body, body * { font-family: Ahem, "Droid Sans Fallback", "Noto Sans Devanagari", "Noto Color Emoji", "DejaVu Sans" !important; '
    "font-weight: normal !important; font-style: normal !important; "
    "font-synthesis: none !important; text-decoration: none !important; "
    "list-style: none !important; font-kerning: none !important; "
    "font-variant-ligatures: none !important; }</style>"
)

DETERMINISTIC_FONT_FAMILY_RUST = (
    'FontFamilyList { families: vec!['
    'FontFamily::Named("Ahem".to_string()), '
    'FontFamily::Named("Droid Sans Fallback".to_string()), '
    'FontFamily::Named("Noto Sans Devanagari".to_string()), '
    'FontFamily::Named("Noto Color Emoji".to_string()), '
    'FontFamily::Named("DejaVu Sans".to_string())] }'
)


def _rust_escape_string(s: str) -> str:
    """Escape a Python string for embedding in a Rust double-quoted literal."""
    escaped = (
        s.replace('\\', '\\\\')
         .replace('"', '\\"')
         .replace('\n', '\\n')
         .replace('\r', '\\r')
         .replace('\t', '\\t')
    )
    # Keep generated Rust source ASCII-only and stable across locale/editor
    # settings while preserving the exact Unicode scalar values.
    return ''.join(
        ch if ord(ch) < 0x80 else f'\\u{{{ord(ch):x}}}'
        for ch in escaped
    )


def _font_family_to_rust(css_family: str) -> str | None:
    """Convert a CSS font-family value to a Rust FontFamilyList expression.

    Preserves the complete authored fallback order. Generic families remain
    typed generic values so `openui-text` can deterministically map the three
    SP16 generics to their vendored DejaVu assets.
    """
    if not css_family:
        return None
    families = []
    current = []
    quote = None
    for ch in css_family:
        if quote:
            current.append(ch)
            if ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
            current.append(ch)
        elif ch == ',':
            families.append(''.join(current).strip())
            current = []
        else:
            current.append(ch)
    families.append(''.join(current).strip())
    if quote or not families or any(not family for family in families):
        return None
    generics = {
        'serif': 'Serif', 'sans-serif': 'SansSerif', 'monospace': 'Monospace',
        'cursive': 'Cursive', 'fantasy': 'Fantasy', 'system-ui': 'SystemUi',
        'ui-serif': 'UiSerif', 'ui-sans-serif': 'UiSansSerif',
        'ui-monospace': 'UiMonospace', 'ui-rounded': 'UiRounded',
    }
    rust_families = []
    for family in families:
        family = family.strip().strip('"\'').strip()
        if not family:
            return None
        generic = generics.get(family.lower())
        if generic:
            rust_families.append(f"FontFamily::Generic(GenericFontFamily::{generic})")
        else:
            rust_families.append(
                f'FontFamily::Named("{_rust_escape_string(family)}".to_string())'
            )
    return "FontFamilyList { families: vec![" + ", ".join(rust_families) + "] }"


def _family_from_font_shorthand(val: str) -> str | None:
    """Extract the font-family portion from a CSS `font` shorthand value.

    e.g. `20px/1 Ahem` -> `Ahem`, `bold 16px Ahem, sans-serif` -> `Ahem, ...`.
    The family list follows the size (and optional /line-height) token.
    """
    if not val:
        return None
    val = val.strip().rstrip(';').strip()
    m = re.search(r'\d[\d.]*(px|pt|em|rem|%)(\s*/\s*\S+)?', val)
    if not m:
        return None
    family = val[m.end():].strip()
    return family or None


def _line_height_from_font_shorthand(val: str) -> str:
    """Return the CSS line-height carried by a `font` shorthand.

    A font shorthand resets line-height to `normal` when the slash component
    is absent. Keeping that reset in the inherited Text-node style is needed
    because generated element styles are not automatically inherited by the
    hand-built DOM.
    """
    if not val:
        return "normal"
    m = re.search(
        r'\d[\d.]*(?:px|pt|em|rem|%)(?:\s*/\s*([^\s]+))?', val.strip()
    )
    if not m or not m.group(1):
        return "normal"
    return m.group(1)


# ── SP14 text-mode helpers ─────────────────────────────────────────────────

# CSS-inherited text properties threaded to Text nodes in text mode, beyond
# the standing INHERITED_PROPS set (kept unchanged for box-mode stability).
TEXT_EXTRA_INHERITED = {
    'text-transform', 'letter-spacing', 'word-spacing', 'text-shadow', 'quotes',
    'text-emphasis-style', 'text-emphasis-position', 'text-emphasis-color',
}

_BORDER_RADIUS_CORNERS = (
    'border-top-left-radius',
    'border-top-right-radius',
    'border-bottom-right-radius',
    'border-bottom-left-radius',
)


def _expand_border_radius_css(value: str) -> dict[str, str] | None:
    """Expand a CSS border-radius shorthand without resolving its units.

    Percentages are computed against the box that receives the inherited
    value, so the porter must retain the CSS tokens rather than prematurely
    converting them against the parent's dimensions.
    """
    parts = value.split('/')
    if len(parts) > 2:
        return None
    horizontal = parts[0].split()
    vertical = parts[1].split() if len(parts) == 2 else horizontal

    def expand(values: list[str]) -> tuple[str, str, str, str] | None:
        if len(values) == 1:
            return (values[0],) * 4
        if len(values) == 2:
            return values[0], values[1], values[0], values[1]
        if len(values) == 3:
            return values[0], values[1], values[2], values[1]
        if len(values) == 4:
            return tuple(values)
        return None

    h_values = expand(horizontal)
    v_values = expand(vertical)
    if h_values is None or v_values is None:
        return None
    return {
        corner: h if h == v else f"{h} {v}"
        for corner, h, v in zip(_BORDER_RADIUS_CORNERS, h_values, v_values)
    }


def _computed_border_radius(styles: dict[str, str]) -> dict[str, str] | None:
    """Return the four computed corner tokens after shorthand expansion."""
    computed = {corner: '0' for corner in _BORDER_RADIUS_CORNERS}
    saw_radius = False
    for prop, value in styles.items():
        if prop == 'border-radius':
            expanded = _expand_border_radius_css(value)
            if expanded is not None:
                computed.update(expanded)
                saw_radius = True
        elif prop in _BORDER_RADIUS_CORNERS:
            computed[prop] = value
            saw_radius = True
    return computed if saw_radius else None

_INLINE_LEVEL_TAGS = {'span', 'a', 'em', 'strong', 'b', 'i', 'u', 'small',
                      'big', 'sub', 'sup', 'abbr', 'cite', 'code', 'mark',
                      'q', 's', 'del', 'ins', 'var', 'kbd', 'samp', 'br', 'wbr',
                      'label',
                      'img', 'canvas', 'svg', 'iframe', 'object', 'audio', 'video',
                      'input', 'button', 'meter', 'textarea', 'select', 'embed'}
_CSS_COLLAPSIBLE_WHITESPACE = ' \t\n\r\f'


def _is_css_whitespace_only(text: str) -> bool:
    """Whether text contains only CSS-collapsible whitespace characters.

    Python's ``str.strip`` also treats U+00A0 NO-BREAK SPACE as whitespace,
    but CSS Text does not collapse it.  Keeping that distinction here is
    essential because a lone ``&nbsp;`` creates a real line box.
    """
    return all(ch in _CSS_COLLAPSIBLE_WHITESPACE for ch in text)


def _subtree_text(node) -> str:
    """Concatenate all text content in a DomNode subtree."""
    if getattr(node, 'is_text', False):
        return getattr(node, 'text_content', '') or ''
    return ''.join(_subtree_text(c) for c in node.children)


def _embedded_multicol_leading_margin(root: 'DomNode') -> float:
    """Return a collapsed UA leading margin retained by an iframe canvas.

    A multicol body establishes the nested document canvas boundary. Blink
    retains the first paragraph's UA margin there, whereas ordinary column
    fragmentation trims a leading adjoining margin. Walk transparent block
    wrappers so static lowering can represent that boundary as padding.
    """
    if not any(prop in root.styles for prop in ('columns', 'column-count', 'column-width')):
        return 0.0
    current = root
    while True:
        children = [child for child in current.children if not child.is_text]
        if not children:
            return 0.0
        child = children[0]
        if child.styles.get('position', 'static') in ('absolute', 'fixed'):
            return 0.0
        authored_margin = child.styles.get('margin-top')
        if authored_margin:
            parsed = parse_length(authored_margin, 16.0)
            match = re.fullmatch(r'Length::px\((-?[\d.]+)\)', parsed or '')
            return float(match.group(1)) if match else 0.0
        if child.tag == 'p':
            return 16.0
        if any(
            prop in child.styles
            for prop in (
                'border', 'border-top', 'border-width', 'border-top-width',
                'padding', 'padding-top', 'height', 'min-height',
                'overflow', 'overflow-y', 'display',
            )
        ):
            return 0.0
        current = child


def _is_inline_level(node) -> bool:
    """Whether a DomNode participates in inline layout (for whitespace rules)."""
    if node is None:
        return False
    if getattr(node, 'is_text', False):
        text = getattr(node, 'text_content', '') or ''
        return bool(text) and not _is_css_whitespace_only(text)
    display = (node.styles or {}).get('display', '').strip()
    if display:
        return display.startswith('inline')
    return node.tag in _INLINE_LEVEL_TAGS


def _has_inline_boundary(node, *, trailing: bool) -> bool:
    """Whether an unboxed sibling exposes an inline box at one boundary.

    ``display:contents`` itself has no box, so inter-element whitespace next
    to it is governed by its first/last generated descendant.  Treating the
    unboxed element as block-level drops real word separators between adjacent
    inline descendants (while blindly treating it as inline would retain
    whitespace next to a nested block).
    """
    if node is None:
        return False
    if getattr(node, 'is_text', False):
        text = getattr(node, 'text_content', '') or ''
        return bool(text) and not _is_css_whitespace_only(text)
    if (node.styles or {}).get('display', '').strip() != 'contents':
        return _is_inline_level(node)

    children = reversed(node.children) if trailing else iter(node.children)
    for child in children:
        if (
            getattr(child, 'is_text', False)
            and _is_css_whitespace_only(getattr(child, 'text_content', '') or '')
        ):
            continue
        return _has_inline_boundary(child, trailing=trailing)
    return False


def _filter_ws_only_text_nodes(node, inherited_white_space='normal'):
    """Drop whitespace-only text nodes except between two inline-level siblings.

    Mirrors CSS white-space collapsing: inter-element whitespace between
    blocks produces no rendering in Chrome, so emitting Text nodes for it
    would create spurious line boxes in the Rust document.
    """
    if getattr(node, 'is_text', False):
        return
    kept = []
    children = node.children
    parent_display = (node.styles or {}).get('display', '').strip()
    white_space = (node.styles or {}).get('white-space', inherited_white_space).strip()
    preserves_white_space = white_space in ('pre', 'pre-wrap', 'break-spaces')
    preserves_segment_breaks = preserves_white_space or white_space == 'pre-line'
    for i, c in enumerate(children):
        if (
            getattr(c, 'is_text', False)
            and _is_css_whitespace_only(getattr(c, 'text_content', '') or '')
        ):
            text = getattr(c, 'text_content', '') or ''
            if preserves_white_space or (
                preserves_segment_breaks and any(ch in text for ch in '\n\r\f')
            ):
                kept.append(c)
                continue
            # Collapsible whitespace between flex items is not wrapped in an
            # anonymous flex item and contributes no flex base size.
            if parent_display in ('flex', 'inline-flex'):
                continue
            prev_node = kept[-1] if kept else None
            next_node = children[i + 1] if i + 1 < len(children) else None
            if not (
                _has_inline_boundary(prev_node, trailing=True)
                and _has_inline_boundary(next_node, trailing=False)
            ):
                continue
        kept.append(c)
    node.children = kept
    for c in kept:
        _filter_ws_only_text_nodes(c, white_space)


def generate_rust_fn(
    fn_name: str,
    root: DomNode,
    html_styles: dict | None = None,
    *,
    root_aware: bool = False,
) -> str:
    """Generate a Rust function that builds a Document matching the DOM tree."""
    global _ACTIVE_RESOURCE_BASE
    resource_base = getattr(root, 'resource_base', None)
    _ACTIVE_RESOURCE_BASE = Path(resource_base).resolve() if resource_base else None
    if RETAIN_TEXT:
        _filter_ws_only_text_nodes(root)
        root_white_space = (root.styles or {}).get('white-space', 'normal').strip()
        root_preserves_segment_breaks = root_white_space in (
            'pre', 'pre-line', 'pre-wrap', 'break-spaces',
        )
        if root_aware and root.children and not root_preserves_segment_breaks:
            # Collapsible whitespace at the start/end of the body formatting
            # context disappears. Trim only boundary text nodes; separators
            # between inline siblings remain intact.
            if root.children[0].is_text:
                root.children[0].text_content = root.children[0].text_content.lstrip(
                    _CSS_COLLAPSIBLE_WHITESPACE
                )
            if root.children and root.children[-1].is_text:
                root.children[-1].text_content = root.children[-1].text_content.rstrip(
                    _CSS_COLLAPSIBLE_WHITESPACE
                )

    def annotate_parent_tags(parent):
        fieldset_legend_index = 0
        for child in parent.children:
            child.parent_tag = parent.tag
            if parent.tag == 'fieldset' and child.tag == 'legend':
                child.fieldset_legend_index = fieldset_legend_index
                fieldset_legend_index += 1
            if not child.is_text:
                annotate_parent_tags(child)

    annotate_parent_tags(root)
    svg_definitions = {}

    def collect_svg_definitions(node):
        node_id = node.attrs.get('id', '') if not node.is_text else ''
        if node_id:
            svg_definitions[node_id] = node
        if not node.is_text:
            for child in node.children:
                collect_svg_definitions(child)

    collect_svg_definitions(root)
    lines = []
    lines.append(f"fn {fn_name}() -> Document {{")
    if root_aware:
        lines.append("    let (mut doc, html, vp) = root_doc();")
    else:
        lines.append("    let (mut doc, vp) = base_doc();")
    if RETAIN_TEXT:
        # Inline line-box struts use the block container's font metrics, not
        # only the leaf Text item's metrics. Pin every generated style to Ahem
        # (matching the template's `body, body *` override), beginning with the
        # viewport/body style. Hand-built DOM styles do not inherit implicitly.
        if not is_real_font_profile():
            lines.append(
                '    doc.node_mut(vp).style.font_family = '
                f'{DETERMINISTIC_FONT_FAMILY_RUST};'
            )

    # ── Body-background propagation (CSS Backgrounds §3.11.1) ──
    # If <html> has a non-transparent background, paint it on the canvas.
    # Otherwise, if <body> has a non-transparent background, propagate body's
    # background to the canvas (the "viewport") and let body still paint its
    # own box (matches Chromium behavior closely enough for solid colors).
    html_styles = html_styles or {}
    body_styles_d = root.styles or {}

    def _is_transparent(val: str) -> bool:
        if not val:
            return True
        v = val.strip().lower()
        return v in ('transparent', 'none', 'rgba(0,0,0,0)', 'rgba(0, 0, 0, 0)')

    html_bg = html_styles.get('background-color', '')
    html_bg_image = html_styles.get('background-image', '')
    body_bg = body_styles_d.get('background-color', '')
    canvas_color_line = None
    if not root_aware and html_bg and not _is_transparent(html_bg):
        c = parse_color(html_bg)
        if c:
            canvas_color_line = f"    doc.node_mut(doc.root()).style.background_color = {c};"
    elif not root_aware and (
        body_bg
        and not _is_transparent(body_bg)
        and (not html_bg_image or _is_transparent(html_bg_image))
    ):
        c = parse_color(body_bg)
        if c:
            canvas_color_line = f"    doc.node_mut(doc.root()).style.background_color = {c};"
    if canvas_color_line:
        lines.append(canvas_color_line)

    counter = [0]
    materialization_required = [False]
    generated_scroll_markers: list[
        tuple[str, CssDeclarations, float, float]
    ] = []
    css_targeted_tags = getattr(root, 'css_targeted_tags', set())

    def _has_meaningful_styles(styles):
        """Check if styles have properties beyond BODY_STYLE * rule defaults."""
        for prop, val in styles.items():
            if prop in ('margin', 'padding', 'box-sizing') and val in ('0', 'content-box'):
                continue
            return True
        return False

    # CSS inherited properties that must propagate to descendants
    INHERITED_PROPS = {'direction', 'writing-mode', 'text-orientation',
                       'text-combine-upright',
                       'color', 'white-space', 'word-break', 'overflow-wrap',
                       'line-break',
                       'hyphens', 'text-wrap', 'text-align',
                       'font-size', 'line-height', 'visibility',
                       'orphans', 'widows', 'ruby-position', 'text-shadow',
                       'text-emphasis-style', 'text-emphasis-position',
                       'text-emphasis-color',
                       'quotes', 'border-collapse', 'border-spacing',
                       'caption-side', 'empty-cells'}
    if is_real_font_profile():
        INHERITED_PROPS |= {
            'font-family', 'font-weight', 'font-style', 'font-stretch',
            'font-variant-caps',
        }
    # CSS-wide `inherit` can apply to any property; keep explicit parent
    # values for non-inherited properties that WPT coverage exercises.
    EXPLICIT_INHERIT_PROPS = INHERITED_PROPS | {
        'background', 'background-color', 'background-clip', 'box-shadow',
        'font-family', 'list-style-position', 'align-self',
        'column-count', 'column-width', 'column-height', 'column-gap',
        'column-fill', 'column-span', 'column-wrap',
        'column-rule-width', 'column-rule-style', 'column-rule-color',
        'unicode-bidi',
    }
    EXPLICIT_NON_INHERITED_INITIALS = {
        'background': 'transparent',
        'background-color': 'transparent',
        'background-clip': 'border-box',
        'box-shadow': 'none',
        'column-count': 'auto',
        'column-width': 'auto',
        'column-height': 'auto',
        'column-gap': 'normal',
        'column-fill': 'balance',
        'column-span': 'none',
        'column-wrap': 'auto',
        'column-rule-width': 'medium',
        'column-rule-style': 'none',
        'column-rule-color': 'currentcolor',
        'unicode-bidi': 'normal',
    }
    REAL_FONT_INITIALS = {
        'font-family': 'sans-serif',
        'font-size': '16px',
        'font-weight': 'normal',
        'font-style': 'normal',
        'font-stretch': 'normal',
        'font-variant-caps': 'normal',
        'line-height': 'normal',
    }

    def _computed_child_boundary(inherited):
        """Start an element's computed-value boundary for its children.

        Values tracked solely so CSS-wide ``inherit`` can read them are not
        naturally inherited. They therefore reset to their initial computed
        value at every element, including an unboxed display:contents one.
        """
        child = dict(inherited)
        child.update(EXPLICIT_NON_INHERITED_INITIALS)
        return child

    def _background_text_color(styles):
        """Return a solid color used by ``background-clip:text``."""
        if styles.get('background-clip', '').strip() != 'text':
            return None
        value = styles.get('background-color', '') or styles.get('background', '')
        if parse_color(value):
            return value
        for token in value.split():
            if parse_color(token):
                return token
        return None

    def _node_resource(source: str):
        """Return stable encoded-resource metadata for one replaced node."""
        inline = _data_url_resource(source)
        if inline is not None:
            source_label, mime, sha, dimensions, data = inline
            return source_label, mime, sha, dimensions, _resource_intrinsic_ratio(
                data, mime, dimensions,
            ), (
                f'vec![{", ".join(str(byte) for byte in data)}]'
            )
        packaged = _packaged_resource(source)
        if packaged is not None:
            filename, source_label, mime, sha, dimensions = packaged
            data = (SP20_ASSET_DIR / filename).read_bytes()
            return source_label, mime, sha, dimensions, _resource_intrinsic_ratio(
                data, mime, dimensions,
            ), _packaged_bytes_expr(filename)
        name = source.rsplit('/', 1)[-1]
        asset = _PAINT_ASSETS.get(name)
        dimensions = _REPLACED_ASSET_DIMENSIONS.get(name)
        if asset is None:
            return None
        filename, source_label, mime, sha = asset
        if filename is None:
            import base64
            data = base64.b64decode(_INLINE_PAINT_ASSETS[name])
            byte_expr = f'vec![{", ".join(str(byte) for byte in data)}]'
        else:
            byte_expr = (
                'include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), '
                f'"/../../../tools/accountability/data/wpt_assets/sp13p/{filename}"))'
                '.as_slice().to_vec()'
            )
        ratio = dimensions if dimensions and dimensions[0] > 0 and dimensions[1] > 0 else None
        return source_label, mime, sha, dimensions, ratio, byte_expr

    def emit_passive_scroll_button_defaults(
        pseudo_var: str, ws: str, *, disabled: bool, system_font: bool
    ) -> None:
        """Emit Chromium's stable Linux passive scroll-button appearance."""
        color = (
            "Color::from_rgba8(16, 16, 16, 77)"
            if disabled else "Color::BLACK"
        )
        border_width = 1 if disabled else 2
        border_color = 208 if disabled else 118
        background = 238 if disabled else 239
        inline_padding = 7 if disabled else 6
        lines.extend([
            f"{ws}doc.node_mut({pseudo_var}).form_control = "
            "Some(openui_dom::FormControlRole::Button);",
            f"{ws}doc.node_mut({pseudo_var}).style.width = Length::fit_content();",
            f"{ws}doc.node_mut({pseudo_var}).style.min_width = Length::px(27.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.height = Length::px(21.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.display = Display::InlineBlock;",
            f"{ws}doc.node_mut({pseudo_var}).style.box_sizing = BoxSizing::BorderBox;",
            f"{ws}if doc.node({pseudo_var}).style.writing_mode.is_horizontal() {{",
            f"{ws}    doc.node_mut({pseudo_var}).style.padding_top = Length::px(1.0);",
            f"{ws}    doc.node_mut({pseudo_var}).style.padding_right = Length::px({inline_padding}.0);",
            f"{ws}    doc.node_mut({pseudo_var}).style.padding_bottom = Length::px(1.0);",
            f"{ws}    doc.node_mut({pseudo_var}).style.padding_left = Length::px({inline_padding}.0);",
            f"{ws}}} else {{",
            f"{ws}    doc.node_mut({pseudo_var}).style.padding_top = Length::px({inline_padding}.0);",
            f"{ws}    doc.node_mut({pseudo_var}).style.padding_right = Length::px(1.0);",
            f"{ws}    doc.node_mut({pseudo_var}).style.padding_bottom = Length::px({inline_padding}.0);",
            f"{ws}    doc.node_mut({pseudo_var}).style.padding_left = Length::px(1.0);",
            f"{ws}}}",
            f"{ws}doc.node_mut({pseudo_var}).style.border_top_width = {border_width};",
            f"{ws}doc.node_mut({pseudo_var}).style.border_right_width = {border_width};",
            f"{ws}doc.node_mut({pseudo_var}).style.border_bottom_width = {border_width};",
            f"{ws}doc.node_mut({pseudo_var}).style.border_left_width = {border_width};",
            f"{ws}doc.node_mut({pseudo_var}).style.border_top_style = BorderStyle::Solid;",
            f"{ws}doc.node_mut({pseudo_var}).style.border_right_style = BorderStyle::Solid;",
            f"{ws}doc.node_mut({pseudo_var}).style.border_bottom_style = BorderStyle::Solid;",
            f"{ws}doc.node_mut({pseudo_var}).style.border_left_style = BorderStyle::Solid;",
            f"{ws}doc.node_mut({pseudo_var}).style.border_top_color = StyleColor::Resolved(Color::from_rgba8({border_color}, {border_color}, {border_color}, 255));",
            f"{ws}doc.node_mut({pseudo_var}).style.border_right_color = StyleColor::Resolved(Color::from_rgba8({border_color}, {border_color}, {border_color}, 255));",
            f"{ws}doc.node_mut({pseudo_var}).style.border_bottom_color = StyleColor::Resolved(Color::from_rgba8({border_color}, {border_color}, {border_color}, 255));",
            f"{ws}doc.node_mut({pseudo_var}).style.border_left_color = StyleColor::Resolved(Color::from_rgba8({border_color}, {border_color}, {border_color}, 255));",
            f"{ws}doc.node_mut({pseudo_var}).style.background_color = Color::from_rgba8({background}, {background}, {background}, 255);",
            f"{ws}doc.node_mut({pseudo_var}).style.border_top_left_radius = (2.0, 2.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.border_top_right_radius = (2.0, 2.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.border_bottom_right_radius = (2.0, 2.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.border_bottom_left_radius = (2.0, 2.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.color = {color};",
            f"{ws}doc.node_mut({pseudo_var}).style.font_size = 13.333333;",
            f"{ws}doc.node_mut({pseudo_var}).style.vertical_align = VerticalAlign::Top;",
            f"{ws}doc.node_mut({pseudo_var}).form_control_disabled = "
            f"{'true' if disabled else 'false'};",
        ])
        if system_font:
            lines.extend([
                f"{ws}doc.node_mut({pseudo_var}).style.font_family = "
                "FontFamilyList { families: vec![FontFamily::Generic("
                "GenericFontFamily::SansSerif)] };",
                f"{ws}doc.node_mut({pseudo_var}).style.native_control_text = true;",
            ])

    def emit_appearance_none_button_defaults(pseudo_var: str, ws: str) -> None:
        """Replace native button decoration after a winning appearance:none."""
        lines.extend([
            f"{ws}doc.node_mut({pseudo_var}).form_control_native_appearance = false;",
            f"{ws}doc.node_mut({pseudo_var}).style.background_color = Color::from_rgba8(250, 250, 250, 255);",
            f"{ws}doc.node_mut({pseudo_var}).style.padding_right = Length::px(6.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.padding_left = Length::px(6.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.border_top_width = 2;",
            f"{ws}doc.node_mut({pseudo_var}).style.border_right_width = 2;",
            f"{ws}doc.node_mut({pseudo_var}).style.border_bottom_width = 2;",
            f"{ws}doc.node_mut({pseudo_var}).style.border_left_width = 2;",
            f"{ws}doc.node_mut({pseudo_var}).style.border_top_color = StyleColor::Resolved(Color::from_rgba8(209, 209, 209, 255));",
            f"{ws}doc.node_mut({pseudo_var}).style.border_left_color = StyleColor::Resolved(Color::from_rgba8(209, 209, 209, 255));",
            f"{ws}doc.node_mut({pseudo_var}).style.border_right_color = StyleColor::Resolved(Color::from_rgba8(183, 183, 183, 255));",
            f"{ws}doc.node_mut({pseudo_var}).style.border_bottom_color = StyleColor::Resolved(Color::from_rgba8(183, 183, 183, 255));",
            f"{ws}doc.node_mut({pseudo_var}).style.border_top_left_radius = (0.0, 0.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.border_top_right_radius = (0.0, 0.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.border_bottom_right_radius = (0.0, 0.0);",
            f"{ws}doc.node_mut({pseudo_var}).style.border_bottom_left_radius = (0.0, 0.0);",
        ])

    def gen_node(node: DomNode, parent_var: str, indent: int,
                 parent_font_size: float = 16.0, inherited: dict | None = None,
                 custom_props: dict[str, str] | None = None,
                 parent_zoom: float = 1.0,
                 html_table_border: bool = False):
        if inherited is None:
            inherited = {}
        if custom_props is None:
            custom_props = {}

        isolated_document = bool(getattr(node, 'isolated_document', False))

        if node.is_text:
            # SP14: emit a real Text node when text emission is enabled; otherwise
            # skip (legacy box-only behavior, default — keeps the passing corpus
            # byte-identical). `text_content` is already whitespace-stripped; our
            # inline layout performs CSS white-space processing on the content.
            text = getattr(node, 'text_content', '') or ''
            structural_nbsp = (
                not EMIT_TEXT_NODES
                and '\u00a0' in text
                and not text.replace('\u00a0', '').strip()
            )
            if EMIT_TEXT_NODES or structural_nbsp:
                # In text mode whitespace-only nodes between inline siblings are
                # significant (word separators) — the block-context ones were
                # already dropped by _filter_ws_only_text_nodes.
                if text.strip() or (RETAIN_TEXT and text) or structural_nbsp:
                    counter[0] += 1
                    tvar = f"n{counter[0]}"
                    ws = "    " * indent
                    lines.append(f"{ws}let {tvar} = doc.create_node(ElementTag::Text);")
                    if inherited.get('__native_control_text'):
                        lines.append(
                            f"{ws}doc.node_mut({tvar}).style.native_control_text = true;"
                        )
                    if inherited.get('__native_button_text_metrics'):
                        lines.append(
                            f"{ws}doc.node_mut({tvar}).style.native_button_text_metrics = true;"
                        )
                    # SP14: our inline layout shapes text using the Text node's
                    # OWN computed style (font is not inherited to the Text child
                    # in the builder path), so set the effective inherited font
                    # and color explicitly on the emitted Text node.
                    ts = f"doc.node_mut({tvar}).style"
                    inherited_viewport_font = _viewport_px_rust(
                        str(inherited.get('font-size', ''))
                    )
                    text_font_size = inherited_viewport_font or str(
                        round(float(parent_font_size) * parent_zoom, 12)
                    )
                    lines.append(f"{ws}{ts}.font_size = {text_font_size};")
                    if isolated_document:
                        lines.append(f"{ws}{ts}.embedded_document_text = true;")
                    if (
                        RETAIN_TEXT
                        and not is_real_font_profile()
                        and not isolated_document
                    ):
                        # Deterministic-font mode: every glyph renders as Ahem on
                        # both sides (TEXT_TEMPLATE_OVERRIDE forces it in Chrome).
                        lines.append(
                            f'{ws}{ts}.font_family = '
                            f'{DETERMINISTIC_FONT_FAMILY_RUST};'
                        )
                    else:
                        fam = inherited.get('font-family')
                        fam_rust = _font_family_to_rust(fam) if fam else None
                        if fam_rust:
                            lines.append(f"{ws}{ts}.font_family = {fam_rust};")
                    col = inherited.get('color')
                    col_rust = parse_color(col) if col else None
                    if col_rust:
                        lines.append(f"{ws}{ts}.color = {col_rust};")
                    if RETAIN_TEXT:
                        # Thread inherited text-affecting properties onto the Text
                        # node itself: the inline items builder reads white-space /
                        # transform / spacing from the item's own style.
                        for prop in (
                            'font-weight', 'font-style', 'font-stretch',
                            'font-variant-caps', 'white-space', 'word-break',
                            'overflow-wrap', 'line-break', 'hyphens', 'text-wrap',
                            'line-height',
                            'text-transform', 'letter-spacing', 'word-spacing',
                            'direction', 'writing-mode',
                            'text-orientation', 'text-combine-upright',
                            'text-shadow', 'quotes', 'visibility',
                            'text-emphasis-style', 'text-emphasis-position',
                            'text-emphasis-color',
                        ):
                            if prop in inherited:
                                code = generate_single_style(prop, inherited[prop], ts, parent_font_size)
                                if code:
                                    for cl in (code if isinstance(code, list) else [code]):
                                        lines.append(f"{ws}{cl}")
                    lines.append(
                        f'{ws}doc.node_mut({tvar}).text = '
                        f'Some("{_rust_escape_string(text)}".to_string());'
                    )
                    lines.append(f"{ws}doc.append_child({parent_var}, {tvar});")
            return

        marker_scope_start = len(generated_scroll_markers)

        if node.tag in ('p', 'strong', 'em', 'b', 'i', 'u', 'a',
                        'h1', 'h2', 'h3', 'h4', 'h5', 'h6'):
            has_real_styles = _has_meaningful_styles(node.styles)
            skip_self = False
            skip_subtree = False
            heading_is_template_unstyled = (
                node.tag in ('h1', 'h2', 'h3', 'h4', 'h5', 'h6')
                and 'style' not in node.attrs
                and 'class' not in node.attrs
                and (
                    'id' not in node.attrs
                    or node.attrs.get('id', '') == 'testdetails'
                )
                and node.tag not in css_targeted_tags
            )
            # The Chrome template removes instructional pass-condition
            # paragraphs before its text-stripping pass.  Keep the generated
            # document tree identical in every porter profile, including the
            # box-only profiles where the paragraph itself may be styled even
            # though none of its text is emitted.
            if node.tag == 'p' and 'test passes' in _subtree_text(node).lower():
                skip_subtree = True
            if RETAIN_TEXT:
                # Text mode keeps wrapper elements (their text is content) but
                # mirrors the Chrome template's stripping exactly:
                #  - instructional "Test passes…" paragraphs are removed on both
                #    sides (template regex strip);
                #  - unstyled headings carry UA font styling we don't replicate,
                #    so both sides drop the whole subtree.
                if heading_is_template_unstyled:
                    skip_subtree = True
            else:
                # Skip unstyled wrapper/heading elements but still process their
                # children (reparented to the grandparent). Headings in WPT tests
                # are usually section labels with user-agent styling (margins,
                # bold, font-size) that our engine doesn't replicate. Skipping
                # them avoids mismatches.
                if heading_is_template_unstyled:
                    skip_subtree = True
                elif not has_real_styles and node.tag in ('p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6'):
                    skip_self = True
                if node.tag in ('strong', 'em', 'b', 'i', 'u', 'a') and not has_real_styles:
                    skip_self = True
            if skip_subtree:
                return
            if skip_self:
                # Merge any inherited props from this skipped node
                child_inherited = _computed_child_boundary(inherited)
                skip_inherit = EXPLICIT_INHERIT_PROPS | (
                    TEXT_EXTRA_INHERITED | set(_BORDER_RADIUS_CORNERS)
                    if RETAIN_TEXT else set()
                )
                for prop in sorted(skip_inherit):
                    if prop in node.styles:
                        child_inherited[prop] = node.styles[prop]
                child_custom_props = dict(custom_props)
                for prop, val in node.styles.items():
                    if prop.startswith('--'):
                        child_custom_props[prop] = val
                for child in node.children:
                    gen_node(
                        child, parent_var, indent, parent_font_size,
                        child_inherited, child_custom_props, parent_zoom,
                        html_table_border,
                    )
                return

        is_display_contents = node.styles.get('display', '').strip() == 'contents'
        if node.tag == 'defs':
            # SVG definitions contribute only when referenced by a graphics
            # element; they never generate their own painted subtree.
            return
        if (
            is_display_contents
            and node.tag == 'svg'
            and getattr(node, 'parent_tag', '') not in {
                'svg', 'g', 'defs', 'text', 'tspan',
            }
        ):
            # An outer SVG participates in HTML as a replaced element, so the
            # unusual-element rule computes `display:contents` to no box.
            return
        if is_display_contents and node.tag == 'use':
            href = node.attrs.get('xlink:href', node.attrs.get('href', ''))
            referenced = svg_definitions.get(href.removeprefix('#'))
            if referenced is not None:
                gen_node(
                    referenced, parent_var, indent, parent_font_size,
                    inherited, custom_props, parent_zoom, html_table_border,
                )
            return
        if is_display_contents and node.tag == 'text':
            # SVG text is a graphics element, not an HTML-style transparent
            # formatting wrapper. The unusual-element rule suppresses its
            # own character data when its principal box computes to
            # `display:contents`.
            return
        if (
            node.tag == 'br'
            and node.styles.get('display', '').strip() in ('none', 'contents')
        ):
            # CSS Display's unusual-element rules suppress a <br> whose
            # computed display is none/contents; it does not generate a forced
            # line break or a principal box.
            return

        if is_display_contents and node.tag in {
            'wbr', 'meter', 'progress', 'canvas', 'embed', 'object', 'audio',
            'iframe', 'img', 'video', 'input', 'textarea', 'select',
        }:
            # CSS Display's unusual-element appendix computes `contents` to
            # no generated box/content for replaced elements and native form
            # controls. In particular, an object's fallback children do not
            # escape when its principal replaced box is suppressed.
            return

        if (
            node.tag == 'br'
            and node.styles.get('clear', 'none') == 'none'
            and not RETAIN_TEXT
        ):
            # Box-only generation may omit glyph-bearing text, but a BR
            # remains a forced fragmentation opportunity even when it has
            # no painted content. Keep that structural effect in parity
            # with the Chrome template, which has always retained BRs.
            counter[0] += 1
            bvar = f"n{counter[0]}"
            ws = "    " * indent
            lines.append(f"{ws}let {bvar} = doc.create_node(ElementTag::Break);")
            lines.append(f"{ws}doc.node_mut({bvar}).style.display = Display::Inline;")
            lines.append(f"{ws}doc.append_child({parent_var}, {bvar});")
            return

        has_generated_contents_pseudo = any(
            pseudo.get('content', '').strip().lower() not in ('', 'normal', 'none')
            for pseudo in (
                node.pseudo_styles.get('before', {}),
                node.pseudo_styles.get('after', {}),
            )
        )
        if (
            RETAIN_TEXT
            and node.styles.get('display', '').strip() == 'contents'
            and not has_generated_contents_pseudo
            and not PRESERVE_DISPLAY_CONTENTS_NODES
        ):
            # display:contents generates no principal box. Reparent its
            # children while retaining the element's inheritance/custom-
            # property boundary; otherwise borders/backgrounds incorrectly
            # paint and block descendants participate in the wrong context.
            effective_styles = _copy_declarations(node.styles)
            for prop in sorted(_SP17_INHERITED_PROPERTIES):
                if prop in inherited and prop not in effective_styles:
                    effective_styles[prop] = inherited[prop]
            if is_real_font_profile():
                for prop in sorted(INHERITED_PROPS):
                    if prop in inherited and prop not in effective_styles:
                        effective_styles[prop] = inherited[prop]
            for prop, val in list(effective_styles.items()):
                if isinstance(val, str) and val.strip() == 'inherit' and prop in inherited:
                    effective_styles[prop] = inherited[prop]
            node_custom_props = dict(custom_props)
            for prop, val in effective_styles.items():
                if prop.startswith('--'):
                    node_custom_props[prop] = val
            for prop, val in list(effective_styles.items()):
                if isinstance(val, str) and not prop.startswith('--'):
                    effective_styles[prop] = _resolve_css_vars(val, node_custom_props)

            contents_zoom = _effective_css_zoom(effective_styles, parent_zoom)
            _ignored_lines, contents_font_size = generate_style_code(
                effective_styles, parent_var, parent_font_size, contents_zoom
            )
            child_inherited = _computed_child_boundary(inherited)
            contents_inherit_props = EXPLICIT_INHERIT_PROPS | TEXT_EXTRA_INHERITED
            for prop in sorted(contents_inherit_props):
                if prop in effective_styles:
                    child_inherited[prop] = effective_styles[prop]
            if is_real_font_profile():
                if 'font-size' in child_inherited:
                    child_inherited['font-size'] = f'{contents_font_size}px'
                if 'line-height' in child_inherited:
                    child_inherited['line-height'] = _computed_inherited_line_height(
                        child_inherited['line-height'], contents_font_size
                    )
            clipped_text_color = _background_text_color(effective_styles)
            if clipped_text_color:
                child_inherited['color'] = clipped_text_color
            if 'font' in effective_styles:
                family = _family_from_font_shorthand(effective_styles['font'])
                if family:
                    child_inherited['font-family'] = family
                child_inherited['line-height'] = _line_height_from_font_shorthand(
                    effective_styles['font']
                )
            contents_children = list(node.children)
            if (
                node.tag == 'optgroup'
                and node.attrs.get('label', '')
                and getattr(node, 'parent_tag', '') != 'select'
            ):
                # The native optgroup label survives when the host principal
                # box is `display:contents`. Represent its anonymous label
                # block explicitly so inherited font/color still apply.
                label_box = DomNode('span', {}, CssDeclarations())
                label_box.styles['display'] = 'block'
                label_box.styles['margin-top'] = '19px'
                label_box.styles['padding-left'] = '2px'
                label_box.styles['padding-bottom'] = '2px'
                label_box.styles['line-height'] = '20.1875px'
                label_box.styles['position'] = 'relative'
                label_box.styles['top'] = '-1px'
                label_text = DomNode('#text', {}, CssDeclarations())
                label_text.is_text = True
                label_text.text_content = node.attrs['label']
                label_box.children = [label_text]
                label_box.parent_tag = node.tag
                label_text.parent_tag = label_box.tag
                contents_children.insert(0, label_box)
            for child in contents_children:
                gen_node(
                    child,
                    parent_var,
                    indent,
                    contents_font_size,
                    child_inherited,
                    node_custom_props,
                    contents_zoom,
                    html_table_border,
                )
            return

        counter[0] += 1
        var = f"n{counter[0]}"
        ws = "    " * indent
        render_children = node.children
        image_source = node.attrs.get('src', '').strip() if node.tag == 'img' else ''
        failed_image_with_alt = bool(
            node.tag == 'img'
            and image_source
            and node.attrs.get('alt', '')
            and _node_resource(image_source) is None
        )
        if (
            node.tag == 'optgroup'
            and node.attrs.get('label', '')
            and getattr(node, 'parent_tag', '') != 'select'
            and node.styles.get('display', '').strip().lower() == 'contents'
        ):
            # The standalone optgroup's native anonymous label survives when
            # the principal box is `display:contents`. Keep it as a child of
            # the unboxed style boundary; layout flattens that boundary into
            # the surrounding formatting context.
            label_box = DomNode('span', {}, CssDeclarations())
            label_box.styles['display'] = 'block'
            label_box.styles['margin-top'] = '19px'
            label_box.styles['padding-left'] = '2px'
            label_box.styles['padding-bottom'] = '2px'
            label_box.styles['line-height'] = '20.1875px'
            label_box.styles['position'] = 'relative'
            label_box.styles['top'] = '-1px'
            label_text = DomNode('#text', {}, CssDeclarations())
            label_text.is_text = True
            label_text.text_content = node.attrs['label']
            label_box.children = [label_text]
            label_box.parent_tag = node.tag
            label_text.parent_tag = label_box.tag
            render_children = [label_box, *node.children]

        # Map HTML tag to ElementTag
        inline_tags = {'span', 'a', 'em', 'strong', 'b', 'i', 'u', 'small', 'big', 'sub', 'sup', 'abbr', 'cite', 'code', 'mark', 'q', 's', 'del', 'ins', 'var', 'kbd', 'samp', 'label'}
        semantic_tags = {
            'ruby': 'ElementTag::Ruby',
            'rt': 'ElementTag::RubyText',
            'br': 'ElementTag::Break',
            'wbr': 'ElementTag::WordBreak',
            'table': 'ElementTag::Table',
            'caption': 'ElementTag::TableCaption',
            'colgroup': 'ElementTag::TableColumnGroup',
            'col': 'ElementTag::TableColumn',
            'thead': 'ElementTag::TableHead',
            'tbody': 'ElementTag::TableBody',
            'tfoot': 'ElementTag::TableFoot',
            'tr': 'ElementTag::TableRow',
            'td': 'ElementTag::TableCell',
            'th': 'ElementTag::TableHeaderCell',
            'img': 'ElementTag::Image',
            'canvas': 'ElementTag::Canvas',
            'svg': 'ElementTag::Svg',
            'iframe': 'ElementTag::IFrame',
            'object': 'ElementTag::Object',
            'audio': 'ElementTag::Audio',
            'video': 'ElementTag::Video',
            'input': 'ElementTag::Input',
            'button': 'ElementTag::Button',
            'meter': 'ElementTag::Meter',
            'progress': 'ElementTag::Progress',
            'fieldset': 'ElementTag::Fieldset',
            'legend': 'ElementTag::Legend',
            'details': 'ElementTag::Details',
            'summary': 'ElementTag::Summary',
            'textarea': 'ElementTag::TextArea',
            'select': 'ElementTag::Select',
            'option': 'ElementTag::Option',
            'optgroup': 'ElementTag::OptGroup',
            'form': 'ElementTag::Form',
            'embed': 'ElementTag::Embed',
        }
        if node.tag in inline_tags or node.tag == 'style':
            element_tag = 'ElementTag::Span'
        else:
            element_tag = semantic_tags.get(node.tag, 'ElementTag::Div')
        lines.append(f"{ws}let {var} = doc.create_node({element_tag});")
        scroll_left = getattr(node, 'scroll_left', 0.0)
        scroll_top = getattr(node, 'scroll_top', 0.0)
        if scroll_left:
            lines.append(f"{ws}doc.node_mut({var}).scroll_left = {scroll_left};")
        if scroll_top:
            lines.append(f"{ws}doc.node_mut({var}).scroll_top = {scroll_top};")
        if (
            RETAIN_TEXT
            and not is_real_font_profile()
            and not isolated_document
        ):
            lines.append(
                f'{ws}doc.node_mut({var}).style.font_family = '
                f'{DETERMINISTIC_FONT_FAMILY_RUST};'
            )
            # TEXT_TEMPLATE_OVERRIDE applies `list-style:none !important` to
            # `body, body *`. Generated styles do not inherit implicitly, so
            # mirror that descendant-wide override on every emitted element.
            lines.append(
                f"{ws}doc.node_mut({var}).style.list_style_type = ListStyleType::None;"
            )
        if node.tag == 'br':
            lines.append(f"{ws}doc.node_mut({var}).style.display = Display::Inline;")
        elif node.tag == 'foreignobject':
            # SVG foreignObject is an atomic graphics element; CSS width and
            # height apply even though its HTML fallback tag maps to Div in
            # the compact DOM vocabulary. Within its SVG viewport it starts a
            # block formatting context at the graphics origin rather than
            # aligning to an HTML inline baseline.
            lines.append(f"{ws}doc.node_mut({var}).style.display = Display::Block;")
            lines.append(f"{ws}doc.node_mut({var}).is_svg_foreign_object = true;")

        if node.tag == 'svg':
            # An outermost inline SVG viewport is a replaced element with the
            # HTML default object size when neither dimension is authored.
            # Width/height presentation attributes accept both CSS lengths
            # and percentages; CSS declarations emitted below remain
            # authoritative and can override them through normal cascade.
            def svg_viewport_length(name, default):
                token = node.attrs.get(name)
                if token is None:
                    # Missing SVG viewport dimensions remain `auto`. Replaced
                    # sizing resolves that value from the available space,
                    # intrinsic dimensions, and viewBox ratio; materializing a
                    # percentage here changes flex base-size semantics.
                    if node.attrs.get('viewbox'):
                        if (
                            name == 'width'
                            and 'height' not in node.styles
                            and 'max-height' not in node.styles
                            and 'height' not in node.attrs
                        ):
                            # An outer SVG with only a viewBox uses stretch-fit
                            # inline sizing. Keep this as the sizing keyword so
                            # flex main-axis sizing can subtract item margins;
                            # `100%` would incorrectly overflow that margin box.
                            return 'Length::stretch()'
                        return None
                    # The 300x150 fallback is an intrinsic replaced size, not
                    # a specified width/height. Child-bearing SVG viewports
                    # keep their descendants in the formatting tree, so leave
                    # these axes auto and let flex/block sizing resolve them.
                    return None
                token = token.strip()
                if re.fullmatch(r'[-+]?(?:\d+(?:\.\d*)?|\.\d+)', token):
                    return f'Length::px({_zoomed_px(float(token))})'
                return parse_length(token, parent_font_size)

            svg_width = svg_viewport_length('width', 300.0)
            svg_height = svg_viewport_length('height', 150.0)
            if svg_width:
                lines.append(f"{ws}doc.node_mut({var}).style.width = {svg_width};")
            if svg_height:
                lines.append(f"{ws}doc.node_mut({var}).style.height = {svg_height};")
            if node.children:
                lines.append(f"{ws}doc.node_mut({var}).style.position = Position::Relative;")

        if node.tag == 'text':
            # SVG text uses its x/y presentation coordinates, where y is the
            # typographic baseline. Lower the graphics leaf to an absolutely
            # positioned shrink-to-fit text box inside its SVG viewport.
            def svg_text_number(name, default):
                token = node.attrs.get(name, str(default)).strip()
                return float(token) if re.fullmatch(
                    r'[-+]?(?:\d+(?:\.\d*)?|\.\d+)', token
                ) else float(default)

            text_x = _zoomed_px(svg_text_number('x', 0.0))
            text_y = _zoomed_px(svg_text_number('y', 0.0) - 13.0)
            lines.append(f"{ws}doc.node_mut({var}).style.display = Display::Block;")
            lines.append(f"{ws}doc.node_mut({var}).style.position = Position::Absolute;")
            lines.append(f"{ws}doc.node_mut({var}).style.left = Length::px({text_x});")
            lines.append(f"{ws}doc.node_mut({var}).style.top = Length::px({text_y});")

        if node.tag == 'g':
            # SVG container elements do not generate CSS layout boxes. Keep
            # their presentation color available to vector descendants while
            # letting those descendants position against the SVG viewport.
            lines.append(f"{ws}doc.node_mut({var}).style.display = Display::Contents;")
            svg_fill = parse_color(node.attrs.get('fill', 'black'))
            if svg_fill:
                lines.append(f"{ws}doc.node_mut({var}).style.color = {svg_fill};")

        if node.tag == 'path':
            # Lower deterministic rectilinear SVG paths to vector-painted
            # boxes. This covers the common WPT reference idiom that spells a
            # rectangle as M/H/V/h/v/z while preserving inherited SVG fill.
            tokens = re.findall(
                r'[MmHhVvZz]|[-+]?(?:\d+(?:\.\d*)?|\.\d+)',
                node.attrs.get('d', ''),
            )
            cursor = 0
            command = None
            x = y = 0.0
            points = []
            valid = True
            while cursor < len(tokens):
                if re.fullmatch(r'[MmHhVvZz]', tokens[cursor]):
                    command = tokens[cursor]
                    cursor += 1
                    if command in 'Zz':
                        break
                if command in ('M', 'm'):
                    if cursor + 1 >= len(tokens):
                        break
                    nx, ny = float(tokens[cursor]), float(tokens[cursor + 1])
                    cursor += 2
                    if command == 'm':
                        x += nx
                        y += ny
                    else:
                        x, y = nx, ny
                    points.append((x, y))
                    command = 'l' if command == 'm' else 'L'
                elif command in ('H', 'h'):
                    if cursor >= len(tokens):
                        break
                    value = float(tokens[cursor])
                    cursor += 1
                    x = x + value if command == 'h' else value
                    points.append((x, y))
                elif command in ('V', 'v'):
                    if cursor >= len(tokens):
                        break
                    value = float(tokens[cursor])
                    cursor += 1
                    y = y + value if command == 'v' else value
                    points.append((x, y))
                else:
                    valid = False
                    break
            if valid and len(points) >= 3:
                min_x = min(point[0] for point in points)
                max_x = max(point[0] for point in points)
                min_y = min(point[1] for point in points)
                max_y = max(point[1] for point in points)
                if max_x > min_x and max_y > min_y:
                    lines.extend([
                        f"{ws}doc.node_mut({var}).style.display = Display::Block;",
                        f"{ws}doc.node_mut({var}).style.position = Position::Absolute;",
                        f"{ws}doc.node_mut({var}).style.left = Length::px({_zoomed_px(min_x)});",
                        f"{ws}doc.node_mut({var}).style.top = Length::px({_zoomed_px(min_y)});",
                        f"{ws}doc.node_mut({var}).style.width = Length::px({_zoomed_px(max_x - min_x)});",
                        f"{ws}doc.node_mut({var}).style.height = Length::px({_zoomed_px(max_y - min_y)});",
                    ])
                    explicit_fill = parse_color(node.attrs.get('fill', ''))
                    if explicit_fill:
                        lines.append(
                            f"{ws}doc.node_mut({var}).style.background_color = {explicit_fill};"
                        )
                    else:
                        lines.append(
                            f"{ws}doc.node_mut({var}).style.background_color = "
                            f"doc.node({parent_var}).style.color;"
                        )

        if node.tag == 'rect':
            # Inline SVG rectangles are deterministic vector primitives, not
            # HTML flow boxes. Represent the primitive as an absolutely
            # positioned painted box inside the SVG viewport. Layout/paint
            # containment on the SVG element then supplies the correct
            # containing block and overflow-clip-margin behavior.
            def svg_number(name, default):
                token = node.attrs.get(name, str(default)).strip()
                match = re.fullmatch(r'[-+]?(?:\d+(?:\.\d*)?|\.\d+)', token)
                return float(token) if match else float(default)

            svg_x = _zoomed_px(svg_number('x', 0.0))
            svg_y = _zoomed_px(svg_number('y', 0.0))
            svg_width = max(0.0, _zoomed_px(svg_number('width', 0.0)))
            svg_height = max(0.0, _zoomed_px(svg_number('height', 0.0)))
            lines.append(f"{ws}doc.node_mut({var}).style.display = Display::Block;")
            lines.append(f"{ws}doc.node_mut({var}).style.position = Position::Absolute;")
            lines.append(f"{ws}doc.node_mut({var}).style.left = Length::px({svg_x});")
            lines.append(f"{ws}doc.node_mut({var}).style.top = Length::px({svg_y});")
            lines.append(f"{ws}doc.node_mut({var}).style.width = Length::px({svg_width});")
            lines.append(f"{ws}doc.node_mut({var}).style.height = Length::px({svg_height});")
            svg_fill = parse_color(node.attrs.get('fill', 'black'))
            if svg_fill:
                lines.append(
                    f"{ws}doc.node_mut({var}).style.background_color = {svg_fill};"
                )
            svg_transform = node.attrs.get('transform', '').strip()
            if svg_transform:
                transform = _transform_2d_rust(svg_transform, parent_font_size)
                if transform:
                    lines.append(f"{ws}doc.node_mut({var}).style.transform = {transform};")
                    lines.append(
                        f"{ws}doc.node_mut({var}).style.transform_origin = "
                        "(Length::px(0.0), Length::px(0.0));"
                    )
                    lines.append(
                        f"{ws}doc.node_mut({var}).style."
                        "establishes_transform_containing_block = true;"
                    )

        table_displays = {
            'table': 'Display::Table',
            'caption': 'Display::TableCaption',
            'colgroup': 'Display::TableColumnGroup',
            'col': 'Display::TableColumn',
            'thead': 'Display::TableHeaderGroup',
            'tbody': 'Display::TableRowGroup',
            'tfoot': 'Display::TableFooterGroup',
            'tr': 'Display::TableRow',
            'td': 'Display::TableCell',
            'th': 'Display::TableCell',
        }
        if node.tag in table_displays:
            lines.append(
                f"{ws}doc.node_mut({var}).style.display = {table_displays[node.tag]};"
            )
            if node.tag == 'table':
                # HTML's UA stylesheet supplies 2px spacing for semantic
                # tables. CSS `display:table` boxes retain the 0px initial.
                try:
                    legacy_spacing = max(0.0, float(node.attrs.get('cellspacing', '2')))
                except ValueError:
                    legacy_spacing = 2.0
                lines.append(
                    f"{ws}doc.node_mut({var}).style.border_spacing = "
                    f"(Length::px({legacy_spacing}), Length::px({legacy_spacing}));"
                )
            elif node.tag == 'caption':
                # Chromium's semantic HTML caption rule centers inline
                # content. A CSS `display:table-caption` box retains the CSS
                # initial alignment, so keep this as an HTML-only UA default.
                lines.append(
                    f"{ws}doc.node_mut({var}).style.text_align = TextAlign::Center;"
                )
            elif node.tag == 'th':
                # HTML header cells are centered by the UA stylesheet. Keep
                # this as an HTML-only default; author declarations emitted
                # below retain normal cascade precedence.
                lines.append(
                    f"{ws}doc.node_mut({var}).style.text_align = TextAlign::Center;"
                )
            if node.tag in {'thead', 'tbody', 'tfoot', 'tr', 'td', 'th'}:
                # Chromium's HTML UA rules establish middle alignment on row
                # groups and pass it through rows to cells. Emit the resolved
                # default directly because the compact DOM stores computed
                # values rather than UA declarations such as `inherit`.
                lines.append(
                    f"{ws}doc.node_mut({var}).style.vertical_align = "
                    "VerticalAlign::Middle;"
                )
        elif node.tag in {
            'img', 'canvas', 'svg', 'iframe', 'object', 'audio', 'video',
            'input', 'button', 'meter', 'progress', 'textarea', 'select', 'embed',
            'marquee',
        }:
            # A failed image with alternative text generates an ordinary
            # inline fallback when the author did not override `display`.
            # Its width/height presentation hints consequently do not size an
            # atomic replaced box; the fallback wraps in the containing line.
            if not (failed_image_with_alt and 'display' not in node.styles):
                lines.append(f"{ws}doc.node_mut({var}).style.display = Display::InlineBlock;")

        if node.tag == 'marquee':
            # The legacy scrolling host clips its anonymous scrolling box.
            # Static startup lowering keeps the initial layout state, so the
            # clip remains observable even though no animation is executed.
            lines.append(f"{ws}doc.node_mut({var}).style.overflow_x = Overflow::Hidden;")
            lines.append(f"{ws}doc.node_mut({var}).style.overflow_y = Overflow::Hidden;")

        if node.tag in {'img', 'iframe', 'video', 'embed', 'svg'}:
            # HTML's replaced-element UA rules use an `overflow: clip`
            # content-box edge. Author declarations are emitted below and
            # therefore retain normal precedence over this default.
            ua_overflow = 'Hidden' if node.tag == 'svg' else 'Clip'
            lines.append(
                f"{ws}doc.node_mut({var}).style.overflow_x = Overflow::{ua_overflow};"
            )
            lines.append(
                f"{ws}doc.node_mut({var}).style.overflow_y = Overflow::{ua_overflow};"
            )
            lines.append(
                f"{ws}doc.node_mut({var}).style.overflow_clip_box = "
                "OverflowClipBox::ContentBox;"
            )

        if node.tag in ('td', 'th'):
            try:
                col_span = max(1, int(node.attrs.get('colspan', '1')))
            except ValueError:
                col_span = 1
            try:
                row_span = max(0, int(node.attrs.get('rowspan', '1')))
            except ValueError:
                row_span = 1
            lines.append(f"{ws}doc.node_mut({var}).table_col_span = {col_span};")
            lines.append(f"{ws}doc.node_mut({var}).table_row_span = {row_span};")
        elif node.tag in ('col', 'colgroup'):
            try:
                col_span = max(1, int(node.attrs.get('span', '1')))
            except ValueError:
                col_span = 1
            lines.append(f"{ws}doc.node_mut({var}).table_col_span = {col_span};")

        control_roles = {
            'button': 'openui_dom::FormControlRole::Button',
            'textarea': 'openui_dom::FormControlRole::TextArea',
            'select': 'openui_dom::FormControlRole::Select',
            'option': 'openui_dom::FormControlRole::Option',
            'optgroup': 'openui_dom::FormControlRole::OptGroup',
            'meter': 'openui_dom::FormControlRole::Meter',
            'progress': 'openui_dom::FormControlRole::Progress',
            'fieldset': 'openui_dom::FormControlRole::Fieldset',
            'legend': 'openui_dom::FormControlRole::Legend',
        }
        if node.tag == 'input':
            input_type = node.attrs.get('type', 'text').lower()
            role = (
                'openui_dom::FormControlRole::Range'
                if input_type == 'range'
                else 'openui_dom::FormControlRole::ColorInput'
                if input_type == 'color'
                else 'openui_dom::FormControlRole::DateInput'
                if input_type == 'date'
                else 'openui_dom::FormControlRole::FileInput'
                if input_type == 'file'
                else 'openui_dom::FormControlRole::Checkbox'
                if input_type == 'checkbox'
                else 'openui_dom::FormControlRole::Radio'
                if input_type == 'radio'
                else 'openui_dom::FormControlRole::Button'
                if input_type in ('button', 'submit', 'reset')
                else 'openui_dom::FormControlRole::TextInput'
            )
            lines.append(f"{ws}doc.node_mut({var}).form_control = Some({role});")
        elif node.tag in control_roles:
            lines.append(
                f"{ws}doc.node_mut({var}).form_control = Some({control_roles[node.tag]});"
            )
        if node.tag in control_roles or node.tag == 'input':
            lines.append(
                f"{ws}doc.node_mut({var}).form_control_disabled = "
                f"{'true' if 'disabled' in node.attrs else 'false'};"
            )
        if node.tag == 'input' and node.attrs.get('type', 'text').lower() == 'file':
            # The file-selector shadow button uses the platform small-control
            # font even though the filename label inherits the host font.
            lines.append(f"{ws}doc.node_mut({var}).style.native_control_text = true;")

        control_all_value = node.styles.get('all', '').strip().lower()
        control_all_reset = control_all_value in {'initial', 'unset'}
        if node.tag in {'input', 'textarea', 'select'}:
            # Pinned Linux Chromium UA appearance. Intrinsic control metrics
            # are carried by ReplacedContent below, leaving computed width
            # and height `auto` so flex/grid stretch can participate.
            appearance_none = any(
                node.styles.get(prop, '').strip().lower() == 'none'
                for prop in ('appearance', '-webkit-appearance', '-moz-appearance')
            )
            lines.append(
                f"{ws}doc.node_mut({var}).style.box_sizing = "
                f"BoxSizing::{'ContentBox' if control_all_reset else 'BorderBox'};"
            )
            if node.tag == 'textarea':
                lines.append(f"{ws}doc.node_mut({var}).style.overflow_x = Overflow::Auto;")
                lines.append(f"{ws}doc.node_mut({var}).style.overflow_y = Overflow::Auto;")
            input_type = node.attrs.get('type', 'text').lower() if node.tag == 'input' else ''
            ua_border_width = 0 if control_all_reset else (
                0 if input_type in {'checkbox', 'radio', 'file'}
                or (input_type == 'range' and not appearance_none)
                else 2 if node.tag == 'input'
                else 1
            )
            for side in ('top', 'right', 'bottom', 'left'):
                lines.append(
                    f"{ws}doc.node_mut({var}).style.border_{side}_width = "
                    f"{ua_border_width};"
                )
                lines.append(
                    f"{ws}doc.node_mut({var}).style.border_{side}_style = "
                    f"BorderStyle::{'None' if control_all_reset else 'Inset'};"
                )
                if node.tag == 'select' and appearance_none:
                    lines.append(
                        f"{ws}doc.node_mut({var}).style.border_{side}_style = "
                        "BorderStyle::Solid;"
                    )
                    lines.append(
                        f"{ws}doc.node_mut({var}).style.border_{side}_color = "
                        "StyleColor::Resolved(Color::from_rgba8(118, 118, 118, 255));"
                    )
            if node.tag == 'select' and appearance_none:
                for corner in (
                    'top_left', 'top_right', 'bottom_right', 'bottom_left'
                ):
                    lines.append(
                        f"{ws}doc.node_mut({var}).style.border_{corner}_radius = (2.0, 2.0);"
                    )

        if node.tag == 'fieldset':
            # Chromium's HTML UA rule supplies a 2px groove border. The
            # comparison harness resets margin and padding, but deliberately
            # leaves that native fieldset border intact. Emit it before the
            # author declarations below so normal cascade overrides (such as
            # `border: none`) retain their precedence.
            for side in ('top', 'right', 'bottom', 'left'):
                lines.append(
                    f"{ws}doc.node_mut({var}).style.border_{side}_width = 2;"
                )
                lines.append(
                    f"{ws}doc.node_mut({var}).style.border_{side}_style = "
                    "BorderStyle::Groove;"
                )

        if node.tag in {'img', 'embed', 'object'}:
            attribute = 'data' if node.tag == 'object' else 'src'
            source = node.attrs.get(attribute, '').strip()
            resource = _node_resource(source) if source else None
            if resource is not None:
                source_label, mime, sha, dimensions, intrinsic_ratio, byte_expr = resource
                intrinsic_width, intrinsic_height = dimensions or (None, None)
                intrinsic_width_expr = (
                    f'Some({intrinsic_width})' if intrinsic_width is not None else 'None'
                )
                intrinsic_height_expr = (
                    f'Some({intrinsic_height})' if intrinsic_height is not None else 'None'
                )
                resource_var = f'{var}_image'
                kind = (
                    'openui_dom::ReplacedResourceKind::StaticSvg'
                    if mime == 'image/svg+xml'
                    else 'openui_dom::ReplacedResourceKind::Image'
                )
                lines.append(
                    f'{ws}let {resource_var} = doc.register_image_resource('
                    f'{json.dumps(source_label)}, {json.dumps(mime)}, {json.dumps(sha)}, '
                    f'{byte_expr});'
                )
                lines.append(
                    f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                    f"resource: {kind}({resource_var}), "
                    f"intrinsic_width: {intrinsic_width_expr}, "
                    f"intrinsic_height: {intrinsic_height_expr}, "
                    "intrinsic_ratio: "
                    f"{'Some((' + str(intrinsic_ratio[0]) + ', ' + str(intrinsic_ratio[1]) + '))' if intrinsic_ratio else 'None'} }});"
                )
                if (
                    node.tag == 'img'
                    and mime == 'image/svg+xml'
                    and dimensions is None
                    and intrinsic_ratio is not None
                    and 'display' not in node.styles
                ):
                    # A ratio-only SVG image uses the containing block's
                    # stretch-fit opportunity. Represent its anonymous atomic
                    # viewport as a block so its margins and border are
                    # removed exactly once and its containing inline-block
                    # exports the same synthesized baseline as Blink.
                    lines.append(f"{ws}doc.node_mut({var}).style.display = Display::Block;")
            elif node.tag == 'embed' or (node.tag == 'object' and not node.children):
                # An object without a usable data resource renders its fallback
                # descendants. Only an actually empty object has the default
                # 300x150 replaced-element dimensions.
                lines.append(
                    f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                    "resource: openui_dom::ReplacedResourceKind::TransparentCanvas, "
                    "intrinsic_width: Some(300.0), intrinsic_height: Some(150.0), "
                    "intrinsic_ratio: None });"
                )
            elif (
                RETAIN_TEXT
                and node.tag == 'img'
                and source
                and node.attrs.get('alt', '')
            ):
                # A failed image with alternative text is non-replaced.
                # Materialize its deterministic UA shadow fallback as a 16px
                # icon slot followed by the alternative text. Ordinary inline
                # or block-container layout (according to the computed
                # display) owns wrapping, height, padding, and overflow.
                icon = DomNode('canvas', {'width': '16', 'height': '16'}, CssDeclarations())
                # Blink's anonymous broken-resource icon sits three pixels
                # below the alternative text baseline at the pinned 16px UA
                # font size; expose that baseline through ordinary inline
                # vertical alignment rather than enlarging the first line.
                icon.styles['vertical-align'] = '-3px'
                text_node = DomNode('#text', {}, CssDeclarations())
                text_node.is_text = True
                text_node.text_content = node.attrs['alt']
                render_children = [icon, text_node]
        elif node.tag == 'video':
            poster = node.attrs.get('poster', '').strip()
            resource = _node_resource(poster) if poster else None
            if resource is not None:
                source_label, mime, sha, dimensions, intrinsic_ratio, byte_expr = resource
                intrinsic_width, intrinsic_height = dimensions or (300.0, 150.0)
                lines.append(
                    f'{ws}let {var}_poster = doc.register_image_resource('
                    f'{json.dumps(source_label)}, {json.dumps(mime)}, {json.dumps(sha)}, {byte_expr});'
                )
                lines.append(
                    f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                    f"resource: openui_dom::ReplacedResourceKind::MediaPoster({var}_poster), "
                    f"intrinsic_width: Some({intrinsic_width}), intrinsic_height: Some({intrinsic_height}), "
                    "intrinsic_ratio: "
                    f"{'Some((' + str(intrinsic_ratio[0]) + ', ' + str(intrinsic_ratio[1]) + '))' if intrinsic_ratio else 'None'} }});"
                )
            else:
                source = node.attrs.get('src', '').strip()
                local = _packaged_resource(source) if source else None
                dimensions = local[4] if local is not None else None
                intrinsic_width, intrinsic_height = dimensions or (300.0, 150.0)
                ratio = (
                    f'Some(({intrinsic_width}, {intrinsic_height}))'
                    if dimensions is not None else 'None'
                )
                decoded_frame = (
                    MEDIA_FIRST_FRAMES.get(local[3]) if local is not None else None
                )
                if decoded_frame is not None:
                    transport = bytes.fromhex(decoded_frame['transport_hex'])
                    byte_expr = 'vec![' + ', '.join(
                        f'0x{byte:02x}' for byte in transport
                    ) + ']'
                    lines.append(
                        f'{ws}let {var}_frame = doc.register_image_resource('
                        f'{json.dumps(local[1] + "#first-frame")}, '
                        f'{json.dumps(decoded_frame["output_mime_type"])}, '
                        f'{json.dumps(decoded_frame["output_sha256"])}, {byte_expr});'
                    )
                    lines.append(
                        f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                        f"resource: openui_dom::ReplacedResourceKind::MediaPoster({var}_frame), "
                        f"intrinsic_width: Some({intrinsic_width}), intrinsic_height: Some({intrinsic_height}), "
                        f"intrinsic_ratio: {ratio} }});"
                    )
                    # The HTML video UA presentation contains its intrinsic
                    # frame inside the concrete element viewport. Author
                    # object-fit declarations are emitted later and override
                    # this default through the normal cascade order.
                    lines.append(
                        f"{ws}doc.node_mut({var}).style.object_fit = ObjectFit::Contain;"
                    )
                else:
                    lines.append(
                        f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                        "resource: openui_dom::ReplacedResourceKind::TransparentCanvas, "
                        f"intrinsic_width: Some({intrinsic_width}), intrinsic_height: Some({intrinsic_height}), "
                        f"intrinsic_ratio: {ratio} }});"
                    )
        elif node.tag == 'svg':
            # An outer SVG viewport is atomic replaced content. Child-bearing
            # roots remain represented by the same viewport fragment; simple
            # full-viewport rectangles are lowered to its background paint.
            # Preserve the HTML/SVG default object size and viewBox ratio so
            # CSS sizing and flex layout see the native replaced contract.
            intrinsic_width = _svg_number(node.attrs.get('width'))
            intrinsic_height = _svg_number(node.attrs.get('height'))
            svg_bounds_inferred = False
            visual_children = [
                child for child in node.children
                if not child.is_text or (child.text_content or '').strip()
            ]
            if (
                intrinsic_width is None
                and intrinsic_height is None
                and not node.attrs.get('viewbox')
                and len(visual_children) == 1
                and visual_children[0].tag == 'rect'
            ):
                # A child-bearing outer SVG whose percentage viewport is
                # cyclic under max-content sizing exposes the numeric graphics
                # bounds as its natural dimensions. This is distinct from an
                # empty SVG, which keeps the HTML 300x150 default object size.
                rect = visual_children[0]
                rect_width = _svg_number(rect.attrs.get('width'))
                rect_height = _svg_number(rect.attrs.get('height'))
                rect_x = _svg_number(rect.attrs.get('x')) or 0.0
                rect_y = _svg_number(rect.attrs.get('y')) or 0.0
                if (
                    rect_x == 0.0
                    and rect_y == 0.0
                    and rect_width is not None
                    and rect_height is not None
                    and rect_width > 0.0
                    and rect_height > 0.0
                ):
                    intrinsic_width = rect_width
                    intrinsic_height = rect_height
                    svg_bounds_inferred = True
            viewbox_ratio = None
            viewbox = node.attrs.get('viewbox', '').replace(',', ' ').split()
            if len(viewbox) == 4:
                try:
                    viewbox_width = float(viewbox[2])
                    viewbox_height = float(viewbox[3])
                    if viewbox_width > 0.0 and viewbox_height > 0.0:
                        viewbox_ratio = (viewbox_width, viewbox_height)
                except ValueError:
                    pass
            intrinsic_ratio = viewbox_ratio
            if (
                intrinsic_ratio is None
                and not svg_bounds_inferred
                and intrinsic_width is not None
                and intrinsic_height is not None
                and intrinsic_width > 0.0
                and intrinsic_height > 0.0
            ):
                # An outer SVG with definite width/height attributes has a
                # natural aspect ratio even without a viewBox. CSS can replace
                # one preferred axis while max-sizing the other, so retaining
                # only the two natural dimensions is insufficient.
                intrinsic_ratio = (intrinsic_width, intrinsic_height)
            ratio_rust = (
                f"Some(({intrinsic_ratio[0]}, {intrinsic_ratio[1]}))"
                if intrinsic_ratio is not None
                else "None"
            )
            intrinsic_width_rust = (
                f"Some({intrinsic_width})" if intrinsic_width is not None else "None"
            )
            intrinsic_height_rust = (
                f"Some({intrinsic_height})" if intrinsic_height is not None else "None"
            )
            if visual_children:
                # Keep vector descendants in the compact formatting tree so
                # they can paint through mixed-axis overflow and
                # overflow-clip-margin. The SVG viewport itself establishes
                # their positioning context; authored position declarations
                # emitted below retain precedence.
                lines.append(
                    f"{ws}doc.node_mut({var}).style.position = Position::Relative;"
                )
                if (
                    intrinsic_width is None
                    and 'width' not in node.styles
                    and 'max-width' not in node.styles
                ):
                    lines.append(f"{ws}doc.node_mut({var}).style.width = Length::px(300.0);")
                if (
                    intrinsic_height is None
                    and 'height' not in node.styles
                    and 'max-height' not in node.styles
                ):
                    lines.append(f"{ws}doc.node_mut({var}).style.height = Length::px(150.0);")
            else:
                lines.append(
                    f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                    "resource: openui_dom::ReplacedResourceKind::TransparentCanvas, "
                    f"intrinsic_width: {intrinsic_width_rust}, "
                    f"intrinsic_height: {intrinsic_height_rust}, "
                    f"intrinsic_ratio: {ratio_rust} }});"
                )
            if (
                viewbox_ratio is not None
                and 'width' not in node.attrs
                and 'height' not in node.attrs
                and 'height' not in node.styles
                and 'max-height' not in node.styles
            ):
                # A ratio-only outer SVG participates as a stretch-fit block
                # in HTML flow. Flex layout blockifies the same atomic item.
                lines.append(f"{ws}doc.node_mut({var}).style.display = Display::Block;")
            if len(visual_children) == 1 and visual_children[0].tag == 'rect':
                rect = visual_children[0]
                viewbox_width = (
                    viewbox_ratio[0] if viewbox_ratio else (intrinsic_width or 300.0)
                )
                viewbox_height = (
                    viewbox_ratio[1] if viewbox_ratio else (intrinsic_height or 150.0)
                )

                def covers_svg_axis(name, expected):
                    token = rect.attrs.get(name, '').strip()
                    if token == '100%':
                        return True
                    try:
                        return float(token) == expected
                    except ValueError:
                        return False

                x = _svg_number(rect.attrs.get('x')) or 0.0
                y = _svg_number(rect.attrs.get('y')) or 0.0
                if (
                    x == 0.0 and y == 0.0
                    and covers_svg_axis('width', viewbox_width)
                    and covers_svg_axis('height', viewbox_height)
                ):
                    fill = parse_color(rect.attrs.get('fill', 'black'))
                    if fill:
                        lines.append(
                            f"{ws}doc.node_mut({var}).style.background_color = {fill};"
                        )
        elif node.tag == 'canvas':
            try:
                intrinsic_width = max(0, int(node.attrs.get('width', '300')))
                intrinsic_height = max(0, int(node.attrs.get('height', '150')))
            except ValueError:
                intrinsic_width, intrinsic_height = 300, 150
            lines.append(
                f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                "resource: openui_dom::ReplacedResourceKind::TransparentCanvas, "
                f"intrinsic_width: Some({float(intrinsic_width)}), "
                f"intrinsic_height: Some({float(intrinsic_height)}), "
                "intrinsic_ratio: "
                f"Some(({float(intrinsic_width)}, {float(intrinsic_height)})) }});"
            )
        elif node.tag == 'iframe':
            # HTML iframe elements have a 300x150 default object size but no
            # natural aspect ratio. Package srcdoc/local documents so their
            # source identity never depends on a live browsing context. A
            # deterministic srcdoc is additionally lowered into an isolated
            # child viewport below; scripts and handlers remain rejected.
            srcdoc = node.attrs.get('srcdoc')
            source = node.attrs.get('src', '').strip()
            embedded_parser = None
            packaged_document_context = bool(
                getattr(node, 'packaged_document_context', False)
            )
            embedded_resource_base = Path(
                getattr(node, 'resource_base', _ACTIVE_RESOURCE_BASE or Path.cwd())
            )
            if srcdoc is not None:
                data = srcdoc.encode('utf-8')
                document_source = f'inline:srcdoc;sha256={hashlib.sha256(data).hexdigest()}'
                document_mime = 'text/html'
                document_sha = hashlib.sha256(data).hexdigest()
                document_bytes = f'vec![{", ".join(str(byte) for byte in data)}]'
                embedded_markup = srcdoc
                style_starts = list(re.finditer(
                    r'<style\b[^>]*>', embedded_markup, re.IGNORECASE
                ))
                style_ends = list(re.finditer(
                    r'</style\s*>', embedded_markup, re.IGNORECASE
                ))
                if style_starts and not style_ends:
                    # HTML raw-text recovery treats a second `<style>` in
                    # legacy srcdoc fixtures as the intended closing token.
                    # Normalize that deterministic typo for the compact DOM
                    # parser while retaining the authored bytes above.
                    if len(style_starts) > 1:
                        closing = style_starts[-1]
                        embedded_markup = (
                            embedded_markup[:closing.start()]
                            + '</style>'
                            + embedded_markup[closing.end():]
                        )
                    else:
                        embedded_markup += '</style>'
                embedded_parser = _parse_wpt_markup(
                    embedded_markup,
                    str(embedded_resource_base),
                    root_aware=True,
                    harness_rules=EMBEDDED_DOCUMENT_RULES,
                )
                embedded_portable, embedded_reason = analyze_portability(embedded_parser)
                if not embedded_portable:
                    raise ValueError(
                        f'unsupported embedded srcdoc behavior: {embedded_reason}'
                    )
                embedded_root = embedded_parser.root
                embedded_root.tag = 'div'

                def mark_isolated_document(embedded_node):
                    embedded_node.isolated_document = True
                    for embedded_child in embedded_node.children:
                        mark_isolated_document(embedded_child)

                mark_isolated_document(embedded_root)
                # Keep the nested document as real child layout while giving
                # its atomic iframe box the HTML default object size. A
                # packaged document without lowered children gets this size
                # from ReplacedContent instead; deterministic srcdoc needs the
                # equivalent used dimensions on its containing viewport.
                lines.append(f"{ws}doc.node_mut({var}).style.width = Length::px(300.0);")
                lines.append(f"{ws}doc.node_mut({var}).style.height = Length::px(150.0);")
                # The body of a nested browsing context forms the embedded
                # canvas boundary. Descendant margins do not collapse through
                # that boundary into the iframe's own inline box.
                if embedded_root.styles.get('display', '').strip() in ('', 'block'):
                    embedded_root.styles['display'] = 'flow-root'
                leading_margin = _embedded_multicol_leading_margin(embedded_root)
                if leading_margin:
                    margin_boundary = DomNode(
                        'div',
                        {'data-openui-embedded-margin-boundary': ''},
                        CssDeclarations(),
                    )
                    margin_boundary.styles['display'] = 'block'
                    margin_boundary.styles['height'] = f'{leading_margin}px'
                    embedded_root.children.insert(0, margin_boundary)
                render_children = [embedded_root]
                # A nested browsing context owns an isolated viewport: its
                # positioned descendants resolve within the iframe and its
                # canvas is clipped to the replaced content edge.
                lines.append(f"{ws}doc.node_mut({var}).style.position = Position::Relative;")
                embedded_canvas_color = (
                    embedded_parser.html_styles.get('background-color')
                    or embedded_root.styles.get('background-color')
                )
                if embedded_canvas_color:
                    canvas_color = parse_color(embedded_canvas_color)
                    if canvas_color:
                        lines.append(
                            f"{ws}doc.node_mut({var}).embedded_canvas_color = "
                            f"Some({canvas_color});"
                        )
            else:
                local_document = (
                    _resolve_local_resource(source, embedded_resource_base)
                    if source and not packaged_document_context else None
                )
                packaged_document = (
                    _packaged_resource(source, embedded_resource_base)
                    if source and not packaged_document_context else None
                )
                if packaged_document is not None and packaged_document[2] in ('text/html', 'application/xhtml+xml'):
                    filename, document_source, document_mime, document_sha, _ = packaged_document
                    document_bytes = _packaged_bytes_expr(filename)
                    if local_document is not None:
                        embedded_markup = local_document.read_text(
                            encoding='utf-8-sig', errors='replace'
                        )
                        embedded_parser = _parse_wpt_markup(
                            embedded_markup,
                            str(local_document.parent),
                            root_aware=True,
                            harness_rules=EMBEDDED_DOCUMENT_RULES,
                        )
                        embedded_portable, embedded_reason = analyze_portability(
                            embedded_parser
                        )
                        if not embedded_portable:
                            raise ValueError(
                                'unsupported packaged document behavior: '
                                f'{embedded_reason}'
                            )
                        embedded_root = embedded_parser.root
                        embedded_root.tag = 'div'
                        if embedded_root.styles.get('display', '').strip() in ('', 'block'):
                            embedded_root.styles['display'] = 'flow-root'

                        def attach_resource_base(embedded_node):
                            embedded_node.resource_base = local_document.parent
                            embedded_node.isolated_document = True
                            embedded_node.packaged_document_context = True
                            for embedded_child in embedded_node.children:
                                attach_resource_base(embedded_child)

                        attach_resource_base(embedded_root)
                        render_children = [embedded_root]
                        lines.append(
                            f"{ws}doc.node_mut({var}).style.width = Length::px(300.0);"
                        )
                        lines.append(
                            f"{ws}doc.node_mut({var}).style.height = Length::px(150.0);"
                        )
                        lines.append(
                            f"{ws}doc.node_mut({var}).style.position = Position::Relative;"
                        )
                else:
                    document_source = 'about:blank'
                    document_mime = 'text/html'
                    document_sha = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
                    document_bytes = 'Vec::new()'
            lines.append(
                f'{ws}let {var}_document = doc.register_image_resource('
                f'{json.dumps(document_source)}, {json.dumps(document_mime)}, '
                f'{json.dumps(document_sha)}, {document_bytes});'
            )
            lines.append(f"{ws}doc.node_mut({var}).embedded_document = Some({var}_document);")
            if embedded_parser is None:
                lines.append(
                    f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                    f"resource: openui_dom::ReplacedResourceKind::PackagedDocument({var}_document), "
                    "intrinsic_width: Some(300.0), intrinsic_height: Some(150.0), "
                    "intrinsic_ratio: None });"
                )
            # Blink's iframe UA rule is a 2px inset border; the legacy
            # frameborder=0 presentation hint suppresses it.
            frame_border = 0 if node.attrs.get('frameborder', '').strip() == '0' else 2
            lines.append(
                f"{ws}doc.node_mut({var}).style.border_top_width = {frame_border};"
            )
            lines.append(
                f"{ws}doc.node_mut({var}).style.border_right_width = {frame_border};"
            )
            lines.append(
                f"{ws}doc.node_mut({var}).style.border_bottom_width = {frame_border};"
            )
            lines.append(
                f"{ws}doc.node_mut({var}).style.border_left_width = {frame_border};"
            )
            if frame_border:
                lines.append(f"{ws}doc.node_mut({var}).style.border_top_style = BorderStyle::Inset;")
                lines.append(f"{ws}doc.node_mut({var}).style.border_right_style = BorderStyle::Inset;")
                lines.append(f"{ws}doc.node_mut({var}).style.border_bottom_style = BorderStyle::Inset;")
                lines.append(f"{ws}doc.node_mut({var}).style.border_left_style = BorderStyle::Inset;")
        elif (
            node.tag == 'input'
            and node.attrs.get('type', 'text').lower() == 'range'
        ):
            # Chromium's Linux range control is a 129x16 atomic widget. The
            # intrinsic-size algorithm handles cyclic percentage preferred
            # sizes; the passive native track/thumb are painted from the DOM
            # form-control role.
            lines.append(
                f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                "resource: openui_dom::ReplacedResourceKind::TransparentCanvas, "
                "intrinsic_width: Some(129.0), intrinsic_height: Some(16.0), "
                "intrinsic_ratio: None });"
            )
        elif (
            node.tag == 'input'
            and node.attrs.get('type', 'text').lower() in {'checkbox', 'radio'}
        ):
            # Linux native checkable controls are 13px square. Their theme
            # border is appearance paint rather than a CSS border, so authored
            # percentage dimensions retain their exact border-box measure.
            lines.append(
                f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                "resource: openui_dom::ReplacedResourceKind::TransparentCanvas, "
                "intrinsic_width: Some(13.0), intrinsic_height: Some(13.0), "
                "intrinsic_ratio: Some((1.0, 1.0)) });"
            )
        elif node.tag in {'meter', 'progress'}:
            # Chromium's Linux meter control is an 80x16 atomic widget.  Its
            # passive track is painted by the form-control role; transparent
            # canvas metadata supplies deterministic replaced sizing without
            # inventing a raster resource.
            lines.append(
                f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                "resource: openui_dom::ReplacedResourceKind::TransparentCanvas, "
                "intrinsic_width: Some(80.0), intrinsic_height: Some(16.0), "
                "intrinsic_ratio: None });"
            )
        elif node.tag == 'button' and not node.children and not control_all_reset:
            # With the comparison reset removing UA padding, an empty native
            # button has a zero-sized content area surrounded only by its 2px
            # theme border. A button whose author-level `all` reset suppresses
            # native appearance is an ordinary non-replaced CSS box, while
            # non-empty native buttons continue through text-driven sizing.
            lines.append(
                f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                "resource: openui_dom::ReplacedResourceKind::TransparentCanvas, "
                "intrinsic_width: Some(0.0), intrinsic_height: Some(0.0), "
                "intrinsic_ratio: None });"
            )
        elif node.tag in {'input', 'textarea', 'select'}:
            # Text-like controls are atomic/monolithic replaced boxes even
            # when their authored background and border are ordinary CSS.
            # Keep their UA size intrinsic rather than authored so an auto
            # cross size remains eligible for flex/grid stretch.
            authored_font_token = node.styles.get('font-size')
            if authored_font_token is None and 'font' in node.styles:
                authored_font_token = parse_font_shorthand(
                    node.styles['font']
                ).get('font-size')

            if node.tag == 'input':
                input_type = node.attrs.get('type', 'text').lower()
                authored_font_size = None
                if authored_font_token is not None:
                    try:
                        authored_font_size = _computed_font_size(
                            authored_font_token, parent_font_size
                        )
                    except (UnsupportedFontShorthand, ValueError):
                        pass
                if input_type in {'button', 'submit', 'reset'}:
                    intrinsic_width = 0.0
                    intrinsic_height = 14.0
                else:
                    intrinsic_width = (
                        authored_font_size * 20.0
                        if authored_font_size is not None else 262.0
                    )
                    intrinsic_height = (
                        authored_font_size
                        if authored_font_size is not None else
                        16.0 if control_all_reset else 14.0
                    )
            elif node.tag == 'textarea':
                try:
                    rows_value = max(1, int(node.attrs.get('rows', '2')))
                    cols_value = max(1, int(node.attrs.get('cols', '20')))
                except ValueError:
                    rows_value, cols_value = 2, 20
                authored_font_size = None
                if authored_font_token is not None:
                    try:
                        authored_font_size = _computed_font_size(
                            authored_font_token, parent_font_size
                        )
                    except (UnsupportedFontShorthand, ValueError):
                        pass
                if authored_font_size is None:
                    intrinsic_width = cols_value * 8.5 + 6.0
                    intrinsic_height = rows_value * 11.0 + 6.0
                else:
                    intrinsic_width = cols_value * authored_font_size + 15.0
                    intrinsic_height = rows_value * authored_font_size
            else:
                size = node.attrs.get('size', '')
                multiple = 'multiple' in node.attrs
                try:
                    visible_rows = max(1, int(size)) if size else (4 if multiple else 1)
                except ValueError:
                    visible_rows = 4 if multiple else 1
                visible_optgroup = any(
                    child.tag == 'optgroup'
                    and child.styles.get('display', '').strip().lower() != 'none'
                    for child in node.children
                    if not child.is_text
                )
                intrinsic_width = (
                    20.0 if visible_rows == 1 else 4.0 if visible_optgroup else 0.0
                )
                select_font_size = parent_font_size
                if authored_font_token is not None:
                    try:
                        select_font_size = _computed_font_size(
                            authored_font_token, parent_font_size
                        )
                    except (UnsupportedFontShorthand, ValueError):
                        pass
                select_row_height = (
                    17.0 if visible_optgroup else
                    15.0 if authored_font_token is None else
                    float(max(3.0, round(select_font_size)))
                )
                intrinsic_height = float(visible_rows * select_row_height)
            lines.append(
                f"{ws}doc.node_mut({var}).replaced = Some(openui_dom::ReplacedContent {{ "
                "resource: openui_dom::ReplacedResourceKind::TransparentCanvas, "
                f"intrinsic_width: Some({intrinsic_width}), "
                f"intrinsic_height: Some({intrinsic_height}), intrinsic_ratio: None }});"
            )

        # Set display:block for block-level HTML elements (our engine defaults to inline)
        block_tags = {'div', 'p', 'section', 'article', 'header', 'footer', 'nav', 'main',
                      'aside', 'figure', 'figcaption', 'blockquote', 'pre', 'address',
                      'details', 'summary', 'fieldset', 'legend', 'form', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
                      'dl', 'dt', 'dd', 'ol', 'ul', 'menu', 'li', 'hr',
                      'option', 'optgroup'}
        supported_display_values = {
            'block', 'inline', 'inline-block', 'none', 'flow-root',
            'contents', 'list-item', 'flow-root list-item',
            'list-item flow-root', 'flex', 'inline-flex', 'grid',
            'inline-grid', 'table', 'inline-table', 'table-row-group',
            'table-header-group', 'table-footer-group', 'table-row',
            'table-cell', 'table-column-group', 'table-column', 'table-caption',
        }
        display_value = node.styles.get('display', '').strip()
        if node.tag == 'li' and (
            'display' not in node.styles or display_value not in supported_display_values
        ):
            lines.append(f"{ws}doc.node_mut({var}).style.display = Display::ListItem;")
        elif node.tag == 'summary' and (
            'display' not in node.styles or display_value not in supported_display_values
        ):
            # The HTML UA sheet gives an authored summary a list-item
            # principal box.  The comparison stylesheet may suppress its
            # marker, but it does not change that display type; preserving it
            # is observable when inline summary content fragments in columns.
            lines.append(f"{ws}doc.node_mut({var}).style.display = Display::ListItem;")
        elif node.tag in block_tags and (
            'display' not in node.styles or display_value not in supported_display_values
        ):
            lines.append(f"{ws}doc.node_mut({var}).style.display = Display::Block;")

        # Apply UA default styles for HTML elements (before explicit styles so CSS can override)
        _UA_DEFAULTS = {
            'p': [('margin_top', 'Length::px(16.0)'), ('margin_bottom', 'Length::px(16.0)')],
            'ul': [('margin_top', 'Length::px(16.0)'), ('margin_bottom', 'Length::px(16.0)'),
                   ('padding_left', 'Length::px(40.0)')],
            'ol': [('margin_top', 'Length::px(16.0)'), ('margin_bottom', 'Length::px(16.0)'),
                   ('padding_left', 'Length::px(40.0)')],
            'blockquote': [('margin_top', 'Length::px(16.0)'), ('margin_bottom', 'Length::px(16.0)'),
                           ('margin_left', 'Length::px(40.0)'), ('margin_right', 'Length::px(40.0)')],
            'dd': [('margin_left', 'Length::px(40.0)')],
        }
        _UA_CSS_MAP = {
            'margin_top': {'margin-top', 'margin', 'margin-block-start', 'margin-block'},
            'margin_bottom': {'margin-bottom', 'margin', 'margin-block-end', 'margin-block'},
            'margin_left': {'margin-left', 'margin', 'margin-inline-start', 'margin-inline'},
            'margin_right': {'margin-right', 'margin', 'margin-inline-end', 'margin-inline'},
            'padding_left': {'padding-left', 'padding', 'padding-inline-start', 'padding-inline'},
        }
        if node.tag in _UA_DEFAULTS:
            s = f"doc.node_mut({var}).style"
            for field, val in _UA_DEFAULTS[node.tag]:
                css_names = _UA_CSS_MAP.get(field, set())
                if not any(p in node.styles for p in css_names):
                    lines.append(f"{ws}{s}.{field} = {val};")

        # Resolve explicit CSS-wide `inherit` before generating style code.
        effective_styles = _copy_declarations(node.styles)
        if node.tag == 'marquee':
            # Chromium exposes column properties on the legacy host, but its
            # anonymous scrolling block remains a single vertical formatting
            # context. Lower that anonymous structure by keeping columns off
            # the principal layout box.
            for prop in ('columns', 'column-count', 'column-width'):
                effective_styles.pop(prop, None)
        if (
            node.tag == 'legend'
            and getattr(node, 'fieldset_legend_index', 0) > 0
            and 'position' not in effective_styles
            and 'top' not in effective_styles
        ):
            # Only the first direct legend is the fieldset's special legend.
            # Subsequent legends are normal flow children below the fieldset's
            # block-start padding edge; pin that one-device-pixel UA offset as
            # a relative visual displacement without changing flow height.
            effective_styles['position'] = 'relative'
            effective_styles['top'] = '1px'
        if (
            node.tag == 'optgroup'
            and getattr(node, 'parent_tag', '') != 'select'
            and effective_styles.get('display', '').strip().lower() != 'contents'
        ):
            # A standalone optgroup reserves one native label-row before its
            # authored contents. Padding, rather than a fixed height, lets
            # visible text contribute its own line box below that row. The
            # native anonymous label is not itself a CSS multicol container,
            # even though the host's computed column properties are visible.
            for prop in ('columns', 'column-count', 'column-width'):
                effective_styles.pop(prop, None)
            if 'padding-top' not in effective_styles:
                effective_styles['padding-top'] = '20.1875px'
        is_input_button = (
            node.tag == 'input'
            and node.attrs.get('type', 'text').lower() in {'button', 'submit', 'reset'}
        )
        uses_native_button_ahem_metrics = False

        if RETAIN_TEXT and node.tag in {'button', 'input', 'textarea', 'select'}:
            # The deterministic font override changes the family but not the
            # HTML form-control UA size. Chromium's Linux UA sheet specifies
            # 13.3333px controls; materialize that computed value so their
            # anonymous text and multicol contents inherit the same metrics.
            if (
                'font-size' not in effective_styles
                and 'font' not in effective_styles
                and not (
                    node.tag in {'button', 'input', 'textarea', 'select'}
                    and control_all_reset
                )
            ):
                effective_styles['font-size'] = '13.333333px'
                uses_native_button_ahem_metrics = node.tag == 'button' or is_input_button
            if (
                not control_all_reset
                and (node.tag == 'button' or is_input_button)
                and 'text-align' not in effective_styles
            ):
                effective_styles['text-align'] = 'center'
            if (
                not control_all_reset
                and (node.tag == 'button' or is_input_button)
                and 'white-space' not in effective_styles
            ):
                # Form buttons use a non-wrapping anonymous label box. This is
                # observable when deterministic Ahem makes the label exactly
                # as wide as the native control's shrink-to-fit width.
                effective_styles['white-space'] = 'pre'
        authored_button_background = any(
            prop in effective_styles
            for prop in {'background', 'background-color', 'background-image'}
        )
        if (node.tag == 'button' or is_input_button) and not control_all_reset:
            # Passive Linux button appearance. The comparison harness resets
            # margin/padding/box-sizing as author CSS, but leaves these UA
            # paint declarations in force unless the test overrides them.
            if not any(
                name in effective_styles
                for name in {'overflow', 'overflow-x', 'overflow-y'}
            ):
                effective_styles['overflow'] = 'clip'
            if (
                'background' not in effective_styles
                and 'background-color' not in effective_styles
            ):
                effective_styles['background-color'] = 'rgb(239, 239, 239)'
            if not any(
                name == 'border' or name.startswith('border-')
                for name in effective_styles
            ):
                effective_styles['border-width'] = '2px'
                if authored_button_background:
                    # Painting an authored button face suppresses the native
                    # rounded widget decoration but retains the legacy Linux
                    # outset edge. Resolve the pinned platform ButtonBorder
                    # palette into its computed physical side colors. Keeping
                    # solid sides here also lets the ordinary border painter
                    # form Chromium's #2a corner miters deterministically.
                    effective_styles['border-style'] = 'solid'
                    effective_styles['border-top-color'] = '#545454'
                    effective_styles['border-left-color'] = '#545454'
                    effective_styles['border-right-color'] = 'black'
                    effective_styles['border-bottom-color'] = 'black'
                    effective_styles['border-radius'] = '0'
                else:
                    # The pinned Linux native button theme exposes its passive
                    # `buttonborder` edge as a uniform #767676 rounded stroke.
                    # This is appearance geometry, not CSS `outset` color
                    # shading, even though Blink's UA declaration uses that
                    # legacy keyword.
                    effective_styles['border-style'] = 'solid'
                    effective_styles['border-color'] = '#767676'
                    effective_styles['border-radius'] = '2px'
        elif (
            node.tag == 'input'
            and node.attrs.get('type', 'text').lower() == 'color'
        ):
            # Linux Chromium paints the passive color well with the same
            # rounded ButtonFace/ButtonBorder shell as an input button. The
            # black color swatch is overlaid by the public control painter.
            if (
                'background' not in effective_styles
                and 'background-color' not in effective_styles
            ):
                effective_styles['background-color'] = 'rgb(239, 239, 239)'
            if not any(
                name == 'border' or name.startswith('border-')
                for name in effective_styles
            ):
                effective_styles['border-width'] = '2px'
                effective_styles['border-style'] = 'solid'
                effective_styles['border-color'] = '#767676'
                effective_styles['border-radius'] = '2px'

        # A floated semantic table remains a table formatting context after
        # display blockification (only `inline-table` blockifies to `table`).
        # Make the UA display role explicit before the generic float lowering
        # runs so it cannot mistake the element's initial builder value for an
        # authored inline box.
        semantic_table_displays = {
            'table': 'table',
            'caption': 'table-caption',
            'colgroup': 'table-column-group',
            'col': 'table-column',
            'thead': 'table-header-group',
            'tbody': 'table-row-group',
            'tfoot': 'table-footer-group',
            'tr': 'table-row',
            'td': 'table-cell',
            'th': 'table-cell',
        }
        if node.tag in semantic_table_displays and 'display' not in effective_styles:
            effective_styles['display'] = semantic_table_displays[node.tag]

        # HTML table presentational hints participate below author CSS in the
        # cascade. Generated documents do not run Chromium's HTML mapping, so
        # materialize the corresponding declarations only when author CSS did
        # not supply the property itself.
        table_width_tags = {'table', 'colgroup', 'col', 'td', 'th', 'img'}
        table_height_tags = {'table', 'tr', 'td', 'th', 'img'}

        def _legacy_dimension(value: str) -> str | None:
            token = value.strip()
            if not token:
                return None
            try:
                if token.endswith('%'):
                    amount = max(0.0, float(token[:-1]))
                    return f'{amount}%'
                amount = max(0.0, float(token))
                return f'{amount}px'
            except ValueError:
                return None

        if node.tag in table_width_tags and 'width' not in effective_styles:
            hinted = _legacy_dimension(node.attrs.get('width', ''))
            if hinted is not None:
                effective_styles['width'] = hinted
        if node.tag in table_height_tags and 'height' not in effective_styles:
            hinted = _legacy_dimension(node.attrs.get('height', ''))
            if hinted is not None:
                effective_styles['height'] = hinted
        if node.tag == 'table' and 'float' not in effective_styles:
            legacy_align = node.attrs.get('align', '').strip().lower()
            if legacy_align in {'left', 'right'}:
                effective_styles['float'] = legacy_align
        if node.tag == 'table':
            try:
                legacy_border_width = max(0, int(node.attrs.get('border', '0') or '0'))
            except ValueError:
                legacy_border_width = 1 if 'border' in node.attrs else 0
            html_table_border = legacy_border_width > 0
            if html_table_border:
                authored_border = any(
                    name == 'border' or name.startswith('border-')
                    for name in node.styles
                )
                if not authored_border:
                    effective_styles['border-width'] = f'{legacy_border_width}px'
                    effective_styles['border-style'] = 'outset'
                    effective_styles['border-color'] = 'gray'
        elif node.tag in {'td', 'th'} and html_table_border:
            authored_border = any(
                name == 'border' or name.startswith('border-')
                for name in node.styles
            )
            if not authored_border:
                # HTML's legacy table-border mapping gives descendant cells a
                # one-pixel inset border independently of the table's width.
                effective_styles['border-width'] = '1px'
                effective_styles['border-style'] = 'inset'
                effective_styles['border-color'] = 'gray'
        if (
            node.tag in {'thead', 'tbody', 'tfoot', 'tr', 'td', 'th'}
            and 'vertical-align' not in effective_styles
        ):
            legacy_valign = node.attrs.get('valign', '').strip().lower()
            if legacy_valign in {'top', 'middle', 'bottom', 'baseline'}:
                effective_styles['vertical-align'] = legacy_valign
        if 'background-color' not in effective_styles and 'background' not in effective_styles:
            legacy_background = node.attrs.get('bgcolor', '').strip()
            if legacy_background:
                effective_styles['background-color'] = legacy_background
        if (
            RETAIN_TEXT
            and node.tag == 'rt'
            and 'font-size' not in effective_styles
            and 'font' not in effective_styles
        ):
            # HTML's ruby UA rules size unstyled annotations to half of the
            # ruby base. Generated documents have no UA cascade.
            effective_styles['font-size'] = '50%'
        if (
            RETAIN_TEXT
            and node.tag == 'a'
            and 'href' in node.attrs
            and 'color' not in effective_styles
        ):
            # Generated documents do not run the HTML UA sheet. Its :any-link
            # rule supplies a specified blue color, which wins over an
            # inherited ancestor color unless author CSS targets the anchor.
            effective_styles['color'] = '#0000ee'
        for prop in sorted(_SP17_INHERITED_PROPERTIES):
            if prop in inherited and prop not in effective_styles:
                effective_styles[prop] = inherited[prop]
        if (
            RETAIN_TEXT
            and 'line-height' in inherited
            and 'line-height' not in effective_styles
            and 'font' not in effective_styles
        ):
            # `lh` units use the element's inherited computed line-height.
            # Materialize it before resolving geometry rather than in the
            # later inherited-style emission pass.
            effective_styles['line-height'] = inherited['line-height']
        if is_real_font_profile():
            # Generated DOM nodes do not run a cascade. Materialize inherited
            # font properties before resolving ch/ex/lh so face selection sees
            # the actual computed values.
            for prop in sorted(INHERITED_PROPS):
                if prop in inherited and prop not in effective_styles:
                    effective_styles[prop] = inherited[prop]
            if (
                node.tag in ('i', 'em')
                and 'font-style' not in node.styles
                and 'font' not in node.styles
            ):
                # HTML's UA sheet gives semantic emphasis an italic used
                # style. Generated documents have no UA cascade, so retain
                # that style in the real-font profile unless author CSS wins.
                effective_styles['font-style'] = 'italic'
        if (
            RETAIN_TEXT
            and node.tag in ('h1', 'h2', 'h3', 'h4', 'h5', 'h6')
            and 'font-size' not in node.styles
            and 'font' not in node.styles
        ):
            # Chromium's UA sheet gives headings a relative font size. Styled
            # headings are retained in deterministic-text mode, so properties
            # such as `height: 4em` must resolve against that computed size too.
            # Test the authored declarations, not `effective_styles`: the
            # real-font profile materializes inherited font-size there before
            # this UA cascade step.  The heading rule wins over inheritance.
            # Store the absolute computed value to keep the legacy box-only
            # generator byte-for-byte unchanged and avoid treating the UA `em`
            # value as author CSS during code generation.
            heading_scale = {
                'h1': 2.0,
                'h2': 1.5,
                'h3': 1.17,
                'h4': 1.0,
                'h5': 0.83,
                'h6': 0.67,
            }[node.tag]
            effective_styles['font-size'] = f'{parent_font_size * heading_scale}px'
        if (
            RETAIN_TEXT
            and node.tag in ('h1', 'h2', 'h3', 'h4', 'h5', 'h6')
            and 'font-weight' not in node.styles
            and 'font' not in node.styles
        ):
            # The UA heading rule also contributes bold weight. Materialize it
            # for generated documents for the same reason as the relative
            # heading size above: builders do not run a browser UA cascade.
            effective_styles['font-weight'] = 'bold'
        if RETAIN_TEXT and effective_styles.get('border-radius', '').strip() == 'inherit':
            # The shorthand inherits the parent's four computed longhands,
            # including initial zero values for corners the parent omitted.
            del effective_styles['border-radius']
            for corner in _BORDER_RADIUS_CORNERS:
                effective_styles[corner] = inherited.get(corner, '0')
        for prop, val in list(effective_styles.items()):
            if isinstance(val, str) and val.strip() == 'inherit':
                if prop in inherited:
                    effective_styles[prop] = inherited[prop]
                elif is_real_font_profile() and prop in REAL_FONT_INITIALS:
                    effective_styles[prop] = REAL_FONT_INITIALS[prop]
                elif RETAIN_TEXT and prop in _BORDER_RADIUS_CORNERS:
                    effective_styles[prop] = '0'
        node_custom_props = dict(custom_props)
        for prop, val in effective_styles.items():
            if prop.startswith('--'):
                node_custom_props[prop] = val
        for prop, val in list(effective_styles.items()):
            if isinstance(val, str) and not prop.startswith('--'):
                effective_styles[prop] = _resolve_css_vars(val, node_custom_props)

        if node.tag in {'button', 'input', 'textarea', 'select'}:
            # `appearance` and its vendor aliases are one cascaded control
            # property in Chromium. Select the winning alias by the preserved
            # declaration priority instead of depending on dictionary order.
            appearance_candidates = []
            for appearance_prop in (
                'appearance', '-webkit-appearance', '-moz-appearance'
            ):
                if appearance_prop in effective_styles:
                    appearance_candidates.append((
                        effective_styles.cascade_priority.get(
                            appearance_prop, (0, 0, 0, 0, 0, 0, 0)
                        ),
                        appearance_prop,
                        effective_styles[appearance_prop].strip().lower(),
                    ))
            if appearance_candidates:
                _, _, appearance_value = max(appearance_candidates)
                if appearance_value == 'none':
                    lines.append(
                        f"{ws}doc.node_mut({var}).form_control_native_appearance = false;"
                    )
                    if node.tag == 'button':
                        if not any(
                            name in node.styles
                            for name in {'background', 'background-color'}
                        ):
                            effective_styles['background-color'] = 'rgb(250, 250, 250)'
                        if not any(
                            name == 'border' or name.startswith('border-')
                            for name in node.styles
                        ):
                            effective_styles['border-width'] = '2px'
                            effective_styles['border-style'] = 'solid'
                            effective_styles['border-top-color'] = '#d1d1d1'
                            effective_styles['border-left-color'] = '#d1d1d1'
                            effective_styles['border-right-color'] = '#b7b7b7'
                            effective_styles['border-bottom-color'] = '#b7b7b7'
                            effective_styles['border-radius'] = '0'
                    if (
                        node.tag == 'input'
                        and not any(
                            name == 'border-color'
                            or (name.startswith('border-') and name.endswith('-color'))
                            or name == 'border'
                            for name in effective_styles
                        )
                    ):
                        # Blink's non-native text-field UA inset border has
                        # an explicit ButtonBorder base rather than black
                        # currentColor. Its CSS 3D shading yields #212121 on
                        # the recessed edges and #767676 on the raised edges.
                        effective_styles['border-color'] = '#767676'
            elif control_all_reset:
                # Author-level `all: initial` / `all: unset` resets
                # `appearance` to its initial non-native value. UA button
                # paint and metrics must not be reintroduced after the author
                # cascade has already won.
                lines.append(
                    f"{ws}doc.node_mut({var}).form_control_native_appearance = false;"
                )

        if (
            node.tag == 'button'
            and 'disabled' in node.attrs
            and 'color' not in effective_styles
        ):
            # HTML's disabled button system color is a translucent near-black.
            # Keeping its alpha is observable over both the native #eee face
            # and the #fafafa non-native appearance-none face.
            effective_styles['color'] = 'rgba(16, 16, 16, 0.3)'

        # Generate style code
        node_zoom = _effective_css_zoom(effective_styles, parent_zoom)
        style_lines, node_font_size = generate_style_code(
            effective_styles, var, parent_font_size, node_zoom
        )
        for sl in style_lines:
            lines.append(f"{ws}{sl}")
        if uses_native_button_ahem_metrics:
            lines.append(
                f"{ws}doc.node_mut({var}).style.native_button_text_metrics = true;"
            )

        if (
            RETAIN_TEXT
            and not is_real_font_profile()
            and not isolated_document
        ):
            # TEXT_TEMPLATE_OVERRIDE is author-important and therefore wins
            # over every declaration in the source test. Materialize those
            # computed values after authored style emission on the Rust side.
            lines.append(
                f'{ws}doc.node_mut({var}).style.font_family = '
                f'{DETERMINISTIC_FONT_FAMILY_RUST};'
            )

        if node.styles.get('overflow-clip-margin') == 'inherit':
            lines.append(f"{ws}let {var}_inherited_overflow_clip_margin = doc.node({parent_var}).style.overflow_clip_margin;")
            lines.append(f"{ws}let {var}_inherited_overflow_clip_box = doc.node({parent_var}).style.overflow_clip_box;")
            lines.append(f"{ws}doc.node_mut({var}).style.overflow_clip_margin = {var}_inherited_overflow_clip_margin;")
            lines.append(f"{ws}doc.node_mut({var}).style.overflow_clip_box = {var}_inherited_overflow_clip_box;")
        if node.styles.get('border') == 'inherit':
            # `border` is not inherited by default, but the CSS-wide inherit
            # keyword copies all twelve computed longhands from the parent.
            # The declaration map intentionally retains the shorthand, so
            # materialize this dynamic parent dependency after ordinary style
            # emission instead of trying to resolve it through inherited text
            # properties.
            lines.append(
                f"{ws}let {var}_inherited_border = doc.node({parent_var}).style.clone();"
            )
            for side in ('top', 'right', 'bottom', 'left'):
                for suffix in ('width', 'style', 'color'):
                    field = f"border_{side}_{suffix}"
                    lines.append(
                        f"{ws}doc.node_mut({var}).style.{field} = "
                        f"{var}_inherited_border.{field};"
                    )

        # Apply inherited CSS properties from ancestors that this node doesn't override
        font_shorthand_longhands = {
            'font-family', 'font-size', 'font-weight', 'font-style',
            'font-stretch', 'font-variant-caps', 'line-height',
        }
        for prop in sorted(INHERITED_PROPS):
            if 'font' in effective_styles and prop in font_shorthand_longhands:
                continue
            if prop in inherited and prop not in effective_styles:
                val = inherited[prop]
                s = f"doc.node_mut({var}).style"
                inh_lines = generate_single_style(prop, val, s, node_font_size)
                if inh_lines is not None:
                    if isinstance(inh_lines, str):
                        inh_lines = [inh_lines]
                    for il in inh_lines:
                        lines.append(f"{ws}{il}")

        if node_zoom != 1.0 and not is_real_font_profile():
            lines.append(
                f"{ws}doc.node_mut({var}).style.font_size = "
                f"{round(node_font_size * node_zoom, 12)};"
            )

        lines.append(f"{ws}doc.append_child({parent_var}, {var});")

        # Preserve source attributes for generated `attr()` values. The
        # document keeps these separately from labels/debug metadata.
        for attr_name, attr_value in sorted(node.attrs.items()):
            lines.append(
                f'{ws}doc.set_attribute({var}, '
                f'"{_rust_escape_string(attr_name)}", '
                f'"{_rust_escape_string(attr_value or "")}");'
            )

        # Build inherited props for children: parent inherited + this node's own
        child_inherited = _computed_child_boundary(inherited)
        if uses_native_button_ahem_metrics:
            # This is an internal builder contract, not a CSS property. Carry
            # it through arbitrary inline descendants so only text belonging
            # to the native button receives the platform advance metrics.
            child_inherited['__native_button_text_metrics'] = True
        child_inherit_props = EXPLICIT_INHERIT_PROPS | (
            TEXT_EXTRA_INHERITED if RETAIN_TEXT else set()
        )
        for prop in sorted(child_inherit_props):
            if prop in effective_styles:
                child_inherited[prop] = effective_styles[prop]
        if (
            node.tag in {'caption', 'th'}
            and 'text-align' not in (node.styles or {})
        ):
            # The semantic UA rule wins over inherited alignment and is itself
            # inherited by the caption/header-cell descendants. The compact
            # DOM has no runtime cascade, so materialize that computed boundary
            # in the porter's inheritance state as well as on the element.
            child_inherited['text-align'] = 'center'
        if is_real_font_profile():
            if 'font-size' in child_inherited:
                child_inherited['font-size'] = f'{node_font_size}px'
            if 'line-height' in child_inherited:
                child_inherited['line-height'] = _computed_inherited_line_height(
                    child_inherited['line-height'], node_font_size
                )
        clipped_text_color = _background_text_color(effective_styles)
        if clipped_text_color:
            child_inherited['color'] = clipped_text_color
        if RETAIN_TEXT:
            computed_radius = _computed_border_radius(effective_styles)
            if computed_radius is not None:
                child_inherited.update(computed_radius)
        # SP14: thread font-family from the `font` shorthand too (for text nodes).
        if 'font' in effective_styles:
            _fam = _family_from_font_shorthand(effective_styles['font'])
            if _fam:
                child_inherited['font-family'] = _fam
            # The shorthand resets and supplies the computed font size too.
            # Thread the resolved absolute value rather than the authored token
            # so descendants materialize the same used value as text nodes.
            child_inherited['font-size'] = f'{node_font_size}px'
            child_inherited['line-height'] = _line_height_from_font_shorthand(
                effective_styles['font']
            )

        def resolved_pseudo_styles(name: str) -> CssDeclarations:
            pseudo = _copy_declarations(node.pseudo_styles.get(name, {}))
            for prop, value in list(pseudo.items()):
                if isinstance(value, str):
                    pseudo[prop] = _resolve_css_vars(value, node_custom_props)
            return pseudo

        def establishes_scroll_container() -> bool:
            shorthand = effective_styles.get('overflow', 'visible').strip().lower()
            overflow_x = effective_styles.get('overflow-x', shorthand).strip().lower()
            overflow_y = effective_styles.get('overflow-y', shorthand).strip().lower()
            return overflow_x in {'auto', 'scroll', 'hidden'} or overflow_y in {
                'auto', 'scroll', 'hidden'
            }

        def emit_generated_pseudo(
            name: str, *, structural: bool = False
        ) -> str | None:
            pseudo_styles = resolved_pseudo_styles(name)
            content_value = pseudo_styles.get('content')
            if not structural and (
                content_value is None
                or content_value.strip().lower() in ('normal', 'none')
            ):
                return None
            materialization_required[0] = True
            counter[0] += 1
            pseudo_var = f"n{counter[0]}"
            kinds = {
                'before': 'Before',
                'after': 'After',
                'marker': 'Marker',
                'scroll-marker': 'ScrollMarker',
                'scroll-marker-group': 'ScrollMarkerGroup',
                'column': 'Column',
                'column-scroll-marker': 'ColumnScrollMarker',
                'details-content': 'DetailsContent',
                'scroll-button-up': 'ScrollButton(openui_dom::ScrollButtonDirection::Up)',
                'scroll-button-right': 'ScrollButton(openui_dom::ScrollButtonDirection::Right)',
                'scroll-button-down': 'ScrollButton(openui_dom::ScrollButtonDirection::Down)',
                'scroll-button-left': 'ScrollButton(openui_dom::ScrollButtonDirection::Left)',
                'scroll-button-block-start': 'ScrollButton(openui_dom::ScrollButtonDirection::BlockStart)',
                'scroll-button-block-end': 'ScrollButton(openui_dom::ScrollButtonDirection::BlockEnd)',
                'scroll-button-inline-start': 'ScrollButton(openui_dom::ScrollButtonDirection::InlineStart)',
                'scroll-button-inline-end': 'ScrollButton(openui_dom::ScrollButtonDirection::InlineEnd)',
            }
            kind = kinds[name]
            lines.append(
                f"{ws}let {pseudo_var} = doc.insert_pseudo_element("
                f"{var}, openui_dom::PseudoElementKind::{kind});"
            )
            lines.append(
                f"{ws}let {pseudo_var}_style = "
                f"ComputedStyle::for_pseudo(&doc.node({var}).style);"
            )
            lines.append(f"{ws}doc.node_mut({pseudo_var}).style = {pseudo_var}_style;")
            if name.startswith('scroll-button-'):
                # Linux Chromium exposes passive scroll buttons through the
                # platform button role. Preserve its stable minimum geometry
                # and native-looking initial decoration; authored pseudo
                # declarations below retain normal precedence.
                emit_passive_scroll_button_defaults(
                    pseudo_var,
                    ws,
                    disabled=not establishes_scroll_container(),
                    system_font=not any(
                        prop in pseudo_styles for prop in {'font', 'font-family'}
                    ),
                )
            pseudo_lines, _ = generate_style_code(
                pseudo_styles, pseudo_var, node_font_size, node_zoom
            )
            for pseudo_line in pseudo_lines:
                lines.append(f"{ws}{pseudo_line}")
            if (
                name.startswith('scroll-button-')
                and pseudo_styles.get('appearance', '').strip().lower() == 'none'
            ):
                emit_appearance_none_button_defaults(pseudo_var, ws)
            if (
                name.startswith('scroll-button-')
                and 'width' in pseudo_styles
                and 'min-width' not in pseudo_styles
            ):
                lines.append(
                    f"{ws}doc.node_mut({pseudo_var}).style.min_width = Length::auto();"
                )
                # The platform's ordinary six-pixel inline padding determines
                # generated-content alignment inside a fixed author width.
                # The one-pixel fit-content compensation above applies only
                # when native text metrics determine the outer width.
                lines.extend([
                    f"{ws}if doc.node({pseudo_var}).style.writing_mode.is_horizontal() {{",
                    f"{ws}    doc.node_mut({pseudo_var}).style.padding_top = Length::px(1.0);",
                    f"{ws}    doc.node_mut({pseudo_var}).style.padding_right = Length::px(6.0);",
                    f"{ws}    doc.node_mut({pseudo_var}).style.padding_bottom = Length::px(1.0);",
                    f"{ws}    doc.node_mut({pseudo_var}).style.padding_left = Length::px(6.0);",
                    f"{ws}}} else {{",
                    f"{ws}    doc.node_mut({pseudo_var}).style.padding_top = Length::px(6.0);",
                    f"{ws}    doc.node_mut({pseudo_var}).style.padding_right = Length::px(1.0);",
                    f"{ws}    doc.node_mut({pseudo_var}).style.padding_bottom = Length::px(6.0);",
                    f"{ws}    doc.node_mut({pseudo_var}).style.padding_left = Length::px(1.0);",
                    f"{ws}}}",
                ])
            if (
                name.startswith('scroll-button-')
                and not any(
                    prop == 'border' or prop.startswith('border-')
                    for prop in pseudo_styles
                )
                and any(
                    prop in {'background', 'background-color', 'background-image'}
                    for prop in pseudo_styles
                )
            ):
                for side in ('top', 'right', 'bottom', 'left'):
                    channel = 84 if side in ('top', 'left') else 0
                    lines.append(
                        f"{ws}doc.node_mut({pseudo_var}).style.border_{side}_width = 2;"
                    )
                    lines.append(
                        f"{ws}doc.node_mut({pseudo_var}).style.border_{side}_color = "
                        f"StyleColor::Resolved(Color::from_rgba8({channel}, {channel}, {channel}, 255));"
                    )
                if establishes_scroll_container():
                    lines.append(
                        f"{ws}doc.node_mut({pseudo_var}).style.color = "
                        f"doc.node({var}).style.color;"
                    )
            if name.startswith('scroll-button-') and any(
                prop == 'border'
                or prop.startswith('border-')
                or prop in {'background', 'background-color', 'background-image'}
                for prop in pseudo_styles
            ):
                # Author-painted scroll buttons leave the native theme path;
                # CSS's initial corner radii are square even when the
                # platform control default above is rounded.
                for corner in (
                    'top_left', 'top_right', 'bottom_right', 'bottom_left'
                ):
                    lines.append(
                        f"{ws}doc.node_mut({pseudo_var}).style.border_{corner}_radius = "
                        "(0.0, 0.0);"
                    )
            if name == 'scroll-marker-group':
                # The pseudo's initial display is block and it establishes
                # layout containment. It deliberately does not imply size
                # containment: floats and other marker contents determine an
                # automatic group size.
                if 'display' not in pseudo_styles:
                    lines.append(
                        f"{ws}doc.node_mut({pseudo_var}).style.display = Display::Block;"
                    )
                lines.append(
                    f"{ws}doc.node_mut({pseudo_var}).style.contain |= "
                    "Containment::LAYOUT;"
                )
            if name in ('scroll-marker', 'column-scroll-marker') and 'color' not in pseudo_styles:
                # Scroll markers have link-like activation semantics and use
                # the UA link color when author CSS does not specify one.
                lines.append(
                    f"{ws}doc.node_mut({pseudo_var}).style.color = "
                    "Color::from_rgba8(0, 0, 238, 255);"
                )
            if name in ('scroll-marker', 'column-scroll-marker'):
                current_styles = resolved_pseudo_styles(
                    f'{name}-target-current'
                )
                def static_px(prop: str) -> float:
                    value = effective_styles.get(prop, '0').strip().lower()
                    match = re.fullmatch(r'([+-]?(?:\d+(?:\.\d*)?|\.\d+))(?:px)?', value)
                    return float(match.group(1)) if match else 0.0
                generated_scroll_markers.append((
                    pseudo_var, current_styles,
                    static_px('top'), static_px('left'),
                ))
            return pseudo_var

        def apply_current_scroll_marker_state() -> None:
            group_position = effective_styles.get(
                'scroll-marker-group', ''
            ).strip().lower()
            if group_position not in ('before', 'after'):
                return
            candidates = generated_scroll_markers[marker_scope_start:]
            if not candidates:
                return
            writing_mode = effective_styles.get(
                'writing-mode', 'horizontal-tb'
            ).strip().lower()
            if writing_mode == 'vertical-lr':
                marker_var, current_styles, _, _ = min(
                    candidates, key=lambda candidate: candidate[3]
                )
            elif writing_mode == 'vertical-rl':
                marker_var, current_styles, _, _ = max(
                    candidates, key=lambda candidate: candidate[3]
                )
            else:
                marker_var, current_styles, _, _ = min(
                    candidates, key=lambda candidate: candidate[2]
                )
            current_lines, _ = generate_style_code(
                current_styles, marker_var, node_font_size, node_zoom
            )
            if 'background' in current_styles or 'background-color' in current_styles:
                lines.append(
                    f"{ws}let {marker_var}_inactive_background = "
                    f"doc.node({marker_var}).style.background_color;"
                )
                lines.append(
                    f"{ws}doc.node_mut({marker_var}).scroll_marker_inactive_background = "
                    f"Some({marker_var}_inactive_background);"
                )
            for current_line in current_lines:
                lines.append(f"{ws}{current_line}")

        def emit_highlight_pseudo(name: str) -> None:
            pseudo_styles = resolved_pseudo_styles(name)
            if not pseudo_styles:
                return
            counter[0] += 1
            pseudo_var = f"n{counter[0]}"
            field = 'first_line_style' if name == 'first-line' else 'first_letter_style'
            lines.append(f"{ws}let {pseudo_var} = doc.create_node(ElementTag::Span);")
            lines.append(
                f"{ws}let {pseudo_var}_style = "
                f"ComputedStyle::for_pseudo(&doc.node({var}).style);"
            )
            lines.append(f"{ws}doc.node_mut({pseudo_var}).style = {pseudo_var}_style;")
            pseudo_lines, _ = generate_style_code(
                pseudo_styles, pseudo_var, node_font_size, node_zoom
            )
            for pseudo_line in pseudo_lines:
                lines.append(f"{ws}{pseudo_line}")
            # The deterministic Ahem override targets ordinary elements, but
            # a specified highlight-pseudo family wins over inheritance (the
            # override selector does not select ::first-line/::first-letter).
            # Preserve that pseudo-local computed value without relaxing the
            # frozen element-font profile.
            if not is_real_font_profile() and 'font-family' in pseudo_styles:
                family = _font_family_to_rust(pseudo_styles['font-family'])
                if family:
                    lines.append(
                        f"{ws}doc.node_mut({pseudo_var}).style.font_family = {family};"
                    )
            lines.append(f"{ws}let {pseudo_var}_resolved = doc.node({pseudo_var}).style.clone();")
            lines.append(
                f"{ws}doc.node_mut({var}).style.{field} = "
                f"Some(Box::new({pseudo_var}_resolved));"
            )

        emit_highlight_pseudo('first-line')
        emit_highlight_pseudo('first-letter')
        emit_generated_pseudo('before')
        emit_generated_pseudo('marker')
        if effective_styles.get('scroll-marker-group', '').strip().lower() == 'before':
            marker_group = emit_generated_pseudo('scroll-marker-group', structural=True)
            if marker_group is not None and not establishes_scroll_container():
                # The pseudo exists in the computed pseudo tree, but CSS
                # Overflow 5 suppresses its box unless the origin establishes
                # a scroll container.
                lines.append(
                    f"{ws}doc.node_mut({marker_group}).style.display = Display::None;"
                )
        if node.pseudo_styles.get('column'):
            emit_generated_pseudo('column', structural=True)

        # Process children (inherit font_size + CSS inherited props)
        def emit_generated_quote(text: str) -> None:
            """Materialize the HTML UA ``q`` generated content.

            The builder path has no pseudo-element tree, so the default
            ``q::before``/``q::after`` open-quote and close-quote content must
            be represented by anonymous text children.  Generated content is
            present even in the legacy box-only profile: unlike author text,
            it is part of the element's rendered principal content.
            """
            counter[0] += 1
            quote_var = f"n{counter[0]}"
            quote_style = f"doc.node_mut({quote_var}).style"
            lines.append(
                f"{ws}    let {quote_var} = doc.create_node(ElementTag::Text);"
            )
            lines.append(
                f"{ws}    {quote_style}.font_size = "
                f"{round(float(node_font_size) * node_zoom, 12)};"
            )
            if (
                RETAIN_TEXT
                and not is_real_font_profile()
                and not isolated_document
            ):
                # Generated quote text participates in the same deterministic
                # Ahem runner override as author text. The override is added
                # after CSS collection, so it is not present in
                # ``child_inherited`` and must be materialized here too.
                lines.append(
                    f'{ws}    {quote_style}.font_family = '
                    f'{DETERMINISTIC_FONT_FAMILY_RUST};'
                )
            else:
                family = child_inherited.get('font-family')
                family_rust = _font_family_to_rust(family) if family else None
                if family_rust:
                    lines.append(f"{ws}    {quote_style}.font_family = {family_rust};")
            color = child_inherited.get('color')
            color_rust = parse_color(color) if color else None
            if color_rust:
                lines.append(f"{ws}    {quote_style}.color = {color_rust};")
            for prop in (
                'font-weight', 'font-style', 'font-stretch',
                'font-variant-caps', 'white-space', 'word-break',
                'overflow-wrap', 'line-break', 'hyphens', 'text-wrap',
                'line-height',
                'text-transform', 'letter-spacing', 'word-spacing',
                'direction', 'writing-mode',
                'text-orientation', 'text-combine-upright',
                'text-shadow', 'quotes',
                'text-emphasis-style', 'text-emphasis-position',
                'text-emphasis-color',
            ):
                if prop in child_inherited:
                    code = generate_single_style(
                        prop, child_inherited[prop], quote_style, node_font_size
                    )
                    if code:
                        for generated_line in (
                            code if isinstance(code, list) else [code]
                        ):
                            lines.append(f"{ws}    {generated_line}")
            lines.append(
                f'{ws}    {quote_style[:-6]}.text = '
                f'Some("{_rust_escape_string(text)}".to_string());'
            )
            lines.append(f"{ws}    doc.append_child({var}, {quote_var});")

        if node.tag == 'q':
            emit_generated_quote('\u201c')
        parent_display = effective_styles.get('display', '').strip().lower()

        table_internal_tags = {
            'caption', 'colgroup', 'col', 'thead', 'tbody', 'tfoot',
            'tr', 'td', 'th',
        }
        table_internal_displays = {
            'table-caption', 'table-column-group', 'table-column',
            'table-header-group', 'table-row-group', 'table-footer-group',
            'table-row', 'table-cell',
        }

        def _flattens_only_to_table_internal(candidate: DomNode) -> bool:
            if candidate.is_text:
                return False
            candidate_display = candidate.styles.get('display', '').strip().lower()
            if candidate_display == 'contents':
                generated = [child for child in candidate.children if not child.is_text]
                return bool(generated) and all(
                    _flattens_only_to_table_internal(child) for child in generated
                )
            return (
                candidate.tag in table_internal_tags
                or candidate_display in table_internal_displays
            )

        if node.tag == 'details' and not any(
            not child.is_text and child.tag == 'summary' for child in node.children
        ):
            # HTML supplies a generated "Details" summary when no authored
            # summary child exists. Keep it as a real list-item formatting
            # object so its disclosure marker, inline advance, and line box
            # participate in layout before the open details content.
            counter[0] += 1
            default_summary = f"n{counter[0]}"
            counter[0] += 1
            default_summary_text = f"n{counter[0]}"
            lines.extend([
                f"{ws}    let {default_summary} = doc.create_node(ElementTag::Summary);",
                f"{ws}    doc.node_mut({default_summary}).style = "
                f"ComputedStyle::for_anonymous_box(&doc.node({var}).style);",
                f"{ws}    doc.node_mut({default_summary}).style.display = Display::ListItem;",
                f"{ws}    doc.node_mut({default_summary}).style.list_style_type = "
                + (
                    "ListStyleType::DisclosureOpen;"
                    if 'open' in node.attrs
                    else "ListStyleType::DisclosureClosed;"
                ),
                f"{ws}    doc.node_mut({default_summary}).style.list_style_position = "
                "ListStylePosition::Inside;",
                f"{ws}    doc.node_mut({default_summary}).style.padding_left = Length::px(17.0);",
                f"{ws}    doc.node_mut({default_summary}).style.line_height = "
                f"LineHeight::Length(doc.node({default_summary}).style.font_size);",
                f"{ws}    doc.append_child({var}, {default_summary});",
                f"{ws}        let {default_summary_text} = doc.create_node(ElementTag::Text);",
                f"{ws}        doc.node_mut({default_summary_text}).style = "
                f"ComputedStyle::for_anonymous_box(&doc.node({default_summary}).style);",
                f"{ws}        doc.node_mut({default_summary_text}).text = Some(\"Details\".to_string());",
                f"{ws}        doc.append_child({default_summary}, {default_summary_text});",
            ])

        if node.tag == 'details' and node.pseudo_styles.get('details-content'):
            # CSS creates one ::details-content wrapper around every authored
            # child except the first summary. The pseudo is structural even
            # without generated `content`; its display and box decorations
            # apply to the wrapped subtree.
            first_summary = next((
                child for child in node.children
                if not child.is_text and child.tag == 'summary'
            ), None)
            if first_summary is not None:
                gen_node(
                    first_summary, var, indent + 1, node_font_size,
                    child_inherited, node_custom_props, node_zoom,
                    html_table_border,
                )
            details_content = emit_generated_pseudo('details-content', structural=True)
            for child in node.children:
                if child is first_summary:
                    continue
                gen_node(
                    child, details_content, indent + 1, node_font_size,
                    child_inherited, node_custom_props, node_zoom,
                    html_table_border,
                )
        elif parent_display == 'table-row':
            # A run of non-cell children in a table row generates one
            # anonymous table cell. Keeping text and inline descendants in a
            # single wrapper is observable through intrinsic column sizing.
            index = 0
            while index < len(node.children):
                child = node.children[index]
                child_display = child.styles.get('display', '').strip().lower()
                if child.tag in {'td', 'th'} or child_display == 'table-cell':
                    gen_node(
                        child, var, indent + 1, node_font_size,
                        child_inherited, node_custom_props, node_zoom,
                        html_table_border,
                    )
                    index += 1
                    continue

                run = []
                while index < len(node.children):
                    candidate = node.children[index]
                    candidate_display = candidate.styles.get('display', '').strip().lower()
                    if candidate.tag in {'td', 'th'} or candidate_display == 'table-cell':
                        break
                    run.append(candidate)
                    index += 1
                counter[0] += 1
                anonymous_cell = f"n{counter[0]}"
                lines.append(
                    f"{ws}    let {anonymous_cell} = doc.create_node(ElementTag::Div);"
                )
                lines.append(
                    f"{ws}    doc.node_mut({anonymous_cell}).style = "
                    f"ComputedStyle::for_anonymous_box(&doc.node({var}).style);"
                )
                lines.append(
                    f"{ws}    doc.node_mut({anonymous_cell}).style.display = "
                    "Display::TableCell;"
                )
                lines.append(f"{ws}    doc.append_child({var}, {anonymous_cell});")
                for run_child in run:
                    gen_node(
                        run_child, anonymous_cell, indent + 2, node_font_size,
                        child_inherited, node_custom_props, node_zoom,
                        html_table_border,
                    )
        elif parent_display in {
            'table-header-group', 'table-row-group', 'table-footer-group'
        }:
            # Row-group children first generate anonymous rows; within each
            # row, consecutive improper content generates one anonymous cell.
            index = 0
            while index < len(node.children):
                child = node.children[index]
                child_display = child.styles.get('display', '').strip().lower()
                if child.tag == 'tr' or child_display == 'table-row':
                    gen_node(
                        child, var, indent + 1, node_font_size,
                        child_inherited, node_custom_props, node_zoom,
                        html_table_border,
                    )
                    index += 1
                    continue

                run = []
                while index < len(node.children):
                    candidate = node.children[index]
                    candidate_display = candidate.styles.get('display', '').strip().lower()
                    if candidate.tag == 'tr' or candidate_display == 'table-row':
                        break
                    run.append(candidate)
                    index += 1

                counter[0] += 1
                anonymous_row = f"n{counter[0]}"
                lines.append(
                    f"{ws}    let {anonymous_row} = doc.create_node(ElementTag::Div);"
                )
                lines.append(
                    f"{ws}    doc.node_mut({anonymous_row}).style = "
                    f"ComputedStyle::for_anonymous_box(&doc.node({var}).style);"
                )
                lines.append(
                    f"{ws}    doc.node_mut({anonymous_row}).style.display = "
                    "Display::TableRow;"
                )
                lines.append(f"{ws}    doc.append_child({var}, {anonymous_row});")

                run_index = 0
                while run_index < len(run):
                    run_child = run[run_index]
                    run_display = run_child.styles.get('display', '').strip().lower()
                    if run_child.tag in {'td', 'th'} or run_display == 'table-cell':
                        gen_node(
                            run_child, anonymous_row, indent + 2, node_font_size,
                            child_inherited, node_custom_props, node_zoom,
                            html_table_border,
                        )
                        run_index += 1
                        continue

                    cell_run = []
                    while run_index < len(run):
                        candidate = run[run_index]
                        candidate_display = candidate.styles.get('display', '').strip().lower()
                        if candidate.tag in {'td', 'th'} or candidate_display == 'table-cell':
                            break
                        cell_run.append(candidate)
                        run_index += 1
                    counter[0] += 1
                    anonymous_cell = f"n{counter[0]}"
                    lines.append(
                        f"{ws}        let {anonymous_cell} = "
                        "doc.create_node(ElementTag::Div);"
                    )
                    lines.append(
                        f"{ws}        doc.node_mut({anonymous_cell}).style = "
                        f"ComputedStyle::for_anonymous_box("
                        f"&doc.node({anonymous_row}).style);"
                    )
                    lines.append(
                        f"{ws}        doc.node_mut({anonymous_cell}).style.display = "
                        "Display::TableCell;"
                    )
                    lines.append(
                        f"{ws}        doc.append_child({anonymous_row}, {anonymous_cell});"
                    )
                    for cell_child in cell_run:
                        gen_node(
                            cell_child, anonymous_cell, indent + 3, node_font_size,
                            child_inherited, node_custom_props, node_zoom,
                            html_table_border,
                        )
        elif parent_display in {'table', 'inline-table'} and node.tag != 'table':
            # CSS table fixup groups each consecutive run of improper direct
            # children into one anonymous row/cell. Keeping the run intact is
            # essential for inline sequences such as `text<br>text`, which are
            # one cell rather than three adjacent columns.
            index = 0
            while index < len(node.children):
                child = node.children[index]
                child_display = child.styles.get('display', '').strip().lower()
                if (
                    child.tag in table_internal_tags
                    or child_display in table_internal_displays
                    or _flattens_only_to_table_internal(child)
                ):
                    gen_node(
                        child, var, indent + 1, node_font_size,
                        child_inherited, node_custom_props, node_zoom,
                        html_table_border,
                    )
                    index += 1
                    continue

                run = []
                while index < len(node.children):
                    candidate = node.children[index]
                    candidate_display = candidate.styles.get('display', '').strip().lower()
                    if (
                        candidate.tag in table_internal_tags
                        or candidate_display in table_internal_displays
                        or _flattens_only_to_table_internal(candidate)
                    ):
                        break
                    run.append(candidate)
                    index += 1
                counter[0] += 1
                anonymous_row = f"n{counter[0]}"
                counter[0] += 1
                anonymous_cell = f"n{counter[0]}"
                lines.append(
                    f"{ws}    let {anonymous_row} = "
                    "doc.create_node(ElementTag::Div);"
                )
                lines.append(
                    f"{ws}    doc.node_mut({anonymous_row}).style = "
                    f"ComputedStyle::for_anonymous_box(&doc.node({var}).style);"
                )
                lines.append(
                    f"{ws}    doc.node_mut({anonymous_row}).style.display = "
                    "Display::TableRow;"
                )
                lines.append(f"{ws}    doc.append_child({var}, {anonymous_row});")
                lines.append(
                    f"{ws}        let {anonymous_cell} = "
                    "doc.create_node(ElementTag::Div);"
                )
                lines.append(
                    f"{ws}        doc.node_mut({anonymous_cell}).style = "
                    f"ComputedStyle::for_anonymous_box("
                    f"&doc.node({anonymous_row}).style);"
                )
                lines.append(
                    f"{ws}        doc.node_mut({anonymous_cell}).style.display = "
                    "Display::TableCell;"
                )
                for prop in sorted(INHERITED_PROPS):
                    if prop not in child_inherited:
                        continue
                    inherited_code = generate_single_style(
                        prop,
                        child_inherited[prop],
                        f"doc.node_mut({anonymous_cell}).style",
                        node_font_size,
                    )
                    for generated_line in (
                        inherited_code if isinstance(inherited_code, list)
                        else [inherited_code] if inherited_code else []
                    ):
                        lines.append(f"{ws}        {generated_line}")
                lines.append(
                    f"{ws}        doc.append_child({anonymous_row}, {anonymous_cell});"
                )
                for run_child in run:
                    gen_node(
                        run_child, anonymous_cell, indent + 2, node_font_size,
                        child_inherited, node_custom_props, node_zoom,
                        html_table_border,
                    )
        elif parent_display in {'', 'block', 'flow-root', 'list-item'}:
            # CSS Display table fixup wraps a consecutive run of internal
            # table boxes found directly in a block formatting context in one
            # anonymous table. `display:contents` nodes have already lost their
            # principal box, so classify their flattened children here while
            # retaining the inheritance boundary in `gen_node` itself.
            anonymous_table = None
            ordered_children = render_children
            if node.tag == 'details':
                first_summary = next((
                    child for child in node.children
                    if not child.is_text and child.tag == 'summary'
                ), None)
                if 'open' not in node.attrs:
                    # Closed details exposes only its first summary; all other
                    # children remain in the DOM but generate no boxes.
                    ordered_children = [first_summary] if first_summary is not None else []
                elif first_summary is not None:
                    # The HTML details shadow tree slots its first authored
                    # summary before the details-content slot, independent of
                    # the summary's DOM position. Preserve that rendered order
                    # even when no explicit ::details-content rule generated a
                    # structural wrapper above.
                    ordered_children = [first_summary] + [
                        child for child in node.children if child is not first_summary
                    ]
            for child in ordered_children:
                child_parent = var
                child_indent = indent + 1
                if _flattens_only_to_table_internal(child):
                    if anonymous_table is None:
                        counter[0] += 1
                        anonymous_table = f"n{counter[0]}"
                        lines.append(
                            f"{ws}    let {anonymous_table} = "
                            "doc.create_node(ElementTag::Div);"
                        )
                        lines.append(
                            f"{ws}    doc.node_mut({anonymous_table}).style = "
                            f"ComputedStyle::for_anonymous_box(&doc.node({var}).style);"
                        )
                        lines.append(
                            f"{ws}    doc.node_mut({anonymous_table}).style.display = "
                            "Display::Table;"
                        )
                        lines.append(
                            f"{ws}    doc.append_child({var}, {anonymous_table});"
                        )
                    child_parent = anonymous_table
                    child_indent = indent + 2
                else:
                    anonymous_table = None
                gen_node(
                    child, child_parent, child_indent, node_font_size,
                    child_inherited, node_custom_props, node_zoom,
                    html_table_border,
                )
        else:
            for child in render_children:
                gen_node(
                    child, var, indent + 1, node_font_size,
                    child_inherited, node_custom_props, node_zoom,
                    html_table_border,
                )
        if node.tag == 'q':
            emit_generated_quote('\u201d')

        emit_generated_pseudo('column-scroll-marker')
        emit_generated_pseudo('scroll-marker')
        # `insert_after_sibling` reverses creation order. Emit each directional
        # pair end-before-start so the resulting CSS tree order is
        # start-before-end for both physical and logical axes.
        for button_name in (
            'scroll-button-down', 'scroll-button-up',
            'scroll-button-right', 'scroll-button-left',
            'scroll-button-block-end', 'scroll-button-block-start',
            'scroll-button-inline-end', 'scroll-button-inline-start',
        ):
            emit_generated_pseudo(button_name)
        apply_current_scroll_marker_state()
        if effective_styles.get('scroll-marker-group', '').strip().lower() == 'after':
            marker_group = emit_generated_pseudo('scroll-marker-group', structural=True)
            if marker_group is not None and not establishes_scroll_container():
                lines.append(
                    f"{ws}doc.node_mut({marker_group}).style.display = Display::None;"
                )
        emit_generated_pseudo('after')

    # Process body children
    # Apply body-level styles if any
    root_font_size = 16.0
    body_inherited = {
        prop: html_styles[prop]
        for prop in sorted(EXPLICIT_INHERIT_PROPS | TEXT_EXTRA_INHERITED)
        if root_aware and prop in html_styles
    }
    body_custom_props = {}
    html_zoom = 1.0
    if root_aware and html_styles:
        computed_html_styles = _copy_declarations(html_styles)
        if is_real_font_profile():
            for prop, val in list(computed_html_styles.items()):
                if val.strip() == 'inherit' and prop in REAL_FONT_INITIALS:
                    computed_html_styles[prop] = REAL_FONT_INITIALS[prop]
        html_zoom = _effective_css_zoom(computed_html_styles)
        html_style_lines, _html_font_size = generate_style_code(
            computed_html_styles, 'html', 16.0, html_zoom
        )
        for sl in html_style_lines:
            lines.append(f"    {sl}")
    body_zoom = html_zoom
    if root.styles:
        for prop, val in root.styles.items():
            if prop.startswith('--'):
                body_custom_props[prop] = val
        computed_root_styles = _copy_declarations(root.styles)
        for prop in sorted(_SP17_INHERITED_PROPERTIES):
            if prop in body_inherited and prop not in computed_root_styles:
                computed_root_styles[prop] = body_inherited[prop]
        if is_real_font_profile():
            for prop in sorted(INHERITED_PROPS):
                if prop in body_inherited and prop not in computed_root_styles:
                    computed_root_styles[prop] = body_inherited[prop]
            for prop, val in list(computed_root_styles.items()):
                if val.strip() == 'inherit' and prop in REAL_FONT_INITIALS:
                    computed_root_styles[prop] = body_inherited.get(
                        prop, REAL_FONT_INITIALS[prop]
                    )
        body_zoom = _effective_css_zoom(computed_root_styles, html_zoom)
        body_styles, root_font_size = generate_style_code(
            computed_root_styles, 'vp', 16.0, body_zoom
        )
        for sl in body_styles:
            lines.append(f"    {sl}")
        if RETAIN_TEXT and not is_real_font_profile():
            lines.append(
                '    doc.node_mut(vp).style.font_family = '
                f'{DETERMINISTIC_FONT_FAMILY_RUST};'
            )
            lines.append(
                "    doc.node_mut(vp).style.list_style_type = ListStyleType::None;"
            )
        if RETAIN_TEXT and root.styles.get('display', '').strip() == 'contents':
            # `base_doc()` models <body> as a concrete child of the viewport.
            # A display:contents body instead contributes its children directly
            # to the document-element formatting context. Hide the synthetic
            # body box below and attach its already-computed descendants to the
            # viewport root; this also gives their line boxes the correct
            # document-element strut while retaining body inheritance.
            lines.append("    doc.node_mut(vp).style.display = Display::None;")
        if root_aware:
            for prop in sorted(INHERITED_PROPS):
                if prop in body_inherited and prop not in root.styles:
                    inherited_line = generate_single_style(
                        prop, body_inherited[prop], 'doc.node_mut(vp).style', root_font_size
                    )
                    if inherited_line:
                        for sl in (
                            inherited_line
                            if isinstance(inherited_line, list)
                            else [inherited_line]
                        ):
                            lines.append(f"    {sl}")
        # Collect inherited props from body for propagation to children
        body_inherit_props = EXPLICIT_INHERIT_PROPS | (
            TEXT_EXTRA_INHERITED if RETAIN_TEXT else set()
        )
        for prop in sorted(body_inherit_props):
            if prop in computed_root_styles:
                body_inherited[prop] = computed_root_styles[prop]
        if is_real_font_profile():
            if 'font-size' in body_inherited:
                body_inherited['font-size'] = f'{root_font_size}px'
            if 'line-height' in body_inherited:
                body_inherited['line-height'] = _computed_inherited_line_height(
                    body_inherited['line-height'], root_font_size
                )
        clipped_text_color = _background_text_color(root.styles)
        if clipped_text_color:
            body_inherited['color'] = clipped_text_color
        if RETAIN_TEXT:
            computed_radius = _computed_border_radius(root.styles)
            if computed_radius is not None:
                body_inherited.update(computed_radius)
        if 'font' in root.styles:
            _fam = _family_from_font_shorthand(root.styles['font'])
            if _fam:
                body_inherited['font-family'] = _fam
            # A shorthand supplies the body's computed font size even though
            # the parsed declaration is retained as `font`. Generated nodes do
            # not run inheritance, so descendants need the absolute computed
            # value just as ordinary element descendants do in `gen_node`.
            body_inherited['font-size'] = f'{root_font_size}px'
            body_inherited['line-height'] = _line_height_from_font_shorthand(
                root.styles['font']
            )

    body_is_contents = (
        RETAIN_TEXT and root.styles.get('display', '').strip() == 'contents'
    )
    body_parent = (
        'html' if body_is_contents and root_aware
        else 'doc.root()' if body_is_contents
        else 'vp'
    )

    def lower_static_picture_sources(node: DomNode):
        """Select unconditional single-candidate ``picture`` resources.

        Responsive source selection needs media, type, sizes, and device-pixel
        evaluation.  A source without those conditions and with one literal
        candidate is deterministic at generation time, however, and supplies
        the nested image's replaced resource exactly like a literal ``src``.
        """
        if node.tag == 'picture':
            selected = None
            for child in node.children:
                if (
                    child.tag == 'source'
                    and not child.attrs.get('media', '').strip()
                    and not child.attrs.get('type', '').strip()
                ):
                    srcset = child.attrs.get('srcset', '').strip()
                    if ',' not in srcset:
                        parts = srcset.split()
                        if len(parts) == 1 or (len(parts) == 2 and parts[1] == '1x'):
                            selected = parts[0]
                            break
            if selected:
                for child in node.children:
                    if child.tag == 'img' and not child.attrs.get('src', '').strip():
                        child.attrs['src'] = selected
        for child in node.children:
            if not child.is_text:
                lower_static_picture_sources(child)

    lower_static_picture_sources(root)
    for child in root.children:
        gen_node(
            child, body_parent, 1, root_font_size,
            body_inherited, body_custom_props, body_zoom,
        )

    # The parser's synthetic body is not the origin of ``:root`` generated
    # boxes. Materialize document-element pseudos separately after descendants
    # exist, so root marker groups can collect their complete marker cohort.
    root_pseudo_styles = getattr(root, 'html_pseudo_styles', {})
    root_origin = 'html' if root_aware else 'doc.root()'
    # In the compact document shape `vp` is the synthetic body. Root pseudos
    # inherit from the document element, not from body declarations (including
    # the deterministic body font override used by the comparison harness).
    root_style_source = 'html' if root_aware else 'doc.root()'
    if not root_aware and html_styles.get('scroll-marker-group', '').strip().lower() in {
        'before', 'after'
    }:
        lines.append(
            "    let root_scroll_marker_group = "
            "doc.node(vp).style.scroll_marker_group;"
        )
        lines.append(
            "    doc.node_mut(doc.root()).style.scroll_marker_group = "
            "root_scroll_marker_group;"
        )
        lines.append(
            "    doc.node_mut(vp).style.scroll_marker_group = ScrollMarkerGroup::None;"
        )

    def emit_root_generated_pseudo(
        name: str, kind: str, *, structural: bool = False
    ) -> str | None:
        pseudo_styles = _copy_declarations(root_pseudo_styles.get(name, {}))
        root_custom_props = {
            prop: value for prop, value in html_styles.items()
            if prop.startswith('--')
        }
        for prop, value in list(pseudo_styles.items()):
            if isinstance(value, str):
                pseudo_styles[prop] = _resolve_css_vars(value, root_custom_props)
        content_value = pseudo_styles.get('content')
        if not structural and (
            content_value is None
            or content_value.strip().lower() in ('normal', 'none')
        ):
            return None
        materialization_required[0] = True
        counter[0] += 1
        pseudo_var = f"n{counter[0]}"
        lines.append(
            f"    let {pseudo_var} = doc.insert_pseudo_element("
            f"{root_origin}, openui_dom::PseudoElementKind::{kind});"
        )
        lines.append(
            f"    let {pseudo_var}_style = "
            f"ComputedStyle::for_pseudo(&doc.node({root_style_source}).style);"
        )
        lines.append(f"    doc.node_mut({pseudo_var}).style = {pseudo_var}_style;")
        if name.startswith('scroll-button-'):
            emit_passive_scroll_button_defaults(
                pseudo_var,
                '    ',
                disabled=False,
                system_font=not any(
                    prop in pseudo_styles for prop in {'font', 'font-family'}
                ),
            )
        pseudo_lines, _ = generate_style_code(
            pseudo_styles, pseudo_var, root_font_size, html_zoom
        )
        for pseudo_line in pseudo_lines:
            lines.append(f"    {pseudo_line}")
        if (
            name.startswith('scroll-button-')
            and pseudo_styles.get('appearance', '').strip().lower() == 'none'
        ):
            emit_appearance_none_button_defaults(pseudo_var, '    ')
        if name.startswith('scroll-button-') and any(
            prop == 'border'
            or prop.startswith('border-')
            or prop in {'background', 'background-color', 'background-image'}
            for prop in pseudo_styles
        ):
            for corner in ('top_left', 'top_right', 'bottom_right', 'bottom_left'):
                lines.append(
                    f"    doc.node_mut({pseudo_var}).style.border_{corner}_radius = "
                    "(0.0, 0.0);"
                )
        if name == 'scroll-marker-group':
            if 'display' not in pseudo_styles:
                lines.append(
                    f"    doc.node_mut({pseudo_var}).style.display = Display::Block;"
                )
            lines.append(
                f"    doc.node_mut({pseudo_var}).style.contain |= Containment::LAYOUT;"
            )
        return pseudo_var

    for button_name, direction in (
        ('scroll-button-down', 'Down'),
        ('scroll-button-up', 'Up'),
        ('scroll-button-right', 'Right'),
        ('scroll-button-left', 'Left'),
        ('scroll-button-block-end', 'BlockEnd'),
        ('scroll-button-block-start', 'BlockStart'),
        ('scroll-button-inline-end', 'InlineEnd'),
        ('scroll-button-inline-start', 'InlineStart'),
    ):
        emit_root_generated_pseudo(
            button_name,
            f"ScrollButton(openui_dom::ScrollButtonDirection::{direction})",
        )
    if html_styles.get('scroll-marker-group', '').strip().lower() in {'before', 'after'}:
        emit_root_generated_pseudo(
            'scroll-marker-group', 'ScrollMarkerGroup', structural=True
        )

    if materialization_required[0]:
        lines.append("    doc.materialize_generated_content();")
    lines.append("    doc")
    lines.append("}")

    generated = engineify_module('\n'.join(lines))
    if getattr(root, 'lowered_layout_barriers', []):
        needle = "    Ok(doc.into_engine())"
        replacement = (
            "    let mut engine = doc.into_engine();\n"
            "    // The audited layout reads do not influence later control flow.\n"
            "    // Flush the final Engine state through its geometry/scene API.\n"
            "    let _ = engine.update()?;\n"
            "    Ok(engine)"
        )
        if generated.count(needle) != 1:
            raise ValueError('could not lower the audited Engine layout barrier')
        generated = generated.replace(needle, replacement, 1)
    return generated


def _outer_html_sections(source: str):
    """Return outer-document style blocks, body markup, and body attributes.

    Unlike a regular expression, this small tokenizer does not interpret tag
    text inside a quoted ``srcdoc`` attribute as part of the outer document.
    It is intentionally used only for documents that carry embedded markup so
    historical templates remain byte-identical.
    """
    tokens = []
    index = 0
    while index < len(source):
        if source.startswith('<!--', index):
            end = source.find('-->', index + 4)
            index = len(source) if end < 0 else end + 3
            continue
        if source[index] != '<':
            index += 1
            continue
        match = re.match(r'<\s*(/?)\s*([a-zA-Z][-_a-zA-Z0-9]*)\b', source[index:])
        if match is None:
            index += 1
            continue
        quote = None
        cursor = index + match.end()
        while cursor < len(source):
            char = source[cursor]
            if quote is not None:
                if char == quote:
                    quote = None
            elif char in ('"', "'"):
                quote = char
            elif char == '>':
                break
            cursor += 1
        if cursor >= len(source):
            break
        tokens.append((
            index,
            cursor + 1,
            match.group(2).lower(),
            bool(match.group(1)),
            source[index:cursor + 1],
        ))
        index = cursor + 1

    styles = []
    body_markup = None
    body_attrs = None
    for token_index, token in enumerate(tokens):
        start, end, name, closing, raw = token
        if name == 'style' and not closing:
            for candidate in tokens[token_index + 1:]:
                if candidate[2] == 'style' and candidate[3]:
                    styles.append(source[start:candidate[1]])
                    break
        if name == 'body' and not closing and body_markup is None:
            body_attrs = raw[raw.lower().find('body') + 4:-1]
            for candidate in tokens[token_index + 1:]:
                if candidate[2] == 'body' and candidate[3]:
                    body_markup = source[end:candidate[0]]
                    break
    return styles, body_markup, body_attrs


def _embedded_text_override(markup: str) -> str:
    """Inject the deterministic text harness into a nested document."""
    override = TEXT_TEMPLATE_OVERRIDE.replace(
        'body, body *', 'html, body, body *', 1
    )
    head_end = re.search(r'</head\s*>', markup, re.IGNORECASE)
    if head_end:
        return markup[:head_end.start()] + override + markup[head_end.start():]
    return override + markup


def generate_html_template(
    html_path: str,
    *,
    root_aware: bool = False,
    final_state_parser: WptHtmlParser | None = None,
) -> str:
    """Read an HTML file and extract body content + style blocks for Chrome rendering.
    The template must include <style> blocks so Chrome applies the same CSS rules
    that the Rust code generator parsed and encoded into Document builder code.
    Handles external <link rel="stylesheet"> by inlining their content.
    """
    with open(html_path, 'r', encoding='utf-8-sig', errors='replace') as f:
        content = f.read()

    # Structural tag-like text inside HTML comments is inert. Keep the
    # original source for stylesheet extraction, but use comment-free markup
    # when locating html/body/link tags and extracting the rendered body.
    # Otherwise a comment mentioning `<body>` can be mistaken for the real
    # opening tag and leak the remainder of the comment into the screenshot.
    markup_content = re.sub(r'<!--.*?-->', '', content, flags=re.DOTALL)

    html_dir = os.path.dirname(os.path.abspath(html_path))

    final_state = (
        final_state_parser.lowered_final_state
        if final_state_parser is not None else None
    )
    if final_state_parser is not None and final_state is None:
        raise ValueError('final-state template requires a lowered mutation IR')

    has_embedded_markup = bool(re.search(r'\bsrcdoc\s*=', content, re.IGNORECASE))
    embedded_body = None
    embedded_body_attrs = None
    if final_state is not None:
        style_blocks = []
        for attrs, style_text, _prefix in final_state['author_style_blocks']:
            encoded_attrs = _serialize_mutation_attrs(attrs, None)
            style_blocks.append(f'<style{encoded_attrs}>{style_text}</style>')
    elif has_embedded_markup:
        style_blocks, embedded_body, embedded_body_attrs = _outer_html_sections(content)
    else:
        # Extract <style> blocks from anywhere in the document (head or body)
        style_blocks = re.findall(
            r'<style[^>]*>.*?</style>', content, re.DOTALL | re.IGNORECASE
        )

    # Preserve inline <body style="..."> declarations in the Chrome template.
    # The Rust generator applies parsed body styles to `vp`; without this,
    # Chromium comparisons silently use the harness default body style instead.
    # Attribute values may themselves contain ``>`` (the check-layout corpus
    # commonly uses selectors such as ``div > div`` in body/onload).  A plain
    # ``[^>]*`` stops inside the quoted value and leaves the remainder as
    # visible body text in the comparison template.
    body_tag = r'<body\b((?:[^>"\']+|"[^"]*"|\'[^\']*\')*)>'
    body_open = re.search(body_tag, markup_content, re.IGNORECASE)
    body_attrs = (
        None if final_state is not None
        else embedded_body_attrs if has_embedded_markup
        else body_open.group(1) if body_open else None
    )
    if body_attrs is not None:
        style_attr = re.search(r'\bstyle=["\']([^"\']*)["\']', body_attrs, re.IGNORECASE)
        if style_attr:
            style_blocks.append(f"<style>body {{{style_attr.group(1)}}}</style>")
        onload_attr = re.search(
            r'''\bonload\s*=\s*(["'])(.*?)\1''',
            body_attrs, re.IGNORECASE | re.DOTALL,
        )
        if onload_attr:
            class_add = re.fullmatch(
                r'''\s*document\.body\.classList\.add\(\s*(["'])([-_a-zA-Z][-_a-zA-Z0-9]*)\1\s*\)\s*;?\s*''',
                onload_attr.group(2),
            )
            if class_add:
                class_name = re.escape(class_add.group(2))
                style_blocks = [
                    re.sub(rf'(?i)\bbody\.{class_name}\b', 'body', block)
                    for block in style_blocks
                ]

    if root_aware and final_state is None:
        html_open = re.search(r'<html\b([^>]*)>', markup_content, re.IGNORECASE)
        if html_open:
            style_attr = re.search(
                r'\bstyle=["\']([^"\']*)["\']', html_open.group(1), re.IGNORECASE
            )
            if style_attr:
                style_blocks.append(f"<style>html {{{style_attr.group(1)}}}</style>")

    # Inline external stylesheets referenced by <link rel="stylesheet">
    for m in re.finditer(
        r'<link[^>]*rel=["\']?stylesheet["\']?[^>]*>', markup_content, re.IGNORECASE
    ):
        href_match = re.search(r'href=["\']([^"\']+)["\']', m.group(0))
        if href_match:
            href = href_match.group(1)
            css_path = os.path.join(html_dir, href)
            if os.path.isfile(css_path):
                with open(css_path, 'r', encoding='utf-8-sig', errors='replace') as f:
                    css_text = f.read()
                # Strip -webkit- prefixed duplicates to keep styles clean
                css_text = re.sub(r'\s*-webkit-[a-z-]+:\s*[^;]+;\n?', '', css_text)
                style_blocks.insert(0, f'<style>{css_text}</style>')

    style_prefix = '\n'.join(style_blocks)

    # Extract body content
    body_match = None if has_embedded_markup or final_state is not None else re.search(
        # Use the last body end tag. The HTML parser ignores an early stray
        # `</body>` when later body content follows, so stopping at the first
        # token would make the Chromium harness render a different document
        # from both the browser's source parse and the generated DOM.
        body_tag + r'(.*)</body\s*>',
        markup_content,
        re.DOTALL | re.IGNORECASE,
    )
    if final_state is not None:
        body = final_state['body_markup']
    elif embedded_body is not None:
        body = embedded_body
    elif body_match:
        body = body_match.group(2)
    else:
        # No explicit body — use content after meta/link tags
        body = markup_content
        # Remove DOCTYPE, html, head, meta, link, title, script tags (NOT style)
        body = re.sub(r'<!DOCTYPE[^>]*>', '', body, flags=re.IGNORECASE)
        body = re.sub(r'<html[^>]*>|</html>', '', body, flags=re.IGNORECASE)
        # Remove <head> but preserve <style> blocks (already extracted above)
        body = re.sub(r'<head[^>]*>.*?</head>', '', body, flags=re.DOTALL | re.IGNORECASE)
        # Attribute values may legally contain literal angle brackets. A
        # simple `[^>]*` stops at the first one inside a quoted assertion and
        # leaks the remainder of metadata into the visible test body.
        quoted_attributes = r'(?:[^>"\']+|"[^"]*"|\'[^\']*\')*'
        body = re.sub(
            rf'<link\b{quoted_attributes}>', '', body, flags=re.IGNORECASE
        )
        body = re.sub(
            rf'<meta\b{quoted_attributes}>', '', body, flags=re.IGNORECASE
        )
        body = re.sub(r'<title[^>]*>.*?</title>', '', body, flags=re.DOTALL | re.IGNORECASE)
        body = re.sub(r'<script[^>]*>.*?</script>', '', body, flags=re.DOTALL | re.IGNORECASE)
        # Remove outer style blocks from body (they're already extracted
        # above), while preserving literal style markup inside a quoted
        # `srcdoc` attribute.
        if has_embedded_markup:
            for style_block in style_blocks:
                body = body.replace(style_block, '', 1)
        else:
            body = re.sub(
                r'<style[^>]*>.*?</style>', '', body,
                flags=re.DOTALL | re.IGNORECASE,
            )

    # Strip instructional paragraphs before the text-preserving template pass.
    # HTML permits their end tag to be omitted before block content, so accept
    # either an explicit </p> or the first paragraph-closing start tag.
    paragraph_end = '|'.join(sorted(WptHtmlParser.P_IMPLICIT_END_TAGS))
    body = re.sub(
        rf'<p\b[^>]*>(?:(?!</?p\b).)*?Test passes(?:(?!</?p\b).)*?'
        rf'(?:</p\s*>|(?=<(?:{paragraph_end})\b))',
        '',
        body,
        flags=re.DOTALL | re.IGNORECASE,
    )

    # Strip ALL bare text nodes from the HTML body.  Our Rust layout engine
    # does not render text (cross-SP dependency), so leaving text in Chrome's
    # HTML causes a systematic Y-offset mismatch wherever descriptive text
    # precedes or sits between test elements.  We use an HTML parser to
    # selectively remove text while preserving element structure.
    from html.parser import HTMLParser
    import io

    # Extract tag names targeted by CSS rules in style blocks so that
    # the TextStripper keeps elements that have stylesheet-based styles
    # even when they lack inline style/class/id attributes.
    css_targeted_tags = set()
    for sb in style_blocks:
        # Strip the <style> wrapper
        inner = re.sub(r'<style[^>]*>|</style>', '', sb, flags=re.IGNORECASE)
        # Find bare tag selectors (e.g.  "p {" or "div {")
        for m in re.finditer(r'(?:^|[},;])\s*([a-zA-Z][a-zA-Z0-9]*)\s*\{', inner):
            css_targeted_tags.add(m.group(1).lower())
        # Preserve elements targeted by compound selectors too (`.box p`,
        # `main > div`). Dropping such an empty box changes paint geometry
        # even when its text node is intentionally stripped.
        for rule in re.finditer(r'([^{}]+)\{', inner):
            selector = re.sub(r'/\*.*?\*/', '', rule.group(1), flags=re.DOTALL)
            for m in re.finditer(
                r'(?:^|[\s>+~])([a-zA-Z][a-zA-Z0-9-]*)', selector
            ):
                css_targeted_tags.add(m.group(1).lower())

    class TextStripper(HTMLParser):
        """Remove text nodes and unstyled heading/p tags from HTML, preserving element structure.

        In SP14 text mode (RETAIN_TEXT) text is kept and wrapper elements are
        NOT unwrapped — only unstyled instructional headings are dropped
        (mirroring the Rust generator, which skips those subtrees). All other
        differences from Chrome UA defaults are neutralized by
        TEXT_TEMPLATE_OVERRIDE appended to the template.
        """
        # Tags to skip entirely (including children) when unstyled.
        # These are instructional headings in WPT tests with user-agent
        # default styling that our engine can't replicate.
        SKIP_UNSTYLED = {'h1', 'h2', 'h3', 'h4', 'h5', 'h6'}
        # Tags to unwrap (remove tag but keep children) when unstyled
        # AND not targeted by any CSS rule in the stylesheet.
        # These are instructional elements with UA default block-level
        # margins (p, ul, ol, dl) or list items (li, dt, dd) — keeping them
        # introduces a vertical Y-offset mismatch since OpenUI doesn't apply
        # UA stylesheet defaults.
        UNWRAP_UNSTYLED = {'p', 'ul', 'ol', 'li', 'dl', 'dt', 'dd'}
        # Attribute keys that count an element as "styled" for the purposes
        # of unwrap decisions. Note that id="testdetails" is a documented WPT
        # convention for instructional elements and is treated as unstyled.
        STYLED_ATTRS = {'style', 'class', 'id'}
        UNSTYLED_ID_VALUES = {'testdetails'}
        def __init__(self):
            super().__init__(convert_charrefs=False)
            self.out = io.StringIO()
            self.in_style = False
            self.skip_depth = 0  # >0 means we're inside a fully-skipped element
            self.unwrap_tags = []  # stack of unwrapped tags (to suppress end tag)

        def _is_unstyled(self, attrs):
            """Check if element has no style or class attributes; an id is OK
            only if it's in UNSTYLED_ID_VALUES (e.g. id="testdetails").
            """
            for k, v in attrs:
                if k == 'style' or k == 'class':
                    return False
                if k == 'id' and (v or '') not in self.UNSTYLED_ID_VALUES:
                    return False
            return True

        def handle_starttag(self, tag, attrs):
            if self.skip_depth > 0:
                self.skip_depth += 1
                return
            # Fully skip unstyled instructional headings, but preserve headings
            # that are actually targeted by the test stylesheet.
            if tag in self.SKIP_UNSTYLED and self._is_unstyled(attrs) and tag not in css_targeted_tags:
                self.skip_depth = 1
                return
            # Unwrap unstyled <p> only if no CSS rule targets the tag
            # (box mode only — text mode keeps wrappers so their text stays).
            if (not RETAIN_TEXT and tag in self.UNWRAP_UNSTYLED and self._is_unstyled(attrs)
                    and tag not in css_targeted_tags):
                self.unwrap_tags.append(tag)
                return
            attr_str = ''
            for k, v in attrs:
                if v is None:
                    attr_str += f' {k}'
                else:
                    if tag == 'iframe' and k == 'srcdoc' and RETAIN_TEXT:
                        v = _embedded_text_override(v)
                    attr_str += f' {k}="{html_module.escape(v, quote=True)}"'
            self.out.write(f'<{tag}{attr_str}>')
            if tag == 'style':
                self.in_style = True

        def handle_endtag(self, tag):
            if self.skip_depth > 0:
                self.skip_depth -= 1
                return
            # If this tag was unwrapped, just pop from stack
            if self.unwrap_tags and self.unwrap_tags[-1] == tag:
                self.unwrap_tags.pop()
                return
            self.out.write(f'</{tag}>')
            if tag == 'style':
                self.in_style = False

        def handle_startendtag(self, tag, attrs):
            # Preserve XHTML void elements as one HTML void element. The
            # HTMLParser default calls start+end, producing `<br></br>`; HTML5
            # parses that invalid pair as two breaks and diverges from the Rust
            # builder's single forced break.
            self.handle_starttag(tag, attrs)
            if tag not in WptHtmlParser.VOID_TAGS:
                self.handle_endtag(tag)

        def handle_data(self, data):
            if self.skip_depth > 0:
                return
            # Preserve text inside <style> tags (CSS rules)
            if self.in_style:
                self.out.write(data)
                return
            if RETAIN_TEXT:
                # SP14 text mode: keep text content (Chrome applies its own
                # CSS white-space collapsing; the Rust side mirrors it).
                self.out.write(data)
            # Otherwise drop all text content

        def handle_entityref(self, name):
            if self.skip_depth > 0:
                return
            self.out.write(f'&{name};')

        def handle_charref(self, name):
            if self.skip_depth > 0:
                return
            self.out.write(f'&#{name};')

    stripper = TextStripper()
    stripper.feed(body)
    body = stripper.out.getvalue()

    # Combine: style blocks first, then body content
    template = style_prefix + '\n' + body.strip() if style_prefix else body.strip()
    if RETAIN_TEXT and not is_real_font_profile():
        # Deterministic-font override LAST so it wins the cascade.
        template = template + '\n' + TEXT_TEMPLATE_OVERRIDE
    # All visual and embedded resources are lowered to immutable data URLs.
    # This is deliberately unconditional: the SP20 package is content
    # addressed, and historical hard-coded assets remain the fallback.
    template = _embed_paint_asset_urls(template, html_dir)
    if final_state is not None:
        def directive(name: str, value) -> str:
            payload = base64.urlsafe_b64encode(
                json.dumps(
                    value, sort_keys=True, separators=(',', ':')
                ).encode('utf-8')
            ).decode('ascii')
            return f'<!--OPENUI_{name}:{payload}-->'

        body_final_attrs = dict(final_state['body_attrs'])
        if final_state['body_style']:
            body_final_attrs['style'] = final_state['body_style']
        else:
            body_final_attrs.pop('style', None)
        html_final_attrs = dict(final_state['html_attrs'])
        if final_state['html_style']:
            html_final_attrs['style'] = final_state['html_style']
        else:
            html_final_attrs.pop('style', None)
        directives = [
            directive('FINAL_HTML_ATTRS', html_final_attrs),
            directive('FINAL_BODY_ATTRS', body_final_attrs),
            directive('FINAL_SCROLLS', final_state['scrolls']),
        ]
        template = '\n'.join(directives) + '\n' + template
    if root_aware:
        template = '<!--OPENUI_ROOT_AWARE-->\n' + template
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
            test_id = f"wpt/{prefix}/{filename}"
            parser = parse_wpt_html(
                str(html_path), mutation_ir=_candidate_mutation_ir(test_id)
            )
            portable, reason = analyze_portability(parser)

            if not portable:
                results['not_portable'].append((filename, reason))
                continue

            if not has_layout_content(parser):
                results['not_portable'].append((filename, "no_layout_content"))
                continue

            fn_name = f"{prefix}_{sanitize_fn_name(filename)}"
            rust_code = generate_rust_fn(fn_name, parser.root, parser.html_styles)
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
    lines.append("use openui_dom::ElementTag;")
    lines.append(
        "use openui_engine::{Engine, EngineError, NodeHandle as NodeId, RendererNodeState};"
    )
    lines.append("use openui_geometry::{Length, ViewportMetrics};")
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
        f.write(engineify_module('\n'.join(lines) + '\n'))

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
