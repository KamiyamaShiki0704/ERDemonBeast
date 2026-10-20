"""Exercise actual three-file PowerShell transactions on disposable roots."""
import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name('Manage-SplitBehavior.ps1')
NAMES = ['chr/c0010.behbnd.dcx','action/script/c9997.hks','action/script/d0010.hks']
PROTECTED = ['regulation.bin','chr/c0010.anibnd.dcx','action/script/c0010.hks',
             'action/eventnameid.txt','action/statenameid.txt','action/variablenameid.txt']


def put(path, contents):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(contents)
    return hashlib.sha256(contents).hexdigest()


class SplitInstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='erdb-split-')
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.ga, self.dll, self.package = root/'ga', root/'dll', root/'package'
        self.record = self.dll/'deployment-split-hks-1.json'
        self.files = [dict(relative=n, before=put(self.ga/n, b'before:'+n.encode()),
                           after=put(self.package/n, b'after:'+n.encode())) for n in NAMES]
        manifest = dict(result='PASS_ASSET_STRUCTURE', files=self.files,
                        protected=[dict(relative=n,hash=put(self.ga/n,b'protected')) for n in PROTECTED],
                        dll_protected=[dict(relative=n,hash=put(self.dll/n,b'native')) for n in ['er_demon_beast.dll','er-demon-beast.toml']])
        put(self.package/'split-build.json', json.dumps(manifest).encode())
        put(self.package/'split-verification.json', json.dumps(dict(result='PASS_OFFLINE', files={f['relative']:f['after'] for f in self.files})).encode())

    def run_tool(self, mode, passes=True):
        result = subprocess.run(['pwsh','-NoProfile','-File',str(SCRIPT),'-Mode',mode,
                                 '-Package',str(self.package),'-GaRoot',str(self.ga),'-DllRoot',str(self.dll)],
                                capture_output=True,text=True,encoding='utf-8')
        self.assertEqual(result.returncode == 0, passes, result.stdout+result.stderr)
        return result.stdout+result.stderr

    def assert_set(self, prefix):
        for n in NAMES:
            self.assertEqual((self.ga/n).read_bytes(), prefix+n.encode())

    def test_install_restore_preserves_all_files_and_guards(self):
        self.run_tool('Install'); self.assert_set(b'after:')
        data = json.loads(self.record.read_text(encoding='utf-8-sig'))
        for n in NAMES:
            self.assertEqual((Path(data['backup'])/n).read_bytes(),b'before:'+n.encode())
        self.run_tool('Restore'); self.assert_set(b'before:')
        for n in PROTECTED:
            self.assertEqual((self.ga/n).read_bytes(), b'protected')
        self.assertEqual(json.loads(self.record.read_text(encoding='utf-8-sig'))['status'],'restored')

    def test_later_common_edit_blocks_entire_restore(self):
        self.run_tool('Install')
        put(self.ga/NAMES[1],b'user update')
        self.assertIn('Later edit',self.run_tool('Restore',False))
        self.assertEqual((self.ga/NAMES[0]).read_bytes(),b'after:'+NAMES[0].encode())
        self.assertEqual((self.ga/NAMES[1]).read_bytes(),b'user update')

    def test_corrupt_backup_blocks_entire_restore(self):
        self.run_tool('Install')
        data=json.loads(self.record.read_text(encoding='utf-8-sig'))
        put(Path(data['backup'])/NAMES[2],b'corrupt')
        self.assertIn('Backup hash',self.run_tool('Restore',False))
        self.assert_set(b'after:')

    def test_stale_input_blocks_install(self):
        put(self.ga/NAMES[2],b'user update')
        self.assertIn('Installed source changed',self.run_tool('Install',False))
        self.assertFalse(self.record.exists())
        self.assertEqual((self.ga/NAMES[0]).read_bytes(),b'before:'+NAMES[0].encode())

    def test_changed_protected_file_blocks_install(self):
        put(self.ga/'regulation.bin',b'user update')
        self.assertIn('Protected GA file changed',self.run_tool('Install',False))
        self.assert_set(b'before:')

    def test_unverified_package_edit_blocks_install(self):
        put(self.package/NAMES[2],b'untested')
        self.assertIn('Package or verification hash',self.run_tool('Install',False))
        self.assert_set(b'before:')


if __name__ == '__main__':
    unittest.main()
