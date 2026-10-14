"""Qualify native Rust mutations against repeated immutable Chromium captures."""
import ast, base64, hashlib, importlib.util, io, json, os, shutil, signal, subprocess, sys, tempfile
from pathlib import Path
from PIL import Image

ROOT = Path('/dev/shm/openui-native-inline-fallback-retry-7d09f7a1')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-inline-fallback-resource-consumer-v1575'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
STORE.mkdir(); OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
from residuals import analyze_image_difference
sha = lambda p: hashlib.file_digest(p.open('rb'), 'sha256').hexdigest()
build_path = RAW / 'native-inline-fallback-clean-v1575/build.json'
build = json.loads(build_path.read_bytes())
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == '7d09f7a159b2c3f211662d20dbf7c7092d6024ed'
assert source == build['source'] == build['source_after']
assert build['all_commands_terminal'] and len(build['steps']) == 9
assert all(r['observed_exit_code'] == 0 for r in build['steps'])
binary = build_path.parent / 'native_inline_replaced'
assert sha(binary) == next(r['binary_sha256'] for r in build['steps'] if r['name'] == 'inline-build')
asset = ROOT / 'bindings/rust/openui/tests/assets/green-200.png'
assert sha(asset) == 'd49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe'
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec); spec.loader.exec_module(capture)
chrome = Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome')
env = capture.chrome_environment(str(chrome.parent), True, False)
report = dict(schema_version=1, source=source, source_after=source,
    all_commands_terminal=False, release_qualification=False, promotion_allowed=False,
    new_release_states_admitted=0, public_native_rust_api=True, javascript_executed_by_openui=False,
    pixel_tolerance=0, native_binary_sha256=sha(binary), chromium_binary_sha256=sha(chrome),
    capture_harness_sha256=sha(harness), fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),
    image_sha256=sha(asset), probe_sha256=sha(Path(__file__)), build_receipt_sha256=sha(build_path),
    unstable_reference_captures=[], inputs={}, native_runs=[], cases=[])
save = lambda: (OUT / 'receipt.json').write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
reference_source = Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py')
reference_text = reference_source.read_text()
node = next(n for n in ast.parse(reference_text).body if isinstance(n, ast.FunctionDef) and n.name == 'reference')
original = ast.get_source_segment(reference_text, node)
assert "assert query['natural'] == dict(width=200, height=200)" in original
exec(compile(original, str(reference_source), 'exec'))
report.update(reference_source_sha256=sha(reference_source),
    unchanged_reference_function_sha256=hashlib.sha256(original.encode()).hexdigest(),
    capture_conditions_unchanged=True, strict_chromium_capture_pairs=True,
    preserves_both_unstable_reference_captures=True)
expression = '''(async()=>{
if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));
await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
const image=document.getElementById('image'),r=image.getBoundingClientRect();
return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},
 bounds:{x:r.x,y:r.y,width:r.width,height:r.height},
 natural:{width:image.naturalWidth,height:image.naturalHeight}};
})()'''
for phase in [0.0, 0.25, 0.5]:
    for state in ['before', 'after', 'hidden', 'reattached']:
        width = 150 if state == 'before' else 100
        display = 'none' if state == 'hidden' else 'inline'
        path = OUT / f'phase-{phase:g}-{state}.html'
        path.write_text('<!doctype html><meta charset="utf-8"><style>'
            '*{margin:0;padding:0;border:0;box-sizing:content-box}html,body{width:320px;height:240px;background:white;overflow:hidden}'
            f'#parent{{display:block;position:absolute;left:{20+phase}px;top:{20+phase}px;width:150px;height:150px}}'
            f'#image{{display:{display};width:{width}px;height:150px}}'
            '</style><div id="parent"><img id="image" src="data:image/png;base64,' +
            base64.b64encode(asset.read_bytes()).decode() + '"></div>\n')
        report['inputs'][f'{phase:g}-{state}'] = dict(path=str(path), sha256=sha(path))
save()
try:
    for repeat in [1, 2]:
        command = [str(binary), str(OUT / ('native-' + str(repeat)))]
        result = subprocess.run(command, cwd=ROOT, capture_output=True)
        log = OUT / f'native-{repeat}.log'; log.write_bytes(result.stdout + result.stderr)
        report['native_runs'].append(dict(command=command, observed_exit_code=result.returncode, log_sha256=sha(log)))
        save()
        assert result.returncode == 0, 'native Rust callbacks, geometry or teardown failed'
        assert result.stdout.count(b'callback=1 owned-bounds/detach/teardown passed') == 15
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0]:
        for phase in [0.0, 0.25, 0.5]:
            directory = OUT / f'scale-{scale:g}' / f'phase-{phase:g}'
            directory.mkdir(parents=True)
            for state in ['before', 'after', 'hidden', 'reattached']:
                native_relative = f'scale-{scale:g}/phase-{phase:g}/{state}.png'
                actual = OUT / 'native-1' / native_relative
                assert actual.read_bytes() == (OUT / 'native-2' / native_relative).read_bytes()
                refs, observations = [], []
                for repeat in [1, 2]:
                    path = directory / f'{state}-chromium-{repeat}.png'
                    observations.append(reference(Path(report['inputs'][f'{phase:g}-{state}']['path']), path, 320, 240, scale))
                    refs.append(path)
                assert refs[0].read_bytes() == refs[1].read_bytes()
                assert observations[0]['query'] == observations[1]['query']
                expected = dict(x=0, y=0, width=0, height=0) if state == 'hidden' else dict(
                    x=20+phase, y=20+phase, width=150 if state == 'before' else 100, height=150)
                assert observations[0]['query']['bounds'] == expected
                analysis = analyze_image_difference(refs[0], actual)
                report['cases'].append(dict(scale=scale, phase=phase, state=state,
                    analysis=analysis, native_png_sha256=sha(actual), chromium_png_sha256=sha(refs[0]),
                    geometry_exact=True, native_geometry_verified_by_application_assertions=True,
                    independent_reference_runs=observations))
                save()
            last = report['cases'][-4:]
            print(json.dumps(dict(scale=scale, phase=phase, states=4,
                mismatched_pixels=[r['analysis']['mismatched_pixels'] for r in last])), flush=True)
    rows = report['cases']
    report['totals'] = dict(images=len(rows), pixel_exact=sum(r['analysis']['mismatched_pixels'] == 0 for r in rows),
        geometry_exact=sum(r['geometry_exact'] for r in rows), native_application_runs=2,
        deterministic_native_image_pairs=60, independent_chromium_capture_processes=120,
        consecutive_chromium_captures=240, native_callback_states=30)
    report['observed_exit_code'] = int(len(rows) != 60 or report['totals']['pixel_exact'] != 60)
except BaseException as error:
    report.update(observed_exit_code=1, failure=str(error)); raise
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert source == report['source_after'] and sha(binary) == report['native_binary_sha256']
    assert all(sha(Path(r['path'])) == r['sha256'] for r in report['inputs'].values())
    save()
raise SystemExit(report['observed_exit_code'])
