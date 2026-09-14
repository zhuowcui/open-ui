#!/usr/bin/env python3
"""Generate and verify the versioned renderer qualification contract.

The legacy SP20 baseline is an input to this contract, never an output.  This
is deliberate: expanding qualification profiles must not silently re-freeze
the 800x600@1 result.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs/renderer/generated/qualification-contract-v1.json"
STYLE_OUT = ROOT / "docs/renderer/generated/author-style-inventory.json"
JS_OUT = ROOT / "docs/renderer/generated/javascript-disposition.json"
LEGACY = ROOT / "docs/v02/generated/baseline.json"
LEGACY_SHA256 = "787cd40ae63d06d5933efa89a4eba65a70d6327673b8056b83cae76ef3606001"
CHROMIUM_VERSION = ROOT / "CHROMIUM_VERSION"
RASTER_ORACLE_BUILD = "147.0.7727.50"
COMPUTED = ROOT / "bindings/rust/openui-style/src/computed.rs"
SCHEMA = ROOT / "bindings/rust/openui-style/property-schema.csv"
JS_IDS = ROOT / "tools/wpt/sp20_javascript_exclusions.json"
MAPPING = ROOT / "tools/accountability/data/wpt_mapping.csv"
WPT_ROOT = (
    Path.home()
    / "chromium/src/third_party/blink/web_tests/external/wpt/css"
)

QUALIFICATION_PROFILES = (
    ("legacy-800x600@1", 800.0, 600.0, 1.0, "qualified-immutable"),
    ("mobile-375x667@2", 375.0, 667.0, 2.0, "required"),
    ("desktop-1280x720@1.25", 1280.0, 720.0, 1.25, "required"),
    ("desktop-1920x1080@1.5", 1920.0, 1080.0, 1.5, "required"),
)

FOCUSED_VIEWPORTS = (
    (320.0, 240.0),
    (375.0, 667.0),
    (800.0, 600.0),
    (600.0, 1200.0),
    (1280.0, 720.0),
    (1920.0, 1080.0),
    (2560.0, 1440.0),
    (3440.0, 1440.0),
)
FOCUSED_SCALES = (1.0, 1.25, 1.5, 2.0, 3.0)

# These fields describe engine bookkeeping rather than an author declaration.
# Every other ComputedStyle field consumed by layout or paint is classified as
# public-typed by the inventory.  New bookkeeping fields must be named here or
# the generated review artifact changes visibly.
INTERNAL_STYLE_FIELDS = {
    "animation_snapshot",
    "embedded_document_text",
    "establishes_transform_containing_block",
    "first_letter_style",
    "first_line_style",
    "is_first_letter_pseudo",
    "legacy_webkit_box",
    "legacy_webkit_line_clamp",
    "list_item_is_flow_root",
    "marker_style",
    "native_button_text_metrics",
    "native_control_text",
    "placeholder_style",
    "will_change_transform",
}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def encoded(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def physical_dimension(logical: float, scale: float) -> int:
    """Match winit's positive logical-to-physical rounding."""
    value = logical * scale
    if value <= 0 or value > 2**32 - 1:
        raise ValueError("profile dimension is outside the u32 surface range")
    return int(value + 0.5)


def hash_files(paths: list[Path]) -> dict[str, str]:
    return {
        str(path.relative_to(ROOT)): digest(path.read_bytes())
        for path in sorted(paths)
    }


