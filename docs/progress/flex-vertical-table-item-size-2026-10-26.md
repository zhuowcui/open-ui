# Vertical table flex item size, 26 October 2026

`css_flexbox/ortho-table-item-001` was missing its whole 100×100 green square
at every profile.

## Finding

The test makes a `writing-mode: vertical-rl` table, 100×100, the only item
of a zero-width row flex container. The item has `min-width: 0`, so flexing
shrinks its main size (the table's block size) to 0. In Chrome the table
stays 100px wide: a table's block size is at least the extent of its rows.

Open UI's table layout already honoured that, and its rows were 100px wide.
After layout, though, the flex algorithm replaced the size of every vertical
item that had a fixed or stretched cross size with the flexed pair
(`axis_mapping.physical_size(final_main, cross_size_for_child)`). So the
table box was 0×100, and its green background, painted on that box, vanished.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A table wrapper flex item keeps its laid-out fragment size; only other vertical items take the projected flexed pair | `openui-layout` | `flex/algorithm.rs`, the final item layout |

## Measurement

Paired expanded census in WSL against the
[atomic inline clamp](atomic-inline-min-max-clamp-2026-10-26.md) run (clone
commits `a18118c1` and `33204b70`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 22,061 to 22,065 |
| Gained | 4 |
| Lost, improved, worsened | none |

`ortho-table-item-001` becomes exact at all four profiles. No other
comparison changes. Chromium is identical in every comparison. Rows are in the
[evidence](evidence/flex-vertical-table-item-size-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,061 to 22,065 exact; 4 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,928 passed, 0 failed |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,459 passed, 0 failed, 20 ignored |
