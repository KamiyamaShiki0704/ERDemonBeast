# 共用生命与Pose诊断记录

基准：9b734d5工作树，DLL0.4.1/HKSv8；用户确认共用玩家生命、归零正常死亡，并确认spec-shared-health1.0的Probe。研究/修复均离线，未启动游戏或读取游戏活体内存。

## 红灯与最小复现

执行入口（项目根，lua为已安装5.3）：

```powershell
lua Scripts/delivery/feat-er-demon-beast/script/test_c0010_damage.lua <v8-private-hks> input/c0010-action-audit/c0010.lua tmp/c0010-animation-ids.lua
lua Scripts/delivery/feat-er-demon-beast/script/test_c0010_v5.lua <v8-private-hks> input/c0010-action-audit/c0010.lua tmp/hit-replay damage_pending
```

关键输出：

```text
non-playable damage state dispatched: W_DamageMiddleFront requires 8040; reference-pose/no cancel recovery risk
damage interruption retained pending attack; controls never recovered
```

第一项重放真实ExecDamage→ExecNewDamageDefault；资源可用集合取自实际anibnd的TAE。第二项是攻击请求尚未激活即被受击打断，受击结束回Idle后再次按攻击。没有运行Havok，故证明的是缺动画分派与输入锁逻辑，不把Pose视觉或实机命中报为已重现通过。

排序检查：1.缺动画回退；2.实际受击状态绑定/回调；3.中断残留pending_action。检查发现8040不存在（受击区只有8060、8700及9000/9500混合资源）；W_DamageExtraLarge在图中进入DamageExtraLarge，8060/end2，但GA公共HKS仅有DamageSpecial回调。初次候选仅包装DamageSpecial，换成图中真实状态的回归后仍红灯；该候选未安装。补齐真实状态激活/更新后通过。

## 修复与限制

- 私有ExecNewDamageDefault改用存在性检查完整的普通方向路由，不发送缺失轻/中/重普通受击；HP照常由引擎结算。不把8060强行当作每次轻击的替代动作。
- 真实DamageExtraLarge与其他受击激活清理攻击/pending/连段计时；受击标记直到回Idle/Move才释放，正常解除等待受击结束。特殊受击/架势崩溃继续使用现有资源。
- 新HealthLink在FrameEnd按兽体生命增量合并到玩家当前生命；进入继承当前/最大/基础生命，解除最后同步，不使用还原生命快照。零生命保持零，触发原有同帧紧急清理以撤销玩家保护。写失败停止该链接，避免重试重复扣血。
- 测试生命读写使用隔离字节内存和实际native读写函数，不构造含无效指针的CS对象。未证明游戏死亡动画/存档结算，只证明零值与生命周期请求。

自动：Rust45、Python17、Clippy/fmt/release、64KiB DLL加载通过；HKS44方向/伤害组合、真实中断/受击退出、连按翻滚20轮、连段、登退场、战技、原版/四向与Rust配置映射通过。源码哈希见build-result-health-0.4.2.json。

最终安装包dist/shared-health-0.4.2-r2；DLL e6af0a5a14f4637138329a42ca6ce99f4887c6ef1745cce77d23d0ad3f55883d，HKS 791dab7b275b65215ed657ce43af7b5b91961110304ebbd50dc88d23d0f2092c。部署记录deployment-health-0.4.2.json；11文件及备份、7受保护输入校验通过。游戏结果待用户验收。

预防：受击测试必须绑定真实图中的状态名和实际角色资源表；仅调用公共脚本里看似对应的DamageSpecial会漏掉缺失的DamageExtraLarge回调。生命状态必须与临时可回滚控制Patch分离，以免解除返还已经损失的HP。
