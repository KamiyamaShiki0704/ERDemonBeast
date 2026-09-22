# Prototype 0.1.0 — 测试说明

## 目前已验证的范围

本项目已具备可编译的原生 DLL 与独立生命周期逻辑。离线单元测试验证状态机及字段事务；磁盘核验验证目标 EXE 和关键控制指令；这些结果不证明游戏内功能通过。

仅接受本机核验的 ER WW 2.7.1.0，EXE SHA256：
`1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891`。
DLL 启动时计算实际进程 EXE 哈希，不匹配则拒绝注册游戏任务。

## 构建与检查

在项目根运行：

```powershell
cargo test --locked --features native
cargo clippy --locked --features native --all-targets -- -D warnings
cargo fmt -- --check
cargo build --release --locked --features native
python Scripts/delivery/feat-er-demon-beast/script/check_target.py '实际游戏路径/eldenring.exe'
```

必须使用 `--features native` 生成游戏 DLL；不带该特性只构建离线生命周期库。构建目标为 x86_64-pc-windows-msvc，静态 CRT，保留调试信息。

## 隔离候选包

候选目录：`dist/er-demon-beast-0.1.0/`。包含 DLL、PDB、TOML 配置、`prototype.me3` 和本文；现用 GoldenAge 配置未修改。

默认 `enabled = false`：通过加载器载入后只检查 EXE/ABI 和配置、写启动诊断，不注册输入或游戏任务。这个默认值用于将“能加载”与“进行控制实验”分开。

准备执行人工单机试验时，将候选包内 TOML 的 `enabled` 改为 `true`，退出并通过独立 `prototype.me3` 重新启动。该 profile 只加载本项目 DLL、使用独立存档名并设置单机；profile 的加载表现本轮未实测。不要并行加载其他接管 NPC 或共享 debug creator 的插件。测试结束退出游戏即可移除 profile，运行中不热卸载 DLL。

## 人工试验顺序（尚未执行）

前置：平坦且没有环境伤害的场景，角色存活、不骑乘、不处于菜单或剧情控制中。默认测试兽体为 c4070 Wolf；配置 NpcParam/NpcThinkParam 行缺失会拒绝进入。

1. 先确认默认关闭模式的日志包含 `ABI verified; enabled=false`，且没有 `game task installed`。
2. 启用并重启后，确认日志包含 `game task installed`。F6 请求进入；F7 请求恢复。仅在游戏窗口前台时接收按键边沿。
3. 先只测试进入和立即恢复，检查本体可见性、控制及镜头。出现异常时退出本次测试进程，保留日志，不追加其他输入试验。
4. 按 Spec 六组 Probe 验证移动/攻击、15 秒自动恢复、手动恢复、重复进入与十轮复用；失败兽体配置需重启单独验证。
5. 对比活动前后本体位置、生命与控制状态，人工记录镜头、攻击表现和是否有残留可见兽体。

F6/F7 只负责进入/退出；兽体移动与攻击依赖 ER 调试控制器的原生映射。具体按键映射尚未实机证明，不承诺与最终 NR 魔兽一致。

## 证据与已知限制

- DLL 同目录的 `er-demon-beast.log` 只记录诊断、转换和失败原因，**不是 FrameRecorderProbe 报告**，不能当作六组 Probe 自动通过证据。
- 当前 KB 的公共 FrameRecorderProbe 仅找到 GPUECli/Unreal 用法，ER 原生适配缺失；未启动自动多帧测试或创建替代 Recorder。
- 受伤结算、联机、传送、切图、死亡和最终 NR 资产不在首轮验收范围。上下文或实体身份变化时停止新建，保留清理状态；无法安全还原旧内存时需退出测试进程。
- 已发出的异步创建无法仅靠超时宣称取消。DLL 会继续等待归属兽体并隐藏迟到结果；如果引擎一直不返回结果，会保持故障态阻止重复创建。
- 恢复后保留至多一个隐藏兽体用于复用，没有调用未在本项目验证的实体析构。
- 本项目只使用已命名保护字段，未照搬公开代码的未知 data+19B 位或把本体 ChrType 改成 NPC。保护的实机效果仍待验证。
