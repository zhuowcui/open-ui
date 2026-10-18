#!/usr/bin/env python3
"""Index complete focused and primitive raster matrices without relaxing either gate."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import residuals


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "docs/renderer/generated/qualification-contract-v2.json"
OWNERSHIP = ROOT / "tools/qualification/residual-ownership-v2.json"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def checked_report(path: Path, suite: str, contract: dict) -> dict:
    report = json.loads(path.read_text())
    manifest = contract["suite_manifests"][suite]
    manifest_path = ROOT / manifest["path"]
    manifest_ids = json.loads(manifest_path.read_text())
    if sha256(manifest_path) != manifest["sha256"]:
        raise ValueError(f"checked-in {suite} manifest changed")
    expected_profiles = [
        f"focused-{int(entry['logical_size_css_px']['width'])}x"
        f"{int(entry['logical_size_css_px']['height'])}@"
        f"{float(entry['device_scale']):g}"
        for entry in contract["focused_cross_product"]
    ]
    if report["suite"] != suite or not report["complete_contract_scope"]:
        raise ValueError(f"incomplete {suite} report: {path}")
    if report["contract_sha256"] != sha256(CONTRACT):
        raise ValueError(f"contract changed in {suite} report")
    if report["id_manifest"]["sha256"] != manifest["sha256"]:
        raise ValueError(f"manifest changed in {suite} report")
    if report["id_manifest"]["count"] != manifest["count"]:
        raise ValueError(f"wrong {suite} manifest count")
    if report["commit"] != report["source"]["commit"]:
        raise ValueError(f"inconsistent {suite} source commit")
    if not report["source"]["clean"] or report["evidence"]["tolerance_pixels"] != 0:
        raise ValueError(f"unclean or nonexact {suite} report")
    if report["openui"]["raster_backend_identity"]["backend"] != contract["raster"]["qualification_backend"]:
        raise ValueError(f"wrong {suite} raster backend")
    if [entry["profile"] for entry in report["profiles"]] != expected_profiles:
        raise ValueError(f"incomplete or reordered {suite} profile set")
    if report["results"]["total"] != manifest["count"] * len(expected_profiles):
        raise ValueError(f"wrong {suite} comparison count")
    for profile in report["profiles"]:
        tests = profile["tests"]
        if len(tests) != manifest["count"]:
            raise ValueError(f"wrong {suite} per-profile test count")
        if [row["id"] for row in tests] != manifest_ids:
            raise ValueError(f"missing, duplicate, or reordered {suite} IDs")
        if profile["ordered_id_sha256"] != hashlib.sha256(
            "\n".join(manifest_ids).encode()
        ).hexdigest():
            raise ValueError(f"changed {suite} ordered ID hash")
        if profile["result_sha256"] != residuals.canonical_sha256(tests):
            raise ValueError(f"changed {suite} profile results")
        for status, field in (("exact", "exact"), ("different", "different"), ("error", "errors")):
            if profile[field] != sum(row["status"] == status for row in tests):
                raise ValueError(f"wrong {suite} {field} count")
    for field in ("exact", "different", "errors", "total"):
        if report["results"][field] != sum(profile[field] for profile in report["profiles"]):
            raise ValueError(f"wrong {suite} total {field}")
    if report["results"]["profile_result_sha256"] != residuals.canonical_sha256(
        [profile["result_sha256"] for profile in report["profiles"]]
    ):
        raise ValueError(f"changed {suite} aggregate results")
    return report


def summarize(focused_path: Path, primitive_path: Path) -> dict:
    contract = json.loads(CONTRACT.read_text())
    focused = checked_report(focused_path, "focused", contract)
    primitive = checked_report(primitive_path, "primitive", contract)
    if focused["results"]["different"] or focused["results"]["errors"]:
        raise ValueError("focused matrix is not exact; investigate its residuals separately")
    if primitive["results"]["errors"]:
        raise ValueError("primitive matrix contains render errors")
    for key in ("source", "chromium", "openui", "contract_sha256", "font_byte_hashes", "resource_hashes"):
        if focused[key] != primitive[key]:
            raise ValueError(f"focused and primitive {key} identities differ")
    ownership, ownership_sha = residuals.load_ownership(OWNERSHIP)
    differing: dict[str, list[dict]] = {}
    for profile in primitive["profiles"]:
        for row in profile["tests"]:
            if row["status"] == "exact":
                continue
            signature = row.get("diff_signature") or {}
            if row["status"] != "different" or not signature.get("comparable"):
                raise ValueError("primitive report has missing or incomparable pixel evidence")
            if signature.get("mismatched_pixels") != row.get("mismatched_pixels"):
                raise ValueError("primitive pixel count disagrees with its signature")
            differing.setdefault(row["id"], []).append({
                "profile": profile["profile"],
                "mismatched_pixels": row["mismatched_pixels"],
                "mismatch_bounds": signature["mismatch_bounds"],
                "connected_region_count": signature["connected_region_count"],
                "maximum_absolute_channel_delta": {
                    channel: stats["maximum_absolute_delta"]
                    for channel, stats in signature["channel_deltas"].items()
                },
                "diff_signature_sha256": residuals.canonical_sha256(signature),
            })
    entries = []
    for test_id, comparisons in sorted(differing.items()):
        review = ownership.get(test_id)
        entries.append({
            "test_id": test_id,
            "different_profiles": len(comparisons),
            "comparisons": comparisons,
            "root_cause": review["root_cause"] if review else None,
            "owning_subsystem": review["owning_subsystem"] if review else None,
            "minimized_reproducer": review["minimized_reproducer"] if review else None,
        })
    result = {
        "schema_version": 1,
        "status": "diagnostic-open" if primitive["results"]["different"] else "exact",
        "source": focused["source"],
        "chromium": {key: focused["chromium"][key] for key in
                     ("build_identity", "binary_sha256", "capture_harness_sha256")},
        "openui": focused["openui"],
        "contract_sha256": focused["contract_sha256"],
        "ownership_input_sha256": ownership_sha,
        "input_reports": {
            "focused": sha256(focused_path),
            "primitive": sha256(primitive_path),
        },
        "focused": focused["results"],
        "primitive": primitive["results"],
        "unowned_residual_test_count": sum(entry["root_cause"] is None for entry in entries),
        "entries": entries,
    }
    result["output_sha256"] = residuals.canonical_sha256(result)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--focused-report", type=Path, required=True)
    parser.add_argument("--primitive-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    summary = summarize(args.focused_report, args.primitive_report)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(f"focused={summary['focused']['exact']}/{summary['focused']['total']} "
          f"primitive={summary['primitive']['exact']}/{summary['primitive']['total']} "
          f"unowned={summary['unowned_residual_test_count']}")


if __name__ == "__main__":
    main()
