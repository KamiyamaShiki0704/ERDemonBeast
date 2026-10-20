---
id: RESEARCH-ERDB-c0010-param-compatibility-20260925
type: research
version: 1.0
updated: 2026-09-25
---

# c0010 参数跨作兼容性离线调查

结论：可以迁移经过语义筛选的攻击/行为/弹丸字段与依赖图；不能直接复制 NR 行字节，也不能以同名字段或同值枚举作为语义兼容证明。当前已存在的 ER c0010 加载/控制参数应作为宿主；新增攻击配置应修正 `behaviorVariationId` 并补齐其实际引用，而不是把 NR 的玩家身份、复活/不死与 Hero 大招系统带入 ER。

本报告仅离线读取现役 regulation、本地 Paramdex/Smithbox 源定义与既有 KB；只写此 Markdown。没有启动、控制游戏或读取活体内存；没有验证实机伤害、死亡、特效或恢复行为。

## 证据源与版本

- [ER 现役 regulation](F:/GoldenAge/_GoldenAge/GA/regulation.bin)：本轮读取 SHA256 `63D3E7902547B4CF4BD0E323EEC712580C8192CAD35C33AB5A960A2C24EFC9C3`。
- [NR 现役 regulation](F:/SteamLibrary/steamapps/common/ELDEN%20RING%20NIGHTREIGN/Game/regulation.bin)：SHA256 `2570844BBA290F3F3D6FFCF6D1FA493E3C6E01503F1E4F735F7F75BCA104BC65`，Binder.Version `10350000`。
- 主定义：WitchyBND 3.0.1.0 的 [ER Defs](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/ER/Defs) 与 [NR Defs](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/NR/Defs)，下文记为 W/ER、W/NR。
- 补充源定义与枚举：Smithbox `Smithbox_FT_Temp_260508/03-Smithbox-Binary/Assets/PARAM/{ER,NR}`，下文记为 S/ER、S/NR。这些是工具自身结构/标签的直接来源；枚举名称不是游戏引擎执行语义的独立证明。
- 任务最初引用的 `Smithbox_FT_260831/.../NR/Defs` 当前不存在；已定位上述 Temp 目录，不把旧路径当作有效证据。
- [KB 主题入口](D:/_Ugit/_FTProject/KnowledgeBase/Topics/Combat/feat-hero-ult-demon-beast/feat-readme.md) 与 [机制证据](D:/_Ugit/_FTProject/KnowledgeBase/Topics/Combat/feat-hero-ult-demon-beast/research/switch-execution-and-control.md)：只作为已完成机制调查的引用；当前数值关键项已直接重读 regulation。

## 实际 schema 校验

以下由 SoulsFormats 解密并读取 PARAM header/DetectedSize，与 PARAMDEF.DataVersion/GetRowSize(0) 对比；宽度为字节。定义名与表名不同必须显式映射：`AtkParam_Npc → AtkParam.xml`、ER `Bullet → BulletParam.xml`、ER `SpEffectParam → SpEffect.xml`。

| 表 | ER 实际/定义版本 | ER 宽度 | NR 实际/旧定义版本 | NR 实际/旧定义宽度 | 决策 |
|---|---:|---:|---:|---:|---|
| AtkParam_Npc | 4/4 | 456 | 5/5 | 464/464 | 字段映射，禁止行复制 |
| BehaviorParam | 2/2 | 32 | 2/2 | 32/32 | 字段结构一致，仍需修正 refType/refId 依赖 |
| Bullet | 4/4 | 272 | 4/4 | 348/332 | W/NR 不匹配，换 S/NR 新定义 |
| SpEffectParam | 4/4 | 912 | 7/4 | 1024/1024 | 同宽不代表版本匹配；只准受审字段读取 |
| ChrModelParam | 3/2 | 16 | 2/2 | 16/16 | ER 版本不匹配需明确记录；两 def 字段结构一致 |
| NpcParam | 9/9 | 736 | 10/9 | 848/848 | 受审物理字段与引用迁移 |
| NpcThinkParam | 2/2 | 228 | 2/2 | 228/228 | NR复用了原padding；保留ER AI宿主更安全 |

