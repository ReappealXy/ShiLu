import { invoke } from "@tauri-apps/api/core";

export type ArticleSkeleton = {
  id: string;
  folderName: string;
  folderPath: string;
  markdownPath: string;
};

export type ImportedImage = {
  fileName: string;
  originalFileName: string;
  path: string;
  relativePath: string;
  extension: string;
  sizeBytes: number;
};

export type ImageImportResult = {
  articleId: string;
  articleFolderName: string;
  images: ImportedImage[];
};

export type CaptureResult = {
  article: ArticleSkeleton;
  images: ImageImportResult;
};

export function createArticleSkeleton(title: string, sourceUrl?: string): Promise<ArticleSkeleton> {
  return invoke<ArticleSkeleton>("create_article_skeleton", {
    title,
    sourceUrl: sourceUrl?.trim() || null,
  });
}

export function importArticleImages(articleReference: string, sourcePaths: string[]): Promise<ImageImportResult> {
  return invoke<ImageImportResult>("import_article_images", {
    articleReference,
    sourcePaths,
  });
}

/** Create the article first, then copy its source images into the article folder. */
export async function createArticleWithImages(
  title: string,
  sourceUrl: string,
  sourcePaths: string[],
): Promise<CaptureResult> {
  const article = await createArticleSkeleton(title, sourceUrl);
  const images = await importArticleImages(article.folderName, sourcePaths);
  return { article, images };
}

