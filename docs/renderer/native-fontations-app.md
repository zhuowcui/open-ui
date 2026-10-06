# Native app with the real Fontations factory

Open UI never executes JavaScript. Applications use public native Rust methods
and Rust callbacks. Pinned Chromium supplies the expected pixels and element
bounds. Historical Open UI output remains immutable provenance.

## Native intrinsic sizing and fractional line height

Clean private `8f756039` completes its entire verification run. All **8,575
workspace tests pass**, zero failed and 13 ignored, and all **21 read-only
checks pass**. Rust/C/C++ ABI consumers preserve 113 exports and 30 layouts.
Both new public Rust callback guards execute and pass. All stages run under
one exclusive owner; source identities stay clean and unchanged.

The consuming Rust app constructs and changes elements through public methods,
then calls Rust click callbacks, queries owned bounds, and checks teardown.
Both runs perform **2,880 callbacks** and return byte-identical output across
**3,690 cases and 7,380 geometry states**. With viewport overflow explicitly
matched on both sides, **7,360/7,380 full rectangles** match pinned
Chromium, compared with 1,640/7,380 on the caption-only source.
There are **5,720 rectangle gains and 0 losses**.
These are geometry measurements; native pixels were not captured by this app.

The shared fix maps intrinsic contributions into physical positioned axes,
applies both minimum and maximum bounds, and lets a minimum win a conflict.
Fit-content uses the space left after insets and margins. A vertical box with
automatic width takes its laid-out column extent when insets or an actual
aspect ratio do not fix that width. Unicode soft breaks also apply to ordinary
font families. No fixture, font-name, font-size, backend or test-ID condition
is added by these changes.

Twenty states remain different: `in the box` in DejaVu Sans at 18.72px, in both
vertical directions, across five scales. The native natural inline size is
93px versus Chromium's 92px; its minimum is 33px versus 32px. Shared intrinsic
text advance and strike selection owns the investigation. Its precise cause
is not yet reviewed; the failed states remain open.

The fractional line-height fix follows the pinned Chromium conversion and
fixed-point calculations. At 16px, unitless `1.2` uses **19.1875px**, while
`120%` and a fixed `19.2px` use **19.203125px**. Authored percentages first
become fixed lengths at computed font size, including Chromium's integral
percentage conversion. The native guards cover both font families, three
writing modes and five scales against **270 repeated Chromium observations**.
They also check callback mutations, owned snapshots and weak-handle teardown.
The older approximate line-height assertion is replaced with Chromium's exact
value. Earlier failed guards and captures remain preserved.

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Original | 21,334/22,924 | 1,590 | 0 | 1 |
| Expanded | 22,137/23,728 | 1,591 | 0 | 1 |
| Focused | 640/640 | 0 | 0 | 0 |
| Primitive | 960/960 | 0 | 0 | 0 |

The original census changes **6 rows**, gains **0 exact comparisons**
and loses **0**. The expanded census gains **0** and loses
**0**. All **48,252 Chromium image and oracle records stay fixed**.
The exact focused and primitive gates retain their own actual results above.
**6 original comparisons have more wrong pixels** than the accepted
renderer; improved native geometry does not waive these failures.
Each of three sizing variants changes from 57 to 114 wrong pixels at 1.25×
and from 384 to 406 at 1.5×, the same unresolved differences as the caption
trial. These failures block applying the combined implementation.

The repeated native font app stays **200/200 images and 12,800/12,800 phase
cells and bounds exact**. Every native pair repeats; all images and geometry
remain unchanged from `2fcdc66d`. This selected Fontations configuration does
not qualify every font or raster setting. Full parity and reviewed ownership
of every residual are still required.

An earlier full sweep stopped at the disk-space guard after three profiles;
its partial results are not qualifying. The retry uses a fresh checkout, app,
probe and result directory. Moving the cache preserved all 66,093 files and
their original paths byte for byte. The slow serial transfer was stopped
before it changed the original cache, and its partial copy is preserved.