这些版本值来自本轮读取，并非从文件名推测。只对同宽 NR SpEffect/Npc 显式 `ApplyParamdef` 读取；其版本差异是证据限制。强制套旧 Bullet 定义会把不匹配隐藏起来，不能以其结果进入写入流程。

### 已找到匹配的 Bullet 定义

[S/NR Bullet.xml](D:/_Ugit/_FTProject/SmithBox_FT/Smithbox_FT_Temp_260508/03-Smithbox-Binary/Assets/PARAM/NR/Defs/Bullet.xml) SHA256 `2243E5F8530666DE8630D29BC118E87A1627AE0F589AA06D514BBF7BF6D2A638`。其尾部新增 `u8 fh_unknown_1..4` 和 `dummy8 fh_unknown_padding[12]`，全部标记 `FirstVersion="10340000"`。现役 NR Binder.Version=10350000，应包括这些尾字段。

本轮以现有 SoulsFormats `XmlDeserialize(path,false,false)` 实测 `DataVersion=4`、`GetRowSize(0)=348`、`GetRowSize(10340000)=348`；与实际 PARAM 行宽一致。此模式不执行版本裁剪，因此不能据此声称旧 regulation 同样可用。推荐进入父任务的解码/读回检查时直接使用此匹配定义；尾部未知字段没有 ER 语义，应不迁入 ER。

## 707100 与 707110 的具体问题

| 行 | 当前 NR 实读 | 当前 ER 实读 | 建议 |
|---|---|---|---|
| SpEffect 707100 | stateInfo=2290、spCategory=20、effectEndurance=-1；vfxId及vfxId1..7均-1；常规链引用均-1 | 已存在，stateInfo=0、category20、duration-1 | 保留 ER 纯标记设计，禁止把2290写回 |
| SpEffect 707110 | stateInfo=143、spCategory=20、effectEndurance=-1；vfx与常规链引用同上 | 不存在 | 不直接复制143；若没有脚本实际依赖，省略此行与Npc槽位；若需要ID检测，只可新建ER中性标记并明确语义降级 |
| NpcParam 100000 | spEffectID16=707100、spEffectID21=707110 | 当前全部spEffectID0..31=-1 | 只接线已经证明必要且转换完成的效果 |

[NR SP_EFFECT_TYPE](D:/_Ugit/_FTProject/SmithBox_FT/Smithbox_FT_Temp_260508/03-Smithbox-Binary/Assets/PARAM/NR/Param%20Enums/SP_EFFECT_TYPE.json) 把143标为 `Immortality`，2290仍标 `Unknown`；[ER SP_EFFECT_TYPE](D:/_Ugit/_FTProject/SmithBox_FT/Smithbox_FT_Temp_260508/03-Smithbox-Binary/Assets/PARAM/ER/Param%20Enums/SP_EFFECT_TYPE.json) 把143标为 `Character Respawn`，没有2290。事实是工具标签冲突与 ER 定义缺值；“NR不死语义无法等价移植”是据此与 KB 的双实体/配对HP机制作出的保守工程判断。没有反汇编 ER143 执行体，不能把工具标签直接当作已经实测的游戏行为。

同名字段还有重要陷阱：NR `physicsDiffence/magicDiffence/fireDiffence/thunderDiffence` 为 f32，NR这两行均1.0；ER同名为s32、定义默认0。当前ER707100实读这四字段均整数1。若需要真正中性标记，应按 ER 默认保留0，不能将 NR中性倍率1.0强转ER整数1。此差异已向父任务提示；本调查没有修改该行或证明整数1的实机效果。`spCategory` NR是s16、ER是u16，须校验取值范围，不能简单复用有符号转换。

## 攻击枚举同值不同义

