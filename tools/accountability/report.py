#!/usr/bin/env python3
"""
report.py — Open UI Accountability Dashboard

Reads CSV tracking files and produces an honest status report.
No narrative claims — only numbers backed by data.

Usage:
    python3 report.py                    # Full summary dashboard
    python3 report.py --area sp11        # Detailed SP11 breakdown
    python3 report.py --area sp12        # Detailed SP12 breakdown
    python3 report.py --area sp13        # Detailed SP13 breakdown
    python3 report.py --failing          # List all failing tests
    python3 report.py --not-compared     # Features without pixel comparison
    python3 report.py --not-implemented  # Features not yet implemented
    python3 report.py --csv              # Machine-readable CSV output
"""

import argparse
import csv
import json
import os
import sys
from collections import defaultdict
from datetime import datetime


SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CHROMIUM_DIR = os.path.join(SCRIPT_DIR, "data", "chromium_tests")
FEATURE_DIR = os.path.join(SCRIPT_DIR, "data", "feature_matrix")

# Map CSV filenames to sprint areas
CHROMIUM_CSV_AREA = {
    "sp11_css_text_tests.csv": "SP11",
    "sp11_css_fonts_tests.csv": "SP11",
    "sp11_css_text_decor_tests.csv": "SP11",
    "sp11_css_writing_modes_tests.csv": "SP11",
    "sp11_css_ruby_tests.csv": "SP11",
    "sp11_css_color_tests.csv": "SP11",
    "sp11_blink_text_tests.csv": "SP11",
    "sp11_blink_writing_mode_tests.csv": "SP11",
    "sp12_css_display_tests.csv": "SP12",
    "sp12_css_box_tests.csv": "SP12",
    "sp12_css_position_tests.csv": "SP12",
    "sp12_css_flexbox_tests.csv": "SP12",
    "sp12_css_multicol_tests.csv": "SP12",
    "sp12_css_overflow_tests.csv": "SP12",
    "sp12_css_sizing_tests.csv": "SP12",
    "sp12_css_break_tests.csv": "SP12",
    "sp12_blink_block_tests.csv": "SP12",
    "sp12_css_backgrounds_tests.csv": "SP12",
    "sp12_css_values_tests.csv": "SP12",
    "sp12_blink_borders_tests.csv": "SP12",
    "sp12_blink_overflow_tests.csv": "SP12",
    "sp13_css_inline_tests.csv": "SP13",
    "sp13_css_pseudo_tests.csv": "SP13",
    "sp13_blink_inline_tests.csv": "SP13",
}

FEATURE_CSV_AREA = {
    "sp11_text_features.csv": "SP11",
    "sp12_block_features.csv": "SP12",
    "sp13_inline_features.csv": "SP13",
}

AREA_LABELS = {
    "SP11": "Text Rendering",
    "SP12": "Block Layout",
    "SP13": "Inline Layout",
}

SUMMARY_JSON = os.path.join(SCRIPT_DIR, "data", "pixel_comparison", "results", "summary.json")


def load_pixel_summary():
    """Load authoritative pixel results from summary.json.

    Returns a list of test dicts or None if the file is missing.
    """
    if not os.path.isfile(SUMMARY_JSON):
        return None
    with open(SUMMARY_JSON) as f:
        return json.load(f).get("tests", [])


def load_chromium_csvs():
    """Load all chromium test CSVs into a list of dicts."""
    rows = []
    for filename, area in CHROMIUM_CSV_AREA.items():
        filepath = os.path.join(CHROMIUM_DIR, filename)
        if not os.path.exists(filepath):
            continue
        with open(filepath, newline="") as f:
            reader = csv.DictReader(f)
            for row in reader:
                row["_area"] = area
                row["_source"] = filename
                rows.append(row)
    return rows


def load_feature_csvs():
    """Load all feature matrix CSVs into a list of dicts."""
    rows = []
    for filename, area in FEATURE_CSV_AREA.items():
        filepath = os.path.join(FEATURE_DIR, filename)
        if not os.path.exists(filepath):
            continue
        with open(filepath, newline="") as f:
            reader = csv.DictReader(f)
            for row in reader:
                row["_area"] = area
                row["_source"] = filename
                rows.append(row)
    return rows


def print_header(title):
    width = 60
    print("═" * width)
    print(f"  {title}")
    print("═" * width)
    print()


