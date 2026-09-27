# Rounded border and content-background coverage

The CPU primitive test `wpt/css_backgrounds/background-clip-content-box-with-border-radius-002`
still differs from Chromium in all 40 focused profiles. The clean
[v6 primitive evidence](generated/focused-primitive-raster-v6.json) records
6,728 differing pixels for this ID. Chromium remains the expected image.

The [reduced fixture](../../tools/qualification/reproducers/rounded-content-border-seam.html)
is a 50 × 50 CSS pixel black content background inside a
25 pixel solid black border, with `background-clip: content-box` and a 100%
top-left radius. The sibling `-003` fixture has an exact four-profile result,
so the failure is specific to how this border and background meet, not a
general inability to draw a rounded content clip.
Captured with the pinned Chromium at 320 × 240 CSS pixels and 1.5 scale,
the reduced fixture produced the same PNG bytes as the matrix fixture
(`4017832a100689bb0b46ff0dd878a4d6179ad23d40b834836025ea286a5dea08`).

At 320 × 240 CSS pixels and 1.5 device scale, the right and bottom content
edges fall at physical coordinate 142.5. At `(142, 80)`, Chromium's gray
channel is 63 and the clean Open UI channel is 127. With the border paint
temporarily disabled, Open UI remains at 127; with the background paint
disabled, it becomes 255. This isolates a missing fractional **border**
contribution. The nonrenderable rounded-border path first applies a hard
polygon clip to each side, then subtracts an antialiased inner rounded
contour. The hard side clip discards the half-covered device cell at the
content edge before the inner contour can contribute to it. The same
straight-edge pattern appears along the right and bottom seams.

In a dirty diagnostic build, extending the polygon's interior vertices by
one CSS pixel and using a float16 layer restored that border contribution.
The test improved from 6,728 to 6,224 differing pixels across the same 40
profiles. At `(142, 80)` the channel became 64, still one above Chromium;
the shared corner also overpainted. All 40 profiles remained different.
Chromium oracle identities and decoded hashes matched the clean run in
all 40 profiles. The experimental paint change was reverted because it did
not satisfy the exact gate. These diagnostics do not qualify a renderer build.

**Candidate owner:** `openui-paint` rounded-border coverage and layer compositing.
Ownership remains unreviewed in the qualification ledger because this
investigation does not yet account for every changed pixel. The next fix must
preserve fractional side coverage without duplicating
corner coverage, then match Chromium's channel rounding. It must pass all 40
primitive profiles, the exact neighboring fixtures, and the full four-profile
census without regression before the residual can be closed.
