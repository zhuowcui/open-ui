#!/usr/bin/env bash
# extract_chromium_tests.sh — Scan Chromium web_tests directories and generate
# CSV inventories of every test file for areas covered by Open UI (SP11-SP13).
#
# Usage:
#   ./extract_chromium_tests.sh [chromium_web_tests_dir]
#
# Default: ~/chromium/src/third_party/blink/web_tests
#
# Output: data/chromium_tests/*.csv

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_TESTS="${1:-$HOME/chromium/src/third_party/blink/web_tests}"
OUT_DIR="$SCRIPT_DIR/data/chromium_tests"

if [ ! -d "$WEB_TESTS" ]; then
    echo "ERROR: Chromium web_tests directory not found at $WEB_TESTS" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

CSV_HEADER="chromium_test_path,test_name,test_type,css_property,our_test_file,our_test_name,ported,port_status,pass_fail,notes"

# Detect test type from file content and naming conventions.
# reftest: has <link rel=match> or <link rel=mismatch> or filename has -ref
# testharness: has <script src=*testharness.js*>
# crashtest: path contains /crashtests/ or filename has crash
# Otherwise: visual
detect_test_type() {
    local filepath="$1"
    local filename
    filename="$(basename "$filepath")"

    # Check if it's a reference file first (not a test itself) — must precede
    # crashtest check because some ref files contain "crash" in the name.
    if [[ "$filename" == *-ref.html ]] || [[ "$filename" == *-ref.htm ]] || \
       [[ "$filename" == *-notref.html ]] || [[ "$filename" == *-notref.htm ]]; then
        echo "reference"
        return
    fi

    # Check path patterns (fast)
    if [[ "$filepath" == */crashtests/* ]] || [[ "$filename" == *crash* ]]; then
        echo "crashtest"
        return
    fi

    # Check for support/resource files
    if [[ "$filepath" == */support/* ]] || [[ "$filepath" == */resources/* ]]; then
        echo "support"
        return
    fi

    # Check content (first 50 lines for speed)
    local head_content
    head_content="$(head -50 "$filepath" 2>/dev/null || true)"

    if echo "$head_content" | grep -qi 'rel=.*match\|rel="match"\|rel='\''match'\'''; then
        echo "reftest"
        return
    fi

    if echo "$head_content" | grep -qi 'testharness\.js\|testharnessreport\.js'; then
        echo "testharness"
        return
    fi

    echo "visual"
}

# Infer the primary CSS property from the test path and filename.
infer_css_property() {
    local filepath="$1"
    local filename
    filename="$(basename "$filepath" | sed 's/\.\(html\|htm\|xhtml\)$//')"

    # Try to extract from directory name (e.g., css-text/word-break/ → word-break)
    local dir
    dir="$(dirname "$filepath")"
    local leaf_dir
    leaf_dir="$(basename "$dir")"

    # Common CSS property directory names
    case "$leaf_dir" in
        word-break|overflow-wrap|white-space|text-align|text-indent|text-transform|\
        letter-spacing|word-spacing|line-break|hyphens|text-overflow|text-justify|\
        hanging-punctuation|text-wrap)
            echo "$leaf_dir"; return ;;
        text-decoration|text-emphasis|text-shadow|text-underline-offset|\
        text-underline-position|text-decoration-skip-ink|text-decoration-thickness)
            echo "$leaf_dir"; return ;;
        font-family|font-weight|font-style|font-size|font-stretch|font-variant|\
        font-feature-settings|font-variation-settings|font-display|font-face|\
        font-palette|font-size-adjust)
            echo "$leaf_dir"; return ;;
        writing-mode|direction|unicode-bidi|text-orientation|text-combine-upright)
            echo "$leaf_dir"; return ;;
        ruby-align|ruby-position)
            echo "$leaf_dir"; return ;;
        position|top|right|bottom|left|z-index|sticky)
            echo "$leaf_dir"; return ;;
        flex-direction|flex-wrap|flex-flow|flex-grow|flex-shrink|flex-basis|\
        justify-content|align-items|align-self|align-content|order|gap)
            echo "$leaf_dir"; return ;;
        column-count|column-width|column-gap|column-rule|column-span|columns)
            echo "$leaf_dir"; return ;;
        overflow|overflow-x|overflow-y|overflow-clip-margin)
            echo "$leaf_dir"; return ;;
        width|height|min-width|max-width|min-height|max-height|box-sizing|aspect-ratio)
            echo "$leaf_dir"; return ;;
        break-before|break-after|break-inside|orphans|widows)
            echo "$leaf_dir"; return ;;
        display|float|clear)
            echo "$leaf_dir"; return ;;
        initial-letter|vertical-align|line-height|dominant-baseline|baseline-source)
            echo "$leaf_dir"; return ;;
        first-letter|first-line)
            echo "::$leaf_dir"; return ;;
        parsing|computed|getComputedStyle)
            # Go up one more directory
            local parent_dir
            parent_dir="$(basename "$(dirname "$dir")")"
            echo "$parent_dir"; return ;;
    esac

    # Try to extract from filename prefix (e.g., word-break-normal-001.html → word-break)
    # Match longest known property prefix
    local prop
    for prop in text-decoration-skip-ink text-decoration-thickness text-decoration-style \
                text-decoration-color text-decoration-line text-underline-position \
                text-underline-offset text-combine-upright text-emphasis-position \
                text-emphasis-style text-emphasis-color text-align-last text-decoration \
                text-emphasis text-transform text-overflow text-justify text-indent \
                text-shadow text-wrap text-align overflow-wrap font-variation-settings \
                font-feature-settings font-variant-east-asian font-variant-numeric \
                font-variant-ligatures font-variant-caps font-variant-position \
                font-variant-alternates font-size-adjust font-synthesis font-optical-sizing \
                font-display font-palette font-stretch font-weight font-family font-style \
                font-size writing-mode text-orientation unicode-bidi direction \
                justify-content align-content align-items align-self flex-direction \
                flex-shrink flex-basis flex-grow flex-wrap column-count column-width \
                column-gap column-rule column-span overflow-clip-margin overflow \
                line-height vertical-align initial-letter dominant-baseline baseline-source \
                break-before break-after break-inside box-decoration-break box-sizing \
                aspect-ratio min-width max-width min-height max-height position \
                word-break line-break white-space word-spacing letter-spacing hyphens \
                hanging-punctuation ruby-position ruby-align display float clear \
                margin padding border width height orphans widows; do
        if [[ "$filename" == ${prop}* ]]; then
            echo "$prop"
            return
        fi
    done

    echo "unknown"
}

# Escape a value for CSV (handle commas and quotes).
csv_escape() {
    local val="$1"
    if [[ "$val" == *,* ]] || [[ "$val" == *\"* ]] || [[ "$val" == *$'\n'* ]]; then
        val="${val//\"/\"\"}"
        echo "\"$val\""
    else
        echo "$val"
    fi
}

# Generate a CSV for one Chromium directory.
# $1 = output CSV filename (without path)
# $2 = directory relative to $WEB_TESTS
generate_csv() {
    local csv_name="$1"
    local rel_dir="$2"
    local full_dir="$WEB_TESTS/$rel_dir"
    local out_file="$OUT_DIR/$csv_name"

    if [ ! -d "$full_dir" ]; then
        echo "WARNING: Directory not found: $full_dir — skipping $csv_name" >&2
        return
    fi

    echo "$CSV_HEADER" > "$out_file"

    local count=0
    while IFS= read -r filepath; do
        local rel_path="${filepath#$WEB_TESTS/}"
        local filename
        filename="$(basename "$filepath")"
        local test_name="${filename%.*}"

        local test_type
        test_type="$(detect_test_type "$filepath")"

        # Skip reference files and support files — they aren't tests
        if [[ "$test_type" == "reference" ]] || [[ "$test_type" == "support" ]]; then
            continue
        fi

        local css_prop
        css_prop="$(infer_css_property "$rel_path")"

        # Write CSV row: path, name, type, property, then empty tracking columns
        echo "$(csv_escape "$rel_path"),$(csv_escape "$test_name"),$test_type,$(csv_escape "$css_prop"),,,,not_started,not_run," >> "$out_file"
        count=$((count + 1))
    done < <(find "$full_dir" -type f \( -name "*.html" -o -name "*.htm" -o -name "*.xhtml" \) | sort)

    echo "  $csv_name: $count tests"
}

echo "═══════════════════════════════════════════════════"
echo "  Chromium Test Extraction"
echo "  Source: $WEB_TESTS"
echo "  Output: $OUT_DIR"
echo "═══════════════════════════════════════════════════"
echo ""

echo "SP11 — Text Rendering"
generate_csv "sp11_css_text_tests.csv"         "external/wpt/css/css-text"
generate_csv "sp11_css_fonts_tests.csv"        "external/wpt/css/css-fonts"
generate_csv "sp11_css_text_decor_tests.csv"   "external/wpt/css/css-text-decor"
generate_csv "sp11_css_writing_modes_tests.csv" "external/wpt/css/css-writing-modes"
generate_csv "sp11_css_ruby_tests.csv"         "external/wpt/css/css-ruby"
echo ""

echo "SP12 — Block Layout"
generate_csv "sp12_css_display_tests.csv"      "external/wpt/css/css-display"
generate_csv "sp12_css_box_tests.csv"          "external/wpt/css/css-box"
generate_csv "sp12_css_position_tests.csv"     "external/wpt/css/css-position"
generate_csv "sp12_css_flexbox_tests.csv"      "external/wpt/css/css-flexbox"
generate_csv "sp12_css_multicol_tests.csv"     "external/wpt/css/css-multicol"
generate_csv "sp12_css_overflow_tests.csv"     "external/wpt/css/css-overflow"
generate_csv "sp12_css_sizing_tests.csv"       "external/wpt/css/css-sizing"
generate_csv "sp12_css_break_tests.csv"        "external/wpt/css/css-break"
echo ""

echo "SP12 — Blink Layout Tests"
# Combine fast/block + css2.1 into one CSV
{
    echo "$CSV_HEADER"
} > "$OUT_DIR/sp12_blink_block_tests.csv"

blink_block_count=0
for blink_dir in "fast/block" "css2.1" "fast/multicol" "fast/flexbox" "fast/overflow"; do
    if [ -d "$WEB_TESTS/$blink_dir" ]; then
        while IFS= read -r filepath; do
            rel_path="${filepath#$WEB_TESTS/}"
            filename="$(basename "$filepath")"
            test_name="${filename%.*}"
            test_type="$(detect_test_type "$filepath")"
            if [[ "$test_type" == "reference" ]] || [[ "$test_type" == "support" ]]; then
                continue
            fi
            css_prop="$(infer_css_property "$rel_path")"
            echo "$(csv_escape "$rel_path"),$(csv_escape "$test_name"),$test_type,$(csv_escape "$css_prop"),,,,not_started,not_run," >> "$OUT_DIR/sp12_blink_block_tests.csv"
            blink_block_count=$((blink_block_count + 1))
        done < <(find "$WEB_TESTS/$blink_dir" -type f \( -name "*.html" -o -name "*.htm" \) | sort)
    fi
done
echo "  sp12_blink_block_tests.csv: $blink_block_count tests"
echo ""

echo "SP13 — Inline Layout"
generate_csv "sp13_css_inline_tests.csv"       "external/wpt/css/css-inline"
generate_csv "sp13_css_pseudo_tests.csv"       "external/wpt/css/css-pseudo"

# Blink inline tests
{
    echo "$CSV_HEADER"
} > "$OUT_DIR/sp13_blink_inline_tests.csv"

blink_inline_count=0
for blink_dir in "fast/inline" "fast/text" "fast/writing-mode"; do
    if [ -d "$WEB_TESTS/$blink_dir" ]; then
        while IFS= read -r filepath; do
            rel_path="${filepath#$WEB_TESTS/}"
            filename="$(basename "$filepath")"
            test_name="${filename%.*}"
            test_type="$(detect_test_type "$filepath")"
            if [[ "$test_type" == "reference" ]] || [[ "$test_type" == "support" ]]; then
                continue
            fi
            css_prop="$(infer_css_property "$rel_path")"
            echo "$(csv_escape "$rel_path"),$(csv_escape "$test_name"),$test_type,$(csv_escape "$css_prop"),,,,not_started,not_run," >> "$OUT_DIR/sp13_blink_inline_tests.csv"
            blink_inline_count=$((blink_inline_count + 1))
        done < <(find "$WEB_TESTS/$blink_dir" -type f \( -name "*.html" -o -name "*.htm" \) | sort)
    fi
done
echo "  sp13_blink_inline_tests.csv: $blink_inline_count tests"
echo ""

# Summary
echo "═══════════════════════════════════════════════════"
echo "  Extraction Complete"
echo "═══════════════════════════════════════════════════"
total=0
for csv in "$OUT_DIR"/*.csv; do
    # Subtract 1 for header row
    rows=$(($(wc -l < "$csv") - 1))
    total=$((total + rows))
    printf "  %-42s %6d tests\n" "$(basename "$csv")" "$rows"
done
echo "  ──────────────────────────────────────────────"
printf "  %-42s %6d tests\n" "TOTAL" "$total"
