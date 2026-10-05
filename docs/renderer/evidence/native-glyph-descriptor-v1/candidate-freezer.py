"""Freeze a general glyph descriptor candidate and an identical baseline guard."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

MAIN = Path('/home/nero/code/open-ui')
RAW = MAIN / 'out/renderer-evidence/native-viewport-scroll-v1'
ROOT = Path('/dev/shm/openui-native-glyph-descriptor-0733955a')
BASELINE = Path('/dev/shm/openui-native-glyph-descriptor-baseline-0733955a-v1740')
BASE = '0733955a8f6a94053f3a253c33219e84ed9702c0'
FILE = 'bindings/rust/openui-text/src/shaping/shape_result.rs'
PAINT = 'bindings/rust/openui-paint/src/text_painter.rs'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
run = lambda args, root=MAIN: subprocess.run(args, cwd=root, check=True)
capture = lambda args, root=MAIN: subprocess.check_output(args, cwd=root, text=True)
assert capture(['git', 'rev-parse', 'HEAD'], ROOT).strip() == BASE
assert not BASELINE.exists()
text = (ROOT / FILE).read_text()
module = text.index('#[cfg(test)]\nmod tests {')
start = text.index('    #[test]\n    fn fontations_preserves_physical_strike_descriptor_for_real_fonts()', module)
opening = text.index('{', start)
depth, end = 1, opening + 1
while depth:
    depth += (text[end] == '{') - (text[end] == '}')
    end += 1
guard = text[start:end] + '\n'
assert text.count('fn fontations_preserves_physical_strike_descriptor_for_real_fonts') == 1
assert 'font_size - 10.0' not in text
assert 'chromium_lcd_raster_x' not in text and 'chromium_lcd_strike_y_offset' not in text
assert 'let mut font = source_font.clone();' in text
assert 'builder.set_metrics(&metrics, inverse_size)' in text
assert sorted(capture(['git', 'diff', '--name-only'], ROOT).splitlines()) == sorted([FILE, PAINT])
run(['git', 'diff', '--check'], ROOT)
run(['git', 'worktree', 'add', '--quiet', '-b', 'agent/native-glyph-descriptor-baseline-v1740',
     str(BASELINE), BASE])
old = (BASELINE / FILE).read_text()
needle = '#[cfg(test)]\nmod tests {\n'
assert old.count(needle) == 1
(BASELINE / FILE).write_text(old.replace(needle, needle + guard, 1))
run(['bash', '-c', 'ulimit -s 262144; rustfmt --edition 2021 --config skip_children=true ' + FILE], BASELINE)
run(['git', 'diff', '--check'], BASELINE)
run(['git', 'add', FILE], BASELINE)
run(['git', 'commit', '--quiet', '-m', 'test(text): reproduce lost physical glyph strike descriptor'], BASELINE)
run(['git', 'add', FILE, PAINT], ROOT)
run(['git', 'commit', '--quiet', '-m', 'fix(text): preserve physical strike descriptors and configured glyph coverage'], ROOT)
sys.path.insert(0, str(MAIN / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
source = repository_source_identity(ROOT)
baseline_source = repository_source_identity(BASELINE)
assert source['clean'] and baseline_source['clean']
patch = RAW / 'native-glyph-descriptor-source-v1740.patch'
assert not patch.exists()
patch.write_bytes(subprocess.check_output(['git', 'diff', BASE, source['commit'], '--', FILE, PAINT], cwd=ROOT))
report = dict(schema_version=1, source=source, baseline_source=baseline_source,
              root=str(ROOT), baseline_root=str(BASELINE), parent=BASE,
              changed_paths=[FILE, PAINT], patch_path=str(patch), patch_sha256=sha(patch),
              guard_sha256=hashlib.sha256(guard.encode()).hexdigest(),
              physical_font_size_retained=True, normalized_fitted_outlines=True,
              configured_hinting_edging_and_pixel_geometry_preserved=True,
              old_size_specific_phase_overrides_removed=True,
              public_native_rust_and_c_apis_unchanged=True, raster_defaults_unchanged=True,
              glyph_commands_executed=0, screenshots_generated=0,
              local_build_and_raster_owner_not_started=True,
              awaiting_whole_width_pipeline_v1717=True,
              javascript_executed_by_openui=False, public_native_rust_apis_required=True,
              pixel_tolerance=0, release_qualification=False, new_release_states_admitted=0,
              probe_sha256=sha(Path(__file__)))
p = RAW / 'native-glyph-descriptor-source-v1740.json'
assert not p.exists()
p.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(source=source['commit'], baseline=baseline_source['commit'],
                      receipt_sha256=sha(p), glyph_build_and_pixels_unexecuted=True)), flush=True)
