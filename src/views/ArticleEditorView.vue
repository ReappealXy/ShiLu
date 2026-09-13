<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { Archive, ArchiveRestore, ArrowLeft, Check, FileText, ImagePlus, LoaderCircle, Save, Sparkles, X } from "@lucide/vue";
import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { onBeforeRouteLeave, onBeforeRouteUpdate, useRoute, useRouter } from "vue-router";
import MarkdownEditor from "../components/MarkdownEditor.vue";
import { getErrorMessage } from "../services/library";
import { readArticle, saveArticle, setArticleStatus, type ArticleDocument, type ArticleStatus } from "../services/editor";
import { getModelSettings, polishWithModel } from "../services/model";
import { importArticleSources, type ImageSource } from "../services/capture";
import { hasPastedText, imageFromBlob, pastedImageFiles, readClipboardImages } from "../services/clipboard";
import { articleImageUrl, localImageReferences, renderArticleMarkdown } from "../services/markdown";
import { displayPath } from "../services/paths";

const route = useRoute();
const router = useRouter();
const articleReference = String(route.params.id ?? "");
const article = ref<ArticleDocument | null>(null);
const title = ref("");
const summary = ref("");
const sourceUrl = ref("");
const tagsText = ref("");
const content = ref("");
const notes = ref("");
const sourceImages = ref<string[]>([]);
const editor = ref<InstanceType<typeof MarkdownEditor>>();
const fileInput = ref<HTMLInputElement>();
const loading = ref(true);
const saving = ref(false);
const dirty = ref(false);
const imageBusy = ref(false);
const statusBusy = ref(false);
const leaving = ref(false);
const polishRunning = ref(false);
const polishText = ref("");
const polishBase = ref("");
const polishError = ref("");
const errorMessage = ref("");
const successMessage = ref("");
let disposed = false;
let loaded = false;
let revision = 0;
let saveTimer: ReturnType<typeof window.setTimeout> | undefined;
let saveQueue: Promise<boolean> = Promise.resolve(true);

const status = computed(() => article.value?.status ?? "active");
const statusLabel = computed(() => ({ draft: "处理中", active: "已收录", archived: "已归档" })[status.value]);
const returnPath = computed(() => status.value === "archived" ? "/archive" : "/library");
const returnLabel = computed(() => status.value === "archived" ? "返回归档箱" : "返回资料库");
const polishStale = computed(() => Boolean(polishText.value) && content.value !== polishBase.value);
const polishPreview = computed(() => renderArticleMarkdown(polishText.value, article.value?.markdownPath ?? ""));
const savedHint = computed(() => saving.value ? "正在保存..." : dirty.value ? "有未保存的修改" : status.value === "draft" ? "已保存当前进度" : "修改已保存");

async function loadArticle() {
  loading.value = true;
  loaded = false;
  errorMessage.value = "";
  try {
    const document = await readArticle(articleReference);
    if (disposed) return;
    article.value = document;
    title.value = document.title;
    summary.value = document.summary;
    sourceUrl.value = document.sourceUrl;
    tagsText.value = document.tags.join(", ");
    content.value = document.content;
    notes.value = document.notes;
    sourceImages.value = [...(document.sourceImages ?? [])];
    dirty.value = false;
    loaded = true;
    if (route.query.ocr === "success") successMessage.value = "OCR 识别完成，正文已填入编辑区。";
  } catch (error) { errorMessage.value = getErrorMessage(error, "无法读取资料，请返回列表后重试。"); }
  finally { loading.value = false; }
}

function snapshot(nextStatus?: ArticleStatus) {
  return {
    title: title.value.trim() || "未命名资料", summary: summary.value, sourceUrl: sourceUrl.value,
    tags: tagsText.value.split(/[,，]/).map(tag => tag.trim()).filter(Boolean),
    content: content.value, notes: notes.value, captureStep: 3 as const,
    sourceImages: [...sourceImages.value],
    ...(nextStatus ? { status: nextStatus } : {}),
  };
}

function persistArticle(showFeedback = true, nextStatus?: ArticleStatus): Promise<boolean> {
  if (saveTimer) window.clearTimeout(saveTimer);
  const operation = saveQueue.then(() => writeArticle(showFeedback, nextStatus));
  saveQueue = operation.catch(() => false);
  return operation;
}

