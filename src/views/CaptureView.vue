<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { stat } from "@tauri-apps/plugin-fs";
import {
  ArrowDown,
  ArrowUp,
  CheckCircle2,
  ClipboardPaste,
  FileImage,
  ImagePlus,
  Link2,
  LoaderCircle,
  Sparkles,
  Trash2,
  Upload,
  X,
} from "@lucide/vue";
import { createArticleWithSources, type ImageSource } from "../services/capture";
import { hasPastedText, imageFromBlob, pastedImageFiles, readClipboardImages, type ClipboardImage } from "../services/clipboard";
import { getErrorMessage } from "../services/library";
import { useRouter } from "vue-router";

const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "webp"]);

type CaptureImage = {
  id: string;
  source: ImageSource;
  path: string;
  name: string;
  extension: string;
  previewUrl: string;
  objectUrl: boolean;
  sizeBytes: number | null;
  width: number | null;
  height: number | null;
};

const title = ref("");
const router = useRouter();
const sourceUrl = ref("");
const images = ref<CaptureImage[]>([]);
const fileInput = ref<HTMLInputElement | null>(null);
const dragActive = ref(false);
const reorderFrom = ref<number | null>(null);
const saving = ref(false);
const pendingAdds = ref(0);
const errorMessage = ref("");
const successMessage = ref("");
const savedFolderName = ref("");
let imageSequence = 0;
let stopNativeDragListener: (() => void) | undefined;
let disposed = false;
let addQueue = Promise.resolve();

const imageCountLabel = computed(() => `${images.value.length} 张图片`);
const queueLocked = computed(() => saving.value || pendingAdds.value > 0 || Boolean(successMessage.value));
const canSave = computed(() => Boolean(title.value.trim()) && images.value.length > 0 && !queueLocked.value);

function getFileName(path: string): string {
  const normalized = path.replaceAll("\\", "/");
  return normalized.slice(normalized.lastIndexOf("/") + 1) || "未命名图片";
}

function getExtension(name: string): string {
  const extension = name.split(".").pop()?.toLowerCase() ?? "";
  return extension === "jpg" ? "jpg" : extension;
}

function isSupportedImage(name: string): boolean {
  return IMAGE_EXTENSIONS.has(getExtension(name));
}

function imagePreview(path: string): string {
  if (!isTauri()) return path;
  try {
    return convertFileSrc(path);
  } catch {
    return path;
  }
}

function createImage(path: string, name = getFileName(path), previewUrl = imagePreview(path), objectUrl = false, sizeBytes: number | null = null): CaptureImage {
  const image: CaptureImage = {
    id: `${Date.now()}-${imageSequence++}`,
    source: { kind: "path", path },
    path,
    name,
    extension: getExtension(name),
    previewUrl,
    objectUrl,
    sizeBytes,
    width: null,
    height: null,
  };
  inspectImage(image);
  return image;
}

async function inspectImage(image: CaptureImage) {
  if (image.path && isTauri()) {
    try {
      image.sizeBytes = (await stat(image.path)).size;
    } catch {
      // File metadata is optional; the image can still be imported.
    }
  }

  const probe = new Image();
  probe.onload = () => {
    image.width = probe.naturalWidth || null;
    image.height = probe.naturalHeight || null;
  };
  probe.src = image.previewUrl;
}

function addImagePaths(paths: string[]) {
  if (queueLocked.value || disposed) return;
  const existingPaths = new Set(images.value.map((image) => image.path).filter(Boolean));
  const unsupported: string[] = [];
  for (const path of paths) {
    const name = getFileName(path);
    if (!isSupportedImage(name)) {
      unsupported.push(name);
      continue;
    }
    if (existingPaths.has(path)) continue;
    existingPaths.add(path);
    images.value.push(createImage(path));
  }

  if (unsupported.length) {
    errorMessage.value = `已跳过不支持的文件：${unsupported.slice(0, 3).join("、")}${unsupported.length > 3 ? "等" : ""}。`;
  } else if (paths.length) {
    errorMessage.value = "";
  }
  successMessage.value = "";
}

