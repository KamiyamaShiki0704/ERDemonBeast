# c0010 键鼠动作无响应：2026-09-24

当前结论：未修复玩法。用户确认变身、移动、转向可用，但其他按键无效果，动画异常。0.2.2 是取证版本，不应标记 SPEC-ERDB-002 PASS。

## 已有证据

- 源码基线 798fa8d，Spec 1.0 / Design 1.3。用户要求实机测试由人执行；代理只读磁盘、构建和离线测试。
- 0.2.1 现场 capture run `1790178283214-25840`（DLL 目录 Saved/er-demon-beast 下）：三个 Active 区间都退出并析构，恢复约 0.051 秒；无正式 Case。用户确认此次使用键鼠。
- DLL 将 WorldChrManDbg.debug_manipulator 关联兽体，未将兽体当作 PlayerIns，也没有直接移交 c0000 HKS。因此“看起来像玩家动画”尚不能证明正在执行 c0000 脚本。
- GA/action/script/c0010.hks 缺失。安装的 c0010.behbnd Characters/c0010.hkx 引用 c9997.hks、modifier.hks、c0010.hks。
- 已离线反编译本机 NR c0010/c9997.hks，结果放 input/c0010-action-audit（不分发游戏脚本）。NR IdleDefault/IdleBattle 的 IsTDB 分支调用 IdleCommonFunction_TDB，后者经 GetAttackRequest_TDB 把 R1/R2/L1/L2 转换为 W_Attack3000/3005/3030/3035。GA 通用脚本没有这些 TDB 动作函数。
- 专用测试 NpcParam 清除了常驻效果，当前无 NR 707100 识别标志；行为参数仍是 ER 狼的基础引用。直接添加 NR 标志不能补齐 ER 缺少的控制器、脚本、TAE 和伤害参数适配。

## 离线反馈入口与边界

```powershell
F:/Lua/luacdes/5.3/bin/lua.exe Scripts/delivery/feat-er-demon-beast/script/check_c0010_dispatch.lua F:/GoldenAge/_GoldenAge/GA/action/script/c9997.hks raw-r1
F:/Lua/luacdes/5.3/bin/lua.exe Scripts/delivery/feat-er-demon-beast/script/check_c0010_dispatch.lua F:/GoldenAge/_GoldenAge/GA/action/script/c9997.hks ai-3000
```

实际输出：raw-r1 consumed=0 events=，攻击断言 FAIL；ai-3000 consumed=1 events=W_Attack3000，PASS。此为真实 GA HKS 的 ExecAIAction 调用，env/act 是离线桩。证明两个入口并不等价，不能证明调试控制器在游戏中实际提供了哪个请求，也不能替代用户症状的端到端复现。

按证据依次区分：①键鼠攻击未进入调试通道；②原始请求存在但未转换为 NPC 动作；③NPC 动作存在但行为/角色脚本未推进。FrameEnd 请求可能在此前已消费，零值本身不能排除②③。物理键与用户改键后的语义也不能混为一谈。

## 0.2.2 定向取证

配置新增 action_trace，默认 false。交付 c0010 配置开启：只在 Active 期间逐 FrameEnd 记录，补充动作读回、输入允许/禁用掩码、NPC 动作读回和移动/TAE 取消标志。游戏处于前台时，仅记录鼠标左右键、Shift/Ctrl/Space 的按下状态，不读取文本输入；没有增加引擎写入或新模块指针链。普通状态维持原采样策略，有界写队列保留丢帧计数。

新增 audit_actions.py 保守汇总信号，不给玩法 PASS。修复日志收集仅收 manual-runs 的遗漏，直接从现有加载器启动的 Saved 日志也能打包。

人工反馈入口见交付 ACTION_TEST.md。代理不启动或操作游戏。下一步依据这次人工捕获确认入口，再做 c0010 专属适配；不全局替换 GA 的通用 c9997，不将 NR 引擎专用脚本直接当作 ER 兼容实现。
