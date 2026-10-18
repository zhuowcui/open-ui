import fcntl,gzip,hashlib,json,os,subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-c-qualification-v3528');REF=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-reference-v3506');DEST=ROOT/'docs/renderer/evidence/native-keyboard-controls-v1';SUMMARY=ROOT/'docs/renderer/generated/native-keyboard-controls-v1.json'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a+');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
baseline=json.loads(Path('/tmp/openui-native-keyboard-baseline-completed-audit-v3514.json').read_bytes());assert baseline['complete'] and baseline['behavior_results']==dict(total=99,exact=13,different=86)
r=json.loads((OUT/'receipt.json').read_bytes());a=json.loads(Path('/tmp/openui-native-keyboard-c-completed-audit-v3529.json').read_bytes());integration=json.loads(Path('/mnt/d/openui-v02-qualification-d174ea0b/public-native-keyboard-integration-v3531/receipt.json').read_bytes());t=json.loads(Path('/tmp/openui-native-keyboard-c-terminal-v3530.json').read_bytes())
assert r['all_commands_terminal'] and r['observed_exit_code']==0 and r['source_unchanged'];assert a['complete'] and a['behavior_exact'];assert integration['all_commands_terminal'] and integration['actual_exit_code']==0 and integration['exact_source_bytes_adopted'];assert r['source']==integration['public_source'];assert t['all_commands_terminal'] and t['owner_absent']
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()==r['source']['commit'];assert not subprocess.check_output(['git','status','--porcelain=v1','-z'],cwd=ROOT)
assert not DEST.exists() and not SUMMARY.exists()
assert all(count==dict(total=99,exact=99,different=0) for count in r['behavior_results'].values())
assert 'C ABI verified: symbols=131 C_examples=29 C++=23 ran=True' in (OUT/'all-c-cpp-abi-consumers.log').read_text()
files={};DEST.mkdir()
def archive(origin,name,compressed=False):
 origin=Path(origin);data=origin.read_bytes();target=DEST/name;assert not target.exists();target.write_bytes(gzip.compress(data,mtime=0) if compressed else data)
 assert (gzip.decompress(target.read_bytes()) if compressed else target.read_bytes())==data
 files[name]=dict(sha256=sha(target),bytes=target.stat().st_size,uncompressed_sha256=hashlib.sha256(data).hexdigest(),origin=str(origin))
