#!/usr/bin/env python3
"""Focused tests for the SP14 deterministic text porter and splice tool."""

from __future__ import annotations

import contextlib
import csv
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import port_wpt
import splice_text_port
import generate_sp14_text_closure

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "accountability"))
import run_all_pixel_comparisons
import shared_detectors
import audit
import generate_sp12_5_csv


class TextPorterTests(unittest.TestCase):
    def setUp(self):
        self.old_profile = port_wpt.ACTIVE_PORTER_PROFILE
        self.old_emit = port_wpt.EMIT_TEXT_NODES
        self.old_retain = port_wpt.RETAIN_TEXT
        # SP14 tests exercise the deterministic-font generator unless a test
        # explicitly opts into the later real-font profile.
        port_wpt.ACTIVE_PORTER_PROFILE = port_wpt.PorterProfile.LEGACY_BOX_ONLY
        self.temp = tempfile.TemporaryDirectory()

    def tearDown(self):
        port_wpt.ACTIVE_PORTER_PROFILE = self.old_profile
        port_wpt.EMIT_TEXT_NODES = self.old_emit
        port_wpt.RETAIN_TEXT = self.old_retain
        self.temp.cleanup()

    def html(self, content: str, name: str = "sample.html") -> Path:
        path = Path(self.temp.name, name)
        path.write_text(content, encoding="utf-8")
        return path

    def test_box_mode_stays_text_free(self):
        path = self.html(
            "<!doctype html><body><div>alpha <span>beta</span></div></body>"
        )
        port_wpt.EMIT_TEXT_NODES = False
        port_wpt.RETAIN_TEXT = False
        parser = port_wpt.parse_wpt_html(str(path))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)
        template = port_wpt.generate_html_template(str(path))
        self.assertNotIn("ElementTag::Text", rust)
        self.assertNotIn("alpha", rust)
        self.assertNotIn("alpha", template)
        self.assertNotIn(port_wpt.TEXT_TEMPLATE_OVERRIDE, template)
        self.assertNotIn("doc.node_mut(vp).style.display = Display::Block", rust)

    def test_nested_whitespace_entities_break_and_inherited_text_styles(self):
        path = self.html(
            """<!doctype html><body><div style="font: 20px/1 Ahem;
            color: rgb(255, 0, 0); white-space: pre-wrap;
            text-transform: uppercase; letter-spacing: 2px; word-spacing: 3px">alpha &amp;  <span> beta</span><br>gamma</div></body>"""
        )
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True
        parser = port_wpt.parse_wpt_html(str(path))
        portable, reason = port_wpt.analyze_portability(parser)
        self.assertTrue(portable, reason)
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)
        template = port_wpt.generate_html_template(str(path))

        self.assertIn(port_wpt.DETERMINISTIC_FONT_FAMILY_RUST, rust)
        self.assertIn(
            'doc.node_mut(vp).style.font_family = '
            + port_wpt.DETERMINISTIC_FONT_FAMILY_RUST,
            rust,
        )
        self.assertNotIn("doc.node_mut(vp).style.display = Display::Block", rust)
        self.assertIn(
            "doc.node_mut(vp).style.list_style_type = ListStyleType::None",
            rust,
        )
        self.assertIn(
            "style.list_style_type = ListStyleType::None",
            rust,
        )
        self.assertIn("font_size = 20.0", rust)
        self.assertIn("color = Color::from_rgba8(255, 0, 0, 255)", rust)
        self.assertIn("white_space = WhiteSpace::PreWrap", rust)
        self.assertIn("line_height = LineHeight::Number(1.0)", rust)
        self.assertIn("text_transform = TextTransform::Uppercase", rust)
        self.assertIn("letter_spacing = 2.0", rust)
        self.assertIn("word_spacing = 3.0", rust)
        self.assertIn('Some("alpha &  ".to_string())', rust)
        self.assertIn('Some(" beta".to_string())', rust)
        self.assertIn('Some("\\n".to_string())', rust)
        self.assertIn("alpha &amp;  <span> beta</span><br>gamma", template)
        self.assertNotIn("</br>", template)
        self.assertTrue(template.endswith(port_wpt.TEXT_TEMPLATE_OVERRIDE))

    def test_verified_unicode_repertoire_is_preserved_as_ascii_rust(self):
        path = self.html(
            "<!doctype html><body><div>&nbsp;É…\u202ea\u202d←↓</div></body>"
        )
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True
        parser = port_wpt.parse_wpt_html(str(path))
        self.assertEqual(port_wpt.analyze_portability(parser), (True, ""))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)
        template = port_wpt.generate_html_template(str(path))
        rust.encode("ascii")
        self.assertIn(
            r'Some("\u{a0}\u{c9}\u{2026}\u{202e}a\u{202d}\u{2190}\u{2193}".to_string())',
            rust,
        )
        self.assertIn('font-family: Ahem, "DejaVu Sans"', template)

    def test_unsupported_unicode_text_is_rejected(self):
        path = self.html("<!doctype html><body><div>snowman ☃</div></body>")
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True
        parser = port_wpt.parse_wpt_html(str(path))
        self.assertEqual(
            port_wpt.analyze_portability(parser), (False, "text_non_ascii")
        )

    def test_output_is_deterministic_across_hash_seeds(self):
        path = self.html(
            "<!doctype html><body><div style='color:blue;white-space:pre-wrap'>"
            "alpha <span style='letter-spacing:2px'> beta</span></div></body>"
        )
        module_dir = str(Path(port_wpt.__file__).resolve().parent)
        script = (
            "import sys; "
            f"sys.path.insert(0, {module_dir!r}); import port_wpt as p; "
            "p.EMIT_TEXT_NODES=True; p.RETAIN_TEXT=True; "
            f"q=p.parse_wpt_html({str(path)!r}); "
            "print(p.generate_rust_fn('demo', q.root, q.html_styles)); "
            f"print(p.generate_html_template({str(path)!r}))"
        )
        outputs = []
        for seed in ("1", "8675309"):
            env = os.environ.copy()
            env["PYTHONHASHSEED"] = seed
            outputs.append(subprocess.check_output([sys.executable, "-c", script], env=env))
        self.assertEqual(outputs[0], outputs[1])

    def test_border_radius_inherit_uses_all_computed_parent_corners(self):
        path = self.html(
            """<!doctype html><body>
            <div style="border-top-left-radius:20% 25px;
                        border-bottom-right-radius:20pt 3em">
              <div style="width:100px;height:50px;border-radius:inherit">text</div>
            </div></body>"""
        )
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True
        parser = port_wpt.parse_wpt_html(str(path))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)

        # The child receives the parent's two explicit corners and zero for
        # the other computed longhands. Percentages resolve on the child box.
        self.assertGreaterEqual(
            rust.count("border_top_left_radius = (20.0_f32, 25.0_f32)"), 2
        )
        self.assertGreaterEqual(
            rust.count("border_bottom_right_radius = (26.666666666666664_f32, 48.0_f32)"),
            2,
        )
        self.assertIn("border_top_right_radius = (0.0_f32, 0.0_f32)", rust)
        self.assertIn("border_bottom_left_radius = (0.0_f32, 0.0_f32)", rust)

        shorthand = self.html(
            """<!doctype html><body><div style="border-radius:20% 25px">
            <div style="width:100px;height:50px;border-radius:inherit">text</div>
            </div></body>""",
            "shorthand.html",
        )
        parser = port_wpt.parse_wpt_html(str(shorthand))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)
        self.assertIn("border_top_left_radius = (20.0_f32, 20.0_f32)", rust)
        self.assertIn("border_top_left_radius = (20.0_f32, 10.0_f32)", rust)

    def test_display_contents_reparents_text_without_painting_a_box(self):
        path = self.html(
            """<!doctype html><body><div><div style="display:contents;
            border:10px solid red;color:blue;font-size:20px">P<span>A</span>SS</div>
            </div></body>"""
        )
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True
        parser = port_wpt.parse_wpt_html(str(path))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)

        self.assertNotIn("Display::Contents", rust)
        self.assertNotIn("border_top_width = 10", rust)
        self.assertIn('Some("P".to_string())', rust)
        self.assertIn('Some("A".to_string())', rust)
        self.assertIn('Some("SS".to_string())', rust)
        self.assertGreaterEqual(rust.count("font_size = 20.0"), 3)
        self.assertGreaterEqual(rust.count("color = Color::BLUE"), 3)

    def test_flex_inter_element_whitespace_does_not_become_an_item(self):
        path = self.html(
            """<!doctype html><body><div style="display:flex">
            <span style="flex:1 0 0%"></span>
            <span style="flex:1 0 0%"></span>
            </div></body>"""
        )
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True
        parser = port_wpt.parse_wpt_html(str(path))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)

        self.assertEqual(rust.count("ElementTag::Span"), 2)
        self.assertNotIn("ElementTag::Text", rust)

    def test_styled_heading_uses_ua_font_size_for_em_geometry(self):
        path = self.html(
            """<!doctype html><body><h1 style="margin:0;height:4em;
            position:absolute">cover</h1></body>"""
        )
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True
        parser = port_wpt.parse_wpt_html(str(path))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)

        self.assertIn("font_size = 32.0", rust)
        self.assertIn("height = Length::px(128.0)", rust)

        # The UA compatibility path is text-mode-only.
        port_wpt.EMIT_TEXT_NODES = False
        port_wpt.RETAIN_TEXT = False
        parser = port_wpt.parse_wpt_html(str(path))
        legacy = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)
        self.assertNotIn("font_size = 32.0", legacy)
        self.assertIn("height = Length::px(64.0)", legacy)

    def test_real_font_heading_ua_size_wins_over_inherited_size(self):
        port_wpt.set_porter_profile(
            port_wpt.PorterProfile.REAL_FONT, retain_text=True
        )
        parser = port_wpt.WptHtmlParser()
        parser.feed(
            '<body><div style="font-size:16px"><h1 style="column-span:all">'
            'heading</h1></div></body>'
        )
        parser.finalize()

        rust = port_wpt.generate_rust_fn(
            "real_font_heading_ua_size", parser.root, parser.html_styles
        )
        h1_start = rust.index("style.column_span = ColumnSpan::All")
        h1_prefix = rust[max(0, h1_start - 800):h1_start]
        self.assertIn("style.font_size = 32.0", h1_prefix)

    def test_clearing_break_emits_semantic_zero_height_break(self):
        path = self.html(
            "<!doctype html><style>br{clear:both}</style><body>"
            "<div style='float:left;width:20px;height:20px'></div><br>after</body>"
        )
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True
        parser = port_wpt.parse_wpt_html(str(path))
        rust = port_wpt.generate_rust_fn("demo", parser.root, parser.html_styles)
        self.assertIn("ElementTag::Break", rust)
        self.assertIn("clear = Clear::Both", rust)
        self.assertNotIn("height = Length::px(16.0)", rust)


class SpliceTransactionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        root = Path(self.temp.name)
        self.wpt_root = root / "upstream"
        self.rust_dir = root / "rust"
        self.template_dir = root / "templates"
        self.wpt_root.mkdir()
        self.rust_dir.mkdir()
        self.template_dir.mkdir()

        (self.wpt_root / "sample.html").write_text(
            "<!doctype html><body><div>sample text</div></body>", encoding="utf-8"
        )
        (self.rust_dir / "wpt_demo.rs").write_text(
            "fn demo_sample() -> Document {\n"
            "    let literal = \"} retained source brace\";\n"
            "    old_doc(literal)\n"
            "}\n\n"
            "pub fn demo_registry() -> Vec<(&'static str, fn() -> Document)> {\n"
            "    vec![(\n"
            "        \"wpt/demo/sample\",\n"
            "        demo_sample as fn() -> Document\n"
            "    )]\n"
            "}\n",
            encoding="utf-8",
        )
        for filename in ("all_wpt_templates.json", "wpt_demo_templates.json"):
            (self.template_dir / filename).write_text(
                json.dumps({"wpt/demo/sample": "<div></div>"}, indent=2) + "\n",
                encoding="utf-8",
            )
        (self.template_dir / "wpt_demo_report.csv").write_text(
            "filename,status,fn_name,reason\n"
            "sample,ported,demo_sample,\n",
            encoding="utf-8",
        )
        self.manifest = self.template_dir / "text_ported_tests.json"
        self.manifest.write_text("[]\n", encoding="utf-8")
        self.mapping_csv = root / "mapping.csv"
        self.mapping_csv.write_text(
            "chromium_test_path,test_name,sp_area,our_test_id\n"
            "sample.html,sample,demo,wpt/demo/sample\n",
            encoding="utf-8",
        )
        self.mapping = {
            "wpt/demo/sample": {
                "chromium_test_path": "sample.html",
                "test_name": "sample",
                "sp_area": "demo",
                "our_test_id": "wpt/demo/sample",
            }
        }
        self.patches = [
            mock.patch.object(splice_text_port, "WPT_ROOT", str(self.wpt_root)),
            mock.patch.object(splice_text_port, "RUST_WPT_DIR", str(self.rust_dir)),
            mock.patch.object(splice_text_port, "WPT_PORTED_DIR", str(self.template_dir)),
            mock.patch.object(splice_text_port, "TEXT_PORTED_LIST", str(self.manifest)),
            mock.patch.object(splice_text_port, "MAPPING_CSV", str(self.mapping_csv)),
        ]
        for patcher in self.patches:
            patcher.start()
        port_wpt.EMIT_TEXT_NODES = True
        port_wpt.RETAIN_TEXT = True

    def tearDown(self):
        for patcher in reversed(self.patches):
            patcher.stop()
        self.temp.cleanup()

    def snapshot(self) -> dict[str, bytes]:
        paths = list(self.rust_dir.iterdir()) + list(self.template_dir.iterdir())
        return {str(path): path.read_bytes() for path in paths}

    def add_unported_fixture(self, name: str, html: str | None = None) -> str:
        test_id = f"wpt/demo/{name}"
        filename = f"{name}.html"
        (self.wpt_root / filename).write_text(
            html or f"<!doctype html><body><div>{name} text</div></body>",
            encoding="utf-8",
        )
        self.mapping[test_id] = {
            "chromium_test_path": filename,
            "test_name": name,
            "sp_area": "demo",
            "our_test_id": "",
        }
        with (self.template_dir / "wpt_demo_report.csv").open("a", encoding="utf-8") as f:
            f.write(f"{name},not_portable,,no_layout_content\n")
        return test_id

    def test_dry_run_is_side_effect_free(self):
        before = self.snapshot()
        with mock.patch.object(
            sys, "argv", ["splice_text_port.py", "--dry-run", "wpt/demo/sample"]
        ), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(splice_text_port.main(), 0)
        self.assertEqual(before, self.snapshot())

    def test_repeated_real_run_is_idempotent(self):
        generated, originals, changes = splice_text_port.prepare_changes(
            ["wpt/demo/sample"], self.mapping
        )
        self.assertEqual(len(generated), 1)
        splice_text_port.commit_changes(originals, changes)
        once = self.snapshot()
        _generated, originals, changes = splice_text_port.prepare_changes(
            ["wpt/demo/sample"], self.mapping
        )
        splice_text_port.commit_changes(originals, changes)
        self.assertEqual(once, self.snapshot())
        self.assertEqual(json.loads(self.manifest.read_text()), ["wpt/demo/sample"])

    def test_mixed_add_and_replace_transaction_is_idempotent(self):
        added = self.add_unported_fixture("added")
        generated, originals, changes = splice_text_port.prepare_changes(
            [added, "wpt/demo/sample"], self.mapping
        )
        self.assertEqual([item.test_id for item in generated], sorted([added, "wpt/demo/sample"]))
        splice_text_port.commit_changes(originals, changes)

        rust = (self.rust_dir / "wpt_demo.rs").read_text(encoding="utf-8")
        self.assertEqual(rust.count("fn demo_added() -> Document"), 1)
        self.assertEqual(rust.count(f'"{added}"'), 1)
        self.assertNotIn("vec![\n\n", rust)
        self.assertIn(
            f'("{added}", demo_added as fn() -> Document)', rust
        )
        for filename in ("all_wpt_templates.json", "wpt_demo_templates.json"):
            templates = json.loads((self.template_dir / filename).read_text())
            self.assertIn(added, templates)
        with (self.template_dir / "wpt_demo_report.csv").open(newline="") as report:
            report_rows = {row["filename"]: row for row in csv.DictReader(report)}
        self.assertEqual(report_rows["added"]["status"], "ported")
        self.assertEqual(report_rows["added"]["fn_name"], "demo_added")
        self.assertEqual(report_rows["added"]["reason"], "")
        self.assertEqual(
            json.loads(self.manifest.read_text()), sorted([added, "wpt/demo/sample"])
        )

        once = self.snapshot()
        _generated, originals, changes = splice_text_port.prepare_changes(
            ["wpt/demo/sample", added], self.mapping
        )
        splice_text_port.commit_changes(originals, changes)
        self.assertEqual(once, self.snapshot())

    def test_ids_file_drives_a_dry_run(self):
        ledger = Path(self.temp.name, "ids.json")
        ledger.write_text('["wpt/demo/sample"]\n', encoding="utf-8")
        before = self.snapshot()
        with mock.patch.object(
            sys,
            "argv",
            ["splice_text_port.py", "--ids-file", str(ledger), "--dry-run"],
        ), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(splice_text_port.main(), 0)
        self.assertEqual(before, self.snapshot())

        ledger.write_text(
            '["wpt/demo/sample", "wpt/demo/sample"]\n', encoding="utf-8"
        )
        with mock.patch.object(
            sys, "argv", ["splice_text_port.py", "--ids-file", str(ledger)]
        ), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(splice_text_port.main(), 1)
        self.assertEqual(before, self.snapshot())

    def test_unported_mapping_rows_receive_canonical_ids(self):
        self.mapping_csv.write_text(
            "chromium_test_path,test_name,sp_area,our_test_id\n"
            "sample.html,sample,demo,\n",
            encoding="utf-8",
        )
        rows = splice_text_port.load_mapping_rows()
        self.assertEqual(set(rows), {"wpt/demo/sample"})

    def test_function_collision_and_partial_add_state_write_nothing(self):
        first = self.add_unported_fixture("new-a")
        second = self.add_unported_fixture("new_a")
        before = self.snapshot()
        with self.assertRaises(ValueError):
            splice_text_port.prepare_changes([first, second], self.mapping)
        self.assertEqual(before, self.snapshot())

        partial = self.add_unported_fixture("partial")
        all_templates = self.template_dir / "all_wpt_templates.json"
        templates = json.loads(all_templates.read_text())
        templates[partial] = "<div>partial</div>"
        all_templates.write_text(json.dumps(templates, indent=2) + "\n")
        before = self.snapshot()
        with self.assertRaises(ValueError):
            splice_text_port.prepare_changes([partial], self.mapping)
        self.assertEqual(before, self.snapshot())

    def test_missing_upstream_rejected_before_writes(self):
        missing = "wpt/demo/missing"
        mapping = dict(self.mapping)
        mapping[missing] = {
            "chromium_test_path": "missing.html",
            "test_name": "missing",
            "sp_area": "demo",
            "our_test_id": "",
        }
        before = self.snapshot()
        with self.assertRaises(FileNotFoundError):
            splice_text_port.prepare_changes([missing], mapping)
        self.assertEqual(before, self.snapshot())

    def test_validation_failure_writes_nothing(self):
        module_templates = self.template_dir / "wpt_demo_templates.json"
        module_templates.write_text("{}\n", encoding="utf-8")
        before = self.snapshot()
        with self.assertRaises(ValueError):
            splice_text_port.prepare_changes(["wpt/demo/sample"], self.mapping)
        self.assertEqual(before, self.snapshot())

    def test_cross_module_validation_failure_writes_nothing(self):
        (self.wpt_root / "other.html").write_text(
            "<!doctype html><body><div>other text</div></body>", encoding="utf-8"
        )
        (self.rust_dir / "wpt_other.rs").write_text(
            "fn other_other() -> Document { old_doc() }\n"
            "pub fn other_registry() -> Vec<(&'static str, fn() -> Document)> {\n"
            "    vec![(\"wpt/other/other\", other_other as fn() -> Document)]\n"
            "}\n",
            encoding="utf-8",
        )
        all_templates = self.template_dir / "all_wpt_templates.json"
        templates = json.loads(all_templates.read_text(encoding="utf-8"))
        templates["wpt/other/other"] = "<div></div>"
        all_templates.write_text(json.dumps(templates, indent=2) + "\n", encoding="utf-8")
        # The module template deliberately lacks its matching identity.
        (self.template_dir / "wpt_other_templates.json").write_text(
            "{}\n", encoding="utf-8"
        )
        mapping = dict(self.mapping)
        mapping["wpt/other/other"] = {
            "chromium_test_path": "other.html",
            "test_name": "other",
            "sp_area": "other",
            "our_test_id": "wpt/other/other",
        }

        before = self.snapshot()
        with self.assertRaises(ValueError):
            splice_text_port.prepare_changes(
                ["wpt/demo/sample", "wpt/other/other"], mapping
            )
        self.assertEqual(before, self.snapshot())

    def test_commit_failure_rolls_back_replaced_files(self):
        files = sorted([self.manifest, self.template_dir / "all_wpt_templates.json"])
        originals = {str(path): path.read_text(encoding="utf-8") for path in files}
        changes = {str(path): originals[str(path)] + "changed\n" for path in files}
        real_replace = os.replace
        calls = 0

        def fail_second_replace(src, dst):
            nonlocal calls
            calls += 1
            if calls == 2:
                raise OSError("injected commit failure")
            return real_replace(src, dst)

        with mock.patch.object(splice_text_port.os, "replace", fail_second_replace):
            with self.assertRaises(OSError):
                splice_text_port.commit_changes(originals, changes)
        for path in files:
            self.assertEqual(path.read_text(encoding="utf-8"), originals[str(path)])
        self.assertFalse(list(self.template_dir.glob(".*.tmp")))