The [v7 evidence](generated/native-fontations-factory-v7.json) preserves the
complete census summaries, native apps and callbacks, fresh diagnostic
Chromium observations, all failed measured attempts, source checks and image
regions. Every archive member is hash-verified. An independent Git index
reconstructs all four measured failed and final source trees exactly from
public `9f0d2211` and their preserved patches. No oracle is rewritten and no
release state is admitted.

This source remains **private, unapplied and unqualified**. The accepted
original result stays **21,334/22,924 exact**. Hosted umbrella `376e0ded`
passes six executed jobs, with five skips, and all nine native/comparison
guards execute and pass. Those results do not qualify private `8f756039`;
skipped jobs are not release passes.

## Earlier caption sizing follow-up (`c83d24f3`)

Clean private `c83d24f3` completes the entire follow-up: **8,573 workspace
tests pass**, zero failed and 13 ignored; all **20 read-only checks pass**.
The native caption guard exercises public Rust element construction, mutation,
click callbacks, owned bounds and teardown at five scales and three border
spacings. The Linux Rust/C/C++ ABI consumers retain 113 exports and 30 layouts.
The upstream Fontations tests and exact RGBA guards execute and pass.

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Original | 21,334/22,924 | 1,590 | 0 | 1 |
| Expanded | 22,137/23,728 | 1,591 | 0 | 1 |
| Focused | 640/640 | 0 | 0 | 0 |
| Primitive | 960/960 | 0 | 0 | 0 |

Each census gains **0 exact comparisons and loses 0**
against the accepted renderer, with 6 changed rows. All **48,252 Chromium
comparison/oracle records stay unchanged**. The repeated native font app stays
**200/200 images and 12,800/12,800 phase cells and bounds exact**; its pixels
and geometry remain unchanged from `2fcdc66d`. Every native process repeats its
results and succeeds. These measured results do not qualify every font policy
or close the complete Chromium and native API gates.

The caption root cause is now reviewed. Caption minimum width and grid
border-box width constrain the same table wrapper. The old code added empty
grid spacing to the caption minimum. Unshaped text measured too narrowly and
accidentally canceled that layout error. With correct shaped measurement,
the table became 135px wide where Chromium uses 133px. The shared table fix
takes the maximum of the two constraints and removes that double count. It
uses no fixture, font-name, font-size, backend or test-ID condition. Both
original caption cases are exact at three profiles, restoring the six
regressions. Each retains the accepted one-pixel failure in the fourth profile.

A separate native Rust probe records **2,160** raw advances, shaped advances
and element bounds; its two outputs are byte-identical. On earlier source
`02401648`, five-scale Chromium queries match **535/540 natural inline sizes
and 260/540 minimum inline sizes**. Vertical `min-content` height still uses
line thickness in cases where Chromium uses an unbroken word's advance:
16px Ahem `min` is 16px natively versus 48px in Chromium. Shared intrinsic and
positioned sizing owns this remaining API investigation; its precise fix is
not yet reviewed. Six existing sizing pixel differences still worsen: each
of three variants changes from 57 to 114 wrong pixels at 1.25× and from 384
to 406 at 1.5×. They remain failures and block applying this trial.

The fresh Chromium diagnostic adds only `--disable-dev-shm-usage` to its
process launch. Its five reduced capture/query pairs and six original
capture/query pairs repeat exactly. All six original captures retain the
immutable Chromium RGBA bytes. This storage flag and these selected captures
are recorded as diagnostic evidence, rather than global release harness
qualification. JavaScript executes only inside the separate Chromium process.
Open UI uses native Rust throughout.

The [v6 evidence](generated/native-fontations-factory-v6.json) preserves all
complete census records, focused/primitive and native results, changed image
sets with bounds/regions/channel deltas, fresh Chromium geometry, failed
native builds and browser captures, and the incorrect first absolute-table
guard. Each archive member is hash-verified. An independent Git index
reconstructs the entire measured tracked tree from public `9f0d2211` and the
compressed patch. The earlier failed formatter check remains preserved;
all seven copied upstream Rust files now pass on the measured source.

