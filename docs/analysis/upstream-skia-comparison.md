# Upstream Skia vs Chromium's Skia Fork — Comparison

## Summary

Chromium does **not maintain a fork** of Skia. Instead, it pins a specific commit
from the upstream Skia repository (`skia.googlesource.com/skia`) and applies
configuration overlays via `skia/` (separate from `third_party/skia/`).

**Recommendation for Open UI:** Start with Chromium's Skia configuration for the
initial extraction (SP2), since it's battle-tested and we need its memory/logging
integration. We can strip the Chromium overlay (`skia/ext/`, `SkMemory_new_handler`)
and replace with our own simpler shims during SP2 Phase B.

---

## Version Pin

| Property | Value |
|---|---|
| Upstream source | `https://skia.googlesource.com/skia.git` |
| Chromium pin | `3d014aec545ac64c9afd304b84a20e0b95ac8607` |
| Chromium version | M147 (147.0.7727.24) |
| Pin location | `DEPS` file, `'skia_revision'` |

## Architecture: Two Directories

Chromium's Skia setup uses two directories:

1. **`third_party/skia/`** — Upstream Skia source (unmodified).
   Managed by `gclient` as a separate git repository.
   Contains Skia's own `BUILD.gn`, `gn/`, `include/`, `src/`, etc.

2. **`skia/`** — Chromium's overlay.
   Contains Chromium-specific configuration, extensions, and build rules.
   This is NOT a fork — it's an overlay that wraps upstream Skia.

## Chromium Overlay (`skia/`) Contents

### Build System: `skia/BUILD.gn` (1,306 lines)

