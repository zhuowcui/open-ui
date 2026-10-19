#!/usr/bin/env python3
"""Compile C/C++ consumers and verify the exact Open UI cdylib exports."""

from __future__ import annotations

import argparse
import os
import shutil
import shlex
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
INCLUDE = ROOT / "include"
EXAMPLES = ROOT / "examples/c_v02"
SYMBOLS = ROOT / "docs/v02/generated/openui-ffi-symbols.txt"
WINDOWS = sys.platform == "win32"
DEFAULT_LIBRARY = ROOT / "bindings/rust/target/debug" / (
    "openui_ffi.dll" if WINDOWS else "libopenui_ffi.so"
)
PARITY_RUST_CONFIG = ROOT / "bindings/rust/.cargo/config.chromium.toml"


def run(command: list[str]) -> None:
    subprocess.run(command, check=True, cwd=ROOT)


def dynamic_symbols(library: Path) -> set[str]:
    if WINDOWS:
        return coff_exports(library)
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


def coff_exports(library: Path) -> set[str]:
    """Names in the DLL export table; Windows has no versioned dynamic symbols."""
    output = subprocess.run(
        ["llvm-readobj", "--coff-exports", str(library)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    names = set()
    for line in output.splitlines():
        field, _, value = line.strip().partition(":")
        if field == "Name" and value.strip().startswith("oui_"):
            names.add(value.strip())
    return names


def windows_compilers() -> tuple[str | None, str | None, list[str], list[str], list[str]]:
    """Clang's GNU driver targets MSVC and locates the Windows SDK itself."""
    cc = os.environ.get("CC") or shutil.which("clang")
    cxx = os.environ.get("CXX") or shutil.which("clang++")
    # The consumers use portable C runtime calls that MSVC marks deprecated.
    defines = ["-D_CRT_SECURE_NO_WARNINGS"]
    c_flags = [*defines, *shlex.split(os.environ.get("CFLAGS", ""))]
    cxx_flags = [*defines, *shlex.split(os.environ.get("CXXFLAGS", ""))]
    # Match the 8 MiB Linux main-thread stack; the Windows default is 1 MiB and
    # deep layouts recurse further than that on the consumer's thread.
    link_flags = ["-Wl,/STACK:8388608", *shlex.split(os.environ.get("LDFLAGS", ""))]
    return cc, cxx, c_flags, cxx_flags, link_flags


def compilers() -> tuple[str | None, str | None, list[str], list[str], list[str]]:
    if WINDOWS:
        return windows_compilers()
    system_cc = os.environ.get("CC") or shutil.which("cc")
    system_cxx = os.environ.get("CXX") or shutil.which("c++")
    configured: dict[str, str] = {}
    use_system = bool(system_cc and system_cxx and Path("/usr/include/stdio.h").is_file())
    if not use_system and PARITY_RUST_CONFIG.is_file():
        configured = tomllib.loads(
            PARITY_RUST_CONFIG.read_text(encoding="utf-8")
        ).get("env", {})
    cc = system_cc if use_system else configured.get("CC", system_cc)
    cxx = system_cxx if use_system else configured.get("CXX", system_cxx or cc)
    configured_flags = [] if use_system else shlex.split(configured.get("CXXFLAGS", ""))
    c_flags = ["-pthread", *configured_flags, *shlex.split(os.environ.get("CFLAGS", ""))]
    cxx_flags = ["-pthread", *configured_flags, *shlex.split(os.environ.get("CXXFLAGS", ""))]
    link_flags: list[str] = shlex.split(os.environ.get("LDFLAGS", ""))
    sysroot = None if use_system else configured.get("PKG_CONFIG_SYSROOT_DIR")
    if sysroot:
        gcc = Path(sysroot) / "usr/lib/gcc/x86_64-linux-gnu/10"
        link_flags.extend(
            [
                f"-L{Path(sysroot) / 'usr/lib/x86_64-linux-gnu'}",
                f"-L{gcc}",
                f"-B{gcc}",
            ]
        )
    return cc, cxx, c_flags, cxx_flags, link_flags


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--library", type=Path, default=DEFAULT_LIBRARY)
    parser.add_argument("--skip-run", action="store_true")
    parser.add_argument(
        "--keep-going", action="store_true",
        help="run every linked consumer and fail if any consumer fails",
    )
    args = parser.parse_args()
    cc, cxx, c_flags, cxx_flags, link_flags = compilers()
    inspector = "llvm-readobj" if WINDOWS else "nm"
    if not cc or not cxx or not shutil.which(inspector):
        raise SystemExit(f"a C/C++ compiler and {inspector} are required")
    library = args.library.resolve()
    if not library.is_file():
        raise SystemExit(f"missing cdylib: {library}")
    import_library = library.with_name(library.name + ".lib")
    if WINDOWS and not import_library.is_file():
        raise SystemExit(f"missing import library: {import_library}")

    expected = set(SYMBOLS.read_text(encoding="utf-8").splitlines())
    actual = dynamic_symbols(library)
    if actual != expected:
        raise SystemExit(
            f"export drift: missing={sorted(expected - actual)} extra={sorted(actual - expected)}"
        )

    with tempfile.TemporaryDirectory(prefix="openui-ffi-") as temporary:
        temporary = Path(temporary)
        if WINDOWS:
            # The loader finds the DLL beside each consumer executable.
            runnable_library = temporary / library.name
            shutil.copy2(library, runnable_library)
            before_object: list[str] = []
            after_object = [str(import_library)]
        else:
            runnable_library = temporary / "libopenui.so.0"
            shutil.copy2(library, runnable_library)
            before_object = ["-fuse-ld=lld"]
            after_object = [str(runnable_library), f"-Wl,-rpath,{temporary}"]
        binaries: dict[str, list[Path]] = {"C": [], "C++": []}
        for language, pattern, compiler, standard, flags in (
            ("C", "*.c", cc, "c11", c_flags),
            ("C++", "*.cc", cxx, "c++17", cxx_flags),
        ):
            for source in sorted(EXAMPLES.glob(pattern)):
                # C and C++ consumers may share a stem; keep their outputs apart.
                object_file = temporary / f"{source.name}.o"
                output = temporary / f"{source.name}.{'exe' if WINDOWS else 'bin'}"
                run(
                    [
                        compiler,
                        f"-std={standard}",
                        "-Wall",
                        "-Wextra",
                        "-Werror",
                        *flags,
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
                        *cxx_flags,
                        *link_flags,
                        *before_object,
                        str(object_file),
                        *after_object,
                        "-o",
                        str(output),
                    ]
                )
                binaries[language].append(output)
        failures: list[tuple[str, int]] = []
        if not args.skip_run:
            for consumers in binaries.values():
                for binary in consumers:
                    try:
                        run([str(binary)])
                    except subprocess.CalledProcessError as error:
                        if not args.keep_going:
                            raise
                        failures.append((binary.name, error.returncode))
        if failures:
            raise SystemExit(f"C ABI consumer failures: {failures}")
    print(
        f"C ABI verified: symbols={len(expected)} C_examples={len(binaries['C'])} "
        f"C++={len(binaries['C++'])} ran={not args.skip_run} "
        f"library={library.name}"
    )


if __name__ == "__main__":
    main()
