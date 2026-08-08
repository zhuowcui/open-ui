#!/usr/bin/env python3
"""Freeze the SP14 W3/W4 text-closure accountability ledgers."""

from __future__ import annotations

import csv
import json
import os
import sys
from collections import Counter
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent
ACCOUNTABILITY_DIR = PROJECT_ROOT / "tools" / "accountability"
DATA_DIR = ACCOUNTABILITY_DIR / "data"
PORTED_DIR = DATA_DIR / "wpt_ported"
MAPPING_CSV = DATA_DIR / "wpt_mapping.csv"
SUMMARY_JSON = DATA_DIR / "pixel_comparison" / "results" / "summary.json"
WPT_ROOT = Path(
    os.environ.get(
        "CHROMIUM_WPT_CSS",
        os.path.expanduser(
            "~/chromium/src/third_party/blink/web_tests/external/wpt/css"
        ),
    )
)

BASELINE_JSON = PORTED_DIR / "sp14_w3_baseline_exact.json"
W3_JSON = PORTED_DIR / "sp14_w3_targets.json"
W4_JSON = PORTED_DIR / "sp14_w4_residuals.json"

EXPECTED_ORIGINAL = 4045
EXPECTED_BASELINE = 2715
EXPECTED_W3 = 111
EXPECTED_W4 = 3934
EXPECTED_W3_AREAS = {
    "css2_floats": 2,
    "css_backgrounds": 64,
    "css_flexbox": 25,
    "css_multicol": 15,
    "css_sizing": 5,
}
METADATA_CATEGORIES = {"reference_test", "non_visual_test"}

sys.path.insert(0, str(SCRIPT_DIR))
import port_wpt  # noqa: E402
from splice_text_port import canonical_test_id  # noqa: E402

sys.path.insert(0, str(ACCOUNTABILITY_DIR))
from shared_detectors import (  # noqa: E402
    CATEGORY_FOR_DEP,
    classify_dependencies,
    dependency_for_portability_reason,
)


def _categories(value: str) -> set[str]:
    return {part.strip() for part in value.split(",") if part.strip()}


def build_ledgers(
    mapping_rows: list[dict[str, str]], summary: dict, wpt_root: Path
) -> tuple[list[str], list[str], list[dict]]:
    """Derive all three immutable ledgers from one authoritative snapshot."""
    baseline = sorted(
        test["id"]
        for test in summary.get("tests", [])
        if test.get("status") == "pass" and test.get("mismatch_pct") == 0.0
    )
    if len(baseline) != len(set(baseline)):
        raise ValueError("duplicate exact-pass ID in authoritative summary")

    original_rows = [
        row
        for row in mapping_rows
        if row.get("ported") == "no"
        and "needs_text" in _categories(row.get("failure_category", ""))
    ]
    canonical = [canonical_test_id(row) for row in original_rows]
    if len(canonical) != len(set(canonical)):
        raise ValueError("ambiguous canonical ID in original needs_text inventory")

    old_emit = port_wpt.EMIT_TEXT_NODES
    old_retain = port_wpt.RETAIN_TEXT
    port_wpt.EMIT_TEXT_NODES = True
    port_wpt.RETAIN_TEXT = True
    try:
        w3: list[str] = []
        w4: list[dict] = []
        for row in sorted(original_rows, key=canonical_test_id):
            test_id = canonical_test_id(row)
            chromium_path = row.get("chromium_test_path", "").strip()
            upstream = wpt_root / chromium_path
            if not chromium_path or not upstream.is_file():
                raise FileNotFoundError(f"{test_id}: missing upstream file {upstream}")

            parser = port_wpt.parse_wpt_html(str(upstream))
            portable, reason = port_wpt.analyze_portability(parser)
            if portable and port_wpt.has_layout_content(parser):
                w3.append(test_id)
                continue
            if portable:
                reason = "no_layout_content"

            html = upstream.read_text(encoding="utf-8", errors="ignore")
            dependencies = classify_dependencies(
                html, test_id=test_id, excluded={"text_rendering"}
            )
            rejection_dependency = dependency_for_portability_reason(reason)
            if rejection_dependency not in dependencies:
                dependencies.append(rejection_dependency)
            owner_categories = sorted(
                {CATEGORY_FOR_DEP[dependency] for dependency in dependencies}
            )
            rejection_owner = CATEGORY_FOR_DEP[rejection_dependency]
            functional = set(owner_categories) - METADATA_CATEGORIES
            if (
                not reason
                or not functional
                or rejection_owner not in owner_categories
                or "needs_text" in owner_categories
            ):
                raise ValueError(f"{test_id}: invalid reason-backed W4 ownership")
            w4.append(
                {
                    "test_id": test_id,
                    "chromium_test_path": chromium_path,
                    "rejection_reason": reason,
                    "rejection_owner": rejection_owner,
                    "owner_categories": owner_categories,
                }
            )
    finally:
        port_wpt.EMIT_TEXT_NODES = old_emit
        port_wpt.RETAIN_TEXT = old_retain

    return baseline, w3, w4


