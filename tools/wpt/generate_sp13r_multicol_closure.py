#!/usr/bin/env python3
"""Freeze and validate the SP13-R runnable multicol closure ledgers."""

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

BASELINE_JSON = PORTED_DIR / "sp13r_baseline_exact.json"
TARGETS_JSON = PORTED_DIR / "sp13r_multicol_targets.json"
RESIDUALS_JSON = PORTED_DIR / "sp13r_multicol_residuals.json"

OWNER = "sp13_multicol"
FALLBACK_CATEGORIES = {"sp12_layout_bug", "not_ported"}
EXPECTED_INVENTORY = 1369
EXPECTED_BASELINE = 2823
EXPECTED_TARGETS = 351
EXPECTED_RESIDUALS = 1018
EXPECTED_RUNNABLE = 3566
EXPECTED_UNPORTED = 4107
EXPECTED_EXACT = EXPECTED_BASELINE + EXPECTED_TARGETS


def categories(value: str) -> set[str]:
    return {part.strip() for part in value.split(",") if part.strip()}


def canonical_id(row: dict[str, str]) -> str:
    canonical = f"wpt/{row['sp_area'].strip()}/{row['test_name'].strip()}"
    recorded = row.get("our_test_id", "").strip()
    if recorded and recorded != canonical:
        raise ValueError(f"mapping identity drift: {recorded!r} != {canonical!r}")
    return canonical


def validate_ledgers(baseline: list, targets: list, residuals: list) -> None:
    residual_ids = [item.get("test_id", "") for item in residuals if isinstance(item, dict)]
    if baseline != sorted(set(baseline)) or len(baseline) != EXPECTED_BASELINE:
        raise ValueError("SP13-R baseline is not the frozen sorted 2,823-ID set")
    if targets != sorted(set(targets)) or len(targets) != EXPECTED_TARGETS:
        raise ValueError("SP13-R target ledger is not the frozen sorted 351-ID set")
    if residual_ids != sorted(set(residual_ids)) or len(residuals) != EXPECTED_RESIDUALS:
        raise ValueError("SP13-R residual ledger is not the frozen sorted 1,018-ID set")
    if set(targets) & set(residual_ids):
        raise ValueError("SP13-R target and residual ledgers overlap")
    if len(set(targets) | set(residual_ids)) != EXPECTED_INVENTORY:
        raise ValueError("SP13-R ledgers do not form a 1,369-ID disjoint cover")
    if set(baseline) & (set(targets) | set(residual_ids)):
        raise ValueError("SP13-R baseline overlaps its multicol inventory")

    required = {
        "test_id", "chromium_test_path", "rejection_reason", "owner_categories"
    }
    for item in residuals:
        owners = item.get("owner_categories", [])
        if (
            set(item) != required
            or not item.get("chromium_test_path")
            or not item.get("rejection_reason")
            or owners != sorted(set(owners))
            or OWNER not in owners
            or set(owners) & FALLBACK_CATEGORIES
        ):
            raise ValueError(f"invalid SP13-R residual disposition: {item!r}")


def build_ledgers(rows: list[dict[str, str]], summary: dict) -> tuple[list, list, list]:
    baseline = sorted(
        item["id"]
        for item in summary.get("tests", [])
        if item.get("status") == "pass" and item.get("mismatch_pct") == 0.0
    )
    owned = [row for row in rows if OWNER in categories(row["failure_category"])]
    targets = sorted(canonical_id(row) for row in owned if row["ported"] == "yes")
    residuals = [
        {
            "test_id": canonical_id(row),
            "chromium_test_path": row["chromium_test_path"],
            "rejection_reason": row.get("notes", "")
            .removeprefix("Porter deferred: ")
            .strip(),
            "owner_categories": sorted(categories(row["failure_category"])),
        }
        for row in sorted(
            (row for row in owned if row["ported"] == "no"), key=canonical_id
        )
    ]
    validate_ledgers(baseline, targets, residuals)
    if {canonical_id(row) for row in owned} != set(targets) | {
        item["test_id"] for item in residuals
    }:
        raise ValueError("SP13-R ledgers do not cover the original multicol inventory")
    return baseline, targets, residuals


