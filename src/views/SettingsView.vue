<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  CheckCircle2,
  ExternalLink,
  FilePlus2,
  FolderOpen,
  KeyRound,
  LoaderCircle,
  MonitorCog,
  RefreshCw,
  Save,
  ScanText,
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
  saveModelSettings,
  testModel,
  type ModelConfig,
  type ModelKind,
} from "../services/model";

const libraryStatus = ref<LibraryStatus | null>(null);
const loadingStatus = ref(true);
const initializing = ref(false);
const migrating = ref(false);
const creatingTestArticle = ref(false);
const errorMessage = ref("");
const successMessage = ref("");

const defaultModel = (): ModelConfig => ({ enabled: false, baseUrl: "", apiKey: "", model: "" });
const modelConfigs = ref<Record<ModelKind, ModelConfig>>({ ocr: defaultModel(), polish: defaultModel() });
const modelOptions = ref<Record<ModelKind, string[]>>({ ocr: [], polish: [] });
const modelLoading = ref(true);
const modelSaving = ref<ModelKind | null>(null);
const modelTesting = ref<ModelKind | null>(null);
const modelFetching = ref<ModelKind | null>(null);
const modelFeedback = ref<Record<ModelKind, string>>({ ocr: "", polish: "" });
const modelErrors = ref<Record<ModelKind, string>>({ ocr: "", polish: "" });

function modelTitle(kind: ModelKind) {
  return kind === "ocr" ? "OCR 识别模型" : "文案润色模型";
}

function modelDescription(kind: ModelKind) {
  return kind === "ocr" ? "把截图转换成可编辑的原始文字。需要支持图片输入的视觉模型。" : "把 OCR 原文整理、分段并润色成新的文案，不会自动覆盖原文。";
}

function modelConfig(kind: ModelKind) {
  return modelConfigs.value[kind];
}

async function loadModelSettings() {
  modelLoading.value = true;
  try {
    const settings = await getModelSettings();
    modelConfigs.value.ocr = { ...defaultModel(), ...(settings.ocrModel ?? {}) };
    modelConfigs.value.polish = { ...defaultModel(), ...(settings.polishModel ?? {}) };
  } catch (error) {
    modelErrors.value.ocr = getErrorMessage(error, "无法读取模型设置。");
  } finally {
    modelLoading.value = false;
  }
}

async function saveModel(kind: ModelKind) {
  modelErrors.value[kind] = "";
  modelFeedback.value[kind] = "";
  modelSaving.value = kind;
  try {
    const settings = await saveModelSettings({ ...modelConfigs.value.ocr }, { ...modelConfigs.value.polish });
    const saved = kind === "ocr" ? settings.ocrModel : settings.polishModel;
    if (saved) modelConfigs.value[kind] = { ...defaultModel(), ...saved };
    modelFeedback.value[kind] = "已保存模型设置。";
  } catch (error) {
    modelErrors.value[kind] = getErrorMessage(error, "保存失败，请检查配置后重试。");
  } finally {
    modelSaving.value = null;
  }
}

async function fetchModels(kind: ModelKind) {
  modelErrors.value[kind] = "";
  modelFeedback.value[kind] = "";
  modelFetching.value = kind;
  try {
    const result = await fetchModelList({ ...modelConfig(kind) });
    modelOptions.value[kind] = result.models ?? [];
    modelFeedback.value[kind] = result.models?.length ? `已获取 ${result.models.length} 个模型。` : "接口返回为空，可手动填写模型名称。";
  } catch (error) {
    modelErrors.value[kind] = getErrorMessage(error, "获取模型列表失败；你仍可以手动填写模型名称。");
  } finally {
    modelFetching.value = null;
  }
}

