# Open UI Accountability Framework

**Purpose:** Track progress against Chromium with proof at every level. No narrative claims without data.

## Directory Structure

```
tools/accountability/
├── extract_chromium_tests.sh    # Generate Chromium test inventory CSVs
├── run_pixel_comparison.sh      # Render in Chrome + our engine, diff pixels
├── pixel_diff.py                # Pixel-by-pixel image comparison
├── update_tracking.py           # Update feature CSVs from pixel results
├── report.py                    # Dashboard reading all CSVs
├── data/
│   ├── chromium_tests/          # 17 CSVs listing every Chromium test
│   ├── feature_matrix/          # 3 CSVs listing every feature variant
│   └── pixel_comparison/
│       ├── html_tests/          # HTML files for pixel comparison
│       └── results/             # PNG diffs and result.json files
└── README.md                    # This file
```

## Quick Start

```bash
# Generate the honest status report
python3 tools/accountability/report.py

# Detailed breakdown by area
python3 tools/accountability/report.py --area sp11
python3 tools/accountability/report.py --area sp12
python3 tools/accountability/report.py --area sp13

# List what's not implemented
python3 tools/accountability/report.py --not-implemented

# List what hasn't been pixel-compared
python3 tools/accountability/report.py --not-compared

# Machine-readable CSV output
python3 tools/accountability/report.py --csv
```

## Data Files

### Chromium Test Inventory (`data/chromium_tests/*.csv`)

One CSV per Chromium test area. Each row = one actual Chromium test file.

| Column | Description |
|--------|-------------|
| `chromium_test_path` | Path relative to `web_tests/` |
| `test_name` | Filename without extension |
| `test_type` | reftest / testharness / crashtest / visual |
| `css_property` | Primary CSS property tested |
| `our_test_file` | Our corresponding Rust test file |
| `our_test_name` | Our test function name |
| `ported` | yes / no / partial |
| `port_status` | not_started / in_progress / ported / not_applicable |
| `pass_fail` | pass / fail / skip / not_run |
| `notes` | Free text |

**Source:** Extracted from `~/chromium/src/third_party/blink/web_tests/`

### Feature Matrix (`data/feature_matrix/*.csv`)

One CSV per sprint area. Each row = one testable CSS feature variant.

| Column | Description |
|--------|-------------|
| `sp` | SP11 / SP12 / SP13 |
| `feature` | CSS property name |
| `sub_feature` | Sub-property or behavior |
| `variant` | Specific value being tested |
| `css_spec_section` | CSS spec reference |
| `implemented` | yes / no / partial |
| `implementation_file` | Source file |
| `has_unit_test` | yes / no |
| `pixel_compared_with_chromium` | yes / no |
| `pixel_result` | pass / fail / not_run |
| `pixel_diff_pct` | 0.0 = perfect match |
| `perf_compared_with_chromium` | yes / no |
| `perf_ratio` | our_time / chromium_time |
| `notes` | Free text |

## Pixel Comparison Pipeline

```bash
# Run pixel comparison for all HTML test files
./tools/accountability/run_pixel_comparison.sh --all

# Run and update feature CSVs automatically
./tools/accountability/run_pixel_comparison.sh --update-csv

# Compare a single HTML file
./tools/accountability/run_pixel_comparison.sh data/pixel_comparison/html_tests/sp11/text-decoration-line_underline.html
```

**Requirements:**
- Chrome binary (set `CHROME_BIN` env var or build from `~/chromium/src/`)
- Pillow: `pip install Pillow`
- Our render binary: `cd bindings/rust && cargo build --bin pixel_compare`

## How It Prevents False Claims

1. **Every Chromium test is listed** — you can see exactly which ones we've ported
2. **Every feature variant has a row** — implementation status is tracked per-variant
3. **Pixel comparison requires actual PNGs** — pass/fail is computed, not self-reported
4. **Report reads CSVs directly** — no way to claim progress without data backing it
5. **All data is version-controlled** — audit trail in git history

## Regenerating Data

```bash
# Re-extract Chromium test lists (if Chromium source updated)
./tools/accountability/extract_chromium_tests.sh

# Feature matrices are manually maintained — edit CSVs directly
```
