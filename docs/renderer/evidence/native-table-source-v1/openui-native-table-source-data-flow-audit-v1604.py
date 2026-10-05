"""Recompute the geometry projection from unchanged debug logs."""
import hashlib,json,re
from pathlib import Path
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
path=RAW/'native-table-source-data-flow-v1599.json';data=json.loads(path.read_bytes())
receipt=RAW/'native-table-fragment-debug-v1596/receipt.json';debug=json.loads(receipt.read_bytes())
assert debug['all_commands_terminal'] and debug['observed_exit_code']==0 and debug['independent_debug_runs_identical']
assert data['debug_receipt_sha256']==sha(receipt) and data['source']==debug['source']
recomputed=[]
for row in debug['runs'][0]['rows']:
    log=Path(row['log']);assert sha(log)==row['log_sha256']
    selected={};lines=log.read_text().splitlines()
    for line in lines:
        match=re.search(r'node=NodeId\((\d+)\)',line)
        if not match:continue
        node=int(match[1])
        if node not in [4,5,6,8,10,11,12] or node in selected:continue
        values=[int(x) for x in re.findall(r'raw=(-?\d+)',line.split(' margin=',1)[0])]
        entry={'node':node,'offset_raw':values[:2],'size_raw':values[2:4]}
        source=re.search(r'slice=Some\(DecorationSlice \{ source_block_offset: LayoutUnit\([^\[]+\[raw=(-?\d+)\]\), source_block_size: LayoutUnit\([^\[]+\[raw=(-?\d+)\]\)',line)
        if source:entry.update(decoration_source_offset_raw=int(source[1]),decoration_source_size_raw=int(source[2]))
        selected[node]=entry
    table_height=selected[5]['decoration_source_size_raw']/64
    header=selected[6]['size_raw'][1]/64;footer=selected[8]['size_raw'][1]/64
    row_height=selected[10]['size_raw'][1]/64;body_height=selected[12]['size_raw'][1]/64
    recomputed.append(dict(profile=row['profile'],source_table_css_height=table_height,header_css_height=header,
        footer_css_height=footer,cropped_first_body_row_css_height=row_height,
        retained_body_descendant_css_height=body_height,source_body_css_height=table_height-header-footer,
        source_body_remainder_not_represented_by_cropped_row_css_height=body_height-row_height,
        table_fragments_in_this_different_source_fixture=sum('node=NodeId(5) baseline=' in line for line in lines),
        log_sha256=row['log_sha256']))
assert recomputed==data['projections']
assert data['debug_fixture_is_different_from_native_engine_guard'] and data['no_cross_source_result_substitution']
source=Path('/dev/shm/openui-native-table-progress-b7e28e56/bindings/rust/openui-layout/src/block.rs')
assert sha(source)==data['reviewed_function_source_sha256']
report=dict(schema_version=1,all_commands_terminal=True,observed_exit_code=0,
    projection_receipt_sha256=sha(path),debug_receipt_sha256=sha(receipt),
    all_four_projections_recomputed_identically=True,cross_source_result_substitution=False,
    release_qualification=False,chromium_pixel_qualification=False,
    cargo_commands_run=0,screenshots_generated=0,probe_sha256=sha(Path(__file__)))
out=RAW/'native-table-source-data-flow-audit-v1604.json';assert not out.exists();out.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'path':str(out),'sha256':sha(out),'four_profile_projection_verified':True}))
