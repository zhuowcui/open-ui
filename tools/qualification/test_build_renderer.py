import importlib.util
import copy
import hashlib
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("build_renderer", ROOT / "tools/qualification/build_renderer.py")
BUILD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILD)


class RendererLibraryProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name) / "measured"
        self.root.mkdir()
        self.packages = []
        self.messages = []
        for name in sorted(BUILD.RENDERER_PACKAGES):
            folder = self.root / "bindings/rust" / name
            binary = name == "pixel-compare"
            package = {"id": "path+file://" + str(folder) + "#" + name + "@0.2.0",
                       "name": name, "source": None,
                       "manifest_path": str(folder / "Cargo.toml")}
            self.packages.append(package)
            self.messages.append({"reason": "compiler-artifact", "package_id": package["id"],
                                  "manifest_path": package["manifest_path"], "fresh": False,
                                  "target": {"kind": ["bin" if binary else "lib"],
                                             "name": "pixel_compare" if binary else name,
                                             "src_path": str(folder / "src" / ("main.rs" if binary else "lib.rs"))},
                                  "filenames": [],
                                  "executable": str(self.root / "target/pixel_compare") if binary else None})
        self.metadata = {"packages": self.packages}

    def test_every_linked_renderer_library_must_be_fresh(self):
        result = BUILD.verify_artifacts(self.metadata, self.messages, self.root)
        self.assertEqual(set(result["artifacts"]), BUILD.RENDERER_PACKAGES)

    def test_current_runner_cannot_relabel_a_cached_library(self):
        for message in self.messages:
            if message["target"]["name"] == "pixel_compare":
                continue
            with self.subTest(package=message["package_id"]):
                cached = [{**item, "fresh": True} if item is message else item for item in self.messages]
                with self.assertRaisesRegex(ValueError, "cached local artifact"):
                    BUILD.verify_artifacts(self.metadata, cached, self.root)

    def test_another_checkout_cannot_supply_a_fresh_library(self):
        foreign = [{**self.messages[0], "manifest_path": str(self.root.parent / "private/Cargo.toml")},
                   *self.messages[1:]]
        with self.assertRaisesRegex(ValueError, "another checkout"):
            BUILD.verify_artifacts(self.metadata, foreign, self.root)

    def test_source_path_must_stay_inside_the_measured_checkout(self):
        foreign = [{**self.messages[0], "target": {**self.messages[0]["target"],
                    "src_path": str(self.root.parent / "private/lib.rs")}}, *self.messages[1:]]
        with self.assertRaisesRegex(ValueError, "artifact source outside"):
            BUILD.verify_artifacts(self.metadata, foreign, self.root)

    def test_missing_library_record_is_not_a_complete_build(self):
        with self.assertRaisesRegex(ValueError, "missing fresh renderer artifacts"):
            BUILD.verify_artifacts(self.metadata, self.messages[:-1], self.root)

    def test_dependency_override_outside_source_identity_is_rejected(self):
        foreign = {"packages": [{**self.packages[0], "manifest_path": str(self.root.parent / "private/Cargo.toml")},
                                *self.packages[1:]]}
        with self.assertRaisesRegex(ValueError, "outside measured checkout"):
            BUILD.verify_artifacts(foreign, self.messages, self.root)

    def test_build_script_is_not_evidence_that_its_library_was_compiled(self):
        script_only = [{**self.messages[0], "target": {**self.messages[0]["target"], "kind": ["custom-build"]}},
                       *self.messages[1:]]
        with self.assertRaisesRegex(ValueError, "missing fresh renderer artifacts"):
            BUILD.verify_artifacts(self.metadata, script_only, self.root)

    def receipt(self):
        self.binary = self.root / "runner"
        self.binary.write_bytes(b"unit-test executable bytes")
        self.source = {"clean": True, "commit": "measured", "source_tree_sha256": "tree"}
        verified = BUILD.verify_artifacts(self.metadata, self.messages, self.root)
        return {"schema_version": 1, "source": self.source, "source_after": self.source,
                "source_root": str(self.root), "all_commands_terminal": True,
                "renderer_library_provenance_verified": True, "observed_exit_code": 0,
                "binary_sha256": hashlib.sha256(self.binary.read_bytes()).hexdigest(),
                "artifacts": verified["artifacts"]}

    def test_receipt_binds_libraries_to_the_actual_executable(self):
        receipt = self.receipt()
        BUILD.verify_build_receipt(receipt, self.binary, self.source)
        self.binary.write_bytes(b"other executable")
        with self.assertRaisesRegex(ValueError, "another executable"):
            BUILD.verify_build_receipt(receipt, self.binary, self.source)

    def test_failed_or_unfinished_build_is_not_matrix_evidence(self):
        receipt = self.receipt()
        for field, value in [("all_commands_terminal", False), ("observed_exit_code", 1),
                             ("renderer_library_provenance_verified", False)]:
            with self.subTest(field=field):
                with self.assertRaisesRegex(ValueError, "did not complete"):
                    BUILD.verify_build_receipt({**receipt, field: value}, self.binary, self.source)

    def test_source_change_after_build_invalidates_receipt(self):
        receipt = self.receipt()
        receipt["source_after"] = {**self.source, "source_tree_sha256": "changed"}
        with self.assertRaisesRegex(ValueError, "source does not match"):
            BUILD.verify_build_receipt(receipt, self.binary, self.source)

    def test_current_runner_receipt_cannot_hide_cached_dependencies(self):
        receipt = copy.deepcopy(self.receipt())
        receipt["artifacts"]["openui-paint"][0]["fresh"] = True
        with self.assertRaisesRegex(ValueError, "cached linked library"):
            BUILD.verify_build_receipt(receipt, self.binary, self.source)

    def test_receipt_cannot_substitute_a_build_script_for_a_library(self):
        receipt = copy.deepcopy(self.receipt())
        receipt["artifacts"]["openui-paint"][0]["target"]["kind"] = ["custom-build"]
        with self.assertRaisesRegex(ValueError, "compiled linked library"):
            BUILD.verify_build_receipt(receipt, self.binary, self.source)

    def test_dirty_build_requires_explicit_diagnostic_mode(self):
        receipt = self.receipt()
        dirty = {**self.source, "clean": False}
        receipt.update(source=dirty, source_after=dirty)
        with self.assertRaisesRegex(ValueError, "not clean"):
            BUILD.verify_build_receipt(receipt, self.binary, dirty)
        BUILD.verify_build_receipt(receipt, self.binary, dirty, allow_dirty_diagnostics=True)


if __name__ == "__main__":
    unittest.main()
