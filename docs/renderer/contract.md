# Renderer qualification contract

Open UI accepts native structure, state, typed styles, and immutable resources.
Its rendering target is pinned Chromium 147; HTML parsing, JavaScript execution,
navigation, networking, iframe browsing contexts, storage, and media playback
are not part of the renderer contract.
Application interaction is implemented through the public native Rust API,
including retained `Document` and `Element` methods and Rust callbacks. A
test fixture that reaches a state through native Engine operations is evidence
about rendering that state, not evidence of JavaScript or browser API support.

Static media presentation may consume a generated first frame. Those pixels
are decoded ahead of rendering by the Chromium-matched codec revision, bound
to the source bytes and decoder by SHA-256 in
`media-first-frames-v1.json`, and transported as dimensioned sRGB RGBA8. This
does not add playback, timing, or an ambient host-codec dependency to the
renderer.

The generated v2 contract records four complete-suite profiles and the
40-profile focused viewport/scale cross-product. The 800×600@1 profile refers
to the existing 5,731-case test inventory by hash. The generator refuses to
rewrite or accept drift in that historical evidence. That baseline preserves
historical pass records from a comparator with a four-level channel tolerance
and a 15-pixel excluded right strip. Its 5,731 passes are archival
accountability data, not expected output bytes or a release gate. Exact
Chromium equality must be established by the separate zero-tolerance
qualification matrix. The [frozen oracle audit](frozen-oracle-audit.md)
records the discrepancy.
The Chromium source/API inventory remains pinned to `147.0.7727.24`; Linux
pixel qualification records and verifies the installed `147.0.7727.50`
Chrome-for-Testing raster oracle used by the frozen comparison harness.

Layout, input, hit testing, scrolling, selection, and accessibility use logical
CSS pixels. Paint commands are recorded in logical coordinates and replayed to
the profile's physical surface after applying device scale. Whole-frame
resampling is not a conforming presentation path.

`author-style-inventory.json` classifies every `ComputedStyle` field observed
by the layout and paint source trees. Engine bookkeeping has an explicit
`internal` classification; all remaining fields are author-facing typed
properties. An unclassified field is a qualification failure.

`javascript-disposition.json` preserves the immutable 393-case final-state
candidate inventory from historical WPT files that contain scripts. Offline
test tooling uses the Acorn copy in the pinned Chromium checkout to parse
those scripts into an ordered mutation IR in `javascript-mutation-audit-v2.json`.
Open UI does not execute that JavaScript. Native Rust test fixtures reproduce
only deterministic final visual states; a fixture is admitted only when its
Engine operations are exact against Chromium across all four profiles. A
script's behavioral or nonvisual outcome is outside pixel admission. Every
rejected case carries an AST-derived reason; porter syntax is never a final
disposition. Product interactions must be available through the public
`openui` Rust API; test-only Engine lowering does not establish that coverage.
At clean checkpoint `5acc962a`, the 36 AST-lowered pending cases produced
21/144 exact comparisons, 123 differences, and zero errors. None was exact
at all four required profiles, so none was admitted. The
[pending-candidate evidence index](generated/pending-mutation-candidates-v1.json)
records every profile result and links each case to its ordered mutation IR.

Application and system font ownership, registration limits, and C handle
lifetime rules are documented in [font collections](font-collections.md).
Font selection and shaping precedence is documented in
[font selection and shaping](font-selection-and-shaping.md), and the shared
layout/paint flow is documented in [text layout and paint](text-layout-and-paint.md).

