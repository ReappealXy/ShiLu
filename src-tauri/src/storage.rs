use crate::clipboard::{decode_clipboard_png, validate_image_bytes, MAX_IMAGE_BYTES};
use chrono::{Local, SecondsFormat};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

const CONFIG_FILE_NAME: &str = "library.json";
const ARTICLES_DIR_NAME: &str = "articles";
const TRASH_DIR_NAME: &str = ".shilu-trash";
const SETTINGS_FILE_NAME: &str = "settings.json";
const SEARCH_INDEX_FILE_NAME: &str = "search-index.json";
const ALLOWED_IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
const MAX_IMPORT_IMAGES: usize = 50;
const MAX_IMPORT_BYTES: u64 = 200 * 1024 * 1024;

static ARTICLE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static IMAGE_IMPORT_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageError {
    pub code: String,
    pub message: String,
}

impl StorageError {
    pub(crate) fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    fn io(action: &str, error: std::io::Error) -> Self {
        Self::new("IO_ERROR", format!("{}：{}", action, error))
    }
}

pub(crate) type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    pub configured: bool,
    pub library_path: Option<String>,
    pub ready: bool,
    pub missing_items: Vec<String>,
    pub article_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleSkeleton {
    pub id: String,
    pub folder_name: String,
    pub folder_path: String,
    pub markdown_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashMoveResult {
    pub folder_name: String,
    pub trash_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedImage {
    pub file_name: String,
    pub original_file_name: String,
    pub path: String,
    pub relative_path: String,
    pub extension: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageImportResult {
    pub article_id: String,
    pub article_folder_name: String,
    pub images: Vec<ImportedImage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ImageSource {
    Path { path: String },
    Clipboard { name: String, base64: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleCreateResult {
    pub article: ArticleSkeleton,
    pub images: ImageImportResult,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleDocument {
    pub id: String,
    pub folder_name: String,
    pub title: String,
    pub summary: String,
    pub source_url: String,
    pub tags: Vec<String>,
    pub content: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
    pub markdown_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleDraft {
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleSummary {
    pub id: String,
    pub folder_name: String,
    pub title: String,
    pub summary: String,
    pub source_url: String,
    pub tags: Vec<String>,
    pub updated_at: String,
    pub markdown_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    pub article_id: String,
    pub text: String,
    pub image_count: usize,
    pub raw_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryConfig {
    #[serde(alias = "library_path")]
    library_path: Option<String>,
    #[serde(default = "default_theme")]
    theme: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
}

#[derive(Debug, Serialize)]
struct LibrarySettings {
    schema_version: u8,
    app_name: &'static str,
}

#[derive(Debug, Serialize)]
struct SearchIndex {
    schema_version: u8,
    articles: Vec<serde_json::Value>,
}

#[tauri::command]
pub fn get_library_status(app: AppHandle) -> StorageResult<LibraryStatus> {
    let config_path = app_config_path(&app)?;
    let Some(library_path) = read_library_config(&config_path)? else {
        return Ok(LibraryStatus {
            configured: false,
            library_path: None,
            ready: false,
            missing_items: Vec::new(),
            article_count: 0,
        });
    };

    let root = PathBuf::from(&library_path);
    let (ready, missing_items, article_count) = inspect_library(&root);
    Ok(LibraryStatus {
        configured: true,
        library_path: Some(library_path),
        ready,
        missing_items,
        article_count,
    })
}

#[tauri::command]
pub fn get_app_settings(app: AppHandle) -> StorageResult<AppSettings> {
    let config_path = app_config_path(&app)?;
    let config = read_app_config(&config_path)?;
    Ok(AppSettings {
        theme: config.theme,
    })
}

#[tauri::command]
pub fn set_theme_preference(app: AppHandle, theme: String) -> StorageResult<AppSettings> {
    let theme = normalize_theme(&theme)?;
    let config_path = app_config_path(&app)?;
    let mut config = read_app_config(&config_path)?;
    config.theme = theme.clone();
    write_json_atomic(&config_path, &config)?;
    Ok(AppSettings { theme })
}

#[tauri::command]
pub fn initialize_library(app: AppHandle, library_path: String) -> StorageResult<LibraryStatus> {
    let root = validate_library_root(Path::new(&library_path))?;
    initialize_library_at(&root)?;

    let config_path = app_config_path(&app)?;
    let absolute_path = path_to_string(&root)?;
    let mut config = read_app_config(&config_path)?;
    config.library_path = Some(absolute_path);
    write_json_atomic(&config_path, &config)?;

    let (ready, missing_items, article_count) = inspect_library(&root);
    Ok(LibraryStatus {
        configured: true,
        library_path: Some(path_to_string(&root)?),
        ready,
        missing_items,
        article_count,
    })
}

#[tauri::command]
pub fn migrate_library(app: AppHandle, library_path: String) -> StorageResult<LibraryStatus> {
    let destination = validate_library_root(Path::new(&library_path))?;
    let config_path = app_config_path(&app)?;
    let Some(current_path) = read_library_config(&config_path)? else {
        initialize_library_at(&destination)?;
        return library_status_for(&destination);
    };

    let source = validate_library_root(Path::new(&current_path))?;
    if source == destination {
        return library_status_for(&source);
    }
    if destination.starts_with(&source) || source.starts_with(&destination) {
        return Err(StorageError::new(
            "INVALID_LIBRARY_MIGRATION",
            "新资料库不能位于旧资料库内部，或反过来包含旧资料库。",
        ));
    }
    if fs::read_dir(&destination)
        .map_err(|error| StorageError::io("无法检查新资料库文件夹", error))?
        .next()
        .is_some()
    {
        return Err(StorageError::new(
            "LIBRARY_DESTINATION_NOT_EMPTY",
            "新资料库文件夹必须为空，避免覆盖已有文件。",
        ));
    }

    if let Err(error) = copy_directory_contents(&source, &destination) {
        let _ = clear_directory_contents(&destination);
        return Err(error);
    }
    let (ready, _, _) = inspect_library(&destination);
    if !ready {
        let _ = clear_directory_contents(&destination);
        return Err(StorageError::new(
            "LIBRARY_MIGRATION_INCOMPLETE",
            "资料库迁移后结构不完整，旧资料仍保留在原位置。",
        ));
    }

    let absolute_destination = path_to_string(&destination)?;
    let mut config = read_app_config(&config_path)?;
    config.library_path = Some(absolute_destination);
    if let Err(error) = write_json_atomic(&config_path, &config) {
        let _ = clear_directory_contents(&destination);
        return Err(error);
    }

    fs::remove_dir_all(&source)
        .map_err(|error| StorageError::io("资料已复制到新位置，但无法清理旧资料库文件夹", error))?;
    library_status_for(&destination)
}

#[tauri::command]
pub fn create_article_skeleton(
    app: AppHandle,
    title: String,
    source_url: Option<String>,
) -> StorageResult<ArticleSkeleton> {
    let title = title.trim();
    if title.is_empty() {
        return Err(StorageError::new("INVALID_TITLE", "资料标题不能为空。"));
    }

    let root = configured_ready_library(&app)?;
    create_article_skeleton_at(&root, title, source_url.as_deref())
}

#[tauri::command]
pub fn move_article_to_trash(
    app: AppHandle,
    folder_name: String,
) -> StorageResult<TrashMoveResult> {
    if !is_valid_article_folder_name(&folder_name) {
        return Err(StorageError::new(
            "INVALID_ARTICLE_FOLDER",
            "文章文件夹名称不合法，无法移动到回收站。",
        ));
    }

    let root = configured_ready_library(&app)?;
    move_article_to_trash_at(&root, &folder_name)
}

#[tauri::command]
pub fn import_article_images(
    app: AppHandle,
    article_reference: String,
    source_paths: Vec<String>,
) -> StorageResult<ImageImportResult> {
    let root = configured_ready_library(&app)?;
    import_article_images_at(&root, &article_reference, &source_paths)
}

#[tauri::command]
pub async fn create_article_with_sources(
    app: AppHandle,
    title: String,
    source_url: Option<String>,
    sources: Vec<ImageSource>,
) -> StorageResult<ArticleCreateResult> {
    let root = configured_ready_library(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        create_article_with_sources_at(&root, &title, source_url.as_deref(), &sources)
    })
    .await
    .map_err(|_| StorageError::new("IMAGE_IMPORT_FAILED", "图片导入任务失败，请重试。"))?
}

#[tauri::command]
pub async fn import_article_sources(
    app: AppHandle,
    article_reference: String,
    sources: Vec<ImageSource>,
) -> StorageResult<ImageImportResult> {
    let root = configured_ready_library(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        import_article_sources_at(&root, &article_reference, &sources)
    })
    .await
    .map_err(|_| StorageError::new("IMAGE_IMPORT_FAILED", "图片导入任务失败，请重试。"))?
}

#[tauri::command]
pub fn read_article(app: AppHandle, article_reference: String) -> StorageResult<ArticleDocument> {
    let root = configured_ready_library(&app)?;
    read_article_at(&root, &article_reference)
}

#[tauri::command]
pub fn save_article(
    app: AppHandle,
    article_reference: String,
    draft: ArticleDraft,
) -> StorageResult<ArticleDocument> {
    let root = configured_ready_library(&app)?;
    save_article_at(&root, &article_reference, draft)
}

#[tauri::command]
pub fn list_articles(app: AppHandle, query: String) -> StorageResult<Vec<ArticleSummary>> {
    let root = configured_ready_library(&app)?;
    list_articles_at(&root, query.trim())
}

#[tauri::command]
pub fn ocr_article_images(app: AppHandle, article_reference: String) -> StorageResult<OcrResult> {
    let root = configured_ready_library(&app)?;
    ocr_article_images_at(&root, &article_reference)
}

fn app_config_path(app: &AppHandle) -> StorageResult<PathBuf> {
    let config_dir = app.path().app_config_dir().map_err(|error| {
        StorageError::new(
            "APP_CONFIG_UNAVAILABLE",
            format!("无法获取应用配置目录：{}", error),
        )
    })?;
    Ok(config_dir.join(CONFIG_FILE_NAME))
}

fn read_library_config(config_path: &Path) -> StorageResult<Option<String>> {
    if !config_path.exists() {
        return Ok(None);
    }

    let config = read_app_config(config_path)?;
    let Some(library_path) = config.library_path else {
        return Ok(None);
    };
    if library_path.trim().is_empty() {
        return Err(StorageError::new(
            "INVALID_APP_CONFIG",
            "资料库配置中缺少资料库路径。",
        ));
    }
    Ok(Some(library_path))
}

fn read_app_config(config_path: &Path) -> StorageResult<LibraryConfig> {
    if !config_path.exists() {
        return Ok(LibraryConfig {
            library_path: None,
            theme: default_theme(),
        });
    }
    let content = fs::read_to_string(config_path)
        .map_err(|error| StorageError::io("无法读取应用配置", error))?;
    serde_json::from_str(&content).map_err(|error| {
        StorageError::new("INVALID_APP_CONFIG", format!("应用配置格式有误：{}", error))
    })
}

fn default_theme() -> String {
    "light".to_string()
}

fn normalize_theme(theme: &str) -> StorageResult<String> {
    match theme.trim().to_ascii_lowercase().as_str() {
        "light" => Ok("light".to_string()),
        "dark" => Ok("dark".to_string()),
        _ => Err(StorageError::new(
            "INVALID_THEME",
            "主题只能是 light 或 dark。",
        )),
    }
}

fn configured_ready_library(app: &AppHandle) -> StorageResult<PathBuf> {
    let config_path = app_config_path(app)?;
    let Some(library_path) = read_library_config(&config_path)? else {
        return Err(StorageError::new(
            "LIBRARY_NOT_CONFIGURED",
            "请先选择并初始化资料库位置。",
        ));
    };
    let root = validate_library_root(Path::new(&library_path))?;
    let (ready, missing_items, _) = inspect_library(&root);
    if !ready {
        return Err(StorageError::new(
            "LIBRARY_NOT_READY",
            format!("资料库结构不完整，缺少：{}。", missing_items.join("、")),
        ));
    }
    Ok(root)
}

fn library_status_for(root: &Path) -> StorageResult<LibraryStatus> {
    let (ready, missing_items, article_count) = inspect_library(root);
    Ok(LibraryStatus {
        configured: true,
        library_path: Some(path_to_string(root)?),
        ready,
        missing_items,
        article_count,
    })
}

fn validate_library_root(path: &Path) -> StorageResult<PathBuf> {
    if !path.is_absolute() {
        return Err(StorageError::new(
            "INVALID_LIBRARY_PATH",
            "资料库路径必须是绝对路径。",
        ));
    }
    if !path.is_dir() {
        return Err(StorageError::new(
            "INVALID_LIBRARY_PATH",
            "资料库路径必须是一个已存在的文件夹。",
        ));
    }
    fs::canonicalize(path).map_err(|error| StorageError::io("无法确认资料库路径", error))
}

fn initialize_library_at(root: &Path) -> StorageResult<()> {
    validate_library_root(root)?;
    for directory in [root.join(ARTICLES_DIR_NAME), root.join(TRASH_DIR_NAME)] {
        fs::create_dir_all(&directory)
            .map_err(|error| StorageError::io("无法创建资料库目录", error))?;
    }

    let settings_path = root.join(SETTINGS_FILE_NAME);
    if !settings_path.exists() {
        write_json_atomic(
            &settings_path,
            &LibrarySettings {
                schema_version: 1,
                app_name: "ShiLu",
            },
        )?;
    }

    let search_index_path = root.join(SEARCH_INDEX_FILE_NAME);
    if !search_index_path.exists() {
        write_json_atomic(
            &search_index_path,
            &SearchIndex {
                schema_version: 1,
                articles: Vec::new(),
            },
        )?;
    }
    Ok(())
}

fn copy_directory_contents(source: &Path, destination: &Path) -> StorageResult<()> {
    for entry in
        fs::read_dir(source).map_err(|error| StorageError::io("无法读取旧资料库", error))?
    {
        let entry = entry.map_err(|error| StorageError::io("无法读取资料库项目", error))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| StorageError::io("无法读取资料库项目类型", error))?;
        if file_type.is_symlink() {
            return Err(StorageError::new(
                "UNSUPPORTED_LIBRARY_ITEM",
                "资料库中包含符号链接，暂不支持自动迁移。",
            ));
        }
        if file_type.is_dir() {
            fs::create_dir_all(&destination_path)
                .map_err(|error| StorageError::io("无法创建迁移目录", error))?;
            copy_directory_contents(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path)
                .map_err(|error| StorageError::io("无法复制资料库文件", error))?;
        }
    }
    Ok(())
}

fn clear_directory_contents(root: &Path) -> StorageResult<()> {
    for entry in fs::read_dir(root).map_err(|error| StorageError::io("无法清理迁移目录", error))?
    {
        let path = entry
            .map_err(|error| StorageError::io("无法读取迁移目录项目", error))?
            .path();
        if path.is_dir() {
            fs::remove_dir_all(path)
                .map_err(|error| StorageError::io("无法清理迁移目录", error))?;
        } else {
            fs::remove_file(path).map_err(|error| StorageError::io("无法清理迁移文件", error))?;
        }
    }
    Ok(())
}

fn inspect_library(root: &Path) -> (bool, Vec<String>, usize) {
    let required_paths = [
        (ARTICLES_DIR_NAME, root.join(ARTICLES_DIR_NAME), true),
        (TRASH_DIR_NAME, root.join(TRASH_DIR_NAME), true),
        (SETTINGS_FILE_NAME, root.join(SETTINGS_FILE_NAME), false),
        (
            SEARCH_INDEX_FILE_NAME,
            root.join(SEARCH_INDEX_FILE_NAME),
            false,
        ),
    ];
    let missing_items: Vec<String> = required_paths
        .iter()
        .filter_map(|(name, path, is_directory)| {
            let is_present = if *is_directory {
                path.is_dir()
            } else {
                path.is_file()
            };
            (!is_present).then(|| (*name).to_string())
        })
        .collect();
    let article_count = fs::read_dir(root.join(ARTICLES_DIR_NAME))
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    entry.path().is_dir()
                        && is_valid_article_folder_name(&entry.file_name().to_string_lossy())
                })
                .count()
        })
        .unwrap_or(0);
    (missing_items.is_empty(), missing_items, article_count)
}

fn create_article_skeleton_at(
    root: &Path,
    title: &str,
    source_url: Option<&str>,
) -> StorageResult<ArticleSkeleton> {
    let articles_dir = root.join(ARTICLES_DIR_NAME);
    let now = Local::now();
    let timestamp = now.format("%Y%m%d-%H%M%S").to_string();
    let iso_timestamp = now.to_rfc3339_opts(SecondsFormat::Secs, false);
    let id = short_id();
    let folder_name = format!("{}-{}-{}", timestamp, slugify(title), id);
    let folder_path = articles_dir.join(&folder_name);
    fs::create_dir(&folder_path).map_err(|error| StorageError::io("无法创建文章文件夹", error))?;

    let creation_result = (|| -> StorageResult<()> {
        for child in ["images", "raw", "versions"] {
            fs::create_dir(folder_path.join(child))
                .map_err(|error| StorageError::io("无法创建文章子目录", error))?;
        }
        let markdown = create_markdown(title, source_url, &id, &iso_timestamp);
        write_text_atomic(&folder_path.join("index.md"), &markdown)
    })();

    if let Err(error) = creation_result {
        let _ = fs::remove_dir_all(&folder_path);
        return Err(error);
    }

    Ok(ArticleSkeleton {
        id,
        folder_name,
        folder_path: path_to_string(&folder_path)?,
        markdown_path: path_to_string(&folder_path.join("index.md"))?,
    })
}

fn move_article_to_trash_at(root: &Path, folder_name: &str) -> StorageResult<TrashMoveResult> {
    let articles_dir = fs::canonicalize(root.join(ARTICLES_DIR_NAME))
        .map_err(|error| StorageError::io("无法读取文章目录", error))?;
    let article_path = articles_dir.join(folder_name);
    if !article_path.is_dir() {
        return Err(StorageError::new(
            "ARTICLE_NOT_FOUND",
            "未找到要删除的文章。",
        ));
    }
    let canonical_article_path = fs::canonicalize(&article_path)
        .map_err(|error| StorageError::io("无法确认文章路径", error))?;
    if !canonical_article_path.starts_with(&articles_dir) {
        return Err(StorageError::new(
            "UNSAFE_ARTICLE_PATH",
            "文章路径不在当前资料库内，已拒绝操作。",
        ));
    }

    let trash_dir = root.join(TRASH_DIR_NAME);
    let mut trash_path = trash_dir.join(folder_name);
    if trash_path.exists() {
        let unique_suffix = short_id();
        trash_path = trash_dir.join(format!("{}-{}", folder_name, unique_suffix));
    }
    fs::rename(&article_path, &trash_path)
        .map_err(|error| StorageError::io("无法将文章移动到回收站", error))?;
    Ok(TrashMoveResult {
        folder_name: folder_name.to_string(),
        trash_path: path_to_string(&trash_path)?,
    })
}

struct ValidatedImageSource {
    data: ValidatedImageData,
    original_file_name: String,
    extension: String,
    size_bytes: u64,
}

enum ValidatedImageData {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

fn validate_sources(sources: &[ImageSource]) -> StorageResult<Vec<ValidatedImageSource>> {
    if sources.len() > MAX_IMPORT_IMAGES {
        return Err(StorageError::new(
            "TOO_MANY_IMAGES",
            "一次最多导入 50 张图片，请分批添加。",
        ));
    }
    let mut total_bytes = 0_u64;
    sources
        .iter()
        .map(|source| {
            let source = match source {
                ImageSource::Path { path } => {
                    let mut validated = validate_image_source(path)?;
                    if validated.size_bytes > MAX_IMAGE_BYTES as u64 {
                        return Err(StorageError::new(
                            "IMAGE_TOO_LARGE",
                            "每张图片不能超过 20 MB。",
                        ));
                    }
                    let ValidatedImageData::Path(path) = &validated.data else {
                        unreachable!()
                    };
                    let mut bytes = Vec::with_capacity(validated.size_bytes as usize);
                    fs::File::open(path)
                        .map_err(|error| StorageError::io("无法读取来源图片", error))?
                        .take((MAX_IMAGE_BYTES + 1) as u64)
                        .read_to_end(&mut bytes)
                        .map_err(|error| StorageError::io("无法读取来源图片", error))?;
                    let format = match validated.extension.as_str() {
                        "png" => image::ImageFormat::Png,
                        "webp" => image::ImageFormat::WebP,
                        _ => image::ImageFormat::Jpeg,
                    };
                    validate_image_bytes(&bytes, format)?;
                    validated.size_bytes = bytes.len() as u64;
                    validated.data = ValidatedImageData::Bytes(bytes);
                    validated
                }
                ImageSource::Clipboard { name, base64 } => {
                    let bytes = decode_clipboard_png(base64)?;
                    // The supplied display name is metadata only; filenames are always generated below.
                    let name = name.rsplit(['/', '\\']).next().unwrap_or("").trim();
                    ValidatedImageSource {
                        original_file_name: if name.is_empty() {
                            "截图.png".to_string()
                        } else {
                            name.chars().take(200).collect()
                        },
                        extension: "png".to_string(),
                        size_bytes: bytes.len() as u64,
                        data: ValidatedImageData::Bytes(bytes),
                    }
                }
            };
            total_bytes += source.size_bytes;
            if total_bytes > MAX_IMPORT_BYTES {
                return Err(StorageError::new(
                    "IMAGE_BATCH_TOO_LARGE",
                    "一次导入的图片合计不能超过 200 MB，请分批添加。",
                ));
            }
            Ok(source)
        })
        .collect()
}

fn create_article_with_sources_at(
    root: &Path,
    title: &str,
    source_url: Option<&str>,
    sources: &[ImageSource],
) -> StorageResult<ArticleCreateResult> {
    create_article_with_sources_using(
        root,
        title,
        source_url,
        sources,
        import_validated_sources_at,
    )
}

fn create_article_with_sources_using(
    root: &Path,
    title: &str,
    source_url: Option<&str>,
    sources: &[ImageSource],
    importer: impl FnOnce(&Path, &str, &[ValidatedImageSource]) -> StorageResult<ImageImportResult>,
) -> StorageResult<ArticleCreateResult> {
    let title = title.trim();
    if title.is_empty() {
        return Err(StorageError::new("INVALID_TITLE", "资料标题不能为空。"));
    }
    // Validate every input before creating the article, including full image decoding.
    let validated = validate_sources(sources)?;
    let article = create_article_skeleton_at(root, title, source_url)?;
    match importer(root, &article.folder_name, &validated) {
        Ok(images) => Ok(ArticleCreateResult { article, images }),
        Err(error) => {
            if let Err(cleanup_error) = fs::remove_dir_all(Path::new(&article.folder_path)) {
                return Err(StorageError::new(
                    "ARTICLE_ROLLBACK_FAILED",
                    format!(
                        "{} 新建资料清理失败，请检查文件夹 {}：{}",
                        error.message, article.folder_path, cleanup_error,
                    ),
                ));
            }
            Err(error)
        }
    }
}

fn import_article_sources_at(
    root: &Path,
    article_reference: &str,
    sources: &[ImageSource],
) -> StorageResult<ImageImportResult> {
    if sources.is_empty() {
        return Err(StorageError::new(
            "NO_IMAGES_SELECTED",
            "请至少选择或粘贴一张图片。",
        ));
    }
    // Resolve first so an invalid article reference is rejected before processing image bytes.
    resolve_article_path(root, article_reference)?;
    let validated = validate_sources(sources)?;
    import_validated_sources_at(root, article_reference, &validated)
}

fn import_article_images_at(
    root: &Path,
    article_reference: &str,
    source_paths: &[String],
) -> StorageResult<ImageImportResult> {
    if source_paths.is_empty() {
        return Err(StorageError::new(
            "NO_IMAGES_SELECTED",
            "请至少选择一张要导入的图片。",
        ));
    }

    resolve_article_path(root, article_reference)?;
    let sources = source_paths
        .iter()
        .map(|source_path| validate_image_source(source_path))
        .collect::<StorageResult<Vec<_>>>()?;
    import_validated_sources_at(root, article_reference, &sources)
}

fn import_validated_sources_at(
    root: &Path,
    article_reference: &str,
    sources: &[ValidatedImageSource],
) -> StorageResult<ImageImportResult> {
    let _guard = IMAGE_IMPORT_LOCK.lock().map_err(|_| {
        StorageError::new("IMAGE_IMPORT_BUSY", "图片导入状态异常，请重启软件后重试。")
    })?;
    let (article_path, article_folder_name, article_id) =
        resolve_article_path(root, article_reference)?;
    let images_path = fs::canonicalize(article_path.join("images"))
        .map_err(|error| StorageError::io("无法读取文章图片目录", error))?;
    if !images_path.is_dir() || !images_path.starts_with(&article_path) {
        return Err(StorageError::new(
            "UNSAFE_IMAGES_PATH",
            "文章图片目录不在当前文章文件夹内，已拒绝导入。",
        ));
    }

    let first_index = next_image_index(&images_path)?;
    let mut imported = Vec::with_capacity(sources.len());
    let mut created_paths = Vec::with_capacity(sources.len());

    let import_result = (|| -> StorageResult<()> {
        for (offset, source) in sources.iter().enumerate() {
            let index = first_index.checked_add(offset as u64).ok_or_else(|| {
                StorageError::new("IMAGE_INDEX_OVERFLOW", "文章中的图片数量超出支持范围。")
            })?;
            let file_name = format!("{:03}.{}", index, source.extension);
            let destination = images_path.join(&file_name);
            let path = path_to_string(&destination)?;
            write_imported_image(&source.data, &destination)?;
            created_paths.push(destination);
            imported.push(ImportedImage {
                file_name: file_name.clone(),
                original_file_name: source.original_file_name.clone(),
                path,
                relative_path: format!("images/{}", file_name),
                extension: source.extension.clone(),
                size_bytes: source.size_bytes,
            });
        }
        Ok(())
    })();
    if let Err(error) = import_result {
        cleanup_imported_images(&created_paths);
        return Err(error);
    }

    Ok(ImageImportResult {
        article_id,
        article_folder_name,
        images: imported,
    })
}

fn resolve_article_path(
    root: &Path,
    article_reference: &str,
) -> StorageResult<(PathBuf, String, String)> {
    let article_reference = article_reference.trim();
    let articles_path = fs::canonicalize(root.join(ARTICLES_DIR_NAME))
        .map_err(|error| StorageError::io("无法读取文章目录", error))?;

    let folder_name = if is_valid_article_folder_name(article_reference) {
        article_reference.to_string()
    } else if is_valid_article_id(article_reference) {
        let matches = fs::read_dir(&articles_path)
            .map_err(|error| StorageError::io("无法扫描文章目录", error))?
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| {
                is_valid_article_folder_name(name)
                    && article_id_from_folder_name(name) == Some(article_reference)
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [folder_name] => folder_name.clone(),
            [] => {
                return Err(StorageError::new(
                    "ARTICLE_NOT_FOUND",
                    "未找到要导入图片的文章。",
                ))
            }
            _ => {
                return Err(StorageError::new(
                    "AMBIGUOUS_ARTICLE_ID",
                    "资料 ID 对应多个文章文件夹，请改用完整文件夹名。",
                ))
            }
        }
    } else {
        return Err(StorageError::new(
            "INVALID_ARTICLE_REFERENCE",
            "文章标识必须是完整文件夹名或 6 位资料 ID。",
        ));
    };

    let article_path = articles_path.join(&folder_name);
    if !article_path.is_dir() {
        return Err(StorageError::new(
            "ARTICLE_NOT_FOUND",
            "未找到要导入图片的文章。",
        ));
    }
    let article_path = fs::canonicalize(article_path)
        .map_err(|error| StorageError::io("无法确认文章路径", error))?;
    if !article_path.starts_with(&articles_path) {
        return Err(StorageError::new(
            "UNSAFE_ARTICLE_PATH",
            "文章路径不在当前资料库内，已拒绝导入。",
        ));
    }
    let article_id = article_id_from_folder_name(&folder_name)
        .ok_or_else(|| StorageError::new("INVALID_ARTICLE_FOLDER", "文章文件夹名称不合法。"))?
        .to_string();
    Ok((article_path, folder_name, article_id))
}

fn read_article_at(root: &Path, article_reference: &str) -> StorageResult<ArticleDocument> {
    let (article_path, folder_name, article_id) = resolve_article_path(root, article_reference)?;
    let markdown_path = article_path.join("index.md");
    let markdown = fs::read_to_string(&markdown_path)
        .map_err(|error| StorageError::io("无法读取文章 Markdown", error))?;
    let document = parse_article_markdown(&markdown, &article_id, &folder_name);
    Ok(ArticleDocument {
        markdown_path: path_to_string(&markdown_path)?,
        ..document
    })
}

fn save_article_at(
    root: &Path,
    article_reference: &str,
    draft: ArticleDraft,
) -> StorageResult<ArticleDocument> {
    let (article_path, folder_name, article_id) = resolve_article_path(root, article_reference)?;
    let markdown_path = article_path.join("index.md");
    let current = if markdown_path.is_file() {
        parse_article_markdown(
            &fs::read_to_string(&markdown_path)
                .map_err(|error| StorageError::io("无法读取文章 Markdown", error))?,
            &article_id,
            &folder_name,
        )
    } else {
        ArticleDocument {
            id: article_id.clone(),
            folder_name: folder_name.clone(),
            title: String::new(),
            summary: String::new(),
            source_url: String::new(),
            tags: Vec::new(),
            content: String::new(),
            notes: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            markdown_path: String::new(),
        }
    };
    let title = draft.title.trim();
    if title.is_empty() {
        return Err(StorageError::new("INVALID_TITLE", "资料标题不能为空。"));
    }
    let updated_at = Local::now().to_rfc3339_opts(SecondsFormat::Secs, false);
    let created_at = if current.created_at.is_empty() {
        updated_at.clone()
    } else {
        current.created_at
    };
    let document = ArticleDocument {
        id: article_id,
        folder_name,
        title: title.to_string(),
        summary: draft.summary.trim().to_string(),
        source_url: draft.source_url.trim().to_string(),
        tags: clean_tags(draft.tags),
        content: draft.content.trim().to_string(),
        notes: draft.notes.trim().to_string(),
        created_at,
        updated_at,
        markdown_path: path_to_string(&markdown_path)?,
    };
    write_text_atomic(&markdown_path, &render_article_markdown(&document))?;
    Ok(document)
}

fn list_articles_at(root: &Path, query: &str) -> StorageResult<Vec<ArticleSummary>> {
    let articles_path = fs::canonicalize(root.join(ARTICLES_DIR_NAME))
        .map_err(|error| StorageError::io("无法读取文章目录", error))?;
    let needle = query.to_lowercase();
    let mut articles = Vec::new();
    for entry in fs::read_dir(&articles_path)
        .map_err(|error| StorageError::io("无法扫描文章目录", error))?
        .flatten()
    {
        let path = entry.path();
        let folder_name = entry.file_name().to_string_lossy().to_string();
        if !path.is_dir() || !is_valid_article_folder_name(&folder_name) {
            continue;
        }
        let markdown_path = path.join("index.md");
        let Ok(markdown) = fs::read_to_string(&markdown_path) else {
            continue;
        };
        let document = parse_article_markdown(
            &markdown,
            article_id_from_folder_name(&folder_name).unwrap_or_default(),
            &folder_name,
        );
        let haystack = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            document.title,
            document.summary,
            document.source_url,
            document.tags.join(" "),
            document.content,
            document.notes
        )
        .to_lowercase();
        if !needle.is_empty() && !haystack.contains(&needle) {
            continue;
        }
        articles.push(ArticleSummary {
            id: document.id,
            folder_name,
            title: document.title,
            summary: document.summary,
            source_url: document.source_url,
            tags: document.tags,
            updated_at: document.updated_at,
            markdown_path: path_to_string(&markdown_path)?,
        });
    }
    articles.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.title.cmp(&right.title))
    });
    Ok(articles)
}

