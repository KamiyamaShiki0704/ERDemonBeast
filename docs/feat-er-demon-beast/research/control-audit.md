---
id: FEAT-ER-demon-beast
title: ER 调试控制链源码与 2.7.1 静态审计
type: research
version: 0.1
updated: 2026-09-23
source: source-and-binary-audit
related: []
---

# 结论与证据边界

问题：公开 NPC 控制实现是否能成为独立兽体的输入、镜头和恢复接口？

结论：存在可追踪的 **WorldChrManDbg 调试控制链**。公开实现关联调试 manipulator、目标 ChrCtrl 的 override 槽和镜头覆盖角色，未将 NPC 写入 `WorldChrMan.main_player`。本机 2.7.1 的只读反汇编进一步证实了 override 优先选择和镜头覆盖读取；这足以支持继续设计，但不证明兽体已能移动、攻击或安全解除。公开实现的旧 `ChrIns+0x530` 不可直接移植到当前依赖布局。

方法：固定公开 commit，读取源码、许可证；比较两个本地 Rust revision；对指定游戏磁盘文件做签名扫描和局部反汇编。没有注入、启动游戏、写游戏内存或取得运行时 Probe。下文把源码事实、二进制静态事实和待证语义分开。

## 固定来源

- 公开源 E：`AlpinDale/er_enemy_control@8056f10c0864e3e10d6c20cc85827676fac36ab4`，由 `git ls-remote ... refs/heads/master` 定位后按 SHA 读取。[dllmain.cpp](https://github.com/AlpinDale/er_enemy_control/blob/8056f10c0864e3e10d6c20cc85827676fac36ab4/src/dllmain.cpp)；[LICENSE](https://github.com/AlpinDale/er_enemy_control/blob/8056f10c0864e3e10d6c20cc85827676fac36ab4/LICENSE) 为 Apache-2.0。本文只记录接口调查，未复制其实现。
- 本地源 A：`D:/_Ugit/FSTool/_Mod/_Project/fromsoftware-rs@3c8c1d7633a99309fb004c9f894ea10b7967d0e0`，本文检查的四个 `cs` 文件无工作树修改。该树虽在 `rva.rs` 声明支持 2.7.1，仍不能据此推定每个结构布局已适配。
- 本地源 B：`C:/Users/33333/.cargo/git/checkouts/fromsoftware-rs-7e356ae17759562a/02fa568@02fa5681e27fd2ecd9da79d34aef9fae96805539`，工作树仅有 Cargo 的未跟踪 `.cargo-ok`；父任务选定的 2.7 系列依赖。以下本地行号默认指 B，路径前缀为 `crates/eldenring/src/`。
- 二进制 X：`F:/SteamLibrary/steamapps/common/ELDEN RING/Game/eldenring.exe`，父任务读取 FileVersion `2.7.1.0`；本次独立计算 SHA256 `1a3547101327f65d0c76da2f9190ac0aa66871ea42bae2aecc61e11a8b597891`。静态 VA 以 PE ImageBase `0x140000000` 为基准；运行地址必须重新加实际模块基址。

## 公开变量实际对应的对象

E 的名称有误导性。按其解引用和 B 的布局交叉比对：

| E 的名字/偏移 | 实际对应或合理解释 | 依据与限制 |
|---|---|---|
| `world_root` | WorldChrMan | E L1301–1317；X 的 world 签名后读取 `+1E508`；B `cs/world_chr_man.rs` L58–59 为 PlayerIns 类型的 main_player |
| `actor_mgr` | 玩家 ChrIns/PlayerIns | E 从 WorldChrMan `+10EF8` 再解引用得到角色；不是一个新 manager。应改由已验证的 main_player 入口取本体 |
| `actor_ctrl = *(actor_mgr+190)` | ChrInsModuleContainer | B `cs/chr_ins.rs` 的 module_container；`cs/chr_ins/module.rs` L41–57，data 位于 `+0`，physics 位于 `+68`。它不是 `ChrCtrl` |
| `player_root` | WorldChrManDbg | B `cs/world_chr_man_dbg.rs` L69–71 分别是 `+A8 debug_manipulator`、`+B0 player_session_holder`、`+B8 cam_override_chr_ins`；X 独立印证该 singleton 的 `+B8` 读取 |
| `link_a = *(player_root+A8)` | 调试 manipulator 指针 | E 随后写其 `+A8 = target`；B `cs/chr_manipulator.rs` L8–28 的 owning_chr 对应尾部指针，但具体派生类型与生命周期仍需确认 |
| `target58 = *(target+58)` | 目标 ChrCtrl | B `cs/chr_ins.rs` L145，ChrCtrl L567–572 的普通 manipulator 位于 `+18` |

`ChrManipulator` 只有有限基类建模，`ManipulatorType` 的 Pad/Com 等枚举不能证明 debug 实例是哪个派生类，也不能证明任意非人角色拥有合适的输入动作映射。来源：B `cs/chr_manipulator.rs` L8–51。

## link / unlink 与关键偏移

[E `link_target` L842–867、`unlink_target` L869–887](https://github.com/AlpinDale/er_enemy_control/blob/8056f10c0864e3e10d6c20cc85827676fac36ab4/src/dllmain.cpp#L842) 做三个关联：

1. 清掉旧相机目标的 `ChrCtrl+3B0`。
2. 将 `debug_manipulator+ A8` 指向新目标，将新目标的 `ChrCtrl+3B0` 指向 debug_manipulator。
3. 将 `WorldChrManDbg.cam_override_chr_ins (+B8)` 指向目标。

unlink 把上述关联清零；它没有恢复进入前非零的既有 debug 关系，也没有保存它们。因此本项目需明确槽位空闲前置或保存并校验归属后恢复，不能把“写零”视为普适恢复。

**不要混淆两个类的 `+3B0`：** A 的 ChrIns 在 `+3B0` 后紧接 cloth_state，B 的 ChrIns 多出 `unk3b8: usize`，使后续字段后移 8；E 的 backlink 则位于 **ChrCtrl+3B0**。B 的 ChrCtrl 仍在 L687 保留 `unk3b0: usize`，L688 是 `unk3b8: [u8;0x18]`，并没有随 ChrIns 一起移位。

| 字段 | A | B | 移植判断 |
|---|---:|---:|---|
| ChrIns module_container / chr_ctrl | +190 / +58 | +190 / +58 | 可按类型读，仍需运行时类型与 owner 校验 |
| ChrIns debug_flags | +530 | +538 | E 硬编码 +530 对 B 错位；必须使用字段访问/编译期布局断言 |
| ChrCtrl override manipulator 候选槽 | +3B0 | +3B0 | X 证实优先于 +18；B 暂未公开命名该私有字段 |
| WorldChrManDbg debug manipulator / camera target | +A8 / +B8 | +A8 / +B8 | 布局与 X 一致；不等于运行时实例必存在 |

来源：A `cs/chr_ins.rs` L294–324、L686–687；B 同文件 L294–325、L687–688。表格偏移为结构布局审计，实施时仍应保留 `offset_of!`/大小断言。E 的锁定目标参数还硬编码 `PlayerIns+6B0`（L984–991）；ChrIns 尺寸变化会影响 PlayerIns 后缀，须独立核对 B 的 `locked_on_enemy`（L913），不能据旧命名填充字段认定偏移。首轮已知兽体句柄无需依赖该锁定目标取样路径。

## 本机二进制静态证据

使用 Python pefile/Capstone 对 X 的磁盘字节扫描。E 三个完整 masked signature 均只有一个匹配：world RVA `0x3FFBF6` 解出 singleton RVA `0x3D69FF8`；debug RVA `0x508DCF` 解出 singleton RVA `0x3D6A208`；target 函数 RVA `0x508A50`。唯一匹配只是定位检查，并不保证公开函数 ABI 或所有字段正确。

下列连续指令按原字节解码，足以证明 override 的读取优先级：

```text
RVA 003C6F50  mov rax,[rcx+3B0]
RVA 003C6F57  test rax,rax
RVA 003C6F5A  jne 003C6F60
RVA 003C6F5C  mov rax,[rcx+18]
RVA 003C6F60  ret
RVA 003C726C  mov rcx,[rcx+3B0]
RVA 003C7273  test rcx,rcx
RVA 003C7276  jne 003C727C
RVA 003C7278  mov rcx,[rax+18]
RVA 003C727C  mov rax,[rcx]
RVA 003C727F  call [rax+D0]
```

结合 B 的 ChrCtrl `+18 manipulator`，这是 `+3B0` 为替代 manipulator 的强静态证据；尚未命名 vtable `+D0` 的语义。另 X RVA `0x508DCF` 从 debug singleton 取对象，经 `0x514010 -> 0x5AF943` 返回 `[rcx+B8]`；若为空，RVA `0x508E05` 读取 WorldChrMan `[rdi+1E508]`。它印证镜头覆盖角色/主玩家回退通路。没有证据表明需要把 NPC 伪装成 PlayerIns。

## 本体保护、位置与恢复审计

[E `set_control_flags` L889–926](https://github.com/AlpinDale/er_enemy_control/blob/8056f10c0864e3e10d6c20cc85827676fac36ab4/src/dllmain.cpp#L889) 的实际含义如下。B 对应源码路径/定位列出，以免把所有位统称为“控制开关”：

| 写入 | 对应语义 | 本项目处理建议 |
|---|---|---|
| 本体 debug_flags `|0x30` | 禁止 secondary actions 和 movement，B `cs/chr_ins.rs` L529–532 | 当前偏移 +538；保存相关位原值 |
| 本体 `+1C5 &=~8` | 关闭渲染，B L445–446 `enable_render` | 可使用类型化标志；恢复原位，非无条件置真 |
| 本体 module.data `+19B |=1` | 私有 data.debug_flags 的 bit0，B `module/data.rs` L45–48 仅注释“第二位”使不可伤害 | bit0 的真实语义未由此注释证明；不能拿它独立证明保护 |
| 本体 module.physics `+1D3=1` | fade_out_gravity_disabled，B `module/physics.rs` L70–72 | 并非常规 gravity_disabled（+1D5）；需证明写入时机与持续有效性 |
| 本体 `+68=5`，恢复写0 | `ChrIns.chr_type`（B L153）；B L1039–1046 定义 Npc=5 | 不是泛化 actor state；不应无条件覆盖原类型，需证明该类型切换的实际必要性 |

[E 玩家保护 L545–603、恢复 L696–727](https://github.com/AlpinDale/er_enemy_control/blob/8056f10c0864e3e10d6c20cc85827676fac36ab4/src/dllmain.cpp#L545) 额外保存/覆盖 `ChrCtrl.modifier.data.action_flags`：bit2 置真避免被锁定，bit3 清零允许锁定他人；`ChrCtrl.flags (+F0)` bit1 置真禁止受击。B `cs/chr_ins.rs` L744–749、L787 给出相符语义。公开恢复是整字回写，会覆盖活动期间其它系统的位变化；本项目应限定自己持有的位并校验角色、控制器和 module 身份没有变。

[E 阵营 L1169–1249](https://github.com/AlpinDale/er_enemy_control/blob/8056f10c0864e3e10d6c20cc85827676fac36ab4/src/dllmain.cpp#L1169) 保存双方 team，目标拷贝玩家阵营，本体默认中立化0；可参考事务顺序，但阵营不是免伤证明。[HP 同步 L605–694](https://github.com/AlpinDale/er_enemy_control/blob/8056f10c0864e3e10d6c20cc85827676fac36ab4/src/dllmain.cpp#L605) 把目标 HP/max/base 持续写入玩家，最低夹到1，解除恢复进入前的四项生命数据；首轮并不需要这套生命镜像，它不能成为伤害结算的依据。

[E 位置 L928–982](https://github.com/AlpinDale/er_enemy_control/blob/8056f10c0864e3e10d6c20cc85827676fac36ab4/src/dllmain.cpp#L928) 从相机目标的 physics `position (+70/+74/+78)` 写本体 physics；活动期 Y 减0.875，解除前直接复制 XYZ，无朝向、地图块、碰撞安全、传送请求或距离验证。B `module/physics.rs` L21–28 定义位置与朝向。首轮平地范围内可作为候选方式，仍须用实际位置和水平误差 ≤1m 的 Probe 证明；不外推为通用 warp。

公开恢复与异常路径还不足以直接满足 Spec：

- `handle_f1` L1252–1294 没有活动期重入保护，即使保护步骤未成功也标 active；不满足进入失败的事务性要求。
- `release_control` L730–766 只在目标仍 valid 时恢复阵营，否则清掉保存标记，可能遗留本体中立；其 align 调用重新读取当前 global root，不能保证 fallback root 情况的位置恢复。
- `tick` L1301–1319 在 world/角色/module 缺失时早退，可能跳过解除；`DllMain` L1388–1393 不执行完整 unlink/还原。
- `link_target` 和普通读写有未保护指针解引用；`safe_read` 的可读性不等于实体代际、类型、所有权或生命周期仍匹配。

上述是[固定 E 源码的静态控制流](https://github.com/AlpinDale/er_enemy_control/blob/8056f10c0864e3e10d6c20cc85827676fac36ab4/src/dllmain.cpp#L1252)，不是已在玩家现场复现的缺陷报告。

## Design 建议与仍需证明的点

优先采用“已知独立兽体 + 调试 manipulator override + 镜头 override”，保留本体 PlayerIns；以 B 固定依赖封装类型化接口。先只读核对 debug manipulator 的 RTTI/owner、`ChrCtrl.owner`、目标句柄代际以及三个关联槽当前值；槽非空且不属本能力时拒绝进入。需要封装的未知面主要是 ChrCtrl 私有 +3B0 与 debug manipulator 派生类，不是整个控制器框架。

写入事务应先快照、逐步绑定、验证关联，再宣布活动；任何中间失败执行反向恢复。解除时只恢复本能力持有的字段，验证目标身份后保存最后位置、解除输入和镜头关系，恢复本体控制与渲染；重复进入不再次覆盖快照。每一步必须返回结果，不能只凭 active 标记报告成功。

待证：debug 实例是否总存在及何时失效；非人角色移动/攻击输入映射；受控 AI 是否并行执行；镜头跟随和观察输入；保护位在游戏更新后是否持续；位置写入对物理是否有效；10 次重入/恢复的无残留性。`SPEC-ERDB-001..006` 的通过结论只能来自本项目经批准的 Recorder 运行证据与人工项，本调查不提供 PASS。
