#!/usr/bin/env python3
"""Build deterministic Open UI v0.2 Linux SDK and native packages."""

from __future__ import annotations

import argparse
import datetime
import gzip
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RUST = ROOT / "bindings/rust"
VERSION = "0.2.0"
SUPPORTED_TARGETS = {
    "x86_64-unknown-linux-gnu": ("amd64", "x86_64"),
    "aarch64-unknown-linux-gnu": ("arm64", "aarch64"),
}
PUBLIC_CRATES = (
    "openui-geometry",
    "openui-style",
    "openui-dom",
    "openui-text",
    "openui-layout",
    "openui-paint",
    "openui-compositor",
    "openui-engine",
    "openui-platform",
    "openui-macros",
    "openui",
)


def run(
    command: list[str],
    *,
    cwd: Path = ROOT,
    env: dict[str, str] | None = None,
    echo_output: bool = True,
    combine_output: bool = True,
) -> str:
    print("+", " ".join(command), flush=True)
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT if combine_output else subprocess.PIPE,
    )
    if completed.stdout and (echo_output or completed.returncode != 0):
        print(completed.stdout, end="")
    if completed.stderr and completed.returncode != 0:
        print(completed.stderr, end="")
    if completed.returncode != 0:
        raise subprocess.CalledProcessError(
            completed.returncode, command, output=completed.stdout
        )
    return completed.stdout


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_dependency_security() -> Path:
    """Every artifact build gets fresh security evidence, including local builds."""
    evidence_root = Path(
        os.environ.get("OPENUI_DEPENDENCY_SECURITY_EVIDENCE_ROOT", tempfile.gettempdir())
    )
    evidence_root.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="openui-dependency-security-", dir=evidence_root)) / "report"
    run([sys.executable, "tools/release/check_dependency_security.py", "--output", str(evidence)])
    return evidence


def write_text(path: Path, value: str, mode: int = 0o644) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value, encoding="utf-8", newline="\n")
    path.chmod(mode)


def normalize_tree(root: Path, epoch: int) -> None:
    # Symlinks are skipped, so following is only refused where the platform
    # supports it; Windows has no lutime equivalent.
    follow = os.utime not in os.supports_follow_symlinks
    for path in sorted(root.rglob("*"), reverse=True):
        if path.is_symlink():
            continue
        path.chmod(0o755 if path.is_dir() else 0o644)
        os.utime(path, (epoch, epoch), follow_symlinks=follow)
    os.utime(root, (epoch, epoch), follow_symlinks=follow)


def verify_source() -> dict:
    for generator in (
        [sys.executable, "tools/release/generate_v02_contract.py", "--check"],
        [sys.executable, "tools/style/generate_properties.py", "--check"],
        [sys.executable, "tools/ffi/generate_ffi.py", "--check"],
        [sys.executable, "tools/conformance/verify_v02.py"],
        [sys.executable, "tools/performance/verify_v02.py"],
    ):
        run(generator)
    metadata = json.loads(
        run(
            ["cargo", "metadata", "--locked", "--format-version", "1",
             "--features", "openui-ffi/linux"],
            cwd=RUST,
            echo_output=False,
            combine_output=False,
        )
    )
    packages = {package["name"]: package for package in metadata["packages"]}
    errors = []
    for name in PUBLIC_CRATES:
        package = packages.get(name)
        if package is None:
            errors.append(f"missing public crate {name}")
        elif package["version"] != VERSION:
            errors.append(f"{name} is {package['version']}, expected {VERSION}")
        elif package.get("publish") == []:
            errors.append(f"{name} is marked publish=false")
        elif package.get("license") != "Apache-2.0":
            errors.append(f"{name} does not declare Apache-2.0")
    ffi = packages.get("openui-ffi")
    if ffi is None or ffi.get("publish") != [] or ffi.get("version") != VERSION:
        errors.append("openui-ffi must be a non-crates.io 0.2.0 native artifact")
    if errors:
        raise SystemExit("release source verification failed:\n- " + "\n- ".join(errors))
    print(f"release source: {len(PUBLIC_CRATES)} public crates at {VERSION}; native ABI frozen")
    return metadata


