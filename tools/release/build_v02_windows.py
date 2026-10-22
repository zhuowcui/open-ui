#!/usr/bin/env python3
"""Build a deterministic Open UI v0.2 Windows x64 MSVC SDK archive.

The archive holds the C headers, the `openui_ffi.dll` runtime with its import
library and PDB, a relocatable CMake package, the C and Rust examples, the
license texts and an SPDX SBOM. `--smoke` extracts the archive into a fresh
directory and builds and runs a C consumer through `find_package(OpenUI)`.

Windows is an experimental development platform; this archive is not a v0.2
release artifact.
"""

from __future__ import annotations

import argparse
import datetime
import json
import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_v02_linux as common  # noqa: E402

ROOT = common.ROOT
RUST = common.RUST
VERSION = common.VERSION
TARGET = "x86_64-pc-windows-msvc"
RUST_EXAMPLES = ("hello", "counter", "todo", "dashboard", "accessibility")


def build_libraries(epoch: int) -> Path:
    env = dict(os.environ)
    env["SOURCE_DATE_EPOCH"] = str(epoch)
    env.setdefault("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1")
    env.setdefault("CARGO_PROFILE_RELEASE_LTO", "thin")
    common.run(
        ["cargo", "build", "--release", "--locked", "--target", TARGET,
         "--package", "openui-ffi", "--features", "windows"],
        cwd=RUST,
        env=env,
    )
    target_root = Path(env.get("CARGO_TARGET_DIR", RUST / "target"))
    if not target_root.is_absolute():
        target_root = RUST / target_root
    return target_root / TARGET / "release"


def locate_libraries(library_dir: Path) -> tuple[Path, Path, Path | None]:
    dll = library_dir / "openui_ffi.dll"
    import_library = library_dir / "openui_ffi.dll.lib"
    missing = [str(path) for path in (dll, import_library) if not path.is_file()]
    if missing:
        raise SystemExit("missing built libraries: " + ", ".join(missing))
    pdb = library_dir / "openui_ffi.pdb"
    return dll, import_library, pdb if pdb.is_file() else None


def stage_sdk(stage: Path, library_dir: Path, metadata: dict, epoch: int, revision: str) -> None:
    dll, import_library, pdb = locate_libraries(library_dir)
    backports = common.dependency_backports(metadata)
    include = stage / "include/openui"
    bin_dir = stage / "bin"
    lib = stage / "lib"
    share = stage / "share/openui"
    for directory in (include, bin_dir, lib, share):
        directory.mkdir(parents=True, exist_ok=True)

    shutil.copy2(ROOT / "include/openui.h", include / "openui.h")
    shutil.copy2(ROOT / "include/openui_style_properties.h", include / "openui_style_properties.h")
    shutil.copy2(dll, bin_dir / "openui_ffi.dll")
    # The import library names the DLL it binds to; renaming only the file
    # keeps that binding.
    shutil.copy2(import_library, lib / "openui_ffi.lib")
    if pdb is not None:
        (stage / "debug").mkdir(parents=True, exist_ok=True)
        shutil.copy2(pdb, stage / "debug/openui_ffi.pdb")

    common.write_text(
        lib / "cmake/OpenUI/OpenUIConfig.cmake",
        f"""# Open UI {VERSION} relocatable package configuration (Windows x64 MSVC).
get_filename_component(_OPENUI_PREFIX "${{CMAKE_CURRENT_LIST_DIR}}/../../.." ABSOLUTE)
add_library(OpenUI::OpenUI SHARED IMPORTED)
set_target_properties(OpenUI::OpenUI PROPERTIES
  IMPORTED_LOCATION "${{_OPENUI_PREFIX}}/bin/openui_ffi.dll"
  IMPORTED_IMPLIB "${{_OPENUI_PREFIX}}/lib/openui_ffi.lib"
  INTERFACE_INCLUDE_DIRECTORIES "${{_OPENUI_PREFIX}}/include/openui")
set(OpenUI_RUNTIME_DLL "${{_OPENUI_PREFIX}}/bin/openui_ffi.dll")
set(OpenUI_VERSION "{VERSION}")
unset(_OPENUI_PREFIX)
""",
    )
    common.write_text(
        lib / "cmake/OpenUI/OpenUIConfigVersion.cmake",
        f"""set(PACKAGE_VERSION "{VERSION}")
if(PACKAGE_FIND_VERSION VERSION_GREATER PACKAGE_VERSION)
  set(PACKAGE_VERSION_COMPATIBLE FALSE)
else()
  set(PACKAGE_VERSION_COMPATIBLE TRUE)
  if(PACKAGE_FIND_VERSION VERSION_EQUAL PACKAGE_VERSION)
    set(PACKAGE_VERSION_EXACT TRUE)
  endif()
endif()
if(NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
  set(PACKAGE_VERSION_UNSUITABLE TRUE)
endif()
""",
    )

    shutil.copytree(ROOT / "examples/c_v02", share / "examples/c", dirs_exist_ok=True)
    common.stage_dependency_backports(share, backports)
    for example in RUST_EXAMPLES:
        destination = share / "examples/rust" / example
        destination.mkdir(parents=True, exist_ok=True)
        common.write_text(
            destination / "Cargo.toml",
            f"""[package]
name = "openui-{example}-example"
version = "{VERSION}"
edition = "2021"
publish = false

[features]
default = []
windows = ["openui/windows"]

[dependencies]
openui = "={VERSION}"
""" + common.rust_example_backport_patches(backports),
        )
        shutil.copytree(RUST / "examples" / example / "src", destination / "src", dirs_exist_ok=True)
    (share / "licenses").mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / "LICENSE", share / "licenses/Apache-2.0.txt")
    shutil.copytree(
        RUST / "openui-text/fonts",
        share / "licenses/fonts",
        dirs_exist_ok=True,
        ignore=shutil.ignore_patterns("*.ttf"),
    )
    shutil.copy2(ROOT / "docs/v02/unsupported-features.md", share / "UNSUPPORTED.md")
    shutil.copy2(ROOT / "docs/v02/migration-v01-v02.md", share / "MIGRATION.md")
    shutil.copy2(ROOT / "CHANGELOG.md", share / "CHANGELOG.md")
    common.write_text(
        share / "BUILD-METADATA.json",
        json.dumps(
            {
                "abi_checksum": (ROOT / "docs/v02/generated/openui-ffi-checksum.txt")
                .read_text(encoding="utf-8").strip(),
                "release_qualification": False,
                "revision": revision,
                "source_date_epoch": epoch,
                "target": TARGET,
                "version": VERSION,
            },
            indent=2,
            sort_keys=True,
        ) + "\n",
    )
    common.write_text(
        share / "openui.spdx.json",
        json.dumps(common.make_sbom(metadata, TARGET, revision, epoch), indent=2, sort_keys=True) + "\n",
    )


