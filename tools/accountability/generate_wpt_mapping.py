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

# Import shared detectors (single source of truth)
sys.path.insert(0, str(Path(__file__).resolve().parent))
from shared_detectors import (
    CATEGORY_FOR_DEP,
    classify_dependencies,
    classify_failure_categories,
)

# --- Configuration ---

CHROMIUM_WPT_BASE = Path(
    os.environ.get(
        "CHROMIUM_WPT_CSS",
        os.path.expanduser("~/chromium/src/third_party/blink/web_tests/external/wpt/css"),
    )
)

SCRIPT_DIR = Path(__file__).resolve().parent
DATA_DIR = SCRIPT_DIR / "data"
TEMPLATES_JSON = DATA_DIR / "wpt_ported" / "all_wpt_templates.json"
SUMMARY_JSON = DATA_DIR / "pixel_comparison" / "results" / "summary.json"
OUTPUT_CSV = DATA_DIR / "wpt_mapping.csv"
PORT_REPORT_DIR = DATA_DIR / "wpt_ported"

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

REPORT_FILE_FOR_AREA = {
    "css2_floats": "wpt_css2_floats_report.csv",
    "css_position": "wpt_css_position_report.csv",
    "css_flexbox": "wpt_css_flexbox_report.csv",
    "css_multicol": "wpt_css_multicol_report.csv",
    "css_overflow": "wpt_css_overflow_report.csv",
    "css_sizing": "wpt_css_sizing_report.csv",
    "css_break": "wpt_css_break_report.csv",
    "css_display": "wpt_css_display_report.csv",
    "css_box": "wpt_css_box_report.csv",
    "css_backgrounds": "wpt_css_backgrounds_report.csv",
}


TEST_EXTENSIONS = {".html", ".xht", ".xhtml", ".htm"}


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


def load_portability_reasons() -> dict[tuple[str, str], str]:
    """Load porter rejection reasons by (area, test_name)."""
    reasons: dict[tuple[str, str], str] = {}
    for area, filename in REPORT_FILE_FOR_AREA.items():
        path = PORT_REPORT_DIR / filename
        if not path.exists():
            continue
        with open(path) as f:
            for row in csv.DictReader(f):
                if row.get("status") == "ported":
                    continue
                reasons[(area, row.get("filename", ""))] = row.get("reason", "")
    return reasons


def dependency_for_portability_reason(reason: str) -> str:
    """Map porter rejection reasons to a named deferred dependency key."""
    r = reason.lower()
    if "javascript" in r:
        return "javascript"
    if "line-clamp" in r or "-webkit-box-orient" in r:
        return "line_clamp"
    if "grid" in r:
        return "grid_layout"
    if "table" in r or "border-collapse" in r or "border-spacing" in r or "caption-side" in r:
        return "table_layout"
    if "writing-mode" in r or "unicode-bidi" in r:
        return "writing_mode"
    if "margin-trim" in r:
        return "margin_trim"
    if "contain" in r:
        return "css_containment"
    if "transform" in r or "filter" in r or "clip-path" in r or "mask" in r or "animation" in r or "transition" in r:
        return "visual_effects"
    if "img" in r or "image" in r or "iframe" in r or "video" in r:
        return "image_rendering"
    if "canvas" in r or "svg" in r:
        return "canvas_svg"
    if any(tag in r for tag in ("button", "input", "select", "textarea", "fieldset", "details", "dialog", "audio", "form")):
        return "form_controls"
    if "content" in r or "before" in r or "after" in r or "first-letter" in r or "first-line" in r:
        return "generated_content"
    if "complex_css_selector" in r:
        return "advanced_selectors"
    if "no_layout_content" in r:
        return "non_visual"
    return "advanced_selectors"


def classify_unported_test(html: str, area: str, name: str, reason: str) -> tuple[str, str]:
    """Classify an unported Chromium test into explicit owning dependencies."""
    test_id = f"wpt/{area}/{name}"
    deps = classify_dependencies(html, test_id=test_id)
    if not deps:
        deps = [dependency_for_portability_reason(reason)]
    categories = [CATEGORY_FOR_DEP[d] for d in deps]
    labels = []
    from shared_detectors import DEPENDENCY_DEFS
    for key in deps:
        for k, label, _sp, _det in DEPENDENCY_DEFS:
            if k == key:
                labels.append(label)
                break
    return ",".join(categories), "; ".join(labels)


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
    portability_reasons = load_portability_reasons()

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
                # Treat render/diff errors as failures for tracking purposes.
                status = pr["status"]
                if status == "error":
                    status = "fail"
                row["pixel_result"] = status
                row["mismatch_pct"] = pr["mismatch_pct"]

                if status == "fail":
                    html = templates.get(test_id, "")
                    category, dep = classify_failure_categories(html, test_id=test_id)
                    row["failure_category"] = category
                    row["dependency"] = dep
        else:
            row["ported"] = "no"
            html_path = CHROMIUM_WPT_BASE / test["chromium_test_path"]
            html = html_path.read_text(errors="ignore") if html_path.exists() else ""
            reason = portability_reasons.get((area, name), "")
            category, dep = classify_unported_test(html, area, name, reason)
            row["failure_category"] = category
            row["dependency"] = dep
            row["notes"] = f"Porter deferred: {reason}" if reason else "Porter deferred: outside current Rust renderer support"

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
            # Split multi-label categories for accurate per-atom counting
            for part in row["failure_category"].split(","):
                part = part.strip()
                if part:
                    area_stats[area]["categories"][part] += 1

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
