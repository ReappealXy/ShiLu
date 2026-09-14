<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { ArrowLeft, Check, ImagePlus, Link2, LoaderCircle, Save, Sparkles, Upload, X } from "@lucide/vue";
import { onBeforeRouteLeave, useRoute, useRouter } from "vue-router";
import { createArticleWithSources, importArticleSources, type ImageSource } from "../services/capture";
import { hasPastedText, imageFromBlob, pastedImageFiles, readClipboardImages, type ClipboardImage } from "../services/clipboard";
import { readArticle, saveArticle, type ArticleDocument } from "../services/editor";
import { getErrorMessage } from "../services/library";
import MarkdownEditor from "../components/MarkdownEditor.vue";
import { getModelSettings, polishWithModel } from "../services/model";
import { localImageReferences, renderArticleMarkdown } from "../services/markdown";
import { displayPath } from "../services/paths";

type ImageItem = { id: string; source: ImageSource; previewUrl: string; relativePath?: string; name: string };
const route = useRoute(); const router = useRouter();
const title = ref(""); const sourceUrl = ref(""); const content = ref("");
const images = ref<ImageItem[]>([]); const folderName = ref(""); const article = ref<ArticleDocument | null>(null);
const loading = ref(false); const saving = ref(false); const adding = ref(false); const errorMessage = ref(""); const successMessage = ref("");
const polishRunning = ref(false); const polishText = ref(""); const polishBase = ref(""); const polishError = ref("");
const fileInput = ref<HTMLInputElement>(); let seq = 0; let disposed = false; let stopDrag: (() => void) | undefined;
const hasContent = computed(() => Boolean(title.value.trim() || sourceUrl.value.trim() || content.value.trim() || images.value.length));
const canSave = computed(() => !loading.value && !saving.value && !adding.value && Boolean(title.value.trim()) && Boolean(content.value.trim()));
const markdownPath = computed(() => article.value?.markdownPath ?? "");
const polishStale = computed(() => Boolean(polishText.value) && content.value !== polishBase.value);
const polishPreview = computed(() => renderArticleMarkdown(polishText.value, markdownPath.value));

