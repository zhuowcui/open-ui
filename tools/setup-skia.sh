#!/usr/bin/env bash
# tools/setup-skia.sh — Set up Skia for standalone building
#
# Usage:
#   ./tools/setup-skia.sh [--chromium-clang /path/to/chromium/src]
#
# This script:
#   1. Clones upstream Skia at the pinned commit (if not already present)
#   2. Syncs Skia's third-party dependencies
#   3. Configures and builds libskia.a
#
# Requirements:
#   - git, python3, ninja (or path to ninja binary)
#   - clang/clang++ (system, or specify --chromium-clang to use Chromium's)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SKIA_DIR="$REPO_ROOT/third_party/skia"
SKIA_COMMIT="3d014aec545ac64c9afd304b84a20e0b95ac8607"
SKIA_URL="https://skia.googlesource.com/skia.git"

CHROMIUM_SRC=""
BUILD_TYPE="Release"
JOBS=8

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --chromium-clang PATH   Path to Chromium src/ (uses its clang + sysroot)"
    echo "  --jobs N                Parallel build jobs (default: 8)"
    echo "  --debug                 Build in debug mode"
    echo "  --help                  Show this help"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --chromium-clang)
            CHROMIUM_SRC="$2"
            shift 2
            ;;
        --jobs)
            JOBS="$2"
            shift 2
            ;;
        --debug)
            BUILD_TYPE="Debug"
            shift
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

echo "=== Open UI Skia Setup ==="
echo "Skia dir:  $SKIA_DIR"
echo "Commit:    $SKIA_COMMIT"
echo "Build:     $BUILD_TYPE"

# Step 1: Clone Skia if not present
if [ ! -d "$SKIA_DIR/.git" ]; then
    echo ""
    echo "--- Cloning Skia ---"
    mkdir -p "$(dirname "$SKIA_DIR")"
    git clone --depth=1 "$SKIA_URL" "$SKIA_DIR"
    cd "$SKIA_DIR"
    git fetch --depth=1 origin "$SKIA_COMMIT"
    git checkout "$SKIA_COMMIT"
else
    echo ""
    echo "--- Skia already cloned ---"
    cd "$SKIA_DIR"
    CURRENT=$(git rev-parse HEAD)
    if [ "$CURRENT" != "$SKIA_COMMIT" ]; then
        echo "WARNING: Skia is at $CURRENT, expected $SKIA_COMMIT"
        echo "Run: cd $SKIA_DIR && git fetch --depth=1 origin $SKIA_COMMIT && git checkout $SKIA_COMMIT"
    fi
fi

# Step 2: Sync dependencies
echo ""
echo "--- Syncing Skia dependencies ---"
cd "$SKIA_DIR"
python3 tools/git-sync-deps

# Step 3: Find build tools
echo ""
echo "--- Configuring build ---"

# Find ninja
NINJA=""
if command -v ninja &>/dev/null; then
    NINJA="ninja"
elif [ -n "$CHROMIUM_SRC" ] && [ -x "$CHROMIUM_SRC/third_party/ninja/ninja" ]; then
    NINJA="$CHROMIUM_SRC/third_party/ninja/ninja"
else
    echo "ERROR: ninja not found. Install ninja-build or use --chromium-clang."
    exit 1
fi

# Determine compiler flags
EXTRA_CFLAGS=""
EXTRA_LDFLAGS=""
CC_ARG=""
CXX_ARG=""
AR_ARG=""

if [ -n "$CHROMIUM_SRC" ]; then
    CLANG_DIR="$CHROMIUM_SRC/third_party/llvm-build/Release+Asserts/bin"
    SYSROOT="$CHROMIUM_SRC/build/linux/debian_bullseye_amd64-sysroot"

    if [ ! -x "$CLANG_DIR/clang++" ]; then
        echo "ERROR: Chromium clang not found at $CLANG_DIR"
        exit 1
    fi
    if [ ! -d "$SYSROOT" ]; then
        echo "ERROR: Chromium sysroot not found at $SYSROOT"
        exit 1
    fi

    CC_ARG="cc = \"$CLANG_DIR/clang\""
    CXX_ARG="cxx = \"$CLANG_DIR/clang++\""
    AR_ARG="ar = \"$CLANG_DIR/llvm-ar\""
    EXTRA_CFLAGS="extra_cflags = [\"--sysroot=$SYSROOT\", \"-Wno-gcc-install-dir-libstdcxx\"]"
    EXTRA_LDFLAGS="extra_ldflags = [\"--sysroot=$SYSROOT\"]"
    echo "Using Chromium clang: $CLANG_DIR/clang++"
    echo "Using sysroot: $SYSROOT"
else
    echo "Using system compiler"
fi

IS_DEBUG="false"
IS_OFFICIAL="true"
if [ "$BUILD_TYPE" = "Debug" ]; then
    IS_DEBUG="true"
    IS_OFFICIAL="false"
fi

OUT_DIR="out/$BUILD_TYPE"

# Generate GN args
cd "$SKIA_DIR"
bin/gn gen "$OUT_DIR" --args="
  is_official_build = $IS_OFFICIAL
  is_debug = $IS_DEBUG

  $CC_ARG
  $CXX_ARG
  $AR_ARG
  $EXTRA_CFLAGS
  $EXTRA_LDFLAGS

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

  skia_use_system_freetype2 = false
  skia_use_system_harfbuzz = false
  skia_use_system_icu = false
  skia_use_system_libjpeg_turbo = false
  skia_use_system_libpng = false
  skia_use_system_libwebp = false
  skia_use_system_zlib = false
"

# Step 4: Build
echo ""
echo "--- Building Skia ($BUILD_TYPE) ---"
"$NINJA" -C "$OUT_DIR" -j"$JOBS"

echo ""
echo "=== Skia build complete ==="
echo "Library: $SKIA_DIR/$OUT_DIR/libskia.a"
ls -lh "$OUT_DIR/libskia.a"