源：[ER ATK_SUB_CATEGORY](D:/_Ugit/_FTProject/SmithBox_FT/Smithbox_FT_Temp_260508/03-Smithbox-Binary/Assets/PARAM/ER/Param%20Enums/ATK_SUB_CATEGORY.json)、[NR ATK_SUB_CATEGORY](D:/_Ugit/_FTProject/SmithBox_FT/Smithbox_FT_Temp_260508/03-Smithbox-Binary/Assets/PARAM/NR/Param%20Enums/ATK_SUB_CATEGORY.json)。下列是源标签事实，应逐条对本次 c0010 攻击实际值过滤；不能一律保留数字。

| 值 | NR标签 | ER标签 | 保守处理 |
|---:|---|---|---|
| 120 | Throwing Knife Item Attack | Two Handed Attack | 不按同值迁移 |
| 121 | Throwing Stone Item Attack | Backstep / Rolling Attack | 不按同值迁移 |
| 122 | Character Skill | Dash Attack | NR专用技能类别省略/映射普通类别 |
| 123 | Ultimate Art | Magma Attack | 省略NR大招类别，绝不能原值解释为熔岩 |
| 124 | Two-handed Attack | Storm Attack | 仅有明确需求时可按语义映射ER120 |
| 125 | Dual Wield Attack | Divine Beast Frost Stomp Attack | 无直接映射，保留ER中性类别 |
| 126 | No-Damage Sudden Death | 无枚举 | 不迁移 |
| 127 | Dash Attack | Stomp / Kick Attack | 有需求时映射ER122 |
| 128 | Rolling Attack | Unused Attack | 有需求时映射ER121，但ER合并了翻滚/后撤 |
| 129 | Backstep Attack | Dance Attack | 同上 |
| 130 | Melee Weapon Attack | Messmer Skill Attack | 不按同值迁移 |

`atkBehaviorId/atkBehaviorId_2` 的 [NR枚举](D:/_Ugit/_FTProject/SmithBox_FT/Smithbox_FT_Temp_260508/03-Smithbox-Binary/Assets/PARAM/NR/Param%20Enums/ATKPARAM_BEHAVIOR_ID.json) 新增12=`Extra Large Damage (Ult Stagger)`、15=`Claw Shot`、16=`Claw Shot Pull`；ER枚举无这些值。共同1..10也有明显标签差异，例如NR1=Rolling、ER1=Continuous Attack；NR10=Prevent Stagger after Falling 10+、ER10=Start Grab。不能用“值在ER枚举里存在”替代语义审查。

`spAttribute` 的 [NR ATKPARAM_SPATTR_TYPE](D:/_Ugit/_FTProject/SmithBox_FT/Smithbox_FT_Temp_260508/03-Smithbox-Binary/Assets/PARAM/NR/Param%20Enums/ATKPARAM_SPATTR_TYPE.json) 新增27..31（Dragonwound Arrow、Blackflame Arrow、Bloodflame Arrow、Holy Greatarrow、Magic Bolt）；这些既影响Atk也影响Bullet，不能把新值原样保留。

Npc `teamType` 本轮 NR100000=1、当前ER100000=6。NR枚举标签1=Player，ER1=Live；NR还有 ER无定义的34/40/49/50/52/53/56/77/80等。建议保留本项目已验证的ER队伍/控制配置，避免根据“c0010原版是玩家”覆写阵营。

## 各表迁移策略