def validate_frozen_counts(
    baseline: list[str], w3: list[str], w4: list[dict]
) -> None:
    if any(not isinstance(test_id, str) or not test_id for test_id in baseline + w3):
        raise ValueError("baseline and W3 ledgers must contain non-empty string IDs")
    if any(not isinstance(item, dict) for item in w4):
        raise ValueError("W4 ledger must contain JSON objects")
    w4_ids = [item["test_id"] for item in w4]
    if len(baseline) != EXPECTED_BASELINE:
        raise ValueError(
            f"expected {EXPECTED_BASELINE} exact baseline IDs, got {len(baseline)}"
        )
    if len(w3) != EXPECTED_W3 or len(w4) != EXPECTED_W4:
        raise ValueError(
            f"expected W3/W4 {EXPECTED_W3}/{EXPECTED_W4}, got "
            f"{len(w3)}/{len(w4)}"
        )
    if baseline != sorted(set(baseline)):
        raise ValueError("baseline ledger must be sorted and unique")
    if w3 != sorted(set(w3)) or w4_ids != sorted(set(w4_ids)):
        raise ValueError("W3/W4 ledgers must be sorted and unique")
    if set(w3) & set(w4_ids) or len(set(w3) | set(w4_ids)) != EXPECTED_ORIGINAL:
        raise ValueError("W3/W4 ledgers are not a disjoint cover of the backlog")
    areas = Counter(test_id.split("/")[1] for test_id in w3)
    if dict(areas) != EXPECTED_W3_AREAS:
        raise ValueError(f"unexpected W3 area split: {dict(areas)}")

    required = {
        "test_id",
        "chromium_test_path",
        "rejection_reason",
        "rejection_owner",
        "owner_categories",
    }
    for item in w4:
        if set(item) != required:
            raise ValueError(f"malformed W4 ledger row: {item.get('test_id', item)!r}")
        owners = item["owner_categories"]
        functional = set(owners) - METADATA_CATEGORIES
        if (
            not item["chromium_test_path"]
            or not item["rejection_reason"]
            or not item["rejection_owner"]
            or owners != sorted(set(owners))
            or item["rejection_owner"] not in owners
            or not functional
            or "needs_text" in owners
        ):
            raise ValueError(f"invalid W4 ledger row: {item['test_id']}")


def load_frozen_ledgers() -> tuple[list[str], list[str], list[dict]]:
    values = []
    for path in (BASELINE_JSON, W3_JSON, W4_JSON):
        with open(path, encoding="utf-8") as f:
            values.append(json.load(f))
    baseline, w3, w4 = values
    if not isinstance(baseline, list) or not isinstance(w3, list) or not isinstance(w4, list):
        raise ValueError("SP14 ledgers must contain JSON arrays")
    validate_frozen_counts(baseline, w3, w4)
    return baseline, w3, w4


def validate_closed_snapshot(
    mapping_rows: list[dict[str, str]], summary: dict, baseline: list[str],
    w3: list[str], w4: list[dict]
) -> None:
    """Check immutable ledgers against the post-closure authoritative artifacts."""
    summary_by_id = {test["id"]: test for test in summary.get("tests", [])}
    bad_baseline = [
        test_id
        for test_id in baseline
        if summary_by_id.get(test_id, {}).get("status") != "pass"
        or summary_by_id.get(test_id, {}).get("mismatch_pct") != 0.0
    ]
    if bad_baseline:
        raise ValueError(f"baseline exact-pass regression: {bad_baseline[0]}")

    mapping_by_id = {canonical_test_id(row): row for row in mapping_rows}
    bad_w3 = [
        test_id
        for test_id in w3
        if test_id not in mapping_by_id or mapping_by_id[test_id]["ported"] != "yes"
    ]
    if bad_w3:
        raise ValueError(f"W3 target is not runnable: {bad_w3[0]}")
    for item in w4:
        if item["test_id"] not in mapping_by_id:
            raise ValueError(f"W4 residual is absent from mapping: {item['test_id']}")
        row = mapping_by_id[item["test_id"]]
        if row["ported"] != "no":
            raise ValueError(f"W4 residual became runnable: {item['test_id']}")
        categories = _categories(row.get("failure_category", ""))
        if not set(item["owner_categories"]).issubset(categories):
            raise ValueError(f"W4 ownership drift: {item['test_id']}")
        if item["rejection_reason"] not in row.get("notes", ""):
            raise ValueError(f"W4 rejection-reason drift: {item['test_id']}")


def _encoded(value) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    check_only = sys.argv[1:] == ["--check"]
    if sys.argv[1:] not in ([], ["--check"]):
        print("Usage: generate_sp14_text_closure.py [--check]", file=sys.stderr)
        return 2
    with open(MAPPING_CSV, newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))
    with open(SUMMARY_JSON, encoding="utf-8") as f:
        summary = json.load(f)

    original_rows = [
        row
        for row in rows
        if row.get("ported") == "no"
        and "needs_text" in _categories(row.get("failure_category", ""))
    ]
    if check_only and not original_rows:
        baseline, w3, w4 = load_frozen_ledgers()
        validate_closed_snapshot(rows, summary, baseline, w3, w4)
        print(
            f"SP14 ledgers: baseline={len(baseline)}, W3={len(w3)}, W4={len(w4)}"
        )
        return 0

    baseline, w3, w4 = build_ledgers(rows, summary, WPT_ROOT)
    validate_frozen_counts(baseline, w3, w4)
    outputs = {
        BASELINE_JSON: _encoded(baseline),
        W3_JSON: _encoded(w3),
        W4_JSON: _encoded(w4),
    }
    if check_only:
        drift = [path for path, content in outputs.items() if path.read_text() != content]
        if drift:
            raise ValueError("ledger drift: " + ", ".join(map(str, drift)))
    else:
        for path, content in outputs.items():
            path.write_text(content, encoding="utf-8")
    print(
        f"SP14 ledgers: baseline={len(baseline)}, W3={len(w3)}, W4={len(w4)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