def cmd_summary():
    """Print the full dashboard summary."""
    chromium = load_chromium_csvs()
    features = load_feature_csvs()

    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    print_header(f"OPEN UI ACCOUNTABILITY REPORT — {now}")

    # ── Chromium Test Coverage ───────────────────────────────
    print("CHROMIUM TEST COVERAGE")
    print("──────────────────────")

    area_stats = defaultdict(lambda: {"total": 0, "ported": 0, "passing": 0, "failing": 0})
    for row in chromium:
        area = row["_area"]
        area_stats[area]["total"] += 1
        if row.get("port_status") == "ported":
            area_stats[area]["ported"] += 1
        if row.get("pass_fail") == "pass":
            area_stats[area]["passing"] += 1
        if row.get("pass_fail") == "fail":
            area_stats[area]["failing"] += 1

    grand_total = 0
    grand_ported = 0
    grand_passing = 0
    for area in ["SP11", "SP12", "SP13"]:
        s = area_stats[area]
        pct = (s["ported"] / s["total"] * 100) if s["total"] > 0 else 0.0
        label = f"{area} {AREA_LABELS.get(area, '')}:"
        print(
            f"  {label:<24} {s['ported']:>5} / {s['total']:<6} ({pct:>5.1f}%) ported"
            f"  |  {s['passing']:>5} / {s['ported']:<5} passing"
        )
        grand_total += s["total"]
        grand_ported += s["ported"]
        grand_passing += s["passing"]

    print("  " + "─" * 56)
    gpct = (grand_ported / grand_total * 100) if grand_total > 0 else 0.0
    print(
        f"  {'TOTAL:':<24} {grand_ported:>5} / {grand_total:<6} ({gpct:>5.1f}%) ported"
        f"  |  {grand_passing:>5} / {grand_ported:<5} passing"
    )
    print()

    # ── Per-CSV Breakdown ────────────────────────────────────
    print("CHROMIUM TEST COVERAGE BY FILE")
    print("──────────────────────────────")
    csv_stats = defaultdict(lambda: {"total": 0, "ported": 0, "passing": 0})
    for row in chromium:
        src = row["_source"]
        csv_stats[src]["total"] += 1
        if row.get("port_status") == "ported":
            csv_stats[src]["ported"] += 1
        if row.get("pass_fail") == "pass":
            csv_stats[src]["passing"] += 1

    for src in sorted(csv_stats.keys()):
        s = csv_stats[src]
        pct = (s["ported"] / s["total"] * 100) if s["total"] > 0 else 0.0
        print(f"  {src:<44} {s['ported']:>5}/{s['total']:<5} ({pct:>5.1f}%)")
    print()

    # ── Feature Implementation ───────────────────────────────
    if features:
        print("FEATURE IMPLEMENTATION")
        print("──────────────────────")

        feat_stats = defaultdict(
            lambda: {
                "total": 0,
                "implemented": 0,
                "partial": 0,
                "not_impl": 0,
                "tested": 0,
                "pixel_compared": 0,
                "pixel_pass": 0,
                "pixel_fail": 0,
                "perf_compared": 0,
            }
        )
        for row in features:
            area = row.get("sp", row.get("_area", ""))
            feat_stats[area]["total"] += 1
            impl = row.get("implemented", "").lower()
            if impl == "yes":
                feat_stats[area]["implemented"] += 1
            elif impl == "partial":
                feat_stats[area]["partial"] += 1
            else:
                feat_stats[area]["not_impl"] += 1
            if row.get("has_unit_test", "").lower() == "yes":
                feat_stats[area]["tested"] += 1
            if row.get("pixel_compared_with_chromium", "").lower() == "yes":
                feat_stats[area]["pixel_compared"] += 1
            if row.get("pixel_result", "").lower() == "pass":
                feat_stats[area]["pixel_pass"] += 1
            if row.get("pixel_result", "").lower() == "fail":
                feat_stats[area]["pixel_fail"] += 1
            if row.get("perf_compared_with_chromium", "").lower() == "yes":
                feat_stats[area]["perf_compared"] += 1

        grand_feat = 0
        grand_impl = 0
        grand_partial = 0
        grand_pixel_comp = 0
        grand_pixel_pass = 0
        for area in ["SP11", "SP12", "SP13"]:
            s = feat_stats[area]
            full_impl = s["implemented"] + s["partial"]
            pct = (full_impl / s["total"] * 100) if s["total"] > 0 else 0.0
            print(
                f"  {area}: {s['implemented']} full + {s['partial']} partial"
                f" / {s['total']} variants ({pct:.1f}%)"
                f"  |  {s['tested']} unit-tested"
            )
            grand_feat += s["total"]
            grand_impl += s["implemented"]
            grand_partial += s["partial"]
            grand_pixel_comp += s["pixel_compared"]
            grand_pixel_pass += s["pixel_pass"]
        print()

        # ── Pixel Comparison ─────────────────────────────────
        print("PIXEL COMPARISON WITH CHROMIUM")
        print("──────────────────────────────")

        # Read authoritative pixel results from summary.json if available
        pixel_tests = load_pixel_summary()
        if pixel_tests is not None:
            pixel_summary = {"tests": pixel_tests}
            sp_pixel = {"sp11": {"total": 0, "pass": 0, "fail": 0, "error": 0},
                        "sp12": {"total": 0, "pass": 0, "fail": 0, "error": 0},
                        "sp13": {"total": 0, "pass": 0, "fail": 0, "error": 0}}
            for t in pixel_summary.get("tests", []):
                sp = t["id"].split("/")[0]
                if sp in sp_pixel:
                    sp_pixel[sp]["total"] += 1
                    if t["status"] == "pass":
                        sp_pixel[sp]["pass"] += 1
                    elif t["status"] == "error":
                        sp_pixel[sp]["error"] += 1
                    else:
                        sp_pixel[sp]["fail"] += 1
            gt = sum(s["total"] for s in sp_pixel.values())
            gp = sum(s["pass"] for s in sp_pixel.values())
            gf = sum(s["fail"] for s in sp_pixel.values())
            ge = sum(s["error"] for s in sp_pixel.values())
            print("  (from summary.json — authoritative)")
            for sp_name in ["sp11", "sp12", "sp13"]:
                s = sp_pixel[sp_name]
                label = sp_name.upper()
                print(f"  {label}: {s['pass']:>4} / {s['total']:<4} pass"
                      f"  |  {s['fail']} fail  {s['error']} error")
            prate = (gp / gt * 100) if gt > 0 else 0.0
            print(f"\n  TOTAL: {gp}/{gt} pass ({prate:.1f}%)")
        else:
            # Fall back to feature matrix data
            for area in ["SP11", "SP12", "SP13"]:
                s = feat_stats[area]
                ppct = (s["pixel_compared"] / s["total"] * 100) if s["total"] > 0 else 0.0
                print(
                    f"  {area}: {s['pixel_compared']:>4} / {s['total']:<4} compared ({ppct:.1f}%)"
                    f"  |  {s['pixel_pass']} pass  {s['pixel_fail']} fail"
                )
            print(f"\n  TOTAL: {grand_pixel_comp} / {grand_feat} compared"
                  f"  |  {grand_pixel_pass} passing")
        print()

        # ── Performance Comparison ───────────────────────────
        print("PERFORMANCE COMPARISON WITH CHROMIUM")
        print("────────────────────────────────────")
        for area in ["SP11", "SP12", "SP13"]:
            s = feat_stats[area]
            print(f"  {area}: {s['perf_compared']:>4} / {s['total']:<4} benchmarked")

        # Check Chromium perf test inventory
        perf_csv = os.path.join(CHROMIUM_DIR, "perf_layout_tests.csv")
        if os.path.isfile(perf_csv):
            with open(perf_csv) as f:
                perf_rows = list(csv.DictReader(f))
            perf_total = len(perf_rows)
            perf_ours = sum(1 for r in perf_rows if r.get("our_benchmark", "").strip())
            print(f"  Chromium perf tests tracked: {perf_ours}/{perf_total} with our benchmarks")
        print()

        # ── Unimplemented Features ───────────────────────────
        not_impl = [r for r in features if r.get("implemented", "").lower() == "no"]
        if not_impl:
            print("UNIMPLEMENTED FEATURES (CRITICAL)")
            print("─────────────────────────────────")
            # Group by feature
            by_feature = defaultdict(list)
            for r in not_impl:
                key = f"{r.get('sp', '')}: {r.get('feature', '')}"
                variant = r.get("variant", r.get("sub_feature", ""))
                by_feature[key].append(variant)
            for key in sorted(by_feature.keys()):
                variants = by_feature[key]
                if len(variants) <= 3:
                    print(f"  {key} — {', '.join(variants)}")
                else:
                    print(f"  {key} — {len(variants)} variants unimplemented")
            print()


