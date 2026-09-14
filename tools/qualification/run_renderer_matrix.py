#!/usr/bin/env python3
"""Run the versioned renderer matrix against the pinned Chromium oracle.

The runner is deliberately fail-closed: a successful process means every
selected pixel was identical. Partial selections are useful for development,
but their report is marked incomplete and cannot satisfy the contract gate.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import sys
import threading
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageChops


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "docs/renderer/generated/qualification-contract-v1.json"
DEFAULT_IDS = ROOT / "tools/wpt/sp20_focused_ids.json"
TEMPLATES = ROOT / "tools/accountability/data/wpt_ported/all_wpt_templates.json"
ACCOUNTABILITY = ROOT / "tools/accountability"
sys.path.insert(0, str(ACCOUNTABILITY))
import run_all_pixel_comparisons as pixel_runner  # noqa: E402


@dataclass(frozen=True)
class Profile:
    name: str
    logical_width: int
    logical_height: int
    physical_width: int
    physical_height: int
    scale: float


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def bytes_sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_sha256(value: object) -> str:
    return bytes_sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    )


def load_ids(path: Path) -> list[str]:
    values = json.loads(path.read_text(encoding="utf-8"))
    if (
        not isinstance(values, list)
        or any(not isinstance(value, str) or not value for value in values)
        or values != sorted(set(values))
    ):
        raise ValueError(f"invalid sorted unique ID manifest: {path}")
    return values


def is_complete_id_selection(
    contract: dict[str, object], manifest_path: Path, manifest_ids: list[str], ids: list[str]
) -> bool:
    expected_hash = contract["resource_hashes"]["tools/wpt/sp20_focused_ids.json"]
    return (
        len(manifest_ids) == 5731
        and sha256(manifest_path) == expected_hash
        and ids == manifest_ids
    )


def profile_name(width: int, height: int, scale: float) -> str:
    scale_text = f"{scale:g}"
    return f"focused-{width}x{height}@{scale_text}"


def profile_from_entry(entry: dict[str, object], name: str | None = None) -> Profile:
    logical = entry["logical_size_css_px"]
    physical = entry["physical_size_px"]
    assert isinstance(logical, dict) and isinstance(physical, dict)
    width = int(logical["width"])
    height = int(logical["height"])
    scale = float(entry["device_scale"])
    return Profile(
        name=name or str(entry["name"]),
        logical_width=width,
        logical_height=height,
        physical_width=int(physical["width"]),
        physical_height=int(physical["height"]),
        scale=scale,
    )


def contract_profiles(contract: dict[str, object], suite: str) -> list[Profile]:
    if suite == "full":
        entries = contract["qualification_profiles"]
        assert isinstance(entries, list)
        return [profile_from_entry(entry) for entry in entries]
    entries = contract["focused_cross_product"]
    assert isinstance(entries, list)
    result = []
    for entry in entries:
        logical = entry["logical_size_css_px"]
        assert isinstance(logical, dict)
        width = int(logical["width"])
        height = int(logical["height"])
        scale = float(entry["device_scale"])
        result.append(profile_from_entry(entry, profile_name(width, height, scale)))
    return result


def select_profiles(profiles: list[Profile], requested: list[str]) -> list[Profile]:
    if not requested:
        return profiles
    by_name = {profile.name: profile for profile in profiles}
    unknown = sorted(set(requested) - set(by_name))
    if unknown:
        raise ValueError("unknown profile(s): " + ", ".join(unknown))
    if len(requested) != len(set(requested)):
        raise ValueError("profile selection contains duplicates")
    return [by_name[name] for name in requested]


def load_templates() -> dict[str, str]:
    result = dict(pixel_runner.HTML_TEMPLATES)
    result.update(json.loads(TEMPLATES.read_text(encoding="utf-8")))
    return result


def chromium_identity(binary: Path, directory: Path) -> str:
    environment = pixel_runner.chrome_environment(str(directory))
    completed = subprocess.run(
        [binary, "--version"],
        env=environment,
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    match = re.search(r"\b(\d+\.\d+\.\d+\.\d+)\b", completed.stdout)
    if not match:
        raise ValueError(f"could not parse Chromium identity: {completed.stdout!r}")
    return match.group(1)


def image_payload(path: Path) -> tuple[tuple[int, int], bytes]:
    with Image.open(path) as image:
        rgba = image.convert("RGBA")
        return rgba.size, rgba.tobytes()


def compare_images(chromium: Path, openui: Path) -> tuple[int, str, str]:
    with Image.open(chromium) as expected_image, Image.open(openui) as actual_image:
        expected = expected_image.convert("RGBA")
        actual = actual_image.convert("RGBA")
        if expected.size != actual.size:
            return -1, bytes_sha256(expected.tobytes()), bytes_sha256(actual.tobytes())
        difference = ImageChops.difference(expected, actual).tobytes()
        mismatched = sum(
            any(difference[index:index + 4])
            for index in range(0, len(difference), 4)
        )
        return mismatched, bytes_sha256(expected.tobytes()), bytes_sha256(actual.tobytes())


def render_one(
    test_id: str,
    profile: Profile,
    profile_dir: Path,
    templates: dict[str, str],
    chrome_binary: Path,
    chrome_dir: Path,
    text_ids: set[str],
    real_font_ids: set[str],
    freetype_ids: set[str],
    sp18_ids: set[str],
) -> dict[str, object]:
    test_dir = profile_dir / test_id
    test_dir.mkdir(parents=True, exist_ok=True)
    html_path = test_dir / "test.html"
    openui_path = test_dir / "openui.png"
    chromium_path = test_dir / "chromium.png"
    template = re.sub(r"<!\[CDATA\[|\]\]>", "", templates[test_id])
    html_path.write_text(pixel_runner.build_html_document(template), encoding="utf-8")

    use_real_font = test_id in real_font_ids
    use_ahem = test_id in text_ids and not use_real_font
    preserve_subpixel = bool(
        re.search(
            r'<input\b[^>]*\btype\s*=\s*["\']?range\b',
            template,
            re.IGNORECASE,
        )
    )
    common = {
        "logical_width": profile.logical_width,
        "logical_height": profile.logical_height,
        "device_scale": profile.scale,
    }
    if not pixel_runner.render_openui(
        test_id,
        str(openui_path),
        use_ahem_noaa=use_ahem,
        use_real_font=use_real_font,
        preserve_subpixel_positioning=preserve_subpixel,
        **common,
    ):
        return {"id": test_id, "status": "error", "error": "Open UI render failed"}
    if not pixel_runner.render_chrome(
        str(html_path),
        str(chromium_path),
        str(chrome_binary),
        str(chrome_dir),
        use_ahem_noaa=use_ahem,
        use_real_font=use_real_font,
        use_freetype_backend=test_id in freetype_ids,
        use_sp18_features=test_id in sp18_ids,
        **common,
    ):
        return {"id": test_id, "status": "error", "error": "Chromium render failed"}

    openui_size, _ = image_payload(openui_path)
    chromium_size, _ = image_payload(chromium_path)
    expected_size = (profile.physical_width, profile.physical_height)
    if openui_size != expected_size or chromium_size != expected_size:
        return {
            "id": test_id,
            "status": "error",
            "error": "physical surface size mismatch",
            "expected_size": list(expected_size),
            "openui_size": list(openui_size),
            "chromium_size": list(chromium_size),
        }
    mismatched, chromium_rgba, openui_rgba = compare_images(chromium_path, openui_path)
    return {
        "id": test_id,
        "status": "exact" if mismatched == 0 else "different",
        "mismatched_pixels": mismatched,
        "chromium_png_sha256": sha256(chromium_path),
        "openui_png_sha256": sha256(openui_path),
        "chromium_rgba_sha256": chromium_rgba,
        "openui_rgba_sha256": openui_rgba,
        "font_profile": "registered-real-font" if use_real_font else (
            "registered-ahem" if use_ahem else "deterministic-test-collection"
        ),
    }


def run_profile(
    profile: Profile,
    ids: list[str],
    results_dir: Path,
    templates: dict[str, str],
    chrome_binary: Path,
    chrome_dir: Path,
    jobs: int,
) -> dict[str, object]:
    profile_dir = results_dir / profile.name
    profile_dir.mkdir(parents=True, exist_ok=True)
    text_ids = set(load_ids(Path(pixel_runner.TEXT_PORTED_LIST)))
    real_font_ids = set(load_ids(Path(pixel_runner.REAL_FONT_LIST)))
    freetype_ids = set(load_ids(Path(pixel_runner.FREETYPE_TEXT_LIST)))
    sp18_ids = set(load_ids(Path(pixel_runner.SP18_TARGET_LIST)))
    results: list[dict[str, object]] = []
    lock = threading.Lock()
    completed = 0

    def task(test_id: str) -> dict[str, object]:
        return render_one(
            test_id, profile, profile_dir, templates, chrome_binary, chrome_dir,
            text_ids, real_font_ids, freetype_ids, sp18_ids,
        )

    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as executor:
        futures = {executor.submit(task, test_id): test_id for test_id in ids}
        for future in concurrent.futures.as_completed(futures):
            result = future.result()
            results.append(result)
            with lock:
                completed += 1
                if completed % 100 == 0 or completed == len(ids):
                    print(f"{profile.name}: {completed}/{len(ids)}", flush=True)
    results.sort(key=lambda item: str(item["id"]))
    exact = sum(result["status"] == "exact" for result in results)
    different = sum(result["status"] == "different" for result in results)
    errors = len(results) - exact - different
    return {
        "profile": profile.name,
        "logical_size_css_px": [profile.logical_width, profile.logical_height],
        "physical_size_px": [profile.physical_width, profile.physical_height],
        "device_scale": profile.scale,
        "total": len(results),
        "exact": exact,
        "different": different,
        "errors": errors,
        "ordered_id_sha256": bytes_sha256("\n".join(ids).encode("utf-8")),
        "result_sha256": canonical_sha256(results),
        "tests": results,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--suite", choices=("full", "focused"), required=True)
    parser.add_argument("--profile", action="append", default=[])
    parser.add_argument("--ids-file", type=Path, default=DEFAULT_IDS)
    parser.add_argument("--test-id", action="append", default=[])
    parser.add_argument("--results-dir", type=Path, default=ROOT / "out/renderer-qualification")
    parser.add_argument("--pixel-compare", type=Path, default=Path(pixel_runner.PIXEL_COMPARE))
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--plan", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.jobs < 1:
        raise SystemExit("--jobs must be positive")
    contract_bytes = CONTRACT.read_bytes()
    contract = json.loads(contract_bytes)
    all_profiles = contract_profiles(contract, args.suite)
    try:
        profiles = select_profiles(all_profiles, args.profile)
        manifest_ids = load_ids(args.ids_file)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        raise SystemExit(str(error)) from error
    ids = args.test_id or manifest_ids
    if len(ids) != len(set(ids)) or any(test_id not in manifest_ids for test_id in ids):
        raise SystemExit("--test-id values must be unique members of --ids-file")

    plan = {
        "suite": args.suite,
        "profile_count": len(profiles),
        "test_count": len(ids),
        "case_count": len(profiles) * len(ids),
        "profiles": [profile.name for profile in profiles],
        "complete_profile_set": profiles == all_profiles,
        "complete_id_set": is_complete_id_selection(
            contract, args.ids_file, manifest_ids, ids
        ),
    }
    if args.plan:
        print(json.dumps(plan, indent=2, sort_keys=True))
        return

    if not args.pixel_compare.is_file():
        raise SystemExit(f"missing pixel renderer: {args.pixel_compare}")
    pixel_runner.PIXEL_COMPARE = str(args.pixel_compare.resolve())
    if args.chrome is None:
        chrome, chrome_dir = pixel_runner.find_chrome()
        if chrome is None:
            raise SystemExit("pinned Chromium binary was not found")
        chrome_binary = Path(chrome)
        chrome_directory = Path(chrome_dir)
    else:
        chrome_binary = args.chrome.resolve()
        chrome_directory = chrome_binary.parent
    actual_chromium = chromium_identity(chrome_binary, chrome_directory)
    expected_chromium = contract["chromium"]["raster_oracle_build_identity"]
    if actual_chromium != expected_chromium:
        raise SystemExit(
            f"Chromium build mismatch: expected {expected_chromium}, got {actual_chromium}"
        )

    registered = set(
        subprocess.run(
            [args.pixel_compare, "list"], check=True, capture_output=True, text=True,
        ).stdout.splitlines()
    )
    unknown = sorted(set(ids) - registered)
    if unknown:
        raise SystemExit(f"ID manifest contains {len(unknown)} unregistered tests")
    templates = load_templates()
    missing_templates = sorted(set(ids) - set(templates))
    if missing_templates:
        raise SystemExit(f"ID manifest contains {len(missing_templates)} tests without templates")

    args.results_dir.mkdir(parents=True, exist_ok=True)
    summaries = [
        run_profile(
            profile, ids, args.results_dir, templates, chrome_binary,
            chrome_directory, args.jobs,
        )
        for profile in profiles
    ]
    exact = sum(int(summary["exact"]) for summary in summaries)
    different = sum(int(summary["different"]) for summary in summaries)
    errors = sum(int(summary["errors"]) for summary in summaries)
    complete = plan["complete_profile_set"] and plan["complete_id_set"]
    report = {
        "schema_version": 1,
        "contract_sha256": bytes_sha256(contract_bytes),
        "suite": args.suite,
        "complete_contract_scope": complete,
        "chromium": {
            "build_identity": actual_chromium,
            "binary_sha256": sha256(chrome_binary),
        },
        "openui": {"binary_sha256": sha256(args.pixel_compare)},
        "id_manifest": {
            "path": str(args.ids_file.resolve()),
            "sha256": sha256(args.ids_file),
            "count": len(ids),
        },
        "resource_hashes": contract["resource_hashes"],
        "font_byte_hashes": contract["font_byte_hashes"],
        "raster": contract["raster"],
        "results": {
            "total": sum(int(summary["total"]) for summary in summaries),
            "exact": exact,
            "different": different,
            "errors": errors,
            "profile_result_sha256": canonical_sha256(
                [summary["result_sha256"] for summary in summaries]
            ),
        },
        "profiles": summaries,
    }
    report_path = args.results_dir / f"{args.suite}-summary.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"matrix report: {report_path}")
    print(f"exact={exact} different={different} errors={errors}")
    if different or errors:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
