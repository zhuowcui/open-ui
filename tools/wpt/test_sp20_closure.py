#!/usr/bin/env python3
"""Exactly thirty focused regressions for the SP20 static-visual closure."""

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

import generate_sp20_closure as closure  # noqa: E402
import port_wpt  # noqa: E402
import run_all_pixel_comparisons as pixel_runner  # noqa: E402
import splice_text_port  # noqa: E402


class Sp20ClosureAndPorterTests(unittest.TestCase):
    def setUp(self):
        self.profile = port_wpt.ACTIVE_PORTER_PROFILE
        self.emit = port_wpt.EMIT_TEXT_NODES
        self.retain = port_wpt.RETAIN_TEXT
        self.resource_base = port_wpt._ACTIVE_RESOURCE_BASE
        port_wpt.set_porter_profile(port_wpt.PorterProfile.DETERMINISTIC_AHEM)

    def tearDown(self):
        port_wpt.ACTIVE_PORTER_PROFILE = self.profile
        port_wpt.EMIT_TEXT_NODES = self.emit
        port_wpt.RETAIN_TEXT = self.retain
        port_wpt._ACTIVE_RESOURCE_BASE = self.resource_base

    def parse(self, css: str, body: str = "<div id='x'>text</div>"):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        path = Path(temp.name) / "case.html"
        path.write_text(f"<!doctype html><style>{css}</style>{body}", encoding="utf-8")
        parser = port_wpt.parse_wpt_html(str(path), root_aware=True)
        return parser

    def generate(self, css: str, body: str = "<div id='x'>text</div>") -> str:
        parser = self.parse(css, body)
        self.assertEqual(port_wpt.analyze_portability(parser), (True, ""))
        return port_wpt.generate_rust_fn(
            "sp20_case", parser.root, parser.html_styles, root_aware=True
        )

    def test_01_manifest_counts_and_hash_pins(self):
        expected = {
            closure.EXACT_BASELINE: 4962,
            closure.TARGETS: 769,
            closure.JAVASCRIPT_EXCLUSIONS: 1912,
            closure.NONVISUAL_EXCLUSIONS: 30,
            closure.FOCUSED: 5731,
            closure.PROJECTED_UNPORTED: 1942,
        }
        for path, count in expected.items():
            self.assertEqual(len(json.loads(path.read_text())), count)
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), closure.MANIFEST_SHA256[path])

    def test_02_partition_counts_and_hash_pins(self):
        partitions = closure.load_partitions()
        self.assertEqual({key: len(value) for key, value in partitions.items()}, closure.PARTITION_COUNTS)
        for wave, ids in partitions.items():
            self.assertEqual(closure.digest(closure.encoded(ids)), closure.PARTITION_SHA256[wave])
        areas = json.loads(closure.AREA_PARTITIONS.read_text())
        self.assertEqual(
            {key: len(value) for key, value in areas.items()}, closure.AREA_COUNTS
        )
        self.assertEqual(
            hashlib.sha256(closure.AREA_PARTITIONS.read_bytes()).hexdigest(),
            closure.MANIFEST_SHA256[closure.AREA_PARTITIONS],
        )
        accumulated = json.loads(closure.ACCUMULATED_PARTITIONS.read_text())
        self.assertEqual(
            {key: len(value) for key, value in accumulated.items()},
            closure.ACCUMULATED_COUNTS,
        )
        self.assertEqual(
            hashlib.sha256(closure.ACCUMULATED_PARTITIONS.read_bytes()).hexdigest(),
            closure.MANIFEST_SHA256[closure.ACCUMULATED_PARTITIONS],
        )

    def test_03_exclusions_are_disjoint_and_owned(self):
        javascript = set(json.loads(closure.JAVASCRIPT_EXCLUSIONS.read_text()))
        nonvisual = set(json.loads(closure.NONVISUAL_EXCLUSIONS.read_text()))
        targets = set(json.loads(closure.TARGETS.read_text()))
        self.assertFalse(javascript & nonvisual)
        self.assertFalse(targets & (javascript | nonvisual))

    def test_04_source_inventory_covers_targets(self):
        inventory = json.loads(closure.SOURCE_INVENTORY.read_text())
        self.assertEqual([item["test_id"] for item in inventory], json.loads(closure.TARGETS.read_text()))
        self.assertTrue(all(len(item["source_sha256"]) == 64 for item in inventory))

    def test_05_resource_manifest_is_content_addressed(self):
        manifest = json.loads(closure.RESOURCE_MANIFEST.read_text())
        self.assertEqual(
            hashlib.sha256(closure.RESOURCE_MANIFEST.read_bytes()).hexdigest(),
            closure.MANIFEST_SHA256[closure.RESOURCE_MANIFEST],
        )
        self.assertEqual(manifest["asset_count"], len(manifest["assets"]))
        self.assertGreater(manifest["occurrence_count"], 1000)
        for asset in manifest["assets"]:
            self.assertEqual(len(asset["sha256"]), 64)
            self.assertEqual(len(asset["dimensions"]), 2)
            if asset["packaged_path"]:
                path = closure.ROOT / asset["packaged_path"]
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), asset["sha256"])

    def test_06_literal_startup_class_is_the_only_handler(self):
        inventory = json.loads(closure.SOURCE_INVENTORY.read_text())
        handled = [item for item in inventory if item["event_handlers"]]
        self.assertEqual(len(handled), 1)
        self.assertEqual(handled[0]["lowered_body_classes"], ["changed"])

    def test_07_body_class_add_is_lowered_before_cascade(self):
        parser = self.parse("body.changed #x{width:23px}", '<body onload="document.body.classList.add(\'changed\')"><div id=x></div></body>')
        self.assertEqual(port_wpt.analyze_portability(parser), (True, ""))
        self.assertIn("changed", parser.root.attrs.get("class", "").split())
        self.assertEqual(parser.root.children[0].styles["width"], "23px")

    def test_08_other_event_behavior_is_rejected(self):
        parser = self.parse("", '<body onload="window.x=1"><div></div></body>')
        self.assertEqual(port_wpt.analyze_portability(parser), (False, "uses_javascript"))

    def test_09_executable_script_is_rejected(self):
        parser = self.parse("", "<script>document.body.id='x'</script><div></div>")
        self.assertEqual(port_wpt.analyze_portability(parser), (False, "uses_javascript"))

    def test_10_css_comments_are_removed_before_parsing(self):
        parser = self.parse("/* } bogus { */ #x { width: 31px; /* color:red */ height: 7px }")
        self.assertEqual(parser.root.children[0].styles["width"], "31px")
        self.assertEqual(parser.root.children[0].styles["height"], "7px")

    def test_11_xhtml_comment_tokens_are_not_selectors(self):
        parser = self.parse("<!-- prose --> #x { width: 19px } #x { height: 7px }")
        self.assertNotIn("width", parser.root.children[0].styles)
        self.assertEqual(parser.root.children[0].styles["height"], "7px")
        self.assertEqual(port_wpt.analyze_portability(parser), (True, ""))

    def test_12_invalid_declarations_do_not_break_cascade(self):
        parser = self.parse("#x{width:11px;not-a-property:broken;width:17px}")
        self.assertEqual(parser.root.children[0].styles["width"], "17px")

    def test_13_nested_rules_preserve_parent_selector(self):
        parser = self.parse(".host{width:20px;&>.item{height:30px}}", "<div class=host><div class=item></div></div>")
        self.assertEqual(parser.root.children[0].children[0].styles["height"], "30px")

    def test_14_structural_and_attribute_selectors_match(self):
        parser = self.parse(
            "div[data-v='yes']:only-child{width:29px} p~div span{height:31px}",
            "<section><div data-v=yes></div></section><p></p><div><span></span></div>",
        )
        self.assertEqual(parser.root.children[0].children[0].styles["width"], "29px")
        self.assertEqual(parser.root.children[2].children[0].styles["height"], "31px")

    def test_15_details_content_pseudo_emits(self):
        rust = self.generate("details::details-content{display:block;background:green}", "<details><summary>x</summary>y</details>")
        self.assertIn("PseudoElementKind::DetailsContent", rust)

    def test_16_wildcard_scroll_buttons_expand_physically(self):
        rust = self.generate("#x::scroll-button(*){content:'>';background:green}")
        for direction in ("Up", "Right", "Down", "Left"):
            self.assertIn(f"ScrollButtonDirection::{direction}", rust)

    def test_17_textarea_role_and_intrinsic_metrics_emit(self):
        rust = self.generate(
            "textarea{font-size:20px;width:max-content}",
            "<textarea rows=3 cols=4>abcdef</textarea><input type=checkbox>",
        )
        self.assertIn("ElementTag::TextArea", rust)
        self.assertIn("FormControlRole::TextArea", rust)
        self.assertIn("FormControlRole::Checkbox", rust)
        self.assertIn("intrinsic_width: Some(95.0)", rust)
        self.assertIn("style.box_sizing = BoxSizing::BorderBox", rust)

    def test_18_select_option_optgroup_roles_emit(self):
        rust = self.generate("", "<select><optgroup><option>x</option></optgroup></select>")
        for role in ("Select", "OptGroup", "Option"):
            self.assertIn(f"FormControlRole::{role}", rust)

    def test_19_form_and_embed_dom_roles_emit(self):
        rust = self.generate("embed{width:10px}", "<form><embed></form>")
        self.assertIn("ElementTag::Form", rust)
        self.assertIn("ElementTag::Embed", rust)

    def test_20_legacy_box_alignment_maps_compatibly(self):
        rust = self.generate("#x{display:flex;-webkit-box-align:end;-webkit-box-pack:center}")
        self.assertIn("ItemPosition::FlexEnd", rust)
        self.assertIn("ContentPosition::Center", rust)

    def test_21_deterministic_unicode_spaces_are_accepted(self):
        parser = self.parse("", "<div>\u1680\u2002\u2060</div>")
        self.assertEqual(port_wpt.analyze_portability(parser), (True, ""))
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "malformed-body.html"
            path.write_text(
                "<body><div>&ensp;B</div></body>"
                "<div>&emsp;C</div><div>&thinsp;D</div></body>",
                encoding="utf-8",
            )
            template = port_wpt.generate_html_template(str(path))
            self.assertIn("&ensp;B", template)
            self.assertIn("&emsp;C", template)
            self.assertIn("&thinsp;D", template)

    def test_22_inset_clip_path_emits_public_style_contract(self):
        rust = self.generate("#x{clip-path:inset(1px 2px 3px 4px)}")
        self.assertIn("clip_path_inset = Some([", rust)
        self.assertIn("Length::px(4.0)", rust)

    def test_23_individual_translate_lowers_to_transform(self):
        rust = self.generate("#x{translate:10px 20px}")
        self.assertIn("Transform2D", rust)
        self.assertIn("e: 10.0", rust)
        self.assertIn("f: 20.0", rust)
        percentage = self.generate(
            "#x{width:50px;height:20px;transform:translateX(-50%)}"
        )
        self.assertIn("e: -25.0", percentage)

    def test_24_inline_raster_and_svg_dimensions_are_deterministic(self):
        png = port_wpt._data_url_resource("data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==")
        svg = port_wpt._data_url_resource("data:image/svg+xml,%3Csvg%20viewBox='0%200%204%202'%3E%3C/svg%3E")
        self.assertEqual(png[3], (1.0, 1.0))
        self.assertIsNone(svg[3])
        self.assertEqual(
            port_wpt._resource_intrinsic_ratio(svg[4], svg[1], svg[3]),
            (4.0, 2.0),
        )
        picture_path = (
            port_wpt.WPT_SOURCE_ROOT
            / "css/css-sizing/aspect-ratio/replaced-element-012.html"
        )
        parser = port_wpt.parse_wpt_html(str(picture_path), root_aware=True)
        picture = port_wpt.generate_rust_fn(
            "sp20_picture", parser.root, parser.html_styles, root_aware=True
        )
        self.assertIn("ReplacedResourceKind::Image", picture)
        self.assertIn("intrinsic_width: Some(20.0)", picture)
        with tempfile.TemporaryDirectory() as temp:
            iframe_path = Path(temp) / "iframe.html"
            iframe_path.write_text(
                '<iframe srcdoc="<style>html { background-color: red; } <style>"></iframe>',
                encoding="utf-8",
            )
            iframe_template = port_wpt.generate_html_template(str(iframe_path))
            self.assertIn("html { background-color: red; }", iframe_template)
            iframe_parser = port_wpt.parse_wpt_html(str(iframe_path), root_aware=True)
            iframe_rust = port_wpt.generate_rust_fn(
                "sp20_iframe", iframe_parser.root, iframe_parser.html_styles,
                root_aware=True,
            )
            self.assertIn("style.width = Length::px(300.0)", iframe_rust)
            self.assertIn("style.height = Length::px(150.0)", iframe_rust)
            self.assertIn("embedded_canvas_color = Some(Color::RED)", iframe_rust)

    def test_25_packaged_local_resource_matches_manifest(self):
        resource = port_wpt._packaged_resource("/media/1x1-green.png", closure.WPT_BASE)
        self.assertIsNotNone(resource)
        filename, source, mime, sha, dimensions = resource
        self.assertEqual(source, "media/1x1-green.png")
        self.assertEqual(mime, "image/png")
        self.assertEqual(dimensions, (1.0, 1.0))
        self.assertTrue((port_wpt.SP20_ASSET_DIR / filename).is_file())
        self.assertEqual(len(sha), 64)
        template = port_wpt._embed_paint_asset_urls(
            '<picture><source srcset="support/black20x20.png 1x">'
            '<img></picture>'
        )
        self.assertIn('srcset="data:image/png;base64,', template)
        self.assertIn(' 1x"', template)

    def test_26_transaction_rolls_back_on_install_failure(self):
        with tempfile.TemporaryDirectory() as temp:
            first, second = Path(temp) / "a", Path(temp) / "b"
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
            self.assertEqual((first.read_bytes(), second.read_bytes()), (b"old-a", b"old-b"))

    def test_27_closure_regeneration_is_byte_deterministic(self):
        mapping, summary = closure._historical_bytes()
        first = closure.build_outputs(mapping, summary)
        second = closure.build_outputs(mapping, summary)
        self.assertEqual(first, second)

    def test_28_mapping_promotion_preserves_non_targets(self):
        mapping, _ = closure._historical_bytes()
        target = json.loads(closure.TARGETS.read_text())[0]
        result = {target: {"id": target, "status": "pass", "mismatch_pct": 0.0}}
        promoted = closure.promoted_mapping_bytes(mapping, result, {target})
        before = {closure.canonical_id(row): row for row in closure.parse_mapping(mapping)}
        after = {closure.canonical_id(row): row for row in closure.parse_mapping(promoted)}
        self.assertEqual({key: value for key, value in before.items() if key != target}, {key: value for key, value in after.items() if key != target})
        self.assertEqual(after[target]["ported"], "yes")

    def test_29_exact_id_runner_selects_partition_without_resume(self):
        partitions = closure.load_partitions()
        all_tests = partitions["w2_break_multicol"] + ["unrelated"]
        selected, resume, _ = pixel_runner.select_tests(
            all_tests, ["--ids-file", str(closure.PARTITIONS), "--partition", "w2_break_multicol"]
        )
        self.assertEqual(selected, partitions["w2_break_multicol"])
        self.assertFalse(resume)
        document = pixel_runner.build_html_document(
            "<style>@keyframes resize{from{height:100px}to{height:50px}}"
            ".target{animation:resize 1s}</style><div class=target></div>"
        )
        self.assertIn("animation-play-state: paused !important", document)

    def test_30_final_projection_is_complete_and_read_only(self):
        focused = set(json.loads(closure.FOCUSED.read_text()))
        unported = set(json.loads(closure.PROJECTED_UNPORTED.read_text()))
        rows = closure.parse_mapping(closure.KICKOFF_MAPPING.read_bytes())
        self.assertEqual(focused | unported, {closure.canonical_id(row) for row in rows})
        before = {path: (path.stat().st_mtime_ns, path.read_bytes()) for path in closure.build_outputs(*closure._historical_bytes())}
        current_wave = closure.validate_live_snapshot(
            closure.parse_mapping(closure.LIVE_MAPPING.read_bytes()),
            json.loads(closure.LIVE_SUMMARY.read_text(encoding="utf-8")),
        )
        self.assertEqual(closure.check(), current_wave)
        after = {path: (path.stat().st_mtime_ns, path.read_bytes()) for path in before}
        self.assertEqual(before, after)


if __name__ == "__main__":
    unittest.main()
