---
id: FEAT-ER-demon-beast
title: 初始可行性与证据索引
type: research
version: 0.1
updated: 2026-09-23
source: source-audit
related: []
---

# 初始可行性调查

## 问题与方法

问题：现有 NR 魔兽化研究与 ER 工程能力，能否支持在 ER 中复现执行者变身？

方法：只读检查 NR 主题及其研究脚本、ER 本地源码和历史验收记录，查阅公开 NPC 控制源码。未运行游戏，未复跑源研究的全套反汇编/内存 Probe，未取得本项目运行证据。

结论：独立实体与大招计时有工程基础；ER 公开源码提供另一实体接管的参考。输入/镜头移交、生命结算、异常恢复以及 NR 行为资产兼容仍需本项目证明。优先用 ER 原生非人角色验证控制闭环。

## 本地一手来源

下列 HEAD 是 2026-09-23 读取时的仓库定位；工作树可能含未提交变更，不视为已固定的干净源码快照。开始实现前应固定实际使用版本与文件哈希。

| 来源与 revision | 定位 | 支持的事实及限制 |
|---|---|---|
| NR 机制 KB；HEAD `8119ee174b685a39806a243a9c5acfbc5dbd912b` | `D:/_Ugit/_FTProject/KnowledgeBase/Topics/Combat/feat-hero-ult-demon-beast/feat-readme.md@1.3`；`research/switch-execution-and-control.md@1.2` §1–§5 | 记录双实体、专用控制器、输入函数和解除路径；部分语义及运行时配对仍待印证。创建链记载硬编码 c0010，不能据参数值推出任意角色可换。 |
| Summon；HEAD `20c259d0c8a2597b5421d88c2565aa0dc89fb7bb` | `D:/_Ugit/FSTool/_Mod/_Project/Summon/src/lib.rs`：`spawn_debug_character`、`activate_existing_summon`、`prepare_summon` | 生成独立角色、请求已有动作、控制阵营、位置和可见性；不是输入接管。 |
| Summon；同上 | `D:/_Ugit/FSTool/_Mod/_Project/Summon/docs/compatibility/engine-chr-lifecycle.md`：Still unverified | 新增销毁与实体 ID 注册有代码和静态分析；文档明确未跑该构建的 live session。 |
| 工程 KB；HEAD `9511e3e8137fd2cda9fca74bb07a4b688fe89087` | `D:/_Ugit/FSTool/_Mod/Topics/Gameplay/bug-summon-er-2-7-compat/build.md`：v0.1.3 人工验收记录 | 历史召唤、隐藏、复用和朝向通过；不覆盖后续修改或本项目。 |
| nightreign_style_hud；HEAD `215a2158bf32e5027dc9167368ca68bcee705368` | `D:/_Ugit/FSTool/_Mod/_Project/nightreign_style_hud/README.md`：Executor；`examples/nightreign-style-hud/src/state.rs`：`executor_transform_end`、`apply_pending_executor_events` | 执行者有 15 秒扣槽、取消和自动结束信号；相关分支不创建/接管兽体。 |
| fromsoftware-rs；HEAD `3c8c1d7633a99309fb004c9f894ea10b7967d0e0` | `D:/_Ugit/FSTool/_Mod/_Project/fromsoftware-rs/crates/eldenring/src/cs/world_chr_man.rs`：`main_player`；`chr_manipulator.rs`；`chr_ins/module/action_request.rs` | 有控制器/输入结构；main_player 是 PlayerIns 类型，不能推出安全替换任意 NPC 的能力。 |

## 外部源码参考

- [ER Enemy Control](https://github.com/AlpinDale/er_enemy_control)：README 描述 F1 接管 NPC、F2 解除控制。
- [dllmain.cpp](https://github.com/AlpinDale/er_enemy_control/blob/master/src/dllmain.cpp)：`handle_f1`、`release_control`、`start_hp_sync`、`stop_hp_sync` 提供控制关联、恢复和生命同步参考。解除时恢复保存的生命数据，不能直接当作 NR 魔兽化伤害结算。

外部源码本次未固定 commit、未编译、未在当前 ER 验证。采用前核对 commit、许可证、目标版本和实际接口布局。

## 不确定项与后续证据

1. 当前 ER 可执行版本、安装目录与实际加载配置需重新定位；源主题部分绝对路径本次不可用。
2. 主题所引用 TAE 模板的当前文件未复现其事件名描述；不能把模板命名当作已经独立确认的引擎语义。需重新定位来源版本与导出原件。
3. NR 专用控制器及 TAE 事件的行为不是 ER 现成接口；任何跨版本地址、枚举和字段都需重新验证。
4. 当前没有本项目关于输入接管、镜头绑定、NR 兽体动作/攻击兼容、双实体生命结算的运行证明。
5. 下一步以 spec.md 的首轮语义 Probe 收敛需求，确认后再选择实现与公共 Recorder 的 ER 取证适配；不以成功加载或编译替代玩法验证。
