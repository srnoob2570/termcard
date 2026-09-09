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

/// Ejecuta el comando en una PTY y devuelve la captura como JSON SIN
/// redactar: la redacción se aplica al renderizar (preview/export) para que
/// editar reglas actualice el preview sin re-capturar.
#[tauri::command]
async fn run_capture(
    state: State<'_, AppState>,
    command: String,
    cwd: Option<String>,
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
        capture::run_command(&command, cwd.as_deref(), 240, 80, None)
            .map(|out: RunOutcome| {
                serde_json::to_value(&out.capture)
                    .map_err(|e| capture::CaptureError(format!("serialización IR: {e}")))
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

/// Aplica las reglas de redacción sobre commandLine y el texto de cada run.
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

/// Genera el PNG de la captura a la escala pedida y lo devuelve en base64.
/// Siempre aplica las reglas guardadas: el export nunca muestra sin censura.
///
/// El render (fontdb + resvg + encode) es CPU-intensivo y bloqueante; corre en
/// `spawn_blocking` para no congelar el hilo principal (la UI dejaría de
/// responder y el spinner de exportación no llegaría a pintarse).
#[tauri::command]
async fn export_png(
    capture: serde_json::Value,
    theme: Theme,
    rules: Vec<redact::RedactRule>,
    scale: u32,
) -> Result<String, String> {
    if !matches!(scale, 2..=4) {
        return Err(format!("escala inválida: {scale}"));
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
    .map_err(|e| format!("export cancelado: {e}"))?
}

/// SVG exacto que el exportador rasteriza; el preview lo muestra tal cual,
/// así preview y PNG no pueden divergir. `showRaw` solo lo pide el preview
/// para el toggle temporal "mostrar sin censura".
///
/// Igual que `export_png`: el render va a `spawn_blocking` para no bloquear el
/// hilo principal mientras se escribe en el panel de tema/reglas.
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
    .map_err(|e| format!("export cancelado: {e}"))?
}

/// Tema completo de un preset. El frontend reemplaza su tema entero al
/// cambiar de preset: los overrides parciales dejaban restos del anterior.
#[tauri::command]
fn preset_theme(preset: &str) -> Result<Theme, String> {
    theme::Preset::from_id(preset)
        .map(|p| p.theme())
        .ok_or_else(|| format!("preset desconocido: {preset}"))
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
    // Arranca en ~/Imágenes (o $HOME si XDG no la define) en vez del directorio
    // desde el que se lanzó el proceso.
    let inicio = app.path().picture_dir().map_err(|e| e.to_string())?;
    let dialog = app
        .dialog()
        .file()
        .set_directory(inicio)
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
