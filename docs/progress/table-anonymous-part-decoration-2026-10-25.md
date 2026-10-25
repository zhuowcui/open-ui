# Table decoration with anonymous parts, 25 October 2026

A survey of deterministic census failures found a family that differs by one
whole 100×100 square at every profile (10,000 pixels at 1×, 40,000 at 2×).
In each test, Chrome paints a green `display: table` box, and Open UI paints
nothing there.

## Finding

Open UI's layout is correct: the table fragment has the right size and
position, but its background is never painted. Every affected table has a
cell, or block content, as a direct child, so layout creates an anonymous
section and row. These anonymous parts borrow the table's node id.

Block layout marks a fragment as a structural wrapper, and skips its
decoration, when the fragment has a child box with the same node
(`openui-layout/src/block.rs`). This exists for captioned tables, where the
same-node grid box owns the background and border. The anonymous row matched
that rule, so the table fragment set `skip_box_decoration`. The row paints
into cells, and an anonymous cell paints no table background, so the table
background was lost.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Same-node table sections, rows and cells no longer make their table a structural wrapper; a same-node grid box still does | layout | `openui-layout/src/block.rs` |

## Measurement

Paired expanded census in WSL against `main`'s rendering (clone commits
`d85cd626` and `accfd7e6`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 21,701 to 21,750 |
| Gained | 49 |
| Lost, worsened, improved | none |

Twelve tests become exact at all four profiles:

- `table_break-after-table-cell` and `-child`
- `table_break-before-table-cell` and `-child`
- `table-as-item-fixed-min-width-2` and `-3`
- `table-as-item-min-content-height-1` and `-2` (tentative)
- `table-as-item-min-height-1`
- `table-as-item-stretch-cross-size-4`
- `line-clamp_line-clamp-032`
- `stretch_fixed-table-1`

The 49th gain is `table_section-with-overflow-000` at 1.25×. Chromium is
identical in every comparison. Rows are in the
[evidence](evidence/table-anonymous-part-decoration-census-v1.json).

Other members of the 10,000-pixel family have other owners. For example,
`table_specified-block-size-007`'s row backgrounds do not paint into its
fragmented anonymous cells. They are unchanged.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 21,701 to 21,750 exact; 49 gains, 0 losses |
| Layout unit tests | `cargo test --locked -p openui-layout --lib` | 746 pass |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,459 passed, 0 failed, 20 ignored |
