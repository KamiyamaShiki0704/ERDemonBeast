import hashlib
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
import zipfile
from manual_test import make_profile, prepare, collect


class ManualPreparation(unittest.TestCase):
    def test_collect_includes_direct_loader_trace_but_never_save_or_dll(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            trace = root / 'Saved/er-demon-beast/direct/frames.jsonl'
            trace.parent.mkdir(parents=True)
            trace.write_text('')
            trace.with_name('meta.json').write_text(json.dumps({'run':'direct', 'dll_sha256':'fixture'}))
            (root / 'er-demon-beast.log').write_text('fixture')
            (trace.parent / 'ER0000.Shiki').write_bytes(b'private-save')
            (root / 'er_demon_beast.dll').write_bytes(b'dll')
            result = collect(root)
            with zipfile.ZipFile(result['log_archive']) as archive:
                names = archive.namelist()
            self.assertIn('Saved/er-demon-beast/direct/frames.jsonl', names)
            self.assertIn('Saved/er-demon-beast/direct/action-report.json', names)
            self.assertIn('er-demon-beast.log', names)
            self.assertFalse(any(n.endswith(('.Shiki', '.dll')) for n in names))

    def test_profiles_are_offline_and_never_reuse_summon_or_duplicate_beast(self):
        base = {"start_online":True, "natives":[{"path":"dll/Summon/Summon.dll"},
                {"path":"dll/er_demon_beast.dll"}, {"path":"dll/hud.dll"}],
                "packages":[{"id":"GA","path":"GA"}]}
        root = Path.cwd()
        full = make_profile(base, root, root/'new.dll', True)
        self.assertFalse(full['start_online'])
        self.assertEqual(full['savefile'], 'ER0000.Shiki')
        self.assertEqual([Path(e['path']).name for e in full['natives']], ['hud.dll','new.dll'])
        self.assertEqual(len(make_profile(base,root,root/'new.dll',False)['natives']),1)
        self.assertTrue(base['start_online'])

    def test_preparation_preserves_original_profile_and_save_and_sets_modes(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp); ga=root/'ga'; ga.mkdir(); (ga/'GA').mkdir()
            profile=b'profileVersion="v1"\nstart_online=true\npackages=[{id="GA",path="GA"}]\n'
            (ga/'Start_GoldenAge.me3').write_bytes(profile)
            (root/'er_demon_beast.dll').write_bytes(b'fixture')
            (root/'build-result.json').write_text(json.dumps({'dll_sha256':hashlib.sha256(b'fixture').hexdigest()}))
            (root/'er-demon-beast.toml').write_text('enabled=false\ncontrol_enabled=false\n')
            save=root/'saves/account'; save.mkdir(parents=True); (save/'ER0000.Shiki').write_bytes(b'original-save')
            for mode in ['load','observe','control','missing']:
                info=prepare(root,ga,mode,False,root/'saves')
                config=tomllib.loads((Path(info['run'])/'er-demon-beast.toml').read_text())
                self.assertEqual(config['enabled'],mode!='load')
                self.assertEqual(config['control_enabled'],mode in ('control','missing'))
                if mode=='missing': self.assertEqual(config['npc_param_id'],2147483647)
                self.assertEqual((Path(info['save_backup'])/'ER0000.Shiki').read_bytes(),b'original-save')
                self.assertFalse(tomllib.loads(Path(info['profile']).read_text())['start_online'])
            self.assertEqual((ga/'Start_GoldenAge.me3').read_bytes(),profile)
            self.assertEqual((save/'ER0000.Shiki').read_bytes(),b'original-save')

    def test_multiform_nested_config_survives_manual_launcher(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp); ga=root/'ga';ga.mkdir();(ga/'GA').mkdir()
            (ga/'Start_GoldenAge.me3').write_text('profileVersion="v1"\npackages=[{id="GA",path="GA"}]\n')
            (root/'er_demon_beast.dll').write_bytes(b'fixture')
            (root/'build-result.json').write_text(json.dumps({'dll_sha256':hashlib.sha256(b'fixture').hexdigest()}))
            config='enabled=false\ncontrol_enabled=false\ndefault_form="a"\n[[forms]]\nkey="a"\nnpc_param_id=100000\n[forms.inputs]\nlight=1\nheavy=0\n[[forms.combos]]\nfrom=3000\nlight=3001\nheavy=3005\n'
            (root/'er-demon-beast.toml').write_text(config)
            save=root/'saves/account';save.mkdir(parents=True);(save/'ER0000.Shiki').write_bytes(b'save')
            for mode in ['control','missing']:
                info=prepare(root,ga,mode,False,root/'saves')
                parsed=tomllib.loads((Path(info['run'])/'er-demon-beast.toml').read_text())
                self.assertEqual(parsed['forms'][0]['inputs'],{'light':1,'heavy':0})
                self.assertEqual(parsed['forms'][0]['combos'][0]['light'],3001)
                self.assertEqual(parsed['forms'][0]['npc_param_id'],2147483647 if mode=='missing' else 100000)
                self.assertNotIn('npc_param_id',parsed)


if __name__=='__main__': unittest.main()
