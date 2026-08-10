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

VALID_FAILURE_CATEGORIES = set(CATEGORY_FOR_DEP.values()) | {"sp12_layout_bug", "sp13_fragmentation_architecture", "not_ported"}
TEXT_PORT_METADATA_CATEGORIES = {"reference_test", "non_visual_test"}
SP14_BASELINE_PATH = os.path.join(
    DATA_DIR, "wpt_ported", "sp14_w3_baseline_exact.json"
)
SP14_W3_PATH = os.path.join(DATA_DIR, "wpt_ported", "sp14_w3_targets.json")
SP14_W4_PATH = os.path.join(DATA_DIR, "wpt_ported", "sp14_w4_residuals.json")
SP14_EXPECTED_INVENTORY = 7673
SP14_EXPECTED_BASELINE = 2715
SP14_EXPECTED_W3 = 111
SP14_EXPECTED_W4 = 3934
SP15_BASELINE_PATH = os.path.join(
    DATA_DIR, "wpt_ported", "sp15_baseline_exact.json"
)
SP15_TARGETS_PATH = os.path.join(
    DATA_DIR, "wpt_ported", "sp15_actionable_targets.json"
)
SP15_RESIDUALS_PATH = os.path.join(
    DATA_DIR, "wpt_ported", "sp15_residual_dispositions.json"
)
SP15_RETIRED_CATEGORIES = {
    "needs_inline_box_decoration_break",
    "needs_clearing_break_after_floats",
    "needs_display_contents_style_element",
    "needs_display_contents_list_layout",
    "needs_root_body_layout",
}
SP15_EXPECTED_BASELINE = 2767
SP15_EXPECTED_TARGETS = 76
SP15_EXPECTED_RESIDUALS = 54
SP15_EXPECTED_RUNNABLE = 3566

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


def text_port_ownership_errors(rows, text_ported_tests):
    """Return manifest failures with stale or metadata-only ownership."""
    stale_text = []
    metadata_only = []
    for row in rows:
        test_id = row.get("our_test_id", "").strip()
        if test_id not in text_ported_tests or row.get("pixel_result") != "fail":
            continue
        categories = {
            part.strip()
            for part in row.get("failure_category", "").split(",")
            if part.strip()
        }
        if "needs_text" in categories:
            stale_text.append(test_id)
        if not (categories - TEXT_PORT_METADATA_CATEGORIES - {"needs_text"}):
            metadata_only.append(test_id)
    return stale_text, metadata_only


def canonical_mapping_id(row):
    """Return the canonical identity even when an unported row has no ID cell."""
    return f"wpt/{row.get('sp_area', '').strip()}/{row.get('test_name', '').strip()}"


