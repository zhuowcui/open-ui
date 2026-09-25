#!/usr/bin/env python3
"""Evidence-derived renderer residual analysis.

This module deliberately does not inspect test names or HTML.  Pixel evidence
describes *what* differs; the reviewed ownership ledger records *why* it
differs.  Keeping those inputs separate prevents a CSS keyword from silently
becoming a root-cause decision.
"""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Iterable

from PIL import Image, ImageChops


SCHEMA_VERSION = 2
ROOT = Path(__file__).resolve().parents[2]
ROOT_CAUSES = (
    "physical-snapping",
    "analytic-coverage",
    "color-quantization",
    "glyph-strike-origin",
    "font-selection-shaping",
    "clipping-overflow",
    "image-sampling",
    "fragmentation-layout",
    "controls",
    "viewport-logic",
    "external-backend-discrepancy",
)
OWNING_SUBSYSTEMS = (
    "geometry",
    "layout",
    "paint",
    "compositor",
    "text",
    "platform",
    "external",
)


def canonical_sha256(value: object) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def _bounds(left: int, top: int, right: int, bottom: int) -> dict[str, int]:
    return {
        "x": left,
        "y": top,
        "width": right - left,
        "height": bottom - top,
    }


def _connected_regions(mask: bytearray, width: int, height: int) -> list[dict[str, object]]:
    """Return deterministic 8-connected run components without image libraries."""

    parents: list[int] = []
    ranks: list[int] = []
    regions: list[list[int]] = []  # left, top, right, bottom, pixels

    def make(left: int, right: int, y: int) -> int:
        label = len(parents)
        parents.append(label)
        ranks.append(0)
        regions.append([left, y, right, y + 1, right - left])
        return label

    def find(label: int) -> int:
        while parents[label] != label:
            parents[label] = parents[parents[label]]
            label = parents[label]
        return label

    def union(first: int, second: int) -> int:
        first = find(first)
        second = find(second)
        if first == second:
            return first
        if ranks[first] < ranks[second]:
            first, second = second, first
        parents[second] = first
        if ranks[first] == ranks[second]:
            ranks[first] += 1
        a = regions[first]
        b = regions[second]
        a[0] = min(a[0], b[0])
        a[1] = min(a[1], b[1])
        a[2] = max(a[2], b[2])
        a[3] = max(a[3], b[3])
        a[4] += b[4]
        b[4] = 0
        return first

    previous: list[tuple[int, int, int]] = []
    for y in range(height):
        row = y * width
        current: list[tuple[int, int, int]] = []
        row_end = row + width
        first = mask.find(1, row, row_end)
        while first != -1:
            end = mask.find(0, first, row_end)
            if end == -1:
                end = row_end
            left = first - row
            right = end - row
            label = make(left, right, y)
            for prior_left, prior_right, prior_label in previous:
                if prior_right < left:
                    continue
                if prior_left > right:
                    break
                label = union(label, prior_label)
            current.append((left, right, label))
            first = mask.find(1, end, row_end)
        previous = current

    result = []
    for label, region in enumerate(regions):
        if find(label) != label or region[4] == 0:
            continue
        result.append(
            {
                "bounds": _bounds(region[0], region[1], region[2], region[3]),
                "mismatched_pixels": region[4],
            }
        )
    result.sort(
        key=lambda item: (
            -int(item["mismatched_pixels"]),
            int(item["bounds"]["y"]),
            int(item["bounds"]["x"]),
        )
    )
    return result


