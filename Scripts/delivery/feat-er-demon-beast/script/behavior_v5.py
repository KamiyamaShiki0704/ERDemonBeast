"""Translate two NR 1011 notifications into existing graph transition events.

Only the twelve clip trigger pointers and two added trigger arrays may differ.
Anibnd, graph names, all states/transitions, and existing triggers stay intact.
"""
import argparse
import struct
from collections import deque
from pathlib import Path
from xml.etree import ElementTree as ET
from behavior_v3 import normalized

NOTIFICATIONS = {'Event20010_CMSG': (17/30, 'W_Idle'),
                 'Attack3035_CMSG': (110/30, 'W_Event20010')}


def patch(source, target):
    tree = ET.parse(source)
    root = tree.getroot()
    types = {t.find('name').get('value'): t.get('id') for t in root.findall('type')
             if t.find('name') is not None}
    objects = {o.get('id'): o for o in root.findall('object')}
    events = [n.get('value') for n in root.findall(".//field[@name='eventNames']/array/string")]
    next_id = max(int(k.removeprefix('object')) for k in objects)+1
    for name, (time, event) in NOTIFICATIONS.items():
        matches = [o for o in objects.values()
                   if o.find(f"record/field[@name='name']/string[@value='{name}']") is not None]
        assert len(matches) == 1, name
        clips = matches[0].findall("record/field[@name='generators']/array/pointer")
        assert len(clips) == 6, name
        obj_id = f'object{next_id}'
        next_id += 1
        obj = ET.SubElement(root, 'object', id=obj_id, typeid=types['hkbClipTriggerArray'])
        record = ET.SubElement(obj, 'record')
        bag = ET.SubElement(record, 'field', name='propertyBag')
        ET.SubElement(bag, 'array', count='0', elementtypeid='type15')
        field = ET.SubElement(record, 'field', name='triggers')
        array = ET.SubElement(field, 'array', count='1', elementtypeid=types['hkbClipTrigger'])
        trigger = ET.SubElement(array, 'record')
        time = struct.unpack('<f', struct.pack('<f', time))[0]  # actual TAE float32
        f = ET.SubElement(trigger, 'field', name='localTime')
        ET.SubElement(f, 'real', dec=str(time), hex='#'+struct.pack('>d',time).hex())
        f = ET.SubElement(trigger, 'field', name='event')
        e = ET.SubElement(f, 'record')
        ET.SubElement(ET.SubElement(e,'field',name='id'),'integer',value=str(events.index(event)))
        ET.SubElement(ET.SubElement(e,'field',name='payload'),'pointer',id='object0')
        for key, value in [('relativeToEndOfClip','false'),('acyclic','true'),('isAnnotation','false')]:
            ET.SubElement(ET.SubElement(trigger,'field',name=key),'bool',value=value)
        for p in clips:
            clip = objects[p.get('id')]
            ptr = clip.find("record/field[@name='triggers']/pointer")
            assert ptr.get('id') == 'object0', 'must not overwrite existing triggers'
            ptr.set('id', obj_id)
    tree.write(target, encoding='utf-8', xml_declaration=True)


def equal_graph(a, b):
    """Full pointer-preserving graph isomorphism, including shared references."""
    assert [normalized(t) for t in a.findall('type')] == [normalized(t) for t in b.findall('type')]
    objects_a = {o.get('id'): o for o in a.findall('object')}
    objects_b = {o.get('id'): o for o in b.findall('object')}
    assert len(objects_a) == len(objects_b)
    forward, reverse = {'object0':'object0'}, {'object0':'object0'}
    queue = deque()

    def link(x, y):
        if x in forward:
            assert forward[x] == y and reverse[y] == x, (x,y,'alias changed')
        else:
            assert y not in reverse
            forward[x], reverse[y] = y, x
            queue.append((x,y))

    def compare(x, y):
        assert x.tag == y.tag
        if x.tag == 'pointer':
            link(x.get('id'), y.get('id'))
        elif x.tag == 'real':
            assert int(x.get('hex')[1:],16) == int(y.get('hex')[1:],16)
        else:
            aa, bb = dict(x.attrib), dict(y.attrib)
            if x.tag == 'object': aa.pop('id'); bb.pop('id')
            assert aa == bb, (x.tag,aa,bb)
        assert (x.text or '').strip() == (y.text or '').strip()
        assert len(x) == len(y)
        for xx, yy in zip(x,y): compare(xx,yy)

    link('object1','object1')
    while queue:
        x,y = queue.popleft()
        compare(objects_a[x],objects_b[y])
    assert len(forward)-1 == len(objects_a), 'unreachable objects require explicit audit'


def verify(source, expected, roundtrip):
    patch(source, expected.with_suffix('.recomputed.xml'))
    a = ET.parse(expected).getroot()
    b = ET.parse(roundtrip).getroot()
    equal_graph(a,b)
    assert normalized(a) == normalized(ET.parse(expected.with_suffix('.recomputed.xml')).getroot())
    print('PASS: two 1011 clip notifications, 12 pointers, complete graph roundtrip')


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('mode', choices=['patch','verify'])
    p.add_argument('paths', nargs='+', type=Path)
    a = p.parse_args()
    (patch if a.mode=='patch' else verify)(*a.paths)
