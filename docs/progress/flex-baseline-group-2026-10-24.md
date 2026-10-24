# Flex baseline groups in every writing mode, 24 October 2026

The [central baseline record](vertical-central-baseline-2026-10-21.md) left
`css_flexbox/alignment_flex-align-baseline-line-clamp-003.tentative` as a
residual and attributed it to the clamp marker's line geometry. Measuring
Chrome's geometry with the Windows harness (same Blink 147 layout) shows the
clamp is not the owner.

## Finding

In the test's second container, a vertical-lr flex row aligns two items by
`last baseline`. The clamped box is 100 px in the cross axis and its last
baseline is 90 px from its start: an ellipsis-only line of zero block size
after two 40 px lines. Open UI computes the same 90. Chrome sizes the flex
line as the largest pre-baseline extent plus the largest post-baseline extent:
90 + max(10, 15) = 105 px. Open UI used only the largest item cross size,
100 px, so every item sat 5 px toward block-start.

Open UI's hypothetical cross sizing built that baseline-sharing extent only
in horizontal writing modes. It also skipped every line that held a
line-clamped item, because re-laying out a clamped box could see its
unclamped continuation.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| Baseline-sharing line extents are computed in every writing mode | flex layout | `openui-layout/src/flex/algorithm.rs` `compute_line_cross_sizes` |
| A clamped item's last baseline is capped at its cross edge, as final placement does. A clamped item with no content baseline stays out of the extent, keeping the legacy measurement its unit test pins | flex layout | same |
| Missing baselines are synthesized exactly as final placement synthesizes them: centered for upright and mixed vertical text, block-start for sideways vertical-lr | flex layout | same |

The first attempt synthesized vertical baselines at the cross edge and lost
`baseline-synthesis-003` and `-004` at every profile. Using final placement's
synthesis restored them.

## Measurement

Paired expanded census in WSL against `main`'s Linux rendering (clone commits
`5d3d1afa` and `fc9335e7`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 21,699 to 21,701 |
| Gained | `alignment_flex-align-baseline-line-clamp-003.tentative` at 1× and 2× |
| Improved | the same test at 1.25× (3,323 to 24 pixels) and 1.5× (9,148 to 50) |
| Lost, worsened | none |

Chromium is identical in all 23,728 comparisons, and only these four change.
Per-comparison rows are in the [evidence](evidence/flex-baseline-group-census-v1.json).
The Windows comparison shows the same three line-clamp baseline tests exact
at 1× and 2×.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 21,699 to 21,701 exact; 2 gains, 0 losses |
| Flex unit tests | `cargo test -p openui-layout --lib flex::` | 73 pass, including `baseline_line_cross_size_preserves_legacy_line_clamp_measurement` |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast` | 9,457 passed, 0 failed, 20 ignored |
