import { createApp } from "vue";
import { createRouter, createWebHashHistory } from "vue-router";
import App from "./App.vue";
import AppShell from "./layouts/AppShell.vue";
import CaptureView from "./views/CaptureView.vue";
import DashboardView from "./views/DashboardView.vue";
import LibraryView from "./views/LibraryView.vue";
import SettingsView from "./views/SettingsView.vue";
import ArticleEditorView from "./views/ArticleEditorView.vue";
import ArticleDetailView from "./views/ArticleDetailView.vue";
import "./styles.css";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/overview" },
    {
      path: "/",
      component: AppShell,
      children: [
        { path: "overview", name: "overview", component: DashboardView, meta: { title: "总览", section: "SHILU / WORKSPACE" } },
        { path: "library", name: "library", component: LibraryView, meta: { title: "资料库", section: "YOUR ARCHIVE" } },
        // Keep the old URL as a compatibility redirect; unfinished captures remain internal.
        { path: "drafts", redirect: "/library" },
        { path: "archive", name: "archive", component: LibraryView, meta: { title: "归档箱", section: "ARCHIVED", articleStatus: "archived" } },
        { path: "capture", name: "capture", component: CaptureView, meta: { title: "新建资料", section: "CAPTURE A THOUGHT" } },
        { path: "settings", name: "settings", component: SettingsView, meta: { title: "设置", section: "WORKSPACE PREFERENCES" } },
        { path: "articles/:id/edit", name: "article-edit", component: ArticleEditorView, meta: { title: "编辑资料", section: "ARTICLE EDITOR" } },
        { path: "articles/:id", name: "article-detail", component: ArticleDetailView, meta: { title: "阅读资料", section: "ARTICLE READER" } },
      ],
    },
    { path: "/:pathMatch(.*)*", redirect: "/overview" },
  ],
});

createApp(App).use(router).mount("#app");
