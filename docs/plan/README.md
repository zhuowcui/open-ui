# Open UI — Project Plan Overview

> Extract Chromium's battle-tested rendering pipeline into a modular, standalone UI framework programmable from any language.

## Vision

Chromium's rendering layer is pure native code magic — layout, compositing, and rasterization running at 60fps+ with best-in-class correctness across virtually every OS. But it's buried under layers of web platform machinery: HTML/CSS/JS parsing, downloading, evaluation, and a DOM that consumes enormous memory and CPU.

**Open UI** extracts that rendering layer — Skia, the compositor, the layout engine, and the style system — into a standalone UI framework with a stable C ABI. Any language can drive it. No browser required.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Application Code                       │
│              (Rust, C, Python, Go, etc.)                 │
├─────────────────────────────────────────────────────────┤
│                    openui.h                               │
│          Retained Scene Graph + Declarative API          │
├──────────┬──────────┬───────────────┬───────────────────┤
│  Style   │  Layout  │  Compositor   │       Skia        │
│ System   │  Engine  │    (cc/)      │   (2D Graphics)   │
│          │ (LayoutNG)│              │                   │
├──────────┴──────────┴───────────────┴───────────────────┤
│                 Platform Layer                            │
│         (Linux/X11/Wayland → macOS → Windows)            │
└─────────────────────────────────────────────────────────┘
```

Each layer is an **independent module** with its own C API. Users can opt into the full stack for app development, or use individual layers (e.g., just Skia + Compositor for a game engine).

## Key Decisions

| Decision | Choice | Rationale |
|---|---|---|
| **Scope** | All four layers (Skia, Compositor, Layout, Style) | Full pipeline is what gives Chromium its magic |
| **Modularity** | Each layer is an independent module | Maximum flexibility for diverse use cases |
| **Core API** | Stable C ABI | Universal FFI surface — every language can call C |
| **First bindings** | Rust | Memory safety without GC, excellent C FFI, strong systems community |
| **Programming model** | Retained scene graph + declarative API | Maps naturally to Blink's internals; industry direction |
| **Initial platform** | Linux desktop (X11/Wayland) | Simplest windowing, best OSS GPU debugging, easiest Chromium build |
| **Build system** | GN/Ninja (Chromium-native) | Minimize extraction friction; evaluate CMake migration later |
| **Chromium version** | Pin to a stable release | Avoid upstream churn; backport selectively |

## Sub-Projects

The project is divided into 8 sequential sub-projects. Each produces a usable, testable artifact and has its own detailed plan document.

| # | Sub-Project | Deliverable | Dependencies |
|---|---|---|---|
| 1 | [Research & Infrastructure](./01-research-and-infrastructure.md) ✅ | Dependency maps, build system, Skia POC | — |
| 2 | [Skia Extraction & C API](./02-skia-extraction.md) | `libopenui_skia.so` | SP1 |
| 3 | [Compositor Extraction](./03-compositor-extraction.md) | `libopenui_compositor.so` | SP2 |
| 4 | [Layout Engine Extraction](./04-layout-engine-extraction.md) | `libopenui_layout.so` | SP3 |
| 5 | [Style System Extraction](./05-style-system-extraction.md) | `libopenui_style.so` | SP4 |
| 6 | [Unified Pipeline & Scene Graph](./06-scene-graph.md) | `libopenui.so` | SP5 |
| 7 | [Native Widget Toolkit](./07-native-widget-toolkit.md) | HTML-equivalent compiled widgets + platform services | SP6 |
| 8 | [Rust Bindings & Developer Experience](./08-rust-bindings.md) | `openui` crate on crates.io | SP7 |
| 9 | [Platform Expansion & Ecosystem](./09-platform-expansion.md) | Cross-platform framework + tooling | SP8 |

```
SP1 → SP2 → SP3 → SP4 → SP5 → SP6 → SP7 → SP8 → SP9
```

## Repository Structure

```
open-ui/
├── docs/
│   ├── plan/                    # This folder — project plans
│   └── architecture/            # Technical architecture docs (created in SP1)
├── third_party/
│   ├── chromium/                # Extracted Chromium sources (vendored)
│   └── ...                      # Other third-party deps
├── src/
│   ├── base/                    # Extracted/minimal base utilities
│   ├── skia/                    # Skia integration layer + C API
│   ├── compositor/              # Extracted cc/ + C API
│   ├── layout/                  # Extracted layout engine + C API
│   ├── style/                   # Extracted style system + C API
│   ├── scene_graph/             # Retained scene graph
│   └── platform/                # Platform abstraction (windowing, input)
├── include/
│   └── openui/                  # Public C API headers
│       ├── openui.h
│       ├── openui_skia.h
│       ├── openui_compositor.h
│       ├── openui_layout.h
│       └── openui_style.h
├── bindings/
│   └── rust/                    # Rust crate workspace
├── examples/                    # Demo applications
├── tests/                       # Integration tests
├── BUILD.gn                     # Top-level build file
└── .gn                          # GN configuration
```

## Risk Assessment

| Risk | Severity | Mitigation |
|---|---|---|
| `base/` dependency sprawl | **Critical** | Map early. Create minimal shim implementing only what we need. |
| Layout engine DOM coupling | **High** | Target LayoutNG (cleaner interfaces). Create LayoutNode adapter. |
| Build system complexity | **High** | Start with Chromium's GN/Ninja. Migrate only if extraction justifies it. |
| Chromium upstream churn | **Medium** | Pin to stable release. Update periodically, not continuously. |
| Performance regression | **Medium** | Continuous benchmarking against Chromium. Aggressive profiling. |
| Scope creep | **Medium** | Strict phase gates. Each sub-project ships a usable artifact. |

## Success Criteria

| Sub-Project | Gate |
|---|---|
| SP1 | Build system compiles Skia. Dependency map complete. |
| SP2 | Render text and shapes to a Linux window via C API. |
| SP3 | Multi-layer compositing at 60fps via C API. |
| SP4 | Correct Flexbox and Grid layout via C API. |
| SP5 | Style cascade and computed values via C API. |
| SP6 | Declarative hello world app renders correctly. |
| SP7 | Widget gallery renders all HTML-equivalent controls. |
| SP8 | `cargo add openui` → build a working app. |
| SP9 | Same app runs on Linux, macOS, Windows. |
