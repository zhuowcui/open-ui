#!/usr/bin/env bash
# run_pixel_comparison.sh — Render HTML in headless Chrome AND our engine,
# then diff the resulting PNGs pixel-by-pixel.
#
# Usage:
#   ./run_pixel_comparison.sh <html_file>              # Compare one file
#   ./run_pixel_comparison.sh --all                    # Compare all in html_tests/
#   ./run_pixel_comparison.sh --update-csv             # Run all + update feature CSVs
#
# Prerequisites:
#   - Chrome binary (set CHROME_BIN or auto-detected)
#   - Our render_to_png binary (cargo build in bindings/rust)
#
# Output: data/pixel_comparison/results/<test_name>/
#           ├── chromium.png
#           ├── openui.png
#           ├── diff.png
#           └── result.json

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HTML_DIR="$SCRIPT_DIR/data/pixel_comparison/html_tests"
RESULTS_DIR="$SCRIPT_DIR/data/pixel_comparison/results"
FEATURE_DIR="$SCRIPT_DIR/data/feature_matrix"

VIEWPORT_W=800
VIEWPORT_H=600
SCALE=1
TOLERANCE=2  # Per-channel tolerance for anti-aliasing

# ── Find Chrome ──────────────────────────────────────────
find_chrome() {
    if [ -n "${CHROME_BIN:-}" ] && [ -x "$CHROME_BIN" ]; then
        echo "$CHROME_BIN"
        return
    fi
    # Check common locations
    local repo_root
    repo_root="$(cd "$SCRIPT_DIR/../.." && pwd)"
    # Find downloaded Chrome in repo (from @puppeteer/browsers)
    local downloaded
    downloaded="$(find "$repo_root/chrome" -name "chrome" -type f 2>/dev/null | head -1)"
    for bin in \
        "$HOME/chromium/src/out/Release/chrome" \
        "$HOME/chromium/src/out/Default/chrome" \
        "${downloaded:-}" \
        "$(which google-chrome 2>/dev/null || true)" \
        "$(which chromium 2>/dev/null || true)" \
        "$(which chromium-browser 2>/dev/null || true)" \
        "/usr/bin/google-chrome" \
        "/usr/bin/chromium-browser"; do
        if [ -n "$bin" ] && [ -x "$bin" ]; then
            echo "$bin"
            return
        fi
    done
    echo ""
}

CHROME="$(find_chrome)"

# ── Render with Chrome ───────────────────────────────────
chrome_screenshot() {
    local html_file="$1"
    local output_png="$2"

    if [ -z "$CHROME" ]; then
        echo "ERROR: Chrome binary not found. Set CHROME_BIN or build Chromium." >&2
        return 1
    fi

    # Set LD_LIBRARY_PATH to Chrome's directory for bundled shared libs
    local chrome_dir
    chrome_dir="$(dirname "$CHROME")"
    LD_LIBRARY_PATH="${chrome_dir}:${LD_LIBRARY_PATH:-}" \
    "$CHROME" \
        --headless=new \
        --disable-gpu \
        --no-sandbox \
        --disable-software-rasterizer \
        --force-device-scale-factor="$SCALE" \
        --window-size="${VIEWPORT_W},${VIEWPORT_H}" \
        --screenshot="$output_png" \
        --default-background-color=0 \
        "file://$html_file" \
        2>/dev/null

    if [ ! -f "$output_png" ]; then
        echo "ERROR: Chrome failed to produce screenshot for $html_file" >&2
        return 1
    fi
}

# ── Render with our engine ───────────────────────────────
openui_render() {
    local html_file="$1"
    local output_png="$2"

    # Use the pixel_compare binary from our Rust crate
    local render_bin="$SCRIPT_DIR/../../bindings/rust/target/release/pixel_compare"
    if [ ! -x "$render_bin" ]; then
        render_bin="$SCRIPT_DIR/../../bindings/rust/target/debug/pixel_compare"
    fi

    if [ ! -x "$render_bin" ]; then
        echo "ERROR: pixel_compare binary not found. Build with: cd bindings/rust && cargo build --bin pixel_compare" >&2
        return 1
    fi

    "$render_bin" render "$html_file" "$output_png" "$VIEWPORT_W" "$VIEWPORT_H"
}