def cmd_area(area_filter):
    """Show detailed breakdown for one area."""
    area_filter = area_filter.upper()
    if area_filter not in AREA_LABELS:
        print(f"Unknown area: {area_filter}. Use sp11, sp12, or sp13.", file=sys.stderr)
        sys.exit(1)

    chromium = [r for r in load_chromium_csvs() if r["_area"] == area_filter]
    features = [r for r in load_feature_csvs() if r.get("sp", r.get("_area", "")).upper() == area_filter]

    print_header(f"{area_filter} {AREA_LABELS[area_filter]} — Detail Report")

    # Chromium tests by property
    print(f"CHROMIUM TESTS ({len(chromium)} total)")
    print("─" * 50)
    by_prop = defaultdict(lambda: {"total": 0, "ported": 0, "passing": 0})
    for r in chromium:
        prop = r.get("css_property", "unknown")
        by_prop[prop]["total"] += 1
        if r.get("port_status") == "ported":
            by_prop[prop]["ported"] += 1
        if r.get("pass_fail") == "pass":
            by_prop[prop]["passing"] += 1

    for prop in sorted(by_prop.keys()):
        s = by_prop[prop]
        pct = (s["ported"] / s["total"] * 100) if s["total"] > 0 else 0.0
        print(f"  {prop:<36} {s['ported']:>4}/{s['total']:<4} ({pct:>5.1f}%) ported  {s['passing']:>4} passing")
    print()

    # Features
    if features:
        print(f"FEATURE VARIANTS ({len(features)} total)")
        print("─" * 50)
        by_feat = defaultdict(list)
        for r in features:
            by_feat[r.get("feature", "unknown")].append(r)

        for feat in sorted(by_feat.keys()):
            rows = by_feat[feat]
            impl = sum(1 for r in rows if r.get("implemented", "").lower() in ("yes", "partial"))
            pixel = sum(1 for r in rows if r.get("pixel_compared_with_chromium", "").lower() == "yes")
            pixel_pass = sum(1 for r in rows if r.get("pixel_result", "").lower() == "pass")
            print(f"  {feat:<30} {impl:>3}/{len(rows):<3} impl  {pixel:>3}/{len(rows):<3} pixel-compared  {pixel_pass} pass")
        print()

    # Pixel comparison from summary.json (authoritative)
    pixel_tests = load_pixel_summary()
    if pixel_tests is not None:
        area_pixel = [t for t in pixel_tests if t["id"].split("/")[0].upper() == area_filter]
        if area_pixel:
            print(f"PIXEL COMPARISON ({len(area_pixel)} tests from summary.json)")
            print("─" * 50)
            p = sum(1 for t in area_pixel if t["status"] == "pass")
            fl = sum(1 for t in area_pixel if t["status"] == "fail")
            e = sum(1 for t in area_pixel if t["status"] == "error")
            print(f"  {p} pass  |  {fl} fail  |  {e} error")
            for t in area_pixel:
                if t["status"] != "pass":
                    mismatch = t.get("mismatch_pct", 0.0)
                    label = "error" if t["status"] == "error" else f"{mismatch:.2f}% mismatch"
                    print(f"    {t['id']} — {label}")
            print()


