# 拾录产品原型图

这些原型图用于确认页面结构、信息层级和交互方向，后续开发以 `PRODUCT.md`、`UI_DESIGN.md` 和本页为依据。原型图中的资料、链接、数量和图片均为演示内容。

## 页面清单

| 文件 | 页面 | 主要确认内容 |
| --- | --- | --- |
| [01-library-dashboard.png](../output/prototypes/01-library-dashboard.png) | 资料总览 | 侧栏、搜索、统计、最近编辑和标签云 |
| [02-capture-recognition.png](../output/prototypes/02-capture-recognition.png) | 截图导入与识别 | 拖拽区、多图队列、识别方式和进度状态 |
| [03-article-editor.png](../output/prototypes/03-article-editor.png) | 文章编辑 | 结构化字段、Markdown 编辑器、素材和 AI 操作 |
| [04-polish-compare.png](../output/prototypes/04-polish-compare.png) | AI 润色对比 | 原文/建议稿并排、改动高亮和替换确认 |
| [05-settings-library.png](../output/prototypes/05-settings-library.png) | 设置与资料库 | 资料库路径、模型连接、Key 掩码和文件树 |

## 设计说明

- 整体沿用参考项目中有效的桌面工作台模式：窄侧栏、固定顶部栏、浅色面板、轻阴影和短促反馈动画。
- 拾录将主色收敛为蓝色，绿色只表示连接或保存成功，橙色表示待确认，避免状态含义混淆。
- 编辑页是核心页面，原始截图、识别结果、AI 建议和用户最终正文必须同时可追溯。
- 原型中的文字可能存在少量模型生成排版误差；实现时以字段名称和 `DATA_SPEC.md` 为准。