async function testConfiguredModel(kind: ModelKind) {
  modelErrors.value[kind] = "";
  modelFeedback.value[kind] = "";
  modelTesting.value = kind;
  try {
    await testModel(kind, { ...modelConfig(kind) });
    modelFeedback.value[kind] = `${modelTitle(kind)}测试成功，可以正常使用。`;
  } catch (error) {
    modelErrors.value[kind] = getErrorMessage(error, "测试失败，请检查地址、Key 和所选模型。");
  } finally {
    modelTesting.value = null;
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
          <p v-if="libraryIsReady" class="settings-action-hint">更换资料库时请选择空文件夹；软件会自动搬迁 Markdown、图片、OCR 原文和设置索引，旧目录不会在迁移完成前删除。</p>
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
              <h3 id="model-settings-heading">模型连接</h3>
              <p class="settings-card-subtitle">两个模型分别负责识别截图和润色文案，可使用不同平台的 OpenAI 兼容接口。</p>
            </div>
            <span v-if="modelLoading" class="status-badge">正在读取</span>
            <span v-else class="status-badge status-badge-ready">可配置</span>
          </div>

          <div v-if="modelLoading" class="settings-loading-lines" aria-label="正在读取模型设置"><span></span><span></span></div>
          <div v-else class="model-config-grid">
            <section v-for="kind in (['ocr', 'polish'] as ModelKind[])" :key="kind" class="model-config-panel" :aria-labelledby="`${kind}-model-heading`">
              <div class="model-config-heading">
                <div class="model-config-title"><ScanText v-if="kind === 'ocr'" :size="17" /><Sparkles v-else :size="17" /><h4 :id="`${kind}-model-heading`">{{ modelTitle(kind) }}</h4></div>
                <label class="model-switch"><input v-model="modelConfigs[kind].enabled" type="checkbox" /><span>{{ modelConfigs[kind].enabled ? "已启用" : "未启用" }}</span></label>
              </div>
              <p class="model-config-description">{{ modelDescription(kind) }}</p>
              <div class="model-fields">
                <label class="model-field model-field-wide"><span>API 地址</span><input v-model.trim="modelConfigs[kind].baseUrl" type="url" placeholder="https://api.example.com/v1" autocomplete="url" /></label>
                <label class="model-field model-field-wide"><span>API Key</span><input v-model="modelConfigs[kind].apiKey" type="text" placeholder="sk-..." autocomplete="off" /></label>
                <label class="model-field model-field-wide"><span>模型</span><select v-model="modelConfigs[kind].model" :disabled="!modelOptions[kind].length"><option value="">{{ modelOptions[kind].length ? "请选择模型" : "请先获取模型列表或手动填写" }}</option><option v-for="model in modelOptions[kind]" :key="model" :value="model">{{ model }}</option></select><input v-if="!modelOptions[kind].length" v-model.trim="modelConfigs[kind].model" type="text" class="model-manual-input" placeholder="例如：gpt-4o-mini" autocomplete="off" /></label>
              </div>
              <div class="model-actions">
                <button class="button button-secondary" type="button" :disabled="modelFetching === kind || modelSaving === kind || modelTesting === kind" @click="fetchModels(kind)"><LoaderCircle v-if="modelFetching === kind" :size="15" class="model-spin" /><RefreshCw v-else :size="15" />{{ modelFetching === kind ? "获取中..." : "获取模型列表" }}</button>
                <button class="button button-secondary" type="button" :disabled="modelTesting === kind || modelSaving === kind || !modelConfigs[kind].baseUrl || !modelConfigs[kind].model" @click="testConfiguredModel(kind)"><LoaderCircle v-if="modelTesting === kind" :size="15" class="model-spin" /><CheckCircle2 v-else :size="15" />{{ modelTesting === kind ? "测试中..." : "测试模型" }}</button>
                <button class="button button-primary" type="button" :disabled="modelSaving === kind || modelTesting === kind" @click="saveModel(kind)"><LoaderCircle v-if="modelSaving === kind" :size="15" class="model-spin" /><Save v-else :size="15" />{{ modelSaving === kind ? "保存中..." : "保存配置" }}</button>
              </div>
              <p v-if="modelFeedback[kind]" class="settings-feedback settings-feedback-success" role="status"><CheckCircle2 :size="15" />{{ modelFeedback[kind] }}</p>
              <p v-if="modelErrors[kind]" class="settings-feedback settings-feedback-error" role="alert">{{ modelErrors[kind] }}</p>
            </section>
          </div>
          <p class="settings-action-hint model-settings-hint">获取模型列表后，可从下拉框选择模型；如果平台不支持模型列表接口，也可以直接填写模型名称。测试只发送内置测试内容，不会写入资料库。</p>
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

<style scoped>
.model-settings-card { align-items: flex-start; }
.settings-card-subtitle { margin-top: 7px !important; }
.model-config-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; margin-top: 16px; }
.model-config-panel { min-width: 0; padding: 15px; background: var(--surface-alt); border: 1px solid var(--line); border-radius: 10px; }
.model-config-heading { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
.model-config-title { display: flex; align-items: center; gap: 7px; color: var(--primary); }
.model-config-title h4 { margin: 0; color: var(--ink); font-size: 13px; font-weight: 730; }
.model-switch { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: 11px; cursor: pointer; }
.model-switch input { accent-color: var(--primary); }
.model-config-description { min-height: 38px; margin-top: 7px !important; font-size: 11px !important; line-height: 1.5 !important; }
.model-fields { display: grid; gap: 9px; margin-top: 12px; }
.model-field { display: grid; gap: 5px; color: var(--muted-strong); font-size: 11px; font-weight: 650; }
.model-field input, .model-field select { width: 100%; min-height: 34px; padding: 0 9px; color: var(--ink); background: var(--surface); border: 1px solid var(--line-strong); border-radius: 7px; outline: 0; font-size: 12px; transition: border-color 130ms ease-out, box-shadow 130ms ease-out; }
.model-field input:focus, .model-field select:focus { border-color: var(--primary); box-shadow: 0 0 0 3px var(--focus); }
.model-field select:disabled { display: none; }
.model-manual-input { display: block !important; }
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
</style>
