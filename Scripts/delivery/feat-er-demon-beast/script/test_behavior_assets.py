import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from behavior_assets import append_names, name_table, digest


class BehaviorAssets(unittest.TestCase):
    def test_append_preserves_existing_ids_encoding_and_eof_sentinel(self):
        source = b'#\x82\xa0\r\nNum  = 2\r\n1 = "B"\r\n2 = "A"\r\n\0\0\0\0'
        patched, added = append_names(source, {'A', 'B', 'C', 'D'})
        self.assertEqual(added, [(3, 'C'), (4, 'D')])
        self.assertTrue(patched.startswith(b'#\x82\xa0\r\nNum  = 4\r\n1 = "B"\r\n2 = "A"\r\n'))
        self.assertTrue(patched.endswith(b'\0\0\0\0'))
        self.assertEqual(name_table(patched)[:2], name_table(source))
        self.assertEqual(append_names(patched, {'A', 'B', 'C', 'D'}), (patched, []))

    def test_reject_bad_num_and_duplicate_id(self):
        for data in (b'Num=2\n1="A"\n', b'Num=2\n1="A"\n1="B"\n'):
            with self.assertRaises(ValueError):
                append_names(data, {'C'})

    def test_install_restore_and_later_edit_guard_with_disposable_files(self):
        shell = shutil.which('pwsh')
        self.assertIsNotNone(shell, 'pwsh is needed to verify the actual installer')
        script = Path(__file__).with_name('Manage-C0010Behavior.ps1')
        names = ['action/eventnameid.txt','action/statenameid.txt','action/variablenameid.txt',
                 'action/script/d0010.hks','chr/c0010.behbnd.dcx']
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); ga = root/'GA'; package = root/'package'; record = root/'deployment.json'
            files = []
            for name in names:
                dest, src = ga/name, package/name
                src.parent.mkdir(parents=True, exist_ok=True)
                src.write_bytes(('after '+name).encode())
                old = None
                if name != 'action/script/d0010.hks':
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    dest.write_bytes(('before '+name).encode())
                    old = digest(dest.read_bytes())
                files.append({'relative':name,'before':old,'after':digest(src.read_bytes())})
            protected = {}
            for name in ['action/script/c9997.hks','action/script/c0010.hks','chr/c0010.anibnd.dcx','regulation.bin']:
                path = ga/name; path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(b'protected')
                protected[name] = digest(path.read_bytes())
            (package/'behavior-build.json').write_text(json.dumps({'files':files,'protected_inputs':protected}))
            def run(mode):
                return subprocess.run([shell,'-NoProfile','-NonInteractive','-File',str(script),'-Mode',mode,
                    '-Package',str(package),'-GameAssets',str(ga),'-Record',str(record)],capture_output=True,text=True,encoding='utf-8')
            result = run('Install'); self.assertEqual(result.returncode,0,result.stderr)
            for file in files: self.assertEqual(digest((ga/file['relative']).read_bytes()),file['after'])
            changed = ga/names[0]; expected = changed.read_bytes(); changed.write_bytes(b'user later edit')
            result = run('Restore'); self.assertNotEqual(result.returncode,0)
            self.assertIn('Refusing to replace later edits', result.stderr)
            self.assertTrue((ga/'action/script/d0010.hks').exists(), 'restore must preflight all files before mutation')
            changed.write_bytes(expected)
            result = run('Restore'); self.assertEqual(result.returncode,0,result.stderr)
            for file in files:
                path = ga/file['relative']
                if file['before'] is None: self.assertFalse(path.exists())
                else: self.assertEqual(digest(path.read_bytes()),file['before'])
            for name, sha in protected.items(): self.assertEqual(digest((ga/name).read_bytes()),sha)


if __name__ == '__main__': unittest.main()
