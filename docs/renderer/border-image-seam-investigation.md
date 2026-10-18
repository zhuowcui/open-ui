# Border-image nine-slice seams

Chromium 147 is the pixel target. This investigation used the unchanged
Chromium oracle and the clean [v15 four-profile census](generated/four-profile-census-v15.json)
as its baseline. The source experiment below was a dirty diagnostic; it is not
qualification evidence.

## Shared residual

Six equivalent border-image cases (`border-image-017` through `-020` and
`border-image-slice-001` through `-002`) have identical diff signatures at
each failing profile. Each is exact at 800×600@1 and 375×667@2, but differs
by 249 pixels at 1280×720@1.25 and 225 pixels at 1920×1080@1.5. The source
is a 100×100 PNG whose outer strips are `(0,128,0)` green and center is
`(255,0,0)` red. The border uses four unequal widths and nine-slice repeat.

At 1.25 scale the mismatches form the slice joins: a full vertical line at
physical x=62 and horizontal lines at y=43 and y=106. The vertical line has
125 differing pixels. At `(62,41)`, Open UI stores `(63,97,0,255)` and
Chromium stores `(63,96,0,255)`; at their intersection `(62,43)`, Open UI
stores `(76,90,0,255)` and Chromium `(74,89,0,255)`. The six cases share
the same positions and channel deltas. This localizes the gap to border-image
patch joins and their physical sampling or coverage arithmetic. The exact
rounding step remains unresolved; the tests are still unowned in the strict
qualification ledger.

## Rejected layer experiment

A diagnostic wrapped all nine patches in one Skia save layer at the outer
border-image bounds. The idea was to combine adjacent slice coverage before
compositing with the document. A dirty debug `pixel_compare` binary was run
against all 89 `wpt/css_backgrounds/border-image*` IDs in the immutable
complete manifest at all four required profiles, 356 comparisons in total.
At clean base commit `6ed07ff8`, the temporary source edit added
`canvas.save_layer_alpha_f(Some(outer), 1.0)`
immediately before the nine-slice `for row in 0..3` loop in
`paint_border_image` and `canvas.restore()` immediately after it. With that
edit compiled into a debug `pixel_compare`, the selection and run were:

```sh
python3 - <<'PY'
import json
from pathlib import Path
ids = json.loads(Path('tools/qualification/manifests/complete-5731.json').read_text())
selected = [test_id for test_id in ids if test_id.startswith('wpt/css_backgrounds/border-image')]
Path('/dev/shm/openui-border-image-ids-v1.json').write_text(json.dumps(selected, indent=2) + '\n')
PY
python3 tools/qualification/run_renderer_matrix.py --suite full \
  --ids-file /dev/shm/openui-border-image-ids-v1.json \
  --pixel-compare bindings/rust/target/debug/pixel_compare \
  --cache-dir out/renderer-qualification-cache \
  --results-dir /dev/shm/openui-border-layer-diagnostic-v1 \
  --allow-dirty-diagnostics --jobs 8
```

The [versioned diagnostic index](generated/border-image-layer-diagnostic-v1.json)
records all 31 changed comparisons, source and binary identities, and the
complete diagnostic report's SHA-256
`6655467640095523ac865e5d0e5ba982c0aa97f4620169527ceeef75926320ca`.

The clean baseline had 305/356 exact. The layer produced 290/356 exact,
66 different, and zero errors: 15 previously exact comparisons regressed,
16 already-different comparisons worsened, and none improved. The six
equivalent seam cases remained different at 1.25; their 1.5-scale mismatch
counts increased from 225 to 300 each. Every Chromium oracle identity and
decoded hash matched the clean baseline. The layer edit was reverted and
never committed. It is not a release path.

The next repair must match the shared source sampling and adjacent patch
coverage rule while retaining the 305 exact border-image comparisons. A
reduced Engine-backed fixture and five-scale phase sweep are still needed
before assigning a reviewed root cause in the ownership ledger.
