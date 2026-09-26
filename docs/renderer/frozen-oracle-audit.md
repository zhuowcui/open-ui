# Historical Open UI archive versus Chromium

Chromium is the sole v0.2 pixel target. The old Open UI screenshots and SP20
pass records remain immutable so their provenance can be checked, but matching
those screenshots is not a release requirement. The v1 machine-readable report
predates this contract correction; its `blocked` status and
`both_required_pixel_gates_can_pass_on_same_image` field describe the retired
two-gate policy, not the current release decision.

The SP20 historical result files call 5,731 comparisons passes. Those records
were produced by `tools/accountability/pixel_diff.py`, whose default permits a
per-channel delta of 4 and compares only 785 of the 800 viewport columns.
Every historical result reports 471,000 compared pixels (785 × 600). Of the
5,731 passes, 186 explicitly report a nonzero maximum channel delta. Thus
their `mismatched_pixels: 0` and `mismatch_pct: 0.0` fields do not prove exact
RGBA equality. The generated kickoff baseline preserves those historical
records by hash; it must not be read as a fresh zero-tolerance result.

The immutable Open UI PNG archive is separately useful: replaying it measures
how the renderer changed over time. Replay does not compare with Chromium and
is diagnostic only. The qualification matrix uses zero tolerance and a pinned
Chrome-for-Testing oracle; only that comparison determines pixel parity.

The 106-byte native fixture for
`wpt/css_backgrounds/background-image-gradient-interpolation-repaint-ref`
is a compact reproducer:

```html
<div style="width: 100px; height: 100px; background-image: linear-gradient(in oklch, yellow, blue)"></div>
```

Its historical pass record reports `max_channel_diff: 1`. Comparing the
frozen Open UI PNG with the pinned Chromium capture at 800×600@1 reveals
4,348 differing pixels in a 100×99 region starting at (20, 20); every changed
channel differs by at most one. The historical and fresh Chromium captures
have the same decoded RGBA hash for this fixture. No reference image was
rewritten to obtain this result.

The current renderer differs from Chromium by 12 pixels on this fixture.
The archived Open UI RGBA hash is different from the pinned Chromium RGBA hash
for the same fixture and profile. That explains why preserving old output
would obstruct Chromium parity. The historical
[minimal evidence](generated/frozen-oracle-audit-v1.json) records the hashes,
capture and runner identities, bounds, and region count.

The complete fresh legacy-profile audit found 5,526 cases where archive,
renderer, and Chromium all agree. In 155 cases the current renderer equals
Chromium while the archive differs; in 23 the renderer equals the archive
while Chromium differs; in two the archive equals Chromium while the renderer
differs; and in 25 all three differ. Thus 203 archived images differ from the
live oracle, 182 from the current renderer, and 50 current renders from the
oracle. These counts come from decoded RGBA hashes over the same 5,731 IDs.

The development workstation also retains ignored historical Chromium PNGs
and `test.html` files. An optional comparison of those local files found 188
archived Open UI images different from their then-current Chromium capture,
162 historical fixture documents different from the current fixture document,
and 15 historical Chromium captures different from the fresh oracle. All 15
changed captures belong to changed fixtures; none changed when the fixture
bytes stayed the same. Two of the 188 image differences occur only in the
excluded right strip and consequently have a reported maximum channel delta
of zero. These ignored local files are diagnostic inputs, not
part of the immutable archive or a release qualification source.

Run `python3 tools/accountability/audit_frozen_oracle.py` to verify the
archive hash and all historical metadata. Pass `--example-report` with a
clean-source, one-fixture legacy-profile matrix report to reproduce the
minimal conflict. Pass `--matrix-report` with a
complete, clean-source four-profile census to compare every frozen Open UI
image against the live Chromium oracle and current renderer. The
`--prior-local-captures` option adds the local historical-capture comparison
when those ignored files are available. The CI historical audit verifies the
archive and metadata without requiring the current renderer to recreate old
pixels. The existing v1 report remains unmodified; fresh reports use schema
v2 and describe the archive/Chromium difference without a release-blocked
status.

From a clean checkout with the pinned Chromium and a freshly built
`pixel_compare` binary, the minimal report is reproducible with:

```sh
python3 tools/qualification/run_renderer_matrix.py --suite full \
  --test-id wpt/css_backgrounds/background-image-gradient-interpolation-repaint-ref \
  --profile legacy-800x600@1 \
  --pixel-compare bindings/rust/target/debug/pixel_compare \
  --cache-dir out/frozen-conflict-cache \
  --oracle-cache-dir out/frozen-conflict-oracle \
  --results-dir out/frozen-conflict
python3 tools/accountability/audit_frozen_oracle.py \
  --example-report out/frozen-conflict/full-summary.json \
  --image-cache-dir out/frozen-conflict-cache
```

The matrix command exits nonzero because the current renderer is not exact
on this example. Its report is diagnostic; the audit separately compares the
immutable archive with the verified Chromium capture.

The original archive, Chromium oracle, and 5,731-case manifest remain
immutable. This discrepancy is historical evidence, not a contradiction in
the release contract. v0.2 still fails Chromium qualification because the
complete four-profile census has 1,813 current renderer differences.
