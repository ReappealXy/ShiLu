import { createApp } from "vue";
import { createRouter, createWebHashHistory } from "vue-router";
import App from "./App.vue";
import AppShell from "./layouts/AppShell.vue";
import CaptureView from "./views/CaptureView.vue";
import DashboardView from "./views/DashboardView.vue";
import LibraryView from "./views/LibraryView.vue";
import SettingsView from "./views/SettingsView.vue";
import ArticleEditorView from "./views/ArticleEditorView.vue";
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
        { path: "capture", name: "capture", component: CaptureView, meta: { title: "新建资料", section: "CAPTURE A THOUGHT" } },
        { path: "settings", name: "settings", component: SettingsView, meta: { title: "设置", section: "WORKSPACE PREFERENCES" } },
        { path: "articles/:id/edit", name: "article-edit", component: ArticleEditorView, meta: { title: "编辑资料", section: "ARTICLE EDITOR" } },
      ],
    },
    { path: "/:pathMatch(.*)*", redirect: "/overview" },
  ],
});

createApp(App).use(router).mount("#app");
