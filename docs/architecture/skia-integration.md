# Skia Integration in Chromium — Architecture Reference

> **Open UI** · Architecture Document
> Chromium M147 (`147.0.7727.24`) · June 2025

This document describes how Chromium integrates, configures, and uses Skia for 2D
graphics rasterization. It serves as the reference for Open UI's Skia extraction
work (Sub-Project 2) and informs decisions about what to extract versus what to
replace with upstream Skia defaults.

---

## Table of Contents

1. [Skia in Chromium — Overview](#1-skia-in-chromium--overview)
2. [Build Configuration](#2-build-configuration)
3. [Chromium-Specific Patches](#3-chromium-specific-patches)
4. [GPU Backend Architecture](#4-gpu-backend-architecture)
5. [Canvas Usage Patterns](#5-canvas-usage-patterns)
6. [Font and Text Pipeline](#6-font-and-text-pipeline)
7. [Image Pipeline](#7-image-pipeline)
8. [Open UI Extraction Notes](#8-open-ui-extraction-notes)

---

## 1. Skia in Chromium — Overview

### What is Skia?

Skia is a 2D graphics library written in C++ that provides a common API across
multiple hardware and software backends. It handles rasterization of paths, text,
images, gradients, and effects. Chromium uses Skia as its sole 2D rendering
engine — every pixel you see in a Chrome tab was rasterized by Skia.

### Location in the Chromium Source Tree

```
chromium/src/
├── third_party/skia/           # Skia source (rolled from upstream)
│   ├── BUILD.gn                # Chromium's GN build rules for Skia
│   ├── include/                # Public Skia headers
│   │   ├── core/               #   SkCanvas, SkPaint, SkPath, SkImage, ...
│   │   ├── gpu/                #   GrDirectContext, GrBackendTexture, ...
│   │   ├── effects/            #   SkGradientShader, SkImageFilters, ...
│   │   └── codec/              #   SkCodec, SkEncodedImageFormat, ...
│   ├── src/                    # Implementation
│   │   ├── core/               #   Core rasterization pipeline
│   │   ├── gpu/                #   GPU backend (Ganesh)
│   │   │   ├── ganesh/         #     GrDirectContext implementation
│   │   │   ├── vk/             #     Vulkan backend
│   │   │   ├── gl/             #     OpenGL backend
│   │   │   └── mtl/            #     Metal backend
│   │   ├── codec/              #   Image codecs
│   │   ├── ports/              #   Platform-specific code (FreeType, FontConfig, CoreText)
│   │   └── shaders/            #   Shader implementations
│   ├── modules/                # Optional modules
│   │   ├── skshaper/           #   HarfBuzz-based text shaper
│   │   └── skunicode/          #   ICU integration
│   └── gn/                     # Upstream Skia GN helpers (partially overridden)
├── DEPS                        # Pins Skia revision (among other deps)
└── skia/                       # Chromium-side Skia config
    └── ext/                    # Chromium extensions to Skia
```

### Upstream Skia

- **Website:** <https://skia.org>
- **Source:** <https://skia.googlesource.com/skia>
- **Bug tracker:** <https://bugs.chromium.org/p/skia>

Skia is maintained by Google but is an independent project with its own release
cadence. Other consumers include Android, Flutter, Firefox (partial), and
LibreOffice.

### Rolling Mechanism

Chromium does **not** track Skia's `main` branch directly. Instead:

1. The `DEPS` file in Chromium's root pins a specific Skia commit hash:
   ```python
   # chromium/src/DEPS (excerpt)
   'src/third_party/skia':
     Var('skia_git') + '/skia.git@' + Var('skia_revision'),
   ```
2. An automated roller (`skia-autoroll@`) proposes CL updates roughly daily.
3. Each roll is tested against Chromium's full CI before landing.
4. The `skia_revision` variable resolves to a 40-character SHA from Skia's repo.

To find the exact Skia revision Chromium M147 uses:

```bash
# In a Chromium checkout at tag 147.0.7727.24
grep "'skia_revision'" DEPS
```

### For Open UI

Our sparse-checkout includes `third_party/skia/` from the M147 tag
(see `docs/adr/001-sparse-checkout-submodule.md`). This gives us the exact Skia
version Chromium ships. We then evaluate whether to use this version or upstream
Skia directly (see [Section 8](#8-open-ui-extraction-notes)).

---

## 2. Build Configuration

### Chromium's Skia Build

Chromium replaces Skia's own build with a heavily customized GN file:

```
third_party/skia/BUILD.gn          # Primary Skia build rules (Chromium-maintained)
third_party/skia/gn/               # Upstream GN helpers (some overridden)
skia/BUILD.gn                      # Chromium-side Skia extensions (skia/ext/)
```

The critical file is `third_party/skia/BUILD.gn`. Chromium maintains this file
**inside the Skia checkout** but it diverges from upstream's `BUILD.gn`. It
defines targets like `skia` (the main library), `skia_core`, `skia_effects`,
`skia_gpu`, `skia_codec`, etc.

### Key GN Args

These arguments control which Skia features Chromium enables. They are set via
`args.gn` or Chromium's build config logic in `build/config/`:

#### GPU Backends

| GN Arg | Default | Description |
|--------|---------|-------------|
| `skia_enable_gpu` | `true` | Master switch for GPU acceleration. When false, Skia only does CPU rasterization. |
| `skia_use_gl` | `true` | Enable the OpenGL/ES backend. Used on all platforms as a baseline. |
| `skia_use_vulkan` | platform-dependent | Enable Vulkan backend. Preferred on Linux and Android. Set `true` on Linux, Android; `false` on macOS, Windows (historically). |
| `skia_use_metal` | `true` on macOS/iOS | Enable Metal backend. macOS and iOS only. |
| `skia_use_dawn` | `false` (experimental) | Enable Dawn/WebGPU backend. Used for `--enable-features=SkiaGraphite` experiments. |

#### Font and Text

| GN Arg | Default | Description |
|--------|---------|-------------|
| `skia_use_freetype` | `true` on Linux/Android | Use FreeType for font rasterization (glyph outline → bitmap). |
| `skia_use_fontconfig` | `true` on Linux | Use FontConfig for system font discovery and matching. |
| `skia_use_harfbuzz` | `true` | Use HarfBuzz for OpenType text shaping (character → glyph mapping, kerning, ligatures). |
| `skia_use_icu` | `true` | Use ICU for Unicode text handling (bidi, line breaking, normalization). |

#### Image Codecs

| GN Arg | Default | Description |
|--------|---------|-------------|
| `skia_use_libpng` | `true` | PNG encode/decode via libpng. |
| `skia_use_libjpeg_turbo` | `true` | JPEG decode via libjpeg-turbo. |
| `skia_use_libwebp` | `true` | WebP encode/decode. |
| `skia_use_zlib` | `true` | zlib compression (used by PNG and other formats). |
| `skia_use_wuffs` | `true` | Wuffs for memory-safe GIF and PNG decoding. |

#### Other Notable Args

| GN Arg | Default | Description |
|--------|---------|-------------|
| `skia_enable_skottie` | `false` in Chromium | Lottie animation support (enabled in Android WebView). |
| `skia_enable_skshaper` | `true` | Text shaper module (wraps HarfBuzz). |
| `skia_enable_skunicode` | `true` | Unicode module (wraps ICU). |
| `skia_enable_pdf` | `false` in Chromium | PDF generation (Chromium uses its own `printing/` stack). |
| `skia_enable_graphite` | experimental | Next-gen GPU backend (successor to Ganesh). |
| `skia_use_perfetto` | `true` | Tracing integration with Perfetto. |
| `skia_use_piex` | `true` on Android | Raw image preview extraction. |

### Chromium's Config vs. Upstream Defaults

Upstream Skia's default configuration (what you get from a clean Skia checkout)
differs from Chromium's in several key ways:

| Area | Upstream Default | Chromium Override |
|------|-----------------|-------------------|
| Third-party deps | Fetched via `third_party/externals/` | Redirected to Chromium's `third_party/` (shared deps) |
| ICU | Upstream ICU or `SkUnicode_icu` | Chromium's `third_party/icu/` with custom data files |
| FreeType | Upstream FreeType | Chromium's `third_party/freetype/` (patched, see below) |
| PDF | Enabled | Disabled |
| Skottie | Enabled | Disabled (except Android WebView) |
| GPU backends | All enabled | Platform-selective |
| System allocator | Default | Chromium's PartitionAlloc or system malloc |

To diff the two configurations:

```bash
# Compare Chromium's Skia BUILD.gn args vs upstream defaults
# 1. In Chromium checkout:
gn args --list out/Default | grep skia_

# 2. In upstream Skia checkout:
gn args --list out/Default | grep skia_

# 3. Diff the output to see overrides
diff <(cd chromium/src && gn args --list out/Default | grep skia_ | sort) \
     <(cd upstream-skia && gn args --list out/Default | grep skia_ | sort)
```

### Third-Party Dependency Redirection

Chromium does not use Skia's `third_party/externals/` mechanism. Instead,
Skia's `BUILD.gn` is modified to reference Chromium's copies:

```
Skia references:               → Chromium provides:
skia/third_party/freetype2     → //third_party/freetype/
skia/third_party/harfbuzz      → //third_party/harfbuzz-ng/
skia/third_party/icu           → //third_party/icu/
skia/third_party/libpng        → //third_party/libpng/
skia/third_party/libjpeg-turbo → //third_party/libjpeg_turbo/
skia/third_party/libwebp       → //third_party/libwebp/
skia/third_party/zlib          → //third_party/zlib/
skia/third_party/vulkanmemoryallocator → //third_party/vulkan_memory_allocator/
```

This is controlled by GN path remapping in `third_party/skia/BUILD.gn` and
`third_party/skia/gn/` config files.

---

## 3. Chromium-Specific Patches

### Overview

Chromium sometimes carries patches on top of upstream Skia. These are changes
that live in Chromium's copy of `third_party/skia/` but have not (yet) been
upstreamed. The number of such patches is generally small (Chromium prefers to
upstream fixes), but they exist.

### Identifying Patches

```bash
# In Chromium checkout: compare the Skia directory against the pinned upstream rev
cd third_party/skia

# Find the upstream revision Chromium pins to
SKIA_REV=$(grep -oP "(?<=skia_revision': ')[a-f0-9]+" ../../DEPS)

# List Chromium-local commits on top of upstream
git log --oneline ${SKIA_REV}..HEAD

# Or diff all changes
git diff ${SKIA_REV} -- src/ include/
```

If `git log` shows commits beyond the pinned revision, those are Chromium-local
patches. The `BUILD.gn` and `gn/` directory changes are expected (Chromium
maintains its own build rules) and can be excluded:

```bash
git diff ${SKIA_REV} -- src/ include/ modules/ \
  ':!BUILD.gn' ':!gn/'
```

### Common Patch Categories

| Category | Examples |
|----------|----------|
| **GPU backend fixes** | Vulkan driver workarounds for specific GPU families (Adreno, Mali, Intel Gen9). Fence synchronization fixes. |
| **Performance optimizations** | Tiling heuristics tuned for Chromium's compositing pipeline. Raster cache tuning. |
| **Platform workarounds** | macOS CoreText interop fixes. Android NDK compatibility. Windows DirectWrite integration. |
| **Security hardening** | Bounds checking in codec paths. Fuzzer-found fixes applied before upstream release. |
| **Build/config changes** | `BUILD.gn` modifications for Chromium's dependency graph. |

### For Open UI

We need to evaluate each Chromium-local patch:

1. **GPU driver workarounds** — Likely needed if we target the same GPU hardware.
   Chromium's `gpu/config/gpu_driver_bug_list.json` drives some of these.
2. **Security fixes** — Should be picked regardless. Check if upstream has them.
3. **Performance tuning** — Evaluate case-by-case. Some are specific to
   Chromium's OOP-R pipeline and won't apply.
4. **Build changes** — We write our own `BUILD.gn`, so these don't apply directly.

**Recommendation:** Start with upstream Skia, apply security patches, and
selectively cherry-pick GPU workarounds as issues surface.

---

## 4. GPU Backend Architecture

### Skia's GPU Abstraction (Ganesh)

Skia's GPU rendering layer is called **Ganesh** (being gradually succeeded by
**Graphite**). The core abstraction is `GrDirectContext`, which owns:

- GPU resource cache (textures, buffers, render targets)
- Command buffer recording
- Pipeline state management (shaders, blend modes)
- Flush/submit logic

```
Application
    │
    ▼
SkCanvas (recording)
    │
    ▼
GrDirectContext (Ganesh)
    │
    ├── GrGLGpu          (OpenGL backend)
    ├── GrVkGpu          (Vulkan backend)
    ├── GrMtlGpu         (Metal backend)
    └── GrD3DGpu         (Direct3D 12 backend)
         │
         ▼
    GPU Driver
```

Key header files:

```
third_party/skia/include/gpu/GrDirectContext.h      # GPU context
third_party/skia/include/gpu/GrBackendSurface.h     # Backend surface/texture wrappers
third_party/skia/include/gpu/GrContextOptions.h     # Context configuration
third_party/skia/include/gpu/vk/GrVkBackendContext.h  # Vulkan backend init struct
third_party/skia/include/gpu/gl/GrGLInterface.h     # GL function pointer table
third_party/skia/include/gpu/gl/GrGLTypes.h         # GL type definitions
```

### Chromium's GPU Command Buffer

In Chromium, Skia does **not** talk to the real GPU driver directly. Instead,
Chromium interposes a **GPU command buffer** between Skia and the driver:

```
┌──────────────────────────────────────────────────────────────┐
│  Renderer Process                                             │
│                                                                │
│  Blink Paint → cc::PaintCanvas → SkCanvas (recording)         │
│                     │                                          │
│                     ▼                                          │
│  cc::RasterSource (serialized paint ops)                      │
│                     │                                          │
│                     │  IPC (shared memory / GPU channel)       │
├─────────────────────┼────────────────────────────────────────┤
│  GPU Process        │                                          │
│                     ▼                                          │
│  gpu::raster::RasterDecoderImpl                               │
│                     │                                          │
│                     ▼                                          │
│  SkCanvas (replaying into GrDirectContext)                     │
│                     │                                          │
│                     ▼                                          │
│  GrDirectContext → GrVkGpu / GrGLGpu                          │
│                     │                                          │
│                     ▼                                          │
│  Real GPU Driver (Vulkan / OpenGL)                            │
└──────────────────────────────────────────────────────────────┘
```

This is **Out-of-Process Rasterization (OOP-R)**. The renderer process records
paint operations and ships them to the GPU process, which replays them through
Skia. This provides:

- **Security isolation**: The renderer (untrusted web content) never touches the
  GPU driver directly.
- **Crash resilience**: GPU driver crashes don't take down the renderer.
- **Resource management**: The GPU process manages all GPU memory centrally.

Key files in Chromium:

```
gpu/command_buffer/                  # Command buffer infrastructure
gpu/command_buffer/service/          # GPU-process-side command execution
  raster_decoder.cc                  #   Raster command decoder
  shared_image_manager.cc            #   GPU texture/buffer sharing
gpu/command_buffer/client/           # Renderer-process-side command encoding
  raster_implementation.cc           #   Raster command encoding
gpu/ipc/                             # IPC transport for GPU commands
cc/raster/                           # Raster worker pool
  gpu_raster_buffer_provider.cc      #   OOP-R raster provider
  raster_source.cc                   #   Serialized raster operations
viz/service/display/                 # Display compositor (viz)
  skia_renderer.cc                   #   Final compositing via Skia
```

### For Open UI: Direct GPU Backend Access

Open UI skips the entire command buffer layer. We use Skia's native GPU backends
directly:

```
┌──────────────────────────────────┐
│  Open UI Application              │
│                                    │
│  C API → SkCanvas                 │
│              │                     │
│              ▼                     │
│  GrDirectContext (owned by us)    │
│              │                     │
│              ▼                     │
│  GrVkGpu / GrGLGpu               │
│              │                     │
│              ▼                     │
│  Real GPU Driver                  │
└──────────────────────────────────┘
```

This is a major simplification. We trade Chromium's security isolation for:
- Lower latency (no IPC serialization)
- Simpler architecture (no GPU process)
- Direct control over GPU resources

This is the correct trade-off for a UI toolkit (the application is trusted code,
unlike web content).

### Vulkan Backend Setup

To create a Skia GPU context backed by Vulkan:

```cpp
#include "include/gpu/GrDirectContext.h"
#include "include/gpu/vk/GrVkBackendContext.h"
#include "include/gpu/vk/VulkanExtensions.h"

// 1. Create Vulkan instance and device (standard Vulkan API)
VkInstance instance = ...;
VkPhysicalDevice physicalDevice = ...;
VkDevice device = ...;
VkQueue queue = ...;
uint32_t queueFamilyIndex = ...;

// 2. Populate Skia's Vulkan backend context
GrVkBackendContext backendContext;
backendContext.fInstance = instance;
backendContext.fPhysicalDevice = physicalDevice;
backendContext.fDevice = device;
backendContext.fQueue = queue;
backendContext.fGraphicsQueueIndex = queueFamilyIndex;
backendContext.fMaxAPIVersion = VK_API_VERSION_1_1;
backendContext.fVkExtensions = &extensions;   // skgpu::VulkanExtensions
backendContext.fGetProc = vkGetInstanceProcAddr; // or custom loader

// 3. Create the GrDirectContext
GrContextOptions options;
options.fReduceOpsTaskSplitting = GrContextOptions::Enable::kYes;
sk_sp<GrDirectContext> grContext = GrDirectContexts::MakeVulkan(backendContext, options);

// 4. Create an SkSurface from a VkImage
GrVkImageInfo imageInfo;
imageInfo.fImage = swapchainImage;
imageInfo.fImageLayout = VK_IMAGE_LAYOUT_UNDEFINED;
imageInfo.fFormat = VK_FORMAT_B8G8R8A8_UNORM;
imageInfo.fImageUsageFlags = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT |
                             VK_IMAGE_USAGE_TRANSFER_DST_BIT;
imageInfo.fLevelCount = 1;
imageInfo.fCurrentQueueFamily = queueFamilyIndex;

GrBackendRenderTarget renderTarget(width, height, imageInfo);
sk_sp<SkSurface> surface = SkSurfaces::WrapBackendRenderTarget(
    grContext.get(),
    renderTarget,
    kTopLeft_GrSurfaceOrigin,
    kBGRA_8888_SkColorType,
    SkColorSpace::MakeSRGB(),
    nullptr);

SkCanvas* canvas = surface->getCanvas();
// Draw with canvas...
```

### OpenGL/EGL Backend Setup

```cpp
#include "include/gpu/GrDirectContext.h"
#include "include/gpu/gl/GrGLInterface.h"

// 1. Create EGL context (platform-specific)
EGLDisplay display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
eglInitialize(display, nullptr, nullptr);
// ... EGL config, surface, context creation ...
eglMakeCurrent(display, surface, surface, context);

// 2. Create Skia GL interface (auto-detects function pointers)
sk_sp<const GrGLInterface> glInterface = GrGLMakeNativeInterface();

// 3. Create GrDirectContext
GrContextOptions options;
sk_sp<GrDirectContext> grContext = GrDirectContexts::MakeGL(glInterface, options);

// 4. Wrap the default framebuffer
GrGLFramebufferInfo fbInfo;
fbInfo.fFBOID = 0;  // Default framebuffer
fbInfo.fFormat = GL_RGBA8;

GrBackendRenderTarget renderTarget(width, height, sampleCount, stencilBits, fbInfo);
sk_sp<SkSurface> surface = SkSurfaces::WrapBackendRenderTarget(
    grContext.get(),
    renderTarget,
    kBottomLeft_GrSurfaceOrigin,
    kRGBA_8888_SkColorType,
    SkColorSpace::MakeSRGB(),
    nullptr);
```

---

## 5. Canvas Usage Patterns

### Chromium's Paint Abstraction Layer

Chromium does not use `SkCanvas` directly in its rendering pipeline. Instead, it
wraps Skia's canvas behind its own abstraction in the `cc/` (compositor) layer:

```
Blink (renderer)          cc/ (compositor)           Skia
─────────────────    ────────────────────────    ──────────────
GraphicsContext   →  cc::PaintCanvas            → SkCanvas
                     cc::PaintRecord            → (serialized ops)
                     cc::PaintOpBuffer          → (storage)
                     cc::PaintFlags             → SkPaint
                     cc::PaintImage             → SkImage
```

Key Chromium source files:

```
cc/paint/paint_canvas.h          # PaintCanvas interface
cc/paint/skia_paint_canvas.h     # Direct SkCanvas wrapper (GPU process)
cc/paint/record_paint_canvas.h   # Recording canvas (renderer process)
cc/paint/paint_op_buffer.h       # Paint operation storage
cc/paint/paint_op.h              # Individual paint operations (DrawRectOp, etc.)
cc/paint/paint_flags.h           # Wraps SkPaint with Chromium extensions
cc/paint/paint_record.h          # Completed recording (list of PaintOps)
cc/paint/paint_image.h           # Wraps SkImage with lazy decode
```

### Recording and Playback

In the renderer process (OOP-R), drawing is **recorded** into `PaintOpBuffer`:

```cpp
// Renderer process: recording
auto record = sk_make_sp<cc::PaintOpBuffer>();
cc::RecordPaintCanvas canvas(record.get(), bounds);
canvas.drawRect(gfx::RectFToSkRect(rect), flags);
canvas.drawRRect(SkRRect::MakeRectXY(sk_rect, rx, ry), flags);
// ... more draw calls ...
// record is serialized and sent to GPU process via IPC
```

In the GPU process, the recording is **replayed** onto a real `SkCanvas`:

```cpp
// GPU process: playback
SkCanvas* real_canvas = surface->getCanvas();
cc::SkiaPaintCanvas skia_canvas(real_canvas);
record->Playback(&skia_canvas);
```

### Common Draw Operations

These are the Skia draw calls most frequently used by Chromium's paint system:

| `SkCanvas` Method | Usage in Chromium |
|--------------------|-------------------|
| `drawRect()` | Box backgrounds, borders, outlines |
| `drawRRect()` | Rounded corners (`border-radius`) |
| `drawPath()` | SVG paths, complex clip regions, custom shapes |
| `drawTextBlob()` | All text rendering (via `SkTextBlob` from HarfBuzz shaping) |
| `drawImage()` | `<img>` elements, CSS `background-image`, decoded bitmaps |
| `drawImageRect()` | Scaled/cropped images, CSS `object-fit` |
| `drawPicture()` | Cached sub-trees (SVG, repeated patterns) |
| `drawLine()` | Underlines, strikethroughs, table borders |
| `drawOval()` | Rarely used directly (paths preferred) |
| `drawColor()` | Full-surface fills (background) |
| `clipRect()` | Overflow clipping, CSS `clip`, `overflow: hidden` |
| `clipRRect()` | Rounded overflow clipping |
| `clipPath()` | CSS `clip-path`, SVG clip |
| `save()` / `restore()` | Canvas state management around clipped/transformed regions |
| `concat()` | CSS `transform` (translate, rotate, scale, matrix) |
| `saveLayer()` | Opacity groups, blend modes, `mix-blend-mode`, `isolation: isolate` |

### Canvas State Management

Skia's canvas maintains a state stack:

```
┌─────────────────────────────────────────┐
│  Canvas State Stack                      │
│                                          │
│  save() pushes:                         │
│    • Current transform matrix (CTM)     │
│    • Current clip region                │
│                                          │
│  saveLayer() pushes all of above plus:  │
│    • Offscreen buffer for compositing   │
│    • Optional SkPaint (opacity, blend)  │
│                                          │
│  restore() pops back to previous state  │
└─────────────────────────────────────────┘
```

Chromium's pattern for rendering a DOM element:

```cpp
canvas->save();
canvas->translate(element.offset_x, element.offset_y);
canvas->clipRect(element.bounds);

// Draw background
canvas->drawRect(element.bounds, background_paint);

// Draw border (4 edges as paths or rects)
DrawBorder(canvas, element.border);

// Draw children (recursively)
for (auto& child : element.children) {
  PaintElement(canvas, child);
}

canvas->restore();
```

---

## 6. Font and Text Pipeline

Text rendering in Chromium involves a multi-stage pipeline that spans Blink,
platform font libraries, and Skia:

```
Raw text (UTF-8/16)
    │
    ▼
[1] Font Selection ──── FontConfig (Linux) / CoreText (macOS) / DirectWrite (Win)
    │                    Find font file for requested family/weight/style
    │
    ▼
[2] Font Loading ────── FreeType (Linux/Android) / CoreText / DirectWrite
    │                    Parse font file → glyph outlines, metrics
    │                    Wrapped in SkTypeface
    │
    ▼
[3] Text Shaping ────── HarfBuzz
    │                    Map characters → glyph IDs + positions
    │                    Apply OpenType features (ligatures, kerning, etc.)
    │                    Output: ShapeResult (Blink) → SkTextBlob (Skia)
    │
    ▼
[4] Glyph Rasterization ── Skia (via FreeType on Linux)
    │                       Rasterize glyph outlines to bitmaps
    │                       Cache in Skia's glyph cache (SkStrike)
    │
    ▼
[5] Drawing ──────────── SkCanvas::drawTextBlob()
                          Composite glyph bitmaps into final surface
```

### Stage 1: Font Selection (FontConfig on Linux)

FontConfig is the system font discovery mechanism on Linux. Given a font family
name and style, it locates the appropriate font file.

Chromium source files:

```
third_party/blink/renderer/platform/fonts/linux/font_cache_linux.cc
third_party/blink/renderer/platform/fonts/font_cache.h
third_party/blink/renderer/platform/fonts/font_cache.cc
third_party/blink/renderer/platform/fonts/font_description.h
third_party/blink/renderer/platform/fonts/font_fallback_list.h
third_party/blink/renderer/platform/fonts/font_fallback_list.cc
```

`FontCache` is Chromium's central font lookup cache. It maps
`(family, size, weight, style, stretch)` → `SkTypeface`. On cache miss, it
delegates to `FontConfig` (Linux) to resolve the font file path, then opens
it via FreeType/Skia.

`FontFallbackList` handles font fallback chains. When a glyph isn't found in
the primary font, it walks the fallback list (configured per-locale via
FontConfig or hardcoded in Chromium) until a font containing that glyph is
found.

### Stage 2: Font Loading (FreeType → SkTypeface)

FreeType parses font files (TrueType/OpenType) and provides glyph outline data.
Skia wraps FreeType behind its `SkTypeface` abstraction.

```
third_party/skia/src/ports/SkFontHost_FreeType.cpp     # FreeType integration
third_party/skia/src/ports/SkFontHost_FreeType_common.h
third_party/skia/src/ports/SkFontMgr_fontconfig.cpp    # FontConfig font manager
third_party/freetype/                                    # FreeType library (Chromium copy)
```

`SkTypeface` is an opaque handle to a loaded font. It provides:
- Glyph count and glyph IDs for character codes
- Glyph metrics (advance width, bounds)
- Glyph outlines (SkPath) for vector rendering
- Font tables (for HarfBuzz consumption)

### Stage 3: Text Shaping (HarfBuzz)

HarfBuzz performs the complex mapping from Unicode text to positioned glyphs.
This is where ligatures (fi → ﬁ), kerning, and complex script layout
(Arabic, Devanagari, Thai) happen.

Chromium source files:

```
third_party/blink/renderer/platform/fonts/shaping/
  harfbuzz_shaper.h                    # Main shaper entry point
  harfbuzz_shaper.cc
  shape_result.h                       # Shaping output
  shape_result.cc
  harfbuzz_face.h                      # HarfBuzz ↔ SkTypeface bridge
  harfbuzz_face.cc
third_party/harfbuzz-ng/               # HarfBuzz library (Chromium copy)
```

The shaping pipeline:

```cpp
// Simplified shaping flow
HarfBuzzShaper shaper(text, text_length);
ShapeResult* result = shaper.Shape(font, direction, script, language);

// ShapeResult contains:
// - Glyph IDs (which glyphs to draw)
// - Glyph advances (how far to move after each glyph)
// - Glyph offsets (sub-pixel positioning adjustments)
// - Cluster mapping (glyph ↔ character correspondence)

// Convert to Skia for drawing:
SkTextBlobBuilder builder;
const auto& run = builder.allocRunPos(sk_font, glyph_count);
// Fill in glyph IDs and positions from ShapeResult
sk_sp<SkTextBlob> blob = builder.make();

// Draw
canvas->drawTextBlob(blob.get(), x, y, paint);
```

### Stage 4: Glyph Caching (SkStrike)

Skia maintains a glyph cache called `SkStrike` (formerly `SkGlyphCache`).
Each strike is keyed by `(SkTypeface, size, transform, subpixel position)` and
caches:

- Rasterized glyph bitmaps (or SDF representations for GPU text)
- Glyph metrics
- Glyph paths (for large text or GPU path rendering)

```
third_party/skia/src/core/SkStrike.h
third_party/skia/src/core/SkStrike.cpp
third_party/skia/src/core/SkStrikeCache.h       # Global cache (LRU)
third_party/skia/src/core/SkStrikeCache.cpp
third_party/skia/src/core/SkScalerContext.h      # Per-font rasterization context
```

The cache is process-global and LRU-evicted. In Chromium's GPU process, the
glyph cache is populated as new text is rendered and serves subsequent frames
from cache.

### Stage 5: Emoji Rendering

Emoji use color font technologies. Chromium/Skia supports:

| Format | Description | Platform |
|--------|-------------|----------|
| **COLR/CPAL** | Vector color layers (compact, scalable) | Cross-platform (Noto Color Emoji v2) |
| **CBDT/CBLC** | Bitmap color glyphs (PNG embedded in font) | Android (Noto Color Emoji v1) |
| **sbix** | Apple bitmap color glyphs | macOS/iOS |
| **SVG** | SVG documents embedded in font | Rare (some Mozilla fonts) |
| **COLRv1** | Advanced vector color (gradients, compositing) | Modern (Noto Color Emoji v2.042+) |

Skia handles these transparently through `SkScalerContext`:

```
third_party/skia/src/core/SkScalerContext.cpp    # Dispatches to format-specific code
third_party/skia/src/ports/SkScalerContext_FreeType.cpp  # FreeType-based implementation
```

### For Open UI: Font Pipeline Extraction

The font pipeline is **the most complex extraction** in the Skia layer because
it spans multiple libraries and has deep platform integration:

```
┌──────────────────────────────────────────────────────────┐
│  What we extract from Chromium:                           │
│                                                            │
│  FontCache (simplified)                                   │
│    └── FontConfig integration (Linux)                     │
│         └── SkTypeface creation via FreeType              │
│                                                            │
│  Text shaping (HarfBuzz via SkShaper module)              │
│    └── Character → glyph mapping                          │
│    └── OpenType feature application                       │
│    └── Bidi text handling (ICU)                           │
│                                                            │
│  Exposed through C API:                                   │
│    oui_sk_font_create()      → FontConfig + FreeType      │
│    oui_sk_text_shape()       → HarfBuzz                   │
│    oui_sk_text_measure()     → SkFont metrics             │
│    oui_sk_canvas_draw_text() → SkTextBlob drawing         │
└──────────────────────────────────────────────────────────┘
```

We can use Skia's `SkShaper` module (`modules/skshaper/`) which already wraps
HarfBuzz, rather than reimplementing Blink's `HarfBuzzShaper`. This is simpler
but loses some of Chromium's advanced fallback logic. Acceptable for the POC;
revisit if text rendering quality is insufficient.

---

## 7. Image Pipeline

### Image Decoding

Chromium decodes images through Skia's codec layer, which dispatches to
format-specific decoders:

```
Encoded data (PNG, JPEG, WebP, GIF, AVIF, ...)
    │
    ▼
SkCodec::MakeFromData()           # Auto-detects format
    │
    ├── SkPngCodec                 #  → libpng (or Wuffs for PNG)
    ├── SkJpegCodec                #  → libjpeg-turbo
    ├── SkWebpCodec                #  → libwebp
    ├── SkWuffsCodec               #  → Wuffs (GIF decode)
    ├── SkHeifCodec                #  → libheif (AVIF/HEIF)
    └── SkBmpCodec, SkIcoCodec     #  → built-in
         │
         ▼
    SkBitmap / SkImage (decoded pixels)
```

Key source files:

```
third_party/skia/src/codec/SkCodec.cpp           # Codec factory
third_party/skia/src/codec/SkPngCodec.cpp         # PNG
third_party/skia/src/codec/SkJpegCodec.cpp        # JPEG
third_party/skia/src/codec/SkWebpCodec.cpp        # WebP
third_party/skia/src/codec/SkWuffsCodec.cpp       # GIF (memory-safe Wuffs decoder)
third_party/skia/include/codec/SkCodec.h          # Public API
```

### CPU vs. GPU Images

Skia provides two primary image representations:

| Type | Backing | Creation | Use Case |
|------|---------|----------|----------|
| **`SkImage` from `SkBitmap`** | CPU memory (`malloc`'d pixel buffer) | `SkImages::RasterFromBitmap(bitmap)` | Software rendering, image manipulation |
| **`SkImage` from `GrBackendTexture`** | GPU texture (VRAM) | `SkImages::BorrowTextureFrom(grContext, ...)` | GPU-accelerated rendering |

Transferring between CPU and GPU:

```cpp
// CPU → GPU: upload bitmap to texture
sk_sp<SkImage> cpuImage = SkImages::RasterFromBitmap(bitmap);
sk_sp<SkImage> gpuImage = cpuImage->makeTextureImage(grContext);

// GPU → CPU: readback (slow, avoid in hot path)
sk_sp<SkImage> cpuImage = gpuImage->makeRasterImage();
```

### Image Caching in Chromium

Chromium adds multiple caching layers on top of Skia's raw decode:

```
cc/paint/paint_image.h            # PaintImage: lazy-decoded image handle
cc/paint/decoded_draw_image.h     # Decoded image ready for GPU upload
cc/tiles/image_decode_cache.h     # Interface for decode caching
cc/tiles/gpu_image_decode_cache.h # GPU-backed decode cache
cc/tiles/software_image_decode_cache.h  # CPU-backed decode cache
```

`PaintImage` is a lazy handle — the image is only decoded when rasterization
actually needs the pixels. The decode cache stores decoded bitmaps keyed by
`(image_id, target_size, target_color_space)` so that repeated draws of the
same image skip decoding.

### Animated Images

Animated images (GIF, APNG, animated WebP) are handled through `SkCodec`'s
frame API:

```cpp
std::unique_ptr<SkCodec> codec = SkCodec::MakeFromData(data);

// Query frame count and durations
int frameCount = codec->getFrameCount();
SkCodec::FrameInfo frameInfo;
codec->getFrameInfo(0, &frameInfo);  // duration, disposal method, etc.

// Decode a specific frame
SkImageInfo info = codec->getInfo();
SkBitmap bitmap;
bitmap.allocPixels(info);

SkCodec::Options options;
options.fFrameIndex = frameIndex;
options.fPriorFrame = priorFrameIndex;  // For incremental decode
codec->getPixels(info, bitmap.getPixels(), bitmap.rowBytes(), &options);
```

Chromium manages animated image playback in:

```
third_party/blink/renderer/platform/graphics/image_animation_policy.h
cc/paint/paint_image_builder.h           # PaintImage with animation metadata
cc/paint/image_animation_count.h
```

The compositor drives animation by requesting successive frames from the decode
cache at the appropriate intervals.

---

## 8. Open UI Extraction Notes

### Strategy: Upstream Skia First

**Recommendation:** Build Open UI's Skia layer against **upstream Skia** for the
POC phase, not Chromium's vendored copy.

Rationale:

1. **Skia is designed to be standalone.** Unlike `cc/` or Blink, Skia has a
   clean build system and minimal external dependencies. Building from upstream
   is straightforward.

2. **Chromium's `BUILD.gn` is Chromium-specific.** It hardcodes dependency paths
   (`//third_party/freetype/`, etc.) that don't exist in our tree. Writing our
   own build rules for upstream Skia is less work than adapting Chromium's.

3. **Evaluate the delta later.** Once we have a working POC with upstream Skia,
   we can diff Chromium's vendored copy and cherry-pick relevant patches
   (see [Section 3](#3-chromium-specific-patches)).

4. **Pin to the same Skia revision** that Chromium M147 uses for maximum
   compatibility with later extraction phases (compositor, Blink) that may
   depend on specific Skia APIs.

### Key Simplification: No Command Buffer

The biggest architectural simplification in Open UI vs. Chromium is eliminating
the GPU command buffer and OOP-R pipeline (see [Section 4](#4-gpu-backend-architecture)).

What we skip:

| Chromium Component | Purpose | Open UI Equivalent |
|--------------------|---------|--------------------|
| `gpu::CommandBuffer` | Serialization/IPC of GPU commands | Direct Skia calls |
| `gpu::raster::RasterDecoderImpl` | Command replay in GPU process | Direct canvas API |
| `viz::SkiaRenderer` | Compositing with Skia in viz | Our compositor calls Skia directly |
| `SharedImageManager` | Cross-process texture sharing | In-process texture management |
| GPU process | Isolated process for GPU access | Single-process (or opt-in multi-process) |

### Font Pipeline Extraction is Critical

The font pipeline (Section 6) is the most tightly integrated part and requires
careful extraction:

**Must extract:**
- FontConfig integration for Linux font discovery
- FreeType for glyph rasterization (via Skia's ports)
- HarfBuzz for text shaping (via Skia's `SkShaper` module)
- ICU for Unicode handling (via Skia's `SkUnicode` module)

**Can simplify for POC:**
- Use `SkShaper` module instead of Blink's `HarfBuzzShaper`
- Use `SkFontMgr::RefDefault()` instead of Chromium's `FontCache`
- Skip Chromium's `FontFallbackList` (use Skia/FontConfig's built-in fallback)

**Must address later:**
- Custom font loading (`.ttf`/`.otf` from application bundle)
- Font fallback for emoji and CJK characters
- Subpixel text rendering configuration

### C API Design

Our C API wraps Skia's C++ API behind opaque handles. The full API surface is
documented in `docs/plan/02-skia-extraction.md`. The key design principles:

```c
// Opaque handle types — clients never see C++ internals
typedef struct OuiSkCanvas OuiSkCanvas;
typedef struct OuiSkPaint OuiSkPaint;
typedef struct OuiSkSurface OuiSkSurface;
typedef struct OuiSkGpuContext OuiSkGpuContext;
typedef struct OuiSkPath OuiSkPath;
typedef struct OuiSkFont OuiSkFont;
typedef struct OuiSkImage OuiSkImage;
typedef struct OuiSkTextBlob OuiSkTextBlob;

// Error handling via return codes (not exceptions)
typedef enum {
    OUI_SK_OK = 0,
    OUI_SK_ERROR_INVALID_ARGUMENT,
    OUI_SK_ERROR_OUT_OF_MEMORY,
    OUI_SK_ERROR_GPU_INIT_FAILED,
    OUI_SK_ERROR_DECODE_FAILED,
    // ...
} OuiSkStatus;

// Create/destroy pattern for resource management
OuiSkStatus oui_sk_surface_create_gpu(OuiSkSurface** surface, ...);
void        oui_sk_surface_destroy(OuiSkSurface* surface);

// Non-owning getters return raw pointers (lifetime tied to parent)
OuiSkCanvas* oui_sk_surface_get_canvas(OuiSkSurface* surface);
```

Implementation pattern (in `src/skia/`):

```cpp
// src/skia/surface.cc
struct OuiSkSurface {
    sk_sp<SkSurface> inner;
};

OuiSkStatus oui_sk_surface_create_gpu(OuiSkSurface** out, ...) {
    auto surface = new (std::nothrow) OuiSkSurface;
    if (!surface) return OUI_SK_ERROR_OUT_OF_MEMORY;

    surface->inner = SkSurfaces::RenderTarget(grContext, ...);
    if (!surface->inner) {
        delete surface;
        return OUI_SK_ERROR_GPU_INIT_FAILED;
    }

    *out = surface;
    return OUI_SK_OK;
}

void oui_sk_surface_destroy(OuiSkSurface* surface) {
    delete surface;  // sk_sp releases the SkSurface automatically
}

OuiSkCanvas* oui_sk_surface_get_canvas(OuiSkSurface* surface) {
    // Canvas lifetime is tied to Surface — no separate destroy needed
    return reinterpret_cast<OuiSkCanvas*>(surface->inner->getCanvas());
}
```

### Thread Safety

Skia's GPU contexts (`GrDirectContext`) are **single-threaded**. A context must
only be used from one thread at a time. This is a fundamental Skia constraint
that our C API must document and enforce:

```
┌────────────────────────────────────────────────────────────────┐
│  Thread Safety Rules for Open UI Skia C API                     │
│                                                                  │
│  1. OuiSkGpuContext is NOT thread-safe.                         │
│     - Create and use from a single thread (the "render thread") │
│     - All OuiSkSurface and OuiSkCanvas calls using that context │
│       must happen on the same thread                            │
│                                                                  │
│  2. OuiSkPaint, OuiSkPath, OuiSkFont are thread-safe for reads │
│     - Create on any thread                                      │
│     - Read from multiple threads concurrently                   │
│     - Mutate from one thread at a time (no concurrent mutation) │
│                                                                  │
│  3. SkImage (OuiSkImage) is thread-safe (immutable after create)│
│     - Decode on any thread, draw on render thread               │
│     - Internally refcounted                                     │
│                                                                  │
│  4. CPU-only operations (raster surface, image decode) can run  │
│     on any thread independently                                 │
└────────────────────────────────────────────────────────────────┘
```

Chromium handles this by funneling all GPU work through the GPU process's main
thread (or a dedicated GPU thread). In Open UI, the application is responsible
for thread discipline — our documentation and API headers must make this clear.

### Extraction Checklist

| Item | Difficulty | Notes |
|------|-----------|-------|
| Skia core (rasterizer, canvas, paint, path) | Low | Standalone, clean build |
| GPU backend (Vulkan) | Low | Direct `GrDirectContext` usage |
| GPU backend (OpenGL/EGL) | Low | Direct `GrGLInterface` usage |
| Image codecs (PNG, JPEG, WebP) | Low | Self-contained in `src/codec/` |
| FreeType font rasterization | Low | Skia port, standard dep |
| FontConfig font discovery | Medium | Platform integration, config files |
| HarfBuzz text shaping | Medium | Via `SkShaper` module |
| ICU Unicode support | Medium | Large data tables, via `SkUnicode` module |
| Emoji (color fonts) | Medium | Multiple formats, font-dependent |
| Animated image playback | Medium | State machine, timer integration |
| C API wrapper | Medium | Large surface area, lifetime management |
| Chromium GPU patches | Low | Cherry-pick as needed |

---

## References

- Skia documentation: <https://skia.org/docs/>
- Skia GPU overview: <https://skia.org/docs/user/gpu/>
- Chromium graphics architecture: <https://chromium.googlesource.com/chromium/src/+/main/docs/gpu/>
- Chromium compositing: <https://chromium.googlesource.com/chromium/src/+/main/docs/how_cc_works.md>
- Open UI Skia extraction plan: `docs/plan/02-skia-extraction.md`
- Open UI research plan: `docs/plan/01-research-and-infrastructure.md`
- Chromium version pin: `CHROMIUM_VERSION` (M147 / `147.0.7727.24`)