function addBrowserFiles(files: File[]) {
  const supported = files.filter((file) => isSupportedImage(file.name) || file.type.startsWith("image/"));
  if (!supported.length) {
    errorMessage.value = "请添加 PNG、JPG、JPEG 或 WEBP 图片。";
    return;
  }
  queueClipboardImages(() => Promise.all(supported.map(imageFromBlob)));
}

function queueClipboardImages(load: () => Promise<ClipboardImage[]>) {
  if (saving.value || successMessage.value || disposed) return;
  pendingAdds.value++;
  errorMessage.value = "";
  // Read now, then append in invocation order even when image decoding is slower.
  const result = load().then((items) => ({ items }), (error: unknown) => ({ error }));
  addQueue = addQueue.then(async () => {
    const outcome = await result;
    if (disposed) return;
    if ("error" in outcome) {
      errorMessage.value = getErrorMessage(outcome.error, "无法粘贴图片，请重新截图后再试。");
      return;
    }
    for (const item of outcome.items) {
      images.value.push({
        id: `${Date.now()}-${imageSequence++}`, source: item.source, path: "", name: item.source.name,
        extension: "png", previewUrl: item.previewUrl, objectUrl: false,
        sizeBytes: item.sizeBytes, width: item.width, height: item.height,
      });
    }
  }).finally(() => { pendingAdds.value--; });
}

function pasteImages() {
  queueClipboardImages(readClipboardImages);
}

function onPaste(event: ClipboardEvent) {
  const files = pastedImageFiles(event);
  if (files.length) {
    event.preventDefault();
    if (!saving.value && !successMessage.value) addBrowserFiles(files);
  } else if (!hasPastedText(event) && isTauri()) {
    event.preventDefault();
    pasteImages();
  }
}

async function pickImages() {
  if (queueLocked.value) return;
  errorMessage.value = "";
  if (!isTauri()) {
    fileInput.value?.click();
    return;
  }

  try {
    const selected = await open({
      title: "选择要保存的截图",
      multiple: true,
      directory: false,
      filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp"] }],
    });
    if (Array.isArray(selected)) addImagePaths(selected);
    else if (typeof selected === "string") addImagePaths([selected]);
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "无法打开图片选择器，请稍后重试。");
  }
}

function onFileInput(event: Event) {
  const input = event.target as HTMLInputElement;
  addBrowserFiles(Array.from(input.files ?? []));
  input.value = "";
}

function onDrop(event: DragEvent) {
  event.preventDefault();
  dragActive.value = false;
  if (reorderFrom.value !== null || queueLocked.value || isTauri()) return;
  const files = Array.from(event.dataTransfer?.files ?? []);
  if (files.length) addBrowserFiles(files);
}

function onDragOver() {
  dragActive.value = true;
}

function removeImage(index: number) {
  if (queueLocked.value) return;
  const [removed] = images.value.splice(index, 1);
  if (removed?.objectUrl) URL.revokeObjectURL(removed.previewUrl);
}

function clearImages() {
  if (saving.value || pendingAdds.value > 0) return;
  images.value.forEach((image) => {
    if (image.objectUrl) URL.revokeObjectURL(image.previewUrl);
  });
  images.value = [];
}

function moveImage(index: number, offset: number) {
  if (queueLocked.value) return;
  const target = index + offset;
  if (target < 0 || target >= images.value.length) return;
  const current = images.value[index];
  images.value[index] = images.value[target];
  images.value[target] = current;
}

function startReorder(index: number) {
  if (queueLocked.value) return;
  reorderFrom.value = index;
}

function dropReorder(targetIndex: number) {
  const sourceIndex = reorderFrom.value;
  reorderFrom.value = null;
  if (sourceIndex === null || sourceIndex === targetIndex || queueLocked.value) return;
  const [moved] = images.value.splice(sourceIndex, 1);
  if (moved) images.value.splice(targetIndex, 0, moved);
}

