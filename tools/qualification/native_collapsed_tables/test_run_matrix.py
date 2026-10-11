"""Fault guards for the matrix owner; no Cargo, browser or raster commands."""
from __future__ import annotations
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import types
import unittest
from unittest import mock

SPEC=importlib.util.spec_from_file_location('collapsed_table_matrix',Path(__file__).with_name('run_matrix.py'))
matrix=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(matrix)


class MatrixFaultGuards(unittest.TestCase):
    def test_matching_wrong_sized_images_fail_the_contract(self):
        from PIL import Image
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'frame.png'
            Image.new('RGBA',(2,2),(0,0,0,255)).save(path)
            with self.assertRaisesRegex(AssertionError,'image size'):
                matrix.rgba(path,dict(width=3,height=2))
            self.assertEqual(matrix.rgba(path,dict(width=2,height=2))['width'],2)

    def test_retained_executable_inventory_rejects_changed_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'retained-consumer'
            path.write_bytes(b'original executable')
            inventory={str(path):matrix.sha(path)}
            matrix.verify_hashes(inventory)
            path.write_bytes(b'changed executable')
            with self.assertRaises(AssertionError): matrix.verify_hashes(inventory)

    def test_retire_group_after_its_leader_already_exited(self):
        # A real Python grandchild keeps the owned group alive after its parent
        # exits. It does not execute framework, compiler, browser or raster code.
        with tempfile.TemporaryDirectory() as directory:
            pid_file=Path(directory)/'descendant.pid'
            descendant="import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(60)"
            parent="import subprocess,sys; from pathlib import Path; p=subprocess.Popen([sys.executable,'-c',sys.argv[2]]); Path(sys.argv[1]).write_text(str(p.pid))"
            child=subprocess.Popen([sys.executable,'-c',parent,str(pid_file),descendant],start_new_session=True)
            try:
                self.assertEqual(child.wait(timeout=10),0)
                pid=int(pid_file.read_text())
                self.assertIn(pid,matrix.group_members(child.pid))
                self.assertEqual(matrix.stop(child),0)
                self.assertEqual(matrix.group_members(child.pid),[])
            finally:
                try: os.killpg(child.pid,signal.SIGKILL)
                except ProcessLookupError: pass
                child.wait(timeout=10)

    def test_setup_failure_saves_receipt_and_removes_temporary_directory(self):
        self.check_setup_failure(cleanup_fails=False)

    def test_cleanup_failure_is_saved_without_claiming_an_actual_exit(self):
        self.check_setup_failure(cleanup_fails=True)

    def check_setup_failure(self,*,cleanup_fails):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)/'root';root.mkdir()
            output=Path(directory)/'evidence'
            prior=Path(directory)/'prior.json'
            prior.write_text(json.dumps(dict(all_local_commands_terminal=True,owners_absent=True)))
            identity=types.ModuleType('renderer_source_identity')
            identity.repository_source_identity=lambda path:dict(clean=True,commit='fixture-commit')
            accountability=types.ModuleType('tools.accountability')
            accountability.run_all_pixel_comparisons=types.ModuleType('unused_capture')
            residuals=types.ModuleType('residuals')
            residuals.analyze_image_difference=mock.Mock(side_effect=AssertionError('no raster allowed'))
            argv=['run_matrix.py','--expected-commit','fixture-commit','--prior-reaping-receipt',str(prior),'--output',str(output),'--target',str(root/'target'),'--chromium',str(root/'unused-chromium')]
            real_open=open;real_rmtree=matrix.shutil.rmtree
            def isolated_open(path,*args,**kwargs):
                if path=='/tmp/openui-native-cargo-raster-owner.lock': path=Path(directory)/'test-only-owner.lock'
                return real_open(path,*args,**kwargs)
            def cleanup(path,*args,**kwargs):
                if cleanup_fails and str(path).startswith('/dev/shm/openui-collapsed-table-'):
                    raise OSError('injected temporary cleanup failure')
                return real_rmtree(path,*args,**kwargs)
            modules={'renderer_source_identity':identity,'tools.accountability':accountability,'residuals':residuals}
            try:
                with mock.patch.object(matrix,'ROOT',root),mock.patch.object(matrix,'HERE',root/'inputs'),mock.patch.object(sys,'argv',argv),mock.patch.object(sys,'path',list(sys.path)),mock.patch.dict(sys.modules,modules),mock.patch('builtins.open',side_effect=isolated_open),mock.patch.object(matrix.subprocess,'run',return_value=types.SimpleNamespace(returncode=1)),mock.patch.object(matrix.shutil,'rmtree',side_effect=cleanup):
                    # The native app source is deliberately absent. No process
                    # can launch before this failure, and its truth is saved.
                    with self.assertRaises(FileNotFoundError): matrix.main()
                receipt=json.loads((output/'receipt.json').read_bytes())
                self.assertFalse(receipt['setup_complete'])
                self.assertIn('FileNotFoundError',receipt['failure'])
                self.assertEqual(receipt['steps'],[])
                self.assertTrue(receipt['all_commands_terminal'])
                self.assertTrue(receipt['owned_groups_absent'])
                self.assertFalse(receipt['matrix_qualified'])
                self.assertEqual(receipt['intended_return_code'],1)
                self.assertTrue(receipt['actual_owner_exit_requires_external_receipt'])
                self.assertNotIn('observed_exit_code',receipt)
                self.assertNotIn('actual_owner_exit_code',receipt)
                temporary=Path(receipt['temporary_directory'])
                self.assertEqual(temporary.exists(),cleanup_fails)
                self.assertEqual(receipt['temporary_directory_removed'],not cleanup_fails)
                self.assertEqual(bool(receipt['cleanup_errors']),cleanup_fails)
                if cleanup_fails: self.assertIn('injected temporary cleanup failure',receipt['cleanup_errors'][0]['error'])
            finally:
                if (output/'receipt.json').exists():
                    temporary=Path(json.loads((output/'receipt.json').read_bytes())['temporary_directory'])
                    if temporary.exists(): real_rmtree(temporary)


if __name__=='__main__':unittest.main()
