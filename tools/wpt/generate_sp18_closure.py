#!/usr/bin/env python3
"""Generate and validate the atomic SP18 generated-text closure ledgers.

The syntax inventory is intentionally independent of the accountability
detectors.  In particular, the negative property boundary keeps declarations
such as ``align-content`` and ``justify-content`` out of this sprint without
changing their historical detector classification.
"""

from __future__ import annotations

import csv
import hashlib
import json
import os
import re
import sys
from collections import Counter
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
DATA = ROOT / "tools" / "accountability" / "data"
PORTED = DATA / "wpt_ported"
SP19_KICKOFF_MAPPING = PORTED / "sp19_kickoff_mapping.csv"
SP19_KICKOFF_SUMMARY = PORTED / "sp19_sp18_summary.json"
MAPPING = SP19_KICKOFF_MAPPING if SP19_KICKOFF_MAPPING.is_file() else DATA / "wpt_mapping.csv"
SUMMARY = (
    SP19_KICKOFF_SUMMARY
    if SP19_KICKOFF_SUMMARY.is_file()
    else DATA / "pixel_comparison" / "results" / "summary.json"
)
WPT_ROOT = Path(os.environ.get(
    "CHROMIUM_WPT_CSS",
    os.path.expanduser(
        "~/chromium/src/third_party/blink/web_tests/external/wpt/css"
    ),
))

BASELINE = PORTED / "sp18_baseline_exact.json"
INVENTORY = PORTED / "sp18_syntax_inventory.json"
TARGETS = HERE / "sp18_targets.json"
FOCUSED = HERE / "sp18_focused_ids.json"
RESIDUAL_IDS = HERE / "sp18_residual_ids.json"
RUNNABLE_RESIDUALS = HERE / "sp18_runnable_residuals.json"
RESIDUAL_DISPOSITIONS = PORTED / "sp18_residual_dispositions.json"
VERIFIED_MISSES = PORTED / "sp18_verified_misses.json"

EXPECTED_MAPPING = 7673
EXPECTED_BASELINE = 3807
EXPECTED_INVENTORY = 552
EXPECTED_TARGETS = 251
EXPECTED_PROMOTIONS = 250
EXPECTED_RESIDUAL_IDS = 292
EXPECTED_FOCUSED = 4058
EXPECTED_RUNNABLE_RESIDUALS = 81
EXPECTED_VERIFIED_MISSES = 3
EXPECTED_FINAL_RUNNABLE = 4139
EXPECTED_FINAL_EXACT = 4058
EXPECTED_FINAL_FAILURES = 81
EXPECTED_FINAL_ERRORS = 0
EXPECTED_FINAL_UNPORTED = 3534

MANIFEST_SHA256 = {
    BASELINE: "4dcf84612a43d8d05d7132992bde3e7daac0504d1beafcdc7efbcfe8a95d575c",
    INVENTORY: "d52079afbef557a0a070ae3be8d877780e168352e632d2cc37005b9d2ecb752a",
    TARGETS: "ead0c7db1721eb425df29afa01d505520eb6dc22bf8ec619ca1e66ac37bb7eaf",
    RESIDUAL_IDS: "d65b629f4b086bca587d7005f96d8b23ee8a059ca0798c253e25f416513807d8",
    FOCUSED: "1b56a4c7f218fa2bdea69e9dc9691b4b104983ad6f02c6c16cdde4fd07a76373",
    RUNNABLE_RESIDUALS: "1b9e7d88819931c0f0c67100f799cd95ec3a5ab9ea177fae4c29b37ceb4aff71",
    VERIFIED_MISSES: "85439c9a03963ad29efa2c16294477650fb0ff7dc43b7e383088d7b5da0ad5d8",
}

BASELINE_SUMMARY_SHA256 = (
    "cef6f982a7bcaa646d90c5e7c2cb03e768a525dda34140d399bff839846d3f68"
)
FINAL_SUMMARY_SHA256 = (
    "dec8f5c330c94fab200afcb3f82da6af2888695284284b36d8906f9fc939aa7e"
)

# Strict declarations need a left property boundary: the intentionally broad
# global generated-content detector also matches align-content/justify-content.
PSEUDO_SYNTAX = re.compile(
    r"::?(?:before|after|first-line|first-letter)\b", re.IGNORECASE
)
PROPERTY_SYNTAX = re.compile(
    r"(?<![-\w])(?:content|counter-(?:reset|set|increment)|quotes|"
    r"text-overflow|text-shadow|line-clamp|-webkit-line-clamp|"
    r"-webkit-box-orient)\s*:",
    re.IGNORECASE,
)

