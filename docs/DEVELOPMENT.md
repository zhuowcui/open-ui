# Open UI — Development Guide

> Complete guide to set up, build, and continue development on the Open UI project.
> This document contains everything needed to resume work with fresh context.

## Table of Contents

- [Project Overview](#project-overview)
- [Architecture](#architecture)
- [Repository Structure](#repository-structure)
- [Chromium Source Code Strategy](#chromium-source-code-strategy)
- [Prerequisites & System Requirements](#prerequisites--system-requirements)
- [Setup From Scratch](#setup-from-scratch)
- [Build Instructions](#build-instructions)
- [Running Tests](#running-tests)
- [Current State & Progress](#current-state--progress)
- [SP5: Next Implementation Phase](#sp5-next-implementation-phase)
- [Key Technical Decisions](#key-technical-decisions)
- [Troubleshooting](#troubleshooting)

---

## Project Overview

Open UI extracts Chromium's rendering pipeline into a standalone UI framework programmable from any language via a stable C API. The rendering layer (style → layout → paint) is pure native code that renders at 60fps+. We strip away the web bloat (HTML parsing, JS evaluation, network stack) and expose the rendering engine directly.

**Goal:** Any language with C FFI can build UI that renders with Chromium-quality correctness and performance — pixel-identical to what Chromium produces.

**Non-goals:** We don't ship a browser. No HTML parser, no JS engine (V8 is used internally by blink but not exposed), no network stack, no DevTools.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Application Code (C, Rust, Python, Go, etc.)            │
│  Uses the C API: oui_element_create(), set_style(), etc. │
└────────────────────────┬─────────────────────────────────┘
                         │ C ABI
┌────────────────────────▼─────────────────────────────────┐
│  openui.h — Stable C API (~65 functions)                 │
│  Opaque handles: OuiDocument*, OuiElement*               │
└────────────────────────┬─────────────────────────────────┘
                         │
┌────────────────────────▼─────────────────────────────────┐
│  DOM Adapter Layer (openui_impl.cc)                      │
│  Maps C API calls → blink::Element, blink::Document      │
│  Element factory, style application, geometry queries     │
└────────────────────────┬─────────────────────────────────┘
                         │
┌────────────────────────▼─────────────────────────────────┐
│  Chromium Blink Rendering Pipeline (extracted, not forked)│
│  ├── Style Engine (css/, style/) — 321K LOC              │
│  ├── LayoutNG (layout/) — 264K LOC                       │
│  ├── Paint (paint/) — generates cc::PaintRecord          │
│  └── DummyPageHolder (test infra, used as our "page")    │
└────────────────────────┬─────────────────────────────────┘
                         │
┌────────────────────────▼─────────────────────────────────┐
│  Skia (via cc::PaintRecord::Playback)                    │
│  Rasterizes paint commands → pixels (SkBitmap)           │
└──────────────────────────────────────────────────────────┘
```

## Repository Structure

```
open-ui/                          # Our repo (git)
├── docs/
│   ├── plan/
│   │   ├── README.md             # Master roadmap (SP1-SP9)
│   │   ├── 01-research-and-infrastructure.md
│   │   ├── 02-skia-extraction.md           # Deprecated
│   │   ├── 03-rendering-pipeline-build.md
│   │   ├── 04-dom-adapter-and-c-api.md
│   │   ├── 05-full-pipeline-integration.md # ← SP5 (next)
│   │   ├── 06-widget-coverage-and-svg.md
│   │   ├── 07-animations-and-advanced-css.md
│   │   ├── 08-rust-bindings.md
│   │   └── 09-platform-expansion.md
│   └── DEVELOPMENT.md           # This file
├── src/
│   ├── api/                     # Mirror of chromium/src/openui/ files
│   │   ├── BUILD.gn
│   │   ├── openui.h             # Public C API header
│   │   ├── openui_impl.h/.cc   # C++ implementation
│   │   ├── openui_init.h/.cc   # Blink runtime bootstrap
│   │   ├── openui_element_factory.h/.cc
│   │   ├── openui_api_test.cc  # 78 C++ GTest tests
│   │   └── openui_c_test.c     # 32 pure C tests
│   ├── rendering_pipeline/chromium/
│   │   ├── BUILD.gn
│   │   ├── rendering_test.cc   # 20 rendering pipeline tests (SP3)
│   │   └── smoke_test.cc
│   └── skia/                   # SP2 Skia wrapper (deprecated)
├── examples/
├── tests/
├── CHROMIUM_VERSION             # "147.0.7727.24"
└── BUILD.gn                    # Root build (our standalone GN, unused for chromium builds)
```

### Chromium Side (not checked into our repo)

```
~/chromium/                      # Chromium checkout (37GB)
├── .gclient                     # depot_tools config
└── src/                         # Chromium source
    ├── openui/                  # ← OUR CODE LIVES HERE (symlink or copy)
    │   ├── BUILD.gn             # GN build rules (compiles against blink)
    │   ├── openui.h             # C API header
    │   ├── openui_impl.h/.cc   # Implementation
    │   ├── openui_init.h/.cc   # Runtime init
    │   ├── openui_element_factory.h/.cc
    │   ├── openui_api_test.cc  # C++ tests
    │   ├── openui_c_test.c     # C tests
    │   ├── rendering_test.cc   # Rendering tests
    │   ├── smoke_test.cc
    │   └── skia_poc.cc
    ├── out/Release/             # Build output (8.4GB)
    │   └── args.gn              # Build configuration
    ├── build/config/unsafe_buffers_paths.txt  # Has "-openui/" exemption
    └── third_party/blink/       # The rendering engine we use
```

## Chromium Source Code Strategy

**We do NOT check Chromium source into our repo.** Chromium is ~37GB. Instead:

1. **Chromium is checked out separately** at `~/chromium/` using `depot_tools`
2. **Our code lives inside `chromium/src/openui/`** so it can use Chromium's GN build system and link against blink
3. **We mirror our openui/ files** into `open-ui/src/api/` in our repo for version control
4. **The `CHROMIUM_VERSION` file** records which Chromium version we build against

### Why Inside Chromium?

Blink cannot be built standalone. It depends on:
- V8 (JavaScript engine, used internally for CSS custom properties etc.)
- base/ (Chromium's base library — threading, memory, strings)
- mojo/ (IPC — blink uses it internally)
- cc/ (compositor — paint records, display lists)
- skia/ (rendering backend)
- ICU (internationalization)

These are all compiled as part of Chromium's unified build. Our `openui/BUILD.gn` declares deps on these targets and Chromium's GN/Ninja builds everything together.

### Static Library Approach

We build `openui_lib` as a **static library** (`libopenui_lib.a`). Shared library (.so) doesn't work because V8 uses TLS with `initial-exec` model, incompatible with `-shared`/`-fPIC`.

## Prerequisites & System Requirements

### Hardware
- **Disk:** ~60GB free (37GB Chromium checkout + 8.4GB build output + headroom)
- **RAM:** 16GB minimum (8GB will struggle with linking)
- **CPU:** More cores = faster builds. 8+ cores recommended

### Software (Ubuntu/Debian)

```bash
# System packages (Chromium's install-build-deps.sh handles most of this)
sudo apt update
sudo apt install -y git python3 curl lsb-release sudo

# depot_tools (Google's build tooling)
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git ~/depot_tools
export PATH="$HOME/depot_tools:$PATH"
# Add to ~/.bashrc or ~/.zshrc:
echo 'export PATH="$HOME/depot_tools:$PATH"' >> ~/.bashrc
```

### Verified Working Environment
- **OS:** Ubuntu 24.04 / Pop!_OS 24.04 LTS (x86_64)
- **Python:** 3.12.3
- **Chromium's bundled clang** (in `third_party/llvm-build/Release+Asserts/bin/`)
- **GN:** 2352 (via `buildtools/linux64/gn`)
- **Ninja:** 1.12.1 (via `third_party/ninja/ninja`)

## Setup From Scratch

### Step 1: Clone Our Repo

```bash
git clone <our-repo-url> ~/code/open-ui
cd ~/code/open-ui
```

### Step 2: Fetch Chromium Source

```bash
mkdir ~/chromium && cd ~/chromium

# Create .gclient config
cat > .gclient << 'EOF'
solutions = [
  {
    "name": "src",
    "url": "https://chromium.googlesource.com/chromium/src.git",
    "managed": False,
    "custom_deps": {},
    "custom_vars": {},
  },
]
EOF

# Fetch Chromium (this takes 30-60 minutes on fast internet)
# The version we use is recorded in CHROMIUM_VERSION
gclient sync --no-history --shallow

# Install build dependencies (Linux only)
cd src
./build/install-build-deps.sh
```

**Note:** `gclient sync` downloads ~15GB and unpacks to ~37GB. Use `--no-history --shallow` to minimize download.

### Step 3: Copy Our Code Into Chromium

```bash
# Copy our openui source files into the chromium tree
mkdir -p ~/chromium/src/openui

# Copy API layer files
cp ~/code/open-ui/src/api/*.{h,cc,c} ~/chromium/src/openui/
cp ~/code/open-ui/src/api/BUILD.gn ~/chromium/src/openui/

# Copy rendering pipeline tests
cp ~/code/open-ui/src/rendering_pipeline/chromium/rendering_test.cc ~/chromium/src/openui/
cp ~/code/open-ui/src/rendering_pipeline/chromium/smoke_test.cc ~/chromium/src/openui/

# Add unsafe_buffers exemption for our code
# (Chromium requires unsafe buffer annotations; we skip this)
echo '-openui/' >> ~/chromium/src/build/config/unsafe_buffers_paths.txt
```

### Step 4: Configure Build

```bash
cd ~/chromium/src

# Create build directory with our config
mkdir -p out/Release
cat > out/Release/args.gn << 'EOF'
is_debug = false
is_component_build = false
use_sysroot = true
target_cpu = "x64"
is_official_build = false
symbol_level = 0
blink_symbol_level = 0
root_extra_deps = ["//openui:openui_smoke_test", "//openui:openui_rendering_test"]
EOF

# Generate ninja files (takes ~30 seconds)
./buildtools/linux64/gn gen out/Release
```

### Step 5: Build

```bash
cd ~/chromium/src

# Build the static library (~10-15 min first time, uses all cores)
./third_party/ninja/ninja -C out/Release openui_lib

# Build all test targets
./third_party/ninja/ninja -C out/Release openui_api_test openui_c_test openui_rendering_test
```

### Step 6: Verify

```bash
cd ~/chromium/src

# Run rendering pipeline tests (20 tests)
./out/Release/openui_rendering_test

# Run C++ API tests (78 tests)
./out/Release/openui_api_test

# Run C consumer tests (32 tests)
./out/Release/openui_c_test
```

**Expected output:** All 130 tests pass (78 + 32 + 20).

## Build Instructions

### Quick Reference

All builds happen from `~/chromium/src/`:

```bash
cd ~/chromium/src

# Regenerate ninja (needed after BUILD.gn changes)
./buildtools/linux64/gn gen out/Release

# Build specific targets
./third_party/ninja/ninja -C out/Release openui_lib           # static library
./third_party/ninja/ninja -C out/Release openui_api_test      # C++ API tests
./third_party/ninja/ninja -C out/Release openui_c_test        # C consumer tests
./third_party/ninja/ninja -C out/Release openui_rendering_test # rendering tests

# Build all openui targets at once
./third_party/ninja/ninja -C out/Release openui_lib openui_api_test openui_c_test openui_rendering_test
```

### Build Configuration (args.gn)

```
is_debug = false              # Release build (faster, smaller)
is_component_build = false    # Static linking (required — V8 TLS issue with shared)
use_sysroot = true            # Use Chromium's sysroot
target_cpu = "x64"
is_official_build = false     # Don't need official build optimizations
symbol_level = 0              # No debug symbols (faster build)
blink_symbol_level = 0        # No blink debug symbols
root_extra_deps = [...]       # Our targets added to the top-level build
```

### Adding New Build Targets

Edit `~/chromium/src/openui/BUILD.gn`. The file uses Chromium's GN build system. Key patterns:

```gn
# Shared blink dependencies (used by all targets)
_blink_deps = [
  "//base", "//base/test:test_support",
  "//cc", "//cc/paint",
  "//gin", "//mojo/core/embedder", "//mojo/public/cpp/bindings",
  "//skia",
  "//third_party/blink/renderer/controller",
  "//third_party/blink/renderer/core",
  "//third_party/blink/renderer/core:testing",
  "//third_party/blink/renderer/platform",
  "//third_party/blink/renderer/platform:test_support",
  "//ui/base", "//ui/gfx", "//ui/gfx/geometry",
  "//v8",
]

# Static library target
static_library("openui_lib") {
  testonly = true
  sources = [ ... ]
  deps = _blink_deps
}

# Test target (uses GTest)
test("openui_api_test") {
  sources = [ "openui_api_test.cc" ]
  deps = [ ":openui_lib" ] + _blink_deps
}
```

## Running Tests

```bash
cd ~/chromium/src

# All tests
./out/Release/openui_rendering_test   # 20 tests — rendering pipeline
./out/Release/openui_api_test         # 78 tests — C++ API
./out/Release/openui_c_test           # 32 tests — pure C consumer

# Run specific test
./out/Release/openui_api_test --gtest_filter="OpenUIAPITest.CreateDiv"

# Verbose output
./out/Release/openui_api_test --gtest_print_time=0 --gtest_color=yes
```

## Current State & Progress

### Completed Sub-Projects

| SP | Name | Status | Description |
|----|------|--------|-------------|
| SP1 | Research & Infrastructure | ✅ Done | Chromium checkout, build system, dependency analysis |
| SP2 | Skia Extraction & C API | ✅ Done (deprecated) | Standalone Skia wrapper — replaced by direct blink integration |
| SP3 | Rendering Pipeline Build | ✅ Done | Blink style→layout→paint working, 20 tests |
| SP4 | DOM Adapter & C API | ✅ Done | 65-function C API, 110 tests (78 C++ + 32 C) |

### Next: SP5 — Offscreen Rendering Pipeline

**Goal:** Rasterize paint output into pixels. `PaintRecord::Playback(SkCanvas)` → SkBitmap → RGBA/PNG.

See `docs/plan/05-full-pipeline-integration.md` for full plan.

### Future Sub-Projects (not started)

| SP | Name | Description |
|----|------|-------------|
| SP6 | Widget Coverage & SVG | Full HTML element coverage, SVG stack |
| SP7 | Animations & Advanced CSS | CSS animations, transitions, scroll effects |
| SP8 | Rust Bindings | Type-safe Rust API wrapping the C API |
| SP9 | Platform Expansion | Windows, macOS, Android, iOS builds |

## SP5: Next Implementation Phase

### What SP5 Implements

New C API functions for offscreen rendering:
- `oui_document_render_to_bitmap()` → RGBA pixel buffer
- `oui_document_render_to_png()` → PNG file on disk
- `oui_document_render_to_png_buffer()` → PNG in memory

### Architecture (SP5 specific)

```
C API calls → DOM → Style → Layout → Paint
                                       │
                              UpdateAllLifecyclePhasesForTest()
                                       │
                              LocalFrameView::GetPaintRecord()
                                       │ returns cc::PaintRecord
                              PaintRecord::Playback(SkCanvas*)
                                       │
                              SkBitmap (BGRA pixels in memory)
                                       │
                    ┌──────────────────┴──────────────────┐
                    │                                     │
          render_to_bitmap()                    render_to_png()
          (RGBA pixel buffer)             (gfx::PNGCodec → file)
```

**Key insight:** No cc/ compositor needed. `GetPaintRecord()` returns all drawing commands as a flat `cc::PaintRecord`. Playback to `SkiaPaintCanvas` targeting an `SkBitmap` produces pixels directly. This is the same path Chromium's layout tests use.

### New Files to Create

```
chromium/src/openui/
├── openui_render.h             # Render API internal header
├── openui_render.cc            # Core rasterization implementation
├── openui_pixel_diff.h         # Pixel comparison utility header
├── openui_pixel_diff.cc        # Pixel comparison implementation
├── openui_render_test.cc       # C++ pixel correctness tests
├── openui_c_render_test.c      # C consumer render tests
└── test_data/references/       # Reference PNGs for comparison
```

### BUILD.gn Changes Needed

1. Add `openui_render.cc` to `openui_lib` sources
2. Add `//ui/gfx/codec` to deps (for PNG encoding)
3. Add new test targets: `openui_render_test`, `openui_c_render_test`

### Key Chromium APIs Used

| API | Header | Purpose |
|-----|--------|---------|
| `LocalFrameView::GetPaintRecord()` | `local_frame_view.h` | Get all paint ops as cc::PaintRecord |
| `PaintRecord::Playback(SkCanvas*)` | `cc/paint/paint_record.h` | Replay paint ops to canvas |
| `SkBitmap` + `SkCanvas` | Skia headers | Raster rendering target |
| `gfx::PNGCodec::EncodeBGRASkBitmap()` | `ui/gfx/codec/png_codec.h` | Encode pixels to PNG |
| `cc::WritePNGFile()` | `cc/test/pixel_test_utils.h` | Write PNG to disk (test utility) |

### Tasks (22 total)

**Phase A** (3 tasks): Core rasterization, bitmap API, PNG output
**Phase B** (7 tasks): Pixel correctness tests (colors, layout, text, transforms, opacity)
**Phase C** (3 tasks): Pixel comparison infrastructure (reference images, diff utility)
**Phase D** (2 tasks): Pure C consumer tests
**Phase E** (5 tasks): Edge cases (empty doc, viewport sizes, re-render, multi-doc, large tree)
**Phase F** (2 tasks): BUILD.gn + multi-agent review

## Key Technical Decisions

### 1. Blink Initialization Pattern

There are TWO initialization paths:

**For standalone apps (C test, real applications):**
```c
OuiInitConfig config = {0};
oui_init(&config);
// Creates TaskEnvironment, initializes blink, V8, mojo, etc.
```

**For test harnesses (GTest):**
```cpp
// main() does manual blink init (same as rendering_test.cc):
// 1. base::TestSuite(argc, argv)
// 2. InitializeICUForTesting()
// 3. FeatureList
// 4. ResourceBundle (content_shell.pak)
// 5. mojo::core::Init()
// 6. V8 snapshot
// 7. Platform::InitializeBlink()
// 8. WebThreadScheduler
// 9. InitializeWithoutIsolateForTesting()
// 10. WebRuntimeFeatures

// Then per-test:
class MyTest : public testing::Test {
  void SetUp() override {
    task_env_ = std::make_unique<blink::test::TaskEnvironment>();
    page_holder_ = std::make_unique<blink::DummyPageHolder>(gfx::Size(800, 600));
  }
  // task_env_ creates V8 isolate, page_holder_ creates document
};
```

### 2. Element Wrapper Lifecycle

- `OuiElementImpl` wraps `blink::Persistent<blink::Element>` (prevents GC)
- Element tracker: `std::unordered_map<void*, OuiElementImpl*>`
- `oui_document_destroy()` DELETES all element wrappers (releases Persistent handles)
- After document destroy, caller's `OuiElement*` handles are dangling (same as `free()` in C)
- All API functions guard against null `impl->element` for safety

### 3. DummyPageHolder Quirks

- `SetLayoutSize()` DCHECKs if `LayoutSizeFixedToFrameSize()` is true (default)
- Must call `SetLayoutSizeFixedToFrameSize(false)` before resizing viewport

### 4. oui_shutdown() is Terminal

Blink/V8 don't support re-initialization in a single process. After shutdown, `oui_init()` returns `OUI_ERROR_ALREADY_INITIALIZED`.

### 5. Windows Portability

`base::FilePath::StringType` is `std::wstring` on Windows. Use `base::FilePath::FromUTF8Unsafe()` for cross-platform path construction.

### 6. Content Shell PAK File

Blink requires `content_shell.pak` for the user-agent stylesheet (IDR_UASTYLE_HTML_CSS). Without it, elements don't get default styles. The pak is loaded via `ui::ResourceBundle` at init time, found via `base::DIR_ASSETS`.

## Troubleshooting

### Build Errors

**"unsafe_buffers" warnings:**
Ensure `build/config/unsafe_buffers_paths.txt` has `-openui/` exemption.

**"TLS initial-exec" link error:**
V8 uses initial-exec TLS. Cannot use `-shared` flag. Must build as static library.

**GN gen fails:**
```bash
# Regenerate from scratch
rm -rf out/Release
mkdir -p out/Release
# Write args.gn (see Step 4 above)
./buildtools/linux64/gn gen out/Release
```

### Runtime Crashes

**SEGV in oui_init():**
- Check that `content_shell.pak` exists in `out/Release/`
- If not, build it: `./third_party/ninja/ninja -C out/Release content_shell_pak`

**V8 isolate crash in tests:**
- Ensure `blink::Persistent<Element>` handles are released BEFORE `TaskEnvironment` destruction
- In test fixtures: destroy `page_holder_` before `task_env_`

**Double TaskEnvironment:**
- `oui_init()` creates its own TaskEnvironment. Don't create another in test code.
- For test harnesses, call `openui_runtime_mark_initialized_externally()` instead of `oui_init()`

### Syncing Files

After editing files in `~/chromium/src/openui/`, copy them back to the repo:
```bash
cp ~/chromium/src/openui/*.{h,cc,c} ~/code/open-ui/src/api/
cp ~/chromium/src/openui/BUILD.gn ~/code/open-ui/src/api/
cp ~/chromium/src/openui/rendering_test.cc ~/code/open-ui/src/rendering_pipeline/chromium/
```

### Disk Space

Full Chromium checkout + build: ~45GB. If low on space:
```bash
# Clean build artifacts (need to rebuild after)
rm -rf ~/chromium/src/out/Release

# Or just clean ninja cache
./third_party/ninja/ninja -C out/Release -t clean
```
