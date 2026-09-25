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
import subprocess
import sys
from html.parser import HTMLParser
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs/renderer/generated/qualification-contract-v2.json"
STYLE_OUT = ROOT / "docs/renderer/generated/author-style-inventory.json"
JS_OUT = ROOT / "docs/renderer/generated/javascript-disposition.json"
JS_AST_OUT = ROOT / "docs/renderer/generated/javascript-mutation-audit-v2.json"
MUTATION_IDS_OUT = ROOT / "tools/qualification/manifests/mutation-candidates-v1.json"
LEGACY = ROOT / "docs/v02/generated/baseline.json"
LEGACY_SHA256 = "787cd40ae63d06d5933efa89a4eba65a70d6327673b8056b83cae76ef3606001"
CHROMIUM_VERSION = ROOT / "CHROMIUM_VERSION"
RASTER_ORACLE_BUILD = "147.0.7727.50"
COMPUTED = ROOT / "bindings/rust/openui-style/src/computed.rs"
SCHEMA = ROOT / "bindings/rust/openui-style/property-schema.csv"
INTERNAL_SCHEMA = ROOT / "bindings/rust/openui-style/internal-style-fields.csv"
JS_IDS = ROOT / "tools/wpt/sp20_javascript_exclusions.json"
MAPPING = ROOT / "tools/accountability/data/wpt_mapping.csv"
FULL_MANIFEST = ROOT / "tools/qualification/manifests/complete-5731.json"
FOCUSED_MANIFEST = ROOT / "tools/qualification/manifests/focused-raster.json"
PRIMITIVE_MANIFEST = ROOT / "tools/qualification/manifests/primitive-raster.json"
EXPANDED_MANIFEST = ROOT / "tools/qualification/manifests/expanded-v1.json"
RESIDUAL_OWNERSHIP = ROOT / "tools/qualification/residual-ownership-v2.json"
MEDIA_FIRST_FRAMES = ROOT / "docs/renderer/generated/media-first-frames-v1.json"
WPT_ROOT = (
    Path.home()
    / "chromium/src/third_party/blink/web_tests/external/wpt/css"
)
ACORN = (
    Path.home()
    / "chromium/src/third_party/node/node_modules/acorn/dist/acorn.mjs"
)
MUTATION_IR = ROOT / "tools/wpt/javascript_mutation_ir.mjs"
sys.path.insert(0, str(ROOT))

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


