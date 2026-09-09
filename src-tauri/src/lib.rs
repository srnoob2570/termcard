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

/// Estado global: bandera de stop para la captura en curso.
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
            STORE_PATH.get().cloned().expect("store no inicializado")
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

/// Ejecuta el comando en una PTY y devuelve la captura como JSON.
/// `stop` de capturas previas se resetea al iniciar.
#[tauri::command]
async fn run_capture(
    state: State<'_, AppState>,
    command: String,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
    rules: Vec<redact::RedactRule>,
) -> Result<serde_json::Value, String> {
    {
        let mut running = state.running.lock().await;
        if *running {
            return Err("Ya hay una captura en curso".into());
        }
        *running = true;
    }

    state
        .stop
        .store(false, std::sync::atomic::Ordering::Relaxed);

    // La PTY es bloqueante; correr en hilo aparte para no bloquear el runtime.
    let result = tokio::task::spawn_blocking(move || {
        capture::run_command(&command, cwd.as_deref(), cols, rows, None)
            .map(|out: RunOutcome| serde_json::to_value(&out.capture).unwrap())
    })
    .await
    .map_err(|e| e.to_string())?;

    // Redacción se aplica sobre el IR completo antes de devolver.
    let redaction = Redaction { rules };
    let redacted = result
        .map(|mut v| {
            apply_redaction_to_json(&mut v, &redaction);
            v
        })
        .map_err(|e: capture::CaptureError| e.0);

    *state.running.lock().await = false;
    redacted
}

#[tauri::command]
fn stop_capture(state: State<'_, AppState>) {
    state.stop.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Aplica las reglas de redacción sobre commandLine y el texto de cada run.
fn apply_redaction_to_json(v: &mut serde_json::Value, r: &Redaction) {
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

/// Genera el PNG de la captura a la escala pedida y lo devuelve en base64.
#[tauri::command]
fn export_png(capture: serde_json::Value, theme: Theme, scale: u32) -> Result<String, String> {
    if !matches!(scale, 2 | 3 | 4) {
        return Err(format!("escala inválida: {scale}"));
    }
    let cap: ir::Capture = serde_json::from_value(capture).map_err(|e| e.to_string())?;
    let png = export::render_png(&cap, &theme, &Palette::default(), scale)?;
    use base64::Engine as _;
    Ok(base64::engine::general_purpose::STANDARD.encode(png))
}

/// Reglas por defecto generadas del entorno real del usuario.
#[tauri::command]
fn default_rules() -> Vec<redact::RedactRule> {
    redact::default_rules()
}

#[tauri::command]
fn get_home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

/// Abre un diálogo de guardado y escribe el PNG (base64) en la ruta elegida.
#[tauri::command]
async fn save_png(
    app: AppHandle,
    base64_png: String,
    suggested_name: String,
) -> Result<String, String> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_png)
        .map_err(|e| format!("base64 inválido: {e}"))?;

    let path = rfd_dialog(&app, &suggested_name).await?;
    if path.is_empty() {
        return Ok(String::new()); // usuario canceló
    }
    std::fs::write(&path, bytes).map_err(|e| format!("no se pudo escribir {path}: {e}"))?;
    Ok(path)
}

async fn rfd_dialog(app: &AppHandle, name: &str) -> Result<String, String> {
    let dialog = app.dialog().file().set_file_name(name);
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
            default_rules,
            get_home,
            save_png
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
