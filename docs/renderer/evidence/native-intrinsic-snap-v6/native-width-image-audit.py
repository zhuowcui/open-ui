"""Separate the measured width correction from unchanged glyph pixels."""
import collections
import hashlib
import json
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OLD = RAW / 'native-text-style-runtime-consumer-v1677/receipt.json'
NEW = RAW / 'native-intrinsic-snap-consumer-v1716/receipt.json'
OUT = RAW / 'native-width-image-audit-v1761.json'
assert not OUT.exists()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
old, new = json.loads(OLD.read_bytes()), json.loads(NEW.read_bytes())
assert old['all_commands_terminal'] and new['all_commands_terminal']
assert old['source'] == old['source_after'] and new['source'] == new['source_after']
assert old['source']['commit'] == '41b616c3be5224b074e9d06ba6aa963731a1b265'
assert new['source']['commit'] == '727da10e580c9439f5db83d36bfc3e2340168207'
assert old['font_assets'] == new['font_assets'] and old['chromium'] == new['chromium']
assert old['capture_harness_sha256'] == new['capture_harness_sha256']
key = lambda c: (c['policy'], c['family'], c['size'], c['scale'])
before = {key(c): c for c in old['cases']}
after = {key(c): c for c in new['cases']}
assert before.keys() == after.keys() and len(after) == 100
changes = []
counts = collections.Counter()
images = 0
for k, case in after.items():
    a = {(r['language'], r['state']): r for r in before[k]['images']}
    b = {(r['language'], r['state']): r for r in case['images']}
    assert a.keys() == b.keys() and len(b) == 6
    for image_key, row in b.items():
        prior = a[image_key]
        assert row['native_png_sha256'] == prior['native_png_sha256']
        assert row['chromium_png_sha256'] == prior['chromium_png_sha256']
        assert row['analysis'] == prior['analysis']
        images += 1
        for first, last in zip(prior['phases'], row['phases']):
            assert first['logical_phase_64ths'] == last['logical_phase_64ths']
            assert first['chromium_bounds'] == last['chromium_bounds']
            assert first['mismatched_pixels'] == last['mismatched_pixels']
            if first['native_bounds'] != last['native_bounds']:
                fields = [name for name in first['native_bounds']
                          if first['native_bounds'][name] != last['native_bounds'][name]]
                assert fields == ['width']
                delta = last['native_bounds']['width'] - first['native_bounds']['width']
                assert delta == 1 / 64 and not first['geometry_exact'] and last['geometry_exact']
                counts[(k[1], k[2])] += 1
                changes.append(dict(policy=k[0], family=k[1], size=k[2], scale=k[3],
                                    language=image_key[0], state=image_key[1], phase=last['logical_phase_64ths'],
                                    before=first['native_bounds'], after=last['native_bounds'],
                                    chromium=last['chromium_bounds']))
            else:
                assert first['geometry_exact'] == last['geometry_exact']
assert images == 600 and len(changes) == 3840
report = dict(schema_version=1, source=new['source'], prior_source=old['source'],
              input_receipts={str(OLD): sha(OLD), str(NEW): sha(NEW)},
              all_600_native_png_hashes_unchanged=True, all_600_chromium_png_hashes_unchanged=True,
              all_600_difference_analyses_unchanged=True, all_38400_chromium_bounds_unchanged=True,
              corrected_geometry_states=len(changes), corrected_dimension='width', exact_width_delta=1 / 64,
              changes_by_family_and_size=[dict(family=f, size=s, states=n) for (f, s), n in sorted(counts.items())],
              changes=changes, images_exact=0, images=600, screenshots_generated=0,
              no_raster_or_browser_commands_run=True, formal_wpt_residual_ownership_unchanged=True,
              javascript_executed_by_openui=False, public_native_rust_apis_required=True,
              release_qualification=False, pixel_tolerance=0, new_release_states_admitted=0,
              probe_sha256=sha(Path(__file__)))
OUT.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(receipt_sha256=sha(OUT), unchanged_native_images=600,
                      corrected_geometry_states=3840, unchanged_chromium_images=600)), flush=True)
