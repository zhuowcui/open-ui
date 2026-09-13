#!/usr/bin/env python3
"""Compile C/C++ consumers and verify the exact Open UI cdylib exports."""

from __future__ import annotations

import argparse
import os
import shutil
import shlex
import subprocess
import tempfile
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
INCLUDE = ROOT / "include"
EXAMPLES = ROOT / "examples/c_v02"
SYMBOLS = ROOT / "docs/v02/generated/openui-ffi-symbols.txt"
DEFAULT_LIBRARY = ROOT / "bindings/rust/target/debug/libopenui_ffi.so"
RUST_CONFIG = ROOT / "bindings/rust/.cargo/config.toml"


def run(command: list[str]) -> None:
    subprocess.run(command, check=True, cwd=ROOT)


def dynamic_symbols(library: Path) -> set[str]:
    output = subprocess.run(
        ["nm", "-D", "--defined-only", "--format=posix", str(library)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return {
        line.split()[0].split("@", 1)[0]
        for line in output.splitlines()
        if line and line.split()[0].startswith("oui_")
    }


def compilers() -> tuple[str | None, str | None, list[str], list[str]]:
    system_cc = os.environ.get("CC") or shutil.which("cc")
    system_cxx = os.environ.get("CXX") or shutil.which("c++")
    if system_cc and system_cxx and Path("/usr/include/stdio.h").is_file():
        return system_cc, system_cxx, [], []
    configured: dict[str, str] = {}
    if RUST_CONFIG.is_file():
        configured = tomllib.loads(RUST_CONFIG.read_text(encoding="utf-8")).get("env", {})
    cc = configured.get("CC", system_cc)
    cxx = configured.get("CXX", system_cxx or cc)
    flags = shlex.split(configured.get("CXXFLAGS", ""))
    link_flags: list[str] = []
    sysroot = configured.get("PKG_CONFIG_SYSROOT_DIR")
    if sysroot:
        gcc = Path(sysroot) / "usr/lib/gcc/x86_64-linux-gnu/10"
        link_flags.extend(
            [
                f"-L{Path(sysroot) / 'usr/lib/x86_64-linux-gnu'}",
                f"-L{gcc}",
                f"-B{gcc}",
            ]
        )
    return cc, cxx, flags, link_flags


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--library", type=Path, default=DEFAULT_LIBRARY)
    parser.add_argument("--skip-run", action="store_true")
    args = parser.parse_args()
    cc, cxx, toolchain_flags, link_flags = compilers()
    if not cc or not cxx or not shutil.which("nm"):
        raise SystemExit("a C/C++ compiler and nm are required")
    library = args.library.resolve()
    if not library.is_file():
        raise SystemExit(f"missing cdylib: {library}")

    expected = set(SYMBOLS.read_text(encoding="utf-8").splitlines())
    actual = dynamic_symbols(library)
    if actual != expected:
        raise SystemExit(
            f"export drift: missing={sorted(expected - actual)} extra={sorted(actual - expected)}"
        )

    with tempfile.TemporaryDirectory(prefix="openui-ffi-") as temporary:
        temporary = Path(temporary)
        cpp = temporary / "header_smoke.o"
        run(
            [
                cxx,
                "-x",
                "c++",
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-Werror",
                *toolchain_flags,
                f"-I{INCLUDE}",
                str(EXAMPLES / "header_smoke.cc"),
                "-c",
                "-o",
                str(cpp),
            ]
        )
        binaries = []
        for source in sorted(EXAMPLES.glob("*.c")):
            object_file = temporary / f"{source.stem}.o"
            output = temporary / source.stem
            run(
                [
                    cc,
                    "-std=c11",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    *toolchain_flags,
                    f"-I{INCLUDE}",
                    str(source),
                    "-c",
                    "-o",
                    str(object_file),
                ]
            )
            run(
                [
                    cxx,
                    *toolchain_flags,
                    *link_flags,
                    "-fuse-ld=lld",
                    str(object_file),
                    str(library),
                    f"-Wl,-rpath,{library.parent}",
                    "-o",
                    str(output),
                ]
            )
            binaries.append(output)
        if not args.skip_run:
            for binary in binaries:
                run([str(binary)])
    print(
        f"C ABI verified: symbols={len(expected)} C_examples=4 C++=1 "
        f"library={library.name}"
    )


if __name__ == "__main__":
    main()
