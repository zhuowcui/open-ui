"""Distinguish immutable reference typeface choices from a routing bug claim."""
import hashlib, json, subprocess
from pathlib import Path

MAIN = Path('/home/nero/code/open-ui')
RAW = MAIN / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = RAW / 'native-glyph-font-engine-review-v1837.json'
assert not OUT.exists()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
prior_path = RAW / 'native-text-style-runtime-consumer-v1677/receipt.json'
prior = json.loads(prior_path.read_bytes())
assert sha(prior_path) == 'e05ea78e7ed8b270c5c1f31a82f3df50c41442633368890a86985637fa64524c'
observations = [run for case in prior['cases'] for image in case['images'] if image['language'] == 'rust' for run in image['independent_reference_runs']]
assert len(observations) == 400
assert all(run['feature_flag'] == '--enable-features=FontDataServiceLinux:typeface/Fontations' for run in observations)
harness_path = MAIN / 'tools/accountability/run_all_pixel_comparisons.py'
assert sha(harness_path) == prior['capture_harness_sha256']
harness = harness_path.read_text()
start = harness.index('            if use_real_font or use_freetype_backend:')
end = harness.index('            process = subprocess.Popen(', start)
excerpt = harness[start:end]
assert 'FontDataServiceLinux:typeface/Freetype' in excerpt
sources = {}
for label, commit in [('baseline', 'db03c8facca7502d42f327bae4833acac7379946'), ('candidate', 'c68d946c18ecd1bb6d2f3f84accecaa84cba3651')]:
    text = subprocess.check_output(['git', 'show', commit + ':bindings/rust/openui-geometry/src/raster.rs'], cwd=MAIN, text=True)
    assert 'ChromiumLinuxFontations' not in text and 'chromium_linux_fontations_lcd' not in text
    sources[label] = dict(commit=commit, configuration_source_sha256=hashlib.sha256(text.encode()).hexdigest(), explicit_fontations_constructor_available=False)
report = dict(schema_version=1, all_commands_terminal=True, probe_sha256=sha(Path(__file__)),
              native_reference_receipt_sha256=sha(prior_path), native_reference_observations=400,
              native_reference_font_engine='Fontations', original_real_font_reference_engine='Freetype',
              capture_harness_sha256=sha(harness_path), capture_harness_excerpt=excerpt, sources=sources,
              no_reference_bytes_changed=True, no_new_captures=True,
              source_inference='Authored LCD text remains on FreeType with chromium_linux_lcd. This matches the original real-font capture conditions, but not the saved native Fontations references. The missing explicit Fontations choice must be implemented and measured through public native Rust apps; switching all existing captures to another engine is not a fix.',
              earlier_routing_observation_sha256=sha(RAW / 'native-glyph-authored-routing-review-v1835.json'),
              runtime_dispatch_trace_captured=False, every_pixel_root_cause_proved=False,
              new_formal_wpt_owners_assigned=0, pixel_tolerance=0, javascript_executed_by_openui=False,
              public_native_rust_apis_required=True, new_release_states_admitted=0, release_qualification=False)
OUT.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps({'native_reference_observations':400, 'native_engine':'Fontations', 'original_real_font_engine':'Freetype', 'index_sha256':sha(OUT)}))
