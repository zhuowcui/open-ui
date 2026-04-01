# Sub-Project 2: Skia Extraction & C API

> Build upstream Skia standalone, wrap it in a stable C API, and render to a Linux
> window — the foundation every other layer paints into.

## Objective

Produce `libopenui_skia.so` — a standalone shared library that exposes Skia's 2D
graphics capabilities through a stable C ABI. Zero Chromium dependencies. Any language
that can call C functions can use it to render text, shapes, images, gradients, and
effects at 60fps via GPU (Vulkan/GL) or CPU.

## Resolved Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Skia source | **Upstream Skia** (same commit Chromium pins: `3d014ae`) | Zero base/ deps, clean standalone build, proven by SP1 analysis |
| Chromium overlay | **Not used** — write our own slim shims | Eliminates all Chromium dependencies (~200 LOC of shims) |
| GPU backends | **Vulkan** (primary) + **OpenGL/EGL** (fallback) + **CPU raster** (always) | Vulkan = modern + performant; GL = wider compat; CPU = headless/CI |
| Windowing | **SDL3** as temporary shim | Proven X11/Wayland/GL/Vulkan surface; replaced with custom in SP6 |
| Font stack | **FreeType** + **HarfBuzz** + **FontConfig** + **ICU** | Standard Linux font stack; Fontations (Rust) deferred |
| Build approach | **Skia's own GN** for libskia.a, then our GN for C wrapper | Least friction — Skia's build already works standalone |

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Application Code (C)                       │
│          #include <openui/openui_skia.h>                     │
├──────────────────────────────────────────────────────────────┤
│                  C API Wrapper Layer                          │
│   src/skia/oui_sk_*.cc — thin C wrappers over C++ API       │
│   Opaque handles, create/destroy lifecycle, error codes      │
├──────────────────────────────────────────────────────────────┤
│                    Open UI Shims                              │
│   src/skia/shims/ — SkMemory, SkDebugf, Histogram (no-ops)  │
├──────────────────────────────────────────────────────────────┤
│                   Upstream Skia (libskia.a)                   │
│   third_party/skia/ — built standalone via Skia's own GN     │
│   Raster + Vulkan + GL, FreeType, HarfBuzz, ICU, codecs     │
├──────────────────────────────────────────────────────────────┤
│                  SDL3 (temporary windowing)                   │
│   Window creation, GL/Vulkan surface, event loop             │
└──────────────────────────────────────────────────────────────┘
```

## Skia Source & Dependencies

Skia at commit `3d014ae` (same as Chromium M147 pin) with upstream's own GN build.

**Skia GN configuration for our build:**
```
is_official_build = true
is_debug = false
skia_use_gl = true
skia_use_egl = true
skia_use_vulkan = true
skia_use_x11 = true
skia_use_freetype = true
skia_use_harfbuzz = true
skia_use_fontconfig = true
skia_use_icu = true
skia_use_libjpeg_turbo_decode = true
skia_use_libjpeg_turbo_encode = true
skia_use_libpng_decode = true
skia_use_libpng_encode = true
skia_use_libwebp_decode = true
skia_use_libwebp_encode = true
skia_use_wuffs = true
skia_use_zlib = true
skia_enable_ganesh = true
skia_enable_graphite = false
skia_enable_pdf = false
skia_enable_svg = false
skia_enable_skottie = false
skia_use_perfetto = false
```

**Dependencies fetched by `tools/git-sync-deps`:**
- freetype, harfbuzz, icu, libjpeg-turbo, libpng, libwebp, zlib
- vulkan-headers, vulkan-tools, spirv-tools, spirv-cross, glslang
- highway (SIMD), brotli (compression), expat (XML)
- wuffs (GIF/BMP decoders)

---

## Task Breakdown

### Phase A: Skia Standalone Build

Get upstream Skia building completely outside Chromium's tree.

| Task | Description |
|---|---|
| SP2-A1 | **Skia source setup** — Clone/checkout upstream Skia at pinned commit `3d014ae` into `third_party/skia/`. Set up `tools/git-sync-deps` to fetch dependencies. Create `tools/setup-skia.sh` automation script. |
| SP2-A2 | **Skia standalone GN build** — Configure GN args for our target profile (Vulkan + GL + raster, FreeType, HarfBuzz, ICU, all image codecs). Build `libskia.a` with `ninja`. Verify build produces a working static library. |
| SP2-A3 | **Verify standalone build** — Write a minimal C++ test (`tests/skia/skia_standalone_test.cc`) that links against the standalone `libskia.a` and renders shapes to a PNG, proving the library works without any Chromium code. Compare output to SP1 POC. |
| SP2-A4 | **Shim layer** — Write Open UI shims to replace Chromium's overlay: `src/skia/shims/oui_sk_memory.cc` (standard malloc/free), `src/skia/shims/oui_sk_debug.cc` (stderr + callback logging), `src/skia/shims/oui_sk_histogram.cc` (no-op counters). Write `src/skia/shims/OuiSkUserConfig.h`. Verify Skia links and runs with our shims instead of Chromium's. |
| SP2-A5 | **Integration into our GN build** — Set up our `BUILD.gn` to invoke Skia's standalone build as a dependency (via `exec_script` or pre-build step), then link the resulting `libskia.a` into our targets. Update `build/config/` with Skia include paths and link flags. |

### Phase B: C API Design & Core Implementation

Design the stable C ABI and implement wrappers for Skia's core drawing API.

| Task | Description |
|---|---|
| SP2-B1 | **C API header design** — Write `include/openui/openui_skia.h` with the full public C API. Handle-based opaque types, create/destroy lifecycle, `OuiSkStatus` error codes, thread-safety documentation. Get the API surface right before implementing. |
| SP2-B2 | **Surface & context wrappers** — Implement `oui_sk_surface_create_raster()`, `oui_sk_surface_create_gpu()`, `oui_sk_gpu_context_create()`, `oui_sk_surface_get_canvas()`, `oui_sk_surface_destroy()`. These wrap `SkSurface`, `GrDirectContext` (Ganesh). |
| SP2-B3 | **Canvas wrappers** — Implement all canvas operations: `save/restore`, `translate/scale/rotate/concat_matrix`, `clip_rect/clip_path`, `clear`, `draw_rect/draw_rrect/draw_circle/draw_oval/draw_path/draw_line`, `draw_image/draw_image_rect`. |
| SP2-B4 | **Paint wrappers** — Implement `OuiSkPaint` create/destroy, color, style (fill/stroke/fill_and_stroke), stroke width, anti-alias, blend mode, alpha. Shader and image filter attachment. |
| SP2-B5 | **Path wrappers** — Implement `OuiSkPath` create/destroy, `move_to/line_to/quad_to/cubic_to/arc_to/close`. Use `SkPathBuilder` internally (Chromium's Skia removed `SkPath::moveTo` etc.). Also: path from SVG string, path bounds, path contains point. |
| SP2-B6 | **Geometry types** — Implement value types: `OuiSkRect`, `OuiSkRRect`, `OuiSkPoint`, `OuiSkSize`, `OuiSkMatrix`, `OuiSkColor` (RGBA8888 + float variants), `OuiSkColorSpace`. |

### Phase C: Text, Image & Effects

Wrap Skia's text rendering, image handling, and visual effects.

| Task | Description |
|---|---|
| SP2-C1 | **Font & text wrappers** — Implement `OuiSkFont` (create from family+size+style, via `SkFontMgr` + FreeType/FontConfig), `OuiSkTypeface`, font metrics. `oui_sk_canvas_draw_text()` for simple text, `oui_sk_canvas_draw_text_blob()` for shaped text. |
| SP2-C2 | **Text shaping** — Implement `oui_sk_text_shape()` using HarfBuzz via Skia's `SkShaper`. Supports complex scripts (Arabic RTL, CJK, Devanagari, emoji). Returns `OuiSkTextBlob` with positioned glyphs. Paragraph layout with line breaking for multi-line text. |
| SP2-C3 | **Image wrappers** — Implement `oui_sk_image_decode()` (from memory buffer), `oui_sk_image_load_file()` (from path), `oui_sk_image_encode_png/jpeg/webp()`, `oui_sk_image_width/height()`, `oui_sk_image_destroy()`. All codecs: PNG, JPEG, WebP, GIF, BMP. |
| SP2-C4 | **Shader wrappers** — Implement gradient shaders: `oui_sk_shader_linear_gradient()`, `oui_sk_shader_radial_gradient()`, `oui_sk_shader_sweep_gradient()`. Image shader (tiled images). Color filter shader. Shader composition. |
| SP2-C5 | **Image filter wrappers** — Implement `oui_sk_image_filter_blur()`, `oui_sk_image_filter_drop_shadow()`, `oui_sk_image_filter_color_filter()`, `oui_sk_image_filter_compose()`. These are critical for CSS box-shadow, blur, etc. |
| SP2-C6 | **Mask filter & color filter wrappers** — Implement `oui_sk_mask_filter_blur()` (for text shadow), color matrix filter, table color filter, blend mode color filter. |

### Phase D: GPU & Windowing

Render to a real window with GPU acceleration.

| Task | Description |
|---|---|
| SP2-D1 | **SDL3 integration** — Add SDL3 as a dependency (either vendored or system). Create `src/platform/sdl_window.cc` that creates an SDL window with GL or Vulkan context. Wrap in our `oui_window_*` API. |
| SP2-D2 | **OpenGL backend** — Create a Skia `GrDirectContext` from SDL's GL context. Create `SkSurface` bound to the GL framebuffer. Implement present (swap buffers). Verify rendering in a window. |
| SP2-D3 | **Vulkan backend** — Create a Skia `GrDirectContext` from SDL's Vulkan instance/device. Create `SkSurface` bound to the Vulkan swap chain. Implement present. Verify rendering. Handle device lost/recreate. |
| SP2-D4 | **Backend selection & fallback** — Auto-detect available backends: try Vulkan first, fall back to GL, fall back to CPU raster. `OuiSkBackend` enum + `oui_sk_gpu_context_create()` with backend preference. |
| SP2-D5 | **Event loop integration** — Poll SDL events, translate to `OuiEvent` (mouse move/click, keyboard, resize, close). Basic event loop: `while (oui_window_poll_event(...)) { render(); oui_window_present(); }`. VSync support. |
| SP2-D6 | **Resize & DPI handling** — Handle window resize (recreate surface), DPI scale factor detection (HiDPI), coordinate scaling. |

### Phase E: Shared Library Build

Package everything into `libopenui_skia.so`.

| Task | Description |
|---|---|
| SP2-E1 | **Shared library target** — Create GN target that links all C wrapper code + shims + `libskia.a` into `libopenui_skia.so`. Symbol visibility: export only `oui_sk_*` and `oui_window_*` symbols, hide all Skia C++ internals. Use version script or `-fvisibility=hidden` + `__attribute__((visibility("default")))`. |
| SP2-E2 | **Header packaging** — Ensure `include/openui/openui_skia.h` is self-contained (no C++ headers, no Skia headers leaked). Package with `pkg-config` file for easy integration. |
| SP2-E3 | **ABI stability verification** — Generate ABI dump with `abidiff` or similar. Document ABI stability policy. Verify no C++ symbols leak through the C API. Test that a C-only program can link and run against the .so. |

### Phase F: Validation & Benchmarks

| Task | Description |
|---|---|
| SP2-F1 | **Render test suite** — Pixel-level tests for: primitives (rect, rrect, circle, oval, path, line), text (Latin, CJK, Arabic RTL, emoji), images (decode+draw all formats), gradients (linear, radial, sweep), clipping (rect, rrect, path), blend modes (all), transforms (translate, rotate, scale, perspective), filters (blur, shadow, color). Compare output PNGs to reference images with tolerance threshold. |
| SP2-F2 | **Hello world example** — `examples/skia_hello.c`: window with background, colored shapes, styled text, an image. Pure C, no C++. Demonstrates the core C API. |
| SP2-F3 | **Rendering gallery example** — `examples/skia_gallery.c`: comprehensive showcase of all API features. Multiple pages/scenes. Interactive (keyboard to switch scenes). |
| SP2-F4 | **Performance benchmarks** — Measure: draw 10k rects, draw 1k text strings, decode+draw 100 images, gradient-filled complex paths. Compare C API overhead vs direct Skia C++ (target: < 1%). Measure frame times for gallery scene (target: < 5ms at 1080p). |
| SP2-F5 | **Memory safety** — Run all tests under ASan + LSan. Run C API usage under Valgrind. Fuzz the C API with random inputs (invalid handles, null pointers, huge sizes). Verify zero leaks, zero UB. |
| SP2-F6 | **API documentation** — Document every function in `openui_skia.h` with Doxygen comments: parameters, return values, thread safety, lifecycle, error conditions. Generate HTML docs. |

---

## C API Summary

```
Surface/Context:  6 functions  (create, destroy, get_canvas, present)
Canvas:          20 functions  (save/restore, transform, clip, draw_*)
Paint:           12 functions  (create, destroy, set_*)
Path:            10 functions  (create, destroy, move/line/quad/cubic/arc/close)
Geometry:         8 functions  (rect, rrect, point, size, matrix, color)
Font/Text:        8 functions  (create, destroy, measure, shape, draw)
Image:            6 functions  (decode, load, encode, width, height, destroy)
Shader:           4 functions  (linear/radial/sweep gradient, image)
Filter:           5 functions  (blur, shadow, color, compose, destroy)
Window:           5 functions  (create, get_surface, present, poll_event, destroy)
─────────────────────────────────
Total:          ~84 functions
```

All functions prefixed with `oui_sk_` (Skia) or `oui_window_` (windowing).

---

## Risk Mitigation

| Risk | Mitigation |
|---|---|
| Skia standalone build has missing deps | Pin exact commit, use `git-sync-deps`, automate in setup script |
| Vulkan not available on CI/dev machines | Always have GL + CPU raster fallback; CI uses CPU raster |
| C API doesn't cover enough Skia features | Start with SP7 widget requirements list; add APIs as needed |
| Text shaping complexity (ICU, HarfBuzz) | Leverage Skia's built-in `SkShaper` which already integrates them |
| Shared lib size too large | Strip symbols, LTO, `-Oz`; target < 15MB for .so |
| ABI breaks between versions | Opaque handles only; no structs with inline fields in ABI |

## Deliverables

| Deliverable | Description |
|---|---|
| `third_party/skia/` | Upstream Skia source at pinned commit |
| `tools/setup-skia.sh` | Automated Skia checkout + dependency fetch + build script |
| `src/skia/shims/` | Memory, logging, histogram shims (~200 LOC) |
| `src/skia/oui_sk_*.cc` | C API wrapper implementation (~2,000 LOC) |
| `src/platform/sdl_window.cc` | SDL3 windowing integration |
| `include/openui/openui_skia.h` | Public C API header (~400 LOC) |
| `libopenui_skia.so` | Shared library with C ABI |
| `examples/skia_hello.c` | Hello world window app (pure C) |
| `examples/skia_gallery.c` | Feature gallery app (pure C) |
| `tests/skia/` | Render test suite with reference images |
| `benchmarks/skia/` | Performance benchmarks |
| `docs/api/skia.md` | API documentation |

## Success Criteria

- [ ] Pure C program renders colored text + shapes to a Linux window via `libopenui_skia.so`
- [ ] Vulkan, OpenGL, and CPU raster backends all work
- [ ] Text shaping handles Latin, CJK, Arabic (RTL), and emoji correctly
- [ ] All image codecs work (PNG, JPEG, WebP, GIF, BMP decode; PNG/JPEG/WebP encode)
- [ ] C API overhead vs direct Skia C++ < 1% on benchmarks
- [ ] Zero memory leaks (ASan + Valgrind clean)
- [ ] Shared library < 15MB stripped
- [ ] `openui_skia.h` is self-contained — no C++ or Skia headers required
- [ ] ABI contains only `oui_sk_*` and `oui_window_*` symbols

## Dependencies

- **SP1** (complete) — build system, Chromium checkout (for reference), analysis docs
