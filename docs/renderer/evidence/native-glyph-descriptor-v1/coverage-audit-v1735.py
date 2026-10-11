"""Audit existing native glyph coverage against immutable Chromium captures."""
import collections
import hashlib
import json
import subprocess
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
RECEIPT = RAW / 'native-text-style-runtime-consumer-v1677/receipt.json'
OUTPUT = RAW / 'native-default-glyph-policy-audit-v1735.json'
assert not OUTPUT.exists()
sha = lambda b: hashlib.sha256(b).hexdigest()
receipt_bytes = RECEIPT.read_bytes()
report = json.loads(receipt_bytes)
assert report['all_commands_terminal'] and report['source'] == report['source_after']
assert report['source']['commit'] == '41b616c3be5224b074e9d06ba6aa963731a1b265'
assert report['native_raster_policy'] == 'immutable default EngineOptions'
assert len(report['cases']) == 100 and report['observed_exit_code'] == 1
rows = []
by_family = collections.defaultdict(collections.Counter)

for case in report['cases']:
    folder = RECEIPT.parent / case['policy'] / (case['family'].replace(' ', '-') + '-' + str(case['size'])) / str(case['scale'])
    assert case['native_repeats_identical'] and case['uses_unchanged_prior_chromium_images']
    for state in ['before', 'after']:
        comparison = next(row for row in case['images'] if row['language'] == 'rust' and row['state'] == state)
        native = folder / 'rust-1' / (state + '.png')
        chromium = folder / (state + '-chromium-1.png')
        native_bytes, chromium_bytes = native.read_bytes(), chromium.read_bytes()
        assert sha(native_bytes) == comparison['native_png_sha256']
        assert sha(chromium_bytes) == comparison['chromium_png_sha256']
        assert native_bytes == (folder / 'rust-2' / (state + '.png')).read_bytes()
        assert chromium_bytes == (folder / (state + '-chromium-2.png')).read_bytes()
        for language in ['c', 'cpp']:
            assert native_bytes == (folder / (language + '-1') / (state + '.png')).read_bytes()
        with Image.open(native) as image:
            native_rgba = np.asarray(image.convert('RGBA'))
        with Image.open(chromium) as image:
            chromium_rgba = np.asarray(image.convert('RGBA'))
        assert native_rgba.shape == chromium_rgba.shape
        native_unequal_rg = int(np.count_nonzero(native_rgba[:, :, 0] != native_rgba[:, :, 1]))
        chromium_unequal_rg = int(np.count_nonzero(chromium_rgba[:, :, 0] != chromium_rgba[:, :, 1]))
        native_nonopaque = int(np.count_nonzero(native_rgba[:, :, 3] != 255))
        chromium_nonopaque = int(np.count_nonzero(chromium_rgba[:, :, 3] != 255))
        # Both authored colors have equal red and green: black before the
        # native callback, blue after it. Equal-channel grayscale coverage
        # keeps that equality. LCD channel coverage need not keep it.
        row = dict(family=case['family'], size=case['size'], scale=case['scale'], state=state,
                   native_path=str(native), chromium_path=str(chromium),
                   native_png_sha256=sha(native_bytes), chromium_png_sha256=sha(chromium_bytes),
                   native_rgba_sha256=sha(native_rgba.tobytes()), chromium_rgba_sha256=sha(chromium_rgba.tobytes()),
                   native_unequal_red_green_pixels=native_unequal_rg,
                   chromium_unequal_red_green_pixels=chromium_unequal_rg,
                   native_nonopaque_pixels=native_nonopaque, chromium_nonopaque_pixels=chromium_nonopaque,
                   mismatched_pixels=int(np.count_nonzero(np.any(native_rgba != chromium_rgba, axis=2))))
        rows.append(row)
        group = by_family[case['family']]
        group['images'] += 1
        group['native_has_channel_specific_coverage'] += int(native_unequal_rg > 0)
        group['chromium_has_channel_specific_coverage'] += int(chromium_unequal_rg > 0)
        group['native_unequal_rg_pixels'] += native_unequal_rg
        group['chromium_unequal_rg_pixels'] += chromium_unequal_rg
        assert native.read_bytes() == native_bytes and chromium.read_bytes() == chromium_bytes

assert len(rows) == 200
assert RECEIPT.read_bytes() == receipt_bytes
source_paths = ['bindings/rust/openui-geometry/src/raster.rs',
                'bindings/rust/openui-paint/src/text_painter.rs',
                'bindings/rust/openui-text/src/shaping/shape_result.rs']
sources = {}
for path in source_paths:
    original = subprocess.check_output(['git', 'show', report['source']['commit'] + ':' + path], cwd=ROOT)
    assert original == (ROOT / path).read_bytes()
    sources[path] = sha(original)
result = dict(schema_version=1, source=report['source'], input_receipt_sha256=sha(receipt_bytes),
              source_files=sources, existing_images_audited=200, existing_native_app_cases=100,
              no_renderer_or_browser_commands_run=True, no_images_generated_or_changed=True,
              all_native_and_chromium_repeats_unchanged=True, all_actual_c_cpp_images_match_rust=True,
              native_images_with_channel_specific_coverage=sum(r['native_unequal_red_green_pixels'] > 0 for r in rows),
              chromium_images_with_channel_specific_coverage=sum(r['chromium_unequal_red_green_pixels'] > 0 for r in rows),
              all_images_opaque=all(r['native_nonopaque_pixels'] == r['chromium_nonopaque_pixels'] == 0 for r in rows),
              compiled_native_default=dict(backend='Skia', author_edging='AntiAlias', author_hinting='Slight',
                                           pixel_geometry='Unknown', gamma_milli=1000, contrast_milli=0),
              root_cause_owner='shared default text raster policy and glyph rendering',
              review_scope='channel coverage discrepancy only; other outline, phase, hinting and color differences remain open',
              family_totals={key: dict(value) for key, value in sorted(by_family.items())}, rows=rows,
              formal_wpt_residual_ownership_changed=False, native_width_candidate_not_qualified_by_this_audit=True,
              javascript_executed_by_openui=False, public_native_rust_apis_required=True,
              pixel_tolerance=0, release_qualification=False, new_release_states_admitted=0,
              probe_sha256=sha(Path(__file__).read_bytes()))
OUTPUT.write_text(json.dumps(result, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(receipt_sha256=sha(OUTPUT.read_bytes()), images=200,
                      native_images_with_channel_specific_coverage=result['native_images_with_channel_specific_coverage'],
                      chromium_images_with_channel_specific_coverage=result['chromium_images_with_channel_specific_coverage'],
                      all_images_opaque=result['all_images_opaque'], family_totals=result['family_totals'])), flush=True)
