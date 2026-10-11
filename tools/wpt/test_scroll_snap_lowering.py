"""Complete snap declarations must survive shared Engine snapshot lowering."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, ROOT / "tools/wpt" / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PORT = load("snap_port_wpt", "port_wpt.py")
LOWER = load("snap_engine_fixture_codegen", "engine_fixture_codegen.py")


class ScrollSnapLoweringTests(unittest.TestCase):
    def code(self, name, value):
        result = PORT.generate_single_style(name, value, "doc.node_mut(node).style", 16.0)
        return "\n".join(result) if isinstance(result, list) else result

    def test_type_preserves_mandatory_and_default_proximity(self):
        for value, axis, strictness in [
            ("x mandatory", "X", "Mandatory"),
            ("y proximity", "Y", "Proximity"),
            ("inline", "Inline", "Proximity"),
            ("block mandatory", "Block", "Mandatory"),
            ("both", "Both", "Proximity"),
        ]:
            with self.subTest(value=value):
                lowered = LOWER.lower_assignments(self.code("scroll-snap-type", value))
                self.assertIn(f"RendererStyleValue::ScrollSnapAxis(ScrollSnapAxis::{axis})", lowered)
                self.assertIn(f"RendererStyleValue::ScrollSnapStrictness(ScrollSnapStrictness::{strictness})", lowered)
                self.assertNotIn("node_mut", lowered)
        self.assertEqual(self.code("scroll-snap-type", "none"),
                         "doc.node_mut(node).style.scroll_snap_axis = ScrollSnapAxis::None;")
        for value in ["none mandatory", "x invalid", "both mandatory proximity", ""]:
            self.assertIsNone(self.code("scroll-snap-type", value))

    def test_split_alignment_survives_as_distinct_computed_axes(self):
        lowered = LOWER.lower_assignments(self.code("scroll-snap-align", "end start"))
        self.assertIn("RendererStyleValue::ScrollSnapAlign(ScrollSnapAlign::None)", lowered)
        self.assertIn("RendererStyleValue::ScrollSnapAlignBlock(Some(ScrollSnapAlign::End))", lowered)
        self.assertIn("RendererStyleValue::ScrollSnapAlignInline(Some(ScrollSnapAlign::Start))", lowered)
        scalar = LOWER.lower_assignments(self.code("scroll-snap-align", "center"))
        self.assertIn("RendererStyleValue::ScrollSnapAlign(ScrollSnapAlign::Center)", scalar)
        self.assertNotIn("ScrollSnapAlignBlock", scalar)
        for value in ["start end center", "start invalid", ""]:
            self.assertIsNone(self.code("scroll-snap-align", value))

    def test_padding_margin_and_stop_are_retained_engine_properties(self):
        padding = LOWER.lower_assignments(self.code("scroll-padding", "10% 2px 3px auto"))
        for side, value in [("Top", "Length::percent(10.0)"), ("Right", "Length::px(2.0)"),
                            ("Bottom", "Length::px(3.0)"), ("Left", "Length::auto()")]:
            self.assertIn(f"RendererStyleValue::ScrollPadding{side}({value})", padding)
        margin = LOWER.lower_assignments(self.code("scroll-margin", "-5px 2px"))
        self.assertIn("RendererStyleValue::ScrollMarginTop(Length::px(-5.0))", margin)
        self.assertIn("RendererStyleValue::ScrollMarginLeft(Length::px(2.0))", margin)
        stop = LOWER.lower_assignments(self.code("scroll-snap-stop", "always"))
        self.assertIn("RendererStyleValue::ScrollSnapStop(ScrollSnapStop::Always)", stop)
        self.assertNotIn("scroll-snap-stop", PORT.IGNORED_PROPERTIES)
        for name, value in [("scroll-padding-left", "-1px"), ("scroll-margin-left", "10%"),
                            ("scroll-margin", "auto"), ("scroll-snap-stop", "invalid")]:
            self.assertIsNone(self.code(name, value))

    def test_ordered_offset_state_lowers_to_retained_engine_operation(self):
        code = "doc.node_mut(node).scroll_left = 190.0; doc.node_mut(node).scroll_top = 20.0;"
        lowered = LOWER.lower_assignments(code)
        self.assertIn("doc.set_node_state(node, RendererNodeState::ScrollLeft(190.0));", lowered)
        self.assertIn("doc.set_node_state(node, RendererNodeState::ScrollTop(20.0));", lowered)
        self.assertLess(lowered.index("ScrollLeft"), lowered.index("ScrollTop"))
        self.assertNotIn("node_mut", lowered)


if __name__ == "__main__":
    unittest.main()