def cmd_failing():
    """List all failing ported tests and pixel failures."""
    chromium = load_chromium_csvs()
    failing = [r for r in chromium if r.get("pass_fail") == "fail"]

    # Pixel failures from authoritative summary.json
    pixel_tests = load_pixel_summary()
    pixel_failing = []
    if pixel_tests is not None:
        pixel_failing = [t for t in pixel_tests if t["status"] in ("fail", "error")]

    total = len(failing) + len(pixel_failing)
    print_header(f"FAILING TESTS ({total} total)")

    if failing:
        print(f"  CHROMIUM TEST FAILURES ({len(failing)}):")
        for r in failing:
            print(f"    [{r['_area']}] {r.get('chromium_test_path', 'unknown')}")
            if r.get("notes"):
                print(f"           Note: {r['notes']}")
        print()

    if pixel_failing:
        print(f"  PIXEL COMPARISON FAILURES ({len(pixel_failing)}):")
        for t in pixel_failing:
            sp = t["id"].split("/")[0].upper()
            mismatch = t.get("mismatch_pct", 0.0)
            label = "error" if t["status"] == "error" else f"{mismatch:.2f}% mismatch"
            print(f"    [{sp}] {t['id']} — {label}")
        print()

    if total == 0:
        print("  No failing tests found.")
        print()


