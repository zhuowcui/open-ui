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
import os
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
ACTIONABLE_JSON = PORTED_DIR / "sp17_actionable_targets.json"
RESIDUALS_JSON = PORTED_DIR / "sp17_residual_dispositions.json"
W1M_TARGETS_JSON = SCRIPT_DIR / "sp17_w1m_targets.json"
W1M_FOCUSED_JSON = SCRIPT_DIR / "sp17_w1m_focused_ids.json"
W1N_TARGETS_JSON = SCRIPT_DIR / "sp17_w1n_targets.json"
W1N_FOCUSED_JSON = SCRIPT_DIR / "sp17_w1n_focused_ids.json"
W1O_TARGETS_JSON = SCRIPT_DIR / "sp17_w1o_targets.json"
W1O_FOCUSED_JSON = SCRIPT_DIR / "sp17_w1o_focused_ids.json"
W2A_TARGETS_JSON = SCRIPT_DIR / "sp17_w2a_targets.json"
W2A_FOCUSED_JSON = SCRIPT_DIR / "sp17_w2a_focused_ids.json"
W2B_W4_TARGETS_JSON = SCRIPT_DIR / "sp17_w2b_w4_targets.json"
W2B_W4_FOCUSED_JSON = SCRIPT_DIR / "sp17_w2b_w4_focused_ids.json"
SP13P_TARGETS_JSON = PORTED_DIR / "sp13p_paint_targets.json"
WPT_ROOT = Path(os.environ.get(
    "CHROMIUM_WPT_CSS",
    os.path.expanduser(
        "~/chromium/src/third_party/blink/web_tests/external/wpt/css"
    ),
))

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
EXPECTED_ACTIONABLE = 311
EXPECTED_RESIDUALS = 531
EXPECTED_PROBE_SP17_RESIDUALS = 2
EXPECTED_W1M_TARGETS = 29
EXPECTED_W1M_FOCUSED = 58
EXPECTED_W1M_RESIDUAL_ADMISSIONS = 12
EXPECTED_W1M_NON_SP17_ADMISSIONS = 17
EXPECTED_W1M_PROMOTIONS = 166
EXPECTED_W1M_RUNNABLE = 3746
EXPECTED_W1M_EXACT = 3460
EXPECTED_W1M_FAILURES = 286
EXPECTED_W1M_UNPORTED = 3927
EXPECTED_W1M_LIVE_OWNED = 676
W1M_MANIFEST_SHA256 = {
    "sp17_w1m_targets.json": "778eb7fe073c718ec36f7ed187865c7831699bcd09ef9f4d32f531739d50f496",
    "sp17_w1m_focused_ids.json": "65ad5944ee26aaed03ab29b28209b72a8fc5ddca8666bd765f9d04196c407c7e",
}
EXPECTED_W1N_TARGETS = 4
EXPECTED_W1N_FOCUSED = 19
EXPECTED_W1N_PROMOTIONS = 170
EXPECTED_W1N_RUNNABLE = 3746
EXPECTED_W1N_EXACT = 3464
EXPECTED_W1N_FAILURES = 282
EXPECTED_W1N_UNPORTED = 3927
EXPECTED_W1N_LIVE_OWNED = 672
W1N_MANIFEST_SHA256 = {
    "sp17_w1n_targets.json": "4b7c151d7f87f72cd8af93ba97d94e3e7cd2638104a87798f604b2da927ddff4",
    "sp17_w1n_focused_ids.json": "f4fbe9fea8048642b0a5461cfcc0b9cf73f1d77bdcc6fa83a6c5e3fe30f3a9d1",
}
EXPECTED_W1O_TARGETS = 1
EXPECTED_W1O_FOCUSED = 15
EXPECTED_W1O_PROMOTIONS = 171
EXPECTED_W1O_RUNNABLE = 3746
EXPECTED_W1O_EXACT = 3465
EXPECTED_W1O_FAILURES = 281
EXPECTED_W1O_UNPORTED = 3927
EXPECTED_W1O_LIVE_OWNED = 671
W1O_MANIFEST_SHA256 = {
    "sp17_w1o_targets.json": "2170b9c4a44cc2419c482b66ad723e45d8b10c9a7ab2d692aac4205bc8c5bcdb",
    "sp17_w1o_focused_ids.json": "dba54fcf13c00bc42737cbaae0d1c459701f569849239621f4f83828bccb21d6",
}
EXPECTED_W2A_TARGETS = 22
EXPECTED_W2A_FOCUSED = 49
EXPECTED_W2A_PROMOTIONS = 193
EXPECTED_W2A_RUNNABLE = 3768
EXPECTED_W2A_EXACT = 3487
EXPECTED_W2A_FAILURES = 281
EXPECTED_W2A_UNPORTED = 3905
EXPECTED_W2A_LIVE_OWNED = 649
W2A_MANIFEST_SHA256 = {
    "sp17_w2a_targets.json": "4162bd75b614a81ab43c897aef201456660b35f6cdac8109c41b1aa126ec963f",
    "sp17_w2a_focused_ids.json": "b44b3ec2d5c1aea2e6e159f923858d66cdb24ff9f4a958a75ec8092771012511",
}
EXPECTED_W2B_W4_TARGETS = 132
EXPECTED_W2B_W4_FOCUSED = 325
EXPECTED_W2B_W4_PROMOTIONS = 325
EXPECTED_W2B_W4_RUNNABLE = 3889
EXPECTED_W2B_W4_EXACT = 3619
EXPECTED_W2B_W4_FAILURES = 270
EXPECTED_W2B_W4_UNPORTED = 3784
EXPECTED_W2B_W4_LIVE_OWNED = 517
EXPECTED_W2B_W4_RESIDUAL_ADMISSIONS = 2
EXPECTED_SP13P_LATER_EXACT = 188
SP13P_TARGETS_SHA256 = "322d86866abdd23d14e435cba71e7f1239d49f7ad1914ac607ac7a5b7a387324"
W2B_W4_MANIFEST_SHA256 = {
    "sp17_w2b_w4_targets.json": "0085f0df34162f355c1f2a24deae01967cf52f1753049f27a32ec7425e3ce089",
    "sp17_w2b_w4_focused_ids.json": "78efe59229615167e9603c6e40295c3eca0937c2f973f1097cd98a545d2793d3",
}
W2A_EXISTING_EXACT_GUARDS = frozenset({
    "wpt/css_flexbox/flexbox-writing-mode-slr-ref",
})
W1M_EXISTING_RUNNABLE = frozenset({
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-fallback-justify-content-001",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-fallback-justify-content-001-ref",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-justify-self-001",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-justify-self-001-ref",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-margin-001",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-margin-001-ref",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-margin-002",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-margin-002-ref",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-margin-003",
    "wpt/css_flexbox/abspos_flex-abspos-staticpos-margin-003-ref",
})
KICKOFF_LEDGER_SHA256 = {
    "sp17_baseline_exact.json": "59a514d3b76b83ecc44efd43dda5a16ec2a0203803849407f3dce035d9e9fc20",
    "sp17_writing_mode_inventory.json": "b72a0b0b4e74f4c1cb912ab65642dd6f5bef76a570f0a9b219f68729de6d10ad",
    "sp17_initial_runnable_targets.json": "f82d99182bf276ddd524b2894e2de6b21e8f1f5bf7d8f3bd984c8ef0e2c040c4",
    "sp17_initial_runnable_results.json": "1fcc02a0e742df04cec80e06750f74e6859ef11552d5488300c590095432d00d",
}
PROBE_LEDGER_SHA256 = {
    "sp17_actionable_targets.json": "9d2b53070cf206b6c37a5c66bd0d37ea757e12b7ca6d4a19a5eb98ee4579f96c",
    "sp17_residual_dispositions.json": "314a7a27f250ef1fb5f65b86e69f48771e5116a4b6592bce2190d474a9338776",
}

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

