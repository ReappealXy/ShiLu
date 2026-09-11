import { invoke } from "@tauri-apps/api/core";

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
};

export type ArticleDraft = Pick<ArticleDocument, "title" | "summary" | "sourceUrl" | "tags" | "content" | "notes">;

export type ArticleSummary = Pick<ArticleDocument, "id" | "folderName" | "title" | "summary" | "sourceUrl" | "tags" | "updatedAt" | "markdownPath">;

export type OcrResult = { articleId: string; text: string; imageCount: number; rawPath: string };

export function readArticle(articleReference: string): Promise<ArticleDocument> {
  return invoke<ArticleDocument>("read_article", { articleReference });
}

export function saveArticle(articleReference: string, draft: ArticleDraft): Promise<ArticleDocument> {
  return invoke<ArticleDocument>("save_article", { articleReference, draft });
}

export function listArticles(query = ""): Promise<ArticleSummary[]> {
  return invoke<ArticleSummary[]>("list_articles", { query });
}

export function ocrArticleImages(articleReference: string): Promise<OcrResult> {
  return invoke<OcrResult>("ocr_article_images", { articleReference });
}
