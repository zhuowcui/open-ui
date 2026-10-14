"""Public native app raster-field behavior; these checks do not admit pixel cases.

Chromium qualification is owned by the separate consumer and complete matrices.
Translation below verifies the documented physical-origin API, not an oracle.
No reference PNG is rewritten or generated from native output.
"""
import functools
import hashlib
import json
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageChops

ROOT = Path('/dev/shm/openui-native-raster-api-3d4eea11')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-raster-consumer-application-v1424'
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
build_path = RAW / 'native-raster-consumer-clean-v1424/build.json'
build = json.loads(build_path.read_bytes())
assert source['clean'] and source == build['source'] == build['source_after']
assert build['all_commands_terminal'] and len(build['steps']) == 17
assert all(s['observed_exit_code'] == 0 for s in build['steps'])
binary = build_path.parent / 'native_raster_fields'
assert sha(binary) == next(s['binary_sha256'] for s in build['steps'] if s['name'] == 'fields-build')
settings = [
    ('lcd', 'none', True, False),
    ('lcd', 'normal', True, False),
    ('lcd', 'normal', True, True),
    ('lcd', 'full', True, False),
    ('lcd', 'slight', False, False),
    ('aa', 'none', True, False),
    ('alias', 'full', False, False),
]
report = dict(schema_version=1, source=source, source_after=source,
    build_receipt_sha256=sha(build_path), binary_sha256=sha(binary),
    probe_sha256=sha(Path(__file__)), pixel_tolerance=0,
    native_api_contract_checks=True, chromium_pixel_qualification=False,
    javascript_executed_by_openui=False, release_qualification=False,
    promotion_allowed=False, new_release_states_admitted=0,
    scales=[1.0, 1.25, 1.5, 2.0, 3.0], logical_origin_phases_64ths=list(range(64)),
    tested_physical_phase_64ths=[-128, -64, -1, 0, 1, 24, 63, 64, 128],
    all_commands_terminal=False, cases=[], source_owner='openui-paint and openui-text raster policy flow')
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()

def native(directory, scale, policy, family, configuration, phase):
    edging, hinting, subpixel, autohint = configuration
    geometries = []
    runs = []
    for repeat in [1, 2]:
        output = directory / ('native-' + str(repeat))
        command = [str(binary), str(output), str(scale), policy, family, '17',
                   edging, hinting, str(subpixel).lower(), str(autohint).lower(), str(phase)]
        result = subprocess.run(command, cwd=ROOT, capture_output=True)
        log = directory / ('native-' + str(repeat) + '.log')
        log.write_bytes(result.stdout + result.stderr)
        runs.append(dict(command=command, observed_exit_code=result.returncode, log_sha256=sha(log)))
        if result.returncode:
            return dict(runs=runs, native_application_success=False)
        geometry = json.loads((output / 'geometry.json').read_bytes())
        assert geometry['callback_count'] == 1 and geometry['lcd_phase_64ths'] == phase
        assert geometry['subpixel_positioning'] == subpixel and geometry['force_autohint'] == autohint
        assert len(geometry['before']) == len(geometry['after']) == 64
        geometries.append(geometry)
    deterministic = geometries[0] == geometries[1] and all(
        (directory / ('native-1/' + state + '.png')).read_bytes()
        == (directory / ('native-2/' + state + '.png')).read_bytes() for state in ['before', 'after'])
    return dict(runs=runs, native_application_success=True, deterministic=deterministic,
                geometry=geometries[0], geometry_sha256=sha(directory / 'native-1/geometry.json'))

