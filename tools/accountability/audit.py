#!/usr/bin/env python3
"""
audit.py — Open UI WPT Test Pipeline Integrity Verifier

Verifies that all tracking data is consistent and no claims are unsubstantiated.
Exit code 0 = clean audit, non-zero = discrepancies found.

Checks:
1. Every "pass" has result.json (status="pass", mismatch_pct=0.0) + PNGs exist.
   Also validates: no duplicate IDs, valid status values.
2. Template ↔ summary consistency (mismatches are ERRORS)
3. Every ported test has Rust code with registry entries (excluding comments)
4. wpt_mapping.csv cross-checked with summary.json (accounting identity enforced)
5. SP12.5 deferred CSV tests exist in summary AND are failing
6. No orphan result directories (ERRORS)
7. Mapping ↔ deferred classification cross-check
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

# Import from shared_detectors to stay in sync automatically
sys.path.insert(0, SCRIPT_DIR)
from shared_detectors import CATEGORY_FOR_DEP

VALID_FAILURE_CATEGORIES = set(CATEGORY_FOR_DEP.values()) | {"sp12_layout_bug", "not_ported"}

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
    """Check 1: Every pass has proof — result.json with status=pass, mismatch_pct=0.0, PNGs exist."""
    print("\n── Check 1: Pass claims have proof (with image verification) ──")
    summary_path = os.path.join(RESULTS_DIR, "summary.json")
    if not os.path.isfile(summary_path):
        issue("summary.json not found")
        return

    with open(summary_path) as f:
        summary = json.load(f)

    # Check for duplicate test IDs
    all_ids = [t["id"] for t in summary["tests"]]
    id_counts = Counter(all_ids)
    dupes = {tid: cnt for tid, cnt in id_counts.items() if cnt > 1}
    if dupes:
        issue(f"{len(dupes)} duplicate test IDs in summary.json")
        for tid in sorted(dupes)[:3]:
            print(f"         {tid} (×{dupes[tid]})")

    # Validate status values
    valid_statuses = {"pass", "fail"}
    invalid_statuses = {t["status"] for t in summary["tests"]} - valid_statuses
    if invalid_statuses:
        issue(f"Invalid status values in summary.json: {invalid_statuses}")

    pass_count = 0
    fail_count = 0
    proof_failures = 0
    missing_results = 0
    missing_images = 0

    for test in summary["tests"]:
        if test["status"] == "pass":
            pass_count += 1
            result_dir = os.path.join(RESULTS_DIR, test["id"])
            result_path = os.path.join(result_dir, "result.json")

            if not os.path.isfile(result_path):
                missing_results += 1
                if missing_results <= 3:
                    issue(f"Pass claimed but no result.json: {test['id']}")
                continue

            with open(result_path) as f:
                result = json.load(f)

            # Each test triggers at most one proof failure (no double-counting)
            has_proof_issue = False

            # Strict check: mismatch_pct must be present AND exactly 0.0
            pct = result.get("mismatch_pct")
            if pct is None:
                has_proof_issue = True
                proof_failures += 1
                if proof_failures <= 3:
                    issue(f"Pass claimed but mismatch_pct missing: {test['id']}")
            elif pct != 0.0:
                has_proof_issue = True
                proof_failures += 1
                if proof_failures <= 3:
                    issue(f"Pass claimed but mismatch={pct}%: {test['id']}")

            # Verify status field in result.json matches
            if not has_proof_issue:
                result_status = result.get("status")
                if result_status != "pass":
                    has_proof_issue = True
                    proof_failures += 1
                    if proof_failures <= 3:
                        issue(f"Pass claimed but result.json status='{result_status}': {test['id']}")

            # Verify both PNG screenshots exist
            ours_png = os.path.join(result_dir, "openui.png")
            chrome_png = os.path.join(result_dir, "chromium.png")
            if not os.path.isfile(ours_png) or not os.path.isfile(chrome_png):
                missing_images += 1
                if missing_images <= 3:
                    issue(f"Pass claimed but PNG screenshot(s) missing: {test['id']}")
        else:
            fail_count += 1

    if missing_results > 3:
        issue(f"... and {missing_results - 3} more passes without result.json")
    if proof_failures > 3:
        issue(f"... and {proof_failures - 3} more proof failures")
    if missing_images > 3:
        issue(f"... and {missing_images - 3} more passes with missing PNGs")

    if missing_results == 0 and proof_failures == 0 and missing_images == 0:
        ok(f"All {pass_count} passes have verified 0.0% mismatch + PNG proof")

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
    """Check 2: Templates match summary test count (mismatches are ERRORS)."""
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

    missing_in_summary = template_ids - summary_ids
    missing_in_templates = summary_ids - template_ids

    if missing_in_summary:
        issue(f"{len(missing_in_summary)} templates not in summary (comparison incomplete)")
        for tid in sorted(missing_in_summary)[:3]:
            print(f"         {tid}")
    if missing_in_templates:
        issue(f"{len(missing_in_templates)} summary tests have no template (dropped tests)")
        for tid in sorted(missing_in_templates)[:3]:
            print(f"         {tid}")
    if not missing_in_templates and not missing_in_summary:
        ok(f"All {len(template_ids)} templates have matching summary entries")
    if len(template_ids) != len(summary_ids):
        issue(f"Template count ({len(template_ids)}) != summary count ({len(summary_ids)})")


def check_rust_code_exists():
    """Check 3: Every ported test has Rust code with proper fn + registry entry."""
    print("\n── Check 3: Ported tests have Rust code ──")
    summary_path = os.path.join(RESULTS_DIR, "summary.json")
    if not os.path.isfile(summary_path):
        issue("summary.json not found")
        return

    with open(summary_path) as f:
        summary = json.load(f)

    # Extract test IDs from Rust registry entries, excluding comments
    rust_test_ids = set()
    for fname in os.listdir(WPT_DIR):
        if fname.endswith(".rs") and fname != "mod.rs":
            fpath = os.path.join(WPT_DIR, fname)
            with open(fpath) as f:
                lines = f.readlines()
            for line in lines:
                stripped = line.lstrip()
                if stripped.startswith("//") or stripped.startswith("/*"):
                    continue
                for m in re.finditer(r'\("(wpt/[^"]+)"', line):
                    rust_test_ids.add(m.group(1))

    summary_ids = {t["id"] for t in summary["tests"]}
    missing_rust = summary_ids - rust_test_ids
    extra_rust = rust_test_ids - summary_ids

    if missing_rust:
        issue(f"{len(missing_rust)} tests in summary but no Rust registry entry")
        for tid in sorted(missing_rust)[:3]:
            print(f"         {tid}")
    if extra_rust:
        warn(f"{len(extra_rust)} Rust tests not in summary (possibly not compared)")

    if not missing_rust:
        ok(f"All {len(summary_ids)} summary tests have Rust registry entries")


def check_mapping_coverage():
    """Check 4: wpt_mapping.csv cross-checked with summary + accounting identity."""
    print("\n── Check 4: WPT mapping coverage + accounting identity ──")
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

    # Check for duplicate test paths
    paths = [r["chromium_test_path"] for r in rows]
    dupes = [p for p, c in Counter(paths).items() if c > 1]
    if dupes:
        issue(f"{len(dupes)} duplicate chromium_test_path entries in mapping")
        for p in dupes[:3]:
            print(f"         {p}")

    # Validate failure categories (supports comma-separated multi-labels)
    invalid_cats = set()
    for r in rows:
        cat = r.get("failure_category", "").strip()
        if cat:
            for part in cat.split(","):
                part = part.strip()
                if part and part not in VALID_FAILURE_CATEGORIES:
                    invalid_cats.add(part)
    if invalid_cats:
        issue(f"Invalid failure categories: {invalid_cats}")
    else:
        ok("All failure categories are from the valid set")

    # Accounting identity: ported + not_ported = total
    if ported + not_ported != total:
        issue(f"Accounting: ported({ported}) + not_ported({not_ported}) != total({total})")
    else:
        ok(f"Accounting: {ported} ported + {not_ported} not_ported = {total} total")

    # Accounting identity: pass + fail = ported
    if passing + failing != ported:
        issue(f"Accounting: pass({passing}) + fail({failing}) != ported({ported})")
    else:
        ok(f"Accounting: {passing} pass + {failing} fail = {ported} ported")

    # Cross-check with summary (these are now ERRORS, not warnings)
    summary_path = os.path.join(RESULTS_DIR, "summary.json")
    if os.path.isfile(summary_path):
        with open(summary_path) as f:
            summary = json.load(f)
        if ported != summary["total"]:
            issue(f"Mapping says {ported} ported but summary has {summary['total']} tests")
        if passing != summary["passed"]:
            issue(f"Mapping says {passing} passing but summary says {summary['passed']}")
        if failing != summary["failed"]:
            issue(f"Mapping says {failing} failing but summary says {summary['failed']}")
        if ported == summary["total"] and passing == summary["passed"]:
            ok(f"Mapping ↔ summary cross-check passed")

    # Split multi-label categories for accurate counting
    categories = Counter()
    for r in rows:
        cat = r.get("failure_category", "").strip()
        if cat:
            for part in cat.split(","):
                part = part.strip()
                if part:
                    categories[part] += 1
    ok(f"Mapping covers {total} Chromium tests ({ported} ported, {not_ported} not ported)")
    print(f"         Pass: {passing}, Fail: {failing}")
    print(f"         Categories: {dict(categories.most_common(8))}")


def check_deferred_consistency():
    """Check 5: SP12.5 deferred tests exist in summary AND are failing."""
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
    all_ids = {t["id"] for t in summary["tests"]}
    passing_ids = {t["id"] for t in summary["tests"] if t["status"] == "pass"}

    with open(deferred_path) as f:
        reader = csv.DictReader(f)
        deferred = list(reader)

    deferred_but_passing = []
    deferred_not_found = []
    for row in deferred:
        tid = row["test_id"]
        if tid not in all_ids:
            deferred_not_found.append(tid)
        elif tid in passing_ids:
            deferred_but_passing.append(tid)

    if deferred_not_found:
        issue(f"{len(deferred_not_found)} deferred tests not found in summary (phantom tests)")
        for tid in deferred_not_found[:3]:
            print(f"         {tid}")

    if deferred_but_passing:
        issue(f"{len(deferred_but_passing)} deferred tests actually pass (stale CSV — regenerate)")
        for tid in deferred_but_passing[:3]:
            print(f"         {tid}")

    if not deferred_not_found and not deferred_but_passing:
        ok(f"All {len(deferred)} deferred tests exist in summary and are confirmed failing")

    # Cross-check deferred count with SP12.5 plan doc
    plan_path = os.path.join(PROJECT_ROOT, "docs", "SP12.5-PLAN.md")
    if os.path.isfile(plan_path):
        with open(plan_path) as f:
            plan_text = f.read()
        # Extract the count from "N WPT tests from SP12 scope"
        m = re.search(r"(\d+)\s+WPT tests from SP12 scope", plan_text)
        if m:
            plan_count = int(m.group(1))
            if plan_count != len(deferred):
                warn(f"SP12.5-PLAN.md says {plan_count} tests but CSV has {len(deferred)}")
            else:
                ok(f"Deferred count matches SP12.5-PLAN.md ({plan_count})")


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

    # Walk results directory recursively to handle any nesting depth
    orphans = 0
    wpt_results = os.path.join(RESULTS_DIR, "wpt")
    if os.path.isdir(wpt_results):
        for root, dirs, files in os.walk(wpt_results):
            if "result.json" in files:
                rel = os.path.relpath(root, RESULTS_DIR)
                if rel not in summary_ids:
                    orphans += 1

    if orphans:
        issue(f"{orphans} orphan result directories (no corresponding summary entry)")
    else:
        ok("No orphan result directories")


def check_classification_consistency():
    """Check 7: Mapping and deferred CSVs agree on classifications."""
    print("\n── Check 7: Mapping ↔ deferred classification consistency ──")
    mapping_path = os.path.join(DATA_DIR, "wpt_mapping.csv")
    deferred_path = os.path.join(DATA_DIR, "sp12_5_deferred.csv")

    if not os.path.isfile(mapping_path) or not os.path.isfile(deferred_path):
        warn("Missing mapping or deferred CSV — skipping cross-check")
        return

    # Load mapping: test_id → set of categories
    mapping_cats = {}
    with open(mapping_path) as f:
        for row in csv.DictReader(f):
            tid = row.get("our_test_id", "").strip()
            cat = row.get("failure_category", "").strip()
            if tid and cat:
                mapping_cats[tid] = set(p.strip() for p in cat.split(",") if p.strip())

    # Load deferred: test_id → set of dependency keys
    # Use CATEGORY_FOR_DEP from shared_detectors (single source of truth)
    deferred_cats = {}
    with open(deferred_path) as f:
        for row in csv.DictReader(f):
            tid = row["test_id"]
            deps = row.get("dependency", "").strip()
            if deps:
                cats = set()
                for d in deps.split(","):
                    d = d.strip()
                    if d in CATEGORY_FOR_DEP:
                        cats.add(CATEGORY_FOR_DEP[d])
                deferred_cats[tid] = cats

    # Cross-check: for every deferred test, its dependency categories
    # should be a subset of (or equal to) the mapping categories
    mismatches = 0
    for tid, dcats in deferred_cats.items():
        mcats = mapping_cats.get(tid, set())
        # Mapping might say "sp12_layout_bug" if no cross-SP deps detected,
        # but deferred says it HAS deps. That's an inconsistency.
        if dcats and not dcats.issubset(mcats):
            mismatches += 1

    if mismatches:
        warn(f"{mismatches} tests have mapping↔deferred classification disagreements")
    else:
        ok(f"Mapping and deferred classifications consistent for {len(deferred_cats)} tests")


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
    check_classification_consistency()

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