def load_ledgers() -> tuple[list, list, list]:
    values = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in (BASELINE_JSON, TARGETS_JSON, RESIDUALS_JSON)
    ]
    validate_ledgers(*values)
    return values[0], values[1], values[2]


def runnable_wpt_results(summary: dict) -> dict[str, dict]:
    """Return the runnable WPT rows from the runner's mixed native/WPT summary."""
    return {
        item["id"]: item
        for item in summary.get("tests", [])
        if item.get("id", "").startswith("wpt/")
    }


def validate_closed_snapshot(
    rows: list[dict[str, str]], summary: dict, baseline: list, targets: list,
    residuals: list,
) -> None:
    validate_ledgers(baseline, targets, residuals)
    # The full pixel runner writes native SP11-SP13 smoke tests alongside the
    # runnable WPT inventory.  SP13-R's inventory and immutable ledgers are WPT
    # only, so do not let those independently-accounted native rows inflate the
    # 3,566-row closure count.
    summary_by_id = runnable_wpt_results(summary)
    mapping = {canonical_id(row): row for row in rows}
    if len(rows) != 7673:
        raise ValueError(f"SP13-R mapping inventory changed: {len(rows)}")
    if len(summary_by_id) != EXPECTED_RUNNABLE:
        raise ValueError(f"SP13-R runnable inventory changed: {len(summary_by_id)}")
    if sum(row.get("ported") == "no" for row in rows) != EXPECTED_UNPORTED:
        raise ValueError("SP13-R unported inventory changed")
    if sum(
        item.get("status") == "pass" and item.get("mismatch_pct") == 0.0
        for item in summary_by_id.values()
    ) < EXPECTED_EXACT:
        raise ValueError("SP13-R exact pass floor is below 3,174")
    if any(item.get("status") == "error" for item in summary_by_id.values()):
        raise ValueError("SP13-R closed snapshot contains render/diff errors")
    for test_id in baseline:
        item = summary_by_id.get(test_id)
        if not item or item.get("status") != "pass" or item.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP13-R baseline exact pass regressed: {test_id}")
    for test_id in targets:
        row = mapping.get(test_id)
        result = summary_by_id.get(test_id)
        if not row or row.get("ported") != "yes" or row.get("our_test_id") != test_id:
            raise ValueError(f"SP13-R target is not runnable: {test_id}")
        if not result or result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP13-R target is not exact: {test_id}")
        if OWNER in categories(row.get("failure_category", "")):
            raise ValueError(f"SP13-R runnable target retains multicol ownership: {test_id}")
    for item in residuals:
        test_id = item["test_id"]
        row = mapping.get(test_id)
        if not row or row.get("ported") != "no":
            raise ValueError(f"SP13-R residual unexpectedly became runnable: {test_id}")
        if categories(row.get("failure_category", "")) != set(item["owner_categories"]):
            raise ValueError(f"SP13-R residual ownership drift: {test_id}")
        if row.get("chromium_test_path") != item["chromium_test_path"]:
            raise ValueError(f"SP13-R residual Chromium path drift: {test_id}")
        if row.get("notes") != f"Porter deferred: {item['rejection_reason']}":
            raise ValueError(f"SP13-R residual porter rejection drift: {test_id}")
    stale = [
        canonical_id(row)
        for row in rows
        if OWNER in categories(row.get("failure_category", ""))
        and row.get("ported") != "no"
    ]
    if stale:
        raise ValueError(f"SP13-R owner remains on a runnable row: {stale[0]}")


def encoded(value: object) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    if sys.argv[1:] not in ([], ["--check"]):
        print("Usage: generate_sp13r_multicol_closure.py [--check]", file=sys.stderr)
        return 2
    check = sys.argv[1:] == ["--check"]
    rows = list(csv.DictReader(MAPPING_CSV.open(newline="", encoding="utf-8")))
    summary = json.loads(SUMMARY_JSON.read_text(encoding="utf-8"))
    if check:
        validate_closed_snapshot(rows, summary, *load_ledgers())
    else:
        values = build_ledgers(rows, summary)
        for path, content in zip(
            (BASELINE_JSON, TARGETS_JSON, RESIDUALS_JSON), map(encoded, values)
        ):
            path.write_text(content, encoding="utf-8")
    print("SP13-R ledgers: baseline=2823, targets=351, residuals=1018")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
