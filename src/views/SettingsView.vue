<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  CheckCircle2,
  ExternalLink,
  FilePlus2,
  FolderOpen,
  KeyRound,
  Info,
  LoaderCircle,
  MonitorCog,
  RefreshCw,
  Save,
  Settings2,
  Sparkles,
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
import {
  fetchModelList,
  getModelSettings,
  saveSingleModelSettings,
  testModel,
  type ModelConfig,
} from "../services/model";

const libraryStatus = ref<LibraryStatus | null>(null);
const loadingStatus = ref(true);
const initializing = ref(false);
const migrating = ref(false);
const creatingTestArticle = ref(false);
const errorMessage = ref("");
const successMessage = ref("");

const defaultModel = (): ModelConfig => ({ enabled: false, baseUrl: "", apiKey: "", model: "" });
const modelConfig = ref<ModelConfig>(defaultModel());
const modelOptions = ref<string[]>([]);
const modelLoading = ref(true);
type ModelOperation = "fetch" | "test" | "save";
type OperationState = { pending: boolean; message: string; detail: string; tone: "success" | "error" | "notice" };
const newOperation = (): OperationState => ({ pending: false, message: "", detail: "", tone: "notice" });
const modelOperations = ref<Record<ModelOperation, OperationState>>({ fetch: newOperation(), test: newOperation(), save: newOperation() });
const modelLoadError = ref("");
let active = true;

onUnmounted(() => { active = false; });

function connectionSignature(config: ModelConfig) {
  return JSON.stringify([config.baseUrl.trim(), config.apiKey.trim()]);
}

function configSignature(config: ModelConfig) {
  return JSON.stringify([connectionSignature(config), config.model.trim(), config.enabled]);
}

function availableModels() {
  return modelOptions.value;
}

function startOperation(operation: ModelOperation) {
  const state = modelOperations.value[operation];
  if (state.pending) return null;
  Object.assign(state, { pending: true, message: "", detail: "", tone: "notice" });
  return state;
}

async function loadModelSettings() {
  modelLoading.value = true;
  modelLoadError.value = "";
  try {
    const settings = await getModelSettings();
    if (!active) return;
    // Reuse an older OCR-only configuration when no Markdown model was saved yet.
    modelConfig.value = { ...defaultModel(), ...(settings.polishModel ?? settings.ocrModel ?? {}) };
  } catch (error) {
    if (active) modelLoadError.value = getErrorMessage(error, "无法读取模型设置，请重试。");
  } finally {
    modelLoading.value = false;
  }
}

async function saveModel() {
  const state = startOperation("save");
  if (!state) return;
  const snapshot = { ...modelConfig.value };
  try {
    await saveSingleModelSettings("polish", snapshot);
    if (!active) return;
    const unchanged = configSignature(snapshot) === configSignature(modelConfig.value);
    state.tone = unchanged ? "success" : "notice";
    state.message = unchanged ? "Markdown 整理模型配置已保存。" : "已保存点击时的配置；之后的修改尚未保存。";
  } catch (error) {
    if (!active) return;
    state.tone = "error";
    state.message = getErrorMessage(error, "保存失败，输入内容已保留，请重试。");
  } finally {
    state.pending = false;
  }
}

async function fetchModels() {
  const state = startOperation("fetch");
  if (!state) return;
  const snapshot = { ...modelConfig.value };
  try {
    const result = await fetchModelList(snapshot);
    if (!active) return;
    if (connectionSignature(snapshot) !== connectionSignature(modelConfig.value)) {
      state.message = "地址或 Key 已变更，旧连接的列表未应用，请重新获取。";
      return;
    }
    modelOptions.value = [...new Set(result.models.filter(model => model.trim()))].sort();
    state.tone = modelOptions.value.length ? "success" : "notice";
    state.message = modelOptions.value.length ? `已获取 ${modelOptions.value.length} 个模型。` : "接口返回空列表，请手动填写模型名称。";
  } catch (error) {
    if (!active) return;
    state.tone = "error";
    state.message = `获取列表失败：${getErrorMessage(error, "请检查地址和 Key。") } 可重试或手动填写模型名称。`;
  } finally {
    state.pending = false;
  }
}

async function testConfiguredModel() {
  const state = startOperation("test");
  if (!state) return;
  const snapshot = { ...modelConfig.value };
  try {
    const result = await testModel("polish", snapshot);
    if (!active) return;
    const unchanged = configSignature(snapshot) === configSignature(modelConfig.value);
    state.tone = unchanged ? "success" : "notice";
    state.message = `${snapshot.model || "所选模型"} Markdown 整理测试成功。${unchanged ? "" : "配置已变更，此结果仅对应测试时的配置。"}`;
    state.detail = result;
  } catch (error) {
    if (!active) return;
    state.tone = "error";
    state.message = `${snapshot.model || "Markdown 整理模型"} 测试失败：${getErrorMessage(error, "请检查地址、Key 和所选模型后重试。")}`;
  } finally {
    state.pending = false;
  }
}