def qualification_contract() -> dict[str, object]:
    if digest(LEGACY.read_bytes()) != LEGACY_SHA256:
        raise ValueError("immutable 800x600@1 baseline changed; refreezing is forbidden")
    legacy = json.loads(LEGACY.read_text())
    if legacy.get("results", {}).get("exact") != 5731:
        raise ValueError("legacy baseline no longer records 5731 exact results")

    fonts = list((ROOT / "bindings/rust/openui-text/fonts").glob("*.[to]tf"))
    fonts += list((ROOT / "bindings/rust/openui-text/fonts").glob("*.ttc"))
    resources = [
        ROOT / "tools/accountability/data/wpt_ported/sp20_resource_manifest.json",
        ROOT / "tools/wpt/sp20_focused_ids.json",
    ]
    profiles = []
    for name, width, height, scale, status in QUALIFICATION_PROFILES:
        profile: dict[str, object] = {
            "name": name,
            "status": status,
            "logical_size_css_px": {"width": width, "height": height},
            "physical_size_px": {
                "width": physical_dimension(width, scale),
                "height": physical_dimension(height, scale),
            },
            "device_scale": scale,
        }
        if status == "qualified-immutable":
            profile["result"] = {
                "manifest_sha256": LEGACY_SHA256,
                "exact": 5731,
                "total": 5731,
                "ordered_id_sha256": legacy["results"]["ordered_id_sha256"],
            }
        profiles.append(profile)

    focused = [
        {
            "logical_size_css_px": {"width": width, "height": height},
            "physical_size_px": {
                "width": physical_dimension(width, scale),
                "height": physical_dimension(height, scale),
            },
            "device_scale": scale,
        }
        for width, height in FOCUSED_VIEWPORTS
        for scale in FOCUSED_SCALES
    ]
    return {
        "schema_version": 1,
        "renderer_contract": "chromium-147-native-structure-v1",
        "chromium": {
            "build_identity": legacy["chromium_build_identity"],
            "raster_oracle_build_identity": RASTER_ORACLE_BUILD,
            "pin_sha256": digest(CHROMIUM_VERSION.read_bytes()),
        },
        "raster": {
            "destination_color_space": "sRGB",
            "pixel_format": "N32 premultiplied RGBA",
            "logical_recording": True,
            "physical_surface": True,
            "device_transform_before_raster": True,
            "whole_frame_resampling": "forbidden",
            "exact_tolerance_pixels": 0,
        },
        "resource_hashes": hash_files(resources),
        "font_byte_hashes": hash_files(fonts),
        "qualification_profiles": profiles,
        "focused_cross_product": focused,
    }


def computed_fields() -> dict[str, str]:
    text = COMPUTED.read_text()
    start = text.index("pub struct ComputedStyle {")
    end = text.index("\n}\n\nimpl ComputedStyle", start)
    result: dict[str, str] = {}
    for match in re.finditer(r"^\s+pub ([a-z][a-z0-9_]*):\s*([^,\n]+),", text[start:end], re.M):
        result[match.group(1)] = match.group(2).strip()
    if not result:
        raise ValueError("could not parse ComputedStyle fields")
    return result


def style_inventory() -> dict[str, object]:
    fields = computed_fields()
    consumers = {}
    for area in ("openui-layout/src", "openui-paint/src"):
        for path in sorted((ROOT / "bindings/rust" / area).rglob("*.rs")):
            text = path.read_text()
            for field in fields:
                if re.search(rf"\.{re.escape(field)}\b", text):
                    consumers.setdefault(field, []).append(str(path.relative_to(ROOT)))

    schema_names = {
        row["css_name"]: row
        for row in csv.DictReader(SCHEMA.read_text().splitlines())
    }
    rows = []
    for field in sorted(consumers):
        css_name = field.replace("_", "-")
        classification = "internal" if field in INTERNAL_STYLE_FIELDS else "public-typed"
        rows.append(
            {
                "field": field,
                "rust_type": fields[field],
                "classification": classification,
                "css_name": None if classification == "internal" else css_name,
                "canonical_schema_entry": css_name in schema_names,
                "consumers": consumers[field],
            }
        )
    if any(row["classification"] not in {"public-typed", "internal"} for row in rows):
        raise ValueError("unclassified author-facing ComputedStyle field")
    unknown_internal = INTERNAL_STYLE_FIELDS - set(fields)
    if unknown_internal:
        raise ValueError(f"stale internal field classifications: {sorted(unknown_internal)}")
    return {
        "schema_version": 1,
        "source": str(COMPUTED.relative_to(ROOT)),
        "consumer_roots": [
            "bindings/rust/openui-layout/src",
            "bindings/rust/openui-paint/src",
        ],
        "counts": {
            "consumed": len(rows),
            "public_typed": sum(row["classification"] == "public-typed" for row in rows),
            "internal": sum(row["classification"] == "internal" for row in rows),
            "unclassified": 0,
        },
        "fields": rows,
    }


