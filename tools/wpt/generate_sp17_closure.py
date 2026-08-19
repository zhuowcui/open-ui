#!/usr/bin/env python3
"""Freeze and validate the SP17 writing-mode kickoff ledgers.

W0A deliberately freezes only facts available before porter expansion.  The
final actionable/residual split belongs to W0B, after transactional probing of
the directly rejected writing-mode and unicode-bidi declarations.
"""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent
DATA_DIR = PROJECT_ROOT / "tools" / "accountability" / "data"
PORTED_DIR = DATA_DIR / "wpt_ported"
RESULTS_DIR = DATA_DIR / "pixel_comparison" / "results"
MAPPING_CSV = DATA_DIR / "wpt_mapping.csv"
SUMMARY_JSON = RESULTS_DIR / "summary.json"

BASELINE_JSON = PORTED_DIR / "sp17_baseline_exact.json"
INVENTORY_JSON = PORTED_DIR / "sp17_writing_mode_inventory.json"
INITIAL_TARGETS_JSON = PORTED_DIR / "sp17_initial_runnable_targets.json"
INITIAL_RESULTS_JSON = PORTED_DIR / "sp17_initial_runnable_results.json"

SP17_CATEGORY = "needs_writing_mode"
EXPECTED_MAPPING = 7673
EXPECTED_INVENTORY = 842
EXPECTED_RUNNABLE_OWNED = 19
EXPECTED_UNPORTED_OWNED = 823
EXPECTED_DIRECT_WRITING_MODE = 337
EXPECTED_DIRECT_UNICODE_BIDI = 3
EXPECTED_RUNNABLE = 3566
EXPECTED_BASELINE = 3267
EXPECTED_FAILURES = 299
EXPECTED_ERRORS = 0
EXPECTED_UNPORTED = 4107

# These files are historical evidence, not SP17 generator output.  Pinning
# their bytes makes accidental regeneration visible immediately.
HISTORICAL_LEDGER_SHA256 = {
    "sp13r_baseline_exact.json": "07305185d51727fca6a0b739f2c87b0da18963e93216407868d6284d89e98c1c",
    "sp13r_multicol_residuals.json": "1268c9bbdc9f36ef23fc86204ac1c34259340711fe34dfca4fcd67b3ab45de67",
    "sp13r_multicol_targets.json": "140edff40c172d06b6e2dc3d91e27adc0fddf12bf9c12c04ea9cdfa7c73dd6b9",
    "sp14_w2_targets.json": "8ef76f5f38baaaaed12542285b9716b57a4fda1a318d35ecfbb3633904fee600",
    "sp14_w3_baseline_exact.json": "6c350c67d62a2a5868fb3346851db02f16ca293ae7cd24f113df28ee6236c078",
    "sp14_w3_targets.json": "ead9d861c44ec296a99a3b0887e14d5e66f456f9de368ba8cdf89c02620ea86d",
    "sp14_w4_residuals.json": "258e001a0fbe41adfe2008899ba09d746aea8a33bbe21b0bf532711491149f4a",
    "sp15_actionable_targets.json": "210023e7342d96acc9cd491521e4f100c6fe3a2cbc4bf56a596f05b18405f9ad",
    "sp15_baseline_exact.json": "9dcebde69adc70b39660ef8fff771af6f57339c0fd4dd242109a22bf5a4d391d",
    "sp15_residual_dispositions.json": "f901127ee8ae19a150965a6edbc121185f2a47b9d6d121cc895b453ae4d430af",
    "sp16_actionable_targets.json": "fc457282f5335773d1081fa9a949aaf29d401bac6dcaf38560564a1bcf3aef86",
    "sp16_baseline_exact.json": "668387215999c79c1dbae11fd8a0e06aceaf0c497b1b3afeda1b31725aa3b668",
    "sp16_real_font_tests.json": "fc457282f5335773d1081fa9a949aaf29d401bac6dcaf38560564a1bcf3aef86",
    "sp16_residual_dispositions.json": "72df8f9e2d8c6141cf266d51a4d2114101592b58dd006494b5c8eb9196d04bcc",
}


