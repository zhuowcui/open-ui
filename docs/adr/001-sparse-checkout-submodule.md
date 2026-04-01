# ADR 001: Sparse-Checkout Git Submodule for Chromium Sources

## Status

Accepted

## Context

Open UI extracts components from the Chromium source tree (~30GB full checkout). We need these sources available at build time but don't want to:
- Vendor the entire source into our repo (massive bloat)
- Require developers to maintain a separate full Chromium checkout
- Pull 30GB+ of irrelevant browser code

Options considered:
1. **Full vendor**: Copy needed source files into `third_party/chromium/`. Full control, but ~5-10GB in repo, manual sync on updates.
2. **Full git submodule**: Point at Chromium's git repo. Simple setup, but requires fetching 30GB+.
3. **Sparse-checkout git submodule**: Submodule with sparse-checkout configured to pull only needed directories (~2-3GB).

## Decision

Use a **sparse-checkout git submodule** pointing at `https://chromium.googlesource.com/chromium/src.git` with sparse-checkout configured for approximately 15 directories:

```
base/
build/
cc/
gpu/
ui/gfx/
viz/
third_party/skia/
third_party/blink/renderer/core/layout/
third_party/blink/renderer/core/css/
third_party/blink/renderer/core/style/
third_party/blink/renderer/core/paint/
third_party/blink/renderer/platform/fonts/
third_party/blink/renderer/platform/graphics/
third_party/icu/
third_party/harfbuzz-ng/
third_party/freetype/
third_party/fontconfig/
third_party/zlib/
third_party/libpng/
third_party/libjpeg_turbo/
third_party/libwebp/
third_party/abseil-cpp/
testing/gtest/
```

Pinned to tag `147.0.7727.24` (Chromium M147).

## Consequences

**Benefits:**
- Tracks upstream Chromium — easy to update the pin version
- Only ~2-3GB instead of 30GB+ 
- Standard git workflow (submodule update, version pinning)
- CI can checkout with `--recursive`

**Drawbacks:**
- Sparse-checkout configuration must be maintained as we discover new needed directories
- First clone is slower than a vendored approach (fetching from Chromium's repo)
- Git sparse-checkout behavior can be finicky with deep nested paths
- Developers need git 2.25+ for sparse-checkout support

**Mitigation:**
- Document sparse-checkout setup clearly in README and CONTRIBUTING
- Provide a setup script (`tools/setup-submodule.sh`) 
- CI validates that the sparse-checkout contains everything we need
