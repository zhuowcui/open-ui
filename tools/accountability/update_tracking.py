#!/usr/bin/env python3
"""
update_tracking.py — Update feature matrix CSVs from pixel comparison results.

Usage:
    python3 update_tracking.py --pixel-results <results_dir> --feature-dir <feature_dir>

Reads result.json files from pixel comparison runs and updates the corresponding
rows in feature matrix CSVs.
"""

import argparse
import csv
import json
import os
import sys
from pathlib import Path


def normalize(s: str) -> str:
    """Normalize a name: lowercase, replace hyphens/spaces/plus with underscores."""
    return s.lower().replace("-", "_").replace(" ", "_").replace("+", "_")


def load_pixel_results(results_dir: str) -> dict:
    """Load pixel results from summary.json (authoritative) or result.json files."""
    results = {}
    results_path = Path(results_dir)
    if not results_path.exists():
        return results

    # Prefer summary.json (authoritative)
    summary = results_path / "summary.json"
    if summary.exists():
        with open(summary) as f:
            data = json.load(f)
        for test in data.get("tests", []):
            tid = test.get("id", "")
            results[tid] = {
                "status": test.get("status", "error"),
                "mismatch_pct": test.get("mismatch_pct", 0.0),
            }
        return results

    # Fallback: scan result.json files
    for sp_dir in results_path.iterdir():
        if not sp_dir.is_dir():
            continue
        for test_dir in sp_dir.iterdir():
            if not test_dir.is_dir():
                continue
            json_path = test_dir / "result.json"
            if json_path.exists():
                with open(json_path) as f:
                    data = json.load(f)
                key = f"{sp_dir.name}/{test_dir.name}"
                results[key] = data
    return results


def build_lookup(results: dict) -> dict:
    """Build a normalized lookup from test IDs to results."""
    lookup = {}
    for key, val in results.items():
        # Key format: "sp12/margin_basic"
        lookup[key] = val
        # Also index by just test name (no sp prefix)
        parts = key.split("/", 1)
        if len(parts) == 2:
            lookup[parts[1]] = val
    return lookup


def find_result(lookup: dict, sp_prefix: str, feature: str, sub: str, variant: str):
    """Try multiple key formats to find a matching pixel result."""
    candidates = []

    # 1. variant as-is (new rows use test_id as variant)
    candidates.append(f"{sp_prefix}/{variant}")
    candidates.append(f"{sp_prefix}/{normalize(variant)}")

    # 2. feature_variant (e.g., "text-decoration-line_underline" → "text_decoration_line_underline")
    fv = f"{feature}_{variant}"
    candidates.append(f"{sp_prefix}/{normalize(fv)}")

    # 3. feature_sub_variant
    if sub:
        fsv = f"{feature}_{sub}_{variant}"
        candidates.append(f"{sp_prefix}/{normalize(fsv)}")

    # 4. feature without sub suffix + variant (e.g., "text-decoration" + "underline")
    if sub and feature.endswith(f"-{sub}"):
        stem = feature[: -len(f"-{sub}")]
        sv = f"{stem}_{variant}"
        candidates.append(f"{sp_prefix}/{normalize(sv)}")

    for c in candidates:
        if c in lookup:
            return lookup[c]
    return None


def update_feature_csv(csv_path: str, lookup: dict, sp_prefix: str):
    """Update pixel comparison columns in a feature matrix CSV."""
    if not os.path.exists(csv_path):
        print(f"  Skipping {csv_path} — not found", file=sys.stderr)
        return

    rows = []
    updated = 0
    with open(csv_path, newline="") as f:
        reader = csv.DictReader(f)
        fieldnames = reader.fieldnames
        for row in reader:
            feature = row.get("feature", "").strip()
            variant = row.get("variant", "").strip()
            sub = row.get("sub_feature", "").strip()

            result = find_result(lookup, sp_prefix, feature, sub, variant)
            if result:
                row["pixel_compared_with_chromium"] = "yes"
                status = result.get("status", "error")
                if status == "pass":
                    row["pixel_result"] = "pass"
                    row["pixel_diff_pct"] = "0.0"
                elif status == "fail":
                    row["pixel_result"] = "fail"
                    row["pixel_diff_pct"] = str(result.get("mismatch_pct", ""))
                else:
                    row["pixel_result"] = "error"
                    row["pixel_diff_pct"] = ""
                updated += 1

            rows.append(row)

    # Write back
    with open(csv_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print(f"  {os.path.basename(csv_path)}: {updated} rows updated from pixel results")


def main():
    parser = argparse.ArgumentParser(description="Update feature CSVs from pixel comparison results")
    parser.add_argument("--pixel-results", required=True, help="Path to pixel comparison results directory")
    parser.add_argument("--feature-dir", required=True, help="Path to feature matrix CSVs directory")
    args = parser.parse_args()

    results = load_pixel_results(args.pixel_results)
    print(f"Loaded {len(results)} pixel comparison results")

    lookup = build_lookup(results)

    update_feature_csv(
        os.path.join(args.feature_dir, "sp11_text_features.csv"),
        lookup, "sp11"
    )
    update_feature_csv(
        os.path.join(args.feature_dir, "sp12_block_features.csv"),
        lookup, "sp12"
    )
    update_feature_csv(
        os.path.join(args.feature_dir, "sp13_inline_features.csv"),
        lookup, "sp13"
    )


if __name__ == "__main__":
    main()
