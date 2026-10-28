# Spanner overflow in a resumed nested multicol, 28 October 2026

A nested multicol whose spanner content overflows into the next outer column
now paints that overflow, as Chrome 147 does. The resumed part of the nested
multicol had a clip on both axes that hid the whole outer column. It now
clips only its inline axis. The census goes from 22,103 to 22,114 exact with
no losses.

## Finding

In `spanner-fragmentation-004` a 100px square two-column
`column-fill: auto` multicol with a red background holds a balanced
two-column multicol. That multicol's first child is a 50px
`column-span: all` box holding 200px of content: 50px green, 50px empty,
100px green. A 10px green spanner and an 80px green block follow. The nested
multicol ends in the outer multicol's first column, and the spanner's last
100px resumes in the second. Chrome 147 paints both outer columns green at
every profile. Open UI painted the second column red.

The layout tree was close to right. The second outer column held the nested
multicol's resumed part, with the replayed row and an appended slice that
carries the spanner's overflow tail at block offset 0. In
`layout_multicol`, the branch for `coordinate_sliced_nested_spanner_overflow`
gives that part `has_overflow_clip` with `block_axis_clip_only = false`,
which paint applies on both axes. That clip hid everything in the part. The
comment on the branch asks only that inline overflow columns from the
replayed row not escape the continuation's border box a second time. The
outer column box already clips the block axis.

Experiments on Windows at 1x:

- Dropping the clip makes `-004` and `-008` exact, but `-005` then shows
  inline overflow at x 120-145.
- An inline-only clip makes `-004`, `-005`, `-006` and `-008` exact.
- Clearing the part's `decoration_paint_block_size` or `decoration_slice`
  under the two-axis clip changes nothing.

Why paint's two-axis clip hides content inside the part's own box was not
traced. The owner is the fragment clip path in `openui-paint` `painter.rs`.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The resumed part of a nested multicol with coordinate-sliced spanner overflow clips only its inline axis (`inline_axis_clip_only = true`) | `openui-layout` | `block.rs` `layout_multicol` |

The test
`spanner_overflow_in_nested_multicol_continuation_is_not_block_clipped` in
`bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the
`-004` tree. It asserts that the nested continuation in the second outer
column clips only its inline axis, and that it carries the spanner's 100px
tail at block offset 0. It fails on `main`.

## Measurement

Paired expanded census in WSL. The base is the PR 44 head `9d82b235`, whose
rendering code equals `main` at `1644fa2a`. The head is `1e3b2844`, which has
the same tree as the rebased branch commit. Both share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,103 to 22,114 |
| Gained | 11: `spanner-fragmentation-004`, `-005` and `-008` at 1x, 1.5x and 2x; `-006` at 1x and 2x |
| Improved | 5: `-004` (7,875 to 125), `-005` (6,325 to 75), `-006` (2,015 to 375) and `-008` (6,300 to 125) at 1.25x; `-006` at 1.5x (2,850 to 300) |
| Lost, worsened | none |

Seventeen Open UI images changed. The other one is `multicol-nested-017` at
1.25x, which still differs with the same 125 mismatched pixels. Chromium is
identical in every comparison. Rows are in the
[census evidence](evidence/nested-spanner-overflow-clip-census-v1.json).

A Windows 1x render of all 2,029 `css_multicol` and `css_break` fixtures,
compared before and after the change, differs only in the four targets.

## Remaining failures

- `spanner-fragmentation-004`, `-005` and `-008` at 1.25x, plus `-006` at
  1.25x and 1.5x: the outer column seam at CSS x 70 falls inside a device
  pixel. Chrome blends about 75% green there; Open UI blends about 50%. In
  `-006` some other columns differ the same way. This is the same seam
  coverage as the earlier `spanner-fragmentation-000` to `-002` residuals.
  The likely owner is the ceil-snapped left edge that paint gives a
  non-first inline-only clip in `openui-paint` `painter.rs`. Not diagnosed.
- The two-axis paint clip finding above.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,103 to 22,114 exact; 11 gains, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol` and `css_break`, before and after | 4 changed, 2,025 identical |
| Layout tests | `cargo test --locked -p openui-layout` | 5,938 passed, 0 failed |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,469 passed, 0 failed, 20 ignored |
