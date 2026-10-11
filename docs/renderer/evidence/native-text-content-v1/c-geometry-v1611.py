"""Isolate native C text geometry using a verified existing library; no raster."""
import ctypes as c
import hashlib
import json
import re
import sys
from pathlib import Path

MAIN = Path('/home/nero/code/open-ui')
ROOT = Path('/dev/shm/openui-native-raster-fields-retry-e0dc491e')
RAW = MAIN / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = RAW / 'native-font-c-geometry-v1611'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

build_path = RAW / 'native-raster-fields-retry-clean-v1559/build.json'
build = json.loads(build_path.read_bytes())
source = repository_source_identity(ROOT)
assert source['clean'] and source == build['source'] == build['source_after']
assert source['commit'] == 'e0dc491e61e17ce4407ff2dd30289e741690572b'
assert build['all_commands_terminal'] and len(build['steps']) == 17
assert all(x['observed_exit_code'] == 0 for x in build['steps'])
row = next(x for x in build['steps'] if x['name'] == 'ffi-build')
library = Path(row['binary'])
assert sha(library) == row['binary_sha256']
abi_path = ROOT / 'docs/v02/generated/openui-ffi-layout.json'
abi = json.loads(abi_path.read_bytes())['types']
header_paths = [ROOT / 'include/openui.h', ROOT / 'include/openui_style_properties.h']
header = '\n'.join(p.read_text() for p in header_paths)
constants = {n: int(v) for n, v in re.findall(r'\b(OUI_[A-Z0-9_]+)\s*=\s*(\d+)\b', header)}
abi_version = re.search(r'^#define OUI_ABI_VERSION (0x[0-9a-fA-F]+|[0-9]+)[uU]?$', header, re.MULTILINE)
assert abi_version is not None
constants['OUI_ABI_VERSION'] = int(abi_version.group(1), 0)

class Utf8(c.Structure):
    _fields_ = [('data', c.c_void_p), ('length', c.c_size_t)]
class Length(c.Structure):
    _fields_ = [('value', c.c_float), ('unit', c.c_uint32)]
class Payload(c.Union):
    _fields_ = [('length', Length), ('number', c.c_float), ('integer', c.c_int32), ('compound', c.c_void_p)]
class Value(c.Structure):
    _fields_ = [('tag', c.c_uint32), ('reserved', c.c_uint32), ('data', Payload)]
class Metrics(c.Structure):
    _fields_ = [('logical_width', c.c_double), ('logical_height', c.c_double), ('physical_width', c.c_uint32), ('physical_height', c.c_uint32), ('device_scale', c.c_double), ('authority', c.c_uint32), ('reserved', c.c_uint32)]
class Config(c.Structure):
    _fields_ = [('struct_size', c.c_uint32), ('abi_version', c.c_uint32), ('viewport', Metrics)]
class Rect(c.Structure):
    _fields_ = [('x', c.c_float), ('y', c.c_float), ('width', c.c_float), ('height', c.c_float)]
class Descriptor(c.Structure):
    _fields_ = [('struct_size', c.c_uint32), ('abi_version', c.c_uint32), ('family', Utf8), ('face_index', c.c_uint32), ('style', c.c_uint32), ('style_min', c.c_float), ('style_max', c.c_float), ('weight_min', c.c_float), ('weight_max', c.c_float), ('stretch_min', c.c_float), ('stretch_max', c.c_float), ('unicode_ranges', c.c_void_p), ('unicode_range_count', c.c_size_t), ('feature_defaults', c.c_void_p), ('feature_default_count', c.c_size_t), ('size_adjust', c.c_float), ('ascent_override', c.c_float), ('descent_override', c.c_float), ('line_gap_override', c.c_float), ('flags', c.c_uint32), ('reserved', c.c_uint32)]
for name, typ in [('OuiUtf8', Utf8), ('OuiLength', Length), ('OuiStylePayload', Payload), ('OuiStyleValue', Value), ('OuiViewportMetrics', Metrics), ('OuiDocumentConfig', Config), ('OuiRect', Rect), ('OuiFontFaceDescriptor', Descriptor)]:
    assert {'size': c.sizeof(typ), 'align': c.alignment(typ)} == abi[name]

api = c.CDLL(str(library))
def bind(name, args):
    fn = getattr(api, name)
    fn.argtypes = args
    fn.restype = c.c_int32
    return fn
create = bind('oui_document_create', [c.POINTER(Config), c.POINTER(c.c_void_p)])
root = bind('oui_document_root', [c.c_void_p, c.POINTER(c.c_void_p)])
element = bind('oui_element_create', [c.c_void_p, c.c_int32, c.POINTER(c.c_void_p)])
append = bind('oui_element_append_child', [c.c_void_p, c.c_void_p])
parse = bind('oui_style_value_parse', [c.c_int32, Utf8, c.POINTER(Value)])
set_property = bind('oui_element_set_property', [c.c_void_p, c.c_int32, c.POINTER(Value)])
set_text = bind('oui_element_set_text', [c.c_void_p, Utf8])
get_bounds = bind('oui_element_get_bounds', [c.c_void_p, c.POINTER(Rect)])
family_create = bind('oui_font_family_create', [Utf8, c.c_int32, c.POINTER(c.c_void_p)])
compound_destroy = bind('oui_style_compound_destroy', [c.c_void_p])
register_font = bind('oui_document_register_font', [c.c_void_p, c.c_void_p, c.c_size_t, c.POINTER(Descriptor), c.POINTER(c.c_void_p)])
font_destroy = bind('oui_font_face_destroy', [c.c_void_p])
element_destroy = bind('oui_element_destroy', [c.c_void_p])
document_destroy = bind('oui_document_destroy', [c.c_void_p])