1. **NpcParam**：以当前ER100000为宿主；保留已验证体型、生命与控制配置。攻击图接通时将 `behaviorVariationId=100` 与对应 Behavior 实际行保持一致；这会替换当前狼宿主40700，必须同一候选包落地。掉落、GameClear/dlcGameClear、NR奖励/日数/匹配/角色状态不要整行带入。spEffect槽位按实际必需效果白名单重建。
2. **BehaviorParam**：两作 def 的12个字段一致；保留行为ID寻址规则，审计 `refType/refId` 是否指向本次迁入的Atk/Bullet/SpEffect或已验证ER等价项。行ID与数值字段同构不能证明被引用资源可用。
3. **AtkParam_Npc**：ER默认行或同类ER攻击为宿主，迁共同且同类型的碰撞、伤害与受击字段；额外审计类别、属性、atkBehaviorId、spEffectId引用。NR无ER `finalDamageRateId`，保持ER默认/宿主值；NR专有蓄大招、复活伤害、Mixcraft、JustGuard等字段不迁移。
4. **Bullet**：使用S/NR348字节定义解析，序列化只用ER272字节定义。追踪 atkId_Bullet、HitBulletID、intervalCreateBulletId、spEffectId等引用并核实特效资源；NR unknown/fh_unknown尾部全部省略。ER独有mayRemoteDetonate/isDisableAiProjectileDodge保持ER默认。
5. **SpEffectParam**：不做共同字段全拷；先分类成纯标记、普通数值效果、NR引擎状态、视觉依赖。普通数值效果须审stateInfo、引用、类型与中性值；NR Hero/Ult/characterSkill/subSa/necromancy/revenant字段不迁入。纯标记用ER中性行重建，只保留ID契约与明确的持续时间/分类。
6. **ChrModelParam**：8字段结构同构、16字节；逐字段比较现有ER100后决定是否新增或保持，不把模板缺版本3悄悄当通过。此表不能移植模型资产或换身引擎。
7. **NpcThinkParam**：NR100000 logicId10000/battleGoalID50000的机制见KB，而KB查无对应普通AI战斗脚本。保留ER控制宿主AI参数，除非交付中也明确实现/证明NR AI依赖；不复制NR独占unknown字段。

## 自动验证边界

候选应记录每个新增/修改行的来源、目的ID、白名单字段、被省略或翻译字段、引用闭包、源/候选哈希；重新解密并用ER定义读回所有改变表，确认行宽/类型以及候选差异只落在允许行。特别断言707100无2290、707110不携带未经证明的143、NR类别123不作为ER熔岩类别、Atk/Bullet所有有效引用可定位。

离线闭包可以证明“记录存在并指向预期行”，不能证明NR FFX资产在ER可加载、攻击手感/伤害正确、HitType/Behavior枚举执行兼容、死亡恢复或Boss交互正确。最终运行结论仍由人验收。

## 本次实际 c0010 闭包的最小转换清单

后续补充来源：[nr-direct-rows.json](D:/_Ugit/FSTool/_Mod/_Project/er-demon-beast/tmp/combat-param-inventory/nr-direct-rows.json)、[er-collisions.json](D:/_Ugit/FSTool/_Mod/_Project/er-demon-beast/tmp/combat-param-inventory/er-collisions.json)，由父任务解码；本子任务独立分析字段值。以下计数基于读取时已扩展依赖闭包的21个Atk、12个Bullet；这些临时文件可能由父任务继续扩展，交付时应绑定最终快照哈希。

| 对象 | 实际值 | 最小转换 |
|---|---|---|
| Atk 10100/10110/10120/10121/10130/10150/10170/10171/10200/10250/10450/10400/10405/10700/10800/10135/10175/10255/10455 | subCategory2=123 | 全改0；删除NR大招分类而保留基本攻击 |
| Atk 10450、10455 | atkBehaviorId=12 | 改0，保留原dmgLevel/普通受击数值；明确NR专用Ult Stagger未迁移 |
| 其余Atk（包括10990、通用1） | atkBehaviorId=0；全部atkBehaviorId_2=0、spAttribute=0 | 保留0 |
| Atk subCategory1 | 仅0、102、104、106 | 保留；两作枚举分别同为无类别/Jump Attack/Final Chain Attack/Roar Attack |
| 全部12个Bullet | spAttribute=254；无NR27..31属性 | 保留254 |
| Bullet10135/10175/10255/10455 | atkId_Bullet=1，HitBulletID=10136/10176/10256/10456 | 保留起始弹丸无目标攻击1，补齐后继Bullet及其Atk10135/10175/10255/10455 |
| Bullet10800 | spEffectId0=7011701 | 不能留下缺失引用；迁入受审治疗效果或显式关闭此遗物治疗分支 |

