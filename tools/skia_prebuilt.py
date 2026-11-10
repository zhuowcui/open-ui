#!/usr/bin/env python3
"""Build one Skia for all Windows worktrees and point Cargo at it.

rust-skia can install prebuilt Skia binaries instead of compiling Skia: with
FORCE_SKIA_BINARIES_DOWNLOAD set, its build script fetches
SKIA_BINARIES_URL (``file://`` works) with ``{tag}`` and ``{key}`` filled in.
Its key holds only the rust-skia commit, target triple and Skia features. It
does not hold SKIA_GN_ARGS or the compiler, so this script stores archives in
a directory named by a hash of those inputs as well, and a changed input never
reuses a stale Skia.

``build`` first asks rust-skia which key the given cargo arguments request
(a forced download from a missing file prints it), then compiles Skia once
from source with FORCE_SKIA_BUILD in its own target directory and packs the
files rust-skia's own export ships: the static libraries, ``icudtl.dat`` and
``bindings.rs``. (rust-skia's export hook itself refuses to run on Windows,
where it calls the workspace's ``freetype-woff2`` feature redundant.)
``env`` prints the variables that make a worktree's Cargo install that
archive instead of compiling Skia. The archive holds the same bytes the
from-source build linked, so pixels do not change.

Windows only. On other targets rust-skia passes Cargo's OPT_LEVEL to the
Skia compiler as ``-O<level>`` but leaves it out of the key, so a dev-profile
archive could be installed into a release build and change pixels.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tarfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
WORKSPACE = REPO / "bindings" / "rust"
DEFAULT_CARGO_ARGS = ["check", "--locked", "--workspace", "--all-targets"]

# Environment variables rust-skia's from-source build reads that change the
# compiled Skia but are absent from its binary key.
BUILD_INPUT_VARS = (
    "SKIA_GN_ARGS",
    "SKIA_DEBUG",
    "SKIA_USE_SYSTEM_LIBRARIES",
    "CC",
    "CXX",
    "CLANGCC",
    "CLANGCXX",
    "LLVM_HOME",
)

TRYING = re.compile(r"TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: (\S+)/(\S+)")


def fail(message: str) -> None:
    print(f"skia_prebuilt: {message}", file=sys.stderr)
    raise SystemExit(1)


def main_checkout_out() -> Path:
    """Return ``out/`` of the main checkout, shared by every worktree."""
    common = subprocess.run(
        ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
        cwd=REPO,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    return Path(common).parent / "out"


def rust_skia_rev(lock_text: str) -> str:
    match = re.search(
        r'name = "skia-bindings"\nversion = "[^"]+"\nsource = "git\+[^#"]+#([0-9a-f]{40})"',
        lock_text,
    )
    if not match:
        fail("skia-bindings is not a pinned git dependency in Cargo.lock")
    return match.group(1)


def compiler_identity() -> str:
    for compiler in ("clang-cl", "clang"):
        try:
            result = subprocess.run(
                [compiler, "--version"], capture_output=True, text=True, check=True
            )
        except (OSError, subprocess.CalledProcessError):
            continue
        return result.stdout.splitlines()[0].strip() if result.stdout else compiler
    fail("clang-cl or clang must be on PATH (rust-skia compiles Skia with it)")
    raise AssertionError


def build_inputs(env: dict[str, str]) -> dict[str, str]:
    if not env.get("SKIA_GN_ARGS"):
        fail("SKIA_GN_ARGS is unset; set the CI Skia variables first")
    inputs = {name: env.get(name, "") for name in BUILD_INPUT_VARS}
    inputs["rust_skia_rev"] = rust_skia_rev(
        (WORKSPACE / "Cargo.lock").read_text(encoding="utf-8")
    )
    inputs["compiler"] = compiler_identity()
    return inputs


def inputs_id(inputs: dict[str, str]) -> str:
    digest = hashlib.sha256(
        json.dumps(inputs, sort_keys=True).encode("utf-8")
    ).hexdigest()
    return f"{inputs['rust_skia_rev'][:12]}-{digest[:16]}"


def require_windows() -> None:
    if os.name != "nt":
        fail(
            "shared prebuilt Skia is Windows-only: on other targets rust-skia "
            "compiles Skia with Cargo's OPT_LEVEL, which its binary key omits"
        )


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def run_cargo(cargo_args: list[str], env: dict[str, str]) -> subprocess.CompletedProcess:
    """Run cargo with JSON messages; echo stderr; return the captured result."""
    result = subprocess.run(
        ["cargo", cargo_args[0], "--message-format=json", *cargo_args[1:]],
        cwd=WORKSPACE,
        env=env,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    sys.stderr.write(result.stderr)
    return result


def requested_key(cargo_args: list[str], env: dict[str, str], missing: Path) -> tuple[str, str]:
    """Ask rust-skia which archive these cargo arguments install.

    The key depends on the Skia features Cargo resolves for this selection,
    less the ones rust-skia drops for the target. Rather than re-derive it,
    force a download from a missing file: the build script prints the tag and
    key, then fails.
    """
    probe = dict(env)
    probe.pop("FORCE_SKIA_BUILD", None)
    probe["FORCE_SKIA_BINARIES_DOWNLOAD"] = "1"
    probe["SKIA_BINARIES_URL"] = f"file://{missing.as_posix()}/{{tag}}-{{key}}.tar.gz"
    result = run_cargo(cargo_args, probe)
    match = TRYING.search(result.stdout + result.stderr)
    if not match:
        fail("rust-skia did not report its binary key; see the cargo output above")
    return match.group(1), match.group(2)


def skia_out_dir(stdout: str) -> Path:
    out_dirs = set()
    for line in stdout.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get("reason") == "build-script-executed" and "skia-bindings" in message.get(
            "package_id", ""
        ):
            out_dirs.add(message["out_dir"])
    if len(out_dirs) != 1:
        fail(f"expected one skia-bindings build, found {sorted(out_dirs)}")
    return Path(out_dirs.pop()) / "skia"


def packed_files(skia_dir: Path) -> list[Path]:
    """The files rust-skia's own export ships: libraries, ICU data, bindings."""
    files = sorted(skia_dir.glob("*.lib"))
    files += [skia_dir / name for name in ("icudtl.dat", "bindings.rs")]
    missing = [path.name for path in files if not path.is_file()]
    if missing or not (skia_dir / "skia.lib").is_file():
        fail(f"incomplete Skia build in {skia_dir}: missing {missing or ['skia.lib']}")
    return files


