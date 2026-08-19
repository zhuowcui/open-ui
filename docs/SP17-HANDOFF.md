# SP17 Handoff — Advanced Text and Writing Modes

This is the starting contract for the next parity agent after PR #1 lands on
`main`. SP17 owns advanced text behavior: vertical and sideways writing modes,
logical-axis layout, bidi controls, text orientation, transformation,
decoration/emphasis placement, complex scripts, and emoji. SP18 continues to
own generated content, first-line/first-letter, counters/quotes, text shadow,
and overflow ellipsis.

## Start from the landed state

Do not continue on `001-complete-sp12-parity`. Start from the merge of PR #1:

```bash
git switch main
git pull --ff-only origin main
git status --short --branch
git switch -c agent/sp17-advanced-text
python3 tools/accountability/audit.py
```

The expected starting accountability state is:

| Metric | Value |
|---|---:|
| Inventory | 7,673 |
| Runnable | 3,566 |
| Exact passes | 3,267 |
| Functional failures | 299 |
| Errors | 0 |
| Unported | 4,107 |
| Runnable `needs_writing_mode` | 19 |
| Unported `needs_writing_mode` | 823 |

SP17 must preserve every exact pass, including the immutable 2,823-ID SP13-R
baseline and all 351 SP13-R exact targets. The only passing threshold is zero
mismatched pixels.

Before freezing SP17 ledgers, run the complete WPT suite without resume and
confirm these counts. If `main` has changed them, use the verified new counts
and document the delta instead of copying this snapshot.

## Frozen inventory to create

At SP17 start, add a generator and Python tests following the SP13-R/SP16
patterns. Freeze:

1. every exact pass at the start of SP17 (expected: 3,267);
2. the complete original `needs_writing_mode` inventory (842 rows);
3. the runnable/actionable target set after transactional porter probing; and
4. the remaining unported rows with Chromium path, actual first porter
   rejection, and complete non-writing-mode owner set.

The 842-row inventory currently divides into 19 runnable and 823 unported rows.
Of the unported rows, 337 stop directly on `writing-mode` (255 style-block
rejections and 82 declaration rejections), while 486 stop first on another
unsupported feature. Ninety unported rows currently have writing mode as their
sole functional owner. Treat 337 as the first porter opportunity, not as a
guaranteed target count: regenerate transactionally and record the actual
outcome.

Do not remove or weaken the `needs_writing_mode` detector at sprint start. It
can be retired only after the frozen 842-row inventory is a disjoint,
reason-backed cover and no runnable row still needs that owner.

## First runnable slice

Run these 19 IDs without resume before implementation and retain their initial
results as evidence:

```text
wpt/css2_floats/floats-placement-007
wpt/css_flexbox/align-content-wrap-004
wpt/css_flexbox/auto-height-with-flex
wpt/css_flexbox/flex-basis-011-ref
wpt/css_flexbox/flex-wrap-002
wpt/css_flexbox/flex-wrap-003
wpt/css_flexbox/flex-wrap-004
wpt/css_flexbox/flexbox-align-self-vert-002
wpt/css_flexbox/flexbox-align-self-vert-004
wpt/css_flexbox/flexbox-align-self-vert-rtl-002
wpt/css_flexbox/flexbox-align-self-vert-rtl-004
wpt/css_flexbox/flexbox-align-self-vert-rtl-005
wpt/css_flexbox/flexbox-baseline-empty-001b
wpt/css_flexbox/flexbox-justify-content-horiz-006-ref
wpt/css_flexbox/stretch-obeys-min-max-003
wpt/css_position/position-absolute-center-001
wpt/css_position/static-position_htb-rtl-ltr.tentative
wpt/css_position/static-position_htb-rtl-rtl
wpt/css_position/sticky_position-sticky-margins-002
```

Only `auto-height-with-flex` is currently sole-owned. The others overlap with
inline-block, empty-block margin collapse, complex borders, positioned inline
layout, absolute flex static positioning, generated content, or sticky layout.
SP17 should remove genuine writing-mode differences and leave any remaining
pixel failure assigned to those concrete owners.

## Implementation order

### W0 — accountability and transactional CSS

- Create sorted immutable baseline, inventory, target, and residual ledgers.
- Add parsing/cascade coverage for `writing-mode`, `direction`, `unicode-bidi`,
  `text-orientation`, and the SP17 text properties used by the corpus.
- Move `writing-mode` and `unicode-bidi` out of porter rejection lists only
  after their builders preserve inheritance, shorthand/cascade order, and
  invalid-declaration behavior.
