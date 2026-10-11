"""Retain the complete measured trial without promoting its source or relaxing pixels."""
import gzip, hashlib, io, json, subprocess, tarfile
from pathlib import Path

MAIN = Path('/home/nero/code/open-ui')
RAW = MAIN / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = MAIN / 'docs/renderer/evidence/native-glyph-descriptor-v6'
INDEX = MAIN / 'docs/renderer/generated/native-glyph-descriptor-v6.json'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
load = lambda p: json.loads(p.read_bytes())
owner_path = RAW / 'native-glyph-current-pipeline-v1830/receipt.json'
owner = load(owner_path)
assert owner['all_commands_terminal'] and len(owner['steps']) == 8 and not owner.get('failure')
assert owner['source'] == owner['source_after'] and owner['source']['clean']
native = load(RAW / 'native-glyph-current-native-audit-v1831.json')
matrix = load(RAW / 'native-glyph-current-matrix-audit-v1832.json')
assert native['source'] == matrix['source'] == owner['source']
assert native['all_400_chromium_png_inputs_unchanged'] and matrix['all_chromium_inputs_unchanged']
assert not OUT.exists() and not INDEX.exists()
OUT.mkdir()
rows = []

def keep(p, name, expected=None, compress=False):
    data = p.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if expected:
        assert digest == expected, name
    target = OUT / name
    target.parent.mkdir(parents=True, exist_ok=True)
    assert not target.exists()
    target.write_bytes(gzip.compress(data, mtime=0) if compress else data)
    row = dict(path=str(target.relative_to(MAIN)), sha256=sha(target), bytes=target.stat().st_size,
               source_path=str(p))
    if compress:
        assert gzip.decompress(target.read_bytes()) == data
        row.update(encoding='gzip', decompressed_sha256=digest, decompressed_bytes=len(data))
    rows.append(row)
    return json.loads(data) if p.suffix == '.json' else None

keep(owner_path, 'owner-terminal.json')
for step in owner['steps']:
    keep(owner_path.parent / (step['name'] + '.log'), 'owner-' + step['name'] + '.log.gz', step['log_sha256'], True)
