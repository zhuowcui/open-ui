# A forced overflow child before its break, 28 October 2026

When a forced break inside a fixed-height box sends a child into the next
column as visible overflow, the first column no longer paints that child
too. This resolves the remaining failure in the
[overflow continuation record](overflow-continuation-principal-box-2026-10-28.md):
`block-in-inline-000` is now exact at all four profiles.

## Finding

In `block-in-inline-000` a 100px square two-column `column-fill: auto`
multicol holds a span with a 50px block. Inside that block, a line break is
followed by a 100px `break-before: column` block, which overflows the 50px
block into the second column.

`layout_multicol` builds the second column's part of the 50px block through
its visible overflow path. That path clones the source fragment and shifts
its children up by the consumed source. When the overflow starts at a forced
break, the continuations resume at the forced child's offset
(`forced_overflow_break_top`). The first column's part was the unchanged
source fragment, however, and still held the forced child below the line
break. The column clipped it, but its top part painted in the first column's
lower area. At 1.25x the column seam then took three antialiased green
layers where Chrome 147 composites two, and 167 pixels differed.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| After the visible overflow continuations are cloned, the first column's part keeps only the in-flow children that start before the forced break, plus its out-of-flow children. | `openui-layout` | `block.rs` `layout_multicol` |

`forced_overflow_child_is_not_painted_before_its_break` in
`bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds
`block-in-inline-000` without the inline. It asserts that the first column's
part of the 50px block holds only the 16px block, and that the second
column's part holds both the 16px block and the forced block. Without the
change the first part also holds the forced block.

## Measurement

Paired expanded census in WSL. The base is the PR 52 census head `089bc85f`,
the head is `25b93820`. The branch commit rebased onto `main` at `9499d11f`
has the same code. The base was cut before PR 51 merged, so the head also
holds PR 51's `out-of-flow-in-multicolumn-047` repair. Its four gains are
PR 51's and were already counted in its own record. Both share one Chromium
cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,153 to 22,158 |
| Gained by this change | `block-in-inline-000` at 1.25x, from 167 mismatched pixels |
| Gained by PR 51 in the head | `out-of-flow-in-multicolumn-047` at all four profiles |
| Lost, improved, worsened | none |

Chromium is identical in every comparison. Rows are in the
[census evidence](evidence/forced-overflow-first-fragment-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,153 to 22,158 exact; 1 gain of this change, 4 of PR 51, 0 losses |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,478 passed, 0 failed, 20 ignored |