The trial remains **unapplied and unqualified**. Accepted original pixels
remain **21,334/22,924 exact**. Umbrella `739a9f25` completes its hosted checks:
six executed jobs pass, five skip, and all nine existing native/comparison
guards execute and pass. These hosted results do not qualify the private
caption implementation; skipped jobs do not count as release passes.

## Earlier complete attempt (`02401648`, before the caption fix)

Clean private source `02401648` completes the entire qualification run. All
**8,572 workspace tests pass**, with zero failures and 13 ignored. The Linux
Rust/C/C++ ABI consumers pass with 113 exports and 30 layouts. Both exact RGBA
comparison guards execute and pass; generated diagnostics and every reference
remain outside source mutations. All 18 read-only checks pass.

The native Rust app still matches **200/200 Chromium images and
12,800/12,800 phase cells and bounds**. Every repeated native process succeeds,
all PNG and geometry results repeat, and every native row matches the prior
`2fcdc66d` result. The full renderer gate nevertheless fails:

| Suite | Exact | Different | Errors | Actual exit |
|---|---:|---:|---:|---:|
| Original | 21,328/22,924 | 1,596 | 0 | 1 |
| Expanded | 22,131/23,728 | 1,597 | 0 | 1 |
| Focused | 640/640 | 0 | 0 | 0 |
| Primitive | 960/960 | 0 | 0 | 0 |

Both complete censuses lose **six exact comparisons and gain none**. Fourteen
rows change in each census, across five original test IDs. The six lost exact
results come from `multicol-span-all-004` and its reference at three profiles.
Their existing fourth-profile failures also worsen. Three existing
`block-size-with-min-or-max-content-1` variants worsen at two desktop profiles.
The earlier selected 20-test suite did not include these cases. All 48,252
Chromium comparison/oracle records remain unchanged; the focused and primitive
rows retain all nine comparison invariants. Shared intrinsic text measurement
owns the new investigation. The precise cause and a reduced native probe still
need review. No new residual is qualified or silently assigned a completed
cause.

The [versioned evidence](generated/native-fontations-factory-v5.json) preserves
both full census summaries, native runs, complete focused/primitive results,
all fourteen regression image sets and their channel/region metrics, failed
workspace preparations, and hosted job logs. An independent Git index
reconstructs the measured source and its formatted follow-up exactly from
public `9f0d2211` and their compressed patches. Every archive member is
hash-verified. Full-census individual image caches remain at their measured
local paths; the archive retains their complete comparison records and the
changed images.

The first full attempt failed nine upstream font tests because their four
fonts were missing. The exact pinned Skia inputs are now included with hashes;
all original assertions pass. A subsequent attempt stopped after 221 passing
tests when the disk guard fired. Both partial runs remain preserved. Older
terminal binaries are relocated with their recorded hashes checked and their
original paths retained. No reference or failed result is rewritten.

A separate required formatter check finds indentation and line wrapping in two
upstream Rust files. Fresh private follow-up `24155c19` fixes that formatting
and records original/local hashes. All **19 read-only checks pass**; its runtime
and renderer gates have not run. Neither source is applied to the umbrella,
which promotes only the exact legacy comparison-helper correction. Umbrella
`b8527c6e` has six executed PR jobs passing and five skipped; all seven existing
native guards and both comparison guards execute and pass. Skips and the
unqualified trial are not release passes. The accepted original result stays
**21,334/22,924 exact**; v0.2 is unfinished.

## Exact text diagnostics

The legacy text comparison helper now reads surface pixels in PNG RGBA order
with straight alpha, rejects any changed RGBA channel, and writes generated
images to ignored `out/pixel_text/openui_renders/`. It preserves the tracked
historical renders and Chromium references. Guards cover a one-bit change in
each channel, an alpha-only diff image, channel order, and translucent colors.