for label in ['baseline', 'candidate']:
    root = Path('/dev/shm/openui-native-glyph-' + label + '-16187f4f-v1825')
    expected = owner['baseline_source'] if label == 'baseline' else owner['source']
    import sys
    sys.path.insert(0, str(MAIN / 'tools/qualification'))
    from renderer_source_identity import repository_source_identity
    assert repository_source_identity(root) == expected
    for kind in ['checks', 'build']:
        path = RAW / ('native-glyph-current-' + label + '-' + kind + ('-v1827/receipt.json' if kind == 'checks' else '-v1828/build.json'))
        record = keep(path, label + '-' + kind + '.json')
        assert record['all_commands_terminal'] and record['source'] == record['source_after'] == expected
        if kind == 'checks':
            assert len(record['checks']) == 15 and all(s['observed_exit_code'] == 0 for s in record['checks'])
        else:
            assert record['observed_exit_code'] == 0
            stages = {s['name']: s for s in record['steps']}
            assert stages['physical-strike-guard']['observed_exit_code'] == (101 if label == 'baseline' else 0)
            if label == 'candidate':
                assert (stages['all-text-tests']['passed'], stages['all-text-tests']['failed'], stages['all-text-tests']['ignored']) == (343, 0, 0)
                assert (stages['workspace']['passed'], stages['workspace']['failed'], stages['workspace']['ignored']) == (8560, 0, 13)
        for step in record['checks' if kind == 'checks' else 'steps']:
            keep(path.parent / (step['name'] + '.log'), label + '-' + kind + '-' + step['name'] + '.log.gz', step['log_sha256'], True)
    consumer_path = RAW / ('native-glyph-current-' + label + '-consumer-v1828/receipt.json')
    consumer = keep(consumer_path, label + '-native-consumer.json.gz', compress=True)
    assert consumer['all_commands_terminal'] and not consumer.get('failure') and consumer['totals']['images'] == 400
    assert consumer['source'] == consumer['source_after'] == expected
    # Preserve all unique native runs, both geometry runs, actual logs, and
    # the unchanged reference image for every measured state. Repeated image
    # bytes are validated before their duplicate copies are omitted.
    selected = {}
    def select(p):
        selected[str(p.relative_to(consumer_path.parent))] = p
    for case in consumer['cases']:
        directory = consumer_path.parent / case['raster'] / (case['family'].replace(' ', '-') + '-' + str(case['size'])) / str(case['scale'])
        for run in case['native_runs']:
            assert run['observed_exit_code'] == 0
            log = directory / ('native-' + str(run['repeat']) + '.log')
            assert sha(log) == run['log_sha256']
            select(log)
            select(directory / ('native-' + str(run['repeat'])) / 'geometry.json')
        for image in case['images']:
            state = image['state']
            actual = directory / 'native-1' / (state + '.png')
            repeated = directory / 'native-2' / (state + '.png')
            reference = directory / (state + '-chromium-1.png')
            reference_repeat = directory / (state + '-chromium-2.png')
            assert sha(actual) == sha(repeated) == image['native_png_sha256']
            assert sha(reference) == sha(reference_repeat) == image['chromium_png_sha256']
            select(actual)
            select(reference)
    for entry in consumer['inputs'].values():
        path = Path(entry['path'])
        assert sha(path) == entry['sha256']
        select(path)
    manifest = []
    content_names = {}
    target = OUT / (label + '-native-artifacts.tar.gz')
    with target.open('xb') as stream, gzip.GzipFile(fileobj=stream, mode='wb', mtime=0, filename='') as compressed, tarfile.open(fileobj=compressed, mode='w|') as archive:
        for name, path in sorted(selected.items()):
            data = path.read_bytes()
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = 0o644
            digest = hashlib.sha256(data).hexdigest()
            if digest in content_names:
                info.type = tarfile.LNKTYPE
                info.linkname = content_names[digest]
                info.size = 0
                archive.addfile(info)
            else:
                content_names[digest] = name
                archive.addfile(info, io.BytesIO(data))
            manifest.append(dict(path=name, sha256=digest, bytes=len(data)))
    with tarfile.open(target, 'r:gz') as archive:
        assert archive.getnames() == [r['path'] for r in manifest]
        for entry in manifest:
            assert hashlib.sha256(archive.extractfile(entry['path']).read()).hexdigest() == entry['sha256']
    rows.append(dict(path=str(target.relative_to(MAIN)), sha256=sha(target), bytes=target.stat().st_size,
                     encoding='tar-gzip', member_count=len(manifest), members=manifest))
    for source in ['bindings/rust/openui/examples/native_glyph_coverage.rs', 'bindings/rust/openui-text/src/shaping/shape_result.rs', 'bindings/rust/openui-paint/src/text_painter.rs']:
        keep(root / source, label + '-source/' + source)

for suite in ['focused', 'primitive', 'full', 'expanded']:
    summary = RAW / ('native-glyph-current-clean-' + suite + '-v1828') / (suite + '-summary.json')
    keep(summary, suite + '-summary.json.gz', matrix['suites'][suite]['summary_sha256'], True)
    exit_path = RAW / ('native-glyph-current-' + suite + '-exit-v1828.json')
    record = keep(exit_path, suite + '-actual-exit.json', matrix['suites'][suite]['actual_exit_receipt_sha256'])
    keep(RAW / ('native-glyph-current-clean-' + suite + '-v1828.log'), suite + '.log.gz', record['log_sha256'], True)
for name in ['native-audit-v1831', 'matrix-audit-v1832']:
    keep(RAW / ('native-glyph-current-' + name + '.json'), name + '.json.gz', compress=True)