function preview(source: ImageSource): string { return source.kind === "clipboard" ? source.base64 : (isTauri() ? convertFileSrc(displayPath(source.path)) : source.path); }
function addItems(items: ClipboardImage[]) { for (const item of items) images.value.push({ id: `${Date.now()}-${seq++}`, source: item.source, previewUrl: item.previewUrl, name: item.source.name }); }
async function addSources(sources: ImageSource[]) { if (!sources.length) return; adding.value = true; errorMessage.value = ""; try { for (const source of sources) images.value.push({ id: `${Date.now()}-${seq++}`, source, previewUrl: preview(source), name: source.kind === "path" ? source.path.split(/[\\/]/).pop() || "图片" : source.name }); successMessage.value = `已添加 ${sources.length} 张配图。`; } catch (e) { errorMessage.value = getErrorMessage(e, "图片添加失败。"); } finally { adding.value = false; } }
async function pasteImages() { adding.value = true; errorMessage.value = ""; try { addItems(await readClipboardImages()); successMessage.value = "已添加剪贴板配图。"; } catch (e) { errorMessage.value = getErrorMessage(e, "无法粘贴图片。"); } finally { adding.value = false; } }
function onPaste(event: ClipboardEvent) { const files = pastedImageFiles(event); if (files.length) { event.preventDefault(); adding.value = true; Promise.all(files.map(imageFromBlob)).then(addItems).catch(e => errorMessage.value = getErrorMessage(e, "无法粘贴图片。" )).finally(() => { adding.value = false; }); } else if (!hasPastedText(event) && isTauri()) { event.preventDefault(); void pasteImages(); } }
async function chooseImages() { if (!isTauri()) { fileInput.value?.click(); return; } try { const selected = await open({ title: "选择正文配图", multiple: true, filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp"] }] }); const paths = selected ? (Array.isArray(selected) ? selected : [selected]) : []; await addSources(paths.map(path => ({ kind: "path" as const, path }))); } catch (e) { errorMessage.value = getErrorMessage(e, "无法选择图片。"); } }
function onFiles(event: Event) { const files = Array.from((event.target as HTMLInputElement).files ?? []); if (files.length) Promise.all(files.map(imageFromBlob)).then(addItems); (event.target as HTMLInputElement).value = ""; }
function removeImage(index: number) { images.value.splice(index, 1); }
async function runPolish() {
  if (polishRunning.value || !content.value.trim()) return;
  polishRunning.value = true;
  polishError.value = "";
  polishText.value = "";
  polishBase.value = content.value;
  try {
    const settings = await getModelSettings();
    if (!settings.polishModel?.enabled) throw new Error("请先在设置中配置并启用 Markdown 整理模型。");
    const result = await polishWithModel(settings.polishModel, polishBase.value);
    if (disposed) return;
    if (!result.trim()) throw new Error("模型没有返回文字，请重新测试整理模型。");
    polishText.value = result;
    successMessage.value = "整理完成，请查看结果后决定是否采用。";
  } catch (e) {
    if (!disposed) polishError.value = getErrorMessage(e, "AI 整理失败，正文未被修改。");
  } finally { polishRunning.value = false; }
}
function adoptPolish() {
  if (!polishText.value || polishStale.value) return;
  const expected = localImageReferences(content.value);
  const actual = localImageReferences(polishText.value);
  if (expected.some(path => !actual.includes(path))) {
    polishError.value = "整理结果缺少原文配图引用。请补回对应图片后再采用，原正文未被修改。";
    return;
  }
  content.value = polishText.value;
  polishText.value = "";
  polishError.value = "";
  successMessage.value = "已采用 AI 整理结果。";
}
async function save(show = true) {
  if (!canSave.value) { errorMessage.value = "请填写标题和正文后再保存。"; return false; }
  saving.value = true; errorMessage.value = "";
  try {
    if (!folderName.value) { const result = await createArticleWithSources(title.value.trim(), sourceUrl.value.trim(), images.value.map(i => i.source)); folderName.value = result.article.folderName; images.value.forEach((i, idx) => { const imported = result.images.images[idx]; if (imported) { i.relativePath = imported.relativePath; i.source = { kind: "path", path: imported.path }; } }); }
    const pending = images.value.filter(i => !i.relativePath); if (pending.length) { const result = await importArticleSources(folderName.value, pending.map(i => i.source)); pending.forEach((i, idx) => { const imported = result.images[idx]; if (imported) { i.relativePath = imported.relativePath; i.source = { kind: "path", path: imported.path }; } }); }
    const imageRefs = images.value.map(i => i.relativePath ? `![配图](${i.relativePath})` : "").filter(Boolean);
    const body = content.value.trim();
    const missingRefs = imageRefs.filter(ref => !body.includes(ref));
    article.value = await saveArticle(folderName.value, { title: title.value.trim(), sourceUrl: sourceUrl.value.trim(), content: [body, ...missingRefs].filter(Boolean).join("\n\n"), status: "active", captureStep: 3, sourceImages: [] });
    if (show) successMessage.value = "资料已保存到资料库。"; return true;
  } catch (e) { errorMessage.value = getErrorMessage(e, "保存失败，请重试。"); return false; } finally { saving.value = false; }
}
async function loadDraft() { const refId = typeof route.query.draft === "string" ? route.query.draft : ""; if (!refId) return; loading.value = true; try { const doc = await readArticle(refId); article.value = doc; folderName.value = doc.folderName; title.value = doc.title; sourceUrl.value = doc.sourceUrl; content.value = doc.content; images.value = (doc.sourceImages ?? []).map((p, i) => ({ id: `${i}`, source: { kind: "path", path: `${doc.markdownPath.replace(/[\\/][^\\/]+$/, "")}/${p}` }, previewUrl: isTauri() ? convertFileSrc(displayPath(`${doc.markdownPath.replace(/[\\/][^\\/]+$/, "")}/${p}`)) : "", relativePath: p, name: p.split(/[\\/]/).pop() || "图片" })); } catch (e) { errorMessage.value = getErrorMessage(e, "无法读取资料。"); } finally { loading.value = false; } }
async function leave() {
  if (polishRunning.value || polishText.value) {
    polishError.value = "请先等待 AI 整理完成，并采用或放弃整理结果后再离开。";
    return false;
  }
  if (hasContent.value && !saving.value) await save(false);
  return true;
}
function onDrop(event: DragEvent) { event.preventDefault(); const files = Array.from(event.dataTransfer?.files ?? []).filter(f => f.type.startsWith("image/")); if (files.length) Promise.all(files.map(imageFromBlob)).then(addItems); }
onBeforeRouteLeave(leave); watch(() => route.query.draft, loadDraft); onMounted(() => { window.addEventListener("paste", onPaste); void loadDraft(); if (isTauri()) getCurrentWebview().onDragDropEvent(e => { if (e.payload.type === "drop" && e.payload.paths) void addSources(e.payload.paths.map(path => ({ kind: "path" as const, path }))); }).then(stop => { if (!disposed) stopDrag = stop; else stop(); }); }); onBeforeUnmount(() => { disposed = true; window.removeEventListener("paste", onPaste); stopDrag?.(); });
</script>

<template>
  <section class="page-view capture-view"><div class="page-heading"><div><button class="editor-back" type="button" @click="router.push('/library')"><ArrowLeft :size="16" />返回资料库</button><h2>{{ folderName ? '编辑资料' : '新建资料' }}</h2></div><button class="button button-primary" :disabled="!canSave" @click="save()"><LoaderCircle v-if="saving" class="spin" :size="16" /><Save v-else :size="16" />{{ saving ? '保存中…' : '保存资料' }}</button></div>
    <div v-if="loading" class="capture-loading">正在读取资料…</div><form v-else class="capture-form" @submit.prevent="save()"><label>标题 <input v-model="title" required maxlength="120" placeholder="请输入资料标题" /></label><label>来源链接 <span class="input-icon"><Link2 :size="15" /><input v-model="sourceUrl" type="url" placeholder="https://（可选）" /></span></label><div class="capture-body"><div class="capture-body-heading"><h3>正文（支持 Markdown）</h3><button class="button button-secondary" type="button" :disabled="polishRunning || !content.trim() || saving || adding" @click="runPolish"><LoaderCircle v-if="polishRunning" class="spin" :size="16" /><Sparkles v-else :size="16" />{{ polishRunning ? '正在整理...' : 'AI 整理为 Markdown' }}</button></div><MarkdownEditor v-model="content" :markdown-path="markdownPath" :preview-title="title" :preview-source-url="sourceUrl" :disabled="saving || adding" @paste-image="onPaste" @paste-images="pasteImages" @choose-images="chooseImages" /></div><section class="image-panel"><div class="image-panel-head"><strong>正文配图</strong><span>{{ images.length }} 张</span></div><div class="dropzone" @dragover.prevent @drop="onDrop"><Upload :size="22" /><span>将图片拖入此处，或</span><button class="button button-secondary" type="button" :disabled="adding" @click="chooseImages"><ImagePlus :size="16" />选择图片</button><button class="button button-secondary" type="button" :disabled="adding" @click="pasteImages">粘贴图片</button><small>图片会保存到本资料文件夹的 images 目录</small></div><div v-if="images.length" class="image-grid"><figure v-for="(image, i) in images" :key="image.id"><img :src="image.previewUrl" :alt="image.name" /><figcaption>{{ image.name }} <button type="button" @click="removeImage(i)" aria-label="移除图片"><X :size="14" /></button></figcaption></figure></div></section></form><section v-if="polishRunning || polishText || polishError" class="polish-review"><div class="capture-body-heading"><h3>整理结果</h3><button v-if="!polishRunning" class="icon-button" type="button" title="放弃整理结果" aria-label="放弃整理结果" @click="polishText = ''; polishError = ''"><X :size="17" /></button></div><p v-if="polishRunning" role="status">模型正在处理，原正文保持不变。</p><p v-if="polishError" class="feedback error" role="alert">{{ polishError }}</p><p v-if="polishStale" class="feedback warning" role="status">正文在本次整理后已有修改，请重新整理，以免覆盖新内容。</p><template v-if="polishText"><div class="polish-columns"><textarea v-model="polishText" aria-label="整理结果" rows="9" /><article class="markdown-body" v-html="polishPreview" /></div><div class="polish-actions"><button class="button button-secondary" type="button" @click="polishText = ''; polishError = ''">放弃</button><button class="button button-primary" type="button" :disabled="polishStale || saving || adding" @click="adoptPolish"><Check :size="16" />采用整理结果</button></div></template></section><p v-if="successMessage" class="feedback success">{{ successMessage }}</p><p v-if="errorMessage" class="feedback error">{{ errorMessage }}</p>
  </section>
</template>

<style scoped>
.capture-form{display:grid;gap:20px;max-width:980px}.capture-form>label{display:grid;gap:8px;color:var(--muted-strong);font-size:13px;font-weight:600}.capture-form input{width:100%;padding:11px 12px;color:var(--ink);background:var(--surface);border:1px solid var(--line-strong);border-radius:8px;font:inherit;font-weight:400}.input-icon{display:flex;align-items:center;gap:8px;padding:0 10px;border:1px solid var(--line-strong);border-radius:8px}.input-icon input{border:0;padding-left:0}.capture-body{display:grid;gap:8px}.capture-body-heading{display:flex;align-items:center;justify-content:space-between;gap:12px}.capture-body-heading h3{margin:0;color:var(--ink);font-size:15px}.image-panel{border-top:1px solid var(--line);padding-top:16px}.image-panel-head{display:flex;justify-content:space-between;margin-bottom:10px}.dropzone{display:flex;flex-wrap:wrap;align-items:center;justify-content:center;gap:10px;padding:28px;border:1px dashed var(--line-strong);border-radius:8px;color:var(--muted-strong)}.dropzone small{width:100%;text-align:center;font-size:11px}.image-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(140px,1fr));gap:12px;margin-top:14px}.image-grid figure{margin:0;border:1px solid var(--line);border-radius:8px;overflow:hidden;background:var(--surface-alt)}.image-grid img{display:block;width:100%;height:120px;object-fit:contain}.image-grid figcaption{display:flex;align-items:center;justify-content:space-between;padding:7px;font-size:11px}.image-grid button{border:0;background:transparent;color:var(--muted-strong)}.feedback{margin-top:16px;padding:10px;border-radius:6px}.success{color:var(--success);background:var(--success-soft)}.error{color:#b42318;background:#fff1f0}.warning{color:var(--warning)}.polish-review{display:grid;gap:10px;max-width:980px;padding-top:20px;border-top:1px solid var(--line)}.polish-columns{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px}.polish-columns textarea{width:100%;min-height:220px;padding:12px;color:var(--ink);background:var(--surface);border:1px solid var(--line-strong);border-radius:6px;font:14px/1.7 Consolas,monospace;resize:vertical}.polish-columns article{min-width:0;max-height:400px;overflow:auto;padding:12px;background:var(--surface-alt)}.polish-actions{display:flex;justify-content:flex-end;gap:8px}.spin{animation:spin 1s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}
@media (max-width: 700px){.capture-body-heading{align-items:flex-start;flex-direction:column}.polish-columns{grid-template-columns:minmax(0,1fr)}}
</style>
