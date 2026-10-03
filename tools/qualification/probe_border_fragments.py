#!/usr/bin/env python3
"""Compare native Engine border contexts with fresh, preserved Chromium captures.

This is a diagnostic sweep, not a release qualification run. No existing
Chromium cache, fixture, or historical output is read or rewritten.
"""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import itertools
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from tools.accountability import run_all_pixel_comparisons as capture
from tools.qualification import run_renderer_matrix as matrix
from tools.qualification.residuals import analyze_image_difference


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def image_identity(path: Path) -> dict[str, object]:
    dimensions, rgba = matrix.image_payload(path)
    return {
        "png_sha256": sha256(path),
        "rgba_sha256": hashlib.sha256(rgba).hexdigest(),
        "dimensions": list(dimensions),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--results-dir", type=Path, required=True)
    parser.add_argument("--mode", choices=("plain", "clip", "columns"), action="append")
    parser.add_argument("--cases-file", type=Path, help="Explicit [mode, scale, width, offset] rows")
    parser.add_argument("--jobs", type=int, default=4)
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    baseline = args.baseline.resolve(strict=True)
    candidate = args.candidate.resolve(strict=True)
    results = args.results_dir.resolve()
    results.parent.mkdir(parents=True, exist_ok=True)
    results.mkdir()  # Never replace an earlier capture or report.
    chrome, chrome_dir = capture.find_chrome()
    if chrome is None:
        raise RuntimeError("pinned Chromium is unavailable")
    chromium = Path(chrome)
    contract = json.loads(
        (ROOT / "docs/renderer/generated/qualification-contract-v2.json").read_text()
    )
    version = matrix.chromium_identity(chromium, Path(chrome_dir))
    if version != contract["chromium"]["raster_oracle_build_identity"]:
        raise RuntimeError(f"unexpected Chromium build: {version}")
    source = matrix.repository_source_identity()
    binary_hashes = {"baseline": sha256(baseline), "candidate": sha256(candidate)}
    cases = list(itertools.product(
        args.mode or ("plain", "clip", "columns"),
        (1.0, 1.25, 1.5, 2.0, 3.0), range(1, 6), range(4),
    ))
    if args.cases_file:
        selection = [tuple(row) for row in json.loads(args.cases_file.read_text())]
        if not selection or len(set(selection)) != len(selection) or any(
            row not in cases for row in selection
        ):
            parser.error("--cases-file must select distinct rows from the declared sweep")
        cases = selection

    def probe(case: tuple[str, float, int, int]) -> dict[str, object]:
        mode, scale, border_width, offset = case
        label = f"{mode}-dpr{scale:g}-width{border_width}-offset{offset}"
        directory = results / label
        directory.mkdir()
        native = {}
        for name, binary in (("baseline", baseline), ("candidate", candidate)):
            output = directory / name
            completed = subprocess.run(
                [str(binary), str(output), str(scale), str(border_width),
                 str(offset), "320", "300", mode],
                check=True, capture_output=True, text=True, timeout=120,
            )
            native[name] = {
                "geometry": completed.stdout,
                "image": image_identity(output / "openui.png"),
            }
        html = directory / "baseline/test.html"
        if html.read_bytes() != (directory / "candidate/test.html").read_bytes():
            raise RuntimeError(f"native inputs disagree for {label}")
        attempts = []
        captured = False
        for attempt in range(3):
            oracle = directory / (
                "chromium.png" if attempt == 0 else f"chromium-attempt-{attempt + 1}.png"
            )
            captured = capture.render_chrome(
                str(html), str(oracle), chrome, chrome_dir, use_ahem_noaa=True,
                logical_width=320, logical_height=300, device_scale=scale,
            )
            attempts.append({
                "path": oracle.name, "capture_succeeded": captured,
                "preserved_file_sha256": sha256(oracle) if oracle.exists() else None,
            })
            if captured:
                break
        if not captured:
            raise RuntimeError(f"Chromium capture failed for {label}; inputs preserved")
        for name in native:
            native[name]["difference"] = analyze_image_difference(
                oracle, directory / name / "openui.png",
            )
        return {
            "id": label, "mode": mode, "device_scale": scale,
            "border_width": border_width, "offset": offset,
            "logical_viewport": [320, 300], "fixture_sha256": sha256(html),
            "chromium": {"path": oracle.name, **image_identity(oracle)},
            "capture_attempts": attempts, **native,
        }

    records = []
    errors = []
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        futures = {pool.submit(probe, case): case for case in cases}
        for future in as_completed(futures):
            try:
                records.append(future.result())
            except Exception as error:
                errors.append({"case": list(futures[future]), "error": str(error)})
            if (len(records) + len(errors)) % 20 == 0:
                print(f"completed={len(records)} errors={len(errors)} total={len(cases)}", flush=True)
    if source != matrix.repository_source_identity():
        raise RuntimeError("source changed during the diagnostic")
    if binary_hashes != {"baseline": sha256(baseline), "candidate": sha256(candidate)}:
        raise RuntimeError("native binary changed during the diagnostic")
    report = {
        "schema_version": 1, "release_qualification": False,
        "scope": "Native Engine hollow-border context diagnostic; no census update",
        "source": source, "native_binary_sha256": binary_hashes,
        "native_example_sha256": sha256(
            ROOT / "bindings/rust/openui-engine/examples/border_raster_reproducer.rs"
        ),
        "chromium": {
            "build_identity": version, "binary_sha256": sha256(chromium),
            "capture_harness_sha256": sha256(Path(capture.__file__)),
            "font_profile": "ahem-noaa", "fixture_fonts_and_resources": "none",
            "fontconfig_sha256": sha256(Path(capture.AHEM_FONTCONFIG)),
        },
        "tolerance_pixels": 0, "case_count": len(cases),
        "errors": errors, "cases": sorted(records, key=lambda item: item["id"]),
    }
    (results / "summary.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(f"report={results / 'summary.json'} completed={len(records)} errors={len(errors)}")
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