These guards execute successfully on private source `02401648`. Its full
workspace reports 8,572 passed, zero failed and 13 ignored; Rust/C/C++ ABI
verification passes. The complete renderer result is recorded above. The ten
legacy scenarios do not count as Chromium passes when reference files are
absent. The complete Chromium matrices remain the release pixel gates.

## Shared measurement follow-up

Clean private source `2fcdc66d` now matches **200/200 Chromium images and
12,800/12,800 element bounds**. The native gate exits **0**. All 200 PNGs stay
byte-identical to the real factory trial. The only geometry changes are the
1,280 previously short widths, each increased by exactly 1/64 CSS pixel.
Every reference image and query remains unchanged; all 200 native processes
repeat their results and pass callback, owned-snapshot and teardown checks.

The correction changes shared text measurement. Intrinsic sizing now measures
text through `TextShaper`, and ceil-converts the shaped width without removing
small positive fractions. It adds no font, size, backend or test-ID condition.
A reduced native Rust consumer records 2,100 raw advances, shaped advances and
public element bounds across seven explicit policies. Its two runs are
byte-identical. At 16px, thirteen Ahem characters have a raw width of
208.000396729px. Aliased shaping produces 208px; measuring the raw value and
then rounding upward creates the wrong extra layout unit. LCD shaping retains
its fractional advance, which must survive the grid ceiling. This explains
why removing the old normalization alone was insufficient.

The actual renderer retains all nine comparison invariants in **80/80 rows**
for the 20 previously affected tests at four profiles. All 21 earlier exact
regressions are restored. The selected gate is still **68/80 exact, 12
different, zero errors, exit 1**, matching the accepted renderer exactly.
Those twelve existing failures remain open. Complete focused **640/640** and
primitive **960/960** matrices pass with all 1,600 comparison invariants
unchanged. All **1,013 text and layout tests** and **17 source checks** pass.

The [versioned record](generated/native-fontations-factory-v3.json) preserves
the reduced consumer, failed preparations, clean source, repeated native
images, complete selected/focused/primitive results, paired audits and hosted
logs for umbrella `3a7f0f75`. Its six PR jobs pass and five skip; no full manual
hardening run is attributed to that head. The
[source patch](evidence/native-fontations-factory-v3/reproduction-from-9f0d2211.patch.gz)
applies to public checkpoint `9f0d2211`; an independent Git index reconstructs
the entire private source tree exactly. Every archive member is hash-verified.

This source remains private and unapplied. Original and expanded censuses,
full workspace, MSRV, C/ABI, remaining font settings and release qualification
have not run on it. It admits no release state or formal original residual
owner. The last accepted census remains **21,334/22,924 exact**. The native
result does not establish full project parity.

## Completed result at `9e4baa08`

Clean private source `9e4baa08` renders **200/200 native app images exactly like
Chromium**. It gains the remaining 38 exact images over the earlier 162/200
outline trial and loses none. All 12,800 measured logical phase cells have
exact pixels. The full original renderer census has not run on this source.

The Rust app creates 64 text positions, draws `X` in black, then uses a Rust
click callback to change the text to `XX` and the color to blue. Four font
families, five sizes and five scales produce 200 before/after images. Two
independent native runs check each combination: all 200 processes finish,
their PNG bytes repeat exactly, callbacks fire once, owned options and bounds
remain usable, and weak element handles expire after teardown.

Every Chromium image, font asset, input and repeated reference query remains
unchanged. This result uses the existing references; it creates no new oracle
bytes and does not replace a frozen reference.

| Scale | Exact images | Total |
|---|---:|---:|
| 1 | 40 | 40 |
| 1.25 | 40 | 40 |
| 1.5 | 40 | 40 |
| 2 | 40 | 40 |
| 3 | 40 | 40 |

## What changed

