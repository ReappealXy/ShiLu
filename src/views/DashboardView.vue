<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Archive, ArrowRight, FileText, Tags } from "@lucide/vue";
import { listArticles, type ArticleSummary } from "../services/editor";
import { getErrorMessage } from "../services/library";
const articles = ref<ArticleSummary[]>([]);
const tagCount = ref(0);
const archiveCount = ref(0);
const loading = ref(true);
const error = ref("");
const recent = computed(() => articles.value.slice(0, 5));
onMounted(async () => {
  try {
    const [active, archived] = await Promise.all([listArticles(), listArticles("", "archived")]);
    articles.value = active;
    tagCount.value = new Set(active.flatMap((article) => article.tags)).size;
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
        <p class="page-description">截图、链接和你的补充，之后都会整理成可阅读的本地资料。</p>
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
        <span class="overview-stat-icon"><Tags :size="19" aria-hidden="true" /></span>
        <span class="overview-stat-value">{{ loading || error ? '—' : tagCount }}</span>
        <span class="overview-stat-label">已建立标签</span>
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
          <p class="panel-kicker">最近整理</p>
          <h3 id="recent-heading">{{ loading ? '正在读取资料' : recent.length ? '最近保存的资料' : '还没有可显示的资料' }}</h3>
        </div>
      </div>
      <p v-if="error" class="dashboard-error" role="alert">{{ error }} <RouterLink to="/settings">前往设置</RouterLink></p>
      <div v-else-if="!loading && !recent.length" class="empty-state empty-state-inline">
        <span class="empty-state-icon"><Archive :size="26" aria-hidden="true" /></span>
        <p>资料保存后会显示在这里，方便你回看最近整理的内容。</p>
      </div>
      <ol v-else class="dashboard-recent"><li v-for="item in recent" :key="item.folderName"><RouterLink :to="`/articles/${item.folderName}`"><span>{{ item.title }}</span><ArrowRight :size="16" /></RouterLink></li></ol>
    </section>
  </section>
</template>

<style scoped>
.dashboard-recent { margin: 0; padding: 0 24px 18px; list-style: none; }
.dashboard-recent a { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 14px 0; border-top: 1px solid var(--line); color: var(--ink); font-size: 14px; text-decoration: none; }
.dashboard-recent span { overflow-wrap: anywhere; }
.dashboard-recent a:hover { color: var(--primary); }
.dashboard-error { padding: 0 24px 24px; font-size: 13px; color: var(--muted-strong); }
.dashboard-error a { color: var(--primary); }
</style>