def source_epoch(value: int | None) -> int:
    if value is not None:
        return value
    configured = os.environ.get("SOURCE_DATE_EPOCH")
    if configured:
        return int(configured)
    return int(run(["git", "show", "-s", "--format=%ct", "HEAD"]).strip())


def git_revision() -> str:
    return run(["git", "rev-parse", "HEAD"]).strip()


def build_libraries(target: str, epoch: int) -> Path:
    env = dict(os.environ)
    env["SOURCE_DATE_EPOCH"] = str(epoch)
    env.setdefault("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1")
    env.setdefault("CARGO_PROFILE_RELEASE_LTO", "thin")
    run(
        [
            "cargo",
            "build",
            "--release",
            "--locked",
            "--target",
            target,
            "--package",
            "openui-ffi",
            "--features",
            "linux",
        ],
        cwd=RUST,
        env=env,
    )
    target_root = Path(env.get("CARGO_TARGET_DIR", RUST / "target"))
    if not target_root.is_absolute():
        target_root = RUST / target_root
    return target_root / target / "release"


def locate_libraries(library_dir: Path) -> tuple[Path, Path]:
    static = library_dir / "libopenui_ffi.a"
    shared = library_dir / "libopenui_ffi.so"
    missing = [str(path) for path in (static, shared) if not path.is_file()]
    if missing:
        raise SystemExit("missing built libraries: " + ", ".join(missing))
    return static, shared


def split_debug_symbols(shared: Path, debug: Path) -> None:
    objcopy = shutil.which("objcopy")
    strip = shutil.which("strip")
    if not objcopy or not strip:
        raise SystemExit("objcopy and strip are required to build an SDK")
    run([objcopy, "--only-keep-debug", str(shared), str(debug)])
    run([strip, "--strip-debug", str(shared)])
    run([objcopy, f"--add-gnu-debuglink={debug}", str(shared)])


def assert_native_dependencies(shared: Path) -> None:
    readelf = shutil.which("readelf")
    if not readelf:
        raise SystemExit("readelf is required to verify native dependencies")
    dynamic = run([readelf, "-d", str(shared)], echo_output=False)
    forbidden = ("chromium", "content_shell", "blink_core", "libcef")
    lowered = dynamic.lower()
    found = [name for name in forbidden if name in lowered]
    if found:
        raise SystemExit("unsupported browser dependency in SDK: " + ", ".join(found))


def spdx_id(name: str, version: str) -> str:
    return "SPDXRef-Package-" + re.sub(r"[^A-Za-z0-9.-]", "-", f"{name}-{version}")


def original_backport_path(value: str | Path) -> Path:
    """Admit absolute paths without erasing parent traversal components."""
    path = Path(value)
    if not path.is_absolute() or ".." in path.parts:
        raise SystemExit("noncanonical dependency backport path: " + str(value))
    return path


def reject_backport_symlinks(path: Path) -> None:
    """Check the caller's path before resolve can erase a symlink component."""
    for component in (*reversed(path.parents), path):
        if component.is_symlink():
            raise SystemExit("symlink in dependency backport path: " + str(component))


