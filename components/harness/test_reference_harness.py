"""Unit tests for the components reference harness; no Chromium needed."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
import reference_harness as harness  # noqa: E402


class TokenTests(unittest.TestCase):
    def test_parses_opaque_and_translucent_srgb(self) -> None:
        self.assertEqual(
            harness.parse_srgb("color(srgb 0.0393766 0.039393 0.0393945)"),
            (0.0393766, 0.039393, 0.0393945, 1.0),
        )
        self.assertEqual(
            harness.parse_srgb("color(srgb 0.999871 1.00005 1.00007 / 0.1)"),
            (0.999871, 1.00005, 1.00007, 0.1),
        )
        with self.assertRaises(ValueError):
            harness.parse_srgb("oklch(0.5 0 0)")

    def test_rounds_and_clamps_channels(self) -> None:
        self.assertEqual(harness.to_u8(0.898044), 229)
        self.assertEqual(harness.to_u8(1.00007), 255)
        self.assertEqual(harness.to_u8(-0.0958511), 0)

    def test_token_entry_records_gamut(self) -> None:
        entry = harness.token_entry({
            "theme": "light",
            "token": "destructive",
            "specified": "oklch(0.577 0.245 27.325)",
            "srgb": "color(srgb 0.906415 -0.0958511 0.0423579)",
        })
        self.assertEqual(entry["stand_in_rgba"], [0.906415, -0.0958511, 0.0423579, 1.0])
        self.assertEqual(entry["rgb8"], [231, 0, 11])
        self.assertFalse(entry["in_gamut"])

    def test_committed_tokens_match_their_recorded_conversion(self) -> None:
        evidence = json.loads(
            (harness.ROOT / "components/evidence/theme-tokens-v1.json").read_text()
        )
        self.assertEqual(evidence["schema"], harness.TOKENS_SCHEMA)
        for token in evidence["tokens"]:
            r, g, b, a = harness.parse_srgb(token["chromium_srgb"])
            self.assertEqual(token["stand_in_rgba"], [r, g, b, a])
            self.assertEqual(
                token["rgb8"], [harness.to_u8(r), harness.to_u8(g), harness.to_u8(b)]
            )


class ComparisonTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.path = Path(self.directory.name)

    def tearDown(self) -> None:
        self.directory.cleanup()

    def image(self, name: str, pixels: dict[tuple[int, int], tuple[int, int, int]]) -> Path:
        image = Image.new("RGB", (4, 3), (255, 255, 255))
        for position, color in pixels.items():
            image.putpixel(position, color)
        path = self.path / name
        image.save(path)
        return path

    def test_identical_images_are_exact(self) -> None:
        a = self.image("a.png", {(1, 1): (10, 10, 10)})
        b = self.image("b.png", {(1, 1): (10, 10, 10)})
        result = harness.compare_images(a, b)
        self.assertEqual(result["status"], "exact")
        self.assertEqual(result["mismatched_pixels"], 0)
        self.assertEqual(result["chromium_rgba_sha256"], result["openui_rgba_sha256"])

    def test_counts_pixels_bbox_and_largest_delta(self) -> None:
        a = self.image("a.png", {(1, 1): (10, 10, 10), (3, 2): (0, 0, 0)})
        b = self.image("b.png", {(1, 1): (10, 12, 10)})
        result = harness.compare_images(a, b)
        self.assertEqual(result["status"], "different")
        self.assertEqual(result["mismatched_pixels"], 2)
        self.assertEqual(result["diff_bbox"], [1, 1, 4, 3])
        self.assertEqual(result["max_channel_delta"], 255)

    def test_size_mismatch_is_an_error(self) -> None:
        a = self.image("a.png", {})
        b = self.path / "b.png"
        Image.new("RGB", (5, 3)).save(b)
        self.assertEqual(harness.compare_images(a, b)["status"], "error")


class ReportTests(unittest.TestCase):
    def test_summary_tallies_each_axis(self) -> None:
        rows = [
            {"component": "separator", "profile": "p1", "theme": "light", "status": "exact"},
            {"component": "separator", "profile": "p2", "theme": "dark", "status": "different"},
            {"component": "button", "profile": "p1", "theme": "dark", "status": "error"},
        ]
        summary = harness.summarize(rows)
        self.assertEqual((summary["exact"], summary["different"], summary["error"]), (1, 1, 1))
        self.assertEqual(summary["by_component"]["separator"],
                         {"exact": 1, "different": 1, "error": 0})
        self.assertEqual(summary["by_profile"]["p1"], {"exact": 1, "different": 0, "error": 1})

    def test_census_profiles_are_the_four_required_profiles(self) -> None:
        profiles = harness.census_profiles()
        self.assertEqual(
            [(p.name, p.physical_size) for p in profiles],
            [
                ("legacy-800x600@1", (800, 600)),
                ("mobile-375x667@2", (750, 1334)),
                ("desktop-1280x720@1.25", (1600, 900)),
                ("desktop-1920x1080@1.5", (2880, 1620)),
            ],
        )

    def test_committed_pages_match_their_manifest(self) -> None:
        manifest = harness.load_manifest()
        self.assertEqual(manifest["schema"], "openui-components-reference-v1")
        self.assertTrue(manifest["fixtures"])


if __name__ == "__main__":
    unittest.main()
