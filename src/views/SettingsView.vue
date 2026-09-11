<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  CheckCircle2,
  ExternalLink,
  FilePlus2,
  FolderOpen,
  KeyRound,
  MonitorCog,
  Settings2,
} from "@lucide/vue";
import {
  chooseLibraryDirectory,
  createArticleSkeleton,
  getErrorMessage,
  getLibraryStatus,
  initializeLibrary,
  migrateLibrary,
  type LibraryStatus,
} from "../services/library";

const libraryStatus = ref<LibraryStatus | null>(null);
const loadingStatus = ref(true);
const initializing = ref(false);
const migrating = ref(false);
const creatingTestArticle = ref(false);
const errorMessage = ref("");
const successMessage = ref("");

const libraryIsReady = computed(() => libraryStatus.value?.configured && libraryStatus.value.ready);

async function loadLibraryStatus() {
  loadingStatus.value = true;
  errorMessage.value = "";

  try {
    libraryStatus.value = await getLibraryStatus();
  } catch (error) {
    libraryStatus.value = null;
    errorMessage.value = getErrorMessage(error, "无法读取资料库状态，请稍后重试。");
  } finally {
    loadingStatus.value = false;
  }
}

async function selectLibraryDirectory() {
  errorMessage.value = "";
  successMessage.value = "";

  try {
    const libraryPath = await chooseLibraryDirectory();
    if (!libraryPath) return;

    if (libraryStatus.value?.configured && libraryStatus.value.libraryPath) {
      migrating.value = true;
      libraryStatus.value = await migrateLibrary(libraryPath);
      successMessage.value = "资料库已迁移完成，旧资料也已移动到新位置。";
    } else {
      initializing.value = true;
      libraryStatus.value = await initializeLibrary(libraryPath);
      successMessage.value = "资料库已准备完成，之后的资料会保存在这里。";
    }
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "无法初始化这个资料库文件夹，请选择其他位置后重试。");
  } finally {
    initializing.value = false;
    migrating.value = false;
  }
}

async function openLibraryInExplorer() {
  const libraryPath = libraryStatus.value?.libraryPath;
  if (!libraryPath) return;
  try {
    const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
    await revealItemInDir(libraryPath);
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "无法打开资料库文件夹。");
  }
}

async function createTestArticle() {
  errorMessage.value = "";
  successMessage.value = "";
  creatingTestArticle.value = true;

  try {
    const article = await createArticleSkeleton("测试资料");
    libraryStatus.value = await getLibraryStatus();
    successMessage.value = `已创建测试资料：${article.folderName}`;
  } catch (error) {
    errorMessage.value = getErrorMessage(error, "无法创建测试资料，请检查资料库后重试。");
  } finally {
    creatingTestArticle.value = false;
  }
}

onMounted(loadLibraryStatus);
</script>

<template>
  <section class="page-view settings-view" aria-labelledby="settings-heading">
    <div class="page-heading">
      <div>
        <p class="page-eyebrow">工作区偏好</p>
        <h2 id="settings-heading">设置</h2>
        <p class="page-description">在这里选择本地资料库。你的 Markdown、图片和原始识别结果都会保存在这个文件夹内。</p>
      </div>
    </div>

    <div class="settings-list" aria-label="设置项目">
      <section class="content-panel settings-card" aria-labelledby="library-settings-heading">
        <span class="settings-card-icon"><FolderOpen :size="20" aria-hidden="true" /></span>
        <div class="settings-card-content">
          <div class="settings-card-heading">
            <h3 id="library-settings-heading">本地资料库</h3>
            <span v-if="loadingStatus" class="status-badge">正在读取</span>
            <span v-else-if="libraryIsReady" class="status-badge status-badge-ready">已准备好</span>
            <span v-else class="status-badge">{{ libraryStatus?.configured ? "需要修复" : "尚未设置" }}</span>
          </div>
          <div v-if="loadingStatus" class="settings-loading-lines" aria-label="正在读取资料库状态">
            <span></span><span></span>
          </div>
          <template v-else-if="libraryStatus?.configured">
            <p>资料库位置</p>
            <code class="library-path">{{ libraryStatus.libraryPath }}</code>
            <p v-if="libraryStatus.ready">目前有 {{ libraryStatus.articleCount }} 条资料。每条资料都会保存为独立的 Markdown 文件夹。</p>
            <p v-else>文件夹结构不完整，缺少：{{ libraryStatus.missingItems.join("、") || "必要文件" }}。</p>
          </template>
          <p v-else>选择一个空文件夹后，拾录会在里面建立 `articles`、设置文件和搜索索引。更换已有资料库时，软件会先完整复制资料，确认成功后再清理旧位置。</p>
          <div class="settings-card-actions">
            <button class="button button-primary" type="button" :disabled="loadingStatus || initializing || migrating" @click="selectLibraryDirectory">
              <FolderOpen :size="17" aria-hidden="true" />
              {{ migrating ? "正在迁移资料库..." : initializing ? "正在准备资料库..." : libraryStatus?.configured ? "更换并迁移资料库" : "选择资料库文件夹" }}
            </button>
            <button v-if="libraryStatus?.configured" class="button button-secondary" type="button" :disabled="migrating" @click="openLibraryInExplorer">
              <ExternalLink :size="17" aria-hidden="true" />
              在资源管理器中打开
            </button>
            <button v-if="libraryIsReady" class="button button-secondary" type="button" :disabled="creatingTestArticle" @click="createTestArticle">
              <FilePlus2 :size="17" aria-hidden="true" />
              {{ creatingTestArticle ? "正在创建测试资料..." : "创建测试资料" }}
            </button>
          </div>
          <p v-if="libraryIsReady" class="settings-action-hint">更换资料库时请选择空文件夹；软件会自动搬迁 Markdown、图片、OCR 原文和设置索引，旧目录不会在迁移完成前删除。</p>
          <p v-if="successMessage" class="settings-feedback settings-feedback-success" role="status">
            <CheckCircle2 :size="16" aria-hidden="true" />
            {{ successMessage }}
          </p>
          <p v-if="errorMessage" class="settings-feedback settings-feedback-error" role="alert">{{ errorMessage }}</p>
        </div>
      </section>

      <section class="content-panel settings-card" aria-labelledby="model-settings-heading">
        <span class="settings-card-icon"><KeyRound :size="20" aria-hidden="true" /></span>
        <div class="settings-card-content">
          <div class="settings-card-heading">
            <h3 id="model-settings-heading">模型连接</h3>
            <span class="status-badge">暂未启用</span>
          </div>
          <p>模型地址、名称和密钥会在视觉识别功能接入后提供配置。</p>
        </div>
      </section>

      <section class="content-panel settings-card" aria-labelledby="appearance-settings-heading">
        <span class="settings-card-icon"><MonitorCog :size="20" aria-hidden="true" /></span>
        <div class="settings-card-content">
          <div class="settings-card-heading">
            <h3 id="appearance-settings-heading">界面外观</h3>
            <span class="status-badge status-badge-ready">可用</span>
          </div>
          <p>可通过顶部的主题按钮切换浅色与深色界面，偏好会保存在本机。</p>
        </div>
      </section>
    </div>

    <div class="settings-footnote">
      <Settings2 :size="16" aria-hidden="true" />
      <span>更多设置会随资料库、识别和编辑功能逐步加入。</span>
    </div>
  </section>
</template>
