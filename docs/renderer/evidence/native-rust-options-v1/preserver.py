"""Preserve executed native Rust options guards, apps, and hosted checks."""
import gzip
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = ROOT / 'docs/renderer/evidence/native-rust-options-v1'
INDEX = ROOT / 'docs/renderer/generated/native-rust-options-v1.json'
assert not OUT.exists() and not INDEX.exists()
OUT.mkdir()
rows = []
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
def keep(path, name, expected=None, compress=False):
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if expected:
        assert digest == expected
    target = OUT/name
    target.parent.mkdir(parents=True, exist_ok=True)
    assert not target.exists()
    target.write_bytes(gzip.compress(data, mtime=0) if compress else data)
    row = dict(path=str(target.relative_to(ROOT)), sha256=sha(target), bytes=target.stat().st_size, source_path=str(path))
    if compress:
        assert gzip.decompress(target.read_bytes()) == data
        row.update(encoding='gzip', decompressed_sha256=digest, decompressed_bytes=len(data))
    rows.append(row)
    return json.loads(data) if path.suffix == '.json' else None

owner = keep(RAW/'native-rust-options-pipeline-v1801/receipt.json', 'owner.json',
    '1766ce501a12fbaf93794fb0614f0040190f648398d31e04169aecfeca5f7e49')
assert owner['all_commands_terminal'] and [s['observed_exit_code'] for s in owner['steps']] == [0,0]
for step in owner['steps']:
    keep(RAW/'native-rust-options-pipeline-v1801'/(step['name']+'.log'),
         'owner-'+step['name']+'.log.gz', step['log_sha256'], True)
build = keep(RAW/'native-rust-options-clean-v1800/build.json', 'build.json',
    '14f77769a5c625e314341f2e800d2a63853274ff68b3b8a8cfd5a30ae75b3850')
assert build['all_commands_terminal'] and len(build['steps']) == 4
assert all(s['observed_exit_code'] == 0 and not s['disk_guard_triggered'] for s in build['steps'])
assert build['source'] == build['source_after'] == owner['source'] == owner['source_after']
assert build['source']['clean'] and build['source']['commit'] == '27a47ba25185e20e7ae20801d0a45a386ce9e78c'
workspace = next(s for s in build['steps'] if s['name']=='workspace')
assert (workspace['passed'],workspace['failed'],workspace['ignored']) == (8555,0,13)
guard_names = ['native_raster_selection_survives_callbacks_cloning_and_resize',
               'default_native_constructors_preserve_rendering_and_configuration',
               'window_and_headless_apps_keep_explicit_native_raster_selection',
               'selecting_ganesh_does_not_fall_back_to_cpu_raster']
log = (RAW/'native-rust-options-clean-v1800/workspace.log').read_text()
for name in guard_names:
    assert re.search(r'^test '+re.escape(name)+r' \.\.\. ok$',log,re.M)
for step in build['steps']:
    keep(RAW/'native-rust-options-clean-v1800'/(step['name']+'.log'),
         'build-'+step['name']+'.log.gz', step['log_sha256'], True)
consumer_path = RAW/'native-rust-options-consumer-v1800/receipt.json'
consumer = keep(consumer_path, 'consumer/receipt.json',
    '86bedaf66ba41570e015f581f7584a7fd261cd58e3c6661d3e22c3fe6dfa8ced')
assert consumer['all_commands_terminal'] and consumer['observed_exit_code'] == 0
assert consumer['source'] == consumer['source_after'] == build['source']
assert consumer['totals']['pixel_exact'] == consumer['totals']['geometry_exact'] == 10
assert not consumer['unstable_reference_captures']
for path in sorted(consumer_path.parent.rglob('*')):
    if path.is_file() and path != consumer_path and path.suffix in ('.png','.html','.log'):
        name = 'consumer/'+str(path.relative_to(consumer_path.parent))
        keep(path, name+('.gz' if path.suffix=='.log' else ''), compress=path.suffix=='.log')
checks_path = RAW/'native-rust-options-checks-v1798/receipt.json'
checks = keep(checks_path, 'checks.json')
assert checks['all_commands_terminal'] and checks['source'] == checks['source_after'] == build['source']
assert len(checks['checks']) == 15 and all(c['observed_exit_code']==0 for c in checks['checks'])
for check in checks['checks']:
    keep(checks_path.parent/(check['name']+'.log'), 'check-'+check['name']+'.log.gz', check['log_sha256'], True)
hosted_path = RAW/'native-rust-options-hosted-v1807/receipt.json'
hosted = keep(hosted_path, 'hosted.json', '58557b1d0cd16cede4af11b477131ae026b8214172004cfbae45cf430cc2c484')
assert hosted['all_commands_terminal'] and hosted['source'] == build['source']['commit']
assert (hosted['successful_jobs'],hosted['skipped_jobs'],hosted['failed_jobs']) == (7,0,0)
for workflow in hosted['workflows']:
    keep(Path(workflow['path']), 'hosted-'+str(workflow['run_id'])+'.json', workflow['sha256'])
for job in hosted['job_logs']:
    keep(Path(job['path']), 'hosted-'+str(job['job_id'])+'.log.gz',job['sha256'],True)
for path,digest in owner['immutable_probe_hashes'].items():
    keep(Path(path), 'probes/'+Path(path).name,digest)
keep(Path('/tmp/openui-native-rust-options-pipeline-v1801.py'),'probes/owner.py',owner['probe_sha256'])
keep(Path('/tmp/openui-native-rust-options-checks-v1798.py'),'probes/checks.py',checks['probe_sha256'])
keep(Path('/tmp/openui-native-rust-options-hosted-collect-v1807.py'),'probes/hosted.py',hosted['probe_sha256'])
keep(RAW/'native-rust-options-prepared-v1802.json','preparation.json')
patch = subprocess.check_output(['git','diff','c094d645a1e184d287a4698f07dde5c86fa51255',build['source']['commit']],cwd=ROOT)
target = OUT/'source.patch';assert not target.exists();target.write_bytes(patch)
rows.append(dict(path=str(target.relative_to(ROOT)),sha256=sha(target),bytes=len(patch)))
keep(Path(__file__),'preserver.py')
report = dict(schema_version=1,source=build['source'],source_after=build['source_after'],
    all_commands_terminal=True,all_15_read_only_checks_passed=True,
    public_native_rust_options_guard_names=guard_names,all_four_guards_executed_and_passed=True,
    workspace_passed=8555,workspace_failed=0,workspace_ignored=13,
    consuming_rust_app_callbacks_bounds_configurations_and_teardown_passed=True,
    native_pixel_exact=10,native_geometry_exact=10,independent_native_runs=2,
    independent_chromium_processes=20,consecutive_chromium_captures=40,
    successful_hosted_jobs=7,skipped_hosted_jobs=0,failed_hosted_jobs=0,
    default_constructor_behaviour_guard_passed=True,ganesh_selection_no_cpu_fallback_guard_passed=True,
    all_configuration_field_rendering_effects_unqualified=True,
    own_complete_renderer_matrices_unexecuted=True,release_qualification=False,
    javascript_executed_by_openui=False,public_native_rust_apis_required=True,
    pixel_tolerance=0,new_release_states_admitted=0,artifacts=rows,
    artifact_count=len(rows),artifact_bytes=sum(r['bytes'] for r in rows))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'index':str(INDEX.relative_to(ROOT)),'sha256':sha(INDEX),'artifacts':len(rows),'native_exact':10}),flush=True)