The Chromium overlay BUILD.gn:
- Imports upstream Skia's `gn/shared_sources.gni` for source file lists
- Adds ~89 Chromium-specific extension files from `skia/ext/`
- Replaces Skia's default memory allocator with `SkMemory_new_handler.cpp`
  (routes through `base::UncheckedMalloc` for Chromium's memory management)
- Configures platform-specific sources (iOS, macOS, Windows, Linux)
- Sets up Rust FFI bridges for PNG, BMP, EXIF, ICC, and font handling
- Defines codec availability (PNG via Rust, JPEG, WebP, BMP, WBMP, GIF, ICO)
- Integrates with Chromium's component build system (`SK_API` → `COMPONENT_EXPORT`)

### Configuration: `skia/config/SkUserConfig.h` (170 lines)

Key customizations:
- **Debug mode:** Maps `DCHECK_ALWAYS_ON` → `SK_DEBUG`
- **Font PDF subset:** `SK_PDF_USE_HARFBUZZ_SUBSET`
- **Canvas save prealloc:** `SK_CANVAS_SAVE_RESTORE_PREALLOC_COUNT = 16`
- **Export macro:** `SK_API` → `COMPONENT_EXPORT(SKIA)` for shared library builds
- **Ref counting:** Custom debug/release ref count tracking via `sk_ref_cnt_ext_*.h`
- **Logging redirect:** `SkDebugf` → routes through `__FILE__`/`__LINE__`
- **Memory allocation:** Custom `sk_malloc_flags`, `sk_free` via base::UncheckedMalloc
- **Histogram integration:** `SK_HISTOGRAM_*` macros → `base::UmaHistogram*`

### Extensions: `skia/ext/` (89 files)

| Extension | Purpose |
|---|---|
| `SkMemory_new_handler.cpp` | Replaces sk_malloc → base::UncheckedMalloc |
| `skia_histogram.h/.cc` | Routes SK_HISTOGRAM to Chrome's UMA |
| `image_operations.cc/.h` | High-quality image resize (Lanczos, Mitchell) |
| `convolver*.cc/.h` | SSE2/NEON/LSX optimized image convolution |
| `platform_canvas.cc/.h` | Platform-specific canvas creation |
| `skia_utils_base.cc/.h` | Serialization helpers for IPC |
| `skia_memory_dump_provider.*` | Memory dump integration |
| `skia_trace_memory_dump_impl.*` | Tracing memory dump |
| `benchmarking_canvas.*` | Rendering benchmarking support |
| `cicp.cc/.h` | CICP color space handling |
| `codec_utils.cc/.h` | Codec utilities |
| `draw_gainmap_image.*` | HDR gainmap rendering |
| `rgba_to_yuva.*` | Color space conversion |
| `recursive_gaussian_convolution.*` | Gaussian blur optimization |

### Feature flags: `skia/features.gni`

Defines build-time feature toggles for Skia configuration within Chromium.

## Key Differences from Upstream Standalone Build

| Aspect | Upstream Skia | Chromium's Skia |
|---|---|---|
| **Build system** | `gn` + `third_party/externals/` deps | Chromium's `gn` with `gclient` deps |
| **Memory allocator** | `malloc`/`free` (standard) | `base::UncheckedMalloc`/`Free` (PartitionAlloc) |
| **PNG codec** | libpng (C) | Rust `png` crate via CXX bridge |
| **Font handling** | FreeType + HarfBuzz (standalone builds) | Chromium's FreeType + HarfBuzz + Fontations (Rust) |
| **Logging** | `SkDebugf` → fprintf | `SkDebugf` → base::logging (with file/line) |
| **Histograms** | No-op | Routes to Chrome UMA |
| **Export symbols** | `SK_API` = default visibility | `SK_API` = COMPONENT_EXPORT |
| **C++ stdlib** | System libc++ or libstdc++ | Chromium's bundled libc++ |
| **GPU backends** | GL, Vulkan, Metal, Dawn (all optional) | GL, Vulkan, Dawn (Metal on macOS) |
| **base/ dependency** | **NONE** (zero includes) | ~5 headers (memory, debug, logging, histogram) |

## Impact on Open UI Extraction (SP2)

### What we keep from Chromium's overlay:
1. **Build configuration** — The codec, font, and GPU backend choices are well-tested
2. **Font integration** — Fontations (Rust) + FreeType + HarfBuzz is the modern path
3. **Image codecs** — Rust PNG encoder is the future direction

### What we replace:
1. **`SkMemory_new_handler.cpp`** — Replace with standard malloc/free shim
   (eliminates `base::UncheckedMalloc` dependency)
2. **`skia_histogram.h`** — Replace with no-op or simple counters
   (eliminates `base::UmaHistogram` dependency)
3. **`SkUserConfig.h`** — Simplify: remove `COMPONENT_EXPORT`, `base/` includes
4. **Logging** — Replace `SkDebugf` with simple stderr or callback

### Estimated delta:
- **5 files to replace** (memory, histogram, logging, config, ref counting)
- **~200 lines of shim code** to eliminate all base/ dependencies from Skia
- This confirms our earlier analysis: Skia has ZERO `#include "base/..."` in its
  own source; ALL base/ dependencies come from Chromium's overlay layer

## Building Upstream Skia Standalone

For reference, upstream Skia can be built standalone:

```bash
git clone https://skia.googlesource.com/skia.git
cd skia
python3 tools/git-sync-deps
# For raster-only:
bin/gn gen out/Release --args='is_debug=false skia_use_gl=false skia_use_vulkan=false'
ninja -C out/Release
```

This produces a standalone `libskia.a` without any Chromium dependencies.
The trade-off: no Fontations (Rust fonts), no Rust PNG, no PartitionAlloc.

## Conclusion

Chromium's Skia is upstream Skia + a thin overlay. The overlay adds ~89 files
(mostly `skia/ext/`) and customizes memory allocation, logging, and histograms
to integrate with Chromium's base/ library. The upstream Skia source itself has
zero Chromium dependencies.

For Open UI, we should:
1. **Use upstream Skia directly** as our base (no Chromium overlay)
2. **Write our own slim shims** for `sk_malloc`, `SkDebugf`, and histograms
3. **Optionally adopt** Chromium's Fontations/Rust PNG if we want those features
4. This eliminates the base/ dependency entirely for the Skia layer