父任务发现 Behavior200100201 指向源侧不存在的 Atk10201，且 c0010 TAE type1/2 未引用此行为。可以不迁这个孤儿；依据是本次引用图审计，不是它处于某个ID范围外。本子任务未重新解析TAE，最终证据应由父任务的tae-events/closure报告绑定。

### 本次效果具体分类

- **707120**：本子任务重新直接解码 NR，stateInfo=0、spCategory=20、effectEndurance=0、categoryPriority=0、vfxId=-1、所有常规伤害倍率1。可用ER中性模板重建同ID瞬时标记；不得把duration擅自改-1。
- **707130**：stateInfo=0、category1002、priority254、duration-1，普通倍率1；属于分类覆盖效果。最小攻击闭包不需Npc22，建议不施加。若目标是恢复此分类优先级，按ER中性模板仅设这些明确字段；不要称作不会影响其他SpEffect的无作用标记。
- **705012**：stateInfo624，NR枚举为Is Hero/Player Character，ER枚举无624。省略Npc30；已有ER控制代码负责操作，不用NR状态冒充引擎玩家对象。
- **700500**：NR行名是钩爪目标/拉动方向标记；没有ER钩爪实现需求，省略Npc29。
- **707110**：除了143，还包含slash/blow/thrust/neutral/magic/fire/thunder/dark八种DamageCutRate=0.3。因此不是纯标记。省略它同时放弃NR不死/复活相关机制与该70%减伤；如希望只保留减伤，可新建state0的ER效果只移植八倍率与目标标志，但这会改变平衡，不能隐含在基础攻击迁移中。
- **7011701**：state0、duration0.9、motionInterval1、changeHpRate=-15、bCurrHPIndependeMaxHP=1、vfxId1642000；作用目标是自己/友方。ER可表达这些通用治疗字段，但NR VFX1642000另需验证资源与SpEffectVfx条目。最小方案可明确删除视觉引用保留治疗语义，或禁用整个遗物治疗引用；不能声称原版治疗演出已完整移植。

### 源 Npc 常驻效果白名单

当前ER已有 `5400,5369,90000,90010,90020,90040,90300,90060,90100,5851,5890,102400,102401`。逐行对照NR与ER共同字段，这组各只有6项差异：四种Diffence的ER0/NR1.0、vitalSpotChangeRate的ER-1/NR1、unknown_0x352_6的ER0/NR1。其余共同字段相同。若需要恢复源Npc对应动画偏移、受击与异常/坠落规则，可引用这些ER现有行，不覆盖其内容；此处证明共同字段接近并保留ER语义，未证明NR独有字段等价。

`90030` 额外存在真正开关差异：ER `disableCurse=0`、NR `disableCurse=1`。因此不能把现有ER90030称为已恢复NR死咒免疫。最小方案不接入该槽；如选择沿用ER90030，需明确保留ER语义的偏差。不可全局改现有90030，以免影响其他角色。

为满足“保留已有SpEffect”原则，建议保留现有707100不在本次偷偷清理其四种Diffence；将默认值问题单列为已知差异。其余新增效果用ER模板，不复制这项历史差异。对接Npc槽位前，应根据当前项目希望保留的受击/异常/坠落行为选择上述白名单，而不是将所有NR非负槽位直接写回。

## schema 差异附录

以下按 W/ER 与 W/NR 的InternalName对比，过滤dummy8/pad字段；“NR独有”仅指当前这两份定义中ER没有同名字段，不声称每项都是NR新引擎能力。共同类型差异仅列实际类型/位宽变化，不把默认值变化误算为类型变化。

### AtkParam

来源：[ER定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/ER/Defs/AtkParam.xml) / [NR定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/NR/Defs/AtkParam.xml)。

NR独有：`spAttributeMixcraft`、`unknown_2`、`ultimateChargeCorrection`、`atkSa_JustGuard`、`reviveDamageCorrection`、`reviveDamage`、`unknown_4`、`subCategory5`、`unknown_5d`。

ER独有：`finalDamageRateId`。

