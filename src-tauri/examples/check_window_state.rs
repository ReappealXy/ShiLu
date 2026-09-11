//! Native Tauri API restart checks. This does not test mouse interaction with the tray.
//! Every invocation uses its own application identifier and never touches ShiLu data.

#[path = "../src/window_state.rs"]
mod window_state;

use std::{
    path::PathBuf,
    process::{Command, ExitCode},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, LogicalSize, Manager, RunEvent, WebviewUrl, WebviewWindow};

type CheckResult = Result<(), Box<dyn std::error::Error>>;

fn settle() {
    thread::sleep(Duration::from_millis(600));
}

fn check(condition: bool, message: impl Into<String>) -> CheckResult {
    if condition {
        Ok(())
    } else {
        Err(message.into().into())
    }
}

fn check_size(window: &WebviewWindow, width: f64, height: f64) -> CheckResult {
    let actual = window
        .inner_size()?
        .to_logical::<f64>(window.scale_factor()?);
    check(
        (actual.width - width).abs() <= 2.0 && (actual.height - height).abs() <= 2.0,
        format!(
            "expected normal size {width}x{height}, observed {}x{}",
            actual.width, actual.height
        ),
    )
}

fn check_centered(window: &WebviewWindow) -> CheckResult {
    check(
        !window.is_maximized()? && !window.is_fullscreen()?,
        "centering check requires an ordinary window",
    )?;
    let monitor = window.current_monitor()?.ok_or("missing current monitor")?;
    let work_area = monitor.work_area();
    let position = window.outer_position()?;
    let size = window.outer_size()?;

    // Tauri centers in the monitor work area. Allow the small difference between
    // Windows' invisible resize border and the visible frame used by Tauri.

    if size.width > work_area.size.width || size.height > work_area.size.height {
        println!(
            "CENTER skipped: window {}x{} exceeds work area {}x{}",
            size.width, size.height, work_area.size.width, work_area.size.height
        );
        return Ok(());
    }
    let expected_x = work_area.position.x + (work_area.size.width - size.width) as i32 / 2;
    let expected_y = work_area.position.y + (work_area.size.height - size.height) as i32 / 2;
    println!(
        "CENTER actual=({}, {}) expected=({}, {}) frame={}x{} work_area=({}, {}, {}x{})",
        position.x,
        position.y,
        expected_x,
        expected_y,
        size.width,
        size.height,
        work_area.position.x,
        work_area.position.y,
        work_area.size.width,
        work_area.size.height
    );
    check(
        (position.x - expected_x).abs() <= 16 && (position.y - expected_y).abs() <= 16,
        format!(
            "restored window is not centered: expected ({expected_x}, {expected_y}), observed ({}, {})",
            position.x, position.y
        ),
    )
}

fn check_visible(window: &WebviewWindow) -> CheckResult {
    check(window.is_visible()?, "startup window must be visible")?;
    check(
        !window.is_minimized()?,
        "startup window must not be minimized",
    )
}

fn normal_size(window: &WebviewWindow) -> CheckResult {
    window.set_fullscreen(false)?;
    window.unmaximize()?;
    window.unminimize()?;
    settle();
    window.set_size(LogicalSize::new(1200.0, 740.0))?;
    settle();
    check_size(window, 1200.0, 740.0)
}

fn save(app: &AppHandle) -> CheckResult {
    window_state::save(app);
    let path = app.path().app_config_dir()?.join(".window-state.json");
    let state: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    check(
        state.get("main").is_some(),
        "saved state has no main window",
    )?;
    println!("STATE {} {}", path.display(), state);
    Ok(())
}

fn run_phase(app: &AppHandle, phase: &str) -> CheckResult {
    let window = app
        .get_webview_window("main")
        .ok_or("missing main window")?;
    settle();
    check_visible(&window)?;
    match phase {
        "normal-save" => {
            normal_size(&window)?;
            save(app)?;
        }
        "normal-restore-max-save" => {
            check(
                !window.is_maximized()?,
                "ordinary window unexpectedly maximized",
            )?;
            check_size(&window, 1200.0, 740.0)?;
            check_centered(&window)?;
            window.maximize()?;
            settle();
            check(window.is_maximized()?, "maximize did not take effect")?;
            window.hide()?;
            save(app)?;
        }
        "max-restore" => {
            check(window.is_maximized()?, "maximized state was not restored")?;
            window.unmaximize()?;
            settle();
            check_size(&window, 1200.0, 740.0)?;
        }
        "fullscreen-save" => {
            normal_size(&window)?;
            window.set_fullscreen(true)?;
            settle();
            check(window.is_fullscreen()?, "fullscreen did not take effect")?;
            window.hide()?;
            save(app)?;
        }
        "fullscreen-restore" => {
            check(window.is_fullscreen()?, "fullscreen state was not restored")?;
            window.set_fullscreen(false)?;
            settle();
            check_size(&window, 1200.0, 740.0)?;
        }
        "hidden-save" => {
            normal_size(&window)?;
            window.hide()?;
            check(!window.is_visible()?, "hide did not take effect")?;
            save(app)?;
        }
        "hidden-restore" => {
            check_size(&window, 1200.0, 740.0)?;
            check_centered(&window)?;
        }
        "minimized-save" => {
            normal_size(&window)?;
            window.maximize()?;
            settle();
            window.minimize()?;
            settle();
            check(window.is_minimized()?, "minimize did not take effect")?;
            save(app)?;
        }
        "minimized-restore" => {
            check(
                window.is_maximized()?,
                "maximized state lost after minimizing",
            )?;
            window.unmaximize()?;
            settle();
            check_size(&window, 1200.0, 740.0)?;
        }
        "corrupt-save" => save(app)?,
        "corrupt-restore" => {
            check(
                !window.is_maximized()?,
                "corrupt state unexpectedly maximized",
            )?;
            check(
                !window.is_fullscreen()?,
                "corrupt state unexpectedly fullscreen",
            )?;
            check_size(&window, 1440.0, 900.0)?;
        }
        _ => return Err(format!("unknown phase {phase}").into()),
    }
    println!("PASS {phase}");
    Ok(())
}

