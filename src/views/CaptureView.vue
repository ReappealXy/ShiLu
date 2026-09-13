<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { stat } from "@tauri-apps/plugin-fs";
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Check, CheckCircle2, ClipboardPaste, FileImage, ImagePlus, Link2, LoaderCircle, Save, ScanText, Settings2, Trash2, Upload, X } from "@lucide/vue";
import { onBeforeRouteLeave, onBeforeRouteUpdate, useRoute, useRouter } from "vue-router";
import { createCaptureDraft, importArticleSources, type ImageSource, type ImportedImage } from "../services/capture";
import { hasPastedText, imageFromBlob, pastedImageFiles, readClipboardImages, type ClipboardImage } from "../services/clipboard";
import { readArticle, saveArticle, type ArticleDocument } from "../services/editor";
import { getErrorMessage } from "../services/library";
import { getModelSettings, ocrArticleWithModel } from "../services/model";
import { cleanOcrText, inferOcrTitle, removeOcrTitle } from "../services/markdown";
import { displayPath } from "../services/paths";

const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "webp"]);

type CaptureImage = {
  id: string;
  source: ImageSource;
  path: string;
  name: string;
  previewUrl: string;
  relativePath?: string;
  sizeBytes: number | null;
  width: number | null;
  height: number | null;
};

const route = useRoute();
const router = useRouter();
const step = ref<1 | 2>(1);
const title = ref("");
const sourceUrl = ref("");
const images = ref<CaptureImage[]>([]);
const document = ref<ArticleDocument | null>(null);
const savedFolderName = ref("");
const fileInput = ref<HTMLInputElement | null>(null);
const titleInput = ref<HTMLInputElement | null>(null);
const dragActive = ref(false);
const reorderFrom = ref<number | null>(null);
const loading = ref(false);
const loadFailed = ref(false);
const saving = ref(false);
const ocrRunning = ref(false);
const pendingAdds = ref(0);
const pendingOcrText = ref<string | null>(null);
const errorMessage = ref("");
const successMessage = ref("");
const modelNeedsSetup = ref(false);
const lastSavedFingerprint = ref("");
let imageSequence = 0;
let stopNativeDragListener: (() => void) | undefined;
let disposed = false;
let addQueue = Promise.resolve();
let allowNavigation = false;

const queueLocked = computed(() => loading.value || saving.value || ocrRunning.value || pendingAdds.value > 0);
const hasContent = computed(() => images.value.length > 0 || Boolean(title.value.trim() || sourceUrl.value.trim()));
const fingerprint = computed(() => JSON.stringify({ title: title.value, sourceUrl: sourceUrl.value, step: step.value, images: images.value.map((image) => image.id), ocr: pendingOcrText.value }));
const dirty = computed(() => !loading.value && !loadFailed.value && (Boolean(savedFolderName.value) || hasContent.value) && fingerprint.value !== lastSavedFingerprint.value);
const canSave = computed(() => !queueLocked.value && !loadFailed.value && (Boolean(savedFolderName.value) || hasContent.value));
const canRecognize = computed(() => !queueLocked.value && images.value.length > 0);

function notify(message: string) {
  window.dispatchEvent(new CustomEvent("shilu:toast", { detail: message }));
}

function fileName(path: string): string {
  const normalized = path.replaceAll("\\", "/");
  return normalized.slice(normalized.lastIndexOf("/") + 1) || "未命名图片";
}

function isSupportedImage(name: string): boolean {
  return IMAGE_EXTENSIONS.has(name.split(".").pop()?.toLowerCase() ?? "");
}

function imagePreview(path: string): string {
  return isTauri() ? convertFileSrc(displayPath(path)) : path;
}

function createImage(path: string, name = fileName(path), relativePath?: string): CaptureImage {
  return { id: `${Date.now()}-${imageSequence++}`, source: { kind: "path", path }, path, name, previewUrl: imagePreview(path), relativePath, sizeBytes: null, width: null, height: null };
}

async function inspectImage(image: CaptureImage) {
  if (image.path && isTauri()) {
    try { image.sizeBytes = (await stat(image.path)).size; } catch { /* Metadata is optional for a valid image. */ }
  }
  if (disposed) return;
  const probe = new Image();
  probe.onload = () => {
    if (disposed) return;
    image.width = probe.naturalWidth || null;
    image.height = probe.naturalHeight || null;
  };
  probe.src = image.previewUrl;
}

