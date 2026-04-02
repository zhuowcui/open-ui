# Open UI — Project Plan Overview

> Extract Chromium's actual rendering pipeline into a standalone UI framework with pixel-perfect web rendering, programmable from any language via a stable C ABI.

## Vision

Chromium's rendering layer is pure native code magic — style resolution, layout (LayoutNG), paint recording, compositing (cc/), and Skia rasterization running at 60fps+ with best-in-class correctness across virtually every OS. It renders every HTML element and CSS property with pixel-perfect accuracy.

**Open UI** extracts that rendering pipeline — the actual Chromium code, not a reimplementation — into a standalone framework. Applications build UI trees programmatically through a C API. Every HTML element (div, span, button, input, table, SVG, etc.) renders identically to Chromium. Everything is compiled ahead of time: no HTML parsing, no CSS parsing, no JavaScript, no network stack.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Application Code                       │
│              (Rust, C, Python, Go, etc.)                 │
├─────────────────────────────────────────────────────────┤
│                    openui.h                               │
│         Programmatic Element Tree + Style API            │
├─────────────────────────────────────────────────────────┤
│              Open UI Adapter Layer                        │
│     (Minimal DOM, Programmatic Style, Event Bridge)      │
├──────────┬──────────┬──────────┬────────────────────────┤
│  Style   │  Layout  │  Paint   │   Compositor (cc/)     │
│  System  │ (LayoutNG)│ Pipeline │  + Skia Rasterization  │
│ (Blink)  │ (Blink)  │ (Blink)  │                        │
├──────────┴──────────┴──────────┴────────────────────────┤
│          Chromium Base (minimal subset)                   │
│   Threading, Task Runners, Memory, Geometry Types        │
├─────────────────────────────────────────────────────────┤
│                 Platform Layer                            │
│         (Linux/X11/Wayland → macOS → Windows)            │
└─────────────────────────────────────────────────────────┘
```

**Key difference from old plan:** The rendering pipeline is extracted as a **single tightly-coupled unit** from Chromium, not built as independent modules. Blink's style, layout, and paint are deeply intertwined — separating them would prevent pixel-perfect rendering.

## Key Decisions

| Decision | Choice | Rationale |
|---|---|---|
| **Extraction strategy** | Actual Chromium code, not reimplementation | Pixel-perfect rendering requires the real code; reimplementation would take years and never fully match |
| **Pipeline coupling** | Single library, not separate modules | Style/layout/paint/cc/ are tightly coupled in Chromium; separation breaks correctness |
| **What we extract** | Style + Layout + Paint + cc/ + Skia (~1.27M rendering LOC) | Full pipeline from styled elements to pixels |
| **What we strip** | V8, network stack, HTML/CSS parsers, browser process, DevTools | Everything that makes Chromium a "browser" vs a "renderer" |
| **DOM strategy** | Minimal adapter DOM (satisfies LayoutNG's input requirements) | Blink's layout expects Element/Node tree; we provide minimal versions driven by C API |
| **Core API** | Stable C ABI | Universal FFI — every language can call C |
| **First bindings** | Rust | Memory safety without GC, excellent C FFI |
| **Programming model** | Programmatic element tree + style properties | Users create elements, set CSS properties via C API, framework handles layout/paint/composite |
| **Initial platform** | Linux desktop (X11/Wayland) | Easiest Chromium build, best debugging |
| **Build system** | GN/Ninja (Chromium-native) | Chromium's own build system for maximum extraction fidelity |
| **Chromium version** | Pin to stable release | Avoid upstream churn |

## Sub-Projects

| # | Sub-Project | Deliverable | Status |
|---|---|---|---|
| 1 | [Research & Infrastructure](./01-research-and-infrastructure.md) | Build system, dependency maps, Skia POC | ✅ Complete |
| 2 | [Skia Extraction & C API](./02-skia-extraction.md) | `libopenui_skia.so` (standalone Skia wrapper) | ✅ Complete (deprecated — superseded by integrated pipeline) |
| 3 | [Rendering Pipeline Build](./03-rendering-pipeline-build.md) | Chromium rendering code compiling standalone | |
| 4 | [DOM Adapter & C API](./04-dom-adapter-and-c-api.md) | Programmatic element/style C API, layout queries | |
| 5 | [Full Pipeline Integration](./05-full-pipeline-integration.md) | Style → Layout → Paint → Composite → Pixels on screen | |
| 6 | [Widget Coverage & SVG](./06-widget-coverage-and-svg.md) | All HTML elements + full SVG, pixel-perfect verified | |
| 7 | [Animations & Advanced CSS](./07-animations-and-advanced-css.md) | CSS animations, transitions, transforms, filters, 60fps | |
| 8 | [Rust Bindings & Developer Experience](./08-rust-bindings.md) | `openui` crate on crates.io | |
| 9 | [Platform Expansion & Ecosystem](./09-platform-expansion.md) | Cross-platform (Linux, macOS, Windows) + tooling | |

```
SP1 ✅ → SP2 ✅ (deprecated) → SP3 → SP4 → SP5 → SP6 → SP7 → SP8 → SP9
```

**SP2 deprecation note:** SP2's standalone Skia C API was valuable for proving the C ABI approach and learning Skia's API surface. Going forward, Skia is used through Chromium's integrated rendering pipeline, not directly. The SP2 library and tests remain in the repo as reference.

## Repository Structure

```
open-ui/
├── docs/
│   ├── plan/                    # Project plans (this folder)
│   └── api/                     # API reference documentation
├── chromium/                    # Symlink → Chromium source checkout
├── src/
│   ├── adapter/                 # Minimal DOM adapter (Element, Node, Document)
│   ├── api/                     # C API implementation (openui.h wrapper)
│   ├── platform/                # Platform abstraction (windowing, input, GPU)
│   └── skia/                    # SP2 Skia wrapper (deprecated, kept as reference)
├── include/
│   └── openui/
│       └── openui.h             # Single unified C API header
├── build/                       # GN build configuration for extraction
├── bindings/
│   └── rust/                    # Rust crate workspace
├── examples/                    # Demo applications
├── tests/                       # Integration + pixel-comparison tests
├── tools/                       # Build scripts, test runners
├── BUILD.gn                     # Top-level build file
└── CHROMIUM_VERSION             # Pinned Chromium version
```

## What We Extract vs. Strip

### ✅ Extract (Chromium's rendering code)
| Component | ~LOC | Purpose |
|-----------|------|---------|
| `cc/` | 371K | Compositor: layer tree, tiling, raster, animations, frame scheduling |
| `third_party/blink/renderer/core/css/` | 295K | CSS cascade, selector matching, property resolution |
| `third_party/blink/renderer/core/layout/` | 264K | LayoutNG: flexbox, grid, block, inline, table, SVG layout |
| `third_party/blink/renderer/core/paint/` | 124K | Display item recording, paint invalidation |
| `third_party/blink/renderer/core/svg/` | 49K | SVG elements and rendering |
| `third_party/blink/renderer/core/style/` | 26K | ComputedStyle data structures |
| `third_party/blink/renderer/platform/graphics/` | 39K | Paint properties, compositing bridge |
| `base/` (subset) | ~100K | Threading, task runners, memory, logging |
| `ui/gfx/` | 80K | Geometry types, transforms, color spaces |
| `third_party/skia/` | ~3M | 2D graphics rasterization |

### ❌ Strip (web browser machinery)
- **V8** — JavaScript engine (stub bindings interfaces)
- **Network stack** — `net/`, fetch, XHR, WebSocket
- **HTML parser** — `html_parser/`, tokenizer, tree builder
- **CSS text parser** — `css_parser/` (we set styles programmatically)
- **Browser process** — `chrome/`, `content/browser/`
- **DevTools** — `devtools/`
- **Extensions** — `extensions/`
- **Media** — `media/` (audio, video codecs)
- **WebRTC** — Real-time communication
- **Service Workers** — Background workers
- **Storage** — IndexedDB, LocalStorage, cookies

## Risk Assessment

| Risk | Severity | Mitigation |
|---|---|---|
| V8 dependency in Blink | **Critical** | Stub V8 bindings interfaces. Layout/paint don't use V8 directly — it's the DOM API layer. |
| `base/` dependency sprawl | **Critical** | Extract minimal subset. Replace what we can with std:: equivalents. |
| Build complexity | **High** | Use Chromium's own GN/Ninja. Don't fight the build system. |
| Binary size | **High** | Link-time optimization, dead code stripping. Target <50MB initial, optimize later. |
| DOM adapter fidelity | **High** | Minimal DOM must satisfy LayoutNG's assumptions. Extensive pixel-comparison testing. |
| Chromium upstream churn | **Medium** | Pin to stable release. Update periodically with careful diffing. |
| Performance regression | **Medium** | Benchmark against Chromium rendering. Same code = same perf. |

## Success Criteria

| Sub-Project | Gate |
|---|---|
| SP3 | Chromium rendering code compiles and links as standalone library |
| SP4 | Create `<div>` with flexbox children via C API → get correct layout coordinates |
| SP5 | Render styled elements to a Linux window at 60fps via compositor |
| SP6 | Widget gallery renders all HTML elements identically to Chromium (pixel-diff < 0.1%) |
| SP7 | CSS animations and transitions run on compositor thread at 60fps |
| SP8 | `cargo add openui` → build a working app in Rust |
| SP9 | Same app runs on Linux, macOS, Windows |
