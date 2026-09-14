import importlib.util
import json
import pathlib
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


class RendererMatrixTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(MATRIX.CONTRACT.read_text())

    def test_contract_expands_to_required_profile_counts(self):
        full = MATRIX.contract_profiles(self.contract, "full")
        focused = MATRIX.contract_profiles(self.contract, "focused")
        self.assertEqual(len(full), 4)
        self.assertEqual(len(focused), 40)
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

    def test_canonical_result_hash_ignores_mapping_insertion_order(self):
        self.assertEqual(
            MATRIX.canonical_sha256({"a": 1, "b": 2}),
            MATRIX.canonical_sha256({"b": 2, "a": 1}),
        )

    def test_manifest_is_the_immutable_5731_case_set(self):
        ids = MATRIX.load_ids(MATRIX.DEFAULT_IDS)
        self.assertEqual(len(ids), 5731)
        self.assertEqual(MATRIX.sha256(MATRIX.DEFAULT_IDS), self.contract["resource_hashes"][
            "tools/wpt/sp20_focused_ids.json"
        ])
        self.assertTrue(
            MATRIX.is_complete_id_selection(
                self.contract, MATRIX.DEFAULT_IDS, ids, ids
            )
        )
        with tempfile.TemporaryDirectory() as directory:
            partial = pathlib.Path(directory) / "partial.json"
            partial.write_text(json.dumps(ids[:1]))
            self.assertFalse(
                MATRIX.is_complete_id_selection(
                    self.contract, partial, ids[:1], ids[:1]
                )
            )


if __name__ == "__main__":
    unittest.main()
