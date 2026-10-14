from __future__ import annotations

import hashlib
import importlib.util
import json
import shutil
import tarfile
import tempfile
import tomllib
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("build_v02_linux.py")
SPEC = importlib.util.spec_from_file_location("build_v02_linux", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)


class V02PackagingTests(unittest.TestCase):
    def backport_metadata(self, source: Path) -> dict:
        return {"packages": [{
            "name": "zbus_xml", "version": "5.1.0", "source": None,
            "license": "MIT", "manifest_path": str(source / "Cargo.toml"),
        }]}

    def test_backport_sdk_source_license_identity_and_example_root_patch(self) -> None:
        source = release.RUST / "vendor/zbus_xml-v5_1"
        metadata = self.backport_metadata(source)
        backports = release.dependency_backports(metadata)
        provenance = json.loads((source / "OPENUI-BACKPORT.json").read_bytes())
        self.assertEqual(len(backports), 1)
        self.assertEqual(provenance["package"]["rust_version"], "1.85")
        sbom = release.make_sbom(metadata, "x86_64-unknown-linux-gnu", "abc", 1_700_000_000)
        record = sbom["packages"][0]
        self.assertEqual(record["downloadLocation"], "NOASSERTION")
        self.assertEqual(record["licenseDeclared"], "MIT")
        self.assertEqual(record["checksums"][0]["checksumValue"],
                         provenance["source_inventory_sha256"])
        self.assertNotEqual(record["checksums"][0]["checksumValue"],
                            provenance["upstream"]["archive_sha256"])
        self.assertIn(provenance["upstream"]["archive_sha256"], record["sourceInfo"])
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            hashes = []
            for repetition in range(2):
                share = root / str(repetition) / "share/openui"
                release.stage_dependency_backports(share, backports)
                staged = share / "sources/backports/zbus_xml-5.1.0"
                for entry in provenance["files"]:
                    self.assertEqual((staged / entry["path"]).read_bytes(),
                                     (source / entry["path"]).read_bytes())
                self.assertEqual((share / "licenses/zbus_xml-5.1.0-LICENSE.txt").read_bytes(),
                                 (source / "LICENSE").read_bytes())
                example = share / "examples/rust/hello"
                example.mkdir(parents=True)
                manifest = "[package]\nname=\"example\"\nversion=\"0.1.0\"\n" + \
                    release.rust_example_backport_patches(backports)
                parsed = tomllib.loads(manifest)
                dependency = parsed["patch"]["crates-io"]["zbus_xml"]
                self.assertEqual((example / dependency["path"]).resolve(), staged.resolve())
                archive = root / f"{repetition}.tar.gz"
                release.normalize_tree(share, 1_700_000_000)
                release.deterministic_tar(share, archive, "sources", 1_700_000_000)
                hashes.append(hashlib.sha256(archive.read_bytes()).digest())
            self.assertEqual(*hashes)

    def backport_test_root(self, root: Path) -> tuple[Path, Path]:
        rust = root / "rust"
        source = rust / "vendor/zbus_xml-v5_1"
        shutil.copytree(release.RUST / "vendor/zbus_xml-v5_1", source)
        shutil.copyfile(release.RUST / "Cargo.toml", rust / "Cargo.toml")
        return rust, source

    def test_backport_rejects_changed_missing_added_and_symlinked_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            for corruption in ("changed", "missing", "added", "symlink", "provenance"):
                rust, source = self.backport_test_root(Path(temporary) / corruption)
                if corruption == "changed":
                    (source / "src/lib.rs").write_text("modified source\n")
                elif corruption == "missing":
                    (source / "LICENSE").unlink()
                elif corruption == "added":
                    (source / "src/extra.rs").write_text("extra source\n")
                elif corruption == "symlink":
                    (source / "unlisted-link").symlink_to(source / "src", target_is_directory=True)
                else:
                    (source / "OPENUI-BACKPORT.json").unlink()
                with self.assertRaises(SystemExit):
                    release.dependency_backports(self.backport_metadata(source), rust_root=rust)

    def test_required_backport_rejects_manifest_and_directory_symlinks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            for corruption in ("manifest", "directory", "parent"):
                rust, source = self.backport_test_root(Path(temporary) / corruption)
                if corruption == "manifest":
                    manifest = source / "Cargo.toml"
                    original = source.parent / "original-manifest.toml"
                    manifest.rename(original)
                    manifest.symlink_to(original)
                elif corruption == "directory":
                    original = source.parent / "original-source"
                    source.rename(original)
                    source.symlink_to(original, target_is_directory=True)
                else:
                    original = rust / "original-vendor"
                    (rust / "vendor").rename(original)
                    (rust / "vendor").symlink_to(original, target_is_directory=True)
                with self.assertRaisesRegex(SystemExit, "symlink"):
                    release.dependency_backports(self.backport_metadata(source), rust_root=rust)

    def test_required_backport_rejects_missing_wrong_and_registry_selection(self) -> None:
        source = release.RUST / "vendor/zbus_xml-v5_1"
        for corruption in ("absent", "version", "license", "registry", "manifest"):
            metadata = self.backport_metadata(source)
            if corruption == "absent":
                metadata["packages"] = []
            elif corruption == "version":
                metadata["packages"][0]["version"] = "5.1.1"
            elif corruption == "license":
                metadata["packages"][0]["license"] = "Apache-2.0"
            elif corruption == "registry":
                metadata["packages"][0]["source"] = "registry+https://github.com/rust-lang/crates.io-index"
            else:
                metadata["packages"][0]["manifest_path"] = str(source / "other.toml")
            with self.subTest(corruption=corruption), self.assertRaisesRegex(SystemExit, "not selected"):
                release.dependency_backports(metadata)

    def test_backport_rejects_parent_traversal_before_path_normalization(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            rust, source = self.backport_test_root(Path(temporary))
            outside = Path(temporary) / "outside/nested"
            outside.mkdir(parents=True)
            (rust / "vendor/alias").symlink_to(outside, target_is_directory=True)
            metadata = self.backport_metadata(source)
            # Lexical abspath would erase alias/.. and name the declared source;
            # the OS follows alias first, so the original path has another owner.
            metadata["packages"][0]["manifest_path"] = str(
                rust / "vendor/alias/../zbus_xml-v5_1/Cargo.toml"
            )
            with self.assertRaisesRegex(SystemExit, "noncanonical"):
                release.dependency_backports(metadata, rust_root=rust)
            with self.assertRaisesRegex(SystemExit, "noncanonical"):
                release.dependency_backports(self.backport_metadata(source),
                                            rust_root=rust / "vendor/alias/../..")
            metadata["packages"][0]["manifest_path"] = "vendor/zbus_xml-v5_1/Cargo.toml"
            with self.assertRaisesRegex(SystemExit, "noncanonical"):
                release.dependency_backports(metadata, rust_root=rust)

    def test_preexisting_vendor_without_backport_record_remains_supported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            rust, source = self.backport_test_root(Path(temporary))
            legacy = rust / "vendor/ordinary"
            legacy.mkdir()
            (legacy / "Cargo.toml").write_text('[package]\nname="ordinary"\nversion="1.0.0"\n')
            metadata = self.backport_metadata(source)
            metadata["packages"].append({
                "name": "ordinary", "version": "1.0.0", "source": None,
                "license": "MIT", "manifest_path": str(legacy / "Cargo.toml"),
            })
            backports = release.dependency_backports(metadata, rust_root=rust)
            self.assertEqual([(item["name"], item["version"]) for item in backports],
                             [("zbus_xml", "5.1.0")])

    def test_deterministic_archive_normalizes_metadata_and_symlinks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stage = root / "stage"
            (stage / "lib").mkdir(parents=True)
            (stage / "lib/libopenui.so.0.2.0").write_bytes(b"library")
            (stage / "include.h").write_text("header\n", encoding="utf-8")
            (stage / "lib/libopenui.so").symlink_to("libopenui.so.0.2.0")
            release.normalize_tree(stage, 1_700_000_000)
            first = root / "first.tar.gz"
            second = root / "second.tar.gz"
            release.deterministic_tar(stage, first, "sdk", 1_700_000_000)
            release.deterministic_tar(stage, second, "sdk", 1_700_000_000)
            self.assertEqual(hashlib.sha256(first.read_bytes()).digest(), hashlib.sha256(second.read_bytes()).digest())
            with tarfile.open(first, "r:gz") as archive:
                for member in archive.getmembers():
                    self.assertEqual((member.uid, member.gid), (0, 0))
                    self.assertEqual((member.uname, member.gname), ("root", "root"))
                    self.assertEqual(member.mtime, 1_700_000_000)
                self.assertEqual(
                    archive.getmember("sdk/lib/libopenui.so").linkname,
                    "libopenui.so.0.2.0",
                )

    def test_sbom_is_deterministic_and_populated(self) -> None:
        metadata = {
            "packages": [
                {
                    "name": "openui",
                    "version": "0.2.0",
                    "source": None,
                    "license": "Apache-2.0",
                    "checksum": None,
                }
            ]
        }
        metadata["packages"].extend(self.backport_metadata(
            release.RUST / "vendor/zbus_xml-v5_1"
        )["packages"])
        first = release.make_sbom(metadata, "x86_64-unknown-linux-gnu", "abc", 1_700_000_000)
        second = release.make_sbom(metadata, "x86_64-unknown-linux-gnu", "abc", 1_700_000_000)
        self.assertEqual(first, second)
        self.assertEqual(first["spdxVersion"], "SPDX-2.3")
        self.assertEqual(first["packages"][0]["licenseDeclared"], "Apache-2.0")
        self.assertEqual(first["creationInfo"]["created"], "2023-11-14T22:13:20Z")

    def test_public_release_graph_and_targets_are_frozen(self) -> None:
        self.assertEqual(release.VERSION, "0.2.0")
        self.assertEqual(len(release.PUBLIC_CRATES), 11)
        self.assertEqual(
            set(release.SUPPORTED_TARGETS),
            {"x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"},
        )


if __name__ == "__main__":
    unittest.main()
