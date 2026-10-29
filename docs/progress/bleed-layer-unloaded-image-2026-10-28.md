# A bleed layer that holds only its rounded border, 28 October 2026

A box with rounded corners, a translucent uniform solid border and a
background image that cannot load now paints its border straight through
the rounded clip, as Chrome does. Open UI had painted the border into an
intermediate layer. The census goes from 22,174 to 22,188 exact with no
losses.

## Finding

`background-origin/origin-{border,content,padding}-box_with_radius` and
`background-clip/clip-border-box_with_radius` give rounded boxes a
translucent border and a relative `url()` background image. The census
oracle does not package those images, so Chrome 147 fails to load them.
The fixtures had dropped the image layer entirely.

Blink still counts a declared image that fails to load as a background.
`BoxDecorationData::ComputeBleedAvoidance` therefore selects
`kBackgroundBleedClipLayer` for a box with a radius and a border that does
not obscure it. `BoxFragmentPainter` clips to the outer rounded border and
opens a layer. Nothing in the background renders, so the layer holds only
the border, which `BoxBorderPainter` paints on its fast path: one solid,
uniform-color `DrawPath` of the inner contour with inverse winding.
cc's `PlaybackFoldingIterator` folds SaveLayerAlpha -> one drawing op ->
Restore into that op. Chrome thus paints the path directly through the
antialiased clip.

With the image dropped, Open UI chose no bleed layer for these boxes, so
it painted a different border. With the image kept, Open UI opened a real
layer, and the border's edge pixels differed by one level from the folded
draw.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| When the bleed layer would hold only a solid uniform border, because no background layer can render, Open UI saves without a layer, as cc folds it. Fieldsets, collapsed table borders, borders painted after children, inset shadows, gradients and background colors keep the layer. So do boxes whose content-box corner is erased. | `openui-paint` | `painter.rs` `paint_box_decoration_background` (`bleed_layer_holds_only_border`, `bleed_layer_erases_content_corner`) |
| The four fixtures keep their unloadable image layer as an image resource with no bytes, registered as `unavailable:<url>`. | `pixel-compare` fixtures | `bindings/rust/pixel-compare/src/wpt/wpt_css_backgrounds.rs` |
| The porter emits that layer for a `url()` it cannot resolve, instead of dropping it. | WPT porter | `tools/wpt/port_wpt.py` `_unavailable_url_image_rust`; test `test_unloadable_url_background_layer_is_kept_without_bytes` |

The four builders were edited directly rather than regenerated. Splicing
them through `splice_text_port.py` also rewrites their profile and
templates, which is outside this change. The other builders the porter
generated earlier are not regenerated.

`border_over_unrenderable_background_image_paints_without_a_layer` in
`bindings/rust/openui-paint/tests/bleed_layer_fold_tests.rs` paints a
116x76 box with an 8px translucent border, 20px radii and an unloadable
image. It compares the result with an antialiased rounded clip and the
inverse-winding inner path drawn directly. It asserts that no pixel
differs. Without the change, 89 pixels differ.

## Measurement

This is a paired expanded census in WSL. The base is `2aa6f582`, whose
rendering code equals `main` at `000abb9b`. The head is `fc678b3e`. Both
use one shared Chromium cache.

| Result | Comparisons |
| --- | --- |
| Exact | 22,174 to 22,188 |
| Gained | 14 in all: the three `background-origin` fixtures at all four profiles, and `clip-border-box_with_radius` at 1x and 1.5x |
| Improved | `clip-border-box_with_radius`: 743 to 182 mismatched pixels at 1.25x, and 3,843 to 3,707 at 2x |
| Lost, worsened | none |

Chromium is identical in every comparison, and no other comparison changed.
Rows are in the
[census evidence](evidence/bleed-layer-unloaded-image-census-v1.json).

### Remaining failure

`background-clip/clip-border-box_with_radius` still differs at 1.25x and 2x.
Every remaining pixel lies on an edge of an Ahem glyph in the test's
description text or its orange text. The rounded borders and backgrounds
match. This residual belongs to text glyph rasterization (`openui-text`),
not to box decoration paint.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,174 to 22,188 exact; 14 gains, 0 losses |
| Fold test | `cargo test --locked -p openui-paint --test bleed_layer_fold_tests` | passes; 89 differing pixels without the change |
| Python CI suite and ledgers | the `ci.yml` unittest list, plus the frozen archive, renderer contract, SP13-R, SP17 and SP18 checks | 216 tests OK (6 skipped); all checks pass |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,483 passed, 0 failed, 20 ignored |