sys.path.insert(0, str(SCRIPT_DIR))
import port_wpt  # noqa: E402

ACCOUNTABILITY_DIR = PROJECT_ROOT / "tools" / "accountability"
sys.path.insert(0, str(ACCOUNTABILITY_DIR))
from shared_detectors import (  # noqa: E402
    CATEGORY_FOR_DEP,
    classify_dependencies,
    dependency_for_portability_reason,
)

METADATA_CATEGORIES = {"reference_test", "non_visual_test"}


def categories(value: str) -> set[str]:
    return {part.strip() for part in value.split(",") if part.strip()}


def canonical_id(row: dict[str, str]) -> str:
    test_id = f"wpt/{row['sp_area'].strip()}/{row['test_name'].strip()}"
    recorded = row.get("our_test_id", "").strip()
    if recorded and recorded != test_id:
        raise ValueError(f"mapping identity drift: {recorded!r} != {test_id!r}")
    return test_id


def load_w1m_manifests() -> tuple[list[str], list[str]]:
    manifests = []
    for path, expected_count in (
        (W1M_TARGETS_JSON, EXPECTED_W1M_TARGETS),
        (W1M_FOCUSED_JSON, EXPECTED_W1M_FOCUSED),
    ):
        actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual_hash != W1M_MANIFEST_SHA256[path.name]:
            raise ValueError(f"SP17 W1M manifest byte drift: {path.name}: {actual_hash}")
        values = json.loads(path.read_text(encoding="utf-8"))
        if values != sorted(set(values)) or len(values) != expected_count:
            raise ValueError(
                f"SP17 W1M manifest is not the sorted {expected_count}-ID set: {path.name}"
            )
        manifests.append(values)
    targets, focused = manifests
    if not set(targets).isdisjoint(W1M_EXISTING_RUNNABLE):
        raise ValueError("SP17 W1M new and existing runnable cohorts overlap")
    if not set(targets) | W1M_EXISTING_RUNNABLE <= set(focused):
        raise ValueError("SP17 W1M focused proof silently shrank the 39-ID cohort")
    return targets, focused