function changedImages() {
  pendingOcrText.value = null;
  successMessage.value = "";
}

function addImagePaths(paths: string[]) {
  if (queueLocked.value || disposed || step.value !== 1) return;
  const existingPaths = new Set(images.value.map((image) => image.path).filter(Boolean));
  const unsupported: string[] = [];
  let added = 0;
  for (const path of paths) {
    if (!isSupportedImage(fileName(path))) { unsupported.push(fileName(path)); continue; }
    if (existingPaths.has(path)) continue;
    existingPaths.add(path);
    images.value.push(createImage(path));
    void inspectImage(images.value[images.value.length - 1]);
    added++;
  }
  if (added) changedImages();
  errorMessage.value = unsupported.length ? `已跳过不支持的文件：${unsupported.slice(0, 3).join("、")}。` : "";
  successMessage.value = added ? `已添加 ${added} 张截图。` : unsupported.length ? "" : "这些截图已在列表中。";
}

function queueClipboardImages(load: () => Promise<ClipboardImage[]>) {
  if (loading.value || saving.value || ocrRunning.value || disposed || step.value !== 1) return;
  pendingAdds.value++;
  errorMessage.value = "";
  successMessage.value = "";
  // Read immediately, then append in invocation order even when decoding finishes out of order.
  const result = load().then((items) => ({ items }), (error: unknown) => ({ error }));
  addQueue = addQueue.then(async () => {
    const outcome = await result;
    if (disposed) return;
    if ("error" in outcome) { errorMessage.value = getErrorMessage(outcome.error, "无法粘贴图片，请重新截图后再试。"); return; }
    for (const item of outcome.items) {
      images.value.push({ id: `${Date.now()}-${imageSequence++}`, source: item.source, path: "", name: item.source.name, previewUrl: item.previewUrl, sizeBytes: item.sizeBytes, width: item.width, height: item.height });
    }
    if (outcome.items.length) {
      changedImages();
      successMessage.value = `已添加 ${outcome.items.length} 张截图。`;
    }
  }).finally(() => { pendingAdds.value--; });
}

function addBrowserFiles(files: File[]) {
  const supported = files.filter((file) => isSupportedImage(file.name) || file.type.startsWith("image/"));
  if (!supported.length) { errorMessage.value = "请添加 PNG、JPG、JPEG 或 WEBP 图片。"; return; }
  queueClipboardImages(() => Promise.all(supported.map(imageFromBlob)));
}

function pasteImages() { queueClipboardImages(readClipboardImages); }

function onPaste(event: ClipboardEvent) {
  if (step.value !== 1 || loading.value) return;
  const files = pastedImageFiles(event);
  if (files.length) { event.preventDefault(); addBrowserFiles(files); }
  else if (!hasPastedText(event) && isTauri()) { event.preventDefault(); pasteImages(); }
}

