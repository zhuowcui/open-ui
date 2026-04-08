#!/usr/bin/env python3
"""Generate a comprehensive WPT mapping CSV.

Maps every Chromium WPT test in SP12 areas to our ported equivalents,
classifies failures, and outputs a sorted CSV plus a summary report.
"""

import csv
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

# --- Configuration ---

CHROMIUM_WPT_BASE = Path(
    "/home/nero/chromium/src/third_party/blink/web_tests/external/wpt/css"
)

PROJECT_ROOT = Path(__file__).resolve().parent
DATA_DIR = PROJECT_ROOT / "data"
TEMPLATES_JSON = DATA_DIR / "wpt_ported" / "all_wpt_templates.json"
SUMMARY_JSON = DATA_DIR / "pixel_comparison" / "results" / "summary.json"
OUTPUT_CSV = DATA_DIR / "wpt_mapping.csv"

# Chromium directory → our area name
SP12_AREAS = {
    "CSS2/floats": "css2_floats",
    "css-position": "css_position",
    "css-flexbox": "css_flexbox",
    "css-multicol": "css_multicol",
    "css-overflow": "css_overflow",
    "css-sizing": "css_sizing",
    "css-break": "css_break",
    "css-display": "css_display",
    "css-box": "css_box",
    "css-backgrounds": "css_backgrounds",
}

SKIP_DIRS = {"support", "reference", "reftest"}
TEST_EXTENSIONS = {".html", ".xht", ".xhtml", ".htm"}

CSV_COLUMNS = [
    "chromium_test_path",
    "test_name",
    "sp_area",
    "ported",
    "our_test_id",
    "pixel_result",
    "mismatch_pct",
    "failure_category",
    "dependency",
    "notes",
]


# --- Failure classification ---

def _strip_style_blocks(html: str) -> str:
    """Remove all <style>...</style> blocks from HTML."""
    return re.sub(r"<style[^>]*>.*?</style>", "", html, flags=re.DOTALL | re.IGNORECASE)


def _has_descendant_selectors(html: str) -> bool:
    """Check for descendant selectors in <style> blocks.

    A descendant selector has whitespace between parts but no combinator
    (>, +, ~) and no pseudo-class/pseudo-element (:).
    """
    style_blocks = re.findall(
        r"<style[^>]*>(.*?)</style>", html, flags=re.DOTALL | re.IGNORECASE
    )
    for block in style_blocks:
        # Remove comments
        block = re.sub(r"/\*.*?\*/", "", block, flags=re.DOTALL)
        # Extract selectors (text before { )
        selectors = re.findall(r"([^{}]+)\{", block)
        for sel in selectors:
            sel = sel.strip()
            # Skip @-rules
            if sel.startswith("@"):
                continue
            # Split comma-separated selectors
            for part in sel.split(","):
                part = part.strip()
                if not part:
                    continue
                # Tokenize by whitespace
                tokens = part.split()
                if len(tokens) < 2:
                    continue
                # Check if any adjacent pair is a descendant selector
                # (no combinator between them)
                for i in range(len(tokens) - 1):
                    left = tokens[i]
                    right = tokens[i + 1]
                    # Skip combinators
                    if right in (">", "+", "~"):
                        continue
                    if left in (">", "+", "~"):
                        continue
                    # Skip pseudo-selectors that look like descendant
                    if ":" in left or ":" in right:
                        continue
                    return True
    return False


def _has_visible_text(html: str) -> bool:
    """Check for visible text content between tags (after stripping style blocks)."""
    cleaned = _strip_style_blocks(html)
    # Remove all HTML tags
    text = re.sub(r"<[^>]+>", " ", cleaned)
    # Remove HTML entities that are whitespace
    text = re.sub(r"&nbsp;", " ", text)
    # Remove HTML comments
    text = re.sub(r"<!--.*?-->", "", text, flags=re.DOTALL)
    # Check for any visible non-whitespace text
    text = text.strip()
    return bool(re.search(r"[A-Za-z0-9]", text))


def _has_image_url(html: str) -> bool:
    """Check for url() near background/border-image properties."""
    return bool(re.search(r"(background|border-image)[^;]*url\s*\(", html, re.IGNORECASE))


def _has_gradient(html: str) -> bool:
    """Check for gradient() in HTML."""
    return bool(re.search(r"gradient\s*\(", html, re.IGNORECASE))


def _has_ch_unit(html: str) -> bool:
    """Check for ch unit usage."""
    return bool(re.search(r"\d+ch\b", html, re.IGNORECASE))


def _has_containment(html: str) -> bool:
    """Check for contain or content-visibility CSS property."""
    return bool(
        re.search(r"\bcontain\s*:", html, re.IGNORECASE)
        or re.search(r"\bcontent-visibility\s*:", html, re.IGNORECASE)
    )


def _has_margin_trim(html: str) -> bool:
    """Check for margin-trim CSS property."""
    return bool(re.search(r"\bmargin-trim\s*:", html, re.IGNORECASE))


def classify_failure(html: str) -> tuple[str, str]:
    """Classify a failing test into a failure category.

    Returns (failure_category, dependency).
    """
    if _has_descendant_selectors(html):
        return "generator_bug", "descendant selectors in style"
    if _has_visible_text(html):
        return "needs_text", "text rendering"
    if _has_image_url(html):
        return "needs_image", "background/border images"
    if _has_gradient(html):
        return "needs_gradient", "CSS gradients"
    if _has_ch_unit(html):
        return "needs_font_metrics", "ch unit / font metrics"
    if _has_containment(html):
        return "needs_containment", "CSS containment"
    if _has_margin_trim(html):
        return "needs_margin_trim", "margin-trim property"
    return "sp12_layout_bug", ""


# --- Data loading ---

