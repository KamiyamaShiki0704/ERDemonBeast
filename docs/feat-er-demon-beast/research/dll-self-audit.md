# DLL 加载崩溃自检 — 2026-09-23

结论：**已确认 DLL 自身存在加载阶段栈溢出，已修复并通过离线复现回归；另一例游戏本体访问异常仍未归因。不能宣布 DLL 已具备游戏内稳定性。**

用户最新指令为“你先别测了……先自检一下是不是 dll 有问题”。此后仅读取既有转储、检查源码、构建和运行独立测试宿主；没有再次启动游戏。0.1.2 默认为 disabled，未部署到 GoldenAge。

## 已证实的根因

- 原 src/native.rs 的 DllMain 在 `reason == DLL_PROCESS_ATTACH` 分支中声明 `[u16; 32768]`，约 64KiB。
- 优化后的入口在检查 reason 之前执行 `mov eax, 0x10090; call __chkstk; sub rsp, rax`。因此每次 DLL_THREAD_ATTACH/DETACH 也承担同样栈开销。
- 游戏转储 `eldenring.exe.55524.dmp`：异常 `0xC00000FD`，位置 `er_demon_beast.dll+0x74047`，正在 __chkstk 探测栈页；RDX=2（DLL_THREAD_ATTACH），RAX=0x10090；栈内存在返回 DllMain+0xD 的地址。
- [crash-55524.json](crash-55524.json) 固定了异常上下文及指令。该文件中的 stack_candidates 是匹配模块的原始栈字，不冒充完整 unwind 调用栈。

独立宿主 `Scripts/delivery/feat-er-demon-beast/script/dll_thread_smoke.rs` 用 LoadLibrary 加载 DLL，再用 CreateThread 创建 64KiB 保留栈。未启动游戏，EXE 哈希保护阻止插件注册游戏任务。结果：

| DLL | 结果 |
|---|---|
| 原 0.1.0，SHA256 B9BAF5…688 | 栈溢出，退出 -1073741571 / 0xC00000FD |
| 原 0.1.1，SHA256 937A50…211 | 栈溢出，同一退出码 |
| 改为堆缓冲区后的 0.1.1 | PASS，线程返回 42，宿主退出 0 |
| 当前 0.1.2 | PASS，同一回归程序 |

修复：DllMain 路径缓冲区改为堆上的 Vec，不在游戏创建的每个线程栈上放置 64KiB 数组。没有通过关闭线程通知掩盖问题（静态 CRT 仍正常接收通知）。

## 第二例不能混同

[crash-18256.json](crash-18256.json) 对应堆缓冲区修复后的 **enabled=false** 启动。异常是 `eldenring.exe+0x1F109C4` 处 `mov [rcx+0x18], rax`，RCX=0xB，试图写地址 0x23，属于访问异常而非栈溢出。这次配置没有注册本项目游戏任务、采集或控制逻辑。

现有资料不足以判定其根因：不把 EXE 内崩溃自动归为游戏/其他 Mod，也不把栈溢出修复当作它的修复。之前无本 DLL 的几个转储是不同异常位置，见 [crash-comparison.json](crash-comparison.json)，不能据此宣称同一故障原已存在。没有额外启动游戏来复现它。

## 初始化与运行路径审查

1. disabled 分支：EXE 文件哈希、静态 ABI、配置和诊断日志之后返回，无 WorldChrMan 访问、CSTask 注册或游戏对象写入。DllMain 本身仍会收到系统线程通知，所以原栈溢出不受 enabled=false 阻止。
2. enabled 分支：版本绑定后等待系统/CSTask，注册 ChrIns_PostPhysics 回调；任务句柄保持至进程退出。依赖的 RecurringTask 不支持安全热卸载，继续禁止热卸载。
3. 新增采集原先在尚未开始 Case 时也访问本体/动作模块；已改为普通状态无请求无 Case 则直接返回。历史帧中出现过地图标识为 m255_255_255_255 的临时本体，这不足以证明游戏场景已就绪。
4. 本项是降低加载/菜单阶段风险的防护，**并无证据证明它是第二例崩溃原因**。正式 Case 和控制期间仍依赖绑定库的类型布局与引擎实体生命周期，离线 ABI/字段测试不能完整证明指针有效性和线程时序。
5. 恢复时保留 handle/owner 校验、字段掩码和控制关联所有权检查；未新增 NPC 析构、main_player 替换或未经验证的伤害内存位。
6. 分析脚本对缺失/截断/丢帧作 INCONCLUSIVE，原始未开始的六组 Probe 均为 NOT_RUN；状态数据不等于完整功能通过。

## 本轮离线验证

- Rust：9 项通过（6 生命周期 + 3 ABI/内存事务）。
- Python 分析器：6 项通过（含空记录、截断、丢帧、字段未还原和重复 Case）。
- 小栈 DLL 宿主：旧包失败、新包通过，1 项针对真实根因的回归。
- cargo clippy --all-targets -D warnings、fmt --check、release 构建通过。
- 测试边界：小栈宿主不会进入 ER 任务注册/角色采集路径。后者不能以本轮离线 PASS 替代实机结论。

## 保留与下一步

0.1.2 仅保存在项目 dist；0.1.1 的运行配置已改回 enabled=false。用户指定 `F:/GoldenAge/_GoldenAge` 和直接使用 ER0000.Shiki 的决定保留，但实机暂停指令优先。原始 Shiki 备份位于 `input/save-backups/20260923-014131/ER0000.Shiki`，SHA256 `399499294E3CEB094CF84940F23FCC0D2009C3D95FF6499F6729C9A3BFAEB506`。

第二例访问异常与六组功能 Probe 均保持未解决/未验收。后续只在用户恢复游戏测试后，才建立最小加载组合进行验证；本轮不自动重开。
