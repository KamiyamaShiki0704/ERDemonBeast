---
id: FEAT-ER-demon-beast
title: Summon 框架整合与人工测试候选
type: build
version: 1.3
updated: 2026-09-23
based_on: [spec.md@1.0, design.md@1.3]
code_paths: [src/lib.rs, src/session.rs, src/native.rs, src/native/capture.rs, src/retirement.rs, src/vendor/summon/PROVENANCE.md]
project_paths: [Cargo.toml, Cargo.lock, er-demon-beast.toml, MANUAL_TEST.md]
---

# Build 结果

最新资源交付：2026-09-24 已安装 c0010 行为适配包 v1，DLL 沿用 0.2.2。真实 HKS 离线调用、14 项 Python 检查（含安装/回滚实际脚本 fixture）、四 HKX 结构审计、BND 写回读回和原名称编号保持通过；游戏待人工。五个目标与保护文件哈希见 research/behavior-build-v1.json、research/behavior-deployment-v1.json，备份位于 `F:/GoldenAge/_GoldenAge/dll/demon_beast/_backups/behavior-20260924-004322-305`。行为范围与剩余限制见 BEHAVIOR_TEST.md。以下 0.2.2/0.2.1 部分为 DLL 历史证据。

当前自动结论：**PARTIAL**。0.2.2 为 c0010 键鼠动作诊断版，尚未修复攻击行为。21 Rust + 11 Python、fmt、Clippy、release 和小栈加载检查通过。用户确认 0.2.1 c0010 变身/移动/转向可用，但其他按键无效；详见 research/c0010-action-diagnosis.md 和 ACTION_TEST.md。没有启动或操作游戏。下面保留历史 0.2.1 构建与部署证据。

0.2.2 已安装至 F:/GoldenAge/_GoldenAge/dll/demon_beast，841728 bytes，SHA256 `63D42284E9BE348E49521BAD750A5D14D7B49417E3D49C972024545A2A38B176`。来源提交 03a0a35，构建绑定见 research/build-result-0.2.2.json。替换前已确认 ER 未运行，旧文件备份至该目录 `_backups/action-trace-20260924-000718`，安装文件哈希逐一相等，配置 action_trace=true。GA regulation 哈希仍为 `07444D99AEF7166A2AD9E8A505842134518CA62851BDEAAEE20E6F6149ABC4CD`。已使用新收集器离线处理五份既有直接加载日志，成功生成 ZIP；没有将旧日志当作新 DLL 的实机验证。

## 固定输入和产物

- Spec 1.0，Design 1.3；源码 revision `0f98949737a3351ab394b84dee1e6c645323b505`。
- 工程 KB revision `9511e3e8137fd2cda9fca74bb07a4b688fe89087`；NR 研究 KB revision `8119ee174b685a39806a243a9c5acfbc5dbd912b`。
- fsrs `02fa5681e27fd2ecd9da79d34aef9fae96805539`；Summon 复用来源 `01491edfa709545786d0fe795ab2fb2458dca4e1`，原文件及 fixture 的哈希固定于 vendor provenance。
- ER WW 2.7.1.0，EXE SHA256 `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`。
- 产物 `dist/er-demon-beast-0.2.1/er_demon_beast.dll`，834048 bytes。
- DLL SHA256 `D075B652731E399048CA140CC8E42BA7A82F0490499DEC3EF73B98EC8D96FB82`。
- [构建清单](research/build-result-0.2.1.json) 绑定源码、依赖、DLL 和检查结果。

## 本轮改动

用户已测试 0.2.0 的 03/04，日志记录一次 37.115 秒清理等待。0.2.1 修复已用真实内存测试复现的跨可读区域误判，以及真实 NativeBackend::restore 的旧地址绑定残留；两项回归均先失败后通过。增加限频 read_error 和按键暂缓记录，05 日志包增加恢复耗时报告。详见 [清理等待诊断](research/retirement-stall-0.2.1.md)。旧运行中 None 的具体来源仍未知，不能将离线修复称为实机问题已经解决。

以下为延续的基础实现：

复用 Summon 的实体 ID 注册与两阶段退出/原分配器释放；确认创建归属后才接管。恢复本体后等待帧末注销任务、集合脱离与引用排空，清理完成前拒绝重入。操作统一到严格匹配的 FrameEnd，OwnedPtr 解引用前增加模块可读性/owner 检查。保留已有 DllMain 小栈修复。

