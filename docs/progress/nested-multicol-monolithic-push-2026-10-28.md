# A nested multicol row with leading monolithic content, 28 October 2026

An auto-height multicol nested in an auto-filled multicol, after earlier
content in the same outer column, now starts in the next outer column when
its leading monolithic content cannot fit in the space that remains. That is
how Chrome 147 lays it out. Open UI had let that content overflow the short
row. The census goes from 22,114 to 22,118 exact with no losses.

## Finding

In `multicol-nested-024` a 100px square red box holds a 150px tall,
two-column `column-fill: auto` multicol. A 100px green block comes first. It
is followed by a balanced two-column multicol of three 100px
`contain: size` blocks, two green and one transparent. Only 50px of the first
outer column remain for the nested multicol. Chrome 147 starts the nested
multicol in the second outer column, where its first row holds the two green
blocks side by side, and paints a green square at every profile. Open UI
laid out the first nested row in the 50px remainder. The two green blocks
overflowed below the red box, and the second outer column held only the
transparent block.

Blink's `ColumnLayoutAlgorithm::LayoutRow`
(`third_party/blink/renderer/core/layout/column_layout_algorithm.cc`, the
`may_have_more_space_in_next_outer_fragmentainer` path) covers this case.
When a nested multicol is constrained by its outer fragmentation context
after earlier progress there, it avoids breaks worse than those already
taken. If a column overflows the outer space with content taller than that
space, it lays out the row again at that height. The outer algorithm then
breaks before the nested multicol, at a better break point.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| In an auto-filled multicol, an auto-height, visible-overflow nested multicol after earlier progress in a column moves to the next column when its leading monolithic unit (`leading_monolithic_descendant_extent`) plus its block-start margin exceeds the remaining space, but the unit fits a fresh column | `openui-layout` | `block.rs` `layout_multicol` |

The test
`nested_multicol_with_leading_monolithic_row_starts_in_the_next_outer_column`
in `bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the
`-024` tree. It asserts that the nested multicol appears only in outer
columns 1 and 2. On `main` it appears in columns 0 and 1.

## Measurement

Paired expanded census in WSL. The base is the PR 45 head `1e3b2844`, whose
rendering code equals `main` at `117707c1`. The head is `fe8b546b`, which has
the same tree as the rebased branch commit. Both share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,114 to 22,118 |
| Gained | 4: `multicol-nested-024` at all four profiles |
| Lost, improved, worsened | none |

Four Open UI images changed, all gains. Chromium is identical in every
comparison. Rows are in the
[census evidence](evidence/nested-multicol-monolithic-push-census-v1.json).

A Windows 1x render of all 2,029 `css_multicol` and `css_break` fixtures,
compared before and after the change, differs only in `multicol-nested-024`.

## Remaining failures

The rule covers only the auto-filled outer multicol and the leading
monolithic unit. Blink also pushes a row for a monolithic unit that leads a
later inner column, and applies the rule inside balanced outer multicols.
Neither case has a census failure attributed to it here.

Related nested-row failures with separate causes:

- `multicol-nested-013` and `-014`: a later nested row reuses the first row's
  shortened capacity.
- `multicol-nested-028`: a block's final source slice grows to the full
  nested row.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,114 to 22,118 exact; 4 gains, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol` and `css_break`, before and after | 1 changed, 2,028 identical |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,470 passed, 0 failed, 20 ignored |
