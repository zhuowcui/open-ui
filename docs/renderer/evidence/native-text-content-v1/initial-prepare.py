"""Freeze native text verification after every preceding whole pipeline."""
import ast
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-text-content-f25cd722')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
FIXED = 'd5bd17d77a2262f10b97e445d380723dff093d51'
BASELINE = '1c1b1bffb9cc5b50513e1b060ce25a9c7295ece1'
BRANCH = 'agent/native-text-content-v1613'
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == FIXED
checks_path = RAW / 'native-text-content-checks-v1616/receipt.json'
checks = json.loads(checks_path.read_bytes())
assert checks['source'] == checks['source_after'] == source and checks['all_commands_terminal']
assert len(checks['checks']) == 13 and all(x['observed_exit_code'] == 0 for x in checks['checks'])
prior_probe = Path('/tmp/openui-native-table-source-pipeline-v1601.py')
prior_text = prior_probe.read_text()
prior = ast.literal_eval(ast.parse(prior_text).body[0].value)
priors = prior['prior_pipelines'] + [prior['name']]
assert len(priors) == len(set(priors)) == 32
files = {}
originals = {str(prior_probe): sha(prior_probe)}
for kind in ('guards', 'build', 'matrices'):
    old = Path('/tmp/openui-native-table-source-' + kind + '-v1600.py')
    text = old.read_text()
    originals[str(old)] = sha(old)
    for before, after in [('/dev/shm/openui-native-table-source-1d846e68', str(ROOT)),
                          ('e389b26ab28f259544e43c3052c895501e4c8686', FIXED),
                          ('cd6d80be4be16151a607cddcbad94e2f73a79e62', BASELINE),
                          ('agent/native-table-source-v1597', BRANCH),
                          ('native-table-source-', 'native-text-content-'),
                          ('v1600', 'v1617'), (repr(prior['prior_pipelines']), repr(priors))]:
        text = text.replace(before, after)
    files[Path('/tmp/openui-native-text-content-' + kind + '-v1617.py')] = text

path = Path('/tmp/openui-native-text-content-guards-v1617.py')
text = files[path]
text = text.replace('tests::native_repeated_table_body_progress_survives_retained_mutations',
                    'tests::c_element_text_content_renders_and_matches_native_rust')
text = text.replace("['test', '--locked', '-p', 'openui-engine', '--lib']",
                    "['test', '--locked', '-p', 'openui-ffi', '--lib', '--features', 'linux']")
text = text.replace('table-progress-guard', 'native-text-content-guard')
text = text.replace('Chromium table continuation count', 'C text setter must create authored text children')
start = text.index("    row, content = run('fixed-fragmentation-neighbors'")
end = text.index("    report['observed_exit_code'] = 0", start)
text = text[:start] + '''    for name, command, exact_test in [
        ('fixed-text-content-storage', build_base + ['test', '--locked', '-p', 'openui-engine', '--lib'],
         'tests::native_text_content_replaces_children_and_reuses_owned_storage'),
        ('fixed-public-native-conformance', build_base + ['test', '--locked', '-p', 'openui', '--test', 'v02_conformance'], None),
    ]:
        if exact_test:
            command = command + [exact_test, '--', '--exact']
        row, content = run(name, command, FIXED)
        assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
        counts = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', content)
        assert len(counts) == 1 and int(counts[0][0]) > 0 and counts[0][1:] == (b'0', b'0')
        row['test_counts'] = [list(map(int, counts[0]))]
        if exact_test:
            assert counts == [(b'1', b'0', b'0')] and (exact_test + ' ... ok').encode() in content
''' + text[end:]
files[path] = text

