# Grid auto-placement starts from the first cell, 10 November 2026

Auto-placed grid items now start from the grid's first row and column, as
in Chromium, after the items with a definite position are placed. Before
this change, a definite item moved the auto-placement cursor past itself,
so the next auto-placed item could skip a whole row. The components track
found this in the shadcn card header: the action, at
`grid-column: 2; grid-row: 1 / span 2`, pushed the title and description
down one row, and the header was 52px tall instead of Chromium's 44px.

## Finding

`place_items` in `openui-layout/src/grid.rs` places items in the order of
the CSS Grid placement algorithm's steps: fully definite items first, then
items locked to a track of the auto-flow axis, then the rest. In sparse
packing it moved one shared cursor to the end of every item it placed,
whatever the step. Two related differences were in the same code:

- A row auto-flow item with a definite column searched down from the
  cursor's row, even when its column was behind the cursor.
- Items locked to a row each searched from the first column, not from the
  end of the previous item locked to that row.

Chromium 147 follows the specification in
`GridPlacement::RunAutoPlacementAlgorithm`
(`third_party/blink/renderer/core/layout/grid/grid_placement.cc`):

1. `PlaceNonAutoGridItems` places the fully definite items.
2. `PlaceGridItemsLockedToMajorAxis` places items locked to a track of the
   auto-flow axis. With sparse packing, each such track keeps its own
   cursor (`minor_cursors`), at the end of the last item placed in it.
3. Step 4 creates a new `AutoPlacementCursor` at the first placed position,
   that is the first row and column, and places the rest in order. For an
   item with a definite position in the other axis,
   `AutoPlacementCursor::MoveToMinorLine` moves the cursor to the next
   track of the auto-flow axis when that position is behind the cursor.
   Each placed item leaves the cursor at its own end.

These sources were read at 147.0.7727.24, the local source checkout, which
is on the same branch as the pinned 147.0.7727.50.

The pinned Chromium was then measured.
`tools/qualification/capture_grid_placement_oracle.py` lays out four grids
with fixed-size items in the pinned Windows Chrome-for-Testing
147.0.7727.50 and records each item's box relative to its grid
([oracle](evidence/grid-placement-cursor-v1/chromium-windows.json)):

| Case | Chromium | Open UI before |
| --- | --- | --- |
| Card header: title, description, action at `grid-column: 2; grid-row: 1 / span 2`, `gap: 8px` | title y 0, description y 24 | title y 8, description y 32 |
| Row flow, three 50px columns: `a` at `grid-column: 2`, then `b` at `grid-column: 1` | `b` at x 0, y 20 | `b` at x 0, y 0 |
| Row flow: `x` at row 1 column 2, `l1` at row 1 spanning 2, `l2` at row 1 | `l2` at x 200 | `l2` at x 0 |
| Column flow, two 20px rows: `d` at row 2 column 1, then two auto items | auto items at x 0 and x 50, both y 0 | x 50 y 0 and x 50 y 20 |

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Items placed in steps 1 and 2 no longer move the step 4 cursor. | `openui-layout` | `bindings/rust/openui-layout/src/grid.rs` |
| In sparse packing, an item locked to a track of the auto-flow axis starts after the last item locked to that track. | `openui-layout` | same |
| A step 4 item with a definite position behind the cursor starts on the next track of the auto-flow axis. | `openui-layout` | same |

Dense packing is unchanged: it already restarted from the first cell for
every item.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Chromium oracle | `python tools/qualification/capture_grid_placement_oracle.py --chrome E:\open-ui\out\chrome-win\win-147.0.7727.50\chrome-win64\chrome.exe` | 4 cases, table above |
| The four cases against the oracle | `cargo test -p openui-layout --test grid_auto_placement_cursor_tests` (Windows) | 4 passed. With the old `place_items` all 4 fail, with the "Open UI before" boxes above |
| Layout crate | `cargo test -p openui-layout --no-fail-fast` (Windows) | 5,956 passed, 0 failed, 5 ignored |
| Generators | `generate_properties.py --check`; `generate_v02_contract.py --check`; `generate_renderer_contract.py --check` | no drift |
| `cargo fmt --all --check` | in `bindings/rust` | pass |

The expected boxes are written into the test from the oracle JSON; the
layout crate has no JSON parser as a dependency.

CENSUS

## Not done here

- The auto-placement scan still wraps at the explicit grid's track count.
  Blink wraps at the implicit grid's, which also counts tracks created by
  definite items beyond the explicit grid (`minor_max_end_line_`).
- Blink's `MoveToNextMajorLine` can jump to the end of an overlapping
  item; Open UI steps one track at a time. Whether the two can ever choose
  different positions was not checked.
- The card header's `auto` column width is a separate change, grid track
  sizing on `engine/grid-auto-track-max`. The cases here use a fixed-width
  action, so they do not depend on it.