for origin,name in [('/tmp/openui-native-keyboard-baseline-completed-audit-v3514.json','baseline-completed-audit.json'),(OUT/'receipt.json','qualification-receipt.json'),('/tmp/openui-native-keyboard-c-completed-audit-v3529.json','completed-audit.json'),('/tmp/openui-native-keyboard-c-terminal-v3530.json','terminal-status.json'),('/mnt/d/openui-v02-qualification-d174ea0b/public-native-keyboard-integration-v3531/receipt.json','integration-receipt.json'),(REF/'receipt.json','chromium-receipt.json'),(REF/'cases.json','cases.json'),(REF/'probe.py','chromium-probe.py'),('/tmp/openui-native-keyboard-c-failed-terminal-v3526.json','failed-c-compilation-terminal.json'),('/tmp/openui-native-keyboard-c-qualification-v3528.py','qualification-driver.py'),('/tmp/openui-native-keyboard-c-completed-audit-v3529.py','audit-driver.py'),(__file__,'evidence-builder.py')]:archive(origin,name)
for origin,name in [(REF/'observations-1.json','chromium-observations.json.gz'),(Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-c-qualification-v3524')/'receipt.json','failed-c-compilation-receipt.json.gz')]:archive(origin,name,True)
for language in ['rust','c','cpp']:
 for repeat in [1,2]:archive(OUT/('observations-'+language+'-'+str(repeat)+'.json'),'observations-'+language+'-'+str(repeat)+'.json.gz',True)
 archive(OUT/('comparisons-'+language+'.json'),'comparisons-'+language+'.json')
for step in r['steps']:archive(OUT/(step['name']+'.log'),step['name']+'.log.gz',True)
summary=dict(schema_version=1,scope='Native radio, checkbox and button keyboard defaults and owned active-state queries',measured_source=r['source'],measurement_checkout='private qualification clone',implementation_integrated=True,exact_measured_commit_adopted=True,public_source_bytes_equal_measured_source=True,new_public_qualification_commands_executed=False,current_public_hosted_qualification_complete=False,javascript_executed_by_openui=False,offline_chromium_reference_scripts_only=True,pixel_target='pinned Chromium',pixel_tolerance=0,historical_openui_archive_is_a_pixel_target=False,native_keyboard_behavior_by_language=r['behavior_results'],repeated_processes_identical=True,baseline_rust_behavior=dict(total=99,exact=13,different=86),rust_behavior_gains=86,exports=131,layouts=34,existing_exports_preserved=130,layouts_changed=0,keyboard_recovery_guards_passed=2,active_query_reentrant_borrow_guard_passed=True,c_cpp_active_query_ownership_and_callback_guards_passed=True,abi_consumer_command_passed=True,c_consumers=29,cpp_consumers=23,behavior_trace_fields=['target','related target','event type','bubbles','cancelable','key','key code','modifiers','focus','checked','indeterminate','active pseudo state','connected'],renderer_commands_executed=False,keyboard_pixels_qualified=False,keypress_api_qualified=False,rtl_keyboard_qualified=False,full_workspace_at_this_source_qualified=False,all_native_apis_qualified=False,renderer_qualified=False,compositor_qualified=False,hardware_qualified=False,release_qualified=False,remaining=['Broader and RTL keyboard behavior','Keypress, composition and remaining native APIs','Keyboard focus and held-control appearance','Fresh workspace and hosted qualification','Neighboring control, selection and form regression qualification','Full exact Chromium renderer and expanded corpus gates','Compositor, lab and release gates'],files=files)
SUMMARY.write_text(json.dumps(summary,sort_keys=True,indent=2)+'\n')
block="""The native keyboard control fixes are integrated at code checkpoint
`a0841c9e`. Rust, C and C++ apps each match the same 99 measured Chromium
scenarios, with repeated runs identical. Radio arrow navigation, Space release,
modifiers, cancellation and callback changes use the shared native document.
C adds `oui_element_is_active_v1`; the ABI has 131 exports and the same 34
layouts, preserving all 130 preceding exports. Two Rust cancellation/panic
recovery guards and C/C++ ownership and callback guards pass.

These checks measure state and events. Keyboard appearance, broader keyboard
APIs, fresh workspace and hosted checks, neighboring regressions and renderer
qualification remain open on this source. No new pixel result is claimed.
[Native keyboard evidence](docs/renderer/generated/native-keyboard-controls-v1.json).

"""
edits={}
p=ROOT/'README.md';text=p.read_text();anchor='## Verified status\n\n';assert text.count(anchor)==1;edits[p]=text.replace(anchor,anchor+block)
p=ROOT/'docs/v02/release.md';text=p.read_text();anchor='claimed by source code alone.\n\n';assert text.count(anchor)==1;edits[p]=text.replace(anchor,anchor+block.replace('docs/renderer/generated/','../renderer/generated/'))
p=ROOT/'docs/v02/supported-platforms.md';text=p.read_text();anchor='Lookup, mutation, geometry, focus, scrolling, controls, and event dispatch';assert text.count(anchor)==1
addition="""Native keyboard control defaults now run through public
`Document::dispatch_key_input` and Rust callbacks; C uses the same retained
document. Rust, C and C++ each match 99 measured Chromium scenarios at clean
code checkpoint `a0841c9e`. C exposes the native active-state query through
`oui_element_is_active_v1`. These state and event checks leave keyboard pixels,
broader keyboard APIs and current-source full qualification open.
[Evidence](../renderer/generated/native-keyboard-controls-v1.json).

""";edits[p]=text.replace(anchor,addition+anchor)
for p,text in edits.items():
 tmp=p.with_suffix(p.suffix+'.v3532.tmp');tmp.write_text(text);os.replace(tmp,p)
subprocess.run(['git','diff','--check'],cwd=ROOT,check=True)
print(json.dumps(dict(evidence_files=len(files),evidence_bytes=sum(v['bytes'] for v in files.values()),summary_sha256=sha(SUMMARY))),flush=True)
