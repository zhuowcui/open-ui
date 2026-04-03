# Open UI

**Extract Chromium's rendering pipeline as a standalone, language-agnostic UI framework.**

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
├──────────┴──────────┴───────────────┴───────────────────┤
│                 Platform Layer                            │
│         (Linux/X11/Wayland → macOS → Windows)            │
└─────────────────────────────────────────────────────────┘
```

## What is this?

Chromium's rendering layer is native code magic — layout, compositing, and rasterization running at 60fps+ with world-class correctness. But it's buried under layers of web platform machinery (HTML/CSS/JS parsing, DOM, downloading, evaluation) that consume enormous memory and CPU.

**Open UI** extracts that rendering layer into four modular libraries with a stable C ABI:

| Library | Source | Purpose |
|---|---|---|
| `libopenui_skia` | Skia | 2D graphics rasterization |
| `libopenui_compositor` | `cc/` | GPU-accelerated compositing, tiling, animations |
| `libopenui_layout` | Blink LayoutNG | CSS layout (Block, Flex, Grid, Inline) |
| `libopenui_style` | Blink Style | Cascade, inheritance, computed values |
| **`libopenui`** | All above | Unified framework with declarative scene graph |

Each layer is independently usable. Use the full stack for app development, or just Skia + Compositor for a game engine.

## Status

| Sub-Project | Status | Description |
|---|---|---|
| SP1: Research & Infrastructure | ✅ Done | Chromium checkout, build system, dependency analysis |
| SP2: Skia Extraction | ✅ Done (deprecated) | Standalone Skia wrapper — replaced by direct blink integration |
| SP3: Rendering Pipeline | ✅ Done | Blink style→layout→paint integrated, 20 tests |
| SP4: DOM Adapter & C API | ✅ Done | 65-function C API, 130 tests passing |
| SP5: Offscreen Rendering | 🔨 Next | Rasterize to pixels/PNG via PaintRecord playback |
| SP6-SP9 | 📋 Planned | Widgets, animations, Rust bindings, platform expansion |

See [`docs/plan/`](docs/plan/) for the full project roadmap.

## Getting Started

See **[`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md)** for complete setup instructions, build guide, and architecture details.

### Quick Build (assumes Chromium already checked out)

```bash
cd ~/chromium/src

# Build all Open UI targets
./third_party/ninja/ninja -C out/Release openui_lib openui_api_test openui_c_test openui_rendering_test

# Run all tests (130 total)
./out/Release/openui_rendering_test   # 20 tests
./out/Release/openui_api_test         # 78 tests
./out/Release/openui_c_test           # 32 tests
```

## Chromium Version

Pinned to **M147** (`147.0.7727.24`). See [`CHROMIUM_VERSION`](CHROMIUM_VERSION).

## Project Structure

```
open-ui/
├── src/                  # Framework source code
│   ├── skia/             # Skia integration + C API
│   ├── compositor/       # Compositor + C API
│   ├── layout/           # Layout engine + C API
│   ├── style/            # Style system + C API
│   ├── scene_graph/      # Unified scene graph
│   └── platform/         # Platform abstraction
├── include/openui/       # Public C API headers
├── bindings/rust/        # Rust crate
├── third_party/chromium/ # Chromium sources (sparse submodule)
├── examples/             # Demo applications
├── docs/                 # Architecture docs & plans
└── tools/                # Analysis & build scripts
```

## License

Apache-2.0. See [LICENSE](LICENSE).
