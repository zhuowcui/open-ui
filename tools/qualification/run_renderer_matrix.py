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
import shutil
import subprocess
import sys
import tempfile
import threading
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageChops


ROOT = Path(__file__).resolve().parents[2]
QUALIFICATION_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(QUALIFICATION_DIR))
import residuals  # noqa: E402

CONTRACT = ROOT / "docs/renderer/generated/qualification-contract-v2.json"
FULL_IDS = ROOT / "tools/qualification/manifests/complete-5731.json"
FOCUSED_IDS = ROOT / "tools/qualification/manifests/focused-raster.json"
PRIMITIVE_IDS = ROOT / "tools/qualification/manifests/primitive-raster.json"
EXPANDED_IDS = ROOT / "tools/qualification/manifests/expanded-v1.json"
RESIDUAL_OWNERSHIP = ROOT / "tools/qualification/residual-ownership-v2.json"
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


def chromium_capture_harness_sha256() -> str:
    """Hash only code which can affect Chromium's captured pixels."""
    path = ROOT / "tools/accountability/run_all_pixel_comparisons.py"
    relative = str(path.relative_to(ROOT)).encode("utf-8")
    content = path.read_bytes()
    digest = hashlib.sha256()
    digest.update(len(relative).to_bytes(8, "big"))
    digest.update(relative)
    digest.update(len(content).to_bytes(8, "big"))
    digest.update(content)
    return digest.hexdigest()


def load_ids(path: Path) -> list[str]:
    values = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(values, dict) and values.get("schema_version") == 1:
        base_path = (path.parent / values["base_manifest"]).resolve()
        if sha256(base_path) != values.get("base_manifest_sha256"):
            raise ValueError(f"expanded manifest base hash changed: {path}")
        base = load_ids(base_path)
        additions = values.get("additions")
        if not isinstance(additions, list):
            raise ValueError(f"expanded manifest additions must be a list: {path}")
        values = sorted(set(base) | set(additions))
    if (
        not isinstance(values, list)
        or any(not isinstance(value, str) or not value for value in values)
        or values != sorted(set(values))
    ):
        raise ValueError(f"invalid sorted unique ID manifest: {path}")
    return values


def suite_manifest_path(suite: str) -> Path | None:
    return {
        "full": FULL_IDS,
        "focused": FOCUSED_IDS,
        "primitive": PRIMITIVE_IDS,
        "expanded": EXPANDED_IDS,
    }.get(suite)


def is_complete_id_selection(
    contract: dict[str, object], suite: str, manifest_path: Path,
    manifest_ids: list[str], ids: list[str]
) -> bool:
    if suite not in contract["suite_manifests"]:
        return False
    expected = contract["suite_manifests"][suite]
    return (
        len(manifest_ids) == int(expected["count"])
        and sha256(manifest_path) == expected["sha256"]
        and ids == manifest_ids
    )


def select_shard(ids: list[str], index: int, count: int) -> list[str]:
    if count < 1 or index < 0 or index >= count:
        raise ValueError("shard index must be in the range [0, shard count)")
    return [test_id for position, test_id in enumerate(ids) if position % count == index]


def repository_commit() -> str:
    return subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
        capture_output=True, text=True,
    ).stdout.strip()