TARGET_AREA_COUNTS = {
    "css_overflow": 204,
    "css_flexbox": 27,
    "css_display": 11,
    "css_backgrounds": 5,
    "css_break": 3,
    "css_multicol": 1,
}

sys.path.insert(0, str(HERE))
import port_wpt  # noqa: E402

ACCOUNTABILITY = ROOT / "tools" / "accountability"
sys.path.insert(0, str(ACCOUNTABILITY))
from shared_detectors import (  # noqa: E402
    CATEGORY_FOR_DEP,
    classify_dependencies,
    dependency_for_portability_reason,
)

METADATA_OWNERS = {"reference_test", "non_visual_test"}
COHORT_DEPENDENCIES = {"generated_content", "line_clamp"}


def encoded(value: object) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def canonical_id(row: dict[str, str]) -> str:
    test_id = f"wpt/{row['sp_area'].strip()}/{row['test_name'].strip()}"
    recorded = row.get("our_test_id", "").strip()
    if recorded and recorded != test_id:
        raise ValueError(f"mapping identity drift: {recorded!r} != {test_id!r}")
    return test_id


def load_rows() -> list[dict[str, str]]:
    with MAPPING.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream))
    if len(rows) != EXPECTED_MAPPING:
        raise ValueError(f"mapping row count changed: {len(rows)}")
    return rows


def exact_ids(summary: dict) -> list[str]:
    return sorted(
        item["id"]
        for item in summary.get("tests", [])
        if item.get("status") == "pass" and item.get("mismatch_pct") == 0.0
    )


def strict_syntax_inventory(rows: list[dict[str, str]]) -> list[str]:
    ids = []
    for row in rows:
        upstream = WPT_ROOT / row["chromium_test_path"].strip()
        if not upstream.is_file():
            raise FileNotFoundError(upstream)
        html = upstream.read_text(encoding="utf-8", errors="ignore")
        if PSEUDO_SYNTAX.search(html) or PROPERTY_SYNTAX.search(html):
            ids.append(canonical_id(row))
    ids.sort()
    if ids != sorted(set(ids)) or len(ids) != EXPECTED_INVENTORY:
        raise ValueError(f"SP18 strict syntax inventory changed: {len(ids)}")
    return ids


def load_id_manifest(path: Path, expected_count: int) -> list[str]:
    expected_hash = MANIFEST_SHA256[path]
    actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual_hash != expected_hash:
        raise ValueError(f"SP18 manifest byte drift: {path.name}: {actual_hash}")
    values = json.loads(path.read_text(encoding="utf-8"))
    if values != sorted(set(values)) or len(values) != expected_count:
        raise ValueError(
            f"SP18 {path.name} is not the sorted {expected_count}-ID set"
        )
    return values


def load_manifests() -> tuple[list[str], list[str], list[str], list[str], list[str], list[str]]:
    return (
        load_id_manifest(BASELINE, EXPECTED_BASELINE),
        load_id_manifest(INVENTORY, EXPECTED_INVENTORY),
        load_id_manifest(TARGETS, EXPECTED_TARGETS),
        load_id_manifest(RESIDUAL_IDS, EXPECTED_RESIDUAL_IDS),
        load_id_manifest(FOCUSED, EXPECTED_FOCUSED),
        load_id_manifest(RUNNABLE_RESIDUALS, EXPECTED_RUNNABLE_RESIDUALS),
    )


def load_verified_misses() -> list[dict]:
    expected_hash = MANIFEST_SHA256[VERIFIED_MISSES]
    actual_hash = hashlib.sha256(VERIFIED_MISSES.read_bytes()).hexdigest()
    if actual_hash != expected_hash:
        raise ValueError(f"SP18 verified-miss byte drift: {actual_hash}")
    values = json.loads(VERIFIED_MISSES.read_text(encoding="utf-8"))
    required = {
        "test_id", "chromium_test_path", "mismatch_pct", "rejection_reason",
        "rejection_owner", "owner_categories",
    }
    if (
        len(values) != EXPECTED_VERIFIED_MISSES
        or [item.get("test_id") for item in values]
        != sorted(item.get("test_id") for item in values)
    ):
        raise ValueError("SP18 verified-miss inventory changed")
    for item in values:
        if (
            set(item) != required
            or not item["chromium_test_path"]
            or not isinstance(item["mismatch_pct"], float)
            or item["mismatch_pct"] <= 0.0
            or not item["rejection_reason"].startswith("verified_pixel_mismatch: ")
            or item["rejection_owner"] not in item["owner_categories"]
            or item["owner_categories"] != sorted(set(item["owner_categories"]))
        ):
            raise ValueError(f"invalid SP18 verified miss: {item!r}")
    return values