class RunnerScopeTests(unittest.TestCase):
    def test_w2_w3_ledgers_are_sorted_unique_and_fully_manifested(self):
        data_dir = Path(run_all_pixel_comparisons.__file__).resolve().parent / "data"
        ported_dir = data_dir / "wpt_ported"
        w2 = json.loads((ported_dir / "sp14_w2_targets.json").read_text())
        w3 = json.loads((ported_dir / "sp14_w3_targets.json").read_text())
        manifest = json.loads((ported_dir / "text_ported_tests.json").read_text())
        self.assertEqual(len(w2), 286)
        self.assertEqual(len(w3), 111)
        self.assertEqual(w2, sorted(set(w2)))
        self.assertEqual(w3, sorted(set(w3)))
        self.assertFalse(set(w2) & set(w3))
        sp15 = json.loads((ported_dir / "sp15_actionable_targets.json").read_text())
        sp13r = json.loads((ported_dir / "sp13r_multicol_targets.json").read_text())
        self.assertEqual(len(manifest), 687)
        self.assertEqual(len(set(manifest) - set(w2) - set(w3)), 290)
        self.assertTrue(set(w2) | set(w3) <= set(manifest))
        self.assertTrue(set(sp15) <= set(manifest))
        self.assertTrue(set(sp13r) <= set(manifest))

    def test_ahem_fontconfig_is_only_added_for_manifest_opt_in(self):
        with tempfile.NamedTemporaryFile() as config, mock.patch.object(
            run_all_pixel_comparisons, "AHEM_FONTCONFIG", config.name
        ), mock.patch.dict(os.environ, {"PATH": os.environ.get("PATH", "")}, clear=True):
            ordinary = run_all_pixel_comparisons.chrome_environment("/chrome", False)
            text_ported = run_all_pixel_comparisons.chrome_environment("/chrome", True)
        self.assertNotIn("FONTCONFIG_FILE", ordinary)
        self.assertEqual(text_ported["FONTCONFIG_FILE"], config.name)

    def test_exact_id_manifest_selection_is_validated_and_ordered(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp, "ids.json")
            path.write_text('["wpt/a/one", "wpt/c/three"]\n', encoding="utf-8")
            selected, resume, label = run_all_pixel_comparisons.select_tests(
                ["wpt/a/one", "wpt/b/two", "wpt/c/three"],
                ["--ids-file", str(path)],
            )
            self.assertEqual(selected, ["wpt/a/one", "wpt/c/three"])
            self.assertFalse(resume)
            self.assertIn("exact IDs", label)

            path.write_text('["wpt/a/one", "wpt/a/one"]\n', encoding="utf-8")
            with self.assertRaises(ValueError):
                run_all_pixel_comparisons.select_tests(
                    ["wpt/a/one"], ["--ids-file", str(path)]
                )

    def test_legacy_prefix_and_resume_selection_still_work(self):
        selected, resume, label = run_all_pixel_comparisons.select_tests(
            ["wpt/a/one", "wpt/b/two"], ["--resume", "wpt/a/"]
        )
        self.assertEqual(selected, ["wpt/a/one"])
        self.assertTrue(resume)
        self.assertIn("prefix", label)

    def test_reset_style_node_cannot_be_made_renderable_by_test_css(self):
        document = run_all_pixel_comparisons.build_html_document(
            "<style>* { display: contents }</style><br><whatever>PASS</whatever>"
        )
        self.assertIn('<style style="display:none!important">', document)
        self.assertEqual(document.count(run_all_pixel_comparisons.BODY_STYLE), 1)

    def test_openui_alias_mode_is_only_added_for_manifest_opt_in(self):
        with mock.patch.dict(
            os.environ, {"PATH": os.environ.get("PATH", "")}, clear=True
        ):
            ordinary = run_all_pixel_comparisons.openui_environment(False)
            text_ported = run_all_pixel_comparisons.openui_environment(True)
        self.assertNotIn("OPENUI_EDGING", ordinary)
        self.assertEqual(text_ported["OPENUI_EDGING"], "alias")
        self.assertEqual(text_ported["OPENUI_SUBPIXEL"], "0")
        self.assertEqual(text_ported["OPENUI_HINTING"], "none")


