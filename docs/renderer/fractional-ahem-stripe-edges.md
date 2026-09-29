# Fractional Ahem stripe edges: open investigation

The clean [v35 Chromium census](generated/four-profile-census-v35.json) shows a
shared 1.25× raster difference in the multicolumn rule tests. Chromium is the
pixel target; the historical Open UI archive is not an expected image. The
affected WPT source sets `font: 3.125em/1 Ahem`, black text on a black box,
and a 10 CSS px green rule or span border. Its font size is 50 CSS px, or
62.5 physical px at the failing scale.

| Case | 1× | 2× | 1.25× | 1.5× |
|---|---:|---:|---:|---:|
| `multicol-rule-none-000` | exact | exact | 189 different pixels | exact |
| `multicol-rule-solid-000` | exact | exact | 813 different pixels | exact |
| `multicol-rule-solid-000-ref` | exact | exact | 1,250 different pixels | exact |

The plain reference has no multicolumn layout and still differs. It is a
reduced existing corpus reproducer for the shared text and border edge issue;
the `rule-none` source provides a nearby case without a painted rule. For the
plain reference, the four connected mismatch regions are the right stripe at
`(287,25)` and `(287,100)`, each 251×63 with 375 changed pixels, and bottom
rows at `(25,87)` and `(25,162)`, each 250×1 with 250 changed pixels. Alpha is
unchanged throughout. At `(25,87)`, Open UI produces RGB 127 while Chromium
produces RGB 63; the same pair occurs at `(25,162)`. Most differences on those
rows are that 64-channel gap. This is consistent with a second half-covered
black glyph edge composited over a half-covered black box edge in Chromium,
but the precise raster operation remains unconfirmed. Along the green border
edge, Open UI RGB `(0,65,0)` versus Chromium `(0,64,0)` occurs on 124 pixels;
that one-level difference may have a separate rounding cause.

Two local diagnostic trials were rejected. Enabling the strong aliased outline
path for all fractional Ahem runs changed none of these 12 images because the
selected test raster policy uses LCD author text. Removing Ahem from the
special author-LCD exclusion also changed none: that route is gated on aliased
author text. Both trials were reverted; neither is a repair or a qualifying
run. The renderer code remains unchanged by them.

Investigation owner: text and border rasterization in `openui-paint`. A
reviewed root cause, a source-level repair, and neighboring 40-profile and
complete four-profile regression checks remain open. These tests retain their
failing status until their Open UI bytes match the pinned Chromium captures.
