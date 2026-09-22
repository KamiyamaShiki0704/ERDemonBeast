# Summon 整合与崩溃风险自检 — 0.2.0

用户要求复用新完成的 Summon 创建/消失清理框架、继续自检修复，并由用户执行游戏测试。本轮没有启动或操作游戏，没有修改 Summon 工程或 GoldenAge 现用配置。

## 固定来源

Summon clean revision `01491edfa709545786d0fe795ab2fb2458dca4e1`。直接复制 chr_lifecycle.rs、retirement.rs、三份原生 fixture 和 MIT 许可；副本字节与来源逐一相等，哈希见 `src/vendor/summon/provenance.json`。其中 ChrSet free、task free、event groups 的 fixture 已与本机目标 EXE 的 RVA 0x495890、0xEB3530、0x494800 精确比较通过。

该工程只导出 DLL 入口，没有供其他插件调用的召唤 API。本项目采用固定源码复用；不另改 Summon.dll、不依赖兄弟目录的未提交工作。手动测试组合排除同时加载 Summon.dll，避免两个独立创建状态机竞争同一生成器。

## 已修复/加固

| 风险 | 处理与证据 | 不能据此宣称的内容 |
|---|---|---|
| DllMain 小栈线程溢出 | 延续堆缓冲区修复；0.2.0 小栈宿主正常退出 | 不能证明所有引擎路径稳定 |
| 在 OwnedPtr 校验前解引用模块 | 先用 ReadProcessMemory 检查 ChrIns、控制器、模块容器及 data/physics/modifier，核对 owner；采集还核对 action 模块。空、未映射地址用真实进程内存读取测试拒绝 | 不把可读性当作无限期有效的生命周期证明 |
| 在角色处理阶段与引擎读取者同时改动/释放 | 整体操作改到 FrameEnd，仅接受 168 和 Summon 已观测的 0x900000A8 编码；采用其任务注销、引用排空及分配器协议 | 本项目尚未实测实际调度/释放结果 |
| 假设创建器自动注册 event ID，可能找不到已生成实体 | 保留创建前 handle 集合；校验请求未被覆盖、新增实体和 last-created 指针一致，再复用 Summon 注册 ID。归属不确定时不认领/释放他人实体 | 两个创建插件并发抢占不是受支持场景 |
| 控制/相机仍引用兽体就销毁 | 先反向恢复控制与本体字段、隐藏兽体；额外检查 camera、manipulator owner、override 和事务为空后才开始脱离 | 外部重新接管会被隔离，不保证无条件释放 |
| 在读者仍持有旧地址时析构、用错误 allocator 删除 | 原样复用 Summon 的六任务注销、ChrSet 脱离、连续两次 FrameEnd 无引用、destructor(false)+owner.deallocate | 隔离/超时不算释放成功 |
| 加载即同时启用所有风险路径，无法定位失败阶段 | enabled/control_enabled 分离；手动 load/observe/control 入口逐层测试 | 观察模式没有进行能力行为验证 |

清理期间维持 Restoring，不接收新进入；清理完成回 Normal，每轮重新创建。若世界/玩家身份变化、已有控制引用、原生接口校验失败，保持 Fault/隔离，阻止下一轮。原有“隐藏后复用”的行为已由真正延迟释放替代。

## 离线结果与限制

- 17 项 Rust 测试通过：含复用的分配器/原生指令/任务引用排空测试、原有生命周期和新增坏地址/FrameEnd 编码检查。
- 8 项 Python 测试通过：包含手动入口模式生成、强制单机、排除重复插件、原 profile/存档不改动、独立备份及保守报告判断。
- Clippy、fmt、release 构建、小栈 DLL 加载回归通过。
- 三份原生字节 fixture 与精确目标 EXE 相等；PE 为 AMD64，DllMain 可定位。
- PowerShell 启动脚本仅语法解析，未执行启动分支；生成器以临时测试文件验证。

历史 EXE+0x1F109C4 写 0x23 的异常仍未定位到确切原因。这轮防护与 Summon 复用是针对已识别风险的改动，不能当作第二例的实机修复证据。没有新增游戏证据或正式 Probe PASS。

## 人工交付

包：`dist/er-demon-beast-0.2.0/`。01 仅加载，02 仅观察，03 最小控制组合，04 GoldenAge 其他 DLL 组合，05 收集日志，06 无效兽体参数场景。每次用户启动都会在包内建立独立 run 并备份 ER0000.Shiki，使用指定 F:/GoldenAge/_GoldenAge 资源和启动器。详细方法见根目录 MANUAL_TEST.md。

先回传失败的步骤与时点、是否出现狼、该次 run 目录日志；如果生成了新的崩溃 dump，再做离线归因。后续游戏操作继续由用户完成。
