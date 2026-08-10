#!/usr/bin/env python3
"""Freeze and validate the SP16 real-font metric closure ledgers."""

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

BASELINE_JSON = PORTED_DIR / "sp16_baseline_exact.json"
TARGETS_JSON = PORTED_DIR / "sp16_actionable_targets.json"
RESIDUALS_JSON = PORTED_DIR / "sp16_residual_dispositions.json"
REAL_FONT_JSON = PORTED_DIR / "sp16_real_font_tests.json"

SP16_CATEGORY = "needs_font_metrics"
METADATA_CATEGORIES = {"reference_test", "non_visual_test"}
FALLBACK_CATEGORIES = {"sp12_layout_bug", "not_ported"}
EXPECTED_INVENTORY = 776
EXPECTED_BASELINE = 2804
EXPECTED_TARGETS = 226
EXPECTED_RESIDUALS = 550
EXPECTED_RUNNABLE = 3566
EXPECTED_UNPORTED = 4107


def categories(value: str) -> set[str]:
    return {part.strip() for part in value.split(",") if part.strip()}


def canonical_id(row: dict[str, str]) -> str:
    canonical = f"wpt/{row['sp_area'].strip()}/{row['test_name'].strip()}"
    recorded = row.get("our_test_id", "").strip()
    if recorded and recorded != canonical:
        raise ValueError(f"mapping identity drift: {recorded!r} != {canonical!r}")
    return canonical


def validate_ledgers(baseline: list, targets: list, residuals: list, manifest: list) -> None:
    residual_ids = [item.get("test_id", "") for item in residuals if isinstance(item, dict)]
    if baseline != sorted(set(baseline)) or len(baseline) != EXPECTED_BASELINE:
        raise ValueError("SP16 exact baseline is not the frozen sorted 2,804-ID set")
    if targets != sorted(set(targets)) or len(targets) != EXPECTED_TARGETS:
        raise ValueError("SP16 actionable ledger is not the frozen sorted 226-ID set")
    if manifest != targets:
        raise ValueError("SP16 real-font manifest must exactly equal the actionable ledger")
    if residual_ids != sorted(set(residual_ids)) or len(residuals) != EXPECTED_RESIDUALS:
        raise ValueError("SP16 residual ledger is not the frozen sorted 550-ID set")
    if set(targets) & set(residual_ids):
        raise ValueError("SP16 actionable and residual ledgers overlap")
    if len(set(targets) | set(residual_ids)) != EXPECTED_INVENTORY:
        raise ValueError("SP16 ledgers do not form a 776-ID disjoint cover")
    if set(baseline) & (set(targets) | set(residual_ids)):
        raise ValueError("SP16 exact baseline overlaps its owner inventory")

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
            or SP16_CATEGORY in owners
            or set(owners) & FALLBACK_CATEGORIES
            or not (set(owners) - METADATA_CATEGORIES)
        ):
            raise ValueError(f"invalid SP16 residual disposition: {item!r}")


def build_ledgers(rows: list[dict[str, str]], summary: dict) -> tuple[list, list, list, list]:
    baseline = sorted(
        item["id"]
        for item in summary.get("tests", [])
        if item.get("status") == "pass" and item.get("mismatch_pct") == 0.0
    )
    owned = [row for row in rows if SP16_CATEGORY in categories(row["failure_category"])]
    targets = sorted(canonical_id(row) for row in owned if row["ported"] == "yes")
    residuals = []
    for row in sorted((row for row in owned if row["ported"] == "no"), key=canonical_id):
        residuals.append({
            "test_id": canonical_id(row),
            "chromium_test_path": row["chromium_test_path"],
            "rejection_reason": row.get("notes", "").removeprefix("Porter deferred: ").strip(),
            "owner_categories": sorted(categories(row["failure_category"]) - {SP16_CATEGORY}),
        })
    manifest = list(targets)
    validate_ledgers(baseline, targets, residuals, manifest)
    if {canonical_id(row) for row in owned} != set(targets) | {
        item["test_id"] for item in residuals
    }:
        raise ValueError("SP16 ledgers do not cover the original owner inventory")
    return baseline, targets, residuals, manifest


def load_ledgers() -> tuple[list, list, list, list]:
    values = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in (BASELINE_JSON, TARGETS_JSON, RESIDUALS_JSON, REAL_FONT_JSON)
    ]
    validate_ledgers(*values)
    return values[0], values[1], values[2], values[3]


def validate_closed_snapshot(
    rows: list[dict[str, str]], summary: dict, baseline: list, targets: list,
    residuals: list, manifest: list,
) -> None:
    validate_ledgers(baseline, targets, residuals, manifest)
    summary_by_id = {item["id"]: item for item in summary.get("tests", [])}
    mapping = {canonical_id(row): row for row in rows}
    if len(summary_by_id) != EXPECTED_RUNNABLE:
        raise ValueError(f"SP16 runnable inventory changed: {len(summary_by_id)}")
    if sum(row.get("ported") == "no" for row in rows) != EXPECTED_UNPORTED:
        raise ValueError("SP16 unported inventory changed")
    for test_id in baseline:
        item = summary_by_id.get(test_id)
        if not item or item.get("status") != "pass" or item.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP16 baseline exact pass regressed: {test_id}")
    for test_id in targets:
        row = mapping.get(test_id)
        result = summary_by_id.get(test_id)
        if not row or row.get("ported") != "yes" or not result:
            raise ValueError(f"SP16 actionable target is not runnable: {test_id}")
        if result.get("status") == "error":
            raise ValueError(f"SP16 actionable target has a render error: {test_id}")
    for item in residuals:
        row = mapping.get(item["test_id"])
        if not row or row.get("ported") != "no":
            raise ValueError(f"SP16 residual unexpectedly became runnable: {item['test_id']}")
        if not set(item["owner_categories"]).issubset(categories(row["failure_category"])):
            raise ValueError(f"SP16 residual ownership drift: {item['test_id']}")
    stale = [canonical_id(row) for row in rows if SP16_CATEGORY in categories(row["failure_category"])]
    if stale:
        raise ValueError(f"retired SP16 owner remains in mapping: {stale[0]}")


def encoded(value: object) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    if sys.argv[1:] not in ([], ["--check"]):
        print("Usage: generate_sp16_closure.py [--check]", file=sys.stderr)
        return 2
    check = sys.argv[1:] == ["--check"]
    rows = list(csv.DictReader(MAPPING_CSV.open(newline="", encoding="utf-8")))
    summary = json.loads(SUMMARY_JSON.read_text(encoding="utf-8"))
    original = [row for row in rows if SP16_CATEGORY in categories(row["failure_category"])]
    if check and not original:
        validate_closed_snapshot(rows, summary, *load_ledgers())
    else:
        values = build_ledgers(rows, summary)
        outputs = dict(zip(
            (BASELINE_JSON, TARGETS_JSON, RESIDUALS_JSON, REAL_FONT_JSON),
            map(encoded, values),
        ))
        if check:
            drift = [path for path, content in outputs.items() if path.read_text() != content]
            if drift:
                raise ValueError("SP16 ledger drift: " + ", ".join(map(str, drift)))
        else:
            for path, content in outputs.items():
                path.write_text(content, encoding="utf-8")
    print("SP16 ledgers: baseline=2804, actionable=226, residuals=550")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
