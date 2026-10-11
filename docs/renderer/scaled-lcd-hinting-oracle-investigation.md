# Scaled LCD hinting and Chromium oracle investigation

Chromium remains Open UI's sole pixel target. A shared font-hinting experiment
was rejected because it regressed exact comparisons. Fresh Chromium captures
also exposed two different decoded images under the same recorded oracle
identity. This investigation preserves the evidence and qualifies no renderer
change. The capture discrepancy's root cause remains unreviewed.

## Rejected renderer change

The pinned Chromium source's `ui/gfx/font_render_params_linux.cc` disables
hinting when antialiasing and subpixel positioning are enabled, with subpixel
positioning selected for a device scale above one. Blink's
`FontPlatformData::QuerySystemRenderStyle` passes the font cache's device scale
to the Linux font service. That source path suggested testing a shared policy
of no hinting for scaled LCD fonts using requested slight hinting, leaving
Ahem unchanged. Source inspection alone does not establish which strike was
used by a particular captured frame.

The diagnostic covered all 226 registered real-font cases at the four required
profiles, or 904 comparisons:

| Result against the existing cached Chromium captures | Baseline | Candidate |
|---|---:|---:|
| Exact | 747 | 732 |
| Different | 157 | 172 |
| Render errors | 0 | 0 |

The candidate changed 107 Open UI images. It made 28 comparisons exact and
regressed 43 formerly exact comparisons. All 904 recorded Chromium identities
and decoded images remained unchanged during that cached diagnostic. The
candidate used a dirty, partial source report and cannot qualify a release.
The font policy and canonical runner were restored; no renderer change was
kept. The candidate runner hash and report provenance are recorded in the
[investigation index](generated/scaled-lcd-hinting-investigation-v1.json).

## Conflicting recorded oracle identity

Three native Engine cases were captured through the ordinary, unchanged
Chromium harness at all four profiles, using empty, separate renderer and
oracle caches. Eleven of these 12 fresh decoded Chromium images matched the
older cached images. One differed:

| Input | Recorded value |
|---|---|
| Case | `wpt/css2_floats/float-nowrap-3` |
| Logical viewport and scale | 1920×1080@1.5 |
| Physical image | 2880×1620 |
| Chromium build | 147.0.7727.50 |
| Oracle identity SHA-256 | `6dd266570d288cd9de962452816846bcfed463dea115163d557d3172386253de` |
| Fixture SHA-256 | `0acbfc80859f785c8d3ccd4b99b833339d693124d8e5d7c2ac67823a8e1857df` |

Five further fresh Chromium processes, each with separate empty caches,
produced the same image as the first fresh capture. All six fresh images
agree. The evidence therefore shows an older cached variant disagreeing with
stable fresh observations; it does not demonstrate variation between the
fresh repeats or establish the cause of the older variant.

Both oracle-entry `identity` objects are identical, including the fixture,
Chromium binary, capture harness, font bytes, resources, font profile, viewport,
and scale. Preserved copies are available as
[cached-entry.json](evidence/chromium-font-oracle-identity-v1/cached-entry.json)
and [fresh-entry.json](evidence/chromium-font-oracle-identity-v1/fresh-entry.json).
The image variants are:

| Capture | Decoded RGBA SHA-256 |
|---|---|
| [Older cached image](evidence/chromium-font-oracle-identity-v1/cached.png) | `8da6c02ae59521fd9aacc772949d5e2f68afb707a6bd3a5641c1d5e4bf03687c` |
| [Fresh image](evidence/chromium-font-oracle-identity-v1/fresh.png) | `d666546cb3622a34639553046618dc66b33e11c79c20e74fdddb3b54ce292c6f` |

The variants differ at 2,620 pixels in 42 connected regions, bounded by
physical `x=31, y=35, width=428, height=51`. Alpha is identical; the largest
absolute RGB channel delta is 240. The investigation index records every
region and each channel's deltas. The baseline Open UI image matches the fresh
Chromium image exactly. The rejected candidate matches the older cached image
exactly. Neither observation reconciles the recorded oracle identity.

The [735-byte static HTML reproducer](evidence/chromium-font-oracle-identity-v1/repro.html)
is copied byte-for-byte from the captured input and contains no JavaScript.
The corresponding native case is `css2_floats_float_nowrap_3` in
[`wpt_css2_floats.rs`](../../bindings/rust/pixel-compare/src/wpt/wpt_css2_floats.rs).
Open UI builds that document through native Engine operations. Browser scripts
used by capture tooling run only in the separate Chromium oracle; Open UI
continues to expose application interaction through native Rust APIs.

## Capture setup diagnostics

Two separate capture protocols were tested on the three cases at 1.5×. Each
protocol bound its wrapper to a distinct capture-harness identity:

| Diagnostic protocol | Result for the rejected candidate |
|---|---|
| Force layout and two animation frames after device-metrics emulation, before navigation | 0/3 exact; images matched the ordinary fresh captures |
| Set the device scale at browser process startup | 0/3 exact; geometry and pixels also changed |

Neither protocol repaired the candidate. Neither is promoted into the pinned
capture harness. The `--no-sandbox` switch does not bypass Chromium's Linux
font service in this harness: the pinned `RendererBlinkPlatformImpl` uses the
single-process switch for that decision. A startup or font-initialization
cause remains a hypothesis requiring runtime evidence.

## Audit and qualification state

[`audit_chromium_oracles.py`](../../tools/qualification/audit_chromium_oracles.py)
checks recorded report digests and groups observations by oracle identity and
decoded RGBA hash. Encoding differences with identical decoded pixels are
accepted as consistent observations. Different decoded pixels under one
identity produce a failing exit status. It does not recapture images or
qualify unobserved cases or a renderer.

The [audit index](generated/chromium-font-oracle-audit-v1.json) includes the
existing 22,924-comparison clean census, the 12 fresh captures, and five fresh
repeats: 22,924 observed identities, 17 fresh observations across 12 identities,
and one contradicted identity. Reproduce it with:

```bash
python3 tools/qualification/audit_chromium_oracles.py \
  out/renderer-evidence/rounded-clip-x-full-15f9f12d/full-summary.json \
  out/renderer-evidence/scaled-lcd-live-oracle-diagnostic-9640617c/expanded-summary.json \
  out/renderer-evidence/real-font-hinting-investigation/repeat-{1,2,3,4,5}/expanded-summary.json \
  --output out/renderer-evidence/chromium-font-oracle-audit-reproduced.json
```

The expected exit status is one, reporting the observed contradiction. The
existing clean census counts remain unchanged as comparisons against their
preserved cached inputs. Final qualification requires a reviewed cause and
reconciled capture identities. The original archive, oracle entries, images,
fixture, and capture harness remain unchanged; neither variant is substituted
for the other. No tolerance, fixture-specific renderer rule, or release pass
is introduced.