#[cfg(windows)]
fn ocr_article_images_at(root: &Path, article_reference: &str) -> StorageResult<OcrResult> {
    use windows::{
        core::HSTRING,
        Graphics::Imaging::BitmapDecoder,
        Media::Ocr::OcrEngine,
        Storage::{FileAccessMode, Streams::FileRandomAccessStream},
    };
    let (article_path, _folder_name, article_id) = resolve_article_path(root, article_reference)?;
    let images_path = article_path.join("images");
    let mut image_paths = fs::read_dir(&images_path)
        .map_err(|error| StorageError::io("无法读取文章图片目录", error))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| {
                    ALLOWED_IMAGE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    image_paths.sort();
    if image_paths.is_empty() {
        return Err(StorageError::new(
            "NO_ARTICLE_IMAGES",
            "这条资料还没有图片可供识别。",
        ));
    }
    let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|error| {
        StorageError::new("OCR_UNAVAILABLE", format!("Windows OCR 不可用：{}", error))
    })?;
    let mut blocks = Vec::new();
    for path in &image_paths {
        // Rust canonicalization uses verbatim Windows paths; WinRT rejects that namespace.
        // Keep canonical paths for storage checks and adapt only at this API boundary.
        let native_path = path_to_string(path)?;
        let winrt_path = if let Some(unc) = native_path.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else {
            native_path
                .strip_prefix(r"\\?\")
                .unwrap_or(&native_path)
                .to_string()
        };
        let stream =
            FileRandomAccessStream::OpenAsync(&HSTRING::from(winrt_path), FileAccessMode::Read)
                .map_err(|error| {
                    StorageError::new("OCR_IMAGE_OPEN_FAILED", format!("无法打开图片：{}", error))
                })?
                .get()
                .map_err(|error| {
                    StorageError::new("OCR_IMAGE_OPEN_FAILED", format!("无法打开图片：{}", error))
                })?;
        let decoder = BitmapDecoder::CreateAsync(&stream)
            .map_err(|error| {
                StorageError::new("OCR_DECODER_FAILED", format!("无法读取图片：{}", error))
            })?
            .get()
            .map_err(|error| {
                StorageError::new("OCR_DECODER_FAILED", format!("无法读取图片：{}", error))
            })?;
        let bitmap = decoder
            .GetSoftwareBitmapAsync()
            .map_err(|error| {
                StorageError::new("OCR_BITMAP_FAILED", format!("无法读取图片：{}", error))
            })?
            .get()
            .map_err(|error| {
                StorageError::new("OCR_BITMAP_FAILED", format!("无法读取图片：{}", error))
            })?;
        let result = engine
            .RecognizeAsync(&bitmap)
            .map_err(|error| {
                StorageError::new("OCR_RECOGNIZE_FAILED", format!("图片识别失败：{}", error))
            })?
            .get()
            .map_err(|error| {
                StorageError::new("OCR_RECOGNIZE_FAILED", format!("图片识别失败：{}", error))
            })?;
        blocks.push(
            result
                .Text()
                .map_err(|error| {
                    StorageError::new("OCR_RESULT_FAILED", format!("无法读取识别结果：{}", error))
                })?
                .to_string(),
        );
    }
    let text = blocks
        .into_iter()
        .enumerate()
        .map(|(index, block)| format!("## 图片 {}\n\n{}", index + 1, block.trim()))
        .collect::<Vec<_>>()
        .join("\n\n");
    let raw_path = article_path.join("raw").join("ocr.txt");
    write_text_atomic(&raw_path, &text)?;
    Ok(OcrResult {
        article_id,
        text,
        image_count: image_paths.len(),
        raw_path: path_to_string(&raw_path)?,
    })
}

#[cfg(not(windows))]
fn ocr_article_images_at(_root: &Path, _article_reference: &str) -> StorageResult<OcrResult> {
    Err(StorageError::new(
        "OCR_UNAVAILABLE",
        "本地 OCR 目前只支持 Windows。",
    ))
}

fn parse_article_markdown(markdown: &str, article_id: &str, folder_name: &str) -> ArticleDocument {
    let (front_matter, body) = if let Some(rest) = markdown.strip_prefix("---\n") {
        rest.split_once("\n---\n")
            .map_or(("", markdown), |parts| parts)
    } else {
        ("", markdown)
    };
    let title = yaml_value(front_matter, "title").unwrap_or_else(|| first_heading(body));
    let summary = yaml_value(front_matter, "summary").unwrap_or_default();
    let source_url = yaml_value(front_matter, "source_url").unwrap_or_default();
    let created_at = yaml_value(front_matter, "created_at").unwrap_or_default();
    let updated_at = yaml_value(front_matter, "updated_at").unwrap_or_else(|| created_at.clone());
    let tags = yaml_list(front_matter, "tags");
    let content = section_body(body, "正文");
    let notes = section_body(body, "我的备注");
    ArticleDocument {
        id: article_id.to_string(),
        folder_name: folder_name.to_string(),
        title,
        summary,
        source_url,
        tags,
        content,
        notes,
        created_at,
        updated_at,
        markdown_path: String::new(),
    }
}

fn render_article_markdown(document: &ArticleDocument) -> String {
    let tags = if document.tags.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            document
                .tags
                .iter()
                .map(|tag| format!("\"{}\"", escape_yaml(tag)))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!(
        "---\nid: \"{}\"\ntitle: \"{}\"\nsummary: \"{}\"\ncreated_at: \"{}\"\nupdated_at: \"{}\"\nsource_url: \"{}\"\ntags: {}\n---\n\n# {}\n\n## 正文\n\n{}\n\n## 我的备注\n\n{}\n",
        escape_yaml(&document.id),
        escape_yaml(&document.title),
        escape_yaml(&document.summary),
        escape_yaml(&document.created_at),
        escape_yaml(&document.updated_at),
        escape_yaml(&document.source_url),
        tags,
        document.title,
        document.content,
        document.notes
    )
}

fn yaml_value(front_matter: &str, key: &str) -> Option<String> {
    front_matter.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim() != key {
            return None;
        }
        let value = value.trim();
        Some(
            value
                .trim_matches('"')
                .replace("\\\"", "\"")
                .replace("\\\\", "\\"),
        )
    })
}