def load_templates() -> dict[str, str]:
    """Load ported test templates: {test_id: html_string}."""
    with open(TEMPLATES_JSON) as f:
        return json.load(f)


def load_pixel_results() -> dict[str, dict]:
    """Load pixel comparison results: {test_id: {status, mismatch_pct}}."""
    with open(SUMMARY_JSON) as f:
        data = json.load(f)
    return {t["id"]: t for t in data["tests"]}


def collect_chromium_tests() -> list[dict]:
    """Walk Chromium WPT directories and collect all test files.

    Handles .html, .xht, .xhtml, and .htm extensions.  For tests in
    subdirectories the flat name is ``{subdir}_{stem}`` which mirrors
    our ported test-ID convention.
    """
    rows = []
    for chromium_dir, area in SP12_AREAS.items():
        base = CHROMIUM_WPT_BASE / chromium_dir
        if not base.exists():
            print(f"WARNING: Chromium directory not found: {base}", file=sys.stderr)
            continue
        for root, dirs, files in os.walk(base):
            dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
            rel_root = Path(root).relative_to(base)
            for fname in files:
                ext = os.path.splitext(fname)[1].lower()
                if ext not in TEST_EXTENSIONS:
                    continue
                full_path = Path(root) / fname
                rel_path = full_path.relative_to(CHROMIUM_WPT_BASE)
                stem = os.path.splitext(fname)[0]

                # Flatten subdirectory into the test name
                if str(rel_root) != ".":
                    flat_name = str(rel_root).replace("/", "_") + "_" + stem
                else:
                    flat_name = stem

                rows.append(
                    {
                        "chromium_test_path": str(rel_path),
                        "test_name": flat_name,
                        "sp_area": area,
                    }
                )
    return rows


# --- Main ---

def main():
    templates = load_templates()
    pixel_results = load_pixel_results()

    # Build lookup: test_name → test_id per area
    ported_lookup: dict[tuple[str, str], str] = {}
    for test_id in templates:
        parts = test_id.split("/")
        if len(parts) >= 3:
            area = parts[1]
            name = parts[2]
            ported_lookup[(area, name)] = test_id

    chromium_tests = collect_chromium_tests()

    # Build CSV rows
    csv_rows = []
    for test in chromium_tests:
        area = test["sp_area"]
        name = test["test_name"]
        key = (area, name)

        row = {
            "chromium_test_path": test["chromium_test_path"],
            "test_name": name,
            "sp_area": area,
            "ported": "",
            "our_test_id": "",
            "pixel_result": "",
            "mismatch_pct": "",
            "failure_category": "",
            "dependency": "",
            "notes": "",
        }

        if key in ported_lookup:
            test_id = ported_lookup[key]
            row["ported"] = "yes"
            row["our_test_id"] = test_id

            pr = pixel_results.get(test_id)
            if pr:
                row["pixel_result"] = pr["status"]
                row["mismatch_pct"] = pr["mismatch_pct"]

                if pr["status"] == "fail":
                    html = templates.get(test_id, "")
                    category, dep = classify_failure(html)
                    row["failure_category"] = category
                    row["dependency"] = dep
        else:
            row["ported"] = "no"
            row["failure_category"] = "not_ported"

        csv_rows.append(row)

    # Sort by sp_area, then test_name
    csv_rows.sort(key=lambda r: (r["sp_area"], r["test_name"]))

    # Write CSV
    OUTPUT_CSV.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_CSV, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=CSV_COLUMNS)
        writer.writeheader()
        writer.writerows(csv_rows)

    print(f"CSV written to {OUTPUT_CSV} ({len(csv_rows)} rows)")
    print()

    # --- Summary ---
    area_stats: dict[str, dict] = defaultdict(
        lambda: {
            "total": 0,
            "ported": 0,
            "passing": 0,
            "failing": 0,
            "categories": defaultdict(int),
        }
    )

    for row in csv_rows:
        area = row["sp_area"]
        area_stats[area]["total"] += 1
        if row["ported"] == "yes":
            area_stats[area]["ported"] += 1
            if row["pixel_result"] == "pass":
                area_stats[area]["passing"] += 1
            elif row["pixel_result"] == "fail":
                area_stats[area]["failing"] += 1
        if row["failure_category"]:
            area_stats[area]["categories"][row["failure_category"]] += 1

    # Collect all category names for column alignment
    all_categories = set()
    for stats in area_stats.values():
        all_categories.update(stats["categories"].keys())
    all_categories = sorted(all_categories)

    # Print table
    header = f"{'Area':<20} {'Total':>6} {'Ported':>7} {'Pass':>6} {'Fail':>6}"
    for cat in all_categories:
        header += f" {cat:>18}"
    print(header)
    print("-" * len(header))

    grand = {"total": 0, "ported": 0, "passing": 0, "failing": 0, "categories": defaultdict(int)}
    for area in sorted(area_stats):
        s = area_stats[area]
        line = f"{area:<20} {s['total']:>6} {s['ported']:>7} {s['passing']:>6} {s['failing']:>6}"
        for cat in all_categories:
            line += f" {s['categories'].get(cat, 0):>18}"
        print(line)
        grand["total"] += s["total"]
        grand["ported"] += s["ported"]
        grand["passing"] += s["passing"]
        grand["failing"] += s["failing"]
        for cat in all_categories:
            grand["categories"][cat] += s["categories"].get(cat, 0)

    print("-" * len(header))
    line = f"{'TOTAL':<20} {grand['total']:>6} {grand['ported']:>7} {grand['passing']:>6} {grand['failing']:>6}"
    for cat in all_categories:
        line += f" {grand['categories'].get(cat, 0):>18}"
    print(line)


if __name__ == "__main__":
    main()
