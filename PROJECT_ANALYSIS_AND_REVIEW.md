# key-mouse-sharing 项目分析与代码评审

评审日期：2026-09-19  
评审环境：Windows 11，Node.js 24.14.0，npm 11.9.0，Rust 1.85.1  
项目版本：`0.1.0`

## 1. 执行结论

本项目是一个基于 Tauri 2、Vue 3 和 Rust 的跨平台局域网键鼠共享原型。当前已经具备“控制端采集输入 → TCP 传输 → 接收端模拟输入”的基础链路，桌面应用能够成功编译并启动，前端页面也能正常渲染。

目前仍属于 PoC/原型阶段，不建议直接在不可信网络或生产环境中使用。主要原因是：当前所谓的配对鉴权会把配对码明文发送给连接方，不能真正证明对方持有密钥；工作线程生命周期与全局状态存在竞态；启动命令会在底层监听或连接真正成功前就向前端返回成功；项目没有自动化测试。文件共享和屏幕共享尚未实现。

本次实际验证结果：

| 项目 | 结果 |
| --- | --- |
| `npm run build` | 通过，Vue/TypeScript/Vite 构建成功 |
| `cargo check` | 通过，有 4 条 warning |
| `cargo test` | 通过，但实际为 0 个测试 |
| `cargo clippy --all-targets -- -D warnings` | 未通过，4 个 warning 被提升为 error |
| `npm run tauri dev` | 成功，桌面程序已启动 |
| 页面核验 | 键鼠共享页、文件共享页、屏幕共享页均可访问；后两者为占位页 |

## 2. 产品功能现状

### 2.1 已实现

- 控制端和接收端角色选择。
- 目标 IP、端口和配对码配置，并通过 Local Storage 持久化。
- Windows/macOS 全局键鼠事件采集的平台适配层。
- 基于 TCP 的一对一连接、断线重连和输入事件传输。
- JSON 事件序列化，支持鼠标相对移动、按键、滚轮和鼠标按钮。
- 接收端通过 `enigo` 模拟键鼠输入。
- 光标到屏幕边缘时进入或退出远端控制。
- 后端通过 Tauri Event 向前端推送运行日志。
- 前端日志分类、筛选、计数与清空。
- 基础参数校验和按来源 IP 的鉴权失败限流。
- 明暗主题及侧边栏导航。

### 2.2 未实现或不完整

- 文件共享仅有占位页面。
- 屏幕共享仅有占位页面。
- 不支持设备发现、设备列表和多设备拓扑。
- 不支持连接质量、延迟、事件吞吐等指标。
- 没有 TLS/Noise、可靠的挑战应答认证或端到端加密。
- 没有协议版本、序列号、消息长度限制、心跳或 ACK。
- 没有自动化测试、Lint 脚本或 CI/CD。
- Linux 平台未实现，当前 `platform` 仅覆盖 Windows 和 macOS。

## 3. 技术架构

### 3.1 技术栈

| 层次 | 技术 |
| --- | --- |
| 桌面容器 | Tauri 2 |
| 前端 | Vue 3、TypeScript、Vite 6 |
| UI | Tailwind CSS 4、Reka UI、Lucide、项目内 shadcn 风格组件 |
| 路由 | Vue Router，Memory History |
| 本地状态 | Vue `ref/computed` 与 VueUse `useStorage` |
| 后端 | Rust |
| 输入采集 | `rdev`、Windows Hook / macOS CoreGraphics |
| 输入模拟 | `enigo` |
| 网络 | Rust 标准库 `TcpListener` / `TcpStream` |
| 序列化 | Serde + JSON Lines 风格文本帧 |

### 3.2 模块划分

- `src/App.vue`：应用壳层、侧边栏和路由出口。
- `src/pages/KeyMouseSharing.vue`：配置、启动/停止命令、运行日志与前端状态。
- `src/pages/FileSharing.vue`、`ScreenSharing.vue`：未开发功能的占位页。
- `src-tauri/src/lib.rs`：Tauri 初始化、插件和 command 注册。
- `src-tauri/src/mouse_share.rs`：连接、握手、限流、共享状态、事件队列和数据传输。
- `src-tauri/src/logic.rs`：跨平台事件模型及 `enigo` 输入模拟封装。
- `src-tauri/src/platform/win.rs`、`mac.rs`：平台输入监听、按键映射与光标操作。