def dependency_backports(metadata: dict, rust_root: Path | None = None) -> list[dict]:
    """Verify root-declared backports and discovered owned vendor inventories."""
    root = original_backport_path(rust_root or RUST)
    root_manifest = root / "Cargo.toml"
    reject_backport_symlinks(root_manifest)
    root_data = tomllib.loads(root_manifest.read_text(encoding="utf-8"))
    required = root_data.get("workspace", {}).get("metadata", {}).get(
        "openui", {}
    ).get("dependency-backports", {})
    if not isinstance(required, dict):
        raise SystemExit("invalid required dependency backport declarations")
    vendor = root / "vendor"
    declarations = {}
    for name, declaration in required.items():
        if not isinstance(declaration, dict) or set(declaration) != {
            "version", "path", "license", "provenance-sha256",
            "source-inventory-sha256", "upstream-archive-sha256",
        }:
            raise SystemExit("invalid required dependency backport: " + name)
        relative = Path(declaration["path"])
        if (
            relative.is_absolute() or ".." in relative.parts
            or relative.as_posix() != declaration["path"]
            or "\\" in declaration["path"]
            or not relative.parts or relative.parts[0] != "vendor"
            or any(not re.fullmatch(r"[0-9a-f]{64}", declaration[key]) for key in (
                "provenance-sha256", "source-inventory-sha256", "upstream-archive-sha256"
            ))
        ):
            raise SystemExit("invalid required dependency backport path/digest: " + name)
        manifest = root / relative / "Cargo.toml"
        reject_backport_symlinks(manifest)
        provenance_path = manifest.parent / "OPENUI-BACKPORT.json"
        reject_backport_symlinks(provenance_path)
        if not manifest.is_file() or not provenance_path.is_file():
            raise SystemExit("missing required dependency backport manifest/provenance: " + name)
        matches = [package for package in metadata["packages"] if package["name"] == name]
        selected_manifest = None
        if len(matches) == 1 and matches[0].get("manifest_path"):
            selected_manifest = original_backport_path(matches[0]["manifest_path"])
            reject_backport_symlinks(selected_manifest)
        if len(matches) != 1 or (
            matches[0].get("source")
            or matches[0]["version"] != declaration["version"]
            or matches[0].get("license") != declaration["license"]
            or not matches[0].get("manifest_path")
            or selected_manifest != manifest
        ):
            raise SystemExit("required dependency backport not selected: " + name)
        declarations[manifest] = declaration
    backports = []
    for package in metadata["packages"]:
        if package.get("source") or not package.get("manifest_path"):
            continue
        manifest = original_backport_path(package["manifest_path"])
        # Keep ordinary preexisting vendors supported. Any owned vendor path is
        # checked in its original form, without resolving away linked components.
        if not manifest.is_relative_to(vendor):
            continue
        reject_backport_symlinks(manifest)
        source = manifest.parent
        provenance_path = source / "OPENUI-BACKPORT.json"
        declaration = declarations.get(manifest)
        if not provenance_path.exists():
            if declaration is not None:
                raise SystemExit("missing required dependency backport provenance: " + str(source))
            continue

        provenance_bytes = provenance_path.read_bytes()
        if declaration is not None and (
            hashlib.sha256(provenance_bytes).hexdigest() != declaration["provenance-sha256"]
        ):
            raise SystemExit("required dependency backport provenance drift: " + str(source))
        provenance = json.loads(provenance_bytes)
        expected = provenance["package"]
        if (
            provenance["schema_version"] != 1
            or manifest.name != "Cargo.toml"
            or expected["name"] != package["name"]
            or expected["version"] != package["version"]
            or expected["license"] != package.get("license")
            or provenance_path.is_symlink()
        ):
            raise SystemExit("invalid dependency backport identity: " + str(source))
        if any(path.is_symlink() for path in source.rglob("*")):
            raise SystemExit("symlink in dependency backport: " + str(source))
        inventory = []
        for entry in provenance["files"]:
            relative = Path(entry["path"])
            if (
                relative.is_absolute()
                or relative.as_posix() != entry["path"]
                or ".." in relative.parts
                or "\\" in entry["path"]
                or not re.fullmatch(r"[0-9a-f]{64}", entry["sha256"])
            ):
                raise SystemExit("invalid dependency backport inventory: " + str(source))
            path = source / relative
            if any((source / Path(*relative.parts[:i])).is_symlink()
                   for i in range(1, len(relative.parts) + 1)):
                raise SystemExit("symlink in dependency backport: " + str(path))
            if not path.is_file() or sha256(path) != entry["sha256"]:
                raise SystemExit("dependency backport source drift: " + str(path))
            inventory.append({"path": entry["path"], "sha256": entry["sha256"]})
        paths = [entry["path"] for entry in inventory]
        actual_paths = sorted(
            path.relative_to(source).as_posix()
            for path in source.rglob("*") if path.is_file()
            and path != provenance_path
        )
        if paths != sorted(set(paths)) or paths != actual_paths:
            raise SystemExit("incomplete dependency backport inventory: " + str(source))
        digest = hashlib.sha256(
            json.dumps(inventory, sort_keys=True, separators=(",", ":")).encode("utf-8")
        ).hexdigest()
        if digest != provenance["source_inventory_sha256"]:
            raise SystemExit("dependency backport inventory digest drift: " + str(source))
        license_file = provenance["license_file"]
        if license_file not in paths or "Cargo.toml" not in paths:
            raise SystemExit("missing dependency backport manifest/license: " + str(source))
        upstream = provenance["upstream"]
        if not re.fullmatch(r"[0-9a-f]{64}", upstream["archive_sha256"]):
            raise SystemExit("invalid upstream backport archive digest: " + str(source))
        if declaration is not None and (
            digest != declaration["source-inventory-sha256"]
            or upstream["archive_sha256"] != declaration["upstream-archive-sha256"]
        ):
            raise SystemExit("required dependency backport source identity drift: " + str(source))
        backports.append({
            "name": package["name"], "version": package["version"],
            "directory": source,
            "license_file": license_file,
            "source_inventory_sha256": digest,
            "provenance_sha256": hashlib.sha256(provenance_bytes).hexdigest(),
            "upstream": upstream,
            "changes": provenance["changes"],
        })
    return sorted(backports, key=lambda item: (item["name"], item["version"]))