新增 load/observe/control 分离的手动入口，每次启动生成包内独立 run，强制单机并备份用户指定的 ER0000.Shiki。默认只加载本 DLL 与 GA 资源；兼容性入口再增加其他 DLL，排除重复召唤生成器。原 GoldenAge profile 和 Summon 工程均未修改。

## 验证

| 检查 | 结果 | 范围 |
|---|---|---|
| Rust tests | 20 PASS | 状态机、内存事务、模块拒绝、FrameEnd 编码与 Summon 原生/分配器/引用排空协议 |
| Python tests | 10 PASS | 报告保守判定、手动配置、备份与原文件不改动 |
| 小栈 LoadLibrary 宿主 | PASS | DllMain 线程通知；不进入 ER 引擎 |
| Summon 原生 fixture | 3 PASS | 与目标 EXE 精确字节相等 |
| Clippy / fmt / release build / PE | PASS | 编译、静态检查和 AMD64 DLL |
| 启动脚本 | 语法/临时文件测试 PASS | 未执行游戏启动分支 |
| 游戏功能与实际释放 | NOT_RUN | 用户手动验收 |

## 人工交付与证据

具体入口和按键见 [MANUAL_TEST.md](../../MANUAL_TEST.md)。检查 `outcome=destroyed retirement=frame-end-drained`，不能仅以本体出现判断内存已释放。日志按每次 run 留存，05-collect-logs.cmd 可生成汇总 ZIP；正式 Case 报告路径由 meta.report_path 定位。

- [本轮自检](research/summon-integration-audit.md)：各风险、采取的修复、来源和未证明项。
- [历史崩溃自检](research/dll-self-audit.md)：已确认的 C00000FD 栈溢出及未归因的访问异常。
- 旧 0.1.1 两份无 Case 的采集报告仍保留在 dist 对应 Saved 路径，仅为历史加载记录；不复用为当前验收结果。

本次用户优先复测 03/04，每种组合十轮 F6/F7 及一次自然到期。若再次卡住，等十秒写出定位日志后退出收集。基础稳定后再按 Spec 进行六组 Probe。最终 PASS/FAIL 由用户给出。

## 后续人工反馈

用户回复“1和2看起来没问题”，按路线中的控制闭环与首轮行为记录为人工初步通过。已读取 0.2.1 完整组合运行，两次恢复 0.116/0.219 秒并完成析构；见 research/control-baseline-and-next-stage.md 和 research/retirement-user-check-0.2.1.json。本构建的历史离线清单不回写，未录制的正式 Case 不补报自动 PASS。下一阶段准备真实 c0010 最小形态。

## c0010 加载/控制测试部署

用户已提供 GA 的 c0010 资源，并明确授权部署 DLL 到 F:/GoldenAge/_GoldenAge/dll/demon_beast、补齐参数。沿用 0.2.1 DLL，配置 chr_id=10、NpcParam/NpcThinkParam=100000、两个开关开启。现有 profile 已引用此 DLL，未修改 profile。

仅向 GA/regulation.bin 的 NpcParam 和 NpcThinkParam 各新增 100000；ER 40700000 为测试基础，白名单适配 NR 物理字段并清除狼的常驻效果及掉落。不是完整战斗参数移植。写回读回验证两个表内原有 9919 行的原始数据和名字均一致，其他表的原始字节及 binder ID/flags 均一致。备份及部署哈希见 research/deployment-c0010.json；参数详情见 research/c0010-param-patch-report.json。

已核对五个资源包容器能读回。GA/action/script/c0010.hks 未找到，c9997.hks 存在，动作兼容尚未证实。测试步骤见 C0010_TEST.md。附独立离线手动入口和带哈希保护的回滚脚本；代理未执行游戏启动。回滚脚本语法检查通过，临时文件执行测试被“游戏正在运行”守卫拒绝，未做真实回滚，不报告该执行测试已通过。安装前游戏未运行；部署后检测到用户游戏已启动，随后仅做只读核对和项目记录。

## c0010 行为 v2

