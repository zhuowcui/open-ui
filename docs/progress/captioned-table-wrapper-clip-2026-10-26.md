# Captioned table wrapper clip, 26 October 2026

A triage of structural census failures found the fragmented table border
tests (`table_table-border-002` to `-004`). At the top of the first column,
Open UI shows the red multicol background where Chrome shows the green
caption.

## Finding

A table with captions has a wrapper box around a separately decorated table
grid. Layout gives the wrapper no border: the grid owns the table's border
(`FragmentTablePart::GridDecoration`). Open UI's fragment geometry was
correct, with the caption at 0–50 and the grid from 50.

The wrapper's clip during painting came from `compute_clip_rect`, though. That
function first takes the scroll-area client rectangle retained by layout,
which derives from `ScrollArea::source_border`. For a grid-delegating wrapper
that returns the grid's border. So the first fragmentation slice of the
wrapper clipped its children 10px below the slice top, which cut off the top
of the caption above the grid.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A table wrapper with a `GridDecoration` child clips at its own border box: the retained client rectangle and the delegated grid border no longer inset it | `openui-paint` | `painter.rs` `compute_clip_rect` |

## Measurement

Paired expanded census in WSL against `main`'s rendering (clone commits
`ed948ac8` and `86506085`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 21,992 to 22,021 |
| Gained | 29 |
| Improved | 15 |
| Lost, worsened | none |

The gains:

- `table_table-border-002` to `-004` and `table_repeated-section_header-after-break` become exact at all four profiles.
- `table_table-grid-paint-*` and `table_table-collapsed-borders-paint-*` (seven tests) become exact at 1× and 2×.

Those seven improve at 1.25× and 1.5× (for example, `table-grid-paint-htb-ltr`
goes from 2,832 to 1,332 pixels at 1.25×). Chromium is identical in every
comparison. Rows are in the
[evidence](evidence/captioned-table-wrapper-clip-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 21,992 to 22,021 exact; 29 gains, 0 losses |
| Paint unit tests | `cargo test --locked -p openui-paint --lib` | 252 pass |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,459 passed, 0 failed, 20 ignored |