def cmd_build(args: argparse.Namespace) -> None:
    require_windows()
    inputs = build_inputs(dict(os.environ))
    root = args.cache_dir / inputs_id(inputs)
    cargo_args = args.cargo_args or DEFAULT_CARGO_ARGS
    if cargo_args and cargo_args[0] == "--":
        cargo_args = cargo_args[1:]
    root.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env.pop("FORCE_SKIA_BINARIES_DOWNLOAD", None)
    env.pop("SKIA_BINARIES_URL", None)
    env["CARGO_TARGET_DIR"] = str(args.target_dir)
    print(f"skia_prebuilt: cargo {' '.join(cargo_args)} (target {args.target_dir})", flush=True)

    tag, key = requested_key(cargo_args, env, root / "probe-missing")
    print(f"skia_prebuilt: rust-skia requests {tag}/{key}", flush=True)

    env["FORCE_SKIA_BUILD"] = "1"
    result = run_cargo(cargo_args, env)
    if result.returncode != 0:
        fail(f"cargo failed with status {result.returncode}")
    skia_dir = skia_out_dir(result.stdout)
    files = packed_files(skia_dir)

    archive = root / tag / f"skia-binaries-{key}.tar.gz"
    archive.parent.mkdir(parents=True, exist_ok=True)
    partial = archive.with_name(archive.name + ".partial")
    with tarfile.open(partial, "w:gz") as tar:
        for path in files:
            tar.add(path, arcname=f"skia-binaries/{path.name}")
    os.replace(partial, archive)
    manifest = {
        "inputs": inputs,
        "tag": tag,
        "key": key,
        "cargo_args": cargo_args,
        "source": str(skia_dir),
        "files": {path.name: sha256_file(path) for path in files},
        "archive_sha256": sha256_file(archive),
    }
    archive.with_name(archive.name + ".json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"skia_prebuilt: wrote {archive}", flush=True)


def cmd_env(args: argparse.Namespace) -> None:
    require_windows()
    root = args.cache_dir / inputs_id(build_inputs(dict(os.environ)))
    archives = sorted(root.glob("*/skia-binaries-*.tar.gz"))
    if not archives:
        fail(f"no archive in {root}; run `python tools/skia_prebuilt.py build` first")
    url = f"file://{root.as_posix()}/{{tag}}/skia-binaries-{{key}}.tar.gz"
    if args.shell == "powershell":
        print("Remove-Item Env:FORCE_SKIA_BUILD -ErrorAction SilentlyContinue")
        print("$env:FORCE_SKIA_BINARIES_DOWNLOAD = '1'")
        print(f"$env:SKIA_BINARIES_URL = '{url}'")
    else:
        print("unset FORCE_SKIA_BUILD")
        print("export FORCE_SKIA_BINARIES_DOWNLOAD=1")
        print(f"export SKIA_BINARIES_URL='{url}'")
    for archive in archives:
        print(f"# available: {archive.parent.name}/{archive.name}", file=sys.stderr)


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=None,
        help="archive root (default: <main checkout>/out/skia-prebuilt)",
    )
    sub = parser.add_subparsers(dest="command", required=True)
    build = sub.add_parser("build", help="compile Skia once and archive it")
    build.add_argument(
        "--target-dir",
        type=Path,
        default=None,
        help="Cargo target directory for the one source build (default: <out>/skp)",
    )
    build.add_argument(
        "cargo_args",
        nargs=argparse.REMAINDER,
        help=f"cargo arguments selecting the Skia features (default: {' '.join(DEFAULT_CARGO_ARGS)})",
    )
    env = sub.add_parser("env", help="print variables that install the archive")
    env.add_argument("--shell", choices=("powershell", "sh"), default="powershell")
    args = parser.parse_args(argv)
    out = main_checkout_out()
    if args.cache_dir is None:
        args.cache_dir = out / "skia-prebuilt"
    if args.command == "build":
        if args.target_dir is None:
            args.target_dir = out / "skp"
        cmd_build(args)
    else:
        cmd_env(args)


if __name__ == "__main__":
    main()