def stage_dependency_backports(share: Path, backports: list[dict]) -> None:
    records = []
    for backport in backports:
        name = f"{backport['name']}-{backport['version']}"
        destination = share / "sources/backports" / name
        shutil.copytree(backport["directory"], destination)
        license_path = share / "licenses" / f"{name}-LICENSE.txt"
        license_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(backport["directory"] / backport["license_file"], license_path)
        records.append({
            key: value for key, value in backport.items()
            if key not in {"directory", "license_file"}
        } | {"source_path": f"sources/backports/{name}",
             "license_path": f"licenses/{name}-LICENSE.txt"})
    write_text(share / "DEPENDENCY-BACKPORTS.json",
               json.dumps(records, indent=2, sort_keys=True) + "\n")


def rust_example_backport_patches(backports: list[dict]) -> str:
    if not backports:
        return ""
    return "\n[patch.crates-io]\n" + "".join(
        f"{entry['name']} = {{ path = \"../../../sources/backports/"
        f"{entry['name']}-{entry['version']}\" }}\n"
        for entry in backports
    )


def make_sbom(metadata: dict, target: str, revision: str, epoch: int) -> dict:
    packages = []
    relationships = []
    root_id = "SPDXRef-DOCUMENT"
    backports = {
        (entry["name"], entry["version"]): entry
        for entry in dependency_backports(metadata)
    }
    for package in sorted(metadata["packages"], key=lambda item: (item["name"], item["version"])):
        package_id = spdx_id(package["name"], package["version"])
        source = package.get("source") or "NOASSERTION"
        declared = package.get("license") or "NOASSERTION"
        record = {
            "SPDXID": package_id,
            "name": package["name"],
            "versionInfo": package["version"],
            "downloadLocation": source,
            "filesAnalyzed": False,
            "licenseConcluded": "NOASSERTION",
            "licenseDeclared": declared,
            "copyrightText": "NOASSERTION",
        }
        if package.get("checksum"):
            record["checksums"] = [
                {"algorithm": "SHA256", "checksumValue": package["checksum"]}
            ]
        if backport := backports.get((package["name"], package["version"])):
            upstream = backport["upstream"]
            record["sourceInfo"] = (
                "Open UI local dependency backport. SHA256 below identifies the "
                "canonical source inventory, not the unmodified registry archive. "
                f"Provenance SHA256: {backport['provenance_sha256']}; "
                f"upstream archive: {upstream['archive_url']}; "
                f"upstream archive SHA256: {upstream['archive_sha256']}."
            )
            record["checksums"] = [{
                "algorithm": "SHA256",
                "checksumValue": backport["source_inventory_sha256"],
            }]
        packages.append(record)
        relationships.append(
            {
                "spdxElementId": root_id,
                "relationshipType": "DESCRIBES",
                "relatedSpdxElement": package_id,
            }
        )
    namespace_revision = revision if revision != "UNKNOWN" else "source"
    return {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": root_id,
        "name": f"Open UI {VERSION} {target}",
        "documentNamespace": (
            "https://github.com/zhuowcui/open-ui/spdx/"
            f"{VERSION}/{target}/{namespace_revision}"
        ),
        "creationInfo": {
            "creators": ["Tool: tools/release/build_v02_linux.py"],
            "created": datetime.datetime.fromtimestamp(
                epoch, tz=datetime.timezone.utc
            ).strftime("%Y-%m-%dT%H:%M:%SZ"),
        },
        "packages": packages,
        "relationships": relationships,
    }


