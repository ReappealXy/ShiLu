# 拾录数据规范

## 1. 文件夹结构

```text
ShiLuData/
├─ articles/
│  └─ 20260907-143012-newsnow-a1b2c3/
│     ├─ index.md
│     ├─ images/
│     │  ├─ 001.png
│     │  └─ 002.jpg
│     ├─ raw/
│     │  ├─ ocr.txt
│     │  └─ ai-draft.json
│     └─ versions/
│        └─ 20260907-143500.md
├─ settings.json
└─ search-index.json
```

每条资料只读写自己的文件夹。删除资料时先移动到软件回收站目录，用户确认后再永久删除。

## 2. Markdown 格式

```md
---
title: AI 创作工作台
summary: 面向短篇小说、剧本和互动影像的创作工具集合。
source: TG 小树屋
tags:
  - AI
  - 写作
links:
  - https://example.com
created_at: 2026-09-07T14:30:12+08:00
updated_at: 2026-09-07T14:35:00+08:00
---

## 正文

这里是用户确认后的文章内容。

## 我的备注

这里是用户自己的补充。
```

## 3. 文件命名规则

- 目录名：`YYYYMMDD-HHmmss-slug-shortid`。
- 图片名：按导入顺序生成 `001`、`002`，保留原始扩展名。
- 不使用用户输入直接作为路径；标题只生成 slug，过滤 Windows 保留字符。
- 文件路径统一使用 UTF-8，正文保留中文原文。

## 4. 版本策略

`index.md` 是当前最终版本；`raw/` 保存原始 OCR 和最后一次 AI 草稿；开启版本保留时，历史 Markdown 放在 `versions/`。所有版本都引用同一资料目录内的图片。

