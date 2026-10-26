# Orthogonal atomic inline min-content, 26 October 2026

`intrinsic-percent-replaced-029.tentative` and `-030.tentative` were missing a
whole 100×100 green square at every profile.

## Finding

Each test puts a `writing-mode: vertical-rl` canvas inside a
`width: min-content` div. The canvas has `block-size: 100%` (`-029`) or
`max-block-size: 50%` (`-030`). Chrome sizes the div to the canvas's natural
100px (200px in `-030`), then resolves the percentage against it. Open UI
sized the div to 0, so the canvas collapsed to 0×0.

The canvas's own contribution was already right: 100px for both endpoints.
The div's inline formatting context measures it as an atomic inline item,
though (`InlineItemsBuilder`). For an orthogonal atomic item, that path
probed the completed fragment for max-content and hard-coded min-content to
zero. The min-content div therefore got zero width.

An orthogonal box contributes its block size to both endpoints of its
container's inline size. Blink measures it once and reports that size as both
min and max (`ComputeMinAndMaxContentContribution`).

## Change

| Change | Owner | Source |
| --- | --- | --- |
| An orthogonal atomic inline uses its probed block size as its min-content contribution, as well as its max-content | `openui-layout` | `inline/items_builder.rs`, the atomic item's intrinsic endpoints |

## Measurement

Expanded census in WSL of `392e3d52`, paired with the
[flex break](flex-row-break-propagation-2026-10-26.md) run `806adda3`, sharing
one Chromium cache. That base predates PR 33, so the 16 table comparisons that
PR 33 changed are excluded from the lists below.

| Result | Comparisons |
| --- | --- |
| Gained | 8 |
| Lost, improved, worsened | none |
| Raw run totals | 22,035 to 22,057 exact, including PR 33's 14 gains |

`intrinsic-percent-replaced-029.tentative` and `-030.tentative` become exact
at all four profiles. No other comparison changes. Chromium is identical in
every comparison. Rows are in the
[evidence](evidence/orthogonal-atomic-min-content-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 8 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 0 failures |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,459 passed, 0 failed, 20 ignored |
