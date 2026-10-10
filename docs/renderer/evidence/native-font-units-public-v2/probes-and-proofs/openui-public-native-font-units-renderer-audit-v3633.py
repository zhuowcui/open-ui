import fcntl, hashlib, json, subprocess, sys
from pathlib import Path
ROOT = Path('/home/nero/code/open-ui')
MAIN = Path('/home/nero/code/open-ui')
OUT = Path('/mnt/d/openui-v02-qualification-d174ea0b/public-native-font-units-renderer-v3632')
PROOF = Path('/tmp/openui-public-native-font-units-renderer-completed-audit-v3633.json')
sys.path.insert(0, str(ROOT / 'tools/qualification'))
sys.path.insert(0, '/tmp')
from openui_parallel_source_identity_v3060 import identity as repository_source_identity
from residuals import canonical_sha256
def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(2**20), b''):
            h.update(block)
    return h.hexdigest()
load = lambda p: json.loads(Path(p).read_bytes())
lock = open('/tmp/openui-native-cargo-raster-owner.lock', 'a+')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
r = load(OUT / 'receipt.json')
assert r['all_commands_terminal'] and r['source_unchanged']
assert subprocess.run(['ps', '-p', str(r['owner_pid'])], capture_output=True).returncode == 1
for name in ['cargo', 'rustc', 'rustfmt', 'pixel_compare']:
    assert subprocess.run(['pgrep', '-x', name], capture_output=True).returncode == 1
assert repository_source_identity(ROOT) == r['source'] == r['source_after']
assert sha('/tmp/openui-public-native-font-units-renderer-v3632.py') == r['driver_sha256']
assert sha('/tmp/openui-native-font-units-native-reaped-v3630.json') == r['native_completed_audit_sha256']
assert sha('/mnt/d/openui-v02-qualification-d174ea0b/native-font-units-chromium-v3628/receipt.json') == r['native_diagnostic_receipt_sha256']
assert sha('/tmp/openui_parallel_source_identity_v3060.py') == r['source_identity_reader_sha256']
assert repository_source_identity(MAIN) == r['public_source'] == r['public_source_after']
assert r['public_source_unchanged']
for step in r['steps']:
    assert sha(OUT / (step['name'] + '.log')) == step['log_sha256']
bpath = OUT / 'guarded-build/receipt.json'; b = load(bpath)
assert b['all_commands_terminal'] and b['observed_exit_code'] == 0 and b['renderer_library_provenance_verified']
assert b['source'] == b['source_after'] == r['source']
assert sha(bpath) == r['guarded_build_receipt_sha256']
assert sha(b['binary']) == b['binary_sha256']
assert json.loads(subprocess.check_output([b['binary'], 'build-source-identity']))['source'] == r['source']
for path, digest in b['compiled_artifact_sha256'].items():
    assert sha(path) == digest
for step in b['steps']:
    assert step['observed_exit_code'] == 0
    for channel in ['stdout', 'stderr']:
        assert sha(bpath.parent / (step['name'] + '.' + channel)) == step[channel + '_sha256']
records = 0
for package, rows in b['artifacts'].items():
    for record in rows:
        records += 1
        assert record['fresh'] is False and record['reason'] == 'compiler-artifact'
        assert Path(record['manifest_path']).resolve().is_relative_to(ROOT)
        assert Path(record['target']['src_path']).resolve().is_relative_to(ROOT)
results = {}; audited = 0
for suite in ['focused', 'primitive', 'full', 'expanded']:
    path = OUT / (suite + '-matrix') / (suite + '-summary.json')
    m = load(path); c = r['comparisons'][suite]; prev = load(c['previous_summary'])
    assert sha(path) == r['matrices'][suite]['summary_sha256']
    assert sha(c['previous_summary']) == c['previous_summary_sha256']
    assert m['source'] == m['source_after'] == r['source'] and m['complete_contract_scope']
    old = {(p['profile'], t['id']): t for p in prev['profiles'] for t in p['tests']}
    exact = different = errors = gains = losses = worsened = changes = count = 0
    for p in m['profiles']:
        assert p['result_sha256'] == canonical_sha256(p['tests'])
        for t in p['tests']:
            prior = old.pop((p['profile'], t['id'])); count += 1
            assert all(key in prior and key in t for key in c['fields'])
            assert all(prior[key] == t[key] for key in c['fields'] if key.startswith('chromium_'))
            exact += t['status'] == 'exact'; different += t['status'] == 'different'; errors += t['status'] == 'error'
            gains += prior['status'] != 'exact' and t['status'] == 'exact'
            losses += prior['status'] == 'exact' and t['status'] != 'exact'
            worsened += t['mismatched_pixels'] > prior['mismatched_pixels']
            changes += any(prior[key] != t[key] for key in c['fields'])
    assert not old and count == dict(focused=640, primitive=960, full=22924, expanded=23728)[suite]
    assert exact == m['results']['exact'] and different == m['results']['different'] and errors == m['results']['errors'] == 0
    assert gains == c['exact_gains'] and losses == c['exact_losses'] and worsened == c['worsened']
    assert changes == len(c['changes']) and count == c['audited_rows']
    assert r['matrices'][suite]['results'] == m['results']
    step = next(s for s in r['steps'] if s['name'] == suite)
    assert step['observed_exit_code'] == r['matrices'][suite]['observed_exit_code'] == int(different != 0)
    results[suite] = dict(total=count, exact=exact, different=different, errors=errors,
                          gains=gains, losses=losses, worsened=worsened, changed_rows=changes,
                          chromium_reference_changed_rows=0, actual_exit_code=step['observed_exit_code'])
    audited += count
nonregression = all(row['losses'] == 0 and row['worsened'] == 0 for row in results.values())
assert r['candidate_nonregression_verified'] == nonregression
assert r['observed_exit_code'] == int(any(row['actual_exit_code'] for row in results.values()) or not nonregression)
assert 'failure' not in r
proof = dict(schema_version=1, complete=True, owner_terminal=True, owner_absent=True,
             actual_whole_exit_code=r['observed_exit_code'], source=r['source'],
             receipt_sha256=sha(OUT / 'receipt.json'), build_receipt_sha256=sha(bpath),
             linked_local_libraries_fresh=True, compiler_artifact_records=records,
             compiled_paths_verified=len(b['compiled_artifact_sha256']),
             native_diagnostic_proof_sha256=r['native_completed_audit_sha256'], public_source=r['public_source'],
             matrix_rows_verified=audited, results=results, candidate_nonregression=nonregression,
             actual_combined_umbrella_source_measured=True, implementation_integrated=True, private_candidate_source_measured=False,
             pixel_target='pinned Chromium', pixel_tolerance=0,
             historical_openui_archive_is_a_pixel_target=False, javascript_executed_by_openui=False,
             renderer_qualified=False, all_native_apis_qualified=False, release_qualified=False,
             retired_for_reexecution_after_source_or_target_mutation=True, audit_driver_sha256=sha(__file__))
assert not PROOF.exists()
PROOF.write_text(json.dumps(proof, sort_keys=True, indent=2) + '\n')
print(json.dumps(proof), flush=True)