function endReorder() {
  reorderFrom.value = null;
}

function formatBytes(bytes: number | null): string {
  if (bytes === null) return "读取中";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDimensions(image: CaptureImage): string {
  return image.width && image.height ? `${image.width} × ${image.height}` : "读取尺寸中";
}

function getNativeDragPath(event: { payload: { type: string; paths?: string[] } }) {
  const payload = event.payload;
  if (payload.type === "enter") dragActive.value = true;
  if (payload.type === "leave") dragActive.value = false;
  if (payload.type === "drop") {
    dragActive.value = false;
    if (payload.paths?.length) addImagePaths(payload.paths);
  }
}

async function saveArticle() {
  if (queueLocked.value) return;
  errorMessage.value = "";
  successMessage.value = "";
  if (!title.value.trim()) {
    errorMessage.value = "请先填写资料标题。";
    return;
  }
  if (!images.value.length) {
    errorMessage.value = "请至少添加一张图片。";
    return;
  }
  saving.value = true;
  try {
    const result = await createArticleWithSources(title.value.trim(), sourceUrl.value.trim(), images.value.map((image) => image.source));
    savedFolderName.value = result.article.folderName;
    successMessage.value = `已保存 ${result.images.images.length} 张图片，资料文件夹已经创建。`;
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "保存资料失败，请检查资料库设置后重试。");
  } finally {
    saving.value = false;
  }
}

function startAnotherArticle() {
  clearImages();
  title.value = "";
  sourceUrl.value = "";
  errorMessage.value = "";
  successMessage.value = "";
  savedFolderName.value = "";
}

onMounted(async () => {
  window.addEventListener("paste", onPaste);
  if (!isTauri()) return;
  try {
    stopNativeDragListener = await getCurrentWebview().onDragDropEvent(getNativeDragPath);
  } catch {
    // Browser development mode does not provide the native drag event bridge.
  }
});

onBeforeUnmount(() => {
  disposed = true;
  window.removeEventListener("paste", onPaste);
  stopNativeDragListener?.();
  images.value.forEach((image) => {
    if (image.objectUrl) URL.revokeObjectURL(image.previewUrl);
  });
});
</script>

