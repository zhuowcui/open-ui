"""Prepare a new immutable queue; do not execute Cargo or capture images."""
import ast
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-image-coverage-d913041d')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
FIXED = '2eacae2c8aad222850af30f9cea6f8a79bbddd4a'
BASELINE = '6570b65258677fff7369b336db6e55bb7645daec'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == FIXED
assert subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT) == b''
parent_text = Path('/tmp/openui-native-image-occlusion-pipeline-v1437.py').read_text()
parent_config = ast.literal_eval(ast.parse(parent_text).body[0].value)
prior = parent_config['prior_pipelines'] + ['native-image-occlusion-pipeline-v1437']
assert len(prior) == 17 and len(set(prior)) == 17

def translated(kind):
    text = Path(f'/tmp/openui-native-image-occlusion-{kind}-v1436.py').read_text()
    return (text.replace('/dev/shm/openui-native-image-occlusion-3827199a', str(ROOT))
        .replace('native-image-occlusion', 'native-image-coverage')
        .replace('v1436', 'v1448')
        .replace('d913041dba9b36987ce83a1b3ed85dc6b4bf93e4', FIXED))

scripts = {}
for kind in ['guards', 'build', 'fieldsets', 'consumer', 'matrices']:
    content = translated(kind)
    if kind == 'guards':
        content = content.replace('954c22a4e32edb71a356050b02fdf27c451863af', BASELINE)
        content = content.replace('agent/native-image-occlusion-v1432', 'agent/native-image-coverage-v1446')
        content = content.replace('agent/native-image-coverage-v1432', 'agent/native-image-coverage-v1446')
        content = content.replace('tests::opaque_image_foreground_obscures_only_a_fully_covered_native_background',
            'tests::native_image_edge_coverage_blends_after_sampling_without_white_halo')
        content = content.replace('fixed-image-foreground-guard', 'fixed-image-edge-guard')
        old = "for pipeline in ['scene-recording-cache-pipeline-v1233', 'native-fieldset-pipeline-v1265', 'native-font-backends-pipeline-v1281']:"
        assert old in content
        content = content.replace(old, f'for pipeline in {prior!r}:')
        old = "and b'0 passed; 1 failed;' in content)"
        assert old in content
        content = content.replace(old,
            "and b'0 passed; 1 failed;' in content\n"
            "        and b'image coverage must blend once over its red backdrop' in content)")
        old = "'baseline-native-inheritance-guard'"
        assert old in content
        content = content.replace(old, "'baseline-image-edge-guard'")
        content = content.replace("'owned-recording-guard': ['tests::compositor_distinguishes_owned_recordings_with_equal_document_generations']}",
            "'owned-recording-guard': ['tests::compositor_distinguishes_owned_recordings_with_equal_document_generations'], "
            "'image-background-culling-guard': ['tests::opaque_image_foreground_obscures_only_a_fully_covered_native_background']}")
        old = "steps = [\n"
        assert content.count(old) == 1
        content = content.replace(old, old +
            "    ('image-background-culling-guard', base + ['-p', 'openui-engine', '--lib',\n"
            "        'tests::opaque_image_foreground_obscures_only_a_fully_covered_native_background', '--', '--exact']),\n")
        content = content.replace("expected_counts = {'fixed-image-edge-guard': 1,", "expected_counts = {'image-background-culling-guard': 1, 'fixed-image-edge-guard': 1,")
    if kind == 'consumer':
        old = "variants = ['opaque', 'partial', 'opacity', 'padding', 'contain', 'offset', 'blur', 'shadow']"
        assert old in content
        content = content.replace(old, old[:-1] + ", 'white', 'white-opacity', 'transparent', 'clip']")
        old = "                case = f'{display}-{variant}-{phase}'\n"
        assert old in content
        content = content.replace(old, old +
            "                asset = assets['white'] if variant in ('white', 'white-opacity') else assets['transparent'] if variant == 'transparent' else assets['green']\n")
        old = "                    image_width = 100 if variant == 'partial' else 150\n"
        assert old in content
        content = content.replace(old,
            "                    effects['white-opacity'] = 'opacity:.5;'\n"
            "                    effects['clip'] = 'object-fit:none;overflow-x:hidden;overflow-y:hidden;'\n" + old)
        old = "assert sha(asset) == 'd49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe'\n"
        assert old in content
        content = content.replace(old, old +
            "assets = dict(green=asset, white=asset.parent / '1x1-white.png', transparent=asset.parent / 'green-transparent-200x200.png')\n"
            "assert sha(assets['white']) == 'b31782b0ecaa71394f1bccf3cc4647ba70b7208464244546b48521a71e1f1dd0'\n"
            "assert sha(assets['transparent']) == '77e8da29ee253660e7a650f43241d96e636b8e2cec5547cb99fd160e95419422'\n")
        content = content.replace("image_sha256=sha(asset),", "image_sha256=sha(asset), resource_sha256={key: sha(path) for key, path in assets.items()},")
        content = content.replace('cases=240, images=480,', 'cases=360, images=720,')
        content = content.replace('geometry_exact=480, pixel_exact=480, native_runs_passed=480, deterministic_native_repeats=240, callback_contracts=240',
            'geometry_exact=720, pixel_exact=720, native_runs_passed=720, deterministic_native_repeats=360, callback_contracts=360')
        # Preserve the old 480 image inputs byte-for-byte if the preceding
        # candidate got far enough to write them. Its failure never waives
        # this candidate's own complete public application gate.
        old = "                report['inputs'][case] = inputs\n"
        assert old in content
        content = content.replace(old,
            "                if variant in variants[:8]:\n"
            "                    for state, item in inputs.items():\n"
            "                        previous = RAW / 'native-image-occlusion-consumer-v1436' / f'{case}-{state}.html'\n"
            "                        if previous.exists():\n"
            "                            assert previous.read_bytes() == Path(item['path']).read_bytes()\n" + old)
    ast.parse(content)
    path = Path(f'/tmp/openui-native-image-coverage-{kind}-v1448.py')
    assert not path.exists()
    path.write_text(content)
    scripts[kind] = dict(path=str(path), sha256=sha(path))

