#!/usr/bin/env python3
"""Generate the hash-pinned renderer font-container certification fixtures.

The inputs are licensed files from Chromium's pinned WPT checkout.  The OTC
fixture is a deterministic one-face OpenType collection derived from the OTF
input; it does not depend on a host font tool or its version.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import struct
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WPT_ROOT = Path(
    os.environ.get(
        "CHROMIUM_WPT_ROOT",
        str(
            Path.home()
            / "chromium/src/third_party/blink/web_tests/external/wpt"
        ),
    )
).resolve()
OUTPUT = ROOT / "bindings/rust/openui-text/fonts/certification"

SOURCES = {
    "fixture.ttf": (
        "preload/resources/font.ttf",
        "b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448",
    ),
    "fixture.otf": (
        "css/css-font-loading/resources/Rochester.otf",
        "235f7207a202026a0a73c38c64713d58eace082bdf605f5abd2a28166fab61aa",
    ),
    "fixture.woff": (
        "css/css-fonts/support/fonts/pass.woff",
        "4e0e5135fa60f01e456e74ea50af23537d39cfc7822ab01dfa499a6efd8544e0",
    ),
    "fixture.woff2": (
        "css/css-font-loading/resources/GenR102.woff2",
        "984e2d1e7062a65a5ac746f38520a9ce0e1058e33533bfddd243bb5cfba061d4",
    ),
    "fixture.ttc": (
        "css/css-fonts/variations/resources/ahem.ttc",
        "29456016c3d05578b3702eb38258f2872e82048c811ae7e5927406ec8048dc55",
    ),
    "LICENSE-WPT.md": (
        "LICENSE.md",
        "5fac07febb0e2a97fb0d7b0def149ec08b642e1ba4b9c345283ab1cbd2af6570",
    ),
}
OTC_SHA256 = "d646722927f94b620e35e3ed3a99ad34e856b400dd2fb191a51d3d357a97d216"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_sources() -> dict[str, bytes]:
    values = {}
    for output_name, (relative, expected_hash) in SOURCES.items():
        path = WPT_ROOT / relative
        data = path.read_bytes()
        actual_hash = digest(data)
        if actual_hash != expected_hash:
            raise ValueError(
                f"pinned WPT input changed: {relative}: "
                f"expected {expected_hash}, got {actual_hash}"
            )
        values[output_name] = data
    return values


def make_otc(otf: bytes) -> bytes:
    """Wrap one CFF-flavoured SFNT in a TTC v1 container.

    Table directory offsets in a TTC are relative to the beginning of the TTC,
    so each absolute offset is advanced by the 16-byte collection header.
    """

    if otf[:4] != b"OTTO" or len(otf) < 12:
        raise ValueError("OTC source is not an OpenType/CFF SFNT")
    table_count = struct.unpack_from(">H", otf, 4)[0]
    directory_end = 12 + table_count * 16
    if directory_end > len(otf):
        raise ValueError("OTC source has a truncated table directory")
    adjusted = bytearray(otf)
    for index in range(table_count):
        offset_position = 12 + index * 16 + 8
        table_offset = struct.unpack_from(">I", adjusted, offset_position)[0]
        if table_offset >= len(otf):
            raise ValueError("OTC source has an invalid table offset")
        struct.pack_into(">I", adjusted, offset_position, table_offset + 16)
    result = b"ttcf" + struct.pack(">III", 0x00010000, 1, 16) + adjusted
    if digest(result) != OTC_SHA256:
        raise ValueError("deterministic OTC output hash changed")
    return result


def generated_outputs() -> dict[Path, bytes]:
    inputs = read_sources()
    outputs = {OUTPUT / name: data for name, data in inputs.items()}
    otc = make_otc(inputs["fixture.otf"])
    outputs[OUTPUT / "fixture.otc"] = otc
    fixtures = []
    for output_name, (relative, input_hash) in SOURCES.items():
        if output_name.startswith("LICENSE"):
            continue
        data = inputs[output_name]
        fixtures.append(
            {
                "format": output_name.rsplit(".", 1)[1],
                "path": output_name,
                "byte_length": len(data),
                "sha256": digest(data),
                "source": relative,
                "source_sha256": input_hash,
            }
        )
    fixtures.append(
        {
            "format": "otc",
            "path": "fixture.otc",
            "byte_length": len(otc),
            "sha256": digest(otc),
            "source": SOURCES["fixture.otf"][0],
            "source_sha256": SOURCES["fixture.otf"][1],
            "derivation": "ttc-v1-one-face-offset-adjustment-v1",
        }
    )
    manifest = {
        "schema_version": 1,
        "source_checkout": "pinned Chromium WPT",
        "license_path": "LICENSE-WPT.md",
        "license_sha256": SOURCES["LICENSE-WPT.md"][1],
        "fixtures": sorted(fixtures, key=lambda item: item["format"]),
    }
    outputs[OUTPUT / "manifest-v1.json"] = (
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    ).encode()
    return outputs


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    outputs = generated_outputs()
    if args.check:
        drift = [
            str(path.relative_to(ROOT))
            for path, data in outputs.items()
            if not path.is_file() or path.read_bytes() != data
        ]
        if drift:
            raise SystemExit("font fixture drift: " + ", ".join(drift))
    else:
        OUTPUT.mkdir(parents=True, exist_ok=True)
        for path, data in outputs.items():
            path.write_bytes(data)
    print(f"font certification fixtures: {len(outputs) - 2} font formats")


if __name__ == "__main__":
    main()
