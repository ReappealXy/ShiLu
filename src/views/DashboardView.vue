<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Archive, ArrowRight, FileText, Image as ImageIcon } from "@lucide/vue";
import { listArticles, type ArticleSummary } from "../services/editor";
import { getErrorMessage } from "../services/library";
import { articleImageUrl } from "../services/markdown";
const articles = ref<ArticleSummary[]>([]);
const imageCount = ref(0);
const archiveCount = ref(0);
const loading = ref(true);
const error = ref("");
function imageUrl(article: ArticleSummary) {
  return article.firstContentImage ? articleImageUrl(article.markdownPath, article.firstContentImage) : "";
}
onMounted(async () => {
  try {
    const [active, archived] = await Promise.all([listArticles(), listArticles("", "archived")]);
    articles.value = active;
    imageCount.value = active.filter((article) => Boolean(article.firstContentImage)).length;
    archiveCount.value = archived.length;
  } catch (value) { error.value = getErrorMessage(value, "无法读取资料库，请到设置中检查保存位置。"); }
  finally { loading.value = false; }
});
</script>

<template>
  <section class="page-view dashboard-view" aria-labelledby="dashboard-heading">
    <div class="page-heading">
      <div>
        <p class="page-eyebrow">个人资料工作台</p>
        <h2 id="dashboard-heading">从一条值得留下的内容开始</h2>
        <p class="page-description">文字、链接和配图，都可以整理成可阅读的本地资料。</p>
      </div>
      <RouterLink class="button button-primary" to="/capture">
        前往新建资料
        <ArrowRight :size="17" aria-hidden="true" />
      </RouterLink>
    </div>

    <div class="overview-stats" aria-label="资料库概览">
      <article class="overview-stat">
        <span class="overview-stat-icon"><Archive :size="19" aria-hidden="true" /></span>
        <span class="overview-stat-value">{{ loading || error ? '—' : articles.length }}</span>
        <span class="overview-stat-label">已收录资料</span>
      </article>
      <article class="overview-stat">
        <span class="overview-stat-icon"><ImageIcon :size="19" aria-hidden="true" /></span>
        <span class="overview-stat-value">{{ loading || error ? '—' : imageCount }}</span>
        <span class="overview-stat-label">含配图资料</span>
      </article>
      <article class="overview-stat">
        <span class="overview-stat-icon"><FileText :size="19" aria-hidden="true" /></span>
        <span class="overview-stat-value">{{ loading || error ? '—' : archiveCount }}</span>
        <RouterLink class="overview-stat-label" to="/archive">已归档资料</RouterLink>
      </article>
    </div>

    <section class="content-panel dashboard-empty-panel" aria-labelledby="recent-heading">
      <div class="panel-heading">
        <div>
          <p class="panel-kicker">按最近更新时间排序</p>
          <h3 id="recent-heading">{{ loading ? '正在读取资料' : articles.length ? '全部已收录资料' : '还没有可显示的资料' }}</h3>
        </div>
      </div>
      <p v-if="error" class="dashboard-error" role="alert">{{ error }} <RouterLink to="/settings">前往设置</RouterLink></p>
      <div v-else-if="!loading && !articles.length" class="empty-state empty-state-inline">
        <span class="empty-state-icon"><Archive :size="26" aria-hidden="true" /></span>
        <p>资料保存后会显示在这里，方便你回看已收录的内容。</p>
      </div>
      <div v-else class="dashboard-recent" aria-label="全部已收录资料">
        <RouterLink v-for="item in articles" :key="item.folderName" class="dashboard-recent-card" :to="`/articles/${item.folderName}`">
          <span class="dashboard-card-media">
            <img v-if="imageUrl(item)" :src="imageUrl(item)" :alt="`${item.title} 的正文配图`" loading="lazy" />
            <span v-else class="dashboard-card-placeholder" aria-hidden="true"><ImageIcon :size="25" /></span>
            <span class="dashboard-card-overlay" aria-hidden="true"><ArrowRight :size="18" /></span>
          </span>
          <span class="dashboard-card-footer">
            <strong>{{ item.title || '未命名资料' }}</strong>
            <ArrowRight class="dashboard-card-arrow" :size="16" aria-hidden="true" />
          </span>
        </RouterLink>
      </div>
    </section>
  </section>
</template>

<style scoped>
.dashboard-recent { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 250px)); gap: 14px; margin: 0; padding: 0 24px 24px; }
.dashboard-recent-card { display: block; min-width: 0; overflow: hidden; color: var(--ink); background: var(--surface); border: 1px solid var(--line); border-radius: 10px; text-decoration: none; transition: border-color 180ms ease-out, box-shadow 180ms ease-out, transform 180ms ease-out; }
.dashboard-recent-card:hover { border-color: var(--line-strong); box-shadow: var(--shadow-sm); transform: translateY(-3px); }
.dashboard-recent-card:focus-visible { outline: 2px solid var(--focus); outline-offset: 3px; }
.dashboard-card-media { position: relative; display: block; aspect-ratio: 16 / 9; overflow: hidden; background: var(--surface-alt); }
.dashboard-card-media img { display: block; width: 100%; height: 100%; object-fit: cover; transition: transform 240ms cubic-bezier(0.16, 1, 0.3, 1); }
.dashboard-recent-card:hover .dashboard-card-media img { transform: scale(1.035); }
.dashboard-card-placeholder { display: grid; width: 100%; height: 100%; place-items: center; color: var(--primary); background: var(--primary-soft); }
.dashboard-card-overlay { position: absolute; right: 10px; bottom: 10px; display: grid; width: 32px; height: 32px; place-items: center; color: var(--primary); background: var(--surface); border-radius: 8px; opacity: 0; transform: translateY(4px); transition: opacity 180ms ease-out, transform 180ms ease-out; }
.dashboard-recent-card:hover .dashboard-card-overlay, .dashboard-recent-card:focus-visible .dashboard-card-overlay { opacity: 1; transform: translateY(0); }
.dashboard-card-footer { display: flex; min-height: 58px; align-items: center; justify-content: space-between; gap: 10px; padding: 12px 13px; }
.dashboard-card-footer strong { display: -webkit-box; min-width: 0; overflow: hidden; -webkit-box-orient: vertical; -webkit-line-clamp: 2; font-size: 14px; line-height: 1.45; overflow-wrap: anywhere; }
.dashboard-card-arrow { flex: 0 0 auto; color: var(--muted); transition: color 180ms ease-out, transform 180ms ease-out; }
.dashboard-recent-card:hover .dashboard-card-arrow, .dashboard-recent-card:focus-visible .dashboard-card-arrow { color: var(--primary); transform: translateX(2px); }
.dashboard-error { padding: 0 24px 24px; font-size: 13px; color: var(--muted-strong); }
.dashboard-error a { color: var(--primary); }
@media (max-width: 620px) { .dashboard-recent { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; padding-right: 14px; padding-left: 14px; } .dashboard-card-footer { padding-right: 10px; padding-left: 10px; } }
@media (max-width: 400px) { .dashboard-recent { grid-template-columns: 1fr; } }
@media (prefers-reduced-motion: reduce) { .dashboard-recent-card, .dashboard-card-media img, .dashboard-card-overlay, .dashboard-card-arrow { transition: none; } }
</style>
