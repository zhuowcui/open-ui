# Text Rendering Parity — SP14+ Roadmap

The text engine, inline layout, and glyph painter already exist. The text track connects
them to the Chromium accountability corpus through deterministic text-retaining ports,
then expands from Ahem geometry to real-font and advanced-text parity.

## Current state

SP14 W0–W4 closed the global `needs_text` category, SP15 closed its inline/layout
and root/body follow-up, and SP16 closed all 776 real-font-metric rows. The text
manifest remains 496 IDs. The complete 3,566-test run has 2,823 exact passes and
zero errors, preserves all 2,804 frozen SP16 baseline passes, and passes the 7/7 audit.

Deterministic ports use repo-vendored Ahem. Real-font ports use vendored DejaVu Sans,
Sans Mono, and Serif plus the pinned Chromium 147 FreeType runtime. Each raster policy
is manifest-scoped on both renderers. Exact zero-pixel parity remains the standard;
AA near misses are never promoted to passes.

## Chronological work

### SP14 W3/W4 — unported text closure

Complete. The 111 deterministic cases were ported transactionally (52 exact, 59 named
functional owners), all 3,934 residuals record their actual porter rejection plus merged
upstream ownership, and the global text detector was retired after coverage was proven.

### SP15 — inline layout, line breaking, and root/body propagation

Complete. All 130 original owner rows are frozen in 76 actionable and 54 residual
ledgers. Decorated inline continuations, semantic clearing breaks, `display:contents`
inheritance/style handling, and root/body canvas and overflow propagation are implemented.
The actionable set finishes 34 exact, 42 functionally owned, and zero errors.

### SP16 — real-font metrics and parity

Complete. The 226 runnable targets finish 19 exact and 207 functionally owned with
zero errors; 550 unported residuals retain their actual rejection and non-font owners.
Shared font metrics, `ch`/`ex`/`lh`, used line height, shorthand parsing, deterministic
family selection, and the real-glyph LCD raster profile are implemented. The global
`needs_font_metrics` category is retired.

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
