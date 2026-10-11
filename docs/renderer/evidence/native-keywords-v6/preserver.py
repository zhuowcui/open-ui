"""Retain exact native callback images and all complete census outcomes."""
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = ROOT / 'docs/renderer/evidence/native-keywords-v6'
INDEX = ROOT / 'docs/renderer/generated/native-keywords-v6.json'
assert not OUT.exists() and not INDEX.exists()
OUT.mkdir()
rows = []
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

def keep(path, name, expected=None, compress=False):
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if expected:
        assert digest == expected
    target = OUT / name
    target.parent.mkdir(parents=True, exist_ok=True)
    assert not target.exists()
    target.write_bytes(gzip.compress(data, mtime=0) if compress else data)
    row = dict(path=str(target.relative_to(ROOT)), sha256=sha(target),
               bytes=target.stat().st_size, source_path=str(path))
    if compress:
        assert gzip.decompress(target.read_bytes()) == data
        row.update(encoding='gzip', decompressed_sha256=digest, decompressed_bytes=len(data))
    rows.append(row)
    return json.loads(data) if path.suffix == '.json' else None

owners = []
seen_probes = set()
for name, exits in [('native-keywords-capture-pipeline-v1786', [0,1,1,0,0,1,1]),
                    ('native-keywords-native-retry-pipeline-v1788', [0,0,0])]:
    owner = keep(RAW/name/'receipt.json', name+'/receipt.json')
    assert owner['all_commands_terminal']
    assert [s['observed_exit_code'] for s in owner['steps']] == exits
    owners.append(owner)
    for step in owner['steps']:
        keep(RAW/name/(step['name']+'.log'), name+'/'+step['name']+'.log.gz',
             step['log_sha256'], True)
    for path, digest in owner['immutable_probe_hashes'].items():
        if path not in seen_probes:
            keep(Path(path), 'probes/'+Path(path).name, digest)
            seen_probes.add(path)
    keep(Path('/tmp/openui-'+name+'.py'), 'probes/openui-'+name+'.py', owner['probe_sha256'])
assert owners[0]['source'] == owners[0]['source_after'] == owners[1]['source'] == owners[1]['source_after']
assert owners[0]['source']['clean'] and owners[0]['source']['commit'] == '7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
audit = keep(RAW/'native-keywords-capture-matrix-audit-v1795.json', 'complete-matrix-audit.json',
             '76e5208a8d826747bea4c4c583fe25b10007557f33d848d02bea0389e937fd7f')
assert all(s['changed_comparisons'] == 0 and s['exact_losses'] == 0 and s['exact_gains'] == 0
           for s in audit['suites'].values())
for suite, record in audit['suites'].items():
    keep(RAW/f'native-keywords-capture-clean-{suite}-v1785/{suite}-summary.json',
         suite+'-summary.json.gz', record['summary_sha256'], True)
    keep(RAW/f'native-keywords-capture-{suite}-exit-v1785.json', suite+'-actual-exit.json',
         record['actual_exit_receipt_sha256'])
    keep(RAW/f'native-keywords-capture-clean-{suite}-v1785.log', suite+'.log.gz', compress=True)
keep(Path('/tmp/openui-native-keywords-capture-matrix-audit-v1795.py'),
     'probes/matrix-audit.py', audit['probe_sha256'])
for name in ['native-keywords-capture-consumer-v1785',
             'native-keywords-capture-table-geometry-v1785',
             'native-keywords-native-retry-consumer-v1787',
             'native-keywords-native-retry-table-geometry-v1787']:
    receipt = RAW/name/'receipt.json'
    record = keep(receipt, name+'/receipt.json')
    assert record['all_commands_terminal']
    for path in sorted(receipt.parent.rglob('*')):
        if path.is_file() and path != receipt and path.suffix in ('.png', '.html', '.log'):
            keep(path, name+'/'+str(path.relative_to(receipt.parent)), compress=path.suffix=='.log')
consumer = json.loads((RAW/'native-keywords-native-retry-consumer-v1787/receipt.json').read_bytes())
table = json.loads((RAW/'native-keywords-native-retry-table-geometry-v1787/receipt.json').read_bytes())
assert consumer['observed_exit_code'] == 0 and consumer['totals']['pixel_exact'] == 10
assert consumer['totals']['geometry_exact'] == 10 and not consumer['unstable_reference_captures']
assert consumer['source'] == consumer['source_after'] == owners[0]['source']
assert table['observed_exit_code'] == 0 and table['independent_native_runs_identical']
assert table['geometry_equality_with_chromium_not_claimed'] and table['observations'] == 120
keep(RAW/'native-keywords-native-retry-prepared-v1787.json', 'native-retry-preparation.json')
keep(Path('/tmp/openui-native-keywords-native-retry-prepare-v1787.py'), 'probes/native-retry-preparation.py')
keep(Path(__file__), 'preserver.py')
report = dict(schema_version=1, source=owners[0]['source'], source_after=owners[0]['source_after'],
    all_commands_terminal=True, all_four_renderer_matrices_complete=True,
    completed_owner_stage_exits=[[0,1,1,0,0,1,1],[0,0,0]],
    public_native_rust_callback_images_exact=10, public_native_rust_bounds_exact=10,
    independent_native_runs=2, independent_chromium_processes=20,
    consecutive_chromium_captures=40, all_reference_capture_pairs_stable=True,
    c_table_native_observations=120, c_table_repeats_identical=True,
    c_table_geometry_equality_with_chromium_not_claimed=True,
    suites=audit['suites'], unchanged_nine_invariant_rows=48252,
    original_rows_identical_in_expanded=22924, additions_unchanged=804,
    addition_cases_four_profile_exact=200, original_residual_ids=882, expanded_residual_ids=883,
    full_pixel_gates_still_fail=True, reviewed_wpt_residual_ownership_unchanged=True,
    original_startup_and_fractional_viewport_failures_preserved=True,
    release_qualification=False, candidate_applied_to_umbrella=False,
    javascript_executed_by_openui=False, public_native_rust_apis_required=True,
    old_openui_pixels_are_provenance_only=True, pixel_tolerance=0,
    new_release_states_admitted=0, artifacts=rows, artifact_count=len(rows),
    artifact_bytes=sum(row['bytes'] for row in rows))
INDEX.write_text(json.dumps(report, sort_keys=True, indent=2)+'\n')
print(json.dumps({'index':str(INDEX.relative_to(ROOT)), 'sha256':sha(INDEX),
                  'artifacts':len(rows), 'native_pixel_exact':10, 'changed_census_rows':0}), flush=True)
