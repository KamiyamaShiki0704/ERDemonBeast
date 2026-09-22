---
id: FEAT-ER-demon-beast
title: 首轮原生候选构建与离线验证
type: build
version: 1.0
updated: 2026-09-23
based_on:
  - spec.md@1.0
  - design.md@1.0
code_paths: [src/lib.rs, src/session.rs, src/native.rs, tests/lifecycle.rs]
project_paths: [Cargo.toml, Cargo.lock, er-demon-beast.toml, prototype.me3]
---

# Build 结果

自动结论：**PARTIAL**。离线构建与检查通过；六组游戏内 Probe 均未运行，最终人工验收待定。没有安装到现用 GoldenAge profile、启动游戏或写运行中游戏内存。

## 输入与版本

- Spec：`spec.md@1.0`；设计：`design.md@1.0`。
- 源码 revision：首次实现提交后记录于当前工作包。
- 工程 KB revision：`9511e3e8137fd2cda9fca74bb07a4b688fe89087`。
- NR 机制 KB revision：`8119ee174b685a39806a243a9c5acfbc5dbd912b`，作为研究来源，不是本项目实现。
- Runtime dependency：fsrs `02fa5681e27fd2ecd9da79d34aef9fae96805539`；其它依赖固定在 Cargo.lock。
- 工具链：rustc/cargo 1.96.0；目标 x86_64-pc-windows-msvc。
- 目标 EXE：WW 2.7.1.0，SHA256 `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`。

## 实现

- 生命周期：普通、进入、活动、恢复、故障；3 秒进入/恢复超时；从接管成功开始计算 15 秒活动期。
- 进入失败和活动期失效都进入清理；恢复失败保留所有权并阻止新建，恢复完成才允许下一次进入。
- 原生适配：游戏线程任务中生成 c4070、检查参数存在性和角色归属，接入 ER 调试控制器与相机覆盖；不替换 main_player。
- 私有字段只在目标 EXE 哈希完全匹配、ABI 与身份检查通过时访问。跨帧重新解析句柄并核对角色、控制器、模块及子模块地址/owner。
- 保存并按掩码恢复本体保护字段，控制关联仅在仍属于本能力时恢复。未使用公开代码中的未知 data+19B 位及本体 ChrType 覆盖。
- 恢复后隐藏兽体并复用；不调用实体析构。已发出的异步创建即使超时仍保留清理责任，迟到结果不能重新激活。
- 加载配置默认为 `enabled=false`，仅作版本/ABI 诊断。独立候选 profile 为单机配置并使用独立存档名。

## 已执行的自动检查

| 检查 | 结果 | 能证明的范围 |
|---|---|---|
| `cargo test --locked --features native` | 9 passed, 0 failed | 6 个模拟后端生命周期测试；3 个 ABI/当前测试进程内存事务测试 |
| `cargo clippy --locked --features native --all-targets -- -D warnings` | PASS | 编译与静态 lint；不证明游戏内接口语义 |
| `cargo fmt -- --check` | PASS | Rust 格式 |
| `cargo build --release --locked --features native` | PASS | 实验 DLL 可构建 |
| `check_target.py <本机EXE>` | PASS | EXE SHA256、控制覆盖 getter/dispatch、镜头覆盖读取的磁盘字节匹配 |
| PE 检查 | PASS | AMD64 DLL、DllMain 导出、无 VCRUNTIME/MSVCP 动态依赖 |

离线生命周期检查包括：延迟生成不提前隐藏本体或扣活动时长；超时到达的结果不激活；十轮重入/恢复；创建/活动失败清理；恢复超时后持续重试；同时按键和加载中解除。

字段事务检查只在测试进程自己分配的内存上执行，验证保留无关位与不覆盖后来接管的控制者，没有访问游戏进程。

## 候选产物

- `dist/er-demon-beast-0.1.0/er_demon_beast.dll`，607744 bytes。
- DLL SHA256：`B9BAF5AEE5354FC40CEA3376B748D257D9CB747CE42452D3F7DB1E07FF1AC688`。
- 同目录包含 PDB、配置、独立 `prototype.me3`、TESTING.md、THIRD_PARTY.md。
- 操作和恢复说明：[TESTING.md](../../TESTING.md)。产物在忽略目录内，本地保留。

## Spec 与未证明项

| Spec | 当前证据 | 运行状态 |
|---|---|---|
| SPEC-ERDB-001 | 进入事务/超时/失败清理逻辑及 ABI 检查 | NOT RUN：真实创建、本体保护和控制接管 |
| SPEC-ERDB-002 | 控制与镜头读取通道静态证据 | NOT RUN：移动、输入映射、攻击命中及镜头手感 |
| SPEC-ERDB-003 | 反向恢复、字段掩码与失效身份保护 | NOT RUN：真实 3 秒恢复和 ≤1m 位置误差 |
| SPEC-ERDB-004 | 15 秒边界模拟测试 | NOT RUN：游戏调度下的实际计时容差 |
| SPEC-ERDB-005 | 十轮模拟循环、拒绝重入 | NOT RUN：实际实体数量与重复使用稳定性 |
| SPEC-ERDB-006 | 失败后清理模型和参数检查路径 | NOT RUN：真实无效兽体/异步创建故障情形 |

取证缺口：KB 的 FrameRecorderProbe 目前只有 GPUECli/Unreal 入口，尚无 ER 原生适配。本轮没有建立替代 Recorder。当前诊断日志不能替代指定的运行证据；需补适配或由用户明确调整本项目运行取证要求后继续自动多帧验证。

附加限制：不支持跨地图继续该实验会话；会话身份变化可能进入需要退出进程的故障态。该行为属于当前隔离候选的限制，不是完整魔兽化异常恢复能力。异步创建不返回时也会保留故障态，不能报告 SPEC-ERDB-006 已满足。

## Evidence References

- [control-audit.md](research/control-audit.md)：固定公开源码、精确目标二进制与关键反汇编，解释控制和镜头通路。
- [target-2710.json](research/target-2710.json)：本轮可重复的静态目标检查。
- `tests/lifecycle.rs` 与 `src/native.rs` 测试模块：本轮已执行的离线测试入口。
- Recorder Markdown：无；`meta.report_path`：无；最终人工 PASS/FAIL：未给出。
