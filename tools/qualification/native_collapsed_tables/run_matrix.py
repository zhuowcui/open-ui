"""Scheduled public-native/pinned-Chromium matrix owner; never import to start.

Run only after the prior exclusive Cargo/raster owner has been audited/reaped.
Outputs are new evidence. This never edits originals or a frozen image archive.
An incomplete owner result remains unqualified even when its finished cells pass.
"""
from __future__ import annotations
import argparse
import base64
import fcntl
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
READY="new Promise(resolve=>{const done=()=>document.fonts.ready.then(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));if(document.readyState==='complete')done();else addEventListener('load',done,{once:true});})"


def load(path): return json.loads(Path(path).read_bytes())
def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()
def verify_hashes(inventory):
    for path,expected in inventory.items(): assert sha(path)==expected,path
def json_save(path,value):
    temporary=path.with_name(path.name+'.tmp')
    temporary.write_text(json.dumps(value,sort_keys=True,indent=2)+'\n')
    os.replace(temporary,path)
def group_members(group):
    """Return live writers, including descendants of an exited group leader."""
    members=[]
    for entry in os.scandir('/proc'):
        if not entry.name.isdigit(): continue
        try: fields=(Path(entry.path)/'stat').read_text().rsplit(')',1)[1].split()
        except (FileNotFoundError,ProcessLookupError): continue
        # An unreadable status is unknown, never proof that a group is absent.
        if int(fields[2])==group and fields[0] not in ('Z','X'):
            members.append(int(entry.name))
    return sorted(members)
def stop(child):
    # Every owned child starts a new session. Its descendants can remain after
    # child.poll() reaps the leader, so always retire the whole owned group.
    for action in (signal.SIGTERM,signal.SIGKILL):
        try: os.killpg(child.pid,action)
        except ProcessLookupError: pass
        if action==signal.SIGTERM:
            try: child.wait(timeout=5)
            except subprocess.TimeoutExpired: pass
    code=child.wait(timeout=5)
    deadline=time.monotonic()+5
    while group_members(child.pid):
        if time.monotonic()>=deadline:
            raise TimeoutError(f'owned process group {child.pid} still has writers')
        time.sleep(0.01)
    return code
def rgba(path,physical):
    from PIL import Image
    with Image.open(path) as image:
        expected=(int(physical['width']),int(physical['height']))
        assert image.size==expected,f'{path}: image size {image.size} != contract {expected}'
        converted=image.convert('RGBA')
        return dict(width=converted.width,height=converted.height,rgba_sha256=hashlib.sha256(converted.tobytes()).hexdigest(),png_sha256=sha(path))
