# Native monochrome font policy

Open UI never executes JavaScript. Consuming applications select native Rust
options, create retained elements, and handle interaction through Rust callbacks.
Needed browser-style element behavior remains public native API work.
Pinned Chromium alone supplies expected pixels.

## Shared native API

`RasterConfiguration::chromium_linux_monochrome()` selects the pinned Linux
monochrome author-text policy: alias edging, full hinting, and no subpixel
positioning. Native control labels use their separate configuration. Select it
in `EngineOptions` before constructing the document. The main `openui` crate
and its prelude expose the constituent raster enums and settings directly.

[Chromium's pinned Linux font policy](https://chromium.googlesource.com/chromium/src/+/refs/tags/147.0.7727.50/ui/gfx/font_render_params_linux.cc)
forces full hinting when Fontconfig disables antialiasing. Its font service
passes that setting to Blink. Pinned Skia's monochrome no-hint path uses linear
FreeType advances; the full-hint path uses fitted advances. The preserved
8,640-configuration probe repeats 34,560 samples identically in two processes.
For both native font collections, a 16px Ahem space is 16.000030517578125px
without hinting and 16px with full hinting. The word is 144.00027465820312px /
144px. Strict intrinsic ceiling then produces 16.015625px / 16px and
144.015625px / 144px. No advance epsilon or output pixel correction is applied.

## Fitted monochrome repair

The native Rust application at clean `1576f01a` now matches **40/40
pictures and 1,920/1,920 bounds** across five scales, both font collections and
both explicit CPU paths, before and after a Rust text callback. The 3× glyph
difference is repaired: an old unhinted-mask workaround had drawn a second
row over already fitted monochrome glyphs. The shared paint policy now respects
the selected hint configuration. No font/test-ID/size/scale patch or reference
change is added.

This verifies the reduced native application. Complete focused and primitive
diagnostics are recorded here; original and expanded qualification remain open.
Every remaining rendering gap and needed public native Rust API remains open.
Open UI executes no JavaScript; pinned Chromium supplies expected pixels.

The [public example](../../bindings/rust/openui/examples/native_monochrome_font.rs)
supplies retained elements and a Rust text callback. The independently compiled
consumer adds explicit CPU-path selection and captures glyph metrics. It
retains the same document and reference input as the preceding measurement.
Its 40 independent processes repeat unchanged frames, pictures, measurements,
callbacks and weak teardown successfully. Sixteen fresh compiler records and
29 compiled paths are independently audited before target reuse.

| Configuration | Exact pictures | Exact bounds |
|---|---:|---:|
| ChromiumLinux CPU; registered fonts | 10/10 | 480/480 |
| ChromiumLinux CPU; system fonts | 10/10 | 480/480 |
| Direct Skia CPU; registered fonts | 10/10 | 480/480 |
| Direct Skia CPU; system fonts | 10/10 | 480/480 |

The scales are 1×, 2×, 1.25×, 1.5× and 3×. Each includes before and after the
callback. The existing three native font tests pass, preserving fractional
advances and registration, and the existing no-hint paint-policy guard passes.

The cause is the shared paint policy. Before the repair, all 80 full-hint
baseline markers match Chromium, and all 80 no-hint baseline heights match.
The markers change neither reference nor full-hint native pixels. At 3× the
fitted glyph outline extends from -39px to +10px relative to the baseline;
the old replay draws it a second time one physical row toward block start.
Skipping that replay for fitted monochrome strikes removes all 6,672/6,720
wrong pixels. Both CPU paths now match the same unchanged references.
Other legacy font/size/phase workarounds remain renderer debt; this measured
repair does not establish that those paths are qualified.

The [new immutable evidence](generated/native-fitted-monochrome-v1.json)
preserves 859 members / 3,518,756 stored bytes,
including the preceding failed comparisons, baseline markers, primary source,
fresh compiler records, repaired pictures, channel deltas and regions, and
actual terminal/audit/reaping proofs. The
[preceding evidence](generated/native-monochrome-font-v1.json) remains unchanged:
its clean `85f87e2b` app matched 16/20 pictures and 960/960 bounds, with the four
3× pictures failing. That is a historical measured result.

## Chromium font backend evidence

The pinned 147.0.7727.50 renderer installs `FontDataManager` only when its
sandbox is enabled. The existing capture harness uses `--no-sandbox`. Its
`FontDataServiceLinux:typeface/Freetype` argument alone therefore does not
prove that the reference selected the service's FreeType typeface. Pinned
source uses a Fontations scanner for the default Linux FCI manager. Actual
binary-internal typeface and strike tracing remains open. The local Skia C++
fontations file and vendored Skrifa hint file match the pinned primary source
byte for byte. The preserved Skia Cargo lockfile is not Chromium's GN
dependency selection; pinned Chromium's Rust directories use Skrifa v0_40.

## Explicit matrix diagnostics

The matrix runner accepts
`--monochrome-raster-config chromium-linux-monochrome` to measure the fitted
policy for the existing aliased author-text cohort. Real-font and other font
profiles retain their existing configuration. The default matrix selection
is unchanged. This option changes no Chromium fixture, font, capture argument,
or oracle identity; each native result records the configuration actually
passed to the executable, with a separate renderer-cache identity.

An explicit candidate run reports complete ID/profile selections separately
from the declared qualification policy. Even when every selected image is
exact, it remains a diagnostic and cannot satisfy the current release gate.
A matching full census and reviewed font-origin/settings behavior are needed
before changing the declared policy.

## Explicit monochrome matrices (diagnostic; policy not promoted)

At clean `f6d318e6`, explicit fitted monochrome text restores
**21 exact comparisons** in the selected strict-ceil regression cohort:
**48/84 → 69/84 exact**, with zero new exact losses. Four already failing
table comparisons worsen, so strict nonregression still fails. These are
21 selected original cases across four profiles, not a complete census.

| Matrix, complete 40-profile selection | Automatic policy | Fitted candidate |
|---|---:|---:|
| Focused | 640/640 | 640/640 |
| Primitive | 960/960 | 960/960 |

All six matrix reports have zero render errors. Fresh local libraries,
**3,368 comparison rows**, saved PNG bytes, decoded pixels and full difference
signatures are independently audited. Chromium fixture/capture/font inputs
and reference pixels stay unchanged. Source and target remain frozen until
actual owner completion, a new completed audit and saved reaping proof.

`--monochrome-raster-config chromium-linux-monochrome` is an explicit diagnostic
choice. It selects the existing aliased author-text cohort and preserves other
font profiles. Native cache identities record the selected policy; sharded
summaries reject mixed policies. The default selection and release gate remain
unchanged. Exact candidate matrices do not establish full renderer qualification.

The worsened `display-contents-table-002`/reference pictures have a one-column
caption shift at 1.25× and narrow glyph-column changes at 1.5×. Their root cause
and reviewed ownership remain open; a reduced public Rust table consumer is
needed. The last complete original census remains **21,329/22,924 exact**,
with **1,595 differences**, zero errors. No new complete original or expanded
census is claimed. Every needed native Rust API remains required; Open UI
never executes JavaScript.

[Immutable matrix and table evidence](generated/native-monochrome-policy-matrices-v1.json).

## Remaining work

The focused and primitive candidate diagnostics are exact. Run the original
and expanded census with matched font policies, repair the worsened table
comparisons, and qualify the complete declared policy. The last complete original result remains 21,329/22,924 exact,
with 1,595 differences and no errors. Every remaining needed native Rust API,
compositor, hardware and release gate remains required.
