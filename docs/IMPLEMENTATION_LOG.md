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

## 2026-09-11 增补 1：滚轮自动切页与鼠标直接点击

**状态：已完成，导航回归通过。**

本节按最新需求替代上文“滚轮只选择、Enter 才跳转”的规则，弧线、配色和布局保持不变。

- 滚轮停稳约 130ms 后自动吸附并进入选中页面，无需按 Enter；合并同一手势的连续事件，防止触控板抖动造成连续切页。
- 鼠标点击任意导航项立即切页；仅在确实拖动时捕获指针，修复普通点击被导航容器吞掉的问题。
- 点击、键盘选择或外部路由变化会取消待执行的滚轮切页，避免旧手势覆盖新操作；Ctrl+滚轮不触发导航。
- 保留原有拖动只选择、键盘确认、首尾边界和折叠模式；不增加其他操作模式。
- 新增 `scripts/check-navigation.mjs`，使用隔离的本机 Chrome 和真实鼠标按下/松开事件测试，不使用 Codex 内置浏览器。

验证：`npm run build` 通过；`node scripts/check-navigation.mjs` 连续两次 10/10 通过，覆盖滚轮、鼠标、竞态、拖动、键盘、折叠和减少动态效果；1440×900、1100×680 截图和几何检查无横向溢出或顶部/侧栏重叠。浏览器缺少 Tauri 桥接的资料库提示不计为桌面数据功能验收。

Git Summary：`fix: navigate directly from sidebar wheel and clicks`

Git Description：`滚轮吸附后自动切页，鼠标点击立即进入；修复指针捕获导致的点击失效，取消过期导航任务，并加入真实鼠标输入和窗口尺寸回归测试。`

大白话：现在左侧滚到哪一项就进哪一页，不用再按回车；也可以直接拿鼠标点。

## 2026-09-11 增补 2：关闭窗口隐藏到系统托盘

**状态：代码、构建和安装完成；托盘恢复/退出的桌面点击验收待确认。**

- 启用 Tauri 原生托盘，复用现有应用图标，不修改视觉风格。
- 关闭主窗口时隐藏到系统托盘，保留页面和进程；左键托盘图标恢复窗口，右键提供“打开拾录”和“退出”。
- 恢复时同时取消最小化并聚焦；“退出”彻底结束应用，不再次隐藏到托盘。
- 仅在托盘存在且隐藏成功时阻止关闭；托盘初始化失败则保持普通关闭行为，避免程序隐藏后无法找回。不拦截应用级系统退出。

验证：`cargo test --manifest-path src-tauri/Cargo.toml` 10/10 通过；Rust 格式检查与 `git diff --check` 通过；`npm run tauri:build` 成功生成 Windows x64 NSIS 安装包，安装器退出码为 0。实际从 `work/installed-0.1.0/shilu.exe` 启动，观察到“拾录 ShiLu”主窗口、响应正常的进程。桌面工具启动监听失败后改由命令行启动；之后工具报告人工输入，窗口隐藏但进程保留，无法把这一变化归因于自动点击。工具未返回可操作的托盘入口，因此未宣称左键恢复、右键打开/退出通过，不影响用户手动验收。

安装包：`src-tauri/target/release/bundle/nsis/拾录 ShiLu_0.1.0_x64-setup.exe`（未签名测试版；缺少 WebView2 的电脑首次安装可能需要联网）。未更改或迁移现有资料库，也未操作卸载。

Git Summary：`feat: keep ShiLu in the system tray on window close`

Git Description：`关闭主窗口后隐藏到系统托盘，增加打开与退出菜单，恢复时取消最小化并聚焦；托盘不可用时保留普通关闭行为。已重新打包和安装，桌面托盘点击验收限制已记录。`

大白话：右上角叉号现在是“收起来”，不是关掉软件；从托盘可以找回来，要彻底关闭用托盘里的“退出”。

## 2026-09-11 增补 3：记住上次的窗口大小和显示模式

**状态：已完成，原生跨进程重启检查 11/11 通过，最终 Windows x64 安装包已生成。**

- 用独立 Tauri 窗口状态模块记住主窗口普通宽高、最大化和全屏状态，原子替换状态文件。
- 启动时先恢复状态再显示窗口；没有历史配置或配置无法解析时使用原来的默认尺寸。
- 关闭到托盘前、托盘“退出”前显式保存，应用退出时也保存。
- 不恢复隐藏、最小化、位置或窗口装饰，避免下次启动看不到窗口；未新增设置页选项。
- 窗口状态写入应用配置目录内的 `.window-state.json`，与资料库配置和 Markdown 文件分开，不影响资料库路径和迁移。
- 新增独立 Tauri API 跨进程测试，使用专用测试标识隔离窗口状态，不覆盖 `com.shilu.desktop` 用户配置；该测试不等同于托盘鼠标点击验收。

实现调整：先试用官方 `tauri-plugin-window-state`，真实重启测试发现其会把全屏尺寸写成普通尺寸，并在最大化后最小化退出时丢失最大化状态。因此移除该依赖，分别保存普通宽高与最后非最小化模式。没有手改第三方依赖，也没有添加界面设置。

验证：前端构建通过，Rust 14 项单元测试通过；`./scripts/check-window-state.ps1` 的真实 Tauri API 重启测试 11/11 通过，包含普通尺寸保存/还原、最大化及还原到正常尺寸、真全屏及取消全屏后原尺寸、隐藏后重启可见、最大化后最小化退出不丢模式、损坏配置回退。测试通过独立子进程 JSON 报告判定，避免 Tauri 退出码掩盖失败。测试程序起初缺少 Windows common-controls v6 声明，脚本现使用 SDK `mt.exe` 仅为测试产物补齐，不修改正式程序资源。退出期间出现 WebView2 类注销 Error 1412 日志，但所有窗口状态断言及独立报告通过，测试进程已结束。

最终构建：`npm run tauri:build`、`cargo check --manifest-path src-tauri/Cargo.toml --examples`、Rust 格式和 Git 差异检查通过。安装包为 `src-tauri/target/release/bundle/nsis/拾录 ShiLu_0.1.0_x64-setup.exe`，2375078 字节，生成时间 2026-09-11 16:18。本次未覆盖安装现有用户程序；原生 API 重启测试不代替托盘菜单的实际鼠标点击验收。测试父进程进一步要求成功报告和成功退出码同时满足，该加固已通过编译检查。

