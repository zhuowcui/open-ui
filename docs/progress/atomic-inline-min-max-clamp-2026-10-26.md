# Atomic inline min and max in intrinsic sizing, 26 October 2026

`intrinsic-size-min-content-box-sizing` failed at every profile (2,520 pixels
at 1×).

## Finding

The test puts an inline-block with `width: min-content`, `min-width: 200px`,
`padding: 10px` and `box-sizing: border-box` inside a shrink-to-fit
inline-block wrapper. The inner box laid out at 200px, but the wrapper sized
itself to the inner box's 40px min-content width. The inner box overflowed,
and the wrapper's border was drawn around the wrong width.

The wrapper measures its inline formatting context through the atomic inline
item (`InlineItemsBuilder`). That item stored only the child's preferred
content size. The child's own `min-width` and `max-width` never bounded it. In
Blink, an atomic inline's contribution is its preferred size clamped by its
min and max sizes (`ComputeMinAndMaxContentContribution`).

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A fixed `min-width` or `max-width` (or the height pair, in vertical writing) bounds both intrinsic endpoints of a parallel atomic inline item, in content-box terms. Percentages and intrinsic keywords are left to the existing paths | `openui-layout` | `inline/items_builder.rs`, the atomic item's intrinsic endpoints |

## Measurement

Paired expanded census in WSL against the
[orthogonal min-content](orthogonal-atomic-min-content-2026-10-26.md) run
(clone commits `392e3d52` and `a18118c1`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 22,057 to 22,061 |
| Gained | 4 |
| Lost, improved, worsened | none |

`intrinsic-size-min-content-box-sizing` becomes exact at all four profiles.
No other comparison changes. Chromium is identical in every comparison. Rows
are in the [evidence](evidence/atomic-inline-min-max-clamp-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,057 to 22,061 exact; 4 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,928 passed, 0 failed |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,459 passed, 0 failed, 20 ignored |
