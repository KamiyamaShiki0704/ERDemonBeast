"""Split c0010's private common copy at the engine luaFiles seam (offline)."""
import argparse
import copy
import re
from pathlib import Path
from xml.etree import ElementTree as ET

from behavior_v3 import normalized
from behavior_v5 import equal_graph

OLD = ['d0010.hks', 'modifier.hks', 'c0010.hks', 'c0010.hks']
ORDER = ['c9997.hks', 'd0010.hks', 'modifier.hks', 'c0010.hks', 'c0010.hks']


def change_order(root, before, after):
    arrays = root.findall(".//field[@name='luaFiles']/array")
    assert len(arrays) == 1, 'Unexpected character script topology'
    array = arrays[0]
    assert [n.get('value') for n in array] == before
    assert array.get('count') == str(len(before))
    for n in list(array):
        array.remove(n)
    array.set('count', str(len(after)))
    for name in after:
        ET.SubElement(array, 'string', value=name)


def safe_common(data):
    # Preserve encoding, BOM and all other bytes; this unused local cannot affect gameplay.
    assert len(re.findall(rb'\bscript_dir\b', data)) == 1, 'Path local now has consumers; review needed'
    pattern = rb'(?m)^local script_dir = debug\.getinfo\(1, "S"\)\.source:[^\r\n]*'
    result, count = re.subn(pattern, b'-- Unused path initialization removed: engine chunk names may have no directory.', data)
    assert count == 1, 'Shared script changed; review path initialization'
    return result


def prepare(source, common, adapter, out):
    marker = b'ERDB_BEHAVIOR_VERSION = 9'
    baseline = common.with_name('d0010.hks').read_bytes().replace(b'\r\n', b'\n')
    separate = adapter.read_bytes().replace(b'\r\n', b'\n')
    assert baseline.count(marker) == separate.count(marker) == 1
    assert baseline[baseline.index(marker):] == separate[separate.index(marker):], 'Adapter executable body changed'
    tree = ET.parse(source)
    before = copy.deepcopy(tree.getroot())
    change_order(tree.getroot(), OLD, ORDER)
    undone = copy.deepcopy(tree.getroot())
    change_order(undone, ORDER, OLD)
    assert normalized(undone) == normalized(before), 'Unintended character changes'
    tree.write(out / 'character-split.xml', encoding='utf-8', xml_declaration=True)
    scripts = out / 'action/script'
    scripts.mkdir(parents=True)
    (scripts / 'c9997.hks').write_bytes(safe_common(common.read_bytes()))
    (scripts / 'd0010.hks').write_bytes(adapter.read_bytes())


def verify(source, candidate, roundtrip):
    original = ET.parse(source).getroot()
    expected = ET.parse(candidate).getroot()
    restored = copy.deepcopy(expected)
    change_order(restored, ORDER, OLD)
    assert normalized(restored) == normalized(original)
    equal_graph(expected, ET.parse(roundtrip).getroot())
    print('PASS: complete character graph roundtrip; only luaFiles changed')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['prepare', 'verify'])
    parser.add_argument('paths', nargs='+', type=Path)
    args = parser.parse_args()
    (prepare if args.mode == 'prepare' else verify)(*args.paths)
