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
| SP5: Offscreen Rendering | ✅ Done | Rasterize to pixels/PNG, 14 pixel-perfect test pages, 196 tests |
| SP6: Widget Coverage & SVG | ✅ Done | 117 elements, SVG, resource provider, 39 pixel-perfect pages |
| SP7-SP9 | 📋 Planned | Animations, Rust bindings, platform expansion |

See [`docs/plan/`](docs/plan/) for the full project roadmap.

## Getting Started

See **[`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md)** for complete setup instructions, build guide, and architecture details.

### Quick Build (assumes Chromium already checked out)

```bash
cd ~/chromium/src

# Build all Open UI targets
./third_party/ninja/ninja -C out/Release openui_lib openui_api_test openui_c_test openui_render_test openui_c_render_test openui_render_pages -j24

# Run unit tests (176 total)
./out/Release/openui_api_test         # 78 tests
./out/Release/openui_c_test           # 32 tests
./out/Release/openui_render_test      # 20 tests
./out/Release/openui_c_render_test    # 46 tests

# Run pixel comparison (39 pages at 0% tolerance)
./out/Release/openui_render_pages --html tests/pixel_comparison/html_pages html_renders/
./out/Release/openui_render_pages openui_renders/ --html-dir tests/pixel_comparison/html_pages
node tests/pixel_comparison/compare_pixels.js html_renders openui_renders 0
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

## Pixel Comparison Screenshots

All 39 test pages render identically through both the HTML pipeline and C API at **0% pixel tolerance**.

### Element Test Sheets (15 pages)

| Test | Screenshot | Description |
|---|---|---|
| Semantic Blocks | ![](docs/screenshots/sp6/test_semantic_blocks.png) | `<section>`, `<article>`, `<aside>`, `<header>`, `<footer>`, `<nav>`, `<main>`, `<figure>`, `<address>` |
| Inline Text | ![](docs/screenshots/sp6/test_inline_text.png) | `<strong>`, `<em>`, `<code>`, `<kbd>`, `<mark>`, `<sub>`, `<sup>`, `<abbr>`, `<time>` |
| Headings & Text | ![](docs/screenshots/sp6/test_headings_text.png) | `<h1>`–`<h6>`, `<p>`, `<blockquote>`, `<pre>`, `<hr>` |
| Lists | ![](docs/screenshots/sp6/test_lists.png) | `<ul>`, `<ol>`, `<li>`, `<dl>`, `<dt>`, `<dd>`, nested lists |
| Tables | ![](docs/screenshots/sp6/test_tables.png) | `<table>`, `<thead>`, `<tbody>`, `<tfoot>`, colspan/rowspan, border-collapse |
| Forms | ![](docs/screenshots/sp6/test_forms.png) | `<input>`, `<select>`, `<textarea>`, `<button>`, `<fieldset>`, `<progress>`, `<meter>` |
| Flexbox | ![](docs/screenshots/sp6/test_flexbox.png) | All flex properties: direction, wrap, grow/shrink, align, justify, gap |
| Grid | ![](docs/screenshots/sp6/test_grid.png) | Grid template, areas, auto-flow, span, gap, alignment |
| Positioning | ![](docs/screenshots/sp6/test_positioning.png) | static, relative, absolute, fixed, sticky, z-index stacking |
| Box Model | ![](docs/screenshots/sp6/test_box_model.png) | margin, padding, border, box-sizing, outline, overflow |
| Colors & Backgrounds | ![](docs/screenshots/sp6/test_colors_backgrounds.png) | Linear/radial gradients, multiple backgrounds, opacity |
| Transforms & Filters | ![](docs/screenshots/sp6/test_transforms_filters.png) | rotate, scale, skew, translate, perspective, CSS filters |
| Advanced CSS | ![](docs/screenshots/sp6/test_advanced_css.png) | clip-path, columns, writing-mode, aspect-ratio, blend modes |
| SVG Shapes | ![](docs/screenshots/sp6/test_svg_shapes.png) | `<rect>`, `<circle>`, `<ellipse>`, `<line>`, `<polygon>`, `<path>` |
| SVG Advanced | ![](docs/screenshots/sp6/test_svg_advanced.png) | Gradients, filters, clip-path, masks, text paths |

### Rich Website Integration Tests (10 pages)

| Test | Screenshot | Description |
|---|---|---|
| Blog | ![](docs/screenshots/sp6/website_blog.png) | Article layout with sidebar, typography, tags |
| E-Commerce | ![](docs/screenshots/sp6/website_ecommerce.png) | Product grid, cards, pricing, cart |
| Dashboard | ![](docs/screenshots/sp6/website_dashboard.png) | Sidebar nav, charts area, stat cards, table |
| Landing Page | ![](docs/screenshots/sp6/website_landing.png) | Hero section, features grid, CTA, footer |
| Portfolio | ![](docs/screenshots/sp6/website_portfolio.png) | Project cards, skills grid, about section |
| News Portal | ![](docs/screenshots/sp6/website_news.png) | Multi-column layout, headlines, categories |
| Documentation | ![](docs/screenshots/sp6/website_docs.png) | Side nav, code blocks, API reference tables |
| Social Media | ![](docs/screenshots/sp6/website_social.png) | Feed, posts, profiles, interactions |
| Email Client | ![](docs/screenshots/sp6/website_email.png) | Folder list, message list, message view |
| Analytics | ![](docs/screenshots/sp6/website_analytics.png) | Dashboard with charts, KPIs, data tables |

### SP5 Core Pages (14 pages)

| Test | Screenshot |
|---|---|
| Red Box | ![](docs/screenshots/sp6/red_box.png) |
| RGB Flex | ![](docs/screenshots/sp6/rgb_flex.png) |
| Border Box | ![](docs/screenshots/sp6/border_box.png) |
| Nested Flex | ![](docs/screenshots/sp6/nested_flex.png) |
| Grid Colors | ![](docs/screenshots/sp6/grid_colors.png) |
| Rounded Shadows | ![](docs/screenshots/sp6/rounded_shadows.png) |
| Transforms | ![](docs/screenshots/sp6/transforms.png) |
| Opacity Gradients | ![](docs/screenshots/sp6/opacity_gradients.png) |
| Positioning Z-Index | ![](docs/screenshots/sp6/positioning_zindex.png) |
| Overflow Clipping | ![](docs/screenshots/sp6/overflow_clipping.png) |
| Complex UI | ![](docs/screenshots/sp6/complex_ui.png) |
| Typography | ![](docs/screenshots/sp6/typography.png) |
| Borders & Shadows | ![](docs/screenshots/sp6/borders_shadows.png) |
| Dashboard Layout | ![](docs/screenshots/sp6/dashboard_layout.png) |
