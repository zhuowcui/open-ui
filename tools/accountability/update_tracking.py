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


def load_pixel_results(results_dir: str) -> dict:
    """Load all result.json files into a dict keyed by test name."""
    results = {}
    results_path = Path(results_dir)
    if not results_path.exists():
        return results

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
                # Key format: sp/test_name (e.g., sp11/text-decoration-line_underline)
                key = f"{sp_dir.name}/{test_dir.name}"
                results[key] = data
    return results


def update_feature_csv(csv_path: str, results: dict, sp_prefix: str):
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
            # Build test key from feature + variant
            feature = row.get("feature", "").strip()
            variant = row.get("variant", "").strip()
            sub = row.get("sub_feature", "").strip()

            # Try multiple key formats
            test_name = f"{feature}_{variant}".replace(" ", "_").replace("+", "_")
            if sub:
                test_name_alt = f"{feature}_{sub}_{variant}".replace(" ", "_")
            else:
                test_name_alt = test_name

            key = f"{sp_prefix}/{test_name}"
            key_alt = f"{sp_prefix}/{test_name_alt}"

            result = results.get(key) or results.get(key_alt)
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

    update_feature_csv(
        os.path.join(args.feature_dir, "sp11_text_features.csv"),
        results, "sp11"
    )
    update_feature_csv(
        os.path.join(args.feature_dir, "sp12_block_features.csv"),
        results, "sp12"
    )
    update_feature_csv(
        os.path.join(args.feature_dir, "sp13_inline_features.csv"),
        results, "sp13"
    )


if __name__ == "__main__":
    main()
