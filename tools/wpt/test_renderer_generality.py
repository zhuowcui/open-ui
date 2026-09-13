import importlib.util
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "port_wpt", ROOT / "tools" / "wpt" / "port_wpt.py"
)
PORT_WPT = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(PORT_WPT)


class RendererGeneralityTests(unittest.TestCase):
    def test_viewport_lengths_stay_semantic(self):
        self.assertEqual(
            PORT_WPT.parse_length("25vw"),
            "crate::fixture_viewport_length(LengthValue::ViewportWidth(25.0))",
        )
        self.assertEqual(
            PORT_WPT.parse_length("25vh"),
            "crate::fixture_viewport_length(LengthValue::ViewportHeight(25.0))",
        )
        self.assertEqual(
            PORT_WPT.parse_length("25vmin"),
            "crate::fixture_viewport_length(LengthValue::ViewportMin(25.0))",
        )
        self.assertEqual(
            PORT_WPT.parse_length("25vmax"),
            "crate::fixture_viewport_length(LengthValue::ViewportMax(25.0))",
        )

    def test_static_geometry_and_transforms_never_assume_800_by_600(self):
        self.assertIsNone(PORT_WPT._css_length_px("10vw"))
        self.assertIsNone(PORT_WPT._css_length_px("10vh"))
        self.assertIsNone(PORT_WPT._parse_calc_token("10vmin"))
        self.assertIsNone(PORT_WPT._parse_calc_token("10vmax"))
        self.assertIsNone(PORT_WPT._transform_2d_rust("translateX(10vw)", 16.0))


if __name__ == "__main__":
    unittest.main()
