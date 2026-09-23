# v3 进入超时与 v4 私有 HKS 修复

## Failure Packet

运行 1790187506821-58692；DLL 0.3.0 fcbff114cd3cd64b9dc6e9595a3c69c0e0bd6335db05e4e4e54a799dcfa47daa。用户 F6 后无登场动画、无法操作、几秒恢复。四轮进入到超时分别 3.016/3.001/3.000/3.000 秒，Active 帧为 0；HKS 每轮只有 loaded/starting/Update reached，没有 ready。随后均完成析构、归属实体为零。证据 entry-timeout-v3.json，不是 15 秒到期或旧清理残留故障。

## 可证事实、推断与候选

已证事实：命令到达 HKS；v3 在 frame=0 的根 Update 直接 Fire 登场，随后封锁普通输入，并要求动态创建的 Event20000_onActivate 设置 intro_live 才接受结束。原回归人工直接调用了这个新回调，且总是接受首个根 Update 的事件；未覆盖实际初始化顺序。

候选排序：首帧图尚未就绪而丢失事件（用户补充无登场，吻合）；晚建激活回调未被调用；关闭 TAE 导致完成标记不可用。第三项静态检查表明 TAE 更新分支与 clip-end 通知分支独立，不能将 TAE=false 直接归为根因。前两项尚没有实机逐回调日志，因此不宣称已证明唯一运行根因。

新 test_c0010_entry.lua 明确模拟根 Update 先于初始状态就绪、不主动注入 Event 激活回调。v3 红测 entry event was lost before initial state could receive it；v4 绿测。此测试是覆盖调度边界的实际 HKS seam，不把模拟的调度顺序冒充游戏实测。

v4 根 Update 只准备进入请求；由已有 Idle/Move 状态回调发出 W_Event20000。实际 Event20000/20010_CMSG 激活且先观察到未结束、再观察到结束才 ready/done，不依赖动态添加的 onActivate，不用固定时长伪造完成。原 c0010 Event20000 的 TDB 调用由生命周期持有，避免提前离开。DLL 状态机、3 秒保护超时、TAE 开关、行为图、窗口数据、相机均不改。原有动作、连段、攻击结束后退出回归保留。

## 验证与预防

verification-v4.json 保存旧/新私有脚本哈希及红绿输出；语法、五动作/九衔接、三种 chunk 名、锁定方向、combo、等待当前攻击/退场、错误节点结束信号均 PASS。hks-animation-end-static.json 固定 EXE 及 CMSG 分支证据。没有重新编译或伪称重新验证未变化 DLL；无游戏操作。

后续修改生命周期必须保留初始化调度与真实节点身份测试，不能直接由测试调用新增回调就认定引擎会调用它。临时有界阶段日志保留到用户实机确认后再清理。仍待人测：登场实际播放、节点结束标记、可操控、攻击后 F7、自然到期；完整 c0000 起手/特效仍未移植。