class TextClosureLedgerTests(unittest.TestCase):
    def test_ledger_derivation_is_deterministic_and_merges_rejection_owner(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "portable.html").write_text(
                "<!doctype html><body><div>portable</div></body>", encoding="utf-8"
            )
            (root / "scripted.html").write_text(
                "<!doctype html><body><script>run()</script><div>text</div></body>",
                encoding="utf-8",
            )
            (root / "root-only.html").write_text(
                "<!doctype html><style>body{overflow:scroll}</style>"
                "<body>root text</body>",
                encoding="utf-8",
            )
            rows = [
                {
                    "chromium_test_path": filename,
                    "test_name": name,
                    "sp_area": "demo",
                    "ported": "no",
                    "our_test_id": "",
                    "failure_category": "needs_text",
                }
                for filename, name in (
                    ("portable.html", "portable"),
                    ("scripted.html", "scripted"),
                    ("root-only.html", "root-only"),
                )
            ]
            summary = {
                "tests": [
                    {"id": "wpt/z/exact", "status": "pass", "mismatch_pct": 0.0},
                    {"id": "wpt/a/fail", "status": "fail", "mismatch_pct": 1.0},
                ]
            }
            first = generate_sp14_text_closure.build_ledgers(rows, summary, root)
            second = generate_sp14_text_closure.build_ledgers(rows, summary, root)
            self.assertEqual(first, second)
            baseline, w3, w4 = first
            self.assertEqual(baseline, ["wpt/z/exact"])
            self.assertEqual(w3, ["wpt/demo/portable"])
            by_id = {item["test_id"]: item for item in w4}
            self.assertEqual(
                by_id["wpt/demo/scripted"]["rejection_owner"],
                "needs_javascript",
            )
            self.assertIn(
                "needs_javascript",
                by_id["wpt/demo/scripted"]["owner_categories"],
            )
            self.assertEqual(
                by_id["wpt/demo/root-only"]["rejection_owner"],
                "needs_root_body_layout",
            )

    def test_closed_snapshot_preserves_baseline_and_w3_w4_disposition(self):
        rows = [
            {
                "chromium_test_path": "portable.html",
                "test_name": "portable",
                "sp_area": "demo",
                "ported": "yes",
                "failure_category": "",
                "notes": "",
            },
            {
                "chromium_test_path": "scripted.html",
                "test_name": "scripted",
                "sp_area": "demo",
                "ported": "no",
                "failure_category": "needs_javascript",
                "notes": "Porter deferred: uses_javascript",
            },
        ]
        summary = {
            "tests": [
                {
                    "id": "wpt/demo/portable",
                    "status": "pass",
                    "mismatch_pct": 0.0,
                }
            ]
        }
        w4 = [
            {
                "test_id": "wpt/demo/scripted",
                "chromium_test_path": "scripted.html",
                "rejection_reason": "uses_javascript",
                "rejection_owner": "needs_javascript",
                "owner_categories": ["needs_javascript"],
            }
        ]
        generate_sp14_text_closure.validate_closed_snapshot(
            rows,
            summary,
            ["wpt/demo/portable"],
            ["wpt/demo/portable"],
            w4,
        )
        rows[1]["failure_category"] = "reference_test"
        with self.assertRaisesRegex(ValueError, "ownership drift"):
            generate_sp14_text_closure.validate_closed_snapshot(
                rows,
                summary,
                ["wpt/demo/portable"],
                ["wpt/demo/portable"],
                w4,
            )