def stage_sdk(
    stage: Path,
    target: str,
    library_dir: Path,
    metadata: dict,
    epoch: int,
    revision: str,
) -> None:
    static_source, shared_source = locate_libraries(library_dir)
    backports = dependency_backports(metadata)
    include = stage / "include/openui"
    lib = stage / "lib"
    debug = stage / "debug"
    share = stage / "share/openui"
    for directory in (include, lib, debug, share):
        directory.mkdir(parents=True, exist_ok=True)

    shutil.copy2(ROOT / "include/openui.h", include / "openui.h")
    shutil.copy2(
        ROOT / "include/openui_style_properties.h",
        include / "openui_style_properties.h",
    )
    shutil.copy2(static_source, lib / "libopenui.a")
    shutil.copy2(shared_source, lib / f"libopenui.so.{VERSION}")
    os.symlink(f"libopenui.so.{VERSION}", lib / "libopenui.so.0")
    os.symlink("libopenui.so.0", lib / "libopenui.so")
    split_debug_symbols(lib / f"libopenui.so.{VERSION}", debug / "libopenui.so.debug")
    assert_native_dependencies(lib / f"libopenui.so.{VERSION}")

    write_text(
        lib / "pkgconfig/openui.pc",
        f"""prefix=${{pcfiledir}}/../..
exec_prefix=${{prefix}}
libdir=${{prefix}}/lib
includedir=${{prefix}}/include

Name: Open UI
Description: Typed retained Linux and headless UI engine
Version: {VERSION}
Libs: -L${{libdir}} -lopenui
Cflags: -I${{includedir}}/openui
""",
    )
    write_text(
        lib / "cmake/OpenUI/OpenUIConfig.cmake",
        f"""# Open UI {VERSION} relocatable package configuration.
get_filename_component(_OPENUI_PREFIX "${{CMAKE_CURRENT_LIST_DIR}}/../../.." ABSOLUTE)
add_library(OpenUI::OpenUI SHARED IMPORTED)
set_target_properties(OpenUI::OpenUI PROPERTIES
  IMPORTED_LOCATION "${{_OPENUI_PREFIX}}/lib/libopenui.so.{VERSION}"
  IMPORTED_SONAME "libopenui.so.0"
  INTERFACE_INCLUDE_DIRECTORIES "${{_OPENUI_PREFIX}}/include/openui")
set(OpenUI_VERSION "{VERSION}")
unset(_OPENUI_PREFIX)
""",
    )
    write_text(
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
""",
    )

    shutil.copytree(ROOT / "examples/c_v02", share / "examples/c", dirs_exist_ok=True)
    stage_dependency_backports(share, backports)
    for example in ("hello", "counter", "todo", "dashboard"):
        destination = share / "examples/rust" / example
        destination.mkdir(parents=True, exist_ok=True)
        write_text(
            destination / "Cargo.toml",
            f"""[package]
name = "openui-{example}-example"
version = "{VERSION}"
edition = "2021"
publish = false

[features]
default = []
linux = ["openui/linux"]

[dependencies]
openui = "={VERSION}"
""" + rust_example_backport_patches(backports),
        )
        shutil.copytree(
            RUST / "examples" / example / "src",
            destination / "src",
            dirs_exist_ok=True,
        )
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
    write_text(
        share / "BUILD-METADATA.json",
        json.dumps(
            {
                "abi_checksum": (ROOT / "docs/v02/generated/openui-ffi-checksum.txt")
                .read_text(encoding="utf-8")
                .strip(),
                "revision": revision,
                "source_date_epoch": epoch,
                "target": target,
                "version": VERSION,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
    )
    write_text(
        share / "openui.spdx.json",
        json.dumps(make_sbom(metadata, target, revision, epoch), indent=2, sort_keys=True) + "\n",
    )
    normalize_tree(stage, epoch)


def deterministic_tar(source: Path, output: Path, top_level: str, epoch: int) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=epoch) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                paths = [source, *sorted(source.rglob("*"))]
                for path in paths:
                    relative = path.relative_to(source)
                    name = top_level if relative == Path(".") else f"{top_level}/{relative}"
                    info = archive.gettarinfo(str(path), arcname=name)
                    info.uid = 0
                    info.gid = 0
                    info.uname = "root"
                    info.gname = "root"
                    info.mtime = epoch
                    if path.is_file():
                        with path.open("rb") as data:
                            archive.addfile(info, data)
                    else:
                        archive.addfile(info)


def make_deb(stage: Path, output: Path, target: str, epoch: int) -> None:
    dpkg_deb = shutil.which("dpkg-deb")
    if not dpkg_deb:
        raise SystemExit("dpkg-deb is required for --format deb")
    deb_arch, _ = SUPPORTED_TARGETS[target]
    with tempfile.TemporaryDirectory(prefix="openui-deb-") as temporary:
        package = Path(temporary) / "package"
        shutil.copytree(stage, package / "usr", symlinks=True)
        installed_size = sum(path.stat().st_size for path in stage.rglob("*") if path.is_file()) // 1024
        write_text(
            package / "DEBIAN/control",
            f"""Package: openui-sdk
Version: {VERSION}
Section: devel
Priority: optional
Architecture: {deb_arch}
Maintainer: Open UI contributors
Depends: libstdc++6, libfreetype6, libfontconfig1, libx11-6, libxcb1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libwayland-cursor0, libwayland-egl1, libegl1, libglx0
Installed-Size: {installed_size}
Description: Open UI typed retained Linux and headless engine
 Includes the v0.2 C ABI, development headers, static library, examples,
 CMake and pkg-config metadata, licenses, and detached debug symbols.
""",
        )
        normalize_tree(package, epoch)
        env = dict(os.environ)
        env["SOURCE_DATE_EPOCH"] = str(epoch)
        run([dpkg_deb, "--root-owner-group", "--build", str(package), str(output)], env=env)


def make_rpm(stage: Path, output_dir: Path, target: str, epoch: int) -> Path:
    rpmbuild = shutil.which("rpmbuild")
    if not rpmbuild:
        raise SystemExit("rpmbuild is required for --format rpm")
    _, rpm_arch = SUPPORTED_TARGETS[target]
    with tempfile.TemporaryDirectory(prefix="openui-rpm-") as temporary:
        top = Path(temporary)
        for name in ("BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"):
            (top / name).mkdir()
        payload = top / "SOURCES/openui-sdk-payload.tar.gz"
        deterministic_tar(stage, payload, ".", epoch)
        spec = top / "SPECS/openui.spec"
        write_text(
            spec,
            f"""Name: openui
Version: {VERSION}
Release: 1%{{?dist}}
Summary: Typed retained Linux and headless UI engine
License: Apache-2.0
URL: https://github.com/zhuowcui/open-ui
Source0: openui-sdk-payload.tar.gz
BuildArch: {rpm_arch}
Requires: libstdc++, freetype, fontconfig, libX11.so.6()(64bit), libxcb.so.1()(64bit), libxkbcommon.so.0()(64bit), libxkbcommon-x11.so.0()(64bit), libwayland-client.so.0()(64bit), libwayland-cursor.so.0()(64bit), libwayland-egl.so.1()(64bit), libEGL.so.1()(64bit), libGLX.so.0()(64bit)

%description
Open UI v0.2 C ABI, headers, libraries, examples, and build metadata.

%prep

%build

%install
mkdir -p %{{buildroot}}/usr
tar -xzf %{{SOURCE0}} -C %{{buildroot}}/usr

%files
/usr/include/openui
/usr/lib/libopenui.a
/usr/lib/libopenui.so*
/usr/lib/pkgconfig/openui.pc
/usr/lib/cmake/OpenUI
/usr/debug/libopenui.so.debug
/usr/share/openui
""",
        )
        run(
            [
                rpmbuild,
                "-bb",
                "--define",
                f"_topdir {top}",
                "--define",
                f"_source_date_epoch {epoch}",
                "--define",
                "use_source_date_epoch_as_buildtime 1",
                "--define",
                "clamp_mtime_to_source_date_epoch 1",
                str(spec),
            ]
        )
        built = next((top / "RPMS").rglob("*.rpm"))
        output = output_dir / built.name
        shutil.copy2(built, output)
        return output


def write_sidecars(artifacts: list[Path], target: str, revision: str, epoch: int) -> None:
    for artifact in artifacts:
        write_text(artifact.with_suffix(artifact.suffix + ".sha256"), f"{sha256(artifact)}  {artifact.name}\n")
    statement = {
        "_type": "https://in-toto.io/Statement/v1",
        "subject": [
            {"name": artifact.name, "digest": {"sha256": sha256(artifact)}}
            for artifact in artifacts
        ],
        "predicateType": "https://slsa.dev/provenance/v1",
        "predicate": {
            "buildDefinition": {
                "buildType": "https://github.com/zhuowcui/open-ui/v0.2-linux-release",
                "externalParameters": {"target": target, "version": VERSION},
                "internalParameters": {"source_date_epoch": epoch},
                "resolvedDependencies": [
                    {"uri": "git+https://github.com/zhuowcui/open-ui", "digest": {"gitCommit": revision}}
                ],
            },
            "runDetails": {
                "builder": {"id": "tools/release/build_v02_linux.py"},
                "metadata": {"invocationId": f"{revision}:{target}:{epoch}"},
            },
        },
    }
    write_text(
        artifacts[0].parent / f"openui-{VERSION}-{target}.intoto.jsonl",
        json.dumps(statement, sort_keys=True, separators=(",", ":")) + "\n",
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=sorted(SUPPORTED_TARGETS), default=None)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--library-dir", type=Path, help="use prebuilt openui-ffi libraries")
    parser.add_argument("--source-date-epoch", type=int)
    parser.add_argument(
        "--format",
        action="append",
        choices=("sdk", "deb", "rpm"),
        dest="formats",
        help="artifact to build; repeat as needed (default: sdk)",
    )
    parser.add_argument("--verify-source", action="store_true", help="verify release inputs only")
    parser.add_argument(
        "--allow-dirty",
        action="store_true",
        help="allow a non-release local artifact from an uncommitted tree",
    )
    args = parser.parse_args()

    metadata = verify_source()
    if args.verify_source:
        return
    verify_dependency_security()
    dirty = bool(
        run(["git", "status", "--porcelain"], echo_output=False).strip()
    )
    if dirty and not args.allow_dirty:
        raise SystemExit("release artifacts require a clean worktree (or --allow-dirty for local testing)")
    target = args.target
    if target is None:
        machine = platform.machine()
        target = "aarch64-unknown-linux-gnu" if machine == "aarch64" else "x86_64-unknown-linux-gnu"
    epoch = source_epoch(args.source_date_epoch)
    revision = git_revision()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    library_dir = args.library_dir.resolve() if args.library_dir else build_libraries(target, epoch)

    with tempfile.TemporaryDirectory(prefix="openui-sdk-") as temporary:
        stage = Path(temporary) / "sdk"
        stage_sdk(stage, target, library_dir, metadata, epoch, revision)
        artifacts = []
        formats = args.formats or ["sdk"]
        if "sdk" in formats:
            archive = output / f"openui-sdk-{VERSION}-{target}.tar.gz"
            deterministic_tar(stage, archive, f"openui-sdk-{VERSION}-{target}", epoch)
            artifacts.append(archive)
        if "deb" in formats:
            package = output / f"openui-sdk_{VERSION}_{SUPPORTED_TARGETS[target][0]}.deb"
            make_deb(stage, package, target, epoch)
            artifacts.append(package)
        if "rpm" in formats:
            artifacts.append(make_rpm(stage, output, target, epoch))
        write_sidecars(artifacts, target, revision, epoch)
        for artifact in artifacts:
            print(f"artifact: {artifact} sha256={sha256(artifact)}")


if __name__ == "__main__":
    main()
