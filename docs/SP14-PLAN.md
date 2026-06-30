# SP14 — Text Rendering Foundation

> First SP of the text track. See `docs/plan/10-text-rendering-parity.md` for the full
> chronological roadmap and rationale.

## Overview

Wire real text end-to-end through the **accountability pixel pipeline** and reach **exact
Chromium pixel parity** on the simplest deterministic slice: **single-line, left-to-right,
horizontal, Ahem-font** text.

This is a *porting + parity* SP, not an engine build. The engine (`openui-text` shaping +
metrics + bidi), inline layout (`openui-layout/src/inline`), the glyph painter
(`openui-paint/src/text_painter.rs`), and the DOM text node (`ElementTag::Text`) already
exist. The missing link is that `tools/wpt/port_wpt.py` emits **box-only** builders, so text
is never exercised against Chromium. SP14 closes that link on the deterministic Ahem subset.

## Why Ahem first

The Ahem font renders every glyph as an exact filled square aligned to the em box, so glyph
rasterization is deterministic and **0.0% pixel parity is achievable**. ~245 corpus tests use
Ahem. Single-line LTR horizontal Ahem text is the smallest end-to-end vertical slice that
proves the whole path under the strict 0.0% standard.

## In scope

- Extend `tools/wpt/port_wpt.py` to emit `ElementTag::Text` nodes with text content and the
  font properties needed to render them (`font-family`, `font-size`, `color`), **gated to the
  Ahem single-line subset**. Box-only behavior for all other tests is unchanged.
- A hand-picked **pilot set** of single-line LTR Ahem tests, ported and rendered.
- Fix parity bugs that surface in: Ahem metrics (ascent/descent/baseline), single-line box
  height, glyph x-advance/positioning, text color fill.
- Regenerate `wpt_mapping.csv` / `sp12_5_deferred.csv`; keep `audit.py` 7/7; reclassify
  newly-passing tests out of `needs_text`.

## Out of scope (later SPs)

- Multi-line / wrapping / `white-space` / `text-align` → SP15.
- Real (non-Ahem) fonts and font-metric units (`ch`/`ex`, `line-height: normal`) → SP16.
- Bidi/RTL, vertical writing modes, `text-transform`, decoration, emphasis → SP17.
- `::first-line` / `::first-letter`, `text-shadow`, `text-overflow`, generated content → SP18.

## Approach (incremental, accountable)

1. **Ahem availability** — locate the Ahem `.ttf` the pipeline ships and confirm both our
   renderer (`openui-text` font resolution / Skia font-mgr) and the headless-Chromium
   reference resolve it identically.
2. **End-to-end smoke test** — hand-build one Ahem single-line `ElementTag::Text` Document,
   render via `block_layout` + paint, compare to a Chromium render of the equivalent HTML;
   drive it to **0.0%**. This proves the path before any porting-tool changes.
3. **Extend `port_wpt.py`** — emit text nodes + font props for Ahem single-line tests only;
   verify existing box-only builders are byte-identical (no churn).
4. **Port the pilot batch** — small set; run focused pixel comparisons; fix shaping/metrics/
   paint parity bugs as they surface.
5. **Regression guard** — re-run `css_break`, `css_multicol`, `css_position` slices to prove
   the 2671 box-only passes are unchanged.
6. **Promote** — full `wpt/` run, regenerate artifacts, `audit.py` 7/7, independent verifier,
   commit + push.

## Font-strategy finding (SP14 execution) — use the shared DejaVu fallback, do NOT install Ahem

Investigation during SP14 step 1 found:

- **Ahem is not installed** on the environment; `fc-match "Ahem"` falls back to **DejaVu Sans**.
  Both our Skia `FontMgr::default()` and the headless-Chromium reference go through the same
  fontconfig, so both currently resolve `font-family: Ahem` → DejaVu Sans **identically**.
- **119 currently-passing tests reference Ahem** (they render box-only on our side and pass
  against a Chromium reference that used the Ahem→DejaVu fallback). **Installing Ahem globally
  would re-render those Chromium references with real Ahem square glyphs and regress up to 119
  passing tests** (our box-only builders would no longer match).

Decision: **SP14 uses the existing shared DejaVu fallback, not a real Ahem install.**
Determinism still holds because both sides use the identical font + Skia shaping. The
accountability standard is *parity with headless Chromium*, which the DejaVu-fallback
reference already encodes. Installing real Ahem (and re-rendering all 245 Ahem references +
rendering their text on our side) is a coordinated migration deferred to a later SP. The
smoke test (step 2) therefore targets DejaVu-fallback text, not real Ahem squares.

## Exit criteria

- `port_wpt.py` can emit text nodes for the Ahem subset, and the behavior is documented.
- The pilot single-line LTR Ahem tests pass at **0.0% pixel mismatch**.
- **Zero regressions** to the existing 2671 box-only passes.
- `audit.py` passes 7/7; mapping/deferred regenerated; newly-passing tests reclassified out
  of `needs_text`; an independent sub-agent confirms the exit conditions.

## Risks / unknowns

- **Anti-aliasing parity** — even Ahem edges can differ by a few AA pixels. Mitigate by
  starting with integer-aligned font sizes/positions; keep exact 0.0% as the bar and route
  AA-only deltas to the existing `near_miss_aa` bucket (tracked, not hidden).
- **Font resolution mismatch** — Ahem must resolve to the same face/metrics on both sides;
  verify before scaling the pilot.
- **Porting-tool churn** — text emission must be strictly gated so the existing 2671 passing
  box-only builders do not change.

## Status

**SP14 smoke test complete — the precise starting bug is identified.**

Ran existing text builders (`sp13/inline_*`, `first_letter_basic`, `mixed_block_inline`,
`line_breaking_normal_wrap`) through the real pixel pipeline (they use `ElementTag::Span` +
`.text`, with matching HTML templates). Findings:

- The path is wired (builders emit text; Chromium renders the text in the shared DejaVu
  fallback), and single-line cases report small mismatches (`inline_single_span` 0.36%,
  `inline_multiple_spans` 0.38%, `first_letter_basic` 0.33%).
- **But our engine paints zero glyph pixels.** `first_letter_basic` and
  `line_breaking_normal_wrap` render fully white; `mixed_block_inline` paints its background
  boxes (24000 px) but **no text**; `inline_single_span` renders nothing in the text row.
  The small mismatch % is only because a missing single text line is a small fraction of the
  800×600 canvas.
- The `openui-text` suite passes 186/186 (shaping/metrics are correct), so the gap is
  specifically in the **layout→paint glyph path inside `pixel-compare`'s `render_to_png`** —
  text is shaped and laid out but never rasterized to the canvas.

**SP14 first implementation task (revised):** make inline text fragments actually paint
glyphs in the pixel-compare render path, starting with a single-line block of text
(e.g. a `<div>Text</div>`), and drive `inline_single_span` / `first_letter_basic` toward
0.0%. Then proceed to the port-tool + pilot steps.

_To be regenerated as SP14 progresses (pilot pass count, audit result, commit SHAs)._
