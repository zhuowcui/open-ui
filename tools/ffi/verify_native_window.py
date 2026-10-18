#!/usr/bin/env python3
"""Build and run native C/C++ consumers through the shared Linux application loop."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

from verify_abi import DEFAULT_LIBRARY, INCLUDE, ROOT, compilers


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--library", type=Path, default=DEFAULT_LIBRARY)
    parser.add_argument("--backend", choices=("software", "opengl"), default="software")
    parser.add_argument("--protocol", choices=("x11", "wayland"), required=True)
    parser.add_argument("--report", type=Path, help="write a source-bound functional smoke report")
    args = parser.parse_args()
    cc, cxx, c_flags, cxx_flags, link_flags = compilers()
    if not cc or not cxx:
        raise SystemExit("a C/C++ compiler is required")
    environment = dict(os.environ)
    if args.protocol == "x11":
        if not environment.get("DISPLAY"):
            raise SystemExit("X11 smoke requires DISPLAY")
        environment.pop("WAYLAND_DISPLAY", None)
        environment.pop("WAYLAND_SOCKET", None)
    else:
        if not environment.get("WAYLAND_DISPLAY") or not environment.get("XDG_RUNTIME_DIR"):
            raise SystemExit("Wayland smoke requires WAYLAND_DISPLAY and XDG_RUNTIME_DIR")
        environment.pop("DISPLAY", None)
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
                              capture_output=True, text=True).stdout.strip()
    dirty = subprocess.run(["git", "status", "--porcelain", "--untracked-files=all"],
                           cwd=ROOT, check=True, capture_output=True, text=True).stdout.splitlines()
    report = {
        "schema_version": 1,
        "release_qualification": False,
        "kind": "native_c_cpp_window_functional_smoke",
        "source_commit": revision,
        "source_clean": not dirty,
        "source_changes": dirty,
        "protocol": args.protocol,
        "backend": args.backend,
        "library_sha256": digest(args.library.resolve()),
        "header_sha256": digest(INCLUDE / "openui.h"),
        "tool_sha256": digest(Path(__file__).resolve()),
        "consumers": [],
        "passed": False,
    }
    with tempfile.TemporaryDirectory(prefix="openui-native-consumers-") as directory:
        temporary = Path(directory)
        library = temporary / "libopenui.so.0"
        shutil.copy2(args.library.resolve(), library)
        for suffix, compiler, flags, standard in (
            ("c", cc, c_flags, "c11"), ("cc", cxx, cxx_flags, "c++17")
        ):
            source = ROOT / "examples/c_v02/native" / f"window.{suffix}"
            object_file = temporary / f"window-{suffix}.o"
            binary = temporary / f"window-{suffix}"
            subprocess.run([compiler, f"-std={standard}", "-Wall", "-Wextra", "-Werror",
                            *flags, f"-I{INCLUDE}", str(source), "-c", "-o", str(object_file)],
                           check=True)
            subprocess.run([cxx, *cxx_flags, *link_flags, "-fuse-ld=lld", str(object_file),
                            str(library), f"-Wl,-rpath,{temporary}", "-o", str(binary)], check=True)
            consumer = {
                "source": str(source.relative_to(ROOT)),
                "source_sha256": digest(source),
                "binary_sha256": digest(binary),
                "compiler": str(compiler),
                "returncode": None,
                "timed_out": False,
                "stdout": "",
                "stderr": "",
            }
            try:
                result = subprocess.run(
                    [str(binary), "2" if args.backend == "software" else "1"],
                    env=environment, capture_output=True, text=True, timeout=45,
                )
                consumer.update(returncode=result.returncode, stdout=result.stdout, stderr=result.stderr)
            except subprocess.TimeoutExpired as error:
                consumer["timed_out"] = True
                consumer["stdout"] = (error.stdout or b"").decode(errors="replace")
                consumer["stderr"] = (error.stderr or b"").decode(errors="replace")
            report["consumers"].append(consumer)
            print(consumer["stdout"], end="")
            print(consumer["stderr"], end="")
    report["passed"] = all(consumer["returncode"] == 0 for consumer in report["consumers"])
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if not report["passed"]:
        raise SystemExit("native C/C++ window consumers failed")
    print(f"native C/C++ window consumers passed: protocol={args.protocol} backend={args.backend}")


if __name__ == "__main__":
    main()
