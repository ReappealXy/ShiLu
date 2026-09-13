import MarkdownIt from "markdown-it";
import DOMPurify from "dompurify";
import { convertFileSrc, isTauri } from "@tauri-apps/api/core";
import { displayPath } from "./paths";

export function articleImageUrl(markdownPath: string, relativePath: string): string {
  if (!/^images\/[a-zA-Z0-9_.-]+\.(png|jpe?g|webp)$/i.test(relativePath)) return "";
  const directory = displayPath(markdownPath).replaceAll("\\", "/").replace(/\/[^/]*$/, "");
  return isTauri() ? convertFileSrc(`${directory}/${relativePath}`) : `${directory}/${relativePath}`;
}

/**
 * Remove wrappers that are useful in an OCR response but are not article content.
 * OCR providers sometimes return fenced Markdown, blockquote prefixes, or the
 * synthetic per-image/section headings that the capture pipeline used to add.
 */
export function cleanOcrText(text: string): string {
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  const cleaned: string[] = [];
  let previousBlank = false;
  for (const rawLine of lines) {
    const normalized = rawLine.replace(/^\s*>\s?/, "").trimEnd();
    const line = normalized.trim();
    if (/^```(?:markdown|md)?$/i.test(line) || line === "```") continue;
    if (/^#{1,6}\s*(?:图片\s*\d+|正文|我的备注)\s*$/i.test(line)) continue;
    if (/^---+$/.test(line) && cleaned.length === 0) continue;
    if (!normalized.trim()) {
      if (!previousBlank && cleaned.length) cleaned.push("");
      previousBlank = true;
      continue;
    }
    cleaned.push(normalized);
    previousBlank = false;
  }
  return cleaned.join("\n").trim();
}

/** Infer a likely title from a provider response when the capture title was left blank. */
export function inferOcrTitle(text: string): string {
  const lines = text.replace(/\r\n?/g, "\n").split("\n").map(line => line.trim()).filter(Boolean);
  if (lines.length < 2) return "";
  const candidate = lines[0].replace(/^#{1,6}\s+/, "").replace(/^[-*•]\s+/, "").trim();
  if (!candidate || candidate.length > 80) return "";
  if (/[。！？!?，,；;：:。]$/.test(candidate)) return "";
  return candidate;
}

export function removeOcrTitle(text: string, title: string): string {
  if (!title.trim()) return text;
  const lines = text.split("\n");
  const first = lines.findIndex(line => line.trim());
  if (first < 0) return text;
  const normalized = lines[first].trim().replace(/^#{1,6}\s+/, "").replace(/^[-*•]\s+/, "").trim();
  if (normalized !== title.trim()) return text;
  lines.splice(first, 1);
  while (lines[first] !== undefined && !lines[first].trim()) lines.splice(first, 1);
  return lines.join("\n").trim();
}

export function cleanDisplayMarkdown(text: string, title = ""): string {
  return removeOcrTitle(cleanOcrText(text), title);
}

export function renderArticleMarkdown(text: string, markdownPath: string): string {
  const markdown = new MarkdownIt({ html: false, linkify: true, breaks: true });
  const renderImage = markdown.renderer.rules.image!;
  markdown.renderer.rules.image = (tokens, index, options, env, renderer) => {
    const token = tokens[index];
    const source = String(token.attrGet("src") ?? "");
    const resolved = articleImageUrl(markdownPath, source);
    if (!resolved) return `<span class="image-unavailable">${markdown.utils.escapeHtml(String(token.content || "图片路径不可用"))}</span>`;
    token.attrSet("src", resolved);
    token.attrSet("loading", "lazy");
    return String(renderImage(tokens, index, options, env, renderer));
  };
  const renderLink = markdown.renderer.rules.link_open ?? ((tokens, index, options, _env, renderer) => renderer.renderToken(tokens, index, options));
  markdown.renderer.rules.link_open = (tokens, index, options, env, renderer) => {
    tokens[index].attrSet("target", "_blank");
    tokens[index].attrSet("rel", "noopener noreferrer");
    return renderLink(tokens, index, options, env, renderer);
  };
  return DOMPurify.sanitize(markdown.render(text), {
    ADD_ATTR: ["target"],
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|asset):|[^a-z]|[a-z+.-]+(?:[^a-z+.-:]|$))/i,
  });
}

export function localImageReferences(text: string): string[] {
  const markdown = new MarkdownIt({ html: false });
  return markdown.parse(text, {}).flatMap(token => (token.children ?? [])
    .filter(child => child.type === "image")
    .map(child => String(child.attrGet("src") ?? ""))
    .filter(src => src.startsWith("images/")));
}
