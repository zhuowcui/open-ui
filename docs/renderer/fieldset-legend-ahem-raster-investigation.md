# Fractional Ahem legend raster investigation

Chromium 147 remains the only pixel target. This investigation covers the
remaining native final-state addition
`wpt/css_display/display-contents-dynamic-fieldset-legend-001` at
1280×720@1.25. It does not qualify that case or change the renderer, fixture,
or Chromium capture.

At clean renderer checkpoint `ee0b04dd`, the case has 1,315 wrong pixels and
zero render errors. All wrong pixels lie in 14 one-pixel-high rows, at physical
`y = 61, 93, 112, 126, 141, 183, 198, 212, 227, 246, 261, 303, 318, 351`,
within bounds `x = 26..177, y = 61..351`. Open UI paints black at all 1,315
locations; Chromium paints white at 1,307 and gray `(191,191,191)` at eight.
The top and side pixels of these Ahem glyphs otherwise match. The generated
native final-state case and the original `-ref` fixture have byte-identical
Open UI images and byte-identical Chromium images at this profile, so the
failure is shared by the ordinary text paint path, not introduced by the
native mutation sequence. The other three required profiles are exact.

The clean images are retained in the ignored
`out/renderer-evidence/button-clip-expanded-ee0b04dd/desktop-1280x720@1.25/wpt/css_display/display-contents-dynamic-fieldset-legend-001/`
report directory. Their PNG SHA-256 values are
`e2868eca2192bfe9db457c6d1174471eb818c571298af4661ff6fbd8196256dd`
for Open UI and
`fcfb723c9995c3acb5f750bc3335ae47efc84a9febeba59200f597c4e9a29ddb`
for Chromium. The source `test.html` in that report has SHA-256
`ac249a03334dfe0067168cf56d9de1b8bdaf0e4c92c3f44de6aef4f222ac4266`.

Three local diagnostic changes were rejected. Snapping the small Ahem clip's
block end down removed the extra bottom rows but raised the difference to
2,318 pixels: it also changed partial coverage along vertical glyph edges.
Adding a separate block-end clip produced the same image. Keeping the
original clip and vertically scaling glyph drawing to that snapped end also
produced the same 2,318-pixel image. Those experiments were reverted; no
painter change was kept. The new side-edge differences are consistent with a
change in Skia's glyph mask selection or coverage in both axes.
The exact raster root cause remains unreviewed, so no ownership entry or
release pass is claimed. The next investigation needs a minimized Engine-backed
Ahem case and a comparison of physical-strike mask generation against
Chromium before changing the shared text raster policy.

## Reduced geometry at the first failing legend

Two independent Chromium processes at five scales agree on the first failing
legend's geometry and font selection. The [reduced HTML input](reproducers/fieldset-legend-fractional-origin-v1.html)
has the same `LEGEND` bounds and text range as the original document's second
legend: `x=22, y=39, width=60, height=10` in logical pixels. At scale 1.25,
its physical bottom is 61.25, beside the first differing row at `y=61`.
The original first legend ends at physical `y=40` and already matches; the
earlier reduction of that legend is a neighboring guard rather than evidence
of the failing pixels.

The [query evidence](evidence/fieldset-legend-origin-query-v1.json) preserves
all 20 full/reduced observations, pinned input and tool identities, and prior
probe errors. Chromium reports the Ahem face and six glyphs. With the Canvas
font explicitly reconstructed from CSS longhands, `LEGEND` measures 60px
wide, with 8px ascent and 2px descent. An earlier probe assigned an empty CSS
font shorthand to Canvas and measured its default font; those Canvas metrics
are invalid. Its DOM geometry and platform-font observations remain valid.

These queries generated no screenshots or Canvas pixels and ran no JavaScript
inside Open UI. They establish a reduced Chromium geometry input, not an
Engine-backed raster reproducer or a pixel pass. The native reduction and
exact shared raster cause still require verification. All original Open UI
and Chromium images remain unchanged.