def phase_comparison(baseline, actual, phase, edging):
    with Image.open(baseline) as original, Image.open(actual) as shifted:
        original = original.convert('RGBA')
        shifted = shifted.convert('RGBA')
        assert original.size == shifted.size
        # Inspect pixels without writing any transformed or replacement image.
        width, height = original.size
        assert phase % 64 == 0 or edging != 'lcd'
        dx = phase // 64 if edging == 'lcd' else 0
        source_box = (max(0, -dx), 0, min(width, width - dx), height)
        target_box = (max(0, dx), 0, min(width, width + dx), height)
        delta = ImageChops.difference(original.crop(source_box), shifted.crop(target_box))
        mask = functools.reduce(ImageChops.lighter, delta.split())
        mismatches = sum(mask.histogram()[1:])
        if dx:
            edge_box = (0, 0, dx, height) if dx > 0 else (width + dx, 0, width, height)
            mismatches += sum(pixel != (255, 255, 255, 255)
                              for pixel in shifted.crop(edge_box).getdata())
        return dict(expected_physical_shift=dx, mismatched_pixels=mismatches,
                    baseline_png_sha256=sha(baseline), actual_png_sha256=sha(actual))

try:
    for policy in ['freetype', 'fontations']:
        for family in ['Ahem', 'DejaVu Sans', 'DejaVu Serif', 'DejaVu Sans Mono']:
            for scale in report['scales']:
                for configuration in settings:
                    edging, hinting, subpixel, autohint = configuration
                    key = '-'.join([edging, hinting, str(subpixel), str(autohint)])
                    root = OUT / policy / family.replace(' ', '-') / str(scale) / key
                    baseline = None
                    phases = ([0, -128, -64, -1, 1, 24, 63, 64, 128]
                              if configuration == settings[0] else [0, 64])
                    for phase in phases:
                        directory = root / str(phase)
                        directory.mkdir(parents=True)
                        row = dict(policy=policy, family=family, scale=scale,
                            edging=edging, hinting=hinting, subpixel=subpixel,
                            autohint=autohint, physical_phase_64ths=phase,
                            directory=str(directory), owner=report['source_owner'])
                        report['cases'].append(row)
                        row.update(native(directory, scale, policy, family, configuration, phase))
                        if not row['native_application_success']:
                            row['contract_exact'] = False
                            save()
                            continue
                        row['geometry_unchanged_by_phase'] = True
                        row['phase_contract_checks'] = []
                        if phase == 0:
                            baseline = row
                            with Image.open(directory / 'native-1/before.png') as image:
                                pixels = list(image.convert('RGB').getdata())
                            if edging == 'alias':
                                row['mask_contract_exact'] = all(p in [(0, 0, 0), (255, 255, 255)] for p in pixels)
                            elif edging == 'aa':
                                row['mask_contract_exact'] = all(r == g == b for r, g, b in pixels)
                            else:
                                row['mask_contract_exact'] = True
                        elif baseline is not None:
                            row['geometry_unchanged_by_phase'] = all(
                                row['geometry'][state] == baseline['geometry'][state]
                                for state in ['before', 'after'])
                            if phase % 64 == 0 or edging != 'lcd':
                                for state in ['before', 'after']:
                                    row['phase_contract_checks'].append(phase_comparison(
                                        Path(baseline['directory']) / ('native-1/' + state + '.png'),
                                        directory / ('native-1/' + state + '.png'), phase, edging))
                        else:
                            row['geometry_unchanged_by_phase'] = False
                        row['contract_exact'] = (row['deterministic']
                            and row['geometry_unchanged_by_phase']
                            and row.get('mask_contract_exact', True)
                            and all(c['mismatched_pixels'] == 0 for c in row['phase_contract_checks']))
                        save()
                    print(json.dumps(dict(policy=policy, family=family, scale=scale, settings=key)), flush=True)
    report['totals'] = dict(cases=len(report['cases']),
        contract_exact=sum(c['contract_exact'] for c in report['cases']),
        integer_phase_image_checks=sum(len(c.get('phase_contract_checks', [])) for c in report['cases']),
        geometry_and_callback_states=sum(128 for c in report['cases'] if c['native_application_success']))
    report['observed_exit_code'] = int(any(not c['contract_exact'] for c in report['cases']))
except BaseException as error:
    report.update(observed_exit_code=1, failure=str(error))
    raise
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source and sha(binary) == report['binary_sha256']
    save()
raise SystemExit(report['observed_exit_code'])
