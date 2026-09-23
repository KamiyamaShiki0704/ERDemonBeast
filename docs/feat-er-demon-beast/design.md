---
id: FEAT-ER-demon-beast
title: 首轮控制闭环技术设计
type: design
version: 1.3
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
- `native.rs`：FrameEnd 回调中的实体创建、身份校验、控制事务及回滚。保留 main_player，操作受控 NPC 通道，所有跨帧引用以 handle 重新解析，模块先检查可读性和 owner 再解引用。
- `lib.rs`：版本检查、配置、日志、启动与按键入口。首次产物只供隔离验证，不安装到现用 Mod 配置。
- 输入：F6 请求进入，F7 请求解除；重复进入忽略。原生移动/攻击输入由 ER 自身控制通道接收。
- 首轮替代角色选 c4070 Wolf，候选 NpcParam/NpcThinkParam 为 40700000；实际参数存在性由运行入口检查后才允许生成。无法证明时拒绝进入，保持本体。
- 创建与控制各自有事务所有权；公共 debug creator 忙或已有相机控制覆盖时拒绝抢占。
- 兽体先生成并确认可用，再改变本体。创建请求保留已有 handle 集合，结合未被覆盖的请求和 last-created 指针确认新实体，随后调用固定 Summon 源码注册实体 ID。拒绝不确定归属。
- 先解除控制关联、恢复本体，再隐藏兽体；下一 FrameEnd 通过 Summon 注销六个任务并脱离集合，连续两个帧末引用检查均为空后析构并使用原分配器释放。清理完成才回到普通状态，不再隐藏复用。
- Summon 来源固定为 `01491edfa709545786d0fe795ab2fb2458dca4e1`；源码/fixture/许可及哈希在 `src/vendor/summon/`，未修改兄弟工程。此为源码复用，不调用已安装的 Summon.dll；实机验证仍归本项目。
- 接受该版本已观测的 FrameEnd ID：168 或 0x900000A8，其他 stage 拒绝。世界、本体、控制引用或释放接口校验失败时隔离对象并阻止新建，不把“没能释放”报告为成功。
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
| 005 | 状态门禁、帧末安全释放后重新创建 | 重入及连续十轮、引用排空/分配器协议 | 实体数量、镜头/控制稳定 |
| 006 | 创建拒绝/超时后清理 | 故障注入、迟到结果不激活 | 不可用兽体场景下本体恢复 |

## 原生运行环境与取证

目标拓扑为 ER 单机，不存在 PIE/world 索引。绑定条件：精确 EXE 哈希/版本、当前进程、WorldChrMan 本体身份、兽体 handle 与创建归属一致。拒绝陌生版本、现有控制覆盖、无本体、非有效场景或身份变化。

公共 FrameRecorderProbe 只定位到 GPUECli/Unreal 用法，无原生 ER 适配。用户 2026-09-23 回复“允许”，明确批准本项目使用 **ER 原生状态采集 + 日志分析 + 人工游戏验收** 替代。此为项目局部例外；不修改 Spec 1.0 的前置、输入、输出或阈值，不声称执行 UE Recorder。

`src/native/capture.rs` 在 FrameEnd 回调中采集。未开启 Case、未请求能力且为普通状态时不遍历角色；Case 开启后目标间隔 50ms，状态变化及进入/解除请求总是记录，含同次 tick 写入前/后的快照。采集单调时钟，输出通过有界队列交给独立写线程；丢帧计数、身份解析错误保留到证据，分析不能据此宣布通过。

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

## 0.2.1 恢复期读取与会话结束

固定 Summon 原件保留，运行时引用读取使用本项目 src/retirement.rs 适配副本。以 ReadProcessMemory 完整复制数组及节点，拒绝不完整复制或向量头变化，允许跨多个可读内存区域。保留连续两次零引用和任务代理清空条件，增加限频 read_error 定位，不放宽 Spec 的 3 秒恢复阈值。

完全释放并恢复成功后清空旧世界及调试控制地址；活动或清理中的世界变化仍拒绝释放。用户 0.2.0 的 37.115 秒失败和新版本待复测状态见 research/retirement-stall-0.2.1.md。

## 0.2.2 c0010 输入诊断补充

action_trace 默认关闭，测试配置开启时在 Active 期间每个 FrameEnd 保留已有前后快照，并增加动作读回/输入允许掩码及有限物理键状态。只记录游戏前台的鼠标左右键、Shift/Ctrl/Space；不将物理键当作语义动作，不增加引擎写入。无 Case 时的活动采样不再限制为一秒，但仍不构成正式 Case 验收。普通状态及加载屏幕入口策略保持原规则。

FrameEnd 的零动作可能已被此前行为更新消费，TAE 字段是请求队列标识，不能独立证明动画播放。audit_actions.py 只汇总观察值；缺帧和截断标记为不完整。收集器同时包含直接 DLL 加载与 manual-runs 两种日志。人工输入顺序见 ACTION_TEST.md；现场攻击根因和修复尚待此证据，未复制 NR 引擎专用 HKS 到 ER。

## c0010 行为适配包 v1（2026-09-24 用户批准）

0.2.2 实测已证实请求进入兽体。用户批准同步 Behbnd/HKS，并要求核对名称映射。将 c0010 Characters HKX 的第一项 luaFiles 从 c9997.hks 改为私有 d0010.hks，后者由当前 GA 通用 HKS 加本项目地面动作适配器构成；原版 c0010 仍按原引用顺序加载。

适配器通过 ER ActionRequest / ActionCancelRequest 分别处理起手与动作取消，保留原版地面连段选择。既有 CMSG 负责动作结束返回待机。ER 编号与 NR 不同的 ULTRA/ROLLING 不直接照搬。对图引用的全量名称做差集追加，保留三个 name-ID 表既有 ID，并在写回后验证引用完备。所有五个安装目标与四个保护输入使用 SHA256 预检，安装先备份，回滚拒绝覆盖后续修改。

脚本、实际结构证据和离线入口见 research/c0010-behavior-adapter-v1.md；具体人工输入见 BEHAVIOR_TEST.md。该资源候选不改变 DLL 控制、计时与退出生命周期，也不扩张完整伤害、跳跃或联机验收范围。

## c0010 行为 v2 起手修正

用户实测 v1 攻击/闪避失败。目标 EXE 静态核验后，在私有脚本非取消入口补 Wait 标志，使 ActionRequest 读取新按键；连段继续受 ActionCancelRequest 限制。私有副本移除一行未使用且会在裸/反斜线 chunk 名下抛错的路径初始化。只更新 d0010，其他行为资源与 DLL 不变。加入每实例最多 80 行、IO 失败隔离的临时脚本证据；见 research/c0010-behavior-adapter-v2.md。玩法仍由人验收。
