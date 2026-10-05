"""Retain terminal width stages while the complete pixel owner is still running."""
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = ROOT / 'docs/renderer/evidence/native-intrinsic-snap-v5'
INDEX = ROOT / 'docs/renderer/generated/native-intrinsic-snap-v5.json'
assert not OUT.exists() and not INDEX.exists()
OUT.mkdir()
sha = lambda b: hashlib.sha256(b).hexdigest()
artifacts = []
def keep(p, name, expected=None, compress=False):
    data = Path(p).read_bytes()
    if expected:
        assert sha(data) == expected
    q = OUT / name
    assert not q.exists()
    q.write_bytes(gzip.compress(data, mtime=0) if compress else data)
    row = dict(path=str(q.relative_to(ROOT)), sha256=sha(q.read_bytes()), bytes=q.stat().st_size,
               source_path=str(p))
    if compress:
        row.update(encoding='gzip', decompressed_sha256=sha(data), decompressed_bytes=len(data))
        assert gzip.decompress(q.read_bytes()) == data
    artifacts.append(row)
    return json.loads(data) if str(p).endswith('.json') else None

p = RAW / 'native-intrinsic-snap-pipeline-v1717/receipt.json'
data = p.read_bytes()
observed = json.loads(data)
assert not observed['all_commands_terminal'] and observed['state'] in ['running-full', 'running-expanded']
assert observed['source'] == observed['source_after'] and observed['source']['clean']
assert observed['source']['commit'] == '727da10e580c9439f5db83d36bfc3e2340168207'
snapshot = RAW / 'native-intrinsic-snap-owner-at-observation-v1748.json'
assert not snapshot.exists()
snapshot.write_bytes(data)
keep(snapshot, 'whole-owner-at-observation.json', sha(data))
build_path = RAW / 'native-intrinsic-snap-clean-v1716/build.json'
build = keep(build_path, 'build-terminal.json')
assert build['all_commands_terminal'] and build['source'] == build['source_after'] == observed['source']
assert len(build['steps']) == 13 and all(s['observed_exit_code'] == 0 for s in build['steps'])
workspace = next(s for s in build['steps'] if s['name'] == 'workspace')
assert (workspace['passed'], workspace['failed'], workspace['ignored']) == (8552, 0, 13)
for step in build['steps']:
    keep(build_path.parent / (step['name'] + '.log'), 'build-' + step['name'] + '.log.gz', step['log_sha256'], True)
app_path = RAW / 'native-intrinsic-snap-consumer-v1716/receipt.json'
app = keep(app_path, 'native-app-terminal.json.gz', '39c7811e612731ed6d04fd392d36354a55aa3c14e576afb8e8598d3707974456', True)
assert app['all_commands_terminal'] and app['source'] == app['source_after'] == observed['source']
assert app['observed_exit_code'] == 1 and app['totals']['geometry_exact'] == app['totals']['phase_states'] == 38400
assert app['totals']['images_exact'] == 0 and app['totals']['images'] == 600
actual = [r for c in app['cases'] for r in c['images'] if r['language'] in ['c', 'cpp']]
assert len(actual) == 400 and all(r['rust_pixels_equal'] for r in actual)
for suite, count in [('focused', 640), ('primitive', 960)]:
    d = keep(RAW / f'native-intrinsic-snap-clean-{suite}-v1716/{suite}-summary.json', suite + '-summary.json.gz', compress=True)
    assert d['source'] == d['source_after'] == observed['source']
    assert d['results']['total'] == d['results']['exact'] == count and d['results']['errors'] == 0
    exitpath = RAW / f'native-intrinsic-snap-{suite}-exit-v1716.json'
    exited = keep(exitpath, suite + '-actual-exit.json')
    assert exited['observed_exit_code'] == 0
    keep(RAW / f'native-intrinsic-snap-clean-{suite}-v1716.log', suite + '.log.gz', exited['log_sha256'], True)
for step in observed['steps'][:5]:
    keep(p.parent / (step['name'] + '.log'), 'owner-' + step['name'] + '.log.gz', step['log_sha256'], True)
keep(Path(__file__), 'preserver.py')
report = dict(schema_version=1, source=observed['source'], source_after=observed['source_after'],
              complete_whole_owner_still_running_at_observation=True, whole_owner_state_at_observation=observed['state'],
              all_13_build_stages_passed=True, workspace_passed=8552, workspace_failed=0, workspace_ignored=13,
              native_app_actual_exit=1, native_images_exact=0, native_images=600,
              native_geometry_exact=38400, native_geometry_total=38400,
              actual_c_cpp_images_matching_rust=400, rust_self_rows_excluded=200,
              focused_exact=640, focused_total=640, primitive_exact=960, primitive_total=960,
              original_and_expanded_final_results_still_required=True, candidate_applied_to_umbrella=False,
              formal_wpt_residual_ownership_unchanged=True, javascript_executed_by_openui=False,
              public_native_rust_apis_required=True, release_qualification=False, pixel_tolerance=0,
              new_release_states_admitted=0, artifacts=artifacts, artifact_count=len(artifacts),
              artifact_bytes=sum(r['bytes'] for r in artifacts))
INDEX.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)), sha256=sha(INDEX.read_bytes()), artifacts=len(artifacts))), flush=True)