def analyze_image_difference(expected_path: Path, actual_path: Path) -> dict[str, object]:
    """Compute exact decoded-RGBA geometry and channel statistics."""

    with Image.open(expected_path) as expected_image, Image.open(actual_path) as actual_image:
        expected = expected_image.convert("RGBA")
        actual = actual_image.convert("RGBA")
        if expected.size != actual.size:
            return {
                "comparable": False,
                "expected_size": list(expected.size),
                "actual_size": list(actual.size),
            }
        width, height = expected.size
        expected_bytes = expected.tobytes()
        actual_bytes = actual.tobytes()

    channel_names = ("red", "green", "blue", "alpha")
    channel_stats = {
        name: {
            "changed_pixels": 0,
            "minimum_signed_delta": 0,
            "maximum_signed_delta": 0,
            "maximum_absolute_delta": 0,
            "signed_delta_sum": 0,
        }
        for name in channel_names
    }
    if expected_bytes == actual_bytes:
        # The common exact case needs no per-channel Python scan. Compare
        # decoded pixels, not PNG files, so distinct lossless encodings still
        # yield the same zero-difference signature.
        return {
            "comparable": True,
            "mismatched_pixels": 0,
            "mismatch_bounds": None,
            "connected_region_count": 0,
            "connected_regions": [],
            "channel_deltas": channel_stats,
        }
    difference = ImageChops.difference(expected, actual)
    difference_mask = difference.getchannel("R")
    for channel in ("G", "B", "A"):
        difference_mask = ImageChops.lighter(
            difference_mask, difference.getchannel(channel)
        )
    # Pillow identifies differing pixels in native code. Keep the signed
    # channel accounting below on only those pixels, and retain 0/1 bytes for
    # the deterministic run-component algorithm.
    mask = bytearray(difference_mask.point(lambda value: int(value != 0)).tobytes())
    mismatch_count = 0
    left, top, right, bottom = width, height, 0, 0
    pixel = mask.find(1)
    while pixel != -1:
        offset = pixel * 4
        for channel, name in enumerate(channel_names):
            delta = actual_bytes[offset + channel] - expected_bytes[offset + channel]
            if delta:
                stats = channel_stats[name]
                stats["changed_pixels"] += 1
                stats["minimum_signed_delta"] = min(stats["minimum_signed_delta"], delta)
                stats["maximum_signed_delta"] = max(stats["maximum_signed_delta"], delta)
                stats["maximum_absolute_delta"] = max(
                    stats["maximum_absolute_delta"], abs(delta)
                )
                stats["signed_delta_sum"] += delta
        x = pixel % width
        y = pixel // width
        mismatch_count += 1
        left = min(left, x)
        top = min(top, y)
        right = max(right, x + 1)
        bottom = max(bottom, y + 1)
        pixel = mask.find(1, pixel + 1)

    regions = [] if mismatch_count == 0 else _connected_regions(mask, width, height)
    return {
        "comparable": True,
        "mismatched_pixels": mismatch_count,
        "mismatch_bounds": (
            None if mismatch_count == 0 else _bounds(left, top, right, bottom)
        ),
        "connected_region_count": len(regions),
        "connected_regions": regions,
        "channel_deltas": channel_stats,
    }


def load_ownership(path: Path) -> tuple[dict[str, dict[str, object]], str]:
    raw = path.read_bytes()
    value = json.loads(raw)
    if value.get("schema_version") != SCHEMA_VERSION:
        raise ValueError(f"residual ownership must use schema v{SCHEMA_VERSION}: {path}")
    entries = value.get("entries")
    if not isinstance(entries, list):
        raise ValueError(f"residual ownership entries must be a list: {path}")
    result: dict[str, dict[str, object]] = {}
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError(f"invalid residual ownership entry: {path}")
        test_id = entry.get("test_id")
        cause = entry.get("root_cause")
        owner = entry.get("owning_subsystem")
        reproducer = entry.get("minimized_reproducer")
        if not isinstance(test_id, str) or not test_id or test_id in result:
            raise ValueError(f"duplicate or invalid residual test identity: {test_id!r}")
        if cause not in ROOT_CAUSES:
            raise ValueError(f"unknown residual root cause for {test_id}: {cause!r}")
        if owner not in OWNING_SUBSYSTEMS:
            raise ValueError(f"unknown owning subsystem for {test_id}: {owner!r}")
        if not isinstance(reproducer, dict) or not {
            "kind", "identity", "sha256"
        } <= reproducer.keys():
            raise ValueError(f"missing minimized reproducer for {test_id}")
        identity = reproducer["identity"]
        expected_hash = reproducer["sha256"]
        if (
            not isinstance(identity, str)
            or not identity
            or not isinstance(expected_hash, str)
            or re.fullmatch(r"[0-9a-f]{64}", expected_hash) is None
        ):
            raise ValueError(f"invalid minimized reproducer identity for {test_id}")
        if "://" not in identity:
            reproducer_path = (ROOT / identity).resolve()
            try:
                reproducer_path.relative_to(ROOT)
            except ValueError as error:
                raise ValueError(f"reproducer escapes repository for {test_id}") from error
            if not reproducer_path.is_file():
                raise ValueError(f"missing minimized reproducer for {test_id}: {identity}")
            actual_hash = hashlib.sha256(reproducer_path.read_bytes()).hexdigest()
            if actual_hash != expected_hash:
                raise ValueError(f"minimized reproducer hash changed for {test_id}")
        result[test_id] = entry
    return result, hashlib.sha256(raw).hexdigest()


