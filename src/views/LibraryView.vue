<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { Archive, ArchiveRestore, Check, LibraryBig, LoaderCircle, Plus, RefreshCw, Search, Settings, Trash2 } from "@lucide/vue";
import { getErrorMessage, getLibraryStatus, type LibraryStatus } from "../services/library";
import { deleteArticlePermanently, listArticles, setArticleStatus, type ArticleStatus, type ArticleSummary } from "../services/editor";

const route = useRoute();
const status = computed<ArticleStatus>(() => route.meta.articleStatus === "archived" ? "archived" : "active");
const heading = computed(() => status.value === "archived" ? "归档箱" : "资料库");
const libraryStatus = ref<LibraryStatus | null>(null);
const articles = ref<ArticleSummary[]>([]);
const query = ref("");
const loadingStatus = ref(true);
const loadingArticles = ref(false);
const errorMessage = ref("");
const successMessage = ref("");
const pending = ref(new Set<string>());
let searchTimer: ReturnType<typeof window.setTimeout> | undefined;
let requestId = 0;
let disposed = false;

async function loadArticles() {
  if (!libraryStatus.value?.ready) return;
  const id = ++requestId;
  loadingArticles.value = true;
  errorMessage.value = "";
  try {
    const result = await listArticles(query.value, status.value);
    if (!disposed && id === requestId) articles.value = result;
  } catch (error) { if (!disposed && id === requestId) errorMessage.value = getErrorMessage(error, "无法读取资料列表，请重试。"); }
  finally { if (!disposed && id === requestId) loadingArticles.value = false; }
}
async function loadLibraryStatus() {
  loadingStatus.value = true;
  errorMessage.value = "";
  try { libraryStatus.value = await getLibraryStatus(); await loadArticles(); }
  catch (error) { errorMessage.value = getErrorMessage(error, "无法读取资料库，请检查保存位置。"); }
  finally { loadingStatus.value = false; }
}
async function changeStatus(article: ArticleSummary) {
  if (pending.value.has(article.folderName)) return;
  pending.value = new Set(pending.value).add(article.folderName);
  errorMessage.value = "";
  successMessage.value = "";
  try {
    const next = status.value === "archived" ? "active" : "archived";
    await setArticleStatus(article.folderName, next);
    if (disposed) return;
    successMessage.value = next === "archived" ? `“${article.title}”已归档，可在归档箱恢复。` : `“${article.title}”已恢复到资料库。`;
    await loadArticles();
  } catch (error) { if (!disposed) errorMessage.value = getErrorMessage(error, "操作失败，资料保留在原位置，请重试。"); }
  finally {
    const nextPending = new Set(pending.value);
    nextPending.delete(article.folderName);
    pending.value = nextPending;
  }
}
async function deleteArticle(article: ArticleSummary) {
  if (pending.value.has(article.folderName)) return;
  const title = article.title || "未命名资料";
  if (!window.confirm(`确定永久删除“${title}”吗？\nMarkdown 文件夹及其中的图片都会被删除，且无法恢复。`)) return;

  pending.value = new Set(pending.value).add(article.folderName);
  errorMessage.value = "";
  successMessage.value = "";
  try {
    await deleteArticlePermanently(article.folderName);
    if (disposed) return;
    successMessage.value = `“${title}”已永久删除，本地文件和图片已清理。`;
    window.dispatchEvent(new CustomEvent("shilu:article-deleted", { detail: article.folderName }));
    await loadArticles();
  } catch (error) {
    if (!disposed) errorMessage.value = getErrorMessage(error, "永久删除失败，资料仍然保留。请重试。");
  } finally {
    const nextPending = new Set(pending.value);
    nextPending.delete(article.folderName);
    pending.value = nextPending;
  }
}
function articleLink(article: ArticleSummary) {
  return { path: `/articles/${article.folderName}`, query: { from: status.value === "archived" ? "archive" : "library" } };
}
function formatDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "未记录时间" : date.toLocaleString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}
/**
 * 列表片段使用正文（后端可选返回 content）。
 * 这里仅做纯文本截取，避免把 Markdown 原始标记直接铺在列表中。
 */