用户确认 v1 移动/转向正常、攻击/闪避失败。修复私有 HKS 的 Wait 起手条件与裸/反斜线 chunk 名初始化；证据和红绿测试见 research/c0010-behavior-adapter-v2.md。四个资产测试与真实 HKS 回归通过。确认 ER 进程完全退出后，仅部署 d0010.hks，SHA256 9275e9bfa0620b4aab68a19e454b25a4aa80f6d2e91149c47e3314e951bc238f。DLL 仍为 0.2.2，其余四个行为文件和四个保护输入逐项验哈希不变。安装元数据保留最初备份，旧整包回滚入口仍有效；v1 私有脚本另存 _backups/behavior-v2-20260924-010918。实际攻击、闪避和恢复待用户按 BEHAVIOR_TEST.md 验收，不宣称游戏修复已通过。

## c0010 行为 v3 / DLL 0.3.0 候选

Spec 1.1 / Design 1.4；用户确认 v2 五动作可用并要求 F7 与到期等待当前动作结束。新增方向分派、动作所有权和 TAE 时间下界、Finishing 与动画确认、两段兽体演出、运行时镜头恢复。红绿证据与限制见 research/c0010-behavior-adapter-v3.md；25 Rust、16 Python、Lua 回归、图往返、固定 EXE、64KiB DLL 线程加载检查通过，见 research/verification-v3.json。人工步骤 BEHAVIOR_TEST.md。未启动游戏。

Manage-C0010V3.ps1 按固定白名单与前后哈希成套安装/恢复 DLL、PDB、配置、HKS、Behbnd 及元数据；所有修改前备份，安装异常回滚。三张名字表和四个原始保护输入只验哈希。成套回退入口 Restore-C0010V3.cmd。完整 c0000 起手/视觉及实际镜头仍为未完成/未验证项，不能声明原版演出全部还原。

### v3 安装结果

2026-09-24 01:59（本地）确认 ER 未运行后完成十文件事务安装；源码 188bdb20629c5d44befb094adfdf1f990c232c07，DLL SHA256 fcbff114cd3cd64b9dc6e9595a3c69c0e0bd6335db05e4e4e54a799dcfa47daa。安装后十个目标与全部 v2 备份逐项哈希核对通过，七项保护输入保持不变，原资产整体回滚记录也一致。事务记录 research/deployment-v3.json；精确构建与源码哈希 research/build-result-v3.json。备份 complete-v3-20260924-015939-004。未实际执行回退或游戏测试；回退脚本只做语法及完整备份/当前目标哈希核对，不报告实机恢复 PASS。

## v3 进入人测失败与 v4 私有脚本修复

v3 run 1790187506821-58692 四轮 enter_timeout、无 Active，用户确认无登场动画，故不能继续保留 v3 进入功能通过的表述。证据 entry-timeout-v3.json。v4 修正根 Update 过早发事件以及依赖晚建激活回调的假设，新增初始化顺序与实际节点完成红绿回归，未用延长超时或定时伪完成掩盖错误。

2026-09-24 02:35 完全退出 ER 后安装 HKS v4（67ffb22b9a837cf9d92a7fd00c6ab5a6e55dc4737ff0247bab0751b6bb478ea3）及四份文档/版本元数据。DLL 0.3.0、PDB、配置、Behbnd、三张名字表、原始 c0010/c9997、anibnd 和参数内容保持原哈希。复用 v3 安装器的十文件事务，输出中的 v3 标签是旧脚本固定文案；准确包版本见 deployment-v4.json / behavior-build-v4.json。安装与备份核对通过，未启动游戏。

父 deployment-v3.json 的 after 哈希已同步新包，before 和原始备份不变；Restore-C0010V3.cmd 仍可整套退回用户已验证动作可用的 DLL 0.2.2 + 行为 v2。此次失败 v3 资源单独备份在 complete-v3-20260924-023558-127，保留父记录原件。新版本仍待用户按 BEHAVIOR_TEST.md 的“先验证本次进入修复”复测；未宣称已确定唯一实机回调故障或实际登场已成功。

## v5 / DLL 0.3.1

Spec 1.2 / Design 1.5。v4 登场获用户确认，四轮正常 Active 后退场仍触发 8 秒超时；记录 exit-notification-v4.json。实现原版/四向配置、两处原版 1011 到 Behbnd clip trigger 的转换、HKS 实际节点完成确认及 Native 主动完成接收。详见 research/c0010-behavior-adapter-v5.md。自动检查只证明离线契约；新图事件的 ER 投递及实际演出仍待人工。

