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
| Fresh four-profile Chromium census | 21,208/22,924 exact, 1,716 different, 0 errors |
| Focused / primitive 40-profile matrices | 640/640 / 960/960 exact |
| Expanded native final-state additions | 197/200 exact at all four profiles in the latest clean run; three demoted |
| Full inventory | 7,673 |
| Explicitly unported | 1,942 |
| Accountability audit | 7/7 |
| Application conformance scenarios | 39 across 10 domains |
| Frozen / current C exports | 84 / 102 |
| C examples / C++ consumers | 5 / 1 |
| Workspace tests | pass |
| Python closure and qualification tests | 231 pass |
| Owned objects after 10,000 mutation soak | no growth/leak |
| Unchanged-frame lifecycle | zero layout, paint, and raster work |

The [latest complete clean census](../renderer/generated/four-profile-census-v16.json)
at `4b89fd05` gained one exact comparison from v15: the 1.5× fieldset
reference. Four other already-different comparisons became smaller; no
previously exact comparison regressed. Five Open UI decoded images changed and
all 22,924 Chromium oracle identities and decoded hashes stayed fixed. The
clean [v17 raster index](../renderer/generated/focused-primitive-raster-v17.json)
confirms both 40-profile matrices stayed exact with all 1,600 Open UI and
Chromium decoded hashes unchanged. The census still fails exactness, with 949
unowned residual test IDs. The [fragment clip investigation](../renderer/fragment-decoration-clip-investigation.md)
records the cause, rejected broad change, and guarded repair. The earlier
[background-clip investigation](../renderer/background-clip-hard-clip-investigation.md)
remains the record for the v15 gains.
The [border-image seam investigation](../renderer/border-image-seam-investigation.md)
records a separate six-case fractional-scale residual and a rejected
offscreen-layer diagnostic; it changes no qualifying count.
The [repeat-space shader investigation](../renderer/background-repeat-space-shader-investigation.md)
records two rejected 92-comparison sampling diagnostics and a clean restored
runner; it also changes no qualifying count.

The clean [expanded v7 requalification](../renderer/generated/expanded-requalification-v7.json)
at `4b89fd05` measured 22,005/23,724 exact comparisons, 1,719 different,
and zero errors. All 200 native final-state additions retained their prior
four-profile statuses and Open UI/Chromium decoded hashes: 197 remain exact
at all four profiles and three remain demoted. The new exact comparison
belongs to the original inventory. The
[v8 diagnostic selection](../../tools/qualification/manifests/expanded-v8.json)
retains only the 197 exact additions; it does not alter the original manifest.

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
- The four-profile Chromium census fails exactness; 949 residual test IDs
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