const libraryIsReady = computed(() => libraryStatus.value?.configured && libraryStatus.value.ready);
const displayLibraryPath = computed(() => {
  const value = libraryStatus.value?.libraryPath ?? "";
  if (value.startsWith("\\\\?\\UNC\\")) return `\\\\${value.slice(8)}`;
  if (value.startsWith("\\\\?\\")) return value.slice(4);
  return value;
});

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

onMounted(() => {
  void loadLibraryStatus();
  void loadModelSettings();
});
</script>

<template>
  <section class="page-view settings-view" aria-labelledby="settings-heading">
    <div class="page-heading">
      <div>
        <p class="page-eyebrow">工作区偏好</p>
        <h2 id="settings-heading">设置</h2>
        <p class="page-description">在这里选择本地资料库。你的 Markdown 正文和配图都会保存在这个文件夹内。</p>
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
            <code class="library-path">{{ displayLibraryPath }}</code>
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
          <p v-if="libraryIsReady" class="settings-action-hint">更换资料库时请选择空文件夹；软件会自动搬迁 Markdown、图片和设置索引，旧目录不会在迁移完成前删除。</p>
          <p v-if="successMessage" class="settings-feedback settings-feedback-success" role="status">
            <CheckCircle2 :size="16" aria-hidden="true" />
            {{ successMessage }}
          </p>
          <p v-if="errorMessage" class="settings-feedback settings-feedback-error" role="alert">{{ errorMessage }}</p>
        </div>
      </section>

      <section class="content-panel settings-card model-settings-card" aria-labelledby="model-settings-heading">
        <span class="settings-card-icon"><KeyRound :size="20" aria-hidden="true" /></span>
        <div class="settings-card-content">
          <div class="settings-card-heading">
            <div>
              <h3 id="model-settings-heading">Markdown 整理模型</h3>
              <p class="settings-card-subtitle">用于把你粘贴的正文整理成结构清晰的 Markdown，保留事实、链接和配图引用。</p>
            </div>
            <span v-if="modelLoading" class="status-badge">正在读取</span>
            <span v-else class="status-badge status-badge-ready">可配置</span>
          </div>

          <p class="settings-action-hint">获取列表只需地址和 Key；选择模型后即可测试，无须先保存。获取列表、测试和保存可以独立进行。</p>
          <div v-if="modelLoading" class="settings-loading-lines" aria-label="正在读取模型设置"><span></span><span></span></div>
          <div v-else-if="!modelLoadError" class="model-config-grid">
            <section class="model-config-panel" aria-labelledby="markdown-model-heading">
              <div class="model-config-heading">
                <div class="model-config-title"><Sparkles :size="17" /><h4 id="markdown-model-heading">Markdown 整理模型</h4></div>
                <label class="model-switch"><input v-model="modelConfig.enabled" type="checkbox" /><span>{{ modelConfig.enabled ? "已启用" : "未启用" }}</span></label>
              </div>
              <p class="model-config-description">AI 会根据原文整理标题、段落、列表等 Markdown 格式，不擅自补充事实。</p>
              <div class="model-fields">
                <label class="model-field model-field-wide"><span>API 地址</span><input v-model.trim="modelConfig.baseUrl" type="url" placeholder="https://api.example.com/v1" autocomplete="url" /></label>
                <label class="model-field model-field-wide"><span>API Key</span><input v-model="modelConfig.apiKey" type="text" placeholder="sk-..." autocomplete="off" /></label>
                <label class="model-field model-field-wide"><span>模型</span><select v-if="availableModels().length" v-model="modelConfig.model"><option value="">请选择模型</option><option v-if="modelConfig.model && !availableModels().includes(modelConfig.model)" :value="modelConfig.model">自定义：{{ modelConfig.model }}</option><option v-for="model in availableModels()" :key="model" :value="model">{{ model }}</option></select><input v-else v-model.trim="modelConfig.model" type="text" class="model-manual-input" placeholder="可手动填写，例如：gpt-4o-mini" autocomplete="off" /></label>
              </div>
              <div class="model-actions">
                <button class="button button-secondary" type="button" :disabled="modelOperations.fetch.pending || !modelConfig.baseUrl || !modelConfig.apiKey" @click="fetchModels"><LoaderCircle v-if="modelOperations.fetch.pending" :size="15" class="model-spin" /><RefreshCw v-else :size="15" />{{ modelOperations.fetch.pending ? "获取中..." : "获取模型列表" }}</button>
                <button class="button button-secondary" type="button" :disabled="modelOperations.test.pending || !modelConfig.baseUrl || !modelConfig.apiKey || !modelConfig.model" @click="testConfiguredModel"><LoaderCircle v-if="modelOperations.test.pending" :size="15" class="model-spin" /><CheckCircle2 v-else :size="15" />{{ modelOperations.test.pending ? "测试中..." : "测试模型" }}</button>
                <button class="button button-primary" type="button" :disabled="modelOperations.save.pending" @click="saveModel"><LoaderCircle v-if="modelOperations.save.pending" :size="15" class="model-spin" /><Save v-else :size="15" />{{ modelOperations.save.pending ? "保存中..." : "保存配置" }}</button>
              </div>
              <template v-for="operation in (['fetch', 'test', 'save'] as ModelOperation[])" :key="operation">
                <p v-if="modelOperations[operation].message" class="settings-feedback" :class="modelOperations[operation].tone === 'error' ? 'settings-feedback-error' : modelOperations[operation].tone === 'success' ? 'settings-feedback-success' : 'model-feedback-notice'" role="status"><CheckCircle2 v-if="modelOperations[operation].tone === 'success'" :size="15" /><Info v-else-if="modelOperations[operation].tone === 'notice'" :size="15" />{{ modelOperations[operation].message }}</p>
                <p v-if="modelOperations[operation].detail" class="model-test-result" role="status">接口返回：{{ modelOperations[operation].detail }}</p>
              </template>
            </section>
          </div>
          <p v-if="modelLoadError" class="settings-feedback settings-feedback-error" role="alert">{{ modelLoadError }}</p>
          <button v-if="modelLoadError" class="button button-secondary" type="button" :disabled="modelLoading" @click="loadModelSettings"><RefreshCw :size="15" />重新读取模型配置</button>
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
      <span>更多设置会随资料库和编辑功能逐步加入。</span>
    </div>
  </section>
