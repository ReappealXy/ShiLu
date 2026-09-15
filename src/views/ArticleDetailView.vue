<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Archive, ArchiveRestore, ArrowLeft, Check, ExternalLink, FileText, LoaderCircle, SquarePen, Trash2 } from "@lucide/vue";
import { useRoute, useRouter } from "vue-router";
import { getErrorMessage } from "../services/library";
import { deleteArticlePermanently, readArticle, setArticleStatus, type ArticleDocument, type ArticleStatus } from "../services/editor";
import { articleImageUrl, cleanDisplayMarkdown, localImageReferences, renderArticleMarkdown } from "../services/markdown";

const route = useRoute();
const router = useRouter();
const articleReference = String(route.params.id ?? "");
const article = ref<ArticleDocument | null>(null);
const loading = ref(true);
const errorMessage = ref("");
const feedback = ref("");
const statusBusy = ref(false);
const deleteBusy = ref(false);

const status = computed<ArticleStatus>(() => article.value?.status ?? "active");
const statusLabel = computed(() => ({ draft: "处理中", active: "已收录", archived: "已归档" })[status.value]);
const returnPath = computed(() => status.value === "archived" ? "/archive" : "/library");
const returnLabel = computed(() => status.value === "archived" ? "返回归档箱" : "返回资料库");
const displayMarkdown = computed(() => cleanDisplayMarkdown(article.value?.content ?? "", article.value?.title ?? ""));
const contentImages = computed(() => localImageReferences(displayMarkdown.value));
const renderedBody = computed(() => renderArticleMarkdown(
  displayMarkdown.value.replace(/!\[[^\]]*\]\(images\/[a-zA-Z0-9_.-]+\.(?:png|jpe?g|webp)\)/gi, ""),
  article.value?.markdownPath ?? "",
));

function editArticle() {
  if (!article.value) return;
  void router.push({
    path: `/articles/${article.value.folderName}/edit`,
    query: { from: status.value === "archived" ? "archive" : "library" },
  });
}

async function changeStatus(next: ArticleStatus) {
  if (!article.value || statusBusy.value) return;
  statusBusy.value = true;
  errorMessage.value = "";
  feedback.value = "";
  try {
    article.value = await setArticleStatus(article.value.folderName, next);
    window.dispatchEvent(new CustomEvent("shilu:article-status", { detail: next }));
    feedback.value = next === "archived" ? "已归档，文章和图片均已保留。" : "已恢复到资料库。";
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "状态更新失败，请重试。");
  } finally {
    statusBusy.value = false;
  }
}

async function deleteArticle() {
  if (!article.value || statusBusy.value || deleteBusy.value) return;
  const title = article.value.title || "未命名资料";
  if (!window.confirm(`确定永久删除“${title}”吗？\nMarkdown 文件夹及其中的图片都会被删除，且无法恢复。`)) return;
  deleteBusy.value = true;
  errorMessage.value = "";
  try {
    await deleteArticlePermanently(article.value.folderName);
    window.dispatchEvent(new CustomEvent("shilu:article-deleted", { detail: article.value.folderName }));
    await router.replace(returnPath.value);
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "永久删除失败，资料仍然保留。请重试。");
  } finally {
    deleteBusy.value = false;
  }
}

function formatDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "" : date.toLocaleString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

async function loadArticle() {
  loading.value = true;
  errorMessage.value = "";
  try {
    article.value = await readArticle(articleReference);
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "无法读取资料，请返回列表后重试。");
  } finally {
    loading.value = false;
  }
}

onMounted(() => void loadArticle());
</script>