def qualification_contract(javascript_audit: bytes) -> dict[str, object]:
    if digest(LEGACY.read_bytes()) != LEGACY_SHA256:
        raise ValueError("immutable 800x600@1 baseline changed; refreezing is forbidden")
    legacy = json.loads(LEGACY.read_text())
    if legacy.get("results", {}).get("exact") != 5731:
        raise ValueError("legacy baseline no longer records 5731 exact results")

    font_root = ROOT / "bindings/rust/openui-text/fonts"
    fonts = [
        path
        for path in font_root.rglob("*")
        if path.is_file()
        and path.suffix.lower() in {".ttf", ".otf", ".ttc", ".otc", ".woff", ".woff2"}
    ]
    resources = [
        ROOT / "tools/accountability/data/wpt_ported/sp20_resource_manifest.json",
        FULL_MANIFEST,
        FOCUSED_MANIFEST,
        PRIMITIVE_MANIFEST,
        EXPANDED_MANIFEST,
        RESIDUAL_OWNERSHIP,
        MEDIA_FIRST_FRAMES,
    ]
    full_ids = json.loads(FULL_MANIFEST.read_text())
    focused_ids = json.loads(FOCUSED_MANIFEST.read_text())
    primitive_ids = json.loads(PRIMITIVE_MANIFEST.read_text())
    expanded = json.loads(EXPANDED_MANIFEST.read_text())
    ownership = json.loads(RESIDUAL_OWNERSHIP.read_text())
    if full_ids != sorted(set(full_ids)) or len(full_ids) != 5731:
        raise ValueError("complete renderer manifest is not the immutable 5,731-case set")
    ownership_ids = [entry["test_id"] for entry in ownership.get("entries", [])]
    if len(ownership_ids) != len(set(ownership_ids)) or not set(ownership_ids) <= set(full_ids):
        raise ValueError("residual ownership has duplicate or out-of-contract test IDs")
    if focused_ids != sorted(set(focused_ids)) or not focused_ids:
        raise ValueError("focused raster manifest must be sorted, unique, and nonempty")
    if not set(focused_ids) <= set(full_ids):
        raise ValueError("focused raster manifest contains a test outside the complete suite")
    if primitive_ids != sorted(set(primitive_ids)) or not primitive_ids:
        raise ValueError("primitive raster manifest must be sorted, unique, and nonempty")
    if not set(primitive_ids) <= set(full_ids):
        raise ValueError("primitive raster manifest contains a test outside the complete suite")
    expanded_base = (EXPANDED_MANIFEST.parent / expanded["base_manifest"]).resolve()
    if expanded_base != FULL_MANIFEST.resolve() or expanded["base_manifest_sha256"] != digest(
        FULL_MANIFEST.read_bytes()
    ):
        raise ValueError("expanded manifest does not preserve the immutable 5,731-case base")
    expanded_audit = (EXPANDED_MANIFEST.parent / expanded["candidate_audit"]).resolve()
    if (
        expanded_audit != JS_AST_OUT.resolve()
        or expanded["candidate_audit_sha256"] != digest(javascript_audit)
    ):
        raise ValueError("expanded manifest candidate audit hash changed")
    expanded_additions = expanded.get("additions")
    if expanded_additions != sorted(set(expanded_additions)):
        raise ValueError("expanded renderer additions must be sorted and unique")
    expanded_ids = sorted(set(full_ids) | set(expanded_additions))
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
        "schema_version": 2,
        "renderer_contract": "chromium-147-native-structure-v2",
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
            "qualification_backend": "cpu-skia",
            "portable_backend": "cpu-skia",
            "candidate_backend": {
                "name": "ganesh-gl",
                "promotion_policy": "global primitive, focused, and full-census improvement only",
            },
            "backend_selection": "explicit-immutable",
            "backend_identity_fields": [
                "backend",
                "gl_renderer",
                "gl_version",
                "driver",
                "color_type",
                "sample_count",
                "surface_properties",
            ],
        },
        "qualification_evidence": {
            "clean_source_tree_required": True,
            "dirty_runs": "diagnostic-only",
            "report_schema_version": 2,
            "chromium_oracle_cache": "immutable-browser-only-write-once",
            "chromium_oracle_identity_includes": [
                "chromium_binary_sha256",
                "chromium_build_identity",
                "chromium_capture_harness_sha256",
                "contract_sha256",
                "fixture_sha256",
                "font_profile",
                "font_byte_hashes",
                "resource_hashes",
                "chromium_feature_flags",
                "viewport",
                "device_scale",
            ],
            "cache_identity_includes": [
                "source_tree_sha256",
                "harness_sha256",
                "renderer_binary_sha256",
                "chromium_oracle_identity_sha256",
                "fixture_sha256",
                "font_byte_hashes",
                "resource_hashes",
                "raster_backend_identity",
                "viewport",
                "device_scale",
            ],
        },
        "residuals": {
            "ownership_path": str(RESIDUAL_OWNERSHIP.relative_to(ROOT)),
            "ownership_sha256": digest(RESIDUAL_OWNERSHIP.read_bytes()),
            "unknown_ownership": "generator-error",
            "root_cause_enum": [
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
            ],
            "scale_sweep": {
                "logical_size_css_px": {"width": 800.0, "height": 600.0},
                "device_scales": list(FOCUSED_SCALES),
            },
            "viewport_sweep": {
                "logical_sizes_css_px": [
                    {"width": width, "height": height}
                    for width, height in FOCUSED_VIEWPORTS
                ],
                "device_scale": 1.0,
            },
            "unisolated_interaction_sweep": "focused_cross_product",
        },
        "resource_hashes": hash_files(resources),
        "font_byte_hashes": hash_files(fonts),
        "font_certification": {
            "fixture_manifest": "bindings/rust/openui-text/fonts/certification/manifest-v1.json",
            "container_formats": ["ttf", "otf", "woff", "woff2", "ttc", "otc"],
            "device_scales": list(FOCUSED_SCALES),
            "metadata_parity": ["rust", "c"],
            "qualifying_font_source": "document-owned-hash-pinned-bytes",
            "ambient_system_font_expansion": "forbidden",
        },
        "media_certification": {
            "first_frame_manifest": str(MEDIA_FIRST_FRAMES.relative_to(ROOT)),
            "first_frame_manifest_sha256": digest(MEDIA_FIRST_FRAMES.read_bytes()),
            "decoded_transport": "OUIR-v1-sRGB-unpremultiplied-RGBA8",
            "playback": "outside-renderer-contract",
        },
        "suite_manifests": {
            "full": {
                "path": str(FULL_MANIFEST.relative_to(ROOT)),
                "sha256": digest(FULL_MANIFEST.read_bytes()),
                "count": len(full_ids),
                "evidence": "diagnostic-census",
            },
            "focused": {
                "path": str(FOCUSED_MANIFEST.relative_to(ROOT)),
                "sha256": digest(FOCUSED_MANIFEST.read_bytes()),
                "count": len(focused_ids),
                "evidence": "qualification-gate",
            },
            "primitive": {
                "path": str(PRIMITIVE_MANIFEST.relative_to(ROOT)),
                "sha256": digest(PRIMITIVE_MANIFEST.read_bytes()),
                "count": len(primitive_ids),
                "evidence": "qualification-gate",
                "primitive_classes": [
                    "fills",
                    "strokes",
                    "rectangles",
                    "rounded-rectangles",
                    "circles-and-controls",
                    "paths",
                    "gradients",
                    "shadows-and-filters",
                    "images",
                    "glyphs-and-decorations",
                    "clips",
                    "fractional-phases",
                ],
            },
            "expanded": {
                "path": str(EXPANDED_MANIFEST.relative_to(ROOT)),
                "sha256": digest(EXPANDED_MANIFEST.read_bytes()),
                "count": len(expanded_ids),
                "original_count": len(full_ids),
                "admitted_candidate_count": len(expanded_additions),
                "evidence": "qualification-gate",
            },
        },
        "qualification_profiles": profiles,
        "focused_cross_product": focused,
    }


