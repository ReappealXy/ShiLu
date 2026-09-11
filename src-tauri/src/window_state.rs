use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path, sync::Mutex};
use tauri::{
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, PhysicalSize, RunEvent, Runtime, Window, WindowEvent,
};

const STATE_FILE: &str = ".window-state.json";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct WindowState {
    width: u32,
    height: u32,
    maximized: bool,
    fullscreen: bool,
}

#[derive(Deserialize, Serialize)]
struct SavedState {
    main: WindowState,
}

#[derive(Default)]
struct StateCache(Mutex<Option<WindowState>>);

impl WindowState {
    fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0 && self.width <= 32768 && self.height <= 32768
    }

    fn update(
        &mut self,
        size: PhysicalSize<u32>,
        minimized: bool,
        maximized: bool,
        fullscreen: bool,
    ) {
        // Windows reports a minimized maximized window as non-maximized. Keep its last mode.
        if minimized {
            return;
        }
        self.fullscreen = fullscreen;
        if fullscreen {
            return;
        }
        self.maximized = maximized;
        // Fullscreen and maximized rectangles must never replace the normal restore size.
        if !maximized && size.width > 0 && size.height > 0 {
            self.width = size.width;
            self.height = size.height;
        }
    }
}

fn load(path: &Path) -> Option<WindowState> {
    let saved: SavedState = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    saved.main.is_valid().then_some(saved.main)
}

fn write(path: &Path, state: &WindowState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(&SavedState {
        main: state.clone(),
    })?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temporary, bytes)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

fn capture<R: Runtime>(window: &Window<R>) -> tauri::Result<()> {
    let minimized = window.is_minimized()?;
    let maximized = window.is_maximized()?;
    let fullscreen = window.is_fullscreen()?;
    let size = window.inner_size()?;
    let cache = window.state::<StateCache>();
    if let Some(state) = cache.0.lock().unwrap().as_mut() {
        state.update(size, minimized, maximized, fullscreen);
    }
    Ok(())
}

fn restore<R: Runtime>(window: &Window<R>) -> tauri::Result<()> {
    let cached = window.state::<StateCache>().0.lock().unwrap().clone();
    if let Some(state) = cached {
        window.set_size(PhysicalSize::new(state.width, state.height))?;
        // Config centering used the default size; recenter the restored normal rectangle.
        window.center()?;
        if state.maximized {
            window.maximize()?;
        }
        if state.fullscreen {
            window.set_fullscreen(true)?;
        }
    } else {
        let size = window.inner_size()?;
        *window.state::<StateCache>().0.lock().unwrap() = Some(WindowState {
            width: size.width,
            height: size.height,
            maximized: window.is_maximized()?,
            fullscreen: window.is_fullscreen()?,
        });
        window.center()?;
    }
    Ok(())
}

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("shilu-window-state")
        .setup(|app, _| {
            let state = app
                .path()
                .app_config_dir()
                .ok()
                .and_then(|dir| load(&dir.join(STATE_FILE)));
            app.manage(StateCache(Mutex::new(state)));
            Ok(())
        })
        .on_window_ready(|window| {
            if window.label() == "main" {
                if let Err(error) = restore(&window) {
                    eprintln!("Unable to restore ShiLu window state: {error}");
                }
            }
        })
        .on_event(|app, event| match event {
            RunEvent::WindowEvent { label, event, .. } if label == "main" => {
                if matches!(
                    event,
                    WindowEvent::Resized(_)
                        | WindowEvent::ScaleFactorChanged { .. }
                        | WindowEvent::Focused(false)
                ) {
                    if let Some(window) = app.get_webview_window("main") {
                        if let Err(error) = capture(&window.as_ref().window()) {
                            eprintln!("Unable to track ShiLu window state: {error}");
                        }
                    }
                }
            }
            RunEvent::Exit => save(app),
            _ => {}
        })
        .build()
}

pub fn save<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = capture(&window.as_ref().window()) {
            eprintln!("Unable to capture ShiLu window state: {error}");
        }
    }
    let state = app.state::<StateCache>().0.lock().unwrap().clone();
    if let Some(state) = state {
        let result = app
            .path()
            .app_config_dir()
            .map_err(io::Error::other)
            .and_then(|dir| write(&dir.join(STATE_FILE), &state));
        if let Err(error) = result {
            eprintln!("Unable to save ShiLu window state: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normal() -> WindowState {
        WindowState {
            width: 1200,
            height: 740,
            maximized: false,
            fullscreen: false,
        }
    }

    #[test]
    fn maximized_then_minimized_keeps_last_display_mode() {
        let mut state = normal();
        state.update(PhysicalSize::new(2240, 1400), false, true, false);
        state.update(PhysicalSize::new(0, 0), true, false, false);
        assert!(state.maximized);
        assert_eq!((state.width, state.height), (1200, 740));
    }

    #[test]
    fn fullscreen_does_not_overwrite_normal_size_or_previous_maximization() {
        let mut state = normal();
        state.maximized = true;
        state.update(PhysicalSize::new(2240, 1400), false, false, true);
        assert!(state.fullscreen && state.maximized);
        assert_eq!((state.width, state.height), (1200, 740));
    }

    #[test]
    fn normal_resize_clears_modes_and_saves_new_dimensions() {
        let mut state = normal();
        state.maximized = true;
        state.fullscreen = true;
        state.update(PhysicalSize::new(1280, 800), false, false, false);
        assert_eq!(
            state,
            WindowState {
                width: 1280,
                height: 800,
                maximized: false,
                fullscreen: false
            }
        );
    }

    #[test]
    fn persisted_state_has_no_hidden_or_minimized_flag() {
        let value = serde_json::to_value(normal()).unwrap();
        assert!(value.get("visible").is_none());
        assert!(value.get("minimized").is_none());
        assert!(!WindowState {
            width: 0,
            ..normal()
        }
        .is_valid());
    }
}
