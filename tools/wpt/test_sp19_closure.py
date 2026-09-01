#!/usr/bin/env python3
"""Twenty-four focused closure and porter regressions for SP19."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "accountability"))

import generate_sp19_closure as closure  # noqa: E402
import port_wpt  # noqa: E402
import run_all_pixel_comparisons as pixel_runner  # noqa: E402
import splice_text_port  # noqa: E402


class Sp19ClosureAndPorterTests(unittest.TestCase):
    def setUp(self):
        self.old_profile = port_wpt.ACTIVE_PORTER_PROFILE
        self.old_emit = port_wpt.EMIT_TEXT_NODES
        self.old_retain = port_wpt.RETAIN_TEXT
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)

    def tearDown(self):
        port_wpt.ACTIVE_PORTER_PROFILE = self.old_profile
        port_wpt.EMIT_TEXT_NODES = self.old_emit
        port_wpt.RETAIN_TEXT = self.old_retain

    def parse(self, css: str, body: str = "<div id='x'>text</div>"):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        path = Path(temp.name) / "case.html"
        path.write_text(
            f"<!doctype html><style>{css}</style>{body}", encoding="utf-8"
        )
        parser = port_wpt.parse_wpt_html(str(path), root_aware=True)
        self.assertEqual(port_wpt.analyze_portability(parser), (True, ""))
        return parser

    def generate(self, css: str, body: str = "<div id='x'>text</div>") -> str:
        parser = self.parse(css, body)
        return port_wpt.generate_rust_fn(
            "sp19_case", parser.root, parser.html_styles, root_aware=True
        )

    def test_01_manifest_counts_and_hash_pins(self):
        expected = {
            closure.REPAIR_TARGETS: 81,
            closure.REPAIRED_BASELINE: 4139,
            closure.LAYOUT_TARGETS: 823,
            closure.COMBINED_TARGETS: 904,
            closure.FOCUSED: 4962,
            closure.JAVASCRIPT_EXCLUSIONS: 221,
            closure.PROJECTED_UNPORTED: 2711,
        }
        for path, count in expected.items():
            self.assertEqual(len(json.loads(path.read_text(encoding="utf-8"))), count)
            self.assertEqual(
                hashlib.sha256(path.read_bytes()).hexdigest(),
                closure.MANIFEST_SHA256[path],
            )

    def test_02_repair_partitions_are_exact(self):
        partitions = closure.load_partitions(
            closure.REPAIR_PARTITIONS, closure.REPAIR_COUNTS
        )
        targets = set(json.loads(closure.REPAIR_TARGETS.read_text(encoding="utf-8")))
        self.assertEqual(set().union(*map(set, partitions.values())), targets)

    def test_03_layout_partitions_are_exact(self):
        partitions = closure.load_partitions(
            closure.LAYOUT_PARTITIONS, closure.LAYOUT_COUNTS
        )
        targets = set(json.loads(closure.LAYOUT_TARGETS.read_text(encoding="utf-8")))
        self.assertEqual(set().union(*map(set, partitions.values())), targets)
        self.assertEqual(tuple(map(len, partitions.values())), (341, 208, 231, 43))
        self.assertEqual(
            splice_text_port.load_ids_file(
                str(closure.LAYOUT_PARTITIONS), "w3_table_only"
            ),
            partitions["w3_table_only"],
        )
        args, summary = pixel_runner.extract_summary_file(
            ["--summary-file", "focused.json", "--ids-file", "targets.json"]
        )
        self.assertEqual(args, ["--ids-file", "targets.json"])
        self.assertEqual(summary, str(Path("focused.json").resolve()))

    def test_04_strict_layout_syntax_classification(self):
        self.assertEqual(
            closure.syntax_features(
                "<table><td></td></table><style>.x{display:grid;contain:size}</style>"
            ),
            {"table", "grid", "containment"},
        )
        self.assertEqual(closure.syntax_features(".x{display:flex}"), set())

    def test_05_javascript_exclusion_ignores_comments(self):
        self.assertFalse(closure.has_executable_script("<!-- <script>x</script> -->"))
        self.assertTrue(closure.has_executable_script("<script type=module>x</script>"))

    def test_06_final_projection_is_disjoint_and_complete(self):
        repaired = set(json.loads(closure.REPAIRED_BASELINE.read_text()))
        layout = set(json.loads(closure.LAYOUT_TARGETS.read_text()))
        focused = set(json.loads(closure.FOCUSED.read_text()))
        self.assertFalse(repaired & layout)
        self.assertEqual(repaired | layout, focused)

    def test_07_closure_outputs_are_byte_deterministic(self):
        mapping, summary = closure._historical_bytes()
        self.assertEqual(
            closure.build_outputs(mapping, summary),
            closure.build_outputs(mapping, summary),
        )
        target = json.loads(closure.LAYOUT_TARGETS.read_text(encoding="utf-8"))[0]
        exact = {target: {"id": target, "status": "pass", "mismatch_pct": 0.0}}
        promoted = closure.promoted_mapping_bytes(mapping, exact, {target})
        self.assertEqual(
            closure.promoted_mapping_bytes(promoted, exact, {target}), promoted
        )
        original_rows = {
            closure.canonical_id(row): row for row in closure.parse_mapping(mapping)
        }
        promoted_rows = {
            closure.canonical_id(row): row for row in closure.parse_mapping(promoted)
        }
        self.assertEqual(original_rows.keys(), promoted_rows.keys())
        for test_id in original_rows.keys() - {target}:
            self.assertEqual(original_rows[test_id], promoted_rows[test_id])
        self.assertEqual(promoted_rows[target]["ported"], "yes")
        self.assertEqual(promoted_rows[target]["our_test_id"], target)
        self.assertEqual(promoted_rows[target]["pixel_result"], "pass")

    def test_08_html_table_optional_end_tags_are_fixed_up(self):
        parser = self.parse("", "<table><tr><td>a<td>b<tr><th>c</table>")
        table = next(node for node in parser.root.children if node.tag == "table")
        group = next(node for node in table.children if node.tag == "tbody")
        self.assertEqual([node.tag for node in group.children], ["tr", "tr"])
        self.assertEqual(
            [[cell.tag for cell in row.children if not cell.is_text] for row in group.children],
            [["td", "td"], ["th"]],
        )

    def test_09_table_structural_roles_emit(self):
        rust = self.generate(
            "table{table-layout:fixed;border-collapse:collapse}",
            "<table><caption>x</caption><tbody><tr><td>y</td></tr></tbody></table>",
        )
        for role in ("Table", "TableCaption", "TableRowGroup", "TableRow", "TableCell"):
            self.assertIn(f"Display::{role}", rust)
        self.assertNotIn("style.display = Display::Block", rust)
        self.assertIn("TableLayout::Fixed", rust)
        self.assertIn("BorderCollapse::Collapse", rust)
        self.assertIn("VerticalAlign::Middle", rust)
        anonymous = self.generate("div{display:table}", "<div>a<br>b</div>")
        self.assertEqual(anonymous.count("Display::TableRow;"), 1)
        self.assertEqual(anonymous.count("Display::TableCell;"), 1)
        orphan = self.parse("", "<div><td>a</td><tbody>b</tbody></div>")
        div = next(node for node in orphan.root.children if node.tag == "div")
        self.assertTrue(all(child.is_text for child in div.children))
        fostered = self.parse(
            "", "<div><table>before<br>after</table><span>end</span></div>"
        )
        host = next(node for node in fostered.root.children if node.tag == "div")
        self.assertEqual(
            [node.tag for node in host.children],
            ["#text", "br", "#text", "table", "span"],
        )
        table = next(node for node in host.children if node.tag == "table")
        self.assertFalse(table.children)
        align_inherit = self.generate(
            ".p{display:flex;align-self:flex-end}.p>*{display:table;align-self:inherit}",
            "<div class=p><div>x</div></div>",
        )
        self.assertGreaterEqual(
            align_inherit.count("ItemAlignment::new(ItemPosition::FlexEnd)"), 2
        )
        presentational = self.generate(
            "",
            '<table width="300" align="left"><tr><td height="20" valign="top">x</td></tr></table>',
        )
        self.assertIn("style.width = Length::px(300.0)", presentational)
        self.assertIn("style.height = Length::px(20.0)", presentational)
        self.assertIn("style.float = Float::Left", presentational)
        self.assertIn("VerticalAlign::Top", presentational)
        self.assertNotIn("style.display = Display::Block", presentational)

    def test_10_table_spans_emit_normalized_metadata(self):
        rust = self.generate("", "<table><tr><td colspan=3 rowspan=0></td></tr></table>")
        self.assertIn("table_col_span = 3", rust)
        self.assertIn("table_row_span = 0", rust)

    def test_11_grid_track_grammar_emits_intrinsic_and_flex_tracks(self):
        value = port_wpt.parse_grid_track_list("min-content minmax(10px, 1fr)")
        self.assertIn("GridTrackBreadth::MinContent", value)
        self.assertIn("GridTrackSize::MinMax", value)
        self.assertIn("GridTrackBreadth::Flex(1.0)", value)

    def test_12_grid_repeat_and_subgrid_grammar_emit(self):
        repeated = port_wpt.parse_grid_track_list("repeat(auto-fit, [a] 20px)")
        self.assertIn("GridRepetition::AutoFit", repeated)
        self.assertIn('"a".to_string()', repeated)
        subgrid = port_wpt.parse_grid_track_list("subgrid [left] [right]")
        self.assertIn("GridTrackList::Subgrid", subgrid)

    def test_13_grid_placement_and_spans_emit(self):
        placement = port_wpt.parse_grid_placement("named 2 / span 3 edge")
        self.assertIn('name: Some("named".to_string())', placement)
        self.assertIn("count: 3", placement)
        self.assertIn('name: Some("edge".to_string())', placement)

    def test_14_grid_template_areas_preserve_empty_cells(self):
        areas = port_wpt.parse_grid_template_areas('"head head" "main ."')
        self.assertIn('Some("head".to_string())', areas)
        self.assertIn('Some("main".to_string())', areas)
        self.assertIn("None", areas)

    def test_15_containment_flags_and_visibility_emit(self):
        rust = self.generate("#x{contain:inline-size layout paint;content-visibility:auto}")
        self.assertIn("Containment::INLINE_SIZE | Containment::LAYOUT | Containment::PAINT", rust)
        self.assertIn("ContentVisibility::Auto", rust)

    def test_16_intrinsic_fallback_and_container_shorthand_emit(self):
        rust = self.generate(
            "#x{contain-intrinsic-size:auto 30px 40px;container:card / size}"
        )
        self.assertIn("ContainIntrinsicLength::auto_length", rust)
        self.assertIn('"card".to_string()', rust)
        self.assertIn("ContainerType::Size", rust)

    def test_17_nested_rules_and_container_blocks_are_balanced(self):
        parser = self.parse(
            ".host{width:20px;&>.item{height:30px}}"
            "@container (width){.host::before{content:'q'}}",
            "<div class=host><div class=item></div></div>",
        )
        selectors = [selector for selector, _ in parser.css_rules]
        self.assertIn(".host>.item", selectors)
        self.assertIn(".host::before", selectors)

    def test_18_replaced_canvas_metadata_emits(self):
        rust = self.generate("canvas{object-fit:none;object-position:0% 0%}", "<canvas width=40 height=20></canvas>")
        self.assertIn("ReplacedResourceKind::TransparentCanvas", rust)
        self.assertIn("intrinsic_width: Some(40.0)", rust)
        self.assertIn("ObjectFit::None", rust)
        self.assertIn("ObjectPosition", rust)
        image = self.generate(
            "",
            '<img src="support/black20x20.png" width="40" height="30">',
        )
        self.assertIn("ReplacedResourceKind::Image", image)
        self.assertIn("3f08031eb1f4aa651ed4b94e383920d3", image)
        self.assertIn("style.width = Length::px(40.0)", image)
        self.assertIn("style.height = Length::px(30.0)", image)
        template = port_wpt._embed_paint_asset_urls(
            "<img src='support/black20x20.png' width='40' height='30'>"
        )
        self.assertIn('data:image/png;base64,', template)
        data_image = self.generate(
            "img{width:10px;height:20px}",
            "<img src='data:image/png;base64,"
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6Q"
            "AAAA1JREFUGFdjYGhg+A8AAoQBgNXA8F0AAAAASUVORK5CYII='>",
        )
        self.assertIn("intrinsic_width: Some(1.0)", data_image)
        self.assertIn("ReplacedResourceKind::Image", data_image)

    def test_19_form_control_roles_emit(self):
        rust = self.generate(
            "", "<fieldset><legend>x</legend><input type=range><button>x</button><meter></meter></fieldset>"
        )
        for role in ("Fieldset", "Legend", "Range", "Button", "Meter"):
            self.assertIn(f"FormControlRole::{role}", rust)

    def test_20_marker_group_button_and_column_pseudos_emit(self):
        rust = self.generate(
            "#x{scroll-marker-group:before}"
            "#x::scroll-marker-group{display:flex}"
            "#x::column::scroll-marker{content:'*'}"
            "#x::scroll-button(right){content:'>'}"
        )
        self.assertIn("PseudoElementKind::ScrollMarkerGroup", rust)
        self.assertIn("PseudoElementKind::ColumnScrollMarker", rust)
        self.assertIn("ScrollButton(openui_dom::ScrollButtonDirection::Right)", rust)

    def test_21_masks_transforms_and_shapes_emit(self):
        rust = self.generate(
            "#x{mask-image:linear-gradient(black,transparent);"
            "transform:translateX(0) rotate(0deg);shape-outside:margin-box circle(10px)}"
        )
        self.assertIn("mask_layers = vec!", rust)
        self.assertIn("Transform2D", rust)
        self.assertIn("ShapeOutside::Circle", rust)

    def test_22_animation_snapshot_is_frozen_at_zero(self):
        rust = self.generate("#x{animation:bgcolor 100s}")
        self.assertIn("AnimationSnapshot", rust)
        self.assertIn("document_time_ms: 0.0", rust)

    def test_23_transaction_rolls_back_after_install_failure(self):
        with tempfile.TemporaryDirectory() as temp:
            first = Path(temp) / "a.json"
            second = Path(temp) / "b.json"
            first.write_bytes(b"old-a")
            second.write_bytes(b"old-b")
            real_replace = closure.os.replace
            calls = 0

            def fail_second(source, destination):
                nonlocal calls
                calls += 1
                if calls == 2:
                    raise OSError("injected")
                return real_replace(source, destination)

            with mock.patch.object(closure.os, "replace", side_effect=fail_second):
                with self.assertRaises(OSError):
                    closure.write_transaction({first: b"new-a", second: b"new-b"})
            self.assertEqual(first.read_bytes(), b"old-a")
            self.assertEqual(second.read_bytes(), b"old-b")

    def test_24_porter_generation_is_byte_deterministic(self):
        css = "#x{display:grid;grid-template-columns:repeat(2,1fr);contain:layout}"
        first = self.generate(css)
        second = self.generate(css)
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
