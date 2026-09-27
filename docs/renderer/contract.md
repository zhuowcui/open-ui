# Renderer qualification contract

Open UI accepts native structure, state, typed styles, and immutable resources.
Its rendering target is pinned Chromium 147; HTML parsing, JavaScript execution,
navigation, networking, iframe browsing contexts, storage, and media playback
are not part of the renderer contract.
Application interaction is implemented through the public native Rust API,
including retained `Document` and `Element` methods and Rust callbacks. A
test fixture that reaches a state through native Engine operations establishes
rendering evidence for that state. Each element behavior needed by a consuming
application also requires a public Rust operation over the same engine; fixture
lowering alone does not complete application API coverage.

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
script's behavioral or nonvisual outcome is outside **pixel** admission, but
an application-needed interaction remains a native API obligation. Every
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
At clean checkpoint `e942aebc`, the CPU runner produced 640/640 exact focused
comparisons and 960/960 exact primitive comparisons. The
[v13 raster index](generated/focused-primitive-raster-v13.json) validates all
40 profiles in each suite with zero errors. All 1,600 Open UI decoded hashes,
Chromium oracle identities, and Chromium decoded hashes match the preceding
clean v12 raster run. The
[column start clip investigation](column-start-clip-investigation.md) records
the shared fragmentainer repair and rejected broad diagnostic.
At clean checkpoint `dad5c9e8`, the CPU runner produced 640/640 exact focused
comparisons and 960/960 exact primitive comparisons. The
[v12 raster index](generated/focused-primitive-raster-v12.json) validates all
40 viewport/scale profiles in each suite with zero errors and zero unowned
residuals. Compared with the [v11 index](generated/focused-primitive-raster-v11.json),
the eight 1.25-scale rounded content-border comparisons became exact. Only
those eight Open UI decoded image hashes changed among the 1,600 comparisons;
all Chromium oracle identities and decoded hashes stayed unchanged. The
focused and primitive CPU gates are exact.
The v11 index records the earlier 952/960 primitive state. Compared with the
[v9 index](generated/focused-primitive-raster-v9.json), its preceding
rounded-border change made 32 comparisons exact.
The v9 index records eight fractional-scale shadow comparisons becoming exact
without changes to the other 1,592 Open UI decoded pixel hashes.
The v8 and v7 indices record four gradient profiles becoming exact after the
renderer selected tiles from the laid-out content footprint and clipped
overflowing paint at the physical viewport edge. The gradient now matches at
all 40 profiles. Earlier indices remain historical evidence.
The [rounded border coverage investigation](rounded-border-coverage-investigation.md)
records the clip-order repair and its earlier diagnostics.
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
`expanded-v1.json` preserves the prior 200 AST-lowered admissions. The clean
complete expanded matrix at `1b652919` contains 21,966/23,724 exact
comparisons, 1,758 differences, and zero errors. Of the 200 additions, 197
are exact at all four profiles and three differ at one profile each. Their
statuses and all 800 Open UI and Chromium decoded hashes match the prior
expanded run. The [v2 requalification ledger](generated/expanded-requalification-v2.json)
records this result; `expanded-v3.json` retains only the 197 exact additions.
The later clean complete expanded matrix at `d73077e7` contains 21,976/23,724
exact comparisons, 1,748 differences, and zero errors. Its ten gains are in
the original `css_break` inventory. All 800 Open UI and Chromium decoded
hashes and statuses for the 200 additions match the prior expanded run, and
all Chromium oracle identities across the matrix remain fixed. The
[v3 requalification ledger](generated/expanded-requalification-v3.json)
records this result; `expanded-v4.json` again retains only the 197 exact
additions. The prior [v1 ledger](generated/expanded-requalification-v1.json),
`expanded-v2.json` manifest, and `expanded-v3.json` diagnostic selection
remain unchanged. The v4 selection is diagnostic until a new contract admits
it; neither the 36 originally pending cases nor the three demotions count as
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
without rewriting it. The current
[v12 diagnostic index](generated/four-profile-census-v12.json), with clean
`e942aebc` source identity, contains 21,179 exact, 1,745 different, and zero
errored comparisons across the four required profiles; 968 residual test IDs
remain unowned. Compared with the
[v11 index](generated/four-profile-census-v11.json), 24 Open UI decoded images
changed across ten multicolumn test IDs: ten comparisons became exact, 14
remain different, and none regressed from exact. All 22,924 Chromium oracle
identities and decoded hashes stayed unchanged. The v11 index records the
earlier 21,169 exact and 1,755 different comparisons. The intermediate
[v9 index](generated/four-profile-census-v9.json) records a paint checkpoint
with five exact-to-different regressions; v10 repaired them. Earlier indices
remain historical evidence. None is a qualification result while residuals
remain.
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
