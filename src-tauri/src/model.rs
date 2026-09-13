use crate::storage::{
    app_config_path, read_app_config, write_app_config, AppSettings, StorageError, StorageResult,
    APP_CONFIG_LOCK,
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
    validate_list_config(config)?;
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
    let url = reqwest::Url::parse(config.base_url.trim()).map_err(|_| {
        StorageError::new(
            "MODEL_CONFIG_INVALID",
            "API 地址无效，请填写完整的 http:// 或 https:// 地址。",
        )
    })?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(StorageError::new(
            "MODEL_CONFIG_INVALID",
            "API 地址必须是 HTTP(S) 服务地址，不能包含账号、查询参数或片段。",
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
    validate_list_config(config)?;
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
    validate_config(config)?;
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
    decode_response(status.as_u16(), &text, &config.api_key)
}

fn decode_response(status: u16, text: &str, api_key: &str) -> StorageResult<serde_json::Value> {
    let parsed = serde_json::from_str::<serde_json::Value>(text);
    if !(200..300).contains(&status) {
        let detail = parsed
            .as_ref()
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
            })
            .unwrap_or("");
        let hint = match status {
            401 => "API Key 无效或已过期，请检查 Key。",
            403 => "当前 Key 没有访问权限，请检查账户权限。",
            404 => "接口或模型不存在，请检查 API 地址和模型名称。",
            429 => "请求过于频繁或额度不足，请稍后重试并检查余额。",
            500..=599 => "模型服务暂时异常，请稍后重试。",
            _ => "请检查配置后重试。",
        };
        let detail = if api_key.trim().is_empty() {
            detail.to_string()
        } else {
            detail.replace(api_key.trim(), "[已隐藏 Key]")
        };
        let detail: String = detail.chars().take(500).collect();
        return Err(StorageError::new(
            "MODEL_API_ERROR",
            format!("HTTP {status}：{hint} {detail}"),
        ));
    }
    parsed.map_err(|_| {
        StorageError::new(
            "MODEL_RESPONSE_ERROR",
            format!("接口返回的不是有效 JSON（HTTP {status}），请检查 API 地址。"),
        )
    })
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
    let _guard = APP_CONFIG_LOCK
        .lock()
        .map_err(|_| StorageError::new("CONFIG_BUSY", "设置暂时不可用，请重试。"))?;
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
pub fn save_single_model_settings(
    app: AppHandle,
    kind: String,
    model: ModelConfig,
) -> StorageResult<AppSettings> {
    let _guard = APP_CONFIG_LOCK
        .lock()
        .map_err(|_| StorageError::new("CONFIG_BUSY", "设置暂时不可用，请重试。"))?;
    save_single_model_at(&app_config_path(&app)?, &kind, model)
}

fn save_single_model_at(
    path: &std::path::Path,
    kind: &str,
    model: ModelConfig,
) -> StorageResult<AppSettings> {
    validate_kind(kind)?;
    if model.enabled {
        validate_config(&model)?;
    }
    let mut config = read_app_config(path)?;
    if kind == "ocr" {
        config.ocr_model = Some(model);
    } else {
        config.polish_model = Some(model);
    }
    write_app_config(path, &config)?;
    Ok(AppSettings {
        theme: config.theme,
        ocr_model: config.ocr_model,
        polish_model: config.polish_model,
    })
}

fn validate_kind(kind: &str) -> StorageResult<()> {
    if matches!(kind, "ocr" | "polish") {
        return Ok(());
    }
    Err(StorageError::new(
        "MODEL_CONFIG_INVALID",
        "未知的模型类型。",
    ))
}

#[tauri::command]
pub async fn fetch_model_list(model: ModelConfig) -> StorageResult<ModelListResult> {
    let value = send_get_models(&model).await?;
    let mut models: Vec<String> = value
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| StorageError::new("MODEL_RESPONSE_ERROR", "模型列表响应缺少 data 数组。"))?
        .iter()
        .filter_map(|item| {
            item.get("id")
                .and_then(|id| id.as_str())
                .filter(|id| !id.trim().is_empty())
                .map(str::to_string)
        })
        .collect();
    models.sort();
    models.dedup();
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
    decode_response(status.as_u16(), &text, &config.api_key)
}

