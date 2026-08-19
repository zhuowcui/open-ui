#!/usr/bin/env python3
"""Focused SP13-R multicol ledger, porter, runner, and audit regressions."""

from __future__ import annotations

import csv
import hashlib
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
ACCOUNTABILITY = ROOT / "tools" / "accountability"
PORTED = ACCOUNTABILITY / "data" / "wpt_ported"
UPSTREAM_FIXTURES = HERE / "fixtures" / "upstream"
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ACCOUNTABILITY))

import generate_sp13r_multicol_closure as closure  # noqa: E402
import port_wpt  # noqa: E402
import run_all_pixel_comparisons as runner  # noqa: E402
import splice_text_port  # noqa: E402

_audit_spec = importlib.util.spec_from_file_location(
    "sp13r_audit", ACCOUNTABILITY / "audit.py"
)
audit = importlib.util.module_from_spec(_audit_spec)
_audit_spec.loader.exec_module(audit)


class LedgerTests(unittest.TestCase):
    def test_runnable_inventory_excludes_native_runner_rows(self):
        summary = {
            "tests": [
                {"id": "sp13/native-smoke", "status": "pass"},
                {"id": "wpt/css_multicol/example", "status": "pass"},
            ]
        }
        self.assertEqual(
            list(closure.runnable_wpt_results(summary)),
            ["wpt/css_multicol/example"],
        )

    def test_frozen_ledgers_are_sorted_disjoint_and_complete(self):
        baseline, targets, residuals = closure.load_ledgers()
        residual_ids = [item["test_id"] for item in residuals]
        self.assertEqual(len(baseline), 2823)
        self.assertEqual(len(targets), 351)
        self.assertEqual(len(residuals), 1018)
        self.assertEqual(baseline, sorted(set(baseline)))
        self.assertEqual(targets, sorted(set(targets)))
        self.assertEqual(residual_ids, sorted(set(residual_ids)))
        self.assertFalse(set(targets) & set(residual_ids))
        self.assertEqual(len(set(targets) | set(residual_ids)), 1369)
        self.assertFalse(set(baseline) & (set(targets) | set(residual_ids)))

    def test_residuals_capture_complete_reason_backed_ownership(self):
        _, _, residuals = closure.load_ledgers()
        for item in residuals:
            self.assertEqual(
                set(item),
                {
                    "test_id",
                    "chromium_test_path",
                    "rejection_reason",
                    "owner_categories",
                },
            )
            self.assertTrue(item["chromium_test_path"])
            self.assertTrue(item["rejection_reason"])
            self.assertEqual(
                item["owner_categories"], sorted(set(item["owner_categories"]))
            )
            self.assertIn("sp13_multicol", item["owner_categories"])

    def test_encoding_is_idempotent(self):
        for path, value in zip(
            (closure.BASELINE_JSON, closure.TARGETS_JSON, closure.RESIDUALS_JSON),
            closure.load_ledgers(),
        ):
            self.assertEqual(path.read_text(encoding="utf-8"), closure.encoded(value))

    def test_historical_sp14_through_sp16_ledgers_are_unchanged(self):
        expected = {
            "sp14_w3_baseline_exact.json": "6c350c67d62a2a5868fb3346851db02f16ca293ae7cd24f113df28ee6236c078",
            "sp14_w3_targets.json": "ead9d861c44ec296a99a3b0887e14d5e66f456f9de368ba8cdf89c02620ea86d",
            "sp14_w4_residuals.json": "258e001a0fbe41adfe2008899ba09d746aea8a33bbe21b0bf532711491149f4a",
            "sp15_baseline_exact.json": "9dcebde69adc70b39660ef8fff771af6f57339c0fd4dd242109a22bf5a4d391d",
            "sp15_actionable_targets.json": "210023e7342d96acc9cd491521e4f100c6fe3a2cbc4bf56a596f05b18405f9ad",
            "sp15_residual_dispositions.json": "f901127ee8ae19a150965a6edbc121185f2a47b9d6d121cc895b453ae4d430af",
            "sp16_baseline_exact.json": "668387215999c79c1dbae11fd8a0e06aceaf0c497b1b3afeda1b31725aa3b668",
            "sp16_actionable_targets.json": "fc457282f5335773d1081fa9a949aaf29d401bac6dcaf38560564a1bcf3aef86",
            "sp16_residual_dispositions.json": "72df8f9e2d8c6141cf266d51a4d2114101592b58dd006494b5c8eb9196d04bcc",
            "sp16_real_font_tests.json": "fc457282f5335773d1081fa9a949aaf29d401bac6dcaf38560564a1bcf3aef86",
        }
        for name, digest in expected.items():
            self.assertEqual(hashlib.sha256((PORTED / name).read_bytes()).hexdigest(), digest)


