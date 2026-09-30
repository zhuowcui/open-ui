import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

from PIL import Image


ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "run_renderer_matrix", ROOT / "tools/qualification/run_renderer_matrix.py"
)
MATRIX = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = MATRIX
SPEC.loader.exec_module(MATRIX)
import audit_chromium_oracles as ORACLE_AUDIT


class ChromiumOracleAuditTests(unittest.TestCase):
    def write_report(self, path, rgba, *, identity="oracle", png="png", source="capture"):
        rows = [{
            "id": "native-font-case",
            "status": "exact",
            "chromium_oracle_identity_sha256": identity,
            "chromium_rgba_sha256": rgba,
            "chromium_oracle_rgba_sha256": rgba,
            "chromium_png_sha256": png,
            "chromium_oracle_source": source,
        }]
        report = {
            "schema_version": 2,
            "evidence": {"tolerance_pixels": 0},
            "source": {"clean": False},
            "complete_contract_scope": False,
            "contract_sha256": "contract",
            "font_byte_hashes": {},
            "resource_hashes": {},
            "chromium": {"build_identity": "pinned", "binary_sha256": "browser",
                         "capture_harness_sha256": "harness"},
            "profiles": [{"profile": "profile", "tests": rows,
                          "result_sha256": MATRIX.canonical_sha256(rows)}],
        }
        path.write_text(json.dumps(report))

    def test_fresh_capture_conflict_is_reported_despite_renderer_exact_status(self):
        with tempfile.TemporaryDirectory() as directory:
            first, second = [pathlib.Path(directory) / name for name in ("old.json", "fresh.json")]
            self.write_report(first, "old-pixels", source="cache")
            self.write_report(second, "fresh-pixels")
            result = ORACLE_AUDIT.audit([first, second])
            self.assertEqual(result["status"], "contradicted-oracle-identity")
            self.assertEqual(result["contradicted_identity_count"], 1)
            self.assertEqual(result["fresh_capture_observation_count"], 1)
            self.assertFalse(result["renderer_qualification"])

    def test_encoding_changes_with_same_decoded_pixels_are_consistent(self):
        with tempfile.TemporaryDirectory() as directory:
            first, second = [pathlib.Path(directory) / name for name in ("a.json", "b.json")]
            self.write_report(first, "pixels", png="encoding-a")
            self.write_report(second, "pixels", png="encoding-b")
            result = ORACLE_AUDIT.audit([first, second])
            self.assertEqual(result["status"], "consistent-observations")
            self.assertEqual(result["fresh_capture_observation_count"], 2)
            self.assertFalse(result["renderer_qualification"])

    def test_distinct_capture_protocols_do_not_alias_one_oracle(self):
        with tempfile.TemporaryDirectory() as directory:
            first, second = [pathlib.Path(directory) / name for name in ("a.json", "b.json")]
            self.write_report(first, "old-pixels", identity="old-protocol")
            self.write_report(second, "new-pixels", identity="new-protocol")
            result = ORACLE_AUDIT.audit([first, second])
            self.assertEqual(result["contradicted_identity_count"], 0)
            self.assertEqual(result["observed_identity_count"], 2)

    def test_changed_rows_cannot_reuse_a_profile_evidence_digest(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "report.json"
            self.write_report(path, "pixels")
            report = json.loads(path.read_text())
            report["profiles"][0]["tests"][0]["chromium_rgba_sha256"] = "changed"
            path.write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, "changed profile results"):
                ORACLE_AUDIT.audit([path])


class RendererMatrixTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(MATRIX.CONTRACT.read_text())

    def test_contract_expands_to_required_profile_counts(self):
        self.assertEqual(self.contract["schema_version"], 2)
        full = MATRIX.contract_profiles(self.contract, "full")
        focused = MATRIX.contract_profiles(self.contract, "focused")
        residual = MATRIX.contract_profiles(self.contract, "residual")
        residual_cross = MATRIX.contract_profiles(self.contract, "residual-cross")
        self.assertEqual(len(full), 4)
        self.assertEqual(len(focused), 40)
        self.assertEqual(len(residual), 12)
        self.assertEqual(len(residual_cross), 40)
        self.assertEqual(full[1].physical_width, 750)
        self.assertEqual(full[2].physical_width, 1600)
        self.assertEqual(focused[-1].physical_height, 4320)

    def test_profile_selection_is_ordered_and_fail_closed(self):
        profiles = MATRIX.contract_profiles(self.contract, "full")
        selected = MATRIX.select_profiles(
            profiles, ["desktop-1920x1080@1.5", "legacy-800x600@1"]
        )
        self.assertEqual(
            [profile.name for profile in selected],
            ["desktop-1920x1080@1.5", "legacy-800x600@1"],
        )
        with self.assertRaises(ValueError):
            MATRIX.select_profiles(profiles, ["not-a-profile"])

    def test_pixel_comparison_uses_decoded_rgba_not_png_encoding(self):
        with tempfile.TemporaryDirectory() as directory:
            first = pathlib.Path(directory) / "first.png"
            second = pathlib.Path(directory) / "second.png"
            Image.new("RGBA", (2, 1), (10, 20, 30, 255)).save(first, compress_level=0)
            Image.new("RGBA", (2, 1), (10, 20, 30, 255)).save(second, compress_level=9)
            mismatch, first_hash, second_hash = MATRIX.compare_images(first, second)
            self.assertEqual(mismatch, 0)
            self.assertEqual(first_hash, second_hash)
            self.assertNotEqual(MATRIX.sha256(first), MATRIX.sha256(second))

    def test_pixel_comparison_counts_each_changed_pixel_across_all_channels(self):
        with tempfile.TemporaryDirectory() as directory:
            expected = pathlib.Path(directory) / "expected.png"
            actual = pathlib.Path(directory) / "actual.png"
            first = Image.new("RGBA", (4, 1), (10, 20, 30, 255))
            second = first.copy()
            second.putpixel((0, 0), (11, 20, 30, 255))
            second.putpixel((1, 0), (10, 20, 30, 254))
            second.putpixel((2, 0), (10, 21, 31, 255))
            first.save(expected)
            second.save(actual)
            mismatched, first_hash, second_hash = MATRIX.compare_images(expected, actual)
            self.assertEqual(mismatched, 3)
            self.assertNotEqual(first_hash, second_hash)
            self.assertEqual(
                MATRIX.residuals.analyze_image_difference(expected, actual)["mismatched_pixels"],
                mismatched,
            )

    def test_pixel_comparison_rejects_size_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            expected = pathlib.Path(directory) / "expected.png"
            actual = pathlib.Path(directory) / "actual.png"
            Image.new("RGBA", (2, 1), (0, 0, 0, 255)).save(expected)
            Image.new("RGBA", (3, 1), (0, 0, 0, 255)).save(actual)
            self.assertEqual(MATRIX.compare_images(expected, actual)[0], -1)

    def test_canonical_result_hash_ignores_mapping_insertion_order(self):
        self.assertEqual(
            MATRIX.canonical_sha256({"a": 1, "b": 2}),
            MATRIX.canonical_sha256({"b": 2, "a": 1}),
        )

    def test_manifests_are_distinct_and_contract_pinned(self):
        full_ids = MATRIX.load_ids(MATRIX.FULL_IDS)
        focused_ids = MATRIX.load_ids(MATRIX.FOCUSED_IDS)
        primitive_ids = MATRIX.load_ids(MATRIX.PRIMITIVE_IDS)
        expanded_ids = MATRIX.load_ids(MATRIX.EXPANDED_IDS)
        self.assertEqual(len(full_ids), 5731)
        self.assertEqual(len(focused_ids), 16)
        self.assertEqual(len(primitive_ids), 24)
        self.assertEqual(len(expanded_ids), len(full_ids) + 201)
        self.assertTrue(set(full_ids) < set(expanded_ids))
        self.assertEqual(len(set(expanded_ids)), len(expanded_ids))
        self.assertTrue(set(focused_ids) < set(full_ids))
        self.assertTrue(set(primitive_ids) < set(full_ids))
        for suite, path, ids in (
            ("full", MATRIX.FULL_IDS, full_ids),
            ("focused", MATRIX.FOCUSED_IDS, focused_ids),
            ("primitive", MATRIX.PRIMITIVE_IDS, primitive_ids),
            ("expanded", MATRIX.EXPANDED_IDS, expanded_ids),
        ):
            expected = self.contract["suite_manifests"][suite]
            self.assertEqual(MATRIX.sha256(path), expected["sha256"])
            self.assertTrue(
                MATRIX.is_complete_id_selection(
                    self.contract, suite, path, ids, ids
                )
            )
        with tempfile.TemporaryDirectory() as directory:
            partial = pathlib.Path(directory) / "partial.json"
            partial.write_text(json.dumps(full_ids[:1]))
            self.assertFalse(
                MATRIX.is_complete_id_selection(
                    self.contract, "full", partial, full_ids[:1], full_ids[:1]
                )
            )

    def test_shards_are_disjoint_and_reassemble_in_manifest_order(self):
        ids = ["a", "b", "c", "d", "e"]
        shards = [MATRIX.select_shard(ids, index, 3) for index in range(3)]
        self.assertEqual(shards, [["a", "d"], ["b", "e"], ["c"]])
        self.assertEqual(set().union(*map(set, shards)), set(ids))
        with self.assertRaises(ValueError):
            MATRIX.select_shard(ids, 3, 3)

    def test_cache_rejects_an_identity_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            cache = pathlib.Path(directory)
            identity = {"commit": "abc", "profile": {"scale": 2}}
            result = {"id": "fixture", "status": "exact"}
            MATRIX.store_cached_result(cache, identity, result)
            self.assertEqual(MATRIX.cached_result(cache, identity), result)
            key = MATRIX.canonical_sha256(identity)
            path = cache / key[:2] / f"{key}.json"
            payload = json.loads(path.read_text())
            payload["identity"]["profile"]["scale"] = 1
            path.write_text(json.dumps(payload))
            with self.assertRaises(ValueError):
                MATRIX.cached_result(cache, identity)

    def test_chromium_oracle_identity_excludes_openui_implementation(self):
        base = {
            "chromium_binary_sha256": "browser",
            "chromium_build_identity": "147.0.7727.50",
            "chromium_capture_harness_sha256": "capture",
            "contract_sha256": "contract",
            "resource_hashes": {"resource": "hash"},
            "font_byte_hashes": {"font": "hash"},
            "source_tree_sha256": "source-a",
            "openui_binary_sha256": "renderer-a",
            "raster_backend_identity": {"backend": "cpu-skia"},
        }
        profile = MATRIX.Profile("test", 800, 600, 1000, 750, 1.25)
        first = MATRIX.chromium_oracle_identity(
            base, "fixture", "fixture-hash", profile,
            "registered-real-font", True, False,
        )
        changed = dict(base)
        changed.update({
            "source_tree_sha256": "source-b",
            "openui_binary_sha256": "renderer-b",
            "raster_backend_identity": {"backend": "ganesh-gl"},
        })
        second = MATRIX.chromium_oracle_identity(
            changed, "fixture", "fixture-hash", profile,
            "registered-real-font", True, False,
        )
        self.assertEqual(first, second)
        self.assertNotIn("source_tree_sha256", first)
        self.assertNotIn("openui_binary_sha256", first)
        self.assertNotIn("raster_backend_identity", first)

    def test_chromium_oracle_is_immutable_and_content_addressed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            oracle = root / "oracle"
            content = root / "content"
            identity = {"schema_version": 1, "fixture": "case"}
            captured = root / "captured.png"
            Image.new("RGBA", (2, 1), (10, 20, 30, 255)).save(captured)
            stored = MATRIX.store_chromium_oracle(
                oracle, content, identity, captured
            )
            replay = root / "replay.png"
            self.assertEqual(
                MATRIX.cached_chromium_oracle(
                    oracle, content, identity, replay
                ),
                stored,
            )
            self.assertEqual(MATRIX.image_payload(captured), MATRIX.image_payload(replay))

            changed = root / "changed.png"
            Image.new("RGBA", (2, 1), (11, 20, 30, 255)).save(changed)
            with self.assertRaisesRegex(ValueError, "non-deterministic Chromium"):
                MATRIX.store_chromium_oracle(
                    oracle, content, identity, changed
                )

    def test_residual_diff_is_pixel_derived_and_has_connected_regions(self):
        with tempfile.TemporaryDirectory() as directory:
            expected = pathlib.Path(directory) / "expected.png"
            actual = pathlib.Path(directory) / "actual.png"
            Image.new("RGBA", (4, 3), (0, 0, 0, 255)).save(expected)
            changed = Image.new("RGBA", (4, 3), (0, 0, 0, 255))
            changed.putpixel((0, 0), (10, 0, 0, 255))
            changed.putpixel((1, 1), (20, 0, 0, 255))
            changed.putpixel((3, 2), (0, 7, 0, 255))
            changed.save(actual)
            diff = MATRIX.residuals.analyze_image_difference(expected, actual)
            self.assertEqual(diff["mismatched_pixels"], 3)
            self.assertEqual(diff["mismatch_bounds"], {"x": 0, "y": 0, "width": 4, "height": 3})
            self.assertEqual(diff["connected_region_count"], 2)
            self.assertEqual(diff["channel_deltas"]["red"]["maximum_absolute_delta"], 20)

    def test_residual_diff_exact_fast_path_uses_decoded_pixels(self):
        with tempfile.TemporaryDirectory() as directory:
            expected = pathlib.Path(directory) / "expected.png"
            actual = pathlib.Path(directory) / "actual.png"
            Image.new("RGB", (4, 3), (12, 34, 56)).save(expected)
            Image.new("RGBA", (4, 3), (12, 34, 56, 255)).save(actual)
            self.assertNotEqual(expected.read_bytes(), actual.read_bytes())
            diff = MATRIX.residuals.analyze_image_difference(expected, actual)
            self.assertEqual(diff["mismatched_pixels"], 0)
            self.assertIsNone(diff["mismatch_bounds"])
            self.assertEqual(diff["connected_region_count"], 0)
            self.assertTrue(
                all(stats["changed_pixels"] == 0 for stats in diff["channel_deltas"].values())
            )

    def test_residual_ledger_fails_closed_on_unknown_ownership(self):
        summaries = [{
            "profile": "p",
            "logical_size_css_px": [800, 600],
            "physical_size_px": [1000, 750],
            "device_scale": 1.25,
            "result_sha256": "abc",
            "tests": [{"id": "case", "status": "different", "mismatched_pixels": 1}],
        }]
        with self.assertRaisesRegex(ValueError, "unowned renderer residuals"):
            MATRIX.residuals.residual_ledger(summaries, {}, "owner-hash")

    def test_reviewed_reproducer_is_content_addressed(self):
        ownership, ownership_hash = MATRIX.residuals.load_ownership(
            MATRIX.RESIDUAL_OWNERSHIP
        )
        self.assertRegex(ownership_hash, r"^[0-9a-f]{64}$")
        radio = ownership["wpt/css_flexbox/stretch-flex-item-radio-input"]
        reproducer = radio["minimized_reproducer"]
        self.assertEqual(
            MATRIX.sha256(ROOT / reproducer["identity"]), reproducer["sha256"]
        )

    def test_raster_configuration_identity_includes_backend(self):
        self.assertEqual(
            MATRIX.raster_configuration_name(True, False, False, "ganesh-gl"),
            "ganesh-gl:deterministic-alias",
        )

    def test_chromium_capture_requires_exact_device_metrics(self):
        matches = MATRIX.pixel_runner._device_metrics_match
        self.assertTrue(matches({"width": 375, "height": 667, "dpr": 2}, 375, 667, 2))
        self.assertTrue(matches({"width": 800, "height": 600, "dpr": 1.25}, 800, 600, 1.25))
        self.assertFalse(matches({"width": 500, "height": 667, "dpr": 2}, 375, 667, 2))
        self.assertFalse(matches(None, 375, 667, 2))

    def test_ast_mutation_audit_distinguishes_sync_writes_from_async_behavior(self):
        acorn = pathlib.Path(
            os.environ.get(
                "CHROMIUM_ACORN",
                str(
                    pathlib.Path.home()
                    / "chromium/src/third_party/node/node_modules/acorn/dist/acorn.mjs"
                ),
            )
        ).expanduser()
        if not acorn.is_file():
            self.skipTest("pinned Chromium Acorn checkout is unavailable")
        documents = [
            {
                "test_id": "sync",
                "external_scripts": [],
                "scripts": [{
                    "origin": "inline",
                    "code": "document.body.offsetTop; target.style.width = '50px';",
                }],
            },
            {
                "test_id": "async",
                "external_scripts": [],
                "scripts": [{
                    "origin": "inline",
                    "code": "requestAnimationFrame(() => target.style.width = '50px');",
                }],
            },
            {
                "test_id": "called-function",
                "external_scripts": [],
                "scripts": [{
                    "origin": "inline",
                    "code": "function finish() { target.style.width = '50px'; } finish();",
                }],
            },
            {
                "test_id": "uncalled-function",
                "external_scripts": [],
                "scripts": [{
                    "origin": "inline",
                    "code": "function neverRuns() { target.style.width = '50px'; }",
                }],
            },
            {
                "test_id": "network-callback",
                "external_scripts": [],
                "scripts": [{
                    "origin": "inline",
                    "code": (
                        "const link = document.createElement('link'); "
                        "link.onload = () => target.remove(); "
                        "document.head.append(link);"
                    ),
                }],
            },
            {
                "test_id": "constant-control",
                "external_scripts": [],
                "scripts": [{
                    "origin": "inline",
                    "code": (
                        "if (false) target.remove(); "
                        "for (let i = 0; i < 3; i++) { "
                        "document.body.append(document.createElement('div')); }"
                    ),
                }],
            },
        ]
        completed = subprocess.run(
            ["node", ROOT / "tools/wpt/javascript_mutation_ir.mjs", acorn],
            input=json.dumps(documents),
            capture_output=True,
            text=True,
            check=True,
        )
        results = {entry["test_id"]: entry for entry in json.loads(completed.stdout)}
        self.assertTrue(results["sync"]["lowerable"])
        self.assertEqual(
            [operation["kind"] for operation in results["sync"]["operations"]],
            ["layout-barrier", "set-style"],
        )
        self.assertFalse(results["async"]["lowerable"])
        self.assertIn(
            "animation-frame-dependency", results["async"]["rejection_reasons"]
        )
        self.assertTrue(results["called-function"]["lowerable"])
        self.assertEqual(
            [operation["kind"] for operation in results["called-function"]["operations"]],
            ["set-style"],
        )
        self.assertFalse(results["uncalled-function"]["lowerable"])
        self.assertEqual(results["uncalled-function"]["operations"], [])
        self.assertFalse(results["network-callback"]["lowerable"])
        self.assertIn(
            "network-dependency", results["network-callback"]["rejection_reasons"]
        )
        self.assertTrue(results["constant-control"]["lowerable"])
        self.assertEqual(
            [operation["kind"] for operation in results["constant-control"]["operations"]],
            ["create-element", "append"] * 3,
        )


if __name__ == "__main__":
    unittest.main()