#[tauri::command]
pub async fn test_model(kind: Option<String>, model: ModelConfig) -> StorageResult<String> {
    let kind = kind.as_deref().unwrap_or("polish");
    validate_kind(kind)?;
    if kind == "ocr" {
        let text = ocr_with_model(
            model,
            STANDARD.encode(include_bytes!("../fixtures/ocr-test.png")),
        )
        .await?;
        let normalized: String = text
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .flat_map(char::to_uppercase)
            .collect();
        if normalized != "SHILU8246" {
            return Err(StorageError::new("OCR_TEST_MISMATCH", format!("接口已连接，但测试图片文字未正确识别。预期：SHILU 8246；实际：{}。请确认所选模型支持图片输入。", text.chars().take(300).collect::<String>())));
        }
        Ok(text)
    } else {
        polish_with_model(
            model,
            "今天我整理了几个网页里面的资料，把它们保存起来，以后我可以方便查找。".to_string(),
        )
        .await
    }
}

#[tauri::command]
pub async fn ocr_with_model(model: ModelConfig, image_base64: String) -> StorageResult<String> {
    let config = model;
    let image = image_base64.trim();
    let image = if image.starts_with("data:") {
        image.split_once(',').map(|(_, data)| data).unwrap_or("")
    } else {
        image
    };
    if image.len() > crate::clipboard::MAX_IMAGE_BYTES as usize * 4 / 3 + 8 {
        return Err(StorageError::new(
            "MODEL_INPUT_INVALID",
            "待识别图片超过 20 MiB，请缩小图片后重试。",
        ));
    }
    let bytes = STANDARD
        .decode(image)
        .map_err(|_| StorageError::new("MODEL_INPUT_INVALID", "图片数据无效，请重新添加图片。"))?;
    let format = image::guess_format(&bytes)
        .map_err(|_| StorageError::new("MODEL_INPUT_INVALID", "图片数据无效，请重新添加图片。"))?;
    let mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::WebP => "image/webp",
        _ => {
            return Err(StorageError::new(
                "MODEL_INPUT_INVALID",
                "仅支持 PNG、JPEG 和 WebP 图片。",
            ))
        }
    };
    crate::clipboard::validate_image_bytes(&bytes, format)?;
    let value = send_json(&config, endpoint(&config.base_url, "/chat/completions"), serde_json::json!({
        "model": config.model, "messages": [{"role":"user", "content":[{"type":"text","text":"请准确提取图片中的文字，使用 Markdown 保留标题、列表、段落等结构，仅返回识别到的文字，不要解释、补充内容或用代码围栏包裹全文。"},{"type":"image_url","image_url":{"url":format!("data:{mime};base64,{}", STANDARD.encode(bytes))}}]}], "max_tokens": 4096
    })).await?;
    extract_content(&value)
        .ok_or_else(|| StorageError::new("MODEL_RESPONSE_ERROR", "OCR 模型没有返回文字。"))
}

/// Only explicitly tagged source screenshots participate in OCR; illustrations do not.
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
    let document = crate::storage::read_article_at(&root, &article_reference)?;
    let paths = crate::storage::resolve_source_image_paths(&article_path, &document.source_images)?;
    if paths.is_empty() {
        return Err(StorageError::new(
            "NO_ARTICLE_IMAGES",
            "这条资料没有标记为待识别的截图；正文配图不会参与 OCR。",
        ));
    }
    let mut blocks = Vec::with_capacity(paths.len());
    for path in &paths {
        let bytes = fs::read(path).map_err(|e| {
            StorageError::new("OCR_IMAGE_READ_FAILED", format!("无法读取图片：{e}"))
        })?;
        let encoded = STANDARD.encode(bytes);
        let text = ocr_with_model(config.clone(), encoded).await?;
        let text = clean_ocr_text(&text);
        if !text.is_empty() {
            blocks.push(text);
        }
    }
    let text = blocks.join("\n\n");
    if text.is_empty() {
        return Err(StorageError::new(
            "MODEL_RESPONSE_ERROR",
            "OCR 模型没有返回可编辑文字。",
        ));
    }
    let raw_path = article_path.join("raw").join("ocr.txt");
    crate::storage::write_text_atomic(&raw_path, &text)?;
    Ok(crate::storage::OcrResult {
        article_id,
        text,
        image_count: paths.len(),
        raw_path: raw_path.to_string_lossy().to_string(),
    })
}

