#!/usr/bin/env python3
"""Verify and summarize a sharded four-profile renderer census.

This is an evidence index, not a qualification shortcut. In particular,
unreviewed residual ownership is an error unless the caller explicitly asks
for a nonqualifying diagnostic snapshot.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import residuals


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "docs/renderer/generated/qualification-contract-v2.json"
MANIFEST = ROOT / "tools/qualification/manifests/complete-5731.json"
OWNERSHIP = ROOT / "tools/qualification/residual-ownership-v2.json"
OUTPUT = ROOT / "docs/renderer/generated/four-profile-census-v1.json"
SHARED_KEYS = (
    "commit", "contract_sha256", "source", "chromium", "openui",
    "resource_hashes", "font_byte_hashes", "raster", "manifest_scope",
)


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def summarize(
    paths: list[Path], *, contract_path: Path = CONTRACT,
    manifest_path: Path = MANIFEST, ownership_path: Path = OWNERSHIP,
    allow_unowned_diagnostics: bool = False,
) -> dict[str, object]:
    contract_bytes = contract_path.read_bytes()
    contract = json.loads(contract_bytes)
    manifest_bytes = manifest_path.read_bytes()
    manifest = json.loads(manifest_bytes)
    _require(isinstance(manifest, list) and manifest == sorted(set(manifest)),
             "manifest must contain sorted unique IDs")
    expected_manifest = contract["suite_manifests"]["full"]
    _require(len(manifest) == expected_manifest["count"], "manifest count changed")
    _require(hashlib.sha256(manifest_bytes).hexdigest() == expected_manifest["sha256"],
             "manifest hash changed")
    _require(bool(paths), "at least one shard report is required")

    reports = []
    for path in paths:
        raw = path.read_bytes()
        report = json.loads(raw)
        _require(report.get("schema_version") == 2 and report.get("suite") == "full",
                 f"not a schema-v2 full census: {path}")
        _require(report.get("contract_sha256") == hashlib.sha256(contract_bytes).hexdigest(),
                 f"contract identity changed: {path}")
        _require(report["id_manifest"]["sha256"] == expected_manifest["sha256"],
                 f"manifest identity changed: {path}")
        _require(report["evidence"]["tolerance_pixels"] == 0,
                 f"nonzero tolerance: {path}")
        reports.append((report["shard"]["index"], report, path, hashlib.sha256(raw).hexdigest()))

    reports.sort(key=lambda item: item[0])
    first = reports[0][1]
    shard_count = first["shard"]["count"]
    _require(shard_count > 0 and [item[0] for item in reports] == list(range(shard_count)),
             "shard indices must be complete and unique")
    expected_profiles = {item["name"]: item for item in contract["qualification_profiles"]}
    observed: dict[str, dict[str, dict[str, object]]] = {
        name: {} for name in expected_profiles
    }
    input_files = []
    for index, report, path, digest in reports:
        _require(report["shard"] == {"index": index, "count": shard_count},
                 f"inconsistent shard metadata: {path}")
        for key in SHARED_KEYS:
            _require(report[key] == first[key], f"mixed {key} identity: {path}")
        expected_ids = manifest[index::shard_count]
        _require(report["id_manifest"]["count"] == len(expected_ids),
                 f"wrong shard manifest count: {path}")
        profiles = report["profiles"]
        _require({item["profile"] for item in profiles} == set(expected_profiles)
                 and len(profiles) == len(expected_profiles),
                 f"incomplete or duplicate profile set: {path}")
        profile_hashes = []
        for profile in profiles:
            name = profile["profile"]
            expected = expected_profiles[name]
            for field, value in (
                ("logical_size_css_px", [expected["logical_size_css_px"]["width"],
                                         expected["logical_size_css_px"]["height"]]),
                ("physical_size_px", [expected["physical_size_px"]["width"],
                                      expected["physical_size_px"]["height"]]),
                ("device_scale", expected["device_scale"]),
            ):
                _require(profile[field] == value, f"wrong {name} {field}: {path}")
            tests = profile["tests"]
            ids = [test["id"] for test in tests]
            _require(ids == expected_ids, f"missing, duplicate, or unordered IDs in {name}: {path}")
            _require(profile["ordered_id_sha256"] == hashlib.sha256(
                "\n".join(expected_ids).encode("utf-8")
            ).hexdigest(), f"ordered ID hash changed in {name}: {path}")
            _require(profile["result_sha256"] == residuals.canonical_sha256(tests),
                     f"profile result hash changed in {name}: {path}")
            profile_hashes.append(profile["result_sha256"])
            statuses = [test["status"] for test in tests]
            _require(set(statuses) <= {"exact", "different", "error"},
                     f"unknown comparison status in {name}: {path}")
            for status, field in (("exact", "exact"), ("different", "different"),
                                  ("error", "errors")):
                _require(profile[field] == statuses.count(status),
                         f"inconsistent {field} count in {name}: {path}")
            _require(profile["total"] == len(tests), f"wrong total in {name}: {path}")
            for test in tests:
                if test["status"] == "different":
                    signature = test.get("diff_signature")
                    _require(isinstance(signature, dict)
                             and signature.get("comparable") is True
                             and signature.get("mismatched_pixels") == test.get("mismatched_pixels")
                             and isinstance(signature.get("mismatch_bounds"), dict)
                             and isinstance(signature.get("connected_region_count"), int)
                             and isinstance(signature.get("channel_deltas"), dict),
                             f"incomplete pixel evidence for {test['id']} in {name}: {path}")
                observed[name][test["id"]] = test
        _require(report["results"]["profile_result_sha256"] ==
                 residuals.canonical_sha256(profile_hashes),
                 f"report result hash changed: {path}")
        for field in ("total", "exact", "different", "errors"):
            _require(report["results"][field] == sum(p[field] for p in profiles),
                     f"report {field} count changed: {path}")
        input_files.append({"shard": index, "sha256": digest})

    _require(all(set(tests) == set(manifest) for tests in observed.values()),
             "shard union does not cover the complete manifest")
    ownership, ownership_hash = residuals.load_ownership(ownership_path)
    profile_order = [item["name"] for item in contract["qualification_profiles"]]
    profile_counts = {}
    for name in profile_order:
        results = observed[name].values()
        profile_counts[name] = {
            status: sum(test["status"] == status for test in results)
            for status in ("exact", "different", "error")
        }
    entries = []
    for test_id in manifest:
        comparisons = []
        for name in profile_order:
            test = observed[name][test_id]
            if test["status"] == "exact":
                continue
            signature = test.get("diff_signature") or {}
            comparisons.append({
                "profile": name,
                "status": test["status"],
                "mismatched_pixels": test.get("mismatched_pixels"),
                "mismatch_bounds": signature.get("mismatch_bounds"),
                "connected_region_count": signature.get("connected_region_count"),
                "channel_delta_extrema": {
                    channel: stats["maximum_absolute_delta"]
                    for channel, stats in signature.get("channel_deltas", {}).items()
                },
                "diff_signature_sha256": residuals.canonical_sha256(signature),
                "font_profile": test.get("font_profile"),
                "raster_configuration": test.get("raster_configuration"),
            })
        if comparisons:
            review = ownership.get(test_id)
            entries.append({
                "test_id": test_id,
                "comparisons": comparisons,
                "root_cause": review["root_cause"] if review else None,
                "owning_subsystem": review["owning_subsystem"] if review else None,
                "minimized_reproducer": review["minimized_reproducer"] if review else None,
            })
    unowned = [entry["test_id"] for entry in entries if entry["root_cause"] is None]
    if unowned and not allow_unowned_diagnostics:
        raise ValueError(f"unowned renderer residuals: {len(unowned)} tests")
    totals = {status: sum(item[status] for item in profile_counts.values())
              for status in ("exact", "different", "error")}
    fractional = {name for name in profile_order
                  if not float(expected_profiles[name]["device_scale"]).is_integer()}
    snapshot = {
        "schema_version": 1,
        "qualifying": False,
        "policy": "diagnostic evidence only; no tolerance, allowlist, or baseline change",
        "contract_sha256": first["contract_sha256"],
        "manifest_sha256": expected_manifest["sha256"],
        "manifest_count": len(manifest),
        "source": first["source"],
        "chromium": {key: first["chromium"][key] for key in
                     ("build_identity", "binary_sha256", "capture_harness_sha256")},
        "openui": first["openui"],
        "ownership_input_sha256": ownership_hash,
        "input_reports": input_files,
        "input_reports_sha256": residuals.canonical_sha256(input_files),
        "comparison_counts": totals,
        "profile_counts": profile_counts,
        "residual_test_count": len(entries),
        "residual_comparison_count": totals["different"] + totals["error"],
        "fractional_only_test_count": sum(
            all(comparison["profile"] in fractional for comparison in entry["comparisons"])
            for entry in entries
        ),
        "unowned_test_count": len(unowned),
        "entries_sha256": residuals.canonical_sha256(entries),
        "entries": entries,
    }
    snapshot["output_sha256"] = residuals.canonical_sha256(snapshot)
    return snapshot


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reports", nargs="+", type=Path)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    parser.add_argument("--allow-unowned-diagnostics", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    snapshot = summarize(args.reports, allow_unowned_diagnostics=args.allow_unowned_diagnostics)
    rendered = json.dumps(snapshot, sort_keys=True, indent=2) + "\n"
    if args.check:
        _require(args.output.read_text(encoding="utf-8") == rendered,
                 f"census snapshot is stale: {args.output}")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(f"{snapshot['comparison_counts']}; residual tests={snapshot['residual_test_count']}; "
          f"unowned={snapshot['unowned_test_count']}; sha256={snapshot['output_sha256']}")


if __name__ == "__main__":
    main()
