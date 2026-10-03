# Native SVG viewport investigation

Open UI runs no JavaScript. The diagnostic Rust consumer creates a
`foreignObject` viewport through a proposed public `Element::create` path,
changes typed styles, reads owned bounds and renders through the shared
Engine. The public API addition and renderer changes are still unapplied.
The [consumer and patch](evidence/native-svg-decoration-v1/) are reviewable
implementation evidence, not an admitted feature or release qualification.

## Sizing and border drawing

Chromium's `LayoutSVGForeignObject::UpdateSVGLayout` supplies fixed viewport
width and height to its block layout. Border and padding do not enlarge the
physical fragment. Open UI instead produced a 4×1 fragment for a 1×1 viewport
with a 3-pixel left border. Its paint path then clipped that wrong geometry
and applied an unexplained `49/50` opacity adjustment.

The prototype supplies the fixed viewport dimensions and removes that opacity
adjustment. Chromium's picture traces contain an opaque border fill through
rounded and rectangular clips, with no opacity layer. Chromium's `BoxBorderPainter` and `BorderEdge` clamp an edge to the snapped
box dimensions and make a double border narrower than three pixels solid.
For a single rounded edge that fills an empty inner contour, the shared
prototype uses that effective style and draws through the rounded outer clip
and authored side rectangle.

Chromium also snaps the SVG root's position before replaying local decoration.
A prototype that retained the unsnapped root position regressed 46 previously
exact native comparisons and is rejected. The later prototype separates the
snapped translation from local edge snapping. Its remaining SVG coordinate,
clipping and border behavior still needs qualification.

The [evidence index](generated/native-svg-viewport-v1.json) records the pinned
Chromium sources, picture traces, source snapshots and executable identities.
Every traced screenshot matches its retained Chromium reference. Reference
images and the old Open UI archive have not been rewritten.

## Native application measurements

The [manifest](evidence/native-svg-decoration-v1/cases.json) contains 96
script-free states, each tested at scales 1, 1.25, 1.5, 2 and 3. Controls cover
small and fractional dimensions, oversized solid and double borders, radius,
padding, box sizing, native children, opacity and vertical writing modes.

| Diagnostic implementation | Exact pixels | Exact owned bounds |
|---|---:|---:|
| Fixed viewport, existing shared borders | 74/480 | 480/480 |
| Empty-inner border correction | 114/480 | 480/480 |
| Unsnapped local-position experiment, rejected | 68/480 | 480/480 |
| Snapped root position and local decoration | 134/480 | 480/480 |

Against the first viewport prototype, the latest experiment makes 60 more
comparisons exact and loses no exact comparisons. However, 54 already failing
comparisons worsen and 346 still differ. This is incomplete renderer work.
The tiny double-border control is exact at all five scales. That single
control does not establish correctness for other SVG states.

The locked prototype workspace passes 8,492 Rust tests, with zero failures
and 13 ignored tests. Its new application API test checks that decoration
mutations, box sizing, cloning and resizing preserve viewport semantics.
The test-generated historical PNG was preserved and the tracked original
restored before later renderer source snapshots.

The 312 affected original-profile guards are 182/312 exact, with zero render
errors. The SVG changes make four comparisons exact and leave the other 308
native images unchanged against the preceding prototype. All 23 of the
earlier prototype's formerly exact regressions have been restored in this
selection. Against the accepted C9 renderer, 20 comparisons become exact,
none lose exactness, and 13 previously failing comparisons still worsen.
All Chromium images and oracle identities are unchanged. The focused and
primitive 40-profile matrices remain 640/640 and 960/960 exact, with unchanged
native and Chromium images against C9. These are development runs with stable
source and binary identities, not clean-source release qualification.

## Clean native checkpoints

The [clean evidence](generated/native-svg-viewport-v4.json) rebases the viewport
work onto private coverage-region source `6368057f`. Clean `4daf1876` exposes
`Element::create_svg_foreign_object` and the existing string constructor path
over the shared Engine. The consuming Rust application mutates decoration
from a Rust click callback, reads owned bounds and checks document teardown.
All 480 bounds, callback and teardown checks pass; its 134 exact pixel results
and all 480 images reproduce the earlier development prototype. The native
API unit test also checks decoration mutation, box sizing, cloning and resize.
Both complete 40-profile suites pass: 640/640 focused and 960/960 primitive,
with all 1,600 comparison invariants unchanged from the preceding candidate.
The affected original selection is 176/304 exact: the tiny SVG control is now
exact at every required profile, and the other 300 comparisons stay unchanged.

