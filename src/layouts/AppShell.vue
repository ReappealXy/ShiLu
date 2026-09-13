<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  Archive,
  ChevronLeft,
  ChevronRight,
  Command,
  FolderOpen,
  Home,
  Library,
  Moon,
  Plus,
  Search,
  Settings,
  Sun,
  X,
} from "@lucide/vue";
import { applyTheme, getStoredTheme, setStoredTheme } from "../services/settings";
import { getLibraryStatus } from "../services/library";

type CommandItem = {
  label: string;
  description: string;
  to: string;
  icon: typeof Home;
};

const route = useRoute();
const router = useRouter();
const sidebarCollapsed = ref(false);
const darkMode = ref(false);
const query = ref("");
const commandOpen = ref(false);
const searchInput = ref<HTMLInputElement>();
const toast = ref("");
const wheelPosition = ref(0);
const wheelSettled = ref(true);
const reducedMotion = ref(false);
const dragging = ref(false);
const articleSection = ref("");
const libraryLabel = ref("正在读取");
async function refreshLibraryLabel() {
  try { const result = await getLibraryStatus(); libraryLabel.value = result.ready ? "已就绪" : result.configured ? "需要检查" : "未设置"; }
  catch { libraryLabel.value = "读取失败"; }
}
let toastTimer: ReturnType<typeof window.setTimeout> | undefined;
let snapTimer: ReturnType<typeof window.setTimeout> | undefined;
let dragStartY: number | undefined;
let dragStartPosition: number | undefined;
let dragPointerId: number | undefined;
let suppressClickUntil = 0;
let reducedMotionQuery: MediaQueryList | undefined;

const navigation: CommandItem[] = [
  { label: "总览", description: "查看当前资料库状态", to: "/overview", icon: Home },
  { label: "新建资料", description: "从截图或手动内容开始", to: "/capture", icon: Plus },
  { label: "资料库", description: "浏览和查找已保存资料", to: "/library", icon: Library },
  { label: "归档箱", description: "查看或恢复已归档资料", to: "/archive", icon: Archive },
];

const settingsItem: CommandItem = {
  label: "设置",
  description: "管理资料库和工作区偏好",
  to: "/settings",
  icon: Settings,
};

const commands = [...navigation, settingsItem];
const selectedNavIndex = computed(() => clampNavIndex(Math.round(wheelPosition.value)));
const routeTitle = computed(() => String(route.meta.title ?? "拾录"));
const routeSection = computed(() => String(route.meta.section ?? "SHILU"));
const matchedCommands = computed(() => {
  const value = query.value.trim().toLowerCase();
  if (!value) return commands;
  return commands.filter((item) => `${item.label} ${item.description}`.toLowerCase().includes(value));
});

function isNavItemCurrent(item: CommandItem) {
  const normalizedPath = normalizedNavPath(route.path);
  return normalizedPath === item.to || normalizedPath.startsWith(`${item.to}/`);
}

function syncSelectedNavItem(path: string) {
  const normalizedPath = normalizedNavPath(path);
  const index = commands.findIndex((item) => normalizedPath === item.to || normalizedPath.startsWith(`${item.to}/`));
  if (index >= 0) settleOnNavItem(index);
}

function normalizedNavPath(path: string) {
  if (!path.startsWith("/articles/")) return path;
  if (articleSection.value) return articleSection.value;
  return route.query.from === "archive" ? "/archive" : "/library";
}

function clampNavIndex(index: number) {
  return Math.max(0, Math.min(commands.length - 1, index));
}

function relativeNavOffset(index: number) {
  return index - wheelPosition.value;
}

function nearestWheelPosition(index: number) {
  return clampNavIndex(index);
}

function isNavItemAtBaseline(index: number) {
  return wheelSettled.value && Math.abs(relativeNavOffset(index)) < 0.01;
}

function settleOnNavItem(index: number) {
  cancelWheelSnap();
  wheelPosition.value = nearestWheelPosition(index);
  wheelSettled.value = true;
}

function moveWheel(delta: number) {
  settleOnNavItem(Math.round(wheelPosition.value) + delta);
}

