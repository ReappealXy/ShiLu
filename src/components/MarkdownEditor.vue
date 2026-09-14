<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { Bold, ClipboardPaste, Columns2, Eye, Heading2, ImagePlus, Italic, Link2, List, SquarePen } from "@lucide/vue";
import { renderArticleMarkdown } from "../services/markdown";

const props = defineProps<{ modelValue: string; markdownPath: string; disabled?: boolean; previewTitle?: string; previewSourceUrl?: string }>();
const emit = defineEmits<{ "update:modelValue": [value: string]; pasteImage: [event: ClipboardEvent]; pasteImages: []; chooseImages: [] }>();
const mode = ref<"edit" | "split" | "preview">("split");
const input = ref<HTMLTextAreaElement>();
const selection = ref({ start: 0, end: 0 });
const html = computed(() => renderArticleMarkdown(props.modelValue, props.markdownPath));
function rememberSelection() {
  if (input.value) selection.value = { start: input.value.selectionStart, end: input.value.selectionEnd };
}
async function insert(text: string, before = "", after = "") {
  const { start, end } = selection.value;
  emit("update:modelValue", props.modelValue.slice(0, start) + before + text + after + props.modelValue.slice(end));
  selection.value = { start: start + before.length + text.length + after.length, end: start + before.length + text.length + after.length };
  await nextTick();
  input.value?.focus();
  input.value?.setSelectionRange(selection.value.start, selection.value.end);
}
function format(before: string, after = "") {
  rememberSelection();
  void insert(props.modelValue.slice(selection.value.start, selection.value.end), before, after);
}
function onInput(event: Event) {
  emit("update:modelValue", (event.target as HTMLTextAreaElement).value);
  rememberSelection();
}
function onPaste(event: ClipboardEvent) { rememberSelection(); emit("pasteImage", event); }
defineExpose({ insert, rememberSelection });
</script>

<template>
  <section class="markdown-editor" aria-label="Markdown 编辑区">
    <div class="markdown-toolbar">
      <div class="markdown-tools" aria-label="正文工具">
        <button class="icon-button" type="button" title="二级标题" aria-label="二级标题" :disabled="disabled || mode === 'preview'" @click="format('## ')"><Heading2 :size="17" /></button>
        <button class="icon-button" type="button" title="加粗" aria-label="加粗" :disabled="disabled || mode === 'preview'" @click="format('**', '**')"><Bold :size="17" /></button>
        <button class="icon-button" type="button" title="斜体" aria-label="斜体" :disabled="disabled || mode === 'preview'" @click="format('*', '*')"><Italic :size="17" /></button>
        <button class="icon-button" type="button" title="列表" aria-label="列表" :disabled="disabled || mode === 'preview'" @click="format('- ')"><List :size="17" /></button>
        <button class="icon-button" type="button" title="插入链接" aria-label="插入链接" :disabled="disabled || mode === 'preview'" @click="format('[', '](https://)')"><Link2 :size="17" /></button>
        <span class="tool-divider" />
        <button class="icon-button" type="button" title="粘贴配图" aria-label="粘贴配图" :disabled="disabled" @click="emit('pasteImages')"><ClipboardPaste :size="17" /></button>
        <button class="icon-button" type="button" title="选择配图" aria-label="选择配图" :disabled="disabled" @click="emit('chooseImages')"><ImagePlus :size="17" /></button>
      </div>
      <div class="markdown-modes" aria-label="正文视图">
        <button type="button" title="编辑" :aria-pressed="mode === 'edit'" @click="mode = 'edit'"><SquarePen :size="15" />编辑</button>
        <button type="button" title="编辑与预览" :aria-pressed="mode === 'split'" @click="mode = 'split'"><Columns2 :size="15" />对照</button>
        <button type="button" title="预览" :aria-pressed="mode === 'preview'" @click="mode = 'preview'"><Eye :size="15" />预览</button>
      </div>
    </div>
    <div class="markdown-panes" :class="`markdown-${mode}`">
      <textarea v-show="mode !== 'preview'" ref="input" :value="modelValue" :disabled="disabled" class="editor-content" aria-label="Markdown 正文" spellcheck="false" @input="onInput" @select="rememberSelection" @keyup="rememberSelection" @click="rememberSelection" @paste="onPaste" />
      <article v-show="mode !== 'edit'" class="editor-preview markdown-body" aria-label="整篇资料预览" tabindex="0" @paste="onPaste">
        <header v-if="props.previewTitle" class="markdown-preview-header">
          <h1 v-if="props.previewTitle">{{ props.previewTitle }}</h1>
        </header>
        <div v-html="html" />
        <footer v-if="props.previewSourceUrl" class="markdown-preview-source">
          <span>来源：</span>
          <a :href="props.previewSourceUrl" target="_blank" rel="noopener noreferrer">{{ props.previewSourceUrl }}</a>
        </footer>
      </article>
    </div>
  </section>
</template>

<style scoped>
.markdown-editor { min-width: 0; }
.markdown-toolbar, .markdown-tools, .markdown-modes { display: flex; align-items: center; gap: 4px; }
.markdown-toolbar { justify-content: space-between; flex-wrap: wrap; gap: 8px; padding: 10px 0; border-bottom: 1px solid var(--line); }
.markdown-tools { flex-wrap: wrap; }
.markdown-tools .icon-button { width: 34px; height: 34px; }
.tool-divider { width: 1px; height: 20px; margin: 0 5px; background: var(--line); }
.markdown-modes { padding: 3px; border-radius: 6px; background: var(--surface-alt); }
.markdown-modes button { display: flex; align-items: center; gap: 5px; border: 0; border-radius: 4px; padding: 6px 8px; background: transparent; color: var(--muted-strong); font-size: 12px; }
.markdown-modes button[aria-pressed='true'] { color: var(--primary); background: var(--primary-soft); }
.markdown-panes { display: grid; grid-template-columns: minmax(0, 1fr); min-height: 400px; }
.markdown-split { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.markdown-preview-header { margin-bottom: 24px; padding-bottom: 18px; border-bottom: 1px solid var(--line); }
.markdown-preview-header h1 { margin: 0; color: var(--ink); font-size: clamp(22px, 3vw, 32px); line-height: 1.3; overflow-wrap: anywhere; }
.markdown-preview-source { display: flex; max-width: 100%; align-items: baseline; gap: 4px; margin-top: 24px; padding-top: 14px; border-top: 1px solid var(--line); color: var(--muted-strong); font-size: 13px; overflow-wrap: anywhere; }
.markdown-preview-source a { color: var(--primary); overflow-wrap: anywhere; }
.editor-content { width: 100%; min-height: 400px; padding: 20px 16px; resize: vertical; border: 0; border-radius: 0; color: var(--ink); background: var(--surface); font-family: Consolas, monospace; font-size: 14px; line-height: 1.8; }
.editor-content:focus-visible { outline: 2px solid var(--primary); outline-offset: -2px; }
.editor-preview { min-width: 0; padding: 20px; background: var(--surface-alt); }
.markdown-split .editor-preview { border-left: 1px solid var(--line); }
@media (max-width: 1100px) { .markdown-split { grid-template-columns: minmax(0, 1fr); } .markdown-split .editor-preview { border-left: 0; border-top: 1px solid var(--line); } }
</style>