def _profile_behavior(
    test_id: str, summaries: Iterable[dict[str, object]]
) -> dict[str, object]:
    observations = []
    for summary in summaries:
        result = next(
            (item for item in summary["tests"] if item["id"] == test_id), None
        )
        if result is None:
            continue
        logical = summary["logical_size_css_px"]
        observations.append(
            {
                "profile": summary["profile"],
                "logical_size_css_px": logical,
                "device_scale": summary["device_scale"],
                "status": result["status"],
                "mismatched_pixels": result.get("mismatched_pixels"),
            }
        )
    differing = [item for item in observations if item["status"] != "exact"]
    differing_scales = sorted({float(item["device_scale"]) for item in differing})
    differing_viewports = sorted(
        {tuple(item["logical_size_css_px"]) for item in differing}
    )
    return {
        "observations": observations,
        "only_fractional_scales": bool(differing)
        and all(not float(item["device_scale"]).is_integer() for item in differing),
        "scale_sensitive": len(differing_scales) > 1,
        "viewport_sensitive": len(differing_viewports) > 1,
    }


def residual_ledger(
    summaries: list[dict[str, object]],
    ownership: dict[str, dict[str, object]],
    ownership_sha256: str,
) -> dict[str, object]:
    residual_ids = sorted(
        {
            str(result["id"])
            for summary in summaries
            for result in summary["tests"]
            if result["status"] != "exact"
        }
    )
    unknown = sorted(set(residual_ids) - set(ownership))
    if unknown:
        preview = ", ".join(unknown[:10])
        suffix = "" if len(unknown) <= 10 else f" (+{len(unknown) - 10} more)"
        raise ValueError(f"unowned renderer residuals: {preview}{suffix}")

    behaviors = {
        test_id: _profile_behavior(test_id, summaries) for test_id in residual_ids
    }
    entries = []
    for summary in summaries:
        for result in summary["tests"]:
            if result["status"] == "exact":
                continue
            test_id = str(result["id"])
            reviewed = ownership[test_id]
            entry = {
                "test_id": test_id,
                "profile": summary["profile"],
                "logical_size_css_px": summary["logical_size_css_px"],
                "physical_size_px": summary["physical_size_px"],
                "device_scale": summary["device_scale"],
                "status": result["status"],
                "mismatched_pixels": result.get("mismatched_pixels"),
                "diff_signature": result.get("diff_signature"),
                "font_profile": result.get("font_profile"),
                "raster_configuration": result.get("raster_configuration"),
                "viewport_scale_behavior": behaviors[test_id],
                "root_cause": reviewed["root_cause"],
                "owning_subsystem": reviewed["owning_subsystem"],
                "minimized_reproducer": reviewed["minimized_reproducer"],
                "error": result.get("error"),
            }
            entries.append(entry)
    entries.sort(key=lambda item: (item["test_id"], item["profile"]))
    counts: dict[str, int] = {}
    for entry in entries:
        cause = str(entry["root_cause"])
        counts[cause] = counts.get(cause, 0) + 1
    return {
        "schema_version": SCHEMA_VERSION,
        "qualifying": False,
        "policy": "diagnostic only; never an allowlist, tolerance, or baseline",
        "ownership_input_sha256": ownership_sha256,
        "input_profile_results_sha256": canonical_sha256(
            [summary["result_sha256"] for summary in summaries]
        ),
        "counts_by_root_cause": dict(sorted(counts.items())),
        "residual_test_count": len(residual_ids),
        "residual_comparison_count": len(entries),
        "entries_sha256": canonical_sha256(entries),
        "entries": entries,
    }