Git Summary：`feat: remember window size and display mode across launches`

Git Description：`保存并恢复主窗口宽高、最大化和全屏状态，在关闭到托盘和退出前保存；启动时先恢复再显示，不记住隐藏状态，增加隔离的原生重启回归测试。`

大白话：上次窗口多大，下次打开就多大；上次是最大化或全屏，下次也按那个状态打开，不用每次再调。

## 2026-09-11 增补 4：恢复窗口尺寸后重新居中

**状态：已完成，原生重启测试 11/11 通过，Windows x64 安装包已重新生成。**

原因：配置中的 `center: true` 按默认 1440×900 尺寸居中；后续恢复上次宽高时没有重新居中，导致较小窗口保留偏上的位置。

修改：恢复普通窗口宽高后立即调用 Tauri 原生 `center()`，再恢复最大化或全屏，最后显示窗口；首次启动及配置损坏回退时也居中。居中使用当前显示器可用工作区，排除任务栏。只改变启动定位，不增加位置记忆，不修改托盘恢复、配色或资料库。

验证：Rust 14 项单元测试、原生窗口重启 11 个阶段全部通过。新增普通重启和隐藏后重启的实际坐标检查，并保留原有尺寸、最大化、全屏及最小化回归。两次测量均为实际位置 `(209, 85)`，窗口外框 `1822×1166`，当前工作区 `2240×1328`；按外框计算中心位置为 `(209, 81)`，纵向 4 个物理像素差异来自 Windows 可见边框与不可见缩放边框的区别，处于测试允许的 16 像素范围内。测试期间不修改用户窗口配置或资料库。

`npm run tauri:build`、Rust 格式检查及 `git diff --check` 通过。最终安装包：`src-tauri/target/release/bundle/nsis/拾录 ShiLu_0.1.0_x64-setup.exe`，生成时间 2026-09-11 16:49；未替用户覆盖安装或提交 Git。

Git Summary：`fix: center the window after restoring its saved size`

Git Description：`启动时先恢复窗口宽高再居中，保留最大化和全屏模式；补充普通启动及隐藏后重启的中心坐标回归检查。`

大白话：窗口大小照旧记住，但每次打开普通窗口都会放到屏幕中间，不再偏到顶部。

## 2026-09-11 增补 5：折叠侧栏改为紧凑图标导航

**状态：已完成，外部 Chrome 回归 20/20 通过，Windows x64 安装包已重新生成。**

原因：折叠导航继承了展开态的高度拉伸，网格把四项分散到整条侧栏；透明隐藏的导航文字和品牌文字仍占据布局空间，造成图标偏左和顶部额外空白。

修改：仅在折叠态取消导航拉伸，按固定 44×44 按钮、8px 间隔纵向排列，距品牌行 24px；图标和高亮居中，隐藏文字使用 `display: none` 去掉占位，导航按钮补充可访问名称。底部展开按钮仍固定在底部，展开态弧形规则、颜色、滚轮直接切页和点击切页行为均保留。按 impeccable 的布局检查规则固定尺寸和内容分组，不扩展页面设计范围。

验证：`node scripts/check-navigation.mjs` 20/20 通过，覆盖四入口实际鼠标点击、滚轮切页、连续折叠展开 3 次、减少动态效果模式、原有展开导航回归。折叠态分别检查 1440×900、1100×680、1920×1080，以及 1100×680 的 1.5 倍像素密度，深浅主题共 8 组截图；按钮均为 44×44，间隔均为 8px，导航组高度 200px，品牌行高度 44px，隐藏文字不占位，图标中心对齐。已查看浅色 1.5 倍、深色和反复折叠后展开的截图。

验收边界：使用隔离外部 Chrome，不打开 Codex 内置浏览器，不访问用户浏览器配置或资料库。浏览器中缺少 Tauri 后端时出现的资料读取/主题保存提示不属于本轮原生验收。1.5 倍检查是浏览器像素密度模拟，不等同于切换 Windows 系统缩放；本轮未实际覆盖安装或进行原生桌面点击验收。

`npm run tauri:build`（含类型检查、前端构建与 NSIS 打包）及 `git diff --check` 通过。安装包：`src-tauri/target/release/bundle/nsis/拾录 ShiLu_0.1.0_x64-setup.exe`。验收截图：`work/navigation-collapsed-1100x680-1.5x-light.png`、`work/navigation-collapsed-1440x900-1x-dark.png`。

Git Summary：`fix: compact collapsed sidebar navigation`

Git Description：`折叠侧栏改为居中的 44×44 图标按钮与 8px 固定间距，移除隐藏文字占位，保留展开弧形和直接切页交互，补充多尺寸及深浅主题回归测试。`

大白话：折叠后四个图标紧凑地排在 Logo 下方，不再随着窗口变大而被撑散，也不再偏左。

## 2026-09-11 增补 6：统一 Windows 任务栏与桌面快捷方式图标

**状态：本机快捷方式修复完成，用户确认重启后任务栏已显示图 8；新版安装包构建完成。**

排查：正式安装目录 `D:\CommonSoftware\拾录 ShiLu` 内的 EXE 和构建产物均已包含图 8 的 9 种尺寸图标，逐项资源字节与源 ICO 相同。但开始菜单快捷方式仍指向旧测试目录 `work/installed-0.1.0/shilu.exe`，桌面快捷方式也未显式指定图标。未将旧图标现象简单归因于未更新 EXE。

本机修复：备份开始菜单原快捷方式后，将目标修正为正式安装目录，复制独立 `shilu-icon-08.ico` 并显式设置图标位置，保留 AppUserModelID。操作期间用户移除了桌面快捷方式，经单独确认后重新创建，同样使用图 8。通过 Shell 更新通知刷新变更项目，没有删除全局图标缓存、重启资源管理器或强制结束 ShiLu。用户从托盘退出再从新快捷方式启动后，确认“任务栏已显示图 8”。原快捷方式备份保存在 `work/shortcut-backup-20260911-191807-429`。