</template>

<style scoped>
.model-settings-card { align-items: flex-start; }
.settings-card-subtitle { margin-top: 7px !important; }
.model-config-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; margin-top: 16px; }
.model-config-panel { min-width: 0; padding: 15px 0; border-top: 1px solid var(--line); }
.model-config-heading { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
.model-config-title { display: flex; align-items: center; gap: 7px; color: var(--primary); }
.model-config-title h4 { margin: 0; color: var(--ink); font-size: 13px; font-weight: 730; }
.model-switch { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: 11px; cursor: pointer; }
.model-switch input { accent-color: var(--primary); }
.model-config-description { margin-top: 7px !important; font-size: 11px !important; line-height: 1.5 !important; }
.model-fields { display: grid; gap: 9px; margin-top: 12px; }
.model-field { display: grid; gap: 5px; color: var(--muted-strong); font-size: 11px; font-weight: 650; }
.model-field input, .model-field select { width: 100%; min-height: 34px; padding: 0 9px; color: var(--ink); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 7px; outline: 0; font-size: 12px; transition: border-color 130ms ease-out, box-shadow 130ms ease-out; }
.model-field input:focus, .model-field select:focus { border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus); }
.model-field select:disabled { display: none; }
.model-manual-input { display: block !important; }
.model-test-result { margin: 5px 0 0 !important; padding: 7px 9px; color: var(--muted-strong); background: var(--surface); border-radius: 6px; font: 12px/1.5 ui-monospace, SFMono-Regular, Consolas, monospace; overflow-wrap: anywhere; }
.model-feedback-notice { color: var(--muted-strong); }
.model-config-panel .settings-feedback-success { color: oklch(0.46 0.115 150) !important; }
:global([data-theme="dark"] .model-config-panel .settings-feedback-success) { color: var(--success) !important; }
.settings-feedback { overflow-wrap: anywhere; }
.settings-feedback svg { flex-shrink: 0; }
.model-actions { display: flex; flex-wrap: wrap; gap: 7px; margin-top: 13px; }
.model-actions .button { min-height: 32px; padding: 0 9px; font-size: 11px; }
.model-actions .button-primary { margin-left: auto; }
.model-settings-hint { margin-top: 13px !important; }
.model-spin { animation: model-spin 850ms linear infinite; }
@keyframes model-spin { to { transform: rotate(360deg); } }
@media (max-width: 860px) {
  .model-config-grid { grid-template-columns: 1fr; }
}
@media (max-width: 540px) {
  .model-actions .button-primary { margin-left: 0; }
}
@media (prefers-reduced-motion: reduce) {
  .model-spin { animation: none; }
  .model-field input, .model-field select { transition: none; }
}
</style>
