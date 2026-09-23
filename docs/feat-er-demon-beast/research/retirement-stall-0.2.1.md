# F7 后清理长时间等待：0.2.1 修复与诊断

用户已实际测试 0.2.0 的入口 03、04，反馈基本可用，但偶发 F7 后 c4070 不消失、F6/F7 无响应。此记录不把初步可用反馈当作六组 Spec 的正式通过。

## 已确认的实机事实

日志解析证据为 `retirement-stall-0.2.0.json`，含两份原始日志路径与 SHA256。最小组合一次恢复 2.131 秒；完整组合两次为 0.216、37.115 秒。慢的一轮在 t=68.247 进入 Restoring，t=71.252 进入 Fault，t=105.362 才析构并回 Normal。六个任务已注销，引用检查反复返回 Some(0)/None，记录了 168 次 None 状态切换。旧日志只记录状态变化，不能据此推导失败总帧数。

Fault 仍持续轮询清理；它并非永久停止执行。F6/F7 在尚有清理所有权时不创建新实体，这是保护约束。日志最后 t=114.377 出现旧世界绑定拒绝；其发生时的用户操作未知。

离线反馈命令（对旧证据预期退出码 1）：

```powershell
python Scripts/delivery/feat-er-demon-beast/script/audit_retirement.py dist/er-demon-beast-0.2.0/manual-runs
```

## 诊断与修复边界

排序检查了：读取范围/布局误判、列表在采样中变化、清理后旧会话绑定残留。离线查看目标 EXE 的 0x50ff80/0x510c90 等向量消费者确认首个列表是 8 字节指针步长，0x510e91 起的链表消费者按 (character,next) 遍历至空；没有证据允许更改偏移、跳过该列表或把 None 当零引用。

两处可离线复现的缺陷已修复：

1. 旧 `readable()` 仅允许读取位于同一 VirtualQuery region 内的数组。用 VirtualAlloc 分配两页、第二页 PAGE_READONLY，创建跨两页且无目标引用的合法数组，真实 `count_vector -> DrainGate` 连续 180 次无法允许释放。回归测试先 FAIL，再修复为完整 ReadProcessMemory 复制后 PASS。新实现检查完整读取成功及读取前后向量头一致；无效地址、未完整读取、仍有引用继续拒绝释放。
2. Retirement::Complete 只清 body，残留 world_address、调试控制地址。真实 NativeBackend::restore 回归先 FAIL（world_address 仍 123），修复后清空已结束会话的地址绑定。只有完全恢复才清绑定，活动/清理期间世界变化仍拒绝继续释放。

旧运行没有内存区域和失败字段快照，因此第一处缺陷尚不能被认定为 37.115 秒等待的实际根因。这是有回归证据的读取修复加定向诊断版本，不宣称实机卡住已经消失。

## 新证据

引用读取失败日志增加 `read_error`：vector 偏移、header-unreadable / invalid-bounds / data-unreadable / header-changed；链表则记录 head/node-unreadable、limit-or-cycle 或 head-changed。等待摘要最多每两秒一次；清理期按 F6/F7 会明确记录 deferred。成功释放仍要求连续两次完整零引用观察与任务代理清空，仍采用原析构及分配器，3 秒 Spec 阈值未改。

固定 Summon 原件与 fixture 保持不变；本项目适配副本是 `src/retirement.rs`，`src/lib.rs` 改用该副本。兄弟 Summon 工程未修改。

0.2.1 离线结果：20 Rust 测试、10 Python 测试、fmt、Clippy、release、小栈加载回归通过；构建哈希见 build-result-0.2.1.json。旧日志回放保持 FAIL 是正确结果，不能因为改代码就重标旧运行通过。

## 用户复测

0.2.1 包内先 03，再 04；每种组合连续 10 轮 F6（活动 2 秒）/F7，每轮等待 5 秒再进入。检查 3 秒内恢复、兽体消失且下一轮可进入。若卡住，额外等 10 秒使诊断摘要写出，然后退出并用 05 收集日志；不要反复按键尝试强制清理。最后补测一轮 15 秒自动恢复。

本轮代理未启动或操作游戏。新包的实际卡住率、列表失败位置和最终验收继续等待用户结果。