安装包修复：通过资源映射附带独立 ICO，安装后仅对目标与当前安装 EXE 完全匹配的现有快捷方式更新图标；使用 `IShellLink::SetIconLocation` 和 `IPersistFile::Save` 保留链接原有属性，不重建缺失链接，不改变用户未选择创建快捷方式的行为。交互式安装完成页才新建的桌面链接继续使用 EXE 内嵌图 8；已有链接及安装阶段生成的链接使用独立 ICO。卸载流程自动清理该 ICO。

维护脚本：`scripts/repair-windows-shortcuts.ps1` 只接受指定安装的完整目标路径，迁移旧目标必须显式传入 `-PreviousExecutable`，且须匹配 ShiLu 的 AppUserModelID。修改前备份，再验证目标、图标和身份。不会将其他有效安装的快捷方式重定向过来。

验证：`npm run tauri:build`、PowerShell 语法检查与 `git diff --check` 通过。独立 NSIS 测试器在 `work/nsis-shortcut-regression-20260911` 下运行，11/11 通过：本安装链接更新图标，目标/参数/工作目录/说明/热键/窗口状态/AppUserModelID 均保留，其他安装链接字节不变，缺失链接不创建，宏寄存器完整恢复。未覆盖安装本机程序；本机快捷方式修复已生效，新安装器行为在隔离测试目录验证。

Git Summary：`fix: unify Windows shortcut icons with concept 08`

Git Description：`安装包附带独立图 8 图标并安全刷新本安装的现有快捷方式，增加有备份和精确目标限制的修复脚本；修正本机旧开始菜单链接、重建桌面快捷方式，用户确认任务栏图标已更新。`

大白话：桌面入口已换成图 8，任务栏重开后也正常了；以后安装更新会更明确地使用同一张图标。

## 2026-09-11 增补 7：截图直接粘贴、保存和 OCR

**状态：功能及回归验证完成。**

新建资料和编辑资料均增加“粘贴图片”按钮及 Ctrl + V 图片粘贴。截图先保留在内存中预览，用户确认后才存入当前资料库中该条资料的 `images/`。新建页允许文件与截图混合排序；编辑页点击“保存图片”追加，随后手动 OCR，识别结果仍需点击“插入正文”。普通文字粘贴照常；不轮询剪贴板，不自动调用大模型。

处理同名截图、按触发顺序加入的异步解码、保存期间队列锁定、离开页面后的迟到结果、追加失败保留预览。编辑页导入不重新读取文章，避免覆盖正在编辑的正文。新图片统一独占创建、按序编号，整批校验失败不落盘，写入失败撤回本批，新建失败清理本次空资料。限制为单张 20 MiB、4000 万像素、单边 20000 像素，每批 50 张/200 MiB。

