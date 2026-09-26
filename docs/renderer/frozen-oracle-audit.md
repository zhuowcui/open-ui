# Frozen Chromium oracle discrepancy

The SP20 historical result files call 5,731 comparisons passes. Those records
were produced by `tools/accountability/pixel_diff.py`, whose default permits a
per-channel delta of 4 and compares only 785 of the 800 viewport columns.
Every historical result reports 471,000 compared pixels (785 × 600). Of the
5,731 passes, 186 explicitly report a nonzero maximum channel delta. Thus
their `mismatched_pixels: 0` and `mismatch_pct: 0.0` fields do not prove exact
RGBA equality. The generated kickoff baseline preserves those historical
records by hash; it must not be read as a fresh zero-tolerance result.

The immutable Open UI PNG archive is separately useful: replaying it checks
whether the current renderer can still produce those same bytes. Replay does
not compare the bytes with Chromium. The qualification matrix uses zero
tolerance and a pinned Chrome-for-Testing oracle, so it must report its own
exact count. The two gates are independent and neither may be silently
substituted for the other.

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

The development workstation also retains ignored historical Chromium PNGs
and `test.html` files. An optional comparison of those local files found 188
archived Open UI images different from their then-current Chromium capture,
162 historical fixture documents different from the current fixture document,
and 15 historical Chromium captures different from the fresh oracle. All 15
changed captures belong to changed fixtures; none changed when the fixture
bytes stayed the same. These ignored local files are diagnostic inputs, not
part of the immutable archive or a release qualification source.

Run `python3 tools/accountability/audit_frozen_oracle.py` to verify the
archive hash and all historical metadata. Pass `--matrix-report` with a
complete, clean-source four-profile census to compare every frozen Open UI
image against the live Chromium oracle and current renderer. The
`--prior-local-captures` option adds the local historical-capture comparison
when those ignored files are available. The
`--require-original-exact` option is a CI gate and exits nonzero while the
historical exactness conflict remains. A complete diagnostic report may be
written as `docs/renderer/generated/frozen-oracle-audit-v1.json`; it is
evidence, not a qualification override.

The original archive, Chromium oracle, and 5,731-case manifest remain
immutable. The release gate stays blocked until the historical claim is
reconciled through reviewed evidence and the current four-profile matrix
itself reaches 22,924 exact comparisons with zero errors.
