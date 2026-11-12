mod audio;
mod commands;
mod db;
mod dictation;
mod hotkeys;
mod injection;
mod logging;
mod model_manager;
mod models;
mod screenshot;
mod services;
mod settings;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{Emitter, Manager};

use audio::AudioEngine;
use db::Database;
use hotkeys::HotkeyService;
use models::Note;
use services::ServiceManager;

pub struct AppState {
    pub db: Arc<Database>,
    pub audio: Arc<AudioEngine>,
    pub services: Arc<ServiceManager>,
    pub http: reqwest::Client,
    pub hotkeys: Mutex<Option<HotkeyService>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_dir = logging::init().unwrap_or_else(|error| {
        eprintln!("Verbatim logging could not start: {error}");
        std::env::temp_dir()
    });
    tracing::info!(version = env!("CARGO_PKG_VERSION"), log_dir = %log_dir.display(), "starting Verbatim");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = logging::data_dir();
            std::fs::create_dir_all(&data_dir)
                .map_err(|error| format!("Could not create {}: {error}", data_dir.display()))?;
            let db = Arc::new(Database::open(&data_dir.join("notes.db"))?);
            seed_welcome_note(&db);

            let app_settings = db.get_settings();
            let http = reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(8))
                .timeout(Duration::from_secs(30 * 60))
                .build()
                .map_err(|error| format!("Build HTTP client: {error}"))?;
            let service_manager = ServiceManager::new(app.handle().clone())
                .map_err(|error| format!("Service supervisor: {error}"))?;
            let state = AppState {
                db: Arc::clone(&db),
                audio: Arc::new(AudioEngine::new()),
                services: Arc::clone(&service_manager),
                http,
                hotkeys: Mutex::new(None),
            };
            app.manage(state);
            service_manager.start();

            let hotkey_result = HotkeyService::start(
                app.handle().clone(),
                &app_settings.hotkey_dictation,
                &app_settings.hotkey_ocr,
            );
            match hotkey_result {
                Ok(service) => {
                    let state = app.state::<AppState>();
                    if let Ok(mut slot) = state.hotkeys.lock() {
                        *slot = Some(service);
                    }
                }
                Err(error) => {
                    tracing::error!(error = %error, "global hotkeys unavailable");
                    let _ = app.emit("hotkeys:error", error);
                }
            }

            let report = model_manager::check(app.handle());
            let _ = app.emit("models:status", &report);
            tracing::info!(
                ready = report.ready,
                ollama = report.ollama_installed,
                "model check complete"
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::save_note,
            commands::get_notes,
            commands::list_notes,
            commands::delete_note,
            commands::list_messages,
            commands::save_message,
            commands::get_settings,
            commands::save_settings,
            commands::set_hotkeys,
            commands::dictate_start,
            commands::dictate_stop,
            commands::screenshot_ocr,
            commands::capture_region_ocr,
            commands::undo_last_dictation,
            commands::check_models,
            commands::get_setup_status,
            commands::download_model,
            commands::delete_model,
            commands::service_status,
            commands::restart_service,
            commands::transcribe_path,
            commands::ocr_path,
            commands::ocr_pdf_path,
            commands::searchable_pdf_path,
            commands::read_image_preview,
            commands::process_url,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Verbatim");

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                state.services.shutdown();
            }
            tracing::info!("Verbatim exiting");
        }
    });
}

fn seed_welcome_note(db: &Database) {
    if db.get_notes(None).map(|notes| !notes.is_empty()).unwrap_or(true) {
        return;
    }
    let markdown = r#"# Welcome to Verbatim

**Every word, on your machine.**

Verbatim turns speech, media, images, and PDFs into clean Markdown without sending a byte to a cloud service.

## Key Points

- **Dictation** types into any focused app with `Ctrl+Shift+D`.
- **Notes** converts audio, video, YouTube, and TikTok links into structured Markdown.
- **Vision** runs OCR, QR/barcode detection, batch exports, and searchable PDFs.
- **Everything local** lives in SQLite under `~/.verbatim/`.

## Detailed Notes

Start a dictation from the Dictation tab. Drop a file in Notes, or paste a URL. Drop images or PDFs in Vision and select the text in the result panel.

## Action Items

- [ ] Download the local models from Settings → Models
- [ ] Try `Ctrl+Shift+D` in another app
- [ ] Make a searchable PDF from a scan
"#;
    let note = Note {
        id: uuid::Uuid::new_v4().to_string(),
        title: "Welcome to Verbatim".into(),
        source: "Verbatim".into(),
        source_type: "manual".into(),
        markdown: markdown.into(),
        transcript: String::new(),
        duration: 0.0,
        created_at: models::now_iso(),
        updated_at: models::now_iso(),
        tags: vec!["welcome".into()],
    };
    if let Err(error) = db.save_note(note) {
        tracing::warn!(error = %error, "could not seed welcome note");
    }
}