function snapWheel() {
  settleOnNavItem(Math.round(wheelPosition.value));
}

function cancelWheelSnap() {
  if (snapTimer) window.clearTimeout(snapTimer);
  snapTimer = undefined;
}

function scheduleWheelSnap() {
  cancelWheelSnap();
  // Finish one wheel gesture before routing so touchpad events cannot fight the route sync.
  snapTimer = window.setTimeout(() => {
    snapWheel();
    if (!isNavItemCurrent(commands[selectedNavIndex.value])) activateNavItem();
  }, 130);
}

function activateNavItem(index = selectedNavIndex.value) {
  const nextIndex = clampNavIndex(index);
  settleOnNavItem(nextIndex);
  navigate(commands[nextIndex]);
}

function navItemStyle(index: number) {
  const offset = relativeNavOffset(index);
  const distance = Math.abs(offset);
  const angle = Math.max(-1.45, Math.min(1.45, offset * 0.5));
  const x = -64 * (1 - Math.cos(angle));
  const y = 150 * Math.sin(angle);
  return {
    "--arc-x": `${x.toFixed(2)}px`,
    "--arc-y": `${y.toFixed(2)}px`,
    "--arc-rotate": `${(-offset * 7.5).toFixed(2)}deg`,
    "--arc-scale": `${Math.max(0.74, 1 - distance * 0.1).toFixed(3)}`,
    "--arc-opacity": `${Math.max(0.12, 1 - distance * 0.42).toFixed(3)}`,
    "--arc-blur": `${Math.max(0, (distance - 0.15) * 0.7).toFixed(2)}px`,
    "--arc-depth": `${Math.round(20 - distance * 5)}`,
    visibility: !sidebarCollapsed.value && !reducedMotion.value && distance > 2.5 ? "hidden" as const : "visible" as const,
  };
}

function onNavWheel(event: WheelEvent) {
  if (event.ctrlKey || event.deltaY === 0 || dragPointerId !== undefined) return;
  event.preventDefault();
  const unit = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? 16
    : event.deltaMode === WheelEvent.DOM_DELTA_PAGE ? (event.currentTarget as HTMLElement).clientHeight : 1;
  const pixels = event.deltaY * unit;
  const movement = Math.max(-0.72, Math.min(0.72, pixels / 120));
  wheelSettled.value = false;
  wheelPosition.value = Math.max(0, Math.min(commands.length - 1, wheelPosition.value + movement));
  scheduleWheelSnap();
}

function onNavPointerDown(event: PointerEvent) {
  if (reducedMotion.value || sidebarCollapsed.value) return;
  if (!event.isPrimary || (event.pointerType === "mouse" && event.button !== 0)) return;
  cancelWheelSnap();
  dragPointerId = event.pointerId;
  dragStartY = event.clientY;
  dragStartPosition = wheelPosition.value;
  dragging.value = false;
}

function onNavPointerMove(event: PointerEvent) {
  if (event.pointerId !== dragPointerId || dragStartY === undefined || dragStartPosition === undefined) return;
  if (event.pointerType === "mouse" && !(event.buttons & 1)) {
    onNavPointerUp(event);
    return;
  }
  const delta = event.clientY - dragStartY;
  if (!dragging.value && Math.abs(delta) > 5) {
    dragging.value = true;
    // Capturing on pointerdown retargets ordinary button clicks to the nav container.
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }
  if (!dragging.value) return;
  wheelSettled.value = false;
  wheelPosition.value = Math.max(0, Math.min(commands.length - 1, dragStartPosition - delta / 82));
}

function onNavPointerUp(event: PointerEvent) {
  if (event.pointerId !== dragPointerId) return;
  const wasDragging = dragging.value;
  if (dragStartY !== undefined) snapWheel();
  dragStartY = undefined;
  dragStartPosition = undefined;
  dragPointerId = undefined;
  const target = event.currentTarget as HTMLElement;
  if (target.hasPointerCapture?.(event.pointerId)) target.releasePointerCapture(event.pointerId);
  if (wasDragging) suppressClickUntil = Date.now() + 250;
  dragging.value = false;
}

function onNavPointerLeave(event: PointerEvent) {
  if (!dragging.value) onNavPointerUp(event);
}

function onNavKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowDown" || event.key === "ArrowRight") {
    event.preventDefault();
    moveWheel(1);
    focusSelectedNavItem();
  } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
    event.preventDefault();
    moveWheel(-1);
    focusSelectedNavItem();
  } else if (event.key === "Home") {
    event.preventDefault();
    settleOnNavItem(0);
    focusSelectedNavItem();
  } else if (event.key === "End") {
    event.preventDefault();
    settleOnNavItem(commands.length - 1);
    focusSelectedNavItem();
  } else if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    activateNavItem();
  }
}

function focusSelectedNavItem() {
  nextTick(() => {
    const item = document.querySelector<HTMLButtonElement>(`.arc-nav-item[data-nav-index="${selectedNavIndex.value}"]`);
    item?.focus();
  });
}

function updateReducedMotion() {
  reducedMotion.value = reducedMotionQuery?.matches ?? false;
}

watch(() => route.path, (path) => {
  articleSection.value = "";
  void refreshLibraryLabel();
  cancelWheelSnap();
  syncSelectedNavItem(path);
}, { immediate: true });

async function setTheme(next: boolean) {
  const theme = next ? "dark" : "light";
  darkMode.value = next;
  applyTheme(theme);
  try {
    await setStoredTheme(theme);
    showToast(next ? "已切换为深色主题" : "已切换为浅色主题");
  } catch {
    showToast("主题已切换，但暂时无法保存偏好");
  }
}

function showToast(message: string) {
  toast.value = message;
  if (toastTimer) window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toast.value = "";
  }, 2200);
}

function onToast(event: Event) {
  const message = (event as CustomEvent).detail;
  if (typeof message === "string") showToast(message);
}

function onArticleStatus(event: Event) {
  const status = (event as CustomEvent).detail;
  articleSection.value = status === "archived" ? "/archive" : "/library";
  syncSelectedNavItem(route.path);
}

async function navigate(item: CommandItem) {
  await router.push(item.to);
  syncSelectedNavItem(route.path);
  query.value = "";
  commandOpen.value = false;
}

function focusSearch() {
  commandOpen.value = true;
  nextTick(() => searchInput.value?.focus());
}

function onGlobalKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    focusSearch();
  }

  if (event.key === "Escape") {
    commandOpen.value = false;
    searchInput.value?.blur();
  }
}

onMounted(async () => {
  try {
    const savedTheme = await getStoredTheme();
    darkMode.value = savedTheme === "dark";
    applyTheme(savedTheme);
  } catch {
    applyTheme("light");
  }
  reducedMotionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
  updateReducedMotion();
  reducedMotionQuery.addEventListener?.("change", updateReducedMotion);
  window.addEventListener("keydown", onGlobalKeydown);
  window.addEventListener("shilu:toast", onToast);
  window.addEventListener("shilu:article-status", onArticleStatus);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
  window.removeEventListener("shilu:toast", onToast);
  window.removeEventListener("shilu:article-status", onArticleStatus);
  reducedMotionQuery?.removeEventListener?.("change", updateReducedMotion);
  if (toastTimer) window.clearTimeout(toastTimer);
  cancelWheelSnap();
});
</script>

