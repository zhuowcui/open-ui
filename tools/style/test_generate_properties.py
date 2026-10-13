"""Fault guards for the canonical ownership generator and read-only gate."""

from __future__ import annotations

import csv
import hashlib
import importlib.util
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("generate_properties.py")
SPEC = importlib.util.spec_from_file_location("generate_style_properties", SCRIPT)
assert SPEC and SPEC.loader
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class CanonicalOwnershipGeneratorTests(unittest.TestCase):
    def mutated_schema(self, mutate):
        rows = list(csv.DictReader(GENERATOR.CANONICAL_SCHEMA.read_text().splitlines()))
        mutate(rows)
        stream = io.StringIO()
        writer = csv.DictWriter(stream, fieldnames=["property", "canonical_groups", "mutation_fields"])
        writer.writeheader()
        writer.writerows(rows)
        return stream.getvalue()

    def rejected(self, data, message):
        with tempfile.TemporaryDirectory() as directory:
            schema = Path(directory) / "canonical-style-fields.csv"
            schema.write_text(data)
            with patch.object(GENERATOR, "CANONICAL_SCHEMA", schema):
                with self.assertRaisesRegex(SystemExit, message):
                    GENERATOR.outputs()

    def test_missing_or_duplicate_owner_is_a_generator_failure(self):
        self.rejected(self.mutated_schema(lambda rows: rows.pop()), "every property exactly once")
        self.rejected(self.mutated_schema(lambda rows: rows.__setitem__(1, rows[0])), "every property exactly once")

    def test_unknown_group_and_inconsistent_write_set_are_failures(self):
        self.rejected(self.mutated_schema(lambda rows: rows[0].__setitem__("canonical_groups", "MissingOwner")), "invalid canonical groups")
        self.rejected(self.mutated_schema(lambda rows: rows[0].__setitem__("mutation_fields", "unknown_field")), "mutation fields disagree")

    def test_invalid_corner_selector_cannot_escape_the_generator(self):
        schema = GENERATOR.rows()
        groups = GENERATOR.canonical_group_fields(schema)
        groups["BorderTopLeftRadius"] = ["border_radius_lengths[4]"]
        text = self.mutated_schema(lambda rows: [
            row.__setitem__("mutation_fields", ";".join(dict.fromkeys(
                field for name in row["canonical_groups"].split(";") for field in groups[name]
            ))) for row in rows
        ])
        with patch.object(GENERATOR, "canonical_group_fields", return_value=groups):
            self.rejected(text, "invalid canonical array projection")

    def test_summaries_do_not_own_longhands_and_native_flag_overrides_are_explicit(self):
        schema = GENERATOR.rows()
        rows = {row["property"]: row for row in GENERATOR.canonical_rows(schema)}
        self.assertEqual(rows["WhiteSpace"]["canonical_groups"], "WhiteSpaceCollapse;TextWrapMode")
        self.assertEqual(rows["TextWrap"]["canonical_groups"], "TextWrapMode;TextWrapStyle")
        self.assertEqual(rows["WordWrap"]["canonical_groups"], "OverflowWrap")
        self.assertEqual(rows["BorderRadiusPercent"]["canonical_groups"], "BorderRadiusPercent")
        self.assertNotIn("FontSynthesisPosition", rows["FontSynthesis"]["canonical_groups"])
        self.assertNotIn("TextEmphasisPosition", rows["TextEmphasis"]["canonical_groups"])
        self.assertIn("BorderImage", rows["Border"]["canonical_groups"].split(";"))

    def test_real_cli_check_rejects_drift_without_writing_outputs(self):
        generated = GENERATOR.outputs()
        inputs = [GENERATOR.SCHEMA, GENERATOR.INTERNAL_SCHEMA, GENERATOR.CANONICAL_SCHEMA, GENERATOR.COMPUTED,
                  GENERATOR.ROOT / "bindings/rust/openui-style/src/property.rs",
                  GENERATOR.ROOT / "bindings/rust/openui-style/src/enums.rs",
                  GENERATOR.ROOT / "bindings/rust/openui-style/src/layout_systems.rs"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target_script = root / "tools/style/generate_properties.py"
            for source in [SCRIPT, *inputs]:
                target = root / source.relative_to(GENERATOR.ROOT)
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(source.read_bytes())
            outputs = []
            for source, data in generated.items():
                target = root / source.relative_to(GENERATOR.ROOT)
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                outputs.append(target)
            outputs[0].write_bytes(outputs[0].read_bytes() + b"\n// deliberate copied drift\n")
            before = {path: (hashlib.sha256(path.read_bytes()).hexdigest(), path.stat().st_mtime_ns) for path in outputs}
            actual = subprocess.run([sys.executable, str(target_script), "--check"], capture_output=True, text=True)
            self.assertNotEqual(actual.returncode, 0)
            self.assertIn("generated style schema drift", actual.stderr)
            after = {path: (hashlib.sha256(path.read_bytes()).hexdigest(), path.stat().st_mtime_ns) for path in outputs}
            self.assertEqual(before, after)


if __name__ == "__main__":
    unittest.main()
