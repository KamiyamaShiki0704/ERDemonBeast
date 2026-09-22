# Prototype 0.1.2 — 离线自检候选

**当前暂停所有游戏内测试。** 用户要求先自检加载崩溃；0.1.2 仅完成离线回归，未部署或启动游戏验证。不要把旧候选继续用于实机试验。崩溃证据与边界见 [自检报告](docs/feat-er-demon-beast/research/dll-self-audit.md)。下面的实机流程仅供以后用户恢复测试时使用。

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
python -m unittest discover -s Scripts/delivery/feat-er-demon-beast/script -p test_probe.py -v
rustc Scripts/delivery/feat-er-demon-beast/script/dll_thread_smoke.rs -o target/dll-thread-smoke.exe
./target/dll-thread-smoke.exe "$PWD/target/x86_64-pc-windows-msvc/release/er_demon_beast.dll"
```

必须使用 `--features native` 生成游戏 DLL；不带该特性只构建离线生命周期库。构建目标为 x86_64-pc-windows-msvc，静态 CRT，保留调试信息。

## 隔离候选包

候选目录：`dist/er-demon-beast-0.1.2/`。包含 DLL、PDB、TOML 配置、`prototype.me3` 和本文。修复 DLL 入口在游戏小栈线程上溢出，并停止未开始 Case/未请求能力时的角色遍历。

用户指定运行环境为 `F:/GoldenAge/_GoldenAge`，并明确选择直接使用 ER0000.Shiki；已备份到项目 input/save-backups/。先前临时 Start_ERDemonBeast_Offline.me3 已被外部移除，本项目未重建。原 Start_GoldenAge.me3 未修改。恢复测试时应新建专用入口，引用该目录 GA 包、强制单机，并明确处理与 Summon 共用 debug creator 的关系。ER EXE 仍由该环境的启动器指向本机 Steam Game 目录。

默认 `enabled = false`：通过加载器载入后只检查 EXE/ABI 和配置、写启动诊断，不注册输入或游戏任务。这个默认值用于将“能加载”与“进行控制实验”分开。

准备执行单机试验时，将候选包内 TOML 的 `enabled` 改为 `true`，退出并通过上述 GoldenAge 专用入口重新启动。项目内 prototype.me3 只用于最小化对照。测试结束先 F7 恢复，再退出游戏；运行中不热卸载 DLL。

## 实机 Probe 操作

前置：平坦且没有环境伤害的场景，角色存活、不骑乘、不处于菜单或剧情控制中。默认测试兽体为 c4070 Wolf；配置 NpcParam/NpcThinkParam 行缺失会拒绝进入。

1. 先确认默认关闭模式的日志包含 `ABI verified; enabled=false`，且没有 `game task installed`。
2. 启用并重启后，确认日志包含 `game task installed`。F6 请求进入；F7 请求恢复。仅在游戏窗口前台时接收按键边沿。
3. 先只测试进入和立即恢复，检查本体可见性、控制及镜头。出现异常时退出本次测试进程，保留日志，不追加其他输入试验。
4. 按 Spec 六组 Probe 验证移动/攻击、15 秒自动恢复、手动恢复、重复进入与十轮复用；失败兽体配置需重启单独验证。
5. 对比活动前后本体位置、生命与控制状态，人工记录镜头、攻击表现和是否有残留可见兽体。

开始 Case 前确认所有语义前置，并记录实际地图/场景。在项目根执行（这里以 003 为例）：

```powershell
$probe = 'Scripts/delivery/feat-er-demon-beast/script/probe.py'
$candidate = 'dist/er-demon-beast-0.1.2'
python $probe command $candidate start --case 3 --note '填写实际地图及平地、无环境伤害、存活、不骑乘、无菜单/剧情等前置'
# 游戏内 F6，移动，F7；恢复后原地静置超过 5 秒
python $probe command $candidate end
$meta = Get-Content "$candidate/capture-latest.json" | ConvertFrom-Json
python $probe report $meta.frames_path
```

命令文件有 run/id 防重放保护；start/end/mark/enter/revert 均记录接受或拒绝的回执，发送成功不等于执行成功。enter/revert 只在 Case 开始且游戏前台时接收。自动命令的游戏执行时间以 frames.jsonl 为准。每个进程的每组 Case 仅运行一次；同组重跑重新启动，以免合并不完整尝试。

输出在候选目录 Saved/er-demon-beast/&lt;run&gt;/。meta.json 绑定 EXE/DLL 哈希、配置和构建清单，meta.report_path 定位报告。没有开始的 Case 为 NOT_RUN；采集缺失为 INCONCLUSIVE；客观不符为 FAIL；自动子检查通过最多为 AUTOMATIC_PARTIAL，人工项与最终结论仍待人确认。

F6/F7 只负责进入/退出；兽体移动与攻击依赖 ER 调试控制器的原生映射。具体按键映射尚未实机证明，不承诺与最终 NR 魔兽一致。

## 证据与已知限制

- 用户已明确批准本项目采用 ER 原生多帧采集和分析替代 FrameRecorderProbe。证据主体为 frames.jsonl 与分析报告；简短 er-demon-beast.log 只用于诊断。
- 可见性字段、相机指针及目标 HP 不能单独证明画面、镜头表现或攻击因果；这些必须对应真实输入和人工观察。
- 受伤结算、联机、传送、切图、死亡和最终 NR 资产不在首轮验收范围。上下文或实体身份变化时停止新建，保留清理状态；无法安全还原旧内存时需退出测试进程。
- 已发出的异步创建无法仅靠超时宣称取消。DLL 会继续等待归属兽体并隐藏迟到结果；如果引擎一直不返回结果，会保持故障态阻止重复创建。
- 恢复后保留至多一个隐藏兽体用于复用，没有调用未在本项目验证的实体析构。
- 本项目只使用已命名保护字段，未照搬公开代码的未知 data+19B 位或把本体 ChrType 改成 NPC。保护的实机效果仍待验证。
