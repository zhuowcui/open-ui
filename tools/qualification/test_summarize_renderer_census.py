import hashlib
import json
import pathlib
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/qualification"))
import residuals  # noqa: E402
import summarize_renderer_census as census  # noqa: E402


class CensusSummaryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = pathlib.Path(self.temporary.name)
        self.manifest = self.root / "manifest.json"
        self.manifest.write_text(json.dumps(["a", "b"]))
        self.contract = self.root / "contract.json"
        self.contract.write_text(json.dumps({
            "suite_manifests": {"full": {
                "count": 2,
                "sha256": census.file_sha256(self.manifest),
            }},
            "qualification_profiles": [
                self.profile_contract("integral", 1),
                self.profile_contract("fractional", 1.25),
            ],
        }))
        self.ownership = self.root / "ownership.json"
        self.ownership.write_text(json.dumps({"schema_version": 2, "entries": []}))
        self.paths = [self.write_shard(index) for index in range(2)]

    @staticmethod
    def profile_contract(name, scale):
        return {
            "name": name,
            "device_scale": scale,
            "logical_size_css_px": {"width": 8, "height": 6},
            "physical_size_px": {"width": round(8 * scale), "height": round(6 * scale)},
        }

    def write_shard(self, index, *, source="source"):
        test_id = ["a", "b"][index]
        profiles = []
        for name, scale in (("integral", 1), ("fractional", 1.25)):
            status = "different" if test_id == "b" and name == "fractional" else "exact"
            signature = {
                "comparable": True,
                "mismatched_pixels": 1,
                "mismatch_bounds": {"x": 1, "y": 2, "width": 1, "height": 1},
                "connected_region_count": 1,
                "channel_deltas": {"red": {"maximum_absolute_delta": 1}},
            } if status == "different" else {}
            tests = [{"id": test_id, "status": status,
                      "mismatched_pixels": int(status == "different"),
                      "diff_signature": signature}]
            profiles.append({
                "profile": name,
                "logical_size_css_px": [8, 6],
                "physical_size_px": [round(8 * scale), round(6 * scale)],
                "device_scale": scale,
                "tests": tests,
                "total": 1,
                "exact": int(status == "exact"),
                "different": int(status == "different"),
                "errors": 0,
                "ordered_id_sha256": hashlib.sha256(test_id.encode()).hexdigest(),
                "result_sha256": residuals.canonical_sha256(tests),
            })
        report = {
            "schema_version": 2,
            "suite": "full",
            "contract_sha256": census.file_sha256(self.contract),
            "commit": "commit",
            "source": {"source_tree_sha256": source, "harness_sha256": "harness", "clean": False},
            "chromium": {"build_identity": "147", "binary_sha256": "browser",
                         "capture_harness_sha256": "capture"},
            "openui": {"binary_sha256": "renderer", "raster_backend_identity": "cpu"},
            "resource_hashes": {},
            "font_byte_hashes": {},
            "raster": {},
            "manifest_scope": {"reported_suite": "full"},
            "shard": {"index": index, "count": 2},
            "id_manifest": {"sha256": census.file_sha256(self.manifest), "count": 1},
            "evidence": {"tolerance_pixels": 0},
            "profiles": profiles,
            "results": {
                "total": 2,
                "exact": sum(profile["exact"] for profile in profiles),
                "different": sum(profile["different"] for profile in profiles),
                "errors": 0,
                "profile_result_sha256": residuals.canonical_sha256(
                    [profile["result_sha256"] for profile in profiles]
                ),
            },
        }
        path = self.root / f"shard-{index}.json"
        path.write_text(json.dumps(report))
        return path

    def summarize(self, paths=None, allow=True):
        return census.summarize(
            self.paths if paths is None else paths,
            contract_path=self.contract, manifest_path=self.manifest,
            ownership_path=self.ownership, allow_unowned_diagnostics=allow,
        )

    def test_diagnostic_reassembles_and_hashes_complete_shards(self):
        snapshot = self.summarize(list(reversed(self.paths)))
        self.assertFalse(snapshot["qualifying"])
        self.assertEqual(snapshot["comparison_counts"],
                         {"exact": 3, "different": 1, "error": 0})
        self.assertEqual(snapshot["residual_test_count"], 1)
        self.assertEqual(snapshot["fractional_only_test_count"], 1)
        self.assertEqual(snapshot["unowned_test_count"], 1)
        self.assertEqual(snapshot["entries"][0]["test_id"], "b")
        self.assertEqual(snapshot["output_sha256"], residuals.canonical_sha256(
            {key: value for key, value in snapshot.items() if key != "output_sha256"}
        ))

    def test_missing_or_duplicate_shards_fail(self):
        for paths in (self.paths[:1], [self.paths[0], self.paths[0]]):
            with self.assertRaisesRegex(ValueError, "shard indices"):
                self.summarize(paths)

    def test_mixed_source_and_tampered_result_fail(self):
        self.write_shard(1, source="different-source")
        with self.assertRaisesRegex(ValueError, "mixed source identity"):
            self.summarize()
        self.write_shard(1)
        report = json.loads(self.paths[1].read_text())
        report["profiles"][1]["tests"][0]["mismatched_pixels"] = 99
        self.paths[1].write_text(json.dumps(report))
        with self.assertRaisesRegex(ValueError, "profile result hash changed"):
            self.summarize()

    def test_unowned_residuals_fail_by_default(self):
        with self.assertRaisesRegex(ValueError, "unowned renderer residuals"):
            self.summarize(allow=False)


if __name__ == "__main__":
    unittest.main()
