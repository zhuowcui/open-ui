# Open UI — Product Specification

## Vision

Open UI extracts Chromium's rendering pipeline — the most battle-tested, standards-compliant
rendering engine in the world — into a standalone, language-agnostic UI framework. It enables
any application, in any language, to render UIs with the exact same quality and correctness
as Google Chrome, without embedding a web browser.

## Problem Statement

Chromium's rendering layer (Skia + Blink LayoutNG + cc/ compositor) is extraordinary
engineering: correct CSS layout, GPU-accelerated compositing, subpixel text rendering,
all running at 60fps+. But it's inaccessible outside the browser. It's entangled with web
platform machinery — HTML/CSS/JS parsing, DOM, networking, V8 — that consumes enormous
memory and CPU for applications that don't need a full browser.

Native UI frameworks (Qt, GTK, Win32, AppKit) have their own rendering engines with
different layout semantics, different text rendering, and different visual output. None
match Chromium's rendering quality. Game engines have custom rendering but no CSS layout.
Electron gives you Chromium but at the cost of shipping an entire browser.

## Solution

Open UI surgically extracts Chromium's rendering layer into four modular libraries:

| Library | Chromium Source | Purpose |
|---|---|---|
| `libopenui_skia` | Skia | 2D graphics: shapes, text, images, gradients |
| `libopenui_compositor` | cc/ | GPU compositing, tiling, layer management |
| `libopenui_layout` | Blink LayoutNG | CSS layout: Block, Flex, Grid, Inline, Table |
| `libopenui_style` | Blink Style | CSS cascade, inheritance, computed values |
| **`libopenui`** | All above | Unified framework with scene graph API |

### Key Properties

1. **Stable C ABI**: All libraries expose C headers. Any language with C FFI can use them.
2. **Modular**: Use the full stack for app development, or individual layers for specific needs.
3. **Identical output**: Pixel-for-pixel identical to Chrome for equivalent CSS/layout.
4. **Native performance**: No JavaScript runtime, no DOM overhead, no networking stack.
5. **Chromium-pinned**: Tracks a specific Chromium version (currently M147, `147.0.7727.24`).

## Target Users

1. **Application developers** who want Chrome-quality rendering without embedding a browser
2. **Game developers** who need CSS layout for UI elements within game engines
3. **Embedded systems** that need high-quality rendering with constrained resources
4. **Language ecosystem maintainers** building UI frameworks for Rust, Go, Python, etc.
5. **Desktop application frameworks** that want Chromium-grade text rendering and layout

## Non-Goals

- We are NOT a web browser. No HTML parsing, no JavaScript, no networking.
- We are NOT a widget toolkit. We provide rendering primitives, not buttons and dialogs.
- We do NOT implement deprecated CSS features (features marked deprecated in Chromium are skipped).
- We do NOT provide our own rendering algorithms. Every algorithm is ported from Chromium.

## Quality Requirements

| Requirement | Standard |
|---|---|
| Pixel accuracy | 0% tolerance vs headless Chromium |
| CSS compliance | 100% of Chromium's WPT pass rate for implemented features |
| Performance | Within 2x of Chromium's native performance |
| Test coverage | Chromium's own test suites + exhaustive pixel comparison |
| Code quality | Dual-model AI review (Opus 4.6 + GPT 5.4) with 0 issues |
| Documentation | CSS spec references on every algorithm, architecture docs on every module |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Application Code                       │
│              (Rust, C, Python, Go, etc.)                 │
├─────────────────────────────────────────────────────────┤
│                    openui.h / Rust crate                  │
│          Retained Scene Graph + Declarative API          │
├──────────┬──────────┬───────────────┬───────────────────┤
│  Style   │  Layout  │  Compositor   │       Skia        │
│ System   │  Engine  │    (cc/)      │   (2D Graphics)   │
├──────────┴──────────┴───────────────┴───────────────────┤
│                 Platform Layer                            │
│         (Linux/X11/Wayland → macOS → Windows)            │
└─────────────────────────────────────────────────────────┘
```

### Rust Crate Structure (bindings/rust/)

```
openui-geometry/     — LayoutUnit, LogicalSize, PhysicalRect, writing mode conversion
openui-style/        — CSS properties, computed values, cascade, inheritance
openui-text/         — Font loading, HarfBuzz shaping, text segmentation, BiDi
openui-layout/       — Block, Flex, Inline layout algorithms, margin collapsing, floats
openui-paint/        — Skia-based painting: text, decorations, backgrounds, borders
openui/              — Framework: view! macro, signals, components, App shell
```

## Chromium Version

Pinned to **M147** (`147.0.7727.24`). The pinned version is recorded in `CHROMIUM_VERSION`
at the repository root. All algorithm ports and test comparisons target this specific version.
