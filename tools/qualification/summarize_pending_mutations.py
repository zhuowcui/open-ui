#!/usr/bin/env python3
"""Disposition the AST-lowered candidates using all four exact pixel profiles."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import residuals


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "docs/renderer/generated/qualification-contract-v2.json"
AST_AUDIT = ROOT / "docs/renderer/generated/javascript-mutation-audit-v2.json"
ORIGINAL_MANIFEST = ROOT / "tools/qualification/manifests/complete-5731.json"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def summarize(report_path: Path) -> dict:
    contract = json.loads(CONTRACT.read_text())
    ast = json.loads(AST_AUDIT.read_text())
    pending = {
        entry["test_id"]: entry for entry in ast["entries"]
        if entry["disposition"] == "ast-lowered-pending-exact"
    }
    ids = sorted(pending)
    if len(ids) != 36 or len(pending) != ast["counts"]["ast-lowered-pending-exact"]:
        raise ValueError("AST candidate inventory changed")
    report = json.loads(report_path.read_text())
    if report["suite"] != "full" or report["complete_contract_scope"]:
        raise ValueError("expected a partial, four-profile full-suite diagnostic")
    if not report["source"]["clean"] or report["evidence"]["tolerance_pixels"] != 0:
        raise ValueError("pending candidates require clean-source exact diagnostics")
    if report["commit"] != report["source"]["commit"]:
        raise ValueError("source commit identity differs")
    if report["contract_sha256"] != sha256(CONTRACT):
        raise ValueError("qualification contract differs")
    if report["chromium"]["build_identity"] != contract["chromium"]["raster_oracle_build_identity"]:
        raise ValueError("Chromium oracle build differs")
    if report["openui"]["raster_backend_identity"]["backend"] != contract["raster"]["qualification_backend"]:
        raise ValueError("qualification raster backend differs")
    if report["results"]["total"] != len(ids) * 4 or report["results"]["errors"]:
        raise ValueError("missing comparisons or render errors")
    expected_profiles = [entry["name"] for entry in contract["qualification_profiles"]]
    if [entry["profile"] for entry in report["profiles"]] != expected_profiles:
        raise ValueError("not the four required profiles")
    observed: dict[str, list[dict]] = {test_id: [] for test_id in ids}
    for profile in report["profiles"]:
        tests = profile["tests"]
        if [row["id"] for row in tests] != ids:
            raise ValueError("pending IDs are missing, duplicated, or reordered")
        if profile["result_sha256"] != residuals.canonical_sha256(tests):
            raise ValueError("pending profile results changed")
        for row in tests:
            status = row["status"]
            if status not in {"exact", "different"}:
                raise ValueError("pending candidate has a render error")
            signature = row.get("diff_signature") or {}
            if status == "different" and (
                not signature.get("comparable")
                or signature.get("mismatched_pixels") != row.get("mismatched_pixels")
            ):
                raise ValueError("pending difference lacks exact pixel evidence")
            observed[row["id"]].append({
                "profile": profile["profile"],
                "status": status,
                "mismatched_pixels": row["mismatched_pixels"],
                "mismatch_bounds": signature.get("mismatch_bounds"),
                "connected_region_count": signature.get("connected_region_count"),
                "maximum_absolute_channel_delta": {
                    channel: stats["maximum_absolute_delta"]
                    for channel, stats in signature.get("channel_deltas", {}).items()
                },
                "diff_signature_sha256": residuals.canonical_sha256(signature),
            })
    entries = []
    for test_id in ids:
        comparisons = observed[test_id]
        eligible = all(row["status"] == "exact" for row in comparisons)
        entries.append({
            "test_id": test_id,
            "ast_disposition": pending[test_id]["disposition"],
            "mutation_ir_sha256": residuals.canonical_sha256(pending[test_id]["mutation_ir"]),
            "four_profile_exact": eligible,
            "expanded_manifest_admission": "eligible" if eligible else "pending-exact",
            "comparisons": comparisons,
        })
    result = {
        "schema_version": 1,
        "status": "diagnostic-pending-exact",
        "source": report["source"],
        "chromium": {key: report["chromium"][key] for key in
                     ("build_identity", "binary_sha256", "capture_harness_sha256")},
        "openui": report["openui"],
        "contract_sha256": report["contract_sha256"],
        "ast_audit_sha256": sha256(AST_AUDIT),
        "original_manifest_sha256": sha256(ORIGINAL_MANIFEST),
        "matrix_report_sha256": sha256(report_path),
        "candidate_count": len(ids),
        "comparison_counts": report["results"],
        "four_profile_exact_candidate_count": sum(entry["four_profile_exact"] for entry in entries),
        "entries": entries,
    }
    result["output_sha256"] = residuals.canonical_sha256(result)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = summarize(args.matrix_report)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(f"{result['four_profile_exact_candidate_count']}/"
          f"{result['candidate_count']} candidates exact in all four profiles")


if __name__ == "__main__":
    main()