config = dict(parent_config, root=str(ROOT), commit=FIXED,
    name='native-image-coverage-pipeline-v1449',
    initial_state='awaiting-every-prior-whole-pipeline-including-image-background-culling',
    prior_pipelines=prior,
    scripts=[Path(scripts[kind]['path']).name for kind in scripts],
    stages=[(name, [argument.replace('native-image-occlusion', 'native-image-coverage').replace('v1436', 'v1448') for argument in command],
        terminal.replace('native-image-occlusion', 'native-image-coverage').replace('v1436', 'v1448'), flag)
        for name, command, terminal, flag in parent_config['stages']])
body = parent_text.split('\n', 1)[1]
body = body.replace("baseline_commit='954c22a4e32edb71a356050b02fdf27c451863af'", f'baseline_commit={BASELINE!r}')
body = body.replace("reviewed_root_cause_owner='openui-paint foreground opacity and background culling'",
    "reviewed_root_cause_owner='openui-paint image sampling and geometric coverage blending'")
body = body.replace('color_coverage_rounding_root_cause_still_open=True',
    'coverage_blending_candidate_requires_native_confirmation=True, strict_baseline_assertion_marker_required=True')
owner = Path('/tmp/openui-native-image-coverage-pipeline-v1449.py')
assert not owner.exists()
owner.write_text(f'CONFIG = {config!r}\n' + body)
ast.parse(owner.read_text())
for commit in (BASELINE, FIXED):
    for asset, digest in [('green-200.png', 'd49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe'),
        ('1x1-white.png', 'b31782b0ecaa71394f1bccf3cc4647ba70b7208464244546b48521a71e1f1dd0')]:
        data = subprocess.check_output(['git', 'show', f'{commit}:bindings/rust/openui/tests/assets/{asset}'], cwd=ROOT)
        assert hashlib.sha256(data).hexdigest() == digest
report = dict(schema_version=1, fixed_commit=FIXED, baseline_commit=BASELINE, scripts=scripts,
    owner_probe=str(owner), owner_probe_sha256=sha(owner), whole_prior_pipelines=prior,
    source_root=str(ROOT), cargo_commands_run=0, screenshots_generated=0,
    strict_chromium_capture_pairs=True, public_native_rust_callbacks=True,
    public_consumer_expected_images=720, fieldset_expected_images=128,
    all_four_renderer_matrices_required=True, pixel_tolerance=0,
    release_qualification=False, promotion_allowed=False, javascript_executed_by_openui=False)
out = RAW / 'native-image-coverage-prepared-v1448.json'
assert not out.exists()
out.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps({'prepared':str(out),'sha256':sha(out),'fixed_commit':FIXED,'prior_owners':len(prior),'local_build_or_image_work_run':False}))
