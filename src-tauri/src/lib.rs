mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            storage::get_library_status,
            storage::initialize_library,
            storage::migrate_library,
            storage::create_article_skeleton,
            storage::move_article_to_trash,
            storage::import_article_images,
            storage::read_article,
            storage::save_article,
            storage::list_articles,
            storage::ocr_article_images,
            storage::get_app_settings,
            storage::set_theme_preference
        ])
        .run(tauri::generate_context!())
        .expect("error while running ShiLu application");
}
