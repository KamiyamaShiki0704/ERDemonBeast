"""Offline c0010 graph/name-table audit and deterministic private-script preparation."""
import argparse
import hashlib
import json
from pathlib import Path
import re
from xml.etree import ElementTree as ET

ATTACKS = (3000, 3001, 3002, 3003, 3005, 3007, 3010, 3030, 3035, 6000)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def private_script(common, adapter):
    # GA has one unused local whose path calculation throws on a bare HKS
    # chunk name or Windows backslashes, before the appended adapter executes.
    # Remove only that statement from the private copy; never edit GA c9997.
    lines = common.removeprefix(b'\xef\xbb\xbf').splitlines(keepends=True)
    matches = [i for i, line in enumerate(lines) if line.startswith(b'local script_dir = debug.getinfo(1, "S").source:')]
    if len(matches) != 1 or len(re.findall(rb'\bscript_dir\b', common)) != 1:
        raise ValueError('unexpected GA script_dir declaration/use; review private script preparation')
    lines[matches[0]] = b'-- ERDB: omitted unused script_dir calculation (bare/backslash chunk names).\n'
    return b''.join(lines) + b'\n\n-- ERDB private c0010 adapter follows.\n' + adapter


def name_table(data):
    pairs = [(int(i), n.decode('ascii')) for i, n in re.findall(rb'(?m)^\s*(\d+)\s*=\s*"([^"\r\n]+)"', data)]
    declared = int(re.search(rb'Num\s*=\s*(\d+)', data)[1])
    if declared != len(pairs) or [i for i, _ in pairs] != list(range(1, declared + 1)):
        raise ValueError('name table must have contiguous existing IDs and matching Num')
    # Existing aliases by name are retained; a name only needs to be present once.
    return pairs


def append_names(data, required):
    before = name_table(data)
    missing = sorted(set(required) - {n for _, n in before})
    if not missing:
        return data, []
    updated = re.sub(rb'(Num\s*=\s*)\d+', lambda m: m[1] + str(len(before) + len(missing)).encode(), data, count=1)
    # Preserve the existing encoding, comments and any EOF NUL sentinel.
    core = updated.rstrip(b'\0')
    sentinel = updated[len(core):]
    newline = b'\r\n' if b'\r\n' in core else b'\n'
    if not core.endswith(b'\n'):
        core += newline
    additions = [(len(before) + 1 + i, n) for i, n in enumerate(missing)]
    for i, name in additions:
        core += str(i).encode() + b' = "' + name.encode('ascii') + b'"' + newline
    result = core + sentinel
    after = name_table(result)
    assert after[:len(before)] == before and len(after) == len(before) + len(missing)
    assert set(required) <= {n for _, n in after}
    return result, additions


def field(record, name, tag):
    node = record.find(f"field[@name='{name}']/{tag}")
    return node.attrib.get('value', node.attrib.get('id')) if node is not None else None


