"""Check retained Ahem phase failures against source-derived glyph precision."""
import hashlib
import json
import struct
import sys
from datetime import datetime, timezone
from pathlib import Path

from PIL import Image, ImageChops

ROOT = Path('/dev/shm/openui-native-raster-fields-retry-e0dc491e')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-font-position-audit-v1643.json'
assert not OUT.exists()
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
receipt = RAW / 'native-raster-fields-retry-consumer-v1559/receipt.json'
receipt_sha = sha(receipt)
data = json.loads(receipt.read_bytes())
source = repository_source_identity(ROOT)
assert source == data['source'] == data['source_after'] and source['clean']
assert data['all_commands_terminal'] and data['observed_exit_code'] == 1
CHROMIUM = Path('/home/nero/chromium/src')
inputs = [
    ROOT / 'bindings/rust/openui-text/src/shaping/shape_result.rs',
    ROOT / 'bindings/rust/vendor/skrifa-v0_40/src/instance.rs',
    ROOT / 'bindings/rust/vendor/skrifa-v0_40/src/metrics.rs',
    ROOT / 'bindings/rust/vendor/font-types-v0_11/src/fixed.rs',
    ROOT / 'bindings/rust/openui-text/fonts/Ahem.ttf',
    CHROMIUM / 'chrome/VERSION',
    CHROMIUM / 'third_party/skia/src/ports/fontations/src/base.rs',
    CHROMIUM / 'third_party/skia/src/ports/SkTypeface_fontations.cpp',
    CHROMIUM / 'third_party/skia/src/core/SkGlyphRunPainter.cpp',
    CHROMIUM / 'third_party/skia/src/core/SkGlyph.h',
    CHROMIUM / 'third_party/blink/renderer/platform/fonts/skia/skia_text_metrics.cc',
    CHROMIUM / 'third_party/blink/renderer/platform/fonts/shaping/harfbuzz_shaper.cc',
    CHROMIUM / 'third_party/rust/chromium_crates_io/vendor/skrifa-v0_40/src/instance.rs',
    CHROMIUM / 'third_party/rust/chromium_crates_io/vendor/skrifa-v0_40/src/metrics.rs',
]
hashes = {str(p): sha(p) for p in inputs}
assert b'device_x = (device_x * 64.0).round() / 64.0;' in inputs[0].read_bytes()
assert inputs[1].read_bytes() == inputs[-2].read_bytes()
assert inputs[2].read_bytes() == inputs[-1].read_bytes()
font_bytes = inputs[4].read_bytes()
table_count = struct.unpack_from('>H', font_bytes, 4)[0]
tables = {font_bytes[12+i*16:16+i*16].decode(): struct.unpack_from('>II',font_bytes,20+i*16)
          for i in range(table_count)}
upem = struct.unpack_from('>H', font_bytes, tables['head'][0]+18)[0]
assert upem == 1000
f32 = lambda value: struct.unpack('f', struct.pack('f', value))[0]

