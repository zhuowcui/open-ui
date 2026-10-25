import base64
import unittest

from tools.qualification import capture_scrollbar_probe_grid as grid
from tools.qualification import trace_chrome_layers as trace

# Traced from Chrome 147 for an Auto 16x14 forced-scrollbar box at 1.25.
CLIP_OP = (
    "c2tpYXBpY3RtAAAAAAAAAAAAAAAAACBDAAAgQwFkYWVyJAAAAAQAAB4cAAADAAAAAAAAAAAAAIBBAABgQREAAAAg"
    "AAAABAAAHHRjYWYEAAAAAAAAAGNmcHQAAAAAeWFyYQgAAABndWxzAAAAACBmb2U="
)
BAR_RECORD = (
    "c2tpYXBpY3RtAAAAAAAAAAAAAAAAACBDAAAgQwFkYWVyPAAAAAQAAB4cAAADAAAAAAAAgL8AAIA/AABgQQEAAAA4"
    "AAAAGAAAFQEAAAAAAAAAAACAvwAAgD8AAGBBBAAAHHRjYWYEAAAAAAAAAGNmcHQAAAAAeWFyYSwAAAAgdG5wAQAA"
    "AAAAAAAAAIBA/vx8P/78fD/+/Hw/AACAPwADAABndWxzAAAAACBmb2U="
)


class TraceDecodeTest(unittest.TestCase):
    def test_overflow_controls_clip_is_antialiased(self):
        self.assertEqual(
            trace.decode_ops(base64.b64decode(CLIP_OP)),
            ["save", "clipRect(0,0,16,14) aa=1", "restore"],
        )

    def test_bar_record_is_a_hard_clip_and_rect(self):
        self.assertEqual(
            trace.decode_ops(base64.b64decode(BAR_RECORD)),
            ["save", "clipRect(0,-1,1,14) aa=0", "drawRect(0,-1,1,14)", "restore"],
        )

    def test_non_picture_bytes_decode_to_nothing(self):
        self.assertEqual(trace.decode_ops(b"not a picture"), [])


class ProbeGridTest(unittest.TestCase):
    def test_grid_covers_both_styles_and_every_scale(self):
        cases = grid.cases()
        self.assertEqual(len(cases), 1600)
        self.assertEqual(len(set(cases)), 1600)
        self.assertEqual({case[3] for case in cases}, {1.0, 1.25, 1.5, 1.75, 2.0})

    def test_case_names_keep_integer_and_float_spellings(self):
        self.assertEqual(grid.case_name("auto", 1, 14.5, 1.0), "auto-1-14.5-1.0")

    def test_probe_html_replaces_only_the_box_size(self):
        html = grid.probe_html("thin", 15.6, 13.8)
        self.assertIn(b"width:15.6px;height:13.8px", html)
        self.assertIn(b"scrollbar-width:thin", html)


if __name__ == "__main__":
    unittest.main()