def categories(value: str) -> set[str]:
    return {part.strip() for part in value.split(",") if part.strip()}


def canonical_id(row: dict[str, str]) -> str:
    test_id = f"wpt/{row['sp_area'].strip()}/{row['test_name'].strip()}"
    recorded = row.get("our_test_id", "").strip()
    if recorded and recorded != test_id:
        raise ValueError(f"mapping identity drift: {recorded!r} != {test_id!r}")
    return test_id


def first_rejection(row: dict[str, str]) -> str | None:
    if row.get("ported") == "yes":
        return None
    value = row.get("notes", "").removeprefix("Porter deferred: ").strip()
    if not value:
        raise ValueError(f"unported row has no rejection reason: {canonical_id(row)}")
    return value


def build_baseline(summary: dict) -> list[str]:
    baseline = sorted(
        item["id"]
        for item in summary.get("tests", [])
        if item.get("status") == "pass" and item.get("mismatch_pct") == 0.0
    )
    if baseline != sorted(set(baseline)) or len(baseline) != EXPECTED_BASELINE:
        raise ValueError(
            f"SP17 exact baseline is not the sorted {EXPECTED_BASELINE}-ID set"
        )
    return baseline


def build_inventory(rows: list[dict[str, str]]) -> list[dict]:
    inventory = []
    for row in sorted(
        (row for row in rows if SP17_CATEGORY in categories(row["failure_category"])),
        key=canonical_id,
    ):
        inventory.append({
            "test_id": canonical_id(row),
            "kickoff_state": "runnable" if row["ported"] == "yes" else "unported",
            "chromium_test_path": row["chromium_test_path"].strip(),
            "first_rejection": first_rejection(row),
            "owner_categories": sorted(categories(row["failure_category"])),
        })
    validate_inventory(inventory)
    return inventory


def build_initial_targets(inventory: list[dict]) -> list[str]:
    targets = sorted(
        item["test_id"]
        for item in inventory
        if item["kickoff_state"] == "runnable"
    )
    if targets != sorted(set(targets)) or len(targets) != EXPECTED_RUNNABLE_OWNED:
        raise ValueError("SP17 initial target ledger is not the sorted 19-ID set")
    return targets


def validate_inventory(inventory: list[dict]) -> None:
    required = {
        "test_id", "kickoff_state", "chromium_test_path", "first_rejection",
        "owner_categories",
    }
    ids = [item.get("test_id", "") for item in inventory if isinstance(item, dict)]
    if ids != sorted(set(ids)) or len(inventory) != EXPECTED_INVENTORY:
        raise ValueError("SP17 inventory is not the sorted frozen 842-row set")
    for item in inventory:
        owners = item.get("owner_categories", [])
        state = item.get("kickoff_state")
        if (
            set(item) != required
            or state not in {"runnable", "unported"}
            or not item.get("chromium_test_path")
            or owners != sorted(set(owners))
            or SP17_CATEGORY not in owners
            or (state == "runnable") != (item.get("first_rejection") is None)
        ):
            raise ValueError(f"invalid SP17 inventory row: {item!r}")
    runnable = sum(item["kickoff_state"] == "runnable" for item in inventory)
    if runnable != EXPECTED_RUNNABLE_OWNED:
        raise ValueError(f"SP17 runnable kickoff inventory changed: {runnable}")
    if len(inventory) - runnable != EXPECTED_UNPORTED_OWNED:
        raise ValueError("SP17 unported kickoff inventory changed")
    rejections = [item["first_rejection"] for item in inventory]
    writing_mode = sum(
        value in {
            "style_block_unsupported property: writing-mode",
            "unsupported property: writing-mode",
        }
        for value in rejections
    )
    unicode_bidi = rejections.count("style_block_unsupported property: unicode-bidi")
    if writing_mode != EXPECTED_DIRECT_WRITING_MODE or unicode_bidi != EXPECTED_DIRECT_UNICODE_BIDI:
        raise ValueError("SP17 direct-property rejection partition changed")


