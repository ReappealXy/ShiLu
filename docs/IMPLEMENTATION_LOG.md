# 拾录实施记录

## 第 1 步：建立 Windows 桌面项目

**状态：已完成**

完成内容：创建 Tauri 2、Vue 3、TypeScript 和 Rust 基础工程；配置 Windows 窗口、安装包目标、默认权限和拾录应用图标；安装 Visual Studio Build Tools 以支持本机 Windows 原生编译。

验证结果：

- `npm run build` 通过。
- `cargo check --manifest-path src-tauri/Cargo.toml` 在 Visual Studio C++ 开发环境中通过。
- `npm run tauri:dev` 成功启动 Vite 服务和 `shilu.exe` 开发窗口。

Git Summary：`chore: initialize ShiLu Windows desktop application`

Git Description：大白话说，拾录现在已经有了真正的 Windows 软件外壳，不是普通网页。开发时能打开桌面窗口，后续可以在里面逐步加入资料库、截图和 AI 功能。

## 第 2 步：搭建软件外壳与导航

**状态：已完成**

完成内容：实现总览、资料库、新建资料和设置四个路由页面；完成可折叠侧边栏、顶部快速入口、主题切换、提示反馈和页面切换动画；为窗口较窄时的图标侧栏、深色主题、键盘焦点和减少动效偏好补齐样式。页面使用真实的空状态，未提前写入资料库、导入、识别或 AI 功能。

验证结果：

- `npm run build` 通过，包含 Vue 类型检查和生产打包。
- `cargo check --manifest-path src-tauri/Cargo.toml` 在 Visual Studio C++ 开发环境中通过。
- `npm run tauri:dev` 成功启动 Vite 服务和 `shilu.exe` 开发窗口。
- 本地开发页面 `http://localhost:1420/` 返回 `200`；外部 Chrome 在当前环境不可用，因此未做浏览器截图检查。

Git Summary：`feat: build the ShiLu desktop shell and navigation`

Git Description：大白话说，拾录现在已经像一个完整的软件工作台了。左侧能在总览、资料库、新建资料和设置之间切换；顶部可以用 `Ctrl+K` 快速前往页面，也能切换深浅主题。真正的资料保存、图片导入和 AI 识别会从下一步开始接入，当前不会拿假数据冒充已经完成。

## 第 3 步：实现本地资料库和目录管理

**状态：已完成**

完成内容：新增 Rust 本地存储服务和前端调用层；用户可在设置中选择资料库文件夹，软件会创建 `articles/`、`.shilu-trash/`、`settings.json` 和 `search-index.json`；创建资料时会生成独立的 `index.md`、`images/`、`raw/` 和 `versions/` 子目录；删除动作先移动到资料库内的回收站；应用配置会保存资料库路径和浅色/深色主题，并使用原子写入避免中断时留下半个 JSON 文件。

验证结果：

- `npm run build` 通过，包含 Vue 类型检查和生产打包。
- `cargo check --manifest-path src-tauri/Cargo.toml` 在 Visual Studio C++ 开发环境中通过。
- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，4 项测试全部成功：目录初始化、资料结构、回收站移动、原子写入和主题配置保留。
- `git diff --check` 通过。
- `shilu.exe` 开发窗口保持运行，Vite 本地服务返回 `200`。

Git Summary：`feat: persist local library and theme preferences`

Git Description：大白话说，拾录现在能记住你把资料放在哪，也能记住你喜欢浅色还是深色界面。选好文件夹后，软件会自动准备好存放文章、图片、原始识别内容和历史版本的目录；每条资料都是一个独立文件夹，删除时先放进回收站，资料不会直接消失。

## 第 4 步：实现截图导入与素材管理

**状态：已完成**

完成内容：新建资料页支持系统图片选择器、Tauri 原生拖拽和开发环境文件选择；支持 PNG、JPG、JPEG、WEBP，多张图片缩略图预览、尺寸和文件大小展示；素材可以删除、清空、上下移动或拖动排序。填写标题后保存会先创建资料文件夹，再按当前顺序把原图复制到该资料自己的 `images/` 目录，原始文件不会被修改。

验证结果：

- `npm run build` 通过，包含 Vue 类型检查和生产打包。
- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，7 项测试全部成功，覆盖顺序追加、文章 ID 引用、格式预校验、失败清理和原图不变。
- `cargo check --manifest-path src-tauri/Cargo.toml` 通过。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 通过。
- `git diff --check` 通过（仅保留 Git 对现有换行符的提示）。

