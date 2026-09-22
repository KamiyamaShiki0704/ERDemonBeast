"""Read existing crash dumps offline; stack candidates are NOT unwound frames."""
import argparse
import json
from pathlib import Path
import struct
from minidump.minidumpfile import MinidumpFile
from minidump.streams.ContextStream import CONTEXT
import capstone


def audit(path):
    dump = MinidumpFile.parse(str(path))
    exception = dump.exception.exception_records[0]
    dump.file_handle.seek(exception.ThreadContext.Rva)
    context = CONTEXT.parse(dump.file_handle)
    reader = dump.get_reader()

    def module(address):
        for item in dump.modules.modules:
            if item.baseaddress <= address < item.endaddress:
                return f"{Path(item.name).name}+0x{address-item.baseaddress:x}"
        return hex(address)

    instructions = []
    try:
        code = reader.read(context.Rip, 48)
        disassembler = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
        instructions = [f"{module(i.address)}: {i.mnemonic} {i.op_str}" for i in disassembler.disasm(code, context.Rip)]
    except Exception as error:
        instructions = [str(error)]
    candidates = []
    for offset in range(0, 4096, 8):
        try:
            address = struct.unpack("<Q", reader.read(context.Rsp+offset, 8))[0]
        except Exception:
            break
        label = module(address)
        if "+0x" in label:
            candidates.append(dict(stack_offset=hex(offset), candidate=label))
    return dict(dump=path.name, thread_id=exception.ThreadId,
                exception=str(exception.ExceptionRecord.ExceptionCode),
                exception_information=exception.ExceptionRecord.ExceptionInformation,
                exception_site=module(exception.ExceptionRecord.ExceptionAddress),
                registers={key:hex(getattr(context, key)) for key in
                           ["Rip", "Rsp", "Rbp", "Rax", "Rbx", "Rcx", "Rdx", "Rsi", "Rdi", "R8", "R9", "R10", "R11", "R12", "R13", "R14", "R15"]},
                instructions=instructions,
                stack_candidates=candidates[:80],
                note="Stack candidates are raw words matching loaded modules, not a validated call stack.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("dump", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    data = json.dumps(audit(args.dump), indent=2)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(data, encoding="utf-8")
    print(data)