def load_w1n_manifests() -> tuple[list[str], list[str]]:
    manifests = []
    for path, expected_count in (
        (W1N_TARGETS_JSON, EXPECTED_W1N_TARGETS),
        (W1N_FOCUSED_JSON, EXPECTED_W1N_FOCUSED),
    ):
        actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual_hash != W1N_MANIFEST_SHA256[path.name]:
            raise ValueError(f"SP17 W1N manifest byte drift: {path.name}: {actual_hash}")
        values = json.loads(path.read_text(encoding="utf-8"))
        if values != sorted(set(values)) or len(values) != expected_count:
            raise ValueError(
                f"SP17 W1N manifest is not the sorted {expected_count}-ID set: {path.name}"
            )
        manifests.append(values)
    targets, focused = manifests
    if not set(targets) <= set(focused):
        raise ValueError("SP17 W1N focused proof silently dropped a target")
    if (
        EXPECTED_W1N_PROMOTIONS != EXPECTED_W1M_PROMOTIONS + len(targets)
        or EXPECTED_W1N_RUNNABLE != EXPECTED_W1M_RUNNABLE
        or EXPECTED_W1N_UNPORTED != EXPECTED_W1M_UNPORTED
        or EXPECTED_W1N_EXACT != EXPECTED_W1M_EXACT + len(targets)
        or EXPECTED_W1N_FAILURES != EXPECTED_W1M_FAILURES - len(targets)
        or EXPECTED_W1N_LIVE_OWNED != EXPECTED_W1M_LIVE_OWNED - len(targets)
    ):
        raise ValueError("SP17 W1N is not the projected four-ID delta from W1M")
    return targets, focused


def load_w1o_manifests() -> tuple[list[str], list[str]]:
    manifests = []
    for path, expected_count in (
        (W1O_TARGETS_JSON, EXPECTED_W1O_TARGETS),
        (W1O_FOCUSED_JSON, EXPECTED_W1O_FOCUSED),
    ):
        actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual_hash != W1O_MANIFEST_SHA256[path.name]:
            raise ValueError(f"SP17 W1O manifest byte drift: {path.name}: {actual_hash}")
        values = json.loads(path.read_text(encoding="utf-8"))
        if values != sorted(set(values)) or len(values) != expected_count:
            raise ValueError(
                f"SP17 W1O manifest is not the sorted {expected_count}-ID set: {path.name}"
            )
        manifests.append(values)
    targets, focused = manifests
    if not set(targets) <= set(focused):
        raise ValueError("SP17 W1O focused proof silently dropped its target")
    if (
        EXPECTED_W1O_PROMOTIONS != EXPECTED_W1N_PROMOTIONS + len(targets)
        or EXPECTED_W1O_RUNNABLE != EXPECTED_W1N_RUNNABLE
        or EXPECTED_W1O_UNPORTED != EXPECTED_W1N_UNPORTED
        or EXPECTED_W1O_EXACT != EXPECTED_W1N_EXACT + len(targets)
        or EXPECTED_W1O_FAILURES != EXPECTED_W1N_FAILURES - len(targets)
        or EXPECTED_W1O_LIVE_OWNED != EXPECTED_W1N_LIVE_OWNED - len(targets)
    ):
        raise ValueError("SP17 W1O is not the projected one-ID delta from W1N")
    return targets, focused