# ── Pixel diff ───────────────────────────────────────────
pixel_diff() {
    local img_a="$1"   # chromium.png
    local img_b="$2"   # openui.png
    local diff_out="$3" # diff.png
    local json_out="$4" # result.json

    python3 "$SCRIPT_DIR/pixel_diff.py" \
        "$img_a" "$img_b" "$diff_out" "$json_out" \
        --tolerance "$TOLERANCE"
}

# ── Compare one HTML file ────────────────────────────────
compare_one() {
    local html_file="$1"
    local basename
    basename="$(basename "$html_file" .html)"
    local sp_dir
    sp_dir="$(basename "$(dirname "$html_file")")"
    local result_dir="$RESULTS_DIR/$sp_dir/$basename"

    mkdir -p "$result_dir"

    local chromium_png="$result_dir/chromium.png"
    local openui_png="$result_dir/openui.png"
    local diff_png="$result_dir/diff.png"
    local result_json="$result_dir/result.json"

    echo "  Comparing: $sp_dir/$basename"

    # Step 1: Chrome screenshot
    if ! chrome_screenshot "$html_file" "$chromium_png"; then
        echo "    ❌ Chrome screenshot failed"
        echo "{\"status\": \"error\", \"reason\": \"chrome_screenshot_failed\"}" > "$result_json"
        return 1
    fi

    # Step 2: Our render
    if ! openui_render "$html_file" "$openui_png"; then
        echo "    ❌ OpenUI render failed"
        echo "{\"status\": \"error\", \"reason\": \"openui_render_failed\"}" > "$result_json"
        return 1
    fi

    # Step 3: Diff
    if ! pixel_diff "$chromium_png" "$openui_png" "$diff_png" "$result_json"; then
        echo "    ❌ Diff failed"
        return 1
    fi

    # Step 4: Report
    local mismatched
    mismatched=$(python3 -c "import json; d=json.load(open('$result_json')); print(d.get('mismatched_pixels', -1))")
    local total
    total=$(python3 -c "import json; d=json.load(open('$result_json')); print(d.get('total_pixels', -1))")
    local pct
    pct=$(python3 -c "import json; d=json.load(open('$result_json')); print(f\"{d.get('mismatch_pct', -1):.4f}\")")

    if [ "$mismatched" = "0" ]; then
        echo "    ✅ PASS — 0 mismatched pixels (100% pixel perfect)"
    else
        echo "    ❌ FAIL — $mismatched / $total pixels mismatched ($pct%)"
    fi
}

# ── Update feature CSVs ─────────────────────────────────
update_csvs() {
    python3 "$SCRIPT_DIR/update_tracking.py" --pixel-results "$RESULTS_DIR" --feature-dir "$FEATURE_DIR"
}

# ── Main ─────────────────────────────────────────────────
case "${1:-}" in
    --all)
        echo "═══════════════════════════════════════════════════"
        echo "  Pixel Comparison — All Tests"
        echo "═══════════════════════════════════════════════════"
        echo "  Chrome: ${CHROME:-NOT FOUND}"
        echo ""
        pass=0
        fail=0
        error=0
        for html in $(find "$HTML_DIR" -name "*.html" | sort); do
            if compare_one "$html"; then
                result_json="$RESULTS_DIR/$(basename "$(dirname "$html")")/$(basename "$html" .html)/result.json"
                mismatched=$(python3 -c "import json; d=json.load(open('$result_json')); print(d.get('mismatched_pixels', -1))" 2>/dev/null || echo "-1")
                if [ "$mismatched" = "0" ]; then
                    pass=$((pass + 1))
                else
                    fail=$((fail + 1))
                fi
            else
                error=$((error + 1))
            fi
        done
        echo ""
        echo "  Results: $pass pass, $fail fail, $error error"
        ;;
    --update-csv)
        echo "Running all comparisons and updating CSVs..."
        "$0" --all
        update_csvs
        ;;
    "")
        echo "Usage: $0 <html_file> | --all | --update-csv"
        exit 1
        ;;
    *)
        compare_one "$(realpath "$1")"
        ;;
esac
