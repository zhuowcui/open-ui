# Padded multicolumn principal-source continuity, 21 October 2026

The checkpoint's next renderer task: reproduce the two strict producer
failures of the archived candidate `3a8c9fbe` against the authoritative
branch, then repair finite principal ownership.

## Reproduction

The archived patch (`docs/progress/evidence/branch-checkpoint-2026-10-17.tar.gz`,
member `unadopted-candidates/3a8c9fbe.patch`) still applies to `main`. On
Windows its new test target `split_multicol_padded_continuation` fails the two
strict producer tests the checkpoint names, with the archived symptom,
"principal source must remain contiguous":

- `padded_split_principal_source_conserves_actual_probe_padding_and_short_tail`
- `padded_split_first_and_fresh_capacities_share_source_progress_and_live_outer_size`

The fixture is a positioned, padded, definite-height containing block split by
a column spanner inside a nested multicol, with an absolutely positioned
descendant. Each column piece of the containing block records the window of
its border-box source that it owns; the windows must tile the source exactly.

## Causes and repairs

| Failure | Cause | Repair | Source |
| --- | --- | --- | --- |
| Fractional padding: the last owned window is 87.5 px where 90.75 px remain, and a stale 120 px window is repeated | The final portion of a definite-height split container distributed its height without its block-end padding and border, so no column piece owned them. The completed-row projection then rejected the stream and the older continuation path laid the rows out again | The final portion distributes the larger of its principal border box (`paint_height` plus all its edges) and its in-flow content extent, which can overflow a definite box without carrying the edges | `block.rs`, both split-portion distribution sites |
| A taller outer column: the whole inner multicol is laid out a second time | A final column row keeps the fresh fragmentainer's capacity (160 px), which extends past the multicol's border-box end (284 px), so the projection rejected the row | A column row's source membership is the principal piece it owns, not its capacity | `block.rs`, `project_completed_split_principal_rows` |

Adopting the candidate also regressed existing guards and WPT comparisons. Two
more repairs keep its scope to the split containing blocks it was written for:

| Regression | Cause | Repair |
| --- | --- | --- |
| `sp13r_multicol_tests::nested_post_spanner_row_resumes_as_sequential_block_flow` (three outer columns instead of two) | The finite-principal predicate admitted a definite wrapper with no positioned descendant, switching it to split-row capacities | The predicate requires a positioned descendant whose containing block is the container |
| `css_multicol/multicol-nested-016` (lost at four profiles) | After a nested remaining-capacity relayout, the candidate linearized the columns again even when they held block-level boxes | Re-linearize only when the measured flow was linearized and every column holds only line boxes |

## Scope

This change adopts the candidate's layout producer and engine projection: the
`finite_principal_source` split portions, the completed-row projection,
authored projection records (`authored_projection.rs`), the engine's
projection-aware box ownership, and five of its test targets. It does not
adopt the candidate's 3,100-line painter change (decoration and positioned
column paint phases). A painter-only bisect showed that change produced both
census gains (for example `css_break/fieldset-002`) and losses
(`css_break/out-of-flow-in-multicolumn-053`, `css_break/fieldset-001`,
`css_break/overflow-clip-014` to `-017`), so it needs its own task. Its test
target `fragmented_column_paint_phases` and its Chromium crops stay in the
archive.

`sp13r_multicol_paint_tests::middle_sliced_continuation_preserves_fractional_fragmentainer_edge_coverage`
pins internal pixel values, not Chromium observations. Rendered by pinned
Linux Chrome-for-Testing 147.0.7727.50 at 800x600, 1.25x, the scenario differs
from `main` in 2,708 pixels and from this change in 2,499; neither pinned value
matches Chrome. The guard is re-pinned to this change's coverage.

## Verification

Local measurements of this branch.

| Check | Command | Result |
| --- | --- | --- |
| Strict producer tests, Windows | `cargo test --locked -p openui-engine --test split_multicol_padded_continuation` | 3 pass; 1 root-only diagnostic ignored |
| Adopted candidate targets, Windows | the five engine test targets above | All pass |
| Windows workspace, MSVC | `cargo test --workspace --locked --no-fail-fast` | 9,432 passed, 0 failed, 20 ignored (two root-only diagnostics) |
| Linux workspace, WSL | `cargo --config 'target.x86_64-unknown-linux-gnu.rustflags=[]' test --workspace --locked --no-fail-fast --exclude pixel-compare` | 9,431 passed, 0 failed, 15 ignored (two root-only diagnostics) |
| Rust formatting | `cargo fmt --all --check` | Pass |

### Paired renderer census

`build_renderer.py` and `run_renderer_matrix.py --suite expanded` in WSL,
against `main` with the central baseline (`896015de`, the same tree as
`a22a5500`), sharing one Chromium cache. Of 23,728 comparisons, Chromium is
identical in all and Open UI changes in 10, with **7 gains, 2 improved and no
losses or worsened comparisons**. Exact rises from 21,679 to 21,686:
`css_multicol/multicol-nested-018` at all four profiles and
`css_break/inline-skipping-fragmentainer-001` at three; the fourth profile of
the latter and `css_multicol/multicol-span-all-children-height-007` at 1.25x
improve. The per-comparison delta is in
[multicol-principal-source-census-v1.json](evidence/multicol-principal-source-census-v1.json).

An earlier census of the whole candidate with only the first two repairs had
20 gains and 16 losses. The two scope repairs and leaving out the painter
change removed every loss; the painter-only gains are recorded above for the
follow-up task.
