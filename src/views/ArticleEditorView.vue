<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ArrowLeft, Check, Eye, FileText, LoaderCircle, ScanText, Save, SquarePen } from "@lucide/vue";
import { useRoute, useRouter } from "vue-router";
import { getErrorMessage } from "../services/library";
import { ocrArticleImages, readArticle, saveArticle, type ArticleDocument } from "../services/editor";

const route = useRoute();
const router = useRouter();
const articleReference = computed(() => String(route.params.id ?? ""));
const article = ref<ArticleDocument | null>(null);
const title = ref("");
const summary = ref("");
const sourceUrl = ref("");
const tagsText = ref("");
const content = ref("");
const notes = ref("");
const viewMode = ref<"edit" | "preview">("edit");
const loading = ref(true);
const saving = ref(false);
const dirty = ref(false);
const errorMessage = ref("");
const successMessage = ref("");
const ocrText = ref("");
const ocrRunning = ref(false);
let loaded = false;
let saveTimer: ReturnType<typeof window.setTimeout> | undefined;

const tags = computed(() => tagsText.value.split(",").map((tag) => tag.trim()).filter(Boolean));
const previewHtml = computed(() => renderMarkdown(content.value));

function escapeHtml(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

function renderInline(value: string): string {
  return escapeHtml(value)
    .replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2" target="_blank" rel="noreferrer">$1</a>')
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/`([^`]+)`/g, "<code>$1</code>");
}

function renderMarkdown(markdown: string): string {
  const lines = markdown.split(/\r?\n/);
  const output: string[] = [];
  let inList = false;
  for (const line of lines) {
    if (/^[-*] /.test(line)) {
      if (!inList) { output.push("<ul>"); inList = true; }
      output.push(`<li>${renderInline(line.slice(2))}</li>`);
      continue;
    }
    if (inList) { output.push("</ul>"); inList = false; }
    if (line.startsWith("### ")) output.push(`<h3>${renderInline(line.slice(4))}</h3>`);
    else if (line.startsWith("## ")) output.push(`<h2>${renderInline(line.slice(3))}</h2>`);
    else if (line.startsWith("# ")) output.push(`<h1>${renderInline(line.slice(2))}</h1>`);
    else if (!line.trim()) output.push("<div class=\"preview-space\"></div>");
    else output.push(`<p>${renderInline(line)}</p>`);
  }
  if (inList) output.push("</ul>");
  return output.join("");
}

function fillDocument(document: ArticleDocument) {
  article.value = document;
  title.value = document.title;
  summary.value = document.summary;
  sourceUrl.value = document.sourceUrl;
  tagsText.value = document.tags.join(", ");
  content.value = document.content;
  notes.value = document.notes;
  dirty.value = false;
}

async function loadArticle() {
  loading.value = true;
  errorMessage.value = "";
  try {
    fillDocument(await readArticle(articleReference.value));
    loaded = true;
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "无法读取这条资料，请返回资料库后重试。");
  } finally {
    loading.value = false;
  }
}

async function persistArticle(showFeedback = true) {
  if (!loaded || !article.value || saving.value) return;
  if (!title.value.trim()) {
    errorMessage.value = "标题不能为空。";
    return;
  }
  saving.value = true;
  errorMessage.value = "";
  try {
    const saved = await saveArticle(articleReference.value, {
      title: title.value,
      summary: summary.value,
      sourceUrl: sourceUrl.value,
      tags: tags.value,
      content: content.value,
      notes: notes.value,
    });
    article.value = saved;
    dirty.value = false;
    if (showFeedback) {
      successMessage.value = "已保存";
      window.setTimeout(() => { successMessage.value = ""; }, 1800);
    }
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "保存失败，原内容仍保留在编辑器中。");
  } finally {
    saving.value = false;
  }
}

async function runOcr() {
  ocrRunning.value = true;
  errorMessage.value = "";
  try {
    const result = await ocrArticleImages(articleReference.value);
    ocrText.value = result.text;
    successMessage.value = `已读取 ${result.imageCount} 张图片的文字`;
    window.setTimeout(() => { successMessage.value = ""; }, 2200);
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "本地 OCR 失败，请确认资料中有图片。");
  } finally { ocrRunning.value = false; }
}

function insertOcrText() {
  if (!ocrText.value) return;
  content.value = content.value.trim() ? `${content.value.trim()}\n\n${ocrText.value}` : ocrText.value;
  dirty.value = true;
}

function scheduleAutoSave() {
  if (!loaded) return;
  dirty.value = true;
  if (saveTimer) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => persistArticle(false), 1000);
}

function onKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
    event.preventDefault();
    void persistArticle();
  }
}

watch([title, summary, sourceUrl, tagsText, content, notes], scheduleAutoSave);
onMounted(() => { window.addEventListener("keydown", onKeydown); void loadArticle(); });
onBeforeUnmount(() => { window.removeEventListener("keydown", onKeydown); if (saveTimer) window.clearTimeout(saveTimer); });
</script>

<template>
  <section class="page-view editor-view" aria-labelledby="editor-heading">
    <div class="page-heading editor-heading">
      <div>
        <button class="editor-back" type="button" @click="router.push('/library')"><ArrowLeft :size="16" />返回资料库</button>
        <p class="page-eyebrow">ARTICLE EDITOR</p>
        <h2 id="editor-heading">{{ article?.title || "编辑资料" }}</h2>
        <p class="page-description">编辑后的内容会保存回这条资料的 Markdown 文件，原图仍在同一文件夹内。</p>
      </div>
      <div class="editor-actions"><span v-if="dirty" class="editor-dirty">未保存</span><span v-if="successMessage" class="editor-saved"><Check :size="15" />{{ successMessage }}</span><button class="button button-primary" type="button" :disabled="loading || saving || !dirty" @click="persistArticle()"><LoaderCircle v-if="saving" :size="16" class="editor-spin" /><Save v-else :size="16" />{{ saving ? "保存中..." : "保存" }}</button></div>
    </div>

    <div v-if="loading" class="content-panel editor-loading"><LoaderCircle :size="22" class="editor-spin" /><span>正在读取资料...</span></div>
    <div v-else-if="errorMessage && !article" class="content-panel editor-error"><FileText :size="22" /><h3>暂时无法打开资料</h3><p>{{ errorMessage }}</p><button class="button button-secondary" type="button" @click="loadArticle">重新读取</button></div>
    <div v-else class="editor-layout">
      <section class="content-panel editor-main" aria-label="文章内容编辑器">
        <div class="editor-field-grid"><label class="editor-field editor-title-field"><span>标题</span><input v-model="title" type="text" maxlength="120" /></label><label class="editor-field"><span>来源链接</span><input v-model="sourceUrl" type="url" placeholder="https://..." /></label><label class="editor-field editor-summary-field"><span>摘要</span><textarea v-model="summary" rows="2" placeholder="用一句话概括这条资料"></textarea></label><label class="editor-field"><span>标签</span><input v-model="tagsText" type="text" placeholder="AI, 写作, 工具" /></label></div>
        <div class="editor-tabbar"><div class="editor-tabs"><button type="button" :class="{ active: viewMode === 'edit' }" @click="viewMode = 'edit'"><SquarePen :size="15" />编辑 Markdown</button><button type="button" :class="{ active: viewMode === 'preview' }" @click="viewMode = 'preview'"><Eye :size="15" />预览</button></div><span>Ctrl + S 保存</span></div>
        <textarea v-if="viewMode === 'edit'" v-model="content" class="editor-content" spellcheck="false" placeholder="在这里写下正文..." aria-label="Markdown 正文"></textarea><article v-else class="editor-preview" v-html="previewHtml"></article>
        <label class="editor-field editor-notes-field"><span>我的备注</span><textarea v-model="notes" rows="4" placeholder="写下你的补充、待办或思考"></textarea></label>
        <p v-if="errorMessage" class="editor-feedback editor-feedback-error" role="alert">{{ errorMessage }}</p>
      </section>
      <aside class="content-panel editor-meta"><p class="panel-kicker">DOCUMENT</p><h3>资料信息</h3><dl><div><dt>文件夹</dt><dd>{{ article?.folderName }}</dd></div><div><dt>创建时间</dt><dd>{{ article?.createdAt }}</dd></div><div><dt>最后修改</dt><dd>{{ article?.updatedAt }}</dd></div></dl><p class="editor-path">{{ article?.markdownPath }}</p><div class="editor-ocr"><div class="editor-ocr-heading"><span>本地 OCR</span><button class="icon-button" type="button" title="识别图片文字" aria-label="识别图片文字" :disabled="ocrRunning" @click="runOcr"><LoaderCircle v-if="ocrRunning" :size="15" class="editor-spin" /><ScanText v-else :size="15" /></button></div><p v-if="!ocrText">识别结果会保存在 raw/ocr.txt，不会自动覆盖正文。</p><pre v-else>{{ ocrText }}</pre><button v-if="ocrText" class="button button-secondary editor-ocr-insert" type="button" @click="insertOcrText">插入正文</button></div></aside>
    </div>
  </section>
</template>

<style scoped>
.editor-heading { align-items: end; }
.editor-heading h2 { max-width: 650px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.editor-back { display: inline-flex; align-items: center; gap: 5px; margin: 0 0 18px; padding: 0; color: var(--muted-strong); background: transparent; border: 0; font-size: 12px; font-weight: 650; }
.editor-back:hover { color: var(--primary); }
.editor-actions { display: flex; align-items: center; gap: 11px; padding-bottom: 3px; }
.editor-dirty { color: var(--warning); font-size: 12px; font-weight: 680; }
.editor-saved { display: inline-flex; align-items: center; gap: 4px; color: var(--success); font-size: 12px; }
.editor-layout { display: grid; grid-template-columns: minmax(0, 1fr) 250px; align-items: start; gap: 14px; }
.editor-main { min-width: 0; padding: 22px; }
.editor-field-grid { display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr); gap: 15px; }
.editor-field { display: grid; gap: 7px; color: var(--muted-strong); font-size: 12px; font-weight: 680; }
.editor-field span { display: block; }
.editor-title-field { grid-column: 1; }
.editor-summary-field { grid-column: 1 / -1; }
.editor-field input, .editor-field textarea { width: 100%; padding: 9px 10px; color: var(--ink); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 8px; outline: 0; font-size: 13px; line-height: 1.55; resize: vertical; transition: border-color 130ms ease-out, box-shadow 130ms ease-out; }
.editor-field input { min-height: 39px; }
.editor-field input:focus, .editor-field textarea:focus { border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus); }
.editor-tabbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 22px; padding-bottom: 9px; border-bottom: 1px solid var(--line); }
.editor-tabs { display: flex; gap: 4px; }
.editor-tabs button { display: inline-flex; align-items: center; gap: 6px; padding: 7px 9px; color: var(--muted); background: transparent; border: 0; border-radius: 6px; font-size: 12px; font-weight: 680; }
.editor-tabs button:hover { color: var(--ink); background: var(--surface-alt); }
.editor-tabs button.active { color: var(--primary); background: var(--primary-soft); }
.editor-tabbar > span { color: var(--muted); font-size: 10px; }
.editor-content { display: block; width: 100%; min-height: 390px; margin-top: 15px; padding: 14px; color: var(--ink); background: var(--surface-alt); border: 1px solid var(--line); border-radius: 8px; outline: 0; font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 13px; line-height: 1.7; resize: vertical; }
.editor-content:focus { border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus); }
.editor-preview { min-height: 390px; margin-top: 15px; padding: 14px; color: var(--ink); background: var(--surface-alt); border: 1px solid var(--line); border-radius: 8px; font-size: 14px; line-height: 1.75; }
.editor-preview :deep(h1), .editor-preview :deep(h2), .editor-preview :deep(h3) { margin: 0 0 10px; line-height: 1.35; }
.editor-preview :deep(p) { margin: 0 0 11px; }
.editor-preview :deep(ul) { margin: 0 0 12px; padding-left: 22px; }
.editor-preview :deep(code) { padding: 2px 4px; background: var(--surface); border-radius: 4px; font-family: ui-monospace, monospace; font-size: 12px; }
.editor-preview :deep(a) { color: var(--primary); }
.preview-space { height: 8px; }
.editor-notes-field { margin-top: 18px; }
.editor-meta { padding: 22px; }
.editor-meta h3 { margin: 0; color: var(--ink); font-size: 15px; }
.editor-meta dl { display: grid; gap: 15px; margin: 22px 0 0; }
.editor-meta dl div { min-width: 0; }
.editor-meta dt { color: var(--muted); font-size: 10px; font-weight: 680; }
.editor-meta dd { margin: 4px 0 0; overflow-wrap: anywhere; color: var(--ink); font-size: 12px; line-height: 1.45; }
.editor-path { margin: 22px 0 0; padding-top: 14px; overflow-wrap: anywhere; color: var(--muted); border-top: 1px solid var(--line); font-size: 10px; line-height: 1.5; }
.editor-ocr { margin-top: 22px; padding-top: 15px; border-top: 1px solid var(--line); }
.editor-ocr-heading { display: flex; align-items: center; justify-content: space-between; color: var(--ink); font-size: 12px; font-weight: 700; }
.editor-ocr-heading .icon-button { width: 28px; height: 28px; }
.editor-ocr p { margin: 8px 0 0; color: var(--muted); font-size: 11px; line-height: 1.55; }
.editor-ocr pre { max-height: 180px; margin: 9px 0 0; padding: 9px; overflow: auto; color: var(--ink); background: var(--surface-alt); border-radius: 7px; font-size: 10px; line-height: 1.55; white-space: pre-wrap; }
.editor-ocr-insert { width: 100%; min-height: 31px; margin-top: 9px; padding: 0 8px; font-size: 11px; }
.editor-loading, .editor-error { display: flex; min-height: 240px; flex-direction: column; align-items: center; justify-content: center; gap: 10px; color: var(--muted); text-align: center; }
.editor-error h3 { margin: 2px 0 0; color: var(--ink); font-size: 16px; }
.editor-error p { max-width: 450px; margin: 0; font-size: 13px; line-height: 1.6; }
.editor-feedback { margin: 14px 0 0; padding: 10px 11px; border-radius: 8px; font-size: 12px; }
.editor-feedback-error { color: oklch(0.55 0.17 25); background: color-mix(in oklch, oklch(0.55 0.17 25) 9%, var(--surface)); }
.editor-spin { animation: editor-spin 850ms linear infinite; }
@keyframes editor-spin { to { transform: rotate(360deg); } }
@media (max-width: 1180px) { .editor-layout { grid-template-columns: minmax(0, 1fr) 220px; } }
@media (prefers-reduced-motion: reduce) { .editor-spin { animation: none; } }
</style>
