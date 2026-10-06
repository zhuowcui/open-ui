"""Read native table geometry through a verified existing C library; no raster."""
import ctypes as c
import hashlib, json, re, math
from pathlib import Path

MAIN=Path('/home/nero/code/open-ui')
ROOT=Path('/dev/shm/openui-native-keywords-native-7d6ffabf-v1787')
RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=RAW/'native-keywords-native-retry-table-geometry-v1787'
STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
build_path=RAW/'native-keywords-current-retry-clean-v1779/build.json'
build=json.loads(build_path.read_bytes())
assert build['all_commands_terminal'] and build['source']==build['source_after'] and build['source']['clean']
row=next(r for r in build['steps'] if r['name']=='ffi-build')
library=Path(row['binary']);assert row['observed_exit_code']==0 and sha(library)==row['binary_sha256']
source_layout=json.loads((ROOT/'docs/v02/generated/openui-ffi-layout.json').read_bytes())
main_layout=json.loads((MAIN/'docs/v02/generated/openui-ffi-layout.json').read_bytes())
used_types=['OuiUtf8','OuiLength','OuiStylePayload','OuiStyleValue','OuiViewportMetrics','OuiDocumentConfig','OuiRect']
assert all(source_layout['types'][name]==main_layout['types'][name] for name in used_types)
assert source_layout['types']==main_layout['types']
assert build['source']['commit']=='7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
header_path=ROOT/'include/openui.h';properties_header_path=ROOT/'include/openui_style_properties.h';header=header_path.read_text()+'\n'+properties_header_path.read_text()
constants={name:int(value) for name,value in re.findall(r'\b(OUI_[A-Z0-9_]+)\s*=\s*(\d+)\b',header)}
required_properties=['DISPLAY','POSITION','LEFT','TOP','WIDTH','HEIGHT','COLUMN_COUNT','COLUMN_FILL','COLUMN_GAP','MARGIN_BOTTOM','BREAK_INSIDE']
assert all('OUI_STYLE_PROPERTY_'+name in constants for name in required_properties)
assert constants['OUI_STYLE_VALUE_COMPOUND']==6 and constants['OUI_VIEWPORT_LOGICAL']==1
api=c.CDLL(str(library))
class Utf8(c.Structure):_fields_=[('data',c.c_void_p),('length',c.c_size_t)]
class Length(c.Structure):_fields_=[('value',c.c_float),('unit',c.c_uint32)]
class Payload(c.Union):_fields_=[('length',Length),('number',c.c_float),('integer',c.c_int32),('compound',c.c_void_p)]
class Value(c.Structure):_fields_=[('tag',c.c_uint32),('reserved',c.c_uint32),('data',Payload)]
class Metrics(c.Structure):_fields_=[('logical_width',c.c_double),('logical_height',c.c_double),('physical_width',c.c_uint32),('physical_height',c.c_uint32),('device_scale',c.c_double),('authority',c.c_uint32),('reserved',c.c_uint32)]
class Config(c.Structure):_fields_=[('struct_size',c.c_uint32),('abi_version',c.c_uint32),('viewport',Metrics)]
class Rect(c.Structure):_fields_=[('x',c.c_float),('y',c.c_float),('width',c.c_float),('height',c.c_float)]
assert c.sizeof(Value)==16 and c.sizeof(Metrics)==40 and c.sizeof(Config)==48 and c.sizeof(Rect)==16
for name,kind in [('OuiUtf8',Utf8),('OuiLength',Length),('OuiStylePayload',Payload),('OuiStyleValue',Value),('OuiViewportMetrics',Metrics),('OuiDocumentConfig',Config),('OuiRect',Rect)]:
    assert dict(size=c.sizeof(kind),align=c.alignment(kind))==source_layout['types'][name]
def bind(name,args):
    fn=getattr(api,name);fn.argtypes=args;fn.restype=c.c_int32;return fn
document_create=bind('oui_document_create',[c.POINTER(Config),c.POINTER(c.c_void_p)])
document_root=bind('oui_document_root',[c.c_void_p,c.POINTER(c.c_void_p)])
element_create=bind('oui_element_create',[c.c_void_p,c.c_int32,c.POINTER(c.c_void_p)])
append=bind('oui_element_append_child',[c.c_void_p,c.c_void_p])
parse=bind('oui_style_value_parse',[c.c_int32,Utf8,c.POINTER(Value)])
set_property=bind('oui_element_set_property',[c.c_void_p,c.c_int32,c.POINTER(Value)])
compound_destroy=bind('oui_style_compound_destroy',[c.c_void_p])
element_destroy=bind('oui_element_destroy',[c.c_void_p])
document_destroy=bind('oui_document_destroy',[c.c_void_p])
client_rects=bind('oui_element_get_client_rects_v1',[c.c_void_p,c.POINTER(Rect),c.c_size_t,c.POINTER(c.c_size_t)])
def ok(status,operation):assert status==0,(operation,status)
def property(element,name,literal):
    identifier=constants['OUI_STYLE_PROPERTY_'+name]
    data=literal.encode();buffer=c.create_string_buffer(data);value=Value()
    ok(parse(identifier,Utf8(c.cast(buffer,c.c_void_p),len(data)),c.byref(value)),'parse '+name+' '+literal)
    try:ok(set_property(element,identifier,c.byref(value)),'set '+name)
    finally:
        if value.tag==constants['OUI_STYLE_VALUE_COMPOUND']:ok(compound_destroy(value.data.compound),'compound destroy')