共同类型差异：无（默认值另须核验）。

### BehaviorParam

来源：[ER定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/ER/Defs/BehaviorParam.xml) / [NR定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/NR/Defs/BehaviorParam.xml)。

NR独有：无。

ER独有：无。

共同类型差异：无（默认值另须核验）。

### Bullet

来源：[ER定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/ER/Defs/BulletParam.xml) / [NR定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/NR/Defs/Bullet.xml)。

NR独有：`unknown_1`、`unknown_2`、`unknown_3`、`unknown_4`、`unknown_5`、`unknown_6`、`unknown_7`、`unknown_8`、`unknown_9`、`unknown_10`、`unknown_11`、`unknown_12`、`unknown_13`、`unknown_14`、`unknown_15`、`unknown_16`、`unknown_17`。

ER独有：`mayRemoteDetonate`、`isDisableAiProjectileDodge`。

共同类型差异：无（默认值另须核验）。

### SpEffectParam

来源：[ER定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/ER/Defs/SpEffect.xml) / [NR定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/NR/Defs/SpEffectParam.xml)。

NR独有：`poisonInflictRate`、`diseaseInflictRate`、`frostInflictRate`、`sleepInflictRate`、`madnessInflictRate`、`applyIdOnKillSp`、`secondaryIconId`、`bloodInflictRate`、`curseInflictRate`、`subSa`、`subSaBonusMax`、`subSaBonusRate_Magic`、`subSaBonusRate_Fire`、`subSaBonusRate_Lightning`、`subSaBonusRate_Holy`、`tertiaryIconId`、`changeGuardAngle`、`wepTypeTrigger`、`wepTypeTriggerCount`、`startingItemAmount`、`enemyStateInfoTrigger`、`startGoodsId`、`heroStatusId`、`startSwordArtsId`、`subSaBonusRate_Poison`、`subSaBonusRate_ScarletRot`、`subSaBonusRate_Bleed`、`subSaBonusRate_Blight`、`subSaBonusRate_Frostbite`、`subSaBonusRate_Sleep`、`subSaBonusRate_Madness`、`spEffectTextId_2`、`spEffectTextId_3`、`spEffectTextId_4`、`isPeriodicEffect`、`ultimateArtGaugeRate`、`characterSkillCooldownReduction`、`ultimateArtGauge`、`ultimateArtGaugeRecovery`、`arrowBehaviorOffset`、`playerHandValue`、`spEffectTextId_1`、`currentHealthAttackRate`、`triggerOnWepType`、`triggerAttachedWeapon`、`additionalCharacterSkillUse`、`triggerEffectOnKnockback_unk`、`weakPointValue`、`runeDiscountValue`、`modifyPlayerLevel`、`ultimateArtDuration`、`restageAttackRate`、`unknown_240`、`revenantFamily_spEffectId_1`、`revenantFamily_spEffectId_2`、`revenantFamily_spEffectId_3`、`unknown_241a`、`unknown_241b_1`、`isSubSaUseMultiplayCorrect`、`unknown_241b_3`、`randomChanceValue`、`unknown_241d`、`unk_00`、`necromancyActivationSpEffectId`、`ultimateArtGaugeChargeRate`、`conditionStamina`、`unk_04`、`poisonDamageRate`、`diseaseDamageRate`、`unk_05`、`rainDamageCorrection`、`weaponTypeDiscovery`、`heroStatusModifier`、`analyzeAcquire_effectId`、`analyzeSelfLevel1_effectId`、`analyzeSelfLevel2_effectId`、`analyzeSelfLevel3_effectId`、`startMagicId`、`frostInflictRate_old`、`sleepInflictRate_old`、`madnessInflictRate_old`、`applyIdOnKillSp_old`。

