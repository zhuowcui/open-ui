# Text Rendering Parity — SP14+ Roadmap

The text engine, inline layout, and glyph painter already exist. The text track connects
them to the Chromium accountability corpus through deterministic text-retaining ports,
then expands from Ahem geometry to real-font and advanced-text parity.

## Current state

SP14 W0–W2 closed runnable `needs_text` ownership. The text manifest contains 334
tests: 41 exact and 293 with detector-backed non-text owners. The complete 3,406-test
run has 2,715 exact passes and zero errors, preserves all 2,700 W1 baseline passes, and
passes the 7/7 audit. The remaining text backlog is 4,045 unported tests.

Deterministic ports use repo-vendored Ahem plus an explicit DejaVu Sans fallback for
verified missing glyphs. The no-AA/no-hinting environment is scoped by
`text_ported_tests.json` on both renderers. Exact zero-pixel parity remains the standard;
AA near misses are never promoted to passes.

## Chronological work

### SP14 W3/W4 — unported text closure

Process the 4,045 unported `needs_text` rows in increasing dependency coupling. Port
representable cases transactionally, reject unsupported content before any write, and
move residuals only to dependencies demonstrated by original upstream HTML. Retain the
global text detector until this inventory reaches zero.

### SP15 — inline layout and line breaking

Address functional residuals exposed by text ports: inline box decoration, clearing
breaks beside floats, inline-block interaction, wrapping, baseline alignment, and related
line construction behavior.

### SP16 — real-font metrics and parity

Move beyond deterministic Ahem geometry to font metrics, `ch`/`ex`, font shorthand,
`line-height: normal`, hinting, and real-glyph raster parity. Runnable font-metric
ownership currently covers 225 failures.

### SP17 — advanced text

Cover bidi/RTL, vertical writing modes, transformation, decoration, emphasis, complex
scripts, and emoji.

### SP18 — generated content and text effects

Cover first-line/first-letter behavior, counters and quotes, text shadow, and overflow
ellipsis.

## Accountability discipline

1. Use the surgical splice workflow; do not batch-regenerate committed WPT modules.
2. Snapshot the full summary before focused runs and run focused targets without resume.
3. Require zero mismatched pixels for every promoted deterministic pass.
4. Run the complete `wpt/` suite without resume before updating authoritative artifacts.
5. Regenerate mapping then deferred artifacts, require audit 7/7, and independently verify
   the milestone before completion.