async function writeArticle(showFeedback: boolean, nextStatus?: ArticleStatus): Promise<boolean> {
  if (!loaded || !article.value || disposed) return false;
  if (!dirty.value && !nextStatus) {
      if (showFeedback) successMessage.value = status.value === "draft" ? "当前进度已保存。" : "所有修改已保存。";
    return true;
  }
  const savedRevision = revision;
  const draft = snapshot(nextStatus);
  saving.value = true;
  errorMessage.value = "";
  const operation = (async () => {
    try {
      const saved = await saveArticle(articleReference, draft);
      if (disposed) return false;
      article.value = saved;
      dirty.value = revision !== savedRevision;
      if (showFeedback) successMessage.value = nextStatus === "active" ? "已保存到资料库。" : status.value === "draft" ? "当前进度已保存。" : "修改已保存。";
      return true;
    } catch (error) {
      errorMessage.value = getErrorMessage(error, "保存失败，内容仍在编辑区，请重试。");
      return false;
    } finally { saving.value = false; }
  })();
  const result = await operation;
  if (result && dirty.value && !disposed) saveTimer = window.setTimeout(() => void persistArticle(false), 700);
  return result;
}

function markEdited() {
  if (!loaded) return;
  revision++;
  dirty.value = true;
  successMessage.value = "";
  if (saveTimer) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => void persistArticle(false), 900);
}

async function publish() {
  if (statusBusy.value || imageBusy.value) return;
  if (!title.value.trim() || !content.value.trim()) { errorMessage.value = "请填写标题和正文后，再保存到资料库。"; return; }
  statusBusy.value = true;
  try { await persistArticle(true, "active"); }
  finally { statusBusy.value = false; }
}

async function changeStatus(next: ArticleStatus) {
  if (statusBusy.value || imageBusy.value || polishRunning.value) return;
  statusBusy.value = true;
  try {
    if (!(await persistArticle(false))) return;
    article.value = await setArticleStatus(articleReference, next);
    successMessage.value = next === "archived" ? "已归档，文章和图片均已保留。" : "已恢复到资料库。";
  } catch (error) { errorMessage.value = getErrorMessage(error, "状态更新失败，请重试。"); }
  finally { statusBusy.value = false; }
}

async function runPolish() {
  if (polishRunning.value || !content.value.trim()) return;
  polishRunning.value = true;
  polishError.value = "";
  polishText.value = "";
  polishBase.value = content.value;
  try {
    const settings = await getModelSettings();
    if (!settings.polishModel?.enabled) throw new Error("请先在设置中配置并启用文案润色模型。");
    const result = await polishWithModel(settings.polishModel, polishBase.value);
    if (disposed) return;
    if (!result.trim()) throw new Error("模型没有返回文字，请重新测试润色模型。");
    polishText.value = result;
    successMessage.value = "润色完成，请查看结果后决定是否采用。";
  } catch (error) { if (!disposed) polishError.value = getErrorMessage(error, "润色失败，正文未被修改。"); }
  finally { polishRunning.value = false; }
}

function adoptPolish() {
  if (!polishText.value || polishStale.value) return;
  const expected = localImageReferences(content.value);
  const actual = localImageReferences(polishText.value);
  if (expected.some(path => !actual.includes(path))) {
    polishError.value = "润色结果缺少原文配图引用。请补回对应图片后再采用，原正文未被修改。";
    return;
  }
  content.value = polishText.value;
  polishText.value = "";
  successMessage.value = "已采用润色结果。";
}

async function addImages(load: () => Promise<ImageSource[]>) {
  if (imageBusy.value || statusBusy.value || !article.value) return;
  imageBusy.value = true;
  errorMessage.value = "";
  try {
    const sources = await load();
    if (!sources.length) throw new Error("没有可添加的图片。");
    const result = await importArticleSources(articleReference, sources);
    if (disposed) return;
    await editor.value?.insert(result.images.map(image => `\n\n![配图](${image.relativePath})\n\n`).join(""));
    if (await persistArticle(false)) successMessage.value = `已插入 ${result.images.length} 张配图，未进行 OCR。`;
  } catch (error) { errorMessage.value = getErrorMessage(error, "配图添加失败，正文未被清空，请重试。"); }
  finally { imageBusy.value = false; }
}

function sourceImageUrl(relativePath: string): string {
  return article.value ? articleImageUrl(article.value.markdownPath, relativePath) : "";
}

async function insertSourceImage(relativePath: string) {
  if (imageBusy.value || statusBusy.value || leaving.value || !article.value) return;
  await editor.value?.insert(`\n\n![资料配图](${relativePath})\n\n`);
  successMessage.value = "已将源截图插入正文，保存后会在阅读页显示。";
}