真实 OCR 回归发现原有 `FileRandomAccessStream` 无法打开 Rust 规范化后带 `\\?\` 前缀的路径（0x800700A1）。现仅在 WinRT 接口边界转成普通盘符/UNC 格式，磁盘写入及目录安全检查继续使用规范路径。

验证：24 项 Rust 常规测试通过；`scripts/check-clipboard.mjs` 的外部 Chrome 检查 6/6 通过，覆盖同名截图、多来源排序、失败重试、普通文字不拦截、异步返回、正文保留及手动插入 OCR。查看了 1100×680 浅色和 1440×900 深色截图，新增按钮及图片队列无横向溢出；impeccable 用于检查原风格下的小窗口布局与状态提示。

另以生成的文字 PNG 经新剪贴板来源接口写入隔离资料库，再调用真实 Windows OCR：成功读取文字、生成 `raw/ocr.txt`，原 PNG 字节与 Markdown 保持不变。可通过 `scripts/check-clipboard-ocr.ps1` 重跑。中文系统 OCR 对部分英文字符有误识别，因此以稳定词 SCREENSHOT / OCR TEST 验证，不要求全文逐字一致。

验收边界：Chrome 中的 IPC 和粘贴事件使用模拟数据；原生 OCR 使用真实接口和隔离目录。自动化未读取或改写用户当前剪贴板，也未代替安装版截图 Ctrl + V 的人工验收。

Git Summary：`feat: paste screenshots into capture and article editor`

Git Description：`新增截图粘贴、预览和确认保存，支持文件与截图混合排序及编辑页追加；完善图片校验、失败回滚和草稿保留，修复 Windows OCR 读取规范路径失败，补充界面与真实 OCR 回归。`

大白话：截图不用先存成文件了，直接粘贴到拾录，确认保存后就能识别；连续截图不会覆盖旧图，追加时也不会清掉正在写的正文。

## 2026-09-11 增补 8：重复启动唤回原窗口

**状态：功能完成，隔离原生测试 7/7 通过。**

在其余插件之前注册 `tauri-plugin-single-instance`，按 `com.shilu.desktop` 识别同一应用。第二次启动复用托盘的显示/取消最小化/聚焦逻辑，随后结束第二个进程，不导航、不重建页面或托盘。

`scripts/check-single-instance.ps1` 覆盖普通显示、关闭到托盘、最小化、最大化后隐藏、最大化后最小化、全屏隐藏，以及同一 EXE 复制到其他路径后启动。7 次回调仅有 1 次应用 setup 和 1 次托盘创建；每次第二进程退出，原窗口恢复，真实 WebView 的文档标记、内存草稿、输入框内容、光标位置和路由均不变。

测试使用独立应用标识与 WebView 数据目录，没有操作已安装拾录或用户资料。报告在 `C:/Users/13528/AppData/Local/Temp/shilu-single-instance-check-11460.1789138527187/report.json`。这是原生进程与窗口 API 回归，不声称完成托盘菜单鼠标点击验收。

Git Summary：`fix: restore the existing window on repeated launches`

Git Description：`加入应用单实例检查，重复启动时恢复原有窗口并退出第二进程；复用托盘恢复逻辑，增加七种原生场景回归，验证未保存页面和窗口模式保留。`

大白话：拾录已经开着时，再点快捷方式只会找回原窗口，不会再多开一个托盘图标，也不会清空当前页面。

## 2026-09-12 增补 9：模型配置设置页

**状态：前端设置页完成，构建检查通过。**

设置页新增 OCR 识别模型和文案润色模型两张独立配置卡，可分别填写 API 地址、API Key 和模型名称，启用状态互不影响。每张卡支持获取 OpenAI 兼容接口的模型列表，并通过下拉框选择具体模型；接口不支持列表时仍可手动填写。测试模型按钮使用当前填写并选中的模型，测试过程显示加载状态和可读错误，不写入资料库。

同时修正本地资料库路径的展示：界面自动隐藏 Windows 内部 `\\?\\` 前缀，实际磁盘路径和迁移逻辑不变。模型配置服务已与 `save_model_settings`、`fetch_model_list`、`test_model` 命令对齐。

验证：`npm run build` 和 `cargo check --manifest-path src-tauri/Cargo.toml` 通过，`git diff --check` 无新增空白错误。

Git Summary：`feat: add OCR and polish model settings`
Git Description：`新增双模型配置、模型列表下拉选择、独立测试与保存状态反馈，修正资料库路径显示前缀。`
大白话：现在可以在设置里分别配置识图模型和润色模型，拉取模型后选一个保存，还能马上点测试看这个模型到底能不能用。

补充实现：OCR 识别已改为读取当前资料图片并调用已保存的 OCR 模型；未配置或调用失败时不再回退本地 OCR，而是显示明确错误。新增文案润色按钮，使用已保存的润色模型生成独立草稿，用户确认后才插入正文。两条模型配置均以当前下拉框选中模型为准。

最终验证：`cargo fmt -- --check`、`cargo test --manifest-path src-tauri/Cargo.toml --lib`（24 通过，1 个需 OCR fixture 的测试忽略）、`npm run build`、`git diff --check` 通过；外部 Chrome 1100×680 设置页截图已检查。`npm run tauri:build` 生成 Windows x64 NSIS 安装包：`src-tauri/target/release/bundle/nsis/拾录 ShiLu_0.1.0_x64-setup.exe`，3,833,741 字节，SHA256 `1B16C717373D2728369581E18E64FDAA3F4E56CCFCB29C3BF1A6AF0F8B766D63`。未使用真实 API Key 请求外部服务，未覆盖安装现有程序。

Git Summary：`feat: configure and test OCR and polish models`
Git Description：`增加双模型 OpenAI 兼容配置、模型列表下拉选择和按当前模型测试；OCR 与润色均走外部接口，支持独立保存、错误反馈与人工确认插入，修复资料库路径展示。`
大白话：设置里现在能分别选识图模型和润色模型，点“测试模型”就能知道当前选中的到底能不能用；识别失败不会偷偷换成本地 OCR。

本轮交付：`npm run tauri:build`（包含前端类型检查和构建）、Rust 格式检查、PowerShell/Node 脚本语法检查及 `git diff --check` 通过。Windows x64 NSIS 安装包于 2026-09-11 23:02 生成：`src-tauri/target/release/bundle/nsis/拾录 ShiLu_0.1.0_x64-setup.exe`，2,897,965 字节，SHA256 `3719495CCEF5D2FC08B66B0D7EC2F6CE0B1739BF4A0B9D90820A79E7587FD010`。已停止本轮临时 1421 端口测试服务；未覆盖安装用户的拾录，未提交 Git。升级前请从托盘退出旧版本，再运行安装包；旧进程本身不具备新增的单实例检查。

## 2026-09-12 增补 10：草稿与归档的文件存储

状态：实现与定向验证完成。

文章 Markdown 元信息新增 status、capture_step、source_images。旧文章默认正式资料；草稿与归档均保留原文章文件夹，归档恢复只更新元信息，不移动或删除正文与图片。新增草稿创建、状态更新和按状态查询命令；正文保存保留未传入的生命周期字段。修复原解析器遇到正文二级标题时截断内容的问题。源截图记录保持顺序，正文配图不自动加入 OCR 输入；图片路径拒绝越界和重复。

验证：storage 定向测试 21 项通过，1 项旧 Windows OCR 测试忽略。覆盖草稿提交、归档恢复、原始 Markdown 与自定义元信息保留、旧资料读取、图片顺序、追加隔离和失败回滚。所有写入均为临时测试资料库。

Git Summary：`支持草稿保存及资料归档恢复`

Git Description：`为文章记录草稿、正式与归档状态，支持分类查询及可恢复归档；保留图片与正文，修复 Markdown 小节截断并补齐存储回归。`

大白话：没整理完的放草稿箱，不常用的放归档箱；文章和图片还在原处，随时能继续整理或恢复。

## 2026-09-12 增补 11：连续收集与 Markdown 编辑流程

状态：实现与外部 Chrome 流程验收完成。

新建页分为截图收集与文章信息两步，点击下一步先存草稿再 OCR；识别后的文字直接进入第三步正文编辑。保存失败留在当前页，识别成功但写盘失败时可重试保存已获得的文字，无需再次请求 OCR。草稿恢复回到原步骤；页面切换前暂存，失败会阻止离开。编辑页统一维护最新正文，自动保存串行执行，保存过程中继续输入不会被旧结果误标成已保存。

引入 markdown-it 与 DOMPurify，支持 Markdown 编辑、对照及预览，渲染标题、列表、表格、代码、链接和文章内的图片。配图粘贴在光标处，存入当前文章 images/ 并插入相对引用。润色针对最新正文，可编辑与预览结果，确认采用才替换；正文已变化时不能直接采用旧结果，缺失配图引用时提示补回。新增图片显示所需的 Tauri asset protocol，静态范围为空，按资料库访问动态允许文章目录；Windows 规范路径仅在显示边界调整。

侧栏新增草稿箱、归档箱，六个入口保留滚轮直接切页及折叠紧凑模式。总览计数和本地资料库状态改为实际读取。草稿、正式资料和归档共用列表展示与搜索，正式资料可以归档，归档可以恢复。按 impeccable 检查层级、错误状态、小窗口与禁用状态，保留原有配色；根据截图将小窗口的新建页保存按钮保持在标题同一行，并明确禁用按钮外观。

验证：`scripts/check-article-workflow.mjs` 10/10；`scripts/check-capture-flow.mjs` 9/9。覆盖截图两步返回、重复保存不重建、导入增删排序、源图恢复、OCR 失败重试、识别成功写盘失败重试、未配模型暂存、普通编辑与配图、润色新旧正文保护、发布与归档恢复、失败阻止离开，以及 1440×900/1100×680 展开与折叠导航。已查看截图；窄窗对照视图上下排列，无横向溢出。

Git Summary：`接通分步收集与正文编辑并增加草稿箱和归档箱`

Git Description：`OCR 结果直接进入统一 Markdown 编辑器，支持光标配图和最新正文润色；接通草稿恢复、保存保护、归档恢复与六入口导航，补充完整流程和失败场景回归。`

大白话：截图、补标题链接、识别、改文字、加配图可以连续完成；中途离开先存草稿，完成后放资料库，不常用时可归档。

## 2026-09-12 增补 12：独立模型测试与操作反馈

状态：模型接口和设置页验证完成。

修复获取模型列表复用完整模型校验的问题，列表现在只要求地址和 Key。OCR/润色分别维护列表、测试与保存的进度和结果，可同时操作。所有请求使用点击时的表单快照，旧列表不覆盖新连接，旧测试结果标明对应配置；单独保存一项配置，不覆盖另一项未保存的输入。设置写入共用锁，保留资料库和主题配置。

OCR 测试使用固定文字 PNG 并检查识别内容，不能只凭通用“连接成功”判为通过。实际 OCR 只读取 source_images，并依据字节识别 PNG/JPEG/WebP MIME。润色提示要求保留事实、Markdown 图片和链接。模型成功展示对应名称及返回文本，空列表、读取失败、HTTP 错误等有明确提示与重试入口。

验证：8 项 Rust 本地 HTTP 模型测试通过；外部 Chrome `scripts/check-model-settings.mjs` 9/9，覆盖空模型获取列表、下拉选择、OCR/润色并发、同卡列表/测试并发、保存互不影响、旧配置结果、失败重试、窄窗与深浅主题。已查看设置页截图。

Git Summary：`修复模型列表获取并支持独立并发测试`

Git Description：`分离双模型的列表、测试和保存状态，按当前选择测试并显示实际结果；完善迟到响应、空列表与错误反馈，单独保存配置并补充接口和界面回归。`

大白话：先获取列表再选模型即可；两个模型可以同时测试，结果各管各的，保存一个也不会把另一个改掉。

验收边界：前端验收使用独立外部 Chrome、生成图片和模拟 IPC；后端测试使用本地 HTTP 模拟服务与临时资料库。没有使用真实 API Key，没有读取用户剪贴板或修改用户资料，没有操作或覆盖安装现有拾录。Tauri 原生图片显示、实际截图粘贴及真实模型服务仍需安装版实际使用确认，不能用 Chrome 模拟结果代替。

## 2026-09-13 增补 13：移除用户可见草稿箱

状态：实现与前端构建验证完成。

按当前使用习惯移除侧边栏、首页统计和资料列表中的草稿箱入口；旧 `/drafts` 地址保留兼容跳转到资料库，避免历史链接失效。新建和编辑流程仍以内部 `draft` 状态保存处理中内容，用于 OCR 失败重试、写盘失败保护和页面离开保护，但用户界面统一显示为“当前进度”或“处理中”，不再暴露草稿箱概念。资料库只展示正式资料，归档箱继续独立保留。

验证：`npm run build`（`vue-tsc --noEmit` 与 Vite 生产构建）通过；`rg` 检查确认可见导航、首页统计及列表没有草稿箱入口，内部状态字段仍保留。

Git Summary：`移除草稿箱入口并保留处理中状态`

Git Description：`删除侧边栏与首页的草稿箱入口，旧地址兼容跳转资料库；新建和编辑流程继续使用内部临时状态保障 OCR 与保存过程，界面改用当前进度和处理中提示。`

大白话：你不会再看到草稿箱，但软件在识别或保存中途仍会暂时保留内容，出错时可以继续，不会因为去掉入口而丢数据。

补充核对：修正无标题草稿恢复时默认名称不一致的问题，将“未命名资料”与旧占位名称还原为空输入，保持 OCR 前补充标题的校验。截图流程测试同步模拟后端默认标题行为，重新运行 9/9 通过，确认无标题草稿保留截图且不会直接发起 OCR。

## 2026-09-13 最终打包验收

状态：完成。

针对上一轮中断后的工作重新执行最终构建，确认前端产物、Rust 原生程序和 Windows x64 NSIS 安装包均由当前工作区生成。最终安装包为 [`src-tauri/target/release/bundle/nsis/拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/拾录%20ShiLu_0.1.0_x64-setup.exe)，大小 3,929,756 字节，生成时间 2026-09-12 23:55:17，SHA256：`BD2BAC2341AEDE8CABB842EEFB7B7A1A37F41BF883DA5FDFED4FADE42E7AF037`。

