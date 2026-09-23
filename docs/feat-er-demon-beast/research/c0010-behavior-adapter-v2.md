# c0010 行为 v2：起手输入与脚本初始化

2026-09-24 用户反馈 v1 移动/转向正常，攻击/闪避失败。最终玩法仍待人测，不能标记 SPEC-ERDB-002 PASS。

## 现场证据

运行 `1790182279124-59300`：1,396 个 Active 帧、零丢帧；兽体仍有 R1/L1/SP_MOVE 等输入，本体动作请求为零。五个行为文件及四个保护输入与 v1 哈希一致。TAE 队列出现 0/2100/5000/5001/5002/43000，但没有 3000/6000；此字段不是完整动画播放记录。随后启动的 `1790182473263-14552` 尚无 Active 帧，不能替代前一轮证据。

## 两处可复现缺陷

1. 当前 EXE 的 act(9100) 在 RVA 0x40E5E2 设置 action_flag.animation_action_flags bit0。env(1106) 在 RVA 0x4130F9 调用 0x407930；设置该位时读新按键 +0x20，否则读队列/输入窗口允许的请求 +0x30。原 NR Idle/Move 会执行 act(9100)，v1 的 ER NPC 通用路径未执行。先前测试把 env(1106) 直接等同原始输入，漏掉此条件。
2. GA 通用脚本末尾未使用的 script_dir 初始化依赖 source:match 非空。裸 @d0010.hks 或 Windows 反斜线路径均使其报错，且该语句位于附加适配器之前。现场 chunk 名尚无记录，因此它是已确认的初始化兼容缺陷，不能断言就是本次唯一根因。

静态证据：hks-wait-static-2.7.1.json，绑定目标 EXE SHA256。入口 audit_hks_wait.py 仅读磁盘文件，未访问运行进程。参考 SDE 本地源码只用于定位 HKS 分派器特征码，未修改该兄弟项目或增加 SDE 运行依赖。

## 修改

- 私有适配器既有 ExecAIAction 入口在查询前调用 Wait；攻击连段继续走 env(1107)，静态证据显示它独立读取 cancel-ready 位，不被 Wait 绕过。
- 私有脚本生成时只移除该未使用路径声明，并输出无 BOM UTF-8。若原声明数量或使用情况变化则拒绝生成。GA c9997 与用户原版 c0010 不修改。
- 追加 [DEBUG-erdb-v2] 限量日志：初始化、Update/动作入口、每次输入请求/精力/动画存在性、发出事件。每个脚本实例最多 80 行，IO 缺失/失败不影响动作。该临时诊断在实机闭环完成后移除；当前不是已完成的故障复盘。
- DLL 仍为 0.2.2；Behbnd、三张名称表、参数和动画文件不变。v2 仅替换已绑定的 d0010.hks。

## 红绿验证

`lua test_c0010_adapter.lua <v1 private> <original c0010.lua> bundled @d0010.hks`：FAIL，14461 行 nil 路径错误。

同一 v1 以正斜杠路径 dofile：FAIL，expected W_Attack3000, got nil，复现新按键存在而 queued inputs 为空的分支。

同一真实 HKS 测试改用 v2，裸文件名/反斜线文件名均 PASS。覆盖待机/步行/跑动起手、五动作、九连段边、关闭取消窗、按住/松开、精力、缺动画、死亡优先、AI 回退、Event20000、编号冲突、诊断 IO 失败。四个 Python 资产/真实安装恢复测试 PASS，luac 语法检查 PASS。

引擎条件由已核验的静态语义建模，仍不是游戏端到端测试。需要人按 BEHAVIOR_TEST.md 验证动作实际播放、取消/恢复，并结合新脚本日志判断剩余障碍。

## 回滚

安装前备份 v1 私有脚本及两个元数据文件。活动 behavior-deployment.json 仅更新 d0010 的 after 哈希，保留原始 before/null 与最初四文件备份；原 Restore-C0010Behavior.cmd 仍能恢复到 v1 安装前状态。具体路径和哈希见 behavior-deployment-v2.json。
