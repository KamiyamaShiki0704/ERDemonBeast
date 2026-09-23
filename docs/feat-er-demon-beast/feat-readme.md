---
id: FEAT-ER-demon-beast
title: ER 执行者魔兽化
type: feat
version: 0.1
updated: 2026-09-23
source: human-raw
related: []
code_paths: []
project_paths: []
tags: [elden-ring, transformation, executor]
---

# ER 执行者魔兽化

## 目标体验

玩家发动能力后操控兽体移动、攻击与观察环境；能力结束时回到原角色，并能再次使用。最终希望还原 NR 执行者魔兽化的动作、演出和退出行为。

用户已要求建立独立项目承接此功能，并于 2026-09-23 确认首轮 Spec 与 Probe。具体行为以 spec.md@1.0 为准。

## 建议阶段

1. **控制与还原闭环**：使用 ER 原生非人角色验证进入、操控、镜头、解除、恢复与重复使用。
2. **魔兽资源接入**：适配 NR 魔兽的模型、动作、行为、攻击判定及特效。
3. **玩法完整性**：连接大招资源、明确受伤与死亡结算，覆盖坠落、传送、切图等中断。

用户已同意真实魔兽最小形态接入，并要求其后提供多形态自定义：每种形态配置 SpEffect 自动/手动触发、独立时长（-1 无限），动作模式先实现玩家输入与招式映射，预留完整 c0000 脚本复用。详见 [后续需求](requirements-custom-forms.md)。联机同步单独评估。首轮成功不代表 NR 资源兼容或完整魔兽化已通过。

## 文档

- [Spec 与语义 Probe](spec.md)：首轮行为与验证边界。
- [技术设计](design.md)、[Build 记录](build.md)：实现和证据状态。
- [可行性调查](research/feasibility.md)：源机制、已有工程能力、证据限制。
- [当前工作包](../../WORK_PACKET.yaml)：阶段与确认状态。

当前已构建实验 DLL；尚无六组运行 Probe 的通过结论。