Git Summary：`feat: import and manage article images`

Git Description：大白话说，现在可以在“新建资料”里把多张截图一次放进去，看到缩略图后调整顺序、删掉不要的图片，再写标题和来源链接。点击保存后，软件会把这些原图复制到这一条资料自己的图片文件夹里，后面编辑文章时不会和其他资料混在一起。浏览器开发时拖入的图片只能预览，正式保存请使用桌面窗口里的系统选择器或原生拖拽。

## 第 5 步：实现文章编辑和 Markdown 保存

**状态：已完成**

完成内容：新增文章编辑路由和编辑页；支持标题、摘要、来源链接、标签、Markdown 正文和个人备注；支持编辑/预览切换、`Ctrl+S` 手动保存和短延迟自动保存；后端会读取和写回同一资料文件夹里的 `index.md`，保存时保留创建时间并更新修改时间，标签去重，原图目录不受影响。新建资料保存成功后可以直接进入编辑页。

验证结果：

- `npm run build` 通过，包含 Vue 类型检查和生产打包。
- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，8 项测试全部成功，新增文章字段保存/读取回归测试。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 通过。
- 编辑器的 Markdown 预览覆盖标题、段落、列表、链接、加粗和行内代码等常用内容。

Git Summary：`feat: add markdown article editor`

Git Description：大白话说，现在保存截图后可以直接打开这条资料写文章。你可以修改标题、摘要、来源、标签、正文和自己的备注，切换预览查看效果，按 `Ctrl+S` 或等待自动保存，内容会写回这条资料自己的 `index.md` 文件。

## 第 6 步：实现资料列表、详情和关键词搜索

**状态：已完成**

完成内容：资料库页现在会扫描真实的 Markdown 资料文件夹并显示列表；新增搜索框，支持在标题、摘要、正文、备注、标签、来源链接中搜索；列表按最后修改时间排序，点击一条资料可以直接进入文章编辑页；搜索无结果、资料库为空、读取失败和资料库未设置都有对应状态。

验证结果：

- `npm run build` 通过，包含 Vue 类型检查和生产打包。
- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，9 项测试全部成功，新增资料列表搜索回归测试。
- 搜索结果来自真实 `index.md` 文件，未引入 SQL；删除或损坏搜索缓存不会影响扫描列表。

Git Summary：`feat: add searchable article library`

Git Description：大白话说，现在资料库页面终于能真正找东西了。你可以按标题、正文、标签、来源或链接搜索，找到后点进去继续编辑；即使没有资料、关键词没匹配到，软件也会明确告诉你发生了什么。

## 第 7 步：接入本地 OCR

**状态：已完成**

完成内容：接入 Windows 自带 OCR 能力；文章编辑页新增本地 OCR 操作，按 `images/` 中的图片顺序识别中文或英文文字，结果保存到该资料的 `raw/ocr.txt`，编辑页可以预览识别原文并手动插入正文。没有图片、系统 OCR 不可用或单张图片识别失败时，正文和原图都不会被覆盖。

验证结果：

- `npm run build` 通过，包含 OCR 按钮和编辑页状态处理。
- `cargo check --manifest-path src-tauri/Cargo.toml` 通过，Windows OCR API 编译成功。
- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，9 项测试全部成功。
- OCR 结果写入 `raw/ocr.txt`，不写入最终正文，需用户点击“插入正文”后才会进入编辑内容。

Git Summary：`feat: add native Windows OCR workflow`

Git Description：大白话说，即使不配置大模型 Key，拾录也可以先用 Windows 自带功能读截图里的文字。识别结果会单独保存下来，你看过以后再决定要不要放进正文，识别出错也不会影响已经保存的图片和文章。

## UI 响应式修复：修复缩小窗口后的布局

**状态：已完成**

完成内容：移除 1279px 断点下的强制图标侧栏，侧栏折叠改为只由用户按钮控制；展开侧栏调整为 232px，手动折叠调整为 84px，并保留明确的展开入口；品牌图标、导航项和顶部栏增加了更稳定的尺寸；顶部标题、搜索框、资料库状态区支持在窄窗口下收缩，快捷菜单和提示消息不会超出窗口；页面标题允许换行，CSS 视口不再因为 `min-width` 被硬裁剪。

验证结果：

