#!/bin/bash
# tools/verify-abi.sh — Verify ABI stability of libopenui_skia.so
#
# Checks:
# 1. Only oui_sk_* and oui_window_* symbols are exported
# 2. No C++ mangled symbols in the public API
# 3. Header is self-contained (compiles as pure C)
# 4. Symbol count matches expectations

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SO="$ROOT_DIR/out/libopenui_skia.so"
HEADER="$ROOT_DIR/include/openui/openui_skia.h"

CHROMIUM_SRC="${CHROMIUM_SRC:-/home/nero/chromium/src}"
CC="$CHROMIUM_SRC/third_party/llvm-build/Release+Asserts/bin/clang"
SYSROOT="$CHROMIUM_SRC/build/linux/debian_bullseye_amd64-sysroot"

PASS=0
FAIL=0

check() {
    local name="$1"
    local result="$2"
    if [ "$result" = "0" ]; then
        printf "  %-50s PASS\n" "$name"
        PASS=$((PASS + 1))
    else
        printf "  %-50s FAIL\n" "$name"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== ABI Verification ==="

# 1. Check shared library exists
if [ ! -f "$SO" ]; then
    echo "ERROR: $SO not found. Build first."
    exit 1
fi

# 2. All exported T symbols should be oui_sk_* or oui_window_*
NON_OUI=$(nm -D "$SO" | grep " T " | grep -v "oui_sk_\|oui_window_" | wc -l || true)
check "Only oui_ symbols exported" "$NON_OUI"

# 3. No C++ mangled symbols (starting with _Z) in exports
MANGLED=$(nm -D "$SO" | grep " T _Z" | wc -l || true)
check "No C++ mangled symbols exported" "$MANGLED"

# 4. Count exported symbols (should be ~100+)
EXPORTED=$(nm -D "$SO" | grep -c " T ")
if [ "$EXPORTED" -ge 80 ]; then
    check "Sufficient symbols exported ($EXPORTED >= 80)" "0"
else
    check "Sufficient symbols exported ($EXPORTED < 80)" "1"
fi

# 5. Header is self-contained C (compiles without any other includes)
TMPFILE=$(mktemp /tmp/abi_test_XXXXXX.c)
cat > "$TMPFILE" <<'CEOF'
#include "openui/openui_skia.h"
int main(void) {
    OuiSkColor c = oui_sk_color_make(255, 0, 0, 255);
    (void)c;
    return 0;
}
CEOF

if "$CC" --sysroot="$SYSROOT" -std=c11 -I"$ROOT_DIR/include" \
    -Wno-gcc-install-dir-libstdcxx -fsyntax-only "$TMPFILE" 2>/dev/null; then
    check "Header compiles as pure C11" "0"
else
    check "Header compiles as pure C11" "1"
fi
rm -f "$TMPFILE"

# 6. No Skia headers are #included from the public header
SKIA_INCLUDES=$(grep '#include.*Sk\|#include.*Gr\|#include.*skia' "$HEADER" | wc -l || true)
check "No Skia headers in public API" "$SKIA_INCLUDES"

# 7. No 'class' or 'namespace' keywords in public header
CPP_KEYWORDS=$(grep -c '\bclass\b\|\bnamespace\b\|\btemplate\b' "$HEADER" || true)
check "No C++ keywords in public header" "$CPP_KEYWORDS"

# 8. Library size is reasonable (< 40MB, includes paragraph/unicode/ICU)
SIZE_BYTES=$(stat -c%s "$SO")
SIZE_MB=$((SIZE_BYTES / 1048576))
if [ "$SIZE_MB" -le 40 ]; then
    check "Library size reasonable (${SIZE_MB}MB <= 40MB)" "0"
else
    check "Library size reasonable (${SIZE_MB}MB > 40MB)" "1"
fi

echo ""
echo "=== ABI Verification: $PASS passed, $FAIL failed ==="
echo "  Library: $SO (${SIZE_MB}MB)"
echo "  Exported symbols: $EXPORTED"

exit "$FAIL"
