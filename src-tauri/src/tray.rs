use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Runtime, Window, WindowEvent,
};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ID: &str = "shilu-main-tray";
const OPEN_MENU_ID: &str = "shilu-tray-open";
const EXIT_MENU_ID: &str = "shilu-tray-exit";

pub fn setup<R: Runtime>(app: &App<R>) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text(OPEN_MENU_ID, "打开拾录")
        .separator()
        .text(EXIT_MENU_ID, "退出")
        .build()?;
    let icon = app
        .default_window_icon()
        .cloned()
        .unwrap_or_else(|| tauri::include_image!("icons/32x32.png"));

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("拾录 ShiLu")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_MENU_ID => restore_main_window(app),
            // Exiting the app bypasses the window-close-to-tray behavior.
            EXIT_MENU_ID => {
                crate::window_state::save(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                restore_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub(crate) fn restore_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        if let Err(error) = window.show() {
            eprintln!("Unable to show ShiLu window: {error}");
        }
        if let Err(error) = window.unminimize() {
            eprintln!("Unable to restore ShiLu window: {error}");
        }
        if let Err(error) = window.set_focus() {
            eprintln!("Unable to focus ShiLu window: {error}");
        }
    }
}

pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if window.label() != MAIN_WINDOW_LABEL {
            return;
        }
        // Closing to tray is not an app exit, so persist before hiding the window.
        crate::window_state::save(window.app_handle());
        if window.app_handle().tray_by_id(TRAY_ID).is_some() {
            // Only cancel closing after hiding succeeds and a restore entry is available.
            // Do not intercept app ExitRequested events (including explicit/system exits).
            match window.hide() {
                Ok(()) => api.prevent_close(),
                Err(error) => eprintln!("Unable to hide ShiLu window to tray: {error}"),
            }
        }
    }
}
