import { invoke } from "@tauri-apps/api/core";

export type ModelKind = "ocr" | "polish";

/** OpenAI-compatible endpoint configuration. The model list is kept in the view. */
export type ModelConfig = {
  enabled: boolean;
  baseUrl: string;
  apiKey: string;
  model: string;
};

export type ModelSettings = {
  theme?: string;
  ocrModel?: ModelConfig | null;
  polishModel?: ModelConfig | null;
};

export type ModelListResult = { models: string[] };

export function getModelSettings(): Promise<ModelSettings> {
  return invoke<ModelSettings>("get_app_settings");
}

export function saveModelSettings(ocrModel: ModelConfig, polishModel: ModelConfig): Promise<ModelSettings> {
  return invoke<ModelSettings>("save_model_settings", { ocrModel, polishModel });
}

export function fetchModelList(config: ModelConfig): Promise<ModelListResult> {
  return invoke<ModelListResult>("fetch_model_list", { model: config });
}

export function testModel(kind: ModelKind, config: ModelConfig): Promise<void> {
  return invoke<string>("test_model", { kind, model: config }).then(() => undefined);
}

export function ocrWithModel(model: ModelConfig, imageBase64: string): Promise<string> {
  return invoke<string>("ocr_with_model", { model, imageBase64 });
}

export function ocrArticleWithModel(articleReference: string): Promise<{ text: string; imageCount: number; rawPath: string; articleId: string }> {
  return invoke("ocr_article_with_model", { articleReference });
}

export function polishWithModel(model: ModelConfig, text: string): Promise<string> {
  return invoke<string>("polish_with_model", { model, text });
}