def load_w2a_manifests() -> tuple[list[str], list[str]]:
    manifests = []
    for path, expected_count in (
        (W2A_TARGETS_JSON, EXPECTED_W2A_TARGETS),
        (W2A_FOCUSED_JSON, EXPECTED_W2A_FOCUSED),
    ):
        actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual_hash != W2A_MANIFEST_SHA256[path.name]:
            raise ValueError(f"SP17 W2A manifest byte drift: {path.name}: {actual_hash}")
        values = json.loads(path.read_text(encoding="utf-8"))
        if values != sorted(set(values)) or len(values) != expected_count:
            raise ValueError(
                f"SP17 W2A manifest is not the sorted {expected_count}-ID set: {path.name}"
            )
        manifests.append(values)
    targets, focused = manifests
    if not (set(targets) | W2A_EXISTING_EXACT_GUARDS) <= set(focused):
        raise ValueError("SP17 W2A focused proof dropped a target or exact guard")
    if (
        EXPECTED_W2A_PROMOTIONS != EXPECTED_W1O_PROMOTIONS + len(targets)
        or EXPECTED_W2A_RUNNABLE != EXPECTED_W1O_RUNNABLE + len(targets)
        or EXPECTED_W2A_UNPORTED != EXPECTED_W1O_UNPORTED - len(targets)
        or EXPECTED_W2A_EXACT != EXPECTED_W1O_EXACT + len(targets)
        or EXPECTED_W2A_FAILURES != EXPECTED_W1O_FAILURES
        or EXPECTED_W2A_LIVE_OWNED != EXPECTED_W1O_LIVE_OWNED - len(targets)
    ):
        raise ValueError("SP17 W2A is not the projected 22-ID delta from W1O")
    return targets, focused


def load_w2b_w4_manifests() -> tuple[list[str], list[str]]:
    manifests = []
    for path, expected_count in (
        (W2B_W4_TARGETS_JSON, EXPECTED_W2B_W4_TARGETS),
        (W2B_W4_FOCUSED_JSON, EXPECTED_W2B_W4_FOCUSED),
    ):
        actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual_hash != W2B_W4_MANIFEST_SHA256[path.name]:
            raise ValueError(
                f"SP17 W2B-W4 manifest byte drift: {path.name}: {actual_hash}"
            )
        values = json.loads(path.read_text(encoding="utf-8"))
        if values != sorted(set(values)) or len(values) != expected_count:
            raise ValueError(
                f"SP17 W2B-W4 manifest is not the sorted "
                f"{expected_count}-ID set: {path.name}"
            )
        manifests.append(values)
    targets, focused = manifests
    if not set(targets) <= set(focused):
        raise ValueError("SP17 W2B-W4 focused proof dropped a target")
    if (
        EXPECTED_W2B_W4_PROMOTIONS != EXPECTED_W2A_PROMOTIONS + len(targets)
        or EXPECTED_W2B_W4_RUNNABLE != EXPECTED_W2A_RUNNABLE + 121
        or EXPECTED_W2B_W4_UNPORTED != EXPECTED_W2A_UNPORTED - 121
        or EXPECTED_W2B_W4_EXACT != EXPECTED_W2A_EXACT + len(targets)
        or EXPECTED_W2B_W4_FAILURES != EXPECTED_W2A_FAILURES - 11
        or EXPECTED_W2B_W4_LIVE_OWNED != EXPECTED_W2A_LIVE_OWNED - len(targets)
    ):
        raise ValueError("SP17 W2B-W4 projection is not the pinned 121/11 delta")
    return targets, focused


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


def _probe_owner_categories(
    item: dict, upstream: Path, rejection_reason: str,
) -> tuple[str, list[str]]:
    html = upstream.read_text(encoding="utf-8", errors="ignore")
    dependencies = classify_dependencies(
        html, test_id=item["test_id"], excluded={"writing_mode"}
    )
    rejection_dependency = dependency_for_portability_reason(rejection_reason)
    if rejection_dependency not in dependencies:
        dependencies.append(rejection_dependency)
    rejection_owner = CATEGORY_FOR_DEP[rejection_dependency]
    owners = sorted({CATEGORY_FOR_DEP[dependency] for dependency in dependencies})
    if rejection_owner not in owners or not (set(owners) - METADATA_CATEGORIES):
        raise ValueError(f"reasonless SP17 residual ownership: {item['test_id']}")
    return rejection_owner, owners


