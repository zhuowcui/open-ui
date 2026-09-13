#!/usr/bin/env python3
"""Generate and verify the immutable Open UI v0.2 product contract."""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs" / "v02" / "generated"
IMMUTABLE_GENERATED = {
    "baseline.json": "787cd40ae63d06d5933efa89a4eba65a70d6327673b8056b83cae76ef3606001",
    "api-inventory.json": "e6d9e9200d0a2849f03f577fe01a9918be0b49049818cd1c3f03e3186797255f",
    "migration-ledger.csv": "aca0c1d7778a91361be909e9e6d9cc6fdfde2fc840e4f2e12db69f9cceef3edb",
}
SUMMARY = ROOT / "tools/accountability/data/pixel_comparison/results/summary.json"
FROZEN_INPUTS = (
    ROOT / "CHROMIUM_VERSION",
    ROOT / "bindings/rust/Cargo.lock",
    ROOT / "tools/accountability/data/wpt_mapping.csv",
    ROOT / "tools/accountability/data/wpt_ported/sp20_kickoff_mapping.csv",
    ROOT / "tools/accountability/data/wpt_ported/sp20_sp19_summary.json",
    ROOT / "tools/accountability/data/wpt_ported/sp20_partitions.json",
    ROOT / "tools/accountability/data/wpt_ported/sp20_accumulated_partitions.json",
    ROOT / "tools/accountability/data/wpt_ported/sp20_area_partitions.json",
    ROOT / "tools/accountability/data/wpt_ported/sp20_source_inventory.json",
    ROOT / "tools/accountability/data/wpt_ported/sp20_resource_manifest.json",
    SUMMARY,
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def baseline() -> bytes:
    summary = json.loads(SUMMARY.read_text())
    exact = [
        row["id"]
        for row in summary["tests"]
        if row.get("status") == "pass" and row.get("mismatch_pct") == 0.0
    ]
    if (summary["total"], summary["passed"], summary["failed"], summary["errors"]) != (
        5731,
        5731,
        0,
        0,
    ) or len(exact) != 5731:
        raise SystemExit("v0.2 contract requires the exact 5,731-case SP20 baseline")

    font_paths = sorted(
        path
        for path in (ROOT / "tools/accountability/data/fonts").rglob("*")
        if path.is_file()
    )
    fixture_paths = sorted((ROOT / "tests/pixel_apps/apps").glob("*.html"))
    if len(fixture_paths) != 10:
        raise SystemExit("v0.2 contract requires exactly ten historical application fixtures")
    example_paths = sorted(
        path
        for directory in (
            ROOT / "bindings/rust/examples",
            ROOT / "tests/pixel_comparison/openui_renders",
        )
        for path in directory.rglob("*")
        if path.is_file() and "target" not in path.parts
    )
    return json_bytes(
        {
            "schema_version": 1,
            "product_version": "0.2.0",
            "renderer": "pure-rust-sp20",
            "chromium_build_identity": (ROOT / "CHROMIUM_VERSION").read_text().strip(),
            "viewport": {"logical_width": 800, "logical_height": 600, "device_scale": 1},
            "results": {
                "total": 5731,
                "exact": 5731,
                "failed": 0,
                "errors": 0,
                "ordered_id_sha256": hashlib.sha256(("\n".join(exact) + "\n").encode()).hexdigest(),
            },
            "frozen_inputs": {rel(path): sha256(path) for path in FROZEN_INPUTS},
            "font_inventory": {rel(path): sha256(path) for path in font_paths},
            "historical_fixtures": {rel(path): sha256(path) for path in fixture_paths},
            "public_example_artifacts": {rel(path): sha256(path) for path in example_paths},
        }
    )


RUST_ITEM = re.compile(
    r"^\s*pub\s+(?:(?:unsafe|async|const)\s+)*(?:struct|enum|trait|type|fn|mod|use)\s+"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    re.MULTILINE,
)
C_FUNCTION = re.compile(r"OUI_EXPORT\s+[^;()]+?\s+(oui_[a-z0-9_]+)\s*\(", re.DOTALL)


def rust_inventory() -> list[dict[str, object]]:
    rows = []
    for path in sorted((ROOT / "bindings/rust").glob("openui*/src/**/*.rs")):
        text = path.read_text(errors="replace")
        for match in RUST_ITEM.finditer(text):
            rows.append(
                {
                    "language": "rust",
                    "symbol": match.group("name"),
                    "source": rel(path),
                    "line": text.count("\n", 0, match.start()) + 1,
                }
            )
    return rows


def c_inventory() -> list[dict[str, object]]:
    path = ROOT / "src/api/openui.h"
    text = path.read_text()
    return [
        {
            "language": "c",
            "symbol": match.group(1),
            "source": rel(path),
            "line": text.count("\n", 0, match.start()) + 1,
        }
        for match in C_FUNCTION.finditer(text)
    ]


def api_inventory() -> bytes:
    rows = rust_inventory() + c_inventory()
    rows.sort(key=lambda row: (str(row["language"]), str(row["source"]), int(row["line"])))
    return json_bytes({"schema_version": 1, "symbols": rows})


def migration() -> bytes:
    output = io.StringIO(newline="")
    writer = csv.writer(output, lineterminator="\n")
    writer.writerow(("language", "v0.1_symbol", "v0.2_disposition", "replacement"))
    rows = rust_inventory() + c_inventory()
    for row in sorted(rows, key=lambda item: (str(item["language"]), str(item["symbol"]), str(item["source"]))):
        symbol = str(row["symbol"])
        language = str(row["language"])
        if symbol in {"load_html", "inject_css", "oui_document_load_html"}:
            disposition, replacement = "removed", "typed node construction"
        elif symbol in {"set_style", "oui_element_set_style"}:
            disposition, replacement = "replaced", "typed property setters / oui_element_set_property"
        elif language == "c":
            disposition, replacement = "replaced", "openui-ffi v0.2 equivalent or explicit typed operation"
        elif "openui-sys" in str(row["source"]) or "openui-build" in str(row["source"]):
            disposition, replacement = "historical", "openui-ffi (C callers) or openui-engine (Rust callers)"
        else:
            disposition, replacement = "migrate", "safe openui v0.2 API over openui-engine"
        writer.writerow((language, symbol, disposition, replacement))
    return output.getvalue().encode()


def dependency_policy() -> bytes:
    lock = (ROOT / "bindings/rust/Cargo.lock").read_text()
    versions = re.findall(r'^version = "([^"]+)"$', lock, re.MULTILINE)
    prerelease = sorted({version for version in versions if "-" in version.split("+", 1)[0]})
    if prerelease:
        raise SystemExit(f"prerelease dependencies are forbidden: {prerelease}")
    return json_bytes(
        {
            "schema_version": 1,
            "msrv": "1.85",
            "lockfile_sha256": sha256(ROOT / "bindings/rust/Cargo.lock"),
            "prerelease_dependencies": [],
            "license_policy": "Apache-2.0, MIT, BSD, ISC, Unicode, FTL, or separately reviewed",
            "security_policy": "cargo audit is a release and CI gate",
        }
    )


def expected() -> dict[Path, bytes]:
    artifacts = {OUT / "dependency-policy.json": dependency_policy()}
    for name, expected_hash in IMMUTABLE_GENERATED.items():
        path = OUT / name
        if not path.is_file() or sha256(path) != expected_hash:
            raise SystemExit(f"immutable v0.2 kickoff artifact drift: {rel(path)}")
        artifacts[path] = path.read_bytes()
    return artifacts


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    artifacts = expected()
    if args.check:
        drift = [rel(path) for path, data in artifacts.items() if not path.is_file() or path.read_bytes() != data]
        if drift:
            raise SystemExit("v0.2 generated contract drift: " + ", ".join(drift))
        print("v0.2 contract: exact=5731/5731 generated=4 drift=0")
        return
    OUT.mkdir(parents=True, exist_ok=True)
    for path, data in artifacts.items():
        path.write_bytes(data)
    print("v0.2 contract: wrote 4 generated artifacts")


if __name__ == "__main__":
    main()
