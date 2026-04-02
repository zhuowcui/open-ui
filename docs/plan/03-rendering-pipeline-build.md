# Sub-Project 3: Rendering Pipeline Build

> Extract Chromium's rendering code (style, layout, paint, cc/, Skia) and build it as a standalone library.

## Objective

Get Chromium's actual rendering pipeline — from ComputedStyle through LayoutNG through paint through cc/ compositing through Skia rasterization — compiling and linking as a standalone shared library outside of a full Chromium build. This is the foundation everything else builds on.

**This is NOT a reimplementation.** We extract the real Chromium source files, handle their dependencies, stub what we don't need (V8, networking), and produce a linkable library.

## Scope

### What Compiles In This Phase

| Component | Source Location | ~LOC | Notes |
|-----------|----------------|------|-------|
| `base/` (minimal) | `base/` | ~100K | Threading, task runners, callbacks, time, logging, memory, containers |
| `ui/gfx/` | `ui/gfx/` | 80K | Geometry, transforms, color, GPU fences |
| WTF | `third_party/blink/renderer/platform/wtf/` | ~50K | Blink's STL replacement (Vector, HashMap, String) |
| Platform fonts | `third_party/blink/renderer/platform/fonts/` | ~30K | Font selection, shaping, metrics |
| Platform geometry | `third_party/blink/renderer/platform/geometry/` | ~10K | LayoutUnit, PhysicalSize, etc. |
| Platform graphics | `third_party/blink/renderer/platform/graphics/` | ~40K | Paint properties, compositing bridge |
| Style | `third_party/blink/renderer/core/style/` | 26K | ComputedStyle |
| CSS | `third_party/blink/renderer/core/css/` | 295K | Style resolution (we keep resolver, stub parser) |
| Layout | `third_party/blink/renderer/core/layout/` | 264K | LayoutNG: all algorithms |
| Paint | `third_party/blink/renderer/core/paint/` | 124K | Display items, paint controller |
| SVG | `third_party/blink/renderer/core/svg/` + `core/layout/svg/` | 65K | SVG elements + layout |
| cc/ | `cc/` | 371K | Compositor, layers, tiling, raster, animations |
| Skia | `third_party/skia/` | ~3M | Already builds (SP1/SP2 proved this) |

### What We Stub/Remove

| Component | Strategy |
|-----------|----------|
| V8 | Stub binding interfaces — layout/paint never call V8 directly |
| HTML parser | Remove entirely — we build DOM programmatically |
| CSS text parser | Stub — we set style properties programmatically |
| Network stack | Remove entirely |
| Browser/content process | Remove entirely |
| DevTools | Remove entirely |
| Media (audio/video) | Remove — can add back as optional module later |
| `viz/` display compositor | Replace with direct-to-screen compositing |
| GPU command buffer | Simplify — in-process, no IPC needed |

## Phases

### Phase A: Dependency Mapping & Build Skeleton

**Goal:** Understand exactly which files we need and create the GN build structure.

**Tasks:**

1. **A1: Generate complete include graph** — For each rendering component (style, layout, paint, cc/), trace all `#include` paths to build a complete dependency graph. Identify every file needed.

2. **A2: Classify dependencies** — For each dependency, classify as: EXTRACT (need the real code), STUB (provide empty/minimal implementation), or REMOVE (strip via preprocessor/build flag).

3. **A3: Create GN build skeleton** — Set up `BUILD.gn` files that reference Chromium source files in their original locations (via the Chromium checkout symlink). Don't copy files — reference them. This way we track exactly which Chromium files we use.

4. **A4: Create stub headers** — For V8, networking, and other removed components, create stub headers that satisfy `#include` directives with minimal/empty implementations.

### Phase B: Base & Platform Compilation

**Goal:** Get `base/` subset and platform utilities compiling.

**Tasks:**

5. **B1: base/ subset** — Compile the minimal base/ we need: `base/threading/`, `base/task/`, `base/callback*.h`, `base/time/`, `base/logging.h`, `base/memory/`, `base/containers/`. This is the foundation everything depends on.

6. **B2: ui/gfx/ types** — Compile geometry types, transform utilities, color spaces. These are used everywhere in rendering code.

7. **B3: WTF & platform utilities** — Compile Blink's WTF (containers, strings) and platform utilities (fonts, geometry types).

8. **B4: Verification** — All base/platform code compiles and links. Run any existing unit tests that come along.

### Phase C: Rendering Core Compilation

**Goal:** Get style, layout, and paint code compiling.

**Tasks:**

