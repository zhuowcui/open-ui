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
| Fresh four-profile Chromium census | 21,185/22,924 exact, 1,739 different, 0 errors |
| Focused / primitive 40-profile matrices | 640/640 / 960/960 exact |
| Expanded native final-state additions | 197/200 exact at all four profiles in the latest clean run; three demoted |
| Full inventory | 7,673 |
| Explicitly unported | 1,942 |
| Accountability audit | 7/7 |
| Application conformance scenarios | 38 across 10 domains |
| Frozen / current C exports | 84 / 102 |
| C examples / C++ consumers | 5 / 1 |
| Workspace tests | pass |
| Python closure and qualification tests | 231 pass |
| Owned objects after 10,000 mutation soak | no growth/leak |
| Unchanged-frame lifecycle | zero layout, paint, and raster work |

The [latest complete clean census](../renderer/generated/four-profile-census-v14.json)
at `2dce665c` made four more comparisons exact than v13, with no previously
exact comparison regressing. Opaque-background culling now follows an
anonymous line box while retaining the background inside multicolumn
fragmentainers. Both remaining fractional profiles of
`negative-margins-001` are exact; `flex-grow-006` and
`background-color-border-box` each gained one exact profile. A fifth changed
image, `background-clip-color` at 1.25×, remains different but fell from 225
to 150 mismatched pixels. All 22,924 Chromium decoded hashes stayed fixed.
The clean [v15 raster index](../renderer/generated/focused-primitive-raster-v15.json)
confirms both 40-profile matrices stayed exact with all 1,600 Open UI and
Chromium decoded hashes unchanged. The census still fails exactness, with 965
unowned residual test IDs. The [investigation](../renderer/flex-negative-margin-investigation.md)
records the cause, rejected diagnostic, and clean repair.

The clean [expanded v5 requalification](../renderer/generated/expanded-requalification-v5.json)
at `2dce665c` measured 21,982/23,724 exact comparisons, 1,742 different,
and zero errors. All 200 native final-state additions retained their prior
four-profile statuses and Open UI/Chromium decoded hashes: 197 remain exact
at all four profiles and three remain demoted. The four new exact comparisons
belong to the original inventory. The
[v6 diagnostic selection](../../tools/qualification/manifests/expanded-v6.json)
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
- The four-profile Chromium census fails exactness; 965 residual test IDs
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