最终检查：`cargo test --manifest-path src-tauri/Cargo.toml --lib` 为 37 项通过、0 失败、1 项因需要 Windows OCR fixture 忽略；`cargo fmt -- --check`、`npm run build`、`git diff --check` 通过。此前三组外部 Chrome 隔离回归共 28 项通过（模型设置 9/9、文章流程 10/10、截图流程 9/9）。未使用真实 API Key，没有覆盖安装现有程序，也没有提交 Git。

Git Summary：`完善资料整理流程、草稿归档与模型测试`

Git Description：`接通截图收集、信息补充和 Markdown 编辑，支持草稿恢复、正文配图与归档恢复；修复模型列表获取及双模型并发测试，补齐操作反馈、无标题草稿校验和 Windows 安装包验收。`

大白话：这版安装包已经重新打好，截图到 OCR、编辑、配图、草稿、归档和模型设置都做过自动检查；无标题草稿不会偷偷跳过标题填写。明天可以直接安装这个包验收。

## 2026-09-13 增补 14：文章阅读卡片、干净正文与配图入口

状态：实现进行中，待最终打包验收。

资料库和首页最近资料现在进入只读文章阅读页，不再直接打开编辑器。阅读页渲染标题、来源链接、正文和配图；只有点击“编辑”才进入 Markdown 编辑器。编辑器预览同步显示整篇资料，而不是只显示正文片段。内部 front matter 仍用于搜索、状态和兼容旧资料，但不会出现在阅读界面。

