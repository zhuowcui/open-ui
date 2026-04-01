# Sub-Project 1: Research & Infrastructure ✅ COMPLETE

> Deep understanding of Chromium's rendering internals, documented dependency maps, and a build infrastructure that can compile Chromium components in isolation.
>
> **Status:** All 27 tasks complete. SP1-E5 (GPU stretch goal) deferred to SP2.

## Objective

Before we extract a single line of code, we need to know exactly what we're extracting, what it depends on, and where the natural seams are. This phase produces the knowledge base and tooling that every subsequent phase depends on.

## Resolved Decisions

These open questions from the initial plan have been resolved:

| Question | Decision | Rationale |
|---|---|---|
| Vendor vs. submodule? | **Sparse-checkout git submodule** | Tracks upstream Chromium but pulls only the ~15 directories we need (~2-3GB vs 30GB+). Full control via sparse-checkout config. |
| Chromium version? | **Pin to M147 (147.0.7727.x)** | Latest stable as of project start (March 2026). Freeze here; evaluate upstream backports quarterly. |
| Upstream Skia vs. Chromium fork? | **Upstream Skia for POC, then evaluate delta** | Upstream builds standalone easily. During POC, catalog Chromium's patches to decide if we need them. |
| Reimplement vs. extract `base/`? | **Extract first, shim later** | Start by extracting the real `base/` subset for correctness. Once stable, identify candidates to replace with lighter alternatives (e.g., `absl` types, standard C++20). |

---

## Phase A: Environment & Chromium Checkout

Get a working Chromium source tree and development environment.

### A1. Development environment setup

**Install system prerequisites (Ubuntu 22.04+):**
```bash
# Build essentials
sudo apt install -y build-essential clang lld ninja-build python3 git curl

# Chromium-specific
sudo apt install -y gperf bison flex pkg-config libglib2.0-dev
sudo apt install -y libdrm-dev libgbm-dev libegl-dev libgl-dev
sudo apt install -y libfontconfig-dev libfreetype-dev
sudo apt install -y libx11-xcb-dev libxcb1-dev libxcb-shm0-dev
sudo apt install -y libwayland-dev wayland-protocols
sudo apt install -y libvulkan-dev vulkan-tools
```

**Install `depot_tools`:**
```bash
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git
export PATH="$PATH:$(pwd)/depot_tools"
```

**Validate:** `gn --version` and `ninja --version` both succeed.

### A2. Chromium source acquisition (full checkout)

We need a full Chromium checkout as a **reference** for analysis. The sparse submodule comes later (A4) once we know exactly which directories to include.

```bash
mkdir chromium-ref && cd chromium-ref
fetch --nohooks --no-history chromium
cd src
gclient runhooks
```

**Important:** `--no-history` avoids pulling full git history (~80GB). We only need the source tree at M147.

```bash
git fetch origin refs/tags/147.0.7727.24
git checkout 147.0.7727.24
gclient sync --nohooks
gclient runhooks
```

**Validate:** `gn gen out/Default && ninja -C out/Default base` compiles base/.

### A3. Verify reference build compiles key targets

Before analysis, verify that the key components build in the reference checkout:

```bash
# Build individual targets to validate isolation
ninja -C out/Default cc                       # Compositor
ninja -C out/Default third_party/skia         # Skia
ninja -C out/Default blink_core               # Contains layout + style

# Build the content_shell as a baseline "does everything work" check
ninja -C out/Default content_shell
```

**Validate:** All targets compile. content_shell launches and renders a page.

### A4. Set up sparse-checkout submodule in open-ui repo

Once we know the reference checkout works, configure our repo to pull only what we need:

```bash
cd /path/to/open-ui
git submodule add --depth 1 --branch 147.0.7727.24 \
  https://chromium.googlesource.com/chromium/src.git third_party/chromium
cd third_party/chromium
git sparse-checkout init --cone
git sparse-checkout set \
  base \
  build \
  cc \
  gpu \
  ui/gfx \
  viz \
  third_party/skia \
  third_party/blink/renderer/core/layout \
  third_party/blink/renderer/core/css \
  third_party/blink/renderer/core/style \
  third_party/blink/renderer/core/paint \
  third_party/blink/renderer/platform/fonts \
  third_party/blink/renderer/platform/graphics \
  third_party/icu \
  third_party/harfbuzz-ng \
  third_party/freetype \
  third_party/fontconfig \
  third_party/zlib \
  third_party/libpng \
  third_party/libjpeg_turbo \
  third_party/libwebp \
  third_party/abseil-cpp \
  testing/gtest
```