function articleExcerpt(article: ArticleSummary) {
  const source = article.content || "";
  const plain = source
    .replace(/!\[[^\]]*\]\([^)]*\)/g, "")
    .replace(/\[[^\]]*\]\([^)]*\)/g, "")
    .replace(/[#>*_`~-]/g, "")
    .replace(/\s+/g, " ")
    .trim();
  return plain ? (plain.length > 140 ? `${plain.slice(0, 140)}…` : plain) : "暂无正文内容";
}
watch(query, () => {
  ++requestId;
  if (searchTimer) window.clearTimeout(searchTimer);
  searchTimer = window.setTimeout(() => void loadArticles(), 220);
});
onMounted(loadLibraryStatus);
onBeforeUnmount(() => { disposed = true; ++requestId; if (searchTimer) window.clearTimeout(searchTimer); });
</script>

<template>
  <section class="page-view library-view" aria-labelledby="library-heading">
    <div class="page-heading library-heading"><div><h2 id="library-heading">{{ heading }}</h2></div><RouterLink class="button button-primary" to="/capture"><Plus :size="16" />新建资料</RouterLink></div>
    <div v-if="loadingStatus" class="collection-empty" role="status"><LoaderCircle :size="24" class="collection-spin" />正在读取{{ heading }}...</div>
    <div v-else-if="!libraryStatus?.ready" class="collection-empty"><LibraryBig :size="30" /><h3>{{ libraryStatus?.configured ? '资料库暂时不可用' : '尚未设置资料库' }}</h3><p v-if="errorMessage" role="alert">{{ errorMessage }}</p><RouterLink class="button button-primary" to="/settings"><Settings :size="16" />设置资料库位置</RouterLink><button class="button button-secondary" type="button" @click="loadLibraryStatus"><RefreshCw :size="16" />重新读取</button></div>
    <template v-else>
      <div class="library-toolbar"><label class="library-search"><Search :size="17" /><input v-model="query" type="search" :aria-label="`搜索${heading}`" placeholder="搜索标题、正文或链接" /></label><span class="library-result-count" role="status">{{ loadingArticles ? '正在查找...' : `${articles.length} 条资料` }}</span><button class="icon-button" type="button" title="刷新列表" aria-label="刷新列表" :disabled="loadingArticles" @click="loadArticles"><RefreshCw :size="17" /></button></div>
      <p v-if="successMessage" class="collection-feedback collection-success" role="status"><Check :size="16" />{{ successMessage }}</p>
      <p v-if="errorMessage" class="collection-feedback collection-error" role="alert">{{ errorMessage }}<button type="button" @click="loadArticles">重新读取</button></p>
      <div v-if="!articles.length && !loadingArticles && !errorMessage" class="collection-empty"><Archive v-if="status === 'archived'" :size="30" /><Search v-else :size="30" /><h3>{{ query ? '没有匹配的资料' : status === 'archived' ? '暂时没有归档资料' : '还没有保存的资料' }}</h3><button v-if="query" class="button button-secondary" type="button" @click="query = ''">清除搜索</button><RouterLink v-else-if="status !== 'archived'" class="button button-secondary" to="/capture"><Plus :size="16" />新建资料</RouterLink></div>
      <div class="article-list" :aria-label="`${heading}列表`" :aria-busy="loadingArticles">
        <article v-for="article in articles" :key="article.folderName" class="article-row">
          <RouterLink class="article-open" :to="articleLink(article)"><span class="article-row-icon"><Archive v-if="status === 'archived'" :size="20" /><LibraryBig v-else :size="20" /></span><span class="article-row-main"><strong>{{ article.title }}</strong><small>{{ articleExcerpt(article) }}</small><small v-if="article.sourceUrl" class="article-row-source">{{ article.sourceUrl }}</small></span><time class="article-row-date">{{ formatDate(article.updatedAt) }}</time></RouterLink>
          <div class="article-row-actions">
            <button class="icon-button article-status-action" type="button" :title="status === 'archived' ? '恢复到资料库' : '归档'" :aria-label="`${status === 'archived' ? '恢复' : '归档'}：${article.title}`" :disabled="pending.has(article.folderName)" @click="changeStatus(article)"><LoaderCircle v-if="pending.has(article.folderName)" :size="17" class="collection-spin" /><ArchiveRestore v-else-if="status === 'archived'" :size="17" /><Archive v-else :size="17" /></button>
            <button class="icon-button article-delete-action" type="button" title="永久删除" :aria-label="`永久删除：${article.title}`" :disabled="pending.has(article.folderName)" @click="deleteArticle(article)"><LoaderCircle v-if="pending.has(article.folderName)" :size="17" class="collection-spin" /><Trash2 v-else :size="17" /></button>
          </div>
        </article>
      </div>
    </template>
  </section>
</template>

<style scoped>
.library-toolbar { display: flex; align-items: center; gap: 12px; margin-bottom: 18px; }
.library-search { display: flex; flex: 1; min-width: 0; height: 42px; align-items: center; gap: 8px; padding: 0 12px; color: var(--muted-strong); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 6px; }
.library-search:focus-within { border-color: var(--primary); outline: 2px solid var(--focus); }
.library-search input { min-width: 0; width: 100%; color: var(--ink); background: transparent; border: 0; outline: 0; font-size: 13px; }
.library-result-count { color: var(--muted-strong); font-size: 12px; white-space: nowrap; }
.article-list { border-top: 1px solid var(--line); }
.article-row { display: flex; align-items: center; gap: 8px; border-bottom: 1px solid var(--line); padding: 4px 0; transition: background-color 160ms ease, box-shadow 160ms ease; }
.article-row:hover { background: var(--surface-alt); box-shadow: inset 3px 0 0 var(--primary-soft); }
.article-open { display: flex; flex: 1; align-items: center; gap: 14px; min-width: 0; padding: 16px 8px; text-decoration: none; color: var(--ink); border-radius: 6px; outline: 0; }
.article-open:hover .article-row-main strong { color: var(--primary); }
.article-open:focus-visible { box-shadow: 0 0 0 2px var(--focus); }
.article-row-icon { display: grid; width: 36px; height: 36px; flex-shrink: 0; place-items: center; color: var(--primary); background: var(--primary-soft); border-radius: 6px; }
.article-row-main { min-width: 0; flex: 1; }
.article-row-main strong, .article-row-main small { display: block; overflow-wrap: anywhere; }
.article-row-main strong { font-size: 14px; }
.article-row-main small { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; margin-top: 5px; color: var(--muted-strong); font-size: 12px; }
.article-row-main .article-row-source { -webkit-line-clamp: 1; margin-top: 4px; color: var(--primary); font-size: 11px; }
.article-row-date { color: var(--muted-strong); font-size: 11px; white-space: nowrap; }
.article-row-actions { display: flex; align-items: center; gap: 4px; margin-right: 8px; flex-shrink: 0; }
.article-status-action, .article-delete-action { flex-shrink: 0; }
.article-delete-action { color: var(--danger, #c2413b); }
.article-delete-action:hover:not(:disabled), .article-delete-action:focus-visible { color: var(--danger, #c2413b); background: color-mix(in srgb, var(--danger, #c2413b) 12%, var(--surface)); }
.collection-empty { min-height: 280px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 14px; color: var(--muted-strong); text-align: center; }
.collection-empty h3 { color: var(--ink); font-size: 16px; margin: 0; }
.collection-feedback { display: flex; align-items: center; gap: 8px; padding: 10px 12px; background: var(--surface-alt); border-radius: 6px; font-size: 13px; overflow-wrap: anywhere; }
.collection-success { color: var(--success); }
.collection-error { color: oklch(0.52 0.18 25); }
.collection-feedback button { color: inherit; background: transparent; border: 0; text-decoration: underline; }
.collection-spin { animation: collection-spin 900ms linear infinite; }
@keyframes collection-spin { to { transform: rotate(360deg); } }
@media (max-width: 1000px) { .article-row-date { display: none; } }
@media (prefers-reduced-motion: reduce) { .collection-spin { animation: none; } }
</style>