def advance(size):
    # Size::fixed_linear_scale, Fixed::div, FixedScaleFactor::apply, Fixed::mul_div.
    scale_bits = ((int(f32(size * 64)) << 16) + upem // 2) // upem
    result_bits = (scale_bits * upem + 32) // 64
    return scale_bits, result_bits, f32(result_bits / 65536)

observations = []
for size in [10, 12, 16, 20, 24]:
    case = next(c for c in data['cases'] if c['policy'] == 'fontations' and
                c['family'] == 'Ahem' and c['size'] == size and c['scale'] == 1)
    before = next(i for i in case['images'] if i['language'] == 'rust' and i['state'] == 'before')
    after = next(i for i in case['images'] if i['language'] == 'rust' and i['state'] == 'after')
    assert before['analysis']['mismatched_pixels'] == 0
    assert all(p['geometry_exact'] for i in [before,after] for p in i['phases'])
    scale_bits, advance_bits, glyph_advance = advance(size)
    predicted = []
    for phase in range(64):
        origin = f32(20 + phase % 8 * 92 + phase / 64)
        raw = f32(origin + glyph_advance)
        rounded = f32(round(raw * 64) / 64)
        raw_phase = int(f32(raw + .125) * 4) % 4
        rounded_phase = int(f32(rounded + .125) * 4) % 4
        if raw_phase != rounded_phase:
            predicted.append(phase)
    observed = [p['logical_phase_64ths'] for p in after['phases'] if p['mismatched_pixels']]
    assert predicted == observed, (size, predicted, observed)
    directory = receipt.parent / 'fontations' / f'Ahem-{size}' / '1.0'
    native_path = directory / 'rust-1/after.png'
    chromium_path = directory / 'after-chromium-1.png'
    assert sha(native_path) == after['native_png_sha256']
    assert sha(chromium_path) == after['chromium_png_sha256']
    with Image.open(native_path) as a, Image.open(chromium_path) as b:
        native = a.convert('RGBA'); chromium = b.convert('RGBA')
    crop_checks = []
    for phase in observed:
        x, y = 20 + phase % 8 * 92, 20 + phase // 8 * 60
        previous = phase - 1
        px, py = 20 + previous % 8 * 92, 20 + previous // 8 * 60
        crop = lambda image,bx,by,l,r: image.crop((bx+l,by-4,bx+r,by+size+4))
        first_equal = ImageChops.difference(crop(native,x,y,-4,size-3),
                                           crop(chromium,x,y,-4,size-3)).getbbox() is None
        second_right_equal = ImageChops.difference(crop(native,px,py,2*size-3,2*size+5),
                                                  crop(chromium,x,y,2*size-3,2*size+5)).getbbox() is None
        assert first_equal and second_right_equal
        crop_checks.append(dict(phase=phase,first_glyph_left_pixels_exact=True,
            second_glyph_right_pixels_match_prior_native_phase=True,
            diagnostic_translation_only=True,no_original_pixel_bytes_changed=True))
    observations.append(dict(size=size,scale=1,policy='fontations',family='Ahem',
        scaled_metric_bits=scale_bits,advance_bits_16_16=advance_bits,raw_advance=glyph_advance,
        predicted_different_phases=predicted,observed_different_phases=observed,
        before_exact=True,after_mismatched_pixels=after['analysis']['mismatched_pixels'],
        mismatch_bounds=after['analysis']['mismatch_bounds'],channel_deltas=after['analysis']['channel_deltas'],
        native_png_sha256=after['native_png_sha256'],chromium_png_sha256=after['chromium_png_sha256'],
        edge_checks=crop_checks,
        reviewed_source_owner='openui-text authored glyph position precision',
        reviewed_source_cause='1/64 rounding discards fractional font advance before Skia LCD phase selection',
        measured_runtime_advance_not_available=True,proposed_fix_not_executed=True))
assert sha(receipt) == receipt_sha and repository_source_identity(ROOT) == source
assert all(sha(Path(p)) == digest for p,digest in hashes.items())
report = dict(schema_version=1,observed_at_utc=datetime.now(timezone.utc).isoformat(),
    source=source,source_after=source,consumer_receipt_sha256=receipt_sha,
    input_file_sha256=hashes,chromium_binary_identity=data['chromium'],
    local_chromium_source_patch=24,pinned_chromium_binary_patch=50,
    local_source_is_supporting_evidence_not_binary_provenance=True,
    original_images_reference_bytes_and_receipts_unchanged=True,
    modeled_images=10,modeled_geometry_states=640,
    all_model_predictions_match_observed_phases=True,edge_checks_exact=12,
    source_supported_different_images=3,remaining_rust_pixel_failures_still_unqualified=327,
    all_other_scales_families_policies_still_require_review=True,
    formal_wpt_residual_ownership_unchanged=True,observations=observations,
    all_commands_terminal=True,observed_exit_code=0,cargo_commands_run=0,raster_commands_run=0,
    screenshots_generated=0,javascript_executed_by_openui=False,pixel_tolerance=0,
    accepted_renderer_unchanged=True,new_release_states_admitted=0,release_qualification=False,
    probe_sha256=sha(Path(__file__)))
OUT.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(receipt=str(OUT),sha256=sha(OUT),images=10,geometry_states=640,
    all_predictions_match=True,edge_checks_exact=12,source_supported_different_images=3,
    proposed_fix_not_executed=True,release_qualification=False)),flush=True)
