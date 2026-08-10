#!/usr/bin/env python3
"""Freeze and validate the SP15 inline/layout + root/body closure ledgers."""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent
DATA_DIR = PROJECT_ROOT / "tools" / "accountability" / "data"
PORTED_DIR = DATA_DIR / "wpt_ported"
MAPPING_CSV = DATA_DIR / "wpt_mapping.csv"
SUMMARY_JSON = DATA_DIR / "pixel_comparison" / "results" / "summary.json"

BASELINE_JSON = PORTED_DIR / "sp15_baseline_exact.json"
TARGETS_JSON = PORTED_DIR / "sp15_actionable_targets.json"
RESIDUALS_JSON = PORTED_DIR / "sp15_residual_dispositions.json"

SP15_CATEGORIES = {
    "needs_inline_box_decoration_break",
    "needs_clearing_break_after_floats",
    "needs_display_contents_style_element",
    "needs_display_contents_list_layout",
    "needs_root_body_layout",
}
METADATA_CATEGORIES = {"reference_test", "non_visual_test"}
EXPECTED_INVENTORY = 130
EXPECTED_BASELINE = 2767
EXPECTED_TARGETS = 76
EXPECTED_RESIDUALS = 54


def categories(value: str) -> set[str]:
    return {part.strip() for part in value.split(",") if part.strip()}


def canonical_id(row: dict[str, str]) -> str:
    recorded = row.get("our_test_id", "").strip()
    canonical = f"wpt/{row['sp_area'].strip()}/{row['test_name'].strip()}"
    if recorded and recorded != canonical:
        raise ValueError(f"mapping identity drift: {recorded!r} != {canonical!r}")
    return canonical


def build_ledgers(rows: list[dict[str, str]], summary: dict) -> tuple[list, list, list]:
    baseline = sorted(
        item["id"]
        for item in summary.get("tests", [])
        if item.get("status") == "pass" and item.get("mismatch_pct") == 0.0
    )
    owned = [row for row in rows if categories(row["failure_category"]) & SP15_CATEGORIES]
    targets = sorted(
        canonical_id(row)
        for row in owned
        if row["ported"] == "yes"
        or "needs_root_body_layout" in categories(row["failure_category"])
    )
    target_set = set(targets)
    residuals = []
    for row in sorted(owned, key=canonical_id):
        test_id = canonical_id(row)
        if test_id in target_set:
            continue
        owners = sorted(categories(row["failure_category"]) - SP15_CATEGORIES)
        reason = row.get("notes", "").removeprefix("Porter deferred: ").strip()
        residuals.append(
            {
                "test_id": test_id,
                "chromium_test_path": row["chromium_test_path"],
                "rejection_reason": reason,
                "owner_categories": owners,
            }
        )
    validate_ledgers(baseline, targets, residuals)
    if set(targets) | {item["test_id"] for item in residuals} != {
        canonical_id(row) for row in owned
    }:
        raise ValueError("SP15 ledgers do not cover the original owner inventory")
    return baseline, targets, residuals


def validate_ledgers(baseline: list, targets: list, residuals: list) -> None:
    residual_ids = [item.get("test_id", "") for item in residuals if isinstance(item, dict)]
    if baseline != sorted(set(baseline)) or len(baseline) != EXPECTED_BASELINE:
        raise ValueError("SP15 exact-pass baseline is not the frozen sorted 2,767-ID set")
    if targets != sorted(set(targets)) or len(targets) != EXPECTED_TARGETS:
        raise ValueError("SP15 actionable ledger is not the frozen sorted 76-ID set")
    if residual_ids != sorted(set(residual_ids)) or len(residuals) != EXPECTED_RESIDUALS:
        raise ValueError("SP15 residual ledger is not the frozen sorted 54-ID set")
    if set(targets) & set(residual_ids):
        raise ValueError("SP15 actionable and residual ledgers overlap")
    if len(set(targets) | set(residual_ids)) != EXPECTED_INVENTORY:
        raise ValueError("SP15 ledgers do not form a 130-ID disjoint cover")
    required = {
        "test_id",
        "chromium_test_path",
        "rejection_reason",
        "owner_categories",
    }
    for item in residuals:
        owners = item.get("owner_categories", [])
        if (
            set(item) != required
            or not item.get("chromium_test_path")
            or not item.get("rejection_reason")
            or owners != sorted(set(owners))
            or set(owners) & SP15_CATEGORIES
            or not (set(owners) - METADATA_CATEGORIES)
        ):
            raise ValueError(f"invalid SP15 residual disposition: {item!r}")


def load_ledgers() -> tuple[list, list, list]:
    values = [json.loads(path.read_text(encoding="utf-8")) for path in (
        BASELINE_JSON, TARGETS_JSON, RESIDUALS_JSON
    )]
    validate_ledgers(*values)
    return values[0], values[1], values[2]


def validate_closed_snapshot(
    rows: list[dict[str, str]], summary: dict, baseline: list, targets: list, residuals: list
) -> None:
    summary_by_id = {item["id"]: item for item in summary.get("tests", [])}
    mapping = {canonical_id(row): row for row in rows}
    for test_id in baseline:
        item = summary_by_id.get(test_id)
        if not item or item.get("status") != "pass" or item.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP15 baseline exact pass regressed: {test_id}")
    for test_id in targets:
        row = mapping.get(test_id)
        if not row or row.get("ported") != "yes":
            raise ValueError(f"SP15 actionable target is not runnable: {test_id}")
        if summary_by_id.get(test_id, {}).get("status") == "error":
            raise ValueError(f"SP15 actionable target has a render error: {test_id}")
    for item in residuals:
        row = mapping.get(item["test_id"])
        if not row or row.get("ported") != "no":
            raise ValueError(f"SP15 residual unexpectedly became runnable: {item['test_id']}")
        mapped = categories(row.get("failure_category", ""))
        if not set(item["owner_categories"]).issubset(mapped):
            raise ValueError(f"SP15 residual ownership drift: {item['test_id']}")
    stale = [canonical_id(row) for row in rows if categories(row["failure_category"]) & SP15_CATEGORIES]
    if stale:
        raise ValueError(f"retired SP15 owner remains in mapping: {stale[0]}")


def encoded(value: object) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    if sys.argv[1:] not in ([], ["--check"]):
        print("Usage: generate_sp15_closure.py [--check]", file=sys.stderr)
        return 2
    check = sys.argv[1:] == ["--check"]
    rows = list(csv.DictReader(MAPPING_CSV.open(newline="", encoding="utf-8")))
    summary = json.loads(SUMMARY_JSON.read_text(encoding="utf-8"))
    original = [row for row in rows if categories(row["failure_category"]) & SP15_CATEGORIES]
    if check and not original:
        baseline, targets, residuals = load_ledgers()
        validate_closed_snapshot(rows, summary, baseline, targets, residuals)
    else:
        baseline, targets, residuals = build_ledgers(rows, summary)
        outputs = {
            BASELINE_JSON: encoded(baseline),
            TARGETS_JSON: encoded(targets),
            RESIDUALS_JSON: encoded(residuals),
        }
        if check:
            drift = [path for path, content in outputs.items() if path.read_text() != content]
            if drift:
                raise ValueError("SP15 ledger drift: " + ", ".join(map(str, drift)))
        else:
            for path, content in outputs.items():
                path.write_text(content, encoding="utf-8")
    print("SP15 ledgers: baseline=2767, actionable=76, residuals=54")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