def repository_source_identity() -> dict[str, object]:
    status = subprocess.run(
        ["git", "status", "--porcelain=v1", "-z", "--untracked-files=all"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    ).stdout
    paths = subprocess.run(
        ["git", "ls-files", "-co", "--exclude-standard", "-z"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    ).stdout.split(b"\0")
    digest = hashlib.sha256()
    for encoded_path in sorted(path for path in paths if path):
        path = ROOT / os.fsdecode(encoded_path)
        digest.update(len(encoded_path).to_bytes(8, "big"))
        digest.update(encoded_path)
        if not path.exists() and not path.is_symlink():
            content = b"<deleted>"
        elif path.is_symlink():
            content = os.readlink(path).encode("utf-8", "surrogateescape")
        else:
            content = path.read_bytes()
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    harness_paths = [
        *sorted(QUALIFICATION_DIR.glob("*.py")),
        ROOT / "tools/accountability/run_all_pixel_comparisons.py",
        ROOT / "bindings/rust/pixel-compare/Cargo.toml",
        ROOT / "bindings/rust/pixel-compare/src/main.rs",
        ROOT / "bindings/rust/pixel-compare/src/wpt/mod.rs",
    ]
    harness = hashlib.sha256()
    for path in harness_paths:
        relative = str(path.relative_to(ROOT)).encode("utf-8")
        content = path.read_bytes()
        harness.update(len(relative).to_bytes(8, "big"))
        harness.update(relative)
        harness.update(len(content).to_bytes(8, "big"))
        harness.update(content)
    return {
        "commit": repository_commit(),
        "clean": not status,
        "status_sha256": bytes_sha256(status),
        "source_tree_sha256": digest.hexdigest(),
        "harness_sha256": harness.hexdigest(),
    }


def raster_configuration_name(
    use_ahem: bool, use_real_font: bool, preserve_subpixel: bool,
    raster_backend: str = "cpu-skia",
) -> str:
    if use_real_font:
        text = "chromium-linux-lcd"
    elif use_ahem:
        text = "deterministic-alias-subpixel" if preserve_subpixel else "deterministic-alias"
    else:
        text = "default"
    return f"{raster_backend}:{text}"


def cached_result(cache_dir: Path, identity: dict[str, object]) -> dict[str, object] | None:
    key = canonical_sha256(identity)
    path = cache_dir / key[:2] / f"{key}.json"
    if not path.is_file():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"invalid renderer cache entry: {path}") from error
    if payload.get("schema_version") != 2 or payload.get("identity") != identity:
        raise ValueError(f"stale or cross-profile renderer cache entry: {path}")
    result = payload.get("result")
    if not isinstance(result, dict):
        raise ValueError(f"renderer cache entry has no result: {path}")
    return result


def store_cached_result(
    cache_dir: Path, identity: dict[str, object], result: dict[str, object]
) -> None:
    key = canonical_sha256(identity)
    directory = cache_dir / key[:2]
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / f"{key}.json"
    temporary = directory / f".{key}.{threading.get_ident()}.tmp"
    temporary.write_text(
        json.dumps(
            {"schema_version": 2, "identity": identity, "result": result},
            indent=2, sort_keys=True,
        ) + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def chromium_oracle_identity(
    cache_base: dict[str, object],
    test_id: str,
    fixture_sha256: str,
    profile: Profile,
    font_profile: str,
    use_freetype_backend: bool,
    use_sp18_features: bool,
) -> dict[str, object]:
    """Return the browser-only identity for one immutable oracle capture.

    OpenUI commit, source, binary, raster backend, and raster configuration are
    intentionally absent. Changing the implementation under test must never
    cause Chromium's expected pixels to be recaptured.
    """
    return {
        "schema_version": 1,
        "chromium_binary_sha256": cache_base["chromium_binary_sha256"],
        "chromium_build_identity": cache_base["chromium_build_identity"],
        "chromium_capture_harness_sha256": cache_base[
            "chromium_capture_harness_sha256"
        ],
        "contract_sha256": cache_base["contract_sha256"],
        "test_id": test_id,
        "fixture_sha256": fixture_sha256,
        "profile": {
            "logical_width": profile.logical_width,
            "logical_height": profile.logical_height,
            "physical_width": profile.physical_width,
            "physical_height": profile.physical_height,
            "device_scale": profile.scale,
        },
        "font_profile": font_profile,
        "chromium_freetype_backend": use_freetype_backend,
        "chromium_sp18_features": use_sp18_features,
        "resource_hashes": cache_base["resource_hashes"],
        "font_byte_hashes": cache_base["font_byte_hashes"],
    }


def chromium_oracle_entry_path(
    oracle_cache_dir: Path, identity: dict[str, object]
) -> Path:
    key = canonical_sha256(identity)
    return oracle_cache_dir / key[:2] / f"{key}.json"


def cached_chromium_oracle(
    oracle_cache_dir: Path,
    content_cache_dir: Path,
    identity: dict[str, object],
    destination: Path,
) -> dict[str, object] | None:
    path = chromium_oracle_entry_path(oracle_cache_dir, identity)
    if not path.is_file():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"invalid Chromium oracle cache entry: {path}") from error
    if payload.get("schema_version") != 1 or payload.get("identity") != identity:
        raise ValueError(f"stale or cross-profile Chromium oracle entry: {path}")
    result = payload.get("result")
    if not isinstance(result, dict):
        raise ValueError(f"Chromium oracle entry has no result: {path}")
    relative = result.get("png_cache_path")
    expected_sha256 = result.get("png_sha256")
    if not isinstance(relative, str) or not isinstance(expected_sha256, str):
        raise ValueError(f"Chromium oracle entry is incomplete: {path}")
    source = content_cache_dir / relative
    if not source.is_file() or sha256(source) != expected_sha256:
        raise ValueError(f"Chromium oracle PNG is missing or corrupt: {source}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    return result


def store_chromium_oracle(
    oracle_cache_dir: Path,
    content_cache_dir: Path,
    identity: dict[str, object],
    source: Path,
) -> dict[str, object]:
    png_sha256 = sha256(source)
    png_cache_path = store_content_addressed_png(
        content_cache_dir, source, png_sha256
    )
    size, rgba = image_payload(source)
    result = {
        "png_sha256": png_sha256,
        "png_cache_path": png_cache_path,
        "rgba_sha256": bytes_sha256(rgba),
        "physical_size_px": list(size),
    }
    path = chromium_oracle_entry_path(oracle_cache_dir, identity)
    if path.is_file():
        # Oracle entries are write-once. A second producer is accepted only
        # when it independently produced the same decoded pixels.
        with tempfile.TemporaryDirectory() as directory:
            existing_path = Path(directory) / "chromium.png"
            existing = cached_chromium_oracle(
                oracle_cache_dir, content_cache_dir, identity, existing_path
            )
        if (
            existing.get("rgba_sha256") != result["rgba_sha256"]
            or existing.get("physical_size_px") != result["physical_size_px"]
        ):
            raise ValueError(
                "non-deterministic Chromium capture for immutable oracle "
                f"{canonical_sha256(identity)}"
            )
        return existing
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{threading.get_ident()}.tmp")
    temporary.write_text(
        json.dumps(
            {"schema_version": 1, "identity": identity, "result": result},
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    # Do not replace an oracle another worker may have installed while this
    # capture was running. Re-open it and enforce identical content instead.
    try:
        os.link(temporary, path)
        temporary.unlink()
    except FileExistsError:
        temporary.unlink(missing_ok=True)
        with tempfile.TemporaryDirectory() as directory:
            existing_path = Path(directory) / "chromium.png"
            existing = cached_chromium_oracle(
                oracle_cache_dir, content_cache_dir, identity, existing_path
            )
        if (
            existing.get("rgba_sha256") != result["rgba_sha256"]
            or existing.get("physical_size_px") != result["physical_size_px"]
        ):
            raise ValueError(
                "non-deterministic Chromium capture for immutable oracle "
                f"{canonical_sha256(identity)}"
            )
        return existing
    return result


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
    if suite in {"full", "expanded"}:
        entries = contract["qualification_profiles"]
        assert isinstance(entries, list)
        return [profile_from_entry(entry) for entry in entries]
    if suite == "residual":
        residual_contract = contract["residuals"]
        scale_sweep = residual_contract["scale_sweep"]
        viewport_sweep = residual_contract["viewport_sweep"]
        scale_size = scale_sweep["logical_size_css_px"]
        entries = [
            {
                "logical_size_css_px": scale_size,
                "physical_size_px": {
                    "width": int(float(scale_size["width"]) * float(scale) + 0.5),
                    "height": int(float(scale_size["height"]) * float(scale) + 0.5),
                },
                "device_scale": scale,
            }
            for scale in scale_sweep["device_scales"]
        ]
        entries.extend(
            {
                "logical_size_css_px": size,
                "physical_size_px": {
                    "width": int(float(size["width"]) + 0.5),
                    "height": int(float(size["height"]) + 0.5),
                },
                "device_scale": 1.0,
            }
            for size in viewport_sweep["logical_sizes_css_px"]
            if size != scale_size
        )
    else:
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
        expected_bytes = expected.tobytes()
        actual_bytes = actual.tobytes()
        expected_hash = bytes_sha256(expected_bytes)
        actual_hash = bytes_sha256(actual_bytes)
        if expected.size != actual.size:
            return -1, expected_hash, actual_hash
        if expected_bytes == actual_bytes:
            return 0, expected_hash, actual_hash
        difference = ImageChops.difference(expected, actual)
        mask = difference.getchannel("R")
        for channel in ("G", "B", "A"):
            mask = ImageChops.lighter(mask, difference.getchannel(channel))
        # Pillow counts every nonzero channel maximum in native code. This is
        # still an exact, whole-image RGBA comparison with no tolerance.
        mismatched = sum(mask.histogram()[1:])
        return mismatched, expected_hash, actual_hash


def store_content_addressed_png(cache_dir: Path, source: Path, digest: str) -> str:
    relative = Path("png") / digest[:2] / f"{digest}.png"
    destination = cache_dir / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    if not destination.is_file():
        temporary = destination.with_name(
            f".{destination.name}.{threading.get_ident()}.tmp"
        )
        shutil.copyfile(source, temporary)
        temporary.replace(destination)
    elif sha256(destination) != digest:
        raise ValueError(f"content-addressed PNG hash collision: {destination}")
    return str(relative)


def renderer_backend_identity(binary: Path, backend: str) -> dict[str, object]:
    completed = subprocess.run(
        [binary, "raster-identity", "--backend", backend],
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )
    identity = json.loads(completed.stdout)
    if identity.get("backend") != backend:
        raise ValueError("renderer returned a cross-backend raster identity")
    return identity


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
    cache_dir: Path,
    oracle_cache_dir: Path,
    cache_base: dict[str, object],
    raster_backend: str,
) -> dict[str, object]:
    test_dir = profile_dir / test_id
    test_dir.mkdir(parents=True, exist_ok=True)
    html_path = test_dir / "test.html"
    openui_path = test_dir / "openui.png"
    chromium_path = test_dir / "chromium.png"
    template = re.sub(r"<!\[CDATA\[|\]\]>", "", templates[test_id])
    html_document = pixel_runner.build_html_document(template)
    html_path.write_text(html_document, encoding="utf-8")

    use_real_font = test_id in real_font_ids
    use_ahem = test_id in text_ids and not use_real_font
    preserve_subpixel = bool(
        re.search(
            r'<input\b[^>]*\btype\s*=\s*["\']?range\b',
            template,
            re.IGNORECASE,
        )
    )
    raster_configuration = raster_configuration_name(
        use_ahem, use_real_font, preserve_subpixel, raster_backend
    )
    fixture_sha256 = bytes_sha256(html_document.encode("utf-8"))
    font_profile = "registered-real-font" if use_real_font else (
        "registered-ahem" if use_ahem else "deterministic-test-collection"
    )
    use_freetype_backend = test_id in freetype_ids
    use_sp18_features = test_id in sp18_ids
    oracle_identity = chromium_oracle_identity(
        cache_base,
        test_id,
        fixture_sha256,
        profile,
        font_profile,
        use_freetype_backend,
        use_sp18_features,
    )
    oracle_identity_sha256 = canonical_sha256(oracle_identity)
    identity = {
        **cache_base,
        "test_id": test_id,
        "fixture_sha256": fixture_sha256,
        "profile": {
            "logical_width": profile.logical_width,
            "logical_height": profile.logical_height,
            "physical_width": profile.physical_width,
            "physical_height": profile.physical_height,
            "device_scale": profile.scale,
        },
        "raster_configuration": raster_configuration,
        "font_profile": font_profile,
        "chromium_freetype_backend": use_freetype_backend,
        "chromium_sp18_features": use_sp18_features,
        "chromium_oracle_identity_sha256": oracle_identity_sha256,
    }
    hit = cached_result(cache_dir, identity)
    if hit is not None:
        return hit
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
        raster_backend=raster_backend,
        **common,
    ):
        return {"id": test_id, "status": "error", "error": "Open UI render failed"}
    oracle_result = cached_chromium_oracle(
        oracle_cache_dir, cache_dir, oracle_identity, chromium_path
    )
    oracle_source = "cache"
    if oracle_result is None:
        oracle_source = "capture"
        chrome_rendered = any(
            pixel_runner.render_chrome(
                str(html_path),
                str(chromium_path),
                str(chrome_binary),
                str(chrome_dir),
                use_ahem_noaa=use_ahem,
                use_real_font=use_real_font,
                use_freetype_backend=use_freetype_backend,
                use_sp18_features=use_sp18_features,
                **common,
            )
            for _attempt in range(3)
        )
        if not chrome_rendered:
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
    if oracle_result is None:
        oracle_result = store_chromium_oracle(
            oracle_cache_dir, cache_dir, oracle_identity, chromium_path
        )
    mismatched, chromium_rgba, openui_rgba = compare_images(chromium_path, openui_path)
    diff_signature = residuals.analyze_image_difference(chromium_path, openui_path)
    if diff_signature.get("mismatched_pixels") != mismatched:
        raise ValueError(f"inconsistent diff analysis for {test_id}")
    chromium_png = sha256(chromium_path)
    openui_png = sha256(openui_path)
    result = {
        "id": test_id,
        "status": "exact" if mismatched == 0 else "different",
        "mismatched_pixels": mismatched,
        "chromium_png_sha256": chromium_png,
        "openui_png_sha256": openui_png,
        "chromium_png_cache_path": store_content_addressed_png(
            cache_dir, chromium_path, chromium_png
        ),
        "openui_png_cache_path": store_content_addressed_png(
            cache_dir, openui_path, openui_png
        ),
        "chromium_rgba_sha256": chromium_rgba,
        "openui_rgba_sha256": openui_rgba,
        "font_profile": font_profile,
        "raster_configuration": raster_configuration,
        "chromium_oracle_identity_sha256": oracle_identity_sha256,
        "chromium_oracle_source": oracle_source,
        "chromium_oracle_rgba_sha256": oracle_result["rgba_sha256"],
        "diff_signature": diff_signature,
        "cache_identity_sha256": canonical_sha256(identity),
    }
    store_cached_result(cache_dir, identity, result)
    return result


def run_profile(
    profile: Profile,
    ids: list[str],
    results_dir: Path,
    templates: dict[str, str],
    chrome_binary: Path,
    chrome_dir: Path,
    jobs: int,
    cache_dir: Path,
    oracle_cache_dir: Path,
    cache_base: dict[str, object],
    raster_backend: str,
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
            text_ids, real_font_ids, freetype_ids, sp18_ids, cache_dir,
            oracle_cache_dir, cache_base, raster_backend,
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
    parser.add_argument(
        "--suite",
        choices=("full", "expanded", "focused", "primitive", "residual", "residual-cross"),
        required=True,
    )
    parser.add_argument("--profile", action="append", default=[])
    parser.add_argument("--ids-file", type=Path)
    parser.add_argument("--test-id", action="append", default=[])
    parser.add_argument("--results-dir", type=Path, default=ROOT / "out/renderer-qualification")
    parser.add_argument("--cache-dir", type=Path, default=ROOT / "out/renderer-qualification-cache")
    parser.add_argument(
        "--oracle-cache-dir",
        type=Path,
        help="immutable Chromium-only cache (defaults to CACHE_DIR/chromium-oracle)",
    )
    parser.add_argument("--residual-ownership", type=Path, default=RESIDUAL_OWNERSHIP)
    parser.add_argument("--pixel-compare", type=Path, default=Path(pixel_runner.PIXEL_COMPARE))
    parser.add_argument("--chrome", type=Path)
    parser.add_argument("--raster-backend", choices=("cpu-skia", "ganesh-gl"), default="cpu-skia")
    parser.add_argument("--allow-dirty-diagnostics", action="store_true")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--shard-index", type=int, default=0)
    parser.add_argument("--shard-count", type=int, default=1)
    parser.add_argument("--plan", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.jobs < 1:
        raise SystemExit("--jobs must be positive")
    try:
        select_shard([], args.shard_index, args.shard_count)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    if args.ids_file is None:
        args.ids_file = suite_manifest_path(args.suite)
    if args.ids_file is None:
        raise SystemExit(f"--ids-file is required for the {args.suite} diagnostic suite")
    contract_bytes = CONTRACT.read_bytes()
    contract = json.loads(contract_bytes)
    if contract.get("schema_version") != 2:
        raise SystemExit("renderer qualification contract must use schema v2")
    source_identity = repository_source_identity()
    all_profiles = contract_profiles(contract, args.suite)
    try:
        profiles = select_profiles(all_profiles, args.profile)
        manifest_ids = load_ids(args.ids_file)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        raise SystemExit(str(error)) from error
    requested_ids = args.test_id or manifest_ids
    ids = select_shard(requested_ids, args.shard_index, args.shard_count)
    if len(ids) != len(set(ids)) or any(test_id not in manifest_ids for test_id in ids):
        raise SystemExit("--test-id values must be unique members of --ids-file")

    plan = {
        "suite": args.suite,
        "profile_count": len(profiles),
        "test_count": len(ids),
        "case_count": len(profiles) * len(ids),
        "profiles": [profile.name for profile in profiles],
        "manifest": str(args.ids_file),
        "shard": {"index": args.shard_index, "count": args.shard_count},
        "complete_profile_set": profiles == all_profiles,
        "complete_id_set": is_complete_id_selection(
            contract, args.suite, args.ids_file, manifest_ids, ids
        ),
        "raster_backend": args.raster_backend,
        "source": source_identity,
    }
    if args.plan:
        print(json.dumps(plan, indent=2, sort_keys=True))
        return

    if not source_identity["clean"] and not args.allow_dirty_diagnostics:
        raise SystemExit(
            "authoritative renderer qualification requires a clean worktree; "
            "use --allow-dirty-diagnostics only for nonqualifying investigation"
        )

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
    openui_binary_sha256 = sha256(args.pixel_compare)
    chromium_binary_sha256 = sha256(chrome_binary)
    backend_identity = renderer_backend_identity(args.pixel_compare, args.raster_backend)
    oracle_cache_dir = args.oracle_cache_dir or args.cache_dir / "chromium-oracle"
    commit = str(source_identity["commit"])
    cache_base = {
        "schema_version": 3,
        "commit": commit,
        "source_tree_sha256": source_identity["source_tree_sha256"],
        "harness_sha256": source_identity["harness_sha256"],
        "contract_sha256": bytes_sha256(contract_bytes),
        "openui_binary_sha256": openui_binary_sha256,
        "chromium_binary_sha256": chromium_binary_sha256,
        "chromium_build_identity": actual_chromium,
        "chromium_capture_harness_sha256": chromium_capture_harness_sha256(),
        "resource_hashes": contract["resource_hashes"],
        "font_byte_hashes": contract["font_byte_hashes"],
        "raster_backend_identity": backend_identity,
    }

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
            chrome_directory, args.jobs, args.cache_dir, oracle_cache_dir,
            cache_base, args.raster_backend,
        )
        for profile in profiles
    ]
    exact = sum(int(summary["exact"]) for summary in summaries)
    different = sum(int(summary["different"]) for summary in summaries)
    errors = sum(int(summary["errors"]) for summary in summaries)
    complete = plan["complete_profile_set"] and plan["complete_id_set"]
    qualified = (
        bool(source_identity["clean"])
        and complete
        and args.raster_backend == contract["raster"]["qualification_backend"]
        and different == 0
        and errors == 0
    )
    report = {
        "schema_version": 2,
        "contract_sha256": bytes_sha256(contract_bytes),
        "commit": commit,
        "suite": args.suite,
        "complete_contract_scope": complete,
        "evidence": {
            "kind": "qualification" if qualified else "diagnostic-census",
            "qualified": qualified,
            "tolerance_pixels": 0,
            "qualification_backend_match": (
                args.raster_backend == contract["raster"]["qualification_backend"]
            ),
        },
        "source": source_identity,
        "shard": {"index": args.shard_index, "count": args.shard_count},
        "chromium": {
            "build_identity": actual_chromium,
            "binary_sha256": chromium_binary_sha256,
            "capture_harness_sha256": cache_base[
                "chromium_capture_harness_sha256"
            ],
            "oracle_cache": str(oracle_cache_dir.resolve()),
        },
        "openui": {
            "binary_sha256": openui_binary_sha256,
            "raster_backend_identity": backend_identity,
        },
        "id_manifest": {
            "path": str(args.ids_file.resolve()),
            "sha256": sha256(args.ids_file),
            "count": len(ids),
        },
        "resource_hashes": contract["resource_hashes"],
        "font_byte_hashes": contract["font_byte_hashes"],
        "raster": contract["raster"],
        "manifest_scope": {
            "original_count": contract["suite_manifests"]["full"]["count"],
            "expanded_count": contract["suite_manifests"]["expanded"]["count"],
            "admitted_candidate_count": contract["suite_manifests"]["expanded"][
                "admitted_candidate_count"
            ],
            "reported_suite": args.suite,
        },
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
    if args.suite in {"full", "expanded"}:
        ownership, ownership_sha256 = residuals.load_ownership(args.residual_ownership)
        try:
            report["residual_ledger"] = residuals.residual_ledger(
                summaries, ownership, ownership_sha256
            )
        except ValueError as error:
            # Diagnostic censuses must retain their complete pixel evidence so
            # reviewers can assign ownership.  Unknown ownership still fails
            # qualification, but it no longer discards the report needed to
            # create the reviewed ledger.
            report["residual_ledger"] = {
                "schema_version": 2,
                "qualifying": False,
                "ownership_input_sha256": ownership_sha256,
                "error": str(error),
            }
            report["evidence"]["qualified"] = False
    suffix = "" if args.shard_count == 1 else f"-shard-{args.shard_index}-of-{args.shard_count}"
    report_path = args.results_dir / f"{args.suite}{suffix}-summary.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"matrix report: {report_path}")
    print(f"exact={exact} different={different} errors={errors}")
    if errors or different or report.get("residual_ledger", {}).get("error"):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
