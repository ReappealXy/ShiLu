use crate::storage::{
    app_config_path, read_app_config, write_app_config, AppSettings, StorageError, StorageResult,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListResult {
    pub models: Vec<String>,
}

fn validate_config(config: &ModelConfig) -> StorageResult<()> {
    if config.base_url.trim().is_empty() {
        return Err(StorageError::new(
            "MODEL_CONFIG_INVALID",
            "请填写 API 地址。",
        ));
    }
    if config.api_key.trim().is_empty() {
        return Err(StorageError::new(
            "MODEL_CONFIG_INVALID",
            "请填写 API Key。",
        ));
    }
    if config.model.trim().is_empty() {
        return Err(StorageError::new(
            "MODEL_CONFIG_INVALID",
            "请选择或填写模型名称。",
        ));
    }
    Ok(())
}

fn validate_list_config(config: &ModelConfig) -> StorageResult<()> {
    if config.base_url.trim().is_empty() {
        return Err(StorageError::new(
            "MODEL_CONFIG_INVALID",
            "请填写 API 地址。",
        ));
    }
    if config.api_key.trim().is_empty() {
        return Err(StorageError::new(
            "MODEL_CONFIG_INVALID",
            "请填写 API Key。",
        ));
    }
    Ok(())
}

fn endpoint(base: &str, suffix: &str) -> String {
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        format!("{trimmed}{suffix}")
    } else {
        format!("{trimmed}/v1{suffix}")
    }
}

fn client(config: &ModelConfig) -> StorageResult<reqwest::Client> {
    validate_config(config)?;
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| StorageError::new("MODEL_CLIENT_ERROR", format!("无法创建网络客户端：{e}")))
}

async fn send_json(
    config: &ModelConfig,
    url: String,
    body: serde_json::Value,
) -> StorageResult<serde_json::Value> {
    let response = client(config)?
        .post(url)
        .headers(auth_headers(config)?)
        .json(&body)
        .send()
        .await
        .map_err(|e| StorageError::new("MODEL_NETWORK_ERROR", format!("请求模型接口失败：{e}")))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| StorageError::new("MODEL_RESPONSE_ERROR", format!("读取模型响应失败：{e}")))?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|_| {
        StorageError::new(
            "MODEL_RESPONSE_ERROR",
            format!("模型返回的不是有效 JSON（HTTP {status}）。"),
        )
    })?;
    if !status.is_success() {
        let detail = value
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or(&text);
        return Err(StorageError::new(
            "MODEL_API_ERROR",
            format!("模型接口返回 HTTP {}：{}", status.as_u16(), detail),
        ));
    }
    Ok(value)
}

fn auth_headers(config: &ModelConfig) -> StorageResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    let token = format!("Bearer {}", config.api_key.trim());
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&token)
            .map_err(|_| StorageError::new("MODEL_CONFIG_INVALID", "API Key 包含非法字符。"))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(headers)
}

#[tauri::command]
pub fn save_model_settings(
    app: AppHandle,
    ocr_model: Option<ModelConfig>,
    polish_model: Option<ModelConfig>,
) -> StorageResult<AppSettings> {
    let path = app_config_path(&app)?;
    let mut config = read_app_config(&path)?;
    config.ocr_model = ocr_model;
    config.polish_model = polish_model;
    write_app_config(&path, &config)?;
    Ok(AppSettings {
        theme: config.theme,
        ocr_model: config.ocr_model,
        polish_model: config.polish_model,
    })
}

#[tauri::command]
pub async fn fetch_model_list(model: ModelConfig) -> StorageResult<ModelListResult> {
    let value = send_get_models(&model).await?;
    let models = value
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| StorageError::new("MODEL_RESPONSE_ERROR", "模型列表响应缺少 data 数组。"))?
        .iter()
        .filter_map(|item| {
            item.get("id")
                .and_then(|id| id.as_str())
                .map(str::to_string)
        })
        .collect();
    Ok(ModelListResult { models })
}

async fn send_get_models(config: &ModelConfig) -> StorageResult<serde_json::Value> {
    validate_list_config(config)?;
    let response = client(config)?
        .get(endpoint(&config.base_url, "/models"))
        .headers(auth_headers(config)?)
        .send()
        .await
        .map_err(|e| StorageError::new("MODEL_NETWORK_ERROR", format!("请求模型列表失败：{e}")))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| StorageError::new("MODEL_RESPONSE_ERROR", format!("读取模型列表失败：{e}")))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| StorageError::new("MODEL_RESPONSE_ERROR", "模型列表返回的不是有效 JSON。"))?;
    if !status.is_success() {
        return Err(StorageError::new(
            "MODEL_API_ERROR",
            format!("获取模型列表失败（HTTP {}）。", status.as_u16()),
        ));
    }
    Ok(value)
}