ER独有：`normalSpotChangeRate`、`insideDurability`、`maxDurability`、`equipWeightChangeRate`、`allItemWeightChangeRate`、`grabityRate`、`soulStealRate`、`antiDarkSightRadius`、`antiDarkSightDmypolyId`、`changeStrengthPoint`、`changeAgilityPoint`、`changeMagicPoint`、`changeFaithPoint`、`changeLuckPoint`、`recoverArtsPoint_Str`、`recoverArtsPoint_Dex`、`recoverArtsPoint_Magic`、`recoverArtsPoint_Miracle`、`defPlayerDmgCorrectRate_Physics`、`defPlayerDmgCorrectRate_Magic`、`defPlayerDmgCorrectRate_Fire`、`defPlayerDmgCorrectRate_Thunder`、`defPlayerDmgCorrectRate_Dark`、`atkPlayerDmgCorrectRate_Physics`、`atkPlayerDmgCorrectRate_Magic`、`atkPlayerDmgCorrectRate_Fire`、`atkPlayerDmgCorrectRate_Thunder`、`atkPlayerDmgCorrectRate_Dark`、`atkEnemyDmgCorrectRate_Physics`、`atkEnemyDmgCorrectRate_Magic`、`atkEnemyDmgCorrectRate_Fire`、`atkEnemyDmgCorrectRate_Thunder`、`atkEnemyDmgCorrectRate_Dark`、`unknown_0x352_7`、`unknown_0x353_0`、`unknown_0x353_1`、`unknown_0x353_2`、`unknown_0x353_3`、`unknown_0x353_4`、`spiritDeathSpEffectId`。

共同类型差异：`physicsDiffence`：ER `s32 physicsDiffence` / NR `f32 physicsDiffence`；`magicDiffence`：ER `s32 magicDiffence` / NR `f32 magicDiffence`；`fireDiffence`：ER `s32 fireDiffence` / NR `f32 fireDiffence`；`thunderDiffence`：ER `s32 thunderDiffence` / NR `f32 thunderDiffence`；`spCategory`：ER `u16 spCategory` / NR `s16 spCategory`。

### ChrModelParam

来源：[ER定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/ER/Defs/ChrModelParam.xml) / [NR定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/NR/Defs/ChrModelParam.xml)。

NR独有：无。

ER独有：无。

共同类型差异：无（默认值另须核验）。

### NpcParam

来源：[ER定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/ER/Defs/NpcParam.xml) / [NR定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/NR/Defs/NpcParam.xml)。

NR独有：`unknown_0`、`rewardItemLot_1`、`rewardItemLot_2`、`day2SpEffectID`、`poiseBonusRate_Magic`、`poiseBonusRate_Fire`、`poiseBonusRate_Lightning`、`poiseBonusRate_Holy`、`poiseBonusRate_Poison`、`poiseBonusRate_ScarletRot`、`poiseBonusRate_Bleed`、`poiseBonusRate_Blight`、`poiseBonusRate_Frostbite`、`poiseBonusRate_Sleep`、`poiseBonusRate_Madness`、`poiseBonusRate_WeakA`、`poiseBonusRate_WeakB`、`poiseBonusRate_WeakC`、`poiseBonusRate_WeakD`、`poiseBonusRate_WeakE`、`poiseBonusRate_WeakF`、`poiseBonusRate_Unk`、`poiseBonusMax`、`unknown_22`、`chaosMatchingCorrectParamId`、`chaosMatchingSpEffectSetParamId`、`chaosMatchingRewardLotId`、`chaosMatchingItemLotId`、`chaosMatchingUnk`。

ER独有：`chrActivateConditionParamId`、`GameClearSpEffectID`、`hpBurnDamageRate`、`isConsideredUndead`、`dlcGameClearSpEffectID`。

共同类型差异：无（默认值另须核验）。

### NpcThinkParam

来源：[ER定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/ER/Defs/NpcThinkParam.xml) / [NR定义](D:/_Ugit/FSTool/FSModify/UnpackingTools/WitchyBND/WitchyBND-3.0.1.0-win-x64/Assets/Paramdex/NR/Defs/NpcThinkParam.xml)。

NR独有：`unknown_1`、`unknown_2`、`unknown_3`、`unknown_4`。

ER独有：无。

共同类型差异：无（默认值另须核验）。
