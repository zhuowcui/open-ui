# Vertical-lr Ahem rotation anchor

Chromium 147 is the only pixel target. This repair changes the shared text
paint path, not a WPT fixture, a Chromium capture, or a completed image.

## Reduced mismatch

The native final-state case
`wpt/css_multicol/crashtests_table-cell-writing-mode-root` contains a
`display: table-cell` element with `writing-mode: vertical-lr`, two columns,
and one inline Ahem `x`. At the 375×667@2 profile, the text fragment begins
at CSS x=20.1875 with a 16 CSS px width. Before the repair, Open UI painted
the black 32×32 physical glyph at x=41–72; Chromium painted it at x=40–71.
The two one-column regions each held 32 wrong pixels, for 64 total. The case
was already exact at the other three required profiles. The source template
itself is the reduced Engine-backed reproducer; it has one visible glyph.

The clockwise text path translated its canvas to the fragment's right edge
before rotation. That translation could retain a fractional device-column
phase for vertical-lr Ahem. The one-column displacement is consistent with
Skia selecting the next mask column for the whole glyph at 2×. The repair
aligns that rotation anchor to the nearest physical column for aliased Ahem
in vertical-lr. It leaves layout geometry, glyph advances, and other writing
modes unchanged; it does not edit the finished image. There is no test ID,
viewport, color, or output-pixel branch.

## Guard and rejected broader rule

A diagnostic that aligned every clockwise aliased Ahem run was too broad. In
1,408 comparisons over all 352 original fixtures whose source contains
vertical or sideways writing, it made nine comparisons exact but regressed
two formerly exact `flexbox-align-self-baseline-horiz-006` comparisons at
1.5× and worsened two already failing `-008` comparisons. Those cases use
vertical-rl or sideways-rl anchoring. That rule was rejected.

Restricting the shared rule to vertical-lr changed nine of those 1,408
comparisons, made all nine exact, worsened none, and removed 30,184 wrong
pixels. The newly exact original comparisons are the mobile 2× versions of
`flexbox-writing-mode-010`, `-010-ref`, `-014`, `-014-ref`, `gap-007-lr`, and
`gap-007-lr-ref`, plus the 1.5× versions of
`vert-block-size-small-or-larger-than-container-with-min-or-max-content-2-ref`,
`-2a`, and `-2b`. The pending native final-state table-cell case also became
exact at all four required profiles.

## Clean qualification evidence

At clean commit `e14e3e64`, the rebuilt `pixel_compare` binary had SHA-256
`ff6dd728e3eb5e80438c554a4ecfcf680b6cafd405bca41f4e8d7b785ea6d84a`.
The complete [v39 original census](generated/four-profile-census-v39.json)
is **21,264/22,924 exact**, 1,660 different, and zero errors. Exactly nine
Open UI decoded images changed from v38, all nine became exact, and no exact
comparison regressed. All 22,924 Chromium oracle identities and decoded
images stayed fixed. The raw clean report is preserved at
`out/renderer-evidence/vertical-lr-full-cleanbinary-e14e3e64/full-summary.json`
with SHA-256
`efc0c383caa81227ff48a17b72019e18df7158f59a97a05ce7c2bc2e796974d4`.
The earlier full diagnostic and the clean rebuilt-binary run have identical
decoded Open UI images, statuses, and wrong-pixel counts across all 22,924
comparisons.

The clean [v40 raster index](generated/focused-primitive-raster-v40.json)
is 640/640 focused and 960/960 primitive exact across their 40 profiles.
The complete [v22 expanded requalification](generated/expanded-requalification-v22.json)
is **22,066/23,728 exact**, 1,662 different, and zero errors; it includes all
201 admitted additions. Exactly one added image changed from the prior
expanded run: the table-cell glyph at mobile 2×. The addition is now exact at
all four profiles, so 199 of 201 additions pass. Every original expanded
result matches the separate clean v39 census, and all 23,728 Chromium oracle
identities and decoded images match the prior expanded run. The raw expanded
report has SHA-256
`90be466204d91b46e32c600774f67ad60a7384887a07a54b74074480cabf1428`.

The two remaining admitted addition failures are
`display-contents-dynamic-fieldset-legend-001` at 1.25× and
`containing-block-change-button` at 1.25×. The complete original census still
has 916 unowned residual test IDs, so this repair does not qualify the final
renderer release gate.
