"""Bounded upgrade of the installed v2 graph. Offline project copies only."""
import argparse
import json
import re
from fractions import Fraction
from pathlib import Path
from xml.etree import ElementTree as ET
from behavior_assets import digest, private_script


def normalized(node):
    return (node.tag, sorted(node.attrib.items()), (node.text or '').strip(),
            [normalized(child) for child in node])


def patch(source, target):
    root = ET.parse(source)
    changed = []
    for record in root.findall('.//object/record'):
        name = record.find("field[@name='name']/string")
        if name is not None and name.get('value') in ('Event20000_CMSG', 'Event20010_CMSG'):
            field = record.find("field[@name='enableTae']/bool")
            assert field.get('value') == 'true'
            field.set('value', 'false')
            changed.append(name.get('value'))
    assert sorted(changed) == ['Event20000_CMSG', 'Event20010_CMSG']
    root.write(target, encoding='utf-8', xml_declaration=True)


def finish(output, ga, adapter, baseline):
    original = ET.parse(output/'graph-original.xml').getroot()
    patched = ET.parse(output/'graph-patched.xml').getroot()
    roundtrip = ET.parse(output/'graph-roundtrip.xml').getroot()
    assert normalized(patched) == normalized(roundtrip), 'HKX roundtrip semantics changed'
    # Recompute expected two-field patch independently of the roundtrip file.
    patch(output/'graph-original.xml', output/'graph-expected.xml')
    assert normalized(patched) == normalized(ET.parse(output/'graph-expected.xml').getroot())
    assert len(original.findall('object')) == len(roundtrip.findall('object'))
    report = json.loads(baseline.read_text(encoding='utf-8-sig'))
    assert report['version'] == 2
    for file in report['files']:
        assert digest((ga/file['relative']).read_bytes()) == file['after'], file['relative']
    for name, sha in report['protected_inputs'].items():
        assert digest((ga/name).read_bytes()) == sha, name
    # Validate each adapter lower bound against newly extracted TAE flags.
    windows = json.loads((output/'tae-windows.json').read_text(encoding='utf-8-sig'))
    text = adapter.read_text()
    rows = dict(re.findall(r'^    \[(\d+)\]=\{([^}]*)\}', text.split('local function erdb_window_open')[0], re.M))
    flags = {0:115, 1:116, 2:16, 3:103, 5:26, 35:11}
    for item in windows:
        values = {int(k):float(Fraction(v)) for k,v in re.findall(r'\[(\d+)\]=([\d./]+)',rows[str(item['id'])])}
        expected = {key:min(w['start'] for w in item['windows'] if w['flag']==flag)
                    for key,flag in flags.items() if any(w['flag']==flag for w in item['windows'])}
        assert values.keys()==expected.keys(), item['id']
        assert all(abs(values[k]-expected[k])<1e-6 for k in values), item['id']
    private = output/'action/script/d0010.hks'
    private.parent.mkdir(parents=True,exist_ok=True)
    private.write_bytes(private_script((ga/'action/script/c9997.hks').read_bytes(),adapter.read_bytes()))
    for file in report['files']:
        relative=file['relative']
        file['before']=file['after']
        target=output/relative
        if relative.startswith('action/') and relative.endswith('nameid.txt'):
            target.parent.mkdir(parents=True,exist_ok=True)
            target.write_bytes((ga/relative).read_bytes())
        file['after']=digest(target.read_bytes())
    report.update(version=3, adapter_sha256=digest(adapter.read_bytes()),
                  scope='c0010 lock direction, owned-action windows, intro/outro handshake; DLL 0.3.0 required',
                  graph_patch=['Event20000_CMSG.enableTae=false','Event20010_CMSG.enableTae=false'],
                  roundtrip='PASS: complete graph XML semantic equality; other three binder entries byte-identical',
                  tae_windows=windows, runtime='NOT_RUN; human acceptance required')
    report['name_tables']={k:{**v,'additions':[]} for k,v in report['name_tables'].items()}
    (output/'behavior-build.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print('PASS: graph roundtrip, v2 input hashes, protected inputs, all 10 TAE window tables, private script')


if __name__ == '__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('mode',choices=['patch','finish'])
    p.add_argument('paths',nargs='+',type=Path)
    args=p.parse_args()
    (patch if args.mode=='patch' else finish)(*args.paths)
