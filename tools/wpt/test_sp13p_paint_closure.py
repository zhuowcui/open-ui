#!/usr/bin/env python3
"""Six focused SP13-P manifest, porter, splice, and closure regressions."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
ACCOUNTABILITY = ROOT / "tools" / "accountability"
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ACCOUNTABILITY))

import generate_sp13p_paint_closure as closure  # noqa: E402
import port_wpt  # noqa: E402
import splice_text_port  # noqa: E402


class Sp13PPaintClosureTests(unittest.TestCase):
    def test_manifest_and_projection_pins(self):
        targets, focused, residuals, _ = closure.load_manifests()
        self.assertEqual((len(targets), len(focused), len(residuals)), (188, 3807, 82))
        self.assertEqual(len(set(focused) | set(residuals)), 3889)
        summary = json.loads(closure.SUMMARY.read_text(encoding="utf-8"))
        self.assertEqual(
            closure.non_target_projection(summary, set(targets)),
            closure.EXPECTED_NON_TARGET_PROJECTION,
        )

    def test_asset_hashes_and_provenance(self):
        _, _, _, assets = closure.load_manifests()
        closure.validate_assets(assets)
        self.assertEqual(assets["license"], "W3C 3-Clause BSD")
        self.assertEqual(len(assets["assets"]), 20)
        self.assertEqual(len(assets["embedded_data_urls"]), 2)

    def test_complete_background_layer_emission(self):
        previous = port_wpt.EMIT_PAINT_LAYERS
        port_wpt.set_paint_layer_emission(True)
        try:
            styles = port_wpt.parse_inline_styles(
                "background-image:"
                "linear-gradient(to right, currentColor 10% 20%, transparent 20%),"
                "radial-gradient(circle at right 5px bottom 10px, red 0 30%, blue 30%),"
                "conic-gradient(from 45deg at 25% 75%, green 0% 25%, yellow 25%);"
                "background-repeat:no-repeat,round space;"
                "background-position:right 5px bottom 10px,25% 75%;"
                "background-size:cover,calc(50% - 2px) 40%,contain;"
                "background-origin:content-box,padding-box;"
                "background-clip:padding-box,content-box,border-box;"
                "background-attachment:fixed,local"
            )
            lines, _ = port_wpt.generate_style_code(styles, "n", 16.0)
        finally:
            port_wpt.set_paint_layer_emission(previous)
        emitted = "\n".join(lines)
        self.assertEqual(emitted.count("BackgroundLayer {"), 3)
        self.assertIn("CssImage::LinearGradient", emitted)
        self.assertIn("CssImage::RadialGradient", emitted)
        self.assertIn("CssImage::ConicGradient", emitted)
        self.assertIn("BackgroundAttachment::Fixed", emitted)
        self.assertIn("BackgroundAttachment::Local", emitted)
        self.assertIn("BackgroundRepeat::Space", emitted)
        self.assertIn("BackgroundClip::ContentBox", emitted)

    def test_flex_alignment_fixtures_retain_complete_gradient_shorthands(self):
        rust_path = (
            ROOT
            / "bindings/rust/pixel-compare/src/wpt/wpt_css_flexbox.rs"
        )
        rust = rust_path.read_text(encoding="utf-8")
        expected_stops = {
            "css_flexbox_align_content_001": 6,
            "css_flexbox_align_content_003": 4,
            "css_flexbox_align_content_005": 10,
            "css_flexbox_align_items_001": 6,
            "css_flexbox_align_items_003": 4,
            "css_flexbox_align_items_004": 10,
            "css_flexbox_justify_content_003": 4,
        }
        for function, stop_count in expected_stops.items():
            with self.subTest(function=function):
                start, end = splice_text_port._rust_function_span(rust, function)
                builder = rust[start:end]
                self.assertIn(
                    "RendererStyleValue::BackgroundLinearGradient", builder
                )
                self.assertEqual(
                    builder.count("LinearGradientStop {"), stop_count
                )

    def test_border_shadow_and_canvas_metadata(self):
        previous = port_wpt.EMIT_PAINT_LAYERS
        port_wpt.set_paint_layer_emission(True)
        try:
            styles = port_wpt.parse_inline_styles(
                "border-image:url('support/green.png') 10 fill / 2 / 3 round space;"
                "box-shadow:inset 1px 2px 3px 4px currentColor,"
                "5px 6px 7px rgba(0,0,0,.5)"
            )
            lines, _ = port_wpt.generate_style_code(styles, "n", 16.0)
        finally:
            port_wpt.set_paint_layer_emission(previous)
        emitted = "\n".join(lines)
        self.assertIn("border_image = Some(BorderImage", emitted)
        self.assertIn("fill: true", emitted)
        self.assertIn("BorderImageRepeat::Round", emitted)
        self.assertIn("BorderImageRepeat::Space", emitted)
        self.assertIn("spread_radius: 4.0", emitted)
        self.assertIn("inset: true", emitted)
        tree = (ROOT / "bindings/rust/openui-dom/src/tree.rs").read_text()
        painter = (ROOT / "bindings/rust/openui-paint/src/painter.rs").read_text()
        self.assertIn("background_layers.is_empty()", tree)
        self.assertIn("fn paint_canvas_background", painter)
        canvas_paint = painter.split("fn paint_canvas_background", 1)[1].split(
            "fn resolve_border_image_slice", 1
        )[0]
        self.assertIn("paint_background_layers(", canvas_paint)
        self.assertIn("adjusted.background_layers.clear()", painter)

    def test_two_no_write_generations_and_two_surgical_splices(self):
        test_ids = [
            "wpt/css_backgrounds/background-334",
            "wpt/css_backgrounds/box-shadow-003",
        ]
        mapping = splice_text_port.load_mapping_rows()
        rust_path = Path(splice_text_port.RUST_WPT_DIR) / "wpt_css_backgrounds.rs"
        before = hashlib.sha256(rust_path.read_bytes()).hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            upstream = Path(directory)
            fixtures = {
                test_ids[0]: (
                    "<style>div{width:20px;height:20px;"
                    "background:linear-gradient(red,blue) round}</style><div></div>"
                ),
                test_ids[1]: (
                    "<style>div{width:20px;height:20px;"
                    "box-shadow:1px 2px 3px 4px green}</style><div></div>"
                ),
            }
            for test_id, source in fixtures.items():
                path = upstream / mapping[test_id]["chromium_test_path"]
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source, encoding="utf-8")
            with mock.patch.object(splice_text_port, "WPT_ROOT", str(upstream)):
                generated, originals, changes = splice_text_port.prepare_changes(
                    test_ids, mapping, paint_layers=True
                )
        self.assertEqual([row.test_id for row in generated], sorted(test_ids))
        changed_rust = changes[str(rust_path)]
        for row in generated:
            self.assertEqual(
                changed_rust.count(f"fn {row.fn_name}(viewport: ViewportMetrics)"),
                1,
            )
        self.assertEqual(originals[str(rust_path)], rust_path.read_text())
        self.assertEqual(before, hashlib.sha256(rust_path.read_bytes()).hexdigest())

    def test_deterministic_no_drift_closure_twice(self):
        before = {path: closure.digest_file(path) for path in closure.EXPECTED_HASHES}
        closure.validate_closed_snapshot()
        closure.validate_closed_snapshot()
        after = {path: closure.digest_file(path) for path in closure.EXPECTED_HASHES}
        self.assertEqual(before, after)


if __name__ == "__main__":
    unittest.main()