original = keep(RAW / 'native-glyph-original-audit-v1843.json', 'original-audit.json.gz', compress=True)
assert original['source'] == owner['source'] and original['original_suite_complete']
assert (original['exact_gains'], original['exact_losses'], original['changed_comparisons']) == (matrix['suites']['full']['exact_gains'], matrix['suites']['full']['exact_losses'], matrix['suites']['full']['changed_comparisons'])
keep(Path('/tmp/openui-native-glyph-original-audit-v1843.py'), 'probes/original-audit.py', original['probe_sha256'])
keep(RAW / 'native-glyph-authored-routing-review-v1835.json', 'authored-routing-review.json', 'a5f67d4d227a76b9b274484ddd61e6fda4d4278f9496f41be6854c51eb0d91fe')
keep(RAW / 'native-glyph-font-engine-review-v1837.json', 'font-engine-reference-review.json', '70b4f734d98a52ddf7a191ef53576b90e24bbe4a3ad2c083568d7ed232ef02c5')
keep(Path('/tmp/openui-native-glyph-font-engine-review-v1837.py'), 'probes/font-engine-reference-review.py')
keep(RAW / 'native-glyph-current-source-v1826.json', 'source-preparation.json')
keep(RAW / 'native-glyph-current-preparation-conflict-v1826.txt', 'preparation-conflict.txt')
for name in ['current-prepare-v1826', 'current-baseline-checks-v1827', 'current-candidate-checks-v1827',
             'current-build-v1828', 'current-consumer-v1828', 'current-matrices-v1828', 'current-pipeline-v1830',
             'current-native-audit-v1831', 'current-matrix-audit-v1832']:
    keep(Path('/tmp/openui-native-glyph-' + name + '.py'), 'probes/' + name + '.py')
keep(Path(__file__), 'preserver.py')
patch = subprocess.check_output(['git', 'diff', owner['baseline_source']['commit'], owner['source']['commit'], '--',
                               'bindings/rust/openui-text/src/shaping/shape_result.rs', 'bindings/rust/openui-paint/src/text_painter.rs'], cwd=MAIN)
patch_path = OUT / 'physical-strike.patch'
patch_path.write_bytes(patch)
rows.append(dict(path=str(patch_path.relative_to(MAIN)), sha256=sha(patch_path), bytes=len(patch)))
record = dict(schema_version=1, source=owner['source'], source_after=owner['source_after'], baseline_source=owner['baseline_source'],
              all_eight_stages_terminal=True, actual_stage_exits={r['name']: r['observed_exit_code'] for r in owner['steps']},
              baseline_guard_actual_exit=101, candidate_guard_actual_exit=0, candidate_text_tests_passed=343,
              candidate_workspace_tests_passed=8560, candidate_workspace_tests_failed=0, candidate_workspace_tests_ignored=13,
              read_only_checks_passed_per_source=15, current_c_exports=113, current_c_layouts=30,
              actual_c_consumers=13, actual_cpp_consumers=7, native_totals=native['totals'], native_groups=native['groups'],
              default_native_pixels_unchanged=True, all_native_pairs_deterministic=True, all_chromium_inputs_unchanged=True,
              native_image_regressions_do_not_qualify_the_candidate=True, suites=matrix['suites'],
              addition_cases=201, addition_cases_four_profile_exact=matrix['addition_cases_four_profile_exact'],
              candidate_has_exact_regressions=matrix['candidate_has_exact_regressions'], candidate_applied_to_umbrella=False,
              authored_subpixel_text_can_bypass_fontations=True, runtime_dispatch_trace_captured=False,
              original_real_font_reference_engine='Freetype', native_reference_font_engine='Fontations',
              explicit_fontations_constructor_missing=True,
              reviewed_root_cause_for_every_residual=False, new_formal_wpt_owners_assigned=0,
              native_artifacts_retained='All first-run native PNGs, both geometry runs, actual process logs, input HTML, and unchanged first reference PNGs. All second PNG/reference copies are hash-verified identical.',
              whole_owner_lock_covered_all_stages_and_gaps=True, javascript_executed_by_openui=False,
              public_native_rust_apis_required=True, pixel_tolerance=0, new_release_states_admitted=0,
              release_qualification=False, artifacts=rows, artifact_count=len(rows), artifact_bytes=sum(r['bytes'] for r in rows))
INDEX.write_text(json.dumps(record, sort_keys=True, indent=2) + '\n')
print(json.dumps({'artifacts':len(rows), 'artifact_bytes':record['artifact_bytes'], 'index_sha256':sha(INDEX)}))
