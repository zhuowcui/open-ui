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
# Build the renderer used by WPT comparison
cd bindings/rust
cargo build --release --package pixel-compare
cd ../..

# Run the full WPT comparison set
LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" \
  python3 -u tools/accountability/run_all_pixel_comparisons.py 'wpt/'

# Regenerate tracking artifacts
python3 tools/accountability/generate_wpt_mapping.py
python3 tools/accountability/generate_sp12_5_csv.py

# Verify all accountability invariants
python3 tools/accountability/audit.py
```

Current verified snapshot:

| Metric | Value |
|---|---:|
| Chromium SP12-scope inventory rows | 7673 |
| Ported/runnable WPT tests | 3566 |
| Runnable passes | 3267 |
| Runnable failures | 299 |
| Runnable render/diff errors | 0 |
| Unported but explicitly categorized rows | 4107 |
| Generic `not_ported` bucket rows | 0 |
| `sp12_layout_bug` rows | 0 |
| `needs_text` rows | 0 |
| `needs_font_metrics` rows | 0 |
| Runnable `sp13_multicol` rows | 0 |
| Unported `sp13_multicol` residuals | 1018 |

This is the verified SP13-R snapshot: all 2823 frozen exact baseline IDs and all
351 runnable multicol targets pass at 0.0% mismatch. The 1018 unported multicol
rows remain reason-owned in the residual ledger.

## Data Files

### WPT Mapping (`data/wpt_mapping.csv`)

Full Chromium SP12-scope inventory. Each row is one Chromium WPT file, whether or
not it has a runnable Rust port.

| Column | Description |
|--------|-------------|
| `chromium_test_path` | Path relative to Chromium's external WPT CSS directory |
| `test_name` | Flattened test name used by the Rust WPT registry |
| `sp_area` | Area such as `css_flexbox`, `css_backgrounds`, or `css_overflow` |
| `ported` | `yes` for runnable Rust tests, `no` for explicitly deferred tests |
| `our_test_id` | `wpt/...` test id for ported rows |
| `pixel_result` | `pass` or `fail` for ported rows |
| `mismatch_pct` | Pixel mismatch percentage for ported rows |
| `failure_category` | Explicit owning category for failing or unported rows |
| `dependency` | Human-readable dependency label |
| `notes` | Porter rejection reason or additional tracking note |

`not_ported` is not an acceptable long-term category. Unported rows must be assigned
to named categories such as `needs_javascript`, `needs_writing_mode`, `sp13_fragmentation`,
`needs_grid`, or `needs_table_layout`.

### Pixel Summary (`data/pixel_comparison/results/summary.json`)

Authoritative status for the runnable generated Rust WPT tests. Focused runs overwrite
this file, so save and restore it when probing a subset.

### Deferred Runnable Tests (`data/sp12_5_deferred.csv`)

Generated list of failing runnable tests whose remaining dependencies are owned by
another SP/future feature. This CSV intentionally excludes unported rows because they
do not have runnable pixel results yet.

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
# Full run
LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" \
  python3 -u tools/accountability/run_all_pixel_comparisons.py 'wpt/'

# Focused prefix run; remember this overwrites summary.json
cp tools/accountability/data/pixel_comparison/results/summary.json /tmp/summary.json
rm -f tools/accountability/data/pixel_comparison/results/<test-id>/result.json
LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" \
  python3 -u tools/accountability/run_all_pixel_comparisons.py '<prefix>'
cp /tmp/summary.json tools/accountability/data/pixel_comparison/results/summary.json
```

**Requirements:**
- Chrome binary (set `CHROME_BIN` env var or build from `~/chromium/src/`)
- Pillow: `pip install Pillow`
- Our render binary: `cd bindings/rust && cargo build --release --package pixel-compare`

## How It Prevents False Claims

1. **Every Chromium test is listed** — you can see exactly which ones we've ported
2. **Every feature variant has a row** — implementation status is tracked per-variant
3. **Pixel comparison requires actual PNGs** — pass/fail is computed, not self-reported
4. **Report reads CSVs directly** — no way to claim progress without data backing it
5. **All data is version-controlled** — audit trail in git history
6. **Generic buckets fail review** — unported tests need named dependency categories
7. **Audit is the gate** — pass/fail claims are not accepted without `audit.py`

## Regenerating Data

```bash
# Re-extract Chromium test lists (if Chromium source updated)
./tools/accountability/extract_chromium_tests.sh

# Regenerate WPT mapping and deferred docs from current summary/templates
python3 tools/accountability/generate_wpt_mapping.py
python3 tools/accountability/generate_sp12_5_csv.py
```
