# 效果控制设计

输入：spec-effect-controls.md@1.0，用户已确认双效果且ID自定义。

- Config增加默认false的debug_hotkeys_enabled，仅控制F6/F7物理键入口；离线Probe命令保留既有独立通道。Form增加可省略的revert_sp_effect_id，正整数，与所有启用形态的进入效果冲突时拒绝。解除效果可由多个形态共享。默认形态允许automatic；automatic仍必须提供进入效果ID。
- Selector独立记住本体/兽体已见效果，避免两个来源交替出现掩盖边沿。只有当前形态的解除ID有效；Normal对将进入的候选应用解除优先规则。读取失败保持对应seen，身份变化重置对应源。退场中解除取消queued。与输入无关，手柄和键鼠共用效果路径。
- 进入仅采样玩家本体。解除采样本体和本项目当前持有的兽体；兽体必须通过WorldChrMan句柄和完整Identity校验，且退休队列尚未接管。Normal/Restoring/Fault不解析兽体。无关NPC不枚举。
- 两个来源分别返回有效、未知或无兽体；本体失败不伪造空集合，兽体失败不阻止有效本体信号。只读ChrIns::special_effect，复用已固定ABI和有界链表读取。读取容器后重新验证manager指针/owner/head；不将兽体当PlayerIns，不新增写内存地址。
- 保留100ms采样；信号应保持到有效采样，建议接线时保持至少0.5秒且再次触发前有可观察的移除间隔。此为配置建议，不声称短脉冲无损捕获。
- F6/F7只有debug_hotkeys_enabled=true才提供输入；disabled时仍更新物理按下状态，防止配置/焦点边沿误触。已有动画完成通知、HKS、Behbnd和实体清理不变。
- 0.4.1交付不猜测真实SpEffect ID或修改regulation/动画。现有已安装manual测试形态显式保留debug_hotkeys_enabled=true直到用户接线；模板默认false并提供清晰迁移步骤。

自动检查：自动默认形态校验、非法/冲突ID、两源独立边沿与读失败、Normal解除优先、退场取消queued、本体/兽体解除经Session等待动作清理、关闭调试键与效果入口独立、原生命周期回归；release与64KiB加载。游戏施加效果与手柄操作为人工项。