def result_path(test_id: str) -> Path:
    return RESULTS_DIR / Path(test_id) / "result.json"


def build_initial_results(
    targets: list[str], summary: dict, *, require_focused: bool = False,
) -> dict:
    summary_tests = summary.get("tests", [])
    summary_by_id = {item.get("id"): item for item in summary_tests}
    if require_focused and (
        len(summary_tests) != len(targets) or set(summary_by_id) != set(targets)
    ):
        raise ValueError("initial evidence capture requires the exact 19-ID summary")
    missing = set(targets) - set(summary_by_id)
    if missing:
        raise ValueError(f"summary is missing SP17 initial target: {min(missing)}")

    results = []
    for test_id in targets:
        path = result_path(test_id)
        if not path.is_file():
            raise ValueError(f"missing pixel result evidence: {path}")
        raw = json.loads(path.read_text(encoding="utf-8"))
        result = {
            "test_id": test_id,
            "status": raw.get("status"),
            "mismatched_pixels": raw.get("mismatched_pixels"),
            "mismatch_pct": raw.get("mismatch_pct"),
            "total_pixels": raw.get("total_pixels"),
            "size_match": raw.get("size_match"),
            "max_channel_diff": raw.get("max_channel_diff"),
            "avg_channel_diff": raw.get("avg_channel_diff"),
        }
        if result["status"] != summary_by_id[test_id].get("status"):
            raise ValueError(f"summary/result status mismatch: {test_id}")
        results.append(result)
    evidence = {
        "schema_version": 1,
        "run_scope": "SP17 kickoff exact 19-ID manifest without resume",
        "chromium_build": "147.0.7727.50-linux",
        "viewport": {"width": 800, "height": 600},
        "tests": results,
    }
    validate_initial_results(evidence, targets)
    return evidence


def validate_initial_results(evidence: dict, targets: list[str]) -> None:
    required_top = {
        "schema_version", "run_scope", "chromium_build", "viewport", "tests"
    }
    required_result = {
        "test_id", "status", "mismatched_pixels", "mismatch_pct", "total_pixels",
        "size_match", "max_channel_diff", "avg_channel_diff",
    }
    tests = evidence.get("tests", []) if isinstance(evidence, dict) else []
    ids = [item.get("test_id", "") for item in tests if isinstance(item, dict)]
    if set(evidence) != required_top or ids != targets:
        raise ValueError("SP17 initial result evidence does not match the 19-ID manifest")
    if evidence.get("schema_version") != 1 or evidence.get("viewport") != {
        "width": 800, "height": 600,
    }:
        raise ValueError("SP17 initial result provenance changed")
    for item in tests:
        if (
            set(item) != required_result
            or item.get("status") != "fail"
            or not isinstance(item.get("mismatched_pixels"), int)
            or item["mismatched_pixels"] <= 0
            or not isinstance(item.get("mismatch_pct"), (int, float))
            or item["mismatch_pct"] <= 0.0
            or item.get("total_pixels") != 471000
            or item.get("size_match") is not True
        ):
            raise ValueError(f"invalid SP17 initial result: {item!r}")


def validate_historical_ledgers() -> None:
    for name, expected in HISTORICAL_LEDGER_SHA256.items():
        path = PORTED_DIR / name
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f"historical ledger byte drift: {name}: {actual}")


