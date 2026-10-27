# Abspos boxes in a stacked nested multicol row, 27 October 2026

An auto-height multicol nested in a fragmented multicol lays out one column
row for each outer fragmentainer, stacked in its own block direction. Open UI
now resumes an abspos box laid out in a later stacked row in the outer slice
that shows that row. It had left the box in the first slice, clipped. The
census goes from 22,093 to 22,097 exact with no losses.

## Finding

In `out-of-flow-in-multicolumn-016` a 300px wide, 100px tall two-column
multicol holds a 100px wide auto-height two-column multicol, shifted by
`left: -150px`. Its content is a 200px block, then a 200px relative box with
a 50x200 green abspos box at `top: 0`. The inner multicol fills its first row
in the outer multicol's first column, which the shift moves off-screen. Its
second row, holding the relative box, belongs in the outer multicol's second
column, shifted to x 0-100, where the green box should cover the inner
multicol's red background. Chrome 147 paints the green square at every
profile. Open UI painted the red background and no green.

The inner multicol's own fragment is correct: column rows at block offsets 0
and 100, and the abspos continuations at (0, 100) and (50, 100), numbered as
that row's fragmentainers 0 and 1. The outer multicol's slicing had two
faults:

- `resume_nested_multicol_positioned_row` took logical index 0 to mean the
  positioned flow starts in the first row, so it kept both continuations in
  the first slice and none in the second.
- When positioned continuation rows are present, the slicer does not shift a
  slice's content, so the second slice kept its column row at block offset
  100, below the outer column's clip.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| When positioned continuations restart their logical indices in a stacked row, their untranslated block offset (less `visual_offset` and `continuation_visual_offset`) selects that row, and they move to the slice's block-start | `openui-layout` | `block.rs` `resume_nested_multicol_positioned_row` |
| A slice with positioned continuation rows moves a column row that starts at or below the consumed offset to its block-start | `openui-layout` | `block.rs` `layout_multicol` |

The test
`abspos_in_stacked_nested_multicol_row_resumes_in_the_ancestor_continuation`
in `bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the
`-016` tree. It asserts no abspos fragment in the first inner slice, and the
second slice's columns and both 100px abspos fragments at block offset 0. It
fails on `main`.

Subtracting the relative-position translation matters: in
`out-of-flow-in-multicolumn-062` relative offsets put the continuations at
block offset 150, which is not a stacked row. Without the subtraction that
test regressed by 5,000 pixels at 1x.

## Measurement

Paired expanded census in WSL. The base is the PR 42 head `99d3aa72`, whose
rendering code equals `main` at `900fdace`. The head is `73db0d1d`, and both
share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,093 to 22,097 |
| Gained | 4: `out-of-flow-in-multicolumn-016` at all four profiles |
| Lost, improved, worsened | none |

Eight Open UI images changed. The other four are
`out-of-flow-in-multicolumn-038`, which still differs with the same
mismatched pixel count at every profile. Chromium is identical in every
comparison. Rows are in the
[census evidence](evidence/nested-multicol-stacked-abspos-census-v1.json).

## Remaining failures

`out-of-flow-in-multicolumn-037`, `-038` and `-039` take the same positioned
continuation path. At 1x, `-037` paints its fixed-position green square 50px
right of Chrome's; `-038` and `-039` differ over the same 150x100 region. The
likely owner is the inline mapping of fixed continuations in a nested
multicol in `openui-layout` `block.rs`. They are not diagnosed here.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,093 to 22,097 exact; 4 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,936 passed, 0 failed |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,467 passed, 0 failed, 20 ignored |
