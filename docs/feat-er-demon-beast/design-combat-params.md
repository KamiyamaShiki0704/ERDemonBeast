# c0010 参数兼容移植设计

输入：spec-combat-params.md@1.0，用户已确认移植并安装。目标为当前 GoldenAge regulation 的增量合并，不以旧文件覆盖。DLL/HKS/Behbnd/Anibnd 不改。

- 按 PARAM 类型和实际行宽选择两作定义，ER 新行从 ER 默认值构造。按已知共同字段逐项复制，拒绝未显式处理的类型差异；未知字段、NR 独有字段不复制，记录排除清单。
- 20 Behavior、20 Atk、12 Bullet、4 SpEffect 使用源 ID，导入前要求这些 ID 在 ER 不存在，冲突立即停止。源孤儿 Behavior200100201 不导入。Npc100000 仅修改 behaviorVariationId=100 及行名；保留控制/阵营/数值与其他槽位。
- Atk.subCategory2 的123转0，两个 atkBehaviorId12转0；SpEffect 四个浮点防御中性倍率1转ER整数增量0，spCategory检查可无损转u16。vitalSpotChangeRate 的NR中性1使用ER默认-1，未知位保留ER默认。
- 参数引用检查覆盖 Behavior→Atk/Bullet，Bullet 子链/效果/AI，Atk 效果/AI音效/攻击修正，SpEffect 链/行为/视觉/足迹；c0010 TAE 中攻击、子弹和效果通知均校验可解析。已有共用行保持原始字节，不递归覆盖。
- CP-P01：解密写出后的加密候选重新检查；原有行逐行字节/名字比较，唯一允许旧行差异为 Npc100000 指定字段/名字；其余表文件字节与 binder 标识逐一比较。检查新行各单元读回值。失败不安装。
- 安装器要求游戏已退出、当前目标仍等于候选输入哈希；保存原件，先写同目录临时文件，原子替换并保留真实替换备份，安装后验证。回退仅在当前文件仍等于本次输出时允许，避免覆盖用户之后的修改。
- CP-P02/03/04 由用户操作游戏，沿用项目例外，不启动游戏或读活体内存。报告只给离线结论，命中、受击和子弹实际行为待用户验收。

automation_scripts: `Scripts/delivery/feat-er-demon-beast/script/port_c0010_combat_params.ps1`、`install_c0010_combat_params.ps1`。

## 构建中发现的工具行为

当前 AquaToolset SoulsFormats 的 XmlDeserialize 会遗漏 `Field Def="... = value"` 中的默认值，例如 maxHpRate=1、vitalSpotChangeRate=-1 实读成0；生成器因此显式从 XML 恢复这些默认值，再构造 ER 新行。候选独立读回确认四个新效果的生命倍率1、常规/弱点替换-1、防御增量0和ER专属攻防倍率1。

ER Atk 的 finalDamageRateId 在当前定义中默认是0，没有 -1 声明。保留该 ER 默认并验证其目标行存在；当前 Mod 行0的五属性伤害倍率为1，精力倍率1.25、削韧倍率1.35。此行为属于沿用 ER 当前共用数据，不等于 NR 完全相同最终伤害，不修改全局行0。
