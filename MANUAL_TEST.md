# ER Demon Beast 0.2.0 — 人工测试

这一版由你手动启动和验证。代理本轮没有启动游戏。已修复确认的加载栈溢出，并接入固定版本的 Summon 清理源码；游戏本体的另一例访问异常尚不能凭离线检查宣布消除。

## 测试包与环境

包目录：`D:\_Ugit\FSTool\_Mod\_Project\er-demon-beast\dist\er-demon-beast-0.2.0`。

启动脚本读取 `F:\GoldenAge\_GoldenAge\Start_GoldenAge.me3`，使用该目录的 ME3 与 GA 资源、当前 Steam ER EXE，强制单机。按你的选择直接使用 `ER0000.Shiki`；每次启动前自动备份到包内 `save-backups/<本次运行>/`，不覆盖之前备份。

前三步只加载本 DLL 与 GoldenAge 资源，不加载其他原生 DLL；第四步才加入 GoldenAge 其他 DLL。第四步仍排除 Summon.dll，以免两个独立状态机争用同一 debug creator；本 DLL 内已复用它的生命周期与内存释放源码。原 Start_GoldenAge.me3、Summon.dll、summon.toml 都不会改动。

每次运行使用独立 `manual-runs/<时间>-<模式>/`，配置、DLL、日志互不混淆。开始下一步前完整退出上一轮游戏。若任何一步崩溃，停在该步并保留证据，无需继续后面的步骤。

## 按顺序双击

| 步骤 | 入口 | 操作与预期 |
|---|---|---|
| 1 | `01-load-only.cmd` | 只加载 DLL，两个开关都关闭。到标题画面等待 30 秒，再加载角色，正常走动 30 秒后退出。预期不崩溃；日志显示 `enabled=false control_enabled=false`，没有安装游戏任务。 |
| 2 | `02-observe-only.cmd` | 注册 FrameEnd 回调，但禁止召唤与控制。重复上一步；可按一次 F6，预期只记录拒绝输入，角色不变。日志应有 `first callback ... frame_end_accepted=true`。 |
| 3 | `03-control-minimal.cmd` | 开启控制。进入平地、角色存活、不骑乘、不在菜单或剧情中；先找没有敌人攻击的位置。F6 进入，2 秒后 F7 解除。检查本体、镜头和移动恢复，再等 5 秒。成功后再测 15 秒自然到期、移动/一次攻击及重复使用。 |
| 4 | `04-control-goldenage.cmd` | 第三步稳定后再执行；加入 GoldenAge 其他 DLL（不含 Summon.dll），重复单次 F6/F7。用于区分本功能问题与插件组合问题。 |

首轮兽体仍为 ER 原生 **c4070 Wolf**，不是 NR 魔兽外观。F6 请求进入，F7 请求恢复；15 秒从控制接管成功开始计算。清理期间再次进入会被忽略。

第三步优先看以下日志顺序：

```text
owned beast ready
control linked
... Active -> Restoring ...
summon retirement: ... phase=detached ... tasks_unregistered=6
summon retirement: ... phase=draining ...
summon removed: ... outcome=destroyed retirement=frame-end-drained
... Restoring -> Normal ...
```

“本体出现”不等于实体内存已经释放，应同时看到 `outcome=destroyed`。若出现 `retirement quarantined`、`restore_timeout`、`ownership uncertain` 或生成器占用，停止这一轮并退出；这表示程序拒绝继续操作不确定的对象，不能作为通过结果。

## 日志怎么给我

游戏退出后双击 **`05-collect-logs.cmd`**，会在包目录产生 `erdb-logs-<时间>.zip`，包含每次启动日志、DLL 日志、配置与原生采集报告，不包含存档、DLL 或 PDB。

你在当前任务告诉我 **第几步、在标题/读档/按 F6/按 F7/到期中的哪一刻失败，以及有没有出现狼** 即可；日志留在本机，我可以直接读取。若 Windows 生成了转储，位置通常为 `%LOCALAPPDATA%\CrashDumps\eldenring.exe.<进程号>.dmp`，也可告诉我文件名。

## 六组正式 Probe（基础加载稳定后）

基础步骤用于定位崩溃，不能冒充六组 Spec 自动通过。需要精确报告时，游戏已进入符合条件的场景后，在包目录 PowerShell 中开始对应 Case：

```powershell
.\Probe-Case.ps1 -Action start -Case 3 -Note '填写实际场景及已满足的前置'
# 切回游戏，按下述对应步骤输入
.\Probe-Case.ps1 -Action end
```

| Case | 输入与观测 |
|---|---|
| 1 | F6，3 秒内进入；安排测试敌人实际尝试攻击本体，检查不可见、无输入响应、不掉血。 |
| 2 | F6 后分别向前、侧向移动并转动镜头，输入后 1 秒内响应；对近处敌人攻击一次，5 秒内观察命中掉血与因果。 |
| 3 | F6 后移动，再 F7；3 秒内恢复、水平位置误差 ≤1 米，恢复后原地静置至少 6 秒再 end。 |
| 4 | F6 后不按 F7，等待自动恢复；活动 15±0.25 秒触发，18.25 秒内恢复，再观察 5 秒。 |
| 5 | 连续十轮 F6/F7；每轮活动后额外按 F6 三次，按键之间留出明确松开间隔；每次等 `Normal` 后再开始。末次恢复后静置至少 11 秒再 end。 |
| 6 | 用 `06-missing-beast.cmd` 单独启动，开始 Case 6 后 F6。该入口使用参数行 2147483647；预期日志通过实际查找报告 `missing configured NpcParam/NpcThinkParam`，3 秒内保持普通状态；随后移动和攻击再 end。若未报告参数缺失，则此 Case 的不可用兽体前置未成立，不能算通过。 |

同一进程内，每组 Case 只开始/结束一次；重跑同组请重启，以免合并失败尝试。收集日志时自动生成报告；未标记的 Case 为 NOT_RUN，缺数据为 INCONCLUSIVE，客观不符为 FAIL，自动子检查通过最多为 AUTOMATIC_PARTIAL。镜头、输入表现、命中归因及最终 PASS/FAIL 仍由你判断。

暂不测试切图、传送、死亡、坠落或联机。当前整合尚未在本项目实机验收，Summon 既有验证不能替代本项目结果。