def validate_full_snapshot(rows: list[dict[str, str]], summary: dict) -> None:
    if len(rows) != EXPECTED_MAPPING:
        raise ValueError(f"SP17 Chromium inventory changed: {len(rows)}")
    if sum(row.get("ported") == "yes" for row in rows) != EXPECTED_RUNNABLE:
        raise ValueError("SP17 runnable mapping inventory changed")
    if sum(row.get("ported") == "no" for row in rows) != EXPECTED_UNPORTED:
        raise ValueError("SP17 unported mapping inventory changed")
    tests = summary.get("tests", [])
    counts = (
        len(tests), summary.get("passed"), summary.get("failed"), summary.get("errors")
    )
    expected = (
        EXPECTED_RUNNABLE, EXPECTED_BASELINE, EXPECTED_FAILURES, EXPECTED_ERRORS
    )
    if counts != expected:
        raise ValueError(f"SP17 full pixel snapshot changed: {counts} != {expected}")
    ids = [item.get("id") for item in tests]
    if len(set(ids)) != EXPECTED_RUNNABLE:
        raise ValueError("SP17 full pixel summary contains duplicate IDs")


def encoded(value: object) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def load_inputs() -> tuple[list[dict[str, str]], dict]:
    rows = list(csv.DictReader(MAPPING_CSV.open(newline="", encoding="utf-8")))
    summary = json.loads(SUMMARY_JSON.read_text(encoding="utf-8"))
    return rows, summary


def build_outputs(rows: list[dict[str, str]], summary: dict) -> dict[Path, str]:
    validate_full_snapshot(rows, summary)
    baseline = build_baseline(summary)
    inventory = build_inventory(rows)
    targets = build_initial_targets(inventory)
    evidence = build_initial_results(targets, summary)
    if set(baseline) & set(targets):
        raise ValueError("SP17 exact baseline overlaps the failing initial target slice")
    return {
        BASELINE_JSON: encoded(baseline),
        INVENTORY_JSON: encoded(inventory),
        INITIAL_TARGETS_JSON: encoded(targets),
        INITIAL_RESULTS_JSON: encoded(evidence),
    }


def load_ledgers() -> tuple[list, list, list, dict]:
    baseline = json.loads(BASELINE_JSON.read_text(encoding="utf-8"))
    inventory = json.loads(INVENTORY_JSON.read_text(encoding="utf-8"))
    targets = json.loads(INITIAL_TARGETS_JSON.read_text(encoding="utf-8"))
    evidence = json.loads(INITIAL_RESULTS_JSON.read_text(encoding="utf-8"))
    if baseline != sorted(set(baseline)) or len(baseline) != EXPECTED_BASELINE:
        raise ValueError("invalid frozen SP17 exact baseline")
    validate_inventory(inventory)
    if build_initial_targets(inventory) != targets:
        raise ValueError("SP17 initial target ledger does not match the inventory")
    validate_initial_results(evidence, targets)
    return baseline, inventory, targets, evidence


def main() -> int:
    usage = "Usage: generate_sp17_closure.py [--check|--capture-initial-results]"
    if sys.argv[1:] not in ([], ["--check"], ["--capture-initial-results"]):
        print(usage, file=sys.stderr)
        return 2

    validate_historical_ledgers()
    rows, summary = load_inputs()
    if sys.argv[1:] == ["--capture-initial-results"]:
        _, inventory, targets, _ = load_ledgers()
        if build_inventory(rows) != inventory or build_initial_targets(inventory) != targets:
            raise ValueError("SP17 mapping drifted before initial evidence capture")
        evidence = build_initial_results(targets, summary, require_focused=True)
        INITIAL_RESULTS_JSON.write_text(encoded(evidence), encoding="utf-8")
    else:
        outputs = build_outputs(rows, summary)
        if sys.argv[1:] == ["--check"]:
            drift = [
                path for path, content in outputs.items()
                if not path.is_file() or path.read_text(encoding="utf-8") != content
            ]
            if drift:
                raise ValueError("SP17 kickoff ledger drift: " + ", ".join(map(str, drift)))
            load_ledgers()
        else:
            for path, content in outputs.items():
                path.write_text(content, encoding="utf-8")
    print("SP17 W0A ledgers: baseline=3267, inventory=842, initial=19")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
