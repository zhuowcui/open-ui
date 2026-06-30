# Text Rendering Parity — SP14+ Roadmap

Chronological text-track plan. We pause layout (SP12 complete in-scope; SP13
fragmentation/multicol at ~48 hard residuals) and pivot to **text**, the single largest
unlock of the Chromium WPT corpus.

## Why text, and why it is tractable

`needs_text` blocks **4045 unported** tests and **336 runnable failures**;
`needs_font_metrics` another **230 runnable / 553 unported**. It is the biggest lever by far.

Critically, text is **not** a from-scratch build:

- **Engine exists (SP11):** `bindings/rust/openui-text` — font resolution/metrics,
  HarfBuzz/Skia shaping, bidi (UAX#9), hyphenation, emoji, emphasis, transform; with
  `wpt_text_tests`, `wpt_font_tests`, `sp11_round21..30` suites.
- **Inline layout already shapes text:** `openui-layout/src/inline/algorithm.rs` calls
  `TextShaper`/`FontMetrics`/`shape_text()`; line-breaking, first-line, first-letter,
  initial-letter, text-combine modules exist.
- **Paint already renders glyphs:** `openui-paint` has `text_painter.rs`,
  `decoration_painter.rs`, `emphasis_painter.rs`.
- **DOM supports text:** `ElementTag::Text` + `NodeData.text`.

**The gap is the accountability pixel pipeline.** `tools/wpt/port_wpt.py` emits box-only
builders — there are currently **0 `ElementTag::Text` nodes** across all ported WPT tests, so
text is never compared against headless Chromium. The text track is therefore a
**porting + parity** effort: emit text, render it, drive it to pixel parity.

## Determinism strategy

- **Ahem first.** ~245 corpus tests use the **Ahem** font (exact filled-square glyphs), so
  pixel-exact (0.0%) parity is achievable. Start here.
- **Then real fonts.** Default is **DejaVu Sans** — the same font the headless-Chromium
  reference uses, both rendered through Skia — so real-text parity is also achievable,
  just harder (anti-aliasing/hinting).
- **Standard unchanged:** exact 0.0% mismatch is the bar; AA-only near-misses are tracked in
  the existing `near_miss_aa` bucket, never hidden.

## Chronological SPs

### SP14 — Text Rendering Foundation (Ahem, single-line, LTR)
Wire text end-to-end through the pixel pipeline; reach exact parity on single-line,
left-to-right, horizontal Ahem text. Detailed in `docs/SP14-PLAN.md`.
Exit: pilot single-line Ahem tests at 0.0%, zero regressions to the 2671 box-only passes,
audit 7/7, port tool can emit text.

### SP15 — Inline layout & line breaking
Multi-line text, soft/forced wraps, `white-space`, `text-align`, `line-height` (incl.
font-metric `normal`/unitless), `<br>`, `<span>` inline boxes, baseline `vertical-align`.
Targets much of `needs_inline_block` (108 runnable) and wrapping text tests.

### SP16 — Real-font metrics & parity (DejaVu Sans)
Move beyond Ahem to the default real font; `ch`/`ex` units, `font` shorthand,
`line-height: normal`. Drive `needs_font_metrics` (230 runnable / 553 unported) to parity.

### SP17 — Advanced text
Bidi/RTL runs, vertical writing modes + char-orientation, `text-transform`, decoration
(underline/overline/line-through), emphasis marks, complex-script shaping, emoji. Targets
text portions of `needs_writing_mode` (79 runnable) and the advanced `openui-text` modules.

### SP18 — Generated content & text pseudo/effects
`::first-line`, `::first-letter`, `text-shadow`, `text-overflow: ellipsis`, generated
content (counters/quotes). Targets `needs_generated_content` (31 runnable).

Boundaries are proposals; refine as each SP is reached. Images, gradients, grid, tables,
advanced selectors, and JavaScript remain deferred behind text.

## Accountability discipline (every SP)

Same pipeline as layout work:

1. Regenerate templates / port via `tools/wpt/port_wpt.py` (and any batch driver).
2. Build: `cd bindings/rust && cargo build --release --package pixel-compare`.
3. Run: `LD_LIBRARY_PATH=... python3 -u tools/accountability/run_all_pixel_comparisons.py 'wpt/'`
   (snapshot `summary.json` before any filtered run — it overwrites on partial runs).
4. Regenerate `wpt_mapping.csv` + `sp12_5_deferred.csv`; reclassify newly-passing tests out
   of `needs_text`/`needs_font_metrics`.
5. `python3 tools/accountability/audit.py` must pass 7/7.
6. Guard slices for zero regression before promoting; independent verifier before completion.
