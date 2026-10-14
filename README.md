# ER Demon Beast

为 **Elden Ring** 提供可配置的角色变身功能：保留玩家本体，生成并操控独立形态，在解除后恢复玩家控制。

当前版本：**0.4.1**。当前动作适配器面向 **c0010**。

## 功能

- **多形态配置**：分别设置角色 ID、NpcParam、NpcThinkParam、优先级和持续时间。
- **SpEffect 控制**：通过两个自定义效果分别请求变身和解除，可接入游戏道具、战技或动画，让键鼠和手柄共用游戏原有操作。
- **独立时长**：从形态可操作时开始计时；`-1` 取消到期计时，仍保留解除效果和战技原有的自动解除。
- **动作与连段映射**：配置轻击、重击、防御、战技、闪避及连段目标，沿用游戏语义输入与衔接窗口。
- **完整退场流程**：常规解除等待当前动作和退场动画，切换形态时先清理旧实体，再生成新实体。
- **镜头与移动配置**：每个形态可设置镜头距离、高度及锁定时的四向移动选项。
- **可选调试热键**：F6 请求进入或选择形态，F7 请求解除；正式 SpEffect 入口不依赖这些热键。

## 环境与资源

- Windows x64，Elden Ring **App 1.17.1 / WW 2.7.1**。DLL 会检查目标程序版本，不匹配时拒绝初始化。
- 支持加载原生 DLL 的 Mod 加载器，例如支持原生插件的 ME3 配置。
- 对应角色的模型、动画、行为文件和参数，以及与本项目适配器配套的私有 HKS。

仓库包含 DLL 源码和 c0010 HKS 适配器，不包含游戏模型、原版脚本、动画、`regulation.bin` 或完整角色资源包。`mapped` 模式要求配套的 c0010 行为资源；仅修改角色 ID 不会自动完成其他角色的动作适配。

## 构建与加载

安装 Rust 工具链和 Visual Studio C++ 构建工具，在仓库目录执行：

```powershell
cargo build --release --features native --locked
```

输出文件：

```text
target/x86_64-pc-windows-msvc/release/er_demon_beast.dll
```

将 DLL 和 `er-demon-beast.toml` 放在同一目录，并在 Mod 加载器中加载该 DLL。仓库配置默认关闭 `enabled` 和 `control_enabled`，完成角色资源和参数配置后再启用。配置更改需要完全重启游戏。

`assets/action/script/c0010-er-adapter.hks` 是追加到兼容公共脚本副本中的适配器，不是独立替换所有公共脚本的完整 HKS。c0010 的 Behbnd 应引用生成的私有 `d0010.hks`，并具备对应的进入、动作衔接和退场节点。

当前版本的 DLL/HKS 行为通信使用固定共享目录。迁移安装位置时，应同步调整 `src/native.rs` 的桥接目录与 HKS 的 `ERDB_BRIDGE_DIR`，重新编译 DLL 并生成私有脚本。两端必须使用同一目录。

## SpEffect 配置

以下为单个 c0010 形态的配置示例。**12345、12346 是占位 ID**，应替换为自己的有效 SpEffect；程序不会自动创建这些参数行。

```toml
enabled = true
control_enabled = true
debug_hotkeys_enabled = false
action_trace = false
ownership_tag = 1987040701
default_form = "demon_beast"

[[forms]]
key = "demon_beast"
enabled = true
chr_id = 10
npc_param_id = 100000
npc_think_param_id = 100000
trigger = "automatic"
sp_effect_id = 12345
revert_sp_effect_id = 12346
priority = 0
duration_seconds = -1
action_mode = "mapped"
locked_four_way_movement = false
camera_distance = 7.0
camera_height = 2.6
```

使用前需要准备相应的 NpcParam / NpcThinkParam 行，并由游戏动作施加约定效果：

- **进入效果**只检测玩家本体。
- **解除效果**检测玩家本体或当前受控形态，不检测无关 NPC。
- 效果从不存在变为存在时触发一次；持续存在不会重复触发，移除再出现才会重新触发。
- 进入效果消失不会自动解除变身。
- 解除请求优先于同时出现的进入请求，并取消尚未开始的切换。
- 当前按 100ms 间隔读取效果；效果及两次触发之间的移除间隔需要足够长，建议至少 0.5 秒，避免使用仅存在一帧的标记。

键鼠和手柄通过同一个游戏动作施加效果即可使用；单独填写效果 ID 不会自动绑定某个游戏按钮。

完整配置示例见 [SpEffect 控制](examples/sp-effect-controls.toml) 与 [多形态配置](examples/custom-forms.toml)。

## 形态选项

| 字段 | 说明 |
|---|---|
| `key` | 唯一形态名称 |
| `chr_id` | 角色编号，例如 `10` 对应 c0010 |
| `npc_param_id` / `npc_think_param_id` | 对应角色参数行 |
| `trigger` | `automatic`：效果出现时进入；`manual`：匹配效果后由调试 F6 请求进入 |
| `sp_effect_id` | 本体上的进入或选择效果；automatic 必填 |
| `revert_sp_effect_id` | 当前形态的解除效果；可省略，不得与启用形态的进入效果冲突 |
| `priority` | 多个效果匹配时取最高值；启用形态之间不能重复 |
| `duration_seconds` | 正数为持续秒数，`-1` 为不计时 |
| `action_mode` | `mapped` 使用 c0010 适配器；`native` 使用角色原生控制 |
| `locked_four_way_movement` | `false` 保留原版移动逻辑；`true` 需要对应方向动画，只适用于 mapped |
| `camera_distance` / `camera_height` | 形态镜头距离、高度的最低值，解除时恢复 |

默认形态由根配置 `default_form` 指定。非默认的启用形态需要进入效果 ID。`direct_c0000` 是保留模式，目前不支持直接运行完整 c0000 脚本。

## 输入与招式映射

在对应 `[[forms]]` 下定义输入来源和起手动作：

```toml
[forms.inputs]
light = 0
heavy = 1
guard = 2
skill = 3
dodge = 5

[forms.actions]
light = 3000
heavy = 3005
guard = 3030
skill = 3035
dodge = 6000
```

输入编号代表游戏的轻击、重击、防御、战技、闪避语义，沿用游戏当前绑定；输入设为 `-1` 或动作设为 `0` 可禁用对应映射。有效输入来源不能重复。

当前适配动作包括 `3000`、`3001`、`3002`、`3003`、`3005`、`3007`、`3010`、`3030`、`3035`、`6000`。自定义连段可使用：

```toml
[[forms.combos]]
from = 3000
light = 3001
heavy = 3005
```

省略 `combos` 会保留默认连段；自定义时需列出希望保留的轻重击衔接。不支持把任意动画编号直接作为已适配动作。

## 许可证

本项目采用 [MIT License](LICENSE)。第三方代码保留其原始许可证，详见 [THIRD_PARTY.md](THIRD_PARTY.md)。

## Building with ShikiFSRS

Cargo pins `KamiyamaShiki0704/ShikiFSRS` to `fb9f4524da1fb648e0a696f2d47aa08d3acf35cc`. `.cargo/config.toml` enables Git CLI fetching; no token is stored in this repository.

The migration was checked offline against ER WW 1.17.1 (EXE 2.7.1.0). Native engine ownership, phase/lifetime requirements and unexecuted gameplay behavior remain experimental and require manual acceptance. Existing ignored game/asset fixtures were not executed.
