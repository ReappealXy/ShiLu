import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type LibraryStatus = {
  configured: boolean;
  libraryPath: string | null;
  ready: boolean;
  missingItems: string[];
  articleCount: number;
};

export type ArticleSkeleton = {
  id: string;
  folderName: string;
  folderPath: string;
  markdownPath: string;
};

export async function chooseLibraryDirectory(): Promise<string | null> {
  const selectedPath = await open({
    directory: true,
    multiple: false,
    title: "选择资料库文件夹",
  });

  return typeof selectedPath === "string" ? selectedPath : null;
}

export function getLibraryStatus(): Promise<LibraryStatus> {
  return invoke<LibraryStatus>("get_library_status");
}

export function initializeLibrary(libraryPath: string): Promise<LibraryStatus> {
  return invoke<LibraryStatus>("initialize_library", { libraryPath });
}

export function migrateLibrary(libraryPath: string): Promise<LibraryStatus> {
  return invoke<LibraryStatus>("migrate_library", { libraryPath });
}

export function createArticleSkeleton(title: string, sourceUrl?: string): Promise<ArticleSkeleton> {
  return invoke<ArticleSkeleton>("create_article_skeleton", {
    title,
    sourceUrl: sourceUrl ?? null,
  });
}

export function getErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.trim()) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string" && message.trim()) return message;
  }
  return fallback;
}
