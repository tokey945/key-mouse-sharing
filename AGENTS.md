# 仓库代理指引（AGENTS）

目的
- 让 AI 编码代理能在此仓库中立即高效工作；默认使用中文（简体）进行所有思考与回答。

核心约定
- 语言：所有代理的“思考、推理、建议与回答”均使用中文（简体）。
  - 注意：若希望在本地 VS Code 全局生效，请参考并部署用户级 prompts（见下文“运行与构建（快速参考）”后的说明）。
- Git 提交信息：自动或代理生成的 Git 提交信息必须为中文。推荐遵循 Conventional Commits 风格，保留英文 type（如 `feat`, `fix`, `chore` 等）以便工具兼容，但 commit 描述主体请使用中文，例如：
  - `feat(ui): 新增屏幕共享按钮和快捷键支持`
  - `fix(mouse): 修复在 Windows 下鼠标坐标偏移的问题`
- 最小化重复：若仓库已有文档（如 README、arch.md、docs/** 等），请链接它们而非复制全文。

运行与构建（快速参考）
- 常用 npm 脚本（项目根目录）：
  - `npm run dev` — 本地开发（Vite）
  - `npm run typecheck` — 类型检查（vue-tsc）
  - `npm run build` — 构建（含 typecheck）
  - `npm run audit` — 依赖安全审计
  - `npm run preview` — 预览构建产物
  - `npm run tauri` — 运行 tauri CLI（用于桌面打包/运行）
- Tauri / Rust：Tauri 代码位于 `src-tauri/`，原生构建可使用 `cargo build` 或 `npm run tauri`（参见 `src-tauri/` 和 `tauri.conf.json`）。

代码位置速查
- 前端（Vue + Vite）：`src/`
- 原生桌面（Tauri / Rust）：`src-tauri/`
- 文档与架构：请参阅 [README.md](README.md) 和 [arch.md](arch.md)

行为建议（对 AI 代理）
- 变更有风险或影响广泛时，先在 PR 描述中用中文总结变更动机与回归测试步骤。
- 遵循仓库既有编码风格；若不确定，先打开一个小型 PR 并在 PR 描述中说明你的假设。
- 提交前运行 `npm run typecheck` 并在可能时运行本地应用以验证。对于 Tauri 相关改动，同步编译 `src-tauri`。

如果你想创建更细的 agent 指令（例如分别针对前端/后端/构建流程），请告知，我可以基于此文件再生成子级指引或自定义 skill/agent。

---
（说明：本文件遵循“链接而非复制”的原则，指向仓库内已有文档以避免重复。）