def profiles(contract,suite):
    rows=[]
    sets=[('full',contract['qualification_profiles']),('focused',contract['focused_cross_product'])]
    for label,entries in sets:
        if suite not in ['all',label]: continue
        for index,entry in enumerate(entries):
            logical=entry['logical_size_css_px']
            rows.append(dict(name=f'{label}-{index}',suite=label,width=int(logical['width']),height=int(logical['height']),scale=float(entry['device_scale']),physical=entry['physical_size_px']))
    return rows

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--expected-commit',required=True)
    parser.add_argument('--prior-reaping-receipt',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--target',required=True,type=Path)
    parser.add_argument('--chromium',required=True,type=Path)
    parser.add_argument('--suite',choices=['all','full','focused'],default='all')
    parser.add_argument('--case',action='append')
    parser.add_argument('--policy',choices=['auto','fitted'],default='auto')
    args=parser.parse_args()
    lock=open('/tmp/openui-native-cargo-raster-owner.lock','a+')
    fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    prior=load(args.prior_reaping_receipt)
    assert prior.get('all_local_commands_terminal') is True and prior.get('owners_absent') is True,'prior owner requires saved terminal/reaping evidence'
    for process in ['cargo','rustc','rustfmt','pixel_compare']:
        assert subprocess.run(['pgrep','-x',process],capture_output=True).returncode==1,process+' is live'
    sys.path[:0]=[str(ROOT),str(ROOT/'tools/qualification')]
    from renderer_source_identity import repository_source_identity
    from tools.accountability import run_all_pixel_comparisons as capture
    from residuals import analyze_image_difference
    source=repository_source_identity(ROOT)
    assert source['clean'] and source['commit']==args.expected_commit
    assert not args.output.exists(),'output must be fresh'
    assert not args.output.resolve().is_relative_to(ROOT.resolve()),'evidence must stay outside source tree'
    args.output.mkdir()
    # Save partial truth before any temporary directories, copies or font setup.
    report=dict(schema_version=2,owner_pid=os.getpid(),source=source,input_sha256={},driver_sha256=sha(__file__),chromium_binary=str(args.chromium),required_full_scope=(not args.case and args.suite=='all'),policy=args.policy,pixel_target='Chromium 147.0.7727.50',pixel_tolerance=0,javascript_executed_by_openui=False,oracle_html_has_scripts=False,oracle_measurement_uses_cdp_scripts=True,setup_complete=False,all_commands_terminal=False,owned_groups_absent=False,matrix_qualified=False,renderer_qualified=False,release_qualified=False,steps=[],cases=[],test_results=[],cleanup_errors=[])
    receipt=args.output/'receipt.json'
    inputs={};compiled={};retained=None;children=[];retired=set();temporary=None
    def save(): json_save(receipt,report)
    def retire(child,name):
        if child.pid in retired: return True
        try:
            stop(child)
            retired.add(child.pid)
            return True
        except BaseException as error:
            report['cleanup_errors'].append(dict(name=name,pid=child.pid,error=repr(error)))
            return False
    def guard():
        assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip()==source['commit']
        assert subprocess.check_output(['git','status','--porcelain=v1','--untracked-files=all'],cwd=ROOT)==b''
        verify_hashes(inputs)
        assert sha(args.chromium)==report['chromium_binary_sha256']
        assert sha(chrome_env['FONTCONFIG_FILE'])==report['fontconfig_sha256']
        verify_hashes(compiled)
    def run(name,command,timeout=1200,cwd=ROOT/'bindings/rust'):
        guard();print('Starting '+name,flush=True)
        output=args.output/(name+'.stdout.log');errors=args.output/(name+'.stderr.log')
        child=None;code=None;started=time.monotonic();failure=None
        try:
            with output.open('xb') as stdout,errors.open('xb') as stderr:
                child=subprocess.Popen(command,cwd=cwd,env=env,stdout=stdout,stderr=stderr,start_new_session=True);children.append(child)
                report['current_process']=dict(name=name,pid=child.pid);save()
                while child.poll() is None:
                    if time.monotonic()-started>timeout: raise TimeoutError(name)
                    guard();time.sleep(1)
                code=child.wait()
        except BaseException as error:
            failure=repr(error)
            raise
        finally:
            cleaned=child is None or retire(child,name)
            if child is not None: code=child.poll()
            report['steps'].append(dict(name=name,command=command,pid=child.pid if child else None,actual_exit_code=code,failure=failure,owned_group_absent=cleaned,stdout_sha256=sha(output) if output.exists() else None,stderr_sha256=sha(errors) if errors.exists() else None))
            report.pop('current_process',None);save()
            if not cleaned and failure is None: raise RuntimeError(name+' cleanup failed')
        return code,output
    def evaluate(client,expression):
        result=client.command('Runtime.evaluate',dict(expression=expression,awaitPromise=True,returnByValue=True))
        assert 'exceptionDetails' not in result,result
        return result['result'].get('value')
    def browser(directory,profile,repeat,entries):
        guard();profile_dir=temporary/f"chrome-{profile['name']}-{repeat}";profile_dir.mkdir()
        command=[str(args.chromium),'--headless','--disable-gpu','--no-sandbox','--no-first-run','--no-default-browser-check','--remote-debugging-port=0',f'--user-data-dir={profile_dir}',f"--window-size={round(profile['width']*profile['scale'])},{round(profile['height']*profile['scale'])+87}",'about:blank']
        child=subprocess.Popen(command,env=chrome_env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True);children.append(child)
        report['current_process']=dict(name='chromium',pid=child.pid);save();client=None
        try:
            port=capture._wait_for_devtools_endpoint(profile_dir,child)
            client=capture._CdpWebSocket(capture._page_websocket_url(port));client.command('Page.enable')
            client.command('Emulation.setDeviceMetricsOverride',dict(width=profile['width'],height=profile['height'],deviceScaleFactor=profile['scale'],mobile=False,screenWidth=profile['width'],screenHeight=profile['height']))
            for case,entry in zip(cases,entries):
                guard();folder=directory/case['name'];folder.mkdir(exist_ok=True)
                original=HERE/('original' if case['origin']=='reviewed-original' else 'neighbors')/(case['name']+'.html')
                instrumented=HERE/case['oracle']
                assert sha(original)==case['original_sha256'] and sha(instrumented)==case['oracle_sha256']
                node_names=[node['id'] for node in case['nodes'] if node['probe']]
                query="(()=>{const rect=r=>({x:r.x,y:r.y,width:r.width,height:r.height});const names="+json.dumps(node_names)+";return {nodes:names.map(name=>{const e=document.getElementById(name),tag=e.tagName.toLowerCase();return {name,tag,parent:e.parentElement?.id||null,bounds:rect(e.getBoundingClientRect()),client_rects:Array.from(e.getClientRects(),rect),scroll_left:e.scrollLeft,scroll_top:e.scrollTop,scroll_metrics:{client_width:e.clientWidth,client_height:e.clientHeight,scroll_width:e.scrollWidth,scroll_height:e.scrollHeight},column_span:['td','th'].includes(tag)?e.colSpan:['col','colgroup'].includes(tag)?e.span:null,row_span:['td','th'].includes(tag)?e.rowSpan:null};}),hits:"+json.dumps(case['hit_points'])+".map(([x,y])=>({x,y,id:document.elementFromPoint(x,y)?.id||null}))};})()"
                result=dict(repeat=repeat,states={})
                for input_kind,path in [('original',original),('instrumented',instrumented)]:
                    client.command('Page.navigate',dict(url=path.resolve().as_uri()))
                    evaluate(client,READY+'.then(()=>true)')
                    actual=evaluate(client,'({width:innerWidth,height:innerHeight,dpr:devicePixelRatio})')
                    assert capture._device_metrics_match(actual,profile['width'],profile['height'],profile['scale'])
                    if input_kind=='original': states=['initial']
                    else: states=['initial','callback','restored']
                    for state in states:
                        if state=='callback':evaluate(client,"document.querySelector('table').style.setProperty('opacity','0.5');true")
                        elif state=='restored':evaluate(client,"document.querySelector('table').style.removeProperty('opacity');true")
                        evaluate(client,READY+'.then(()=>true)')
                        key=f'{input_kind}-{state}';images=[]
                        for number in [1,2]:
                            filename=folder/f'chromium-{repeat}-{key}-{number}.png'
                            raw=base64.b64decode(client.command('Page.captureScreenshot',dict(format='png',fromSurface=True,captureBeyondViewport=False))['data'],validate=True)
                            filename.write_bytes(raw);images.append(rgba(filename,profile['physical']))
                        assert images[0]==images[1],'browser unchanged frame differs'
                        observation=dict(images=images)
                        if input_kind=='instrumented':
                            bounds=[evaluate(client,READY+'.then(()=>'+query+')') for _ in [1,2]]
                            assert bounds[0]==bounds[1],'browser geometry is unstable'
                            filename=folder/f'chromium-{repeat}-{state}-bounds.json';json_save(filename,bounds[0])
                            observation.update(snapshot=bounds[0],snapshot_sha256=sha(filename))
                        result['states'][key]=observation
                original=result['states']['original-initial']['images'][0]
                initial=result['states']['instrumented-initial']['images'][0]
                restored=result['states']['instrumented-restored']['images'][0]
                result['instrumentation_pixel_inert']=original['rgba_sha256']==initial['rgba_sha256']
                result['oracle_restoration_exact']=initial==restored
                entry['oracles'].append(result);save()
                print(json.dumps(dict(stage='oracle',profile=profile['name'],case=case['name'],repeat=repeat,instrumentation_pixel_inert=result['instrumentation_pixel_inert'])),flush=True)
        finally:
            name=f"chromium-{profile['name']}-{repeat}"
            # Closing a websocket must not prevent retiring browser writers.
            if client: capture._close_chromium_client(client)
            cleaned=retire(child,name)
            report['steps'].append(dict(name=name,command=command,pid=child.pid,actual_exit_code=child.poll(),owned_group_absent=cleaned))
            report.pop('current_process',None)
            if cleaned:
                try: shutil.rmtree(profile_dir)
                except BaseException as error:
                    report['cleanup_errors'].append(dict(name=name+'-profile',error=repr(error)))
                    cleaned=False
            save()
            if not cleaned: raise RuntimeError(name+' cleanup failed')
    code=1
    try:
        save()
        temporary=Path(tempfile.mkdtemp(prefix='openui-collapsed-table-',dir='/dev/shm'))
        report['temporary_directory']=str(temporary);save()
        project=args.output/'consumer';(project/'src').mkdir(parents=True)
        app_source=ROOT/'bindings/rust/openui/examples/native_collapsed_table.rs'
        shutil.copyfile(app_source,project/'src/main.rs')
        manifest=(HERE/'consumer/Cargo.toml.in').read_text().replace('@OPENUI_ROOT@',str(ROOT))
        (project/'Cargo.toml').write_text(manifest)
        shutil.copyfile(HERE/'consumer/Cargo.lock',project/'Cargo.lock')
        fixture_path=HERE/'fixtures.json';fixtures=load(fixture_path)
        assert fixtures['original_case_count']==26 and len(fixtures['cases'])==31
        cases=fixtures['cases']
        if args.case:
            assert len(set(args.case))==len(args.case)
            by_name={case['name']:case for case in cases}
            assert set(args.case)<=by_name.keys()
            cases=[case for case in cases if case['name'] in args.case]
        contract_path=ROOT/'docs/renderer/generated/qualification-contract-v2.json'
        selected_profiles=profiles(load(contract_path),args.suite)
        inputs.update({str(path):sha(path) for path in sorted(HERE.rglob('*')) if path.is_file()})
        inputs.update({str(app_source):sha(app_source),str(contract_path):sha(contract_path),str(ROOT/'bindings/rust/Cargo.lock'):sha(ROOT/'bindings/rust/Cargo.lock'),str(args.prior_reaping_receipt):sha(args.prior_reaping_receipt)})
        inputs.update({str(path):sha(path) for path in project.rglob('*') if path.is_file()})
        chrome_env=capture.chrome_environment(str(args.chromium.parent),True,False)
        chrome_env['TMPDIR']=str(temporary)
        chrome_env.pop('LD_PRELOAD',None)
        env=dict(os.environ,CARGO_TARGET_DIR=str(args.target),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='4',RUST_MIN_STACK='4194304',PYTHONDONTWRITEBYTECODE='1',TMPDIR=str(temporary),FONTCONFIG_FILE=chrome_env['FONTCONFIG_FILE'])
        env.pop('LD_PRELOAD',None);env.pop('LD_LIBRARY_PATH',None)
        report.update(input_sha256=inputs,chromium_binary_sha256=sha(args.chromium),fontconfig=str(Path(chrome_env['FONTCONFIG_FILE'])),fontconfig_sha256=sha(chrome_env['FONTCONFIG_FILE']),capture_harness_sha256=sha(ROOT/'tools/accountability/run_all_pixel_comparisons.py'),planned_case_count=len(cases)*len(selected_profiles),profiles=selected_profiles,setup_complete=True)
        save()
        status,_=run('lowering-input-check',[sys.executable,'-B',str(HERE/'lower_guards.py'),'--check'],cwd=ROOT)
        assert status==0,'native table inputs differ from their shared offline lowering'
        status,version_log=run('chromium-version',[str(args.chromium),'--version'],cwd=ROOT)
        assert status==0
        report['chromium_version']=version_log.read_text().strip();assert report['chromium_version'].split()[-1]=='147.0.7727.50'
        report['font_inputs']=[]
        for index,family in enumerate(['Times New Roman','monospace']):
            status,match_log=run(f'font-match-{index}',['fc-match','-f','%{file}\n',family],cwd=ROOT)
            assert status==0
            font=Path(match_log.read_text().strip());report['font_inputs'].append(dict(family=family,file=str(font),sha256=sha(font)));inputs[str(font)]=sha(font)
        cargo=['cargo','--config',str(ROOT/'bindings/rust/.cargo/config.chromium.toml')]
        manifest_args=['--manifest-path',str(project/'Cargo.toml')]
        status,metadata_log=run('metadata',cargo+['metadata','--locked','--offline','--format-version','1']+manifest_args)
        assert status==0
        metadata=load(metadata_log)
        local_ids={package['id'] for package in metadata['packages'] if Path(package['manifest_path']).resolve().is_relative_to(ROOT) or Path(package['manifest_path']).resolve().is_relative_to(project)}
        report['expected_local_package_ids']=sorted(local_ids)
        workspace_args=['--manifest-path',str(ROOT/'bindings/rust/Cargo.toml')]
        for name,arguments in [
            ('engine-span-guards',['test','--locked','--offline','-p','openui-engine','--lib','table_spans::','--','--test-threads=1']),
            ('layout-table-guards',['test','--locked','--offline','-p','openui-layout','--lib','table','--','--test-threads=1']),
            ('paint-all-guards',['test','--locked','--offline','-p','openui-paint','--lib','--','--test-threads=1']),
            ('public-span-guards',['test','--locked','--offline','-p','openui','--test','native_table_spans','--','--test-threads=1']),
            ('public-conformance',['test','--locked','--offline','-p','openui','--test','v02_conformance','--','--test-threads=1']),
        ]:
            required_sources={
                'engine-span-guards':['bindings/rust/openui-engine/src/table_spans.rs'],
                'layout-table-guards':['bindings/rust/openui-layout/src/table.rs','bindings/rust/openui-layout/src/resolved_table_borders.rs'],
                'paint-all-guards':['bindings/rust/openui-paint/src/collapsed_table_painter.rs'],
                'public-span-guards':['bindings/rust/openui/tests/native_table_spans.rs'],
                'public-conformance':['bindings/rust/openui/tests/v02_conformance.rs'],
            }[name]
            required_tests={test for path in required_sources for test in re.findall(r'#\[test\]\s*fn\s+([a-z][a-z0-9_]*)\s*\(', (ROOT/path).read_text())}
            assert required_tests,'required guard source contains no tests'
            status,test_log=run(name,cargo+arguments[:3]+workspace_args+arguments[3:])
            passed_tests={test.rsplit('::',1)[-1] for test in re.findall(r'^test ([a-zA-Z0-9_:]+) \.\.\. ok$',test_log.read_text(),re.M)}
            missing_tests=sorted(required_tests-passed_tests)
            report['test_results'].append(dict(name=name,actual_exit_code=status,required_tests=sorted(required_tests),required_tests_confirmed=not missing_tests,missing_successful_tests=missing_tests));save()
        status,_=run('clean-local-packages',cargo+['clean','--locked','--offline']+manifest_args+[arg for package in sorted(local_ids) for arg in ['-p',package]])
        assert status==0
        status,build_log=run('native-build',cargo+['build','--locked','--offline','--message-format=json-render-diagnostics']+manifest_args)
        assert status==0
        artifacts=[];binary=None
        for line in build_log.read_bytes().splitlines():
            record=json.loads(line)
            if record.get('reason')=='compiler-artifact' and record['package_id'] in local_ids:
                assert record['fresh'] is False
                artifacts.append(record)
                for path in record['filenames']:compiled[path]=sha(path)
                if record.get('executable') and Path(record['target']['src_path']).resolve().is_relative_to(project):binary=Path(record['executable'])
        assert binary and {record['package_id'] for record in artifacts}==local_ids,'incomplete local compiler receipt'
        retained=args.output/'native-consumer';shutil.copyfile(binary,retained);retained.chmod(0o755)
        assert sha(retained)==sha(binary)
        compiled[str(retained)]=sha(retained)
        report.update(compiler_artifacts=artifacts,compiled_artifact_sha256=compiled,retained_binary=str(retained),binary_sha256=sha(retained));save()
        for profile in selected_profiles:
            directory=args.output/profile['name'];directory.mkdir()
            entries=[dict(profile=profile,name=case['name'],origin=case['origin'],oracles=[],native=[],complete=False,exact=False) for case in cases]
            report['cases'].extend(entries);save()
            for repeat in [1,2]:browser(directory,profile,repeat,entries)
            # Browser processes are stopped/reaped before any native raster run.
            for case,entry in zip(cases,entries):
                folder=directory/case['name']
                for repeat in [1,2]:
                    native=folder/f'native-{repeat}'
                    status,_=run(f"{profile['name']}-{case['name']}-{repeat}",[str(retained),str(native),str(profile['width']),str(profile['height']),str(profile['scale']),str(fixture_path),case['name'],args.policy],timeout=240,cwd=ROOT)
                    observation=dict(repeat=repeat,actual_exit_code=status,directory=str(native),files={path.name:sha(path) for path in sorted(native.iterdir()) if path.is_file()} if native.exists() else {},states={})
                    if (native/'lifecycle.json').exists():observation['lifecycle']=load(native/'lifecycle.json')
                    for state,image_name,bounds_name in [('initial','frame.png','bounds.json'),('callback','callback-frame.png','callback-bounds.json'),('restored','restored-frame.png','restored-bounds.json')]:
                        if not (native/image_name).exists() or not (native/bounds_name).exists():continue
                        expected=entry['oracles'][0]['states']['instrumented-'+state]
                        snapshot=load(native/bounds_name)
                        reference=folder/f'chromium-1-instrumented-{state}-1.png'
                        assert sha(reference)==expected['images'][0]['png_sha256']
                        frame=rgba(native/image_name,profile['physical'])
                        difference=analyze_image_difference(reference,native/image_name)
                        observation['states'][state]=dict(snapshot=snapshot,snapshot_exact=snapshot==expected['snapshot'],difference=difference,frame=frame)
                    entry['native'].append(observation);save()
                entry['complete']=len(entry['native'])==2 and all(set(row['states'])=={'initial','callback','restored'} for row in entry['native'])
                oracle_repeat=entry['oracles'][0]['states']==entry['oracles'][1]['states']
                native_repeat=entry['native'][0]['files']==entry['native'][1]['files']
                entry['exact']=entry['complete'] and oracle_repeat and native_repeat and all(row['instrumentation_pixel_inert'] and row['oracle_restoration_exact'] for row in entry['oracles']) and all(row['actual_exit_code']==0 and all(state['snapshot_exact'] and state['difference']['comparable'] and state['difference']['mismatched_pixels']==0 for state in row['states'].values()) for row in entry['native'])
                entry.update(oracle_repeats_identical=oracle_repeat,native_repeats_identical=native_repeat);save()
                print(json.dumps(dict(stage='native',profile=profile['name'],case=case['name'],complete=entry['complete'],exact=entry['exact'])),flush=True)
        guard();report['source_after']=repository_source_identity(ROOT)
        assert report['source_after']==source
        report.update(completed_case_count=sum(row['complete'] for row in report['cases']),exact_case_count=sum(row['exact'] for row in report['cases']))
        report['all_planned_cases_complete']=report['completed_case_count']==report['planned_case_count']
        report['all_measured_cases_exact']=report['all_planned_cases_complete'] and report['exact_case_count']==report['planned_case_count']
        code=int(not(report['all_measured_cases_exact'] and all(row['actual_exit_code']==0 and row['required_tests_confirmed'] for row in report['test_results'])))
    except BaseException as error:
        report['failure']=repr(error);raise
    finally:
        for child in children: retire(child,'owner-finally')
        report['owned_groups_absent']=all(child.pid in retired for child in children)
        report['all_commands_terminal']=report['owned_groups_absent'] and all(child.poll() is not None for child in children)
        if temporary is not None and report['owned_groups_absent']:
            try: shutil.rmtree(temporary)
            except BaseException as error:
                report['cleanup_errors'].append(dict(name='temporary-directory',error=repr(error)))
        report['temporary_directory_removed']=temporary is None or not temporary.exists()
        code=int(bool(code or report['cleanup_errors'] or not report['all_commands_terminal'] or not report['temporary_directory_removed']))
        # Only the external producer/reaping receipt can record Python's actual
        # exit. An exception after the last save must not look like actual 0.
        report['intended_return_code']=code
        report['actual_owner_exit_requires_external_receipt']=True
        report['fresh_independent_audit_and_saved_reaping_required']=True
        report['matrix_qualified']=False
        try: save()
        finally: lock.close()
    print(json.dumps(dict(planned=report['planned_case_count'],complete=report.get('completed_case_count',0),exact=report.get('exact_case_count',0),intended_return_code=code,all_commands_terminal=report['all_commands_terminal'],matrix_qualified=False)),flush=True)
    return code

if __name__=='__main__':raise SystemExit(main())
