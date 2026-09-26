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
SP19_LAYOUT_TARGETS_JSON = SCRIPT_DIR / "sp19_layout_targets.json"
SP20_TARGETS_JSON = SCRIPT_DIR / "sp20_targets.json"
MUTATION_AUDIT_JSON = PROJECT_ROOT / "docs/renderer/generated/javascript-mutation-audit-v2.json"

OWNER = "sp13_multicol"
FALLBACK_CATEGORIES = {"sp12_layout_bug", "not_ported"}
EXPECTED_INVENTORY = 1369
EXPECTED_BASELINE = 2823
EXPECTED_TARGETS = 351
EXPECTED_RESIDUALS = 1018
EXPECTED_RUNNABLE = 3566
EXPECTED_UNPORTED = 4107
EXPECTED_EXACT = EXPECTED_BASELINE + EXPECTED_TARGETS
LATER_EXACT_PROMOTIONS = frozenset({
    "wpt/css_break/borders-006",
    "wpt/css_break/borders-007",
    "wpt/css_break/flexbox_multi-line-row-flex-fragmentation-056",
    "wpt/css_break/flexbox_single-line-column-flex-fragmentation-044",
    "wpt/css_break/flexbox_single-line-row-flex-fragmentation-030",
    "wpt/css_break/out-of-flow-in-multicolumn-063",
    "wpt/css_break/out-of-flow-in-multicolumn-064",
    "wpt/css_break/out-of-flow-in-multicolumn-066",
    "wpt/css_break/out-of-flow-in-multicolumn-067",
    "wpt/css_break/out-of-flow-in-multicolumn-093",
    "wpt/css_break/out-of-flow-in-multicolumn-118",
    "wpt/css_break/out-of-flow-in-multicolumn-119",
    "wpt/css_break/overflow-clip-000",
    "wpt/css_break/overflow-clip-001",
    "wpt/css_break/overflow-clip-002",
    "wpt/css_multicol/multicol-fill-balance-004",
    "wpt/css_multicol/multicol-dynamic-add-004-ref",
    "wpt/css_multicol/multicol-span-auto-size-in-vertical-writing-mode-001",
    "wpt/css_multicol/multicol-span-auto-size-in-vertical-writing-mode-002",
    "wpt/css_multicol/multicol-under-vertical-rl-scroll",
    "wpt/css_multicol/orthogonal-writing-mode-shrink-to-fit",
    "wpt/css_multicol/orthogonal-writing-mode-spanner",
    "wpt/css_overflow/no-scrollable-overflow-vertical-rl",
    "wpt/css_overflow/no-scrollable-overflow-vertical-rl-2",
    "wpt/css_overflow/line-clamp_webkit-line-clamp-041-crash",
    "wpt/css_overflow/line-clamp_webkit-line-clamp-042-crash",
    "wpt/css_overflow/line-clamp_webkit-line-clamp-043",
    "wpt/css_overflow/scroll-markers_column-scroll-marker-006-ref",
    "wpt/css_overflow/scroll-markers_column-scroll-marker-007-ref",
    "wpt/css_break/background-image-000",
    "wpt/css_break/background-image-001",
    "wpt/css_break/background-image-002",
    "wpt/css_break/box-shadow-003",
    "wpt/css_break/box-shadow-004",
    "wpt/css_break/break-inside-avoid-min-block-size-2",
    "wpt/css_break/break-inside-avoid-min-block-size-2-ref",
    "wpt/css_break/flexbox_multi-line-column-flex-fragmentation-046",
    "wpt/css_break/flexbox_multi-line-column-flex-fragmentation-047",
    "wpt/css_break/flexbox_multi-line-row-flex-fragmentation-057",
    "wpt/css_break/flexbox_single-line-column-flex-fragmentation-045",
    "wpt/css_break/flexbox_single-line-row-flex-fragmentation-031",
    "wpt/css_break/rounded-clipped-border",
    "wpt/css_break/table_table-col-paint-vlr-rtl-ref",
    "wpt/css_break/table_table-col-paint-vrl-rtl-ref",
    "wpt/css_break/table_table-collapsed-borders-paint-vlr-rtl-ref",
    "wpt/css_break/table_table-collapsed-borders-paint-vrl-ltr-ref",
    "wpt/css_break/table_table-grid-paint-vlr-rtl-ref",
    "wpt/css_break/table_table-grid-paint-vrl-rtl-ref",
    "wpt/css_break/table_table-row-paint-vlr-rtl-ref",
    "wpt/css_break/table_table-row-paint-vrl-rtl-ref",
    "wpt/css_break/table_table-section-paint-vlr-rtl-ref",
    "wpt/css_break/table_table-section-paint-vrl-rtl-ref",
    "wpt/css_flexbox/flexbox-column-row-gap-002",
    "wpt/css_flexbox/flexbox-column-row-gap-004",
    "wpt/css_flexbox/flexbox-column-row-gap-004-ref",
    "wpt/css_multicol/column-fill-balance-orthog-block-001",
    "wpt/css_multicol/crashtests_vertical-rl-column-rules-wide-columns",
    "wpt/css_multicol/multicol-span-all-008",
    "wpt/css_multicol/multicol-span-all-011",
    "wpt/css_multicol/multicol-span-all-rule-002",
    "wpt/css_position/multicol_static-position_vlr-in-multicol-ref",
    "wpt/css_position/multicol_static-position_vlr-ltr-ltr-in-multicol",
    "wpt/css_position/multicol_static-position_vlr-ltr-rtl-in-multicol.tentative",
    "wpt/css_position/multicol_static-position_vlr-rtl-ltr-in-multicol.tentative",
    "wpt/css_position/multicol_static-position_vlr-rtl-rtl-in-multicol",
    "wpt/css_position/multicol_static-position_vrl-in-multicol-ref",
    "wpt/css_position/multicol_static-position_vrl-ltr-ltr-in-multicol",
    "wpt/css_position/multicol_static-position_vrl-ltr-rtl-in-multicol.tentative",
    "wpt/css_position/multicol_static-position_vrl-rtl-ltr-in-multicol.tentative",
    "wpt/css_position/multicol_static-position_vrl-rtl-rtl-in-multicol",
    "wpt/css_position/multicol_vlr-in-multicols-ref",
    "wpt/css_position/multicol_vlr-ltr-ltr-in-multicols",
    "wpt/css_position/multicol_vlr-ltr-rtl-in-multicols.tentative",
    "wpt/css_position/multicol_vlr-rtl-ltr-in-multicols.tentative",
    "wpt/css_position/multicol_vlr-rtl-rtl-in-multicols",
    "wpt/css_position/multicol_vrl-in-multicols-ref",
    "wpt/css_position/multicol_vrl-ltr-ltr-in-multicols",
    "wpt/css_position/multicol_vrl-ltr-rtl-in-multicols.tentative",
    "wpt/css_position/multicol_vrl-rtl-ltr-in-multicols.tentative",
    "wpt/css_position/multicol_vrl-rtl-rtl-in-multicols",
})