def cmd_not_compared():
    """List features without pixel comparison."""
    features = load_feature_csvs()
    impl_not_compared = [
        r for r in features
        if r.get("implemented", "").lower() in ("yes", "partial")
        and r.get("pixel_compared_with_chromium", "").lower() != "yes"
    ]

    print_header(f"IMPLEMENTED BUT NOT PIXEL-COMPARED ({len(impl_not_compared)} variants)")
    for r in impl_not_compared:
        feat = r.get("feature", "unknown")
        variant = r.get("variant", r.get("sub_feature", ""))
        area = r.get("sp", r.get("_area", ""))
        print(f"  [{area}] {feat}: {variant}")
    print()


def cmd_not_implemented():
    """List features not yet implemented."""
    features = load_feature_csvs()
    not_impl = [r for r in features if r.get("implemented", "").lower() == "no"]

    print_header(f"NOT IMPLEMENTED ({len(not_impl)} variants)")
    by_area = defaultdict(list)
    for r in not_impl:
        area = r.get("sp", r.get("_area", ""))
        by_area[area].append(r)

    for area in ["SP11", "SP12", "SP13"]:
        items = by_area.get(area, [])
        if items:
            print(f"  {area} ({len(items)} variants):")
            for r in items:
                feat = r.get("feature", "unknown")
                variant = r.get("variant", r.get("sub_feature", ""))
                note = r.get("notes", "")
                line = f"    {feat}: {variant}"
                if note:
                    line += f"  — {note}"
                print(line)
            print()


def cmd_csv_output():
    """Machine-readable summary as CSV."""
    chromium = load_chromium_csvs()
    features = load_feature_csvs()
    pixel_tests = load_pixel_summary()

    # Pre-aggregate pixel data from summary.json by area
    pixel_by_area = defaultdict(lambda: {"compared": 0, "passing": 0})
    if pixel_tests is not None:
        for t in pixel_tests:
            sp = t["id"].split("/")[0].upper()
            pixel_by_area[sp]["compared"] += 1
            if t["status"] == "pass":
                pixel_by_area[sp]["passing"] += 1

    writer = csv.writer(sys.stdout)
    writer.writerow([
        "area", "chromium_tests_total", "chromium_tests_ported", "chromium_tests_passing",
        "feature_variants_total", "feature_variants_implemented",
        "pixel_compared", "pixel_passing",
        "perf_compared",
    ])

    for area in ["SP11", "SP12", "SP13"]:
        c_rows = [r for r in chromium if r["_area"] == area]
        f_rows = [r for r in features if r.get("sp", r.get("_area", "")).upper() == area]

        c_total = len(c_rows)
        c_ported = sum(1 for r in c_rows if r.get("port_status") == "ported")
        c_passing = sum(1 for r in c_rows if r.get("pass_fail") == "pass")

        f_total = len(f_rows)
        f_impl = sum(1 for r in f_rows if r.get("implemented", "").lower() in ("yes", "partial"))
        f_perf = sum(1 for r in f_rows if r.get("perf_compared_with_chromium", "").lower() == "yes")

        # Pixel stats from summary.json (authoritative) or fall back to CSVs
        if pixel_tests is not None:
            f_pixel = pixel_by_area[area]["compared"]
            f_pixel_pass = pixel_by_area[area]["passing"]
        else:
            f_pixel = sum(1 for r in f_rows if r.get("pixel_compared_with_chromium", "").lower() == "yes")
            f_pixel_pass = sum(1 for r in f_rows if r.get("pixel_result", "").lower() == "pass")

        writer.writerow([area, c_total, c_ported, c_passing, f_total, f_impl, f_pixel, f_pixel_pass, f_perf])


def main():
    parser = argparse.ArgumentParser(
        description="Open UI Accountability Report — reads CSV tracking data and reports honest status."
    )
    parser.add_argument("--area", type=str, help="Show detailed breakdown for an area (sp11, sp12, sp13)")
    parser.add_argument("--failing", action="store_true", help="List all failing ported tests")
    parser.add_argument("--not-compared", action="store_true", help="List implemented features without pixel comparison")
    parser.add_argument("--not-implemented", action="store_true", help="List features not yet implemented")
    parser.add_argument("--csv", action="store_true", help="Machine-readable CSV output")

    args = parser.parse_args()

    if args.area:
        cmd_area(args.area)
    elif args.failing:
        cmd_failing()
    elif args.not_compared:
        cmd_not_compared()
    elif args.not_implemented:
        cmd_not_implemented()
    elif args.csv:
        cmd_csv_output()
    else:
        cmd_summary()


if __name__ == "__main__":
    main()