path = Path('/tmp/openui-native-text-content-build-v1617.py')
text = files[path]
start = text.index('steps=[\n')
end = text.index("assert source['commit']", start)
text = text[:start] + '''steps=[
 ('clean-workspace',base+['clean']+[arg for package in packages for arg in ['-p',package]],None),
 ('workspace',base+['test','--workspace','--locked','--features','openui-ffi/linux'],None),
 ('text-build',base+['build','--locked','-p','openui','--example','native_text_content'],'debug/examples/native_text_content'),
 ('ffi-build',base+['build','--locked','-p','openui-ffi','--features','linux'],'debug/libopenui_ffi.so'),
 ('pixel-build',base+['build','--locked','-p','pixel-compare','--bin','pixel_compare'],'debug/pixel_compare'),
 ('ffi-consumers',['python3',str(root/'tools/ffi/verify_abi.py'),'--library',str(out/'libopenui_ffi.so')],None),
]
import importlib.util
spec=importlib.util.spec_from_file_location('ffi_verify',root/'tools/ffi/verify_abi.py')
ffi_verify=importlib.util.module_from_spec(spec);spec.loader.exec_module(ffi_verify)
cc,cxx,c_flags,cxx_flags,link_flags=ffi_verify.compilers();assert cc and cxx
for language,suffix,compiler,standard,flags in [('c','c',cc,'c11',c_flags),('cpp','cc',cxx,'c++17',cxx_flags)]:
 object_file=out/('text_content-'+language+'.o');binary=out/('text_content-'+language)
 steps.extend([
  ('compile-'+language,[compiler,'-std='+standard,'-Wall','-Wextra','-Werror',*flags,'-I'+str(root/'include'),str(root/'examples/c_v02'/('text_content.'+suffix)),'-c','-o',str(object_file)],None),
  ('link-'+language,[cxx,*cxx_flags,*link_flags,'-fuse-ld=lld',str(object_file),str(out/'libopenui_ffi.so'),'-Wl,-rpath,'+str(out),'-o',str(binary)],str(binary)),
  ('run-'+language,[str(binary)],None),
 ])
assert len(steps)==12
''' + text[end:]
files[path] = text

old = Path('/tmp/openui-native-raster-fields-retry-consumer-v1559.py')
text = old.read_text()
originals[str(old)] = sha(old)
for before, after in [('/dev/shm/openui-native-raster-fields-retry-e0dc491e', str(ROOT)),
                      ('e0dc491e61e17ce4407ff2dd30289e741690572b', FIXED),
                      ('native-raster-fields-retry-consumer-v1559', 'native-text-content-consumer-v1617'),
                      ('native-raster-fields-retry-clean-v1559', 'native-text-content-clean-v1617'),
                      ("len(build['steps']) == 17", "len(build['steps']) == 12"),
                      ("'native_font_raster'", "'native_text_content'"),
                      ("'raster_configuration-c'", "'text_content-c'"),
                      ("'raster_configuration-cpp'", "'text_content-cpp'"),
                      ("policies = dict(freetype='Freetype', fontations='Fontations')", "policies = dict(fontations='Fontations')"),
                      ("prior_path = RAW / 'native-font-retry-consumer-v1311/receipt.json'", "prior_path = RAW / 'native-raster-fields-retry-consumer-v1559/receipt.json'")]:
    assert before in text, before
    text = text.replace(before, after)
text = text.replace("args = [str(scale), policy, family, str(size), 'all', '800', '600'] if language == 'rust'", "args = [str(scale), family, str(size)] if language == 'rust'")
text = text.replace("else [str(scale), str(presets[policy]), family, str(size), str(font_dir / font_files[family])]", "else [str(scale), family, str(size), str(font_dir / font_files[family])]")
text = text.replace('dict(cases=200, images=1200, phase_states=76800,', 'dict(cases=100, images=600, phase_states=38400,')
text = text.replace('images_exact=1200, phases_exact=76800, geometry_exact=76800, rust_pixels_equal=1200, rust_geometry_equal=1200)', 'images_exact=600, phases_exact=38400, geometry_exact=38400, rust_pixels_equal=600, rust_geometry_equal=600)')
text = text.replace("owner='openui-text: font selection, physical strike and origin' if analysis['mismatched_pixels'] else None,", "owner='native text layout and default glyph raster; root cause review required' if analysis['mismatched_pixels'] else None,")
text = text.replace("report = dict(schema_version=1, source=source, source_after=source,", "report = dict(schema_version=1, native_raster_policy='immutable default EngineOptions', chromium_reference_policy='pinned Linux default Fontations', cross_language_self_rows_are_not_cross_language_passes=True, source=source, source_after=source,")
files[Path('/tmp/openui-native-text-content-consumer-v1617.py')] = text