#[tauri::command]
pub async fn test_model(kind: Option<String>, model: ModelConfig) -> StorageResult<String> {
    let config = model;
    let value = if kind.as_deref() == Some("ocr") {
        let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
        send_json(&config, endpoint(&config.base_url, "/chat/completions"), serde_json::json!({"model": config.model, "messages":[{"role":"user","content":[{"type":"text","text":"请回复：连接成功"},{"type":"image_url","image_url":{"url":format!("data:image/png;base64,{png}")}}]}],"max_tokens":20})).await?
    } else {
        send_json(&config, endpoint(&config.base_url, "/chat/completions"), serde_json::json!({"model": config.model, "messages":[{"role":"user","content":"请只回复：连接成功"}],"max_tokens":20})).await?
    };
    extract_content(&value)
        .ok_or_else(|| StorageError::new("MODEL_RESPONSE_ERROR", "模型响应中没有可读文本。"))
}

#[tauri::command]
pub async fn ocr_with_model(model: ModelConfig, image_base64: String) -> StorageResult<String> {
    let config = model;
    let image = image_base64
        .trim()
        .trim_start_matches("data:image/png;base64,");
    if image.is_empty() {
        return Err(StorageError::new(
            "MODEL_INPUT_INVALID",
            "没有可识别的图片数据。",
        ));
    }
    let value = send_json(&config, endpoint(&config.base_url, "/chat/completions"), serde_json::json!({
        "model": config.model, "messages": [{"role":"user", "content":[{"type":"text","text":"请准确提取图片中的文字，仅返回识别到的文字，不要解释。"},{"type":"image_url","image_url":{"url":format!("data:image/png;base64,{image}")}}]}], "max_tokens": 4096
    })).await?;
    extract_content(&value)
        .ok_or_else(|| StorageError::new("MODEL_RESPONSE_ERROR", "OCR 模型没有返回文字。"))
}

/// OCR every image already stored in an article using the configured vision model.
#[tauri::command]
pub async fn ocr_article_with_model(
    app: AppHandle,
    article_reference: String,
) -> StorageResult<crate::storage::OcrResult> {
    let root = crate::storage::configured_ready_library(&app)?;
    let config = crate::storage::read_app_config(&crate::storage::app_config_path(&app)?)?
        .ocr_model
        .filter(|model| model.enabled)
        .ok_or_else(|| {
            StorageError::new(
                "OCR_MODEL_NOT_CONFIGURED",
                "请先在设置中启用并保存 OCR 模型。",
            )
        })?;
    let (article_path, _folder, article_id) =
        crate::storage::resolve_article_path(&root, &article_reference)?;
    let images_dir = article_path.join("images");
    let mut paths = fs::read_dir(&images_dir)
        .map_err(|e| StorageError::new("OCR_IMAGE_READ_FAILED", format!("无法读取文章图片：{e}")))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| p.is_file())
        .collect::<Vec<_>>();
    paths.sort();
    if paths.is_empty() {
        return Err(StorageError::new(
            "NO_ARTICLE_IMAGES",
            "这条资料还没有图片可供识别。",
        ));
    }
    let mut blocks = Vec::with_capacity(paths.len());
    for path in &paths {
        let bytes = fs::read(path).map_err(|e| {
            StorageError::new("OCR_IMAGE_READ_FAILED", format!("无法读取图片：{e}"))
        })?;
        let encoded = STANDARD.encode(bytes);
        let text = ocr_with_model(config.clone(), encoded).await?;
        blocks.push(format!("## 图片 {}\n\n{}", blocks.len() + 1, text.trim()));
    }
    let text = blocks.join("\n\n");
    let raw_path = article_path.join("raw").join("ocr.txt");
    crate::storage::write_text_atomic(&raw_path, &text)?;
    Ok(crate::storage::OcrResult {
        article_id,
        text,
        image_count: paths.len(),
        raw_path: raw_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn polish_with_model(model: ModelConfig, text: String) -> StorageResult<String> {
    let config = model;
    if text.trim().is_empty() {
        return Err(StorageError::new(
            "MODEL_INPUT_INVALID",
            "没有可润色的文字。",
        ));
    }
    let value = send_json(&config, endpoint(&config.base_url, "/chat/completions"), serde_json::json!({
        "model": config.model, "messages": [{"role":"user", "content":format!("请在不改变事实的前提下，整理并润色以下文字，输出可直接发布的中文文案：\n\n{}", text)}], "max_tokens": 4096
    })).await?;
    extract_content(&value)
        .ok_or_else(|| StorageError::new("MODEL_RESPONSE_ERROR", "润色模型没有返回文字。"))
}

fn extract_content(value: &serde_json::Value) -> Option<String> {
    let content = value
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?;
    content.as_str().map(str::to_string).or_else(|| {
        content.as_array().map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
    })
}
