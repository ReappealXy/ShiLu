# 拾录技术架构

## 1. 技术选型

- **桌面容器：Tauri 2**：生成 Windows EXE，体积和内存占用较小，Rust 负责安全的本地文件操作。
- **前端：Vue 3 + TypeScript + Vite**：适合快速实现编辑器、列表、弹窗和动画。
- **组件：PrimeVue 4 + PrimeIcons**：提供稳定的按钮、对话框、标签、进度和通知组件。
- **状态：Pinia**：管理资料列表、编辑会话、识别任务和设置状态。
- **本地逻辑：Rust**：目录创建、文件读写、图片复制、搜索索引和密钥保护。
- **内容格式：Markdown + YAML Front Matter**：人类可读、易备份、易被 AI 和其他笔记软件读取。
- **设置格式：JSON**：保存资料库路径、界面偏好和模型连接配置。
- **搜索：内存索引 + `search-index.json` 缓存**：第一版不使用 SQL；启动时扫描 Markdown，修改后增量更新缓存。

## 2. 推荐目录

```text
ShiLu/
├─ docs/
├─ src/                    # Vue 页面和组件
├─ src-tauri/
│  └─ src/                 # Rust 命令与文件服务
├─ public/
└─ package.json
```

## 3. 功能模块

- `library`：资料列表、筛选、搜索、排序。
- `capture`：图片导入、预览、排序和识别任务。
- `editor`：Markdown 编辑和结构化字段编辑。
- `ai`：视觉识别、润色、摘要和标签生成；统一适配 OpenAI 兼容接口。
- `storage`：资料文件夹、图片、草稿和版本文件的读写。
- `indexer`：关键词索引生成、增量更新和重建。
- `settings`：模型配置、资料库路径、主题和备份操作。

### 设置文件位置

- 应用配置位于 Windows 应用配置目录，文件名为 `library.json`，保存当前资料库路径和主题偏好。它让软件在重启后仍知道资料库在哪里。
- 资料库根目录保留 `settings.json` 和 `search-index.json`。这两个文件与 `articles/` 一起移动或备份，保持资料库目录本身完整、可迁移。

## 4. AI 接口边界

前端不直接保存明文 Key。调用流程为：前端发起任务 -> Rust 读取本地加密设置 -> Rust 发送请求 -> 返回结构化 JSON -> 前端显示可编辑草稿。

模型适配器至少支持：`base_url`、`api_key`、`model`、`temperature`、超时和重试次数。视觉请求使用图片文件转 Base64 或服务商接受的本地上传格式。服务端返回必须经过字段校验，失败时保留原始 OCR 文本。

## 5. Windows 打包

- 开发：`npm run tauri dev`
- 前端检查：`npm run build`
- Windows 安装包：`npm run tauri build -- --bundles nsis`
- 发布格式：优先 NSIS 安装包，后续可补 MSI。
- 首次启动引导用户选择资料库目录；默认目录放在用户文档下的 `ShiLuData`。

## 6. 安全与恢复

- API Key 使用 Windows DPAPI 或 Tauri Stronghold 加密，界面只显示掩码。
- 任何 AI 失败都不能覆盖用户现有正文。
- 保存采用临时文件写入后原子替换，防止中断导致 Markdown 损坏。
- 提供“打开资料库目录”和“重建搜索索引”操作。

## 7. 已实现的截图粘贴与单实例（2026-09-11）

- 新建和编辑页监听用户触发的 `paste`，普通文字粘贴照常；“粘贴图片”按钮调用 Rust 的 `read_clipboard_image`，不轮询剪贴板。
- `arboard` 读取 Windows 图片，`image` 编码 PNG；前端保存预览及原始 Base64。`create_article_with_sources` / `import_article_sources` 接收路径和 PNG 两种来源，在后台线程验证、按顺序写入本条资料的 `images/`。
- 用户确认后才保存图片；OCR 仍由用户手动发起。追加图片不重载正文，OCR 不自动替换 Markdown。WinRT OCR 调用前将 Rust 的 Windows 规范路径转换成该接口接受的格式。
- `tauri-plugin-single-instance` 在其余插件之前注册，以应用标识 `com.shilu.desktop` 限制重复启动；第二个进程请求显示、还原、聚焦原窗口后退出。回调复用托盘恢复逻辑，不导航或重建页面。
- 回归入口：`node scripts/check-clipboard.mjs`（先启动 Vite，外部 Chrome 模拟 IPC），`scripts/check-clipboard-ocr.ps1`（真实 Windows OCR、隔离资料库），`scripts/check-single-instance.ps1`（隔离原生进程）。均不覆盖用户资料或剪贴板。