Create `CHROMIUM_VERSION` in repo root:
```
147.0.7727.24
```

Create `.gitmodules` entry and commit.

**Validate:** `ls third_party/chromium/base/` shows base/ contents, `du -sh third_party/chromium/` is < 3GB.

---

## Phase B: Architecture Study & Documentation

Deep-read the Chromium source to understand the rendering pipeline before attempting extraction.

### B1. Document the rendering pipeline overview

Study and produce `docs/architecture/rendering-pipeline-overview.md`:

**Data flow to trace and document (with source file citations):**

```
                         Main Thread
                            │
   ┌────────────────────────┼────────────────────────┐
   │                        │                        │
   ▼                        ▼                        ▼
Style Resolution         Layout                    Paint
(StyleResolver →         (LayoutNG →               (PaintController →
 ComputedStyle)           NGLayoutResult →           PaintRecord →
                          NGPhysicalFragment)         DisplayItemList)
                                                     │
                                          ┌──────────┼──────────┐
                                          ▼                     ▼
                                     Commit to              Property Tree
                                     Compositor             Update
                                          │                     │
                          ────────────────┼─────────────────────┼──────
                         Compositor Thread│                     │
                                          ▼                     ▼
                                     Layer Tree              Property Trees
                                     Activation              (Transform, Clip,
                                          │                    Effect, Scroll)
                                          ▼
                                     Tile Management
                                     & Rasterization ──────▶ Raster Workers
                                          │                    (Skia on thread pool)
                                          ▼
                                     Frame Assembly
                                     & GPU Submit
                                          │
                                          ▼
                                     Display / VSync
```

**For each stage, document:**
- Entry point function name and file
- Input data structures
- Output data structures
- Threading constraints (which thread does it run on?)
- Key configuration knobs
- Where our extraction "cut" will go

### B2. Document Skia integration points

Produce `docs/architecture/skia-integration.md`:

- How Chromium configures Skia's build (`skia/BUILD.gn` args)
- Chromium-specific Skia patches (diff upstream vs. `third_party/skia/`)
- GPU backend configuration (Vulkan, GL, Dawn/WebGPU)
- `SkCanvas` usage patterns in paint code
- `SkImage`, `SkPicture`, `SkSurface` lifecycle
- Font backend: how Chromium configures FreeType + HarfBuzz through Skia
- List every Chromium-side file that directly calls Skia API

### B3. Document compositor architecture

Produce `docs/architecture/compositor-architecture.md`:

- `cc/` directory structure walkthrough (every subdirectory's purpose)
- Layer types and their use cases
- Property trees: what they are, how they're built, how they optimize rendering
- The commit cycle: `BeginMainFrame` → `Commit` → `Activate` → `Draw`
- Tile lifecycle: creation → priority → rasterization → GPU upload → eviction
- Interaction with `viz/`: what does `cc/` send to viz? What do we replace?
- Interaction with `gpu/`: command buffer vs. direct GPU calls
- `cc::PaintRecord` / `cc::DisplayItemList` — how paint output feeds into compositor

### B4. Document layout engine architecture

Produce `docs/architecture/layout-engine-architecture.md`:

- LayoutNG vs. legacy layout: which types use which?
- `NGConstraintSpace` → `NGLayoutAlgorithm` → `NGLayoutResult` → `NGPhysicalFragment` pipeline
- For each algorithm (Block, Inline, Flex, Grid, Table):
  - Entry point file and class
  - Input requirements
  - Output structure
  - DOM dependencies (what does it read from `Element`/`Node`?)
- Inline layout deep dive: `NGInlineNode`, `NGLineBreaker`, text shaping pipeline
- How layout reads style (`ComputedStyle` access patterns)
- Layout tree vs. DOM tree: `LayoutObject` tree, how it differs from DOM

### B5. Document style system architecture

Produce `docs/architecture/style-system-architecture.md`:

- `StyleResolver` entry point and resolution algorithm
- Cascade: specificity calculation, origin sorting, `!important` handling
- `ComputedStyle`: memory layout, field storage, style sharing
- Property system: how individual CSS properties are defined (`css_properties.json5`)
- Inheritance: which properties inherit, resolution of `inherit`/`initial`/`unset`
- Value resolution: relative units, `calc()`, `currentColor`, custom properties
- Style invalidation: what triggers re-style? How is it scoped?
- Interaction with layout: which style changes trigger re-layout?

### B6. Document threading model

Produce `docs/architecture/threading-model.md`:

- Main thread responsibilities and ownership
- Compositor thread: `SingleThreadProxy` vs. `ThreadProxy`
- Raster worker pool: `CategorizedWorkerPool`
- Thread-safe vs. main-thread-only data structures
- Synchronization points: commit, activation, begin-frame
- How Chromium avoids data races between threads
- Implications for our extraction (we want the same model)

---

## Phase C: Dependency Analysis

Systematic dependency mapping for each target layer.

### C1. Tooling: Build dependency extraction scripts

Create scripts in `tools/` that automate dependency analysis using GN:

**`tools/analyze_deps.py`** — Given a GN target, produce:
- Direct dependency list (`gn desc out/Default <target> deps`)
- Transitive dependency tree (`gn desc out/Default <target> deps --tree`)
- Source file list (`gn desc out/Default <target> sources`)
- Include analysis (grep `#include` in source files, categorize by component)

**`tools/analyze_base_usage.py`** — For a set of source files:
- Grep all `#include "base/..."` directives
- Categorize by `base/` subsystem (threading, memory, containers, etc.)
- Produce a usage matrix: which source files use which `base/` features

**`tools/analyze_cross_layer.py`** — For a component boundary:
- Find all `#include` directives that cross from one component to another
- Identify the actual symbols used at each crossing
- Classify as "can be stubbed" vs. "must be extracted"

### C2. Skia dependency analysis

Run tools against Skia targets. Produce `docs/architecture/deps-skia.md`:

```
Layer: Skia (third_party/skia/)
────────────────────────────────
Source files: [count from gn desc]
Total LOC: [via cloc/scc]

Direct Chromium dependencies:
  - build/: Build configuration, toolchain definitions
  - third_party/freetype/: Font rasterization
  - third_party/harfbuzz-ng/: Text shaping
  - third_party/icu/: Unicode support (subset)
  - third_party/zlib/: Compression for image codecs
  - third_party/libpng/: PNG codec
  - third_party/libjpeg_turbo/: JPEG codec
  - third_party/libwebp/: WebP codec

base/ usage: [expected: minimal — Skia has its own utilities]

Chromium-specific configuration:
  - GN args that differ from upstream Skia build
  - Patches applied on top of upstream

Natural API boundary:
  - Skia already has a clean C++ API (SkCanvas, SkPaint, SkSurface, etc.)
  - Our C API wraps this directly
  - Boundary is clean: Skia is a leaf dependency

Extraction difficulty: Low
  - Skia is designed to be standalone
  - Chromium patches are additive, not structural
```

### C3. Compositor (cc/) dependency analysis

Run tools against cc/ targets. Produce `docs/architecture/deps-compositor.md`:

Focus areas:
- Complete list of `base/` symbols used by cc/
- All `#include` paths into `gpu/`, `viz/`, `ui/gfx/`
- Exactly which `viz/` APIs cc/ calls (these must be replaced)
- Which `gpu/` APIs cc/ uses (command buffer? GL context? Vulkan?)
- Catalog every `base::TaskRunner` and `base::Thread` usage
- Map cc/'s interaction with Blink's paint output (`PaintRecord`, `DisplayItemList`)

### C4. Layout engine dependency analysis

Run tools against layout targets. Produce `docs/architecture/deps-layout.md`:

Focus areas:
- Every `#include` from `core/layout/` into `core/dom/` — these are the DOM couplings we must sever
- Every `ComputedStyle` property access from layout code — this defines our style→layout interface
- Platform font dependencies (`platform/fonts/`)
- ICU usage (bidi, line breaking, segmentation)
- HarfBuzz usage (text shaping from inline layout)
- `LayoutObject` hierarchy: which subclasses exist and what they add

### C5. Style system dependency analysis

Run tools against style targets. Produce `docs/architecture/deps-style.md`:

Focus areas:
- `css_properties.json5` analysis: which properties exist, which are inherited, which affect layout vs. paint
- `StyleResolver` dependencies: what does it need beyond the style system itself?
- `ComputedStyle` field inventory: every stored property, its type, default value
- Cascade implementation dependencies
- Custom property (CSS variable) implementation scope

### C6. `base/` minimal subset specification

Synthesize C2-C5 results into `docs/architecture/base-minimal-subset.md`:

- Union of all `base/` symbols used across all four layers
- Categorize each into:
  - **Must extract as-is** (complex behavior that's hard to reimplement: task runners, threading)
  - **Can replace with std/absl** (containers, optional, span, string_view)
  - **Can stub** (logging, tracing, histograms — replace with no-ops or simple impls)
  - **Can drop** (features only needed by browser chrome, IPC, etc.)
- Estimate: LOC of full `base/` vs. our minimal subset
- Target: < 20% of full `base/`

### C7. Cross-layer interface mapping

Produce `docs/architecture/cross-layer-interfaces.md`:

Map the exact function signatures at each layer boundary:
- **Blink → Compositor**: How does Blink commit paint output to cc/?
- **Compositor → Skia**: How does cc/ invoke Skia for rasterization?
- **Style → Layout**: How does layout read style? (`ComputedStyle` getters)
- **Layout → Paint**: How do layout results feed into paint? (`NGPhysicalFragment` → paint)

For each interface, note:
- The actual function signatures
- Data structures passed across the boundary
- Threading constraints
- Where we'll "cut" to insert our C API

---

## Phase D: Repository Structure & Build System

Set up the project infrastructure.

### D1. Repository skeleton

Create the directory structure:

```
open-ui/
├── .gn                              # GN root configuration
├── BUILD.gn                         # Top-level build targets
├── BUILDCONFIG.gn                   # Toolchain and default configs
├── CHROMIUM_VERSION                  # "147.0.7727.24"
├── LICENSE                           # Apache-2.0 (matches Chromium's BSD spirit)
├── README.md                         # Project overview
├── CONTRIBUTING.md                   # Contribution guide + coding standards
├── .clang-format                     # Chromium clang-format config
├── .gitmodules                       # Chromium sparse submodule
├── docs/
│   ├── plan/                         # Project plans (existing)
│   ├── architecture/                 # Architecture docs (from Phase B)
│   └── adr/                          # Architecture Decision Records
├── third_party/
│   └── chromium/                     # Sparse-checkout submodule
├── src/
│   ├── base/                         # (future) Extracted/shimmed base utilities
│   ├── skia/                         # (future) Skia integration + C API
│   ├── compositor/                   # (future) Compositor + C API
│   ├── layout/                       # (future) Layout engine + C API
│   ├── style/                        # (future) Style system + C API
│   ├── scene_graph/                  # (future) Scene graph
│   └── platform/                     # (future) Platform abstraction
├── include/
│   └── openui/                       # (future) Public C API headers
├── bindings/
│   └── rust/                         # (future) Rust crate workspace
├── examples/
│   └── skia_poc/                     # Skia POC (Phase E)
├── tests/                            # (future) Integration tests
├── tools/
│   ├── analyze_deps.py               # Dependency analysis scripts (Phase C)
│   ├── analyze_base_usage.py
│   └── analyze_cross_layer.py
├── build/
│   ├── toolchain/                    # GN toolchain definitions
│   │   └── linux/
│   │       └── BUILD.gn              # Linux clang toolchain
│   └── config/
│       ├── compiler/
│       │   └── BUILD.gn              # Compiler flags
│       └── BUILD.gn                  # Shared build config
└── .github/
    └── workflows/
        ├── ci.yml                    # Main CI pipeline
        └── format-check.yml         # clang-format check
```

### D2. GN/Ninja build system bootstrap

We need a build system that can:
1. Reference Chromium sources from the submodule
2. Build our own code alongside extracted Chromium code
3. Use Chromium's toolchain configuration

**`.gn`** (root):
```gn
buildconfig = "//BUILDCONFIG.gn"
```

**`BUILDCONFIG.gn`:**
- Define the default toolchain (Linux, Clang)
- Set default compiler flags (C++20, warnings, optimization levels)
- Import Chromium's build config where needed

**`build/toolchain/linux/BUILD.gn`:**
- Define `clang` toolchain with standard Linux paths
- Configure LLD linker
- Set sysroot if needed

**Initial target: build a "hello world" C++ file with our build system.**

```bash
gn gen out/Debug
ninja -C out/Debug hello_world
./out/Debug/hello_world  # prints "Open UI build system works!"
```

### D3. Integrate Chromium submodule sources into build

Configure GN to find and compile sources from `third_party/chromium/`:

- Set up `//third_party/chromium/` as a source root
- Import Chromium's `build/` config selectively (compiler config, platform detection)
- Test: build a single `base/` file (e.g., `base/logging.cc`) from the submodule
- This validates that our GN setup can compile Chromium sources

### D4. CI/CD pipeline

**`.github/workflows/ci.yml`:**

```yaml
name: CI
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-22.04
    strategy:
      matrix:
        build_type: [Debug, Release]
        compiler: [clang]  # Start with clang only (Chromium's primary)
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive  # Pulls sparse-checkout submodule
      - name: Install dependencies
        run: ./tools/install-deps.sh
      - name: Generate build
        run: gn gen out/${{ matrix.build_type }} --args='is_debug=${{ matrix.build_type == "Debug" }}'
      - name: Build
        run: ninja -C out/${{ matrix.build_type }}
      - name: Test
        run: ninja -C out/${{ matrix.build_type }} test && out/${{ matrix.build_type }}/run_tests
```

**`.github/workflows/format-check.yml`:**
- Run `clang-format --dry-run --Werror` on all `src/` and `include/` files
- Run `gn format --dry-run` on all `BUILD.gn` files

### D5. Project documentation files

**`README.md`:**
- Project vision (one paragraph)
- Architecture diagram (ASCII)
- Build instructions
- Link to `docs/plan/` and `docs/architecture/`

**`CONTRIBUTING.md`:**
- C++ style: Chromium style for extracted code, Google C++ style for our code
- C API conventions: `oui_` prefix, handle-based, error codes, no C++ in headers
- Commit message format
- PR process
- How to run tests

**`LICENSE`:**
- Apache-2.0 (compatible with Chromium's BSD-3-Clause, permits combining)

**`docs/adr/001-sparse-checkout-submodule.md`:**
- First ADR: documenting the sparse-checkout decision

**`docs/adr/002-chromium-version-pinning.md`:**
- Document M147 pinning decision and quarterly review cadence

**`docs/adr/003-base-extraction-strategy.md`:**
- Document extract-first-shim-later strategy for `base/`

---

## Phase E: Skia Standalone POC

Validate the full extraction approach by building Skia outside of Chromium.

### E1. Catalog Skia build targets from Chromium

In the reference checkout:
```bash
gn desc out/Default //third_party/skia:skia deps --tree > skia_deps_tree.txt
gn desc out/Default //third_party/skia:skia sources > skia_sources.txt
```

Analyze:
- How many source files?
- Which third-party deps are pulled in?
- What GN args configure the Skia build? (check `third_party/skia/BUILD.gn`)

### E2. Attempt upstream Skia build

As a comparison, build upstream Skia (from skia.org) to see how its standalone build works:

```bash
git clone https://skia.googlesource.com/skia.git upstream-skia
cd upstream-skia
python3 tools/git-sync-deps
bin/gn gen out/Release --args='
  is_official_build=true
  skia_use_vulkan=true
  skia_use_gl=true
  skia_enable_gpu=true
  skia_use_freetype=true
  skia_use_fontconfig=true
  skia_use_harfbuzz=true
  skia_use_icu=true
'
ninja -C out/Release
```

**Document the delta**: diff upstream Skia config vs. Chromium's `third_party/skia/` config.

### E3. Write Skia POC application

Create `examples/skia_poc/main.cc` (~200 lines):

```cpp
// 1. Create a raster surface (no windowing needed for POC)
// 2. Get canvas from surface
// 3. Draw:
//    a. Clear background to dark blue
//    b. Draw a red rounded rectangle
//    c. Draw a gradient-filled circle
//    d. Draw "Hello, Open UI!" text with HarfBuzz shaping
//    e. Draw a complex path (star shape)
// 4. Encode surface to PNG
// 5. Write PNG to "output.png"
```

This validates:
- Skia raster backend works
- Font loading and text shaping work
- Image encoding works
- Path rendering works

### E4. Build POC with our build system

Create `examples/skia_poc/BUILD.gn`:
- Depends on Skia (from submodule or upstream, whichever we got working)
- Links necessary third-party libs
- Produces `skia_poc` executable

```bash
gn gen out/Debug
ninja -C out/Debug skia_poc
./out/Debug/skia_poc
# → writes output.png
```

**Validate:** Open `output.png`, verify it shows the expected shapes and text.

### E5. POC with GPU rendering (stretch goal)

If time permits, extend the POC to render via GPU:
- Create a minimal X11 window with an EGL/Vulkan surface
- Bind Skia to the GPU surface
- Render the same scene as E3 to the window
- Present to screen

This validates GPU context setup for SP2.

---

## Deliverables

| ID | Deliverable | Location | Phase |
|---|---|---|---|
| D-1 | Chromium reference checkout at M147 | External (not in repo) | A |
| D-2 | Sparse-checkout submodule | `third_party/chromium/` | A |
| D-3 | Rendering pipeline overview doc | `docs/architecture/rendering-pipeline-overview.md` | B |
| D-4 | Skia integration doc | `docs/architecture/skia-integration.md` | B |
| D-5 | Compositor architecture doc | `docs/architecture/compositor-architecture.md` | B |
| D-6 | Layout engine architecture doc | `docs/architecture/layout-engine-architecture.md` | B |
| D-7 | Style system architecture doc | `docs/architecture/style-system-architecture.md` | B |
| D-8 | Threading model doc | `docs/architecture/threading-model.md` | B |
| D-9 | Dependency analysis scripts | `tools/analyze_*.py` | C |
| D-10 | Skia dependency report | `docs/architecture/deps-skia.md` | C |
| D-11 | Compositor dependency report | `docs/architecture/deps-compositor.md` | C |
| D-12 | Layout dependency report | `docs/architecture/deps-layout.md` | C |
| D-13 | Style dependency report | `docs/architecture/deps-style.md` | C |
| D-14 | `base/` minimal subset spec | `docs/architecture/base-minimal-subset.md` | C |
| D-15 | Cross-layer interface map | `docs/architecture/cross-layer-interfaces.md` | C |
| D-16 | Repo skeleton + build system | Root, `build/`, `.gn`, `BUILD.gn` | D |
| D-17 | CI pipeline | `.github/workflows/` | D |
| D-18 | README, CONTRIBUTING, LICENSE | Root | D |
| D-19 | ADRs (×3) | `docs/adr/` | D |
| D-20 | Skia POC application | `examples/skia_poc/` | E |

## Success Criteria

- [ ] Chromium M147 reference checkout builds `cc`, `skia`, and `blink_core` targets
- [ ] Sparse-checkout submodule is < 3GB and contains all needed directories
- [ ] All 6 architecture documents are complete with source file citations
- [ ] All 4 dependency analysis reports are complete with exact symbol lists
- [ ] `base/` minimal subset is identified at < 20% of full `base/`
- [ ] Cross-layer interface map identifies every cut point with function signatures
- [ ] GN/Ninja build system compiles our code on Ubuntu 22.04+
- [ ] CI pipeline runs on every push and reports build status
- [ ] Skia POC compiles with our build system and produces correct `output.png`
- [ ] All 3 ADRs are written and committed

## Task Dependency Graph

```
A1 (dev env)
 └──▶ A2 (chromium checkout)
       └──▶ A3 (verify build)
             ├──▶ A4 (sparse submodule)──────────────────▶ D1 (repo skeleton)
             │                                              ├──▶ D2 (GN bootstrap)
             │                                              │     └──▶ D3 (integrate submodule)
             │                                              │           └──▶ E4 (build POC)
             │                                              ├──▶ D4 (CI)
             │                                              └──▶ D5 (docs)
             ├──▶ B1 (pipeline overview)
             ├──▶ B2 (skia integration)──────────────────▶ E2 (upstream skia)
             ├──▶ B3 (compositor arch)                      └──▶ E3 (write POC)
             ├──▶ B4 (layout arch)                                └──▶ E4 (build POC)
             ├──▶ B5 (style arch)                                       └──▶ E5 (GPU stretch)
             └──▶ B6 (threading model)
                   │
                   ▼
             C1 (analysis scripts)
             ├──▶ C2 (skia deps)
             ├──▶ C3 (compositor deps)
             ├──▶ C4 (layout deps)
             └──▶ C5 (style deps)
                   └──▶ C6 (base/ subset)
                         └──▶ C7 (cross-layer interfaces)
```

**Parallelism opportunities:**
- B1-B6 can all run in parallel once A3 is done
- C2-C5 can run in parallel once C1 is done
- D1-D5 can mostly run in parallel (D2→D3 is sequential)
- Phase B and Phase D can run concurrently