def validate_partitions(
    baseline: list[str], inventory: list[str], targets: list[str],
    residual_ids: list[str], focused: list[str], runnable_residuals: list[str],
) -> None:
    baseline_set = set(baseline)
    inventory_set = set(inventory)
    target_set = set(targets)
    residual_set = set(residual_ids)
    if target_set & residual_set or target_set | residual_set != inventory_set - baseline_set:
        raise ValueError("SP18 target/residual projection does not cover new inventory IDs")
    if focused != sorted(baseline_set | target_set):
        raise ValueError("SP18 focused proof is not baseline union targets")
    if not set(runnable_residuals).isdisjoint(target_set):
        raise ValueError("SP18 target and runnable-residual sets overlap")
    area_counts = Counter(test_id.split("/")[1] for test_id in targets)
    if dict(area_counts) != TARGET_AREA_COUNTS:
        raise ValueError(f"SP18 target area partition changed: {dict(area_counts)}")


def _external_owners(html: str, test_id: str, reason: str) -> tuple[str, list[str]]:
    dependencies = classify_dependencies(
        html, test_id=test_id, excluded=COHORT_DEPENDENCIES
    )
    rejection_dependency = dependency_for_portability_reason(reason)
    if rejection_dependency in COHORT_DEPENDENCIES:
        raise ValueError(f"SP18 residual still has an in-scope rejection: {test_id}: {reason}")
    if rejection_dependency not in CATEGORY_FOR_DEP:
        if rejection_dependency == "root_body_layout":
            rejection_dependency = "non_visual"
        else:
            raise ValueError(f"unowned SP18 rejection: {test_id}: {reason}")
    if rejection_dependency not in dependencies:
        dependencies.append(rejection_dependency)
    rejection_owner = CATEGORY_FOR_DEP[rejection_dependency]
    owners = sorted({CATEGORY_FOR_DEP[dep] for dep in dependencies})
    if rejection_owner not in owners or not (set(owners) - METADATA_OWNERS):
        raise ValueError(f"incomplete SP18 residual ownership: {test_id}")
    return rejection_owner, owners


def build_residual_dispositions(
    rows: list[dict[str, str]], residual_ids: list[str],
) -> list[dict]:
    by_id = {canonical_id(row): row for row in rows}
    verified = {item["test_id"]: item for item in load_verified_misses()}
    if not set(verified).issubset(residual_ids):
        raise ValueError("SP18 verified misses escaped the residual projection")
    result = []
    old_profile = port_wpt.ACTIVE_PORTER_PROFILE
    old_emit = port_wpt.EMIT_TEXT_NODES
    old_retain = port_wpt.RETAIN_TEXT
    try:
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        for test_id in residual_ids:
            if test_id in verified:
                result.append(dict(verified[test_id]))
                continue
            row = by_id[test_id]
            chromium_path = row["chromium_test_path"].strip()
            upstream = WPT_ROOT / chromium_path
            html = upstream.read_text(encoding="utf-8", errors="ignore")
            parser = port_wpt.parse_wpt_html(str(upstream))
            portable, reason = port_wpt.analyze_portability(parser)
            if portable and not port_wpt.has_layout_content(parser):
                portable, reason = False, "no_layout_content"
            if portable:
                raise ValueError(f"SP18 residual became portable: {test_id}")
            rejection_owner, owners = _external_owners(html, test_id, reason)
            result.append({
                "test_id": test_id,
                "chromium_test_path": chromium_path,
                "rejection_reason": reason,
                "rejection_owner": rejection_owner,
                "owner_categories": owners,
            })
    finally:
        port_wpt.ACTIVE_PORTER_PROFILE = old_profile
        port_wpt.EMIT_TEXT_NODES = old_emit
        port_wpt.RETAIN_TEXT = old_retain
    validate_residual_dispositions(result, residual_ids)
    return result