def text(value):
    buffer = c.create_string_buffer(value.encode())
    result = Utf8(c.cast(buffer, c.c_void_p), len(value.encode()))
    result.keepalive = buffer
    return result
def ok(status, operation):
    assert status == 0, (operation, status)
def length(node, property_name, number):
    value = Value(tag=constants['OUI_STYLE_VALUE_LENGTH'])
    value.data.length = Length(number, constants['OUI_LENGTH_PX'])
    ok(set_property(node, constants['OUI_STYLE_PROPERTY_' + property_name], c.byref(value)), property_name)
def parsed(node, property_name, literal):
    value = Value()
    property_id = constants['OUI_STYLE_PROPERTY_' + property_name]
    ok(parse(property_id, text(literal), c.byref(value)), 'parse ' + property_name)
    ok(set_property(node, property_id, c.byref(value)), 'set parsed ' + property_name)
    if value.tag == constants['OUI_STYLE_VALUE_COMPOUND']:
        ok(compound_destroy(value.data.compound), 'destroy parsed ' + property_name)
def bounds(node):
    rect = Rect()
    ok(get_bounds(node, c.byref(rect)), 'bounds')
    return {name: getattr(rect, name) for name in ('x', 'y', 'width', 'height')}

report = {'schema_version': 1, 'source': source, 'source_after': source, 'library_sha256': sha(library), 'build_receipt_sha256': sha(build_path), 'abi_sha256': sha(abi_path), 'header_sha256': {str(p): sha(p) for p in header_paths}, 'probe_sha256': sha(Path(__file__)), 'all_commands_terminal': False, 'raster_commands': 0, 'screenshots_generated': 0, 'cargo_commands': 0, 'javascript_executed_by_openui': False, 'release_qualification': False, 'runs': []}
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
try:
    for repeat in (1, 2):
        rows = []
        for family, asset in [('Ahem', 'Ahem.ttf'), ('DejaVu Sans', 'DejaVuSans.ttf')]:
            for register in (False, True):
                document = c.c_void_p()
                cfg = Config(c.sizeof(Config), constants['OUI_ABI_VERSION'], Metrics(800, 600, 0, 0, 1, constants['OUI_VIEWPORT_LOGICAL'], 0))
                ok(create(c.byref(cfg), c.byref(document)), 'create')
                handles = []
                face = c.c_void_p()
                try:
                    if register:
                        data = (ROOT / 'bindings/rust/openui-text/fonts' / asset).read_bytes()
                        storage = c.create_string_buffer(data)
                        desc = Descriptor(struct_size=c.sizeof(Descriptor), abi_version=constants['OUI_ABI_VERSION'], family=text(family), weight_min=400, weight_max=400, stretch_min=100, stretch_max=100)
                        ok(register_font(document, storage, len(data), c.byref(desc), c.byref(face)), 'register font')
                    body = c.c_void_p()
                    ok(root(document, c.byref(body)), 'root')
                    handles.append(body)
                    node = c.c_void_p()
                    ok(element(document, constants['OUI_ELEMENT_DIV'], c.byref(node)), 'element')
                    handles.append(node)
                    ok(append(body, node), 'append')
                    ok(set_text(node, text('X')), 'text')
                    observations = [{'step': 'default', 'bounds': bounds(node)}]
                    parsed(node, 'POSITION', 'absolute')
                    length(node, 'LEFT', 20)
                    length(node, 'TOP', 20)
                    observations.append({'step': 'position', 'bounds': bounds(node)})
                    length(node, 'FONT_SIZE', 20)
                    observations.append({'step': 'scalar-font-size', 'bounds': bounds(node)})
                    parsed(node, 'FONT_SIZE', '20px')
                    observations.append({'step': 'parsed-font-size', 'bounds': bounds(node)})
                    parsed(node, 'LINE_HEIGHT', '1')
                    observations.append({'step': 'line-height', 'bounds': bounds(node)})
                    compound = c.c_void_p()
                    ok(family_create(text(family), 0, c.byref(compound)), 'family')
                    value = Value(tag=constants['OUI_STYLE_VALUE_COMPOUND'])
                    value.data.compound = compound
                    ok(set_property(node, constants['OUI_STYLE_PROPERTY_FONT_FAMILY'], c.byref(value)), 'set family')
                    observations.append({'step': 'family-live', 'bounds': bounds(node)})
                    ok(compound_destroy(compound), 'destroy family')
                    observations.append({'step': 'family-destroyed', 'bounds': bounds(node)})
                    ok(set_text(node, text('XX')), 'mutate text')
                    observations.append({'step': 'text-mutated', 'bounds': bounds(node)})
                    rows.append({'family': family, 'registered': register, 'observations': observations})
                finally:
                    for handle in reversed(handles):
                        ok(element_destroy(handle), 'destroy element')
                    if face:
                        ok(font_destroy(face), 'destroy font')
                    ok(document_destroy(document), 'destroy document')
        report['runs'].append({'repeat': repeat, 'rows': rows})
        save()
    assert report['runs'][0]['rows'] == report['runs'][1]['rows']
    report.update(observed_exit_code=0, repeats_identical=True)
except BaseException as error:
    report.update(observed_exit_code=1, failure=str(error))
    raise
finally:
    report['all_commands_terminal'] = True
    report['source_after'] = repository_source_identity(ROOT)
    assert report['source_after'] == source and sha(library) == report['library_sha256']
    save()
print(json.dumps({'receipt': str(receipt), 'sha256': sha(receipt), 'rows': report['runs'][0]['rows'], 'all_commands_terminal': True}), flush=True)
