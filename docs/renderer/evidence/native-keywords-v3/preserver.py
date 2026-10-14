"""Retain completed hosted jobs and the width-only native image audit."""
import gzip
import hashlib
import json
import re
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
sha = lambda b: hashlib.sha256(b).hexdigest()
def preserve(name, inputs, report):
    out = ROOT / 'docs/renderer/evidence' / name
    index = ROOT / 'docs/renderer/generated' / (name + '.json')
    assert not out.exists() and not index.exists()
    out.mkdir()
    rows = []
    for source, target, expected, compress in inputs + [(Path(__file__), 'preserver.py', None, False)]:
        source = Path(source)
        data = source.read_bytes()
        if expected:
            assert sha(data) == expected
        p = out / target
        assert not p.exists()
        p.write_bytes(gzip.compress(data, mtime=0) if compress else data)
        row = dict(path=str(p.relative_to(ROOT)), sha256=sha(p.read_bytes()), bytes=p.stat().st_size, source_path=str(source))
        if compress:
            row.update(encoding='gzip', decompressed_sha256=sha(data), decompressed_bytes=len(data))
            assert gzip.decompress(p.read_bytes()) == data
        rows.append(row)
    report.update(schema_version=1, artifacts=rows, artifact_count=len(rows), artifact_bytes=sum(r['bytes'] for r in rows),
                  javascript_executed_by_openui=False, public_native_rust_apis_required=True,
                  release_qualification=False, pixel_tolerance=0, new_release_states_admitted=0)
    index.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
    print(json.dumps(dict(index=str(index.relative_to(ROOT)), sha256=sha(index.read_bytes()), artifacts=len(rows))), flush=True)

for name, version, source, expected in [
    ('native-glyph-descriptor', 1760, '2f53d5de2e6fce9b4c9dc2fbc94fcfc763544b23', '71978a8a9952a1d5db19fcb2ee7e0492516a21fd5a570ef4fbfd276bf0412b59'),
    ('native-keywords-current', 1762, '7d6ffabfea1f72c90f97475488dba8a8058ac4c7', '2272da9b41553fd38e81e837664ebd646f91b62db9768edf838d9e286ec4559b'),
]:
    p = RAW / (name + '-hosted-v' + str(version)) / 'receipt.json'
    d = json.loads(p.read_bytes())
    assert sha(p.read_bytes()) == expected and d['source'] == source
    assert d['all_commands_terminal'] and d['all_seven_jobs_passed'] and d['job_conclusions'] == dict(success=7)
    inputs = [(p, 'hosted-terminal.json', expected, False),
              (p.parent / 'attempt1.json', 'hosted-attempt1.json', d['metadata_sha256'], False),
              (Path('/tmp/openui-' + name + '-hosted-v' + str(version) + '.py'), 'hosted-collector.py', d['probe_sha256'], False)]
    for job in d['job_logs']:
        assert job['capture_actual_exit'] == 0
        inputs.append((Path(job['path']), str(job['job_id']) + '.log.gz', job['sha256'], True))
    if name == 'native-glyph-descriptor':
        preserve('native-glyph-descriptor-v4', inputs,
                 dict(source=source, all_seven_hosted_jobs_passed=True, skipped_jobs=0,
                      all_available_job_logs_preserved=True, full_text_crate_unit_tests_not_executed_by_hardening=True,
                      absent_test_constructor_still_prevents_full_text_tests=True,
                      fresh_corrected_fce42e08_not_qualified_by_previous_source_hosted_run=True,
                      clean_local_glyph_guards_build_and_pixels_unexecuted=True, candidate_applied_to_umbrella=False))
    else:
        job = next(j for j in d['job_logs'] if j['job'] == 'conformance-and-platform')
        content = re.sub(rb'\x1b\[[0-9;]*m', b'', Path(job['path']).read_bytes())
        named = ['tests::native_fragment_keywords_reach_engine_and_keep_owned_property_identity',
                 'tests::native_table_display_literals_keep_existing_scalar_encoding']
        assert all(('test ' + n + ' ... ok').encode() in content for n in named)
        preserve('native-keywords-v3', inputs,
                 dict(source=source, all_seven_hosted_jobs_passed=True, skipped_jobs=0,
                      all_available_job_logs_preserved=True, both_named_ffi_keyword_guards_pass_in_hosted_platform_job=True,
                      selected_named_guards=named, source_identified_local_baseline_and_fixed_guards_still_required=True,
                      clean_local_build_consuming_applications_and_pixels_unexecuted=True, candidate_applied_to_umbrella=False))

p = RAW / 'native-width-image-audit-v1761.json'
d = json.loads(p.read_bytes())
assert sha(p.read_bytes()) == 'cecd0b7a36fce58aab195b256082a42103265e65c2ae534d42eda2404a365e38'
assert d['corrected_geometry_states'] == 3840 and d['all_600_native_png_hashes_unchanged']
assert d['all_600_chromium_png_hashes_unchanged'] and d['all_600_difference_analyses_unchanged']
preserve('native-intrinsic-snap-v6',
         [(p, 'native-width-image-audit.json.gz', sha(p.read_bytes()), True),
          (Path('/tmp/openui-native-width-image-audit-v1761.py'), 'native-width-image-audit.py', d['probe_sha256'], False)],
         dict(source=d['source'], prior_source=d['prior_source'], corrected_geometry_states=3840,
              exact_width_delta=1 / 64, all_600_native_png_hashes_unchanged=True,
              all_600_chromium_png_hashes_unchanged=True, all_600_difference_analyses_unchanged=True,
              all_38400_chromium_bounds_unchanged=True, screenshots_generated=0,
              full_original_and_expanded_results_still_required=True, candidate_applied_to_umbrella=False))