def computed_fields() -> dict[str, str]:
    text = COMPUTED.read_text()
    start = text.index("pub struct ComputedStyleFields {")
    end = text.index("\n}\n", start)
    result: dict[str, str] = {}
    for match in re.finditer(
        r"^\s+(?:pub(?:\(crate\))?\s+)?([a-z][a-z0-9_]*):\s*(.+),$",
        text[start:end],
        re.M,
    ):
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

    schema = list(csv.DictReader(SCHEMA.read_text().splitlines()))
    mapped: dict[str, dict[str, str]] = {}
    for row in schema:
        value = row["computed_fields"].strip()
        for field in [] if value == "-" else value.split(";"):
            if field in mapped:
                raise ValueError(f"duplicate computed-field mapping: {field}")
            if field not in fields:
                raise ValueError(f"unknown computed-field mapping: {field}")
            mapped[field] = row
    internal_rows = list(csv.DictReader(INTERNAL_SCHEMA.read_text().splitlines()))
    internal: dict[str, str] = {}
    for row in internal_rows:
        field = row["field"]
        rationale = row["rationale"].strip()
        if field in internal:
            raise ValueError(f"duplicate internal field mapping: {field}")
        if field not in fields:
            raise ValueError(f"stale internal field mapping: {field}")
        if not rationale:
            raise ValueError(f"missing rationale for internal field: {field}")
        if field in mapped:
            raise ValueError(f"field is both authored and internal: {field}")
        internal[field] = rationale
    rows = []
    for field in sorted(consumers):
        if field in mapped:
            classification = "public-typed"
            schema_row = mapped[field]
        elif field in internal:
            classification = "internal"
            schema_row = None
        else:
            raise ValueError(f"unclassified consumed ComputedStyle field: {field}")
        rows.append(
            {
                "field": field,
                "rust_type": fields[field],
                "classification": classification,
                "css_name": None if schema_row is None else schema_row["css_name"],
                "property_id": None if schema_row is None else int(schema_row["id"]),
                "canonical_schema_entry": schema_row is not None,
                "internal_rationale": internal.get(field),
                "consumers": consumers[field],
            }
        )
    public_count = sum(row["classification"] == "public-typed" for row in rows)
    if public_count != 202:
        raise ValueError(f"expected 202 consumed author fields, found {public_count}")
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