fn yaml_list(front_matter: &str, key: &str) -> Vec<String> {
    let value = yaml_value(front_matter, key).unwrap_or_default();
    value
        .trim_matches(['[', ']'])
        .split(',')
        .filter_map(|item| {
            let item = item.trim().trim_matches('"');
            (!item.is_empty()).then(|| item.to_string())
        })
        .collect()
}

fn first_heading(body: &str) -> String {
    body.lines()
        .find_map(|line| {
            line.strip_prefix("# ")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "未命名资料".to_string())
}

fn section_body(body: &str, heading: &str) -> String {
    let marker = format!("## {}", heading);
    let Some((_, rest)) = body.split_once(&marker) else {
        return String::new();
    };
    let rest = rest.trim_start_matches([' ', '\n', '\r']);
    let Some((content, _)) = rest.split_once("\n## ") else {
        return rest.trim().to_string();
    };
    content.trim().to_string()
}

fn clean_tags(tags: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for tag in tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
    {
        if !result.contains(&tag) {
            result.push(tag);
        }
    }
    result
}

fn validate_image_source(source_path: &str) -> StorageResult<ValidatedImageSource> {
    let source_path = Path::new(source_path.trim());
    if source_path.as_os_str().is_empty() || !source_path.is_absolute() {
        return Err(StorageError::new(
            "INVALID_IMAGE_PATH",
            "图片来源必须是绝对文件路径。",
        ));
    }
    let canonical_path = fs::canonicalize(source_path)
        .map_err(|error| StorageError::io("无法确认图片来源路径", error))?;
    if !canonical_path.is_file() {
        return Err(StorageError::new(
            "INVALID_IMAGE_PATH",
            "图片来源必须是一个已存在的文件。",
        ));
    }

    let original_file_name = canonical_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| StorageError::new("INVALID_IMAGE_PATH", "图片文件名无法识别。"))?
        .to_string();
    let extension = canonical_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|extension| ALLOWED_IMAGE_EXTENSIONS.contains(&extension.as_str()))
        .ok_or_else(|| {
            StorageError::new(
                "UNSUPPORTED_IMAGE_FORMAT",
                "仅支持 PNG、JPG、JPEG 和 WEBP 图片。",
            )
        })?;
    let size_bytes = fs::metadata(&canonical_path)
        .map_err(|error| StorageError::io("无法读取图片信息", error))?
        .len();

    Ok(ValidatedImageSource {
        data: ValidatedImageData::Path(canonical_path),
        original_file_name,
        extension,
        size_bytes,
    })
}

