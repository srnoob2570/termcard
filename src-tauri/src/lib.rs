pub mod capture;
pub mod export;
pub mod ir;
pub mod redact;
pub mod theme;

use capture::RunOutcome;
use once_store::Store;
use redact::Redaction;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use theme::{Palette, Theme};
use tokio::sync::Mutex;

/// Global state: stop flag for the capture in progress.
pub struct AppState {
    pub stop: Arc<AtomicBool>,
    pub running: Mutex<bool>,
}

mod once_store {
    use std::path::PathBuf;
    use std::sync::OnceLock;

    static STORE_PATH: OnceLock<PathBuf> = OnceLock::new();

    pub struct Store;

    impl Store {
        pub fn init(app_dir: PathBuf) {
            let _ = STORE_PATH.set(app_dir.join("termcard-store.json"));
        }

        fn path() -> PathBuf {
            STORE_PATH.get().cloned().expect("store not initialized")
        }

        pub fn load_json() -> serde_json::Value {
            std::fs::read_to_string(Self::path())
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::json!({}))
        }

        pub fn save_json(value: &serde_json::Value) -> Result<(), String> {
            let path = Self::path();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
            std::fs::write(path, text).map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
fn get_prefs() -> serde_json::Value {
    Store::load_json()
}

#[tauri::command]
fn save_prefs(prefs: serde_json::Value) -> Result<(), String> {
    Store::save_json(&prefs)
}

/// Runs the command in a PTY and returns the capture as UNREDACTED JSON:
/// redaction is applied at render time (preview/export) so that editing
/// rules updates the preview without re-capturing.
#[tauri::command]
async fn run_capture(
    state: State<'_, AppState>,
    command: String,
    cwd: Option<String>,
) -> Result<serde_json::Value, String> {
    {
        let mut running = state.running.lock().await;
        if *running {
            return Err("A capture is already in progress".into());
        }
        *running = true;
    }

    state
        .stop
        .store(false, std::sync::atomic::Ordering::Relaxed);

    // The PTY is blocking; run it on a separate thread so the runtime isn't blocked.
    let result = tokio::task::spawn_blocking(move || {
        capture::run_command(&command, cwd.as_deref(), 240, 80, None)
            .map(|out: RunOutcome| {
                serde_json::to_value(&out.capture)
                    .map_err(|e| capture::CaptureError(format!("IR serialization: {e}")))
            })
            .and_then(|v| v)
    })
    .await
    .map_err(|e| e.to_string())?;
    *state.running.lock().await = false;
    result.map_err(|e: capture::CaptureError| e.0)
}

#[tauri::command]
fn stop_capture(state: State<'_, AppState>) {
    state.stop.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Applies the redaction rules to commandLine and the text of every run.
fn redact_capture_value(v: &mut serde_json::Value, r: &Redaction) {
    if let Some(cl) = v.get_mut("commandLine").and_then(|c| c.as_str()) {
        let redacted = r.apply(cl);
        v["commandLine"] = serde_json::Value::String(redacted);
    }
    if let Some(lines) = v.get_mut("lines").and_then(|l| l.as_array_mut()) {
        for line in lines {
            if let Some(runs) = line.get_mut("runs").and_then(|rr| rr.as_array_mut()) {
                for run in runs {
                    if let Some(text) = run.get_mut("text").and_then(|t| t.as_str()) {
                        let redacted = r.apply(text);
                        run["text"] = serde_json::Value::String(redacted);
                    }
                }
            }
        }
    }
}

/// Generates the PNG of the capture at the requested scale and returns it in base64.
/// Always applies the saved rules: the export never shows uncensored.
///
/// The render (fontdb + resvg + encode) is CPU-intensive and blocking; it runs in
/// `spawn_blocking` to avoid freezing the main thread (the UI would stop
/// responding and the export spinner would never get painted).
#[tauri::command]
async fn export_png(
    capture: serde_json::Value,
    theme: Theme,
    rules: Vec<redact::RedactRule>,
    scale: u32,
) -> Result<String, String> {
    if !matches!(scale, 2..=4) {
        return Err(format!("invalid scale: {scale}"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut cap = capture;
        redact_capture_value(&mut cap, &Redaction { rules });
        let cap: ir::Capture = serde_json::from_value(cap).map_err(|e| e.to_string())?;
        let png = export::render_png(&cap, &theme, &Palette::default(), scale)?;
        use base64::Engine as _;
        Ok(base64::engine::general_purpose::STANDARD.encode(png))
    })
    .await
    .map_err(|e| format!("export cancelled: {e}"))?
}

/// The exact SVG the exporter rasterizes; the preview displays it as-is,
/// so preview and PNG can't diverge. Only the preview requests `showRaw`
/// for the temporary "show uncensored" toggle.
///
/// Same as `export_png`: the render goes to `spawn_blocking` so the main
/// thread isn't blocked while typing in the theme/rules panel.
#[tauri::command]
async fn export_svg(
    capture: serde_json::Value,
    theme: Theme,
    rules: Vec<redact::RedactRule>,
    show_raw: Option<bool>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut cap = capture;
        if !show_raw.unwrap_or(false) {
            redact_capture_value(&mut cap, &Redaction { rules });
        }
        let cap: ir::Capture = serde_json::from_value(cap).map_err(|e| e.to_string())?;
        Ok(export::render_svg(&cap, &theme, &Palette::default(), 1))
    })
    .await
    .map_err(|e| format!("export cancelled: {e}"))?
}

/// Full theme of a preset. The frontend replaces its whole theme when
/// switching presets: partial overrides left leftovers of the previous one.
#[tauri::command]
fn preset_theme(preset: &str) -> Result<Theme, String> {
    theme::Preset::from_id(preset)
        .map(|p| p.theme())
        .ok_or_else(|| format!("unknown preset: {preset}"))
}

/// Default rules generated from the user's real environment.
#[tauri::command]
fn default_rules() -> Vec<redact::RedactRule> {
    redact::default_rules()
}

#[tauri::command]
fn get_home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

/// Opens a save dialog and writes the PNG (base64) to the chosen path.
#[tauri::command]
async fn save_png(
    app: AppHandle,
    base64_png: String,
    suggested_name: String,
) -> Result<String, String> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_png)
        .map_err(|e| format!("invalid base64: {e}"))?;

    let path = rfd_dialog(&app, &suggested_name).await?;
    if path.is_empty() {
        return Ok(String::new()); // user cancelled
    }
    std::fs::write(&path, bytes).map_err(|e| format!("could not write {path}: {e}"))?;
    Ok(path)
}

async fn rfd_dialog(app: &AppHandle, name: &str) -> Result<String, String> {
    // Starts at ~/Pictures (or $HOME if XDG doesn't define it) instead of the
    // directory the process was launched from.
    let start_dir = app.path().picture_dir().map_err(|e| e.to_string())?;
    let dialog = app
        .dialog()
        .file()
        .set_directory(start_dir)
        .set_file_name(name);
    match dialog.blocking_save_file() {
        Some(p) => Ok(p.to_string()),
        None => Ok(String::new()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            once_store::Store::init(dir);
            Ok(())
        })
        .manage(AppState {
            stop: Arc::new(AtomicBool::new(false)),
            running: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_prefs,
            save_prefs,
            run_capture,
            stop_capture,
            export_png,
            export_svg,
            preset_theme,
            default_rules,
            get_home,
            save_png
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
