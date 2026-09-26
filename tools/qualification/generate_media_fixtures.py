#!/usr/bin/env python3
"""Generate hash-pinned decoded first-frame fixtures for static WPT media.

Media demuxing and decoding are intentionally outside the compact renderer.
This generator records the decoded pixels produced by Chromium's pinned VP9
decoder, while binding them to both the original WPT bytes and decoder
revision. Paint consumes a tiny codec-independent RGBA8 transport.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
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
OUTPUT = ROOT / "docs/renderer/generated/media-first-frames-v1.json"
DECODER = {
    "chromium_build": "147.0.7727.50",
    "codec": "vp9-profile-0",
    "implementation": "libvpx",
    "revision": "9a2d3d1f46afbdfa9b9820a9fd3aacb084e65e2f",
}

# The full-range sRGB VP9 frame decodes to Y/G=127, U/B=0, V/R=0 at every
# sample. Keep decoded data here, rather than a renderer branch keyed by the
# source, so adding another deterministic media fixture uses the same path.
FIXTURES = (
    {
        "source": "css/css-sizing/aspect-ratio/support/2x2-green.webm",
        "source_sha256": "1702f1648bf13716817d740357271a9a072d9ce57ff496ebda4055a8ae18562f",
        "source_byte_length": 559,
        "container": "video/webm",
        "coded_size": (2, 2),
        "timestamp_us": 0,
        "color_space": "srgb-full-range-identity",
        "rgba8": bytes((0, 127, 0, 255)) * 4,
    },
)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def rgba8_transport(width: int, height: int, pixels: bytes) -> bytes:
    if width <= 0 or height <= 0 or len(pixels) != width * height * 4:
        raise ValueError("invalid decoded RGBA8 fixture dimensions")
    return (
        b"OUIR"
        + bytes((1, 0, 0, 0))
        + width.to_bytes(4, "little")
        + height.to_bytes(4, "little")
        + pixels
    )


def generated_bytes() -> bytes:
    rows = []
    for fixture in FIXTURES:
        source = WPT_ROOT / fixture["source"]
        source_bytes = source.read_bytes()
        if len(source_bytes) != fixture["source_byte_length"]:
            raise ValueError(f"pinned WPT input length changed: {fixture['source']}")
        actual_source_hash = digest(source_bytes)
        if actual_source_hash != fixture["source_sha256"]:
            raise ValueError(
                f"pinned WPT input changed: {fixture['source']}: "
                f"expected {fixture['source_sha256']}, got {actual_source_hash}"
            )
        width, height = fixture["coded_size"]
        transport = rgba8_transport(width, height, fixture["rgba8"])
        rows.append(
            {
                "coded_size": [width, height],
                "color_space": fixture["color_space"],
                "container": fixture["container"],
                "output_byte_length": len(transport),
                "output_mime_type": "image/x-openui-rgba8",
                "output_sha256": digest(transport),
                "source": fixture["source"],
                "source_byte_length": len(source_bytes),
                "source_sha256": actual_source_hash,
                "timestamp_us": fixture["timestamp_us"],
                "transport_hex": transport.hex(),
            }
        )
    manifest = {
        "schema_version": 1,
        "decoder": DECODER,
        "fixture_count": len(rows),
        "fixtures": sorted(rows, key=lambda row: row["source"]),
        "license": "WPT source fixture; see the pinned WPT checkout LICENSE.md",
    }
    return (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = generated_bytes()
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_bytes() != output:
            raise SystemExit(f"media first-frame fixture drift: {OUTPUT.relative_to(ROOT)}")
    else:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_bytes(output)
    print(f"media first-frame fixtures: {len(FIXTURES)}")


if __name__ == "__main__":
    main()
