# Open UI

Open UI is a typed, reactive desktop and headless UI framework built around one
pure-Rust renderer. Version 0.2 targets Linux on x86-64 and AArch64, with X11,
Wayland, OpenGL presentation, automatic software fallback, and a co-equal C ABI.

```text
safe Rust API ─┐
               ├─> openui-engine ─> style/layout/text ─> immutable scene
typed C ABI ───┘                                           │
Linux events ─────> interaction/accessibility ─────────────┼─> OpenGL window
headless clock ────────────────────────────────────────────└─> exact Skia raster
```

The supported application path has no Blink/Chromium runtime, resource pack,
HTML loader, CSS text parser, JavaScript engine, or network stack. Chromium 147
is the pinned rendering reference used to prove renderer compatibility.
Open UI never executes JavaScript, in this or future versions. Applications
handle interaction in native Rust through `openui::Document`, `openui::Element`,
signals, and Rust event callbacks. Document lookup by ID,
element and text-node mutation, class lookup and updates, focus, scrolling,
controls, and event handling use public Rust methods. Element operations do
not require JavaScript: browsers expose them through JavaScript, and Open UI
must implement any needed equivalent in Rust and expose it as a public method
on the retained document or element. The consuming native app calls that
method directly. A missing public native operation is unfinished API work,
even when an internal test fixture can already produce the same visual state. See the
[native interaction contract](docs/v02/supported-platforms.md#native-interaction-api).

## Verified status

The current v0.2 release candidate has:

- a historical archive of 5,731 Open UI renders, with 5,549 byte-identical
  on replay and 182 changed; these old screenshots are not pixel targets;
- a [complete clean umbrella census](docs/renderer/generated/native-nested-scroll-v9.json)
  at `d174ea0b` with 21,334 of 22,924 comparisons exact, 1,590 different, and
  zero render errors; all Chromium images and identities remain unchanged,
  and the full pixel gate still fails;
- clean 40-profile raster matrices with 640/640 focused and 960/960 primitive
  comparisons exact in the [clipping repair evidence](docs/renderer/generated/native-viewport-full-v2.json);
- 201 native final-state cases in the expanded contract, including one newly
  added case; 200 of 201 meet the four-profile gate in the earlier complete
  viewport run, and one remains a failure in the
  [expanded evidence](docs/renderer/generated/native-viewport-full-v9.json).
  The other 35 AST-lowered cases remain pending;
- a 7/7 repository accountability audit over all 7,673 inventoried tests;
- 58 application scenarios covering retained updates, controls, editing,
  accessibility, resources, scrolling, animation, bidi, and multi-document use;
- generation-checked Rust and C handles, deterministic manual clocks, immutable
  scenes, X11/Wayland operation, software presentation, and OpenGL upload;
- 84 frozen retained-engine/headless C exports, with 113 current exports and
  checked layouts and an ABI checksum; the [native scroll and inset consumers](docs/v02/native-scroll-metrics.md)
  and [native style consumers](bindings/rust/openui-ffi/README.md#native-style-operations)
  pass through public Rust, C and C++ APIs; the clean umbrella checkpoint
  passes 8,528 workspace tests with 13 ignored at `580539c2`; its own
  640/640 focused and 960/960 primitive pixel suites remain exact, with all
  1,600 comparison invariants unchanged;
- sanitizer, Miri, fuzz, leak, latency, idle-work, and package gates defined
  in CI; several remain open or failing.

Chromium is the sole pixel target. The archived Open UI bytes disagree with
Chromium for some fixtures, which is why replaying old screenshots cannot be a
release gate. A [font oracle audit](docs/renderer/scaled-lcd-hinting-oracle-investigation.md)
also found one older cached Chromium capture that differs from six fresh
captures under the same recorded identity; that evidence needs reconciliation.
The latest clean umbrella census has 1,590 differences. Against the SVG
checkpoint, nine comparisons become exact and none lose exactness, while four
already failing comparisons worsen. Every residual still requires review.
Earlier renderer measurements below remain attributed to their named sources. The
[private sampling candidate](docs/renderer/generated/native-viewport-full-v14.json)
loses 23 exact Chromium comparisons and gains 14 in its complete census;
it remains unapplied. A subsequent
[generated-tile format trial](docs/renderer/generated-image-sampling.md)
repairs 18 of those regressions in the affected selection, preserving all
850 exact native controls. A later opaque-layer composition trial restores
four more exact comparisons in that selection, leaving one SVG pixel
regression. A subsequent [coverage-region trial](docs/renderer/generated/native-viewport-full-v17.json)
preserves those four matches and all 850 existing native controls while fixing
transparent-canvas erasure in neighboring Rust consumers. The SVG decoration
alpha, native scroll extents, remaining control pixels, and complete candidate
qualification remain open before promotion.
The [clean private SVG work](docs/renderer/generated/native-svg-viewport-v11.json)
adds native viewport creation and corrects shared curved-border painting.
Two complete original censuses finish at 21,325/22,924 exact, zero errors:
17 comparisons become exact, none lose exactness, and 13 already failing
comparisons worsen. The solid-border source also passes both complete
40-profile raster suites. These private results do not replace the accepted
renderer's census. The work is now rebased over the current native APIs.
Its Rust consumers preserve every SVG and scrolling image; 1,176/1,920 SVG
states and all 850 existing scrolling controls match Chromium exactly.
All SVG owned bounds, callbacks and teardown checks pass. C/C++ consumers
verify the shared viewport constructor and native sizing callbacks, preserving
110 exports and existing layouts. Its Linux-enabled workspace passes 8,516
tests with zero failures and 13 ignored. Its fresh 40-profile matrices pass
640/640 focused and 960/960 primitive comparisons, with unchanged results.
Its own complete original census also finishes at 21,325/22,924 exact,
1,599 different and zero errors, with all comparison invariants unchanged
from the double-border source. Its complete expanded run is 22,128/23,728
exact, 1,600 different and zero errors. All original rows agree between the
two suites, and all 804 additions stay unchanged: 200/201 cases meet all four
profiles. Both complete pixel gates still fail.
Complete rebased qualification, the 744
remaining SVG pixel failures, transforms and scrolling ranges remain open.
The [reviewable source patch](docs/renderer/evidence/native-svg-decoration-v1/native-svg-rebase-api-v293.patch)
is applied in this umbrella checkpoint together with the reviewed scrolling
repair and native reveal API. Its complete clean umbrella pixel gate still fails.
Earlier disk failures are preserved.

The [nested scrolling work](docs/v02/native-scroll-metrics.md#nested-scrolling-candidate)
implements shared native dimensions, ranges, offset rounding and detached
queries. Its earlier complete `6e255ca3` run is 21,321/22,924 original exact
and 22,124/23,728 expanded exact, zero errors. Eight comparisons worsen and
four lose exactness because anonymous lines omit child overflow. The next
clean `02c0296e` repairs that shared propagation and native sticky invalidation.
It restores all eight comparisons in the partial sticky sweep, matches all
50 reduced Rust application states, and gains 12 exact native scroll images
with zero loss versus `6e255ca3`: 1,775/2,560 pixels and 2,320/2,560 dimensions.
Its shared Rust/C constructor passes nine C and four C++ consumer processes;
8,516 Linux-enabled workspace tests and both 40-profile raster gates pass.
Its complete original census is 21,332/22,924 exact and expanded is
22,135/23,728 exact, zero errors, with actual exits 1. All Chromium inputs stay
unchanged. It gains nine original exact matches but loses two flex-overflow
matches at scale 1.25; three fragmentation comparisons also worsen. Six paint
reviews remain open, so this source cannot be promoted. A later clean block
scrollbar candidate at `b5a2044f` repairs all 240 missing geometry states:
2,560/2,560 match Chromium. Pixels remain 1,775/2,560 exact, with no exact
image lost. Its workspace, C/C++ consumers, 50 reduced Rust states and both
40-profile raster matrices pass. Its partial sticky gate still fails in four
states, and no complete census is inferred for that source.
[Versioned evidence](docs/renderer/generated/native-nested-scroll-v7.json)
and unapplied patches preserve earlier failures. These private results do not
replace the accepted renderer's census.

The complete `3f95e617` runs finish at 21,319/22,924 original and
22,122/23,728 expanded exact, zero errors, with actual exits 1. They expose
15 lost exact comparisons against the SVG checkpoint. The shared scrollbar
capture precedence and clip-margin correction at `83d45e0c` restores all 15
in the affected selection: 157/172 exact, 15 different, zero errors. Four
already-failing comparisons still worsen against SVG; no full result is
inferred for this correction.

This umbrella checkpoint applies the reviewed SVG and scroll repairs and adds
public Rust `scroll_into_view` and `smooth_scroll_into_view`, backed by the same
Engine operation as accessibility and two additive C functions. The private
API source passes 35 Engine tests and ten C/four C++ consumers; all 30 reduced
native geometry states match Chromium, while 16/20 endpoint images are exact.
Scroll-margin/padding support, broader alignment coverage, four scale-1.25
pixel failures and two legacy contour calibration paths remain open.
[Versioned evidence](docs/renderer/generated/native-nested-scroll-v9.json)
preserves every earlier failure. Own clean umbrella runs complete at
21,334/22,924 original and 22,137/23,728 expanded exact, zero errors, with
observed exits 1. All original rows agree, all 804 additions stay unchanged,
and 200/201 additions meet all four profiles. The workspace, ten C/four C++
headless consumers, 249 Python tests and ten read-only checks pass.

The font-cache lifetime fix is applied and pushed at `a41fdeb9`. Its own
Linux-enabled workspace passes 8,526 tests, zero failures and 13 ignored;
all ten read-only checks pass, with source unchanged. The
[latest lifetime evidence](docs/renderer/generated/native-font-cache-lifetime-v4.json)
records complete clean private original and expanded pixel sweeps. Every
comparison invariant agrees with `d174ea0b`: the fix changes no rendered
pixels and leaves the full pixel failures open. Its earlier local sanitizer,
fuzz and ABI checks and failed attempts remain preserved.

All three pull-request workflows at this checkpoint pass. The separate
[manual hardening run](https://github.com/zhuowcui/open-ui/actions/runs/37149510887)
passes all seven jobs: address/leak sanitizers, Miri, C UBSan, Linux windows,
MSRV and all five fuzz targets. Skipped pull-request jobs remain open results.
The [latest private scroll evidence](docs/renderer/generated/native-scroll-insets-v9.json)
records complete `dac78e25` runs at 21,340/22,924 original and 22,143/23,728
expanded exact, zero errors, with actual exits 1. Eight comparisons become exact
but two lose exactness; the screen correction remains unapplied. The two losses
are owned by painting and must be repaired before promotion.

A separate clean `cf59ea29` combines compatible nested rectangular clips before
rasterization. Its C matrix now matches all 175 geometry and pixel states,
repairing the remaining ten clip differences with no exact losses. All 510 Rust
states and all 1,600 focused/primitive comparisons stay unchanged and exact.
Its workspace passes 8,532 tests, zero failures and 13 ignored; ten read-only
checks pass. Complete original and expanded runs finish at 21,340/22,924 and
22,143/23,728 exact, zero errors, actual exits 1. Every comparison invariant
agrees with `dac78e25`, including both known exact losses.

The separate `6d6768a8` fixes native reveal traversal through layout containers
and containing-block ownership when public transforms are added or removed.
All 90 Rust and 60 C geometry and pixel states match Chromium; the C test gains
25 geometry matches and 15 pixel matches without losing an exact image.
All 510 existing Rust states stay byte-identical. Its workspace passes 8,535
tests, zero failures and 13 ignored, and ten read-only checks. Both raster gates
pass with all 1,600 comparison invariants unchanged. The viewport cutoff at
`2cc950e0` recovers five C geometry/image matches, preserving all 660 previous
states and both raster gates. A wider check then finds ten failures for
viewport-fixed controls on an already-scrolled page.
The combined `4dd50621` repair passes all six fresh build stages, 8,538 workspace
tests, ten read-only checks, all 935 native Rust/C geometry/image states and
both raster gates. It preserves all 885 earlier states and recovers all ten
fixed-control failures. Its complete censuses finish at 21,341/22,924 original
and 22,144/23,728 expanded exact, zero errors, actual exits 1. Six comparisons
lose exactness against `cf59ea29` and five against the applied umbrella; all
Chromium inputs and 804 addition results remain unchanged.
The included scroll-edge change at `c5769f2d` repairs the two earlier exact
losses in a 144-comparison selection, but introduces three new exact losses
against `dac78e25` at 1.5 scale. That blocks promotion; reduced results do not
establish a complete census result.
Reviewable source patches are retained; these private changes
remain unapplied and unqualified, and no new release passes are admitted.

The [new scroll investigation](docs/renderer/generated/native-scroll-insets-v11.json)
records a missing native trailing-margin extent and a clip policy applied to
an `auto` box without a scroll transform. The high-DPI candidate passes its
native consumers and focused matrix, but fails eight primitive comparisons.
A clean Rust follow-up passes 8,539 workspace tests, all 935 earlier native
states and both exact raster matrices; the eight gradient regressions are
repaired. All 105 new dimension queries match, but 14 opaque white images and
20 wider collapsed-margin API states still fail. Complete image censuses are
running. The [latest guards](docs/renderer/generated/native-scroll-insets-v12.json)
retain every failure; the patch remains unapplied and unqualified.
The [collapsed-margin follow-up](docs/renderer/generated/native-scroll-insets-v13.json)
retains shared layout data and adds a consuming Rust app. Its ten read-only
checks pass; its clean build and native probes are queued behind the immutable
image sweeps. All 765 fresh Chromium metric states are reference observations,
not native passes. No rendering or API qualification is claimed for this source.

The [v14 follow-up evidence](docs/renderer/generated/native-scroll-insets-v14.json)
completes the private `45ddeee3` censuses: 21,350/22,924 original and
22,153/23,728 expanded exact, zero errors, actual exits 1. One multicolumn
regression against the applied renderer remains; all Chromium inputs and 804
addition results stay unchanged. Corrected margin source `2240ee9c` passes
8,540 workspace tests and all 45 C mutation states, but its wider Rust probe is
555/765 exact, with 210 vertical empty-block failures. Its pixel guards did not
start. A separate owned float-color C API and public Rust/C/C++ consumers are
implemented in a private checkpoint; their own runtime build and native
consumers now pass, as recorded in the
[v15 evidence](docs/renderer/generated/native-scroll-insets-v15.json).
The opaque-scroll review owns 29 of 40 neighboring pixel failures. None of
these private changes is promoted or admitted as a release pass.

The float-color candidate passes 8,542 workspace tests and all public Rust,
eleven C and five C++ consumers. It resolves the eight native process failures
using the original float-color inputs; all 40 geometry states agree, but only
11 images are exact. All 32 earlier native control images stay unchanged.
A public Rust check confirms missing writing-mode inheritance on native
attachment. The shared Engine repair passes all eight clean build stages,
8,542 workspace tests, 765/765 Rust states and 45/45 C states. All 210 prior
vertical failures are repaired without exact losses or changed Chromium inputs.
Its pixel guards now pass all 935 earlier native states, 70 two-child states
and both raster matrices unchanged. All 105 dimension queries agree, but
14 opaque white images still differ. Other inherited properties and pixel
qualification remain separate work; this candidate remains unapplied.

The standalone float-color constructor and public Rust/C/C++ consumers are
now applied to the umbrella branch. The existing Rust API supplies float
colors directly; C uses the new owned `oui_style_value_color_f32_v1` operation
over the same Engine. The [v16 evidence](docs/renderer/generated/native-scroll-insets-v16.json)
records six fresh umbrella build stages, 8,528 workspace tests, five Rust runs,
eleven C and five C++ consumers, and both exact raster matrices. The first
hosted checkpoint failed C formatting; `aeed821c` corrects only C whitespace,
passes the rebuilt C/C++ consumers and all three hosted workflows. Five
skipped hardening jobs remain unverified on this source. The margin and
writing-mode repairs remain separate, unapplied candidates; no new full
census or release pass is claimed.

The [native inheritance and style trials](docs/renderer/generated/native-scroll-insets-v21.json)
remain separate from the accepted renderer. The earlier `f328ed62` complete
censuses fail at 21,312/22,924 original and 22,115/23,728 expanded exact,
losing 22 exact matches across twelve static-position tests. Its `3f1d296f`
follow-up passes 8,535 workspace tests, eight native C geometry states,
50 native Rust images and both complete 40-profile raster matrices; that
checkpoint has no own complete census.

The clean private `5cc75147` follow-up identifies and repairs the regression:
resolved computed fixture values had been sent through the normal inheritance
path, replacing descendant indentation zero with the parent's 20px. Two
independent Chromium queries verify the original reset behavior. The shared
Engine now preserves computed snapshots while native app declarations still
inherit. All 48 selected comparison invariants return to the applied baseline,
restoring all 22 lost exact matches. The other 26 selected failures remain.
Chromium images, source fixtures, fonts and resources stay unchanged.

Five new guards also cover snapshot replacement, inherited animations,
unchanged work and percentage line-height transports. The public typed
line-height setter already passed; the generic Renderer payload is repaired.
The fresh eight-stage build passes 8,540 workspace tests, zero failures and
13 ignored, eleven C/five C++ consumers, ten read-only checks and Rust
formatting. All 113 exports and ABI layouts are preserved. Its own native
relative guards retain all eight exact C geometry states and 50 exact Rust
images against repeated Chromium captures.

The new public Rust static-position example passes all 30 callback and owned
snapshot runs. Its 60 bounds and 60 pixel comparisons all fail the exact gate.
Absolute auto widths omit inherited indentation; subpixel text advances and
glyph painting also require review. Layout, text and paint ownership is
recorded, with minimized inputs, repeated Chromium captures, bounds, connected
regions and channel deltas. Failed capture probes remain separate evidence.
Its own complete matrices now finish at 640/640 focused, 960/960 primitive,
21,334/22,924 original and 22,137/23,728 expanded exact. All comparison
invariants restore the applied baseline, including all 804 additions. Both
complete census commands exit 1; the full pixel gate still fails.
This candidate is unapplied and unqualified; no new release state is admitted.
Open UI runs no JavaScript in any version. Needed interaction uses public
native Rust methods and Rust callbacks over the shared Engine.

The [intrinsic sizing follow-up](docs/renderer/generated/native-scroll-insets-v23.json) reviews the
missing width behavior against 120 repeated Chromium advance observations and
2,000 neighboring measurements at five scales. Chromium retains positive
shaped-width remainders on its 1/64px layout grid and includes first-line
indentation. The private source preserves both, uses shaped intrinsic text,
and adds a consuming Rust app for 200 sizing cases. It also exposes immutable
raster choice through native `Document`, `AppBuilder` and `HeadlessApp`
constructors; the previous document API always used the default.

The initial new Engine guard used the wrong text setup and measured an empty
box on both sources. Its failed build is preserved, with 208 tests passed and
one failure before the workspace stopped; the two raster API guards passed.
The corrected guard attaches actual native text children. It now proves the
old 80px width fails the 80.015625px Chromium result, and the fixed source
passes at all five scales, including inherited indentation and a reset. Ten
read-only checks pass. Its fresh complete workspace build and native/raster
comparisons are running or queued in the frozen observation. The full fixed
workspace, native pixels and censuses are not yet verified. No runtime
promotion or new pixel admission is claimed. Prior snapshots and all
reference bytes remain unchanged.

This repository is not yet
declaring the final v0.2 release. Physical-GPU
and reference-machine qualification, automated AT-SPI operation, direct Skia
GPU qualification, retained per-node layers, compositor-owned animation
curves, release-lab C/C++ application qualification, and signed publication
still remain. The C ABI now runs native Linux windows through the Rust `App`
and retained `Document`. See
[current status](docs/progress/current-status.md)
and [release qualification](docs/v02/release.md).

## Rust quick start

Rust 1.85 or newer, C/C++ build tools, and the host C runtime development files
are required. A Chromium checkout is not.

```toml
[dependencies]
openui = { version = "0.2.0", features = ["linux"] }
```

```rust,no_run
use openui::prelude::*;

fn main() -> Result<(), Error> {
    let count = create_signal(0_i32);
    let app = App::builder()
        .title("Open UI")
        .size(LogicalSize::new(800.0, 600.0))
        .backend(BackendPreference::Auto)
        .build()?;

    app.run(move || view! {
        <button
            style:display={Display::Flex}
            style:padding="8px 16px"
            on:click={move |_| count.update(|value| *value += 1)}
        >
            {count.get()}
        </button>
    })
}
```

From this checkout:

```bash
cd bindings/rust
cargo run --locked --package hello                 # deterministic hello.png
cargo run --locked --package hello --features linux # native Linux window
cargo run --locked --package framework-test -- --headless /tmp/openui-framework-test.png
cargo run --locked --package framework-test --features linux -- --window
```

Use `OUI_BACKEND=software` or `OUI_BACKEND=opengl` to force a window backend.
Headless applications use `HeadlessApp::render_at(time)` for repeatable frames.
The [framework test app](bindings/rust/examples/framework-test/README.md) checks
a reactive click and writes the resulting PNG in headless mode.
If the linker reports missing `Scrt1.o` or `crti.o`, the host C runtime
development files are absent. On the Chromium-equipped maintainer machine,
the checked-in `.cargo/config.chromium.toml` supplies a pinned sysroot; add
`--config .cargo/config.chromium.toml` immediately after `cargo` in the
commands above.

## Native SDK

The v0.2 header uses length-delimited UTF-8, versioned configuration structs,
tagged style values, checked ownership, and structured thread-local errors.

```bash
python3 tools/release/build_v02_linux.py \
  --target x86_64-unknown-linux-gnu --format sdk --format deb
```

The release driver emits headers, static/shared libraries, pkg-config and CMake
metadata, C and Rust examples, detached debug symbols, licenses, an SPDX SBOM,
checksums, and SLSA-style provenance. RPM production runs on Fedora through the
release workflow. See [packaging instructions](docs/v02/packaging.md).

## Repository map

| Path | Purpose |
|---|---|
| `bindings/rust/openui` | Safe application framework and reactive runtime |
| `bindings/rust/openui-engine` | Retained document, interaction, animation, resources, accessibility |
| `bindings/rust/openui-compositor` | Immutable scenes and raster scheduling |
| `bindings/rust/openui-platform` | Feature-gated Linux event loop and presentation |
| `bindings/rust/openui-ffi` | Validated static/shared C ABI |
| `bindings/rust/openui-{style,layout,text,paint}` | Exact rendering pipeline |
| `include/` | Generated v0.2 C headers |
| `examples/c_v02/` | C examples matching the Rust examples |
| `tools/accountability/` | Chromium comparison inventory and historical evidence audit |
| `tools/release/` | Contract generation and reproducible packaging |
| `docs/v02/` | Supported architecture and release contract |

Historical GN/Blink and SP2 experiments remain in Git for provenance, but are
excluded from the workspace and v0.2 packages. They are not supported engines.

## Documentation

- [Development](docs/DEVELOPMENT.md)
- [Architecture](docs/architecture/rendering-pipeline-overview.md)
- [CI and release gates](docs/CI.md)
- [Rust/C migration guide](docs/v02/migration-v01-v02.md)
- [Unsupported features](docs/v02/unsupported-features.md)
- [Typed style reference](docs/v02/generated/style-properties.md)
- [C ABI guide](bindings/rust/openui-ffi/README.md)

Open UI is licensed under Apache-2.0. Bundled font and dependency notices are
preserved with the relevant sources and release artifacts.