def build_probe_ledgers(
    inventory: list[dict], wpt_root: Path,
) -> tuple[list[str], list[dict]]:
    """Probe W0B with retained deterministic text and no repository writes."""
    validate_inventory(inventory)
    actionable = [
        item["test_id"]
        for item in inventory if item["kickoff_state"] == "runnable"
    ]
    residuals = []
    old_profile = port_wpt.ACTIVE_PORTER_PROFILE
    old_emit = port_wpt.EMIT_TEXT_NODES
    old_retain = port_wpt.RETAIN_TEXT
    try:
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        for item in inventory:
            if item["kickoff_state"] == "runnable":
                continue
            upstream = wpt_root / item["chromium_test_path"]
            if not upstream.is_file():
                raise FileNotFoundError(
                    f"{item['test_id']}: missing upstream file {upstream}"
                )
            parser = port_wpt.parse_wpt_html(str(upstream))
            portable, reason = port_wpt.analyze_portability(parser)
            if portable and not port_wpt.has_layout_content(parser):
                portable, reason = False, "no_layout_content"
            if portable:
                function_name = "probe_" + port_wpt.sanitize_fn_name(
                    item["test_id"].replace("/", "_")
                )
                generated = port_wpt.generate_rust_fn(
                    function_name, parser.root, parser.html_styles
                )
                if not generated.strip():
                    raise ValueError(f"empty SP17 builder probe: {item['test_id']}")
                actionable.append(item["test_id"])
                continue

            rejection_owner, owners = _probe_owner_categories(
                item, upstream, reason
            )
            residuals.append({
                "test_id": item["test_id"],
                "chromium_test_path": item["chromium_test_path"],
                "rejection_reason": reason,
                "rejection_owner": rejection_owner,
                "owner_categories": owners,
            })
    finally:
        port_wpt.ACTIVE_PORTER_PROFILE = old_profile
        port_wpt.EMIT_TEXT_NODES = old_emit
        port_wpt.RETAIN_TEXT = old_retain

    actionable.sort()
    residuals.sort(key=lambda item: item["test_id"])
    validate_probe_ledgers(inventory, actionable, residuals)
    return actionable, residuals


def validate_probe_ledgers(
    inventory: list[dict], actionable: list[str], residuals: list[dict],
) -> None:
    validate_inventory(inventory)
    residual_ids = [item.get("test_id", "") for item in residuals]
    if (
        actionable != sorted(set(actionable))
        or len(actionable) != EXPECTED_ACTIONABLE
    ):
        raise ValueError("SP17 W0B actionable ledger is not the sorted 311-ID set")
    if (
        residual_ids != sorted(set(residual_ids))
        or len(residuals) != EXPECTED_RESIDUALS
    ):
        raise ValueError("SP17 W0B residual ledger is not the sorted 531-row set")
    inventory_ids = {item["test_id"] for item in inventory}
    if set(actionable) & set(residual_ids):
        raise ValueError("SP17 W0B actionable and residual ledgers overlap")
    if set(actionable) | set(residual_ids) != inventory_ids:
        raise ValueError("SP17 W0B ledgers do not cover the frozen 842-row inventory")
    initial = {
        item["test_id"] for item in inventory
        if item["kickoff_state"] == "runnable"
    }
    if not initial.issubset(actionable):
        raise ValueError("SP17 W0B actionable ledger lost a kickoff runnable ID")

    required = {
        "test_id", "chromium_test_path", "rejection_reason", "rejection_owner",
        "owner_categories",
    }
    sp17_residuals = []
    for item in residuals:
        owners = item.get("owner_categories", [])
        if (
            set(item) != required
            or not item.get("chromium_test_path")
            or not item.get("rejection_reason")
            or owners != sorted(set(owners))
            or item.get("rejection_owner") not in owners
            or not (set(owners) - METADATA_CATEGORIES)
        ):
            raise ValueError(f"invalid SP17 W0B residual: {item!r}")
        if SP17_CATEGORY in owners:
            sp17_residuals.append(item)
    if (
        len(sp17_residuals) != EXPECTED_PROBE_SP17_RESIDUALS
        or {item["rejection_reason"] for item in sp17_residuals}
        != {"text_non_ascii"}
    ):
        raise ValueError("unexpected SP17-owned W0B probe residual")


def load_probe_ledgers(inventory: list[dict]) -> tuple[list[str], list[dict]]:
    for path in (ACTIONABLE_JSON, RESIDUALS_JSON):
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        expected = PROBE_LEDGER_SHA256[path.name]
        if actual != expected:
            raise ValueError(f"SP17 W0B ledger byte drift: {path.name}: {actual}")
    actionable = json.loads(ACTIONABLE_JSON.read_text(encoding="utf-8"))
    residuals = json.loads(RESIDUALS_JSON.read_text(encoding="utf-8"))
    validate_probe_ledgers(inventory, actionable, residuals)
    return actionable, residuals