function pasteImages() { void addImages(async () => (await readClipboardImages()).map(image => image.source)); }
function onPaste(event: ClipboardEvent) {
  const files = pastedImageFiles(event);
  if (files.length) {
    event.preventDefault();
    void addImages(async () => (await Promise.all(files.map(imageFromBlob))).map(image => image.source));
  } else if (!hasPastedText(event) && isTauri()) {
    event.preventDefault();
    pasteImages();
  }
}
async function chooseImages() {
  if (!isTauri()) { fileInput.value?.click(); return; }
  try {
    const paths = await open({ title: "选择正文配图", multiple: true, filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp"] }] });
    if (paths) await addImages(async () => (Array.isArray(paths) ? paths : [paths]).map(path => ({ kind: "path" as const, path })));
  } catch (error) { errorMessage.value = getErrorMessage(error, "无法选择图片。"); }
}
function onFiles(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = Array.from(input.files ?? []);
  if (files.length) void addImages(async () => (await Promise.all(files.map(imageFromBlob))).map(image => image.source));
  input.value = "";
}

async function canLeave() {
  if (imageBusy.value || statusBusy.value) { errorMessage.value = "正在保存图片或文章，请完成后再切换页面。"; return false; }
  if (polishRunning.value || polishText.value) { polishError.value = "请先等待润色完成，并采用或放弃润色结果后再离开。"; return false; }
  leaving.value = true;
  try { return !loaded || await persistArticle(false); }
  finally { leaving.value = false; }
}
function onKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") { event.preventDefault(); void persistArticle(); }
}
function onBeforeUnload(event: BeforeUnloadEvent) {
  if (dirty.value || saving.value || imageBusy.value) { event.preventDefault(); event.returnValue = ""; }
}
watch([title, summary, sourceUrl, tagsText, content, notes, sourceImages], markEdited, { flush: "sync", deep: true });
watch(status, value => window.dispatchEvent(new CustomEvent("shilu:article-status", { detail: value })));
onBeforeRouteLeave(canLeave);
onBeforeRouteUpdate(canLeave);
onMounted(() => { window.addEventListener("keydown", onKeydown); window.addEventListener("beforeunload", onBeforeUnload); void loadArticle(); });
onBeforeUnmount(() => { disposed = true; window.removeEventListener("keydown", onKeydown); window.removeEventListener("beforeunload", onBeforeUnload); if (saveTimer) window.clearTimeout(saveTimer); });
</script>

<template>
  <section class="page-view editor-view" aria-labelledby="editor-heading">
    <div class="page-heading editor-heading">
      <div>
        <button class="editor-back" type="button" @click="router.push(returnPath)"><ArrowLeft :size="16" />{{ returnLabel }}</button>
        <h2 id="editor-heading">{{ title || "编辑资料" }} <span class="article-status">{{ statusLabel }}</span></h2>
      </div>
      <div v-if="article" class="editor-actions">
        <span class="save-hint" :class="{ 'save-hint-dirty': dirty }" role="status">{{ savedHint }}</span>
        <button class="button button-secondary" type="button" :disabled="loading || saving || imageBusy || statusBusy" @click="persistArticle()"><Save :size="16" />{{ status === 'draft' ? '保存当前进度' : '保存修改' }}</button>
        <button v-if="status === 'draft'" class="button button-primary" type="button" :disabled="saving || imageBusy || statusBusy" @click="publish"><Check :size="16" />保存到资料库</button>
        <button v-else class="button button-secondary" type="button" :disabled="saving || imageBusy || statusBusy || polishRunning" @click="changeStatus(status === 'archived' ? 'active' : 'archived')"><ArchiveRestore v-if="status === 'archived'" :size="16" /><Archive v-else :size="16" />{{ status === 'archived' ? '恢复到资料库' : '归档' }}</button>
      </div>
    </div>
    <ol v-if="status === 'draft'" class="editor-steps" aria-label="新建资料进度"><li><Check :size="15" />添加截图</li><li><Check :size="15" />补充信息</li><li aria-current="step">3 编辑正文</li></ol>
    <p v-if="successMessage" class="editor-feedback feedback-success" role="status"><Check :size="16" />{{ successMessage }}</p>
    <p v-if="errorMessage" class="editor-feedback feedback-error" role="alert">{{ errorMessage }}<button v-if="dirty" type="button" @click="persistArticle()">重试保存</button></p>
    <div v-if="loading" class="editor-loading" role="status"><LoaderCircle :size="22" class="editor-spin" />正在读取资料...</div>
    <div v-else-if="!article" class="editor-loading"><FileText :size="24" /><button class="button button-secondary" type="button" @click="loadArticle">重新读取</button></div>
    <template v-else>
      <div class="editor-fields">
        <label class="editor-field"><span>标题</span><input v-model="title" aria-label="资料标题" type="text" maxlength="120" :disabled="statusBusy || leaving" /></label>
        <label class="editor-field"><span>来源链接</span><input v-model="sourceUrl" type="url" placeholder="https://" :disabled="statusBusy || leaving" /></label>
      </div>
      <section v-if="sourceImages.length" class="editor-source-images" aria-labelledby="source-images-heading">
        <div class="editor-body-heading editor-source-heading"><div><h3 id="source-images-heading">源截图</h3><p>这些图片仅用于 OCR；点击“插入正文”后才会显示在阅读页。</p></div><span>{{ sourceImages.length }} 张</span></div>
        <div class="editor-source-grid">
          <figure v-for="(path, index) in sourceImages" :key="path" class="editor-source-item">
            <img :src="sourceImageUrl(path)" :alt="`源截图 ${index + 1}`" loading="lazy" />
            <figcaption><span>源截图 {{ index + 1 }}</span><div><button class="button button-tertiary" type="button" :disabled="imageBusy || statusBusy || leaving" @click="insertSourceImage(path)"><ImagePlus :size="14" />插入正文</button></div></figcaption>
          </figure>
        </div>
      </section>
      <div class="editor-body-heading"><h3>正文</h3><button class="button button-secondary" type="button" :disabled="polishRunning || !content.trim() || imageBusy || statusBusy" @click="runPolish"><LoaderCircle v-if="polishRunning" class="editor-spin" :size="16" /><Sparkles v-else :size="16" />{{ polishRunning ? '正在润色...' : 'AI 润色' }}</button></div>
      <p v-if="imageBusy" class="editor-feedback" role="status"><LoaderCircle class="editor-spin" :size="16" />正在保存并插入配图...</p>
      <MarkdownEditor ref="editor" v-model="content" :markdown-path="article.markdownPath" :preview-title="title" :preview-source-url="sourceUrl" :disabled="imageBusy || statusBusy || leaving" @paste-image="onPaste" @paste-images="pasteImages" @choose-images="chooseImages" />
      <input ref="fileInput" type="file" accept="image/png,image/jpeg,image/webp" multiple hidden @change="onFiles" />
      <section v-if="polishRunning || polishText || polishError" class="polish-review" aria-labelledby="polish-heading">
        <div class="editor-body-heading"><h3 id="polish-heading">润色结果</h3><button v-if="!polishRunning" class="icon-button" type="button" title="放弃润色结果" aria-label="放弃润色结果" @click="polishText = ''; polishError = ''"><X :size="17" /></button></div>
        <p v-if="polishRunning" role="status">模型正在处理，原正文保持不变。</p>
        <p v-if="polishError" class="editor-feedback feedback-error" role="alert">{{ polishError }}</p>
        <p v-if="polishStale" class="editor-feedback feedback-warning" role="status">正文在本次润色后已有修改，请重新润色，以免覆盖新内容。</p>
        <template v-if="polishText">
          <div class="polish-columns"><textarea v-model="polishText" aria-label="润色结果" rows="9" /><article class="markdown-body" v-html="polishPreview" /></div>
          <div class="polish-actions"><button class="button button-secondary" type="button" @click="polishText = ''; polishError = ''">放弃</button><button class="button button-primary" type="button" :disabled="polishStale || imageBusy" @click="adoptPolish"><Check :size="16" />采用润色结果</button></div>
        </template>
      </section>
      <details class="editor-extra"><summary>摘要、标签与备注</summary><div class="editor-fields"><label class="editor-field"><span>摘要</span><textarea v-model="summary" :disabled="statusBusy || leaving" rows="2" /></label><label class="editor-field"><span>标签</span><input v-model="tagsText" :disabled="statusBusy || leaving" placeholder="AI, 写作, 工具" /></label><label class="editor-field notes-field"><span>我的备注</span><textarea v-model="notes" :disabled="statusBusy || leaving" rows="3" /></label></div></details>
      <details class="editor-extra"><summary>文件信息</summary><p class="editor-path">{{ displayPath(article.markdownPath) }}</p><p class="editor-file-meta">原始截图：{{ article.sourceImages?.length ?? 0 }} 张 · 最后修改：{{ article.updatedAt }}</p></details>
    </template>
  </section>
</template>

<style scoped>
.editor-heading { gap: 16px; margin-bottom: 20px; }
.editor-heading h2 { overflow-wrap: anywhere; font-size: 22px; line-height: 1.5; }
.editor-back { display: inline-flex; align-items: center; gap: 6px; padding: 0; margin-bottom: 12px; border: 0; color: var(--muted-strong); background: transparent; font-size: 12px; }
.article-status { display: inline-block; vertical-align: middle; padding: 3px 7px; color: var(--primary); background: var(--primary-soft); border-radius: 4px; font-size: 11px; font-weight: 600; }
.page-heading > .editor-actions { flex: 0 1 auto; display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.save-hint { width: 100%; color: var(--muted-strong); text-align: right; font-size: 12px; }
.save-hint-dirty { color: var(--warning); }
.editor-steps { display: flex; flex-wrap: wrap; gap: 22px; margin: 0 0 20px; padding: 0 0 16px; border-bottom: 1px solid var(--line); list-style: none; font-size: 12px; color: var(--muted-strong); }
.editor-steps li { display: flex; align-items: center; gap: 7px; }
.editor-steps [aria-current] { color: var(--primary); font-weight: 700; }
.editor-fields { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
.editor-field { display: grid; gap: 7px; align-content: start; font-size: 12px; color: var(--muted-strong); font-weight: 600; }
.editor-field input, .editor-field textarea, .polish-columns textarea { width: 100%; padding: 10px; color: var(--ink); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 6px; font-size: 14px; line-height: 1.65; resize: vertical; }
.editor-body-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 24px; }
.editor-body-heading h3 { margin: 0; font-size: 15px; }
.editor-source-images { margin-top: 24px; padding: 16px 0 2px; border-top: 1px solid var(--line); }
.editor-source-heading { margin-top: 0; margin-bottom: 12px; }
.editor-source-heading > div { min-width: 0; }
.editor-source-heading p { margin: 4px 0 0; color: var(--muted-strong); font-size: 11px; font-weight: 500; }
.editor-source-heading > span { color: var(--muted-strong); font-size: 12px; white-space: nowrap; }
.editor-source-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 12px; }
.editor-source-item { min-width: 0; margin: 0; overflow: hidden; background: var(--surface-alt); border: 1px solid var(--line); border-radius: 8px; }
.editor-source-item > img { display: block; width: 100%; height: 150px; object-fit: contain; background: var(--surface); }
.editor-source-item figcaption { display: grid; gap: 8px; padding: 9px; color: var(--muted-strong); font-size: 11px; }
.editor-source-item figcaption > span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.editor-source-item figcaption > div { display: flex; flex-wrap: wrap; gap: 6px; }
.button-tertiary { min-height: 30px; padding: 5px 8px; font-size: 11px; }
.editor-feedback { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin: 10px 0; padding: 10px 12px; background: var(--surface-alt); border-radius: 6px; font-size: 13px; line-height: 1.65; overflow-wrap: anywhere; }
.feedback-success { color: var(--success); }
.feedback-error { color: oklch(0.52 0.18 25); }
.feedback-warning { color: var(--warning); }
.editor-feedback button { color: inherit; background: transparent; border: 0; text-decoration: underline; }
.editor-loading { min-height: 220px; display: flex; align-items: center; justify-content: center; gap: 12px; color: var(--muted-strong); }
.editor-extra, .polish-review { padding: 18px 0; border-top: 1px solid var(--line); }
.editor-extra summary { cursor: pointer; color: var(--muted-strong); font-size: 13px; }
.editor-extra .editor-fields { margin-top: 16px; }
.notes-field { grid-column: 1 / -1; }
.polish-review .editor-body-heading { margin-top: 0; margin-bottom: 12px; }
.polish-columns { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
.polish-columns article { padding: 12px; background: var(--surface-alt); max-height: 400px; overflow: auto; }
.polish-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }
.editor-path, .editor-file-meta { font-size: 12px; color: var(--muted-strong); overflow-wrap: anywhere; }
.editor-spin { animation: editor-spin 900ms linear infinite; }
@keyframes editor-spin { to { transform: rotate(360deg); } }
@media (max-width: 900px) { .editor-fields, .polish-columns { grid-template-columns: minmax(0, 1fr); } .save-hint { text-align: left; } }
@media (prefers-reduced-motion: reduce) { .editor-spin { animation: none; } }
</style>
