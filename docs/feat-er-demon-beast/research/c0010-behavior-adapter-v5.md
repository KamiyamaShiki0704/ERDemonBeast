# v5 修复证据与边界

对应 Spec 1.2 / Design 1.5，DLL 0.3.1。旧版实机 Failure Packet：exit-notification-v4.json。用户确认登场、操作及其余项正常，退场仍卡住；锁定移动要求调整为可配置且默认原版。

原版 c9997-nr.lua 的 IdleCommonFunction_TDB、MoveCommonFunction_TDB、AttackCommonFunction_TDB 使用 act(2018)。当前 ER 常量 LockonFixedAngleCancel 同号，静态 handler 也已核对；默认分支恢复此调用并复用移动选择。四向开启分支只发存在的方向动画。没有改 anibnd 或人为补造侧移动画。

当前资源 20010 的 HKX 约 0.667 秒，3035 约 5.933 秒；两者 TAE 1011 的时间分别为 0.56666666 与 3.6666667，RevertType=0。原版在动画结束前执行解除，因此单凭 IsAnimEnd 并不等价。v4 outro 节点活动但未收到该结束标志，未确定其底层唯一原因。

选择转换原版实际通知：两个共享 hkbClipTriggerArray 挂到两个 CMSG 各六个 clip，分别发送已有 W_Idle / W_Event20010。HKS 对实际节点转换确认 done；Native 在 Active 或 Finishing 接收当前 nonce 完成后进入原有清理。没有新增内存指针或 NR engine call，没有游戏时间累计器伪造完成。

完整图往返保留全部对象的一一对应和共享指针；只有十二个 triggers 指针和两个新数组属于允许差异。三个其他 binder 条目逐字节相同。TAE 起点从当前 anibnd 重新提取校验，三张 name-ID 表无需修改。HKLib 重编号通过全图引用映射核对，不能简单删掉所有指针字段来比较。

离线回归：旧 HKS 在新退出节点重放下明确失败；新版本通过同一测试，以及战技自动解除、战技中 F7、原版/四向/缺少方向动画、登场初始化顺序、动作衔接等待和五动作回归。测试确实加载实际私有 HKS 与原版 c0010 Lua；节点变化是测试输入，不代表游戏实际触发已验证。Rust 26、Python 16、Clippy、release、64KiB DLL 线程加载通过。

人工重点：默认锁定移动是否符合原版；站立/攻击后正常退场是否消除 8 秒等待；战技是否在原通知处直接返回；重复 F6/F7 与镜头/清理。特别需确认 ER 会投递新 clip trigger。游戏测试未执行，完整特效/伤害与变速精确衔接不在本次结论中。
