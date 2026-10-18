"""Keep corrected glyph tests and current native keyword source as unqualified evidence."""
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
sha = lambda b: hashlib.sha256(b).hexdigest()
def preserve(name, files, report):
    out = ROOT / 'docs/renderer/evidence' / name
    index = ROOT / 'docs/renderer/generated' / (name + '.json')
    assert not out.exists() and not index.exists()
    out.mkdir()
    rows = []
    for source, target, expected, compress in files + [(Path(__file__), 'preserver.py', None, False)]:
        source = Path(source)
        b = source.read_bytes()
        if expected:
            assert sha(b) == expected, str(source)
        p = out / target
        assert not p.exists()
        p.write_bytes(gzip.compress(b, mtime=0) if compress else b)
        row = dict(path=str(p.relative_to(ROOT)), sha256=sha(p.read_bytes()), bytes=p.stat().st_size,
                   source_path=str(source))
        if compress:
            row.update(encoding='gzip', decompressed_sha256=sha(b), decompressed_bytes=len(b))
            assert gzip.decompress(p.read_bytes()) == b
        rows.append(row)
    report.update(schema_version=1, artifacts=rows, artifact_count=len(rows), artifact_bytes=sum(r['bytes'] for r in rows),
                  javascript_executed_by_openui=False, public_native_rust_apis_required=True,
                  pixel_tolerance=0, release_qualification=False, new_release_states_admitted=0)
    index.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
    print(json.dumps(dict(index=str(index.relative_to(ROOT)), sha256=sha(index.read_bytes()), artifacts=len(rows))), flush=True)

checks_path = RAW / 'native-glyph-descriptor-corrected-checks-v1755/receipt.json'
checks = json.loads(checks_path.read_bytes())
source_path = RAW / 'native-glyph-descriptor-corrected-source-v1754.json'
source = json.loads(source_path.read_bytes())
assert checks['all_commands_terminal'] and checks['source'] == checks['source_after'] == source['source']
assert source['source']['clean'] and source['source']['commit'] == 'fce42e086dee48f6e2d725b2b4f3bb8815e79e5c'
assert len(checks['checks']) == 14 and all(r['observed_exit_code'] == 0 for r in checks['checks'])
assert source['correction_is_test_only'] and source['native_build_guards_and_pixels_unexecuted']
files = [(source_path, 'corrected-source.json', '5fc201c39165d7e167a0efe4d1eaa12c72e01786bdafb83b4403a24e3e2a8be1', False),
         (RAW / 'native-glyph-descriptor-absent-test-api-v1754.json', 'previous-source-test-api-finding.json', None, False),
         (Path(source['patch_path']), 'corrected-candidate.patch', source['patch_sha256'], False),
         (checks_path, 'corrected-source-checks.json', sha(checks_path.read_bytes()), False),
         (Path('/tmp/openui-native-glyph-descriptor-corrected-checks-v1755.py'), 'corrected-source-checks.py', checks['probe_sha256'], False)]
files.extend((checks_path.parent / (r['name'] + '.log'), r['name'] + '.log.gz', r['log_sha256'], True) for r in checks['checks'])
preserve('native-glyph-descriptor-v3', files,
         dict(source=source['source'], baseline_commit=source['baseline_commit'], all_14_read_only_checks_passed=True,
              previous_test_source_cannot_compile_due_to_absent_constructor=True,
              correction_uses_existing_public_raster_constructor=True, production_code_unchanged_from_2f53d5de=True,
              native_guards_build_and_pixels_unexecuted=True, candidate_applied_to_umbrella=False))

source_path = RAW / 'native-keywords-current-source-v1750.json'
source = json.loads(source_path.read_bytes())
checks_path = Path(source['source_checks_path'])
checks = json.loads(checks_path.read_bytes())
assert sha(source_path.read_bytes()) == '3f9272bfcae70101ba7981d6f3e29ab99bace450b5a380c775fe5355e6e21597'
assert sha(checks_path.read_bytes()) == source['source_checks_sha256']
assert checks['all_commands_terminal'] and checks['source'] == checks['source_after'] == source['source']
assert len(checks['checks']) == 16 and all(r['observed_exit_code'] == 0 for r in checks['checks'])
files = [(source_path, 'current-source.json', sha(source_path.read_bytes()), False),
         (Path(source['patch_path']), 'current-source.patch', source['patch_sha256'], False),
         (checks_path, 'current-source-checks.json', sha(checks_path.read_bytes()), False),
         (Path('/tmp/openui-native-keywords-current-checks-v1749.py'), 'current-source-checks.py', checks['probe_sha256'], False),
         (RAW / 'native-keywords-current-format-correction-v1747.json', 'format-correction.json', None, False),
         (RAW / 'native-keywords-current-format-failed-v1747.log', 'format-failed.log.gz', None, True)]
files.extend((checks_path.parent / (r['name'] + '.log'), r['name'] + '.log.gz', r['log_sha256'], True) for r in checks['checks'])
owner_path = RAW / 'native-keywords-pipeline-v1588/receipt.json'
owner = json.loads(owner_path.read_bytes())
guards_path = RAW / 'native-keywords-guards-v1587/receipt.json'
guards = json.loads(guards_path.read_bytes())
assert owner['all_commands_terminal'] and owner['steps'][0]['observed_exit_code'] == 241
assert guards['all_commands_terminal'] and guards['baseline_regression_reproduced']
assert [r['observed_exit_code'] for r in guards['steps']] == [0, 101, 0, -15, -15, -15]
assert all(r['disk_guard_triggered'] for r in guards['steps'][3:])
files += [(owner_path, 'previous-whole-owner-terminal.json', sha(owner_path.read_bytes()), False),
          (guards_path, 'previous-guards-terminal.json', sha(guards_path.read_bytes()), False),
          (Path('/tmp/openui-native-keywords-guards-v1587.py'), 'previous-guards.py', guards['probe_sha256'], False)]
files.extend((guards_path.parent / (r['name'] + '.log'), 'previous-' + r['name'] + '.log.gz', r['log_sha256'], True) for r in guards['steps'])
p = RAW / 'native-keywords-current-dispatch-v1753.json'
dispatch = json.loads(p.read_bytes())
files += [(p, 'current-hosted-dispatch.json', 'c04e811d7bda839100eb3ec96d34f5ca8e82ef9971f636510325d5fd8dae79c6', False),
          (Path('/tmp/openui-native-keywords-current-dispatch-v1753.py'), 'current-hosted-dispatch.py', dispatch['probe_sha256'], False)]
files.extend((Path(r['log_path']), 'dispatch-' + r['name'] + '.log', r['log_sha256'], False) for r in dispatch['steps'])
preserve('native-keywords-v2', files,
         dict(source=source['source'], baseline_commit=source['baseline_commit'], all_16_read_only_checks_passed=True,
              generated_properties_regenerated=True, current_c_cpp_consumers_formatted=True,
              all_113_exports_and_30_layouts_preserved=True, native_text_consumers_added_to_cmake=True,
              previous_named_baseline_failure_reproduced=True, previous_three_fixed_guards_stopped_on_disk=True,
              previous_fixed_guard_failures_not_counted_as_passes=True,
              current_native_build_guards_and_pixels_unexecuted=True, current_hosted_run=37388643017,
              hosted_run_not_qualified_by_dispatch=True, candidate_applied_to_umbrella=False))
