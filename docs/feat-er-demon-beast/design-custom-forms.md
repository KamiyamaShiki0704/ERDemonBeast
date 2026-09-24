# 多形态配置设计候选（未发布）

依据 spec-custom-forms.md@0.1 中已确认的部分；无限时长与战技解除仍待选择，候选不安装。

- forms::Config/Form：严格 TOML、稳定 key、全局唯一 ownership_tag、每个启用形态唯一 priority。原单形态配置明确要求迁移，不静默丢弃字段。默认 c0010，正数秒到期或 None 表示无计时。
- forms::Selector：维护已观察效果集合及最多一个待切换索引。只有 Normal 可产生 enter；Active 请求另一形态转为 graceful revert，Finishing/Restoring/Fault 禁止重复进入，F7 清待切换目标。效果读取失败保留上次已见集合，不能当作消失而意外重装自动触发。目标失败后清队列，不重复重试。
- session：仅 Normal 接受新时长，Active 从 ready 时刻计时；无到期时间不使用超大秒数模拟。恢复/析构保持现有流程。
- native：FrameEnd 10 Hz/手动进入时采样玩家本体 SpecialEffect；验证本体身份、容器 owner、链头稳定，以 ReadProcessMemory 复制链节点，最多1024节点并拒绝循环。仅新采样身份且 Normal 时重置边沿状态。后台窗口不产生新形态请求，但已有会话的到期/清理仍运行。
- ABI 来源为 Cargo 固定 fromsoftware-rs 02fa568… 的 eldenring/cs/sp_effect.rs 与 chr_ins.rs，程序同时锁定既有 ER EXE SHA256。SpecialEffect head+8/owner+0x10，Entry param_id+8/next+0x30/removal+0x40；公开字段 offsetof 增加启动验证。只读本体效果，不调用 NR 引擎函数，也不扫描所有进程。
- 行为桥：在 nonce 和 enter/exit/closed 后携带 v2 固定文本，inputs/actions 五整数及最多十条 combo 行，最多2048字节；HKS 不执行配置文本。Legacy 两/三行命令用于回归兼容，不用于新配置偷偷回退。
- 映射：inputs 为游戏语义来源（-1禁用），actions 为起手目标（0禁用），combos 为当前动作到轻/重目标。现阶段白名单为当前 c0010 的10个已适配动作。缺少combo行只禁用该轻重衔接，不移除防御/闪避等其他窗口。重复目标节点活动时拒绝新派发，防止任意可配置输入复现 v7 闪避锁死模式。
- action_mode=mapped 目前要求 c0010 的私有适配器；native 允许其他 chr_id 使用引擎原有控制，但拒绝假装支持输入/四向覆写；direct_c0000 明确拒绝，保留扩展点。
- Capture 记录所选 form_key/duration；启动保存完整配置。人工启动工具保持嵌套形态配置，missing 模式只破坏默认形态参数用于失败测试。

## 离线与人工边界

Rust 覆盖配置拒绝、排序、持效重装、未知采样、切换/清理/失败不重试、F7取消、有限与60秒无限计时；实际 Native 调度与内存采样仍需人工游戏验证。Lua 加载真实私有/common/原版文件，使用由 Rust 导出的同一协议检查映射、禁用、连段窗口与非法文本，保留已通过的退场/翻滚回归。图未改，故沿用 v5 图往返证据。没有宣称其他 ID 的战斗动作已适配，也未新增任意 SpEffect 行。