### 3.3 运行数据流

1. 用户在 Vue 页面选择角色并调用 Tauri command。
2. 接收端绑定端口；控制端持续尝试连接目标 IP。
3. 双方执行当前的明文“挑战/确认”握手。
4. 控制端启动全局输入监听；鼠标触及右边缘后进入共享态。
5. 输入事件进入 `VecDeque`，Serde 序列化后包装为 `DATA <json>\n`。
6. 接收端逐行读取并使用 `enigo` 执行输入。
7. 接收端光标到达左边缘时发送 `DATA RELEASE`，控制端退出共享态。

### 3.4 当前协议

握手：

```text
接收端 -> 控制端: CHALLENGE <pair_code>\n
控制端 -> 接收端: AUTH_OK\n 或 AUTH_FAIL\n
```

数据：

```text
DATA <serde_json_event>\n
DATA RELEASE\n
```

该协议没有加密、完整性保护、消息长度上限、协议版本、序列号或重放保护。

## 4. 启动与使用

### 4.1 环境要求

- Node.js、npm。
- Rust stable 工具链。
- Tauri 2 对应的平台编译依赖。
- Windows/macOS 的全局输入监听和输入模拟权限。

### 4.2 开发启动

```powershell
npm install
npm run tauri dev
```

仅查看前端页面可运行：

```powershell
npm run dev
```

浏览器模式无法调用 Tauri 后端，因此不能实际共享键鼠。

### 4.3 构建检查

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

### 4.4 使用注意事项

- 当前默认配对码 `1234567` 不符合程序自身校验规则，必须先改为至少 8 位且同时包含字母和数字，例如随机生成的长配对码。
- 接收端需要允许防火墙入站访问所选端口。
- 两台设备必须能够通过 IP 和端口互相访问。
- Windows/macOS 可能要求辅助功能、输入监控等权限。
- 当前协议不安全，只应在隔离、可信的测试网络中使用。

## 5. Code Review 问题清单

### P0：发布阻断问题

#### 5.1 配对码由接收端直接明文发送，鉴权可被绕过

位置：`src-tauri/src/mouse_share.rs:154-220`

接收端主动发送 `CHALLENGE <pair_code>`，控制端只需返回 `AUTH_OK`。这意味着任何能连接端口的客户端都会先收到正确配对码，然后可以直接回复成功；旁路监听者也能取得配对码。当前限流无法补救这一设计，因为秘密在鉴权前已经泄露。

影响：不可信的局域网节点可能建立连接并注入键鼠事件，属于远程输入控制场景中的严重安全问题。

建议：使用随机 nonce + HMAC(pair_code, nonce) 的挑战应答，至少做到不传输秘密本身；正式版本优先使用 TLS/Noise，并绑定首次配对得到的设备公钥。

#### 5.2 共享会话使用全局布尔状态，停止后快速重启会“复活”旧线程

位置：`src-tauri/src/mouse_share.rs:21-27, 223-245, 474-518`

所有会话共享同一个 `IS_RUNNING: bool`。`stop_sharing` 设为 `false` 后，如果旧线程尚未退出就再次启动，新启动会把它设回 `true`，旧线程可能继续运行。每次控制端启动还会新建永久性的底层事件监听线程。

影响：重复启停后可能出现多个监听/发送线程、重复事件、端口冲突、状态错乱，甚至停止按钮无法彻底停止旧会话。

建议：用会话对象管理取消令牌、socket 和 JoinHandle；每次启动分配不可复用的 generation/session ID；启动前原子检查当前状态，停止时取消并等待线程退出。

#### 5.3 启动 command 在实际监听/连接成功前返回成功

