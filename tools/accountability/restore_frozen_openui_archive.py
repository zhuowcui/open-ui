#!/usr/bin/env python3
"""Verify and restore the byte-frozen SP20 OpenUI PNG archive.

The original PNGs remain unchanged and git-ignored.  This compact archive
only makes their exact bytes available to clean CI checkouts.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import tarfile
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "tools/wpt/sp20_focused_ids.json"
ARCHIVE = (
    ROOT
    / "tools/accountability/data/pixel_comparison/frozen-openui-5731-v1.tar.xz"
)
RESULTS = ROOT / "tools/accountability/data/pixel_comparison/results"
ARCHIVE_SHA256 = "f8006421f2e5e90b4050079fab6ebd197267271b3f127babc8e8e99f5d09af86"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def frozen_images() -> dict[str, bytes]:
    if hashlib.sha256(ARCHIVE.read_bytes()).hexdigest() != ARCHIVE_SHA256:
        raise ValueError("frozen OpenUI archive hash changed")
    ids = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if len(ids) != 5731 or len(set(ids)) != len(ids):
        raise ValueError("frozen manifest is not the 5,731-case unique set")
    if any(
        not test_id.startswith("wpt/")
        or test_id != PurePosixPath(test_id).as_posix()
        or any(part in {"", ".", ".."} for part in test_id.split("/"))
        or "\\" in test_id
        for test_id in ids
    ):
        raise ValueError("frozen manifest contains an unsafe test ID")
    expected = {f"{test_id}/openui.png" for test_id in ids}
    images: dict[str, bytes] = {}
    with tarfile.open(ARCHIVE, mode="r:xz") as archive:
        members = archive.getmembers()
        if len(members) != len(expected) or {m.name for m in members} != expected:
            raise ValueError("frozen OpenUI archive entries differ from the manifest")
        for member in members:
            if not member.isfile() or member.size <= len(PNG_SIGNATURE):
                raise ValueError(f"invalid frozen image entry: {member.name}")
            source = archive.extractfile(member)
            if source is None:
                raise ValueError(f"unreadable frozen image entry: {member.name}")
            data = source.read()
            if len(data) != member.size or not data.startswith(PNG_SIGNATURE):
                raise ValueError(f"invalid frozen PNG bytes: {member.name}")
            images[member.name] = data
    return images


def main() -> None:
    parser = argparse.ArgumentParser()
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true")
    action.add_argument("--verify-local", action="store_true")
    action.add_argument("--restore", action="store_true")
    parser.add_argument("--results-dir", type=Path, default=RESULTS)
    args = parser.parse_args()

    images = frozen_images()
    if args.check:
        print(f"frozen OpenUI archive: {len(images)} byte-pinned PNGs")
        return

    missing: list[tuple[Path, bytes]] = []
    for name, data in images.items():
        destination = args.results_dir / name
        if destination.is_file():
            if destination.read_bytes() != data:
                raise ValueError(f"existing frozen PNG differs: {destination}")
        elif args.verify_local:
            raise ValueError(f"frozen PNG is missing: {destination}")
        else:
            missing.append((destination, data))

    if args.restore:
        for destination, data in missing:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
        print(f"frozen OpenUI archive: restored {len(missing)}, verified {len(images) - len(missing)}")
    else:
        print(f"frozen OpenUI archive: {len(images)} local PNGs byte-identical")


if __name__ == "__main__":
    main()
