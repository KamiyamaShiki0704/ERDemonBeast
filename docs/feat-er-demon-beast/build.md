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
