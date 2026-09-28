# Open UI current status

Open UI is in v0.2 release-candidate closure for the pure-Rust Linux/headless
product. Waves W0 through W10 are locally committed. W11 source packaging and
documentation are implemented, but final external and hardware qualifications
remain open.

## Verified repository state

| Evidence | Result |
|---|---:|
| Historical frozen SP20 pass records | 5,731, using a tolerant comparator |
| Optional historical byte replay | 5,549 unchanged, 182 changed, 0 errors; not a gate |
| Fresh four-profile Chromium census | 21,223/22,924 exact, 1,701 different, 0 errors |
| Focused / primitive 40-profile matrices | 640/640 / 960/960 exact |
| Expanded native final-state additions | 197/200 exact at all four profiles in the latest clean run; three demoted |
| Full inventory | 7,673 |
| Explicitly unported | 1,942 |
| Accountability audit | 7/7 |
| Application conformance scenarios | 40 across 10 domains |
| Frozen / current C exports | 84 / 102 |
| C examples / C++ consumers | 5 / 1 |
| Workspace tests | pass |
| Python closure and qualification tests | 231 pass |
| Owned objects after 10,000 mutation soak | no growth/leak |
| Unchanged-frame lifecycle | zero layout, paint, and raster work |

The [latest complete clean census](../renderer/generated/four-profile-census-v25.json)
at `5c7aaa9c` changed exactly eight Open UI images in four one-axis repeated
linear-gradient test/reference pairs. Their wrong-pixel counts fell sharply
at fractional scales, but none became exact because the separate PNG edge
still differs. No exact comparison regressed, and all 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[gradient picture-shader investigation](../renderer/one-axis-gradient-picture-shader.md)
records the source rule and remaining raster edge. The clean
[v26 raster index](../renderer/generated/focused-primitive-raster-v26.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded images unchanged. The clean
[v4 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v4.json)
remains 797/800 exact, with all 800 Open UI and Chromium decoded images
unchanged. The release census still fails exactness with 1,701 differences
across 939 unowned residual test IDs.

The [prior complete clean census](../renderer/generated/four-profile-census-v24.json)
at `2f560e46` gained four exact comparisons from v23 by painting mixed border
styles in Chromium's alpha, style, and side order. Exactly 16 Open UI images
changed across four border fixtures; none regressed, and all 22,924 Chromium
oracle identities and decoded hashes stayed fixed. The
[border paint-order investigation](../renderer/mixed-border-paint-order.md)
records the compact case, source rule, rejected clip-only diagnostic, and
four-profile pixel counts. The clean
[v25 raster index](../renderer/generated/focused-primitive-raster-v25.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v24. The clean
[v3 selected-additions recheck](../renderer/generated/expanded-additions-recheck-v3.json)
remains 797/800 exact with three differences and zero errors; all 800 decoded
images and statuses are unchanged. The complete census still fails exactness
with 1,701 differences across 939 unowned residual test IDs.

An [earlier complete clean census](../renderer/generated/four-profile-census-v23.json)
at `d0592ccd` gained one exact comparison from v22:
`out-of-flow-in-multicolumn-047` at 1280×720@1.25. Its parent decoration
now ends at its used block size, and the outer visual continuation no longer
replays inner columns whose in-flow source was already consumed. Only that
Open UI decoded image changed; no previously exact comparison regressed.
All 22,924 Chromium oracle identities and decoded hashes stayed fixed. The
[consumed nested columns investigation](../renderer/multicol-consumed-nested-columns.md)
includes reduced reproductions of both masked errors. The clean
[v24 raster index](../renderer/generated/focused-primitive-raster-v24.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium hashes unchanged from v23. The census still fails exactness,
with 1,705 differences across 939 unowned residual test IDs.
The clean [v2 selected-additions
recheck](../renderer/generated/expanded-additions-recheck-v2.json) at
`db763999` measured 797/800 exact, three different, and zero errors. All
800 Open UI and Chromium hashes matched the prior full expanded run; this
selection does not replace complete expanded-manifest qualification.

The prior [v22 clean census](../renderer/generated/four-profile-census-v22.json)
at `6e21bd9b` gained one exact comparison from v21:
`out-of-flow-in-multicolumn-046` at 1280×720@1.25. Its zero-height
positioned parent no longer paints a background through the visual
continuation needed by its absolute child. Only that Open UI decoded image
changed; no previously exact comparison regressed. All 22,924 Chromium oracle
identities and decoded hashes stayed fixed. The
[zero-height continuation investigation](../renderer/multicol-zero-height-positioned-decoration.md)
includes a reduced reproducer. The clean
[v23 raster index](../renderer/generated/focused-primitive-raster-v23.json)
remains 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium hashes unchanged from v22. The census still fails exactness,
with 1,706 differences across 940 unowned residual test IDs.

The prior [v21 clean census](../renderer/generated/four-profile-census-v21.json)
at `99fd4432` gained three exact comparisons from v20:
`flexbox_multi-line-row-flex-fragmentation-029` at 1280×720@1.25 and
`-030` at 1280×720@1.25 and 1920×1080@1.5. Only those three Open UI decoded
images changed; no previously exact comparison regressed. All 22,924 Chromium
oracle identities and decoded hashes stayed fixed. The
[final-decoration investigation](../renderer/flex-final-continuation-decoration.md)
records the reduced reproducer and the rejected broad rule. The clean
[v22 raster index](../renderer/generated/focused-primitive-raster-v22.json)
confirms 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v21. The census still fails
exactness, with 941 unowned residual test IDs.

The prior [v20 clean census](../renderer/generated/four-profile-census-v20.json)
at `89a0a1f3` gained one exact comparison from v19:
`flexbox_multi-line-row-flex-fragmentation-027` at 1280×720@1.25.
No previously exact comparison regressed. Only that Open UI decoded image
changed; all 22,924 Chromium oracle identities and decoded hashes stayed
fixed. The [first-line break investigation](../renderer/nested-row-flex-first-line-break.md)
records the masked layout error, reduced reproducer, and neighboring guard.
The clean [v21 raster index](../renderer/generated/focused-primitive-raster-v21.json)
confirms 640/640 focused and 960/960 primitive exact, with all 1,600 Open UI
and Chromium decoded hashes unchanged from v20. The census still fails
exactness, with 943 unowned residual test IDs.

The prior [v19 clean census](../renderer/generated/four-profile-census-v19.json)
at `17952772` gained three exact comparisons from v18:
`out-of-flow-in-multicolumn-042`, `-043`, and `-045` at 1280×720@1.25.
No previously exact comparison regressed. Only those three Open UI decoded
images changed; all 22,924 Chromium oracle identities and decoded hashes
stayed fixed. The [relative continuation clip investigation](../renderer/multicol-relative-clip-translation.md)
records the cause, reduced reproducer, and authored-overflow guard. The clean
[v20 raster index](../renderer/generated/focused-primitive-raster-v20.json)
confirms both 40-profile matrices stayed exact with all 1,600 Open UI and
Chromium decoded hashes unchanged from the preceding clean v19 raster run. The
census had 944 unowned residual test IDs. The prior
[multicolumn investigation](../renderer/multicol-nested-positioned-continuation.md)
records the v18 repair. The prior
[fragment clip investigation](../renderer/fragment-decoration-clip-investigation.md)
records the v16 repair.
The [border-image seam investigation](../renderer/border-image-seam-investigation.md)
records a separate six-case fractional-scale residual and a rejected
offscreen-layer diagnostic; it changes no qualifying count.
The [repeat-space shader investigation](../renderer/background-repeat-space-shader-investigation.md)
records two rejected 92-comparison sampling diagnostics and a clean restored
runner; it also changes no qualifying count.

The clean [expanded v12 requalification](../renderer/generated/expanded-requalification-v12.json)
at `6e21bd9b` measured 22,015/23,724 exact comparisons, 1,709 different,
and zero errors. Only the same original `-046` image changed from v11, and it
became exact. All 23,724 Chromium oracle identities and decoded hashes stayed
fixed. All 200 native additions kept their prior four-profile statuses and
all 800 decoded Open UI images: 197 remain exact at all four profiles and
three remain demoted. The
[v13 diagnostic selection](../../tools/qualification/manifests/expanded-v13.json)
retains those 197 additions without changing the original manifest.

The prior clean [expanded v11 requalification](../renderer/generated/expanded-requalification-v11.json)
at `99fd4432` measured 22,014/23,724 exact comparisons, 1,710 different,
and zero errors. Only the same three original flex comparisons changed from
v10. All 23,724 Chromium oracle identities and decoded hashes stayed fixed.
All 200 native additions kept their prior four-profile statuses and all 800
decoded Open UI images: 197 remain exact at all four profiles and three remain
demoted. The [v12 diagnostic selection](../../tools/qualification/manifests/expanded-v12.json)
retains only those 197 additions; it does not alter the original manifest.

The prior [expanded v10 requalification](../renderer/generated/expanded-requalification-v10.json)
at `89a0a1f3` measured 22,011/23,724 exact comparisons, 1,713 different,
and zero errors. Only the same original `-027` comparison changed from the
previous full expanded run. All 23,724 Chromium oracle identities and decoded
hashes stayed fixed. All 200 native additions kept their prior four-profile
statuses and all 800 decoded Open UI images: 197 remain exact at all four
profiles and three remain demoted. The
[v11 diagnostic selection](../../tools/qualification/manifests/expanded-v11.json)
retains only those 197 additions; it does not alter the original manifest.

The prior clean [expanded v9 requalification](../renderer/generated/expanded-requalification-v9.json)
at `de8304fe` measured 22,010/23,724 exact comparisons, 1,714 different,
and zero errors. Against the prior v8 expanded run, four original
`out-of-flow-in-multicolumn-*` comparisons became exact: `-060` from the v18
census repair and `-042`, `-043`, `-045` from the v19 repair. No other Open UI
decoded image or status changed. All 23,724 Chromium oracle identities and
decoded hashes stayed fixed. All 200 native final-state additions retained
their prior four-profile statuses and all 800 decoded Open UI images: 197
remain exact at all four profiles and three remain demoted. The
[v10 diagnostic selection](../../tools/qualification/manifests/expanded-v10.json)
retains only the 197 exact additions; it does not alter the original manifest.
The prior [v8 ledger](../renderer/generated/expanded-requalification-v8.json)
and [v9 selection](../../tools/qualification/manifests/expanded-v9.json)
remain historical evidence.

At the earlier clean checkpoint `93b00601`, a
[four-profile additions recheck](../renderer/generated/expanded-additions-recheck-v1.json)
rendered all 200 native additions: 797/800 profile comparisons exact, three
different, and zero errors. The same three cases remain demoted. All 800
Open UI decoded hashes, Chromium oracle identities, and Chromium decoded
hashes match the prior clean v8 expanded run. This subset report is
diagnostic; it does not replace a complete expanded-manifest qualification.

The historical baseline records Chromium identity, viewport, device scale,
fonts, resources, and result hashes. A fresh pinned Chromium capture differs
from 203 archived Open UI images; the old screenshots are not expected output.
See the [audit](../renderer/frozen-oracle-audit.md). Chromium is the sole pixel
target but a test oracle only; supported builds and packages do not link or
load it.

## Implemented v0.2 surface

- Canonical generated typed style properties for Rust macros, engine metadata,
  and C tagged values.
- Thread-affine retained documents with generation-checked node handles,
  transactions, dirty generations, resources, hit testing, and immutable scene
  snapshots.
- Direct safe Rust framework with signals, effects, scopes, `Show`, keyed
  `For`, components, `view!`, `AppBuilder`, and `HeadlessApp`.
- Panic-contained C ABI with ownership/thread validation, structured errors,
  compound builders, events, controls, animation, owned accessibility-tree
  snapshots, and rendering.
- Core controls, routed pointer/keyboard/text/composition events, focus and
  modal containment, selection, Unicode editing, clipboard, undo/redo,
  scrolling, and pointer capture.
- Public native element activation and details disclosure: `Element::click`,
  `set_open`, and `is_open` use the same retained event and control path;
  closed details content leaves layout and the accessibility tree.
- Public native class-token changes and attached-document class lookup use
  the same retained tree from Rust callbacks.
- Public native kind lookup and `Element::kind` cover the deterministic
  test cases that find elements by tag, with attached-document order and
  generation-checked handles. Authored names such as `div` and `main` can
  share a native kind.
- Retained accessibility trees/actions and Linux AccessKit integration.
- Winit X11/Wayland runtime, backend-correct clipboard, IME/data events,
  softbuffer presentation, and multiple isolated windows.
- OpenGL presentation of the exact CPU Skia frame with automatic software
  fallback, diagnostics, resize recovery, and newest-scene scheduling.
- Typed transitions/keyframes, easing, manual clocks, lifecycle events,
  document/scroll/view timelines, smooth scrolling, snap, and reduced motion.
- Conformance, fuzz, sanitizer, Miri, soak, platform, performance, and
  reproducible package gates.

## Release blockers

- Direct Skia Ganesh raster builds behind explicit backend selection. On the
  clean Mesa llvmpipe [comparison](../renderer/generated/ganesh-raster-comparison-v1.json),
  it reached 408/640 focused and 624/960 primitive exact, below CPU Skia's
  640/640 and 960/960. It remains unpromoted; OpenGL presentation still uploads
  a CPU-rasterized frame.
- The [current hardening run](https://github.com/zhuowcui/open-ui/actions/runs/36365378115)
  still fails AddressSanitizer, LeakSanitizer, and fuzz on process-exit
  Fontconfig allocations. Its MSRV, Miri, Linux platform, and C UBSan jobs
  passed.
- The four-profile Chromium census fails exactness; 943 residual test IDs
  have no reviewed owner. Both 40-profile CPU raster matrices are exact.
- The C ABI covers the retained engine, headless renderer, and an owned full
  accessibility-tree snapshot, but does not yet export the owned Linux event loop.
- Retained per-node layers and compositor-owned animation curves are incomplete,
  so the strict blocked-UI 100-animation gate is not yet qualified.
- Full preserve-3d and backface layer semantics remain incomplete.
- Physical-GPU/context-loss and automated AT-SPI release-lab runs are pending.
- x86-64 and AArch64 clean-container packages must be built twice, compared,
  attested, signed, installed, and exercised.
- Crates.io publication requires final release approval and credentials.

See [release qualification](../v02/release.md),
[hardening](../v02/hardening.md), and the
[unsupported list](../v02/unsupported-features.md). Do not describe the WSL2
performance artifact as release qualification.

## Canonical local gates

```bash
cd bindings/rust
cargo test --workspace --locked
cd ../..

python3 tools/release/build_v02_linux.py --verify-source
python3 tools/accountability/audit.py
python3 tools/accountability/restore_frozen_openui_archive.py --check
python3 tools/accountability/audit_frozen_oracle.py --output /tmp/openui-historical-audit.json
python3 tools/ffi/verify_abi.py
```

Pixel qualification uses the pinned Chromium oracle through the complete
four-profile matrix described in the [renderer contract](../renderer/contract.md).
The archive check above only protects historical evidence.

For the workstation-only exact toolchain, pass
`--config .cargo/config.chromium.toml` to Cargo. Ordinary development and
release builds use the portable default configuration.