def validate_live_snapshot(
    rows: list[dict[str, str]],
    summary: dict,
    baseline: list[str],
    inventory: list[dict],
    actionable: list[str],
) -> set[str]:
    """Validate the evolving live snapshot against immutable kickoff facts.

    W0A counts describe the start of SP17, not a permanent runnable ceiling.
    A kickoff-unported row may move only from the frozen actionable ledger to
    an exact, zero-error runnable result, and losing SP17 ownership is allowed
    only for such an exact runnable row.
    """
    if len(rows) != EXPECTED_MAPPING:
        raise ValueError(f"SP17 Chromium inventory changed: {len(rows)}")

    row_by_id = {canonical_id(row): row for row in rows}
    if len(row_by_id) != EXPECTED_MAPPING:
        raise ValueError("SP17 live mapping contains duplicate IDs")
    ported_ids = {
        test_id for test_id, row in row_by_id.items()
        if row.get("ported") == "yes"
    }
    unported_ids = set(row_by_id) - ported_ids

    kickoff_unported = {
        item["test_id"] for item in inventory
        if item["kickoff_state"] == "unported"
    }
    inventory_ids = {item["test_id"] for item in inventory}
    w1m_targets, _ = load_w1m_manifests()
    w1m_target_ids = set(w1m_targets)
    w1n_targets, _ = load_w1n_manifests()
    w1n_target_ids = set(w1n_targets)
    if (
        not w1n_target_ids <= set(actionable)
        or w1n_target_ids & (w1m_target_ids | W1M_EXISTING_RUNNABLE)
    ):
        raise ValueError("SP17 W1N targets changed outside the frozen actionable cohort")
    w1o_targets, _ = load_w1o_manifests()
    w1o_target_ids = set(w1o_targets)
    if (
        not w1o_target_ids <= set(actionable)
        or w1o_target_ids & (
            w1m_target_ids | W1M_EXISTING_RUNNABLE | w1n_target_ids
        )
    ):
        raise ValueError("SP17 W1O target changed outside the frozen actionable cohort")
    w2a_targets, _ = load_w2a_manifests()
    w2a_target_ids = set(w2a_targets)
    if (
        not w2a_target_ids <= set(actionable)
        or w2a_target_ids
        & (w1m_target_ids | W1M_EXISTING_RUNNABLE | w1n_target_ids | w1o_target_ids)
    ):
        raise ValueError("SP17 W2A targets changed outside the frozen actionable cohort")
    w2b_w4_targets, w2b_w4_focused = load_w2b_w4_manifests()
    w2b_w4_target_ids = set(w2b_w4_targets)
    w2b_w4_residual_admissions = w2b_w4_target_ids - set(actionable)
    if (
        len(w2b_w4_residual_admissions) != EXPECTED_W2B_W4_RESIDUAL_ADMISSIONS
        or w2b_w4_residual_admissions
        != {
            "wpt/css_flexbox/css-flexbox-test1",
            "wpt/css_flexbox/css-flexbox-test1-ref",
        }
    ):
        raise ValueError("SP17 W2B-W4 residual admission partition changed")
    if w2b_w4_target_ids & (
        w1m_target_ids
        | W1M_EXISTING_RUNNABLE
        | w1n_target_ids
        | w1o_target_ids
        | w2a_target_ids
    ):
        raise ValueError("SP17 W2B-W4 targets overlap an earlier checkpoint")
    w1m_residual_admissions = w1m_target_ids & (inventory_ids - set(actionable))
    w1m_non_sp17_admissions = w1m_target_ids - inventory_ids
    if (
        len(w1m_residual_admissions) != EXPECTED_W1M_RESIDUAL_ADMISSIONS
        or len(w1m_non_sp17_admissions) != EXPECTED_W1M_NON_SP17_ADMISSIONS
        or w1m_target_ids & set(actionable)
    ):
        raise ValueError("SP17 W1M admission partition changed")

    promoted_unported = kickoff_unported & ported_ids
    authorized_promotions = (
        set(actionable) | w1m_residual_admissions | w2b_w4_residual_admissions
    )
    if not promoted_unported.issubset(authorized_promotions):
        raise ValueError(
            "SP17 live mapping promoted a row outside the frozen promotion authorization"
        )
    if not w1m_residual_admissions.issubset(promoted_unported):
        raise ValueError("SP17 W1M residual admission is not runnable")
    admitted_non_sp17 = w1m_non_sp17_admissions & ported_ids
    if admitted_non_sp17 != w1m_non_sp17_admissions:
        raise ValueError("SP17 W1M non-SP17 admission is not runnable")
    expected_runnable = (
        EXPECTED_RUNNABLE + len(promoted_unported) + len(admitted_non_sp17)
    )
    expected_unported = (
        EXPECTED_UNPORTED - len(promoted_unported) - len(admitted_non_sp17)
    )
    if (
        expected_runnable != EXPECTED_W2B_W4_RUNNABLE
        or expected_unported != EXPECTED_W2B_W4_UNPORTED
    ):
        raise ValueError("SP17 W2B-W4 projected mapping totals changed")
    if len(ported_ids) != expected_runnable:
        raise ValueError(
            "SP17 runnable mapping changed outside exact actionable promotions: "
            f"{len(ported_ids)} != {expected_runnable}"
        )
    if len(unported_ids) != expected_unported:
        raise ValueError("SP17 unported mapping inventory changed unexpectedly")

    tests = summary.get("tests", [])
    summary_by_id = {item.get("id"): item for item in tests}
    if len(summary_by_id) != len(tests):
        raise ValueError("SP17 full pixel summary contains duplicate IDs")
    if set(summary_by_id) != ported_ids:
        raise ValueError("SP17 full pixel summary and runnable mapping IDs differ")
    repaired_kickoff_runnable = {
        item["test_id"]
        for item in inventory
        if item["kickoff_state"] == "runnable"
        and summary_by_id.get(item["test_id"], {}).get("status") == "pass"
        and summary_by_id[item["test_id"]].get("mismatch_pct") == 0.0
    }
    promoted = promoted_unported | repaired_kickoff_runnable
    if not promoted.issubset(authorized_promotions):
        raise ValueError("SP17 exact promotion is outside the W1M authorization")
    if (
        not w1n_target_ids <= promoted
        or not w1o_target_ids <= promoted
        or not w2a_target_ids <= promoted
        or not w2b_w4_target_ids <= promoted
        or len(
            promoted
            - w1n_target_ids
            - w1o_target_ids
            - w2a_target_ids
            - w2b_w4_target_ids
        )
        != EXPECTED_W1M_PROMOTIONS
        or len(promoted - w1o_target_ids - w2a_target_ids - w2b_w4_target_ids)
        != EXPECTED_W1N_PROMOTIONS
        or len(promoted - w2a_target_ids - w2b_w4_target_ids)
        != EXPECTED_W1O_PROMOTIONS
        or len(promoted - w2b_w4_target_ids) != EXPECTED_W2A_PROMOTIONS
    ):
        raise ValueError(
            "SP17 W2B-W4 did not preserve the W1M/W1N/W1O/W2A checkpoints"
        )
    if len(promoted) != EXPECTED_W2B_W4_PROMOTIONS:
        raise ValueError(
            f"SP17 W2B-W4 exact promotion count changed: {len(promoted)}"
        )
    if set(w2b_w4_focused) != promoted:
        raise ValueError("SP17 W2B-W4 focused proof is not the full promotion set")
    actual_sp13p_hash = hashlib.sha256(SP13P_TARGETS_JSON.read_bytes()).hexdigest()
    if actual_sp13p_hash != SP13P_TARGETS_SHA256:
        raise ValueError("SP17 later SP13-P target manifest byte drift")
    sp13p_later_exact = set(json.loads(SP13P_TARGETS_JSON.read_text(encoding="utf-8")))
    if (
        len(sp13p_later_exact) != EXPECTED_SP13P_LATER_EXACT
        or sp13p_later_exact & promoted
    ):
        raise ValueError("SP17 later SP13-P exact-promotion partition changed")
    for test_id in sp13p_later_exact:
        result = summary_by_id.get(test_id, {})
        if result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP17 later SP13-P promotion is not exact: {test_id}")

    passed = summary.get("passed")
    failed = summary.get("failed")
    errors = summary.get("errors")
    if (
        len(tests) != expected_runnable
        or errors != EXPECTED_ERRORS
        or passed != EXPECTED_W2B_W4_EXACT + len(sp13p_later_exact)
        or failed != EXPECTED_W2B_W4_FAILURES - len(sp13p_later_exact)
        or passed + failed + errors != expected_runnable
        or passed < EXPECTED_BASELINE + len(promoted)
    ):
        raise ValueError(
            "SP17 live pixel snapshot violates monotonic exact closure: "
            f"tests={len(tests)}, pass={passed}, fail={failed}, errors={errors}"
        )

    for test_id in baseline:
        result = summary_by_id.get(test_id, {})
        if result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP17 frozen baseline regressed: {test_id}")
    for test_id in promoted:
        result = summary_by_id[test_id]
        if result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP17 promotion is not exact: {test_id}")
        if SP17_CATEGORY in categories(row_by_id[test_id]["failure_category"]):
            raise ValueError(f"SP17 exact promotion retained live ownership: {test_id}")

    for test_id in w1m_target_ids | W1M_EXISTING_RUNNABLE:
        result = summary_by_id.get(test_id, {})
        if result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP17 W1M cohort is not exact: {test_id}")

    for test_id in w1n_target_ids:
        result = summary_by_id.get(test_id, {})
        if result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP17 W1N target is not exact: {test_id}")

    for test_id in w1o_target_ids:
        result = summary_by_id.get(test_id, {})
        if result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP17 W1O target is not exact: {test_id}")

    for test_id in w2a_target_ids:
        result = summary_by_id.get(test_id, {})
        if result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP17 W2A target is not exact: {test_id}")

    for test_id in w2b_w4_target_ids:
        result = summary_by_id.get(test_id, {})
        if result.get("status") != "pass" or result.get("mismatch_pct") != 0.0:
            raise ValueError(f"SP17 W2B-W4 target is not exact: {test_id}")

    live_owned = {
        test_id for test_id, row in row_by_id.items()
        if SP17_CATEGORY in categories(row["failure_category"])
    }
    if not live_owned.issubset(inventory_ids):
        raise ValueError("SP17 ownership expanded outside the frozen inventory")
    if len(live_owned) != EXPECTED_W2B_W4_LIVE_OWNED:
        raise ValueError(f"SP17 W2B-W4 live ownership changed: {len(live_owned)}")
    for test_id in inventory_ids - live_owned:
        result = summary_by_id.get(test_id, {})
        if (
            test_id not in ported_ids
            or result.get("status") != "pass"
            or result.get("mismatch_pct") != 0.0
        ):
            raise ValueError(
                f"SP17 ownership disappeared without an exact runnable result: {test_id}"
            )
    return promoted


