#!/usr/bin/env bash
# capture_chromium.sh — Render each HTML test page in headless Chrome and
# save a PNG screenshot for pixel comparison with Open UI.
#
# Usage:  ./capture_chromium.sh [chrome_binary]
#
# If no binary is given the script probes the usual locations:
#   1. ~/chromium/src/out/Release/chrome   (locally built Chromium)
#   2. google-chrome                       (distro / snap)
#   3. chromium-browser
#   4. chromium
#
# Screenshots are written to tests/pixel_text/chromium_refs/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HTML_DIR="$SCRIPT_DIR"
OUT_DIR="$SCRIPT_DIR/chromium_refs"
WINDOW_SIZE="500,1200"

# ── Locate Chrome ───────────────────────────────────────────────────
find_chrome() {
    if [[ -n "${1:-}" ]]; then
        echo "$1"; return
    fi
    local candidates=(
        "$HOME/chromium/src/out/Release/chrome"
        "$(command -v google-chrome 2>/dev/null || true)"
        "$(command -v chromium-browser 2>/dev/null || true)"
        "$(command -v chromium 2>/dev/null || true)"
    )
    for c in "${candidates[@]}"; do
        if [[ -n "$c" && -x "$c" ]]; then
            echo "$c"; return
        fi
    done
    echo ""
}

CHROME=$(find_chrome "${1:-}")
if [[ -z "$CHROME" ]]; then
    echo "ERROR: No Chrome/Chromium binary found."
    echo "Install Chrome or pass the path as the first argument."
    echo "  e.g.  $0 /usr/bin/google-chrome"
    exit 1
fi

echo "Using Chrome binary: $CHROME"
echo "Chrome version: $("$CHROME" --version 2>/dev/null || echo 'unknown')"

# ── Capture screenshots ─────────────────────────────────────────────
mkdir -p "$OUT_DIR"

HTML_FILES=(
    basic_text.html
    line_breaking.html
    text_alignment.html
    vertical_align.html
    text_decoration.html
    letter_word_spacing.html
    text_transform.html
    bidi_mixed.html
    line_height.html
    white_space.html
)

for html in "${HTML_FILES[@]}"; do
    name="${html%.html}"
    src="$HTML_DIR/$html"
    dst="$OUT_DIR/${name}_chromium.png"

    if [[ ! -f "$src" ]]; then
        echo "SKIP  $html (not found)"
        continue
    fi

    echo -n "Capturing $html ... "
    "$CHROME" \
        --headless \
        --disable-gpu \
        --disable-software-rasterizer \
        --run-all-compositor-stages-before-draw \
        --no-sandbox \
        --disable-dev-shm-usage \
        --force-device-scale-factor=1 \
        --font-render-hinting=none \
        --screenshot="$dst" \
        --window-size="$WINDOW_SIZE" \
        "file://$src" \
        2>/dev/null

    if [[ -f "$dst" ]]; then
        echo "OK  → $dst"
    else
        echo "FAIL (no output file)"
    fi
done

echo ""
echo "Done. Chromium reference screenshots saved to: $OUT_DIR"