def sp14_text_closure_errors(
    rows,
    summary_by_id,
    templates,
    text_ported_tests,
    baseline,
    w3,
    w4,
    *,
    enforce_frozen_counts=True,
    superseded_actionable=(),
    superseded_residuals=None,
):
    """Validate the complete W3/W4 closure against authoritative artifacts."""
    errors = []
    w4_ids = [item.get("test_id", "") for item in w4 if isinstance(item, dict)]
    if baseline != sorted(set(baseline)):
        errors.append("baseline ledger is not sorted and unique")
    if w3 != sorted(set(w3)):
        errors.append("W3 ledger is not sorted and unique")
    if len(w4_ids) != len(w4) or w4_ids != sorted(set(w4_ids)):
        errors.append("W4 ledger is malformed or not sorted and unique")
    if set(w3) & set(w4_ids):
        errors.append("W3 and W4 ledgers overlap")
    if enforce_frozen_counts:
        expected = (
            (len(rows), SP14_EXPECTED_INVENTORY, "inventory"),
            (len(baseline), SP14_EXPECTED_BASELINE, "baseline"),
            (len(w3), SP14_EXPECTED_W3, "W3"),
            (len(w4), SP14_EXPECTED_W4, "W4"),
            (len(set(w3) | set(w4_ids)), SP14_EXPECTED_W3 + SP14_EXPECTED_W4, "W3/W4 cover"),
        )
        for actual, wanted, label in expected:
            if actual != wanted:
                errors.append(f"{label} count {actual} != {wanted}")

    mapping_by_id = {}
    for row in rows:
        test_id = canonical_mapping_id(row)
        if test_id in mapping_by_id:
            errors.append(f"duplicate canonical mapping ID: {test_id}")
        mapping_by_id[test_id] = row
        if "needs_text" in {
            part.strip()
            for part in row.get("failure_category", "").split(",")
            if part.strip()
        }:
            errors.append(f"mapping retains needs_text: {test_id}")

    for test_id in baseline:
        result = summary_by_id.get(test_id)
        if not result or result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            errors.append(f"baseline exact pass regressed: {test_id}")

    for test_id in w3:
        row = mapping_by_id.get(test_id)
        result = summary_by_id.get(test_id)
        if not row or row.get("ported") != "yes" or row.get("our_test_id") != test_id:
            errors.append(f"W3 target is not ported: {test_id}")
        if test_id not in templates or test_id not in text_ported_tests:
            errors.append(f"W3 target is not fully manifested: {test_id}")
        if not result or result.get("status") == "error":
            errors.append(f"W3 target is not runnable without error: {test_id}")

    superseded_actionable = set(superseded_actionable)
    superseded_residuals = superseded_residuals or {}
    for item in w4:
        if not isinstance(item, dict):
            continue
        test_id = item.get("test_id", "")
        row = mapping_by_id.get(test_id)
        if test_id in superseded_actionable:
            result = summary_by_id.get(test_id)
            if not row or row.get("ported") != "yes" or row.get("our_test_id") != test_id:
                errors.append(f"SP15-superseded W4 target is not ported: {test_id}")
            if test_id not in templates or test_id not in text_ported_tests:
                errors.append(f"SP15-superseded W4 target is not manifested: {test_id}")
            if not result or result.get("status") == "error":
                errors.append(f"SP15-superseded W4 target is not runnable: {test_id}")
            continue
        if test_id in superseded_residuals:
            disposition = superseded_residuals[test_id]
            if not row or row.get("ported") != "no":
                errors.append(f"SP15-superseded W4 residual is not unported: {test_id}")
                continue
            mapped_owners = {
                part.strip()
                for part in row.get("failure_category", "").split(",")
                if part.strip()
            }
            if not set(disposition.get("owner_categories", [])) <= mapped_owners:
                errors.append(f"SP15 residual mapping ownership drift: {test_id}")
            if row.get("chromium_test_path") != disposition.get("chromium_test_path"):
                errors.append(f"SP15 residual Chromium path drift: {test_id}")
            if row.get("notes") != f"Porter deferred: {disposition.get('rejection_reason', '')}":
                errors.append(f"SP15 residual rejection reason drift: {test_id}")
            if test_id in templates or test_id in summary_by_id or test_id in text_ported_tests:
                errors.append(f"SP15 residual unexpectedly became runnable: {test_id}")
            continue
        owners = item.get("owner_categories")
        rejection_owner = item.get("rejection_owner", "")
        reason = item.get("rejection_reason", "")
        if (
            not isinstance(owners, list)
            or owners != sorted(set(owners))
            or not reason
            or rejection_owner not in set(owners or [])
            or "needs_text" in set(owners or [])
            or not (set(owners or []) - TEXT_PORT_METADATA_CATEGORIES)
        ):
            errors.append(f"W4 target lacks reason-backed functional ownership: {test_id}")
            continue
        if not row or row.get("ported") != "no":
            errors.append(f"W4 target is not unported: {test_id}")
            continue
        if row.get("chromium_test_path") != item.get("chromium_test_path"):
            errors.append(f"W4 Chromium path drift: {test_id}")
        mapped_owners = {
            part.strip()
            for part in row.get("failure_category", "").split(",")
            if part.strip()
        }
        if not set(owners) <= mapped_owners:
            errors.append(f"W4 mapping ownership drift: {test_id}")
        if row.get("notes") != f"Porter deferred: {reason}":
            errors.append(f"W4 rejection reason drift: {test_id}")
        if test_id in templates or test_id in summary_by_id or test_id in text_ported_tests:
            errors.append(f"W4 target unexpectedly became runnable: {test_id}")
    return errors


