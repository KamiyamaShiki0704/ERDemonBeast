# c0010 公共脚本与适配器分离

状态：用户回复“同意”授权实施；属于保持行为的加载结构重构。继续使用 spec.md@1.2、spec-custom-forms.md@1.0、spec-effect-controls.md@1.0、spec-shared-health.md@1.0 的已确认语义及人工阈值，不新增玩法规格。DLL0.4.2/HKSv9 的人工验收只证明原组合，不外推为新加载组合的实机验收。

## 加载契约

Characters/c0010.hkx 的 luaFiles 从 `[d0010, modifier, c0010, c0010]` 改为 `[c9997, d0010, modifier, c0010, c0010]`，均带 `.hks` 后缀。引擎依次加载公共函数和适配器，再执行原角色脚本。保留 modifier 和两次原角色加载，保持适配器相对于原脚本的位置。适配器捕获公共函数，公共脚本内部局部变量仍由其原函数闭包持有。运行代码不使用 dofile，也不修改全局 debug 函数。

`d0010.hks` 只安装 assets/action/script/c0010-er-adapter.hks。相比已验收完整副本，其从 ERDB_BEHAVIOR_VERSION 起的执行正文在统一换行后逐字节一致，只更新两行头注释。版本仍为行为9，DLL仍0.4.2。

共享 `c9997.hks` 中唯一 script_dir 引用是未使用的局部初始化：其对无目录或反斜杠 chunk source 执行 match 后得到 nil，再调用 gsub 会报错。本轮将这一行替换为注释，其余编码、BOM与字节保留。该修正扩展之前“共享文件保留”的写范围，依据本轮已说明路径兼容处理方案后用户“同意”的授权。没有取得引擎真实 chunk source，不声称原共享脚本在当前游戏必然报错。

## 自动证据

- prepare_split_behavior.ps1 与 split_behavior.py：从实际安装 Behbnd 提取字符文件，结构修改、序列化、回读；完整指针关系等价比对，只允许 luaFiles 差异，其他三个 binder 条目逐字节一致。
- verify_split_behavior.py：按实际 luaFiles 的 common/adapter 前缀，以独立 Lua chunk 执行现有回归，角色脚本继续加载两次；23 次调用包含三种 chunk source、44 伤害方向组合、受击打断/等待退出、20轮翻滚、连段、生命周期与映射。modifier 与 Havok 引擎仍为人工覆盖项。
- Manage-SplitBehavior.ps1：三个文件统一备份、哈希前置检查和替换；保护当前 regulation、原角色脚本、anibnd、name-ID、DLL与配置。回退也预检整组，拒绝覆盖后续修改。隔离目录六项安装/恢复/拒绝路径测试通过。

## 安装与后续维护

候选 dist/split-hks-1-r2；安装记录 research/deployment-split-hks-1.json；汇总 research/build-result-split-hks-1.json。以每份记录中的实际哈希识别文件，不使用旧 health 包的共享脚本哈希判断新加载方案。

日后只改公共函数时更新 c9997；只改适配逻辑时把适配器源文件部署为 d0010，保持上述加载顺序。公共接口改动仍需检查适配器。旧 prepare_shared_health.py 等脚本保留为历史打包工具，不能用于新加载组合；拆分迁移脚本也只接收旧四项加载列表，重复运行会主动失败。

人工检查沿用已确认操作：变身进入、攻击衔接与连续闪避、受击和共用生命、常规解除退场。用户执行游戏操作；Agent不启动游戏或读取活体内存。进入/动作结束后恢复仍以3秒、血条刷新差异仍以0.25秒为既有验收阈值，未取得实测则不记录数值PASS。