fn child(phase: String, identifier: String, report: PathBuf) -> i32 {
    if !identifier.starts_with("com.shilu.window-state-check.") {
        eprintln!("refusing non-test application identifier");
        return 2;
    }
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(15));
        eprintln!("FAIL native window phase exceeded 15 seconds");
        std::process::exit(124);
    });
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = identifier;
    context.config_mut().app.windows[0].visible = false;
    context.config_mut().app.windows[0].title = format!("ShiLu isolated window check: {phase}");
    // Geometry checks do not need the application UI or a development HTTP server.
    context.config_mut().app.windows[0].url =
        WebviewUrl::External("about:blank".parse().expect("valid blank URL"));
    let app = tauri::Builder::default()
        .plugin(window_state::plugin())
        .setup(move |app| {
            let window = app
                .get_webview_window("main")
                .ok_or("missing main window")?;
            window.show()?;
            Ok(())
        })
        .build(context)
        .expect("build isolated Tauri application");
    app.run(move |app, event| {
        if matches!(event, RunEvent::Ready) {
            let app = app.clone();
            let phase = phase.clone();
            let report = report.clone();
            thread::spawn(move || {
                let result = run_phase(&app, &phase);
                let error_message = result.as_ref().err().map(ToString::to_string);
                let exit_code = if let Err(error) = &result {
                    eprintln!("FAIL {phase}: {error}");
                    1
                } else {
                    0
                };
                // Tauri may terminate the process inside its event loop before run() returns.
                // A report written before exit is authoritative, not the OS exit code.
                let state_path = app
                    .path()
                    .app_config_dir()
                    .unwrap()
                    .join(".window-state.json");
                let summary = serde_json::json!({
                    "phase": phase,
                    "passed": result.is_ok(),
                    "error": error_message,
                    "state_path": state_path,
                });
                std::fs::write(report, serde_json::to_vec_pretty(&summary).unwrap())
                    .expect("write isolated phase report");
                app.exit(exit_code);
            });
        }
    });
    0
}

fn parent() -> CheckResult {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let session = format!("{}.{}", std::process::id(), nonce);
    let executable = std::env::current_exe()?;
    let report_directory = std::env::temp_dir().join(format!("shilu-window-state-check-{session}"));
    std::fs::create_dir_all(&report_directory)?;
    println!("REPORTS {}", report_directory.display());
    let phases = [
        ("normal-save", "normal"),
        ("normal-restore-max-save", "normal"),
        ("max-restore", "normal"),
        ("fullscreen-save", "fullscreen"),
        ("fullscreen-restore", "fullscreen"),
        ("hidden-save", "hidden"),
        ("hidden-restore", "hidden"),
        ("minimized-save", "minimized"),
        ("minimized-restore", "minimized"),
        ("corrupt-save", "corrupt"),
        ("corrupt-restore", "corrupt"),
    ];
    let mut failures = Vec::new();
    for (phase, group) in phases {
        let identifier = format!("com.shilu.window-state-check.{session}.{group}");
        let report_path = report_directory.join(format!("{phase}.json"));
        println!("RUN {phase}: {identifier}");
        let mut process = Command::new(&executable)
            .args(["--phase", phase, &identifier])
            .arg(&report_path)
            .spawn()?;
        let started = Instant::now();
        loop {
            if let Some(status) = process.try_wait()? {
                let report = std::fs::read(&report_path)
                    .ok()
                    .and_then(|data| serde_json::from_slice::<serde_json::Value>(&data).ok());
                match report {
                    Some(report) if report["passed"] == true && status.success() => {
                        if phase == "corrupt-save" {
                            let state_path = PathBuf::from(report["state_path"].as_str().unwrap());
                            check(
                                state_path
                                    .parent()
                                    .and_then(|p| p.file_name())
                                    .and_then(|n| n.to_str())
                                    == Some(identifier.as_str()),
                                "refusing malformed fixture outside isolated test config",
                            )?;
                            std::fs::write(&state_path, b"{ intentionally broken json")?;
                            println!("CORRUPT_FIXTURE {}", state_path.display());
                        }
                    }
                    Some(report) => {
                        failures.push(format!("{phase}: {}; process {status}", report["error"]))
                    }
                    None => failures.push(format!("{phase}: no phase report; process {status}")),
                }
                break;
            }
            if started.elapsed() > Duration::from_secs(20) {
                process.kill()?;
                process.wait()?;
                failures.push(format!("{phase}: parent timeout"));
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    println!(
        "Native Tauri API restart checks: {} passed, {} failed. Not tray mouse acceptance.",
        phases.len() - failures.len(),
        failures.len()
    );
    check(failures.is_empty(), failures.join("\n"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--phase") && args.len() == 5 {
        return ExitCode::from(
            child(args[2].clone(), args[3].clone(), PathBuf::from(&args[4])) as u8,
        );
    }
    match parent() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
