#!/usr/bin/env python3
"""Compare complete, clean CPU and Ganesh raster gates against one Chromium oracle."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

import residuals
from summarize_raster_gates import CONTRACT, checked_report, sha256


def summarize(paths: dict[str, dict[str, Path]]) -> dict:
    contract = json.loads(CONTRACT.read_text())
    ganesh_contract = {
        **contract,
        "raster": {**contract["raster"], "qualification_backend": "ganesh-gl"},
    }
    reports = {
        backend: {
            suite: checked_report(path, suite, contract if backend == "cpu-skia" else ganesh_contract)
            for suite, path in suites.items()
        }
        for backend, suites in paths.items()
    }
    for backend, suites in reports.items():
        if suites["focused"]["source"] != suites["primitive"]["source"]:
            raise ValueError(f"{backend} reports have different source identities")
        if suites["focused"]["openui"] != suites["primitive"]["openui"]:
            raise ValueError(f"{backend} reports have different runner identities")

    changes = []
    totals = {}
    for suite in ("focused", "primitive"):
        cpu = reports["cpu-skia"][suite]
        ganesh = reports["ganesh-gl"][suite]
        for key in ("chromium", "contract_sha256", "font_byte_hashes", "resource_hashes"):
            if cpu[key] != ganesh[key]:
                raise ValueError(f"{suite} {key} differs between raster backends")
        if cpu["results"]["errors"] or ganesh["results"]["errors"]:
            raise ValueError(f"{suite} contains render errors")

        transitions: Counter[str] = Counter()
        per_profile = []
        for cpu_profile, ganesh_profile in zip(cpu["profiles"], ganesh["profiles"], strict=True):
            if cpu_profile["profile"] != ganesh_profile["profile"]:
                raise ValueError(f"{suite} profile order differs")
            per_profile.append({
                "profile": cpu_profile["profile"],
                "cpu_exact": cpu_profile["exact"],
                "ganesh_exact": ganesh_profile["exact"],
            })
            for cpu_row, ganesh_row in zip(cpu_profile["tests"], ganesh_profile["tests"], strict=True):
                if cpu_row["id"] != ganesh_row["id"]:
                    raise ValueError(f"{suite} test order differs")
                for key in ("chromium_oracle_identity_sha256", "chromium_oracle_rgba_sha256"):
                    if cpu_row[key] != ganesh_row[key]:
                        raise ValueError(f"{suite} Chromium oracle changed for {cpu_row['id']}")
                transitions[f"{cpu_row['status']}->{ganesh_row['status']}"] += 1
                if cpu_row["openui_rgba_sha256"] != ganesh_row["openui_rgba_sha256"]:
                    changes.append({
                        "suite": suite,
                        "profile": cpu_profile["profile"],
                        "test_id": cpu_row["id"],
                        "cpu_status": cpu_row["status"],
                        "ganesh_status": ganesh_row["status"],
                        "cpu_mismatched_pixels": cpu_row["mismatched_pixels"],
                        "ganesh_mismatched_pixels": ganesh_row["mismatched_pixels"],
                        "ganesh_mismatch_bounds": ganesh_row["diff_signature"]["mismatch_bounds"],
                    })
        totals[suite] = {
            "cpu": cpu["results"],
            "ganesh": ganesh["results"],
            "status_transitions": dict(sorted(transitions.items())),
            "profiles": per_profile,
        }

    result = {
        "schema_version": 1,
        "status": "diagnostic-unpromoted",
        "contract_sha256": sha256(CONTRACT),
        "chromium": reports["cpu-skia"]["focused"]["chromium"],
        "source": {backend: suites["focused"]["source"] for backend, suites in reports.items()},
        "runner": {backend: suites["focused"]["openui"] for backend, suites in reports.items()},
        "input_reports": {
            backend: {suite: sha256(path) for suite, path in suites.items()}
            for backend, suites in paths.items()
        },
        "suites": totals,
        "changed_comparisons": changes,
        "chromium_oracle_unchanged": True,
        "ganesh_promoted": False,
    }
    result["output_sha256"] = residuals.canonical_sha256(result)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for backend in ("cpu", "ganesh"):
        for suite in ("focused", "primitive"):
            parser.add_argument(f"--{backend}-{suite}", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    paths = {
        "cpu-skia": {suite: getattr(args, f"cpu_{suite}") for suite in ("focused", "primitive")},
        "ganesh-gl": {suite: getattr(args, f"ganesh_{suite}") for suite in ("focused", "primitive")},
    }
    result = summarize(paths)
    output = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.check:
        if args.output.read_text() != output:
            raise SystemExit(f"backend comparison changed: {args.output}")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output)
    for suite, totals in result["suites"].items():
        print(f"{suite}: CPU {totals['cpu']['exact']}/{totals['cpu']['total']}, "
              f"Ganesh {totals['ganesh']['exact']}/{totals['ganesh']['total']}")


if __name__ == "__main__":
    main()
