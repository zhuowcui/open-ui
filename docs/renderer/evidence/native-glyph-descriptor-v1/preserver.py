"""Preserve completed CI and native glyph analysis without claiming qualification."""
import gzip
import hashlib
import json
import subprocess
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
        original = source.read_bytes()
        if expected:
            assert sha(original) == expected, str(source)
        destination = out / target
        assert not destination.exists()
        destination.write_bytes(gzip.compress(original, compresslevel=9, mtime=0) if compress else original)
        row = dict(path=str(destination.relative_to(ROOT)), sha256=sha(destination.read_bytes()),
                   bytes=destination.stat().st_size, source_path=str(source))
        if compress:
            row.update(encoding='gzip', decompressed_sha256=sha(original), decompressed_bytes=len(original))
        rows.append(row)
    report.update(schema_version=1, artifacts=rows, artifact_count=len(rows), artifact_bytes=sum(r['bytes'] for r in rows),
                  javascript_executed_by_openui=False, public_native_rust_apis_required=True,
                  release_qualification=False, pixel_tolerance=0, new_release_states_admitted=0)
    index.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
    for row in rows:
        data = (ROOT / row['path']).read_bytes()
        assert sha(data) == row['sha256']
        if row.get('encoding') == 'gzip':
            assert sha(gzip.decompress(data)) == row['decompressed_sha256']
    print(json.dumps(dict(index=str(index.relative_to(ROOT)), sha256=sha(index.read_bytes()),
                          artifacts=len(rows), bytes=report['artifact_bytes'])), flush=True)

checks_path = RAW / 'native-text-style-final-checks-v1730/receipt.json'
checks = json.loads(checks_path.read_bytes())
assert sha(checks_path.read_bytes()) == 'd601e3a2a34c0cc72dd9ef58d6fcecb5ead40f831f4f1453e5b21fae6e7a3ff7'
assert checks['all_commands_terminal'] and checks['source'] == checks['source_after']
assert checks['source']['commit'] == '0733955a8f6a94053f3a253c33219e84ed9702c0'
assert len(checks['checks']) == 14 and all(r['observed_exit_code'] == 0 for r in checks['checks'])
inputs = [(checks_path, 'source-checks-terminal.json', sha(checks_path.read_bytes()), False),
          (Path('/tmp/openui-native-text-style-final-checks-v1730.py'), 'source-checks.py', checks['probe_sha256'], False)]
for row in checks['checks']:
    inputs.append((checks_path.parent / (row['name'] + '.log'), 'source-' + row['name'] + '.log.gz', row['log_sha256'], True))
p = RAW / 'native-current-hosted-v1739/receipt.json'
hosted = json.loads(p.read_bytes())
assert sha(p.read_bytes()) == 'ffd93958591507e4560a2fc3884dfb926409a8f28cb443f082c85604ccfd5f0f'
assert hosted['all_commands_terminal'] and hosted['successful_jobs'] == 6 and hosted['skipped_jobs'] == 5 and hosted['failed_jobs'] == 0
inputs += [(p, 'hosted-terminal.json', sha(p.read_bytes()), False),
           (Path('/tmp/openui-native-current-hosted-collect-v1739.py'), 'hosted-collector.py', hosted['probe_sha256'], False)]
for run in hosted['workflows']:
    inputs.append((Path(run['path']), str(run['run_id']) + '.json', run['sha256'], False))
for job in hosted['job_logs']:
    if job['conclusion'] != 'skipped':
        assert job['capture_actual_exit'] == 0
        inputs.append((Path(job['path']), str(job['job_id']) + '.log.gz', job['sha256'], True))
preserve('native-text-inheritance-v8', inputs,
         dict(source_checked=checks['source'], all_14_source_checks_passed=True,
              all_three_hosted_workflows_terminal=True, successful_jobs=6, skipped_jobs=5, failed_jobs=0,
              skipped_jobs_do_not_satisfy_release_gates=True, all_available_job_logs_preserved=True,
              previous_format_failure_preserved_in_v7=True, full_chromium_pixel_gate_still_failed=True))