fn next_image_index(images_path: &Path) -> StorageResult<u64> {
    let highest_index = fs::read_dir(images_path)
        .map_err(|error| StorageError::io("无法扫描文章图片目录", error))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let extension = path.extension()?.to_str()?.to_ascii_lowercase();
            if !ALLOWED_IMAGE_EXTENSIONS.contains(&extension.as_str()) {
                return None;
            }
            path.file_stem()?.to_str()?.parse::<u64>().ok()
        })
        .max()
        .unwrap_or(0);
    highest_index
        .checked_add(1)
        .ok_or_else(|| StorageError::new("IMAGE_INDEX_OVERFLOW", "文章中的图片数量超出支持范围。"))
}

fn write_imported_image(source: &ValidatedImageData, destination: &Path) -> StorageResult<()> {
    // Exclusive creation prevents a competing import from overwriting an existing image.
    let mut destination_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                StorageError::new(
                    "IMAGE_DESTINATION_EXISTS",
                    "目标图片已存在，未覆盖任何文件，请重试。",
                )
            } else {
                StorageError::io("无法创建导入图片", error)
            }
        })?;
    let copy_result = (|| -> Result<(), std::io::Error> {
        match source {
            ValidatedImageData::Path(path) => {
                let mut source_file = fs::File::open(path)?;
                std::io::copy(&mut source_file, &mut destination_file)?;
            }
            ValidatedImageData::Bytes(bytes) => destination_file.write_all(bytes)?,
        }
        destination_file.sync_all()?;
        Ok(())
    })();
    drop(destination_file);
    if let Err(error) = copy_result {
        let _ = fs::remove_file(destination);
        return Err(StorageError::io("无法复制图片", error));
    }
    Ok(())
}

