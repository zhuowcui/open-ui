# Text Rendering Parity — SP14+ Roadmap

The text engine, inline layout, and glyph painter already exist. The text track connects
them to the Chromium accountability corpus through deterministic text-retaining ports,
then expands from Ahem geometry to real-font and advanced-text parity.

## Current state

SP14 W0–W4 closed the global `needs_text` category. The text manifest contains 445
tests: 93 exact and 352 with detector-backed non-text owners. The complete 3,517-test
run has 2,767 exact passes and zero errors, preserves all 2,715 frozen baseline passes,
and passes the 7/7 audit. The former 4,045-row unported text backlog is split into 111
runnable W3 ports and 3,934 structured W4 dispositions.

Deterministic ports use repo-vendored Ahem plus an explicit DejaVu Sans fallback for
verified missing glyphs. The no-AA/no-hinting environment is scoped by
`text_ported_tests.json` on both renderers. Exact zero-pixel parity remains the standard;
AA near misses are never promoted to passes.

## Chronological work

### SP14 W3/W4 — unported text closure

Complete. The 111 deterministic cases were ported transactionally (52 exact, 59 named
functional owners), all 3,934 residuals record their actual porter rejection plus merged
upstream ownership, and the global text detector was retired after coverage was proven.

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
