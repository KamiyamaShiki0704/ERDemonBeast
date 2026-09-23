"""Summarize observed FrameEnd input/action signals; never infer gameplay PASS."""
import argparse
import json
from pathlib import Path


def analyze(rows):
    active = [r for r in rows if r.get("kind") == "frame" and r.get("phase") == "active"]
    traced = [r for r in active if r.get("action_trace")]
    keys = ("mouse_left", "mouse_right", "shift", "ctrl", "space")
    summary = {"scope": "FrameEnd observations only; no gameplay verdict",
               "status": "OBSERVED" if traced else "INSUFFICIENT_TRACE",
               "active_frames": len(active), "traced_frames": len(traced),
               "dropped": max((r.get("dropped", 0) for r in rows), default=0),
               "physical_held_frames": {k: sum(bool((r.get("physical_input") or {}).get(k)) for r in traced) for k in keys}}
    actors = {}
    for role in ("body", "beast"):
        samples = [r.get(side, {}).get(role) for r in active for side in ("pre", "post")]
        samples = [s for s in samples if s]
        actors[role] = {k: sorted({s[k] for s in samples if k in s}) for k in (
            "actions", "new_actions", "readback_new_actions", "npc_action",
            "readback_npc_action", "tae", "disabled_actions", "possible_inputs", "possible_cancels")}
    summary["actors"] = actors
    summary["limitations"] = "No physical key implies no conclusion about remapped keys; a cleared FrameEnd request does not prove it was never consumed earlier. TAE ID is the request queue state, not full animation playback."
    return summary


def report(path):
    rows, errors = [], []
    for number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        if not line.strip():
            continue
        try:
            rows.append(json.loads(line))
        except ValueError:
            errors.append(f"invalid/truncated JSON at line {number}")
    result = analyze(rows)
    result['errors'] = errors
    if errors or result['dropped']:
        result['status'] = 'INCOMPLETE_TRACE'
    path.with_name("action-report.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("frames", type=Path)
    print(json.dumps(report(parser.parse_args().frames), indent=2))