位置：`src-tauri/src/mouse_share.rs:238-267, 460, 463-483, 618-633, 773`；`src/pages/KeyMouseSharing.vue` 的 `start()`

Rust command 只是创建线程就返回 `Ok(())`。接收端即使绑定端口失败，前端也会把 `isRunning` 设为 `true` 并提示“已启动”；控制端尚未连接目标时也被 UI 表示为已运行。

影响：UI 状态和真实状态不一致，失败只能从异步日志发现；用户可能误以为键鼠共享已经生效。

建议：引入 `Idle/Starting/Listening/Connecting/Connected/Sharing/Stopping/Error` 状态机。接收端至少在 bind 成功后再返回；控制端区分“后台重连中”和“已连接”。

### P1：高优先级问题

#### 5.4 默认配对码必定校验失败

位置：`src/pages/KeyMouseSharing.vue` 中 `useStorage('kms-pair-code', '1234567')`

默认值只有 7 位且没有字母，而前后端均要求至少 8 位并包含字母和数字。首次启动按默认配置点击启动一定失败。

建议：默认留空并引导生成随机配对码，或提供符合要求的开发默认值；生产环境不应硬编码共享秘密。

#### 5.5 数据帧无长度限制，可导致内存耗尽

位置：`src-tauri/src/mouse_share.rs:348-364, 661-669`

`BufRead::read_line` 会持续增长 `String`，协议没有最大帧长度。已连接的恶意或故障节点可以发送不带换行的超大数据。

建议：改为长度前缀协议并设置严格上限，或使用 `take(max + 1)`/限长 codec；超过上限立即断开。

#### 5.6 事件队列没有容量上限

位置：`src-tauri/src/mouse_share.rs:492-496, 546-548, 577-584`

`VecDeque` 是无界队列。网络卡顿或接收端处理变慢时，输入事件会持续堆积，增加内存占用并造成恢复后的巨大输入延迟。

建议：使用有界 channel；鼠标移动事件可合并/覆盖，按键和按钮事件保序；达到阈值时丢弃可合并事件并上报指标。

#### 5.7 连接断开后，按键/鼠标按下状态可能残留

位置：`src-tauri/src/mouse_share.rs:357-448, 703-749`

断线或解析失败时没有维护并释放当前按下的键和鼠标按钮。如果 `KeyDown` 已执行而 `KeyUp` 丢失，接收端可能保持“按键卡住”。

建议：会话内跟踪 pressed keys/buttons，在 RELEASE、断线、超时和 stop 时统一执行释放；协议增加序列号和状态同步帧。

#### 5.8 CSP 被完全关闭，且 opener 权限可能不是当前功能所需

位置：`src-tauri/tauri.conf.json:22-24`，`src-tauri/capabilities/default.json:8-13`

`csp: null` 放宽了 WebView 攻击面；当前业务代码未见 opener 使用，却启用了 `opener:default` 和插件。

建议：配置最小 CSP，移除未使用插件与 capability，并定期审查 Tauri 权限清单。

#### 5.9 后台监听线程中的 `unwrap` 可能导致无提示退出

位置：`src-tauri/src/platform/win.rs:240,292` 及多个 Mutex lock/序列化调用

Windows 的 `rdev::listen(callback).unwrap()` 遇到权限或底层监听错误会 panic；大量 poisoned Mutex `unwrap` 也可能放大单次 panic。

建议：将错误转为结构化运行事件；线程退出时更新状态并允许安全重试；避免在长期运行线程中依赖不可恢复的 `unwrap`。

### P2：工程质量与体验问题

#### 5.10 没有测试，当前 `cargo test` 的“通过”是 0 tests

至少应覆盖：配对码校验、帧编解码、错误/超长帧、鉴权挑战应答、事件顺序、断线释放、状态机和重复启停。网络部分可抽象 transport 后做 loopback 集成测试。

#### 5.11 Clippy 严格检查失败

当前问题包括：未使用的 `hide_cursor`/`KeyEvent` import、未使用函数、未使用 `AUTH_PREFIX`、无效的 `connection_broken` 赋值。普通 `cargo check` 仅给出 warning，但 `-D warnings` 会失败。

