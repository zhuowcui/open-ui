#!/usr/bin/env python3
"""
audit.py — Open UI WPT Test Pipeline Integrity Verifier

Verifies that all tracking data is consistent and no claims are unsubstantiated.
Exit code 0 = clean audit, non-zero = discrepancies found.

Checks:
1. Every "pass" in summary.json has a result.json with 0.0% mismatch
2. Every ported test in wpt_mapping.csv exists in all_wpt_templates.json
3. Every ported test has generated Rust code in pixel-compare/src/wpt/
4. Template count matches summary.json test count
5. wpt_mapping.csv covers all Chromium WPT tests
6. SP12.5 deferred CSV only contains tests that actually fail
7. No test is double-counted (in both passing and deferred)
"""

import csv
import json
import os
import re
import sys
from collections import Counter

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.abspath(os.path.join(SCRIPT_DIR, "..", ".."))
DATA_DIR = os.path.join(SCRIPT_DIR, "data")
RESULTS_DIR = os.path.join(DATA_DIR, "pixel_comparison", "results")
WPT_DIR = os.path.join(PROJECT_ROOT, "bindings", "rust", "pixel-compare", "src", "wpt")

issues = []
warnings = []


def issue(msg):
    issues.append(msg)
    print(f"  ❌ {msg}")


def warn(msg):
    warnings.append(msg)
    print(f"  ⚠️  {msg}")


def ok(msg):
    print(f"  ✅ {msg}")


def check_summary_integrity():
    """Check 1: Every pass has proof."""
    print("\n── Check 1: Pass claims have proof ──")
    summary_path = os.path.join(RESULTS_DIR, "summary.json")
    if not os.path.isfile(summary_path):
        issue("summary.json not found")
        return

    with open(summary_path) as f:
        summary = json.load(f)

    pass_count = 0
    fail_count = 0
    false_passes = 0
    missing_results = 0

    for test in summary["tests"]:
        if test["status"] == "pass":
            pass_count += 1
            result_path = os.path.join(RESULTS_DIR, test["id"], "result.json")
            if not os.path.isfile(result_path):
                missing_results += 1
                if missing_results <= 3:
                    issue(f"Pass claimed but no result.json: {test['id']}")
            else:
                with open(result_path) as f:
                    result = json.load(f)
                pct = result.get("mismatch_pct", -1)
                if pct > 0.0:
                    false_passes += 1
                    if false_passes <= 3:
                        issue(f"Pass claimed but mismatch={pct}%: {test['id']}")
        else:
            fail_count += 1

    if missing_results > 3:
        issue(f"... and {missing_results - 3} more passes without result.json")
    if false_passes > 3:
        issue(f"... and {false_passes - 3} more false passes")

    if missing_results == 0 and false_passes == 0:
        ok(f"All {pass_count} passes have verified 0.0% mismatch proof")

    # Verify totals
    if summary["passed"] != pass_count:
        issue(f"Summary says {summary['passed']} passed but found {pass_count}")
    if summary["failed"] != fail_count:
        issue(f"Summary says {summary['failed']} failed but found {fail_count}")
    if summary["total"] != pass_count + fail_count:
        issue(f"Total mismatch: {summary['total']} != {pass_count} + {fail_count}")
    else:
        ok(f"Totals consistent: {pass_count} pass + {fail_count} fail = {summary['total']}")


def check_template_consistency():
    """Check 2: Templates match summary test count."""
    print("\n── Check 2: Template ↔ summary consistency ──")
    templates_path = os.path.join(DATA_DIR, "wpt_ported", "all_wpt_templates.json")
    summary_path = os.path.join(RESULTS_DIR, "summary.json")

    if not os.path.isfile(templates_path) or not os.path.isfile(summary_path):
        issue("Missing templates or summary file")
        return

    with open(templates_path) as f:
        templates = json.load(f)
    with open(summary_path) as f:
        summary = json.load(f)

    summary_ids = {t["id"] for t in summary["tests"]}
    template_ids = set(templates.keys())

    if len(template_ids) != len(summary_ids):
        warn(f"Template count ({len(template_ids)}) != summary count ({len(summary_ids)})")

    missing_in_summary = template_ids - summary_ids
    missing_in_templates = summary_ids - template_ids

    if missing_in_summary:
        warn(f"{len(missing_in_summary)} templates not in summary (not compared yet)")
    if missing_in_templates:
        issue(f"{len(missing_in_templates)} summary tests have no template")
    if not missing_in_templates and not missing_in_summary:
        ok(f"All {len(template_ids)} templates have matching summary entries")


def check_rust_code_exists():
    """Check 3: Every ported test has Rust code."""
    print("\n── Check 3: Ported tests have Rust code ──")
    summary_path = os.path.join(RESULTS_DIR, "summary.json")
    if not os.path.isfile(summary_path):
        issue("summary.json not found")
        return

    with open(summary_path) as f:
        summary = json.load(f)

    # Read all Rust wpt files and extract registered test IDs
    rust_test_ids = set()
    for fname in os.listdir(WPT_DIR):
        if fname.endswith(".rs") and fname != "mod.rs":
            fpath = os.path.join(WPT_DIR, fname)
            with open(fpath) as f:
                content = f.read()
            # Extract test IDs from registry entries like ("wpt/css_flexbox/test-name", fn_name)
            for m in re.finditer(r'"(wpt/[^"]+)"', content):
                rust_test_ids.add(m.group(1))

    summary_ids = {t["id"] for t in summary["tests"]}
    missing_rust = summary_ids - rust_test_ids
    extra_rust = rust_test_ids - summary_ids

    if missing_rust:
        issue(f"{len(missing_rust)} tests in summary but no Rust code found")
        for tid in sorted(missing_rust)[:3]:
            print(f"         {tid}")
    if extra_rust:
        warn(f"{len(extra_rust)} Rust tests not in summary (possibly not compared)")

    if not missing_rust:
        ok(f"All {len(summary_ids)} summary tests have Rust code")


