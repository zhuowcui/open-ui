# A fieldset's trailing child margin at a column end, 28 October 2026

In an auto-filled multicol, the bottom margin of a fieldset's last child is
now truncated at the column end when the next sibling forces a column break.
Open UI had carried the margin into the next column, so the fieldset took a
column of its own and pushed the forced block out of the multicol. The census
goes from 22,134 to 22,138 exact with no losses.

## Finding

In `trailing-child-margin-002` a 100px square two-column `column-fill: auto`
multicol with a red background holds a green fieldset with no border,
padding or margin. The fieldset holds a 70px block with a 50px bottom margin.
A 100px green block with `break-before: column` follows the fieldset. The
margin crosses the end of the first column and is truncated there
(css-break-3, "Adjoining Margins at Breaks"), so the fieldset fills the first
column and the forced block fills the second. Chrome 147 paints a green
square at every profile.

In Open UI the fieldset continued into the second column with the 20px of
margin left over. The forced block then started a third column outside the
multicol, and most of the second column stayed red.

`layout_multicol` truncates the trailing margin of an auto-height child
before a forced break only when the child establishes a new formatting
context (`establishes_new_fc_for_node`). A fieldset's own box does not, but
its rendered content sits in an anonymous content box that does, so the last
child's margin stays inside the fieldset and does not collapse through it.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The trailing-margin truncation before a forced break also applies to a fieldset child | `openui-layout` | `block.rs` `layout_multicol` |

The test `fieldset_trailing_child_margin_is_truncated_before_a_forced_break`
in `bindings/rust/openui-layout/tests/sp13r_multicol_tests.rs` builds the
`-002` tree. It asserts that the first column holds the fieldset and the
second the forced block. On `main` there are three columns: the fieldset, the
fieldset continuation and the forced block.

## Measurement

Paired expanded census in WSL. The base is the PR 49 census head `24fbc966`,
whose rendering code equals `main` at `6dba8872`. The head is `a6151760`,
which has the same code as the rebased branch commit. Both share one
Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,134 to 22,138 |
| Gained | 4: `trailing-child-margin-002` at all four profiles |
| Lost, improved, worsened | none |

Four Open UI images changed, all gains. Chromium is identical in every
comparison. Rows are in the
[census evidence](evidence/fieldset-trailing-margin-census-v1.json).

A Windows 1x render of 2,051 fixtures, all of `css_multicol` and `css_break`
plus the fieldset fixtures, compared before and after the change, differs
only in `trailing-child-margin-002`.

## Remaining failures surveyed

These fragmentation failures were examined during this work and stay open.
Each has a separate cause.

| Fixture | Finding | Owner |
| --- | --- | --- |
| `contain-strict-with-opacity-and-oof` | The opacity 0.2 composite is off by one in 10,000 pixels, (203,229,203) against Chrome's (204,230,204). The reference without columns is exact, so the difference is opacity paint inside a multicol fragment. | `openui-paint` |
| `out-of-flow-in-multicolumn-047` | The positioned overflow continuation of a nested multicol in the second outer column keeps the inner multicol's positioned slices, which the first column already holds, and widens them to the column. The owner is the abspos overflow path of `layout_multicol`. | `openui-layout` |
| `block-in-inline-000` | A block inside an inline's continuation is clipped at the inline's authored height. The continuation carries a block-axis clip and a decoration paint size of 34px. | `openui-paint` clip |
| `out-of-flow-in-multicolumn-039` | An absolutely positioned box beyond the in-flow content needs outer overflow columns. | `openui-layout` |
| `out-of-flow-in-multicolumn-038` | Edge blending at 1.25x and 1.5x. | `openui-paint` |
| Column seams | In 46 fractional-profile comparisons Chrome covers more of a column seam. In `spanner-fragmentation-000` Chrome snaps column backgrounds to pixels where Open UI antialiases them. In the `transform-000` to `-005` seams Chrome covers 75% of the seam pixel and Open UI 50%. The mechanisms differ. | `openui-paint` |

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,134 to 22,138 exact; 4 gains, 0 losses |
| Windows 1x fixture renders | `pixel_compare render` over `css_multicol`, `css_break` and fieldset fixtures, before and after | 1 changed, 2,050 identical |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,474 passed, 0 failed, 20 ignored |