2026-09-24 03:01 确认游戏退出后完成十文件事务部署，源码 c14f05031dcc3d5d4f7775164e2a23f9c80a2533。DLL SHA256 50f0a3287bebefc8c05893bb4d3960eb475deceeaada598e143c8ada101cd119。十个目标、十个对应备份、七个保护输入核对通过；备份 F:\GoldenAge\_GoldenAge\dll\demon_beast\_backups\complete-v3-20260924-030148-254。新增 Restore-C0010V5.cmd 以完整退回 v4/DLL 0.3.0；旧 deployment-v3.json 不改，当前 v5 会被旧回退的哈希守卫拒绝，先恢复 v4 后其 v2 回退才重新适用。新回退未实际执行；脚本语法与备份哈希检查通过不代表游戏恢复验收。未启动游戏。

## v6 私有 HKS 退场修正

用户报告 v5 无退场演出；日志 outro:active 与 done 同帧，定位到混合期间旧 Idle 活动被误判为退场完成。outro-blend-v5.json 保留失败信号。新增多活动节点回归，v5 红、v6 绿；进入、动作、连段、普通解除、战技、移动配置回归与 Lua 语法检查通过。DLL 0.3.1 / Behbnd 不变，沿用 v5 的原生与图检查；本轮不重新声称已完成实机验证。

已在游戏退出时完成 v6 十文件事务（实际差异仅 HKS 与四份版本/测试文档），十个安装目标、对应备份、七项保护输入均核对通过。私有 HKS SHA256 4f7260b9b8a2f431fff07590222dfcc5776d81000e463d9605761893ae7c2d7e；备份 F:\GoldenAge\_GoldenAge\dll\demon_beast\_backups\complete-v3-20260924-212537-383。Restore-C0010V6.cmd 退回 v5，DLL/PDB/配置/Behbnd 原哈希不变。回退仅做语法与备份哈希检查，未实际执行；游戏验收待用户。

### v6 退场人工确认

用户反馈“现在有变身结束动画了”。run 1790257785590-44536 的末轮从退场独占、与待机混合到退出退场后 done，Finishing 95.544 -> Restoring 96.610 -> Normal 96.674，记录原生析构完成且该轮未走超时恢复。证据 research/outro-human-check-v6.json。仅标记可见退场项人工 PASS；不把完整攻击等待、锁定移动、六组正式 Probe 或未来多形态配置补报通过。没有修改已安装文件。

## v7 连按闪避输入卡住修复候选

用户明确为兽体与动作输入卡住，游戏/镜头继续运行；两轮最终 behavior_finish_timeout。新增真实 HKS seam 重放重复 Step 未重新激活以及 Idle/Step 重叠两条路径，v6 均红、v7 均绿；20 轮重复请求/下一有效闪避与 F7 退场通过，原进入、动作、连段、退场、战技和配置回归及 Lua 语法通过。Failure Packet 为 research/roll-input-stall-v6.json。运行仍待人测，不把离线图回调重放当作唯一运行根因已证实。

用户确认退出游戏，安装器亦检查进程不存在后完成 v7 十文件事务。十个安装目标、十个对应备份、七个保护输入核对通过。HKS SHA256 68f20aeb1238d70ddbfcccc534112f04f6fe173abd70683fdad629d10f7f8459；备份 F:\GoldenAge\_GoldenAge\dll\demon_beast\_backups\complete-v3-20260924-220224-452。新增 Restore-C0010V7.cmd，恢复到退场已确认的 v6；DLL/PDB/配置/Behbnd 未变，原生与图验证沿用既有证据。回退只检查语法/备份哈希未实际执行，未启动游戏。

### v7 连按翻滚人工回归通过

用户反馈“测试了一下没有问题”。run 1790258643664-15076 已完成的一轮中，frame 298 拦下仍活动 Step 的重复请求，309/400 继续执行闪避、438 接轻击 3010；活动满 15 秒后正常退场，t=65.584 回到 Normal，原生析构完成且该轮无 finish_timeout。随后一次重新进入 Active，采样时尚未完成，不计第二轮完整恢复。记录 research/roll-human-check-v7.json。本轮反馈记为人工通过，不补报所有正式 Probe。保持已安装包不变。
