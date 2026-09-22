---
id: FEAT-ER-demon-beast
title: DLL 自检与原生采集构建
type: build
version: 1.1
updated: 2026-09-23
based_on: [spec.md@1.0, design.md@1.1]
code_paths: [src/lib.rs, src/session.rs, src/native.rs, src/native/capture.rs, tests/lifecycle.rs]
project_paths: [Cargo.toml, Cargo.lock, er-demon-beast.toml, prototype.me3]
---

# Build 结果

自动结论：**PARTIAL**。已确认 DLL 入口小栈线程溢出，修复版通过离线回归。另一例游戏本体访问异常未归因。用户要求暂停实机后未再启动游戏；六组正式 Probe 均 NOT_RUN，最终验收 pending。

## 输入、版本与产物

- Spec 1.0；Design 1.1；用户明确批准本项目 ER 原生采集例外。
- 源码 revision：`4f877655983854e84f2f49a339889ee25ab63821`。
- 工程 KB revision：`9511e3e8137fd2cda9fca74bb07a4b688fe89087`；NR 研究 KB revision：`8119ee174b685a39806a243a9c5acfbc5dbd912b`。
- fsrs：`02fa5681e27fd2ecd9da79d34aef9fae96805539`；其余依赖固定 Cargo.lock；Rust 1.96.0 / x86_64-pc-windows-msvc。
- 目标：ER WW 2.7.1.0，EXE SHA256 `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`。
- 最新离线候选：`dist/er-demon-beast-0.1.2/er_demon_beast.dll`，800768 bytes。
- DLL SHA256：`1EC7F9EF2305F5CBA81F5BEAF93E89478EC19A46115CA307C55631B7FD489FD8`；默认 enabled=false，未部署、未运行游戏验证。
- 精确源码哈希、入口指令与构建信息：[build-result-0.1.2.json](research/build-result-0.1.2.json)。

## 改动与核验

保留已有生命周期、原生 c4070 控制事务及回滚。增加获批的原生 JSONL 采集、run/id 绑定命令与保守分析报告；新增 DLL 入口小栈回归宿主和离线 dump 诊断入口。

实机加载发现 DllMain 栈上 32768 个 u16 的缓冲区在 reason 判断前就分配，导致游戏线程初始化栈溢出。改为堆缓冲区；优化后二进制入口栈分配由 0x10090 降到 0xA0。另在普通状态、未开始 Case 且无请求时停止遍历角色，避免无必要读取加载/菜单阶段的临时对象。该防护不等于第二例访问异常的已证明修复。

| 检查 | 结果 | 证明边界 |
|---|---|---|
| Rust tests | 9 PASS | 生命周期、ABI 与测试进程内事务 |
| Python analyzer tests | 6 PASS | 空记录、截断/丢帧、未恢复、重复 Case 不会误报完整通过 |
| 64KiB DLL thread smoke | 旧 0.1.0/原 0.1.1 均 C00000FD；新 0.1.2 PASS | 真正经过 LoadLibrary/系统 DLL_THREAD_ATTACH，不进入 ER 引擎 |
| Clippy / fmt / release build | PASS | 编译与静态质量 |
| PE/入口检查 | AMD64；入口不再分配 64KiB | 不是游戏运行稳定性证明 |
| 原生加载日志/采集 | 0.1.1 能安装任务并产生状态文件 | 不是正式 Probe；不代表角色控制已通过 |

完整故障分类、原始 dump 摘要、复现命令和风险见 [dll-self-audit.md](research/dll-self-audit.md)。所有离线测试与构建在用户要求停止游戏测试后完成；没有追加游戏启动。

## 运行环境与暂停状态

用户指定 `F:/GoldenAge/_GoldenAge`，并明确要求直接使用 ER0000.Shiki。测试前已备份存档，路径及哈希在自检报告中。原 Start_GoldenAge.me3 未改；临时 Start_ERDemonBeast_Offline.me3 已被外部移除，未重建。0.1.1 运行配置已恢复为 disabled。

第一类崩溃：本 DLL +0x74047 / C00000FD，证据充分，离线修复回归通过。第二类：EXE +0x1F109C4 / C0000005，发生在本 DLL enabled=false 的运行，未归因；不能将它自动归于其他 Mod，也不能宣称已修复。

已生成但未开始任何 Case 的采集报告：

- `dist/er-demon-beast-0.1.1/Saved/er-demon-beast/1790098817780-42724/report.md`
- `dist/er-demon-beast-0.1.1/Saved/er-demon-beast/1790098892822-55148/report.md`

两份报告六组均 NOT_RUN，meta.report_path 可定位。001 进入保护、002 控制/攻击/镜头、003 手动恢复、004 到期恢复、005 十轮重入、006 不可用兽体恢复均保留原 Spec 阈值，仍待实机与人工验收。

后续范围：只在用户恢复实机测试后继续最小组合诊断和六组 Probe。当前不会自动重开游戏。跨地图、NR 资产、完整伤害结算、异步创建永久不返回等限制保持原设计约束。