class InlineScriptCollector(HTMLParser):
    """Extract JavaScript without using JavaScript syntax as a heuristic."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=False)
        self.scripts: list[dict[str, str]] = []
        self.external_scripts: list[str] = []
        self._script: list[str] | None = None
        self._script_origin = ""

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = {name.lower(): value or "" for name, value in attrs}
        if tag.lower() == "script":
            source = attributes.get("src")
            script_type = attributes.get("type", "").lower()
            if source:
                self.external_scripts.append(source)
            elif script_type in {"", "text/javascript", "application/javascript", "module"}:
                self._script = []
                self._script_origin = "inline-script"
        for name, value in attrs:
            if name.lower().startswith("on") and value:
                self.scripts.append(
                    {"origin": f"inline-handler:{name.lower()}", "code": value}
                )

    def handle_data(self, data: str) -> None:
        if self._script is not None:
            self._script.append(data)

    def handle_endtag(self, tag: str) -> None:
        if tag.lower() == "script" and self._script is not None:
            self.scripts.append(
                {"origin": self._script_origin, "code": "".join(self._script)}
            )
            self._script = None


def _cssom_property_name(value: str) -> str:
    if value == "cssFloat":
        return "float"
    return re.sub(r"([A-Z])", lambda match: "-" + match.group(1).lower(), value)


def _validate_mutation_styles(analysis: dict[str, object]) -> None:
    """Attach canonical schema identities to every static cascade delta."""

    from tools.wpt.port_wpt import parse_inline_styles

    by_css_name = {
        row["css_name"]: row
        for row in csv.DictReader(SCHEMA.read_text().splitlines())
    }
    rejections = analysis["rejections"]
    assert isinstance(rejections, list)

    def reject(operation: dict[str, object], detail: str) -> None:
        rejections.append(
            {
                "reason": "style-declaration-outside-engine-schema",
                "origin": operation["origin"],
                "line": operation["line"],
                "node_type": "CssStyleDeclaration",
                "detail": detail,
            }
        )

    def delta(operation: dict[str, object], name: str, value: str) -> dict[str, object] | None:
        canonical = _cssom_property_name(name)
        row = by_css_name.get(canonical)
        if row is None:
            reject(operation, canonical)
            return None
        return {
            "css_name": canonical,
            "property_id": int(row["id"]),
            "rust_name": row["rust_name"],
            "value": value,
            "engine_api": "Engine::set_property",
        }

    for operation in analysis["operations"]:
        kind = operation["kind"]
        if kind == "set-style":
            value = operation.get("value")
            name = operation.get("property")
            if isinstance(name, str) and isinstance(value, (str, int, float)):
                css_value = str(value)
                operation["value"] = css_value
                item = delta(operation, name, css_value)
                operation["cascade_delta"] = item
            continue
        style_text = None
        if kind == "set-style-text" and isinstance(operation.get("value"), str):
            style_text = operation["value"]
        elif (
            kind == "set-attribute"
            and operation.get("name") == "style"
            and isinstance(operation.get("value"), str)
        ):
            style_text = operation["value"]
        if style_text is not None:
            declarations = parse_inline_styles(style_text)
            operation["cascade_deltas"] = [
                item
                for name, value in declarations.items()
                if (item := delta(operation, name, value)) is not None
            ]
    reasons = sorted({entry["reason"] for entry in rejections})
    analysis["rejection_reasons"] = reasons
    analysis["lowerable"] = bool(
        any(op["kind"] != "layout-barrier" for op in analysis["operations"])
        and not reasons
    )


def _validate_mutation_lowering_scope(analysis: dict[str, object]) -> None:
    """Reject AST-described behavior outside the renderer's document model."""
    rejections = analysis["rejections"]
    assert isinstance(rejections, list)

    def contains(value: object, predicate) -> bool:
        if isinstance(value, dict):
            return predicate(value) or any(contains(item, predicate) for item in value.values())
        if isinstance(value, list):
            return any(contains(item, predicate) for item in value)
        return False

    for operation in analysis["operations"]:
        if contains(operation, lambda item: item.get("kind") == "expression"):
            rejections.append(
                {
                    "reason": "runtime-dependent-expression-target",
                    "origin": operation["origin"],
                    "line": operation["line"],
                    "node_type": "MutationTarget",
                    "detail": operation["kind"],
                }
            )
            continue
        if contains(
            operation,
            lambda item: item.get("kind") == "member"
            and item.get("property") == "contentDocument",
        ):
            rejections.append(
                {
                    "reason": "embedded-document-mutation",
                    "origin": operation["origin"],
                    "line": operation["line"],
                    "node_type": "MutationTarget",
                    "detail": operation["kind"],
                }
            )
            continue
        target = operation.get("target")
        if (
            operation.get("kind") in {"append", "insert-before"}
            and isinstance(target, dict)
            and target.get("kind") == "member"
            and target.get("property") == "documentElement"
        ):
            rejections.append(
                {
                    "reason": "document-root-structure-mutation",
                    "origin": operation["origin"],
                    "line": operation["line"],
                    "node_type": "MutationTarget",
                    "detail": operation["kind"],
                }
            )

    reasons = sorted({entry["reason"] for entry in rejections})
    analysis["rejection_reasons"] = reasons
    analysis["lowerable"] = bool(
        any(op["kind"] != "layout-barrier" for op in analysis["operations"])
        and not reasons
    )