fn cleanup_imported_images(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> StorageResult<()> {
    let content = serde_json::to_vec_pretty(value).map_err(|error| {
        StorageError::new(
            "JSON_SERIALIZATION_ERROR",
            format!("无法生成 JSON 数据：{}", error),
        )
    })?;
    write_bytes_atomic(path, &content)
}

fn write_text_atomic(path: &Path, content: &str) -> StorageResult<()> {
    write_bytes_atomic(path, content.as_bytes())
}

fn write_bytes_atomic(path: &Path, content: &[u8]) -> StorageResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| StorageError::new("INVALID_FILE_PATH", "无法确定文件所在目录。"))?;
    fs::create_dir_all(parent).map_err(|error| StorageError::io("无法创建文件目录", error))?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| StorageError::new("INVALID_FILE_PATH", "文件名称无效。"))?;
    let temp_path = parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        ARTICLE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| StorageError::io("无法创建临时文件", error))?;
    let write_result = (|| -> Result<(), std::io::Error> {
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(StorageError::io("无法原子保存文件", error));
    }
    Ok(())
}

fn create_markdown(title: &str, source_url: Option<&str>, id: &str, timestamp: &str) -> String {
    let source_url = source_url.unwrap_or("").trim();
    format!(
        "---\nid: \"{}\"\ntitle: \"{}\"\nsummary: \"\"\ncreated_at: \"{}\"\nupdated_at: \"{}\"\nsource_url: \"{}\"\ntags: []\n---\n\n# {}\n\n## 正文\n\n\n## 我的备注\n\n",
        escape_yaml(id),
        escape_yaml(title),
        escape_yaml(timestamp),
        escape_yaml(timestamp),
        escape_yaml(source_url),
        title,
    )
}

