//! Isolated native-process regression for the production single-instance restore path.
//! No ShiLu documents, clipboard, user shortcuts, or production processes are touched.

#[path = "../src/tray.rs"]
mod tray;
#[path = "../src/window_state.rs"]
mod window_state;

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, ExitCode},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, RunEvent, WebviewUrl, WebviewWindow};

type CheckResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const PREFIX: &str = "com.shilu.single-instance-check.";
const SENTINEL: &str = r#"({
  documentToken: window.shiLuDocumentToken,
  draft: window.shiLuUnsavedDraft,
  input: document.getElementById('draft')?.value,
  selection: document.getElementById('draft')?.selectionStart,
  route: window.location.hash
})"#;

fn check(condition: bool, message: impl Into<String>) -> CheckResult {
    if condition {
        Ok(())
    } else {
        Err(message.into().into())
    }
}

fn settle() {
    thread::sleep(Duration::from_millis(350));
}

fn wait_for(mut condition: impl FnMut() -> bool, description: &str) -> CheckResult {
    let started = Instant::now();
    while !condition() {
        check(
            started.elapsed() < Duration::from_secs(6),
            format!("timeout: {description}"),
        )?;
        thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn evaluate(window: &WebviewWindow, javascript: &str) -> CheckResult<serde_json::Value> {
    let (send, receive) = mpsc::channel();
    window.eval_with_callback(javascript, move |result| {
        let _ = send.send(result);
    })?;
    Ok(serde_json::from_str(
        &receive.recv_timeout(Duration::from_secs(5))?,
    )?)
}

fn log_event(directory: &Path, event: &str) -> CheckResult {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("lifecycle.log"))?;
    writeln!(file, "{event} {}", std::process::id())?;
    Ok(())
}

fn wait_child(process: &mut Child, timeout: Duration) -> CheckResult {
    let started = Instant::now();
    loop {
        if let Some(status) = process.try_wait()? {
            return check(
                status.success(),
                format!("duplicate process failed: {status}"),
            );
        }
        if started.elapsed() > timeout {
            process.kill()?;
            process.wait()?;
            return Err("duplicate did not exit within the deadline".into());
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn launch_duplicate(executable: &Path, identifier: &str, directory: &Path) -> CheckResult {
    let mut process = Command::new(executable)
        .arg("--instance")
        .arg(identifier)
        .arg(directory)
        .arg("secondary")
        .spawn()?;
    wait_child(&mut process, Duration::from_secs(10))
}

fn normal(window: &WebviewWindow) -> CheckResult {
    window.set_fullscreen(false)?;
    window.unminimize()?;
    window.unmaximize()?;
    window.show()?;
    wait_for(
        || {
            !window.is_fullscreen().unwrap_or(true)
                && !window.is_maximized().unwrap_or(true)
                && !window.is_minimized().unwrap_or(true)
        },
        "normal test window",
    )?;
    settle();
    Ok(())
}

fn run_checks(
    app: &AppHandle,
    identifier: &str,
    directory: &Path,
    callbacks: &AtomicUsize,
) -> CheckResult<serde_json::Value> {
    let window = app
        .get_webview_window("main")
        .ok_or("missing primary main window")?;
    let executable = std::env::current_exe()?;
    let alternate = directory
        .join("alternate-location")
        .join("check_single_instance.exe");
    settle();
    check(window.is_visible()?, "primary window not visible")?;
    check(
        app.tray_by_id("shilu-main-tray").is_some(),
        "primary tray not created",
    )?;
    wait_for(
        || evaluate(&window, "Boolean(document.body)").is_ok_and(|value| value == true),
        "isolated blank document body",
    )?;
    let initialized = evaluate(
        &window,
        r#"(() => {
      document.body.innerHTML = '<textarea id="draft"></textarea>';
      window.shiLuDocumentToken = 'document-' + Date.now() + '-' + Math.random();
      window.shiLuUnsavedDraft = { title: 'Unsaved isolated draft', body: 'Do not reset this document', revision: 7 };
      const draft = document.getElementById('draft');
      draft.value = 'unsaved textarea content';
      draft.setSelectionRange(4, 4);
      location.hash = 'editor/unsaved-isolated-draft';
      return true;
    })()"#,
    )?;
    check(
        initialized == true,
        format!("WebView fixture initialization returned {initialized}"),
    )?;
    let original = evaluate(&window, SENTINEL)?;
    check(
        original["documentToken"].is_string(),
        "WebView document sentinel was not initialized",
    )?;
    let mut reports = Vec::new();
    for (index, phase) in [
        "visible",
        "close-to-tray",
        "minimized",
        "maximized-hidden",
        "maximized-minimized",
        "fullscreen-hidden",
        "alternate-executable",
    ]
    .into_iter()
    .enumerate()
    {
        normal(&window)?;
        let maximized = matches!(phase, "maximized-hidden" | "maximized-minimized");
        let fullscreen = phase == "fullscreen-hidden";
        if maximized {
            window.maximize()?;
            wait_for(
                || window.is_maximized().unwrap_or(false),
                "maximize before duplicate",
            )?;
        }
        if fullscreen {
            window.set_fullscreen(true)?;
            wait_for(
                || window.is_fullscreen().unwrap_or(false),
                "fullscreen before duplicate",
            )?;
        }
        match phase {
            "close-to-tray" | "maximized-hidden" | "fullscreen-hidden" | "alternate-executable" => {
                window.close()?;
                wait_for(
                    || !window.is_visible().unwrap_or(true),
                    "production close-to-tray handler",
                )?;
                check(
                    app.get_webview_window("main").is_some(),
                    "close destroyed primary document",
                )?;
            }
            "minimized" | "maximized-minimized" => {
                window.minimize()?;
                wait_for(
                    || window.is_minimized().unwrap_or(false),
                    "minimize before duplicate",
                )?;
            }
            _ => {}
        }
        launch_duplicate(
            if phase == "alternate-executable" {
                &alternate
            } else {
                &executable
            },
            identifier,
            directory,
        )?;
        wait_for(
            || callbacks.load(Ordering::SeqCst) == index + 1,
            "primary single-instance callback",
        )?;
        wait_for(
            || window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true),
            "existing main window restored",
        )?;
        settle();
        check(
            window.is_maximized()? == maximized,
            format!("{phase}: maximized state changed"),
        )?;
        check(
            window.is_fullscreen()? == fullscreen,
            format!("{phase}: fullscreen state changed"),
        )?;
        check(
            evaluate(&window, SENTINEL)? == original,
            format!("{phase}: real WebView document or unsaved draft reset"),
        )?;
        check(
            app.webview_windows().len() == 1,
            format!("{phase}: extra window created"),
        )?;
        check(
            app.tray_by_id("shilu-main-tray").is_some(),
            format!("{phase}: original tray missing"),
        )?;
        let lifecycle = fs::read_to_string(directory.join("lifecycle.log"))?;
        check(
            lifecycle
                .lines()
                .filter(|line| line.starts_with("setup "))
                .count()
                == 1,
            format!("{phase}: secondary process reached setup"),
        )?;
        check(
            lifecycle
                .lines()
                .filter(|line| line.starts_with("tray "))
                .count()
                == 1,
            format!("{phase}: secondary tray created"),
        )?;
        println!(
            "PASS {phase}: duplicate exited, original window restored, document and tray retained"
        );
        reports.push(serde_json::json!({ "phase": phase, "passed": true,
            "maximized": maximized, "fullscreen": fullscreen, "document_preserved": true }));
    }
    normal(&window)?;
    Ok(
        serde_json::json!({ "identifier": identifier, "primary_pid": std::process::id(),
        "callback_count": callbacks.load(Ordering::SeqCst), "phases": reports,
        "document_token": original["documentToken"], "setup_count": 1, "tray_count": 1 }),
    )
}

fn instance(identifier: String, directory: PathBuf, primary: bool) -> i32 {
    if !identifier.starts_with(PREFIX) || !directory.is_dir() {
        eprintln!("refusing non-isolated instance arguments");
        return 2;
    }
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(90));
        eprintln!("FAIL isolated single-instance watchdog");
        std::process::exit(124);
    });
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = identifier.clone();
    let config = &mut context.config_mut().app.windows[0];
    config.visible = false;
    config.title = "ShiLu isolated single-instance regression".into();
    config.url = WebviewUrl::External("about:blank".parse().unwrap());
    config.data_directory = Some(directory.join("webview-data"));
    let callbacks = Arc::new(AtomicUsize::new(0));
    let callback_counter = callbacks.clone();
    let setup_directory = directory.clone();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(move |app, _, _| {
            tray::restore_main_window(app);
            callback_counter.fetch_add(1, Ordering::SeqCst);
        }))
        .plugin(window_state::plugin())
        .setup(move |app| {
            log_event(&setup_directory, "setup")?;
            tray::setup(app)?;
            log_event(&setup_directory, "tray")?;
            app.get_webview_window("main")
                .ok_or("missing main window")?
                .show()?;
            Ok(())
        })
        .on_window_event(tray::on_window_event)
        .build(context)
        .expect("build isolated single-instance application");
    app.run(move |app, event| {
        if matches!(event, RunEvent::Ready) {
            if !primary {
                eprintln!("FAIL secondary instance reached Ready");
                app.exit(2);
                return;
            }
            let app = app.clone();
            let identifier = identifier.clone();
            let directory = directory.clone();
            let callbacks = callbacks.clone();
            thread::spawn(move || {
                let result = run_checks(&app, &identifier, &directory, &callbacks);
                let report = match &result {
                    Ok(details) => serde_json::json!({"passed": true, "details": details}),
                    Err(error) => serde_json::json!({"passed": false, "error": error.to_string()}),
                };
                fs::write(
                    directory.join("report.json"),
                    serde_json::to_vec_pretty(&report).unwrap(),
                )
                .expect("write isolated test report");
                if let Err(error) = &result {
                    eprintln!("FAIL {error}");
                }
                app.exit(if result.is_ok() { 0 } else { 1 });
            });
        }
    });
    0
}

fn parent() -> CheckResult {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let session = format!("{}.{}", std::process::id(), nonce);
    let identifier = format!("{PREFIX}{session}");
    let directory = std::env::temp_dir().join(format!("shilu-single-instance-check-{session}"));
    fs::create_dir_all(directory.join("alternate-location"))?;
    let executable = std::env::current_exe()?;
    fs::copy(
        &executable,
        directory
            .join("alternate-location")
            .join("check_single_instance.exe"),
    )?;
    println!("REPORTS {}", directory.display());
    let mut process = Command::new(&executable)
        .arg("--instance")
        .arg(identifier)
        .arg(&directory)
        .arg("primary")
        .spawn()?;
    wait_child(&mut process, Duration::from_secs(95))?;
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("report.json"))?)?;
    check(
        report["passed"] == true,
        format!("native regression failed: {report}"),
    )?;
    println!("Native single-instance checks: 7 passed, 0 failed. Isolated processes only.");
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--instance") && args.len() == 5 {
        return ExitCode::from(instance(
            args[2].clone(),
            PathBuf::from(&args[3]),
            args[4] == "primary",
        ) as u8);
    }
    match parent() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
