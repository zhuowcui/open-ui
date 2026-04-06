# Open UI — Technical Implementation Plan

## Current State

Open UI has completed 12 sprints (SP1–SP12) covering:

- **C/C++ layer**: Full Chromium Blink rendering pipeline wrapped with 65-function C ABI
- **Rust layer**: Native rendering engine with Block, Flex, Inline layout + full text rendering
- **Test infrastructure**: 7,505 tests, 612+ pixel comparison tests, WPT test translation framework
- **Developer API**: `view!` macro for React-like Rust UI development

### Crate Architecture

```
bindings/rust/
├── openui-geometry/     (2,200 LOC) — LayoutUnit, logical/physical types, writing modes
├── openui-style/        (8,500 LOC) — CSS properties, computed values, cascade
├── openui-text/         (12,000 LOC) — Font system, HarfBuzz shaping, BiDi, line breaking
├── openui-layout/       (18,000 LOC) — Block, Flex, Inline, Ruby layout algorithms
├── openui-paint/        (8,000 LOC) — Skia painting: text, decorations, backgrounds, borders
└── openui/              (1,600 LOC) — Framework: view! macro, signals, components
```

**Total**: ~50,000 LOC source + ~90,000 LOC tests = ~140,000 LOC Rust

### What's Implemented

| CSS Feature | Source | Status | Tests |
|---|---|---|---|
| Block layout (BFC, margins, floats, clearance) | Blink LayoutNG | ✅ 100% | 7,505 |
| Flex layout | Blink FlexLayoutAlgorithm | ✅ 100% | 617 |
| Inline layout (line breaking, vertical-align) | Blink InlineLayoutAlgorithm | ✅ 100% | 1,902 |
| Text rendering (shaping, decorations, BiDi) | Blink/HarfBuzz | ✅ 100% | 3,371 |
| Ruby annotations | Blink RubyAnnotation | ✅ 100% | — |
| CSS Box Model | CSS 2.1 | ✅ 100% | — |
| CSS Positioning (relative, absolute, fixed, sticky) | CSS Position 3 | ✅ 100% | — |
| CSS Overflow (visible, hidden, scroll, auto, clip) | CSS Overflow 3 | ✅ 100% | — |
| CSS Sizing (intrinsic, min/max, fit-content) | CSS Sizing 3 | ✅ 100% | — |
| Block fragmentation / multicol | CSS Break 3 | ✅ 100% | — |

### What's NOT Yet Implemented

| CSS Feature | Chromium Source | Priority |
|---|---|---|
| Table layout | Blink TableLayoutAlgorithm | High |
| Grid layout | Blink GridLayoutAlgorithm | High |
| CSS Transforms | Blink TransformPaintPropertyNode | Medium |
| CSS Animations | Blink Animation | Medium |
| CSS Filters | Blink FilterEffect | Medium |
| CSS Variables | Blink CSSVariableResolver | Medium |
| Scroll snap | Blink SnapCoordinator | Low |
| CSS Containment | Blink LayoutNGContainment | Low |

## Upcoming Sprints

### SP13: CSS Inline Layout Integration (Next)

Complete the integration between inline and block layout:
- Resolve deferred items from SP12 (abspos inline static position, float avoidance in inline)
- Inline intrinsic block-size contribution to block containers
- Mixed inline/block content handling
- Inline-block, inline-flex, inline-grid formatting contexts

### SP14: CSS Table Layout

Port Blink's `TableLayoutAlgorithm`:
- Table, table-row, table-cell formatting
- Column/row sizing algorithm
- Border-collapse model
- `<colgroup>`/`<col>` width distribution
- Percentage and auto sizing

### SP15: CSS Grid Layout

Port Blink's `GridLayoutAlgorithm`:
- Track sizing algorithm (definite, min-content, max-content, fr)
- Auto-placement algorithm
- Named grid areas and lines
- Alignment (justify/align items/content/self)
- Subgrid

### SP16: CSS Transforms & Visual Effects

- 2D/3D transforms (translate, rotate, scale, skew, matrix)
- Transform origin
- Perspective
- CSS Filters (blur, brightness, contrast, etc.)
- Blend modes
- Clipping (clip-path)

### SP17: Platform Windowing

- winit or SDL2 integration for native windows
- Event loop (keyboard, mouse, touch)
- Compositor integration (GPU tiling, damage tracking)
- High-DPI support
- Multiple monitors

## Technology Stack

| Component | Technology |
|---|---|
| Language (native engine) | Rust 1.94.1 |
| Language (Chromium layer) | C++ (Chromium M147) |
| 2D Graphics | Skia (via skia-safe) |
| Text shaping | HarfBuzz (via Skia's SkShaper) |
| Build system (Rust) | Cargo |
| Build system (C++) | GN + Ninja (Chromium) |
| Testing | Rust's built-in test framework |
| CI | Chromium's test infrastructure |
| Pixel comparison | headless Chromium vs our renderer, 0% tolerance |

## Build & Test

```bash
# Rust layer (primary development)
cd bindings/rust
export PATH="$HOME/.cargo/bin:$HOME/local/bin:$HOME/depot_tools:$PATH"
cargo test --workspace          # Full test suite (7,505 tests)
cargo check --workspace         # Fast compilation check

# C/C++ layer (Chromium integration)
cd ~/chromium/src
./third_party/ninja/ninja -C out/Release openui_lib -j24
./out/Release/openui_api_test   # 78 tests
./out/Release/openui_c_test     # 32 tests
```