config = dict(prior, root=str(ROOT), commit=FIXED, name='native-text-content-pipeline-v1618',
              initial_state='awaiting-all-32-prior-whole-pipelines', prior_pipelines=priors,
              selections=['native-text-content-checks-v1616/receipt.json',
                          'native-text-content-consumer-audit-v1615.json',
                          'native-raster-fields-retry-consumer-v1559/receipt.json'])
config['scripts'] = [p.name for p in files]
config['stages'] = [
    ('guards', ['/usr/bin/python3', '/tmp/openui-native-text-content-guards-v1617.py'], 'native-text-content-guards-v1617/receipt.json', True),
    ('native-build', ['/usr/bin/python3', '/tmp/openui-native-text-content-build-v1617.py'], 'native-text-content-clean-v1617/build.json', True),
    ('native-application', ['/usr/bin/python3', '/tmp/openui-native-text-content-consumer-v1617.py'], 'native-text-content-consumer-v1617/receipt.json', True),
] + [(suite, ['/usr/bin/python3', '/tmp/openui-native-text-content-matrices-v1617.py', suite],
      f'native-text-content-clean-{suite}-v1617/{suite}-summary.json', False)
     for suite in ('focused', 'primitive', 'full', 'expanded')]
body = prior_text.split('\n', 1)[1]
start = body.index('report.update(owner_pid=')
end = body.index("receipt = OUT / 'receipt.json'", start)
body = body[:start] + f'''report.update(owner_pid=os.getpid(), baseline_commit={BASELINE!r}, expected_stages=7,
    workspace_cleaned_at_every_source_switch=True, existing_113_exports_and_30_layouts_preserved=True,
    native_event_api_preserved=True, c_abi_unchanged=True, actual_pixel_gain_claimed=False,
    root_cause_owner='shared Engine native text replacement and C facade',
    strict_named_baseline_failure_required=True, public_rust_c_cpp_callbacks_and_owned_bounds=True,
    strict_chromium_capture_pairs=True, preserves_both_unstable_reference_captures=True,
    accepted_renderer_unchanged=True, native_api_and_chromium_pixels_still_pending=True)
''' + body[end:]
owner = Path('/tmp/openui-native-text-content-pipeline-v1618.py')
files[owner] = 'CONFIG = ' + repr(config) + '\n' + body
for path, text in files.items():
    assert not path.exists()
    ast.parse(text)
    assert '/dev/shm/openui-native-table-source-' not in text
    path.write_text(text)
report = {'schema_version': 1, 'source': source, 'source_after': source, 'baseline_commit': BASELINE,
          'branch': BRANCH, 'read_only_checks_passed': 13, 'prior_whole_owners': 32,
          'prior_whole_pipelines': priors, 'required_stages': 7, 'required_build_steps': 12,
          'strict_named_c_baseline_failure_required': True, 'repeated_native_updates': 10000,
          'required_native_cases': 100, 'required_native_image_comparisons': 600,
          'required_native_phase_states': 38400, 'immutable_chromium_references_reused': True,
          'chromium_reference_policy': 'pinned Linux default Fontations',
          'native_raster_policy': 'immutable default EngineOptions',
          'all_four_matrices_required': True, 'current_exports': 113, 'current_c_layouts': 30,
          'javascript_executed_by_openui': False, 'public_native_rust_c_cpp_api_required': True,
          'applied_to_umbrella': False, 'native_or_pixel_qualification': False,
          'new_release_states_admitted': 0, 'release_qualification': False,
          'cargo_commands_run': 0, 'screenshots_generated': 0,
          'scripts': {str(p): sha(p) for p in files}, 'original_probe_sha256': originals,
          'checks_receipt_sha256': sha(checks_path), 'probe_sha256': sha(Path(__file__))}
path = RAW / 'native-text-content-queue-prepared-v1617.json'
assert not path.exists()
path.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
assert repository_source_identity(ROOT) == source
print(json.dumps({'receipt': str(path), 'sha256': sha(path), 'source': FIXED,
                  'baseline': BASELINE, 'prior_whole_owners': 32, 'stages': 7}), flush=True)