OCR 结果进入正文前会清理代码围栏、合成图片标题、重复引用符号和“正文/我的备注”包装；首个明确短标题可自动填入标题字段，并从正文移除重复标题。源截图在编辑页单独展示，可主动插入正文；普通粘贴/选择配图仍保存到当前文章 images/，且不参与 OCR。

按需求移除用户可见草稿箱：侧边栏、首页统计、资料库草稿列表和相关操作文案已删除，旧 `/drafts` 地址仅兼容跳转资料库。内部临时状态保留用于 OCR 失败重试和写盘保护，不在界面展示。

Git Summary：`新增文章阅读卡片并移除用户可见草稿箱`

Git Description：`将首页与资料库入口改为只读文章阅读页，隐藏 Markdown 内部字段并统一整篇预览；清理 OCR 正文、支持源截图主动插入正文，同时移除草稿箱用户界面。`

大白话：以后点资料先看到排版好的文章，想改再点编辑；OCR 截图只在编辑时查看，只有主动插进正文的图片才会在阅读页出现；草稿箱从界面消失，不会再让你管理它。

最终打包验收：`npm run tauri:build` 成功生成 Windows x64 NSIS 安装包 [`拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/拾录%20ShiLu_0.1.0_x64-setup.exe)，大小 3,939,035 字节，生成时间 2026-09-13 11:14:38，SHA256：`53D52202997BE9DF30A256AF00D8085B110CED4F0241197CAF284D412D6EE0D8`。`cargo test --manifest-path src-tauri/Cargo.toml --lib` 为 38 项通过、0 失败、1 项跳过；`npm run build`、`cargo fmt -- --check`、`git diff --check` 均通过。模型设置界面回归 9/9 通过；旧截图流程脚本仍包含已删除草稿箱的旧断言，未将其作为本轮结果使用。

## 2026-09-13 增补 15：隔离 OCR 源图与正文配图

状态：实现、代码回归检查与 Windows 安装包验收完成。

修正文章阅读页的图片来源规则：不再把 `source_images` 自动当作封面或底部画廊，阅读页只渲染正文 Markdown 中实际存在的 `images/...` 引用。删除正文图片引用并保存后，阅读页不再显示该图片；磁盘中的 OCR 源图仍保留。来源链接已固定放在正文之后，没有链接时不显示空占位。

编辑页继续单独展示 OCR 源截图，保留“插入正文”操作；移除“设为封面”及编辑器预览自动封面参数，避免源截图在未主动插入时出现在文章内容中。普通粘贴或选择配图的现有流程不变。

验证：`npm run build`、`cargo test --manifest-path src-tauri/Cargo.toml --lib`（38 项通过、1 项跳过）、`cargo fmt -- --check` 和 `git diff --check` 通过；静态检查确认阅读页不存在 `sourceImages` 自动渲染、编辑器不存在 `previewCoverUrl` 参数。

Git Summary：`隔离 OCR 源图与正文配图显示`

Git Description：`阅读页仅显示 Markdown 正文引用的图片，移除 OCR 源图自动封面和画廊；来源链接移至文末，编辑页保留源截图查看与主动插入正文。`

大白话：OCR 截图只是给模型识字用的，平时看文章不会自动出现；你点“插入正文”后它才会显示，删掉正文里的图片链接后也会消失，原图仍然留在文章文件夹里。

最终打包验收：重新执行 `npm run tauri:build` 成功生成当前代码对应的 Windows x64 NSIS 安装包 [`拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/拾录%20ShiLu_0.1.0_x64-setup.exe)，大小 3,939,364 字节，生成时间 2026-09-13 12:23:04，SHA256：`E37E091B8EEDFABC826AD159BB3ADAC974C36FCC98076CF76C10EB06EEB12DEB`。

## 2026-09-13 增补 16：首页最近资料卡片化

状态：实现、回归检查与 Windows 安装包验收完成。

首页“最近保存的资料”由标题列表改为响应式卡片网格。每张卡片从正文 Markdown 中提取第一张合法的 `images/...` 配图作为封面；没有正文配图时显示统一的无图占位。OCR 源截图仍然不会被当作封面。整张卡片点击进入只读文章阅读页。

卡片增加悬停上浮、封面轻微缩放、操作箭头显现和键盘焦点反馈；标题限制为两行并在窄窗口下自动切换为双列或单列。为避免首页列表接口加载整篇正文，后端摘要新增 `firstContentImage` 字段，仅返回首张正文图片路径。

验证：`npm run build`、`cargo test --manifest-path src-tauri/Cargo.toml --lib`（39 项通过、1 项跳过）、`cargo fmt -- --check` 和 `git diff --check` 通过。

Git Summary：`首页最近资料改为带首图的卡片网格`

Git Description：`从正文 Markdown 提取首张配图用于首页最近资料卡片，保留无图占位并加入悬停与键盘交互反馈；OCR 源图不参与封面展示，适配窄窗口布局。`

大白话：首页最近保存的资料现在是一张张卡片，有正文配图就显示第一张，没有就显示占位；鼠标放上去会有反馈，点整张卡片就能查看文章。

最终打包验收：`npm run tauri:build` 成功生成当前代码对应的 Windows x64 NSIS 安装包 [`拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/拾录%20ShiLu_0.1.0_x64-setup.exe)，大小 3,949,188 字节，生成时间 2026-09-13 17:16:25，SHA256：`D6758213A6132661636C7E79621AEE5EE2939FF297423479B51972991C540D6C`。
## 2026-09-13 增补 17：简化资料字段与单模型整理

状态：实现与 Windows 安装包验收完成。

新建/编辑资料现在只面向用户显示标题、正文和来源链接；正文支持 Markdown，图片可通过粘贴、选择或拖拽加入，并写入每条资料独立文件夹的 `images` 目录。阅读页与首页/资料库均以文章卡片或渲染后的正文展示，Markdown front matter 不会暴露给普通查看；来源链接固定显示在正文末尾。旧资料的摘要、标签、备注和 OCR 源图字段仍可兼容读取，但新资料不再生成这些冗余字段。资料详情页新增永久删除，会同步删除本地文章文件夹及图片。

设置页收敛为一个“Markdown 整理模型”，支持 API 地址、Key、获取模型列表、下拉选择、当前配置测试和保存；获取、测试、保存互不取消，测试使用点击时的表单快照。保留后端旧 OCR/双模型字段用于兼容历史配置。

验证：`npm run build`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`git diff --check` 均通过；`cargo test --manifest-path src-tauri/Cargo.toml --lib` 为 41 项通过、1 项因需要 Windows OCR fixture 忽略。`npm run tauri:build` 成功生成 Windows x64 NSIS 安装包 [`拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/拾录%20ShiLu_0.1.0_x64-setup.exe)，大小 3,948,598 字节，生成时间 2026-09-13 22:50:02，SHA256：`F9E74E5B09465D6588E16191ACDB56A37337E6D19A4CE35DB8FF0FB044F448DB`。