Clean `169fc7fe` follows Chromium's complex border path for a lone opaque
rounded double edge. It clips the outer and inner contours, restricts the
owning side, then draws the two stripes through inset rounded contours.
This shared painter handles all four physical sides without reading output
pixels or selecting a fixture. It makes 112 more of the original native
comparisons exact, reaching 246/480. All 240 changed images improve, no exact
image is lost, and all bounds, callbacks and teardown checks still pass.

Fresh repeated Chromium captures add 1,440 top/right/bottom controls. All
1,440 owned bounds and native callback/teardown checks pass; 694 pixel results
are exact. Across all four sides, 940/1,920 pixel comparisons are exact and
every owned-bounds check passes. The 980 remaining pixel failures require
further work on solid contours, rounded child clipping, group opacity and
fractional coverage. The double-border trial preserves every invariant in
the 304-comparison selection. These new controls are not admitted release
cases, and the Rust API and renderer patches remain unapplied to the umbrella.
Native SVG coordinates, transforms and C parity also remain open.

Disk exhaustion stopped earlier full, selected and neighboring-control
attempts; their partial outputs and failed/empty receipts are retained.
Clean inactive checkouts and reproducible target copies were reclaimed while
preserving all source commits, branches, successful executable pins and prior
evidence. The double-border 40-profile suites finish with observed exits 0:
640/640 focused and 960/960 primitive exact. All 1,600 comparison invariants
remain unchanged from the earlier clean SVG source. Eight read-only generator
checks, archive integrity and repository accountability also pass on clean
umbrella `71217f2b`. The complete original reruns later stopped on disk exhaustion;
expanded reruns and full candidate qualification remain pending. No complete
census count is inferred from these selections.
The [reviewable patches and consuming app](evidence/native-svg-decoration-v1/)
preserve the exact implementations and inputs used by the clean builds.

## Solid-border contour correction

Clean `d94b55f8` extends the shared curved-edge path to opaque solid borders.
Chromium's `Paint` clips the outer and inner contours; `DrawCurvedBoxSide`
fills the edge through a side clip that includes its corner tangents. Open UI's
previous solid-side clip removed tangent pixels beyond the straight border
width. A 24×18 viewport with a 3-pixel left border and 7-pixel radius lost 16
pixels in its two corners. The shared correction restores those pixels without
inspecting raster output or selecting a test case.

The original native controls reach 304/480 exact, gaining 58; top/right/bottom
controls reach 872/1,440, gaining 178. Combined, 1,176/1,920 are exact, with no
exact loss and all 368 changed images improved. All owned bounds, Rust callbacks
and teardown checks pass. The selected original guards preserve all 304
comparison invariants. The remaining 744 pixel failures still include rounded
child clipping, group opacity and fractional coverage. The patch is unapplied.

The solid-border raster suites and a debug workspace build stopped on disk
exhaustion. Their incomplete outputs, logs and terminal exits are retained.
Fifteen unused build executables have verified compressed copies; successful
pins remain unchanged. A workspace-package clean reclaims 12.9 GiB of
reproducible outputs after an earlier cleanup command rejected a wrong package
name. A smaller workspace retry is running in an independent clean checkout;
pixel qualification will run separately from Cargo builds. No incomplete run
is counted as a pass. Hosted umbrella `3836e9de` completes six successful jobs
and five skipped hardening jobs; skips remain unverified.

## Earlier development qualification status

The complete original diagnostic run is 21,328/22,924 exact, with 1,596
differences, zero errors, and an observed terminal exit of 1. Compared with
the accepted C9 renderer, 116 images change: 83 improve, 13 already failing
comparisons worsen, and 20 become exact. No previously exact image regresses.
All 22,924 Chromium images and oracle identities remain unchanged, and the
source and executable stay fixed throughout the run. Compared with the
earlier rejected source-less V2 prototype, all 36 changed images improve and
26 become exact. The full run therefore confirms that every earlier exact
regression has been restored.

The complete expanded diagnostic is 22,131/23,728 exact, with 1,597
differences, zero errors, and observed terminal exit 1. All 22,924 original
rows agree with the separate census. Every one of the 804 addition images
is unchanged against C9, retaining 200 of 201 additions exact across all
four profiles. The fieldset/legend addition still differs at scale 1.25 and
remains in the declared contract. All Chromium references and the executable
and source identities remain fixed. Clean-source release qualification is
still outstanding.

The standalone patch applies to the umbrella branch but remains unapplied.
Its measured executable also contains earlier unapplied opacity, clip,
generated-image and scroll prototypes. The measurements do not qualify the
standalone SVG patch on the accepted renderer or an ordinary consuming-app
build. These 96 states are not admitted release cases. The latest clean
accepted renderer remains 21,308/22,924 exact against Chromium, with 1,616
differences and zero render errors.

Native API, layout and paint owners must finish the remaining behavior and
repeat the required matrices from clean source before adoption. SVG position
and transform behavior outside these reduced controls also remains open.
