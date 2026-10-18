"""Preserve actual source-qualified descriptor guard results without admitting pixels."""
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = ROOT / 'docs/renderer/evidence/native-glyph-descriptor-v5'
INDEX = ROOT / 'docs/renderer/generated/native-glyph-descriptor-v5.json'
assert not OUT.exists() and not INDEX.exists()
OUT.mkdir()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
p = RAW / 'native-glyph-descriptor-guards-v1764/receipt.json'
d = json.loads(p.read_bytes())
assert sha(p) == 'dc11460af53cd8f1433c363ccdf3a3532577ea2b7afed008a2b76acaff6b3fe1'
assert d['all_commands_terminal'] and d['observed_exit_code'] == 0 and d['baseline_regression_reproduced']
assert d['source'] == d['source_after'] and d['source']['commit'] == 'fce42e086dee48f6e2d725b2b4f3bb8815e79e5c'
assert [s['actual_exit'] for s in d['steps']] == [0, 101, 0, 0, 0]
assert d['steps'][-1]['passed'] == 343 and d['steps'][-1]['failed'] == d['steps'][-1]['ignored'] == 0
assert not any(s['disk_guard_triggered'] for s in d['steps'])
inputs = [(p, 'guards-terminal.json', sha(p), False),
          (Path('/tmp/openui-native-glyph-descriptor-guards-v1764.py'), 'guards.py', d['probe_sha256'], False)]
inputs.extend((p.parent / (s['name'] + '.log'), s['name'] + '.log.gz', s['log_sha256'], True) for s in d['steps'])
inputs.append((Path(__file__), 'preserver.py', sha(Path(__file__)), False))
rows = []
for source, name, expected, compress in inputs:
    assert sha(source) == expected
    b = source.read_bytes()
    q = OUT / name
    q.write_bytes(gzip.compress(b, mtime=0) if compress else b)
    row = dict(path=str(q.relative_to(ROOT)), sha256=sha(q), bytes=q.stat().st_size, source_path=str(source))
    if compress:
        row.update(encoding='gzip', decompressed_sha256=expected, decompressed_bytes=len(b))
        assert gzip.decompress(q.read_bytes()) == b
    rows.append(row)
report = dict(schema_version=1, source=d['source'], source_after=d['source_after'], baseline_source=d['baseline_source'],
              all_five_guard_stages_terminal=True, actual_stage_exits=[0, 101, 0, 0, 0],
              named_baseline_regression_reproduced=True, fixed_named_guard_passed=True,
              text_tests_passed=343, text_tests_failed=0, text_tests_ignored=0,
              workspace_packages_cleaned_at_each_source=18, whole_owner_lock_covered_all_stages_and_gaps=True,
              clean_full_workspace_native_consumers_and_pixel_matrices_still_required=True,
              native_images_generated=0, candidate_applied_to_umbrella=False,
              javascript_executed_by_openui=False, public_native_rust_apis_required=True,
              pixel_tolerance=0, release_qualification=False, new_release_states_admitted=0,
              artifacts=rows, artifact_count=len(rows), artifact_bytes=sum(r['bytes'] for r in rows))
INDEX.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)), sha256=sha(INDEX), artifacts=len(rows))), flush=True)
