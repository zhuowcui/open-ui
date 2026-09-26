# Renderer qualification contract

Open UI accepts native structure, state, typed styles, and immutable resources.
Its rendering target is pinned Chromium 147; HTML parsing, JavaScript execution,
navigation, networking, iframe browsing contexts, storage, and media playback
are not part of the renderer contract.

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
candidate inventory. `javascript-mutation-audit-v2.json` parses those cases
with the Acorn copy in the pinned Chromium checkout and emits an ordered
mutation IR. Pure synchronous mutations remain pending until their native
Engine lowering is exact across all four profiles. Every rejected case carries
an AST-derived behavioral reason; porter syntax is never a final disposition.
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
At clean checkpoint `5acc962a`, the fresh CPU runner produced 640/640 exact
focused comparisons and 884/960 exact primitive comparisons. The 76 primitive
differences are limited to four IDs; their per-profile bounds, region counts,
channel deltas, scale behavior, and unreviewed ownership are recorded in the
[focused and primitive evidence index](generated/focused-primitive-raster-v1.json).
The primitive gate remains open, and all four residuals are unowned.
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
[v2 diagnostic index](generated/four-profile-census-v2.json), with its clean
source identity recorded inside the file, contains 21,111 exact,
1,813 different, and zero errored comparisons across the four required
profiles, with 1,016 residual test IDs still unowned. The previous v1 index
remains unchanged at 21,108 exact and 1,019 residual IDs. Neither is a
qualification result.

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
