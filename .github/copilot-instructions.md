# GitHub Copilot 指令（中文）

目的
- 让 GitHub Copilot 在此仓库中以中文（简体）成为一个高效的编码协作伙伴。

核心规则（必须遵守）
- 语言：所有**思考、推理过程**与对外**回答/建议**（包括注释、PR 描述、提交信息示例以及用户可见文本）均使用中文（简体）。
- Git 提交信息：代理生成的 Git 提交信息请使用中文。推荐保留 Conventional Commits 的 type（如 `feat`, `fix`, `chore`, `docs`, `refactor`），但 commit 描述主体必须为中文。例如：

  - feat(ui): 新增屏幕共享按钮和快捷键支持
  - fix(mouse): 修复在 Windows 下鼠标坐标偏移的问题

  如果你希望完全本地化 type（例如使用 `新增`/`修复`），请先与项目维护者沟通并在本文件更新说明。

- 最小化重复：如果仓库已有文档（如 README.md、arch.md、AGENTS.md），请链接到它们而非复制大量内容。

快速运行与构建命令
- 前端开发（在项目根目录）：

```bash
npm run dev
npm run typecheck
npm run build
npm run preview
npm run audit
```

- Tauri / 原生（位于 `src-tauri/`）：

```bash
# 使用 tauri CLI
npm run tauri
# 或直接使用 cargo
cd src-tauri && cargo build
```

重要行为建议
- 在影响范围较大的改动前，用中文在 PR 描述中总结变更动机、回归测试步骤与需要注意的点。
- 提交前运行 `npm run typecheck`；对于 Tauri 相关改动，也请尝试编译 `src-tauri` 以捕捉原生层问题。
- 遵循仓库编码风格；不确定时提交小改动并在 PR 中说明假设与测试方法。

引用与链接
- 参见仓库指导：[AGENTS.md](AGENTS.md) 、[README.md](README.md) 、[arch.md](arch.md)

如果你希望我把提交类型也完全中文化，或为前端/原生分别生成更详细的 Copilot 指令模板（并在全局同步），请回复我你的偏好，我会更新本文件并提供一键部署脚本。