def sp15_closure_errors(
    rows, summary_by_id, templates, text_ported_tests, baseline, targets, residuals
):
    """Validate the frozen SP15 cover and its closed runnable snapshot."""
    errors = []
    residual_ids = [item.get("test_id", "") for item in residuals if isinstance(item, dict)]
    if baseline != sorted(set(baseline)) or len(baseline) != SP15_EXPECTED_BASELINE:
        errors.append("SP15 baseline ledger is not the frozen sorted 2,767-ID set")
    if targets != sorted(set(targets)) or len(targets) != SP15_EXPECTED_TARGETS:
        errors.append("SP15 actionable ledger is not the frozen sorted 76-ID set")
    if residual_ids != sorted(set(residual_ids)) or len(residuals) != SP15_EXPECTED_RESIDUALS:
        errors.append("SP15 residual ledger is malformed or not sorted and unique")
    if set(targets) & set(residual_ids) or len(set(targets) | set(residual_ids)) != 130:
        errors.append("SP15 actionable/residual ledgers are not a disjoint 130-ID cover")

    mapping_by_id = {canonical_mapping_id(row): row for row in rows}
    for test_id in baseline:
        result = summary_by_id.get(test_id)
        if not result or result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            errors.append(f"SP15 baseline exact pass regressed: {test_id}")
    for test_id in targets:
        row = mapping_by_id.get(test_id)
        result = summary_by_id.get(test_id)
        if not row or row.get("ported") != "yes" or row.get("our_test_id") != test_id:
            errors.append(f"SP15 actionable target is not ported: {test_id}")
        if test_id not in templates or test_id not in text_ported_tests:
            errors.append(f"SP15 actionable target is not fully manifested: {test_id}")
        if not result or result.get("status") == "error":
            errors.append(f"SP15 actionable target is not runnable without error: {test_id}")

    for item in residuals:
        if not isinstance(item, dict):
            continue
        test_id = item.get("test_id", "")
        owners = item.get("owner_categories", [])
        row = mapping_by_id.get(test_id)
        if (
            set(item) != {
                "test_id", "chromium_test_path", "rejection_reason", "owner_categories"
            }
            or not item.get("chromium_test_path")
            or not item.get("rejection_reason")
            or owners != sorted(set(owners))
            or set(owners) & SP15_RETIRED_CATEGORIES
            or not (set(owners) - TEXT_PORT_METADATA_CATEGORIES)
        ):
            errors.append(f"SP15 residual disposition is invalid: {test_id}")
            continue
        if not row or row.get("ported") != "no":
            errors.append(f"SP15 residual is not unported: {test_id}")
            continue
        mapped = {
            part.strip()
            for part in row.get("failure_category", "").split(",")
            if part.strip()
        }
        if not set(owners) <= mapped:
            errors.append(f"SP15 residual ownership drift: {test_id}")
        if row.get("chromium_test_path") != item.get("chromium_test_path"):
            errors.append(f"SP15 residual Chromium path drift: {test_id}")
        if row.get("notes") != f"Porter deferred: {item.get('rejection_reason', '')}":
            errors.append(f"SP15 residual rejection reason drift: {test_id}")

    stale = []
    fallback_owned = []
    metadata_only_failures = []
    for row in rows:
        test_id = canonical_mapping_id(row)
        categories = {
            part.strip()
            for part in row.get("failure_category", "").split(",")
            if part.strip()
        }
        if categories & SP15_RETIRED_CATEGORIES:
            stale.append(test_id)
        if categories & {"sp12_layout_bug", "not_ported"}:
            fallback_owned.append(test_id)
        if row.get("pixel_result") == "fail" and not (
            categories - TEXT_PORT_METADATA_CATEGORIES
        ):
            metadata_only_failures.append(test_id)
    if stale:
        errors.append(f"mapping retains retired SP15 ownership: {stale[0]}")
    if fallback_owned:
        errors.append(f"mapping retains fallback ownership: {fallback_owned[0]}")
    if metadata_only_failures:
        errors.append(
            "runnable failures lack functional non-metadata ownership: "
            + metadata_only_failures[0]
        )
    if len(rows) != SP14_EXPECTED_INVENTORY:
        errors.append(f"SP15 inventory count {len(rows)} != {SP14_EXPECTED_INVENTORY}")
    if len(templates) != SP15_EXPECTED_RUNNABLE or len(summary_by_id) != SP15_EXPECTED_RUNNABLE:
        errors.append(
            f"SP15 runnable count templates={len(templates)}, summary={len(summary_by_id)} "
            f"!= {SP15_EXPECTED_RUNNABLE}"
        )
    return errors


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
    valid_statuses = {"pass", "fail", "error"}
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
            # Both 'fail' and 'error' (render/diff failures) are counted as failures.
            fail_count += 1

    if missing_results > 3:
        issue(f"... and {missing_results - 3} more passes without result.json")
    if proof_failures > 3:
        issue(f"... and {proof_failures - 3} more proof failures")
    if missing_images > 3:
        issue(f"... and {missing_images - 3} more passes with missing PNGs")

    if missing_results == 0 and proof_failures == 0 and missing_images == 0:
        ok(f"All {pass_count} passes have verified 0.0% mismatch + PNG proof")

    # Verify totals (errors are folded into failed for accounting).
    error_count = sum(1 for t in summary["tests"] if t["status"] == "error")
    expected_failed = summary["failed"] + summary.get("errors", 0)
    if expected_failed != fail_count:
        issue(f"Summary says {summary['failed']} failed + {summary.get('errors', 0)} errors but found {fail_count}")
    if summary["passed"] != pass_count:
        issue(f"Summary says {summary['passed']} passed but found {pass_count}")
    if summary["total"] != pass_count + fail_count:
        issue(f"Total mismatch: {summary['total']} != {pass_count} + {fail_count}")
    else:
        ok(f"Totals consistent: {pass_count} pass + {fail_count} fail (incl. {error_count} render-error) = {summary['total']}")


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

    # Extract test IDs from Rust registry entries AND function definitions
    rust_test_ids = set()
    rust_fn_names = set()
    for fname in os.listdir(WPT_DIR):
        if fname.endswith(".rs") and fname != "mod.rs":
            fpath = os.path.join(WPT_DIR, fname)
            with open(fpath) as f:
                content = f.read()
            # Strip block comments /* ... */ (handles multi-line)
            content = re.sub(r"/\*.*?\*/", "", content, flags=re.DOTALL)
            for m in re.finditer(r'"(wpt/[^"]+)"', content):
                rust_test_ids.add(m.group(1))
            for line in content.splitlines():
                stripped = line.lstrip()
                if stripped.startswith("//"):
                    continue
                # Also collect function definitions
                m_fn = re.match(r'^fn\s+(\w+)\s*\(', stripped)
                if m_fn:
                    rust_fn_names.add(m_fn.group(1))

    summary_ids = {t["id"] for t in summary["tests"]}
    missing_rust = summary_ids - rust_test_ids
    extra_rust = rust_test_ids - summary_ids

    if missing_rust:
        issue(f"{len(missing_rust)} tests in summary but no Rust registry entry")
        for tid in sorted(missing_rust)[:3]:
            print(f"         {tid}")
    if extra_rust:
        warn(f"{len(extra_rust)} Rust tests not in summary (possibly not compared)")

    # Verify function definitions exist for registry entries
    # Registry format: ("wpt/area/name", fn_name as fn() -> Document)
    registry_fns_missing = 0
    for fname in os.listdir(WPT_DIR):
        if fname.endswith(".rs") and fname != "mod.rs":
            fpath = os.path.join(WPT_DIR, fname)
            with open(fpath) as f:
                content = f.read()
            content = re.sub(r"/\*.*?\*/", "", content, flags=re.DOTALL)
            for line in content.splitlines():
                stripped = line.lstrip()
                if stripped.startswith("//"):
                    continue
                for m in re.finditer(r',\s*(\w+)\s+as\s+fn\(\)', line):
                    fn_name = m.group(1)
                    if fn_name not in rust_fn_names:
                        registry_fns_missing += 1
                        if registry_fns_missing <= 3:
                            issue(f"Registry references fn '{fn_name}' but no definition found")

    if not missing_rust and registry_fns_missing == 0:
        ok(f"All {len(summary_ids)} summary tests have Rust registry entries and function definitions")


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
    untracked_not_ported = 0
    unclassified_unported = 0
    for r in rows:
        cat = r.get("failure_category", "").strip()
        if r["ported"] == "no":
            if not cat:
                unclassified_unported += 1
            if "not_ported" in {p.strip() for p in cat.split(",") if p.strip()}:
                untracked_not_ported += 1
        if cat:
            for part in cat.split(","):
                part = part.strip()
                if part and part not in VALID_FAILURE_CATEGORIES:
                    invalid_cats.add(part)
    if invalid_cats:
        issue(f"Invalid failure categories: {invalid_cats}")
    else:
        ok("All failure categories are from the valid set")
    if untracked_not_ported:
        issue(f"{untracked_not_ported} unported tests still use the generic not_ported bucket")
    if unclassified_unported:
        issue(f"{unclassified_unported} unported tests have no explicit dependency category")
    if not untracked_not_ported and not unclassified_unported:
        ok(f"All {not_ported} unported tests have explicit dependency categories")

    text_manifest_path = os.path.join(DATA_DIR, "wpt_ported", "text_ported_tests.json")
    if os.path.isfile(text_manifest_path):
        with open(text_manifest_path, encoding="utf-8") as f:
            text_ported_tests = set(json.load(f))
        stale_text, metadata_only = text_port_ownership_errors(rows, text_ported_tests)
        if stale_text:
            issue(f"{len(stale_text)} text-ported failures still retain needs_text")
        if metadata_only:
            issue(
                f"{len(metadata_only)} text-ported failures have only reference/non-visual metadata ownership"
            )
        if not stale_text and not metadata_only:
            ok(
                f"All failing text ports have functional non-text ownership "
                f"({len(text_ported_tests)} manifest IDs)"
            )

    closure_paths = (
        SP14_BASELINE_PATH,
        SP14_W3_PATH,
        SP14_W4_PATH,
        os.path.join(DATA_DIR, "wpt_ported", "all_wpt_templates.json"),
        os.path.join(RESULTS_DIR, "summary.json"),
        text_manifest_path,
    )
    sp15_paths = (
        SP15_BASELINE_PATH,
        SP15_TARGETS_PATH,
        SP15_RESIDUALS_PATH,
    )
    missing_closure_paths = [
        path for path in closure_paths + sp15_paths if not os.path.isfile(path)
    ]
    if missing_closure_paths:
        issue(
            "SP14 W3/W4 closure artifacts missing: "
            + ", ".join(os.path.relpath(path, PROJECT_ROOT) for path in missing_closure_paths)
        )
    else:
        with open(SP14_BASELINE_PATH, encoding="utf-8") as f:
            baseline = json.load(f)
        with open(SP14_W3_PATH, encoding="utf-8") as f:
            w3 = json.load(f)
        with open(SP14_W4_PATH, encoding="utf-8") as f:
            w4 = json.load(f)
        with open(closure_paths[3], encoding="utf-8") as f:
            closure_templates = json.load(f)
        with open(closure_paths[4], encoding="utf-8") as f:
            closure_summary = json.load(f)
        with open(text_manifest_path, encoding="utf-8") as f:
            closure_manifest = set(json.load(f))
        with open(SP15_BASELINE_PATH, encoding="utf-8") as f:
            sp15_baseline = json.load(f)
        with open(SP15_TARGETS_PATH, encoding="utf-8") as f:
            sp15_targets = json.load(f)
        with open(SP15_RESIDUALS_PATH, encoding="utf-8") as f:
            sp15_residuals = json.load(f)
        sp15_residuals_by_id = {
            item.get("test_id", ""): item
            for item in sp15_residuals
            if isinstance(item, dict)
        }
        closure_summary_by_id = {
            test["id"]: test for test in closure_summary["tests"]
        }
        closure_errors = sp14_text_closure_errors(
            rows,
            closure_summary_by_id,
            closure_templates,
            closure_manifest,
            baseline,
            w3,
            w4,
            superseded_actionable=sp15_targets,
            superseded_residuals=sp15_residuals_by_id,
        )
        if closure_errors:
            issue(f"SP14 W3/W4 closure has {len(closure_errors)} invariant violations")
            for message in closure_errors[:3]:
                print(f"         {message}")
        else:
            ok(
                f"SP14 closure: {len(baseline)} baseline exact, "
                f"{len(w3)} W3 runnable, {len(w4)} W4 reason-owned"
            )
        sp15_errors = sp15_closure_errors(
            rows,
            closure_summary_by_id,
            closure_templates,
            closure_manifest,
            sp15_baseline,
            sp15_targets,
            sp15_residuals,
        )
        if sp15_errors:
            issue(f"SP15 closure has {len(sp15_errors)} invariant violations")
            for message in sp15_errors[:3]:
                print(f"         {message}")
        else:
            ok(
                f"SP15 closure: {len(sp15_baseline)} baseline exact, "
                f"{len(sp15_targets)} actionable, {len(sp15_residuals)} residual"
            )

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

    # Cross-check with summary (errors are treated as failures in mapping).
    summary_path = os.path.join(RESULTS_DIR, "summary.json")
    if os.path.isfile(summary_path):
        with open(summary_path) as f:
            summary = json.load(f)
        summary_failed = summary["failed"] + summary.get("errors", 0)
        if ported != summary["total"]:
            issue(f"Mapping says {ported} ported but summary has {summary['total']} tests")
        if passing != summary["passed"]:
            issue(f"Mapping says {passing} passing but summary says {summary['passed']}")
        if failing != summary_failed:
            issue(f"Mapping says {failing} failing but summary says {summary_failed} (failed+errors)")
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

    # Bidirectional cross-check:
    # 1. Every deferred test's deps should appear in mapping categories
    # 2. Every mapping test with cross-SP categories should be in deferred
    deferred_not_in_mapping = 0
    mapping_not_in_deferred = 0

    for tid, dcats in deferred_cats.items():
        mcats = mapping_cats.get(tid, set())
        if dcats and not dcats.issubset(mcats):
            deferred_not_in_mapping += 1

    # Check reverse: mapping tests with cross-SP categories not in deferred
    cross_sp_cats = set(CATEGORY_FOR_DEP.values())
    for tid, mcats in mapping_cats.items():
        if mcats & cross_sp_cats:  # has at least one cross-SP category
            if tid not in deferred_cats:
                mapping_not_in_deferred += 1

    total_mismatches = deferred_not_in_mapping + mapping_not_in_deferred
    if total_mismatches:
        issue(f"Classification drift: {deferred_not_in_mapping} deferred∉mapping, "
              f"{mapping_not_in_deferred} mapping∉deferred")
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