<template>
  <section class="page-view capture-view" aria-labelledby="capture-heading">
    <div class="page-heading capture-heading">
      <div>
        <p class="page-eyebrow">建立一条资料</p>
        <h2 id="capture-heading">新建资料</h2>
        <p class="page-description">先放入截图，再补充标题和来源。保存后每张图片都会复制到这条资料自己的文件夹里。</p>
      </div>
      <div class="capture-progress" aria-label="当前步骤"><span class="capture-progress-active">01 素材</span><i></i><span>02 编辑</span></div>
    </div>

    <div class="capture-layout">
      <section class="content-panel capture-assets-panel" aria-labelledby="capture-assets-heading">
        <div class="capture-panel-heading">
          <div><p class="panel-kicker">STEP 01</p><h3 id="capture-assets-heading">添加截图</h3></div>
          <span class="capture-count">{{ imageCountLabel }}</span>
        </div>

        <div class="capture-dropzone" :class="{ 'capture-dropzone-active': dragActive }" @dragover.prevent="onDragOver" @dragleave.prevent="dragActive = false" @drop.prevent="onDrop">
          <span class="capture-dropzone-icon"><Upload :size="22" aria-hidden="true" /></span>
          <strong>{{ dragActive ? "松开鼠标即可添加" : "把截图拖到这里" }}</strong>
          <p>截图后可直接按 Ctrl + V 粘贴，无需先保存文件。</p>
          <div class="capture-add-actions"><button class="button button-secondary" type="button" :disabled="queueLocked" @click="pickImages"><ImagePlus :size="17" aria-hidden="true" />选择图片</button><button class="button button-secondary" type="button" :disabled="queueLocked" @click="pasteImages"><LoaderCircle v-if="pendingAdds" :size="17" class="capture-spin" aria-hidden="true" /><ClipboardPaste v-else :size="17" aria-hidden="true" />{{ pendingAdds ? "正在读取..." : "粘贴图片" }}</button></div>
          <small>PNG、JPG、JPEG、WEBP</small>
          <input ref="fileInput" class="capture-file-input" type="file" accept="image/png,image/jpeg,image/webp" multiple @change="onFileInput" />
        </div>

        <p v-if="pendingAdds" class="capture-paste-status" role="status">正在读取图片，完成后即可确认保存。</p>
        <div v-if="images.length" class="capture-queue-heading"><span>素材顺序</span><button class="capture-clear-button" type="button" :disabled="queueLocked" @click="clearImages"><Trash2 :size="14" aria-hidden="true" />清空</button></div>
        <ol v-if="images.length" class="capture-queue" aria-label="已选择的图片">
          <li v-for="(image, index) in images" :key="image.id" class="capture-queue-item" :class="{ 'capture-queue-item-dragging': reorderFrom === index }" :draggable="!queueLocked" @dragstart="startReorder(index)" @dragend="endReorder" @dragover.prevent @drop.stop="dropReorder(index)">
            <span class="capture-queue-index">{{ String(index + 1).padStart(2, "0") }}</span>
            <div class="capture-thumb"><img :src="image.previewUrl" :alt="image.name" /></div>
            <div class="capture-image-info"><strong :title="image.name">{{ image.name }}</strong><small>{{ formatDimensions(image) }} · {{ formatBytes(image.sizeBytes) }}</small></div>
            <div class="capture-item-actions">
              <button class="icon-button" type="button" title="上移" :aria-label="`将 ${image.name} 上移`" :disabled="queueLocked || index === 0" @click="moveImage(index, -1)"><ArrowUp :size="15" /></button>
              <button class="icon-button" type="button" title="下移" :aria-label="`将 ${image.name} 下移`" :disabled="queueLocked || index === images.length - 1" @click="moveImage(index, 1)"><ArrowDown :size="15" /></button>
              <button class="icon-button capture-remove-button" type="button" title="移除图片" :disabled="queueLocked" :aria-label="`移除 ${image.name}`" @click="removeImage(index)"><X :size="15" /></button>
            </div>
          </li>
        </ol>
        <div v-else class="capture-empty-queue"><FileImage :size="19" aria-hidden="true" /><span>还没有添加图片</span></div>
      </section>

      <section class="content-panel capture-details-panel" aria-labelledby="capture-details-heading">
        <div class="capture-panel-heading">
          <div><p class="panel-kicker">STEP 02</p><h3 id="capture-details-heading">补充信息</h3></div>
          <Sparkles :size="18" class="capture-heading-sparkle" aria-hidden="true" />
        </div>

        <form class="capture-form" @submit.prevent="saveArticle">
          <label class="capture-field"><span>资料标题 <b>必填</b></span><input v-model="title" type="text" maxlength="120" placeholder="例如：AI 创作工具合集" autocomplete="off" /></label>
          <label class="capture-field"><span>来源链接 <em>可选</em></span><span class="capture-input-with-icon"><Link2 :size="16" aria-hidden="true" /><input v-model="sourceUrl" type="url" placeholder="https://..." autocomplete="url" /></span></label>
          <p class="capture-form-hint">确认保存后，截图会存入这条资料的 images 文件夹。之后可进入编辑页手动识别文字，或继续补充图片。</p>

          <div class="capture-form-actions"><button class="button button-primary capture-save-button" type="submit" :disabled="!canSave"><LoaderCircle v-if="saving" :size="17" class="capture-spin" aria-hidden="true" /><CheckCircle2 v-else :size="17" aria-hidden="true" />{{ saving ? "正在保存..." : "保存资料" }}</button><button v-if="successMessage" class="button button-secondary" type="button" @click="router.push(`/articles/${savedFolderName}/edit`)"><FileImage :size="16" aria-hidden="true" />编辑资料</button><button v-if="successMessage" class="button button-secondary" type="button" @click="startAnotherArticle"><ImagePlus :size="16" aria-hidden="true" />继续新建</button></div>
        </form>

        <p v-if="successMessage" class="capture-feedback capture-feedback-success" role="status"><CheckCircle2 :size="16" aria-hidden="true" /><span>{{ successMessage }}<small>{{ savedFolderName }}</small></span></p>
        <p v-if="errorMessage" class="capture-feedback capture-feedback-error" role="alert">{{ errorMessage }}</p>
      </section>
    </div>
  </section>