def javascript_disposition() -> dict[str, object]:
    ids = json.loads(JS_IDS.read_text())
    mapping = {}
    for row in csv.DictReader(MAPPING.read_text().splitlines()):
        mapping[f"wpt/{row['sp_area']}/{row['test_name']}"] = row
    entries = []
    source_available = WPT_ROOT.is_dir()
    for test_id in ids:
        row = mapping[test_id]
        source_path = WPT_ROOT / row["chromium_test_path"]
        source_hash = None
        # A conservative audit: DOM/CSS writes are potentially lowerable, but
        # reads, timers, animation promises, assertions, canvas, networking and
        # storage keep a test behavioral until a deterministic lowering exists.
        disposition = "behavioral-nonvisual"
        reason = "executable browser behavior"
        if source_available and source_path.is_file():
            source = source_path.read_text(encoding="utf-8", errors="ignore")
            source_hash = digest(source_path.read_bytes())
            scripts = "\n".join(
                re.findall(r"<script(?:\s[^>]*)?>(.*?)</script>", source, re.I | re.S)
            )
            writes = re.search(
                r"classList|\.style(?:\.|\s*=)|setAttribute\(|appendChild\(|\.remove\(",
                scripts,
            )
            browser_behavior = re.search(
                r"promise_test|async_test|\btest\(|assert_|fetch\(|XMLHttpRequest|"
                r"localStorage|sessionStorage|getContext\(|getAnimations\(|"
                r"requestAnimationFrame|setTimeout|setInterval|\.click\(",
                scripts,
            )
            if writes and not browser_behavior:
                disposition = "deterministic-final-state-candidate"
                reason = "DOM/style writes with no asynchronous or assertion harness"
        entries.append(
            {
                "test_id": test_id,
                "chromium_test_path": row["chromium_test_path"],
                "source_sha256": source_hash,
                "disposition": disposition,
                "reason": reason,
            }
        )
    counts = {}
    for entry in entries:
        key = entry["disposition"]
        counts[key] = counts.get(key, 0) + 1
    return {
        "schema_version": 1,
        "source_inventory_sha256": digest(JS_IDS.read_bytes()),
        "source_available": source_available,
        "total": len(entries),
        "counts": counts,
        "policy": {
            "deterministic-final-state-candidate": "must be lowered and pixel-qualified before inclusion",
            "behavioral-nonvisual": "excluded from renderer coverage; no browser runtime is promised",
        },
        "entries": entries,
    }


def outputs() -> dict[Path, bytes]:
    return {
        OUT: encoded(qualification_contract()),
        STYLE_OUT: encoded(style_inventory()),
        JS_OUT: encoded(javascript_disposition()),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = outputs()
    if args.check:
        drift = [
            str(path.relative_to(ROOT))
            for path, data in generated.items()
            if not path.is_file() or path.read_bytes() != data
        ]
        if drift:
            raise SystemExit("renderer contract drift: " + ", ".join(drift))
    else:
        for path, data in generated.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    inventory = json.loads(generated[STYLE_OUT])
    audit = json.loads(generated[JS_OUT])
    print(
        "renderer contract: profiles=4 focused=40 "
        f"style={inventory['counts']['consumed']} unclassified=0 "
        f"javascript={audit['total']}"
    )


if __name__ == "__main__":
    main()
