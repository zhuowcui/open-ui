from __future__ import annotations

import hashlib
import importlib.util
import tarfile
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("build_v02_linux.py")
SPEC = importlib.util.spec_from_file_location("build_v02_linux", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)


class V02PackagingTests(unittest.TestCase):
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