Git Summary：`简化资料字段并接入 Markdown 整理模型`

Git Description：`新建与编辑聚焦标题正文链接，支持独立资料配图、渲染阅读与永久删除；移除用户可见旧元数据，设置页改为单一 Markdown 整理模型并保留历史配置兼容。`

大白话：以后新增资料只填标题、正文、链接，图片直接粘贴就会跟着这篇资料保存；平时点进去看到的是排版好的文章，想改才进入编辑。设置里只管一个负责整理 Markdown 的模型，能获取模型列表、测试后再保存。永久删除会把本地文件夹一起删掉。

## 2026-09-13 增补 18：最终文案同步与安装包重打

状态：完成。

同步产品文档与界面提示，使软件口径统一为“标题、正文、来源链接、正文配图”和“Markdown 整理模型”。移除残留的“截图或手动内容”和“未进行 OCR”等旧提示，不改变资料保存结构或历史资料兼容逻辑。

验证：`npm run build`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`cargo test --manifest-path src-tauri/Cargo.toml --lib`（41 项通过、1 项因需要 Windows OCR fixture 忽略）和 `git diff --check` 均通过。重新执行 `npm run tauri:build` 成功生成 Windows x64 NSIS 安装包 [`拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/拾录%20ShiLu_0.1.0_x64-setup.exe)，大小 3,943,798 字节，生成时间 2026-09-13 23:04:44，SHA256：`D6CC994F667383CF136F47733DD450A5DE09107185EF98D62022357B15F92254`。

Git Summary：`同步最终资料流程文案并重打 Windows 安装包`

Git Description：`将产品文档和界面提示统一为标题正文链接与正文配图流程，保留 Markdown 整理模型及历史资料兼容；完成前端、Rust 测试和 Windows x64 NSIS 安装包重打。`

大白话：现在软件里的说法和实际功能一致了，不会再把新流程说成截图 OCR。最新安装包已经重新生成，明天直接安装这个包验收即可。

## 2026-09-13 增补 19：新建资料页 Markdown 编辑与 AI 整理

状态：完成。

新建资料页与编辑资料页统一使用 Markdown 编辑器，支持编辑、对照和预览模式，以及标题、粗体、斜体、列表、链接等工具。新建资料页增加“AI 整理为 Markdown”流程：使用设置中的 Markdown 整理模型生成可编辑结果，同时展示渲染预览；用户点击“采用整理结果”后才替换正文，正文在请求期间被修改或模型遗漏图片引用时会阻止覆盖。剪贴板粘贴、文件选择和拖拽配图继续保留，图片保存到独立资料文件夹并在保存时补入正文引用。

验证：`npm run build`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`cargo test --manifest-path src-tauri/Cargo.toml --lib`（41 项通过、1 项因需要 Windows OCR fixture 忽略）和 `git diff --check` 均通过。重新执行 `npm run tauri:build` 成功生成 Windows x64 NSIS 安装包 [`拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/拾录%20ShiLu_0.1.0_x64-setup.exe)，大小 3,942,464 字节，生成时间 2026-09-13 23:16:22，SHA256：`0E480BF8FF203AB2FDEB4F659EB61BAD46FA2E0DAF9BABA9AF4E550712D53681`。

Git Summary：`统一新建页 Markdown 编辑与 AI 整理流程`

Git Description：`新建资料页接入 Markdown 编辑器、预览和可确认的 AI 整理结果，保留剪贴板、文件与拖拽配图导入；完善图片引用校验、正文变更保护，并完成 Windows x64 安装包重打。`

大白话：新增资料时现在就能直接写 Markdown、看排版、贴图片，也能先让 AI 整理后再决定要不要采用；AI 不会直接悄悄覆盖你的原文。最新安装包已包含这次更新。

## 2026-09-15 增补 21：资料阅读、配图与 AI 整理交互优化

状态：完成、回归检查与 Windows 安装包验收完成。

资料详情页的“返回资料库”改为与软件一致的图标按钮，提供悬停和键盘焦点反馈。阅读页会从正文 Markdown 中按引用顺序提取全部正文配图，集中显示在标题之后、正文之前；正文渲染会移除这些原位置的图片引用，因此不会重复显示。来源链接继续固定显示在文末。

新建资料页的正文配图面板已移到标题和来源链接之后、Markdown 编辑器之前，添加后立即显示缩略图。保存时会将本次管理的图片引用去重并统一放在正文顶部。新建资料保存成功后自动跳转“总览”，编辑已有资料时则留在当前页。

资料库和归档箱的每条资料现在同时提供归档/恢复与永久删除按钮。删除前必须确认，删除后会删除对应本地资料文件夹、Markdown 和图片，并刷新列表。AI 整理改为在正文区域内直接切换到左右对照：左侧保留原正文，右侧显示可编辑的 AI Markdown 结果及其渲染预览；仅点击“采用整理结果”才会替换正文，且仍会拦截缺失配图引用的结果。