def sp19_live_promotions(mapping: dict[str, dict[str, str]]) -> set[str]:
    """Return later closure targets installed by the current completed wave.

    The historical function name remains part of the accountability audit's
    public contract. SP20 can legitimately promote rows frozen as SP13-R
    residuals, so include its pinned target manifest as another source of
    live, exact promotions.
    """
    targets = set()
    for path in (SP19_LAYOUT_TARGETS_JSON, SP20_TARGETS_JSON):
        if path.is_file():
            targets.update(json.loads(path.read_text(encoding="utf-8")))
    return {
        test_id
        for test_id in targets
        if mapping.get(test_id, {}).get("ported") == "yes"
    }


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


def lowered_candidate_promotions(
    mapping: dict[str, dict[str, str]], *, require_complete: bool = True
) -> set[str]:
    """Identify later AST-lowered ports outside the immutable 5,731-case result set."""
    audit = json.loads(MUTATION_AUDIT_JSON.read_text(encoding="utf-8"))
    if audit.get("schema_version") != 2 or len(audit.get("entries", [])) != 393:
        raise ValueError("JavaScript mutation audit inventory changed")
    entries = audit["entries"]
    if len({entry["test_id"] for entry in entries}) != len(entries):
        raise ValueError("JavaScript mutation audit has duplicate identities")
    promotions = {
        entry["test_id"] for entry in entries
        if entry["disposition"] in {"lowered-exact", "ast-lowered-pending-exact"}
        and isinstance(entry.get("mutation_ir"), dict)
        and entry["mutation_ir"].get("lowerable") is True
    }
    if not require_complete:
        promotions &= mapping.keys()
    for test_id in promotions:
        row = mapping.get(test_id)
        if not row or row.get("ported") != "yes" or row.get("our_test_id") != test_id:
            raise ValueError(f"AST-lowered candidate is not a live port: {test_id}")
    return promotions


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
    live_promotions = set(LATER_EXACT_PROMOTIONS) | sp19_live_promotions(mapping)
    candidate_promotions = lowered_candidate_promotions(mapping)
    if len(rows) != 7673:
        raise ValueError(f"SP13-R mapping inventory changed: {len(rows)}")
    if len(mapping) != len(rows):
        raise ValueError("SP13-R mapping contains duplicate identities")
    ported_ids = {
        test_id for test_id, row in mapping.items() if row.get("ported") == "yes"
    }
    unported = len(rows) - len(ported_ids)
    if (
        len(ported_ids) < EXPECTED_RUNNABLE
        or set(summary_by_id) != ported_ids - candidate_promotions
    ):
        raise ValueError(
            "SP13-R live runnable identity changed incompatibly: "
            f"mapping={len(ported_ids)}, summary={len(summary_by_id)}, "
            f"floor={EXPECTED_RUNNABLE}"
        )
    if unported > EXPECTED_UNPORTED:
        raise ValueError(
            f"SP13-R unported inventory grew: {unported} > {EXPECTED_UNPORTED}"
        )
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
        if test_id in candidate_promotions:
            if OWNER in categories(row.get("failure_category", "")):
                raise ValueError(f"AST-lowered candidate retains multicol ownership: {test_id}")
            continue
        if test_id in live_promotions:
            result = summary_by_id.get(test_id)
            if (
                not row
                or row.get("ported") != "yes"
                or row.get("our_test_id") != test_id
                or OWNER in categories(row.get("failure_category", ""))
                or not result
                or result.get("status") != "pass"
                or result.get("mismatch_pct") != 0.0
            ):
                raise ValueError(
                    f"SP13-R later promotion is not runnable and exact: {test_id}"
                )
            continue
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