<template>
  <section class="page-view article-detail-view" aria-labelledby="article-detail-heading">
    <div class="article-detail-topbar">
      <button class="editor-back" type="button" @click="router.push(returnPath)"><ArrowLeft :size="16" />{{ returnLabel }}</button>
      <div v-if="article" class="article-detail-actions">
        <button class="button button-secondary" type="button" @click="editArticle"><SquarePen :size="16" />编辑</button>
        <button class="button button-secondary" type="button" :disabled="statusBusy" @click="changeStatus(status === 'archived' ? 'active' : 'archived')">
          <LoaderCircle v-if="statusBusy" class="article-detail-spin" :size="16" />
          <ArchiveRestore v-else-if="status === 'archived'" :size="16" />
          <Archive v-else :size="16" />
          {{ status === 'archived' ? '恢复到资料库' : '归档' }}
        </button>
        <button class="button button-danger" type="button" :disabled="statusBusy || deleteBusy" @click="deleteArticle">
          <LoaderCircle v-if="deleteBusy" class="article-detail-spin" :size="16" />
          <Trash2 v-else :size="16" />
          {{ deleteBusy ? '正在删除...' : '永久删除' }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="article-detail-loading" role="status"><LoaderCircle :size="24" class="article-detail-spin" />正在读取资料...</div>
    <div v-else-if="!article" class="article-detail-loading" role="alert"><FileText :size="26" /><span>{{ errorMessage || '资料不存在。' }}</span><button class="button button-secondary" type="button" @click="loadArticle">重新读取</button></div>
    <template v-else>
      <p v-if="feedback" class="article-detail-feedback article-detail-feedback-success" role="status"><Check :size="16" />{{ feedback }}</p>
      <p v-if="errorMessage" class="article-detail-feedback article-detail-feedback-error" role="alert">{{ errorMessage }}<button type="button" @click="loadArticle">重新读取</button></p>

      <article class="article-detail-card">
        <header class="article-detail-header">
          <div class="article-detail-heading-line">
            <h1 id="article-detail-heading">{{ article.title || '未命名资料' }}</h1>
            <span class="article-status">{{ statusLabel }}</span>
          </div>
        </header>

        <div v-if="contentImages.length" class="article-detail-images" aria-label="正文配图">
          <img v-for="(path, index) in contentImages" :key="`${path}-${index}`" :src="articleImageUrl(article.markdownPath, path)" :alt="`${article.title || '资料'}的正文配图 ${index + 1}`" loading="lazy" />
        </div>
        <div class="article-detail-body markdown-body" v-if="article.content.trim()" v-html="renderedBody" />
        <div v-else class="article-detail-empty-body"><FileText :size="24" /><p>还没有正文内容</p><button class="button button-secondary" type="button" @click="editArticle"><SquarePen :size="16" />开始编辑</button></div>

        <a v-if="article.sourceUrl" class="article-detail-source article-detail-source-footer" :href="article.sourceUrl" target="_blank" rel="noopener noreferrer">
          <ExternalLink :size="16" />
          <span>{{ article.sourceUrl }}</span>
        </a>

        <footer class="article-detail-footer">
          <time v-if="formatDate(article.updatedAt)" :datetime="article.updatedAt">最后更新 {{ formatDate(article.updatedAt) }}</time>
        </footer>
      </article>
    </template>
  </section>
</template>

<style scoped>
.article-detail-view { max-width: 980px; }
.article-detail-topbar { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 22px; }
.editor-back { display: inline-flex; align-items: center; gap: 8px; min-height: 34px; padding: 7px 11px 7px 8px; border: 1px solid var(--line); color: var(--muted-strong); background: var(--surface); border-radius: 8px; font-size: 12px; box-shadow: var(--shadow-sm); transition: color 160ms ease, border-color 160ms ease, transform 160ms ease; }
.editor-back:hover { color: var(--primary); border-color: var(--primary); transform: translateX(-2px); }
.article-detail-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 8px; }
.button-danger { color: var(--danger, #c2413b); border-color: color-mix(in srgb, var(--danger, #c2413b) 35%, var(--line-strong)); background: color-mix(in srgb, var(--danger, #c2413b) 8%, var(--surface)); }
.button-danger:hover:not(:disabled) { color: #fff; border-color: var(--danger, #c2413b); background: var(--danger, #c2413b); }
.article-detail-loading { display: flex; min-height: 300px; align-items: center; justify-content: center; gap: 12px; color: var(--muted-strong); }
.article-detail-spin { animation: article-detail-spin 900ms linear infinite; }
@keyframes article-detail-spin { to { transform: rotate(360deg); } }
.article-detail-feedback { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin: 0 0 14px; padding: 10px 12px; border-radius: 6px; font-size: 13px; line-height: 1.55; }
.article-detail-feedback-success { color: var(--success); background: var(--success-soft); }
.article-detail-feedback-error { color: oklch(0.52 0.18 25); background: var(--surface-alt); }
.article-detail-feedback button { color: inherit; background: transparent; border: 0; text-decoration: underline; }
.article-detail-card { overflow: hidden; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; box-shadow: var(--shadow-sm); }
.article-detail-header { padding: 26px 32px 22px; }
.article-detail-heading-line { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; }
.article-detail-heading-line h1 { min-width: 0; margin: 0; color: var(--ink); font-size: 32px; line-height: 1.3; overflow-wrap: anywhere; }
.article-status { display: inline-block; padding: 4px 8px; color: var(--primary); background: var(--primary-soft); border-radius: 5px; font-size: 11px; font-weight: 680; white-space: nowrap; }
.article-detail-source { display: flex; max-width: 100%; align-items: center; gap: 7px; color: var(--primary); font-size: 13px; line-height: 1.5; overflow-wrap: anywhere; }
.article-detail-source-footer { margin: 0 32px 28px; padding-top: 18px; border-top: 1px solid var(--line); }
.article-detail-source span { overflow-wrap: anywhere; }
.article-detail-body { padding: 0 32px 30px; }
.article-detail-images { display: grid; gap: 14px; padding: 0 32px 26px; }
.article-detail-images img { display: block; width: 100%; max-height: 620px; object-fit: contain; border: 1px solid var(--line); border-radius: 8px; background: var(--surface-alt); }
.article-detail-body :deep(.image-unavailable) { display: none; }
.article-detail-empty-body { display: flex; min-height: 220px; flex-direction: column; align-items: center; justify-content: center; gap: 10px; color: var(--muted-strong); border-top: 1px solid var(--line); }
.article-detail-empty-body p { margin: 0 0 4px; }
.article-detail-footer { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 48px; padding: 10px 32px; color: var(--muted); background: var(--surface-alt); border-top: 1px solid var(--line); font-size: 11px; }
.article-detail-footer time { white-space: nowrap; }
@media (max-width: 680px) {
  .article-detail-topbar { align-items: flex-start; flex-direction: column; }
  .article-detail-actions { width: 100%; justify-content: flex-start; }
  .article-detail-heading-line h1 { font-size: 26px; }
  .article-detail-header, .article-detail-body { padding-right: 20px; padding-left: 20px; }
  .article-detail-images { padding-right: 20px; padding-left: 20px; }
  .article-detail-source-footer { margin-right: 20px; margin-left: 20px; }
  .article-detail-footer { align-items: flex-start; flex-direction: column; padding-right: 20px; padding-left: 20px; }
  .article-detail-footer time { white-space: normal; }
}
@media (prefers-reduced-motion: reduce) { .article-detail-spin { animation: none; } }
</style>