验证：`npm run build`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`git diff --check` 均通过；`cargo test --manifest-path src-tauri/Cargo.toml --lib` 为 41 项通过、1 项因需要 Windows OCR fixture 忽略。`npm run tauri:build` 成功生成 Windows x64 NSIS 安装包 [`拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/%E6%8B%BE%E5%BD%95%20ShiLu_0.1.0_x64-setup.exe)，大小 3,944,004 字节，生成时间 2026-09-15 13:51:13，SHA256：`F61A5B62AFE6F360CDEF1619C1873EF7441C10DE75A28603E194C2208EC0E357`。

Git Summary：`优化资料配图、AI 对照润色与列表管理`

Git Description：`统一正文配图在阅读和新建流程中的顶部展示，新增保存后跳转总览及资料列表直删；重做 AI Markdown 整理为原文与结果对照复核，优化详情页返回按钮并完成 Windows 安装包验收。`

大白话：图片现在一加就能在正文前面看到，打开文章也是先看图片再看文字；新建保存后会直接回总览。资料库不必点进文章才能删除。AI 润色时左边是你原来的文字，右边是 AI 改过的 Markdown，确认满意再点采纳。

## 2026-09-13 增补 20：最终安装包验收

状态：完成。

在增补 19 的代码基础上重新执行 Windows x64 NSIS 打包，确认安装包生成成功且与当前工作区代码一致。

最终安装包：[`拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/%E6%8B%BE%E5%BD%95%20ShiLu_0.1.0_x64-setup.exe)，大小 3,942,572 字节，生成时间 2026-09-13 23:28:01，SHA256：`7CA53E0482B722429608EF126E131C80A0594C439A33DBE7464F6F323FF7DAC1`。

Git Summary：`完成最终 Windows 安装包验收`

Git Description：`基于当前资料编辑、Markdown 整理、正文配图与永久删除功能完成最终 Windows x64 NSIS 打包，记录安装包大小、生成时间和 SHA256 校验值。`

大白话：这是目前最新的一版安装包，安装前可以用 SHA256 校验值确认文件没有被替换或损坏。

## 2026-09-16 增补 23：统一资料数量并清理遗留草稿

状态：完成。

修复设置页与总览页资料数量不一致的问题。设置页原先按 `articles` 目录中的所有合法文件夹计数，而总览只展示 `active` 正式资料；现在后端统计会读取每条 `index.md` 的状态，仅统计正式资料，旧资料缺少状态字段时仍按正式资料兼容处理。新增回归测试覆盖正式资料与草稿混合场景。

已按确认范围清理资料库中的遗留草稿 `远程控制台`（目录 `20260913-221003-article-afe8e5`），删除其本地 Markdown 资料文件夹及图片。清理后资料库目录剩余 10 个正式资料文件夹，草稿目录为 0。

验证：`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`cargo test --manifest-path src-tauri/Cargo.toml --lib`（42 项通过、1 项因需要 Windows OCR fixture 忽略）、`npm run tauri:build` 和 `git diff --check` 均通过。最新 Windows x64 NSIS 安装包 [`拾录 ShiLu_0.1.0_x64-setup.exe`](../src-tauri/target/release/bundle/nsis/%E6%8B%BE%E5%BD%95%20ShiLu_0.1.0_x64-setup.exe)，大小 3,947,319 字节，生成时间 2026-09-16 09:07:48，SHA256：`6E1D19EC4DF9B0A5264E421B59C6041771C08E7B7EBEDC9B5DE73149ABE7D043`。

Git Summary：`统一资料数量统计并清理遗留草稿`

Git Description：`设置页改为只统计 active 正式资料，新增混合状态回归测试；按确认范围永久清理远程控制台遗留草稿，核对资料库剩余 10 条并重新生成 Windows 安装包。`

大白话：之前文件夹里有 11 条，其中 1 条是旧草稿，所以总览只显示 10 条。现在旧草稿已经删掉，设置页和总览都会显示 10 条，安装包也已重新生成。

## 2026-09-15 增补 22：总览显示全部资料并补齐新建页返回按钮

状态：实现、外部 Chrome 界面验收、Windows 安装包构建完成；未覆盖安装或操作用户真实资料库。

步骤 1 — 总览卡片：移除 `slice(0, 5)`，直接展示全部已收录资料，沿用后端更新时间倒序；区域标题同步为“全部已收录资料”，保留正文首图、悬停交互和空状态。自检：用 9 条模拟资料确认统计与 9 张卡片一致，滚动到底可点击打开第 9 条。

Git Summary：`总览展示全部已收录资料`

Git Description：`移除总览只展示前五条的数量限制，使用完整资料集合渲染卡片并同步区域标题，保留更新时间排序、首图和空状态。`

大白话：收录多少条就能在总览看到多少张卡片，超出一屏时向下滚动查看。

步骤 2 — 返回按钮：上次只修改了详情页的局部样式，新建页同名按钮没有对应样式，仍然使用系统原生外观。本次将 `.editor-back` 放入公共样式，删除详情页、编辑页中的冲突局部规则；三个页面统一圆角、边框、图标间距、悬停、按下和键盘焦点反馈。仅标题上方按钮保留与标题的间距，详情页横向工具栏不额外增加下边距。按 impeccable 产品界面规范复用现有主题与减少动画设置。

Git Summary：`统一返回按钮样式并修复新建页漏修`

Git Description：`将返回资料库和归档箱按钮样式集中到公共样式表，覆盖新建、编辑和阅读页，移除局部冲突并补齐悬停、按下、键盘焦点及减少动画状态。`

大白话：新建资料页的返回按钮也真正换成软件统一风格了，三个页面不会各用一套样式。

验证：`npm run build` 与 `git diff --check` 通过。使用独立外部 Chrome、生产构建和模拟 Tauri 接口完成 13 项界面检查，包括 1920×1080、1440×900、1100×680 的卡片展示及无横向溢出，第 9 条打开，三个页面返回资料库，已归档资料返回归档箱，鼠标点击、键盘 Enter、深色主题、折叠侧栏、减少动画及空状态。浏览器无运行时异常；测试未连接真实资料库。截图和结果位于 `work/overview-return-check/`。本次未修改 Rust 后端，未重复运行无关后端测试。

`npm run tauri:build` 成功。安装包：`src-tauri/target/release/bundle/nsis/拾录 ShiLu_0.1.0_x64-setup.exe`，大小 3,943,611 字节，生成时间 2026-09-15 17:40:40，SHA256：`6D9F918C1721CA579B9AE0E0BF9ACFD6F30AC322B9A81F795D84E032BF4EF891`。
