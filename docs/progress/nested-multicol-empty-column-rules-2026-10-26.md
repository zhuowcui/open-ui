# Nested column rules beside empty columns, 26 October 2026

A triage of structural census failures (largest channel difference 100 or more
at 1×) found the `multicol-breaking-*` family. Each differs by one full-height,
2px fuchsia column rule that Open UI paints and Chrome does not.

## Finding

The tests nest a two-column multicol, with a definite 300px height, inside a
four-column multicol whose columns are 100px tall. The outer fragmentainers
slice the inner multicol, and its text runs out before the third outer
column. Open UI's fragment tree for that third slice still holds both inner
column boxes, though their lines all sit at negative offsets (they belong to
the earlier slice). It also holds a rule, carried into the slice because
"rules belong to every used column row".

Blink's column rule painter draws a rule only between two column fragments of
the same multicol fragment. A column with no content in a slice has no
fragment there, so Chrome paints no rule beside it.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| After slicing a nested multicol, a column rule stays only when the columns on both sides have content inside the slice's block range | layout | `openui-layout/src/block.rs` `retain_rules_between_content_columns` |

## Measurement

Paired expanded census in WSL against `main`'s rendering (clone commits
`1bdddcb4` and `ed948ac8`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 21,984 to 21,992 |
| Gained | 8 |
| Improved | 4 |
| Lost, worsened | none |

`multicol-breaking-001`, `-003`, `-nobackground-001` and `-nobackground-003`
become exact at 1× and 1.25×. At 1.5× they improve from 7,424 to 6,824 pixels;
the rest is text. Chromium is identical in every comparison. Rows are in the
[evidence](evidence/nested-multicol-empty-column-rules-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 21,984 to 21,992 exact; 8 gains, 0 losses |
| Layout unit tests | `cargo test --locked -p openui-layout --lib` | 746 pass |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,459 passed, 0 failed, 20 ignored |
