# Table part backgrounds in anonymous cells, 26 October 2026

A triage of structural census failures found `table_overflow-scroll-row` and
`table_overflow-scroll-section`. In Chrome the green row background fills both
columns. In Open UI it was missing: 8,500 pixels at 1× and 34,000 at 2×.

## Finding

The row of these tests holds content that is not a cell, so layout wraps it in
an anonymous cell. A fragment dump shows the row (`NodeId(5)`) with one child
cell whose node is `NodeId::NONE`.

Open UI paints a row or section background into each of its cells
(`paint_table_part_background_in_cell`). That function returned early for a
cell without a node, so no part background reached an anonymous cell.

Blink makes no such exception. `TableRowPainter::PaintTablePartBackgroundIntoCells`
paints the part's background into every cell fragment, and an anonymous cell
is an ordinary `LayoutTableCell` there
(`third_party/blink/renderer/core/paint/table_painters.cc`, tag
147.0.7727.50). `TableCellPainter::PaintBackgroundForTablePart` checks only
the cell's visibility and whether its background transfers to the view.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A table part paints its background into an anonymous cell. The cell inherits from the part that generated it, so the part's style and node stand in for the cell's. An anonymous cell has no border radius of its own and never supplies the canvas background | `openui-paint` | `painter.rs` `paint_table_part_background_in_cell` |

## Measurement

Paired expanded census in WSL against `main`'s rendering (clone commits
`86506085` and `034abe84`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 22,021 to 22,035 |
| Gained | 14 |
| Improved | 2 |
| Lost, worsened | none |

The gains:

- `table_overflow-scroll-row` and `table_overflow-scroll-section` become exact at all four profiles.
- `table_sections-and-captions-mixed-order` and `table_specified-block-size-007` become exact at 1×, 1.25× and 2×.

At 1.5× these two improve: from 16,122 to 264 pixels, and from 22,800 to 129.
Chromium is identical in every comparison. Rows are in the
[evidence](evidence/anonymous-cell-part-background-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,021 to 22,035 exact; 14 gains, 0 losses |
| Paint and layout tests | `cargo test --locked -p openui-paint -p openui-layout` | all pass |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,459 passed, 0 failed, 20 ignored |

The Windows workspace needs the CI job's `SKIA_GN_ARGS` and `RUST_MIN_STACK`.
A first run without them rebuilt Skia with default flags: one scrollbar pixel
in `source_layer_raster` changed, and two nested multicol tests overflowed the
stack. Both pass with the CI environment, and `main` fails the same way
without it.