The [standalone factory prototype](native-fontations-factory.md) is combined
with the current branch's native font resolution. An explicit immutable
`RasterConfiguration::chromium_linux_fontations_lcd` selects the real
Fontations typeface and scaler from the existing Skia pin. System and
application registries still select the face. Its bytes, collection member,
variation and palette arguments reach the native factory; shaping and painting
then use that typeface.

The new path uses ordinary Skia text blobs. It bypasses the custom outline
adapter and the legacy FreeType glyph-origin compensation. It does not choose
an engine by font name, size, test ID or environment, and does not patch output
pixels. Unsupported factory data returns no face instead of silently switching
to FreeType. The existing default and explicit FreeType choices remain in the
source and require complete regression qualification.

The candidate passes all **344 text tests** and **16 read-only source checks**.
Its metrics guard verifies a real `fnta` typeface and all 30 independent
Chromium metrics observations. Fixed hinting requests remain fixed at the five
tested scales. This source retains the integrated variable-font correction;
it does not inherit the old descriptor trial's 29 census losses.

## Earlier geometry failure at `9e4baa08`

Element bounds remain **11,520/12,800 exact**. All 12,800 native bounds are
unchanged from the outline trial. The remaining 1,280 differences affect width
alone, each exactly 1/64 CSS pixel short. No difference is tolerated: the
complete native gate exits **1**, despite the exact image pixels.

The existing [intrinsic width investigation](native-intrinsic-snap.md) identifies
a shared helper that discards small positive shaped fractions before rounding
to the layout grid. Its earlier general correction closes native widths but
loses 21 exact original-renderer comparisons, so it remains rejected. The real
factory does not remove this allocation issue. Text and layout still need a
reviewed measurement fix and guards for the earlier losses; no font-specific
width adjustment is justified.

## Evidence and reproduction

The [versioned record](generated/native-fontations-factory-v2.json) preserves
the completed owner, native app sweep, full paired audit, reference identities,
source checks, source files and all failed attempts. Every member of the
[evidence archive](evidence/native-fontations-factory-v2/completed-evidence.tar.gz)
is hash-verified.

The [complete source patch](evidence/native-fontations-factory-v2/reproduction-from-9f0d2211.patch.gz)
decompresses to the original patch bytes and applies to public checkpoint `9f0d2211`. An independent Git index reconstructs
the candidate's entire tracked source tree exactly. The new crate vendors the
113 unchanged pinned headers needed by the factory and does not need the
prototype's external include-path variable. Source reconstruction is not a
new render or a relabelled source measurement.

The first current-branch build records its source before an unlocked Cargo
clean completes dependency resolution. Its 344 passing text tests and app
build therefore have dirty source identities. The native worker also fails
its owner-environment preflight before running an app. Its incomplete receipt
and source remain unchanged; a separate terminal observation records the
finished processes. A fresh retry uses the complete lockfile, a regenerated
dependency-policy artifact, locked cleaning, a clean commit and the correct
owner environment. All measured retry stages retain their clean source
identity. Earlier file-mode and header-declaration preflight failures are also
preserved and are not passes.

## Remaining qualification

This candidate is private and unapplied. It admits no release state or formal
original-census residual owner. Its original, expanded, focused and primitive
renderer matrices, full workspace tests, MSRV, Miri, C ABI consumers and
packaging have not run. Native C selection, collection and variation behavior,
palette effects, compressed fonts and every configuration field still need
behavior qualification. Default native rendering, other glyphs, writing modes,
effects and all remaining public element APIs stay required.

The accepted complete census remains 21,334/22,924 original and
22,137/23,728 expanded exact, zero errors and actual exits 1. These counts are
not results for the new source. Matching this native font corpus does not
declare full renderer or release qualification.

Separate umbrella checkpoint `412e804c` passes all three PR workflows: six
jobs pass, five skip and none fail. All seven native guards actually run and
pass in hosted parity. Full manual hardening has not run on that checkpoint.
Hosted results do not waive the candidate's geometry or other open gates.
