# 原版魔兽输入规则与 0.2.2 实测（2026-09-24）

来源：本机 NR action/script/c9997.hks、c0010.hks 经 DSLuaDecompiler 离线反编译，全文保留在 input/c0010-action-audit/c9997-nr.lua、c0010.lua；下列行号对应这些文件。只读核对，没有运行游戏。

用户补入 GA/action/script/c0010.hks 后重新读取：35407 bytes，SHA256 BA4D2C96119B8A9986009192C2B4E08B4A45BFB7DA9A601F5A8075EEFB5C0A1F，与本机 NR 原件逐文件哈希一致。此前“缺 c0010.hks”是旧状态，本轮后不得继续当作现状。该文件的原版事件回调依赖 c9997 中的 TDB 函数，不能据单个文件存在宣称输入规则完整。

## 原版规则

- c9997 L513 IsTDB：当前角色具有 SpEffect 707100。测试用 ER NpcParam[100000] 当前没有这一常驻效果；GA c9997 虽有 IsTDB 辅助函数，但没有 NR 的完整 TDB 函数族。
- L7897 IdleDefault_onUpdate / L7908 IdleBattle_onUpdate：IsTDB 分支进入 IdleCommonFunction_TDB。
- L1233 GetAttackRequest_TDB：读取 env(1106, ACTION_ARM_*)；按 R1、ULTRA、R2、L1、L2 顺序选请求。R2 且 ACTION 持续时间 >0 时转成 ULTRA。输入是游戏语义请求，不是固定鼠标键位。
- L1270 ExecAttack_TDB：精力 <=0 时清请求并拒绝出招；精力耗尽且 StaminaComboResetTest=1 时置连段重置标记。有效请求按当前状态提供的 r1/r2/l1/l2 选择动作，ExecAttack 发出 W_Attack 加编号；ULTRA 发 W_Event20010。空中分支只接受 R1/R2 攻击，不接受 L1/L2。
- L6371 IdleCommonFunction_TDB：基础映射 R1=3000，R2=3005，L1=3030，L2=3035。L6399 AttackCommonFunction_TDB 为连段、跳跃、闪避与移动提供共同转换；TAE/引擎输入队列决定何时有可消费请求。
- L8147/8170/8193/8216：轻击链 3000→3001→3002→3003→3000；这些状态的 R2 均为 3005。
- L8251：3005 状态中 R1=3001，R2=3007；连段重置时改为 3000/3005。L8285：3007 状态回到 R1=3000、R2=3005。
- L8330：3010 状态中 R1=3001、R2=3007，重置时 3000/3005。
- L8968 Step6000_onUpdate：闪避状态的 R1=3010，R2=3005。L1315 ExecEvasion_TDB 消费 ROLLING，发 W_Step6000。
- L927 ExecJump_TDB：CHANGE_STYLE 请求或 AI 跳跃请求；MoveSpeedLevel>=0.75 发 W_Jump_TDB_R，0<level<0.75 发 W_Jump_TDB_F，否则发 W_Jump_TDB_N。对应落地还有独立的 TDB 状态，不能只复制起跳事件。
- c0010.lua L1 Event20000_onUpdate 调用 AttackCommonFunction_TDB(3000,3005,3030,3035,...)，事件动画结束后发 W_Idle；主要攻击/移动/输入规则在共享 c9997 中。

## NR 与 ER 输入编号不是完全相同

| 请求 | NR c9997 常量 | ER 固定 fsrs ChrActions |
|---|---:|---:|
| R1/R2/L1/L2 | 0/1/2/3 | 0/1/2/3 |
| ACTION / SP_MOVE / CHANGE_STYLE(Jump) | 4/5/6 | 4/5/6 |
| ULTRA | 17 | 无同语义已证实项；ER 17 是 ROLLING |
| BACKSTEP | 18 | 16 |
| ROLLING | 19 | 17 |

这也是不能整份复制 NR c9997 的具体原因：数值相同不等于接口语义相同。移植应保留原版状态到招式规则，分别适配 ER 输入、取消窗口、精力、移动/跳跃及解除生命周期。

## 0.2.2 人工测试证据

运行 1790179759050-37584，DLL SHA256 63D42284E9BE348E49521BAD750A5D14D7B49417E3D49C972024545A2A38B176。677 活动帧全部带 action_trace，丢帧 0，两个变身周期均恢复并析构，耗时约 0.051 秒。

兽体 raw/new/readback 输入出现 1/2/4/32，即 ER R1/R2/L1/SP_MOVE；本体输入始终 0。兽体 npc_action/readback_npc_action 始终 -1，请求队列 TAE 仅 -1/9500。证实输入到达兽体，排除“键鼠未进入兽体”的分支；npc_action=-1 本身不是错误，因为原版 TDB 可以直接派发行为事件。TAE 队列字段也不是完整动画播放证据。

分析命令：`python Scripts/delivery/feat-er-demon-beast/script/audit_actions.py F:/GoldenAge/_GoldenAge/dll/demon_beast/Saved/er-demon-beast/1790179759050-37584/frames.jsonl`。原始采集、meta 和 action-report.json 均在该 run 目录。

项目内的最小固定映射草案已移至 input/c0010-action-audit/experimental，未交付。其原始共同脚本 dispatch 基线能复现收到 R1 不派发；草案基本映射测试推进至动作结束场景，但该场景的引擎环境桩仍不足，不能称为通过。用户补入原版后，下一阶段按本文原版状态规则做兼容适配，不覆盖用户刚提供的原件；尚未修改 GA、DLL 或 regulation，也未宣称攻击修复。
