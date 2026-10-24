# Central baseline for vertical flow, 21 October 2026

Windows qualification left one failure:
`openui-layout::flex::algorithm::tests::vertical_column_cross_margin_reduces_fit_content_before_wrapping`.
For a 16px Arial `vertical-lr` line with two empty 15px inline-blocks, Windows
Chrome-for-Testing 147.0.7727.50 measures a line block size of 18 px. Open UI
measured 18.5 px. The cause is general: Open UI aligned every line on the
alphabetic baseline, while Blink aligns vertical flow on the central one.

## Blink behavior

The rules below come from the pinned 147.0.7727.50 sources.

| Rule | Blink source |
| --- | --- |
| A line uses its container's baseline. Vertical flow uses the central baseline unless glyphs are sideways (`text-orientation: sideways`, `sideways-*` writing modes); everything else uses the alphabetic baseline | `ComputedStyle::GetFontBaseline`, `InlineLayoutAlgorithm` |
| Strut, text and inline box metrics are `GetFontHeight(baseline)`. For the central baseline that is `Height() - Height() / 2` above and `Height() / 2` below, where `Height()` is the integer ascent plus the integer descent | `InlineBoxState::ComputeTextMetrics`, `FontMetrics::IntAscentInternal` |
| An atomic inline without a baseline synthesizes it from its margin box: `(size, 0)` alphabetic, `(size - size / 2, size / 2)` central, in layout units | `LogicalBoxFragment::BaselineMetrics` |
| A box exposes a descendant baseline only in its own writing mode | `LogicalBoxFragment::FirstBaseline` (`IsWritingModeEqual`) |
| Text paints at the run's top plus the primary font's alphabetic integer ascent, whichever baseline placed the run | `TextFragmentPainter::Paint` |
| A flex item without a baseline synthesizes it at its middle for a central container | `LogicalBoxFragment::SynthesizedBaseline` |

Arial 16px has integer metrics 14 + 3, so its central split is 9 + 8. Half
leading makes that 9.5 + 8.5. Each inline-block contributes 7.5 + 7.5. The
line is 9.5 + 8.5 = 18. DejaVu Sans (15 + 4) gives 19 under either baseline,
which is why Linux never showed the gap.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `FontBaseline` and `FontMetrics::relative_to_baseline` re-express ascent and descent for the central baseline | `openui-text` | `font/metrics.rs` |
| `create_line_box` measures strut, text, inline box and `<br>` metrics from the container's baseline. Atomic inlines without a baseline use the synthesized split for baseline, length, percentage, `sub` and `super` alignment | `openui-layout` | `inline/algorithm.rs` |
| Text, hyphen, ellipsis and clamp-marker fragments keep their alphabetic geometry around the glyph baseline Blink paints: the line baseline less the central ascent plus the integer alphabetic ascent. Inline box decoration bounds follow the same metrics, so they also stay put around the glyphs | `openui-layout` | `inline/algorithm.rs` |
| Block baselines, and the last-line baseline an inline-block exports, come only from descendants in the same writing mode | `openui-layout` | `block.rs`, `inline/algorithm.rs` |
| Vertical flex rows align on the item's first baseline, synthesizing a missing one at the middle of a mixed or upright vertical item. The previous emulation, which centred vertical-rl items and shifted line-clamped groups by half the font-size difference, is removed | `openui-layout` | `flex/algorithm.rs` |
| The intrinsic block-size probe lays inline content out in the node's own writing direction, as block layout does. Before, a vertical node's lines were probed horizontally, measuring atomic inlines by their physical height | `openui-layout` | `intrinsic_sizing.rs` |

For a line of text in a single font the line box and the painted glyphs do not
change. Horizontal and sideways lines are unchanged.

## Verification

Local measurements of this branch.

| Check | Command | Result |
| --- | --- | --- |
| Windows workspace, MSVC, with the Skia define | `cargo test --workspace --locked --no-fail-fast` | 9,407 passed, 0 failed, 18 ignored. The vertical flex test passes with Windows Chrome's 18 |
| Linux workspace, WSL, system clang 21 | `cargo --config 'target.x86_64-unknown-linux-gnu.rustflags=[]' test --workspace --locked --no-fail-fast --exclude pixel-compare` | 9,406 passed, 0 failed, 13 ignored |
| New unit and layout tests | `font::metrics::tests` in `openui-text`; `vertical_flow_aligns_atomic_inlines_on_the_central_baseline` in `openui-layout` | Pass. 16px Ahem with an empty 30px inline-block gives a 30px vertical line and a 33px sideways line |
| Rust formatting | `cargo fmt --all --check` | Pass |

### Paired renderer census

`build_renderer.py` and `run_renderer_matrix.py --suite expanded` in WSL, for
`main` (`db121c39`) and this change (`896015de`, the same tree), sharing one
Chromium cache. Of 23,728 comparisons, Chromium is identical in all and Open UI
changes in 6. Exact falls from 21,681 to 21,679. The per-comparison delta is in
[vertical-central-baseline-census-v1.json](evidence/vertical-central-baseline-census-v1.json).

| Result | Comparisons |
| --- | --- |
| Improved | `css_sizing/vert-block-size-small-or-larger-than-container-with-min-or-max-content-1` and its reference at 2×: 33,872 to 12,400 differing pixels each |
| Lost | `css_flexbox/alignment_flex-align-baseline-line-clamp-003.tentative` at 1× and 2× |
| Worsened | the same test at 1.25× (24 to 3,323) and 1.5× (50 to 9,148) |

The remaining difference is one container of that tentative test: a
vertical-lr `-webkit-box` clamped to three lines whose last line holds only the
block ellipsis, aligned by its last baseline. Open UI lays that line out with
zero inline size, so its central baseline sits on its line-over edge; Chrome's
expected offsets place it 20 px further in, at the middle of a 40 px line.
Before this change the old alphabetic offsets and a half-leading correction
happened to land on Chrome's pixels. The line-clamp marker geometry is the
owner; it is recorded here, not worked around.

Four intermediate censuses guided the change. The first, with the central
metrics alone, lost 34 comparisons: text fragments had moved relative to their
glyph baselines, and device snapping of vertical-lr Ahem anchors shifted whole
glyph columns. Keeping fragments placed around the glyph baseline restored
those. Making vertical flex synthesis and descendant baselines follow Blink
restored `baseline-synthesis-004`, `line-clamp-002` and an orthogonal multicol
crash test.

## Not covered

Native form controls and failed images that export a synthesized text
baseline, and ruby annotation layout, still assume the alphabetic baseline in
vertical flow.

## Update, 24 October 2026

The `line-clamp-003` residual is fixed. Its owner was flex line cross sizing
for vertical and clamped baseline groups, not the clamp marker; see
[the flex baseline record](flex-baseline-group-2026-10-24.md).
