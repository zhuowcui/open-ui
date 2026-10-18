#!/usr/bin/env python3
"""Find conflicting decoded Chromium captures under the same oracle identity.

This compares recorded matrix evidence. Agreement proves consistency only for
the observed captures; it does not qualify a renderer or an unobserved case.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import residuals


def audit(paths: list[Path]) -> dict:
    observations: dict[str, dict] = {}
    inputs = []
    fresh_count = 0
    for path in paths:
        raw = path.read_bytes()
        report = json.loads(raw)
        if report.get("schema_version") != 2:
            raise ValueError(f"not a schema-v2 matrix report: {path}")
        if report["evidence"]["tolerance_pixels"] != 0:
            raise ValueError(f"nonexact comparison report: {path}")
        inputs.append({
            "path": str(path),
            "sha256": hashlib.sha256(raw).hexdigest(),
            "source": report["source"],
            "complete_contract_scope": report["complete_contract_scope"],
        })
        environment = {
            key: report[key] for key in
            ("contract_sha256", "font_byte_hashes", "resource_hashes")
        }
        environment["chromium"] = {
            key: report["chromium"][key] for key in
            ("build_identity", "binary_sha256", "capture_harness_sha256")
        }
        for profile in report["profiles"]:
            rows = profile["tests"]
            if profile["result_sha256"] != residuals.canonical_sha256(rows):
                raise ValueError(f"changed profile results: {path}")
            for row in rows:
                if row["status"] == "error":
                    raise ValueError(f"missing oracle capture: {path}: {row['id']}")
                identity = row["chromium_oracle_identity_sha256"]
                rgba = row["chromium_rgba_sha256"]
                if rgba != row["chromium_oracle_rgba_sha256"]:
                    raise ValueError(f"inconsistent oracle digest: {path}: {row['id']}")
                capture_source = row["chromium_oracle_source"]
                if capture_source not in {"capture", "cache"}:
                    raise ValueError(f"unknown oracle capture source: {path}")
                fresh_count += capture_source == "capture"
                item = observations.setdefault(identity, {
                    "test_id": row["id"],
                    "profile": profile["profile"],
                    "environment": environment,
                    "variants": {},
                })
                if (item["test_id"] != row["id"]
                        or item["profile"] != profile["profile"]
                        or item["environment"] != environment):
                    raise ValueError(f"oracle identity aliases different inputs: {identity}")
                item["variants"].setdefault(rgba, []).append({
                    "report": str(path),
                    "source": capture_source,
                    "png_sha256": row["chromium_png_sha256"],
                })
    conflicts = []
    for identity, item in sorted(observations.items()):
        if len(item["variants"]) <= 1:
            continue
        conflicts.append({
            "oracle_identity_sha256": identity,
            "test_id": item["test_id"],
            "profile": item["profile"],
            "environment": item["environment"],
            "variants": [
                {"rgba_sha256": rgba, "observations": entries}
                for rgba, entries in sorted(item["variants"].items())
            ],
        })
    result = {
        "schema_version": 1,
        "status": "contradicted-oracle-identity" if conflicts else "consistent-observations",
        "renderer_qualification": False,
        "input_reports": inputs,
        "observed_identity_count": len(observations),
        "fresh_capture_observation_count": fresh_count,
        "identity_count_with_fresh_capture": sum(
            any(entry["source"] == "capture" for entries in item["variants"].values()
                for entry in entries) for item in observations.values()
        ),
        "contradicted_identity_count": len(conflicts),
        "conflicts": conflicts,
    }
    result["output_sha256"] = residuals.canonical_sha256(result)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reports", nargs="+", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    result = audit(args.reports)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(f"identities={result['observed_identity_count']} "
          f"fresh={result['fresh_capture_observation_count']} "
          f"conflicts={result['contradicted_identity_count']}")
    if result["conflicts"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
