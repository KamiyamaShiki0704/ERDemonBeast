"""Prepare isolated manual profiles or collect logs. This script NEVER launches ER."""
import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import shutil
import tomllib
import zipfile


def toml_value(value):
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{k} = {toml_value(v)}" for k, v in value.items()) + " }"
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(v) for v in value) + "]"
    return json.dumps(value, ensure_ascii=False)


def make_profile(base, ga_root, dll, full_stack):
    natives = []
    if full_stack:
        for native in base.get("natives", []):
            entry = dict(native)
            path = Path(entry["path"])
            if path.name.lower() in ("summon.dll", "er_demon_beast.dll"):
                continue  # independent creator/retirement owners must not compete
            entry["path"] = str((ga_root / path).resolve())
            natives.append(entry)
    natives.append({"path": str(dll.resolve())})
    packages = []
    for package in base.get("packages", []):
        entry = dict(package)
        entry["path"] = str((ga_root / entry["path"]).resolve())
        packages.append(entry)
    return {"profileVersion": "v1", "savefile": "ER0000.Shiki", "start_online": False,
            "natives": natives, "supports": [{"game": "elden-ring"}], "packages": packages}


def prepare(root, ga_root, mode, full_stack, save_root):
    manifest = json.loads((root / "build-result.json").read_text(encoding="utf-8"))
    digest = hashlib.sha256((root / "er_demon_beast.dll").read_bytes()).hexdigest()
    if digest.lower() != manifest["dll_sha256"].lower():
        raise ValueError("DLL does not match build manifest")
    base = tomllib.loads((ga_root / "Start_GoldenAge.me3").read_text(encoding="utf-8-sig"))
    run = root / "manual-runs" / (datetime.now().strftime("%Y%m%d-%H%M%S-%f") + "-" + mode)
    run.mkdir(parents=True, exist_ok=False)
    for name in ["er_demon_beast.dll", "build-result.json"]:
        shutil.copy2(root / name, run / name)
    config = tomllib.loads((root / "er-demon-beast.toml").read_text(encoding="utf-8-sig"))
    config.update(enabled=mode != "load", control_enabled=mode in ("control", "missing"))
    if mode == "missing":
        config["npc_param_id"] = 2147483647
    (run / "er-demon-beast.toml").write_text("\n".join(f"{k} = {toml_value(v)}" for k, v in config.items()) + "\n", encoding="utf-8")
    profile = make_profile(base, ga_root, run / "er_demon_beast.dll", full_stack)
    for entry in profile["natives"] + profile["packages"]:
        if not Path(entry["path"]).exists():
            raise ValueError("Missing profile path: " + entry["path"])
    (run / "manual.me3").write_text("\n".join(f"{k} = {toml_value(v)}" for k, v in profile.items()) + "\n", encoding="utf-8")
    saves = list(save_root.glob("*/ER0000.Shiki"))
    if len(saves) != 1:
        raise ValueError("Expected exactly one active ER0000.Shiki; no game was started")
    backup = root / "save-backups" / run.name
    backup.mkdir(parents=True, exist_ok=False)
    shutil.copy2(saves[0], backup / "ER0000.Shiki")
    info = {"mode":mode, "full_stack":full_stack, "run":str(run.resolve()),
            "profile":str((run / "manual.me3").resolve()), "save_backup":str(backup.resolve()),
            "source_profile_sha256":hashlib.sha256((ga_root / "Start_GoldenAge.me3").read_bytes()).hexdigest(),
            "save_backup_sha256":hashlib.sha256((backup / "ER0000.Shiki").read_bytes()).hexdigest()}
    (run / "manual-session.json").write_text(json.dumps(info, indent=2), encoding="utf-8")
    (root / "manual-latest.json").write_text(json.dumps(info, indent=2), encoding="utf-8")
    return info


def collect(root):
    from probe import report
    from audit_retirement import analyze
    from audit_actions import report as action_report
    logs = root / "manual-runs"
    for log in logs.glob("*/er-demon-beast.log"):
        result = {"scope": "restoration deadline only; not full Spec acceptance",
                  "cycles": analyze(log.read_text(encoding="utf-8", errors="replace"))}
        (log.parent / "retirement-report.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    for trace in logs.glob("*/Saved/er-demon-beast/*/frames.jsonl"):
        report(trace)
        action_report(trace)
    # Users may load this DLL directly from their existing GoldenAge profile.
    for trace in (root / "Saved/er-demon-beast").glob("*/frames.jsonl"):
        report(trace)
        action_report(trace)
    destination = root / ("erdb-logs-" + datetime.now().strftime("%Y%m%d-%H%M%S") + ".zip")
    with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in logs.rglob("*"):
            if path.is_file() and path.suffix.lower() in (".log", ".json", ".jsonl", ".md", ".toml", ".me3", ".txt"):
                archive.write(path, path.relative_to(root))
        direct = [root / name for name in ("er-demon-beast.log", "capture-latest.json", "er-demon-beast.toml", "build-result.json")]
        direct.extend((root / "Saved/er-demon-beast").rglob("*"))
        for path in direct:
            if path.is_file() and path.suffix.lower() in (".log", ".json", ".jsonl", ".md", ".toml"):
                archive.write(path, path.relative_to(root))
    return {"log_archive":str(destination.resolve()), "contains_saves_or_dlls":False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("op", choices=["prepare", "collect"])
    parser.add_argument("root", type=Path)
    parser.add_argument("--ga", type=Path, default=Path("F:/GoldenAge/_GoldenAge"))
    parser.add_argument("--mode", choices=["load", "observe", "control", "missing"], default="load")
    parser.add_argument("--full-stack", action="store_true")
    parser.add_argument("--save-root", type=Path)
    args = parser.parse_args()
    if args.op == "prepare":
        if args.save_root is None:
            parser.error("--save-root required for a verified backup")
        result = prepare(args.root, args.ga, args.mode, args.full_stack, args.save_root)
    else:
        result = collect(args.root)
    print(json.dumps(result, ensure_ascii=False))
