"""Check recorded restoration deadlines; does not launch or access the game."""
import argparse
import hashlib
import json
from pathlib import Path
import re


def analyze(text):
    cycles = []
    current = None
    for number, line in enumerate(text.splitlines(), 1):
        match = re.search(r"t=([\d.]+).* -> Restoring", line)
        if match:
            current = {"start": float(match[1]), "line": number,
                       "unknown_observations": 0, "destroyed": False,
                       "timeout": False, "status": "INCONCLUSIVE"}
            cycles.append(current)
        if current is None:
            continue
        current["unknown_observations"] += int("references=None" in line)
        current["destroyed"] |= "outcome=destroyed" in line
        current["timeout"] |= "restore_timeout" in line
        if current["timeout"]:
            current["status"] = "FAIL"
        match = re.search(r"t=([\d.]+).* -> Normal", line)
        if match:
            current["seconds"] = round(float(match[1]) - current["start"], 3)
            current["status"] = ("PASS" if current["seconds"] <= 3
                                 and current["destroyed"] and not current["timeout"] else "FAIL")
            current = None
    return cycles


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    records = []
    for path in sorted(args.root.rglob("er-demon-beast.log")):
        data = path.read_bytes()
        records.append({"log": str(path), "sha256": hashlib.sha256(data).hexdigest(),
                        "cycles": analyze(data.decode("utf-8", errors="replace"))})
    result = {"scope": "recorded restoration deadline and destructor evidence only; not full Spec acceptance",
              "runs": records}
    rendered = json.dumps(result, indent=2, ensure_ascii=False)
    if args.output:
        args.output.write_text(rendered + "\n", encoding="utf-8")
    print(rendered)
    return int(any(c["status"] == "FAIL" for r in records for c in r["cycles"]))


if __name__ == "__main__":
    raise SystemExit(main())
