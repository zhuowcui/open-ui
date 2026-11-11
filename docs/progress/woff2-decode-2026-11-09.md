# WOFF and WOFF2 decode before the scaler on every platform, 9 November 2026

Open UI decodes WOFF and WOFF2 application fonts to sfnt before the
platform font manager loads them, on Linux as already on Windows. Before
this change, Linux handed the container to FreeType. FreeType reads WOFF2
only when built with Brotli, and the pinned census FreeType
(`tools/accountability/data/fonts/sp16_linux_x86_64/libfreetype.so.6`) is
not, so every WOFF2 face failed to register with `MalformedFont`. The
components track measured this with the Geist and Inter WOFF2 files they
bundle (plan unit C4): all 24 font fixture comparisons were errors.

## Finding

Chromium never gives a web font container to the platform scaler.
`FontCustomPlatformData::Create` passes every custom font buffer to
`WebFontDecoder::Decode`
(`third_party/blink/renderer/platform/fonts/web_font_decoder.cc`), which
runs the OpenType Sanitizer (`ots_context.Process`) into a memory stream
and creates the typeface from the decoded sfnt with
`WebFontTypefaceFactory::CreateTypeface`. There is no platform branch, so
this holds on Linux (FreeType) and Windows (DirectWrite) alike. Read from
the local Chromium checkout at 147.0.7727.24 (`out/chromium-src`); the
census pins 147.0.7727.50, a patch release of the same branch.
Open UI already did the same on Windows with `wuff`, for the
[DirectWrite text work](windows-directwrite-text-2026-10-21.md); Linux was
the exception.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `platform_font_data` decodes WOFF (`wuff::decompress_woff1`) and WOFF2 (`wuff::decompress_woff2`) to sfnt on every platform; other containers pass through unchanged. A decode failure is `MalformedFont`. Registration keeps the caller's original bytes for identity (`sha256`, `byte_length`) and scenes, as before. | `openui-text` | `bindings/rust/openui-text/src/font/collection.rs` (`platform_font_data`) |
| `wuff` 0.2.9 becomes an ordinary dependency instead of a Windows-only one. `Cargo.lock` is unchanged; it already listed `wuff` and its dependencies. | `openui-text` | `bindings/rust/openui-text/Cargo.toml` |

OTS also validates and may drop tables; `wuff` only decodes. Fonts that OTS
would reject or rewrite are not covered by this change.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Decoding is independent of the FreeType loaded at run time | `cargo test -p openui-text --lib font::collection` (new `web_font_containers_reach_the_scaler_as_sfnt_on_every_platform`) | passes on Windows and Linux |
| Font tests on Linux with the pinned census FreeType, before (`main` `b8dcfaf8`) | WSL, CI `SKIA_GN_ARGS`, `LD_LIBRARY_PATH=tools/accountability/data/fonts/sp16_linux_x86_64 cargo test --locked -p openui-text --lib font::collection` | 11 passed, 3 failed; `pinned_wpt_font_containers_report_identical_owned_metadata` fails with "Certification WOFF2 registration failed: MalformedFont" |
| Same, after (`86aaa5d4`) | same | 13 passed, 2 failed; the WOFF2 registration passes |
| Windows | `cargo test -p openui-text --lib font::collection` | 16 passed |
| CSS Grid coverage | `run_renderer_matrix.py --suite coverage --ids-file tools/coverage/css_grid/manifest.json --jobs 16`, compared per test with the baseline | 2,218 to 2,218 exact; 0 of 4,204 measured Open UI images changed |
| `cargo fmt --all --check` | in `bindings/rust` | pass |

The two tests that fail with the pinned FreeType before and after,
`deterministic_collection_owns_generic_aliases_explicitly` and
`custom_palette_values_are_owned_and_invalidate_instances`, use the
bundled test collection rather than a web font container and are
unchanged by this PR. CI runs them against the system FreeType, where they
pass; they are recorded here as an open difference of the pinned library,
owned by `openui-text`.

The paired expanded census in WSL has no change
([delta](evidence/woff2-decode-census-v1.json)). The base is `9d7bccf6`,
as for the [color functions census](color-functions-2026-11-09.md); `main`
has changed no renderer code since. The head is `86aaa5d4`, this change.
Both are built with the CI `SKIA_GN_ARGS` and share one Chromium cache.
Chromium is identical in all 23,728 comparisons, and Open UI's output is
identical in all 23,728: 22,517 exact before and after.

The CSS Grid coverage suite has no change either
([delta](evidence/woff2-decode-coverage-v1.json)). The base is `04d1eeab`,
the recorded coverage baseline, and the head is `b911ec6f`. In all 4,204
measured comparisons the status and the Open UI image are unchanged: 2,218
exact before and after. The two references that the baseline lists as
unmeasured, whose renderer memory is unbounded, are left out of both runs.

The components track checked this end to end on their branch
`components/static-basics` at `0d3e81c3`, with this PR's `openui-text`
change at `d6389428` applied, built in the census environment (pinned
FreeType) and compared with pinned Chromium 147 by
`components/harness/reference_harness.py`. Before the change: 8 exact,
8 different and 176 `MalformedFont` errors out of 192 comparisons. After:
8 exact, 184 different, 0 errors. 188 of the 192 Open UI images are
byte-identical to their earlier preview, which loaded the WOFF2 files
through a system FreeType built with Brotli, so decoding first gives the
scaler the same font. The remaining differences (text, badge edges,
translucent borders, an opacity layer, column-flex item height, grid auto
column width) are not caused by font loading and are tracked separately.
This is the components track's measurement, not a test of this branch's
CI.
