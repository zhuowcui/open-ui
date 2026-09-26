# Font selection and shaping contract

Open UI resolves fonts within the `FontCollection` owned by an engine/document.
For each authored family it considers application faces before the system font
manager. Application-face matching is lexicographic in CSS order: style,
stretch, weight, Unicode coverage, then registration order. Platform fallback
uses the computed BCP47 language and must cover the complete extended grapheme
cluster before it can replace a missing glyph run.

Variable faces are cloned with Skia `FontArguments`. CSS-derived `wght`,
`wdth`, `ital`/`slnt`, and automatic `opsz` coordinates are installed first;
explicit `font-variation-settings` values replace coordinates with the same
tag. Values are clamped to the face's advertised axes and unsupported axes are
ignored. `font-synthesis-weight` and `font-synthesis-style` gate synthetic bold
and oblique. Registered face size/metric overrides affect the same platform
font data consumed by shaping, intrinsic sizing, line boxes, and paint.

Color faces use the ordinary Skia glyph path for COLR/CPAL, bitmap, sbix, and
SVG glyphs. `normal`, CPAL light/dark flags, and native custom palette resources
all become Skia palette arguments; custom entry overrides are immutable
document resources and invalidate cached font instances on registration or
removal.

All shaping uses Unicode script and bidi run iterators plus the computed BCP47
language (or the explicit language-system override), including fallback runs.
Face feature defaults precede variant-derived and explicit CSS features.
Vertical upright/mixed runs enable `vert` and `vrt2`.

Automatic hyphenation is resolved from the document-owned
`HyphenationRegistry`. The deterministic US-English dictionary is bundled.
Applications can register bounded UTF-8 Knuth-Liang dictionaries for other
BCP47 locales; a missing dictionary means no automatic hyphenation.