9. **C1: Style/CSS compilation** — Get `core/style/` and `core/css/` compiling. The style resolver is the most complex piece. Stub the CSS text parser (we won't parse CSS strings).

10. **C2: Layout compilation** — Get `core/layout/` compiling including LayoutNG algorithms (block, flex, grid, inline, table) and SVG layout. This will expose the DOM dependency — layout expects Element/Node/Document.

11. **C3: DOM stubs** — Create minimal stubs for DOM types that satisfy layout's requirements. Layout needs: Node tree traversal, Element → ComputedStyle, Element → LayoutObject. It does NOT need: event handling, attribute parsing, JS API.

12. **C4: Paint compilation** — Get `core/paint/` and `platform/graphics/paint/` compiling. Paint depends on layout output (LayoutObject tree + fragments).

13. **C5: SVG compilation** — Get `core/svg/` compiling. SVG has its own element types and layout algorithms.

### Phase D: Compositor Compilation

**Goal:** Get cc/ compiling.

**Tasks:**

14. **D1: cc/ core** — Compile layer system, property trees, damage tracking, tile management.

15. **D2: cc/ raster** — Compile rasterization pipeline. Replace GPU command buffer with direct Skia GPU calls (we're in-process).

16. **D3: cc/ scheduling** — Compile frame scheduler, animation system.

17. **D4: Compositing bridge** — Compile `PaintArtifactCompositor` — the critical bridge between Blink's paint output and cc/'s layer tree.

### Phase E: Linking & Verification

**Goal:** Everything links into a single shared library.

**Tasks:**

18. **E1: Link resolution** — Resolve all undefined symbols. This is where missing stubs and dependencies surface. Iteratively fix until clean link.

19. **E2: Smoke test** — Write a minimal C++ test that creates a ComputedStyle, a LayoutObject, runs layout, and reads back the computed geometry. This proves the pipeline works end-to-end.

20. **E3: Library packaging** — Produce `libopenui_rendering.so` (or similar) with all rendering code. Measure binary size. Apply LTO and dead-code stripping.

## Build Strategy

### Reference, Don't Copy

We reference Chromium source files in-place via symlink:
```
open-ui/chromium → /home/nero/chromium/src
```

Our `BUILD.gn` files specify source lists that point into the Chromium tree:
```gn
source_set("layout") {
  sources = [
    "//chromium/third_party/blink/renderer/core/layout/layout_object.cc",
    "//chromium/third_party/blink/renderer/core/layout/block_layout_algorithm.cc",
    # ...
  ]
  include_dirs = [
    "//chromium",
    "//chromium/third_party/blink/renderer",
    "//src/stubs",  # Our stub headers
  ]
}
```

This approach:
- Tracks exactly which Chromium files we depend on
- Makes Chromium version upgrades a diffable operation
- Keeps our repo small (we don't vendor millions of LOC)
- Uses Chromium's own compiler and build infrastructure

### Stub Architecture

For removed dependencies, we provide stub headers in `src/stubs/`:
```
src/stubs/
├── v8/                    # Empty V8 API headers
│   ├── v8.h
│   └── v8-isolate.h
├── net/                   # Empty networking headers
├── mojo/                  # Empty Mojo IPC headers
└── services/              # Empty service headers
```

Stubs provide the minimal type definitions and function signatures needed to satisfy `#include` directives and compile-time checks, with empty or no-op implementations.

## Deliverables

| Deliverable | Description |
|---|---|
| `libopenui_rendering.so` | Standalone shared library containing full rendering pipeline |
| `build/BUILD.gn` | GN build files referencing Chromium sources |
| `src/stubs/` | Stub headers for removed dependencies |
| `tests/build_smoke_test.cc` | Minimal end-to-end compilation/link test |

## Success Criteria

- [ ] All rendering code compiles with zero errors
- [ ] All rendering code links into a single shared library
- [ ] Smoke test: create style → create layout object → run layout → read geometry
- [ ] Binary size measured and documented
- [ ] Build time < 30 minutes on a modern machine

## Key Risks

| Risk | Severity | Mitigation |
|---|---|---|
| V8 stubbing breaks compilation | **Critical** | Blink has deep V8 integration in DOM layer. We stub at the narrowest interface. Layout/paint/cc/ don't use V8 directly. |
| `base/` subset is insufficient | **High** | Start broad (include more of base/), narrow later. Better to have extra code than missing deps. |
| Circular dependencies | **High** | Chromium's include graph has cycles. May need to compile everything as a single target initially. |
| DOM stubs inadequate for layout | **High** | LayoutNG has assumptions about the DOM tree. Our stubs must satisfy those assumptions precisely. |
| Build takes too long | **Medium** | Use Chromium's compiler cache (goma/sccache). Incremental builds after initial compilation. |
