<script setup lang="ts">
import { Archive, ArrowRight, FileText, Tags } from "@lucide/vue";
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
        <span class="overview-stat-value">0</span>
        <span class="overview-stat-label">已收录资料</span>
      </article>
      <article class="overview-stat">
        <span class="overview-stat-icon"><Tags :size="19" aria-hidden="true" /></span>
        <span class="overview-stat-value">0</span>
        <span class="overview-stat-label">已建立标签</span>
      </article>
      <article class="overview-stat">
        <span class="overview-stat-icon"><FileText :size="19" aria-hidden="true" /></span>
        <span class="overview-stat-value">0</span>
        <span class="overview-stat-label">本周新增</span>
      </article>
    </div>

    <section class="content-panel dashboard-empty-panel" aria-labelledby="recent-heading">
      <div class="panel-heading">
        <div>
          <p class="panel-kicker">最近整理</p>
          <h3 id="recent-heading">还没有可显示的资料</h3>
        </div>
      </div>
      <div class="empty-state empty-state-inline">
        <span class="empty-state-icon"><Archive :size="26" aria-hidden="true" /></span>
        <p>资料保存后会显示在这里，方便你回看最近整理的内容。</p>
      </div>
    </section>
  </section>
</template>