inputs = []
for version, digest in [(1735, '50e68421f2b5658521639de41f73f3a303035e68ddc4bb0b305bacd210493fc9'),
                        (1736, '161411b99c4b67f7ce35e004b41982165ad9ca439ea8664d1ce317b005b414db')]:
    p = RAW / ('native-default-glyph-policy-audit-v' + str(version) + '.json')
    audit = json.loads(p.read_bytes())
    assert sha(p.read_bytes()) == digest
    inputs += [(p, 'coverage-audit-v' + str(version) + '.json', digest, False),
               (Path('/tmp/openui-native-default-glyph-policy-audit-v' + str(version) + '.py'),
                'coverage-audit-v' + str(version) + '.py', audit['probe_sha256'], False)]
assert audit['existing_images_audited'] == 200 and audit['native_images_with_channel_specific_coverage'] == 0
assert audit['chromium_images_with_channel_specific_coverage'] == 184
assert audit['all_images_opaque'] and not audit['formal_wpt_residual_ownership_changed']
for size, scale in [(12, 1.0), (24, 3.0)]:
    row = next(r for r in audit['rows'] if r['family'] == 'DejaVu Sans' and r['size'] == size and r['scale'] == scale and r['state'] == 'before')
    for language in ['native', 'chromium']:
        inputs.append((Path(row[language + '_path']), language + '-' + str(size) + '-' + str(scale) + '.png',
                       row[language + '_png_sha256'], False))
source_path = Path('/home/nero/.cargo/git/checkouts/rust-skia-b4d79b6a888cdb4c/a31b86b/skia-bindings/skia/src/core/SkScalerContext.cpp')
source = source_path.read_bytes()
excerpt = RAW / 'native-glyph-skia-lcd-size-excerpt-v1741.txt'
assert not excerpt.exists()
excerpt.write_bytes(b''.join(source.splitlines(keepends=True)[1050:1125]))
assert b'SK_MAX_SIZE_FOR_LCDTEXT' in excerpt.read_bytes() and b'too_big_for_lcd' in excerpt.read_bytes()
inputs.append((excerpt, 'skia-lcd-size-source-excerpt.txt', sha(excerpt.read_bytes()), False))
p = RAW / 'native-glyph-descriptor-source-v1740.json'
candidate = json.loads(p.read_bytes())
assert sha(p.read_bytes()) == '871909658bda0be5c7af6ae284d4514da313829312f2646395c7c6a32d0e5de4'
assert candidate['glyph_commands_executed'] == 0 and candidate['screenshots_generated'] == 0
inputs += [(p, 'candidate-source.json', sha(p.read_bytes()), False),
           (Path('/tmp/openui-native-glyph-descriptor-source-v1740.py'), 'candidate-freezer.py', candidate['probe_sha256'], False),
           (Path(candidate['patch_path']), 'candidate.patch', candidate['patch_sha256'], False),
           (RAW / 'native-glyph-descriptor-preflight-failed-v1738.json', 'draft-placement-preflight-failed.json', None, False),
           (RAW / 'native-glyph-descriptor-test-placement-failed-v1738.rs', 'draft-placement-failed.rs.gz',
            'e7478d340d7516cd71998184f1f02050cacd8553196aa5ea73a5a398018ff7e9', True)]
preserve('native-glyph-descriptor-v1', inputs,
         dict(audited_source=audit['source'], existing_images_audited=200,
              native_channel_coverage_images=0, chromium_channel_coverage_images=184,
              chromium_grayscale_images_with_ink=16, no_new_captures_or_renders=True,
              formal_wpt_residual_ownership_unchanged=True,
              skia_source_path=str(source_path), skia_source_sha256=sha(source), source_excerpt_lines=[1051, 1125],
              candidate_source=candidate['source'], baseline_source=candidate['baseline_source'],
              candidate_build_and_pixels_unexecuted=True, candidate_applied_to_umbrella=False,
              raster_defaults_unchanged=True, previous_mutable_draft_failure_preserved=True,
              glyph_policy_and_full_pixel_gaps_still_open=True))