<template>
  <div class="app-shell" :class="{ 'sidebar-is-collapsed': sidebarCollapsed }">
    <aside class="sidebar" aria-label="主导航">
      <div class="brand-row">
        <button class="brand-mark" type="button" aria-label="拾录首页" title="拾录" @click="router.push('/overview')">
          <img src="/shilu-icon.png" alt="" width="38" height="38" draggable="false" />
        </button>
        <div class="brand-copy"><strong>拾录</strong><span>ShiLu</span></div>
        <button v-if="!sidebarCollapsed" class="icon-button sidebar-toggle" type="button" title="折叠侧边栏" aria-label="折叠侧边栏" @click="sidebarCollapsed = true">
          <ChevronLeft :size="18" />
        </button>
      </div>

      <nav
        class="side-nav arc-nav"
        :class="{ 'arc-nav-reduced': reducedMotion, 'arc-nav-dragging': dragging }"
        aria-label="主导航"
        @wheel="onNavWheel"
        @pointerdown="onNavPointerDown"
        @pointermove="onNavPointerMove"
        @pointerup="onNavPointerUp"
        @pointercancel="onNavPointerUp"
        @lostpointercapture="onNavPointerUp"
        @pointerleave="onNavPointerLeave"
        @keydown="onNavKeydown"
      >
        <span class="arc-nav-orbit" aria-hidden="true" />
        <span class="arc-nav-baseline" aria-hidden="true" />
        <button
          v-for="(item, index) in commands"
          :key="item.to"
          class="nav-item arc-nav-item"
          :class="{ 'arc-nav-item-selected': isNavItemAtBaseline(index) }"
          :style="navItemStyle(index)"
          type="button"
          :title="item.description"
          :aria-label="item.label"
          :data-nav-index="index"
          :tabindex="index === selectedNavIndex ? 0 : -1"
          :aria-selected="index === selectedNavIndex"
          :aria-current="isNavItemCurrent(item) ? 'page' : undefined"
          @click="Date.now() >= suppressClickUntil && activateNavItem(index)"
        >
          <span class="arc-nav-icon"><component :is="item.icon" :size="18" /></span>
          <span class="arc-nav-label">{{ item.label }}</span>
        </button>
      </nav>

      <div class="sidebar-bottom">
        <div v-if="!sidebarCollapsed" class="storage-mini">
          <div class="storage-mini-heading"><span>本地资料库</span><span>{{ libraryLabel }}</span></div>
        </div>
        <button v-else class="icon-button expand-button" type="button" title="展开侧边栏" aria-label="展开侧边栏" @click="sidebarCollapsed = false">
          <ChevronRight :size="18" />
        </button>
      </div>
    </aside>

    <section class="workspace">
      <header class="topbar">
        <div class="topbar-title">
          <span class="topbar-path">{{ routeSection }}</span>
          <h1>{{ routeTitle }}</h1>
        </div>
        <div class="topbar-actions">
          <div class="global-search">
            <label class="search-box" aria-label="快速前往页面">
              <Search :size="17" />
              <input ref="searchInput" v-model="query" type="search" autocomplete="off" placeholder="快速前往页面" @focus="commandOpen = true" @keydown.enter.prevent="matchedCommands[0] && navigate(matchedCommands[0])" />
              <kbd>Ctrl K</kbd>
            </label>
            <div v-if="commandOpen" class="command-menu" role="listbox" aria-label="页面快捷入口">
              <div class="command-menu-label"><Command :size="14" />快速前往</div>
              <button v-for="item in matchedCommands" :key="item.to" type="button" role="option" class="command-option" @mousedown.prevent="navigate(item)">
                <span class="command-option-icon"><component :is="item.icon" :size="16" /></span>
                <span><strong>{{ item.label }}</strong><small>{{ item.description }}</small></span>
                <ChevronRight :size="15" />
              </button>
              <p v-if="!matchedCommands.length" class="command-empty">没有匹配的页面</p>
            </div>
          </div>
          <button class="icon-button" type="button" :title="darkMode ? '切换浅色主题' : '切换深色主题'" :aria-label="darkMode ? '切换浅色主题' : '切换深色主题'" @click="setTheme(!darkMode)">
            <Sun v-if="darkMode" :size="18" /><Moon v-else :size="18" />
          </button>
          <RouterLink to="/settings" class="workspace-status" title="查看设置">
            <span class="workspace-status-icon"><FolderOpen :size="16" /></span>
            <span><strong>本地资料库</strong><small>{{ libraryLabel }}</small></span>
          </RouterLink>
        </div>
      </header>

      <main class="page-content">
        <RouterView v-slot="{ Component, route: childRoute }">
          <Transition name="page-fade" mode="out-in">
            <component :is="Component" :key="childRoute.fullPath" />
          </Transition>
        </RouterView>
      </main>
    </section>

    <Transition name="toast-fade">
      <div v-if="toast" class="toast" role="status"><span>{{ toast }}</span><button type="button" aria-label="关闭提示" @click="toast = ''"><X :size="15" /></button></div>
    </Transition>
  </div>
</template>