def rectangles(element):
    count=c.c_size_t();ok(client_rects(element,None,0,c.byref(count)),'rectangle count')
    output=(Rect*count.value)();ok(client_rects(element,output,count.value,c.byref(count)),'rectangles')
    return [dict(x=r.x,y=r.y,width=r.width,height=r.height) for r in output]
report=dict(schema_version=1,source=build['source'],all_commands_terminal=False,release_qualification=False,
    promotion_allowed=False,javascript_executed_by_openui=False,cargo_commands_run=0,screenshots_generated=0,
    native_raster_commands_run=0,public_c_api=True,library_path=str(library),library_sha256=sha(library),
    header_sha256=sha(header_path),properties_header_sha256=sha(properties_header_path),build_receipt_sha256=sha(build_path),probe_sha256=sha(Path(__file__)),runs=[])
save=lambda:(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
report['used_abi_types_verified'] = used_types
report.update(chromium_pixel_qualification=False, geometry_diagnostic_only=True, viewport_rounding_matches_positive_winit_half_away_from_zero=True, geometry_equality_with_chromium_not_claimed=True, expected_native_observations=120, missing_column_fill_constructor_previous_actual_exit=1)
report['prior_preflight'] = dict(probe='/tmp/openui-native-table-c-geometry-v1582.py',probe_sha256=sha(Path('/tmp/openui-native-table-c-geometry-v1582.py')),actual_exit=1,native_library_loaded=False,reason='The pinned raster source has two additive raster configuration structs absent from accepted main; this diagnostic validates every ABI type it uses against the pinned source and checks the existing layouts remain unchanged.')
report['prior_adapter_failure'] = dict(probe='/tmp/openui-native-table-c-geometry-v1583.py',probe_sha256=sha(Path('/tmp/openui-native-table-c-geometry-v1583.py')),actual_exit=1,geometry_queries_completed=0,reason='Style property constants live in the included generated header. The corrected adapter reads both source headers and follows their exact field declarations.')
save()
try:
    for repeat in [1,2]:
        rows=[]
        for scale in [1,1.25,1.5,2,3]:
            for outer_height in [40,30,20,40.5]:
                config=Config(c.sizeof(Config),0x00020000,Metrics(375,667,math.floor(375*scale+0.5),math.floor(667*scale+0.5),scale,constants['OUI_VIEWPORT_LOGICAL'],0))
                document=c.c_void_p();root=c.c_void_p();handles=[]
                ok(document_create(c.byref(config),c.byref(document)),'document create')
                try:
                    ok(document_root(document,c.byref(root)),'document root');handles.append(root)
                    def child(parent,display):
                        node=c.c_void_p();ok(element_create(document,constants['OUI_ELEMENT_DIV'],c.byref(node)),'element create')
                        handles.append(node);property(node,'DISPLAY',display);ok(append(parent,node),'append');return node
                    outer=child(root,'block')
                    for name,literal in [('POSITION','absolute'),('LEFT','120px'),('TOP','120px'),('WIDTH','135px'),('HEIGHT',f'{outer_height}px'),('COLUMN_COUNT','4'),('COLUMN_FILL','auto'),('COLUMN_GAP','16px')]:property(outer,name,literal)
                    spacer=child(outer,'block');property(spacer,'MARGIN_BOTTOM','-60px')
                    inner=child(outer,'block');property(inner,'COLUMN_COUNT','1');property(inner,'COLUMN_FILL','auto')
                    table=child(inner,'table')
                    for display in ['table-header-group','table-footer-group']:
                        group=child(table,display);property(group,'BREAK_INSIDE','avoid')
                        content=child(group,'block');property(content,'WIDTH','20px');property(content,'HEIGHT','20px')
                    table_row=child(table,'table-row');cell=child(table_row,'table-cell');body=child(cell,'block')
                    for state,body_height in [('before',100),('after',50),('restored',100)]:
                        property(body,'HEIGHT',f'{body_height}px')
                        rows.append(dict(scale=scale,outer_height=outer_height,state=state,body_height=body_height,
                            outer=rectangles(outer),inner=rectangles(inner),table=rectangles(table),body=rectangles(body)))
                finally:
                    for handle in reversed(handles):ok(element_destroy(handle),'element destroy')
                    ok(document_destroy(document),'document destroy')
        report['runs'].append(dict(repeat=repeat,rows=rows));save()
        print(json.dumps(dict(repeat=repeat,observations=len(rows),raster_commands=0)),flush=True)
    assert report['runs'][0]['rows']==report['runs'][1]['rows']
    report.update(observed_exit_code=0,independent_native_runs_identical=True,observations=120)
except BaseException as error:
    report.update(observed_exit_code=1,failure=str(error));raise
finally:
    report['all_commands_terminal']=True;assert sha(library)==report['library_sha256'];save()
print(json.dumps(dict(path=str(OUT/'receipt.json'),sha256=sha(OUT/'receipt.json'),observations=120,identical=True)),flush=True)
