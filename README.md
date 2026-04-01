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

🚧 **Sub-Project 1: Research & Infrastructure** — Setting up build system and analyzing Chromium's rendering pipeline.

See [`docs/plan/`](docs/plan/) for the full project roadmap.

## Building

### Prerequisites (Ubuntu 22.04+)

```bash
# System dependencies
sudo apt install -y build-essential clang lld ninja-build python3 git curl
sudo apt install -y libfontconfig-dev libfreetype-dev libvulkan-dev
sudo apt install -y libx11-xcb-dev libxcb1-dev libwayland-dev

# depot_tools (provides GN)
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git
export PATH="$PATH:$(pwd)/depot_tools"
```

### Build

```bash
git clone --recursive https://github.com/user/open-ui.git
cd open-ui
gn gen out/Debug
ninja -C out/Debug
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
