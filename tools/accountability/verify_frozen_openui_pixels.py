#!/usr/bin/env python3
"""Re-render the frozen SP20 manifest and byte-compare archived OpenUI PNGs."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import run_all_pixel_comparisons as runner


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "tools/wpt/sp20_focused_ids.json"
RESULTS = ROOT / "tools/accountability/data/pixel_comparison/results"
WPT_TEMPLATES = ROOT / "tools/accountability/data/wpt_ported/all_wpt_templates.json"
sys.path.insert(0, str(ROOT / "tools/qualification"))
import residuals  # noqa: E402


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_state() -> dict[str, object]:
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
        capture_output=True, text=True,
    ).stdout.strip()
    status = subprocess.run(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        cwd=ROOT, check=True, capture_output=True,
    ).stdout
    return {"commit": commit, "clean": not status, "status_sha256": hashlib.sha256(status).hexdigest()}


def load_ids(path: Path) -> list[str]:
    values = json.loads(path.read_text(encoding="utf-8"))
    if (
        not isinstance(values, list)
        or any(not isinstance(value, str) or not value for value in values)
        or len(values) != len(set(values))
    ):
        raise ValueError(f"invalid exact-ID manifest: {path}")
    return values


def load_set(path: str) -> set[str]:
    return set(load_ids(Path(path)))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--pixel-compare", type=Path, default=Path(runner.PIXEL_COMPARE))
    parser.add_argument("--results-dir", type=Path, default=RESULTS)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--jobs", type=int, default=8)
    args = parser.parse_args()
    if args.jobs < 1:
        raise SystemExit("--jobs must be positive")
    if not args.pixel_compare.is_file():
        raise SystemExit(f"missing release pixel renderer: {args.pixel_compare}")
    source_before = source_state()
    binary_before = sha256(args.pixel_compare)

    ids = load_ids(args.manifest)
    text_ids = load_set(runner.TEXT_PORTED_LIST)
    real_font_ids = load_set(runner.REAL_FONT_LIST)
    templates = dict(runner.HTML_TEMPLATES)
    templates.update(json.loads(WPT_TEMPLATES.read_text(encoding="utf-8")))

    registered = set(
        subprocess.run(
            [args.pixel_compare, "list"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.splitlines()
    )
    unknown = sorted(set(ids) - registered)
    if unknown:
        raise SystemExit(f"manifest contains {len(unknown)} unregistered IDs")

    def verify(item: tuple[int, str], temporary: Path) -> tuple[int, str, str | None, dict[str, object] | None]:
        index, test_id = item
        expected = args.results_dir / test_id / "openui.png"
        if not expected.is_file():
            return index, test_id, "missing restored frozen openui.png", None
        template = templates.get(test_id, "")
        preserve_subpixel = bool(
            re.search(
                r'<input\b[^>]*\btype\s*=\s*["\']?range\b',
                template,
                re.IGNORECASE,
            )
        )
        actual = temporary / f"{index}.png"
        use_real_font = test_id in real_font_ids
        use_ahem_noaa = test_id in text_ids and not use_real_font
        raster_profile = "default"
        if use_real_font:
            raster_profile = "legacy-chromium-linux-lcd"
        elif use_ahem_noaa:
            raster_profile = (
                "legacy-deterministic-alias-subpixel"
                if preserve_subpixel
                else "legacy-deterministic-alias"
            )
        environment = runner.openui_environment(use_real_font=use_real_font)
        completed = subprocess.run(
            [
                args.pixel_compare,
                "render",
                test_id,
                actual,
                "--raster-config",
                raster_profile,
            ],
            env=environment,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            timeout=120,
        )
        if completed.returncode != 0:
            detail = completed.stderr.decode("utf-8", errors="replace").strip()
            return index, test_id, f"render failed: {detail}", None
        if actual.read_bytes() != expected.read_bytes():
            signature = residuals.analyze_image_difference(expected, actual)
            evidence = {
                "frozen_png_sha256": sha256(expected),
                "replay_png_sha256": sha256(actual),
                "mismatched_pixels": signature.get("mismatched_pixels"),
                "mismatch_bounds": signature.get("mismatch_bounds"),
                "connected_region_count": signature.get("connected_region_count"),
                "largest_connected_regions": signature.get("connected_regions", [])[:5],
                "channel_delta_extrema": {
                    name: stats["maximum_absolute_delta"]
                    for name, stats in signature.get("channel_deltas", {}).items()
                },
                "diff_signature_sha256": residuals.canonical_sha256(signature),
            }
            return index, test_id, "PNG bytes differ from frozen result", evidence
        return index, test_id, None, None

    failures: list[tuple[int, str, str, dict[str, object] | None]] = []
    completed_count = 0
    with tempfile.TemporaryDirectory(prefix="openui-sp20-replay-") as directory:
        temporary = Path(directory)
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
            futures = [executor.submit(verify, item, temporary) for item in enumerate(ids)]
            for future in concurrent.futures.as_completed(futures):
                index, test_id, error, evidence = future.result()
                completed_count += 1
                if error is not None:
                    failures.append((index, test_id, error, evidence))
                if completed_count % 500 == 0:
                    print(f"replayed {completed_count}/{len(ids)} frozen pixels")

    failures.sort()
    if args.report is not None:
        source_after = source_state()
        binary_after = sha256(args.pixel_compare)
        report = {
            "schema_version": 1,
            "evidence_kind": "frozen-openui-byte-replay",
            "qualified_archive_fidelity": (
                not failures and bool(source_before["clean"])
                and source_before == source_after and binary_before == binary_after
            ),
            "manifest_sha256": sha256(args.manifest),
            "pixel_compare_sha256": binary_before,
            "pixel_compare_stable": binary_before == binary_after,
            "source_before": source_before,
            "source_after": source_after,
            "results": {
                "total": len(ids),
                "byte_identical": len(ids) - len(failures),
                "changed_or_error": len(failures),
            },
            "failures": [
                {"test_id": test_id, "reason": error, "pixel_evidence": evidence}
                for _, test_id, error, evidence in failures
            ],
        }
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(f"frozen replay report: {args.report}")

    if failures:
        for _, test_id, error, _ in failures[:50]:
            print(f"FAIL {test_id}: {error}")
        raise SystemExit(f"{len(failures)}/{len(ids)} frozen pixel replays changed")
    print(f"frozen pixel replay: {len(ids)}/{len(ids)} byte-identical")


if __name__ == "__main__":
    main()
