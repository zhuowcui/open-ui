#!/usr/bin/env python3
"""Audit what the frozen SP20 pass records actually prove.

The historical comparator allowed channel differences and omitted a viewport
strip. A fresh matrix report can additionally compare the immutable Open UI
archive with the zero-tolerance Chromium oracle without rewriting either.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import sys
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from tools.accountability import pixel_diff, restore_frozen_openui_archive as frozen  # noqa: E402
from tools.qualification import residuals  # noqa: E402

MANIFEST = ROOT / "tools/qualification/manifests/complete-5731.json"
HISTORICAL_RESULTS = ROOT / "tools/accountability/data/pixel_comparison/results"
EXAMPLE_ID = "wpt/css_backgrounds/background-image-gradient-interpolation-repaint-ref"


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def rgba_sha256(png: bytes) -> str:
    with Image.open(io.BytesIO(png)) as image:
        return sha256(image.convert("RGBA").tobytes())


def historical_audit(ids: list[str]) -> dict[str, object]:
    tolerance = pixel_diff.compare_images.__defaults__[0]
    if tolerance != 4:
        raise ValueError(f"historical comparator policy changed: {tolerance}")
    positive = []
    compared_widths = set()
    for test_id in ids:
        path = HISTORICAL_RESULTS / test_id / "result.json"
        row = json.loads(path.read_text(encoding="utf-8"))
        if row["status"] != "pass" or row["mismatched_pixels"] != 0:
            raise ValueError(f"historical pass record changed: {path}")
        if row["image_a_size"] != [800, 600] or row["image_b_size"] != [800, 600]:
            raise ValueError(f"historical image size changed: {path}")
        if row["total_pixels"] % 600:
            raise ValueError(f"invalid compared pixel count: {path}")
        compared_widths.add(row["total_pixels"] // 600)
        if row["max_channel_diff"] > 0:
            positive.append(test_id)
    if compared_widths != {785}:
        raise ValueError(f"historical comparison widths changed: {compared_widths}")
    return {
        "reported_passes": len(ids),
        "channel_tolerance": tolerance,
        "compared_width_px": 785,
        "excluded_right_strip_px": 15,
        "passes_with_nonzero_compared_channel_delta": len(positive),
        "nonzero_delta_ids_sha256": sha256(("\n".join(positive) + "\n").encode()),
        "establishes_zero_tolerance_equality": False,
    }


def matrix_audit(report_path: Path, ids: list[str], images: dict[str, bytes]) -> dict[str, object]:
    raw = report_path.read_bytes()
    report = json.loads(raw)
    if report.get("suite") != "full" or not report.get("complete_contract_scope"):
        raise ValueError("a complete four-profile full matrix report is required")
    if not report["source"]["clean"] or report["evidence"]["tolerance_pixels"] != 0:
        raise ValueError("matrix report must have clean source and zero tolerance")
    if report["id_manifest"]["sha256"] != sha256(MANIFEST.read_bytes()):
        raise ValueError("matrix report uses a different immutable manifest")
    profiles = [item for item in report["profiles"] if item["profile"] == "legacy-800x600@1"]
    if len(profiles) != 1 or [item["id"] for item in profiles[0]["tests"]] != ids:
        raise ValueError("matrix report lacks the complete legacy profile")

    counts = {
        "archive_vs_live_oracle_different": 0,
        "archive_vs_current_renderer_different": 0,
        "current_renderer_vs_live_oracle_different": 0,
        "all_three_exact": 0,
    }
    example = None
    for row in profiles[0]["tests"]:
        test_id = row["id"]
        if row["status"] == "error":
            raise ValueError(f"matrix legacy profile has render error: {test_id}")
        if row["chromium_rgba_sha256"] != row["chromium_oracle_rgba_sha256"]:
            raise ValueError(f"Chromium capture differs from immutable oracle: {test_id}")
        archive_png = images[f"{test_id}/openui.png"]
        archive_rgba = rgba_sha256(archive_png)
        oracle_rgba = row["chromium_rgba_sha256"]
        renderer_rgba = row["openui_rgba_sha256"]
        if archive_rgba != oracle_rgba:
            counts["archive_vs_live_oracle_different"] += 1
        if archive_rgba != renderer_rgba:
            counts["archive_vs_current_renderer_different"] += 1
        if renderer_rgba != oracle_rgba:
            counts["current_renderer_vs_live_oracle_different"] += 1
        if archive_rgba == renderer_rgba == oracle_rgba:
            counts["all_three_exact"] += 1
        if test_id == EXAMPLE_ID:
            live_oracle = report_path.parent / "legacy-800x600@1" / test_id / "chromium.png"
            if not live_oracle.is_file():
                live_oracle = (
                    Path(report["chromium"]["oracle_cache"]).parent
                    / row["chromium_png_cache_path"]
                )
            if not live_oracle.is_file() or sha256(live_oracle.read_bytes()) != row["chromium_png_sha256"]:
                raise ValueError("minimized example's live oracle PNG is unavailable or changed")
            with tempfile.TemporaryDirectory() as directory:
                archived = Path(directory) / "frozen.png"
                archived.write_bytes(archive_png)
                signature = residuals.analyze_image_difference(live_oracle, archived)
            example = {
                "test_id": test_id,
                "template": json.loads((ROOT / "tools/accountability/data/wpt_ported/all_wpt_templates.json").read_text())[test_id],
                "frozen_openui_png_sha256": sha256(archive_png),
                "frozen_openui_rgba_sha256": archive_rgba,
                "live_chromium_png_sha256": row["chromium_png_sha256"],
                "live_chromium_rgba_sha256": oracle_rgba,
                "current_openui_rgba_sha256": renderer_rgba,
                "mismatched_pixels": signature["mismatched_pixels"],
                "mismatch_bounds": signature["mismatch_bounds"],
                "connected_region_count": signature["connected_region_count"],
                "maximum_absolute_channel_delta": {
                    name: stats["maximum_absolute_delta"]
                    for name, stats in signature["channel_deltas"].items()
                },
            }
    if example is None:
        raise ValueError("minimized example is absent from matrix report")
    return {
        "matrix_report_sha256": sha256(raw),
        "commit": report["commit"],
        "source_tree_sha256": report["source"]["source_tree_sha256"],
        "chromium_binary_sha256": report["chromium"]["binary_sha256"],
        "chromium_capture_harness_sha256": report["chromium"]["capture_harness_sha256"],
        "openui_binary_sha256": report["openui"]["binary_sha256"],
        "legacy_profile": counts,
        "minimized_example": example,
    }


def prior_local_capture_audit(
    prior_root: Path, report_path: Path, ids: list[str], images: dict[str, bytes]
) -> dict[str, object]:
    """Describe ignored workstation captures; never treat them as pinned proof."""

    report = json.loads(report_path.read_text(encoding="utf-8"))
    legacy = next(item for item in report["profiles"] if item["profile"] == "legacy-800x600@1")
    rows = {item["id"]: item for item in legacy["tests"]}
    counts = {
        "archived_openui_vs_prior_chromium_different": 0,
        "prior_vs_current_fixture_different": 0,
        "prior_vs_live_chromium_different": 0,
        "oracle_difference_without_fixture_change": 0,
    }
    changed_oracles = []
    input_hashes = []
    for test_id in ids:
        prior = prior_root / test_id
        current = report_path.parent / "legacy-800x600@1" / test_id
        prior_html = (prior / "test.html").read_bytes()
        prior_chrome_png = (prior / "chromium.png").read_bytes()
        current_html = (current / "test.html").read_bytes()
        prior_oracle = rgba_sha256(prior_chrome_png)
        live_oracle = rows[test_id]["chromium_rgba_sha256"]
        fixture_changed = prior_html != current_html
        oracle_changed = prior_oracle != live_oracle
        input_hashes.append((test_id, sha256(prior_html), sha256(prior_chrome_png)))
        if rgba_sha256(images[f"{test_id}/openui.png"]) != prior_oracle:
            counts["archived_openui_vs_prior_chromium_different"] += 1
        if fixture_changed:
            counts["prior_vs_current_fixture_different"] += 1
        if oracle_changed:
            counts["prior_vs_live_chromium_different"] += 1
            if not fixture_changed:
                counts["oracle_difference_without_fixture_change"] += 1
            changed_oracles.append(
                {
                    "test_id": test_id,
                    "prior_fixture_sha256": sha256(prior_html),
                    "current_fixture_sha256": sha256(current_html),
                    "prior_chromium_rgba_sha256": prior_oracle,
                    "live_chromium_rgba_sha256": live_oracle,
                }
            )
    return {
        "evidence_kind": "local-ignored-captures-diagnostic",
        "prior_input_manifest_sha256": sha256(
            json.dumps(input_hashes, separators=(",", ":")).encode()
        ),
        "counts": counts,
        "changed_oracles": changed_oracles,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix-report", type=Path)
    parser.add_argument("--prior-local-captures", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--require-original-exact", action="store_true")
    args = parser.parse_args()
    ids = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if ids != sorted(set(ids)) or len(ids) != 5731:
        raise SystemExit("immutable full manifest changed")
    images = frozen.frozen_images()
    report = {
        "schema_version": 1,
        "status": "blocked-historical-exact-proof",
        "frozen_archive_sha256": sha256(frozen.ARCHIVE.read_bytes()),
        "manifest_sha256": sha256(MANIFEST.read_bytes()),
        "historical": historical_audit(ids),
    }
    if args.matrix_report is not None:
        report["live_matrix"] = matrix_audit(args.matrix_report, ids, images)
    if args.prior_local_captures is not None:
        if args.matrix_report is None:
            raise SystemExit("--prior-local-captures requires --matrix-report")
        report["prior_local_captures"] = prior_local_capture_audit(
            args.prior_local_captures, args.matrix_report, ids, images
        )
    output = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output is not None:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output, encoding="utf-8")
    else:
        print(output, end="")
    if args.require_original_exact and (
        report["historical"]["passes_with_nonzero_compared_channel_delta"]
        or report.get("live_matrix", {}).get("legacy_profile", {}).get(
            "archive_vs_live_oracle_different", 0
        )
    ):
        raise SystemExit("frozen historical pass records do not establish exact oracle pixels")


if __name__ == "__main__":
    main()