def audit_graphs(paths):
    required = {k: set() for k in ('event', 'state', 'variable')}
    links, selectors, lua_files = {}, {}, []
    for path in paths:
        root = ET.parse(path).getroot()
        types = {t.attrib['id']: t.find('name').attrib['value'] for t in root.findall('type')}
        objects = {o.attrib['id']: o for o in root.findall('object')}
        events = []
        for fname, kind in [('eventNames', 'event'), ('variableNames', 'variable')]:
            names = [n.attrib['value'] for n in root.findall(f".//field[@name='{fname}']/array/string")]
            required[kind].update(names)
            if kind == 'event':
                events = names
        lua_files.extend(n.attrib['value'] for n in root.findall(".//field[@name='luaFiles']/array/string"))
        for obj in objects.values():
            typ, rec = types.get(obj.attrib['typeid']), obj.find('record')
            if typ == 'hkbStateMachine::StateInfo':
                required['state'].add(field(rec, 'name', 'string'))
            if typ == 'CustomManualSelectorGenerator':
                name = field(rec, 'name', 'string')
                selectors[name] = {'animation': field(rec, 'animId', 'integer'),
                                   'script': field(rec, 'enableScript', 'bool'),
                                   'tae': field(rec, 'enableTae', 'bool'),
                                   'end': field(rec, 'animeEndEventType', 'integer')}
            if typ != 'hkbStateMachine':
                continue
            state_names = {}
            for p in rec.findall("field[@name='states']/array/pointer"):
                state = objects[p.attrib['id']].find('record')
                state_names[int(field(state, 'stateId', 'integer'))] = field(state, 'name', 'string')
            wildcard = objects.get(field(rec, 'wildcardTransitions', 'pointer'))
            if wildcard is None:
                continue
            for tr in wildcard.findall("record/field[@name='transitions']/array/record"):
                event_id, target = int(field(tr, 'eventId', 'integer')), int(field(tr, 'toStateId', 'integer'))
                if 0 <= event_id < len(events):
                    links.setdefault(events[event_id], set()).add(state_names.get(target))
    checked = {}
    for animation in ATTACKS:
        name = ('Step' if animation == 6000 else 'Attack') + str(animation)
        assert name in links.get('W_' + name, set()), f'missing event -> state link: {name}'
        selector = selectors[name + '_CMSG']
        assert selector == {'animation': str(animation), 'script': 'true', 'tae': 'true', 'end': '2'}, (name, selector)
        checked[name] = selector
    assert lua_files == ['c9997.hks', 'modifier.hks', 'c0010.hks', 'c0010.hks'], lua_files
    return required, checked, lua_files


def prepare(extracted, ga, adapter, output):
    required, checked, scripts = audit_graphs(sorted(extracted.glob('*.xml')))
    report = {'version': 2, 'graph_contract': checked, 'original_lua_files': scripts, 'name_tables': {}, 'files': []}
    for kind, names in required.items():
        relative = f'action/{kind}nameid.txt'
        source = (ga / relative).read_bytes()
        patched, additions = append_names(source, names)
        dest = output / relative
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(patched)
        report['name_tables'][kind] = {'before': len(name_table(source)), 'after': len(name_table(patched)),
                                       'required': len(names), 'additions': additions}
        report['files'].append({'relative': relative, 'before': digest(source), 'after': digest(patched)})
    character = (extracted / '1.hkx').read_bytes()
    old, new = b'c9997.hks\0', b'd0010.hks\0'
    assert character.count(old) == 1 and len(old) == len(new)
    patched = character.replace(old, new)
    (output / 'character-patched.hkx').write_bytes(patched)
    report['character_patch'] = {'offset': character.index(old), 'length_preserved': len(character) == len(patched),
                                  'changed_bytes': [i for i, (a, b) in enumerate(zip(character, patched)) if a != b]}
    common = ga / 'action/script/c9997.hks'
    private = output / 'action/script/d0010.hks'
    private.parent.mkdir(parents=True, exist_ok=True)
    private.write_bytes(private_script(common.read_bytes(), adapter.read_bytes()))
    previous = ga / 'action/script/d0010.hks'
    if previous.exists():
        raise ValueError('private d0010.hks already exists; refuse an unreviewed replacement')
    report['files'].append({'relative': 'action/script/d0010.hks', 'before': None, 'after': digest(private.read_bytes())})
    report['protected_inputs'] = {str(p.relative_to(ga)).replace('\\', '/'): digest(p.read_bytes()) for p in (
        common, ga / 'action/script/c0010.hks', ga / 'chr/c0010.anibnd.dcx', ga / 'regulation.bin')}
    report['adapter_sha256'] = digest(adapter.read_bytes())
    report['scope'] = 'Ground action/ER cancel adapter; existing behavior transitions preserved; runtime pending human'
    (output / 'behavior-build.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('extracted', 'ga', 'adapter', 'output'):
        parser.add_argument(arg, type=Path)
    args = parser.parse_args()
    result = prepare(args.extracted, args.ga, args.adapter, args.output)
    print(json.dumps({k: {'before': v['before'], 'after': v['after'], 'added': len(v['additions'])}
                      for k, v in result['name_tables'].items()}))
