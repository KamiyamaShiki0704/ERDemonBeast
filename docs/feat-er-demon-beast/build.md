---
id: FEAT-ER-demon-beast
title: Summon 框架整合与人工测试候选
type: build
version: 1.2
updated: 2026-09-23
based_on: [spec.md@1.0, design.md@1.2]
code_paths: [src/lib.rs, src/session.rs, src/native.rs, src/native/capture.rs, src/vendor/summon/PROVENANCE.md]
project_paths: [Cargo.toml, Cargo.lock, er-demon-beast.toml, MANUAL_TEST.md]
---

# Build 结果

自动结论：**PARTIAL**。0.2.0 离线检查通过，人工测试包已准备。按用户要求，本轮没有启动或操作游戏。第二例 EXE+1F109C4 访问异常尚未证明根因；六组正式 Probe 均无本版本实机通过结果。

## 固定输入和产物

- Spec 1.0，Design 1.2；源码 revision `32b177e17ffa21bdbf2f199b095d556477c064ee`。
- 工程 KB revision `9511e3e8137fd2cda9fca74bb07a4b688fe89087`；NR 研究 KB revision `8119ee174b685a39806a243a9c5acfbc5dbd912b`。
- fsrs `02fa5681e27fd2ecd9da79d34aef9fae96805539`；Summon 复用来源 `01491edfa709545786d0fe795ab2fb2458dca4e1`，原文件及 fixture 的哈希固定于 vendor provenance。
- ER WW 2.7.1.0，EXE SHA256 `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`。
- 产物 `dist/er-demon-beast-0.2.0/er_demon_beast.dll`，828928 bytes。
- DLL SHA256 `F3FC865F9FE0F29CD1114400F6E32B0A02B04C11D9567E591464A74EADC20F3F`。
- [构建清单](research/build-result-0.2.0.json) 绑定源码、依赖、DLL 和检查结果。

## 本轮改动

复用 Summon 的实体 ID 注册与两阶段退出/原分配器释放；确认创建归属后才接管。恢复本体后等待帧末注销任务、集合脱离与引用排空，清理完成前拒绝重入。操作统一到严格匹配的 FrameEnd，OwnedPtr 解引用前增加模块可读性/owner 检查。保留已有 DllMain 小栈修复。

新增 load/observe/control 分离的手动入口，每次启动生成包内独立 run，强制单机并备份用户指定的 ER0000.Shiki。默认只加载本 DLL 与 GA 资源；兼容性入口再增加其他 DLL，排除重复召唤生成器。原 GoldenAge profile 和 Summon 工程均未修改。

## 验证

| 检查 | 结果 | 范围 |
|---|---|---|
| Rust tests | 17 PASS | 状态机、内存事务、模块拒绝、FrameEnd 编码与 Summon 原生/分配器/引用排空协议 |
| Python tests | 8 PASS | 报告保守判定、手动配置、备份与原文件不改动 |
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

人工验证顺序为仅加载、仅观察、最小控制、完整插件组合，再按 Spec 进行六组 Probe。最终 PASS/FAIL 由用户给出。