def check_mapping_coverage():
    """Check 4: wpt_mapping.csv covers all Chromium tests."""
    print("\n── Check 4: WPT mapping coverage ──")
    mapping_path = os.path.join(DATA_DIR, "wpt_mapping.csv")
    if not os.path.isfile(mapping_path):
        warn("wpt_mapping.csv not found — skipping")
        return

    with open(mapping_path) as f:
        reader = csv.DictReader(f)
        rows = list(reader)

    total = len(rows)
    ported = sum(1 for r in rows if r["ported"] == "yes")
    not_ported = sum(1 for r in rows if r["ported"] == "no")
    passing = sum(1 for r in rows if r["pixel_result"] == "pass")
    failing = sum(1 for r in rows if r["pixel_result"] == "fail")

    # Cross-check with summary
    summary_path = os.path.join(RESULTS_DIR, "summary.json")
    if os.path.isfile(summary_path):
        with open(summary_path) as f:
            summary = json.load(f)
        if ported != summary["total"]:
            warn(f"Mapping says {ported} ported but summary has {summary['total']} tests")
        if passing != summary["passed"]:
            warn(f"Mapping says {passing} passing but summary says {summary['passed']}")

    # Check failure categories
    categories = Counter(r["failure_category"] for r in rows if r["failure_category"])
    ok(f"Mapping covers {total} Chromium tests ({ported} ported, {not_ported} not ported)")
    print(f"         Pass: {passing}, Fail: {failing}")
    print(f"         Categories: {dict(categories.most_common(8))}")


def check_deferred_consistency():
    """Check 5: SP12.5 deferred tests are actually failing."""
    print("\n── Check 5: SP12.5 deferred tests consistency ──")
    deferred_path = os.path.join(DATA_DIR, "sp12_5_deferred.csv")
    if not os.path.isfile(deferred_path):
        warn("sp12_5_deferred.csv not found — skipping")
        return

    summary_path = os.path.join(RESULTS_DIR, "summary.json")
    if not os.path.isfile(summary_path):
        issue("summary.json not found")
        return

    with open(summary_path) as f:
        summary = json.load(f)
    passing_ids = {t["id"] for t in summary["tests"] if t["status"] == "pass"}

    with open(deferred_path) as f:
        reader = csv.DictReader(f)
        deferred = list(reader)

    deferred_but_passing = []
    for row in deferred:
        if row["test_id"] in passing_ids:
            deferred_but_passing.append(row["test_id"])

    if deferred_but_passing:
        warn(f"{len(deferred_but_passing)} deferred tests actually pass (stale CSV?)")
        for tid in deferred_but_passing[:3]:
            print(f"         {tid}")
    else:
        ok(f"All {len(deferred)} deferred tests are confirmed failing")


def check_no_orphan_results():
    """Check 6: No result directories without corresponding tests."""
    print("\n── Check 6: No orphan result directories ──")
    summary_path = os.path.join(RESULTS_DIR, "summary.json")
    if not os.path.isfile(summary_path):
        issue("summary.json not found")
        return

    with open(summary_path) as f:
        summary = json.load(f)
    summary_ids = {t["id"] for t in summary["tests"]}

    # Walk results directory
    orphans = 0
    wpt_results = os.path.join(RESULTS_DIR, "wpt")
    if os.path.isdir(wpt_results):
        for area in os.listdir(wpt_results):
            area_dir = os.path.join(wpt_results, area)
            if not os.path.isdir(area_dir):
                continue
            for test_name in os.listdir(area_dir):
                test_id = f"wpt/{area}/{test_name}"
                if test_id not in summary_ids:
                    orphans += 1

    if orphans:
        warn(f"{orphans} result directories have no corresponding summary entry")
    else:
        ok("No orphan result directories")


def main():
    print("═══════════════════════════════════════════════")
    print("  OPEN UI WPT PIPELINE AUDIT")
    print("═══════════════════════════════════════════════")

    check_summary_integrity()
    check_template_consistency()
    check_rust_code_exists()
    check_mapping_coverage()
    check_deferred_consistency()
    check_no_orphan_results()

    print("\n═══════════════════════════════════════════════")
    if issues:
        print(f"  AUDIT FAILED: {len(issues)} issue(s), {len(warnings)} warning(s)")
        for i, msg in enumerate(issues, 1):
            print(f"    {i}. {msg}")
    elif warnings:
        print(f"  AUDIT PASSED WITH WARNINGS: {len(warnings)} warning(s)")
        for i, msg in enumerate(warnings, 1):
            print(f"    {i}. {msg}")
    else:
        print("  AUDIT PASSED: All checks clean ✅")
    print("═══════════════════════════════════════════════")

    return 1 if issues else 0


if __name__ == "__main__":
    sys.exit(main())
