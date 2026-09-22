"""Read-only target verification; this is NOT a runtime Recorder Probe.

Topic: FEAT-ER-demon-beast
Topic-Path: docs/feat-er-demon-beast
Spec: spec.md@1.0
Script-Version: 1.0
Verifies: prerequisite for SPEC-ERDB-001..006, not gameplay results
Project-Relative-Path: Scripts/delivery/feat-er-demon-beast/script/check_target.py
"""
import argparse
import hashlib
import json
import struct
from datetime import datetime, timezone
from pathlib import Path

EXPECTED_HASH = "1a3547101327f65d0c76da2f9190ac0aa66871ea42bae2aecc61e11a8b597891"
CHECKS = {
    "ctrl_override_getter": (0x3C6F50, "488b81b00300004885c07504488b4118c3"),
    "ctrl_override_dispatch": (0x3C726C, "488b89b00300004885c97504488b4818488b01ff90d0000000"),
    "camera_override_read": (0x5AF943, "488b81b8000000"),
}


def rva_bytes(data, rva, size):
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("invalid PE signature")
    count = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    sections = pe + 24 + optional_size
    for index in range(count):
        header = sections + 40 * index
        _, va, raw_size, raw = struct.unpack_from("<4I", data, header + 8)
        if va <= rva and rva + size <= va + raw_size:
            offset = raw + rva - va
            result = data[offset:offset + size]
            if len(result) == size:
                return result
    raise ValueError(f"RVA not backed by file: {rva:#x}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("exe", type=Path)
    parser.add_argument("--output", type=Path, help="Optional static JSON evidence path")
    args = parser.parse_args()
    data = args.exe.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    checks = []
    for name, (rva, expected) in CHECKS.items():
        actual = rva_bytes(data, rva, len(bytes.fromhex(expected))).hex()
        checks.append({"name": name, "rva": hex(rva), "expected": expected,
                       "actual": actual, "passed": actual == expected})
    supported = digest == EXPECTED_HASH and all(c["passed"] for c in checks)
    result = {"kind": "static-target-check", "topic": "FEAT-ER-demon-beast",
              "spec_version": "1.0", "script_version": "1.0",
              "executed_at": datetime.now(timezone.utc).isoformat(),
              "exe": str(args.exe.resolve()), "sha256": digest,
              "supported": supported, "runtime_tested": False, "checks": checks}
    output = json.dumps(result, indent=2)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output + "\n", encoding="utf-8")
    print(output)
    raise SystemExit(0 if supported else 1)


if __name__ == "__main__":
    main()
