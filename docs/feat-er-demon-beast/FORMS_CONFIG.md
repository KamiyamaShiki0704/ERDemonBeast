# 多形态配置 0.4.0 / 行为 v8

依据已确认的 spec-custom-forms.md@1.0。无限时长只取消到期计时，保留原版战技解除、F7 手动解除和异常恢复。实机结果仍由用户验收；以部署记录确认当前安装版本。

配置模板：项目根 er-demon-beast.toml。旧平铺的 chr_id 等字段移入 [[forms]]；默认形态 key=demon_beast。修改后需完全重启游戏。

| 字段 | 含义 |
|---|---|
| default_form | 无效果匹配时 F6 使用的形态 key，必须为启用的 manual 形态 |
| forms.key / enabled | 唯一名字与启用开关 |
| chr_id / npc_param_id / npc_think_param_id | 目标角色、对应 ER 参数行；资源必须另行具备 |
| trigger | manual：匹配效果时按 F6；automatic：效果新出现时自动请求 |
| sp_effect_id | 玩家本体用于选择的效果 ID；默认形态可省略，其他启用形态必须提供 |
| priority | 数值越大越优先；启用形态不得相同 |
| duration_seconds | 正数秒或 -1；从可操作开始计时，-1 无计时到期，战技仍按原版解除，F7也可解除 |
| action_mode | mapped：现有 c0010 适配器；native：原生角色控制；direct_c0000 暂不支持 |
| locked_four_way_movement | false 沿用原版，true 要求四方向动画存在；仅 mapped 有效 |
| camera_distance / camera_height | 当前形态镜头最低距离与高度，解除恢复原值 |

## 输入与动作

inputs 表中的值是 ER 的语义输入编号，沿用游戏当前键鼠/手柄绑定：0轻击、1重击、2防御、3战技、5闪避；-1禁用。不同映射不能共用同一来源。比如交换轻重键：

```toml
[forms.inputs]
light = 1
heavy = 0
guard = 2
skill = 3
dodge = 5
```

actions 表配置起手动画：light=3000、heavy=3005、guard=3030、skill=3035、dodge=6000 为当前默认；0禁用。支持的动画仅为 3000/3001/3002/3003/3005/3007/3010/3030/3035/6000。不接受随意存在于 anibnd 中但未适配的其他动作。

省略 combos 保留现有连段树。自定义时列出希望保留的各行；未列出的当前动作不再有轻重衔接，但其他动作仍按原窗口判断：

```toml
[[forms.combos]]
from = 3000
light = 3001
heavy = 3005

[[forms.combos]]
from = 3001
light = 3002
heavy = 3007
```

若需要全部禁用轻重连段，把 combos=[] 放在该 [[forms]] 下、[forms.inputs] 之前。禁用新输入不等于已经运行的动画立刻停止。正常 F7/到期仍等待动作和退场。

## 添加另一形态

复制一个 [[forms]] 段并修改 key、priority、sp_effect_id、时长和目标身份。先用 enabled=false 编辑；效果 ID 必须换成你实际使用的 SpEffect。程序只读取该效果，不替你创建参数行或施加效果。examples/custom-forms.toml 提供禁用的占位示例，不能原样当作实机效果配置。

多个匹配效果始终按 priority 选一个。自动效果持续存在时，解除后不会立即重进；必须移除再施加。F6 可显式再次进入。切换要等旧实体清理，等待中 F7 取消下一形态。效果消失不会自动解除当前形态。

F6/F7 当前为两个独立请求键：F6进入/选择，F7解除。F6不是开关，同一形态活动中再按不会解除或重新计时。F7在正常活动期等待当前动作和退场；进入阶段F7会中止进入。退场/清理期间F6不排队，必须恢复本体后松开再按。长按不会连发，两个键同时新按下时F7优先；游戏需在前台。

mapped 当前只能用于 c0010；其他角色可先用 native 验证生成、基础控制与清理，其攻击行为和输入映射需要对应适配器。不能把复制 chr_id 等同于完成新角色移植。

## 离线校验与人工验证

在项目目录执行 cargo run --offline --features native --example check_forms -- er-demon-beast.toml 可检查配置，不启动游戏。

安装后按 spec-custom-forms.md 的 CF-P01..06：先默认 c0010 回归，再5秒/15秒到期、60秒无限和移除效果、无限时长下战技原版解除、效果自动/手动及优先级、连续切换/取消/失败恢复、输入互换与禁用。镜头/模型/实际命中由人检查，离线通过不替代实机。0.4.0 尚未安装时不要拿旧 DLL 测新配置。
