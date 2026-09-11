<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { FolderCog, LibraryBig, RefreshCw, Search, Settings, SlidersHorizontal } from "@lucide/vue";
import { getErrorMessage, getLibraryStatus, type LibraryStatus } from "../services/library";
import { listArticles, type ArticleSummary } from "../services/editor";

const libraryStatus = ref<LibraryStatus | null>(null);
const articles = ref<ArticleSummary[]>([]);
const query = ref("");
const loadingStatus = ref(true);
const loadingArticles = ref(false);
const errorMessage = ref("");
let searchTimer: ReturnType<typeof window.setTimeout> | undefined;

async function loadArticles() {
  if (!libraryStatus.value?.ready) return;
  loadingArticles.value = true;
  errorMessage.value = "";
  try { articles.value = await listArticles(query.value); }
  catch (error) { errorMessage.value = getErrorMessage(error, "无法读取资料列表，请稍后重试。"); }
  finally { loadingArticles.value = false; }
}

async function loadLibraryStatus() {
  loadingStatus.value = true;
  errorMessage.value = "";
  try {
    libraryStatus.value = await getLibraryStatus();
    await loadArticles();
  } catch (error) {
    libraryStatus.value = null;
    errorMessage.value = getErrorMessage(error, "无法读取资料库状态，请稍后重试。");
  } finally { loadingStatus.value = false; }
}

function scheduleSearch() {
  if (searchTimer) window.clearTimeout(searchTimer);
  searchTimer = window.setTimeout(() => void loadArticles(), 220);
}

