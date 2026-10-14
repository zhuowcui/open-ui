import fcntl, hashlib, json, os, shutil, signal, subprocess, sys, time
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
BASE = Path('/mnt/d/openui-v02-qualification-d174ea0b')
OUT = BASE / 'public-native-keyboard-renderer-v3552'
TARGET = Path('/mnt/e/openui-v02-cargo-c73754e2/target')
CACHE = BASE / 'public-native-keyboard-renderer-cache-v3552'
POOL = Path('/mnt/e/openui-v02-qualification-d174ea0b/renderer-cache/png')
TEMP = Path('/dev/shm/ou3552')
AUDIT = Path('/tmp/openui-native-keyboard-regressions-completed-audit-v3551.json')
TERMINAL = Path('/tmp/openui-native-keyboard-regressions-terminal-v3553.json')
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
from residuals import canonical_sha256

def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(2**20), b''):
            h.update(chunk)
    return h.hexdigest()

load = lambda p: json.loads(Path(p).read_bytes())
owner = open('/tmp/public-native-keyboard-renderer-v3552.lock', 'a+')
fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
global_lock = open('/tmp/openui-native-cargo-raster-owner.lock', 'a+')
fcntl.flock(global_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
for program in ['cargo', 'rustc', 'rustfmt', 'pixel_compare']:
    assert subprocess.run(['pgrep', '-x', program], capture_output=True).returncode == 1
native = load(AUDIT)
terminal = load(TERMINAL)
assert native['complete'] and native['owner_terminal'] and native['actual_whole_exit_code'] == 0
assert terminal['all_local_commands_terminal'] and terminal['audit_sha256'] == sha(AUDIT)
source = repository_source_identity(ROOT)
assert source == native['source'] and source['clean']
assert not OUT.exists() and not CACHE.exists() and not TEMP.exists() and POOL.is_dir()
OUT.mkdir(); CACHE.mkdir(); TEMP.mkdir()
(CACHE / 'png').symlink_to(POOL, target_is_directory=True)
env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET), CARGO_PROFILE_DEV_DEBUG='0',
           CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='4',
           RUST_MIN_STACK='4194304', PYTHONDONTWRITEBYTECODE='1', TMPDIR=str(TEMP))
env.pop('LD_PRELOAD', None); env.pop('LD_LIBRARY_PATH', None)
report = dict(schema_version=1, owner_pid=os.getpid(), source=source,
              driver_sha256=sha(__file__), native_completed_audit_sha256=sha(AUDIT),
              native_terminal_proof_sha256=sha(TERMINAL), steps=[], matrices={}, comparisons={},
              all_commands_terminal=False, implementation_integrated=True,
              actual_combined_umbrella_source_measured=True, native_metadata_cache_fresh=True,
              cache_directory=str(CACHE), png_pool=str(POOL),
              pixel_target='pinned Chromium', pixel_tolerance=0,
              historical_openui_archive_is_a_pixel_target=False,
              javascript_executed_by_openui=False, renderer_qualified=False,
              all_native_apis_qualified=False, release_qualified=False)

def save():
    temp = OUT / 'receipt.tmp'
    temp.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
    os.replace(temp, OUT / 'receipt.json')

def sweep():
    live = []
    for proc in Path('/proc').iterdir():
        if not proc.name.isdigit():
            continue
        try:
            live.append((proc / 'cmdline').read_bytes())
        except (FileNotFoundError, ProcessLookupError):
            continue
        except PermissionError:
            if proc.stat().st_uid == os.getuid():
                return
    for path in TEMP.glob('openui-chrome-profile-*'):
        if path.is_dir() and not path.is_symlink() and time.time() - path.stat().st_mtime > 20:
            if not any(str(path).encode() in cmd for cmd in live):
                shutil.rmtree(path)
    sockets = [line.split(maxsplit=7)[-1] for line in Path('/proc/net/unix').read_text().splitlines()[1:]
               if len(line.split(maxsplit=7)) == 8]
    for path in TEMP.glob('org.chromium.Chromium.*'):
        if path.is_dir() and not path.is_symlink() and time.time() - path.stat().st_mtime > 30:
            if any(s.startswith(str(path) + '/') for s in sockets):
                continue
            children = list(path.iterdir())
            if all(c.is_symlink() or c.is_socket() for c in children):
                for child in children:
                    child.unlink()
                path.rmdir()

def run(name, command, allowed=(0,)):
    assert not report.get('disk_guard_triggered')
    assert repository_source_identity(ROOT) == source
    print('Starting ' + name, flush=True)
    log = OUT / (name + '.log')
    with log.open('xb') as stream:
        child = subprocess.Popen(command, cwd=ROOT, env=env, stdout=stream,
                                 stderr=subprocess.STDOUT, start_new_session=True)
        report['current_process'] = dict(pid=child.pid, stage=name); save()
        try:
            while child.poll() is None:
                sweep()
                if (shutil.disk_usage(ROOT).free < 128 * 2**20
                        or shutil.disk_usage(OUT).free < 10 * 2**30
                        or shutil.disk_usage(TARGET).free < 10 * 2**30
                        or shutil.disk_usage(TEMP).free < 256 * 2**20):
                    report['disk_guard_triggered'] = True
                    os.killpg(child.pid, signal.SIGTERM)
                    break
                time.sleep(1)
            code = child.wait(timeout=20)
        except BaseException:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL); child.wait()
            raise
        finally:
            report.pop('current_process', None)
            report['steps'].append(dict(name=name, command=command, observed_exit_code=child.returncode,
                                        log_sha256=sha(log))); save()
    print(json.dumps(dict(stage=name, actual_exit_code=code)), flush=True)
    assert code in allowed, name
    assert repository_source_identity(ROOT) == source
    return code

