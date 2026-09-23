# c0010 Behbnd/HKS 同步适配 v1

输入：用户确认继续同步 Behbnd 与 HKS，并提示核对三个 name-ID 文件。基于此前 0.2.2 实测已证实原始输入进入兽体；保留 Spec 1.0 的控制/恢复语义，不将本候选当作完整战斗验收。

## 结构化证据与改动

用 HKLibCLI 0.1.3-NR 将安装 Behbnd 的四个 HKX 读成 XML，追踪 hkbStateMachine 的事件索引与 wildcardTransitions，确认十个动作的事件→目标状态；对应 CustomManualSelectorGenerator 的 animation ID 正确、enableScript/enableTae=true、animeEndEventType=2（FireIdleEvent）。同步检查安装 anibnd 的 TAE 中存在这十个 ID。

Characters/c0010.hkx 的 luaFiles 原为 c9997.hks、modifier.hks、c0010.hks、c0010.hks。仅将第一个改为同长度 d0010.hks，字节偏移 1758 起五个字节变化，文件长度及全部结构偏移保持不变。封装为 BND 后重新读取：另三条 entry 字节相等，所有 entry 的名称/ID/flags 相等，修改 entry 与候选字节相等；修改的 HKX 再用 HKLib 解析核对 luaFiles。

私有 d0010.hks 由现用 GA c9997.hks 原字节加本项目 c0010-er-adapter.hks 生成。原版 c0010.hks 按 Behbnd 原加载顺序保留并继续覆盖其角色事件回调。适配补足 Event20000 依赖的 AttackCommonFunction_TDB，不导入 NR 的特殊生命/充能/解除接口。绑定依靠此角色私有脚本引用，当前无需把 NR SpEffect 707100 的未知引擎效果带进 ER。

新增 Lua 局部状态不需要写入 variablenameid。三张表针对四个 HKX 真正引用的名称取差集，保留原 ID/名称/顺序/编码/注释和 EOF NUL，追加名称并更新 Num：event 2915→2944（+29）；state 2390→2411（+21）；variable 667→681（+14）。具体新增项、输入及输出 SHA256 见 behavior-build.json。追加登记只解决名称引用，不证明 NR 跳跃等功能已移植。

## 行为范围与限制

保留原版地面动作选择：四段轻击、两段重击、3030/3035、6000→3010 及九条已测衔接。ER R1/R2/L1/L2=0/1/2/3；消费 ER ActionRequest 或 ActionCancelRequest，按住防止重复启动，松开在 Update 重新使能。精力不足拒绝；原 StaminaComboResetTest=1 时重置连段。测试 NpcParam[100000] stamina=50、恢复基础值21，只读核对，未再修改参数。

ER 调试控制器现场提供 SP_MOVE=5，当前适配将其按下映射到 6000；不是完整的玩家长按冲刺/松键闪避语义。ER 17 不能当 NR ULTRA 17；本包不实现 ULTRA、跳跃和完整空中/落地逻辑，也不承诺 NR 攻击 TAE 的伤害/无敌帧已经兼容。完整移动动画修正另需运行反馈。

## 离线反馈与自动结果

`test_c0010_adapter.lua <GA c9997.hks> <原版 c0010 反编译文本> baseline` 在真实 IdleDefault_onUpdate 调用处因缺 W_Attack3000 失败。替换为最终打包的 d0010.hks，并重复加载原版 c0010 两次后，同一调用通过；覆盖五个输入路由、九条连段、取消窗口关闭、按住/松开、缺动画、精力重置、死亡优先、AI 回退、Event20000 依赖和 ER/NR 输入编号冲突。

这是运行实际 HKS 的离线测试，env/act 是引擎环境桩，行为图播放由人验证。没有把缺省 env=0 当作合法无伤害状态：GetReceivedDamageType 使用 DAMAGE_TYPE_NONE=99999，事件请求使用 -1，动画存在性按 c0010 已有 ID 限定。

Python 检查覆盖原编号保持、已有编码与 EOF NUL、幂等、错误 Num/重复 ID 拒绝，以及实际 PowerShell 安装/回滚脚本在临时五文件 fixture 上的完整执行和“后续改动拒绝覆盖”。安装脚本对五个目标全部预检，复制备份后原子替换，安装失败尝试还原已应用项。资源包、原版 c0010、公用 c9997、anibnd 和 regulation 均有哈希约束。

游戏未由代理启动或操作；最终人工步骤见 BEHAVIOR_TEST.md。交付为资源候选 v1，沿用已安装的 0.2.2 DLL。