function formatDate(value: string): string {
  if (!value) return "未记录时间";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

watch(query, scheduleSearch);
onMounted(loadLibraryStatus);
</script>

<template>
  <section class="page-view library-view" aria-labelledby="library-heading">
    <div class="page-heading library-heading">
      <div><p class="page-eyebrow">本地 Markdown 资料</p><h2 id="library-heading">资料库</h2><p class="page-description">在标题、正文、标签、来源和链接中搜索，找回你保存过的内容。</p></div>
      <RouterLink class="button button-primary" to="/capture"><LibraryBig :size="16" />新建资料</RouterLink>
    </div>

    <section v-if="loadingStatus" class="content-panel library-empty-panel"><div class="empty-state"><span class="empty-state-icon empty-state-icon-large"><LibraryBig :size="32" /></span><h3>正在读取资料库</h3><div class="library-loading-lines" aria-hidden="true"><span></span><span></span></div></div></section>
    <section v-else-if="errorMessage && !libraryStatus" class="content-panel library-empty-panel"><div class="empty-state"><span class="empty-state-icon empty-state-icon-large"><FolderCog :size="32" /></span><h3>无法读取资料库</h3><p>{{ errorMessage }}</p><div class="empty-state-actions"><button class="button button-secondary" type="button" @click="loadLibraryStatus"><RefreshCw :size="17" />重新读取</button><RouterLink class="button button-primary" to="/settings"><Settings :size="17" />前往设置</RouterLink></div></div></section>
    <section v-else-if="!libraryStatus?.ready" class="content-panel library-empty-panel"><div class="empty-state"><span class="empty-state-icon empty-state-icon-large"><LibraryBig :size="32" /></span><h3>{{ libraryStatus?.configured ? "资料库需要重新准备" : "资料库尚未设置" }}</h3><p>{{ libraryStatus?.configured ? `资料库缺少：${libraryStatus.missingItems.join("、") || "必要文件"}。` : "先选择一个本地文件夹。每条资料都会保存在独立的 Markdown 文件夹中。" }}</p><div class="empty-state-actions"><RouterLink class="button button-primary" to="/settings"><Settings :size="17" />设置资料库</RouterLink></div></div><div class="empty-state-footer"><FolderCog :size="17" /><span>设置完成后，资料会在此处显示。</span></div></section>
    <template v-else>
      <div class="library-toolbar"><label class="library-search"><Search :size="17" /><input v-model="query" type="search" placeholder="搜索标题、正文、标签、来源或链接" /><kbd>Enter</kbd></label><span class="library-result-count"><SlidersHorizontal :size="15" />{{ loadingArticles ? "正在搜索..." : `${articles.length} 条资料` }}</span></div>
      <section v-if="errorMessage" class="library-inline-error" role="alert">{{ errorMessage }} <button type="button" @click="loadArticles">重试</button></section>
      <section v-else-if="!articles.length && !loadingArticles" class="content-panel library-empty-panel library-no-results"><div class="empty-state"><span class="empty-state-icon empty-state-icon-large"><Search :size="30" /></span><h3>{{ query ? "没有匹配的资料" : "资料库还是空的" }}</h3><p>{{ query ? "换一个关键词试试，搜索会覆盖已经保存的正文和标签。" : "保存第一条截图资料后，它会出现在这里。" }}</p><div class="empty-state-actions"><RouterLink class="button button-primary" to="/capture">新建资料</RouterLink></div></div></section>
      <section v-else class="article-list" aria-label="资料列表"><RouterLink v-for="article in articles" :key="article.folderName" class="article-row" :to="`/articles/${article.folderName}/edit`"><span class="article-row-icon"><LibraryBig :size="18" /></span><span class="article-row-main"><strong>{{ article.title }}</strong><small>{{ article.summary || "暂无摘要" }}</small><span v-if="article.tags.length" class="article-tags"><i v-for="tag in article.tags" :key="tag">{{ tag }}</i></span></span><span class="article-row-meta"><small>{{ formatDate(article.updatedAt) }}</small><b v-if="article.sourceUrl">有来源链接</b></span></RouterLink></section>
    </template>
  </section>
</template>

<style scoped>
.library-heading { align-items: end; }
.library-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 14px; }
.library-search { display: flex; width: min(100%, 590px); height: 40px; align-items: center; gap: 8px; padding: 0 9px 0 12px; color: var(--muted); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 9px; transition: border-color 130ms ease-out, box-shadow 130ms ease-out; }
.library-search:focus-within { color: var(--primary); border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus); }
.library-search input { min-width: 0; flex: 1; color: var(--ink); background: transparent; border: 0; outline: 0; font-size: 13px; }
.library-search input::placeholder { color: var(--muted-strong); opacity: 1; }
.library-search kbd { background: var(--surface-alt); }
.library-result-count { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: 12px; white-space: nowrap; }
.article-list { display: grid; gap: 8px; }
.article-row { display: grid; grid-template-columns: 38px minmax(0, 1fr) auto; align-items: center; gap: 13px; min-height: 86px; padding: 14px 16px; color: var(--ink); text-decoration: none; background: var(--surface); border: 1px solid var(--line); border-radius: 10px; transition: border-color 140ms ease-out, box-shadow 140ms ease-out, transform 140ms ease-out; }
.article-row:hover { border-color: var(--primary); box-shadow: var(--shadow-sm); transform: translateY(-1px); }
.article-row-icon { display: grid; width: 38px; height: 38px; place-items: center; color: var(--primary); background: var(--primary-soft); border-radius: 9px; }
.article-row-main { min-width: 0; }
.article-row-main strong, .article-row-main small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.article-row-main strong { font-size: 14px; font-weight: 720; }
.article-row-main small { max-width: 690px; margin-top: 5px; color: var(--muted-strong); font-size: 12px; }
.article-tags { display: flex; flex-wrap: wrap; gap: 5px; margin-top: 8px; }
.article-tags i { padding: 3px 6px; color: var(--primary); background: var(--primary-soft); border-radius: 5px; font-size: 10px; font-style: normal; font-weight: 680; }
.article-row-meta { display: grid; justify-items: end; gap: 7px; color: var(--muted); font-size: 11px; text-align: right; white-space: nowrap; }
.article-row-meta b { color: var(--success); font-size: 10px; font-weight: 680; }
.library-no-results { min-height: 310px; }
.library-inline-error { margin-bottom: 14px; padding: 10px 12px; color: oklch(0.55 0.17 25); background: color-mix(in oklch, oklch(0.55 0.17 25) 9%, var(--surface)); border-radius: 8px; font-size: 12px; }
.library-inline-error button { margin-left: 6px; padding: 0; color: inherit; background: transparent; border: 0; text-decoration: underline; }
@media (max-width: 1140px) { .article-row { grid-template-columns: 38px minmax(0, 1fr); } .article-row-meta { display: none; } }
</style>
