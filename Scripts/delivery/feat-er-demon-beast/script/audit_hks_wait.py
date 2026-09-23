"""Read-only, hash-pinned ER HKS input-dispatch evidence (requires pefile/capstone)."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import pefile
import capstone
from check_target import EXPECTED_HASH


def audit(path):
    data = path.read_bytes()
    assert hashlib.sha256(data).hexdigest() == EXPECTED_HASH, 'unsupported EXE'
    pe = pefile.PE(data=data)
    def branch(byte_table, jump_table, index):
        slot = pe.get_data(byte_table + index, 1)[0]
        return struct.unpack('<I', pe.get_data(jump_table + slot * 4, 4))[0]
    assert branch(0x40E900, 0x40E8DC, 9100 - 9001) == 0x40E5E2
    assert branch(0x413ABC, 0x413A58, 1106 - 1001) == 0x4130F9
    assert branch(0x413ABC, 0x413A58, 1107 - 1001) == 0x41312E
    blocks = {'act9100': (0x40E5E2, 0x17), 'env1106': (0x4130F9, 0x35),
              'action_request': (0x407930, 0x76), 'action_cancel': (0x407EC0, 0x15)}
    decoder = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    result = {'exe_sha256': EXPECTED_HASH, 'scope': 'static executable, no game/process access', 'blocks': {}}
    for name, (rva, size) in blocks.items():
        code = pe.get_data(rva, size)
        result['blocks'][name] = {'rva': hex(rva), 'bytes': code.hex(), 'instructions': [
            f'{i.address:#x}: {i.mnemonic} {i.op_str}' for i in decoder.disasm(code, pe.OPTIONAL_HEADER.ImageBase + rva)]}
    result['finding'] = ('act9100 sets action_flag+0x10 bit0. env1106 calls ActionRequest with mode=1; '
                         'waiting selects action_request+0x20 (new presses), otherwise queue-mode or +0x30 '
                         '(queued inputs). env1107 reads +0x28 (cancel-ready); Wait does not bypass it.')
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('exe', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    result = audit(args.exe)
    args.output.write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(result['finding'])