def validate_residual_dispositions(result: list[dict], residual_ids: list[str]) -> None:
    required = {
        "test_id", "chromium_test_path", "rejection_reason", "rejection_owner",
        "owner_categories",
    }
    if [item.get("test_id") for item in result] != residual_ids:
        raise ValueError("SP18 structured residual IDs changed")
    verified_ids = {item["test_id"] for item in load_verified_misses()}
    for item in result:
        item_required = required | ({"mismatch_pct"} if item["test_id"] in verified_ids else set())
        if (
            set(item) != item_required
            or not item["chromium_test_path"]
            or not item["rejection_reason"]
            or item["rejection_owner"] not in item["owner_categories"]
            or item["owner_categories"] != sorted(set(item["owner_categories"]))
        ):
            raise ValueError(f"invalid SP18 residual disposition: {item!r}")


def build_outputs(rows: list[dict[str, str]]) -> dict[Path, str]:
    baseline, inventory, targets, residual_ids, focused, runnable_residuals = load_manifests()
    validate_partitions(
        baseline, inventory, targets, residual_ids, focused, runnable_residuals
    )
    actual_inventory = strict_syntax_inventory(rows)
    if actual_inventory != inventory:
        raise ValueError("SP18 strict upstream syntax inventory drifted")
    dispositions = build_residual_dispositions(rows, residual_ids)
    return {RESIDUAL_DISPOSITIONS: encoded(dispositions)}


def validate_live_snapshot(rows: list[dict[str, str]], summary: dict) -> None:
    baseline, inventory, targets, residual_ids, focused, runnable_residuals = load_manifests()
    validate_partitions(
        baseline, inventory, targets, residual_ids, focused, runnable_residuals
    )
    tests = summary.get("tests", [])
    ids = [item.get("id") for item in tests]
    if len(ids) == 3889:
        # The frozen kickoff state is accepted so no-write generation can be
        # verified before the surgical splice.
        if exact_ids(summary) != baseline:
            raise ValueError("SP18 kickoff exact baseline drifted")
        return
    status = Counter(item.get("status") for item in tests)
    if (
        len(tests) != EXPECTED_FINAL_RUNNABLE
        or len(exact_ids(summary)) != EXPECTED_FINAL_EXACT
        or status["fail"] != EXPECTED_FINAL_FAILURES
        or status["error"] != EXPECTED_FINAL_ERRORS
    ):
        raise ValueError(f"SP18 final summary totals changed: {dict(status)}, {len(tests)}")
    by_id = {item["id"]: item for item in tests}
    if any(
        by_id.get(test_id, {}).get("status") != "pass"
        or by_id[test_id].get("mismatch_pct") != 0.0
        for test_id in focused
    ):
        raise ValueError("SP18 focused proof is not 4,058/4,058 exact")
    failures = sorted(item["id"] for item in tests if item.get("status") == "fail")
    if failures != runnable_residuals:
        raise ValueError("SP18 runnable residual failure set changed")
    ported = sum(row.get("ported") == "yes" for row in rows)
    if len(rows) - ported != EXPECTED_FINAL_UNPORTED:
        raise ValueError(f"SP18 final unported count changed: {len(rows) - ported}")


def check() -> None:
    rows = load_rows()
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    expected_summary_hash = (
        BASELINE_SUMMARY_SHA256
        if len(summary.get("tests", [])) == 3889
        else FINAL_SUMMARY_SHA256
    )
    actual_summary_hash = hashlib.sha256(SUMMARY.read_bytes()).hexdigest()
    if actual_summary_hash != expected_summary_hash:
        raise ValueError(
            f"SP18 summary byte drift: {actual_summary_hash}"
        )
    outputs = build_outputs(rows)
    for path, content in outputs.items():
        if not path.is_file() or path.read_text(encoding="utf-8") != content:
            raise ValueError(f"SP18 closure ledger drift: {path}")
    validate_live_snapshot(rows, summary)


def main() -> int:
    if sys.argv[1:] not in ([], ["--check"]):
        print("Usage: generate_sp18_closure.py [--check]", file=sys.stderr)
        return 2
    rows = load_rows()
    if sys.argv[1:] == ["--check"]:
        check()
    else:
        outputs = build_outputs(rows)
        for path, content in outputs.items():
            path.write_text(content, encoding="utf-8")
    print(
        "SP18 closure: baseline=3807 inventory=552 targets=251 "
        "focused=4058 residuals=292 verified-misses=3 runnable-residuals=81"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
