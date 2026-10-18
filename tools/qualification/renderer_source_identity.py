"""Source identity shared by the renderer build and qualification runner.

This module uses only the Python standard library so the offline pixel tool can
record its inputs during Cargo builds without loading the image-analysis stack.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]


def git(root: Path, *args: str) -> bytes:
    return subprocess.run(
        ["git", *args], cwd=root, check=True, capture_output=True,
    ).stdout


def source_paths(root: Path) -> list[bytes]:
    return sorted(
        path for path in git(root, "ls-files", "-co", "--exclude-standard", "-z").split(b"\0")
        if path
    )


def repository_source_identity(root: Path = ROOT) -> dict[str, object]:
    status = git(root, "status", "--porcelain=v1", "-z", "--untracked-files=all")
    digest = hashlib.sha256()
    for encoded_path in source_paths(root):
        path = root / os.fsdecode(encoded_path)
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
        *sorted((root / "tools/qualification").glob("*.py")),
        root / "tools/accountability/run_all_pixel_comparisons.py",
        root / "bindings/rust/pixel-compare/Cargo.toml",
        root / "bindings/rust/pixel-compare/src/main.rs",
        root / "bindings/rust/pixel-compare/src/wpt/mod.rs",
    ]
    harness = hashlib.sha256()
    for path in harness_paths:
        relative = str(path.relative_to(root)).encode("utf-8")
        content = path.read_bytes()
        harness.update(len(relative).to_bytes(8, "big"))
        harness.update(relative)
        harness.update(len(content).to_bytes(8, "big"))
        harness.update(content)
    return {
        "commit": git(root, "rev-parse", "HEAD").decode().strip(),
        "clean": not status,
        "status_sha256": hashlib.sha256(status).hexdigest(),
        "source_tree_sha256": digest.hexdigest(),
        "harness_sha256": harness.hexdigest(),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cargo-output", type=Path, required=True)
    args = parser.parse_args()
    identity = repository_source_identity()
    args.cargo_output.write_text(
        json.dumps({
            "schema_version": 1,
            "source": identity,
            "skia_gn_args": os.environ.get("SKIA_GN_ARGS"),
        }, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    for path in source_paths(ROOT):
        print(f"cargo:rerun-if-changed={ROOT / os.fsdecode(path)}")
    git_dir = Path(git(ROOT, "rev-parse", "--absolute-git-dir").decode().strip())
    for name in ("HEAD", "index", "packed-refs"):
        print(f"cargo:rerun-if-changed={git_dir / name}")
    branch = subprocess.run(
        ["git", "symbolic-ref", "-q", "HEAD"], cwd=ROOT, capture_output=True, text=True,
    )
    if branch.returncode == 0:
        path = Path(git(ROOT, "rev-parse", "--git-path", branch.stdout.strip()).decode().strip())
        print(f"cargo:rerun-if-changed={path if path.is_absolute() else ROOT / path}")
    print("cargo:rerun-if-env-changed=SKIA_GN_ARGS")


if __name__ == "__main__":
    main()