fn clean_ocr_text(text: &str) -> String {
    let mut cleaned = Vec::new();
    let mut previous_blank = false;
    for raw_line in text.replace("\r\n", "\n").replace('\r', "\n").lines() {
        let normalized = raw_line
            .trim_start()
            .strip_prefix('>')
            .map_or(raw_line, |line| line.strip_prefix(' ').unwrap_or(line))
            .trim_end();
        let line = normalized.trim();
        if line == "```"
            || line.eq_ignore_ascii_case("```md")
            || line.eq_ignore_ascii_case("```markdown")
            || synthetic_ocr_heading(line)
        {
            continue;
        }
        if line.is_empty() {
            if !previous_blank && !cleaned.is_empty() {
                cleaned.push(String::new());
            }
            previous_blank = true;
        } else {
            cleaned.push(normalized.to_string());
            previous_blank = false;
        }
    }
    while cleaned.last().is_some_and(String::is_empty) {
        cleaned.pop();
    }
    cleaned.join("\n")
}

fn synthetic_ocr_heading(line: &str) -> bool {
    let heading = line.trim_start_matches('#').trim();
    line.starts_with('#')
        && (matches!(heading, "正文" | "我的备注")
            || heading.strip_prefix("图片").is_some_and(|number| {
                !number.trim().is_empty() && number.trim().chars().all(|ch| ch.is_ascii_digit())
            }))
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
        "model": config.model, "messages": [{"role":"user", "content":format!("请在不改变事实的前提下，整理并润色以下 Markdown 正文。保留所有链接、图片引用及其相对路径，不删减事实，不用代码围栏包裹全文，只输出润色后的 Markdown：\n\n{}", text)}], "max_tokens": 4096
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
    content
        .as_str()
        .map(str::to_string)
        .or_else(|| {
            content.as_array().map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("")
            })
        })
        .filter(|text| !text.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn mock_server(
        status: u16,
        response: serde_json::Value,
    ) -> (ModelConfig, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut data = Vec::new();
            let mut buffer = [0; 8192];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0, "request ended before headers and body");
                data.extend_from_slice(&buffer[..count]);
                if let Some(end) = data.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let body = response.to_string();
            write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            String::from_utf8(data).unwrap()
        });
        (
            ModelConfig {
                enabled: true,
                base_url: format!("http://{address}"),
                api_key: "test-only-key".to_string(),
                model: "selected-test-model".to_string(),
            },
            handle,
        )
    }

    fn completion(text: &str) -> serde_json::Value {
        serde_json::json!({"choices":[{"message":{"content":text}}]})
    }

    fn body(request: &str) -> serde_json::Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    #[test]
    fn lists_models_without_a_selected_model_and_deduplicates() {
        let (mut config, request) = mock_server(
            200,
            serde_json::json!({"data":[{"id":"beta"},{"id":"alpha"},{"id":"beta"},{"id":""},{}]}),
        );
        config.model.clear();
        let models = tauri::async_runtime::block_on(fetch_model_list(config))
            .unwrap()
            .models;
        assert_eq!(models, vec!["alpha", "beta"]);
        let request = request.join().unwrap();
        assert!(request.starts_with("GET /v1/models "));
        assert!(request
            .to_lowercase()
            .contains("authorization: bearer test-only-key"));
    }

    #[test]
    fn requires_model_for_test_but_not_list() {
        let config = ModelConfig {
            base_url: "http://127.0.0.1:1".into(),
            api_key: "test-key".into(),
            ..Default::default()
        };
        assert!(validate_list_config(&config).is_ok());
        assert_eq!(
            validate_config(&config).unwrap_err().code,
            "MODEL_CONFIG_INVALID"
        );
        assert_eq!(
            tauri::async_runtime::block_on(test_model(Some("polish".into()), config))
                .unwrap_err()
                .code,
            "MODEL_CONFIG_INVALID"
        );
    }

    #[test]
    fn ocr_test_sends_real_image_without_answer_in_prompt() {
        let (config, request) = mock_server(200, completion("SHILU 8246"));
        assert_eq!(
            tauri::async_runtime::block_on(test_model(Some("ocr".into()), config)).unwrap(),
            "SHILU 8246"
        );
        let value = body(&request.join().unwrap());
        assert_eq!(value["model"], "selected-test-model");
        let prompt = value["messages"][0]["content"][0]["text"].as_str().unwrap();
        assert!(!prompt.contains("8246") && !prompt.contains("SHILU"));
        let data_url = value["messages"][0]["content"][1]["image_url"]["url"]
            .as_str()
            .unwrap();
        let image = image::load_from_memory(
            &STANDARD
                .decode(data_url.split_once(',').unwrap().1)
                .unwrap(),
        )
        .unwrap();
        assert_eq!((image.width(), image.height()), (640, 160));
    }

    #[test]
    fn ocr_test_rejects_a_generic_connection_reply() {
        let (config, request) = mock_server(200, completion("连接成功"));
        let error =
            tauri::async_runtime::block_on(test_model(Some("ocr".into()), config)).unwrap_err();
        assert_eq!(error.code, "OCR_TEST_MISMATCH");
        request.join().unwrap();
    }

    #[test]
    fn polish_sends_current_markdown_and_preserves_returned_content() {
        let (config, request) = mock_server(200, completion("润色后的正文"));
        let text = "我刚刚修改的内容\n\n![说明](images/002.jpg)";
        assert_eq!(
            tauri::async_runtime::block_on(polish_with_model(config, text.into())).unwrap(),
            "润色后的正文"
        );
        let value = body(&request.join().unwrap());
        assert_eq!(value["model"], "selected-test-model");
        assert!(value["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains(text));
    }

    #[test]
    fn ocr_uses_real_jpeg_mime_even_with_incorrect_data_url_prefix() {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(4, 4)
            .write_to(&mut bytes, image::ImageFormat::Jpeg)
            .unwrap();
        let (config, request) = mock_server(200, completion("识别文字"));
        let encoded = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(bytes.into_inner())
        );
        tauri::async_runtime::block_on(ocr_with_model(config, encoded)).unwrap();
        let value = body(&request.join().unwrap());
        assert!(value["messages"][0]["content"][1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/jpeg;base64,"));
    }

    #[test]
    fn cleans_ocr_transport_wrappers_without_removing_article_structure() {
        let text = "```markdown\r\n## 图片 1\r\n\r\n> 作者基于真实语料。\r\n> 第二段。\r\n\r\n## 内容小节\r\n- 保留列表\r\n```";
        assert_eq!(
            clean_ocr_text(text),
            "作者基于真实语料。\n第二段。\n\n## 内容小节\n- 保留列表"
        );
    }

    #[test]
    fn http_failures_are_actionable_and_hide_keys() {
        let (config, request) = mock_server(
            401,
            serde_json::json!({"error":{"message":"bad token test-only-key"}}),
        );
        let error = tauri::async_runtime::block_on(fetch_model_list(config)).unwrap_err();
        assert!(error.message.contains("401") && error.message.contains("API Key"));
        assert!(!error.message.contains("test-only-key"));
        request.join().unwrap();
        let error = decode_response(503, "<html>error</html>", "test-key").unwrap_err();
        assert!(error.message.contains("稍后重试"));
        assert_eq!(
            decode_response(200, "not json", "test-key")
                .unwrap_err()
                .code,
            "MODEL_RESPONSE_ERROR"
        );
        assert!(extract_content(&completion(" \n ")).is_none());
    }

    #[test]
    fn saving_one_model_preserves_other_model_theme_and_library() {
        let directory = std::env::temp_dir().join(format!(
            "shilu-single-model-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("library.json");
        fs::write(&path, serde_json::json!({"libraryPath":"D:/ExistingLibrary", "theme":"dark", "polishModel":{"enabled":false,"model":"existing-polish","baseUrl":"","apiKey":""}}).to_string()).unwrap();
        save_single_model_at(
            &path,
            "ocr",
            ModelConfig {
                model: "new-ocr".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["polishModel"]["model"], "existing-polish");
        assert_eq!(value["ocrModel"]["model"], "new-ocr");
        assert_eq!(value["theme"], "dark");
        assert_eq!(value["libraryPath"], "D:/ExistingLibrary");
        assert!(save_single_model_at(&path, "unknown", ModelConfig::default()).is_err());
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&directory).unwrap();
    }
}
