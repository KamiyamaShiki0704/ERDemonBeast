"""ER-native capture control and conservative, reproducible evidence analysis."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import time


def command(root, action, case=None, note=None):
    meta = json.loads((root / "capture-latest.json").read_text(encoding="utf-8"))
    request = dict(run=meta["run"], id=time.time_ns() // 1000,
                   action=action, case=case, note=note)
    temp = root / "capture-command.tmp"
    temp.write_text(json.dumps(request), encoding="utf-8")
    os.replace(temp, root / "capture-command.json")
    print(json.dumps(request))
    # An acknowledgement, not the mere existence of this file, proves execution.


def distance(a, b):
    return math.hypot(a[0] - b[0], a[2] - b[2])


def restored(state, baseline):
    body, original = state.get("body"), baseline.get("body")
    if not body or not original:
        return False
    keys = ("handle", "address", "render", "alpha", "tint_alpha", "disable_hit", "override")
    return (all(body.get(k) == original.get(k) for k in keys)
            and body.get("debug_flags", 0) & 0x38 == original.get("debug_flags", 0) & 0x38
            and state.get("camera") == baseline.get("camera")
            and state.get("manip_owner") == baseline.get("manip_owner")
            and state.get("patches") == 0 and state.get("linked") is False
            and state.get("pending_spawn") is False)


HUMAN = {
    1: "确认敌人确实尝试命中本体、本体不可见且无输入响应；保护标志及 HP 不变不能单独证明免伤。",
    2: "分别验证前/侧向输入后 ≤1 秒响应、镜头跟随与自由观察；在 5 秒窗口确认本次攻击造成敌人掉血。",
    3: "确认本体可操控、兽体不再响应、静置 5 秒内无镜头持续错位。",
    4: "确认自动恢复演出、控制和镜头正常。",
    5: "确认十轮本体、镜头及重复使用表现正常；最终静置 10 秒。",
    6: "确认兽体不可用这一前置，失败反馈可理解，随后本体能够移动及攻击。",
}


def analyze(rows):
    results = []
    for case in range(1, 7):
        selected = [r for r in rows if r.get("case") == case]
        frames = [r for r in selected if r.get("kind") == "frame"]
        commands = [r for r in selected if r.get("kind") == "command" and r.get("accepted")]
        checks = []

        def check(name, condition, detail="", known=True):
            checks.append(dict(name=name, status=("PASS" if condition else "FAIL") if known else "INCONCLUSIVE", detail=detail))

        started = [r for r in commands if r.get("action") == "start"]
        ended = [r for r in commands if r.get("action") == "end"]
        result = dict(case=f"PROBE-ERDB-{case:03}", status="NOT_RUN", checks=checks,
                      human_pending=HUMAN[case], frames=len(frames))
        results.append(result)
        if not started:
            continue
        result["status"] = "INCONCLUSIVE"
        if len(started) != 1 or len(ended) != 1 or not frames:
            check("完整且独立的测试区间", False, "每个文件每组 Case 只运行一次；需 start、帧、end。", known=False)
            continue
        times = [r["t"] for r in frames]
        gap = max([b-a for a, b in zip(times, times[1:])] + [0])
        valid = (times == sorted(times) and gap <= .25
                 and started[0]["t"] <= times[0] <= times[-1] <= ended[0]["t"]
                 and times[0] - started[0]["t"] <= .25
                 and ended[0]["t"] - times[-1] <= .25
                 and all(not r.get("dropped") for r in selected)
                 and all("body" in r.get("pre", {}) and "body" in r.get("post", {}) for r in frames))
        check("采集连续且实体可解析", valid, f"最大采样间隔 {gap:.3f}s；大于 0.25s 不足以作精确时序结论。", known=valid)
        if not valid:
            continue
        baseline = frames[0]["pre"]
        check("独立普通状态起点", frames[0]["before"] == "normal" and baseline.get("patches") == 0)
        check("归属兽体数量上限", all(r["post"].get("owned_count", 999) <= 1 for r in frames), "统计本项目归属标识的实体，含隐藏待复用兽体。")
        requests = [r for r in frames if r.get("enter") and r["before"] == "normal" and not r.get("revert")]
        active = [r for r in frames if r["phase"] == "active"]
        activations = [r for r in active if r["before"] != "active"]
        exits = [r for r in frames if r["before"] == "active" and r["phase"] != "active"]
        recoveries = [r for r in frames if r["phase"] == "normal" and r["before"] != "normal"]
        for req in requests:
            following = next((r for r in activations if r["t"] >= req["t"]), None)
            if case != 6:
                check("进入 ≤3s", following is not None and following["t"] - req["t"] <= 3,
                      f"request={req['t']:.3f}, active={following['t'] if following else None}")
        if case != 6:
            check("无运行错误或故障态", all(not r.get("error") and r["phase"] != "fault" for r in frames))
            check("实际进入活动状态", bool(requests) and bool(activations))
            check("活动控制与隐藏字段", all(
                r["post"].get("beast") and r["post"].get("linked") is True
                and r["post"].get("camera") == r["post"]["beast"]["address"]
                and r["post"].get("manip_owner") == r["post"]["beast"]["address"]
                and r["post"]["beast"].get("override") == r["post"].get("manipulator")
                and r["post"]["body"]["handle"] == baseline["body"]["handle"]
                and r["post"]["body"].get("render") is False
                and r["post"]["body"].get("alpha") == 0
                and r["post"]["body"].get("disable_hit") is True
                for r in active), known=bool(active))
        if case == 1:
            check("活动采样期间本体 HP 不变", all(r["post"]["body"]["hp"] == baseline["body"]["hp"] for r in active), known=bool(active))
        if case == 2 and active:
            positions = [r["post"]["beast"]["position"] for r in active if r["post"].get("beast")]
            movement = max((distance(positions[0], p) for p in positions), default=0)
            targets = [r["post"]["target"] for r in active if r["post"].get("target")]
            check("操控/命中因果待人工核对", False,
                  f"兽体水平最大位移 {movement:.3f}m；锁定目标样本 {len(targets)}。输入时间/方向、镜头和攻击归因须人工确认。", known=False)
        if case in (3, 4, 5):
            check("已记录恢复", bool(exits) and bool(recoveries))
            for exit_row in exits:
                recovered = next((r for r in recoveries if r["t"] >= exit_row["t"]), None)
                check("恢复 ≤3s", recovered is not None and recovered["t"] - exit_row["t"] <= 3)
                if recovered:
                    check("恢复字段与进入前一致", restored(recovered["post"], baseline))
                    beast = exit_row["pre"].get("beast")
                    delta = distance(recovered["post"]["body"]["position"], beast["position"]) if beast else None
                    check("恢复位置水平误差 ≤1m", delta is not None and delta <= 1, str(delta))
            if case in (3, 5):
                required = 5 if case == 3 else 10
                check(f"恢复后静置采集 ≥{required}s", bool(recoveries) and times[-1] - recoveries[-1]["t"] >= required)
                if recoveries:
                    tail = [r for r in frames if r["t"] >= recoveries[-1]["t"]]
                    check("末次恢复后无控制残留", all(r["phase"] == "normal" and restored(r["post"], baseline) for r in tail))
            if case == 3:
                check("存在手动解除请求", any(r.get("revert") for r in exits))
            if case == 4:
                check("无手动解除输入", not any(r.get("revert") for r in frames))
                check("独立一次活动", len(activations) == len(exits) == len(recoveries) == 1)
                if len(activations) == len(exits) == len(recoveries) == 1:
                    elapsed = exits[0]["t"] - activations[0]["t"]
                    check("15s 到期触发 ±0.25s", abs(elapsed - 15) <= .25, str(elapsed))
                    check("活动开始后 ≤18.25s 恢复", recoveries[0]["t"] - activations[0]["t"] <= 18.25)
            if case == 5:
                check("完成十轮手动闭环", len(requests) == len(activations) == len(exits) == len(recoveries) == 10
                      and all(r.get("revert") for r in exits))
                for activation, exit_row in zip(activations, exits):
                    repeats = [r for r in frames if activation["t"] < r["t"] < exit_row["t"] and r.get("enter")]
                    check("该轮额外三次进入被忽略", len(repeats) == 3 and all(r["phase"] == "active" for r in repeats))
        if case == 6:
            check("不可用兽体进入请求", len(requests) == 1)
            if requests:
                req = requests[0]
                recovered = next((r for r in frames if r["t"] >= req["t"] and r["phase"] == "normal" and r.get("error")), None)
                check("3s 内恢复并有失败原因", recovered is not None and recovered["t"] - req["t"] <= 3)
                check("未误激活兽体", not active)
                if recovered:
                    check("失败后字段恢复", restored(recovered["post"], baseline), recovered["error"])
        statuses = {c["status"] for c in checks}
        result["status"] = "FAIL" if "FAIL" in statuses else "INCONCLUSIVE" if "INCONCLUSIVE" in statuses else "AUTOMATIC_PARTIAL"
    return results


def report(path):
    raw = path.read_bytes()
    errors, rows = [], []
    for index, line in enumerate(raw.splitlines(), 1):
        try:
            rows.append(json.loads(line))
        except (ValueError, UnicodeError):
            errors.append(f"line {index}: invalid/truncated JSON")
    meta = json.loads(path.with_name("meta.json").read_text(encoding="utf-8"))
    results = analyze(rows)
    if errors:
        for result in results:
            if result["status"] != "NOT_RUN":
                result["status"] = "INCONCLUSIVE"
    document = dict(meta=meta, evidence_sha256=hashlib.sha256(raw).hexdigest(), errors=errors,
                    results=results, final_acceptance="PENDING_HUMAN")
    path.with_name("report.json").write_text(json.dumps(document, indent=2, ensure_ascii=False), encoding="utf-8")
    lines = ["# ER 原生 Probe 报告", "", f"Run: `{meta['run']}`", "",
             f"DLL SHA256: `{meta['dll_sha256']}`", "", f"Frames SHA256: `{document['evidence_sha256']}`", "",
             "最终验收：PENDING_HUMAN。AUTOMATIC_PARTIAL 仅表示已实现的数据检查通过，不能替代完整 Spec 或人工结论。", ""]
    lines.extend(errors)
    for result in results:
        lines.extend([f"## {result['case']} — {result['status']}", "", f"采样帧数：{result['frames']}", ""])
        lines.extend(f"- {c['status']}：{c['name']}。{c['detail']}" for c in result["checks"])
        lines.extend(["", f"人工待验：{result['human_pending']}", ""])
    path.with_name("report.md").write_text("\n".join(lines), encoding="utf-8")
    print(json.dumps({"meta.report_path":str(path.with_name("report.md")),
                      "results":{r["case"]:r["status"] for r in results}}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    subs = parser.add_subparsers(dest="op", required=True)
    cmd = subs.add_parser("command")
    cmd.add_argument("root", type=Path, help="running DLL directory")
    cmd.add_argument("action", choices=["start", "end", "mark", "enter", "revert"])
    cmd.add_argument("--case", type=int, choices=range(1, 7))
    cmd.add_argument("--note")
    rep = subs.add_parser("report")
    rep.add_argument("frames", type=Path)
    args = parser.parse_args()
    if args.op == "command":
        command(args.root, args.action, args.case, args.note)
    else:
        report(args.frames)