class TransactionalMulticolPorterTests(unittest.TestCase):
    def test_modern_rgb_color_syntax_is_preserved(self):
        self.assertEqual(
            port_wpt.parse_color("rgb(96 139 168)"),
            "Color::from_rgba8(96, 139, 168, 255)",
        )
        self.assertEqual(
            port_wpt.parse_color("rgb(100% 0% 50% / 25%)"),
            "Color::from_rgba_f32(1.0, 0.0, 0.5, 0.25)",
        )
        self.assertEqual(
            port_wpt.parse_color("rgba(0, 0, 255, 0.3)"),
            "Color::from_rgba_f32(0.0 / 255.0, 0.0 / 255.0, "
            "255.0 / 255.0, 0.3)",
        )
        generated, _ = port_wpt.generate_style_code(
            {"border": "2px solid rgb(96 139 168)"}, "n", 16.0
        )
        self.assertIn(
            "doc.node_mut(n).style.border_top_color = "
            "StyleColor::Resolved(Color::from_rgba8(96, 139, 168, 255));",
            generated,
        )

    def test_legacy_q_materializes_ua_generated_quotes(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.LEGACY_BOX_ONLY)
        parser = port_wpt.WptHtmlParser()
        parser.feed('<body><q style="display:flex"></q></body>')
        parser.finalize()
        generated = port_wpt.generate_rust_fn(
            "generated_quotes", parser.root, parser.html_styles
        )
        self.assertEqual(generated.count("ElementTag::Text"), 2)
        self.assertIn('Some("\\u{201c}".to_string())', generated)
        self.assertIn('Some("\\u{201d}".to_string())', generated)

    def test_gradient_shorthand_emits_stops_without_stealing_inner_color(self):
        generated = port_wpt.generate_single_style(
            "background",
            "linear-gradient(green 80px, red 60px)",
            "style",
        )
        self.assertIsInstance(generated, list)
        joined = "\n".join(generated)
        self.assertIn("background_linear_gradient = Some(LinearGradient", joined)
        self.assertIn("repeating: false", joined)
        self.assertIn("GradientStopPosition::Px(80.0)", joined)
        self.assertIn("GradientStopPosition::Px(60.0)", joined)
        self.assertNotIn("background_color = Color::RED", joined)

    def test_repeating_linear_gradient_preserves_repeat_semantics(self):
        generated = port_wpt.generate_single_style(
            "background-image",
            "repeating-linear-gradient(orange 0, orange 30px, blue 30px, blue 60px)",
            "style",
        )
        self.assertIsInstance(generated, str)
        self.assertIn("background_linear_gradient = Some(LinearGradient", generated)
        self.assertIn("repeating: true", generated)
        self.assertIn("GradientStopPosition::Px(60.0)", generated)

    def test_box_profile_prunes_styled_instruction_paragraph_subtree(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.LEGACY_BOX_ONLY)
        parser = port_wpt.WptHtmlParser()
        parser.feed(
            '<body><p style="line-height:20px;margin:16px 0">'
            'Test passes if there is <strong>no red</strong>.</p>'
            '<div style="width:100px;height:100px"></div></body>'
        )
        parser.finalize()
        generated = port_wpt.generate_rust_fn(
            "instruction_pruning", parser.root, parser.html_styles
        )
        self.assertEqual(generated.count("doc.create_node("), 1)

    def test_implicit_paragraph_close_preserves_following_test_content(self):
        markup = (
            '<body><p>Test passes if both boxes are <strong>identical</strong>.'
            '<div id="test" style="width:10px;height:10px">x</div></body>'
        )
        port_wpt.set_porter_profile(
            port_wpt.PorterProfile.REAL_FONT, retain_text=True
        )
        parser = port_wpt.WptHtmlParser()
        parser.feed(markup)
        parser.finalize()
        self.assertEqual([node.tag for node in parser.root.children], ["p", "div"])
        generated = port_wpt.generate_rust_fn(
            "implicit_paragraph_close", parser.root, parser.html_styles
        )
        self.assertIn("ElementTag::Div", generated)
        self.assertNotIn("Test passes", generated)

        with tempfile.TemporaryDirectory() as directory:
            html_path = Path(directory) / "implicit-p.html"
            html_path.write_text(markup, encoding="utf-8")
            template = port_wpt.generate_html_template(str(html_path))
        self.assertNotIn("Test passes", template)
        self.assertIn('<div id="test"', template)

    def test_stray_paragraph_end_tag_inserts_empty_paragraph(self):
        port_wpt.set_porter_profile(
            port_wpt.PorterProfile.REAL_FONT, retain_text=True
        )
        parser = port_wpt.WptHtmlParser()
        parser.feed('<body><span><p><div></div>before</p>after</span></body>')
        parser.finalize()

        span = parser.root.children[0]
        self.assertEqual(
            [node.tag for node in span.children if not node.is_text],
            ['p', 'div', 'p'],
        )
        self.assertEqual(span.children[-1].text_content, 'after')

    def test_menu_uses_ua_block_display(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        parser = port_wpt.WptHtmlParser()
        parser.feed('<body><span>before<menu></menu>after</span></body>')
        parser.finalize()

        generated = port_wpt.generate_rust_fn(
            'menu_uses_ua_block_display', parser.root, parser.html_styles
        )
        self.assertIn('style.display = Display::Block;', generated)

    def test_nonbreaking_space_is_not_pruned_as_source_whitespace(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        parser = port_wpt.WptHtmlParser()
        parser.feed(
            '<body><div><div style="float:left;width:100%"></div>'
            '&nbsp;<br><br></div></body>'
        )
        parser.finalize()
        generated = port_wpt.generate_rust_fn(
            "nonbreaking_space", parser.root, parser.html_styles
        )
        self.assertIn('Some("\\u{a0}".to_string())', generated)

    def test_box_profile_keeps_nonpainting_structural_inline_content(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.LEGACY_BOX_ONLY)
        parser = port_wpt.WptHtmlParser()
        parser.feed(
            '<body><div><div style="float:left;width:100%"></div>'
            '&nbsp;<br></div></body>'
        )
        parser.finalize()
        generated = port_wpt.generate_rust_fn(
            "structural_break", parser.root, parser.html_styles
        )
        self.assertIn("ElementTag::Break", generated)
        self.assertIn("ElementTag::Text", generated)
        self.assertIn('Some("\\u{a0}".to_string())', generated)

    def test_leading_utf8_bom_is_an_encoding_signature_not_body_text(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.REAL_FONT)
        with tempfile.TemporaryDirectory() as directory:
            html_path = Path(directory) / "bom.html"
            html_path.write_bytes(
                b"\xef\xbb\xbf<!DOCTYPE html><style>div{width:10px}</style>"
                b"<div></div>"
            )
            parser = port_wpt.parse_wpt_html(str(html_path))
            generated = port_wpt.generate_rust_fn(
                "bom_signature", parser.root, parser.html_styles
            )
            template = port_wpt.generate_html_template(str(html_path))

        self.assertNotIn("ElementTag::Text", generated)
        self.assertNotIn("\\u{feff}", generated.lower())
        self.assertNotIn("\ufeff", template)

    def test_generated_quotes_use_the_deterministic_font_override(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        parser = port_wpt.WptHtmlParser()
        parser.feed('<body><q style="display:flex">f</q></body>')
        parser.finalize()
        generated = port_wpt.generate_rust_fn(
            "deterministic_generated_quotes", parser.root, parser.html_styles
        )

        self.assertEqual(generated.count("ElementTag::Text"), 3)
        self.assertGreaterEqual(
            generated.count('FontFamily::Named("Ahem".to_string())'), 4
        )

    def test_widows_and_orphans_are_materialized_on_descendants(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        parser = port_wpt.WptHtmlParser()
        parser.feed(
            '<body><div style="widows:3;orphans:4">'
            '<div><br><br></div></div></body>'
        )
        parser.finalize()
        generated = port_wpt.generate_rust_fn(
            "inherited_widows_orphans", parser.root, parser.html_styles
        )
        self.assertGreaterEqual(generated.count(".widows = 3_u32;"), 2)
        self.assertGreaterEqual(generated.count(".orphans = 4_u32;"), 2)

    def test_chained_adjacent_sibling_selector_keeps_full_specificity(self):
        siblings = [("span", [], ""), ("span", [], "")]
        self.assertTrue(
            port_wpt.match_selector(
                "span + span + span",
                "span",
                [],
                "",
                [("div", [], "")],
                3,
                3,
                siblings,
            )
        )
        self.assertFalse(
            port_wpt.match_selector(
                ".first + span + span",
                "span",
                [],
                "",
                [("div", [], "")],
                3,
                3,
                siblings,
            )
        )

    def test_unmatched_heading_selector_does_not_style_div_reference_boxes(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        parser = port_wpt.WptHtmlParser()
        parser.feed(
            "<style>body{column-count:1;outline:1px solid black}"
            "h3{outline:1px solid blue}</style>"
            "<body><div></div><div><div></div></div><div></div></body>"
        )
        parser.finalize()

        generated = port_wpt.generate_rust_fn(
            "unmatched_heading_selector", parser.root, parser.html_styles
        )
        self.assertNotIn("outline_color = StyleColor::Resolved(Color::BLUE)", generated)

    def test_invalid_longhand_does_not_replace_valid_cascade_value(self):
        parsed = port_wpt.parse_inline_styles(
            "column-count:4;column-count:-1;column-width:12em;column-width:-1px;column-width:100%"
        )
        self.assertEqual(parsed["column-count"], "4")
        self.assertEqual(parsed["column-width"], "12em")

    def test_zero_column_dimensions_are_valid_specified_values(self):
        parsed = port_wpt.parse_inline_styles(
            "column-width:0;column-height:0;columns:1 0px"
        )
        self.assertEqual(parsed["column-width"], "0px")
        self.assertEqual(parsed["column-count"], "1")
        self.assertEqual(parsed["column-height"], "0")

    def test_columns_shorthand_resets_longhands_atomically(self):
        parsed = port_wpt.parse_inline_styles(
            "column-count:7;column-width:10px;columns:20px 3;columns:2 4"
        )
        self.assertEqual(parsed["column-width"], "20px")
        self.assertEqual(parsed["column-count"], "3")
        self.assertNotIn("columns", parsed)

    def test_column_rule_shorthand_resets_and_invalid_is_ignored(self):
        parsed = port_wpt.parse_inline_styles(
            "column-rule:4px dashed red;column-rule:bogus"
        )
        self.assertEqual(parsed["column-rule-width"], "4px")
        self.assertEqual(parsed["column-rule-style"], "dashed")
        self.assertEqual(parsed["column-rule-color"], "red")

    def test_initial_normal_fractional_and_balance_all_values(self):
        parsed = port_wpt.parse_inline_styles(
            "column-gap:normal;column-rule-width:.5px;column-fill:balance-all"
        )
        self.assertEqual(parsed["column-gap"], "normal")
        self.assertEqual(parsed["column-rule-width"], ".5px")
        self.assertEqual(parsed["column-fill"], "balance-all")
        generated, _ = port_wpt.generate_style_code(parsed, "n", 16.0)
        self.assertIn("doc.node_mut(n).style.column_fill = ColumnFill::BalanceAll;", generated)

    def test_zoom_lowering_is_cumulative_and_preserves_border_device_pixels(self):
        parsed = port_wpt.parse_inline_styles(
            "zoom:.1;zoom:-1;height:10px;column-gap:65536px;"
            "border:3px solid black;float:right"
        )
        self.assertEqual(parsed["zoom"], ".1")
        outer_zoom = port_wpt._effective_css_zoom(parsed)
        generated, size = port_wpt.generate_style_code(
            parsed, "n", 16.0, outer_zoom
        )
        self.assertEqual(size, 16.0)
        self.assertIn("doc.node_mut(n).style.height = Length::px(1.0);", generated)
        self.assertIn(
            "doc.node_mut(n).style.column_gap = Some(Length::px(6553.6));",
            generated,
        )
        self.assertIn("doc.node_mut(n).style.border_top_width = 3;", generated)
        self.assertIn("doc.node_mut(n).style.display = Display::Block;", generated)
        self.assertIn("doc.node_mut(n).style.font_size = 1.6;", generated)

        inner_zoom = port_wpt._effective_css_zoom({"zoom": ".1"}, outer_zoom)
        inner, _ = port_wpt.generate_style_code(
            {"column-gap": "65536px"}, "child", size, inner_zoom
        )
        self.assertIn(
            "doc.node_mut(child).style.column_gap = Some(Length::px(655.36));",
            inner,
        )
        self.assertIn("doc.node_mut(child).style.font_size = 0.16;", inner)

    def test_font_relative_border_and_rule_width_use_computed_font_size(self):
        generated, size = port_wpt.generate_style_code(
            {
                "font": "1.25em/1 Ahem",
                "border": "gray solid 1em",
                "column-rule": "lime solid 0.5em",
            },
            "n",
            16.0,
        )
        self.assertEqual(size, 20.0)
        self.assertIn("doc.node_mut(n).style.border_top_width = 20;", generated)
        self.assertIn("doc.node_mut(n).style.column_rule_width = 10;", generated)

    def test_font_shorthand_threads_computed_size_to_element_descendants(self):
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)
        parser = port_wpt.WptHtmlParser()
        parser.feed(
            '<body><div style="font:20px/1 Ahem">'
            '<span><span>f</span></span></div></body>'
        )
        parser.finalize()
        generated = port_wpt.generate_rust_fn(
            "font_shorthand_inheritance", parser.root, parser.html_styles
        )

        self.assertGreaterEqual(generated.count(".style.font_size = 20.0;"), 4)


class GenerationAndRunnerTests(unittest.TestCase):
    def setUp(self):
        self.upstream_patch = mock.patch.object(
            splice_text_port, "WPT_ROOT", str(UPSTREAM_FIXTURES)
        )
        self.upstream_patch.start()

    def tearDown(self):
        self.upstream_patch.stop()

    def test_author_root_box_styling_selects_root_aware_generation(self):
        parser = port_wpt.WptHtmlParser()
        parser.feed("<style>*{max-height:10vh;border-top-style:dotted;columns:1 0px}</style><div></div>")
        parser.finalize()
        self.assertTrue(splice_text_port.requires_distinct_root_box(parser))

    def test_real_font_splice_is_idempotent_for_overlapping_target(self):
        test_id = "wpt/css_multicol/multicol-count-002"
        mapping = splice_text_port.load_mapping_rows()
        generated, _, changes = splice_text_port.prepare_changes(
            [test_id], mapping, profile=port_wpt.PorterProfile.REAL_FONT
        )
        self.assertEqual([item.test_id for item in generated], [test_id])
        self.assertNotIn(splice_text_port.TEXT_PORTED_LIST, changes)
        for path, content in changes.items():
            self.assertEqual(content, Path(path).read_text(encoding="utf-8"))

    def test_default_splice_preserves_real_font_profile_for_overlap(self):
        test_id = "wpt/css_multicol/multicol-count-002"
        mapping = splice_text_port.load_mapping_rows()
        generated, _, changes = splice_text_port.prepare_changes([test_id], mapping)
        explicit, _, _ = splice_text_port.prepare_changes(
            [test_id], mapping, profile=port_wpt.PorterProfile.REAL_FONT
        )
        self.assertEqual([item.test_id for item in generated], [test_id])
        self.assertEqual(generated[0].rust_code, explicit[0].rust_code)
        for path, content in changes.items():
            self.assertEqual(content, Path(path).read_text(encoding="utf-8"))

    def test_real_font_runner_profile_keeps_precedence(self):
        env = runner.openui_environment(use_ahem_noaa=True, use_real_font=True)
        self.assertEqual(env["OPENUI_EDGING"], "subpixel")
        self.assertEqual(env["OPENUI_HINTING"], "slight")
        self.assertTrue(
            env["LD_LIBRARY_PATH"].startswith(runner.REAL_FONT_FREETYPE_DIR + ":")
        )

    def test_mapping_owner_is_confined_to_frozen_unported_residuals(self):
        _, targets, residuals = closure.load_ledgers()
        residual_by_id = {item["test_id"]: item for item in residuals}
        with closure.MAPPING_CSV.open(newline="", encoding="utf-8") as stream:
            rows = list(csv.DictReader(stream))
        mapping = {closure.canonical_id(row): row for row in rows}
        owned = {
            test_id
            for test_id, row in mapping.items()
            if closure.OWNER in closure.categories(row["failure_category"])
        }
        self.assertEqual(owned, set(residual_by_id))
        for test_id in targets:
            self.assertEqual(mapping[test_id]["ported"], "yes")
        for test_id, item in residual_by_id.items():
            row = mapping[test_id]
            self.assertEqual(row["ported"], "no")
            self.assertEqual(
                closure.categories(row["failure_category"]),
                set(item["owner_categories"]),
            )


if __name__ == "__main__":
    unittest.main()