save()
try:
    # The official builder owns the shared lock. Reacquire it only after that child is reaped.
    fcntl.flock(global_lock, fcntl.LOCK_UN)
    build_dir = OUT / 'guarded-build'
    run('guarded-build', [sys.executable, str(ROOT / 'tools/qualification/build_renderer.py'),
                         '--source-root', str(ROOT), '--results-dir', str(build_dir),
                         '--target-dir', str(TARGET), '--cargo-config',
                         str(ROOT / 'bindings/rust/.cargo/config.chromium.toml'), '--offline'])
    fcntl.flock(global_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    build = load(build_dir / 'receipt.json')
    assert build['all_commands_terminal'] and build['observed_exit_code'] == 0
    assert build['renderer_library_provenance_verified'] and build['source'] == build['source_after'] == source
    binary = Path(build['binary'])
    assert sha(binary) == build['binary_sha256']
    report['guarded_build_receipt_sha256'] = sha(build_dir / 'receipt.json'); save()
    fields = ['openui_png_sha256', 'openui_rgba_sha256', 'chromium_png_sha256',
              'chromium_rgba_sha256', 'chromium_oracle_identity_sha256',
              'chromium_oracle_rgba_sha256', 'mismatched_pixels', 'status', 'diff_signature']
    for suite in ['focused', 'primitive', 'full', 'expanded']:
        path = OUT / (suite + '-matrix')
        code = run(suite, [sys.executable, str(ROOT / 'tools/qualification/run_renderer_matrix.py'),
                          '--suite', suite, '--results-dir', str(path), '--cache-dir', str(CACHE),
                          '--oracle-cache-dir', str(ROOT / 'out/renderer-qualification-cache/chromium-oracle'),
                          '--pixel-compare', str(binary), '--build-receipt', str(build_dir / 'receipt.json'),
                          '--chrome', str(ROOT / 'chrome/linux-147.0.7727.50/chrome-linux64/chrome'),
                          '--raster-backend', 'cpu-skia', '--jobs', str(2 if suite in ['focused', 'primitive'] else 8)],
                   allowed=(0, 1))
        summary_path = path / (suite + '-summary.json')
        summary = load(summary_path)
        assert summary['source'] == summary['source_after'] == source and summary['complete_contract_scope']
        assert summary['results']['total'] == dict(focused=640, primitive=960, full=22924, expanded=23728)[suite]
        assert summary['results']['errors'] == 0 and code == int(summary['results']['different'] != 0)
        previous_path = BASE / 'private-native-form-owner-renderer-v3478/trial' / (suite + '-matrix') / (suite + '-summary.json')
        previous = load(previous_path)
        assert previous['complete_contract_scope'] and previous['results']['errors'] == 0
        assert previous['contract_sha256'] == summary['contract_sha256']
        for key in ['resource_hashes', 'font_byte_hashes', 'raster']:
            assert previous[key] == summary[key]
        for key in ['binary_sha256', 'build_identity', 'capture_harness_sha256']:
            assert previous['chromium'][key] == summary['chromium'][key]
        assert previous['id_manifest']['sha256'] == summary['id_manifest']['sha256']
        old = {(p['profile'], t['id']): t for p in previous['profiles'] for t in p['tests']}
        changes = []; gains = losses = worsened = count = 0
        for profile in summary['profiles']:
            assert profile['result_sha256'] == canonical_sha256(profile['tests'])
            for test in profile['tests']:
                prior = old.pop((profile['profile'], test['id'])); count += 1
                assert all(key in prior and key in test for key in fields)
                assert all(prior[key] == test[key] for key in fields if key.startswith('chromium_'))
                gains += prior['status'] != 'exact' and test['status'] == 'exact'
                losses += prior['status'] == 'exact' and test['status'] != 'exact'
                worsened += test['mismatched_pixels'] > prior['mismatched_pixels']
                changed = [key for key in fields if prior[key] != test[key]]
                if changed:
                    changes.append(dict(profile=profile['profile'], id=test['id'], fields=changed,
                                        before={k: prior[k] for k in changed}, after={k: test[k] for k in changed}))
        assert not old
        report['matrices'][suite] = dict(summary_sha256=sha(summary_path), results=summary['results'],
                                         observed_exit_code=code)
        report['comparisons'][suite] = dict(previous_summary=str(previous_path), previous_summary_sha256=sha(previous_path),
                                             audited_rows=count, fields=fields, changes=changes,
                                             exact_gains=gains, exact_losses=losses, worsened=worsened)
        save(); print(json.dumps(dict(suite=suite, results=summary['results'], gains=gains, losses=losses)), flush=True)
    report['candidate_nonregression_verified'] = all(c['exact_losses'] == 0 and c['worsened'] == 0 for c in report['comparisons'].values())
    report['observed_exit_code'] = int(any(m['observed_exit_code'] for m in report['matrices'].values())
                                      or not report['candidate_nonregression_verified'])
except BaseException as error:
    report.update(observed_exit_code=130 if isinstance(error, KeyboardInterrupt) else 1, failure=repr(error))
    raise
finally:
    sweep()
    report['source_after'] = repository_source_identity(ROOT)
    report['source_unchanged'] = report['source_after'] == source
    report['all_commands_terminal'] = True
    save()
    global_lock.close()
print(json.dumps({k: report.get(k) for k in ['observed_exit_code', 'source_unchanged', 'candidate_nonregression_verified']}), flush=True)
raise SystemExit(report['observed_exit_code'])