- Prove builder generation is idempotent and does not alter non-SP17 modules.

### W1 — authoritative logical geometry

- Thread `WritingDirectionMode` through constraint spaces and layout results.
- Make block, inline, flex, fragmentation, and out-of-flow layout operate in
  logical inline/block coordinates and convert once at fragment boundaries.
- Handle vertical-rl, vertical-lr, sideways-rl, and sideways-lr sizing,
  min/max constraints, margins, borders, padding, overflow, static positions,
  baselines, and scrollbar sides.
- Keep physical fragment geometry authoritative for paint; paint must not
  infer or recompute layout axes.

### W2 — bidi and vertical text

- Connect the existing UAX #9 `BidiParagraph` runs to inline item construction,
  line breaking, visual ordering, caret-independent fragment placement, and
  glyph painting.
- Implement `unicode-bidi` normal/embed/isolate/bidi-override/isolate-override/
  plaintext behavior with inherited `direction`.
- Use `font_orientation(style.writing_mode, style.text_orientation)` during
  shaping and place upright, mixed, and sideways glyph runs with correct
  advances and baselines.
- Cover text-combine-upright, emphasis marks, decorations, transforms, complex
  scripts, and emoji only through shared text/layout/paint behavior.

### W3 — porter expansion and co-owner cleanup

- Probe the 337 direct writing-mode rejections first, then compatible rows
  rejected by `unicode-bidi`, `transform`, images, containment, or other
  owners as their dependencies permit.
- Process sole-owned rows before co-owned rows.
- Re-run every actionable ID without resume. Promote only exact 0.0% results;
  retain specific non-SP17 owners for functional residuals.

### W4 — closure

- Require complete disjoint coverage of the frozen 842-row inventory.
- Require every starting exact ID to remain exact and zero render/diff errors.
- Require no runnable row to contain `needs_writing_mode`.
- Run the complete WPT suite without resume, regenerate mapping/deferred/report/
  status artifacts twice, require byte-identical output, and pass audit 7/7.

## Existing foundations and real gaps

Useful foundations already exist:

- `bindings/rust/openui-style/src/enums.rs` defines `WritingMode`,
  `TextOrientation`, `UnicodeBidi`, and `Direction`.
- `bindings/rust/openui-geometry/src/writing_mode.rs` implements logical to
  physical converters.
- `bindings/rust/openui-text/src/bidi/` implements UAX #9 paragraph analysis.
- `bindings/rust/openui-text/src/font/description.rs` derives font orientation.
- `bindings/rust/openui-paint/src/emphasis_painter.rs` contains horizontal and
  vertical emphasis placement helpers.
- `bindings/rust/openui-layout/tests/wpt_writing_mode_tests.rs` has extensive
  helper-level coverage.

The principal gap is integration: production block/flex/inline layout barely
consumes `style.writing_mode`, and the helper tests do not prove pixel-correct
vertical flow. Start by tracing `ConstraintSpace` creation and fragment
construction in `openui-layout/src/block.rs`, `flex/algorithm.rs`, and
`inline/`, then carry the same resolved writing direction into out-of-flow
layout and paint.

Porter/classification hotspots:

- `tools/wpt/port_wpt.py` rejection lists, style emission, and inherited
  property threading;
- `tools/wpt/splice_text_port.py` for surgical module updates; and
- `tools/accountability/shared_detectors.py` for the single authoritative
  owner mapping.

Do not batch-regenerate historical WPT modules. Preserve the SP16 real-font and
Ahem runner profiles for overlapping IDs.

## Canonical verification

```bash
cd bindings/rust
cargo build --release --package pixel-compare
cargo test --locked \
  --package openui-style \
  --package openui-text \
  --package openui-layout \
  --package openui-paint
cd ../..

# Focused runs overwrite summary.json. Snapshot it first.
LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" \
  python3 -u tools/accountability/run_all_pixel_comparisons.py \
  --ids-file tools/accountability/data/wpt_ported/<sp17-target-ledger>.json

LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" \
  python3 -u tools/accountability/run_all_pixel_comparisons.py 'wpt/'

python3 tools/accountability/generate_wpt_mapping.py
python3 tools/accountability/generate_sp12_5_csv.py
python3 tools/accountability/audit.py
```

Never use per-test substitutions, geometry exceptions, pixel offsets, or
raster-setting overrides. Fix shared standards behavior and keep every
remaining failure visibly owned.