fn escape_yaml(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut previous_separator = false;
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_separator = false;
        } else if !previous_separator && !slug.is_empty() {
            slug.push('-');
            previous_separator = true;
        }
        if slug.len() >= 48 {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "article".to_string()
    } else {
        slug.to_string()
    }
}

fn short_id() -> String {
    let sequence = ARTICLE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:06x}", (nanos as u64 ^ sequence) & 0x00ff_ffff)
}

fn is_valid_article_folder_name(folder_name: &str) -> bool {
    let mut sections = folder_name.splitn(3, '-');
    let Some(date) = sections.next() else {
        return false;
    };
    let Some(time) = sections.next() else {
        return false;
    };
    let Some(slug_and_id) = sections.next() else {
        return false;
    };
    if date.len() != 8 || !date.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    if time.len() != 6 || !time.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Some((slug, id)) = slug_and_id.rsplit_once('-') else {
        return false;
    };
    !slug.is_empty()
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && id.len() == 6
        && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_valid_article_id(article_id: &str) -> bool {
    article_id.len() == 6 && article_id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn article_id_from_folder_name(folder_name: &str) -> Option<&str> {
    is_valid_article_folder_name(folder_name)
        .then(|| folder_name.rsplit_once('-').map(|(_, id)| id))
        .flatten()
}

fn path_to_string(path: &Path) -> StorageResult<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| StorageError::new("INVALID_PATH_ENCODING", "路径包含无法识别的字符。"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};

    fn screenshot_png(color: [u8; 4]) -> Vec<u8> {
        use image::ImageEncoder;
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&color, 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        bytes
    }

    fn screenshot_source(color: [u8; 4]) -> ImageSource {
        ImageSource::Clipboard {
            name: "截图.png".to_string(),
            base64: STANDARD.encode(screenshot_png(color)),
        }
    }

    fn temporary_library_root(test_name: &str) -> PathBuf {
        let nonce = format!("{}-{}", std::process::id(), short_id());
        std::env::temp_dir().join(format!("shilu-storage-test-{}-{}", test_name, nonce))
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Requires Windows OCR and SHILU_OCR_TEST_IMAGE with a PNG containing SCREENSHOT and OCR TEST"]
    fn clipboard_png_reaches_windows_ocr_without_changing_markdown() {
        let fixture =
            std::env::var("SHILU_OCR_TEST_IMAGE").expect("set a controlled OCR fixture path");
        let png = fs::read(fixture).expect("read controlled PNG fixture");
        let root = temporary_library_root("clipboard-ocr");
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let created = create_article_with_sources_at(
                &root,
                "剪贴板 OCR 验收",
                Some("https://example.com/clipboard-fixture"),
                &[ImageSource::Clipboard {
                    name: "image.png".into(),
                    base64: STANDARD.encode(&png),
                }],
            )?;
            let markdown_before = fs::read(&created.article.markdown_path).unwrap();
            let recognized = ocr_article_images_at(&root, &created.article.folder_name)?;
            assert_eq!(recognized.image_count, 1);
            assert!(
                recognized.text.to_uppercase().contains("SCREENSHOT"),
                "{}",
                recognized.text
            );
            assert!(
                recognized.text.to_uppercase().contains("OCR TEST"),
                "{}",
                recognized.text
            );
            assert_eq!(
                fs::read_to_string(&recognized.raw_path).unwrap(),
                recognized.text
            );
            assert_eq!(fs::read(&created.images.images[0].path).unwrap(), png);
            assert_eq!(
                fs::read(&created.article.markdown_path).unwrap(),
                markdown_before
            );
            println!("PNG saved unchanged; real Windows OCR recognized SCREENSHOT and OCR TEST; raw/ocr.txt saved; Markdown unchanged");
            Ok(())
        })();
        fs::remove_dir_all(&root).unwrap();
        result.expect("clipboard PNG should be recognized by real Windows OCR");
    }

    #[test]
    fn mixed_sources_keep_order_duplicate_names_and_append_without_overwrite() {
        let root = temporary_library_root("mixed-sources");
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let source_path = root.join("source.png");
            let blue = screenshot_png([0, 0, 255, 255]);
            fs::write(&source_path, &blue).unwrap();
            let created = create_article_with_sources_at(
                &root,
                "混合截图",
                None,
                &[
                    screenshot_source([255, 0, 0, 255]),
                    ImageSource::Path {
                        path: path_to_string(&source_path)?,
                    },
                    screenshot_source([0, 255, 0, 255]),
                ],
            )?;
            assert_eq!(
                created
                    .images
                    .images
                    .iter()
                    .map(|image| image.file_name.as_str())
                    .collect::<Vec<_>>(),
                ["001.png", "002.png", "003.png"]
            );
            assert_eq!(
                created.images.images[0].original_file_name,
                created.images.images[2].original_file_name
            );
            assert_eq!(
                fs::read(&created.images.images[0].path).unwrap(),
                screenshot_png([255, 0, 0, 255])
            );
            assert_eq!(fs::read(&created.images.images[1].path).unwrap(), blue);
            assert_eq!(
                fs::read(&created.images.images[2].path).unwrap(),
                screenshot_png([0, 255, 0, 255])
            );
            let added = import_article_sources_at(
                &root,
                &created.article.id,
                &[screenshot_source([0, 0, 0, 0])],
            )?;
            assert_eq!(added.images[0].file_name, "004.png");
            assert_eq!(
                fs::read(&created.images.images[0].path).unwrap(),
                screenshot_png([255, 0, 0, 255])
            );
            Ok(())
        })();
        fs::remove_dir_all(&root).unwrap();
        result.expect("混合图片与同名截图应保持顺序且不会覆盖旧图");
    }

    #[test]
    fn malformed_clipboard_batch_never_leaves_a_new_article_or_partial_images() {
        let root = temporary_library_root("invalid-clipboard");
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let sources = [
                screenshot_source([1, 2, 3, 255]),
                ImageSource::Clipboard {
                    name: "broken.png".into(),
                    base64: STANDARD.encode(b"not a PNG"),
                },
            ];
            assert!(create_article_with_sources_at(&root, "应回滚", None, &sources).is_err());
            assert_eq!(
                fs::read_dir(root.join(ARTICLES_DIR_NAME)).unwrap().count(),
                0
            );
            let article = create_article_skeleton_at(&root, "已有资料", None)?;
            assert!(import_article_sources_at(&root, &article.id, &sources).is_err());
            assert_eq!(
                fs::read_dir(Path::new(&article.folder_path).join("images"))
                    .unwrap()
                    .count(),
                0
            );
            assert!(Path::new(&article.markdown_path).is_file());
            Ok(())
        })();
        fs::remove_dir_all(&root).unwrap();
        result.expect("无效剪贴板图片不能造成残留资料");
    }

    #[test]
    fn failed_image_write_rolls_back_only_this_batch() {
        let root = temporary_library_root("import-rollback");
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let article = create_article_with_sources_at(
                &root,
                "保留已有图片",
                None,
                &[screenshot_source([9, 8, 7, 255])],
            )?;
            let images_path = Path::new(&article.article.folder_path).join("images");
            // The directory is not counted as an existing numbered image, but blocks the second write.
            fs::create_dir(images_path.join("003.png")).unwrap();
            fs::write(images_path.join("003.png").join("keep.txt"), "keep").unwrap();
            assert!(import_article_sources_at(
                &root,
                &article.article.id,
                &[
                    screenshot_source([1, 2, 3, 255]),
                    screenshot_source([4, 5, 6, 255]),
                ]
            )
            .is_err());
            assert!(!images_path.join("002.png").exists());
            assert_eq!(
                fs::read(images_path.join("001.png")).unwrap(),
                screenshot_png([9, 8, 7, 255])
            );
            assert_eq!(
                fs::read_to_string(images_path.join("003.png").join("keep.txt")).unwrap(),
                "keep"
            );
            Ok(())
        })();
        fs::remove_dir_all(&root).unwrap();
        result.expect("导入失败应清理本批图片并保留原文件");
    }

    #[test]
    fn image_write_failure_removes_only_newly_created_article() {
        let root = temporary_library_root("article-image-rollback");
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let previous = create_article_skeleton_at(&root, "应保留", None)?;
            let error = create_article_with_sources_using(
                &root,
                "导入应失败",
                None,
                &[
                    screenshot_source([1, 2, 3, 255]),
                    screenshot_source([4, 5, 6, 255]),
                ],
                |root, reference, sources| {
                    let (path, _, _) = resolve_article_path(root, reference)?;
                    fs::create_dir(path.join("images").join("002.png")).unwrap();
                    import_validated_sources_at(root, reference, sources)
                },
            )
            .expect_err("第二张图片写入必须失败");
            assert!(matches!(
                error.code.as_str(),
                "IMAGE_DESTINATION_EXISTS" | "IO_ERROR"
            ));
            assert_eq!(
                fs::read_dir(root.join(ARTICLES_DIR_NAME)).unwrap().count(),
                1
            );
            assert!(Path::new(&previous.markdown_path).is_file());
            Ok(())
        })();
        fs::remove_dir_all(&root).unwrap();
        result.expect("新建资料导入失败应回滚整个新文件夹");
    }

    #[test]
    fn rejects_corrupted_path_images_and_too_many_sources_before_creation() {
        let root = temporary_library_root("source-limits");
        fs::create_dir_all(&root).unwrap();
        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let corrupt = root.join("corrupt.png");
            fs::write(&corrupt, b"not an image").unwrap();
            assert!(create_article_with_sources_at(
                &root,
                "损坏图片",
                None,
                &[ImageSource::Path {
                    path: path_to_string(&corrupt)?
                },]
            )
            .is_err());
            let sources = (0..51)
                .map(|_| screenshot_source([1, 2, 3, 255]))
                .collect::<Vec<_>>();
            let error =
                create_article_with_sources_at(&root, "太多图片", None, &sources).unwrap_err();
            assert_eq!(error.code, "TOO_MANY_IMAGES");
            assert_eq!(
                fs::read_dir(root.join(ARTICLES_DIR_NAME)).unwrap().count(),
                0
            );
            // An article with no images remains supported for manual text entry.
            assert!(create_article_with_sources_at(&root, "纯文字", None, &[])?
                .images
                .images
                .is_empty());
            Ok(())
        })();
        fs::remove_dir_all(&root).unwrap();
        result.expect("图片数量与文件内容校验应在新建资料前完成");
    }

    #[test]
    fn exclusive_image_write_never_replaces_an_existing_file() {
        let root = temporary_library_root("exclusive-image");
        fs::create_dir_all(&root).unwrap();
        let destination = root.join("001.png");
        fs::write(&destination, "original").unwrap();
        let result = write_imported_image(&ValidatedImageData::Bytes(vec![1, 2, 3]), &destination);
        let preserved = fs::read_to_string(&destination).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(result.unwrap_err().code, "IMAGE_DESTINATION_EXISTS");
        assert_eq!(preserved, "original");
    }

    #[test]
    fn initializes_library_and_creates_article_structure() {
        let root = temporary_library_root("initialize");
        fs::create_dir_all(&root).expect("应创建测试目录");

        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            assert!(root.join(ARTICLES_DIR_NAME).is_dir());
            assert!(root.join(TRASH_DIR_NAME).is_dir());
            assert!(root.join(SETTINGS_FILE_NAME).is_file());
            assert!(root.join(SEARCH_INDEX_FILE_NAME).is_file());

            let article = create_article_skeleton_at(
                &root,
                "Telegram 收藏内容",
                Some("https://example.com"),
            )?;
            assert!(is_valid_article_folder_name(&article.folder_name));
            assert!(Path::new(&article.folder_path).join("images").is_dir());
            assert!(Path::new(&article.folder_path).join("raw").is_dir());
            assert!(Path::new(&article.folder_path).join("versions").is_dir());
            let markdown = fs::read_to_string(&article.markdown_path)
                .map_err(|error| StorageError::io("无法读取测试文章", error))?;
            assert!(markdown.contains("title: \"Telegram 收藏内容\""));
            assert!(markdown.contains("## 正文"));
            assert!(markdown.contains("https://example.com"));

            let moved = move_article_to_trash_at(&root, &article.folder_name)?;
            assert!(!Path::new(&article.folder_path).exists());
            assert!(Path::new(&moved.trash_path).is_dir());
            Ok(())
        })();

        let cleanup = fs::remove_dir_all(&root);
        result.expect("初始化与文章创建应成功");
        cleanup.expect("应清理精确的测试目录");
    }

    #[test]
    fn validates_article_folder_names() {
        assert!(is_valid_article_folder_name(
            "20260908-201530-telegram-note-ab12ef"
        ));
        assert!(!is_valid_article_folder_name("..\\outside"));
        assert!(!is_valid_article_folder_name("20260908-201530-no-id"));
        assert!(!is_valid_article_folder_name(
            "20260908-201530-UPPER-ab12ef"
        ));
    }

    #[test]
    fn atomically_replaces_an_existing_file() {
        let root = temporary_library_root("atomic-write");
        fs::create_dir_all(&root).expect("应创建测试目录");
        let target = root.join("settings.json");

        let result = (|| -> StorageResult<()> {
            write_text_atomic(&target, "first")?;
            write_text_atomic(&target, "second")?;
            let saved = fs::read_to_string(&target)
                .map_err(|error| StorageError::io("无法读取原子写入测试文件", error))?;
            assert_eq!(saved, "second");
            Ok(())
        })();

        let cleanup = fs::remove_dir_all(&root);
        result.expect("原子写入应成功替换原文件");
        cleanup.expect("应清理精确的测试目录");
    }

    #[test]
    fn saves_theme_without_losing_the_library_path() {
        let root = temporary_library_root("app-settings");
        fs::create_dir_all(&root).expect("应创建测试目录");
        let config_path = root.join(CONFIG_FILE_NAME);

        let result = (|| -> StorageResult<()> {
            write_json_atomic(
                &config_path,
                &LibraryConfig {
                    library_path: Some("C:\\Users\\Example\\Documents\\ShiLuData".to_string()),
                    theme: default_theme(),
                },
            )?;
            let mut config = read_app_config(&config_path)?;
            config.theme = normalize_theme("dark")?;
            write_json_atomic(&config_path, &config)?;

            let saved = read_app_config(&config_path)?;
            assert_eq!(saved.theme, "dark");
            assert_eq!(
                saved.library_path.as_deref(),
                Some("C:\\Users\\Example\\Documents\\ShiLuData")
            );
            Ok(())
        })();

        let cleanup = fs::remove_dir_all(&root);
        result.expect("保存主题后应保留资料库路径");
        cleanup.expect("应清理精确的测试目录");
    }

    #[test]
    fn copies_library_contents_without_overwriting_destination() {
        let source = temporary_library_root("migration-source");
        let destination = temporary_library_root("migration-destination");
        fs::create_dir_all(&source).expect("应创建源资料库");
        fs::create_dir_all(&destination).expect("应创建目标资料库");

        let result = (|| -> StorageResult<()> {
            initialize_library_at(&source)?;
            let article = create_article_skeleton_at(&source, "迁移测试", None)?;
            let source_image = source.join("source.png");
            fs::write(&source_image, b"source image")
                .map_err(|error| StorageError::io("无法写入迁移测试图片", error))?;
            import_article_images_at(
                &source,
                &article.folder_name,
                &[source_image.to_string_lossy().into_owned()],
            )?;
            fs::write(source.join("custom-note.txt"), "custom")
                .map_err(|error| StorageError::io("无法写入迁移测试文件", error))?;

            copy_directory_contents(&source, &destination)?;
            assert!(destination.join(ARTICLES_DIR_NAME).is_dir());
            assert!(destination.join(SETTINGS_FILE_NAME).is_file());
            assert_eq!(
                fs::read_to_string(destination.join("custom-note.txt"))
                    .map_err(|error| StorageError::io("无法读取迁移测试文件", error))?,
                "custom"
            );
            assert_eq!(
                fs::read(
                    destination
                        .join(ARTICLES_DIR_NAME)
                        .join(&article.folder_name)
                        .join("images")
                        .join("001.png")
                )
                .map_err(|error| StorageError::io("无法读取迁移测试图片", error))?,
                b"source image"
            );

            let non_empty = destination.join("non-empty");
            fs::create_dir_all(&non_empty).expect("应创建非空目标测试目录");
            fs::write(non_empty.join("keep.txt"), "keep")
                .map_err(|error| StorageError::io("无法写入非空目标测试文件", error))?;
            assert!(fs::read_dir(&non_empty)
                .map_err(|error| StorageError::io("无法检查非空目标测试目录", error))?
                .next()
                .is_some());
            Ok(())
        })();

        let source_cleanup = fs::remove_dir_all(&source);
        let destination_cleanup = fs::remove_dir_all(&destination);
        result.expect("资料库内容应能完整复制");
        source_cleanup.expect("应清理迁移源测试目录");
        destination_cleanup.expect("应清理迁移目标测试目录");
    }

    #[test]
    fn imports_images_in_order_and_appends_existing_images() {
        let root = temporary_library_root("image-import");
        fs::create_dir_all(&root).expect("应创建测试目录");
        let source_dir = root.join("sources");
        fs::create_dir_all(&source_dir).expect("应创建图片来源目录");
        let first_source = source_dir.join("telegram-shot.PNG");
        let second_source = source_dir.join("article-photo.jpg");
        let third_source = source_dir.join("follow-up.webp");
        fs::write(&first_source, b"first image").expect("应写入第一张测试图片");
        fs::write(&second_source, b"second image").expect("应写入第二张测试图片");
        fs::write(&third_source, b"third image").expect("应写入第三张测试图片");

        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let article = create_article_skeleton_at(&root, "图片导入测试", None)?;
            let first_batch = import_article_images_at(
                &root,
                &article.folder_name,
                &[
                    first_source.to_string_lossy().into_owned(),
                    second_source.to_string_lossy().into_owned(),
                ],
            )?;
            assert_eq!(first_batch.article_id, article.id);
            assert_eq!(first_batch.images.len(), 2);
            assert_eq!(first_batch.images[0].file_name, "001.png");
            assert_eq!(
                first_batch.images[0].original_file_name,
                "telegram-shot.PNG"
            );
            assert_eq!(first_batch.images[1].file_name, "002.jpg");
            assert_eq!(first_batch.images[1].relative_path, "images/002.jpg");
            assert_eq!(
                fs::read(article.folder_path.clone() + "\\images\\001.png")
                    .map_err(|error| StorageError::io("无法读取第一张导入图片", error))?,
                b"first image"
            );

            let second_batch = import_article_images_at(
                &root,
                &article.id,
                &[third_source.to_string_lossy().into_owned()],
            )?;
            assert_eq!(second_batch.images[0].file_name, "003.webp");
            assert!(Path::new(&second_batch.images[0].path).is_file());
            Ok(())
        })();

        let cleanup = fs::remove_dir_all(&root);
        result.expect("图片导入应成功并保持顺序");
        cleanup.expect("应清理精确的测试目录");
    }

    #[test]
    fn rejects_unsupported_images_before_copying_any_file() {
        let root = temporary_library_root("image-validation");
        fs::create_dir_all(&root).expect("应创建测试目录");
        let source_dir = root.join("sources");
        fs::create_dir_all(&source_dir).expect("应创建图片来源目录");
        let valid_source = source_dir.join("valid.png");
        let invalid_source = source_dir.join("notes.txt");
        fs::write(&valid_source, b"valid").expect("应写入有效测试图片");
        fs::write(&invalid_source, b"invalid").expect("应写入无效测试文件");

        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let article = create_article_skeleton_at(&root, "校验测试", None)?;
            let error = import_article_images_at(
                &root,
                &article.folder_name,
                &[
                    valid_source.to_string_lossy().into_owned(),
                    invalid_source.to_string_lossy().into_owned(),
                ],
            )
            .expect_err("不支持的格式应被拒绝");
            assert_eq!(error.code, "UNSUPPORTED_IMAGE_FORMAT");
            assert!(fs::read_dir(Path::new(&article.folder_path).join("images"))
                .expect("应读取图片目录")
                .next()
                .is_none());
            Ok(())
        })();

        let cleanup = fs::remove_dir_all(&root);
        result.expect("图片格式校验应成功阻止非法导入");
        cleanup.expect("应清理精确的测试目录");
    }

    #[test]
    fn rejects_empty_image_selection_and_invalid_article_reference() {
        let root = temporary_library_root("image-errors");
        fs::create_dir_all(&root).expect("应创建测试目录");
        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let empty_error =
                import_article_images_at(&root, "missing", &[]).expect_err("空图片列表应被拒绝");
            assert_eq!(empty_error.code, "NO_IMAGES_SELECTED");

            let invalid_reference =
                import_article_images_at(&root, "../../outside", &["C:\\missing.png".to_string()])
                    .expect_err("非法文章标识应被拒绝");
            assert_eq!(invalid_reference.code, "INVALID_ARTICLE_REFERENCE");
            Ok(())
        })();

        let cleanup = fs::remove_dir_all(&root);
        result.expect("图片导入错误应有稳定错误码");
        cleanup.expect("应清理精确的测试目录");
    }

    #[test]
    fn saves_and_reads_article_document_fields() {
        let root = temporary_library_root("article-document");
        fs::create_dir_all(&root).expect("应创建测试目录");

        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let article = create_article_skeleton_at(&root, "原始标题", None)?;
            let saved = save_article_at(
                &root,
                &article.folder_name,
                ArticleDraft {
                    title: "更新后的标题".to_string(),
                    summary: "一段摘要".to_string(),
                    source_url: "https://example.com".to_string(),
                    tags: vec!["AI".to_string(), "AI".to_string(), "写作".to_string()],
                    content: "## 小节\n\n正文内容".to_string(),
                    notes: "我的备注".to_string(),
                },
            )?;
            assert_eq!(saved.title, "更新后的标题");
            assert_eq!(saved.tags, vec!["AI", "写作"]);

            let loaded = read_article_at(&root, &article.folder_name)?;
            assert_eq!(loaded.title, "更新后的标题");
            assert_eq!(loaded.summary, "一段摘要");
            assert_eq!(loaded.source_url, "https://example.com");
            assert_eq!(loaded.content, "## 小节\n\n正文内容");
            assert_eq!(loaded.notes, "我的备注");
            Ok(())
        })();

        let cleanup = fs::remove_dir_all(&root);
        result.expect("文章字段保存与读取应成功");
        cleanup.expect("应清理精确的测试目录");
    }

    #[test]
    fn lists_articles_by_searchable_fields() {
        let root = temporary_library_root("article-list");
        fs::create_dir_all(&root).expect("应创建测试目录");
        let result = (|| -> StorageResult<()> {
            initialize_library_at(&root)?;
            let article =
                create_article_skeleton_at(&root, "视觉工具", Some("https://example.com/vision"))?;
            save_article_at(
                &root,
                &article.folder_name,
                ArticleDraft {
                    title: "视觉工具".to_string(),
                    summary: "图片识别整理".to_string(),
                    source_url: "https://example.com/vision".to_string(),
                    tags: vec!["AI".to_string()],
                    content: "模型提示词".to_string(),
                    notes: "收藏".to_string(),
                },
            )?;
            let matches = list_articles_at(&root, "提示词")?;
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].title, "视觉工具");
            assert_eq!(list_articles_at(&root, "不存在")?.len(), 0);
            Ok(())
        })();
        let cleanup = fs::remove_dir_all(&root);
        result.expect("文章列表搜索应成功");
        cleanup.expect("应清理精确的测试目录");
    }
}
