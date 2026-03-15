# key-mouse-sharing 项目评审文档

评审日期：2026-03-08  
项目路径：`/Users/tokey/CursorProjects/key-mouse-sharing`

## 1. 评审范围与方法

本次评审覆盖：
- 前端（Vue 3 + Tauri 调用层）
- Rust 核心逻辑（键鼠捕获、网络传输、事件模拟）
- 构建配置与安全配置
- 文档与工程化完整度

执行方式：
- 静态代码审查（逐文件）
- 构建验证：
  - `npm run build -- --outDir /tmp/kms-dist`（通过）
  - `cargo check --manifest-path .../src-tauri/Cargo.toml --target-dir /tmp/kms-target`（通过，13 条 warning）

## 2. 执行摘要

该项目已经具备「键鼠共享」最小闭环原型（控制端 -> TCP -> 客户端），但稳定性和工程化水平仍处于 PoC 阶段。

核心结论：
- 功能可跑通的前提较多，存在若干高风险实现问题，会导致共享中断、按键丢失或事件顺序异常。
- 文件共享/屏幕共享仍是占位页面，尚未进入可交付开发状态。
- 安全、可观测性、协议设计、测试体系基本缺失，不适合直接面向真实局域网场景发布。

## 3. 当前架构概览

### 3.1 技术栈

- 前端：Vue 3 + Vite + TypeScript + shadcn-vue/reka-ui
- 桌面容器：Tauri 2
- 后端：Rust（`rdev` 捕获输入、`enigo` 事件模拟、TCP 传输）
- 路由：`vue-router`（memory history）

### 3.2 核心流程（当前实现）

1. 前端调用 Tauri command：`start_mouse_server/start_mouse_client/stop_sharing`。
2. 控制端（server 角色）监听本机输入，在光标触达右边缘后进入共享状态。
3. 事件序列化为 JSON（按行分隔）通过 TCP 发送。
4. 客户端接收事件并调用 `enigo` 模拟键鼠输入。
5. 客户端光标到左边缘时发送 `RELEASE`，控制端恢复本地光标。

## 4. 关键问题清单（按优先级）

### P0（应立即修复）

1. 事件监听线程可能提前退出，导致后续无法进入共享  
定位：[mouse_share.rs:164](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:164)、[mouse_share.rs:165](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:165)  
问题：在未连接状态直接 `return`，会结束监听线程而不是等待连接。  
影响：首次连接时机稍有不对就可能永久失效，表现为“启动成功但无法共享”。

2. macOS `KeyA` 被硬编码过滤  
定位：[mac.rs:385](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/platform/mac.rs:385)、[mac.rs:397](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/platform/mac.rs:397)、[mac.rs:445](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/platform/mac.rs:445)  
问题：`if keycode != 0` 会丢掉 keycode=0（即 `KeyA`）。  
影响：A 键按下/抬起无法转发。

3. 事件队列使用 `Vec::push + pop` 导致 LIFO（逆序）发送  
定位：[mouse_share.rs:211](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:211)、[mouse_share.rs:329](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:329)  
问题：后入先出会打乱原始事件时序。  
影响：按键组合、按下/抬起顺序可能异常，造成输入行为不确定。

### P1（高优先级）

1. 停止共享不够及时，阻塞点较多  
定位：[mouse_share.rs:22](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:22)、[mouse_share.rs:40](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:40)、[mouse_share.rs:56](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:56)、[mouse_share.rs:283](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:283)  
问题：`stop_sharing` 只改布尔值，`accept/read_line` 阻塞期间无法快速响应。  
影响：用户点击“停止”后体验滞后，状态不一致。

2. 进入共享时可能出现首帧大跳变  
定位：[mouse_share.rs:157](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:157)、[mouse_share.rs:176](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:176)、[mouse_share.rs:197](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:197)  
问题：`last_x/last_y` 与边缘切换时坐标状态未做原子重置。  
影响：首次 `MoveDelta` 可能非常大，客户端光标跳跃。

3. 明文 TCP、无鉴权、无加密  
定位：[mouse_share.rs:36](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:36)、[mouse_share.rs:269](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/mouse_share.rs:269)  
问题：局域网任意可达方理论上可注入控制事件。  
影响：存在安全风险，不满足生产发布要求。

4. Tauri 安全策略过宽（CSP 关闭）  
定位：[tauri.conf.json:23](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/tauri.conf.json:23)  
问题：`"csp": null`。  
影响：扩大潜在攻击面。

### P2（中优先级）