#### 5.12 高频坐标轮询（已于 2026-09-19 修复）

原实现位置：`src-tauri/src/mouse_share.rs` 的鼠标采样线程与发送循环

原实现通过 1ms 定时线程读取共享坐标并反复把光标移回中心。平台事件回调与定时线程不同步，会产生零位移、重复回中和批量位移，是跨屏后鼠标卡顿的重要原因。

现已改为由真实鼠标移动事件直接计算相对位移，并合并同一发送周期内相邻的移动事件；序列化和网络写入也不再持有输入队列锁。发送主循环仍保留 1ms 检查，后续可进一步改为 channel/条件变量驱动以降低空闲功耗。

#### 5.13 前端错误解析不可靠

位置：`src/pages/KeyMouseSharing.vue` 的 `catch (error: any)`

Tauri invoke 的拒绝值可能是字符串，不一定有 `message` 字段；当前会把部分真实错误显示为“未知错误”。

建议：统一 `normalizeError(error)`，优先处理 string、Error 和可序列化对象。

#### 5.14 README 仍是脚手架模板，已有 `arch.md` 也已落后于实现

README 没有项目介绍、权限要求、真实启动步骤、网络模型和安全警告；`arch.md` 未描述当前配对、限流、日志和线程模型。

#### 5.15 产品命名容易混淆

代码中 `start_mouse_server` 表示主动连接并发送输入的“控制端”，`start_mouse_client` 反而是绑定端口的“接收端”。这与常见的 server=监听端、client=主动连接相反，增加维护成本。

建议：改名为 `start_controller` / `start_receiver`，网络角色另用 `connector` / `listener` 描述。

## 6. 做得较好的部分

- 已把平台差异收敛到 `platform` 模块，业务层事件模型相对清晰。
- 事件队列已经使用 FIFO `VecDeque`，避免旧版本 LIFO 导致的按键顺序问题。
- 网络读取设置了超时，监听 socket 使用非阻塞模式，停止响应比纯阻塞实现更好。
- 前后端都执行配对码基本校验，且接收端加入了按 IP 的失败限流。
- 后端运行日志已通过 Tauri Event 接入前端，日志数量也有上限。
- 前端区分 processing/running，能阻止界面上的重复点击。
- 前端构建与 Rust 编译目前均能成功完成。

## 7. 推荐改造路线

### 第一阶段：保证可控和可测

1. 建立单一会话状态机及可取消任务模型，修复重复启停竞态。
2. 修复 command 过早返回和前端状态失真。
3. 修复默认配对码，统一错误解析。
4. 为纯函数与协议增加单元测试，为 loopback 连接增加集成测试。
5. 清理 Clippy 错误并把 build/check/test/clippy 加入 CI。

### 第二阶段：修复安全与可靠性

1. 用 nonce + HMAC 或 TLS/Noise 替换明文配对码握手。
2. 增加协议版本、长度前缀、最大帧、序列号、心跳和超时。
3. 使用有界队列和鼠标移动合并策略。
4. 断线/停止时释放所有按下状态。
5. 收紧 CSP 和 Tauri capabilities。

### 第三阶段：完善产品能力

1. 设备发现与可信设备管理。
2. 文件共享：分片、校验、进度、取消和断点续传。
3. 屏幕共享：捕获、编码、传输、解码与权限引导。
4. 增加延迟、吞吐、重连次数和错误原因等可观测指标。

## 8. 验收建议

进入可发布阶段前，至少应满足：

- 重复执行 100 次启动/停止不产生多余线程、重复输入或端口占用。
- 断网、进程退出、协议错误后不会留下按键或鼠标按钮按下状态。
- 未持有密钥的节点无法通过抓包或简单重放完成鉴权。
- 超长帧、高速事件和慢接收端不会造成无界内存增长。
- 单元测试、网络集成测试、前端构建和严格 Clippy 在 CI 中全部通过。
- Windows 与 macOS 的权限拒绝、授权、重启和异常恢复路径均有实机验证。
