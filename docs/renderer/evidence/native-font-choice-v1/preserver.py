"""Preserve the uncompiled native choice and its read-only checks without qualification."""
import gzip, hashlib, json, subprocess, sys
from pathlib import Path

MAIN = Path('/home/nero/code/open-ui')
ROOT = Path('/dev/shm/openui-native-font-choice-c68d946c-v1838')
RAW = MAIN / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = MAIN / 'docs/renderer/evidence/native-font-choice-v1'
INDEX = MAIN / 'docs/renderer/generated/native-font-choice-v1.json'
CHECKS = RAW / 'native-font-choice-checks-v1839/receipt.json'
sys.path.insert(0, str(MAIN / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
checks = json.loads(CHECKS.read_bytes())
source = repository_source_identity(ROOT)
assert source == checks['source'] == checks['source_after'] and source['clean']
assert source['commit'] == 'ef8880b083876319df54599416c0fe11c543a26c'
assert checks['all_commands_terminal'] and len(checks['checks']) == 15
assert all(row['observed_exit_code'] == 0 for row in checks['checks'])
assert not OUT.exists() and not INDEX.exists()
OUT.mkdir()
rows = []
def keep(p, name, expected=None, compress=False):
    data = p.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if expected:
        assert digest == expected
    target = OUT / name
    target.parent.mkdir(parents=True, exist_ok=True)
    assert not target.exists()
    target.write_bytes(gzip.compress(data, mtime=0) if compress else data)
    row = dict(path=str(target.relative_to(MAIN)), sha256=sha(target), bytes=target.stat().st_size, source_path=str(p))
    if compress:
        assert gzip.decompress(target.read_bytes()) == data
        row.update(encoding='gzip', decompressed_sha256=digest, decompressed_bytes=len(data))
    rows.append(row)

keep(CHECKS, 'checks.json')
for row in checks['checks']:
    keep(CHECKS.parent / (row['name'] + '.log'), 'check-' + row['name'] + '.log.gz', row['log_sha256'], True)
keep(Path('/tmp/openui-native-font-choice-checks-v1839.py'), 'checks.py', checks['probe_sha256'])
keep(Path(__file__), 'preserver.py')
paths = subprocess.check_output(['git','diff','--name-only','c68d946c18ecd1bb6d2f3f84accecaa84cba3651',source['commit']],cwd=MAIN,text=True).splitlines()
assert len(paths) == 7
for path in paths:
    keep(ROOT / path, 'source/' + path)
patch = subprocess.check_output(['git','diff','c68d946c18ecd1bb6d2f3f84accecaa84cba3651',source['commit']],cwd=MAIN)
p = OUT / 'native-font-choice.patch'
p.write_bytes(patch)
rows.append(dict(path=str(p.relative_to(MAIN)), sha256=sha(p), bytes=len(patch)))
current = (ROOT / 'bindings/rust/openui-geometry/src/raster.rs').read_text()
parent = subprocess.check_output(['git','show','c68d946c18ecd1bb6d2f3f84accecaa84cba3651:bindings/rust/openui-geometry/src/raster.rs'],cwd=MAIN,text=True)
body = lambda s:s[s.index('impl Default for RasterConfiguration {'):s.index('\n/// ',s.index('impl Default for RasterConfiguration {'))]
assert body(current) == body(parent)
assert repository_source_identity(ROOT) == source
report = dict(schema_version=1, source=source, source_after=source, parent_commit='c68d946c18ecd1bb6d2f3f84accecaa84cba3651',
              public_rust_constructor='RasterConfiguration::chromium_linux_fontations_lcd',
              backend='RasterBackend::ChromiumLinuxFontations', same_retained_engine=True,
              immutable_typeface_choice_retained_with_resolved_fonts=True, ordinary_freetype_choice_preserved=True,
              existing_defaults_unchanged=True, no_new_font_family_size_fixture_or_test_id_rule=True,
              all_fifteen_read_only_checks_passed=True, compiled=False, rust_tests_executed=False,
              native_apps_executed=False, pixel_matrices_executed=False, own_hosted_hardening_executed=False,
              api_rendering_behavior_qualified=False, candidate_applied_to_umbrella=False,
              parent_candidate_native_pixels_unqualified=True,
              remaining_qualification=['Clean compilation and consuming native Rust app execution', 'Exact saved native Fontations reference images and complete scale and phase sweep', 'Default native font pixels', 'All raster-field effects, unsupported outlines and color or variable font behavior', 'Neighboring writing-mode, decoration, shadow, clipping and transform cases', 'All four renderer matrices and actual hosted jobs'],
              javascript_executed_by_openui=False, public_native_rust_apis_required=True, pixel_tolerance=0,
              new_release_states_admitted=0, release_qualification=False, artifacts=rows, artifact_count=len(rows),
              artifact_bytes=sum(r['bytes'] for r in rows))
INDEX.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps({'artifacts':len(rows),'source':source['commit'],'compiled':False,'index_sha256':sha(INDEX)}))