class AccountabilityDetectorTests(unittest.TestCase):
    def test_mask_longhands_have_a_functional_visual_effects_owner(self):
        categories, _dependency = shared_detectors.classify_failure_categories(
            "<style>.x{-webkit-mask-image:url(mask.png);mask-size:100px}</style>",
        )
        self.assertIn("needs_visual_effects", categories.split(","))

    def test_sp15_exposed_residuals_have_precise_functional_owners(self):
        cases = {
            "<p>label</p><div></div><div></div><div style='margin:10px'></div>":
                "needs_empty_block_margin_collapse",
            "<style>body{background:lightblue}</style><div></div>":
                "needs_body_canvas_background_extent",
            "<style>.x{overflow:auto;scrollbar-color:blue blue}</style>":
                "needs_scrollbar_paint",
            "<div style='display:flow-root'><i style='float:left'></i>"
            "<i style='float:right'></i><i style='float:left'></i>"
            "<i style='float:right'></i></div>": "needs_float_row_packing",
            "<style>body{column-count:2}</style>": "sp13_multicol",
        }
        for html, owner in cases.items():
            with self.subTest(owner=owner):
                categories, _dependency = shared_detectors.classify_failure_categories(html)
                self.assertIn(owner, categories.split(","))

    def test_unported_classification_merges_detector_and_rejection_owners(self):
        categories, _dependency = shared_detectors.classify_failure_categories(
            "<div style=\"background:url(asset.png)\"></div>",
            excluded={"text_rendering"},
        )
        self.assertIn("needs_image", categories)

        import generate_wpt_mapping

        categories, _dependency = generate_wpt_mapping.classify_unported_test(
            "<div style=\"background:url(asset.png)\"></div>",
            "demo",
            "sample",
            "uses_javascript",
        )
        self.assertEqual(
            set(categories.split(",")), {"needs_image", "needs_javascript"}
        )

    def test_sp14_audit_invariants_cover_baseline_w3_and_w4(self):
        rows = [
            {
                "chromium_test_path": "portable.html",
                "test_name": "portable",
                "sp_area": "demo",
                "ported": "yes",
                "our_test_id": "wpt/demo/portable",
                "pixel_result": "pass",
                "failure_category": "",
                "notes": "",
            },
            {
                "chromium_test_path": "scripted.html",
                "test_name": "scripted",
                "sp_area": "demo",
                "ported": "no",
                "our_test_id": "",
                "pixel_result": "",
                "failure_category": "needs_javascript",
                "notes": "Porter deferred: uses_javascript",
            },
        ]
        summary = {
            "wpt/demo/portable": {
                "id": "wpt/demo/portable",
                "status": "pass",
                "mismatch_pct": 0.0,
            }
        }
        w4 = [
            {
                "test_id": "wpt/demo/scripted",
                "chromium_test_path": "scripted.html",
                "rejection_reason": "uses_javascript",
                "rejection_owner": "needs_javascript",
                "owner_categories": ["needs_javascript"],
            }
        ]
        errors = audit.sp14_text_closure_errors(
            rows,
            summary,
            {"wpt/demo/portable": "<div>portable</div>"},
            {"wpt/demo/portable"},
            ["wpt/demo/portable"],
            ["wpt/demo/portable"],
            w4,
            enforce_frozen_counts=False,
        )
        self.assertEqual(errors, [])

        w4[0]["rejection_owner"] = "non_visual_test"
        w4[0]["owner_categories"] = ["non_visual_test"]
        errors = audit.sp14_text_closure_errors(
            rows,
            summary,
            {"wpt/demo/portable": "<div>portable</div>"},
            {"wpt/demo/portable"},
            ["wpt/demo/portable"],
            ["wpt/demo/portable"],
            w4,
            enforce_frozen_counts=False,
        )
        self.assertTrue(any("functional ownership" in error for error in errors))

    def test_deferred_classifier_uses_upstream_html_for_text_ports(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            upstream = root / "CSS2" / "floats" / "sample.html"
            upstream.parent.mkdir(parents=True)
            upstream.write_text("<div>original upstream evidence</div>")
            with mock.patch.object(generate_sp12_5_csv, "CHROMIUM_WPT_BASE", root):
                self.assertEqual(
                    generate_sp12_5_csv.classification_html(
                        "wpt/demo/text",
                        "<div>normalized comparison template</div>",
                        {"wpt/demo/text"},
                        {"wpt/demo/text": "CSS2/floats/sample.html"},
                    ),
                    "<div>original upstream evidence</div>",
                )
                self.assertEqual(
                    generate_sp12_5_csv.classification_html(
                        "wpt/demo/ordinary",
                        "<div>ordinary template</div>",
                        {"wpt/demo/text"},
                        {"wpt/demo/text": "CSS2/floats/sample.html"},
                    ),
                    "<div>ordinary template</div>",
                )

    def test_text_port_ownership_rejects_stale_and_metadata_only_categories(self):
        rows = [
            {
                "our_test_id": "wpt/demo/stale",
                "pixel_result": "fail",
                "failure_category": "needs_text,reference_test",
            },
            {
                "our_test_id": "wpt/demo/owned",
                "pixel_result": "fail",
                "failure_category": "reference_test,needs_inline_block",
            },
            {
                "our_test_id": "wpt/demo/pass",
                "pixel_result": "pass",
                "failure_category": "",
            },
        ]
        stale, metadata_only = audit.text_port_ownership_errors(
            rows, {"wpt/demo/stale", "wpt/demo/owned", "wpt/demo/pass"}
        )
        self.assertEqual(stale, ["wpt/demo/stale"])
        self.assertEqual(metadata_only, ["wpt/demo/stale"])

    def test_commented_script_is_not_a_javascript_dependency(self):
        html = """<div style="border-radius:25px"></div>
            <!-- <script src="disabled-helper.js"></script> -->"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertEqual(categories, "needs_rounded_border_paint")
        self.assertEqual(dependency, "Paint Quality: Rounded Borders")

    def test_solid_rounded_border_has_precise_non_text_owner(self):
        html = """<style>.box { border: 2px solid #a1a1a1;
                  border-bottom-left-radius: 25px; }</style>
                  <div class="box">retained text</div>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertEqual(categories, "needs_rounded_border_paint")
        self.assertEqual(dependency, "Paint Quality: Rounded Borders")

    def test_multi_value_border_style_has_complex_border_owner(self):
        html = """<style>#test { border: black solid 5px;
                  border-style: solid dotted dashed double; }</style>
                  <div id="test"></div>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertEqual(categories, "needs_complex_border")
        self.assertEqual(dependency, "Paint Quality: Complex Borders")

    def test_positioned_block_through_inline_has_precise_owner(self):
        html = """<p>Test passes if green covers red.</p>
            <div><span style="position:relative;top:100px">
            <div style="position:relative;top:-100%"></div>
            </span></div>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertEqual(categories, "needs_positioned_inline_layout")
        self.assertEqual(dependency, "SP13: Positioned Inline Layout")

    def test_abspos_flex_static_position_has_precise_owner(self):
        html = """<style>.container { display:flex }
            .container > * { position:absolute }</style>
            <div class="container"><div></div></div>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertEqual(categories, "needs_abspos_flex_static_position")
        self.assertEqual(dependency, "SP12: Abspos Flex Static Position")

    def test_float_descendant_of_inline_has_precise_owner(self):
        html = """<style>.left { float:left }</style>
            <p><span>Hello<span class="left"></span></span>Kitty</p>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertEqual(categories, "needs_float_descendant_of_inline")
        self.assertEqual(dependency, "SP13: Float Descendant of Inline")

        direct_float = """<style>span { float:left }</style>
            <p><span></span>content beside the direct float</p>"""
        categories, _dependency = shared_detectors.classify_failure_categories(
            direct_float, excluded={"text_rendering"}
        )
        self.assertNotIn("needs_float_descendant_of_inline", categories)

    def test_float_bfc_phantom_margin_separation_has_precise_owner(self):
        html = """<div style="overflow:hidden"><div>
            <div style="float:left"></div><span></span>
            <div style="margin-top:200px;overflow:hidden"></div>
            </div></div>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertEqual(categories, "needs_float_bfc_phantom_margin_separation")
        self.assertEqual(dependency, "SP12: Float/BFC Phantom Margin Separation")

    def test_inline_box_decoration_break_owner_is_retired(self):
        html = """<style>.slice { box-decoration-break:slice;
                  border:10px solid blue }</style>
                  <div>AAA<span class="slice">AAA<br>AA</span>AA</div>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertNotIn("needs_inline_box_decoration_break", categories)
        self.assertEqual(categories, "sp12_layout_bug")

    def test_clearing_break_after_floats_owner_is_retired(self):
        html = """<style>.container { float:left } br { clear:both }</style>
                  <div class="container"></div><br>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertNotIn("needs_clearing_break_after_floats", categories)
        self.assertEqual(categories, "sp12_layout_bug")

    def test_display_contents_style_element_owner_is_retired(self):
        html = """<style>* { display: contents }</style><br><whatever>PASS</whatever>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertEqual(categories, "non_visual_test")
        self.assertNotIn("SP15", dependency)

    def test_mixed_inline_block_layout_has_precise_successor_owner(self):
        html = """<style>.contents { display:contents }
                  .inline { display:inline }</style>
                  <ul><li><div class="contents"><div class="inline"><div>text</div>
                  </div></div></li></ul>"""
        categories, dependency = shared_detectors.classify_failure_categories(
            html, excluded={"text_rendering"}
        )
        self.assertEqual(categories, "needs_mixed_inline_block_layout")
        self.assertEqual(dependency, "Future SP: Mixed Inline/Block Layout")

        linked_fixture = """<link rel="stylesheet" href="support/acid.css">
            <ul><li><div class="contents"><div class="inline"><div>x</div>
            </div></div></li></ul>"""
        categories, _dependency = shared_detectors.classify_failure_categories(
            linked_fixture, excluded={"text_rendering"}
        )
        self.assertIn("needs_mixed_inline_block_layout", categories.split(","))

    def test_all_sp15_categories_are_retired_from_global_registry(self):
        retired = {
            "needs_inline_box_decoration_break",
            "needs_clearing_break_after_floats",
            "needs_display_contents_style_element",
            "needs_display_contents_list_layout",
            "needs_root_body_layout",
        }
        self.assertFalse(retired & set(shared_detectors.CATEGORY_FOR_DEP.values()))


if __name__ == "__main__":
    unittest.main()
