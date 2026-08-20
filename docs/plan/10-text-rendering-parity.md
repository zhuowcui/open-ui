# Text Rendering Parity — SP14+ Roadmap

The text engine, inline layout, and glyph painter already exist. The text track connects
them to the Chromium accountability corpus through deterministic text-retaining ports,
then expands from Ahem geometry to real-font and advanced-text parity.

## Current state

SP14 W0–W4 closed the global `needs_text` category, SP15 closed its inline/layout
and root/body follow-up, SP16 closed all 776 real-font-metric rows, and SP13-R
then closed every runnable multicol owner. The text manifest now contains 711
runner-scoped IDs after the first 20 SP17 admissions and corrected retained-text
references. The complete 3,586-test run has 3,287 exact passes, 299 functional
failures, and zero errors. It preserves the 2,823-ID SP13-R baseline plus all
351 exact SP13-R targets and passes the 7/7 audit.

Deterministic ports use repo-vendored Ahem. Real-font ports use vendored DejaVu Sans,
Sans Mono, and Serif plus the pinned Chromium 147 FreeType runtime. Each raster policy
is manifest-scoped on both renderers. Exact zero-pixel parity remains the standard;
AA near misses are never promoted to passes.

SP17 W0A reproduced that full snapshot without resume and froze all 3,267 exact
IDs, the original 842 writing-mode owner rows, and the 19 runnable kickoff
outcomes. W0B then froze a faithful porter disposition of 311 actionable and
531 residual rows without changing generated Rust builders or pixel evidence.
W1A added the shared constraint/logical-geometry foundation and preserved all
3,267 exact IDs at `0.0%`. W1B routes normal block, atomic-inline, and flex-item
child spaces through it, converts flex placement through logical axes, and
admits `flexbox-writing-mode-001` at exact parity while retaining every kickoff
pass. W1C corrects final flex placement direction propagation, covers all
horizontal-tb/RTL flex-flow reversals, and admits companion 004 at exact parity.
W1D projects vertical flex main/cross sizes into physical fragments, covers all
four vertical writing-mode/direction matrices, and admits companions 002, 003,
005, and 006 at exact parity. W1E centralizes the flex container/child axis
mapping and admits 14 orthogonal sizing, alignment, intrinsic, aspect-ratio,
wrapping, and overflow-padding targets at exact parity.

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

Cover bidi/RTL, vertical writing modes, transformation, decoration, emphasis,
complex scripts, and emoji. W0A and W0B are complete on the fresh-main branch:
the original 842-row inventory is frozen as 311 actionable and 531 residual
rows after transactional CSS handling and a real builder probe. W1E now has an
authoritative constraint direction, shared logical style/edge projection, and
shared normal block/atomic-inline/flex child boundaries. The exact actionable
set now covers 001–009 plus 11 named orthogonal flex sizing cases. Continue
with the remaining vertical flex families, then out-of-flow positions and
fragmentation/multicol. Keep 010–015 for W2 vertical glyph shaping/paint.
Follow `docs/SP17-PLAN.md`.

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