async function pickImages() {
  if (queueLocked.value) return;
  errorMessage.value = "";
  if (!isTauri()) { fileInput.value?.click(); return; }
  try {
    const selected = await open({ title: "选择待识别截图", multiple: true, directory: false, filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp"] }] });
    if (Array.isArray(selected)) addImagePaths(selected);
    else if (typeof selected === "string") addImagePaths([selected]);
  } catch (error) { errorMessage.value = getErrorMessage(error, "无法打开图片选择器，请稍后重试。"); }
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

function removeImage(index: number) {
  if (queueLocked.value) return;
  images.value.splice(index, 1);
  changedImages();
}

function clearImages() {
  if (queueLocked.value) return;
  images.value = [];
  changedImages();
}

function moveImage(index: number, offset: number) {
  if (queueLocked.value) return;
  const target = index + offset;
  if (target < 0 || target >= images.value.length) return;
  [images.value[index], images.value[target]] = [images.value[target], images.value[index]];
  changedImages();
}

function dropReorder(targetIndex: number) {
  const sourceIndex = reorderFrom.value;
  reorderFrom.value = null;
  if (sourceIndex === null || sourceIndex === targetIndex || queueLocked.value) return;
  const [moved] = images.value.splice(sourceIndex, 1);
  if (moved) images.value.splice(targetIndex, 0, moved);
  changedImages();
}

function formatBytes(bytes: number | null): string {
  if (bytes === null) return "本地图片";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function getNativeDragPath(event: { payload: { type: string; paths?: string[] } }) {
  if (step.value !== 1 || queueLocked.value) return;
  const payload = event.payload;
  if (payload.type === "enter") dragActive.value = true;
  if (payload.type === "leave") dragActive.value = false;
  if (payload.type === "drop") { dragActive.value = false; if (payload.paths?.length) addImagePaths(payload.paths); }
}

function rememberImported(pending: CaptureImage[], imported: ImportedImage[]) {
  for (let index = 0; index < pending.length; index++) {
    const item = imported[index];
    if (!item) throw new Error("图片保存结果不完整，请保留当前页面并重试。");
    pending[index].relativePath = item.relativePath;
    pending[index].path = item.path;
    pending[index].source = { kind: "path", path: item.path };
  }
}

async function persistDraft(showFeedback = true): Promise<boolean> {
  if (loading.value || saving.value || pendingAdds.value || loadFailed.value) return false;
  if (!savedFolderName.value && !hasContent.value) return true;
  saving.value = true;
  errorMessage.value = "";
  try {
    if (!savedFolderName.value) {
      const result = await createCaptureDraft(title.value.trim(), sourceUrl.value.trim(), images.value.map((image) => image.source));
      savedFolderName.value = result.article.folderName;
      rememberImported(images.value, result.images.images);
    }
    if (!document.value) document.value = await readArticle(savedFolderName.value);
    const pending = images.value.filter((image) => !image.relativePath);
    if (pending.length) {
      const result = await importArticleSources(savedFolderName.value, pending.map((image) => image.source));
      rememberImported(pending, result.images);
    }
    document.value = await saveArticle(savedFolderName.value, {
      title: title.value.trim(), summary: document.value.summary, sourceUrl: sourceUrl.value.trim(), tags: document.value.tags,
      content: pendingOcrText.value ?? document.value.content, notes: document.value.notes,
      status: "draft", captureStep: pendingOcrText.value !== null ? 3 : step.value,
      sourceImages: images.value.map((image) => image.relativePath!),
    });
    lastSavedFingerprint.value = fingerprint.value;
    if (showFeedback) successMessage.value = "已保存当前进度。";
    return true;
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "保存失败，当前内容仍保留在页面中，请重试。");
    return false;
  } finally { saving.value = false; }
}

async function goToDetails() {
  if (queueLocked.value) return;
  if (!images.value.length) { errorMessage.value = "请至少添加一张待识别截图。"; return; }
  errorMessage.value = "";
  successMessage.value = "";
  step.value = 2;
  await nextTick();
  titleInput.value?.focus();
}

function goToImages() {
  if (queueLocked.value) return;
  step.value = 1;
  errorMessage.value = "";
  successMessage.value = "";
}

async function recognizeAndEdit() {
  if (!canRecognize.value) return;
  if (sourceUrl.value.trim()) {
    try {
      const url = new URL(sourceUrl.value.trim());
      if (url.protocol !== "http:" && url.protocol !== "https:") throw new Error();
    } catch { errorMessage.value = "来源链接应以 http:// 或 https:// 开头，也可以暂时留空。"; return; }
  }
  ocrRunning.value = true;
  errorMessage.value = "";
  successMessage.value = "";
  modelNeedsSetup.value = false;
  try {
    if (!(await persistDraft(false))) return;
    if (pendingOcrText.value === null) {
      const settings = await getModelSettings();
      const config = settings.ocrModel;
      if (!config?.enabled || !config.baseUrl.trim() || !config.apiKey.trim() || !config.model.trim()) {
        modelNeedsSetup.value = true;
        errorMessage.value = "请先在设置中启用并保存 OCR 模型。截图和文章信息已保存，请完成模型设置后继续。";
        return;
      }
      const result = await ocrArticleWithModel(savedFolderName.value);
      if (!result.text.trim()) throw new Error("模型未返回可编辑文字，请检查截图或 OCR 模型后重试。");
      let cleanedText = cleanOcrText(result.text);
      if (!cleanedText) throw new Error("模型返回的内容只有格式包装，没有可编辑文字，请检查截图或 OCR 模型后重试。");
      if (!title.value.trim()) {
        const inferredTitle = inferOcrTitle(cleanedText);
        if (inferredTitle) {
          title.value = inferredTitle;
        }
      }
      if (!title.value.trim()) {
        errorMessage.value = "OCR 未识别到明确标题，请返回补充资料标题后重试。";
        titleInput.value?.focus();
        return;
      }
      cleanedText = removeOcrTitle(cleanedText, title.value);
      pendingOcrText.value = cleanedText;
      if (!(await persistDraft(false))) return;
    }
    allowNavigation = true;
    const failure = await router.push({ path: `/articles/${savedFolderName.value}/edit`, query: { from: "library", ocr: "success" } });
    if (failure) { allowNavigation = false; errorMessage.value = "文字已保存，暂时无法进入编辑页，请再次点击下一步。"; }
  } catch (error) {
    allowNavigation = false;
    errorMessage.value = getErrorMessage(error, "OCR 识别失败，截图和信息已保留，请重试。");
  } finally { ocrRunning.value = false; }
}

async function loadDraft() {
  allowNavigation = false;
  loading.value = true;
  loadFailed.value = false;
  errorMessage.value = "";
  successMessage.value = "";
  modelNeedsSetup.value = false;
  document.value = null;
  savedFolderName.value = "";
  pendingOcrText.value = null;
  title.value = "";
  sourceUrl.value = "";
  images.value = [];
  step.value = 1;
  const reference = typeof route.query.draft === "string" ? route.query.draft : "";
  try {
    if (reference) {
      const article = await readArticle(reference);
      if (disposed) return;
      if (article.status !== "draft" || article.captureStep === 3) {
        loading.value = false;
        allowNavigation = true;
        await router.replace({ path: `/articles/${article.folderName}/edit`, query: { from: article.status === "archived" ? "archive" : "library" } });
        return;
      }
      document.value = article;
      savedFolderName.value = article.folderName;
      title.value = ["未命名资料", "未命名草稿"].includes(article.title) ? "" : article.title;
      sourceUrl.value = article.sourceUrl;
      step.value = article.captureStep === 1 ? 1 : 2;
      const directory = article.markdownPath.replace(/[\\/][^\\/]+$/, "");
      images.value = article.sourceImages.map((relativePath) => createImage(`${directory}/${relativePath}`, fileName(relativePath), relativePath));
      images.value.forEach((image) => void inspectImage(image));
      successMessage.value = "已恢复未完成的资料。";
    }
    lastSavedFingerprint.value = fingerprint.value;
  } catch (error) {
    loadFailed.value = true;
    errorMessage.value = getErrorMessage(error, "无法读取这份资料，请重试或返回资料库。");
  } finally { loading.value = false; }
}

async function beforeLeave() {
  if (allowNavigation) return true;
  if (queueLocked.value) {
    errorMessage.value = ocrRunning.value ? "正在识别并保存截图，请完成后再切换页面。" : "正在处理图片或保存当前进度，请稍候再切换页面。";
    notify(errorMessage.value);
    return false;
  }
  if (!dirty.value) return true;
  const saved = await persistDraft(false);
  if (saved) notify("已自动保存当前进度。");
  else notify("保存失败，内容已保留在当前页面，请重试。");
  return saved;
}

function onBeforeUnload(event: BeforeUnloadEvent) {
  if (!dirty.value && !queueLocked.value) return;
  event.preventDefault();
  event.returnValue = "";
}

function onKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
    event.preventDefault();
    if (canSave.value) void persistDraft();
  }
}

onBeforeRouteLeave(beforeLeave);
onBeforeRouteUpdate(beforeLeave);
watch(() => route.query.draft, () => { void loadDraft(); });
onMounted(async () => {
  window.addEventListener("paste", onPaste);
  window.addEventListener("beforeunload", onBeforeUnload);
  window.addEventListener("keydown", onKeydown);
  await loadDraft();
  if (!isTauri() || disposed) return;
  try {
    const stop = await getCurrentWebview().onDragDropEvent(getNativeDragPath);
    if (disposed) stop(); else stopNativeDragListener = stop;
  } catch { /* Browser development does not provide the native drag bridge. */ }
});
onBeforeUnmount(() => {
  disposed = true;
  window.removeEventListener("paste", onPaste);
  window.removeEventListener("beforeunload", onBeforeUnload);
  window.removeEventListener("keydown", onKeydown);
  stopNativeDragListener?.();
});
</script>

<template>
  <section class="page-view capture-view" aria-labelledby="capture-heading">
    <div class="page-heading capture-heading">
      <div><h2 id="capture-heading">{{ savedFolderName ? "继续整理资料" : "新建资料" }}</h2></div>
      <div class="capture-save-actions"><span v-if="dirty" class="capture-unsaved">未保存</span><button class="button button-secondary" type="button" :disabled="!canSave" @click="persistDraft()"><LoaderCircle v-if="saving" :size="16" class="capture-spin" /><Save v-else :size="16" />{{ saving ? "保存中..." : "保存进度" }}</button></div>
    </div>

    <ol class="capture-progress" aria-label="新建资料进度">
      <li :class="{ active: step === 1, complete: step > 1 }" :aria-current="step === 1 ? 'step' : undefined"><span class="capture-step-number"><Check v-if="step > 1" :size="15" /><template v-else>1</template></span><span>添加截图</span></li>
      <li :class="{ active: step === 2 }" :aria-current="step === 2 ? 'step' : undefined"><span class="capture-step-number">2</span><span>补充信息</span></li>
      <li><span class="capture-step-number">3</span><span>编辑正文</span></li>
    </ol>

    <div v-if="loading" class="capture-loading" aria-busy="true"><div></div><div></div><div></div><span role="status">正在读取资料...</span></div>
    <div v-else-if="loadFailed" class="capture-load-error"><FileImage :size="24" /><h3>暂时无法打开资料</h3><p role="alert">{{ errorMessage }}</p><div><button class="button button-secondary" type="button" @click="loadDraft">重新读取</button><button class="button button-secondary" type="button" @click="router.push('/library')">返回资料库</button></div></div>
    <div v-else class="capture-workspace">
      <section v-show="step === 1" class="capture-assets-panel" aria-labelledby="capture-assets-heading">
        <div class="capture-panel-heading"><h3 id="capture-assets-heading">待识别截图</h3><span class="capture-count">{{ images.length }} 张</span></div>
        <div class="capture-dropzone" :class="{ 'capture-dropzone-active': dragActive }" @dragover.prevent="dragActive = !queueLocked" @dragleave.prevent="dragActive = false" @drop.prevent="onDrop">
          <Upload :size="26" class="capture-upload-icon" aria-hidden="true" />
          <strong>{{ dragActive ? "松开鼠标即可添加" : "添加截图" }}</strong>
          <div class="capture-add-actions"><button class="button button-secondary" type="button" :disabled="queueLocked" @click="pickImages"><ImagePlus :size="17" />选择图片</button><button class="button button-secondary" type="button" :disabled="queueLocked" @click="pasteImages"><LoaderCircle v-if="pendingAdds" :size="17" class="capture-spin" /><ClipboardPaste v-else :size="17" />{{ pendingAdds ? "正在读取..." : "粘贴图片" }}</button></div>
          <small>PNG · JPG · JPEG · WEBP</small>
          <input ref="fileInput" class="capture-file-input" type="file" accept="image/png,image/jpeg,image/webp" multiple @change="onFileInput" />
        </div>
        <div v-if="images.length" class="capture-queue-heading"><span>识别顺序</span><button class="capture-clear-button" type="button" :disabled="queueLocked" @click="clearImages"><Trash2 :size="14" />清空列表</button></div>
        <ol v-if="images.length" class="capture-queue" aria-label="待识别截图列表">
          <li v-for="(image, index) in images" :key="image.id" class="capture-queue-item" :class="{ 'capture-queue-item-dragging': reorderFrom === index }" :draggable="!queueLocked" @dragstart="!queueLocked && (reorderFrom = index)" @dragend="reorderFrom = null" @dragover.prevent @drop.prevent.stop="dropReorder(index)">
            <span class="capture-queue-index">{{ index + 1 }}</span>
            <div class="capture-thumb"><img :src="image.previewUrl" :alt="`待识别截图 ${index + 1}`" /></div>
            <div class="capture-image-info"><strong :title="image.name">{{ image.name }}</strong><small><template v-if="image.width && image.height">{{ image.width }} × {{ image.height }} · </template>{{ formatBytes(image.sizeBytes) }}</small></div>
            <div class="capture-item-actions"><button class="icon-button" type="button" title="上移" :aria-label="`将第 ${index + 1} 张截图上移`" :disabled="queueLocked || index === 0" @click="moveImage(index, -1)"><ArrowUp :size="16" /></button><button class="icon-button" type="button" title="下移" :aria-label="`将第 ${index + 1} 张截图下移`" :disabled="queueLocked || index === images.length - 1" @click="moveImage(index, 1)"><ArrowDown :size="16" /></button><button class="icon-button capture-remove-button" type="button" title="移除截图" :aria-label="`移除第 ${index + 1} 张截图`" :disabled="queueLocked" @click="removeImage(index)"><X :size="16" /></button></div>
          </li>
        </ol>
        <div v-else class="capture-empty-queue"><FileImage :size="18" /><span>尚未添加截图</span></div>
      </section>

      <section v-show="step === 2" class="capture-details-panel" aria-labelledby="capture-details-heading">
        <div class="capture-panel-heading"><h3 id="capture-details-heading">补充文章信息</h3><span class="capture-count">{{ images.length }} 张待识别</span></div>
        <div class="capture-source-preview" aria-label="本次识别截图"><img v-for="(image, index) in images.slice(0, 5)" :key="image.id" :src="image.previewUrl" :alt="`待识别截图 ${index + 1}`" /><span v-if="images.length > 5">+{{ images.length - 5 }}</span></div>
        <form id="capture-details-form" class="capture-form" novalidate @submit.prevent="recognizeAndEdit">
          <label class="capture-field"><span>资料标题 <b>必填</b></span><input ref="titleInput" v-model="title" :disabled="queueLocked" type="text" maxlength="120" placeholder="例如：AI 创作工具合集" autocomplete="off" /></label>
          <label class="capture-field"><span>来源链接 <em>可选</em></span><span class="capture-input-with-icon"><Link2 :size="16" aria-hidden="true" /><input v-model="sourceUrl" :disabled="queueLocked" type="url" placeholder="https://..." autocomplete="url" /></span></label>
        </form>
        <div v-if="ocrRunning" class="capture-recognizing" role="status"><LoaderCircle :size="20" class="capture-spin" /><span>{{ saving ? "正在保存截图与文字..." : "正在识别截图文字..." }}</span></div>
      </section>

      <div class="capture-feedback-area" aria-live="polite"><p v-if="successMessage && !errorMessage" class="capture-feedback capture-feedback-success" role="status"><CheckCircle2 :size="16" /><span>{{ successMessage }}</span></p><p v-if="errorMessage" class="capture-feedback capture-feedback-error" role="alert">{{ errorMessage }}</p><button v-if="modelNeedsSetup" class="button button-secondary capture-settings-button" type="button" :disabled="queueLocked" @click="router.push('/settings')"><Settings2 :size="16" />前往模型设置</button></div>
      <footer class="capture-footer"><button v-if="step === 2" class="button button-secondary" type="button" :disabled="queueLocked" @click="goToImages"><ArrowLeft :size="16" />上一步</button><span v-else></span><button v-if="step === 1" class="button button-primary" type="button" :disabled="queueLocked || !images.length" @click="goToDetails">下一步：补充信息<ArrowRight :size="16" /></button><button v-else class="button button-primary" type="submit" form="capture-details-form" :disabled="!canRecognize"><LoaderCircle v-if="ocrRunning" :size="16" class="capture-spin" /><ScanText v-else :size="16" />{{ ocrRunning ? "正在识别..." : pendingOcrText !== null ? "下一步：编辑正文" : "下一步：识别并编辑" }}</button></footer>
    </div>
  </section>
</template>

<style scoped>
.capture-heading { align-items: center; }
.page-heading > .capture-save-actions { flex: 0 0 auto; display: flex; align-items: center; gap: 12px; }
.capture-unsaved { color: var(--warning); font-size: 12px; }
.capture-progress { display: flex; gap: 0; margin: 0 0 24px; padding: 0; color: var(--muted-strong); border-bottom: 1px solid var(--line); list-style: none; }
.capture-progress li { display: flex; flex: 1; align-items: center; justify-content: center; gap: 9px; min-width: 0; padding: 16px 8px; border-bottom: 2px solid transparent; font-size: 13px; font-weight: 650; }
.capture-progress li.active { color: var(--primary); border-bottom-color: var(--primary); }
.capture-step-number { display: grid; width: 26px; height: 26px; flex: 0 0 26px; place-items: center; color: var(--muted-strong); background: var(--surface-alt); border: 1px solid var(--line); border-radius: 50%; font-size: 12px; }
.capture-progress .active .capture-step-number { color: var(--surface); background: var(--primary); border-color: var(--primary); }
.capture-progress .complete .capture-step-number { color: var(--success); background: var(--success-soft); border-color: transparent; }
.capture-workspace { max-width: 920px; min-width: 0; margin: 0 auto; }
.capture-assets-panel, .capture-details-panel { min-width: 0; }
.capture-panel-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin: 0 0 16px; }
.capture-panel-heading h3 { margin: 0; color: var(--ink); font-size: 16px; font-weight: 700; }
.capture-count { color: var(--muted-strong); font-size: 12px; }
.capture-dropzone { display: flex; min-height: 210px; flex-direction: column; align-items: center; justify-content: center; gap: 16px; padding: 24px; color: var(--muted-strong); text-align: center; background: var(--surface-alt); border: 1px dashed var(--line-strong); border-radius: 8px; transition: border-color 160ms ease-out, background-color 160ms ease-out; }
.capture-dropzone-active { background: var(--primary-soft); border-color: var(--primary); }
.capture-upload-icon { color: var(--primary); }
.capture-dropzone strong { color: var(--ink); font-size: 15px; font-weight: 650; }
.capture-dropzone small { color: var(--muted-strong); font-size: 11px; }
.capture-add-actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; }
.capture-file-input { display: none; }
.capture-queue-heading { display: flex; align-items: center; justify-content: space-between; margin: 20px 0 8px; color: var(--muted-strong); font-size: 12px; font-weight: 650; }
.capture-clear-button { display: inline-flex; align-items: center; gap: 5px; min-height: 32px; padding: 4px 8px; color: var(--muted-strong); background: transparent; border: 0; border-radius: 6px; font-size: 12px; }
.capture-clear-button:hover { color: oklch(0.55 0.17 25); background: var(--surface-alt); }
.capture-queue { display: flex; flex-direction: column; gap: 8px; margin: 0; padding: 0; list-style: none; }
.capture-queue-item { display: grid; grid-template-columns: 24px 56px minmax(0, 1fr) auto; align-items: center; gap: 12px; min-height: 76px; padding: 8px; background: var(--surface-alt); border: 1px solid transparent; border-radius: 8px; cursor: grab; transition: border-color 160ms ease-out, opacity 160ms ease-out; }
.capture-queue-item:hover { border-color: var(--line-strong); }
.capture-queue-item-dragging { opacity: 0.45; border-color: var(--primary); }
.capture-queue-index { color: var(--muted-strong); font-size: 12px; font-variant-numeric: tabular-nums; text-align: center; }
.capture-thumb { width: 56px; height: 56px; overflow: hidden; background: var(--surface); border: 1px solid var(--line); border-radius: 6px; }
.capture-thumb img { display: block; width: 100%; height: 100%; object-fit: contain; }
.capture-image-info { min-width: 0; }
.capture-image-info strong, .capture-image-info small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.capture-image-info strong { color: var(--ink); font-size: 13px; font-weight: 650; }
.capture-image-info small { margin-top: 5px; color: var(--muted-strong); font-size: 11px; }
.capture-item-actions { display: flex; align-items: center; gap: 4px; }
.capture-item-actions .icon-button { width: 34px; height: 34px; border-radius: 6px; }
.capture-remove-button:hover { color: oklch(0.55 0.17 25); }
.capture-empty-queue { display: flex; min-height: 64px; align-items: center; justify-content: center; gap: 8px; color: var(--muted-strong); font-size: 12px; }
.capture-details-panel { max-width: 620px; margin: 0 auto; }
.capture-source-preview { display: flex; align-items: center; gap: 8px; margin-bottom: 24px; overflow: hidden; }
.capture-source-preview img { display: block; width: 64px; height: 72px; flex-shrink: 0; object-fit: contain; background: var(--surface-alt); border: 1px solid var(--line); border-radius: 6px; }
.capture-source-preview > span { color: var(--muted-strong); font-size: 12px; }
.capture-form { display: grid; gap: 24px; }
.capture-field { display: grid; gap: 8px; color: var(--ink); font-size: 13px; font-weight: 650; }
.capture-field > span:first-child { display: flex; align-items: center; gap: 8px; }
.capture-field b, .capture-field em { color: var(--muted-strong); font-size: 11px; font-style: normal; font-weight: 500; }
.capture-field input { width: 100%; min-width: 0; min-height: 44px; padding: 0 12px; color: var(--ink); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 8px; outline: 0; font-size: 14px; transition: border-color 160ms ease-out, box-shadow 160ms ease-out; }
.capture-field input::placeholder { color: var(--muted-strong); }
.capture-field input:focus { border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus); }
.capture-input-with-icon { display: flex; align-items: center; gap: 8px; min-height: 44px; padding: 0 12px; color: var(--muted-strong); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 8px; }
.capture-input-with-icon svg { flex-shrink: 0; }
.capture-input-with-icon:focus-within { border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus); }
.capture-input-with-icon input { min-height: 40px; padding: 0; border: 0; box-shadow: none; }
.capture-input-with-icon input:focus { box-shadow: none; }
.capture-recognizing { display: flex; align-items: center; gap: 10px; margin-top: 24px; color: var(--primary); font-size: 13px; }
.capture-feedback-area { margin-top: 20px; }
.capture-feedback { display: flex; align-items: start; gap: 8px; margin: 0; padding: 12px; border-radius: 8px; font-size: 13px; line-height: 1.6; overflow-wrap: anywhere; }
.capture-feedback svg { flex-shrink: 0; margin-top: 2px; }
.capture-feedback-success { color: var(--success); background: var(--success-soft); }
.capture-feedback-error { color: oklch(0.55 0.17 25); background: color-mix(in oklch, oklch(0.55 0.17 25) 9%, var(--surface)); }
.capture-settings-button { margin-top: 12px; }
.capture-footer { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 24px; padding-top: 20px; border-top: 1px solid var(--line); }
.capture-footer .button { min-height: 40px; }
.capture-view button:disabled, .capture-view input:disabled { opacity: 0.55; cursor: not-allowed; }
.capture-view button:focus-visible { outline: 2px solid var(--primary); outline-offset: 3px; }
.capture-loading { display: grid; gap: 16px; max-width: 620px; margin: 0 auto; }
.capture-loading > div { height: 44px; background: var(--surface-alt); border-radius: 8px; }
.capture-loading > div:first-child { height: 140px; }
.capture-loading > span { color: var(--muted-strong); font-size: 13px; }
.capture-load-error { display: flex; min-height: 240px; flex-direction: column; align-items: center; justify-content: center; gap: 12px; text-align: center; color: var(--muted-strong); }
.capture-load-error h3 { margin: 0; color: var(--ink); font-size: 16px; }
.capture-load-error p { max-width: 520px; margin: 0; line-height: 1.6; overflow-wrap: anywhere; font-size: 13px; }
.capture-load-error > div { display: flex; flex-wrap: wrap; gap: 8px; }
.capture-spin { animation: capture-spin 850ms linear infinite; }
@keyframes capture-spin { to { transform: rotate(360deg); } }
@media (max-width: 760px) {
  .capture-heading { align-items: flex-start; }
  .capture-save-actions { flex-wrap: wrap; justify-content: flex-end; gap: 6px; }
  .capture-progress li { gap: 6px; padding: 12px 4px; font-size: 12px; }
  .capture-queue-item { grid-template-columns: 20px 48px minmax(0, 1fr); gap: 8px; }
  .capture-thumb { width: 48px; height: 48px; }
  .capture-item-actions { grid-column: 3; justify-content: flex-end; }
  .capture-footer { flex-wrap: wrap; }
  .capture-footer .button-primary { margin-left: auto; }
}
@media (prefers-reduced-motion: reduce) { .capture-spin { animation: none; } .capture-dropzone, .capture-queue-item, .capture-field input { transition: none; } }
</style>
