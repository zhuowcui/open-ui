#!/usr/bin/env python3
"""Version exact native final-state additions after a complete matrix run."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path

import residuals


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "docs/renderer/generated/qualification-contract-v2.json"
ORIGINAL = ROOT / "tools/qualification/manifests/complete-5731.json"
PRIOR = ROOT / "tools/qualification/manifests/expanded-v18.json"
AST_AUDIT = ROOT / "docs/renderer/generated/javascript-mutation-audit-v3.json"
LEDGER = ROOT / "docs/renderer/generated/expanded-requalification-v1.json"
MANIFEST = ROOT / "tools/qualification/manifests/expanded-v19.json"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def encoded(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def requalify(
    report_path: Path,
    ledger_output: Path = LEDGER,
    manifest_output: Path = MANIFEST,
) -> tuple[dict, dict]:
    contract = json.loads(CONTRACT.read_text())
    original = json.loads(ORIGINAL.read_text())
    prior = json.loads(PRIOR.read_text())
    ast = json.loads(AST_AUDIT.read_text())
    additions = prior["additions"]
    if len(original) != 5731 or original != sorted(set(original)):
        raise ValueError("immutable original manifest changed")
    if len(additions) != 201 or additions != sorted(set(additions)):
        raise ValueError("current 201-case admission changed")
    if prior["base_manifest_sha256"] != sha256(ORIGINAL):
        raise ValueError("prior admission refers to a different original manifest")
    if prior["candidate_audit_sha256"] != sha256(AST_AUDIT):
        raise ValueError("prior candidate audit changed")
    prior_exact = {
        entry["test_id"] for entry in ast["entries"]
        if entry["disposition"] == "lowered-exact"
    }
    if prior_exact != set(additions):
        raise ValueError("prior AST admission inventory differs")
    ids = sorted(set(original) | set(additions))
    report = json.loads(report_path.read_text())
    if report["suite"] != "expanded" or not report["complete_contract_scope"]:
        raise ValueError("complete expanded-v18 matrix report required")
    if not report["source"]["clean"] or report["evidence"]["tolerance_pixels"] != 0:
        raise ValueError("expanded requalification requires clean exact diagnostics")
    if report["commit"] != report["source"]["commit"]:
        raise ValueError("source commit identity differs")
    if report["contract_sha256"] != sha256(CONTRACT):
        raise ValueError("qualification contract differs")
    if report["id_manifest"]["sha256"] != sha256(PRIOR):
        raise ValueError("expanded-v18 manifest identity differs")
    if report["chromium"]["build_identity"] != contract["chromium"]["raster_oracle_build_identity"]:
        raise ValueError("Chromium oracle build differs")
    if report["openui"]["raster_backend_identity"]["backend"] != contract["raster"]["qualification_backend"]:
        raise ValueError("qualification raster backend differs")
    if report["results"]["total"] != len(ids) * 4 or report["results"]["errors"]:
        raise ValueError("missing expanded comparisons or render errors")
    expected_profiles = [entry["name"] for entry in contract["qualification_profiles"]]
    if [profile["profile"] for profile in report["profiles"]] != expected_profiles:
        raise ValueError("not the four required profiles")
    observed = {test_id: [] for test_id in additions}
    for profile in report["profiles"]:
        rows = profile["tests"]
        if [row["id"] for row in rows] != ids:
            raise ValueError("expanded IDs are missing, duplicated, or reordered")
        if profile["result_sha256"] != residuals.canonical_sha256(rows):
            raise ValueError("expanded profile results changed")
        for row in rows:
            if row["id"] not in observed:
                continue
            if row["status"] not in {"exact", "different"}:
                raise ValueError("admitted candidate has a render error")
            signature = row.get("diff_signature") or {}
            if row["status"] == "different" and (
                not signature.get("comparable")
                or signature.get("mismatched_pixels") != row.get("mismatched_pixels")
            ):
                raise ValueError("admitted candidate lacks exact pixel evidence")
            observed[row["id"]].append({
                "profile": profile["profile"],
                "status": row["status"],
                "mismatched_pixels": row["mismatched_pixels"],
                "mismatch_bounds": signature.get("mismatch_bounds"),
                "connected_region_count": signature.get("connected_region_count"),
                "channel_delta_extrema": {
                    channel: stats["maximum_absolute_delta"]
                    for channel, stats in signature.get("channel_deltas", {}).items()
                },
                "diff_signature_sha256": residuals.canonical_sha256(signature),
            })
    retained = [test_id for test_id in additions if all(
        row["status"] == "exact" for row in observed[test_id]
    )]
    demoted = [test_id for test_id in additions if test_id not in retained]
    ledger = {
        "schema_version": 1,
        "status": "diagnostic-requalification; original contract remains blocked",
        "source": report["source"],
        "chromium": {key: report["chromium"][key] for key in
                     ("build_identity", "binary_sha256", "capture_harness_sha256")},
        "openui": report["openui"],
        "contract_sha256": report["contract_sha256"],
        "original_manifest_sha256": sha256(ORIGINAL),
        "prior_expanded_manifest_sha256": sha256(PRIOR),
        "ast_audit_sha256": sha256(AST_AUDIT),
        "matrix_report_sha256": sha256(report_path),
        "prior_admitted_count": len(additions),
        "retained_four_profile_exact_count": len(retained),
        "demoted_count": len(demoted),
        "demoted": [
            {"test_id": test_id, "comparisons": observed[test_id]}
            for test_id in demoted
        ],
    }
    ledger["output_sha256"] = residuals.canonical_sha256(ledger)
    manifest = {
        "schema_version": 1,
        "base_manifest": "complete-5731.json",
        "base_manifest_sha256": sha256(ORIGINAL),
        "candidate_audit": prior["candidate_audit"],
        "candidate_audit_sha256": sha256(AST_AUDIT),
        "prior_manifest": PRIOR.name,
        "prior_manifest_sha256": sha256(PRIOR),
        "qualification_evidence": Path(os.path.relpath(
            ledger_output.resolve(), manifest_output.parent.resolve()
        )).as_posix(),
        "qualification_evidence_sha256": hashlib.sha256(encoded(ledger)).hexdigest(),
        "policy": "Only additions exact at all four profiles in the requalification report are retained; this selection remains diagnostic until a new contract is admitted.",
        "additions": retained,
        "demoted_ids": demoted,
    }
    return ledger, manifest


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix-report", type=Path, required=True)
    parser.add_argument("--ledger-output", type=Path, default=LEDGER)
    parser.add_argument("--manifest-output", type=Path, default=MANIFEST)
    args = parser.parse_args()
    ledger, manifest = requalify(
        args.matrix_report, args.ledger_output, args.manifest_output
    )
    for path, value in ((args.ledger_output, ledger), (args.manifest_output, manifest)):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(encoded(value))
    print(f"retained={ledger['retained_four_profile_exact_count']} "
          f"demoted={ledger['demoted_count']}")


if __name__ == "__main__":
    main()
