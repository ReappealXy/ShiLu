import { invoke } from "@tauri-apps/api/core";

export type ArticleStatus = "draft" | "active" | "archived";
export type CaptureStep = 1 | 2 | 3;

export type ArticleDocument = {
  id: string;
  folderName: string;
  title: string;
  summary: string;
  sourceUrl: string;
  tags: string[];
  content: string;
  notes: string;
  createdAt: string;
  updatedAt: string;
  markdownPath: string;
  status: ArticleStatus;
  captureStep: CaptureStep;
  sourceImages: string[];
};

/** Payload accepted when saving. Legacy metadata remains optional for old files. */
export type ArticleDraft = Pick<ArticleDocument, "title" | "sourceUrl" | "content">
  & Partial<Pick<ArticleDocument, "summary" | "tags" | "notes" | "status" | "captureStep" | "sourceImages">>;

export type ArticleSummary = Pick<ArticleDocument, "id" | "folderName" | "title" | "summary" | "sourceUrl" | "tags" | "updatedAt" | "markdownPath" | "status" | "captureStep" | "sourceImages"> & {
  firstContentImage: string | null;
  content?: string;
};

export type OcrResult = { articleId: string; text: string; imageCount: number; rawPath: string };

export function readArticle(articleReference: string): Promise<ArticleDocument> {
  return invoke<ArticleDocument>("read_article", { articleReference });
}

export function saveArticle(articleReference: string, draft: ArticleDraft): Promise<ArticleDocument> {
  return invoke<ArticleDocument>("save_article", { articleReference, draft });
}

export function listArticles(query = "", status: ArticleStatus = "active"): Promise<ArticleSummary[]> {
  return invoke<ArticleSummary[]>("list_articles", { query, status });
}

export function setArticleStatus(articleReference: string, status: ArticleStatus): Promise<ArticleDocument> {
  return invoke<ArticleDocument>("set_article_status", { articleReference, status });
}

/** Permanently removes an article folder, markdown file, and associated images. */
export function deleteArticlePermanently(articleReference: string): Promise<void> {
  return invoke<void>("delete_article_permanently", { articleReference });
}

export function ocrArticleImages(articleReference: string): Promise<OcrResult> {
  return invoke<OcrResult>("ocr_article_images", { articleReference });
}
