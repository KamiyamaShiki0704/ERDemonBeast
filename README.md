# ER Demon Beast

在 Elden Ring 中实现执行者魔兽化：保留玩家本体，操控独立兽体，并在结束后恢复本体。

项目根：`D:/_Ugit/FSTool/_Mod/_Project/er-demon-beast`。

本项目于 2026-09-23 按用户要求独立建立，Spec 1.0 已确认。用户测试 0.2.0 的最小/完整组合基本可用，但出现一次 37.115 秒清理等待。当前候选 0.2.1 修复跨可读内存区域误判及恢复后的旧会话地址残留，并增加定向诊断；离线检查通过；用户反馈控制闭环和首轮行为看起来没问题，0.2.1 完整组合日志两次恢复为 0.116/0.219 秒，保留为下一阶段控制基线。正式 Case 的自动报告仍按实际证据记录。

## 开始阅读

- [多形态自定义需求](docs/feat-er-demon-beast/requirements-custom-forms.md)：真实魔兽阶段之后的 SpEffect 路由、独立时长与玩家动作规则扩展。

- [控制基线与下一阶段](docs/feat-er-demon-beast/research/control-baseline-and-next-stage.md)：人工反馈、两次释放证据及 c0010 源资产盘点。

- [主题入口](docs/feat-er-demon-beast/feat-readme.md)：目标和分阶段范围。
- [手动测试步骤](MANUAL_TEST.md)：按加载/观察/控制顺序双击启动，日志回传方法。
- [F7 清理等待诊断](docs/feat-er-demon-beast/research/retirement-stall-0.2.1.md)：实机日志、已修复缺陷和待确认原因。
- [Summon 整合自检](docs/feat-er-demon-beast/research/summon-integration-audit.md)：复用来源、风险修复与验证边界。
- [DLL 自检报告](docs/feat-er-demon-beast/research/dll-self-audit.md)：已确认崩溃、修复证据及剩余不确定性。
- [首轮规格与 Probe](docs/feat-er-demon-beast/spec.md)：已确认的六组可观察行为。
- [技术设计](docs/feat-er-demon-beast/design.md)：实现接口、获批的 ER 原生取证和回滚。
- [测试说明](TESTING.md)：构建命令、独立 profile、按键和实测边界。
- [Build 记录](docs/feat-er-demon-beast/build.md)：自动检查与未证明项。
- [可行性证据](docs/feat-er-demon-beast/research/feasibility.md)：已有能力、出处及未证明项。
- [当前工作包](WORK_PACKET.yaml)：当前阶段、写入范围和下一动作。

## 第一阶段目标

先用 ER 原生非人角色证明“生成 → 输入驱动 → 镜头跟随 → 解除 → 恢复本体 → 再次使用”。该闭环通过后，再接入 NR 魔兽模型、动作、行为和特效，逐步补全受伤、死亡和场景切换规则。

用户“可以，继续任务”已确认首轮范围和数值。当前候选使用 ER 原生 c4070 Wolf 验证接口；尚未接入 NR 魔兽资源。候选包配置默认不启用游戏任务，实测步骤见 TESTING.md。

## 项目组织

本目录使用独立 Git 历史。`src/session.rs` 管理生命周期，`src/native.rs` 适配 ER 原生接口；依赖通过 Cargo.lock 与固定 Git revision 绑定，不依赖兄弟项目的未提交代码。

工程自动化脚本沿用 `Scripts/delivery/feat-er-demon-beast/script/` 路径契约，按需创建。资源样本、构建产物、运行日志留在本地忽略目录；原始游戏资产只作为输入引用。
