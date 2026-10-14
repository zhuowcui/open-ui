"""Consuming native Rust controls: options, callbacks, snapshots and lifetime."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

from PIL import Image

ROOT = Path('/dev/shm/openui-native-raster-fields-retry-e0dc491e')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-raster-fields-retry-controls-v1559'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
build_path = RAW / 'native-raster-fields-retry-clean-v1559/build.json'
build = json.loads(build_path.read_bytes())
assert source['clean'] and source == build['source'] == build['source_after']
assert build['all_commands_terminal'] and len(build['steps']) == 17
assert all(s['observed_exit_code'] == 0 for s in build['steps'])
binary = build_path.parent / 'native_control_raster'
assert sha(binary) == next(s['binary_sha256'] for s in build['steps'] if s['name'] == 'control-build')
report = dict(schema_version=1, source=source, source_after=source,
    probe_sha256=sha(Path(__file__)), native_binary_sha256=sha(binary),
    build_receipt_sha256=sha(build_path), public_native_rust_api=True,
    javascript_executed_by_openui=False, chromium_pixel_qualification=False,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    control_count_per_state=64, logical_origin_phases_64ths=list(range(64)),
    all_commands_terminal=False, cases=[])
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
try:
    for policy in ['default', 'freetype', 'fontations']:
        for hint in ['none', 'slight', 'normal', 'full']:
            for scale in [1.0, 1.25, 1.5, 2.0, 3.0]:
                directory = OUT / policy / hint / str(scale)
                directory.mkdir(parents=True)
                row = dict(policy=policy, native_hinting=hint, scale=scale, runs=[])
                report['cases'].append(row)
                geometries = []
                for repeat in [1, 2]:
                    command = [str(binary), str(directory / ('native-' + str(repeat))), str(scale), policy, hint]
                    result = subprocess.run(command, cwd=ROOT, capture_output=True)
                    log = directory / ('native-' + str(repeat) + '.log')
                    log.write_bytes(result.stdout + result.stderr)
                    row['runs'].append(dict(command=command, observed_exit_code=result.returncode, log_sha256=sha(log)))
                    if result.returncode == 0:
                        geometries.append(json.loads((directory / ('native-' + str(repeat) + '/geometry.json')).read_bytes()))
                row['native_application_success'] = all(s['observed_exit_code'] == 0 for s in row['runs'])
                if row['native_application_success']:
                    assert len(geometries) == 2
                    row['callback_and_owned_bounds_verified'] = all(
                        g['callback_count'] == 1 and g['before'] == g['after']
                        and len(g['before']) == 64 for g in geometries)
                    row['repeatability_verified'] = geometries[0] == geometries[1]
                    row['images'] = []
                    for state in ['before', 'after']:
                        first = directory / ('native-1/' + state + '.png')
                        second = directory / ('native-2/' + state + '.png')
                        identical = first.read_bytes() == second.read_bytes()
                        with Image.open(first) as image:
                            assert image.size == (round(800 * scale), round(600 * scale))
                        row['repeatability_verified'] &= identical
                        row['images'].append(dict(state=state, png_sha256=sha(first), independent_repeat_png_sha256=sha(second)))
                    row['opacity_mutation_changes_frame'] = row['images'][0]['png_sha256'] != row['images'][1]['png_sha256']
                    row['contract_checks_passed'] = all(row[k] for k in [
                        'callback_and_owned_bounds_verified', 'repeatability_verified', 'opacity_mutation_changes_frame'])
                else:
                    row['contract_checks_passed'] = False
                save()
                print(json.dumps({k:v for k,v in row.items() if k not in ['runs','images']}), flush=True)
    report['totals'] = dict(cases=len(report['cases']),
        contract_checks_passed=sum(c['contract_checks_passed'] for c in report['cases']),
        images=sum(len(c.get('images', [])) for c in report['cases']),
        native_control_states=sum(128 for c in report['cases'] if c['native_application_success']))
    report['observed_exit_code'] = int(report['totals'] != dict(
        cases=60, contract_checks_passed=60, images=120, native_control_states=7680))
except BaseException as error:
    report.update(observed_exit_code=1, failure=str(error))
    raise
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source and sha(binary) == report['native_binary_sha256']
    save()
raise SystemExit(report['observed_exit_code'])