def encoded(value: object) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def load_inputs() -> tuple[list[dict[str, str]], dict]:
    rows = list(csv.DictReader(MAPPING_CSV.open(newline="", encoding="utf-8")))
    summary = json.loads(SUMMARY_JSON.read_text(encoding="utf-8"))
    return rows, summary


def build_outputs(rows: list[dict[str, str]], summary: dict) -> dict[Path, str]:
    baseline, inventory, targets, evidence = load_ledgers()
    actionable, _ = load_probe_ledgers(inventory)
    validate_live_snapshot(rows, summary, baseline, inventory, actionable)
    return {
        BASELINE_JSON: encoded(baseline),
        INVENTORY_JSON: encoded(inventory),
        INITIAL_TARGETS_JSON: encoded(targets),
        INITIAL_RESULTS_JSON: encoded(evidence),
    }


def load_ledgers() -> tuple[list, list, list, dict]:
    for path in (
        BASELINE_JSON, INVENTORY_JSON, INITIAL_TARGETS_JSON, INITIAL_RESULTS_JSON,
    ):
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        expected = KICKOFF_LEDGER_SHA256[path.name]
        if actual != expected:
            raise ValueError(f"SP17 kickoff ledger byte drift: {path.name}: {actual}")
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
    usage = (
        "Usage: generate_sp17_closure.py "
        "[--check|--capture-initial-results|--probe]"
    )
    if sys.argv[1:] not in (
        [], ["--check"], ["--capture-initial-results"], ["--probe"]
    ):
        print(usage, file=sys.stderr)
        return 2

    validate_historical_ledgers()
    rows, summary = load_inputs()
    if sys.argv[1:] == ["--probe"]:
        _, inventory, _, _ = load_ledgers()
        actionable, residuals = build_probe_ledgers(inventory, WPT_ROOT)
        ACTIONABLE_JSON.write_text(encoded(actionable), encoding="utf-8")
        RESIDUALS_JSON.write_text(encoded(residuals), encoding="utf-8")
    elif sys.argv[1:] == ["--capture-initial-results"]:
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
            _, inventory, _, _ = load_ledgers()
            if ACTIONABLE_JSON.is_file() or RESIDUALS_JSON.is_file():
                if not ACTIONABLE_JSON.is_file() or not RESIDUALS_JSON.is_file():
                    raise ValueError("partial SP17 W0B probe ledger state")
                load_probe_ledgers(inventory)
        else:
            for path, content in outputs.items():
                path.write_text(content, encoding="utf-8")
    if ACTIONABLE_JSON.is_file() and RESIDUALS_JSON.is_file():
        print(
            "SP17 ledgers: baseline=3267, inventory=842, initial=19, "
            "actionable=311, residuals=531"
        )
    else:
        print("SP17 W0A ledgers: baseline=3267, inventory=842, initial=19")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