def javascript_disposition() -> dict[str, object]:
    legacy = json.loads(JS_OUT.read_text())
    legacy_candidates = [
        entry["test_id"]
        for entry in legacy["entries"]
        if entry["disposition"] == "deterministic-final-state-candidate"
    ]
    if legacy.get("schema_version") != 1 or len(legacy_candidates) != 393:
        raise ValueError("immutable JavaScript candidate inventory is not the 393-case set")
    expanded = json.loads(EXPANDED_MANIFEST.read_text())
    if expanded.get("schema_version") != 1:
        raise ValueError("unsupported expanded renderer manifest schema")
    admitted = set(expanded.get("additions", []))
    if len(admitted) != len(expanded.get("additions", [])):
        raise ValueError("expanded renderer manifest contains duplicate additions")
    unknown_admissions = admitted - set(legacy_candidates)
    if unknown_admissions:
        raise ValueError(
            "expanded renderer manifest admits non-candidates: "
            + ", ".join(sorted(unknown_admissions))
        )
    mapping = {}
    for row in csv.DictReader(MAPPING.read_text().splitlines()):
        mapping[f"wpt/{row['sp_area']}/{row['test_name']}"] = row
    source_available = WPT_ROOT.is_dir()
    documents = []
    sources = {}
    for test_id in legacy_candidates:
        row = mapping[test_id]
        source_path = WPT_ROOT / row["chromium_test_path"]
        if not source_path.is_file():
            documents.append({"test_id": test_id, "scripts": [], "external_scripts": []})
            sources[test_id] = None
            continue
        raw = source_path.read_bytes()
        collector = InlineScriptCollector()
        collector.feed(raw.decode("utf-8", errors="ignore"))
        documents.append(
            {
                "test_id": test_id,
                "scripts": collector.scripts,
                "external_scripts": collector.external_scripts,
            }
        )
        sources[test_id] = digest(raw)
    if not ACORN.is_file():
        raise ValueError(f"pinned Chromium Acorn parser is missing: {ACORN}")
    completed = subprocess.run(
        ["node", MUTATION_IR, ACORN],
        input=json.dumps(documents),
        capture_output=True,
        text=True,
        check=True,
    )
    analyses = {entry["test_id"]: entry for entry in json.loads(completed.stdout)}
    entries = []
    for test_id in legacy_candidates:
        analysis = analyses[test_id]
        _validate_mutation_styles(analysis)
        _validate_mutation_lowering_scope(analysis)
        if not source_available or sources[test_id] is None:
            disposition = "behavioral-nonvisual"
            reason = "source-unavailable"
        elif analysis["lowerable"]:
            if test_id in admitted:
                disposition = "lowered-exact"
                reason = "AST-lowered final state is exact at all four qualification profiles"
            else:
                disposition = "ast-lowered-pending-exact"
                reason = "bounded synchronous mutation IR requires four-profile pixel qualification"
        else:
            disposition = "behavioral-nonvisual"
            reasons = analysis["rejection_reasons"]
            reason = reasons[0] if reasons else "no-final-state-mutation"
        row = mapping[test_id]
        entries.append(
            {
                "test_id": test_id,
                "chromium_test_path": row["chromium_test_path"],
                "source_sha256": sources[test_id],
                "disposition": disposition,
                "reason": reason,
                "mutation_ir": analysis,
            }
        )
    counts = {}
    for entry in entries:
        key = entry["disposition"]
        counts[key] = counts.get(key, 0) + 1
    return {
        "schema_version": 2,
        "source_inventory_sha256": digest(JS_OUT.read_bytes()),
        "candidate_inventory_sha256": digest(
            ("\n".join(legacy_candidates) + "\n").encode()
        ),
        "parser": {
            "name": "Acorn",
            "path": "third_party/node/node_modules/acorn/dist/acorn.mjs",
            "sha256": digest(ACORN.read_bytes()),
        },
        "source_available": source_available,
        "total": len(entries),
        "counts": counts,
        "policy": {
            "ast-lowered-pending-exact": "must be applied through Engine APIs and pixel-qualified before inclusion",
            "lowered-exact": "admitted only after exact results at all four qualification profiles",
            "behavioral-nonvisual": "excluded from renderer coverage; no browser runtime is promised",
        },
        "entries": entries,
    }


def outputs() -> dict[Path, bytes]:
    javascript_audit = encoded(javascript_disposition())
    audit = json.loads(javascript_audit)
    mutation_ids = sorted(
        entry["test_id"]
        for entry in audit["entries"]
        if entry["disposition"] in {"ast-lowered-pending-exact", "lowered-exact"}
    )
    return {
        OUT: encoded(qualification_contract(javascript_audit)),
        STYLE_OUT: encoded(style_inventory()),
        JS_AST_OUT: javascript_audit,
        MUTATION_IDS_OUT: encoded(mutation_ids),
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
    audit = json.loads(generated[JS_AST_OUT])
    print(
        "renderer contract: profiles=4 focused=40 "
        f"style={inventory['counts']['consumed']} unclassified=0 "
        f"javascript={audit['total']}"
    )


if __name__ == "__main__":
    main()