- `npm run build` 通过，包含 Vue 类型检查和生产打包。
- `cargo check --manifest-path src-tauri/Cargo.toml` 通过（本次仅修改前端样式，Rust 行为未改动）。
- `git diff --check` 通过，仅有工作区既有换行符提示。
- 使用外部 Chrome 检查 1440、1280、1100 CSS 像素：页面横向滚动宽度等于视口宽度，侧栏分别保持 232px 展开；1100 CSS 像素时顶部标题、搜索和内容没有重叠或裁剪。
- 手动点击折叠按钮后，侧栏为 84px，底部展开按钮可见。

Git Summary：`fix: stabilize responsive desktop layout`

Git Description：大白话说，窗口缩小或系统放大后，拾录不会再自动把左侧导航压成一条窄窄的图标栏；侧栏、左上角品牌区和顶部搜索区都有足够空间，想折叠时可以自己点按钮，之后也能正常展开。本次只修复响应式布局，没有提前改成新的个性化视觉风格。

## 弧形滚轮导航与资料库自动迁移

**状态：已完成**

完成内容：侧边导航改为带中央基准线的弧形滚轮选择器。总览、资料库、新建资料和设置沿弧线排列，滚轮和拖动只改变中心高亮，点击或按 Enter/Space 才切换页面；支持方向键、Home/End、焦点管理和减少动态效果时的普通列表降级。设置页支持选择新资料库文件夹，迁移时先复制全部 Markdown、图片、OCR 原文、版本和索引，验证新目录结构完整后才更新配置并清理旧目录；目标文件夹非空、路径嵌套或复制失败时会拒绝迁移并保留旧资料。

验证结果：

- `npm run build` 通过，包含 Vue 类型检查和生产打包。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 通过。
- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，10 项测试全部成功，新增资料库目录复制回归测试。
- `git diff --check` 通过，仅有工作区既有换行符提示。
- 使用外部 Chrome 检查 `1440 / 1280 / 1100 / 920` CSS 像素：页面无横向溢出；1100 像素时四个弧形导航项完整显示。
- 外部 Chrome 行为测试确认：滚轮不改变路由；点击非中心项会先吸附再进入页面；方向键后按 Enter 可以导航。

Git Summary：`feat: add arc navigation and library migration`

Git Description：大白话说，左边导航现在像一个弧形滚轮，滚动时只是在挑选项目，点一下或按回车才真正进入；在设置里更换资料库时，软件会把旧文件夹里的 Markdown、图片和识别内容完整搬到新文件夹，确认搬好后才清理旧位置，避免资料丢失。

## 弧形滚轮导航重做：连续曲线与边界交互

**状态：已完成**

完成内容：按参考图重做侧边导航结构。导航项现在由连续的浮点滚轮位置驱动，沿同一段垂直圆弧排列；中心项靠近右侧短基准刻度，远端项目向左退入弧线，并随距离旋转、缩小、淡出和模糊。滚轮和拖动只改变候选位置，停止后吸附到最近项目；只有吸附到基准线才高亮，点击或 Enter/Space 才进入页面。首尾位置有边界，不会循环跳转；折叠侧栏和减少动态效果时退化为普通图标/列表导航。展开侧栏宽度调整为 252px，为弧线和中文标签留出稳定空间；导航区域会填充品牌区与底部状态区之间的剩余高度，基准线随窗口高度保持在中段。

交互补充：路由变化会取消过期的延迟跳转；拖动后的合成点击会被抑制；方向键、Home/End 只改变候选项并移动焦点，键盘确认后才导航；所有模式保持单一 roving tabindex。

验证结果：

- `npm run build` 通过，包含 Vue 类型检查和 Vite 生产打包。
- `cargo check --manifest-path src-tauri/Cargo.toml` 通过。
- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，10 项测试全部成功。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 通过。
- `git diff --check` 通过。
- 外部 Chrome 截图检查 1440×900、1280×820、1100×820 和 1100×620：导航弧线、短基准线、远端淡出和窄窗口布局均正常；页面无横向溢出。开发环境下资料库读取错误来自未启动的 Tauri API，不属于本次导航改动。

Git Summary：`refactor: rebuild arc wheel navigation interaction`

Git Description：大白话说，左边导航这次真正改成了“弧形滚轮”。滚动和拖动时只是挑选位置，项目会沿一条弧线移动，离中心越远越小、越淡、越模糊；停到基准线才亮起来，点一下或按回车才打开页面。窗口缩小、侧栏折叠、键盘操作和减少动效时也都有对应的正常状态。
