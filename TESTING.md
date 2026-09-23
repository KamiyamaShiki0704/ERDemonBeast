# 构建、自检与测试入口

当前候选为 **0.2.1**。游戏测试由用户手动执行；启动顺序、日志收集与六组 Probe 见 [MANUAL_TEST.md](MANUAL_TEST.md)。根目录配置两个开关都默认关闭；手动启动器为每轮生成独立配置，不修改现用 GoldenAge profile。

## 离线检查

```powershell
cargo test --locked --features native
cargo clippy --locked --features native --all-targets -- -D warnings
cargo fmt -- --check
cargo build --release --locked --features native
python -m unittest discover -s Scripts/delivery/feat-er-demon-beast/script -p 'test_*.py' -v
rustc Scripts/delivery/feat-er-demon-beast/script/dll_thread_smoke.rs -o target/dll-thread-smoke.exe
./target/dll-thread-smoke.exe "$PWD/target/x86_64-pc-windows-msvc/release/er_demon_beast.dll"
```

必须带 native 特性构建游戏 DLL。当前为 20 项 Rust + 10 项 Python 测试，另有真正加载 DLL 的小栈线程回归。Summon 的两份源码和三个原生 fixture 按 provenance.json 固定；新实体注册、任务注销、脱离与原分配器释放沿用其代码；引用排空使用 src/retirement.rs 适配副本，保留原件。

## 判断边界

- 小栈宿主证明已确认的 DllMain 栈溢出修复有效；宿主 EXE 哈希不匹配，所以不会注册 ER 任务。
- 字段/指针与调度防护、创建归属检查、帧末释放已通过离线检查，不等于本项目实机通过。
- 第二例 EXE+1F109C4 访问异常尚未证明根因；需要用户按加载/观察/控制三个阶段定位。
- 真实析构必须看到 `outcome=destroyed retirement=frame-end-drained`；隔离或超时不是释放成功。
- 切图、传送、死亡、联机、NR 外观和完整伤害结算仍不在首轮范围。

构建记录见 [build.md](docs/feat-er-demon-beast/build.md)；历史崩溃证据见 [dll-self-audit.md](docs/feat-er-demon-beast/research/dll-self-audit.md)。