`tools/qualification/run_renderer_matrix.py` uses separate, contract-pinned
manifests for the complete 5,731-case census and the focused raster corpus.
The focused and primitive corpora are exact-equality gates across all 40
viewport/scale profiles. Residual investigations use `--suite residual` for
the five-scale 800×600 sweep plus the eight contract viewports at 1×, and use
`--suite residual-cross` when that sweep does not isolate the interaction.
The four-profile complete run is also qualifying only when it is
complete, decoded-RGBA exact, error-free, and produced from a clean source tree.
At clean checkpoint `0d1c7e82`, the CPU runner produced 640/640 exact focused
comparisons and 908/960 exact primitive comparisons. The 52 primitive
differences are limited to three IDs; their per-profile bounds, region counts,
channel deltas, scale behavior, and unreviewed ownership are recorded in the
[focused and primitive evidence index](generated/focused-primitive-raster-v6.json).
The primitive gate remains open, and all three residuals are unowned in the
qualification ledger. The subsequent border investigation identifies one
missing coverage contribution, but it does not close the full root cause. Compared
with the [v5 index](generated/focused-primitive-raster-v5.json), all 1,600 Open UI
decoded pixel hashes and Chromium oracle identities and decoded hashes stayed
unchanged. The v2 index recorded eight fractional-scale shadow comparisons
becoming exact; v3 records the same counts before the curved-clip guard. The v1
index remains historical evidence at 884/960 exact.
The [rounded border coverage investigation](rounded-border-coverage-investigation.md)
isolates one of the remaining primitive failures. Its experimental output is
diagnostic and does not change the gate.
An explicit Ganesh raster run on Mesa llvmpipe completed the same clean
40-profile suites. Its [backend comparison](generated/ganesh-raster-comparison-v1.json)
records 408/640 focused and 624/960 primitive exact, compared with CPU Skia's
640/640 and 900/960. All Chromium oracle hashes matched across backends;
Ganesh changed 508 CPU-exact comparisons to different and repaired none of the
60 CPU primitive residuals. The renderer code was unchanged between the CPU
and Ganesh source checkpoints. This diagnostic does not qualify Ganesh or
change the portable CPU selection.
The runner defaults to `bindings/rust/target/release/pixel_compare`. Rebuild
that executable from the clean checkpoint with the pinned toolchain and pass
its path explicitly using `--pixel-compare`; a recent debug build does not
refresh the release executable. Matrix reports record the executable SHA-256.
The optional historical byte replay uses its own explicitly selected
executable and never substitutes for Chromium qualification.
`expanded-v1.json` preserves the prior 200 AST-lowered admissions. A fresh
four-profile run found 197 still exact and three different at one profile
each. The [requalification ledger](generated/expanded-requalification-v1.json)
records those pixels; `expanded-v2.json` retains only the 197 exact additions
without changing the original 5,731 IDs or rewriting the prior manifest.
This v2 selection is diagnostic until it is admitted by a future contract;
neither the 36 originally pending cases nor the three demotions count as
current exact coverage.
Non-exact runs emit a v2 residual ledger whose pixel bounds, connected regions,
channel deltas, scale behavior, reviewed root cause, owner, and minimized
reproducer are all explicit. Test names and fixture keywords never select an
owner. Unknown ownership fails the qualifying ledger; diagnostic reports retain
their pixel evidence and record the ownership error. A residual ledger never
becomes a tolerance, allowlist, or alternate baseline.

`tools/qualification/summarize_renderer_census.py` verifies the complete,
disjoint shard set against the pinned manifest, profile geometry, report hashes,
and common source/backend identities before emitting the versioned
`generated/four-profile-census-v1.json` evidence index. Its default mode fails
on any unowned residual. `--allow-unowned-diagnostics` explicitly emits a
nonqualifying snapshot for investigation; `--check` verifies that snapshot
without rewriting it. The fresh
[v7 diagnostic index](generated/four-profile-census-v7.json), with its clean
source identity recorded inside the file, contains 21,160 exact,
1,764 different, and zero errored comparisons across the four required
profiles, with 971 residual test IDs still unowned. Compared with the
[v6 index](generated/four-profile-census-v6.json), all Open UI and Chromium
decoded pixel hashes stayed unchanged. Compared with the v5 index, three round-adjusted raster
background comparisons at 1920×1080@1.5 became exact, with no regression and
no changed Chromium oracle identity or decoded pixel hash across all 22,924
comparisons. The v3 index recorded two shadow comparisons becoming exact and a
third improving from 256 to five differing pixels. The v4 index recorded an
earlier checkpoint where partitioned shadow coverage added ten one-channel
differences under a curved ancestor clip; v5 restored that case byte for byte
through a general clip-shape guard. Earlier indices remain historical evidence.
None is a qualification result.
The earlier v3 repair was inline text reaching a later block's border:
that later decoration must paint in the block phase before the earlier text
ink. The change applies by fragment geometry, while preserving atomic flex,
grid, mask, and paint-containment groups outside the established text path.

Results can be resumed through a content-addressed cache; decoded evidence PNGs
are retained by content hash. Chromium captures live in a separate immutable,
write-once oracle cache whose identity contains only browser-side inputs: the
Chromium binary and build, capture harness, fixture, fonts, resources, feature
flags, viewport, and device scale. OpenUI source, binary, and backend identities
are deliberately excluded, so a renderer revision cannot silently recapture a
different expected glyph strike. A conflicting producer for an existing oracle
identity fails closed.

Each OpenUI result-cache identity includes the Git commit and source-tree hash,
qualification-harness hash, renderer binary, the immutable Chromium oracle
identity, logical and physical viewport, device scale, immutable raster and
GPU/driver identity, generated fixture, fonts, and resources. Shards are
deterministic; a final unsharded run can reuse completed shards while rejecting
stale or cross-profile entries. Authoritative runs reject dirty worktrees.
Explicitly allowed dirty runs carry source and harness hashes and remain
nonqualifying. A partial `--profile`, `--test-id`, or sharded run is marked
incomplete and cannot be represented as full contract evidence. Use `--plan`
to inspect scope without producing or mutating qualification evidence.

CPU Skia remains the portable and qualification backend. `ganesh-gl` is an
explicit feature and raster configuration backed by offscreen EGL/Mesa; it is
never selected from the environment. Reports and cache keys record its GL
renderer, version, EGL/driver identity, color type, sample count, and surface
properties. It may replace CPU qualification only after repeated primitive,
focused, and full-census runs are byte-identical and improve the global census.

The font matrix uses licensed TTF, OTF, WOFF, WOFF2, TTC, and deterministically
generated OTC fixtures from the pinned WPT checkout. The generator verifies
source and output hashes, and Rust/C tests require identical container, face
index, byte length, and SHA-256 metadata. These fixtures remain document-owned;
they do not expand qualifying dependence on ambient system fonts.
