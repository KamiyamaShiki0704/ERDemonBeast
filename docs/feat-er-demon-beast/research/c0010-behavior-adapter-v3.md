# c0010 行为 v3 / DLL 0.3.0 修复记录

## 输入与故障证据

用户确认 v2 的五项动作可执行，报告锁定四方向前移、连点抢断、到期截断、缺少过渡和镜头仍似玩家，并明确 F7 和到期都等待动作结束。最近实际运行 1790183660130-51800 的 HKS 日志出现 3005→3007 间隔约 6 帧；不将其解释为合法动作完成。

只读源：固定 ER 2.7.1 EXE、已安装 GA 私有/原版 HKS、c0010 anibnd/behbnd，以及研究中的 NR 演出流程。没有启动游戏或检查实时进程内存。

## 定位与修复

1. common ExecMove 无 AI 目标分支把非跑步方向统一选成 WALK_FRONT。玩家锁定与 AI 认知不等价。v3 显式选战斗方向表；真实 HKS 模拟左/右/后输入，v2 失败、v3 通过。
2. env1107 是全局 cancel-ready，不自带动作身份。增加当前动作所有权、实际激活后的游戏 DT 下界、Fire 到 onActivate 之间的占有保护。窗口从当前 TAE 的 115/116/16/103/26/11 标记核对；必须同时满足 ER cancel 信号。模拟混合期间旧取消信号，v2 抢断、v3 拒绝；窗口后保留连段。
3. DLL 增加 Finishing，F7/到期同一流程。私有脚本等待当前动作、请求 Event20010，动画实际运行后结束才 done。超时/失联走失败恢复，不作为正常完成。旧 nonce/部分消息不能确认新的变身。
4. Event20000/20010 两个 CMSG 关闭 TAE 防止 NR 引擎专有触发，脚本确认真实动画结束。完整 c0000 起手和相关特效仍未移植。这两段的原有 TAE 特效、声音也未执行。
5. 运行时 LockCamParam 最低距离/高度覆盖及有条件原值恢复。未修改 regulation；特殊临时镜头仍未证明覆盖。

## 验证与重建

证据 verification-v3.json；详细资产窗口/哈希在 behavior-build-v3.json。prepare_c0010_v3.ps1 接收当前 v2 基线、adapter、全新输出目录，完成提取、两个字段修改、HKX 往返语义比较、binder 三个未变子文件逐字节验证、10 个 TAE 窗口核对与私有 HKS 生成。behavior_v3.py 拒绝基线哈希漂移。

cargo test --offline --features native：25 PASS；Python unittest discover：16 PASS；fmt/clippy/release/64KiB DLL 线程加载回归 PASS。Lua 实际公共脚本 + 原始 c0010 两次加载：五动作、九连段、三种 chunk 名、精力、死亡、IO 错误、锁定方向、激活间隙/全局旧取消、进入/攻击等待/退场确认通过。图转换后所有 XML 语义一致，只有原始图的两处 enableTae 改为 false。

## 未证明与人工项

实际 IsLockon/MoveAngle 来源、CMSG 激活与结束回调、第一轮加载时序、最终方向/连段手感/镜头都需实机。游戏 DT 下界未证明变速与 hitstop 的帧精度。文件握手仅限当前单 c0010 测试环境；不是后续多形态正式接口。8 秒等待超时是故障兜底，不能掩盖退场失败。完整玩家起手、视觉特效、TAE 转换、伤害/碰撞仍不在本候选通过声明中。
