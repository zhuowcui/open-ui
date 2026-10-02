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

## Qualification status

The complete original diagnostic run is 21,328/22,924 exact, with 1,596
differences, zero errors, and an observed terminal exit of 1. Compared with
the accepted C9 renderer, 116 images change: 83 improve, 13 already failing
comparisons worsen, and 20 become exact. No previously exact image regresses.
All 22,924 Chromium images and oracle identities remain unchanged, and the
source and executable stay fixed throughout the run. Compared with the
earlier rejected source-less V2 prototype, all 36 changed images improve and
26 become exact. The full run therefore confirms that every earlier exact
regression has been restored. The expanded suite and clean-source release
qualification remain outstanding.

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
