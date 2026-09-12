mod clipboard;
mod model;
mod storage;
mod tray;
mod window_state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Acquire the application identity before a second window or tray is created.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::restore_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(window_state::plugin())
        .setup(|app| {
            // Keep normal window closing available if the system tray cannot be created.
            if let Err(error) = tray::setup(app) {
                eprintln!("Unable to create ShiLu system tray: {error}");
            }
            // Restore geometry before the first visible frame.
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
                window.set_focus()?;
            }
            Ok(())
        })
        .on_window_event(tray::on_window_event)
        .invoke_handler(tauri::generate_handler![
            storage::get_library_status,
            storage::initialize_library,
            storage::migrate_library,
            storage::create_article_skeleton,
            storage::create_article_with_sources,
            storage::move_article_to_trash,
            storage::import_article_images,
            storage::import_article_sources,
            clipboard::read_clipboard_image,
            storage::read_article,
            storage::save_article,
            storage::list_articles,
            storage::ocr_article_images,
            storage::get_app_settings,
            storage::set_theme_preference,
            model::save_model_settings,
            model::fetch_model_list,
            model::test_model,
            model::ocr_with_model,
            model::ocr_article_with_model,
            model::polish_with_model
        ])
        .run(tauri::generate_context!())
        .expect("error while running ShiLu application");
}