def deterministic_zip(source: Path, output: Path, top_level: str, epoch: int) -> None:
    """Zip `source` with sorted entries, fixed times and fixed permissions."""
    # ZIP timestamps start at 1980 and carry no time zone.
    stamp = datetime.datetime.fromtimestamp(max(epoch, 315532800), datetime.timezone.utc)
    date_time = stamp.timetuple()[:6]
    output.parent.mkdir(parents=True, exist_ok=True)
    entries = sorted(source.rglob("*"), key=lambda path: path.relative_to(source).as_posix())
    with zipfile.ZipFile(output, "w") as archive:
        for path in entries:
            name = f"{top_level}/{path.relative_to(source).as_posix()}"
            if path.is_dir():
                info = zipfile.ZipInfo(name + "/", date_time)
                info.external_attr = (0o40755 << 16) | 0x10
                archive.writestr(info, b"")
                continue
            info = zipfile.ZipInfo(name, date_time)
            info.external_attr = 0o100644 << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, path.read_bytes(), compresslevel=9)


def smoke(archive: Path) -> None:
    """Build and run the hello C example against the extracted SDK."""
    cmake = shutil.which("cmake")
    if cmake is None:
        raise SystemExit("--smoke requires CMake")
    with tempfile.TemporaryDirectory(prefix="openui-sdk-smoke-") as temporary:
        root = Path(temporary)
        with zipfile.ZipFile(archive) as source:
            source.extractall(root / "sdk")
        prefix = next((root / "sdk").iterdir())
        project = root / "consumer"
        project.mkdir()
        examples = prefix / "share/openui/examples/c"
        for name in ["hello.c", *(path.name for path in examples.glob("*.h"))]:
            shutil.copy2(examples / name, project / name)
        common.write_text(
            project / "CMakeLists.txt",
            """cmake_minimum_required(VERSION 3.20)
project(openui_sdk_smoke C)
find_package(OpenUI 0.2 REQUIRED CONFIG)
add_executable(hello hello.c)
target_link_libraries(hello PRIVATE OpenUI::OpenUI)
target_compile_definitions(hello PRIVATE _CRT_SECURE_NO_WARNINGS)
add_custom_command(TARGET hello POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_if_different ${OpenUI_RUNTIME_DLL} $<TARGET_FILE_DIR:hello>)
""",
        )
        build = root / "build"
        common.run([cmake, "-S", str(project), "-B", str(build), "-G", "Ninja",
                    "-DCMAKE_C_COMPILER=clang-cl", f"-DCMAKE_PREFIX_PATH={prefix}",
                    "-DCMAKE_BUILD_TYPE=Release"])
        common.run([cmake, "--build", str(build)])
        output = common.run([str(build / "hello.exe")], cwd=build)
        print(f"sdk smoke: hello.exe ran from the extracted archive ({len(output)} bytes of output)")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--library-dir", type=Path, help="use a prebuilt openui-ffi DLL directory")
    parser.add_argument("--source-date-epoch", type=int)
    parser.add_argument("--smoke", action="store_true", help="build and run a consumer from the archive")
    parser.add_argument("--allow-dirty", action="store_true",
                        help="allow a local archive from an uncommitted tree")
    args = parser.parse_args()

    metadata = common.verify_source()
    dirty = bool(common.run(["git", "status", "--porcelain"], echo_output=False).strip())
    if dirty and not args.allow_dirty:
        raise SystemExit("SDK archives require a clean worktree (or --allow-dirty for local testing)")
    epoch = common.source_epoch(args.source_date_epoch)
    revision = common.git_revision()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    library_dir = args.library_dir.resolve() if args.library_dir else build_libraries(epoch)
    archive = output / f"openui-sdk-{VERSION}-{TARGET}.zip"
    with tempfile.TemporaryDirectory(prefix="openui-sdk-") as temporary:
        stage = Path(temporary) / "sdk"
        stage_sdk(stage, library_dir, metadata, epoch, revision)
        deterministic_zip(stage, archive, f"openui-sdk-{VERSION}-{TARGET}", epoch)
    common.write_text(archive.with_suffix(".zip.sha256"), f"{common.sha256(archive)}  {archive.name}\n")
    print(f"artifact: {archive} sha256={common.sha256(archive)}")
    if args.smoke:
        smoke(archive)


if __name__ == "__main__":
    main()
