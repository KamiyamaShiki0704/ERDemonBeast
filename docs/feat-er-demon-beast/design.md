---
id: FEAT-ER-demon-beast
title: 首轮控制闭环技术设计
type: design
version: 1.1
updated: 2026-09-23
based_on:
  - spec.md@1.0
code_paths: [src/session.rs, src/native.rs, src/native/capture.rs, src/lib.rs]
project_paths: [Cargo.toml, er-demon-beast.toml]
---

# 设计与验证边界

## 已确认输入和目标

用户 2026-09-23 的“可以，继续任务”确认 Spec 1.0 的六组语义 Probe。
目标为本机 ER WW 2.7.1.0（App 1.17.1），EXE SHA256
`1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`。
当前读取路径是 `F:/SteamLibrary/steamapps/common/ELDEN RING/Game/eldenring.exe`，旧 G: 路径不作为当前输入。

## 最小实现

Rust cdylib，运行时绑定固定采用 Summon 的 fsrs `02fa5681e27fd2ecd9da79d34aef9fae96805539`，避免另一分支不同 ChrIns 尾部布局。状态逻辑独立于原生内存，可用故障注入后端验证。

- `session.rs`：普通/进入/活动/恢复/故障状态、3 秒进入超时、15 秒活动期、恢复重试、拒绝重入。
- `native.rs`：游戏任务回调中的实体创建、身份校验、控制事务及回滚。保留 main_player，操作受控 NPC 通道，所有跨帧引用以 handle 重新解析。
- `lib.rs`：版本检查、配置、日志、启动与按键入口。首次产物只供隔离验证，不安装到现用 Mod 配置。
- 输入：F6 请求进入，F7 请求解除；重复进入忽略。原生移动/攻击输入由 ER 自身控制通道接收。
- 首轮替代角色选 c4070 Wolf，候选 NpcParam/NpcThinkParam 为 40700000；实际参数存在性由运行入口检查后才允许生成。无法证明时拒绝进入，保持本体。
- 创建与控制各自有事务所有权；公共 debug creator 忙或已有相机控制覆盖时拒绝抢占。
- 兽体先生成并确认可用，再改变本体。先解除控制关联、恢复本体，再惰化兽体。首轮保留一个隐藏兽体用于复用，不引入未经 live 验证的析构路径。
- 恢复使用进入前的字段快照，仅恢复本功能修改的字段/位；不使用固定默认值覆盖其他 Mod 的设置。
- 失效或恢复失败进入故障态并继续尝试清理，阻止再次创建。不能把恢复失败报告为普通状态。

原生写入的具体字段与静态证据见 `research/control-audit.md`。未证实字段不能因公开源码使用过就被认定为有效；未通过接口核验的原生路径须保持关闭。

## Spec 覆盖与实施顺序

| Spec | 实现/观测 | 离线检查 | 运行验收 |
|---|---|---|---|
| 001 | 创建就绪后原子接管；保存本体快照 | 延迟创建、失败回滚、超时 | 本体仍在、隐形、不受击、单一兽体 |
| 002 | NPC 控制器与相机覆盖关联 | ABI/指令位置核对 | 移动、观察、真实攻击命中；不能由状态机测试替代 |
| 003 | 解除关联、位置恢复、字段还原 | 恢复失败重试与故障保留 | 3 秒、水平误差 ≤1 米、镜头恢复 |
| 004 | 从接管成功开始计时 | 15 秒边界、慢加载不扣时长 | 原生回调调度下的真实到期时间 |
| 005 | 状态门禁、隐藏兽体复用 | 重入及连续十轮 | 实体数量、镜头/控制稳定 |
| 006 | 创建拒绝/超时后清理 | 故障注入、迟到结果不激活 | 不可用兽体场景下本体恢复 |

## 原生运行环境与取证

目标拓扑为 ER 单机，不存在 PIE/world 索引。绑定条件：精确 EXE 哈希/版本、当前进程、WorldChrMan 本体身份、兽体 handle 与创建归属一致。拒绝陌生版本、现有控制覆盖、无本体、非有效场景或身份变化。

公共 FrameRecorderProbe 只定位到 GPUECli/Unreal 用法，无原生 ER 适配。用户 2026-09-23 回复“允许”，明确批准本项目使用 **ER 原生状态采集 + 日志分析 + 人工游戏验收** 替代。此为项目局部例外；不修改 Spec 1.0 的前置、输入、输出或阈值，不声称执行 UE Recorder。

`src/native/capture.rs` 在 ChrIns_PostPhysics 回调中采集。未开启 Case、未请求能力且为普通状态时不遍历角色；Case 开启后目标间隔 50ms，状态变化及进入/解除请求总是记录，含同次 tick 写入前/后的快照。采集单调时钟，输出通过有界队列交给独立写线程；丢帧计数、身份解析错误保留到证据，分析不能据此宣布通过。

- 绑定：每进程唯一 run ID、EXE/DLL SHA256、配置、构建清单和 Spec 版本。
- 阶段：start → ordinary → entering → active → restoring/normal → end；同 tick 恢复直接记录 before=active、phase=normal。
- 字段：本体/兽体 handle 与地址、位置、HP、render/alpha、保护标志、动作请求与 TAE、NPC 动作、相机覆盖、debug 输入 owner、兽体 override、归属实体数量、残留事务数、锁定目标身份/HP。
- 请求：F6/F7 或 `probe.py command`。文件命令须匹配当前 run，单调递增 id，进入/解除仅在 Case 已开始且游戏前台时接收；每条接受/拒绝都有回执。start/end/mark 仅标注，不宣称语义前置成立。
- 路径：DLL 目录 `Saved/er-demon-beast/<run>/frames.jsonl` 与 `meta.json`；最新运行由 `capture-latest.json` 定位；`probe.py report` 产生 report.json/md，meta.report_path 指向 Markdown。
- 地图：保持 Spec 的同一平坦无环境伤害场景，运行前在 start 的 note 记录具体场景与条件；不能以菜单中的无本体错误替代不可用兽体 Probe。

六组均输出独立结论。001 检查进入时限、关联、保护字段与 HP，敌人实际尝试命中及隐形/不响应输入仍人工验证。002 保留位移、动作及锁定目标 HP 供对应输入与命中取证；不能由位移或掉血自动推断因果。003 检查恢复时限、字段、位置和 5 秒后观测。004 检查无手动输入、15±0.25 秒触发和 18.25 秒总时限。005 检查十轮、每轮三次额外请求、实体上限和末尾 10 秒观察。006 检查失败原因、3 秒恢复和字段；随后移动/攻击仍人工验证。

分析器只给 NOT_RUN、INCONCLUSIVE、FAIL 或 AUTOMATIC_PARTIAL；后者仅表示已实现的自动数据检查通过。缺 start/end、截断、缺帧或解析错误都不能通过。最终 PASS/FAIL 由人给出。运行结束先恢复普通状态，再退出测试进程；不在活动中热卸载。

## 构建与交付

先固定来源并核对 ABI，再运行 Rust 离线生命周期测试、格式/静态检查、release 构建及 PE/哈希检查。不是 UE 项目，不运行 buildeditor_development.bat。

脚本位于 `Scripts/delivery/feat-er-demon-beast/script/`。隔离产物位于忽略目录 `dist/`；不更改游戏原文件或现用启动配置。回滚为退出测试进程并移除独立测试 profile 中的 DLL，严禁活跃状态热卸载 DLL。