</template>

<style scoped>
.capture-heading { align-items: end; }
.capture-progress { display: flex; align-items: center; gap: 9px; padding-bottom: 3px; color: var(--muted); font-size: 11px; font-weight: 700; white-space: nowrap; }
.capture-progress i { display: block; width: 28px; height: 1px; background: var(--line-strong); }
.capture-progress-active { color: var(--primary); }
.capture-layout { display: grid; grid-template-columns: minmax(0, 1.15fr) minmax(310px, 0.85fr); align-items: start; gap: 14px; }
.capture-assets-panel, .capture-details-panel { min-width: 0; padding: 22px; }
.capture-panel-heading { display: flex; align-items: start; justify-content: space-between; gap: 12px; margin-bottom: 17px; }
.capture-panel-heading h3 { margin: 0; color: var(--ink); font-size: 16px; font-weight: 730; line-height: 1.35; }
.capture-count { padding: 5px 8px; color: var(--muted-strong); background: var(--surface-alt); border-radius: 6px; font-size: 11px; font-weight: 680; }
.capture-heading-sparkle { color: var(--primary); }
.capture-dropzone { display: grid; min-height: 230px; align-content: center; justify-items: center; padding: 24px; color: var(--muted-strong); text-align: center; background: var(--surface-alt); border: 1px dashed var(--line-strong); border-radius: 10px; transition: border-color 150ms ease-out, background-color 150ms ease-out, transform 150ms ease-out; }
.capture-dropzone-active { color: var(--primary); background: var(--primary-soft); border-color: var(--primary); transform: translateY(-1px); }
.capture-dropzone-icon { display: grid; width: 45px; height: 45px; place-items: center; margin-bottom: 12px; color: var(--primary); background: var(--surface); border: 1px solid var(--line); border-radius: 11px; }
.capture-dropzone strong { color: var(--ink); font-size: 15px; font-weight: 720; }
.capture-dropzone p { margin: 7px 0 15px; color: var(--muted); font-size: 12px; line-height: 1.55; }
.capture-dropzone small { margin-top: 10px; color: var(--muted); font-size: 10px; }
.capture-add-actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; }
.capture-paste-status { margin: 12px 0; color: var(--muted-strong); font-size: 12px; line-height: 1.5; }
.capture-add-actions button:disabled, .capture-clear-button:disabled { opacity: 0.55; cursor: default; }
.capture-file-input { display: none; }
.capture-queue-heading { display: flex; align-items: center; justify-content: space-between; margin: 21px 0 9px; color: var(--muted-strong); font-size: 12px; font-weight: 700; }
.capture-clear-button { display: inline-flex; align-items: center; gap: 5px; padding: 4px 6px; color: var(--muted); background: transparent; border: 0; border-radius: 5px; font-size: 11px; }
.capture-clear-button:hover { color: oklch(0.55 0.17 25); background: var(--surface-alt); }
.capture-queue { display: grid; gap: 7px; margin: 0; padding: 0; list-style: none; }
.capture-queue-item { display: grid; grid-template-columns: 22px 50px minmax(0, 1fr) auto; align-items: center; gap: 9px; min-height: 66px; padding: 7px 8px; background: var(--surface-alt); border: 1px solid transparent; border-radius: 8px; cursor: grab; transition: border-color 130ms ease-out, background-color 130ms ease-out, opacity 130ms ease-out; }
.capture-queue-item:hover { border-color: var(--line-strong); }
.capture-queue-item:active { cursor: grabbing; }
.capture-queue-item-dragging { opacity: 0.45; border-color: var(--primary); }
.capture-queue-index { color: var(--muted); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 10px; text-align: center; }
.capture-thumb { display: grid; width: 50px; height: 50px; place-items: center; overflow: hidden; background: var(--surface); border: 1px solid var(--line); border-radius: 7px; }
.capture-thumb img { display: block; width: 100%; height: 100%; object-fit: cover; }
.capture-image-info { min-width: 0; }
.capture-image-info strong, .capture-image-info small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.capture-image-info strong { color: var(--ink); font-size: 12px; font-weight: 680; }
.capture-image-info small { margin-top: 5px; color: var(--muted); font-size: 10px; }
.capture-item-actions { display: flex; align-items: center; gap: 2px; }
.capture-item-actions .icon-button { width: 27px; height: 27px; border-radius: 6px; }
.capture-item-actions .icon-button:disabled { color: var(--line-strong); }
.capture-remove-button:hover { color: oklch(0.55 0.17 25); }
.capture-empty-queue { display: flex; min-height: 65px; align-items: center; justify-content: center; gap: 8px; color: var(--muted); border: 1px dashed var(--line); border-radius: 8px; font-size: 12px; }
.capture-form { display: grid; gap: 18px; }
.capture-field { display: grid; gap: 8px; color: var(--muted-strong); font-size: 12px; font-weight: 680; }
.capture-field span:first-child { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.capture-field b, .capture-field em { color: var(--primary); font-size: 10px; font-style: normal; font-weight: 680; }
.capture-field input { width: 100%; min-height: 40px; padding: 0 11px; color: var(--ink); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 8px; outline: 0; font-size: 13px; transition: border-color 130ms ease-out, box-shadow 130ms ease-out; }
.capture-field input:focus { border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus); }
.capture-input-with-icon { display: flex; align-items: center; gap: 8px; min-height: 40px; padding: 0 11px; color: var(--muted); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 8px; transition: border-color 130ms ease-out, box-shadow 130ms ease-out; }
.capture-input-with-icon:focus-within { color: var(--primary); border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus); }
.capture-input-with-icon input { min-height: 36px; padding: 0; border: 0; box-shadow: none; }
.capture-form-hint { margin: -2px 0 0; color: var(--muted); font-size: 11px; line-height: 1.6; }
.capture-form-actions { display: flex; flex-wrap: wrap; gap: 9px; padding-top: 3px; }
.capture-save-button { min-width: 126px; }
.capture-save-button:disabled { color: var(--muted); background: var(--surface-alt); border-color: var(--line); }
.capture-spin { animation: capture-spin 850ms linear infinite; }
.capture-feedback { display: flex; align-items: start; gap: 7px; margin: 18px 0 0; padding: 10px 11px; border-radius: 8px; font-size: 12px; line-height: 1.5; }
.capture-feedback span { min-width: 0; }
.capture-feedback small { display: block; margin-top: 4px; overflow-wrap: anywhere; color: var(--muted); font-size: 10px; }
.capture-feedback-success { color: var(--success); background: var(--success-soft); }
.capture-feedback-error { color: oklch(0.55 0.17 25); background: color-mix(in oklch, oklch(0.55 0.17 25) 9%, var(--surface)); }
@keyframes capture-spin { to { transform: rotate(360deg); } }

@media (max-width: 1180px) {
  .capture-layout { grid-template-columns: minmax(0, 1fr) minmax(300px, 0.8fr); }
  .capture-assets-panel, .capture-details-panel { padding: 18px; }
}

@media (prefers-reduced-motion: reduce) {
  .capture-spin { animation: none; }
}
</style>
