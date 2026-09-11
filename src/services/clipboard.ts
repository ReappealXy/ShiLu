import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ImageSource } from "./capture";

export type ClipboardImage = {
  source: Extract<ImageSource, { kind: "clipboard" }>;
  previewUrl: string;
  width: number;
  height: number;
  sizeBytes: number;
};

let sequence = 0;
const MAX_IMAGE_BYTES = 20 * 1024 * 1024;

function readDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(new Error("无法读取图片，请重新复制后再试。"));
    reader.readAsDataURL(blob);
  });
}

/** Clipboard images are kept in memory until the user confirms saving. */
export async function imageFromBlob(blob: Blob): Promise<ClipboardImage> {
  if (!blob.size) throw new Error("图片为空，请重新截图后再试。");
  if (blob.size > MAX_IMAGE_BYTES) throw new Error("单张图片不能超过 20 MB，请缩小截图范围后再试。");
  const bitmap = await createImageBitmap(blob).catch(() => {
    throw new Error("无法读取这张图片，请复制 PNG、JPG 或 WEBP 图片后再试。");
  });
  const { width, height } = bitmap;
  let png = blob;
  try {
    if (width > 20_000 || height > 20_000 || width * height > 40_000_000) {
      throw new Error("图片尺寸过大，请分成几张截图后再添加。");
    }
    if (blob.type !== "image/png") {
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const context = canvas.getContext("2d");
      if (!context) throw new Error("无法处理图片，请重新打开软件后再试。");
      context.drawImage(bitmap, 0, 0);
      png = await new Promise<Blob>((resolve, reject) => {
        canvas.toBlob((result) => result ? resolve(result) : reject(new Error("图片转换失败，请重新截图后再试。")), "image/png");
      });
    }
  } finally {
    bitmap.close();
  }
  if (png.size > MAX_IMAGE_BYTES) throw new Error("转换后的图片超过 20 MB，请缩小截图范围后再试。");
  const previewUrl = await readDataUrl(png);
  const name = `截图-${Date.now()}-${++sequence}.png`;
  return { source: { kind: "clipboard", name, base64: previewUrl.slice(previewUrl.indexOf(",") + 1) }, previewUrl, width, height, sizeBytes: png.size };
}

export function pastedImageFiles(event: ClipboardEvent): File[] {
  const items = Array.from(event.clipboardData?.items ?? []);
  const files = items.filter((item) => item.kind === "file" && item.type.startsWith("image/"))
    .map((item) => item.getAsFile()).filter((file): file is File => file !== null);
  return files.length ? files : Array.from(event.clipboardData?.files ?? []).filter((file) => file.type.startsWith("image/"));
}

export function hasPastedText(event: ClipboardEvent): boolean {
  return Array.from(event.clipboardData?.types ?? []).some((type) => type.startsWith("text/"));
}

/** Called only by an explicit paste action, never by a background listener. */
export async function readClipboardImages(): Promise<ClipboardImage[]> {
  if (isTauri()) {
    const { base64 } = await invoke<{ base64: string }>("read_clipboard_image");
    const bytes = Uint8Array.from(atob(base64), (character) => character.charCodeAt(0));
    const blob = new Blob([bytes], { type: "image/png" });
    return [await imageFromBlob(blob)];
  }
  if (!navigator.clipboard?.read) throw new Error("当前环境无法直接读取剪贴板，请使用 Ctrl + V 粘贴图片。");
  const items = await navigator.clipboard.read();
  const images: ClipboardImage[] = [];
  for (const item of items) {
    const type = item.types.find((value) => value === "image/png") ?? item.types.find((value) => value.startsWith("image/"));
    if (type) images.push(await imageFromBlob(await item.getType(type)));
  }
  if (!images.length) throw new Error("剪贴板中没有图片，请先截图或复制图片后再试。");
  return images;
}
