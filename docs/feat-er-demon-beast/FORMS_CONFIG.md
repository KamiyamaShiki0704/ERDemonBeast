# 多形态配置 0.4.1 / 行为 v8

依据已确认的 spec-custom-forms.md@1.0 和 spec-effect-controls.md@1.0。无限时长只取消到期计时，保留原版战技解除、效果请求解除和异常恢复。实机结果仍由用户验收；以部署记录确认当前安装版本。

配置模板：项目根 er-demon-beast.toml。旧平铺的 chr_id 等字段移入 [[forms]]；默认形态 key=demon_beast。修改后需完全重启游戏。

| 字段 | 含义 |
|---|---|
| default_form | 默认形态 key，必须启用；可为 automatic，调试F6无效果匹配时使用它 |
| debug_hotkeys_enabled | 根配置项，默认false；只有true时才启用调试F6/F7，效果控制不依赖它 |
| forms.key / enabled | 唯一名字与启用开关 |
| chr_id / npc_param_id / npc_think_param_id | 目标角色、对应 ER 参数行；资源必须另行具备 |
| trigger | manual：匹配效果时按 F6；automatic：效果新出现时自动请求 |
| sp_effect_id | 玩家本体用于选择的效果 ID；默认形态可省略，其他启用形态必须提供 |
| revert_sp_effect_id | 每形态可选解除效果ID，出现在本体或当前受控兽体上时请求解除；不能与任何启用形态的进入ID相同 |
| priority | 数值越大越优先；启用形态不得相同 |
| duration_seconds | 正数秒或 -1；从可操作开始计时，-1 无计时到期，战技仍按原版解除，解除效果也可解除 |
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

## 正式游戏操作与手柄入口

进入与解除由两个独立效果控制，ID不写死在DLL中。游戏的道具/战技/动画负责施加效果；DLL只检测并执行变身生命周期，不直接绑定新的键盘键或手柄按钮。键鼠与手柄执行同一个已接线的游戏动作即可触发。

1. 在根配置设置 `debug_hotkeys_enabled = false`（不要放入某个 `[forms.inputs]` 子表）。
2. 在目标 `[[forms]]` 中设置 `trigger = "automatic"`、实际 `sp_effect_id` 与实际 `revert_sp_effect_id`。下面示例12345/12346仅占位，必须替换，程序没有新增这两行。
3. 进入动作把进入效果施加到玩家本体。解除动作把解除效果施加到玩家本体或当前受控兽体；两者任一均有效，无关NPC不算。
4. 完全退出并重启游戏加载配置，分别用键鼠与手柄测试动作。

```toml
# These lines belong inside your existing [[forms]], before [forms.inputs].
trigger = "automatic"
sp_effect_id = 12345          # placeholder: replace with real enter effect
revert_sp_effect_id = 12346   # placeholder: replace with real exit effect
duration_seconds = -1
```

完整且默认禁用的示例见 `examples/sp-effect-controls.toml`。配置ID并不自动让某个游戏按钮施加效果；需要对应参数/动作接线。ID尚未确定的安装迁移配置仍显式保留 `debug_hotkeys_enabled=true`、原manual形态和15秒供过渡测试；填好ID并接线后再改automatic并关闭调试键，不要把过渡配置当作已完成手柄入口。

检测按效果出现边沿触发：持续存在只触发一次，移除再出现才重触发；进入效果消失不解除。解除优先于同时进入，并取消待切换；解除仍等动作及退场。进入只认本体效果，兽体效果不会意外选择另一形态。两来源分别记录出现边沿。

当前每100ms采样一次，不能保证收到仅存在一帧的效果。接线时建议信号至少保持0.5秒，重触发前留出可被观察的消失间隔；不要用永不消失的标记做连续开关。此建议不替代实机测试。

## 添加另一形态

复制一个 [[forms]] 段并修改 key、priority、sp_effect_id、时长和目标身份。先用 enabled=false 编辑；效果 ID 必须换成你实际使用的 SpEffect。程序只读取该效果，不替你创建参数行或施加效果。examples/custom-forms.toml 提供禁用的占位示例，不能原样当作实机效果配置。

多个匹配效果始终按 priority 选一个。自动效果持续存在时，解除后不会立即重进；必须移除再施加。F6 可显式再次进入。切换要等旧实体清理，等待中 F7 取消下一形态。效果消失不会自动解除当前形态。

显式启用debug_hotkeys_enabled时，F6/F7为两个调试请求键：F6进入/选择，F7解除。F6不是开关，同一形态活动中再按不会解除或重新计时。F7在正常活动期等待当前动作和退场；进入阶段F7会中止进入。退场/清理期间F6不排队，必须恢复本体后松开再按。长按不会连发，两个键同时新按下时F7优先；游戏需在前台。效果路径也要求游戏在前台。

mapped 当前只能用于 c0010；其他角色可先用 native 验证生成、基础控制与清理，其攻击行为和输入映射需要对应适配器。不能把复制 chr_id 等同于完成新角色移植。

## 离线校验与人工验证

在项目目录执行 cargo run --offline --features native --example check_forms -- er-demon-beast.toml 可检查配置，不启动游戏。

安装后先按 spec-custom-forms.md 的 CF-P01..06 回归；效果ID与施加动作准备好后按 spec-effect-controls.md 的 EC-P01..06 验证键鼠/手柄、两来源解除、优先级与持效去重。镜头/模型/实际命中由人检查，离线通过不替代实机。