1. 前端日志为模拟数据，缺乏真实可观测性  
定位：[KeyMouseSharing.vue:23](/Users/tokey/CursorProjects/key-mouse-sharing/src/pages/KeyMouseSharing.vue:23)、[KeyMouseSharing.vue:125](/Users/tokey/CursorProjects/key-mouse-sharing/src/pages/KeyMouseSharing.vue:125)  
问题：日志未与 Rust 实时事件联动。  
影响：排障困难，UI 状态与真实运行状态可能不一致。

2. 文件共享/屏幕共享仍是占位  
定位：[FileSharing.vue:3](/Users/tokey/CursorProjects/key-mouse-sharing/src/pages/FileSharing.vue:3)、[ScreenSharing.vue:3](/Users/tokey/CursorProjects/key-mouse-sharing/src/pages/ScreenSharing.vue:3)

3. 工程化不足（无 lint/test 脚本）  
定位：[package.json:6](/Users/tokey/CursorProjects/key-mouse-sharing/package.json:6)  
问题：缺少 `lint/test` 自动门禁，回归风险高。

4. README 仍为模板内容，缺少真实项目说明  
定位：[README.md:1](/Users/tokey/CursorProjects/key-mouse-sharing/README.md:1)

5. Rust 侧存在未使用代码/导入与 `static mut` 警告  
定位：[platform/mod.rs:1](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/platform/mod.rs:1)、[mac.rs:209](/Users/tokey/CursorProjects/key-mouse-sharing/src-tauri/src/platform/mac.rs:209)

## 5. 优化与升级建议

### 5.1 短期（1-2 周，先稳住可用性）

1. 改造共享状态机  
建议状态：`Idle -> Connecting -> Connected -> Sharing -> Stopping`。  
收益：消除“连接状态+共享状态”分散在多线程布尔值中的竞态。

2. 修复事件链路的三个关键点
- `return` 改为可恢复等待（或 `continue` + 连接条件变量）。
- 队列改 `VecDeque`，统一 FIFO。
- 进入共享瞬间重置 `last_x/last_y` 并忽略首个 synthetic move。

3. 停止逻辑可中断化
- listener/reader 设置超时或非阻塞。
- 通过 channel/原子标志广播 stop 信号。
- stop 后主动 shutdown socket。

4. 修正键盘映射问题
- 移除 `keycode != 0` 过滤。
- 未识别键统一上报 `Unknown(code)`，便于后续补全映射。

### 5.2 中期（2-4 周，提升可靠性与安全）

1. 升级协议层
- 帧结构加入：`version/seq/timestamp/device_id/event_type/payload`。
- 加入 `heartbeat + ack + reconnect backoff`。
- 对单连接的消息长度和速率做限制。

2. 加入配对与鉴权
- 最低方案：首次配对码 + HMAC。
- 更优方案：Noise/TLS（可选证书 pinning）。

3. 可观测性建设
- Rust 侧统一日志层（建议 `tracing`）。
- 通过 Tauri `emit` 把结构化事件推到前端日志面板。
- 增加“连接质量/延迟/丢包估计”指标。

4. 安全配置收敛
- 设置明确 CSP（至少限制 script/connect 来源）。
- 精简不必要 capability 权限。

### 5.3 长期（4-8 周，功能升级）

1. 文件共享模块
- 分片传输 + 断点续传 + 哈希校验。
- 同网段自动发现设备（mDNS）+ 手动直连兜底。

2. 屏幕共享模块
- 优先实现“低帧率预览 + 输入反控”MVP。
- 后续演进为硬件编码链路（平台差异化处理）。

3. 多设备拓扑
- 从 1 对 1 升级为 1 对 N，支持设备编组和优先级切换。

## 6. 推荐实施路线图

### Phase 1（本周）

- 修复 P0 三项问题。
- 增加最小集成测试：连接建立、共享进入、释放、停止。
- README 重写为项目真实说明（运行步骤、权限要求、已知限制）。

### Phase 2（下周）

- 协议加 `seq/heartbeat`。
- 接入真实日志面板。
- 增加前端表单校验（IP、端口范围 1-65535、运行中按钮状态）。

### Phase 3（2-4 周）

- 引入配对鉴权。
- 文件共享 MVP。
- 自动化测试与发布流水线。

## 7. 附录：已执行验证

- 前端构建：成功，产物输出到 `/tmp/kms-dist`。
- Rust 校验：成功，`cargo check` 通过，但存在 13 条 warning（主要是 unused 和 `static mut` 相关兼容性警告）。

