# An `auto` grid column grows to its max-content width before `fr` columns, 10 November 2026

In a grid container with a definite inline size, columns now go through
the maximize step of CSS Grid track sizing: each column grows toward its
growth limit before flexible columns take the remaining space. An `auto`
column beside a `1fr` column is now as wide as its item's max-content width,
as in Chromium, not its min-content width. The components track found this
in the shadcn card header (`grid-template-columns: 1fr auto`), whose action
text wrapped onto two lines in Open UI.

## Finding

`grid_layout_contents` in `openui-layout/src/grid.rs` keeps one size per
track, its base size. With a definite container inline size,
`size_intrinsic_tracks` gives an `auto` column only its items' minimum
contributions. The only maximize step, `maximize_fixed_max_tracks`, grows
`minmax(..., <length>)` tracks. `distribute_flexible_tracks` then gives all
the remaining space to the `1fr` column, and the `auto` column stays at its
min-content width. Without a flexible column, content alignment then shares
the free space equally between `auto` columns, whatever their max-content
widths.

Chromium 147 follows the specification's track sizing algorithm in
`GridTrackSizingAlgorithm::ComputeUsedTrackSizes`
(`third_party/blink/renderer/core/layout/grid/grid_track_sizing_algorithm.cc`):

1. `ResolveIntrinsicTrackSizes` takes the items that do not span a flexible
   track, grouped by span size. For each group, after the base size steps,
   it raises the growth limits of tracks with an intrinsic maximum by the
   items' min-content contributions (`kForIntrinsicMaximums`). It then
   raises the limits of `auto`, `max-content` and `fit-content` maximums by
   the max-content contributions (`kForMaxContentMaximums`), as
   `GridLayoutAlgorithm::ContributionSizeForGridItem` selects them.
   `DistributeExtraSpaceToSets` gives each track an equal share, smallest
   growth potential first; a limit that was finite before the group's first
   maximum step grows only after every "infinitely growable" track is
   filled.
2. `SetIndefiniteGrowthLimitsToBaseSize`.
3. `MaximizeTracks` distributes the free space (the available size less
   the tracks and gutters) equally to all base sizes, each frozen at its
   growth limit.
4. `ExpandFlexibleTracks`.
5. `StretchAutoTracks` shares what is left equally among `auto` maximums.

These sources were read at 147.0.7727.24, the local source checkout, which
is on the same branch as the pinned 147.0.7727.50.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `intrinsic_growth_limits` resolves a growth limit for each column, following the two maximum steps above, from the base sizes `size_intrinsic_tracks` already resolved. | `openui-layout` | `bindings/rust/openui-layout/src/grid.rs` |
| `maximize_tracks` is Blink's `MaximizeTracks`, using the same share arithmetic as `DistributeExtraSpaceToSets` (truncated layout units, smallest potential first, the last growable track takes the remainder). | `openui-layout` | same |
| With a definite container inline size, columns use these two steps in place of `maximize_fixed_max_tracks`. With an indefinite inline size, and for rows, nothing changes. | `openui-layout` | same |
| An item whose writing mode is orthogonal to the grid's gives both maximum steps the contribution it gave the base sizes (`base_size_contribution`, shared with `size_intrinsic_tracks`), so it does not grow its columns past them. Blink's `ContributionSizeForGridItem` gives such an item one contribution for `kForIntrinsicMaximums` and `kForMaxContentMaximums` alike, `BlockContributionSize()`: its block size laid out at the size of its rows. This engine does not measure that size. | `openui-layout` | same |

Rows are not changed: `size_row_tracks_from_layout` already gives a row the
laid-out block size of its items, which for block contributions is both the
min-content and the max-content contribution.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| `1fr auto` with wrappable text; two `auto` columns; a `vertical-lr` item in an `auto` column; a `vertical-lr` item with `min-width: 10px` spanning three `auto` columns | `cargo test -p openui-layout --test grid_auto_track_max_tests` (Windows) | 4 passed. With the old path restored, the first two fail: the `auto` column is 26.69px (min-content), not 58.92px (max-content), and the narrow `auto` column is 146px, not 113.77px. With the orthogonal guard removed, the third fails: the column grows from 26.69px to the item's horizontal max-content, 91.16px. The third test checks only that the column is not widened; its Chromium oracle is the coverage run's `subgrid/orthogonal-writing-mode-004-ref`. With an orthogonal item contributing its min-content size to both maximum steps instead, the fourth fails: the item is 26.69px wide, not 10px |
| Maximize arithmetic: smallest potential first, then stretch; free space limit | `cargo test -p openui-layout --lib grid::tests` (Windows) | 4 passed |
| Layout crate | `cargo test -p openui-layout --no-fail-fast` (Windows) | 5,958 passed, 0 failed, 5 ignored |
| Generators | `generate_properties.py --check`; `generate_v02_contract.py --check`; `generate_renderer_contract.py --check` | no drift |
| `cargo fmt --all --check` | in `bindings/rust` | pass |

The text tests live in `openui-layout/tests/`, not in `grid.rs`: the
renderer contract counts a style field named in a source file as consumed
there, and `grid.rs` does not read `font_size` or `line_height`.

## Earlier heads, measured and not merged

Two earlier heads of this change were measured against the same bases.
Neither changed the paired expanded census (22,517 to 22,517 exact). Both
lost CSS Grid coverage, so neither was merged:

- At `fbb915b6`, every item gave the max-content step its max-content
  contribution. Coverage gained 8 (`subgrid/auto-track-sizing-001` and its
  reference in all 4 profiles) and lost 3:
  `subgrid/orthogonal-writing-mode-004-ref` in the legacy, mobile and
  1920 profiles, whose vertical items were wider than Chromium's.
- At `7d0d38f7`, an orthogonal item gave both maximum steps its
  min-content contribution as measured here. Coverage still gained the
  same 8 and lost the same 3 (2,218 to 2,223 exact). Of the 83 images that
  changed and stayed different, 56 improved and 27 got worse: 21 in tests
  with orthogonal items, and 6 in `subgrid/line-names-005-ref` and
  `subgrid/writing-directions-002`.

This head gives an orthogonal item the contribution it gave the base sizes
instead.

CENSUS

## Not done here

- Base sizes still come from the existing `size_intrinsic_tracks`, not
  from Blink's per-group base size steps; growth limits are computed after
  all base sizes. A multi-span item can still differ from Chromium.
- Items that span a flexible track are left out of the growth limits, as in
  Blink, but their base size handling is unchanged.
- Under an indefinite inline size, the existing path is unchanged.
- An orthogonal item does not raise growth limits past the base sizes it
  gave, where Blink would raise them by its `BlockContributionSize()`.
  Measuring that size needs the item laid out at its rows' size before the
  columns are final.
