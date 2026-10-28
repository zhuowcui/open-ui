# Nested balanced rows after descendant forced breaks, 28 October 2026

A balanced multicol nested in an auto-filled multicol now lays out its later
rows from the source that remains after forced breaks inside its content.
Open UI had estimated the rows as if no forced break ended a column early,
so the last row was too short for the content left. The census goes from
22,130 to 22,134 exact with no losses.

## Finding

In `multicol-nested-column-rule-003` a 100px square two-column
`column-fill: auto` multicol with a red background holds a balanced
two-column multicol. Its one green child holds a 10px block, another 10px
block and a 200px block, the first two with `break-after: column`. The forced
breaks end the first nested row, in the first outer column, after 20px of
source. The second nested row, in the second outer column, balances the
200px block into two 100px columns. Chrome 147 paints a green square at every
profile.

In Open UI the second nested row was 10px tall with two 10px columns, so most
of the second outer column stayed red. The row heights came from
`balanced_outer_row_heights`, which `layout_multicol` uses for a balanced
nested multicol in sequential outer rows. That estimate sees each principal
child as one unbreakable stream of source. It cannot see that the forced
breaks inside the green child leave the first row with only 20px. It
therefore balanced the whole child across both rows.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The constrained balanced row estimate is skipped when the nested multicol has a descendant with a forced break (`subtree_has_forced_break_descendant`); its rows then come from the laid-out column sizes | `openui-layout` | `block.rs` `layout_multicol` |

The test `nested_balanced_rows_resume_after_descendant_forced_breaks` in
`bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the
`column-rule-003` tree without its rules. It asserts that the nested
continuation in the second outer column is 100px tall with two 100px
columns. On `main` it is 10px tall with two 10px columns.

## Measurement

Paired expanded census in WSL. The base is the PR 48 head `f382fbe9`, whose
rendering code equals `main` at `84e6eb2a`. The head is `24fbc966`, which has
the same code as the rebased branch commit. Both share one Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,130 to 22,134 |
| Gained | 4: `multicol-nested-column-rule-003` at all four profiles |
| Lost, improved, worsened | none |

Four Open UI images changed, all gains. Chromium is identical in every
comparison. Rows are in the
[census evidence](evidence/nested-multicol-forced-rows-census-v1.json).

A Windows 1x render of all 2,029 `css_multicol` and `css_break` fixtures,
compared before and after the change, differs only in
`multicol-nested-column-rule-003`.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,130 to 22,134 exact; 4 gains, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol` and `css_break`, before and after | 1 changed, 2,028 identical |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,473 passed, 0 failed, 20 ignored |
