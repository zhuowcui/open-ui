#!/bin/bash
# tools/build-openui-skia.sh — Build libopenui_skia.so and examples
#
# Prerequisites:
#   - Run tools/setup-skia.sh first to build third_party/skia/
#   - Chromium checkout at $CHROMIUM_SRC (for clang + sysroot)
#
# Usage:
#   ./tools/build-openui-skia.sh [--test] [--clean]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

# ── Configuration ─────────────────────────────────────────────────

CHROMIUM_SRC="${CHROMIUM_SRC:-/home/nero/chromium/src}"
SKIA="$ROOT_DIR/third_party/skia"
SKIA_OUT="$SKIA/out/Release"
OUT="$ROOT_DIR/out"

CXX="$CHROMIUM_SRC/third_party/llvm-build/Release+Asserts/bin/clang++"
CC="$CHROMIUM_SRC/third_party/llvm-build/Release+Asserts/bin/clang"
SYSROOT="$CHROMIUM_SRC/build/linux/debian_bullseye_amd64-sysroot"
LLD_DIR="$CHROMIUM_SRC/third_party/llvm-build/Release+Asserts/bin"

# ── Parse args ────────────────────────────────────────────────────

RUN_TESTS=false
CLEAN=false
for arg in "$@"; do
    case "$arg" in
        --test) RUN_TESTS=true ;;
        --clean) CLEAN=true ;;
        --help|-h)
            echo "Usage: $0 [--test] [--clean]"
            echo "  --test   Run C API test suite after building"
            echo "  --clean  Remove out/ before building"
            exit 0
            ;;
    esac
done

# ── Verify prerequisites ─────────────────────────────────────────

if [ ! -f "$SKIA_OUT/libskia.a" ]; then
    echo "ERROR: $SKIA_OUT/libskia.a not found."
    echo "Run tools/setup-skia.sh first to build Skia."
    exit 1
fi

if [ ! -f "$CXX" ]; then
    echo "ERROR: Clang not found at $CXX"
    echo "Set CHROMIUM_SRC to your Chromium checkout path."
    exit 1
fi

# ── Clean ─────────────────────────────────────────────────────────

if $CLEAN; then
    echo "Cleaning out/ ..."
    rm -rf "$OUT/obj" "$OUT/libopenui_skia.so" "$OUT/c_api_test" \
           "$OUT/skia_hello" "$OUT/skia_standalone_test"
fi

mkdir -p "$OUT/obj"

# ── Common flags ──────────────────────────────────────────────────

CXXFLAGS=(
    --sysroot="$SYSROOT"
    -std=c++17
    -fPIC
    -I"$SKIA"
    -I"$ROOT_DIR"
    -Wno-gcc-install-dir-libstdcxx
    -DSK_GANESH -DSK_GL -DSK_VULKAN
    -DOUI_SK_BUILDING_DLL
    -fvisibility=hidden
    -O2
)

CFLAGS=(
    --sysroot="$SYSROOT"
    -std=c11
    -I"$ROOT_DIR/include"
    -Wno-gcc-install-dir-libstdcxx
)

LDFLAGS=(
    -fuse-ld=lld
    -B"$LLD_DIR"
    -Wl,--dynamic-linker=/lib64/ld-linux-x86-64.so.2
)

# ── Compile C++ sources ──────────────────────────────────────────

SRCS=(
    src/skia/oui_sk_surface.cc
    src/skia/oui_sk_canvas.cc
    src/skia/oui_sk_paint.cc
    src/skia/oui_sk_path.cc
    src/skia/oui_sk_font.cc
    src/skia/oui_sk_image.cc
    src/skia/oui_sk_effects.cc
    src/skia/oui_sk_util.cc
    src/platform/oui_window_x11.cc
)

# GLX interface source from Skia (not built into libskia.a by default)
SKIA_EXTRA_SRCS=(
    third_party/skia/src/gpu/ganesh/gl/glx/GrGLMakeGLXInterface.cpp
)

echo "=== Compiling C API wrappers ==="
OBJS=()
for src in "${SRCS[@]}"; do
    obj="$OUT/obj/$(basename "${src%.cc}.o")"
    OBJS+=("$obj")
    if [ "$src" -nt "$obj" ] || $CLEAN; then
        echo "  CC  $src"
        "$CXX" "${CXXFLAGS[@]}" -c "$src" -o "$obj"
    else
        echo "  --  $src (up to date)"
    fi
done

# Compile Skia extra sources (GLX interface etc.)
for src in "${SKIA_EXTRA_SRCS[@]}"; do
    obj="$OUT/obj/$(basename "${src%.cpp}.o")"
    OBJS+=("$obj")
    if [ "$src" -nt "$obj" ] || $CLEAN; then
        echo "  CC  $src (Skia extra)"
        "$CXX" "${CXXFLAGS[@]}" -c "$src" -o "$obj"
    else
        echo "  --  $src (up to date)"
    fi
done

# ── Link shared library ──────────────────────────────────────────

echo "=== Linking libopenui_skia.so ==="
"$CXX" --sysroot="$SYSROOT" \
    -shared -o "$OUT/libopenui_skia.so" \
    "${LDFLAGS[@]}" \
    -Wl,--whole-archive "$SKIA_OUT/libskia.a" -Wl,--no-whole-archive \
    "${OBJS[@]}" \
    -L"$SKIA_OUT" \
    -lskshaper -lskunicode_core -lskunicode_icu -lskparagraph \
    -lharfbuzz -lfreetype2 -licu -lpng -ljpeg -lwebp -lwuffs -lskcms \
    -lcompression_utils_portable \
    -lfontconfig -lfreetype \
    -lX11 -lGL -lGLX -lEGL \
    -ldl -lpthread -lm -lz -lstdc++ \
    -Wl,--version-script="$ROOT_DIR/src/skia/openui_skia.map"

SO_SIZE=$(ls -lh "$OUT/libopenui_skia.so" | awk '{print $5}')
EXPORTED=$(nm -D "$OUT/libopenui_skia.so" | grep -c " T ")
echo "  libopenui_skia.so: $SO_SIZE, $EXPORTED exported symbols"

# ── Build examples ────────────────────────────────────────────────

echo "=== Building examples ==="
echo "  CC  examples/skia_hello.c"
"$CC" "${CFLAGS[@]}" "${LDFLAGS[@]}" \
    examples/skia_hello.c -o "$OUT/skia_hello" \
    -L"$OUT" -lopenui_skia

echo "  CC  examples/skia_gallery.c"
"$CC" "${CFLAGS[@]}" "${LDFLAGS[@]}" -lm \
    examples/skia_gallery.c -o "$OUT/skia_gallery" \
    -L"$OUT" -lopenui_skia

# ── Run tests ─────────────────────────────────────────────────────

if $RUN_TESTS; then
    echo ""
    echo "=== Building test suite ==="
    echo "  CC  tests/skia/c_api_test.c"
    "$CC" "${CFLAGS[@]}" "${LDFLAGS[@]}" \
        tests/skia/c_api_test.c -o "$OUT/c_api_test" \
        -L"$OUT" -lopenui_skia

    echo ""
    echo "=== Running C API test suite ==="
    LD_LIBRARY_PATH="$OUT" "$OUT/c_api_test"
fi

echo ""
echo "=== Build complete ==="
echo "  Library: $OUT/libopenui_skia.so ($SO_SIZE)"
echo "  Header:  include/openui/openui_skia.h"
echo "  Example: $OUT/skia_hello"
echo ""
echo "Quick start:"
echo "  LD_LIBRARY_PATH=$OUT $OUT/skia_hello output.png"
